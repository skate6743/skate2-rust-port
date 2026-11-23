//! `FrontEndState_MusicPlayer` (FE state 30, screen 35: `musicplayer/musicplayertest.swf`), the
//! movie's script, and `MusicPlayerManager` over the profile's `tMusicPlayer`. The music system
//! is the audio crate's: this side queues `MusicSystem` calls and is told the playing song.
use super::custom_songs;
use super::pause_menu::PauseMenuInput;
use crate::native_skater_random::rw_math::{global_next, RwRandomLockError};
pub use skate2_formats::skater_profile::{ProfileMusicPlayer, ProfileSettings, MUSIC_PLAYER_PLAYLISTS};

/// `MusicSystem::eMusicMode`.
pub mod music_mode {
    pub const NONE: i32 = 2;
    pub const ENVIRONMENT: i32 = 4;
    pub const IPOD: i32 = 8;
}
/// `MusicSystem::eIPodTrigger`.
pub mod ipod_trigger {
    pub const START: i32 = 1;
    pub const RESTART: i32 = 3;
    pub const STOP: i32 = 4;
}
/// `ProfileBackEnd::eMusicMode`.
pub mod profile_music_mode {
    pub const OFF: i32 = 0;
    pub const WORLD: i32 = 1;
    pub const MUSIC_PLAYER: i32 = 2;
}

/// `rw::math::Random`'s global generator was unavailable to `ShufflePlaylist`.
#[derive(Debug)]
pub struct RandomError;
impl std::fmt::Display for RandomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("rw::math::Random lock poisoned")
    }
}
impl std::error::Error for RandomError {}
impl From<RwRandomLockError> for RandomError {
    fn from(_: RwRandomLockError) -> Self {
        Self
    }
}

/// A `MusicSystem` call, in the order the front end made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicRequest {
    /// `MusicSystem::SetDefaultMode`, a `music_mode` value.
    SetDefaultMode(i32),
    /// `MusicSystem::IPodTrigger(trigger, song, 0, -1)`.
    IPodTrigger { trigger: i32, song: u32 },
}

