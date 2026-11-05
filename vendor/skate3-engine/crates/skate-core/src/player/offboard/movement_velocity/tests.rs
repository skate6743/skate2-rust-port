use super::*;

#[test]
fn tiny_velocity_delta_remains_finite_and_reaches_target() {
    // Flat-ground convergence can leave a tiny vertical correction while X/Z
    // already match. Its squared length is positive but subnormal.
    let current = [-1.6481168, 1.0e-21, 5.7692037, 0.0];
    let target = [-1.6481168, 0.0, 5.7692037, 0.0];
    let delta = sub(target, current);
    assert!(dot(delta, delta).is_subnormal());
    let result = limit_delta(target, current, 0.2);
    assert_eq!(result, target);
    assert!(length(delta).is_finite());
    assert!((length(delta) / 1.0e-21 - 1.0).abs() < 0.001);
}

#[test]
fn tiny_velocity_refinement_boundaries_preserve_limits_and_invalid_inputs() {
    for exponent in -74..=-60 {
        let component = 2.0f32.powi(exponent);
        let delta = [component, 0.0, 0.0, 0.0];
        let measured = length(delta);
        assert!(measured.is_finite() && measured > 0.0);
        assert!((measured / component - 1.0).abs() < 1e-6);
        let maximum = component * 0.5;
        let limited = limit_delta(delta, [0.0; 4], maximum);
        assert!(limited.into_iter().all(f32::is_finite));
        assert!((limited[0] / maximum - 1.0).abs() < 1e-6);
    }
    assert_eq!(length([0.0; 4]), 0.0);
    assert!(length([f32::NAN, 0.0, 0.0, 0.0]).is_nan());
    assert!(limit_delta([f32::NAN, 0.0, 0.0, 0.0], [0.0; 4], 0.2)[0].is_nan());
}

#[test]
fn tiny_velocity_correction_does_not_poison_walking_state() {
    let mut state = State {
        velocity: [0.0, 1.0e-21, 5.0, 0.0],
        speed: 5.0,
        ..State::default()
    };
    let mut input = input();
    input.steering = 0.0;
    for _ in 0..120 {
        state.update(&settings(), &input);
        assert_eq!(state.velocity, [0.0, 0.0, 5.0, 0.0]);
        assert_eq!(state.speed, 5.0);
        assert!(state.forward_delta.is_finite() && state.right_delta.is_finite());
    }
}
fn curve(value: f32) -> PointGraph<8> {
    PointGraph {
        x: std::array::from_fn(|i| i as f32),
        y: [value; 8],
    }
}
fn settings() -> Settings {
    Settings {
        slope_speed_scalar: curve(1.0),
        slope_mode_speed: curve(3.0),
        turn_vs_speed: curve(90.0),
        turn_delta_vs_speed: curve(15.0),
    }
}
fn input() -> Input {
    Input {
        forward: [0.0, 0.0, 1.0, 0.0],
        right: [1.0, 0.0, 0.0, 0.0],
        plane_normal: [0.0, 1.0, 0.0, 0.0],
        desired_speed: 5.0,
        steering: 1.0,
        slope_mode: false,
        obstacle: false,
        obstacle_normal: [0.0, 0.0, -1.0, 0.0],
        override_gate: -1.0,
        override_duration: 1.0,
        override_velocity: [0.0; 4],
        flags: 0,
    }
}
#[test]
fn desired_forward_delta_differs_from_limited_velocity() {
    let mut state = State::default();
    state.update(&settings(), &input());
    assert_eq!(state.forward_delta, 5.0);
    assert!((state.velocity[2] - 0.2).abs() < 0.000001);
    assert_eq!(state.right_delta, 0.0);
    assert_eq!(state.turn, 15.0 * f32::from_bits(0x3c8e_fa35));
    assert_eq!(state.override_remaining, -1.0);
}
#[test]
fn slope_mode_uses_its_own_speed_and_ignores_override() {
    let mut state = State::default();
    let mut input = input();
    input.slope_mode = true;
    input.override_gate = 0.0;
    input.override_velocity = [20.0, 0.0, 0.0, 0.0];
    state.update(&settings(), &input);
    assert_eq!(state.forward_delta, 3.0);
    assert!((state.speed - 0.5).abs() < 0.000001);
    assert_eq!(state.override_remaining, -1.0);
}
#[test]
fn override_first_tick_retains_old_velocity_then_fast_target_reduces_blend() {
    let mut state = State::default();
    let mut input = input();
    input.override_gate = 0.0;
    input.override_velocity = [0.0, 0.0, 10.0, 0.0];
    state.update(&settings(), &input);
    assert_eq!(state.velocity, [0.0; 4]);
    assert!(state.override_remaining < 1.0);
    state.update(&settings(), &input);
    assert_eq!(state.velocity, [0.0, 0.0, 5.0, 0.0]);
}
#[test]
fn minimum_motion_override_uses_projected_forward() {
    let mut state = State::default();
    let mut input = input();
    input.override_gate = 0.0;
    input.flags = 8;
    state.update(&settings(), &input);
    assert_eq!(state.velocity, [0.0, 0.0, 1.0, 0.0]);
}
#[test]
fn obstacle_projects_desired_and_limited_velocity_independently() {
    let mut state = State::default();
    let mut input = input();
    input.obstacle = true;
    state.update(&settings(), &input);
    assert_eq!(state.velocity, [0.0; 4]);
    assert_eq!(state.forward_delta, 0.0);
}
#[test]
fn nonunit_projection_subtracts_original_direction_and_zero_is_safe() {
    assert_eq!(
        remove_positive([2.0, 0.0, 0.0, 0.0], [2.0, 0.0, 0.0, 0.0]),
        [-2.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(
        remove_positive([-2.0, 0.0, 0.0, 0.0], [2.0, 0.0, 0.0, 0.0]),
        [-2.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(
        remove_positive([2.0, 0.0, 0.0, 0.0], [0.0; 4]),
        [2.0, 0.0, 0.0, 0.0]
    );
    assert_eq!(limit_delta([0.0; 4], [0.0; 4], 0.2), [0.0; 4]);
}

#[test]
fn upward_slowdown_retains_gravity_even_beyond_two_per_second_bound() {
    let mut state = State {
        velocity: [0.0, 6.0, 8.0, 0.0],
        speed: 10.0,
        ..State::default()
    };
    let mut input = input();
    input.forward = [0.0, 0.6, 0.8, 0.0];
    input.plane_normal = [1.0, 0.0, 0.0, 0.0];
    input.desired_speed = 1.0;
    state.update(&settings(), &input);
    let expected = 10.0 + (f32::from_bits(0xc11c_cccd) * f32::from_bits(0x3c88_8889)) * 0.6;
    assert!((state.speed - expected).abs() < 0.00001);
    assert!(state.speed < 10.0 - 2.0 * f32::from_bits(0x3c88_8889));
}

#[test]
fn vector_delta_limits_xyz_and_preserves_scaled_w() {
    let mut state = State::default();
    let mut input = input();
    input.forward[3] = 2.0;
    state.update(&settings(), &input);
    assert!((state.speed - 0.2).abs() < 0.000001);
    assert!((state.velocity[3] - 0.4).abs() < 0.000001);
    assert_eq!(
        remove_positive([2.0, 0.0, 0.0, 7.0], [2.0, 0.0, 0.0, 3.0]),
        [-2.0, 0.0, 0.0, 1.0]
    );
}
