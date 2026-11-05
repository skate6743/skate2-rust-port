//! Ordinary local-player teleport82DB8998, operating on the live assemblies.
use super::*;
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{drive_frames::RetailAffineTransform, skeleton_animation_record::IDENTITY},
    player::input_phase::PlayerInputState,
};

impl Callbacks<'_, '_> {
    pub(super) fn reset_player(
        &mut self,
        board: &mut BoardRuntime,
        runtime: &mut GroundRuntime,
        requested: NativeMatrix,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
        p: &mut ProcessedPhysicsInput,
    ) -> Result<(), String> {
        self.life
            .skeleton_controller
            .request_ground(self.collision_mode)?;
        // The requested matrix is world facing. reset_physical still applies the
        // live switch flip (Processed2468 bit20), so pre-flip here to cancel it.
        let mut facing = horizontal_spawn(requested);
        // Biped targets use locomotion's packet mirror here. Riding Actor
        // targets have already been converted at the host reset boundary.
        // A checkpoint can restore a different leading foot than the live walk;
        // compensate the CURRENT reset packet, not the marker's former stance.
        // ProcessData has not populated Processed2476's mirror bit yet.
        // Actor packet10370 already carries this reset's fresh GivenStance.
        if p.byte_1600 & 1 == 0 && self.packet.flag_10370 != 0 {
            flip_heading(&mut facing);
        }
        let target = match_board_stance(facing, p.flags_2468);
        crate::skate3_native::trace::info!(
            on_board = p.byte_1600 & 1 != 0,
            requested_x = requested[3][0],
            requested_y = requested[3][1],
            requested_z = requested[3][2],
            requested_at_x = requested[2][0],
            requested_at_z = requested[2][2],
            applied_x = target[3][0],
            applied_y = target[3][1],
            applied_z = target[3][2],
            applied_at_x = target[2][0],
            applied_at_z = target[2][2],
            switch = p.flags_2468 & 0x0010_0000 != 0,
            "player teleport spawn"
        );
        player.flags_1296 = ((player.flags_1296 & 0xffef_ffff) & 0x81ff_ffff) | 0x6000_0000;
        player.previous_spin_input_1360 = 0.0;
        player.spin_same_direction_frames_1324 = 0;
        player.dismount_request_frames_1332 = 0;
        player.time_on_ground_1352 = 0.0;
        player.signed_ground_time_1356 = 0.0;
        player.queued_vector_1280 = [0; 4];
        player.hips_line_test_1488 = Default::default();
        player.left_line_test_1536 = Default::default();
        player.right_line_test_1584 = Default::default();
        crate::skate3_native::physics::player_input::reset_outputs(physical);
        board.reset_physical(
            self.settings.authored,
            affine(target),
            p.flags_2468,
            self.settings.step.simulation.gravity_acceleration,
        );
        crate::skate3_native::physics::ground_phase::reset_board_state(
            self.ground,
            runtime,
            self.life,
            self.wiping_out,
        );
        self.skeleton_air.reset_board(); //Board Reset82C0606C.
        self.footplant.full_reset();
        self.handplant.full_reset();
        self.riding.reset_for_teleport();
        {
            let mut owners = SkeletonOwners {
                animated: self.animated,
                body: self.body,
                drives: self.drives,
                ik: self.ik,
                animation_input: self.animation_input,
                correction: &mut self.output.correction,
                pose_errors: self.pose_errors,
            };
            self.skeleton_input.reset_for_teleport(
                &mut owners,
                self.collision_mode,
                self.feedback,
                &mut self.output.wobble,
                &mut self.life.skeleton_elapsed_16505,
            );
        }
        //ProcessData below reads the reset collision manager, not the snapshot
        //taken before this callback entered ResetSystems.
        self.collision.contact_4070 = self.feedback.flags.compliant;
        self.collision.has_pose_error_4077 = self.feedback.flags.has_impulse;
        self.collision.partial_ragdoll = self.collision_mode.partial_ragdoll;
        self.collision.drive_weight_4028 = self.feedback.drive_weight;
        //ResetSystems82DB92D0 preserves Reckoning filter histories. Seed the
        //fresh deck heading and recompute its actual Ground transform.
        self.riding.reckoning.reset();
        self.riding.update_ground_reckoning(
            board,
            crate::skate3_native::physics::riding_outputs::RidingPoseInputs {
                com_to_deck: Vector3::new(
                    self.animated.record.com_to_deck_world[0],
                    self.animated.record.com_to_deck_world[1],
                    self.animated.record.com_to_deck_world[2],
                ),
                body_spin: self.animation_input.extra.physical_body_spin,
            },
            p.flags_2468,
            self.animation_input.fields.balance,
            false,
            p,
        );
        self.wipeout.state.reset_systems();
        self.life.skeleton_controller.effective = 0;
        self.life.skeleton_controller.requested = 0;
        self.life.skeleton_controller.has_request = false;
        self.life.skeleton_controller.override_enabled = false;
        self.life.skeleton_controller.flag_18 = false;
        //82DB93E8/93EC resets Player1840 through82D749D0, not ballistic work.
        self.offboard_grab.invalidate();
        player.manager_1856_counter_320 = 0;
        player.probe = Default::default();

        let toolkit = runtime.prepare_toolkit(board, p);
        let packet = self.packet;
        self.process_skeleton(board, &toolkit, packet, physical, p)?;
        self.skeleton_input.update_teleport(
            board,
            &self.riding.reckoning_frames.system,
            p,
            &mut SkeletonOwners {
                animated: self.animated,
                body: self.body,
                drives: self.drives,
                ik: self.ik,
                animation_input: self.animation_input,
                correction: &mut self.output.correction,
                pose_errors: self.pose_errors,
            },
            self.globals,
            &self.collision,
            self.settings.step.simulation,
        )?;
        self.wipeout.state.teleport(); //82DB8DB4, separate from ResetSystems.
        self.teleported = true;
        Ok(())
    }
}

