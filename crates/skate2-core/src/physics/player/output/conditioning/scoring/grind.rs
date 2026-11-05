//! GrindCollector82df9858/9600/9790; score the existing filtered grind output.
use super::{
    super::scoring::Scoring2, collectors::GrindCollector, scoring_attributes::ScoreError,
    scoring_catalog::info, scoring_database::TrickDatabase,
};
use crate::skeleton_root_transforms::SkeletonRootVector;

pub(super) struct GrindFrame {
    pub filtered_scorable_id: i32,
    pub pathed_guid: u64,
    pub local_guid: u64,
    pub tick: u32,
    pub dt: f32,
    pub deck_position: SkeletonRootVector,
    pub minimum_distance: f32,
    pub switched: bool,
    pub fakie: bool,
}
impl GrindCollector {
    /// The timed scorable's travel measure: Skate 3's step length in Skate 3 scoring, else
    /// Skate 2's.
    fn arc_measure(&self, f: &GrindFrame) -> super::scoring_timed::ArcMeasure {
        #[cfg(feature = "skate3-runtime")]
        if self.base.states.skate3_enabled() {
            return super::scoring_timed::ArcMeasure::Skate3 { along: None };
        }
        super::scoring_timed::ArcMeasure::Skate2 { minimum_distance: f.minimum_distance }
    }
    fn end_current(&mut self, db: &TrickDatabase) -> Result<(), ScoreError> {
        if let Some(id) = self.scorable.valid_id() {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("grind holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            holder.announce_ending(db, &mut self.scorable)?;
            // Changing grind records the trick, leaving distance/time points in
            // the continuous tracker until the entire grind ends.
            holder.end_trick(id, 0.0);
            self.scorable.reset();
        }
        Ok(())
    }
    fn start_current(
        &mut self,
        db: &TrickDatabase,
        f: &GrindFrame,
        id: i32,
    ) -> Result<(), ScoreError> {
        let Some(item) = info(id) else { return Ok(()) };
        if item.trick_type != 5 {
            return Err(ScoreError::Invalid("non-grind in grind collector"));
        }
        self.base.start_scorable(
            db,
            item.name,
            id,
            1.0,
            &mut self.scorable,
            f.tick,
            0.0,
            f.switched,
            f.fakie,
        )?;
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("grind holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .attempt_beginning(&mut self.scorable)?;
        Ok(())
    }
    pub(super) fn update_tricks(
        &mut self,
        db: &TrickDatabase,
        f: &GrindFrame,
        out: &Scoring2,
    ) -> Result<(), ScoreError> {
        let mut id = f.filtered_scorable_id;
        let anim_id = self.base.filtered_scorable_id;
        // Raw82df98d4..98e0: decompiler incorrectly drops this assignment.
        if id != anim_id && info(anim_id).is_some() && db.is_grind_grab(anim_id)? {
            id = anim_id;
        }
        if id != self.scorable.valid_id().unwrap_or(-1) {
            self.end_current(db)?;
            self.start_current(db, f, id)?;
        }
        // cmpdi, not a comparison of either 32-bit half.
        if f.pathed_guid != u64::MAX {
            self.info.spline_pathed_guid = f.pathed_guid;
        }
        if f.local_guid != u64::MAX {
            self.info.spline_local_guid = f.local_guid;
        }
        if out.current_frame_grind_object_info.is_valid {
            self.info.object_info = out.current_frame_grind_object_info.clone();
        }
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("grind holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_active(&mut self.scorable)?;
        let eligible = self.scorable.valid_id().is_some() && self.scorable.is_beginning_announced;
        self.tracker.update(
            true,
            eligible,
            self.scorable.point_rate_scalar,
            self.scorable.valid_id().unwrap_or(-1),
            f.deck_position,
            f.dt,
            self.arc_measure(f),
        )?;
        super::scoring_module_finish::trace_score(format_args!(
            "grind frame tick {} id {:?} eligible {eligible} distance {} deck {:?}",
            f.tick,
            self.scorable.valid_id(),
            self.tracker.total_distance,
            f.deck_position.xyz,
        ));
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("grind holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        // TU3 82DA9FD8: held while ScoreHolder+0x1C5D is set.
        if !holder.trick_score_held() {
            holder.trick_score += self.tracker.score;
        }
        Ok(())
    }
    pub(super) fn exit(
        &mut self,
        score_in_progress: bool,
        db: &TrickDatabase,
        f: &GrindFrame,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        super::scoring_module_finish::trace_score(format_args!(
            "grind exit score={score_in_progress} scorable={:?} length={} pathed={:#x} local={:#x}",
            self.scorable.valid_id(),
            self.tracker.total_distance,
            self.info.spline_pathed_guid,
            self.info.spline_local_guid
        ));
        if score_in_progress {
            out.just_finished_grind = true;
            self.info.length = self.tracker.total_distance;
            out.finished_grind_info = self.info.clone();
            if let Some(id) = self.scorable.valid_id() {
                self.tracker.score = if self.tracker.score >= 0.0 {
                    self.tracker.score
                } else {
                    0.0
                };
                self.tracker.update(
                    false,
                    false,
                    0.0,
                    id,
                    f.deck_position,
                    f.dt,
                    self.arc_measure(f),
                )?;
                self.base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("grind holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?
                    .announce_ending(db, &mut self.scorable)?;
            }
            self.update_thresholds(out)?;
        } else if self.scorable.valid_id().is_some() {
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("grind holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .announce_cancelled(&mut self.scorable)?;
        }
        Ok(())
    }
    pub(super) fn fill_output(&mut self, out: &mut Scoring2) -> Result<(), ScoreError> {
        out.is_in_state_grinding = true;
        self.info.length = self.tracker.total_distance;
        out.current_grind_info = self.info.clone();
        self.update_thresholds(out)
    }
    fn update_thresholds(&self, out: &mut Scoring2) -> Result<(), ScoreError> {
        let current = out.current_grind_info.length;
        let finished = out.finished_grind_info.length;
        let value = if current - finished >= 0.0 {
            current
        } else {
            finished
        };
        super::scoring_thresholds::update(
            &self.base.general,
            &mut out.big_trick_achieved_data,
            1,
            value,
        )
    }
}
