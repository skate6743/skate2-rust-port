//! screenmanager.apt's open and destroy queues, and the dimmer it plays, as
//! `FrontEndManager::Update` (0x82640130) drives them each frame: `AptIntegration::Update`
//! (0x826014c0) runs the APT tick (`tick` 0x82a21c10: the display lists, `RunActions` with the
//! screen manager's `onEnterFrame`, `AptLinker::Update`) and
//! `AptLoadingIntegration::CheckAndCompleteAsyncLoad` (0x82a728b0); then
//! `StateManagerFrontEndBase::Update` (0x82d68370) asks for the screens of the states whose
//! `OpenScreen` (0x82d67000) left them in `eScreenState_RequestLoad`, and `ExecuteActions` pushes
//! and pops states.
//!
//! `ScreenManager.Update` runs `ProcessDestroyedScreens`, and `ProcessOpenScreens` only when that
//! removed nothing. `ProcessOpenScreens` counts `mBlockScreenOpen` down, else creates the first
//! queued screen and sets `mBlockScreenOpen` to 8; once nothing is queued it clears
//! `onEnterFrame`, so the count stays where a `CreateScreen` left it until the next `ReadyScreen`
//! or `DestroyScreen` sets `onEnterFrame` again. A screen opened after another was created, with
//! no screen destroyed since, waits those 8 ticks.
//!
//! MainMenu -> Extras is an ordinary PushState(24), not a NIS (OnMenuSelectGame
//! 0x82681338). FE states 22 and 24 both have mbTransition=false in gFEStateInfo
//! 0x82fded80: UpdateOnLeave (0x82d66300) closes the old screen directly. The main
//! menu is a standard screen; Extras' TransitionAtEnd initially has no managed
//! predecessor and uses InitialScreenReady. Their open/destroy queues supply
//! the timing; there is no additional wait-for-outro gate for this pair.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;
use std::collections::VecDeque;

/// A front-end state's screen the port opens through the screen manager.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Screen {
    /// `FrontEndState_StartHub`'s screen 11, starthub/starthubmenu.swf.
    StartHub,
    /// `FrontEndState_Options`' screen 120, options/options.swf.
    Options,
    /// `FrontEndState_MusicPlayer`'s screen 35, musicplayer/musicplayertest.swf.
    MusicPlayer,
    /// MainMenu's screen 8, main/mainmenu.swf (standard OpenScreen).
    MainMenu,
    /// Extras' screen 70, main/extras.swf (TransitionAtEnd).
    Extras,
    /// Replay Editor screen15, replay2/replay_editor2.
    ReplayEditor,
    /// Local saved footage, screen102, reached by Replay2's save-limit flow.
    ReplayFootage,
    OnlineOptions,
    SkateReelGeneral,
    SkateReelPhoto,
    TrickGuide,
    Contacts,
    /// `FrontEndState_SkateProfile`'s screen 93, onlinehub/skateProfile.swf.
    SkateProfile,
    /// `FrontEndState_DLC`'s screen 10, main/dlc.swf.
    Dlc,
}

