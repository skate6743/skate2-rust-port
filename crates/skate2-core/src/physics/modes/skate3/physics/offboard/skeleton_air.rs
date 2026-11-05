//! BipedAir Skeleton82BDDD70 with the existing skeleton, drives and solver.
//! The caller supplies the shared reckoning service; no second physics owner.
use super::skeleton_ground::ReckoningUpdate;
use crate::skate3_native::physics::{
    skeleton_air::SkeletonAir,
    skeleton_input_runtime::{CollisionInput, SkeletonInputRuntime, SkeletonOwners},
};
use skate3_core::{
    animation::output::NativeMatrix,
    physics::{
        board_runtime::BoardRuntime,
        rigid_body::RetailSimulationStep,
        skeleton_animation_record::{AnimationPartTransform as Frame, compose_affine},
    },
    player::input_phase::ProcessedPhysicsInput,
};

pub(crate) struct Input {
    pub frame_208: Frame,
    pub trajectory_position_272: [f32; 4],
    pub body_target_416: [f32; 4],
    pub lift_436: f32,
    ///Actual processed board-forward96, NOT effective animation-forward224.
    pub board_forward_96: [f32; 4],
}

impl SkeletonInputRuntime {
    ///Old-root skate/COM -> new animation root -> held-board target ->
    ///GeneralUpdate -> shared Reckoning -> clear next trajectory12176.
    ///Return the actual processed0..48 target for the parent's input owner.
    pub(crate) fn update_biped_air<F>(
        &mut self,
        air: &mut SkeletonAir,
        board: &mut BoardRuntime,
        input: Input,
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
        finish_reckoning: F,
    ) -> Result<Frame, String>
    where
        F: FnOnce(ReckoningUpdate, &ProcessedPhysicsInput, f32) -> Result<(), String>,
    {
        let s = &mut owners.animated;
        //82BDEE08 and82BDE310 use the OLD root, before82BDDDC8 changes it.
        s.board_frames.skate_root = compose_affine(&s.roots.animation_to_world, &s.record.pose[0]);
        s.board_frames.update_com_lift(
            &s.roots.animation_to_world,
            input.body_target_416,
            input.lift_436,
        );
        let target = skate3_core::physics::skeleton_biped_air::prepare(
            &mut s.roots,
            input.frame_208,
            input.trajectory_position_272,
            s.record.centre_of_mass,
            &self.drive_frames[0],
            p.flags_2476,
            &mut p.flags_2468,
        );
        s.board_frames.animation_target = target;
        if p.flags_2480 & 0x8000 != 0 {
            s.board_frames.physical_board = air.apply_board(board, &target, true);
        }
        self.general_update(p, owners, globals, collision, simulation)?;
        finish_reckoning(
            ReckoningUpdate {
                up: owners.animated.roots.animation_to_world[1],
                forward: input.board_forward_96,
                blend: 0.5,
            },
            p,
            owners.animation_input.extra.physical_body_spin,
        )?;
        owners.animated.finish_ground();
        owners.animation_input.fields.flags2468 = p.flags_2468;
        Ok(target)
    }
}
