//! The dedicated server under load: simulated players in games of a few each, every one skating
//! (a replay frame each tick, a run each few ticks) over a network that delays each datagram.
//! Prints the server's work for each second of play (its share of one processor core), its
//! longest passes (the most a relayed run waits on the server), its traffic, and whether every
//! frame reached exactly its own game's other players.
//!
//! `cargo run --release -p skate2-net --example server_load -- [players] [seconds] [game size]
//! [one-way delay ms] [ticks a run]` (20 players, 30 s, games of 7, 30 ms, runs of 3)

use skate2_net::client::{ClientCore, ClientEvent};
use skate2_net::crypto::Build;
use skate2_net::frames::{BONE_LANES, HEADER_WORDS, INTERVAL, MAX_RUN, NetFrame};
use skate2_net::protocol::{ClientMessage, PlayNow, PlayNowMode, ServerMessage, challenge};
use skate2_net::server::ServerCore;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Bones of a skater's pose.
const BONES: usize = 133;
const STEP: Duration = Duration::from_millis(1);
const TICK: Duration = Duration::from_micros(16_667);
/// How far a bone's rotation strays from its smooth path each sample, in radians: the coded runs
/// come out about the size measured live (a few KB/s a skater).
const JITTER: f32 = 0.01;
const VERSION: &str = "load";

/// What a run of the load measures.
struct Settings {
    players: usize,
    seconds: u64,
    game_size: usize,
    delay: Duration,
    interval: u32,
}

impl Settings {
    fn from_args() -> Self {
        let mut args = std::env::args().skip(1).map(|arg| arg.parse::<u64>().ok());
        let mut arg = |default: u64| args.next().flatten().unwrap_or(default);
        Self {
            players: usize::try_from(arg(20)).unwrap_or(20).max(1),
            seconds: arg(30).max(1),
            game_size: usize::try_from(arg(7)).unwrap_or(7).clamp(1, 8),
            delay: Duration::from_millis(arg(30)),
            interval: u32::try_from(arg(u64::from(INTERVAL))).unwrap_or(INTERVAL).clamp(1, u32::try_from(MAX_RUN).unwrap_or(8)),
        }
    }
}

/// A deterministic source of noise.
struct Noise(u64);

impl Noise {
    /// A value in [-1, 1).
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        #[expect(clippy::cast_precision_loss, reason = "24 bits of noise")]
        let unit = (self.0 >> 40) as f32 / 16_777_216.0;
        unit.mul_add(2.0, -1.0)
    }
}

/// One tick of a skater riding a circle of its own, every bone swinging.
#[expect(clippy::cast_precision_loss, reason = "test motion")]
fn frame(player: usize, tick: u32, sample: bool, noise: &mut Noise) -> NetFrame {
    let seconds = tick as f32 / 60.0;
    let heading = 0.4f32.mul_add(seconds, player as f32);
    let position = [15.0f32.mul_add(heading.cos(), 40.0 * player as f32), 0.2 * (3.0 * seconds).sin(), 15.0 * heading.sin()];
    let rotation = [0.0, (heading * 0.5).sin(), 0.0, (heading * 0.5).cos()];
    let mut header = [0; HEADER_WORDS];
    for (word, value) in header[32..36].iter_mut().zip(rotation) {
        *word = value.to_bits();
    }
    for (word, value) in header[36..39].iter_mut().zip(position) {
        *word = value.to_bits();
    }
    header[39] = 1.0f32.to_bits();
    header[46] = (6.0 + seconds.sin()).to_bits();
    header[48] = if tick % 120 < 4 { 0x1000 } else { 0xc000 };
    let pose = sample.then(|| {
        (0..BONES)
            .map(|bone| {
                let b = bone as f32;
                let axis = [b.sin(), (1.7 * b).cos(), 0.3f32.mul_add(b, 1.0).sin()];
                let length = axis.iter().map(|lane| lane * lane).sum::<f32>().sqrt().max(1e-3);
                let rate = 0.3f32.mul_add((bone % 7) as f32, 1.0);
                let angle = 0.4f32.mul_add((2.0 * seconds).mul_add(rate, b).sin(), JITTER * noise.next());
                let (sin, cos) = (angle * 0.5).sin_cos();
                let mut lanes = [0.0; BONE_LANES];
                lanes[0..4].copy_from_slice(&[1.0; 4]);
                lanes[4..7].copy_from_slice(&axis.map(|lane| lane / length * sin));
                lanes[7] = cos;
                let swing = 0.01f32.mul_add((4.0 * seconds + b).sin(), 0.1 * (bone % 5) as f32);
                lanes[8..12].copy_from_slice(&[0.05 * (bone % 9) as f32, swing, 0.03 * (bone % 4) as f32, 1.0]);
                lanes
            })
            .collect()
    });
    NetFrame {
        tick,
        header,
        pose,
        twist: [0.1f32.mul_add(seconds.sin(), 0.5), 0.3, 0.05f32.mul_add((2.0 * seconds).cos(), 0.2), 0.0],
        camera: vec![position[0] - 4.0 * heading.cos(), position[1] + 1.5, position[2] - 4.0 * heading.sin(), rotation[0], rotation[1], rotation[2], rotation[3], 60.0],
        audio_floats: (0..60).map(|index| (seconds * (1 + index % 5) as f32).sin() * (index % 3) as f32).collect(),
        audio_words: (0..16).map(|index| if index == 3 { tick / 30 } else { 7 }).collect(),
        hud: (0..15).map(|index| if index == 7 { tick / 6 } else { 0 }).collect(),
    }
}

