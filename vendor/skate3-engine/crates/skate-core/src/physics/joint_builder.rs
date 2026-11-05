//! Semantic port of TU3 `rw::physics::JointJacobian::Build`.
//!
//! This module is intentionally separate from generic engine joints.  It
//! accepts the exact 64-byte parameter and 80-byte frame records emitted by
//! `SkateboardBody::CreateJoints`, plus the rigid-body fields read by
//! `0x82AE3BC8`, and emits the 384-byte workspace consumed by the joint branch
//! of `0x82AE27D0`.
//!
//! Evidence labels used below:
//! - **Observed**: direct field loads, stores, constants, or branches in TU3.
//! - **Derived**: scalar algebra preserving the observed VMX data flow.
//! - **Inferred**: source-level naming where the executable has no type name.

use super::{
    constraint_frames,
    joint_records::{RetailJointFramesRaw, RetailJointParametersRaw},
    joint_solver::RetailJointJacobian,
    native_arithmetic,
    rigid_body::{RetailPackedWorldInverseInertia, RetailQuaternion},
};
use crate::math::{Basis3, Vector3};
mod arithmetic;
use arithmetic::*;

pub mod tu3 {
    pub const JOINT_BATCH_BUILD: u32 = 0x82AE_39D0;
    pub const JOINT_JACOBIAN_BUILD: u32 = 0x82AE_3BC8;
    pub const JACOBIAN_RQD_CREATE: u32 = 0x82AE_0DB0;
    pub const ITERATIVE_CONSTRAINT_SOLVER: u32 = 0x82AE_27D0;

    pub const ACTIVE_BODY: u32 = 4;
    pub const JACOBIAN_BYTES: usize = 0x180;
    pub const JACOBIAN_WORDS: usize = JACOBIAN_BYTES / 4;

    /// Observed TU3 vector constant used as the finite "unbounded" value.
    pub const FINITE_INFINITY: f32 = f32::from_bits(0x7F7F_FFFF);
    /// Observed singularity guard initialized at `0x830BDE50`.
    pub const NEAR_PARALLEL: f32 = f32::from_bits(0x3F7F_FF58);
}

/// Explicit body snapshot consumed by `JointJacobian::Build`.
///
/// All fields are **Observed** inputs.  `basis` is kept separate from
/// `orientation` because TU3 reads both rather than rebuilding one from the
/// other.  Likewise the reaction address is metadata copied into the raw
/// Jacobian; this port never dereferences it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailJointBodyInput {
    pub reaction_guest_address: u32,
    pub state: u32,
    pub orientation: RetailQuaternion,
    pub center_of_mass: Vector3,
    pub basis: Basis3,
    pub linear_velocity: Vector3,
    pub angular_velocity: Vector3,
    pub force_acceleration: Vector3,
    pub torque_acceleration: Vector3,
    pub inverse_mass: f32,
    pub world_inverse_inertia: RetailPackedWorldInverseInertia,
}

