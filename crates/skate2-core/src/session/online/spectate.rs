//! Spectating another player from an online game's pre-game lobby (PC port addition): a free
//! skate's lobby skate with no activity under way, or a challenge game's, before its challenge or
//! while one waits there through it, watching another who waits too. The player list's
//! Spectate, in place of View Gamercard, shows the player's skater from its owner's game camera, as
//! a challenge's `Online.SwitchCamera` (0x8288a4e0) shows another player's turn: from the replay
//! frames every member sends the others at the base rate, whose camera samples (twenty a second)
//! the view blends between. The skater's root moves the presentation's stream focus
//! (`Pres_OnlineStreamerController::UpdateStreamer` 0x8277d310), so the city streams around it
//! however far away it skates, and its audio is the local skater's.
//!
//! The local skater stands pinned without pad input while the console watches. B (the skate.TV
//! overlay's Stop Spectating), the player leaving, no longer showing or going into the challenge,
//! a change of location, a challenge or activity loading with the local player, a travel or the
//! game ending stops it. A move of the view to where the presentation has not streamed, and back,
//! goes behind the fade until it has, as `cTeleportManager::TeleportRequiresLoadScreen`
//! (0x828a9420) puts a teleport the presentation focus has not loaded for behind a load screen.

use super::{SelectedLiveError, SelectedLiveSession, error, trace};
use crate::ui::FeSoundRequest;
use crate::ui::pause_menu::PauseMenuInput;
use skate2_net::protocol::{Phase, challenge};
use std::time::{Duration, Instant};

/// How long a move of the view waits behind the fade for the presentation to stream before the
/// fade lifts all the same.
const STREAM_WAIT: Duration = Duration::from_secs(6);

/// The console watching another player's skater.
pub(super) struct Spectate {
    /// The player watched, or none once the view goes back to the local skater.
    target: Option<u64>,
    /// The skater the view shows: the target once the move is made, none for the local one.
    shown: Option<u64>,
    /// The fade covers for a move of the view.
    covering: bool,
    /// The view moved behind the fade, which lifts once the presentation streams there.
    streaming: Option<Instant>,
}

impl Spectate {
    /// The player whose skater the view shows.
    pub(super) const fn shown(&self) -> Option<u64> {
        self.shown
    }

