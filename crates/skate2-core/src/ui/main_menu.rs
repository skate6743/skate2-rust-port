//! FENISManager82615470 / StateMainMenu82778340 / MainMenu82678c20.
//! This frontend retains the running free-skate session. The original offline
//! Skate.Reel route is available from Online Options beside Free Skate/Extras.
//! PC additions: the Exit to Desktop row, and the Downloadable Content screen listing the linked
//! downloadable content in place of the Marketplace (`dlc_menu`).
use super::{freeskate_option, party_play};
use crate::attribute_runtime::AttributeRuntimeVault;
use skate2_formats::{attrib::string_hash64, ice_camera::CameraSet, mount::MountSet};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const BOOT: u32 = 0xda792ba5;
pub const ENTER: u32 = 0x30a969ab;
pub const RETURN: u32 = 0x3dcbc1ca;
/// `FENISManager::mNISList` (0x82fdb240) entries 2 and 4, keyed by `ICELITE::StringHash` of
/// "From MainMenu To FreeSkate" and "From FreeSkate To MainMenu".
pub const TO_FREESKATE: u32 = 0x3ad9cfca;
pub const FROM_FREESKATE: u32 = 0xad533c2a;
/// Entries 3 and 5: "From MainMenu To PartyPlay" and "From PartyPlay To MainMenu".
pub const TO_PARTY_PLAY: u32 = 0x25aa8f76;
pub const FROM_PARTY_PLAY: u32 = 0x8c14ddd6;
/// FENIS table82fdb240 entries8/7 retain the historical Extras names for
/// the Online screen. These are distinct from the actual Extras tab push.
pub const TO_ONLINE:u32=0x2b361307;
pub const FROM_ONLINE:u32=0xf0a38de7;
/// The `fe_3d_screen` quads `RunNIS` places the APT layer on: the main menu's and FreeSkate's.
const MAIN_QUAD: u64 = 0x5453b391061ee548;
const FREESKATE_QUAD: u64 = 0xf17b6509256ee58b;
const PARTY_PLAY_QUAD: u64 = 0x40424a7d0c50aef8;
// PC intro trim: skip BootToPressStart and begin 45 frames
// into PressStartToMainMenu. This is an intentional presentation preference,
// not an original-game timing value. Remaining ICE samples/speed are unchanged.
const STARTUP_ALLEY_TIME: f32 = 45.0 / 60.0;
/// The original's seven rows, then the PC's Exit to Desktop in menu_picker's eighth authored
/// slot (`dummy7`).
pub const LABELS: [&str; 8] = [
    "ID_MAINMENU_CONTINUECAREER",
    "ID_SCREEN_GAME_MODE_ONLINE",
    "ID_SCREEN_GAME_MODE_FREESKATE",
    "ID_SCREEN_GAME_MODE_PARTY_PLAY",
    "ID_MAIN_MENU_DOWNLOADABLE_CONTENT",
    "ID_MAIN_MENU_OPTIONS",
    "ID_MAIN_MENU_EXTRAS",
    "#Exit to Desktop",
];

/// mainmenu.apt asks `ProfileCmdTranslator::sMethod_IsNewProfile`82604118
/// for row 0: New Career while `ProfileBackEnd.mNewProfile` is set.
#[must_use]
pub fn labels(new_profile: bool) -> [&'static str; 8] {
    let mut labels = LABELS;
    if new_profile {
        labels[0] = "ID_MAINMENU_NEWCAREER";
    }
    labels
}

/// The front end state the camera serves: the main menu (state 22), FreeSkateOption (state 40),
/// or a FENIS sequence between them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    MainMenu,
    ToFreeSkate,
    FreeSkate,
    FromFreeSkate,
    ToPartyPlay,
    PartyPlay,
    FromPartyPlay,
    ToOnline,
    Online,
    FromOnline,
}

