//! Concrete primitive component constructors inside the retained `PhysOut` owner.
//! These values are not an independently constructed output owner. Motion and
//! System remainders exclude fields already retained by the existing projection.
//! Mount only through the complete nineteen-component constructor sequence.
use super::{FORWARD, RIGHT, SkeletonRootMatrix, SkeletonRootVector, UP, ZERO};
fn identity() -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns([RIGHT, UP, FORWARD, ZERO])
}

/// Native `PhysOut_SkateboardMotion` constructor/reset fields.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(crate) struct MotionRemainder {
    pub(crate) teleport_transform: SkeletonRootMatrix,
    pub(crate) deck_acceleration: SkeletonRootVector,
    pub(crate) emperical_deck_acceleration: SkeletonRootVector,
    pub(crate) emperical_deck_velocity: SkeletonRootVector,
    pub(crate) speed_wobble_effect: f32,
    pub(crate) speed_wobble_tilt: f32,
    pub(crate) deck_tilt: f32,
    pub(crate) pump_potential: f32,
    pub(crate) heading_adjust_factor: f32,
    pub(crate) anim_push_speed: f32,
    pub(crate) wheel_hardness: f32,
    pub(crate) wheel_linear_velocity: [SkeletonRootVector; 4],
    pub(crate) did_teleport: bool,
    pub(crate) skateboard_flipped: bool,
    pub(crate) is_push_accelerating: bool,
    pub(crate) is_physically_pushing: bool,
    pub(crate) is_at_pushable_speed: bool,
    pub(crate) is_intentional_pump: bool,
}
impl MotionRemainder {
    pub(crate) fn reset_values() -> Self {
        Self {
            teleport_transform: identity(),
            deck_acceleration: ZERO,
            emperical_deck_acceleration: ZERO,
            emperical_deck_velocity: ZERO,
            speed_wobble_effect: 0.0,
            speed_wobble_tilt: 0.0,
            deck_tilt: 0.0,
            pump_potential: 0.0,
            heading_adjust_factor: 0.0,
            anim_push_speed: 0.0,
            wheel_hardness: 1.0,
            wheel_linear_velocity: [ZERO; 4],
            did_teleport: false,
            skateboard_flipped: false,
            is_push_accelerating: false,
            is_physically_pushing: false,
            is_at_pushable_speed: false,
            is_intentional_pump: false,
        }
    }
}

/// Native `PhysOut_SystemReckoning` constructor/reset fields.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SystemRemainder {
    pub(crate) damped_centre_of_mass_velocity: SkeletonRootVector,
    pub(crate) ball_spring_offset: SkeletonRootVector,
    pub(crate) damped_centre_of_mass: SkeletonRootVector,
    pub(crate) target_lean_angle: f32,
}
impl SystemRemainder {
    pub(crate) fn reset_values() -> Self {
        Self {
            damped_centre_of_mass_velocity: ZERO,
            ball_spring_offset: ZERO,
            damped_centre_of_mass: ZERO,
            target_lean_angle: 0.0,
        }
    }
}

/// Native `PhysOut_SkateboardReckoning` constructor/reset fields.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkateboardReckoning {
    pub(crate) transform: SkeletonRootMatrix,
    pub(crate) effective_transform: SkeletonRootMatrix,
    pub(crate) deck_nose_dir: SkeletonRootVector,
    pub(crate) deck_position: SkeletonRootVector,
    pub(crate) vel_direction: SkeletonRootVector,
    pub(crate) skater_look_direction: SkeletonRootVector,
    pub(crate) deck_to_wheel_base: f32,
    pub(crate) contact_error_dist: f32,
    pub(crate) wheel_pos: [SkeletonRootVector; 4],
    pub(crate) is_transform_flipped: bool,
}
impl SkateboardReckoning {
    pub(crate) fn reset_values() -> Self {
        Self {
            transform: identity(),
            effective_transform: identity(),
            deck_nose_dir: RIGHT,
            deck_position: ZERO,
            vel_direction: ZERO,
            skater_look_direction: ZERO,
            deck_to_wheel_base: 0.0,
            contact_error_dist: 0.0,
            wheel_pos: [ZERO; 4],
            is_transform_flipped: false,
        }
    }
}

/// Native `PhysOut_Physics` constructor/reset fields.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Physics {
    pub(crate) timestep: f32,
    pub(crate) wipeout_impulse: SkeletonRootVector,
    pub(crate) is_impulse_for_wipeout: bool,
}
impl Physics {
    pub(crate) fn reset_values() -> Self {
        Self {
            timestep: f32::from_bits(0x3c88_8889),
            wipeout_impulse: ZERO,
            is_impulse_for_wipeout: false,
        }
    }
}

