//! The start hub (`FrontEndState_StartHub`, FE state 7) on the career's challenge system: the main
//! challenge its hub variants read, what the in-challenge screen shows, Quit Challenge's
//! `cMsgAbandonChallenge`, and Quit Game (`QuitPopupCallback`, `DoQuit`, `FadeToExit`,
//! `FrontEndManager::ExitToMainMenu`).

use super::{CareerSession, SelectedLiveError, SelectedLiveSession, error, localized};
use crate::{
    challenge::{dynamic_hull, keys::hash_value32, start_hub, unlockables},
    edit_skater_profile::{AudioProfile, EditSkaterProfileStore, HandedSave},
    skeleton_root_transforms::SkeletonRootMatrix,
    ui::pause_menu::{MainChallenge, StartHub},
};
use skate2_formats::skater_profile::ProfileData;
use std::sync::Arc;

/// `cLocationManager::GetSpawnLocation(hash, index)` (`0x82a8dce8`) for a location it does not
/// have (`0x82a8dd34`): the identity axes over (0, 0.5, -5 (index + 1)).
fn unknown_spawn_location(index: i32) -> [f32; 16] {
    #[expect(clippy::cast_precision_loss, reason = "the native (float) conversion")]
    let z = (index + 1) as f32 * -5.0;
    [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, z, 0.0]
}

/// A transform's four rows as `ProgressionOrientation` keeps them.
const fn rows(m: [f32; 16]) -> [[f32; 3]; 4] {
    [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]], [m[12], m[13], m[14]]]
}

fn trace(message: std::fmt::Arguments<'_>) {
    if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
        eprintln!("career: {message}");
    }
}

impl CareerSession {
    /// `cActivityManager::GetMainChallenge` as the start hub reads it.
    pub(crate) fn start_hub_main_challenge(&self) -> Option<MainChallenge> {
        start_hub::main_challenge(self.system.world())
    }

    /// `cLocationManager::GetSpawnLocation(GetHashValue32(name), index)` (`0x82a8d200`).
    fn spawn_point(&self, name: &str, index: i32) -> Result<[f32; 16], SelectedLiveError> {
        let locations = self.locations.lock().map_err(|_| error("location manager lock poisoned"))?;
        Ok(locations.spawn_location(hash_value32(name), index).unwrap_or_else(|| unknown_spawn_location(index)))
    }

