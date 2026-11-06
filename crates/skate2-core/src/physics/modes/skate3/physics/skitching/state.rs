//! Skitch scalar state transitions:82d88c58/82d88dc8/82d95ce0/82d9da00/82d9e6d0.
use super::{Skitching, StateConstructorError, read_floats};
use crate::physical_player_grind_manager::math::fsel;

pub(super) const DT: f32 = f32::from_bits(0x3c88_8889);
pub(super) const ENTERING: i32 = 0;
pub(super) const MIDDLE: i32 = 1;
pub(super) const EDGE: i32 = 2;
pub(super) const EXITING: i32 = 3;
pub(super) const END: i32 = 4;
pub(super) const HAND_ON: i32 = 0;
pub(super) const HAND_OFF: i32 = 1;

#[derive(Debug)]
pub(crate) enum SkitchError {
    Missing(&'static str),
    Attribute(StateConstructorError),
    InvalidState(i32),
    Geometry(crate::physical_player_grind_manager::view::GrindError),
}
impl From<crate::physical_player_grind_manager::view::GrindError> for SkitchError {
    fn from(error: crate::physical_player_grind_manager::view::GrindError) -> Self {
        Self::Geometry(error)
    }
}
pub(super) fn need<T>(value: Option<T>, field: &'static str) -> Result<T, SkitchError> {
    value.ok_or(SkitchError::Missing(field))
}
pub(super) fn clamp(value: f32, low: f32, high: f32) -> f32 {
    let lower = fsel(low - value, low, value);
    fsel(high - lower, lower, high)
}
pub(super) fn same_direction(a: f32, b: f32) -> bool {
    a != 0.0 && b != 0.0 && (a > 0.0) == (b > 0.0)
}
pub(super) fn opposing_direction(a: f32, b: f32) -> bool {
    a != 0.0 && b != 0.0 && (a > 0.0) != (b > 0.0)
}
#[derive(Clone, Copy)]
pub(crate) struct SkitchControlCvars {
    pub(crate) min_turn_for_exit: f32,
    pub(crate) stick_hand_slow_down: f32,
    pub(crate) force_stick: bool,
    pub(crate) frames_to_exit: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkitchExitForce {
    None,
    Back,
    Forward,
    /// Skate 3 0x82d4b0c8..b124 adds the push-back force, then the push-forward force.
    Both,
}

impl Skitching {
    pub(super) fn attr(&self, key: u64) -> Result<f32, SkitchError> {
        Ok(
            read_floats::<1>(need(self.attribs.as_ref(), "skitch attributes")?, key)
                .map_err(SkitchError::Attribute)?[0],
        )
    }
    ///82d88c58: both height thresholds and the wheel-contact timer are read.
    pub(crate) fn update_local_physical(
        &mut self,
        wheel_height: f32,
        time_since_wheels: f32,
    ) -> Result<(), SkitchError> {
        let high = wheel_height > self.attr(0x40aa_d3fd_99b4_64f4)?;
        let low = wheel_height > self.attr(0xf178_f963_558d_24a0)?;
        let time = self.attr(0x03a5_9cdf_a0b0_b967)?;
        self.in_air = Some(high || (low && time_since_wheels > time));
        Ok(())
    }
    ///82d88dc8: opposing spin does not cancel turn; otherwise use the greater
    /// signed magnitude in the turn's direction (including the zero-turn case).
    pub(crate) fn update_local_control(&mut self, turn: f32, spin: f32) {
        self.turn_scalar = Some(combined_turn(turn, spin));
    }
    pub(super) fn turning_towards_edge(&self, tolerance: f32) -> Result<bool, SkitchError> {
        let turn = need(self.turn_scalar, "turn scalar")?;
        let offset = need(self.grab_spline_deck_offset, "deck offset")?;
        Ok(turn.abs() > tolerance && (turn > 0.0) == (offset > 0.0))
    }
    pub(super) fn turning_away_from_edge(&self, tolerance: f32) -> Result<bool, SkitchError> {
        let turn = need(self.turn_scalar, "turn scalar")?;
        let offset = need(self.grab_spline_deck_offset, "deck offset")?;
        Ok(turn.abs() > tolerance && (turn > 0.0) == (offset < 0.0))
    }
    fn change_skitch_state(&mut self, state: i32) {
        self.skitch_state = Some(state);
        self.hands_moving = Some(true);
        self.time_in_skitch_state = Some(0.0);
    }
    fn clear_target(&mut self) {
        self.grab_spline_target_offset_locked = Some(false);
        self.grab_spline_target_offset_active = Some(false);
    }
    ///82d95ce0. Keep before UpdateFollowPoint and UpdateSkitchState.
    pub(crate) fn check_state_change(&mut self, c: &SkitchControlCvars) -> Result<(), SkitchError> {
        match need(self.skitch_state, "skitch state")? {
            ENTERING => {
                self.push_back_on_exit = Some(false);
                self.push_forward_on_exit = Some(false);
                if need(self.time_in_skitch_state, "state time")?
                    > self.attr(0xcc25_7f85_6577_59a1)?
                    || need(self.left_hand_state, "left hand")? == HAND_ON
                    || need(self.right_hand_state, "right hand")? == HAND_ON
                {
                    let offset = need(self.grab_spline_deck_offset, "deck offset")?;
                    let half = need(self.grab_spline_half_length_mag, "half length")?;
                    self.grab_spline_hands_dist = Some(offset);
                    self.grab_spline_follow_point_offset = Some(offset);
                    self.change_skitch_state(if offset.abs() <= half { MIDDLE } else { EDGE });
                }
            }
            MIDDLE => {
                self.push_back_on_exit = Some(true);
                self.push_forward_on_exit = Some(false);
                if need(self.grab_spline_deck_offset, "deck offset")?.abs()
                    > need(self.grab_spline_half_length_mag, "half length")?
                    && !self.turning_away_from_edge(0.01)?
                {
                    self.change_skitch_state(EDGE);
                }
            }
            EDGE => {
                self.push_back_on_exit = Some(false);
                self.push_forward_on_exit = Some(true);
                if need(self.time_in_skitch_state, "state time")?
                    <= self.attr(0x81a1_fc1e_9c2b_44d7)?
                {
                    return Ok(());
                }
                let max_hang = self.attr(0x08fe_1471_1b1c_2bfa)?;
                let offset = need(self.grab_spline_deck_offset, "deck offset")?;
                let half = need(self.grab_spline_half_length_mag, "half length")?;
                // IsDeckOffsetWithinEdgeRegion82d886f0 calls the limit helper
                // only outside the centre interval.
                let within_edge = if offset.abs() <= half {
                    false
                } else {
                    offset.abs() <= half + self.attr(0x6027_2903_ad60_ff3b)?
                };
                if within_edge
                    && self.turning_towards_edge(c.min_turn_for_exit)?
                    && !opposing_direction(
                        need(self.centrifugal_force_state, "centrifugal state")? as f32,
                        offset,
                    )
                {
                    self.change_skitch_state(EXITING);
                } else if offset.abs() <= half || self.turning_away_from_edge(0.01)? {
                    self.change_skitch_state(MIDDLE);
                } else if need(self.time_on_edge_hanging_on_for_dear_life, "edge time")? > max_hang
                {
                    self.push_back_on_exit = Some(false);
                    self.push_forward_on_exit = Some(false);
                    self.change_skitch_state(END);
                }
                if self.skitch_state != Some(EDGE) {
                    self.time_on_edge_hanging_on_for_dear_life = Some(0.0);
                    self.clear_target();
                }
            }
            EXITING => {
                self.push_back_on_exit = Some(false);
                self.push_forward_on_exit = Some(true);
                if need(self.time_in_skitch_state, "state time")?
                    < self.attr(0xd60a_7ae0_8b00_e714)?
                    && !self.turning_towards_edge(c.min_turn_for_exit)?
                {
                    self.change_skitch_state(EDGE);
                    self.delay_until_hands_off_matters = Some(self.attr(0x44ee_9c49_8ee7_c3c2)?);
                }
                if self.skitch_state != Some(EXITING) {
                    self.clear_target();
                }
                if need(self.time_in_skitch_state, "state time")?
                    > self.attr(0xba19_8476_1d05_7860)?
                {
                    self.delay_until_reskitchable = Some(self.attr(0xf5f9_d6cf_4d6a_c6d1)?);
                    self.skitch_state = Some(END);
                }
            }
            END => {}
            state => return Err(SkitchError::InvalidState(state)),
        }
        Ok(())
    }
    ///82d9da00, including the hand update and fixed-step clocks.
    pub(crate) fn update_skitch_state(
        &mut self,
        c: &SkitchControlCvars,
    ) -> Result<(), SkitchError> {
        match need(self.skitch_state, "skitch state")? {
            ENTERING => {
                let half = need(self.grab_spline_half_length_mag, "half length")?;
                self.grab_spline_hand_target_offset = Some(clamp(
                    need(self.grab_spline_follow_point_offset, "follow offset")?,
                    -half,
                    half,
                ));
            }
            MIDDLE => {
                self.left_hand_state = Some(HAND_ON);
                self.right_hand_state = Some(HAND_ON);
            }
            EDGE => {
                let offset = need(self.grab_spline_deck_offset, "deck offset")?;
                let half = need(
                    self.grab_spline_half_length_mag_with_edge_gap,
                    "edge half length",
                )?;
                self.update_target(if offset <= 0.0 { -half } else { half })?;
                let max = self.attr(0x007c_f9ea_e1f6_4c3c)?;
                let acceleration = need(
                    self.grab_spline_centrifugal_acceleration,
                    "centrifugal acceleration",
                )?;
                self.time_on_edge_hanging_on_for_dear_life = Some(
                    if same_direction(acceleration, offset) && acceleration.abs() > max {
                        need(self.time_on_edge_hanging_on_for_dear_life, "edge time")? + DT
                    } else {
                        0.0
                    },
                );
                self.left_hand_state = Some(if offset <= 0.0 { HAND_ON } else { HAND_OFF });
                self.right_hand_state = Some(if offset <= 0.0 { HAND_OFF } else { HAND_ON });
            }
            EXITING => {
                let target = need(self.grab_spline_half_length_mag, "half length")?
                    + self.attr(0xaed1_d7b2_0da7_cbba)?;
                self.update_target(
                    if need(self.grab_spline_deck_offset, "deck offset")? <= 0.0 {
                        -target
                    } else {
                        target
                    },
                )?;
                self.left_hand_state = Some(HAND_OFF);
                self.right_hand_state = Some(HAND_OFF);
            }
            END => {}
            state => return Err(SkitchError::InvalidState(state)),
        }
        self.update_skitch_hands(c)?;
        self.parametric_position = Some(clamp(
            need(self.grab_spline_deck_offset, "deck offset")?
                / need(self.grab_spline_half_length_mag, "half length")?,
            -1.0,
            1.0,
        ));
        let delay = need(self.delay_until_hands_off_matters, "hands off delay")?;
        if delay > 0.0 {
            self.delay_until_hands_off_matters = Some(delay - DT);
        }
        self.time_in_skitch_state = Some(need(self.time_in_skitch_state, "state time")? + DT);
        Ok(())
    }
    fn update_target(&mut self, target: f32) -> Result<(), SkitchError> {
        if !need(self.grab_spline_target_offset_active, "target active")? {
            self.grab_spline_target_offset = Some(target);
            self.grab_spline_target_offset_locked = Some(false);
            self.grab_spline_target_offset_active = Some(true);
        }
        if need(self.grab_spline_target_offset_locked, "target locked")? {
            self.grab_spline_follow_point_offset = Some(target);
        }
        Ok(())
    }
    ///82d9e6d0 through the force selection. Caller applies the returned force
    ///before finish_exit_check, preserving AddSkateboardForce before count++.
    pub(crate) fn check_exit(
        &mut self,
        grab_requested: bool,
        c: &SkitchControlCvars,
    ) -> Result<SkitchExitForce, SkitchError> {
        if !grab_requested && !c.force_stick {
            self.skitch_state = Some(END);
        }
        let state = need(self.skitch_state, "skitch state")?;
        if state != ENTERING
            && state != EXITING
            && state != END
            && need(self.left_hand_state, "left hand")? == HAND_OFF
            && need(self.right_hand_state, "right hand")? == HAND_OFF
            // 0x82d4b034 bgt: an unordered delay also passes.
            && !(need(self.delay_until_hands_off_matters, "hands off delay")? > 0.0)
        {
            self.skitch_state = Some(END);
            self.force_exit = Some(true);
            self.push_back_on_exit = Some(false);
            self.push_forward_on_exit = Some(false);
        }
        if self.skitch_state == Some(END) {
            // 0x82d4b064 bgt: an unordered delay also reloads.
            if !(need(self.delay_until_reskitchable, "reskitch delay")? > 0.0) {
                self.delay_until_reskitchable = Some(self.attr(0x6595_3772_8ce3_0b02)?);
            }
            if need(self.exit_count, "exit count")? == 1 {
                let back = need(self.push_back_on_exit, "push back")?;
                let forward = need(self.push_forward_on_exit, "push forward")?;
                return Ok(match (back, forward) {
                    (true, true) => SkitchExitForce::Both,
                    (true, false) => SkitchExitForce::Back,
                    (false, true) => SkitchExitForce::Forward,
                    (false, false) => SkitchExitForce::None,
                });
            }
        }
        Ok(SkitchExitForce::None)
    }
    pub(crate) fn finish_exit_check(&mut self, c: &SkitchControlCvars) -> Result<(), SkitchError> {
        if need(self.skitch_state, "skitch state")? == END {
            let count = need(self.exit_count, "exit count")?.wrapping_add(1);
            self.exit_count = Some(count);
            if count > c.frames_to_exit {
                self.force_exit = Some(true);
            }
        } else {
            self.exit_count = Some(0);
        }
        Ok(())
    }
}
fn combined_turn(turn: f32, spin: f32) -> f32 {
    if opposing_direction(turn, spin) {
        return turn;
    }
    let sign = if turn <= 0.0 { -1.0 } else { 1.0 };
    let a = sign * turn;
    let b = sign * spin;
    fsel(a - b, a, b) * sign
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opposing_sticks_preserve_turn_and_zero_turn_retains_native_sign_choice() {
        assert_eq!(combined_turn(0.25, -1.0), 0.25);
        assert_eq!(combined_turn(-0.25, 1.0), -0.25);
        assert_eq!(combined_turn(0.25, 0.8), 0.8);
        assert_eq!(combined_turn(-0.25, -0.8), -0.8);
        assert_eq!(combined_turn(0.0, 0.8).to_bits(), 0.0f32.to_bits());
        assert_eq!(combined_turn(0.0, -0.8), -0.8);
    }
}
