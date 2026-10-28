//! PC port: the Downloadable Content screen's packs and its Link DLCs (`ui::dlc_menu`), and what
//! a link refreshes: data read from the vaults is read again, with the new content's, when next
//! needed, as `StateDLCFlow` reloads the vaults (`LoadDLCAttribVaults` 0x827686f8) and
//! re-enumerates the challenges (`ReenumerateChallenges` 0x827575e0) after a content refresh.
use super::SelectedLiveSession;
use crate::ui::dlc_menu::{self, Pack};

impl<T, C> SelectedLiveSession<T, C> {
    /// The packs the host linked.
    pub fn set_linked_dlc(&mut self, packs: Vec<Pack>) {
        self.linked_dlc = packs;
    }

    /// Main menu row 4: FE state 25's screen with the linked packs.
    pub(super) fn open_dlc(&mut self) {
        let packs = self.linked_dlc.clone();
        if let Some(menu) = self.main_menu.as_mut() {
            menu.dlc = Some(dlc_menu::Controller::new(packs));
        }
    }

    /// Link DLCs was selected since the last call: the host opens its file picker.
    pub fn take_dlc_link_request(&mut self) -> bool {
        std::mem::take(&mut self.dlc_link_request)
    }

    /// The host's file picker closed, with files to link or none.
    pub fn dlc_picked(&mut self, any: bool) {
        if let Some(dlc) = self.main_menu.as_mut().and_then(|menu| menu.dlc.as_mut()) {
            dlc.picked(any);
        }
    }

    /// The host linked the picked files: every pack now linked, sk8popup's report, and whether
    /// any pack is new. A new pack's vaults, unlocks and challenges join what reads the
    /// challenge system's database next, and its global locators the next location manager
    /// (`StateDLCFlow::LoadGlobalLocators`).
    pub fn dlc_linked(&mut self, packs: Vec<Pack>, report: String, changed: bool) {
        if changed {
            self.freeskate.data = None;
            self.online.locations = None;
            // The server's matchmaking and lobby browser offer games at the new content's
            // challenges as well.
            if self.online.signed_in()
                && let Err(failure) = self.online_send_content()
            {
                eprintln!("online: content after the link: {failure}");
            }
        }
        self.linked_dlc.clone_from(&packs);
        if let Some(dlc) = self.main_menu.as_mut().and_then(|menu| menu.dlc.as_mut()) {
            dlc.linked(packs, report);
        }
    }

    /// The screen's intro finished (`ScreenManager.IntroComplete`).
    pub fn acknowledge_dlc(&mut self, intro: bool) {
        if let Some(dlc) = self.main_menu.as_mut().and_then(|menu| menu.dlc.as_mut()) {
            dlc.intro_complete |= intro;
        }
    }
}
