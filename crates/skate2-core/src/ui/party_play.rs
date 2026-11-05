//! `Sk8::FE::FrontEndState_PartyPlay` (FE state 35, screen 51 `main/partyplay.swf`) and
//! `Sk8::BE::PartyPlayBackEnd` (back-end module 19): the players with their controllers and
//! skaters, then the game and its location.
use super::{
    freeskate_option::{leaf_descendants, Character},
    pause_menu::PauseMenuInput,
};
use crate::challenge::{
    attrib_db::AttribDb,
    unlockables::{category, string_hash, Unlockables},
};

/// `challenge_global_data`, whose leaves `FELocationManager::GetPartyPlayLocations` 0x82635180
/// collects, and `challenges`.
const CHALLENGE_GLOBAL_DATA: u64 = 0xA4F3_5B15_9D74_32B5;
const CHALLENGES: u64 = 0x8876_E0B5_5674_0E36;
/// `challenge_global_data`'s career challenge (`RefSpec`) whose points unlock the location, the
/// flag S.K.A.T.E.'s single-player list keeps, and `FELargeImage`.
const UNLOCK_CHALLENGE: u64 = 0xB2C8_6F65_0BDE_FCE5;
const PARTY_PLAY_FLAG: u64 = 0xF9A9_A1DD_ED84_A643;
const FE_LARGE_IMAGE: u64 = 0x7974_27ED_230C_9980;
/// `challenges`' `PointRequirement` and `Title`.
const POINT_REQUIREMENT: u64 = 0x0C0C_FEFB_9FC6_EB73;
const TITLE: u64 = 0xE2A5_0788_B6C2_0C92;
/// The parent collections `PartyPlayBackEnd::GetPartyPlayLocations` 0x82566d18 asks for:
/// `spot_battle`, `skate2_single_player`, `skate2_online` and `hall_of_meat`.
const SPOT_BATTLE: u64 = 0xE107_B171_03FF_61C1;
const SKATE_SINGLE_PLAYER: u64 = 0x29BB_63AC_B748_A562;
const SKATE_ONLINE: u64 = 0x844F_DE45_E50C_82DE;
const HALL_OF_MEAT: u64 = 0x7680_DF06_42C4_A9BE;

/// `ePartyPlayChallenge`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Game {
    #[default]
    SpotBattle = 0,
    Skate = 1,
    Hom = 2,
}

impl Game {
    pub const ALL: [Self; 3] = [Self::SpotBattle, Self::Skate, Self::Hom];

    /// PC port: the `eChallengeTypes` of the game's challenges (Own the Spot, S.K.A.T.E., Hall
    /// of Meat), whose linked Skate 3 games follow the game's own locations.
    #[must_use]
    pub const fn challenge_type(self) -> i32 {
        match self {
            Self::SpotBattle => 9,
            Self::Skate => 4,
            Self::Hom => 31,
        }
    }
}

/// One location of a game: the challenge activity it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub key: u64,
    /// `GetPartyPlayName` 0x82566a98: the activity's `Title`, "#Invalid Title" without one.
    pub title: String,
    /// `FELargeImage`, the image `GetPartyPlayLargeImage` 0x82566bc0 names under
    /// `fe\source\images\locations\`.
    pub image: String,
}

/// `FELocationManager::GetPartyPlayLocations` for one parent: every leaf while all locations are
/// unlocked; otherwise a leaf whose career challenge's "ABD Points Earned" (`points`) reaches that
/// challenge's `PointRequirement`, or one without a career challenge. `flagged` keeps instead
/// the leaves with `PARTY_PLAY_FLAG` set.
fn add_locations(
    db: &AttribDb,
    all_unlocked: bool,
    parent: u64,
    flagged: bool,
    points: &dyn Fn(u64) -> i32,
    keys: &mut Vec<u64>,
) {
    for key in leaf_descendants(db, CHALLENGE_GLOBAL_DATA, parent) {
        let keep = if flagged {
            db.attribute(CHALLENGE_GLOBAL_DATA, key, PARTY_PLAY_FLAG).and_then(|flag| flag.bool(0)) == Some(true)
        } else if all_unlocked {
            true
        } else {
            match db.attribute(CHALLENGE_GLOBAL_DATA, key, UNLOCK_CHALLENGE).and_then(|refspec| refspec.ref_spec(0)) {
                Some((_, challenge)) if challenge != 0 => {
                    let required = db.attribute(CHALLENGES, challenge, POINT_REQUIREMENT).and_then(|value| value.i32(0)).unwrap_or(0);
                    points(challenge) >= required
                }
                _ => true,
            }
        };
        if keep {
            keys.push(key);
        }
    }
}

