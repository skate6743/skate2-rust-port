//! MoveObject82d923d0..2f18. Raw VMX/FMA graph, native PID histories and
//! deferred manager force route; no independent DMO body or movement solver.
use super::pushing_position::graph;
use super::{GrabDescriptor, InputCvarReferences, PushingCore, VectorPid, read_floats};
use crate::{
    input_cvar_runtime::pushing::{PushingBool as B, PushingFloat as F},
    physical_player_grab_spline::{PlayerGrabSpline, manager::GrabMessageKind},
    physical_player_grind_manager::{
        math::{GrindNum, J, V4, ZERO, abs1, add, cross, fsel, madd, neg, neg1, scale, sub},
        view::{GrindError, error, need},
    },
    physical_player_trajectory_owners::selector_grind::vmax,
    reckoning_numeric::{
        try_clamp, try_normalize_safe, try_signed_angle, try_unsigned_angle, wrap_turns_to_radians,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
};

impl VectorPid {
    /// PIDController<Vector3>::Update82d89b98 (not Reckoning's PIDFilter).
    /// The input is already an error; output accumulates P/I/D corrections.
    pub(crate) fn update(&mut self, error: V4) -> V4 {
        self.integral = madd(
            self.integral,
            1.0 - self.control[3],
            scale(error, self.control[3]),
        );
        self.derivative = sub(error, self.last_error);
        self.last_error = error;
        let p_i = madd(
            error,
            self.control[0],
            scale(self.integral, self.control[1]),
        );
        let correction = madd(self.derivative, self.control[2], p_i);
        self.output = add(self.output, correction);
        self.output
    }
}

/// The animation's object move controls (`MoveObjectZ`, `MoveObjectX`, `MoveObjectRot`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct MoveControls {
    pub(crate) z: f32,
    pub(crate) x: f32,
    pub(crate) rotation: f32,
}

impl PushingCore {
    pub(crate) fn try_move_object(
        &mut self,
        processed: &SkeletonProcessedContact,
        grab_owner: &PlayerGrabSpline,
        refs: &InputCvarReferences,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let fields = processed.animation_attribute_fields();
        let controls = MoveControls {
            z: need(fields.move_object_z, "MoveObjectZ")?,
            x: need(fields.move_object_x, "MoveObjectX")?,
            rotation: need(fields.move_object_rotation, "MoveObjectRot")?,
        };
        let ground_y = need(
            processed.prior_output().ground_position_all_states(),
            "ground position all states+1f0",
        )?
        .xyz[1];
        let (descriptor, message) = self.try_move_object_with(controls, ground_y, refs, num)?;
        grab_owner.defer_message(descriptor, message)
    }

