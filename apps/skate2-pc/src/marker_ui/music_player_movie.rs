//! Original musicplayer/musicplayertest APT (`FrontEndState_MusicPlayer`, screen 35), fingerprint
//! 18056383. `mMusicPlayer`'s labels are the three screens; its controls are imported
//! highlight_slider/highlight_selector rows, a basic_slider scroller and a Button Menu.
//! Options' `OnMenuSelect` 0x826871a0 pushes the state (`PushState(30)`, not keeping Options):
//! Options' `OnLeave` closes its screen, the dimmer stays on (both states' `mbDimmer`), and the
//! screen manager links this screen once it loaded it; B pops it (`PopState`) and Options' screen
//! loads anew while this one plays its outro.
use super::{
    assets::Assets,
    clips::Clip,
    edit_skater_movie, localizer, map_movie,
    pause_movie::{bounds, child},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    edit_skater::EditSkaterRow,
    music_player::{AptState, Snapshot, EDIT_LIST_SIZE, PLAY_LIST_SIZE},
};

pub(super) const ASSET: usize = 49;

pub(super) struct MusicPlayerMovie {
    pub root: Clip,
    /// The state is on the stack (between its `OnEnter` and `OnLeave`).
    open: bool,
    /// `OnEnter`'s `OpenScreen(35)` waits for the screen manager to link the screen.
    pending: bool,
    /// The state's latest snapshot, which the screen shows once it exists.
    target: Snapshot,
    /// The snapshot the movie last showed.
    shown: Option<Snapshot>,
    /// `UpdateMode` runs for the player and ambient screens only.
    tabs: AptState,
    /// The screen linked since the last take: `Game.OnScreenLoaded` runs the state's `OnLoaded`.
    loaded: bool,
    /// The intro's `IntroComplete` since the last take.
    intro_complete: bool,
    /// The outro's `OutroComplete` since the last take.
    outro_complete: bool,
    /// Frame 1's `LetterBox.EnableLetterBox(false)` since the last take.
    letterbox_off: bool,
}

