//! Typed scoring fields of PhysOutConditionerExternalInput (native size2784).
//! Nearby lists contain only the original count prefix; producers must retain
//! the native selection order and eight/seven-object capacities.
use super::{scoring_attributes::ScoreError, sequence::PatternItem};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

#[derive(Clone)]
pub(crate) struct NearbyVehicle {
    pub transform: SkeletonRootMatrix,
    pub length: f32,
    pub width: f32,
    pub unique_id: u32,
    pub kind: u64,
    pub colliding: bool,
}
#[derive(Clone)]
pub(crate) struct NearbyPedestrian {
    pub torso: SkeletonRootVector,
    pub legs: SkeletonRootVector,
    pub unique_id: u32,
    pub kind: u64,
    pub colliding: bool,
}
#[derive(Clone)]
pub(crate) struct NearbySkater {
    pub upper_body: SkeletonRootVector,
    pub lower_body: SkeletonRootVector,
    pub unique_id: u32,
}
/// Another online player's skater as the local skater's scoring collects it: its game slot and
/// the world positions of its head, hips and feet, from the pose its replay frames show (PC
/// port: the original simulated every player's skater and read its skeleton output).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RemoteScoringSkater {
    pub slot: u8,
    pub head: [f32; 3],
    pub hips: [f32; 3],
    pub left_foot: [f32; 3],
    pub right_foot: [f32; 3],
}
impl RemoteScoringSkater {
    /// `CollectNearbyEntitiesForScoring`'s body points of a skater: the upper body halfway from
    /// the hips to the head, the lower body the hips' and feet's mean with the hips counted
    /// twice.
    #[must_use]
    pub fn nearby(&self, unique_id: u32) -> NearbySkater {
        NearbySkater {
            upper_body: SkeletonRootVector { xyz: std::array::from_fn(|i| (self.head[i] + self.hips[i]) * 0.5), w: None },
            lower_body: SkeletonRootVector {
                xyz: std::array::from_fn(|i| ((self.hips[i] + self.hips[i]) + self.left_foot[i] + self.right_foot[i]) * 0.25),
                w: None,
            },
            unique_id,
        }
    }
}
#[derive(Clone)]
pub(crate) struct NearbyDmo {
    pub transform: SkeletonRootMatrix,
    pub minimum: SkeletonRootVector,
    pub maximum: SkeletonRootVector,
    pub kind: u64,
    pub unique_id: u32,
    pub colliding: bool,
    pub skater_is_grinding: bool,
}
#[derive(Clone)]
pub(crate) struct StaticObject {
    pub position: SkeletonRootVector,
    pub normal: SkeletonRootVector,
    pub kind: u64,
    pub valid: bool,
}
#[derive(Clone)]
pub(crate) struct TrickPattern {
    pub length: i32,
    pub items: [PatternItem; 4],
}
#[derive(Clone)]
pub(crate) struct ScoringExternalInput {
    pub activate_patterns: [bool; 3],
    pub deactivate_patterns: [bool; 3],
    pub patterns: [TrickPattern; 3],
    pub enabled: [bool; 14],
    pub vehicles: Vec<NearbyVehicle>,
    pub pedestrians: Vec<NearbyPedestrian>,
    pub skaters: Vec<NearbySkater>,
    pub dmos: Vec<NearbyDmo>,
    pub static_objects: [StaticObject; 2],
    pub tick: u32,
    pub visual_tick: u32,
    pub last_frame_number_written: u32,
    pub score_over_skaters: bool,
    pub reset_player_damage: bool,
    pub hall_of_meat_effect: bool,
    pub reset_scoring: bool,
    pub multiplier_active: bool,
    pub crouching: bool,
    pub reset_path_recording: bool,
    pub recording_ai_path: bool,
    pub manuals_are_connectors: bool,
    pub reset_sequence: bool,
}
impl TrickPattern {
    /// `cTrickPatternManager::GetTrickPattern`82da19f8's nine words: the length and
    /// four (`mMatchType`, `mValue`) items.
    pub(crate) fn from_words(length: i32, items: [(i32, i32); 4]) -> Self {
        Self { length, items: items.map(|(match_type, value)| PatternItem { match_type, value }) }
    }
}
impl ScoringExternalInput {
    /// Selected World.UpdateConditioners827af468 with no active challenge.
    /// Reset827ab250 clears flags/counts; ChallengeScoringManager82da2178
    /// then enables all fourteen kinds. No pattern has been activated.
    pub(crate) fn selected_no_challenge(
        tick: u32,
        visual_tick: u32,
        hall_of_meat_effect: bool,
    ) -> Self {
        Self {
            activate_patterns: [false; 3],
            deactivate_patterns: [false; 3],
            patterns: std::array::from_fn(|_| TrickPattern {
                length: 0,
                items: std::array::from_fn(|_| PatternItem {
                    match_type: 0,
                    value: 0,
                }),
            }),
            enabled: [true; 14],
            vehicles: Vec::with_capacity(8),
            pedestrians: Vec::with_capacity(8),
            skaters: Vec::with_capacity(7),
            dmos: Vec::with_capacity(8),
            static_objects: std::array::from_fn(|_| StaticObject {
                position: SkeletonRootVector::known([0.; 4]),
                normal: SkeletonRootVector::known([0.; 4]),
                kind: 0,
                valid: false,
            }),
            tick,
            visual_tick,
            // Selected startup admits no replay recorder/DataTracker entries.
            // DataMan.GetLastFrameNumberWritten82992138 returns0 when empty.
            last_frame_number_written: 0,
            score_over_skaters: false,
            reset_player_damage: false,
            hall_of_meat_effect,
            reset_scoring: false,
            multiplier_active: true,
            crouching: false,
            reset_path_recording: false,
            recording_ai_path: false,
            manuals_are_connectors: false,
            reset_sequence: false,
        }
    }
    pub(super) fn validate(&self) -> Result<(), ScoreError> {
        if self.vehicles.len() > 8
            || self.pedestrians.len() > 8
            || self.skaters.len() > 7
            || self.dmos.len() > 8
        {
            return Err(ScoreError::Invalid("scoring nearby count"));
        }
        for (enabled, pattern) in self.activate_patterns.iter().zip(&self.patterns) {
            if *enabled && !(0..=4).contains(&pattern.length) {
                return Err(ScoreError::Invalid("scoring pattern length"));
            }
        }
        Ok(())
    }
}
