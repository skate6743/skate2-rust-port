//! Selected queued physical tick followed by same-frame publication.
//!
//! The caller must retain the already constructed world, Actor, animation,
//! terrain, input queue, and presentation owners. This is not a boot constructor,
//! pad poller, native whole-frame schedule, or renderer registration.

use crate::{
    actor_construction::{
        ActorConstruction, ActorIdentity,
        physical_animation_inputs::ActorPhysicalAnimationInputs,
        presentation_frame_source::ActorPresentationFrameSource,
        selected_tick::{
            SelectedPlayerTick, SelectedQueuedTickInput, SelectedQueuedTickOutcome,
            SelectedTickError, SelectedTickNumerics, SelectedTickOwners, SelectedTickRequest,
            SelectedTickSkater, SelectedTickStage,
        },
    },
    actor_face_playback::presentation::{
        frame::{
            SubmittedPresentationFrame,
            selected_runtime::{
                SelectedPresentationError, SelectedPresentationRuntime, SelectedPresentationStage,
            },
        },
        move_root::matrix_inverse::PresentationMatrixNumeric,
        read::decode::PresentationDecodeNumeric,
        startup::SelectedPresentationStartup,
    },
    character::{material_admission::CharacterMaterialError, palette::CharacterPaletteError},
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    game_input_command::GameInputCommandNumeric,
    game_input_frame_session::GameInputFrameSession,
    game_input_manager::GameInputSyncLevel,
    game_input_player::recognizer::GameInputRecognizerServices,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    player::attributes_runtime::PlayerAttributeServices,
    players_backend_settings::PlayersBackendSettings,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::contact_feedback::SimulationContactFeedbackInput,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Debug)]
pub(crate) enum SelectedSessionFrameError<S, R, F, C, N, O> {
    Tick(SelectedTickError<S, R, F, C>),
    Presentation(
        SelectedPresentationError<
            N,
            InputCvarError,
            Infallible,
            crate::actor_construction::body_session::BodySessionError,
            O,
        >,
    ),
    Palette(CharacterPaletteError),
    Materials(CharacterMaterialError),
    #[cfg(feature = "skate3-runtime")]
    Skate3(String),
}

/// A skater after the first in `SkaterManager`'s list, with its own presentation.
pub(crate) struct SelectedFrameSkater<'a> {
    pub(crate) board: &'a SimulationSkaterAssemblyHandle,
    pub(crate) skeleton: &'a SimulationSkaterAssemblyHandle,
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) animation: &'a mut ActorPhysicalAnimationInputs,
    pub(crate) presentation: &'a SelectedPresentationStartup,
    pub(crate) presentation_runtime: &'a mut SelectedPresentationRuntime,
}

/// Borrowed view of the *same* retained selected owners; the future session
/// boot constructor must establish their identity and lifetime before this view
/// can be made available to the app. No owner or original asset is copied here.
pub(crate) struct SelectedSessionFrame<'a> {
    pub(crate) tick: &'a mut SelectedPlayerTick,
    pub(crate) world_owner: &'a Arc<Mutex<PhysicalAssemblyWorld>>,
    pub(crate) world: &'a mut PhysicalAssemblyWorld,
    pub(crate) board: &'a SimulationSkaterAssemblyHandle,
    pub(crate) skeleton: &'a SimulationSkaterAssemblyHandle,
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) animation: &'a mut ActorPhysicalAnimationInputs,
    pub(crate) input: &'a mut GameInputFrameSession<ActorIdentity>,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) settings: &'a PlayersBackendSettings,
    pub(crate) services: &'a PlayerAttributeServices,
    pub(crate) presentation: &'a SelectedPresentationStartup,
    pub(crate) presentation_runtime: &'a mut SelectedPresentationRuntime,
    /// The skaters after the first, in player index order.
    pub(crate) others: Vec<SelectedFrameSkater<'a>>,
    /// The local skater's place in the list.
    pub(crate) local: usize,
    /// The challenge's trick patterns and enabled kinds.
    pub(crate) challenge_scoring: Option<&'a std::sync::Mutex<crate::challenge::skate::ChallengeScoring>>,
    pub(crate) living_world:
        Option<&'a mut crate::selected_living_world_dmo::SelectedLivingWorldDmo>,
    pub(crate) living_world_recipes:
        Option<crate::selected_living_world_recipes::SelectedRecipeFrame<'a>>,
    pub(crate) ai: Option<&'a mut dyn crate::actor_construction::selected_tick::SelectedTickAi>,
}