impl MusicPlayerMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self {
            root,
            open: false,
            pending: false,
            target: Snapshot::default(),
            shown: None,
            tabs: AptState::Ambient,
            loaded: false,
            intro_complete: false,
            outro_complete: false,
            letterbox_off: false,
        })
    }

    /// The state waits for its screen.
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// The screen linked since the last call.
    pub fn take_loaded(&mut self) -> bool {
        std::mem::take(&mut self.loaded)
    }

    /// The screen's `IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// The screen's `OutroComplete` since the last call.
    pub fn take_outro_complete(&mut self) -> bool {
        std::mem::take(&mut self.outro_complete)
    }

    /// Frame 1's `LetterBox.EnableLetterBox(false)` since the last call.
    pub fn take_letterbox_off(&mut self) -> bool {
        std::mem::take(&mut self.letterbox_off)
    }

    /// `OnEnter`'s screen comes when the screen manager links it, `OnLeave`'s `CloseScreen` plays
    /// the outro at once, and the shown screen takes what the state's `SetState` calls changed.
    pub fn update(&mut self, assets: &Assets, snapshot: &Snapshot) -> RenderResult<()> {
        self.target = snapshot.clone();
        if snapshot.visible && !self.open {
            self.open = true;
            self.shown = None;
            self.pending = true;
            return Ok(());
        }
        if !snapshot.visible && self.open {
            self.open = false;
            self.shown = None;
            if !std::mem::take(&mut self.pending) {
                self.root.goto(assets, "outro")?;
            }
            return Ok(());
        }
        let Some(previous) = self.shown.clone() else { return Ok(()) };
        if previous == *snapshot {
            return Ok(());
        }
        if previous.state == snapshot.state {
            self.refresh(assets, &previous, snapshot)?;
        } else {
            self.set_state(assets, snapshot)?;
        }
        self.shown = Some(snapshot.clone());
        Ok(())
    }

    /// `SetState` for a screen: its label, `UpdateMode`, every field, the selection and the
    /// button help.
    fn set_state(&mut self, assets: &Assets, snapshot: &Snapshot) -> RenderResult<()> {
        let label = match snapshot.state {
            AptState::Ambient => "ambient",
            AptState::Player => "musicplayer",
            AptState::Edit => "editplaylist",
        };
        let help = button_help(snapshot);
        if snapshot.state != AptState::Edit {
            self.tabs = snapshot.state;
        }
        let tabs = self.tabs;
        let player = self.root.named(&["mMusicPlayer"])?;
        player.goto(assets, label)?;
        player.named(&["mTabAmbient"])?.goto(assets, if tabs == AptState::Ambient { "on" } else { "off" })?;
        player.named(&["mTabMusicplayer"])?.goto(assets, if tabs == AptState::Player { "on" } else { "off" })?;
        map_movie::configure_button_items(player.named(&["mButtonHelp"])?, assets, &help, 0xffff_ffec)?;
        // ChangeState: every control and row unselected but the new state's first.
        for control in ["mSlider", "mPlaylist", "mShuffle"] {
            let selected = controls(snapshot).get(snapshot.selected) == Some(&control);
            self.highlight(assets, snapshot, control, selected)?;
        }
        if snapshot.state == AptState::Edit {
            for index in 0..EDIT_LIST_SIZE {
                self.select(assets, snapshot, index, index == snapshot.selected)?;
            }
        }
        self.play_items(assets, snapshot)?;
        self.edit_items(assets, snapshot)?;
        let player = self.root.named(&["mMusicPlayer"])?;
        player.named(&["mPlaylistNumber", "labelText"])?.text = Some(playlist_number(snapshot).encode_utf16().collect());
        Ok(())
    }

    /// The same screen: `SetSelection`'s `SelectItem` pair, then each refreshed field.
    fn refresh(&mut self, assets: &Assets, previous: &Snapshot, snapshot: &Snapshot) -> RenderResult<()> {
        if previous.selected != snapshot.selected {
            self.select(assets, snapshot, previous.selected, false)?;
            self.select(assets, snapshot, snapshot.selected, true)?;
        }
        let player = self.root.named(&["mMusicPlayer"])?;
        // PC port: Remove Song comes and goes with a custom song's highlight.
        if previous.selected_custom != snapshot.selected_custom {
            map_movie::configure_button_items(player.named(&["mButtonHelp"])?, assets, &button_help(snapshot), 0xffff_ffec)?;
        }
        if previous.volume != snapshot.volume {
            let selected = snapshot.state != AptState::Edit && controls(snapshot).get(snapshot.selected) == Some(&"mSlider");
            edit_skater_movie::slider_value(player.named(&["mSlider"])?, assets, &percent(assets, snapshot)?, snapshot.volume, selected)?;
        }
        if previous.playlist_number != snapshot.playlist_number {
            edit_skater_movie::selector_subtitle(player.named(&["mPlaylist"])?, assets, &playlist_number(snapshot))?;
            player.named(&["mPlaylistNumber", "labelText"])?.text = Some(playlist_number(snapshot).encode_utf16().collect());
        }
        if previous.shuffle != snapshot.shuffle {
            edit_skater_movie::selector_subtitle(player.named(&["mShuffle"])?, assets, &shuffle(assets, snapshot)?)?;
        }
        if previous.play_items != snapshot.play_items || previous.speaker != snapshot.speaker {
            self.play_items(assets, snapshot)?;
        }
        if previous.edit_items != snapshot.edit_items || previous.first_index != snapshot.first_index {
            self.edit_items(assets, snapshot)?;
        }
        Ok(())
    }

    /// `SelectItem`: a list row plays its label; a control row takes `SetState`.
    fn select(&mut self, assets: &Assets, snapshot: &Snapshot, index: usize, selected: bool) -> RenderResult<()> {
        if snapshot.state == AptState::Edit {
            let label = if selected { "selected" } else { "unselected" };
            return self.root.named(&["mMusicPlayer", &format!("listItem{index}")])?.goto(assets, label);
        }
        let Some(&control) = controls(snapshot).get(index) else { return Ok(()) };
        self.highlight(assets, snapshot, control, selected)
    }

    /// A control's `SetState`, with the title `OnLoaded` gave it and its current value.
    fn highlight(&mut self, assets: &Assets, snapshot: &Snapshot, control: &str, selected: bool) -> RenderResult<()> {
        let row = match control {
            "mSlider" => EditSkaterRow {
                kind: "slider".into(),
                title: assets.localize("ID_MUSIC_PLAYER_VOLUME")?,
                subtitle: percent(assets, snapshot)?,
                value: snapshot.volume,
                ..Default::default()
            },
            "mPlaylist" => EditSkaterRow {
                kind: "selector".into(),
                title: assets.localize("ID_MUSIC_PLAYER_PLAYLIST")?,
                subtitle: playlist_number(snapshot),
                ..Default::default()
            },
            _ => EditSkaterRow {
                kind: "selector".into(),
                title: assets.localize("ID_MUSIC_PLAYER_SHUFFLE")?,
                subtitle: shuffle(assets, snapshot)?,
                ..Default::default()
            },
        };
        edit_skater_movie::highlight(self.root.named(&["mMusicPlayer", control])?, assets, &row, selected)
    }

    /// `UpdateCurrentPlayList`: the speaker on one row and each row's song and artist.
    fn play_items(&mut self, assets: &Assets, snapshot: &Snapshot) -> RenderResult<()> {
        let player = self.root.named(&["mMusicPlayer"])?;
        for index in 0..PLAY_LIST_SIZE {
            let item = player.named(&[&format!("playItem{index}")])?;
            item.named(&["mSpeaker"])?.goto(assets, if snapshot.speaker == Some(index) { "on" } else { "off" })?;
            if let Some(song) = snapshot.play_items[index] {
                song_texts(item, assets, song)?;
            }
            item.visible = snapshot.play_items[index].is_some();
        }
        Ok(())
    }

    /// `UpdateEditedPlayList`: eleven catalogue rows, their checkboxes, and the scroller.
    fn edit_items(&mut self, assets: &Assets, snapshot: &Snapshot) -> RenderResult<()> {
        let player = self.root.named(&["mMusicPlayer"])?;
        for index in 0..EDIT_LIST_SIZE {
            let item = player.named(&[&format!("listItem{index}")])?;
            match snapshot.edit_items[index] {
                Some((song, checked)) => {
                    song_texts(item, assets, song)?;
                    item.named(&["mCheckbox"])?.goto(assets, if checked { "on" } else { "off" })?;
                    item.visible = true;
                }
                None => item.visible = false,
            }
        }
        // OnLoaded: SetSliderLength(listItem0._height * kEditListSize), SetNumberOfItems.
        let height = bounds(player.named(&["listItem0"])?, assets)?.map_or(0.0, |r| r.bottom - r.top);
        let scroller = child(player, "mScroller")?;
        map_movie::update_scrollbar(
            &mut scroller.clip,
            assets,
            snapshot.catalogue_len,
            EDIT_LIST_SIZE,
            snapshot.first_index,
            height * EDIT_LIST_SIZE as f32,
        )
    }

    /// The screen manager linked the movie: its constructor plays `intro`, then `Game.OnScreenLoaded`
    /// runs the native `OnLoaded` 0x8266a968, which sets the screen of the profile's music mode.
    pub fn link(&mut self, assets: &Assets) -> RenderResult<()> {
        if !std::mem::take(&mut self.pending) {
            return Ok(());
        }
        self.root = Clip::new(assets, ASSET, 0)?;
        self.root.visible = true;
        self.root.goto(assets, "intro")?;
        let target = self.target.clone();
        self.set_state(assets, &target)?;
        self.shown = Some(target);
        self.loaded = true;
        self.calls()
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if self.root.visible {
            self.root.tick(assets)?;
        }
        self.calls()
    }

    fn calls(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "MusicPlayerLetterBoxOff" => self.letterbox_off = true,
                "MusicPlayerIntroComplete" => self.intro_complete = true,
                "MusicPlayerOutroComplete" => {
                    self.root.visible = false;
                    self.outro_complete = true;
                }
                _ => return Err(format!("untranslated music player callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// The screen's button help, rightmost first. PC port: the player screen's Add Song (X), and
/// the edit list's Remove Song (Y) while a custom song is highlighted.
fn button_help(snapshot: &Snapshot) -> Vec<(&'static str, &'static str)> {
    let mut help = vec![("B", "ID_COMMON_BACK")];
    match snapshot.state {
        AptState::Ambient => {}
        AptState::Player => help.extend([("A", "ID_MUSIC_PLAYER_EDIT"), ("X", "#Add Song")]),
        AptState::Edit => {
            help.extend([("A", "ID_MUSIC_PLAYER_PLAY"), ("X", "ID_MUSIC_PLAYER_SONG_ONOFF")]);
            if snapshot.selected_custom {
                help.push(("Y", "#Remove Song"));
            }
        }
    }
    help
}

/// `GetSelectionItem`'s controls for the player and ambient screens.
fn controls(snapshot: &Snapshot) -> &'static [&'static str] {
    match snapshot.state {
        AptState::Ambient => &["mSlider"],
        AptState::Player => &["mPlaylist", "mShuffle", "mSlider"],
        AptState::Edit => &[],
    }
}

/// `UpdateVolume`: `ID_COMMON_PERCENT` of the rounded volume times ten.
fn percent(assets: &Assets, snapshot: &Snapshot) -> RenderResult<String> {
    let volume = localizer::AptArgument::Integer(snapshot.volume * 10);
    Ok(String::from_utf16_lossy(&localizer::localized_format_string(assets, "ID_COMMON_PERCENT", &[volume])?))
}

/// `'#0' + mPlayListId`, a literal.
fn playlist_number(snapshot: &Snapshot) -> String {
    format!("0{}", snapshot.playlist_number)
}

fn shuffle(assets: &Assets, snapshot: &Snapshot) -> RenderResult<String> {
    assets.localize(if snapshot.shuffle { "ID_COMMON_ON" } else { "ID_COMMON_OFF" })
}

/// `GetSongInfo`'s title in `songText` and artist in `artistText`; the strings are `#` literals.
fn song_texts(item: &mut Clip, assets: &Assets, song: u32) -> RenderResult<()> {
    // PC port: a custom song shows the names typed for it (literals, as the game's are).
    let (artist, title) = match skate2_core::ui::custom_songs::info(song) {
        Some((artist, title)) => (format!("#{artist}"), format!("#{title}")),
        None => {
            let (artist, title, _) = skate2_audio::system::AudioSystem::music_player_song_info(song, |id| {
                assets.localized.get(&id).map(|units| String::from_utf16_lossy(units))
            });
            (artist, title)
        }
    };
    let literal = |s: &str| s.strip_prefix('#').unwrap_or(s).encode_utf16().collect::<Vec<u16>>();
    item.named(&["songText"])?.text = Some(literal(&title));
    item.named(&["artistText"])?.text = Some(literal(&artist));
    Ok(())
}

/// The class body, translated by `MusicPlayerMovie`.
pub(super) fn init_action(asset: usize, _character: u32, offset: usize) -> bool {
    asset == ASSET && offset == 0x3b60
}

pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        // `var screen = new musicplayertest(this)` and its class; MusicPlayerMovie owns both.
        0x3b3c | 0x3b60 => {}
        // Frame 1: ScreenManager.EnableDimmer(true), a method screenmanager.apt never defines
        // (the FE states' mbDimmer drives the dimmer), then LetterBox.EnableLetterBox(false),
        // whose PlayOutro replays the bars' niceoutro from its first frame.
        0x4b90 => clip.calls.push("MusicPlayerLetterBoxOff"),
        0x4bb4 => {
            clip.playback.stop();
            clip.calls.push("MusicPlayerIntroComplete");
        }
        // Frame 15: ScreenManager.EnableDimmer(false), undefined as well.
        0x4bd8 => {}
        0x4bf4 => {
            clip.playback.stop();
            clip.calls.push("MusicPlayerOutroComplete");
        }
        _ => return Err(format!("untranslated music player APT action {offset:#x}").into()),
    }
    Ok(())
}
