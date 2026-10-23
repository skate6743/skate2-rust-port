//! `Math::TwoBoneIKSolve` (0x829b1e30), core 0x829b1240..0x829b1e30.
//! The original chain stays fixed while the desired end can be clamped and
//! retried. Full4 arithmetic, output prefixes and numeric service ordering are
//! preserved; Xenon estimate/reduction and exceptional VMX selection remain
//! the existing declared host numeric boundaries.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits the actual active Skeleton IK caller")
)]

use crate::{
    reckoning_numeric::{
        ReckoningNumeric, cross_full4, length_from_squared_and_estimate, try_acos,
    },
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_cosine, try_sin_cos},
    simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::cmp::Ordering;

#[cfg(test)]
#[path = "skeleton_two_bone_tests.rs"]
mod tests;

const EPSILON: f32 = f32::from_bits(0x3a03_126f);
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const RADIANS_TO_DEGREES: f32 = f32::from_bits(0x4265_2ee1);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

/// Copied original geometry; recursion never changes these three points.
#[derive(Debug)]
pub(crate) struct TwoBoneChain {
    pub(crate) root: SkeletonRootVector,
    pub(crate) middle: SkeletonRootVector,
    pub(crate) end: SkeletonRootVector,
}

/// Wrapper scalar f32 values, splatted by the source before entering the core.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TwoBoneLimits {
    pub(crate) minimum_degrees: f32,
    pub(crate) maximum_degrees: f32,
    pub(crate) override_maximum_with_original: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TwoBoneIkOutcome {
    Reachable,
    Unreachable,
    Invalid,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TwoBoneIkError<N, R> {
    MinimumAngleAssertion { bits: u32 },
    Numeric(N),
    Trigonometry(R),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for TwoBoneIkError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "two-bone IK: {self:?}")
    }
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::error::Error for TwoBoneIkError<N, R> {}

type SolveResult<T, N, R> = Result<T, TwoBoneIkError<N, R>>;

/// Source output pointers are separate exclusive outputs. Numeric errors retain
/// every end-target commit from preceding recursive attempts. Exhaustion copies
/// original end first, then original middle, whereas a short chain copies only
/// the middle. No additional finite/collinear/coincident-target gate is added.
pub(crate) fn try_solve_two_bone<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    chain: &TwoBoneChain,
    middle_output: &mut SkeletonRootVector,
    target_and_end_output: &mut SkeletonRootVector,
    limits: TwoBoneLimits,
    mut remaining_calls: u32,
    numeric: &mut N,
    rounding: &mut R,
) -> SolveResult<TwoBoneIkOutcome, N::Error, R::Error> {
    loop {
        if !matches!(
            limits.minimum_degrees.partial_cmp(&90.0),
            Some(Ordering::Less | Ordering::Equal)
        ) {
            return Err(TwoBoneIkError::MinimumAngleAssertion {
                bits: limits.minimum_degrees.to_bits(),
            });
        }
        if remaining_calls == 0 {
            *target_and_end_output = chain.end;
            *middle_output = chain.middle;
            return Ok(TwoBoneIkOutcome::Invalid);
        }
        let lengths = try_chain_lengths(chain, numeric).map_err(TwoBoneIkError::Numeric)?;
        if EPSILON > lengths.upper || EPSILON > lengths.lower {
            *middle_output = chain.middle;
            return Ok(TwoBoneIkOutcome::Invalid);
        }
        match try_attempt(
            chain,
            *target_and_end_output,
            limits,
            &lengths,
            numeric,
            rounding,
        )? {
            Attempt::Retry(target) => {
                *target_and_end_output = target;
                remaining_calls -= 1;
            }
            Attempt::Reachable(middle) => {
                *middle_output = middle;
                return Ok(TwoBoneIkOutcome::Reachable);
            }
            Attempt::Unreachable { middle, end } => {
                *middle_output = middle;
                *target_and_end_output = end;
                return Ok(TwoBoneIkOutcome::Unreachable);
            }
        }
    }
}

