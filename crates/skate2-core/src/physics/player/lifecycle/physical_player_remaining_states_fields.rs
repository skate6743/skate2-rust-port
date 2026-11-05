//! Typed retained state constructor payloads; None means source has not written the field.
#![expect(
    clippy::option_option,
    reason = "outer Option tracks unwritten storage; inner Option preserves a written native NULL pointer"
)]
#![expect(
    clippy::struct_field_names,
    reason = "PDB member names preserve the distinction between state and retained manager"
)]
use super::{
    Arc, AttributeRuntimeInstance, AttributeRuntimeVault, GrabDescriptor, GrabSpline, JumpInfo,
    Matrix, MatrixSlot, OffboardTrajectoryResult, RetainedTrajectory, ScalarPid,
    StateConstructorBindings, StateOwnerLink, Vector, VectorPid,
};
/// PDB 48 bytes; preserves independently unwritten native fields.
pub(crate) struct SlideGround {
    pub(crate) base: StateConstructorBindings,
    pub(crate) target_speed: Option<f32>,
    pub(crate) board_is_flipped: Option<bool>,
    pub(crate) push_scalar: Option<f32>,
    pub(crate) damped_turn: Option<f32>,
}
impl SlideGround {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            target_speed: None,
            board_is_flipped: None,
            push_scalar: None,
            damped_turn: None,
        }
    }
}
/// PDB 96 bytes; preserves independently unwritten native fields.
pub(crate) struct RevertGround {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) pumping: Option<Option<StateOwnerLink>>,
    pub(crate) vel_start: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) revert_direction: Option<f32>,
    pub(crate) direction_scalar: Option<f32>,
    pub(crate) time_in_state: Option<f32>,
    pub(crate) is_reverting: Option<bool>,
    pub(crate) has_started: Option<bool>,
}
impl RevertGround {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            pumping: None,
            vel_start: None,
            revert_direction: None,
            direction_scalar: None,
            time_in_state: None,
            is_reverting: None,
            has_started: None,
        }
    }
}
/// PDB 96 bytes; preserves independently unwritten native fields.
pub(crate) struct GroundAnimation {
    pub(crate) base: StateConstructorBindings,
    pub(crate) jump_info: Option<JumpInfo>,
    pub(crate) jumped_this_frame: Option<bool>,
    pub(crate) jump_vel: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
}
impl GroundAnimation {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            jump_info: None,
            jumped_this_frame: None,
            jump_vel: None,
        }
    }
}
/// PDB 224 bytes; preserves independently unwritten native fields.
pub(crate) struct KnownAir {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attributes: Option<AttributeRuntimeInstance>,
    pub(crate) grind_air_adjust: Option<Option<StateOwnerLink>>,
    pub(crate) foot_plant_manager: Option<Option<StateOwnerLink>>,
    pub(crate) landing_normal: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) landing_heading: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) apex_pos: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) coll_pos: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) start_pos: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) trajectory_error: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) target_com_pos: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) coll_speed: Option<f32>,
    pub(crate) time_in_state: Option<f32>,
    pub(crate) start_y: Option<f32>,
    pub(crate) max_y: Option<f32>,
    pub(crate) com_max_y: Option<f32>,
    pub(crate) coll_time: Option<f32>,
    pub(crate) time_to_apex: Option<f32>,
    pub(crate) body_flip_speed_target: Option<f32>,
    pub(crate) reached_apex: Option<bool>,
    pub(crate) landing_heading_valid: Option<bool>,
    pub(crate) start_flipped: Option<bool>,
    pub(crate) body_flipping: Option<bool>,
    pub(crate) grind_air_adjust_activated: Option<bool>,
    pub(crate) targeting_grind: Option<bool>,
    pub(crate) traj_index: Option<i32>,
}
impl KnownAir {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attributes: None,
            grind_air_adjust: None,
            foot_plant_manager: None,
            landing_normal: None,
            landing_heading: None,
            apex_pos: None,
            coll_pos: None,
            start_pos: None,
            trajectory_error: None,
            target_com_pos: None,
            coll_speed: None,
            time_in_state: None,
            start_y: None,
            max_y: None,
            com_max_y: None,
            coll_time: None,
            time_to_apex: None,
            body_flip_speed_target: None,
            reached_apex: None,
            landing_heading_valid: None,
            start_flipped: None,
            body_flipping: None,
            grind_air_adjust_activated: None,
            targeting_grind: None,
            traj_index: None,
        }
    }
}
/// PDB 208 bytes; preserves independently unwritten native fields.
pub(crate) struct GrindCommon {
    pub(crate) base: StateConstructorBindings,
    pub(crate) grind_dir: Option<Vector>,
    pub(crate) grind_normal: Option<Vector>,
    pub(crate) grind_skew: Option<Vector>,
    pub(crate) leaving_grind: Option<bool>,
    pub(crate) grind_substate: Option<i32>,
    pub(crate) precontact_target_orientation: Option<Matrix>,
    pub(crate) already_jumped: Option<bool>,
    pub(crate) jumped_this_frame: Option<bool>,
    pub(crate) frames_in_grind: Option<i32>,
    pub(crate) inv_exit_frames: Option<i32>,
    pub(crate) crouch: Option<f32>,
    pub(crate) grind_jumper: Option<Option<StateOwnerLink>>,
    pub(crate) jump_vel: Option<Vector>,
}
impl GrindCommon {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            grind_dir: None,
            grind_normal: None,
            grind_skew: None,
            leaving_grind: None,
            grind_substate: None,
            precontact_target_orientation: None,
            already_jumped: None,
            jumped_this_frame: None,
            frames_in_grind: None,
            inv_exit_frames: None,
            crouch: None,
            grind_jumper: None,
            jump_vel: None,
        }
    }
}
/// PDB 64 bytes; preserves independently unwritten native fields.
pub(crate) struct Nonspecific {
    pub(crate) base: StateConstructorBindings,
    pub(crate) grind_jumper: Option<Option<StateOwnerLink>>,
    pub(crate) jumped_this_frame: Option<bool>,
    pub(crate) jump_vel: Option<Vector>,
}
impl Nonspecific {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            grind_jumper: None,
            jumped_this_frame: None,
            jump_vel: None,
        }
    }
}
/// PDB 112 bytes; preserves independently unwritten native fields.
pub(crate) struct Teleporting {
    pub(crate) base: StateConstructorBindings,
    pub(crate) reset_transform: Option<crate::skeleton_root_transforms::SkeletonRootMatrix>,
    pub(crate) phys_out_conditioners: Option<Option<StateOwnerLink>>,
    pub(crate) teleport_to_on_board: Option<i32>,
    pub(crate) data_acquired: Option<i32>,
    pub(crate) teleport_ready: Option<i32>,
}
impl Teleporting {
    /// Enter82d86928 clears the acquired/ready high bits and sets on-board.
    /// The reset transform remains the constructor's unwritten value.
    pub(crate) fn enter(&mut self) {
        self.teleport_to_on_board = Some(1);
        self.data_acquired = Some(0);
        self.teleport_ready = Some(0);
    }

    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            reset_transform: None,
            phys_out_conditioners: None,
            teleport_to_on_board: None,
            data_acquired: None,
            teleport_ready: None,
        }
    }
}
/// PDB 32 bytes; preserves independently unwritten native fields.
pub(crate) struct FollowPath {
    pub(crate) base: StateConstructorBindings,
}
impl FollowPath {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self { base }
    }
}
/// PDB 1312 bytes; preserves independently unwritten native fields.
pub(crate) struct Skitching {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) player_grab_spline: Option<Option<StateOwnerLink>>,
    pub(crate) grab_spline_last_space: Option<Matrix>,
    pub(crate) grab_spline_current_space: Option<Matrix>,
    pub(crate) grab_spline_future_space: Option<Matrix>,
    pub(crate) grab_spline_forward_step_transform: Option<Matrix>,
    pub(crate) grab_spline_to_physical_hand_space_transform: Option<Matrix>,
    pub(crate) grab_spline_to_ik_hand_space_transform: Option<Matrix>,
    pub(crate) grab_spline_anim_space_to_ik_hand_space_transform: Option<Matrix>,
    pub(crate) grab_spline_space: Option<MatrixSlot>,
    pub(crate) grab_spline_physical_hand_space: Option<MatrixSlot>,
    pub(crate) grab_spline_ik_hand_space: Option<MatrixSlot>,
    pub(crate) grab_spline_aim_space: Option<Matrix>,
    pub(crate) deck_pos: Option<Vector>,
    pub(crate) vector_to_deck_pos_on_aim_space: Option<Vector>,
    pub(crate) normalized_vector_to_deck_pos_on_aim_space_ground: Option<Vector>,
    pub(crate) vector_to_deck_pos_on_hand_space: Option<Vector>,
    pub(crate) normalized_vector_to_deck_pos_on_hand_space_ground: Option<Vector>,
    pub(crate) vector_to_deck_pos_on_curve_from_base: Option<Vector>,
    pub(crate) normalized_vector_to_deck_pos_on_curve_from_base_ground: Option<Vector>,
    pub(crate) grab_spline_rh_pos: Option<Vector>,
    pub(crate) grab_spline_lh_pos: Option<Vector>,
    pub(crate) grab_spline_follow_point_target: Option<Vector>,
    pub(crate) grab_spline_velocity: Option<Vector>,
    pub(crate) grab_spline_velocity_ground: Option<Vector>,
    pub(crate) normalized_vector_to_skitch_target_ground: Option<Vector>,
    pub(crate) ground_normal: Option<Vector>,
    pub(crate) grab_spline_speed_ground: Option<f32>,
    pub(crate) grab_spline_deck_offset: Option<f32>,
    pub(crate) grab_spline_deck_offset_speed: Option<f32>,
    pub(crate) grab_spline_false_deck_offset_speed_due_to_turning: Option<f32>,
    pub(crate) distance_to_deck_pos_on_curve_from_base_ground: Option<f32>,
    pub(crate) deck_dist: Option<f32>,
    pub(crate) deck_dist_speed: Option<f32>,
    pub(crate) deck_dist_on_aim_space: Option<f32>,
    pub(crate) grab_spline_follow_point_offset: Option<f32>,
    pub(crate) grab_spline_centrifugal_acceleration: Option<f32>,
    pub(crate) grab_spline_centrifugal_acceleration_sign: Option<f32>,
    pub(crate) grab_spline_turn_acceleration: Option<f32>,
    pub(crate) grab_spline_grip_acceleration: Option<f32>,
    pub(crate) grab_spline_auto_recentering_acceleration: Option<f32>,
    pub(crate) grab_spline_follow_point_acceleration: Option<f32>,
    pub(crate) grab_spline_desired_follow_point_velocity: Option<f32>,
    pub(crate) grab_spline_follow_point_velocity: Option<f32>,
    pub(crate) grab_spline_target_offset: Option<f32>,
    pub(crate) grab_spline_hand_target_offset: Option<f32>,
    pub(crate) grab_spline_hands_dist: Option<f32>,
    pub(crate) grab_spline_hands_dist_delta: Option<f32>,
    pub(crate) predicted_left_hand_extension: Option<f32>,
    pub(crate) predicted_right_hand_extension: Option<f32>,
    pub(crate) turn_scalar: Option<f32>,
    pub(crate) grab_spline_half_length_mag: Option<f32>,
    pub(crate) grab_spline_half_length_mag_with_edge_gap: Option<f32>,
    pub(crate) parametric_position: Option<f32>,
    pub(crate) deck_angle: Option<f32>,
    pub(crate) anim_deck_angle: Option<f32>,
    pub(crate) time_in_skitch_state: Option<f32>,
    pub(crate) desired_heading_angle: Option<f32>,
    pub(crate) contact_speed: Option<f32>,
    pub(crate) time_on_edge_hanging_on_for_dear_life: Option<f32>,
    pub(crate) time_hands_not_moving: Option<f32>,
    pub(crate) left_blend: Option<f32>,
    pub(crate) right_blend: Option<f32>,
    pub(crate) left_blend_target: Option<f32>,
    pub(crate) right_blend_target: Option<f32>,
    pub(crate) grip_hand: Option<i32>,
    pub(crate) anim_crouch: Option<f32>,
    pub(crate) time_grabbing: Option<f32>,
    pub(crate) centrifugal_force_state: Option<i32>,
    pub(crate) follow_point_mode: Option<i32>,
    pub(crate) follow_point_positive_side_clamped_time: Option<f32>,
    pub(crate) follow_point_negative_side_clamped_time: Option<f32>,
    pub(crate) delay_until_reskitchable: Option<f32>,
    pub(crate) delay_until_hands_off_matters: Option<f32>,
    pub(crate) grab_spline: Option<GrabSpline>,
    pub(crate) grab_spline_desc: Option<GrabDescriptor>,
    pub(crate) exit_count: Option<i32>,
    pub(crate) skitch_state: Option<i32>,
    pub(crate) left_hand_state: Option<i32>,
    pub(crate) right_hand_state: Option<i32>,
    pub(crate) last_left_hand_state: Option<i32>,
    pub(crate) last_right_hand_state: Option<i32>,
    pub(crate) grab_spline_target_offset_locked: Option<bool>,
    pub(crate) grab_spline_target_offset_active: Option<bool>,
    pub(crate) apply_side_force: Option<bool>,
    pub(crate) hands_moving: Option<bool>,
    pub(crate) force_exit: Option<bool>,
    pub(crate) push_forward_on_exit: Option<bool>,
    pub(crate) push_back_on_exit: Option<bool>,
    pub(crate) grab_spline_valid: Option<bool>,
    pub(crate) in_air: Option<bool>,
    pub(crate) skitch_steering_active: Option<bool>,
    pub(crate) first_eval: Option<bool>,
}
impl Skitching {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            player_grab_spline: None,
            grab_spline_last_space: None,
            grab_spline_current_space: None,
            grab_spline_future_space: None,
            grab_spline_forward_step_transform: None,
            grab_spline_to_physical_hand_space_transform: None,
            grab_spline_to_ik_hand_space_transform: None,
            grab_spline_anim_space_to_ik_hand_space_transform: None,
            grab_spline_space: None,
            grab_spline_physical_hand_space: None,
            grab_spline_ik_hand_space: None,
            grab_spline_aim_space: None,
            deck_pos: None,
            vector_to_deck_pos_on_aim_space: None,
            normalized_vector_to_deck_pos_on_aim_space_ground: None,
            vector_to_deck_pos_on_hand_space: None,
            normalized_vector_to_deck_pos_on_hand_space_ground: None,
            vector_to_deck_pos_on_curve_from_base: None,
            normalized_vector_to_deck_pos_on_curve_from_base_ground: None,
            grab_spline_rh_pos: None,
            grab_spline_lh_pos: None,
            grab_spline_follow_point_target: None,
            grab_spline_velocity: None,
            grab_spline_velocity_ground: None,
            normalized_vector_to_skitch_target_ground: None,
            ground_normal: None,
            grab_spline_speed_ground: None,
            grab_spline_deck_offset: None,
            grab_spline_deck_offset_speed: None,
            grab_spline_false_deck_offset_speed_due_to_turning: None,
            distance_to_deck_pos_on_curve_from_base_ground: None,
            deck_dist: None,
            deck_dist_speed: None,
            deck_dist_on_aim_space: None,
            grab_spline_follow_point_offset: None,
            grab_spline_centrifugal_acceleration: None,
            grab_spline_centrifugal_acceleration_sign: None,
            grab_spline_turn_acceleration: None,
            grab_spline_grip_acceleration: None,
            grab_spline_auto_recentering_acceleration: None,
            grab_spline_follow_point_acceleration: None,
            grab_spline_desired_follow_point_velocity: None,
            grab_spline_follow_point_velocity: None,
            grab_spline_target_offset: None,
            grab_spline_hand_target_offset: None,
            grab_spline_hands_dist: None,
            grab_spline_hands_dist_delta: None,
            predicted_left_hand_extension: None,
            predicted_right_hand_extension: None,
            turn_scalar: None,
            grab_spline_half_length_mag: None,
            grab_spline_half_length_mag_with_edge_gap: None,
            parametric_position: None,
            deck_angle: None,
            anim_deck_angle: None,
            time_in_skitch_state: None,
            desired_heading_angle: None,
            contact_speed: None,
            time_on_edge_hanging_on_for_dear_life: None,
            time_hands_not_moving: None,
            left_blend: None,
            right_blend: None,
            left_blend_target: None,
            right_blend_target: None,
            grip_hand: None,
            anim_crouch: None,
            time_grabbing: None,
            centrifugal_force_state: None,
            follow_point_mode: None,
            follow_point_positive_side_clamped_time: None,
            follow_point_negative_side_clamped_time: None,
            delay_until_reskitchable: None,
            delay_until_hands_off_matters: None,
            grab_spline: None,
            grab_spline_desc: None,
            exit_count: None,
            skitch_state: None,
            left_hand_state: None,
            right_hand_state: None,
            last_left_hand_state: None,
            last_right_hand_state: None,
            grab_spline_target_offset_locked: None,
            grab_spline_target_offset_active: None,
            apply_side_force: None,
            hands_moving: None,
            force_exit: None,
            push_forward_on_exit: None,
            push_back_on_exit: None,
            grab_spline_valid: None,
            in_air: None,
            skitch_steering_active: None,
            first_eval: None,
        }
    }
}
/// PDB 80 bytes; preserves independently unwritten native fields.
pub(crate) struct HandPlant {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) grab_spline_query_manager: Option<Option<StateOwnerLink>>,
    pub(crate) hand_plant_manager: Option<Option<StateOwnerLink>>,
    pub(crate) grab_object: Option<bool>,
    pub(crate) grab_spline_desc: Option<GrabDescriptor>,
    pub(crate) ground_blend: Option<f32>,
    pub(crate) time_to_apex: Option<f32>,
    pub(crate) hand_plant_state: Option<i32>,
}
impl HandPlant {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            grab_spline_query_manager: None,
            hand_plant_manager: None,
            grab_object: None,
            grab_spline_desc: None,
            ground_blend: None,
            time_to_apex: None,
            hand_plant_state: None,
        }
    }
}
/// PDB 52 bytes; preserves independently unwritten native fields.
pub(crate) struct FootPlant {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) foot_plant_manager: Option<Option<StateOwnerLink>>,
}
impl FootPlant {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            foot_plant_manager: None,
        }
    }
}
/// PDB 96 bytes; preserves independently unwritten native fields.
pub(crate) struct Boneless {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) landing_toe_index: Option<i32>,
    pub(crate) landing_toe_pos: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) right_leg: Option<bool>,
}
impl Boneless {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            landing_toe_index: None,
            landing_toe_pos: None,
            right_leg: None,
        }
    }
}
/// PDB 144 bytes; preserves independently unwritten native fields.
pub(crate) struct OffBoard {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) player_grab_spline: Option<Option<StateOwnerLink>>,
    pub(crate) off_board_feet_ik: Option<Option<StateOwnerLink>>,
    pub(crate) skateboard_controller: Option<Option<StateOwnerLink>>,
    pub(crate) offboard_location_test: Option<Option<StateOwnerLink>>,
    pub(crate) offboard_jumping_test: Option<Option<StateOwnerLink>>,
    pub(crate) last_collision_response_direction:
        Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    /// Native full-vector store; W may be unknown for attached-body COM history.
    pub(crate) starting_com_vel: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) last_collision_response_direction_valid: Option<bool>,
    pub(crate) grab_object: Option<bool>,
    pub(crate) is_locomotion_viable: Option<bool>,
    pub(crate) is_not_upright_entering_offboard: Option<bool>,
    pub(crate) is_moving_to_fast_entering_offboard: Option<bool>,
    pub(crate) grab_spline_desc: Option<GrabDescriptor>,
    pub(crate) ground_blend: Option<f32>,
    pub(crate) auto_turn: Option<f32>,
}
impl OffBoard {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            player_grab_spline: None,
            off_board_feet_ik: None,
            skateboard_controller: None,
            offboard_location_test: None,
            offboard_jumping_test: None,
            last_collision_response_direction: None,
            starting_com_vel: None,
            last_collision_response_direction_valid: None,
            grab_object: None,
            is_locomotion_viable: None,
            is_not_upright_entering_offboard: None,
            is_moving_to_fast_entering_offboard: None,
            grab_spline_desc: None,
            ground_blend: None,
            auto_turn: None,
        }
    }
}
/// PDB 92 bytes; preserves independently unwritten native fields.
pub(crate) struct OffBoardLocomotion {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) player_grab_spline: Option<Option<StateOwnerLink>>,
    pub(crate) off_board_feet_ik: Option<Option<StateOwnerLink>>,
    pub(crate) skateboard_controller: Option<Option<StateOwnerLink>>,
    pub(crate) landing_on_deck_manager: Option<Option<StateOwnerLink>>,
    pub(crate) grab_object: Option<bool>,
    pub(crate) grab_spline_desc: Option<GrabDescriptor>,
    pub(crate) ground_blend: Option<f32>,
    pub(crate) trying_to_land_on_board: Option<bool>,
    pub(crate) try_to_land_on_board_delay: Option<u32>,
    pub(crate) is_moving_to_fast_entering_locomotion: Option<bool>,
}
impl OffBoardLocomotion {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            player_grab_spline: None,
            off_board_feet_ik: None,
            skateboard_controller: None,
            landing_on_deck_manager: None,
            grab_object: None,
            grab_spline_desc: None,
            ground_blend: None,
            trying_to_land_on_board: None,
            try_to_land_on_board_delay: None,
            is_moving_to_fast_entering_locomotion: None,
        }
    }
}
/// PDB 1136 bytes; preserves independently unwritten native fields.
pub(crate) struct OffBoardPushing {
    pub(crate) base: StateConstructorBindings,
    pub(crate) player_grab_spline: Option<Option<StateOwnerLink>>,
    pub(crate) off_board_feet_ik: Option<Option<StateOwnerLink>>,
    pub(crate) skateboard_controller: Option<Option<StateOwnerLink>>,
    /// The state's own fields, which the Skate 3 physics drives without a Skate 2 player.
    pub(crate) core: PushingCore,
}
/// OffBoardPushing's fields besides its owner bindings.
pub(crate) struct PushingCore {
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) attribs_off_board: Option<AttributeRuntimeInstance>,
    pub(crate) previous_spline_hands_transform: Option<Matrix>,
    pub(crate) current_spline_hands_transform: Option<Matrix>,
    pub(crate) blended_spline_hands_transform: Option<Matrix>,
    pub(crate) reach_base: Option<Vector>,
    pub(crate) world_position_delta: Option<Vector>,
    pub(crate) last_error: Option<Vector>,
    pub(crate) spline_dir: Option<Vector>,
    pub(crate) applied_force: Option<Vector>,
    pub(crate) target_velocity: Option<Vector>,
    pub(crate) target_push_normal: Option<Vector>,
    pub(crate) last_push_normal: Option<Vector>,
    pub(crate) target_x_direction: Option<Vector>,
    pub(crate) hand_l_raw: Option<Vector>,
    pub(crate) hand_r_raw: Option<Vector>,
    pub(crate) previous_hand_left_offset: Option<Vector>,
    pub(crate) current_hand_left_offset: Option<Vector>,
    pub(crate) blended_hand_left_offset: Option<Vector>,
    pub(crate) previous_hand_right_offset: Option<Vector>,
    pub(crate) current_hand_right_offset: Option<Vector>,
    pub(crate) blended_hand_right_offset: Option<Vector>,
    pub(crate) hand_left_target: Option<Vector>,
    pub(crate) hand_right_target: Option<Vector>,
    pub(crate) hands_dir: Option<Vector>,
    pub(crate) approximate_skeleton_position: Option<Vector>,
    pub(crate) approximate_skeleton_speed: Option<Vector>,
    pub(crate) smoothed_force: Option<Vector>,
    pub(crate) skeleton_collision_offset: Option<Vector>,
    pub(crate) acceleration: Option<Vector>,
    pub(crate) grab_spline_desc: Option<GrabDescriptor>,
    pub(crate) grab_spline: Option<GrabSpline>,
    pub(crate) pid_object_vel: Option<VectorPid>,
    pub(crate) pid_object_angular_vel: Option<ScalarPid>,
    pub(crate) spline_length: Option<f32>,
    pub(crate) align_angle: Option<f32>,
    pub(crate) grip_offset_along_spline: Option<f32>,
    pub(crate) ik_blend_value: Option<f32>,
    pub(crate) anim_crouch: Option<f32>,
    pub(crate) time_grabbing_this_spline: Option<f32>,
    pub(crate) object_mass: Option<f32>,
    pub(crate) force_mass_scale: Option<f32>,
    pub(crate) object_y_tensor: Option<f32>,
    pub(crate) torque_scale: Option<f32>,
    pub(crate) max_rot_speed: Option<f32>,
    pub(crate) spline_blend_coeff: Option<f32>,
    pub(crate) spline_blend_adjust: Option<f32>,
    pub(crate) time_without_forces_requested: Option<f32>,
    pub(crate) time_in_partial_ragdoll: Option<f32>,
    pub(crate) world_grab_z: Option<f32>,
    pub(crate) world_grab_y: Option<f32>,
    pub(crate) delay_until_repushable: Option<f32>,
    pub(crate) emergency_exit: Option<bool>,
    pub(crate) grab_spline_swapped: Option<bool>,
    pub(crate) is_blending_spline: Option<bool>,
    pub(crate) collision_offset_active: Option<bool>,
    pub(crate) grab_object: Option<bool>,
    pub(crate) hand_ik_targets_valid: Option<bool>,
    pub(crate) hand_ik_targets_locked: Option<bool>,
}
impl std::ops::Deref for OffBoardPushing {
    type Target = PushingCore;
    fn deref(&self) -> &PushingCore {
        &self.core
    }
}
impl std::ops::DerefMut for OffBoardPushing {
    fn deref_mut(&mut self) -> &mut PushingCore {
        &mut self.core
    }
}
impl OffBoardPushing {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            player_grab_spline: None,
            off_board_feet_ik: None,
            skateboard_controller: None,
            core: PushingCore::unconstructed(),
        }
    }
}
impl PushingCore {
    /// Every field unwritten, as before the constructor runs.
    pub(crate) fn unconstructed() -> Self {
        Self {
            attribs: None,
            attribs_off_board: None,
            previous_spline_hands_transform: None,
            current_spline_hands_transform: None,
            blended_spline_hands_transform: None,
            reach_base: None,
            world_position_delta: None,
            last_error: None,
            spline_dir: None,
            applied_force: None,
            target_velocity: None,
            target_push_normal: None,
            last_push_normal: None,
            target_x_direction: None,
            hand_l_raw: None,
            hand_r_raw: None,
            previous_hand_left_offset: None,
            current_hand_left_offset: None,
            blended_hand_left_offset: None,
            previous_hand_right_offset: None,
            current_hand_right_offset: None,
            blended_hand_right_offset: None,
            hand_left_target: None,
            hand_right_target: None,
            hands_dir: None,
            approximate_skeleton_position: None,
            approximate_skeleton_speed: None,
            smoothed_force: None,
            skeleton_collision_offset: None,
            acceleration: None,
            grab_spline_desc: None,
            grab_spline: None,
            pid_object_vel: None,
            pid_object_angular_vel: None,
            spline_length: None,
            align_angle: None,
            grip_offset_along_spline: None,
            ik_blend_value: None,
            anim_crouch: None,
            time_grabbing_this_spline: None,
            object_mass: None,
            force_mass_scale: None,
            object_y_tensor: None,
            torque_scale: None,
            max_rot_speed: None,
            spline_blend_coeff: None,
            spline_blend_adjust: None,
            time_without_forces_requested: None,
            time_in_partial_ragdoll: None,
            world_grab_z: None,
            world_grab_y: None,
            delay_until_repushable: None,
            emergency_exit: None,
            grab_spline_swapped: None,
            is_blending_spline: None,
            collision_offset_active: None,
            grab_object: None,
            hand_ik_targets_valid: None,
            hand_ik_targets_locked: None,
        }
    }
}
/// PDB 92 bytes; preserves independently unwritten native fields.
pub(crate) struct LandingOnDeck {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) lod_attribs: Option<AttributeRuntimeInstance>,
    pub(crate) landing_on_deck_manager: Option<Option<StateOwnerLink>>,
    pub(crate) time_to_land: Option<f32>,
    pub(crate) heavy_landing_wipeout: Option<bool>,
    pub(crate) on_deck: Option<bool>,
    pub(crate) flip_board: Option<bool>,
    pub(crate) hippy_jumping: Option<bool>,
    pub(crate) just_hippy_jumped: Option<i32>,
    pub(crate) turn_speed: Option<f32>,
    pub(crate) turn_speed_adjusted: Option<f32>,
    pub(crate) transition_angle: Option<f32>,
}
impl LandingOnDeck {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            lod_attribs: None,
            landing_on_deck_manager: None,
            time_to_land: None,
            heavy_landing_wipeout: None,
            on_deck: None,
            flip_board: None,
            hippy_jumping: None,
            just_hippy_jumped: None,
            turn_speed: None,
            turn_speed_adjusted: None,
            transition_angle: None,
        }
    }
}
/// PDB 192 bytes; preserves independently unwritten native fields.
pub(crate) struct OffBoardAir {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) offboard_trajectory_result: Option<OffboardTrajectoryResult>,
    pub(crate) offboard_trajectory_selector: Option<Option<StateOwnerLink>>,
    pub(crate) off_board_feet_ik: Option<Option<StateOwnerLink>>,
    pub(crate) skateboard_controller: Option<Option<StateOwnerLink>>,
    pub(crate) landing_on_deck_manager: Option<Option<StateOwnerLink>>,
    pub(crate) body_spin: Option<f32>,
    pub(crate) body_spin_adjusted: Option<f32>,
    pub(crate) transition_angle: Option<f32>,
    pub(crate) current_angle: Option<f32>,
    pub(crate) air_frame: Option<i32>,
    pub(crate) has_landed: Option<bool>,
    pub(crate) request_wipeout: Option<bool>,
}
impl OffBoardAir {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            offboard_trajectory_result: None,
            offboard_trajectory_selector: None,
            off_board_feet_ik: None,
            skateboard_controller: None,
            landing_on_deck_manager: None,
            body_spin: None,
            body_spin_adjusted: None,
            transition_angle: None,
            current_angle: None,
            air_frame: None,
            has_landed: None,
            request_wipeout: None,
        }
    }
}
impl OffboardTrajectoryResult {
    /// Native `OffboardTrajectoryResult::Reset` at 0x82d88138.
    pub(crate) fn reset(&mut self) {
        self.frame = 0;
        self.time_to_land = 0.0;
        self.landing_valid = false;
        // Raw stores +0x20, +0x00, +0x10, +0x30, +0x40 after scalars.
        self.landing_normal = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        self.com_position = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        self.com_velocity = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        self.landing_velocity =
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        self.landing_position =
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
    }
}
/// PDB 176 bytes; preserves independently unwritten native fields.
pub(crate) struct OffBoardJump {
    pub(crate) base: StateConstructorBindings,
    pub(crate) attribs: Option<AttributeRuntimeInstance>,
    pub(crate) trajectory: Option<RetainedTrajectory>,
    pub(crate) off_board_feet_ik: Option<Option<StateOwnerLink>>,
    pub(crate) skateboard_controller: Option<Option<StateOwnerLink>>,
    pub(crate) landing_on_deck_manager: Option<Option<StateOwnerLink>>,
    pub(crate) offboard_jumping_test: Option<Option<StateOwnerLink>>,
    pub(crate) body_spin: Option<f32>,
    pub(crate) body_spin_adjusted: Option<f32>,
    pub(crate) transition_angle: Option<f32>,
    pub(crate) current_angle: Option<f32>,
    pub(crate) air_time: Option<f32>,
    pub(crate) total_air_time: Option<f32>,
    pub(crate) time_to_land: Option<f32>,
    pub(crate) com_height_on_landing: Option<f32>,
    pub(crate) has_landed: Option<bool>,
    pub(crate) is_leap_of_faith: Option<bool>,
}
impl OffBoardJump {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            base,
            attribs: None,
            trajectory: None,
            off_board_feet_ik: None,
            skateboard_controller: None,
            landing_on_deck_manager: None,
            offboard_jumping_test: None,
            body_spin: None,
            body_spin_adjusted: None,
            transition_angle: None,
            current_angle: None,
            air_time: None,
            total_air_time: None,
            time_to_land: None,
            com_height_on_landing: None,
            has_landed: None,
            is_leap_of_faith: None,
        }
    }
}

