use super::{
    contact::{ContactInput, ContactState},
    math::{interpolate, inverse_affine},
    status::{LimbStatus, Mode},
    transforms::LimbFrames,
    two_bone::{self, AngleLimits, SolveResult},
};
use crate::physics::skeleton_animation_record::{IDENTITY, transform_point};

#[test]
fn inverse_preserves_points_for_scaled_and_sheared_authored_frames() {
    let frame = [
        [2.0, 0.0, 0.0, 0.0],
        [0.5, 3.0, 0.0, 0.0],
        [0.0, 0.25, 4.0, 0.0],
        [7.0, -2.0, 5.0, 0.0],
    ];
    let point = [0.3, -0.7, 1.2, 0.0];
    let recovered = transform_point(&inverse_affine(&frame), transform_point(&frame, point));
    for lane in 0..3 {
        assert!((recovered[lane] - point[lane]).abs() < 2e-6);
    }
}

#[test]
fn frame_interpolation_uses_rotation_and_translation_at_the_same_weight() {
    let mut target = IDENTITY;
    target[0] = [0.0, 1.0, 0.0, 0.0];
    target[1] = [-1.0, 0.0, 0.0, 0.0];
    target[3] = [8.0, 2.0, -4.0, 0.0];
    let (half, remaining) = interpolate(&IDENTITY, &target, 0.5);
    assert!((half[0][0] - core::f32::consts::FRAC_1_SQRT_2).abs() < 2e-6);
    assert!((half[0][1] - core::f32::consts::FRAC_1_SQRT_2).abs() < 2e-6);
    assert_eq!(&half[3][..3], &[4.0, 1.0, -2.0]);
    assert!((remaining - core::f32::consts::FRAC_PI_4).abs() < 2e-6);
    assert_eq!(interpolate(&IDENTITY, &target, 1.0), (target, 0.0));
}

#[test]
fn reachable_two_bone_target_keeps_both_segment_lengths() {
    let root = [0.0; 4];
    let middle = [0.3, 0.4, 0.0, 0.0];
    let end = [0.6, 0.0, 0.0, 0.0];
    let mut target = [0.7, 0.0, 0.0, 0.0];
    let mut solved = [0.0; 4];
    let result = two_bone::solve(
        root,
        middle,
        end,
        &mut solved,
        &mut target,
        AngleLimits {
            minimum_degrees: 5.0,
            maximum_degrees: 175.0,
        },
        true,
        2,
    );
    assert_eq!(result, SolveResult::Solved);
    assert_eq!(&target[..3], &[0.7, 0.0, 0.0]);
    let distance = |a: [f32; 4], b: [f32; 4]| {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    assert!((distance(root, solved) - 0.5).abs() < 2e-5, "{solved:?}");
    assert!((distance(solved, target) - 0.5).abs() < 2e-5, "{solved:?}");
}

#[test]
fn foot_queries_affect_the_following_update_and_loss_sets_support_history() {
    let mut contacts = ContactState::default();
    let mut statuses = [LimbStatus::default(); 4];
    statuses[0].mode = Mode::OnDeck;
    let mut frames = [LimbFrames::default(); 4];
    let input = |hit| ContactInput {
        flags_2468: 0x0200_0000,
        contact_bone: 10,
        foot_bones: [10, 20],
        toe_contact_normal: [0.0; 4],
        inverse_board: &IDENTITY,
        board: &IDENTITY,
        current_contacts: [hit, None],
    };
    contacts.update(&statuses, &mut frames, input(None));
    assert!(!contacts.support_failed_this_update);
    assert_eq!(contacts.feet[0].query_state, 2);
    contacts.update(&statuses, &mut frames, input(Some([0.0; 4])));
    assert!(contacts.support_failed_this_update);
    assert!(contacts.support_failed);
    assert_eq!(contacts.feet[0].query_state, 1);
    statuses[0].mode = Mode::Disabled;
    contacts.update(&statuses, &mut frames, input(None));
    assert!(!contacts.support_failed_this_update);
    assert!(contacts.support_failed);
}
