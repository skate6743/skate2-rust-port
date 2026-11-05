use super::*;

#[test]
fn boundary_uses_second_key_but_retains_two_frame_normalization() {
    let first = Sqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 2.0, 0.0],
        translation: [7.0, 0.0, 0.0, 1.0],
    };
    let second = Sqt {
        rotation: [0.0, 0.0, 0.0, 2.0],
        translation: [8.0, 0.0, 0.0, 1.0],
        ..first
    };
    let at_boundary = FrameSelection {
        first: 7,
        second: 8,
        coefficient: 0.25,
    };
    let sampled = sample_key_vbr(first, second, at_boundary);
    assert_eq!(sampled.translation, second.translation);
    assert_eq!(sampled.rotation, [0.0, 0.0, 0.0, 1.0]);
    let exact = FrameSelection {
        first: 8,
        second: 8,
        coefficient: 0.0,
    };
    assert_eq!(
        sample_key_vbr(second, second, exact).rotation,
        second.rotation
    );
    let within = FrameSelection {
        first: 6,
        second: 7,
        coefficient: 0.25,
    };
    assert_eq!(
        sample_key_vbr(first, second, within),
        sample_key(first, second, within)
    );
}
