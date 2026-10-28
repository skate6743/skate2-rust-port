//! The online hub in the game (`FrontEndState_OnlineHubMenu`, FE state 44) and the screens over
//! it (FE states 69 and 68: Host Options' in an online game, the matchmaking ones from a single
//! player game): what the game knows of the session for the hub's menus, and what the menus ask
//! of the game.

use super::online_menu::{MatchScreen, notice_text};
use super::{CHALLENGE_NAME, CHALLENGES, MatchRequest, SelectedLiveError, SelectedLiveSession, SignIn, error, trace};
use crate::challenge::hud::CentralMessage;
use crate::challenge::invisible_boundary::InvisibleBoundaries;
use crate::challenge::players::{GM_PROFILE_FREE_SKATE, GM_SINGLE_PLAYER};
use crate::ui::FeSoundRequest;
use crate::ui::online_config::OnlineConfig;
use crate::ui::online_hub::{Action, Controller, HUB_SOUND, HubKind, Lobby, Popup, ScreenCall, Snapshot};
use crate::ui::online_match::{self, CustomMatch, FreeskateMatch};
use crate::ui::pause_menu::PauseMenuInput;
use crate::ui::profile_name::Prompt;
use skate2_net::protocol::{ClientMessage, Phase, PlayNowMode, challenge};
use std::sync::Arc;

/// The hub as the session holds it.
#[derive(Default)]
pub(super) struct Hub {
    /// FE state 44 with its screen.
    pub(super) menu: Option<Controller>,
    /// FE state 69 or 68 pushed over it by Host Options.
    pub(super) screen: Option<MatchScreen>,
    /// The hub holds `kPAUSE_STATE_FRONTEND` (`gFEStateInfo[44].mbPause`), which the network's
    /// `ResumeGame` keeps from stopping the simulation.
    paused: bool,
    /// `SetScreenState` calls of the hub's screen not yet taken.
    calls: Vec<ScreenCall>,
    /// The screen opened or reopened since the movie last looked.
    opened: bool,
    sounds: Vec<FeSoundRequest>,
    /// `HUD2.ShowCentralMessage`s outside a challenge (`KickPlayerCallback`).
    pub(super) central_messages: Vec<CentralMessage>,
    /// The sign-in's popup or a notice over a single player game, the hub or a screen over it
    /// (`FrontEndManager::OpenPopup`), which takes the input while it is up.
    popup: Option<Popup>,
}

