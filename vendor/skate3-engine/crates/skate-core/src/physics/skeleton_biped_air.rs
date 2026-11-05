//! Biped Skeleton82BDDD70: COM-constrained animation root, not a body teleport.
//! Original S3 SHA431b8eba...0395a; ordinary PC arithmetic, not Xenon parity.
use super::{
    skeleton_animation_record::{AnimationPartTransform as Frame, compose_affine},
    skeleton_root::{SkeletonRootFrames, orthonormalize},
};

///82BDDDC8..DF54. The frame's translation is replaced by the trajectory COM
///minus the rotated LOCAL animation COM10960. Mapped board12624 is independent.
pub fn prepare(
    roots: &mut SkeletonRootFrames,
    mut frame: Frame,
    trajectory_position: [f32; 4],
    local_com: [f32; 4],
    mapped_board: &Frame,
    flags_2476: u32,
    flags_2468: &mut u32,
) -> Frame {
    roots.initialize_heading = true;
    if flags_2476 & 4 != 0 {
        for axis in [0, 2] {
            frame[axis] = frame[axis].map(|lane| -lane);
        }
    }
    frame[3] = core::array::from_fn(|lane| {
        let x = frame[0][lane] * local_com[0];
        let y = frame[1][lane].mul_add(local_com[1], x);
        trajectory_position[lane] - frame[2][lane].mul_add(local_com[2], y)
    });
    roots.reset_initial_alignment(orthonormalize(frame));
    *flags_2468 |= 0x80000;
    compose_affine(&roots.animation_to_world, mapped_board)
}
