//! TimedArcLengthScorable::Update82df2130. Extend the retained native tracker.
use super::{
    collector_base::TimedArcLength,
    scoring_attributes::{ScoreError, curve},
};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_length},
    skeleton_root_transforms::SkeletonRootVector,
};

/// How a timed scorable measures the board's travel each tick.
#[derive(Clone, Copy)]
pub(super) enum ArcMeasure {
    /// Skate 2: the step's length, counted past the cvar's minimum.
    Skate2 { minimum_distance: f32 },
    /// TU3 TimedArcLengthScorable::Update 82DB0588: the step's length, or for a manual its
    /// length along the board's velocity direction (reckoning +160), counted past 0.005 m.
    Skate3 { along: Option<SkeletonRootVector> },
}

impl TimedArcLength {
    pub(super) fn update(
        &mut self,
        in_state: bool,
        eligible: bool,
        point_rate: f32,
        id: i32,
        position: SkeletonRootVector,
        dt: f32,
        measure: ArcMeasure,
    ) -> Result<(), ScoreError> {
        self.just_entered_state = false;
        self.just_exited_state = false;
        if in_state {
            // This sample is taken BEFORE entering resets the distance.
            let previous_distance = self.total_distance;
            if !self.in_state {
                self.total_distance = 0.0;
                self.total_time = 0.0;
                self.in_state = true;
                self.just_entered_state = true;
            } else if eligible {
                let last = self
                    .last_position
                    .ok_or(ScoreError::Missing("timed arc last position"))?;
                let delta = [
                    position.xyz[0] - last.xyz[0],
                    position.xyz[1] - last.xyz[1],
                    position.xyz[2] - last.xyz[2],
                    0.0,
                ];
                let (distance, minimum_distance) = match measure {
                    ArcMeasure::Skate2 { minimum_distance } => (
                        try_length(delta, &mut HostReckoningNumeric).map_err(ScoreError::Numeric)?,
                        minimum_distance,
                    ),
                    ArcMeasure::Skate3 { along: Some(direction) } => (
                        HostReckoningNumeric
                            .dot_vector(SkeletonRootVector::known(delta), direction)
                            .map_err(ScoreError::Numeric)?,
                        f32::from_bits(0x3ba3_d70a),
                    ),
                    ArcMeasure::Skate3 { along: None } => (
                        try_length(delta, &mut HostReckoningNumeric).map_err(ScoreError::Numeric)?,
                        f32::from_bits(0x3ba3_d70a),
                    ),
                };
                if distance > minimum_distance {
                    self.total_distance += distance;
                }
                self.total_time = dt + self.total_time;
            }
            self.last_position = Some(position);
            if eligible {
                let graph = &self.distance_scorer;
                let before = curve(&graph.instance, graph.graph_index * 80, previous_distance)?;
                let change =
                    curve(&graph.instance, graph.graph_index * 80, self.total_distance)? - before;
                let graph = &self.time_scorer;
                let time_score = curve(&graph.instance, graph.graph_index * 80, self.total_time)?;
                // Raw82df22f8 fsel(-change,0,change), followed by TWO fmadds.
                let distance_score = if -change >= 0.0 { 0.0 } else { change };
                self.score = dt
                    .mul_add(time_score, distance_score)
                    .mul_add(point_rate, self.score);
            }
        } else {
            if self.in_state {
                self.just_exited_state = true;
                self.score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("timed arc score holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?
                    .end_trick(id, self.score);
            }
            self.score = 0.0;
        }
        self.in_state = in_state;
        Ok(())
    }
}
