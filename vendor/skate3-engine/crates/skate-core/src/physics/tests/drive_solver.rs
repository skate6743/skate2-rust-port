use super::*;

const DT: f32 = f32::from_bits(0x3C88_8889);

fn active_body(reaction_index: usize) -> RetailDriveBodyState {
    RetailDriveBodyState {
        reaction_index,
        state: ACTIVE_BODY,
        orientation: RetailQuaternion::IDENTITY,
        basis: crate::math::Basis3 {
            columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        center_of_mass: Vector3::ZERO,
        linear_velocity: Vector3::ZERO,
        angular_velocity: Vector3::ZERO,
        force_acceleration: Vector3::new(0.0, -9.8, 0.0),
        torque_acceleration: Vector3::ZERO,
        inverse_mass: 1.0,
        world_inverse_inertia: RetailPackedWorldInverseInertia {
            full: Vector3::new(1.0, 0.0, 0.0),
            split: Vector3::new(1.0, 1.0, 0.0),
        },
    }
}

fn no_drive() -> RetailDriveParams {
    RetailDriveParams {
        spring_or_max_velocity: 0.0,
        damping: 0.0,
        max_strength: 0.0,
        drive_type: RetailDriveType::NoDrive,
    }
}

fn frames_with_translation(translation: Vector3) -> RetailDriveFrames {
    RetailDriveFrames {
        body_a: RetailDriveFrame {
            orientation: RetailQuaternion::IDENTITY,
            translation: Vector3::ZERO,
        },
        body_b: RetailDriveFrame {
            orientation: RetailQuaternion::IDENTITY,
            translation,
        },
    }
}

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-6,
        "actual={actual:?} expected={expected:?}"
    );
}

#[test]
fn rate_and_acceleration_terms_use_retail_dt_and_dt_squared() {
    let mut frame_b = active_body(1);
    frame_b.linear_velocity = Vector3::new(1.0, 0.0, 0.0);
    frame_b.force_acceleration = Vector3::new(1.0, -9.8, 0.0);
    let rows = build_drive_rows(
        active_body(0),
        frame_b,
        frames_with_translation(Vector3::ZERO),
        RetailDriveDynamics {
            linear: RetailDriveParams {
                spring_or_max_velocity: 100.0,
                damping: 0.0,
                max_strength: 100000.0,
                drive_type: RetailDriveType::HardDrive,
            },
            angular: no_drive(),
        },
        DT,
    );

    assert_near(rows.linear_target_impulse[0], 0.5 * (DT + DT * DT));
}
#[test]
fn off_center_linear_impulse_does_not_change_same_pass_angular_candidate() {
    let mut frames = frames_with_translation(Vector3::ZERO);
    frames.body_a.translation = Vector3::new(0.0, 1.0, 0.0);
    frames.body_b.translation = Vector3::new(0.0, 1.0, 0.0);
    let params = RetailDriveParams {
        spring_or_max_velocity: 100.0,
        damping: 0.0,
        max_strength: 100000.0,
        drive_type: RetailDriveType::HardDrive,
    };
    let mut rows = [build_drive_rows(
        active_body(0),
        active_body(1),
        frames,
        RetailDriveDynamics {
            linear: params,
            angular: params,
        },
        DT,
    )];
    rows[0].linear_target_impulse = [0.0, 0.0, 0.1];
    rows[0].angular_target_impulse = [0.0; 3];
    let mut reactions = [RetailReactionCorrections::default(); 2];
    solve_drive_iteration(&mut rows, &mut reactions);
    // 82AE2ED0 reads the angular difference BEFORE the linear reaction writes
    // at 82AE307C. The former port incorrectly canceled this induced torque.
    assert_eq!(rows[0].accumulated_angular_impulse, [0.0; 3]);
    assert!(reactions[0].angular_displacement.x > 0.0);
    assert!(reactions[1].angular_displacement.x < 0.0);
    solve_drive_iteration(&mut rows, &mut reactions);
    assert!(rows[0].accumulated_angular_impulse[0] < 0.0);
}

fn moving_drive(params: RetailDriveParams) -> RetailDriveRows {
    let mut b = active_body(1);
    b.center_of_mass = Vector3::new(2.0, 0.0, 0.0);
    b.linear_velocity = Vector3::new(2.0, 0.0, 0.0);
    b.force_acceleration.x = 4.0;
    build_drive_rows(
        active_body(0),
        b,
        frames_with_translation(Vector3::ZERO),
        RetailDriveDynamics {
            linear: params,
            angular: no_drive(),
        },
        0.5,
    )
}

#[test]
fn hard_drive_caps_position_before_adding_rate_and_acceleration() {
    // Original 82AE1F78..20D0: position 2 is capped to 1, then weighted by
    // 1/(1+c*dt)=1/2. Rate and acceleration each contribute 1 afterward.
    let rows = moving_drive(RetailDriveParams {
        spring_or_max_velocity: 2.0,
        damping: 2.0,
        max_strength: 16.0,
        drive_type: RetailDriveType::HardDrive,
    });
    assert_near(rows.linear_target_impulse[0], 1.25);
    assert_near(rows.linear_softness, 1.0);
    assert_near(rows.linear_maximum_impulse[0], 2.0);
}

#[test]
fn soft_drive_weights_spring_damping_and_acceleration_separately() {
    // Original 82AE1E68..1F30: k*dt^2=2, c*dt=1, denominator=4.
    // Position/rate/acceleration weights are 1/2, 1/4, 3/4 respectively.
    let rows = moving_drive(RetailDriveParams {
        spring_or_max_velocity: 8.0,
        damping: 2.0,
        max_strength: 16.0,
        drive_type: RetailDriveType::SoftDrive,
    });
    assert_near(rows.linear_target_impulse[0], 1.0);
    assert_near(rows.linear_softness, 0.75);
}

#[test]
fn saturated_quaternion_component_uses_all_frame_b_axes() {
    let mut b = active_body(1);
    b.orientation = RetailQuaternion {
        x: 1.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    b.basis.columns = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
    let rows = build_drive_rows(
        active_body(0),
        b,
        frames_with_translation(Vector3::ZERO),
        RetailDriveDynamics {
            linear: no_drive(),
            angular: RetailDriveParams {
                spring_or_max_velocity: 100.0,
                damping: 0.0,
                max_strength: 100.0,
                drive_type: RetailDriveType::HardDrive,
            },
        },
        0.5,
    );
    // Original 82AE20A0..216C uses one saturated component to select all axes.
    for i in 0..3 {
        let a = rows.angular_axes[i];
        assert_near(a.x, b.basis.columns[i][0]);
        assert_near(a.y, b.basis.columns[i][1]);
        assert_near(a.z, b.basis.columns[i][2]);
    }
    assert_near(rows.angular_target_impulse[0], 1.0);
    assert!(rows.angular_target_impulse.iter().all(|v| v.is_finite()));
}

#[test]
fn drive_anchor_uses_independent_body_basis() {
    let mut a = active_body(0);
    a.basis.columns = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
    let mut frames = frames_with_translation(Vector3::ZERO);
    frames.body_a.translation = Vector3::new(2.0, 0.0, 0.0);
    let rows = build_drive_rows(
        a,
        active_body(1),
        frames,
        RetailDriveDynamics {
            linear: no_drive(),
            angular: no_drive(),
        },
        DT,
    );
    // The body quaternion deliberately remains identity: the source reads
    // the independently stored Body+64/+80/+96 for this point transformation.
    assert_eq!(rows.arm_a, Vector3::new(0.0, 0.0, 2.0));
}