impl Hub {
    /// A sound of the online front end from outside the hub's screens.
    pub(super) fn play(&mut self, sound: FeSoundRequest) {
        self.sounds.push(sound);
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The session as the hub's menus read it.
    pub(super) fn online_hub_lobby(&self) -> Option<Lobby> {
        let game = self.online.game.as_ref()?;
        let local = self.online.user_id?;
        let in_round = self.online.round.is_some();
        Some(Lobby {
            kind: HubKind::Online,
            nickname: self.online.nickname.clone(),
            host: game.host() == Some(local),
            ranked: game.attributes.ranked,
            private: game.attributes.private,
            free_skate_type: game.attributes.challenge_type == challenge::ONLINE_FREE_SKATE,
            lobby_mode: !in_round,
            // A member waiting in the pre-game lobby plays no round while the game's challenge runs.
            challenge_in_progress: in_round || game.phase != Phase::LobbySkate,
            changing_location: self.online.changing_location.is_some(),
            players: game.players.iter().filter(|player| player.user_id != local).map(|player| (player.user_id, player.name.clone())).collect(),
            participants: game.players.len(),
            min_participants: challenge::min_participants(game.attributes.challenge_type),
            spectatable: game.remotes.keys().copied().filter(|&user_id| self.online_can_spectate(user_id)).collect(),
            activities: self.online_hub_activities(),
            proposal: self.online_proposal_pending(),
        })
    }

    /// `PickCurrentState` 0x82688470 out of an online game: in a single player or free skate game
    /// (`IsSinglePlayerHubMode` 0x82645170) the single player hub, or the offline one before
    /// signing in; `CheckForOnlineHubMenu` 0x82656428 opens neither during a career challenge
    /// (`GetMainChallenge`). `None` where the hub does not open.
    fn single_player_hub_lobby(&self) -> Result<Option<Lobby>, SelectedLiveError> {
        if self.online.game.is_some() || self.online.start.is_some() || self.main_menu.is_some() || self.map_travel.is_some() {
            return Ok(None);
        }
        let Some(mode) = self.game_mode()? else { return Ok(None) };
        let single_player = mode == i64::from(GM_SINGLE_PLAYER);
        if !(single_player || mode == i64::from(GM_PROFILE_FREE_SKATE))
            || (single_player && self.career.as_ref().is_some_and(super::super::career::CareerSession::main_challenge_active))
        {
            return Ok(None);
        }
        let kind = if self.online.signed_in() { HubKind::SinglePlayer } else { HubKind::Offline };
        Ok(Some(Lobby { kind, nickname: self.online.nickname.clone(), ..Lobby::default() }))
    }

    /// The hub's session: the online game's, else the single player game's.
    fn hub_lobby(&self) -> Result<Option<Lobby>, SelectedLiveError> {
        if self.online.game.is_some() { Ok(self.online_hub_lobby()) } else { self.single_player_hub_lobby() }
    }

    /// `CommonBackEnd::sGameInfo.gameMode`, while a game world is up.
    fn game_mode(&self) -> Result<Option<i64>, SelectedLiveError> {
        let Some(owner) = self.inner.world.as_ref() else { return Ok(None) };
        let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
        let Some(services) = world.physical_world_services() else { return Ok(None) };
        let mode = services.game_info().try_game_mode().map_err(|e| error(format!("{e:?}")))?;
        Ok(Some(i64::from(mode)))
    }

    /// `CheckForOnlineHubMenu` 0x82656428: Back opens the hub while the game plays under the
    /// HUD; then the hub, or a screen over it, takes the input, after an open popup. The input
    /// the game should still see.
    pub(in super::super) fn update_online_hub(&mut self, input: PauseMenuInput, focused: bool, can_open: bool) -> Result<PauseMenuInput, SelectedLiveError> {
        let playing = self.online.network_resumed && self.online.game.as_ref().is_some_and(|game| game.entered) && self.map_travel.is_none();
        let input = self.update_online_popup(input, focused)?;
        let lobby = if playing { self.online_hub_lobby() } else { self.single_player_hub_lobby()? };
        let Some(lobby) = lobby else {
            self.close_online_hub()?;
            return Ok(input);
        };
        // FE state 67, over the hub or up on its own from the activity overlay, holds the front
        // end's pause as the hub does (`gFEStateInfo[67].mbPause`).
        if self.online.activity.sign_up.is_some() {
            self.set_online_hub_paused(true)?;
            if focused {
                self.update_online_activity_signup(input)?;
            }
            if !self.online_hub_open() {
                self.set_online_hub_paused(false)?;
            }
            return Ok(PauseMenuInput::default());
        }
        if self.online.hub.screen.is_some() {
            if focused {
                self.update_online_hub_screen(input)?;
            }
            return Ok(PauseMenuInput::default());
        }
        // FE state 67 can close on its own (its time out, its proposal's end): its pause goes.
        if !self.online_hub_open() {
            self.set_online_hub_paused(false)?;
        }
        let Some(hub) = self.online.hub.menu.as_mut() else {
            if focused && can_open && input.back && !self.skater_being_jacked()? {
                self.open_online_hub(lobby)?;
                return Ok(PauseMenuInput::default());
            }
            return Ok(input);
        };
        hub.refresh(lobby);
        let action = if focused { hub.update(input) } else { Action::None };
        let calls = hub.take_calls();
        let sounds = hub.take_sounds();
        self.online.hub.calls.extend(calls);
        self.online.hub.sounds.extend(sounds);
        self.online_hub_action(action)?;
        Ok(PauseMenuInput::default())
    }

    /// `CheckForOnlineHubMenu`'s sound and `PushState(44)`.
    fn open_online_hub(&mut self, lobby: Lobby) -> Result<(), SelectedLiveError> {
        trace(format_args!("online hub opens"));
        self.online.hub.sounds.push(FeSoundRequest::PlayKey(HUB_SOUND));
        self.reopen_online_hub(lobby);
        self.set_online_hub_paused(true)
    }

    /// `OnEnter` 0x8268a3c0: the root menu anew.
    pub(super) fn reopen_online_hub(&mut self, lobby: Lobby) {
        let mut hub = Controller::open(lobby);
        self.online.hub.calls = hub.take_calls();
        self.online.hub.menu = Some(hub);
        self.online.hub.opened = true;
    }

    /// `OnlineFeManager::Exit` 0x8266c418: every online front-end state pops, FE state 67's too.
    pub(super) fn close_online_hub(&mut self) -> Result<(), SelectedLiveError> {
        if self.online.hub.menu.take().is_some() || self.online.hub.screen.take().is_some() {
            trace(format_args!("online hub closes"));
        }
        self.online_close_activity_signup();
        self.online.hub.calls.clear();
        self.set_online_hub_paused(false)
    }

    fn set_online_hub_paused(&mut self, paused: bool) -> Result<(), SelectedLiveError> {
        if self.online.hub.paused != paused
            && let Some(input) = self.inner.bound_input.as_ref()
        {
            input.try_set_frontend_pause(paused).map_err(error)?;
            self.online.hub.paused = paused;
        }
        Ok(())
    }

    /// `ILWChaseMan::IsSkaterBeingJacked`, under which `CheckForOnlineHubMenu` opens nothing.
    fn skater_being_jacked(&self) -> Result<bool, SelectedLiveError> {
        let Some(owner) = self.inner.world.as_ref() else { return Ok(false) };
        let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
        let living = world.grind_services.living_world().lock().map_err(|_| error("living world lock poisoned"))?;
        Ok(living.chase.as_ref().is_some_and(crate::living_world_chase::ChaseMan::is_skater_being_jacked))
    }

    /// The popup over a single player game, which `StateManagerFrontEnd::ReceiveInput` 0x82656af8
    /// hands the input before any state: the sign-in's while it connects, then a notice of what
    /// ended a search or the sign-in, which replaces it and the searching popup
    /// (`StateOnlineFE::HandleErrorState` 0x82755bc0: `ShowErrorDialog`, `CloseLoadingPopup`).
    /// The input the game should still see.
    fn update_online_popup(&mut self, input: PauseMenuInput, focused: bool) -> Result<PauseMenuInput, SelectedLiveError> {
        if self.main_menu.is_some() || self.main_menu_pending || self.online.game.is_some() {
            self.online.hub.popup = None;
            return Ok(input);
        }
        if focused {
            match self.online.hub.popup {
                // B: `CancelPopupCallback` 0x82748ca0 and `CancelSignIn` 0x82748bd0 disconnect,
                // and `ConnectingToPlasma` 0x8275e730 opens popup 37.
                Some(Popup::SigningIn) if input.cancel && self.online.sign_in == SignIn::Connecting => {
                    trace(format_args!("sign-in cancelled"));
                    self.online.client = None;
                    self.online.sign_in = SignIn::Offline;
                    self.online.user_id = None;
                    self.open_online_popup(Popup::Notice("ID_ONLINE_SIGNIN_CANCELLED"));
                }
                // OK: `HandleErrorState` closes every popup and pops to the game (`PopToState(0)`,
                // or `OnlineFeManager::Exit` 0x8266c418), the hub and its screens with them.
                Some(Popup::Notice(_)) if input.select => {
                    self.online.hub.popup = None;
                    self.online.hub.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
                    self.close_online_hub()?;
                }
                _ => {}
            }
        }
        for notice in self.online.take_notices() {
            if let Some(hub) = self.online.hub.menu.as_mut() {
                hub.search_ended();
            }
            self.open_online_popup(Popup::Notice(notice_text(&notice)));
        }
        // `ConnectingToPlasma`'s exit closes the sign-in's popup once it signs in.
        if self.online.hub.popup == Some(Popup::SigningIn) && self.online.sign_in != SignIn::Connecting {
            self.online.hub.popup = None;
        }
        Ok(if self.online.hub.popup.is_some() { PauseMenuInput::default() } else { input })
    }

    /// `FrontEndState_Popup::OnEnter` 0x826583d0 plays the popup sound.
    fn open_online_popup(&mut self, popup: Popup) {
        self.online.hub.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
        self.online.hub.popup = Some(popup);
    }

    /// A sign-in from the game connects under `ConnectToPlasma` 0x8275df08's popup 33
    /// (`StateSmartPhone::FilterImpl` 0x82779130 signs in cancellable).
    pub(super) fn online_signing_in(&mut self) {
        if self.main_menu.is_none() && self.online.sign_in == SignIn::Connecting {
            self.open_online_popup(Popup::SigningIn);
        }
    }

    /// A search from the single player hub, under its searching popup while it runs.
    fn online_hub_search(&mut self, search: impl FnOnce(&mut Self)) {
        search(self);
        if self.online.searching.is_some()
            && let Some(hub) = self.online.hub.menu.as_mut()
        {
            hub.matching();
        }
    }

    /// `GoToFreeSkateHere` 0x8267bee8 and `StateOnlineFreeskateHere`: the online free skate
    /// location the skater is in by its boundary, tested nearest location first (`DoFindFreeskate`
    /// 0x82778950, `FindClosestFreeskateByDistance` 0x82778120, `LoadBoundary` 0x8274e7e0), else
    /// the one whose boundary is nearest; none without a location.
    fn online_free_skate_here(&mut self) -> Result<Option<u64>, SelectedLiveError> {
        let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_deref()) else { return Ok(None) };
        let position = super::super::local_skater_position(self.inner.world.as_ref().map(|world| &world.world), actor)?;
        let locations = self.online_challenge_list(challenge::ONLINE_FREE_SKATE)?;
        let (db, table) = self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?;
        let mut nearest: Vec<(f32, u64, String)> = locations
            .iter()
            .filter_map(|location| {
                let transform = table.find(&location.location)?.transform;
                let name = db.attribute(CHALLENGES, location.key, CHALLENGE_NAME)?.text(0)?.to_owned();
                let (x, z) = (transform[12] - position[0], transform[14] - position[2]);
                Some((x.mul_add(x, z * z), location.key, name))
            })
            .collect();
        nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
        let mut best = nearest.first().map(|&(_, key, _)| key);
        let mut closest = f32::MAX;
        for (_, key, name) in &nearest {
            let boundaries = InvisibleBoundaries::load(&mounts, name);
            if boundaries.is_empty() {
                continue;
            }
            match boundaries.outside_distance_squared(position) {
                None => return Ok(Some(*key)),
                Some(distance) if distance < closest => {
                    closest = distance;
                    best = Some(*key);
                }
                Some(_) => {}
            }
        }
        Ok(best)
    }

