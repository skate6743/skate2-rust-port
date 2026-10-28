//! The dedicated server (`skate2-pc --server`): sign-in, matchmaking and every game, in the
//! role EA's "RB" servers had. A game's server owns its session state and barriers
//! (`GameReset`, `GameChallengeLoaded`, `GameSyncPoint`, `GameComplete`, results) and relays
//! the players' recipes, replay frames and challenge facts; it simulates nothing.
//!
//! [`ServerCore`] is the logic over datagrams; [`run`] puts it on a socket.

use crate::crypto::{Build, Handshake, KEY_BYTES, PROOF_BYTES, Session};
use crate::frames::{AckState, FrameReceiver, encode_for};
use crate::moderation::{self, BanList, PanelKey};
use crate::panel;
use crate::protocol::{
    ClientMessage, GameAttributes, GameResult, LobbyInfo, MAX_LOBBIES, MAX_PLAYERS, Phase, PlayNow, PlayNowMode, PlayerInfo, PlayerStats,
    ServerMessage, challenge, leave, play_now, proposal, refuse, request, reset, timer, valid_chat, valid_nickname,
};
use crate::stats;
use crate::transport::{self, Endpoint, PROTOCOL_VERSION, kind, read_header};
use crate::words::WordList;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::io::Write as _;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// What a message asks that only the game host peer may: start the challenge, make the game
/// private or public, kick a player, set the next challenge, or change challenge or location.
fn host_only(message: &ClientMessage) -> Option<&'static str> {
    match message {
        ClientMessage::GameRequest { request: request::START_GAME, .. } => Some("start the challenge"),
        ClientMessage::GameRequest { request: request::CHANGE_PRIVATE_STATUS, .. } => Some("change the game's privacy"),
        ClientMessage::GameRequest { request: request::KICK_PLAYER, .. } => Some("kick a player"),
        ClientMessage::ResetAttributes(_) => Some("set the next challenge"),
        ClientMessage::RequestChange { .. } => Some("change the challenge"),
        _ => None,
    }
}

/// The port the server listens on unless told otherwise.
pub const DEFAULT_PORT: u16 = 32_082;
/// Connections one server accepts.
pub const MAX_CONNECTIONS: usize = 512;
/// How long players may take to load a challenge before it starts without them.
const LOAD_TIMEOUT: Duration = Duration::from_secs(90);
/// How long a round barrier waits for a player before going on without them.
const SYNC_TIMEOUT: Duration = Duration::from_secs(45);
/// How long the results wait for a player's `GameComplete` and `GameResults`.
const RESULTS_TIMEOUT: Duration = Duration::from_secs(60);
/// How long a `Stalled` report counts (PC port addition): a round that still waits sends its
/// report again every second, so an old report, from a wait that has ended, joins no later one.
const STALL_REPORT_FRESH: Duration = Duration::from_secs(3);
/// `GameTimer(DisplayResults)`: seconds the results board shows.
const DISPLAY_RESULTS: u64 = 4;
/// How long after `GameExitPostChallenge` the server waits for the host's next attributes.
const NEXT_CHALLENGE_GRACE: Duration = Duration::from_secs(5);
/// `GameChange`'s countdown before the game moves.
const CHANGE_DELAY: Duration = Duration::from_secs(2);
/// How long the members a freeskate activity is proposed to have to accept it.
const PROPOSAL_TIME: Duration = Duration::from_secs(15);
/// How long an accepted activity waits for the members still on their way into the game's world
/// (who joined during the proposal), so that it starts with them.
const ARRIVAL_WAIT: Duration = Duration::from_secs(5);
/// The chat flood limit: the lines a member's chat may relay in any `CHAT_WINDOW`; more are
/// dropped.
const CHAT_LINES: usize = 5;
const CHAT_WINDOW: Duration = Duration::from_secs(10);
/// The most connections one IP address holds: players behind one router share it.
const MAX_CONNECTIONS_PER_IP: usize = 8;
/// How long an accepted connection may go without signing in: the game sends its hello the
/// moment it is accepted, so one that only keeps the connection alive is not the game.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(10);
/// The runs of replay frames a connection may send: the game sends one every two or three ticks
/// (20 or 30 a second), more after a stall; beyond that they are dropped unrelayed, as lost ones
/// are.
const FRAME_RATE: Rate = Rate { per_second: 90.0, burst: 60.0 };
/// The reliable messages a connection may send: the game's facts every tick and a few others. A
/// connection sending more is not running the game and is dropped (`leave::KICKED_FOR_FLOOD`).
const MESSAGE_RATE: Rate = Rate { per_second: 240.0, burst: 600.0 };
/// How often a connection may matchmake: the game asks again only once its search ended, so a
/// player's few quick retries pass and a stream of them does not churn the lobbies.
const PLAY_NOW_RATE: Rate = Rate { per_second: 1.0, burst: 4.0 };
/// How often a connection may ask for the lobby browser's listing: the game asks as the browser
/// opens and on its refresh, at most once a second; past this the requests go unanswered.
const LOBBY_LIST_RATE: Rate = Rate { per_second: 1.0, burst: 5.0 };

/// Ids, seeds and sessions: the system's random source, so no player can work out another's id
/// or a coming seed from its own; a xorshift generator stands in without one.
#[derive(Clone, Debug)]
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        if let Ok(value) = getrandom::u64() {
            return value;
        }
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn next_u32(&mut self) -> u32 {
        u32::try_from(self.next() >> 32).unwrap_or_default()
    }
}

/// A sending rate: tokens a second, and the most kept.
#[derive(Clone, Copy, Debug)]
struct Rate {
    per_second: f32,
    burst: f32,
}

/// A token bucket of a [`Rate`].
#[derive(Debug)]
struct Bucket {
    tokens: f32,
    at: Instant,
}

impl Bucket {
    fn full(rate: Rate, now: Instant) -> Self {
        Self { tokens: rate.burst, at: now }
    }

    /// Takes a token, refilled at the rate since the last take; `false` when none is left.
    fn take(&mut self, rate: Rate, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.at).as_secs_f32();
        self.tokens = elapsed.mul_add(rate.per_second, self.tokens).min(rate.burst);
        self.at = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

/// A duration in whole milliseconds, as messages carry it.
fn millis(duration: Duration) -> u32 {
    u32::try_from(duration.as_millis()).unwrap_or(u32::MAX)
}

/// The next id of a counter that wraps, passing over 0 and the ids still in use: four billion
/// connections or games on, the counter comes round to ids that may still be held.
fn next_id(counter: &mut u32, in_use: impl Fn(u32) -> bool) -> u32 {
    let mut id = *counter;
    while id == 0 || in_use(id) {
        id = id.wrapping_add(1);
    }
    *counter = id.wrapping_add(1);
    id
}

#[derive(Debug)]
struct User {
    id: u64,
    name: String,
    /// The game version its hello named, which the lobby browser's games match.
    version: String,
    game: Option<u32>,
    /// The online challenges its build has (`Content`, PC port addition): matchmaking and the
    /// lobby browser offer it only games at them. Before it says, every game.
    content: Option<BTreeSet<u64>>,
}

#[derive(Debug)]
struct Connection {
    addr: SocketAddr,
    nonce: u64,
    /// The accept datagram, sent again for a repeated connect.
    accept: Vec<u8>,
    /// When the handshake accepted it, for [`SIGN_IN_TIMEOUT`].
    opened: Instant,
    endpoint: Endpoint,
    user: Option<User>,
    /// The replay frames ([`FRAME_RATE`]), reliable messages ([`MESSAGE_RATE`]), matchmaking
    /// requests ([`PLAY_NOW_RATE`]) and lobby listings ([`LOBBY_LIST_RATE`]) it may still send.
    frames: Bucket,
    messages: Bucket,
    play_now: Bucket,
    lobby_list: Bucket,
}

#[derive(Debug)]
struct Member {
    connection: u32,
    user_id: u64,
    name: String,
    slot: u8,
    recipe: Option<Arc<Vec<u8>>>,
    /// When the server last took the member's recipe.
    recipe_at: Option<Instant>,
    frames: FrameReceiver,
    /// What this member holds of each other member's frames, by their user id.
    holds: HashMap<u64, AckState>,
    acked_own: Option<(u32, u32)>,
    /// When the member last sent a replay frame or said it had none to send.
    alive: Instant,
    /// The server holds the member's recipe and a replay frame: its skater is in the game's
    /// world, so its chat lines are relayed and a challenge started now takes it.
    in_world: bool,
    /// When the member's chat lines in the flood limit's window were relayed.
    chat_sent: VecDeque<Instant>,
    /// A run of its frames failed to decode and was logged; later ones are only dropped.
    bad_frames_logged: bool,
}

#[derive(Debug)]
enum GamePhase {
    LobbySkate,
    Loading { participants: Vec<u64>, loaded: BTreeSet<u64>, deadline: Instant },
    /// `stalls`: each participant's last `Stalled` report, the player and tick its round waits on,
    /// and when it came.
    Challenge { participants: Vec<u64>, rounds: BTreeMap<u32, (BTreeSet<u64>, Instant)>, complete: BTreeSet<u64>, results: BTreeMap<u64, GameResult>, all_complete: bool, deadline: Option<Instant>, stalls: BTreeMap<u64, (u64, u32, Instant)> },
    Results { participants: Vec<u64>, until: Instant },
    NextChallenge { participants: Vec<u64>, until: Instant },
}

/// A freeskate activity proposed in an online free skate (`Message_GameProposal`).
#[derive(Debug)]
struct Proposal {
    activity: u64,
    proposer: u64,
    /// The members in the game's world as it was proposed: it starts once every one of them
    /// still in the game has accepted it.
    invited: BTreeSet<u64>,
    accepted: BTreeSet<u64>,
    deadline: Instant,
    /// When every member it was proposed to had accepted: the start waits from then, at most
    /// [`ARRIVAL_WAIT`], for the members still on their way into the game's world.
    accepted_at: Option<Instant>,
}

#[derive(Debug)]
struct Game {
    id: u32,
    attributes: GameAttributes,
    members: Vec<Member>,
    phase: GamePhase,
    session: u32,
    change: Option<(u8, u64, Instant)>,
    /// The addresses of the players its host kicked (PC port addition): the lobby browser and
    /// matchmaking hide the game from them, and it takes none of them again, whatever name they
    /// come back under.
    kicked: BTreeSet<IpAddr>,
    /// The freeskate activity proposed and not yet started or timed out.
    proposal: Option<Proposal>,
    /// The freeskate activity the challenge is, from its load until the game is back in lobby
    /// skate: no matchmaking or lobby browser offers the game meanwhile.
    activity: Option<u64>,
}

impl Game {
    fn host(&self) -> Option<u64> {
        self.members.first().map(|member| member.user_id)
    }

