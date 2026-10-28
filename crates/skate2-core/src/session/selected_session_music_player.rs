//! The profile's `tSettings` and `tMusicPlayer` as Options and the music player edit them, and
//! what the audio system reads of them each frame.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage};
use crate::{
    edit_skater_profile::{AudioProfile, EditSkaterProfileStore},
    ui::{
        controller_config,
        music_player::{self, profile_music_mode, Manager, MusicRequest},
        online_config::OnlineConfig,
        options::{Action, Controller, ProfileSettings},
        pause_menu::PauseMenuInput,
    },
};

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::PreupdateInput, e.to_string())
}

pub(super) struct ProfileAudio {
    /// `ProfileBackEnd::mSettings`.
    pub(super) settings: ProfileSettings,
    /// `FrontEndManager::mMusicPlayerMan` over `mMusicPlayer`, once the music system named its
    /// catalogue.
    pub(super) manager: Option<Manager>,
    pub(super) menu: music_player::Controller,
    /// `tProfileData::mNewProfile`.
    pub(super) new_profile: bool,
    /// The audio system's `IAudioInterface`, when the host has one.
    pub(super) interface: Option<std::sync::Arc<dyn crate::nis::audio::AudioInterface>>,
}

impl Default for ProfileAudio {
    /// Silent until the boot reads SKATER.P's settings (PC port change): the audio system starts
    /// before the profile is read, and at `tSettings::Default`'s full volumes a profile saved at
    /// no volume would sound until then. A profile without settings reads the defaults.
    fn default() -> Self {
        Self {
            settings: ProfileSettings { sfx_volume: 0.0, dialogue_volume: 0.0, music_volume: 0.0, ..ProfileSettings::default() },
            manager: None,
            menu: music_player::Controller::default(),
            new_profile: false,
            interface: None,
        }
    }
}
/// What Options' input did this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OptionsOutcome {
    Hidden,
    Shown,
    /// `RequestExit` closed the screen.
    Exited,
}