    /// The fade covers or holds for a move of the view.
    const fn behind_fade(&self) -> bool {
        self.covering || self.streaming.is_some()
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// Whether the console watches another player, or is on its way to or back from it: the
    /// local skater stands pinned and the pad drives nothing.
    pub(in super::super) fn online_spectating(&self) -> bool {
        self.online.spectate.is_some()
    }

    /// The player the view shows while it watches them, for the skate.TV overlay; none on the way
    /// back.
    #[must_use]
    pub fn online_spectated_name(&self) -> Option<String> {
        let spectate = self.online.spectate.as_ref()?;
        let user_id = spectate.shown.filter(|&shown| spectate.target == Some(shown))?;
        Some(self.online.game.as_ref()?.name(user_id)).filter(|name| !name.is_empty())
    }

    /// Whether the console can watch `user_id`: both wait in the game's pre-game lobby (no round of
    /// the local player's, and the other none of the challenge's players), in a free skate with no
    /// activity under way or loading, with no change of location counting down, and the player's
    /// skater shows.
    pub(super) fn online_can_spectate(&self, user_id: u64) -> bool {
        let Some(game) = self.online.game.as_ref() else { return false };
        let free_skate = game.attributes.challenge_type == challenge::ONLINE_FREE_SKATE;
        game.entered
            && self.online.round.is_none()
            && !game.participants.contains(&user_id)
            && (!free_skate || game.phase == Phase::LobbySkate)
            && self.online.changing_location.is_none()
            && self.map_travel.is_none()
            && game.remotes.get(&user_id).is_some_and(super::remote_skater::RemoteSkater::visible)
    }

    /// The player list's Spectate: the console watches `user_id`'s skater, behind the fade where
    /// the presentation has not streamed around it. Another player's Spectate moves the view on
    /// from the one watched.
    pub(super) fn online_start_spectating(&mut self, user_id: u64) -> Result<(), SelectedLiveError> {
        if !self.online_can_spectate(user_id) || self.online.spectate.as_ref().is_some_and(|spectate| spectate.target == Some(user_id)) {
            return Ok(());
        }
        let root = self.online.game.as_ref().and_then(|game| game.remotes.get(&user_id)).and_then(super::remote_skater::RemoteSkater::root);
        let streamed = match root {
            Some(position) => self.online_presentation_streamed(position)?,
            None => true,
        };
        trace(format_args!("spectating {user_id:016x}{}", if streamed { "" } else { " behind the fade" }));
        if self.online.spectate.is_none() {
            self.set_local_spectating_pin(true)?;
        }
        let spectate = self.online.spectate.get_or_insert(Spectate { target: None, shown: None, covering: false, streaming: None });
        spectate.target = Some(user_id);
        if spectate.behind_fade() {
            // The view moves to the new player once the fade covers.
            spectate.covering = true;
            spectate.streaming = None;
        } else if streamed {
            spectate.shown = Some(user_id);
        } else {
            spectate.covering = true;
            self.frontend_fade.cover();
        }
        Ok(())
    }

    /// B, or the player watched no longer one to watch: the view goes back to the local skater,
    /// behind the fade where the presentation streamed away from it.
    pub(super) fn online_stop_spectating(&mut self) -> Result<(), SelectedLiveError> {
        if self.online.spectate.as_ref().is_none_or(|spectate| spectate.target.is_none()) {
            return Ok(());
        }
        trace(format_args!("spectating stops"));
        let streamed = match self.online_local_root()? {
            Some(position) => self.online_presentation_streamed(position)?,
            None => true,
        };
        let Some(spectate) = self.online.spectate.as_mut() else { return Ok(()) };
        spectate.target = None;
        if spectate.shown.is_none() {
            // Still on the local skater: the fade it took for the move lifts at once.
            if spectate.behind_fade() {
                self.frontend_fade.reveal();
            }
            return self.online_end_spectate();
        }
        if spectate.behind_fade() {
            spectate.covering = true;
            spectate.streaming = None;
            return Ok(());
        }
        if streamed {
            return self.online_end_spectate();
        }
        spectate.covering = true;
        self.frontend_fade.cover();
        Ok(())
    }

    /// B while watching another player, once the front end's screens had the input: the view goes
    /// back to the local skater. B with LB held (`cycle`, LB's quick menu) moves it on to the next
    /// player instead, or starts watching the first.
    pub(in super::super) fn online_spectate_input(&mut self, input: PauseMenuInput, cycle: bool) -> Result<(), SelectedLiveError> {
        if self.online.chat.is_some() {
            return Ok(());
        }
        if cycle {
            if !self.online_spectate_offered() {
                return Ok(());
            }
            self.online.hub.play(FeSoundRequest::Play("FE_SOUND_CORE_B_BUTTON"));
            return self.online_cycle_spectate();
        }
        if !input.cancel || self.online.spectate.as_ref().is_none_or(|spectate| spectate.target.is_none()) {
            return Ok(());
        }
        self.online.hub.play(FeSoundRequest::Play("FE_SOUND_CORE_B_BUTTON"));
        self.online_stop_spectating()
    }

    /// Whether LB's quick menu offers B Spectate (PC port addition): another player can be
    /// watched from the pre-game lobby, or one is.
    pub(in super::super) fn online_spectate_offered(&self) -> bool {
        let Some(game) = self.online.game.as_ref() else { return false };
        self.online.spectate.as_ref().is_some_and(|spectate| spectate.target.is_some())
            || game.remotes.keys().any(|&user_id| self.online_can_spectate(user_id))
    }

    /// LB + B (PC port addition): the next player to watch in the game's join order, the first
    /// one able to be watched after the one watched (from the first when watching no one); past the
    /// last, the view goes back to the local skater. Players are followed by user id, so one who
    /// left is no gap: the order is the players there now.
    fn online_cycle_spectate(&mut self) -> Result<(), SelectedLiveError> {
        let Some(game) = self.online.game.as_ref() else { return Ok(()) };
        let local = self.online.user_id;
        let order: Vec<u64> = game.players.iter().map(|player| player.user_id).filter(|&user_id| Some(user_id) != local).collect();
        let watched = self.online.spectate.as_ref().and_then(|spectate| spectate.target);
        let from = watched.and_then(|watched| order.iter().position(|&user_id| user_id == watched)).map_or(0, |at| at + 1);
        match order.get(from..).and_then(|rest| rest.iter().copied().find(|&user_id| self.online_can_spectate(user_id))) {
            Some(next) => self.online_start_spectating(next),
            None => self.online_stop_spectating(),
        }
    }

    /// Each frame: the watched player still to be watched, and a move of the view behind the fade
    /// made once it covers and ended once the presentation streams there. A travel of the local
    /// skater, a round of its own or the game ending takes the fade with the view.
    pub(super) fn update_online_spectate(&mut self) -> Result<(), SelectedLiveError> {
        let Some(target) = self.online.spectate.as_ref().map(|spectate| spectate.target) else { return Ok(()) };
        let entered = self.online.game.as_ref().is_some_and(|game| game.entered);
        if !entered || self.map_travel.is_some() || self.online.round.is_some() || self.online.start.is_some() {
            return self.online_end_spectate();
        }
        if let Some(user_id) = target
            && !self.online_can_spectate(user_id)
        {
            trace(format_args!("{user_id:016x} can no longer be watched"));
            self.online_stop_spectating()?;
        }
        let covered = self.frontend_fade.covered();
        let Some(spectate) = self.online.spectate.as_mut() else { return Ok(()) };
        if spectate.covering {
            if !covered {
                return Ok(());
            }
            spectate.covering = false;
            spectate.shown = spectate.target;
            spectate.streaming = Some(Instant::now());
        }
        let Some(since) = spectate.streaming else { return Ok(()) };
        let shown = spectate.shown;
        let position = match shown {
            Some(user_id) => self.online.game.as_ref().and_then(|game| game.remotes.get(&user_id)).and_then(super::remote_skater::RemoteSkater::root),
            None => self.online_local_root()?,
        };
        let streamed = match position {
            Some(position) => self.online_presentation_streamed(position)?,
            None => true,
        };
        if !streamed && since.elapsed() < STREAM_WAIT {
            return Ok(());
        }
        self.frontend_fade.reveal();
        let Some(spectate) = self.online.spectate.as_mut() else { return Ok(()) };
        spectate.streaming = None;
        if spectate.target.is_none() {
            return self.online_end_spectate();
        }
        Ok(())
    }

    /// The console watches no one any more: the local skater is free again. The fade is left to
    /// whatever holds it.
    pub(super) fn online_end_spectate(&mut self) -> Result<(), SelectedLiveError> {
        if self.online.spectate.take().is_none() {
            return Ok(());
        }
        trace(format_args!("spectating over"));
        self.set_local_spectating_pin(false)
    }

    /// `cMsgPinPlayer` of the local skater for the time it stands watching.
    fn set_local_spectating_pin(&self, pin: bool) -> Result<(), SelectedLiveError> {
        if let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()) {
            actor.try_set_pinned(pin).map_err(|e| error(format!("{e:?}")))?;
        }
        Ok(())
    }

    /// The local skater's position, where the stream focus goes back to.
    fn online_local_root(&self) -> Result<Option<[f32; 3]>, SelectedLiveError> {
        let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_deref()) else { return Ok(None) };
        super::super::local_skater_position(self.inner.world.as_ref().map(|world| &world.world), actor).map(Some)
    }

    /// Whether the presentation has streamed around `position` as a teleport there would need no
    /// load screen (`TeleportRequiresLoadScreen`'s presentation focus within
    /// `kMinPresRadiusForNoLoadScreenOnTelelport`; streamed while the renderer has told nothing).
    fn online_presentation_streamed(&self, position: [f32; 3]) -> Result<bool, SelectedLiveError> {
        let cvars = &self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.cvars;
        let radius = cvars.try_min_pres_radius_for_no_load_screen_on_teleport().map_err(|e| error(format!("{e:?}")))?;
        let xml = cvars.try_enable_stream_xml().map_err(|e| error(format!("{e:?}")))?;
        Ok(self.presentation_area.as_ref().is_none_or(|area| area.is_area_loaded(position, radius, xml)))
    }
}
