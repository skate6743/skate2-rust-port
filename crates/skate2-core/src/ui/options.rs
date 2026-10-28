//! `FrontEndState_Options` (FE state 73, screen 120: `options/options.swf`), reached from the
//! main menu's Options and the StartHub's Settings. Its screens are `gGameOptionsScreens`
//! 0x82fdfa70 rows this port drives: the main list, Audio Settings, the Video Settings variant
//! `OnMenuSelect` (0x826871a0) picks for the front end, a free skate game or any other game, and
//! Control Settings. In game the list leads with the Music Player (FE state 30). PC port addition: a
//! Display Settings page after Control Settings holds the PC rows (PostFX, Fisheye, Frame Rate
//! Cap, VSync, Resolution Scale, Frame Rate Counter, Fullscreen), saved in their own chunk, and
//! an Online Settings page holds Show Lobby Chat, saved in `online-config.json`. Beside them the
//! PC port calls Video Settings (`ID_GAMESETTINGS_VIDEO_SETTINGS`) Gameplay Settings, for its
//! camera, HUD and subtitle rows. PC port addition: Control Settings' Stick Settings page leads to
//! each stick's multiplier, deadzone and anti-deadzone ([`crate::stick_settings`]), saved in
//! `controller_config.json`.
use super::{FeSoundRequest, pause_menu::PauseMenuInput};
use crate::stick_settings::{ControllerSettings, DEADZONES, MULTIPLIERS, StickSettings};
pub use skate2_formats::skater_profile::{
    FrameRateCap, PcDisplaySettings, ProfileSettings, RESOLUTION_SCALES, RESOLUTION_SCALE_STEP,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Hidden,
    Options,
    Audio,
    Video,
    /// `eSk8FEGameOptionsScreen_ControlSettings`.
    Control,
    /// PC port addition.
    Display,
    /// PC port addition.
    Online,
    /// PC port addition: Control Settings' Stick Settings.
    StickSettings,
    /// PC port addition: Stick Settings' page of each stick.
    LeftStick,
    RightStick,
}

/// The Video Settings screen `OnMenuSelect` (`0x826871a0`) opens: `VideoSettingsMainMenu` in
/// the front end, `VideoSettingsFreeskate` in a free skate game (`GameBackEnd::IsFreeSkateMode`),
/// `VideoSettings` in any other. The online variants need the rebroadcaster.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VideoScreen {
    #[default]
    MainMenu,
    Game,
    FreeSkate,
}
/// A row: an `eSk8FEGameOptions` entry or one of the PC display toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    MusicPlayer,
    AudioSettings,
    VideoSettings,
    /// `eSk8FEGameOption_ControlSettings`: the Control Settings page.
    ControlSettings,
    /// PC port addition: the Display Settings page.
    DisplaySettings,
    /// PC port addition: the Online Settings page.
    OnlineSettings,
    SfxVolume,
    DialogueVolume,
    MusicVolume,
    PostFx,
    Fisheye,
    FrameRateCap,
    /// PC port addition: frames wait for the display's refresh.
    VSync,
    /// PC port addition: the 3D scene's size in percent of the window.
    ResolutionScale,
    FpsCounter,
    Fullscreen,
    /// `eSk8FEGameOption_SubtitlesToggle`: the profile's `mSubtitles`.
    Subtitles,
    /// `eSk8FEGameOption_CameraAngleToggle`: `mCameraAngle`, low or high.
    CameraAngle,
    /// `eSk8FEGameOption_GrindTransparencyToggle`: `mGrindTransparency`, the camera's grind
    /// detector.
    GrindTransparency,
    /// `eSk8FEGameOption_MiniMapToggle`: `mMiniMap`.
    MiniMap,
    /// `eSk8FEGameOption_HUDToggle`: `mHUD`.
    Hud,
    /// `eSk8FEGameOption_OffboardYAxisToggle`: `mInvertYAxis`, the off-board camera's Y axis.
    OffboardYAxis,
    /// `eSk8FEGameOption_VibrationToggle`: `mRumble`.
    Vibration,
    /// PC port addition: `online-config.json`'s `show_lobby_chat`.
    ShowLobbyChat,
    /// PC port addition: `online-config.json`'s `show_ghost_players`.
    ShowGhostPlayers,
    /// PC port addition: Control Settings' Stick Settings page.
    StickSettings,
    /// PC port addition: Stick Settings' page of each stick, and its reset of both to vanilla.
    LeftStick,
    RightStick,
    ResetSticks,
    /// PC port addition: a stick page's multiplier, deadzone and anti-deadzone.
    StickMultiplier,
    StickDeadzone,
    StickAntiDeadzone,
}
impl Row {
    /// `gGameOptionsItems` 0x82fdf9a8's type: `option`, `slider` or `selector`; the stick
    /// deadzones take Edit Skater's `morph` knob (PC port addition).
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::MusicPlayer | Self::AudioSettings | Self::VideoSettings | Self::ControlSettings | Self::DisplaySettings
            | Self::OnlineSettings | Self::StickSettings | Self::LeftStick | Self::RightStick | Self::ResetSticks => "option",
            Self::SfxVolume | Self::DialogueVolume | Self::MusicVolume => "slider",
            Self::StickDeadzone | Self::StickAntiDeadzone => "morph",
            Self::PostFx | Self::Fisheye | Self::FrameRateCap | Self::VSync | Self::ResolutionScale
            | Self::FpsCounter | Self::Fullscreen | Self::Subtitles | Self::CameraAngle | Self::GrindTransparency
            | Self::MiniMap | Self::Hud | Self::OffboardYAxis | Self::Vibration | Self::ShowLobbyChat
            | Self::ShowGhostPlayers | Self::StickMultiplier => "selector",
        }
    }
    /// `gGameOptionsItems`' title id; the PC rows carry literal text.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::MusicPlayer => "ID_GAMESETTINGS_MUSICPLAYER",
            Self::AudioSettings => "ID_GAMESETTINGS_AUDIO_SETTINGS",
            Self::VideoSettings => "Gameplay Settings",
            Self::ControlSettings => "ID_GAMESETTINGS_CONTROL_SETTINGS",
            Self::DisplaySettings => "Display Settings",
            Self::OnlineSettings => "Online Settings",
            Self::SfxVolume => "ID_GAMESETTINGS_SFXVOLUME",
            Self::DialogueVolume => "ID_GAMESETTINGS_DIALOGVOLUME",
            Self::MusicVolume => "ID_GAMESETTINGS_MUSICVOLUME",
            Self::PostFx => "PostFX",
            Self::Fisheye => "Fisheye",
            Self::FrameRateCap => "Frame Rate Cap",
            Self::VSync => "VSync",
            Self::ResolutionScale => "Resolution Scale",
            Self::FpsCounter => "Frame Rate Counter",
            Self::Fullscreen => "Fullscreen",
            Self::Subtitles => "ID_GAMESETTINGS_SUBTITLE_TOGGLE",
            Self::CameraAngle => "ID_GAMESETTINGS_CAMERA_ANGLE_TOGGLE",
            Self::GrindTransparency => "ID_GAMESETTINGS_TRANSPARENCY_TOGGLE",
            Self::MiniMap => "ID_GAMESETTINGS_MINIMAP_TOGGLE",
            Self::Hud => "ID_GAMESETTINGS_HUD_TOGGLE",
            Self::OffboardYAxis => "ID_GAMESETTINGS_OFFBOARD_Y_AXIS_TOGGLE",
            Self::Vibration => "ID_GAMESETTINGS_VIBRATION_TOGGLE",
            Self::ShowLobbyChat => "Show Lobby Chat",
            Self::ShowGhostPlayers => "Show Ghost Players in pregame lobby",
            Self::StickSettings => "Stick Settings",
            Self::LeftStick => "Left Stick",
            Self::RightStick => "Right Stick",
            Self::ResetSticks => "Reset To Default",
            Self::StickMultiplier => "Stick Multiplier",
            Self::StickDeadzone => "Stick Deadzone",
            Self::StickAntiDeadzone => "Stick Anti-Deadzone",
        }
    }
}
/// `gGameOptionsScreens` rows 0 and 1 in their order, with the PC Display Settings page after
/// Control Settings and the PC Online Settings page last; row 0's
/// `eSk8FEGameOption_DeviceSelection` (a storage device) is left out.
const FRONT_END: &[Row] =
    &[Row::AudioSettings, Row::VideoSettings, Row::ControlSettings, Row::DisplaySettings, Row::OnlineSettings];
