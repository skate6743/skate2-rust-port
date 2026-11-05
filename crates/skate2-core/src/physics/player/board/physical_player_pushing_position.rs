//! CalculateSkeletonPosition82d93690, decoded against the raw VMX128 registers.
//! Retains the native crouch/hand curves, fixed timestep and ordered writes.
use super::{InputCvarReferences, PushingCore, read_floats};
use crate::{
    input_cvar_runtime::pushing::{PushingBool as B, PushingFloat as F},
    physical_player_grab_spline::geometry::transform_point,
    physical_player_grind_manager::{
        math::{GrindNum, J, V4, ZERO, add, fsel, madd, neg1, scale, sub},
        view::{GrindError, error, need},
    },
    physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle,
    physics::PointGraph,
    reckoning_numeric::{try_clamp, wrap_turns_to_radians},
};

pub(super) fn graph(values: [f32; 20], input: f32) -> f32 {
    PointGraph::<8> {
        x: values[4..12].try_into().expect("eight graph abscissae"),
        y: values[12..20].try_into().expect("eight graph ordinates"),
    }
    .evaluate(input)
}

impl PushingCore {
    /// CalculateCollisionResponse82d87ca8. The two blend references are
    /// independent: old*CollisionBlendOut + HookToHips*CollisionBlendIn.
    /// Hex-Rays omits the first reference, which raw82d87cb0..cf0 resolves.
    pub(crate) fn try_calculate_collision_response(
        &mut self,
        hook_to_hips: V4,
        cvars: &InputCvarReferences,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let out = cvars
            .try_pushing_float(F::kMOVOBJ_CollisionBlendOut)
            .map_err(error)?;
        let blend_in = cvars
            .try_pushing_float(F::kMOVOBJ_CollisionBlendIn)
            .map_err(error)?;
        let old = need(self.skeleton_collision_offset, "Pushing collision offset")?;
        let minimum = cvars
            .try_pushing_float(F::kMOVOBJ_MinHookToHips)
            .map_err(error)?;
        let offset = madd(old, out, scale(hook_to_hips, blend_in));
        self.skeleton_collision_offset = Some(offset);
        self.collision_offset_active = Some(num.dot(offset, offset)? > minimum * minimum);
        Ok(())
    }

    /// ConsiderEmergencyExit82d87d80 latches the exit flag; the inactive path
    /// deliberately neither clears it nor changes the repush delay.
    pub(crate) fn try_consider_emergency_exit(
        &mut self,
        hook_to_hips: V4,
        cvars: &InputCvarReferences,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let squared = num.dot(hook_to_hips, hook_to_hips)?;
        if cvars
            .try_pushing_bool(B::kEnableEmergencyExit)
            .map_err(error)?
        {
            let minimum = cvars
                .try_pushing_float(F::kMOVOBJ_MinHookToHipsForExit)
                .map_err(error)?;
            if squared > minimum * minimum {
                let delay = if cvars
                    .try_pushing_bool(B::kEnableRepushingTimeLimitOnExit)
                    .map_err(error)?
                {
                    read_floats::<1>(
                        need(self.attribs.as_ref(), "Pushing attributes")?,
                        0xf081_ca3e_c9e5_1bfb,
                    )
                    .map_err(error)?[0]
                } else {
                    0.0
                };
                self.delay_until_repushable = Some(delay);
                self.emergency_exit = Some(true);
            }
        }
        Ok(())
    }

