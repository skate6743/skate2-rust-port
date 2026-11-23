//! main.swf's now-playing layer, source/screens/main/chyron (asset 50, fingerprint ce2ea7f1).
//! main.apt loads it into `_root.gChyron`, the `ScreenLayer` clip made after `gLetterBox`: above
//! the screen manager's screens and the letterbox, under `gSubtitle` and `gPopups`. The
//! InitAction (character 21, `0xd04`) defines the `chyron` class translated here and root
//! action `0xce4` constructs it; `FE::Chyron` decides what it shows.
use super::{assets::Assets, clips::Clip, edit_skater_movie::autosize_text, pause_movie::bounds};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::chyron::{Call, Chyron};

pub(super) const ASSET: usize = 50;

pub(super) struct ChyronMovie {
    pub root: Clip,
    /// `FrontEndManager::mChyron`.
    chyron: Chyron,
    /// The front-end state the chyron last entered.
    state: Option<i32>,
}

impl ChyronMovie {
    /// `loadMovie`: `new chyron(this)` hides and stops the movie.
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "chyron" => {
                    root.visible = false;
                    root.playback.stop();
                }
                _ => return Err(format!("untranslated chyron callback {call}").into()),
            }
        }
        Ok(Self { root, chyron: Chyron::default(), state: None })
    }

    /// The front-end state now on top; a new one runs its `UpdateOnEnter`.
    pub fn enter(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        if self.state != Some(state) {
            self.state = Some(state);
            self.chyron.enter(state);
        }
        self.apply(assets)
    }

    /// `Chyron::Display(songId)`.
    pub fn display(&mut self, assets: &Assets, song: u32) -> RenderResult<()> {
        self.chyron.display(song);
        self.apply(assets)
    }

    /// One front-end update: the movie (`AptIntegration::Update`) and its `Audio` callbacks,
    /// then `FrontEndManager::Update`'s count.
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "ChyronIntroDone" => self.chyron.intro_done(),
                "ChyronOutroDone" => self.chyron.outro_done(),
                _ => return Err(format!("untranslated chyron callback {call}").into()),
            }
        }
        self.chyron.update();
        self.apply(assets)
    }

    fn apply(&mut self, assets: &Assets) -> RenderResult<()> {
        for call in self.chyron.take_calls() {
            match call {
                Call::SetVisible(visible) => self.set_visible(assets, visible)?,
                Call::Display(song) => self.show_song(assets, song)?,
                Call::PlayOutro => self.root.goto(assets, "outro")?,
            }
        }
        Ok(())
    }

    /// `SetVisible`: `mScreen._x = Screen_EdgeOffset` sets zero, then `_visible`; hiding plays
    /// `hidden`, a label the movie lacks.
    fn set_visible(&mut self, assets: &Assets, visible: bool) -> RenderResult<()> {
        self.root.visible = visible;
        if !visible {
            self.root.goto(assets, "hidden")?;
        }
        Ok(())
    }

    /// `Display(artist, title, album)`: `GetSongInfo`'s strings through `FixUpString` into
    /// `mcChyron`'s artist, song and album fields, then `intro` while visible.
    fn show_song(&mut self, assets: &Assets, song: u32) -> RenderResult<()> {
        // PC port: a custom song shows the names typed for it.
        let (artist, title, album) = match skate2_core::ui::custom_songs::info(song) {
            Some((artist, title)) => (format!("#{artist}"), format!("#{title}"), "# ".to_owned()),
            None => skate2_audio::system::AudioSystem::music_player_song_info(song, |id| {
                assets.localized.get(&id).map(|units| String::from_utf16_lossy(units))
            }),
        };
        let card = self.root.named(&["mcChyron"])?;
        for (field, text) in [("artist", artist), ("song", title), ("album", album)] {
            let text = fix_up_string(card, assets, &text)?;
            card.named(&[field])?.text = Some(text.encode_utf16().collect());
        }
        if self.root.visible {
            self.root.goto(assets, "intro")?;
        }
        Ok(())
    }
}

/// `FixUpString`: while `testlength`, auto-sized left, is at least as wide as `artist`, the
/// string loses its last character and gains `...`. `GetSongInfo`'s `#` marks a literal that
/// `SkateAptString` (`0x82a70b28`) drops before measuring.
fn fix_up_string(card: &mut Clip, assets: &Assets, text: &str) -> RenderResult<String> {
    let limit = bounds(card.named(&["artist"])?, assets)?.map_or(0.0, |r| r.right - r.left);
    let mut kept: Vec<char> = text.strip_prefix('#').unwrap_or(text).chars().collect();
    let mut shown: String = kept.iter().collect();
    let test = card.named(&["testlength"])?;
    loop {
        test.text = Some(shown.encode_utf16().collect());
        if autosize_text(test, assets, &shown)? < limit || kept.is_empty() {
            return Ok(shown);
        }
        kept.pop();
        shown = kept.iter().collect::<String>() + "...";
    }
}

/// chyron.apt's sole InitAction defines the guarded global class translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 21, 0xd04))
}

/// Root action `0xce4` (`var screen = new chyron(this)`), and the timeline's `IntroDone` at
/// `shown` and `OutroDone` after `outro`, each followed by `stop()`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0xce4 => clip.calls.push("chyron"),
        0x1054 => {
            clip.calls.push("ChyronIntroDone");
            clip.playback.stop();
        }
        0x1078 => {
            clip.calls.push("ChyronOutroDone");
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated chyron APT action {offset:#x}").into()),
    }
    Ok(())
}
