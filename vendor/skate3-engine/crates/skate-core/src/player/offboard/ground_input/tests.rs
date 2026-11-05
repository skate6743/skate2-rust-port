use super::*;

fn input() -> GroundInput {
    GroundInput {
        processed_flags_2472: 0,
        processed_direct_2684: 9.0,
        processed_direct_2680: -7.0,
        processed_stick_2692: 0.0,
        processed_stick_2688: 1.0,
        processed_scale_2912: 1.0,
        processed_scale_2908: 1.0,
        frame_forward_112: [0.0, 0.0, 1.0],
    }
}

fn constant(value: f32) -> PointGraph<8> {
    PointGraph {
        x: [0.0, 0.1, 0.2, 0.3, 0.4, 0.6, 0.8, 1.0],
        y: [value; 8],
    }
}

fn angle_curve() -> PointGraph<8> {
    let x = [0.0, 0.1, 0.2, 0.3, 0.4, 0.6, 0.8, 1.0];
    PointGraph { x, y: x }
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 2.0e-6, "{actual} != {expected}");
}

#[test]
fn direct_flag_bypasses_invalid_geometry_and_scaling() {
    let mut i = input();
    i.processed_flags_2472 = 0x1000_0000;
    i.frame_forward_112 = [f32::NAN; 3];
    i.processed_scale_2912 = f32::NAN;
    i.processed_stick_2692 = f32::NAN;
    let output = calculate(&i, &constant(f32::NAN), &constant(f32::NAN));
    assert_eq!(
        output,
        GroundInputOutput {
            state_708: 9.0,
            state_712: -7.0,
            state_656: [0.0; 4]
        }
    );
}

#[test]
fn other_flag_does_not_select_direct_input() {
    let mut i = input();
    i.processed_flags_2472 = 0x0800_0000;
    near(calculate(&i, &constant(2.0), &constant(1.0)).state_708, 2.0);
}

#[test]
fn camera_relative_quadrants_and_output_raw_vector() {
    let mut i = input();
    i.processed_stick_2692 = 0.6;
    i.processed_stick_2688 = 0.8;
    let right = calculate(&i, &constant(1.0), &constant(2.0));
    near(right.state_708, 1.0);
    near(right.state_712, 2.0);
    assert_eq!(right.state_656, [0.6, 0.0, 0.8, 0.0]);
    i.processed_stick_2692 = -0.6;
    near(
        calculate(&i, &constant(1.0), &constant(2.0)).state_712,
        -2.0,
    );
}

#[test]
fn slope_is_flattened_before_input_transform() {
    let base = calculate(&input(), &angle_curve(), &constant(1.0));
    let mut i = input();
    i.frame_forward_112 = [0.0, 0.6, 0.8];
    let slope = calculate(&i, &angle_curve(), &constant(1.0));
    near(slope.state_708, base.state_708);
    near(slope.state_712, base.state_712);
}

#[test]
fn rotated_frame_changes_angle_but_not_stored_input() {
    let mut i = input();
    i.frame_forward_112 = [1.0, 0.0, 0.0];
    let result = calculate(&i, &angle_curve(), &constant(1.0));
    near(result.state_708, 0.5);
    near(result.state_712, -1.0);
    assert_eq!(result.state_656, [0.0, 0.0, 1.0, 0.0]);
}

#[test]
fn threshold_equality_uses_identity_and_below_uses_frame() {
    let mut i = input();
    i.frame_forward_112 = [0.1, f32::from_bits(0x3f7f_be77), 0.0];
    near(calculate(&i, &angle_curve(), &constant(1.0)).state_708, 1.0);
    i.frame_forward_112[1] = f32::from_bits(0x3f7f_be76);
    near(calculate(&i, &angle_curve(), &constant(1.0)).state_708, 0.5);
}

#[test]
fn positive_vertical_identity_but_negative_vertical_has_no_invented_guard() {
    let mut i = input();
    i.frame_forward_112 = [0.0, 1.0, 0.0];
    near(calculate(&i, &constant(1.0), &constant(1.0)).state_708, 1.0);
    i.frame_forward_112[1] = -1.0;
    assert!(
        calculate(&i, &constant(1.0), &constant(1.0))
            .state_708
            .is_nan()
    );
}

#[test]
fn zero_stick_is_zero_despite_intermediate_reciprocal_nan() {
    let mut i = input();
    i.processed_stick_2688 = 0.0;
    let result = calculate(&i, &constant(3.0), &constant(4.0));
    assert_eq!(result.state_708, 0.0);
    assert_eq!(result.state_712, 0.0);
}

#[test]
fn magnitude_curve_and_processed_scales_all_contribute() {
    let mut i = input();
    i.processed_stick_2692 = -3.0;
    i.processed_stick_2688 = 4.0;
    i.processed_scale_2912 = 2.0;
    i.processed_scale_2908 = 3.0;
    let result = calculate(&i, &constant(0.5), &constant(2.0));
    near(result.state_708, 5.0);
    near(result.state_712, -30.0);
}

#[test]
fn shared_curve_exact_endpoints_knots_and_unordered_input() {
    let curve = PointGraph {
        x: [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0],
        y: [2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 18.0],
    };
    for index in 0..8 {
        assert_eq!(curve.evaluate(curve.x[index]), curve.y[index]);
    }
    assert_eq!(curve.evaluate(-1.0), 2.0);
    assert_eq!(curve.evaluate(2.0), 18.0);
    assert_eq!(curve.evaluate(f32::NAN), 18.0);
    assert_eq!(curve.evaluate(0.0625), 3.0);
}

#[test]
fn original_angle_tracks_finite_atan_across_reduction_boundaries() {
    for boundary in [
        f32::from_bits(0x3980_0000),
        f32::from_bits(0x3e89_30a3),
        1.0,
    ] {
        for bits in [
            boundary.to_bits() - 1,
            boundary.to_bits(),
            boundary.to_bits() + 1,
        ] {
            let value = f32::from_bits(bits);
            near(math::rational_atan(value), value.atan());
            near(math::rational_atan(-value), (-value).atan());
        }
    }
    for value in [0.0_f32, 0.1, 0.5, 2.0, 10.0, 1.0e30] {
        near(math::rational_atan(value), value.atan());
    }
}

#[test]
fn axis_override_preserves_x_sign_even_when_both_inputs_are_zero() {
    assert_eq!(math::signed_angle(0.0, 0.0).to_bits(), 0x3fc9_0fdb);
    assert_eq!(math::signed_angle(-0.0, -0.0).to_bits(), 0xbfc9_0fdb);
    near(
        math::signed_angle(1.0, -1.0),
        3.0 * std::f32::consts::FRAC_PI_4,
    );
    near(
        math::signed_angle(-1.0, -1.0),
        -3.0 * std::f32::consts::FRAC_PI_4,
    );
}
