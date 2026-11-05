//! Original retained collector dispatch, vtables8236c2b8..8236c3ac.
//! No additional collectors are allocated by this path.
use super::{
    super::{PhysicalPlayerPhysOut, scoring::Scoring2, trick::AnimOutTrickData},
    collector_base::CollectorBase,
    score_module::{CurrentCollector, ScoreModule},
    scoring_air_tricks::AirTrickFrame,
    scoring_attributes::{ScoreError, scalar},
    scoring_database::TrickDatabase,
    scoring_grind::GrindFrame,
    scoring_ground::GroundFrame,
    scoring_meters::MeterKind,
    scoring_module_selection::CollectorKind as Kind,
    scoring_offboard::OffboardFrame,
};
use crate::skeleton_root_transforms::SkeletonRootVector;

/// References to the same conditioned physics frame, shared by the existing
/// collectors. The live producer owns sampling; dispatch never polls input.
pub(super) struct CollectorFrame<'a> {
    pub ground: GroundFrame<'a>,
    pub grind: GrindFrame,
    pub offboard: OffboardFrame<'a>,
    pub air: AirTrickFrame<'a>,
    pub animation: &'a AnimOutTrickData,
    pub damped_system_velocity: SkeletonRootVector,
    pub damped_system_up: SkeletonRootVector,
    pub wheels_in_contact: i32,
    pub min_frames_after_landing: u32,
    pub extra_off_ground_frames: u32,
    pub pumping_acceleration: f32,
    pub deck_up_y: f32,
    pub flipping: bool,
    pub front_flipping: bool,
    pub wiped_out: bool,
    pub score_over_skaters: bool,
}
impl ScoreModule {
    fn kind(&self) -> Result<Kind, ScoreError> {
        Kind::from_raw(
            self.reset_suffix
                .as_ref()
                .ok_or(ScoreError::Missing("score reset"))?
                .collector_type,
        )
    }
    fn with_current_base<T>(
        &mut self,
        f: impl FnOnce(&mut CollectorBase) -> Result<T, ScoreError>,
    ) -> Result<T, ScoreError> {
        match self
            .current_collector
            .as_ref()
            .ok_or(ScoreError::Missing("current collector"))?
        {
            CurrentCollector::Ground => f(&mut self
                .ground
                .published_value_mut()
                .ok_or(ScoreError::Missing("ground"))?
                .base),
            CurrentCollector::Air => f(&mut self
                .air
                .published_value_mut()
                .ok_or(ScoreError::Missing("air"))?
                .base),
            CurrentCollector::Grind => f(&mut self
                .grind
                .published_value_mut()
                .ok_or(ScoreError::Missing("grind"))?
                .base),
            CurrentCollector::Handplant => f(&mut self
                .handplant
                .published_value_mut()
                .ok_or(ScoreError::Missing("handplant"))?
                .base),
            CurrentCollector::Offboard => f(&mut self
                .offboard
                .published_value_mut()
                .ok_or(ScoreError::Missing("offboard"))?
                .base),
            CurrentCollector::Other(owner) => {
                f(&mut *owner.lock().map_err(|_| ScoreError::Poisoned)?)
            }
        }
    }
    pub(super) fn activity_worthy(
        &self,
        was_in_sequence: bool,
        f: &CollectorFrame<'_>,
    ) -> Result<bool, ScoreError> {
        Ok(match self.kind()? {
            Kind::Ground => self
                .ground
                .published_value()
                .ok_or(ScoreError::Missing("ground"))?
                .activity_sequence_worthy(
                    was_in_sequence,
                    f.wheels_in_contact,
                    f.min_frames_after_landing,
                    f.extra_off_ground_frames,
                    f.pumping_acceleration,
                    f.deck_up_y,
                )?,
            Kind::Air => self
                .air
                .published_value()
                .ok_or(ScoreError::Missing("air"))?
                .activity_sequence_worthy(was_in_sequence),
            Kind::Offboard => self
                .offboard
                .published_value()
                .ok_or(ScoreError::Missing("offboard"))?
                .activity_sequence_worthy(),
            // Shared linker-folded bool-return bodies verified in the vtables.
            Kind::Grind | Kind::Handplant => true,
            Kind::Invalid | Kind::Wipeout | Kind::Teleport => false,
        })
    }
    fn exit_collector(
        &mut self,
        score: bool,
        db: &TrickDatabase,
        f: &CollectorFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        match self.kind()? {
            Kind::Ground => self
                .ground
                .published_value_mut()
                .ok_or(ScoreError::Missing("ground"))?
                .exit(score, db, &f.ground, out),
            Kind::Air => self
                .air
                .published_value_mut()
                .ok_or(ScoreError::Missing("air"))?
                .exit(
                    score,
                    db,
                    f.air.ground,
                    f.wiped_out,
                    f.score_over_skaters,
                    out,
                ),
            Kind::Grind => self
                .grind
                .published_value_mut()
                .ok_or(ScoreError::Missing("grind"))?
                .exit(score, db, &f.grind, out),
            Kind::Handplant => self
                .handplant
                .published_value_mut()
                .ok_or(ScoreError::Missing("handplant"))?
                .exit(score, db, out),
            Kind::Offboard => self
                .offboard
                .published_value_mut()
                .ok_or(ScoreError::Missing("offboard"))?
                .exit(score, db, out),
            Kind::Invalid | Kind::Wipeout | Kind::Teleport => Ok(()), // original empty vcall
        }
    }
    fn enter_collector(
        &mut self,
        kind: Kind,
        output: &PhysicalPlayerPhysOut,
        f: &CollectorFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        self.current_collector = Some(match kind {
            Kind::Ground => CurrentCollector::Ground,
            Kind::Air => CurrentCollector::Air,
            Kind::Grind => CurrentCollector::Grind,
            Kind::Handplant => CurrentCollector::Handplant,
            Kind::Offboard => CurrentCollector::Offboard,
            _ => CurrentCollector::Other(
                self.other
                    .published_value()
                    .ok_or(ScoreError::Missing("other"))?
                    .clone(),
            ),
        });
        // Shared Enter82df0a68 calls the concrete Reset virtually. Air has its
        // own Enter and rewards an older pending air sequence before resetting.
        match kind {
            Kind::Ground => self
                .ground
                .published_value_mut()
                .ok_or(ScoreError::Missing("ground"))?
                .reset(),
            Kind::Air => self
                .air
                .published_value_mut()
                .ok_or(ScoreError::Missing("air"))?
                .enter(
                    output,
                    out,
                    f.damped_system_velocity,
                    f.damped_system_up,
                    f.air.switched,
                    f.air.fakie,
                )?,
            Kind::Grind => self
                .grind
                .published_value_mut()
                .ok_or(ScoreError::Missing("grind"))?
                .reset(),
            Kind::Handplant => self
                .handplant
                .published_value_mut()
                .ok_or(ScoreError::Missing("handplant"))?
                .reset(),
            Kind::Offboard => self
                .offboard
                .published_value_mut()
                .ok_or(ScoreError::Missing("offboard"))?
                .reset(),
            _ => self.with_current_base(|base| {
                base.reset_analysis();
                Ok(())
            })?,
        }
        Ok(())
    }
    /// ManageCurrentCollector82dfca20, including Nursary.ResetSequence82deb608.
    pub(super) fn manage_collector(
        &mut self,
        next: Kind,
        output: &PhysicalPlayerPhysOut,
        db: &TrickDatabase,
        f: &CollectorFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let previous = self.kind()?;
        if previous == next {
            if self.reset_suffix.as_ref().unwrap().reset_sequence {
                self.exit_collector(true, db, f, out)?;
                self.enter_collector(next, output, f, out)?;
            }
            self.reset_suffix.as_mut().unwrap().just_entered_wipeout = false;
        } else {
            let wipeout = next == Kind::Wipeout;
            self.reset_suffix.as_mut().unwrap().just_entered_wipeout = wipeout;
            let in_sequence = self
                .holder
                .published_value()
                .ok_or(ScoreError::Missing("holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .sequence_mut()?
                .flags_word
                & 0x8000_0000
                != 0;
            out.wipeout_interrupted_trick = wipeout && self.activity_worthy(in_sequence, f)?;
            self.exit_collector(next.scores_previous(), db, f, out)?;
            self.enter_collector(next, output, f, out)?;
            if next == Kind::Offboard {
                let mut holder = self
                    .holder
                    .published_value()
                    .ok_or(ScoreError::Missing("holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?;
                if out.is_running_out {
                    holder.offboard_runout(out);
                } else {
                    holder.went_offboard(out);
                }
            }
        }
        self.reset_suffix.as_mut().unwrap().collector_type = next as i32;
        Ok(())
    }
    pub(super) fn update_collector(
        &mut self,
        db: &TrickDatabase,
        f: &CollectorFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        self.with_current_base(|base| {
            base.process_animation(db, f.animation, f.ground.enabled, out)
        })?;
        match self.kind()? {
            Kind::Ground => self
                .ground
                .published_value_mut()
                .ok_or(ScoreError::Missing("ground"))?
                .update_tricks(db, &f.ground, out)?,
            Kind::Air => self
                .air
                .published_value_mut()
                .ok_or(ScoreError::Missing("air"))?
                .update_analysis(
                    db,
                    &f.air,
                    f.ground.deck_position,
                    f.offboard.player_matrix,
                    f.offboard.board_matrix,
                    f.damped_system_up,
                    f.flipping,
                    f.front_flipping,
                    out,
                )?,
            Kind::Grind => self
                .grind
                .published_value_mut()
                .ok_or(ScoreError::Missing("grind"))?
                .update_tricks(db, &f.grind, out)?,
            Kind::Handplant => {
                let collector = self
                    .handplant
                    .published_value_mut()
                    .ok_or(ScoreError::Missing("handplant"))?;
                collector.update_tricks(
                    db,
                    f.air.tick,
                    f.air.switched,
                    f.air.fakie,
                    f.air.ground,
                )?;
            }
            Kind::Offboard => {
                let collector = self
                    .offboard
                    .published_value_mut()
                    .ok_or(ScoreError::Missing("offboard"))?;
                collector.update_tricks(db, &f.offboard, out)?;
            }
            _ => {}
        }
        self.finish_raw_animation(out)
    }
    pub(super) fn fill_collector_output(
        &mut self,
        wiped_out: bool,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        match self.kind()? {
            Kind::Ground => self
                .ground
                .published_value()
                .ok_or(ScoreError::Missing("ground"))?
                .fill_output(out)?,
            Kind::Air => self
                .air
                .published_value()
                .ok_or(ScoreError::Missing("air"))?
                .fill_output(out, wiped_out)?,
            Kind::Grind => self
                .grind
                .published_value_mut()
                .ok_or(ScoreError::Missing("grind"))?
                .fill_output(out)?,
            Kind::Handplant => self
                .handplant
                .published_value()
                .ok_or(ScoreError::Missing("handplant"))?
                .fill_output(out)?,
            Kind::Offboard => self
                .offboard
                .published_value()
                .ok_or(ScoreError::Missing("offboard"))?
                .fill_output(out),
            _ => out.is_in_state_other = true,
        }
        Ok(())
    }
    /// Update82dfcddc..cee0. Line prevention lasts a bounded number of held
    /// ground frames; other active collectors prevent zero without that limit.
    pub(super) fn decrement_meters(&mut self, dt: f32, update_hz: f32) -> Result<(), ScoreError> {
        let kind = self.kind()?;
        let (line_fraction, momentum_fraction) = match kind {
            Kind::Ground => {
                let c = self
                    .ground
                    .published_value()
                    .ok_or(ScoreError::Missing("ground"))?;
                (
                    c.line_decrement_fraction()?,
                    c.momentum_decrement_fraction()?,
                )
            }
            Kind::Air => {
                let c = self
                    .air
                    .published_value()
                    .ok_or(ScoreError::Missing("air"))?;
                (
                    c.line_decrement_fraction()?,
                    c.momentum_decrement_fraction()?,
                )
            }
            Kind::Offboard => {
                let c = self
                    .offboard
                    .published_value()
                    .ok_or(ScoreError::Missing("offboard"))?;
                (
                    c.line_decrement_fraction()?,
                    c.momentum_decrement_fraction()?,
                )
            }
            Kind::Grind => (
                scalar(&self.states, 0xb3c6_9aaf_221f_34f9)?,
                scalar(&self.states, 0x3918_2152_6c05_a7f1)?,
            ),
            Kind::Handplant => (
                scalar(&self.states, 0xad14_550e_ff39_20de)?,
                scalar(&self.states, 0x04a1_775c_b52a_8e4d)?,
            ),
            _ => (1.0, 1.0),
        };
        let max_time = scalar(&self.states, 0xbfda_1a50_7e2e_55a1)?;
        let in_sequence = self
            .holder
            .published_value()
            .ok_or(ScoreError::Missing("holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .sequence_mut()?
            .flags_word
            & 0x8000_0000
            != 0;
        // PC Skate 3 policy: an announced manual keeps its earned live score
        // and multiplier until release. Idle ground still uses the native cap.
        #[cfg(feature = "skate3-runtime")]
        let manual_holds_line = self.states.skate3_enabled() && kind == Kind::Ground
            && self.ground.published_value().is_some_and(|ground|
                ground.manual_tracker.in_state && ground.manual_scorable.is_beginning_announced);
        #[cfg(not(feature = "skate3-runtime"))]
        let manual_holds_line = false;
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        // fctidz followed by the low-word load, then an unsigned comparison.
        let max_frames = (max_time * update_hz) as i64 as u32;
        let prevent_zero = in_sequence
            && (manual_holds_line || kind != Kind::Ground
                || state.num_frames_ground_sequence_maintains_line < max_frames);
        let prevented = self
            .line_timer
            .published_value_mut()
            .ok_or(ScoreError::Missing("line"))?
            .decrement(MeterKind::Line, line_fraction, prevent_zero, dt)?;
        state.num_frames_ground_sequence_maintains_line = if prevented && kind == Kind::Ground && !manual_holds_line {
            state
                .num_frames_ground_sequence_maintains_line
                .wrapping_add(1)
        } else {
            0
        };
        self.momentum
            .published_value_mut()
            .ok_or(ScoreError::Missing("momentum"))?
            .base
            .decrement(MeterKind::Momentum, momentum_fraction, false, dt)?;
        Ok(())
    }
}
