//! Selected host tick over retained native input, Actor and physical owners.
//! This does not implement the whole Simulation/camera/locomotion schedule.
use super::{
    ActorConstruction, ActorConstructionError, ActorIdentity,
    breadcrumb_postupdate::ActorBreadCrumbPostUpdateError,
    hands_face_session::{HandsFaceSelection, HandsFaceSessionOutputs},
    physical_animation_inputs::ActorPhysicalAnimationInputs,
    physical_mount::ActorPhysicalMountError,
    selected_frame_session::{
        SelectedActorFrameError, SelectedActorFrameRequest, SelectedActorFrameServices,
    },
};
use crate::{
    actor_face_playback::blend_pose_stream::BlendPoseTokenPool,
    collision_line_numeric::HostCollisionLineNumeric,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    game_input_frame_session::{
        GameInputFrameDelivery, GameInputFrameError, GameInputFrameRequest, GameInputFrameSession,
    },
    game_input_manager::GameInputSyncLevel,
    game_input_player::recognizer::GameInputRecognizerServices,
    gameplay_input_update_manager::GameplayInputUpdateError,
    input_cvar_runtime::InputCvarReferences,
    native_skater_random::SimulationCommandRandom,
    physical_player_assembly::{
        PhysicalAssemblyWorld, selected_animation_conditioning::SelectedGroundAnimationComplete,
    },
    players_backend_settings::PlayersBackendSettings,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    selected_sim_command_queue::{
        CommandConsumeError, CommandStageDelivery, SelectedSimCommandQueue,
    },
    simulation_assembly_backend::constraint_session::{
        SimulationAssemblyConstraintFrameCounts,
        contact_feedback::SimulationContactFeedbackInput,
        ground_pre_solve::{
            GroundPreSolveComplete, GroundPreparationError, GroundPreparationGrindPolicy,
            HostGroundPreparationNumeric,
        },
        motion_force_physics::MotionForcePhysicsWriter,
        post_solve_conditioning::{PostSolveError, PostSolveNumerics, PostSolveRequest},
        prediction::runtime::PredictionRuntimeError,
        skateboard_ground::{
            SkateboardGroundFrameError, SkateboardGroundFrameInput, SkateboardGroundFrameSchedule,
            SkaterSkaterPairs,
        },
        update5_managers::GrindManagerFillPolicy,
    },
    simulation_contact_jacobian::ground_preparation::cpu_job::SimulationStaticGroundJobReport,
    skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
    skater_manager_local_identity::{LocalActorQueryError, SkaterManagerLocalIdentity},
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Instant,
};

