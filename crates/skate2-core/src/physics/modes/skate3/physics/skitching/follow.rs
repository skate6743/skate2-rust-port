//! UpdateFollowPoint82d95468. Original fixed-step grip/centrifugal controller.
use super::{
    Skitching, Vector, read_floats,
    skitch_state::{
        DT, END, ENTERING, SkitchError, clamp, need, opposing_direction, same_direction,
    },
};
use crate::{
    physical_player_grind_manager::{
        math::{GrindNum, fsel},
        view::error,
    },
    physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle,
    reckoning_numeric::wrap_turns_to_radians,
    skeleton_ik_tangent::try_tangent,
};

const RATE: f32 = f32::from_bits(0x426f_ffff);
const BLEND: u64 = 0x1ea5_2a4b_df89_64f7;
const MAX_VELOCITY: u64 = 0x7fa1_fb66_30f7_5c67;
const TURN_ACCEL: u64 = 0xe496_5c6a_27f1_ff96;

impl Skitching {
    pub(super) fn graph(&self, key: u64, input: f32) -> Result<f32, SkitchError> {
        let values = read_floats::<20>(need(self.attribs.as_ref(), "skitch attributes")?, key)
            .map_err(SkitchError::Attribute)?;
        Ok(super::pushing_position::graph(values, input))
    }
    pub(super) fn angle_on_ground(
        &self,
        from: Vector,
        to: Vector,
        n: &mut GrindNum<'_>,
    ) -> Result<f32, SkitchError> {
        let normal = need(self.ground_normal, "ground normal")?;
        let angle = plane_angle(from.into(), to.into(), normal.into(), n.n).map_err(error)?;
        Ok(wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983)))
    }
    pub(crate) fn update_follow_point(
        &mut self,
        deck_forward: Vector,
        centrifugal_enabled: bool,
        fake_physics: bool,
        n: &mut GrindNum<'_>,
    ) -> Result<(), SkitchError> {
        let raw_turn = need(self.turn_scalar, "turn scalar")?;
        let turn = if raw_turn.abs() <= 0.01 {
            0.0
        } else {
            raw_turn
        };
        let turn_sign = if turn > 0.0 {
            1.0
        } else {
            fsel(turn, 0.0, -1.0)
        };
        let half = need(
            self.grab_spline_half_length_mag_with_edge_gap,
            "edge half length",
        )?;
        let deck_offset = need(self.grab_spline_deck_offset, "deck offset")?;
        let distance = need(self.deck_dist_on_aim_space, "aim distance")?;
        let steering = need(self.skitch_steering_active, "skitch steering")?;
        if need(self.first_eval, "first evaluation")? || !steering {
            let angle = self.angle_on_ground(
                deck_forward,
                need(
                    self.normalized_vector_to_deck_pos_on_aim_space_ground,
                    "aim ground direction",
                )?,
                n,
            )?;
            let tangent = try_tangent(-angle, n.n, n.s).map_err(error)?;
            self.grab_spline_follow_point_offset = Some(clamp(
                deck_offset + tangent * clamp(distance, 0.001, 1e9),
                -half,
                half,
            ));
        }
        let fake_accel = -self.fake_centripetal(n)?;
        let blend = self.attr(BLEND)?;
        let old_accel = need(
            self.grab_spline_centrifugal_acceleration,
            "centrifugal acceleration",
        )?;
        let mut centrifugal = (1.0 - blend).mul_add(old_accel, blend * fake_accel);
        self.grab_spline_centrifugal_acceleration = Some(centrifugal);
        self.centrifugal_force_state = Some(0);
        if centrifugal.abs() > self.attr(0x30da_d137_ee60_2142)? {
            self.centrifugal_force_state = Some(if centrifugal <= 0.0 { -1 } else { 1 });
        }
        if !centrifugal_enabled {
            centrifugal = 0.0;
            self.grab_spline_centrifugal_acceleration = Some(0.0);
        }
        let sign = if centrifugal < 0.0 {
            -1.0
        } else if centrifugal > 0.0 {
            1.0
        } else {
            0.0
        };
        let magnitude = centrifugal * sign;
        let graph = self.graph(0x466f_c01c_f6ca_2fd2, magnitude)?;
        let difference = magnitude - graph;
        let grip = -(fsel(-difference, 0.0, difference) * sign);
        self.grab_spline_grip_acceleration = Some(grip);
        let force_state = need(self.centrifugal_force_state, "centrifugal state")?;
        let driving = raw_turn.abs() > 0.01 || force_state != 0;
        let recenter = !driving && need(self.grab_spline_target_offset_active, "target active")?;
        if !recenter {
            self.grab_spline_target_offset_locked = Some(false);
            self.grab_spline_target_offset_active = Some(false);
        }
        if !fake_physics {
            self.grab_spline_turn_acceleration = Some(0.0);
            self.clear_follow_acceleration();
            self.grab_spline_centrifugal_acceleration =
                Some((1.0 - self.attr(BLEND)?) * centrifugal);
            let velocity = self.graph(MAX_VELOCITY, 0.0)?;
            self.grab_spline_follow_point_offset =
                Some((clamp(distance, 0.001, 1e9) * velocity).mul_add(turn, deck_offset));
            return Ok(());
        }
        let state = need(self.skitch_state, "skitch state")?;
        if state == ENTERING || state == END || !steering {
            self.grab_spline_turn_acceleration = Some(0.0);
            self.grab_spline_centrifugal_acceleration =
                Some((1.0 - self.attr(BLEND)?) * centrifugal);
            self.clear_follow_acceleration();
            return Ok(());
        }
        if need(self.grab_spline_target_offset_active, "target active")?
            && need(self.grab_spline_target_offset_locked, "target locked")?
        {
            self.grab_spline_turn_acceleration = Some(0.0);
            let blend = self.attr(BLEND)?;
            self.grab_spline_follow_point_offset = self.grab_spline_target_offset;
            self.grab_spline_centrifugal_acceleration =
                Some((1.0 - blend).mul_add(centrifugal, blend * fake_accel));
            self.clear_follow_acceleration();
            return Ok(());
        }
        let intended = if raw_turn.abs() > 0.01 {
            let input = if force_state != 0 && opposing_direction(turn, centrifugal) {
                magnitude
            } else {
                0.0
            };
            (self.graph(MAX_VELOCITY, input)? * turn).abs()
        } else {
            0.0
        };
        let mut turn_accel = turn_sign * self.attr(TURN_ACCEL)?;
        self.grab_spline_turn_acceleration = Some(turn_accel);
        let combined = centrifugal + grip;
        self.grab_spline_auto_recentering_acceleration = Some(0.0);
        let old_velocity = need(
            self.grab_spline_desired_follow_point_velocity,
            "desired follow velocity",
        )?;
        let mut auto = 0.0;
        if !driving {
            let desired = if need(self.grab_spline_target_offset_active, "target active")? {
                let raw =
                    (need(self.grab_spline_target_offset, "target offset")? - deck_offset) * RATE;
                clamp(raw, -self.attr(TURN_ACCEL)?, self.attr(TURN_ACCEL)?)
            } else {
                0.0
            };
            auto = clamp(
                (desired - combined.mul_add(DT, old_velocity)) * RATE,
                -self.attr(TURN_ACCEL)?,
                self.attr(TURN_ACCEL)?,
            );
            self.grab_spline_auto_recentering_acceleration = Some(auto);
        }
        if same_direction(turn_accel, auto) {
            let difference = turn_accel - auto;
            turn_accel = if turn_accel.abs() <= 0.0 {
                fsel(-difference, difference, 0.0)
            } else {
                fsel(-difference, 0.0, difference)
            };
            self.grab_spline_turn_acceleration = Some(turn_accel);
        }
        let accel = ((centrifugal + auto) + grip) + turn_accel;
        self.grab_spline_follow_point_acceleration = Some(accel);
        let max = if recenter {
            self.graph(MAX_VELOCITY, 0.0)?
        } else {
            intended
        };
        let upper = if force_state == 1 { f32::MAX } else { max };
        let lower = if force_state == -1 { -f32::MAX } else { -max };
        let desired = clamp(accel.mul_add(DT, old_velocity), lower, upper);
        let speed = need(self.grab_spline_speed_ground, "spline speed")?;
        let inverse_speed = 1.0 / clamp(speed, 0.01, 1e9);
        let arm_length = clamp(distance, 0.01, 1e9);
        let inverse_arm = 1.0 / arm_length;
        let positive_speed = clamp(speed, 0.0, 1e9);
        let desired = ((inverse_speed * desired) * arm_length) * inverse_arm * positive_speed;
        self.grab_spline_desired_follow_point_velocity = Some(desired);
        if driving || recenter {
            let correction = need(
                self.grab_spline_false_deck_offset_speed_due_to_turning,
                "false offset speed",
            )?;
            let velocity = -(correction.mul_add(self.attr(0x6c62_538d_9951_2f75)?, -desired));
            let old = need(self.grab_spline_follow_point_offset, "follow offset")?;
            let next = if force_state != 0 {
                self.follow_point_mode = Some(1);
                velocity.mul_add(DT, old)
            } else {
                self.follow_point_mode = Some(0);
                (inverse_speed * velocity).mul_add(arm_length, deck_offset)
            };
            let bounded = clamp(next, -half, half);
            let velocity = ((bounded - deck_offset) * inverse_arm) * positive_speed;
            self.grab_spline_follow_point_velocity = Some(velocity);
            let next = (velocity * inverse_speed).mul_add(arm_length, deck_offset);
            let target = need(self.grab_spline_target_offset, "target offset")?;
            if recenter && same_direction(target - old, next - target) {
                self.grab_spline_follow_point_offset = Some(target);
                self.grab_spline_target_offset_locked = Some(true);
            } else {
                self.grab_spline_follow_point_offset = Some(next);
            }
        } else {
            self.follow_point_mode = Some(1);
        }
        Ok(())
    }
    fn clear_follow_acceleration(&mut self) {
        self.grab_spline_follow_point_velocity = Some(0.0);
        self.grab_spline_desired_follow_point_velocity = Some(0.0);
        self.grab_spline_follow_point_acceleration = Some(0.0);
        self.grab_spline_auto_recentering_acceleration = Some(0.0);
        self.grab_spline_grip_acceleration = Some(0.0);
    }
}
