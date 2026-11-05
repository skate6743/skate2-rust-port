//! ScoreModule.Update82dfcd18. Inputs borrow the already-conditioned PhysOut;
//! the existing Update5 reset owns clearing output events before this stage.
use super::{
    super::{PhysicalPlayerPhysOut, scoring::Scoring2, simple::Animation},
    score_module::ScoreModule,
    scoring_attributes::ScoreError,
    scoring_context_collect::ContextFrame,
    scoring_database::TrickDatabase,
    scoring_external::ScoringExternalInput,
    scoring_module_dispatch::CollectorFrame,
    scoring_module_finish::SequenceFinishFrame,
    scoring_module_selection::CollectorKind,
    scoring_wipeout_lifecycle::{WipeoutFrame, WipeoutTuning},
};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    physics::{FilteredPhysicsState, PhysicsCategory},
    reckoning_numeric::{HostReckoningNumeric, try_length},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

pub(super) struct ModuleFrame<'a> {
    pub collector: CollectorFrame<'a>,
    pub context: ContextFrame<'a>,
    pub wipeout: WipeoutFrame<'a>,
    pub animation: &'a Animation,
    pub physics_state: i32,
    pub category: PhysicsCategory,
    pub requested_teleport: bool,
    pub grind_scoring_blocked: bool,
    pub requested_runout: bool,
    pub retrieving_board: bool,
    pub ground_normal: SkeletonRootVector,
    pub grind_normal: SkeletonRootVector,
    pub system_com: SkeletonRootVector,
    pub gesture: i32,
    pub is_gesturing: bool,
}
pub(super) struct ModuleTuning {
    pub wipeout: WipeoutTuning,
    pub minimum_sketchy_scalar: f32,
    pub force_sketchy: bool,
    pub multiplier_override: Option<i32>,
}
impl ScoreModule {
    /// External reset is performed by the live adapter BEFORE taking the
    /// separate Scoring2 borrow. Native Reset also writes has_been_reset.
    pub(super) fn update_frame(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        out: &mut Scoring2,
        vault: &Arc<AttributeRuntimeVault>,
        db: &TrickDatabase,
        input: Arc<ScoringExternalInput>,
        f: &ModuleFrame<'_>,
        tuning: &ModuleTuning,
    ) -> Result<(), ScoreError> {
        self.check_runout(f.requested_runout, f.animation.running_out, out)?;
        let was_grounded = self
            .reset_suffix
            .as_ref()
            .ok_or(ScoreError::Missing("score reset"))?
            .skater_has_been_on_ground;
        self.digest_external(vault, input.clone(), &f.context, out)?;
        self.metrics
            .published_value()
            .ok_or(ScoreError::Missing("global metrics"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_takeoff_normal(f.category, f.ground_normal, f.grind_normal);
        self.begin_scoring_frame()?;
        // TU3 ScoreModule::Update 82DA3520: with the per-tick holder clears, the air output's
        // locked trajectory (PhysOut_Air+452) into ScoreHolder+0x1C5D for the collectors.
        #[cfg(feature = "skate3-runtime")]
        if self.states.skate3_enabled() {
            self.holder
                .published_value()
                .ok_or(ScoreError::Missing("score holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .skate3_air_reckoning = f.collector.air.use_air_reckoning;
        }
        let kind = self.select_collector(
            f.context.filtered,
            f.physics_state,
            f.requested_teleport,
            f.grind_scoring_blocked,
            f.collector.wheels_in_contact,
            f.collector.animation,
            db,
            &input.enabled,
            out,
        )?;
        super::scoring_module_finish::trace_score(format_args!("collector {kind:?} filtered {} physics {} teleport {} grind {} {:#x} blocked {}",
            f.context.filtered as i32, f.physics_state, f.requested_teleport,
            f.collector.grind.filtered_scorable_id, f.collector.grind.pathed_guid, f.grind_scoring_blocked));
        self.manage_collector(kind, output, db, &f.collector, out)?;
        self.update_collector(db, &f.collector, out)?;
        self.wipeout
            .published_value_mut()
            .ok_or(ScoreError::Missing("wipeout scorer"))?
            .update_wipeout(vault, &input, &f.wipeout, &tuning.wipeout, out)?;
        let (reward_due, in_sequence) = {
            let mut holder = self
                .holder
                .published_value()
                .ok_or(ScoreError::Missing("score holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            (
                holder.check_air_reward(
                    f.animation,
                    tuning.minimum_sketchy_scalar,
                    tuning.force_sketchy,
                )?,
                holder.sequence_mut()?.flags_word & 0x8000_0000 != 0,
            )
        };
        let finish = SequenceFinishFrame {
            activity_worthy: self.activity_worthy(in_sequence, &f.collector)?,
            switched: f.collector.air.switched,
            fakie: f.collector.air.fakie,
            current_is_grind: kind == CollectorKind::Grind,
            current_is_manual: kind == CollectorKind::Ground
                && self
                    .ground
                    .published_value()
                    .ok_or(ScoreError::Missing("ground collector"))?
                    .manual_tracker
                    .in_state,
            multiplier_override: tuning.multiplier_override,
        };
        if reward_due {
            self.reward_air_sequence(&finish, out)?;
        }
        // Original kfUpdateDelta/kfUpdateHz, independent of rendered frames.
        self.decrement_meters(f32::from_bits(0x3c88_8889), 60.0)?;
        self.check_sequence(&finish, out)?;
        self.check_line(f.retrieving_board, out)?;
        self.check_gestures(f.gesture, f.is_gesturing)?;
        // TU3 FillAirPeriodData 82DA83B0 tests the filtered state for a wipeout.
        #[cfg(feature = "skate3-runtime")]
        let wiped_out = if self.states.skate3_enabled() {
            f.context.filtered == FilteredPhysicsState::Wipeout
        } else {
            f.collector.wiped_out
        };
        #[cfg(not(feature = "skate3-runtime"))]
        let wiped_out = f.collector.wiped_out;
        self.fill_collector_output(wiped_out, out)?;
        self.fill_module_output(f.context.filtered, tuning.multiplier_override, out)?;
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        if was_grounded && f.context.filtered != FilteredPhysicsState::Teleport {
            let delta: [f32; 4] = std::array::from_fn(|i| {
                if i < 3 {
                    f.system_com.xyz[i] - state.position_last_frame.xyz[i]
                } else {
                    0.0
                }
            });
            out.distance_travelled_this_frame =
                try_length(delta, &mut HostReckoningNumeric).map_err(ScoreError::Numeric)?;
        }
        state.position_last_frame = f.system_com;
        Ok(())
    }
}
