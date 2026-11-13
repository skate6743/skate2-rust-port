//! FE state 25, `FrontEndState_DLC` (screen 10, `main/dlc.swf`). PC port: in place of the Xbox
//! LIVE Marketplace's offers the screen lists the downloadable content the player linked, each
//! installed, under a first row that links more. It asks the host for files (content packages or the
//! archives extracted from them), shows `ID_DLC_REGISTERING` while the host reads them, as
//! `StateDLCFlow::EnumerateDLC` (`0x827703c0`) shows it while `DLC::Manager::Refresh` runs, and
//! then sk8popup with what was linked.
//!
//! As the original: highlighting a row is `SetScreenState(1)` (`OnMenuHilight` `0x8265ba98`), A
//! selects (`OnMenuSelect` `0x8265ba30`), and B sounds `FE_SOUND_CORE_B_BUTTON` and returns to
//! the main menu (`ReceiveInput` `0x826437b8`, `OnUpdate` `0x8265b990`'s `SetState(22)`). PC port
//! addition: X unlinks the highlighted pack, and sk8popup reports it; a row below Link DLCs links
//! a Skate 3 game folder (`crate::skate3`), picked by its `default.xex`. The two link rows stay at
//! the top, above the packs, however many are linked.
use super::pause_menu::PauseMenuInput;

/// `RefreshDLCContent`'s `mScroller.SetNumberOfItems(6, count)`: the rows in view.
pub const VISIBLE: usize = 6;
/// The rows before the packs, shown as they are: Link DLCs, then Link Skate 3 Data.
pub const LINK_LABEL: &str = "Link DLCs";
pub const LINK_SKATE3_LABEL: &str = "Link Skate 3 Data";
const ACTION_LABELS: [&str; 2] = [LINK_LABEL, LINK_SKATE3_LABEL];

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
    /// The host unlinks the highlighted pack; input waits for its answer.
    Unlinking,
    /// `ID_DLC_REGISTERING`: no options, not cancellable.
    Registering,
    /// sk8popup's description; A closes it.
    Result(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub packs: Vec<Pack>,
    /// The highlighted row: 0 for Link DLCs, 1 for Link Skate 3 Data, then the packs'.
    pub selected: usize,
    /// The first row in view.
    pub top: usize,
    pub page: Page,
}

impl Snapshot {
    /// The rows: Link DLCs and Link Skate 3 Data, then each pack's title.
    #[must_use]
    pub fn rows(&self) -> usize {
        ACTION_LABELS.len() + self.packs.len()
    }

    /// Every row's text: the action rows', then the packs' titles.
    #[must_use]
    pub fn labels(&self) -> Vec<&str> {
        ACTION_LABELS.into_iter().chain(self.packs.iter().map(|pack| pack.title.as_str())).collect()
    }

    /// The pack of the highlighted row; none on an action row.
    #[must_use]
    pub fn selected_pack(&self) -> Option<usize> {
        self.selected.checked_sub(ACTION_LABELS.len()).filter(|&pack| pack < self.packs.len())
    }

