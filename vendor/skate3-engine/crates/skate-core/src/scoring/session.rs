//! Module publication and line settlement, TU3 82DA37B0/82DA3B38.
use super::{
    ScoreHolder,
    timer::{ComboTimer, PointTimer},
};

#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub combo_capacity: f32,
    pub combo_levels: [(f32, f32); 3],
    pub combo_refresh_threshold: f32,
    pub line_capacity: f32,
    pub bail_factor: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Session {
    pub holder: ScoreHolder,
    pub combo: ComboTimer,
    pub line: PointTimer,
}

impl Session {
    /// The collector owns the decision to publish. Capture the multiplier
    /// before crediting timers: a threshold reached by this reward affects
    /// the next publication, not this publication's score (f29 at 82DA37B0).
    pub fn publish_sequence(
        &mut self,
        rules: &Rules,
        landing_factor: f32,
        penalized: bool,
        multiplier_enabled: bool,
    ) -> f32 {
        self.holder.reward_sequence(landing_factor);
        let snapshot = &self.holder.snapshot;
        let mut raw =
            (snapshot.fingerflip_pending + snapshot.general_pending) + snapshot.accumulated;
        if penalized && rules.bail_factor < 1.0 {
            raw *= rules.bail_factor;
        }
        let multiplier = if multiplier_enabled {
            self.combo.multiplier
        } else {
            1.0
        };
        if multiplier_enabled {
            self.combo.credit(
                raw,
                rules.combo_capacity,
                rules.combo_levels,
                rules.combo_refresh_threshold,
            );
            if self.combo.multiplier > f32::from_bits(0x3f8147ae) {
                self.line.credit(raw, rules.line_capacity);
            }
        }
        let reward = multiplier * raw;
        self.holder
            .publish(reward, multiplier_enabled && self.line.points > 0.0);
        reward
    }

    /// Reset/bail/output requests or the expiry edge reset line time and the
    /// multiplier. A merely empty line timer does not reset the multiplier.
    pub fn settle_line(&mut self, reset_requested: bool, collector_active: bool) {
        if reset_requested || self.line.expired {
            self.line = PointTimer::default();
            self.combo.multiplier = 1.0;
            self.holder.finish_line();
        } else if self.line.points <= 0.0 {
            self.holder.bank_line(!collector_active);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::Scorable;
    #[test]
    fn newly_crossed_threshold_applies_to_the_following_publication() {
        let rules = Rules {
            combo_capacity: 801.0,
            combo_levels: [(50.0, 1.5), (450.0, 2.0), (800.0, 3.0)],
            combo_refresh_threshold: 799.0,
            line_capacity: 400.0,
            bail_factor: 0.0,
        };
        let mut session = Session::default();
        let trick = Scorable {
            id: 96,
            class: 3,
            score_type: 2,
        };
        session.holder.end_trick(trick, 50.0);
        session.holder.finish_collector();
        assert_eq!(session.publish_sequence(&rules, 1.0, false, true), 50.0);
        assert_eq!(session.combo.multiplier, 1.5);
        session.holder.end_trick(trick, 10.0);
        session.holder.finish_collector();
        assert_eq!(session.publish_sequence(&rules, 1.0, false, true), 15.0);
        assert_eq!(session.holder.snapshot.line, 65.0);
        session.line.points = 0.0;
        session.settle_line(false, true);
        assert_eq!(session.holder.repetition_count(trick), Some(2));
        session.settle_line(false, false);
        assert_eq!(session.holder.repetition_count(trick), Some(0));
    }
}
