//! Original TU3 animated-board and COM-controlled air frame calculations.
//! 82BDDA10, 82BDE600 and 82BDE7B8. Persistent frames remain in their owners.
use super::{
    skeleton_animation_record::{AnimationPartTransform as Transform, IDENTITY, compose_affine},
    skeleton_board_frames::SkeletonBoardFrames,
    skeleton_root::{SkeletonRootFrames, inverse_rigid, orthonormalize},
};
use crate::trigonometry;

/// 82B985E8 publishes the one-frame request and retained frame count;
/// 82BDE920 loads that count with lwz before unsigned-to-floating conversion.
#[derive(Clone, Copy, Debug)]
pub struct AirDismountRevert {
    pub requested: bool,
    pub frames: u32,
    pub goofy: bool,
}

/// 82BDDA5C..DBBC, after the common UpdateRootTransforms82BE0318.
/// The skate root uses the unblended animation target in this path.
pub fn prepare_animated(
    roots: &SkeletonRootFrames,
    board: &mut SkeletonBoardFrames,
    mapped_board: &Transform,
    flags: &mut u32,
) -> Transform {
    let local = compose_affine(&roots.animation_to_board, mapped_board);
    let mut placement = IDENTITY;
    placement[3] = roots.predicted_board_position;
    let target = compose_affine(&placement, &local);
    *flags |= 1 << 19;
    board.animation_target = target;
    board.skate_root = target;
    board.update_com_lift(&roots.animation_to_world, board.centre_of_mass, 0.0);
    target
}

/// 82BDE7B8: preserve prior heading on entry, apply an authored dismount
/// revert, then place the current animation COM at the integrated air target.
/// It does not update animation_to_board/inverse_board or consume prediction.
pub fn update_known_air_roots(
    roots: &mut SkeletonRootFrames,
    reckoning: &Transform,
    target_com: [f32; 4],
    animation_com: [f32; 4],
    revert: AirDismountRevert,
) {
    if roots.initialize_heading {
        roots.heading_alignment =
            compose_affine(&inverse_rigid(reckoning), &roots.animation_to_world);
        //82BDE8EC overwrites the entire affine translation with zero.
        roots.heading_alignment[3] = [0.0; 4];
        roots.initialize_heading = false;
    }
    if revert.requested {
        let mut angle = f32::from_bits(0x4049_0fdb) / revert.frames as f32;
        if !revert.goofy {
            angle = -angle;
        }
        let (sin, cos) = trigonometry::sin_cos(angle);
        let rotation = [
            [cos, 0.0, -sin, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [sin, 0.0, cos, 0.0],
            [0.0; 4],
        ];
        roots.heading_alignment =
            orthonormalize(compose_affine(&roots.heading_alignment, &rotation));
    }
    let mut world = compose_affine(reckoning, &roots.heading_alignment);
    //82BDEB98..BB8: rotate COM without adding the prior translation.
    world[3] = std::array::from_fn(|lane| {
        let x = world[0][lane] * animation_com[0];
        let y = world[1][lane].mul_add(animation_com[1], x);
        target_com[lane] - world[2][lane].mul_add(animation_com[2], y)
    });
    roots.animation_to_world = orthonormalize(world);
    roots.world_to_animation = inverse_rigid(&roots.animation_to_world);
}

///82BDE634..6DC. ApplyBoardAnimation follows before the completion below.
pub fn prepare_known_air(
    roots: &SkeletonRootFrames,
    board: &mut SkeletonBoardFrames,
    mapped_board: &Transform,
    flags: &mut u32,
) -> Transform {
    let target = compose_affine(&roots.animation_to_world, mapped_board);
    *flags |= 1 << 19;
    board.animation_target = target;
    target
}

///82BDE748..7A4, after applying the target and the board velocity update.
pub fn finish_known_air(
    roots: &mut SkeletonRootFrames,
    board: &mut SkeletonBoardFrames,
    effective_board: Transform,
) {
    board.physical_board = effective_board;
    board.skate_root = effective_board;
    board.update_com_lift(&roots.animation_to_world, board.centre_of_mass, 0.0);
    roots.predicted_board_position = effective_board[3];
    roots.supplied_prediction = Some(effective_board[3]);
}

///82BDEEB8/82BE0E80: anchor an animation bone or COM in world space.
///Plants retain heading_alignment and do not consume board prediction.
pub fn update_plant_roots(
    roots: &mut SkeletonRootFrames,
    reckoning: &Transform,
    world_anchor: [f32; 4],
    animation_anchor: [f32; 4],
) {
    let mut world = compose_affine(reckoning, &roots.heading_alignment);
    world[3] = std::array::from_fn(|i| {
        let x = world[0][i] * animation_anchor[0];
        let y = world[1][i].mul_add(animation_anchor[1], x);
        world_anchor[i] - world[2][i].mul_add(animation_anchor[2], y)
    });
    roots.animation_to_world = orthonormalize(world);
    roots.world_to_animation = inverse_rigid(&roots.animation_to_world);
}
