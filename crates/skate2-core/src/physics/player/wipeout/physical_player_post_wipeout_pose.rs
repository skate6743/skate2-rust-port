//! Actual UpdatePoseBuffer82b4d970..de1c global/local publication.
//! Child of `SkeletonNormalAssembly`. Mandatory board visual tail follows.
use super::SkeletonNormalAssembly;
#[path = "physical_player_post_wipeout_pose_board.rs"]
pub(crate) mod board_visuals;
#[cfg(feature = "skate3-runtime")]
#[path = "../output/physical_player_movement_pose.rs"]
mod movement_pose;
use crate::{
    actor_construction::anim_out_phys_in::{
        AnimOutPhysInOwner,
        pose_lanes::{PoseAccessError, PoseBuffer},
    },
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    ragdoll::SkeletonPartIndex,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skeleton_root_transforms::SkeletonRootMatrix as Matrix,
};

#[derive(Debug)]
pub(crate) enum PosePublicationError {
    MissingBoneParameters(usize),
    Pose(PoseAccessError),
    Part(DynamicAssemblyPartTransformError),
}
impl std::fmt::Display for PosePublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBoneParameters(part) => write!(
                f,
                "UpdatePoseBuffer Part {part} authored parameters missing"
            ),
            Self::Pose(error) => error.fmt(f),
            Self::Part(error) => write!(f, "UpdatePoseBuffer deck: {error:?}"),
        }
    }
}

/// Native deck local captured dbd8, before reverse local publication. This
/// value authorizes only the mandatory de30..ee98 continuation, not success
/// of `UpdatePoseBuffer` or the enclosing `PostWipeoutCheck`.
pub(crate) struct PoseBeforeBoardVisuals {
    pub(crate) deck: Matrix,
}

impl SkeletonNormalAssembly {
    /// `pose` must be the SAME `constructor.bindings.animation_input` locked by
    /// the retained world-pair caller. No secondary matrices are constructed.
    pub(crate) fn try_post_wipeout_pose_records(
        &self,
        pose: &mut AnimOutPhysInOwner,
        board: &SkateboardAssemblySession,
        backend: &SkateboardSimulationBackend,
    ) -> Result<PoseBeforeBoardVisuals, PosePublicationError> {
        use PosePublicationError as E;
        let data = &self.state.binding.body().data;
        for part in SkeletonPartIndex::ALL {
            let index = part.index();
            // PDB Skeleton+40 SkeletonData+8 mBoneHandles is an array of
            // PhysicsParamBoneData pointers, NOT attached rigid-body handles.
            data.parameters()
                .bones
                .get(index)
                .ok_or(E::MissingBoneParameters(index))?;
            let record = self.physics_history.post_wipeout_record_transform(index);
            let world_to_animation = self.root_transforms.world_to_animation();
            let volume_to_bone = Matrix::known(data.volume_to_bone()[index]);
            let global = world_to_animation.compose(&record.compose(&volume_to_bone));
            pose.set_pose_matrix_typed(PoseBuffer::Global, data.bone_ids()[index], &global)
                .map_err(E::Pose)?;
        }
        // Source GetSkateboard/GetBody/GetPosition calls separate the loops.
        // A late deck failure leaves all globals and every old local intact.
        let deck = board
            .try_live_part_transform_typed(backend, 6)
            .map_err(E::Part)?;
        self.try_publish_physical_local_pose(pose)?;
        Ok(PoseBeforeBoardVisuals { deck })
    }

    /// PC physics-mode publication into the existing Skate 2 animation buffers.
    /// Read the actual attached player Parts after their selected solver. Only
    /// the 24 physical bones are replaced; the character's other authored
    /// joints, face and hand animation remain owned by Skate 2.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_publish_mode_physics_pose(
        &self,
        pose: &mut AnimOutPhysInOwner,
        backend: &SkateboardSimulationBackend,
        world_to_animation: &Matrix,
    ) -> Result<(), PosePublicationError> {
        use PosePublicationError as E;
        let data = &self.state.binding.body().data;
        for part in SkeletonPartIndex::ALL {
            let index = part.index();
            let volume = self.session
                .try_live_part_transform_typed(backend, index)
                .map_err(E::Part)?;
            let bone = volume.compose(&Matrix::known(data.volume_to_bone()[index]));
            let global = world_to_animation.compose(&bone);
            pose.set_pose_matrix_typed(PoseBuffer::Global, data.bone_ids()[index], &global)
                .map_err(E::Pose)?;
        }
        self.try_publish_physical_local_pose(pose)
    }

    /// Shared native reverse-order global-to-local publication. The parent
    /// table belongs to Skate 2's physical rig, including its authored roots.
    fn try_publish_physical_local_pose(
        &self,
        pose: &mut AnimOutPhysInOwner,
    ) -> Result<(), PosePublicationError> {
        use PosePublicationError as E;
        let data = &self.state.binding.body().data;
        for part in SkeletonPartIndex::ALL.into_iter().rev() {
            let index = part.index();
            let bone = data.bone_ids()[index];
            let local = if let Some(parent) = data.parents()[index] {
                let parent = pose
                    .pose_matrix_typed(PoseBuffer::Global, data.bone_ids()[parent.index()])
                    .map_err(E::Pose)?;
                let inverse = parent.inverse();
                let child = pose
                    .pose_matrix_typed(PoseBuffer::Global, bone)
                    .map_err(E::Pose)?;
                inverse.compose(&child)
            } else {
                pose.pose_matrix_typed(PoseBuffer::Global, bone)
                    .map_err(E::Pose)?
            };
            pose.set_pose_matrix_typed(PoseBuffer::Local, bone, &local)
                .map_err(E::Pose)?;
        }
        Ok(())
    }
}