/// `FrontEndManager::mMusicPlayerMan` and the `tMusicPlayer` it edits.
#[derive(Clone, Debug)]
pub struct Manager {
    pub profile: ProfileMusicPlayer,
    /// `MusicCore::GetMusicPlayerSongIDFromIndex` for each index below `GetNumMusicPlayerSongs`.
    catalogue: Vec<u32>,
    /// `MusicSystem::GetCurrentMusicPlayerSongID` as the audio system last reported it.
    current_song: u32,
    requests: Vec<MusicRequest>,
    /// The songs `Play` handed `FrontEndManager::mChyron.Display`.
    chyron: Vec<u32>,
}
impl Manager {
    #[must_use]
    pub fn new(profile: ProfileMusicPlayer, catalogue: Vec<u32>) -> Self {
        Self { profile, catalogue, current_song: 0, requests: Vec::new(), chyron: Vec::new() }
    }
    pub fn take_chyron(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.chyron)
    }
    #[must_use]
    pub fn catalogue(&self) -> &[u32] {
        &self.catalogue
    }
    #[must_use]
    pub const fn current_song(&self) -> u32 {
        self.current_song
    }
    pub fn set_current_song(&mut self, song: u32) {
        self.current_song = song;
    }
    pub fn take_requests(&mut self) -> Vec<MusicRequest> {
        std::mem::take(&mut self.requests)
    }
    /// `ProfileBackEnd::GetPlaylist` truncated to `GetNumSongsInPlaylist`.
    #[must_use]
    pub fn playlist(&self, playlist: i32) -> &[u32] {
        let Some(p) = usize::try_from(playlist).ok().and_then(|i| self.profile.playlists.get(i)) else {
            return &[];
        };
        &p.songs[..usize::try_from(p.num_songs).unwrap_or(0).min(p.songs.len())]
    }
    /// The current playlist's songs, the `GetNextSongInCurrentPlaylist` view.
    #[must_use]
    pub fn current_playlist(&self) -> &[u32] {
        self.playlist(self.profile.current_playlist)
    }
    /// `MusicPlayerManager::GetSongIDFromIndex` 0x82629e70.
    #[must_use]
    pub fn song_id_from_index(&self, index: i32) -> u32 {
        usize::try_from(index).ok().and_then(|i| self.catalogue.get(i)).copied().unwrap_or(0)
    }
    fn songs_mut(&mut self, playlist: i32) -> Option<&mut skate2_formats::skater_profile::MusicPlayerPlaylist> {
        usize::try_from(playlist).ok().and_then(|i| self.profile.playlists.get_mut(i))
    }
    fn position(&self, playlist: i32, song: u32) -> Option<usize> {
        self.playlist(playlist).iter().position(|&s| s == song)
    }
    /// `ProfileBackEnd::SetCurrentPlaylist` 0x8255c198.
    fn set_current_playlist(&mut self, playlist: i32) {
        self.profile.current_playlist = playlist;
        self.profile.current_song_index = 0;
    }
    fn trigger(&mut self, trigger: i32, song: u32) {
        self.requests.push(MusicRequest::IPodTrigger { trigger, song });
    }
    /// `MusicPlayerManager::PlayPlaylist` 0x82629cd0.
    pub fn play_playlist(&mut self, playlist: i32, song_index: i32) {
        self.set_current_playlist(playlist);
        self.profile.current_song_index = 0;
        let raw = |m: &Self, i: usize| {
            usize::try_from(playlist).ok().and_then(|p| m.profile.playlists.get(p)).map_or(0, |p| p.songs[i])
        };
        let mut song = raw(self, 0);
        if song_index >= 0 && (song_index as usize) < self.playlist(playlist).len() {
            self.profile.current_song_index = song_index;
            song = raw(self, song_index as usize);
        }
        self.trigger(ipod_trigger::START, song);
    }
    /// `MusicPlayerManager::Play` 0x82639b00: restarts the playing song and shows it on the
    /// chyron, else starts the song, then indexes it in the current playlist.
    pub fn play(&mut self, song: u32) {
        if song == self.current_song {
            self.trigger(ipod_trigger::RESTART, song);
            self.chyron.push(song);
        } else {
            self.trigger(ipod_trigger::START, song);
        }
        self.profile.current_song_index =
            self.position(self.profile.current_playlist, song).map_or(-1, |i| i as i32);
    }
    /// `MusicPlayerManager::SwitchMusicMode` 0x82629fc8.
    pub fn switch_music_mode(&mut self, mode: i32) {
        match mode {
            profile_music_mode::WORLD => self.requests.push(MusicRequest::SetDefaultMode(music_mode::ENVIRONMENT)),
            profile_music_mode::MUSIC_PLAYER => {
                self.requests.push(MusicRequest::SetDefaultMode(music_mode::IPOD));
                self.play_playlist(self.profile.current_playlist, self.profile.current_song_index);
            }
            profile_music_mode::OFF => self.requests.push(MusicRequest::SetDefaultMode(music_mode::NONE)),
            _ => {}
        }
        self.profile.music_mode = mode;
    }
    /// `MusicPlayerManager::RevertMusicMode` 0x8262a088: the music system's default mode back to
    /// the profile's, without `SwitchMusicMode`'s playlist start.
    pub fn revert_music_mode(&mut self) {
        let mode = match self.profile.music_mode {
            profile_music_mode::WORLD => music_mode::ENVIRONMENT,
            profile_music_mode::MUSIC_PLAYER => music_mode::IPOD,
            profile_music_mode::OFF => music_mode::NONE,
            _ => return,
        };
        self.requests.push(MusicRequest::SetDefaultMode(mode));
    }
    /// `MusicPlayerManager::ShufflePlaylist` 0x82607aa0: a hundred `rw::math::Random` swaps,
    /// then the profile's song index follows the song it named.
    pub fn shuffle_playlist(&mut self, playlist: i32) -> Result<(), RandomError> {
        let count = self.playlist(playlist).len() as u32;
        let index = self.profile.current_song_index;
        let Some(p) = self.songs_mut(playlist) else { return Ok(()) };
        let kept = usize::try_from(index).ok().map_or(0, |i| p.songs[i.min(p.songs.len() - 1)]);
        if count != 0 {
            for _ in 0..100 {
                let a = (global_next()? % count) as usize;
                let b = (global_next()? % count) as usize;
                p.songs.swap(a, b);
            }
        }
        self.profile.current_song_index = 0;
        if let Some(i) = self.position(playlist, kept) {
            self.profile.current_song_index = i as i32;
        }
        Ok(())
    }
    /// `MusicPlayerManager::SortPlaylist` 0x8262fb40: the playlist's songs in catalogue order.
    pub fn sort_playlist(&mut self, playlist: i32) {
        let count = self.playlist(playlist).len();
        let catalogue = self.catalogue.clone();
        let Some(p) = self.songs_mut(playlist) else { return };
        let mut cursor = 0;
        for song in catalogue {
            if let Some(found) = p.songs[..count].iter().position(|&s| s == song) {
                p.songs.swap(cursor, found);
                cursor += 1;
            }
        }
    }
    /// `MusicPlayerManager::SetIsShuffle` 0x8262fc28.
    pub fn set_is_shuffle(&mut self, shuffle: bool) -> Result<(), RandomError> {
        if self.profile.shuffle == shuffle {
            return Ok(());
        }
        for playlist in 0..MUSIC_PLAYER_PLAYLISTS as i32 {
            if shuffle {
                self.shuffle_playlist(playlist)?;
            } else {
                self.sort_playlist(playlist);
            }
        }
        self.profile.current_song_index =
            self.position(self.profile.current_playlist, self.current_song).map_or(-1, |i| i as i32);
        self.profile.shuffle = shuffle;
        Ok(())
    }
    /// PC port (`custom_songs`): a custom song joins the catalogue, after the custom songs it
    /// already holds and before the game's songs, and the current playlist while it has room: in
    /// the catalogue's order (`SortPlaylist`), or last in a shuffled one.
    pub fn add_custom_song(&mut self, song: u32) {
        if !self.catalogue.contains(&song) {
            let at = self.catalogue.iter().position(|&held| !custom_songs::is_custom(held)).unwrap_or(self.catalogue.len());
            self.catalogue.insert(at, song);
        }
        let playlist = self.profile.current_playlist;
        let count = self.playlist(playlist).len();
        if self.position(playlist, song).is_none()
            && let Some(p) = self.songs_mut(playlist)
            && count < p.songs.len()
        {
            p.songs[count] = song;
            p.num_songs = count as i32 + 1;
            if !self.profile.shuffle {
                self.sort_playlist(playlist);
                self.profile.current_song_index = self.position(playlist, self.current_song).map_or(self.profile.current_song_index, |i| i as i32);
            }
        }
    }
    /// PC port: a custom song leaves the catalogue and every playlist. A playlist it leaves empty
    /// takes the game's songs again; when it was playing, the current playlist's first song plays.
    pub fn remove_custom_song(&mut self, song: u32) {
        self.catalogue.retain(|&held| held != song);
        let game: Vec<u32> = self.catalogue.iter().copied().filter(|&held| !custom_songs::is_custom(held)).collect();
        for playlist in 0..MUSIC_PLAYER_PLAYLISTS as i32 {
            let before = self.playlist(playlist).len();
            let mut list: Vec<u32> = self.playlist(playlist).iter().copied().filter(|&held| held != song).collect();
            if list.len() == before {
                continue;
            }
            if list.is_empty() {
                list.clone_from(&game);
            }
            if let Some(p) = self.songs_mut(playlist) {
                let count = list.len().min(p.songs.len());
                p.songs.fill(0);
                p.songs[..count].copy_from_slice(&list[..count]);
                p.num_songs = count as i32;
            }
        }
        if self.current_song == song {
            if let Some(&first) = self.current_playlist().first() {
                self.play(first);
            }
        } else {
            self.profile.current_song_index =
                self.position(self.profile.current_playlist, self.current_song).map_or(-1, |i| i as i32);
        }
    }
    /// `FrontEndState_MusicPlayer::IsSongInPlayList` 0x8265c020.
    #[must_use]
    pub fn is_song_in_playlist(&self, playlist: i32, song_index: i32) -> bool {
        self.position(playlist, self.song_id_from_index(song_index)).is_some()
    }
    /// `FrontEndState_MusicPlayer::ToggleSongInPlayList` 0x8265c0d8: never empties a playlist;
    /// dropping the playing song plays the first song left.
    pub fn toggle_song_in_playlist(&mut self, playlist: i32, song_index: i32) {
        let song = self.song_id_from_index(song_index);
        let count = self.playlist(playlist).len();
        let found = self.position(playlist, song);
        if count == 1 && found.is_some() {
            return;
        }
        let Some(p) = self.songs_mut(playlist) else { return };
        let Some(found) = found else {
            if count < p.songs.len() {
                p.songs[count] = song;
                p.num_songs = count as i32 + 1;
            }
            return;
        };
        p.songs[found] = p.songs[count - 1];
        p.songs[count - 1] = 0;
        p.num_songs = count as i32 - 1;
        let next = p.songs[..count].iter().copied().find(|&s| s != 0);
        if self.current_song == song {
            if let Some(next) = next {
                self.play(next);
            }
        }
    }
}

