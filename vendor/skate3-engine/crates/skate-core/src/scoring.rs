//! Native ScoreHolder accounting, TU3 82DA6198/6260/6408/6468/6538.
//!
//! Collectors own recognition and reward calculation. This ledger deliberately
//! accepts their rewards rather than substituting a table of guessed points.
//! Rendering and networking consume snapshots; neither advances accounting.

pub const SCORABLE_COUNT: usize = 332;
pub const SCORE_TYPE_COUNT: usize = 14;
pub mod carrier;
pub mod catalog;
pub mod conversions;
pub mod session;
pub mod timer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scorable {
    pub id: usize,
    /// Fixed metadata table +12. Distinct from the display type at +16.
    pub class: u32,
    pub score_type: usize,
}

impl Scorable {
    pub fn valid(self) -> bool {
        self.id < SCORABLE_COUNT && self.score_type < SCORE_TYPE_COUNT
    }
    /// CalcPointPenalty82DA5D18 excludes metric class5 and class6.
    pub fn repetition_applies(self) -> bool {
        self.valid() && self.class != 5 && self.class != 6
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub completed_lines: f32,    //24
    pub line: f32,               //28
    pub accumulated: f32,        //32
    pub last_reward: f32,        //36
    pub general_pending: f32,    //40
    pub fingerflip_pending: f32, //44
    pub grind_reward: f32,       //4776
}

#[derive(Clone, Debug)]
pub struct ScoreHolder {
    pub snapshot: Snapshot,
    repetitions: [i8; SCORABLE_COUNT],      //4096
    sequence_history: [i8; SCORABLE_COUNT], //4428
    type_history: [i8; SCORE_TYPE_COUNT],   //4760
    pending_sequence: bool,                 //7260
    suppressed: bool,                       //7261 (physical output, not difficulty)
}

impl Default for ScoreHolder {
    fn default() -> Self {
        Self {
            snapshot: Snapshot::default(),
            repetitions: [0; SCORABLE_COUNT],
            sequence_history: [0; SCORABLE_COUNT],
            type_history: [0; SCORE_TYPE_COUNT],
            pending_sequence: false,
            suppressed: false,
        }
    }
}

impl ScoreHolder {
    pub fn has_pending_sequence(&self) -> bool {
        self.pending_sequence
    }
    /// Non-air collectors use6198 then add directly to accumulator32.
    pub fn credit_trick(&mut self, scorable: Scorable, reward: f32) {
        if !scorable.valid() || self.suppressed {
            return;
        }
        self.count_trick(scorable);
        self.snapshot.accumulated += reward;
    }
    fn count_trick(&mut self, scorable: Scorable) {
        self.repetitions[scorable.id] = self.repetitions[scorable.id].saturating_add(1);
        if scorable.score_type != 0 {
            self.sequence_history[scorable.id] =
                self.sequence_history[scorable.id].saturating_add(1);
            self.type_history[scorable.score_type] =
                self.type_history[scorable.score_type].saturating_add(1);
        }
    }
    pub fn set_suppressed(&mut self, suppressed: bool) {
        self.suppressed = suppressed;
    }
    pub fn repetition_count(&self, scorable: Scorable) -> Option<i8> {
        scorable.valid().then(|| self.repetitions[scorable.id])
    }
    /// EndAirTrick82DA6260. Invalid IDs do not touch either history or rewards.
    pub fn end_trick(&mut self, scorable: Scorable, reward: f32) {
        if !scorable.valid() || self.suppressed {
            return;
        }
        self.count_trick(scorable);
        let s = &mut self.snapshot;
        if scorable.class != 5 {
            s.general_pending += s.fingerflip_pending;
            s.fingerflip_pending = 0.0;
        }
        if scorable.class == 3 {
            s.fingerflip_pending += reward;
        } else {
            s.general_pending += reward;
        }
    }
    /// Collector Exit publishes a pending sequence even for a cancelled exit.
    pub fn finish_collector(&mut self) {
        self.pending_sequence = true;
    }
    pub fn cancel_pending(&mut self) {
        if self.pending_sequence {
            self.snapshot.general_pending = 0.0;
            self.snapshot.fingerflip_pending = 0.0;
        }
    }
    /// RewaredAirSequence82DA6468. Repeated calls cannot bank twice.
    pub fn reward_sequence(&mut self, multiplier: f32) {
        if !self.pending_sequence {
            return;
        }
        let s = &mut self.snapshot;
        s.accumulated += (s.general_pending + s.fingerflip_pending) * multiplier;
        s.general_pending = 0.0;
        s.fingerflip_pending = 0.0;
        self.pending_sequence = false;
    }
    /// PublishAndResetAirSequence82DA6538 receives the final reward from the
    /// module; it does not itself calculate the multiplier or bail penalty.
    pub fn publish(&mut self, reward: f32, add_to_line: bool) {
        let s = &mut self.snapshot;
        if add_to_line {
            s.line += reward;
        }
        s.last_reward = reward;
        s.accumulated = 0.0;
        s.general_pending = 0.0;
        s.fingerflip_pending = 0.0;
        self.clear_sequence_history();
    }
    pub fn clear_sequence_history(&mut self) {
        self.sequence_history.fill(0);
        self.type_history.fill(0);
        self.snapshot.grind_reward = 0.0;
    }
    /// Module82DA3B38 banks the line when its timer expires or reset is requested.
    pub fn finish_line(&mut self) {
        self.bank_line(true);
    }
    /// A zero line timer banks the current line even while a collector is
    /// active, but 82DA3B38 retains repetition in that case.
    pub fn bank_line(&mut self, clear_repetition: bool) {
        self.snapshot.completed_lines += self.snapshot.line;
        self.snapshot.line = 0.0;
        if clear_repetition {
            self.repetitions.fill(0);
        }
    }
    /// Native Reset82DA5C78 retains lifetime completed-lines (+24).
    pub fn reset(&mut self) {
        let completed_lines = self.snapshot.completed_lines;
        *self = Self::default();
        self.snapshot.completed_lines = completed_lines;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FLIP: Scorable = Scorable {
        id: 96,
        class: 3,
        score_type: 2,
    };
    #[test]
    fn collector_bank_is_single_use_and_publication_retains_repetition() {
        let mut h = ScoreHolder::default();
        h.end_trick(FLIP, 20.0);
        h.finish_collector();
        h.reward_sequence(1.1);
        h.reward_sequence(1.1);
        assert_eq!(h.snapshot.accumulated, 22.0);
        h.publish(22.0, true);
        assert_eq!(h.snapshot.line, 22.0);
        assert_eq!(h.repetition_count(FLIP), Some(1));
        h.finish_line();
        assert_eq!(h.snapshot.completed_lines, 22.0);
        assert_eq!(h.repetition_count(FLIP), Some(0));
    }
    #[test]
    fn metric_preserves_flip_bucket_other_classes_flush_it() {
        let mut h = ScoreHolder::default();
        h.end_trick(FLIP, 20.0);
        h.end_trick(
            Scorable {
                id: 129,
                class: 5,
                score_type: 0,
            },
            4.0,
        );
        assert_eq!(
            (h.snapshot.general_pending, h.snapshot.fingerflip_pending),
            (4.0, 20.0)
        );
        h.end_trick(
            Scorable {
                id: 128,
                class: 4,
                score_type: 1,
            },
            2.0,
        );
        assert_eq!(
            (h.snapshot.general_pending, h.snapshot.fingerflip_pending),
            (26.0, 0.0)
        );
    }
    #[test]
    fn cancellation_after_collector_exit_prevents_pending_reward() {
        let mut h = ScoreHolder::default();
        h.end_trick(FLIP, 20.0);
        h.finish_collector();
        h.cancel_pending();
        h.reward_sequence(1.1);
        assert_eq!(h.snapshot.accumulated, 0.0);
        h.set_suppressed(true);
        h.end_trick(FLIP, 20.0);
        assert_eq!(h.repetition_count(FLIP), Some(1));
    }
}
