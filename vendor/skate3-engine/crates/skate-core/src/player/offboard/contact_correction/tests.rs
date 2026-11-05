use super::*;
fn state() -> ContactCorrection {
    //Explicit synthetic prior state; constructor/reset ownership is separate.
    ContactCorrection {
        active: true,
        direction: [0.0, 0.0, 1.0, 0.0],
        displacement: [1.0; 4],
    }
}
fn input(a: Vector, b: Vector) -> Input {
    Input {
        collision_displacements: [a, b],
        projection_axis_416: [0.0; 4],
        up_axis_16: [0.0, 1.0, 0.0, 0.0],
    }
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

#[test]
fn inactive_contact_clears_displacement_but_retains_previous_direction() {
    let mut state = state();
    state.update(input([0.0; 4], [0.001, 0.0, 0.0, 0.0]));
    assert!(!state.active);
    assert_eq!(state.direction, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(state.displacement, [0.0; 4]);
}

#[test]
fn equal_contacts_choose_second_and_apply_clearance_before_half_correction() {
    let mut state = state();
    state.update(input([0.15, 0.0, 0.0, 0.0], [0.0, 0.0, -0.15, 0.0]));
    assert!(state.active);
    close(state.direction[2], -1.0);
    close(state.displacement[2], -0.05);
    close(state.displacement[0], 0.0);
}

#[test]
fn large_correction_is_limited_then_projected_off_current_up_axis() {
    let mut state = state();
    state.update(input([3.0, 4.0, 0.0, 0.0], [0.0; 4]));
    close(state.direction[0], 0.6);
    close(state.direction[1], 0.8);
    close(state.displacement[0], 0.06);
    close(state.displacement[1], 0.0);
}

#[test]
fn projection_axis_removes_response_without_disabling_contact_observation() {
    let mut state = state();
    let mut i = input([0.0, 0.0, 0.2, 0.0], [0.0; 4]);
    i.projection_axis_416 = [0.0, 0.0, 1.0, 0.0];
    state.update(i);
    assert!(state.active);
    close(state.direction[2], 1.0);
    assert_eq!(state.displacement, [0.0; 4]);
}