fn affine(m: NativeMatrix) -> RetailAffineTransform {
    RetailAffineTransform {
        basis: Basis3 {
            columns: std::array::from_fn(|i| [m[i][0], m[i][1], m[i][2]]),
        },
        translation: Vector3::new(m[3][0], m[3][1], m[3][2]),
    }
}
const SWITCH: u32 = 0x0010_0000;
const MIN_HEADING: f32 = f32::from_bits(0x3a83_126f); // native .001 reject

///82DB8A10..8BB4 flattens At before normalization, rejects length<=.001,
///then adds the original .1m world-Y teleport clearance. If At is nearly
///vertical, recover yaw from Ri so a downward look still has a heading.
pub(crate) fn horizontal_spawn(requested: NativeMatrix) -> NativeMatrix {
    let mut result = IDENTITY;
    let mut x = requested[2][0];
    let mut z = requested[2][2];
    let mut length = (x * x + z * z).sqrt();
    if length <= MIN_HEADING {
        // At = flatten(Ri) × Up => (-Ri.z, 0, Ri.x)
        x = -requested[0][2];
        z = requested[0][0];
        length = (x * x + z * z).sqrt();
    }
    if length > MIN_HEADING {
        result[2] = [x / length, 0.0, z / length, 0.0];
        result[0] = [result[2][2], 0.0, -result[2][0], 0.0];
        result[1] = [0.0, 1.0, 0.0, 0.0];
    }
    result[3] = requested[3];
    result[3][1] += 0.1;
    result
}

/// Processed2468 bit20 flips Ri/At on SetTransform. Invert first so a world
/// facing survives reset, whether the player is regular or switch.
pub(crate) fn match_board_stance(mut transform: NativeMatrix, flags_2468: u32) -> NativeMatrix {
    if flags_2468 & SWITCH != 0 {
        flip_heading(&mut transform);
    }
    transform
}

pub(crate) fn flip_heading(transform: &mut NativeMatrix) {
    for i in [0, 2] {
        transform[i] = transform[i].map(|v| -v);
    }
}

/// World-space skater facing for a teleport. Switch/mirror bits unflip the
/// live visual axes so the stored At is the direction they were facing.
pub(crate) fn facing_from_visual(
    mut transform: NativeMatrix,
    flags_2468: u32,
    flags_2476: u32,
    on_board: bool,
) -> NativeMatrix {
    if on_board && flags_2468 & SWITCH != 0 || !on_board && flags_2476 & 4 != 0 {
        flip_heading(&mut transform);
    }
    transform
}

#[cfg(any())]
mod tests {
    use super::*;

    #[test]
    fn flattens_horizontal_at_and_builds_right() {
        let mut requested = IDENTITY;
        requested[2] = [3.0, 1.0, 4.0, 0.0];
        requested[3] = [10.0, 2.0, 30.0, 1.0];
        let spawn = horizontal_spawn(requested);
        assert!((spawn[2][0] - 0.6).abs() < 1e-5);
        assert_eq!(spawn[2][1], 0.0);
        assert!((spawn[2][2] - 0.8).abs() < 1e-5);
        assert_eq!(spawn[0], [0.8, 0.0, -0.6, 0.0]);
        assert_eq!(spawn[3][1], 2.1);
    }

    #[test]
    fn recovers_yaw_from_right_when_looking_down() {
        let mut requested = IDENTITY;
        requested[0] = [0.0, 0.0, -1.0, 0.0];
        requested[1] = [-1.0, 0.0, 0.0, 0.0];
        requested[2] = [0.0, -1.0, 0.0, 0.0];
        let spawn = horizontal_spawn(requested);
        assert!((spawn[2][0] - 1.0).abs() < 1e-5);
        assert!(spawn[2][2].abs() < 1e-5);
    }

    #[test]
    fn switch_stance_cancels_across_reset() {
        let mut facing = IDENTITY;
        facing[0] = [0.0, 0.0, -1.0, 0.0];
        facing[2] = [1.0, 0.0, 0.0, 0.0];
        let prepared = match_board_stance(facing, SWITCH);
        let applied = match_board_stance(prepared, SWITCH);
        assert_eq!(applied[2], facing[2]);
        assert_eq!(applied[0], facing[0]);
        assert_eq!(match_board_stance(facing, 0)[2], facing[2]);
    }

    #[test]
    fn visual_switch_unflips_to_facing() {
        let mut visual = IDENTITY;
        visual[2] = [-1.0, 0.0, 0.0, 0.0];
        visual[0] = [0.0, 0.0, 1.0, 0.0];
        let facing = facing_from_visual(visual, SWITCH, 0, true);
        assert_eq!(facing[2], [1.0, 0.0, 0.0, 0.0]);
    }
}
