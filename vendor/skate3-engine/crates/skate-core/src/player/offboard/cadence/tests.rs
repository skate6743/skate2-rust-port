use super::*;

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 2.0e-6, "{actual} != {expected}");
}
fn input() -> CadenceInput {
    CadenceInput {
        motion_512: [0.0, 0.0, 1.0],
        motion_reference_272: [0.0; 3],
        reject_axis_400: [0.0, 0.0, 1.0],
        reject_enabled_708: false,
        up_144: [0.0, 1.0, 0.0],
        frame_rows_0_16_32: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        frame_position_48: [0.0; 3],
        animation_motion_224: [0.0, 0.0, 1.0],
        requested_duration_288: 0.0,
        requested_phase_296: -1.0,
        suppress_adjustment_353: false,
        contact_flags_176: 0,
        contact_point_96: [0.0, 0.0, 1.2],
    }
}
fn thresholds() -> CadenceThresholds {
    CadenceThresholds::from_clip_speeds(2.0, 4.0, 6.0)
}

#[test]
fn stock_initialization_and_clip_thresholds() {
    let c = BipedCadence::default();
    assert_eq!(c.phase.duration, None);
    assert_eq!(c.phase.target, -1.0);
    assert_eq!(thresholds().0, [0.01, 3.0, 5.0, 1.0e10]);
}

#[test]
fn moving_rate_uses_physical_motion_relative_to_animation() {
    let mut c = BipedCadence::default();
    let mut i = input();
    i.motion_512[2] = 4.0;
    i.motion_reference_272[2] = 1.0;
    i.animation_motion_224[2] = 2.0;
    c.update(&i, thresholds());
    assert_eq!(c.locomotion_index, 1); // Exact threshold chooses lower bucket.
    close(c.phase.rate, 1.5);
    close(c.phase.phase, 0.025);
}

#[test]
fn movement_beyond_every_threshold_retains_previous_locomotion() {
    let mut c = BipedCadence {
        locomotion_index: 3,
        ..Default::default()
    };
    c.update(&input(), CadenceThresholds([0.0; 4]));
    assert_eq!(c.locomotion_index, 3);
}

#[test]
fn local_motion_keeps_vertical_component_for_cadence() {
    let mut c = BipedCadence::default();
    let mut i = input();
    i.motion_512 = [2.0, 3.0, 0.0];
    i.frame_rows_0_16_32 = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
    c.update(&i, thresholds());
    close(c.phase.rate, 13.0f32.sqrt());
    assert_eq!(c.locomotion_index, 1);
}

#[test]
fn contact_rejection_can_stop_locomotion_without_overwriting_animation() {
    let mut c = BipedCadence::default();
    c.phase.phase = 0.25;
    let mut i = input();
    i.reject_enabled_708 = true;
    c.update(&i, thresholds());
    assert_eq!(c.locomotion_index, 0);
    assert_eq!(c.phase.target, 0.0); // Equidistant stop targets prefer first.
    assert_eq!(c.phase.duration, Some(0.2));
    close(c.phase.rate, 1.25);
    close(c.phase.phase, 0.25 - 1.25 / 60.0);
}

#[test]
fn repeated_explicit_target_does_not_recompute_rate_each_tick() {
    let mut c = BipedCadence::default();
    c.phase.phase = 0.9;
    let mut i = input();
    i.requested_phase_296 = 0.1;
    i.requested_duration_288 = 0.3;
    c.update(&i, thresholds());
    let rate = c.phase.rate;
    close(rate, 1.0);
    c.update(&i, thresholds());
    assert_eq!(c.phase.rate, rate);
    close(c.phase.phase, 0.9 + 2.0 / 60.0);
}

#[test]
fn forward_snap_preserves_one_endpoint_and_free_motion_wraps() {
    let mut phase = BipedPhase {
        phase: 0.99,
        rate: 2.0,
        target: 1.0,
        forward_target: true,
        ..Default::default()
    };
    phase.advance();
    assert_eq!(phase.phase, 1.0);
    phase.advance();
    assert_eq!(phase.phase, 1.0);
    phase.target = -1.0;
    phase.advance();
    close(phase.phase, 2.0 / 60.0);
}

#[test]
fn closed_wrap_endpoints_are_not_euclidean_remainder() {
    assert_eq!(wrap(0.0, 1.0, 1.0), 1.0);
    assert_eq!(wrap(0.0, -1.0, 1.0), 1.0);
    assert_eq!(wrap(0.0, 2.0, 1.0), 0.0);
    assert_eq!(wrap(-0.5, 0.5, 0.5), 0.5);
    assert_eq!(wrap(-0.5, -0.5, 0.5), -0.5);
}

#[test]
fn nearest_target_can_advance_backwards_across_zero() {
    let mut phase = BipedPhase {
        phase: 0.01,
        rate: 0.3,
        target: 0.99,
        ..Default::default()
    };
    phase.advance();
    close(phase.phase, 0.005);
}

#[test]
fn obstacle_target_interval_preserves_authored_boundary_ties() {
    for (phase, expected_rate) in [(0.37, 0.03), (0.4, 0.5), (0.87, 0.03), (0.88, 0.52)] {
        let mut c = BipedCadence::default();
        c.phase.phase = phase;
        let mut i = input();
        i.contact_flags_176 = 0x100;
        c.update(&i, thresholds());
        close(c.phase.rate, expected_rate);
        assert_eq!(c.phase.target, -1.0);
    }
}

#[test]
fn obstacle_contact_chooses_rate_near_current_and_suppression_skips_it() {
    let mut c = BipedCadence::default();
    let mut i = input();
    i.contact_flags_176 = 0x80;
    i.contact_point_96[2] = 0.7; // Half second to contact after0.2 clearance.
    c.update(&i, thresholds());
    close(c.phase.rate, 0.9);
    i.suppress_adjustment_353 = true;
    c.update(&i, thresholds());
    close(c.phase.rate, 1.0);
}

#[test]
fn rate_selection_uses_inclusive_limits_and_clamps_nearest_candidate() {
    assert_eq!(select_rate(1.0, 2.0, 1.0, 2.0, 2.0), 2.0);
    assert_eq!(select_rate(0.0, 2.0, 1.0, 2.0, 0.0), 2.0);
    assert_eq!(select_rate(0.0, 4.0, 1.0, 3.0, 0.0), 3.0); // Equal distances prefer B.
    assert_eq!(select_rate(0.5, 5.0, 1.0, 3.0, 0.0), 1.0);
}

#[test]
fn no_animation_motion_gives_zero_rate_without_fabricated_clip_speed() {
    let mut c = BipedCadence::default();
    let mut i = input();
    i.animation_motion_224 = [0.0; 3];
    c.update(&i, thresholds());
    assert_eq!(c.phase.rate, 0.0);
    assert_eq!(c.phase.phase, 0.0);
    assert_eq!(c.locomotion_index, 1);
}
