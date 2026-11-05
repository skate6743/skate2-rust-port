use super::*;
use crate::physics::skeleton_animation_record::IDENTITY;
#[test]
fn mapped_hand_choice_and_relative_translation() {
    assert_eq!(selected_hand(4), 0);
    assert_eq!(selected_hand(0), 1);
    let mut actual = IDENTITY;
    actual[3] = [7., 8., 9., 0.];
    let mut reparented = IDENTITY;
    reparented[3] = [2., 3., 4., 0.];
    let result = adjustment(&actual, &reparented);
    assert_eq!(result[3], [5., 5., 5., 0.]);
    assert_eq!(result[..3], IDENTITY[..3]);
}
#[test]
fn relative_rotated_hand_maps_reparented_position_to_actual_position() {
    let mut target = IDENTITY;
    target[0] = [0., 1., 0., 0.];
    target[1] = [-1., 0., 0., 0.];
    target[3] = [2., 3., 0., 0.];
    let result = adjustment(&IDENTITY, &target);
    assert_eq!(result[0], [0., -1., 0., 0.]);
    assert_eq!(result[1], [1., 0., 0., 0.]);
    assert_eq!(result[3], [-3., 2., 0., 0.]);
}
