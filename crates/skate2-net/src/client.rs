//! The game's side of the connection: [`ClientCore`] is the logic over datagrams, and
//! [`Client`] runs it on a socket in its own thread, so neither a slow network nor the frame
//! codec ever holds up a simulation tick.

use crate::crypto::{Build, Handshake, PROOF_BYTES, Session};
use crate::frames::{FrameReceiver, FrameSender, NetFrame};
use crate::protocol::{ClientMessage, FactsNotes, ServerMessage};
use crate::transport::{self, Endpoint, kind, read_header};
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const CONNECT_RETRY: Duration = Duration::from_millis(250);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const PING_INTERVAL: Duration = Duration::from_secs(1);
/// While the game loads, how long the thread lets pass without a replay frame or `Busy` from the
/// game before it says `Busy` itself, and for how long after the load began it does so (a game
/// that stays stuck past it is let go by the server's inactivity rule).
const LOADING_BUSY_INTERVAL: Duration = Duration::from_millis(250);
const LOADING_BUSY_LIMIT: Duration = Duration::from_secs(30);

/// Why the connection ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disconnect {
    /// The server never answered.
    Unreachable,
    /// The server refused the connection (`refuse::*`).
    Refused(u8),
    /// The server closed it (`leave::*`).
    Closed(u8),
    /// No datagram arrived for [`transport::TIMEOUT`].
    TimedOut,
    /// The address could not be resolved or the socket failed.
    Network(String),
}

/// What the game hears from the client.
#[derive(Clone, Debug, PartialEq)]
pub enum ClientEvent {
    SignedIn { user_id: u64 },
    Disconnected(Disconnect),
    Message(ServerMessage),
    /// A remote player's replay frame, decoded, and whether it is the last of its run.
    Frame { user_id: u64, frame: Box<NetFrame>, run_end: bool },
    /// A remote player's facts of one tick.
    Facts { user_id: u64, tick: u32, stint: u32, notes: FactsNotes, data: Vec<u8> },
}

#[derive(Debug)]
enum State {
    Connecting { nonce: u64, next_attempt: Instant, deadline: Instant, handshake: Box<Handshake> },
    Connected { endpoint: Box<Endpoint> },
    Closed,
}

/// The client's logic over datagrams.
#[derive(Debug)]
pub struct ClientCore {
    state: State,
    build: Build,
    hello: ClientMessage,
    events: Vec<ClientEvent>,
    outgoing: Vec<Vec<u8>>,
    own_frames: FrameSender,
    remote_frames: HashMap<u64, FrameReceiver>,
    slots: HashMap<u64, u8>,
    acks_sent: HashMap<u64, (u32, u32)>,
    epoch: Instant,
    last_ping: Option<Instant>,
    rtt: Option<Duration>,
    signed_in: bool,
}

impl ClientCore {
    /// A client of `build` connecting.
    #[must_use]
    pub fn new(nonce: u64, nickname: &str, game_version: &str, build: Build, now: Instant) -> Self {
        let (state, events) = match Handshake::new() {
            Some(handshake) => (State::Connecting { nonce, next_attempt: now, deadline: now + CONNECT_TIMEOUT, handshake: Box::new(handshake) }, Vec::new()),
            None => (State::Closed, vec![ClientEvent::Disconnected(Disconnect::Network("no system random source".to_owned()))]),
        };
        Self {
            state,
            build,
            hello: ClientMessage::Hello { nickname: nickname.to_owned(), game_version: game_version.to_owned() },
            events,
            outgoing: Vec::new(),
            own_frames: FrameSender::default(),
            remote_frames: HashMap::new(),
            slots: HashMap::new(),
            acks_sent: HashMap::new(),
            epoch: now,
            last_ping: None,
            rtt: None,
            signed_in: false,
        }
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        matches!(self.state, State::Closed)
    }

    /// The last ping's round trip.
    #[must_use]
    pub fn rtt(&self) -> Option<Duration> {
        self.rtt
    }