pub(crate) struct SelectedPlayerTick {
    hand_tokens: BlendPoseTokenPool,
    // Simulation.mTick/mVisualTick, advanced at SimUpdate1, not rendering.
    tick: u32,
    visual_tick: u32,
    // Keep all completed HUD facts until the visual consumer drains them;
    // rendering may span multiple simulation ticks and must not lose events.
    pub(crate) scoring_hud_frames: std::cell::RefCell<Vec<crate::ui::trick_backend::PhysicsFrame>>,
    pub(crate) scoring_hud_subscribed: std::cell::Cell<bool>,
    // False for the next tick while an online game follows another player's skater (set before
    // each gameplay tick, back to true after it): the HUD shows that skater's frames
    // (`online::hud_record`), and the tick's local frame waits in `scoring_hud_held` for the
    // online tick to take its backend commands and rate.
    pub(crate) scoring_hud_local: std::cell::Cell<bool>,
    pub(crate) scoring_hud_held: std::cell::RefCell<Option<crate::ui::trick_backend::PhysicsFrame>>,
    // Explicit selected SimUpdate2 camera subsequence, not its omitted managers.
    camera_subsequence: Option<u32>,
    // PhysOutModule_Hud's statistics members and the TricksBackEnd line their bests follow.
    hud_stats: crate::skater_manager_local_identity::stats::hud::HudStats,
    #[cfg(test)]
    requested_transition:Option<crate::simulation_assembly_backend::constraint_session::physical_state::ground_to_air::GroundDestination>,
}
/// Optional host observation boundaries; observers receive no mutable world.
#[derive(Clone, Copy)]
pub(crate) enum SelectedTickStage {
    BeforeActor,
    BeforePreSolve,
    BeforeSolver,
    AfterSolver,
}
type StageObserver<'a> = Option<&'a mut dyn FnMut(SelectedTickStage, &PhysicalAssemblyWorld)>;
fn observe(
    observer: &mut StageObserver<'_>,
    stage: SelectedTickStage,
    world: &PhysicalAssemblyWorld,
) {
    if let Some(callback) = observer.as_mut() {
        callback(stage, world);
    }
}
/// Explicit host numerical profile already used by the selected physical APIs.
/// The caller retains these services; no tuning values or timesteps live here.
pub(crate) struct SelectedTickNumerics {
    pub(crate) input: HostReckoningNumeric,
    pub(crate) actor: HostReckoningNumeric,
    pub(crate) pre: HostGroundPreparationNumeric,
    pub(crate) line: HostCollisionLineNumeric,
    pub(crate) vector: HostReckoningNumeric,
    pub(crate) rounding: HostReckoningSinCosNumeric,
    pub(crate) truck: HostSkateboardTruckDriveNumeric,
    pub(crate) collision: HostSkeletonCollisionNumeric,
}
#[derive(Clone, Copy)]
pub(crate) struct SelectedTickRequest<'a, I = GameInputFrameRequest<'a>> {
    pub(crate) input: I,
    pub(crate) dt: f32,
    pub(crate) lod: i32,
    pub(crate) wait_for_bs_job: bool,
    pub(crate) physics: SkateboardGroundFrameInput<'a>,
    pub(crate) pre_grind: GroundPreparationGrindPolicy,
    pub(crate) post_grind: GrindManagerFillPolicy,
}
impl<'a, I> SelectedTickRequest<'a, I> {
    pub(crate) fn with_input<J>(self, input: J) -> SelectedTickRequest<'a, J> {
        self.map_input(|_| input)
    }
    pub(crate) fn map_input<J>(self, convert: impl FnOnce(I) -> J) -> SelectedTickRequest<'a, J> {
        SelectedTickRequest {
            input: convert(self.input),
            dt: self.dt,
            lod: self.lod,
            wait_for_bs_job: self.wait_for_bs_job,
            physics: self.physics,
            pre_grind: self.pre_grind,
            post_grind: self.post_grind,
        }
    }
}
pub(crate) struct SelectedQueuedTickInput<'a> {
    pub(crate) queue: &'a mut SelectedSimCommandQueue,
    pub(crate) random: &'a mut SimulationCommandRandom,
}
enum TickInputMode<'a> {
    Immediate(GameInputFrameRequest<'a>),
    Queued(SelectedQueuedTickInput<'a>),
}
enum AdmittedTickInput<'a> {
    Immediate(GameInputFrameRequest<'a>),
    Queued(CommandStageDelivery),
}
pub(crate) enum SelectedTickInputDelivery {
    Immediate(GameInputFrameDelivery),
    Queued {
        commands: CommandStageDelivery,
        flushed_events: usize,
    },
}
pub(crate) enum SelectedQueuedTickOutcome {
    NoCommands(CommandStageDelivery),
    Completed(Box<SelectedTickComplete>),
}
/// The selected physical subset admits full flags3, not visual-only scheduling.
fn queued_mask_admitted(report: &CommandStageDelivery) -> Result<bool, u32> {
    match report.update_mask {
        0 => Ok(false),
        3 => Ok(true),
        mask => Err(mask),
    }
}
/// One skater of `SkaterManager::mSkaterList`.
pub(crate) struct SelectedTickSkater<'a> {
    pub(crate) board: &'a SimulationSkaterAssemblyHandle,
    pub(crate) skeleton: &'a SimulationSkaterAssemblyHandle,
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) animation: &'a mut ActorPhysicalAnimationInputs,
}
/// The skater list's AI: `AISkaterManager::Update` in `SimUpdate1` after
/// `GameInputManager::Update`, each AI Actor's `AIController::GenerateActionGraphIntents`
/// inside its `Actor::GenerateActionGraphIntents`, and `AIController::Update` at the top of its
/// `Actor::Update`.
pub(crate) trait SelectedTickAi {
    fn try_appropriate_respawn(
        &mut self,
        actor: &ActorConstruction,
        skaters: &[SelectedTickSkater<'_>],
    ) -> Result<Option<crate::skeleton_root_transforms::SkeletonRootMatrix>, String>;
    fn try_update_manager(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        skaters: &[SelectedTickSkater<'_>],
    ) -> Result<(), String>;
    fn try_generate_intents(
        &mut self,
        index: usize,
        world: &mut PhysicalAssemblyWorld,
        skaters: &[SelectedTickSkater<'_>],
    ) -> Result<(), String>;
    fn try_update_controller(&mut self, skater: &SelectedTickSkater<'_>) -> Result<(), String>;
}
pub(crate) struct SelectedTickOwners<'a> {
    pub(crate) world_owner: &'a Arc<Mutex<PhysicalAssemblyWorld>>,
    pub(crate) world: &'a mut PhysicalAssemblyWorld,
    pub(crate) board: &'a SimulationSkaterAssemblyHandle,
    pub(crate) skeleton: &'a SimulationSkaterAssemblyHandle,
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) animation: &'a mut ActorPhysicalAnimationInputs,
    /// The skaters after the first, in the list's player index order.
    pub(crate) others: Vec<SelectedTickSkater<'a>>,
    /// The local skater's place in the list (`SkaterManager::mLocalIndex`'s Actor).
    pub(crate) local: usize,
    /// The challenge's trick patterns and enabled kinds.
    pub(crate) challenge_scoring: Option<&'a std::sync::Mutex<crate::challenge::skate::ChallengeScoring>>,
    pub(crate) input: &'a mut GameInputFrameSession<ActorIdentity>,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) settings: &'a PlayersBackendSettings,
    /// The LivingWorld DMO schedule, once the main DMO bank is registered.
    pub(crate) living_world:
        Option<&'a mut crate::selected_living_world_dmo::SelectedLivingWorldDmo>,
    pub(crate) living_world_recipes:
        Option<crate::selected_living_world_recipes::SelectedRecipeFrame<'a>>,
    pub(crate) ai: Option<&'a mut dyn SelectedTickAi>,
}
pub(crate) struct SelectedTickComplete {
    pub(crate) input: SelectedTickInputDelivery,
    pub(crate) hands: HandsFaceSessionOutputs,
    pub(crate) before: GroundPreSolveComplete,
    pub(crate) counts: SimulationAssemblyConstraintFrameCounts,
    pub(crate) report: SimulationStaticGroundJobReport,
    pub(crate) after: SelectedGroundAnimationComplete,
}
#[derive(Debug)]
pub(crate) enum SelectedHandsError<C> {
    Local(LocalActorQueryError<ActorConstructionError>),
    Clamp(C),
}
impl<C: std::fmt::Debug> std::fmt::Display for SelectedHandsError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(error) => write!(f, "selected hands locality: {error:?}"),
            Self::Clamp(error) => write!(f, "selected hands clamp: {error:?}"),
        }
    }
}
struct LocalSelection<'a, C> {
    actor: &'a ActorConstruction,
    manager: &'a SkaterManagerLocalIdentity,
    clamp: &'a mut C,
}
impl<C, E> HandsFaceSelection for LocalSelection<'_, C>
where
    C: FnMut() -> Result<i32, E>,
{
    type Error = SelectedHandsError<E>;
    fn is_local_player(&mut self) -> Result<bool, Self::Error> {
        self.manager
            .try_is_local_player(|| {
                let actor = self.actor.prefix()?;
                let fields = actor
                    .fields()
                    .ok_or(ActorConstructionError::ActorFieldsUninitialized)?;
                Ok(i32::from_ne_bytes(fields.player_index.to_ne_bytes()))
            })
            .map_err(SelectedHandsError::Local)
    }
    fn hands_commands_clamp(&mut self) -> Result<i32, Self::Error> {
        (self.clamp)().map_err(SelectedHandsError::Clamp)
    }
}
type HostPostError = PostSolveError<
    ReckoningHostNumericError,
    ReckoningHostNumericError,
    Infallible,
    Infallible,
    Infallible,
