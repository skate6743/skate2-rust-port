//! SkeletonIK::BlendTransforms82BEEEB8.
use super::{
    math::{interpolate, interpolate_affine},
    status::{LimbStatus, Mode},
    transforms::{LimbBinding, LimbFrames},
};
use crate::physics::{
    skeleton_animation_record::{
        AnimationPartTransform as Transform, compose_affine, transform_point,
    },
    skeleton_root::orthonormalize,
};

pub fn update(
    statuses: &[LimbStatus; 4],
    frames: &mut [LimbFrames; 4],
    bindings: &[LimbBinding; 4],
    physical_board: &Transform,
    animation_to_world: &Transform,
    animated_board: &Transform,
) {
    // IK160 supplies only the physical board rotation. Translation comes from
    // the actual animation record's board transformed through IK32.
    let mut board = *physical_board;
    board[3] = transform_point(animation_to_world, animated_board[3]);
    for ((status, frame), binding) in statuses.iter().zip(frames).zip(bindings) {
        match status.mode {
            Mode::Disabled => continue,
            Mode::OnDeck | Mode::External | Mode::Local => {}
        }
        if status.board_blend > 0.0 {
            let target = compose_affine(&board, &frame.board);
            frame.world = board_blend(&frame.world, &target, status.board_blend);
            if binding.parent_part.is_some() {
                let target = compose_affine(&board, &frame.parent_board);
                frame.parent_world = board_blend(&frame.parent_world, &target, status.board_blend);
            }
        }
        if matches!(status.mode, Mode::External | Mode::Local) {
            if status.external_blend >= 1.0 {
                frame.world = frame.external_world;
                frame.parent_world = frame.external_parent_world;
            } else {
                // The native external stage also updates the retained parent
                // frame for a limb without a parent, including at zero weight.
                frame.world =
                    interpolate_affine(&frame.world, &frame.external_world, status.external_blend);
                frame.parent_world = interpolate_affine(
                    &frame.parent_world,
                    &frame.external_parent_world,
                    status.external_blend,
                );
            }
        }
    }
}

fn board_blend(original: &Transform, target: &Transform, weight: f32) -> Transform {
    let translation = core::array::from_fn(|lane| {
        target[3][lane].mul_add(weight, original[3][lane] * (1.0 - weight))
    });
    // Each source frame is orthonormalized independently in Z/Y/X order before
    // the matrix interpolation call; translation is replaced afterwards.
    let mut a = orthonormalize(*original);
    let mut b = orthonormalize(*target);
    a[3] = [0.0; 4];
    b[3] = [0.0; 4];
    let (mut output, _) = interpolate(&a, &b, weight);
    output[3] = translation;
    output
}
