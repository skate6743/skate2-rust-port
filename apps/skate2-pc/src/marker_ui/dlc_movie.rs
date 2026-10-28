//! FE state 25's screen 10, main/dlc.apt d5aa4818. PC port: the linked downloadable content
//! (`skate2_core::ui::dlc_menu`) in place of the Marketplace's offers. menu_picker lists the
//! packs' titles and Link DLCs, basic_slider shows its window of rows, the highlighted pack's
//! description fills `mOfferDescription` and `mInstalledText` reads Installed (`SetState(1)`);
//! Link DLCs, no offer, shows neither. No pack has a cost, so `mAmount` stays hidden. The button
//! menu holds Back, and Select on Link
//! DLCs (`CanSelectionBePurchased`). The PS3 store logos stay off, as the constructor's
//! `Platform_PS3` branch leaves them on other platforms. PC: `mInstalledText` stands where
//! `mAmount` would, but its placement carries no colour, so it would show at full alpha over
//! the entrance; it takes `mAmount`'s fade instead.
use super::{assets::Assets, clips::Clip, letterbox::LetterBox, map_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::dlc_menu::{LINK_LABEL, Page, Snapshot, VISIBLE};

pub(super) const ASSET: usize = 119;
/// `RefreshDLCContent`'s `mMenuPicker.SetItemOffset(32)` and `mScroller.SetSliderLength(215)`.
const ITEM_OFFSET: &str = "32";
const SLIDER_LENGTH: f32 = 215.;
/// The button menu's text colour, the letterbox's.
const BUTTON_COLOUR: u32 = 0xffff_ffec;

pub(super) struct DlcMovie {
    pub root: Clip,
    /// The state the screen shows.
    shown: Option<Snapshot>,
    /// OnEnter's `OpenScreen(10)`, until the screen manager links it.
    pending: bool,
    destroy: bool,
    intro: bool,
    outro_complete: bool,
}

impl DlcMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self { root, shown: None, pending: false, destroy: false, intro: false, outro_complete: false })
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn take_outro_complete(&mut self) -> bool {
        std::mem::take(&mut self.outro_complete)
    }

    /// `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro(&mut self) -> bool {
        std::mem::take(&mut self.intro)
    }

    /// The popup over the screen: `ID_DLC_REGISTERING` with no options, or the result with OK.
    pub fn popup(state: Option<&Snapshot>) -> Option<((String, String, usize), Vec<String>)> {
        match &state?.page {
            Page::Registering => {
                Some((("ID_MAIN_MENU_DOWNLOADABLE_CONTENT".to_owned(), "ID_DLC_REGISTERING".to_owned(), 0), Vec::new()))
            }
            Page::Result(text) => {
                Some((("ID_MAIN_MENU_DOWNLOADABLE_CONTENT".to_owned(), text.clone(), 0), vec!["ID_COMMON_OK".to_owned()]))
            }
            Page::List | Page::Picking => None,
        }
    }

    pub fn update(&mut self, assets: &Assets, state: Option<&Snapshot>) -> RenderResult<()> {
        if self.shown.as_ref() == state {
            return Ok(());
        }
        match (self.shown.is_some(), state.is_some()) {
            (false, true) => {
                self.pending = true;
                self.intro = false;
            }
            // OnLeave's CloseScreen plays the outro.
            (true, false) => {
                if !std::mem::take(&mut self.pending) {
                    self.root.goto(assets, "outro")?;
                }
            }
            _ => {}
        }
        self.shown = state.cloned();
        if let Some(state) = state.filter(|_| !self.pending) {
            self.populate(assets, state)?;
        }
        Ok(())
    }

    /// The screen manager linked screen 10: the constructor, `SetState(1)` once its intro plays.
    pub fn link(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        if !std::mem::take(&mut self.pending) {
            return Ok(());
        }
        self.root = Clip::new(assets, ASSET, 0)?;
        self.destroy = false;
        self.outro_complete = false;
        for logo in ["mSony_store", "mSony_store2"] {
            self.root.named(&[logo])?.visible = false;
        }
        self.root.named(&["mDLC", "mInstalledText"])?.visible = false;
        if let Some(state) = self.shown.clone() {
            self.populate(assets, &state)?;
        }
        // Frame 1: ScreenManager.EnableDimmer(true), which screenmanager.apt never defines (the
        // state has no mbDimmer), and EnableLetterBox(false).
        letterbox.enable(assets, false)?;
        self.root.goto(assets, "intro")?;
        self.callbacks(assets, letterbox)
    }

    /// `RefreshDLCContent` and `SetState(1)` for the highlighted row.
    fn populate(&mut self, assets: &Assets, state: &Snapshot) -> RenderResult<()> {
        let rows: Vec<(&str, bool)> =
            state.packs.iter().map(|pack| (pack.title.as_str(), true)).chain(std::iter::once((LINK_LABEL, true))).collect();
        let dlc = self.root.named(&["mDLC"])?;
        let picker = dlc.named(&["mMenuPicker", "mMenuPicker"])?;
        picker.properties.insert("picker_item_offset".into(), ITEM_OFFSET.into());
        map_movie::update_picker(picker, assets, &rows, state.selected, state.top, VISIBLE, false)?;
        map_movie::update_scrollbar(dlc.named(&["mScroller"])?, assets, rows.len(), VISIBLE, state.top, SLIDER_LENGTH)?;
        let (description, installed) = state.highlighted();
        let offer = dlc.named(&["mOfferDescription"])?;
        offer.visible = description.is_some();
        if let Some(description) = description {
            offer.named(&["labelText"])?.text = Some(description.encode_utf16().collect());
        }
        dlc.named(&["mAmount"])?.visible = false;
        let text = dlc.named(&["mInstalledText"])?;
        text.visible = installed.is_some();
        if let Some(installed) = installed {
            let label = if installed { "ID_COMMON_INSTALLED" } else { "ID_COMMON_NOT_INSTALLED" };
            text.text = Some(assets.localize(label)?.encode_utf16().collect());
        }
        let mut buttons = vec![("B", "ID_COMMON_BACK")];
        if installed.is_none() {
            buttons.push(("A", "ID_COMMON_SELECT"));
        }
        map_movie::configure_button_items(dlc.named(&["mButtonmenu"])?, assets, &buttons, BUTTON_COLOUR)
    }

    pub fn tick(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        if std::mem::take(&mut self.destroy) {
            self.root.visible = false;
        }
        if self.root.visible {
            self.root.tick(assets)?;
            self.fade_installed_text()?;
        }
        self.callbacks(assets, letterbox)
    }

    fn fade_installed_text(&mut self) -> RenderResult<()> {
        let dlc = self.root.named(&["mDLC"])?;
        let alpha = named_child(dlc, "mAmount")?.entry.colour_scale & 0xff00_0000;
        let text = named_child(dlc, "mInstalledText")?;
        text.entry.colour_scale = (text.entry.colour_scale & 0x00ff_ffff) | alpha;
        Ok(())
    }

    fn callbacks(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "DlcIntroComplete" => self.intro = true,
                "DlcOutroBegin" => letterbox.enable_info(assets, false)?,
                "DlcOutroComplete" => {
                    self.destroy = true;
                    self.outro_complete = true;
                }
                _ => return Err(format!("untranslated DLC screen callback {call}").into()),
            }
        }
        Ok(())
    }
}