    /// The freeskate activity under way.
    const fn activity(&self) -> Option<u64> {
        self.activity
    }

    /// The members in the game's world, in join order.
    fn in_world(&self) -> Vec<u64> {
        self.members.iter().filter(|member| member.in_world).map(|member| member.user_id).collect()
    }

    fn member(&self, user_id: u64) -> Option<&Member> {
        self.members.iter().find(|member| member.user_id == user_id)
    }

    fn member_mut(&mut self, user_id: u64) -> Option<&mut Member> {
        self.members.iter_mut().find(|member| member.user_id == user_id)
    }

    fn free_slot(&self) -> Option<u8> {
        (0..MAX_PLAYERS)
            .filter_map(|slot| u8::try_from(slot).ok())
            .find(|slot| self.members.iter().all(|member| member.slot != *slot))
    }

    fn players(&self) -> Vec<PlayerInfo> {
        self.members.iter().map(|member| PlayerInfo { user_id: member.user_id, slot: member.slot, name: member.name.clone() }).collect()
    }

    fn phase_code(&self) -> Phase {
        match self.phase {
            GamePhase::LobbySkate => Phase::LobbySkate,
            GamePhase::Loading { .. } => Phase::LoadingChallenge,
            GamePhase::Challenge { .. } => Phase::Challenge,
            GamePhase::Results { .. } | GamePhase::NextChallenge { .. } => Phase::Results,
        }
    }

    fn full(&self) -> bool {
        self.members.len() >= usize::from(self.attributes.max_players)
    }

    /// The challenge's participants from its load until the game is back in lobby skate.
    fn participants(&self) -> &[u64] {
        match &self.phase {
            GamePhase::Loading { participants, .. }
            | GamePhase::Challenge { participants, .. }
            | GamePhase::Results { participants, .. }
            | GamePhase::NextChallenge { participants, .. } => participants,
            GamePhase::LobbySkate => &[],
        }
    }

    /// Whether `viewer` sees nothing of `source`'s skater: from a challenge's load until the game
    /// is back in lobby skate, its participants see nothing of the members in lobby skate (who
    /// joined during it, were not yet in the world at its start, did not load it, or left it), as
    /// in the original. Those members watch the participants (PC port addition), whom their game
    /// shows as ghosts.
    fn hidden_from(&self, source: u64, viewer: u64) -> bool {
        let participants = self.participants();
        participants.contains(&viewer) && !participants.contains(&source)
    }

    /// The attributes a player joining now loads into: the game's, or the pending `GameChange`'s
    /// challenge and location, where every member is about to go.
    fn upcoming_attributes(&self) -> GameAttributes {
        let mut attributes = self.attributes.clone();
        if let Some((challenge_type, challenge_key, _)) = self.change {
            attributes.challenge_type = challenge_type;
            attributes.challenge_key = challenge_key;
        }
        attributes
    }

    /// The challenge's player table: the participants in slot order from index 0, with no gap
    /// a departed player's slot would leave (the challenge scripts number their skaters 0 to
    /// `GetNumOnlineSkaters() - 1`).
    fn slots(&self, participants: &[u64]) -> [u64; MAX_PLAYERS] {
        let mut members: Vec<&Member> = self.members.iter().filter(|member| participants.contains(&member.user_id)).collect();
        members.sort_by_key(|member| member.slot);
        let mut slots = [0; MAX_PLAYERS];
        for (slot, member) in slots.iter_mut().zip(members) {
            *slot = member.user_id;
        }
        slots
    }
}

/// A line for the server's console.
pub type Log = Box<dyn FnMut(&str) + Send>;

/// The server's logic over datagrams.
pub struct ServerCore {
    build: Build,
    connections: HashMap<u32, Connection>,
    by_addr: HashMap<SocketAddr, u32>,
    games: BTreeMap<u32, Game>,
    next_connection: u32,
    next_game: u32,
    random: Random,
    outgoing: Vec<(SocketAddr, Vec<u8>)>,
    log: Log,
    /// The words no nickname may hold and no relayed chat line holds (`badwords.txt`).
    words: WordList,
    /// The chat-shadowbanned addresses (`chat-shadowbans.json`): a member's chat line from one
    /// comes back to it alone.
    chat_shadowbans: BanList,
    /// The banned addresses (`ip-bans.json`): no datagram from one is read or answered.
    ip_bans: BanList,
    /// When the ban lists' files were last looked at for changes.
    bans_checked: Option<Instant>,
}

impl ServerCore {
    /// A server of `build`, which only clients of the same build connect to.
    #[must_use]
    pub fn new(seed: u64, build: Build, log: Log) -> Self {
        Self {
            build,
            connections: HashMap::new(),
            by_addr: HashMap::new(),
            games: BTreeMap::new(),
            next_connection: 1,
            next_game: 1,
            random: Random(seed | 1),
            outgoing: Vec::new(),
            log,
            words: WordList::default(),
            chat_shadowbans: BanList::default(),
            ip_bans: BanList::default(),
            bans_checked: None,
        }
    }

    /// The word list nicknames and chat lines are checked against.
    pub fn set_words(&mut self, words: WordList) {
        self.words = words;
    }

    /// The chat-shadowbanned and the banned addresses (PC port addition).
    pub fn set_bans(&mut self, chat_shadowbans: BanList, ip_bans: BanList) {
        self.chat_shadowbans = chat_shadowbans;
        self.ip_bans = ip_bans;
        self.bans_checked = None;
    }

    /// Connections signed in.
    #[must_use]
    pub fn players(&self) -> usize {
        self.connections.values().filter(|connection| connection.user.is_some()).count()
    }

    #[must_use]
    pub fn games(&self) -> usize {
        self.games.len()
    }

    /// The datagrams due to be sent, with their destinations.
    pub fn take_outgoing(&mut self) -> Vec<(SocketAddr, Vec<u8>)> {
        std::mem::take(&mut self.outgoing)
    }

    /// A datagram from `addr`. One from a banned address is not even read: the server never
    /// answers it.
    pub fn receive(&mut self, addr: SocketAddr, datagram: &[u8], now: Instant) {
        if self.ip_bans.contains(addr.ip()) {
            return;
        }
        let Some((header, mut reader)) = read_header(datagram) else { return };
        if header.version != PROTOCOL_VERSION {
            if header.kind == kind::CONNECT
                && let Ok(nonce) = reader.u64()
            {
                self.outgoing.push((addr, transport::reject_datagram(nonce, refuse::VERSION)));
            }
            return;
        }
        match header.kind {
            kind::CONNECT => {
                let (Ok(nonce), Ok(key)) = (reader.u64(), transport::read_key(&mut reader)) else { return };
                let Ok(proof) = reader.raw(PROOF_BYTES) else { return };
                self.connect(addr, nonce, &key, proof, now);
            }
            kind::DATA => {
                let Ok(id) = reader.u32() else { return };
                let Some(connection) = self.connections.get_mut(&id).filter(|connection| connection.addr == addr) else { return };
                match connection.endpoint.receive_datagram(datagram, now) {
                    Ok(true) => self.drain(id, now),
                    Ok(false) => {}
                    Err(_) => self.drop_connection(id, leave::LOST_CONNECTION, now),
                }
            }
            kind::DISCONNECT => {
                let Ok(id) = reader.u32() else { return };
                let quit = self
                    .connections
                    .get_mut(&id)
                    .filter(|connection| connection.addr == addr)
                    .is_some_and(|connection| connection.endpoint.open_disconnect(datagram).is_some());
                if quit {
                    self.drop_connection(id, leave::QUIT, now);
                }
            }
            _ => {}
        }
    }

    fn connect(&mut self, addr: SocketAddr, nonce: u64, key: &[u8; KEY_BYTES], proof: &[u8], now: Instant) {
        // A client of another build cannot prove itself: it has to update.
        if !self.build.verify_connect(nonce, key, proof) {
            self.outgoing.push((addr, transport::reject_datagram(nonce, refuse::BUILD)));
            return;
        }
        if let Some(&id) = self.by_addr.get(&addr) {
            let connection = &self.connections[&id];
            if connection.nonce == nonce {
                self.outgoing.push((addr, connection.accept.clone()));
                return;
            }
            // The same address reconnecting: the old connection is gone.
            self.drop_connection(id, leave::LOST_CONNECTION, now);
        }
        if self.connections.len() >= MAX_CONNECTIONS
            || self.connections.values().filter(|connection| connection.addr.ip() == addr.ip()).count() >= MAX_CONNECTIONS_PER_IP
        {
            self.outgoing.push((addr, transport::reject_datagram(nonce, refuse::FULL)));
            return;
        }
        let Some(handshake) = Handshake::new() else {
            (self.log)("no system random source: a connection refused");
            self.outgoing.push((addr, transport::reject_datagram(nonce, refuse::FULL)));
            return;
        };
        let id = next_id(&mut self.next_connection, |id| self.connections.contains_key(&id));
        let Some(session) = Session::server(self.build, &handshake, key, nonce, id) else { return };
        let server_key = handshake.public();
        let accept = transport::accept_datagram(nonce, id, &server_key, &self.build.accept_proof(nonce, id, key, &server_key));
        self.connections.insert(
            id,
            Connection {
                addr,
                nonce,
                accept: accept.clone(),
                opened: now,
                endpoint: Endpoint::new(id, session, now),
                user: None,
                frames: Bucket::full(FRAME_RATE, now),
                messages: Bucket::full(MESSAGE_RATE, now),
                play_now: Bucket::full(PLAY_NOW_RATE, now),
                lobby_list: Bucket::full(LOBBY_LIST_RATE, now),
            },
        );
        self.by_addr.insert(addr, id);
        self.outgoing.push((addr, accept));
        (self.log)(&format!("connection {id} from {addr}"));
    }

    fn drain(&mut self, id: u32, now: Instant) {
        loop {
            let Some(connection) = self.connections.get_mut(&id) else { return };
            let (bytes, reliable) = if let Some(bytes) = connection.endpoint.take_reliable() {
                (bytes, true)
            } else if let Some(bytes) = connection.endpoint.take_unreliable() {
                (bytes, false)
            } else {
                return;
            };
            match ClientMessage::decode(&bytes) {
                Ok(message) => self.handle(id, message, reliable, now),
                // The game sends nothing it cannot decode, reliable or not.
                Err(error) => {
                    (self.log)(&format!("connection {id} sent a message that does not decode ({error}): dropped"));
                    self.drop_connection(id, leave::LOST_CONNECTION, now);
                    return;
                }
            }
        }
    }