const IN_GAME: &[Row] = &[
    Row::MusicPlayer,
    Row::AudioSettings,
    Row::VideoSettings,
    Row::ControlSettings,
    Row::DisplaySettings,
    Row::OnlineSettings,
];
const AUDIO: &[Row] = &[Row::SfxVolume, Row::DialogueVolume, Row::MusicVolume];
/// `gGameOptionsScreens` rows 7, 3 and 5 without `eSk8FEGameOption_VideoCalibration`, whose
/// FE state 0 calibration screen is not ported.
const VIDEO_MAIN_MENU: &[Row] = &[Row::Subtitles];
const VIDEO_GAME: &[Row] = &[Row::CameraAngle, Row::GrindTransparency, Row::Subtitles];
const VIDEO_FREESKATE: &[Row] = &[Row::CameraAngle, Row::GrindTransparency, Row::Subtitles, Row::MiniMap, Row::Hud];
/// `gGameOptionsScreens` row 8, with the PC Stick Settings page last.
const CONTROL: &[Row] = &[Row::OffboardYAxis, Row::Vibration, Row::StickSettings];
const STICK_SETTINGS: &[Row] = &[Row::LeftStick, Row::RightStick, Row::ResetSticks];
const STICK: &[Row] = &[Row::StickMultiplier, Row::StickDeadzone, Row::StickAntiDeadzone];
/// `FE_SOUND_CAS_SLIDER`, which Edit Skater loops while a morph knob moves (`OnUpdate` 0x82670400).
const SLIDER_SOUND: &str = "FE_SOUND_CAS_SLIDER";
const DISPLAY: &[Row] = &[
    Row::PostFx,
    Row::Fisheye,
    Row::FrameRateCap,
    Row::VSync,
    Row::ResolutionScale,
    Row::FpsCounter,
    Row::Fullscreen,
];
const ONLINE: &[Row] = &[Row::ShowLobbyChat, Row::ShowGhostPlayers];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub page: Page,
    pub selected: usize,
    /// Opened outside the front end (`mFEState != eFE_STATE_IN_FE`).
    pub in_game: bool,
    pub values: PcDisplaySettings,
    /// `GetIntegerValue` 0x82643b10 of SFX, dialogue and music volume.
    pub volumes: [i32; 3],
    /// `GetStringValue` 0x82643d48 of Subtitles: `ID_COMMON_ON` with `mSubtitles` set.
    pub subtitles: bool,
    /// `GetStringValue` of the camera angle (`ID_GAMESETTINGS_CAMERA_HIGH` for 1, else
    /// `_LOW`), Grind Transparency, MiniMap and HUD.
    pub camera_high: bool,
    pub grind_transparency: bool,
    pub mini_map: bool,
    pub hud: bool,
    /// `GetStringValue` of the Off-board Y Axis row: `ID_GAMESETTINGS_AXIS_NORMAL` with
    /// `mInvertYAxis` set, else `ID_GAMESETTINGS_AXIS_INVERTED`.
    pub invert_y: bool,
    /// `GetStringValue` of Vibration: `ID_COMMON_ON` with `mRumble` set.
    pub vibration: bool,
    /// Show Lobby Chat (PC port addition).
    pub show_lobby_chat: bool,
    /// Show Ghost Players in pregame lobby (PC port addition).
    pub show_ghost_players: bool,
    /// The stick settings as the rows have them, which the pads follow at once (PC port
    /// addition).
    pub sticks: ControllerSettings,
    /// The Video Settings screen this opening picks.
    pub video: VideoScreen,
    /// `FrontEndState_MusicPlayer` (FE state 30) is pushed over Options: `OnMenuSelect`'s
    /// `PushState(30)` left the screen (`OnLeave`) and the pop opens it anew (`OnEnter`).
    pub music_player: bool,
}
impl Snapshot {
    #[must_use]
    pub const fn rows(&self) -> &'static [Row] {
        match self.page {
            Page::Hidden => &[],
            Page::Options if self.in_game => IN_GAME,
            Page::Options => FRONT_END,
            Page::Audio => AUDIO,
            Page::Video => match self.video {
                VideoScreen::MainMenu => VIDEO_MAIN_MENU,
                VideoScreen::Game => VIDEO_GAME,
                VideoScreen::FreeSkate => VIDEO_FREESKATE,
            },
            Page::Control => CONTROL,
            Page::Display => DISPLAY,
            Page::Online => ONLINE,
            Page::StickSettings => STICK_SETTINGS,
            Page::LeftStick | Page::RightStick => STICK,
        }
    }
    /// The Stick Settings pages, which show both sticks live and take the D-pad alone.
    #[must_use]
    pub const fn stick_page(&self) -> bool {
        matches!(self.page, Page::StickSettings | Page::LeftStick | Page::RightStick)
    }
    /// The settings a stick's page edits.
    #[must_use]
    pub const fn stick(&self) -> Option<StickSettings> {
        match self.page {
            Page::LeftStick => Some(self.sticks.left),
            Page::RightStick => Some(self.sticks.right),
            _ => None,
        }
    }
    /// `gGameOptionsScreenTitle` 0x82fdf97c.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self.page {
            Page::Audio => "ID_GAMESETTINGS_AUDIO_SETTINGS",
            Page::Video => "Gameplay Settings",
            Page::Control => "ID_GAMESETTINGS_CONTROL_SETTINGS",
            Page::Display => "Display Settings",
            Page::Online => "Online Settings",
            Page::StickSettings => "Stick Settings",
            Page::LeftStick => "Left Stick",
            Page::RightStick => "Right Stick",
            Page::Hidden | Page::Options => "ID_GAMESETTINGS_TITLE",
        }
    }
}

