//! Completed body/animation observations for the source footplant candidate.
//! The manager owns the source calculation; this record owns no controller,
//! bodies, clocks, queries or solver. Either native character can supply it.
use super::V;
use skate3_core::animation::output::NativeMatrix;

pub(crate) struct CandidatePose {
    pub inverse_effective: NativeMatrix,
    pub physical_toes: [V; 2],
    pub physical_com: V,
    pub animation_com: V,
    pub animation_toes: [V; 2],
}

/// Source-calculated animation-space target and its two-frame collision gate.
/// The physical owner applies this to its actual foot IK and body volumes.
pub(crate) struct PoseTarget {
    pub toe: usize,
    pub animation_position: V,
    pub blend: f32,
    pub disable_collision: bool,
}

/// Source footplant state's current path and optional release velocity.
pub(crate) struct GroundStep {
    pub position: V,
    pub anchor: V,
    pub toe: usize,
    pub release_velocity: Option<V>,
}
