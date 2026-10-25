//! The client and server cores talking over a simulated network.

use crate::client::{ClientCore, ClientEvent, Disconnect};
use crate::crypto::{Build, Handshake, Session};
use crate::frames::{HEADER_WORDS, INTERVAL, NetFrame};
use crate::protocol::{
    ClientMessage, FactsNotes, GameAttributes, GameResult, LobbyInfo, MAX_PLAYERS, Phase, PlayNow, PlayNowMode, ServerMessage, challenge, leave, play_now, refuse,
    request, reset,
};
use crate::moderation::BanList;
use crate::panel::{Action, Request};
use crate::server::ServerCore;
use crate::transport::{self, Endpoint, kind};
use crate::words::WordList;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

struct Net {
    server: ServerCore,
    clients: Vec<(SocketAddr, ClientCore, Vec<ClientEvent>)>,
    /// Every datagram either way, as a packet capture would see it.
    wire: Vec<Vec<u8>>,
    /// The bytes the server sent each address.
    received_bytes: HashMap<SocketAddr, usize>,
    now: Instant,
    /// Milliseconds run, for each client's `Busy` every quarter second (a game with no frame
    /// to send).
    elapsed: u64,
}

impl Net {
    fn new() -> Self {
        Self {
            server: ServerCore::new(7, Build::DEVELOPMENT, Box::new(|_| {})),
            clients: Vec::new(),
            wire: Vec::new(),
            received_bytes: HashMap::new(),
            now: Instant::now(),
            elapsed: 0,
        }
    }

    fn add(&mut self, name: &str) -> usize {
        self.add_build(name, Build::DEVELOPMENT)
    }

    fn add_build(&mut self, name: &str, build: Build) -> usize {
        let host = u8::try_from(self.clients.len() + 1).expect("few clients");
        self.add_at(name, build, SocketAddr::from(([10, 0, 0, host], 4000)))
    }

    fn add_at(&mut self, name: &str, build: Build, addr: SocketAddr) -> usize {
        let core = ClientCore::new(self.clients.len() as u64 + 100, name, "test", build, self.now);
        self.clients.push((addr, core, Vec::new()));
        self.clients.len() - 1
    }

    /// Runs the network for `ms` milliseconds in 5 ms steps.
    fn run(&mut self, ms: u64) {
        for _ in 0..ms / 5 {
            self.now += Duration::from_millis(5);
            self.elapsed += 5;
            for (addr, core, _) in &mut self.clients {
                if self.elapsed % 250 == 0 {
                    core.send(&ClientMessage::Busy);
                }
                core.update(self.now);
                for datagram in core.take_outgoing() {
                    self.server.receive(*addr, &datagram, self.now);
                    self.wire.push(datagram);
                }
            }
            self.server.update(self.now);
            for (addr, datagram) in self.server.take_outgoing() {
                if let Some((_, core, _)) = self.clients.iter_mut().find(|(client, _, _)| *client == addr) {
                    core.receive(&datagram, self.now);
                }
                *self.received_bytes.entry(addr).or_default() += datagram.len();
                self.wire.push(datagram);
            }
            for (_, core, events) in &mut self.clients {
                events.extend(core.take_events());
            }
        }
    }

    fn send(&mut self, client: usize, message: &ClientMessage) {
        self.clients[client].1.send(message);
    }

    fn events(&mut self, client: usize) -> Vec<ClientEvent> {
        std::mem::take(&mut self.clients[client].2)
    }

    fn messages(&mut self, client: usize) -> Vec<ServerMessage> {
        self.events(client)
            .into_iter()
            .filter_map(|event| match event {
                ClientEvent::Message(message) => Some(message),
                _ => None,
            })
            .collect()
    }
}

fn play(mode: PlayNowMode, create: bool, key: Option<u64>) -> ClientMessage {
    ClientMessage::PlayNow(PlayNow {
        mode,
        ranked: false,
        challenge_type: challenge::SKATE,
        challenge_key: key,
        private: false,
        create,
        fallback_key: 0xabc,
        game_version: "test".into(),
    })
}

/// A free skate search: Join (or Create) at `key`, or at any location.
fn free_skate(create: bool, key: Option<u64>) -> ClientMessage {
    ClientMessage::PlayNow(PlayNow {
        mode: PlayNowMode::Freeskate,
        ranked: false,
        challenge_type: challenge::ONLINE_FREE_SKATE,
        challenge_key: key,
        private: false,
        create,
        fallback_key: 0xabc,
        game_version: "test".into(),
    })
}

fn frame(tick: u32) -> NetFrame {
    let mut header = [0; HEADER_WORDS];
    header[48] = tick;
    NetFrame {
        tick,
        header,
        pose: (tick % INTERVAL == INTERVAL - 1).then(|| vec![[1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 1.0]; 133]),
        twist: [0.5; 4],
        camera: vec![1.0; 8],
        audio_floats: vec![0.25; 64],
        audio_words: vec![3; 16],
        hud: vec![0; 15],
    }
}

