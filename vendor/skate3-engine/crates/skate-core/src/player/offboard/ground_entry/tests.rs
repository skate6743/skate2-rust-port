use super::*;
fn input() -> Input {
    Input {
        animation_frame: IDENTITY,
        processed_velocity_608: [1.0, 2.0, 3.0, 0.0],
        processed_flags_2484: 0,
        processed_flags_2476: 0,
        requested_angle_2936: 0.0,
        requested_duration_2896: 1.0,
        previous_state_2504: 0,
        previous_frame_up_208: [0.0, 1.0, 0.0, 0.0],
        body_position_15872: [0.0, 1.0, 0.0, 0.0],
    }
}
#[test]
fn ordinary_entry_uses_actual_animation_frame_and_rejects_vertical_velocity() {
    let mut state = State::default();
    let mut i = input();
    i.animation_frame[3] = [7.0, 8.0, 9.0, 0.0];
    let placement = state.enter(&i);
    assert_eq!(placement.frame, i.animation_frame);
    assert_eq!(placement.planar_velocity, [1.0, 0.0, 3.0, 0.0]);
    assert_eq!(placement.body_position, i.body_position_15872);
    assert!(state.flags_144_to_150[5]);
    assert!(!state.flags_144_to_150[6]);
}
#[test]
fn entry_resets_previous_flags_and_records_only_matching_previous_state() {
    let mut state = State::default();
    let mut i = input();
    i.previous_state_2504 = 502;
    state.enter(&i);
    assert!(state.flags_144_to_150[4]);
    i.previous_state_2504 = 501;
    i.previous_frame_up_208[1] = 0.7;
    state.enter(&i);
    assert!(state.flags_144_to_150[1]);
    assert!(!state.flags_144_to_150[4]);
    i.previous_frame_up_208[1] = f32::from_bits(0x3f35_c28f);
    state.enter(&i);
    assert!(!state.flags_144_to_150[1]);
}
#[test]
fn directed_entry_aligns_frame_to_velocity_and_preserves_translation() {
    let mut state = State::default();
    let mut i = input();
    i.processed_flags_2484 = 0x4000;
    i.processed_velocity_608 = [3.0, 2.0, 0.0, 0.0];
    i.animation_frame[3] = [7.0, 8.0, 9.0, 0.0];
    let p = state.enter(&i);
    assert!(state.flags_144_to_150[6]);
    assert!((p.frame[2][0] - 1.0).abs() < 1e-6);
    assert_eq!(p.frame[3], i.animation_frame[3]);
}
#[test]
fn low_speed_uses_animation_forward_and_mirroring_reverses_requested_turn() {
    let mut state = State::default();
    let mut i = input();
    i.processed_flags_2484 = 0x4000;
    i.processed_velocity_608 = [0.0; 4];
    i.requested_angle_2936 = 0.5;
    state.enter(&i);
    let normal = state.angular_velocity_176;
    i.processed_flags_2476 = 4;
    state.enter(&i);
    assert!((state.angular_velocity_176 + normal).abs() < 1e-5);
    assert!(normal > 0.0);
}