    fn send(&mut self, id: u32, message: &ServerMessage) {
        if let Some(connection) = self.connections.get_mut(&id) {
            connection.endpoint.send_reliable(&message.encode());
        }
    }

    fn send_unreliable(&mut self, id: u32, message: &ServerMessage) {
        if let Some(connection) = self.connections.get_mut(&id) {
            let bytes = message.encode();
            // A frame coded whole for a player without a baseline may not fit a datagram.
            if bytes.len() > transport::MAX_UNRELIABLE {
                connection.endpoint.send_reliable(&bytes);
            } else {
                connection.endpoint.send_unreliable(bytes);
            }
        }
    }

    fn broadcast(&mut self, game: u32, message: &ServerMessage, except: Option<u64>) {
        let Some(targets) = self.games.get(&game).map(|game| {
            game.members.iter().filter(|member| Some(member.user_id) != except).map(|member| member.connection).collect::<Vec<_>>()
        }) else {
            return;
        };
        let bytes = message.encode();
        for target in targets {
            if let Some(connection) = self.connections.get_mut(&target) {
                connection.endpoint.send_reliable(&bytes);
            }
        }
    }

    fn user_id(&self, id: u32) -> Option<u64> {
        self.connections.get(&id)?.user.as_ref().map(|user| user.id)
    }

    fn game_of(&self, id: u32) -> Option<u32> {
        self.connections.get(&id)?.user.as_ref()?.game
    }

    /// The address a connection's datagrams come from, as kicks and bans hold it.
    fn address(&self, id: u32) -> Option<IpAddr> {
        self.connections.get(&id).map(|connection| moderation::address(connection.addr.ip()))
    }

    fn handle(&mut self, id: u32, message: ClientMessage, reliable: bool, now: Instant) {
        if reliable && self.connections.get_mut(&id).is_some_and(|connection| !connection.messages.take(MESSAGE_RATE, now)) {
            (self.log)(&format!("connection {id} sent reliable messages faster than the game does: dropped"));
            self.drop_connection(id, leave::KICKED_FOR_FLOOD, now);
            return;
        }
        let Some(user_id) = self.user_id(id) else {
            // The game's first message is its hello, on the reliable channel, and it sends nothing
            // else until it is welcomed: it pings and acknowledges frames only once signed in.
            if reliable && let ClientMessage::Hello { nickname, game_version } = &message {
                self.hello(id, nickname, game_version, now);
            } else {
                (self.log)(&format!("connection {id} sent something before signing in: dropped"));
                self.drop_connection(id, leave::KICKED_BEFORE_SIGN_IN, now);
            }
            return;
        };
        // The game asks these only while the server's messages so far make it the first member,
        // and the host changes only when the host leaves, after which its requests find it in no
        // game: a member asking them while another member is the host is not running the game.
        if let Some(asked) = host_only(&message)
            && let Some(game) = self.game_of(id)
            && !self.is_host(game, user_id)
        {
            let name = self.connections.get(&id).and_then(|c| c.user.as_ref()).map(|user| user.name.clone()).unwrap_or_default();
            (self.log)(&format!("{name} asked to {asked} in game {game}, of which it is not the host: connection {id} dropped"));
            self.drop_connection(id, leave::KICKED_FOR_HOST_REQUEST, now);
            return;
        }
        match message {
            ClientMessage::Hello { .. } | ClientMessage::CancelPlayNow => {}
            ClientMessage::PlayNow(request) => {
                if self.connections.get_mut(&id).is_some_and(|connection| connection.play_now.take(PLAY_NOW_RATE, now)) {
                    self.play_now(id, user_id, &request, now);
                } else {
                    self.send(id, &ServerMessage::PlayNowResult { result: play_now::NO_MATCH });
                }
            }
            ClientMessage::ListLobbies if reliable => {
                if self.connections.get_mut(&id).is_some_and(|connection| connection.lobby_list.take(LOBBY_LIST_RATE, now)) {
                    let lobbies = self.lobbies(id);
                    self.send(id, &ServerMessage::Lobbies(lobbies));
                }
            }
            ClientMessage::JoinLobby { game } if reliable => {
                if self.connections.get_mut(&id).is_some_and(|connection| connection.play_now.take(PLAY_NOW_RATE, now)) {
                    self.join_lobby(id, user_id, game, now);
                } else {
                    self.send(id, &ServerMessage::PlayNowResult { result: play_now::NO_MATCH });
                }
            }
            ClientMessage::Content(challenges) if reliable => {
                if let Some(user) = self.connections.get_mut(&id).and_then(|connection| connection.user.as_mut()) {
                    user.content = Some(challenges.into_iter().collect());
                }
            }
            ClientMessage::LeaveGame => self.leave_game(id, leave::QUIT, now),
            ClientMessage::Ping { time } if !reliable => self.send_unreliable(id, &ServerMessage::Pong { time }),
            ClientMessage::Frames(run) => {
                self.mark_alive(id, user_id, now);
                if self.connections.get_mut(&id).is_some_and(|connection| connection.frames.take(FRAME_RATE, now)) {
                    self.relay_frames(id, user_id, &run, now);
                }
            }
            ClientMessage::Busy => self.mark_alive(id, user_id, now),
            ClientMessage::FrameAcks(acks) if !reliable => self.frame_acks(id, user_id, &acks),
            message if reliable => {
                let Some(game) = self.game_of(id) else { return };
                self.game_message(game, user_id, message, now);
            }
            _ => {}
        }
    }

    fn mark_alive(&mut self, id: u32, user_id: u64, now: Instant) {
        let Some(game_id) = self.game_of(id) else { return };
        if let Some(member) = self.games.get_mut(&game_id).and_then(|game| game.member_mut(user_id)) {
            member.alive = now;
        }
    }

    fn hello(&mut self, id: u32, nickname: &str, version: &str, now: Instant) {
        // The game signs in only with what its nickname prompt makes.
        if !valid_nickname(nickname) {
            (self.log)(&format!("connection {id} signed in with a nickname the game cannot make ({} bytes): dropped", nickname.len()));
            self.drop_connection(id, leave::KICKED_FOR_NICKNAME, now);
            return;
        }
        if self.words.holds_word(nickname) {
            (self.log)(&format!("connection {id} signed in as {nickname}, which holds a listed word: dropped"));
            self.drop_connection(id, leave::KICKED_FOR_NICKNAME, now);
            return;
        }
        let user_id = loop {
            let candidate = self.random.next();
            if candidate != 0 && self.connections.values().all(|c| c.user.as_ref().is_none_or(|user| user.id != candidate)) {
                break candidate;
            }
        };
        let name = nickname.to_owned();
        (self.log)(&format!("connection {id} signed in as {name} ({user_id:016x})"));
        if let Some(connection) = self.connections.get_mut(&id) {
            connection.user = Some(User { id: user_id, name, version: version.to_owned(), game: None, content: None });
        }
        self.send(id, &ServerMessage::Welcome { user_id });
    }

    /// `StateOnlinePlayNow`: `FindExistingServers` 0x8276f8a0 with `BuildFilters` 0x8276d2f8's
    /// filters (version, challenge type, ranking, not private, not full; quick unranked
    /// matches leave free skates out) and its `ChallengeKeyPreference` 0x8276c550, whose fit
    /// table takes a chosen location only (1 at its own key, -1, no match, at every other key)
    /// and, with none chosen, every location of the type alike (0.5); then `CreateNewServer`
    /// 0x82767cf8 for everything but a quick match, at the chosen location or the one the game
    /// picked. A free skate or private game is never ranked (`DoPlayNow` 0x827736b8). The search
    /// must be of the player's own version, a free skate searched as one and a challenge as one;
    /// a game with no one in it is never offered, nor one whose host kicked a player from the
    /// searcher's address, nor one playing a freeskate activity (PC port addition).
    fn play_now(&mut self, id: u32, user_id: u64, play: &PlayNow, now: Instant) {
        let Some(version) = self.connections.get(&id).and_then(|c| c.user.as_ref()).map(|user| user.version.clone()) else { return };
        let Some(address) = self.address(id) else { return };
        let free_skate = play.challenge_type == challenge::ONLINE_FREE_SKATE;
        let consistent = play.game_version == version
            && match play.mode {
                PlayNowMode::Quick => true,
                PlayNowMode::Custom => !free_skate,
                PlayNowMode::Freeskate => free_skate,
            };
        if !consistent {
            (self.log)(&format!("connection {id} searched inconsistently (mode {:?}, type {})", play.mode, play.challenge_type));
            self.send(id, &ServerMessage::PlayNowResult { result: play_now::NO_MATCH });
            return;
        }
        if self.game_of(id).is_some() {
            self.leave_game(id, leave::QUIT, now);
        }
        let ranked = play.ranked && !play.private && !free_skate;
        let content = self.connections.get(&id).and_then(|c| c.user.as_ref()).and_then(|user| user.content.as_ref());
        let found = if play.create || play.private {
            None
        } else {
            // A game whose location is changing is matched on where it is going.
            let mut candidates: Vec<&Game> = self
                .games
                .values()
                .filter(|game| {
                    let attributes = game.upcoming_attributes();
                    attributes.game_version == version
                        && !attributes.private
                        && attributes.ranked == ranked
                        && !game.members.is_empty()
                        && !game.full()
                        && !game.kicked.contains(&address)
                        && game.activity.is_none()
                        && content.is_none_or(|content| content.contains(&attributes.challenge_key))
                        && match play.mode {
                            PlayNowMode::Quick => ranked || attributes.challenge_type != challenge::ONLINE_FREE_SKATE,
                            PlayNowMode::Custom | PlayNowMode::Freeskate => {
                                attributes.challenge_type == play.challenge_type
                                    && play.challenge_key.is_none_or(|key| key == attributes.challenge_key)
                            }
                        }
                })
                .collect();
            // The fuller game first, then the older.
            candidates.sort_by_key(|game| (std::cmp::Reverse(game.members.len()), game.id));
            candidates.first().map(|game| game.id)
        };
        let location = play.challenge_key.unwrap_or(play.fallback_key);
        let game = match found {
            Some(game) => game,
            None if play.mode == PlayNowMode::Quick || location == 0 => {
                self.send(id, &ServerMessage::PlayNowResult { result: play_now::NO_MATCH });
                return;
            }
            None => {
                let game = next_id(&mut self.next_game, |id| self.games.contains_key(&id));
                let session = self.random.next_u32();
                self.games.insert(
                    game,
                    Game {
                        id: game,
                        attributes: GameAttributes {
                            game_version: version,
                            challenge_type: play.challenge_type,
                            challenge_key: location,
                            private: play.private,
                            max_players: u8::try_from(MAX_PLAYERS).unwrap_or(u8::MAX),
                            ranked,
                        },
                        members: Vec::new(),
                        phase: GamePhase::LobbySkate,
                        session,
                        change: None,
                        kicked: BTreeSet::new(),
                        proposal: None,
                        activity: None,
                    },
                );
                (self.log)(&format!("game {game} created: type {} challenge {location:016x}", play.challenge_type));
                game
            }
        };
        self.join(id, user_id, game, now);
    }

