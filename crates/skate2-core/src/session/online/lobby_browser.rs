//! The Lobby Browser's requests (PC port addition): the list, asked for on opening, on X and
//! after a failed join's notice, never more than once a second (the server answers that many,
//! `LOBBY_LIST_RATE`) and asked for again when unanswered; and A's join of the selected game,
//! whose `PlayNowResult` failure opens its notice over the list.

use super::{SelectedLiveError, SelectedLiveSession, trace};
use crate::online::catalog::{self, OnlineChallenge};
use crate::ui::lobby_browser::{Action, Lobby, LobbyBrowser, LobbyPhase, Notice, Snapshot, State};
use crate::ui::pause_menu::PauseMenuInput;
use skate2_net::protocol::{ClientMessage, LobbyInfo, Phase, challenge, play_now};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::time::{Duration, Instant};

/// The shortest time between two requests for the list.
const REQUEST_INTERVAL: Duration = Duration::from_secs(1);
/// A list not received by then is asked for again.
const LIST_TIMEOUT: Duration = Duration::from_secs(5);
/// A join the server has not answered by then failed.
const JOIN_TIMEOUT: Duration = Duration::from_secs(10);

/// The open browser and its requests.
pub(super) struct Browser {
    screen: LobbyBrowser,
    /// When the list was last asked for.
    requested: Option<Instant>,
    /// The list is wanted but not yet asked for.
    pending: bool,
    /// When A asked to join.
    joining: Option<Instant>,
}

impl Browser {
    /// A join failed: its notice over the list.
    pub(super) fn fail(&mut self, notice: Notice) {
        self.joining = None;
        self.screen.join_failed(notice);
    }

    /// Whether the list may be asked for now: wanted, or unanswered for `LIST_TIMEOUT`, and the
    /// last request a second old.
    fn request_due(&mut self, now: Instant) -> bool {
        let since = |at: Instant| now.saturating_duration_since(at);
        if self.screen.state() == State::Loading && !self.pending && self.requested.is_some_and(|at| since(at) >= LIST_TIMEOUT) {
            self.pending = true;
        }
        if !self.pending || self.requested.is_some_and(|at| since(at) < REQUEST_INTERVAL) {
            return false;
        }
        self.pending = false;
        self.requested = Some(now);
        true
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The Online options' Lobby Browser row: the browser opens and asks for the list.
    pub(super) fn open_lobby_browser(&mut self) {
        self.online.browser = Some(Browser { screen: LobbyBrowser::new(), requested: None, pending: true, joining: None });
        self.lobby_browser_requests(Instant::now());
    }

    /// The list's request once due, and the end of a join the server never answered.
    fn lobby_browser_requests(&mut self, now: Instant) {
        let Some(browser) = self.online.browser.as_mut() else { return };
        if browser.joining.is_some_and(|at| now.saturating_duration_since(at) >= JOIN_TIMEOUT) {
            browser.fail(Notice::Failed);
        }
        if browser.request_due(now) {
            trace(format_args!("lobby browser lists the games"));
            self.online.send(ClientMessage::ListLobbies);
        }
    }

    /// The browser's update in the Online options: its requests, then its input while the
    /// window has focus.
    pub(super) fn update_lobby_browser(&mut self, input: PauseMenuInput, focused: bool) {
        let now = Instant::now();
        self.lobby_browser_requests(now);
        let Some(browser) = self.online.browser.as_mut() else { return };
        let action = if focused { browser.screen.update(input) } else { Action::None };
        let sounds = browser.screen.take_sounds();
        self.extend_menu_sounds(sounds);
        match action {
            Action::None => {}
            Action::Back => {
                self.online.browser = None;
                if let Some(reel) = self.main_menu.as_mut().and_then(|menu| menu.skate_reel.as_mut()) {
                    reel.ready = false;
                }
            }
            Action::Refresh => {
                if let Some(browser) = self.online.browser.as_mut() {
                    browser.pending = true;
                }
                self.lobby_browser_requests(now);
            }
            Action::Join(game) => {
                trace(format_args!("lobby browser joins game {game}"));
                let free = self.online.signed_in() && self.online.searching.is_none() && self.online.game.is_none();
                if let Some(browser) = self.online.browser.as_mut() {
                    if free {
                        browser.joining = Some(now);
                    } else {
                        browser.fail(Notice::Failed);
                    }
                }
                if free {
                    self.online.send(ClientMessage::JoinLobby { game });
                }
            }
        }
    }

    /// `Lobbies`: a card for each game this build's data has the challenge of, in the server's
    /// order, while the browser waits for the list.
    pub(super) fn online_lobbies(&mut self, lobbies: &[LobbyInfo]) -> Result<(), SelectedLiveError> {
        if !self.online.browser.as_ref().is_some_and(|browser| browser.screen.state() == State::Loading) {
            return Ok(());
        }
        let mut lists: BTreeMap<u8, Vec<OnlineChallenge>> = BTreeMap::new();
        let mut cards = Vec::with_capacity(lobbies.len());
        for lobby in lobbies {
            if let Entry::Vacant(entry) = lists.entry(lobby.challenge_type) {
                entry.insert(self.online_challenge_list_all(lobby.challenge_type)?);
            }
            let Some(known) = lists.get(&lobby.challenge_type).and_then(|list| list.iter().find(|known| known.key == lobby.challenge_key)) else {
                continue;
            };
            cards.push(Lobby {
                game: lobby.game,
                title: known.title.clone(),
                image: crate::skate3::small_location_image(&known.image),
                mode: catalog::challenge_title(lobby.challenge_type),
                phase: (lobby.challenge_type != challenge::ONLINE_FREE_SKATE)
                    .then_some(if lobby.phase == Phase::LobbySkate { LobbyPhase::PreGame } else { LobbyPhase::InGame }),
                players: lobby.players,
                max_players: lobby.max_players,
            });
        }
        trace(format_args!("lobby browser lists {} of {} games", cards.len(), lobbies.len()));
        if let Some(browser) = self.online.browser.as_mut() {
            browser.pending = false;
            browser.screen.listed(cards);
        }
        Ok(())
    }

    /// A join's `PlayNowResult` failure: full, gone (`SER_PLAYNOW_NO_MATCH`) or another.
    pub(super) fn lobby_join_failed(&mut self, result: u8) {
        let Some(browser) = self.online.browser.as_mut().filter(|browser| browser.joining.is_some()) else { return };
        browser.fail(match result {
            play_now::FULL => Notice::Full,
            play_now::NO_MATCH => Notice::Gone,
            _ => Notice::Failed,
        });
    }

    /// The open Lobby Browser's state, for its screen.
    #[must_use]
    pub fn lobby_browser_snapshot(&self) -> Option<Snapshot> {
        self.online.browser.as_ref().map(|browser| browser.screen.snapshot())
    }

    /// The Lobby Browser's screen finished its intro: it takes input.
    pub(super) fn lobby_browser_ready(&mut self) {
        if let Some(browser) = self.online.browser.as_mut() {
            browser.screen.ready = true;
        }
    }
}
