//! UpdateHands82d960e0, preserving the source's scalar FMA boundaries.
use super::{
    Skitching,
    skitch_state::{DT, SkitchControlCvars, SkitchError, clamp, need},
};
impl Skitching {
    pub(super) fn update_skitch_hands(
        &mut self,
        c: &SkitchControlCvars,
    ) -> Result<(), SkitchError> {
        let offset = need(self.grab_spline_deck_offset, "deck offset")?;
        let hands = need(self.grab_spline_hands_dist, "hands distance")?;
        let distance = (offset - hands).abs();
        if need(self.hands_moving, "hands moving")? {
            let mut turn = need(self.turn_scalar, "turn scalar")?;
            if turn.abs() <= 0.01 {
                turn = 0.0;
            }
            let old = need(self.grab_spline_hand_target_offset, "hand target")?;
            let half = need(
                self.grab_spline_half_length_mag_with_edge_gap,
                "edge half length",
            )?;
            let mut target = clamp(offset, -half, half);
            if (turn > c.min_turn_for_exit && target < old)
                || (turn < -c.min_turn_for_exit && target > old)
            {
                target =
                    (1.0 - c.stick_hand_slow_down).mul_add(target, c.stick_hand_slow_down * old);
            }
            self.grab_spline_hand_target_offset = Some(target);
            if need(self.grab_spline_deck_offset_speed, "deck offset speed")?.abs()
                < self.attr(0x1f5f_8c47_ef1c_7c97)?
                && distance.abs() < self.attr(0x4cc8_c0d7_41c1_21d4)?
            {
                self.hands_moving = Some(false);
            }
            self.time_hands_not_moving = Some(0.0);
        } else {
            let time = need(self.time_hands_not_moving, "hands not moving time")? + DT;
            self.time_hands_not_moving = Some(time);
            if distance > self.attr(0x8fef_0ad3_8542_cecd)?
                || time > self.attr(0xcfbb_ddd0_8082_cf69)?
            {
                self.hands_moving = Some(true);
            }
        }
        let old_delta = need(self.grab_spline_hands_dist_delta, "hands delta")?;
        let upper = self.attr(0xea88_5312_1305_05b0)? + old_delta;
        let lower = old_delta - self.attr(0xea88_5312_1305_05b0)?;
        let blend = self.attr(0xff6b_65af_360c_6707)?;
        let desired = (need(self.grab_spline_hand_target_offset, "hand target")? - hands) * blend;
        let delta = clamp(desired, lower, upper);
        self.grab_spline_hands_dist_delta = Some(delta);
        self.grab_spline_hands_dist = Some(delta + hands);
        Ok(())
    }
}