    pub fn take_events(&mut self) -> Vec<ClientEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn take_outgoing(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.outgoing)
    }

    fn close(&mut self, reason: Disconnect) {
        if !self.is_closed() {
            self.state = State::Closed;
            self.events.push(ClientEvent::Disconnected(reason));
        }
    }

    /// Ends the connection, telling the server.
    pub fn disconnect(&mut self) {
        if let State::Connected { endpoint } = &mut self.state {
            for _ in 0..3 {
                self.outgoing.extend(endpoint.disconnect_datagram(crate::protocol::leave::QUIT));
            }
        }
        self.state = State::Closed;
    }

    /// Queues a message for reliable delivery.
    pub fn send(&mut self, message: &ClientMessage) {
        if let State::Connected { endpoint } = &mut self.state {
            endpoint.send_reliable(&message.encode());
        }
    }

    /// Codes a run of the player's frames against what the server acknowledged and queues it.
    pub fn send_frames(&mut self, frames: &[NetFrame]) {
        if let State::Connected { endpoint } = &mut self.state
            && let Some(run) = self.own_frames.encode(frames)
        {
            let message = ClientMessage::Frames(run).encode();
            // A run coded whole (no acknowledged baseline yet) may not fit a datagram.
            if message.len() > transport::MAX_UNRELIABLE {
                endpoint.send_reliable(&message);
            } else {
                endpoint.send_unreliable(message);
            }
        }
    }

    fn user_of(&self, slot: u8) -> Option<u64> {
        self.slots.iter().find(|&(_, &held)| held == slot).map(|(&user_id, _)| user_id)
    }

    /// A datagram from the server.
    pub fn receive(&mut self, datagram: &[u8], now: Instant) {
        let Some((header, mut reader)) = read_header(datagram) else { return };
        match (&mut self.state, header.kind) {
            (State::Connecting { nonce, handshake, .. }, kind::ACCEPT) => {
                let (Ok(answer), Ok(connection), Ok(server)) = (reader.u64(), reader.u32(), transport::read_key(&mut reader)) else { return };
                let Ok(proof) = reader.raw(PROOF_BYTES) else { return };
                // Only a server of this build can prove itself, and only to this connect.
                if answer != *nonce || !self.build.verify_accept(answer, connection, &handshake.public(), &server, proof) {
                    return;
                }
                let Some(session) = Session::client(self.build, handshake, &server, answer, connection) else { return };
                let mut endpoint = Box::new(Endpoint::new(connection, session, now));
                endpoint.send_reliable(&self.hello.encode());
                self.state = State::Connected { endpoint };
            }
            (State::Connecting { nonce, .. }, kind::REJECT) => {
                let (Ok(answer), Ok(reason)) = (reader.u64(), reader.u8()) else { return };
                if answer == *nonce {
                    self.close(Disconnect::Refused(reason));
                }
            }
            (State::Connected { endpoint }, kind::DATA) => {
                if reader.u32().ok() != Some(endpoint.connection()) {
                    return;
                }
                match endpoint.receive_datagram(datagram, now) {
                    Ok(true) => self.drain(now),
                    Ok(false) => {}
                    Err(_) => self.close(Disconnect::Closed(crate::protocol::leave::LOST_CONNECTION)),
                }
            }
            (State::Connected { endpoint }, kind::DISCONNECT) if reader.u32().ok() == Some(endpoint.connection()) => {
                if let Some(reason) = endpoint.open_disconnect(datagram) {
                    self.close(Disconnect::Closed(reason));
                }
            }
            _ => {}
        }
    }

    fn drain(&mut self, now: Instant) {
        loop {
            let State::Connected { endpoint } = &mut self.state else { return };
            let (bytes, reliable) = if let Some(bytes) = endpoint.take_reliable() {
                (bytes, true)
            } else if let Some(bytes) = endpoint.take_unreliable() {
                (bytes, false)
            } else {
                return;
            };
            let Ok(message) = ServerMessage::decode(&bytes) else {
                if reliable {
                    self.close(Disconnect::Closed(crate::protocol::leave::LOST_CONNECTION));
                    return;
                }
                continue;
            };
            self.handle(message, now);
        }
    }

    fn handle(&mut self, message: ServerMessage, now: Instant) {
        match message {
            ServerMessage::Welcome { user_id } => {
                self.signed_in = true;
                self.events.push(ClientEvent::SignedIn { user_id });
            }
            ServerMessage::Refused { reason } => self.close(Disconnect::Refused(reason)),
            ServerMessage::RemoteFrames { slot, run } => {
                let Some(user_id) = self.user_of(slot) else { return };
                let receiver = self.remote_frames.entry(user_id).or_default();
                if let Ok(Some(decoded)) = receiver.decode(&run) {
                    let count = decoded.len();
                    for (index, frame) in decoded.iter().enumerate() {
                        let frame = receiver.complete(frame);
                        self.events.push(ClientEvent::Frame { user_id, frame: Box::new(frame), run_end: index + 1 == count });
                    }
                }
            }
            ServerMessage::Facts { slot, tick, stint, notes, data } => {
                if let Some(user_id) = self.user_of(slot) {
                    self.events.push(ClientEvent::Facts { user_id, tick, stint, notes, data });
                }
            }
            ServerMessage::FrameAck { latest, mask } => self.own_frames.acknowledge(latest, mask),
            ServerMessage::Pong { time } => {
                let elapsed = u64::try_from(now.saturating_duration_since(self.epoch).as_micros()).unwrap_or(u64::MAX);
                self.rtt = Some(Duration::from_micros(elapsed.saturating_sub(time)));
            }
            message => {
                match &message {
                    ServerMessage::GameJoined { players, .. } => {
                        self.slots = players.iter().map(|player| (player.user_id, player.slot)).collect();
                        self.remote_frames.clear();
                        self.acks_sent.clear();
                        self.own_frames.reset();
                    }
                    ServerMessage::PlayerJoin(player) => {
                        self.slots.insert(player.user_id, player.slot);
                        self.remote_frames.remove(&player.user_id);
                    }
                    ServerMessage::PlayerLeave { user_id, .. } => {
                        self.slots.remove(user_id);
                        self.remote_frames.remove(user_id);
                        self.acks_sent.remove(user_id);
                    }
                    _ => {}
                }
                self.events.push(ClientEvent::Message(message));
            }
        }
    }

    /// Retries, acks, pings and the connection's datagrams.
    pub fn update(&mut self, now: Instant) {
        match &mut self.state {
            State::Connecting { nonce, next_attempt, deadline, handshake } => {
                if now >= *deadline {
                    self.close(Disconnect::Unreachable);
                } else if now >= *next_attempt {
                    *next_attempt = now + CONNECT_RETRY;
                    let key = handshake.public();
                    self.outgoing.push(transport::connect_datagram(*nonce, &key, &self.build.connect_proof(*nonce, &key)));
                }
            }
            State::Connected { endpoint } => {
                if endpoint.timed_out(now) {
                    self.close(Disconnect::TimedOut);
                    return;
                }
                let mut acks = Vec::new();
                for (user_id, receiver) in &self.remote_frames {
                    let (Some(ack), Some(&slot)) = (receiver.ack(), self.slots.get(user_id)) else { continue };
                    if self.acks_sent.get(user_id) != Some(&ack) {
                        self.acks_sent.insert(*user_id, ack);
                        acks.push((slot, ack.0, ack.1));
                    }
                }
                if !acks.is_empty() {
                    endpoint.send_deferred(ClientMessage::FrameAcks(acks).encode(), now);
                }
                if self.signed_in && self.last_ping.is_none_or(|last| now.saturating_duration_since(last) >= PING_INTERVAL) {
                    self.last_ping = Some(now);
                    let time = u64::try_from(now.saturating_duration_since(self.epoch).as_micros()).unwrap_or(u64::MAX);
                    endpoint.send_unreliable(ClientMessage::Ping { time }.encode());
                }
                self.outgoing.extend(endpoint.poll(now));
            }
            State::Closed => {}
        }
    }
}

