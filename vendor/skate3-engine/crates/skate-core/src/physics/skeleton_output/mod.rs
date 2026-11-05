//! Original TU3 physical pose worker82DB95E8, submitted by82DB6698.
//! Updates the existing animation buffers after the physical solve.
pub mod board;
pub mod correction;
pub mod wobble;

use super::skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine};
use crate::animation::foot_ik::{drive::Geometry, transforms::inverse_rigid};

/// Stock SkeletonData ids, nearest physical ancestors, and volume inverses.
pub struct SkeletonOutput {
    pub bone_indices: [usize; 24],
    pub geometry: Geometry,
    pub board_bones: board::BoneIndices,
    pub board_settings: board::Settings,
}
pub struct Input<'a> {
    /// Skeleton8016, all24 solved physical volume transforms in world space.
    pub physical_parts: &'a [Transform; 24],
    /// Skeleton11984, captured by82DB66C0 before submitting this worker.
    pub world_to_animation: &'a Transform,
    pub board: board::Input<'a>,
}
impl SkeletonOutput {
    /// Buffers must contain the evaluated stock animation for this same frame.
    /// Unrepresented bones retain their animation. Board locals are modified
    /// after the mapped globals; the worker does not rebuild the full hierarchy.
    pub fn publish(
        &self,
        input: Input<'_>,
        globals: &mut [Transform],
        locals: &mut [Transform],
    ) -> Result<(), &'static str> {
        if globals.len() != locals.len()
            || self.bone_indices.iter().any(|&i| i >= globals.len())
            || self.board_bones.all().iter().any(|&i| i >= locals.len())
        {
            return Err("physical pose output does not match the stock animation hierarchy");
        }
        //82DB96B8..97E4: volume -> animation bone -> animation space.
        for part in 0..24 {
            let world = compose_affine(
                &input.physical_parts[part],
                &self.geometry.inverse_part_frames[part],
            );
            globals[self.bone_indices[part]] = compose_affine(input.world_to_animation, &world);
        }
        //82DB97F4..9938 descends through the physical mapping. This inverse
        //is a rigid transpose, distinct from the authored volume inverse.
        for part in (0..24).rev() {
            let bone = self.bone_indices[part];
            locals[bone] = if let Some(parent) = self.geometry.parents[part] {
                compose_affine(
                    &inverse_rigid(&globals[self.bone_indices[parent]]),
                    &globals[bone],
                )
            } else {
                globals[bone]
            };
        }
        board::publish(input.board, &self.board_settings, self.board_bones, locals);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
