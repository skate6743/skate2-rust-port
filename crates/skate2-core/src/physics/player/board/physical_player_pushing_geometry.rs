//! Pushing501 spline orientation, hand targets and blend lifecycle. These
//! operate on the existing retained state, not a replacement grab controller.
use super::{GrabDescriptor, PushingCore, read_floats};
use crate::{
    physical_player_grab_spline::{
        GrabSpline, PlayerGrabSpline,
        geometry::{GrabBox, inverse_transform_point, world_horizontal_normal},
        manager::GrabMessageKind,
    },
    physical_player_grind_manager::{
        math::{DEG_TO_RAD, GrindNum, J, V4, ZERO, add, cross, fsel, madd, neg, neg1, scale, sub},
        view::{GrindError, error, need},
    },
    physical_player_trajectory_owners::selector_grind::{vmax, vmin},
    physics::PointGraph,
    reckoning_numeric::try_normalize_safe,
    skeleton_affine_interpolation::try_interpolate_affine,
};

impl PushingCore {
    /// GenerateGrabbableBox82d92218 derives forward from the current animation
    /// hands and effective up, then uses the shared box initializer with pitch off.
    pub(crate) fn try_generate_grabbable_box(
        &self,
        effective: [V4; 4],
        raw_pose: &mut impl FnMut(usize) -> Result<V4, GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<GrabBox, GrindError> {
        let left = raw_pose(3)?;
        let right = raw_pose(7)?;
        let z = cross(sub(left, right), effective[1]);
        let x = try_normalize_safe(cross(effective[1], z), ZERO, num.n).map_err(error)?;
        let z = try_normalize_safe(z, ZERO, num.n).map_err(error)?;
        let attrs = need(
            self.attribs_off_board.as_ref(),
            "Pushing offboard attributes",
        )?;
        let dimensions = read_floats::<4>(attrs, 0xa03e_a2ac_5576_e738).map_err(error)?;
        let offset = read_floats::<4>(attrs, 0xfe50_188a_8336_fdf9).map_err(error)?;
        GrabBox::from_player(
            [x, effective[1], z, effective[3]],
            offset,
            dimensions,
            false,
            num,
        )
    }

    /// Pushing's wider angle attributes differ from locomotion's initial query.
    /// Both IsGrabSpline* helpers and Update's next query use this ordered read.
    pub(crate) fn try_grab_angle_limits(&self) -> Result<(f32, f32), GrindError> {
        let attrs = need(
            self.attribs_off_board.as_ref(),
            "Pushing offboard attributes",
        )?;
        let horizontal = read_floats::<1>(attrs, 0x6b82_e41e_1da0_db91).map_err(error)?[0];
        let normal = read_floats::<1>(attrs, 0x5aca_7c5d_3094_e026).map_err(error)?[0];
        Ok((normal * DEG_TO_RAD, horizontal * DEG_TO_RAD))
    }

    /// IsGrabSplineGrabbable82d877a0 versus StillGrabbable82d87908: only a new
    /// spline searches/clamps nearest distance; the retained one tests the grip.
    pub(crate) fn try_is_spline_grabbable(
        &self,
        spline: &GrabSpline,
        bounds: &GrabBox,
        still_grabbed: bool,
        num: &mut GrindNum<'_>,
    ) -> Result<bool, GrindError> {
        let (normal, horizontal) = self.try_grab_angle_limits()?;
        let reach = need(self.reach_base, "Pushing reach base")?;
        if still_grabbed {
            spline.can_grab_at(
                reach,
                need(self.grip_offset_along_spline, "Pushing grip offset")?,
                bounds,
                normal,
                horizontal,
                num,
            )
        } else {
            let exclusion = read_floats::<1>(
                need(
                    self.attribs_off_board.as_ref(),
                    "Pushing offboard attributes",
                )?,
                0x2523_6f55_4677_f74f,
            )
            .map_err(error)?[0];
            spline.can_grab(reach, bounds, exclusion, normal, horizontal, num)
        }
    }

    /// UpdateSpline82d9cb70. Pose callback is the existing Skeleton
    /// GetAnimPartTransformWorldRaw translation, requested in native order.
    pub(crate) fn try_update_spline(
        &mut self,
        first: bool,
        grab: &PlayerGrabSpline,
        raw_pose: &mut impl FnMut(usize) -> Result<V4, GrindError>,
        effective_x: V4,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        self.try_update_spline_with(first, &mut |descriptor, message| grab.defer_message(descriptor, message), raw_pose, effective_x, num)
    }

    /// The same UpdateSpline with the grab spline's messages handed to `send`.
    pub(crate) fn try_update_spline_with(
        &mut self,
        first: bool,
        send: &mut dyn FnMut(GrabDescriptor, GrabMessageKind) -> Result<(), GrindError>,
        raw_pose: &mut impl FnMut(usize) -> Result<V4, GrindError>,
        effective_x: V4,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let spline = need(self.grab_spline.as_ref(), "Pushing grab spline")?;
        let descriptor = GrabDescriptor {
            object_type: spline.descriptor_type,
            object_id: spline.descriptor_object,
        };
        self.grab_spline_desc = Some(descriptor);
        self.reach_base = Some(raw_pose(23)?); // PART_HIPS
        if first {
            send(descriptor, GrabMessageKind::Activate)?;
            self.spline_length = Some(spline.length);
            self.grab_spline_swapped = Some(spline.status & 0x20 != 0);
            self.object_mass = Some(1.0 / spline.inertia.inverse_mass);
            let attrs = need(self.attribs.as_ref(), "Pushing attributes")?;
            let graph = |key, input| -> Result<f32, GrindError> {
                let values = read_floats::<20>(attrs, key).map_err(error)?;
                Ok(PointGraph::<8> {
                    x: values[4..12].try_into().expect("eight graph abscissae"),
                    y: values[12..20].try_into().expect("eight graph ordinates"),
                }
                .evaluate(input))
            };
            self.force_mass_scale = Some(graph(0x57d3_7d69_6363_167e, self.object_mass.unwrap())?);
            self.object_y_tensor = Some(1.0 / spline.inertia.inverse_tensor[1]);
            self.torque_scale = Some(graph(0x1cec_e1cc_07c5_cd94, self.object_y_tensor.unwrap())?);
            self.max_rot_speed = Some(graph(0xeabf_cc79_873a_2859, self.object_y_tensor.unwrap())?);
            let mut offset = 0.0;
            let mut nearest =
                spline.nearest_point(self.reach_base.unwrap(), Some(&mut offset), num)?;
            nearest[1] = 0.0;
            self.approximate_skeleton_position = Some(nearest);
            let mut exclusion = read_floats::<1>(
                need(
                    self.attribs_off_board.as_ref(),
                    "Pushing offboard attributes",
                )?,
                0x2523_6f55_4677_f74f,
            )
            .map_err(error)?[0];
            let half = spline.length * 0.5;
            if exclusion > half {
                exclusion = half;
            }
            self.hand_ik_targets_valid = Some(false);
            self.hand_ik_targets_locked = Some(false);
            self.grip_offset_along_spline =
                Some(vmin(spline.length - exclusion, vmax(exclusion, offset)));
        }
        let spline = need(self.grab_spline.as_mut(), "Pushing grab spline")?;
        let reach = self.reach_base.unwrap();
        if 0.0 > cross(sub(spline.p1, reach), sub(spline.p2, reach))[1] {
            std::mem::swap(&mut spline.p1, &mut spline.p2);
            spline.length_unit = Some(neg(need(spline.length_unit, "grab length unit")?));
            spline.status ^= 0x20;
        }
        let swapped = spline.status & 0x20 != 0;
        if swapped != need(self.grab_spline_swapped, "Pushing previous reversal")? {
            self.grip_offset_along_spline = Some(
                need(self.spline_length, "Pushing spline length")?
                    - need(self.grip_offset_along_spline, "Pushing grip offset")?,
            );
            self.grab_spline_swapped = Some(swapped);
        }
        if first {
            self.try_compute_target_hand_ik(raw_pose, effective_x, num)?;
            let distance = read_floats::<1>(
                need(self.attribs.as_ref(), "Pushing attributes")?,
                0x96ec_c988_38ec_cc11,
            )
            .map_err(error)?[0];
            let position = need(
                self.approximate_skeleton_position,
                "Pushing approximate position",
            )?;
            // Enter initializes targetPushNormal only AFTER this call. The
            // allocator does not clear it on first construction. Preserve that
            // unknown value until CalculateSkeletonPosition's native timeout
            // overwrites the affected speed; do not invent a zero normal.
            self.approximate_skeleton_position = self.target_push_normal.map(|normal| {
                let mut result = madd(normal, distance, position);
                result[1] = 0.0;
                result
            });
        }
        Ok(())
    }

    /// ComputePushNormal82d87678: horizontal normal faces the reach base.
    pub(crate) fn try_compute_push_normal(
        &self,
        p1: V4,
        p2: V4,
        normal: V4,
        num: &mut GrindNum<'_>,
    ) -> Result<V4, GrindError> {
        let normal = world_horizontal_normal(p1, p2, normal, num)?;
        let mut toward = sub(
            scale(add(p1, p2), 0.5),
            need(self.reach_base, "Pushing reach base")?,
        );
        toward[1] = 0.0;
        Ok(if num.dot(normal, toward)? > 0.0 {
            neg(normal)
        } else {
            normal
        })
    }

    /// ComputeTargetHandIK82d92f30. Preserve native endpoint clamps, degenerate
    /// fallbacks and the bit80 latch; do not independently snap either hand.
    pub(crate) fn try_compute_target_hand_ik(
        &mut self,
        raw_pose: &mut impl FnMut(usize) -> Result<V4, GrindError>,
        effective_x: V4,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        self.hand_l_raw = Some(raw_pose(3)?);
        self.hand_r_raw = Some(raw_pose(7)?);
        let half_span =
            0.5 * num.length(sub(self.hand_l_raw.unwrap(), self.hand_r_raw.unwrap()))?;
        let length = need(self.spline_length, "Pushing spline length")?;
        let grip = need(self.grip_offset_along_spline, "Pushing grip offset")?;
        let plus = vmin(length, vmax(0.0, half_span + grip));
        let minus = vmin(length, vmax(0.0, grip - half_span));
        let spline = need(self.grab_spline.as_ref(), "Pushing grab spline")?;
        let mut p1 = spline.position_at_spline_length(plus, num)?;
        let mut p2 = spline.position_at_spline_length(minus, num)?;
        let reach = need(self.reach_base, "Pushing reach base")?;
        let mut difference = sub(p1, p2);
        const DEGENERATE_SQUARED: f32 = f32::from_bits(0x38d1_b717);
        if DEGENERATE_SQUARED > num.dot(difference, difference)? {
            let from_reach = sub(p1, reach);
            difference = if DEGENERATE_SQUARED > num.dot(from_reach, from_reach)? {
                effective_x
            } else {
                cross(J, from_reach)
            };
        }
        let mut x = try_normalize_safe(difference, ZERO, num.n).map_err(error)?;
        if 0.0 > cross(sub(p2, reach), sub(p1, reach))[1] {
            if need(self.hand_ik_targets_locked, "Pushing hand latch")? {
                std::mem::swap(&mut p1, &mut p2);
                x = neg(x);
            }
        } else {
            self.hand_ik_targets_locked = Some(true);
        }
        let z = neg(self.try_compute_push_normal(p1, p2, spline.normal, num)?);
        let origin = spline.position_at_spline_length(grip, num)?;
        let matrix = [x, cross(z, x), z, origin];
        self.current_spline_hands_transform = Some(matrix);
        self.current_hand_left_offset = Some(inverse_transform_point(&matrix, p1));
        self.current_hand_right_offset = Some(inverse_transform_point(&matrix, p2));
        p1[1] = 0.0;
        p2[1] = 0.0;
        self.spline_dir = Some(try_normalize_safe(sub(p1, p2), ZERO, num.n).map_err(error)?);
        Ok(())
    }

    /// StartTargetHandIKBlend82d93428. The original uses this literal near-60
    /// factor, not current render dt, and calls UpdateTargetHandIKBlend at end.
    pub(crate) fn try_start_target_hand_ik_blend(
        &mut self,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        self.previous_spline_hands_transform = Some(need(
            self.blended_spline_hands_transform,
            "Pushing blended hands",
        )?);
        self.previous_hand_left_offset =
            Some(need(self.blended_hand_left_offset, "Pushing blended left")?);
        self.previous_hand_right_offset = Some(need(
            self.blended_hand_right_offset,
            "Pushing blended right",
        )?);
        let current = need(self.current_spline_hands_transform, "Pushing current hands")?;
        let blended = self.blended_spline_hands_transform.unwrap();
        self.spline_blend_coeff = Some(1.0);
        let distance = num.length(sub(current[3], blended[3]))?;
        let attrs = need(
            self.attribs_off_board.as_ref(),
            "Pushing offboard attributes",
        )?;
        // Both native lookups are retained, including the unused first read.
        let _ = read_floats::<1>(attrs, 0x1b8f_17a8_da7b_75aa).map_err(error)?;
        let speed = read_floats::<1>(attrs, 0x1b8f_17a8_da7b_75aa).map_err(error)?[0];
        let frames = num.reciprocal(speed * f32::from_bits(0x426f_ffff))? * distance;
        if frames >= 1.0 {
            self.spline_blend_coeff = Some(1.0);
            self.is_blending_spline = Some(true);
            self.spline_blend_adjust = Some(-1.0 / frames);
        } else {
            self.spline_blend_coeff = Some(-1.0);
            self.spline_blend_adjust = Some(-1.0);
            // Native else does not clear the retained blend flag.
        }
        self.try_update_target_hand_ik_blend(num)
    }

    /// UpdateTargetHandIKBlend82d87a40. Keep the signed adjust*.001 test as
    /// authored, rather than changing it to an absolute-value completion test.
    pub(crate) fn try_update_target_hand_ik_blend(
        &mut self,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        if need(self.is_blending_spline, "Pushing spline blend flag")? {
            let coeff = need(self.spline_blend_coeff, "Pushing spline blend coefficient")?;
            if coeff < 0.0 {
                return Err(GrindError(
                    "Pushing negative spline blend coefficient (native assertion)".into(),
                ));
            }
            let adjust = need(self.spline_blend_adjust, "Pushing spline blend adjust")?;
            let sum = adjust + coeff;
            let low = fsel(neg1(sum), 0.0, sum);
            let updated = fsel(1.0 - low, low, 1.0);
            self.spline_blend_coeff = Some(updated);
            if updated < adjust * f32::from_bits(0x3a83_126f) {
                self.spline_blend_coeff = Some(-1.0);
                self.is_blending_spline = Some(false);
            }
        }
        if self.is_blending_spline.unwrap() {
            let coeff = self.spline_blend_coeff.unwrap();
            let from = need(
                self.previous_spline_hands_transform,
                "Pushing previous hands",
            )?;
            let to = need(self.current_spline_hands_transform, "Pushing current hands")?;
            let flat = |m: [V4; 4]| -> [f32; 16] { std::array::from_fn(|i| m[i / 4][i % 4]) };
            let blended = try_interpolate_affine(&flat(from), &flat(to), 1.0 - coeff, num.n, num.s)
                .map_err(error)?;
            self.blended_spline_hands_transform = Some(std::array::from_fn(|i| {
                std::array::from_fn(|j| blended[i * 4 + j])
            }));
            self.blended_hand_left_offset = Some(madd(
                need(self.previous_hand_left_offset, "Pushing previous left")?,
                coeff,
                scale(
                    need(self.current_hand_left_offset, "Pushing current left")?,
                    1.0 - coeff,
                ),
            ));
            self.blended_hand_right_offset = Some(madd(
                need(self.previous_hand_right_offset, "Pushing previous right")?,
                coeff,
                scale(
                    need(self.current_hand_right_offset, "Pushing current right")?,
                    1.0 - coeff,
                ),
            ));
        } else {
            self.blended_spline_hands_transform = Some(need(
                self.current_spline_hands_transform,
                "Pushing current hands",
            )?);
            self.blended_hand_left_offset =
                Some(need(self.current_hand_left_offset, "Pushing current left")?);
            self.blended_hand_right_offset = Some(need(
                self.current_hand_right_offset,
                "Pushing current right",
            )?);
        }
        Ok(())
    }
}
