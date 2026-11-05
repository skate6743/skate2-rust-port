//! Math::_TwoBoneIKSolve8296F8C0, TU3 complete reachable/recursive/invalid path.
//! Plane construction uses the native general inverse, not a rigid transpose.
use super::math::{Vector, cross, inverse_affine, length, normalize, reciprocal};
use crate::{
    physics::{
        native_arithmetic::{dot3, vector_max, vector_min},
        skeleton_animation_record::{AnimationPartTransform as Transform, transform_point},
    },
    trigonometry::{acos, cos, sin_cos},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveResult {
    Solved,
    Extended,
    Invalid,
}

#[derive(Clone, Copy, Debug)]
pub struct AngleLimits {
    pub minimum_degrees: f32,
    pub maximum_degrees: f32,
}

const EPSILON: f32 = f32::from_bits(0x3A03_126F); //830BD4B0 <-821A0318
const TO_DEGREES: f32 = f32::from_bits(0x4265_2EE1); //830BD390
const TO_RADIANS: f32 = f32::from_bits(0x3C8E_FA35); //830BD360

pub fn solve(
    root: Vector,
    middle: Vector,
    end: Vector,
    solved_middle: &mut Vector,
    target: &mut Vector,
    limits: AngleLimits,
    override_maximum_with_original: bool,
    recursion_budget: u32,
) -> SolveResult {
    if recursion_budget == 0 {
        *target = end;
        *solved_middle = middle;
        return SolveResult::Invalid;
    }
    let upper = subtract(middle, root);
    let lower = subtract(end, middle);
    let upper_length = length(upper);
    let lower_length = length(lower);
    if EPSILON > upper_length || EPSILON > lower_length {
        *solved_middle = middle;
        return SolveResult::Invalid;
    }
    let requested_delta = subtract(*target, root);
    let forward = normalize(subtract(end, root));
    let normal = normalize(cross(upper, lower));
    let original_basis = [forward, cross(forward, normal), normal, root];
    let target_forward = normalize(requested_delta);
    let target_lateral = cross(target_forward, normal);
    let target_normal = cross(target_lateral, target_forward);
    let target_basis = [target_forward, target_lateral, target_normal, root];
    let original_inverse = inverse_affine(&original_basis);
    let target_inverse = inverse_affine(&target_basis);
    // Each point is transformed separately, retaining the original origin's
    // rounding; do not replace these with pre-subtracted vectors.
    let local_root = transform_point(&original_inverse, root);
    let local_middle = transform_point(&original_inverse, middle);
    let local_end = transform_point(&original_inverse, end);
    let local_target = transform_point(&target_inverse, *target);
    let target_delta = subtract(local_target, local_root);
    let planar_squared =
        target_delta[0].mul_add(target_delta[0], target_delta[1] * target_delta[1]);
    let distance = square_root(planar_squared);
    let upper_squared = upper_length * upper_length;
    let lower_squared = lower_length * lower_length;
    let twice_upper = 2.0 * upper_length;
    let twice_product = twice_upper * lower_length;
    let sum_squared = upper_squared + lower_squared;
    let negative_lower = lower.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    let negative_length = f32::from_bits(lower_length.to_bits() ^ 0x8000_0000);
    let original_angle =
        acos(reciprocal(upper_length * negative_length, 2) * dot3(upper, negative_lower))
            * TO_DEGREES;
    let mut maximum = limits.maximum_degrees;
    if original_angle > maximum && override_maximum_with_original {
        maximum = original_angle;
    }
    let minimum_distance =
        square_root(sum_squared - twice_product * cos(limits.minimum_degrees * TO_RADIANS));
    let maximum_distance = square_root(sum_squared - twice_product * cos(maximum * TO_RADIANS));
    if minimum_distance > distance || distance > maximum_distance {
        if distance > upper_length + lower_length && maximum == 180.0 {
            let direction = normalize(target_delta);
            let middle_local = scaled_add(direction, upper_length, local_root);
            let end_local = scaled_add(direction, lower_length, middle_local);
            *solved_middle = transform_point(&target_basis, middle_local);
            *target = transform_point(&target_basis, end_local);
            return SolveResult::Extended;
        }
        let distance = if distance > maximum_distance {
            maximum_distance - EPSILON
        } else {
            minimum_distance + EPSILON
        };
        *target = scaled_add(normalize(requested_delta), distance, root);
        return solve(
            root,
            middle,
            end,
            solved_middle,
            target,
            limits,
            override_maximum_with_original,
            recursion_budget - 1,
        );
    }
    let cosine = clamp_native_cosine(reciprocal(twice_product, 2) * (sum_squared - planar_squared));
    let interior_angle = acos(cosine) * TO_DEGREES;
    if limits.minimum_degrees >= interior_angle {
        *target = scaled_add(normalize(requested_delta), minimum_distance + EPSILON, root);
        return solve(
            root,
            middle,
            end,
            solved_middle,
            target,
            limits,
            override_maximum_with_original,
            recursion_budget - 1,
        );
    }
    let mut target_angle = acos(clamp_native_cosine(
        reciprocal(distance, 2) * target_delta[0],
    ));
    if local_middle[1] < 0.0 && local_target[1] > 0.0
        || local_middle[1] > 0.0 && local_target[1] < 0.0
    {
        target_angle *= -1.0;
    }
    // The target heading is an addend *inside* this acos in the TU3 body
    // (vmaddfp82970088). Preserve it rather than fitting a textbook IK formula.
    let ratio = reciprocal(twice_upper * distance, 2).mul_add(
        (upper_squared + planar_squared) - lower_squared,
        target_angle,
    );
    let angle = acos(clamp_native_cosine(ratio));
    let original_delta = subtract(local_end, local_root);
    let positive = candidate(
        original_delta,
        angle,
        upper_length,
        local_root,
        &target_basis,
    );
    let negative = candidate(
        original_delta,
        f32::from_bits(0x40C9_0FDB) - angle,
        upper_length,
        local_root,
        &target_basis,
    );
    *solved_middle = if local_end[1] > local_middle[1] {
        negative
    } else {
        positive
    };
    SolveResult::Solved
}

///The source uses830BD310=0 and830BD430=1 for these clamps. The lower
///constant is not minus one; its initializer82F82718 loads82165A10.
fn clamp_native_cosine(value: f32) -> f32 {
    vector_min(1.0, vector_max(0.0, value))
}

fn square_root(squared: f32) -> f32 {
    let inverse = crate::physics::board_motion_output::inverse_length_squared(squared, 2);
    let value = squared * inverse;
    if squared == 0.0 { 0.0 } else { value }
}
fn subtract(a: Vector, b: Vector) -> Vector {
    core::array::from_fn(|i| a[i] - b[i])
}
fn scaled_add(a: Vector, scale: f32, b: Vector) -> Vector {
    core::array::from_fn(|i| a[i].mul_add(scale, b[i]))
}

fn candidate(
    delta: Vector,
    angle: f32,
    upper_length: f32,
    root: Vector,
    basis: &Transform,
) -> Vector {
    let (sine, cosine) = sin_cos(angle);
    let rotation = [
        [cosine, sine, 0.0, cosine],
        [-sine, cosine, 0.0, -sine],
        [0.0, 0.0, 1.0, 0.0],
        [0.0; 4],
    ];
    let rotated = normalize(transform_point(&rotation, delta));
    transform_point(basis, scaled_add(rotated, upper_length, root))
}
