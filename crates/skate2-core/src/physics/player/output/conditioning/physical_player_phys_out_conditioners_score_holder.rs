//! `ScoreHolder82df35c8`: retain the holder before its nested Sim Sequence
//! allocation, then perform Reset82dee248 against that same Sequence.
use super::{
    super::scoring::{AirPeriod, Scorables},
    sequence::{GlobalMetrics, Sequence},
};
use crate::attribute_runtime::{AttributeRuntimeVault};
use super::scoring_attributes::ScoringAttributes;
use std::sync::{Arc, Mutex};

pub(super) struct ScoreHolder {
    pub(super) output: Arc<()>,
    pub(super) sequence: Option<Box<Sequence>>,
    pub(super) general: ScoringAttributes,
    pub(super) session_score: f32,
    pub(super) line_score: f32,
    pub(super) sequence_score: f32,
    pub(super) banked_sequence_score: f32,
    pub(super) air_sequence_score: f32,
    pub(super) air_sequence_score_pending: f32,
    pub(super) trick_score: f32,
    pub(super) initial_sequence_score: i32,
    pub(super) tricks_begun: Scorables,
    pub(super) tricks_ended: Scorables,
    pub(super) tricks_converted_from: Scorables,
    pub(super) tricks_converted_to: Scorables,
    pub(super) tricks_cancelled: Scorables,
    pub(super) line_accumulator: [u8; super::SCORABLE_CAPACITY],
    pub(super) tricks_in_sequence: [u8; super::SCORABLE_CAPACITY],
    pub(super) trick_types_in_sequence: [u8; 14],
    pub(super) pending_air_period: AirPeriod,
    pub(super) air_sequence_reward_pending_frame_count: u32,
    pub(super) air_sequence_reward_pending: bool,
    /// TU3 ScoreHolder+7246: separate from the sketchy flag at+7245.
    #[cfg(feature = "skate3-runtime")]
    pub(super) skate3_clean_landing: bool,
    /// TU3 ScoreHolder+0x1C5D: the air output's locked trajectory (+452), copied by
    /// ScoreModule::Update each tick for the collectors.
    #[cfg(feature = "skate3-runtime")]
    pub(super) skate3_air_reckoning: bool,
}
impl ScoreHolder {
    /// TU3: the collectors leave the holder's trick score as it is while ScoreHolder+0x1C5D (the
    /// locked air trajectory) is set. Never in Skate 2 scoring.
    pub(super) fn trick_score_held(&self) -> bool {
        #[cfg(feature = "skate3-runtime")]
        {
            self.skate3_air_reckoning
        }
        #[cfg(not(feature = "skate3-runtime"))]
        {
            false
        }
    }
    /// Constructor prefix through `AirPeriodData` construction. Every remaining
    /// field is backed by the proven Sim zero-filled allocation until its store.
    pub(super) fn before_sequence(output: Arc<()>, vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            output,
            sequence: None,
            // Raw828b14b4..14cc class;82df35dc..35f8 default collection.
            general: ScoringAttributes::new(vault.instance(0x3492_15e2_e817_703c, 0xd7ed_bd36_2d7d_2152), 0x3492_15e2_e817_703c),
            session_score: 0.0,
            line_score: 0.0,
            sequence_score: 0.0,
            banked_sequence_score: 0.0,
            air_sequence_score: 0.0,
            air_sequence_score_pending: 0.0,
            trick_score: 0.0,
            initial_sequence_score: 0,
            tricks_begun: Scorables::new(),
            tricks_ended: Scorables::new(),
            tricks_converted_from: Scorables::new(),
            tricks_converted_to: Scorables::new(),
            tricks_cancelled: Scorables::new(),
            line_accumulator: [0; super::SCORABLE_CAPACITY],
            tricks_in_sequence: [0; super::SCORABLE_CAPACITY],
            trick_types_in_sequence: [0; 14],
            pending_air_period: AirPeriod::new(),
            air_sequence_reward_pending_frame_count: 0,
            air_sequence_reward_pending: false,
            #[cfg(feature = "skate3-runtime")]
            skate3_clean_landing: false,
            #[cfg(feature = "skate3-runtime")]
            skate3_air_reckoning: false,
        }
    }
    /// Called only after the outer allocation adapter admits the nested
    /// Sequence allocation. A null allocation must reach reset's missing-owner
    /// fault instead of constructing a substitute Sequence.
    pub(super) fn construct_sequence(&mut self, metrics: Option<Arc<Mutex<GlobalMetrics>>>) {
        self.sequence = Some(Box::new(Sequence::new_sim(self.output.clone(), metrics)));
    }
    pub(super) fn finish_constructor(&mut self) -> Result<(), MissingSequence> {
        self.session_score = 0.0;
        self.reset()
    }
    /// The five quicklists and session total intentionally survive Reset.
    pub(super) fn reset(&mut self) -> Result<(), MissingSequence> {
        self.sequence.as_mut().ok_or(MissingSequence)?.reset();
        self.line_accumulator.fill(0);
        self.tricks_in_sequence.fill(0);
        self.trick_types_in_sequence.fill(0);
        self.initial_sequence_score = 0;
        self.line_score = 0.0;
        self.sequence_score = 0.0;
        self.banked_sequence_score = 0.0;
        self.air_sequence_score = 0.0;
        self.air_sequence_score_pending = 0.0;
        self.trick_score = 0.0;
        self.pending_air_period.reset();
        self.air_sequence_reward_pending_frame_count = 0;
        self.air_sequence_reward_pending = false;
        #[cfg(feature = "skate3-runtime")]
        { self.skate3_clean_landing = false; }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MissingSequence;
