//! Complete TU3 SetTurning vector remap, 82BB4248.
//! Curves and offsets are supplied from the native animation_turning layout.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct TurnRemap {
    pub magnitude: PointGraph<16>,
    pub angle: PointGraph<16>,
    pub angle_offset: f32,
}

fn clamp(value: f32) -> f32 {
    let lower = if -1.0 - value >= 0.0 { -1.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

fn angle(x: f32, y: f32, magnitude: f32) -> f32 {
    if !(magnitude > f32::from_bits(0x3780_0000)) {
        return 0.0;
    }
    let reciprocal = super::angle::reciprocal_estimate(x);
    let reciprocal = reciprocal.mul_add((-reciprocal).mul_add(x, 1.0), reciprocal);
    let basic = super::angle::atan(y.mul_add(reciprocal, 0.0));
    let sign = y.to_bits() & 0x8000_0000;
    let result = if x < 0.0 {
        f32::from_bits(0x4049_0fdb | sign) + basic
    } else {
        basic
    };
    if x == 0.0 {
        f32::from_bits(0x3fc9_0fdb | sign)
    } else {
        result
    }
}

// Scalar lane of standalone cosine 82473930; its power tree differs from SinCos.
fn cosine(angle: f32) -> f32 {
    let turns = (angle * f32::from_bits(0x3e22_f983)).round_ties_even();
    let x = (-f32::from_bits(0x40c9_0fdb)).mul_add(turns, angle);
    let x2 = x * x;
    let x4 = x2 * x2;
    let x6 = x4 * x2;
    let x8 = x4 * x4;
    let x10 = x6 * x4;
    let x12 = x6 * x6;
    let x14 = x8 * x6;
    let x16 = x8 * x8;
    let x18 = x10 * x8;
    let x22 = x12 * x10;
    let x20 = x10 * x10;
    let mut value = (-0.5_f32).mul_add(x2, 1.0);
    for (power, coefficient) in [
        (x4, 0x3d2a_aaab),
        (x6, 0xbab6_0b61),
        (x8, 0x37d0_0d01),
        (x10, 0xb493_f27e),
        (x12, 0x310f_76c8),
        (x14, 0xad49_cba5),
        (x16, 0x2957_3f9f),
        (x18, 0xa534_13c3),
        (x20, 0x20f2_a15d),
        (x22, 0x9c86_71cb),
    ] {
        value = f32::from_bits(coefficient).mul_add(power, value);
    }
    value
}

impl TurnRemap {
    pub fn apply(&self, input: [f32; 2]) -> [f32; 4] {
        let pi = f32::from_bits(0x4049_0fdb);
        let magnitude = super::controller::magnitude(input[0] * input[0] + input[1] * input[1]);
        let first_angle = angle(input[0], input[1], magnitude);
        let graph_input = if first_angle >= 0.0 {
            first_angle
        } else {
            first_angle + pi
        };
        let scaled = magnitude * self.magnitude.evaluate(graph_input);
        let x = clamp(cosine(first_angle) * scaled);
        let y = clamp(crate::trigonometry::sin(first_angle) * scaled);
        let magnitude = super::controller::magnitude(x * x + y * y);
        let second_angle = angle(x, y, magnitude);
        let graph_input = if second_angle >= 0.0 {
            second_angle
        } else {
            second_angle + pi
        };
        let mapped = self.angle.evaluate(graph_input);
        let mapped = if second_angle >= 0.0 {
            mapped
        } else {
            mapped + pi
        };
        let result_angle = self.angle_offset + mapped;
        [
            clamp(cosine(result_angle) * magnitude),
            clamp(crate::trigonometry::sin(result_angle) * magnitude),
            0.0,
            0.0,
        ]
    }
}
