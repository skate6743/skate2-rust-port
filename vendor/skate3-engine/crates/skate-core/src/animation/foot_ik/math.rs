//! Matrix interpolation used by SkeletonIK, TU3 82BD3150/82BD2E60.
//! The shared Xenon estimate primitives remain host approximations pending
//! hardware comparison; the recovered branch and refinement order is retained.
use crate::{
    input::angle::atan,
    physics::{
        board_motion_output::inverse_length_squared,
        native_arithmetic::{dot3, reciprocal_estimate},
        skeleton_animation_record::{AnimationPartTransform as Transform, IDENTITY},
    },
    trigonometry::sin_cos,
};

pub(super) type Vector = [f32; 4];

pub(super) fn reciprocal(value: f32, refinements: usize) -> f32 {
    let mut inverse = reciprocal_estimate(value);
    for _ in 0..refinements {
        let error = (-inverse).mul_add(value, 1.0);
        inverse = inverse.mul_add(error, inverse);
    }
    inverse
}

pub(super) fn length(value: Vector) -> f32 {
    let squared = dot3(value, value);
    let result = squared * inverse_length_squared(squared, 2);
    if squared == 0.0 { 0.0 } else { result }
}

pub(super) fn normalize(value: Vector) -> Vector {
    let inverse = inverse_length_squared(dot3(value, value), 2);
    value.map(|v| v * inverse)
}

pub(super) fn cross(left: Vector, right: Vector) -> Vector {
    [
        (-left[2]).mul_add(right[1], left[1] * right[2]),
        (-left[0]).mul_add(right[2], left[2] * right[0]),
        (-left[1]).mul_add(right[0], left[0] * right[1]),
        (-left[3]).mul_add(right[3], left[3] * right[3]),
    ]
}

///82BD3D90. The very short-vector branch preserves the original fourth lane.
pub(super) fn limit_length(value: Vector, limit: f32) -> Vector {
    let magnitude = length(value);
    if !(magnitude >= f32::from_bits(0x3780_0000)) {
        return value;
    }
    let capped = if limit - magnitude >= 0.0 {
        magnitude
    } else {
        limit
    };
    let inverse = reciprocal(magnitude, 2);
    value.map(|v| (v * capped) * inverse)
}

///82BD2E60's axis/angle outputs. The unused screw-pivot output is omitted.
pub(super) fn rotation_axis_angle(rotation: &Transform) -> (Vector, f32) {
    let skew = [
        rotation[1][2] - rotation[2][1],
        rotation[2][0] - rotation[0][2],
        rotation[0][1] - rotation[1][0],
        0.0,
    ];
    let sine_twice = length(skew);
    let cosine_twice = ((rotation[0][0] + rotation[1][1]) + rotation[2][2]) - 1.0;
    let mut axis = if sine_twice > 0.0 {
        let inverse = reciprocal(sine_twice, 2) * 1.0;
        skew.map(|v| v * inverse)
    } else {
        [0.0; 4]
    };
    let ratio = sine_twice.mul_add(reciprocal(cosine_twice, 1), 0.0);
    let mut angle = atan(ratio);
    if cosine_twice < 0.0 {
        angle += f32::from_bits(0x4049_0FDB).copysign(sine_twice);
    }
    if cosine_twice == 0.0 {
        angle = f32::from_bits(0x3FC9_0FDB).copysign(sine_twice);
    }
    // Literal word 820CFC28, loaded with lvlx. Do not replace this with a
    // conventional numerical tolerance.
    if sine_twice <= f32::from_bits(0x0020_0000) && cosine_twice <= 0.0 {
        axis = half_turn_axis(rotation);
    }
    (axis, angle)
}

///827B7400 selects the greatest diagonal. Its selected component is squared
/// before normalization (e.g. vmulfp at827B7488), unlike a common textbook
/// matrix-to-axis conversion. Preserve the native formula and tie ordering.
fn half_turn_axis(rotation: &Transform) -> Vector {
    let selected = if rotation[0][0] > rotation[1][1] {
        if rotation[0][0] > rotation[2][2] {
            0
        } else {
            2
        }
    } else if rotation[1][1] > rotation[2][2] {
        1
    } else {
        2
    };
    let mut axis = [0.0; 4];
    for lane in 0..3 {
        axis[lane] = if lane == selected {
            let diagonal = 1.0 + rotation[selected][selected];
            diagonal * diagonal
        } else {
            rotation[selected][lane] + rotation[lane][selected]
        };
    }
    axis[3] = axis[0];
    normalize(axis)
}