    /// Whether the lobby browser lists a game to a player of `version` at `address`: public,
    /// unranked, of the version, wherever a location change is taking it, not one whose host
    /// kicked a player from the address, with no freeskate activity under way, and at a challenge
    /// the player's build has (`content`, PC port addition).
    fn listed(game: &Game, version: &str, address: IpAddr, content: Option<&BTreeSet<u64>>) -> Option<GameAttributes> {
        let attributes = game.upcoming_attributes();
        (attributes.game_version == version
            && !attributes.private
            && !attributes.ranked
            && !game.members.is_empty()
            && !game.kicked.contains(&address)
            && game.activity.is_none()
            && content.is_none_or(|content| content.contains(&attributes.challenge_key)))
        .then_some(attributes)
    }

    /// The lobby browser's listing (PC port addition): every game it lists to the player, the
    /// ones with room first, then the fuller and the older.
    fn lobbies(&self, id: u32) -> Vec<LobbyInfo> {
        let Some(user) = self.connections.get(&id).and_then(|c| c.user.as_ref()) else { return Vec::new() };
        let Some(address) = self.address(id) else { return Vec::new() };
        let mut lobbies: Vec<(&Game, GameAttributes)> = self
            .games
            .values()
            .filter_map(|game| Some((game, Self::listed(game, &user.version, address, user.content.as_ref())?)))
            .collect();
        lobbies.sort_by_key(|(game, _)| (game.full(), std::cmp::Reverse(game.members.len()), game.id));
        lobbies
            .into_iter()
            .take(MAX_LOBBIES)
            .map(|(game, attributes)| LobbyInfo {
                game: game.id,
                challenge_type: attributes.challenge_type,
                challenge_key: attributes.challenge_key,
                players: u8::try_from(game.members.len()).unwrap_or(u8::MAX),
                max_players: attributes.max_players,
                phase: game.phase_code(),
            })
            .collect()
    }

    /// The lobby browser's A (PC port addition): the game, if the browser still lists it and it
    /// has room; else `PlayNowResult` says why, as matchmaking would.
    fn join_lobby(&mut self, id: u32, user_id: u64, game_id: u32, now: Instant) {
        if self.game_of(id) == Some(game_id) {
            return;
        }
        if self.game_of(id).is_some() {
            self.leave_game(id, leave::QUIT, now);
        }
        let Some((version, content)) = self.connections.get(&id).and_then(|c| c.user.as_ref()).map(|user| (user.version.clone(), user.content.clone())) else { return };
        let Some(address) = self.address(id) else { return };
        let full = self.games.get(&game_id).filter(|game| Self::listed(game, &version, address, content.as_ref()).is_some()).map(Game::full);
        match full {
            None => self.send(id, &ServerMessage::PlayNowResult { result: play_now::NO_MATCH }),
            Some(true) => self.send(id, &ServerMessage::PlayNowResult { result: play_now::FULL }),
            Some(false) => self.join(id, user_id, game_id, now),
        }
    }

    fn join(&mut self, id: u32, user_id: u64, game_id: u32, now: Instant) {
        let Some(name) = self.connections.get(&id).and_then(|c| c.user.as_ref()).map(|user| user.name.clone()) else { return };
        let Some(address) = self.address(id) else { return };
        let Some(game) = self.games.get_mut(&game_id) else { return };
        // A private game takes no one after the player who made it: matchmaking never offers one,
        // and nothing else may put a player in.
        if game.attributes.private && !game.members.is_empty() {
            self.send(id, &ServerMessage::PlayNowResult { result: play_now::FULL });
            return;
        }
        // Nor does a game take a player from an address its host kicked one from: matchmaking and
        // the lobby browser never offer it to one.
        if game.kicked.contains(&address) {
            self.send(id, &ServerMessage::PlayNowResult { result: play_now::NO_MATCH });
            return;
        }
        let Some(slot) = game.free_slot().filter(|_| !game.full()) else {
            self.send(id, &ServerMessage::PlayNowResult { result: play_now::FULL });
            return;
        };
        game.members.push(Member {
            connection: id,
            user_id,
            name: name.clone(),
            slot,
            recipe: None,
            recipe_at: None,
            frames: FrameReceiver::default(),
            holds: HashMap::new(),
            acked_own: None,
            alive: now,
            in_world: false,
            chat_sent: VecDeque::new(),
            bad_frames_logged: false,
        });
        let joined = ServerMessage::GameJoined {
            game: game_id,
            slot,
            attributes: game.upcoming_attributes(),
            players: game.players(),
            phase: game.phase_code(),
            participants: game.participants().to_vec(),
        };
        let recipes: Vec<(u64, Arc<Vec<u8>>)> =
            game.members.iter().filter_map(|member| Some((member.user_id, Arc::clone(member.recipe.as_ref()?)))).collect();
        // A freeskate activity being proposed is shown to the joiner too, whom it takes if its
        // skater is in the game's world by the start.
        let pending = game.proposal.as_ref().map(|pending| ServerMessage::GameProposal {
            kind: proposal::BROADCAST,
            activity: pending.activity,
            user_id: pending.proposer,
            timeout: millis(pending.deadline.saturating_duration_since(now)),
        });
        if let Some(connection) = self.connections.get_mut(&id)
            && let Some(user) = connection.user.as_mut()
        {
            user.game = Some(game_id);
        }
        self.send(id, &ServerMessage::PlayNowResult { result: play_now::JOINED });
        self.send(id, &joined);
        for (owner, data) in recipes {
            self.send(id, &ServerMessage::Recipe { user_id: owner, data: data.to_vec() });
        }
        if let Some(pending) = pending {
            self.send(id, &pending);
        }
        self.broadcast(game_id, &ServerMessage::PlayerJoin(PlayerInfo { user_id, slot, name: name.clone() }), Some(user_id));
        (self.log)(&format!("{name} joined game {game_id} in slot {slot}"));
    }

    fn leave_game(&mut self, id: u32, reason: u8, now: Instant) {
        let Some(user_id) = self.user_id(id) else { return };
        let Some(game_id) = self.game_of(id) else { return };
        if let Some(user) = self.connections.get_mut(&id).and_then(|c| c.user.as_mut()) {
            user.game = None;
        }
        let Some(game) = self.games.get_mut(&game_id) else { return };
        let name = game.member(user_id).map(|member| member.name.clone()).unwrap_or_default();
        game.members.retain(|member| member.user_id != user_id);
        for member in &mut game.members {
            member.holds.remove(&user_id);
        }
        if game.members.is_empty() {
            self.games.remove(&game_id);
            (self.log)(&format!("{name} left game {game_id}, which closed"));
            return;
        }
        (self.log)(&format!("{name} left game {game_id} (reason {reason})"));
        self.broadcast(game_id, &ServerMessage::PlayerLeave { user_id, reason }, None);
        self.remove_participant(game_id, user_id, now);
    }

    fn drop_connection(&mut self, id: u32, reason: u8, now: Instant) {
        self.leave_game(id, reason, now);
        if let Some(mut connection) = self.connections.remove(&id) {
            self.by_addr.remove(&connection.addr);
            if let Some(datagram) = connection.endpoint.disconnect_datagram(reason) {
                self.outgoing.push((connection.addr, datagram));
            }
            (self.log)(&format!("connection {id} closed (reason {reason})"));
        }
    }

    fn is_host(&self, game: u32, user_id: u64) -> bool {
        self.games.get(&game).and_then(Game::host) == Some(user_id)
    }

    fn game_message(&mut self, game_id: u32, user_id: u64, message: ClientMessage, now: Instant) {
        match message {
            // The game sends its skater as it enters the world, and again as the player leaves the
            // pre-game Edit Skater (PC port addition): the new one replaces it for every member
            // and for later joiners. One closer than RECIPE_SPACING to the last is dropped, so no
            // member can keep the others building skaters.
            ClientMessage::Recipe(data) => {
                let data = Arc::new(data);
                let Some(member) = self.games.get_mut(&game_id).and_then(|game| game.member_mut(user_id)) else { return };
                if member.recipe_at.is_some_and(|at| now.saturating_duration_since(at) < RECIPE_SPACING) {
                    return;
                }
                let changed = member.recipe.replace(Arc::clone(&data)).is_some();
                member.recipe_at = Some(now);
                let name = member.name.clone();
                self.broadcast(game_id, &ServerMessage::Recipe { user_id, data: data.to_vec() }, Some(user_id));
                if changed {
                    (self.log)(&format!("{name} changed skater in game {game_id}"));
                } else {
                    self.mark_in_world(game_id, user_id);
                }
            }
            ClientMessage::Chat(text) => self.chat(game_id, user_id, text, now),
            ClientMessage::GameRequest { request, value } => self.game_request(game_id, user_id, request, value, now),
            ClientMessage::ResetAttributes(attributes) => {
                if self.is_host(game_id, user_id) {
                    self.next_challenge(game_id, Some(attributes), now);
                }
            }
            // No change of location while a freeskate activity is proposed (`HubStateOnlineHost::
            // OnEvent` offers Host Options only without a proposal).
            ClientMessage::RequestChange { challenge_type, challenge_key } => {
                let Some(game) = self.games.get_mut(&game_id) else { return };
                if game.host() != Some(user_id) || !matches!(game.phase, GamePhase::LobbySkate) || game.change.is_some() || game.proposal.is_some() {
                    return;
                }
                game.change = Some((challenge_type, challenge_key, now + CHANGE_DELAY));
                let delay = millis(CHANGE_DELAY);
                self.broadcast(game_id, &ServerMessage::GameChange { challenge_type, challenge_key, delay }, None);
            }
            ClientMessage::GameProposal { kind: proposal::PROPOSE, activity } => self.propose_activity(game_id, user_id, activity, now),
            ClientMessage::GameProposal { kind: proposal::ACCEPT, activity } => self.accept_activity(game_id, user_id, activity, now),
            ClientMessage::ChallengeLoaded => {
                if let Some(GamePhase::Loading { participants, loaded, .. }) = self.games.get_mut(&game_id).map(|game| &mut game.phase)
                    && participants.contains(&user_id)
                {
                    loaded.insert(user_id);
                }
                self.advance(game_id, now);
            }
            // A participant waits at one sync point at a time, so its arrivals hold at most one
            // round open.
            ClientMessage::SyncPoint { round } => {
                if let Some(GamePhase::Challenge { participants, rounds, .. }) = self.games.get_mut(&game_id).map(|game| &mut game.phase)
                    && participants.contains(&user_id)
                    && !rounds.values().any(|(arrived, _)| arrived.contains(&user_id))
                {
                    rounds.entry(round).or_insert_with(|| (BTreeSet::new(), now + SYNC_TIMEOUT)).0.insert(user_id);
                }
                self.advance(game_id, now);
            }
            ClientMessage::Complete => {
                if let Some(GamePhase::Challenge { participants, complete, deadline, .. }) = self.games.get_mut(&game_id).map(|game| &mut game.phase)
                    && participants.contains(&user_id)
                {
                    complete.insert(user_id);
                    deadline.get_or_insert(now + RESULTS_TIMEOUT);
                }
                self.advance(game_id, now);
            }
            ClientMessage::Results(result) => {
                if let Some(GamePhase::Challenge { participants, results, .. }) = self.games.get_mut(&game_id).map(|game| &mut game.phase)
                    && participants.contains(&user_id)
                {
                    results.insert(user_id, result);
                }
                self.advance(game_id, now);
            }
            ClientMessage::Facts { tick, stint, notes, data } => {
                let Some(slot) = self.games.get(&game_id).and_then(|game| game.members.iter().find(|member| member.user_id == user_id)).map(|member| member.slot) else { return };
                self.relay_to_participants(game_id, user_id, &ServerMessage::Facts { slot, tick, stint, notes, data });
            }
            ClientMessage::Vote { score } => self.relay_to_participants(game_id, user_id, &ServerMessage::Vote { user_id, score }),
            // Leave Challenge (PC port addition): the challenge goes on without the participant,
            // as without one who left the game, and the participant waits in its pre-game lobby
            // with the members in lobby skate.
            ClientMessage::LeaveChallenge => {
                let Some(game) = self.games.get(&game_id) else { return };
                if !game.participants().contains(&user_id) {
                    return;
                }
                let name = game.member(user_id).map(|member| member.name.clone()).unwrap_or_default();
                (self.log)(&format!("{name} left game {game_id}'s challenge"));
                self.broadcast(game_id, &ServerMessage::ChallengeLeave { user_id }, None);
                self.remove_participant(game_id, user_id, now);
            }
            ClientMessage::Stalled { user_id: stalled, tick } => self.stalled(game_id, user_id, stalled, tick, now),
            _ => {}
        }
    }