/// `PartyPlayBackEnd::GetPartyPlayLocations`: Spot Battle and Hall of Meat their parents' unlocked
/// leaves, S.K.A.T.E. the flagged single-player leaves then the online ones.
#[must_use]
pub fn locations(db: &AttribDb, unlockables: &Unlockables, game: Game, points: &dyn Fn(u64) -> i32) -> Vec<Location> {
    let all_unlocked = unlockables.is_unlocked(category::LOCATIONS, string_hash("all_locations_unlock"), true);
    let mut keys = Vec::new();
    match game {
        Game::SpotBattle => add_locations(db, all_unlocked, SPOT_BATTLE, false, points, &mut keys),
        Game::Skate => {
            add_locations(db, all_unlocked, SKATE_SINGLE_PLAYER, true, points, &mut keys);
            add_locations(db, all_unlocked, SKATE_ONLINE, false, points, &mut keys);
        }
        Game::Hom => add_locations(db, all_unlocked, HALL_OF_MEAT, false, points, &mut keys),
    }
    keys.into_iter()
        .map(|key| {
            let title = db
                .attribute(CHALLENGES, key, TITLE)
                .and_then(|title| title.text(0))
                .filter(|title| !title.is_empty())
                .unwrap_or("#Invalid Title")
                .to_owned();
            let image = db.attribute(CHALLENGE_GLOBAL_DATA, key, FE_LARGE_IMAGE).and_then(|image| image.text(0)).unwrap_or_default().to_owned();
            Location { key, title, image }
        })
        .collect()
}

/// `ePartyPlayState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    NumberOfPlayers,
    ControllerSelect,
    PlayerJoin,
    PlayerSelectCharacter,
    GameSelect,
    LocationSelect,
    ToGame,
}

/// The movie's `SetState` numbers (`FrontEndManager::SetScreenState`).
pub mod screen_state {
    pub const SELECT_PLAYERS: u32 = 1;
    pub const REFRESH_SELECT_PLAYERS: u32 = 2;
    pub const CONTROLLER_SELECT: u32 = 3;
    pub const PLAYER_JOIN: u32 = 4;
    pub const PLAYER_SELECT_CHARACTER: u32 = 5;
    pub const REFRESH_SELECT_CHARACTER: u32 = 6;
    pub const ACCEPT_SELECT_CHARACTER: u32 = 7;
    pub const SETUP_GAME: u32 = 8;
    pub const REFRESH_GAME: u32 = 9;
    pub const SETUP_SELECT_LOCATION: u32 = 10;
    pub const REFRESH_SELECT_LOCATION: u32 = 11;
    pub const REFRESH_IMAGE: u32 = 12;
}

/// The players `IncreaseNumberOfPlayers` 0x826432b0 and `DecreaseNumberOfPlayers` 0x82643328
/// allow.
const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 4;

/// One player as `EnterGame` 0x8264ad08 loads them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub character: Character,
    pub controller: i32,
}

/// What `EnterGame` hands the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyGame {
    pub players: Vec<Player>,
    pub game: Game,
    pub location: Location,
    /// The back end's selection, which the game keeps.
    pub backend: BackEnd,
}

/// `PartyPlayBackEnd` (back-end module 19) as the game keeps it: the game and the location among
/// that game's list. Its lists are the set-up screen's `GetPartyPlayLocations`; nothing in a party
/// game changes what they unlock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackEnd {
    /// `mPartyPlayChallenge`.
    pub game: Game,
    /// `mSelectedLocationIndex`.
    pub location: usize,
    /// `mPartyPlayLocations` of each game.
    pub locations: [Vec<Location>; 3],
}

