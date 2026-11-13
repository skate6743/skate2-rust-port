//! GroundCollector trick analysis82dfbfe8, using the existing retained owners.
//! Inputs are samples of the original PhysOut/GlobalMetrics/cvar fields; the
//! live conditioner must provide these, not infer tricks from controller input.
use super::{
    super::scoring::Scoring2,
    collectors::GroundCollector,
    scoring_attributes::{ScoreError, scalar},
    scoring_catalog::info,
    scoring_database::TrickDatabase,
};
use crate::skeleton_root_transforms::SkeletonRootVector;

pub(super) struct GroundFrame<'a> {
    pub enabled: &'a [bool; 14],
    pub tick: u32,
    pub dt: f32,
    pub deck_position: SkeletonRootVector,
    pub deck_speed: f32,
    pub switched: bool,
    pub fakie: bool,
    pub is_reverting: bool,
    pub revert_exited_normally: bool,
    pub minimum_distance: f32,
    pub allow_slow_manuals: bool,
    pub allow_slow_manuals_after_begin: bool,
    /// The board's velocity direction (SkateboardReckoning +160).
    pub vel_direction: SkeletonRootVector,
}

// PPC fsel(x-y,x,y), including source NaN/signed-zero selection.
fn maximum(x: f32, y: f32) -> f32 {
    if x - y >= 0.0 { x } else { y }
}

