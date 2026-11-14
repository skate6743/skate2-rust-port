//! Original Extras screen70, extras.apt7031d76f and menu_picker LIGHT_ON_DARK.
use super::{
    assets::Assets, clips::Clip, credits_movie::CreditsMovie, letterbox::LetterBox, map_movie,
    progression_popup::ProgressionPopup,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::extras::{MAX_OUT, MAX_OUT_DONE, MAX_OUT_QUESTION, Page, Snapshot, labels};
pub(super) const ASSET: usize = 85;
pub(super) struct ExtrasMovie {
    pub root: Clip,
    pub credits: CreditsMovie,
    pub popup: ProgressionPopup,
    shown: Option<Snapshot>,
    intro: bool,
    pending: bool,
    destroy: bool,
    outro_complete: bool,
}
impl ExtrasMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        root.visible = false;
        Ok(Self {
            root,
            credits: CreditsMovie::new(assets)?,
            popup: Default::default(),
            shown: None,
            intro: false,
            pending: false,
            destroy: false,
            outro_complete: false,
        })
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn take_outro_complete(&mut self) -> bool {
        std::mem::take(&mut self.outro_complete)
    }
    pub fn update(
        &mut self,
        assets: &Assets,
        state: Option<&Snapshot>,
        letterbox: &mut LetterBox,
    ) -> RenderResult<()> {
        if self.shown.as_ref() == state {
            return Ok(());
        }
        let visible = |state: Option<&Snapshot>| {
            state.is_some_and(|s| !matches!(s.page, Page::Movie | Page::Credits))
        };
        let was_visible = visible(self.shown.as_ref());
        let is_visible = visible(state);
        if is_visible && !was_visible {
            // OnEnter82653150 requests screen70 through ReadyScreen. Its
            // constructor and intro wait for the screen manager's link.
            self.pending = true;
            self.intro = false;
        } else if was_visible && !is_visible {
            if !std::mem::take(&mut self.pending) {
                self.root.goto(assets, "outro")?;
            }
        }
        let credits = state.is_some_and(|s| s.page == Page::Credits);
        let was_credits = self.shown.as_ref().is_some_and(|s| s.page == Page::Credits);
        if credits && !was_credits {
            self.credits.open(assets)?;
        }
        if was_credits && !credits {
            self.credits.close(assets)?;
        }
        if state.is_some_and(|s| matches!(s.page, Page::Movie | Page::Credits)) {
            letterbox.enable(assets, false)?;
        }
        if let Some(state) = state.filter(|_| is_visible && !self.pending) {
            self.update_picker(assets, state)?;
        }
        let dialog = state.and_then(|state| match &state.page {
            Page::Code { text, cursor } => {
                let mut field = text.clone();
                field.insert(*cursor, '|');
                Some((
                    "ID_EXTRA_PROMO_CODE".to_owned(),
                    format!("#Type a code, then press Enter.\n\n{field}"),
                    0,
                ))
            }
            Page::Result(cheat) => Some((
                "ID_VALID_CODE_TITLE".to_owned(),
                cheat.description().to_owned(),
                0,
            )),
            Page::SaveError(message) => {
                Some(("#Could not save".to_owned(), format!("#{message}"), 0))
            }
            // PC port addition: Max Out Save's yes/no (the progression popup's, No highlighted
            // first) and its result.
            Page::ConfirmMaxOut { yes } => Some((MAX_OUT.to_owned(), MAX_OUT_QUESTION.to_owned(), usize::from(!*yes))),
            Page::MaxedOut => Some((MAX_OUT.to_owned(), MAX_OUT_DONE.to_owned(), 0)),
            _ => None,
        });
        let options: Vec<String> = if state.is_some_and(|s| matches!(s.page, Page::ConfirmMaxOut { .. })) {
            vec!["ID_COMMON_YES".into(), "ID_COMMON_NO".into()]
        } else if state.is_some_and(|s| matches!(s.page, Page::Code { .. })) {
            vec!["#Enter: Submit    Esc: Cancel".into()]
        } else {
            vec!["ID_COMMON_OK".into()]
        };
        let had_dialog = self.shown.as_ref().is_some_and(|s| {
            matches!(
                s.page,
                Page::Code { .. } | Page::Result(_) | Page::SaveError(_) | Page::ConfirmMaxOut { .. } | Page::MaxedOut
            )
        });
        if dialog.is_some() && !had_dialog {
            // A new popup can open while the preceding one's outro is still
            // drawing (e.g. Enter followed immediately by the save result).
            // Give it a new screen; do not leave it on that old outro timeline.
            self.popup = ProgressionPopup::default();
        }
        self.popup.update(assets, dialog.as_ref(), Some(&options))?;
        self.shown = state.cloned();
        self.callbacks(assets, letterbox)
    }
    /// ScreenTransitionManager.InitialScreenReady (APT3ac): show and intro
    /// after ReadyScreen/ProcessOpenScreens/OnLoaded have linked screen70.
    pub fn link(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        if !std::mem::take(&mut self.pending) {
            return Ok(());
        }
        self.root = Clip::new(assets, ASSET, 0)?;
        self.destroy = false;
        self.outro_complete = false;
        if let Some(state) = self.shown.clone() {
            self.update_picker(assets, &state)?;
        }
        letterbox.enable(assets, true)?;
        letterbox.set_title(&assets.localize("ID_MAIN_MENU_EXTRAS")?)?;
        map_movie::configure_button_items(
            letterbox
                .clip
                .named(&["mcLetterbox", "mInfo", "mcBottomMenu", "mButtonMenu"])?,
            assets,
            &[("B", "ID_COMMON_BACK"), ("A", "ID_COMMON_SELECT")],
            0xffff_ffec,
        )?;
        self.root.goto(assets, "intro")?;
        self.callbacks(assets, letterbox)
    }
    fn update_picker(&mut self, assets: &Assets, state: &Snapshot) -> RenderResult<()> {
        let labels = labels(state.count)
            .iter()
            .map(|label| assets.localize(label))
            .collect::<RenderResult<Vec<_>>>()?;
        let rows = labels
            .iter()
            .map(|label| (label.as_str(), true))
            .collect::<Vec<_>>();
        // Extras imports menu_picker's wrapper (character55). Its
        // mMenuPicker child54 owns dummy0, mItemGroup and mIndicator;
        // the authored constructor reads mMenuPicker.dummy0 at 0x294c.
        map_movie::update_picker(
            self.root.named(&["mMenu", "mMenuPicker"])?,
            assets,
            &rows,
            state.selected,
            0,
            state.count,
            false,
        )
    }
    pub fn tick(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        if std::mem::take(&mut self.destroy) {
            self.root.visible = false;
        }
        if self.root.visible {
            self.root.tick(assets)?;
        }
        self.callbacks(assets, letterbox)?;
        self.popup.tick(assets)?;
        self.credits.tick(assets)
    }
    fn callbacks(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "ExtrasIntro" => {
                    self.intro = true;
                    letterbox.enable_info(assets, true)?;
                }
                "ExtrasOutroBegin" => {
                    letterbox.enable_info(assets, false)?;
                    letterbox.cycle(assets)?;
                }
                "ExtrasOutro" => {
                    self.destroy = true;
                    self.outro_complete = true;
                }
                _ => return Err(format!("unknown Extras callback {call}").into()),
            }
        }
        Ok(())
    }
    pub fn text_entry(&self) -> bool {
        self.shown
            .as_ref()
            .is_some_and(|s| matches!(s.page, Page::Code { .. }))
    }
    pub fn take_events(&mut self) -> (bool, bool, bool) {
        (
            std::mem::take(&mut self.intro),
            std::mem::take(&mut self.credits.intro),
            std::mem::take(&mut self.credits.done),
        )
    }
}
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x2ec | 0x308 => clip.playback.stop(),
        0x4ec => {
            clip.playback.stop();
            clip.calls.push("ExtrasIntro");
        }
        0x50c => clip.calls.push("ExtrasOutroBegin"),
        0x530 => {
            clip.playback.stop();
            clip.calls.push("ExtrasOutro");
        }
        _ => return Err(format!("untranslated Extras APT action {offset:#x}").into()),
    }
    Ok(())
}
