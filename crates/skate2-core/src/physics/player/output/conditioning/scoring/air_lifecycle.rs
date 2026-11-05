//! AirCollector Enter82df49d8 / Exit82dfb898. Extend the existing reset owner.
use super::{
    super::{
        PhysicalPlayerPhysOut,
        scoring::{AirPeriod, ContextGroups, Scoring2},
        simple::GroundRemainder,
    },
    air_collector::{AirCollector, AirGroundData},
    scoring_attributes::{ScoreError, scalar},
    scoring_catalog::info,
    scoring_database::TrickDatabase,
};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    skeleton_root_transforms::SkeletonRootVector,
};

impl AirCollector {
    pub(super) fn enter(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        out: &mut Scoring2,
        system_velocity: SkeletonRootVector,
        system_up: SkeletonRootVector,
        switched: bool,
        fakie: bool,
    ) -> Result<(), ScoreError> {
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("air holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .reward_air(1.0, out);
        self.base.reset_analysis();
        self.reset(output).map_err(ScoreError::AirReset)?;
        let recent_type = {
            let holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("air holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            holder
                .sequence
                .as_ref()
                .ok_or(ScoreError::Missing("air sequence"))?
                .most_recent()
                .and_then(|s| s.valid_id())
                .and_then(info)
                .map(|i| i.trick_type)
        };
        self.started_from_manual = recent_type == Some(8);
        self.started_from_grind = recent_type == Some(5);
        if self.started_from_manual {
            self.in_air_point_scalar =
                scalar(&self.base.states, 0x123e_054e_8895_cafe)? * self.in_air_point_scalar;
        }
        if self.started_from_grind {
            self.in_air_point_scalar *= scalar(&self.base.states, 0x63e_61d37_475e_6ab9)?;
        }
        let mut numeric = HostReckoningNumeric;
        let up_length_squared = numeric
            .dot_vector(system_up, system_up)
            .map_err(ScoreError::Numeric)?;
        if !(up_length_squared > f32::from_bits(0x3586_37bd)) {
            return Err(ScoreError::Invalid("scoring component vector length"));
        }
        let dot = numeric
            .dot_vector(system_velocity, system_up)
            .map_err(ScoreError::Numeric)?;
        let projection = dot / up_length_squared;
        let projected = system_up.map(|v| v * projection);
        let tangent = system_velocity.zip(projected, |a, b| a - b);
        let speed_squared = numeric
            .dot_vector(tangent, tangent)
            .map_err(ScoreError::Numeric)?;
        let threshold = scalar(&self.base.states, 0xab8e_acf7_2b29_1746)?;
        self.started_from_stationary = threshold * threshold > speed_squared;
        self.started_switched = switched;
        self.started_fakie = fakie;
        if self.started_from_stationary {
            self.in_air_point_scalar =
                scalar(&self.base.states, 0x9453_69eb_d1e9_a062)? * self.in_air_point_scalar;
        }
        if self.started_switched && self.started_fakie {
            self.started_switched = false;
            self.started_fakie = false;
        }
        if self.started_switched {
            self.in_air_point_scalar *= scalar(&self.base.states, 0xee42_ba52_a131_3fdb)?;
        }
        if self.started_fakie {
            self.in_air_point_scalar =
                scalar(&self.base.states, 0xac54_de5c_98b5_dfad)? * self.in_air_point_scalar;
        }
        out.just_took_off = true;
        self.initial_context = out.current_context_data.clone();
        let metrics = self
            .base
            .global_metrics
            .as_ref()
            .ok_or(ScoreError::Missing("air metrics"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        let count = if metrics.over_dmo_context_buffer_full {
            8
        } else {
            metrics.over_dmo_context_next_index as usize
        };
        let contexts = metrics
            .over_dmo_context
            .get(..count)
            .ok_or(ScoreError::Invalid("DMO context ring count"))?;
        for context in contexts {
            self.initial_context.context_sensitive_data[3].uid_data[0].add(context)?;
        }
        Ok(())
    }
    fn air_over_bonus_scale(
        &self,
        context: &ContextGroups,
        score_over_skaters: bool,
    ) -> Result<f32, ScoreError> {
        let mut factor = 1.0;
        for (group, key) in [
            (1, 0x2be5_5edc_393f_3d9b),
            (0, 0x16cb_3f5d_3d56_0676),
            (2, 0x3bd6_7fae_1e67_1f26),
            (3, 0xba0b_9725_b1f7_aba7),
        ] {
            if (group != 2 || score_over_skaters)
                && context.context_sensitive_data[group].uid_data[0].count != 0
            {
                let scale = scalar(&self.base.states, key)?;
                factor = if group == 1 { scale } else { scale * factor };
            }
        }
        Ok(factor)
    }
    pub(super) fn exit(
        &mut self,
        score_in_progress: bool,
        db: &TrickDatabase,
        ground: &GroundRemainder,
        wiped_out: bool,
        score_over_skaters: bool,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let landing = AirGroundData::from_ground(ground, Some(&self.takeoff_ground));
        if score_in_progress {
            self.end_current_trick(db)?;
            self.air_scorable.reset();
            let mut crossed = self.accumulated_context.clone();
            crossed.remove(&self.initial_context)?;
            crossed.remove(&out.current_context_data)?;
            let bonus = self.air_over_bonus_scale(&crossed, score_over_skaters)?;
            self.horizontal_distance_points *= bonus;
            self.height_to_peak_points *= bonus;
            self.total_height_gain_points *= bonus;
            self.player_spin_points *= bonus;
            self.player_flip_points *= bonus;
            {
                let mut holder = self
                    .base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("air holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?;
                for (id, points) in [
                    (129, self.horizontal_distance_points),
                    (130, self.height_to_peak_points),
                    (131, self.total_height_gain_points),
                    (132, self.player_spin_points),
                    (133, self.player_flip_points),
                ] {
                    holder.end_air_trick(id, points);
                }
                for (group, key) in [
                    (1, 0xf1f4_5142_0d10_d2c5),
                    (0, 0x67eb_ca6d_bb3e_e9f1),
                    (2, 0xf577_3780_23aa_daa2),
                    (3, 0xc5be_c054_7104_1171),
                ] {
                    if (group != 2 || score_over_skaters)
                        && crossed.context_sensitive_data[group].uid_data[0].count != 0
                    {
                        let factor = self.in_air_point_scalar * self.most_recent_trick_point_scalar;
                        let points = scalar(&self.base.states, key)?;
                        let reward = if group == 1 || group == 2 {
                            (factor * bonus) * points
                        } else {
                            (points * factor) * bonus
                        };
                        holder.end_air_trick(237, reward);
                    }
                }
            }
            for (index, gap) in self.gaps.iter_mut().enumerate() {
                gap.update(&landing, false, index != 0 || !landing.is_above_grass)?;
            }
            let score = self.calculate_gap_score()?;
            #[cfg(feature = "skate3-runtime")]
            if self.base.states.skate3_enabled() && crate::diagnostics::test_log::enabled() {
                crate::diagnostics::test_log::event("skate3_air_reward",format_args!(
                    "distance={} peak={} height={} spin180={} metrics={:?} gap_points={} air_factor={} repetition={} stationary={} from_manual={} from_grind={}",
                    self.horizontal_distance,self.height_to_peak,self.total_height_gain,self.player_spin180_count,
                    [self.horizontal_distance_points,self.height_to_peak_points,self.total_height_gain_points,self.player_spin_points,self.player_flip_points],
                    score,self.in_air_point_scalar,self.most_recent_trick_point_scalar,self.started_from_stationary,self.started_from_manual,self.started_from_grind));
            }
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("air holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .end_air_trick(237, score);
            self.update_thresholds(true, out)?;
        } else {
            for gap in &mut self.gaps {
                gap.update(&landing, false, false)?;
            }
            if self.air_scorable.valid_id().is_some() {
                self.base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("air holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?
                    .announce_cancelled(&mut self.air_scorable)?;
            }
        }
        let mut period = AirPeriod::new();
        self.fill_air_period(out, wiped_out, &mut period);
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("air holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        #[cfg(feature = "skate3-runtime")]
        { holder.skate3_clean_landing = false; }
        holder.air_sequence_reward_pending = true;
        holder.pending_air_period = period;
        holder.air_sequence_reward_pending_frame_count = 2;
        Ok(())
    }
}
