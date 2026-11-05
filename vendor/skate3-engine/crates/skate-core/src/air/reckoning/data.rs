//! Additional persistent Reckoning fields and actual inputs for82D8DBD8.
use crate::{
    air::{body_flip::BodyFlipSettings, body_spin::BodySpinSettings},
    physics::skeleton_animation_record::{AnimationPartTransform, IDENTITY},
    point_graph::PointGraph,
};
#[derive(Clone, Debug, PartialEq)]
pub struct AirState {
    ///1568/1572, shared with Ground entry and PhysicsAir exit.
    pub spin_angle: f32,
    pub spin_speed: f32,
    ///1580 is cleared by UpdateAirStates (1576 lives in ReckoningFrames).
    pub secondary_lean_angle: f32,
    pub flip_angle: f32,
    pub flip_speed: f32,
    pub flip_requested_speed: f32,
    ///1072; final combined1008 has one owner, ReckoningFrames.body_flip.
    pub spin_transform: AnimationPartTransform,
    ///1232 and1596; their native trajectory/entry producers remain the caller.
    pub flip_axis: [f32; 4],
    pub flip_active: bool,
    ///1597, BeginBodyFlip82D8EC60 and ResetBodyFlip82D8EAA4.
    pub flip_side: bool,
}
impl AirState {
    ///82D33178: scalar zeros, identity1072, world-X axis and inactive flip.
    pub fn new() -> Self {
        Self {
            spin_angle: 0.0,
            spin_speed: 0.0,
            secondary_lean_angle: 0.0,
            flip_angle: 0.0,
            flip_speed: 0.0,
            flip_requested_speed: 0.0,
            spin_transform: IDENTITY,
            flip_axis: [1.0, 0.0, 0.0, 0.0],
            flip_active: false,
            flip_side: false,
        }
    }
    ///Ground entry82D37538/PhysicsAir exit82D346A0 reset exactly this pair.
    pub fn reset_spin(&mut self) {
        self.spin_angle = 0.0;
        self.spin_speed = 0.0;
    }
}
pub struct Settings {
    pub ground_normal_smoothing: [f32; 4],
    ///PointNegGraphData8 layout336; caller directly evaluates its X/Y352/384.
    pub max_up_angle_delta: PointGraph<8>,
    pub tilt_vs_rotation: PointGraph<8>,
    pub tilt_vs_slope: PointGraph<8>,
    pub body_spin: BodySpinSettings,
    pub body_flip: BodyFlipSettings,
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub landing_normal: [f32; 4],
    pub normal_blend: f32,
    pub target_spin: f32,
    pub flip_request: f32,
    ///Processed752, from the actual Skeleton animation COM-to-deck producer.
    pub com_to_deck: [f32; 4],
    pub timestep: f32,
    ///Processed2812, live PhysBodySpin animation attribute.
    pub physical_body_spin: f32,
    ///Processed2644, produced by Skeleton82BDD008; used only when2484bit15 is set.
    pub grind_adjusted_body_spin: f32,
    pub additive_spin: bool,
    pub direct_spin: bool,
    pub reverse_stance: bool,
    ///Actual selected physics_mode layout108 and28.
    pub easy_body_spins: bool,
    pub perfect_body_flips: bool,
}
