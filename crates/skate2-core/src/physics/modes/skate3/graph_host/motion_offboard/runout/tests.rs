use super::*;
use skate3_core::animation::playback_parameters::SettableAttributes;
fn observation() -> Observation {
    Observation {
        offboard_flag_331: true,
        offboard_velocity_128: [3.0, 0.0, 4.0, 91.0],
        reckoning_velocity_16: [0.0, 0.0, 2.0, 0.0],
        reckoning_up_96: [0.0, 1.0, 0.0, 0.0],
        skeleton_vector_0: [0.0, 0.0, 1.0, 0.0],
        animation_mirrored: false,
    }
}
#[test]
fn offboard_selection_and_mirror_are_sampled_at_begin() {
    let mut state = State::default();
    state.begin(Some(observation()));
    assert!((state.speed_12.unwrap() - 5.0).abs() < 0.0001);
    let angle = state.angle_degrees_8;
    assert!(angle > 36.0 && angle < 38.0);
    let mut mirrored = observation();
    mirrored.animation_mirrored = true;
    state.begin(Some(mirrored));
    assert!((state.angle_degrees_8 + angle).abs() < 0.0001);
    mirrored.offboard_flag_331 = false;
    state.begin(Some(mirrored));
    assert!((state.speed_12.unwrap() - 2.0).abs() < 0.0001);
    assert!(state.angle_degrees_8.abs() < 0.01);
}
#[test]
fn update_reuses_capture_and_preserves_queue_order_and_flags() {
    let mut state = State::default();
    state.begin(Some(observation()));
    let captured = state;
    let mut sink = SettableAttributes::default();
    state.update(Some(&mut sink)).unwrap();
    state.begin(None);
    state.update(Some(&mut sink)).unwrap();
    assert_eq!(state, captured);
    let entries = sink.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, encode(b"BipedStartAngle"));
    assert_eq!(entries[1].name, encode(b"BipedSpeed"));
    assert_eq!(entries[0].value, captured.angle_degrees_8);
    assert_eq!(entries[1].value, captured.speed_12.unwrap());
    assert!(
        entries
            .iter()
            .all(|entry| !entry.normalized && entry.sequence_id == -1)
    );
}
#[test]
fn absent_begin_does_not_fabricate_speed_and_stationary_begin_is_valid() {
    let mut state = State::default();
    let mut sink = SettableAttributes::default();
    assert!(state.update(Some(&mut sink)).is_err());
    assert!(sink.entries().is_empty());
    state.update(None::<&mut SettableAttributes>).unwrap();
    let mut input = observation();
    input.offboard_velocity_128 = [0.0; 4];
    state.begin(Some(input));
    assert_eq!(state.angle_degrees_8, 0.0);
    assert_eq!(state.speed_12, Some(0.0));
    state.update(Some(&mut sink)).unwrap();
}