impl BackEnd {
    /// `NextChallenge` 0x8255d5c8 / `PreviousChallenge` 0x8255d5f0.
    pub fn step_challenge(&mut self, forward: bool) {
        let index = self.game as usize + if forward { 1 } else { Game::ALL.len() - 1 };
        self.game = Game::ALL[index % Game::ALL.len()];
    }

    /// `Reset(bOnlyIndex)` 0x8255d618: the first location, and without `only_index` Spot Battle.
    pub fn reset(&mut self, only_index: bool) {
        self.location = 0;
        if !only_index {
            self.game = Game::SpotBattle;
        }
    }

    /// `RefreshLocations` 0x82566ef0: the game's list from its first location.
    pub fn refresh_locations(&mut self) {
        self.location = 0;
    }

    /// `NextLocation` 0x82566df8 / `PreviousLocation` 0x82566e78, wrapping.
    pub fn step_location(&mut self, forward: bool) {
        let count = self.locations[self.game as usize].len().max(1);
        self.location = if forward { (self.location + 1) % count } else { (self.location + count - 1) % count };
    }

    /// The selected location: `GetChallengeKey` 0x82566fb0's activity, its title and image.
    #[must_use]
    pub fn current(&self) -> Option<&Location> {
        self.locations[self.game as usize].get(self.location)
    }
}

/// `FrontEndState_PartyPlayChangeGame::ePartyPlayOption`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChangeOption {
    #[default]
    SelectChallenge,
    SelectLocation,
}

/// The change-game screen's `SetState` numbers (`partyplaychangegame.kState_*`).
pub mod change_state {
    pub const SELECT_EVENT: u32 = 0;
    pub const SELECT_LOCATION: u32 = 1;
    pub const REFRESH_EVENT: u32 = 2;
    pub const REFRESH_LOCATION: u32 = 3;
    pub const REFRESH_LOCATION_IMAGE: u32 = 4;
}

/// What the change-game screen reads through `FEPartyPlay`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeSnapshot {
    /// `GetPartyPlayChallenge` 0x82604e88.
    pub game: Game,
    /// `GetPartyPlayLocation` 0x82604ea0.
    pub location: String,
    /// `GetPartyPlayLargeImage` 0x82604ed0.
    pub image: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeAction {
    None,
    /// B: `PopState`.
    Back,
    /// A on the running challenge: `RestartMainChallenge(0, true)`, `cMsgHUDRetry` and
    /// `PopToState(0)`.
    Restart,
    /// A on another: `cMsgHUDContinue`, the new challenge and its location in `sGameInfo`, flow
    /// message 6 and `PopToState(0)`.
    Switch(Location),
}

/// `FrontEndState_PartyPlayChangeGame` (FE state 39, screen 65 `challenge/partyplaychangegame`).
#[derive(Clone, Debug, PartialEq)]
pub struct ChangeGame {
    /// `mPartyPlayOption`.
    option: ChangeOption,
    /// `mbRequestNewImage` and `mLocationImageCounter` (seconds).
    image_requested: bool,
    image_time: f32,
    screen_states: Vec<u32>,
}

impl ChangeGame {
    /// `OnEnter` 0x82642ac8: the game, no image request, `RefreshLocations`; then `OpenScreen(65)`.
    pub fn open(backend: &mut BackEnd) -> Self {
        backend.refresh_locations();
        Self { option: ChangeOption::SelectChallenge, image_requested: false, image_time: 0.0, screen_states: Vec::new() }
    }