impl SelectedSessionFrame<'_> {
    /// The caller has already polled/cached the real pad and queued this frame's
    /// selected command. A zero command mask performs no Actor, physics, camera,
    /// or presentation work. A completed tick publishes the palette exactly once.
    /// Failure does not roll back earlier native-order writes or retry them.
    #[expect(
        clippy::too_many_arguments,
        reason = "the source services retain distinct failure and ownership boundaries"
    )]
    #[expect(
        clippy::type_complexity,
        reason = "typed observer callbacks retain distinct tick and presentation failure channels"
    )]
    pub(crate) fn try_step_queued<S, R, F, C, CE, N, O>(
        &mut self,
        request: SelectedTickRequest<'_, SelectedQueuedTickInput<'_>>,
        numeric: &mut SelectedTickNumerics,
        sync: &mut S,
        recognizer: &mut R,
        feedback: &mut F,
        clamp: &mut C,
        presentation_numeric: &mut N,
        tick_observer: Option<&mut dyn FnMut(SelectedTickStage, &PhysicalAssemblyWorld)>,
        presentation_observer: Option<
            &mut dyn FnMut(
                SelectedPresentationStage,
                &SelectedPresentationRuntime,
                &SubmittedPresentationFrame,
            ) -> Result<(), O>,
        >,
    ) -> Result<
        SelectedQueuedTickOutcome,
        SelectedSessionFrameError<S::Error, R::Error, F::Error, CE, N::Error, O>,
    >
    where
        S: GameInputSyncLevel,
        R: GameInputRecognizerServices,
        F: SimulationContactFeedbackInput,
        C: FnMut() -> Result<i32, CE>,
        N: GameInputCommandNumeric + PresentationDecodeNumeric + PresentationMatrixNumeric,
    {
        use SelectedSessionFrameError as E;
        let outcome =
            self.tick
                .try_step_queued(
                    SelectedTickOwners {
                        world_owner: self.world_owner,
                        world: &mut *self.world,
                        board: self.board,
                        skeleton: self.skeleton,
                        actor: self.actor,
                        animation: &mut *self.animation,
                        others: self
                            .others
                            .iter_mut()
                            .map(|other| SelectedTickSkater {
                                board: other.board,
                                skeleton: other.skeleton,
                                actor: other.actor,
                                animation: &mut *other.animation,
                            })
                            .collect(),
                        local: self.local,
                        challenge_scoring: self.challenge_scoring,
                        input: &mut *self.input,
                        cvars: self.cvars,
                        settings: self.settings,
                        living_world: self.living_world.as_deref_mut(),
                        living_world_recipes: self.living_world_recipes.as_mut().map(
                            crate::selected_living_world_recipes::SelectedRecipeFrame::reborrow,
                        ),
                        ai: match self.ai.as_mut() {
                            Some(ai) => Some(&mut **ai),
                            None => None,
                        },
                    },
                    request,
                    numeric,
                    sync,
                    recognizer,
                    feedback,
                    clamp,
                    tick_observer,
                )
                .map_err(E::Tick)?;
        if matches!(outcome, SelectedQueuedTickOutcome::NoCommands(_)) {
            return Ok(outcome);
        }
        let print_profile =
            !crate::diagnostics::is_quiet() && std::env::var_os("SKATE2_PROFILE_REDRAW").is_some();
        let presentation_start =
            (print_profile || crate::diagnostics::frame_profile::enabled()).then(Instant::now);
        if self.presentation_runtime.materials().is_err() {
            let finalized = self
                .presentation
                .material_source
                .as_ref()
                .ok_or(E::Materials(CharacterMaterialError::Missing(
                    "finalized recipe owner",
                )))?;
            self.presentation_runtime
                .try_bind_materials(
                    finalized,
                    std::sync::Arc::clone(&self.services.vault),
                    self.cvars.clone(),
                )
                .map_err(E::Materials)?;
        }
        let mut rounding = HostReckoningSinCosNumeric;
        let mut settings = self.cvars;
        self.presentation_runtime.set_cloth_tick(self.tick.presentation_tick());
        self.presentation_runtime
            .try_update::<N, _, _, _, O>(
                self.presentation,
                ActorPresentationFrameSource {
                    actor: self.actor,
                    services: self.services,
                },
                &mut settings,
                presentation_numeric,
                &mut rounding,
                &mut self.animation.body_services,
                presentation_observer,
            )
            .map_err(E::Presentation)?;
        static TRACE_ROOT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *TRACE_ROOT.get_or_init(|| std::env::var_os("SKATE2_TRACE_PRESENTATION_ROOT").is_some()) {
            eprintln!(
                "presentation root: root={:?} hips={:?}",
                self.presentation_runtime.published_root_position(),
                self.presentation_runtime.published_hips_position(),
            );
        }
        if self.presentation_runtime.published_peer().is_some() {
            self.presentation_runtime.palette().map_err(E::Palette)?;
            self.presentation_runtime.materials().map_err(E::Materials)?;
        }
        for other in &mut self.others {
            if other.presentation_runtime.materials().is_err() {
                let finalized = other.presentation.material_source.as_ref().ok_or(E::Materials(
                    CharacterMaterialError::Missing("finalized recipe owner"),
                ))?;
                other
                    .presentation_runtime
                    .try_bind_materials(
                        finalized,
                        std::sync::Arc::clone(&self.services.vault),
                        self.cvars.clone(),
                    )
                    .map_err(E::Materials)?;
            }
            let mut settings = self.cvars;
            other.presentation_runtime.set_cloth_tick(self.tick.presentation_tick());
            other
                .presentation_runtime
                .try_update::<N, _, _, _, O>(
                    other.presentation,
                    ActorPresentationFrameSource {
                        actor: other.actor,
                        services: self.services,
                    },
                    &mut settings,
                    presentation_numeric,
                    &mut rounding,
                    &mut other.animation.body_services,
                    None,
                )
                .map_err(E::Presentation)?;
            if other.presentation_runtime.published_peer().is_some() {
                other.presentation_runtime.palette().map_err(E::Palette)?;
                other.presentation_runtime.materials().map_err(E::Materials)?;
            }
        }
        if let Some(start) = presentation_start {
            crate::diagnostics::frame_profile::record(8, start.elapsed());
            if print_profile {
                let presentation_ms = start.elapsed().as_secs_f64() * 1000.0;
                use std::io::Write;
                let stderr = std::io::stderr();
                let mut output = std::io::BufWriter::with_capacity(1024, stderr.lock());
                writeln!(
                    output,
                    "diagnostic selected presentation: total={}ms",
                    presentation_ms,
                )
                .expect("write selected profile");
                output.flush().expect("flush selected profile");
            }
        }
        Ok(outcome)
    }
}
