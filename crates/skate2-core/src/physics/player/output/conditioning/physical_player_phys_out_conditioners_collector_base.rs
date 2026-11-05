//! Nursary82df86f8 and non-cleared scoring tracker allocations.
use super::{
    super::{SkeletonRootMatrix, SkeletonRootVector, trick::AnimOutTrickData},
    score_holder::ScoreHolder,
    sequence::GlobalMetrics,
};
use crate::attribute_runtime::{AttributeRuntimeVault};
use super::scoring_attributes::ScoringAttributes;
use std::sync::{Arc, Mutex};

pub(super) struct CollectorBase {
    pub(super) output: Arc<()>,
    pub(super) global_metrics: Option<Arc<Mutex<GlobalMetrics>>>,
    pub(super) score_holder: Option<Arc<Mutex<ScoreHolder>>>,
    pub(super) states: ScoringAttributes,
    pub(super) general: ScoringAttributes,
    pub(super) raw_scorable_id: i32,
    pub(super) filtered_scorable_id: i32,
    pub(super) animation_trick: AnimOutTrickData,
}
impl CollectorBase {
    pub(super) fn new(
        output: Arc<()>,
        global_metrics: Option<Arc<Mutex<GlobalMetrics>>>,
        score_holder: Option<Arc<Mutex<ScoreHolder>>>,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Self {
        Self {
            output,
            global_metrics,
            score_holder,
            states: ScoringAttributes::new(vault.instance(0x546c_36b6_5603_8e04, 0xd7ed_bd36_2d7d_2152), 0x546c_36b6_5603_8e04),
            general: ScoringAttributes::new(vault.instance(0x3492_15e2_e817_703c, 0xd7ed_bd36_2d7d_2152), 0x3492_15e2_e817_703c),
            raw_scorable_id: -1,
            filtered_scorable_id: -1,
            animation_trick: AnimOutTrickData::reset_values(),
        }
    }
    /// Shared reset leaves the prior animation strings and tweak values intact.
    /// `OtherCollector` table8236c3ac routes both Reset and Enter here82deb5e8.
    pub(super) fn reset_analysis(&mut self) {
        self.animation_trick.bits = 0;
        self.raw_scorable_id = -1;
        self.filtered_scorable_id = -1;
    }
}

/// A retained generated-layout address. Clone retains the exact collection and
/// cached layout identity; a null instance selects native `DefaultDataArea`.
/// Construction forms this address without reading any graph samples.
pub(super) struct GraphAddress {
    pub(super) instance: ScoringAttributes,
    pub(super) graph_index: usize,
}
pub(super) struct TimedArcLength {
    pub(super) output: Arc<()>,
    pub(super) score_holder: Option<Arc<Mutex<ScoreHolder>>>,
    pub(super) last_position: Option<SkeletonRootVector>,
    pub(super) total_time: f32,
    pub(super) total_distance: f32,
    pub(super) score: f32,
    pub(super) distance_scorer: GraphAddress,
    pub(super) time_scorer: GraphAddress,
    pub(super) preserved_low_flags: Option<u32>,
    pub(super) just_exited_state: bool,
    pub(super) just_entered_state: bool,
    pub(super) in_state: bool,
}
impl TimedArcLength {
    /// Constructor call sites82df9490/82df9a78 do not initialize last position
    /// or the low29 bits in `PhysicsScoring` memory. Both graph addresses preserve
    /// the same cached layout pointer loaded at82df9514.
    pub(super) fn new(base: &CollectorBase, distance_index: usize, time_index: usize) -> Self {
        let distance_scorer = GraphAddress {
            instance: base.states.clone(),
            graph_index: distance_index,
        };
        let time_scorer = GraphAddress {
            instance: base.states.clone(),
            graph_index: time_index,
        };
        Self {
            output: base.output.clone(),
            score_holder: base.score_holder.clone(),
            last_position: None,
            total_time: 0.0,
            total_distance: 0.0,
            score: 0.0,
            distance_scorer,
            time_scorer,
            preserved_low_flags: None,
            just_exited_state: false,
            just_entered_state: false,
            in_state: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.total_distance = 0.0;
        self.total_time = 0.0;
        self.score = 0.0;
        self.just_exited_state = false;
        self.just_entered_state = false;
        self.in_state = false;
    }
}

pub(super) struct RotationTracker {
    pub(super) last_frame_matrix: Option<SkeletonRootMatrix>,
    pub(super) accumulated_angle_radians: f32,
    pub(super) rotation_type: i32,
}
impl RotationTracker {
    pub(super) fn new_noncleared(rotation_type: i32) -> Self {
        Self {
            last_frame_matrix: None,
            accumulated_angle_radians: 0.0,
            rotation_type,
        }
    }
    pub(super) fn reset(&mut self, matrix: SkeletonRootMatrix) {
        self.last_frame_matrix = Some(matrix);
        self.accumulated_angle_radians = 0.0;
    }
}

pub(super) struct ScoringMeter {
    pub(super) general: ScoringAttributes,
    pub(super) output: Arc<()>,
    pub(super) points: f32,
    pub(super) just_reached_zero: bool,
}
impl ScoringMeter {
    pub(super) fn new(output: Arc<()>, vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            general: ScoringAttributes::new(vault.instance(0x3492_15e2_e817_703c, 0xd7ed_bd36_2d7d_2152), 0x3492_15e2_e817_703c),
            output,
            points: 0.0,
            just_reached_zero: false,
        }
    }
    /// `LineTimer` inherits this Reset82dea238.
    pub(super) fn reset(&mut self) {
        self.just_reached_zero = false;
        self.points = 0.0;
    }
}
pub(super) struct MomentumMeter {
    pub(super) base: ScoringMeter,
    pub(super) multiplier: Option<f32>,
}
impl MomentumMeter {
    /// Constructor writes only the base; derived multiplier first becomes1
    /// through Reset82dea460 in the enclosing `ScoreModule` constructor.
    pub(super) fn new(output: Arc<()>, vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            base: ScoringMeter::new(output, vault),
            multiplier: None,
        }
    }
    pub(super) fn reset(&mut self) {
        self.base.reset();
        self.multiplier = Some(1.0);
    }
}
