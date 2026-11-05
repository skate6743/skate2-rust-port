//!82BDCFB0 at the existing ProcessData boundary; never a physical teleport.
use super::animated_skeleton::AnimatedSkeleton;
use skate3_core::{
    animation::output::NativeMatrix,
    physics::{
        grind_air::{GrindAir, Input, PoseInput, Settings},
        skeleton_animation_record::{AnimationPartTransform, map_animation_parts},
    },
    player::input_phase::ProcessedPhysicsInput,
};
pub(super) fn update(
    state: &mut GrindAir,
    settings: &Settings,
    processed_deck: AnimationPartTransform,
    p: &ProcessedPhysicsInput,
    animated: &mut AnimatedSkeleton,
    globals: &[NativeMatrix],
) -> Result<bool, String> {
    let adjustment = state.update(
        Input {
            active: true,
            board: processed_deck,
            velocity: p.vectors_400_416[0].map(f32::from_bits),
            angular_velocity: p.vectors_720_784_800_816_832_864[0].map(f32::from_bits),
            up: p.vectors_544_560_592_608[0].map(f32::from_bits),
            timestep: p.timestep_2604,
            flags_2468: p.flags_2468,
            flags_2472: p.flags_2472,
            flags_2480: p.flags_2480,
            flags_2484: p.flags_2484,
        },
        settings,
    )?;
    let Some(adjustment) = adjustment else {
        return Ok(false);
    };
    //ProcessData maps this frame before the native producer; do not use last pose.
    let parts = map_animation_parts(globals, &animated.bone_indices, &animated.physics_frames)?;
    let transform = adjustment.local_transform(PoseInput {
        animation_to_world: animated.roots.animation_to_world,
        world_to_animation: animated.roots.world_to_animation,
        physical_forward: animated.board_frames.physical_board[2],
        animation_board_position: parts[0][3],
    });
    animated.board_offset.refresh_transform(transform);
    Ok(true)
}