>;
#[derive(Debug)]
pub(crate) enum SelectedTickError<S, R, F, C> {
    MissingLocalManager,
    LocalSkater(usize),
    CameraScheduleProfile,
    MissingPredictor,
    MissingWorldQueries,
    Predictor(PredictionRuntimeError<Infallible>),
    StepSelector(crate::simulation_assembly_backend::constraint_session::step_selector::StepSelectorError),
    Iterations(crate::simulation_assembly_backend::constraint_session::live_frame_services::LiveContactFeedbackError),
    CollisionCvar(crate::input_cvar_runtime::InputCvarError),
    GameMode(crate::skater_manager_local_identity::LocalIdentityError),
    Camera(crate::skater_manager_local_identity::observers::SkaterObserverError),
    Breadcrumb(ActorBreadCrumbPostUpdateError),
    QueuedCommand(CommandConsumeError<S>),
    QueuedRecognition(GameplayInputUpdateError<R>),
    UnsupportedQueuedMask(u32),
    Physical(ActorPhysicalMountError),
    Input(GameInputFrameError<ReckoningHostNumericError, S, R>),
    Actor(Box<SelectedActorFrameError<ReckoningHostNumericError, SelectedHandsError<C>>>),
    Before(Box<GroundPreparationError>),
    Solve(SkateboardGroundFrameError<F>),
    After(Box<HostPostError>),
    SimRatePoisoned,
    SimRate(crate::simulation_sim_rate::SimRateError),
    ChallengeScoringPoisoned,
    SkaterManager(crate::selected_session_marker::SkaterManagerCommandError),
    LivingWorld(Box<crate::selected_living_world_dmo::SelectedLivingWorldDmoError>),
    LivingWorldRecipes(Box<crate::selected_living_world_recipes::SelectedLivingWorldRecipeError>),
    ScoringCollisionReset(crate::living_world_grind_services::scoring::ScoringEntitiesError),
    ScoringHud(crate::physical_player_assembly::selected_animation_conditioning::SelectedAnimationFinishError),
    HudStats(String),
    Ai(String),
}
impl<S: std::fmt::Debug, R: std::fmt::Debug, F: std::fmt::Debug, C: std::fmt::Debug>
    std::fmt::Display for SelectedTickError<S, R, F, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StepSelector(error) => write!(f, "second step-selector harvest: {error}"),
            Self::Iterations(e) => write!(f, "selected live solver iterations: {e}"),
            Self::CollisionCvar(e) => write!(f, "selected collision pipeline switch: {e}"),
            Self::GameMode(e) => write!(f, "selected tick game mode: {e}"),
            Self::CameraScheduleProfile => {
                f.write_str("camera subsequence requires explicit physics flags3 profile")
            }
            Self::MissingPredictor => {
                f.write_str("second predictor harvest requires World predictor")
            }
            Self::MissingWorldQueries => {
                f.write_str("second predictor harvest requires World queries")
            }
            Self::Predictor(error) => write!(f, "second predictor harvest: {error}"),
            Self::Camera(error) => write!(f, "selected camera subsequence: {error:?}"),
            Self::Breadcrumb(error) => write!(f, "selected breadcrumb PostUpdate: {error:?}"),
            Self::MissingLocalManager => f.write_str("selected tick local manager absent"),
            Self::LocalSkater(index) => write!(f, "selected tick local skater {index} absent"),
            Self::QueuedCommand(e) => write!(f, "selected tick queued command: {e}"),
            Self::QueuedRecognition(e) => write!(f, "selected tick queued recognition: {e}"),
            Self::UnsupportedQueuedMask(mask) => write!(
                f,
                "selected tick queued mask {mask} requires unrecovered scheduling"
            ),
            Self::Physical(e) => write!(f, "selected tick physical owner: {e:?}"),
            Self::Input(e) => write!(f, "selected tick input: {e}"),
            Self::Actor(e) => write!(f, "selected tick Actor: {e}"),
            Self::Before(e) => write!(f, "selected tick pre-solve: {e}"),
            Self::Solve(e) => write!(f, "selected tick solver: {e}"),
            Self::After(e) => write!(f, "selected tick post-solve: {e}"),
            Self::SimRatePoisoned => f.write_str("selected tick Simulation rate lock poisoned"),
            Self::ChallengeScoringPoisoned => f.write_str("selected tick challenge scoring lock poisoned"),
            Self::SimRate(e) => write!(f, "selected tick: {e}"),
            Self::SkaterManager(e) => write!(f, "selected tick: {e}"),
            Self::LivingWorld(e) => write!(f, "selected tick LivingWorld: {e}"),
            Self::LivingWorldRecipes(e) => e.fmt(f),
            Self::ScoringCollisionReset(e) => e.fmt(f),
            Self::ScoringHud(e) => e.fmt(f),
            Self::HudStats(e) => write!(f, "selected tick HUD statistics: {e}"),
            Self::Ai(e) => write!(f, "selected tick AI: {e}"),
        }
    }
}
impl SelectedPlayerTick {
    pub(crate) fn presentation_tick(&self)->u32{self.tick}
    /// Selected source subsequence only: Camera Update follows World.Update3
    /// and precedes the second predictor harvest. Activity/Layer/Trigger/global
    /// location/LivingWorld scheduling is not certified by this opt-in.
    pub(crate) fn select_camera_subsequence(&mut self, submit_mask: u32) {
        self.camera_subsequence = Some(submit_mask);
    }

