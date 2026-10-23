//! An online challenge round: `StateOnlineGame`'s `ChallengeLoadState` (0x82771a28),
//! `ChallengeSkate` (0x82776a78), `WaitingForGameToFinish` (0x82775a78), `SendPlayerResults`
//! (0x82774f08) and `DisplayFinalResults` (0x82772e58).
//!
//! The original ran the challenge script on every console over a lockstep of every pad, so each
//! console saw every skater's output. Here each console simulates only its own skater: every
//! tick it sends that skater's completed output (its "facts", `crate::online::facts`) and runs
//! the challenge system once it holds every player's facts of the tick. Every console runs the
//! same ticks over the same facts with the same `GameReset` seed, so the script reaches the same
//! letters, turns and results everywhere; what the script does to a skater, only that skater's
//! console carries out.

use super::{SelectedLiveError, SelectedLiveSession, error, trace};
use crate::challenge::mission::MissionFrame;
use crate::challenge::players::{OnlineChallenge, PT_HUMAN, PlayersBackEnd};
use crate::challenge::results::{OnlineResultRow, OnlineResults};
use crate::challenge::system::Output;
use crate::online::catalog;
use crate::online::event_log::EventText;
use crate::online::facts::{self, FactsCoder, FactsScope};
use crate::players_backend_settings::PlayersBackendSettings;
use super::super::career::HeldPresentation;
use skate2_net::protocol::{ClientMessage, FactsNotes, GameResult, MAX_PLAYERS, MAX_PRESENTED, Phase, PlayerStats, Presented, reset};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// Challenge ticks one simulation tick may run while catching up with the other players.
const CATCH_UP_TICKS: usize = 4;
/// Ready challenge ticks a round that ran out waits for before it runs on: the other players'
/// facts arrive with their runs of frames, two or three ticks at a time and early or late by the
/// link's jitter, and the ticks they leave waiting are a jitter buffer that keeps the round, and
/// the scenes it plays, at the simulation's pace instead of stopping at every arrival.
const REFILL_LEAD: u32 = 6;
/// Ready challenge ticks a simulation tick still runs one at a time: the buffer and a run's
/// arrival on top of it.
const SMOOTH_LEAD: u32 = 12;
/// Past `SMOOTH_LEAD` a simulation tick runs two; past this, `CATCH_UP_TICKS`.
const CATCH_UP_LEAD: u32 = 30;
/// The most facts kept of another player, the furthest ahead going first: half a minute of its
/// ticks, far past any lead the catch-up leaves. Facts for ticks or runs the round never reaches
/// would otherwise pile up.
const KEPT_FACTS: usize = 1800;

/// The challenge ticks a simulation tick runs with `lead` of them ready.
const fn ticks_for_lead(lead: u32) -> usize {
    if lead == 0 {
        0
    } else if lead <= SMOOTH_LEAD {
        1
    } else if lead <= CATCH_UP_LEAD {
        2
    } else {
        CATCH_UP_TICKS
    }
}

/// The challenge ticks a simulation tick runs with `lead` of them ready, and whether the round
/// then waits to refill: once out of ticks, until [`REFILL_LEAD`] are ready.
const fn pace(lead: u32, refilling: bool) -> (usize, bool) {
    if lead == 0 || (refilling && lead < REFILL_LEAD) { (0, true) } else { (ticks_for_lead(lead), false) }
}
/// The places of the final results' lines: the round's players the results hold, in the
/// round's order. FE state 27's data blocks read `mNumPlayers` lines from the first
/// (`Results_GetLeaderboardDataBlock` 0x826b9608, `Results_GetLeaderboardNameBlock`
/// 0x826b9ba0), and the server's results leave out a player who left the round, so the lines
/// close up over them.
fn result_places(players: &[u64], rows: &[PlayerStats]) -> Vec<u64> {
    players.iter().copied().filter(|&user_id| rows.iter().any(|row| row.user_id == user_id)).collect()
}

/// `GM_OnlineChallenge` and `GM_OnlineFreeSkate` as the session's game info has them.
const GM_ONLINE_CHALLENGE: u32 = 5;
const GM_ONLINE_FREE_SKATE: u32 = 4;

/// `GetChallengeColumnHeader` 0x82605a50 of a challenge type: S.K.A.T.E.'s letters, Death
/// Race's time, otherwise trick points (online free skate's activities are not in the port).
const fn column_header(challenge_type: i32) -> &'static str {
    match challenge_type {
        4 | 5 => "ID_LEADERBOARD_LETTERS_HEADER",
        19 => "ID_LEADERBOARD_HEADER_TIME",
        _ => "ID_LEADERBOARD_HEADER_TRICK_POINTS",
    }
}

/// Where the local player's round is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    /// `ChallengeLoadState`: the challenge's assets load, then `GameChallengeLoaded`.
    Loading { sent: bool },
    /// `ChallengeSkate`.
    Running,
    /// `WaitingForGameToFinish`: `GameComplete` sent.
    Complete,
    /// `DisplayFinalResults`.
    Results,
}

/// Another player's facts as they arrived, decoded in order: by the run of ticks it sent facts
/// for and the tick, with what it told every console at the tick.
struct PlayerFacts {
    coder: FactsCoder,
    frames: BTreeMap<(u32, u32), (MissionFrame, FactsNotes)>,
    left: bool,
    /// The run and tick the player waits at a sync point from, standing still at those facts.
    syncing: Option<(u32, u32)>,
}

impl PlayerFacts {
    /// The facts of a player in a round of `scope`, decoded as the player codes them.
    fn new(scope: FactsScope) -> Self {
        Self { coder: FactsCoder::new(scope), frames: BTreeMap::new(), left: false, syncing: None }
    }
}

/// `StateOnlineGame`'s sync point (`MESSAGE_ChallengeSyncPoint`, `mInSyncPoint`): the round and
/// the local player's spawn point.
struct SyncPoint {
    round: u32,
    spawn: Option<[f32; 16]>,
    /// `GameSyncPoint` went to the server (`ChallengeSkate`'s `mChallengeLoaded`).
    sent: bool,
}

/// `StateOnlineGame::WaitForStreamer` (`0x82748ae8`), which `SetLoadingState(true, true, true)`
/// puts the loading screen up for: `mWaitForStreamerPos` and `mWaitForStreamerTick`, the updates
/// the area there has stayed loaded.
struct StreamerWait {
    position: [f32; 3],
    ticks: u32,
}

/// What an online game shows in place of the game camera.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OnlineView {
    /// A scene's camera.
    Scene,
    /// The camera of the player with this challenge index.
    Player(i32),
}

/// The round in progress.
pub(super) struct Round {
    pub(super) session: u32,
    challenge: u64,
    pub(super) stage: Stage,
    /// The players by challenge index (`GameReset`'s table).
    players: Vec<u64>,
    local: i32,
    /// The next tick of the local skater's facts while it sends them, and the next challenge tick
    /// to run.
    produced: u32,
    processed: u32,
    /// The challenge ticks run, and the first tick since the last sync point.
    ticks: u32,
    base: u32,
    /// The sync points the round has gone through.
    syncs: u32,
    /// What the challenge reads of another player's facts.
    scope: FactsScope,
    coder: FactsCoder,
    /// The local skater's facts by tick: as every console runs the challenge on them, whole (for
    /// what only this console reads of its own skater) and the notes sent with them.
    local_frames: BTreeMap<u32, (MissionFrame, MissionFrame, FactsNotes)>,
    /// The local skater's last whole frame, for the ticks it sends no facts for.
    own: Option<MissionFrame>,
    others: BTreeMap<u64, PlayerFacts>,
    /// The round ran out of other players' facts and waits for [`REFILL_LEAD`] ticks of them.
    refilling: bool,
    /// Simulation ticks the running round ran no tick in, for the trace.
    stalls: u32,
    /// The players whose facts pace the next challenge tick, a bit per player index. In a
    /// turn-based challenge, once a turn has started, the turn player and the players the
    /// challenge has teleported without their output finishing the teleport: a turn runs at its
    /// player's pace, whatever the others' speed, and the others stand still at their last facts
    /// and send none. Otherwise every player still in the round.
    contributors: u8,
    /// S.K.A.T.E., Spot Battle, Hall of Meat and 1-Up: one player plays at a time.
    turn_based: bool,
    /// How many runs of ticks each player has sent facts for, counted alike on every console.
    stints: [u32; MAX_PLAYERS],
    /// Each player's frame of the last tick it sent facts for.
    standing: [Option<MissionFrame>; MAX_PLAYERS],
    /// Votes pressed (player index, score) and players who left while not sending facts, until a
    /// tick's notes count them. A player who left is held by user id and numbered in the notes
    /// by the table then: one who left during the load is not in the challenge's table, which the
    /// server packs again without them, so a load index would name another player.
    votes: BTreeMap<u8, i32>,
    leaving: BTreeSet<u64>,
    /// The facts the local skater sent: ticks and coded bytes, for the trace.
    facts_sent: (u32, usize),
    /// Players whose `cMsgPlayerDisconnected` went to the challenge.
    disconnected: Vec<i32>,
    /// `mMyGameResult`.
    result: Option<GameResult>,
    stance: Arc<Mutex<[bool; MAX_PLAYERS]>>,
    /// The completed tick the challenge last unpinned the local skater at, and how many times
    /// it has, for diagnostics.
    unpinned: Option<u32>,
    turns: u32,
    /// The player `Online.SwitchCamera` last made this console follow.
    camera_player: Option<i32>,
    /// The sync point the challenge waits at.
    sync: Option<SyncPoint>,
    /// Sync points the server let go before this console reached them (its timeout).
    released: BTreeSet<u32>,
    /// `ChallengeLoadState`'s Enter ran: the lobby's travel came first.
    load_entered: bool,
    /// `mWaitingForStreamer`: the loading screen is up.
    streamer: Option<StreamerWait>,
    /// `SKATE2_DIAGNOSTIC_ONLINE_VOTE` voted on the turn the HUD shows.
    diagnostic_voted: bool,
    /// The trigger volumes the local skater entered since the challenge last unpinned it, with
    /// the completed tick, for diagnostics.
    diagnostic_entered: Vec<(u64, u32)>,
    /// `SKATE2_TRACE_ONLINE_INPUTS`: a running hash of the ticks' frames.
    input_hash: u64,
    /// The scenes' and speech lines' events this console saw, numbered (`Presented`), until a
    /// tick delivers them, each with whether this console announced it while leading; the
    /// counts seen and delivered (delivered alike on every console).
    presentation_log: Vec<(Presented, bool)>,
    nis_seen: u32,
    speech_seen: BTreeMap<(i32, i32), u32>,
    nis_delivered: u32,
    speech_delivered: BTreeMap<(i32, i32), u32>,
}