/// `eSk8FEMusicPlayerScreens`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Ambient,
    On,
}
/// The movie's `mState`: `kState_Ambient`, `kState_Player`, `kState_Edit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AptState {
    #[default]
    Ambient,
    Player,
    Edit,
}
impl AptState {
    const fn index(self) -> usize {
        self as usize
    }
}
/// `eSk8FEMusicPlayerControls`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    PlayList,
    Shuffle,
    Volume,
}
/// `gMusicPlayerScreens` 0x82fdf95c.
const SCREENS: [&[Control]; 2] = [&[Control::Volume], &[Control::PlayList, Control::Shuffle, Control::Volume]];
/// `kPlayListSize`.
pub const PLAY_LIST_SIZE: usize = 7;
/// `kEditListSize`.
pub const EDIT_LIST_SIZE: usize = 11;

/// What the movie shows: its `SetState` label, selection, and the fields its `Update*` methods
/// last wrote.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub visible: bool,
    pub state: AptState,
    pub selected: usize,
    /// `mPlayListFirstIndex`, the edit list's first catalogue row.
    pub first_index: usize,
    /// `mPlayListId`: the current playlist plus one.
    pub playlist_number: i32,
    pub shuffle: bool,
    /// `UpdateVolume`'s `Math.round(GetMusicVolume() * 10)`.
    pub volume: i32,
    /// `playItem0..6`: the songs `UpdateCurrentPlayList` shows and which one has the speaker.
    pub play_items: [Option<u32>; PLAY_LIST_SIZE],
    pub speaker: Option<usize>,
    /// `listItem0..10`: catalogue songs and their checkboxes.
    pub edit_items: [Option<(u32, bool)>; EDIT_LIST_SIZE],
    /// `mDefaultSongList.length`, the scroller's item count.
    pub catalogue_len: usize,
    /// PC port: the edit list's highlighted song is a custom song (Remove Song).
    pub selected_custom: bool,
}

/// PC port: what the screen did about custom songs since the last take.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomSongChanges {
    /// Add Song was pressed.
    pub add: bool,
    /// The songs Remove Song took out of the music player.
    pub removed: Vec<u32>,
    /// A playlist's songs changed.
    pub playlists: bool,
}

