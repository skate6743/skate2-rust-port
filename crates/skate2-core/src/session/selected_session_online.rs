//! Online play in the live session: the connection to the dedicated server (signing in, as
//! `StateOnlineSignIn` 0x8275df08 does with FESL), matchmaking (`StateOnlinePlayNow`), the game
//! a player joins (`StateOnlineGame`: lobby skate at the game's location, players joining and
//! leaving) and the remote skaters, which show the replay frames their owners send.

#[path = "online/online_challenge.rs"]
mod online_challenge;
#[path = "online/online_dmos.rs"]
mod online_dmos;
#[path = "online/lobby_hud.rs"]
mod lobby_hud;
#[path = "online/lobby_browser.rs"]
mod lobby_browser;
#[path = "online/online_hub.rs"]
mod online_hub;
#[path = "online/online_menu.rs"]
mod online_menu;
#[path = "online/remote_skater.rs"]
mod remote_skater;

use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage};
use crate::online::chat::{ChatOutcome, ChatPrompt};
use crate::online::event_log::{EventLog, EventText};
use crate::ui::online_config::OnlineConfig;
use remote_skater::RemoteSkater;
use skate2_net::client::{Client, ClientEvent, Disconnect};
use skate2_net::frames::{HEADER_WORDS, INTERVAL, MAX_RUN, NetFrame, PLAYING_INTERVAL};
use skate2_net::protocol::{
    ClientMessage, GameAttributes, Phase, PlayNow, PlayNowMode, PlayerInfo, ServerMessage, challenge, play_now, refuse, request, timer, valid_chat,
};
use std::collections::BTreeMap;
use std::sync::Arc;

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, format!("online: {e}"))
}

fn trace(message: std::fmt::Arguments<'_>) {
    if std::env::var_os("SKATE2_TRACE_ONLINE").is_some() {
        eprintln!("online: {message}");
    }
}

/// `Fesl::HubSingle::ConnInstance()->GetStatus()` as the port has it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SignIn {
    #[default]
    Offline,
    Connecting,
    SignedIn,
    Failed(Disconnect),
}

/// A matchmaking request the front end makes (`StateMessage_EnterOnlineGame`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchRequest {
    pub mode: PlayNowMode,
    pub ranked: bool,
    pub challenge_type: u8,
    pub challenge_key: Option<u64>,
    pub private: bool,
    pub create: bool,
    pub fallback_key: u64,
}

/// What the front end hears of the connection and matchmaking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnlineNotice {
    /// Sign-in failed (`ID_ONLINE_SIGNIN_FAILED_EA_SERVER`).
    SignInFailed,
    /// A quick match found nothing (`SER_PLAYNOW_NO_MATCH`).
    NoMatch,
    /// Matchmaking failed (`SER_PLAYNOW_NO_RB`, a full game).
    MatchFailed,
    /// The server dropped the game (`SER_GAME_DISCONNECTED`).
    Disconnected,
    /// The game host kicked the player (`SER_GAME_KICKED`).
    Kicked,
    /// The server is another build of the game (`refuse::BUILD`, `refuse::VERSION`): the game has
    /// to update. The original had no such notice (Xbox LIVE updated the title).
    UpdateRequired,
    /// The server does not take the nickname (`leave::KICKED_FOR_NICKNAME`): one with a word of its
    /// list. PC port addition, as the nickname is.
    NicknameNotPermitted,
}

/// A player of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnlinePlayer {
    pub user_id: u64,
    pub slot: u8,
    pub name: String,
}

/// The game the player is in.
pub(super) struct OnlineGame {
    pub(super) id: u32,
    pub(super) local_slot: u8,
    pub(super) attributes: GameAttributes,
    /// In join order; the first is the game host peer.
    pub(super) players: Vec<OnlinePlayer>,
    pub(super) phase: Phase,
    /// The members playing the challenge in progress, as the server last said: a player waiting
    /// in its pre-game lobby watches them as ghosts (PC port addition).
    pub(super) participants: Vec<u64>,
    remotes: BTreeMap<u64, RemoteSkater>,
    /// Recipes that arrived before the world could build them.
    recipes: BTreeMap<u64, Vec<u8>>,
    /// Players whose join has been announced (`OnlineEventLog::OnMessage` 0x82619e80 announces
    /// a joiner once its skater can show).
    announced: Vec<u64>,
    /// The local skater is in the game's world.
    pub(super) entered: bool,
    character_sent: bool,
    /// The server holds the local recipe and a replay frame (`ChatEnabled`): T opens the chat.
    chat_enabled: bool,
    /// The last frame's header flags, for a pose sample after a teleport.
    last_teleport: bool,
    /// The camera rotation last sent, whose side of the quaternion the next one keeps.
    last_camera_rotation: Option<[f32; 4]>,
}

impl OnlineGame {
    fn host(&self) -> Option<u64> {
        self.players.first().map(|player| player.user_id)
    }

    fn name(&self, user_id: u64) -> String {
        self.players.iter().find(|player| player.user_id == user_id).map(|player| player.name.clone()).unwrap_or_default()
    }
}

/// The session's online state.
#[derive(Default)]
pub(super) struct Online {
    client: Option<Client>,
    pub(super) sign_in: SignIn,
    pub(super) user_id: Option<u64>,
    nickname: String,
    pub(super) searching: Option<MatchRequest>,
    pub(super) game: Option<OnlineGame>,
    /// A game joined from the front end or the single player hub, entered once the fade covers.
    pub(super) start: Option<[f32; 16]>,
    pub(super) event_log: EventLog,
    notices: Vec<OnlineNotice>,
    /// `SKATE2_DIAGNOSTIC_ONLINE` made its request.
    diagnostic_requested: bool,
    /// The challenge round in progress.
    round: Option<online_challenge::Round>,
    /// `Net::ResumeGame` holds: no pause state stops the simulation.
    pub(super) network_resumed: bool,
    /// `SKATE2_DIAGNOSTIC_ONLINE_START`: completed ticks in lobby skate before the host starts
    /// the game's challenge.
    diagnostic_lobby_ticks: u32,
    /// `SKATE2_DIAGNOSTIC_ONLINE_FRAME_DUMP`: the local frames sent, written raw.
    frame_dump: Option<std::io::BufWriter<std::fs::File>>,
    /// The online screen over the Online options.
    screen: Option<online_menu::MatchScreen>,
    /// The Lobby Browser over the Online options.
    browser: Option<lobby_browser::Browser>,
    /// The front end's English strings by `StringHash`, for the lists' order.
    titles: Option<Arc<BTreeMap<u32, String>>>,
    /// The online hub and the Host Options screens over it.
    hub: online_hub::Hub,
    /// `mIsChangingLocation`: the host's `GameChange` counting down, to its type and challenge.
    changing_location: Option<(u8, u64)>,
    /// `OnlineBackEnd::mPostEventTime`: when the results board leaves.
    results_deadline: Option<std::time::Instant>,
    /// How many times FE state 8's online display has opened.
    results_instance: u32,
    /// `cLocationManager`'s global locators, for each player's spawn location.
    locations: Option<Arc<super::career::locations::LocationManager>>,
    /// The lobby skate HUD, and whether `SetupLobbyHUD` runs again.
    lobby_hud: lobby_hud::LobbyHud,
    lobby_hud_dirty: bool,
    /// What the submitted camera shows in place of the game camera.
    view: Option<online_challenge::OnlineView>,
    /// When the game last sent a replay frame or `Busy`: the server drops a game that sends
    /// neither for two seconds.
    alive_sent: Option<std::time::Instant>,
    /// The facts waiting to go out with the run of replay frames being gathered.
    outbox: Vec<ClientMessage>,
    /// The local skater's replay frames since the last run went out.
    run: Vec<NetFrame>,
    /// The game's invisible boundaries while its area is held (`InvisibleBoundaryManager`).
    pub(super) boundaries: Option<Arc<crate::challenge::invisible_boundary::InvisibleBoundaries>>,
    /// The game's challenge hulls and the static objects `Dmo::SimManager` places from them.
    dmos: online_dmos::OnlineDmos,
    /// The chat prompt while it is open.
    chat: Option<ChatPrompt>,
    /// The nickname the single player hub's Sign In asks for when `online-config.json` has none.
    nickname_prompt: Option<crate::ui::profile_name::Prompt>,
    /// `SKATE2_DIAGNOSTIC_ONLINE_CHAT` sent its line.
    diagnostic_chat_sent: bool,
    /// The pause menu's Leave Challenge answered yes (PC port addition).
    leave_challenge: bool,
}