/// Native `PhysOut_Camera` constructor/reset fields.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(crate) struct Camera {
    pub(crate) camera_matrix: SkeletonRootMatrix,
    pub(crate) physics_debug_system_focus_pos: SkeletonRootVector,
    pub(crate) physics_debug_system_focus_vel: SkeletonRootVector,
    pub(crate) physics_debug_system_focus_front: SkeletonRootVector,
}
impl Camera {
    pub(crate) fn reset_values() -> Self {
        Self {
            camera_matrix: identity(),
            physics_debug_system_focus_pos: ZERO,
            physics_debug_system_focus_vel: ZERO,
            physics_debug_system_focus_front: FORWARD,
        }
    }
}

/// Native `PhysOut_Intents` constructor/reset fields.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(crate) struct Intents {
    pub(crate) is_pushing: bool,
    pub(crate) is_player_providing_input: bool,
    pub(crate) is_new_auto_pump: bool,
    pub(crate) is_kickturning: bool,
    pub(crate) is_anticipating: bool,
    pub(crate) is_generally_doing_coffin: bool,
    pub(crate) is_generally_grabbing: bool,
    pub(crate) has_world_grab_intent: bool,
}
impl Intents {
    pub(crate) fn reset_values() -> Self {
        Self {
            is_pushing: false,
            is_player_providing_input: false,
            is_new_auto_pump: false,
            is_kickturning: false,
            is_anticipating: false,
            is_generally_doing_coffin: false,
            is_generally_grabbing: false,
            has_world_grab_intent: false,
        }
    }
}

/// Native `PhysOut_OffBoard` constructor/reset fields.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(crate) struct OffBoard {
    pub(crate) anim_obj_mov_z: f32,
    pub(crate) anim_obj_mov_x: f32,
    pub(crate) anim_obj_force_z: f32,
    pub(crate) anim_obj_force_x: f32,
    pub(crate) anim_obj_angular_speed: f32,
    pub(crate) object_mass: f32,
    pub(crate) anim_crouch: f32,
    pub(crate) time_to_land: f32,
    pub(crate) yaw_to_free_board: f32,
    pub(crate) pitch_to_free_board: f32,
    pub(crate) com_height_on_landing: f32,
    pub(crate) landing_velocity: f32,
    pub(crate) left_foot_surface_id: u32,
    pub(crate) right_foot_surface_id: u32,
    pub(crate) is_grabbing_object: bool,
    pub(crate) has_valid_interactable_object: bool,
    pub(crate) left_foot_down: bool,
    pub(crate) right_foot_down: bool,
    pub(crate) is_mounting: bool,
    pub(crate) is_unmounting: bool,
    pub(crate) is_air_dismount: bool,
    pub(crate) is_holding_skateboard: bool,
    pub(crate) is_board_mount_invalid: bool,
    pub(crate) on_deck: bool,
    pub(crate) can_land_on_skateboard: bool,
    pub(crate) land_on_skateboard_flip_board: bool,
    pub(crate) trajectory_ready: bool,
    pub(crate) has_landed: bool,
    pub(crate) is_hiding_board: bool,
    pub(crate) is_dropping_board: bool,
    pub(crate) is_retrieving_board: bool,
    pub(crate) is_player_retrieving_board: bool,
    pub(crate) is_jumping: bool,
    pub(crate) is_starting_standing_jump: bool,
    pub(crate) is_locomotion_viable: bool,
}
impl OffBoard {
    pub(crate) fn reset_values() -> Self {
        Self {
            anim_obj_mov_z: 0.0,
            anim_obj_mov_x: 0.0,
            anim_obj_force_z: 0.0,
            anim_obj_force_x: 0.0,
            anim_obj_angular_speed: 0.0,
            object_mass: 0.0,
            anim_crouch: 0.0,
            time_to_land: 0.0,
            yaw_to_free_board: 0.0,
            pitch_to_free_board: 0.0,
            com_height_on_landing: 0.0,
            landing_velocity: 0.0,
            left_foot_surface_id: 0,
            right_foot_surface_id: 0,
            is_grabbing_object: false,
            has_valid_interactable_object: false,
            left_foot_down: false,
            right_foot_down: false,
            is_mounting: false,
            is_unmounting: false,
            is_air_dismount: false,
            is_holding_skateboard: false,
            is_board_mount_invalid: false,
            on_deck: false,
            can_land_on_skateboard: false,
            land_on_skateboard_flip_board: false,
            trajectory_ready: false,
            has_landed: false,
            is_hiding_board: false,
            is_dropping_board: false,
            is_retrieving_board: false,
            is_player_retrieving_board: false,
            is_jumping: false,
            is_starting_standing_jump: false,
            is_locomotion_viable: true,
        }
    }
}

