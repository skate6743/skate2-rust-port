//! Skitch speed/release force and heading helpers82d966a8..97088/82d9e8d8.
use super::{
    Matrix, Skitching, Vector,
    skitch_geometry::on_plane,
    skitch_state::{DT, EDGE, SkitchError, SkitchExitForce, need},
};
use crate::{
    physical_player_grind_manager::math::{DEG_TO_RAD, GrindNum, ZERO, fsel, madd, scale, sub},
    physical_player_trajectory_owners::selector_grind::{vmax, vmin},
    reckoning_numeric::wrap_turns_to_radians,
};

impl Skitching {
    pub(crate) fn exit_force(
        &self,
        kind: SkitchExitForce,
        mass: f32,
        n: &mut GrindNum<'_>,
    ) -> Result<Vector, SkitchError> {
        let speed = match kind {
            SkitchExitForce::None | SkitchExitForce::Both => return Ok(ZERO),
            SkitchExitForce::Back => {
                let lower = -need(self.grab_spline_speed_ground, "spline speed")?;
                let value = need(self.deck_dist_speed, "deck distance speed")?
                    + self.attr(0x58b6_97ca_9ff5_70e4)?;
                let bounded = fsel(lower - value, lower, value);
                fsel(-bounded, bounded, 0.0)
            }
            SkitchExitForce::Forward => {
                need(self.deck_dist_speed, "deck distance speed")?
                    + self.attr(0x8524_e22b_25d7_d0d7)?
            }
        };
        let direction = need(
            self.normalized_vector_to_deck_pos_on_aim_space_ground,
            "aim direction",
        )?;
        Ok(scale(
            scale(scale(direction, speed), mass),
            n.reciprocal(DT)?,
        ))
    }
    /// Skate 3 CalcSpeedForce 0x82d4b500.
    pub(crate) fn speed_force(
        &mut self,
        bone_board_z: f32,
        mirrored: bool,
        world_grab_z: f32,
        mass: f32,
        z_speed: f32,
        n: &mut GrindNum<'_>,
    ) -> Result<Vector, SkitchError> {
        // CalcSpeedForce precedes CalcSkitchHeadingDisplacement: the target
        // direction intentionally comes from the preceding frame.
        let target = need(
            self.normalized_vector_to_skitch_target_ground,
            "skitch target direction",
        )?;
        let aim = need(
            self.normalized_vector_to_deck_pos_on_aim_space_ground,
            "aim direction",
        )?;
        let alignment = vmin(1.0, vmax(0.5, n.dot(aim, target)?));
        // 0x82d4b5a4..b604: the deck distance is first advanced one step by
        // (2 * spline ground speed - Processed+0xA34 Z speed) * DT.
        let spline_speed = need(self.grab_spline_speed_ground, "spline speed")?;
        let distance = ((spline_speed - z_speed) + spline_speed)
            .mul_add(DT, need(self.deck_dist, "deck distance")?)
            + need(
                self.distance_to_deck_pos_on_curve_from_base_ground,
                "curve distance",
            )?;
        let bone_z = if mirrored {
            bone_board_z * -1.0
        } else {
            bone_board_z
        };
        let mut offset = self.attr(0xb7a8_1705_5629_ec4b)?;
        if need(self.skitch_state, "skitch state")? == EDGE {
            offset = self.attr(0x95d9_de65_a73b_b685)?;
        }
        let spring = self.graph(
            0x00a1_b0b3_3f1b_a1bf,
            distance - ((world_grab_z - bone_z) + offset),
        )?;
        let contact_speed = -need(self.deck_dist_speed, "deck distance speed")?;
        self.contact_speed = Some(contact_speed);
        let damping = -(contact_speed * self.attr(0x71a8_94f6_ad65_c859)?);
        let inverse_alignment_a = n.reciprocal(alignment)?;
        let inverse_alignment_b = n.reciprocal(alignment)?;
        let damped = scale(scale(target, damping), inverse_alignment_a);
        let combined = madd(scale(target, spring), inverse_alignment_b, damped);
        let force = scale(scale(combined, mass), n.reciprocal(DT)?);
        // 0x82d4b870: ClampVectorWithinMaxLength(force, 500.0).
        Ok(crate::reckoning_numeric::try_clamp(force, 500.0, n.n)
            .map_err(crate::physical_player_grind_manager::view::error)?)
    }
    pub(crate) fn heading_displacement(
        &mut self,
        deck: Matrix,
        use_torque: bool,
        n: &mut GrindNum<'_>,
    ) -> Result<Vector, SkitchError> {
        let aim = need(self.grab_spline_aim_space, "aim frame")?;
        let follow = need(self.grab_spline_follow_point_offset, "follow offset")?;
        let target = madd(aim[0], follow, aim[3]);
        self.grab_spline_follow_point_target = Some(target);
        let normal = need(self.ground_normal, "ground normal")?;
        let target_ground = on_plane(sub(target, deck[3]), normal, n)?;
        let target_direction = n.normalize_unsafe(target_ground)?;
        let forward = madd(
            aim[0],
            need(self.grab_spline_deck_offset, "deck offset")?,
            aim[3],
        );
        let forward_ground = on_plane(sub(forward, deck[3]), normal, n)?;
        let forward_direction = n.normalize_unsafe(forward_ground)?;
        self.desired_heading_angle =
            Some(self.angle_on_ground(forward_direction, target_direction, n)?);
        self.normalized_vector_to_skitch_target_ground = Some(target_direction);
        let relative = self.angle_on_ground(deck[2], target_direction, n)?;
        let to_forward = self.angle_on_ground(
            deck[2],
            need(
                self.normalized_vector_to_deck_pos_on_aim_space_ground,
                "aim direction",
            )?,
            n,
        )?;
        let forward_to_target =
            wrap_turns_to_radians((relative - to_forward) * f32::from_bits(0x3e22_f983));
        let torque = if use_torque {
            self.skitch_torque(relative, forward_to_target, n)?
        } else {
            ZERO
        };
        self.deck_angle = Some(if need(self.skitch_steering_active, "skitch steering")? {
            forward_to_target
        } else {
            -to_forward
        });
        Ok(torque)
    }
    fn skitch_torque(
        &self,
        relative: f32,
        forward_to_target: f32,
        n: &mut GrindNum<'_>,
    ) -> Result<Vector, SkitchError> {
        let limit = (self.attr(0x0b19_9e27_af7b_96a8)? * DEG_TO_RAD) * DT;
        let angle = vmin(limit, vmax(-limit, relative));
        let sign = if angle >= 0.0 { 1.0 } else { -1.0 };
        let mut normal = need(self.ground_normal, "ground normal")?;
        if 0.0 > normal[1] {
            normal = scale(normal, -1.0);
        }
        let sin = crate::reckoning_sin_cos::sine::try_sine((angle * sign) * sign, n.s)
            .map_err(crate::physical_player_grind_manager::view::error)?;
        let torque = scale(normal, sin);
        let time_strength = self.graph(
            0x3f79_d445_a24e_12ba,
            need(self.time_grabbing, "grab time")?,
        )?;
        let speed_strength = self.graph(
            0x6fb7_a3d9_9216_3663,
            need(self.grab_spline_speed_ground, "spline speed")?,
        )?;
        let degrees = (forward_to_target * 180.0) * f32::from_bits(0x3ea2_f983);
        let extra =
            if need(self.follow_point_mode, "follow point mode")? == 1 && degrees.abs() < 1.0 {
                self.graph(0x1ff7_8fae_8ad4_67fc, degrees)?
            } else {
                1.0
            };
        let speed_strength = extra * speed_strength;
        Ok(scale(
            torque,
            fsel(
                time_strength - speed_strength,
                time_strength,
                speed_strength,
            ),
        ))
    }
}
