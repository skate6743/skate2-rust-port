use super::*;

fn graph(y: [f32; 8]) -> PointGraph<8> {
    PointGraph {
        x: [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0],
        y,
    }
}
fn settings() -> Settings {
    Settings {
        spin: graph([0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0]),
        height: graph([2.0; 8]),
    }
}

#[test]
fn first_update_uses_unscaled_height_seed_without_advancing_time() {
    let settings = settings();
    let mut state = State::new();
    state.begin(&settings);
    state.capture_animation_length(1.0);
    let p = Parameters {
        max_height: 0.3,
        spin_scale: 0.5,
    };
    let first = state.update(0.25, -0.8, p, &settings);
    assert_eq!(
        first,
        Output {
            balance: -0.3,
            spin: -0.0
        }
    );
    assert!(!state.needs_animation_length());
    //Absent/zero input keeps the original negative intent direction.
    assert_eq!(state.update(0.25, 0.0, p, &settings).spin, -0.125);
    assert_eq!(state.update(0.25, -0.0, p, &settings).spin, -0.25);
    state.begin(&settings);
    state.capture_animation_length(1.0);
    assert_eq!(state.update(0.25, 0.0, p, &settings).spin.to_bits(), 0);
}

#[test]
fn phase_clamp_preserves_zero_length_and_end_behavior() {
    let settings = settings();
    let p = Parameters {
        max_height: 0.3,
        spin_scale: 0.5,
    };
    let mut state = State::new();
    state.begin(&settings);
    state.capture_animation_length(0.0);
    //0/0 follows the original fsel path to phase1, rather than phase0.
    assert_eq!(state.update(0.25, 1.0, p, &settings).spin, 0.5);
    assert_eq!(state.update(10.0, -1.0, p, &settings).spin, -0.5);
}
