//! FE state 25, `FrontEndState_DLC` (screen 10, `main/dlc.swf`). PC port: in place of the Xbox
//! LIVE Marketplace's offers the screen lists the downloadable content the player linked, each
//! installed, and a last row links more. It asks the host for files (content packages or the
//! archives extracted from them), shows `ID_DLC_REGISTERING` while the host reads them, as
//! `StateDLCFlow::EnumerateDLC` (`0x827703c0`) shows it while `DLC::Manager::Refresh` runs, and
//! then sk8popup with what was linked.
//!
//! As the original: highlighting a row is `SetScreenState(1)` (`OnMenuHilight` `0x8265ba98`), A
//! selects (`OnMenuSelect` `0x8265ba30`), and B sounds `FE_SOUND_CORE_B_BUTTON` and returns to
//! the main menu (`ReceiveInput` `0x826437b8`, `OnUpdate` `0x8265b990`'s `SetState(22)`).
use super::pause_menu::PauseMenuInput;

/// `RefreshDLCContent`'s `mScroller.SetNumberOfItems(6, count)`: the rows in view.
pub const VISIBLE: usize = 6;
/// The last row, shown as it is.
pub const LINK_LABEL: &str = "Link DLCs";

/// A linked pack, its title and description shown as they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    List,
    /// The host's file picker is open over the screen.
    Picking,
    /// `ID_DLC_REGISTERING`: no options, not cancellable.
    Registering,
    /// sk8popup's description; A closes it.
    Result(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub packs: Vec<Pack>,
    /// The highlighted row: a pack's, or `packs.len()` for Link DLCs.
    pub selected: usize,
    /// The first row in view.
    pub top: usize,
    pub page: Page,
}