    fn online_hub_action(&mut self, action: Action) -> Result<(), SelectedLiveError> {
        match action {
            Action::None => {}
            Action::Exit => self.close_online_hub()?,
            Action::PublicFreeskate => match self.online_free_skate_here()? {
                Some(key) => {
                    trace(format_args!("public free skate here: {key:016x}"));
                    self.online_hub_search(|session| {
                        session.online_play_now(MatchRequest {
                            mode: PlayNowMode::Freeskate,
                            ranked: false,
                            challenge_type: challenge::ONLINE_FREE_SKATE,
                            challenge_key: Some(key),
                            private: false,
                            create: false,
                            fallback_key: key,
                        });
                    });
                }
                None => self.open_online_popup(Popup::Notice("ID_ONLINE_MATCHMAKING_NOSERVER")),
            },
            Action::CustomFreeskate => {
                let locations = self.online_challenge_list(challenge::ONLINE_FREE_SKATE)?;
                self.online.hub.menu = None;
                self.online.hub.screen = Some(MatchScreen::Freeskate(FreeskateMatch::new(locations)));
            }
            Action::QuickMatch => self.online_hub_search(Self::online_quick_match),
            Action::CustomMatch => {
                let lists = self.online_match_lists()?;
                self.online.hub.menu = None;
                self.online.hub.screen = Some(MatchScreen::Custom(CustomMatch::new(lists)));
            }
            // GoToSignIn: the hub closes and the game signs in under its popup, with the nickname
            // the Online options ask first when the file has none.
            Action::SignIn => {
                self.close_online_hub()?;
                if OnlineConfig::load().has_nickname() {
                    self.online_sign_in();
                    self.online_signing_in();
                } else {
                    self.online.nickname_prompt = Some(Prompt::nickname_replacing(&OnlineConfig::load().nickname));
                }
            }
            Action::StartChallenge => self.online_start_challenge(),
            Action::MatchSetup => {
                let private = self.online.game.as_ref().is_some_and(|game| game.attributes.private);
                let lists = self.online_match_lists()?;
                self.online.hub.menu = None;
                self.online.hub.screen = Some(MatchScreen::Custom(CustomMatch::host(lists, private)));
            }
            Action::ChangeLocation => {
                let private = self.online.game.as_ref().is_some_and(|game| game.attributes.private);
                let locations = self.online_challenge_list(challenge::ONLINE_FREE_SKATE)?;
                self.online.hub.menu = None;
                self.online.hub.screen = Some(MatchScreen::Freeskate(FreeskateMatch::host(locations, private)));
            }
            Action::SetPrivate(private) => {
                self.online_set_private(private);
                self.close_online_hub()?;
            }
            Action::Kick(user_id) => {
                self.online_kick(user_id);
                self.online.hub.central_messages.push(CentralMessage { hal_id: "ID_CENTRALMESSAGE_PLAYERKICKED".to_owned(), colour: 0, args: Default::default(), audio_hook: 0 });
                self.close_online_hub()?;
            }
            Action::Spectate(user_id) => {
                self.close_online_hub()?;
                self.online_start_spectating(user_id)?;
            }
            Action::ActivitySignUp(key) => self.online_open_activity_signup(key),
        }
        Ok(())
    }

