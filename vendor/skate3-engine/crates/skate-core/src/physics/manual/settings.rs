//! physics_manual stock layout at global830CFDA4+200, wrapper+4.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct ManualGains {
    pub proportional: f32,
    pub integral: f32,
    pub derivative: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ManualSettings {
    /// NoiseVsSpeed has a 16-byte header; X/Y arrays start at +96/+128.
    pub noise_vs_speed: PointGraph<8>,
    /// +160 TorqueScalarWithNoContact; applies to emitted displacement, not state.
    pub torque_scale_without_contact: f32,
    /// +164 TorqueBleedOffWithNoContact, +172 StartTorqueScalar.
    pub torque_bleed_without_contact: f32,
    pub start_torque_scale: f32,
    /// +184 ProceduralNoiseScalar, +188 ProceduralNoiseFreq.
    pub procedural_noise_scale: f32,
    pub procedural_noise_frequency: f32,
    /// +200/204/208 PowerScalar_P/I/D; +232/236/240 ManualScalar_P/I/D.
    pub powerslide: ManualGains,
    pub manual: ManualGains,
    /// +216 MaxTiltAngle (degrees); +224 MaxAngleDiff (native radians).
    pub maximum_tilt_degrees: f32,
    pub maximum_angle_error: f32,
    /// +244 D_Scalar_Limit, +248 BrakeTiltAngle (degrees), +252 AnimationNoiseScalar.
    pub derivative_limit: f32,
    pub brake_tilt_degrees: f32,
    pub animation_noise_scale: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ManualMode {
    /// physics_mode+32 Hash_84CB2F459F3FC811, compared to abs(local ground
    /// velocity Z). Legacy Rust field name; the source reads Processed416,
    /// not the angular velocity at720. Original stock name remains unresolved.
    pub correction_angular_speed_threshold: f32,
    /// physics_mode+36 Hash_F8CBC0F5FEF2240E, read as a byte.
    pub corrective_force_enabled: bool,
}