impl Screen {
    /// `FrontEndManager::PreloadTextures` (0x82610730) keeps the controls, the StartHub and the
    /// online hub loaded: their `loadMovie` completes at once and `AptLinker::Update` links them in
    /// the tick that creates them.
    const fn preloaded(self) -> bool {
        matches!(self, Self::StartHub)
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// APT ticks from the `CreateScreen` that `loadMovie`s a screen from `screens.big` to the
/// `AptLinker::Update` (0x82a21228) that links it: `CheckAndCompleteAsyncLoad` moves the load one
/// step a tick (`CheckLoadingOperation` 0x82a720d8: the .apt, .const and .geo reads, the .rx2 arena
/// of `LoadAsyncArena` 0x82a70800, then `CompleteLoad` 0x82a72030), with every read done within its
/// tick; disc reads only lengthen it.
const LOAD_TICKS: u32 = 5;

/// `ProcessOpenScreens`' `mBlockScreenOpen` after each `CreateScreen`.
const BLOCK_TICKS: u32 = 8;

/// What the screens told their states since the last take.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ScreenEvents {
    /// The StartHub's `IntroComplete` (`OnScreenReady` 0x82d65f98).
    pub start_hub_intro: bool,
    pub replay_editor_intro: bool,
    pub trick_guide_intro:bool,
    pub contacts_intro:bool,
    /// skate.Profile's `IntroComplete`, from which its state takes input.
    pub skate_profile_intro: bool,
    pub replay_editor_ready: bool,
    pub replay_footage_intro: bool,
    pub skate_reel_intro: [bool;3],
    /// Options' `IntroComplete`.
    pub options_intro: bool,
    /// The music player's screen linked: `Game.OnScreenLoaded` runs the state's `OnLoaded`.
    pub music_player_loaded: bool,
    /// The music player's `IntroComplete`.
    pub music_player_intro: bool,
}

#[derive(Default)]
pub(super) struct ScreenManager {
    /// `mBlockScreenOpen`.
    block: u32,
    /// `mScreenOpenArray`, which `ReadyScreen` appends to.
    open: VecDeque<Screen>,
    /// `mPendingScreensToClear`: the `DestroyScreen`s no `ProcessDestroyedScreens` removed yet.
    clear: u32,
    /// `mScreen.onEnterFrame` is `Update`.
    armed: bool,
    /// States whose `OnEnter` opened their screen since the last state update.
    requests: Vec<Screen>,
    /// Screens `CreateScreen` is loading, with the ticks until they link.
    loading: Vec<(Screen, u32)>,
    /// `mScreenArrayDepth`, which `ReadyScreen` gives the screen it queues and then increments;
    /// `CreateScreen` makes the screen's clip at that depth.
    depth: u32,
    /// Each screen's depth from its last `ReadyScreen`.
    depths: [u32; 14],
}

impl ScreenManager {
    /// A state's `OnEnter` opens its screen (`eScreenState_RequestLoad`).
    pub fn request(&mut self, screen: Screen) {
        self.requests.push(screen);
    }

    /// The state left before its screen came, which is then never shown.
    pub fn cancel(&mut self, screen: Screen) {
        self.requests.retain(|&s| s != screen);
        self.open.retain(|&s| s != screen);
        self.loading.retain(|&(s, _)| s != screen);
    }

    /// A movie's state change: it began waiting for its screen (`request`), or stopped waiting
    /// before the screen came (`cancel`).
    pub fn track(&mut self, screen: Screen, was_pending: bool, pending: bool) {
        if pending && !was_pending {
            self.request(screen);
        } else if was_pending && !pending {
            self.cancel(screen);
        }
    }

    /// A screen's `OutroComplete`: `DestroyScreen` queues its removal and sets `onEnterFrame`. The
    /// screen manager's `onEnterFrame` ran before its screens' frame actions in that tick, so the
    /// next tick removes it.
    pub fn destroy(&mut self) {
        self.clear += 1;
        self.armed = true;
    }

    /// One APT tick: the loads that finished link, and `Update` runs if `onEnterFrame` is set.
    /// Returns the screens linked this tick; their constructors play `intro` after the tick's
    /// frame actions.
    pub fn tick(&mut self) -> Vec<Screen> {
        let mut linked = Vec::new();
        self.loading.retain_mut(|(screen, ticks)| {
            *ticks -= 1;
            if *ticks == 0 {
                linked.push(*screen);
            }
            *ticks != 0
        });
        if !self.armed {
            return linked;
        }
        if self.clear > 0 {
            // ProcessDestroyedScreens: RemoveScreen, and true.
            self.clear -= 1;
        } else if self.block > 0 {
            self.block -= 1;
        } else {
            if let Some(screen) = self.open.pop_front() {
                self.block = BLOCK_TICKS;
                if screen.preloaded() {
                    linked.push(screen);
                } else {
                    self.loading.push((screen, LOAD_TICKS));
                }
            }
            if self.open.is_empty() {
                self.armed = false;
            }
        }
        linked
    }

