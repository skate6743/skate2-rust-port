//! `Sk8::FE::FrontEndState_FreeSkateOption` (FE state 40, screen 66 `main/freeskate_options.swf`):
//! the skater, location, traffic, pedestrian and security choices of a profile free skate, with
//! the `FECharacterManager` / `FELocationManager` lists it steps through.
use super::pause_menu::PauseMenuInput;
use crate::challenge::{
    attrib_db::AttribDb,
    unlockables::{category, string_hash, Unlockables},
};
use skate2_formats::mount::MountSet;

/// `gFECharacterGroups[eFECharacterGroup_FreeSkate]` and
/// `gFELocationTypes[eFELocationType_FreeSkate]`: each class's `freeskate` collection.
const FREESKATE: u64 = 0xF17B_6509_256E_E58B;
/// `gFELocationTypes[eFELocationType_NewCareer]` (`0x82fdb2d0`): the `fe_location_group`
/// collection `FrontEndState_MainMenu::StartSinglePlayer` takes a new career's start from.
const NEW_CAREER: u64 = 0x1700_6FCF_96F1_7E1F;
/// `fe_character_group` and its `marquee` (`RefSpec[]` into `characters_marquee`).
const FE_CHARACTER_GROUP: u64 = 0x7847_1A3E_7D65_0475;
const MARQUEE: u64 = 0x66F0_8967_879F_825C;
pub const CHARACTERS_MARQUEE: u64 = 0x91BC_A669_3EFC_9AA7;
/// `fe_location_group` and its `Location` (`RefSpec[]` into `fe_locations`).
const FE_LOCATION_GROUP: u64 = 0xD897_E7DB_651E_58DC;
const LOCATION: u64 = 0xF1C6_D5FA_4A23_951E;
const FE_LOCATIONS: u64 = 0xDDCD_6BC2_3255_E508;
/// The `fe_locations` world name `CommonBackEnd::GetWorldFilename` 0x82562480 formats.
const WORLD: u64 = 0x116C_A244_49C4_CE44;

/// One `sCharacterItem` of the FreeSkate group: the profile's created skater (`mAttribKey` -1)
/// or a `characters_marquee` collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Character {
    pub marquee: Option<u64>,
    /// `mHALID`: `#` and the profile name, or the marquee's layout text 1.
    pub label: String,
}

/// One `fe_locations` collection of the FreeSkate list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub key: u64,
    /// Layout text 1, the `SetLocationName` start location.
    pub spawn: String,
    /// Layout text 2, `GetLocationName`.
    pub label: String,
    /// Layout text 3, `FELocationManager::GetLocationImage` 0x82615950.
    pub image: String,
}

/// `AttribSysUtils::GetLeafCollections` 0x82ab98c8: the class's leaves `IsDescendantOf` 0x82ab8be8
/// the group, which walks the parent chain from the parent (a collection is not its own).
pub(crate) fn leaf_descendants(db: &AttribDb, class: u64, group: u64) -> Vec<u64> {
    db.leaf_collections(class)
        .into_iter()
        .filter(|&leaf| {
            let mut parent = db.parent(class, leaf);
            while let Some(key) = parent.filter(|&key| key != 0) {
                if key == group {
                    return true;
                }
                parent = db.parent(class, key);
            }
            false
        })
        .collect()
}

fn ref_keys(db: &AttribDb, class: u64, collection: u64, attribute: u64) -> Vec<u64> {
    db.attribute(class, collection, attribute)
        .map(|array| (0..array.len()).filter_map(|i| array.ref_spec(i).map(|(_, key)| key)).collect())
        .unwrap_or_default()
}

/// `FECharacterManager::IsCharacterUnlocked` 0x82606a10 with `kbUnlockFreeSkateSetting` off: the
/// marquee's `GetSkaterName` unlocked, else every character but `big_black` and `dem_bones` once
/// `all_characters_unlock` is.
fn character_unlocked(db: &AttribDb, unlockables: &Unlockables, marquee: u64) -> bool {
    let name = db.layout_text(CHARACTERS_MARQUEE, marquee, 0).unwrap_or_default();
    let hash = string_hash(name);
    if unlockables.is_unlocked(category::CHARACTERS, hash, true) {
        return true;
    }
    unlockables.is_unlocked(category::CHARACTERS, string_hash("all_characters_unlock"), true)
        && hash != string_hash("big_black")
        && hash != string_hash("dem_bones")
}