/// Boardslide PDB240 extends common208 with three constructor-unwritten fields.
pub(crate) struct GrindBoardslide {
    pub(crate) common: GrindCommon,
    pub(crate) wipeout_impulse: Option<Vector>,
    pub(crate) apply_wipeout_impulse: Option<bool>,
    pub(crate) preparing_to_jump: Option<bool>,
}
impl GrindBoardslide {
    pub(super) fn before_constructor(base: StateConstructorBindings) -> Self {
        Self {
            common: GrindCommon::before_constructor(base),
            wipeout_impulse: None,
            apply_wipeout_impulse: None,
            preparing_to_jump: None,
        }
    }
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.common.construct(vault);
    }
}

#[cfg(test)]
mod tests {
    use super::OffboardTrajectoryResult;

    #[test]
    fn offboard_trajectory_reset_clears_all_native_fields() {
        let mut result = OffboardTrajectoryResult {
            com_position: crate::skeleton_root_transforms::SkeletonRootVector::known([
                1.0, 2.0, 3.0, 4.0,
            ]),
            com_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known([
                5.0, 6.0, 7.0, 8.0,
            ]),
            landing_normal: crate::skeleton_root_transforms::SkeletonRootVector::known([
                9.0, 10.0, 11.0, 12.0,
            ]),
            landing_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known([
                13.0, 14.0, 15.0, 16.0,
            ]),
            landing_position: crate::skeleton_root_transforms::SkeletonRootVector::known([
                17.0, 18.0, 19.0, 20.0,
            ]),
            time_to_land: 21.0,
            frame: 22,
            landing_valid: true,
        };

        result.reset();

        assert_eq!(
            result.com_position.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            result.com_velocity.known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            result
                .landing_normal
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            result
                .landing_velocity
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(
            result
                .landing_position
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(result.time_to_land.to_bits(), 0);
        assert_eq!(result.frame, 0);
        assert!(!result.landing_valid);
    }
}