    pub(crate) fn new() -> Self {
        Self {
            hand_tokens: BlendPoseTokenPool::new(),
            tick: 0,
            visual_tick: 0,
            scoring_hud_frames: Default::default(),
            scoring_hud_subscribed: Default::default(),
            scoring_hud_local: std::cell::Cell::new(true),
            scoring_hud_held: Default::default(),
            camera_subsequence: None,
            hud_stats: Default::default(),
            #[cfg(test)]
            requested_transition: None,
        }
    }

    /// Controlled corpus destination only; input/Actor/selection still execute.
    #[cfg(test)]
    pub(crate) fn request_transition_for_test(
        &mut self,
        destination:crate::simulation_assembly_backend::constraint_session::physical_state::ground_to_air::GroundDestination,
    ) {
        self.requested_transition = Some(destination);
    }

    /// Existing controlled immediate entry remains available to isolated drivers.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same selected native service boundaries"
    )]
    pub(crate) fn try_step<S, R, F, C, CE>(
        &mut self,
        owners: SelectedTickOwners<'_>,
        request: SelectedTickRequest<'_>,
        numeric: &mut SelectedTickNumerics,
        sync: &mut S,
        recognizer: &mut R,
        feedback: &mut F,
        clamp: &mut C,
        observer: StageObserver<'_>,
    ) -> Result<SelectedTickComplete, SelectedTickError<S::Error, R::Error, F::Error, CE>>
    where
        S: GameInputSyncLevel,
        R: GameInputRecognizerServices,
        F: SimulationContactFeedbackInput,
        C: FnMut() -> Result<i32, CE>,
    {
        match self.try_step_input(
            owners,
            request.map_input(TickInputMode::Immediate),
            numeric,
            sync,
            recognizer,
            feedback,
            clamp,
            observer,
        )? {
            SelectedQueuedTickOutcome::Completed(completed) => Ok(*completed),
            SelectedQueuedTickOutcome::NoCommands(_) => {
                unreachable!("immediate input never enters queued mask admission")
            }
        }
    }
    /// Explicitly admitted queued entry; does not run native whole-application
    /// pause/streamer/NIS/FE scheduling. No batch exits before Actor/physics reads.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "same selected native service boundaries"
    )]
    pub(crate) fn try_step_queued<S, R, F, C, CE>(
        &mut self,
        owners: SelectedTickOwners<'_>,
        request: SelectedTickRequest<'_, SelectedQueuedTickInput<'_>>,
        numeric: &mut SelectedTickNumerics,
        sync: &mut S,
        recognizer: &mut R,
        feedback: &mut F,
        clamp: &mut C,
        observer: StageObserver<'_>,
    ) -> Result<SelectedQueuedTickOutcome, SelectedTickError<S::Error, R::Error, F::Error, CE>>
    where
        S: GameInputSyncLevel,
        R: GameInputRecognizerServices,
        F: SimulationContactFeedbackInput,
        C: FnMut() -> Result<i32, CE>,
    {
        self.try_step_input(
            owners,
            request.map_input(TickInputMode::Queued),
            numeric,
            sync,
            recognizer,
            feedback,
            clamp,
            observer,
        )
    }

    /// The caller supplies real input/sync/recognition/feedback and the selected
    /// schedule. Clamp is a lazy host service; a missing capability must return
    /// an error, never a manufactured value. All native partial writes persist.
    #[expect(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "selected composition retains independent native host service error types"
    )]
    fn try_step_input<S, R, F, C, CE>(
        &mut self,
        owners: SelectedTickOwners<'_>,
        request: SelectedTickRequest<'_, TickInputMode<'_>>,
        numeric: &mut SelectedTickNumerics,
        sync: &mut S,
        recognizer: &mut R,
        feedback: &mut F,
        clamp: &mut C,
        mut observer: StageObserver<'_>,
    ) -> Result<SelectedQueuedTickOutcome, SelectedTickError<S::Error, R::Error, F::Error, CE>>
    where
        S: GameInputSyncLevel,
        R: GameInputRecognizerServices,
        F: SimulationContactFeedbackInput,
        C: FnMut() -> Result<i32, CE>,
    {
        use SelectedTickError as E;
        let print_profile =
            !crate::diagnostics::is_quiet() && std::env::var_os("SKATE2_PROFILE_REDRAW").is_some();
        let profile_start =
            (print_profile || crate::diagnostics::frame_profile::enabled()).then(Instant::now);
        if self.camera_subsequence.is_some() && request.physics.schedule != crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 {
            return Err(E::CameraScheduleProfile);
        }
        let SelectedTickOwners {
            world_owner,
            world,
            board,
            skeleton,
            actor,
            animation,
            others,
            local: local_skater,
            challenge_scoring,
            input,
            cvars,
            settings,
            mut living_world,
            mut living_world_recipes,
            mut ai,
        } = owners;
        let mut skaters = Vec::with_capacity(1 + others.len());
        skaters.push(SelectedTickSkater { board, skeleton, actor, animation });
        skaters.extend(others);
        let local_actor = skaters
            .get(local_skater)
            .ok_or(E::LocalSkater(local_skater))?
            .actor;
        // SimController::Update1 Dispatch828a739c flushes eEQGameSim before
        // NIS, queued commands and SimUpdate1.
        if let Some(recipes)=living_world_recipes.as_mut() {
            recipes.dispatch_game_sim(world,cvars,request.physics.ground_pool)
                .map_err(|e|E::SimRate(crate::simulation_sim_rate::SimRateError::Queue(e)))?;
        } else {world
            .sim_rate
            .lock()
            .map_err(|_| E::SimRatePoisoned)?
            .try_dispatch()
            .map_err(E::SimRate)?;}
        let mut simulation_random=None;
        let admitted_input = match request.input {
            TickInputMode::Immediate(input) => AdmittedTickInput::Immediate(input),
            TickInputMode::Queued(queued) => {
                let report = input
                    .try_deliver_queued_commands(queued.queue, queued.random, sync)
                    .map_err(E::QueuedCommand)?;
                if !queued_mask_admitted(&report).map_err(E::UnsupportedQueuedMask)? {
                    return Ok(SelectedQueuedTickOutcome::NoCommands(report));
                }
                // SkaterManager::OnCommand82600920 for receiver 1, in batch
                // order; its Actor writes are disjoint from the input receivers.
                if !report.skater_manager.is_empty() {
                    let local = world
                        .local_skater_identity
                        .as_ref()
                        .map(Arc::clone)
                        .ok_or(E::MissingLocalManager)?;
                    let mut commands =
                        crate::selected_session_marker::SelectedSkaterManagerCommands {
                            local: &local,
                            world: &mut *world,
                            ground: request.physics.ground_pool,
                            cvars,
                            found: None,
                        };
                    for (kind, payload) in &report.skater_manager {
                        crate::session_marker::teleport::try_on_command(
                            &mut commands,
                            *kind,
                            payload,
                        )
                        .map_err(E::SkaterManager)?;
                    }
                }
                simulation_random=Some(queued.random);
                AdmittedTickInput::Queued(report)
            }
        };
        actor.cached_physical_allocation().map_err(E::Physical)?;
        // SimUpdate1 8289f474/488 increments before InputManager.Update.
        let update_mask = match &admitted_input {
            AdmittedTickInput::Immediate(request) => request.update_mask,
            AdmittedTickInput::Queued(commands) => commands.update_mask,
        };
        self.visual_tick = self.visual_tick.wrapping_add((update_mask >> 1) & 1);
        self.tick = self.tick.wrapping_add(update_mask & 1);
        // World::UpdateConditioners 827af468 builds one input for every skater; a
        // simulation update pops the challenge's trick patterns.
        let challenge_scoring = challenge_scoring
            .map(|scoring| scoring.lock().map(|mut scoring| scoring.pop(update_mask & 1 != 0)))
            .transpose()
            .map_err(|_| E::ChallengeScoringPoisoned)?;
        let local = world
            .local_skater_identity
            .as_ref()
            .map(Arc::clone)
            .ok_or(E::MissingLocalManager)?;
        let overlay = world.debug_overlay.clone();
        let ground = request.physics.ground_pool;
        observe(&mut observer, SelectedTickStage::BeforeActor, world);
        let queued_time = profile_start.map(|start| start.elapsed());
        let input = match admitted_input {
            AdmittedTickInput::Immediate(request) => SelectedTickInputDelivery::Immediate(
                input
                    .try_step(request, &mut numeric.input, sync, recognizer, &overlay)
                    .map_err(E::Input)?,
            ),
            AdmittedTickInput::Queued(commands) => {
                let flushed_events = input
                    .try_recognize_delivered(commands.update_mask, recognizer, &overlay)
                    .map_err(E::QueuedRecognition)?;
                SelectedTickInputDelivery::Queued {
                    commands,
                    flushed_events,
                }
            }
        };
        // SimUpdate1 8289f558: AISkaterManager.Update follows GameInputManager.Update.
        if let Some(ai) = ai.as_deref_mut() {
            let _profile = crate::diagnostics::frame_profile::span(crate::diagnostics::frame_profile::AI_MANAGER);
            ai.try_update_manager(world, &skaters).map_err(E::Ai)?;
        }
        // SimUpdate1 8289f580: RecipeMan.Update precedes Manager.Update1.
        if let Some(recipes) = living_world_recipes.as_mut() {
            let _profile = crate::diagnostics::frame_profile::span(9);
            recipes
                .update(world, cvars)
                .map_err(|e| E::LivingWorldRecipes(Box::new(e)))?;
        }
        // SimUpdate1 8289f5a8/8289f5e0: LivingWorld Manager::Update1 and
        // World::BeginDMOUpdate, after input and before UpdateAnimation1.
        if let Some(living_world) = living_world.as_deref_mut() {
            let _profile = crate::diagnostics::frame_profile::span(10);
            living_world
                .try_update1(world, local_actor, &local, cvars, living_world_recipes.as_mut(),ground,simulation_random.as_deref_mut())
                .map_err(|error| E::LivingWorld(Box::new(error)))?;
        }
        let max_iterations = world
            .try_sample_prephysics_iterations(cvars)
            .map_err(E::Iterations)?;
        let actor_request = SelectedActorFrameRequest {
            dt: request.dt,
            lod: request.lod,
            wait_for_bs_job: request.wait_for_bs_job,
            cvars,
            settings,
        };
        // SkaterManager::UpdateAnimation1 825ffc70: Phase1 over the skater list. An AI
        // Actor's GenerateActionGraphIntents takes its controller's map, filled first.
        for index in 0..skaters.len() {
            if let Some(ai) = ai.as_deref_mut() {
                ai.try_generate_intents(index, world, &skaters).map_err(E::Ai)?;
            }
            with_actor_services(&mut skaters[index], world, ground, &mut self.hand_tokens, &local, clamp, &mut numeric.actor,
                |actor, services| actor.try_selected_actor_phase1(actor_request, services))
                .map_err(|error| E::Actor(Box::new(error)))?;
        }
        if let Some(living_world) = living_world.as_deref_mut() {
            let _profile = crate::diagnostics::frame_profile::span(11);
            let mut ai_respawn = |actor: &ActorConstruction| {
                ai.as_deref_mut().ok_or_else(|| "Actor AI respawn owner absent".to_owned())?
                    .try_appropriate_respawn(actor, &skaters)
            };
            living_world
                .try_update2(world, local_actor, cvars, ground, living_world_recipes.as_mut(),simulation_random, &mut ai_respawn)
                .map_err(|error| E::LivingWorld(Box::new(error)))?;
        }
        // UpdateAnimation2 825fa310, then UpdateAnimation3 825fa4e0: Phase3 for every
        // skater before any DispatchHandsFaceAnim.
        for skater in &skaters {
            skater
                .actor
                .try_selected_actor_phase2::<HostReckoningNumeric, LocalSelection<'_, C>>(actor_request)
                .map_err(|error| E::Actor(Box::new(error)))?;
        }
        for skater in &mut skaters {
            with_actor_services(skater, world, ground, &mut self.hand_tokens, &local, clamp, &mut numeric.actor,
                |actor, services| actor.try_selected_actor_phase3(actor_request, services))
                .map_err(|error| E::Actor(Box::new(error)))?;
        }
        let mut hands = Vec::with_capacity(skaters.len());
        for skater in &mut skaters {
            hands.push(
                with_actor_services(skater, world, ground, &mut self.hand_tokens, &local, clamp, &mut numeric.actor,
                    |actor, services| actor.try_selected_actor_hands(actor_request, services))
                    .map_err(|error| E::Actor(Box::new(error)))?,
            );
        }
        // UpdatePhysics 825ffd98: each skater's ACS sync, SetUpPhysics and Actor::Update.
        for skater in &mut skaters {
            with_actor_services(skater, world, ground, &mut self.hand_tokens, &local, clamp, &mut numeric.actor,
                |actor, services| {
                    actor.try_selected_actor_physics_sync(services)?;
                    actor.try_selected_actor_set_up_physics(actor_request, services)
                })
                .map_err(|error| E::Actor(Box::new(error)))?;
            // Actor::Update825f10f0 starts with its AIController::Update.
            if let Some(ai) = ai.as_deref_mut() {
                ai.try_update_controller(skater).map_err(E::Ai)?;
            }
            let owner=std::sync::Arc::clone(world.grind_services.living_world());
            let mut living=owner.lock().map_err(|_|E::LivingWorld(Box::new(
                crate::selected_living_world_dmo::SelectedLivingWorldDmoError::LivingWorldLock)))?;
            skater.actor.try_update_after_physics_setup_living_world(request.dt,&mut living,&local)
                .map_err(|error|E::Actor(Box::new(SelectedActorFrameError::AfterPhysicsSetup(Box::new(error)))))?;
        }
        observe(&mut observer, SelectedTickStage::BeforePreSolve, world);
        let actor_time = profile_start.map(|start| start.elapsed());
        // World::Update1 827ae008: the grab query manager, every physical's Update0,
        // then each physical's Update1..3 jobs.
        world
            .try_prepare_ground_pre_solve_world(actor, &mut numeric.pre)
            .map_err(|error| E::Before(Box::new(error)))?;
        let mut ai_respawn = |actor: &ActorConstruction| {
            ai.as_deref_mut().ok_or_else(|| "Actor AI respawn owner absent".to_owned())?
                .try_appropriate_respawn(actor, &skaters)
        };
        for skater in &skaters {
            world
                .try_prepare_ground_pre_solve_update0(
                    world_owner,
                    skater.board,
                    skater.skeleton,
                    skater.actor,
                    ground,
                    &mut numeric.pre,
                    &mut ai_respawn,
                )
                .map_err(|error| E::Before(Box::new(error)))?;
        }
        let mut befores = Vec::with_capacity(skaters.len());
        for skater in &skaters {
            #[cfg(test)]
            let mut unrequested = None;
            #[cfg(test)]
            let requested = if befores.is_empty() { &mut self.requested_transition } else { &mut unrequested };
            befores.push(
                world
                    .try_prepare_ground_pre_solve_pair(
                        world_owner,
                        skater.board,
                        skater.skeleton,
                        skater.actor,
                        ground,
                        request.pre_grind,
                        &mut numeric.pre,
                        &mut ai_respawn,
                        #[cfg(test)]
                        requested,
                    )
                    .map_err(|error| E::Before(Box::new(error)))?,
            );
        }
        // World::Update1 827ae138: deferred grab messages follow all Update3
        // jobs and precede Island::Step_Update1. No force-clear occurs between
        // this dispatch and the solver; begin_dynamic_update_frame sets time.
        world
            .try_update_grab_messages()
            .map_err(|error| E::Before(Box::new(GroundPreparationError::GrabMessages(error))))?;
        // SimUpdate1 8289f77c: Manager.Update3 after World.Update1 and before
        // the locomotion/World.Update2 portion of the shared solver below.
        if let Some(living_world) = living_world.as_deref_mut() {
            living_world.try_update_animation(world,local_actor,&local,cvars,
                living_world_recipes.as_mut(),&mut self.hand_tokens)
                .map_err(|error| E::LivingWorld(Box::new(error)))?;
        }
        observe(&mut observer, SelectedTickStage::BeforeSolver, world);
        let pre_solve_time = profile_start.map(|start| start.elapsed());
        let skater_skater = if !cvars
            .try_run_skater_skater_collision_pipeline()
            .map_err(E::CollisionCvar)?
        {
            SkaterSkaterPairs::Off
        } else if local.backend().try_game_mode().map_err(E::GameMode)? == 2 {
            // Party Play's skaters pass through each other as an online game's remote skaters
            // do, a port change: the original collides them.
            SkaterSkaterPairs::Own
        } else {
            SkaterSkaterPairs::All
        };
        let (counts, report) = world
            .try_step_authored_ground_with_locomotion(
                request.physics,
                request.dt,
                max_iterations,
                feedback,
                skater_skater,
                // SkaterManager::UpdateLocomotion 825f53b0 over the skater list.
                |world| {
                    for skater in &skaters {
                        skater.actor.try_mid_physics_locomotion(3, cvars, world, ground)?;
                    }
                    Ok(())
                },
            )
            .map_err(E::Solve)?;
        observe(&mut observer, SelectedTickStage::AfterSolver, world);
        let solver_time = profile_start.map(|start| start.elapsed());
        // World::Update3 827af8f0: every physical's Update4/Update5 jobs, then
        // UpdateConditioners, whose Scoring input reads the other skaters' output.
        let mut physics = Vec::with_capacity(skaters.len());
        for (skater, before) in skaters.iter().zip(&befores) {
            let owner = Arc::clone(
                skater.actor.cached_physical_allocation().map_err(E::Physical)?.owner(),
            );
            physics.push(
                world
                    .try_post_solve_selected_physics(
                        PostSolveRequest {
                            board: skater.board,
                            skeleton: skater.skeleton,
                            owner: &owner,
                            ground,
                            ground_reckoning: &before.ground_reckoning,
                            actor: skater.actor,
                            grind_policy: request.post_grind,
                            scoring: None,
                        },
                        &mut PostSolveNumerics {
                            line: &mut numeric.line,
                            vector: &mut numeric.vector,
                            rounding: &mut numeric.rounding,
                            truck: &mut numeric.truck,
                            collision: &mut numeric.collision,
                        },
                    )
                    .map_err(|error| E::After(Box::new(error)))?,
            );
        }
        let mut afters = Vec::with_capacity(skaters.len());
        for (skater, physics) in skaters.iter().zip(physics) {
            afters.push(
                world
                    .try_finish_selected_animation(
                        skater.board,
                        skater.skeleton,
                        physics,
                        Some(crate::physical_player_assembly::scoring_input::ScoringFrameSchedule {
                            tick: self.tick, visual_tick: self.visual_tick, settings,
                            challenge: challenge_scoring.as_ref(),
                        }),
                    )
                    .map_err(|error| E::After(Box::new(PostSolveError::FinishAnimation(Box::new(error)))))?,
            );
        }
        let post_solve_time = profile_start.map(|start| start.elapsed());
        // One tick's choice: a step no online game sets up shows the local skater.
        let local_hud_shown = self.scoring_hud_local.replace(true);
        if self.scoring_hud_subscribed.get() {
            let local_owner = Arc::clone(
                local_actor.cached_physical_allocation().map_err(E::Physical)?.owner(),
            );
            let mut frame = world.try_scoring_hud_frame(&local_owner).map_err(E::ScoringHud)?;
            frame.backend_commands=local.backend().take_tricks_commands()
                .map_err(|e|E::SimRate(crate::simulation_sim_rate::SimRateError::Queue(e)))?;
            frame.simulation_rate = world.sim_rate.lock().map_err(|_| E::SimRatePoisoned)?
                .current_sim_update_rate();
            // A held frame no online tick took keeps its commands for the next one queued.
            if let Some(held) = self.scoring_hud_held.get_mut().take() {
                frame.backend_commands.splice(0..0, held.backend_commands);
            }
            if local_hud_shown {
                self.scoring_hud_frames.get_mut().push(frame);
            } else {
                *self.scoring_hud_held.get_mut() = Some(frame);
            }
        }
        // PhysOutModule_Hud::Update 827c79b8 keeps the statistics every frame, HUD shown or not.
        {
            let frame = local_actor.cached_physical_allocation().map_err(E::Physical)?.owner()
                .try_hud_stats_frame().map_err(E::ScoringHud)?;
            let grabbed = match frame.grab_spline {
                Some(spline) => {
                    let living = world.grind_services.living_world().lock()
                        .map_err(|_| E::HudStats("LivingWorld lock".into()))?;
                    match living.dmo_objects.as_deref() {
                        Some(objects) => crate::skater_manager_local_identity::stats::hud::GrabbedObject::find(objects, spline)
                            .map_err(E::HudStats)?,
                        None => None,
                    }
                }
                None => None,
            };
            let period = world.sim_rate.lock().map_err(|_| E::SimRatePoisoned)?.current_sim_update_rate();
            local.backend().update_hud_stats(&mut self.hud_stats, &frame, grabbed, period, cvars)
                .map_err(E::HudStats)?;
        }
        // World.Update3 827abed0, after every physical scoring update and
        // before ResolveContactSpyList publishes this frame's contacts.
        world.grind_services.living_world().lock()
            .map_err(|_| E::ScoringCollisionReset(crate::living_world_grind_services::scoring::ScoringEntitiesError::Missing("LivingWorld lock")))?
            .reset_scoring_collision_flags().map_err(E::ScoringCollisionReset)?;
        // World.Update3 827afa58: ResolveContactSpyList follows the flag reset.
        if let Some(living_world) = living_world.as_deref_mut() {
            living_world
                .try_resolve_contact_spy_lists(world, &local, cvars)
                .map_err(|error| E::LivingWorld(Box::new(error)))?;
        }
        // SimUpdate2 8289fa34: World.Update3 completes before AnimMan.Sync.
        if let Some(living_world) = living_world.as_deref_mut() {
            living_world.try_sync_animation(world,cvars,living_world_recipes.as_mut())
                .map_err(|error| E::LivingWorld(Box::new(error)))?;
        }
        // SimUpdate2 8289fc50: Manager::UpdatePostTransforms follows World
        // Update3 and precedes CameraManager::Update.
        if let Some(living_world) = living_world.as_deref_mut() {
            let _profile = crate::diagnostics::frame_profile::span(12);
            living_world
                .try_update_post_transforms(world, local_actor, &local, cvars,living_world_recipes.as_ref())
                .map_err(|error| E::LivingWorld(Box::new(error)))?;
        }
        if let Some(recipes) = living_world_recipes.as_ref() {
            recipes.present_pedestrian_animation(world, cvars, &self.hand_tokens).map_err(|error|
                E::LivingWorld(Box::new(crate::selected_living_world_dmo::SelectedLivingWorldDmoError::Pedestrians(error))))?;
        }
        // LivingWorldPresentation::Update's UpdateNoSimPedestrians, then SyncLivingWorldAnim's
        // EvalNoSimPedestrians, for the crowd's imposter targets.
        if let Some(recipes) = living_world_recipes.as_mut() {
            recipes.present_crowd(crate::physics::PHYSICS_STEP_SECONDS, cvars).map_err(|error|
                E::LivingWorld(Box::new(crate::selected_living_world_dmo::SelectedLivingWorldDmoError::Pedestrians(error))))?;
        }
        if let Some(submit_mask) = self.camera_subsequence {
            // The admitted profile above and queued-mask check establish mask3.
            // Native camera gate8289fc5c is updateMask & submitMask, not bit1.
            if 3 & submit_mask != 0 {
                let authority = world.physical_manager_authority();
                let mut camera_services = world.camera_line_services(ground, &mut numeric.line);
                local
                    .try_update_selected_camera(request.dt, authority, &mut camera_services)
                    .map_err(E::Camera)?;
            }
        }
        let camera_manager_time = profile_start.map(|start| start.elapsed());
        let mut second_predictor_time = camera_manager_time;
        if request.physics.schedule == SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 {
            // SimUpdate2 8289fce8: after CameraManager.Update/Wait, the same
            // World predictor publishes camera and any later player queries.
            let predictor = Arc::clone(world.prediction_owner().ok_or(E::MissingPredictor)?);
            let services = world.camera_line_services(ground, &mut numeric.line);
            let queries = services.queries.as_mut().ok_or(E::MissingWorldQueries)?;
            predictor
                .try_update_queries::<Infallible>(queries, services.overlay)
                .map_err(E::Predictor)?;
            world.try_update_step_selector().map_err(E::StepSelector)?;
            second_predictor_time = profile_start.map(|start| start.elapsed());
            // SimUpdate2 0x8289fd3c invokes SkaterManager::PostUpdate after
            // this second predictor. The selected Actor flags/AI gates and
            // PhysOut_Skeleton hips copy precedes the reset/distance gates;
            // reached Ground100/104 then refreshes orientation before later
            // breadcrumb-drop constraints.
            for skater in &skaters {
                skater
                    .actor
                    .try_post_update_breadcrumb_hips(
                        3,
                        cvars,
                        &mut numeric.vector,
                        world_owner,
                        world,
                        ground,
                        &mut numeric.line,
                    )
                    .map_err(E::Breadcrumb)?;
            }
            // SimUpdate2 8289fd40..fd78: updateMask3 has bit0, so the countdown
            // follows PostUpdate on every flags3 tick.
            world
                .sim_rate
                .lock()
                .map_err(|_| E::SimRatePoisoned)?
                .try_count_down()
                .map_err(E::SimRate)?;
        }
        if let Some(start) = profile_start {
            let queued = queued_time.unwrap_or_default();
            let actor = actor_time.unwrap_or_default();
            let pre_solve = pre_solve_time.unwrap_or_default();
            let solver = solver_time.unwrap_or_default();
            let post_solve = post_solve_time.unwrap_or_default();
            let camera = start.elapsed();
            let camera_manager = camera_manager_time.unwrap_or(post_solve);
            let second_predictor = second_predictor_time.unwrap_or(camera_manager);
            for (index, duration) in [
                queued,
                actor.saturating_sub(queued),
                pre_solve.saturating_sub(actor),
                solver.saturating_sub(pre_solve),
                post_solve.saturating_sub(solver),
                camera_manager.saturating_sub(post_solve),
                second_predictor.saturating_sub(camera_manager),
                camera.saturating_sub(second_predictor),
            ]
            .into_iter()
            .enumerate()
            {
                crate::diagnostics::frame_profile::record(index, duration);
            }
            if print_profile {
                use std::io::Write;
                let stderr = std::io::stderr();
                let mut output = std::io::BufWriter::with_capacity(1024, stderr.lock());
                writeln!(
                    output,
                    "diagnostic camera subphases: manager={}ms predictor={}ms breadcrumb={}ms",
                    camera_manager.saturating_sub(post_solve).as_secs_f64() * 1000.0,
                    second_predictor
                        .saturating_sub(camera_manager)
                        .as_secs_f64()
                        * 1000.0,
                    camera.saturating_sub(second_predictor).as_secs_f64() * 1000.0
                )
                .expect("write selected profile");
                writeln!(output,
                "diagnostic selected stages: queued={}ms actor={}ms pre_solve={}ms solver={}ms post_solve={}ms camera={}ms",
                queued.as_secs_f64() * 1000.0,
                actor.saturating_sub(queued).as_secs_f64() * 1000.0,
                pre_solve.saturating_sub(actor).as_secs_f64() * 1000.0,
                solver.saturating_sub(pre_solve).as_secs_f64() * 1000.0,
                post_solve.saturating_sub(solver).as_secs_f64() * 1000.0,
                camera.saturating_sub(post_solve).as_secs_f64() * 1000.0,
            ).expect("write selected profile");
                output.flush().expect("flush selected profile");
            }
        }
        Ok(SelectedQueuedTickOutcome::Completed(Box::new(
            SelectedTickComplete {
                input,
                hands: hands.swap_remove(0),
                before: befores.swap_remove(0),
                counts,
                report,
                after: afters.swap_remove(0),
            },
        )))
    }
}

