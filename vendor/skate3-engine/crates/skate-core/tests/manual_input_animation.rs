use skate_core::{
    animation::manual::{Settings, State},
    input::{controller::DerivedControllerInput, manual_intentions},
    point_graph::PointGraph,
};

fn intents(x: f32, y: f32, flags: u32) -> Vec<skate_core::input::riding_intentions::RidingIntent> {
    let mut words = [0; 26];
    words[9] = x.to_bits();
    words[10] = y.to_bits();
    manual_intentions::produce(&DerivedControllerInput::from_words(words), flags)
}

#[test]
fn manual_uses_radial_magnitude_y_sign_and_its_own_suppression_bit() {
    for sign in [-1.0, 1.0] {
        let result = intents(0.3, sign * 0.4, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Manual");
        assert!((result[0].value - sign * 0.5).abs() < 1e-6);
        assert!(intents(0.3, sign * 0.4, 1 << 9).is_empty());
        assert_eq!(intents(0.3, sign * 0.4, 1 << 8), result);
    }
    assert!(intents(0.0, -0.0, 0).is_empty());
    assert!(intents(0.5, 0.0, 0).is_empty());
}

#[test]
fn brake_has_an_independent_strict_threshold_and_retains_horizontal_sign_rule() {
    assert_eq!(intents(0.0, 0.9, 0).len(), 1);
    for (x, y, sign) in [(0.0, 1.0, 1.0), (0.0, -1.0, -1.0), (1.0, 0.0, -1.0)] {
        let result = intents(x, y, 0);
        let brake = result.iter().find(|v| v.name == "ManualBrake").unwrap();
        assert!((brake.value - sign).abs() < 1e-6);
        assert_eq!(result.iter().any(|v| v.name == "Manual"), y != 0.0);
    }
}

fn settings() -> Settings {
    // Independent fixture: a piecewise curve exercises both the dead band
    // and the signed response without requiring any private asset.
    Settings {
        balance: PointGraph {
            x: [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.8, 1.0],
            y: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        },
        velocity_limit: 0.04,
        acceleration_limit: 0.02,
    }
}

#[test]
fn manual_animation_limits_velocity_and_reversal_acceleration_per_update() {
    let settings = settings();
    let mut state = State::default();
    assert_eq!(state.update(Some(0.6), &settings), 0.0);
    assert_eq!(state.update(Some(1.0), &settings), 0.02);
    assert_eq!(state.update(Some(1.0), &settings), 0.06);
    assert_eq!(state.velocity, 0.04);
    // Reversal initially continues forward while velocity decelerates.
    assert_eq!(state.update(Some(-1.0), &settings), 0.08);
    assert_eq!(state.velocity, 0.02);
    assert_eq!(state.update(Some(-1.0), &settings), 0.08);
    assert_eq!(state.velocity, 0.0);
    assert_eq!(state.update(Some(-1.0), &settings), 0.06);
    state.begin();
    assert_eq!(state, State::default());
    assert_eq!(state.update(None, &settings), 0.0);
}