impl Online {
    /// Whether the player is signed in (`CONNECTION_READY`).
    pub(super) fn signed_in(&self) -> bool {
        self.sign_in == SignIn::SignedIn
    }

    pub(super) fn take_notices(&mut self) -> Vec<OnlineNotice> {
        std::mem::take(&mut self.notices)
    }

    /// A message now, after the facts waiting for the run of replay frames.
    fn send(&mut self, message: ClientMessage) {
        let mut messages = std::mem::take(&mut self.outbox);
        messages.push(message);
        if let Some(client) = &self.client {
            client.send_tick(messages, Vec::new());
        }
    }

    /// The tick's facts, sent with the run of replay frames.
    fn queue(&mut self, message: ClientMessage) {
        self.outbox.push(message);
    }

    /// The facts waiting and the run of replay frames gathered, together.
    fn send_run(&mut self) {
        let messages = std::mem::take(&mut self.outbox);
        let frames = std::mem::take(&mut self.run);
        if let Some(client) = &self.client
            && (!frames.is_empty() || !messages.is_empty())
        {
            client.send_tick(messages, frames);
        }
    }
}

const GM_ONLINE_FREE_SKATE: u32 = 4;

/// A presentation frame's header as replay frames carry it, without `peerId` (words 40 and 41):
/// each console writes the sender's id there on receipt.
fn net_header(header: &[u8]) -> [u32; HEADER_WORDS] {
    let mut words = [0_u32; HEADER_WORDS];
    for (word, chunk) in words.iter_mut().zip(header.chunks_exact(4)) {
        *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    words[40..42].fill(0);
    words
}
/// How long the game goes without a replay frame before it says `Busy`.
const BUSY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
const SIM_LOCATIONS: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.loc";
/// `challenges`, its `Name` and `GlobalData`, and `challenge_global_data.Location`.
const CHALLENGES: u64 = 0x8876_E0B5_5674_0E36;
const CHALLENGE_NAME: u64 = 0x942A_B8AE_E4B4_14ED;
const GLOBAL_DATA: u64 = 0x2449_1BF0_0316_AA45;
const LOCATION: u64 = 0xF1C6_D5FA_4A23_951E;
/// The audio's ids of remote skaters (`SkaterData`'s player index, by which
/// `CSTATEMGR_Player::UpdateParams` 0x82556c60 keeps each skater's sounds in one state): their
/// game slots past the SkaterManager's eight Actors, so none shares the local skater's (always 0)
/// or an AI skater's.
const REMOTE_AUDIO_IDS: u32 = 8;
/// The most opacity a challenge's player shows with to a player waiting in its pre-game lobby
/// (PC port addition).
const LOBBY_GHOST_OPACITY: f32 = 0.4;

impl<T, C> SelectedLiveSession<T, C> {
    /// Signs in with `online-config.json`'s nickname and server, unless already signed in or
    /// signing in. The Online screen asks whenever it opens.
    pub(super) fn online_sign_in(&mut self) {
        if matches!(self.online.sign_in, SignIn::Connecting | SignIn::SignedIn) {
            return;
        }
        let config = OnlineConfig::load();
        if !config.has_nickname() {
            return;
        }
        trace(format_args!("signing in to {} as {}", config.server(), config.nickname.trim()));
        self.online.nickname = config.nickname.trim().to_owned();
        self.online.client = Some(Client::connect(config.server(), &self.online.nickname, crate::online::GAME_VERSION));
        self.online.sign_in = SignIn::Connecting;
    }

    /// The nickname was saved: signs in again under it when signed in under another, as the
    /// server knows a player by the nickname it signed in with; in a game or a search, the new one
    /// waits for the next sign-in.
    pub(super) fn online_nickname_changed(&mut self) {
        if matches!(self.online.sign_in, SignIn::Connecting | SignIn::SignedIn) {
            if self.online.game.is_some() || self.online.searching.is_some() || OnlineConfig::load().nickname.trim() == self.online.nickname {
                return;
            }
            trace(format_args!("signing in again under the new nickname"));
            self.online.client = None;
            self.online.sign_in = SignIn::Offline;
            self.online.user_id = None;
        }
        self.online_sign_in();
    }

    /// The front end's matchmaking request (`StateOnlinePlayNow::DoPlayNow` 0x827736b8 or
    /// `DoQuickMatch` 0x827737d0).
    pub(super) fn online_play_now(&mut self, request: MatchRequest) {
        if !self.online.signed_in() || self.online.searching.is_some() || self.online.game.is_some() {
            return;
        }
        trace(format_args!("play now {request:?}"));
        self.online.send(ClientMessage::PlayNow(PlayNow {
            mode: request.mode,
            ranked: request.ranked,
            challenge_type: request.challenge_type,
            challenge_key: request.challenge_key,
            private: request.private,
            create: request.create,
            fallback_key: request.fallback_key,
            game_version: crate::online::GAME_VERSION.to_owned(),
        }));
        self.online.searching = Some(request);
    }

    /// Everything the client heard since the last frame.
    pub(super) fn update_online(&mut self) -> Result<(), SelectedLiveError> {
        let events = self.online.client.as_ref().map(Client::poll).unwrap_or_default();
        for event in events {
            match event {
                ClientEvent::SignedIn { user_id } => {
                    trace(format_args!("signed in as {user_id:016x}"));
                    self.online.sign_in = SignIn::SignedIn;
                    self.online.user_id = Some(user_id);
                }
                ClientEvent::Disconnected(reason) => {
                    trace(format_args!("disconnected: {reason:?} (build {})", skate2_net::crypto::Build::current().id()));
                    let update = matches!(reason, Disconnect::Refused(refuse::BUILD | refuse::VERSION));
                    let nickname = reason == Disconnect::Closed(skate2_net::protocol::leave::KICKED_FOR_NICKNAME);
                    let had_game = self.online.game.is_some();
                    self.online.client = None;
                    self.online.searching = None;
                    self.online.sign_in = SignIn::Failed(reason);
                    self.online.user_id = None;
                    if had_game {
                        self.online_game_lost(OnlineNotice::Disconnected)?;
                    } else {
                        self.online.notices.push(if update {
                            OnlineNotice::UpdateRequired
                        } else if nickname {
                            OnlineNotice::NicknameNotPermitted
                        } else {
                            OnlineNotice::SignInFailed
                        });
                    }
                    // The server takes another version: look for it again and offer it again,
                    // even if the player said No to it.
                    if update {
                        self.update_request = Some(crate::ui::update_prompt::UpdateRequest::Check);
                        if let Some(menu) = self.main_menu.as_mut() {
                            menu.update_declined = None;
                        }
                    }
                }
                ClientEvent::Frame { user_id, frame, run_end } => {
                    let tick = self.completed_frames;
                    // Net::PauseGame holds this console's ticks: an arrival then is not paced.
                    let paced = self.online.network_resumed;
                    // A round's player sees nothing of a member waiting in the pre-game lobby: the
                    // server relays none of its frames, and one already on its way when the member
                    // left the round is dropped.
                    let waiting = self.online.round.is_some() && self.online.game.as_ref().is_some_and(|game| !game.participants.contains(&user_id));
                    if !waiting && let Some(remote) = self.online.game.as_mut().and_then(|game| game.remotes.get_mut(&user_id)) {
                        remote.receive(tick, *frame, paced, run_end);
                    }
                }
                ClientEvent::Facts { user_id, tick, stint, notes, data } => self.online_facts(user_id, tick, stint, notes, &data),
                ClientEvent::Message(message) => {
                    if let Err(e) = self.online_message(message) {
                        self.online_unplayable(&e)?;
                    }
                }
            }
        }
        if std::mem::take(&mut self.online.leave_challenge) {
            self.online_leave_challenge()?;
        }
        if self.online.game.as_ref().is_some_and(|game| game.entered) {
            self.online.event_log.update();
        }
        if self.online.game.is_some() {
            let now = std::time::Instant::now();
            if self.online.alive_sent.is_none_or(|sent| now.saturating_duration_since(sent) >= BUSY_INTERVAL) {
                self.online.send(ClientMessage::Busy);
                self.online.alive_sent = Some(now);
            }
        }
        self.update_network_pause()?;
        self.update_online_round()?;
        self.update_online_lobby_hud();
        self.update_online_results();
        self.online_diagnostic_chat();
        self.online_diagnostic()
    }

    /// A game this console cannot follow (a challenge its data lacks, from another build or a
    /// made-up message, or a round it cannot build): left as a lost connection leaves a game, so
    /// nothing the server or another player sends ends the session.
    fn online_unplayable(&mut self, reason: &dyn std::fmt::Display) -> Result<(), SelectedLiveError> {
        eprintln!("online: leaving the game: {reason}");
        self.online.send(ClientMessage::LeaveGame);
        self.online.searching = None;
        if self.online.game.is_some() {
            return self.online_game_lost(OnlineNotice::Disconnected);
        }
        self.online.notices.push(OnlineNotice::MatchFailed);
        Ok(())
    }

    /// Whether the game's challenge is one of its type's online challenges in this build's data
    /// (the list the menus offer): any other is one this console cannot load.
    fn online_challenge_known(&mut self, attributes: &GameAttributes) -> Result<bool, SelectedLiveError> {
        self.online_data()?;
        let (db, table) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let challenges = crate::online::catalog::online_challenges(db, attributes.challenge_type, &|name| table.find(name).is_some(), &|title| title.to_owned());
        Ok(challenges.iter().any(|challenge| challenge.key == attributes.challenge_key))
    }

    /// `SKATE2_DIAGNOSTIC_ONLINE_START`: the challenge asked for after that many lobby ticks,
    /// counted while every other player's skater shows, as the server starts a challenge only
    /// with the members in the game's world.
    fn online_diagnostic_start(&mut self) {
        let Some(game) = self.online.game.as_ref() else { return };
        if self.online.round.is_some() || game.players.len() < 2 || !game.remotes.values().all(RemoteSkater::visible) {
            return;
        }
        let Some(start) = std::env::var("SKATE2_DIAGNOSTIC_ONLINE_START").ok().and_then(|ticks| ticks.parse::<u32>().ok()) else { return };
        self.online.diagnostic_lobby_ticks += 1;
        if self.online.diagnostic_lobby_ticks == start {
            trace(format_args!("diagnostic start challenge"));
            self.online_start_challenge();
        }
    }

    /// `SKATE2_DIAGNOSTIC_ONLINE_CHAT`: the line, sent once the game's chat is enabled.
    fn online_diagnostic_chat(&mut self) {
        if self.online.diagnostic_chat_sent || !self.online.game.as_ref().is_some_and(|game| game.chat_enabled) {
            return;
        }
        if let Ok(line) = std::env::var("SKATE2_DIAGNOSTIC_ONLINE_CHAT") {
            self.online.diagnostic_chat_sent = true;
            trace(format_args!("diagnostic chat: {line}"));
            self.online.send(ClientMessage::Chat(line));
        }
    }

    /// T (PC port addition): the chat prompt opens in the game's world once the server enabled the
    /// player's chat, unless Show Lobby Chat is off.
    pub fn online_open_chat(&mut self) {
        if self.online.chat.is_none()
            && self.options_menu.show_lobby_chat()
            && self.online.game.as_ref().is_some_and(|game| game.entered && game.chat_enabled)
        {
            self.online.chat = Some(ChatPrompt::default());
        }
    }

    /// The open chat prompt's line and cursor.
    #[must_use]
    pub fn online_chat_prompt(&self) -> Option<ChatPrompt> {
        self.online.chat.clone()
    }

    /// Whether the chat prompt takes the keyboard.
    pub(super) fn online_chat_open(&self) -> bool {
        self.online.chat.is_some()
    }

    /// The open nickname prompt.
    #[must_use]
    pub fn online_nickname_prompt(&self) -> Option<crate::ui::profile_name::Prompt> {
        self.online.nickname_prompt.clone()
    }

    /// Whether the nickname prompt takes the keyboard.
    pub(super) fn online_nickname_prompt_open(&self) -> bool {
        self.online.nickname_prompt.is_some()
    }

    /// A key in the open nickname prompt: Enter saves the nickname, as the Online options' prompt
    /// does, and signs in under it; Escape closes it unsaved.
    pub(super) fn online_nickname_input(&mut self, input: crate::ui::extras::TextInput) {
        let Some(prompt) = self.online.nickname_prompt.as_mut() else { return };
        prompt.input(input);
        if prompt.take_cancelled() {
            self.online.nickname_prompt = None;
            return;
        }
        let Some(nickname) = prompt.take_accepted() else { return };
        let nickname = nickname.trim().to_owned();
        match (OnlineConfig { nickname, ..OnlineConfig::load() }).save() {
            Ok(()) => {
                self.online.nickname_prompt = None;
                self.online_nickname_changed();
                self.online_signing_in();
            }
            Err(e) => prompt.error = Some(format!("Could not save online-config.json: {e}")),
        }
    }

    /// A key in the open chat prompt; Enter sends its line.
    pub(super) fn online_chat_input(&mut self, input: crate::ui::extras::TextInput) {
        let Some(prompt) = self.online.chat.as_mut() else { return };
        match prompt.input(input) {
            ChatOutcome::Open => {}
            ChatOutcome::Closed => self.online.chat = None,
            ChatOutcome::Send(line) => {
                self.online.chat = None;
                self.online.send(ClientMessage::Chat(line));
            }
        }
    }

    /// A member's chat line from the server, into the event log as `[name]: line`; with Show
    /// Lobby Chat off, nowhere.
    fn online_chat(&mut self, user_id: u64, text: &str) {
        if !self.options_menu.show_lobby_chat() {
            return;
        }
        let Some(game) = self.online.game.as_ref() else { return };
        let Some(player) = game.players.iter().find(|player| player.user_id == user_id) else { return };
        if !valid_chat(text) {
            return;
        }
        for line in crate::online::chat::lines(&player.name, text) {
            self.online.event_log.add(EventText::chat(line));
        }
    }

    /// `StateOnlineGame::LobbySkate` (0x82777820): its HUD opens when lobby skate starts in the
    /// game's world and closes when a challenge loads, the skater travels or the game ends;
    /// `SetupLobbyHUD` runs on entering, as players join and leave, and as the game's challenge
    /// starts and ends around a player who joined during it.
    fn update_online_lobby_hud(&mut self) {
        let lobby = self.online.round.is_none() && self.map_travel.is_none() && self.online.game.as_ref().is_some_and(|game| game.entered);
        if lobby != self.online.lobby_hud.is_open() {
            if lobby {
                self.online.lobby_hud.open();
                self.online.lobby_hud_dirty = true;
            } else {
                self.online.lobby_hud.close();
            }
        }
        if lobby
            && std::mem::take(&mut self.online.lobby_hud_dirty)
            && let Some(game) = self.online.game.as_ref()
        {
            let rows: Vec<lobby_hud::LobbyRow> = game
                .players
                .iter()
                .map(|player| lobby_hud::LobbyRow { name: player.name.clone(), local: Some(player.user_id) == self.online.user_id })
                .collect();
            let in_progress = game.phase != Phase::LobbySkate;
            let freeskate = game.attributes.challenge_type == challenge::ONLINE_FREE_SKATE;
            let variant = lobby_hud::variant(in_progress, freeskate, game.attributes.ranked, rows.len());
            self.online.lobby_hud.setup(&rows, variant);
        }
        let row = self.inner.bound_input.as_ref().and_then(|input| u32::try_from(input.player_index()).ok()).unwrap_or(0);
        let stance_regular = self
            .inner
            .player
            .as_ref()
            .and_then(|player| player.selected.owners.services.settings.try_stance_byte(row).ok())
            .is_none_or(|stance| stance != 0);
        self.online.lobby_hud.update(crate::challenge::system::FRONT_END_UPDATE_DT, stance_regular);
    }

    /// `Net::ResumeGame` (0x82791ca8) while the local skater plays in the game's world, and
    /// `Net::PauseGame` (0x82791a88) while it loads, travels or shows the results, as the
    /// game's states call them; and the network game the Simulation's rate reads.
    fn update_network_pause(&mut self) -> Result<(), SelectedLiveError> {
        let loading = self.online.round.as_ref().is_some_and(|round| !round.running());
        let resumed = self.online.game.as_ref().is_some_and(|game| game.entered) && self.map_travel.is_none() && !loading;
        if resumed != self.online.network_resumed
            && let Some(input) = self.inner.bound_input.as_ref()
        {
            input.try_set_network_resumed(resumed).map_err(error)?;
            self.online.network_resumed = resumed;
        }
        // StateOnlineGame::SetupInitalGameInfo sets sGameInfo.isNetworkGame (0x8275dba8) and
        // GameModule::BackToSP clears it (0x8273dac8): no slow motion in an online game, and the
        // network game's solver passes.
        let online = self.online.game.as_ref().is_some_and(|game| game.entered);
        if let Some(owner) = self.inner.world.as_ref() {
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            world.sim_rate.lock().map_err(|_| error("sim rate lock poisoned"))?.set_online_game(online);
        }
        Ok(())
    }

    /// `SKATE2_DIAGNOSTIC_ONLINE=create:<type>[:<challenge>]` or `join:<type>[:<challenge>]`: signs
    /// in from the main menu and creates or joins a custom match of the network challenge type at
    /// the named challenge, else its first. `list` prints every type's online challenges.
    fn online_diagnostic(&mut self) -> Result<(), SelectedLiveError> {
        let Ok(spec) = std::env::var("SKATE2_DIAGNOSTIC_ONLINE") else { return Ok(()) };
        if self.online.diagnostic_requested || self.online.game.is_some() || self.online.searching.is_some() {
            return Ok(());
        }
        if !self.main_menu.as_ref().is_some_and(|menu| menu.settled && !menu.leaving) {
            return Ok(());
        }
        if spec == "list" {
            self.online.diagnostic_requested = true;
            for net_type in 0..challenge::COUNT {
                for listed in self.online_challenge_list(net_type)? {
                    let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
                    let name = db.attribute(CHALLENGES, listed.key, CHALLENGE_NAME).and_then(|name| name.text(0)).unwrap_or_default();
                    eprintln!("online challenge type {net_type} {name} at {}: {}", listed.location, listed.title);
                }
            }
            return Ok(());
        }
        match self.online.sign_in {
            SignIn::Offline => {
                self.online_sign_in();
                return Ok(());
            }
            SignIn::SignedIn => {}
            SignIn::Connecting | SignIn::Failed(_) => return Ok(()),
        }
        let mut parts = spec.split(':');
        let create = parts.next() == Some("create");
        let challenge_type = parts.next().and_then(|text| text.parse().ok()).unwrap_or(challenge::SKATE);
        self.online_data()?;
        let (db, table) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let challenges = crate::online::catalog::online_challenges(db, challenge_type, &|name| table.find(name).is_some(), &|title| title.to_owned());
        let named = parts.next().map(crate::challenge::keys::hash64);
        let key = challenges
            .iter()
            .find(|challenge| named.is_none_or(|key| challenge.key == key))
            .map(|challenge| challenge.key)
            .ok_or_else(|| error(format!("no online challenge of type {challenge_type} {named:x?}")))?;
        trace(format_args!("diagnostic {} of type {challenge_type} at {key:016x}", if create { "create" } else { "join" }));
        self.online.diagnostic_requested = true;
        self.online_play_now(MatchRequest {
            mode: if challenge_type == challenge::ONLINE_FREE_SKATE { PlayNowMode::Freeskate } else { PlayNowMode::Custom },
            ranked: false,
            challenge_type,
            challenge_key: create.then_some(key),
            private: false,
            create,
            fallback_key: key,
        });
        Ok(())
    }

    fn online_message(&mut self, message: ServerMessage) -> Result<(), SelectedLiveError> {
        match message {
            ServerMessage::PlayNowResult { result } => {
                trace(format_args!("play now result {result}"));
                if result != play_now::JOINED {
                    if self.online.searching.take().is_some() {
                        self.online.notices.push(if result == play_now::NO_MATCH { OnlineNotice::NoMatch } else { OnlineNotice::MatchFailed });
                    } else {
                        self.lobby_join_failed(result);
                    }
                }
            }
            ServerMessage::Lobbies(lobbies) => self.online_lobbies(&lobbies)?,
            ServerMessage::GameJoined { game, slot, attributes, players, phase, participants } => self.online_game_joined(game, slot, attributes, players, phase, participants)?,
            ServerMessage::GameAttributes(attributes) => {
                if !self.online_challenge_known(&attributes)? {
                    return self.online_unplayable(&format_args!("the game went to a challenge this build lacks ({:016x})", attributes.challenge_key));
                }
                // A game joined but not yet entered (the front end's fade still covers the join)
                // goes where its attributes now say: its location changed in between.
                if self.online.start.is_some()
                    && let Some(slot) = self.online.game.as_ref().map(|game| game.local_slot)
                {
                    self.online.start = Some(self.online_location(&attributes, slot)?);
                }
                if let Some(game) = self.online.game.as_mut() {
                    // Another challenge (the host's next one) is another location: the lobby reset
                    // that follows takes a player in lobby skate there too, as it does the
                    // challenge's players.
                    if game.attributes.challenge_key != attributes.challenge_key {
                        self.online.changing_location.get_or_insert((attributes.challenge_type, attributes.challenge_key));
                    }
                    game.attributes = attributes;
                }
            }
            ServerMessage::GameReset { kind, seed, session, participants } => self.online_game_reset(kind, seed, session, &participants)?,
            ServerMessage::Vote { user_id, score } => self.online_vote(user_id, score),
            ServerMessage::SyncPoint { round } => self.online_sync_point_released(round)?,
            ServerMessage::AllPlayersComplete => self.online_all_players_complete(),
            ServerMessage::FinalResults(rows) => self.online_final_results(&rows)?,
            ServerMessage::GameTimer { kind: timer::DISPLAY_RESULTS, value } => self.online_results_timer(value),
            ServerMessage::ExitPostChallenge => self.online_exit_post_challenge(),
            ServerMessage::PrivacyChanged { private } => {
                if let Some(game) = self.online.game.as_mut() {
                    game.attributes.private = private;
                    let key = if private { "ID_ONLINE_EVENT_PRIVATE_SESSION" } else { "ID_ONLINE_EVENT_PUBLIC_SESSION" };
                    let host = game.host().map(|host| game.name(host)).unwrap_or_default();
                    self.online.event_log.add(EventText::named(key, &host));
                }
            }
            ServerMessage::PlayerJoin(player) => self.online_player_join(&player),
            ServerMessage::PlayerLeave { user_id, reason } => self.online_player_leave(user_id, reason),
            ServerMessage::Recipe { user_id, data } => self.online_recipe(user_id, data)?,
            ServerMessage::Kicked { .. } => self.online_game_lost(OnlineNotice::Kicked)?,
            ServerMessage::GameChange { challenge_type, challenge_key, .. } => self.online_game_change(challenge_type, challenge_key)?,
            ServerMessage::ChatEnabled => {
                if let Some(game) = self.online.game.as_mut() {
                    game.chat_enabled = true;
                }
            }
            ServerMessage::Chat { user_id, text } => self.online_chat(user_id, &text),
            ServerMessage::ChallengeLeave { user_id } => self.online_challenge_leave(user_id),
            message => trace(format_args!("unhandled {message:?}")),
        }
        Ok(())
    }

    /// `OnEvent(OE_GameJoined)` and `SetupGame` 0x82778530: the game's players, then the game's
    /// location once the front end's fade covers.
    fn online_game_joined(&mut self, id: u32, slot: u8, attributes: GameAttributes, players: Vec<PlayerInfo>, phase: Phase, participants: Vec<u64>) -> Result<(), SelectedLiveError> {
        trace(format_args!("joined game {id} in slot {slot}: {attributes:?}, {} players, {phase:?}", players.len()));
        if !self.online_challenge_known(&attributes)? {
            return self.online_unplayable(&format_args!("game {id} is at a challenge this build lacks ({:016x})", attributes.challenge_key));
        }
        self.online.searching = None;
        self.online.screen = None;
        self.online.browser = None;
        let local = self.online.user_id;
        let remotes = players
            .iter()
            .filter(|player| Some(player.user_id) != local)
            .map(|player| (player.user_id, RemoteSkater::new(player.user_id, player.slot)))
            .collect();
        let start = self.online_location(&attributes, slot)?;
        self.online.game = Some(OnlineGame {
            id,
            local_slot: slot,
            attributes,
            players: players.into_iter().map(|player| OnlinePlayer { user_id: player.user_id, slot: player.slot, name: player.name }).collect(),
            phase,
            participants,
            remotes,
            recipes: BTreeMap::new(),
            announced: Vec::new(),
            entered: false,
            character_sent: false,
            chat_enabled: false,
            last_teleport: false,
            last_camera_rotation: None,
        });
        self.online.chat = None;
        self.online.event_log.clear();
        if let Some(menu) = self.main_menu.as_mut() {
            menu.leaving = true;
        } else {
            // From the single player hub: the game in play unloads under the fade
            // (`online_enter_from_play`).
            self.close_online_hub()?;
        }
        self.online.start = Some(start);
        self.frontend_fade.cover();
        Ok(())
    }

    /// Where a player of the game starts (`Net::GetStreamerPosition` 0x82784318 for
    /// `CalcRealStreamerPosition`'s player index): the challenge's `Location`'s spawn location of
    /// that index (`cLocationManager::GetSpawnLocation`, a slot past the sixth at the sixth's),
    /// or the location itself (`challenge_global_data.Location` in `BAM_Sim.loc`) where the
    /// global locators lack it.
    fn online_location(&mut self, attributes: &GameAttributes, player: u8) -> Result<[f32; 16], SelectedLiveError> {
        let (location, fallback) = self.online_challenge_location(attributes)?;
        if self.online.locations.is_none() {
            let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
            self.online.locations = Some(Arc::new(super::career::locations::LocationManager::with_global_locations(&mounts).map_err(error)?));
        }
        let hash = crate::challenge::keys::hash_value32(&location);
        let spawn = self.online.locations.as_ref().and_then(|locations| locations.online_spawn_location(hash, i32::from(player)));
        Ok(spawn.unwrap_or(fallback))
    }

    /// The challenge's `challenge_global_data.Location` and its transform in `BAM_Sim.loc`
    /// (`cLocationManager::GetLocationPosition`).
    fn online_challenge_location(&mut self, attributes: &GameAttributes) -> Result<(String, [f32; 16]), SelectedLiveError> {
        self.online_data()?;
        let (db, table) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let location = db
            .attribute(CHALLENGES, attributes.challenge_key, GLOBAL_DATA)
            .and_then(|global| global.ref_spec(0))
            .and_then(|(class, key)| db.attribute(class, key, LOCATION))
            .and_then(|location| location.text(0))
            .ok_or_else(|| error(format!("challenge {:016x} has no location", attributes.challenge_key)))?
            .to_owned();
        let transform = table.find(&location).ok_or_else(|| error(format!("location {location} absent")))?.transform;
        Ok((location, transform))
    }

    /// `GameModule::GameLoad`'s network branch for the game's challenge, behind the travel's
    /// fade: the challenge's stream XML holds the simulation focus on the area around its
    /// location (`Net::GetStreamerPosition` for player -1), so the travel waits for the area,
    /// and its invisible boundaries load (`LoadBoundary` 0x82db0e78), both by the challenge's
    /// `Name`.
    pub(super) fn online_load_area(&mut self) -> Result<(), SelectedLiveError> {
        let Some(attributes) = self.online.game.as_ref().map(|game| game.attributes.clone()) else { return Ok(()) };
        let (location, transform) = self.online_challenge_location(&attributes)?;
        let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let name = db
            .attribute(CHALLENGES, attributes.challenge_key, CHALLENGE_NAME)
            .and_then(|name| name.text(0))
            .ok_or_else(|| error(format!("challenge {:016x} has no name", attributes.challenge_key)))?
            .to_owned();
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
        let boundaries = crate::challenge::invisible_boundary::InvisibleBoundaries::load(&mounts, &name);
        trace(format_args!("online area {name} at {location}{}", if boundaries.is_empty() { ", no boundary" } else { "" }));
        if let Some(terrain) = self.inner.terrain.as_mut() {
            terrain.try_set_online_area(Some((&name, [transform[12], transform[13], transform[14]]))).map_err(|e| error(format!("{e:?}")))?;
        }
        self.online.dmos.hold_area(attributes.challenge_key, name);
        self.set_online_boundaries(Some(Arc::new(boundaries)))
    }

    /// `BackToSP` 0x8273da50 for the online area: the simulation focus follows the skater again,
    /// the challenge's hulls go and the boundaries go (`InvisibleBoundaryManager::UnloadAll`).
    fn online_clear_area(&mut self) -> Result<(), SelectedLiveError> {
        if let Some(terrain) = self.inner.terrain.as_mut() {
            terrain.try_set_online_area(None).map_err(|e| error(format!("{e:?}")))?;
        }
        self.release_online_dmos()?;
        self.set_online_boundaries(None)
    }

    /// The boundaries `PlayerUI` tests, and the breadcrumbs through `sGameInfo`'s mirror.
    fn set_online_boundaries(&mut self, boundaries: Option<Arc<crate::challenge::invisible_boundary::InvisibleBoundaries>>) -> Result<(), SelectedLiveError> {
        if let Some(owner) = self.inner.world.as_ref() {
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            if let Some(services) = world.physical_world_services() {
                services.game_info().try_set_online_boundaries(boundaries.clone()).map_err(|e| error(format!("{e:?}")))?;
            }
        }
        self.online.boundaries = boundaries;
        Ok(())
    }

    /// The online game's boundaries while its skater skates them: in the game and not travelling,
    /// whose own teleport takes the skater from wherever it was to the spawn (the original's
    /// loading screen holds the simulation through it).
    pub(super) fn online_held_boundaries(&self) -> Option<Arc<crate::challenge::invisible_boundary::InvisibleBoundaries>> {
        if self.map_travel.is_some() || !self.online.game.as_ref().is_some_and(|game| game.entered) {
            return None;
        }
        self.online.boundaries.clone()
    }

    /// `InvisibleBoundaryManager::Update`'s amount for the local skater this frame, at its trigger
    /// tester's base position, while an online game's boundaries are held.
    pub(super) fn online_boundary_amount(&self) -> Result<Option<f32>, SelectedLiveError> {
        let Some(boundaries) = self.online_held_boundaries() else { return Ok(None) };
        let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_deref()) else { return Ok(None) };
        let position = super::local_skater_base_position(self.inner.world.as_ref().map(|world| &world.world), actor)?;
        Ok(boundaries.warning_amount(position))
    }

    /// `skater.vlt` and `BAM_Sim.loc`, shared with free skate's screens.
    pub(super) fn online_data(&mut self) -> Result<(), SelectedLiveError> {
        if self.freeskate.data.is_none() {
            let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
            let db = Arc::new(crate::challenge::attrib_db::AttribDb::load(&mounts, &crate::challenge::attrib_db::CAREER_VAULTS).map_err(error)?);
            let table = skate2_formats::location::LocationTable::parse(&mounts.read_path(SIM_LOCATIONS).map_err(error)?).map_err(error)?;
            self.freeskate.data = Some((db, table));
        }
        Ok(())
    }

    /// The local skater enters the game: `StartGameModule` in `GM_OnlineFreeSkate` with the
    /// profile's created skater, travelling to the game's location behind the fade.
    /// A game joined from the single player hub, once the fade covers: the game in play unloads
    /// (`StateGameUnload`, as for the main menu) and the online game starts as from the front end.
    /// Its `GameModule::GameLoad` 0x8273c8f0 runs `AISkaterManager::LoadGame` for the network
    /// game, in which no ambient skater updates, as the front end's leave does on that path.
    pub(super) fn online_enter_from_play(&mut self) -> Result<(), SelectedLiveError> {
        let Some(start) = self.online.start.take() else { return Ok(()) };
        trace(format_args!("leaving the game in play for the online game"));
        self.unload_game()?;
        if let Some(ai) = self.inner.ai.as_ref() {
            ai.manager().load_game(true);
        }
        self.start_online_game(start)
    }

    pub(super) fn start_online_game(&mut self, start: [f32; 16]) -> Result<(), SelectedLiveError> {
        self.set_game_mode(GM_ONLINE_FREE_SKATE, None)?;
        self.start_single_player_players()?;
        self.drop_career()?;
        self.online.dmos.begin_game();
        self.freeroam_replay.reset();
        self.freeroam_replay_frame = 0;
        self.replay_visual_focuses = None;
        self.replay_frames.clear();
        self.replay_clock = None;
        self.replay_restorer = crate::presentation_restore::PresentationRestorer::new();
        self.freeskate.hulls = Some(self.online_dynamic_hulls()?);
        self.pause_menu.set_hub(crate::ui::pause_menu::StartHub::OnlineFreeSkate);
        self.load_created_skater()?;
        self.online_load_area()?;
        self.map_travel = Some(start);
        self.map_travel_presentation_ready = false;
        self.map_travel_dispatched = false;
        self.map_travel_on_board = true;
        if let Some(game) = self.online.game.as_mut() {
            game.entered = true;
        }
        let recipes: Vec<(u64, Vec<u8>)> = self.online.game.as_mut().map(|game| std::mem::take(&mut game.recipes).into_iter().collect()).unwrap_or_default();
        for (user_id, data) in recipes {
            self.online_recipe(user_id, data)?;
        }
        Ok(())
    }

    /// `StateOnlineGame::OnMessage(MT_GameChange)` 0x8276b060: the host's change of challenge
    /// counts down; `OnlineEventLog::AddLocationChangeMessage` 0x82639e18 tells of it with the
    /// challenge's type and name (`ID_ONLINE_EVENT_CANNOT_CHANGE_LOCATION` for an unknown one).
    fn online_game_change(&mut self, challenge_type: u8, challenge_key: u64) -> Result<(), SelectedLiveError> {
        trace(format_args!("game changes to {challenge_key:016x} (type {challenge_type})"));
        self.online.changing_location = Some((challenge_type, challenge_key));
        let title = self.online_challenge_list(challenge_type)?.into_iter().find(|challenge| challenge.key == challenge_key).map(|challenge| challenge.title);
        self.online.event_log.add(EventText::plain("ID_ONLINE_EVENT_CHANGED_SETTINGS"));
        self.online.event_log.add(match title {
            Some(title) => EventText::localized("ID_ONLINE_EVENT_LOCATION_CHANGE", vec![crate::online::catalog::challenge_title(challenge_type).to_owned(), title]),
            None => EventText::plain("ID_ONLINE_EVENT_CANNOT_CHANGE_LOCATION"),
        });
        Ok(())
    }

    /// `OE_PlayerJoin`: a remote skater waits for the joiner's recipe. One who joins during the
    /// local player's round waits in the pre-game lobby, which the round's players are told of (PC
    /// port addition).
    fn online_player_join(&mut self, player: &PlayerInfo) {
        trace(format_args!("{} joined in slot {}", player.name, player.slot));
        let apart = self.online_apart_from_round(player.user_id);
        let Some(game) = self.online.game.as_mut() else { return };
        game.players.retain(|known| known.user_id != player.user_id);
        game.players.push(OnlinePlayer { user_id: player.user_id, slot: player.slot, name: player.name.clone() });
        game.remotes.insert(player.user_id, RemoteSkater::new(player.user_id, player.slot));
        game.announced.retain(|&announced| announced != player.user_id);
        self.online.lobby_hud_dirty = true;
        if apart && game.entered {
            self.online.event_log.add(EventText::literal(format!("{} has joined pre-game lobby.", player.name)));
        }
    }

    /// `HandlePlayerLeave` 0x8276aa70 and `OnlineEventLog::OnEvent(OE_PlayerLeave)`: the skater
    /// goes, the log names who left, and the host may change. The original takes no join while a
    /// challenge runs, so a round's players never hear of a member waiting in the pre-game lobby:
    /// that member's leave is not logged to them.
    fn online_player_leave(&mut self, user_id: u64, reason: u8) {
        let apart = self.online_apart_from_round(user_id);
        self.online_round_player_left(user_id);
        let Some(game) = self.online.game.as_mut() else { return };
        let name = game.name(user_id);
        trace(format_args!("{name} left (reason {reason})"));
        let host_before = game.host();
        game.players.retain(|player| player.user_id != user_id);
        game.participants.retain(|&participant| participant != user_id);
        game.remotes.remove(&user_id);
        game.recipes.remove(&user_id);
        game.announced.retain(|&announced| announced != user_id);
        self.online.lobby_hud_dirty = true;
        let host_after = game.host();
        let host_name = host_after.map(|host| game.name(host));
        if game.entered {
            if !apart {
                self.online.event_log.add(EventText::named("ID_ONLINE_EVENT_LEAVE", &name));
            }
            if host_before != host_after
                && let Some(host) = host_name
            {
                self.online.event_log.add(EventText::named("ID_ONLINE_EVENT_HOST_CHANGE", &host));
            }
        }
    }

    /// `StateOnlineRecipe::LoadingRemoteRecipe` 0x8276fb40: a player's look, built once the
    /// local skater is in the game's world.
    fn online_recipe(&mut self, user_id: u64, data: Vec<u8>) -> Result<(), SelectedLiveError> {
        let Some(game) = self.online.game.as_mut() else { return Ok(()) };
        if !game.entered {
            game.recipes.insert(user_id, data);
            return Ok(());
        }
        let Some(remote) = game.remotes.get_mut(&user_id) else { return Ok(()) };
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?;
        trace(format_args!("{user_id:016x}'s character building ({} bytes)", data.len()));
        remote.build_character(data, Arc::clone(&assets.mounts), Arc::clone(&assets.vault), assets.cvars.clone()).map_err(error)
    }

    /// The game ended under the player (lost connection, kicked): back to the main menu with
    /// the error.
    fn online_game_lost(&mut self, notice: OnlineNotice) -> Result<(), SelectedLiveError> {
        let entered = self.online.game.as_ref().is_some_and(|game| game.entered);
        // Lost while the fade still covers the join: the main menu, or the game in play, stays,
        // as the game it was leaving for is gone.
        if !entered && self.online.start.is_some() {
            if let Some(menu) = self.main_menu.as_mut() {
                menu.leaving = false;
            }
            self.frontend_fade.reveal();
        }
        self.close_online_hub()?;
        self.online.changing_location = None;
        if self.online.round.take().is_some()
            && let Some(input) = self.inner.bound_input.as_ref()
        {
            input.try_set_online_pause(false).map_err(error)?;
        }
        self.online.game = None;
        self.online.chat = None;
        self.online.start = None;
        self.online.event_log.clear();
        self.online.notices.push(notice);
        self.online_clear_area()?;
        if entered && !self.main_menu_pending {
            self.request_main_menu()?;
        }
        Ok(())
    }

    /// `IE_LeaveGame` as the game unloads.
    pub(super) fn online_leave_game(&mut self) {
        if self.online.round.take().is_some()
            && let Some(input) = self.inner.bound_input.as_ref()
        {
            let _ = input.try_set_online_pause(false);
        }
        self.online.changing_location = None;
        self.online.hub = online_hub::Hub::default();
        self.online.chat = None;
        if self.online.game.take().is_some() {
            self.online.send(ClientMessage::LeaveGame);
        }
        if let Err(e) = self.online_clear_area() {
            trace(format_args!("online area not cleared: {e:?}"));
        }
        if self.online.network_resumed
            && let Some(input) = self.inner.bound_input.as_ref()
        {
            let _ = input.try_set_network_resumed(false);
            self.online.network_resumed = false;
        }
        self.online.start = None;
        self.online.event_log.clear();
    }

    /// The game host peer's `GameRequest`.
    pub(super) fn online_game_request(&mut self, kind: u8, value: u64) {
        let is_host = self.online.game.as_ref().and_then(OnlineGame::host) == self.online.user_id;
        if is_host {
            self.online.send(ClientMessage::GameRequest { request: kind, value });
        }
    }

    /// Whether the game in progress is a challenge game (`Start Challenge` is offered).
    pub(super) fn online_is_challenge_game(&self) -> bool {
        self.online.game.as_ref().is_some_and(|game| game.attributes.challenge_type != challenge::ONLINE_FREE_SKATE)
    }

    /// A completed tick in an online game: the local skater's replay frame goes out, and each
    /// remote skater moves to the frame it shows now.
    pub(super) fn online_tick(&mut self) -> Result<(), SelectedLiveError> {
        let tick = self.completed_frames;
        let result = self.online_tick_frame();
        // The tick's facts wait for the run being gathered; a tick that made no frame ends it.
        if self.online.run.last().is_none_or(|frame| frame.tick != tick) {
            self.online.send_run();
        }
        result
    }

    fn online_tick_frame(&mut self) -> Result<(), SelectedLiveError> {
        // Online, a skater the challenge pins still gestures (the retail game; the debug build's
        // GenerateActionGraphIntents 0x825fda28 skips the whole fill while pinned).
        let online = self.online.game.as_ref().is_some_and(|game| game.entered);
        if let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()) {
            actor.try_set_pinned_gestures(online).map_err(|e| error(format!("{e:?}")))?;
        }
        let Some(game) = self.online.game.as_ref() else { return Ok(()) };
        if !game.entered || self.map_travel.is_some() {
            return Ok(());
        }
        self.online_diagnostic_start();
        if self.online.game.is_none() {
            return Ok(());
        }
        let tick = self.completed_frames;
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?;
        let vault = Arc::clone(&assets.vault);
        let cvars = assets.cvars.clone();
        let local = match (self.inner.presentation_runtime.as_mut(), self.inner.player.as_ref().and_then(|player| player.selected.owners.presentation.as_ref())) {
            (Some(runtime), Some(presentation)) => runtime.submitted_frame(presentation, &vault).map_err(error)?,
            _ => None,
        };
        // The console's own skater's record, also while the audio takes a followed one as local.
        let audio = self.audio_own_record();
        let projection = self.inner.world.as_ref().and_then(|world| world.local.try_camera_producer_projection().ok());
        // A run ends in a sample: every PLAYING_INTERVAL ticks while the local skater plays the
        // round, every INTERVAL ticks otherwise, and at a teleport and the tick after it. While
        // the other players watch the skater, every tick of its runs is a sample.
        let round = self.online.round.as_ref();
        let plays = round.is_some_and(online_challenge::Round::local_plays);
        let watched = plays && round.is_some_and(online_challenge::Round::local_followed);
        let interval = if plays { PLAYING_INTERVAL } else { INTERVAL };
        let game = self.online.game.as_mut().expect("checked");
        let camera = projection.map(|projection| remote_skater::camera_values(&projection.camera_matrix, projection.chosen_fov_y_degrees, game.last_camera_rotation));
        if let Some(values) = camera {
            game.last_camera_rotation = Some([values[3], values[4], values[5], values[6]]);
        }
        if let Some(frame) = local {
            if !game.character_sent {
                match remote_skater::encode_character(&frame) {
                    Ok(data) => {
                        trace(format_args!("sending our character ({} bytes)", data.len()));
                        if let Some(client) = &self.online.client {
                            client.send(ClientMessage::Recipe(data));
                        }
                        game.character_sent = true;
                    }
                    Err(e) => trace(format_args!("our character cannot be sent: {e}")),
                }
            }
            let header = net_header(&frame.header);
            let teleport = header[48] & 0x1000 != 0;
            let ends = tick % interval == 0 || teleport || game.last_teleport;
            let sample = watched || ends;
            game.last_teleport = teleport;
            let root = [header[36], header[37], header[38]].map(f32::from_bits);
            let (audio_floats, audio_words) = audio.map(|record| crate::online::audio_record::flatten(&crate::online::audio_record::relative_to_root(record, root))).unwrap_or_default();
            let pose = frame
                .pose
                .iter()
                .map(|sqt| {
                    let mut lanes = [0.0; 12];
                    lanes[0..4].copy_from_slice(&sqt.scale);
                    lanes[4..8].copy_from_slice(&sqt.rotation);
                    lanes[8..12].copy_from_slice(&sqt.translation);
                    lanes
                })
                .collect();
            let mut net = NetFrame { tick, header, pose: Some(pose), twist: frame.twist_ratios, camera: camera.map(|values| values.to_vec()).unwrap_or_default(), audio_floats, audio_words };
            if let Some(path) = std::env::var_os("SKATE2_DIAGNOSTIC_ONLINE_FRAME_DUMP")
                && self.online.frame_dump.is_none()
            {
                self.online.frame_dump = std::fs::File::create(path).ok().map(std::io::BufWriter::new);
            }
            // The dump keeps every tick's pose, so any run length can be measured on it.
            if let Some(dump) = self.online.frame_dump.as_mut() {
                use std::io::Write as _;
                let record = skate2_net::frames::dump_record(&net);
                let _ = dump.write_all(&u32::try_from(record.len()).unwrap_or(0).to_le_bytes()).and_then(|()| dump.write_all(&record));
            }
            if !sample {
                net.pose = None;
            }
            // A run's later samples carry its first tick's audio floats: no restore blends them,
            // so they keep the run's rate and cost next to nothing.
            if watched && let Some(first) = self.online.run.first() {
                net.audio_floats.clone_from(&first.audio_floats);
            }
            self.online.run.push(net);
            if ends || self.online.run.len() == MAX_RUN {
                if let Some(client) = &self.online.client {
                    client.send_tick(std::mem::take(&mut self.online.outbox), std::mem::take(&mut self.online.run));
                }
                self.online.alive_sent = Some(std::time::Instant::now());
            }
        }
        let camera_position = camera.map_or([0.0; 3], |values| [values[0], values[1], values[2]]);
        for remote in game.remotes.values_mut() {
            match remote.finish_character() {
                Some(Ok(took)) => trace(format_args!("{:016x}'s character built in {:.1} ms", remote.user_id, took.as_secs_f64() * 1000.0)),
                Some(Err(e)) => trace(format_args!("{:016x}'s character failed: {e}", remote.user_id)),
                None => {}
            }
            let shown = remote.visible();
            if let Err(e) = remote.advance(tick, camera_position, &vault, &cvars) {
                trace(format_args!("{:016x}: {e}", remote.user_id));
            }
            if tick % 600 == 0 {
                let (held, shown, delay, pace, teleports) = remote.take_held();
                if shown > 0 {
                    trace(format_args!(
                        "{:016x}: {held} of {shown} ticks at or past the newest sample, delay {delay:.1}, pace {pace:.3}, {teleports} teleport frames",
                        remote.user_id
                    ));
                }
            }
            if remote.visible() != shown {
                trace(format_args!("{:016x} {}", remote.user_id, if shown { "no longer shows" } else { "shows" }));
            }
            if remote.visible() && !game.announced.contains(&remote.user_id) {
                game.announced.push(remote.user_id);
                let name = game.players.iter().find(|player| player.user_id == remote.user_id).map(|player| player.name.clone()).unwrap_or_default();
                self.online.event_log.add(EventText::named("ID_ONLINE_EVENT_JOIN_SKATER", &name));
            }
        }
        Ok(())
    }

    /// The remote skaters' frames this tick, for the presentation's submission.
    pub(super) fn online_submitted_frames(&self) -> Vec<crate::presentation_restore::skater::SkaterFrame> {
        let Some(game) = self.online.game.as_ref() else { return Vec::new() };
        game.remotes
            .values()
            .filter_map(RemoteSkater::shown)
            .map(|shown| crate::presentation_restore::skater::SkaterFrame {
                rig: Arc::clone(&shown.frame.rig),
                palette: Arc::clone(&shown.frame.palette),
                header: shown.frame.header,
                pose: shown.frame.pose.clone(),
                twist_ratios: shown.frame.twist_ratios,
                materials: shown.frame.materials.clone(),
            })
            .collect()
    }

    /// The remote skaters for the tick's visual snapshot, each named by the game's player whose
    /// skater it is (`SkaterPresEntity`'s constructor 0x827e5740 takes a non-empty name). Its
    /// index is the player's row on the leaderboard HUD: the round's table in a challenge, the
    /// game's players on the lobby board. A player waiting in the pre-game lobby watches the
    /// challenge's players as ghosts (PC port addition), drawn as the ghost technique draws a
    /// skater the camera comes close to.
    pub(super) fn online_visuals(&self) -> Vec<crate::SelectedSkaterVisual<'_>> {
        let Some(game) = self.online.game.as_ref() else { return Vec::new() };
        let round = self.online.round.as_ref();
        game.remotes
            .values()
            .filter_map(|remote| {
                let mut visual = remote.visual()?;
                visual.name_tag = game.players.iter().position(|player| player.user_id == remote.user_id).and_then(|row| {
                    let name = &game.players[row].name;
                    let index = round.and_then(|round| round.player_index(remote.user_id)).unwrap_or(row);
                    (!name.is_empty()).then(|| crate::SkaterNameTag { name: name.clone(), index: u8::try_from(index).unwrap_or(u8::MAX) })
                });
                if round.is_none() && game.participants.contains(&remote.user_id) {
                    visual.opacity = visual.opacity.min(LOBBY_GHOST_OPACITY);
                    visual.ghosted = true;
                    visual.lobby_ghost = true;
                }
                Some(visual)
            })
            .collect()
    }

    /// The remote skaters for `AudioPresentation::DoSimSubmit`, each with the audio record of its
    /// owner's replay frame. The one the console follows is the local skater, as
    /// `Online.SwitchCamera`'s `SkaterManager::SetLocalIndex` (0x82600380) makes it for
    /// `GetLocalSimIndex`: its record sounds as the player's own does in a replay, and the skater
    /// microphone follows it.
    pub(super) fn online_audio_skaters(&self) -> Vec<super::AudioSkater> {
        let Some(game) = self.online.game.as_ref() else { return Vec::new() };
        let local = self.online_audio_local().map(|remote| remote.user_id);
        game.remotes
            .values()
            .filter_map(|remote| {
                let audio = remote.shown()?.audio.clone()?;
                Some(super::AudioSkater {
                    player_index: REMOTE_AUDIO_IDS + u32::from(remote.slot),
                    local: local == Some(remote.user_id),
                    phys_out: audio,
                    can_see_player: false,
                    ai_audio: None,
                })
            })
            .collect()
    }

    /// The remote skater that is the local one for the audio: the one the console follows, while
    /// its frames carry an audio record.
    fn online_audio_local(&self) -> Option<&RemoteSkater> {
        self.online_spectated().filter(|remote| remote.shown().is_some_and(|shown| shown.audio.is_some()))
    }

    /// Whether a remote skater is the local one for the audio, the console's own not.
    pub(super) fn online_remote_is_audio_local(&self) -> bool {
        self.online_audio_local().is_some()
    }

    /// The hips of the remote skater that is the local one for the audio, where `DoSimSubmit`
    /// reads the audio layers.
    pub(super) fn online_audio_local_hips(&self) -> Option<[f32; 3]> {
        self.online_audio_local()?.hips()
    }

    /// The players of the game in progress, for the hub and HUD.
    #[must_use]
    pub fn online_players(&self) -> Vec<OnlinePlayer> {
        self.online.game.as_ref().map(|game| game.players.clone()).unwrap_or_default()
    }

    /// `OnlineProfileManager::GetUserName`: the nickname signed in with.
    #[must_use]
    pub fn online_nickname(&self) -> String {
        self.online.nickname.clone()
    }

    /// Whether the online HUDs are open: the local skater is in an online game's world.
    #[must_use]
    pub fn online_hud_active(&self) -> bool {
        self.online.game.as_ref().is_some_and(|game| game.entered)
    }

    /// `Net::ResumeGame` holds: the game keeps simulating under the pause menu and Options, whose
    /// frames then draw as gameplay's do.
    #[must_use]
    pub fn online_simulates_under_frontend(&self) -> bool {
        self.online.network_resumed
    }

    /// The online event log's lines and whether they changed.
    pub fn take_online_event_log(&mut self) -> Option<crate::online::event_log::EventLogSnapshot> {
        self.online.event_log.take_refresh().then(|| self.online.event_log.snapshot())
    }

    /// The lobby HUD's leaderboard screen states since the last call, and `Mission.GetTitle()`
    /// in lobby skate (`MissionCmdTranslator::sMethod_GetTitle` 0x8260e208 without a main
    /// challenge).
    pub fn take_online_lobby_hud(&mut self) -> (Vec<(i32, crate::challenge::turn_hud::TurnHudView)>, crate::challenge::system::MissionTitle) {
        use crate::challenge::system::MissionTitle;
        let freeskate = self.online.game.as_ref().is_some_and(|game| game.attributes.challenge_type == challenge::ONLINE_FREE_SKATE);
        (self.online.lobby_hud.take_screen_states(), if freeskate { MissionTitle::FreeskateLobby } else { MissionTitle::Lobby })
    }

    /// Whether the local player is the game host peer.
    #[must_use]
    pub fn online_is_host(&self) -> bool {
        self.online.game.as_ref().and_then(OnlineGame::host).is_some_and(|host| Some(host) == self.online.user_id)
    }

    /// Asks to start the game's challenge (`GoToStartChallenge` 0x826617a0's `GameRequest(1)`).
    pub fn online_start_challenge(&mut self) {
        self.online_game_request(request::START_GAME, 0);
    }

    /// Kicks a player (`Net::KickPlayer` 0x8278dfc0).
    pub fn online_kick(&mut self, user_id: u64) {
        self.online_game_request(request::KICK_PLAYER, user_id);
    }

    /// Makes the game private or public (`GoToToggleSlotAccess` 0x8266c998).
    pub fn online_set_private(&mut self, private: bool) {
        self.online_game_request(request::CHANGE_PRIVATE_STATUS, u64::from(private));
    }
}