    /// A S.K.A.T.E. participant's report that its round waits for another's facts at a tick (PC
    /// port addition): once every other participant, at least two of them, has reported the same
    /// player at the same tick within [`STALL_REPORT_FRESH`], that player's game has parted from
    /// theirs (whose set trick a player copied decides whose turn comes next, so a game that
    /// parted holds every round), and the challenge goes on without the player as after Leave
    /// Challenge. Each participant counts once, by the connection it signed in on. Every console
    /// then disconnects the player at the same tick: the facts the player sent before reach every
    /// one of them ahead of the leave, and none after it.
    fn stalled(&mut self, game_id: u32, reporter: u64, stalled: u64, tick: u32, now: Instant) {
        let Some(game) = self.games.get_mut(&game_id) else { return };
        if game.attributes.challenge_type != challenge::SKATE {
            return;
        }
        let GamePhase::Challenge { participants, stalls, .. } = &mut game.phase else { return };
        if reporter == stalled || !participants.contains(&reporter) || !participants.contains(&stalled) {
            return;
        }
        stalls.insert(reporter, (stalled, tick, now));
        let others: Vec<u64> = participants.iter().copied().filter(|&participant| participant != stalled).collect();
        let waiting = |participant: &u64| {
            stalls.get(participant).is_some_and(|&(player, at, when)| player == stalled && at == tick && now.saturating_duration_since(when) <= STALL_REPORT_FRESH)
        };
        if others.len() < 2 || !others.iter().all(waiting) {
            return;
        }
        stalls.clear();
        let name = game.member(stalled).map(|member| member.name.clone()).unwrap_or_default();
        (self.log)(&format!("{name} left game {game_id}'s challenge: every other player's round waited on their facts at tick {tick}"));
        self.broadcast(game_id, &ServerMessage::ChallengeLeave { user_id: stalled }, None);
        self.remove_participant(game_id, stalled, now);
    }

    /// A member is in the game's world once the server holds both its recipe and a replay frame,
    /// the moment the other members can see its skater: `ChatEnabled` tells it.
    fn mark_in_world(&mut self, game_id: u32, user_id: u64) {
        let Some(member) = self.games.get_mut(&game_id).and_then(|game| game.member_mut(user_id)) else { return };
        if member.in_world || member.recipe.is_none() || member.acked_own.is_none() {
            return;
        }
        member.in_world = true;
        let connection = member.connection;
        self.send(connection, &ServerMessage::ChatEnabled);
    }

    /// A member's chat line, to every member of its game (the sender too). The game is the one
    /// the server put the member in, so no line reaches another game; a line the game's prompt
    /// cannot make drops the connection; one sent before `ChatEnabled`, past the flood limit or
    /// holding a word of the server's list is dropped, and a chat-shadowbanned member's comes back
    /// to it alone.
    fn chat(&mut self, game_id: u32, user_id: u64, text: String, now: Instant) {
        let Some(member) = self.games.get_mut(&game_id).and_then(|game| game.member_mut(user_id)) else { return };
        if !valid_chat(&text) {
            let (connection, name) = (member.connection, member.name.clone());
            (self.log)(&format!("{name} sent a chat line the game cannot make ({} bytes): connection {connection} dropped", text.len()));
            self.drop_connection(connection, leave::KICKED_FOR_CHAT, now);
            return;
        }
        if !member.in_world {
            return;
        }
        while member.chat_sent.front().is_some_and(|&sent| now.saturating_duration_since(sent) >= CHAT_WINDOW) {
            member.chat_sent.pop_front();
        }
        if member.chat_sent.len() >= CHAT_LINES {
            return;
        }
        member.chat_sent.push_back(now);
        let (name, connection) = (member.name.clone(), member.connection);
        if self.words.holds_word(&text) {
            (self.log)(&format!("chat in game {game_id} from {name} held back: it holds a listed word"));
            return;
        }
        // A chat-shadowbanned member sees its line go through, as any line does; no one else sees
        // it (PC port addition).
        if self.address(connection).is_some_and(|address| self.chat_shadowbans.contains(address)) {
            (self.log)(&format!("chat in game {game_id} from {name}, chat-shadowbanned, to them alone: {text}"));
            self.send(connection, &ServerMessage::Chat { user_id, text });
            return;
        }
        (self.log)(&format!("chat in game {game_id} from {name}: {text}"));
        self.broadcast(game_id, &ServerMessage::Chat { user_id, text }, None);
    }

    /// A participant's facts or vote, to the challenge's other participants.
    fn relay_to_participants(&mut self, game_id: u32, user_id: u64, message: &ServerMessage) {
        let Some(game) = self.games.get(&game_id) else { return };
        if !matches!(game.phase, GamePhase::Challenge { .. }) || !game.participants().contains(&user_id) {
            return;
        }
        let targets: Vec<u32> = game
            .members
            .iter()
            .filter(|member| member.user_id != user_id && game.participants().contains(&member.user_id))
            .map(|member| member.connection)
            .collect();
        let bytes = message.encode();
        for target in targets {
            if let Some(connection) = self.connections.get_mut(&target) {
                connection.endpoint.send_reliable(&bytes);
            }
        }
    }

    fn game_request(&mut self, game_id: u32, user_id: u64, kind: u8, value: u64, now: Instant) {
        if !self.is_host(game_id, user_id) {
            return;
        }
        match kind {
            // FrontEndState_OnlineHubMenu::GoToStartChallenge 0x826617a0: two participants (one
            // for Spot Battle and Hall of Meat, `challenge::min_participants`), the members in the
            // game's world; one still on its way in stays in lobby skate, apart. No challenge
            // starts while the location changes.
            request::START_GAME => {
                let Some(game) = self.games.get_mut(&game_id) else { return };
                let participants: Vec<u64> = game.members.iter().filter(|member| member.in_world).map(|member| member.user_id).collect();
                if !matches!(game.phase, GamePhase::LobbySkate)
                    || participants.len() < challenge::min_participants(game.attributes.challenge_type)
                    || game.attributes.challenge_type == challenge::ONLINE_FREE_SKATE
                    || game.change.is_some()
                {
                    return;
                }
                let seed = self.random.next_u32();
                game.session = game.session.wrapping_add(1);
                let session = game.session;
                let slots = game.slots(&participants);
                game.phase = GamePhase::Loading { participants, loaded: BTreeSet::new(), deadline: now + LOAD_TIMEOUT };
                (self.log)(&format!("game {game_id}: challenge {session} loading"));
                self.broadcast(game_id, &ServerMessage::GameReset { kind: reset::LOAD_CHALLENGE, seed, session, participants: slots, activity: 0 }, None);
            }
            request::CHANGE_PRIVATE_STATUS => {
                let Some(game) = self.games.get_mut(&game_id) else { return };
                if game.attributes.private == (value != 0) {
                    return;
                }
                game.attributes.private = value != 0;
                if game.attributes.private {
                    game.attributes.ranked = false;
                }
                self.broadcast(game_id, &ServerMessage::PrivacyChanged { private: value != 0 }, None);
            }
            request::KICK_PLAYER => {
                let Some((target, name)) = self.games.get(&game_id).and_then(|game| game.member(value)).map(|member| (member.connection, member.name.clone())) else { return };
                if value == user_id {
                    return;
                }
                // The kicked player's address stays out of the game, under any name (PC port
                // addition).
                if let Some(address) = self.address(target)
                    && let Some(game) = self.games.get_mut(&game_id)
                {
                    game.kicked.insert(address);
                    (self.log)(&format!("game {game_id}'s host kicked {name}: the game takes no one from {address} again"));
                }
                self.send(target, &ServerMessage::Kicked { reason: leave::KICKED_BY_HOST });
                self.leave_game(target, leave::KICKED_BY_HOST, now);
            }
            _ => {}
        }
    }