enum Command {
    Send(ClientMessage),
    /// Messages and a run of replay frames, handed over together so they share a datagram.
    Tick(Vec<ClientMessage>, Vec<NetFrame>),
    /// The game is loading (a travel streaming its destination), when its own thread may stall
    /// past the server's inactivity limit.
    Loading(bool),
    Shutdown,
}

/// A connection to the dedicated server on its own thread.
pub struct Client {
    commands: Sender<Command>,
    events: Receiver<ClientEvent>,
    thread: Option<JoinHandle<()>>,
}

impl Client {
    /// Starts connecting to `server` (`host:port`) as `nickname`.
    #[must_use]
    pub fn connect(server: &str, nickname: &str, game_version: &str) -> Self {
        let (commands, command_queue) = mpsc::channel();
        let (event_queue, events) = mpsc::channel();
        let server = server.to_owned();
        let nickname = nickname.to_owned();
        let game_version = game_version.to_owned();
        let thread = std::thread::Builder::new()
            .name("online".into())
            .spawn(move || {
                if let Err(error) = run_client(&server, &nickname, &game_version, &command_queue, &event_queue) {
                    let _ = event_queue.send(ClientEvent::Disconnected(Disconnect::Network(error)));
                }
            })
            .ok();
        Self { commands, events, thread }
    }

