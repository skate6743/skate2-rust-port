//! Mandatory engine-facing effects used by TU3 PhysicsAir.

use super::{
    data::{AirBoardForce, PhysicsAirReckoningFields, Vector4},
    jump_velocity::PhysicsAirMath,
};

/// Mutations of the 272-byte trajectory launch record recovered at
/// `0x82D33448`, `0x82BE33D0`, and `0x82D34570`.
pub trait PhysicsAirLaunchInfo {
    fn set_start_velocity(&mut self, velocity: Vector4);
    fn centre_of_mass_animation_position(&self) -> Vector4;
    fn set_trajectory_start_position_override(&mut self, position: Vector4);
    fn set_board_position_override(&mut self, position: Vector4);
    fn cone_angles(&self) -> [f32; 2];
    fn set_cone_angles(&mut self, angles: [f32; 2]);
    fn set_player_jumped(&mut self, jumped: bool);
    fn set_use_trajectory_start_position_override(&mut self, enabled: bool);
    fn set_trajectory_count(&mut self, count: u16);
}

/// Every external call/read made by the recovered PhysicsAir methods. There is
/// deliberately no no-op/default implementation: an engine adapter must bind
/// each stock-owned object before this state can be scheduled.
pub trait PhysicsAirRuntime: PhysicsAirMath {
    type LaunchInfo: PhysicsAirLaunchInfo;

    fn set_air_collision_update_enabled(&mut self, enabled: bool);
    fn set_skeleton_collision_state(&mut self, state: u32);
    fn set_skeleton_inverse_kinematics_enabled(&mut self, enabled: bool);
    fn enable_board_angular_drive_only(&mut self);
    fn set_footplant_flag_240(&mut self, value: bool);
    fn reset_footplants(&mut self);
    fn board_transform_height(&mut self) -> f32;
    fn request_skeleton_heading_update(&mut self);

    fn update_air_collision(&mut self);
    fn trajectory_query_just_started(&self) -> bool;
    fn construct_trajectory_launch_info(&mut self) -> Self::LaunchInfo;
    fn fill_skeleton_launch_info(&mut self, info: &mut Self::LaunchInfo);
    fn launch_trajectory(&mut self, info: &Self::LaunchInfo);
    fn update_trajectory_selector(&mut self);
    fn selector_landing_normal(&self) -> Option<Vector4>;
    fn selector_centre_of_mass_trajectory_ready(&self) -> bool;

    /// `AngleBetweenVectors` (`0x8296EBB0`), including its unresolved VMX
    /// normalization behavior. The caller performs the recovered angle wrap.
    fn angle_between_vectors(&mut self, left: Vector4, right: Vector4) -> f32;
    ///Return the completed canonical fields: UpdateSkateboard reads the newly
    ///filtered collision normal1216, after82D8DBD8, not the entry snapshot.
    fn update_reckoning_air_states(
        &mut self,
        landing_normal: Vector4,
        normal_blend: f32,
        body_spin: f32,
        body_flip: f32,
    ) -> PhysicsAirReckoningFields;
    fn update_known_air_skeleton(&mut self, trajectory_position: Vector4);
    fn update_animated_skateboard_skeleton(&mut self, argument: bool);

    fn update_board_steering_tilt(&mut self, tilt: f32);
    /// `Toolkit_CalcCollisionForce` (`0x82D944E8`). The adapter supplies its
    /// bound settings and complete live ProcessedPhysIn; PhysicsAir only passes
    /// the reference normal and receives the point-force record.
    fn calculate_air_collision_force(
        &mut self,
        reference_normal: Vector4,
        force: &mut AirBoardForce,
    ) -> bool;
    fn enqueue_board_force(&mut self, tag: u32, force: AirBoardForce);
    fn set_board_velocity(&mut self, velocity: Vector4);
    fn enable_skateboard_error_on_skeleton(&mut self);

    fn board_body_velocity(&mut self) -> Vector4;
    fn check_for_air_wipeout(&mut self, argument: bool);
}