/// What the owner does after a main-menu update.
#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    None,
    SaveProfileName(String),
    /// Row 1: the Online NIS started; the owner reads `online-config.json`.
    Online,
    SaveOnlineNickname(String),
    /// Row 0: `StartSinglePlayer`.
    ContinueCareer,
    /// Row 2: the FreeSkate NIS started; the owner builds `FrontEndState_FreeSkateOption`.
    FreeSkate,
    /// FreeSkateOption's A.
    EnterFreeSkate(freeskate_option::GameSettings),
    /// Row 3: the PartyPlay NIS started; the owner builds `FrontEndState_PartyPlay`.
    PartyPlay,
    /// PartyPlay's A on the location.
    EnterPartyPlay(party_play::PartyGame),
    OpenExtras,
    Extras(super::extras::Action),
    /// Row 4 (PC): FE state 25's screen; the owner builds it with the linked packs.
    /// `OnMenuSelectGame` 0x82681338 sets that state, the Marketplace, only for a signed-in Xbox
    /// LIVE user.
    OpenDlc,
    /// PC: its Link DLCs; the owner opens the file picker.
    LinkDlc,
    /// PC: row 7, Exit to Desktop.
    ExitToDesktop,
    /// PC: Yes to the update offered (`update_prompt`).
    InstallUpdate,
}