    pub fn send(&self, message: ClientMessage) {
        let _ = self.commands.send(Command::Send(message));
    }

    /// Reliable messages, then a run of replay frames: one datagram where they fit.
    pub fn send_tick(&self, messages: Vec<ClientMessage>, frames: Vec<NetFrame>) {
        let _ = self.commands.send(Command::Tick(messages, frames));
    }

    /// Whether the game is loading: meanwhile the connection's thread says `Busy` for it whenever
    /// the game itself has said nothing for a while.
    pub fn set_loading(&self, loading: bool) {
        let _ = self.commands.send(Command::Loading(loading));
    }

    /// Everything heard since the last call.
    #[must_use]
    pub fn poll(&self) -> Vec<ClientEvent> {
        let mut events = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return events,
            }
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn resolve(server: &str) -> Result<SocketAddr, String> {
    let with_port = if server.rsplit_once(':').is_some_and(|(_, port)| port.parse::<u16>().is_ok()) {
        server.to_owned()
    } else {
        format!("{server}:{}", crate::server::DEFAULT_PORT)
    };
    with_port
        .to_socket_addrs()
        .map_err(|error| format!("{server}: {error}"))?
        .find(SocketAddr::is_ipv4)
        .ok_or_else(|| format!("{server}: no IPv4 address"))
}

/// `SKATE2_DIAGNOSTIC_NET=<delay ms>:<jitter ms>:<loss %>`: each datagram either way waits the
/// delay plus up to the jitter (so some overtake others) or is lost, as over a long link.
struct Impairment {
    delay: Duration,
    jitter: u64,
    loss: u64,
    state: u64,
    outbound: Vec<(Instant, Vec<u8>)>,
    inbound: Vec<(Instant, Vec<u8>)>,
}

impl Impairment {
    fn from_env(seed: u64) -> Option<Self> {
        let spec = std::env::var("SKATE2_DIAGNOSTIC_NET").ok()?;
        let values: Vec<u64> = spec.split(':').filter_map(|value| value.trim().parse().ok()).collect();
        Some(Self {
            delay: Duration::from_millis(values.first().copied().unwrap_or(0)),
            jitter: values.get(1).copied().unwrap_or(0),
            loss: values.get(2).copied().unwrap_or(0),
            state: seed | 1,
            outbound: Vec::new(),
            inbound: Vec::new(),
        })
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn admit(&mut self, inbound: bool, datagram: Vec<u8>, now: Instant) {
        if self.next() % 100 < self.loss {
            return;
        }
        let due = now + self.delay + Duration::from_millis(if self.jitter == 0 { 0 } else { self.next() % (self.jitter + 1) });
        if inbound { &mut self.inbound } else { &mut self.outbound }.push((due, datagram));
    }

    fn due(queue: &mut Vec<(Instant, Vec<u8>)>, now: Instant) -> Vec<Vec<u8>> {
        let mut due = Vec::new();
        queue.retain_mut(|(at, datagram)| {
            if *at <= now {
                due.push((*at, std::mem::take(datagram)));
                false
            } else {
                true
            }
        });
        due.sort_by_key(|(at, _)| *at);
        due.into_iter().map(|(_, datagram)| datagram).collect()
    }
}

fn run_client(server: &str, nickname: &str, game_version: &str, commands: &Receiver<Command>, events: &Sender<ClientEvent>) -> Result<(), String> {
    let addr = resolve(server)?;
    let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(|error| error.to_string())?;
    socket.connect(addr).map_err(|error| error.to_string())?;
    socket.set_nonblocking(true).map_err(|error| error.to_string())?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0x5eed, |time| u64::from(time.subsec_nanos()) ^ time.as_secs().rotate_left(32))
        | 1;
    let mut core = ClientCore::new(nonce, nickname, game_version, Build::current(), Instant::now());
    let mut impairment = Impairment::from_env(nonce);
    let mut buffer = [0_u8; 2048];
    let mut loading: Option<Instant> = None;
    let mut alive_sent = Instant::now();
    loop {
        let mut busy = false;
        loop {
            match commands.try_recv() {
                Ok(Command::Send(message)) => {
                    busy = true;
                    if matches!(message, ClientMessage::Busy) {
                        alive_sent = Instant::now();
                    }
                    core.send(&message);
                }
                Ok(Command::Tick(messages, frames)) => {
                    busy = true;
                    for message in &messages {
                        core.send(message);
                    }
                    if !frames.is_empty() {
                        alive_sent = Instant::now();
                        core.send_frames(&frames);
                    }
                }
                Ok(Command::Loading(on)) => {
                    loading = if on { Some(loading.unwrap_or_else(Instant::now)) } else { None };
                }
                Ok(Command::Shutdown) | Err(TryRecvError::Disconnected) => {
                    core.disconnect();
                    for datagram in core.take_outgoing() {
                        let _ = socket.send(&datagram);
                    }
                    return Ok(());
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        loop {
            match socket.recv(&mut buffer) {
                Ok(length) => {
                    busy = true;
                    match impairment.as_mut() {
                        Some(impairment) => impairment.admit(true, buffer[..length].to_vec(), Instant::now()),
                        None => core.receive(&buffer[..length], Instant::now()),
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                // An ICMP port-unreachable from a datagram the server was not up for.
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(error) => return Err(error.to_string()),
            }
        }
        let now = Instant::now();
        if loading.is_some_and(|since| now.saturating_duration_since(since) < LOADING_BUSY_LIMIT)
            && now.saturating_duration_since(alive_sent) >= LOADING_BUSY_INTERVAL
        {
            core.send(&ClientMessage::Busy);
            alive_sent = now;
        }
        if let Some(impairment) = impairment.as_mut() {
            for datagram in Impairment::due(&mut impairment.inbound, Instant::now()) {
                core.receive(&datagram, Instant::now());
            }
        }
        core.update(Instant::now());
        for datagram in core.take_outgoing() {
            match impairment.as_mut() {
                Some(impairment) => impairment.admit(false, datagram, Instant::now()),
                None => {
                    let _ = socket.send(&datagram);
                }
            }
        }
        if let Some(impairment) = impairment.as_mut() {
            for datagram in Impairment::due(&mut impairment.outbound, Instant::now()) {
                let _ = socket.send(&datagram);
            }
        }
        for event in core.take_events() {
            if events.send(event).is_err() {
                return Ok(());
            }
        }
        if core.is_closed() {
            // Keep the thread until the game drops the client, so it can read the reason.
            match commands.recv() {
                Ok(Command::Shutdown) | Err(_) => return Ok(()),
                Ok(_) => continue,
            }
        }
        if !busy {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