    pub fn take_screen_states(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.screen_states)
    }

    #[must_use]
    pub fn snapshot(backend: &BackEnd) -> ChangeSnapshot {
        let location = backend.current();
        ChangeSnapshot {
            game: backend.game,
            location: location.map_or_else(|| "#Invalid Title".into(), |l| l.title.clone()),
            image: location.map(|l| l.image.clone()).filter(|image| !image.is_empty()),
        }
    }

    /// `OnUpdate` 0x826488a0: 500 ms after a request, `SetScreenState(4)`.
    pub fn tick(&mut self, dt: f32) {
        if !self.image_requested {
            return;
        }
        self.image_time += dt;
        if self.image_time >= 0.5 {
            self.screen_states.push(change_state::REFRESH_LOCATION_IMAGE);
            self.image_requested = false;
            self.image_time = 0.0;
        }
    }

    /// `SelectMode` 0x82648820.
    fn select_mode(&mut self, option: ChangeOption) {
        self.option = option;
        self.screen_states.push(match option {
            ChangeOption::SelectChallenge => change_state::SELECT_EVENT,
            ChangeOption::SelectLocation => change_state::SELECT_LOCATION,
        });
    }

    /// `RefreshLocation` 0x826487a8: an image request, then `SetScreenState(3)`.
    fn refresh_location(&mut self) {
        self.image_time = 0.0;
        self.image_requested = true;
        self.screen_states.push(change_state::REFRESH_LOCATION);
    }

    /// `ReceiveInput` 0x82677178: up and down switch between the game and the location, left
    /// and right step the one chosen (`NextChallenge` 0x82652ab8 refreshes the game, then the
    /// back end's first location); A picks the location, B goes back. `running` is
    /// `sGameInfo.challengeKey`.
    pub fn update(&mut self, input: PauseMenuInput, backend: &mut BackEnd, running: u64) -> ChangeAction {
        match self.option {
            ChangeOption::SelectChallenge => {
                if input.up || input.down {
                    self.select_mode(ChangeOption::SelectLocation);
                } else if input.left || input.right {
                    backend.step_challenge(input.right);
                    self.screen_states.push(change_state::REFRESH_EVENT);
                    backend.reset(true);
                    backend.refresh_locations();
                    self.refresh_location();
                }
            }
            ChangeOption::SelectLocation => {
                if input.up || input.down {
                    self.select_mode(ChangeOption::SelectChallenge);
                } else if input.left || input.right {
                    backend.step_location(input.right);
                    self.refresh_location();
                }
            }
        }
        if input.select {
            return match backend.current() {
                Some(location) if location.key == running => ChangeAction::Restart,
                Some(location) => ChangeAction::Switch(location.clone()),
                None => ChangeAction::None,
            };
        }
        if input.cancel {
            return ChangeAction::Back;
        }
        ChangeAction::None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// B on the player count: `RunNIS(eFENIS_FromPartyPlayToMainMenu)` and `PopState`.
    Back,
    /// A on the location: `EnableBackground` fades over, then `FadeComplete` 0x8265ae38 runs
    /// `EnterGame`.
    EnterGame(PartyGame),
}

/// What `partyplay.swf` reads through `PartyPlayCmdTranslator`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// `GetPlayerCount` 0x826232b8.
    pub players: usize,
    /// `GetPlayerIndex` 0x82623318: `mActivePlayer`.
    pub active: usize,
    /// `GetSelectedCharacterName` 0x826232e8: the selected skater's `mHALID`.
    pub character: String,
    /// `GetControllerIndex` 0x82623348 of each player.
    pub controllers: [i32; MAX_PLAYERS],
    /// `GetPartyPlayChallenge` 0x82604e88.
    pub game: Game,
    /// `GetPartyPlayLocation` 0x82604ea0.
    pub location: String,
    /// The image `mImage.LoadRX2Texture` shows (`GetPartyPlayLargeImage` 0x82604ed0).
    pub image: Option<String>,
}

pub struct Controller {
    characters: Vec<Character>,
    /// `PartyPlayBackEnd::mPartyPlayLocations` of each game.
    locations: [Vec<Location>; 3],
    mode: Mode,
    /// `mCurrentNumberOfPlayers`, `mActivePlayer`.
    players: usize,
    active: usize,
    /// `mSkaterIndex` and `mControllerIndex`, -1 when unset.
    skaters: [i32; MAX_PLAYERS],
    controllers: [i32; MAX_PLAYERS],
    /// `mSelectedCharacterIndex`.
    selected: usize,
    /// `PartyPlayBackEnd::mPartyPlayChallenge` and `mSelectedLocationIndex`.
    game: Game,
    location: usize,
    /// `mLocalInputState == eInputState_Block` once A sends the game in.
    blocked: bool,
    /// `mbRequestNewImage`, `mLocationImageCounter` (seconds) and the shown image.
    image_requested: bool,
    image_time: f32,
    image: Option<String>,
    /// The skater `SimCharacterViewer` shows: started by `OnIntroDone`, hidden in PlayerJoin and
    /// GameSelect.
    viewer: Option<Option<u64>>,
    viewer_started: bool,
    screen_states: Vec<u32>,
    sounds: Vec<&'static str>,
}

