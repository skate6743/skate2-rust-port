//! Original Skeleton::UpdateRootTransformsWipeout82BDFBE8.
use crate::physics::{
    skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine},
    skeleton_root::{SkeletonRootFrames, inverse_rigid},
};
const DT: f32 = f32::from_bits(0x3C88_8889);

///Reconstruct the root from the cached physical hips and animated hips,
///then blend its predicted position toward the actual weighted physical COM.
///This updates animation roots; it does not overwrite any solved body rates.
pub fn update_roots(
    roots: &mut SkeletonRootFrames,
    animation_hips: &Transform,
    physical_hips: &Transform,
    hips_velocity: [f32; 4],
    physical_com: [f32; 4],
    com_velocity: [f32; 4],
    state_time: f32,
) {
    let inverse_hips = inverse_rigid(animation_hips);
    let mut predicted_hips = *physical_hips;
    predicted_hips[3] = std::array::from_fn(|i| hips_velocity[i].mul_add(DT, physical_hips[3][i]));
    let mut root = compose_affine(&predicted_hips, &inverse_hips);
    let weight = state_time + DT;
    let weight = if -weight >= 0.0 { 0.0 } else { weight };
    let weight = if 1.0 - weight >= 0.0 { weight } else { 1.0 };
    let mut desired = physical_com;
    desired[1] += f32::from_bits(0xBF4C_CCCD);
    root[3] = std::array::from_fn(|i| {
        let predicted = com_velocity[i].mul_add(DT, root[3][i]);
        desired[i].mul_add(weight, predicted * (1.0 - weight))
    });
    roots.animation_to_world = root;
    roots.world_to_animation = inverse_rigid(&root);
}
