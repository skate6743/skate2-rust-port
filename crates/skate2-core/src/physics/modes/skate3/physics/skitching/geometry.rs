//! UpdateGrabSpline82d94760: retained current/future frames and chord geometry.
use super::{
    Matrix, MatrixSlot, Skitching, Vector,
    skitch_state::{DT, SkitchError, need},
};
use crate::{
    physical_player_grab_spline::geometry::transform_point,
    physical_player_grind_manager::{
        math::{GrindNum, ZERO, add, cross, madd, scale, sub},
        view::error,
    },
    physical_player_handplant::geometry::{inverse, transform_vector},
    reckoning_numeric::try_normalize_safe,
};

pub(super) fn compose(outer: Matrix, inner: Matrix) -> Matrix {
    [
        transform_vector(outer, inner[0]),
        transform_vector(outer, inner[1]),
        transform_vector(outer, inner[2]),
        transform_point(&outer, inner[3]),
    ]
}
pub(super) fn on_plane(
    v: Vector,
    normal: Vector,
    n: &mut GrindNum<'_>,
) -> Result<Vector, SkitchError> {
    Ok(sub(v, scale(normal, n.dot(v, normal)?)))
}

impl Skitching {
    pub(super) fn space(&self, slot: Option<MatrixSlot>) -> Result<Matrix, SkitchError> {
        match need(slot, "spline frame slot")? {
            MatrixSlot::Current => need(self.grab_spline_current_space, "current spline frame"),
            MatrixSlot::Future => need(self.grab_spline_future_space, "future spline frame"),
        }
    }
    pub(crate) fn update_grab_geometry(
        &mut self,
        deck: Matrix,
        authored: bool,
        n: &mut GrindNum<'_>,
    ) -> Result<(), SkitchError> {
        let normal = need(self.ground_normal, "ground normal")?;
        let first = need(self.first_eval, "first evaluation")?;
        let spline = self
            .grab_spline
            .as_mut()
            .ok_or(SkitchError::Missing("grab spline"))?;
        let mut axis = need(spline.length_unit, "spline length unit")?;
        if 0.0 > n.dot(deck[0], axis)? {
            std::mem::swap(&mut spline.p1, &mut spline.p2);
            axis = crate::physical_player_grind_manager::math::neg(axis);
            spline.length_unit = Some(axis);
            spline.status ^= 0x20;
        }
        let half = spline.half_length;
        let centre = scale(add(spline.p1, spline.p2), 0.5);
        self.grab_spline_half_length_mag = Some(half);
        self.grab_spline_half_length_mag_with_edge_gap =
            Some(self.attr(0x6027_2903_ad60_ff3b)? + half);
        let z = try_normalize_safe(cross(axis, normal), ZERO, n.n).map_err(error)?;
        let newest = [axis, normal, z, centre];
        let (last, current, future, step) = if authored {
            let last = if first {
                newest
            } else {
                need(self.grab_spline_current_space, "current frame")?
            };
            let current = if first {
                newest
            } else {
                need(self.grab_spline_future_space, "future frame")?
            };
            (last, current, newest, compose(newest, inverse(current)))
        } else {
            let last = if first {
                newest
            } else {
                need(self.grab_spline_current_space, "current frame")?
            };
            let step = compose(newest, inverse(last));
            (last, newest, compose(step, newest), step)
        };
        self.grab_spline_last_space = Some(last);
        self.grab_spline_current_space = Some(current);
        self.grab_spline_future_space = Some(future);
        self.grab_spline_forward_step_transform = Some(step);
        let slot = if authored {
            MatrixSlot::Future
        } else {
            MatrixSlot::Current
        };
        self.grab_spline_space = Some(slot);
        self.grab_spline_ik_hand_space = Some(slot);
        self.grab_spline_physical_hand_space = Some(MatrixSlot::Current);
        let spline_space = self.space(self.grab_spline_space)?;
        let ik_space = self.space(self.grab_spline_ik_hand_space)?;
        let to_physical = compose(current, inverse(spline_space));
        self.grab_spline_to_physical_hand_space_transform = Some(to_physical);
        self.grab_spline_to_ik_hand_space_transform =
            Some(compose(ik_space, inverse(spline_space)));
        self.grab_spline_anim_space_to_ik_hand_space_transform = Some(step);
        self.deck_pos = Some(deck[3]);
        let offset = n.dot(current[0], sub(deck[3], current[3]))?;
        let offset_speed = (offset - need(self.grab_spline_deck_offset, "deck offset")?)
            * f32::from_bits(0x426f_ffff);
        self.grab_spline_deck_offset = Some(offset);
        self.grab_spline_deck_offset_speed = Some(offset_speed);
        let base = madd(current[0], offset, current[3]);
        let hand_vector = sub(base, deck[3]);
        self.vector_to_deck_pos_on_hand_space = Some(hand_vector);
        let curve = if offset.abs() >= half {
            base
        } else {
            let p = self
                .grab_spline
                .as_ref()
                .ok_or(SkitchError::Missing("grab spline"))?
                .position_at_straight_length(offset, n)?;
            transform_point(&to_physical, p)
        };
        let curve_from_base = sub(curve, base);
        self.vector_to_deck_pos_on_curve_from_base = Some(curve_from_base);
        self.grab_spline_false_deck_offset_speed_due_to_turning = Some(
            offset_speed
                - need(
                    self.grab_spline_desired_follow_point_velocity,
                    "desired follow velocity",
                )?,
        );
        //82d950a0..b4 uses last-current, not current-last. Preserve its sign.
        let velocity = scale(sub(madd(last[0], offset, last[3]), base), n.reciprocal(DT)?);
        self.grab_spline_velocity = Some(velocity);
        let hand_ground = on_plane(hand_vector, normal, n)?;
        let hand_direction = n.normalize_unsafe(hand_ground)?;
        self.normalized_vector_to_deck_pos_on_hand_space_ground = Some(hand_direction);
        let curve_ground = on_plane(curve_from_base, normal, n)?;
        let mut curve_distance = n.length(curve_ground)?;
        if 0.0 > n.dot(curve_ground, current[2])? {
            curve_distance = -curve_distance;
        }
        self.distance_to_deck_pos_on_curve_from_base_ground = Some(curve_distance);
        self.normalized_vector_to_deck_pos_on_curve_from_base_ground =
            Some(scale(curve_ground, n.reciprocal(curve_distance)?));
        let deck_distance = n.length(hand_ground)?;
        self.deck_dist_speed = Some(if first {
            0.0
        } else {
            (deck_distance - need(self.deck_dist, "deck distance")?) * f32::from_bits(0x426f_ffff)
        });
        self.deck_dist = Some(deck_distance);
        let ground_velocity = scale(hand_direction, n.dot(velocity, hand_direction)?);
        self.grab_spline_velocity_ground = Some(ground_velocity);
        self.grab_spline_speed_ground = Some(n.length(ground_velocity)?);
        let mut aim = current;
        //82d95330 loads this+0x230: AimSpace.zAxis, NOT the curve delta
        //at +0x2a0. The native aim origin is half a metre along the spline's
        //forward axis, including on a straight grab spline.
        aim[3] = madd(aim[2], 0.5, aim[3]);
        self.grab_spline_aim_space = Some(aim);
        let aim_vector = sub(madd(aim[0], offset, aim[3]), deck[3]);
        self.vector_to_deck_pos_on_aim_space = Some(aim_vector);
        let aim_ground = on_plane(aim_vector, normal, n)?;
        self.normalized_vector_to_deck_pos_on_aim_space_ground =
            Some(n.normalize_unsafe(aim_ground)?);
        self.deck_dist_on_aim_space = Some(n.length(aim_ground)?);
        self.skitch_steering_active = Some(
            need(self.grab_spline_speed_ground, "spline ground speed")?
                >= self.attr(0x1f85_f500_908c_5e17)?,
        );
        Ok(())
    }
    /// CalcFakeCentripedalAcceleration82d891c0: raw PPC fixes the decompiler's
    /// apparent return of deck offset. This is a difference of transformed points.
    pub(super) fn fake_centripetal(&self, n: &mut GrindNum<'_>) -> Result<f32, SkitchError> {
        let current = need(self.grab_spline_current_space, "current frame")?;
        let last = need(self.grab_spline_last_space, "last frame")?;
        let offset = need(self.grab_spline_deck_offset, "deck offset")?;
        let difference = sub(
            madd(current[0], offset, current[3]),
            madd(last[0], offset, last[3]),
        );
        Ok(-n.dot(difference, current[0])? * f32::from_bits(0x45e0_fffe))
    }
}