/// Trajectory constructor82e2a300 and reset82e24ce0: vector time is splatted -1.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Trajectory {
    pub(crate) start: SkeletonRootVector,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) acceleration: SkeletonRootVector,
    pub(crate) duration: [f32; 4],
}
/// The source reset preserves bits30 and0..27 of the actual hand-plant word.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HandPlant {
    pub(crate) position: SkeletonRootVector,
    pub(crate) estimated_time_to_apex: f32,
    pub(crate) flags: u32,
}
/// `PhysOut_Air` constructor82e2a300 invokes reset82e24ce0.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(crate) struct Air {
    pub(crate) traj_apex: SkeletonRootVector,
    pub(crate) traj_coll_position: SkeletonRootVector,
    pub(crate) traj_coll_normal: SkeletonRootVector,
    pub(crate) traj_start_position: SkeletonRootVector,
    pub(crate) traj_current_pos: SkeletonRootVector,
    pub(crate) traj_target_heading: SkeletonRootVector,
    pub(crate) trajectory_offset: SkeletonRootVector,
    pub(crate) jump_dv: SkeletonRootVector,
    pub(crate) state_intended_jump_vel: SkeletonRootVector,
    pub(crate) landing_normal: SkeletonRootVector,
    pub(crate) time_in_known_air: f32,
    pub(crate) traj_total_time: f32,
    pub(crate) traj_collision_time: f32,
    pub(crate) traj_collision_speed: f32,
    pub(crate) time_since_wheels_touching: f32,
    pub(crate) total_time_to_apex: f32,
    pub(crate) jump_height: f32,
    pub(crate) body_spin_speed: f32,
    pub(crate) time_to_land_foot_plant: f32,
    pub(crate) foot_plant_total_absorb_time: f32,
    pub(crate) traj_index: i32,
    pub(crate) foot_plant_surface_id: u32,
    pub(crate) hand_plant_surface_id: u32,
    pub(crate) trajectory_struct: Trajectory,
    pub(crate) anim_hand_plant_info_struct: HandPlant,
    pub(crate) height_of_com: [f32; 25],
    pub(crate) is_at_apex: bool,
    pub(crate) is_traj_valid: bool,
    /// TU3 PhysOut_Air+452: the trajectory's velocity is locked (KnownAir, ground wipeout).
    #[cfg(feature = "skate3-runtime")]
    pub(crate) use_air_reckoning: bool,
    pub(crate) is_skateboard_airborne: bool,
    pub(crate) is_preparing_to_jump: bool,
    pub(crate) is_starting_to_jump: bool,
    pub(crate) is_flipping: bool,
    pub(crate) state_triggered_jump: bool,
    pub(crate) is_targeting_grind: bool,
    pub(crate) is_targeting_wall_ride: bool,
    pub(crate) is_front_flipping: bool,
    pub(crate) found_hand_plant_pos: bool,
    pub(crate) is_landing_foot_plant: bool,
    pub(crate) is_foot_planting: bool,
    pub(crate) is_left_foot_planting: bool,
    pub(crate) is_right_foot_planting: bool,
    pub(crate) hand_plant_grabbing: bool,
}
impl Air {
    pub(crate) fn reset_values() -> Self {
        Self {
            traj_apex: ZERO,
            traj_coll_position: ZERO,
            traj_coll_normal: ZERO,
            traj_start_position: ZERO,
            traj_current_pos: ZERO,
            traj_target_heading: ZERO,
            trajectory_offset: ZERO,
            jump_dv: ZERO,
            state_intended_jump_vel: ZERO,
            landing_normal: UP,
            time_in_known_air: 0.0,
            traj_total_time: 0.0,
            traj_collision_time: 0.0,
            traj_collision_speed: 0.0,
            time_since_wheels_touching: 0.0,
            total_time_to_apex: 0.0,
            jump_height: 0.0,
            body_spin_speed: 0.0,
            time_to_land_foot_plant: -1.0,
            foot_plant_total_absorb_time: -1.0,
            traj_index: 0,
            foot_plant_surface_id: 0,
            hand_plant_surface_id: 0,
            trajectory_struct: Trajectory {
                start: ZERO,
                velocity: ZERO,
                acceleration: ZERO,
                duration: [-1.0; 4],
            },
            anim_hand_plant_info_struct: HandPlant {
                position: ZERO,
                estimated_time_to_apex: f32::MAX,
                flags: 0,
            },
            height_of_com: [0.0; 25],
            is_at_apex: false,
            is_traj_valid: false,
            #[cfg(feature = "skate3-runtime")]
            use_air_reckoning: false,
            is_skateboard_airborne: false,
            is_preparing_to_jump: false,
            is_starting_to_jump: false,
            is_flipping: false,
            state_triggered_jump: false,
            is_targeting_grind: false,
            is_targeting_wall_ride: false,
            is_front_flipping: false,
            found_hand_plant_pos: false,
            is_landing_foot_plant: false,
            is_foot_planting: false,
            is_left_foot_planting: false,
            is_right_foot_planting: false,
            hand_plant_grabbing: false,
        }
    }
    pub(crate) fn reset(&mut self) {
        let preserved = self.anim_hand_plant_info_struct.flags & 0x4fff_ffff;
        *self = Self::reset_values();
        self.anim_hand_plant_info_struct.flags = preserved;
    }
}