    /// `GoToStartFreeskateActivity` 0x8266c728's `Message_GameProposal(PT_Propose)` (PC port
    /// addition: the original's game server is not in the game): in an online free skate's lobby
    /// skate, with no change of location counting down, no activity proposed and at least two
    /// members in its world, the proposer one of them, the activity is proposed to every member in
    /// its world (`PT_Broadcast`, with the time left to accept), the proposer having accepted.
    fn propose_activity(&mut self, game_id: u32, user_id: u64, activity: u64, now: Instant) {
        let Some(game) = self.games.get_mut(&game_id) else { return };
        let invited: BTreeSet<u64> = game.in_world().into_iter().collect();
        if game.proposal.is_some() {
            return;
        }
        if game.attributes.challenge_type != challenge::ONLINE_FREE_SKATE
            || !matches!(game.phase, GamePhase::LobbySkate)
            || game.change.is_some()
            || activity == 0
            || !invited.contains(&user_id)
            || invited.len() < 2
        {
            // The proposer hears its proposal went nowhere, as one no one accepted does; with
            // another under way it hears that one's broadcast instead.
            if let Some(connection) = game.member(user_id).map(|member| member.connection) {
                self.send(connection, &ServerMessage::GameProposal { kind: proposal::TIMEOUT, activity, user_id: 0, timeout: 0 });
            }
            return;
        }
        let name = game.member(user_id).map(|member| member.name.clone()).unwrap_or_default();
        game.proposal = Some(Proposal {
            activity,
            proposer: user_id,
            invited,
            accepted: BTreeSet::from([user_id]),
            deadline: now + PROPOSAL_TIME,
            accepted_at: None,
        });
        (self.log)(&format!("{name} proposed the activity {} in game {game_id}", stats::challenge_name(activity)));
        let message = ServerMessage::GameProposal { kind: proposal::BROADCAST, activity, user_id, timeout: millis(PROPOSAL_TIME) };
        self.broadcast(game_id, &message, None);
    }

    /// `PT_Accept` of the proposed activity: every member hears who accepted it
    /// (`ID_ONLINE_EVENT_FREESKATE_ACCEPTED`), and it starts once every member it was proposed to
    /// has.
    fn accept_activity(&mut self, game_id: u32, user_id: u64, activity: u64, now: Instant) {
        let Some(game) = self.games.get_mut(&game_id) else { return };
        let Some(pending) = game.proposal.as_mut().filter(|pending| pending.activity == activity) else { return };
        if !pending.accepted.insert(user_id) {
            return;
        }
        let message = ServerMessage::GameProposal { kind: proposal::ACCEPT, activity, user_id, timeout: 0 };
        self.broadcast(game_id, &message, None);
        self.advance(game_id, now);
    }

    /// The proposed activity in lobby skate: the members it was proposed to who left no longer
    /// count; it ends unstarted (`PT_Timeout`) once its time runs out, its proposer leaves or
    /// fewer than two members are in the game's world. Once every member it was proposed to has
    /// accepted, it loads for every member in the game's world, waiting at most [`ARRIVAL_WAIT`]
    /// for those on their way in; a member not in it by then missed it (`PT_Missed`).
    fn advance_proposal(&mut self, game_id: u32, now: Instant) {
        let Some(game) = self.games.get_mut(&game_id) else { return };
        if !matches!(game.phase, GamePhase::LobbySkate) {
            return;
        }
        let in_world = game.in_world();
        let arriving = game.members.iter().any(|member| !member.in_world);
        let Some(pending) = game.proposal.as_mut() else { return };
        pending.invited.retain(|invited| game.members.iter().any(|member| member.user_id == *invited));
        let accepted = pending.invited.iter().all(|invited| pending.accepted.contains(invited));
        let activity = pending.activity;
        let over = !in_world.contains(&pending.proposer) || in_world.len() < 2 || (!accepted && now >= pending.deadline);
        if over {
            game.proposal = None;
            (self.log)(&format!("game {game_id}: the activity {} was not accepted", stats::challenge_name(activity)));
            self.broadcast(game_id, &ServerMessage::GameProposal { kind: proposal::TIMEOUT, activity, user_id: 0, timeout: 0 }, None);
            return;
        }
        if !accepted {
            return;
        }
        let since = *pending.accepted_at.get_or_insert(now);
        if arriving && now.saturating_duration_since(since) < ARRIVAL_WAIT {
            return;
        }
        game.proposal = None;
        game.activity = Some(activity);
        let seed = self.random.next_u32();
        game.session = game.session.wrapping_add(1);
        let session = game.session;
        let slots = game.slots(&in_world);
        let missed: Vec<u32> = game.members.iter().filter(|member| !in_world.contains(&member.user_id)).map(|member| member.connection).collect();
        game.phase = GamePhase::Loading { participants: in_world, loaded: BTreeSet::new(), deadline: now + LOAD_TIMEOUT };
        (self.log)(&format!("game {game_id}: the activity {} loading ({session})", stats::challenge_name(activity)));
        self.broadcast(game_id, &ServerMessage::GameReset { kind: reset::LOAD_CHALLENGE, seed, session, participants: slots, activity }, None);
        for connection in missed {
            self.send(connection, &ServerMessage::GameProposal { kind: proposal::MISSED, activity, user_id: 0, timeout: 0 });
        }
    }

    /// A participant left: the barriers stop waiting for them.
    fn remove_participant(&mut self, game_id: u32, user_id: u64, now: Instant) {
        if let Some(game) = self.games.get_mut(&game_id) {
            match &mut game.phase {
                GamePhase::Loading { participants, loaded, .. } => {
                    participants.retain(|participant| *participant != user_id);
                    loaded.remove(&user_id);
                }
                GamePhase::Challenge { participants, rounds, complete, results, stalls, .. } => {
                    participants.retain(|participant| *participant != user_id);
                    for (arrived, _) in rounds.values_mut() {
                        arrived.remove(&user_id);
                    }
                    complete.remove(&user_id);
                    results.remove(&user_id);
                    stalls.remove(&user_id);
                    stalls.retain(|_, &mut (stalled, _, _)| stalled != user_id);
                }
                GamePhase::Results { participants, .. } | GamePhase::NextChallenge { participants, .. } => {
                    participants.retain(|participant| *participant != user_id);
                }
                GamePhase::LobbySkate => {}
            }
        }
        self.advance(game_id, now);
    }

    /// Moves a game on once its barrier is met or timed out.
    fn advance(&mut self, game_id: u32, now: Instant) {
        let Some(game) = self.games.get_mut(&game_id) else { return };
        let fewest = challenge::min_participants(game.attributes.challenge_type);
        match &mut game.phase {
            GamePhase::Loading { participants, loaded, deadline } => {
                let everyone = participants.iter().all(|participant| loaded.contains(participant));
                if !(everyone || now >= *deadline) {
                    return;
                }
                participants.retain(|participant| loaded.contains(participant));
                if participants.len() < fewest {
                    (self.log)(&format!("game {game_id}: challenge cancelled, too few players loaded"));
                    game.phase = GamePhase::LobbySkate;
                    game.activity = None;
                    let session = game.session;
                    self.broadcast(game_id, &ServerMessage::GameReset { kind: reset::LOBBY_SKATE, seed: 0, session, participants: [0; MAX_PLAYERS], activity: 0 }, None);
                    return;
                }
                let participants = std::mem::take(participants);
                let slots = game.slots(&participants);
                let seed = self.random.next_u32();
                let session = game.session;
                let activity = game.activity.unwrap_or(0);
                game.phase = GamePhase::Challenge {
                    participants,
                    rounds: BTreeMap::new(),
                    complete: BTreeSet::new(),
                    results: BTreeMap::new(),
                    all_complete: false,
                    deadline: None,
                    stalls: BTreeMap::new(),
                };
                (self.log)(&format!("game {game_id}: challenge {session} started"));
                self.broadcast(game_id, &ServerMessage::GameReset { kind: reset::CHALLENGE, seed, session, participants: slots, activity }, None);
            }
            GamePhase::Challenge { participants, rounds, complete, results, all_complete, deadline, .. } => {
                if participants.is_empty() {
                    (self.log)(&format!("game {game_id}: challenge {} ended, every participant left it", game.session));
                    game.phase = GamePhase::LobbySkate;
                    game.activity = None;
                    let session = game.session;
                    self.broadcast(game_id, &ServerMessage::GameReset { kind: reset::LOBBY_SKATE, seed: 0, session, participants: [0; MAX_PLAYERS], activity: 0 }, None);
                    return;
                }
                let released: Vec<u32> = rounds
                    .iter()
                    .filter(|(_, (arrived, round_deadline))| participants.iter().all(|participant| arrived.contains(participant)) || now >= *round_deadline)
                    .map(|(round, _)| *round)
                    .collect();
                for round in &released {
                    rounds.remove(round);
                }
                let timed_out = deadline.is_some_and(|deadline| now >= deadline);
                let send_complete = !*all_complete && (participants.iter().all(|participant| complete.contains(participant)) || timed_out);
                if send_complete {
                    *all_complete = true;
                    *deadline = Some(now + RESULTS_TIMEOUT);
                }
                let finished = *all_complete && (participants.iter().all(|participant| results.contains_key(participant)) || deadline.is_some_and(|deadline| now >= deadline));
                let rows: Option<Vec<PlayerStats>> = finished.then(|| {
                    participants
                        .iter()
                        .map(|participant| PlayerStats {
                            user_id: *participant,
                            result: results.get(participant).copied().unwrap_or_default(),
                            wager: -1,
                            ..PlayerStats::default()
                        })
                        .collect()
                });
                if rows.is_some() {
                    let participants = participants.clone();
                    game.phase = GamePhase::Results { participants, until: now + Duration::from_secs(DISPLAY_RESULTS) };
                }
                for round in released {
                    self.broadcast(game_id, &ServerMessage::SyncPoint { round }, None);
                }
                if send_complete {
                    self.broadcast(game_id, &ServerMessage::AllPlayersComplete, None);
                }
                if let Some(rows) = rows {
                    (self.log)(&format!("game {game_id}: results"));
                    self.broadcast(game_id, &ServerMessage::FinalResults(rows), None);
                    self.broadcast(game_id, &ServerMessage::GameTimer { kind: timer::DISPLAY_RESULTS, value: DISPLAY_RESULTS }, None);
                }
            }
            GamePhase::Results { participants, until } => {
                if now >= *until {
                    let participants = std::mem::take(participants);
                    game.phase = GamePhase::NextChallenge { participants, until: now + NEXT_CHALLENGE_GRACE };
                    self.broadcast(game_id, &ServerMessage::ExitPostChallenge, None);
                }
            }
            GamePhase::NextChallenge { until, .. } => {
                if now >= *until {
                    self.next_challenge(game_id, None, now);
                }
            }
            GamePhase::LobbySkate => {
                if let Some((challenge_type, challenge_key, at)) = game.change
                    && now >= at
                {
                    game.change = None;
                    game.attributes.challenge_type = challenge_type;
                    game.attributes.challenge_key = challenge_key;
                    let attributes = game.attributes.clone();
                    let session = game.session;
                    self.broadcast(game_id, &ServerMessage::GameAttributes(attributes), None);
                    self.broadcast(game_id, &ServerMessage::GameReset { kind: reset::LOBBY_SKATE, seed: 0, session, participants: [0; MAX_PLAYERS], activity: 0 }, None);
                }
                self.advance_proposal(game_id, now);
            }
        }
    }