///Rodrigues construction in82BD34E0..3548. The native permutation duplicates
/// each column's X component into W; W is not a fabricated affine zero.
pub(super) fn axis_rotation(axis: Vector, angle: f32) -> Transform {
    let (sine, cosine) = sin_cos(angle);
    let complement = 1.0 - cosine;
    let [x, y, z, _] = axis;
    let sx = sine * x;
    let sy = sine * y;
    let sz = sine * z;
    let tx = complement * x;
    let ty = complement * y;
    let tz = complement * z;
    let xx = tx.mul_add(x, cosine);
    let yy = ty.mul_add(y, cosine);
    let zz = tz.mul_add(z, cosine);
    let xy_positive = tx.mul_add(y, sz);
    let xy_negative = ty * x - sz;
    let xz_positive = tz.mul_add(x, sy);
    let xz_negative = tx * z - sy;
    let yz_positive = ty.mul_add(z, sx);
    let yz_negative = tz * y - sx;
    [
        [xx, xy_positive, xz_negative, xx],
        [xy_negative, yy, yz_positive, xy_negative],
        [xz_positive, yz_negative, zz, xz_positive],
        [0.0; 4],
    ]
}

///82BD3150 returns the remaining rotation angle as well as the blended frame.
pub fn interpolate(a: &Transform, b: &Transform, weight: f32) -> (Transform, f32) {
    if weight >= 1.0 {
        return (*b, 0.0);
    }
    let mut relative = IDENTITY;
    for column in 0..3 {
        relative[column] = core::array::from_fn(|lane| {
            let x = a[0][column] * b[0][lane];
            let y = a[1][column].mul_add(b[1][lane], x);
            a[2][column].mul_add(b[2][lane], y)
        });
    }
    let (axis, angle) = rotation_axis_angle(&relative);
    if weight <= 0.0 {
        return (*a, angle);
    }
    let mut output = IDENTITY;
    if angle < f32::from_bits(0x3D0E_FA35) {
        for column in 0..3 {
            output[column] = normalize(core::array::from_fn(|lane| {
                (b[column][lane] - a[column][lane]).mul_add(weight, a[column][lane])
            }));
        }
    } else {
        let rotation = axis_rotation(axis, angle * weight);
        for column in 0..3 {
            output[column] = core::array::from_fn(|lane| {
                let x = a[column][0] * rotation[0][lane];
                let y = a[column][1].mul_add(rotation[1][lane], x);
                a[column][2].mul_add(rotation[2][lane], y)
            });
        }
    }
    output[3] = core::array::from_fn(|lane| (b[3][lane] - a[3][lane]).mul_add(weight, a[3][lane]));
    (output, angle - angle * weight)
}

///82E0A570 overrides the interpolation helper's translation, including at
/// endpoint weights. Preserve its weighted-sum rounding instead of delta lerp.
pub fn interpolate_affine(a: &Transform, b: &Transform, weight: f32) -> Transform {
    let translation =
        core::array::from_fn(|lane| a[3][lane].mul_add(1.0 - weight, b[3][lane] * weight));
    let (mut output, _) = interpolate(a, b, weight);
    output[3] = translation;
    output
}
/// General inverse expanded at8296FB50..FD14 and SkeletonData Init82BD6E9C.
/// The transpose permutation retains the second cofactor in the fourth lane.
pub fn inverse_affine(frame: &Transform) -> Transform {
    let cofactors = [
        cross(frame[1], frame[2]),
        cross(frame[2], frame[0]),
        cross(frame[0], frame[1]),
    ];
    let inverse = reciprocal(dot3(frame[0], cofactors[0]), 2);
    let mut result: Transform = core::array::from_fn(|column| {
        if column == 3 {
            [0.0; 4]
        } else {
            [
                cofactors[0][column] * inverse,
                cofactors[1][column] * inverse,
                cofactors[2][column] * inverse,
                cofactors[1][column] * inverse,
            ]
        }
    });
    let negative = frame[3].map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    result[3] = core::array::from_fn(|lane| {
        let x = negative[0] * result[0][lane];
        let y = negative[1].mul_add(result[1][lane], x);
        negative[2].mul_add(result[2][lane], y)
    });
    result
}
