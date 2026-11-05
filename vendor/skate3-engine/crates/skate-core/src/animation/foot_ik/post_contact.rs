//! SkeletonIK::CheckFootAgainstBoard82BF0D70, including raised deck ends.
use super::{math::reciprocal, settings::Settings, transforms::inverse_rigid};
use crate::physics::{
    native_arithmetic::vector_min,
    skeleton_animation_record::{AnimationPartTransform as Transform, transform_point},
};

#[derive(Clone, Copy, Debug)]
pub struct SettingsPost {
    ///physics_animation/default layout780.
    pub minimum_board_up: f32,
    ///Global212's authored layout412/416, selected by the wipeout argument.
    pub wipeout_height: f32,
    pub riding_height: f32,
}

pub(super) fn target(
    board: &Transform,
    foot: [f32; 4],
    adjacent: [f32; 4],
    animated_board_position: [f32; 4],
    wipeout: bool,
    settings: &Settings,
    post: &SettingsPost,
) -> Option<[f32; 4]> {
    let inverse = inverse_rigid(board);
    let local_foot = transform_point(&inverse, foot);
    let local_adjacent = transform_point(&inverse, adjacent);
    let mut midpoint: [f32; 4] =
        core::array::from_fn(|i| (local_foot[i] + local_adjacent[i]) * 0.5);
    //vrlimi mask4 preserves the foot's Y; X/Z test both foot and midpoint.
    midpoint[1] = local_foot[1];
    let x = vector_min(local_foot[0].abs(), midpoint[0].abs());
    let z = vector_min(local_foot[2].abs(), midpoint[2].abs());
    if x >= settings.deck_half_width + settings.post_ik_padding[0]
        || z >= settings.deck_total_half_length + settings.post_ik_padding[2]
    {
        return None;
    }
    let mut height = if wipeout {
        post.wipeout_height
    } else {
        post.riding_height
    };
    if z > settings.deck_half_length {
        let distance = z - settings.deck_half_length;
        let end = settings.deck_total_half_length - settings.deck_half_length;
        let distance = if end - distance >= 0.0 { distance } else { end };
        //Constructor82BED418 caches tan(clamp(authored degrees*.01745...)).
        let angle = settings.deck_front_angle_degrees * f32::from_bits(0x3C8E_FA35);
        let angle = if 0.01 - angle >= 0.0 { 0.01 } else { angle };
        let maximum = f32::from_bits(0x3FC7_C82D);
        let angle = if maximum - angle >= 0.0 {
            angle
        } else {
            maximum
        };
        height = distance.mul_add(tangent(angle), height);
    }
    if midpoint[1] >= height || midpoint[1] <= height - f32::from_bits(0x3E99_999A) {
        return None;
    }
    let mut result = animated_board_position;
    result[1] = height;
    Some(transform_point(board, result))
}

///Scalar constructor path through original8252D980. Its argument is already
///clamped to [.01,1.5607964]; coefficients come from822FE3C0..E0.
fn tangent(angle: f32) -> f32 {
    let quadrant = (angle * f32::from_bits(0x3F22_F983)).round_ties_even();
    let reduced = (-quadrant).mul_add(f32::from_bits(0x3FC9_0FDB), angle);
    let reduced = (-quadrant).mul_add(f32::from_bits(0x2E85_A309), reduced);
    let squared = reduced * reduced;
    let denominator = squared.mul_add(f32::from_bits(0x3505_BBA8), f32::from_bits(0xB9A3_7B25));
    let denominator = squared.mul_add(denominator, f32::from_bits(0x3CD2_3CF5));
    let denominator = squared.mul_add(denominator, f32::from_bits(0xBEEE_F582));
    let denominator = squared.mul_add(denominator, 1.0);
    let numerator = squared.mul_add(f32::from_bits(0xB795_D5B9), f32::from_bits(0x3B60_7415));
    let numerator = squared.mul_add(numerator, f32::from_bits(0xBE08_95AF));
    let numerator = reduced.mul_add(squared * numerator, reduced);
    //vcmpbfp's two high result bits select corresponding sign/exponent bits.
    //The bounded constructor argument preserves these bits in both operands;
    //retain the actual bit-select rather than replacing it by a boolean branch.
    let bound = f32::from_bits(0x3980_0000);
    let mask = (if reduced > bound { 0x8000_0000 } else { 0 })
        | (if reduced < -bound { 0x4000_0000 } else { 0 });
    let denominator = f32::from_bits((denominator.to_bits() & !mask) | (1.0f32.to_bits() & mask));
    let numerator = f32::from_bits((numerator.to_bits() & !mask) | (reduced.to_bits() & mask));
    if quadrant.abs() as i32 & 1 == 0 {
        numerator * reciprocal(denominator, 2)
    } else {
        denominator * reciprocal(-numerator, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_tangent_rational_spans_both_constructor_quadrants() {
        //Independent mathematical values test argument reduction and rational
        //operand order, without claiming original hardware estimate parity.
        for angle in [0.01f32, 0.25, 0.75, 1.0, 1.5607964] {
            let actual = tangent(angle);
            let reference = f64::from(angle).tan() as f32;
            assert!(
                (actual - reference).abs() < 3e-5 * reference.abs().max(1.0),
                "{angle}: {actual} vs {reference}"
            );
        }
    }
}