    /// The host's `GameResetAttributes` (or none in time): the next challenge's attributes and
    /// back to lobby skate.
    fn next_challenge(&mut self, game_id: u32, attributes: Option<GameAttributes>, _now: Instant) {
        let Some(game) = self.games.get_mut(&game_id) else { return };
        if !matches!(game.phase, GamePhase::Results { .. } | GamePhase::NextChallenge { .. }) {
            return;
        }
        if let Some(attributes) = attributes
            && attributes.challenge_type == game.attributes.challenge_type
        {
            game.attributes.challenge_key = attributes.challenge_key;
        }
        game.phase = GamePhase::LobbySkate;
        game.activity = None;
        let attributes = game.attributes.clone();
        let session = game.session;
        self.broadcast(game_id, &ServerMessage::GameAttributes(attributes), None);
        self.broadcast(game_id, &ServerMessage::GameReset { kind: reset::LOBBY_SKATE, seed: 0, session, participants: [0; MAX_PLAYERS], activity: 0 }, None);
    }

    /// A player's run of frames: kept as baselines, then coded for each other player against the
    /// frames that player holds. Until the server holds the player's recipe its runs go nowhere and
    /// are not acknowledged: the game sends its recipe with its first runs, and no one could show
    /// the skater without it. Every member sees every other (PC port addition): from a
    /// challenge's load until the game is back in lobby skate, its participants and the members in
    /// lobby skate (who joined during it, were not yet in the world at its start, did not load it,
    /// or left it) each show the other side as ghosts, where the original showed neither the other.
    fn relay_frames(&mut self, id: u32, user_id: u64, run: &crate::frames::EncodedRun, now: Instant) {
        let Some(game_id) = self.game_of(id) else { return };
        let Some(game) = self.games.get_mut(&game_id) else { return };
        let Some(source) = game.members.iter().position(|member| member.user_id == user_id && member.recipe.is_some()) else { return };
        let decoded = match game.members[source].frames.decode(run) {
            Ok(Some(decoded)) => decoded,
            Ok(None) => return,
            Err(error) => {
                let member = &mut game.members[source];
                if !std::mem::replace(&mut member.bad_frames_logged, true) {
                    (self.log)(&format!("{}: bad frames: {error}", member.name));
                }
                return;
            }
        };
        game.members[source].acked_own = game.members[source].frames.ack();
        let slot = game.members[source].slot;
        let mut sends = Vec::new();
        for member in &game.members {
            if member.user_id == user_id || game.hidden_from(user_id, member.user_id) {
                continue;
            }
            let acks = member.holds.get(&user_id).copied().unwrap_or_default();
            if let Some(run) = encode_for(&decoded, game.members[source].frames.ring(), &acks) {
                sends.push((member.connection, ServerMessage::RemoteFrames { slot, run }));
            }
        }
        let ack = game.members[source].acked_own.map(|(latest, mask)| ServerMessage::FrameAck { latest, mask });
        for (target, message) in sends {
            self.send_unreliable(target, &message);
        }
        if let Some(ack) = ack
            && let Some(connection) = self.connections.get_mut(&id)
        {
            connection.endpoint.send_deferred(ack.encode(), now);
        }
        self.mark_in_world(game_id, user_id);
    }

    fn frame_acks(&mut self, id: u32, user_id: u64, acks: &[(u8, u32, u32)]) {
        let Some(game_id) = self.game_of(id) else { return };
        let Some(game) = self.games.get_mut(&game_id) else { return };
        // An ack names the newest frame the player holds of a slot's member, one the server
        // relayed: one of a frame the server holds none of is about the slot's member before (a
        // player who left, acknowledged before the player heard so) or too old to code against.
        let sources: Vec<(u64, u32, u32)> = acks
            .iter()
            .filter_map(|&(slot, latest, mask)| {
                let source = game.members.iter().find(|member| member.slot == slot)?;
                source.frames.ring().get(latest).map(|_| (source.user_id, latest, mask))
            })
            .collect();
        let Some(member) = game.member_mut(user_id) else { return };
        for (source, latest, mask) in sources {
            member.holds.entry(source).or_default().update(latest, mask);
        }
    }

    /// Timers and connection upkeep, then every connection's datagrams.
    pub fn update(&mut self, now: Instant) {
        if self.bans_checked.is_none_or(|checked| now.saturating_duration_since(checked) >= BAN_CHECK) {
            self.refresh_bans(now);
        }
        let lost: Vec<u32> = self.connections.iter().filter(|(_, c)| c.endpoint.timed_out(now)).map(|(&id, _)| id).collect();
        for id in lost {
            self.drop_connection(id, leave::LOST_CONNECTION, now);
        }
        let unsigned: Vec<u32> = self
            .connections
            .iter()
            .filter(|(_, c)| c.user.is_none() && now.saturating_duration_since(c.opened) >= SIGN_IN_TIMEOUT)
            .map(|(&id, _)| id)
            .collect();
        for id in unsigned {
            (self.log)(&format!("connection {id} never signed in: dropped"));
            self.drop_connection(id, leave::KICKED_BEFORE_SIGN_IN, now);
        }
        let games: Vec<u32> = self.games.keys().copied().collect();
        for game in games {
            self.advance(game, now);
        }
        // A game that stops sending replay frames, without saying it has none to send, is
        // frozen or paused: `KR_KickedForInactivity`.
        let inactive: Vec<(u32, String)> = self
            .games
            .values()
            .flat_map(|game| game.members.iter().filter(|member| now.saturating_duration_since(member.alive) >= INACTIVITY))
            .map(|member| (member.connection, member.name.clone()))
            .collect();
        for (id, name) in inactive {
            (self.log)(&format!("{name} sent nothing for {} s: kicked for inactivity", INACTIVITY.as_secs()));
            self.send(id, &ServerMessage::Kicked { reason: leave::KICKED_FOR_INACTIVITY });
            self.leave_game(id, leave::KICKED_FOR_INACTIVITY, now);
        }
        for connection in self.connections.values_mut() {
            for datagram in connection.endpoint.poll(now) {
                self.outgoing.push((connection.addr, datagram));
            }
        }
    }

    /// Reads the ban lists' files again where they changed; an address the IP bans now hold
    /// loses its connections.
    fn refresh_bans(&mut self, now: Instant) {
        self.bans_checked = Some(now);
        if let Some(line) = self.chat_shadowbans.refresh() {
            (self.log)(&line);
        }
        if let Some(line) = self.ip_bans.refresh() {
            (self.log)(&line);
            self.close_banned(now);
        }
    }

    /// Closes every connection from a banned address without a word to it: the server only stops
    /// answering, and its game's other members see it lose its connection. The connections closed.
    fn close_banned(&mut self, now: Instant) -> usize {
        let banned: Vec<u32> =
            self.connections.iter().filter(|(_, connection)| self.ip_bans.contains(connection.addr.ip())).map(|(&id, _)| id).collect();
        for &id in &banned {
            self.leave_game(id, leave::LOST_CONNECTION, now);
            if let Some(connection) = self.connections.remove(&id) {
                self.by_addr.remove(&connection.addr);
                (self.log)(&format!("connection {id} from {} closed: its address is banned", connection.addr));
            }
        }
        banned.len()
    }

    /// The moderation panel's request (PC port addition), answered on its channel.
    pub fn moderate(&mut self, request: panel::Request, now: Instant) {
        match request {
            panel::Request::View(reply) => {
                let _ = reply.send(self.moderation_view(now));
            }
            panel::Request::Act(action, reply) => {
                let outcome = self.moderation_action(action, now);
                let _ = reply.send(outcome);
            }
            panel::Request::Stats(reply) => {
                let _ = reply.send(self.public_stats());
            }
        }
    }

    /// The public statistics (PC port addition): how many players are signed in, and every game
    /// that is not private with its players' nicknames in join order, oldest game first.
    fn public_stats(&self) -> stats::Stats {
        let games = self
            .games
            .values()
            .filter(|game| !game.attributes.private && !game.members.is_empty())
            .map(|game| stats::PublicGame {
                challenge_type: game.attributes.challenge_type,
                challenge_key: game.attributes.challenge_key,
                activity: game.activity(),
                in_progress: !matches!(game.phase, GamePhase::LobbySkate),
                players: game.members.iter().map(|member| member.name.clone()).collect(),
            })
            .collect();
        stats::Stats { players_online: self.players(), games }
    }

    /// The signed-in players, by name, and the bans in force.
    fn moderation_view(&self, now: Instant) -> panel::View {
        let mut players: Vec<panel::Player> = self
            .connections
            .values()
            .filter_map(|connection| {
                let user = connection.user.as_ref()?;
                Some(panel::Player {
                    name: user.name.clone(),
                    address: moderation::address(connection.addr.ip()),
                    version: user.version.clone(),
                    game: user.game.map(|game| (game, self.is_host(game, user.id))),
                    online: now.saturating_duration_since(connection.opened),
                    chat_shadowbanned: self.chat_shadowbans.contains(connection.addr.ip()),
                })
            })
            .collect();
        players.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()).then(a.address.cmp(&b.address)));
        let bans = |list: &BanList| -> Vec<(IpAddr, String)> { list.iter().map(|(address, name)| (address, name.to_owned())).collect() };
        panel::View { players, games: self.games.len(), chat_shadowbans: bans(&self.chat_shadowbans), ip_bans: bans(&self.ip_bans) }
    }

    /// Does what the panel asks: what came of it, for the page. A list's file edited by hand since
    /// it was last read is read first, so the action does not write over the edit.
    fn moderation_action(&mut self, action: panel::Action, now: Instant) -> String {
        self.refresh_bans(now);
        let mut outcome = Vec::new();
        match action {
            panel::Action::ChatShadowban(players) => {
                for (address, name) in players {
                    let written = self.chat_shadowbans.insert(address, &name);
                    outcome.push(self.moderated(
                        written,
                        address,
                        format!("{name} is chat-shadowbanned"),
                        format!("{name} was chat-shadowbanned already."),
                        moderation::CHAT_SHADOWBANS_FILE,
                    ));
                }
            }
            panel::Action::IpBan(players) => {
                for (address, name) in players {
                    let written = self.ip_bans.insert(address, &name);
                    outcome.push(self.moderated(
                        written,
                        address,
                        format!("{name} is IP-banned"),
                        format!("{name} was IP-banned already."),
                        moderation::IP_BANS_FILE,
                    ));
                }
                match self.close_banned(now) {
                    0 => {}
                    1 => outcome.push("1 connection closed.".into()),
                    closed => outcome.push(format!("{closed} connections closed.")),
                }
            }
            panel::Action::Lift { chat, ip } => {
                for address in chat {
                    outcome.push(match self.chat_shadowbans.remove(address) {
                        Some((name, written)) => self.moderated(
                            Some(written),
                            address,
                            format!("The chat shadowban on {name} is lifted"),
                            String::new(),
                            moderation::CHAT_SHADOWBANS_FILE,
                        ),
                        None => "A ticked chat shadowban was not in force.".to_owned(),
                    });
                }
                for address in ip {
                    outcome.push(match self.ip_bans.remove(address) {
                        Some((name, written)) => {
                            self.moderated(Some(written), address, format!("The IP ban on {name} is lifted"), String::new(), moderation::IP_BANS_FILE)
                        }
                        None => "A ticked IP ban was not in force.".to_owned(),
                    });
                }
            }
        }
        outcome.join(" ")
    }

    /// A ban's or lift's outcome for the page, which shows no address, logged with the address
    /// when it changed a list: `unchanged` when there was nothing to change; when its file could
    /// not be written, the change holds until the server stops.
    fn moderated(&mut self, written: Option<std::io::Result<()>>, address: IpAddr, change: String, unchanged: String, file: &str) -> String {
        match written {
            None => unchanged,
            Some(Ok(())) => {
                (self.log)(&format!("moderation panel: {change} ({address})"));
                format!("{change}.")
            }
            Some(Err(error)) => {
                (self.log)(&format!("moderation panel: {change} ({address}) until the server stops: {file} could not be written ({error})"));
                format!("{change} until the server stops: {file} could not be written ({error}).")
            }
        }
    }
}

