//! Explicit PC mod observer publication. These are completed source-runtime
//! facts for Skate 2 world consumers, never a certificate of a Skate 2 Fill.
use super::{COMPONENTS, PhysOutOrigin, PhysOutValue, PhysicalPlayerPhysOut};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

fn vector(v: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector::known(v)
}
fn raw(v: [u32; 4]) -> SkeletonRootVector {
    vector(v.map(f32::from_bits))
}
fn value<T>(v: T) -> Option<PhysOutValue<T>> {
    Some(PhysOutValue {
        value: v,
        origin: PhysOutOrigin::Publication,
    })
}
fn matrix(v: [[f32; 4]; 4]) -> SkeletonRootMatrix {
    SkeletonRootMatrix::known(crate::skate3_bridge::player::flatten(v))
}

/// Semantic conversion: BipedAir501 and Pushing502 have different identifiers
/// in the two engines. Shared state IDs retain their original meanings.
pub(crate) fn state_id(state: u32) -> Result<i32, String> {
    Ok(match state {
        501 => 502,
        502 => 501,
        503 => 505,
        0 | 100..=105 | 200..=201 | 300 | 400..=404 | 500 | 600..=602 | 700..=702 => state as i32,
        // Skate 2 has no secondary-air or darkslide state. Consumers receive
        // their family state; simulation continues in the source engine.
        202 => 200,
        405 => 400,
        _ => return Err(format!("Unmapped Skate 3 state {state}")),
    })
}

/// Proves a completed source publication on this exact PhysOut occurrence.
/// It does not certify any Skate 2 solver or conditioner stage.
pub(crate) struct Skate3Publication { output: std::sync::Arc<()> }
impl Skate3Publication {
    pub(crate) fn matches(&self, output: &PhysicalPlayerPhysOut) -> bool {
        std::sync::Arc::ptr_eq(&self.output, output.occurrence())
    }
}

fn name36(name: Option<skate3_core::animation::output::attributes::AttributeName>) -> crate::play_animation::FastString36 {
    let mut words = [0; 6];
    if let Some(name) = name { words[..5].copy_from_slice(&name.0); }
    crate::play_animation::FastString36 { words }
}
fn direction(v: skate3_core::math::Vector3) -> SkeletonRootVector { vector([v.x, v.y, v.z, 0.]) }

impl PhysicalPlayerPhysOut {
    /// Same retained Actor payload read by Update5's native trick publication.
    pub(crate) fn try_publish_actor_gesture(&mut self,gesture:u32,gesturing:bool)->Result<(),String> {
        let trick=self.concrete.trick.as_mut().ok_or("source gesture trick output")?;
        trick.gesture=i32::from_ne_bytes(gesture.to_ne_bytes());
        trick.is_gesturing=gesturing;
        Ok(())
    }

