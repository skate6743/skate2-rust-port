use super::*;

#[test]
fn com_uses_original_bone_volume_excludes_board_and_preserves_reset_order() {
    let mut sizes = [Vector3::new(0.0, 0.0, 0.0); 24];
    sizes[0] = Vector3::new(100.0, 100.0, 100.0);
    sizes[1] = Vector3::new(1.0, 1.0, 1.0);
    sizes[2] = Vector3::new(1.0, 1.0, 3.0);
    let masses = SkeletonAnimationMasses::from_bone_data(sizes, [1; 24], false);
    assert_eq!(masses.total, 4.0);
    assert_eq!(masses.fractional[0..3], [0.0, 0.25, 0.75]);
    let mut pose = [IDENTITY; 24];
    pose[0][3] = [1.0, 0.0, 0.0, 0.0];
    pose[1][3] = [1.0, 2.0, 0.0, 0.0];
    pose[2][3] = [1.0, 6.0, 0.0, 0.0];
    let mut to_board = IDENTITY;
    to_board[3] = [100.0, 20.0, -10.0, 0.0];
    let mut record = SkeletonAnimationRecord::default();
    record.update(&pose, &to_board, &masses);
    assert_eq!(record.centre_of_mass, [1.0, 5.0, 0.0, 0.0]);
    assert_eq!(record.com_to_deck_world, [0.0, 5.0, 0.0, 0.0]);
    assert_eq!(record.com_to_deck_world_delta, [0.0; 4]);
    pose[1][3][1] += 4.0;
    record.update(&pose, &to_board, &masses);
    assert_eq!(record.com_to_deck_world_delta, [0.0, 1.0, 0.0, 0.0]);
    record.reset_history();
    assert_eq!(record.reset_scalar, 1.0);
    assert_eq!(record.pose, pose);
}
#[test]
fn global_bone_then_physics_frame_preserves_native_mapping_and_lanes() {
    let local = physics_bone_frame([0.0, 0.0, 0.0, 1.0], [1.0, 2.0, 3.0, 7.0]);
    assert_eq!(local[0], [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(local[1], [0.0, 1.0, 0.0, 0.0]);
    assert_eq!(local[2], [0.0, 0.0, 1.0, 0.0]);
    // A global quarter turn around Z makes multiplication order observable.
    let global = [
        [0.0, 1.0, 0.0, 11.0],
        [-1.0, 0.0, 0.0, 13.0],
        [0.0, 0.0, 1.0, 17.0],
        [10.0, 20.0, 30.0, 19.0],
    ];
    let mut indices = [0; 24];
    indices[1] = 1;
    let mut frames = [IDENTITY; 24];
    frames[1] = local;
    let mapped = map_animation_parts(&[IDENTITY, global], &indices, &frames).unwrap();
    assert_eq!(mapped[0], IDENTITY);
    assert_eq!(mapped[1][0], global[0]);
    assert_eq!(mapped[1][1], global[1]);
    assert_eq!(mapped[1][2], global[2]);
    assert_eq!(mapped[1][3], [8.0, 21.0, 33.0, 107.0]);
    indices[23] = 2;
    assert!(map_animation_parts(&[IDENTITY, global], &indices, &frames).is_err());
}