impl Round {
    /// The round from `GameReset(LoadChallenge)`'s table, loading.
    fn loading(session: u32, challenge: u64, players: Vec<u64>, local: i32, scope: FactsScope, turn_based: bool, stance: Arc<Mutex<[bool; MAX_PLAYERS]>>) -> Self {
        Self {
            session,
            challenge,
            stage: Stage::Loading { sent: false },
            players,
            local,
            produced: 0,
            processed: 0,
            ticks: 0,
            base: 0,
            syncs: 0,
            scope,
            coder: FactsCoder::new(scope),
            local_frames: BTreeMap::new(),
            own: None,
            others: BTreeMap::new(),
            refilling: false,
            stalls: 0,
            contributors: u8::MAX,
            turn_based,
            stints: [0; MAX_PLAYERS],
            standing: Default::default(),
            votes: BTreeMap::new(),
            leaving: BTreeSet::new(),
            facts_sent: (0, 0),
            disconnected: Vec::new(),
            result: None,
            stance,
            unpinned: None,
            turns: 0,
            camera_player: None,
            sync: None,
            released: BTreeSet::new(),
            load_entered: false,
            streamer: None,
            diagnostic_voted: false,
            diagnostic_entered: Vec::new(),
            input_hash: 0,
            presentation_log: Vec::new(),
            nis_seen: 0,
            speech_seen: BTreeMap::new(),
            nis_delivered: 0,
            speech_delivered: BTreeMap::new(),
        }
    }

    /// `GameReset(Challenge)`'s table: the players who loaded, packed again without those who
    /// left during the load.
    fn start(&mut self, players: Vec<u64>, local: i32) {
        self.players = players;
        self.leaving.retain(|user_id| self.players.contains(user_id));
        self.local = local;
        self.stage = Stage::Running;
    }

    /// A player of the table left: after their last facts they disconnect, and while they send
    /// none, the next facts' notes count them. One the challenge already disconnected (who left
    /// the challenge, then the game) needs no note.
    fn player_left(&mut self, user_id: u64) {
        let Some(index) = self.player_index(user_id) else { return };
        let scope = self.scope;
        self.others.entry(user_id).or_insert_with(|| PlayerFacts::new(scope)).left = true;
        if i32::try_from(index).is_ok_and(|player| !self.disconnected.contains(&player)) {
            self.leaving.insert(user_id);
        }
    }

    /// The challenge index of `user_id`'s skater.
    pub(super) fn player_index(&self, user_id: u64) -> Option<usize> {
        self.players.iter().position(|&player| player == user_id)
    }

    /// The challenge ticks: started and not waiting at a sync point.
    pub(super) fn running(&self) -> bool {
        matches!(self.stage, Stage::Running | Stage::Complete) && self.sync.is_none()
    }

    fn contributes(&self, index: usize) -> bool {
        self.contributors & (1 << index) != 0
    }

    /// Whether the local skater's facts pace the round.
    fn local_contributes(&self) -> bool {
        usize::try_from(self.local).is_ok_and(|local| self.contributes(local))
    }

    /// Whether the local skater plays the round now: its facts pace the challenge, and the other
    /// players see it play (its turn, or a challenge everyone plays at once).
    pub(super) fn local_plays(&self) -> bool {
        self.running() && self.local_contributes()
    }

    /// Whether the round follows the local skater (`Online.SwitchCamera`). A turn-based challenge
    /// switches every console to the turn's player at the same tick, and no challenge everyone
    /// plays at once switches at all, so while the local skater plays, the others watch it.
    pub(super) fn local_followed(&self) -> bool {
        self.camera_player == Some(self.local)
    }

    /// Whether this console leads the round's presentation: it paces the round, and no player
    /// before it does.
    fn presentation_leader(&self) -> bool {
        self.local_contributes() && u32::try_from(self.local).is_ok_and(|local| self.contributors.trailing_zeros() == local)
    }

    /// A scene's or speech line's event this console saw, numbered in its kind's order.
    fn log_presentation(&mut self, event: HeldPresentation) {
        let presented = match event {
            HeldPresentation::Nis(notification) => {
                self.nis_seen += 1;
                Presented::Nis { seq: self.nis_seen, notification }
            }
            HeldPresentation::Speech { character, id } => {
                let seen = self.speech_seen.entry((character, id)).or_default();
                *seen += 1;
                Presented::Speech { character, id, seq: *seen }
            }
        };
        self.presentation_log.push((presented, false));
    }

    /// The leader's events no tick has delivered and it has not yet announced; a console that
    /// does not lead announces again whatever is undelivered once it leads.
    fn announce_presentation(&mut self) -> Vec<Presented> {
        let leader = self.presentation_leader();
        let mut announced = Vec::new();
        for (presented, sent) in &mut self.presentation_log {
            if !leader {
                *sent = false;
            } else if !*sent && announced.len() < MAX_PRESENTED {
                *sent = true;
                announced.push(*presented);
            }
        }
        announced
    }

    /// Whether a tick's presentation event is new to the round; what is delivered leaves the log.
    fn deliver_presented(&mut self, presented: Presented) -> bool {
        let new = match presented {
            Presented::Nis { seq, .. } => {
                let new = seq > self.nis_delivered;
                self.nis_delivered = self.nis_delivered.max(seq);
                new
            }
            Presented::Speech { character, id, seq } => {
                let delivered = self.speech_delivered.entry((character, id)).or_default();
                let new = seq > *delivered;
                *delivered = (*delivered).max(seq);
                new
            }
        };
        let (nis, speech) = (self.nis_delivered, &self.speech_delivered);
        self.presentation_log.retain(|(presented, _)| match *presented {
            Presented::Nis { seq, .. } => seq > nis,
            Presented::Speech { character, id, seq } => speech.get(&(character, id)).is_none_or(|&delivered| seq > delivered),
        });
        new
    }

    /// What the local skater's next facts tell every console.
    fn notes(&self) -> FactsNotes {
        let left = self.leaving.iter().filter_map(|&user_id| self.player_index(user_id)).filter_map(|index| u8::try_from(index).ok()).collect();
        FactsNotes { votes: self.votes.iter().map(|(&player, &score)| (player, score)).collect(), left, ..FactsNotes::default() }
    }

    /// The frame a player stands at for a tick it sends no facts for.
    fn standing_frame(&self, index: usize) -> MissionFrame {
        let player = i32::try_from(index).unwrap_or(i32::MAX);
        let frame = self.standing[index].as_ref().map_or_else(|| self.scope.shared(&MissionFrame::default()), facts::standing);
        MissionFrame { player, local: player == self.local, ..frame }
    }

    /// Whether `next_tick` would run `tick` with the round's pacing players as they stand now.
    fn tick_ready(&self, tick: u32) -> bool {
        self.players.iter().enumerate().all(|(index, user_id)| {
            let player = i32::try_from(index).unwrap_or(i32::MAX);
            if self.disconnected.contains(&player) || !self.contributes(index) {
                return true;
            }
            if player == self.local {
                return self.local_frames.contains_key(&tick);
            }
            let stint = self.stints[index];
            self.others.get(user_id).is_some_and(|facts| {
                facts.frames.contains_key(&(stint, tick))
                    || facts.left
                    || facts.syncing.is_some_and(|(at, from)| at == stint && from <= tick)
                    || facts.frames.range((stint, tick.saturating_add(1))..=(stint, u32::MAX)).next().is_some()
            })
        })
    }

    /// How many ticks from the next one are ready, up to `limit`; a tick's pacing can change
    /// before it runs, so this only spaces them.
    fn ready_lead(&self, limit: u32) -> u32 {
        (0..limit).take_while(|&ahead| self.tick_ready(self.processed.saturating_add(ahead))).fold(0, |lead, _| lead + 1)
    }