    pub(crate) fn try_publish_skate3_observers(
        &mut self,
        frame: &crate::skate3_native::FrameOutput,
        dt: f32,
        teleport: Option<[f32; 16]>,
    ) -> Result<Skate3Publication, String> {
        for component in COMPONENTS {
            self.require_complete_component(component)
                .map_err(|e| format!("Skate 3 output: {e:?}"))?;
        }
        let guard = self
            .sample_write_guard()
            .map_err(|e| format!("Skate 3 output: {e:?}"))?;
        self.check_write_guard(&guard)
            .map_err(|e| format!("Skate 3 output: {e:?}"))?;
        // Update5 normally resets Scoring2's event arrays before the score
        // conditioner fills this tick. The source solver replaces Update5,
        // so retain that output lifecycle here; the score module keeps history.
        self.concrete.scoring.as_deref_mut().ok_or("Native source requires the original scoring output")?.reset();
        let state = state_id(frame.state)?;
        let root = matrix(frame.root_transform);
        let effective_frame = frame.effective_root_transform();
        let effective = matrix(effective_frame);
        let physical = &frame.physical;
        let filtered = match physical.filtered_state_0 {
            0 => crate::physics::FilteredPhysicsState::Invalid,
            1 => crate::physics::FilteredPhysicsState::Ground,
            2 => crate::physics::FilteredPhysicsState::Air,
            3 => crate::physics::FilteredPhysicsState::Grind,
            4 => crate::physics::FilteredPhysicsState::Wipeout,
            5 => crate::physics::FilteredPhysicsState::Teleport,
            6 => crate::physics::FilteredPhysicsState::OffBoard,
            7 => crate::physics::FilteredPhysicsState::OffBoardAir,
            other => return Err(format!("Unmapped Skate 3 filtered state {other}")),
        };
        self.motion_source_frame = None;
        self.motion_vectors = [
            physical.skateboard.vector_64,
            physical.skateboard.vector_80,
            physical.skateboard.vector_96,
        ]
        .map(|v| value(raw(v)));
        self.motion_scalars = [
            physical.skateboard.scalar_160,
            physical.skateboard.scalar_164,
            physical.skateboard.scalar_168,
            physical.skateboard.scalar_172,
        ]
        .map(value);
        self.system_vectors = [
            // TU3 AirCollector82DA81E4 projects velocity against reckoning
            // up. The visual skeleton's Y can differ at a vert takeoff.
            value(raw(physical.reckoning.vector_96)),
            value(vector(frame.root_transform[2])),
            value(vector(frame.root_transform[0])),
            value(vector(frame.centre_of_mass)),
            value(raw(physical.reckoning.vector_16)),
        ];
        self.ground_vectors = [
            physical.ground.vector_64,
            physical.ground.vector_80,
            physical.ground.vector_96,
        ]
        .map(|v| value(raw(v)));
        let skeleton = self.concrete.skeleton.as_deref_mut().expect("checked");
        skeleton.skeleton_root_matrix = root;
        skeleton.effective_skeleton_root_matrix = effective;
        let part = |name: &str| {
            frame
                .skeleton_parts
                .iter()
                .find(|part| part.bone_name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("Skate 3 observer physical part {name} absent"))
        };
        let hips = part("Hips")?;
        // Authored PHYS_TPOSE bindings: host part 1 = NECK1, parts 15/19
        // = toe-base volumes. The visual Head/Foot bones are different parts.
        let head = part("Neck1")?;
        let left = part("LeftToeBase")?;
        let right = part("RightToeBase")?;
        let velocity = |v: [f32; 3]| vector([v[0], v[1], v[2], 0.]);
        // Match the meanings of the host's SkeletonPartFill stores: physical
        // volume origins/axes and independently transformed bone positions.
        // Render bones can differ from solved parts during contacts/ragdoll.
        skeleton.effective_anim_z = vector(effective_frame[2]);
        skeleton.skeleton_hips_pos = vector(hips.body.transform[3]);
        skeleton.skeleton_hips_look = vector(hips.body.transform[0]);
        skeleton.skeleton_hips_mat_y = vector(hips.body.transform[1]);
        skeleton.skeleton_hips_mat_z = vector(hips.body.transform[2]);
        skeleton.skeleton_hips_vel = velocity(hips.body.velocity);
        skeleton.skeleton_torso_angular_vel = velocity(hips.body.omega);
        skeleton.skeleton_head_pos = vector(head.body.transform[3]);
        skeleton.skeleton_head_look = vector(head.body.transform[1]);
        skeleton.skeleton_foot_left_pos = vector(left.body.transform[3]);
        skeleton.skeleton_foot_right_pos = vector(right.body.transform[3]);
        skeleton.skeleton_foot_left_bone_pos = vector(left.bone_transform[3]);
        skeleton.skeleton_foot_right_bone_pos = vector(right.bone_transform[3]);
        skeleton.skeleton_foot_left_look = vector(left.body.transform[0]);
        skeleton.skeleton_foot_right_look = vector(right.body.transform[0]);
        skeleton.skeleton_foot_left_vel = velocity(left.body.velocity);
        skeleton.skeleton_foot_right_vel = velocity(right.body.velocity);
        skeleton.skeleton_left_foot_angular_vel = velocity(left.body.omega);
        skeleton.skeleton_right_foot_angular_vel = velocity(right.body.omega);
        skeleton.is_wipeout_over = physical.skeleton.over_599 != 0;
        skeleton.skate3_response_strength = physical.skeleton.response_strength_580;
        skeleton.is_wipeout_teleport_pending = physical.skeleton.teleport_pending_604 != 0;
        skeleton.time_until_wipeout_teleport = physical.skeleton.time_until_teleport_576;
        let out = self.concrete.state.as_deref_mut().expect("checked");
        out.update_count = frame.tick as i32;
        out.state_update_count = frame.state_frames as i32;
        out.frames_since_reset = frame.frames_since_reset;
        out.physics_state = state;
        out.physics_state_category = frame.category as i32;
        out.is_offboard = frame.offboard;
        out.is_offboard_ground = frame.state == 500;
        out.is_wiping_out = frame.wiping_out;
        // Native WipeoutGround fills this from its own reset timers, which do
        // not run here; left stale it read false for every source bail, and the
        // session marker refused any use until the ragdoll ended. The marker's
        // challenge, HOM and pending-reset checks still apply.
        out.can_leave_wipeout = true;
        out.is_teleport_finished = teleport.is_some();
        out.has_just_teleported = frame.teleported;
        out.is_reverting = frame.state == 102;
        out.interactable_object_id=frame.gameplay.interactable_object.0;
        // BipedGround descriptor kind/id is State36/40, consumed by the S2 HUD.
        out.grab_spline_desc.object_type=physical.state.counter_36 as i32;
        out.grab_spline_desc.object_id=physical.state.skitch_value_40 as i32;
        let motion = self.concrete.motion.as_deref_mut().expect("checked");
        motion.did_teleport = teleport.is_some();
        if let Some(target) = teleport {
            motion.teleport_transform = SkeletonRootMatrix::known(target);
        }
        let filtered_out = self.concrete.filtered.as_deref_mut().expect("checked");
        filtered_out.last_filtered_physics_state = filtered_out.filtered_physics_state;
        filtered_out.filtered_physics_state = filtered;
        let board = self
            .concrete
            .skateboard_reckoning
            .as_deref_mut()
            .expect("checked");
        // GetEffectiveTransform82C01BF8: the deck as the rider faces it.
        let mut effective = frame.board_transform;
        if frame.board_flipped {
            for axis in [0, 2] {
                effective[axis] = effective[axis].map(|value| -value);
            }
        }
        board.transform = matrix(frame.board_transform);
        board.effective_transform = matrix(effective);
        board.deck_position = vector(frame.board_transform[3]);
        board.deck_nose_dir = vector(effective[2]);
        let system = self.concrete.system.as_deref_mut().expect("checked");
        system.damped_centre_of_mass = vector(frame.damped_centre_of_mass.position);
        system.damped_centre_of_mass_velocity = vector(frame.damped_centre_of_mass.velocity);
        let collision = self.concrete.collision.as_deref_mut().expect("checked");
        collision.wheels_in_contact = physical.collision.wheel_count_0 as i32;
        self.concrete
            .physics
            .as_deref_mut()
            .expect("checked")
            .timestep = dt;
        let animation = self.concrete.animation.as_deref_mut().expect("checked");
        animation.skater_off_board = frame.offboard;
        // TU3 CalcLandingQuality82DE61D0 already ran in the selected solver.
        // The dormant S2 conditioner cannot produce this landing publication.
        let landing = frame.gameplay.landing_quality;
        animation.just_analyzed_sketchyness = landing.landing_data_167;
        animation.sketchyness_anim_result = landing.landing_type_96 as i32;
        animation.sketchyness_anim_scalar = landing.spin_92;
        animation.sketchyness_side_speed = landing.sideways_speed_84;
        let offboard = self.concrete.offboard.as_deref_mut().expect("checked");
        offboard.is_grabbing_object=physical.off_board.flag_304 != 0;
        offboard.has_valid_interactable_object=frame.gameplay.interactable_object.1;
        offboard.is_hiding_board = physical.off_board.hiding_board_321 != 0;
        offboard.is_dropping_board = physical.off_board.dropping_board_322 != 0;
        offboard.is_retrieving_board = physical.off_board.retrieving_board_323 != 0;
        // Skitching FillPhysOut 0x82D4C078: the camera reads the grab position, the HUD and
        // animation consumers the rest.
        let skitch = frame.gameplay.skitch.unwrap_or_default();
        let ground = self.concrete.ground.as_deref_mut().expect("checked");
        ground.skitch_spline_height = skitch.spline_height;
        ground.skitch_contact_speed = skitch.contact_speed;
        ground.skitch_parametric_pos = skitch.parametric_position;
        ground.skitch_delay_until_reskitchable = skitch.delay_until_reskitchable;
        let animation = self.concrete.animation.as_deref_mut().expect("checked");
        animation.skitch_crouch = skitch.crouch;
        animation.skitch_hand_speed = skitch.hand_speed;
        animation.skitch_grip_hand = skitch.grip_hand;
        self.try_publish_skate3_gameplay(frame)?;
        Ok(Skate3Publication { output: std::sync::Arc::clone(self.occurrence()) })
    }

