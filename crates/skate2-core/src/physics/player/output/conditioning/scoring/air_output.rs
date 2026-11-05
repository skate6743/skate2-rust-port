//! AirCollector FillAirPeriodData82deacb8 / thresholds82def4d0 / Fill82defaf0.
use super::{
    super::scoring::{AirPeriod, Scoring2},
    air_collector::AirCollector,
    scoring_attributes::ScoreError,
};
impl AirCollector {
    pub(super) fn update_thresholds(
        &self,
        complete: bool,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        // TU3 UpdateScoreThresholds 82DA992C: nothing while the trajectory is locked.
        #[cfg(feature = "skate3-runtime")]
        if self.skate3_locked {
            return Ok(());
        }
        let mut set = |id, value| {
            super::scoring_thresholds::update(
                &self.base.general,
                &mut out.big_trick_achieved_data,
                id,
                value,
            )
        };
        set(4, self.air_grab_time)?;
        set(5, self.air_frames as f32 * f32::from_bits(0x3c88_8889))?;
        set(6, self.horizontal_distance)?;
        set(7, self.height_to_peak)?;
        if complete {
            set(8, self.total_height_gain)?;
            set(9, -self.total_height_gain)?;
        }
        for (index, gap) in self.gaps.iter().enumerate() {
            set(
                11 + index,
                gap.accumulated_gap_distance + gap.current_gap_distance,
            )?;
        }
        Ok(())
    }
    pub(super) fn fill_air_period(&self, out: &Scoring2, wiped_out: bool, air: &mut AirPeriod) {
        air.grab_time = self.air_grab_time;
        air.height_to_peak = self.height_to_peak;
        air.height_to_peak_points = self.height_to_peak_points;
        air.horizontal_distance = self.horizontal_distance;
        air.horizontal_distance_points = self.horizontal_distance_points;
        air.in_air_frames = self.air_frames;
        air.initial_trick = self.first_scorable.clone();
        air.player_flip_points = self.player_flip_points;
        air.player_num_flips = self.player_num_flips;
        air.player_spin180_count = self.player_spin180_count;
        air.player_spin_points = self.player_spin_points;
        air.started_fakie = self.started_fakie;
        air.started_switched = self.started_switched;
        air.total_height_gain = self.total_height_gain;
        air.total_height_gain_points = self.total_height_gain_points;
        air.trick_count = self.trick_count;
        air.tweak_occured = self.tweak_occurred_in_air;
        air.grab_occured = self.grab_occurred_in_air;
        air.wiped_out = wiped_out;
        air.accumulated_context_data = self.accumulated_context.clone();
        air.initial_context_data = self.initial_context.clone();
        air.end_context_data = out.current_context_data.clone();
        air.big_trick_achievements = out.big_trick_achieved_data.clone();
        for (dst, gap) in air.air_gap_data.iter_mut().zip(&self.gaps) {
            dst.accumulated_gap_distance = gap.accumulated_gap_distance + gap.current_gap_distance;
            dst.current_gap_distance = gap.current_gap_distance;
            dst.maximum_gap_distance = if gap.maximum_gap_distance - gap.current_gap_distance >= 0.0
            {
                gap.maximum_gap_distance
            } else {
                gap.current_gap_distance
            };
        }
    }
    pub(super) fn fill_output(
        &self,
        out: &mut Scoring2,
        wiped_out: bool,
    ) -> Result<(), ScoreError> {
        out.is_in_state_air = true;
        self.update_thresholds(false, out)?;
        // Retain fields FillAirPeriodData does not write.
        let mut air = out.current_air_data.clone();
        self.fill_air_period(out, wiped_out, &mut air);
        out.current_air_data = air;
        Ok(())
    }
}