    /// The highlighted row's description and whether it is installed; the action rows have
    /// neither.
    #[must_use]
    pub fn highlighted(&self) -> (Option<&str>, Option<bool>) {
        match self.selected_pack().map(|pack| &self.packs[pack]) {
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
    /// Link Skate 3 Data: the host opens its file picker for Skate 3's `default.xex` and answers
    /// [`Controller::picked`].
    LinkSkate3,
    /// X on a pack: the host unlinks the pack at this index and answers [`Controller::linked`].
    Unlink(usize),
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
        self.state.selected = self.state.selected.min(self.state.rows() - 1);
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
            Page::Picking | Page::Registering | Page::Unlinking => return Action::None,
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
        if input.misc
            && let Some(pack) = self.state.selected_pack()
        {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.state.page = Page::Unlinking;
            return Action::Unlink(pack);
        }
        // Only the action rows take A: a linked pack is installed, as
        // `CanSelectionBePurchased(false)` shows only Back for content already bought.
        if input.select && self.state.selected < ACTION_LABELS.len() {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.state.page = Page::Picking;
            return if self.state.selected == 0 { Action::Link } else { Action::LinkSkate3 };
        }
        Action::None
    }
}

/// sk8popup's text after X unlinked a pack, and why `config.json` would link it again at the
/// next launch when it could not be saved.
#[must_use]
pub fn unlink_report(title: &str, save_error: Option<&str>) -> String {
    let mut text = format!("#The following DLC was unlinked:^^- {title}");
    if let Some(error) = save_error {
        text.push_str(&format!("^^config.json could not be saved ({error}): the DLC links again at the next launch."));
    }
    text
}

/// sk8popup's text after a link: the packs linked, a blank line between each, then those already
/// linked, `^` breaking lines as `SkateAptString` does, and why `config.json` could not be saved
/// when it was not (the packs then stay linked until the game closes). The files that could not
/// be linked are left to the host's log.
#[must_use]
pub fn link_report(linked: &[String], already: &[String], save_error: Option<&str>) -> String {
    fn section(text: &mut String, heading: &str, lines: &[String], bullet: &str) {
        if lines.is_empty() {
            return;
        }
        if text.len() > 1 {
            text.push_str("^^");
        }
        text.push_str(heading);
        for line in lines {
            text.push_str(bullet);
            text.push_str(line);
        }
    }
    let mut text = String::from("#");
    section(&mut text, "The following DLCs were linked:", linked, "^^- ");
    section(&mut text, "Already linked:^", already, "^- ");
    if text.len() == 1 {
        text.push_str("No DLCs were linked.");
    }
    if let Some(error) = save_error {
        text.push_str(&format!("^^config.json could not be saved ({error}): the DLCs stay linked until the game closes."));
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
        assert_eq!(menu.state.labels()[..3], [LINK_LABEL, LINK_SKATE3_LABEL, "Pack 0"]);
        assert_eq!(press(&mut menu, PauseMenuInput { up: true, ..Default::default() }), Action::None);
        assert_eq!((menu.state.selected, menu.state.top), (8, 3), "up from Link DLCs reaches the last pack");
        assert_eq!(menu.state.highlighted(), (Some("About 6"), Some(true)));
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        assert_eq!((menu.state.selected, menu.state.top), (0, 0));
        assert_eq!(menu.state.highlighted(), (None, None), "Link DLCs");
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_NAV_UP", "FE_SOUND_CORE_NAV_DOWN"]);
    }

    #[test]
    fn a_links_only_from_the_first_row_and_b_goes_back() {
        let mut menu = Controller::new(packs(1));
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::None, "not before the intro");
        menu.intro_complete = true;
        press(&mut menu, PauseMenuInput { up: true, ..Default::default() });
        assert_eq!(press(&mut menu, PauseMenuInput { select: true, ..Default::default() }), Action::None, "a pack is installed");
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        let _ = menu.take_sounds();
        assert_eq!(press(&mut menu, PauseMenuInput { select: true, ..Default::default() }), Action::Link);
        assert_eq!((menu.state.page.clone(), menu.take_sounds()), (Page::Picking, vec!["FE_SOUND_CORE_A_BUTTON"]));
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::None, "the picker is open");
        menu.picked(true);
        assert_eq!(menu.state.page, Page::Registering);
        menu.linked(packs(2), link_report(&["Pack 1".into()], &[], None));
        assert_eq!(menu.state.page, Page::Result("#The following DLCs were linked:^^- Pack 1".into()));
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_POPUP"]);
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::None, "only A closes it");
        press(&mut menu, PauseMenuInput { select: true, ..Default::default() });
        assert_eq!(menu.state.page, Page::List);
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), Action::Back);
        assert!(menu.take_sounds().ends_with(&["FE_SOUND_CORE_B_BUTTON"]));
    }

    #[test]
    fn x_unlinks_the_highlighted_pack_and_not_link_dlcs() {
        let mut menu = Controller::new(packs(3));
        menu.intro_complete = true;
        assert_eq!(press(&mut menu, PauseMenuInput { misc: true, ..Default::default() }), Action::None, "Link DLCs unlinks nothing");
        for _ in 0..3 {
            press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        }
        let _ = menu.take_sounds();
        assert_eq!(press(&mut menu, PauseMenuInput { misc: true, ..Default::default() }), Action::Unlink(1));
        assert_eq!((menu.state.page.clone(), menu.take_sounds()), (Page::Unlinking, vec!["FE_SOUND_CORE_A_BUTTON"]));
        assert_eq!(press(&mut menu, PauseMenuInput { misc: true, ..Default::default() }), Action::None, "the host answers first");
        menu.linked(packs(2), unlink_report("Pack 1", None));
        assert_eq!(menu.state.page, Page::Result("#The following DLC was unlinked:^^- Pack 1".into()));
        press(&mut menu, PauseMenuInput { select: true, ..Default::default() });
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        assert_eq!(menu.state.selected, 0, "past the last pack, Link DLCs");
        menu.linked(packs(0), unlink_report("Pack 0", None));
        assert_eq!(menu.state.selected, 0, "the selection stays on a row");
    }

    #[test]
    fn the_row_below_link_dlcs_links_skate3_data() {
        let mut menu = Controller::new(packs(1));
        menu.intro_complete = true;
        assert_eq!(menu.state.labels(), [LINK_LABEL, LINK_SKATE3_LABEL, "Pack 0"]);
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        assert_eq!(menu.state.selected, 1);
        assert_eq!(press(&mut menu, PauseMenuInput { select: true, ..Default::default() }), Action::LinkSkate3);
        assert_eq!(menu.state.page, Page::Picking);
        menu.picked(false);
        assert_eq!(menu.state.page, Page::List);
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
    fn the_report_spaces_the_packs_linked_and_names_those_already_linked() {
        assert_eq!(
            link_report(&["Maloof Money Cup Pack".into(), "Skate 3: After Dark".into()], &["Throwback Pack".into(), "Skate 3: Danny Way's Hawaiian Dream".into()], None),
            "#The following DLCs were linked:^^- Maloof Money Cup Pack^^- Skate 3: After Dark^^Already linked:^^- Throwback Pack^- Skate 3: Danny Way's Hawaiian Dream"
        );
        assert_eq!(
            link_report(&["Throwback Pack".into()], &[], Some("denied")),
            "#The following DLCs were linked:^^- Throwback Pack^^config.json could not be saved (denied): the DLCs stay linked until the game closes."
        );
        assert_eq!(link_report(&[], &[], None), "#No DLCs were linked.");
    }
}
