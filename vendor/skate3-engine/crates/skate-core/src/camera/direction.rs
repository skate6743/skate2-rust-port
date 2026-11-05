//! Complete TU3 8258DC10 direction-to-angular-vector operation, shared by
//! normal rig framing and avoidance. Input Y is consumed directly, not normalized.

use super::{
    normalize_angle,
    vector_tracker::{length, refined_reciprocal},
};

pub fn direction_to_angles(direction: [f32; 4]) -> [f32; 4] {
    let horizontal = [direction[0], 0.0, direction[2], 0.0];
    let magnitude = length(horizontal);
    if !(magnitude > f32::from_bits(0x38d1b717)) {
        return [0.0; 4];
    }
    let inverse = refined_reciprocal(magnitude);
    let x = inverse * horizontal[0];
    let z = inverse * horizontal[2];
    let (component, base) = if x >= 0.0 {
        if z >= 0.0 {
            (x, 0.0)
        } else {
            (-z, f32::from_bits(0x3fc90fdb))
        }
    } else if z <= 0.0 {
        (-x, f32::from_bits(0x40490fdb))
    } else {
        (z, f32::from_bits(0x4096cbe4))
    };
    let heading = normalize_angle(asin_lane(clamp(component)) + base);
    [asin_lane(clamp(direction[1])), heading, 0.0, 0.0]
}

fn clamp(value: f32) -> f32 {
    let lower = if -1.0 > value { -1.0 } else { value };
    if 1.0 < lower { 1.0 } else { lower }
}

// Native inline asin polynomial: 822F9820/30/40, radicand822FB840.
// Keep it before the acos subtraction used by the separate82453298 operation.
fn asin_lane(value: f32) -> f32 {
    let a = value.abs();
    let cube = (value * value) * a;
    let c = f32::from_bits;
    let p0 = c(0x400b1889).mul_add(a, c(0xc0d1360e));
    let p1 = c(0x3e663246).mul_add(a, c(0xbf983f2f));
    let p2 = c(0xbed65553).mul_add(a, c(0x408980bd));
    let p3 = c(0xbd6dd42d).mul_add(a, c(0x3f1dd7b6));
    let p0 = p0.mul_add(a, c(0x40af6ad8));
    let p1 = p1.mul_add(a, c(0x3fb58485));
    let p2 = p2.mul_add(a, c(0xc08f6ad9));
    let p3 = p3.mul_add(a, c(0xbfaf4418));
    let left = p1.mul_add(cube, p0);
    let right = p3.mul_add(cube, p2);
    let radicand = c(0x3f800001) - a;
    let mut r = crate::physics::reciprocal_sqrt::estimate(radicand);
    let correction = (-(radicand * 0.5)).mul_add(r * r, 0.5);
    r = r.mul_add(correction, r);
    let residual = (-a).mul_add(value, value) * right;
    residual.mul_add(r, value * left)
}
