//! HandplantCollector82dfad90/0708/c078/c1f8, extending its retained constructor.
use super::{
    super::{scoring::Scoring2, simple::GroundRemainder},
    collectors::HandplantCollector,
    scoring_attributes::{ScoreError, curve},
    scoring_catalog::info,
    scoring_database::TrickDatabase,
};

const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);

impl HandplantCollector {
    /// EndCurrentHandplant 0x82dface0: a handplant that never validated ends as -1, which
    /// EndTrick ignores (the debug build asserts there, handplantcollector.cpp line 118).
    fn end_current(&mut self, db: &TrickDatabase) -> Result<(), ScoreError> {
        let id = self.scorable.valid_id().unwrap_or(-1);
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("handplant holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        holder.announce_ending(db, &mut self.scorable)?;
        holder.end_trick(id, self.scorable.score);
        Ok(())
    }

    pub(super) fn update_tricks(
        &mut self,
        db: &TrickDatabase,
        tick: u32,
        switched: bool,
        fakie: bool,
        ground: &GroundRemainder,
    ) -> Result<(), ScoreError> {
        let next = info(self.base.filtered_scorable_id)
            .filter(|i| i.trick_type == 6)
            .map(|i| i.id);
        if let Some(id) = next {
            if self.scorable.valid_id() != Some(id) {
                // An unannounced handplant change inherits its original start
                // tick. Announced changes end/bank, then start at current tick.
                let mut start_tick = tick;
                if self.scorable.valid_id().is_some() {
                    if self.scorable.is_beginning_announced {
                        self.end_current(db)?;
                    } else {
                        start_tick = self.scorable.start_tick;
                    }
                }
                let name = self
                    .base
                    .animation_trick
                    .name
                    .decode()
                    .map_err(|_| ScoreError::Invalid("handplant animation name"))?;
                self.base.start_scorable(
                    db,
                    &name,
                    id,
                    1.0,
                    &mut self.scorable,
                    start_tick,
                    0.0,
                    switched,
                    fakie,
                )?;
                self.base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("handplant holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?
                    .attempt_beginning(&mut self.scorable)?;
            }
            self.num_frames_in_handplant = self.num_frames_in_handplant.wrapping_add(1);
        } else if self.scorable.valid_id().is_some() {
            self.end_current(db)?;
            self.scorable.reset();
        }
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("handplant holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_active(&mut self.scorable)?;
        if self.scorable.valid_id().is_some() && self.scorable.is_beginning_announced {
            let rate = curve(
                &self.base.states,
                320,
                self.num_frames_in_handplant as f32 * UPDATE_TIME,
            )?;
            // Raw82dfaf90/9c/a0 use MUL, MUL, ADD (not fused).
            let points = (self.scorable.point_rate_scalar * rate) * UPDATE_TIME;
            self.scorable.score += points;
            self.time_duration_points += points;
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("handplant holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            // TU3 82DABC4C: held while ScoreHolder+0x1C5D is set.
            if !holder.trick_score_held() {
                holder.trick_score = self.scorable.score + holder.trick_score;
            }
        }
        self.num_frames_in_collector = self.num_frames_in_collector.wrapping_add(1);
        self.update_height(ground)?;
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("handplant holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        // TU3 82DAB8E0: held while ScoreHolder+0x1C5D is set.
        if !holder.trick_score_held() {
            holder.trick_score = self.max_height_above_ground_points + holder.trick_score;
        }
        Ok(())
    }

    fn update_height(&mut self, ground: &GroundRemainder) -> Result<(), ScoreError> {
        // Corrected VMX operands82df07d4=62-60,82df0880=57-55: both
        // subtract the actual ground hit Y from that probe's start Y.
        let hips = if ground.below_hips_ground_data_is_valid {
            ground.below_hips_ground_data_start_pos.xyz[1]
                - ground.below_hips_ground_data_position.xyz[1]
        } else {
            0.0
        };
        let deck = if ground.below_deck_ground_data_is_valid {
            ground.below_deck_ground_data_start_pos.xyz[1]
                - ground.below_deck_ground_data_position.xyz[1]
        } else {
            0.0
        };
        let height = if hips - deck >= 0.0 { hips } else { deck };
        self.max_height_above_ground = if self.max_height_above_ground - height >= 0.0 {
            self.max_height_above_ground
        } else {
            height
        };
        self.max_height_above_ground_points =
            curve(&self.base.states, 400, self.max_height_above_ground)?;
        Ok(())
    }

    pub(super) fn exit(
        &mut self,
        score_in_progress: bool,
        db: &TrickDatabase,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        if score_in_progress {
            if self.scorable.valid_id().is_some() {
                self.end_current(db)?;
            }
            // ID253 is the native height metric (typeNone), so shared EndTrick
            // performs precisely the inline histogram+sequence-score stores.
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("handplant holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .end_trick(253, self.max_height_above_ground_points);
            super::scoring_thresholds::update(
                &self.base.general,
                &mut out.big_trick_achieved_data,
                3,
                self.max_height_above_ground,
            )?;
            out.just_finished_handplant = true;
            out.finished_handplant_duration_in_seconds =
                self.num_frames_in_handplant as f32 * UPDATE_TIME;
        } else if self.scorable.valid_id().is_some() {
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("handplant holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .announce_cancelled(&mut self.scorable)?;
        }
        Ok(())
    }

    pub(super) fn fill_output(&self, out: &mut Scoring2) -> Result<(), ScoreError> {
        out.is_in_state_handplant = true;
        super::scoring_thresholds::update(
            &self.base.general,
            &mut out.big_trick_achieved_data,
            3,
            self.max_height_above_ground,
        )
    }
}