/// Native `Skeleton` reset82e253a8; Ground excludes existing projected members.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(crate) struct Skeleton {
    pub(crate) effective_anim_z: SkeletonRootVector,
    pub(crate) skeleton_head_look: SkeletonRootVector,
    pub(crate) skeleton_head_pos: SkeletonRootVector,
    pub(crate) skeleton_hips_look: SkeletonRootVector,
    pub(crate) skeleton_hips_pos: SkeletonRootVector,
    pub(crate) skeleton_hips_mat_y: SkeletonRootVector,
    pub(crate) skeleton_hips_mat_z: SkeletonRootVector,
    pub(crate) skeleton_foot_right_bone_pos: SkeletonRootVector,
    pub(crate) skeleton_foot_left_bone_pos: SkeletonRootVector,
    pub(crate) skeleton_foot_left_pos: SkeletonRootVector,
    pub(crate) skeleton_foot_right_pos: SkeletonRootVector,
    pub(crate) skeleton_hips_vel: SkeletonRootVector,
    pub(crate) skeleton_foot_l_vel_deck_space: SkeletonRootVector,
    pub(crate) skeleton_foot_r_vel_deck_space: SkeletonRootVector,
    pub(crate) skeleton_foot_left_vel: SkeletonRootVector,
    pub(crate) skeleton_foot_right_vel: SkeletonRootVector,
    pub(crate) skeleton_foot_right_look: SkeletonRootVector,
    pub(crate) skeleton_foot_left_look: SkeletonRootVector,
    pub(crate) skeleton_torso_angular_vel: SkeletonRootVector,
    pub(crate) skeleton_right_foot_angular_vel: SkeletonRootVector,
    pub(crate) skeleton_left_foot_angular_vel: SkeletonRootVector,
    pub(crate) skeleton_torso_angular_vel_delta: SkeletonRootVector,
    pub(crate) skeleton_hook_to_hips: SkeletonRootVector,
    pub(crate) skeleton_root_matrix: SkeletonRootMatrix,
    pub(crate) effective_skeleton_root_matrix: SkeletonRootMatrix,
    pub(crate) skeleton_lean: f32,
    pub(crate) skeleton_twist: f32,
    pub(crate) shoulder_twist: f32,
    pub(crate) skeleton_time_to_land: f32,
    pub(crate) pumping_compression: f32,
    pub(crate) absorption_compression: f32,
    pub(crate) ground_normal_compression: f32,
    pub(crate) feet_ik_compression: f32,
    pub(crate) ground_normal_min_crouch: f32,
    pub(crate) deck_angle_compression: f32,
    pub(crate) skeleton_deck_yaw: f32,
    pub(crate) skeleton_deck_pitch: f32,
    pub(crate) time_in_wipeout_over: f32,
    pub(crate) off_board_look_at_x: f32,
    pub(crate) off_board_look_at_y: f32,
    pub(crate) average_left_leg_speed_local: f32,
    pub(crate) average_right_leg_speed_local: f32,
    pub(crate) average_left_arm_speed_local: f32,
    pub(crate) average_right_arm_speed_local: f32,
    pub(crate) time_until_wipeout_teleport: f32,
    pub(crate) abort_push: bool,
    pub(crate) skateboard_dont_push: bool,
    pub(crate) tell_anim_dont_push_or_brake: bool,
    pub(crate) is_wipeout_over: bool,
    /// Skate 3 physics: PhysOut_Skeleton+580, the wipeout response strength TU3's Hall of
    /// Meat sums (UpdateInWipeout 82DAF6D4).
    pub(crate) skate3_response_strength: f32,
    pub(crate) is_left_foot_on_deck: bool,
    pub(crate) is_right_foot_on_deck: bool,
    pub(crate) is_grabbing_left: bool,
    pub(crate) is_grabbing_right: bool,
    pub(crate) is_wipeout_teleport_pending: bool,
    pub(crate) is_proxy_in_collision: bool,
}
impl Skeleton {
    pub(crate) fn reset_values() -> Self {
        Self {
            effective_anim_z: ZERO,
            skeleton_head_look: ZERO,
            skeleton_head_pos: ZERO,
            skeleton_hips_look: ZERO,
            skeleton_hips_pos: ZERO,
            skeleton_hips_mat_y: ZERO,
            skeleton_hips_mat_z: ZERO,
            skeleton_foot_right_bone_pos: ZERO,
            skeleton_foot_left_bone_pos: ZERO,
            skeleton_foot_left_pos: ZERO,
            skeleton_foot_right_pos: ZERO,
            skeleton_hips_vel: ZERO,
            skeleton_foot_l_vel_deck_space: ZERO,
            skeleton_foot_r_vel_deck_space: ZERO,
            skeleton_foot_left_vel: ZERO,
            skeleton_foot_right_vel: ZERO,
            skeleton_foot_right_look: RIGHT,
            skeleton_foot_left_look: RIGHT,
            skeleton_torso_angular_vel: ZERO,
            skeleton_right_foot_angular_vel: ZERO,
            skeleton_left_foot_angular_vel: ZERO,
            skeleton_torso_angular_vel_delta: ZERO,
            skeleton_hook_to_hips: ZERO,
            skeleton_root_matrix: identity(),
            effective_skeleton_root_matrix: identity(),
            skeleton_lean: 0.0,
            skeleton_twist: 0.0,
            shoulder_twist: 0.0,
            skeleton_time_to_land: 0.0,
            pumping_compression: 0.0,
            absorption_compression: 0.0,
            ground_normal_compression: 0.0,
            feet_ik_compression: 0.0,
            ground_normal_min_crouch: 0.0,
            deck_angle_compression: 0.0,
            skeleton_deck_yaw: 0.0,
            skeleton_deck_pitch: 0.0,
            time_in_wipeout_over: 0.0,
            off_board_look_at_x: 0.0,
            off_board_look_at_y: 0.0,
            average_left_leg_speed_local: 0.0,
            average_right_leg_speed_local: 0.0,
            average_left_arm_speed_local: 0.0,
            average_right_arm_speed_local: 0.0,
            time_until_wipeout_teleport: 0.0,
            abort_push: false,
            skateboard_dont_push: false,
            tell_anim_dont_push_or_brake: false,
            is_wipeout_over: false,
            skate3_response_strength: 0.0,
            is_left_foot_on_deck: false,
            is_right_foot_on_deck: false,
            is_grabbing_left: false,
            is_grabbing_right: false,
            is_wipeout_teleport_pending: false,
            is_proxy_in_collision: false,
        }
    }
}