struct Player {
    addr: SocketAddr,
    core: ClientCore,
    game: usize,
    user_id: Option<u64>,
    joined: bool,
    /// When the player's first tick falls: the players' ticks are not in step.
    offset: Duration,
    /// Ticks run, and the first tick's number: each game counts its own.
    ticks: u32,
    first_tick: u32,
    run: Vec<NetFrame>,
    noise: Noise,
    frames_sent: u64,
    bytes_down: u64,
}

/// Datagrams on their way, each due a fixed delay after it was sent, in the order sent.
type Link = VecDeque<(Instant, SocketAddr, Vec<u8>)>;

struct Simulation {
    server: ServerCore,
    players: Vec<Player>,
    by_addr: HashMap<SocketAddr, usize>,
    up: Link,
    down: Link,
    delay: Duration,
    now: Instant,
    elapsed: Duration,
    /// The server's time in each step's pass, while measuring.
    passes: Vec<Duration>,
    measuring: bool,
    server_in: (u64, u64),
    server_out: (u64, u64),
    /// Frames each player heard from each other player, by user id.
    heard: Vec<HashMap<u64, u64>>,
}

impl Simulation {
    /// The players signed in, each in its game with its recipe sent.
    fn start(settings: &Settings) -> Self {
        let origin = Instant::now();
        let players: Vec<Player> = (0..settings.players)
            .map(|index| {
                let [a, b] = u16::try_from(index + 1).unwrap_or(u16::MAX).to_be_bytes();
                let number = index as u64;
                Player {
                    addr: SocketAddr::from(([10, 1, a, b], 4000)),
                    core: ClientCore::new(number + 1, &format!("Skater{index}"), VERSION, Build::DEVELOPMENT, origin),
                    game: index / settings.game_size,
                    user_id: None,
                    joined: false,
                    offset: Duration::from_micros(number * 7_919 % 16_667),
                    ticks: 0,
                    first_tick: u32::try_from(index).unwrap_or(0) * 10_000,
                    run: Vec::new(),
                    noise: Noise(number + 0x9e37_79b9),
                    frames_sent: 0,
                    bytes_down: 0,
                }
            })
            .collect();
        let mut sim = Self {
            server: ServerCore::new(1, Build::DEVELOPMENT, Box::new(|_| {})),
            by_addr: players.iter().enumerate().map(|(index, player)| (player.addr, index)).collect(),
            heard: vec![HashMap::new(); settings.players],
            players,
            up: VecDeque::new(),
            down: VecDeque::new(),
            delay: settings.delay,
            now: origin,
            elapsed: Duration::ZERO,
            passes: Vec::new(),
            measuring: false,
            server_in: (0, 0),
            server_out: (0, 0),
        };
        let size = settings.game_size;
        assert!(sim.wait(|sim| sim.players.iter().all(|player| player.user_id.is_some())), "every player signs in");
        // Each game's first player makes it at its location, then the others join it there.
        for player in sim.players.iter_mut().step_by(size) {
            player.core.send(&free_skate(player.game));
        }
        assert!(sim.wait(|sim| sim.players.iter().step_by(size).all(|player| player.joined)), "every game is made");
        for (_, player) in sim.players.iter_mut().enumerate().filter(|(index, _)| index % size != 0) {
            player.core.send(&free_skate(player.game));
        }
        assert!(sim.wait(|sim| sim.players.iter().all(|player| player.joined)), "every player joins its game");
        for player in &mut sim.players {
            player.core.send(&ClientMessage::Recipe(vec![7; 4096]));
        }
        sim
    }

