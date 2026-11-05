//! Explicit host selection, without any second player controller or solver.
use super::SelectedPlayerTick;
use crate::{
    physical_player_assembly::PhysicalAssemblyWorld,
    static_ground_pool::StaticGroundPool,
    ui::gameplay::{Difficulty, GameplaySettings},
};

impl SelectedPlayerTick {
    pub(crate) fn configure_skate3(
        &mut self,
        settings: GameplaySettings,
        raw: Option<crate::input_pad::XboxPadSample>,
    ) {
        self.mod_settings = settings;
        self.raw_controller = raw;
    }
    pub(crate) fn configure_skate3_focus(&mut self, focused: bool) {
        self.skate3_has_focus = focused;
        if !focused { self.raw_controller = Some(crate::input_pad::XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 }); }
    }
    pub(crate) fn allow_skate3_start(&mut self, allowed: bool) { self.allow_skate3_start = allowed; }
    /// The profile's camera settings (`try_set_camera_profile_settings`'s): angle and look inversion.
    pub(crate) fn configure_skate3_camera(&mut self, camera_type: i32, invert_look: [bool; 2]) {
        self.camera_type = camera_type;
        self.camera_invert_look = invert_look;
    }
    pub(crate) fn skate3_output(&self) -> Option<std::sync::Arc<crate::skate3_native::FrameOutput>> {
        let owner = self.mode_owner.as_ref()?;
        let mode = owner.skate3.lock().ok()?;
        mode.as_ref().map(|player| player.output_frame())
    }
    #[cfg(test)]
    pub(crate) fn skate3_ground_height(&self,point:[f32;3])->Result<f32,String> {
        let owner=self.mode_owner.as_ref().ok_or("ground fixture source owner")?;
        let player=owner.skate3.lock().map_err(|_|"ground fixture source lock")?;
        player.as_ref().ok_or("ground fixture source player")?.test_ground_height(point)
    }
    #[cfg(test)]
    pub(crate) fn skate3_spawn_wheel_gaps(&self)->Result<[f32;4],String> {
        let owner=self.mode_owner.as_ref().ok_or("wheel audit no selected owner")?;
        let player=owner.skate3.lock().map_err(|_|"wheel audit source lock")?;
        player.as_ref().ok_or("wheel audit no source player")?.test_spawn_wheel_gaps()
    }
    #[cfg(test)]
    pub(crate) fn skate3_spawn_stance(&self) -> (u32, i32) {
        let owner=self.mode_owner.as_ref().unwrap();
        let player=owner.skate3.lock().unwrap();
        player.as_ref().unwrap().test_spawn_stance()
    }
    #[cfg(test)]
    pub(crate) fn skate3_boneless_trace(&self) -> String {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_boneless_trace())).expect("source boneless")
    }
    #[cfg(test)]
    pub(crate) fn skate3_bail_control_probe(&self)->([f32;2],Option<[f32;2]>) {
        let owner=self.mode_owner.as_ref().unwrap();let mode=owner.skate3.lock().unwrap();
        mode.as_ref().unwrap().test_bail_control_probe()
    }
    #[cfg(test)]
    pub(crate) fn skate3_plant_control_probe(&self)->([f32;2],[f32;4],Option<[f32;2]>) {
        let owner=self.mode_owner.as_ref().unwrap();let mode=owner.skate3.lock().unwrap();
        mode.as_ref().unwrap().test_plant_control_probe()
    }
    #[cfg(test)]
    pub(crate) fn skate3_footplant_trace(&self) -> String {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_footplant_trace())).expect("source footplant")
    }
    #[cfg(test)]
    pub(crate) fn skate3_grab_audit(&self,world:&PhysicalAssemblyWorld)->Result<(usize,usize,usize),String> {
        let owner=self.mode_owner.as_ref().ok_or("grab audit no selected owner")?;
        let mode=owner.skate3.lock().map_err(|_|"grab audit source lock")?;
        mode.as_ref().ok_or("grab audit no source player")?.test_grab_scene(world)
    }
    #[cfg(test)]
    pub(crate) fn skate3_living_world_impulse(&self) -> (bool,[u32;4]) {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_living_world_impulse())).expect("source impulse")
    }
    #[cfg(test)]
    pub(crate) fn skate3_actor_input(&self) -> (bool,bool,Vec<String>,[f32;2]) {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_actor_input())).expect("source actor input")
    }
    #[cfg(test)]
    pub(crate) fn skate3_offboard_motion_trace(&self)->String {
        let owner=self.mode_owner.as_ref().unwrap();let mode=owner.skate3.lock().unwrap();
        mode.as_ref().unwrap().test_offboard_motion_trace()
    }
    #[cfg(test)]
    pub(crate) fn skate3_input_trace(&self) -> String {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_input_trace())).unwrap_or_default()
    }
    #[cfg(test)]
    pub(crate) fn skate3_world_body_count(&self) -> usize {
        self.mode_owner.as_ref().and_then(|owner| owner.skate3.lock().ok())
            .and_then(|mode| mode.as_ref().map(|player| player.world_body_count())).unwrap_or(0)
    }
    #[cfg(test)]
    pub(crate) fn test_reconcile_skate3(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        skaters: &[super::SelectedTickSkater<'_>],
        selected: usize,
        settings: &crate::players_backend_settings::PlayersBackendSettings,
    ) -> Result<(),String> {
        self.try_reconcile_skate3(world,ground,skaters,selected,settings)
    }
    /// Under Skate 2 physics, whether Skate 3's gestures play over it: never. PC port (user
    /// choice): each physics mode plays its own game's saved gestures, Skate 2 physics the Skate 2
    /// set from the skater record. The Actor gets Skate 2's set, as Skate 2's own gestures stay off
    /// while it holds Skate 3's from Skate 3 physics.
    pub(super) fn try_skate2_physics_skate3_gestures(
        &mut self,
        actor: &crate::actor_construction::ActorConstruction,
    ) -> Result<bool, String> {
        actor.try_set_pc_gesture_preferences(Default::default())?;
        Ok(false)
    }
    pub(super) fn try_reconcile_skate3(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        skaters: &[super::SelectedTickSkater<'_>],
        selected: usize,
        settings: &crate::players_backend_settings::PlayersBackendSettings,
    ) -> Result<(), String> {
        // Skate 2 physics with no Skate 3 player to hand back: nothing to do.
        if !self.mod_settings.skate3 && self.mode_owner.is_none() {
            return Ok(());
        }
        let skater = skaters.get(selected).ok_or("Missing selected native player")?;
        let (actor, board, skeleton) = (skater.actor, skater.board, skater.skeleton);
        if self.gesture_preferences.is_none() || self.style_preferences.is_none() {
            let path=crate::edit_skater_profile::EditSkaterProfileStore::default_path().map_err(|e|e.to_string())?;
            let store=crate::edit_skater_profile::EditSkaterProfileStore::open(path).map_err(|e|e.to_string())?;
            if self.gesture_preferences.is_none() { self.gesture_preferences=Some(store.load_gesture_settings().map_err(|e|e.to_string())?); }
            if self.style_preferences.is_none() { self.style_preferences=Some(store.load_skate3_style().map_err(|e|e.to_string())?); }
        }
        // PC port (user choice): Skate 3 physics plays the saved Skate 3 gestures, whichever set the
        // Edit Skater rows last showed.
        actor.try_set_pc_gesture_preferences(skate2_formats::skater_profile::PcGestureSettings {
            selected: skate2_formats::skater_profile::GestureSet::Skate3,
            ..self.gesture_preferences.unwrap()
        })?;
        let physical = actor.cached_physical_allocation().map_err(|e| e.to_string())?;
        let owner = std::sync::Arc::clone(physical.owner());
        if let Some(previous) = self.mode_owner.as_ref().filter(|previous| !std::sync::Arc::ptr_eq(previous, &owner)) {
            // Party Play changes the local Actor without reordering the skater list.
            // Return the previous Actor to its own solver before selecting another.
            let mut previous_mode = previous.skate3.lock().map_err(|_| "previous native physical mode lock")?;
            if let Some(player) = previous_mode.take() {
                let previous_skater = skaters.iter().find(|candidate| {
                    candidate.actor.cached_physical_allocation().ok().is_some_and(|physical|
                        std::sync::Arc::ptr_eq(physical.owner(), previous))
                });
                if let Some(skater) = previous_skater {
                    player.return_to_skate2(world, skater.actor, previous, skater.board, skater.skeleton)?;
                    skater.actor.try_select_skate3_movement(false).map_err(|e|e.to_string())?;
                } else {
                    player.release(world)?;
                }
            }
            self.active_difficulty = None;
            crate::diagnostics::test_log::event("skate3_player_owner", format_args!("selected={selected} changed=true"));
        }
        self.mode_owner = Some(std::sync::Arc::clone(&owner));
        let mut mode = owner.skate3.lock().map_err(|_| "native physical mode lock")?;
        let (player_index, _, non_player_ai) = owner.actor_query.lock().map_err(|_| "native player query lock")?
            .as_ref().ok_or("native player query missing")?.try_scoring_identity().map_err(|error|error.to_string())?;
        // Frontend admission defers construction.
        if mode.is_none() && !self.allow_skate3_start {
            actor.try_select_skate3_movement(false).map_err(|e| e.to_string())?;
            return Ok(());
        }
        if !self.mod_settings.skate3 || non_player_ai {
            actor.try_select_skate3_movement(false).map_err(|e| e.to_string())?;
            if let Some(player) = mode.take() {
                player.return_to_skate2(world, actor, &owner, board, skeleton)?;
                self.active_difficulty = None;
            }
            if !self.mod_settings.skate3 {
                // Back to Skate 2 physics: its own gestures, and no Skate 3 owner.
                actor.try_set_pc_gesture_preferences(Default::default())?;
                drop(mode);
                self.mode_owner = None;
            }
            return Ok(());
        }
        // A Party Play location/character change can publish a new Actor
        // before its replacement world has any resident collision. The native
        // frontend preview remains its existing owner until admission; creating
        // a source solver against an empty scene used to terminate the game.
        if mode.is_none() && ground.published_volume_count() == 0 {
            actor.try_select_skate3_movement(false).map_err(|error|error.to_string())?;
            self.active_difficulty = None;
            if self.tick.is_multiple_of(60) {
                crate::diagnostics::test_log::event("skate3_scene_wait",format_args!(
                    "tick={} selected={selected} reason=no_resident_terrain phase=player_admission",self.tick));
            }
            return Ok(());
        }
        let difficulty = self.mod_settings.difficulty;
        let profile = match difficulty {
            Difficulty::Easy | Difficulty::EasyMotorized => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Skate2Default => "custom",
            Difficulty::Hardcore => "hardcore",
            Difficulty::Motorized => "motorized",
        };
        let entering = mode.is_none();
        if entering {
            let entry = {
                let output = owner.phys_out().lock().map_err(|_| "native entry PhysOut lock")?;
                if output.try_state_output().map_err(|error|format!("native entry state: {error:?}"))?.is_offboard {
                    let root = output.try_skeleton_output().map_err(|error|format!("native entry skeleton: {error:?}"))?.skeleton_root_matrix;
                    let mut transform = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
                    for (column, value) in root.columns().iter().enumerate() {
                        transform[column * 4..column * 4 + 3].copy_from_slice(&value.xyz);
                    }
                    let (_, velocity) = output.try_system_centre_and_damped_velocity().map_err(|error|format!("native entry velocity: {error:?}"))?;
                    let state = output.try_state_output().map_err(|error|format!("native entry state: {error:?}"))?.physics_state;
                    // S2's separate Locomotion504 is part of S3's BipedGround;
                    // S2's Jump503 reports the same air family as OffBoardAir502.
                    let state = match state { 501 => 502, 502 | 503 => 501, 505 => 503, 500 | 504 => 500,
                        other => return Err(format!("Unsupported offboard entry state {other}")) };
                    let holding_board = output.try_offboard_output().map_err(|error|format!("native entry board: {error:?}"))?.is_holding_skateboard;
                    let filtered = output.try_filtered_output().map_err(|error|format!("native entry filtered: {error:?}"))?;
                    if filtered.is_grinding { return Err("Offboard entry carries a conflicting active grind".into()); }
                    let category = |state| {
                        use crate::physics::FilteredPhysicsState as S2;
                        use skate3_core::physics::filtered_state::FilteredCategory as S3;
                        match state { S2::Invalid => S3::Invalid, S2::Ground => S3::Ground, S2::Air => S3::Air,
                            S2::Grind => S3::Grind, S2::Wipeout => S3::Wipeout, S2::Teleport => S3::Teleport,
                            S2::OffBoard => S3::Offboard, S2::OffBoardAir => S3::OffboardAir }
                    };
                    let filtered = skate3_core::physics::filtered_state::FilteredStateOutput {
                        category: category(filtered.filtered_physics_state),
                        previous_category: category(filtered.last_filtered_physics_state),
                        grinding: false, grind: Default::default(), last_grind_distance: filtered.last_grind_dist,
                    };
                    let camera = world.local_skater_identity.as_ref().ok_or("native entry local camera owner")?
                        .try_camera_producer_projection().map_err(|error|format!("native entry camera: {error:?}"))?;
                    let camera_basis = Some(std::array::from_fn(|column| camera.camera_matrix.columns()[column].xyz));
                    Some(crate::skate3_native::physics::player::PlayerEntry { transform, velocity: velocity.xyz,
                        on_board: false, state, holding_board, filtered: Some(filtered), camera_basis })
                } else { None }
            };
            *mode = Some(Box::new(crate::skate3_native::physics::player::Skate3Player::load(
                world, ground, board, skeleton, profile, entry,
            )?));
        }
        // Profile stance is not the marker's requested leading foot: a switch
        // marker changes the latter without changing the skater's customization.
        mode.as_mut().expect("loaded").configure_character_stance(
            settings.try_stance_byte(player_index).map_err(|error|error.to_string())? != 0,
            actor.try_last_set_foot_forward().map_err(|error|error.to_string())?, entering)?;
        mode.as_mut().expect("loaded").configure_animation_style(self.style_preferences.expect("loaded"))?;
        // Skate 3 tunings ride on the Skate 3 skater's trucks and wheels; Skate 2 tuning keeps the
        // Skate 2 profile's, which it was made for.
        let style = self.style_preferences.expect("loaded");
        mode.as_mut().expect("loaded").configure_equipment(
            (difficulty != Difficulty::Skate2Default).then(|| [style.truck_tightness(), style.wheel_hardness()]));
        if self.active_difficulty != Some(difficulty) {
            let player = mode.as_mut().expect("loaded");
            match difficulty {
                Difficulty::Skate2Default => player.set_skate2_profile(&owner.inputs().vault)?,
                Difficulty::EasyMotorized => {
                    player.set_motorized_profile(profile)?;
                }
                _ => player.set_profile(profile)?,
            }
            self.active_difficulty = Some(difficulty);
        }
        actor.try_select_skate3_movement(true).map_err(|e| e.to_string())?;
        let camera_type = match self.camera_type { 0 => 0, 1 => 1, other => return Err(format!("Invalid native camera angle {other}")) };
        mode.as_mut().expect("loaded").set_graph_gestures(actor.try_skate3_gesture_selections()?);
        mode.as_mut().expect("loaded").set_camera_type(camera_type);
        mode.as_mut().expect("loaded").set_invert_look(self.camera_invert_look);
        mode.as_mut().expect("loaded").set_input_focus(self.skate3_has_focus);
        mode.as_mut().expect("loaded").set_airtime_slow_motion(self.mod_settings.airtime_slow_motion);
        mode.as_mut().expect("loaded").sync_teleport(actor)?;
        if let Some(breadcrumbs) = actor.retained_breadcrumbs().map_err(|error| format!("Skate 3 breadcrumbs: {error:?}"))? {
            let crumbs = breadcrumbs.lock().map_err(|_| "Skate 3 breadcrumb lock")?;
            mode.as_mut().expect("loaded").sync_breadcrumbs(&crumbs)?;
        }
        Ok(())
    }
}
