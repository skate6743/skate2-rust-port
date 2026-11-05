//! TU3 drive Jacobian construction, separate from iterative solving.
use super::*;
use crate::physics::{constraint_frames, native_arithmetic};
/// Scalar translation of TU3 `DriveJacobian::Build`.
///
/// RenderWare pairs frame A with internal `Drive::m_bodyA` at `Drive+0x10`
/// and frame B with `Drive::m_bodyB` at `Drive+0x14`. `Simulation::AddDrive`
/// stores its second body argument as internal A and its first as internal B.
/// The linear error is frame-B minus frame-A; the solver therefore applies
/// positive impulse to frame A and negative impulse to frame B.
pub fn build_drive_rows(
    frame_a_body: RetailDriveBodyState,
    frame_b_body: RetailDriveBodyState,
    frames: RetailDriveFrames,
    dynamics: RetailDriveDynamics,
    time_step: f32,
) -> RetailDriveRows {
    debug_assert!(time_step.is_finite() && time_step > 0.0);

    let frame_a_world = world_frame(frame_a_body, frames.body_a);
    let frame_b_world = world_frame(frame_b_body, frames.body_b);
    let arm_a = frame_a_world.arm;
    let arm_b = frame_b_world.arm;
    let linear_axes = basis_columns(constraint_frames::basis(frame_b_world.orientation));

    let raw_angular =
        constraint_frames::quaternion_rows(frame_a_world.orientation, frame_b_world.orientation);
    let components = [
        raw_angular.relative.x,
        raw_angular.relative.y,
        raw_angular.relative.z,
    ];
    let squared = components
        .map(|c| native_arithmetic::vector_min(native_arithmetic::vector_max(c * c, 0.0), 1.0));
    // 830BDB40 selects bytes [0,4,8,8] in every word: any saturated
    // quaternion component replaces ALL three axes with frame B's basis.
    let singular = squared.iter().any(|&c| c == 1.0);
    let inverse_lengths = if singular {
        [1.0; 3]
    } else {
        squared.map(|c| constraint_frames::reciprocal_sqrt(1.0 - c))
    };
    let angular_axes = core::array::from_fn(|i| {
        scale(
            if singular {
                linear_axes[i]
            } else {
                raw_angular.axes[i]
            },
            inverse_lengths[i],
        )
    });
    let angular_position_error =
        core::array::from_fn(|i| (components[i] * inverse_lengths[i]) * 2.0);

    let linear_position_error = sub(frame_b_world.position, frame_a_world.position);
    let linear_velocity_displacement = sub(
        scale(
            point_rate(
                frame_b_body.linear_velocity,
                frame_b_body.angular_velocity,
                arm_b,
            ),
            time_step,
        ),
        scale(
            point_rate(
                frame_a_body.linear_velocity,
                frame_a_body.angular_velocity,
                arm_a,
            ),
            time_step,
        ),
    );
    let linear_acceleration_displacement = sub(
        scale(
            scale(
                point_rate(
                    frame_b_body.force_acceleration,
                    frame_b_body.torque_acceleration,
                    arm_b,
                ),
                time_step,
            ),
            time_step,
        ),
        scale(
            scale(
                point_rate(
                    frame_a_body.force_acceleration,
                    frame_a_body.torque_acceleration,
                    arm_a,
                ),
                time_step,
            ),
            time_step,
        ),
    );
    let angular_velocity_displacement = scale(
        sub(frame_b_body.angular_velocity, frame_a_body.angular_velocity),
        time_step,
    );
    let angular_acceleration_displacement = scale(
        scale(
            sub(
                frame_b_body.torque_acceleration,
                frame_a_body.torque_acceleration,
            ),
            time_step,
        ),
        time_step,
    );
    let linear_inverse_effective_mass = core::array::from_fn(|index| {
        constraint_frames::reciprocal(linear_effective_mass(
            frame_a_body,
            frame_b_body,
            arm_a,
            arm_b,
            linear_axes[index],
        ))
    });
    let angular_inverse_effective_mass = core::array::from_fn(|index| {
        constraint_frames::reciprocal(angular_effective_mass(
            frame_a_body,
            frame_b_body,
            angular_axes[index],
        ))
    });

    let linear_coefficients = build_coefficients(
        dynamics.linear,
        components_of(linear_position_error),
        components_of(linear_velocity_displacement),
        components_of(linear_acceleration_displacement),
        time_step,
    );
    // Linear weights combine in world space before the projection (20A8..20D0).
    let displacement = linear_coefficients.target_displacement;
    let linear_target = project(
        Vector3::new(displacement[0], displacement[1], displacement[2]),
        linear_axes,
    );
    let angular_coefficients = build_coefficients(
        dynamics.angular,
        angular_position_error,
        project(angular_velocity_displacement, angular_axes),
        project(angular_acceleration_displacement, angular_axes),
        time_step,
    );
    RetailDriveRows {
        frame_a_body,
        frame_b_body,
        arm_a,
        arm_b,
        linear_axes,
        angular_axes,
        linear_inverse_effective_mass,
        angular_inverse_effective_mass,
        linear_softness: linear_coefficients.softness,
        angular_softness: angular_coefficients.softness,
        linear_target_impulse: core::array::from_fn(|i| {
            linear_target[i] * linear_inverse_effective_mass[i]
        }),
        angular_target_impulse: core::array::from_fn(|i| {
            angular_coefficients.target_displacement[i] * angular_inverse_effective_mass[i]
        }),
        linear_maximum_impulse: linear_inverse_effective_mass
            .map(|v| v * linear_coefficients.maximum_strength),
        angular_maximum_impulse: angular_inverse_effective_mass
            .map(|v| v * angular_coefficients.maximum_strength),
        accumulated_linear_impulse: [0.0; 3],
        accumulated_angular_impulse: [0.0; 3],
    }
}

