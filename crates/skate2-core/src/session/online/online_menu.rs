//! The online front end from the Online options: `FrontEndState_UnrankedFreeskate` (FE state
//! 68, screen 109), `FrontEndState_UnrankedCustomMatch` (FE state 69, screen 110) and the quick
//! match's search under its loading popup, the matchmaking each starts
//! (`SendUnrankedMatchMessage` 0x826a85a0, `StateOnlinePlayNow`), the Lobby Browser (PC port
//! addition) and the notices of failed sign-ins and searches.

use super::{MatchRequest, OnlineNotice, SelectedLiveError, SelectedLiveSession, error, trace};
use crate::online::catalog::{self, OnlineChallenge};
use crate::ui::lobby_browser::Notice;
use crate::ui::online_match::{Action, CustomMatch, FreeskateMatch, MatchChoice, Snapshot};
use crate::ui::pause_menu::PauseMenuInput;
use crate::ui::skate_reel::OnlineAction;
use skate2_formats::localization::{Histogram, LanguageCategory, string_hash};
use skate2_net::protocol::{PlayNowMode, challenge};
use std::collections::BTreeMap;
use std::sync::Arc;

/// `LanguageManager`'s English front-end strings the lists sort by.
const LANGUAGE: &str = "data\\fe\\languages\\english\\LANGUAGE_English";

/// The online screen from the Online options.
pub(super) enum MatchScreen {
    Custom(CustomMatch),
    Freeskate(FreeskateMatch),
    /// A quick match searches under the Online options' loading popup.
    Quick,
}

impl MatchScreen {
    /// The screen's state for its movie; the quick match's card has none.
    pub(super) fn snapshot(&self) -> Option<Snapshot> {
        match self {
            Self::Custom(screen) => Some(screen.snapshot()),
            Self::Freeskate(screen) => Some(screen.snapshot()),
            Self::Quick => None,
        }
    }

    /// `ScreenManager.IntroComplete`: the screen takes input.
    pub(super) fn set_ready(&mut self) {
        match self {
            Self::Custom(screen) => screen.ready = true,
            Self::Freeskate(screen) => screen.ready = true,
            Self::Quick => {}
        }
    }

    /// The screen's input and the sounds it played.
    pub(super) fn update(&mut self, input: PauseMenuInput) -> (Action, Vec<&'static str>) {
        match self {
            Self::Custom(screen) => (screen.update(input), screen.take_sounds()),
            Self::Freeskate(screen) => (screen.update(input), screen.take_sounds()),
            Self::Quick => (Action::None, Vec::new()),
        }
    }

    fn matchmaking_failed(&mut self) {
        match self {
            Self::Custom(screen) => screen.matchmaking_failed(),
            Self::Freeskate(screen) => screen.matchmaking_failed(),
            Self::Quick => {}
        }
    }
}

