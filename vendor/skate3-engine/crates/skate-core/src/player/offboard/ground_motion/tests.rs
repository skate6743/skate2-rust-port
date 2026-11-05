use super::math::{DT, IDENTITY, UP, ZERO};
use super::*;
fn state() -> GroundMotionState {
    GroundMotionState {
        frame_0: IDENTITY,
        published_frame_64: IDENTITY,
        previous_support_frame_192: IDENTITY,
        support_velocity_256: ZERO,
        predicted_support_velocity_272: ZERO,
        previous_support_velocity_288: ZERO,
        support_acceleration_304: ZERO,
        filtered_local_acceleration_320: ZERO,
        support_yaw_336: 0.0,
        previous_support_yaw_340: 0.0,
        predicted_support_yaw_344: 0.0,
        support_speed_348: 0.0,
        support_id_352: 1,
        velocity_480: ZERO,
        correction_576: ZERO,
        target_frame_608: IDENTITY,
        angular_velocity_688: 0.0,
        speed_704: 0.0,
        correction_enabled_711: false,
        support_velocity_removed_715: false,
        target_scale_784: -1.0,
    }
}
fn input() -> GroundMotionInput {
    GroundMotionInput {
        contact_position_0: ZERO,
        contact_frame_32: IDENTITY,
        contact_target_96: ZERO,
        contact_normal_112: UP,
        contact_flags_176: 0,
        contact_id_180: 1,
        animation_motion_224: ZERO,
        animation_motion_240: ZERO,
        requested_duration_288: 1.0,
        mirrored_292: false,
        animation_directed_304: false,
        target_frame_present_352: false,
        target_frame_368: IDENTITY,
        reference_frame_128: IDENTITY,
        contact_displacement_384: ZERO,
        velocity_addition_528: ZERO,
        desired_up_544: UP,
        correction_target_592: ZERO,
        obstacle_target_672: ZERO,
        obstacle_enabled_713: false,
    }
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1.0e-5, "{a} != {b}");
}
#[test]
fn free_step_combines_all_actual_displacements() {
    let mut s = state();
    let mut i = input();
    s.frame_0[3] = [10.0, 2.0, 3.0, 0.0];
    s.velocity_480 = [6.0, 0.0, 0.0, 0.0];
    s.predicted_support_velocity_272 = [0.0, 0.0, 3.0, 0.0];
    i.velocity_addition_528 = [0.0, 12.0, 0.0, 0.0];
    i.contact_displacement_384 = [0.5, 0.0, 0.0, 0.0];
    update(&mut s, &i);
    near(s.frame_0[3][0], 10.6);
    near(s.frame_0[3][1], 2.2);
    near(s.frame_0[3][2], 3.05);
    assert_eq!(s.frame_0, s.published_frame_64);
}
#[test]
fn absent_contact_preserves_support_history() {
    let mut s = state();
    let i = input();
    s.support_velocity_256 = [1.0; 4];
    s.support_id_352 = 9;
    support::update(&mut s, &i);
    assert_eq!(s.support_velocity_256, [1.0; 4]);
    assert_eq!(s.support_id_352, 9);
}
#[test]
fn new_support_restores_removed_velocity_and_resets_derivatives() {
    let mut s = state();
    let mut i = input();
    i.contact_flags_176 = 1;
    i.contact_id_180 = 2;
    s.support_velocity_removed_715 = true;
    s.predicted_support_velocity_272 = [3.0, 0.0, 0.0, 0.0];
    s.velocity_480 = [1.0, 0.0, 0.0, 0.0];
    s.predicted_support_yaw_344 = 5.0;
    support::update(&mut s, &i);
    assert_eq!(s.support_id_352, 2);
    near(s.velocity_480[0], 4.0);
    near(s.speed_704, 4.0);
    assert!(!s.support_velocity_removed_715);
    assert_eq!(s.predicted_support_velocity_272, ZERO);
    assert_eq!(s.predicted_support_yaw_344, 0.0);
}
#[test]
fn translated_support_predicts_velocity_and_removes_it_once() {
    let mut s = state();
    let mut i = input();
    i.contact_flags_176 = 1;
    i.contact_frame_32[3][0] = DT;
    s.velocity_480 = [10.0, 0.0, 0.0, 0.0];
    support::update(&mut s, &i);
    near(s.support_velocity_256[0], 1.0);
    near(s.predicted_support_velocity_272[0], 2.0);
    near(s.velocity_480[0], 8.0);
    assert!(s.support_velocity_removed_715);
    i.contact_frame_32[3][0] = 2.0 * DT;
    support::update(&mut s, &i);
    near(s.predicted_support_velocity_272[0], 1.0);
    near(s.velocity_480[0], 8.0);
}
#[test]
fn rotated_support_produces_signed_yaw_history() {
    let mut s = state();
    let mut i = input();
    i.contact_flags_176 = 1;
    let angle = 0.1_f32;
    let (sin, cos) = angle.sin_cos();
    i.contact_frame_32[0] = [cos, 0.0, -sin, 0.0];
    i.contact_frame_32[2] = [sin, 0.0, cos, 0.0];
    support::update(&mut s, &i);
    near(s.support_yaw_336, angle * math::RATE);
    near(s.predicted_support_yaw_344, 2.0 * s.support_yaw_336);
}
#[test]
fn mirrored_support_acceleration_reverses_x_and_z_filter() {
    let mut a = state();
    let mut b = state();
    let mut i = input();
    i.contact_flags_176 = 1;
    i.contact_frame_32[3] = [DT, 0.0, DT, 0.0];
    support::update(&mut a, &i);
    i.mirrored_292 = true;
    support::update(&mut b, &i);
    near(
        a.filtered_local_acceleration_320[0],
        -b.filtered_local_acceleration_320[0],
    );
    near(
        a.filtered_local_acceleration_320[2],
        -b.filtered_local_acceleration_320[2],
    );
    assert_eq!(a.filtered_local_acceleration_320[1], 0.0);
}
#[test]
fn stationary_animation_reaches_target_at_requested_duration() {
    let mut s = state();
    let mut i = input();
    i.animation_directed_304 = true;
    i.requested_duration_288 = 2.0;
    s.target_frame_608[3] = [0.0, 0.0, 10.0, 0.0];
    let (step, _) = approach::calculate(&mut s, &i, ZERO, IDENTITY);
    near(s.target_scale_784, 5.0);
    near(step[2], 5.0 * DT);
}
#[test]
fn moving_animation_uses_current_motion_length_and_retains_scale() {
    let mut s = state();
    let mut i = input();
    i.animation_directed_304 = true;
    i.animation_motion_224 = [0.0, 0.0, 2.0, 0.0];
    i.animation_motion_240 = [0.0, 0.0, 3.0, 0.0];
    s.target_frame_608[3] = [0.0, 0.0, 10.0, 0.0];
    let (step, _) = approach::calculate(&mut s, &i, ZERO, IDENTITY);
    near(s.target_scale_784, 5.0);
    near(step[2], 15.0 * DT);
    i.animation_motion_224[2] = 4.0;
    approach::calculate(&mut s, &i, ZERO, IDENTITY);
    near(s.target_scale_784, 5.0);
}
#[test]
fn target_refresh_uses_right_offset_and_applies_support_delta_after_step() {
    let mut s = state();
    let mut i = input();
    i.animation_directed_304 = true;
    i.target_frame_present_352 = true;
    i.target_frame_368[3] = [0.0, 0.0, 3.0, 0.0];
    let mut delta = IDENTITY;
    delta[3] = [2.0, 0.0, 0.0, 0.0];
    approach::calculate(&mut s, &i, ZERO, delta);
    near(s.target_frame_608[3][0], 2.2);
    near(s.target_frame_608[3][2], 3.0);
}
#[test]
fn contact_height_adjustment_is_five_percent_along_current_up() {
    let mut s = state();
    let mut i = input();
    i.contact_flags_176 = 1;
    i.contact_position_0 = [0.0, 2.0, 0.0, 0.0];
    let (_, height) = approach::calculate(&mut s, &i, ZERO, IDENTITY);
    near(height[1], 0.1);
}
#[test]
fn obstacle_approach_caps_step_to_velocity_budget() {
    let mut s = state();
    let mut i = input();
    s.velocity_480 = [0.0, 0.0, 6.0, 0.0];
    i.obstacle_enabled_713 = true;
    i.obstacle_target_672 = [0.0, 0.0, 10.0, 0.0];
    let (step, _) = approach::calculate(&mut s, &i, ZERO, IDENTITY);
    near(step[2], 0.1);
}
#[test]
fn nearby_target_uses_remaining_tangent_budget() {
    let mut s = state();
    let mut i = input();
    s.velocity_480 = [0.0, 0.0, 6.0, 0.0];
    i.obstacle_enabled_713 = true;
    i.obstacle_target_672 = [0.0, 0.0, 0.05, 0.0];
    let (step, _) = approach::calculate(&mut s, &i, ZERO, IDENTITY);
    near(step[2], 0.1);
}
#[test]
fn contact_target_lateral_component_is_removed() {
    let mut s = state();
    let mut i = input();
    s.velocity_480 = [0.0, 0.0, 6.0, 0.0];
    i.contact_flags_176 = 2;
    i.contact_target_96 = [9.0, 0.0, 1.0, 0.0];
    let (step, _) = approach::calculate(&mut s, &i, ZERO, IDENTITY);
    near(step[0], 0.0);
    near(step[2], 0.1);
}
#[test]
fn publication_decays_correction_without_moving_physical_frame() {
    let mut s = state();
    s.correction_enabled_711 = true;
    s.correction_576 = [1.0, 0.0, 0.0, 0.0];
    publication::update(&mut s, ZERO);
    near(s.correction_576[0], 0.95);
    near(s.published_frame_64[3][0], -0.95);
    near(s.frame_0[3][0], 0.0);
    assert!(s.correction_enabled_711);
}
#[test]
fn publication_threshold_and_large_opposing_step_clear_correction() {
    let mut s = state();
    s.correction_enabled_711 = true;
    s.correction_576 = [0.01, 0.0, 0.0, 0.0];
    publication::update(&mut s, ZERO);
    assert!(!s.correction_enabled_711);
    s.correction_enabled_711 = true;
    s.correction_576 = [1.0, 0.0, 0.0, 0.0];
    publication::update(&mut s, [-2.0, 0.0, 0.0, 0.0]);
    assert_eq!(s.correction_576, ZERO);
    assert!(!s.correction_enabled_711);
}
#[test]
fn limiter_preserves_parallel_target_and_caps_oblique_normal() {
    assert_eq!(math::limit_angle(UP, UP, 0.5), UP);
    let v = math::limit_angle([1.0, 0.0, 0.0, 0.0], UP, std::f32::consts::FRAC_PI_4);
    near(v[0], std::f32::consts::FRAC_1_SQRT_2);
    near(v[1], std::f32::consts::FRAC_1_SQRT_2);
}