    /// The tick's frames of every player still in the round, once the facts of every player
    /// pacing it are in; the players who disconnect at it; the local skater's whole frame.
    fn next_tick(&mut self) -> Option<(Vec<MissionFrame>, Vec<i32>, Option<MissionFrame>, Vec<Presented>)> {
        let tick = self.processed;
        let mut frames = Vec::with_capacity(self.players.len());
        let mut notes = Vec::new();
        let mut disconnects = Vec::new();
        let mut own = self.own.clone();
        for (index, &user_id) in self.players.iter().enumerate() {
            let player = i32::try_from(index).unwrap_or(i32::MAX);
            if self.disconnected.contains(&player) {
                continue;
            }
            if !self.contributes(index) {
                frames.push(self.standing_frame(index));
                continue;
            }
            if player == self.local {
                let (shared, whole, note) = self.local_frames.get(&tick)?.clone();
                own = Some(whole);
                frames.push(shared);
                notes.push(note);
                continue;
            }
            let stint = self.stints[index];
            let scope = self.scope;
            let facts = self.others.entry(user_id).or_insert_with(|| PlayerFacts::new(scope));
            if let Some((frame, note)) = facts.frames.get(&(stint, tick)) {
                frames.push(frame.clone());
                notes.push(note.clone());
            } else if facts.left {
                disconnects.push(player);
            } else if facts.syncing.is_some_and(|(at, from)| at == stint && from <= tick) {
                frames.push(self.standing_frame(index));
            } else if facts.frames.range((stint, tick.saturating_add(1))..=(stint, u32::MAX)).next().is_some() {
                // A player's facts arrive in order, so a later tick of the run without this one
                // means the player began the run after it: it stands at this tick.
                trace(format_args!("player {player} sent no facts for tick {tick} of run {stint}; it stands at it"));
                frames.push(self.standing_frame(index));
            } else {
                return None;
            }
        }
        for note in &notes {
            for &(voter, score) in &note.votes {
                if let Some(frame) = frames.iter_mut().find(|frame| frame.player == i32::from(voter)) {
                    if frame.vote != Some(score) {
                        trace(format_args!("tick {tick}: player {voter} votes {score}"));
                    }
                    frame.vote = Some(score);
                }
                if self.votes.get(&voter) == Some(&score) {
                    self.votes.remove(&voter);
                }
            }
            for &left in &note.left {
                let player = i32::from(left);
                if !self.contributes(usize::from(left)) && !self.disconnected.contains(&player) && !disconnects.contains(&player) {
                    disconnects.push(player);
                }
            }
        }
        frames.retain(|frame| !disconnects.contains(&frame.player));
        for frame in &frames {
            if let Some(index) = usize::try_from(frame.player).ok().filter(|&index| index < MAX_PLAYERS && self.contributes(index)) {
                self.standing[index] = Some(frame.clone());
            }
        }
        self.local_frames.remove(&tick);
        for facts in self.others.values_mut() {
            facts.frames.retain(|&(_, at), _| at > tick);
        }
        for &player in &disconnects {
            if let Some(user_id) = usize::try_from(player).ok().and_then(|index| self.players.get(index)) {
                self.leaving.remove(user_id);
            }
        }
        self.disconnected.extend(&disconnects);
        frames.sort_by_key(|frame| frame.player);
        if let Ok(mut stance) = self.stance.lock() {
            for frame in &frames {
                if let Some(row) = usize::try_from(frame.player).ok().and_then(|player| stance.get_mut(player)) {
                    *row = frame.is_regular_stance;
                }
            }
        }
        let candidates: Vec<Presented> = notes.iter().flat_map(|note| note.presented.iter().copied()).collect();
        let presented = candidates.into_iter().filter(|&presented| self.deliver_presented(presented)).collect();
        self.processed += 1;
        self.ticks += 1;
        Some((frames, disconnects, own, presented))
    }

    /// The players pacing the next tick, after the challenge ran this one. A player who starts
    /// sending facts begins a new run of them; the local skater sends from the next tick on, or
    /// stops and drops what it sent ahead.
    fn set_contributors(&mut self, next: u8) {
        for index in 0..self.players.len().min(MAX_PLAYERS) {
            let bit = 1 << index;
            let local = i32::try_from(index).is_ok_and(|index| index == self.local);
            if next & bit != 0 && self.contributors & bit == 0 {
                self.stints[index] += 1;
                if local {
                    self.produced = self.processed;
                }
            } else if local && next & bit == 0 && self.contributors & bit != 0 {
                let processed = self.processed;
                self.local_frames.retain(|&tick, _| tick < processed);
            }
        }
        self.contributors = next;
    }
}

/// The players back end of an online challenge (`StateOnlineGame::UpdateGameInfo` 0x827538c8):
/// a `PT_Human` per player of the `GameReset` table with the player's name; the local player's
/// rows are the profile's.
struct OnlinePlayers {
    settings: Arc<PlayersBackendSettings>,
    /// The local skater's own row in the profile's back end.
    row: i32,
    local: i32,
    names: Vec<String>,
    peers: Vec<u64>,
    stance: Arc<Mutex<[bool; MAX_PLAYERS]>>,
}

impl OnlinePlayers {
    fn local_row(&self, player: i32) -> Option<i32> {
        (player == self.local).then_some(self.row)
    }

    fn count(&self) -> i32 {
        i32::try_from(self.names.len()).unwrap_or(0)
    }
}

impl PlayersBackEnd for OnlinePlayers {
    fn player_type(&self, player: i32) -> i32 {
        if (0..self.count()).contains(&player) { PT_HUMAN } else { 0 }
    }
    fn player_name(&self, player: i32) -> String {
        usize::try_from(player).ok().and_then(|player| self.names.get(player)).cloned().unwrap_or_default()
    }
    fn is_stance_regular(&self, player: i32) -> bool {
        match self.local_row(player) {
            Some(row) => self.settings.is_stance_regular(row),
            None => usize::try_from(player).ok().and_then(|player| self.stance.lock().ok().map(|stance| stance.get(player).copied().unwrap_or(true))).unwrap_or(true),
        }
    }
    fn peer_id(&self, player: i32) -> i64 {
        usize::try_from(player).ok().and_then(|player| self.peers.get(player)).map_or(-1, |&peer| peer.cast_signed())
    }
    fn set_manuals_are_connectors(&self, player: i32, value: bool) {
        if let Some(row) = self.local_row(player) {
            self.settings.set_manuals_are_connectors(row, value);
        }
    }
    fn set_reset_sequence(&self, player: i32, value: bool) {
        if let Some(row) = self.local_row(player) {
            self.settings.set_reset_sequence(row, value);
        }
    }
    fn set_reset_scoring(&self, player: i32, value: bool) {
        if let Some(row) = self.local_row(player) {
            self.settings.set_reset_scoring(row, value);
        }
    }
    fn character(&self, player: i32) -> u64 {
        self.local_row(player).map_or(u64::MAX, |row| self.settings.character(row))
    }
    fn num_players(&self) -> i32 {
        self.count()
    }
    fn set_player_rank(&self, player: i32, rank: i32) {
        if let Some(row) = self.local_row(player) {
            self.settings.set_player_rank(row, rank);
        }
    }
}

/// `GameReset`'s table as players in index order.
fn table(participants: &[u64; MAX_PLAYERS]) -> Vec<u64> {
    participants.iter().copied().take_while(|&user_id| user_id != 0).collect()
}