/// The notice the Online options show for a failure.
pub(super) const fn notice_text(notice: &OnlineNotice) -> &'static str {
    match notice {
        OnlineNotice::SignInFailed => "ID_ONLINE_SIGNIN_FAILED_EA_SERVER",
        OnlineNotice::NoMatch => "ID_SIDEKICK_ONLINE_QUICK_MATCH_NO_MATCH",
        OnlineNotice::MatchFailed => "ID_ONLINE_MATCHMAKING_NOSERVER",
        OnlineNotice::Disconnected => "ID_ONLINE_GAME_LEFT",
        OnlineNotice::Kicked => "ID_ONLINE_KICKED_BY_GAME_HOST",
        OnlineNotice::UpdateRequired => "#Your game doesn't match the server's version. Update the game to play online.",
        OnlineNotice::NicknameNotPermitted => "#Nickname not permitted.",
        // ShowErrorDialog 0x825735d0: eSk8FEPopup_Online_Error_DLC_Required, ID_EANATION and OK.
        OnlineNotice::DlcRequired => "ID_DLC_REQUIRED",
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The front end's titles in English, by `StringHash`, for the lists' order.
    fn online_titles(&mut self) -> Result<Arc<BTreeMap<u32, String>>, SelectedLiveError> {
        if let Some(titles) = &self.online.titles {
            return Ok(Arc::clone(titles));
        }
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
        let histogram = Histogram::parse(&mounts.read_path(&format!("{LANGUAGE}_HISTOGRAM_skate2ng.BIN")).map_err(error)?).map_err(error)?;
        let mut titles = BTreeMap::new();
        for category in ["Global", "FrontEnd"] {
            let bytes = mounts.read_path(&format!("{LANGUAGE}_{category}_skate2ng.BIN")).map_err(error)?;
            let category = LanguageCategory::parse(&bytes).map_err(error)?;
            for (hash, packed) in category.packed_strings() {
                if let Ok(decoded) = histogram.unpack(packed, 16384, false)
                    && decoded.valid
                {
                    titles.entry(hash).or_insert_with(|| String::from_utf16_lossy(&decoded.units));
                }
            }
        }
        let titles = Arc::new(titles);
        self.online.titles = Some(Arc::clone(&titles));
        Ok(titles)
    }

    /// `challengeArray` and each type's locations: the unranked match types with challenges.
    pub(super) fn online_match_lists(&mut self) -> Result<Vec<(u8, Vec<OnlineChallenge>)>, SelectedLiveError> {
        self.online_data()?;
        let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let types = catalog::unranked_match_types(db, &|name| self.online_location_known(name));
        let mut lists = Vec::with_capacity(types.len());
        for net_type in types {
            lists.push((net_type, self.online_challenge_list(net_type)?));
        }
        Ok(lists)
    }

    /// `GetOnlineChallengesByType` of a type, at the world's locations, sorted by English title.
    pub(super) fn online_challenge_list(&mut self, net_type: u8) -> Result<Vec<OnlineChallenge>, SelectedLiveError> {
        let titles = self.online_titles()?;
        self.online_data()?;
        let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let localize = |label: &str| titles.get(&string_hash(label.as_bytes())).cloned().unwrap_or_else(|| label.to_owned());
        Ok(catalog::online_challenges(db, net_type, &|name| self.online_location_known(name), &localize))
    }

    /// The Online options, each front-end update: the sign-in state its rows read, the screen it
    /// asked for, and any open screen's input. The input the main menu should still see.
    pub(in super::super) fn update_online_menu(&mut self, input: PauseMenuInput, focused: bool) -> Result<PauseMenuInput, SelectedLiveError> {
        let signed_in = self.online.signed_in();
        // In a game the notices open over it (`update_online_popup`).
        let Some(menu) = self.main_menu.as_mut() else {
            self.online.screen = None;
            self.online.browser = None;
            return Ok(input);
        };
        let notices = self.online.take_notices();
        let Some(reel) = menu.skate_reel.as_mut() else {
            self.online.screen = None;
            self.online.browser = None;
            return Ok(input);
        };
        reel.signed_in = signed_in;
        let action = reel.take_online_action();
        for notice in &notices {
            // A game the browser joined but cannot play fails over the list; a lost connection
            // closes the browser for the Online options' notice.
            if matches!(notice, OnlineNotice::NoMatch | OnlineNotice::MatchFailed)
                && let Some(browser) = self.online.browser.as_mut()
            {
                browser.fail(Notice::Failed);
                continue;
            }
            if self.online.browser.take().is_some() {
                reel.ready = false;
            }
            if let Some(screen) = self.online.screen.as_mut() {
                screen.matchmaking_failed();
            }
            if matches!(self.online.screen, Some(MatchScreen::Quick)) {
                self.online.screen = None;
            }
            reel.online_notice(notice_text(notice));
        }
        if let Some(action) = action {
            self.open_online_screen(action)?;
        }
        if self.online.browser.is_some() {
            self.update_lobby_browser(input, focused);
            return Ok(PauseMenuInput::default());
        }
        if self.online.screen.is_none() {
            return Ok(input);
        }
        if !focused {
            return Ok(PauseMenuInput::default());
        }
        if matches!(self.online.screen, Some(MatchScreen::Quick)) {
            // Popup 38 is not cancellable; the search ends in a game or a notice.
            return Ok(PauseMenuInput::default());
        }
        let Some((action, sounds)) = self.online.screen.as_mut().map(|screen| screen.update(input)) else { return Ok(PauseMenuInput::default()) };
        self.extend_menu_sounds(sounds);
        match action {
            Action::None => {}
            Action::Back => {
                self.online.screen = None;
                // The pop enters MainMenuOnlineOptions again, whose screen takes input once
                // its intro completes.
                if let Some(reel) = self.main_menu.as_mut().and_then(|menu| menu.skate_reel.as_mut()) {
                    reel.ready = false;
                }
            }
            Action::Start(choice) => self.start_match_choice(choice),
        }
        Ok(PauseMenuInput::default())
    }

    pub(super) fn extend_menu_sounds(&mut self, sounds: Vec<&'static str>) {
        if let Some(reel) = self.main_menu.as_mut().and_then(|menu| menu.skate_reel.as_mut()) {
            reel.extend_sounds(sounds);
        }
    }

    /// `GoToCustomFreeSkate`, `GoToUnrankedCustomMatch`, the quick match or the Lobby Browser.
    fn open_online_screen(&mut self, action: OnlineAction) -> Result<(), SelectedLiveError> {
        trace(format_args!("online options: {action:?}"));
        self.online.screen = Some(match action {
            OnlineAction::Freeskate => MatchScreen::Freeskate(FreeskateMatch::new(self.online_challenge_list(challenge::ONLINE_FREE_SKATE)?)),
            OnlineAction::CustomMatch => MatchScreen::Custom(CustomMatch::new(self.online_match_lists()?)),
            OnlineAction::QuickMatch => {
                if let Some(reel) = self.main_menu.as_mut().and_then(|menu| menu.skate_reel.as_mut()) {
                    reel.matching();
                }
                self.online_quick_match();
                MatchScreen::Quick
            }
            OnlineAction::LobbyBrowser => {
                self.open_lobby_browser();
                return Ok(());
            }
        });
        Ok(())
    }

    /// `DoQuickMatch` 0x827737d0 under the loading popup: any public unranked game, never a new
    /// one.
    pub(super) fn online_quick_match(&mut self) {
        self.online_play_now(MatchRequest {
            mode: PlayNowMode::Quick,
            ranked: false,
            challenge_type: challenge::SKATE,
            challenge_key: None,
            private: false,
            create: false,
            fallback_key: 0,
        });
    }

    /// `SendUnrankedMatchMessage` 0x826a85a0: Create is a private game at the chosen
    /// challenge, Join a public search of the type at the chosen location (any when none) that
    /// starts a public game there, or at one of the type's, when nothing matches.
    pub(super) fn start_match_choice(&mut self, choice: MatchChoice) {
        let fallback_key = choice.challenge_key.or_else(|| {
            let index = usize::try_from(self.completed_frames).unwrap_or(0) % choice.candidates.len().max(1);
            choice.candidates.get(index).copied()
        });
        let Some(fallback_key) = fallback_key else { return };
        self.online_play_now(MatchRequest {
            mode: choice.mode,
            ranked: false,
            challenge_type: choice.challenge_type,
            challenge_key: choice.challenge_key,
            private: choice.create,
            create: choice.create,
            fallback_key,
        });
    }

    /// The open matchmaking screen's state, for its movie.
    #[must_use]
    pub fn online_match_snapshot(&self) -> Option<Snapshot> {
        self.online.screen.as_ref()?.snapshot()
    }

    /// The matchmaking screen's or the Lobby Browser's intro completed
    /// (`ScreenManager.IntroComplete`): it takes input.
    pub fn online_match_screen_ready(&mut self) {
        if let Some(screen) = self.online.screen.as_mut() {
            screen.set_ready();
        }
        self.lobby_browser_ready();
    }
}
