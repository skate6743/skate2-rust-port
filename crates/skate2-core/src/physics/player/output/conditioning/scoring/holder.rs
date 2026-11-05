//! ScoreHolder announcement, repeat history and banking; native82dee2e8..82df3e38.
use super::{
    super::scoring::{Scorable, Scoring2},
    score_holder::ScoreHolder,
    scoring_attributes::{ScoreError, curve},
    scoring_catalog::info,
    scoring_database::TrickDatabase,
    sequence::Sequence,
};

#[cfg(test)]
#[path = "holder_landing_tests.rs"]
mod landing_tests;

fn accumulate(value: &mut u8, amount: i32) {
    // Histogram82ded098 uses signed char and saturates to [-127,127].
    *value = (((*value as i8) as i32 + amount).clamp(-127, 127) as i8) as u8;
}
impl ScoreHolder {
    /// CheckForAirSequenceReward82dea800. Landing quality can arrive during
    /// either of the two deferred frames; reward is due on the following call.
    pub(super) fn check_air_reward(
        &mut self,
        animation: &super::super::simple::Animation,
        minimum_scalar: f32,
        force_sketchy: bool,
    ) -> Result<bool, ScoreError> {
        if !self.air_sequence_reward_pending {
            return Ok(false);
        }
        if animation.just_analyzed_sketchyness {
            #[cfg(feature = "skate3-runtime")]
            { self.skate3_clean_landing = animation.sketchyness_anim_result == 0; }
            let sketchy = match animation.sketchyness_anim_result {
                0 | 2 => false,
                1 => {
                    let speed =
                        super::scoring_attributes::scalar(&self.general, 0x6dc5_9825_91f8_0a4c)?;
                    // Preserve native ordered <= branches, including unordered inputs.
                    !(animation.sketchyness_side_speed <= speed
                        || animation.sketchyness_anim_scalar.abs() <= minimum_scalar)
                }
                // TU3 checks kind==1 for sketchy and kind==0 for clean;
                // its valid kind3 is neither. S2 keeps its own error policy.
                #[cfg(feature = "skate3-runtime")]
                _ if self.general.skate3_enabled() => false,
                _ => return Err(ScoreError::Invalid("sketchy animation result")),
            };
            self.pending_air_period.just_landed = true;
            self.pending_air_period.landed_succesfully = true;
            self.pending_air_period.landing_was_sketchy = sketchy || force_sketchy;
        }
        if self.air_sequence_reward_pending_frame_count == 0 {
            return Ok(true);
        }
        self.air_sequence_reward_pending_frame_count -= 1;
        Ok(false)
    }
    pub(super) fn sequence_mut(&mut self) -> Result<&mut Sequence, ScoreError> {
        self.sequence
            .as_deref_mut()
            .ok_or(ScoreError::Missing("score sequence"))
    }
    pub(super) fn penalty(&self, id: i32) -> Result<f32, ScoreError> {
        let Some(item) = info(id) else { return Ok(1.0) };
        if item.category == 5 {
            return Ok(1.0);
        }
        curve(
            &self.general,
            260,
            (self.line_accumulator[id as usize] as i8) as f32,
        )
    }
    fn record_finished(&mut self, id: i32) -> bool {
        let Some(item) = info(id) else { return false };
        // TU3 EndTrickGeneric 82DA6198: no history while ScoreHolder+0x1C5D is set; the
        // callers still bank their points.
        if self.trick_score_held() {
            return true;
        }
        accumulate(&mut self.line_accumulator[id as usize], 1);
        if item.trick_type != 0 {
            accumulate(&mut self.tricks_in_sequence[id as usize], 1);
            accumulate(
                &mut self.trick_types_in_sequence[item.trick_type as usize],
                1,
            );
        }
        true
    }
    pub(super) fn end_trick(&mut self, id: i32, points: f32) {
        if self.record_finished(id) {
            self.sequence_score += points;
        }
    }
    pub(super) fn end_air_trick(&mut self, id: i32, points: f32) {
        // TU3 EndAirTrick 82DA629C: nothing is banked while ScoreHolder+0x1C5D is set.
        if !self.record_finished(id) || self.trick_score_held() {
            return;
        }
        let category = info(id).unwrap().category;
        if category != 5 {
            self.air_sequence_score += self.air_sequence_score_pending;
            self.air_sequence_score_pending = 0.0;
        }
        if category == 3 {
            self.air_sequence_score_pending += points;
        } else {
            self.air_sequence_score += points;
        }
    }
    pub(super) fn attempt_beginning(&mut self, trick: &mut Scorable) -> Result<bool, ScoreError> {
        let tick = self
            .sequence
            .as_ref()
            .ok_or(ScoreError::Missing("score sequence"))?
            .global_metrics
            .as_ref()
            .ok_or(ScoreError::Missing("score metrics"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .tick;
        if !trick.ready(tick)? {
            return Ok(false);
        }
        trick.announce_beginning(&self.general)?;
        self.tricks_begun.add(trick)?;
        Ok(true)
    }
    pub(super) fn update_active(&mut self, trick: &mut Scorable) -> Result<bool, ScoreError> {
        let began = self.attempt_beginning(trick)?;
        // TU3 82DA5F4C: not recorded while ScoreHolder+0x1C5D is set.
        if trick.valid_id().is_some() && !trick.is_ending_announced && !self.trick_score_held() {
            self.sequence_mut()?.active_scorables.add(trick)?;
        }
        Ok(began)
    }
    pub(super) fn announce_cancelled(&mut self, trick: &mut Scorable) -> Result<(), ScoreError> {
        if trick.valid_id().is_some() {
            trick.announce_ending(&self.general)?;
            self.tricks_cancelled.add(trick)?;
        }
        Ok(())
    }
    pub(super) fn announce_ending(
        &mut self,
        db: &TrickDatabase,
        trick: &mut Scorable,
    ) -> Result<(), ScoreError> {
        if trick.valid_id().is_some() {
            trick.announce_ending(&self.general)?;
            self.tricks_ended.add(trick)?;
            // TU3 82DA6034: the sequence skips it while ScoreHolder+0x1C5D is set.
            if !self.trick_score_held() {
                self.sequence_mut()?.add_scorable(db, trick)?;
            }
        }
        Ok(())
    }
    pub(super) fn convert(
        &mut self,
        old: &mut Scorable,
        new: &mut Scorable,
    ) -> Result<(), ScoreError> {
        old.announce_converted_from()?;
        new.announce_beginning(&self.general)?;
        self.tricks_converted_from.add(old)?;
        self.tricks_converted_to.add(new)?;
        Ok(())
    }
    pub(super) fn reward_air(&mut self, scalar: f32, out: &mut Scoring2) {
        if !self.air_sequence_reward_pending {
            return;
        }
        self.sequence_score = (self.air_sequence_score_pending + self.air_sequence_score)
            .mul_add(scalar, self.sequence_score);
        let period = &mut self.pending_air_period;
        if !period.just_landed && !period.wiped_out && !period.ran_out && !period.went_offboard {
            period.just_landed = true;
            period.landing_was_sketchy = false;
            period.landed_succesfully = true;
        }
        out.air_finished = true;
        out.air_finished_data = period.clone();
        period.reset();
        self.air_sequence_reward_pending = false;
        self.air_sequence_reward_pending_frame_count = 0;
        self.air_sequence_score = 0.0;
        self.air_sequence_score_pending = 0.0;
    }
    pub(super) fn wipeout(&mut self, out: &mut Scoring2) {
        if !self.air_sequence_reward_pending {
            return;
        }
        out.wipedout_trick_score =
            (self.air_sequence_score + self.air_sequence_score_pending) as i32;
        self.air_sequence_score = 0.0;
        self.air_sequence_score_pending = 0.0;
        self.pending_air_period.just_landed = false;
        self.pending_air_period.wiped_out = true;
        self.pending_air_period.landed_succesfully = false;
        self.reward_air(1.0, out);
    }
    pub(super) fn offboard_runout(&mut self, out: &mut Scoring2) {
        if !self.air_sequence_reward_pending {
            return;
        }
        out.run_out_lost_trick_score =
            (self.air_sequence_score + self.air_sequence_score_pending) as i32;
        self.air_sequence_score = 0.0;
        self.air_sequence_score_pending = 0.0;
        self.pending_air_period.just_landed = false;
        self.pending_air_period.ran_out = true;
        self.pending_air_period.landed_succesfully = false;
        self.reward_air(1.0, out);
    }
    pub(super) fn went_offboard(&mut self, out: &mut Scoring2) {
        if !self.air_sequence_reward_pending {
            return;
        }
        if !self.pending_air_period.just_landed {
            out.offboard_in_air_lost_trick_score =
                (self.air_sequence_score + self.air_sequence_score_pending) as i32;
            self.pending_air_period.went_offboard = true;
            self.pending_air_period.landed_succesfully = false;
            self.air_sequence_score = 0.0;
            self.air_sequence_score_pending = 0.0;
        }
        self.reward_air(1.0, out);
    }
    pub(super) fn end_sequence(&mut self, adjusted: f32, line_active: bool, out: &mut Scoring2) {
        if line_active {
            if self.line_score <= 0.0 && adjusted > 0.0 {
                out.just_started_line_timer = true;
            }
            self.line_score += adjusted;
        }
        self.banked_sequence_score = adjusted;
        self.sequence_score = 0.0;
        self.air_sequence_score = 0.0;
        self.air_sequence_score_pending = 0.0;
        out.finished_sequence_score = adjusted as i64 as u32;
        out.tricks_in_completed_sequence = self.tricks_in_sequence;
        out.trick_types_in_completed_sequence = self.trick_types_in_sequence;
        self.tricks_in_sequence.fill(0);
        self.trick_types_in_sequence.fill(0);
    }
    pub(super) fn fill_output(&self, out: &mut Scoring2) -> Result<(), ScoreError> {
        out.session_score = self.session_score as i32;
        out.line_score = self.line_score as i32;
        out.sequence_score = ((self.air_sequence_score + self.air_sequence_score_pending)
            + self.sequence_score) as i32;
        out.trick_score = self.trick_score as i32;
        out.hud_sequence_score = out.sequence_score.wrapping_add(out.trick_score);
        for (from, to) in [
            (&self.tricks_begun, &mut out.tricks_begun),
            (&self.tricks_ended, &mut out.tricks_ended),
            (&self.tricks_converted_from, &mut out.tricks_converted_from),
            (&self.tricks_converted_to, &mut out.tricks_converted_to),
            (&self.tricks_cancelled, &mut out.tricks_cancelled),
        ] {
            if from.num_entries > 0 {
                *to = from.clone();
            }
        }
        out.tricks_in_current_sequence = self.tricks_in_sequence;
        out.trick_types_in_current_sequence = self.trick_types_in_sequence;
        let banked = if self.banked_sequence_score - self.sequence_score >= 0.0 {
            self.banked_sequence_score
        } else {
            self.sequence_score
        };
        out.banked_point_change_this_frame =
            (banked as i32).wrapping_sub(self.initial_sequence_score);
        self.sequence
            .as_ref()
            .ok_or(ScoreError::Missing("score sequence"))?
            .fill_output(out)
    }
}