/// What `update` asks of the owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// `RequestExit` 0x82681700: save when dirty, then close.
    Exit,
    /// `OnMenuSelect` 0x826871a0's `PushState(30)`.
    MusicPlayer,
}

#[derive(Default)]
pub struct Controller {
    state: Snapshot,
    saved: PcDisplaySettings,
    /// Show Lobby Chat as `online-config.json` last held it.
    saved_lobby_chat: bool,
    /// Show Ghost Players in pregame lobby as `online-config.json` last held it.
    saved_ghost_players: bool,
    /// `mIsDirty`: a profile setting changed since `OnEnter`.
    dirty: bool,
    /// `FE_SOUND_*` globals this input played (options.apt and `FrontEndState_Options`).
    sounds: Vec<&'static str>,
    /// `IsScreenReady` 0x82d661a8: from the screen's `IntroComplete` (`OnScreenReady`
    /// 0x82d65f98) `StateManagerFrontEnd::ReceiveInput` 0x82656af8 hands the state its input.
    ready: bool,
    /// `ToggleVibration` turned vibration on: the controller's `Vibration` is enabled and plays
    /// its test rumble.
    vibration_test: bool,
    /// The stick settings as `controller_config.json` last held them (PC port addition).
    saved_sticks: ControllerSettings,
    /// The D-pad direction a stick page's value follows, and the frames since its press.
    held: Option<(i8, u32)>,
    /// `FE_SOUND_CAS_SLIDER` loops while a deadzone's knob moves.
    slider: bool,
    /// The slider sound's loop and stop since the last take.
    sound_requests: Vec<FeSoundRequest>,
}
/// `GetIntegerValue`: `floor(volume * 10 + 0.5)`.
fn integer(volume: f32) -> i32 {
    (volume * 10.0 + 0.5).floor() as i32
}
/// Unlimited, 60, 120, Refresh Rate, wrapping past either end.
fn step_frame_rate_cap(value: FrameRateCap, forward: bool) -> FrameRateCap {
    const ORDER: [FrameRateCap; 4] =
        [FrameRateCap::Unlimited, FrameRateCap::Fps60, FrameRateCap::Fps120, FrameRateCap::RefreshRate];
    let index = ORDER.iter().position(|&v| v == value).unwrap_or(3);
    ORDER[if forward { (index + 1) % 4 } else { (index + 3) % 4 }]
}
/// PC port addition: whether a Stick Multiplier held for `frames` frames since its press steps
/// now: at once, then from 24 frames (the front end's first repeat) every 4, from a second every
/// 2, and from two seconds every frame.
const fn repeat_due(frames: u32) -> bool {
    match frames {
        0 => true,
        1..24 => false,
        24..60 => (frames - 24) % 4 == 0,
        60..120 => frames % 2 == 0,
        _ => true,
    }
}
/// Whether a deadzone's knob held for `frames` frames steps now: at once, then from 12 frames every
/// 3, a steady 20 percent a second, as Edit Skater's morph knobs move at a steady pace.
const fn slide_due(frames: u32) -> bool {
    frames == 0 || (frames >= 12 && (frames - 12) % 3 == 0)
}
/// One step up or down, stopping at either end of `RESOLUTION_SCALES`.
fn step_resolution_scale(value: u8, forward: bool) -> u8 {
    let next = if forward { value.saturating_add(RESOLUTION_SCALE_STEP) } else { value.saturating_sub(RESOLUTION_SCALE_STEP) };
    next.clamp(*RESOLUTION_SCALES.start(), *RESOLUTION_SCALES.end())
}
impl Controller {
    pub fn load(&mut self, values: PcDisplaySettings) {
        self.state.values = values;
        self.saved = values;
    }
    /// `online-config.json`'s Show Lobby Chat.
    pub fn load_lobby_chat(&mut self, show: bool) {
        self.state.show_lobby_chat = show;
        self.saved_lobby_chat = show;
    }
    /// Show Lobby Chat as the row has it, which the game follows at once.
    #[must_use]
    pub const fn show_lobby_chat(&self) -> bool {
        self.state.show_lobby_chat
    }
    /// `controller_config.json`'s stick settings.
    pub fn load_sticks(&mut self, sticks: ControllerSettings) {
        self.state.sticks = sticks;
        self.saved_sticks = sticks;
    }
    /// The stick settings as the rows have them, which the pads follow at once.
    #[must_use]
    pub const fn sticks(&self) -> ControllerSettings {
        self.state.sticks
    }
    /// Whether exiting saves the stick settings to `controller_config.json`.
    #[must_use]
    pub fn sticks_dirty(&self) -> bool {
        self.state.sticks != self.saved_sticks
    }
    /// The deadzone knob's looping `FE_SOUND_CAS_SLIDER` and its stop since the last call.
    pub fn take_sound_requests(&mut self) -> Vec<FeSoundRequest> {
        std::mem::take(&mut self.sound_requests)
    }
    /// Whether exiting saves Show Lobby Chat to `online-config.json`.
    #[must_use]
    pub const fn lobby_chat_dirty(&self) -> bool {
        self.state.show_lobby_chat != self.saved_lobby_chat
    }
    /// `online-config.json`'s Show Ghost Players in pregame lobby.
    pub fn load_ghost_players(&mut self, show: bool) {
        self.state.show_ghost_players = show;
        self.saved_ghost_players = show;
    }
    /// Show Ghost Players in pregame lobby as the row has it, which the game follows at once.
    #[must_use]
    pub const fn show_ghost_players(&self) -> bool {
        self.state.show_ghost_players
    }
    /// Whether exiting saves Show Ghost Players in pregame lobby to `online-config.json`.
    #[must_use]
    pub const fn ghost_players_dirty(&self) -> bool {
        self.state.show_ghost_players != self.saved_ghost_players
    }
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.state
    }
    #[must_use]
    pub fn visible(&self) -> bool {
        self.state.page != Page::Hidden
    }
    /// Whether exiting saves the PC display chunk.
    #[must_use]
    pub fn display_dirty(&self) -> bool {
        self.state.values != self.saved
    }
    /// Alt+Enter: Fullscreen flips as its row would, wherever the game is.
    pub fn toggle_fullscreen(&mut self) {
        self.state.values.fullscreen = !self.state.values.fullscreen;
    }
    /// The display values were saved outside the screen's exit.
    pub fn display_saved(&mut self) {
        self.saved = self.state.values;
    }
    /// `mIsDirty`; `RequestExit` also folds in the music player's `mbIsDirty`.
    #[must_use]
    pub const fn dirty(&self) -> bool {
        self.dirty
    }
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }
    /// Whether Vibration was turned on since the last call.
    pub fn take_vibration_test(&mut self) -> bool {
        std::mem::take(&mut self.vibration_test)
    }
    /// `OnEnter` 0x8265c468: `mIsDirty` cleared, then the main list (`LoadMainMenu`) once the
    /// screen it opens is ready.
    pub fn open(&mut self, in_game: bool, video: VideoScreen, settings: &ProfileSettings) {
        self.dirty = false;
        self.ready = false;
        self.state.music_player = false;
        self.state.in_game = in_game;
        self.state.video = video;
        self.state.page = Page::Options;
        self.state.selected = 0;
        self.held = None;
        self.refresh(settings);
    }
    pub fn close_after_save(&mut self) {
        self.saved = self.state.values;
        self.saved_lobby_chat = self.state.show_lobby_chat;
        self.saved_ghost_players = self.state.show_ghost_players;
        self.saved_sticks = self.state.sticks;
        self.dirty = false;
        self.state.page = Page::Hidden;
        self.state.music_player = false;
        self.stop_slider();
    }
    /// `OnLeave` 0x8266af70 under the pushed music player: the screen closes without saving.
    pub fn suspend(&mut self) {
        self.state.page = Page::Hidden;
        self.state.music_player = true;
        self.stop_slider();
    }
    /// `OnLeave` 0x8266af70 as `PopAllStates` pops the state (or the music player over it):
    /// the screen closes and nothing saves. The settings keep what the rows changed, and
    /// `mIsDirty` stays until the next `OnEnter` clears it.
    pub fn leave(&mut self) {
        self.state.page = Page::Hidden;
        self.state.music_player = false;
        self.ready = false;
        self.stop_slider();
    }
    /// The deadzone knob stopped: let go, at an end of its range, or its page left.
    fn stop_slider(&mut self) {
        self.held = None;
        if std::mem::take(&mut self.slider) {
            self.sound_requests.push(FeSoundRequest::Stop(SLIDER_SOUND));
        }
    }
    /// The screen's `IntroComplete`: `OnScreenReady` makes it ready for input.
    pub fn acknowledge_intro(&mut self) {
        if self.visible() {
            self.ready = true;
        }
    }
    fn refresh(&mut self, settings: &ProfileSettings) {
        self.state.volumes =
            [integer(settings.sfx_volume), integer(settings.dialogue_volume), integer(settings.music_volume)];
        self.state.subtitles = settings.subtitles;
        self.state.camera_high = settings.camera_angle == 1;
        self.state.grind_transparency = settings.grind_transparency;
        self.state.mini_map = settings.mini_map;
        self.state.hud = settings.hud;
        self.state.invert_y = settings.invert_y;
        self.state.vibration = settings.rumble;
    }
    pub fn update(&mut self, input: PauseMenuInput, settings: &mut ProfileSettings) -> Action {
        if !self.visible() || !self.ready {
            return Action::None;
        }
        if input.start {
            self.stop_slider();
            return Action::Exit;
        }
        if input.cancel {
            // ReceiveInput 0x82686978: core_b_button, then RequestExit or, from a category,
            // LoadMainMenu. PC port: a Stick Settings page goes back one page, to the row that
            // opened it.
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            self.stop_slider();
            let (page, selected) = match self.state.page {
                Page::Options => return Action::Exit,
                Page::StickSettings => (Page::Control, CONTROL.iter().position(|&row| row == Row::StickSettings).unwrap_or(0)),
                Page::LeftStick => (Page::StickSettings, 0),
                Page::RightStick => (Page::StickSettings, 1),
                _ => (Page::Options, 0),
            };
            self.state.page = page;
            self.state.selected = selected;
            return Action::None;
        }
        if self.state.stick_page() {
            self.update_sticks(input);
            return Action::None;
        }
        // options.apt UpdateInput sounds AptUp/AptDown before Prev/NextValue, whose
        // Prev/NextValueCalc wrap past either end.
        let count = self.state.rows().len().max(1);
        if input.up {
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
            self.state.selected = (self.state.selected + count - 1) % count;
        }
        if input.down {
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
            self.state.selected = (self.state.selected + 1) % count;
        }
        let Some(&row) = self.state.rows().get(self.state.selected) else { return Action::None };
        if input.select && row.kind() == "option" {
            // OnMenuSelect: core_a_button, then the category's screen.
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            match row {
                Row::MusicPlayer => return Action::MusicPlayer,
                Row::AudioSettings => self.state.page = Page::Audio,
                Row::ControlSettings => self.state.page = Page::Control,
                Row::StickSettings => self.state.page = Page::StickSettings,
                Row::DisplaySettings => self.state.page = Page::Display,
                Row::OnlineSettings => self.state.page = Page::Online,
                _ => self.state.page = Page::Video,
            }
            self.state.selected = 0;
            return Action::None;
        }
        if input.left == input.right {
            return Action::None;
        }
        let step = if input.right { 1.0 } else { -1.0 };
        let volume = match row {
            Row::SfxVolume => {
                self.sounds.push(if input.right { "FE_SOUND_CORE_SFX_VOLUME_RIGHT" } else { "FE_SOUND_CORE_SFX_VOLUME_LEFT" });
                &mut settings.sfx_volume
            }
            Row::DialogueVolume => {
                self.sounds.push(if input.right { "FE_SOUND_CORE_SPEECH_VOLUME_RIGHT" } else { "FE_SOUND_CORE_SPEECH_VOLUME_LEFT" });
                &mut settings.dialogue_volume
            }
            Row::MusicVolume => {
                self.sounds.push(if input.right { "FE_SOUND_CORE_MUSIC_VOLUME_RIGHT" } else { "FE_SOUND_CORE_MUSIC_VOLUME_LEFT" });
                &mut settings.music_volume
            }
            Row::PostFx | Row::Fisheye | Row::FrameRateCap | Row::VSync | Row::ResolutionScale | Row::FpsCounter
            | Row::Fullscreen | Row::ShowLobbyChat | Row::ShowGhostPlayers => {
                // The PC selectors sound core_nav_left/core_nav_right by key and, as
                // Prev/NextValueCalc do, wrap past either end; Resolution Scale stops at its ends.
                self.sounds.push(if input.right { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                if row == Row::ShowLobbyChat {
                    self.state.show_lobby_chat = !self.state.show_lobby_chat;
                    return Action::None;
                }
                if row == Row::ShowGhostPlayers {
                    self.state.show_ghost_players = !self.state.show_ghost_players;
                    return Action::None;
                }
                let values = &mut self.state.values;
                match row {
                    Row::PostFx => values.postfx = !values.postfx,
                    Row::Fisheye => values.fisheye = !values.fisheye,
                    Row::FpsCounter => values.fps_counter = !values.fps_counter,
                    Row::Fullscreen => values.fullscreen = !values.fullscreen,
                    Row::VSync => values.vsync = !values.vsync,
                    Row::ResolutionScale => values.resolution_scale = step_resolution_scale(values.resolution_scale, input.right),
                    _ => values.frame_rate_cap = step_frame_rate_cap(values.frame_rate_cap, input.right),
                }
                return Action::None;
            }
            Row::Subtitles => {
                // ReceiveInput 0x82686978 sounds core_nav_left or core_nav_right by key, then
                // ToggleSubtitles 0x8265c5e8 flips mSubtitles either way and marks the profile
                // dirty; its SetScreenState(50) refreshes the row.
                self.sounds.push(if input.right { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                settings.subtitles = !settings.subtitles;
                self.dirty = true;
                self.refresh(settings);
                return Action::None;
            }
            Row::CameraAngle | Row::GrindTransparency | Row::MiniMap | Row::Hud => {
                // ReceiveInput's other toggles: the same sounds, then ToggleCameraAngle
                // 0x8265c7d8 (a set angle to low, else high), ToggleGrindTransparency 0x8265c8b8,
                // ToggleMiniMap 0x8265c6b0 or ToggleHUD 0x8265c740 (whose
                // HudManagerSetVisibleOverride applies at once), each marking the profile dirty.
                self.sounds.push(if input.right { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                match row {
                    Row::CameraAngle => settings.camera_angle = i32::from(settings.camera_angle == 0),
                    Row::GrindTransparency => settings.grind_transparency = !settings.grind_transparency,
                    Row::MiniMap => settings.mini_map = !settings.mini_map,
                    _ => settings.hud = !settings.hud,
                }
                self.dirty = true;
                self.refresh(settings);
                return Action::None;
            }
            Row::OffboardYAxis | Row::Vibration => {
                // ReceiveInput's Control Settings toggles: the same sounds, then ToggleInvertYAxis
                // 0x8265c948, or ToggleVibration 0x8265ca68, which on turning vibration on enables
                // the controller's Vibration and plays it a test rumble. Each marks the profile dirty.
                self.sounds.push(if input.right { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                if row == Row::OffboardYAxis {
                    settings.invert_y = !settings.invert_y;
                } else {
                    settings.rumble = !settings.rumble;
                    self.vibration_test = settings.rumble;
                }
                self.dirty = true;
                self.refresh(settings);
                return Action::None;
            }
            _ => return Action::None,
        };
        // The volume clamp, then RefreshCurrentItem 0x8265c5a0 marks the profile dirty.
        let next = step * 0.1 + *volume;
        *volume = if next < 0.0 { 0.0 } else if next > 1.0 { 1.0 } else { next };
        self.dirty = true;
        self.refresh(settings);
        Action::None
    }
    /// PC port addition: a Stick Settings page's input, from the D-pad alone, as the sticks are
    /// what its boxes show. Up and down pick a row, A opens a stick's page or puts both sticks back
    /// to vanilla, and left or right held moves the row's value: the multiplier a hundredth at a
    /// time, sooner the longer it is held, and a deadzone's knob at Edit Skater's steady pace,
    /// looping `FE_SOUND_CAS_SLIDER` while it moves.
    fn update_sticks(&mut self, input: PauseMenuInput) {
        let count = self.state.rows().len().max(1);
        if input.dpad_up || input.dpad_down {
            self.sounds.push(if input.dpad_up { "FE_SOUND_CORE_NAV_UP" } else { "FE_SOUND_CORE_NAV_DOWN" });
            self.state.selected = if input.dpad_up { (self.state.selected + count - 1) % count } else { (self.state.selected + 1) % count };
            self.stop_slider();
        }
        let Some(&row) = self.state.rows().get(self.state.selected) else { return };
        if input.select && row.kind() == "option" {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            match row {
                Row::LeftStick => self.state.page = Page::LeftStick,
                Row::RightStick => self.state.page = Page::RightStick,
                _ => self.state.sticks = ControllerSettings::VANILLA,
            }
            if row != Row::ResetSticks {
                self.state.selected = 0;
            }
            self.stop_slider();
            return;
        }
        let direction: i8 = if input.morph_axis_secondary > 0.5 {
            1
        } else if input.morph_axis_secondary < -0.5 {
            -1
        } else {
            0
        };
        if direction == 0 {
            self.stop_slider();
            return;
        }
        let frames = match self.held {
            Some((held, frames)) if held == direction => frames + 1,
            _ => 0,
        };
        self.held = Some((direction, frames));
        match row {
            Row::StickMultiplier => {
                if repeat_due(frames) && self.step(row, direction) && frames % 4 == 0 {
                    self.sounds.push(if direction > 0 { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                }
            }
            Row::StickDeadzone | Row::StickAntiDeadzone if slide_due(frames) => {
                if !self.step(row, direction) {
                    self.stop_slider();
                    self.held = Some((direction, frames));
                } else if !self.slider {
                    self.slider = true;
                    self.sound_requests.push(FeSoundRequest::Loop(SLIDER_SOUND));
                }
            }
            _ => {}
        }
    }
    /// One step of a stick page's row; whether its value changed.
    fn step(&mut self, row: Row, direction: i8) -> bool {
        let stick = match self.state.page {
            Page::LeftStick => &mut self.state.sticks.left,
            Page::RightStick => &mut self.state.sticks.right,
            _ => return false,
        };
        let (value, range) = match row {
            Row::StickMultiplier => (&mut stick.multiplier, MULTIPLIERS),
            Row::StickDeadzone => (&mut stick.deadzone, DEADZONES),
            Row::StickAntiDeadzone => (&mut stick.anti_deadzone, DEADZONES),
            _ => return false,
        };
        let next = value.saturating_add_signed(direction).clamp(*range.start(), *range.end());
        let changed = next != *value;
        *value = next;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn press(f: impl FnOnce(&mut PauseMenuInput)) -> PauseMenuInput {
        let mut input = PauseMenuInput::default();
        f(&mut input);
        input
    }
    #[test]
    fn independent_toggles_and_back_stack_save_boundary() {
        let mut settings = ProfileSettings::default();
        let mut menu = Controller::default();
        menu.open(false, VideoScreen::MainMenu, &settings);
        assert_eq!(menu.snapshot().rows(), FRONT_END);
        // Input waits for the screen's intro.
        menu.update(press(|i| i.down = true), &mut settings);
        assert_eq!(menu.snapshot().selected, 0);
        menu.acknowledge_intro();
        for _ in 0..3 {
            menu.update(press(|i| i.down = true), &mut settings);
        }
        assert_eq!(menu.update(press(|i| i.select = true), &mut settings), Action::None);
        assert_eq!(menu.snapshot().page, Page::Display);
        menu.update(press(|i| i.left = true), &mut settings);
        assert_eq!(menu.snapshot().values, PcDisplaySettings { postfx: false, ..PcDisplaySettings::default() });
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.right = true), &mut settings);
        assert_eq!(
            menu.snapshot().values,
            PcDisplaySettings { postfx: false, fisheye: false, frame_rate_cap: FrameRateCap::RefreshRate, fps_counter: false, fullscreen: false,
                resolution_scale: 100, vsync: true }
        );
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.left = true), &mut settings);
        menu.update(press(|i| i.left = true), &mut settings);
        assert_eq!(menu.snapshot().values.frame_rate_cap, FrameRateCap::Fps60);
        menu.update(press(|i| i.right = true), &mut settings);
        assert_eq!(menu.snapshot().values.frame_rate_cap, FrameRateCap::Fps120);
        menu.update(press(|i| i.right = true), &mut settings);
        menu.update(press(|i| i.right = true), &mut settings);
        assert_eq!(menu.snapshot().values.frame_rate_cap, FrameRateCap::Unlimited);
        menu.update(press(|i| i.left = true), &mut settings);
        assert_eq!(menu.snapshot().values.frame_rate_cap, FrameRateCap::RefreshRate);
        // VSync toggles on its own, whatever the cap.
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.left = true), &mut settings);
        assert!(!menu.snapshot().values.vsync);
        assert_eq!(menu.snapshot().values.frame_rate_cap, FrameRateCap::RefreshRate);
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(menu.snapshot().values.vsync);
        // Resolution Scale steps by 10 and stops at 50 and 200.
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.right = true), &mut settings);
        assert_eq!(menu.snapshot().values.resolution_scale, 110);
        for _ in 0..10 {
            menu.update(press(|i| i.left = true), &mut settings);
        }
        assert_eq!(menu.snapshot().values.resolution_scale, 50);
        for _ in 0..20 {
            menu.update(press(|i| i.right = true), &mut settings);
        }
        assert_eq!(menu.snapshot().values.resolution_scale, 200);
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(menu.snapshot().values.fps_counter);
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(menu.snapshot().values.fullscreen);
        // Alt+Enter flips it back, and a save made elsewhere leaves nothing for the exit.
        menu.toggle_fullscreen();
        assert!(!menu.snapshot().values.fullscreen);
        menu.display_saved();
        assert!(!menu.display_dirty());
        menu.update(press(|i| i.right = true), &mut settings);
        let back = press(|i| i.cancel = true);
        assert_eq!(menu.update(back, &mut settings), Action::None);
        assert_eq!(menu.update(back, &mut settings), Action::Exit);
        assert!(menu.visible() && menu.display_dirty()); // failed IO must not acknowledge a save
        menu.close_after_save();
        assert!(!menu.visible() && !menu.display_dirty());
    }
    #[test]
    fn audio_sliders_step_clamp_and_dirty() {
        let mut settings = ProfileSettings::default();
        let mut menu = Controller::default();
        menu.open(true, VideoScreen::Game, &settings);
        menu.acknowledge_intro();
        assert_eq!(menu.snapshot().rows(), IN_GAME);
        assert_eq!(menu.update(press(|i| i.select = true), &mut settings), Action::MusicPlayer);
        // The pushed music player leaves the screen; its pop opens it anew, not yet ready.
        menu.suspend();
        assert!(!menu.visible() && menu.snapshot().music_player);
        menu.open(true, VideoScreen::Game, &settings);
        assert!(!menu.snapshot().music_player);
        assert_eq!(menu.update(press(|i| i.down = true), &mut settings), Action::None);
        assert_eq!(menu.snapshot().selected, 0);
        menu.acknowledge_intro();
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.snapshot().page, Page::Audio);
        assert_eq!(menu.snapshot().volumes, [10, 10, 10]);
        menu.update(press(|i| i.right = true), &mut settings);
        assert_eq!(settings.sfx_volume, 1.0);
        assert!(menu.dirty());
        menu.update(press(|i| i.down = true), &mut settings);
        for _ in 0..3 {
            menu.update(press(|i| i.left = true), &mut settings);
        }
        assert_eq!(menu.snapshot().volumes, [10, 7, 10]);
        assert_eq!(menu.take_sounds().last(), Some(&"FE_SOUND_CORE_SPEECH_VOLUME_LEFT"));
    }
    #[test]
    fn subtitles_close_video_settings_and_toggle_the_profile_setting() {
        let mut settings = ProfileSettings::default();
        assert!(!settings.subtitles);
        let mut menu = Controller::default();
        menu.open(false, VideoScreen::MainMenu, &settings);
        menu.acknowledge_intro();
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.snapshot().rows().last(), Some(&Row::Subtitles));
        assert_eq!(Row::Subtitles.kind(), "selector");
        assert_eq!(Row::Subtitles.title(), "ID_GAMESETTINGS_SUBTITLE_TOGGLE");
        menu.take_sounds();
        assert!(!menu.dirty() && !menu.snapshot().subtitles);
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(settings.subtitles && menu.snapshot().subtitles && menu.dirty());
        assert!(!menu.display_dirty());
        menu.update(press(|i| i.left = true), &mut settings);
        assert!(!settings.subtitles && !menu.snapshot().subtitles);
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_NAV_RIGHT", "FE_SOUND_CORE_NAV_LEFT"]);
        // A reopened screen reads the profile's value.
        settings.subtitles = true;
        menu.open(true, VideoScreen::Game, &settings);
        assert!(menu.snapshot().subtitles && !menu.dirty());
    }
    #[test]
    fn show_lobby_chat_toggles_on_the_online_page_and_saves_only_when_changed() {
        let mut settings = ProfileSettings::default();
        let mut menu = Controller::default();
        menu.load_lobby_chat(true);
        menu.open(true, VideoScreen::Game, &settings);
        menu.acknowledge_intro();
        assert_eq!(menu.snapshot().rows(), IN_GAME);
        for _ in 0..5 {
            menu.update(press(|i| i.down = true), &mut settings);
        }
        assert_eq!(menu.update(press(|i| i.select = true), &mut settings), Action::None);
        assert_eq!(menu.snapshot().page, Page::Online);
        assert_eq!(menu.snapshot().title(), "Online Settings");
        assert_eq!(menu.snapshot().rows(), ONLINE);
        menu.take_sounds();
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(!menu.show_lobby_chat() && menu.lobby_chat_dirty());
        // Not a profile setting: the profile is not dirtied.
        assert!(!menu.dirty() && !menu.display_dirty());
        menu.update(press(|i| i.left = true), &mut settings);
        assert!(menu.show_lobby_chat() && !menu.lobby_chat_dirty());
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_NAV_RIGHT", "FE_SOUND_CORE_NAV_LEFT"]);
        menu.update(press(|i| i.left = true), &mut settings);
        menu.close_after_save();
        assert!(!menu.show_lobby_chat() && !menu.lobby_chat_dirty());
        // The front end's list ends with it as well.
        menu.open(false, VideoScreen::MainMenu, &settings);
        assert_eq!(menu.snapshot().rows().last(), Some(&Row::OnlineSettings));
        assert!(!menu.snapshot().show_lobby_chat);
    }
    #[test]
    fn show_ghost_players_toggles_below_show_lobby_chat_and_saves_only_when_changed() {
        let mut settings = ProfileSettings::default();
        let mut menu = Controller::default();
        menu.load_lobby_chat(true);
        menu.load_ghost_players(true);
        menu.open(true, VideoScreen::Game, &settings);
        menu.acknowledge_intro();
        for _ in 0..5 {
            menu.update(press(|i| i.down = true), &mut settings);
        }
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.snapshot().rows(), [Row::ShowLobbyChat, Row::ShowGhostPlayers]);
        menu.update(press(|i| i.down = true), &mut settings);
        assert_eq!(Row::ShowGhostPlayers.kind(), "selector");
        assert_eq!(Row::ShowGhostPlayers.title(), "Show Ghost Players in pregame lobby");
        menu.take_sounds();
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(!menu.show_ghost_players() && !menu.snapshot().show_ghost_players && menu.ghost_players_dirty());
        // Neither a profile setting nor Show Lobby Chat.
        assert!(!menu.dirty() && !menu.lobby_chat_dirty() && menu.show_lobby_chat());
        menu.update(press(|i| i.left = true), &mut settings);
        assert!(menu.show_ghost_players() && !menu.ghost_players_dirty());
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_NAV_RIGHT", "FE_SOUND_CORE_NAV_LEFT"]);
        menu.update(press(|i| i.left = true), &mut settings);
        menu.close_after_save();
        assert!(!menu.show_ghost_players() && !menu.ghost_players_dirty());
    }
    #[test]
    fn video_settings_rows_follow_the_game_mode() {
        let mut settings = ProfileSettings::default();
        let mut menu = Controller::default();
        menu.open(true, VideoScreen::FreeSkate, &settings);
        menu.acknowledge_intro();
        for _ in 0..2 {
            menu.update(press(|i| i.down = true), &mut settings);
        }
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.snapshot().rows(), VIDEO_FREESKATE);
        // Camera Angle: low by default, then high.
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(settings.camera_angle == 1 && menu.snapshot().camera_high && menu.dirty());
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.left = true), &mut settings);
        assert!(settings.grind_transparency && menu.snapshot().grind_transparency);
        for _ in 0..2 {
            menu.update(press(|i| i.down = true), &mut settings);
        }
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(!settings.mini_map && !menu.snapshot().mini_map);
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(!settings.hud && !menu.snapshot().hud);
        menu.open(true, VideoScreen::Game, &settings);
        menu.acknowledge_intro();
        for _ in 0..2 {
            menu.update(press(|i| i.down = true), &mut settings);
        }
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.snapshot().rows(), VIDEO_GAME);
    }
    #[test]
    fn control_settings_toggle_the_off_board_axis_and_vibration() {
        let mut settings = ProfileSettings::default();
        let mut menu = Controller::default();
        menu.open(false, VideoScreen::MainMenu, &settings);
        menu.acknowledge_intro();
        for _ in 0..2 {
            menu.update(press(|i| i.down = true), &mut settings);
        }
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.snapshot().page, Page::Control);
        assert_eq!(menu.snapshot().title(), "ID_GAMESETTINGS_CONTROL_SETTINGS");
        assert_eq!(menu.snapshot().rows(), CONTROL);
        assert!(!menu.snapshot().invert_y && !menu.snapshot().vibration, "a new save's vibration is off");
        menu.take_sounds();
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(settings.invert_y && menu.snapshot().invert_y && menu.dirty());
        assert!(!menu.take_vibration_test());
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.left = true), &mut settings);
        assert!(settings.rumble && menu.snapshot().vibration && menu.take_vibration_test());
        assert!(!menu.take_vibration_test());
        menu.update(press(|i| i.right = true), &mut settings);
        assert!(!settings.rumble && !menu.snapshot().vibration && !menu.take_vibration_test());
        assert_eq!(
            menu.take_sounds(),
            ["FE_SOUND_CORE_NAV_RIGHT", "FE_SOUND_CORE_NAV_DOWN", "FE_SOUND_CORE_NAV_LEFT", "FE_SOUND_CORE_NAV_RIGHT"]
        );
        // B returns to the main list.
        assert_eq!(menu.update(press(|i| i.cancel = true), &mut settings), Action::None);
        assert_eq!(menu.snapshot().page, Page::Options);
    }
    /// The front end's Options, Control Settings, then Stick Settings.
    fn stick_settings(settings: &mut ProfileSettings) -> Controller {
        let mut menu = Controller::default();
        menu.load_sticks(ControllerSettings::VANILLA);
        menu.open(false, VideoScreen::MainMenu, settings);
        menu.acknowledge_intro();
        for _ in 0..2 {
            menu.update(press(|i| i.down = true), settings);
        }
        menu.update(press(|i| i.select = true), settings);
        assert_eq!(menu.snapshot().rows(), CONTROL);
        for _ in 0..2 {
            menu.update(press(|i| i.down = true), settings);
        }
        menu.update(press(|i| i.select = true), settings);
        assert_eq!(menu.snapshot().page, Page::StickSettings);
        menu
    }
    /// The D-pad held left (`-1`) or right (`1`) for `frames` frames, then let go.
    fn hold(menu: &mut Controller, settings: &mut ProfileSettings, axis: f32, frames: u32) {
        for _ in 0..frames {
            menu.update(press(|i| i.morph_axis_secondary = axis), settings);
        }
        menu.update(PauseMenuInput::default(), settings);
    }
    #[test]
    fn stick_settings_take_the_d_pad_alone_and_go_back_a_page_at_a_time() {
        let mut settings = ProfileSettings::default();
        let mut menu = stick_settings(&mut settings);
        assert_eq!(menu.snapshot().rows(), STICK_SETTINGS);
        assert_eq!(menu.snapshot().title(), "Stick Settings");
        assert!(menu.snapshot().stick_page());
        // The front end's generic directions hold the left stick, which these pages show: they move
        // nothing here.
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| { i.right = true; i.morph_axis = 1.; }), &mut settings);
        assert_eq!(menu.snapshot().selected, 0);
        menu.update(press(|i| i.dpad_down = true), &mut settings);
        assert_eq!(menu.snapshot().selected, 1);
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!((menu.snapshot().page, menu.snapshot().title()), (Page::RightStick, "Right Stick"));
        assert_eq!(menu.snapshot().rows(), STICK);
        assert_eq!(Row::StickDeadzone.kind(), "morph");
        // B goes back one page at a time, to the row that opened each.
        menu.update(press(|i| i.cancel = true), &mut settings);
        assert_eq!((menu.snapshot().page, menu.snapshot().selected), (Page::StickSettings, 1));
        menu.update(press(|i| i.cancel = true), &mut settings);
        let snapshot = menu.snapshot();
        assert_eq!((snapshot.page, snapshot.rows()[snapshot.selected]), (Page::Control, Row::StickSettings));
        menu.update(press(|i| i.cancel = true), &mut settings);
        assert_eq!(menu.snapshot().page, Page::Options);
        assert!(!menu.sticks_dirty() && !menu.dirty() && !menu.display_dirty());
    }
    #[test]
    fn a_held_multiplier_steps_a_hundredth_sooner_the_longer_it_is_held() {
        let mut settings = ProfileSettings::default();
        let mut menu = stick_settings(&mut settings);
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.snapshot().page, Page::LeftStick);
        menu.take_sounds();
        hold(&mut menu, &mut settings, -1., 1);
        assert_eq!(menu.sticks().left.multiplier, 99);
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_NAV_LEFT"]);
        // Under the front end's first repeat, nothing more.
        hold(&mut menu, &mut settings, -1., 24);
        assert_eq!(menu.sticks().left.multiplier, 98);
        // Two seconds held: 1, then 9 from the first repeat, then 30 over the next second.
        hold(&mut menu, &mut settings, -1., 120);
        assert_eq!(menu.sticks().left.multiplier, 58);
        // Past two seconds, a hundredth a frame, to the end of its range.
        hold(&mut menu, &mut settings, -1., 240);
        assert_eq!(menu.sticks().left.multiplier, *MULTIPLIERS.start());
        hold(&mut menu, &mut settings, 1., 1);
        assert_eq!(menu.sticks().left.multiplier, 41);
        assert_eq!(menu.sticks().right, StickSettings::VANILLA);
        assert_eq!(menu.snapshot().stick(), Some(menu.sticks().left));
        assert!(menu.sticks_dirty() && !menu.dirty(), "not a profile setting");
        assert_eq!(menu.snapshot().sticks, menu.sticks());
        menu.close_after_save();
        assert!(!menu.sticks_dirty());
    }
    #[test]
    fn a_deadzone_knob_slides_steadily_while_held_and_loops_its_sound() {
        let mut settings = ProfileSettings::default();
        let mut menu = stick_settings(&mut settings);
        menu.update(press(|i| i.select = true), &mut settings);
        menu.update(press(|i| i.dpad_down = true), &mut settings);
        assert_eq!(menu.snapshot().rows()[menu.snapshot().selected], Row::StickDeadzone);
        hold(&mut menu, &mut settings, 1., 1);
        assert_eq!(menu.sticks().left.deadzone, 1);
        assert_eq!(menu.take_sound_requests(), [FeSoundRequest::Loop(SLIDER_SOUND), FeSoundRequest::Stop(SLIDER_SOUND)]);
        // At once, then from a fifth of a second every third frame.
        hold(&mut menu, &mut settings, 1., 72);
        assert_eq!(menu.sticks().left.deadzone, 22);
        // At the end of its range the knob and its sound stop, though the D-pad stays held.
        menu.take_sound_requests();
        for _ in 0..300 {
            menu.update(press(|i| i.morph_axis_secondary = 1.), &mut settings);
        }
        assert_eq!(menu.sticks().left.deadzone, *DEADZONES.end());
        assert_eq!(menu.take_sound_requests(), [FeSoundRequest::Loop(SLIDER_SOUND), FeSoundRequest::Stop(SLIDER_SOUND)]);
        menu.update(PauseMenuInput::default(), &mut settings);
        assert!(menu.take_sound_requests().is_empty());
        // The anti-deadzone below it moves on its own.
        menu.update(press(|i| i.dpad_down = true), &mut settings);
        hold(&mut menu, &mut settings, 1., 1);
        assert_eq!((menu.sticks().left.deadzone, menu.sticks().left.anti_deadzone), (50, 1));
        // Reset To Default puts both sticks back to vanilla.
        menu.update(press(|i| i.cancel = true), &mut settings);
        for _ in 0..2 {
            menu.update(press(|i| i.dpad_down = true), &mut settings);
        }
        assert_eq!(menu.snapshot().rows()[menu.snapshot().selected], Row::ResetSticks);
        menu.update(press(|i| i.select = true), &mut settings);
        assert_eq!(menu.sticks(), ControllerSettings::VANILLA);
        assert_eq!(menu.snapshot().page, Page::StickSettings);
        assert!(!menu.sticks_dirty(), "the saved settings were vanilla");
    }
    #[test]
    fn leaving_a_moving_knob_stops_its_sound() {
        let mut settings = ProfileSettings::default();
        let mut menu = stick_settings(&mut settings);
        menu.update(press(|i| i.select = true), &mut settings);
        menu.update(press(|i| i.dpad_down = true), &mut settings);
        menu.update(press(|i| i.morph_axis_secondary = 1.), &mut settings);
        assert_eq!(menu.take_sound_requests(), [FeSoundRequest::Loop(SLIDER_SOUND)]);
        assert_eq!(menu.update(press(|i| { i.start = true; i.morph_axis_secondary = 1.; }), &mut settings), Action::Exit);
        assert_eq!(menu.take_sound_requests(), [FeSoundRequest::Stop(SLIDER_SOUND)]);
    }
}