    /// The same MoveObject over plain inputs: the grabbed spline's descriptor and the force
    /// message for it. TU3's PhysState_OffBoard_Pushing::MoveObject (`0x82d45318`) is this
    /// function with the pushing cvars at their defaults.
    pub(crate) fn try_move_object_with(
        &mut self,
        controls: MoveControls,
        ground_y: f32,
        refs: &InputCvarReferences,
        num: &mut GrindNum<'_>,
    ) -> Result<(GrabDescriptor, GrabMessageKind), GrindError> {
        let attrs = need(self.attribs.as_ref(), "Pushing attributes")?;
        let scalar = |key| -> Result<f32, GrindError> {
            Ok(read_floats::<1>(attrs, key).map_err(error)?[0])
        };
        let curve = |key, input| -> Result<f32, GrindError> {
            Ok(graph(read_floats::<20>(attrs, key).map_err(error)?, input))
        };
        let forward_limit = scalar(0x2258_076b_6125_69a9)?;
        let backward_limit = scalar(0xf1c0_3872_2ec7_d0c6)?;
        let sideways_limit = scalar(0x096a_4fa6_489e_5541)?;
        let spline = need(self.grab_spline.as_ref(), "Pushing grab spline")?;
        // Native+100 is BLENDED hands translation, not current hands+C0.
        let hands_position = need(self.blended_spline_hands_transform, "Pushing blended hands")?[3];
        let relative_body = sub(spline.body_position, hands_position);
        let lateral = neg(need(self.spline_dir, "Pushing spline direction")?);
        let max_rotation = need(self.max_rot_speed, "Pushing maximum rotation")?;
        let move_z = controls.z;
        let move_x = controls.x;
        let lever = num.dot(lateral, relative_body)?;
        let sign = fsel(lever, 1.0, -1.0);
        let unsigned_lever = lever * sign;
        let rotation_from_push = curve(0xe4ff_0185_da44_cdbd, unsigned_lever)? * sign;
        let rotation_input = controls.rotation + rotation_from_push * move_z;
        // Source clamps ONLY the upper end here, via fsel(1-value,value,1).
        let rotation_input = fsel(1.0 - rotation_input, rotation_input, 1.0);
        let rotation_speed =
            neg1((curve(0xbfb3_bef0_bb26_61c0, unsigned_lever)? * rotation_input) * max_rotation);
        let lever_velocity = lever * rotation_speed;
        let forward = move_z
            * if move_z > 0.0 {
                forward_limit
            } else {
                backward_limit
            };
        let sideways = move_x * sideways_limit;
        let push_from_turn = refs
            .try_pushing_float(F::kPushFromTurnScalar)
            .map_err(error)?;
        let forward = neg1(push_from_turn.mul_add(lever_velocity, neg1(forward))); // fnmsubs82d925f8
        let maximum_torque = scalar(0xad32_7350_d151_b1e3)?;
        let maximum_force = scalar(0x7914_21dd_af54_c2d5)?;
        if !need(self.grab_object, "Pushing grab flag")? {
            return Err(error("MoveObject requires mGrabObject (native assert595)"));
        }
        let current_hands = need(self.current_spline_hands_transform, "Pushing current hands")?;
        let current_normal = neg(current_hands[2]);
        let mut correction_torque = ZERO;
        let turning;
        // bge is not-less: unordered rotation also takes the large-turn path.
        if abs1(rotation_speed) < f32::from_bits(0x3dcc_cccd) {
            let target_normal = need(self.target_push_normal, "Pushing target normal")?;
            let angle = try_unsigned_angle(current_normal, target_normal, num.n).map_err(error)?;
            let angle = wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983));
            let tolerance = scalar(0x557f_a142_008f_d7ce)?;
            if tolerance > abs1(angle) {
                let torque_axis = cross(target_normal, current_normal);
                let strength = scalar(0xd982_1c96_3b1a_f858)?;
                correction_torque = scale(
                    torque_axis,
                    strength * need(self.force_mass_scale, "Pushing force mass scale")?,
                );
                correction_torque[0] = 0.0;
                correction_torque[2] = 0.0;
            } else {
                self.target_push_normal = Some(current_normal);
                self.target_x_direction = Some(neg(current_hands[0]));
            }
            turning = false;
        } else {
            self.target_push_normal = Some(current_normal);
            self.target_x_direction = Some(neg(current_hands[0]));
            turning = true;
        }
        let mut velocity = madd(
            neg(need(self.target_push_normal, "Pushing target normal")?),
            forward,
            scale(need(self.target_x_direction, "Pushing target X")?, sideways),
        );
        let moving = !(f32::from_bits(0x38d1_b717) > num.dot(velocity, velocity)?);
        let force_blend = if turning || moving {
            self.time_without_forces_requested = Some(0.0);
            1.0
        } else {
            let time = need(
                self.time_without_forces_requested,
                "Pushing idle-force time",
            )? + f32::from_bits(0x3c88_8889);
            self.time_without_forces_requested = Some(time);
            curve(0x4563_2781_9b75_0b4e, time)?
        };
        let correction_torque = scale(correction_torque, force_blend);
        if need(self.collision_offset_active, "Pushing collision flag")?
            && refs
                .try_pushing_bool(B::kOBP_EnableCollResponse)
                .map_err(error)?
        {
            let amount = refs
                .try_pushing_float(F::kOBP_CollisionVelScalar)
                .map_err(error)?;
            let collision_velocity = scale(
                need(self.skeleton_collision_offset, "Pushing collision offset")?,
                amount,
            );
            let normal = try_normalize_safe(collision_velocity, ZERO, num.n).map_err(error)?;
            let along = num.dot(velocity, normal)?;
            let collision_along = num.dot(collision_velocity, normal)?;
            velocity = madd(
                normal,
                vmax(along, collision_along),
                sub(velocity, scale(normal, along)),
            );
            velocity[1] = 0.0;
        }
        let mut force = need(self.pid_object_vel.as_mut(), "Pushing velocity PID")?
            .update(sub(velocity, spline.velocity));
        force[1] = 0.0;
        let force = try_clamp(force, maximum_force, num.n).map_err(error)?;
        let mass_scale = need(self.force_mass_scale, "Pushing force mass scale")?;
        let mass = need(self.object_mass, "Pushing object mass")?;
        let force = scale(force, force_blend);
        let desired_acceleration = scale(scale(force, mass_scale), 1.0 / mass);
        let old_acceleration = need(self.acceleration, "Pushing acceleration")?;
        let maximum_jerk = refs.try_pushing_float(F::kMaxJerkPerFrame).map_err(error)?;
        let acceleration_change = try_clamp(
            sub(desired_acceleration, old_acceleration),
            maximum_jerk,
            num.n,
        )
        .map_err(error)?;
        let acceleration = add(old_acceleration, acceleration_change);
        self.acceleration = Some(acceleration);
        // Keep both multiplications and refined reciprocal; do not cancel mass
        // algebraically. The result feeds BACK into the velocity PID at+3c0.
        let inverse_scale = num.reciprocal(mass_scale)?;
        let force = scale(scale(acceleration, mass), inverse_scale);
        let smoothing = refs
            .try_pushing_float(F::kMOVOBJ_ForceSmoothing)
            .map_err(error)?;
        self.smoothed_force = Some(madd(
            force,
            smoothing,
            scale(
                need(self.smoothed_force, "Pushing smoothed force")?,
                1.0 - smoothing,
            ),
        ));
        need(self.pid_object_vel.as_mut(), "Pushing velocity PID")?.output = force;
        if std::env::var_os("SKATE2_TRACE_PUSHING_MOVE").is_some() {
            eprintln!(
                "pushing move: target {velocity:?} object {:?} desired accel {desired_acceleration:?} accel {acceleration:?} force {force:?} mass {mass} mass scale {mass_scale} blend {force_blend}",
                spline.velocity
            );
        }

        let body_height = ground_y - spline.body_position[1];
        let height_scale = scalar(0xfde8_07d9_b85a_6ac2)?;
        let force_position = [0.0, body_height * height_scale, 0.0, 0.0];
        let angle = try_signed_angle(
            need(self.last_push_normal, "Pushing last normal")?,
            current_normal,
            J,
            num.n,
        )
        .map_err(error)?;
        let angle = wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983));
        self.last_push_normal = Some(current_normal);
        let pid = need(self.pid_object_angular_vel.as_mut(), "Pushing angular PID")?;
        let integral_keep = 1.0 - pid.control[3];
        let error_value = neg1(angle.mul_add(f32::from_bits(0x426f_ffff), neg1(rotation_speed)));
        let derivative = error_value - pid.last_error;
        pid.last_error = error_value;
        pid.derivative = derivative;
        pid.integral = integral_keep.mul_add(pid.integral, pid.control[3] * error_value);
        // Scalar inline order is D+I then P, unlike vector PID's P+I then D.
        let d_i = pid.control[2].mul_add(derivative, pid.control[1] * pid.integral);
        pid.output = pid.control[0].mul_add(error_value, d_i) + pid.output;
        let lower = fsel(-maximum_torque - pid.output, -maximum_torque, pid.output);
        pid.output = fsel(maximum_torque - lower, lower, maximum_torque) * force_blend;
        let torque = try_clamp(
            add([0.0, pid.output, 0.0, 0.0], correction_torque),
            maximum_torque,
            num.n,
        )
        .map_err(error)?;
        let torque = scale(torque, need(self.torque_scale, "Pushing torque scale")?);
        let force = scale(
            force,
            need(self.force_mass_scale, "Pushing force mass scale")?,
        );
        Ok((
            GrabDescriptor {
                object_type: spline.descriptor_type,
                object_id: spline.descriptor_object,
            },
            GrabMessageKind::ApplyForce {
                force: force.into(),
                position: force_position.into(),
                torque: torque.into(),
            },
        ))
    }
}