    /// FE state 69 or 68 over the hub: B (`ReceiveInput` 0x8269b368, `ReturnToPreviousScreen`)
    /// pops back to the hub, whose `OnEnter` starts at its root; A sends the host's change
    /// (`SendUnrankedMatchMessage` 0x826a85a0: `CloseOnlineHubMenu`, `Message_GameRequestChange`),
    /// or from a single player game the screen's search, as from the Online options.
    fn update_online_hub_screen(&mut self, input: PauseMenuInput) -> Result<(), SelectedLiveError> {
        let Some((action, sounds)) = self.online.hub.screen.as_mut().map(|screen| screen.update(input)) else { return Ok(()) };
        self.online.hub.sounds.extend(sounds.into_iter().map(FeSoundRequest::Play));
        match action {
            online_match::Action::None => {}
            online_match::Action::Back => {
                self.online.hub.screen = None;
                if let Some(lobby) = self.hub_lobby()? {
                    self.reopen_online_hub(lobby);
                }
            }
            online_match::Action::Start(choice) if self.online.game.is_none() => self.start_match_choice(choice),
            online_match::Action::Start(choice) => {
                if let Some(challenge_key) = choice.challenge_key
                    && self.online_is_host()
                {
                    trace(format_args!("host changes the game to {challenge_key:016x} (type {})", choice.challenge_type));
                    self.online.send(ClientMessage::RequestChange { challenge_type: choice.challenge_type, challenge_key });
                }
                self.close_online_hub()?;
            }
        }
        Ok(())
    }