/// Native `Ground` reset82e250c8; Ground excludes existing projected members.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(crate) struct GroundRemainder {
    pub(crate) last_ground_pos: SkeletonRootVector,
    pub(crate) anti_flip_torque: SkeletonRootVector,
    pub(crate) below_deck_ground_data_start_pos: SkeletonRootVector,
    pub(crate) below_deck_ground_data_position: SkeletonRootVector,
    pub(crate) below_deck_ground_data_normal: SkeletonRootVector,
    pub(crate) below_hips_ground_data_start_pos: SkeletonRootVector,
    pub(crate) below_hips_ground_data_position: SkeletonRootVector,
    pub(crate) below_hips_ground_data_normal: SkeletonRootVector,
    pub(crate) below_deck_ground_data_surface_id: u32,
    pub(crate) below_hips_ground_data_surface_id: u32,
    pub(crate) turn: f32,
    pub(crate) pumping_acceleration: f32,
    pub(crate) ground_force: f32,
    pub(crate) time_to_skitch: f32,
    pub(crate) skitch_spline_height: f32,
    pub(crate) skitch_contact_speed: f32,
    pub(crate) skitch_parametric_pos: f32,
    pub(crate) skitch_delay_until_reskitchable: f32,
    pub(crate) pushing_delay_until_repushable: f32,
    pub(crate) jump_strength: f32,
    pub(crate) wall_ride_exit: bool,
    pub(crate) anti_flip_nudge_present: bool,
    pub(crate) is_pinning: bool,
    pub(crate) below_deck_ground_data_is_valid: bool,
    pub(crate) below_hips_ground_data_is_valid: bool,
    pub(crate) is_hippy_jumping: bool,
    pub(crate) is_full_hippy_jump: bool,
    pub(crate) just_hippy_jumped: bool,
    /// TU3 PhysOut_Ground +0x138: the hippy jump's half turns, from the landing-on-deck state.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) landing_half_turns: i32,
}
impl GroundRemainder {
    pub(crate) fn reset_values() -> Self {
        Self {
            last_ground_pos: ZERO,
            anti_flip_torque: ZERO,
            below_deck_ground_data_start_pos: ZERO,
            below_deck_ground_data_position: ZERO,
            below_deck_ground_data_normal: ZERO,
            below_hips_ground_data_start_pos: ZERO,
            below_hips_ground_data_position: ZERO,
            below_hips_ground_data_normal: ZERO,
            below_deck_ground_data_surface_id: 0,
            below_hips_ground_data_surface_id: 0,
            turn: 0.0,
            pumping_acceleration: 0.0,
            ground_force: 0.0,
            time_to_skitch: -1.0,
            skitch_spline_height: 0.0,
            skitch_contact_speed: 0.0,
            skitch_parametric_pos: 0.0,
            skitch_delay_until_reskitchable: 0.0,
            pushing_delay_until_repushable: 0.0,
            jump_strength: 0.0,
            wall_ride_exit: false,
            anti_flip_nudge_present: false,
            is_pinning: false,
            below_deck_ground_data_is_valid: false,
            below_hips_ground_data_is_valid: false,
            is_hippy_jumping: false,
            is_full_hippy_jump: false,
            just_hippy_jumped: false,
            #[cfg(feature = "skate3-runtime")]
            landing_half_turns: 0,
        }
    }
}

