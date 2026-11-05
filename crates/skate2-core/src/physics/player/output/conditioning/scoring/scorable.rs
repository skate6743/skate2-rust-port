//! Original Scorable lifecycle shared by retained collectors and output copies.
use super::{
    scorable::Scorable,
    scoring_attributes::{ScoreError, curve, scalar},
    scoring_database::TrickDatabase,
};
use super::scoring_attributes::ScoringAttributes;
impl Scorable {
    pub(super) fn init(
        &mut self,
        db: &TrickDatabase,
        general: &ScoringAttributes,
        name: &str,
        point_scalar: f32,
        switched: bool,
        fakie: bool,
        tick: u32,
        extra_min_time: f32,
    ) -> Result<(), ScoreError> {
        self.is_beginning_announced = false;
        self.is_ending_announced = false;
        self.was_a_tap_trick = false;
        self.started_switched = switched;
        self.started_fakie = fakie;
        self.point_scalar = point_scalar;
        self.score = 0.0;
        self.had_object_between_board_and_skater = false;
        self.start_tick = tick;
        db.lookup(name, &mut self.trick_scoring_settings)?;
        if self.trick_scoring_settings.is_valid {
            self.point_rate_scalar = self.point_scalar
                * curve(general, 180, self.trick_scoring_settings.base_score as f32)?;
            self.min_trick_time_in_ticks =
                ((self.trick_scoring_settings.min_trick_time + extra_min_time) * 60.0) as u32;
        } else {
            self.point_rate_scalar = 0.0;
            self.min_trick_time_in_ticks = 0;
        }
        Ok(())
    }
    pub(super) fn base_score(&self, general: &ScoringAttributes) -> Result<f32, ScoreError> {
        if !self.trick_scoring_settings.is_valid {
            return Err(ScoreError::Invalid("base score of invalid trick"));
        }
        let score = self.trick_scoring_settings.base_score as f32 * self.point_scalar;
        Ok(if self.was_a_tap_trick {
            scalar(general, 0x2577_df0f_ce52_51e4)? * score
        } else {
            score
        })
    }
    pub(super) fn ready(&self, tick: u32) -> Result<bool, ScoreError> {
        if self.is_beginning_announced || !self.trick_scoring_settings.is_valid {
            return Ok(false);
        }
        if tick < self.start_tick {
            return Err(ScoreError::Invalid("scorable tick before start"));
        }
        Ok(tick - self.start_tick >= self.min_trick_time_in_ticks)
    }
    pub(super) fn announce_beginning(
        &mut self,
        general: &ScoringAttributes,
    ) -> Result<(), ScoreError> {
        if self.is_beginning_announced {
            return Err(ScoreError::Invalid("duplicate trick beginning"));
        }
        self.is_beginning_announced = true;
        self.score = self.base_score(general)? + self.score;
        Ok(())
    }
    pub(super) fn announce_ending(
        &mut self,
        general: &ScoringAttributes,
    ) -> Result<(), ScoreError> {
        if self.is_ending_announced || self.was_a_tap_trick {
            return Err(ScoreError::Invalid("duplicate trick ending"));
        }
        if !self.is_beginning_announced {
            self.was_a_tap_trick = true;
            self.score = self.base_score(general)? + self.score;
        }
        self.is_ending_announced = true;
        Ok(())
    }
    pub(super) fn announce_converted_from(&mut self) -> Result<(), ScoreError> {
        if self.is_ending_announced || self.was_a_tap_trick {
            return Err(ScoreError::Invalid("converted finished trick"));
        }
        self.is_ending_announced = true;
        Ok(())
    }
    pub(super) fn valid_id(&self) -> Option<i32> {
        let s = &self.trick_scoring_settings;
        (s.is_valid && super::scoring_catalog::info(s.scorable_id).is_some()).then_some(s.scorable_id)
    }
}
impl super::super::scoring::Scorables {
    pub(super) fn add(&mut self, scorable: &Scorable) -> Result<(), ScoreError> {
        let i = usize::try_from(self.num_entries)
            .map_err(|_| ScoreError::Invalid("negative quicklist count"))?;
        let slot = self
            .elements
            .get_mut(i)
            .ok_or(ScoreError::Invalid("scoring quicklist exceeds four"))?;
        *slot = scorable.clone();
        self.num_entries += 1;
        Ok(())
    }
}
