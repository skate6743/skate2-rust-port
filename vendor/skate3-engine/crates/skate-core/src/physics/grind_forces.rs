//! TU3 grind force leaves. Their inputs belong to the retained grind manager;
//! contact admission and lifecycle are separate from these force calculations.
use super::native_arithmetic::dot3;
pub type V = [f32; 4];
#[path = "grind_forces/launch.rs"]
pub mod launch;
#[path = "grind_forces/noise.rs"]
pub mod noise;
#[path = "grind_forces/orientation.rs"]
pub mod orientation;
#[path = "grind_forces/post.rs"]
pub mod post;
#[path = "grind_forces/reckoning.rs"]
pub mod reckoning;
#[path = "grind_forces/release.rs"]
pub mod release;
#[path = "grind_forces/slide.rs"]
pub mod slide;
#[path = "grind_forces/support.rs"]
pub mod support;
#[cfg(test)]
#[path = "grind_forces/tests.rs"]
mod tests;

///82D3FA18, called by the 50-50 update82D41D70 with800,0,.07.
/// Returns a force at the deck origin; None means no accumulator call.
pub fn lateral_pin(
    board: [V; 4],
    point: V,
    across: V,
    velocity: V,
    strength: f32,
    forward_offset: f32,
    up_offset: f32,
    forward_selected: bool,
    slope_multiplier: f32,
) -> Option<V> {
    let direction = if forward_selected {
        board[2]
    } else {
        scale(board[2], -1.0)
    };
    let reference = core::array::from_fn(|i| {
        direction[i].mul_add(forward_offset, board[3][i]) - board[1][i] * up_offset
    });
    let force = scale(across, dot3(sub(point, reference), across) * strength);
    let length = length(force);
    if !(length > 0.0) {
        return None;
    }
    let axis = scale(force, reciprocal(length));
    let damping = scale(
        axis,
        strength * f32::from_bits(0x3e08_3127) * dot3(velocity, axis),
    );
    Some(scale(sub(force, damping), slope_multiplier))
}

///82D3FD88 after its native surface/material selection. It damps velocity in
/// the plane perpendicular to the grind normal, including the along-rail lane.
pub fn friction(
    velocity: V,
    grind_normal: V,
    support: V,
    time_multiplier: f32,
    flagged_surface: bool,
    surface_multiplier: f32,
    geometry_kind: u32,
    strengths: [f32; 3],
) -> V {
    let tangent_velocity = sub(velocity, scale(grind_normal, dot3(velocity, grind_normal)));
    let speed = length(tangent_velocity);
    if speed <= 0.001 {
        return [0.0; 4];
    }
    let strength = strengths[match geometry_kind {
        0 => 0,
        1 => 1,
        _ => 2,
    }];
    let load = support[1].max(0.0);
    let surface_flag_multiplier = if flagged_surface { 1.9 } else { 1.0 };
    let multiplier = load / speed
        * time_multiplier
        * surface_flag_multiplier
        * surface_multiplier
        * strength
        * f32::from_bits(0xbef5_c28f);
    scale(tangent_velocity, multiplier)
}

fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: V, scale: f32) -> V {
    a.map(|v| v * scale)
}

///82D3FD88 skips the accumulator call at or below its planar-speed threshold.
pub fn friction_applies(velocity: V, normal: V) -> bool {
    let planar = sub(velocity, scale(normal, dot3(velocity, normal)));
    length(planar) > 0.001
}

fn length(v: V) -> f32 {
    let square = dot3(v, v);
    if square == 0.0 {
        return 0.0;
    }
    square * super::board_motion_output::inverse_length_squared(square, 2)
}

fn reciprocal(value: f32) -> f32 {
    let mut inverse = value.recip();
    for _ in 0..2 {
        let correction = (-inverse).mul_add(value, 1.0);
        inverse = inverse.mul_add(correction, inverse);
    }
    inverse
}

///82D3FD88's material switch. S3 adds material13 to S2's material2 case.
pub fn material_multiplier(material: u32) -> f32 {
    match material {
        2 | 13 => 1.7,
        3 => 2.6,
        4 => 0.6,
        5 => 4.0,
        6 => 5.0,
        _ => 1.3,
    }
}

///82D3FA18: gravity relief bypasses the authored PinVsSlope four-point graph.
pub fn pin_slope(
    normal: V,
    upmost_normal: V,
    gravity_relief: f32,
    pin_vs_slope: &crate::point_graph::PointGraph<4>,
) -> f32 {
    if gravity_relief > 0.0 {
        1.0
    } else {
        pin_vs_slope.evaluate(dot3(normal, upmost_normal))
    }
}
