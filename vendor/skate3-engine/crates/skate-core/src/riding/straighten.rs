//! Complete TU3 Toolkit_CalcStraightenOutTorque 82D92B60.
use super::vector::{clamp, cross, dot3};
use crate::point_graph::PointGraph;
#[derive(Clone)]
pub struct StraightenSettings {
    pub time_response: PointGraph<16>, // settings +768/+832
    pub opposite_turn_limit: f32,      // +896
    pub time_scalar: f32,              // +900
    pub heading_time_limit: f32,       // surface +236
    pub strength: f32,                 // surface +232
}
pub struct StraightenInput {
    pub heading_time: f32,
    pub scalar_2764: f32,
    pub turn: f32,          // +2676
    pub forward: [f32; 4],  // +352
    pub velocity: [f32; 4], // +400
    pub normal: [f32; 4],   // +464
}
pub fn calculate(s: &StraightenSettings, i: &StraightenInput) -> [f32; 4] {
    let limit = (1.0 - s.time_scalar).mul_add(i.scalar_2764, s.time_scalar) * s.heading_time_limit;
    let strength = s
        .time_response
        .evaluate(clamp(i.heading_time, 0.0, limit) / limit)
        * s.strength;
    let forward = if dot3(i.forward, i.velocity) < 0.0 {
        i.forward.map(|v| -v)
    } else {
        i.forward
    };
    let error = dot3(cross(forward, i.velocity), i.normal);
    let mut output = i.normal.map(|v| v * (error * strength));
    if i.turn * error < 0.0 {
        let scale = 1.0 - clamp(i.turn.abs(), 0.0, s.opposite_turn_limit) / s.opposite_turn_limit;
        output = output.map(|v| v * scale);
    }
    output
}
