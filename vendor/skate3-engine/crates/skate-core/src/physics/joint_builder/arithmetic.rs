//! Original TU3 joint angular limits and shared constraint-frame arithmetic.
use super::*;
use crate::physics::constraint_frames;
pub(super) fn decode_parameters(raw: RetailJointParametersRaw) -> JointParameters {
    JointParameters {
        linear_position_allowance: vector_from_words(&raw.words, 0),
        linear_velocity_allowance: vector_from_words(&raw.words, 4),
        twist_velocity_allowance: f32::from_bits(raw.words[8]),
        swing_velocity_allowance: f32::from_bits(raw.words[9]),
        swing_threshold: f32::from_bits(raw.words[12]),
        twist_threshold: f32::from_bits(raw.words[13]),
        swing_mode: raw.words[14],
        twist_mode: raw.words[15],
    }
}

pub(super) fn decode_frames(raw: RetailJointFramesRaw) -> JointFrames {
    JointFrames {
        orientation_a: quaternion_from_words(&raw.words, 0),
        anchor_a: vector_from_words(&raw.words, 4),
        orientation_b: quaternion_from_words(&raw.words, 8),
        anchor_b: vector_from_words(&raw.words, 12),
        linear_orientation_b: quaternion_from_words(&raw.words, 16),
    }
}

pub(super) fn angular_rows(
    parameters: JointParameters,
    basis_a: Basis3,
    basis_b: Basis3,
    relative: [[f32; 3]; 3],
    rqd: Rqd,
) -> ([Vector3; 3], [f32; 3], [f32; 3]) {
    let finite_infinity = tu3::FINITE_INFINITY;
    // Native defaults are all three frame-B axes, including skipped limits.
    let mut axes = basis_columns(basis_b);
    let mut low = [-finite_infinity; 3];
    let mut high = [finite_infinity; 3];

    match parameters.swing_mode {
        0 => {
            axes[1] = rqd.axes[1];
            axes[2] = rqd.axes[2];
            low[1] = 2.0 * rqd.relative.y;
            high[1] = low[1];
            low[2] = 2.0 * rqd.relative.z;
            high[2] = low[2];
        }
        1 => {
            // 82AE4178..4188 skips the cone calculation when all splatted
            // cosine lanes compare >= the original 0.99999 guard.
            if !(relative[0][0] >= tu3::NEAR_PARALLEL) {
                let first_a = basis_column(basis_a, 0);
                let first_b = basis_column(basis_b, 0);
                let perpendicular_z = relative[0][2];
                let perpendicular_y = relative[0][1];
                let squared =
                    perpendicular_y.mul_add(perpendicular_y, perpendicular_z * perpendicular_z);
                let root = squared * constraint_frames::reciprocal_sqrt(squared);
                let root = if squared == 0.0 { 0.0 } else { root };
                let inverse_length = constraint_frames::reciprocal(root) * 1.0;
                axes[1] = scale(cross(first_a, first_b), inverse_length);
                axes[2] = cross(axes[1], first_a);
                // The bound is written even while inside the cone, allowing
                // the predicted rate to reach the limit during this step.
                low[1] = (parameters.swing_threshold - relative[0][0]) * inverse_length;
            }
        }
        2 | 3 => {
            axes[1] = basis_column(basis_b, 1);
            if parameters.twist_mode == 0 {
                axes[2] = rqd.axes[2];
                low[2] = 2.0 * rqd.relative.z;
                high[2] = low[2];
            } else {
                axes[2] = cross(basis_column(basis_a, 0), axes[1]);
                low[2] = -relative[0][1];
                high[2] = low[2];
            }
            if parameters.swing_mode == 2 {
                let denominator = relative[0][2];
                // 82AE42F4..4304: equality skips the reciprocal and retains
                // both unbounded values. NaN is not equal to zero.
                if denominator != 0.0 {
                    let correction = constraint_frames::reciprocal(denominator)
                        * (parameters.swing_threshold - relative[0][0]);
                    if denominator < 0.0 {
                        high[1] = correction;
                    } else {
                        low[1] = correction;
                    }
                }
            }
        }
        _ => {
            axes[1] = basis_column(basis_b, 1);
            axes[2] = basis_column(basis_b, 2);
        }
    }

    match parameters.twist_mode {
        0 => {
            axes[0] = rqd.axes[0];
            low[0] = 2.0 * rqd.relative.x;
            high[0] = low[0];
        }
        1 => {
            axes[0] = basis_column(basis_a, 0);
            // Derived matrix-index form matching the emitted TU3 subtraction.
            let denominator = relative[2][1] - relative[1][2];
            // 82AE4434..4444 also retains the unbounded interval at zero.
            if denominator != 0.0 {
                let numerator = (1.0 + relative[0][0]) * parameters.twist_threshold
                    - (relative[1][1] + relative[2][2]);
                let correction = constraint_frames::reciprocal(denominator) * numerator;
                if denominator < 0.0 {
                    high[0] = correction;
                } else {
                    low[0] = correction;
                }
            }
        }
        _ => {
            axes[0] = basis_column(basis_b, 0);
        }
    }

    (axes, low, high)
}

