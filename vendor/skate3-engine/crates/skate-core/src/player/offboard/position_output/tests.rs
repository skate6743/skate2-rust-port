use super::*;
const UP: Vector = [0.0, 1.0, 0.0, 0.0];
const FORWARD: Vector = [0.0, 0.0, 1.0, 0.0];
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 2e-6, "{a} != {b}");
}
fn frame() -> FrameOutput {
    FrameOutput {
        frame: [[1.0, 0.0, 0.0, 0.0], UP, FORWARD, [0.0; 4]],
        velocity: [0.0; 4],
    }
}
fn input() -> PositionInput {
    PositionInput {
        previous_origin_112: [0.0; 4],
        override_origin_592: [0.0; 4],
        use_override_711: false,
        projection_axis_416: UP,
        contact_flags_176: 0,
        contact_origin_0: [0.0; 4],
        contact_origin_96: [0.0; 4],
        frame_position_176: [0.0; 4],
        animation_position_272: [2.0, 0.0, 3.0, 0.0],
    }
}

#[test]
fn frame_tracks_horizontal_motion_but_limits_downward_velocity_change() {
    let mut state = frame();
    state.update(FORWARD, UP, UP, [1.0, -1.0, 2.0, 0.0]);
    close(state.velocity[0], 60.0);
    close(state.velocity[1], -1.0);
    close(state.velocity[2], 120.0);
    close(state.frame[3][0], 1.0);
    close(state.frame[3][1], -1.0 / 60.0);
    state.update(FORWARD, UP, UP, [1.0, -1.0, 2.0, 0.0]);
    close(state.velocity[1], -2.0);
    close(state.frame[3][1], -3.0 / 60.0);
}

#[test]
fn upward_frame_velocity_is_not_limited_and_basis_is_normalized() {
    let mut state = frame();
    state.update([0.0, 0.0, 3.0, 0.0], UP, UP, [0.0, 1.0, 0.0, 0.0]);
    close(state.velocity[1], 60.0);
    close(state.frame[3][1], 1.0);
    close(state.frame[0][0], 1.0);
    close(state.frame[2][2], 1.0);
}

#[test]
fn position_follows_planar_animation_and_limits_vertical_change_per_update() {
    let mut position = [0.0; 4];
    update_position(&mut position, input());
    assert_eq!(position[0], 2.0);
    assert_eq!(position[2], 3.0);
    close(position[1], 0.1);
    update_position(&mut position, input());
    close(position[1], 0.2);
}

#[test]
fn alternate_origin_precedes_first_contact_and_second_contact_is_capped() {
    let mut position = [0.0, 0.95, 0.0, 0.0];
    let mut i = input();
    i.contact_flags_176 = 3;
    i.contact_origin_0 = [0.0, 10.0, 0.0, 0.0];
    i.contact_origin_96 = [0.0, 20.0, 0.0, 0.0];
    i.use_override_711 = true;
    i.override_origin_592 = [0.0, -0.3, 0.0, 0.0];
    update_position(&mut position, i);
    close(position[1], 0.95); //override-.3 then second-contact+.3 then offset+.95.
}
