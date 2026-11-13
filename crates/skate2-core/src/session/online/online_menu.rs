//! The online front end from the Online options: `FrontEndState_UnrankedFreeskate` (FE state
//! 68, screen 109), `FrontEndState_UnrankedCustomMatch` (FE state 69, screen 110) and the quick
//! match's search under its loading popup, the matchmaking each starts
//! (`SendUnrankedMatchMessage` 0x826a85a0, `StateOnlinePlayNow`), the Lobby Browser (PC port
//! addition) and the notices of failed sign-ins and searches.

use super::{MatchRequest, OnlineNotice, SelectedLiveError, SelectedLiveSession, error, trace};
use crate::online::catalog::{self, OnlineChallenge};
use crate::ui::lobby_browser::Notice;
use crate::ui::online_match::{Action, CustomMatch, FreeskateMatch, MatchChoice, SKATE2_POOL, SKATE3_DLC_POOL, SKATE3_POOL, Snapshot};
use crate::ui::pause_menu::PauseMenuInput;
use crate::ui::skate_reel::OnlineAction;
use skate2_formats::localization::{Histogram, LanguageCategory, string_hash};
use skate2_net::protocol::{PlayNowMode, challenge};
use std::collections::BTreeMap;
use std::sync::Arc;

/// PC port: the location image a linked Skate 3 data folder's maps show.
const SKATE3_IMAGE: &str = crate::skate3::LOCATION_IMAGE;

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

    /// PC port: the linked Skate 3 data's online games of a type (`crate::skate3::OnlineSpot`),
    /// titled in Skate 3's English (literal text, `#`): those at its districts (the Map Pool's
    /// Skate 3), then those at its downloadable content's maps (Skate 3 DLC).
    pub(super) fn online_skate3_lists(net_type: u8) -> (Vec<OnlineChallenge>, Vec<OnlineChallenge>) {
        let Some(data) = crate::skate3::linked() else { return (Vec::new(), Vec::new()) };
        let (mut districts, mut maps) = (Vec::new(), Vec::new());
        for spot in data.online_spots(net_type) {
            let challenge = OnlineChallenge { key: spot.key, title: format!("#{}", spot.title), image: SKATE3_IMAGE.to_owned(), location: spot.location.clone() };
            if crate::skate3::is_district_stream(spot.stream) {
                districts.push(challenge);
            } else {
                maps.push(challenge);
            }
        }
        (districts, maps)
    }

    /// Every online challenge of a type this build can play: the game's, then the linked Skate 3
    /// data's.
    pub(super) fn online_challenge_list_all(&mut self, net_type: u8) -> Result<Vec<OnlineChallenge>, SelectedLiveError> {
        let mut list = self.online_challenge_list(net_type)?;
        let (districts, maps) = Self::online_skate3_lists(net_type);
        list.extend(districts);
        list.extend(maps);
        Ok(list)
    }

    /// The online free skate screen, with the Map Pool row while Skate 3 data is linked and has
    /// online maps (its districts', its downloadable content's); `host` opens it in its host
    /// state, the pool of the game's own location shown.
    pub(super) fn online_freeskate_screen(&mut self, host: Option<bool>) -> Result<FreeskateMatch, SelectedLiveError> {
        let locations = self.online_challenge_list(challenge::ONLINE_FREE_SKATE)?;
        let (districts, maps) = Self::online_skate3_lists(challenge::ONLINE_FREE_SKATE);
        let screen = match host {
            Some(private) => FreeskateMatch::host(locations, private),
            None => FreeskateMatch::new(locations),
        };
        Ok(screen.with_pools(vec![(SKATE3_POOL, districts), (SKATE3_DLC_POOL, maps)], self.online_game_pool()))
    }

    /// The unranked custom match screen, with the Map Pool row while Skate 3 data is linked and
    /// has online games of its types (at its districts, at its downloadable content's maps);
    /// `host` opens it in its host state.
    pub(super) fn online_custom_screen(&mut self, host: Option<bool>) -> Result<CustomMatch, SelectedLiveError> {
        let mut lists = self.online_match_lists()?;
        let (mut districts, mut maps) = (Vec::new(), Vec::new());
        for &(net_type, _) in &lists {
            let (at_districts, at_maps) = Self::online_skate3_lists(net_type);
            if !at_districts.is_empty() {
                districts.push((net_type, at_districts));
            }
            if !at_maps.is_empty() {
                maps.push((net_type, at_maps));
            }
        }
        // PC port, for now: the game's own 1-Ups stay out until their online play is complete;
        // Skate 3's play Skate 3's own graph.
        lists.retain(|&(net_type, _)| net_type != challenge::ONE_UP);
        let screen = match host {
            Some(private) => CustomMatch::host(lists, private),
            None => CustomMatch::new(lists),
        };
        Ok(screen.with_pools(vec![(SKATE3_POOL, districts), (SKATE3_DLC_POOL, maps)], self.online_game_pool()))
    }

    /// Whether the online game's challenge is one of the linked Skate 3 data's.
    pub(super) fn online_game_is_skate3(&self) -> bool {
        self.online.game.as_ref().is_some_and(|game| crate::skate3::linked().is_some_and(|data| data.online_spot(game.attributes.challenge_key).is_some()))
    }

    /// The Map Pool of the online game's challenge: Skate 3's districts' or its downloadable
    /// content's maps' for one of the linked Skate 3 data's, else Skate 2's.
    fn online_game_pool(&self) -> &'static str {
        let spot = self.online.game.as_ref().and_then(|game| crate::skate3::linked().and_then(|data| data.online_spot(game.attributes.challenge_key).map(|spot| spot.stream)));
        match spot {
            Some(stream) if crate::skate3::is_district_stream(stream) => SKATE3_POOL,
            Some(_) => SKATE3_DLC_POOL,
            None => SKATE2_POOL,
        }
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
            OnlineAction::Freeskate => MatchScreen::Freeskate(self.online_freeskate_screen(None)?),
            OnlineAction::CustomMatch => MatchScreen::Custom(self.online_custom_screen(None)?),
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