    /// Inputs are the actual Processed effective-animation matrix/state timer
    /// and retained OffBoardFeetIK height, sampled by Pushing::Update.
    pub(crate) fn try_calculate_skeleton_position(
        &mut self,
        effective: [V4; 4],
        state_timer: f32,
        height_above_ground: f32,
        cvars: &InputCvarReferences,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let hands = need(self.blended_spline_hands_transform, "Pushing blended hands")?;
        self.hand_left_target = Some(transform_point(
            &hands,
            need(self.blended_hand_left_offset, "Pushing blended left offset")?,
        ));
        self.hand_right_target = Some(transform_point(
            &hands,
            need(
                self.blended_hand_right_offset,
                "Pushing blended right offset",
            )?,
        ));
        let left = need(self.hand_l_raw, "Pushing raw left hand")?;
        let right = need(self.hand_r_raw, "Pushing raw right hand")?;
        let mut flat_left = left;
        flat_left[1] = 0.0;
        let mut flat_right = right;
        flat_right[1] = 0.0;
        let flat_mid = scale(add(flat_left, flat_right), 0.5);
        let max_height = cvars
            .try_pushing_float(F::kMOVOBJ_MaxHeightAboveGround)
            .map_err(error)?;
        let lower = fsel(
            -max_height - height_above_ground,
            -max_height,
            height_above_ground,
        );
        let height = fsel(max_height - lower, lower, max_height);
        self.hands_dir = Some(effective[0]);
        let attrs = need(self.attribs.as_ref(), "Pushing attributes")?;
        let distance = read_floats::<1>(attrs, 0x96ec_c988_38ec_cc11).map_err(error)?[0];
        let dt = f32::from_bits(0x3c88_8889); // kUpdateTime, 1/60
        let blend = cvars
            .try_pushing_float(F::kMOVOBJ_ApproxSpeedBlend)
            .map_err(error)?;
        let normal = need(self.target_push_normal, "Pushing target push normal")?;
        let mut new_position = madd(normal, distance, hands[3]);
        new_position[1] = 0.0;
        let inverse_dt = num.reciprocal(dt)?;
        // The native first entry reads an unwritten target normal in UpdateSpline.
        // Its only consumer here is this derivative. No speed is read before
        // the NoSpeedTimeout overwrite below, so propagate unknown until then.
        self.approximate_skeleton_speed = self
            .approximate_skeleton_position
            .zip(self.approximate_skeleton_speed)
            .map(|(old_position, old_speed)| {
                let delta = scale(sub(new_position, old_position), blend);
                madd(old_speed, 1.0 - blend, scale(delta, inverse_dt))
            });
        self.approximate_skeleton_position = Some(new_position);

        let distance = read_floats::<1>(attrs, 0x96ec_c988_38ec_cc11).map_err(error)?[0];
        let full_mid = scale(add(left, right), 0.5);
        let crouch_scalar = cvars
            .try_pushing_float(F::kMOVOBJ_Crouch_Scalar)
            .map_err(error)?;
        let anchor = sub(new_position, scale(normal, distance));
        let mut correction = sub(anchor, flat_mid);
        // 82d939ec stores the vector; 82d939f0 loads its Y lane. Hex-Rays
        // misses the store and incorrectly prints multiplication by zero.
        let mut crouch_delta = neg1(crouch_scalar * sub(hands[3], full_mid)[1]);
        let position_curve = read_floats::<20>(attrs, 0x702f_25ba_3a5a_aa56).map_err(error)?;
        let direction_curve = read_floats::<20>(attrs, 0x1348_e9a1_f213_b42d).map_err(error)?;
        let end = fsel(
            direction_curve[2] - position_curve[2],
            direction_curve[2],
            position_curve[2],
        );
        let grab_z = need(self.world_grab_z, "Pushing world grab Z")?;
        // Native bgt skips only ordered greater; unordered timer enters.
        if grab_z > 0.0 && !(state_timer > end) {
            // Source reads these attributes again before evaluating each graph.
            let direction_blend = graph(
                read_floats::<20>(attrs, 0x1348_e9a1_f213_b42d).map_err(error)?,
                state_timer,
            );
            let position_blend = graph(
                read_floats::<20>(attrs, 0x702f_25ba_3a5a_aa56).map_err(error)?,
                state_timer,
            );
            let initial_hand = transform_point(
                &effective,
                [
                    0.0,
                    need(self.world_grab_y, "Pushing world grab Y")?,
                    grab_z
                        + cvars
                            .try_pushing_float(F::kMOVOBJ_HandsZOffset)
                            .map_err(error)?,
                    0.0,
                ],
            );
            if cvars
                .try_pushing_bool(B::kOBPushingDebugDraw)
                .map_err(error)?
            {
                return Err(error("Pushing debug overlay singleton is not connected"));
            }
            self.hands_dir = Some(madd(
                need(self.hands_dir, "Pushing hands direction")?,
                1.0 - direction_blend,
                scale(effective[0], direction_blend),
            ));
            correction = madd(
                correction,
                1.0 - direction_blend,
                scale(sub(anchor, initial_hand), direction_blend),
            );
            correction[1] = 0.0;
            let initial_scalar = cvars
                .try_pushing_float(F::kMOVOBJ_Crouch_Scalar_Initial)
                .map_err(error)?;
            let ik_threshold = cvars
                .try_pushing_float(F::kMOVOBJ_HandsLevelToBeginIKBlend)
                .map_err(error)?;
            let remaining = 1.0 - position_blend;
            crouch_delta = (initial_scalar * neg1(sub(initial_hand, full_mid)[1]))
                .mul_add(position_blend, remaining * crouch_delta);
            if remaining > ik_threshold {
                self.hand_left_target = Some(madd(
                    need(self.hand_left_target, "Pushing left target")?,
                    remaining,
                    scale(left, position_blend),
                ));
                self.hand_right_target = Some(madd(
                    need(self.hand_right_target, "Pushing right target")?,
                    remaining,
                    scale(right, position_blend),
                ));
                self.hand_ik_targets_valid = Some(true);
            }
        }
        if state_timer
            < cvars
                .try_pushing_float(F::kMOVOBJ_NoSpeedTimeout)
                .map_err(error)?
        {
            self.approximate_skeleton_speed = Some(ZERO);
        }
        let crouch = need(self.anim_crouch, "Pushing crouch")? + crouch_delta;
        self.anim_crouch = Some(crouch);
        let lower = fsel(neg1(crouch), 0.0, crouch);
        self.anim_crouch = Some(fsel(1.0 - lower, lower, 1.0));
        let alignment_curve = read_floats::<20>(attrs, 0x023d_7396_6b86_69a5).map_err(error)?;
        let time = need(self.time_grabbing_this_spline, "Pushing spline time")?;
        let alignment = graph(alignment_curve, time);
        let angle = plane_angle(
            need(self.hands_dir, "Pushing hands direction")?.into(),
            need(self.spline_dir, "Pushing spline direction")?.into(),
            J.into(),
            num.n,
        )
        .map_err(error)?;
        self.align_angle =
            Some(wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983)) * alignment);
        let movement_curve = read_floats::<20>(attrs, 0x4e91_b4a6_05e5_4cc2).map_err(error)?;
        let movement = graph(movement_curve, time);
        let max_speed = read_floats::<1>(attrs, 0x4243_8d4c_6b20_9c09).map_err(error)?[0];
        let delta = try_clamp(scale(correction, movement), max_speed * dt, num.n).map_err(error)?;
        self.world_position_delta = Some(delta);
        let delta = add(
            delta,
            need(self.skeleton_collision_offset, "Pushing collision offset")?,
        );
        self.world_position_delta = Some(delta);
        let delta = add(delta, [0.0, neg1(height), 0.0, 0.0]);
        self.world_position_delta = Some(delta);
        let minimum = cvars
            .try_pushing_float(F::kMOVOBJ_worldPositionMinDelta)
            .map_err(error)?;
        if minimum * minimum > num.dot(delta, delta)? {
            // 82d93ff4 resolves the distinct MinBlend reference830d94ec.
            let blend = cvars
                .try_pushing_float(F::kMOVOBJ_worldPositionMinBlend)
                .map_err(error)?;
            self.world_position_delta = Some(scale(delta, blend));
        }
        Ok(())
    }
}
