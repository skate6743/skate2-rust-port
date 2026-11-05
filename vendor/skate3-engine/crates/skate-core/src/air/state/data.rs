//! Persistent fields and direct inputs used by TU3 `PhysState_PhysicsAir`.

use crate::point_graph::PointGraph;

pub type Vector4 = [f32; 4];

/// Native trajectory storage at PhysicsAir+80.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirTrajectory {
    pub position: Vector4,
    pub velocity: Vector4,
    pub acceleration: Vector4,
    /// Native trajectory +48. Both recovered initializers write -1.0.
    pub scalar_48: f32,
}

/// Fields owned by the TU3 PhysicsAir object. Enter preserves prior trajectory
/// storage when COM mode is not selected; construct once and retain this owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsAirState {
    /// PhysicsAir+80.
    pub centre_of_mass_trajectory: AirTrajectory,
    /// PhysicsAir+144.
    pub landing_normal: Vector4,
    /// PhysicsAir+160.
    pub time_in_state: f32,
    /// PhysicsAir+164/+168.
    pub start_y: f32,
    pub max_y: f32,
    /// PhysicsAir+172/+173/+174.
    pub reached_apex: bool,
    pub use_centre_of_mass_velocity: bool,
    /// The producer is selector+9662. A more specific stock name is unresolved.
    pub selector_latch_174: bool,
    /// PhysicsAir+176, decremented as a signed wrapping integer.
    pub trajectory_query_countdown: i32,
}

impl Default for PhysicsAirState {
    ///Original constructor82D341D0: zero vectors80/96/112, -1 at128,
    ///up144, zero160/164/168 and172..176. Entry is a separate operation.
    fn default() -> Self {
        Self {
            centre_of_mass_trajectory: AirTrajectory {
                position: [0.0; 4],
                velocity: [0.0; 4],
                acceleration: [0.0; 4],
                scalar_48: -1.0,
            },
            landing_normal: [0.0, 1.0, 0.0, 0.0],
            time_in_state: 0.0,
            start_y: 0.0,
            max_y: 0.0,
            reached_apex: false,
            use_centre_of_mass_velocity: false,
            selector_latch_174: false,
            trajectory_query_countdown: 0,
        }
    }
}

/// Direct ProcessedPhysIn values consumed by the recovered methods.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsAirFrame {
    pub current_velocity_400: Vector4,
    /// Y lane of the all-states ground position, read directly at +500.
    pub ground_position_y_500: f32,
    pub trajectory_position_592: Vector4,
    pub trajectory_velocity_608: Vector4,
    pub jump_velocity_848: Vector4,
    pub flags_2468: u32,
    pub previous_physics_state_2504: i32,
    pub previous_physics_category_2516: i32,
    pub frames_since_jump_correction_2576: i32,
    pub delta_time_2604: f32,
    pub body_spin_input_2640: f32,
    pub gravity_y_2648: f32,
    pub state_timer_2664: f32,
}

/// Cached collection values read at PhysicsAir settings +320..+444.
/// No retail defaults are embedded; the stock vault adapter must provide them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsAirSettings {
    /// PointGraph x at +320 and y at +352.
    pub body_spin_over_time_320: PointGraph<8>,
    pub landing_normal_blend_388: f32,
    pub body_spin_scale_428: f32,
    pub landing_normal_angle_limit_444: f32,
}

/// Reckoning fields read directly by PhysicsAir, apart from the full
/// `Reckoning::UpdateAirStates` effect exposed by the runtime interface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsAirReckoningFields {
    pub current_landing_normal_1152: Vector4,
    pub collision_reference_normal_1216: Vector4,
    pub body_spin_angle_1568: f32,
    pub body_spin_speed_1572: f32,
}

/// Native zero-initialized point-force record passed to `0x82D944E8`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirBoardForce {
    pub force_world: Vector4,
    pub point_board_local: Vector4,
}

impl AirBoardForce {
    pub const ZERO: Self = Self {
        force_world: [0.0; 4],
        point_board_local: [0.0; 4],
    };
}

/// Writes performed by `FillPhysOut` (`0x82D34E90`). `scalar_184_write` is
/// `None` when native code leaves the existing PhysOut field untouched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsAirOutput {
    pub is_at_apex: bool,
    pub jump_height: f32,
    pub landing_normal: Vector4,
    pub scalar_184_write: Option<f32>,
}