pub struct Presentation {
    cameras: CameraSet,
    /// The main menu's, FreeSkate's and PartyPlay's `fe_3d_screen` quads and scales: the APT
    /// screen's, then the secondary texture's.
    walls: [[([[f32; 4]; 4], f32); 2]; 4],
    far: f32,
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub camera: [f32; 16],
    pub clip: [f32; 16],
    pub wall: [[f32; 4]; 4],
    pub scale: f32,
    /// `AptRenderingIntegration`'s secondary quad and scale, which draws `mpSecondaryTextureAsset`.
    pub secondary: ([[f32; 4]; 4], f32),
    pub selected: usize,
    pub menu_visible: bool,
    /// The frontend scene's ICE user channels, applied to `PostfxPresentation` as a NIS take's.
    pub ice_postfx: crate::nis::manager::IcePostfx,
    /// The camera's near and far planes.
    pub planes: [f32; 2],
    /// Row 0 reads New Career (`labels`).
    pub new_profile: bool,
    pub screen: Screen,
    /// The open FreeSkateOption screen.
    pub freeskate: Option<freeskate_option::Snapshot>,
    /// The open PartyPlay screen.
    pub party_play: Option<party_play::Snapshot>,
    pub extras: Option<super::extras::Snapshot>,
    pub name_prompt: Option<super::profile_name::Prompt>,
    pub nickname_prompt: Option<super::profile_name::Prompt>,
    pub skate_reel:Option<super::skate_reel::Snapshot>,
    /// The online matchmaking screen in place of the Online options (FE states 68 and 69).
    pub online_match: Option<super::online_match::Snapshot>,
    /// The Lobby Browser in place of the Online options.
    pub lobby_browser: Option<super::lobby_browser::Snapshot>,
    /// `OnlineProfileManager::GetUserName`: the player's online nickname.
    pub online_nickname: String,
    /// PC: the open Downloadable Content screen.
    pub dlc: Option<super::dlc_menu::Snapshot>,
    /// PC: the update's popup.
    pub update_prompt: Option<super::update_prompt::UpdatePrompt>,
}
/// One `fe_3d_screen` layout: the positions, rotations and scales `RunNIS`82615470 copies into
/// `AptRenderingIntegration`, layout floats 5,4,3 / 13,12,11 / 7 for the APT screen and
/// 2,1,0 / 10,9,8 / 6 for the secondary quad.
fn quads(vault: &std::sync::Arc<AttributeRuntimeVault>, key: u64) -> Result<[([[f32; 4]; 4], f32); 2]> {
    let instance = vault.instance(string_hash64("fe_3d_screen"), key);
    let data = instance
        .try_cached_layout_field(0, 56)?
        .ok_or("frontend quad layout absent")?;
    let mut f = [0.; 14];
    for (v, b) in f.iter_mut().zip(data.bytes().chunks_exact(4)) {
        *v = f32::from_be_bytes(b.try_into()?);
    }
    let quad = |position: [f32; 3], rotation: [f32; 3]| {
        let mut wall = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [position[0], position[1], position[2], 1.],
        ];
        // Render3D8262a550 rotates the three local axes in X,Y,Z order.
        let radians = f32::from_bits(0x3c8efa35);
        for (a, b, angle) in [(1, 2, rotation[0]), (2, 0, rotation[1]), (0, 1, rotation[2])] {
            let (s, c) = (angle * radians).sin_cos();
            let x = wall[a];
            let y = wall[b];
            wall[a] = std::array::from_fn(|i| c * x[i] + s * y[i]);
            wall[b] = std::array::from_fn(|i| c * y[i] - s * x[i]);
        }
        wall
    };
    Ok([
        (quad([f[5], f[4], f[3]], [f[13], f[12], f[11]]), f[7]),
        (quad([f[2], f[1], f[0]], [f[10], f[9], f[8]]), f[6]),
    ])
}
impl Presentation {
    pub fn load(mounts: &MountSet) -> Result<Self> {
        let cameras = CameraSet::parse(&mounts.read_path("data/scene/ice_000.bin")?)?;
        let vault = AttributeRuntimeVault::try_load_mounted(
            mounts,
            "data/db/skater.vlt",
            &[string_hash64("fe_3d_screen"), string_hash64("rendering")],
        )
        .map_err(|e| e.to_string())?;
        let walls = [quads(&vault, MAIN_QUAD)?, quads(&vault, FREESKATE_QUAD)?, quads(&vault, PARTY_PLAY_QUAD)?,quads(&vault,0x62b6afde0ba6567b)?];
        let rendering = vault.instance(string_hash64("rendering"), 0xd7edbd362d7d2152);
        let far = f32::from_be_bytes(
            rendering
                .try_cached_layout_field(0x64, 4)?
                .ok_or("frontend far clip absent")?
                .bytes()
                .try_into()?,
        );
        for hash in [BOOT, ENTER, RETURN, TO_FREESKATE, FROM_FREESKATE, TO_PARTY_PLAY, FROM_PARTY_PLAY,TO_ONLINE,FROM_ONLINE] {
            if !cameras.groups.contains_key(&(0x80000006, hash)) {
                return Err(format!("frontend ICE take {hash:08x} absent").into());
            }
        }
        Ok(Self { cameras, walls, far })
    }
    pub fn duration(&self, hash: u32) -> f32 {
        self.cameras.groups[&(0x80000006, hash)][0].length
    }
    pub fn frame(&self, state: &Controller, near: f32, fraction: f32) -> Result<Frame> {
        let time = if state.ready && !state.settled && !state.leaving {
            (state.time + fraction).min(self.duration(state.track))
        } else {
            state.time
        };
        let (camera, fov) = self.cameras.world_camera(state.track, time)?;
        let playback = self.cameras.world_playback(state.track, time)?;
        let mut ice_postfx = crate::nis::manager::IcePostfx::default();
        ice_postfx.apply_take(playback.bloom(), playback.fade(), crate::nis::manager::PostfxChannels::read(&playback));
        let matrix = crate::skeleton_root_transforms::SkeletonRootMatrix::known(camera);
        let clip = crate::camera_session::producer_projection::project_producer(
            matrix, matrix, fov, fov, near, self.far, false, true, -1, 0,
        )
        .clip_from_world;
        // RunNIS moves the APT quads when its sequence starts.
        let wall = match state.screen {
            Screen::ToFreeSkate | Screen::FreeSkate => 1,
            Screen::ToPartyPlay | Screen::PartyPlay => 2,
            Screen::ToOnline | Screen::Online=>3,
            _ => 0,
        };
        let [(wall, scale), secondary] = self.walls[wall];
        Ok(Frame {
            camera,
            clip,
            wall,
            scale,
            secondary,
            selected: state.selected,
            menu_visible: state.settled && state.screen == Screen::MainMenu && state.extras.is_none() && state.dlc.is_none(),
            ice_postfx,
            planes: [near, self.far],
            new_profile: state.new_profile,
            screen: state.screen,
            extras: state.extras.as_ref().map(super::extras::Controller::snapshot),
            name_prompt: state.name_prompt.as_ref().filter(|_| state.settled && state.intro_complete).cloned(),
            nickname_prompt: state.nickname_prompt.as_ref().filter(|_| state.nickname_prompt_open()).cloned(),
            skate_reel:state.skate_reel.as_ref().filter(|_|state.screen==Screen::Online).map(super::skate_reel::Controller::snapshot),
            freeskate: state.freeskate.as_ref().filter(|_| state.screen == Screen::FreeSkate).map(freeskate_option::Controller::snapshot),
            party_play: state.party_play.as_ref().filter(|_| state.screen == Screen::PartyPlay).map(party_play::Controller::snapshot),
            online_match: None,
            lobby_browser: None,
            online_nickname: String::new(),
            dlc: state.dlc.as_ref().map(super::dlc_menu::Controller::snapshot),
            update_prompt: state.update_prompt.clone(),
        })
    }
    /// Camera positions drive loading; no guessed frontend spawn coordinates.
    pub fn focus(&self) -> Result<[f32; 3]> {
        let (m, _) = self.cameras.world_camera(ENTER, self.duration(ENTER))?;
        Ok([m[12], m[13], m[14]])
    }
}
pub struct Controller {
    pub track: u32,
    pub time: f32,
    pub selected: usize,
    pub settled: bool,
    pub intro_complete: bool,
    pub leaving: bool,
    pub ready: bool,
    pub input_paused: bool,
    /// `ProfileBackEnd.mNewProfile` when the menu opened.
    pub new_profile: bool,
    pub screen: Screen,
    /// `FrontEndState_FreeSkateOption`, built when its NIS starts and shown when it ends.
    pub freeskate: Option<freeskate_option::Controller>,
    /// `FrontEndState_PartyPlay`, likewise.
    pub party_play: Option<party_play::Controller>,
    pub extras: Option<super::extras::Controller>,
    pub name_prompt: Option<super::profile_name::Prompt>,
    /// PC online nickname prompt, opened once the Online screen is shown.
    pub nickname_prompt: Option<super::profile_name::Prompt>,
    /// `online-config.json` had no nickname when Online was selected.
    pub nickname_needed: bool,
    pub skate_reel:Option<super::skate_reel::Controller>,
    /// PC: FE state 25's screen, over the main menu as the original's replaces it.
    pub dlc: Option<super::dlc_menu::Controller>,
    /// PC: the update's popup, and the release the player said No to, which is not offered again
    /// until the server refuses this version.
    pub update_prompt: Option<super::update_prompt::UpdatePrompt>,
    pub update_declined: Option<String>,
    /// PC: the release's changes, shown once the menu stands idle at the first start after an
    /// update.
    pub changelog: Option<super::update_prompt::Changelog>,
    /// `FE_SOUND_*` globals the menu_picker's script played for this input.
    sounds: Vec<&'static str>,
}
impl Controller {
    pub fn new(startup: bool) -> Self {
        Self {
            track: if startup { ENTER } else { RETURN },
            time: if startup { STARTUP_ALLEY_TIME } else { 0. },
            selected: 0,
            settled: false,
            intro_complete: false,
            leaving: false,
            ready: false,
            input_paused: !startup,
            new_profile: false,
            screen: Screen::MainMenu,
            freeskate: None,
            party_play: None,
            extras: None,
            name_prompt: None,
            nickname_prompt: None,
            nickname_needed: false,
            skate_reel:None,
            dlc: None,
            update_prompt: None,
            update_declined: None,
            changelog: None,
            sounds: Vec::new(),
        }
    }
    /// PC: the main menu with nothing open over it, where the update is offered.
    fn idle(&self) -> bool {
        self.settled
            && self.intro_complete
            && !self.leaving
            && self.screen == Screen::MainMenu
            && self.freeskate.is_none()
            && self.party_play.is_none()
            && self.extras.is_none()
            && self.name_prompt.is_none()
            && self.nickname_prompt.is_none()
            && self.skate_reel.is_none()
            && self.dlc.is_none()
    }
    /// PC: the app's updater as it stands: a newer release is offered once the menu is idle
    /// (`FrontEndState_Popup::OnEnter` 0x826583d0 sounds `core_popup`), and the update's progress,
    /// end and failure take the popup's place.
    pub fn set_update_status(&mut self, status: &super::update_prompt::UpdateStatus, current: &str) {
        use super::update_prompt::{UpdatePrompt, UpdateStatus};
        let updating = matches!(self.update_prompt, Some(UpdatePrompt::Downloading { .. }));
        match status {
            UpdateStatus::Available { version }
                if self.update_prompt.is_none() && self.idle() && self.changelog.is_none() && self.update_declined.as_ref() != Some(version) =>
            {
                self.update_prompt = Some(UpdatePrompt::Offer { version: version.clone(), current: current.to_owned(), selected: 0 });
                self.sounds.push("FE_SOUND_CORE_POPUP");
            }
            UpdateStatus::Downloading { version, percent } if updating => {
                self.update_prompt = Some(UpdatePrompt::Downloading { version: version.clone(), percent: *percent });
            }
            UpdateStatus::Installed { version } if updating => self.update_prompt = Some(UpdatePrompt::Installed { version: version.clone() }),
            UpdateStatus::Failed { message } if updating => {
                self.update_prompt = Some(UpdatePrompt::Failed { message: message.clone() });
                self.sounds.push("FE_SOUND_CORE_POPUP");
            }
            _ => {}
        }
    }
    /// The nickname prompt takes text once the Online screen's intro is done.
    pub fn nickname_prompt_open(&self) -> bool {
        self.nickname_prompt.is_some()
            && self.settled
            && self.screen == Screen::Online
            && self.skate_reel.as_ref().is_some_and(|reel| reel.ready)
    }
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        let mut sounds = std::mem::take(&mut self.sounds);
        if let Some(freeskate) = self.freeskate.as_mut() {
            sounds.extend(freeskate.take_sounds());
        }
        if let Some(party_play) = self.party_play.as_mut() {
            sounds.extend(party_play.take_sounds());
        }
        if let Some(extras) = self.extras.as_mut() { sounds.extend(extras.take_sounds()); }
        if let Some(dlc) = self.dlc.as_mut() { sounds.extend(dlc.take_sounds()); }
        if let Some(reel)=&mut self.skate_reel {sounds.extend(reel.take_sounds());}
        sounds
    }
    /// The PartyPlay `SetScreenState` calls since the last take.
    pub fn take_party_play_screen_states(&mut self) -> Vec<u32> {
        self.party_play.as_mut().map(party_play::Controller::take_screen_states).unwrap_or_default()
    }
    /// The FreeSkateOption `SetScreenState` calls since the last take.
    pub fn take_freeskate_screen_states(&mut self) -> Vec<u32> {
        self.freeskate.as_mut().map(freeskate_option::Controller::take_screen_states).unwrap_or_default()
    }
    /// `RunNIS`: the sequence's camera take from its start; the screen it leaves closes.
    fn run_nis(&mut self, track: u32, screen: Screen) {
        self.track = track;
        self.time = 0.;
        self.settled = false;
        self.screen = screen;
        self.intro_complete = false;
    }
    pub fn update(
        &mut self,
        assets: &Presentation,
        dt: f32,
        input: super::pause_menu::PauseMenuInput,
        focused: bool,
    ) -> Selection {
        if !self.ready || self.leaving {
            return Selection::None;
        }
        if !self.settled {
            self.time += dt;
            if self.time >= assets.duration(self.track) {
                if self.track == BOOT {
                    self.time -= assets.duration(BOOT);
                    self.track = ENTER;
                } else {
                    self.time = assets.duration(self.track);
                    self.settled = true;
                    // UpdateFEState82639648 at NISSequenceComplete: PushState(40) after the
                    // FreeSkate sequence, PushState(22) after the return, whose new
                    // menu_picker starts on row 0.
                    self.screen = match self.screen {
                        Screen::ToFreeSkate => Screen::FreeSkate,
                        Screen::ToPartyPlay => Screen::PartyPlay,
                        Screen::ToOnline=>{
                            if std::mem::take(&mut self.nickname_needed) {
                                self.nickname_prompt=Some(super::profile_name::Prompt::nickname_replacing(&super::online_config::OnlineConfig::load().nickname));
                            }
                            Screen::Online
                        }
                        Screen::FromFreeSkate | Screen::FromPartyPlay | Screen::FromOnline => {
                            self.selected = 0;
                            Screen::MainMenu
                        }
                        screen => screen,
                    };
                }
            }
            return Selection::None;
        }
        if let Some(prompt) = self.name_prompt.as_mut() {
            if !focused || !self.intro_complete { return Selection::None; }
            if input.select { prompt.input(super::extras::TextInput::Accept); }
            return prompt.take_accepted().map_or(Selection::None, Selection::SaveProfileName);
        }
        // PC: the release's changes, once, when the menu first stands idle after an update; the
        // input of that frame reaches nothing.
        if self.update_prompt.is_none()
            && self.idle()
            && let Some(changelog) = self.changelog.take()
        {
            self.update_prompt = Some(super::update_prompt::UpdatePrompt::Changelog(changelog));
            self.sounds.push("FE_SOUND_CORE_POPUP");
            return Selection::None;
        }
        if let Some(prompt) = self.update_prompt.as_mut() {
            use super::update_prompt::UpdatePrompt;
            if !focused { return Selection::None; }
            // sk8popup's menu_picker: Yes and No wrap; B is No.
            match prompt {
                UpdatePrompt::Offer { version, selected, .. } => {
                    if input.up || input.down {
                        *selected = 1 - *selected;
                        self.sounds.push(if input.up { "FE_SOUND_CORE_NAV_UP" } else { "FE_SOUND_CORE_NAV_DOWN" });
                    }
                    if input.select && *selected == 0 {
                        self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                        *prompt = UpdatePrompt::Downloading { version: version.clone(), percent: 0 };
                        return Selection::InstallUpdate;
                    }
                    if input.select || input.cancel {
                        self.sounds.push(if input.select { "FE_SOUND_CORE_A_BUTTON" } else { "FE_SOUND_CORE_B_BUTTON" });
                        self.update_declined = Some(version.clone());
                        self.update_prompt = None;
                    }
                }
                UpdatePrompt::Failed { .. } | UpdatePrompt::Changelog(_) if input.select => {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    self.update_prompt = None;
                }
                UpdatePrompt::Failed { .. } | UpdatePrompt::Downloading { .. } | UpdatePrompt::Installed { .. } | UpdatePrompt::Changelog(_) => {}
            }
            return Selection::None;
        }
        if self.screen == Screen::FreeSkate {
            let Some(freeskate) = self.freeskate.as_mut() else {
                return Selection::None;
            };
            freeskate.tick(dt);
            if !focused {
                return Selection::None;
            }
            return match freeskate.update(input) {
                freeskate_option::Action::None => Selection::None,
                freeskate_option::Action::Back => {
                    self.sounds.extend(freeskate.take_sounds());
                    self.freeskate = None;
                    self.run_nis(FROM_FREESKATE, Screen::FromFreeSkate);
                    Selection::None
                }
                freeskate_option::Action::EnterGame(settings) => Selection::EnterFreeSkate(settings),
            };
        }
        if self.screen == Screen::PartyPlay {
            let Some(party_play) = self.party_play.as_mut() else {
                return Selection::None;
            };
            party_play.tick(dt);
            if !focused {
                return Selection::None;
            }
            // The PC pad is controller 0.
            return match party_play.update(input, 0) {
                party_play::Action::None => Selection::None,
                party_play::Action::Back => {
                    self.sounds.extend(party_play.take_sounds());
                    self.party_play = None;
                    self.run_nis(FROM_PARTY_PLAY, Screen::FromPartyPlay);
                    Selection::None
                }
                party_play::Action::EnterGame(game) => Selection::EnterPartyPlay(game),
            };
        }
        if let Some(extras) = self.extras.as_mut() {
            return Selection::Extras(extras.update(input, focused));
        }
        if self.nickname_prompt.is_some() {
            if !focused || !self.nickname_prompt_open() { return Selection::None; }
            let prompt=self.nickname_prompt.as_mut().expect("nickname prompt");
            if input.select { prompt.input(super::extras::TextInput::Accept); }
            if input.cancel { prompt.input(super::extras::TextInput::Cancel); }
            if prompt.take_cancelled() {
                self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                self.nickname_prompt=None;
                return Selection::None;
            }
            return prompt.take_accepted().map_or(Selection::None, Selection::SaveOnlineNickname);
        }
        if let Some(reel)=&mut self.skate_reel {
            let action=reel.update(input,focused);
            if action==super::skate_reel::Action::Back {
                self.sounds.extend(reel.take_sounds());self.skate_reel=None;self.run_nis(FROM_ONLINE,Screen::FromOnline);
            } else if action==super::skate_reel::Action::EditNickname {
                self.nickname_prompt=Some(super::profile_name::Prompt::nickname_with(&super::online_config::OnlineConfig::load().nickname));
            } else if action!=super::skate_reel::Action::None {reel.pending=action;}
            return Selection::None;
        }
        if let Some(dlc) = self.dlc.as_mut() {
            return match dlc.update(input, focused) {
                super::dlc_menu::Action::None => Selection::None,
                super::dlc_menu::Action::Link => Selection::LinkDlc,
                // OnUpdate 0x8265b990's SetState(22): the main menu again, its picker at row 0.
                super::dlc_menu::Action::Back => {
                    self.sounds.extend(dlc.take_sounds());
                    self.dlc = None;
                    self.selected = 0;
                    self.intro_complete = false;
                    Selection::None
                }
            };
        }
        if !focused || !self.intro_complete {
            return Selection::None;
        }
        // Existing PRES repeat events; menu_picker Prev/Next wraps and sounds the move,
        // and OnMenuSelect sounds A before Game.OnMenuSelect.
        if input.up {
            self.selected = (self.selected + LABELS.len() - 1) % LABELS.len();
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
        }
        if input.down {
            self.selected = (self.selected + 1) % LABELS.len();
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
        }
        if input.select {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
        }
        if !input.select {
            return Selection::None;
        }
        // OnMenuSelectGame82681338: row 0 StartSinglePlayer; rows 2 and 3
        // RunNIS(eFENIS_FromMainMenuToFreeSkate / ToPartyPlay) and PopState.
        match self.selected {
            0 => Selection::ContinueCareer,
            1=>{self.skate_reel=Some(super::skate_reel::Controller::new());self.run_nis(TO_ONLINE,Screen::ToOnline);Selection::Online}
            2 => {
                self.run_nis(TO_FREESKATE, Screen::ToFreeSkate);
                Selection::FreeSkate
            }
            3 => {
                self.run_nis(TO_PARTY_PLAY, Screen::ToPartyPlay);
                Selection::PartyPlay
            }
            4 => Selection::OpenDlc,
            6 => Selection::OpenExtras,
            7 => Selection::ExitToDesktop,
            _ => Selection::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::pause_menu::PauseMenuInput;

    fn settled_menu() -> Controller {
        let mut menu = Controller::new(true);
        menu.ready = true;
        menu.settled = true;
        menu.intro_complete = true;
        menu
    }

    fn press(menu: &mut Controller, input: PauseMenuInput) -> Selection {
        // A settled menu never reads its cameras.
        let assets = Presentation {
            cameras: CameraSet { groups: std::collections::BTreeMap::new(), order: Vec::new() },
            walls: [[([[0.; 4]; 4], 0.); 2]; 4],
            far: 1.,
        };
        menu.update(&assets, 1. / 60., input, true)
    }

    fn up() -> PauseMenuInput {
        PauseMenuInput { up: true, ..Default::default() }
    }

    fn down() -> PauseMenuInput {
        PauseMenuInput { down: true, ..Default::default() }
    }

    fn select() -> PauseMenuInput {
        PauseMenuInput { select: true, ..Default::default() }
    }

    fn cancel() -> PauseMenuInput {
        PauseMenuInput { cancel: true, ..Default::default() }
    }

    #[test]
    fn exit_to_desktop_is_the_last_row_and_the_rows_wrap_through_it() {
        let mut menu = settled_menu();
        for _ in 0..7 {
            press(&mut menu, down());
        }
        assert_eq!((menu.selected, LABELS[menu.selected]), (7, "#Exit to Desktop"));
        press(&mut menu, down());
        assert_eq!(menu.selected, 0);
        press(&mut menu, up());
        assert_eq!(menu.selected, 7);
        assert_eq!(press(&mut menu, select()), Selection::ExitToDesktop);
    }

    #[test]
    fn a_newer_release_is_offered_on_the_idle_main_menu_until_the_player_says_no() {
        use crate::ui::update_prompt::{UpdatePrompt, UpdateStatus};
        let available = |version: &str| UpdateStatus::Available { version: version.into() };
        let mut menu = settled_menu();
        menu.dlc = Some(crate::ui::dlc_menu::Controller::new(Vec::new()));
        menu.set_update_status(&available("0.2.0"), "0.1.0");
        assert_eq!(menu.update_prompt, None, "not over the Downloadable Content screen");
        menu.dlc = None;
        menu.set_update_status(&available("0.2.0"), "0.1.0");
        assert_eq!(menu.update_prompt, Some(UpdatePrompt::Offer { version: "0.2.0".into(), current: "0.1.0".into(), selected: 0 }));
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_POPUP"]);
        assert_eq!(press(&mut menu, down()), Selection::None);
        assert_eq!(menu.selected, 0, "the rows under the popup do not move");
        assert_eq!(press(&mut menu, select()), Selection::None, "No");
        assert_eq!(menu.update_prompt, None);
        menu.set_update_status(&available("0.2.0"), "0.1.0");
        assert_eq!(menu.update_prompt, None, "the release said No to is not offered again");
        menu.set_update_status(&available("0.3.0"), "0.1.0");
        assert!(matches!(menu.update_prompt, Some(UpdatePrompt::Offer { .. })), "a newer one is");
        assert_eq!(press(&mut menu, cancel()), Selection::None, "B is No");
        assert_eq!(menu.update_prompt, None);
    }

    #[test]
    fn yes_downloads_the_update_under_its_progress_and_a_failure_s_ok_closes_it() {
        use crate::ui::update_prompt::{UpdatePrompt, UpdateStatus};
        let mut menu = settled_menu();
        menu.set_update_status(&UpdateStatus::Available { version: "0.2.0".into() }, "0.1.0");
        assert_eq!(press(&mut menu, select()), Selection::InstallUpdate);
        assert_eq!(menu.update_prompt, Some(UpdatePrompt::Downloading { version: "0.2.0".into(), percent: 0 }));
        for input in [select(), cancel(), up()] {
            assert_eq!(press(&mut menu, input), Selection::None, "no input while it downloads");
        }
        menu.set_update_status(&UpdateStatus::Downloading { version: "0.2.0".into(), percent: 40 }, "0.1.0");
        assert_eq!(menu.update_prompt, Some(UpdatePrompt::Downloading { version: "0.2.0".into(), percent: 40 }));
        menu.set_update_status(&UpdateStatus::Failed { message: "Try again.".into() }, "0.1.0");
        assert_eq!(menu.update_prompt, Some(UpdatePrompt::Failed { message: "Try again.".into() }));
        assert_eq!(press(&mut menu, select()), Selection::None);
        assert_eq!(menu.update_prompt, None);
        menu.set_update_status(&UpdateStatus::Available { version: "0.2.0".into() }, "0.1.0");
        assert_eq!(press(&mut menu, select()), Selection::InstallUpdate, "offered again after the failure");
        menu.set_update_status(&UpdateStatus::Installed { version: "0.2.0".into() }, "0.1.0");
        assert_eq!(menu.update_prompt, Some(UpdatePrompt::Installed { version: "0.2.0".into() }));
    }

    #[test]
    fn the_changelog_shows_once_on_the_idle_main_menu_before_an_update_is_offered() {
        use crate::ui::update_prompt::{Changelog, UpdatePrompt, UpdateStatus};
        let changelog = Changelog::parse("0.2.5", "Leave Challenge\n- Lobby ghosts\n");
        let available = UpdateStatus::Available { version: "0.2.6".into() };
        let mut menu = settled_menu();
        menu.changelog = Some(changelog.clone());
        menu.name_prompt = Some(Default::default());
        assert_eq!(press(&mut menu, down()), Selection::None);
        assert_eq!(menu.update_prompt, None, "not over the name prompt");
        menu.name_prompt = None;
        menu.set_update_status(&available, "0.2.5");
        assert_eq!(menu.update_prompt, None, "an offer waits for the changelog");
        let _ = menu.take_sounds();
        assert_eq!(press(&mut menu, down()), Selection::None);
        assert_eq!(menu.update_prompt, Some(UpdatePrompt::Changelog(changelog)));
        assert_eq!((menu.selected, menu.take_sounds()), (0, vec!["FE_SOUND_CORE_POPUP"]), "the opening frame's input reaches nothing");
        for input in [up(), down(), cancel()] {
            assert_eq!(press(&mut menu, input), Selection::None);
        }
        assert!(matches!(menu.update_prompt, Some(UpdatePrompt::Changelog(_))), "only A closes it");
        assert_eq!(press(&mut menu, select()), Selection::None);
        assert_eq!((menu.update_prompt.clone(), menu.changelog.clone()), (None, None), "shown once");
        menu.set_update_status(&available, "0.2.5");
        assert!(matches!(menu.update_prompt, Some(UpdatePrompt::Offer { .. })), "then a newer release is offered");
    }

    #[test]
    fn downloadable_content_opens_its_screen_and_b_returns_to_the_first_row() {
        let mut menu = settled_menu();
        menu.selected = 4;
        assert_eq!(press(&mut menu, select()), Selection::OpenDlc);
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_A_BUTTON"]);
        menu.dlc = Some(crate::ui::dlc_menu::Controller::new(Vec::new()));
        assert_eq!(press(&mut menu, select()), Selection::None, "nothing before the intro");
        menu.dlc.as_mut().unwrap().intro_complete = true;
        assert_eq!(press(&mut menu, select()), Selection::LinkDlc, "the only row is Link DLCs");
        menu.dlc.as_mut().unwrap().picked(false);
        assert_eq!(press(&mut menu, cancel()), Selection::None);
        assert!(menu.dlc.is_none());
        assert_eq!((menu.selected, menu.intro_complete), (0, false));
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_A_BUTTON", "FE_SOUND_CORE_B_BUTTON"]);
    }
}
