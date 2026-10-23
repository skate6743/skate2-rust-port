//! Live selected frame inputs over the retained `GameInfo` and manager Params.
use super::{
    SimulationAssemblyConstraintSession,
    contact_feedback::SimulationContactFeedbackInput,
    world_services::{PhysicalGameInfo, PhysicalWorldServicesError},
};
use crate::{
    game_input_manager::GameInputSyncLevel,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};
use std::sync::{Arc, Mutex};

impl GameInputSyncLevel for Arc<PhysicalGameInfo> {
    type Error = PhysicalWorldServicesError;
    fn sync_level(&mut self) -> Result<i32, Self::Error> {
        self.try_sync_level()
    }
}
/// Retained projection of this actual `SimulationManager`'s Params+0x24.
/// It is distinct from the Simulation's mutable replacement cooldown.
pub(crate) struct SimulationManagerFeedbackParameters {
    frames_before_freezing: Mutex<u32>,
}
#[derive(Debug)]
pub(crate) enum LiveContactFeedbackError {
    AlreadyInitialized,
    MissingManagerParameters,
    Lock,
    SimRateLock,
    Cvar(InputCvarError),
}
impl std::fmt::Display for LiveContactFeedbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => {
                f.write_str("selected manager feedback Params already initialized")
            }
            Self::MissingManagerParameters => {
                f.write_str("selected manager feedback Params uninitialized")
            }
            Self::Lock => f.write_str("selected manager feedback Params lock poisoned"),
            Self::SimRateLock => f.write_str("Simulation sim rate lock poisoned"),
            Self::Cvar(e) => e.fmt(f),
        }
    }
}
pub(crate) struct LiveContactFeedbackInput {
    parameters: Arc<SimulationManagerFeedbackParameters>,
    cvars: InputCvarReferences,
}
impl SimulationContactFeedbackInput for LiveContactFeedbackInput {
    type Error = LiveContactFeedbackError;
    fn current_enable_freezing(&mut self) -> Result<u8, Self::Error> {
        self.cvars
            .try_enable_freezing()
            .map(u8::from)
            .map_err(LiveContactFeedbackError::Cvar)
    }
    fn current_frames_before_freezing(&mut self) -> Result<u32, Self::Error> {
        Ok(*self
            .parameters
            .frames_before_freezing
            .lock()
            .map_err(|_| LiveContactFeedbackError::Lock)?)
    }
}
impl<M, C> SimulationAssemblyConstraintSession<M, C> {
    /// Selected `World::Initialize827b0f20/0f7c` Params and the corresponding
    /// manager constructor's separate Simulation cooldown store at `82e0c71c` and its
    /// `freezingThreshold` (Params+20, 1e-5) store to `m_MinEnergy` at `82e0c70c`, the
    /// energy below which a body counts toward freezing.
    /// This projection does not claim the remaining whole World constructor.
    pub(crate) fn try_initialize_world_feedback_parameters(
        &mut self,
    ) -> Result<(), LiveContactFeedbackError> {
        if self.feedback_parameters.is_some() {
            return Err(LiveContactFeedbackError::AlreadyInitialized);
        }
        let parameters = Arc::new(SimulationManagerFeedbackParameters {
            frames_before_freezing: Mutex::new(2),
        });
        self.feedback_parameters = Some(parameters);
        self.backend.rigid_bodies.set_cooldown(2);
        self.backend.rigid_bodies.set_minimum_energy(f32::from_bits(0x3727_c5ac));
        // World Params+10 -> native Simulation+B0, overwritten each prephysics1.
        self.simulation_max_iterations = Some(25);
        Ok(())
    }
    /// Native827b3738..3744, after input recognition and before Actor callbacks.
    /// The store survives later Actor failure; the returned snapshot feeds this solve.
    /// Native selects by the exact retained flags (Both3 is single player), not the submit mask.
    /// The port's online game keeps its Both island but takes the network game's count, as the
    /// original's Simulation and Visual islands do.
    pub(crate) fn try_sample_prephysics_iterations(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<u32, LiveContactFeedbackError> {
        if self.simulation_max_iterations.is_none() {
            return Err(LiveContactFeedbackError::MissingManagerParameters);
        }
        let online = self
            .sim_rate
            .lock()
            .map_err(|_| LiveContactFeedbackError::SimRateLock)?
            .online_game();
        let value = cvars
            .try_island_max_iterations(self.island_flags == 3 && !online)
            .map_err(LiveContactFeedbackError::Cvar)?;
        // Native LWZ/STW preserves all32 bits; do not clamp/sanitize the live target.
        let value = u32::from_ne_bytes(value.to_ne_bytes());
        self.simulation_max_iterations = Some(value);
        Ok(value)
    }
    /// Retains the same manager parameter allocation and live cvar references;
    /// neither current value is sampled or cached by this adapter construction.
    pub(crate) fn try_live_contact_feedback(
        &self,
        cvars: &InputCvarReferences,
    ) -> Result<LiveContactFeedbackInput, LiveContactFeedbackError> {
        Ok(LiveContactFeedbackInput {
            parameters: Arc::clone(
                self.feedback_parameters
                    .as_ref()
                    .ok_or(LiveContactFeedbackError::MissingManagerParameters)?,
            ),
            cvars: cvars.clone(),
        })
    }
}
#[cfg(test)]
#[path = "physical_player_live_frame_services_tests.rs"]
mod tests;
