//! PhysOutConditioner_Scoring.Update82e2b438 after Animation.Update.
use super::{
    Conditioners, ConstructionError, PhysicalPlayerPhysOut,
    animation_update::AnimationUpdateComplete,
    scoring_attributes::ScoreError,
    scoring_database::TrickDatabase,
    scoring_external::ScoringExternalInput,
    scoring_module_update::{ModuleFrame, ModuleTuning},
};
use crate::{attribute_runtime::AttributeRuntimeVault, input_cvar_runtime::InputCvarReferences};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum ScoringUpdateError {
    ForeignOutput,
    MissingScoring,
    MissingModule,
    Reset(ConstructionError<Infallible>),
    Score(ScoreError),
}
impl std::fmt::Display for ScoringUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Conditioners {
    /// PC source solver's actual reset callback; the same ScoreModule.Reset
    /// that native TeleportPlayerNow82e00988 reaches through ResetAll.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_reset_skate3_scoring(&mut self, output: &mut PhysicalPlayerPhysOut)
        -> Result<(), ScoringUpdateError> {
        use ScoringUpdateError as E;
        if !self.matches_output(output) { return Err(E::ForeignOutput); }
        self.scoring.published_value_mut().ok_or(E::MissingScoring)?.payload
            .module.published_value_mut().ok_or(E::MissingModule)?
            .reset(output).map_err(E::Reset)
    }

    pub(crate) fn try_update_scoring(
        &mut self,
        before: &AnimationUpdateComplete,
        output: &mut PhysicalPlayerPhysOut,
        vault: &Arc<AttributeRuntimeVault>,
        input: Arc<ScoringExternalInput>,
        cvars: &InputCvarReferences,
        use_skate3: bool,
    ) -> Result<(), ScoringUpdateError> {
        use ScoringUpdateError as E;
        if !before.matches_output(output) || !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        let bound = self
            .scoring
            .published_value_mut()
            .ok_or(E::MissingScoring)?;
        if !Arc::ptr_eq(&bound.output, output.occurrence()) {
            return Err(E::ForeignOutput);
        }
        let score = &mut bound.payload;
        if score.database.is_none() {
            // The native global database is immutable after Vault construction.
            // Retain its host lookup tables; never rebuild them per simulation tick.
            score.database = Some(TrickDatabase::load(vault).map_err(E::Score)?);
        }
        #[cfg(feature = "skate3-runtime")]
        { score.database.as_mut().unwrap().use_skate3 = use_skate3; }
        #[cfg(not(feature = "skate3-runtime"))]
        let _ = use_skate3;
        let db = score.database.as_ref().unwrap();
        if !db.matches_vault(vault) {
            return Err(E::Score(ScoreError::Invalid("scoring Vault changed")));
        }
        let module = score.module.published_value_mut().ok_or(E::MissingModule)?;
        #[cfg(feature = "skate3-runtime")]
        module.select_skate3(use_skate3).map_err(E::Score)?;
        if input.reset_scoring {
            module.reset(output).map_err(E::Reset)?;
        }
        let tuning = ModuleTuning::sample(cvars).map_err(E::Score)?;
        // Split ownership without copying the large scoring output or creating
        // another PhysOut. Restore it even when an original dependency fails.
        let mut out = output.concrete.scoring.take().ok_or(E::MissingScoring)?;
        let result = (|| {
            let frame = ModuleFrame::sample(output, &input, cvars).map_err(E::Score)?;
            module
                .update_frame(output, &mut out, vault, db, input.clone(), &frame, &tuning)
                .map_err(E::Score)?;
            // UpdateAudioScore82e29908 keeps a positive award for four further
            // updates; the fifth clears it unless another award arrives.
            if out.banked_point_change_this_frame > 0 {
                score.update_tick = 4;
                score.audio_score = out.banked_point_change_this_frame;
            } else if score.update_tick > 0 {
                score.update_tick -= 1;
            } else {
                score.audio_score = 0;
            }
            out.audio_banked_point_change = score.audio_score;
            Ok(())
        })();
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if result.is_ok()
            && *TRACE.get_or_init(|| std::env::var("SKATE2_TRACE_SCORING").is_ok_and(|v| v == "1"))
            && (input.tick % 60 == 0
                || out.sequence_begun
                || out.sequence_completed
                || out.tricks_begun.num_entries != 0
                || out.tricks_ended.num_entries != 0
                || out.tricks_converted_to.num_entries != 0
                || out.banked_point_change_this_frame != 0)
        {
            std::eprintln!(
                "scoring tick={} raw={} begin={} end={} converted={} sequence_begin={} sequence_end={} score={} hud={} banked={} line={} timer={} momentum={} multiplier={}",
                input.tick,
                out.raw_scorable_id,
                out.tricks_begun.num_entries,
                out.tricks_ended.num_entries,
                out.tricks_converted_to.num_entries,
                out.sequence_begun,
                out.sequence_completed,
                out.sequence_score,
                out.pre_multiplied_hud_sequence_score,
                out.banked_point_change_this_frame,
                out.line_score,
                out.line_timer_points,
                out.momentum_meter_points,
                out.momentum_meter_multiplier
            );
        }
        output.concrete.scoring = Some(out);
        result
    }
}