impl Controller {
    /// `OnEnter` 0x8264a870: NumberOfPlayers with two players, the slots cleared, Spot Battle at
    /// its first location, the first skater, then `OpenScreen(51)`.
    #[must_use]
    pub fn new(characters: Vec<Character>, locations: [Vec<Location>; 3]) -> Self {
        Self {
            characters,
            locations,
            mode: Mode::NumberOfPlayers,
            players: MIN_PLAYERS,
            active: 0,
            skaters: [-1; MAX_PLAYERS],
            controllers: [-1; MAX_PLAYERS],
            selected: 0,
            game: Game::SpotBattle,
            location: 0,
            blocked: false,
            image_requested: false,
            image_time: 0.0,
            image: None,
            viewer: None,
            viewer_started: false,
            screen_states: Vec::new(),
            sounds: Vec::new(),
        }
    }

    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    pub fn take_screen_states(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.screen_states)
    }

    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    fn current_location(&self) -> Option<&Location> {
        self.locations[self.game as usize].get(self.location)
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            players: self.players,
            active: self.active,
            character: self.characters.get(self.selected).map(|c| c.label.clone()).unwrap_or_default(),
            controllers: self.controllers,
            game: self.game,
            location: self.current_location().map(|l| l.title.clone()).unwrap_or_else(|| "#Invalid Title".into()),
            image: self.image.clone(),
        }
    }

    /// `OnIntroDone` 0x8264b068: `SimCharacterViewer::Startup` on the `partyplay` location with
    /// the selected skater, not yet shown until a player picks.
    pub fn intro_done(&mut self) {
        self.viewer_started = true;
    }

    /// The skater the viewer shows: a marquee once started, `None` while hidden.
    #[must_use]
    pub fn viewer_character(&self) -> Option<Option<u64>> {
        if self.viewer_started { self.viewer } else { None }
    }

    /// `RefreshPartyPlayLocations` 0x826435d0: the list again from the first location, then the
    /// large image requested (every listed location has one, so `IsValidLargeImage` holds).
    fn refresh_locations(&mut self) {
        self.location = 0;
        self.request_image();
    }

    fn request_image(&mut self) {
        self.image_time = 0.0;
        self.image_requested = true;
    }

    /// `OnUpdate` 0x8264aac8 in LocationSelect: 500 ms after the last request, `SetScreenState(12)`.
    pub fn tick(&mut self, dt: f32) {
        if self.mode != Mode::LocationSelect || !self.image_requested {
            return;
        }
        self.image_time += dt;
        if self.image_time >= 0.5 {
            self.image = self.current_location().map(|l| l.image.clone()).filter(|image| !image.is_empty());
            self.screen_states.push(screen_state::REFRESH_IMAGE);
            self.image_requested = false;
            self.image_time = 0.0;
        }
    }

    /// `NextCharacter` 0x8265ae68 / `PreviousCharacter` 0x8265af80: the next skater, shown, with
    /// the nav sound. The original skips skaters an earlier player holds
    /// (`IsSkaterAlreadySelected` 0x826433e8); here players may share one.
    fn step_character(&mut self, forward: bool) {
        let count = self.characters.len().max(1);
        self.selected = if forward { (self.selected + 1) % count } else { (self.selected + count - 1) % count };
        self.viewer = Some(self.characters.get(self.selected).and_then(|c| c.marquee));
        self.sounds.push(if forward { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
    }

    /// `SetPartyPlayMode` 0x8265aca8.
    fn set_mode(&mut self, mut mode: Mode) {
        loop {
            self.mode = mode;
            match mode {
                Mode::NumberOfPlayers => self.screen_states.push(screen_state::SELECT_PLAYERS),
                Mode::ControllerSelect => {
                    self.screen_states.push(screen_state::CONTROLLER_SELECT);
                    mode = Mode::PlayerJoin;
                    continue;
                }
                Mode::PlayerJoin => {
                    self.screen_states.push(screen_state::PLAYER_JOIN);
                    self.viewer = None;
                }
                Mode::PlayerSelectCharacter => {
                    self.selected = usize::try_from(self.skaters[self.active]).unwrap_or(0);
                    self.screen_states.push(screen_state::PLAYER_SELECT_CHARACTER);
                    self.viewer = Some(self.characters.get(self.selected).and_then(|c| c.marquee));
                }
                Mode::GameSelect => {
                    self.screen_states.push(screen_state::SETUP_GAME);
                    self.viewer = None;
                }
                Mode::LocationSelect => {
                    self.refresh_locations();
                    self.screen_states.push(screen_state::SETUP_SELECT_LOCATION);
                }
                Mode::ToGame => {}
            }
            break;
        }
    }

    /// `ReceiveInput` 0x82669b08 with the pressing controller.
    pub fn update(&mut self, input: PauseMenuInput, controller: i32) -> Action {
        if self.blocked {
            return Action::None;
        }
        match self.mode {
            Mode::NumberOfPlayers => {
                self.active = 0;
                let changed = if input.right && self.players < MAX_PLAYERS {
                    self.sounds.push("FE_SOUND_CORE_NAV_RIGHT");
                    self.players += 1;
                    true
                } else if input.left && self.players > MIN_PLAYERS {
                    self.sounds.push("FE_SOUND_CORE_NAV_LEFT");
                    self.players -= 1;
                    true
                } else {
                    false
                };
                if changed {
                    self.screen_states.push(screen_state::REFRESH_SELECT_PLAYERS);
                } else if input.select {
                    self.set_mode(Mode::ControllerSelect);
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                } else if input.cancel {
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                    return Action::Back;
                }
            }
            Mode::ControllerSelect => {
                if input.cancel {
                    self.set_mode(Mode::NumberOfPlayers);
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                } else if input.select {
                    self.set_mode(Mode::PlayerJoin);
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                }
            }
            Mode::PlayerJoin => {
                if input.cancel {
                    self.screen_states.push(screen_state::CONTROLLER_SELECT);
                    self.skaters[self.active] = -1;
                    if self.active > 0 {
                        self.active -= 1;
                        self.set_mode(Mode::PlayerSelectCharacter);
                    } else {
                        self.set_mode(Mode::NumberOfPlayers);
                    }
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                } else if input.select {
                    self.controllers[self.active] = controller;
                    self.selected = 0;
                    self.skaters[self.active] = self.selected as i32;
                    self.set_mode(Mode::PlayerSelectCharacter);
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                }
            }
            Mode::PlayerSelectCharacter => {
                if self.controllers[self.active] != controller {
                    return Action::None;
                }
                if input.cancel {
                    self.set_mode(Mode::PlayerJoin);
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                } else if input.right || input.left {
                    self.step_character(input.right);
                    self.screen_states.push(screen_state::REFRESH_SELECT_CHARACTER);
                } else if input.select {
                    self.skaters[self.active] = self.selected as i32;
                    self.screen_states.push(screen_state::ACCEPT_SELECT_CHARACTER);
                    self.active += 1;
                    if self.active < self.players {
                        self.set_mode(Mode::PlayerJoin);
                    } else {
                        self.active = self.players - 1;
                        self.set_mode(Mode::GameSelect);
                    }
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                }
            }
            Mode::GameSelect => {
                if input.cancel {
                    self.screen_states.push(screen_state::CONTROLLER_SELECT);
                    self.set_mode(Mode::PlayerSelectCharacter);
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                } else if input.select {
                    self.set_mode(Mode::LocationSelect);
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                } else if input.right || input.left {
                    let index = self.game as usize + if input.right { 1 } else { Game::ALL.len() - 1 };
                    self.game = Game::ALL[index % Game::ALL.len()];
                    self.sounds.push(if input.right { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                    self.screen_states.push(screen_state::REFRESH_GAME);
                }
            }
            Mode::LocationSelect => {
                if input.cancel {
                    self.set_mode(Mode::GameSelect);
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                } else if input.right || input.left {
                    let count = self.locations[self.game as usize].len().max(1);
                    self.location = if input.right { (self.location + 1) % count } else { (self.location + count - 1) % count };
                    self.image_time = 0.0;
                    self.image_requested = true;
                    self.sounds.push(if input.right { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                    self.screen_states.push(screen_state::REFRESH_SELECT_LOCATION);
                } else if input.select {
                    let Some(location) = self.current_location().cloned() else { return Action::None };
                    self.blocked = true;
                    self.mode = Mode::ToGame;
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    let players = (0..self.players)
                        .map(|i| Player {
                            character: self.characters[usize::try_from(self.skaters[i]).unwrap_or(0)].clone(),
                            controller: self.controllers[i],
                        })
                        .collect();
                    let backend = BackEnd { game: self.game, location: self.location, locations: self.locations.clone() };
                    return Action::EnterGame(PartyGame { players, game: self.game, location, backend });
                }
            }
            Mode::ToGame => {}
        }
        Action::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> Controller {
        let characters = (0..4).map(|i| Character { marquee: Some(i), label: format!("ID_SKATER_{i}") }).collect();
        let location = |key: u64| Location { key, title: format!("T{key}"), image: format!("img{key}") };
        Controller::new(characters, [vec![location(1), location(2)], vec![location(3)], vec![location(4)]])
    }

    fn press(c: &mut Controller, input: PauseMenuInput) -> Action {
        c.update(input, 0)
    }

    fn a() -> PauseMenuInput {
        PauseMenuInput { select: true, ..Default::default() }
    }

    fn b() -> PauseMenuInput {
        PauseMenuInput { cancel: true, ..Default::default() }
    }

    fn right() -> PauseMenuInput {
        PauseMenuInput { right: true, ..Default::default() }
    }

    fn left() -> PauseMenuInput {
        PauseMenuInput { left: true, ..Default::default() }
    }

    #[test]
    fn players_join_and_may_share_a_skater_then_pick_game_and_location() {
        let mut c = controller();
        assert_eq!(press(&mut c, right()), Action::None);
        assert_eq!(c.snapshot().players, 3);
        press(&mut c, left());
        press(&mut c, left());
        assert_eq!(c.snapshot().players, 2);
        assert_eq!(c.take_sounds(), ["FE_SOUND_CORE_NAV_RIGHT", "FE_SOUND_CORE_NAV_LEFT"]);
        press(&mut c, a());
        assert_eq!(c.mode(), Mode::PlayerJoin);
        assert_eq!(c.take_screen_states(), [2, 2, screen_state::CONTROLLER_SELECT, screen_state::PLAYER_JOIN]);
        press(&mut c, a());
        assert_eq!(c.snapshot().character, "ID_SKATER_0");
        press(&mut c, a());
        press(&mut c, a());
        assert_eq!(c.snapshot().character, "ID_SKATER_0", "player 1's skater may be shared");
        press(&mut c, right());
        assert_eq!(c.snapshot().character, "ID_SKATER_1");
        press(&mut c, right());
        press(&mut c, right());
        press(&mut c, right());
        assert_eq!(c.snapshot().character, "ID_SKATER_0");
        press(&mut c, a());
        assert_eq!(c.mode(), Mode::GameSelect);
        press(&mut c, right());
        assert_eq!(c.snapshot().game, Game::Skate);
        press(&mut c, a());
        assert_eq!(c.snapshot().location, "T3");
        c.tick(0.6);
        assert_eq!(c.snapshot().image.as_deref(), Some("img3"));
        let Action::EnterGame(game) = press(&mut c, a()) else { panic!("no game") };
        assert_eq!(game.players.iter().map(|p| p.character.marquee).collect::<Vec<_>>(), [Some(0), Some(0)]);
        assert_eq!(game.location.key, 3);
    }

    fn controller_with_created_skater() -> Controller {
        let characters = std::iter::once(Character { marquee: None, label: "#PLAYER 1".into() })
            .chain((0..3).map(|i| Character { marquee: Some(i), label: format!("ID_SKATER_{i}") }))
            .collect();
        let location = |key: u64| Location { key, title: format!("T{key}"), image: format!("img{key}") };
        Controller::new(characters, [vec![location(1)], vec![location(2)], vec![location(3)]])
    }

    #[test]
    fn the_first_player_starts_on_the_created_skater() {
        let mut c = controller_with_created_skater();
        c.intro_done();
        press(&mut c, a());
        press(&mut c, a());
        assert_eq!(c.snapshot().character, "#PLAYER 1");
        assert_eq!(c.viewer_character(), Some(None));
        press(&mut c, right());
        assert_eq!(c.snapshot().character, "ID_SKATER_0");
        press(&mut c, left());
        assert_eq!(c.snapshot().character, "#PLAYER 1");
        press(&mut c, a());
        press(&mut c, a());
        assert_eq!(c.snapshot().character, "#PLAYER 1", "both players may be the created skater");
        press(&mut c, a());
        press(&mut c, a());
        let Action::EnterGame(game) = press(&mut c, a()) else { panic!("no game") };
        assert_eq!(game.players.iter().map(|p| p.character.marquee).collect::<Vec<_>>(), [None, None]);
    }

    #[test]
    fn a_later_player_may_take_the_created_skater() {
        let mut c = controller_with_created_skater();
        press(&mut c, a());
        press(&mut c, a());
        press(&mut c, right());
        press(&mut c, a());
        press(&mut c, a());
        assert_eq!(c.snapshot().character, "#PLAYER 1");
        press(&mut c, right());
        assert_eq!(c.snapshot().character, "ID_SKATER_0", "the first player's skater may be shared");
        press(&mut c, left());
        assert_eq!(c.snapshot().character, "#PLAYER 1");
        press(&mut c, a());
        press(&mut c, a());
        let Action::EnterGame(game) = press(&mut c, a()) else { panic!("no game") };
        assert_eq!(game.players.iter().map(|p| p.character.marquee).collect::<Vec<_>>(), [Some(0), None]);
    }

    #[test]
    fn change_game_steps_the_back_end_and_picks_a_location() {
        let location = |key: u64| Location { key, title: format!("T{key}"), image: format!("img{key}") };
        let mut backend = BackEnd {
            game: Game::SpotBattle,
            location: 1,
            locations: [vec![location(1), location(2)], vec![location(3)], vec![location(4)]],
        };
        let mut change = ChangeGame::open(&mut backend);
        assert_eq!(backend.location, 0, "OnEnter refreshes the locations");
        assert_eq!(change.update(a(), &mut backend, 1), ChangeAction::Restart);
        assert_eq!(change.update(right(), &mut backend, 1), ChangeAction::None);
        assert_eq!(ChangeGame::snapshot(&backend).game, Game::Skate);
        assert_eq!(change.take_screen_states(), [change_state::REFRESH_EVENT, change_state::REFRESH_LOCATION]);
        change.tick(0.6);
        assert_eq!(change.take_screen_states(), [change_state::REFRESH_LOCATION_IMAGE]);
        change.update(PauseMenuInput { down: true, ..Default::default() }, &mut backend, 1);
        assert_eq!(change.take_screen_states(), [change_state::SELECT_LOCATION]);
        assert_eq!(change.update(a(), &mut backend, 1), ChangeAction::Switch(location(3)));
        assert_eq!(change.update(b(), &mut backend, 1), ChangeAction::Back);
    }

    #[test]
    fn back_walks_out_to_the_main_menu() {
        let mut c = controller();
        press(&mut c, a());
        press(&mut c, b());
        assert_eq!(c.mode(), Mode::NumberOfPlayers);
        assert_eq!(press(&mut c, b()), Action::Back);
    }

    #[test]
    fn only_the_joining_controller_picks_its_skater() {
        let mut c = controller();
        press(&mut c, a());
        c.update(a(), 2);
        assert_eq!(c.update(right(), 0), Action::None);
        assert_eq!(c.snapshot().character, "ID_SKATER_0");
        c.update(right(), 2);
        assert_eq!(c.snapshot().character, "ID_SKATER_1");
        assert_eq!(c.snapshot().controllers[0], 2);
    }
}