impl super::Online {
    /// Whether the round's Leave Challenge is offered: the local player plays it and has not
    /// finished it (PC port addition).
    pub(in super::super) fn can_leave_challenge(&self) -> bool {
        self.round.as_ref().is_some_and(|round| round.stage == Stage::Running)
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The round is between `GameReset(LoadChallenge)` and lobby skate.
    pub(in super::super) fn online_challenge_running(&self) -> bool {
        self.online.round.is_some()
    }

    /// `GameReset` (`StateOnlineGame::HandleLobbySkateReset` 0x82776f90 and the challenge
    /// states): a challenge loads, starts, or the game goes back to lobby skate.
    pub(super) fn online_game_reset(&mut self, kind: u8, seed: u32, session: u32, participants: &[u64; MAX_PLAYERS]) -> Result<(), SelectedLiveError> {
        let players = table(participants);
        let local = self.online.user_id.and_then(|user_id| players.iter().position(|&player| player == user_id));
        trace(format_args!("game reset {kind} session {session} players {players:016x?} local {local:?}"));
        // UpdateGameInfo's ResetPlayers (0x82753b44): the scoring settings a challenge gave the
        // players, such as S.K.A.T.E.'s manuals that end a line, end with the reset.
        if let Some(player) = self.inner.player.as_ref() {
            player.selected.owners.services.settings.try_reset_scoring().map_err(error)?;
        }
        if let Some(game) = self.online.game.as_mut() {
            game.phase = match kind {
                reset::LOAD_CHALLENGE => Phase::LoadingChallenge,
                reset::CHALLENGE => Phase::Challenge,
                _ => Phase::LobbySkate,
            };
            game.participants.clone_from(&players);
        }
        self.online.lobby_hud_dirty = true;
        match kind {
            reset::LOAD_CHALLENGE => {
                let Some(local) = local else { return Ok(()) };
                if self.online.round.is_some() {
                    self.end_online_round()?;
                }
                self.start_online_round(session, players, local)?;
            }
            reset::CHALLENGE => {
                let matches = self.online.round.as_ref().is_some_and(|round| round.session == session);
                match local {
                    Some(local) if matches => self.start_online_challenge(seed, players, local)?,
                    _ => self.end_online_round()?,
                }
            }
            _ => {
                // The host's GameChange ends at a lobby reset at the new challenge.
                let changed = self.online.changing_location.take().is_some();
                if self.online.round.is_some() {
                    self.end_online_round()?;
                } else if changed {
                    self.online_travel_to_location()?;
                }
            }
        }
        Ok(())
    }

    fn online_players_backend(&self, players: &[u64], local: usize, stance: &Arc<Mutex<[bool; MAX_PLAYERS]>>) -> Result<OnlinePlayers, SelectedLiveError> {
        let player = self.inner.player.as_ref().ok_or_else(|| error("player absent"))?;
        let game = self.online.game.as_ref().ok_or_else(|| error("online game absent"))?;
        let names: Vec<String> = players.iter().map(|&user_id| game.name(user_id)).collect();
        let row = self.inner.bound_input.as_ref().and_then(|input| i32::try_from(input.player_index()).ok()).unwrap_or(0);
        let backend = OnlinePlayers {
            settings: Arc::clone(&player.selected.owners.services.settings),
            row,
            local: i32::try_from(local).unwrap_or(0),
            names,
            peers: players.to_vec(),
            stance: Arc::clone(stance),
        };
        Ok(backend)
    }

    /// `ChallengeLoadState`: the challenge system of the round and its assets.
    fn start_online_round(&mut self, session: u32, players: Vec<u64>, local: usize) -> Result<(), SelectedLiveError> {
        let (challenge, net_type) = self.online.game.as_ref().map(|game| (game.attributes.challenge_key, game.attributes.challenge_type)).ok_or_else(|| error("online game absent"))?;
        let scope = catalog::game_challenge_type(net_type).map_or(FactsScope::ALL, FactsScope::for_challenge_type);
        let stance = Arc::new(Mutex::new([true; MAX_PLAYERS]));
        let backend = self.online_players_backend(&players, local, &stance)?;
        let local = i32::try_from(local).unwrap_or(0);
        let online = OnlineChallenge { local_player: local, players: i32::try_from(players.len()).unwrap_or(0), ..OnlineChallenge::default() };
        self.start_online_challenge_load(challenge, Arc::new(backend), online)?;
        trace(format_args!("loading challenge {challenge:016x} as player {local} of {}", players.len()));
        let turn_based = catalog::game_challenge_type(net_type).is_some_and(|kind| matches!(kind, 4 | 9 | 31 | 32));
        self.online.round = Some(Round::loading(session, challenge, players, local, scope, turn_based, stance));
        Ok(())
    }

    /// `GameReset(Challenge)` (`HandleLobbySkateReset` 0x82776f90): `PopAllStates`, then
    /// `ResetGame` with the server's seed and `StartMainChallenge` in `GM_OnlineChallenge` with
    /// the players who loaded. `ChallengeSkate`'s Enter: "Disabling pause of sim!"
    /// (`kPAUSE_STATE_ONLINE` pulled, with `Net::ResumeGame`); the loading screen stays up.
    fn start_online_challenge(&mut self, seed: u32, players: Vec<u64>, local: usize) -> Result<(), SelectedLiveError> {
        self.pop_all_front_end_states()?;
        if let Some(input) = self.inner.bound_input.as_ref() {
            input.try_set_online_pause(false).map_err(error)?;
        }
        let stance = self.online.round.as_ref().map(|round| Arc::clone(&round.stance)).ok_or_else(|| error("round absent"))?;
        let backend = self.online_players_backend(&players, local, &stance)?;
        let mut random = self.inner.bound_input.as_mut().map(|input| input.simulation_random_mut().clone()).ok_or_else(|| error("input absent"))?;
        random.set_seed_before_input_reset(seed);
        self.set_game_mode(GM_ONLINE_CHALLENGE, None)?;
        self.pause_menu.set_hub(crate::ui::pause_menu::StartHub::OnlineChallenge);
        let local = i32::try_from(local).unwrap_or(0);
        let count = i32::try_from(players.len()).unwrap_or(0);
        let round = self.online.round.as_mut().ok_or_else(|| error("round absent"))?;
        round.start(players, local);
        let challenge = round.challenge;
        let career = self.career.as_mut().ok_or_else(|| error("online challenge system absent"))?;
        let world = career.system.world_mut();
        world.players = Some(Arc::new(backend));
        world.local_player = local;
        world.online = Some(OnlineChallenge { local_player: local, players: count, ..OnlineChallenge::default() });
        career.online_random = Some(random);
        career.start_online_main_challenge(challenge);
        trace(format_args!("challenge {challenge:016x} started as player {local} of {count}"));
        Ok(())
    }

    /// `StateManagerFrontEndBase::PopAllStates` (`0x82d69f18`): back to the idle HUD state, each
    /// front-end state over the game leaving with its `OnLeave` (`UpdateOnLeave` 0x82d66300),
    /// which saves nothing, and a pausing state's pause going with it.
    fn pop_all_front_end_states(&mut self) -> Result<(), SelectedLiveError> {
        // The music player, then Options (`OnLeave` 0x8266af70) under it or on its own.
        self.audio.menu.close();
        self.options_menu.leave();
        self.skate_profile.close();
        // `FrontEndState_OnlineHubMenu::OnLeave` (0x82661228) closes its popup with its screen.
        self.close_online_hub()?;
        // `FrontEndState_StartHub::OnLeave` (0x8264c4d8), then `UnpauseGame`.
        if self.pause_menu.snapshot().visible {
            self.pause_menu.close();
            if let Some(input) = self.inner.bound_input.as_ref() {
                input.try_set_frontend_pause(false).map_err(error)?;
            }
        }
        Ok(())
    }

    /// Lobby skate again (`SetupGame` 0x82778530 after the round): the round's challenge system
    /// goes and the local skater returns to the game's location.
    pub(super) fn end_online_round(&mut self) -> Result<(), SelectedLiveError> {
        let Some(round) = self.online.round.take() else { return Ok(()) };
        let (ticks, bytes) = round.facts_sent;
        trace(format_args!("facts sent: {ticks} of {} ticks, {bytes} bytes", round.ticks));
        trace(format_args!("back to lobby skate"));
        // A round that ends at a sync point leaves no kPAUSE_STATE_ONLINE behind.
        if let Some(input) = self.inner.bound_input.as_ref() {
            input.try_set_online_pause(false).map_err(error)?;
        }
        if let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()) {
            actor.try_set_pinned(false).map_err(|e| error(format!("{e:?}")))?;
        }
        self.unload_career_living_world()?;
        self.unload_career_dynamic_hulls()?;
        self.unload_career_activities()?;
        self.end_career_hud();
        self.drop_career()?;
        // The lobby's own free-skate hulls, kept through the round, go as well: the travel waits
        // until the last game's hulls are all unloaded (GameModule::GameLoad, IsAllUnloaded
        // 0x82da1680), then the new free-skate game loads its own.
        self.unload_career_dynamic_hulls()?;
        self.freeskate.hulls = None;
        if self.online.game.as_ref().is_some_and(|game| game.entered) {
            self.set_game_mode(GM_ONLINE_FREE_SKATE, None)?;
            self.pause_menu.set_hub(crate::ui::pause_menu::StartHub::OnlineFreeSkate);
            self.freeskate.hulls = Some(self.online_dynamic_hulls()?);
            self.online_travel_to_location()?;
        }
        // The lobby's travel takes the round's loading screen over; without one it comes down.
        if round.streamer.is_some() && self.map_travel.is_none() {
            self.frontend_fade.reveal();
        }
        Ok(())
    }

    /// The local skater goes to the game's challenge location behind the fade, on its board.
    fn online_travel_to_location(&mut self) -> Result<(), SelectedLiveError> {
        let Some((attributes, slot)) = self.online.game.as_ref().filter(|game| game.entered).map(|game| (game.attributes.clone(), game.local_slot)) else { return Ok(()) };
        let start = self.online_location(&attributes, slot)?;
        self.online_load_area()?;
        self.map_travel = Some(start);
        self.map_travel_presentation_ready = false;
        self.map_travel_dispatched = false;
        self.map_travel_on_board = true;
        self.frontend_fade.cover();
        Ok(())
    }

    /// Another player's facts of a tick, decoded in their order whatever their run.
    pub(super) fn online_facts(&mut self, user_id: u64, tick: u32, stint: u32, notes: FactsNotes, data: &[u8]) {
        let Some(round) = self.online.round.as_mut() else { return };
        let Some(index) = round.players.iter().position(|&player| player == user_id) else { return };
        let scope = round.scope;
        let facts = round.others.entry(user_id).or_insert_with(|| PlayerFacts::new(scope));
        match facts.coder.decode(data, i32::try_from(index).unwrap_or(i32::MAX)) {
            Ok(frame) => {
                if notes.syncing && tick >= round.base {
                    facts.syncing = Some((stint, tick));
                }
                facts.frames.insert((stint, tick), (frame, notes));
                while facts.frames.len() > KEPT_FACTS {
                    facts.frames.pop_last();
                }
            }
            Err(e) => trace(format_args!("{user_id:016x}: facts of tick {tick}: {e}")),
        }
    }

    /// Another player's vote press, for the notes of the next facts this console sends.
    pub(super) fn online_vote(&mut self, user_id: u64, score: i32) {
        let Some(round) = self.online.round.as_mut() else { return };
        let Some(index) = round.players.iter().position(|&player| player == user_id).and_then(|index| u8::try_from(index).ok()) else { return };
        trace(format_args!("player {index} pressed vote {score}"));
        round.votes.insert(index, score);
    }

    /// `Online.SwitchCamera` (0x8288a4e0, `SkaterManager::SetLocalIndex`): the console follows
    /// the player's skater, its own or another player's.
    pub(in super::super) fn online_switch_camera(&mut self, player: i32) {
        if let Some(round) = self.online.round.as_mut() {
            trace(format_args!("camera follows player {player}"));
            round.camera_player = Some(player);
        }
    }

    /// The remote skater the console follows: the player `Online.SwitchCamera` last switched to,
    /// when that is another player's skater that shows.
    pub(super) fn online_spectated(&self) -> Option<&super::remote_skater::RemoteSkater> {
        let round = self.online.round.as_ref()?;
        let player = round.camera_player.filter(|&player| player != round.local)?;
        let user_id = *round.players.get(usize::try_from(player).ok()?)?;
        self.online.game.as_ref()?.remotes.get(&user_id).filter(|remote| remote.visible())
    }

    /// What an online game shows in place of the game camera, as it keeps simulating under a
    /// scene (`Net::ResumeGame`): a scene's camera while one plays (a challenge presentation's
    /// NIS), else the camera of the other player `Online.SwitchCamera` follows.
    fn online_view(&self) -> Option<(OnlineView, [f32; 16], f32)> {
        self.online.game.as_ref()?;
        if let (Some(scene), _) = self.career_nis_camera()
            && let Some(matrix) = scene.matrix
        {
            return Some((OnlineView::Scene, matrix, scene.fov));
        }
        let player = self.online.round.as_ref()?.camera_player?;
        let camera = self.online_spectated()?.camera()?;
        Some((OnlineView::Player(player), camera.matrix, camera.fov))
    }

    /// The online view's camera for the frame's visual snapshot, with this console's near plane
    /// (the scene's own while a scene shows).
    pub(in super::super) fn online_view_camera(&self) -> Option<crate::nis::manager::NisCamera> {
        let (view, matrix, fov) = self.online_view()?;
        let near_plane = match (view, self.career_nis_camera()) {
            (OnlineView::Scene, (Some(scene), _)) => scene.near_plane,
            _ => self.inner.world.as_ref()?.local.try_camera_producer_projection().ok()?.near_plane,
        };
        Some(crate::nis::manager::NisCamera { matrix: Some(matrix), fov, near_plane })
    }

