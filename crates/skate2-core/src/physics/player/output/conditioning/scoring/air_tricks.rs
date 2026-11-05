//! AirCollector trick lifecycle82df7a90/7c18/8f98/9080.
use super::{
    super::{scoring::Scoring2, simple::GroundRemainder},
    air_collector::AirCollector,
    scoring_attributes::{ScoreError, curve, scalar},
    scoring_catalog::info,
    scoring_database::TrickDatabase,
};

pub(super) struct AirTrickFrame<'a> {
    pub tick: u32,
    pub dt: f32,
    pub switched: bool,
    pub fakie: bool,
    pub ground: &'a GroundRemainder,
    /// The air output's locked trajectory (TU3 PhysOut_Air+452).
    #[cfg(feature = "skate3-runtime")]
    pub use_air_reckoning: bool,
}

impl AirCollector {
    fn remember_first(&mut self) {
        if self.air_scorable.valid_id().is_some() && self.trick_count == 1 {
            self.first_scorable = self.air_scorable.clone();
        }
    }

    fn start_new_trick(
        &mut self,
        db: &TrickDatabase,
        f: &AirTrickFrame<'_>,
        id: i32,
        tick: u32,
        extra_min_time: f32,
    ) -> Result<(), ScoreError> {
        let Some(item) = info(id) else { return Ok(()) };
        let name = self
            .base
            .animation_trick
            .name
            .decode()
            .map_err(|_| ScoreError::Invalid("air animation name"))?;
        self.base.start_scorable(
            db,
            &name,
            id,
            self.in_air_point_scalar,
            &mut self.air_scorable,
            tick,
            extra_min_time,
            f.switched,
            f.fakie,
        )?;
        {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("air holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            if holder.attempt_beginning(&mut self.air_scorable)?
                && !self.most_recent_trick_point_scalar_is_set
            {
                self.most_recent_trick_point_scalar = holder.penalty(id)?;
                self.most_recent_trick_point_scalar_is_set = true;
            }
        }
        if item.category == 2 {
            self.grab_occurred_in_air = true;
        }
        self.trick_count = self.trick_count.wrapping_add(1);
        self.remember_first();
        Ok(())
    }

    fn convert_to_trick(
        &mut self,
        db: &TrickDatabase,
        f: &AirTrickFrame<'_>,
        id: i32,
    ) -> Result<(), ScoreError> {
        let mut old = self.air_scorable.clone();
        let name = self
            .base
            .animation_trick
            .name
            .decode()
            .map_err(|_| ScoreError::Invalid("air animation name"))?;
        self.base.start_scorable(
            db,
            &name,
            id,
            self.in_air_point_scalar,
            &mut self.air_scorable,
            f.tick,
            0.0,
            f.switched,
            f.fakie,
        )?;
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("air holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .convert(&mut old, &mut self.air_scorable)?;
        self.remember_first();
        Ok(())
    }

    pub(super) fn end_current_trick(&mut self, db: &TrickDatabase) -> Result<(), ScoreError> {
        if let Some(id) = self.air_scorable.valid_id() {
            {
                let mut holder = self
                    .base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("air holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?;
                holder.announce_ending(db, &mut self.air_scorable)?;
                holder.end_air_trick(id, self.air_scorable.score);
            }
            self.remember_first();
        }
        Ok(())
    }

    pub(super) fn update_tricks(
        &mut self,
        db: &TrickDatabase,
        f: &AirTrickFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<f32, ScoreError> {
        let next = info(self.base.filtered_scorable_id).filter(|i| i.trick_type != 5);
        if next.map(|i| i.id) != self.air_scorable.valid_id() {
            if let Some(item) = next {
                if info(item.consumed).is_some() {
                    self.convert_to_trick(db, f, item.id)?;
                } else {
                    let unannounced_grab = !self.air_scorable.is_beginning_announced
                        && self
                            .air_scorable
                            .valid_id()
                            .and_then(info)
                            .is_some_and(|i| i.category == 2)
                        && item.category == 2;
                    if unannounced_grab {
                        let tick = self.air_scorable.start_tick;
                        self.tweaked_grab_sequence_count =
                            self.tweaked_grab_sequence_count.wrapping_add(1);
                        let extra = if self.tweaked_grab_sequence_count > 1 {
                            scalar(&self.base.states, 0xc4bd_c0cd_02c8_3a58)?
                        } else {
                            0.0
                        };
                        self.start_new_trick(db, f, item.id, tick, extra)?;
                    } else {
                        self.tweaked_grab_sequence_count = 0;
                        self.end_current_trick(db)?;
                        self.start_new_trick(db, f, item.id, f.tick, 0.0)?;
                    }
                }
            } else {
                self.end_current_trick(db)?;
                self.air_scorable.reset();
            }
        }
        {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("air holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            if holder.update_active(&mut self.air_scorable)?
                && !self.most_recent_trick_point_scalar_is_set
            {
                self.most_recent_trick_point_scalar =
                    holder.penalty(self.air_scorable.valid_id().unwrap_or(-1))?;
                self.most_recent_trick_point_scalar_is_set = true;
            }
        }
        // Native copies the first scorable before adding this frame's hold/flip bonus.
        self.remember_first();
        let Some(item) = self.air_scorable.valid_id().and_then(info) else {
            return Ok(0.0);
        };
        if !self.air_scorable.is_beginning_announced {
            return Ok(0.0);
        }
        // TU3 UpdateTricks 82DA976C: no hold or hippy-flip points while the air trajectory is
        // locked (the collector's copy of PhysOut_Air+452).
        #[cfg(feature = "skate3-runtime")]
        if self.base.states.skate3_enabled() && f.use_air_reckoning {
            return Ok(0.0);
        }
        if item.category == 2 {
            self.air_grab_time += f.dt;
            let rate = curve(&self.base.states, 1120, self.air_grab_time)?;
            self.air_scorable.score = self
                .air_scorable
                .point_rate_scalar
                .mul_add(rate * f.dt, self.air_scorable.score);
        } else if item.category == 3 && !self.air_scorable.had_object_between_board_and_skater {
            self.base.check_hippy_flip(f.ground, out)?;
            if out.is_hippy_flipping {
                let bonus = self.air_scorable.trick_scoring_settings.base_score as f32
                    * scalar(&self.base.states, 0x1ca6_3104_2d29_2121)?;
                self.air_scorable.score = self
                    .air_scorable
                    .point_scalar
                    .mul_add(bonus, self.air_scorable.score);
                self.air_scorable.had_object_between_board_and_skater = true;
            }
        }
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("air holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        // TU3 82DA9890: held while ScoreHolder+0x1C5D is set.
        if !holder.trick_score_held() {
            holder.trick_score = self.air_scorable.score + holder.trick_score;
        }
        Ok(self.air_scorable.score)
    }
}
