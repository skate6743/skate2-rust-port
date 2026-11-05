//! Original S3 trajectory grind acquisition and typed retained selection.
mod admission;
mod candidate;
use super::grind_surface::LandingOrientation;
use super::{Prediction, Trajectory, math::*};
use crate::{physics::grind_contact::Primitive, point_graph::PointGraph};
pub use admission::{admitted_displacement, apply_admitted_target};
pub use candidate::{
    consider_grind_primitive, descending_plane_time, take_best_grind, trajectory_box_filter,
};

#[derive(Clone, Copy, Debug)]
pub struct GrindTrajectoryCandidate {
    pub point: Vector,
    pub trajectory_point: Vector,
    pub direction: Vector,
    pub approach: Vector,
    pub distance: f32,
    pub time: f32,
    pub angle: f32,
    pub frame: i32,
    pub primitive: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct GrindSurfaceEvidence {
    pub kind: u32,
    pub side: Vector,
}
#[derive(Clone, Debug)]
pub struct GrindAssistLimits {
    pub lock_distance: f32,
    pub max_speed_squared_ledge: f32,
    pub max_speed_squared_rail: f32,
    pub max_downward_speed: f32,
    pub ledge_scalars: [f32; 4],
    pub tip_scalar: f32,
    pub maximum_adjust_angle: f32,
    pub deck_dimensions: [f32; 2],
}
#[derive(Clone, Copy, Debug)]
pub struct GrindTarget {
    pub edge: Primitive,
    pub provider_index: usize,
    pub primitive_flags: u32,
    pub orientation: LandingOrientation,
    pub point: Vector,
    ///Selector2896, distinct from landing contact normal.
    pub vertical_normal: Vector,
}
impl GrindTarget {
    pub fn air_target(self) -> crate::physics::grind_air::Target {
        crate::physics::grind_air::Target {
            edge: self.edge,
            primitive_flags: self.primitive_flags,
            orientation: self.orientation,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct GrindEvaluation {
    pub target: Option<GrindTarget>,
    pub score: f32,
    pub distance: f32,
    ///9663/9664 can be written even when candidate admission fails.
    pub effective_lock_distance: Option<f32>,
    pub penalty_domain: f32,
}
impl GrindEvaluation {
    pub fn penalty_input(self) -> f32 {
        if let Some(distance) = self.effective_lock_distance {
            let denominator = distance + f32::from_bits(0x3e19_999a);
            if denominator < self.penalty_domain {
                return (self.penalty_domain / denominator) * self.distance;
            }
        }
        self.distance
    }
}
