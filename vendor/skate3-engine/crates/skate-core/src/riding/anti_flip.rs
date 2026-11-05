//! Complete TU3 Toolkit_AntiFlipTorque 82D94190, distinct from heading/manual.
use super::vector::{cross, dot3, normalize};
use crate::{point_graph::PointGraph, trigonometry::acos};
#[derive(Clone)]
pub struct AntiFlipSettings {
    pub axis_96_response: PointGraph<8>, // +1832/+1864
    pub axis_64_response: PointGraph<8>, // +1912/+1944
    pub normal_threshold: [f32; 4],      // live 830BD350
}
pub struct AntiFlipInput {
    pub flags_2468: u32,
    pub balance: f32, // +2720
    pub axis_96: [f32; 4],
    pub axis_64: [f32; 4],
    pub projection_axis_544: [f32; 4],
}
pub fn calculate(s: &AntiFlipSettings, i: &AntiFlipInput) -> [f32; 4] {
    if i.flags_2468 & 0x2000_0000 != 0 {
        return [0.0; 4];
    }
    let project = |axis: [f32; 4]| {
        let dot = dot3(i.projection_axis_544, axis);
        normalize(
            core::array::from_fn(|n| axis[n] - i.projection_axis_544[n] * dot),
            s.normal_threshold,
        )
    };
    let a = project(i.axis_96);
    let b = project(i.axis_64);
    let angle = |axis, projected| {
        let d = dot3(axis, projected);
        let d = if 0.0 > d { 0.0 } else { d };
        let d = if 1.0 < d { 1.0 } else { d };
        acos(d) * f32::from_bits(0x3f22_f983)
    };
    let sign_a = if dot3(i.axis_64, cross(i.axis_96, a)) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let sign_b = if dot3(i.axis_96, cross(i.axis_64, b)) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let b_strength = s.axis_64_response.evaluate(angle(i.axis_64, b)) * sign_b;
    let a_strength = (s.axis_96_response.evaluate(angle(i.axis_96, a)) * sign_a)
        * if i.balance == 0.0 { 1.0 } else { 0.0 };
    core::array::from_fn(|n| i.axis_64[n].mul_add(a_strength, i.axis_96[n] * b_strength))
}