    /// One millisecond: the players send, the server takes what is due and answers, and the
    /// players take what is due.
    fn step(&mut self, mut tick: impl FnMut(usize, &mut Player)) {
        self.now += STEP;
        self.elapsed += STEP;
        for (index, player) in self.players.iter_mut().enumerate() {
            tick(index, player);
            player.core.update(self.now);
            for datagram in player.core.take_outgoing() {
                self.up.push_back((self.now + self.delay, player.addr, datagram));
            }
        }
        let started = Instant::now();
        let mut received = (0, 0);
        while self.up.front().is_some_and(|(due, _, _)| *due <= self.now) {
            let Some((_, addr, datagram)) = self.up.pop_front() else { break };
            received.0 += 1;
            received.1 += datagram.len() as u64;
            self.server.receive(addr, &datagram, self.now);
        }
        self.server.update(self.now);
        let outgoing = self.server.take_outgoing();
        let pass = started.elapsed();
        if self.measuring {
            self.passes.push(pass);
            self.server_in.0 += received.0;
            self.server_in.1 += received.1;
        }
        for (addr, datagram) in outgoing {
            if self.measuring {
                self.server_out.0 += 1;
                self.server_out.1 += datagram.len() as u64;
            }
            self.down.push_back((self.now + self.delay, addr, datagram));
        }
        while self.down.front().is_some_and(|(due, _, _)| *due <= self.now) {
            let Some((_, addr, datagram)) = self.down.pop_front() else { break };
            let Some(&index) = self.by_addr.get(&addr) else { continue };
            let player = &mut self.players[index];
            if self.measuring {
                player.bytes_down += datagram.len() as u64;
            }
            player.core.receive(&datagram, self.now);
        }
        for (index, player) in self.players.iter_mut().enumerate() {
            for event in player.core.take_events() {
                match event {
                    ClientEvent::SignedIn { user_id } => player.user_id = Some(user_id),
                    ClientEvent::Message(ServerMessage::GameJoined { .. }) => player.joined = true,
                    ClientEvent::Frame { user_id, .. } => *self.heard[index].entry(user_id).or_default() += 1,
                    ClientEvent::Disconnected(reason) => panic!("player {index} disconnected: {reason:?}"),
                    _ => {}
                }
            }
        }
    }

    /// Steps until `done`, the players saying `Busy` every quarter second as a game with no
    /// frame to send does; false past ten seconds.
    fn wait(&mut self, done: impl Fn(&Self) -> bool) -> bool {
        let until = self.elapsed + Duration::from_secs(10);
        while !done(self) {
            if self.elapsed >= until {
                return false;
            }
            let busy = self.elapsed.as_millis() % 250 == 0;
            self.step(|_, player| {
                if busy {
                    player.core.send(&ClientMessage::Busy);
                }
            });
        }
        true
    }

    /// Every player skates for `duration` from `start`: a frame each tick, the run sent at each
    /// sample (every `interval` ticks).
    fn skate(&mut self, start: Instant, interval: u32, duration: Duration) {
        let until = self.elapsed + duration;
        while self.elapsed < until {
            let now = self.now + STEP;
            self.step(|index, player| {
                while start + player.offset + TICK * player.ticks <= now {
                    let tick = player.first_tick + player.ticks;
                    player.ticks += 1;
                    let sample = tick % interval == 0;
                    player.run.push(frame(index, tick, sample, &mut player.noise));
                    if sample || player.run.len() == MAX_RUN {
                        player.core.send_frames(&player.run);
                        player.frames_sent += player.run.len() as u64;
                        player.run.clear();
                    }
                }
            });
        }
    }
}

