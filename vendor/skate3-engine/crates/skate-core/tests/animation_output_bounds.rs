//! Independent buffer-validation checks; no generated-code fixtures.
use skate_core::animation::output::{PoseBufferError, Sqt, compose_hierarchy, sqt_to_local};

#[test]
fn invalid_buffers_fail_without_partial_publication() {
    let sentinel = [[f32::from_bits(0x3f123456); 4]; 4];
    let sqt = Sqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0; 4],
    };
    let mut destination = [sentinel; 4];
    assert_eq!(
        sqt_to_local(1, 0, &[sqt], &mut destination),
        Err(PoseBufferError::ShortInput)
    );
    assert_eq!(destination, [sentinel; 4]);
    assert_eq!(
        compose_hierarchy(4, &[-1, 0, 1, 9], -1, &[sentinel; 4], &mut destination),
        Err(PoseBufferError::InvalidParent { bone: 3, parent: 9 })
    );
    assert_eq!(destination, [sentinel; 4]);
}
