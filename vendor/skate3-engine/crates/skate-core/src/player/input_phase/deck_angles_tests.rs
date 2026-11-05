use super::SkeletonOutputFields;

#[test]
fn deck_angles_preserve_native_quadrants_stance_and_pitch_projection() {
    let mut output = SkeletonOutputFields::default();
    let pi = std::f32::consts::PI;
    for (x, z, expected) in [
        (1.0, 1.0, pi / 4.0),
        (-1.0, 1.0, -pi / 4.0),
        (1.0, -1.0, 3.0 * pi / 4.0),
        (-1.0, -1.0, -3.0 * pi / 4.0),
        (0.0, 0.0, pi / 2.0),
        (-0.0, -0.0, -pi / 2.0),
    ] {
        output.publish_deck_angles([x, 0.5, z, 0.0], false);
        assert!((output.deck_yaw_536 - expected).abs() < 0.00001);
        assert_eq!(output.deck_pitch_540, 0.5);
    }
    output.publish_deck_angles([1.0, 0.5, 1.0, 0.0], true);
    assert!((output.deck_yaw_536 + 3.0 * pi / 4.0).abs() < 0.00001);
    assert_eq!(output.deck_pitch_540, -0.5);
}