    /// `CameraPresentation::DoSimSubmit` (0x827f23a8) in an online game: the submitted frames are
    /// the online view's camera, which the restore then shows, cut (`mbHardCut`) whenever the
    /// view changes so it never blends between a scene and a player's camera, or two players'.
    pub(in super::super) fn online_camera_submission(&mut self, camera: &mut crate::presentation_restore::camera::CameraSubmission) {
        let view = self.online_view();
        let shown = view.map(|(view, _, _)| view);
        let cut = shown != self.online.view;
        self.online.view = shown;
        for frame in &mut camera.frames {
            if let Some((_, matrix, fov)) = view {
                let matrix = crate::skeleton_root_transforms::SkeletonRootMatrix::known(matrix);
                frame.matrix = matrix;
                frame.previous_matrix = matrix;
                frame.velocity = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
                frame.angular_velocity = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
                frame.field_of_view = fov;
                // The console's own camera cutting (its collision) is not a cut of this view.
                frame.hard_cut = cut;
            } else {
                frame.hard_cut |= cut;
            }
        }
    }

    /// The hips and bounds of the skater the console follows, which the local skater's shadow
    /// follows as `GetLocalIndex` (0x827d0090) does.
    pub(in super::super) fn online_spectated_shadow(&self) -> Option<([f32; 3], [[f32; 3]; 2])> {
        self.online_spectated()?.visual().map(|visual| (visual.hips_position, visual.world_bounds))
    }

    /// The root of the skater the console follows, where `Pres_OnlineStreamerController::
    /// UpdateStreamer` (0x8277d310) moves the presentation's stream focus: the root of the skater
    /// at `GetLocalIndex`, which `Online.SwitchCamera` sets.
    pub(in super::super) fn online_spectated_root(&self) -> Option<[f32; 3]> {
        self.online_spectated()?.root()
    }

    /// The challenge pinned or unpinned the local skater (`cMsgPinPlayer`).
    pub(in super::super) fn online_local_pinned(&mut self, pin: bool) {
        let tick = self.completed_frames;
        if let Some(round) = self.online.round.as_mut() {
            if pin {
                round.unpinned = None;
            } else if round.unpinned.is_none() {
                round.unpinned = Some(tick);
                round.turns += 1;
                round.diagnostic_entered.clear();
            }
        }
    }

    /// The local skater's tester entered a trigger volume, for diagnostics.
    pub(in super::super) fn online_local_entered(&mut self, volume: u64) {
        let tick = self.completed_frames;
        if let Some(round) = self.online.round.as_mut().filter(|round| round.unpinned.is_some()) {
            round.diagnostic_entered.push((volume, tick));
        }
    }

    /// Diagnostics: completed ticks since the local skater last entered the trigger volume
    /// (its local id) since the challenge unpinned it.
    #[must_use]
    pub fn diagnostic_online_entered(&self, volume: u64) -> Option<u32> {
        let round = self.online.round.as_ref()?;
        let &(_, tick) = round.diagnostic_entered.iter().rev().find(|&&(entered, _)| entered == volume)?;
        Some(self.completed_frames.saturating_sub(tick))
    }

    /// Diagnostics: completed ticks since the online challenge unpinned the local skater, and
    /// how many times it has.
    #[must_use]
    pub fn diagnostic_online_turn(&self) -> Option<(u32, u32)> {
        let round = self.online.round.as_ref()?;
        Some((self.completed_frames.saturating_sub(round.unpinned?), round.turns))
    }

    /// Whether a running round has the local skater pinned, for diagnostics.
    #[must_use]
    pub fn diagnostic_online_pinned(&self) -> bool {
        self.online.round.as_ref().is_some_and(|round| round.running() && round.unpinned.is_none())
    }

    /// A player left during the round: after their last facts they disconnect.
    pub(super) fn online_round_player_left(&mut self, user_id: u64) {
        if let Some(round) = self.online.round.as_mut() {
            round.player_left(user_id);
        }
    }

    /// Whether `user_id` waits in the pre-game lobby, apart from the round the local player plays:
    /// a member the round's `GameReset` table does not hold, who joined after its load (a PC port
    /// addition: the original takes no join while a challenge runs).
    pub(super) fn online_apart_from_round(&self, user_id: u64) -> bool {
        self.online.round.as_ref().is_some_and(|round| !round.players.contains(&user_id))
    }

    /// The pause menu's Leave Challenge answered yes: the round ends at the next online update,
    /// where a lobby reset ends one.
    pub(in super::super) fn online_request_leave_challenge(&mut self) {
        self.online.leave_challenge = true;
    }

    /// Leave Challenge (PC port addition): the server takes the local player out of the round,
    /// which goes on for the others as without a player who left the game, and the local game
    /// goes back to lobby skate at the game's location, where it waits in the pre-game lobby and
    /// watches the round's players as ghosts until the game is back in lobby skate.
    pub(super) fn online_leave_challenge(&mut self) -> Result<(), SelectedLiveError> {
        if !self.online.can_leave_challenge() {
            return Ok(());
        }
        trace(format_args!("leaving the challenge for the pre-game lobby"));
        self.online.send(ClientMessage::LeaveChallenge);
        if let Some(game) = self.online.game.as_mut()
            && let Some(local) = self.online.user_id
        {
            game.participants.retain(|&participant| participant != local);
        }
        // The local player abandons the round: the audio system's cMsgAbandonChallenge handler
        // (QuitChallenge 0x824ff478) ends its mission music, and what its challenge gave the
        // players' scoring ends as UpdateGameInfo's ResetPlayers (0x82753b44) ends it at a reset.
        self.quit_career_challenge_music();
        if let Some(player) = self.inner.player.as_ref() {
            player.selected.owners.services.settings.try_reset_scoring().map_err(error)?;
        }
        self.end_online_round()
    }

    /// `ChallengeLeave` (PC port addition): a player left the round for the pre-game lobby. To the
    /// round's other players they leave as one who left the game does (their facts end, then they
    /// disconnect) and their skater goes at once; the members waiting see them as themselves, no
    /// longer a ghost. Everyone else's log names them.
    pub(super) fn online_challenge_leave(&mut self, user_id: u64) {
        let local = Some(user_id) == self.online.user_id;
        let played = !local && self.online.round.as_ref().is_some_and(|round| round.players.contains(&user_id));
        if played {
            self.online_round_player_left(user_id);
        }
        let Some(game) = self.online.game.as_mut() else { return };
        game.participants.retain(|&participant| participant != user_id);
        if local {
            return;
        }
        trace(format_args!("{} left the challenge", game.name(user_id)));
        if played && let Some(remote) = game.remotes.get_mut(&user_id) {
            remote.hide();
        }
        if game.entered {
            self.online.event_log.add(EventText::literal(format!("{} has left the challenge.", game.name(user_id))));
        }
    }

    /// The players pacing the next challenge tick (`Round::contributors`). Pins do not count:
    /// a script pins `vars.LocalPlayerID`, a different player on each console.
    fn online_contributors(&self) -> Option<u8> {
        let round = self.online.round.as_ref()?;
        let online = self.career.as_ref()?.system.world().online.as_ref()?;
        let players = round.players.len().min(MAX_PLAYERS);
        let present = |index: usize| {
            !online.disconnected.get(index).copied().unwrap_or(false) && i32::try_from(index).is_ok_and(|player| !round.disconnected.contains(&player))
        };
        let turn = online.turn_player.and_then(|player| usize::try_from(player).ok()).filter(|&index| round.turn_based && index < players && present(index));
        let contributors = (0..players)
            .filter(|&index| present(index) && turn.is_none_or(|turn| index == turn || online.teleporting[index]))
            .fold(0_u8, |bits, index| bits | (1 << index));
        Some(contributors)
    }

    /// `StateOnlineGame`'s update of the round, which the flow runs every frame whatever
    /// front-end screen is up (a pause menu or Options open on one console would otherwise hold
    /// every player's start until it closed):
    /// - `ChallengeLoadState` (`0x82771a28`), entered once the lobby's travel is done
    ///   (`LobbySkate` handles no reset while its streamer wait is up): the round's challenge
    ///   system loads, and `GameChallengeLoaded` goes to the server once its assets are in and
    ///   the loading screen's streamer wait holds.
    /// - `ChallengeSkate` (`0x82776a78`): at a sync point, `GameSyncPoint` once the streamer wait
    ///   holds and no challenge hull is loading; otherwise the loading screen comes down once the
    ///   streamer wait holds.
    pub(in super::super) fn update_online_round(&mut self) -> Result<(), SelectedLiveError> {
        let Some(round) = self.online.round.as_ref() else { return Ok(()) };
        match round.stage {
            Stage::Loading { sent } => {
                if self.map_travel.is_some() {
                    return Ok(());
                }
                if !round.load_entered {
                    self.enter_online_challenge_load()?;
                }
                let Some(round) = self.online.round.as_ref() else { return Ok(()) };
                let (local, challenge) = (round.local, round.challenge);
                self.career_step(Some((Vec::new(), local, None)))?;
                let streamed = !self.update_online_loading_screen(false)?;
                if !sent && streamed && self.career.as_ref().is_some_and(|career| career.challenge_assets_loaded(challenge)) {
                    trace(format_args!("challenge loaded"));
                    self.online.send(ClientMessage::ChallengeLoaded);
                    if let Some(round) = self.online.round.as_mut() {
                        round.stage = Stage::Loading { sent: true };
                    }
                }
            }
            Stage::Running | Stage::Complete => {
                let Some((sync_round, sent)) = round.sync.as_ref().map(|sync| (sync.round, sync.sent)) else {
                    self.update_online_loading_screen(true)?;
                    return Ok(());
                };
                let streamed = !self.update_online_loading_screen(false)?;
                let hulls = self.career.as_ref().is_some_and(|career| career.loading_challenge_hulls());
                if !sent && streamed && !hulls {
                    trace(format_args!("round {sync_round} sync point reached"));
                    if let Some(sync) = self.online.round.as_mut().and_then(|round| round.sync.as_mut()) {
                        sync.sent = true;
                    }
                    self.online.send(ClientMessage::SyncPoint { round: sync_round });
                }
            }
            Stage::Results => {}
        }
        Ok(())
    }