/// A free skate search at the game's own location: the first finds none and makes one there.
fn free_skate(game: usize) -> ClientMessage {
    let location = 0x1000 + game as u64;
    ClientMessage::PlayNow(PlayNow {
        mode: PlayNowMode::Freeskate,
        ranked: false,
        challenge_type: challenge::ONLINE_FREE_SKATE,
        challenge_key: Some(location),
        private: false,
        create: false,
        fallback_key: location,
        game_version: VERSION.to_owned(),
    })
}

/// The pass at `per_mille` thousandths of the way through the sorted passes.
fn quantile(sorted: &[Duration], per_mille: usize) -> f64 {
    let index = (sorted.len() * per_mille / 1000).min(sorted.len().saturating_sub(1));
    sorted.get(index).copied().unwrap_or_default().as_secs_f64() * 1000.0
}

#[expect(clippy::cast_precision_loss, reason = "figures for display")]
fn report(sim: &Simulation, settings: &Settings) {
    let played = settings.seconds as f64;
    let games = settings.players.div_ceil(settings.game_size);
    let sizes: Vec<usize> = (0..games).map(|game| sim.players.iter().filter(|player| player.game == game).count()).collect();
    println!(
        "{} players in {games} games {sizes:?}, {} s of play, {} ms each way, a run every {} ticks",
        settings.players,
        settings.seconds,
        settings.delay.as_millis(),
        settings.interval
    );
    let busy: Duration = sim.passes.iter().sum();
    let mut sorted = sim.passes.clone();
    sorted.sort_unstable();
    println!(
        "server: {:.1}% of one core ({:.2} s of work in {} s); a pass took at most {:.2} ms, 99.9% under {:.2} ms, 99% under {:.3} ms",
        busy.as_secs_f64() / played * 100.0,
        busy.as_secs_f64(),
        settings.seconds,
        quantile(&sorted, 1000),
        quantile(&sorted, 999),
        quantile(&sorted, 990),
    );
    let rate = |(datagrams, bytes): (u64, u64)| format!("{:.1} KB/s ({:.0} datagrams/s)", bytes as f64 / 1024.0 / played, datagrams as f64 / played);
    println!("server traffic: in {}, out {}", rate(sim.server_in), rate(sim.server_out));
    let down: u64 = sim.players.iter().map(|player| player.bytes_down).sum();
    let pairs: usize = sim.players.iter().map(|player| sizes[player.game] - 1).sum();
    println!(
        "each player downloads {:.1} KB/s: {:.1} KB/s for each other skater in its game",
        down as f64 / 1024.0 / played / settings.players as f64,
        down as f64 / 1024.0 / played / pairs.max(1) as f64,
    );
    // Every frame a player sent reaches each of its game's other players, and none reaches a
    // player of another game.
    let game_of: HashMap<u64, usize> = sim.players.iter().filter_map(|player| Some((player.user_id?, player.game))).collect();
    let (mut expected, mut delivered, mut astray) = (0_u64, 0_u64, 0_u64);
    for (index, player) in sim.players.iter().enumerate() {
        for (&sender, &frames) in &sim.heard[index] {
            if game_of.get(&sender) == Some(&player.game) {
                delivered += frames;
            } else {
                astray += frames;
            }
        }
        for (other, sender) in sim.players.iter().enumerate() {
            if other != index && sender.game == player.game {
                expected += sender.frames_sent;
            }
        }
    }
    println!(
        "frames: {delivered} of {expected} ({:.2}%) reached their game's other players; {astray} reached another game",
        delivered as f64 / expected.max(1) as f64 * 100.0
    );
}

fn main() {
    let settings = Settings::from_args();
    if cfg!(debug_assertions) {
        println!("a debug build: run with --release for the figures that count");
    }
    let mut sim = Simulation::start(&settings);
    let start = sim.now;
    // A second's skating first: every receiver holds frames of every sender, so the runs are
    // coded against them, as in a game under way.
    sim.skate(start, settings.interval, Duration::from_secs(1));
    sim.measuring = true;
    sim.skate(start, settings.interval, Duration::from_secs(settings.seconds));
    sim.measuring = false;
    // The runs still on their way arrive; no more go out.
    let until = sim.elapsed + settings.delay * 4 + Duration::from_millis(100);
    sim.wait(|sim| sim.elapsed >= until);
    report(&sim, &settings);
}
