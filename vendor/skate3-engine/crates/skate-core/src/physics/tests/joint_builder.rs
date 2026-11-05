use super::*;

#[test]
fn output_stride_is_the_observed_three_cache_lines() {
    assert_eq!(core::mem::size_of::<RetailJointJacobian>(), 0x180);
}

#[test]
fn inactive_body_contributes_no_mass_or_inertia() {
    // Synthetic identity bodies replace removed generated constructor output.
    let body = RetailJointBodyInput {
        reaction_guest_address: 0,
        state: 0,
        orientation: RetailQuaternion::IDENTITY,
        center_of_mass: Vector3::ZERO,
        basis: Basis3 {
            columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        linear_velocity: Vector3::ZERO,
        angular_velocity: Vector3::ZERO,
        force_acceleration: Vector3::ZERO,
        torque_acceleration: Vector3::ZERO,
        inverse_mass: 1.0,
        world_inverse_inertia: RetailPackedWorldInverseInertia {
            full: Vector3::new(1.0, 0.0, 0.0),
            split: Vector3::new(1.0, 1.0, 0.0),
        },
    };
    let mut frames = RetailJointFramesRaw { words: [0; 20] };
    for index in [3, 11, 19] {
        frames.words[index] = 1.0f32.to_bits();
    }
    let built = build_retail_joint_jacobian(RetailJointBuildInput {
        parameters: RetailJointParametersRaw { words: [0; 16] },
        frames,
        body_a: body,
        body_b: body,
        time_step: 0.25,
        joint_guest_address: 0,
    });
    assert_eq!(&built.words[84..96], &[0; 12]);
}

fn angular_parameters(swing_mode: u32, twist_mode: u32) -> JointParameters {
    JointParameters {
        linear_position_allowance: Vector3::ZERO,
        linear_velocity_allowance: Vector3::ZERO,
        twist_velocity_allowance: 0.0,
        swing_velocity_allowance: 0.0,
        swing_threshold: 0.5,
        twist_threshold: 0.5,
        swing_mode,
        twist_mode,
    }
}

fn identity_basis() -> Basis3 {
    Basis3 {
        columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    }
}

#[test]
fn angular_limit_zero_denominators_retain_native_intervals() {
    let basis = identity_basis();
    let relative = basis.columns;
    let rqd = create_rqd(RetailQuaternion::IDENTITY, RetailQuaternion::IDENTITY);
    for mode in [1, 2] {
        let (axes, low, high) =
            angular_rows(angular_parameters(mode, 1), basis, basis, relative, rqd);
        assert_eq!(axes, basis_columns(basis));
        // Original cone cosine guard, planar zero reciprocal guard and twist
        // zero reciprocal guard retain the initialized unlimited intervals.
        assert_eq!(low[0], -tu3::FINITE_INFINITY);
        assert_eq!(high[0], tu3::FINITE_INFINITY);
        assert_eq!(low[1], -tu3::FINITE_INFINITY);
        assert_eq!(high[1], tu3::FINITE_INFINITY);
        if mode == 2 {
            assert_eq!(low[2], 0.0);
            assert_eq!(high[2], 0.0);
        }
    }
}

#[test]
fn cone_inside_limit_still_publishes_predictive_bound() {
    let a = identity_basis();
    let b = Basis3 {
        columns: [[0.8, 0.6, 0.0], [-0.6, 0.8, 0.0], [0.0, 0.0, 1.0]],
    };
    let relative = [[0.8, -0.6, 0.0], [0.6, 0.8, 0.0], [0.0, 0.0, 1.0]];
    let (_, low, _) = angular_rows(
        angular_parameters(1, 2),
        a,
        b,
        relative,
        create_rqd(RetailQuaternion::IDENTITY, RetailQuaternion::IDENTITY),
    );
    // 82AE4238 writes (0.5-0.8)/0.6 even though the current angle is inside
    // the limit; the velocity bounds can then constrain this tick's motion.
    assert!((low[1] + 0.5).abs() < 1.0e-6);
}

#[test]
fn rqd_rows_are_bilinear_without_unit_quaternion_reconstruction() {
    let q = RetailQuaternion {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        w: 4.0,
    };
    let rqd = create_rqd(q, RetailQuaternion::IDENTITY);
    // Original 82AE0DB0's bilinear products reduce to these values when B=1.
    assert_eq!(
        rqd.axes,
        [
            Vector3::new(4.0, 3.0, -2.0),
            Vector3::new(-3.0, 4.0, 1.0),
            Vector3::new(2.0, -1.0, 4.0)
        ]
    );
    assert_eq!(
        rqd.relative,
        RetailQuaternion {
            x: -1.0,
            y: -2.0,
            z: -3.0,
            w: 4.0
        }
    );
}