    /// `ChallengeLoadState`'s Enter (`0x82771bfc`): "Pausing the sim!" (`kPAUSE_STATE_ONLINE`
    /// pulled and pushed, with `Net::PauseGame`), then `WaitForStreamer` at the local player's
    /// `Net::GetStreamerPosition`, its spawn point at the challenge's location.
    fn enter_online_challenge_load(&mut self) -> Result<(), SelectedLiveError> {
        if let Some(input) = self.inner.bound_input.as_ref() {
            input.try_set_online_pause(true).map_err(error)?;
        }
        let attributes = self.online.game.as_ref().map(|game| game.attributes.clone()).ok_or_else(|| error("online game absent"))?;
        let local = self.online.round.as_ref().map_or(0, |round| round.local);
        let streamer = self.online_location(&attributes, u8::try_from(local).unwrap_or(0))?;
        self.online_wait_for_streamer([streamer[12], streamer[13], streamer[14]]);
        if let Some(round) = self.online.round.as_mut() {
            round.load_entered = true;
        }
        Ok(())
    }

    /// `WaitForStreamer` (`0x82748ae8`): `SetLoadingState(true, true, true)` puts the loading
    /// screen up (`EnableBackground` snapped black; no pause menu, online hub or backpack opens
    /// while it holds) and the wait starts at `position`. The stream focus stays where the
    /// online game's area holds it, and `CloseAllPopups(false)` closes only popups that take no
    /// input, which an online game has none of.
    fn online_wait_for_streamer(&mut self, position: [f32; 3]) {
        trace(format_args!("loading screen up, streamer at {position:?}"));
        self.frontend_fade.cover_snap();
        if let Some(round) = self.online.round.as_mut() {
            round.streamer = Some(StreamerWait { position, ticks: 0 });
        }
    }