#[test]
fn limiter_keeps_quaternion_w_lane_fused_residuals() {
    let limit = 0.73;
    let from = [0.0, 1.0, 0.0, 0.37];
    let got = math::limit_angle([1.0, 0.0, 0.0, 0.0], from, limit);
    let (_, c) = crate::trigonometry::sin_cos(-limit * 0.5);
    let first_w = (-c).mul_add(from[3], c * from[3]);
    let middle_w = from[3].mul_add(c, first_w);
    let second_w = (-c).mul_add(middle_w, c * middle_w);
    let expected = second_w.mul_add(2.0, from[3]);
    assert_eq!(got[3].to_bits(), expected.to_bits());
}

/// Issue #10: a stray W lane in the ground frame position (native scratch
/// from an animation target or a collision error) must not be amplified by
/// the moving-support velocity derivative. Before the fix a 1e-3 seed reached
/// Inf after ~195 supported ticks and NaN the tick after.
#[test]
fn stray_position_w_is_not_amplified_by_support_velocity() {
    let mut s = state();
    let mut i = input();
    i.contact_flags_176 = 1;
    s.frame_0[3] = [1.0, 0.0, 2.0, 0.0];
    for tick in 0..600 {
        i.contact_displacement_384 = if tick == 5 {
            [0.0, 0.0, 0.0, 1.0e-3]
        } else {
            ZERO
        };
        update(&mut s, &i);
        assert_eq!(s.support_velocity_256[3], 0.0, "tick {tick}");
        assert_eq!(s.predicted_support_velocity_272[3], 0.0, "tick {tick}");
        assert!(
            s.frame_0[3][3].abs() <= 1.0e-3,
            "tick {tick}: W {}",
            s.frame_0[3][3]
        );
        assert!(s.frame_0[3].iter().all(|v| v.is_finite()), "tick {tick}");
    }
    assert_eq!(&s.frame_0[3][..3], &[1.0, 0.0, 2.0]);
}

/// Same loop seeded the way animation-directed movement can seed it: the
/// authored target frame's W lanes enter the step through target - position.
#[test]
fn animation_target_w_scratch_cannot_blow_up_the_ground_frame() {
    let mut s = state();
    let mut i = input();
    i.contact_flags_176 = 1;
    s.frame_0[3] = [1.0, 0.0, 2.0, 0.0];
    for tick in 0..600 {
        let directed = (5..15).contains(&tick);
        i.animation_directed_304 = directed;
        i.target_frame_present_352 = directed;
        i.target_frame_368 = IDENTITY;
        // Native permutation scratch: each row's X repeated into W.
        i.target_frame_368[0][3] = 1.0;
        i.target_frame_368[3] = [1.5, 0.0, 2.0, 1.5];
        update(&mut s, &i);
        assert!(
            s.frame_0[3].iter().all(|v| v.is_finite()),
            "tick {tick}: {:?}",
            s.frame_0[3]
        );
        assert!(
            s.frame_0[3][3].abs() < 10.0,
            "tick {tick}: W {}",
            s.frame_0[3][3]
        );
    }
}