struct ChainLengths {
    upper: f32,
    lower: f32,
    twice_upper: f32,
    upper_squared: f32,
    lower_squared: f32,
}

fn try_chain_lengths<N: ReckoningNumeric>(
    chain: &TwoBoneChain,
    numeric: &mut N,
) -> Result<ChainLengths, N::Error> {
    let upper_vector = subtract(chain.middle, chain.root);
    let lower_vector = subtract(chain.end, chain.middle);
    let upper_squared = dot(upper_vector, upper_vector, numeric)?;
    let lower_squared = dot(lower_vector, lower_vector, numeric)?;
    let upper_seed = numeric.reciprocal_sqrt_estimate(upper_squared)?;
    let lower_seed = numeric.reciprocal_sqrt_estimate(lower_squared)?;
    let upper = length_from_squared_and_estimate(upper_squared, upper_seed);
    let lower = length_from_squared_and_estimate(lower_squared, lower_seed);
    Ok(ChainLengths {
        upper,
        lower,
        twice_upper: 2.0 * upper,
        upper_squared: upper * upper,
        lower_squared: lower * lower,
    })
}

enum Attempt {
    Retry(SkeletonRootVector),
    Reachable(SkeletonRootVector),
    Unreachable {
        middle: SkeletonRootVector,
        end: SkeletonRootVector,
    },
}

struct PlaneCoordinates {
    target_frame: SkeletonRootMatrix,
    root: SkeletonRootVector,
    middle: SkeletonRootVector,
    end: SkeletonRootVector,
    target: SkeletonRootVector,
}

fn try_attempt<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    chain: &TwoBoneChain,
    target: SkeletonRootVector,
    limits: TwoBoneLimits,
    lengths: &ChainLengths,
    numeric: &mut N,
    rounding: &mut R,
) -> SolveResult<Attempt, N::Error, R::Error> {
    let plane = try_plane_coordinates(chain, target, numeric).map_err(TwoBoneIkError::Numeric)?;
    let original_angle_numerator = dot(
        subtract(chain.middle, chain.root),
        negate(subtract(chain.end, chain.middle)),
        numeric,
    )
    .map_err(TwoBoneIkError::Numeric)?;
    let delta = subtract(plane.target, plane.root);
    let distance_squared = delta.xyz[0].mul_add(delta.xyz[0], delta.xyz[1] * delta.xyz[1]);
    let distance_seed = numeric
        .reciprocal_sqrt_estimate(distance_squared)
        .map_err(TwoBoneIkError::Numeric)?;
    let distance = length_from_squared_and_estimate(distance_squared, distance_seed);
    let original_denominator = lengths.upper * negate_lane(lengths.lower);
    let original_reciprocal =
        reciprocal(original_denominator, numeric).map_err(TwoBoneIkError::Numeric)?;
    let original_degrees = try_acos(original_reciprocal * original_angle_numerator, numeric)
        .map_err(TwoBoneIkError::Numeric)?
        * RADIANS_TO_DEGREES;
    let maximum_degrees =
        if original_degrees > limits.maximum_degrees && limits.override_maximum_with_original {
            original_degrees
        } else {
            limits.maximum_degrees
        };
    let twice_product = lengths.twice_upper * lengths.lower;
    let sum_squares = lengths.upper_squared + lengths.lower_squared;
    let minimum_cosine = try_cosine(limits.minimum_degrees * DEGREES_TO_RADIANS, rounding)
        .map_err(TwoBoneIkError::Trigonometry)?;
    let minimum_squared = sum_squares - twice_product * minimum_cosine;
    let minimum_seed = numeric
        .reciprocal_sqrt_estimate(minimum_squared)
        .map_err(TwoBoneIkError::Numeric)?;
    let minimum_distance = length_from_squared_and_estimate(minimum_squared, minimum_seed);
    let maximum_cosine = try_cosine(maximum_degrees * DEGREES_TO_RADIANS, rounding)
        .map_err(TwoBoneIkError::Trigonometry)?;
    let maximum_squared = sum_squares - twice_product * maximum_cosine;
    let maximum_seed = numeric
        .reciprocal_sqrt_estimate(maximum_squared)
        .map_err(TwoBoneIkError::Numeric)?;
    let maximum_distance = length_from_squared_and_estimate(maximum_squared, maximum_seed);
    if minimum_distance > distance || distance > maximum_distance {
        if distance > lengths.upper + lengths.lower
            && maximum_degrees.partial_cmp(&180.0) == Some(Ordering::Equal)
        {
            let direction = unit(delta, numeric).map_err(TwoBoneIkError::Numeric)?;
            let middle = multiply_add(direction, lengths.upper, plane.root);
            let end = multiply_add(direction, lengths.lower, middle);
            return Ok(Attempt::Unreachable {
                middle: transform_point(&plane.target_frame, middle),
                end: transform_point(&plane.target_frame, end),
            });
        }
        let radius = if distance > maximum_distance {
            maximum_distance - EPSILON
        } else {
            minimum_distance + EPSILON
        };
        return try_clamped_target(chain.root, target, radius, numeric)
            .map(Attempt::Retry)
            .map_err(TwoBoneIkError::Numeric);
    }
    let inverse_twice_product =
        reciprocal(twice_product, numeric).map_err(TwoBoneIkError::Numeric)?;
    let derived_degrees = try_acos(
        clamp_zero_to_one(inverse_twice_product * (sum_squares - distance_squared)),
        numeric,
    )
    .map_err(TwoBoneIkError::Numeric)?
        * RADIANS_TO_DEGREES;
    if limits.minimum_degrees >= derived_degrees {
        return try_clamped_target(chain.root, target, minimum_distance + EPSILON, numeric)
            .map(Attempt::Retry)
            .map_err(TwoBoneIkError::Numeric);
    }
    try_reachable_middle(
        &plane,
        lengths,
        distance,
        distance_squared,
        numeric,
        rounding,
    )
    .map(Attempt::Reachable)
}

