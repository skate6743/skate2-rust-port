//! Pushing ctor82d9c980/Reset82d91d70; real PID and conditional delay attributes.
use super::{
    Arc, AttributeRuntimeVault, DEFAULT_COLLECTION, GrabDescriptor, GrabSpline, IDENTITY,
    InputCvarReferences, OffBoardPushing, OwnerMember, PushingCore, ScalarPid, StateConstructorError, VectorPid,
    ZERO, read_floats,
};
impl OffBoardPushing {
    pub(super) fn construct(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<(), StateConstructorError> {
        self.player_grab_spline = Some(self.base.link(OwnerMember::PlayerGrabSpline));
        self.off_board_feet_ik = Some(self.base.link(OwnerMember::OffBoardFeetIK));
        self.skateboard_controller = Some(self.base.link(OwnerMember::SkateboardController));
        self.core.construct(vault, cvars)
    }
}

impl PushingCore {
    /// The constructor's own fields: the attributes, the grab spline, both PIDs, then Reset.
    #[expect(
        clippy::too_many_lines,
        reason = "ordered constructor and Reset preserve partial attribute ownership"
    )]
    pub(crate) fn construct(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<(), StateConstructorError> {
        self.attribs = Some(vault.instance(0x3eda_5b14_0604_613d, DEFAULT_COLLECTION));
        self.attribs_off_board = Some(vault.instance(0x4920_26e0_f096_f283, DEFAULT_COLLECTION));
        self.grab_spline_desc = Some(GrabDescriptor::INVALID);
        self.grab_spline = Some(GrabSpline::in_uninitialized_allocation());
        let control = read_floats::<4>(
            self.attribs.as_ref().expect("constructed attributes"),
            0xdf79_539d_bda0_06ee,
        )?;
        self.pid_object_vel = Some(VectorPid {
            control,
            output: ZERO,
            last_error: ZERO,
            integral: ZERO,
            derivative: ZERO,
        });
        let control = read_floats::<4>(
            self.attribs.as_ref().expect("constructed attributes"),
            0xb467_6428_5ad1_dc5f,
        )?;
        self.pid_object_angular_vel = Some(ScalarPid {
            control,
            output: 0.0,
            last_error: 0.0,
            integral: 0.0,
            derivative: 0.0,
        });
        self.reset(cvars)
    }

    /// Reset82d91d70 is shared by construction and every Enter82da07b8.
    /// Keep the existing PID controls and collaborator links.
    pub(crate) fn reset(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), StateConstructorError> {
        self.previous_spline_hands_transform = Some(IDENTITY);
        self.current_spline_hands_transform = Some(IDENTITY);
        self.blended_spline_hands_transform = Some(IDENTITY);
        self.grab_spline_desc = Some(GrabDescriptor::INVALID);
        self.grab_spline
            .as_mut()
            .expect("constructed grab spline")
            .reset();
        self.reach_base = Some(ZERO);
        self.world_position_delta = Some(ZERO);
        self.last_error = Some(ZERO);
        self.spline_dir = Some(ZERO);
        self.applied_force = Some(ZERO);
        self.target_velocity = Some(ZERO);
        self.hand_l_raw = Some(ZERO);
        self.hand_r_raw = Some(ZERO);
        self.previous_hand_left_offset = Some(ZERO);
        self.current_hand_left_offset = Some(ZERO);
        self.blended_hand_left_offset = Some(ZERO);
        self.previous_hand_right_offset = Some(ZERO);
        self.current_hand_right_offset = Some(ZERO);
        self.blended_hand_right_offset = Some(ZERO);
        self.hand_left_target = Some(ZERO);
        self.hand_right_target = Some(ZERO);
        self.hands_dir = Some(ZERO);
        self.approximate_skeleton_position = Some(ZERO);
        self.approximate_skeleton_speed = Some(ZERO);
        self.smoothed_force = Some(ZERO);
        self.skeleton_collision_offset = Some(ZERO);
        self.acceleration = Some(ZERO);
        // Both already-reset PID histories are reset again; control data is preserved.
        let pid = self.pid_object_vel.as_mut().expect("constructed PID");
        pid.output = ZERO;
        pid.last_error = ZERO;
        pid.integral = ZERO;
        pid.derivative = ZERO;
        let pid = self
            .pid_object_angular_vel
            .as_mut()
            .expect("constructed PID");
        pid.output = 0.0;
        pid.last_error = 0.0;
        pid.integral = 0.0;
        pid.derivative = 0.0;
        self.spline_length = Some(0.0);
        self.align_angle = Some(0.0);
        self.grip_offset_along_spline = Some(0.0);
        self.ik_blend_value = Some(0.0);
        self.anim_crouch = Some(0.5);
        self.time_grabbing_this_spline = Some(0.0);
        self.object_mass = Some(1.0);
        self.force_mass_scale = Some(1.0);
        self.object_y_tensor = Some(1.0);
        self.torque_scale = Some(1.0);
        self.max_rot_speed = Some(0.0);
        self.spline_blend_coeff = Some(-1.0);
        self.spline_blend_adjust = Some(-1.0);
        self.time_without_forces_requested = Some(0.0);
        self.time_in_partial_ragdoll = Some(0.0);
        self.world_grab_z = Some(0.0);
        self.world_grab_y = Some(0.0);
        let delay = if cvars.try_enable_repushing_time_limit()? {
            read_floats::<1>(
                self.attribs.as_ref().expect("constructed attributes"),
                0xb673_0894_f9df_e0b3,
            )?[0]
        } else {
            0.0
        };
        self.delay_until_repushable = Some(delay);
        self.emergency_exit = Some(false);
        self.grab_spline_swapped = Some(false);
        self.is_blending_spline = Some(false);
        self.collision_offset_active = Some(false);
        self.grab_object = Some(false);
        self.hand_ik_targets_valid = Some(false);
        self.hand_ik_targets_locked = Some(false);
        Ok(())
    }
}