/// Animation reset82e255f8; time starts zero, wipeout TTL starts `f32::MAX`.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(crate) struct Animation {
    pub(crate) anim_tilt: SkeletonRootVector,
    pub(crate) last_anim_tilt: SkeletonRootVector,
    pub(crate) animation_turn: f32,
    pub(crate) animation_target_lean: f32,
    pub(crate) animation_input_lean: f32,
    pub(crate) cam_animation_turn: f32,
    pub(crate) cam_animation_input_lean: f32,
    pub(crate) animation_lean_quickness: f32,
    pub(crate) animation_lean_direction: f32,
    pub(crate) animation_holding: f32,
    pub(crate) input_turn: f32,
    pub(crate) input_kickturn: f32,
    pub(crate) centre_of_mass_to_deck: f32,
    pub(crate) pumping_strength: f32,
    pub(crate) sketchyness: f32,
    pub(crate) sketchyness_side_speed: f32,
    pub(crate) sketchyness_cosine: f32,
    pub(crate) sketchyness_anim_scalar: f32,
    pub(crate) sketchyness_anim_result: i32,
    pub(crate) perceived_grind_type: i32,
    pub(crate) deck_ground_acceleration: SkeletonRootVector,
    pub(crate) time_since_last_input: f32,
    pub(crate) skitch_crouch: f32,
    pub(crate) skitch_hand_speed: f32,
    pub(crate) skitch_grip_hand: i32,
    pub(crate) wipeout_estimated_time_to_land: f32,
    pub(crate) is_carving: bool,
    pub(crate) is_perceived_grind_on_front: bool,
    pub(crate) should_anim_perceive_grinding: bool,
    pub(crate) is_riding_fakie: bool,
    pub(crate) is_mirrored: bool,
    pub(crate) is_riding_switch: bool,
    pub(crate) is_regular_stance: bool,
    pub(crate) is_generally_pushing: bool,
    pub(crate) transitioning_on_off_board: bool,
    pub(crate) skater_off_board: bool,
    pub(crate) running_out: bool,
    pub(crate) push_mongo: bool,
    pub(crate) push_regular: bool,
    pub(crate) is_no_input: bool,
    pub(crate) should_disallow_tricks_due_to_stairs: bool,
    pub(crate) just_analyzed_sketchyness: bool,
}
impl Animation {
    pub(crate) fn reset_values() -> Self {
        Self {
            anim_tilt: ZERO,
            last_anim_tilt: ZERO,
            animation_turn: 0.0,
            animation_target_lean: 0.0,
            animation_input_lean: 0.0,
            cam_animation_turn: 0.0,
            cam_animation_input_lean: 0.0,
            animation_lean_quickness: 0.0,
            animation_lean_direction: 0.0,
            animation_holding: 0.0,
            input_turn: 0.0,
            input_kickturn: 0.0,
            centre_of_mass_to_deck: 0.0,
            pumping_strength: 0.0,
            sketchyness: 0.0,
            sketchyness_side_speed: 0.0,
            sketchyness_cosine: 0.0,
            sketchyness_anim_scalar: 0.0,
            sketchyness_anim_result: 0,
            perceived_grind_type: -1,
            deck_ground_acceleration: ZERO,
            time_since_last_input: 0.0,
            skitch_crouch: 0.0,
            skitch_hand_speed: 0.0,
            skitch_grip_hand: 0,
            wipeout_estimated_time_to_land: f32::MAX,
            is_carving: false,
            is_perceived_grind_on_front: false,
            should_anim_perceive_grinding: false,
            is_riding_fakie: false,
            is_mirrored: false,
            is_riding_switch: false,
            is_regular_stance: false,
            is_generally_pushing: false,
            transitioning_on_off_board: false,
            skater_off_board: false,
            running_out: false,
            push_mongo: false,
            push_regular: false,
            is_no_input: false,
            should_disallow_tricks_due_to_stairs: false,
            just_analyzed_sketchyness: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GrabSplineDescriptor {
    pub(crate) object_type: i32,
    pub(crate) object_id: i32,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpecialWipeout {
    pub(crate) state: i32,
    pub(crate) landing_speed: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WheelLanding {
    pub(crate) impact_strength: [f32; 4],
    pub(crate) landed: [bool; 4],
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CollisionAudio {
    pub(crate) contact_force: [f32; 8],
    pub(crate) slide_speed: [f32; 8],
    pub(crate) audio_surface_type: [i32; 8],
    pub(crate) is_groin_contact: bool,
    pub(crate) is_face_contact: bool,
    pub(crate) vehicle_impact: f32,
    pub(crate) skater_impact: f32,
    pub(crate) pedestrian_impact: f32,
}

/// `PhysOut_State` reset82e25550.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(crate) struct State {
    pub(crate) update_count: i32,
    pub(crate) state_update_count: i32,
    pub(crate) teleport_exit_state: i32,
    pub(crate) physics_state_category: i32,
    pub(crate) physics_state: i32,
    pub(crate) frames_since_reset: i32,
    pub(crate) interactable_object_id: u32,
    pub(crate) grab_spline_desc: GrabSplineDescriptor,
    pub(crate) special_wipeout_info: SpecialWipeout,
    pub(crate) is_foot_braking: bool,
    pub(crate) is_manual_braking: bool,
    pub(crate) is_physically_pushing: bool,
    pub(crate) is_generally_pushing: bool,
    pub(crate) is_generally_pushing_right_foot: bool,
    pub(crate) is_generally_braking: bool,
    pub(crate) is_wiping_out: bool,
    pub(crate) is_manual: bool,
    pub(crate) is_teleport_finished: bool,
    pub(crate) has_just_teleported: bool,
    pub(crate) phys_state_wants_wipeout: bool,
    pub(crate) physics_debug_system_on: bool,
    pub(crate) physics_requests_wipeout: bool,
    pub(crate) is_reverting: bool,
    pub(crate) is_debug_render_player: bool,
    pub(crate) physics_requested_teleport: bool,
    pub(crate) revert_exited_normally: bool,
    pub(crate) collide_with_world_only: bool,
    pub(crate) can_leave_wipeout: bool,
    pub(crate) apply_pinning_optimizations: bool,
    pub(crate) is_hand_planting: bool,
    pub(crate) is_offboard: bool,
    pub(crate) is_offboard_ground: bool,
    pub(crate) physics_requests_dismount: bool,
    pub(crate) physics_requests_runout: bool,
    /// Skate 3 physics: PhysOut_State+68/+69 from WipeoutGround FillPhysOut 82D3EEE8 (the
    /// respawn countdown is running / a teleport is requested), TU3's Hall of Meat wipeout gate.
    pub(crate) skate3_wipeout_countdown: bool,
    /// Skate 3 physics: state flag 80 (processed flags 2484 bit 21, a darkcatch), TU3's
    /// `PhysOut_State` +0x50, which its audio reads as the inverted-board flag.
    pub(crate) skate3_dark_catch: bool,
    /// PC port: Skate 3 physics publishes this state.
    pub(crate) skate3_physics: bool,
    pub(crate) skate3_teleport_requested: bool,
}
impl State {
    pub(crate) fn reset_values() -> Self {
        Self {
            update_count: 0,
            state_update_count: 0,
            teleport_exit_state: 0,
            physics_state_category: 0,
            physics_state: 0,
            frames_since_reset: 0,
            interactable_object_id: 0,
            grab_spline_desc: GrabSplineDescriptor {
                object_type: 0,
                object_id: 0,
            },
            special_wipeout_info: SpecialWipeout {
                state: -1,
                landing_speed: 0.0,
            },
            is_foot_braking: false,
            is_manual_braking: false,
            is_physically_pushing: false,
            is_generally_pushing: false,
            is_generally_pushing_right_foot: false,
            is_generally_braking: false,
            is_wiping_out: false,
            is_manual: false,
            is_teleport_finished: false,
            has_just_teleported: false,
            phys_state_wants_wipeout: false,
            physics_debug_system_on: false,
            physics_requests_wipeout: false,
            is_reverting: false,
            is_debug_render_player: false,
            physics_requested_teleport: false,
            revert_exited_normally: false,
            collide_with_world_only: false,
            can_leave_wipeout: false,
            apply_pinning_optimizations: false,
            is_hand_planting: false,
            is_offboard: false,
            is_offboard_ground: false,
            physics_requests_dismount: false,
            physics_requests_runout: false,
            skate3_wipeout_countdown: false,
            skate3_dark_catch: false,
            skate3_physics: false,
            skate3_teleport_requested: false,
        }
    }
}

/// `PhysOut_Audio` reset82e25028.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(crate) struct Audio {
    pub(crate) deck_local_xyz_angular_speed: SkeletonRootVector,
    pub(crate) carving_slide: f32,
    pub(crate) wheel_slide: f32,
    pub(crate) jump_fraction: f32,
    pub(crate) grind_type: i32,
    pub(crate) grind_impact_speed: f32,
    pub(crate) averaged_deck_contact_force: f32,
    pub(crate) grind_audio_surface_type: i32,
    pub(crate) off_board_jump_landing: i32,
    pub(crate) hippy_jump_strength: i32,
    pub(crate) wheel_landing: WheelLanding,
    pub(crate) prev_wheel_landing: WheelLanding,
    pub(crate) averaged_skeleton_collision_info: CollisionAudio,
    pub(crate) is_grinding: bool,
    pub(crate) just_wiped_out: bool,
    pub(crate) just_respawned: bool,
    pub(crate) is_on_stairs: bool,
    pub(crate) is_walking_up: bool,
    pub(crate) is_walking_down: bool,
    pub(crate) left_foot_just_landed: bool,
    pub(crate) right_foot_just_landed: bool,
}
impl Audio {
    pub(crate) fn reset_values() -> Self {
        Self {
            deck_local_xyz_angular_speed: ZERO,
            carving_slide: 0.0,
            wheel_slide: 0.0,
            jump_fraction: 0.0,
            grind_type: -1,
            grind_impact_speed: 0.0,
            averaged_deck_contact_force: 0.0,
            grind_audio_surface_type: 0,
            off_board_jump_landing: 1,
            hippy_jump_strength: 0,
            wheel_landing: WheelLanding {
                impact_strength: [0.0; 4],
                landed: [false; 4],
            },
            prev_wheel_landing: WheelLanding {
                impact_strength: [0.0; 4],
                landed: [false; 4],
            },
            averaged_skeleton_collision_info: CollisionAudio {
                contact_force: [0.0; 8],
                slide_speed: [0.0; 8],
                audio_surface_type: [0; 8],
                is_groin_contact: false,
                is_face_contact: false,
                vehicle_impact: 0.0,
                skater_impact: 0.0,
                pedestrian_impact: 0.0,
            },
            is_grinding: false,
            just_wiped_out: false,
            just_respawned: false,
            is_on_stairs: false,
            is_walking_up: false,
            is_walking_down: false,
            left_foot_just_landed: false,
            right_foot_just_landed: false,
        }
    }
    /// `SetResetValues` deliberately preserves both wheel-landing histories.
    /// Their initial zero stores belong to constructor82e2a370 alone.
    pub(crate) fn reset(&mut self) {
        let current = self.wheel_landing.clone();
        let previous = self.prev_wheel_landing.clone();
        *self = Self::reset_values();
        self.wheel_landing = current;
        self.prev_wheel_landing = previous;
    }
}
