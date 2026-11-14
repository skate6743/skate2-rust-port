//! The career's popups on sk8popup (asset 25). The progression's yes/no dialog:
//! `FrontEndManager::ReceiveProgPopupOpenerMsg` (`0x82638748`) opens it with the message's
//! title and description and the options `ID_COMMON_NO` and `ID_COMMON_YES` (default No, not
//! cancellable), as the pause menu's quit popup does. The several-challenges popup:
//! `SignUpHUD::UpdateDPadUp` (`0x826bd450`) opens it with the title
//! `ID_SIGNUP_MULTIPLIESIGNUP_SIGNUPHUD`, the description `#`, up to four challenge titles and
//! the popup cancellable.
use super::{
    assets::Assets,
    clips::Clip,
    edit_skater_movie::{configure_popup, configure_scrolling_popup},
};
use crate::gpu_renderer::RenderResult;

/// What a popup was last configured with: title, description, options, the highlighted option
/// and whether it is cancellable.
type Shown = (String, String, Vec<String>, usize, bool);

#[derive(Default)]
pub(super) struct ProgressionPopup {
    pub popup: Option<Clip>,
    shown: Option<Shown>,
    /// PC: the first line of a scrolling body (the changelog), as last shown.
    first: Option<usize>,
    /// PC: the last line a scrolling body can start at, from its layout, while one shows.
    pub scroll_limit: Option<usize>,
}

impl ProgressionPopup {
    pub fn update(
        &mut self,
        assets: &Assets,
        dialog: Option<&(String, String, usize)>,
        generic: Option<&[String]>,
    ) -> RenderResult<()> {
        let options = || generic.map_or_else(|| ["ID_COMMON_NO", "ID_COMMON_YES"].map(String::from).to_vec(), <[String]>::to_vec);
        self.show(
            assets,
            dialog.map(|(title, description, selected)| {
                (
                    title.clone(),
                    description.clone(),
                    options(),
                    *selected,
                    false,
                )
            }),
        )
    }

    /// The several-challenges popup, from the career's options and highlighted option.
    pub fn update_sign_up_choice(
        &mut self,
        assets: &Assets,
        choice: Option<&(Vec<String>, usize)>,
    ) -> RenderResult<()> {
        self.show(
            assets,
            choice.map(|(options, selected)| {
                (
                    "ID_SIGNUP_MULTIPLIESIGNUP_SIGNUPHUD".into(),
                    "#".into(),
                    options.clone(),
                    *selected,
                    true,
                )
            }),
        )
    }

    pub(super) fn show(&mut self, assets: &Assets, content: Option<Shown>) -> RenderResult<()> {
        self.show_body(assets, content, None)
    }

    /// PC: a popup holding a long text (`configure_scrolling_popup`) from its line `first`.
    pub(super) fn show_scrolling(&mut self, assets: &Assets, content: Shown, first: usize) -> RenderResult<()> {
        self.show_body(assets, Some(content), Some(first))
    }

    fn show_body(&mut self, assets: &Assets, content: Option<Shown>, first: Option<usize>) -> RenderResult<()> {
        if content == self.shown && first == self.first {
            return Ok(());
        }
        if let Some((title, description, options, selected, cancellable)) = &content {
            if self.popup.is_none() || self.shown.is_none() {
                let mut popup = Clip::new(assets, 25, 0)?;
                popup.goto(assets, "intro")?;
                self.popup = Some(popup);
            }
            let popup = self.popup.as_mut().ok_or("progression popup missing")?;
            let options: Vec<&str> = options.iter().map(String::as_str).collect();
            self.scroll_limit = match first {
                Some(first) => Some(configure_scrolling_popup(
                    popup,
                    assets,
                    *selected,
                    title,
                    description,
                    &options,
                    *cancellable,
                    first,
                )?),
                None => {
                    configure_popup(popup, assets, *selected, title, description, &options, *cancellable)?;
                    None
                }
            };
        } else {
            self.scroll_limit = None;
            if let Some(popup) = self.popup.as_mut() {
                popup.goto(assets, "outro")?;
            }
        }
        self.shown = content;
        self.first = first;
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        let Some(popup) = self.popup.as_mut() else {
            return Ok(());
        };
        popup.tick(assets)?;
        let mut calls = Vec::new();
        popup.take_calls(&mut calls);
        for call in calls {
            match call {
                "CacGenderPopupIntroComplete" => {}
                "CacGenderPopupOutroComplete" => self.popup = None,
                _ => return Err(format!("untranslated progression popup callback {call}").into()),
            }
        }
        Ok(())
    }
}