fn try_clamped_target<N: ReckoningNumeric>(
    root: SkeletonRootVector,
    target: SkeletonRootVector,
    radius: f32,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    Ok(multiply_add(
        unit(subtract(target, root), numeric)?,
        radius,
        root,
    ))
}

fn try_plane_coordinates<N: ReckoningNumeric>(
    chain: &TwoBoneChain,
    target: SkeletonRootVector,
    numeric: &mut N,
) -> Result<PlaneCoordinates, N::Error> {
    let original_delta = subtract(chain.end, chain.root);
    let target_delta = subtract(target, chain.root);
    let normal = cross(
        subtract(chain.middle, chain.root),
        subtract(chain.end, chain.middle),
    );
    let original_squared = dot(original_delta, original_delta, numeric)?;
    let target_squared = dot(target_delta, target_delta, numeric)?;
    let original_seed = numeric.reciprocal_sqrt_estimate(original_squared)?;
    let normal_squared = dot(normal, normal, numeric)?;
    let target_seed = numeric.reciprocal_sqrt_estimate(target_squared)?;
    let normal_seed = numeric.reciprocal_sqrt_estimate(normal_squared)?;
    let original_forward = normalize(original_delta, original_squared, original_seed);
    let target_forward = normalize(target_delta, target_squared, target_seed);
    let normal = normalize(normal, normal_squared, normal_seed);
    let original_side = cross(original_forward, normal);
    let target_side = cross(target_forward, normal);
    let target_normal = cross(target_side, target_forward);
    let original_frame =
        SkeletonRootMatrix::from_columns([original_forward, original_side, normal, chain.root]);
    let target_frame =
        SkeletonRootMatrix::from_columns([target_forward, target_side, target_normal, chain.root]);
    let original_inverse = try_cofactor_inverse(&original_frame, numeric)?;
    let target_inverse = try_cofactor_inverse(&target_frame, numeric)?;
    Ok(PlaneCoordinates {
        target_frame,
        root: transform_point(&original_inverse, chain.root),
        middle: transform_point(&original_inverse, chain.middle),
        end: transform_point(&original_inverse, chain.end),
        target: transform_point(&target_inverse, target),
    })
}

