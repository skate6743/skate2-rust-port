//! Movement-phase continuation of the original Skeleton input owner.
use super::{CollisionInput, SkeletonInputRuntime, SkeletonOwners};
use crate::skate3_native::physics::foot_ik::{DriveOutput, PhysicalInput};
use skate3_core::{
    animation::output::NativeMatrix,
    physics::{
        board_runtime::BoardRuntime,
        drive_parameters::{RetailDriveParams, RetailDriveType},
        rigid_body::RetailSimulationStep,
        skeleton_animation_record::AnimationPartTransform as Transform,
        skeleton_body::{SkeletonDrives, SkeletonTargetInput},
        skeleton_general,
    },
    player::input_phase::ProcessedPhysicsInput,
};

impl SkeletonInputRuntime {
    ///Ground82BDF530: update actual roots/board frames, GeneralUpdate, then
    ///reset next trajectory. Return the native Processed0..48 publication.
    pub fn update_ground(
        &mut self,
        board: &BoardRuntime,
        reckoning: &Transform,
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
    ) -> Result<Transform, String> {
        let frame =
            owners
                .animated
                .prepare_ground(board, reckoning, p.timestep_2604, &mut p.flags_2468);
        self.general_update(p, owners, globals, collision, simulation)?;
        owners.animated.finish_ground();
        owners.animation_input.fields.flags2468 = p.flags_2468;
        Ok(frame)
    }

    ///Source order: root derivative -> real target writes -> discontinuity
    ///reset -> IK -> bone drive update -> fixed-step gravity displacement.
    pub fn general_update(
        &mut self,
        p: &ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
    ) -> Result<DriveOutput, String> {
        self.root_velocity = skeleton_general::root_velocity(
            &mut self.previous_root_position,
            owners.animated.roots.animation_to_world[3],
            p.timestep_2604,
        );
        let s = &owners.animated;
        let target = owners.drives.targets.update_positions(
            SkeletonTargetInput {
                animation_hips: &s.animation_hips,
                animation_board: &s.animation_board,
                animation_to_world: &s.roots.animation_to_world,
                inverse_board: &s.roots.inverse_board,
                skate_root: &s.board_frames.skate_root,
                com_frame: &s.board_frames.com_frame,
                lifted_com_frame: &s.board_frames.lifted_com_frame,
                teleporting: self.teleporting,
            },
            owners.body,
        );
        self.animation_board_to_physics = target.animation_board_to_physics;
        self.extra_target_positions = [
            target.positions.com,
            target.positions.lifted_com,
            target.positions.following_com,
        ];
        owners.pose_errors.set_targets(target.positions);
        if !target.continuous {
            self.reset_physical_pose(owners, simulation);
            if p.state_2508 != 0 && p.state_2508 != 702 {
                self.invalid_target_reset = true;
            }
        }
        let contacts = std::array::from_fn(|i| {
            let query = p.line_tests_960_1008_1056[i];
            (query.valid != 0).then(|| query.position.map(f32::from_bits))
        });
        //GeneralUpdate82BDCAF8 skips UpdateIKData for702.
        if p.state_2508 != 702 {
            owners.ik.toe_contact_normal = collision.toe_contact_normal_1152;
        }
        let drives = owners.ik.update(
            owners.animated,
            globals,
            PhysicalInput {
                state_id: p.state_2508,
                flags_2468: p.flags_2468,
                flags_2472: p.flags_2472,
                flags_2480: p.flags_2480,
                //Unknown event bone -1 cannot equal either real toe index.
                contact_bone: owners.animation_input.contacts.bone as usize,
                physical_board: &owners.animated.board_frames.physical_board,
                toe_contact_normal: owners.ik.toe_contact_normal,
                current_contacts: contacts,
            },
        )?;
        self.drive_frames = drives.frames;
        owners.drives.update(
            &drives.frames,
            collision.partial_ragdoll,
            collision.drive_weight_4028,
        );
        skeleton_general::apply_gravity_displacement(owners.body, p.gravity_2648);
        Ok(drives)
    }

    ///Real82BE2798 reset is also callable by the coordinated player reset;
    ///it clears correction pending but does not invent a fresh state machine.
    pub fn reset_physical_pose(
        &mut self,
        owners: &mut SkeletonOwners<'_>,
        simulation: RetailSimulationStep,
    ) {
        //82BE27B0 clears the same16388 that states arm and PostWipeoutCheck
        //consumes. A discontinuity must not retain a detached lifecycle copy.
        owners.correction.pending = false;
        let s = &mut owners.animated;
        let observation = skeleton_general::reset_to_animation(
            owners.body,
            &mut s.record,
            &self.drive_frames,
            &s.roots.animation_to_world,
            s.board_frames.com_frame,
            simulation,
        );
        s.board_frames.centre_of_mass = observation.world_hips;
        s.board_frames.previous_centre_of_mass = observation.world_hips;
        s.board_frames.com_velocity = [0.; 4];
        self.reset_local_hips = observation.local_hips;
        owners.pose_errors.reset_history();
    }
}

pub(super) fn restore_board_target_drives(drives: &mut SkeletonDrives) {
    let hard = RetailDriveParams {
        spring_or_max_velocity: f32::from_bits(0x4415_FFFF),
        damping: 0.,
        max_strength: f32::from_bits(0x470C_9FFF),
        drive_type: RetailDriveType::HardDrive,
    };
    drives.targets.dynamics[0].linear = hard;
    drives.targets.dynamics[0].angular = hard;
}