#[test]
fn players_find_each_other_and_play_a_challenge() {
    let mut net = Net::new();
    let host = net.add("Host");
    let guest = net.add("Guest");
    net.run(100);
    let user_of = |events: Vec<ClientEvent>| {
        events
            .into_iter()
            .find_map(|event| match event {
                ClientEvent::SignedIn { user_id } => Some(user_id),
                _ => None,
            })
            .expect("signed in")
    };
    let host_id = user_of(net.events(host));
    let guest_id = user_of(net.events(guest));

    // A quick match finds nothing and creates nothing.
    net.send(guest, &play(PlayNowMode::Quick, false, None));
    net.run(100);
    assert!(net.messages(guest).contains(&ServerMessage::PlayNowResult { result: play_now::NO_MATCH }));

    net.send(host, &play(PlayNowMode::Custom, true, Some(0x55)));
    net.send(host, &ClientMessage::Recipe(vec![7; 5000]));
    net.run(100);
    let joined = net.messages(host);
    assert!(joined.iter().any(|message| matches!(message, ServerMessage::GameJoined { slot: 0, phase: Phase::LobbySkate, .. })));

    net.send(guest, &play(PlayNowMode::Custom, false, None));
    net.run(100);
    let guest_messages = net.messages(guest);
    assert!(guest_messages.iter().any(|message| matches!(message, ServerMessage::GameJoined { slot: 1, players, attributes, .. }
        if players.len() == 2 && players[0].user_id == host_id && attributes.challenge_key == 0x55)));
    assert!(guest_messages.contains(&ServerMessage::Recipe { user_id: host_id, data: vec![7; 5000] }));
    assert!(net.messages(host).iter().any(|message| matches!(message, ServerMessage::PlayerJoin(player) if player.user_id == guest_id)));

    // Runs of frames reach the other player, shrinking once acknowledged; the game sends its
    // recipe with its first runs.
    net.send(guest, &ClientMessage::Recipe(vec![8; 100]));
    for run in 0..40 {
        let frames: Vec<NetFrame> = (run * INTERVAL..(run + 1) * INTERVAL).map(frame).collect();
        net.clients[guest].1.send_frames(&frames);
        net.run(45);
    }
    let frames: Vec<NetFrame> = net
        .events(host)
        .into_iter()
        .filter_map(|event| match event {
            ClientEvent::Frame { user_id, frame, .. } if user_id == guest_id => Some(*frame),
            _ => None,
        })
        .collect();
    assert_eq!(frames.len(), 120);
    assert_eq!(frames[119].header[48], 119);

    // A challenge takes the members in the game's world: a recipe and a replay frame each.
    net.clients[host].1.send_frames(&[frame(0)]);
    net.run(100);
    net.events(host);
    net.events(guest);
    net.send(host, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.run(100);
    for player in [host, guest] {
        assert!(net.messages(player).iter().any(|message| matches!(message, ServerMessage::GameReset { kind: reset::LOAD_CHALLENGE, participants, .. }
            if participants[0] == host_id && participants[1] == guest_id)));
    }
    net.send(host, &ClientMessage::ChallengeLoaded);
    net.run(100);
    assert!(net.messages(guest).is_empty());
    net.send(guest, &ClientMessage::ChallengeLoaded);
    net.run(100);
    for player in [host, guest] {
        assert!(net.messages(player).iter().any(|message| matches!(message, ServerMessage::GameReset { kind: reset::CHALLENGE, .. })));
    }

    // Facts go to the other participant, in order.
    for tick in 0..30 {
        net.send(host, &ClientMessage::Facts { tick, stint: 0, notes: FactsNotes::default(), data: tick.to_le_bytes().to_vec() });
    }
    net.run(200);
    let facts: Vec<u32> = net
        .events(guest)
        .into_iter()
        .filter_map(|event| match event {
            ClientEvent::Facts { user_id, tick, .. } if user_id == host_id => Some(tick),
            _ => None,
        })
        .collect();
    assert_eq!(facts, (0..30).collect::<Vec<_>>());

    // A round barrier releases once both arrive.
    net.send(host, &ClientMessage::SyncPoint { round: 1 });
    net.run(100);
    assert!(net.messages(host).is_empty());
    net.send(guest, &ClientMessage::SyncPoint { round: 1 });
    net.run(100);
    assert!(net.messages(host).contains(&ServerMessage::SyncPoint { round: 1 }));

    for player in [host, guest] {
        net.send(player, &ClientMessage::Complete);
    }
    net.run(100);
    assert!(net.messages(host).contains(&ServerMessage::AllPlayersComplete));
    net.send(host, &ClientMessage::Results(GameResult { score: 1, ranking: 1, ..GameResult::default() }));
    net.send(guest, &ClientMessage::Results(GameResult { score: 0, ranking: 2, ..GameResult::default() }));
    net.run(100);
    let results = net.messages(guest);
    assert!(results.iter().any(|message| matches!(message, ServerMessage::FinalResults(rows) if rows.len() == 2)));

    // The board shows, then the game goes back to lobby skate.
    net.run(16_000);
    let after = net.messages(guest);
    assert!(after.contains(&ServerMessage::ExitPostChallenge));
    assert!(after.iter().any(|message| matches!(message, ServerMessage::GameReset { kind: reset::LOBBY_SKATE, .. })));

    // The guest leaves; the host hears it and the game stays open.
    net.send(guest, &ClientMessage::LeaveGame);
    net.run(100);
    assert!(net.messages(host).contains(&ServerMessage::PlayerLeave { user_id: guest_id, reason: leave::QUIT }));
    assert_eq!(net.server.games(), 1);
}

#[test]
fn a_silent_player_is_kicked_for_inactivity() {
    let mut net = Net::new();
    let host = net.add("Host");
    let guest = net.add("Guest");
    net.run(100);
    net.send(host, &play(PlayNowMode::Custom, true, Some(1)));
    net.run(50);
    net.send(guest, &play(PlayNowMode::Custom, false, Some(1)));
    net.run(100);
    net.events(host);
    let silent = net.clients.remove(guest);
    let mut net_without = net;
    net_without.run(1_500);
    assert!(net_without.messages(host).is_empty(), "the host's quarter-second Busy keeps it in");
    net_without.run(1_000);
    let messages = net_without.messages(host);
    assert!(messages.iter().any(|message| matches!(message, ServerMessage::PlayerLeave { reason: leave::KICKED_FOR_INACTIVITY, .. })));
    drop(silent);
}

/// Signs every name in and puts them in one custom game, the first its host.
fn lobby(net: &mut Net, names: &[&str]) -> Vec<usize> {
    let clients: Vec<usize> = names.iter().map(|name| net.add(name)).collect();
    net.run(100);
    net.send(clients[0], &play(PlayNowMode::Custom, true, None));
    net.run(100);
    for &client in &clients[1..] {
        net.send(client, &play(PlayNowMode::Custom, false, None));
        net.run(100);
    }
    for &client in &clients {
        net.events(client);
    }
    clients
}

fn dropped_for_host_request(events: &[ClientEvent]) -> bool {
    events.iter().any(|event| matches!(event, ClientEvent::Disconnected(Disconnect::Closed(leave::KICKED_FOR_HOST_REQUEST))))
}

#[test]
fn a_guest_asking_what_only_the_host_may_is_dropped() {
    let attributes = GameAttributes { game_version: "test".into(), challenge_type: challenge::SKATE, challenge_key: 0x77, private: false, max_players: 6, ranked: false };
    let host_only = [
        ClientMessage::GameRequest { request: request::START_GAME, value: 0 },
        ClientMessage::GameRequest { request: request::CHANGE_PRIVATE_STATUS, value: 1 },
        ClientMessage::GameRequest { request: request::KICK_PLAYER, value: 0 },
        ClientMessage::ResetAttributes(attributes),
        ClientMessage::RequestChange { challenge_type: challenge::SKATE, challenge_key: 0x77 },
    ];
    for message in host_only {
        let mut net = Net::new();
        let clients = lobby(&mut net, &["Host", "Guest"]);
        let (host, guest) = (clients[0], clients[1]);
        net.send(guest, &message);
        net.run(300);
        assert!(dropped_for_host_request(&net.events(guest)), "{message:?}");
        assert_eq!(net.server.players(), 1, "{message:?}: the guest's connection is gone");
        let seen = net.messages(host);
        assert!(seen.iter().any(|seen| matches!(seen, ServerMessage::PlayerLeave { reason: leave::KICKED_FOR_HOST_REQUEST, .. })), "{message:?}");
        assert!(
            !seen.iter().any(|seen| matches!(seen, ServerMessage::GameReset { .. } | ServerMessage::PrivacyChanged { .. } | ServerMessage::GameChange { .. } | ServerMessage::Kicked { .. })),
            "{message:?}: nothing it asked happened"
        );
    }
}

#[test]
fn the_host_and_a_player_who_left_are_never_dropped_for_host_requests() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["First", "Second", "Third"]);
    let (first, second, third) = (clients[0], clients[1], clients[2]);
    // Requests a host sends after leaving are about no game: ignored.
    net.send(first, &ClientMessage::LeaveGame);
    net.send(first, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.send(first, &ClientMessage::GameRequest { request: request::CHANGE_PRIVATE_STATUS, value: 1 });
    net.run(300);
    assert!(!dropped_for_host_request(&net.events(first)));
    // The next member is the host now; it starts, then asks again out of turn, ignored.
    enter(&mut net, second);
    enter(&mut net, third);
    net.send(second, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.run(300);
    for client in [second, third] {
        let events = net.events(client);
        assert!(!dropped_for_host_request(&events));
        assert!(events.iter().any(|event| matches!(event, ClientEvent::Message(ServerMessage::GameReset { kind: reset::LOAD_CHALLENGE, .. }))));
    }
    net.send(second, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.send(second, &ClientMessage::RequestChange { challenge_type: challenge::SKATE, challenge_key: 0x77 });
    net.run(300);
    assert!(!dropped_for_host_request(&net.events(second)));
    assert_eq!(net.server.players(), 3);
    // Back in the game, the old host is its last member: a guest.
    net.send(first, &play(PlayNowMode::Custom, false, None));
    net.run(300);
    net.events(first);
    net.send(first, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.run(300);
    assert!(dropped_for_host_request(&net.events(first)));
    assert_eq!(net.server.players(), 2);
}

#[test]
fn a_nickname_the_prompt_cannot_make_drops_the_connection() {
    let longest = "y".repeat(24);
    let mut net = Net::new();
    let fits = net.add(&longest);
    let bad: Vec<usize> = ["x".repeat(25), String::new(), " Padded".into(), "Tab\tName".into(), "Caf\u{e9}".into(), "Rob!".into(), "---".into()]
        .iter()
        .map(|name| net.add(name))
        .collect();
    net.run(300);
    assert!(net.events(fits).iter().any(|event| matches!(event, ClientEvent::SignedIn { .. })), "24 characters sign in");
    for client in bad {
        assert_eq!(net.events(client), vec![ClientEvent::Disconnected(Disconnect::Closed(leave::KICKED_FOR_NICKNAME))]);
    }
    assert_eq!(net.server.players(), 1);
}

#[test]
fn a_nickname_holding_a_listed_word_drops_the_connection() {
    let mut net = Net::new();
    net.server.set_words(WordList::from_words(["faggot", "nigga"]));
    let fine = net.add("Rob");
    let listed: Vec<usize> = ["F4GG0T", "big n1ggggg4", "x-f a g o t-x"].iter().map(|name| net.add(name)).collect();
    net.run(300);
    assert!(net.events(fine).iter().any(|event| matches!(event, ClientEvent::SignedIn { .. })));
    for client in listed {
        assert_eq!(net.events(client), vec![ClientEvent::Disconnected(Disconnect::Closed(leave::KICKED_FOR_NICKNAME))]);
    }
    assert_eq!(net.server.players(), 1);
}

/// A client speaking the transport itself, to send what the game never does.
struct Raw {
    addr: SocketAddr,
    endpoint: Endpoint,
}

impl Raw {
    fn connect(net: &mut Net, host: u8) -> Self {
        let addr = SocketAddr::from(([10, 0, 9, host], 5000));
        let handshake = Handshake::new().expect("random source");
        let (nonce, key) = (u64::from(host) + 900, handshake.public());
        net.server.receive(addr, &transport::connect_datagram(nonce, &key, &Build::DEVELOPMENT.connect_proof(nonce, &key)), net.now);
        let (_, accept) = net.server.take_outgoing().into_iter().find(|(to, _)| *to == addr).expect("accepted");
        let (_, mut reader) = transport::read_header(&accept).expect("accept");
        assert_eq!(reader.u64().unwrap(), nonce);
        let connection = reader.u32().unwrap();
        let server = transport::read_key(&mut reader).unwrap();
        let session = Session::client(Build::DEVELOPMENT, &handshake, &server, nonce, connection).expect("keys");
        Self { addr, endpoint: Endpoint::new(connection, session, net.now) }
    }

    /// Sends `message`; what came back: the server's messages, and the reason it closed the
    /// connection with if it did.
    fn send(&mut self, net: &mut Net, message: &ClientMessage, reliable: bool) -> (Vec<ServerMessage>, Option<u8>) {
        self.send_bytes(net, message.encode(), reliable)
    }

    fn send_bytes(&mut self, net: &mut Net, bytes: Vec<u8>, reliable: bool) -> (Vec<ServerMessage>, Option<u8>) {
        if reliable {
            self.endpoint.send_reliable(&bytes);
        } else {
            assert!(self.endpoint.send_unreliable(bytes));
        }
        for datagram in self.endpoint.poll(net.now) {
            net.server.receive(self.addr, &datagram, net.now);
        }
        net.server.update(net.now);
        let mut messages = Vec::new();
        let mut closed = None;
        for (to, datagram) in net.server.take_outgoing() {
            let Some((header, _)) = transport::read_header(&datagram).filter(|_| to == self.addr) else { continue };
            match header.kind {
                kind::DATA => {
                    if self.endpoint.receive_datagram(&datagram, net.now).unwrap_or(false) {
                        while let Some(bytes) = self.endpoint.take_reliable() {
                            messages.extend(ServerMessage::decode(&bytes).ok());
                        }
                    }
                }
                kind::DISCONNECT => closed = closed.or(self.endpoint.open_disconnect(&datagram)),
                _ => {}
            }
        }
        (messages, closed)
    }
}

#[test]
fn nothing_comes_before_the_hello() {
    let mut net = Net::new();
    let hello = ClientMessage::Hello { nickname: "Polite".into(), game_version: "test".into() };
    let mut polite = Raw::connect(&mut net, 1);
    let (messages, closed) = polite.send(&mut net, &hello, true);
    assert_eq!(closed, None);
    assert!(messages.iter().any(|message| matches!(message, ServerMessage::Welcome { .. })));
    assert_eq!(polite.send(&mut net, &ClientMessage::Ping { time: 1 }, false).1, None, "signed in, it pings");
    assert_eq!(polite.send(&mut net, &hello, true).1, None, "a second hello changes nothing");
    let mut pinger = Raw::connect(&mut net, 2);
    assert_eq!(pinger.send(&mut net, &ClientMessage::Ping { time: 1 }, false).1, Some(leave::KICKED_BEFORE_SIGN_IN));
    let mut early = Raw::connect(&mut net, 3);
    assert_eq!(early.send(&mut net, &play(PlayNowMode::Quick, false, None), true).1, Some(leave::KICKED_BEFORE_SIGN_IN));
    let mut unreliable = Raw::connect(&mut net, 4);
    assert_eq!(unreliable.send(&mut net, &hello, false).1, Some(leave::KICKED_BEFORE_SIGN_IN), "the game's hello is reliable");
    let mut garbage = Raw::connect(&mut net, 5);
    assert_eq!(garbage.send_bytes(&mut net, vec![0xee, 1, 2, 3], true).1, Some(leave::LOST_CONNECTION));
    assert_eq!(net.server.players(), 1);
}

#[test]
fn a_message_that_does_not_decode_drops_the_connection() {
    let mut net = Net::new();
    let mut player = Raw::connect(&mut net, 1);
    let (messages, _) = player.send(&mut net, &ClientMessage::Hello { nickname: "Fuzzer".into(), game_version: "test".into() }, true);
    assert!(messages.iter().any(|message| matches!(message, ServerMessage::Welcome { .. })));
    assert_eq!(player.send_bytes(&mut net, vec![15, 1], false).1, Some(leave::LOST_CONNECTION), "a ping cut short");
    assert_eq!(net.server.players(), 0);
}

#[test]
fn a_connection_that_never_signs_in_is_dropped() {
    let mut net = Net::new();
    let mut idle = Raw::connect(&mut net, 1);
    let mut closed = None;
    for _ in 0..48 {
        net.now += transport::KEEPALIVE;
        // Its keepalives reach the server: only the missing hello can end it.
        for datagram in idle.endpoint.poll(net.now) {
            net.server.receive(idle.addr, &datagram, net.now);
        }
        net.server.update(net.now);
        for (to, datagram) in net.server.take_outgoing() {
            if to == idle.addr && transport::read_header(&datagram).is_some_and(|(header, _)| header.kind == kind::DISCONNECT) {
                closed = closed.or(idle.endpoint.open_disconnect(&datagram));
            }
        }
    }
    assert_eq!(closed, Some(leave::KICKED_BEFORE_SIGN_IN));
}

#[test]
fn a_game_made_private_takes_no_one_new() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Friend"]);
    let (host, friend) = (clients[0], clients[1]);
    net.send(host, &ClientMessage::GameRequest { request: request::CHANGE_PRIVATE_STATUS, value: 1 });
    net.run(200);
    let stranger = net.add("Stranger");
    net.run(200);
    net.send(stranger, &play(PlayNowMode::Quick, false, None));
    net.run(200);
    net.send(stranger, &play(PlayNowMode::Custom, false, None));
    net.run(200);
    let seen = net.messages(stranger);
    assert!(seen.contains(&ServerMessage::PlayNowResult { result: play_now::NO_MATCH }), "a quick match finds no public game");
    assert!(seen.iter().any(|message| matches!(message, ServerMessage::GameJoined { players, .. } if players.len() == 1)), "a custom join makes its own game");
    for client in [host, friend] {
        assert!(!net.messages(client).iter().any(|message| matches!(message, ServerMessage::PlayerJoin(_))), "no one joined the private game");
    }
    // Public again, it takes players again.
    net.send(host, &ClientMessage::GameRequest { request: request::CHANGE_PRIVATE_STATUS, value: 0 });
    net.run(200);
    let late = net.add("Late");
    net.run(200);
    net.send(late, &play(PlayNowMode::Quick, false, None));
    net.run(200);
    assert!(net.messages(late).iter().any(|message| matches!(message, ServerMessage::GameJoined { players, .. } if players.len() == 3)));
}

#[test]
fn another_build_is_refused_before_it_signs_in() {
    let mut net = Net::new();
    let stale = net.add_build("Stale", Build::new([5; 32]));
    let current = net.add("Current");
    net.run(500);
    assert_eq!(net.events(stale), vec![ClientEvent::Disconnected(Disconnect::Refused(refuse::BUILD))]);
    assert!(net.events(current).iter().any(|event| matches!(event, ClientEvent::SignedIn { .. })));
    assert_eq!(net.server.players(), 1);
}

#[test]
fn a_packet_capture_reads_no_message() {
    let mut net = Net::new();
    let host = net.add("WiresharkMe");
    let guest = net.add("Snoopy");
    net.run(300);
    net.send(host, &play(PlayNowMode::Custom, true, None));
    net.run(200);
    net.send(guest, &play(PlayNowMode::Custom, false, None));
    net.run(500);
    assert_eq!(net.server.players(), 2, "both signed in, so both names crossed the wire");
    assert!(net.wire.len() > 10);
    for name in [&b"WiresharkMe"[..], b"Snoopy"] {
        assert!(!net.wire.iter().any(|datagram| datagram.windows(name.len()).any(|window| window == name)), "a name in the clear");
    }
}

#[test]
fn an_unreachable_server_times_out() {
    let mut core = ClientCore::new(1, "Alone", "test", Build::DEVELOPMENT, Instant::now());
    let start = Instant::now();
    core.update(start);
    assert_eq!(core.take_outgoing().len(), 1);
    core.update(start + Duration::from_secs(9));
    assert_eq!(core.take_events(), vec![ClientEvent::Disconnected(Disconnect::Unreachable)]);
}

fn chats(messages: &[ServerMessage]) -> Vec<String> {
    messages
        .iter()
        .filter_map(|message| match message {
            ServerMessage::Chat { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The guest's recipe and a replay frame, after which the server enables its chat.
fn enter(net: &mut Net, client: usize) {
    net.send(client, &ClientMessage::Recipe(vec![1; 64]));
    net.clients[client].1.send_frames(&[frame(0)]);
    net.run(100);
}

#[test]
fn chat_reaches_only_the_lobby_and_only_once_the_player_is_in() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    let other = net.add("Other");
    net.run(100);
    net.send(other, &play(PlayNowMode::Custom, true, None));
    net.run(100);
    net.events(other);
    // Before its recipe and a replay frame reach the server, nothing is relayed.
    net.send(guest, &ClientMessage::Chat("too early".into()));
    net.run(100);
    assert!(chats(&net.messages(host)).is_empty());
    assert!(chats(&net.messages(guest)).is_empty());
    enter(&mut net, guest);
    assert!(net.messages(guest).contains(&ServerMessage::ChatEnabled));
    net.send(guest, &ClientMessage::Chat("hi all".into()));
    net.run(100);
    assert_eq!(chats(&net.messages(host)), ["hi all"]);
    assert_eq!(chats(&net.messages(guest)), ["hi all"], "the sender sees its line come back");
    assert!(chats(&net.messages(other)).is_empty(), "another game hears nothing");
}

#[test]
fn a_chat_line_the_prompt_cannot_make_drops_the_connection() {
    for line in ["".to_owned(), " padded".to_owned(), "two\nlines".to_owned()] {
        let mut net = Net::new();
        let clients = lobby(&mut net, &["Host", "Guest"]);
        let (host, guest) = (clients[0], clients[1]);
        enter(&mut net, guest);
        net.send(guest, &ClientMessage::Chat(line.clone()));
        net.run(300);
        assert!(
            net.events(guest).iter().any(|event| matches!(event, ClientEvent::Disconnected(Disconnect::Closed(leave::KICKED_FOR_CHAT)))),
            "{line:?}"
        );
        assert!(chats(&net.messages(host)).is_empty(), "{line:?}");
    }
}

#[test]
fn a_chat_line_holding_a_listed_word_is_not_relayed() {
    let mut net = Net::new();
    net.server.set_words(WordList::from_words(["nigga"]));
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    enter(&mut net, guest);
    net.send(guest, &ClientMessage::Chat("lol n 1 g g g g 4".into()));
    net.send(guest, &ClientMessage::Chat("gg".into()));
    net.run(300);
    assert_eq!(chats(&net.messages(host)), ["gg"]);
    let events = net.events(guest);
    assert!(!events.iter().any(|event| matches!(event, ClientEvent::Disconnected(_))), "the sender stays connected");
    let lines: Vec<String> = events
        .into_iter()
        .filter_map(|event| match event {
            ClientEvent::Message(ServerMessage::Chat { text, .. }) => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(lines, ["gg"], "the sender does not see the held line either");
}

#[test]
fn chat_relays_five_lines_in_ten_seconds() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    enter(&mut net, guest);
    for index in 0..7 {
        net.send(guest, &ClientMessage::Chat(format!("line {index}")));
    }
    net.run(200);
    assert_eq!(chats(&net.messages(host)).len(), 5);
    net.run(10_000);
    net.send(guest, &ClientMessage::Chat("later".into()));
    net.run(200);
    assert_eq!(chats(&net.messages(host)), ["later"]);
}

fn frames_heard(net: &mut Net, client: usize) -> usize {
    net.events(client).iter().filter(|event| matches!(event, ClientEvent::Frame { .. })).count()
}

/// The clients in the game's world, then the first's challenge, loaded by every one.
fn challenge(net: &mut Net, clients: &[usize]) {
    for &client in clients {
        enter(net, client);
    }
    net.send(clients[0], &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.run(100);
    for &client in clients {
        net.send(client, &ClientMessage::ChallengeLoaded);
    }
    net.run(100);
    for &client in clients {
        net.events(client);
    }
}

#[test]
fn lobby_skate_watches_a_challenge_that_sees_nothing_of_it() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    challenge(&mut net, &clients);
    let late = net.add("Late");
    net.run(100);
    net.send(late, &play(PlayNowMode::Custom, false, None));
    net.run(100);
    let (players, participants) = net
        .messages(late)
        .into_iter()
        .find_map(|message| match message {
            ServerMessage::GameJoined { phase: Phase::Challenge, players, participants, .. } => Some((players, participants)),
            _ => None,
        })
        .expect("the joiner joins during the challenge");
    assert_eq!(participants, [players[0].user_id, players[1].user_id], "the joiner hears who plays the challenge");
    enter(&mut net, late);
    for client in [host, guest, late] {
        net.events(client);
    }
    for tick in 1..20 {
        net.clients[host].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert!(frames_heard(&mut net, guest) > 0, "a participant sees the other");
    assert!(frames_heard(&mut net, late) > 0, "lobby skate watches the participants");
    for tick in 1..20 {
        net.clients[late].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert_eq!(frames_heard(&mut net, host), 0, "no participant sees lobby skate");
    assert_eq!(frames_heard(&mut net, guest), 0);
    // The challenge ends; back in lobby skate everyone sees everyone.
    for &client in &clients {
        net.send(client, &ClientMessage::Complete);
    }
    net.run(100);
    net.send(host, &ClientMessage::Results(GameResult { score: 1, ranking: 1, ..GameResult::default() }));
    net.send(guest, &ClientMessage::Results(GameResult { score: 0, ranking: 2, ..GameResult::default() }));
    net.run(16_000);
    assert!(net.messages(late).iter().any(|message| matches!(message, ServerMessage::GameReset { kind: reset::LOBBY_SKATE, .. })));
    net.events(host);
    for tick in 20..40 {
        net.clients[late].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert!(frames_heard(&mut net, host) > 0);
}

#[test]
fn a_participant_who_leaves_the_challenge_waits_in_lobby_skate() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest", "Third"]);
    let (host, guest, third) = (clients[0], clients[1], clients[2]);
    challenge(&mut net, &clients);
    net.send(guest, &ClientMessage::LeaveChallenge);
    net.run(100);
    let left = |messages: &[ServerMessage]| messages.iter().find_map(|message| match message {
        ServerMessage::ChallengeLeave { user_id } => Some(*user_id),
        _ => None,
    });
    let (to_host, to_guest) = (net.messages(host), net.messages(guest));
    let leaver = left(&to_host).expect("the participants hear of the leave");
    assert_eq!(left(&to_guest), Some(leaver), "the leaver hears it too");
    assert!(!to_host.iter().any(|message| matches!(message, ServerMessage::PlayerLeave { .. })), "the leaver stays in the game");
    assert_eq!(left(&net.messages(third)), Some(leaver));
    // A second leave, and a barrier, from a member no longer playing the challenge count for nothing.
    net.send(guest, &ClientMessage::LeaveChallenge);
    net.send(guest, &ClientMessage::Complete);
    net.run(100);
    assert!(!net.messages(host).iter().any(|message| matches!(message, ServerMessage::ChallengeLeave { .. } | ServerMessage::AllPlayersComplete)));
    // The leaver watches the challenge; the challenge sees nothing of the leaver.
    for tick in 1..20 {
        net.clients[host].1.send_frames(&[frame(tick)]);
        net.clients[guest].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert!(frames_heard(&mut net, guest) > 0);
    assert!(frames_heard(&mut net, third) > 0, "the third participant sees the host");
    assert_eq!(frames_heard(&mut net, host), 0, "no participant sees the leaver");
    // The two left finish without the leaver.
    for client in [host, third] {
        net.send(client, &ClientMessage::Complete);
    }
    net.run(100);
    net.send(host, &ClientMessage::Results(GameResult { score: 1, ranking: 1, ..GameResult::default() }));
    net.send(third, &ClientMessage::Results(GameResult { score: 0, ranking: 2, ..GameResult::default() }));
    net.run(100);
    let rows = net.messages(host).into_iter().find_map(|message| match message {
        ServerMessage::FinalResults(rows) => Some(rows),
        _ => None,
    });
    assert_eq!(rows.map(|rows| rows.len()), Some(2), "the board holds the players who finished");
}

#[test]
fn a_challenge_every_participant_left_goes_back_to_lobby_skate() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    challenge(&mut net, &clients);
    net.send(host, &ClientMessage::LeaveChallenge);
    net.run(100);
    assert!(!net.messages(guest).iter().any(|message| matches!(message, ServerMessage::GameReset { .. })), "one player still plays it");
    net.send(guest, &ClientMessage::LeaveChallenge);
    net.run(100);
    for client in [host, guest] {
        let messages = net.messages(client);
        assert!(messages.iter().any(|message| matches!(message, ServerMessage::GameReset { kind: reset::LOBBY_SKATE, .. })));
        assert!(!messages.iter().any(|message| matches!(message, ServerMessage::FinalResults(_))), "no board for a challenge no one finished");
    }
    for tick in 1..20 {
        net.clients[host].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert!(frames_heard(&mut net, guest) > 0, "back in lobby skate the two see each other");
}

#[test]
fn a_challenge_takes_only_the_members_in_the_world() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest", "Coming"]);
    let (host, guest, coming) = (clients[0], clients[1], clients[2]);
    enter(&mut net, host);
    // One member in the world is not two.
    net.send(host, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.run(100);
    assert!(!net.messages(guest).iter().any(|message| matches!(message, ServerMessage::GameReset { .. })));
    enter(&mut net, guest);
    net.send(host, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.run(100);
    let participants = net
        .messages(coming)
        .into_iter()
        .find_map(|message| match message {
            ServerMessage::GameReset { kind: reset::LOAD_CHALLENGE, participants, .. } => Some(participants),
            _ => None,
        })
        .expect("the challenge loads");
    assert_eq!(participants.iter().filter(|&&participant| participant != 0).count(), 2, "the member still coming in stays out");
}

#[test]
fn a_player_joining_while_the_location_changes_goes_where_the_game_goes() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let host = clients[0];
    enter(&mut net, host);
    enter(&mut net, clients[1]);
    net.send(host, &ClientMessage::RequestChange { challenge_type: challenge::SKATE, challenge_key: 0x99 });
    net.run(100);
    // No challenge starts while the location changes.
    net.send(host, &ClientMessage::GameRequest { request: request::START_GAME, value: 0 });
    net.run(100);
    assert!(!net.messages(host).iter().any(|message| matches!(message, ServerMessage::GameReset { kind: reset::LOAD_CHALLENGE, .. })));
    // A search for the new location finds the game, and the joiner loads the new location.
    let late = net.add("Late");
    net.run(100);
    net.send(late, &play(PlayNowMode::Custom, false, Some(0x99)));
    net.run(100);
    assert!(net.messages(late).iter().any(|message| matches!(message, ServerMessage::GameJoined { attributes, .. } if attributes.challenge_key == 0x99)));
    net.run(2_500);
    assert!(net.messages(late).iter().any(|message| matches!(message, ServerMessage::GameReset { kind: reset::LOBBY_SKATE, .. })));
}

#[test]
fn one_address_holds_at_most_eight_connections() {
    let mut net = Net::new();
    let home: Vec<usize> = (0..9_u16).map(|port| net.add_at(&format!("Home{port}"), Build::DEVELOPMENT, SocketAddr::from(([10, 0, 7, 1], 6000 + port)))).collect();
    net.run(300);
    let mut refused = 0;
    for player in home {
        if net.events(player).contains(&ClientEvent::Disconnected(Disconnect::Refused(refuse::FULL))) {
            refused += 1;
        }
    }
    assert_eq!(refused, 1);
    assert_eq!(net.server.players(), 8);
}

#[test]
fn reliable_messages_far_faster_than_the_game_sends_drop_the_connection() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Flooder"]);
    for _ in 0..2000 {
        net.send(clients[1], &ClientMessage::Vote { score: 1 });
    }
    net.run(1000);
    assert!(net.events(clients[1]).contains(&ClientEvent::Disconnected(Disconnect::Closed(leave::KICKED_FOR_FLOOD))));
    assert_eq!(net.server.players(), 1);
}

#[test]
fn replay_frames_reach_no_one_before_the_recipe() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    for tick in 1..20 {
        net.clients[guest].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert_eq!(frames_heard(&mut net, host), 0);
    assert!(net.events(guest).iter().all(|event| !matches!(event, ClientEvent::Disconnected(_))), "the frames only kept it in the game");
    net.send(guest, &ClientMessage::Recipe(vec![1; 64]));
    net.run(100);
    for tick in 20..40 {
        net.clients[guest].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert!(frames_heard(&mut net, host) > 0);
}

#[test]
fn a_participant_waits_at_one_sync_point_at_a_time() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    challenge(&mut net, &clients);
    let synced = |messages: Vec<ServerMessage>| -> Vec<u32> {
        messages
            .into_iter()
            .filter_map(|message| match message {
                ServerMessage::SyncPoint { round } => Some(round),
                _ => None,
            })
            .collect()
    };
    net.send(host, &ClientMessage::SyncPoint { round: 1 });
    for round in 2..50 {
        net.send(host, &ClientMessage::SyncPoint { round });
    }
    net.run(100);
    net.send(guest, &ClientMessage::SyncPoint { round: 1 });
    net.run(100);
    assert_eq!(synced(net.messages(host)), [1]);
    assert_eq!(synced(net.messages(guest)), [1]);
    net.send(guest, &ClientMessage::SyncPoint { round: 2 });
    net.run(100);
    assert!(synced(net.messages(guest)).is_empty(), "the host's arrival at round 2 came while it waited at round 1");
}

#[test]
fn frames_past_the_game_s_rate_are_not_relayed() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    enter(&mut net, clients[0]);
    net.events(clients[1]);
    for tick in 1..=300 {
        net.clients[clients[0]].1.send_frames(&[frame(tick)]);
    }
    net.run(1000);
    let heard = frames_heard(&mut net, clients[1]);
    assert!((50..=160).contains(&heard), "{heard} of 300 frames sent at once relayed");
    // At the game's own pace every frame goes through.
    for tick in 301..=360 {
        net.clients[clients[0]].1.send_frames(&[frame(tick)]);
        net.run(15);
    }
    assert!(frames_heard(&mut net, clients[1]) >= 55);
}

#[test]
fn a_second_recipe_and_an_unchanged_privacy_reach_no_one() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    net.send(clients[1], &ClientMessage::Recipe(vec![1; 64]));
    net.run(100);
    net.send(clients[1], &ClientMessage::Recipe(vec![2; 64]));
    net.send(clients[0], &ClientMessage::GameRequest { request: request::CHANGE_PRIVATE_STATUS, value: 0 });
    net.run(100);
    let heard = net.messages(clients[0]);
    let recipes: Vec<&Vec<u8>> = heard.iter().filter_map(|message| match message {
        ServerMessage::Recipe { data, .. } => Some(data),
        _ => None,
    }).collect();
    assert_eq!(recipes, vec![&vec![1; 64]]);
    assert!(!heard.iter().any(|message| matches!(message, ServerMessage::PrivacyChanged { .. })));
    net.send(clients[0], &ClientMessage::GameRequest { request: request::CHANGE_PRIVATE_STATUS, value: 1 });
    net.run(100);
    assert!(net.messages(clients[1]).contains(&ServerMessage::PrivacyChanged { private: true }));
}

#[test]
fn a_stream_of_matchmaking_finds_nothing() {
    let mut net = Net::new();
    let player = net.add("Searcher");
    net.run(100);
    net.events(player);
    for key in 1..=10 {
        net.send(player, &play(PlayNowMode::Custom, true, Some(key)));
    }
    net.run(100);
    let results: Vec<u8> = net.messages(player).into_iter().filter_map(|message| match message {
        ServerMessage::PlayNowResult { result } => Some(result),
        _ => None,
    }).collect();
    assert_eq!(results.iter().filter(|&&result| result == play_now::JOINED).count(), 4, "{results:?}");
    assert_eq!(results.iter().filter(|&&result| result == play_now::NO_MATCH).count(), 6);
    assert_eq!(net.server.games(), 1, "each game it made and left closed");
}

#[test]
fn player_ids_are_not_a_sequence() {
    let mut net = Net::new();
    let ids: Vec<u64> = (0..4).map(|n| net.add(&format!("P{n}"))).collect::<Vec<_>>().into_iter().map(|player| {
        net.run(50);
        net.events(player).into_iter().find_map(|event| match event {
            ClientEvent::SignedIn { user_id } => Some(user_id),
            _ => None,
        }).expect("signed in")
    }).collect();
    let mut generator = ids[0];
    generator ^= generator << 13;
    generator ^= generator >> 7;
    generator ^= generator << 17;
    assert_ne!(ids[1], generator, "the next id follows from the last");
}

fn listing(messages: &[ServerMessage]) -> Option<Vec<LobbyInfo>> {
    messages.iter().rev().find_map(|message| match message {
        ServerMessage::Lobbies(lobbies) => Some(lobbies.clone()),
        _ => None,
    })
}

fn joined_game(messages: &[ServerMessage]) -> Option<(u32, usize)> {
    messages.iter().find_map(|message| match message {
        ServerMessage::GameJoined { game, players, .. } => Some((*game, players.len())),
        _ => None,
    })
}

#[test]
fn the_lobby_browser_lists_the_public_games_and_joins_one() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let loner = net.add("Loner");
    net.run(100);
    let mut private = play(PlayNowMode::Custom, true, Some(0x99));
    if let ClientMessage::PlayNow(request) = &mut private {
        request.private = true;
    }
    net.send(loner, &private);
    net.run(100);
    let (private_game, _) = joined_game(&net.messages(loner)).expect("the private game");
    let browser = net.add("Browser");
    net.run(100);
    net.send(browser, &ClientMessage::ListLobbies);
    net.run(100);
    let listed = listing(&net.messages(browser)).expect("a listing");
    assert_eq!(listed.len(), 1, "the private game is not listed");
    let game = listed[0];
    assert_eq!(
        (game.challenge_type, game.challenge_key, game.players, usize::from(game.max_players), game.phase),
        (challenge::SKATE, 0xabc, 2, MAX_PLAYERS, Phase::LobbySkate)
    );
    net.send(browser, &ClientMessage::JoinLobby { game: game.game });
    net.run(100);
    let seen = net.messages(browser);
    assert!(seen.contains(&ServerMessage::PlayNowResult { result: play_now::JOINED }));
    assert_eq!(joined_game(&seen), Some((game.game, 3)));
    assert!(net.messages(clients[0]).iter().any(|message| matches!(message, ServerMessage::PlayerJoin(player) if player.name == "Browser")));
    let stranger = net.add("Stranger");
    net.run(100);
    for target in [private_game, game.game.wrapping_add(1000)] {
        net.send(stranger, &ClientMessage::JoinLobby { game: target });
        net.run(1_100);
        let seen = net.messages(stranger);
        assert!(seen.contains(&ServerMessage::PlayNowResult { result: play_now::NO_MATCH }), "{target}");
        assert_eq!(joined_game(&seen), None, "{target}");
    }
}

#[test]
fn a_game_holds_eight_players_and_the_browser_says_when_it_is_full() {
    let mut net = Net::new();
    let names: Vec<String> = (1..=MAX_PLAYERS).map(|n| format!("Player{n}")).collect();
    let clients = lobby(&mut net, &names.iter().map(String::as_str).collect::<Vec<_>>());
    net.run(200);
    assert_eq!(clients.len(), MAX_PLAYERS);
    assert_eq!(net.server.games(), 1, "all eight are in the host's game");
    let ninth = net.add("Ninth");
    net.run(100);
    net.send(ninth, &ClientMessage::ListLobbies);
    net.run(100);
    let listed = listing(&net.messages(ninth)).expect("a listing");
    assert_eq!((listed.len(), usize::from(listed[0].players), usize::from(listed[0].max_players)), (1, MAX_PLAYERS, MAX_PLAYERS));
    net.send(ninth, &ClientMessage::JoinLobby { game: listed[0].game });
    net.run(100);
    let seen = net.messages(ninth);
    assert!(seen.contains(&ServerMessage::PlayNowResult { result: play_now::FULL }));
    assert_eq!(joined_game(&seen), None);
}

#[test]
fn the_lobby_browser_asks_only_after_the_hello() {
    let mut net = Net::new();
    let mut early = Raw::connect(&mut net, 1);
    assert_eq!(early.send(&mut net, &ClientMessage::ListLobbies, true).1, Some(leave::KICKED_BEFORE_SIGN_IN));
    let mut joiner = Raw::connect(&mut net, 2);
    assert_eq!(joiner.send(&mut net, &ClientMessage::JoinLobby { game: 1 }, true).1, Some(leave::KICKED_BEFORE_SIGN_IN));
    let mut player = Raw::connect(&mut net, 3);
    player.send(&mut net, &ClientMessage::Hello { nickname: "Browser".into(), game_version: "test".into() }, true);
    let (messages, closed) = player.send(&mut net, &ClientMessage::ListLobbies, true);
    assert_eq!((listing(&messages), closed), (Some(Vec::new()), None));
    assert_eq!(player.send(&mut net, &ClientMessage::ListLobbies, false), (Vec::new(), None), "the game asks on the reliable channel");
}

#[test]
fn a_stream_of_lobby_listings_is_answered_at_the_game_s_pace() {
    let mut net = Net::new();
    let browser = net.add("Browser");
    net.run(100);
    for _ in 0..20 {
        net.send(browser, &ClientMessage::ListLobbies);
    }
    net.run(100);
    let answered = net.messages(browser).iter().filter(|message| matches!(message, ServerMessage::Lobbies(_))).count();
    assert_eq!(answered, 5);
    net.run(1_000);
    net.send(browser, &ClientMessage::ListLobbies);
    net.run(100);
    assert!(listing(&net.messages(browser)).is_some(), "a second later the next is answered");
    assert_eq!(net.server.players(), 1, "only unanswered, never dropped");
}

#[test]
fn a_search_at_one_location_never_joins_a_game_at_another() {
    let mut net = Net::new();
    let lower = net.add("Lower");
    let upper = net.add("Upper");
    let second = net.add("Second");
    let anywhere = net.add("Anywhere");
    net.run(100);
    // Join at a location no game is at starts a public game there, even with another location's
    // free skate open.
    net.send(lower, &free_skate(false, Some(0x10)));
    net.run(100);
    let (lower_game, players) = joined_game(&net.messages(lower)).expect("a game at the first location");
    assert_eq!(players, 1);
    net.send(upper, &free_skate(false, Some(0x20)));
    net.run(100);
    let seen = net.messages(upper);
    let (upper_game, players) = joined_game(&seen).expect("a game at the second location");
    assert_ne!(upper_game, lower_game, "the other location's game is not joined");
    assert_eq!(players, 1);
    assert!(seen.iter().any(|message| matches!(message, ServerMessage::GameJoined { attributes, .. } if attributes.challenge_key == 0x20)));
    // Join at a location a game is at joins that game, and Join at any location the fuller one.
    net.send(second, &free_skate(false, Some(0x10)));
    net.run(100);
    assert_eq!(joined_game(&net.messages(second)), Some((lower_game, 2)));
    net.send(anywhere, &free_skate(false, None));
    net.run(100);
    assert_eq!(joined_game(&net.messages(anywhere)), Some((lower_game, 3)));
    assert_eq!(net.server.games(), 2);
}

#[test]
fn a_challenge_search_at_one_location_never_joins_a_game_at_another() {
    let mut net = Net::new();
    let host = net.add("Host");
    let elsewhere = net.add("Elsewhere");
    net.run(100);
    net.send(host, &play(PlayNowMode::Custom, false, Some(0x55)));
    net.run(100);
    let (host_game, _) = joined_game(&net.messages(host)).expect("a game at the chosen location");
    net.send(elsewhere, &play(PlayNowMode::Custom, false, Some(0x66)));
    net.run(100);
    let seen = net.messages(elsewhere);
    let (game, players) = joined_game(&seen).expect("a game at the other location");
    assert_ne!(game, host_game);
    assert_eq!(players, 1);
    // A quick match still finds a public challenge game anywhere.
    let quick = net.add("Quick");
    net.run(100);
    net.send(quick, &play(PlayNowMode::Quick, false, None));
    net.run(100);
    assert!(joined_game(&net.messages(quick)).is_some_and(|(game, players)| game == host_game && players == 2));
}

#[test]
fn a_search_that_does_not_add_up_finds_nothing_and_changes_nothing() {
    let mut net = Net::new();
    let host = net.add("Host");
    let searcher = net.add("Searcher");
    net.run(100);
    net.send(host, &free_skate(false, Some(0x10)));
    net.send(host, &ClientMessage::Recipe(vec![1; 100]));
    net.run(100);
    let (game, _) = joined_game(&net.messages(host)).expect("the free skate");
    net.send(searcher, &play(PlayNowMode::Custom, false, Some(0x55)));
    net.run(1_100);
    let (own, _) = joined_game(&net.messages(searcher)).expect("the searcher's own game");
    // A free skate searched as a challenge, a challenge searched as a free skate, another
    // version, and a game made with no location.
    let mut requests = Vec::new();
    for (mode, challenge_type, version, key, fallback) in [
        (PlayNowMode::Custom, challenge::ONLINE_FREE_SKATE, "test", Some(0x10), 0xabc),
        (PlayNowMode::Freeskate, challenge::SKATE, "test", Some(0x55), 0xabc),
        (PlayNowMode::Freeskate, challenge::ONLINE_FREE_SKATE, "other", Some(0x10), 0xabc),
        (PlayNowMode::Custom, challenge::SKATE, "test", None, 0),
    ] {
        requests.push(ClientMessage::PlayNow(PlayNow {
            mode,
            ranked: false,
            challenge_type,
            challenge_key: key,
            private: false,
            create: key.is_none(),
            fallback_key: fallback,
            game_version: version.into(),
        }));
    }
    // The inconsistent ones are refused before the searcher leaves its game; the one with no
    // location leaves it (a search leaves the game first) and makes none.
    for (request, games) in requests.iter().zip([2, 2, 2, 1]) {
        net.send(searcher, request);
        net.run(1_100);
        let seen = net.messages(searcher);
        assert!(seen.contains(&ServerMessage::PlayNowResult { result: play_now::NO_MATCH }), "{request:?}");
        assert_eq!(joined_game(&seen), None, "{request:?}");
        assert_eq!(net.server.games(), games, "{request:?}");
    }
    assert_ne!(own, game);
    assert!(net.messages(host).iter().all(|message| !matches!(message, ServerMessage::PlayerJoin(_))), "no one joined the free skate");
}

#[test]
fn a_free_skate_is_never_ranked() {
    let mut net = Net::new();
    let host = net.add("Host");
    net.run(100);
    let mut request = free_skate(false, Some(0x10));
    if let ClientMessage::PlayNow(play) = &mut request {
        play.ranked = true;
    }
    net.send(host, &request);
    net.run(100);
    assert!(net.messages(host).iter().any(|message| matches!(message, ServerMessage::GameJoined { attributes, .. } if !attributes.ranked)));
}

#[test]
fn an_ack_for_a_slot_s_last_member_leaves_its_next_member_coded_against_its_frames() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Watcher", "Leaver"]);
    let (host, watcher, leaver) = (clients[0], clients[1], clients[2]);
    let joiner = net.add("Joiner");
    net.run(100);
    net.events(joiner);
    // The leaver's game has run far longer than the joiner's: its ticks are far past the joiner's.
    let run = |first: u32| (first..first + INTERVAL).map(frame).collect::<Vec<_>>();
    net.send(leaver, &ClientMessage::Recipe(vec![1; 64]));
    for first in (100_000..100_012).step_by(3) {
        net.clients[leaver].1.send_frames(&run(first));
        net.run(45);
    }
    // Clear of the quarter-second Busy, which would carry the acks at once.
    while net.elapsed % 250 != 100 {
        net.run(5);
    }
    // Both players decode the leaver's newest run. The host's ack of it rides on its next message;
    // the watcher's waits for a datagram while the leaver leaves and the joiner takes its slot.
    net.clients[leaver].1.send_frames(&run(100_012));
    net.run(5);
    net.send(host, &ClientMessage::Busy);
    net.send(leaver, &ClientMessage::LeaveGame);
    net.send(joiner, &play(PlayNowMode::Custom, false, None));
    net.run(200);
    assert!(net.messages(joiner).iter().any(|message| matches!(message, ServerMessage::GameJoined { slot: 2, .. })), "the joiner has the leaver's slot");
    net.send(joiner, &ClientMessage::Recipe(vec![2; 64]));
    for first in (10..22).step_by(3) {
        net.clients[joiner].1.send_frames(&run(first));
        net.run(45);
    }
    let bytes = |net: &Net, client: usize| net.received_bytes.get(&net.clients[client].0).copied().unwrap_or(0);
    let before = (bytes(&net, host), bytes(&net, watcher));
    for first in (22..82).step_by(3) {
        net.clients[joiner].1.send_frames(&run(first));
        net.run(45);
    }
    let (to_host, to_watcher) = (bytes(&net, host) - before.0, bytes(&net, watcher) - before.1);
    assert!(frames_heard(&mut net, watcher) > 0);
    assert!(to_watcher < 2 * to_host, "the joiner's runs reach the watcher coded whole: {to_watcher} bytes against the host's {to_host}");
}

fn signed_in(events: &[ClientEvent]) -> Option<u64> {
    events.iter().find_map(|event| match event {
        ClientEvent::SignedIn { user_id } => Some(*user_id),
        _ => None,
    })
}

#[test]
fn a_kicked_player_never_sees_or_joins_that_game_again_under_any_name() {
    let mut net = Net::new();
    let host = net.add("Host");
    let guest = net.add("Guest");
    net.run(100);
    let guest_id = signed_in(&net.events(guest)).expect("the guest signs in");
    net.send(host, &play(PlayNowMode::Custom, true, None));
    net.run(100);
    net.send(guest, &play(PlayNowMode::Custom, false, None));
    net.run(100);
    let (game, _) = joined_game(&net.messages(guest)).expect("the guest joins");
    net.send(host, &ClientMessage::GameRequest { request: request::KICK_PLAYER, value: guest_id });
    net.run(100);
    assert!(net.messages(guest).iter().any(|message| matches!(message, ServerMessage::Kicked { reason: leave::KICKED_BY_HOST })));
    // The player, and its address on a new connection under another name, alike.
    let renamed = net.add_at("Renamed", Build::DEVELOPMENT, SocketAddr::from(([10, 0, 0, 2], 4001)));
    net.run(100);
    for client in [guest, renamed] {
        net.events(client);
        net.send(client, &ClientMessage::ListLobbies);
        net.run(1_100);
        assert_eq!(listing(&net.messages(client)), Some(Vec::new()), "the browser does not list the game");
        net.send(client, &ClientMessage::JoinLobby { game });
        net.run(1_100);
        let seen = net.messages(client);
        assert!(seen.contains(&ServerMessage::PlayNowResult { result: play_now::NO_MATCH }));
        assert_eq!(joined_game(&seen), None, "a join by its id is refused");
        net.send(client, &play(PlayNowMode::Quick, false, None));
        net.run(1_100);
        let seen = net.messages(client);
        assert!(seen.contains(&ServerMessage::PlayNowResult { result: play_now::NO_MATCH }), "matchmaking does not find it");
        assert_eq!(joined_game(&seen), None);
    }
    // A player from another address finds and joins the game as before.
    let stranger = net.add("Stranger");
    net.run(100);
    net.send(stranger, &ClientMessage::ListLobbies);
    net.run(100);
    assert_eq!(listing(&net.messages(stranger)).map(|lobbies| lobbies.len()), Some(1));
    net.send(stranger, &ClientMessage::JoinLobby { game });
    net.run(100);
    assert_eq!(joined_game(&net.messages(stranger)), Some((game, 2)));
}

#[test]
fn a_chat_shadowbanned_player_s_lines_come_back_to_it_alone() {
    let mut net = Net::new();
    net.server.set_bans(BanList::from_bans([(IpAddr::from([10, 0, 0, 2]), "Guest")]), BanList::default());
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    enter(&mut net, host);
    enter(&mut net, guest);
    net.send(guest, &ClientMessage::Chat("you all stink".into()));
    net.run(300);
    assert!(chats(&net.messages(host)).is_empty(), "no one else sees it");
    assert_eq!(chats(&net.messages(guest)), ["you all stink"], "it looks sent to its sender");
    net.send(host, &ClientMessage::Chat("hi".into()));
    net.run(300);
    assert_eq!(chats(&net.messages(guest)), ["hi"], "it still reads the others");
}

#[test]
fn a_banned_address_is_never_answered() {
    let mut net = Net::new();
    net.server.set_bans(BanList::default(), BanList::from_bans([(IpAddr::from([10, 0, 0, 1]), "Banned")]));
    let banned = net.add("Banned");
    let other = net.add("Other");
    net.run(1_000);
    assert_eq!(net.received_bytes.get(&net.clients[banned].0), None, "not one datagram back");
    assert!(signed_in(&net.events(banned)).is_none());
    assert!(signed_in(&net.events(other)).is_some());
    assert_eq!(net.server.players(), 1);
}

#[test]
fn an_ip_ban_closes_the_address_s_connections_without_a_word_and_a_lift_lets_it_back() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Troll"]);
    let (host, troll) = (clients[0], clients[1]);
    let troll_address = IpAddr::from([10, 0, 0, 2]);
    let ask = |net: &mut Net, action: Action| {
        let (reply, answer) = std::sync::mpsc::channel();
        net.server.moderate(Request::Act(action, reply), net.now);
        answer.recv().expect("an outcome")
    };
    let outcome = ask(&mut net, Action::IpBan(vec![(troll_address, "Troll".into())]));
    assert_eq!(outcome, "Troll is IP-banned. 1 connection closed.", "the page shows no address");
    assert_eq!(ask(&mut net, Action::IpBan(vec![(troll_address, "Troll".into())])), "Troll was IP-banned already.");
    let before = net.received_bytes.get(&net.clients[troll].0).copied();
    net.run(1_000);
    assert!(net.messages(host).iter().any(|message| matches!(message, ServerMessage::PlayerLeave { reason: leave::LOST_CONNECTION, .. })));
    assert_eq!(net.received_bytes.get(&net.clients[troll].0).copied(), before, "nothing more reaches the banned address");
    assert!(!net.events(troll).iter().any(|event| matches!(event, ClientEvent::Disconnected(Disconnect::Closed(_)))), "it is not told");
    let (reply, answer) = std::sync::mpsc::channel();
    net.server.moderate(Request::View(reply), net.now);
    let view = answer.recv().expect("a view");
    assert_eq!(view.players.iter().map(|player| player.name.as_str()).collect::<Vec<_>>(), ["Host"]);
    assert_eq!(view.ip_bans, [(troll_address, "Troll".to_owned())]);
    assert_eq!(ask(&mut net, Action::Lift { chat: vec![troll_address], ip: vec![troll_address] }), "A ticked chat shadowban was not in force. The IP ban on Troll is lifted.");
    let back = net.add_at("Back", Build::DEVELOPMENT, SocketAddr::from(([10, 0, 0, 2], 4001)));
    net.run(500);
    assert!(signed_in(&net.events(back)).is_some(), "the address connects again");
}

#[test]
fn a_change_asked_during_a_challenge_changes_nothing() {
    let mut net = Net::new();
    let clients = lobby(&mut net, &["Host", "Guest"]);
    let (host, guest) = (clients[0], clients[1]);
    challenge(&mut net, &clients);
    net.send(host, &ClientMessage::RequestChange { challenge_type: challenge::SKATE, challenge_key: 0x77 });
    net.run(2_100);
    assert!(
        !net.messages(guest).iter().any(|message| matches!(message, ServerMessage::GameChange { .. } | ServerMessage::GameReset { .. })),
        "the challenge goes on"
    );
    for &client in &clients {
        net.send(client, &ClientMessage::Complete);
    }
    net.run(100);
    assert!(net.messages(guest).contains(&ServerMessage::AllPlayersComplete));
}