    /// `StateManagerFrontEndBase::Update` after the tick: each requested screen's `UpdateFEScreen`
    /// (0x82657248) -> `ScreenManager.OpenScreen` -> `ReadyScreen` queues it and sets
    /// `onEnterFrame`.
    pub fn update_states(&mut self) {
        for screen in self.requests.drain(..) {
            self.depths[screen.index()] = self.depth;
            self.depth += 1;
            self.open.push_back(screen);
            self.armed = true;
        }
    }

    /// The screen opened after the other, which draws over it.
    pub fn over(&self, screen: Screen, other: Screen) -> bool {
        self.depths[screen.index()] > self.depths[other.index()]
    }
}

/// mainmisc.swf's `gDimmer` (main/dimmer.swf on `_level8`, the first level
/// `AptRenderingIntegration::Render2D` 0x82631a98 draws, under the 3D levels and main.swf's
/// screens), as screenmanager.apt's `ActivateDimmer` plays it for
/// `StateManagerFrontEnd::EnableDimmer` (0x82656a30).
pub(super) struct Dimmer {
    pub clip: Clip,
    /// `mbDimmerEnabled`: a request that changes nothing plays nothing.
    enabled: bool,
    /// The timeline's `Audio.PlaySound` requests.
    pub sounds: Vec<&'static str>,
}

impl Dimmer {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut clip = Clip::new(assets, 12, 0)?;
        clip.visible = false;
        clip.playback.stop();
        Ok(Self { clip, enabled: false, sounds: Vec::new() })
    }

    /// `ActivateDimmer(enable)`: `intro` or `outro` when the request changes `mbDimmerEnabled`.
    pub fn enable(&mut self, assets: &Assets, enable: bool) -> RenderResult<()> {
        if self.enabled != enable {
            self.enabled = enable;
            self.clip.goto(assets, if enable { "intro" } else { "outro" })?;
        }
        self.calls()
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if self.clip.visible {
            self.clip.tick(assets)?;
        }
        self.calls()
    }

