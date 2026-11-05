use skate3_core::physics::{
    skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine},
    skeleton_board_frames::SkeletonBoardFrames,
    skeleton_root::SkeletonRootFrames,
};
pub(super) fn prepare_pose(
    roots: &mut SkeletonRootFrames,
    board_frames: &mut SkeletonBoardFrames,
    animation_board: &Transform,
    mapped: &Transform,
    retained: &mut Transform,
    input: super::Input<'_>,
    flags_2476: u32,
    flags_2484: u32,
    flags_2468: &mut u32,
) -> Transform {
    //82BDEE08 consumes animation record6464+16 using the OLD world root.
    board_frames.skate_root = compose_affine(&roots.animation_to_world, animation_board);
    //82BDE310 also uses the old basis, with world-space state1056 COM.
    board_frames.update_com_lift(
        &roots.animation_to_world,
        input.centre_of_mass_1056,
        f32::from_bits(0x3e75_c28f),
    );
    let target = prepare(
        roots,
        input.world_frame,
        mapped,
        retained,
        flags_2476,
        flags_2484,
        flags_2468,
    );
    board_frames.animation_target = target;
    target
}
pub(super) fn prepare(
    roots: &mut SkeletonRootFrames,
    world: &Transform,
    mapped: &Transform,
    retained: &mut Transform,
    flags_2476: u32,
    flags_2484: u32,
    flags_2468: &mut u32,
) -> Transform {
    let mut world = *world;
    if flags_2476 & 4 != 0 {
        //830BD4A0 initializer82F825F0 splats8216DEE0=-1 across all lanes.
        for axis in [0, 2] {
            for value in &mut world[axis] {
                *value *= -1.0;
            }
        }
    }
    roots.initialize_heading = true;
    roots.reset_initial_alignment(world);
    if flags_2484 & 1 == 0 {
        *retained = *mapped;
    }
    *flags_2468 |= 0x80000;
    compose_affine(&world, retained)
}
#[cfg(any())]
#[path = "frames_tests.rs"]
mod tests;