    /// `UpdateLoadingScreenState` (`0x82748970`): whether the loading screen's wait goes on. The
    /// area at its position (`IsAreaLoaded`, 30 m) or the stream focus (`IsStreamFocusLoaded`)
    /// has to stay loaded for more than 15 updates; then `take_down` ends the wait
    /// (`SetLoadingState(false, false, false)`: the background fades out and the pause menu
    /// opens again), and without it the count starts over.
    fn update_online_loading_screen(&mut self, take_down: bool) -> Result<bool, SelectedLiveError> {
        let Some(position) = self.online.round.as_ref().and_then(|round| round.streamer.as_ref()).map(|wait| wait.position) else { return Ok(false) };
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("online terrain absent"))?;
        let loaded = terrain.try_area_loaded(position, 30.0).map_err(|e| error(format!("{e:?}")))?
            || terrain.try_focus_loaded().map_err(|e| error(format!("{e:?}")))?;
        let Some(round) = self.online.round.as_mut() else { return Ok(false) };
        let Some(wait) = round.streamer.as_mut() else { return Ok(false) };
        if !loaded {
            wait.ticks = 0;
            return Ok(true);
        }
        wait.ticks += 1;
        if wait.ticks <= 15 {
            return Ok(true);
        }
        if take_down {
            round.streamer = None;
            trace(format_args!("loading screen down"));
            self.frontend_fade.reveal();
        } else {
            wait.ticks = 0;
        }
        Ok(false)
    }

    /// The loading screen is up (`mbDisableBackpackAndPauseMenu`): no pause menu or online hub
    /// opens.
    pub(in super::super) fn online_loading_screen(&self) -> bool {
        self.online.round.as_ref().is_some_and(|round| round.streamer.is_some())
    }

    /// `ChallengeLoadState` holds the simulation (`kPAUSE_STATE_ONLINE` with `Net::PauseGame`).
    pub(in super::super) fn online_round_loading(&self) -> bool {
        self.online.round.as_ref().is_some_and(|round| round.load_entered && matches!(round.stage, Stage::Loading { .. }))
    }

    /// A simulation tick of the round: the local facts out and every challenge tick whose facts
    /// are all in (a loading round loads in `update_online_round`).
    pub(in super::super) fn update_online_challenge(&mut self) -> Result<(), SelectedLiveError> {
        let Some(round) = self.online.round.as_ref() else { return Ok(()) };
        match round.stage {
            Stage::Loading { .. } => Ok(()),
            Stage::Running | Stage::Complete => {
                let local = round.local;
                self.take_held_presentation();
                self.produce_online_facts()?;
                self.diagnostic_race_gates(local)?;
                let completed = self.completed_frames;
                let ticks = self.online.round.as_mut().map_or(0, |round| {
                    let lead = round.ready_lead(CATCH_UP_LEAD + 1);
                    let (ticks, refilling) = pace(lead, round.refilling);
                    round.refilling = refilling;
                    if ticks == 0 && round.running() && round.processed > 0 {
                        round.stalls += 1;
                    }
                    if completed % 600 == 0 && round.processed > 0 {
                        trace(format_args!("round at tick {}: {} ready, stalled {} of the last 600 ticks", round.processed, lead, std::mem::take(&mut round.stalls)));
                    }
                    ticks
                });
                for _ in 0..ticks {
                    let Some((frames, disconnects, own, presented)) = self.online.round.as_mut().filter(|round| round.running()).and_then(Round::next_tick) else { break };
                    if std::env::var_os("SKATE2_TRACE_ONLINE_INPUTS").is_some() {
                        self.trace_online_inputs(&frames);
                    }
                    let local = self.online.round.as_ref().map_or(0, |round| round.local);
                    if let Some(career) = self.career.as_mut() {
                        for player in disconnects {
                            trace(format_args!("player {player} disconnected from the challenge"));
                            career.online_player_disconnected(player);
                        }
                    }
                    let lead = self.online.round.as_ref().filter(|round| round.local_contributes()).map_or(0, |round| round.produced.saturating_sub(round.processed));
                    let tick = self.online.round.as_ref().map_or(0, |round| round.processed);
                    if let Some(career) = self.career.as_mut() {
                        let world = career.system.world_mut();
                        world.timers.display_lead = lead;
                        if let Some(online) = world.online.as_mut() {
                            online.tick = tick;
                        }
                    }
                    self.update_online_round_nis()?;
                    if let Some(career) = self.career.as_mut() {
                        for presented in presented {
                            career.deliver_presentation(match presented {
                                Presented::Nis { notification, .. } => HeldPresentation::Nis(notification),
                                Presented::Speech { character, id, .. } => HeldPresentation::Speech { character, id },
                            });
                        }
                    }
                    self.career_step(Some((frames, local, own)))?;
                    self.take_held_presentation();
                    let voting = self.career.as_ref().is_some_and(|career| career.system.world().voting.in_voting());
                    if let Some(contributors) = self.online_contributors()
                        && let Some(round) = self.online.round.as_mut()
                    {
                        if contributors != round.contributors {
                            trace(format_args!("tick {} paced by {contributors:#04x}", round.processed));
                        }
                        round.set_contributors(contributors);
                        if !voting {
                            round.votes.clear();
                        }
                    }
                    if self.online.round.as_ref().is_none_or(|round| !round.running()) {
                        break;
                    }
                }
                Ok(())
            }
            Stage::Results => Ok(()),
        }
    }

    /// `SKATE2_TRACE_ONLINE_INPUTS`: the running hash of every tick's frames every 60 ticks,
    /// without what only the owner's frame has (the local flag, its pushes); alike on every
    /// console while their challenges take the same facts. `=<from>-<to>` also prints those
    /// ticks' frames.
    fn trace_online_inputs(&mut self, frames: &[MissionFrame]) {
        use std::hash::{Hash as _, Hasher as _};
        let Some(round) = self.online.round.as_mut() else { return };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        round.input_hash.hash(&mut hasher);
        let range = std::env::var("SKATE2_TRACE_ONLINE_INPUTS").ok().and_then(|value| {
            let (from, to) = value.split_once('-')?;
            Some(from.trim().parse::<u32>().ok()?..=to.trim().parse::<u32>().ok()?)
        });
        let tick = round.processed.saturating_sub(1);
        for frame in frames {
            let text = format!("{:?}", MissionFrame { local: false, is_pushing: false, is_physically_pushing: false, is_new_auto_pump: false, ..frame.clone() });
            if range.as_ref().is_some_and(|range| range.contains(&tick)) {
                trace(format_args!("inputs frame @{tick}: {text}"));
            }
            text.hash(&mut hasher);
        }
        round.input_hash = hasher.finish();
        if round.processed % 60 == 0 {
            trace(format_args!("inputs @{}: {:016x}", round.processed, round.input_hash));
        }
    }

    /// The local skater's completed output of this tick, as the round's next facts.
    fn produce_online_facts(&mut self) -> Result<(), SelectedLiveError> {
        let frames = self.career_mission_frames()?;
        let Some(round) = self.online.round.as_mut() else { return Ok(()) };
        let Some(mut own) = frames.into_iter().find(|frame| frame.local) else { return Ok(()) };
        own.player = round.local;
        if let Some(gesture) = own.performed_gesture.filter(|_| round.unpinned.is_none()) {
            trace(format_args!("gesture {gesture} while pinned"));
        }
        round.own = Some(own.clone());
        if !round.local_contributes() {
            return Ok(());
        }
        let shared = round.scope.shared(&own);
        let data = round.coder.encode(&shared);
        let tick = round.produced;
        let mut notes = round.notes();
        notes.presented = round.announce_presentation();
        let stint = usize::try_from(round.local).ok().and_then(|local| round.stints.get(local)).copied().unwrap_or(0);
        round.local_frames.insert(tick, (shared, own, notes.clone()));
        round.produced += 1;
        if round.produced % 300 == 0 {
            trace(format_args!("produced {}, processed {}, paced by {:#04x}", round.produced, round.processed, round.contributors));
        }
        round.facts_sent = (round.facts_sent.0 + 1, round.facts_sent.1 + data.len());
        self.online.queue(ClientMessage::Facts { tick, stint, notes, data });
        Ok(())
    }

    /// What the challenge system asked of the online game: the local result
    /// (`MESSAGE_ChallengeComplete`) and the challenge's clean-up.
    pub(in super::super) fn online_challenge_outputs(&mut self, outputs: Vec<Output>) -> Result<(), SelectedLiveError> {
        for output in outputs {
            match output {
                Output::OnlineChallengeComplete { ranking, score, time, players_choice, finish_reason } => {
                    let Some(round) = self.online.round.as_mut() else { continue };
                    let result = GameResult {
                        event_time: time,
                        score,
                        finish_reason: u32::try_from(finish_reason).unwrap_or(1),
                        ranking: u16::try_from(ranking).unwrap_or(0),
                        players_choice,
                    };
                    trace(format_args!("challenge complete: {result:?}"));
                    round.result = Some(result);
                    if round.stage == Stage::Running {
                        round.stage = Stage::Complete;
                        self.online.send(ClientMessage::Complete);
                    }
                }
                Output::OnlineChallengeCleanupDone => trace(format_args!("challenge cleaned up")),
                Output::OnlineRoundSyncPoint { round, location } => self.online_round_sync_point(round, &location)?,
                Output::OpenEndChallengeOptions(_) => self.online_script_results()?,
                _ => {}
            }
        }
        Ok(())
    }

    /// `FrontEnd.OpenOnlineContinueRetryDialog` in the round: FE state 8's online display on the
    /// script's results, without the quit (`OnEnter` clears `mEnableOnlineQuitButton`).
    fn online_script_results(&mut self) -> Result<(), SelectedLiveError> {
        let Some(challenge) = self.online.round.as_ref().map(|round| round.challenge) else { return Ok(()) };
        let game = self.online.game.as_ref().ok_or_else(|| error("online game absent"))?;
        let net_type = game.attributes.challenge_type;
        let ranked = game.attributes.ranked;
        let challenge_type = catalog::game_challenge_type(net_type).unwrap_or(0);
        let (title, icon) = self.online_challenge_presentation(challenge)?;
        self.online.results_instance += 1;
        let online = OnlineResults {
            title,
            icon,
            free_skate: matches!(challenge_type, 22 | 42),
            ranked,
            column_header: column_header(challenge_type).to_owned(),
            skate: matches!(challenge_type, 4 | 5),
            type_name: catalog::challenge_title(net_type).to_owned(),
            players_choice: -1,
            instance: self.online.results_instance,
            ..OnlineResults::default()
        };
        if let Some(career) = self.career.as_mut() {
            career.show_online_script_results(online);
        }
        Ok(())
    }

    /// `ChallengeVotingManager::Update`'s input (0x825b1a18): while the local player votes on
    /// another player's turn, X, A, B and Y (states 139, 136, 137 and 138) vote bad, OK, good and
    /// sick. The press goes to every console, and the next facts of a player pacing the round
    /// carry it, so every console counts it at one tick.
    pub(in super::super) fn online_vote_input(&mut self, input: crate::ui::pause_menu::PauseMenuInput) {
        use crate::challenge::voting::score;
        let voting = self.career.as_ref().is_some_and(|career| career.system.world().voting.shows_hud());
        // Diagnostics: SKATE2_DIAGNOSTIC_ONLINE_VOTE=<score> votes once on each turn the HUD shows.
        if let Some(round) = self.online.round.as_mut()
            && let Some(diagnostic) = std::env::var("SKATE2_DIAGNOSTIC_ONLINE_VOTE").ok().and_then(|value| value.parse::<i32>().ok())
        {
            let press = voting && !round.diagnostic_voted;
            round.diagnostic_voted = voting;
            if press {
                trace(format_args!("diagnostic vote {diagnostic}"));
                self.online_vote_press(diagnostic);
                return;
            }
        }
        if !voting {
            return;
        }
        let vote = if input.misc {
            score::BAD
        } else if input.select {
            score::OK
        } else if input.cancel {
            score::GOOD
        } else if input.option {
            score::SICK
        } else {
            return;
        };
        self.online_vote_press(vote);
    }

    fn online_vote_press(&mut self, score: i32) {
        let Some(round) = self.online.round.as_mut() else { return };
        if let Ok(local) = u8::try_from(round.local) {
            round.votes.insert(local, score);
        }
        self.online.send(ClientMessage::Vote { score });
    }

    /// The round's challenge ticks run: the NIS manager then updates with them
    /// (`update_online_round_nis`), not with each simulation tick.
    pub(in super::super) fn online_round_ticking(&self) -> bool {
        self.online.round.as_ref().is_some_and(Round::running)
    }

    /// `SimController::Update1`'s NIS manager update, once a challenge tick of an online round:
    /// the original's simulation updates were the lockstep's ticks, so a scene the script plays
    /// ends at the same tick on every console.
    fn update_online_round_nis(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let world = self.inner.world.as_ref().map(|owner| &owner.world);
        let terrain = self.inner.terrain.as_deref();
        career.nis.update_simulation(&mut |start, end| super::super::nis_ground_line(world, terrain, start, end)).map_err(error)?;
        let pauses = career.take_nis_events();
        if let Some(input) = self.inner.bound_input.as_ref() {
            for paused in pauses {
                input.try_set_nis_pause(paused).map_err(error)?;
            }
        }
        Ok(())
    }

    /// The scenes' and speech lines' events the round's challenge has not taken, into the round's
    /// log: the challenge takes them at the tick the pacing leader's facts announce them.
    fn take_held_presentation(&mut self) {
        let Some(career) = self.career.as_mut() else { return };
        career.hold_presentation = true;
        let held = std::mem::take(&mut career.held_presentation);
        if let Some(round) = self.online.round.as_mut() {
            for event in held {
                round.log_presentation(event);
            }
        }
    }

    /// The round waits at a sync point with its simulation paused.
    pub(in super::super) fn online_round_syncing(&self) -> bool {
        self.online.round.as_ref().is_some_and(|round| round.sync.is_some())
    }

    /// `MESSAGE_ChallengeSyncPoint` (`StateOnlineGame::FilterImpl` 0x82771530): the challenge
    /// stops at this tick on every console, its simulation paused (`kPAUSE_STATE_ONLINE`, then
    /// `Net::PauseGame` as the round stops running), and `WaitForStreamer` puts the loading screen
    /// up at the spawn point. `ChallengeSkate` (0x82776a78) sends `GameSyncPoint` once the wait
    /// holds (`update_online_round`), and the round waits until the server has the sync point
    /// from every player.
    fn online_round_sync_point(&mut self, round: i32, location: &str) -> Result<(), SelectedLiveError> {
        let Some(local) = self.online.round.as_ref().map(|state| state.local) else { return Ok(()) };
        let spawn = match self.career.as_ref() {
            Some(career) => career.online_spawn_location(location, local)?,
            None => None,
        };
        let round = u32::try_from(round).unwrap_or(0);
        let tick = self.online.round.as_ref().map_or(0, |state| state.processed);
        trace(format_args!("round {round} sync point at {location} {:?}, tick {tick}", spawn.map(|spawn| [spawn[12], spawn[13], spawn[14]])));
        if let Some(input) = self.inner.bound_input.as_ref() {
            input.try_set_online_pause(true).map_err(error)?;
        }
        let Some(state) = self.online.round.as_mut() else { return Ok(()) };
        state.sync = Some(SyncPoint { round, spawn, sent: false });
        if state.released.remove(&round) {
            return self.online_sync_point_complete();
        }
        // The players still on their way to the sync point stand this console's skater still from
        // its next tick: its simulation is paused until every player is there.
        if state.local_contributes()
            && let Some(own) = state.own.clone()
        {
            let shared = facts::standing(&state.scope.shared(&own));
            let data = state.coder.encode(&shared);
            let tick = state.produced;
            let stint = usize::try_from(state.local).ok().and_then(|local| state.stints.get(local)).copied().unwrap_or(0);
            state.produced += 1;
            state.facts_sent = (state.facts_sent.0 + 1, state.facts_sent.1 + data.len());
            self.online.send(ClientMessage::Facts { tick, stint, notes: FactsNotes { syncing: true, ..FactsNotes::default() }, data });
        }
        // The message's transform is the spawn point (`GetSpawnLocation`). A location the
        // manager lacks has none (natively an unset translation): the stream focus decides.
        self.online_wait_for_streamer(spawn.map_or([0.0; 3], |spawn| [spawn[12], spawn[13], spawn[14]]));
        Ok(())
    }

    /// `GameSyncPoint` from the server: every player has reached the round's sync point.
    pub(super) fn online_sync_point_released(&mut self, round: u32) -> Result<(), SelectedLiveError> {
        let Some(state) = self.online.round.as_mut() else { return Ok(()) };
        if state.sync.as_ref().is_some_and(|sync| sync.round == round) {
            return self.online_sync_point_complete();
        }
        state.released.insert(round);
        Ok(())
    }

    /// `StateOnlineGame::OnMessage`'s sync point completion (0x8276b9cc): `kPAUSE_STATE_ONLINE`
    /// pulled and `Net::ResumeGame` (the round runs again), the local skater's `cMsgTeleport` to
    /// its spawn point (the `cMsgSetPlayer` before it reaches only Party Play's UI), then
    /// `cMsgOnlineSyncComplete`, which every console's next challenge tick delivers.
    fn online_sync_point_complete(&mut self) -> Result<(), SelectedLiveError> {
        let Some(state) = self.online.round.as_mut() else { return Ok(()) };
        let Some(sync) = state.sync.take() else { return Ok(()) };
        let challenge = state.challenge;
        // Every console reached the sync point at a tick of its own (each plays its own player's
        // highlight, for one); all go on from the same tick, the next run of ticks.
        state.syncs += 1;
        state.base = state.syncs << 20;
        state.processed = state.base;
        state.produced = state.base;
        state.local_frames.clear();
        let base = state.base;
        for facts in state.others.values_mut() {
            facts.frames.retain(|&(_, at), _| at >= base);
            facts.syncing = None;
        }
        trace(format_args!("round {} sync point complete", sync.round));
        if let Some(input) = self.inner.bound_input.as_ref() {
            input.try_set_online_pause(false).map_err(error)?;
        }
        if let Some(spawn) = sync.spawn {
            self.online_sync_teleport(spawn)?;
        }
        if let Some(career) = self.career.as_mut() {
            career.online_sync_complete(challenge);
        }
        Ok(())
    }

    /// `GameAllPlayersComplete`: `SendPlayerResults` reports the local result as it stands.
    pub(super) fn online_all_players_complete(&mut self) {
        let Some(round) = self.online.round.as_mut() else { return };
        let completing = round.stage == Stage::Running;
        if completing {
            round.stage = Stage::Complete;
        }
        let result = round.result.unwrap_or_default();
        if completing {
            self.online.send(ClientMessage::Complete);
        }
        self.online.send(ClientMessage::Results(result));
    }

    /// `GameFinalResults` (`StateOnlineGame::OnMessage(MT_GameFinalResults)` 0x8276b060): each
    /// player's line into FE state 27's final results at its place ([`result_places`]), the
    /// players' choice and cash; then `DisplayFinalResults` (0x82772e58) pauses the game and
    /// shows FE state 8 in its online display.
    pub(super) fn online_final_results(&mut self, rows: &[PlayerStats]) -> Result<(), SelectedLiveError> {
        let Some(round) = self.online.round.as_mut() else { return Ok(()) };
        round.stage = Stage::Results;
        let local = self.online.user_id;
        let win = rows.iter().find(|row| Some(row.user_id) == local).is_some_and(|row| row.result.ranking == 1);
        let challenge = round.challenge;
        let places = result_places(&round.players, rows);
        trace(format_args!("final results: {rows:?}"));
        let game = self.online.game.as_ref().ok_or_else(|| error("online game absent"))?;
        let lines: Vec<OnlineResultRow> = rows
            .iter()
            .filter_map(|row| {
                let player = places.iter().position(|&user_id| user_id == row.user_id)?;
                Some(OnlineResultRow {
                    player,
                    name: game.name(row.user_id),
                    position: i32::from(row.result.ranking),
                    score: row.result.score,
                    experience: row.exp,
                    time: row.result.event_time,
                    cash: row.cash,
                    players_choice: row.result.players_choice,
                })
            })
            .collect();
        let net_type = game.attributes.challenge_type;
        let ranked = game.attributes.ranked;
        let challenge_type = catalog::game_challenge_type(net_type).unwrap_or(0);
        let board = OnlineResults::leaderboard(&lines, challenge_type);
        let mut by_rank: Vec<(i32, usize)> = lines.iter().map(|line| (line.position, line.player)).collect();
        by_rank.sort_unstable();
        let mut cash = vec![0; places.len()];
        let mut experience = vec![0.0; places.len()];
        for line in &lines {
            cash[line.player] = line.cash;
            experience[line.player] = line.experience;
        }
        let (title, icon) = self.online_challenge_presentation(challenge)?;
        self.online.results_instance += 1;
        let online = OnlineResults {
            title,
            icon,
            free_skate: matches!(challenge_type, 22 | 42),
            activity_won: false,
            ranked,
            column_header: column_header(challenge_type).to_owned(),
            skate: matches!(challenge_type, 4 | 5),
            last_round: false,
            round: false,
            type_name: catalog::challenge_title(net_type).to_owned(),
            players_choice: lines.iter().find(|line| line.players_choice).map_or(-1, |line| i32::try_from(line.player).unwrap_or(-1)),
            cash,
            experience,
            // The port saves no profile and asks no ranks and badges, so DisplayFinalResults
            // enables the quit at once.
            quit_enabled: true,
            timer: 0,
            round_timer: 0,
            instance: self.online.results_instance,
            selected: 0,
            player_by_rank: by_rank.into_iter().map(|(_, player)| player).collect(),
        };
        if let Some(career) = self.career.as_mut() {
            career.open_online_results(challenge, win, board, online);
        }
        Ok(())
    }

    /// `GetOnlineChallengeInstanceData`'s challenge `Title` and its global data's
    /// `ChallengeIcon` `fe_map_icons` name.
    fn online_challenge_presentation(&mut self, challenge: u64) -> Result<(String, String), SelectedLiveError> {
        const TITLE: u64 = 0xE2A5_0788_B6C2_0C92;
        const GLOBAL_DATA: u64 = 0x2449_1BF0_0316_AA45;
        const CHALLENGE_ICON: u64 = 0xE616_ACDF_5A36_BFAC;
        self.online_data()?;
        let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let title = db.attribute(crate::challenge::keys::class::CHALLENGES, challenge, TITLE).and_then(|value| value.text(0)).unwrap_or_default().to_owned();
        let icon = db
            .attribute(crate::challenge::keys::class::CHALLENGES, challenge, GLOBAL_DATA)
            .and_then(|global| global.ref_spec(0))
            .and_then(|(class, key)| db.attribute(class, key, CHALLENGE_ICON))
            .and_then(|icon| icon.ref_spec(0))
            .and_then(|(_, key)| db.layout_text(crate::challenge::keys::class::MAP_ICON, key, 0))
            .unwrap_or("default")
            .to_owned();
        trace(format_args!("challenge {challenge:016x}: title {title}, icon {icon}"));
        Ok((title, icon))
    }

    /// `GameTimer(DisplayResults)` (`OnlineBackEnd::StartPostEventTimer`): when the board leaves.
    pub(super) fn online_results_timer(&mut self, seconds: u64) {
        self.online.results_deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(seconds));
    }

    /// FE state 8's `OnUpdate`: FE state 27's round timer and `GetPostEventDisplayTime`'s whole
    /// seconds left.
    pub(super) fn update_online_results(&mut self) {
        if let Some(career) = self.career.as_mut() {
            career.update_online_round_timer();
        }
        let Some(deadline) = self.online.results_deadline else { return };
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        let seconds = i32::try_from(left.as_secs() + u64::from(left.subsec_nanos() > 0)).unwrap_or(i32::MAX);
        if let Some(career) = self.career.as_mut() {
            career.update_online_results(seconds, true);
        }
    }

    /// `GameExitPostChallenge`: the board goes, and the game host peer sends the next
    /// challenge (`UpdateNextChallenge`, the same challenge in an unranked game).
    pub(super) fn online_exit_post_challenge(&mut self) {
        self.online.results_deadline = None;
        if let Some(career) = self.career.as_mut()
            && career.online_results_open()
        {
            career.close_online_results();
        }
        if self.online_is_host()
            && let Some(attributes) = self.online.game.as_ref().map(|game| game.attributes.clone())
        {
            self.online.send(ClientMessage::ResetAttributes(attributes));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CATCH_UP_LEAD, CATCH_UP_TICKS, REFILL_LEAD, SMOOTH_LEAD, pace, ticks_for_lead};

    #[test]
    fn a_few_ready_ticks_run_one_a_tick_and_a_backlog_catches_up() {
        assert_eq!(ticks_for_lead(0), 0);
        assert_eq!(ticks_for_lead(1), 1);
        assert_eq!(ticks_for_lead(SMOOTH_LEAD), 1, "a jitter buffer's worth runs at the simulation's pace");
        assert_eq!(ticks_for_lead(SMOOTH_LEAD + 1), 2);
        assert_eq!(ticks_for_lead(CATCH_UP_LEAD), 2);
        assert_eq!(ticks_for_lead(CATCH_UP_LEAD + 1), CATCH_UP_TICKS);
    }

    #[test]
    fn a_round_out_of_facts_refills_a_buffer_then_keeps_pace() {
        assert_eq!(pace(0, false), (0, true));
        assert_eq!(pace(REFILL_LEAD - 1, true), (0, true), "refilling");
        assert_eq!(pace(REFILL_LEAD, true), (1, false));
        assert_eq!(pace(1, false), (1, false), "the buffer drains one a tick");
        // Facts three ticks at a time every three ticks, in order and up to seven ticks late:
        // after the first fill the round never stops.
        let jitter = [0, 7, 2, 0, 4, 1, 3, 0, 7, 0];
        let mut late = 0;
        let arrivals: Vec<u32> = (0..200_u32)
            .map(|run| {
                late = (run * 3 + 2 + jitter[usize::try_from(run).unwrap() % jitter.len()]).max(late);
                late
            })
            .collect();
        let (mut received, mut processed, mut refilling, mut stops, mut started) = (0, 0, false, 0, false);
        for tick in 0..590 {
            received += 3 * arrivals.iter().filter(|&&at| at == tick).count();
            let (ticks, waiting) = pace(u32::try_from(received - processed).unwrap(), refilling);
            refilling = waiting;
            processed += ticks;
            started |= ticks > 0;
            if started && ticks == 0 {
                stops += 1;
            }
        }
        assert_eq!(stops, 0);
    }

    /// The final results of a round a player left hold their lines from the first place on.
    #[test]
    fn final_results_close_up_over_a_player_who_left() {
        let row = |user_id| super::PlayerStats { user_id, ..Default::default() };
        assert_eq!(super::result_places(&[10, 20], &[row(20)]), [20]);
        assert_eq!(super::result_places(&[10, 20, 30], &[row(30), row(10)]), [10, 30]);
        assert_eq!(super::result_places(&[10, 20], &[row(10), row(20)]), [10, 20]);
    }

    /// A player who leaves during the load is not in the challenge's table, which the server
    /// packs again without them: the notes must not name whoever takes their index.
    #[test]
    fn a_player_who_left_during_the_load_names_no_one_in_the_challenge() {
        let stance = std::sync::Arc::new(std::sync::Mutex::new([true; super::MAX_PLAYERS]));
        let mut round = super::Round::loading(1, 0, vec![10, 20, 30], 0, super::FactsScope::ALL, false, stance);
        round.player_left(20);
        assert_eq!(round.notes().left, [1]);
        round.start(vec![10, 30], 0);
        assert!(round.notes().left.is_empty());
        round.player_left(30);
        assert_eq!(round.notes().left, [1]);
        // A player the challenge already disconnected needs no note when they leave the game.
        round.disconnected.push(1);
        round.leaving.clear();
        round.player_left(30);
        assert!(round.notes().left.is_empty());
    }
}