fn marquee_character(db: &AttribDb, marquee: u64) -> Character {
    Character { marquee: Some(marquee), label: db.layout_text(CHARACTERS_MARQUEE, marquee, 1).unwrap_or_default().to_owned() }
}

/// `FECharacterManager::GetNumCharacters` 0x8261bcd8 / `GetCharacter` 0x8261be58 for the FreeSkate
/// group outside party play: the created skater unless the profile is new (`ProfileBackEnd`+0x68),
/// the group's unlocked marquees (`GetMarqueeUnlockedCharacter` 0x82615d70), then every marquee
/// of its leaf descendants. The FreeSkate group has no descendants, so `GetCharacter`'s leaf
/// offset (one past the created skater) never applies.
#[must_use]
pub fn characters(db: &AttribDb, unlockables: &Unlockables, profile_name: Option<&str>) -> Vec<Character> {
    let mut list: Vec<Character> =
        profile_name.map(|name| Character { marquee: None, label: format!("#{name}") }).into_iter().collect();
    list.extend(
        ref_keys(db, FE_CHARACTER_GROUP, FREESKATE, MARQUEE)
            .into_iter()
            .filter(|&key| character_unlocked(db, unlockables, key))
            .map(|key| marquee_character(db, key)),
    );
    for leaf in leaf_descendants(db, FE_CHARACTER_GROUP, FREESKATE) {
        list.extend(ref_keys(db, FE_CHARACTER_GROUP, leaf, MARQUEE).into_iter().map(|key| marquee_character(db, key)));
    }
    list
}

/// `FELocationManager::IsLocationUnlocked` 0x82615868 (DLC unlocks included).
fn location_unlocked(db: &AttribDb, unlockables: &Unlockables, key: u64) -> bool {
    if !db.has_collection(FE_LOCATIONS, key) {
        return false;
    }
    let name = db.layout_text(FE_LOCATIONS, key, 0).unwrap_or_default();
    unlockables.is_unlocked(category::LOCATIONS, string_hash("all_locations_unlock"), true)
        || unlockables.is_unlocked(category::LOCATIONS, string_hash(name), true)
}

/// `FELocationManager::IsValidLocation` 0x82615998: the location's world has a Sim stream
/// (`AssetPaths::BuildStreamPath` 0x82aba3f0, `.xsm`). `GetWorldFilename` formats the world name
/// with the localizer, whose `{0:SponsorWorld}` parameter the port does not have, so a
/// parameterised world does not format.
fn valid_location(db: &AttribDb, mounts: &MountSet, key: u64) -> bool {
    let Some(world) = db.attribute(FE_LOCATIONS, key, WORLD).and_then(|world| world.text(0)) else { return false };
    if world.is_empty() || world.contains('{') {
        return false;
    }
    mounts.contains_path(&format!("data\\content\\world\\stream\\{world}\\{world}_Sim.xsm"))
}

/// A location's `WorldFile`: `BAM` for the main world, an additional world's stream otherwise.
#[must_use]
pub fn location_world(db: &AttribDb, key: u64) -> Option<String> {
    db.attribute(FE_LOCATIONS, key, WORLD).and_then(|world| world.text(0)).map(str::to_owned)
}

/// `FELocationManager::AddLocationsFromGroup` 0x82635018.
fn add_locations(db: &AttribDb, unlockables: &Unlockables, mounts: &MountSet, group: u64, locked: bool, keys: &mut Vec<u64>) {
    for key in ref_keys(db, FE_LOCATION_GROUP, group, LOCATION) {
        if locked && !location_unlocked(db, unlockables, key) {
            continue;
        }
        if !keys.contains(&key) && valid_location(db, mounts, key) {
            keys.push(key);
        }
    }
}

/// `FELocationManager::GetLocations` 0x826354e8 of a location type's group: its valid locations
/// (only the unlocked ones for the FreeSkate type), then its leaf descendants' valid locations.
fn type_locations(db: &AttribDb, unlockables: &Unlockables, mounts: &MountSet, group: u64, locked: bool) -> Vec<Location> {
    let mut keys = Vec::new();
    add_locations(db, unlockables, mounts, group, locked, &mut keys);
    for leaf in leaf_descendants(db, FE_LOCATION_GROUP, group) {
        add_locations(db, unlockables, mounts, leaf, false, &mut keys);
    }
    keys.into_iter()
        .map(|key| {
            let text = |word| db.layout_text(FE_LOCATIONS, key, word).unwrap_or_default().to_owned();
            Location { key, spawn: text(1), label: text(2), image: text(3) }
        })
        .collect()
}