    fn try_publish_skate3_gameplay(&mut self, frame: &crate::skate3_native::FrameOutput) -> Result<(), String> {
        let facts = &frame.gameplay;
        let physical = &frame.physical;
        self.try_publish_collision_type(|| Ok::<_, std::convert::Infallible>(physical.collision.surface_type_16))
            .map_err(|error| format!("Native source surface publication: {error:?}"))?;
        let flag = |byte: usize| facts.state_flags[byte - 52];
        let state = self.concrete.state.as_deref_mut().expect("constructed");
        state.is_foot_braking = flag(52);
        state.is_manual_braking = flag(54);
        state.is_physically_pushing = flag(55);
        state.is_generally_pushing = flag(56);
        state.is_generally_pushing_right_foot = flag(57);
        state.is_generally_braking = flag(58);
        state.is_manual = flag(60);
        (state.special_wipeout_info.state, state.special_wipeout_info.landing_speed) =
            facts.heavy_landing.unwrap_or((-1, 0.0));
        state.skate3_wipeout_countdown = flag(68);
        // TU3 ScoreModule::Update 0x82da3438 reads PhysOut_State +0x4E (flag 78) as
        // mPhysicsRequestsRunout: the line a run-out ends (a held darkcatch stepping off) is lost.
        state.physics_requests_runout = flag(78);
        state.skate3_dark_catch = flag(80);
        state.skate3_physics = true;
        state.skate3_teleport_requested = flag(69);
        // TU3 State+0x45 is mPhysicsRequestedTeleport: the score collector's Wipeout pick
        // (CalculateScoreCollectorType 0x82da3de4), IsSafeToUseSessionMarker and cMsgWipeout read it.
        state.physics_requested_teleport = flag(69);
        if let Some(intents) = self.concrete.intents.as_deref_mut() {
            [intents.skate3_drop_in_lock, intents.skate3_drop_in_release] = facts.drop_in_sounds;
        }
        let animation = self.concrete.animation.as_deref_mut().expect("constructed");
        animation.is_generally_pushing = flag(56);
        [animation.push_mongo, animation.push_regular] = facts.push_style;
        animation.input_turn = facts.animation_turn;
        [animation.is_riding_fakie, animation.is_mirrored, animation.is_riding_switch, animation.is_regular_stance] = facts.stance;
        animation.should_disallow_tricks_due_to_stairs = physical.animation.tricks_blocked_on_stairs_166 != 0;
        let trick = self.concrete.trick.as_deref_mut().expect("constructed");
        trick.current_animation.name = name36(facts.trick);
        trick.current_animation.bits = facts.trick_flags;
        [trick.current_animation.tweak_x, trick.current_animation.tweak_y] = facts.trick_tweak;
        // S3 replaces Update5, including the native bail-pose publication.
        [trick.wipeout_gesture_x,trick.wipeout_gesture_y] = facts.wipeout_gesture;
        let board = self.concrete.skateboard_reckoning.as_deref_mut().expect("constructed");
        board.wheel_pos = std::array::from_fn(|i| vector(frame.board_parts[i].transform[3]));
        let skeleton = self.concrete.skeleton.as_deref_mut().expect("constructed");
        [skeleton.is_left_foot_on_deck, skeleton.is_right_foot_on_deck] = facts.feet.within_deck_box;
        skeleton.skeleton_twist = physical.skeleton.twist_504;
        skeleton.skeleton_deck_yaw = physical.skeleton.deck_yaw_536;
        skeleton.skeleton_deck_pitch = physical.skeleton.deck_pitch_540;
        skeleton.skeleton_foot_l_vel_deck_space = vector(facts.feet.local_velocity[0]);
        skeleton.skeleton_foot_r_vel_deck_space = vector(facts.feet.local_velocity[1]);

        let air = self.concrete.air.as_deref_mut().expect("constructed");
        air.traj_apex = raw(physical.air.trajectory_apex_0);
        air.traj_coll_position = raw(physical.air.collision_position_16);
        air.traj_coll_normal = raw(physical.air.landing_normal_32);
        air.traj_current_pos = raw(physical.air.trajectory_position_64);
        air.traj_target_heading = raw(physical.air.landing_heading_80);
        air.jump_dv = raw(physical.air.jump_velocity_delta_112);
        air.state_intended_jump_vel = raw(physical.air.launch_velocity_128);
        air.landing_normal = raw(physical.air.landing_normal_144);
        air.time_in_known_air = physical.air.time_in_state_176;
        air.traj_collision_time = physical.air.collision_time_180;
        air.traj_collision_speed = physical.air.collision_normal_speed_188;
        air.total_time_to_apex = physical.air.time_to_apex_196;
        air.jump_height = physical.air.jump_height_200;
        air.traj_index = physical.air.trajectory_index_220;
        air.time_to_land_foot_plant = physical.air.footplant_contact_time_208;
        air.foot_plant_total_absorb_time = physical.air.footplant_duration_212;
        air.foot_plant_surface_id = physical.air.footplant_surface_224;
        air.is_left_foot_planting = physical.air.footplant_left_449 != 0;
        air.is_right_foot_planting = physical.air.footplant_right_450 != 0;
        air.is_skateboard_airborne = (200..300).contains(&frame.state) && physical.collision.wheel_count_0 == 0;
        air.is_starting_to_jump = physical.air.launched_442 != 0;
        air.is_foot_planting = physical.air.flag_448 != 0;
        air.is_at_apex = physical.air.reached_apex_436 != 0;
        air.is_traj_valid = physical.air.known_air_valid_437 != 0;
        air.use_air_reckoning = physical.air.use_air_reckoning_452 != 0;
        air.is_flipping = physical.air.flag_441 != 0;
        air.is_front_flipping = physical.air.flag_445 != 0;
        air.height_of_com = physical.air.trajectory_plane_samples_336;
        air.time_since_wheels_touching = facts.board_ground.time_without_wheel_contact;
        // TU3 Update5_ProcessOutput 82DB6FD0 inlines Pumping.Fill: four skeleton
        // absorptions after zeroing the pumping compression, then the acceleration.
        let skeleton = self.concrete.skeleton.as_deref_mut().expect("constructed");
        skeleton.pumping_compression = 0.0;
        skeleton.absorption_compression = frame.pumping.absorption;
        skeleton.ground_normal_compression = frame.pumping.ground_normal_absorption;
        skeleton.ground_normal_min_crouch = frame.pumping.minimum_crouch;
        skeleton.deck_angle_compression = frame.pumping.deck_angle_absorption;
        let ground = self.concrete.ground.as_deref_mut().expect("constructed");
        ground.pumping_acceleration = frame.pumping.pump_acceleration;
        ground.turn = facts.animation_turn;
        ground.jump_strength = facts.jump_strength;
        // TU3 Start/EndSkeletonLineTests82DB63C0/6580 and board probe82C07788
        // already queried the real map. The source solver bypasses S2 Update5,
        // so its dormant ray records cannot supply the air-gap/context collector.
        ground.below_hips_ground_data_start_pos = vector(facts.hips_ground_start);
        ground.below_hips_ground_data_is_valid = facts.hips_ground.valid != 0;
        if ground.below_hips_ground_data_is_valid {
            ground.below_hips_ground_data_position = raw(facts.hips_ground.position);
            ground.below_hips_ground_data_normal = raw(facts.hips_ground.normal);
            ground.below_hips_ground_data_surface_id = facts.hips_ground.surface;
        }
        let probe = facts.deck_ground;
        ground.below_deck_ground_data_start_pos = direction(probe.start);
        ground.below_deck_ground_data_is_valid = probe.hit;
        if probe.hit {
            ground.below_deck_ground_data_position = direction(probe.point);
            ground.below_deck_ground_data_normal = direction(probe.normal);
            ground.below_deck_ground_data_surface_id = probe.surface_tag;
        }
        if crate::diagnostics::test_log::enabled() && (frame.tick % 60 == 0 || self.concrete.state.as_deref().is_some_and(|s| s.has_just_teleported)) {
            crate::diagnostics::test_log::event("skate3_score_ground", format_args!(
                "tick={} hips_start={:?} hips_hit={} hips_point={:?} hips_surface={} deck_start={:?} deck_hit={} deck_point={:?}",
                frame.tick,facts.hips_ground_start,facts.hips_ground.valid != 0,facts.hips_ground.position.map(f32::from_bits),
                facts.hips_ground.surface,probe.start,probe.hit,probe.point));
        }
        ground.is_hippy_jumping = physical.ground.hippy_jumping_322 != 0;
        ground.just_hippy_jumped = physical.ground.hippy_takeoff_323 != 0;
        ground.landing_half_turns = physical.ground.landing_half_turns_312;

        let grinds = self.concrete.grinds.as_deref_mut().expect("constructed");
        let source = &physical.grinds;
        grinds.grind_primitive_dir = raw(source.direction_0);
        grinds.grind_position = raw(source.point_16);
        grinds.grind_normal = raw(source.normal_32);
        grinds.grind_skew = raw(source.across_48);
        grinds.primitive_start_pos = raw(source.primitive_start_64);
        grinds.primitive_end_pos = raw(source.primitive_end_80);
        grinds.cam_target_root = raw(source.camera_target_96);
        grinds.ledge_high_dir = raw(source.high_side_112);
        grinds.grind_impact_speed = source.impact_speed_128;
        grinds.grind_crouch = source.crouch_132;
        grinds.grind_type = match source.words_136_140[0] {
            u32::MAX => crate::physics::GrindType::None,
            0 => crate::physics::GrindType::FiftyFifty,
            1 | 5 => crate::physics::GrindType::Boardslide,
            2 => crate::physics::GrindType::Tipslide,
            3 => crate::physics::GrindType::FiveO,
            4 => crate::physics::GrindType::Backslash,
            other => return Err(format!("Unknown source grind family {other}")),
        };
        grinds.grind_substate = source.words_136_140[1] as i32;
        grinds.grind_scorable_id = super::conditioners::scorable_from_animation_name(name36(source.scoring_name_176));
        let name30 = |value: Option<skate3_core::animation::output::attributes::AttributeName>| crate::play_animation::FastString30 { words: value.map_or([0; 5], |value| value.0) };
        grinds.skating_grind_name = name30(source.animation_name_156);
        grinds.skating_grind_name_scoring = name30(source.scoring_name_176);
        grinds.anim_grind_name = name30(source.surface_name_196);
        grinds.grind_audio_surface_type = source.audio_surface_216 as i32;
        [grinds.grind_pathed_guid, grinds.grind_local_guid] = source.spline_guids_224_232;
        grinds.is_grinding = source.grinding_316 != 0;
        grinds.is_leaving_grind = source.leaving_317 != 0;
        grinds.is_grind_on_front = source.flag_318 != 0;
        grinds.is_grind_facing_forwards = source.flag_319 != 0;
        // SKATE2_TRACE_SKATE3_GRIND: each start and end of the Skate 3 grind's publication.
        {
            static GRINDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if std::env::var_os("SKATE2_TRACE_SKATE3_GRIND").is_some()
                && GRINDING.swap(grinds.is_grinding, std::sync::atomic::Ordering::Relaxed) != grinds.is_grinding
            {
                eprintln!(
                    "skate3 grind publication: grinding={} family={:?} scorable={} scoring_name={:?} animation_name={:?} guids={:#x?} primitive={:?}..{:?}",
                    grinds.is_grinding,
                    source.words_136_140,
                    grinds.grind_scorable_id,
                    source.scoring_name_176.map(|n| n.0),
                    source.animation_name_156.map(|n| n.0),
                    source.spline_guids_224_232,
                    source.primitive_start_64.map(f32::from_bits),
                    source.primitive_end_80.map(f32::from_bits),
                );
            }
        }
        grinds.is_grinding_ledge = source.is_ledge_320 != 0;
        grinds.is_grinding_curb = source.curb_321 != 0;
        grinds.skate3_tipslide_blocks_scoring = source.tipslide_326 != 0;
        // The dormant Physics conditioner's grind stores (Update82e2f970, 82e2dee4..dff0): while
        // the filtered state is Grind, the grind publication, else the cleared values. The
        // grind scoring collector and the mission's GrindCompleted read these.
        let (scorable, pathed, local, kind, on_front, name, scoring_name, crouch) = (
            grinds.grind_scorable_id,
            grinds.grind_pathed_guid,
            grinds.grind_local_guid,
            grinds.grind_type,
            grinds.is_grind_on_front,
            grinds.skating_grind_name,
            grinds.skating_grind_name_scoring,
            grinds.grind_crouch,
        );
        let filtered = self.concrete.filtered.as_deref_mut().expect("checked");
        let grinding = filtered.filtered_physics_state == crate::physics::FilteredPhysicsState::Grind;
        // Skate 2's grind PhysOut keeps its last grind until the next grind fills it, so the
        // conditioner's stores hold the grind through the filtered Grind tail (the exit states
        // after the grind state). Skate 3's grind publication clears at its exit, so its tail
        // keeps the stores of the grind it ended; else the collector ends the trick early and
        // restarts the length.
        let ended_grind_tail = grinding && !grinds.is_grinding && filtered.is_grinding;
        filtered.is_grinding = grinding;
        if !ended_grind_tail {
            filtered.grind_type = if grinding { kind } else { crate::physics::GrindType::None };
            filtered.is_grind_on_front = grinding && on_front;
            filtered.skating_grind_name = if grinding { name } else { crate::play_animation::EMPTY_FAST_STRING_30 };
            filtered.skating_grind_name_scoring = if grinding { scoring_name } else { crate::play_animation::EMPTY_FAST_STRING_30 };
            filtered.grind_scorable_id = if grinding { scorable } else { -1 };
            filtered.grind_crouch = if grinding { crouch } else { 0.0 };
            filtered.grind_pathed_guid = if grinding { pathed } else { u64::MAX };
            filtered.grind_local_guid = if grinding { local } else { u64::MAX };
        }
        let collision = self.concrete.collision.as_deref_mut().expect("constructed");
        collision.average_ground_normal = direction(facts.board_ground.overall_normal);
        collision.anything_in_contact = facts.board_ground.part_contact_count != 0;
        collision.front_truck_in_contact = facts.board_ground.parts[4].in_contact;
        collision.back_truck_in_contact = facts.board_ground.parts[5].in_contact;
        collision.a_truck_in_contact = collision.front_truck_in_contact || collision.back_truck_in_contact;
        collision.deck_in_contact = facts.board_ground.parts[6].in_contact;
        if collision.deck_in_contact {
            let acceleration = direction(facts.board_ground.accelerations[6]);
            let normal = direction(facts.board_ground.parts[6].normal);
            // Host board FillPhysOut's native scale/cap, using source solved acceleration.
            let dot = acceleration.xyz[2].mul_add(normal.xyz[2], acceleration.xyz[1].mul_add(normal.xyz[1], acceleration.xyz[0] * normal.xyz[0]));
            let scaled = f32::from_bits(dot.to_bits() & 0x7fff_ffff) * f32::from_bits(0x3aa3_d70a);
            let positive = if -scaled >= 0. { 0. } else { scaled };
            collision.deck_contact_force = if 1. - positive >= 0. { positive } else { 1. };
        } else { collision.deck_contact_force = 0.; }
        [collision.front_truck_audio_surface, collision.back_truck_audio_surface, collision.deck_audio_surface] = facts.board_ground.part_audio_surfaces.map(|value| value as i32);
        let motion = self.concrete.motion.as_deref_mut().expect("constructed");
        motion.deck_tilt = facts.deck_tilt;
        motion.wheel_hardness = facts.wheel_hardness;
        for wheel in 0..4 {
            let contact = facts.board_ground.parts[wheel];
            collision.wheel_in_contact[wheel] = contact.in_contact;
            collision.wheel_contact_position[wheel] = direction(contact.point);
            collision.wheel_contact_normal[wheel] = direction(contact.normal);
            collision.wheel_audio_surface[wheel] = facts.wheel_lines.audio_surfaces[wheel] as i32;
            collision.wheel_surface_pattern[wheel] = facts.wheel_lines.seam_patterns[wheel] as i32;
            let velocity = frame.board_parts[wheel].velocity;
            motion.wheel_linear_velocity[wheel] = vector([velocity[0], velocity[1], velocity[2], 0.]);
        }
        collision.skeleton_in_contact = facts.contact_flags.any;
        collision.skeleton.contact_force_during_wipeout = facts.wipeout_times[0];
        collision.skeleton.slide_force_during_wipeout = facts.wipeout_times[1];
        collision.skeleton.time_wiping_out = facts.wipeout_times[2];
        collision.skeleton.is_impaled_on_fence = facts.contact_flags.impaled;
        collision.skeleton.has_groin_contacted = facts.specific_contacts[0].recent;
        collision.skeleton.has_face_contacted = facts.specific_contacts[1].recent;
        let audio = &mut collision.skeleton.audio;
        for region in 0..8 {
            let contact = facts.contact_regions[region];
            audio.contact_force[region] = physical.collision.contact_region_response_80[region];
            audio.slide_speed[region] = contact.tangent_speed;
            audio.audio_surface_type[region] = (contact.material_flags & 0x7f) as i32;
        }
        audio.is_groin_contact = facts.specific_contacts[0].current;
        audio.is_face_contact = facts.specific_contacts[1].current;
        [audio.vehicle_impact, audio.skater_impact, audio.pedestrian_impact] = facts.entity_impacts;
        for (index, (target, source)) in collision.skeleton.bones.iter_mut().zip(facts.bone_contacts).enumerate() {
            let delta = facts.bone_velocity_changes[index];
            target.position = vector(source.point);
            target.main.normal = vector(source.normal);
            target.main.slide = vector(source.tangent);
            target.main.force = source.force;
            target.main.acceleration = impact_acceleration(delta, source.normal);
            target.main.surface_id = source.tag;
            target.sub.normal = vector(source.specific_normal);
            target.sub.slide = vector(source.specific_tangent);
            target.sub.force = source.specific_force;
            target.sub.acceleration = impact_acceleration(delta, source.specific_normal);
            target.sub.surface_id = source.specific_tag;
            [target.hit_vehicle, target.hit_pedestrian, target.hit_skater, target.hit_world] = source.groups;
        }
        let offboard = self.concrete.offboard.as_deref_mut().expect("constructed");
        [offboard.left_foot_surface_id, offboard.right_foot_surface_id] = facts.foot_surfaces;
        // Native Feet/footplant/push outputs, also consumed by the current S3 audio source.
        offboard.left_foot_down = physical.off_board.flags_306_307[0] != 0 || physical.air.footplant_left_449 != 0;
        offboard.right_foot_down = physical.off_board.flags_306_307[1] != 0 || physical.air.footplant_right_450 != 0;
        offboard.is_holding_skateboard = physical.off_board.flag_311 != 0;
        Ok(())
    }
}

/// SkeletonCollision FillAudio uses the absolute normal projection of the
/// pose-derived velocity change, without dividing it by a second time step.
fn impact_acceleration(delta: [f32; 4], normal: [f32; 4]) -> f32 {
    ((delta[0]*normal[0]+delta[1]*normal[1])+delta[2]*normal[2]).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bone_impact_uses_normal_velocity_change_and_ignores_tangential_motion() {
        assert_eq!(impact_acceleration([5.,-12.,7.,99.],[0.,1.,0.,33.]),12.);
        assert_eq!(impact_acceleration([5.,0.,7.,0.],[0.,1.,0.,0.]),0.);
        assert_eq!(impact_acceleration([0.;4],[0.,1.,0.,0.]),0.);
    }
    #[test]
    fn native_trick_names_extend_the_shared_encoding_without_rehashing() {
        for name in ["", "kickflip", "nosemanual", "fakie_shuvit", "invert_handplant_fs_1ftfront"] {
            let source = skate3_core::animation::skeleton_input::name::encode(name.as_bytes());
            assert_eq!(name36(Some(source)), crate::play_animation::FastString36::encode(name).unwrap(), "{name}");
        }
    }
}
