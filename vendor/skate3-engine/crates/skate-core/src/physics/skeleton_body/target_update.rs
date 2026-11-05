//! Original Skeleton::UpdateHookPositions82BE1618 and extra followers82BE18B8.
//! Their target changes precede the returned teleport/large-displacement gate.
use super::{SkeletonBody, SkeletonTargets};
use crate::{
    math::Vector3,
    physics::{
        board_pose::orthonormalize_rotation,
        native_arithmetic::dot3,
        skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine},
    },
    riding::ground_orientation::clamp_length,
};

pub struct SkeletonTargetInput<'a> {
    ///Skeleton11792, mapped physical animation hips, not physical COM.
    pub animation_hips: &'a Transform,
    ///Skeleton11728, mapped physical animation board.
    pub animation_board: &'a Transform,
    ///Skeleton11920.
    pub animation_to_world: &'a Transform,
    ///Skeleton12368, inverse of the actual physical board.
    pub inverse_board: &'a Transform,
    ///Skeleton15760, published by the current movement branch.
    pub skate_root: &'a Transform,
    ///Skeleton15824 and15888 from the actual COM/lift producer.
    pub com_frame: &'a Transform,
    pub lifted_com_frame: &'a Transform,
    ///Skeleton16508.
    pub teleporting: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct ExtraTargetPositions {
    ///Skeleton11632,11648,11664 respectively.
    pub com: [f32; 4],
    pub lifted_com: [f32; 4],
    pub following_com: [f32; 4],
}
pub struct SkeletonTargetUpdate {
    ///Skeleton12240, inverse physical board * animated board in world space.
    pub animation_board_to_physics: Transform,
    pub positions: ExtraTargetPositions,
    ///False triggers GeneralUpdate's real teleport/reset behavior after all
    ///target and velocity writes, not a guard suppressing those writes.
    pub continuous: bool,
}

impl SkeletonTargets {
    ///82BE1618 updates only the two hook targets and returns the board-space
    /// transform consumed by the skeleton input owner.
    pub fn update_hook_positions(&mut self, input: &SkeletonTargetInput<'_>) -> Transform {
        let hips = compose_affine(input.animation_to_world, input.animation_hips);
        let board = compose_affine(input.animation_to_world, input.animation_board);
        let animation_board_to_physics = compose_affine(input.inverse_board, &board);
        self.set_transform(0, orthonormalize(hips));
        self.set_transform(1, orthonormalize(*input.skate_root));
        animation_board_to_physics
    }

    pub fn update_positions(
        &mut self,
        input: SkeletonTargetInput<'_>,
        skeleton: &mut SkeletonBody,
    ) -> SkeletonTargetUpdate {
        let previous = self.transform(0)[3];
        let animation_board_to_physics = self.update_hook_positions(&input);
        let hips = self.transform(0)[3];
        let change = std::array::from_fn(|i| previous[i] - hips[i]);
        let positions =
            self.update_extra_targets(skeleton, input.com_frame, input.lifted_com_frame);
        //82BE18A0 checks strictly greater; unordered distance alone does not
        //force this returnfalse. Preserve the independent teleport byte test.
        let continuous = !input.teleporting && !(dot3(change, change) > 1.0);
        SkeletonTargetUpdate {
            animation_board_to_physics,
            positions,
            continuous,
        }
    }

    ///Complete82BE18B8, also called by Reset82BD9990 after constructing its
    ///COM frames. Replaces only physical extra-body velocities, never forces.
    pub fn update_extra_targets(
        &mut self,
        skeleton: &mut SkeletonBody,
        com_frame: &Transform,
        lifted_com_frame: &Transform,
    ) -> ExtraTargetPositions {
        self.set_transform(2, *lifted_com_frame);
        self.set_transform(3, *com_frame);
        let positions = ExtraTargetPositions {
            com: com_frame[3],
            lifted_com: lifted_com_frame[3],
            following_com: com_frame[3],
        };
        //GetPartTransform includes the actual local mass frame. Keep that
        //source even though these centered extra capsules have identity frames.
        let parts = skeleton.part_transforms();
        for (part, target) in [(24, lifted_com_frame[3]), (25, com_frame[3])] {
            let current = parts[part][3];
            let velocity = Vector3::new(
                (target[0] - current[0]) * 60.0,
                (target[1] - current[1]) * 60.0,
                (target[2] - current[2]) * 60.0,
            );
            skeleton.bodies_mut()[part].rates.linear_velocity = clamp_length(velocity, 40.0);
        }
        positions
    }
}
fn orthonormalize(frame: Transform) -> Transform {
    let words = orthonormalize_rotation(std::array::from_fn(|i| frame[i / 4][i % 4].to_bits()));
    std::array::from_fn(|i| std::array::from_fn(|j| f32::from_bits(words[i * 4 + j])))
}