/// `FELocationManager::GetLocations(eFELocationType_FreeSkate)` 0x826354e8: the group's unlocked
/// valid locations, then its leaf descendants' valid locations.
#[must_use]
pub fn locations(db: &AttribDb, unlockables: &Unlockables, mounts: &MountSet) -> Vec<Location> {
    type_locations(db, unlockables, mounts, FREESKATE, true)
}

/// `FELocationManager::GetLocations(eFELocationType_NewCareer)` 0x826354e8: the group's valid
/// locations without the unlock test (`gamestart`), then its leaf descendants'.
#[must_use]
pub fn new_career_locations(db: &AttribDb, unlockables: &Unlockables, mounts: &MountSet) -> Vec<Location> {
    type_locations(db, unlockables, mounts, NEW_CAREER, false)
}

/// `eFreeSkateOptionState`, also `freeskate_options`' `kState_*`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Row {
    #[default]
    Character = 0,
    Location = 1,
    Traffic = 2,
    Pedestrians = 3,
    Security = 4,
}

impl Row {
    const ALL: [Self; 5] = [Self::Character, Self::Location, Self::Traffic, Self::Pedestrians, Self::Security];
}

/// `FrontEndManager::SetScreenState`: the movie's `SetState` refreshes one row, or with 999
/// `UpdateSelection`.
pub const SELECTION_STATE: u32 = 999;

/// `CommonBackEnd::sGameInfo`'s free-skate settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub traffic: f32,
    pub pedestrians: f32,
    pub security: bool,
}

impl Default for Settings {
    /// The `GameInfo` constructor's 1.0 densities with security on.
    fn default() -> Self {
        Self { traffic: 1.0, pedestrians: 1.0, security: true }
    }
}

/// What `EnterGame` 0x8265b230 hands the game module.
#[derive(Clone, Debug, PartialEq)]
pub struct GameSettings {
    pub character: Character,
    pub location: Location,
    pub settings: Settings,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    None,
    /// B: `RunNIS(eFENIS_FromFreeSkateToMainMenu)` and `PopState`.
    Back,
    /// A: `EnableBackground` fades over, then `FadeComplete` 0x8266a5b0 runs `EnterGame`.
    EnterGame(GameSettings),
}

/// What `freeskate_options.swf` reads through `FreeSkateOptionCmdTranslator`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    /// `GetSelection` 0x82643648.
    pub selection: Row,
    /// `GetCharacterName` 0x82643640.
    pub character: String,
    /// `GetLocationName` 0x8264b1b8.
    pub location: String,
    /// `sMethod_GetTrafficSetting` 0x82623398.
    pub traffic: i32,
    /// `sMethod_GetPedestrianSetting` 0x82623408.
    pub pedestrians: i32,
    /// `sMethod_GetSecuritySetting` 0x82623490.
    pub security: bool,
    /// The location image the secondary quad shows (`mbSecondaryQuadActive`).
    pub image: Option<String>,
}

pub struct Controller {
    characters: Vec<Character>,
    locations: Vec<Location>,
    /// `mFreeSkateOption`.
    row: Row,
    /// `mSelectedCharacterIndex`.
    character: usize,
    /// `mSelectedLocationIndex`.
    location: usize,
    settings: Settings,
    /// `mWaitToCompleteOnEnter`: Character, A and B wait for the intro and the first load.
    wait_on_enter: bool,
    /// `mLocalInputState == eInputState_Block` after A.
    blocked: bool,
    /// `mbRequestNewImage`, `mLocationImageCounter` (seconds here) and the shown image.
    image_requested: bool,
    image_time: f32,
    image: Option<String>,
    /// The character `UpdateCharacter` last gave `SimCharacterViewer` (`Startup`, then
    /// `ShowCharacter`): a marquee, or the created skater.
    viewer: Option<Option<u64>>,
    /// `SetScreenState` calls in order, for the movie.
    screen_states: Vec<u32>,
    /// `GlobalFEPlaySound` calls in order.
    sounds: Vec<&'static str>,
}