    /// Whether the hub or a screen over it is up (FE state 44, 68 or 69 on the stack), or FE
    /// state 67.
    #[must_use]
    pub fn online_hub_open(&self) -> bool {
        self.online.hub.menu.is_some() || self.online.hub.screen.is_some() || self.online.activity.sign_up.is_some()
    }

    /// Whether the sign-in's popup or a notice is up over the single player game.
    pub(in super::super) fn online_popup_open(&self) -> bool {
        self.online.hub.popup.is_some()
    }

    /// The hub's menus for its screen, while it is up.
    #[must_use]
    pub fn online_hub_snapshot(&self) -> Option<Snapshot> {
        self.online.hub.menu.as_ref().map(Controller::snapshot)
    }

    /// The popup over the game, the hub or a screen over it, for the hub's movie.
    #[must_use]
    pub fn online_hub_popup(&self) -> Option<Popup> {
        self.online.hub.popup.clone().or_else(|| self.online_activity_popup()).or_else(|| self.online.hub.menu.as_ref().and_then(Controller::popup).cloned())
    }

    /// The hub's screen opened since the last call, and its `SetScreenState` calls since then.
    pub fn take_online_hub_calls(&mut self) -> (bool, Vec<ScreenCall>) {
        (std::mem::take(&mut self.online.hub.opened), std::mem::take(&mut self.online.hub.calls))
    }

    /// The Host Options screen over the hub, for its movie.
    #[must_use]
    pub fn online_hub_match_snapshot(&self) -> Option<online_match::Snapshot> {
        self.online.hub.screen.as_ref()?.snapshot()
    }

    /// The hub's screen completed its intro.
    pub fn online_hub_ready(&mut self) {
        if let Some(hub) = self.online.hub.menu.as_mut() {
            hub.ready = true;
        }
    }

    /// The Host Options screen completed its intro.
    pub fn online_hub_screen_ready(&mut self) {
        if let Some(screen) = self.online.hub.screen.as_mut() {
            screen.set_ready();
        }
    }

    /// Central messages the online game showed outside a challenge.
    pub fn take_online_central_messages(&mut self) -> Vec<CentralMessage> {
        std::mem::take(&mut self.online.hub.central_messages)
    }

    pub(in super::super) fn take_online_hub_sounds(&mut self) -> Vec<FeSoundRequest> {
        std::mem::take(&mut self.online.hub.sounds)
    }
}
