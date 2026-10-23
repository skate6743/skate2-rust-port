//! `FrontEndState_Options` (FE state 73, screen 120: `options/options.swf`), reached from the
//! main menu's Options and the StartHub's Settings. Its screens are `gGameOptionsScreens`
//! 0x82fdfa70 rows this port drives: the main list, Audio Settings, and the Video Settings
//! variant `OnMenuSelect` (0x826871a0) picks for the front end, a free skate game or any other
//! game. In game the list leads with the Music Player (FE state 30). PC port addition: a
//! Display Settings page after Video Settings holds the PC rows (PostFX, Fisheye, Frame Rate
//! Cap, VSync, Resolution Scale, Frame Rate Counter, Fullscreen), saved in their own chunk, and
//! an Online Settings page holds Show Lobby Chat, saved in `online-config.json`. Beside them the
//! PC port calls Video Settings (`ID_GAMESETTINGS_VIDEO_SETTINGS`) Gameplay Settings, for its
//! camera, HUD and subtitle rows.
use super::pause_menu::PauseMenuInput;
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
    /// PC port addition.
    Display,
    /// PC port addition.
    Online,
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
    /// PC port addition: `online-config.json`'s `show_lobby_chat`.
    ShowLobbyChat,
}
impl Row {
    /// `gGameOptionsItems` 0x82fdf9a8's type: `option`, `slider` or `selector`.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::MusicPlayer | Self::AudioSettings | Self::VideoSettings | Self::DisplaySettings | Self::OnlineSettings => {
                "option"
            }
            Self::SfxVolume | Self::DialogueVolume | Self::MusicVolume => "slider",
            Self::PostFx | Self::Fisheye | Self::FrameRateCap | Self::VSync | Self::ResolutionScale
            | Self::FpsCounter | Self::Fullscreen | Self::Subtitles | Self::CameraAngle | Self::GrindTransparency
            | Self::MiniMap | Self::Hud | Self::ShowLobbyChat => "selector",
        }
    }
    /// `gGameOptionsItems`' title id; the PC rows carry literal text.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::MusicPlayer => "ID_GAMESETTINGS_MUSICPLAYER",
            Self::AudioSettings => "ID_GAMESETTINGS_AUDIO_SETTINGS",
            Self::VideoSettings => "Gameplay Settings",
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
            Self::ShowLobbyChat => "Show Lobby Chat",
        }
    }
}
const FRONT_END: &[Row] = &[Row::AudioSettings, Row::VideoSettings, Row::DisplaySettings, Row::OnlineSettings];
const IN_GAME: &[Row] =
    &[Row::MusicPlayer, Row::AudioSettings, Row::VideoSettings, Row::DisplaySettings, Row::OnlineSettings];
const AUDIO: &[Row] = &[Row::SfxVolume, Row::DialogueVolume, Row::MusicVolume];
/// `gGameOptionsScreens` rows 7, 3 and 5 without `eSk8FEGameOption_VideoCalibration`, whose
/// FE state 0 calibration screen is not ported.
const VIDEO_MAIN_MENU: &[Row] = &[Row::Subtitles];
const VIDEO_GAME: &[Row] = &[Row::CameraAngle, Row::GrindTransparency, Row::Subtitles];
const VIDEO_FREESKATE: &[Row] = &[Row::CameraAngle, Row::GrindTransparency, Row::Subtitles, Row::MiniMap, Row::Hud];
const DISPLAY: &[Row] = &[
    Row::PostFx,
    Row::Fisheye,
    Row::FrameRateCap,
    Row::VSync,
    Row::ResolutionScale,
    Row::FpsCounter,
    Row::Fullscreen,
];
const ONLINE: &[Row] = &[Row::ShowLobbyChat];

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
    /// Show Lobby Chat (PC port addition).
    pub show_lobby_chat: bool,
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
            Page::Display => DISPLAY,
            Page::Online => ONLINE,
        }
    }
    /// `gGameOptionsScreenTitle` 0x82fdf97c.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self.page {
            Page::Audio => "ID_GAMESETTINGS_AUDIO_SETTINGS",
            Page::Video => "Gameplay Settings",
            Page::Display => "Display Settings",
            Page::Online => "Online Settings",
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
    /// `mIsDirty`: a profile setting changed since `OnEnter`.
    dirty: bool,
    /// `FE_SOUND_*` globals this input played (options.apt and `FrontEndState_Options`).
    sounds: Vec<&'static str>,
    /// `IsScreenReady` 0x82d661a8: from the screen's `IntroComplete` (`OnScreenReady`
    /// 0x82d65f98) `StateManagerFrontEnd::ReceiveInput` 0x82656af8 hands the state its input.
    ready: bool,
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
    /// Whether exiting saves Show Lobby Chat to `online-config.json`.
    #[must_use]
    pub const fn lobby_chat_dirty(&self) -> bool {
        self.state.show_lobby_chat != self.saved_lobby_chat
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
        self.refresh(settings);
    }
    pub fn close_after_save(&mut self) {
        self.saved = self.state.values;
        self.saved_lobby_chat = self.state.show_lobby_chat;
        self.dirty = false;
        self.state.page = Page::Hidden;
        self.state.music_player = false;
    }
    /// `OnLeave` 0x8266af70 under the pushed music player: the screen closes without saving.
    pub fn suspend(&mut self) {
        self.state.page = Page::Hidden;
        self.state.music_player = true;
    }
    /// `OnLeave` 0x8266af70 as `PopAllStates` pops the state (or the music player over it):
    /// the screen closes and nothing saves. The settings keep what the rows changed, and
    /// `mIsDirty` stays until the next `OnEnter` clears it.
    pub fn leave(&mut self) {
        self.state.page = Page::Hidden;
        self.state.music_player = false;
        self.ready = false;
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
    }
    pub fn update(&mut self, input: PauseMenuInput, settings: &mut ProfileSettings) -> Action {
        if !self.visible() || !self.ready {
            return Action::None;
        }
        if input.start {
            return Action::Exit;
        }
        if input.cancel {
            // ReceiveInput 0x82686978: core_b_button, then RequestExit or, from a category,
            // LoadMainMenu.
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            if self.state.page == Page::Options {
                return Action::Exit;
            }
            self.state.page = Page::Options;
            self.state.selected = 0;
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
            | Row::Fullscreen | Row::ShowLobbyChat => {
                // The PC selectors sound core_nav_left/core_nav_right by key and, as
                // Prev/NextValueCalc do, wrap past either end; Resolution Scale stops at its ends.
                self.sounds.push(if input.right { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                if row == Row::ShowLobbyChat {
                    self.state.show_lobby_chat = !self.state.show_lobby_chat;
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
            _ => return Action::None,
        };
        // The volume clamp, then RefreshCurrentItem 0x8265c5a0 marks the profile dirty.
        let next = step * 0.1 + *volume;
        *volume = if next < 0.0 { 0.0 } else if next > 1.0 { 1.0 } else { next };
        self.dirty = true;
        self.refresh(settings);
        Action::None
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
        menu.update(press(|i| i.down = true), &mut settings);
        menu.update(press(|i| i.down = true), &mut settings);
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
        for _ in 0..4 {
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
}
