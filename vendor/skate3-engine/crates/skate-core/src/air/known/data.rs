//! Persistent state and direct inputs for TU3 `PhysState_KnownAir`.

use crate::point_graph::PointGraph;

pub type Vector4 = [f32; 4];

/// The 64-byte trajectory copied by `FillPhysOut` at `0x82D36A78`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirTrajectory {
    pub position: Vector4,
    pub velocity: Vector4,
    pub acceleration: Vector4,
    pub scalar_48: f32,
    pub word_52: u32,
    pub word_56: u32,
    pub word_60: u32,
}

/// Winning trajectory result at selector+1680. The collision frame lies at
/// result+128, outside the copied 64-byte trajectory at result+144.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirPrediction {
    pub trajectory: KnownAirTrajectory,
    /// Winning PredictionResults+48, distinct from request horizon at +192.
    pub collision_time_48: f32,
    pub collision_frame_128: i32,
}

/// Fields owned by the KnownAir object at native offsets +64..+216.
/// Construction remains caller-owned because Enter does not initialize every
/// field before `InitTrajectoryInfo` and the selector helpers run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirState {
    pub landing_normal_64: Vector4,
    pub landing_heading_80: Vector4,
    pub trajectory_apex_96: Vector4,
    pub collision_position_112: Vector4,
    pub selector_vector_128: Vector4,
    pub trajectory_follow_offset_144: Vector4,
    pub target_com_position_160: Vector4,
    pub collision_normal_speed_176: f32,
    pub time_in_state_180: f32,
    pub start_y_184: f32,
    pub max_y_188: f32,
    pub com_max_y_192: f32,
    pub collision_time_196: f32,
    pub time_to_apex_200: f32,
    pub body_flip_target_speed_204: f32,
    pub reached_apex_208: bool,
    pub landing_heading_valid_209: bool,
    pub start_flipped_210: bool,
    pub body_flipping_211: bool,
    pub grind_air_adjust_activated_212: bool,
    pub targeting_grind_213: bool,
    pub trajectory_index_216: i32,
}

/// Direct ProcessedPhysIn fields consumed by KnownAir.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirFrame {
    pub start_flip_reference_96: Vector4,
    pub alternate_head_target_112: Vector4,
    pub velocity_400: Vector4,
    pub ground_normal_464: Vector4,
    pub start_height_484: f32,
    pub skater_up_544: Vector4,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub flags_2488: u32,
    pub next_physics_state_2500: i32,
    pub delta_time_2604: f32,
    pub forward_speed_2612: f32,
    pub body_spin_input_2640: f32,
    pub selector_landing_normal_2656: Vector4,
}

/// Cached `physics_airstates/default.xml` values used by this state. Values
/// must come from the stock collection adapter; no retail defaults live here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirSettings {
    /// Settings +160; x at +176, y at +208.
    pub max_heading_adjust_vs_up_y_160: PointGraph<8>,
    /// Settings +240; x at +256, y at +288.
    pub landing_speed_scalar_vs_ground_normal_y_240: PointGraph<8>,
    /// Attribute hash `AA79C93673533C5B`; its recovered name remains unknown.
    pub flip_start_collision_time_vs_normal_y: PointGraph<8>,
    pub trajectory_error_blend_away_time_384: f32,
    pub min_target_heading_velocity_420: f32,
    pub min_auto_body_speed_424: f32,
    pub max_spin_speed_428: f32,
    pub frames_for_grind_air_assist_436: f32,
    pub body_flip_min_grab_time_fraction_456: f32,
    /// `physics_reckoning/FlipScalar`, hash `978CE73B0EFE76C8`.
    pub flip_scalar: f32,
}

/// Fields read through ProcessedPhysIn+2548 and its selected physics mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirModeSettings {
    pub upside_down_falling_wipeout_enabled_2: bool,
    pub perfect_body_flips_28: bool,
    pub body_spin_speed_limit_56: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirWipeoutSettings {
    pub air_falling_min_up_y_260: f32,
    pub air_falling_max_angle_264: f32,
}

/// Direct Reckoning fields read or cleared by KnownAir.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirReckoningFields {
    pub landing_normal_1152: Vector4,
    pub heading_axis_1200: Vector4,
    pub body_spin_angle_1568: f32,
    pub body_spin_speed_1572: f32,
}

/// Direct writes to the wipeout request object at +35, +116, and +200.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirWipeoutRequest {
    pub requested_35: bool,
    pub scalar_116: f32,
    pub counter_200: u32,
}

/// Six skeleton bodies disabled, in this exact native order, while the grind
/// air adjustment owns the feet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrindFootBody {
    Body3152,
    Body3156,
    Body3160,
    Body3168,
    Body3172,
    Body3176,
}

/// Trajectory packet passed to the three recovered footplant updates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirFootplantInput {
    pub trajectory: KnownAirTrajectory,
    pub remaining_collision_time: f32,
    pub collision_position: Vector4,
    pub landing_normal: Vector4,
}

/// Native transform at Reckoning+816, interpreted only by the required math
/// adapter because Xenon lane order remains part of that contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AffineTransform {
    pub vectors: [Vector4; 4],
}

/// Stable inputs/outputs around the unresolved VMX geometry in `0x82D35998`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RestoreVelocityGeometry {
    pub tangential_velocity: Vector4,
    pub landing_speed_curve_input: f32,
    /// Source clamped to [0, 1] by the two native `fsel` operations.
    pub landing_speed_blend_source: f32,
}

/// Fields written by `FillPhysOut` (`0x82D36880`). The two conditional fields
/// retain their previous value when flags+2468 bit 3 is clear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownAirOutput {
    pub trajectory_apex_0: Vector4,
    pub collision_position_16: Vector4,
    pub landing_normal_32: Vector4,
    pub selector_vector_48: Vector4,
    pub trajectory_position_64: Vector4,
    pub landing_heading_80: Vector4,
    pub selector_com_position_96: Vector4,
    pub landing_normal_copy_144: Vector4,
    pub locked_trajectory_velocity_160: Vector4,
    pub time_in_state_176: f32,
    pub collision_time_180: f32,
    pub time_until_collision_184: f32,
    pub collision_normal_speed_188: f32,
    pub time_to_apex_196: f32,
    pub jump_height_200: f32,
    pub trajectory_index_220: i32,
    pub selected_trajectory_240: KnownAirTrajectory,
    pub trajectory_plane_samples_336: [f32; 25],
    pub reached_apex_436: bool,
    pub known_air_valid_437: bool,
    pub locked_trajectory_velocity_valid_452: bool,
}