impl ProfileAudio {
    /// `MusicPlayerManager::RevertMusicMode`, once the music system named its catalogue.
    pub(super) fn revert_music_mode(&mut self) {
        if let Some(manager) = self.manager.as_mut() {
            manager.revert_music_mode();
        }
    }
    pub(super) fn profile(&self) -> AudioProfile<'_> {
        AudioProfile {
            settings: &self.settings,
            music_player: self.manager.as_ref().map(|m| (&m.profile, m.catalogue())),
        }
    }
    /// Options' input, then the pushed music player's.
    pub(super) fn update_options(
        &mut self,
        options: &mut Controller,
        input: PauseMenuInput,
        has_focus: bool,
        main_challenge: bool,
        party_play: bool,
    ) -> Result<OptionsOutcome, SelectedLiveError> {
        if self.menu.visible() {
            let manager = self.manager.as_mut().ok_or_else(|| error("music player catalogue absent"))?;
            if has_focus && self.menu.update(input, manager, &mut self.settings).map_err(error)? {
                // PopState: Options' OnEnter reopens its screen.
                self.menu.close();
                let video = options.snapshot().video;
                options.open(true, video, &self.settings);
            }
            return Ok(OptionsOutcome::Shown);
        }
        if !options.visible() {
            return Ok(OptionsOutcome::Hidden);
        }
        if !has_focus {
            return Ok(OptionsOutcome::Shown);
        }
        match options.update(input, &mut self.settings) {
            Action::None => Ok(OptionsOutcome::Shown),
            Action::MusicPlayer => {
                // Without an audio device the music system never named its songs.
                if let Some(manager) = self.manager.as_mut() {
                    options.suspend();
                    self.menu.open(manager, &self.settings);
                }
                Ok(OptionsOutcome::Shown)
            }
            Action::Exit => {
                self.request_options_exit(options, main_challenge, party_play)?;
                Ok(OptionsOutcome::Exited)
            }
        }
    }
    /// `FrontEndState_Options::RequestExit` 0x82681700: the music player's `mbIsDirty` folds
    /// into `mIsDirty`, which saves the profile (`RequestSave(eBeMemcardSave_Profile)`) when no
    /// main challenge runs or the game mode is Party Play. The PC rows save apart from it: the
    /// display chunk, Show Lobby Chat and Show Ghost Players in pregame lobby in
    /// `online-config.json`, and the stick settings in `controller_config.json`.
    fn request_options_exit(&mut self, options: &mut Controller, main_challenge: bool, party_play: bool) -> Result<(), SelectedLiveError> {
        let dirty = self.menu.take_dirty() | options.dirty();
        let path = EditSkaterProfileStore::default_path().map_err(error)?;
        if dirty && (!main_challenge || party_play) {
            EditSkaterProfileStore::open(&path)
                .and_then(|mut store| store.save_audio_profile(&self.profile()))
                .map_err(error)?;
        }
        if options.display_dirty() {
            EditSkaterProfileStore::open(&path)
                .and_then(|mut store| store.save_display_settings(options.snapshot().values))
                .map_err(error)?;
        }
        if options.lobby_chat_dirty() || options.ghost_players_dirty() {
            OnlineConfig { show_lobby_chat: options.show_lobby_chat(), show_ghost_players: options.show_ghost_players(), ..OnlineConfig::load() }
                .save()
                .map_err(error)?;
        }
        if options.sticks_dirty() {
            controller_config::save(&options.sticks()).map_err(error)?;
        }
        options.close_after_save();
        Ok(())
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The host's audio system (`SingletonHolder<AudioSystem>`), which the career's scenes and
    /// challenge audio manager call.
    pub fn set_audio_interface(&mut self, audio: std::sync::Arc<dyn crate::nis::audio::AudioInterface>) {
        self.audio.interface = Some(audio);
    }
    /// The music system's catalogue (`GetMusicPlayerSongIDFromIndex` below
    /// `GetNumMusicPlayerSongs`), over which the profile's `tMusicPlayer` loads.
    pub fn set_music_catalogue(&mut self, catalogue: Vec<u32>) -> Result<(), SelectedLiveError> {
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let profile = store.load_music_player(&catalogue).map_err(error)?;
        self.audio.manager = Some(Manager::new(profile, catalogue));
        Ok(())
    }
    /// `GetCurrentMusicPlayerSongID` and the `mCurrentSongIndex` the music system left after
    /// its update.
    pub fn set_music_player_song(&mut self, song: u32, song_index: i32) {
        if let Some(manager) = self.audio.manager.as_mut() {
            manager.set_current_song(song);
            manager.profile.current_song_index = song_index;
        }
    }
    /// The `MusicSystem` calls the front end made since the last call.
    pub fn take_music_requests(&mut self) -> Vec<MusicRequest> {
        self.audio.manager.as_mut().map_or_else(Vec::new, Manager::take_requests)
    }
    /// The current playlist's songs and `mCurrentSongIndex`.
    #[must_use]
    pub fn music_player_playlist(&self) -> (&[u32], i32) {
        self.audio.manager.as_ref().map_or((&[], 0), |m| (m.current_playlist(), m.profile.current_song_index))
    }
    /// `ProfileBackEnd::mMusicPlayer.mMusicMode`.
    #[must_use]
    pub fn music_mode(&self) -> i32 {
        self.audio.manager.as_ref().map_or(profile_music_mode::WORLD, |m| m.profile.music_mode)
    }
    #[must_use]
    pub fn new_profile(&self) -> bool {
        self.audio.new_profile
    }
    /// `ProfileBackEnd::mSettings`: the volumes the audio system mixes with.
    #[must_use]
    pub fn profile_settings(&self) -> &ProfileSettings {
        &self.audio.settings
    }
    /// A free skate game's (`GameBackEnd::IsFreeSkateMode`, no career challenge system) profile
    /// `mHUD` and `mMiniMap`, which its HUD and minimap follow.
    #[must_use]
    pub fn free_skate_hud_settings(&self) -> Option<(bool, bool)> {
        self.career.is_none().then_some((self.audio.settings.hud, self.audio.settings.mini_map))
    }
    #[must_use]
    pub fn music_player_snapshot(&self) -> &music_player::Snapshot {
        self.audio.menu.snapshot()
    }
    /// What the screens told their states: the StartHub's screen 11, Options' 120 and the music
    /// player's 35 take input from their `IntroComplete` (`OnScreenReady` 0x82d65f98) on, and the
    /// music player's link runs its `OnLoaded`.
    pub fn acknowledge_screens(&mut self, start_hub_intro: bool, options_intro: bool,
        music_player_loaded: bool, music_player_intro: bool) {
        if start_hub_intro {
            self.pause_menu.acknowledge_intro();
        }
        if options_intro {
            self.options_menu.acknowledge_intro();
        }
        if music_player_loaded && let Some(manager) = self.audio.manager.as_mut() {
            self.audio.menu.loaded(manager);
        }
        if music_player_intro {
            self.audio.menu.acknowledge_intro();
        }
    }
    /// `CommonBackEnd::sGameInfo.gameMode == GM_PartyPlay`.
    pub(super) fn party_play_mode(&self) -> Result<bool, SelectedLiveError> {
        let Some(owner) = self.inner.world.as_ref() else { return Ok(false) };
        let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
        let Some(services) = world.physical_world_services() else { return Ok(false) };
        let mode = services.game_info().try_game_mode().map_err(|e| error(format!("{e:?}")))?;
        Ok(i64::from(mode) == i64::from(crate::challenge::players::GM_PARTY_PLAY))
    }
    /// The songs `MusicPlayerManager::Play` showed on the chyron since the last call.
    pub fn take_chyron_songs(&mut self) -> Vec<u32> {
        self.audio.manager.as_mut().map_or_else(Vec::new, Manager::take_chyron)
    }
    /// The front-end state whose `UpdateOnEnter` ran last, for the chyron: in the front end
    /// CaC, Options, FreeSkateOption or MainMenu, else the screen `frontend_audio_state`
    /// reports, else free skate's.
    #[must_use]
    pub fn chyron_state(&self) -> i32 {
        use crate::ui::{main_menu::Screen, FrontendAudioState};
        const MAIN_MENU: i32 = 22;
        const FREE_SKATE_OPTION: i32 = 40;
        // These states can cover the main menu as well as the gameplay HUD.
        // Let Chyron::Enable run its native outro/visibility transitions.
        if self.movie_state_open() || self.replay_editor_open()
            || self.main_menu.as_ref().is_some_and(|menu| menu.extras.is_some()||menu.skate_reel.is_some()) {
            if let Some(state) = self.frontend_audio_state() { return state.state; }
        }
        let Some(menu) = &self.main_menu else {
            return self.frontend_audio_state().unwrap_or(FrontendAudioState::FREE_SKATE).state;
        };
        if self.editor.is_some() {
            FrontendAudioState::CAC.state
        } else if self.options_menu.visible() {
            FrontendAudioState::OPTIONS.state
        } else if matches!(menu.screen, Screen::ToFreeSkate | Screen::FreeSkate) {
            FREE_SKATE_OPTION
        } else {
            MAIN_MENU
        }
    }
}
