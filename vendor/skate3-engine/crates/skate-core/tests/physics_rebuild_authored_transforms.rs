//! Analytic geometry checks using real stock inputs and explicit alternatives.
//! No generated-code outputs or live/settled-pose fixtures are used.
use skate_core::{
    math::Vector3,
    physics::drive_frames::{
        AuthoredTransformInputs, RetailAffineTransform, authored_body_pose_records,
        authored_body_transforms, default_body_transforms,
    },
};

#[test]
fn stock_dimensions_place_all_seven_authored_parts() {
    let parts = default_body_transforms();
    // Decoded XML dimensions: mid length .59, wheel offset .095,
    // truck longitudinal correction -.052, vertical position -.0565.
    let expected = [
        [-0.095, -0.0565, 0.243],
        [0.095, -0.0565, 0.243],
        [-0.095, -0.0565, -0.243],
        [0.095, -0.0565, -0.243],
        [0.0, -0.0565, 0.243],
        [0.0, -0.0565, -0.243],
        [0.0; 3],
    ];
    for (part, expected) in parts.iter().zip(expected) {
        for (actual, expected) in [part.translation.x, part.translation.y, part.translation.z]
            .into_iter()
            .zip(expected)
        {
            assert!((actual - expected).abs() < 0.00000004);
        }
    }
    for part in &parts[..5] {
        assert_eq!(part.basis, RetailAffineTransform::IDENTITY.basis);
    }
    assert_eq!(parts[6], RetailAffineTransform::IDENTITY);
}

#[test]
fn asymmetric_inputs_keep_native_front_back_and_wheel_signs() {
    let parts = authored_body_transforms(AuthoredTransformInputs {
        deck_mid_length: 8.0,
        wheel_x_distance: 0.5,
        truck_z_position_front: -1.0,
        truck_z_position_back: -2.0,
        truck_y_position: -0.25,
    });
    assert_eq!(
        parts.map(|p| p.translation),
        [
            Vector3::new(-0.5, -0.25, 3.0),
            Vector3::new(0.5, -0.25, 3.0),
            Vector3::new(-0.5, -0.25, -2.0),
            Vector3::new(0.5, -0.25, -2.0),
            Vector3::new(0.0, -0.25, 3.0),
            Vector3::new(0.0, -0.25, -2.0),
            Vector3::ZERO,
        ]
    );
}

#[test]
fn negative_offsets_are_not_clamped_or_reordered() {
    let parts = authored_body_transforms(AuthoredTransformInputs {
        deck_mid_length: 2.0,
        wheel_x_distance: -0.5,
        truck_z_position_front: -3.0,
        truck_z_position_back: -4.0,
        truck_y_position: 0.25,
    });
    assert_eq!(parts[0].translation, Vector3::new(0.5, 0.25, -2.0));
    assert_eq!(parts[1].translation, Vector3::new(-0.5, 0.25, -2.0));
    assert_eq!(parts[2].translation, Vector3::new(0.5, 0.25, 3.0));
    assert_eq!(parts[3].translation, Vector3::new(-0.5, 0.25, 3.0));
}

#[test]
fn rear_truck_keeps_quarter_turn_polynomial_residual() {
    let basis = default_body_transforms()[5].basis.columns;
    // Analytic Y quarter-turn axes: -Z, +Y, +X. The literal angle is finite
    // pi/2 and the recovered polynomial retains a nonzero cosine residual.
    assert_eq!(basis[1], [0.0, 1.0, 0.0]);
    assert!(basis[0][0].abs() < 0.000001);
    assert_ne!(basis[0][0], 0.0);
    assert_eq!(basis[0][0].to_bits(), basis[2][2].to_bits());
    assert_eq!(basis[0][2].to_bits(), (-basis[2][0]).to_bits());
    assert!((basis[0][2] + 1.0).abs() < 0.000001);
    assert!((basis[2][0] - 1.0).abs() < 0.000001);
    assert_eq!(basis[0][1], 0.0);
    assert_eq!(basis[2][1], 0.0);
}

#[test]
fn reset_records_preserve_authored_axes_and_zero_carry_lanes() {
    let records = authored_body_pose_records(AuthoredTransformInputs::STOCK);
    let parts = default_body_transforms();
    for (record, part) in records.into_iter().zip(parts) {
        assert_eq!([record[3], record[7], record[11], record[15]], [0; 4]);
        for axis in 0..3 {
            assert_eq!(
                [record[axis * 4], record[axis * 4 + 1], record[axis * 4 + 2]],
                part.basis.columns[axis].map(f32::to_bits)
            );
        }
        assert_eq!(
            [record[12], record[13], record[14]],
            [
                part.translation.x.to_bits(),
                part.translation.y.to_bits(),
                part.translation.z.to_bits()
            ]
        );
    }
}

#[test]
fn dimensions_do_not_change_authored_body_orientations() {
    let stock = default_body_transforms();
    let alternate = authored_body_transforms(AuthoredTransformInputs {
        deck_mid_length: 1.25,
        wheel_x_distance: 0.25,
        truck_z_position_front: 0.0,
        truck_z_position_back: 0.0,
        truck_y_position: -0.125,
    });
    assert_eq!(stock.map(|p| p.basis), alternate.map(|p| p.basis));
    assert_ne!(stock[0].translation, alternate[0].translation);
}