#[derive(Clone, Copy)]
struct WorldFrame {
    orientation: RetailQuaternion,
    arm: Vector3,
    position: Vector3,
}

fn world_frame(body: RetailDriveBodyState, frame: RetailDriveFrame) -> WorldFrame {
    let arm = constraint_frames::transform_direction(body.basis, frame.translation);
    WorldFrame {
        orientation: constraint_frames::compose(body.orientation, frame.orientation),
        arm,
        position: add(body.center_of_mass, arm),
    }
}

#[derive(Clone, Copy)]
struct DriveCoefficients {
    softness: f32,
    target_displacement: [f32; 3],
    maximum_strength: f32,
}

fn components_of(v: Vector3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

/// 82AE1E68..20B8 and the matching angular block219C..237C.
/// Inputs are already scaled in their source order; the hard cap affects only
/// position error, before the rate and acceleration displacements are added.
fn build_coefficients(
    params: RetailDriveParams,
    mut position_error: [f32; 3],
    velocity_displacement: [f32; 3],
    acceleration_displacement: [f32; 3],
    time_step: f32,
) -> DriveCoefficients {
    let damping = time_step * params.damping;
    let (position_weight, velocity_weight, acceleration_weight) =
        if params.drive_type == RetailDriveType::SoftDrive {
            let spring = (time_step * params.spring_or_max_velocity) * time_step;
            let inverse = constraint_frames::reciprocal((1.0 + spring) + damping);
            (
                inverse * spring,
                inverse * damping,
                inverse * (spring + damping),
            )
        } else {
            (constraint_frames::reciprocal(1.0 + damping) * 1.0, 1.0, 1.0)
        };
    if params.drive_type == RetailDriveType::HardDrive {
        let squared = native_arithmetic::dot3(
            [position_error[0], position_error[1], position_error[2], 0.0],
            [position_error[0], position_error[1], position_error[2], 0.0],
        );
        let maximum = time_step * params.spring_or_max_velocity;
        if squared > maximum * maximum {
            let factor = maximum * constraint_frames::reciprocal_sqrt(squared);
            position_error = position_error.map(|v| v * factor);
        }
    }
    DriveCoefficients {
        softness: acceleration_weight,
        target_displacement: core::array::from_fn(|i| {
            let rate = velocity_displacement[i] * velocity_weight;
            let position_and_rate = position_error[i].mul_add(position_weight, rate);
            acceleration_displacement[i].mul_add(acceleration_weight, position_and_rate)
        }),
        // Type zero follows the same native strength load; its authored params
        // normally provide zero strength rather than the builder inventing it.
        maximum_strength: (time_step * params.max_strength) * time_step,
    }
}

fn linear_effective_mass(
    frame_a: RetailDriveBodyState,
    frame_b: RetailDriveBodyState,
    arm_a: Vector3,
    arm_b: Vector3,
    axis: Vector3,
) -> f32 {
    let mut denominator = 0.0;
    if is_active(frame_a) {
        let angular = cross(arm_a, axis);
        denominator += frame_a.inverse_mass
            + dot(
                angular,
                constraint_frames::multiply_inertia(frame_a.world_inverse_inertia, angular),
            );
    }
    if is_active(frame_b) {
        let angular = cross(arm_b, axis);
        denominator += frame_b.inverse_mass
            + dot(
                angular,
                constraint_frames::multiply_inertia(frame_b.world_inverse_inertia, angular),
            );
    }
    denominator
}

fn angular_effective_mass(
    frame_a: RetailDriveBodyState,
    frame_b: RetailDriveBodyState,
    axis: Vector3,
) -> f32 {
    let mut denominator = 0.0;
    if is_active(frame_a) {
        denominator += dot(
            axis,
            constraint_frames::multiply_inertia(frame_a.world_inverse_inertia, axis),
        );
    }
    if is_active(frame_b) {
        denominator += dot(
            axis,
            constraint_frames::multiply_inertia(frame_b.world_inverse_inertia, axis),
        );
    }
    denominator
}