/// 0x829b151c..0x829b1724: adjugate inverse, not the root rigid inverse.
/// The word merges recover W from cofactor XYZ: W duplicates Y in each column.
fn try_cofactor_inverse<N: ReckoningNumeric>(
    frame: &SkeletonRootMatrix,
    numeric: &mut N,
) -> Result<SkeletonRootMatrix, N::Error> {
    let [x, y, z, position] = *frame.columns();
    let cofactors = [cross(y, z), cross(z, x), cross(x, y)];
    let basis = try_inverse_basis_from_cofactors(x, cofactors, numeric)?;
    let negative_position = position.xyz.map(negate_lane);
    let inverse_position = scale(basis[0], negative_position[0]);
    let inverse_position = multiply_add(basis[1], negative_position[1], inverse_position);
    let inverse_position = multiply_add(basis[2], negative_position[2], inverse_position);
    Ok(SkeletonRootMatrix::from_columns([
        basis[0],
        basis[1],
        basis[2],
        inverse_position,
    ]))
}

/// Shared determinant/adjugate basis graph: two-bone inverse 0x829b151c,
/// wheel compression 0x82b2b5f4..b644 and the 1.02 Fill's latched stick
/// 824e63d0..6488. Callers retain their own cofactor construction order and
/// translation multiply operand order.
pub(crate) fn try_inverse_basis_from_cofactors<N: ReckoningNumeric>(
    first_axis: SkeletonRootVector,
    cofactors: [SkeletonRootVector; 3],
    numeric: &mut N,
) -> Result<[SkeletonRootVector; 3], N::Error> {
    let determinant = dot(first_axis, cofactors[0], numeric)?;
    let inverse_determinant = reciprocal(determinant, numeric)?;
    Ok(std::array::from_fn(|lane| {
        scale(
            SkeletonRootVector::known([
                cofactors[0].xyz[lane],
                cofactors[1].xyz[lane],
                cofactors[2].xyz[lane],
                cofactors[1].xyz[lane],
            ]),
            inverse_determinant,
        )
    }))
}

fn try_reachable_middle<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    plane: &PlaneCoordinates,
    lengths: &ChainLengths,
    distance: f32,
    distance_squared: f32,
    numeric: &mut N,
    rounding: &mut R,
) -> SolveResult<SkeletonRootVector, N::Error, R::Error> {
    let inverse_distance = reciprocal(distance, numeric).map_err(TwoBoneIkError::Numeric)?;
    let delta_x = plane.target.xyz[0] - plane.root.xyz[0];
    let mut orientation_angle = try_acos(clamp_zero_to_one(inverse_distance * delta_x), numeric)
        .map_err(TwoBoneIkError::Numeric)?;
    if (plane.middle.xyz[1] < 0.0 && plane.target.xyz[1] > 0.0)
        || (plane.middle.xyz[1] > 0.0 && plane.target.xyz[1] < 0.0)
    {
        orientation_angle *= -1.0;
    }
    let denominator = lengths.twice_upper * distance;
    let numerator = (lengths.upper_squared + distance_squared) - lengths.lower_squared;
    let inverse = reciprocal(denominator, numeric).map_err(TwoBoneIkError::Numeric)?;
    // Raw 0x112a0aee at1a74: the angle is the FMA ADDEND INSIDE acos.
    let angle_input = inverse.mul_add(numerator, orientation_angle);
    let angle =
        try_acos(clamp_zero_to_one(angle_input), numeric).map_err(TwoBoneIkError::Numeric)?;
    let first_trig = try_sin_cos(angle, rounding).map_err(TwoBoneIkError::Trigonometry)?;
    let second_trig =
        try_sin_cos(TWO_PI - angle, rounding).map_err(TwoBoneIkError::Trigonometry)?;
    let original_delta = subtract(plane.end, plane.root);
    let first = rotate_plane_delta(original_delta, first_trig);
    let second = rotate_plane_delta(original_delta, second_trig);
    let first_squared = dot(first, first, numeric).map_err(TwoBoneIkError::Numeric)?;
    let first_seed = numeric
        .reciprocal_sqrt_estimate(first_squared)
        .map_err(TwoBoneIkError::Numeric)?;
    let second_squared = dot(second, second, numeric).map_err(TwoBoneIkError::Numeric)?;
    let second_seed = numeric
        .reciprocal_sqrt_estimate(second_squared)
        .map_err(TwoBoneIkError::Numeric)?;
    let first = multiply_add(
        normalize(first, first_squared, first_seed),
        lengths.upper,
        plane.root,
    );
    let second = multiply_add(
        normalize(second, second_squared, second_seed),
        lengths.upper,
        plane.root,
    );
    let first = transform_point(&plane.target_frame, first);
    let second = transform_point(&plane.target_frame, second);
    Ok(if plane.end.xyz[1] > plane.middle.xyz[1] {
        second
    } else {
        first
    })
}

