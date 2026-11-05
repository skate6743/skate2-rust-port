use super::*;
use crate::physics::mass::*;

const IDENTITY: Basis3 = Basis3 {
    columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-6,
        "actual {actual}, expected {expected}"
    );
}

fn assert_vec_close(actual: Vector3, expected: Vector3) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.z, expected.z);
}

#[test]
fn recovered_deck_mass_frame_is_finite_right_handed_and_orthonormal() {
    let frame = retail_deck_mass_properties().local_mass_frame;
    let right = column(frame.basis, 0);
    let up = column(frame.basis, 1);
    let at = column(frame.basis, 2);
    for value in [
        right.x,
        right.y,
        right.z,
        up.x,
        up.y,
        up.z,
        at.x,
        at.y,
        at.z,
        frame.translation.x,
        frame.translation.y,
        frame.translation.z,
    ] {
        assert!(value.is_finite());
    }
    assert!((length_squared(right) - 1.0).abs() < 2.0e-5);
    assert!((length_squared(up) - 1.0).abs() < 2.0e-5);
    assert!((length_squared(at) - 1.0).abs() < 2.0e-5);
    assert!(dot(right, up).abs() < 2.0e-5);
    assert!(dot(right, at).abs() < 2.0e-5);
    assert!(dot(up, at).abs() < 2.0e-5);
    assert!(dot(cross(right, up), at) > 0.99998);
}

#[test]
fn force_accumulator_applies_inverse_mass_and_point_torque() {
    let output = accumulate_point_force(
        RetailForceAccumulator {
            force_acceleration: Vector3::new(1.0, 2.0, 3.0),
            torque_acceleration: Vector3::ZERO,
            cool_down: 7,
        },
        Vector3::new(0.0, 12.0, 0.0),
        Vector3::new(0.0, 0.0, 2.0),
        IDENTITY,
        0.5,
        IDENTITY,
    );
    assert_vec_close(output.force_acceleration, Vector3::new(1.0, 8.0, 3.0));
    assert_vec_close(output.torque_acceleration, Vector3::new(-24.0, 0.0, 0.0));
    assert_eq!(output.cool_down, 0);
}

#[test]
fn body_step_uses_solver_displacements_without_tangent_projection() {
    let simulation = RetailSimulationStep {
        time_step: 0.25,
        frequency: 4.0,
        cool_down: 8,
        minimum_energy: 0.0,
        gravity_acceleration: Vector3::new(0.0, -9.8, 0.0),
    };
    let output = integrate_body_rates(
        RetailBodyRates {
            orientation: RetailQuaternion::IDENTITY,
            basis: IDENTITY,
            world_inverse_inertia: IDENTITY,
            position: Vector3::new(1.0, 2.0, 3.0),
            linear_velocity: Vector3::new(4.0, 0.0, 0.0),
            angular_velocity: Vector3::new(0.0, 2.0, 0.0),
            force_acceleration: Vector3::new(0.0, -4.0, 0.0),
            torque_acceleration: Vector3::ZERO,
            kinetic_energy: 0.0,
            cool_down: 0,
        },
        RetailInertiaDynamics {
            inverse_tensor: Vector3::new(1.0, 1.0, 1.0),
            inverse_mass: 0.5,
            spherical: 1.0,
            maximum_linear_velocity: 100.0,
            maximum_angular_velocity: 100.0,
            linear_drag: 0.0,
            angular_drag: 0.0,
        },
        simulation,
        RetailReactionCorrections {
            linear_displacement: Vector3::new(0.0, 0.25, 0.0),
            position_displacement: Vector3::new(0.0, 0.0, 0.5),
            angular_displacement: Vector3::new(0.0, 0.0, 0.25),
            orientation_displacement: Vector3::new(0.125, 0.0, 0.0),
        },
    );

    assert_vec_close(output.state.position, Vector3::new(2.0, 2.0, 3.5));
    assert_vec_close(output.state.linear_velocity, Vector3::new(4.0, 0.0, 0.0));
    assert_vec_close(output.state.angular_velocity, Vector3::new(0.0, 2.0, 1.0));
    assert_vec_close(
        output.orientation_displacement,
        Vector3::new(0.125, 0.5, 0.25),
    );
    assert_vec_close(
        output.state.force_acceleration,
        simulation.gravity_acceleration,
    );
    assert_vec_close(output.state.torque_acceleration, Vector3::ZERO);
    assert_eq!(
        output.state.orientation,
        integrate_orientation(RetailQuaternion::IDENTITY, output.orientation_displacement)
    );
}

#[test]
fn drag_is_frequency_minus_drag_on_frame_displacement() {
    let output = integrate_body_rates(
        RetailBodyRates {
            orientation: RetailQuaternion::IDENTITY,
            basis: IDENTITY,
            world_inverse_inertia: IDENTITY,
            position: Vector3::ZERO,
            linear_velocity: Vector3::new(8.0, 0.0, 0.0),
            angular_velocity: Vector3::new(0.0, 6.0, 0.0),
            force_acceleration: Vector3::ZERO,
            torque_acceleration: Vector3::ZERO,
            kinetic_energy: 100.0,
            cool_down: 0,
        },
        RetailInertiaDynamics {
            inverse_tensor: Vector3::new(1.0, 1.0, 1.0),
            inverse_mass: 1.0,
            spherical: 1.0,
            maximum_linear_velocity: 100.0,
            maximum_angular_velocity: 100.0,
            linear_drag: 2.0,
            angular_drag: 3.0,
        },
        RetailSimulationStep {
            time_step: 0.25,
            frequency: 4.0,
            cool_down: 8,
            minimum_energy: 0.0,
            gravity_acceleration: Vector3::ZERO,
        },
        RetailReactionCorrections::default(),
    );
    assert_vec_close(output.state.linear_velocity, Vector3::new(4.0, 0.0, 0.0));
    assert_vec_close(output.state.angular_velocity, Vector3::new(0.0, 1.5, 0.0));
}

