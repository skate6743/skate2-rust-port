//! Actual teleport skeleton stages82BE3508 and82BDF7C0. The player owns the
//! intervening ResetSystems and ProcessData calls; this is the same skeleton.
use super::{CollisionInput, SkeletonInputRuntime, SkeletonOwners};
use skate3_core::{
    animation::{foot_ik::state::FootIkState, output::NativeMatrix},
    physics::{
        board::BodyId,
        board_runtime::BoardRuntime,
        rigid_body::RetailSimulationStep,
        skeleton_animation_record::{AnimationPartTransform as Transform, IDENTITY},
        skeleton_body::{SkeletonCollisionFeedback, SkeletonCollisionMode},
        skeleton_output::wobble::Wobble,
    },
    player::input_phase::ProcessedPhysicsInput,
};

impl SkeletonInputRuntime {
    ///82BE3508, after board reset and before player ResetSystems82DB92D0.
    ///Does not replace owners with startup instances or reset unrelated rates.
    pub fn reset_for_teleport(
        &mut self,
        owners: &mut SkeletonOwners<'_>,
        mode: &mut SkeletonCollisionMode,
        feedback: &mut SkeletonCollisionFeedback,
        wobble: &mut Wobble,
        ground_elapsed_16505: &mut bool,
    ) {
        owners.animated.motion.reset_board_orientation_history();
        self.reenable_requested = true;
        self.offboard_reset_pose_pending = false;
        self.invalid_target_reset = false;
        *ground_elapsed_16505 = true;
        mode.reset_body_state(feedback);
        self.force_mode = 0;
        self.head_tracking_history = [[0.0; 4]; 8];
        self.head_tracking_active = false;
        owners.ik.state = FootIkState::default();
        //Native reset preserves the previously selected cached curves.
        wobble.time = 0.0;
        wobble.amplitude = 0.0;
        wobble.direction = 1.0;
        wobble.active = false;
        wobble.landing = false;
        owners.animated.board_frames.animation_target = IDENTITY;
        owners.animated.board_offset = Default::default();
        owners.animated.board_frames.lift_height = 0.0;
    }

    ///82DB8998 brackets82BDF7C0 with Skeleton16508=true/false after the real
    ///ProcessData call. Reckoning816 is its freshly reset original frame.
    pub fn update_teleport(
        &mut self,
        board: &BoardRuntime,
        reckoning_frame_816: &Transform,
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
    ) -> Result<Transform, String> {
        self.teleporting = true;
        let part = board.part_transforms()[BodyId::Deck.index()];
        let mut deck = [[0.0; 4]; 4];
        for axis in 0..3 {
            deck[axis][..3].copy_from_slice(&part.basis.columns[axis]);
        }
        deck[3] = [
            part.translation.x,
            part.translation.y,
            part.translation.z,
            0.0,
        ];
        let skeleton = &mut owners.animated;
        skeleton
            .roots
            .update_teleport(deck, &skeleton.animation_board, reckoning_frame_816);
        let frame = skeleton.board_frames.prepare_teleport(
            &skeleton.roots,
            &self.drive_frames[0],
            deck,
            &mut p.flags_2468,
        );
        let result = self.general_update(p, owners, globals, collision, simulation);
        //Keep the actual lifecycle flag coherent even if host data validation fails.
        self.teleporting = false;
        result?;
        owners.animated.finish_ground();
        owners.animation_input.fields.flags2468 = p.flags_2468;
        Ok(frame)
    }
}