fn named_child<'a>(clip: &'a mut Clip, name: &str) -> RenderResult<&'a mut super::clips::Child> {
    clip.children
        .values_mut()
        .find(|child| child.entry.name.as_deref() == Some(name))
        .ok_or_else(|| format!("DLC screen {name} absent").into())
}

/// Frame 0's init action defines `_global.dlc` (character 25): its constructor and methods are this
/// module's translation.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    (asset, character, offset) == (ASSET, 25, 0x2994)
}

pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        // Frame 0: `screen = new dlc(this)`. Its constructor's display work is done at the link;
        // the constructor stops the screen (`mScreen.stop()`) until the screen manager plays its
        // intro.
        0x295c => clip.playback.stop(),
        // Frame 1: EnableDimmer (undefined) and EnableLetterBox(false), done at the link.
        0x2eb8 => {}
        // Frame 11: stop, ScreenManager.IntroComplete.
        0x2edc => {
            clip.playback.stop();
            clip.calls.push("DlcIntroComplete");
        }
        // Frame 12: LetterBox.EnableLetterBoxInfo(false).
        0x2f00 => clip.calls.push("DlcOutroBegin"),
        // Frame 24: stop, ScreenManager.OutroComplete, EnableDimmer(false) (undefined).
        0x2f1c => {
            clip.playback.stop();
            clip.calls.push("DlcOutroComplete");
        }
        // mDLC's (character 18) frame 54, the end of its entrance, and the PS3 store logo's
        // (character 24) frames 9, 19, 29 and 39, the ends of its off, sd_480, hd_720 and hd_1080
        // states: stop.
        0x2fc0 | 0x2fd0 | 0x2fdc | 0x2fe8 | 0x2ff4 => clip.playback.stop(),
        _ => return Err(format!("untranslated DLC screen APT action {offset:#x}").into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_core::ui::dlc_menu::Pack;
    use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};

    fn installed_alpha(movie: &mut DlcMovie) -> u32 {
        let dlc = movie.root.named(&["mDLC"]).unwrap();
        dlc.children.values().find(|child| child.entry.name.as_deref() == Some("mInstalledText")).unwrap().entry.colour_scale >> 24
    }

    /// The screen's whole life on the original movie: built with the UI at start, linked, its
    /// intro and every child's timeline played past their stops, a selection shown, then its outro.
    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT"]
    fn the_original_screen_builds_plays_and_closes() {
        let game = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES.iter() {
            mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
        }
        let assets = Assets::load(&mounts).unwrap();
        let mut letterbox = LetterBox::new(&assets).unwrap();
        let mut movie = DlcMovie::new(&assets).unwrap();
        assert!(!movie.root.visible);
        let pack = |title: &str| Pack { title: title.into(), description: format!("{title} for the test") };
        let mut state = Snapshot { packs: vec![pack("Filmer Pack"), pack("Training Facility")], selected: 0, top: 0, page: Page::List };
        movie.update(&assets, Some(&state)).unwrap();
        assert!(movie.pending());
        movie.link(&assets, &mut letterbox).unwrap();
        assert!(!movie.pending() && movie.root.visible);
        movie.tick(&assets, &mut letterbox).unwrap();
        assert_eq!(installed_alpha(&mut movie), 0, "the Installed text waits for the entrance");
        for _ in 0..119 {
            movie.tick(&assets, &mut letterbox).unwrap();
        }
        assert!(movie.take_intro());
        assert!(movie.root.named(&["mDLC", "mOfferDescription"]).unwrap().visible, "a pack shows its description");
        assert_eq!(installed_alpha(&mut movie), 0xff, "the Installed text ends the entrance opaque");
        state.selected = 2;
        movie.update(&assets, Some(&state)).unwrap();
        for _ in 0..30 {
            movie.tick(&assets, &mut letterbox).unwrap();
        }
        assert!(!movie.root.named(&["mDLC", "mOfferDescription"]).unwrap().visible, "Link DLCs shows no description");
        movie.update(&assets, None).unwrap();
        for _ in 0..60 {
            movie.tick(&assets, &mut letterbox).unwrap();
        }
        assert!(movie.take_outro_complete());
        assert!(!movie.root.visible);
    }
}
