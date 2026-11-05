//! Scalar lane of TU3 `XMVectorSinCos`, `0x82473A08`.
//!
//! Coefficients are the executable words at 0x822F97C0..0x822F9860.
//! Arithmetic follows the recovered polynomial structure. Numerical agreement
//! with the original game requires independent validation.
pub fn sin_cos(angle: f32) -> (f32, f32) {
    let turns = (angle * f32::from_bits(0x3E22_F983)).round_ties_even();
    let x = (-f32::from_bits(0x40C9_0FDB)).mul_add(turns, angle);
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    let x5 = x3 * x2;
    let x6 = x3 * x3;
    let x7 = x4 * x3;
    let x8 = x4 * x4;
    let x9 = x5 * x4;
    let x10 = x5 * x5;
    let x11 = x6 * x5;
    let x12 = x6 * x6;
    let x13 = x7 * x6;
    let x15 = x8 * x7;
    let x14 = x7 * x7;
    let x16 = x8 * x8;
    let x17 = x9 * x8;
    let x19 = x10 * x9;
    let x18 = x9 * x9;
    let x21 = x11 * x10;
    let x20 = x10 * x10;
    let x23 = x12 * x11;
    let x22 = x11 * x11;

    let sin = f32::from_bits(0xBE2A_AAAB).mul_add(x3, x);
    let sin = f32::from_bits(0x3C08_8889).mul_add(x5, sin);
    let sin = x7.mul_add(f32::from_bits(0xB950_0D01), sin);
    let sin = f32::from_bits(0x3638_EF1D).mul_add(x9, sin);
    let sin = f32::from_bits(0xB2D7_322B).mul_add(x11, sin);
    let sin = f32::from_bits(0x2F30_9231).mul_add(x13, sin);
    let sin = f32::from_bits(0xAB57_3F9F).mul_add(x15, sin);
    let sin = f32::from_bits(0x274A_963C).mul_add(x17, sin);
    let sin = f32::from_bits(0xA317_A4DA).mul_add(x19, sin);
    let sin = f32::from_bits(0x1EB8_DC78).mul_add(x21, sin);
    let sin = f32::from_bits(0x9A3B_0DA1).mul_add(x23, sin);

    let cos = (-0.5_f32).mul_add(x2, 1.0);
    let cos = f32::from_bits(0x3D2A_AAAB).mul_add(x4, cos);
    let cos = f32::from_bits(0xBAB6_0B61).mul_add(x6, cos);
    let cos = f32::from_bits(0x37D0_0D01).mul_add(x8, cos);
    let cos = f32::from_bits(0xB493_F27E).mul_add(x10, cos);
    let cos = f32::from_bits(0x310F_76C8).mul_add(x12, cos);
    let cos = f32::from_bits(0xAD49_CBA5).mul_add(x14, cos);
    let cos = f32::from_bits(0x2957_3F9F).mul_add(x16, cos);
    let cos = f32::from_bits(0xA534_13C3).mul_add(x18, cos);
    let cos = f32::from_bits(0x20F2_A15D).mul_add(x20, cos);
    let cos = f32::from_bits(0x9C86_71CB).mul_add(x22, cos);
    (sin, cos)
}

/// Standalone TU3 cosine82473930. The even-power tree differs from SinCos;
/// normal gameplay camera roll82BD35B8 calls this function independently.
/// This is the retained readable polynomial also audited for SetTurning.
pub fn cos(angle: f32) -> f32 {
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

/// TU3 vector sine 824531C8. Its sequential odd powers differ from SinCos's
/// multiplication tree, so callers of the standalone sine must use this path.
pub fn sin(angle: f32) -> f32 {
    let turns = (angle * f32::from_bits(0x3E22_F983)).round_ties_even();
    let x = (-f32::from_bits(0x40C9_0FDB)).mul_add(turns, angle);
    let x2 = x * x;
    let mut power = x * x2;
    let mut result = x;
    for coefficient in [
        0xBE2A_AAAB,
        0x3C08_8889,
        0xB950_0D01,
        0x3638_EF1D,
        0xB2D7_322B,
        0x2F30_9231,
        0xAB57_3F9F,
        0x274A_963C,
        0xA317_A4DA,
        0x1EB8_DC78,
        0x9A3B_0DA1,
    ] {
        result = f32::from_bits(coefficient).mul_add(power, result);
        power *= x2;
    }
    result
}

/// Scalar lane of TU3 vector acos 82453298. The native radicand constant is
/// 0x3f800001, and its reciprocal square root receives one refinement.
pub fn acos(value: f32) -> f32 {
    (f32::from_bits(0x4049_0FDB) * 0.5) - asin(value)
}

/// Shared inverse-sine polynomial, also inlined by DisablePushBrake82BA5150.
/// Keep this intermediate separate: that caller subtracts degrees from90,
/// whereas vector acos subtracts radians from half pi.
pub fn asin(value: f32) -> f32 {
    let a = value.abs();
    let cube = (value * value) * a;
    let c = f32::from_bits;
    let p0 = c(0x400B_1889).mul_add(a, c(0xC0D1_360E));
    let p1 = c(0x3E66_3246).mul_add(a, c(0xBF98_3F2F));
    let p2 = c(0xBED6_5553).mul_add(a, c(0x4089_80BD));
    let p3 = c(0xBD6D_D42D).mul_add(a, c(0x3F1D_D7B6));
    let p0 = p0.mul_add(a, c(0x40AF_6AD8));
    let p1 = p1.mul_add(a, c(0x3FB5_8485));
    let p2 = p2.mul_add(a, c(0xC08F_6AD9));
    let p3 = p3.mul_add(a, c(0xBFAF_4418));
    let left = p1.mul_add(cube, p0);
    let right = p3.mul_add(cube, p2);
    let radicand = c(0x3F80_0001) - a;
    let mut r = crate::physics::reciprocal_sqrt::estimate(radicand);
    let correction = (-(radicand * 0.5)).mul_add(r * r, 0.5);
    r = r.mul_add(correction, r);
    let residual = (-a).mul_add(value, value) * right;
    residual.mul_add(r, value * left)
}