/// How long a member may send neither a replay frame nor `Busy` before it is kicked.
const INACTIVITY: Duration = Duration::from_secs(2);

/// How long after a member's recipe the server takes another of it.
const RECIPE_SPACING: Duration = Duration::from_secs(2);

/// How often the ban lists' files are looked at for changes made by hand.
const BAN_CHECK: Duration = Duration::from_secs(1);

/// How often the server prints its player, game and traffic counts.
const STATUS_INTERVAL: Duration = Duration::from_secs(60);

/// The datagrams one pass of the server reads before it sends what they made: past this a stream
/// of datagrams would hold every reply back.
const MAX_BATCH: usize = 64;

/// Lines waiting for the console, past which the newest are dropped.
const CONSOLE_BACKLOG: usize = 1024;

/// The server's console: a thread of its own prints the lines, so a console that stops taking
/// them (a selection held in a Windows console, Ctrl+S, a terminal that closed) neither holds the
/// server up nor ends it. Past [`CONSOLE_BACKLOG`] waiting lines the newest are dropped.
#[derive(Clone)]
struct Console(Option<std::sync::mpsc::SyncSender<String>>);

impl Console {
    fn start() -> Self {
        let (lines, queue) = std::sync::mpsc::sync_channel::<String>(CONSOLE_BACKLOG);
        let printer = std::thread::Builder::new().name("console".into()).spawn(move || {
            let mut stdout = std::io::stdout();
            for line in queue {
                let _ = writeln!(stdout, "{line}");
            }
        });
        Self(printer.is_ok().then_some(lines))
    }

    fn line(&self, line: &str) {
        match &self.0 {
            Some(lines) => {
                let _ = lines.try_send(line.to_owned());
            }
            None => {
                let _ = writeln!(std::io::stdout(), "{line}");
            }
        }
    }
}

/// Datagrams and payload bytes since the last status line, and the datagrams the socket could
/// not hand over.
#[derive(Default)]
struct Traffic {
    received: (u64, u64),
    sent: (u64, u64),
    unreadable: u64,
}

impl Traffic {
    fn received(&mut self, length: usize) {
        self.received.0 += 1;
        self.received.1 += length as u64;
    }

    fn sent(&mut self, length: usize) {
        self.sent.0 += 1;
        self.sent.1 += length as u64;
    }

    fn report(&self, elapsed: Duration) -> String {
        let seconds = elapsed.as_secs_f64().max(1.0);
        #[expect(clippy::cast_precision_loss, reason = "rates for display")]
        let rate = |(count, bytes): (u64, u64)| format!("{:.1} KB/s ({:.0} datagrams/s)", bytes as f64 / 1024.0 / seconds, count as f64 / seconds);
        let unreadable = if self.unreadable == 0 { String::new() } else { format!(", {} unreadable", self.unreadable) };
        format!("in {}, out {}{unreadable}", rate(self.received), rate(self.sent))
    }
}

/// How the server listens.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    /// The build the server is; clients of others are refused.
    pub build: Build,
}

/// Runs the server until the process ends.
///
/// # Errors
/// The socket could not be bound or set up.
pub fn run(config: &ServerConfig) -> std::io::Result<()> {
    let socket = UdpSocket::bind(config.bind)?;
    socket.set_nonblocking(true)?;
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0x5eed, |time| {
        #[expect(clippy::cast_possible_truncation, reason = "the low bits seed the generator")]
        let nanos = time.as_nanos() as u64;
        nanos
    });
    let console = Console::start();
    let log = console.clone();
    let mut core = ServerCore::new(seed, config.build, Box::new(move |line| log.line(line)));
    console.line(&format!("skate2-pc dedicated server listening on {} (build {})", socket.local_addr()?, config.build.id()));
    let words = WordList::beside_server();
    let path = words.path().map(|path| path.display().to_string()).unwrap_or_default();
    console.line(&if words.is_empty() {
        format!("word list {path}: no words, so nicknames and chat are not filtered until it has some")
    } else {
        format!("word list {path}: {} words", words.len())
    });
    core.set_words(words);
    let folder = moderation::folder();
    let mut chat_shadowbans = BanList::from_file(folder.join(moderation::CHAT_SHADOWBANS_FILE));
    let mut ip_bans = BanList::from_file(folder.join(moderation::IP_BANS_FILE));
    for list in [&mut chat_shadowbans, &mut ip_bans] {
        let read = list.refresh();
        let path = list.path().map(|path| path.display().to_string()).unwrap_or_default();
        console.line(&read.unwrap_or_else(|| format!("{path}: no bans")));
    }
    core.set_bans(chat_shadowbans, ip_bans);
    let panel = start_panel(config.bind, &folder, &console);
    let mut buffer = [0_u8; 2048];
    let mut last_status = Instant::now();
    let mut traffic = Traffic::default();
    loop {
        let mut received = false;
        for _ in 0..MAX_BATCH {
            match socket.recv_from(&mut buffer) {
                Ok((length, addr)) => {
                    received = true;
                    traffic.received(length);
                    core.receive(addr, &buffer[..length], Instant::now());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                // Windows reports an earlier datagram's ICMP port-unreachable here.
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
                // A datagram the socket could not hand over whole (Windows refuses one longer than
                // the buffer, WSAEMSGSIZE): lost, as any other.
                Err(_) => traffic.unreadable += 1,
            }
        }
        let now = Instant::now();
        core.update(now);
        if let Some(requests) = &panel {
            while let Ok(request) = requests.try_recv() {
                core.moderate(request, now);
            }
        }
        for (addr, datagram) in core.take_outgoing() {
            traffic.sent(datagram.len());
            // A send that fails (an unreachable peer) is a lost datagram like any other.
            let _ = socket.send_to(&datagram, addr);
        }
        let elapsed = now.saturating_duration_since(last_status);
        if elapsed >= STATUS_INTERVAL {
            last_status = now;
            console.line(&format!("{} players online, {} games, {}", core.players(), core.games(), traffic.report(elapsed)));
            traffic = Traffic::default();
        }
        if !received {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// Opens the moderation panel at the server's address when `server-config.json` names its port,
/// with the public statistics; without a key in `moderation-key.txt` only the statistics: the
/// requests it sends the server's loop.
fn start_panel(bind: SocketAddr, folder: &Path, console: &Console) -> Option<std::sync::mpsc::Receiver<panel::Request>> {
    let config = folder.join(moderation::CONFIG_FILE);
    let port = match moderation::panel_port(&config) {
        Ok(Some(port)) => port,
        Ok(None) => {
            console.line(&format!("moderation panel and server stats: off, as {} names no moderation_panel_port", config.display()));
            return None;
        }
        Err(why) => {
            console.line(&format!("moderation panel and server stats: off, as {why}"));
            return None;
        }
    };
    let key = match PanelKey::read(&folder.join(moderation::KEY_FILE)) {
        Ok(key) => Some(key),
        Err(why) => {
            console.line(&format!("moderation panel: off, as {why}"));
            None
        }
    };
    let panel_on = key.is_some();
    let (requests, receiver) = std::sync::mpsc::channel();
    let log = console.clone();
    match panel::start(SocketAddr::new(bind.ip(), port), key, requests, Arc::new(move |line: &str| log.line(line))) {
        Ok(address) => {
            if panel_on {
                console.line(&format!("moderation panel on http://{address}{}?key=<the key in {}>", panel::PATH, moderation::KEY_FILE));
            }
            console.line(&format!("server stats on http://{address}{}", panel::STATS_PATH));
            Some(receiver)
        }
        Err(error) => {
            console.line(&format!("moderation panel and server stats: off, as they could not start on port {port} ({error})"));
            None
        }
    }
}

/// Where the server keeps a file of its own (`badwords.txt`, `server-config.json`): beside its
/// executable or, when only the working directory holds the file, there.
#[must_use]
pub fn server_file(name: &str) -> PathBuf {
    let beside = std::env::current_exe().ok().and_then(|executable| Some(executable.parent()?.join(name)));
    let working = std::env::current_dir().ok().map(|directory| directory.join(name));
    match (beside, working) {
        (Some(beside), Some(working)) if !beside.is_file() && working.is_file() => working,
        (Some(beside), _) => beside,
        (None, working) => working.unwrap_or_else(|| PathBuf::from(name)),
    }
}

/// Parses `ip:port`, `ip` or `:port` into the address to bind.
#[must_use]
pub fn parse_bind(text: Option<&str>) -> Option<SocketAddr> {
    let text = text.unwrap_or("").trim();
    if text.is_empty() {
        return Some(SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)));
    }
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Some(addr);
    }
    if let Some(port) = text.strip_prefix(':') {
        return Some(SocketAddr::from(([0, 0, 0, 0], port.parse().ok()?)));
    }
    let ip: std::net::IpAddr = text.parse().ok()?;
    Some(SocketAddr::new(ip, DEFAULT_PORT))
}