pub(super) fn create_rqd(world_a: RetailQuaternion, world_b: RetailQuaternion) -> Rqd {
    let rows = constraint_frames::quaternion_rows(world_a, world_b);
    Rqd {
        relative: rows.relative,
        axes: rows.axes,
    }
}

pub(super) fn relative_basis(a: Basis3, b: Basis3) -> [[f32; 3]; 3] {
    core::array::from_fn(|row| {
        core::array::from_fn(|column| dot3(basis_column(a, row), basis_column(b, column)))
    })
}

pub(super) fn scaled_projection_columns(
    axes: [Vector3; 3],
    inverse_effective_mass: [f32; 3],
) -> [[f32; 3]; 3] {
    core::array::from_fn(|component| {
        core::array::from_fn(|row| {
            vector_to_array(axes[row])[component] * inverse_effective_mass[row]
        })
    })
}

pub(super) fn packed_inertia_columns(
    packed: RetailPackedWorldInverseInertia,
    inverse_mass: f32,
) -> [[f32; 4]; 3] {
    [
        [packed.full.x, packed.full.y, packed.full.z, inverse_mass],
        [packed.full.y, packed.split.y, packed.split.z, inverse_mass],
        [packed.full.z, packed.split.z, packed.split.x, inverse_mass],
    ]
}

pub(super) fn zero_inertia() -> RetailPackedWorldInverseInertia {
    RetailPackedWorldInverseInertia {
        full: Vector3::ZERO,
        split: Vector3::ZERO,
    }
}

pub(super) fn basis_from_quaternion_retail(q: RetailQuaternion) -> Basis3 {
    constraint_frames::basis(q)
}

pub(super) fn quaternion_multiply(a: RetailQuaternion, b: RetailQuaternion) -> RetailQuaternion {
    constraint_frames::compose(a, b)
}

pub(super) fn point_rate(linear: Vector3, angular: Vector3, arm: Vector3) -> Vector3 {
    constraint_frames::point_rate(linear, angular, arm)
}

pub(super) fn project(vector: Vector3, axes: [Vector3; 3]) -> [f32; 3] {
    axes.map(|axis| dot3(vector, axis))
}

pub(super) fn multiply_basis(basis: Basis3, vector: Vector3) -> Vector3 {
    constraint_frames::transform_direction(basis, vector)
}

pub(super) fn basis_columns(basis: Basis3) -> [Vector3; 3] {
    core::array::from_fn(|index| basis_column(basis, index))
}

pub(super) fn basis_column(basis: Basis3, index: usize) -> Vector3 {
    Vector3::new(
        basis.columns[index][0],
        basis.columns[index][1],
        basis.columns[index][2],
    )
}

pub(super) fn vector_from_words<const N: usize>(words: &[u32; N], offset: usize) -> Vector3 {
    Vector3::new(
        f32::from_bits(words[offset]),
        f32::from_bits(words[offset + 1]),
        f32::from_bits(words[offset + 2]),
    )
}

pub(super) fn quaternion_from_words<const N: usize>(
    words: &[u32; N],
    offset: usize,
) -> RetailQuaternion {
    RetailQuaternion {
        x: f32::from_bits(words[offset]),
        y: f32::from_bits(words[offset + 1]),
        z: f32::from_bits(words[offset + 2]),
        w: f32::from_bits(words[offset + 3]),
    }
}

pub(super) fn vector_to_array(value: Vector3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

pub(super) fn cross(left: Vector3, right: Vector3) -> Vector3 {
    constraint_frames::cross(left, right)
}

pub(super) fn dot3(left: Vector3, right: Vector3) -> f32 {
    crate::physics::native_arithmetic::dot3(
        [left.x, left.y, left.z, 0.0],
        [right.x, right.y, right.z, 0.0],
    )
}

pub(super) fn add(left: Vector3, right: Vector3) -> Vector3 {
    Vector3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

pub(super) fn sub(left: Vector3, right: Vector3) -> Vector3 {
    Vector3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

pub(super) fn scale(value: Vector3, scalar: f32) -> Vector3 {
    Vector3::new(value.x * scalar, value.y * scalar, value.z * scalar)
}

pub(super) fn vmx_min(left: f32, right: f32) -> f32 {
    crate::physics::native_arithmetic::vector_min(left, right)
}

pub(super) fn vmx_max(left: f32, right: f32) -> f32 {
    crate::physics::native_arithmetic::vector_max(left, right)
}
