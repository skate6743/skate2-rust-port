use super::*;

fn constant<const N: usize>(value: f32) -> PointGraph<N> {
    PointGraph {
        x: std::array::from_fn(|i| i as f32),
        y: [value; N],
    }
}
fn settings() -> Settings {
    Settings {
        sprint_speed: constant(8.0),
        normal_speed: constant(4.0),
        sprint_blend: constant(1.0),
        sprint_time_cap: 2.0,
        slide_steering: constant(0.0),
    }
}
fn input() -> Input {
    Input {
        flags: 0,
        suppress_minimum: false,
        magnitude: 1.0,
        steering: 0.0,
        sprint_pressed: false,
        edge_active: false,
        ignore_obstacle: false,
        direction: [0.0, 0.0, 1.0, 0.0],
        edge_tangent: [0.0, 0.0, 1.0, 0.0],
        edge_point: [0.0; 4],
        right: [1.0, 0.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0, 0.0],
        forward: [0.0, 0.0, 1.0, 0.0],
        position: [0.0; 4],
        obstacle: false,
        obstacle_normal: [0.0, 0.0, -1.0, 0.0],
        sliding: false,
        slide_velocity: [0.0; 4],
    }
}
#[test]
fn sprint_grace_survives_release_but_edge_disables_blend() {
    let mut state = State::default();
    let mut input = input();
    input.sprint_pressed = true;
    state.update(&settings(), &input);
    assert_eq!(state.speed, 8.0);
    input.sprint_pressed = false;
    state.update(&settings(), &input);
    assert_eq!(state.speed, 8.0);
    assert!(state.sprint_grace > 0.0);
    input.edge_active = true;
    state.update(&settings(), &input);
    assert_eq!(state.speed, 4.0);
    assert!(state.sprint_time > 0.0);
}
#[test]
fn centered_obstacle_stops_speed_and_has_priority_over_slide() {
    let mut state = State::default();
    let mut input = input();
    input.obstacle = true;
    input.sliding = true;
    input.slide_velocity = [0.0, 0.0, 3.0, 0.0];
    state.update(&settings(), &input);
    assert!(state.obstacle_centered);
    assert_eq!(state.speed, 0.0);
    input.ignore_obstacle = true;
    state.update(&settings(), &input);
    assert!(!state.obstacle_centered);
    assert!((state.speed - 7.0).abs() < 0.00001);
}
#[test]
fn steering_merge_replaces_opposition_and_preserves_stronger_same_sign() {
    let mut state = State::default();
    state.steering = 0.8;
    state.merge_steering(0.2);
    assert_eq!(state.steering, 0.8);
    state.merge_steering(-0.1);
    assert_eq!(state.steering, -0.1);
    state.merge_steering(-0.6);
    assert_eq!(state.steering, -0.6);
}
#[test]
fn edge_tangent_flips_before_target_and_flag_resets_next_tick() {
    let mut state = State::default();
    let mut input = input();
    input.edge_active = true;
    input.edge_tangent = [0.0, 0.0, -1.0, 0.0];
    input.edge_point = [0.2, 0.0, 1.0, 0.0];
    state.update(&settings(), &input);
    assert_eq!(state.edge_target, [0.2, 0.0, 1.5, 0.0]);
    assert!(state.edge_aligned);
    assert!(state.steering > 0.0);
    input.edge_active = false;
    state.update(&settings(), &input);
    assert!(!state.edge_aligned);
    assert_eq!(state.edge_target, [0.2, 0.0, 1.5, 0.0]);
}
#[test]
fn angle_degeneracy_and_half_turn_boundary_are_retained() {
    assert_eq!(
        signed_angle([0.0; 4], [1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]),
        0.0
    );
    assert_eq!(
        normalize_or([0.0; 4], [0.0, 0.0, 1.0, 0.0]),
        [0.0, 0.0, 1.0, 0.0]
    );
    let pi = f32::from_bits(0x4049_0fdb);
    assert!((wrap_angle(pi) - pi).abs() < 0.000001);
    assert!(wrap_angle(pi + 0.1) < 0.0);
}

#[test]
fn edge_target_preserves_native_fourth_lane() {
    let mut state = State::default();
    let mut input = input();
    input.edge_active = true;
    input.edge_tangent[3] = 2.0;
    input.edge_point[3] = 3.0;
    state.update(&settings(), &input);
    assert_eq!(state.edge_target[3], 4.0);
    assert_eq!(horizontal([1.0, 4.0, 0.0, 9.0]), [1.0, 0.0, 0.0, 1.0]);
}
