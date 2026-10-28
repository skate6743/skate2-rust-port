//! Freeskate activities in an online free skate: the proposal a member makes from the hub's
//! Freeskate Activities (`GoToStartFreeskateActivity` 0x8266c728), its overlay
//! (`FreeskateActivityOverlayHUD`, group `onlinefreeskateactivityoverlay`), the sign-up
//! (`FrontEndState_FreeskateActivitySignUp`, FE state 67) that proposes or accepts it, and the
//! profile's completed activities (`OnlineBackEnd::OnChallengeCompleted` 0x8257d3b8). The server
//! does what the original's game host peer did with `Message_GameProposal`: it holds the
//! proposal, its time and who accepted, and starts the activity once every member it was
//! proposed to accepted, for every member then in the game's world.

use super::{SelectedLiveError, SelectedLiveSession, error, trace};
use crate::challenge::hud::CentralMessage;
use crate::edit_skater_profile::{EditSkaterProfileStore, HandedSave};
use crate::online::activities::{self, Activity, Completed, OverlayView, SignUpView};
use crate::online::event_log::EventText;
use crate::ui::FeSoundRequest;
use crate::ui::online_hub::{HubActivity, Popup};
use crate::ui::pause_menu::PauseMenuInput;
use skate2_formats::skater_profile::ProfileData;
use skate2_net::protocol::{ClientMessage, proposal};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `eFinishReason`'s `FINISH_REASON_NORMAL_COMPLETION`.
pub(super) const NORMAL_COMPLETION: u32 = 1;

/// What FE state 67 shows of an activity (`SetActivityTypeAndProposerName` 0x8268ed70).
#[derive(Clone, Debug)]
struct SignUpData {
    activity: u64,
    /// `mActivityTypeTitle`: the activity's `Title`.
    title: String,
    /// `mGamerTag`: the proposer's name, `ID_COMMON_UNKNOWN` while proposing.
    gamer_tag: String,
    description: String,
    /// `mCurrentFreeskateType`.
    kind: i32,
    /// `mImageName`: the location's `FELargeImage`.
    image: String,
}

/// `FrontEndState_OnlineHubMenu::sProposal` and its overlay's data
/// (`FreeskateActivityOverlayHUDData`).
struct Proposal {
    activity: u64,
    /// The server's timeout as this console's clock has it.
    deadline: Instant,
    /// `mActivityType` (`GetActivityTypeHAL`) and `mGamerTagName`.
    type_title: &'static str,
    proposer_name: String,
    /// `mCurrentMode`: accepting until this console accepts; the proposer's waits from the start.
    accepting: bool,
    /// `mHasAcceptedProposal`: the sign-up is up from the overlay, or this console accepted.
    opened: bool,
    /// The `onlinefreeskateactivityoverlay` group is up: `Update` deletes it once the time runs
    /// out (`CleanupOnlineFreeskateActivityOverlay`).
    overlay: bool,
    /// FE state 67's data for accepting it.
    sign_up: SignUpData,
}

/// FE state 67 while it is up.
pub(super) struct SignUp {
    data: SignUpData,
    /// `mMode`: `eFreeskateActivityModes_Accepting`, else proposing from the hub.
    accepting: bool,
    /// `mTimeOut`, while accepting.
    deadline: Option<Instant>,
    /// Screen 108's intro completed: `StateManagerFrontEnd::ReceiveInput` passes input on.
    ready: bool,
    /// `eSk8FEPopup_Online_NeedMorePlayers_Default` (51) over it.
    popup: bool,
}

/// The activities of the game the session is in.
#[derive(Default)]
pub(super) struct Activities {
    proposal: Option<Proposal>,
    pub(super) sign_up: Option<SignUp>,
    /// `mCompletedFreeSkateActivities`, read from the profile as a game is joined.
    completed: Completed,
    /// `SetOnlineHudsVisible(false)`: the overlay hides the event log until the proposal ends.
    log_hidden: bool,
}