    /// A location of `world` for the local player before the world is loaded: the location
    /// manager's spawn point, else the world's own location (its `_Sim.loc`).
    pub(crate) fn world_spawn_point(
        &self,
        mounts: &skate2_formats::mount::MountSet,
        world: &str,
        name: &str,
    ) -> Result<[f32; 16], SelectedLiveError> {
        let known = self.locations.lock().map_err(|_| error("location manager lock poisoned"))?.is_valid(hash_value32(name));
        if known {
            return self.spawn_point(name, crate::challenge::players::OFFLINE_LOCAL_PLAYER_ID);
        }
        super::super::world_travel::world_location(mounts, world, name)
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The start hub's runtime for a frame it is up: Quit Game's and Quit Challenge's yes, and the
    /// quit's profile save, whose end (`OnComplete` 0x82671960, `OnFail` 0x8266b268, with the hub
    /// in mode 0 or 1) calls `DoQuit`.
    pub(crate) fn update_start_hub(&mut self, hub: StartHub) -> Result<(), SelectedLiveError> {
        if self.pause_menu.take_quit_confirmed() {
            self.start_hub_quit(hub)?;
        }
        if self.pause_menu.take_abandon_confirmed() {
            self.start_hub_abandon_challenge();
        }
        if self.pause_menu.saving() && !self.career_saves.writing.as_ref().is_some_and(|writing| !writing.is_finished()) {
            trace(format_args!("start hub quit's save is written"));
            self.pause_menu.save_finished();
            self.request_main_menu()?;
        }
        Ok(())
    }

    /// `QuitChallengeCallback` (`0x8265e868`)'s yes: `cMsgAbandonChallenge` with `IsQuitting` false
    /// on the challenge broadcaster as the hub pops; the next update's flush delivers it.
    fn start_hub_abandon_challenge(&mut self) {
        if let Some(career) = self.career.as_mut() {
            trace(format_args!("start hub abandons the main challenge"));
            start_hub::abandon(career.system.world_mut(), false);
        }
    }

    /// `QuitPopupCallback` (`0x82671678`)'s yes, offline. Without a main challenge, in the
    /// single-player or offline free-skate hub (`mCurrentStartHubMode` 0 or 1) on a saved profile,
    /// the hub's input is blocked, its screen hidden (`ShowFEScreen(false)`) and the profile saved
    /// (`SaveLoadManager::RequestSave(eBeMemcardSave_Profile)`); [`Self::update_start_hub`] quits
    /// once it is written. Otherwise, with a main challenge, its end point's spawn location
    /// (`GetChallengeEndPoint`, `GetSpawnLocation(name, GetLocalPlayerID())`) becomes the
    /// progression orientation (`QuittingOverride`), and `DoQuit` (`0x8266b1a0`) fades the
    /// background in, after which `FadeToExit` runs ([`Self::start_hub_fade_to_exit`]).
    fn start_hub_quit(&mut self, hub: StartHub) -> Result<(), SelectedLiveError> {
        let main = self.career.as_ref().is_some_and(|career| career.system.world().activities.main.is_some());
        if !main && matches!(hub, StartHub::SinglePlayer | StartHub::OfflineFreeSkate) {
            trace(format_args!("start hub quit saves the profile"));
            self.pause_menu.hide_for_save();
            return if self.career.is_some() { self.save_career() } else { self.save_free_skate_profile() };
        }
        if let Some(career) = self.career.as_mut()
            && let Some(name) = start_hub::challenge_end_point(career.system.world())
        {
            //82671888: the human's offline network ID, not the spectator camera.
            let transform = career.spawn_point(&name, crate::challenge::players::OFFLINE_LOCAL_PLAYER_ID)?;
            trace(format_args!("start hub quit overrides the orientation with {name} at {:?}", rows(transform)[3]));
            start_hub::quitting_override(career.system.world_mut(), rows(transform));
        }
        self.pause_menu.do_quit();
        self.request_main_menu()
    }

    /// `SaveLoadManager::RequestSave(eBeMemcardSave_Profile)` from the main menu's free skate: the
    /// memcard save with `SKATER_PROFILE`'s profile data and settings and the savables the
    /// free-skate game holds, `UNLOCKABLES` and `Sk8ChallengeSystem`'s dynamic hull states, as
    /// the profile gave them. `SignPost` keeps what the profile has: the port's free-skate game
    /// does not load the progression the native's carries unchanged.
    fn save_free_skate_profile(&mut self) -> Result<(), SelectedLiveError> {
        let system = self.freeskate.hulls.as_ref().ok_or_else(|| error("free skate game absent"))?;
        let unlocks = system.world().unlockables.save();
        let hulls = system.world().dynamic_hulls.save();
        let path = EditSkaterProfileStore::default_path().map_err(error)?;
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("free skate assets absent"))?.mounts);
        let settings = self.audio.settings;
        let music = self.audio.manager.as_ref().map(|manager| (manager.profile.clone(), manager.catalogue().to_vec()));
        let handed = HandedSave::hand_over();
        self.career_saves.start(move || {
            let saved = handed.write(path, |store| -> Result<(), String> {
                let data = match store.load_profile_data().map_err(|e| e.to_string())? {
                    Some(data) => data,
                    // ProfileBackEnd::ResetProfile 0x8256d8f0.
                    None => ProfileData::reset(
                        &localized(&mounts, "ID_DEFAULT_PLAYER_NAME").map_err(|e| e.to_string())?,
                        &localized(&mounts, "ID_DEFAULT_PLAYER_NICKNAME").map_err(|e| e.to_string())?,
                    ),
                };
                let audio = AudioProfile {
                    settings: &settings,
                    music_player: music.as_ref().map(|(player, catalogue)| (player, catalogue.as_slice())),
                };
                store
                    .save_memcard(
                        &data,
                        Some(&audio),
                        &[(unlockables::SAVABLE, &[], &unlocks), (dynamic_hull::SAVABLE, &dynamic_hull::SAVABLES_BEFORE, &hulls)],
                    )
                    .map_err(|e| e.to_string())
            });
            match saved.map_err(|e| e.to_string()).and_then(|saved| saved) {
                Ok(()) => eprintln!("free skate: profile saved"),
                Err(e) => eprintln!("free skate: profile save failed: {e}"),
            }
        });
        Ok(())
    }

    /// `FadeToExit` (`0x8265e7f0`) once the quit's fade is over: the hub popped, then
    /// `FrontEndManager::ExitToMainMenu` (`0x82628990`): with a main challenge
    /// `cMsgAbandonChallenge` with `IsQuitting` true, `KillNIS` while a NIS runs, and
    /// `QuittingToFE`, before `UpdateExitToMainMenu` (`0x82628bc0`) enters the front end. The
    /// simulation's post-update flush delivers the message ahead of the unload.
    pub(crate) fn start_hub_fade_to_exit(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let sent = start_hub::exit_to_main_menu(career.system.world_mut());
        if career.nis.is_in_nis() {
            career.nis.kill_nis();
        }
        trace(format_args!("start hub exits to the main menu (abandon sent: {sent})"));
        if sent {
            self.try_update_career_paused()?;
        }
        Ok(())
    }

    /// `cChallengeSignUpManager::Update`'s `eSignUpState_StartChallenge` (`0x82db44a0`) for a
    /// teleport start (a throwdown or Pick-a-Pro from the phone list):
    /// `TeleportLocalPlayerStartChallenge` (`0x828a9678`) moves the local player to the
    /// challenge's location behind the load (the port's map travel) on the board, and the load's
    /// `StartChallengeState` starts it. Without a location the challenge starts in place.
    pub(super) fn teleport_local_player_start_challenge(&mut self, challenge: u64) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let Some(transform) = career.prepare_start_challenge_travel(challenge) else {
            trace(format_args!("teleport start {challenge:#018x} without a location: started in place"));
            career.system.start_main_challenge(challenge);
            return Ok(());
        };
        // TeleportLocalToWorldLocation into the challenge's world location.
        let world_location = crate::challenge::stuff_menu::challenge_world_location(career.system.world(), challenge);
        let world_file = world_location.and_then(|location| crate::challenge::stuff_menu::location_world_file(career.system.world(), location));
        trace(format_args!("teleport start {challenge:#018x} at {:?} in {world_file:?}", rows(transform)[3]));
        let world = self.world_operation(world_location.unwrap_or(u64::MAX), world_file.as_deref()).unwrap_or_default();
        self.start_world_travel(world, transform, true);
        Ok(())
    }

    /// `Teleport.ToWorldFileLocation`'s `cTeleportManager::TeleportLocalToWorldLocation`
    /// (`0x828a91f8`): the location's spawn point for the local player off the board. Into another
    /// world than the one held (`RequiresAdditionalWorldOperation`), the flow's teleport behind the
    /// load screen puts the worlds in place first; otherwise it is the ordinary teleport.
    pub(super) fn teleport_local_to_world_location(&mut self, world_location: u64, world_file: Option<&str>, location: &str) -> Result<(), SelectedLiveError> {
        let Some(world) = self.world_operation(world_location, world_file) else {
            return self.teleport_to_challenge_end_point(location);
        };
        if !self.additional_world_loadable(&world) {
            eprintln!("career: teleport to {location} in the additional world location {world_file:?}, which the port cannot load");
            return Ok(());
        }
        if self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()).is_none() {
            return Ok(());
        }
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("career assets absent"))?.mounts);
        let career = self.career.as_ref().ok_or_else(|| error("career absent"))?;
        let file = world_file.unwrap_or(crate::game::DEBUG_REFERENCE_WORLD_FILE);
        let transform = career.world_spawn_point(&mounts, file, location)?;
        trace(format_args!("teleport to {location} in {file} at {:?}, behind the load screen", rows(transform)[3]));
        self.start_world_travel(world, transform, false);
        Ok(())
    }

    /// `TeleportLocalPlayerToChallengeEndPoint`'s `cTeleportManager::TeleportLocalPlayer`
    /// (`0x828a92a8`) with the local skater: the location's spawn point for the local player, then
    /// `TeleportLocalPlayer` (`0x828a9f40`) off the board without the large load radius. Without a
    /// load screen (`TeleportRequiresLoadScreen` `0x828a9420`) `TeleportPlayer` (`0x828a9888`)
    /// sends `cMsgTeleport`; with one, the flow's teleport behind the load screen (the port's map
    /// travel) and `cMsgClearSessionMarker`.
    pub(super) fn teleport_to_challenge_end_point(&mut self, location: &str) -> Result<(), SelectedLiveError> {
        if self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()).is_none() {
            return Ok(());
        }
        let career = self.career.as_ref().ok_or_else(|| error("career absent"))?;
        let transform = career.spawn_point(location, crate::challenge::players::OFFLINE_LOCAL_PLAYER_ID)?;
        let load_screen = self.teleport_requires_load_screen([transform[12], transform[13], transform[14]])?;
        trace(format_args!(
            "challenge end point {location} at {:?}{}",
            rows(transform)[3],
            if load_screen { ", behind the load screen" } else { "" }
        ));
        if load_screen {
            self.frontend_fade.cover();
            self.map_travel = Some(transform);
            self.map_travel_presentation_ready = false;
            self.map_travel_dispatched = false;
            self.map_travel_on_board = false;
        } else {
            self.player_ui_queues()?.send_teleport(SkeletonRootMatrix::known(transform), false);
        }
        Ok(())
    }
}
