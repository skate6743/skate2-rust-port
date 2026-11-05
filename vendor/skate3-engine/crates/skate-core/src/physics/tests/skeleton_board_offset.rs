use super::*;

#[test]
fn refresh_applies_without_decay_then_height_and_xz_decay_independently() {
    let mut offset = SkateboardOffset::default();
    let mut transform = IDENTITY;
    transform[3] = [15.0, 30.0, 45.0, 0.0];
    offset.refresh_transform(transform);
    let mut board = IDENTITY;
    let mut targets = [IDENTITY; 4];
    offset.update(&mut board, &mut targets);
    assert_eq!(board[3], transform[3]);
    assert_eq!(targets[3][3], transform[3]);
    assert_eq!(offset.orientation_frames, 15.0);
    offset.refresh_height(10.0, 7.0);
    board = IDENTITY;
    targets = [IDENTITY; 4];
    offset.update(&mut board, &mut targets);
    let ratio = 14.0f32 / 15.0;
    assert_eq!(offset.orientation_frames, 14.0);
    assert_eq!(offset.height_frames, 7.0);
    assert_eq!(board[3][0], 15.0 * (ratio * ratio));
    assert_eq!(board[3][1], 10.0);
    assert_eq!(board, targets[0]);
    offset.orientation_frames = 0.0;
    offset.height_frames = 1.0;
    board = IDENTITY;
    offset.update(&mut board, &mut targets);
    assert_eq!(offset.height_frames, 0.0);
    assert_eq!(offset.transform[3][1], 0.0);
    assert!(!offset.height_refreshed);
}
