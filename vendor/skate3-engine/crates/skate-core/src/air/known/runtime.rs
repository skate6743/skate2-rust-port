//! Mandatory engine and Xenon-math boundaries for TU3 KnownAir.

use super::data::{
    AffineTransform, GrindFootBody, KnownAirFootplantInput, KnownAirPrediction, KnownAirTrajectory,
    RestoreVelocityGeometry, Vector4,
};

/// Xenon operations whose exact estimate/refinement, lane, or angle behavior
/// is observable in the recovered routines. There is deliberately no default
/// implementation or scalar substitute.
pub trait KnownAirMath {
    /// `vmsum3fp128` including its native rounding and lane behavior.
    fn dot3(&mut self, left: Vector4, right: Vector4) -> f32;
    /// The `vrefp` reciprocal refinement sequence used for finite differences.
    fn divide_vector_by_scalar(&mut self, vector: Vector4, scalar: f32) -> Vector4;
    /// The two `vrsqrtefp128` refinements, zero selection, and returned length.
    fn normalize_safe_with_length(&mut self, vector: Vector4) -> (Vector4, f32);
    /// `0x8286CD88`, including its native orientation and degeneracy behavior.
    fn signed_angle_about_axis(&mut self, from: Vector4, to: Vector4, axis: Vector4) -> f32;
    /// `0x8258DB98` exactly; callers retain every observed invocation.
    fn wrap_angle_8258db98(&mut self, angle: f32) -> f32;
    /// VMX geometry at `0x82D35A0C..0x82D35B78`. The surrounding trajectory
    /// evaluation, curve evaluation, blend, board write, and speed publication
    /// remain explicit in readable Rust.
    fn restore_velocity_geometry_82d35998(
        &mut self,
        trajectory_velocity: Vector4,
        ground_normal: Vector4,
    ) -> RestoreVelocityGeometry;
    /// Affine point transform at `0x82D36AE4..0x82D36B30`.
    fn transform_point_82d36880(&mut self, transform: AffineTransform, point: Vector4) -> Vector4;
}

/// Every external call and direct engine-owned object mutation performed by
/// the recovered KnownAir routines. Implementers must bind all methods.
pub trait KnownAirRuntime: KnownAirMath {
    fn enable_board_angular_drive_only(&mut self);
    fn set_air_collision_update_enabled(&mut self, enabled: bool);
    fn set_skeleton_collision_state(&mut self, state: u32);
    fn set_skeleton_physics_truck_tilt_enabled(&mut self, enabled: bool);
    fn set_skeleton_inverse_kinematics_enabled(&mut self, enabled: bool);
    fn reset_reckoning_flipping(&mut self);
    fn set_footplant_flag_240(&mut self, value: bool);
    fn reset_footplants(&mut self);
    fn board_transform_height(&mut self) -> f32;
    fn skeleton_com_position(&mut self) -> Vector4;

    fn selector_targeting_grind(&self) -> bool;
    fn start_grind_air_adjust_from_selector(&mut self);
    fn selector_closest_trajectory_point(
        &mut self,
        skeleton_com: Vector4,
        zero: Vector4,
    ) -> (i32, Vector4);
    fn selected_prediction(&self) -> KnownAirPrediction;
    /// Selector+2704, distinct from the winning result trajectory at +1680.
    fn selector_trajectory_2704(&self) -> KnownAirTrajectory;
    fn selector_contact_position(&mut self) -> Vector4;
    fn selector_vector_2848(&self) -> Vector4;
    fn selector_com_position_2784(&self) -> Vector4;
    fn highest_trajectory_position(&mut self, trajectory: &KnownAirTrajectory) -> (Vector4, f32);
    fn selector_has_just_changed(&self) -> bool;
    fn reset_trajectory_selector(&mut self);

    fn begin_reckoning_body_flip(&mut self, side: bool);
    fn update_air_collision(&mut self);
    fn grind_air_adjust_started(&self) -> bool;
    fn set_grind_air_adjust_started(&mut self, value: bool);
    fn set_grind_air_adjust_activated(&mut self, value: bool);
    fn grind_air_adjust_adjusting(&self) -> bool;
    fn disable_grind_foot_collision(&mut self, body: GrindFootBody);
    fn update_reckoning_air_states(
        &mut self,
        landing_normal: Vector4,
        alignment_weight: f32,
        body_spin_speed: f32,
        body_flip_speed: f32,
    );
    fn update_known_air_skeleton(&mut self, target_com_position: Vector4);
    fn set_skeleton_add_skateboard_error(&mut self, value: bool);
    fn set_head_tracking_target(&mut self, target: Vector4, valid: bool);
    fn update_board_steering_tilt(&mut self, tilt: f32);

    fn shift_footplant_trajectory_to_index(
        &mut self,
        trajectory: &mut KnownAirTrajectory,
        index: i32,
    );
    fn update_footplant_prediction(&mut self, input: &KnownAirFootplantInput);
    fn update_footplant_lock(&mut self, input: &KnownAirFootplantInput);
    fn update_footplant_pose(&mut self, input: &KnownAirFootplantInput);

    fn board_body_velocity(&mut self) -> Vector4;
    fn set_board_velocity(&mut self, velocity: Vector4);
    fn board_forward_axis(&mut self) -> Vector4;
    fn check_for_air_wipeout(&mut self, use_com: bool);
    fn reckoning_com_transform_816(&self) -> AffineTransform;
}