fn rotate_plane_delta(value: SkeletonRootVector, (sine, cosine): (f32, f32)) -> SkeletonRootVector {
    let negative_sine = negate_lane(sine);
    let rotation = SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([cosine, sine, 0.0, cosine]),
        SkeletonRootVector::known([negative_sine, cosine, 0.0, negative_sine]),
        SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]),
        SkeletonRootVector::known([0.0; 4]),
    ]);
    transform_point(&rotation, value)
}

fn reciprocal<N: ReckoningNumeric>(value: f32, numeric: &mut N) -> Result<f32, N::Error> {
    let seed = numeric.reciprocal_estimate(value)?;
    Ok(refine_drive_reciprocal(value, seed))
}

fn unit<N: ReckoningNumeric>(
    value: SkeletonRootVector,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let squared = dot(value, value, numeric)?;
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    Ok(normalize(value, squared, seed))
}

fn normalize(value: SkeletonRootVector, squared: f32, seed: f32) -> SkeletonRootVector {
    scale(value, refine_drive_reciprocal_sqrt(squared, seed))
}

fn transform_point(frame: &SkeletonRootMatrix, point: SkeletonRootVector) -> SkeletonRootVector {
    let [x, y, z, position] = *frame.columns();
    let result = multiply_add(x, point.xyz[0], position);
    let result = multiply_add(y, point.xyz[1], result);
    multiply_add(z, point.xyz[2], result)
}

/// Native VMX operand order is max(0,x), then min(1,result). Ordinary host
/// comparisons retain a NaN input; native exceptional selection is unresolved.
fn clamp_zero_to_one(value: f32) -> f32 {
    let lower = if value < 0.0 { 0.0 } else { value };
    if lower > 1.0 { 1.0 } else { lower }
}

fn negate_lane(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}

fn negate(value: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: value.xyz.map(negate_lane),
        w: value.w.map(negate_lane),
    }
}

fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}

fn scale(value: SkeletonRootVector, factor: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: value.xyz.map(|lane| lane * factor),
        w: value.w.map(|lane| lane * factor),
    }
}

fn multiply_add(
    value: SkeletonRootVector,
    factor: f32,
    addend: SkeletonRootVector,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| value.xyz[lane].mul_add(factor, addend.xyz[lane])),
        w: value
            .w
            .zip(addend.w)
            .map(|(value, addend)| value.mul_add(factor, addend)),
    }
}

fn cross(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    let lanes = cross_full4(scratch(left), scratch(right));
    SkeletonRootVector {
        xyz: [lanes[0], lanes[1], lanes[2]],
        w: left.w.zip(right.w).map(|_| lanes[3]),
    }
}

fn scratch(value: SkeletonRootVector) -> [f32; 4] {
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}

fn dot<N: ReckoningNumeric>(
    left: SkeletonRootVector,
    right: SkeletonRootVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    numeric.dot3(scratch(left), scratch(right))
}