impl Controller {
    /// `OnEnter` 0x8265b098: the settings from `sGameInfo`, the first character and location, the
    /// Character row, then `OpenScreen(66)`.
    #[must_use]
    pub fn new(characters: Vec<Character>, locations: Vec<Location>, settings: Settings) -> Self {
        Self {
            characters,
            locations,
            row: Row::Character,
            character: 0,
            location: 0,
            settings,
            wait_on_enter: true,
            blocked: false,
            image_requested: false,
            image_time: 0.0,
            image: None,
            viewer: None,
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
    pub fn snapshot(&self) -> Snapshot {
        let bars = |value: f32| {
            let bars = value * 10.0 + 0.1;
            if bars < 1.0 { 0 } else { bars as i32 }
        };
        Snapshot {
            selection: self.row,
            character: self.characters.get(self.character).map(|c| c.label.clone()).unwrap_or_default(),
            location: self.locations.get(self.location).map(|l| l.label.clone()).unwrap_or_default(),
            traffic: bars(self.settings.traffic),
            pedestrians: bars(self.settings.pedestrians),
            security: self.settings.security,
            image: self.image.clone(),
        }
    }

    /// `OnIntroDone` 0x8264b100 requests the first location's image and loads the created skater;
    /// the loaded player is already that skater, so `OnUpdate`'s wait ends at once with
    /// `UpdateCharacter`.
    pub fn intro_done(&mut self) {
        self.request_image();
        if self.wait_on_enter {
            self.update_character();
            self.wait_on_enter = false;
        }
    }

    fn request_image(&mut self) {
        self.image_requested = true;
        self.image_time = 0.0;
    }

    /// `UpdateCharacter` 0x8265b4d0: `GetCharacter`, `SetScreenState(0)`, then the viewer's
    /// `Startup` or `ShowCharacter` with the character's marquee key.
    fn update_character(&mut self) {
        self.screen_states.push(Row::Character as u32);
        if let Some(character) = self.characters.get(self.character) {
            self.viewer = Some(character.marquee);
        }
    }

    /// The character the viewer shows, once `UpdateCharacter` has started it.
    #[must_use]
    pub const fn viewer_character(&self) -> Option<Option<u64>> {
        self.viewer
    }

    /// `NextLocation` 0x8265b6b0 / `PreviousLocation` 0x8265b7c0: the secondary quad goes, the
    /// image is requested again, then `SetScreenState(1)`.
    fn step_location(&mut self, forward: bool) {
        self.request_image();
        self.image = None;
        let count = self.locations.len().max(1);
        self.location = if forward { (self.location + 1) % count } else { (self.location + count - 1) % count };
        self.screen_states.push(Row::Location as u32);
    }

    /// `More/LessTraffic`, `More/LessPedestrian` (0.1 steps; up to 1.0, under 0.09 is 0).
    fn step_density(value: &mut f32, more: bool) {
        if more {
            *value += 0.1;
            if *value > 1.0 {
                *value = 1.0;
            }
        } else {
            *value -= 0.1;
            if *value < f32::from_bits(0x3DB8_51EC) {
                *value = 0.0;
            }
        }
    }

    /// `OnUpdate` 0x8266a298's image request: 500 ms after the last request the location image
    /// replaces the secondary quad's texture.
    pub fn tick(&mut self, dt: f32) {
        if !self.image_requested {
            return;
        }
        self.image_time += dt;
        if self.image_time >= 0.5 {
            self.image = self.locations.get(self.location).map(|l| l.image.clone());
            self.image_requested = false;
            self.image_time = 0.0;
        }
    }

    /// `ReceiveInput` 0x82670fc8: Down/Up (keycodes 13/12) move between the rows, Right/Left
    /// (15/14) change the row's value, A (2) and B (3) wait for the first character.
    pub fn update(&mut self, input: PauseMenuInput) -> Action {
        if self.blocked {
            return Action::None;
        }
        if input.up {
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
            self.row = Row::ALL[(self.row as usize + 4) % 5];
            self.screen_states.push(SELECTION_STATE);
        }
        if input.down {
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
            self.row = Row::ALL[(self.row as usize + 1) % 5];
            self.screen_states.push(SELECTION_STATE);
        }
        for more in [input.left.then_some(false), input.right.then_some(true)].into_iter().flatten() {
            match self.row {
                Row::Character => {
                    if self.wait_on_enter {
                        continue;
                    }
                    let count = self.characters.len().max(1);
                    self.character = if more { (self.character + 1) % count } else { (self.character + count - 1) % count };
                    self.update_character();
                    self.sounds.push(if more { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                }
                Row::Location => self.step_location(more),
                Row::Traffic => {
                    Self::step_density(&mut self.settings.traffic, more);
                    self.screen_states.push(Row::Traffic as u32);
                }
                Row::Pedestrians => {
                    Self::step_density(&mut self.settings.pedestrians, more);
                    self.screen_states.push(Row::Pedestrians as u32);
                }
                Row::Security => {
                    self.settings.security = !self.settings.security;
                    self.screen_states.push(Row::Security as u32);
                }
            }
        }
        if self.wait_on_enter {
            return Action::None;
        }
        if input.select {
            self.blocked = true;
            return Action::EnterGame(self.game_settings());
        }
        if input.cancel {
            return Action::Back;
        }
        Action::None
    }

    /// `UpdateGameSettings` 0x8264b268's character, location and `sGameInfo` densities.
    fn game_settings(&self) -> GameSettings {
        let mut settings = self.settings;
        for value in [&mut settings.traffic, &mut settings.pedestrians] {
            if *value < f32::from_bits(0x3DB8_51EC) {
                *value = 0.0;
            }
        }
        GameSettings {
            character: self.characters[self.character].clone(),
            location: self.locations[self.location].clone(),
            settings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> Controller {
        let characters = vec![
            Character { marquee: None, label: "#PLAYER 1".into() },
            Character { marquee: Some(1), label: "ID_COMMON_CUZ".into() },
        ];
        let location = |key, label: &str| Location { key, spawn: format!("S{key}"), label: label.into(), image: format!("img{key}") };
        Controller::new(characters, vec![location(1, "A"), location(2, "B")], Settings::default())
    }

    #[test]
    fn rows_wrap_and_character_waits_for_the_intro() {
        let mut c = controller();
        c.update(PauseMenuInput { right: true, ..Default::default() });
        assert_eq!(c.snapshot().character, "#PLAYER 1");
        assert_eq!(c.update(PauseMenuInput { select: true, ..Default::default() }), Action::None);
        c.intro_done();
        assert_eq!(c.take_screen_states(), [0]);
        c.update(PauseMenuInput { right: true, ..Default::default() });
        assert_eq!(c.snapshot().character, "ID_COMMON_CUZ");
        assert_eq!(c.take_sounds(), ["FE_SOUND_CORE_NAV_RIGHT"]);
        c.update(PauseMenuInput { up: true, ..Default::default() });
        assert_eq!(c.snapshot().selection, Row::Security);
        assert_eq!(c.take_screen_states(), [0, SELECTION_STATE]);
    }

    #[test]
    fn densities_step_and_floor_like_the_native() {
        let mut c = controller();
        c.intro_done();
        c.update(PauseMenuInput { down: true, ..Default::default() });
        c.update(PauseMenuInput { down: true, ..Default::default() });
        c.update(PauseMenuInput { right: true, ..Default::default() });
        assert_eq!(c.snapshot().traffic, 10);
        for _ in 0..10 {
            c.update(PauseMenuInput { left: true, ..Default::default() });
        }
        assert_eq!(c.snapshot().traffic, 0);
        let Action::EnterGame(game) = c.update(PauseMenuInput { select: true, ..Default::default() }) else { panic!() };
        assert_eq!(game.settings.traffic, 0.0);
        assert_eq!(game.location.spawn, "S1");
    }

    #[test]
    fn location_image_follows_the_last_step_after_half_a_second() {
        let mut c = controller();
        c.intro_done();
        c.tick(0.3);
        assert_eq!(c.snapshot().image, None);
        c.tick(0.25);
        assert_eq!(c.snapshot().image.as_deref(), Some("img1"));
        c.update(PauseMenuInput { down: true, ..Default::default() });
        c.update(PauseMenuInput { right: true, ..Default::default() });
        assert_eq!(c.snapshot().image, None);
        c.tick(0.5);
        assert_eq!(c.snapshot().image.as_deref(), Some("img2"));
    }
}