impl GroundCollector {
    /// The timed scorables' travel measure: Skate 3's (a manual along the board's velocity
    /// direction) in Skate 3 scoring, else Skate 2's.
    fn arc_measure(&self, f: &GroundFrame<'_>, manual: bool) -> super::scoring_timed::ArcMeasure {
        #[cfg(feature = "skate3-runtime")]
        if self.base.states.skate3_enabled() {
            return super::scoring_timed::ArcMeasure::Skate3 { along: manual.then_some(f.vel_direction) };
        }
        let _ = manual;
        super::scoring_timed::ArcMeasure::Skate2 { minimum_distance: f.minimum_distance }
    }
    /// IsActivitySequenceWorthy82df00e0. Counts are the native current wheel
    /// contact count and cvar frame limits, not elapsed render frames.
    pub(super) fn activity_sequence_worthy(
        &self,
        was_in_sequence: bool,
        wheels_in_contact: i32,
        min_frames_after_landing: u32,
        extra_off_ground_frames: u32,
        pumping_acceleration: f32,
        deck_up_y: f32,
    ) -> Result<bool, ScoreError> {
        let mut wheels_keep_active = was_in_sequence && wheels_in_contact < 3;
        let mut frames_keep_active =
            was_in_sequence && self.num_frames_on_ground < min_frames_after_landing;
        if wheels_keep_active {
            if self.manuals_are_connectors {
                if self.manual_revert_count > 6 {
                    wheels_keep_active = false;
                }
            } else {
                let holder = self
                    .base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("ground holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?;
                if holder
                    .sequence
                    .as_ref()
                    .ok_or(ScoreError::Missing("ground sequence"))?
                    .trick_count as i32
                    > 0
                {
                    wheels_keep_active = false;
                    frames_keep_active = was_in_sequence
                        && self.num_frames_on_ground
                            < extra_off_ground_frames.wrapping_add(min_frames_after_landing);
                }
            }
        }
        let bits = self.base.animation_trick.bits;
        Ok(
            self.total_trick_score.ok_or(ScoreError::Missing("ground analysis"))? > 0.0
            || self.reverting || (was_in_sequence && bits & 0x3000_0000 != 0)
            || bits & 0x0100_0000 != 0
            || (self.grab_scorable.valid_id().is_some() && self.grab_scorable.is_beginning_announced)
            || self.powerslide_scorable.valid_id().is_some()
            || wheels_keep_active || frames_keep_active
            // IsPumping82deb2d8 / VMX-corrected IsWallRiding82df0048.
            || (was_in_sequence && (pumping_acceleration > 0.1 || 0.65 > deck_up_y)),
        )
    }

    pub(super) fn line_decrement_fraction(&self) -> Result<f32, ScoreError> {
        if self
            .total_trick_score
            .ok_or(ScoreError::Missing("ground analysis"))?
            <= 0.0
        {
            Ok(1.0)
        } else {
            scalar(&self.base.states, 0xf813_b960_d045_0300)
        }
    }

    pub(super) fn momentum_decrement_fraction(&self) -> Result<f32, ScoreError> {
        if self
            .total_trick_score
            .ok_or(ScoreError::Missing("ground analysis"))?
            <= 0.0
        {
            Ok(1.0)
        } else {
            scalar(&self.base.states, 0x4bea_9c28_5a3c_da06)
        }
    }

    /// GroundCollector::FillPhysOut82df0580 and UpdateScoreThresholds82df0370.
    pub(super) fn fill_output(&self, out: &mut Scoring2) -> Result<(), ScoreError> {
        out.is_in_state_ground = true;
        out.manual_length = self.manual_tracker.total_distance;
        out.manual_time = self.manual_tracker.total_time;
        out.powerslide_length = self.powerslide_tracker.total_distance;
        out.powerslide_time = self.powerslide_tracker.total_time;
        self.update_thresholds(out)
    }

    fn update_thresholds(&self, out: &mut Scoring2) -> Result<(), ScoreError> {
        let manual = maximum(out.manual_length, out.finished_manual_length);
        let slide = maximum(out.powerslide_length, out.finished_powerslide_length);
        super::scoring_thresholds::update(
            &self.base.general,
            &mut out.big_trick_achieved_data,
            0,
            manual,
        )?;
        super::scoring_thresholds::update(
            &self.base.general,
            &mut out.big_trick_achieved_data,
            2,
            slide,
        )
    }

    /// Exit82dfbe08. Revert exit announces its end without the regular
    /// UpdateRevert EndTrick banking call; preserve that distinction.
    pub(super) fn exit(
        &mut self,
        score_in_progress: bool,
        db: &TrickDatabase,
        f: &GroundFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        if !score_in_progress {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            if self.powerslide_state != 0 {
                holder.announce_cancelled(&mut self.powerslide_scorable)?;
            }
            if self.manual_tracker.in_state {
                holder.announce_cancelled(&mut self.manual_scorable)?;
            }
            if self.reverting {
                holder.announce_cancelled(&mut self.revert_scorable)?;
            }
            if self.grab_tracker.in_state {
                holder.announce_cancelled(&mut self.grab_scorable)?;
            }
            return Ok(());
        }
        if self.powerslide_state != 0 {
            self.end_powerslide(db, out)?;
            self.powerslide_tracker.score = maximum(
                self.powerslide_tracker.score,
                self.powerslide_scorable.score,
            );
            self.powerslide_tracker.update(
                false,
                false,
                0.0,
                self.powerslide_scorable.valid_id().unwrap_or(-1),
                f.deck_position,
                f.dt,
                self.arc_measure(f, false),
            )?;
        }
        if self.manual_tracker.in_state {
            self.end_manual(db, out)?;
            self.manual_tracker.score = if self.manual_tracker.score >= 0.0 {
                self.manual_tracker.score
            } else {
                0.0
            };
            self.manual_tracker.update(
                false,
                false,
                0.0,
                self.manual_scorable.valid_id().unwrap_or(-1),
                f.deck_position,
                f.dt,
                self.arc_measure(f, true),
            )?;
        }
        if self.reverting {
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .announce_ending(db, &mut self.revert_scorable)?;
        }
        if self.grab_tracker.in_state {
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .announce_ending(db, &mut self.grab_scorable)?;
            self.grab_tracker.score = if self.grab_tracker.score >= 0.0 {
                self.grab_tracker.score
            } else {
                0.0
            };
            self.grab_tracker.update(
                false,
                false,
                0.0,
                self.grab_scorable.valid_id().unwrap_or(-1),
                f.deck_position,
                f.dt,
                self.arc_measure(f, false),
            )?;
        }
        self.update_thresholds(out)
    }

    pub(super) fn update_tricks(
        &mut self,
        db: &TrickDatabase,
        frame: &GroundFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        self.total_trick_score = Some(0.0);
        let slide = self.update_powerslide(db, frame, out)?;
        self.total_trick_score = Some(slide + self.total_trick_score.unwrap());
        let revert = self.update_revert(db, frame, out)?;
        self.total_trick_score = Some(revert + self.total_trick_score.unwrap());
        let grab = self.update_grabs(db, frame)?;
        self.total_trick_score = Some(grab + self.total_trick_score.unwrap());
        let manual = self.update_manual(db, frame, out)?;
        self.total_trick_score = Some(manual + self.total_trick_score.unwrap());
        self.num_frames_on_ground = self.num_frames_on_ground.wrapping_add(1);
        Ok(())
    }

    /// StartNewPowerslide/Manual/Revert82df7e20/7f30/8040 share StartScorable,
    /// but grab uses the actual analyzed animation name82df8110.
    fn start_ground_scorable(
        &mut self,
        db: &TrickDatabase,
        frame: &GroundFrame<'_>,
        id: i32,
        slot: GroundScorable,
    ) -> Result<(), ScoreError> {
        let name = if matches!(slot, GroundScorable::Grab) {
            self.base
                .animation_trick
                .name
                .decode()
                .map_err(|_| ScoreError::Invalid("ground grab animation name"))?
        } else {
            info(id)
                .ok_or(ScoreError::Invalid("ground scorable ID"))?
                .name
                .into()
        };
        let trick = match slot {
            GroundScorable::Powerslide => &mut self.powerslide_scorable,
            GroundScorable::Manual => &mut self.manual_scorable,
            GroundScorable::Revert => &mut self.revert_scorable,
            GroundScorable::Grab => &mut self.grab_scorable,
        };
        self.base.start_scorable(
            db,
            &name,
            id,
            1.0,
            trick,
            frame.tick,
            0.0,
            frame.switched,
            frame.fakie,
        )?;
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("ground holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .attempt_beginning(trick)?;
        Ok(())
    }

    fn end_powerslide(&mut self, db: &TrickDatabase, out: &mut Scoring2) -> Result<(), ScoreError> {
        if self.powerslide_state != 0 {
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .announce_ending(db, &mut self.powerslide_scorable)?;
            out.raw_finished_powerslide = self.powerslide_scorable.valid_id().unwrap_or(-1);
            self.powerslide_state = 0;
            out.just_finished_powerslide = true;
            out.finished_powerslide_length = self.powerslide_tracker.total_distance;
        }
        Ok(())
    }

    fn update_powerslide(
        &mut self,
        db: &TrickDatabase,
        f: &GroundFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<f32, ScoreError> {
        let bits = self.base.animation_trick.bits;
        let active = bits & 0xc000_0000 != 0 && f.enabled[9];
        let mut finished_score = 0.0;
        if active {
            let id = i32::from(bits & 0x8000_0000 == 0);
            if self.powerslide_state != id + 1 {
                self.end_powerslide(db, out)?;
                self.start_ground_scorable(db, f, id, GroundScorable::Powerslide)?;
                out.raw_started_powerslide = id;
                self.powerslide_state = id + 1;
            }
        } else {
            self.end_powerslide(db, out)?;
            finished_score = maximum(
                self.powerslide_tracker.score,
                self.powerslide_scorable.score,
            );
            self.powerslide_tracker.score = finished_score;
            self.powerslide_tracker.update(
                false,
                false,
                0.0,
                self.powerslide_scorable.valid_id().unwrap_or(-1),
                f.deck_position,
                f.dt,
                self.arc_measure(f, false),
            )?;
            self.powerslide_scorable.reset();
            self.powerslide_tracker.reset();
        }
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("ground holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_active(&mut self.powerslide_scorable)?;
        let announced = self.powerslide_scorable.valid_id().is_some()
            && self.powerslide_scorable.is_beginning_announced;
        self.powerslide_tracker.update(
            active,
            announced,
            self.powerslide_scorable.point_rate_scalar,
            self.powerslide_scorable.valid_id().unwrap_or(-1),
            f.deck_position,
            f.dt,
            self.arc_measure(f, false),
        )?;
        if self.powerslide_tracker.score > 0.0 {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            // TU3 82DAA840: held while ScoreHolder+0x1C5D is set.
            if !holder.trick_score_held() {
                holder.trick_score += self.powerslide_tracker.score;
            }
        }
        Ok(maximum(finished_score, self.powerslide_tracker.score))
    }

    /// EndCurrentManual 0x82df9f78: a manual that never validated ends as -1 (the debug build
    /// asserts there, groundcollector.cpp line 248).
    fn end_manual(&mut self, db: &TrickDatabase, out: &mut Scoring2) -> Result<(), ScoreError> {
        out.raw_finished_manual = self.manual_scorable.valid_id().unwrap_or(-1);
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("ground holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .announce_ending(db, &mut self.manual_scorable)?;
        out.just_finished_manual = true;
        out.finished_manual_length = self.manual_tracker.total_distance;
        Ok(())
    }

    fn update_manual(
        &mut self,
        db: &TrickDatabase,
        f: &GroundFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<f32, ScoreError> {
        let bits = self.base.animation_trick.bits;
        let mut active = bits & 0x0c00_0000 != 0 && f.enabled[8];
        let (in_sequence, trick_count) = {
            let holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            let seq = holder
                .sequence
                .as_ref()
                .ok_or(ScoreError::Missing("ground sequence"))?;
            (seq.flags_word & 0x8000_0000 != 0, seq.trick_count as i32)
        };
        if active && f.revert_exited_normally {
            active = false;
            self.manual_revert_count = self.manual_revert_count.wrapping_add(1);
        } else if !in_sequence {
            self.manual_revert_count = 0;
        }
        if active && self.manual_revert_count > 6 {
            active = false;
        }
        let announced = self.manual_scorable.valid_id().is_some()
            && self.manual_scorable.is_beginning_announced;
        // TU3 UpdateManual 82DAA8E0 has no speed condition: a slow manual scores.
        #[cfg(feature = "skate3-runtime")]
        let any_speed = self.base.states.skate3_enabled();
        #[cfg(not(feature = "skate3-runtime"))]
        let any_speed = false;
        // Raw lis/ori/insrdi82dfa260..278, not the truncated decompiler key.
        let fast = any_speed
            || f.allow_slow_manuals
            || f.deck_speed > scalar(&self.base.general, 0xacf9_1a8e_5866_0550)?;
        let eligible = active && announced && (f.allow_slow_manuals_after_begin || fast);
        if active && !fast && !announced && !in_sequence {
            active = false;
        }
        if !self.manuals_are_connectors
            && self.manual_tracker.in_state
            && active
            && announced
            && trick_count > 0
            && self
                .total_trick_score
                .ok_or(ScoreError::Missing("ground analysis"))?
                < 0.0001
            && !self.reverting
            && bits & 0x0100_0000 == 0
        {
            active = false;
        }
        self.manual_tracker.update(
            active,
            eligible,
            self.manual_scorable.point_rate_scalar,
            self.manual_scorable.valid_id().unwrap_or(-1),
            f.deck_position,
            f.dt,
            self.arc_measure(f, true),
        )?;
        let score = self.manual_tracker.score;
        if score > 0.0 {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            // TU3 82DAAAFC: held while ScoreHolder+0x1C5D is set.
            if !holder.trick_score_held() {
                holder.trick_score += score;
            }
        }
        if self.manual_tracker.just_entered_state {
            let id = (((!bits >> 27) & 1) | 2) as i32;
            self.start_ground_scorable(db, f, id, GroundScorable::Manual)?;
            out.raw_started_manual = id;
        } else if self.manual_tracker.just_exited_state {
            self.end_manual(db, out)?;
            self.manual_scorable.reset();
            self.manual_tracker.reset();
        }
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("ground holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_active(&mut self.manual_scorable)?;
        Ok(score)
    }

    fn update_revert(
        &mut self,
        db: &TrickDatabase,
        f: &GroundFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<f32, ScoreError> {
        let was_none = self.previous_animation_revert == 0;
        let bits = self.base.animation_trick.bits;
        self.previous_animation_revert = if bits & 0x2000_0000 != 0 {
            1
        } else if bits & 0x1000_0000 != 0 {
            2
        } else {
            0
        };
        if self.previous_animation_revert != 0 {
            self.most_recent_animation_revert = self.previous_animation_revert;
            if was_none {
                out.raw_started_revert = 4 + i32::from(self.previous_animation_revert != 1);
            }
        }
        // UpdateRevert 0x82dfa598 (TU3 82DAAC10) starts a revert whatever side was last seen (an
        // unseen side is id 5) and ends an invalid one without banking it (EndTrick ignores -1).
        // The groundcollector.cpp asserts there (lines 435 and 371) are debug-build checks only.
        let active = f.is_reverting && f.enabled[11];
        let mut score = 0.0;
        if active && !self.reverting {
            let id = 4 + i32::from(self.most_recent_animation_revert != 1);
            self.start_ground_scorable(db, f, id, GroundScorable::Revert)?;
        } else if !active && self.reverting {
            let id = self.revert_scorable.valid_id().unwrap_or(-1);
            out.raw_finished_revert = id;
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            holder.announce_ending(db, &mut self.revert_scorable)?;
            score = self.revert_scorable.score;
            holder.end_trick(id, score);
            self.revert_scorable.reset();
            self.most_recent_animation_revert = 0;
        }
        self.reverting = active;
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("ground holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_active(&mut self.revert_scorable)?;
        Ok(score)
    }

    /// EndCurrentGrab 0x82dfa788: only the announcement (the debug build asserts a valid grab
    /// first, groundcollector.cpp line 462).
    fn end_grab(&mut self, db: &TrickDatabase) -> Result<(), ScoreError> {
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("ground holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .announce_ending(db, &mut self.grab_scorable)
    }

    fn update_grabs(&mut self, db: &TrickDatabase, f: &GroundFrame<'_>) -> Result<f32, ScoreError> {
        let bits = self.base.animation_trick.bits;
        let mut active = bits & 0x0200_0000 != 0 && f.enabled[4];
        // GroundCollector82dfa88c permits only coffin (catalog60).
        if active && self.base.filtered_scorable_id != 60 {
            active = false;
            self.base.filtered_scorable_id = -1;
        }
        let announced =
            self.grab_scorable.valid_id().is_some() && self.grab_scorable.is_beginning_announced;
        if !self.manuals_are_connectors && self.grab_tracker.in_state && active && announced {
            let holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            if holder
                .sequence
                .as_ref()
                .ok_or(ScoreError::Missing("ground sequence"))?
                .trick_count as i32
                > 0
                && self
                    .total_trick_score
                    .ok_or(ScoreError::Missing("ground analysis"))?
                    < 0.0001
                && !self.reverting
                && bits & 0x0100_0000 == 0
            {
                active = false;
            }
        }
        if active {
            let id = self.base.filtered_scorable_id;
            if !self.grab_tracker.in_state {
                if info(id).is_some() {
                    self.start_ground_scorable(db, f, id, GroundScorable::Grab)?;
                }
            } else if Some(id) != self.grab_scorable.valid_id() && info(id).is_some() {
                self.grab_tracker.score = if self.grab_tracker.score >= 0.0 {
                    self.grab_tracker.score
                } else {
                    0.0
                };
                self.grab_tracker.update(
                    false,
                    false,
                    0.0,
                    self.grab_scorable.valid_id().unwrap_or(-1),
                    f.deck_position,
                    f.dt,
                    self.arc_measure(f, false),
                )?;
                self.end_grab(db)?;
                self.start_ground_scorable(db, f, id, GroundScorable::Grab)?;
            }
        } else if self.grab_tracker.in_state {
            self.end_grab(db)?;
            self.grab_tracker.score = if self.grab_tracker.score >= 0.0 {
                self.grab_tracker.score
            } else {
                0.0
            };
            self.grab_tracker.update(
                false,
                false,
                0.0,
                self.grab_scorable.valid_id().unwrap_or(-1),
                f.deck_position,
                f.dt,
                self.arc_measure(f, false),
            )?;
            self.grab_scorable.reset();
            self.grab_tracker.reset();
        }
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("ground holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_active(&mut self.grab_scorable)?;
        let eligible = active
            && self.grab_scorable.valid_id().is_some()
            && self.grab_scorable.is_beginning_announced;
        self.grab_tracker.update(
            active,
            eligible,
            self.grab_scorable.point_rate_scalar,
            self.grab_scorable.valid_id().unwrap_or(-1),
            f.deck_position,
            f.dt,
            self.arc_measure(f, false),
        )?;
        if self.grab_tracker.score > 0.0 {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("ground holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            // TU3 82DAB284: held while ScoreHolder+0x1C5D is set.
            if !holder.trick_score_held() {
                holder.trick_score += self.grab_tracker.score;
            }
        }
        Ok(self.grab_tracker.score)
    }
}

#[derive(Clone, Copy)]
enum GroundScorable {
    Powerslide,
    Manual,
    Revert,
    Grab,
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