#[derive(Default)]
pub struct Controller {
    view: Snapshot,
    screen: Screen,
    edit: bool,
    reorder: bool,
    /// `mbIsDirty`, read by `FrontEndState_Options::RequestExit` 0x82681700.
    dirty: bool,
    /// `mCurrentIndex`, written by `OnMenuHilight` 0x826438e8.
    current_index: i32,
    num_playlists: i32,
    /// The movie's `mSelectedIndex`, one per state.
    selected: [usize; 3],
    sounds: Vec<&'static str>,
    /// `IsScreenReady` 0x82d661a8: input reaches the state once its screen's `IntroComplete`
    /// (`OnScreenReady` 0x82d65f98) ran.
    ready: bool,
    /// An Off profile's `UpdateMusicMode(1)`, which `OnLoaded` runs once the screen links.
    world_on_load: bool,
    /// PC port: Add Song, Remove Song and playlist changes the session has not taken yet.
    custom: CustomSongChanges,
}
impl Controller {
    #[must_use]
    pub fn snapshot(&self) -> &Snapshot {
        &self.view
    }
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.view.visible
    }
    /// Takes `mbIsDirty`, as `FrontEndState_Options::RequestExit` clears it.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }
    /// PC port: Add Song, Remove Song and playlist changes since the last call.
    pub fn take_custom_song_changes(&mut self) -> CustomSongChanges {
        std::mem::take(&mut self.custom)
    }
    /// PC port: a sound of the screen's that the session asks for (Add Song's refusals).
    pub fn play_sound(&mut self, sound: &'static str) {
        self.sounds.push(sound);
    }
    /// PC port: Add Song put a song in the catalogue and the current playlist.
    pub fn song_added(&mut self, manager: &Manager) {
        self.catalogue_changed(manager);
        if self.view.state == AptState::Player {
            self.refresh_playlist(manager);
        }
    }
    /// PC port: the catalogue grew or shrank: the edit list keeps to it.
    fn catalogue_changed(&mut self, manager: &Manager) {
        self.view.catalogue_len = manager.catalogue().len();
        if self.view.state == AptState::Edit {
            let max = self.max_scroll();
            self.view.first_index = self.view.first_index.min(self.view.catalogue_len.saturating_sub(max));
            let selected = self.selected[AptState::Edit.index()].min(max.saturating_sub(1));
            self.set_selection(selected);
            self.update_edited_playlist(manager);
        }
    }
    /// The screen's `IntroComplete`: `OnScreenReady` makes it ready for input.
    pub fn acknowledge_intro(&mut self) {
        if self.visible() {
            self.ready = true;
        }
    }
    /// `OnEnter` 0x826438c8, then the screen the movie's constructor and `OnLoaded` and the native
    /// `OnLoaded` 0x8266a968 show once the screen manager links it, which no input precedes.
    pub fn open(&mut self, manager: &mut Manager, settings: &ProfileSettings) {
        *self = Self { sounds: std::mem::take(&mut self.sounds), ..Self::default() };
        self.view.visible = true;
        self.view.catalogue_len = manager.catalogue().len();
        match manager.profile.music_mode {
            profile_music_mode::MUSIC_PLAYER => {
                self.screen = Screen::On;
                self.set_state(AptState::Player, manager, settings);
                self.refresh_playlist(manager);
            }
            mode => {
                self.world_on_load = mode != profile_music_mode::WORLD;
                self.screen = Screen::Ambient;
                self.set_state(AptState::Ambient, manager, settings);
            }
        }
        self.num_playlists = manager.profile.num_playlists;
    }
    /// The screen linked: `Game.OnScreenLoaded` runs `OnLoaded` 0x8266a968, whose
    /// `UpdateMusicMode(1)` moves an Off profile to the world's music.
    pub fn loaded(&mut self, manager: &mut Manager) {
        if std::mem::take(&mut self.world_on_load) {
            self.update_music_mode(manager, profile_music_mode::WORLD);
        }
    }
    /// `StateManagerFrontEndBase::PopState`: the outro, back to Options.
    pub fn close(&mut self) {
        self.view.visible = false;
    }
    /// `ReceiveInput` 0x8266aa00 first; keys it leaves go to the movie's `UpdateInput`.
    /// True pops the state.
    pub fn update(
        &mut self,
        input: PauseMenuInput,
        manager: &mut Manager,
        settings: &mut ProfileSettings,
    ) -> Result<bool, RandomError> {
        let result = self.receive_input(input, manager, settings);
        self.view.selected_custom =
            self.view.state == AptState::Edit && custom_songs::is_custom(manager.song_id_from_index(self.current_index));
        result
    }
    fn receive_input(
        &mut self,
        input: PauseMenuInput,
        manager: &mut Manager,
        settings: &mut ProfileSettings,
    ) -> Result<bool, RandomError> {
        if !self.visible() || !self.ready {
            return Ok(false);
        }
        if input.cancel {
            if !self.edit {
                return Ok(true);
            }
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            self.enable_edit_mode(false, self.screen, manager, settings)?;
            return Ok(false);
        }
        if input.select {
            if self.screen == Screen::On {
                self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                if self.edit {
                    manager.play(manager.song_id_from_index(self.current_index));
                } else {
                    self.enable_edit_mode(true, self.screen, manager, settings)?;
                }
            }
            return Ok(false);
        }
        // PC port: Remove Song (Y) on a custom song of the edit list.
        if input.option && self.edit {
            let song = manager.song_id_from_index(self.current_index);
            if custom_songs::is_custom(song) {
                self.sounds.push("FE_SOUND_CORE_Y_BUTTON");
                manager.remove_custom_song(song);
                self.custom.removed.push(song);
                self.custom.playlists = true;
                self.reorder = true;
                self.dirty = true;
                self.catalogue_changed(manager);
            }
            return Ok(false);
        }
        if input.misc && self.screen == Screen::On {
            if self.edit {
                self.sounds.push("FE_SOUND_CORE_X_BUTTON");
                manager.toggle_song_in_playlist(manager.profile.current_playlist, self.current_index);
                self.reorder = true;
                self.dirty = true;
                self.custom.playlists = true;
            } else {
                // PC port: Add Song (X) on the player screen.
                self.sounds.push("FE_SOUND_CORE_X_BUTTON");
                self.custom.add = true;
            }
            // ReceiveInput returns false: AptMisc reaches UpdateInput.
            if self.view.state == AptState::Edit {
                let selected = self.selected[AptState::Edit.index()];
                self.update_toggled_song(manager, self.view.first_index + selected, selected);
            }
            return Ok(false);
        }
        if (input.left || input.right) && !self.edit {
            self.adjust(input.right, manager, settings)?;
            return Ok(false);
        }
        if input.left_shift {
            let mut sound = false;
            if self.edit {
                self.enable_edit_mode(false, Screen::Ambient, manager, settings)?;
                sound = true;
            } else if self.screen != Screen::Ambient {
                if self.update_music_mode(manager, profile_music_mode::WORLD) {
                    self.screen = Screen::Ambient;
                    self.set_state(AptState::Ambient, manager, settings);
                }
                sound = true;
            }
            if sound {
                self.sounds.push("FE_SOUND_CORE_TAB");
            }
            return Ok(false);
        }
        if input.right_shift {
            let mut sound = false;
            if self.edit {
                self.enable_edit_mode(false, Screen::On, manager, settings)?;
                sound = true;
            } else if self.screen != Screen::On {
                if self.update_music_mode(manager, profile_music_mode::MUSIC_PLAYER) {
                    self.update_playlist_name(manager, manager.profile.current_playlist);
                    self.screen = Screen::On;
                    self.set_state(AptState::Player, manager, settings);
                    self.refresh_playlist(manager);
                }
                sound = true;
            }
            if sound {
                self.sounds.push("FE_SOUND_CORE_TAB");
            }
            return Ok(false);
        }
        if input.up {
            self.scroll_up(manager);
        } else if input.down {
            self.scroll_down(manager);
        }
        Ok(false)
    }
    /// `ReceiveInput`'s left/right over `gMusicPlayerScreens[mScreenMode][mCurrentIndex]`.
    fn adjust(&mut self, right: bool, manager: &mut Manager, settings: &mut ProfileSettings) -> Result<(), RandomError> {
        let step: i32 = if right { 1 } else { -1 };
        let screen = SCREENS[self.screen as usize];
        let Some(&control) = usize::try_from(self.current_index).ok().and_then(|i| screen.get(i)) else { return Ok(()) };
        let sound = if right { "FE_SOUND_CORE_MUSIC_VOLUME_RIGHT" } else { "FE_SOUND_CORE_MUSIC_VOLUME_LEFT" };
        match control {
            Control::PlayList => {
                if !self.update_playlist_name(manager, manager.profile.current_playlist + step) {
                    return Ok(());
                }
                manager.play_playlist(manager.profile.current_playlist, 0);
            }
            Control::Shuffle => manager.set_is_shuffle(!manager.profile.shuffle)?,
            Control::Volume => {
                self.sounds.push(sound);
                let volume = (step as f32 * 0.1) + settings.music_volume;
                settings.music_volume = if volume < 0.0 { 0.0 } else if volume > 1.0 { 1.0 } else { volume };
                self.refresh_current_item(manager, settings);
                self.dirty = true;
                return Ok(());
            }
        }
        self.sounds.push(sound);
        self.refresh_current_item(manager, settings);
        self.refresh_playlist(manager);
        self.dirty = true;
        Ok(())
    }
    /// `UpdatePlayListName` 0x826438f0: wraps, and is false when the playlist would not change.
    fn update_playlist_name(&mut self, manager: &mut Manager, index: i32) -> bool {
        if self.num_playlists <= 0 {
            return false;
        }
        let mut index = index;
        if index < 0 {
            index = self.num_playlists - 1;
        }
        if index >= self.num_playlists {
            index = 0;
        }
        if index == manager.profile.current_playlist {
            return false;
        }
        manager.set_current_playlist(index);
        true
    }
    /// `UpdateMusicMode` 0x8265c2f0.
    fn update_music_mode(&mut self, manager: &mut Manager, mode: i32) -> bool {
        if mode == manager.profile.music_mode {
            return false;
        }
        manager.switch_music_mode(mode);
        self.dirty = true;
        true
    }
    /// `EnableEditMode` 0x8265c370.
    fn enable_edit_mode(
        &mut self,
        edit: bool,
        screen: Screen,
        manager: &mut Manager,
        settings: &ProfileSettings,
    ) -> Result<(), RandomError> {
        self.edit = edit;
        if edit {
            self.set_state(AptState::Edit, manager, settings);
            return Ok(());
        }
        if self.reorder {
            let playlist = manager.profile.current_playlist;
            if manager.profile.shuffle {
                manager.shuffle_playlist(playlist)?;
            } else {
                manager.sort_playlist(playlist);
            }
            self.reorder = false;
        }
        if screen == Screen::Ambient {
            self.update_music_mode(manager, profile_music_mode::WORLD);
        }
        self.screen = screen;
        self.set_state(if screen == Screen::On { AptState::Player } else { AptState::Ambient }, manager, settings);
        if screen == Screen::On {
            self.refresh_playlist(manager);
        }
        Ok(())
    }
    /// `RefreshCurrentItem` 0x8265c288: `kState_RefreshPlayer` / `kState_RefreshAmbient`.
    fn refresh_current_item(&mut self, manager: &Manager, settings: &ProfileSettings) {
        match self.screen {
            Screen::On => self.update_player_screen(manager, settings),
            Screen::Ambient => self.update_volume(settings),
        }
    }

    /// The movie's `SetState` for the three screens: `ChangeState`, the label, `UpdateMode`,
    /// the screen's `Update*Screen` and its button help.
    fn set_state(&mut self, state: AptState, manager: &Manager, settings: &ProfileSettings) {
        self.selected[state.index()] = 0;
        self.view.state = state;
        self.view.selected = 0;
        self.current_index = 0;
        match state {
            AptState::Player => self.update_player_screen(manager, settings),
            AptState::Ambient => self.update_volume(settings),
            AptState::Edit => {
                self.view.first_index = 0;
                self.update_edited_playlist(manager);
            }
        }
    }
    /// `UpdatePlayerScreen`: the playlist title, shuffle and volume.
    fn update_player_screen(&mut self, manager: &Manager, settings: &ProfileSettings) {
        self.view.playlist_number = manager.profile.current_playlist + 1;
        self.view.shuffle = manager.profile.shuffle;
        self.update_volume(settings);
    }
    fn update_volume(&mut self, settings: &ProfileSettings) {
        self.view.volume = (settings.music_volume * 10.0).round() as i32;
    }
    /// `kState_RefreshPlayList`: `UpdateCurrentPlayList`'s seven-row window over the playlist,
    /// the speaker on the current song.
    fn refresh_playlist(&mut self, manager: &Manager) {
        let playlist = manager.playlist(self.view.playlist_number - 1);
        let len = playlist.len() as i32;
        let mut current = manager.profile.current_song_index;
        let size = PLAY_LIST_SIZE as i32;
        self.view.play_items = [None; PLAY_LIST_SIZE];
        let mut row: i32;
        if current == -1 {
            self.view.play_items[0] = Some(manager.current_song());
            self.view.speaker = Some(0);
            row = 1;
            current = 0;
        } else {
            let remaining = len - current;
            let speaker = if remaining < size {
                let speaker = size - remaining;
                if len < size {
                    let speaker = current;
                    current = 0;
                    speaker
                } else {
                    current = len - size;
                    speaker
                }
            } else {
                0
            };
            self.view.speaker = usize::try_from(speaker).ok().filter(|&s| s < PLAY_LIST_SIZE);
            row = 0;
        }
        while row < size {
            if current >= 0 && current < len {
                self.view.play_items[row as usize] = Some(playlist[current as usize]);
                current += 1;
            }
            row += 1;
        }
    }
    /// `UpdateEditedPlayList`: eleven catalogue rows from `mPlayListFirstIndex`.
    fn update_edited_playlist(&mut self, manager: &Manager) {
        let playlist = manager.profile.current_playlist;
        for (i, item) in self.view.edit_items.iter_mut().enumerate() {
            let index = self.view.first_index + i;
            *item = manager
                .catalogue()
                .get(index)
                .map(|&song| (song, manager.is_song_in_playlist(playlist, index as i32)));
        }
    }
    /// `UpdateToggledSong(songIndex, menuIndex)`.
    fn update_toggled_song(&mut self, manager: &Manager, song_index: usize, menu_index: usize) {
        let checked = manager.is_song_in_playlist(manager.profile.current_playlist, song_index as i32);
        if let Some(Some((_, flag))) = self.view.edit_items.get_mut(menu_index) {
            *flag = checked;
        }
    }
    /// `GetMaxScroll`.
    fn max_scroll(&self) -> usize {
        match self.view.state {
            AptState::Ambient => 1,
            AptState::Player => 3,
            AptState::Edit => self.view.catalogue_len.min(EDIT_LIST_SIZE),
        }
    }
    /// The movie's `SetSelection`: `OnMenuHilight` with the catalogue row in the edit list.
    fn set_selection(&mut self, index: usize) {
        self.selected[self.view.state.index()] = index;
        self.view.selected = index;
        let first = if self.view.state == AptState::Edit { self.view.first_index } else { 0 };
        self.current_index = (index + first) as i32;
    }
    /// `ScrollUp`: the edit list scrolls from its top row.
    fn scroll_up(&mut self, manager: &Manager) {
        let state = self.view.state;
        let selected = self.selected[state.index()];
        if state == AptState::Edit && selected == 0 && self.view.first_index > 0 {
            self.view.first_index -= 1;
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
            self.update_edited_playlist(manager);
            self.current_index = self.view.first_index as i32;
        }
        let index = selected.saturating_sub(1);
        if index != selected {
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
            self.set_selection(index);
        }
    }
    /// `ScrollDown`: the edit list scrolls from its bottom row.
    fn scroll_down(&mut self, manager: &Manager) {
        let state = self.view.state;
        let max = self.max_scroll();
        let selected = self.selected[state.index()];
        if state == AptState::Edit && selected + 1 == max && self.view.first_index + 1 + max <= self.view.catalogue_len {
            self.view.first_index += 1;
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
            self.update_edited_playlist(manager);
            self.current_index = (self.view.first_index + max - 1) as i32;
        }
        let index = (selected + 1).min(max.saturating_sub(1));
        if index != selected {
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
            self.set_selection(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager(mode: i32) -> Manager {
        let catalogue: Vec<u32> = (1..=20).map(|i| i * 10).collect();
        let mut profile = ProfileMusicPlayer::reset(&catalogue);
        profile.music_mode = mode;
        Manager::new(profile, catalogue)
    }
    fn press(f: impl FnOnce(&mut PauseMenuInput)) -> PauseMenuInput {
        let mut input = PauseMenuInput::default();
        f(&mut input);
        input
    }

    #[test]
    fn opens_on_the_saved_mode_and_switches_tabs() {
        let settings = ProfileSettings::default();
        let mut m = manager(profile_music_mode::OFF);
        let mut c = Controller::default();
        c.open(&mut m, &settings);
        // OnLoaded waits for the screen: an Off profile then goes to the world's music.
        assert_eq!(m.profile.music_mode, profile_music_mode::OFF);
        assert!(m.take_requests().is_empty());
        c.loaded(&mut m);
        // Input waits for the screen's intro.
        assert!(!c.update(press(|i| i.cancel = true), &mut m, &mut settings.clone()).unwrap());
        assert!(c.visible());
        c.acknowledge_intro();
        assert_eq!(m.profile.music_mode, profile_music_mode::WORLD);
        assert_eq!(m.take_requests(), [MusicRequest::SetDefaultMode(music_mode::ENVIRONMENT)]);
        assert_eq!(c.snapshot().state, AptState::Ambient);
        let mut settings = settings;
        c.update(press(|i| i.right_shift = true), &mut m, &mut settings).unwrap();
        assert_eq!(c.snapshot().state, AptState::Player);
        assert_eq!(c.take_sounds(), ["FE_SOUND_CORE_TAB"]);
        assert_eq!(
            m.take_requests(),
            [MusicRequest::SetDefaultMode(music_mode::IPOD), MusicRequest::IPodTrigger { trigger: ipod_trigger::START, song: 10 }]
        );
        assert_eq!(c.snapshot().play_items[..3], [Some(10), Some(20), Some(30)]);
        assert_eq!(c.snapshot().speaker, Some(0));
        // A second RT on the player screen is silent.
        c.update(press(|i| i.right_shift = true), &mut m, &mut settings).unwrap();
        assert!(c.take_sounds().is_empty());
        assert!(c.take_dirty());
    }

    #[test]
    fn playlist_window_follows_native_rows() {
        let settings = ProfileSettings::default();
        let mut m = manager(profile_music_mode::MUSIC_PLAYER);
        m.profile.current_song_index = 17;
        let mut c = Controller::default();
        c.open(&mut m, &settings);
        c.acknowledge_intro();
        // 20 songs, index 17: the window starts at 13, the speaker on row 4.
        assert_eq!(c.snapshot().play_items[0], Some(140));
        assert_eq!(c.snapshot().speaker, Some(4));
    }

    #[test]
    fn edit_toggles_and_sorts_on_leave() {
        let mut settings = ProfileSettings::default();
        let mut m = manager(profile_music_mode::MUSIC_PLAYER);
        let mut c = Controller::default();
        c.open(&mut m, &settings);
        c.acknowledge_intro();
        c.update(press(|i| i.select = true), &mut m, &mut settings).unwrap();
        assert_eq!(c.snapshot().state, AptState::Edit);
        assert_eq!(c.snapshot().edit_items[0], Some((10, true)));
        c.update(press(|i| i.down = true), &mut m, &mut settings).unwrap();
        c.update(press(|i| i.misc = true), &mut m, &mut settings).unwrap();
        assert_eq!(c.snapshot().edit_items[1], Some((20, false)));
        assert_eq!(m.current_playlist().len(), 19);
        // Re-adding appends; leaving edit sorts back to catalogue order.
        c.update(press(|i| i.misc = true), &mut m, &mut settings).unwrap();
        assert_eq!(*m.current_playlist().last().unwrap(), 20);
        c.update(press(|i| i.cancel = true), &mut m, &mut settings).unwrap();
        assert_eq!(c.snapshot().state, AptState::Player);
        assert_eq!(m.current_playlist()[..3], [10, 20, 30]);
        // The edit list scrolls from its last row.
        c.update(press(|i| i.select = true), &mut m, &mut settings).unwrap();
        for _ in 0..12 {
            c.update(press(|i| i.down = true), &mut m, &mut settings).unwrap();
        }
        assert_eq!((c.snapshot().first_index, c.snapshot().selected), (2, 10));
        assert_eq!(c.current_index, 12);
    }

    #[test]
    fn add_song_is_asked_for_on_the_player_screen_and_a_custom_song_is_removed_in_the_edit_list() {
        const MINE: u32 = 0xCC00_0001;
        const OTHER: u32 = 0xCC00_0002;
        let mut settings = ProfileSettings::default();
        let mut m = manager(profile_music_mode::MUSIC_PLAYER);
        let mut c = Controller::default();
        c.open(&mut m, &settings);
        c.acknowledge_intro();
        // X on the player screen asks for Add Song and changes nothing.
        c.update(press(|i| i.misc = true), &mut m, &mut settings).unwrap();
        let changes = c.take_custom_song_changes();
        assert!(changes.add && changes.removed.is_empty() && !changes.playlists);
        assert_eq!(m.current_playlist().len(), 20);
        // The added song comes before the game's in the catalogue and the current playlist, and a
        // second one after it; the song that plays keeps its index.
        m.set_current_song(20);
        m.profile.current_song_index = 1;
        m.add_custom_song(MINE);
        m.add_custom_song(OTHER);
        c.song_added(&m);
        assert_eq!((m.catalogue().len(), c.snapshot().catalogue_len), (22, 22));
        assert_eq!(m.catalogue()[..3], [MINE, OTHER, 10]);
        assert_eq!(m.current_playlist()[..4], [MINE, OTHER, 10, 20]);
        assert_eq!(m.profile.current_song_index, 3, "the index follows the song that plays");
        assert!(m.playlist(1).iter().all(|&song| song != MINE), "the other playlists do not take it");
        m.remove_custom_song(OTHER);
        m.take_requests();
        c.song_added(&m);
        // In the edit list the custom song's row is the first and offers Remove Song; a song of
        // the game's does not.
        c.update(press(|i| i.select = true), &mut m, &mut settings).unwrap();
        assert_eq!(c.snapshot().edit_items[0], Some((MINE, true)));
        assert!(c.snapshot().selected_custom);
        c.update(press(|i| i.down = true), &mut m, &mut settings).unwrap();
        assert!(!c.snapshot().selected_custom);
        c.update(press(|i| i.option = true), &mut m, &mut settings).unwrap();
        assert_eq!(m.catalogue().len(), 21, "Y on a song of the game's does nothing");
        c.update(press(|i| i.up = true), &mut m, &mut settings).unwrap();
        assert!(c.snapshot().selected_custom);
        // X takes it out of the playlist, and puts it back.
        c.update(press(|i| i.misc = true), &mut m, &mut settings).unwrap();
        assert!(c.take_custom_song_changes().playlists);
        assert_eq!(m.current_playlist().len(), 20);
        c.update(press(|i| i.misc = true), &mut m, &mut settings).unwrap();
        assert_eq!(m.current_playlist().len(), 21);
        // Y removes it from the catalogue and every playlist; the list's first row is the game's.
        m.set_current_song(MINE);
        m.take_requests();
        c.update(press(|i| i.option = true), &mut m, &mut settings).unwrap();
        let changes = c.take_custom_song_changes();
        assert_eq!(changes.removed, [MINE]);
        assert_eq!((m.catalogue().len(), c.snapshot().catalogue_len), (20, 20));
        assert!(m.current_playlist().iter().all(|&song| song != MINE));
        assert_eq!((c.snapshot().first_index, c.snapshot().selected), (0, 0));
        assert_eq!(c.snapshot().edit_items[0], Some((10, true)));
        assert!(!c.snapshot().selected_custom);
        // It was playing: the playlist's first song plays (the song `ToggleSongInPlayList` moved
        // into the row it took the custom song from, until the list is sorted on leaving).
        assert_eq!(m.take_requests(), [MusicRequest::IPodTrigger { trigger: ipod_trigger::START, song: 200 }]);
        assert_eq!(m.profile.current_song_index, 0);
    }

    #[test]
    fn a_playlist_of_one_removed_custom_song_takes_the_games_songs_again() {
        const MINE: u32 = 0xCC00_0002;
        let mut m = manager(profile_music_mode::MUSIC_PLAYER);
        m.add_custom_song(MINE);
        let list = &mut m.profile.playlists[0];
        list.songs = [0; skate2_formats::skater_profile::MUSIC_PLAYER_SONGS];
        list.songs[0] = MINE;
        list.num_songs = 1;
        m.remove_custom_song(MINE);
        assert_eq!(m.current_playlist().len(), 20);
        assert_eq!(m.current_playlist()[..2], [10, 20]);
    }

    #[test]
    fn playlist_change_and_volume_clamp() {
        let mut settings = ProfileSettings::default();
        let mut m = manager(profile_music_mode::MUSIC_PLAYER);
        let mut c = Controller::default();
        c.open(&mut m, &settings);
        c.acknowledge_intro();
        m.take_requests();
        c.update(press(|i| i.left = true), &mut m, &mut settings).unwrap();
        assert_eq!(m.profile.current_playlist, 4);
        assert_eq!(c.snapshot().playlist_number, 5);
        assert_eq!(m.take_requests(), [MusicRequest::IPodTrigger { trigger: ipod_trigger::START, song: 10 }]);
        c.update(press(|i| i.down = true), &mut m, &mut settings).unwrap();
        c.update(press(|i| i.down = true), &mut m, &mut settings).unwrap();
        c.update(press(|i| i.right = true), &mut m, &mut settings).unwrap();
        assert_eq!(settings.music_volume, 1.0);
        c.update(press(|i| i.left = true), &mut m, &mut settings).unwrap();
        assert_eq!(c.snapshot().volume, 9);
    }
}