    fn calls(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.clip.take_calls(&mut calls);
        for call in calls {
            match call {
                "FE_SOUND_CORE_FADE" => self.sounds.push(call),
                _ => return Err(format!("untranslated dimmer callback {call}").into()),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        letterbox::LetterBox,
        music_player_movie::MusicPlayerMovie,
        pause_movie::{HubRow, PauseMovie},
        settings_movie::SettingsMovie,
    };
    use super::*;
    use skate2_core::ui::{
        music_player::Snapshot as MusicPlayer,
        options::{Page, Snapshot as Options},
        pause_menu::SINGLE_PLAYER_OPTIONS,
    };
    use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};

    #[test]
    fn a_destroyed_screen_unblocks_the_next_open() {
        let mut screens = ScreenManager::default();
        screens.request(Screen::Options);
        screens.update_states();
        // The first open after start: created at once, linked five ticks later.
        assert!(screens.tick().is_empty());
        for _ in 0..4 {
            assert!(screens.tick().is_empty());
        }
        assert_eq!(screens.tick(), [Screen::Options]);
        // The next open waits out the block the creation left.
        screens.request(Screen::StartHub);
        screens.update_states();
        for _ in 0..8 {
            assert!(screens.tick().is_empty());
        }
        assert_eq!(screens.tick(), [Screen::StartHub]);
        // A destroy re-arms the update, which removes it, then counts the block down to idle.
        screens.destroy();
        for _ in 0..10 {
            screens.tick();
        }
        assert!(!screens.armed && screens.block == 0);
        screens.request(Screen::MusicPlayer);
        screens.update_states();
        assert!(screens.tick().is_empty());
        assert_eq!(screens.loading, [(Screen::MusicPlayer, LOAD_TICKS)]);
    }

    struct Front {
        assets: Assets,
        screens: ScreenManager,
        dimmer: Dimmer,
        hub: PauseMovie,
        options: SettingsMovie,
        player: MusicPlayerMovie,
        rows: Vec<HubRow>,
    }

    impl Front {
        /// MarkerUi's APT step.
        fn step(&mut self) {
            let assets = &self.assets;
            let linked = self.screens.tick();
            self.hub.tick(assets).unwrap();
            if self.hub.take_outro_complete() {
                self.screens.destroy();
            }
            self.options.tick(assets, None).unwrap();
            if self.options.take_outro_complete() {
                self.screens.destroy();
            }
            self.player.tick(assets).unwrap();
            if self.player.take_outro_complete() {
                self.screens.destroy();
            }
            for screen in linked {
                match screen {
                    Screen::StartHub => self.hub.link(assets).unwrap(),
                    Screen::Options => self.options.link(assets, None).unwrap(),
                    Screen::MusicPlayer => self.player.link(assets).unwrap(),
                    _ => unreachable!("not part of this fixture"),
                }
            }
            if self.player.take_letterbox_off() {
                self.options.enable_letterbox(assets, false, None).unwrap();
            }
            self.dimmer.tick(assets).unwrap();
            self.screens.update_states();
        }

        /// The frame's state changes: the StartHub on top or not, Options, the music player, and
        /// the dimmer of the state on top.
        fn update(&mut self, hub: bool, options: Options, player: &MusicPlayer) {
            let assets = &self.assets;
            let pending = self.hub.pending();
            self.hub.update(assets, hub, 1, &self.rows).unwrap();
            self.screens.track(Screen::StartHub, pending, self.hub.pending());
            let pending = self.options.pending();
            self.options.update(assets, options, None).unwrap();
            self.screens.track(Screen::Options, pending, self.options.pending());
            let pending = self.player.pending();
            self.player.update(assets, player).unwrap();
            self.screens.track(Screen::MusicPlayer, pending, self.player.pending());
            self.dimmer.enable(assets, options.page != Page::Hidden || options.music_player).unwrap();
        }
    }

    fn sounds(letterbox: &mut LetterBox) -> Vec<&'static str> {
        std::mem::take(&mut letterbox.sounds)
    }

    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT"]
    fn start_hub_options_and_music_player_follow_the_screen_manager() {
        let game = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES.iter() {
            mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
        }
        let assets = Assets::load(&mounts).unwrap();
        let mut front = Front {
            screens: ScreenManager::default(),
            dimmer: Dimmer::new(&assets).unwrap(),
            hub: PauseMovie::new(&assets).unwrap(),
            options: SettingsMovie::new(&assets).unwrap(),
            player: MusicPlayerMovie::new(&assets).unwrap(),
            rows: SINGLE_PLAYER_OPTIONS
                .iter()
                .map(|o| HubRow { label: o.localization_key(), count: None, enabled: o.enabled() })
                .collect(),
            assets,
        };
        let mut options = Options { in_game: true, ..Default::default() };
        let closed = MusicPlayer::default();

        // Start: the StartHub is preloaded, so the screen manager links it as it creates it.
        front.update(true, options, &closed);
        front.step();
        assert!(!front.hub.root.visible);
        front.step();
        assert!(front.hub.root.visible);
        for tick in 3..=10 {
            front.step();
            assert_eq!(front.hub.take_intro_complete(), tick == 10, "tick {tick}");
        }
        for _ in 0..20 {
            front.step();
        }

        // Game Settings: the hub's outro and the dimmer's intro at once; the hub's creation left
        // the 8-tick block, and its destroy takes one update, so Options is created on tick 11
        // and linked on tick 16.
        options.page = Page::Options;
        front.update(false, options, &closed);
        assert_eq!(std::mem::take(&mut front.hub.sounds), ["FE_SOUND_PAUSE_MENU"]);
        assert_eq!(std::mem::take(&mut front.dimmer.sounds), ["FE_SOUND_CORE_FADE"]);
        for tick in 1..=16 {
            front.step();
            assert_eq!(front.options.root.visible, tick == 16, "tick {tick}");
            assert_eq!(front.hub.root.visible, tick < 8, "tick {tick}");
        }
        assert_eq!(front.options.root.playback.timeline().frame(), Some(1));
        for _ in 0..30 {
            front.step();
        }
        assert!(front.options.take_intro_complete());
        assert_eq!(sounds(&mut front.options.letterbox), ["FE_SOUND_LETTERBOX_IN", "FE_SOUND_PAGETITLE_FLICKER"]);
        let dimmed = front.dimmer.clip.playback.timeline().frame();
        assert_eq!(dimmed, Some(29));

        // Music Player: Options' creation left the block, so the music player links on tick 15,
        // over Options' last outro frame; the rows go on tick 16. Its frame 1 replays the bars'
        // outro.
        options.page = Page::Hidden;
        options.music_player = true;
        let mut player = MusicPlayer { visible: true, ..Default::default() };
        front.update(false, options, &player);
        for tick in 1..=16 {
            front.step();
            assert_eq!(front.player.root.visible, tick >= 15, "tick {tick}");
            assert_eq!(front.options.root.visible, tick < 16, "tick {tick}");
        }
        assert!(front.screens.over(Screen::MusicPlayer, Screen::Options));
        assert!(front.player.take_loaded());
        assert_eq!(sounds(&mut front.options.letterbox), ["FE_SOUND_LETTERBOX_OUT", "FE_SOUND_LETTERBOX_OUT"]);
        for _ in 0..40 {
            front.step();
        }
        assert!(front.player.take_intro_complete());
        assert_eq!(front.dimmer.clip.playback.timeline().frame(), dimmed);
        assert!(front.dimmer.sounds.is_empty());

        // B: the music player's destroy came after Options' creation, so the block ran out and
        // Options links 7 ticks after the pop.
        player.visible = false;
        options.page = Page::Options;
        options.music_player = false;
        front.update(false, options, &player);
        for tick in 1..=7 {
            front.step();
            assert_eq!(front.options.root.visible, tick == 7, "tick {tick}");
            assert!(front.player.root.visible);
        }
        // The new Options draws over the music player's outro.
        assert!(front.screens.over(Screen::Options, Screen::MusicPlayer));
        for _ in 0..40 {
            front.step();
        }
        assert!(!front.player.root.visible && front.options.take_intro_complete());
        assert_eq!(sounds(&mut front.options.letterbox), ["FE_SOUND_LETTERBOX_IN", "FE_SOUND_PAGETITLE_FLICKER"]);
        assert_eq!(front.dimmer.clip.playback.timeline().frame(), dimmed);

        // B again, back to the StartHub: its new screen links on the second tick, under Options'
        // outro; the dimmer and the bars run their outros to the end.
        options.page = Page::Hidden;
        front.update(true, options, &player);
        assert_eq!(std::mem::take(&mut front.dimmer.sounds), ["FE_SOUND_CORE_FADE"]);
        for tick in 1..=30 {
            front.step();
            assert_eq!(front.hub.root.visible, tick >= 2, "tick {tick}");
            assert_eq!(front.options.root.visible, tick < 16, "tick {tick}");
            assert_eq!(front.options.letterbox.clip.visible, tick < 27, "tick {tick}");
            assert_eq!(front.dimmer.clip.visible, tick < 29, "tick {tick}");
        }
        assert_eq!(sounds(&mut front.options.letterbox), ["FE_SOUND_LETTERBOX_OUT"]);
        assert!(front.hub.take_intro_complete());
    }
}
