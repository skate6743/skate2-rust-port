//! AirCollector spin82df4e60, body-flip82df5188 and trajectory82df5340.
use super::{
    super::scoring::Scoring2,
    air_collector::AirCollector,
    scoring_attributes::{ScoreError, curve, scalar},
    scoring_catalog::info,
};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, ReckoningVector, try_length},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
const DEGREES: f32 = f32::from_bits(0x4265_2ee1);

impl AirCollector {
    fn skate2_spin_angle(&mut self, player: SkeletonRootMatrix, board: SkeletonRootMatrix) -> Result<f32, ScoreError> {
        let active_trick = self.air_scorable.valid_id().is_some();
        if active_trick == self.accumulating_both_board_and_player_spin {
            self.player_spin_tracker.update(player)?;
            // Raw82df4f44/5020: fused accumulation when switching modes.
            self.banked_spin_value_degrees = self
                .player_spin_tracker
                .accumulated_angle_radians
                .mul_add(DEGREES, self.banked_spin_value_degrees);
            self.player_spin_tracker.reset(player);
            self.board_spin_tracker.reset(board);
            self.accumulating_both_board_and_player_spin = !active_trick;
        } else if active_trick {
            self.board_spin_tracker.reset(board);
            self.player_spin_tracker.update(player)?;
        } else {
            self.player_spin_tracker.update(player)?;
            self.board_spin_tracker.update(board)?;
        }
        let board_degrees = self.board_spin_tracker.accumulated_angle_radians * DEGREES;
        Ok(if self.accumulating_both_board_and_player_spin {
            self.banked_spin_value_degrees + board_degrees
        } else {
            self.player_spin_tracker
                .accumulated_angle_radians
                .mul_add(DEGREES, self.banked_spin_value_degrees)
                + board_degrees
        })
    }

    pub(super) fn update_spin_metric(
        &mut self,
        player: SkeletonRootMatrix,
        board: SkeletonRootMatrix,
        up: SkeletonRootVector,
    ) -> Result<f32, ScoreError> {
        #[cfg(feature = "skate3-runtime")]
        let source = self.base.states.skate3_enabled();
        #[cfg(not(feature = "skate3-runtime"))]
        let source = false;
        let angle;
        #[cfg(feature = "skate3-runtime")]
        if source {
            let basis = |m: SkeletonRootMatrix| std::array::from_fn(|i|m.columns()[i].xyz);
            let rotation = self.skate3_rotation.get_or_insert_with(|| {
                let mut rotation = crate::skate3_native::scoring_runtime::rotation::Rotation::default();
                rotation.reset(basis(self.player_spin_tracker.last_frame_matrix.expect("air reset")),
                    basis(self.board_spin_tracker.last_frame_matrix.expect("air reset")));
                rotation
            });
            angle = rotation.update(basis(player), basis(board), up.xyz,
                self.air_scorable.valid_id().is_some()) * DEGREES;
        } else { angle = self.skate2_spin_angle(player, board)?; }
        #[cfg(not(feature = "skate3-runtime"))]
        { let _ = (source, up); angle = self.skate2_spin_angle(player, board)?; }
        let allowance = scalar(&self.base.states, 0x25b1_1e40_5c14_0e61)?;
        let turns = (angle.abs() + allowance) * f32::from_bits(0x3bb6_0b61);
        let count = HostReckoningNumeric
            .truncate_i32(turns)
            .map_err(ScoreError::Numeric)?;
        self.player_spin180_count = if angle < 0.0 {
            count.wrapping_neg()
        } else {
            count
        };
        let points = curve(
            &self.base.states,
            880,
            (self.player_spin180_count.wrapping_mul(180) as f32).abs(),
        )?;
        // TU3 82DA8E88 scales by the in-air scalar, then by the repeat penalty.
        self.player_spin_points = if source {
            (points * self.in_air_point_scalar) * self.most_recent_trick_point_scalar
        } else {
            points * (self.in_air_point_scalar * self.most_recent_trick_point_scalar)
        };
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("air holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        // TU3 UpdateSpinMetric 82DA8E90: held while ScoreHolder+0x1C5D is set.
        if !holder.trick_score_held() {
            holder.trick_score += self.player_spin_points;
        }
        Ok(self.player_spin_points)
    }

    pub(super) fn update_flip_metric(
        &mut self,
        flipping: bool,
        front_flipping: bool,
        out: &mut Scoring2,
    ) -> Result<f32, ScoreError> {
        if flipping {
            if self.air_scorable.is_beginning_announced
                && self
                    .air_scorable
                    .valid_id()
                    .and_then(info)
                    .is_some_and(|i| i.category == 2)
            {
                self.player_num_flips = if front_flipping { 1 } else { -1 };
            }
            if !self.has_flip_attempt_been_reported {
                self.has_flip_attempt_been_reported = true;
                out.is_attempting_a_body_flip = true;
            }
        }
        let factor = self.in_air_point_scalar * self.most_recent_trick_point_scalar;
        let points = scalar(&self.base.states, 0x6944_10fc_8d57_e794)?;
        self.player_flip_points = (self.player_num_flips.wrapping_abs() as f32 * points) * factor;
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("air holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        // TU3 UpdateFlipMetric 82DA8FD8: held while ScoreHolder+0x1C5D is set.
        if !holder.trick_score_held() {
            holder.trick_score = self.player_flip_points + holder.trick_score;
        }
        Ok(self.player_flip_points)
    }

    pub(super) fn update_trajectory_metrics(
        &mut self,
        deck: SkeletonRootVector,
    ) -> Result<f32, ScoreError> {
        let displacement = deck.zip(self.takeoff_deck_position, |a, b| a - b);
        let mut numeric = HostReckoningNumeric;
        self.total_height_gain = numeric
            .dot_vector(displacement, self.takeoff_normal)
            .map_err(ScoreError::Numeric)?;
        let mut horizontal = displacement;
        horizontal.xyz[1] = 0.0;
        self.horizontal_distance =
            try_length(horizontal.operand(), &mut numeric).map_err(ScoreError::Numeric)?;
        if deck.xyz[1] - self.highest_altitude >= 0.0 {
            self.highest_altitude = deck.xyz[1];
        }
        self.height_to_peak = self.highest_altitude - self.takeoff_altitude;
        let factor = self.most_recent_trick_point_scalar * self.in_air_point_scalar;
        self.horizontal_distance_points =
            curve(&self.base.states, 960, self.horizontal_distance)? * factor;
        self.height_to_peak_points = curve(&self.base.states, 1040, self.height_to_peak)? * factor;
        self.total_height_gain_points =
            curve(&self.base.states, 800, self.total_height_gain)? * factor;
        let mut holder = self
            .base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("air holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        // TU3 UpdateTrajectoryMetrics 82DA9160/917C: both held while ScoreHolder+0x1C5D is set.
        if !holder.trick_score_held() {
            holder.trick_score += self.horizontal_distance_points;
            holder.trick_score = self.height_to_peak_points + holder.trick_score;
        }
        // Source includes normal-relative height in its return value but does
        // not add it to the holder here (82df54dc..551c).
        Ok(
            (self.total_height_gain_points + self.horizontal_distance_points)
                + self.height_to_peak_points,
        )
    }
}
