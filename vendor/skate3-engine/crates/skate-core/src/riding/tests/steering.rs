use super::*;

fn settings() -> SteeringSettings {
    SteeringSettings {
        hard_turn_increase: 1.0,
        damping: 0.7,
        speed_graph_max_speed: 50.0,
        push_scalar_increment: 0.05,
        push_scalar_decrement: 0.015,
        push_scalar_min: 0.5,
        manual_scalar: 0.25,
        general_scalar: 0.6,
        tight_trucks_scalar: 0.7,
        tilt_blending: 0.2,
        speed_graph: PointGraph {
            x: [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 1.0],
            y: [1.0; 8],
        },
        input_graph: PointGraph {
            x: [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 1.0],
            y: [1.0; 8],
        },
    }
}

fn input() -> SteeringInput {
    SteeringInput {
        turn: 0.5,
        flipped_controls_scalar: 1.0,
        ..Default::default()
    }
}

#[test]
fn hard_turn_uses_turn_sign_including_zero_not_hard_turn_sign() {
    let s = settings();
    let mut i = input();
    i.hard_turn = -1.0;
    assert_eq!(calculate_tilt(&s, i, None, None), 1.2);
    i.turn = -0.5;
    assert_eq!(calculate_tilt(&s, i, None, None), -1.2);
    i.turn = -0.0;
    assert_eq!(calculate_tilt(&s, i, None, None), 1.2);
}

#[test]
fn optional_state_pointers_and_pushing_recovery_preserve_native_limits() {
    let s = settings();
    let mut scalar = 1.0;
    let mut turn = 0.0;
    let mut i = input();
    i.pushing = true;
    calculate_tilt(&s, i, Some(&mut scalar), Some(&mut turn));
    assert_eq!(turn, 0.35);
    assert_eq!(scalar, 0.985);
    for _ in 0..100 {
        calculate_tilt(&s, i, Some(&mut scalar), None);
    }
    assert_eq!(scalar, 0.5);
    i.pushing = false;
    for _ in 0..100 {
        calculate_tilt(&s, i, Some(&mut scalar), None);
    }
    assert_eq!(scalar, 1.0);
}

#[test]
fn manual_tightness_and_flipped_controls_multiply_the_tilt() {
    let s = settings();
    let base = calculate_tilt(&s, input(), None, None);
    let i = SteeringInput {
        balance: 1.0,
        truck_tightness: 1.0,
        flipped_controls_scalar: -1.0,
        ..input()
    };
    assert!((calculate_tilt(&s, i, None, None) - base * -0.25 * 0.7).abs() < 1e-7);
}

#[test]
fn truck_contact_flags_swap_with_stance_and_first_inactive_tick_holds_angle() {
    let mut regular = TruckSteeringState::default();
    regular.update(0.6, 0.2, 0, 1 << 27);
    assert!(regular.targets[0] > 0.0);
    assert_eq!(regular.targets[1], 0.0);
    let mut switch = TruckSteeringState::default();
    switch.update(0.6, 0.2, 1 << 20, 1 << 27);
    assert_eq!(regular.targets, [switch.targets[1], switch.targets[0]]);
    let previous = regular.targets[0];
    regular.update(0.6, 0.2, 0, 0);
    assert_eq!(regular.targets[0], previous);
    regular.update(0.6, 0.2, 0, 0);
    assert_eq!(regular.targets[0], previous * 0.983);
}

#[test]
fn truck_reactivation_completes_the_native_ramp() {
    let mut state = TruckSteeringState::default();
    for _ in 0..11 {
        state.update(0.6, 1.0, 0, 3 << 26);
    }
    assert_eq!(state.targets, [0.6; 2]);
    assert!(state.activation_time[0] > 0.165);
}