/// The whole seconds left until `deadline` (`GetServerTime`'s seconds counting down to the
/// timeout), none once it has passed.
fn seconds_left(deadline: Instant, now: Instant) -> u64 {
    let left = deadline.saturating_duration_since(now);
    left.as_secs() + u64::from(left.subsec_nanos() > 0)
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `GetOnlineFreeSkateActivities` of the game's location.
    fn online_location_activities(&self) -> Vec<Activity> {
        let (Some(game), Some((db, _))) = (self.online.game.as_ref(), self.freeskate.data.as_ref()) else { return Vec::new() };
        activities::activities(db, game.attributes.challenge_key)
    }

    /// The location's activities as the hub's menus list them, with the profile's completed
    /// ones (`IsFSACompleted` 0x82566978).
    pub(super) fn online_hub_activities(&self) -> Vec<HubActivity> {
        self.online_location_activities()
            .into_iter()
            .map(|activity| HubActivity { completed: self.online.activity.completed.contains(activity.key), key: activity.key, kind: activity.kind, title: activity.title })
            .collect()
    }

    /// Whether an activity is proposed (`sProposal`).
    pub(super) fn online_proposal_pending(&self) -> bool {
        self.online.activity.proposal.is_some()
    }

    /// Whether the activity proposed starts with the local player: this console proposed or
    /// accepted it (its proposal no longer accepting).
    pub(super) fn online_activity_awaits_local(&self) -> bool {
        self.online.activity.proposal.as_ref().is_some_and(|proposal| !proposal.accepting)
    }

    /// The activity of the location the round plays, by key.
    pub(super) fn online_round_activity(&self, key: u64) -> Result<Activity, SelectedLiveError> {
        self.online_location_activities()
            .into_iter()
            .find(|activity| activity.key == key)
            .ok_or_else(|| error(format!("activity {key:016x} is not one of the game's location")))
    }

    /// FE state 67's data of one of the location's activities.
    fn online_activity_data(&self, key: u64, gamer_tag: String) -> Option<SignUpData> {
        let game = self.online.game.as_ref()?;
        let (db, _) = self.freeskate.data.as_ref()?;
        let activity = activities::activities(db, game.attributes.challenge_key).into_iter().find(|activity| activity.key == key)?;
        Some(SignUpData {
            activity: key,
            title: activity.title,
            gamer_tag,
            description: activity.description,
            kind: activity.kind,
            image: activities::location_image(db, game.attributes.challenge_key),
        })
    }

    /// The profile's completed activities, as a game is joined (`OnlineBackEnd::LoadData`
    /// 0x8255d440's `completed_fsa`).
    pub(super) fn online_load_completed_activities(&mut self) {
        let leaf = EditSkaterProfileStore::default_path()
            .ok()
            .and_then(|path| EditSkaterProfileStore::open(path).ok())
            .and_then(|store| store.load_leaf(activities::SAVABLE, activities::COMPLETED).ok().flatten());
        self.online.activity.completed = leaf.map_or_else(Completed::default, |leaf| Completed::from_leaf(leaf.version, &leaf.tuples));
    }

    /// `GoToFreeskateActivityCIC` 0x82661a98: FE state 67 over the hub, proposing the activity,
    /// with no proposer or time.
    pub(super) fn online_open_activity_signup(&mut self, key: u64) {
        let Some(data) = self.online_activity_data(key, "ID_COMMON_UNKNOWN".to_owned()) else {
            trace(format_args!("activity {key:016x} is not one of the game's location"));
            return;
        };
        trace(format_args!("activity {key:016x} sign-up opens"));
        self.online.hub.menu = None;
        self.online.activity.sign_up = Some(SignUp { data, accepting: false, deadline: None, ready: false, popup: false });
    }

    /// `Message_GameProposal` from the server (`FrontEndState_OnlineHubMenu::OnMessage`
    /// 0x82675cb8): a member accepted, the activity is proposed, the proposal timed out, or the
    /// activity started without this console (`StateOnlineGame::OnMessage` 0x8276b060 puts the
    /// lobby HUD's variant line to `ID_ONLINE_OTHERSKATERS_INACTIVITY` as well).
    pub(super) fn online_proposal(&mut self, kind: u8, activity: u64, user_id: u64, timeout: u32) -> Result<(), SelectedLiveError> {
        trace(format_args!("proposal {kind} of {activity:016x} by {user_id:016x}, {timeout} ms"));
        let Some(game) = self.online.game.as_ref() else { return Ok(()) };
        let entered = game.entered;
        match kind {
            proposal::ACCEPT => {
                let name = game.name(user_id);
                if entered {
                    self.online.hub.central_messages.push(CentralMessage {
                        hal_id: "ID_ONLINE_EVENT_FREESKATE_ACCEPTED".to_owned(),
                        colour: 0,
                        args: [name, String::new(), String::new(), String::new()],
                        audio_hook: 0,
                    });
                }
            }
            proposal::BROADCAST => self.online_proposal_broadcast(activity, user_id, timeout)?,
            proposal::TIMEOUT => {
                self.online_reset_proposal()?;
                if entered {
                    self.online.event_log.add(EventText::plain("ID_EVENTLOG_FSA_PROPOSAL_FAILURE_1"));
                    self.online.event_log.add(EventText::plain("ID_EVENTLOG_FSA_PROPOSAL_FAILURE_2"));
                }
            }
            proposal::MISSED => {
                self.online_reset_proposal()?;
                if entered {
                    self.online.event_log.add(EventText::plain("ID_ONLINE_OTHERSKATERS_INACTIVITY"));
                }
                self.online.lobby_hud_dirty = true;
            }
            _ => {}
        }
        Ok(())
    }

    /// The proposal itself: `InitializeOnlineFreeskateActivityOverlay` and the overlay's data;
    /// to the proposer the waiting overlay, to everyone else `OnlineFeManager::Exit`, the
    /// proposer's name as a central message, the accepting overlay and FE state 67's data.
    fn online_proposal_broadcast(&mut self, activity: u64, proposer: u64, timeout: u32) -> Result<(), SelectedLiveError> {
        let Some(game) = self.online.game.as_ref() else { return Ok(()) };
        let local = self.online.user_id == Some(proposer);
        let known = game.players.iter().any(|player| player.user_id == proposer);
        let proposer_name = if local {
            self.online.nickname.clone()
        } else if known {
            game.name(proposer)
        } else {
            "ID_COMMON_UNKNOWN".to_owned()
        };
        let entered = game.entered;
        let Some(sign_up) = self.online_activity_data(activity, proposer_name.clone()) else {
            trace(format_args!("proposed activity {activity:016x} is not one of the game's location"));
            return Ok(());
        };
        if !local {
            self.close_online_hub()?;
            if entered {
                self.online.hub.central_messages.push(CentralMessage {
                    hal_id: "ID_ONLINE_EVENT_FREESKATE_PROPOSED".to_owned(),
                    colour: 0,
                    args: [proposer_name.clone(), String::new(), String::new(), String::new()],
                    audio_hook: 0,
                });
            }
        }
        self.online.activity.proposal = Some(Proposal {
            activity,
            deadline: Instant::now() + Duration::from_millis(u64::from(timeout)),
            type_title: activities::type_title(sign_up.kind),
            proposer_name,
            accepting: !local,
            opened: false,
            overlay: true,
            sign_up,
        });
        self.online.activity.log_hidden = true;
        Ok(())
    }

    /// `ResetFSAProposal(true)` 0x826721c0: the overlay goes, the event log shows again, the hub
    /// pops with what is over it (`PopState(44)`), and `sProposal` goes. An accepting FE state 67,
    /// up on its own, goes with the proposal it would accept (PC port: the original left it to
    /// time out).
    pub(super) fn online_reset_proposal(&mut self) -> Result<(), SelectedLiveError> {
        let had = self.online.activity.proposal.take().is_some();
        self.online.activity.log_hidden = false;
        let accepting = self.online.activity.sign_up.as_ref().is_some_and(|sign_up| sign_up.accepting);
        if accepting {
            self.online.activity.sign_up = None;
        }
        if had && self.online_hub_open() {
            self.close_online_hub()?;
        }
        Ok(())
    }

    /// `ResetOverlayValues` 0x826a55c0: the overlay goes and the event log shows again, the
    /// proposal staying until the server ends it.
    fn online_reset_overlay(&mut self) {
        if let Some(proposal) = self.online.activity.proposal.as_mut() {
            proposal.overlay = false;
        }
        self.online.activity.log_hidden = false;
    }

    /// FE state 67 closing with the online front end (`OnlineFeManager::Exit`): an accepting one
    /// leaves the overlay to open it again while the proposal stands (PC port: the original's
    /// overlay stayed hidden).
    pub(super) fn online_close_activity_signup(&mut self) {
        let Some(sign_up) = self.online.activity.sign_up.take() else { return };
        if sign_up.accepting
            && let Some(proposal) = self.online.activity.proposal.as_mut()
            && proposal.accepting
            && proposal.activity == sign_up.data.activity
        {
            proposal.opened = false;
        }
    }

    /// The overlay's and FE state 67's updates: `FreeskateActivityOverlayHUD::Update`
    /// (0x826bef58) deletes the overlay once the time runs out, and FE state 67's `OnUpdate`
    /// (0x826a7b10) closes an accepting sign-up then (`ResetOverlayValues`,
    /// `CloseOnlineHubMenu`).
    pub(super) fn update_online_activities(&mut self) {
        let now = Instant::now();
        if let Some(proposal) = self.online.activity.proposal.as_mut()
            && proposal.overlay
            && now >= proposal.deadline
        {
            proposal.overlay = false;
        }
        let expired = self.online.activity.sign_up.as_ref().is_some_and(|sign_up| sign_up.accepting && sign_up.deadline.is_some_and(|deadline| now >= deadline));
        if expired {
            self.online_reset_overlay();
            self.online.activity.sign_up = None;
        }
    }

    /// FE state 67's `ReceiveInput` (0x826a5648), after popup 51's OK:
    /// - proposing, A runs `GoToStartFreeskateActivity`; B returns to the hub;
    /// - accepting, A sends the acceptance and, in time, the overlay waits for the others; B in
    ///   time shows the accepting overlay again; past the time either resets the overlay.
    pub(super) fn update_online_activity_signup(&mut self, input: PauseMenuInput) -> Result<(), SelectedLiveError> {
        let Some(sign_up) = self.online.activity.sign_up.as_mut() else { return Ok(()) };
        if sign_up.popup {
            if input.select {
                sign_up.popup = false;
                self.online.hub.play(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
            }
            return Ok(());
        }
        if !sign_up.ready {
            return Ok(());
        }
        let expired = sign_up.deadline.is_some_and(|deadline| Instant::now() >= deadline);
        let (accepting, activity) = (sign_up.accepting, sign_up.data.activity);
        if input.select {
            if !accepting {
                return self.online_start_activity(activity);
            }
            trace(format_args!("accepting activity {activity:016x}"));
            self.online.send(ClientMessage::GameProposal { kind: proposal::ACCEPT, activity });
            self.online.activity.sign_up = None;
            if expired {
                self.online_reset_overlay();
            } else if let Some(proposal) = self.online.activity.proposal.as_mut() {
                proposal.accepting = false;
                self.online.activity.log_hidden = true;
            }
            return Ok(());
        }
        if input.cancel {
            if !accepting {
                // ReturnToPreviousScreen: back to the hub, whose OnEnter starts at its root.
                self.online.activity.sign_up = None;
                if let Some(lobby) = self.online_hub_lobby() {
                    self.reopen_online_hub(lobby);
                }
                return Ok(());
            }
            self.online.activity.sign_up = None;
            if expired {
                self.online_reset_overlay();
            } else if let Some(proposal) = self.online.activity.proposal.as_mut() {
                proposal.opened = false;
                self.online.activity.log_hidden = true;
            }
        }
        Ok(())
    }

    /// `GoToStartFreeskateActivity` 0x8266c728: with another participant, and no proposal or
    /// game in progress, the proposal goes to the server, the online front end exits and
    /// `ID_CHALLENGE_FREESKATEACTIVITY_ACTIVITYPROPOSED` shows; alone, popup 51 opens.
    fn online_start_activity(&mut self, activity: u64) -> Result<(), SelectedLiveError> {
        let Some(game) = self.online.game.as_ref() else { return Ok(()) };
        if game.players.len() < 2 {
            self.online.hub.play(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
            if let Some(sign_up) = self.online.activity.sign_up.as_mut() {
                sign_up.popup = true;
            }
            return Ok(());
        }
        let in_progress = self.online.round.is_some() || game.phase != skate2_net::protocol::Phase::LobbySkate;
        if self.online.activity.proposal.is_some() || in_progress {
            return Ok(());
        }
        trace(format_args!("proposing activity {activity:016x}"));
        self.online.send(ClientMessage::GameProposal { kind: proposal::PROPOSE, activity });
        self.close_online_hub()?;
        self.online.hub.central_messages.push(CentralMessage {
            hal_id: "ID_CHALLENGE_FREESKATEACTIVITY_ACTIVITYPROPOSED".to_owned(),
            colour: 0,
            args: Default::default(),
            audio_hook: 0,
        });
        Ok(())
    }

    /// `FreeskateActivityOverlayHUD::UpdateInput` 0x826bf050: D-pad up on the accepting overlay
    /// opens FE state 67 to accept the proposal (`OnlineFeManager::Swap(67)`), the overlay hiding
    /// until it closes. The HUD input left over.
    pub(in super::super) fn online_activity_input(&mut self, mut dpad: [bool; 5]) -> [bool; 5] {
        if !dpad[0] || self.online_activity_overlay().is_none() {
            return dpad;
        }
        let Some(proposal) = self.online.activity.proposal.as_mut() else { return dpad };
        if proposal.opened || !proposal.accepting {
            return dpad;
        }
        proposal.opened = true;
        trace(format_args!("activity {:016x} sign-up opens to accept", proposal.activity));
        self.online.activity.sign_up = Some(SignUp { data: proposal.sign_up.clone(), accepting: true, deadline: Some(proposal.deadline), ready: false, popup: false });
        dpad[0] = false;
        dpad
    }

    /// The overlay while it shows: in the game's world with no round, front end or pause menu
    /// over it (`sSystemBusyForFSAProposal`), hidden while FE state 67 accepts from it.
    #[must_use]
    pub fn online_activity_overlay(&self) -> Option<OverlayView> {
        let proposal = self.online.activity.proposal.as_ref()?;
        let entered = self.online.game.as_ref().is_some_and(|game| game.entered);
        if !entered || !proposal.overlay || (proposal.opened && proposal.accepting) || self.online.round.is_some() {
            return None;
        }
        if self.online_hub_open() || self.pause_menu.snapshot().visible || self.map_travel.is_some() {
            return None;
        }
        Some(OverlayView {
            type_title: proposal.type_title,
            proposer: proposal.proposer_name.clone(),
            seconds: seconds_left(proposal.deadline, Instant::now()),
            accepting: proposal.accepting,
        })
    }

    /// FE state 67's screen while it is up.
    #[must_use]
    pub fn online_activity_signup(&self) -> Option<SignUpView> {
        let sign_up = self.online.activity.sign_up.as_ref()?;
        let data = &sign_up.data;
        Some(SignUpView {
            title: data.title.clone(),
            description: data.description.clone(),
            gamer_tag: data.gamer_tag.clone(),
            type_description: activities::type_description(data.kind),
            image: data.image.clone(),
            accepting: sign_up.accepting,
            seconds: sign_up.deadline.map_or(0, |deadline| seconds_left(deadline, Instant::now())),
        })
    }

    /// Popup 51 over FE state 67.
    pub(super) fn online_activity_popup(&self) -> Option<Popup> {
        self.online.activity.sign_up.as_ref().filter(|sign_up| sign_up.popup).map(|_| Popup::Notice("ID_ONLINE_NEED_MORE_PLAYERS"))
    }

    /// Screen 108's intro completed.
    pub fn online_activity_signup_ready(&mut self) {
        if let Some(sign_up) = self.online.activity.sign_up.as_mut() {
            sign_up.ready = true;
        }
    }

    /// Whether the overlay keeps the event log hidden (`SetOnlineHudsVisible(false)`).
    #[must_use]
    pub fn online_event_log_hidden(&self) -> bool {
        self.online.activity.log_hidden && self.online.activity.proposal.is_some()
    }

    /// `SKATE2_DIAGNOSTIC_ONLINE_ACTIVITY=<ticks>[:<index>]`: after that many lobby ticks with
    /// every other skater showing, the game's first player proposes the location's activity of that
    /// index (the first without one), and every console accepts another's proposal as it arrives.
    pub(super) fn online_diagnostic_activity(&mut self) {
        let Ok(spec) = std::env::var("SKATE2_DIAGNOSTIC_ONLINE_ACTIVITY") else { return };
        let mut parts = spec.split(':');
        let Some(ticks) = parts.next().and_then(|ticks| ticks.parse::<u32>().ok()) else { return };
        let index = parts.next().and_then(|index| index.parse::<usize>().ok()).unwrap_or(0);
        if let Some(proposal) = self.online.activity.proposal.as_mut()
            && proposal.accepting
        {
            let activity = proposal.activity;
            proposal.accepting = false;
            trace(format_args!("diagnostic accepts activity {activity:016x}"));
            self.online.send(ClientMessage::GameProposal { kind: proposal::ACCEPT, activity });
            return;
        }
        let Some(game) = self.online.game.as_ref() else { return };
        if self.online.user_id.is_none_or(|local| game.host() != Some(local))
            || game.players.len() < 2
            || !game.remotes.values().all(super::remote_skater::RemoteSkater::visible)
            || self.online.round.is_some()
        {
            return;
        }
        self.online.diagnostic_lobby_ticks += 1;
        if self.online.diagnostic_lobby_ticks != ticks {
            return;
        }
        let activities = self.online_location_activities();
        let Some(activity) = activities.get(index).or_else(|| activities.first()) else {
            trace(format_args!("diagnostic: the location has no activity"));
            return;
        };
        trace(format_args!("diagnostic proposes activity {} {:016x} (type {})", activity.title, activity.key, activity.kind));
        let key = activity.key;
        if let Err(e) = self.online_start_activity(key) {
            trace(format_args!("diagnostic proposal failed: {e:?}"));
        }
    }

    /// The game ended or another one starts: no proposal, sign-up or overlay remains.
    pub(super) fn online_clear_activities(&mut self) {
        self.online.activity.proposal = None;
        self.online.activity.sign_up = None;
        self.online.activity.log_hidden = false;
    }

    /// `ReportLocalPlayerResults` 0x8274ce30: a freeskate activity finished normally
    /// (`FINISH_REASON_NORMAL_COMPLETION`) joins the profile's completed ones
    /// (`OnChallengeCompleted`, unless already there), saved with the `online` savable.
    pub(super) fn online_activity_completed(&mut self, key: u64) -> Result<(), SelectedLiveError> {
        if !self.online.activity.completed.add(key) {
            return Ok(());
        }
        trace(format_args!("activity {key:016x} completed"));
        let leaves = self.online.activity.completed.leaves();
        let path = EditSkaterProfileStore::default_path().map_err(error)?;
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
        let handed = HandedSave::hand_over();
        self.career_saves.start(move || {
            let saved = handed.write(path, |store| -> Result<(), String> {
                let data = match store.load_profile_data().map_err(|e| e.to_string())? {
                    Some(data) => data,
                    // ProfileBackEnd::ResetProfile 0x8256d8f0.
                    None => ProfileData::reset(
                        &super::super::career::localized(&mounts, "ID_DEFAULT_PLAYER_NAME").map_err(|e| e.to_string())?,
                        &super::super::career::localized(&mounts, "ID_DEFAULT_PLAYER_NICKNAME").map_err(|e| e.to_string())?,
                    ),
                };
                store.save_memcard(&data, None, &[(activities::SAVABLE, &activities::SAVABLES_BEFORE, &leaves)]).map_err(|e| e.to_string())
            });
            match saved.map_err(|e| e.to_string()).and_then(|saved| saved) {
                Ok(()) => eprintln!("online: completed freeskate activity saved"),
                Err(e) => eprintln!("online: completed freeskate activity not saved: {e}"),
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::seconds_left;
    use std::time::{Duration, Instant};

    #[test]
    fn the_time_left_counts_whole_seconds_down_to_none() {
        let now = Instant::now();
        assert_eq!(seconds_left(now + Duration::from_millis(15_000), now), 15);
        assert_eq!(seconds_left(now + Duration::from_millis(14_200), now), 15);
        assert_eq!(seconds_left(now + Duration::from_millis(1), now), 1);
        assert_eq!(seconds_left(now, now), 0);
        assert_eq!(seconds_left(now, now + Duration::from_secs(3)), 0);
    }
}
