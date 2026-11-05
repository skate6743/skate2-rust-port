//! Stock layout bindings consumed by TU3 Pumping Update/Calculate.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct PumpingSettings {
    /// physics_pumping +16/+48 (the graph's four header floats are skipped).
    pub pump_vs_speed: PointGraph<8>,
    /// +96/+128, also skipping the PointNegGraphData8 header.
    pub pump_vs_time: PointGraph<8>,
    /// +160/+192, +224/+256, +288/+320.
    pub min_crouch_vs_ground_angle: PointGraph<8>,
    pub compression_vs_ground_angle: PointGraph<8>,
    pub compression_vs_deck_angle: PointGraph<8>,
    /// +364 PumpEffectDamping; +372 MinChangeInCOMBeforePumping.
    pub height_change_damping: f32,
    pub minimum_height_change: f32,
    /// +376 MaxDeltaHeightAllowedPerFrame.
    pub maximum_height_change: f32,
    /// +380 CompressionGroundScalar; +384 CompressionDeckScalar.
    pub ground_compression_scale: f32,
    pub deck_compression_scale: f32,
    /// +392 AngularSpeedDamping.
    pub angular_speed_damping: f32,
}

/// Ground 82D37E74 passes ProcessedPhysIn+2548, then Update loads wrapper+4.
/// ProcessInput 82DB4420..446C selects that wrapper from the five player modes.
#[derive(Clone, Copy, Debug)]
pub struct PumpingMode {
    /// +12 Hash_9D1AEE3D7D8A4A7. Name describes its use, not a recovered name.
    pub maximum_absorption_per_second: f32,
    /// +16 Hash_D77AFD320B6241C5. Multiplied by the pumping timestep.
    pub maximum_acceleration_per_second: f32,
    /// +20 PumpEffectFactorAbsorption; +24 PumpEffectFactor.
    pub absorption_factor: f32,
    pub acceleration_factor: f32,
}
