use super::*;
use crate::physics::drive_frames::default_truck_transforms;

#[test]
fn each_target_changes_only_its_native_drive_and_keeps_anchor() {
    let base = default_truck_transforms();
    let rest = steering_drive_frames(base, [0.0; 2]);
    let first = steering_drive_frames(base, [0.4, 0.0]);
    assert_eq!(first[0], rest[0]);
    assert_ne!(first[1].body_b.orientation, rest[1].body_b.orientation);
    assert_eq!(first[1].body_b.translation, rest[1].body_b.translation);
    let second = steering_drive_frames(base, [0.0, 0.4]);
    assert_eq!(second[1], rest[1]);
    assert_ne!(second[0].body_b.orientation, rest[0].body_b.orientation);
    assert_eq!(second[0].body_b.translation, rest[0].body_b.translation);
}