impl Snapshot {
    /// The rows: each pack's title, then Link DLCs.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.packs.len() + 1
    }

    /// The highlighted row's description and whether it is installed; Link DLCs has neither.
    #[must_use]
    pub fn highlighted(&self) -> (Option<&str>, Option<bool>) {
        match self.packs.get(self.selected) {
            Some(pack) => (Some(pack.description.as_str()), Some(true)),
            None => (None, None),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Back,
    /// Link DLCs: the host opens its file picker and answers [`Controller::picked`].
    Link,
}

pub struct Controller {
    pub state: Snapshot,
    /// The screen's intro is done; input waits for it.
    pub intro_complete: bool,
    sounds: Vec<&'static str>,
}

impl Controller {
    #[must_use]
    pub fn new(packs: Vec<Pack>) -> Self {
        Self { state: Snapshot { packs, selected: 0, top: 0, page: Page::List }, intro_complete: false, sounds: Vec::new() }
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }

    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    /// The picker closed: the host reads the files under `ID_DLC_REGISTERING`, or nothing was
    /// picked.
    pub fn picked(&mut self, any: bool) {
        if self.state.page == Page::Picking {
            self.state.page = if any { Page::Registering } else { Page::List };
        }
    }

    /// The host linked what it could: the list as it now stands and sk8popup's text
    /// (`FrontEndState_Popup::OnEnter` `0x826583d0` sounds `core_popup`).
    pub fn linked(&mut self, packs: Vec<Pack>, text: String) {
        self.state.packs = packs;
        self.state.selected = self.state.selected.min(self.state.packs.len());
        self.clamp_window();
        self.state.page = Page::Result(text);
        self.sounds.push("FE_SOUND_CORE_POPUP");
    }

    fn clamp_window(&mut self) {
        let rows = self.state.rows();
        let top = self.state.top.min(rows.saturating_sub(VISIBLE));
        self.state.top = if self.state.selected < top {
            self.state.selected
        } else if self.state.selected >= top + VISIBLE {
            self.state.selected + 1 - VISIBLE
        } else {
            top
        };
    }

    pub fn update(&mut self, input: PauseMenuInput, focused: bool) -> Action {
        if !focused || !self.intro_complete {
            return Action::None;
        }
        match self.state.page {
            Page::Picking | Page::Registering => return Action::None,
            // sk8popup's one option: A closes it (`FrontEndState_Popup::OnMenuSelect` 0x82658598).
            Page::Result(_) => {
                if input.select {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    self.state.page = Page::List;
                }
                return Action::None;
            }
            Page::List => {}
        }
        if input.cancel {
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            return Action::Back;
        }
        // menu_picker's Prev and Next wrap and sound the move.
        let rows = self.state.rows();
        if input.up {
            self.state.selected = (self.state.selected + rows - 1) % rows;
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
        }
        if input.down {
            self.state.selected = (self.state.selected + 1) % rows;
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
        }
        self.clamp_window();
        // Only Link DLCs takes A: a linked pack is installed, as `CanSelectionBePurchased(false)`
        // shows only Back for content already bought.
        if input.select && self.state.selected == self.state.packs.len() {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.state.page = Page::Picking;
            return Action::Link;
        }
        Action::None
    }
}

/// sk8popup's text after a link: the packs linked, the files that were already linked and the
/// files that are no downloadable content, `^` breaking lines as `SkateAptString` does.
#[must_use]
pub fn link_report(linked: &[String], already: &[String], failed: &[(String, String)]) -> String {
    fn section(text: &mut String, heading: &str, lines: &[String]) {
        if lines.is_empty() {
            return;
        }
        if text.len() > 1 {
            text.push_str("^^");
        }
        text.push_str(heading);
        for line in lines {
            text.push_str("^- ");
            text.push_str(line);
        }
    }
    let mut text = String::from("#");
    section(&mut text, "The following DLCs were linked:^", linked);
    section(&mut text, "Already linked:^", already);
    let failed: Vec<String> = failed.iter().map(|(file, reason)| format!("{file} {reason}")).collect();
    section(&mut text, "These files were not linked:^", &failed);
    if text.len() == 1 {
        text.push_str("No DLCs were linked.");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(menu: &mut Controller, input: PauseMenuInput) -> Action {
        menu.update(input, true)
    }

    fn packs(n: usize) -> Vec<Pack> {
        (0..n).map(|i| Pack { title: format!("Pack {i}"), description: format!("About {i}") }).collect()
    }

    #[test]
    fn the_rows_wrap_through_link_dlcs_and_the_window_follows() {
        let mut menu = Controller::new(packs(7));
        menu.intro_complete = true;
        assert_eq!(press(&mut menu, PauseMenuInput { up: true, ..Default::default() }), Action::None);
        assert_eq!((menu.state.selected, menu.state.top), (7, 2), "up from the first row reaches Link DLCs");
        assert_eq!(menu.state.highlighted(), (None, None));
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        assert_eq!((menu.state.selected, menu.state.top), (0, 0));
        assert_eq!(menu.state.highlighted(), (Some("About 0"), Some(true)));
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_NAV_UP", "FE_SOUND_CORE_NAV_DOWN"]);
    }

    #[test]
    fn a_links_only_from_the_last_row_and_b_goes_back() {
        let mut menu = Controller::new(packs(1));
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::None, "not before the intro");
        menu.intro_complete = true;
        assert_eq!(press(&mut menu, PauseMenuInput { select: true, ..Default::default() }), Action::None, "a pack is installed");
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        let _ = menu.take_sounds();
        assert_eq!(press(&mut menu, PauseMenuInput { select: true, ..Default::default() }), Action::Link);
        assert_eq!((menu.state.page.clone(), menu.take_sounds()), (Page::Picking, vec!["FE_SOUND_CORE_A_BUTTON"]));
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::None, "the picker is open");
        menu.picked(true);
        assert_eq!(menu.state.page, Page::Registering);
        menu.linked(packs(2), link_report(&["Pack 1".into()], &[], &[]));
        assert_eq!(menu.state.page, Page::Result("#The following DLCs were linked:^^- Pack 1".into()));
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_POPUP"]);
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::None, "only A closes it");
        press(&mut menu, PauseMenuInput { select: true, ..Default::default() });
        assert_eq!(menu.state.page, Page::List);
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::Back);
        assert!(menu.take_sounds().ends_with(&["FE_SOUND_CORE_B_BUTTON"]));
    }

    #[test]
    fn nothing_picked_returns_to_the_list() {
        let mut menu = Controller::new(Vec::new());
        menu.intro_complete = true;
        assert_eq!(press(&mut menu, PauseMenuInput { select: true, ..Default::default() }), Action::Link);
        menu.picked(false);
        assert_eq!(menu.state.page, Page::List);
    }

    #[test]
    fn the_report_names_what_was_linked_and_why_others_were_not() {
        assert_eq!(
            link_report(&["Maloof Money Cup Pack".into()], &["Throwback Pack".into()], &[("anim.big".into(), "is not a Skate 2 downloadable content file".into())]),
            "#The following DLCs were linked:^^- Maloof Money Cup Pack^^Already linked:^^- Throwback Pack^^These files were not linked:^^- anim.big is not a Skate 2 downloadable content file"
        );
        assert_eq!(link_report(&[], &[], &[]), "#No DLCs were linked.");
    }
}
