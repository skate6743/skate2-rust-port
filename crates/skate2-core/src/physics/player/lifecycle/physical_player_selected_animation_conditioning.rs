//! Selected retained Animation conditioner after the seven Physics callbacks.
use super::selected_ground_conditioning::SelectedGroundPhysicsComplete;
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_phys_out::conditioners::animation_update::{
        AnimationUpdateComplete, AnimationUpdateError,
    },
    simulation_assembly_backend::constraint_session::{
        physical_island::SimulationPhysicalBodyError, update5_managers::GrindManagerFillPolicy,
    },
};
use std::sync::Arc;

pub(crate) struct SelectedGroundAnimationComplete {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) animation: AnimationUpdateComplete,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}
#[derive(Debug)]
pub(crate) enum SelectedAnimationFinishError {
    Body(SimulationPhysicalBodyError),
    ForeignOwner,
    Poisoned,
    MissingConditioners,
    Update(AnimationUpdateError),
    ScoringInput(super::scoring_input::ScoringInputError),
    Scoring(crate::physical_player_phys_out::conditioners::scoring_update::ScoringUpdateError),
    Hud(crate::physical_player_phys_out::conditioners::scoring_attributes::ScoreError),
    GameCam(
        crate::physical_player_phys_out::conditioners::grinds_update::GrindsConditionerError<
            std::convert::Infallible,
        >,
    ),
}
impl std::fmt::Display for SelectedAnimationFinishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Body(error) => error.fmt(f),
            Self::ForeignOwner => {
                f.write_str("Animation continuation belongs to another physical pair")
            }
            Self::Poisoned => f.write_str("Animation continuation owner poisoned"),
            Self::MissingConditioners => {
                f.write_str("Animation continuation requires retained conditioners")
            }
            Self::Update(error) => error.fmt(f),
            Self::ScoringInput(error) => error.fmt(f),
            Self::Scoring(error) => error.fmt(f),
            Self::Hud(error) => error.fmt(f),
            Self::GameCam(error) => error.fmt(f),
        }
    }
}
impl PhysicalPlayerAssemblyOwner {
    /// `PhysOutModule_Hud::Update`'s statistics view of the completed output.
    pub(crate) fn try_hud_stats_frame(
        &self,
    ) -> Result<crate::skater_manager_local_identity::stats::hud::HudStatsFrame, SelectedAnimationFinishError> {
        use SelectedAnimationFinishError as E;
        let conditioners = self.conditioners.lock().map_err(|_| E::Poisoned)?;
        let conditioners = conditioners.as_ref().ok_or(E::MissingConditioners)?;
        let output = self.phys_out().lock().map_err(|_| E::Poisoned)?;
        conditioners.try_hud_stats_frame(&output).map_err(E::Hud)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_scoring_hud_frame(&self, owner: &PhysicalPlayerAssemblyOwner)
        -> Result<crate::ui::trick_backend::PhysicsFrame, SelectedAnimationFinishError>
    {
        use SelectedAnimationFinishError as E;
        let conditioners = owner.conditioners.lock().map_err(|_| E::Poisoned)?;
        let conditioners = conditioners.as_ref().ok_or(E::MissingConditioners)?;
        let output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        conditioners.try_scoring_hud_frame(&output).map_err(E::Hud)
    }
    /// The HUD's name of the trick with Vault key `key`, from `owner`'s trick database.
    pub(crate) fn try_hud_trick_name(&self, owner: &PhysicalPlayerAssemblyOwner, key: u64)
        -> Result<Option<crate::ui::trick_backend::TrickName>, SelectedAnimationFinishError>
    {
        use SelectedAnimationFinishError as E;
        let conditioners = owner.conditioners.lock().map_err(|_| E::Poisoned)?;
        let conditioners = conditioners.as_ref().ok_or(E::MissingConditioners)?;
        conditioners.try_hud_trick_name(key).map_err(E::Hud)
    }
    /// Selected native Animation.Update82e31970. Other `UpdateAll` children remain
    /// explicit exclusions; this token is not whole-frame completion.
    pub(crate) fn try_finish_selected_animation(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: SelectedGroundPhysicsComplete,
        scoring: Option<super::scoring_input::ScoringFrameSchedule<'_>>,
    ) -> Result<SelectedGroundAnimationComplete, SelectedAnimationFinishError> {
        use SelectedAnimationFinishError as E;
        for handle in [board, skeleton] {
            let actual = self
                .physical_body_owner_for_contacts(handle)
                .map_err(E::Body)?;
            if !Arc::ptr_eq(actual, &before.owner) {
                return Err(E::ForeignOwner);
            }
        }
        let owner = before.owner;
        let animation = {
            let mut conditioners = owner.conditioners.lock().map_err(|_| E::Poisoned)?;
            let conditioners = conditioners.as_mut().ok_or(E::MissingConditioners)?;
            let mut output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
            // World.UpdateConditioners collects external input before invoking
            // UpdateAll. Scoring itself follows Animation and precedes GameCam.
            let scoring = scoring
                .map(|schedule| self.try_scoring_input(&owner, &output, schedule))
                .transpose()
                .map_err(E::ScoringInput)?;
            let animation = conditioners
                .try_update_animation(
                    before.physics,
                    &mut output,
                    &owner.inputs().cvars,
                    &self.debug_overlay,
                )
                .map_err(E::Update)?;
            if let Some(scoring) = scoring {
                conditioners
                    .try_update_scoring(
                        &animation,
                        &mut output,
                        &owner.inputs().vault,
                        scoring,
                        &owner.inputs().cvars,
                        false,
                    )
                    .map_err(E::Scoring)?;
            }
            // PathRecording is not selected; Stats is the empty825187c0.
            // GameCam82e2d380 is the last UpdateAll child.
            conditioners
                .try_update_game_cam(&mut output)
                .map_err(E::GameCam)?;
            animation
        };
        Ok(SelectedGroundAnimationComplete {
            owner,
            animation,
            grind_policy: before.grind_policy,
        })
    }
}

#[cfg(feature = "skate3-runtime")]
impl PhysicalAssemblyWorld {
    /// Complete host gameplay consumers from the source solver's actual facts.
    /// The native Skate 2 physics/animation solver is not advanced here.
    pub(crate) fn try_finish_skate3_gameplay(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        frame: &crate::skate3_native::FrameOutput,
        dt: f32,
        teleport: Option<[f32; 16]>,
        schedule: super::scoring_input::ScoringFrameSchedule<'_>,
        numeric: &mut crate::reckoning_numeric::HostReckoningNumeric,
    ) -> Result<(), String> {
        let mut conditioners = owner.conditioners.lock().map_err(|_| "native gameplay conditioners lock")?;
        let conditioners = conditioners.as_mut().ok_or("native gameplay conditioners missing")?;
        let mut output = owner.phys_out().lock().map_err(|_| "native gameplay output lock")?;
        let publication = output.try_publish_skate3_observers(frame, dt, teleport)?;
        // The same Actor's GetPhysUpdateData payload remains authoritative
        // for gestures even while its physical solver is Skate 3.
        let actor_input=Arc::clone(owner.animation_physics_input.lock()
            .map_err(|_|"source gesture input owner lock")?.as_ref()
            .ok_or("source gesture actual AnimOutPhysIn")?);
        let actor_input=actor_input.lock().map_err(|_|"source gesture input lock")?;
        output.try_publish_actor_gesture(actor_input.gesture,actor_input.gesturing)?;
        drop(actor_input);

        // TeleportPlayerNow82e00988 resets the retained ScoreModule. Relay
        // that callback, including automatic bails, to S2's scoring consumer.
        // Native camera/trajectory conditioners retain their selected source
        // publications; resetting those would invalidate live source anchors.
        if frame.teleported {
            conditioners.try_reset_skate3_scoring(&mut output)
                .map_err(|error|format!("source teleport scoring: {error:?}"))?;
            crate::diagnostics::test_log::event("skate3_gameplay_reset", format_args!(
                "tick={} state={} offboard={} owner={:p} source_reset=true",
                frame.tick,frame.state,frame.offboard,std::sync::Arc::as_ptr(owner)));
        }
        let input = self.try_scoring_input(owner, &output, schedule).map_err(|error|error.to_string())?;
        conditioners.try_update_grinds(&mut output, &owner.inputs().cvars, &self.debug_overlay, numeric)
            .map_err(|error|error.to_string())?;
        conditioners.try_update_audio(&mut output, numeric).map_err(|error|error.to_string())?;
        let animation = conditioners.try_update_skate3_animation(publication, &mut output, &owner.inputs().cvars, &self.debug_overlay)
            .map_err(|error|error.to_string())?;
        conditioners.try_update_scoring(&animation, &mut output, &owner.inputs().vault, input, &owner.inputs().cvars, true)
            .map_err(|error|error.to_string())?;
        Ok(())
    }
}