/// Complete explicit call state for TU3 `JointJacobian::Build`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailJointBuildInput {
    /// **Observed** 64-byte record at `Joint+4`.
    pub parameters: RetailJointParametersRaw,
    /// **Observed** 80-byte record at `Joint+0`.
    pub frames: RetailJointFramesRaw,
    /// **Observed** body at `Joint+0x10`.
    pub body_a: RetailJointBodyInput,
    /// **Observed** body at `Joint+0x14`.
    pub body_b: RetailJointBodyInput,
    /// **Observed** `Simulation+0xA0`.
    pub time_step: f32,
    /// **Observed** source `Joint*`, retained in vector 12's fourth word.
    pub joint_guest_address: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct JointParameters {
    linear_position_allowance: Vector3,
    linear_velocity_allowance: Vector3,
    twist_velocity_allowance: f32,
    swing_velocity_allowance: f32,
    swing_threshold: f32,
    twist_threshold: f32,
    swing_mode: u32,
    twist_mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct JointFrames {
    orientation_a: RetailQuaternion,
    anchor_a: Vector3,
    orientation_b: RetailQuaternion,
    anchor_b: Vector3,
    linear_orientation_b: RetailQuaternion,
}

#[derive(Clone, Copy, Debug)]
struct Rqd {
    relative: RetailQuaternion,
    axes: [Vector3; 3],
}

/// Builds TU3's fixed six-lane, 384-byte joint workspace.
///
/// The three linear and three angular accumulators begin at zero because the
/// retail builder clears all three 128-byte cache lines on every batch build.
pub fn build_retail_joint_jacobian(input: RetailJointBuildInput) -> RetailJointJacobian {
    debug_assert!(input.time_step.is_finite() && input.time_step > 0.0);

    let parameters = decode_parameters(input.parameters);
    let frames = decode_frames(input.frames);
    let active_a = input.body_a.state & tu3::ACTIVE_BODY != 0;
    let active_b = input.body_b.state & tu3::ACTIVE_BODY != 0;

    // Observed composition order: body orientation followed by local frame.
    let world_orientation_a = quaternion_multiply(input.body_a.orientation, frames.orientation_a);
    let world_orientation_b = quaternion_multiply(input.body_b.orientation, frames.orientation_b);
    let world_linear_orientation =
        quaternion_multiply(input.body_b.orientation, frames.linear_orientation_b);
    let angular_basis_a = basis_from_quaternion_retail(world_orientation_a);
    let angular_basis_b = basis_from_quaternion_retail(world_orientation_b);
    let linear_basis = basis_from_quaternion_retail(world_linear_orientation);

    let arm_a = multiply_basis(input.body_a.basis, frames.anchor_a);
    let arm_b = multiply_basis(input.body_b.basis, frames.anchor_b);
    let linear_axes = basis_columns(linear_basis);
    // Retained matrix product: the original gathers an RQD quaternion before
    // constructing this matrix. The VA extension of 82AE3D98 (10242A35) is
    // not independently decoded, so its rounding path remains unverified.
    let relative_matrix = relative_basis(angular_basis_a, angular_basis_b);
    let rqd = create_rqd(world_orientation_a, world_orientation_b);
    let (angular_axes, angular_raw_low, angular_raw_high) = angular_rows(
        parameters,
        angular_basis_a,
        angular_basis_b,
        relative_matrix,
        rqd,
    );

    let inverse_mass_a = if active_a {
        input.body_a.inverse_mass
    } else {
        0.0
    };
    let inverse_mass_b = if active_b {
        input.body_b.inverse_mass
    } else {
        0.0
    };
    let inertia_a = if active_a {
        input.body_a.world_inverse_inertia
    } else {
        zero_inertia()
    };
    let inertia_b = if active_b {
        input.body_b.world_inverse_inertia
    } else {
        zero_inertia()
    };

    let linear_inverse_effective_mass = core::array::from_fn(|index| {
        let a = cross(arm_a, linear_axes[index]);
        let b = cross(arm_b, linear_axes[index]);
        let ia = constraint_frames::multiply_inertia_from_zero(inertia_a, a);
        let ib = constraint_frames::multiply_inertia_from_zero(inertia_b, b);
        // 82AE4814..4990 combines the two bodies component-wise, then uses
        // (Z + Y) + (X + inverse_mass_A + inverse_mass_B), not two dot sums.
        let x = b.x.mul_add(ib.x, a.x.mul_add(ia.x, 0.0));
        let y = b.y.mul_add(ib.y, a.y.mul_add(ia.y, 0.0));
        let z = b.z.mul_add(ib.z, a.z.mul_add(ia.z, 0.0));
        let denominator = (z + y) + (x + (inverse_mass_a + inverse_mass_b));
        native_arithmetic::reciprocal_estimate(denominator)
    });
    let angular_inverse_effective_mass = core::array::from_fn(|index| {
        let axis = angular_axes[index];
        let response = add(
            constraint_frames::multiply_inertia_from_zero(inertia_a, axis),
            constraint_frames::multiply_inertia_from_zero(inertia_b, axis),
        );
        let x = axis.x.mul_add(response.x, 0.0);
        let y = axis.y.mul_add(response.y, 0.0);
        let z = axis.z.mul_add(response.z, 0.0);
        // 82AE496C..49A0: Z + (X + Y), one estimate, then the RQD factor 1/2.
        0.5 * native_arithmetic::reciprocal_estimate(z + (x + y))
    });

    let point_velocity_a = point_rate(
        input.body_a.linear_velocity,
        input.body_a.angular_velocity,
        arm_a,
    );
    let point_velocity_b = point_rate(
        input.body_b.linear_velocity,
        input.body_b.angular_velocity,
        arm_b,
    );
    // Native prediction is computed before the active-body inertia mask.
    // An inactive body can still supply the source-owned kinematic rates.
    let point_acceleration_a = point_rate(
        input.body_a.force_acceleration,
        input.body_a.torque_acceleration,
        arm_a,
    );
    let point_acceleration_b = point_rate(
        input.body_b.force_acceleration,
        input.body_b.torque_acceleration,
        arm_b,
    );
    let displacement_a = scale(
        super::constraint_frames::multiply_add(
            point_acceleration_a,
            input.time_step,
            point_velocity_a,
        ),
        input.time_step,
    );
    let displacement_b = scale(
        super::constraint_frames::multiply_add(
            point_acceleration_b,
            input.time_step,
            point_velocity_b,
        ),
        input.time_step,
    );
    let relative_displacement = sub(displacement_b, displacement_a);
    let predicted_separation = sub(
        add(add(input.body_b.center_of_mass, arm_b), displacement_b),
        add(add(input.body_a.center_of_mass, arm_a), displacement_a),
    );
    let local_rate_displacement = project(relative_displacement, linear_axes);
    let local_separation = project(predicted_separation, linear_axes);

    let mut linear_low = [0.0; 3];
    let mut linear_high = [0.0; 3];
    let position_allowance = vector_to_array(parameters.linear_position_allowance);
    let velocity_allowance = vector_to_array(parameters.linear_velocity_allowance);
    for lane in 0..3 {
        let displacement_low = local_separation[lane] - position_allowance[lane];
        let displacement_high = local_separation[lane] + position_allowance[lane];
        let velocity_low =
            local_rate_displacement[lane] - velocity_allowance[lane] * input.time_step;
        let velocity_high =
            local_rate_displacement[lane] + velocity_allowance[lane] * input.time_step;
        let raw_low = vmx_max(displacement_low, vmx_min(velocity_low, displacement_high));
        let raw_high = vmx_min(displacement_high, vmx_max(velocity_high, displacement_low));
        linear_low[lane] = raw_low * linear_inverse_effective_mass[lane];
        linear_high[lane] = raw_high * linear_inverse_effective_mass[lane];
    }

    let relative_angular_rate = sub(
        super::constraint_frames::multiply_add(
            input.body_b.torque_acceleration,
            input.time_step,
            sub(input.body_b.angular_velocity, input.body_a.angular_velocity),
        ),
        scale(input.body_a.torque_acceleration, input.time_step),
    );
    let local_angular_displacement =
        project(scale(relative_angular_rate, input.time_step), angular_axes);
    let angular_velocity_allowance = [
        parameters.twist_velocity_allowance,
        parameters.swing_velocity_allowance,
        parameters.swing_velocity_allowance,
    ];
    let mut angular_low = [0.0; 3];
    let mut angular_high = [0.0; 3];
    for lane in 0..3 {
        let predicted_low = angular_raw_low[lane] + local_angular_displacement[lane];
        let predicted_high = angular_raw_high[lane] + local_angular_displacement[lane];
        let velocity_low =
            local_angular_displacement[lane] - angular_velocity_allowance[lane] * input.time_step;
        let velocity_high = angular_velocity_allowance[lane]
            .mul_add(input.time_step, local_angular_displacement[lane]);
        let raw_low = vmx_max(predicted_low, vmx_min(velocity_low, predicted_high));
        let raw_high = vmx_min(predicted_high, vmx_max(velocity_high, predicted_low));
        angular_low[lane] = raw_low * angular_inverse_effective_mass[lane];
        angular_high[lane] = raw_high * angular_inverse_effective_mass[lane];
    }

    let linear_projection = scaled_projection_columns(linear_axes, linear_inverse_effective_mass);
    let angular_projection =
        scaled_projection_columns(angular_axes, angular_inverse_effective_mass);
    let inertia_columns_a = packed_inertia_columns(inertia_a, inverse_mass_a);
    let inertia_columns_b = packed_inertia_columns(inertia_b, inverse_mass_b);

    let mut output = RetailJointJacobian {
        words: [0; tu3::JACOBIAN_WORDS],
    };
    write_vector(
        &mut output,
        0,
        [
            arm_a.x,
            arm_a.y,
            arm_a.z,
            f32::from_bits(input.body_a.reaction_guest_address),
        ],
    );
    write_vector(
        &mut output,
        1,
        [
            arm_b.x,
            arm_b.y,
            arm_b.z,
            f32::from_bits(input.body_b.reaction_guest_address),
        ],
    );
    // Vectors 2 and 3 are the cleared linear/angular accumulators.
    write_vector(
        &mut output,
        4,
        [
            linear_projection[0][0],
            linear_projection[0][1],
            linear_projection[0][2],
            0.0,
        ],
    );
    write_vector(
        &mut output,
        5,
        [
            angular_projection[0][0],
            angular_projection[0][1],
            angular_projection[0][2],
            0.0,
        ],
    );
    write_vector(
        &mut output,
        6,
        [
            linear_projection[1][0],
            linear_projection[1][1],
            linear_projection[1][2],
            angular_low[0],
        ],
    );
    write_vector(
        &mut output,
        7,
        [
            angular_projection[1][0],
            angular_projection[1][1],
            angular_projection[1][2],
            angular_low[1],
        ],
    );
    write_vector(
        &mut output,
        8,
        [
            linear_projection[2][0],
            linear_projection[2][1],
            linear_projection[2][2],
            angular_high[0],
        ],
    );
    write_vector(
        &mut output,
        9,
        [
            angular_projection[2][0],
            angular_projection[2][1],
            angular_projection[2][2],
            angular_high[1],
        ],
    );
    write_vector(
        &mut output,
        10,
        [linear_low[0], linear_low[1], linear_low[2], angular_low[2]],
    );
    write_vector(
        &mut output,
        11,
        [
            linear_high[0],
            linear_high[1],
            linear_high[2],
            angular_high[2],
        ],
    );
    write_vector(
        &mut output,
        12,
        [
            linear_axes[0].x,
            linear_axes[0].y,
            linear_axes[0].z,
            f32::from_bits(input.joint_guest_address),
        ],
    );
    // Packed carry lanes remain unverified pending direct TU3 instruction review.
    write_vector(
        &mut output,
        13,
        [
            linear_axes[1].x,
            linear_axes[1].y,
            linear_axes[1].z,
            linear_axes[1].x,
        ],
    );
    write_vector(
        &mut output,
        14,
        [
            linear_axes[2].x,
            linear_axes[2].y,
            linear_axes[2].z,
            linear_axes[0].y,
        ],
    );
    for lane in 0..3 {
        write_vector(
            &mut output,
            15 + lane,
            [
                angular_axes[lane].x,
                angular_axes[lane].y,
                angular_axes[lane].z,
                angular_axes[lane].x,
            ],
        );
    }
    for lane in 0..3 {
        write_vector(&mut output, 18 + lane, inertia_columns_a[lane]);
        write_vector(&mut output, 21 + lane, inertia_columns_b[lane]);
    }
    output
}

fn write_vector(output: &mut RetailJointJacobian, vector: usize, value: [f32; 4]) {
    let start = vector * 4;
    for lane in 0..4 {
        output.words[start + lane] = value[lane].to_bits();
    }
}

#[cfg(test)]
#[path = "tests/joint_builder.rs"]
mod tests;
