use super::*;
fn settings() -> Settings {
    Settings {
        minimum_auto_angle: 5.,
        automatic_speed: 0.1,
        input_speed: 0.2,
        input_delta: 0.01,
        automatic_delta: 0.02,
        maximum_landing_speed: 10.,
    }
}
fn entry() -> Entry {
    Entry {
        previous_category: 100,
        hippy: true,
        strength: 0.5,
        board_position: [0.; 4],
        com_position: [0., 1., 0., 0.],
        com_velocity: [0.; 4],
        board_velocity: [2., 0., 0., 0.],
        up: UP,
        hips_up: [1., 0., 0., 0.],
        animation_right: [1., 0., 0., 0.],
        reversed: false,
    }
}
#[test]
fn hippy_initializes_the_shared_manager_and_follows_gravity() {
    let mut state = State::default();
    let mut manager = Manager::default();
    state.enter(&mut manager, entry());
    assert!(manager.trajectory_valid_164 && manager.force_257);
    assert_eq!(manager.trajectory_32.velocity[0], 2.);
    assert_eq!(manager.trajectory_32.acceleration, GRAVITY);
    let t = manager.trajectory_32;
    assert!(t.position_at(0.1)[1] > t.position[1]);
    assert!(t.velocity_at(1.)[1] < 0.);
    assert!(t.position_at(1.)[1] < t.position[1]);
    assert_eq!(state.takeoff_frames, 2);
}
#[test]
fn biped_entry_preserves_the_actual_manager() {
    let mut state = State::default();
    let mut manager = Manager::default();
    state.enter(&mut manager, entry());
    let mut i = entry();
    let velocity = manager.trajectory_32.velocity;
    i.previous_category = 500;
    i.hippy = false;
    state.enter(&mut manager, i);
    assert_eq!(manager.elapsed_160, STEP);
    assert_eq!(manager.trajectory_32.velocity, velocity);
}
#[test]
fn spin_input_is_not_applied_on_first_frame_or_when_descending() {
    let mut state = State::default();
    state.hippy = true;
    state.align(&settings(), [0., 0., 1., 0.], [0., 0., 1., 0.], 1., 2., 1.);
    assert_eq!(state.applied_spin, 0.);
    state.time_to_land = 1.;
    state.align(&settings(), [0., 0., 1., 0.], [0., 0., 1., 0.], 1., 2., 1.);
    assert_eq!(state.applied_spin, -0.01);
    state.spin_rate = 0.;
    state.align(&settings(), [0., 0., 1., 0.], [0., 0., 1., 0.], 1., -2., 1.);
    assert_eq!(state.applied_spin, 0.);
}
#[test]
fn accurate_landing_uses_physical_feet_and_relative_board_velocity() {
    let t = State::accurate_time(0., 0., -1., [0.3, 0.3], 4);
    assert!(t > 0. && t < 0.2);
    let airborne = State::accurate_time(0., 0., -1., [0.3, 0.3], 0);
    assert!(airborne > t);
    assert_eq!(State::accurate_time(0., 0., 0., [-2., -2.], 4), 0.);
}
