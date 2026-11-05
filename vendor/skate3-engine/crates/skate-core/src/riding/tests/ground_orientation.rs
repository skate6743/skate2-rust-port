use super::*;
fn constant(value: f32) -> PointGraph<8> {
    PointGraph {
        x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        y: [value; 8],
    }
}
fn settings() -> GroundOrientationSettings {
    GroundOrientationSettings {
        ground_normal_smoothing: [1.0, 0.0, 0.0, 1.0],
        up_vector_smoothing_slow: [1.0, 0.0, 0.0, 1.0],
        up_vector_smoothing_fast: [1.0, 0.0, 0.0, 1.0],
        dynamic_up_vs_ground_y: constant(1.0),
        ground_vector_blend: constant(0.0),
        deck_angle_usage_vs_speed: constant(0.0),
        up_vector_smoothing_vs_speed: constant(0.0),
        up_vector_max_delta_vs_speed: constant(1.0),
        ground_blend_max_delta: 0.1,
        up_vector_max_acceleration: 2.0,
        anti_wobble_damping: 0.5,
        extra_side_damping: 0.6,
        minimum_wheels_for_ground_blend: 0,
    }
}
fn input(normal: Vector3) -> GroundOrientationInput {
    GroundOrientationInput {
        com_to_deck: UP,
        ground_normal: normal,
        dynamic_up: normal,
        speed: 0.0,
        wheel_contact_count: 4,
        animation_balance: 0.0,
        deck_angle_curve_input: 0.0,
        board_up: UP,
        board_forward: Vector3::new(0.0, 0.0, 1.0),
        effective_board_forward: Vector3::new(0.0, 0.0, 1.0),
        previous_reckoning_right: Vector3::new(1.0, 0.0, 0.0),
        prevent_up_behind_board: false,
    }
}
#[test]
fn stationary_filter_startup_has_no_false_tilt_or_acceleration() {
    let s = settings();
    let mut state = GroundOrientation::new(&s);
    for _ in 0..120 {
        state.update(&s, input(UP));
    }
    assert_eq!(state.up, UP);
    assert_eq!(state.ground_normal, UP);
    assert_eq!(state.up_velocity, Vector3::ZERO);
}
#[test]
fn changed_contact_normal_does_not_instantly_replace_the_skater_up() {
    let s = settings();
    let mut state = GroundOrientation::new(&s);
    let wall = Vector3::new(1.0, 0.0, 0.0);
    state.update(&s, input(wall));
    assert_eq!(state.ground_normal, wall);
    assert_ne!(state.up, state.ground_normal);
    assert!(state.up.y > 0.99 && state.up.x > 0.0 && state.up.x < 0.04);
    assert_eq!(
        &state.slow_filter.words()[4..8],
        &lanes(state.up).map(f32::to_bits)
    );
}
#[test]
fn landing_with_reported_tiny_up_residual_stays_finite() {
    let s = settings();
    // Exercise the recorded near-vertical orientation and its settled limit.
    for previous_up in [UP, Vector3::new(-1.6165965e-22, 1.0, -4.2145673e-22)] {
        let mut state = GroundOrientation::new(&s);
        state.up = previous_up;
        // Filter velocity is not in the report; exercise a small finite residual.
        state.up_velocity = Vector3::new(1.0e-22, 0.0, 2.0e-22);
        let mut landing = input(UP);
        landing.animation_balance = -0.36950988;
        landing.wheel_contact_count = 2;
        landing.speed = 3.4837468;
        landing.previous_reckoning_right = Vector3::new(-0.49704847, 2.853549e-22, 0.86772275);
        state.update(&s, landing);
        for v in [state.up, state.up_velocity, state.target] {
            assert!(
                v.x.is_finite() && v.y.is_finite() && v.z.is_finite(),
                "{v:?}"
            );
        }
        assert!((state.up.y - 1.0).abs() < 1e-6);
    }
}

#[test]
fn acceleration_clamp_preserves_tiny_finite_delta() {
    let delta = Vector3::new(1.6165965e-22, 0.0, 4.2145673e-22);
    assert_eq!(clamp_length(delta, 0.03), delta);
}
