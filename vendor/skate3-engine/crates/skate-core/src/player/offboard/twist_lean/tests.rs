use super::*;
fn observation(right: f32, up: f32, mirrored: bool) -> Option<Observation> {
    Some(Observation {
        hips_right_angle: right,
        hips_up_angle: up,
        mirrored,
    })
}

#[test]
fn capture_preserves_twist_and_lean_after_physics_and_stance_change() {
    let mut state = State::default();
    state.begin(observation(0.25, 0.75, true));
    assert_eq!(
        state.update(false, observation(1.5, 2.0, false)),
        State {
            twist: -0.75,
            lean: 0.25
        }
    );
    state.begin(observation(1.5, 2.0, false));
    assert_eq!(
        state.update(false, None),
        State {
            twist: 2.0,
            lean: 1.5
        }
    );
}

#[test]
fn always_reads_live_angles_without_changing_capture() {
    let mut state = State::default();
    state.begin(observation(0.25, 0.75, false));
    assert_eq!(
        state.update(true, observation(1.5, 2.0, true)),
        State {
            twist: -2.0,
            lean: 1.5
        }
    );
    assert_eq!(
        state.update(false, None),
        State {
            twist: 0.75,
            lean: 0.25
        }
    );
}

#[test]
fn missing_original_components_preserve_capture_but_live_locals_start_zero() {
    let mut state = State::default();
    state.begin(observation(0.25, 0.75, false));
    state.begin(None);
    assert_eq!(
        state.update(false, None),
        State {
            twist: 0.75,
            lean: 0.25
        }
    );
    assert_eq!(state.update(true, None), State::default());
}