#[expect(clippy::too_many_arguments, reason = "one skater's frame services over shared owners")]
fn with_actor_services<'s, C, CE, R>(
    skater: &mut SelectedTickSkater<'s>,
    world: &mut PhysicalAssemblyWorld,
    ground: &crate::static_ground_pool::StaticGroundPool,
    hand_tokens: &mut BlendPoseTokenPool,
    local: &SkaterManagerLocalIdentity,
    clamp: &mut C,
    numeric: &mut HostReckoningNumeric,
    run: impl FnOnce(
        &'s ActorConstruction,
        &mut SelectedActorFrameServices<'_, HostReckoningNumeric, LocalSelection<'_, C>>,
    ) -> R,
) -> R
where
    C: FnMut() -> Result<i32, CE>,
{
    let actor = skater.actor;
    let mut selection = LocalSelection {
        actor,
        manager: local,
        clamp,
    };
    let mut writer = MotionForcePhysicsWriter::new_with_walking(world, skater.skeleton, ground);
    run(
        actor,
        &mut SelectedActorFrameServices {
            body: &mut skater.animation.body_services,
            input: &mut skater.animation.input_services,
            motion_writer: &mut writer,
            hand_tokens,
            selection: &mut selection,
            numeric,
            camera: local,
        },
    )
}

#[cfg(test)]
#[path = "selected_player_queued_tick_tests.rs"]
mod queued_tests;