#[test]
fn speed_caps_and_retail_cool_down_branch_are_separate() {
    let mut body = RetailBodyRates {
        orientation: RetailQuaternion::IDENTITY,
        basis: IDENTITY,
        world_inverse_inertia: IDENTITY,
        position: Vector3::ZERO,
        linear_velocity: Vector3::new(10.0, 0.0, 0.0),
        angular_velocity: Vector3::new(0.0, 8.0, 0.0),
        force_acceleration: Vector3::ZERO,
        torque_acceleration: Vector3::ZERO,
        kinetic_energy: 200.0,
        cool_down: 2,
    };
    let inertia = RetailInertiaDynamics {
        inverse_tensor: Vector3::new(1.0, 1.0, 1.0),
        inverse_mass: 0.5,
        spherical: 2.0,
        maximum_linear_velocity: 3.0,
        maximum_angular_velocity: 2.0,
        linear_drag: 0.0,
        angular_drag: 0.0,
    };
    let simulation = RetailSimulationStep {
        time_step: 0.25,
        frequency: 4.0,
        cool_down: 4,
        minimum_energy: 20.0,
        gravity_acceleration: Vector3::ZERO,
    };
    let first = integrate_body_rates(
        body,
        inertia,
        simulation,
        RetailReactionCorrections::default(),
    );
    assert_close(first.state.linear_velocity.x, 3.0);
    assert_close(first.state.angular_velocity.y, 2.0);
    assert_close(first.state.kinetic_energy, 13.0);
    assert_eq!(first.state.cool_down, 3);

    body = first.state;
    let second = integrate_body_rates(
        body,
        inertia,
        simulation,
        RetailReactionCorrections::default(),
    );
    assert_eq!(second.state.cool_down, 4);
}

#[test]
fn quaternion_branch_uses_retail_left_multiplication_and_normalizes() {
    let quarter_turn_y = RetailQuaternion {
        x: 0.0,
        y: core::f32::consts::FRAC_1_SQRT_2,
        z: 0.0,
        w: core::f32::consts::FRAC_1_SQRT_2,
    };
    let angular = Vector3::new(0.2, 0.3, 0.4);
    let actual = integrate_orientation(quarter_turn_y, angular);

    let old_vector = Vector3::new(quarter_turn_y.x, quarter_turn_y.y, quarter_turn_y.z);
    let left_product_vector = add(scale(angular, quarter_turn_y.w), cross(angular, old_vector));
    let expected = normalize_quaternion(RetailQuaternion {
        x: quarter_turn_y.x + 0.5 * left_product_vector.x,
        y: quarter_turn_y.y + 0.5 * left_product_vector.y,
        z: quarter_turn_y.z + 0.5 * left_product_vector.z,
        w: quarter_turn_y.w - 0.5 * dot(angular, old_vector),
    });
    assert_eq!(actual, expected);
    assert_close(
        actual.x * actual.x + actual.y * actual.y + actual.z * actual.z + actual.w * actual.w,
        1.0,
    );
}

#[test]
fn quaternion_basis_and_world_inverse_inertia_preserve_body_axes() {
    let quarter_turn_y = RetailQuaternion {
        x: 0.0,
        y: core::f32::consts::FRAC_1_SQRT_2,
        z: 0.0,
        w: core::f32::consts::FRAC_1_SQRT_2,
    };
    let basis = basis_from_quaternion(quarter_turn_y);
    assert_vec_close(column(basis, 0), Vector3::new(0.0, 0.0, -1.0));
    assert_vec_close(column(basis, 1), Vector3::new(0.0, 1.0, 0.0));
    assert_vec_close(column(basis, 2), Vector3::new(1.0, 0.0, 0.0));

    let world = world_inverse_inertia(basis, Vector3::new(2.0, 3.0, 5.0));
    assert_vec_close(column(world, 0), Vector3::new(5.0, 0.0, 0.0));
    assert_vec_close(column(world, 1), Vector3::new(0.0, 3.0, 0.0));
    assert_vec_close(column(world, 2), Vector3::new(0.0, 0.0, 2.0));
}

#[test]
fn packed_world_inverse_inertia_uses_retail_lane_order() {
    let symmetric = Basis3 {
        columns: [[2.0, 3.0, 5.0], [3.0, 7.0, 11.0], [5.0, 11.0, 13.0]],
    };
    let packed = pack_world_inverse_inertia(symmetric);
    assert_eq!(packed.full, Vector3::new(2.0, 3.0, 5.0));
    assert_eq!(packed.split, Vector3::new(13.0, 7.0, 11.0));
    assert_vec_close(
        multiply_packed_world_inverse_inertia(packed, Vector3::new(17.0, 19.0, 23.0)),
        multiply_basis(symmetric, Vector3::new(17.0, 19.0, 23.0)),
    );
}
