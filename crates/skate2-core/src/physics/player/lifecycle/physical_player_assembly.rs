//! Source-ordered physical-player assembly projection. It retains the actual
//! supplied physics inputs shared by board and skeleton; it is not an Actor,
//! UCOM object, processed-input implementation or full `HiLOD` constructor.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "paired physical assembly path awaits live Actor scheduling"
    )
)]

use crate::{
    animation::AnimationMatrix, attribute_runtime::AttributeRuntimeVault,
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
};
use skate2_formats::andale::{AndaleHierarchy, AndalePhysicsParameters};
use std::sync::Arc;

use crate::{
    dynamic_assembly_session::{
        DynamicAssemblySession, DynamicAssemblySessionCreateError,
        hook_pool::SimulationHookPoolError,
        skater_pool::{
            SimulationSkaterPoolError, SimulationSkeletonMutationError,
            board_construction::SimulationConstructedBoardError,
        },
        skeleton_normal::{
            drive_owner::SkeletonDriveConstructionError, history_owner::SkeletonHistoryManagerError,
        },
    },
    simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_body_construction::SkateboardBodyConstructionError,
    skateboard_hook::SkateboardHookError,
    skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
    skateboard_wheels::{FRESH_VOLUME_TRANSFORM, HostSkateboardWheelMassNumeric},
    skeleton_assembly_construction::SkeletonAssemblyConstructionError,
    skeleton_drive_attributes::{SkeletonDriveAttributeError, SkeletonDriveAttributes},
    skeleton_drives::{HostSkeletonDriveNumeric, SkeletonDriveCvars},
    skeleton_physics_history::{HostSkeletonPhysicsHistoryNumeric, SkeletonHistoryInput},
};
use crate::{
    dynamic_assembly_session::{
        hook_pool::{SimulationHookAssemblyCandidate, SimulationHookAssemblyHandle},
        skater_pool::{
            SimulationSkaterAssemblyCandidate, SimulationSkaterAssemblyHandle,
            SimulationSkeletonAssemblyCandidate,
        },
    },
    simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession,
    skateboard_body_construction::SkateboardBodyConstruction,
    skateboard_hook::{SkateboardHookConstruction, SkateboardHookDefinition},
    skateboard_truck_drives::SkateboardTruckDrives,
    skeleton_assembly_construction::SkeletonAssemblyConstruction,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{convert::Infallible, fmt};
#[path = "../runtime/physical_player_hilod_prefix.rs"]
pub(crate) mod hilod_prefix;
#[path = "../runtime/physical_player_hilod_tail.rs"]
pub(crate) mod hilod_tail;

#[path = "physical_player_update5_animation.rs"]
pub(crate) mod update5_animation;
#[path = "physical_player_update5_decisions.rs"]
pub(crate) mod update5_decisions;
#[path = "physical_player_update5_finish_output.rs"]
pub(crate) mod update5_finish_output;
#[path = "physical_player_update5_post_category.rs"]
pub(crate) mod update5_post_category;
#[path = "physical_player_update5_push_intents.rs"]
pub(crate) mod update5_push_intents;
#[path = "physical_player_update5_surface_check.rs"]
pub(crate) mod update5_surface_check;
#[path = "physical_player_update5_trick_payload.rs"]
pub(crate) mod update5_trick_payload;

#[path = "../runtime/physical_player_hippy_jump_velocity.rs"]
pub(crate) mod hippy_jump_velocity;
#[path = "../offboard/physical_player_offboard_air_state.rs"]
pub(crate) mod offboard_air_state;
#[path = "../offboard/physical_player_offboard_bad_ground_correction.rs"]
pub(crate) mod offboard_bad_ground_correction;
#[path = "../offboard/physical_player_offboard_controller_start.rs"]
pub(crate) mod offboard_controller_start;
#[path = "../offboard/physical_player_offboard_controller_stop.rs"]
pub(crate) mod offboard_controller_stop;
#[path = "../offboard/physical_player_offboard_feet_ik_session.rs"]
pub(crate) mod offboard_feet_ik_session;
#[path = "../offboard/physical_player_offboard_jump_state.rs"]
pub(crate) mod offboard_jump_state;
#[path = "physical_player_remaining_states.rs"]
pub(crate) mod remaining_states;

#[cfg(test)]
#[path = "physical_player_controller_corpus.rs"]
mod controller_corpus;

#[cfg(test)]
#[path = "../ground/physical_player_ground_position_corpus.rs"]
mod ground_position_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_prepare_corpus.rs"]
mod prepare_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_set_velocity_corpus.rs"]
mod set_velocity_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_standard_corpus.rs"]
mod standard_corpus;

#[cfg(test)]
#[path = "../skeleton/physical_player_skeleton_controller_corpus.rs"]
mod skeleton_controller_corpus;

#[cfg(test)]
#[path = "../air/physical_player_jump_corpus.rs"]
mod jump_corpus;

#[cfg(test)]
#[path = "physical_player_assembly_corpus.rs"]
pub(crate) mod corpus;

#[cfg(test)]
#[path = "../interaction/physical_player_contact_list_corpus.rs"]
mod contact_list_corpus;

#[cfg(test)]
#[path = "../runtime/physical_player_collision_corpus.rs"]
mod collision_corpus;

#[cfg(test)]
#[path = "../skeleton/physical_player_roots_corpus.rs"]
mod roots_corpus;

#[cfg(test)]
#[path = "physical_player_current_animation_corpus.rs"]
mod current_animation_corpus;

#[cfg(test)]
#[path = "../board/physical_player_board_forces_corpus.rs"]
mod board_forces_corpus;
#[cfg(test)]
#[path = "../board/physical_player_board_steering_corpus.rs"]
mod board_steering_corpus;
#[cfg(test)]
#[path = "../board/physical_player_board_surface_corpus.rs"]
mod board_surface_corpus;
#[cfg(test)]
#[path = "../ground/physical_player_ground_force_tail_corpus.rs"]
mod ground_force_tail_corpus;
#[cfg(test)]
#[path = "../ground/physical_player_grounded_corpus.rs"]
mod grounded_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_ik_complete_corpus.rs"]
mod ik_complete_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_landing_corpus.rs"]
mod landing_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_landing_spring_corpus.rs"]
mod landing_spring_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_motion_corpus.rs"]
mod motion_corpus;
#[cfg(test)]
#[path = "../output/physical_player_phys_out_corpus.rs"]
mod phys_out_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_post_records_corpus.rs"]
mod post_records_corpus;
#[cfg(test)]
#[path = "../runtime/physical_player_surface_motion_corpus.rs"]
mod surface_motion_corpus;
#[cfg(test)]
#[path = "../board/physical_player_wheel_lines_corpus.rs"]
mod wheel_lines_corpus;
#[cfg(test)]
#[path = "../board/physical_player_wheel_material_corpus.rs"]
mod wheel_material_corpus;
#[cfg(test)]
#[path = "../board/physical_player_wheel_support_corpus.rs"]
mod wheel_support_corpus;

#[cfg(test)]
#[path = "physical_player_assembly_tests.rs"]
pub(crate) mod tests;

/// Current Entity flags are sampled separately at EACH `GetPhysicsIsland` call:
/// board constructor before Init, skeleton after its Assembly construction.
/// The caller supplies the real selected manager; singleton/Actor creation and
/// visual-Island routing remain outside this simulation-only projection.
pub(crate) trait PhysicalPlayerAssemblyServices:
    crate::skateboard_spawn::FallibleSkateboardSpawnServices
{
    fn current_entity_flags(&mut self) -> u32;
}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerAssemblyError {
    HiLodPrefixPoisoned,
    PrepareHistoryPoisoned,
    PrepareHistoryAlreadyInitialized,
    ProcessedPoisoned,
    ProcessedAlreadyInitialized,
    PhysOutPoisoned,
    ConditionersPoisoned,
    Conditioners(crate::physical_player_phys_out::conditioners::ConstructionError<Infallible>),
    GrindAirPoisoned,
    HandPlantPoisoned,
    GrindAir(crate::physical_player_grind_air_adjust::GrindAirConstructError<Infallible>),
    SurfacePhysicsPoisoned,
    SurfacePhysics(crate::skateboard_processed_surface::SkateboardSurfaceConstructionError<crate::skateboard_slide_collision_attributes::SkateboardSlideCollisionAttributeError>),
    PhysOut(crate::physical_player_phys_out::construct::ConcretePhysOutError<Infallible>),
    ReckoningPoisoned,
    TrajectoryPoisoned,
    Reckoning(crate::physical_player_reckoning_owner::PhysicalPlayerReckoningConstructError),
    BoardResetPoisoned,
    PumpingPoisoned,
    AnimOutPoisoned,
    Pumping(crate::physical_player_pumping::PumpingError),
    StateChangerPoisoned,
    StateChanger(crate::physical_player_state_changer::StateChangerError),
    SpeedWobble(crate::skateboard_force_runtime::speed_wobble::SkateboardSpeedWobbleError),
    BoardEntry(crate::dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError),
    WrongPhase(PhysicalPlayerAssemblyPhase),
    VisualIslandUnsupported {
        flags: u32,
    },
    OptionalWheelDrivesUnsupported {
        value: u8,
    },
    BoardDefinition(SkateboardBodyConstructionError),
    Board(SimulationConstructedBoardError),
    Pool(SimulationSkaterPoolError),
    Island(SimulationPhysicalBodyError),
    HookDefinition(SkateboardHookError<Infallible>),
    HookSession(DynamicAssemblySessionCreateError),
    Hook(SimulationHookPoolError),
    SkeletonDefinition(SkeletonAssemblyConstructionError),
    SkeletonHandoff,
    SkeletonIk(crate::dynamic_assembly_session::skater_pool::ik_construction::SimulationSkeletonIkConstructionError),
    SkeletonProperties(SimulationSkeletonMutationError<Infallible, Infallible>),
    SkeletonNormal(
        SimulationSkeletonMutationError<
            crate::attribute_runtime::AttributeRuntimeError,
            Infallible,
        >,
    ),
}
impl fmt::Display for PhysicalPlayerAssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HiLodPrefixPoisoned => f.write_str("HiLOD constructor prefix lock poisoned"),
            Self::ConditionersPoisoned => f.write_str("paired conditioner lock poisoned"),
            Self::Conditioners(error) => write!(f, "paired conditioner constructor: {error:?}"),
            Self::GrindAirPoisoned => f.write_str("paired GrindAirAdjust lock poisoned"),
            Self::HandPlantPoisoned => f.write_str("paired HandPlantManager lock poisoned"),
            Self::GrindAir(error) => write!(f, "paired GrindAirAdjust constructor: {error}"),
            Self::PrepareHistoryPoisoned => {
                f.write_str("paired prepared history construction lock poisoned")
            }
            Self::PrepareHistoryAlreadyInitialized => {
                f.write_str("paired prepared history already initialized")
            }
            Self::ProcessedPoisoned => f.write_str("paired Processed constructor lock poisoned"),
            Self::PhysOutPoisoned => f.write_str("paired PhysOut constructor lock poisoned"),
            Self::SurfacePhysicsPoisoned => {
                f.write_str("paired SurfacePhysics constructor lock poisoned")
            }
            Self::SurfacePhysics(error) => {
                write!(f, "paired SurfacePhysics construction: {error:?}")
            }
            Self::PhysOut(error) => write!(f, "paired PhysOut construction: {error:?}"),
            Self::ProcessedAlreadyInitialized => {
                f.write_str("paired Processed constructor already initialized")
            }
            Self::ReckoningPoisoned => f.write_str("paired Reckoning construction lock poisoned"),
            Self::TrajectoryPoisoned => f.write_str("paired trajectory constructor lock poisoned"),
            Self::Reckoning(e) => write!(f, "paired Reckoning construction: {e}"),
            Self::BoardResetPoisoned => f.write_str("board collision Reset state lock poisoned"),
            Self::PumpingPoisoned => f.write_str("paired Pumping owner lock poisoned"),
            Self::AnimOutPoisoned => f.write_str("paired AnimOut owner lock poisoned"),
            Self::Pumping(error) => write!(f, "paired Pumping construction: {error:?}"),
            Self::StateChangerPoisoned => f.write_str("paired StateChanger owner lock poisoned"),
            Self::StateChanger(error) => write!(f, "paired StateChanger construction: {error:?}"),
            Self::SpeedWobble(e) => write!(f, "physical board speed wobble: {e:?}"),
            Self::BoardEntry(e) => write!(f, "physical board entry owner: {e}"),
            Self::WrongPhase(phase) => write!(f, "physical construction cannot run in {phase:?}"),
            Self::VisualIslandUnsupported { flags } => write!(
                f,
                "visual Island routing unsupported for Entity flags {flags:#x}"
            ),
            Self::OptionalWheelDrivesUnsupported { value } => write!(
                f,
                "optional wheel-drive creation unsupported for byte {value:#x}"
            ),
            Self::BoardDefinition(e) => write!(f, "physical board definition: {e}"),
            Self::Board(e) => write!(f, "physical board construction: {e}"),
            Self::Pool(e) => write!(f, "physical assembly pool: {e}"),
            Self::Island(e) => write!(f, "physical Island: {e}"),
            Self::HookDefinition(e) => write!(f, "physical hook definition: {e}"),
            Self::HookSession(e) => write!(f, "physical hook session: {e}"),
            Self::Hook(e) => write!(f, "physical hook: {e}"),
            Self::SkeletonDefinition(e) => write!(f, "physical skeleton definition: {e}"),
            Self::SkeletonHandoff => {
                f.write_str("skeleton construction is not ready for candidate handoff")
            }
            Self::SkeletonProperties(e) => write!(f, "physical skeleton properties: {e}"),
            Self::SkeletonIk(e) => write!(f, "physical skeleton IK: {e}"),
            Self::SkeletonNormal(e) => write!(f, "physical skeleton normal setup: {e}"),
        }
    }
}
impl std::error::Error for PhysicalPlayerAssemblyError {}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerSkeletonFinishError<C, H> {
    GrabPoisoned,
    Grab(crate::physical_player_grab_spline::GrabConstructorError),
    GrindPoisoned,
    Grind(crate::physical_player_grind_manager::GrindConstructorError),
    FootPlantPoisoned,
    FootPlant(crate::physical_player_footplant::FootPlantError),
    WipeoutPoisoned,
    Wipeout(crate::physical_player_wipeout::WipeoutOwnerError),
    WrongPhase(PhysicalPlayerAssemblyPhase),
    Drives(SkeletonDriveConstructionError<SkeletonDriveAttributeError, Infallible, C>),
    Pool(SimulationSkaterPoolError),
    History(SkeletonHistoryManagerError<H>),
}
impl<C: fmt::Debug, H: fmt::Debug> fmt::Display for PhysicalPlayerSkeletonFinishError<C, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FootPlantPoisoned => f.write_str("paired FootPlant constructor lock poisoned"),
            Self::GrabPoisoned => f.write_str("paired grab-spline lock poisoned"),
            Self::Grab(error) => write!(f, "paired grab-spline construction: {error}"),
            Self::GrindPoisoned => f.write_str("paired GrindManager lock poisoned"),
            Self::Grind(error) => write!(f, "paired GrindManager constructor: {error}"),
            Self::FootPlant(error) => write!(f, "paired FootPlant construction: {error:?}"),
            Self::WipeoutPoisoned => f.write_str("paired Wipeout owner lock poisoned"),
            Self::Wipeout(error) => write!(f, "paired Wipeout construction: {error:?}"),
            Self::WrongPhase(phase) => {
                write!(f, "physical skeleton completion cannot run in {phase:?}")
            }
            Self::Drives(e) => write!(f, "physical skeleton drives: {e}"),
            Self::Pool(e) => write!(f, "physical skeleton pool: {e}"),
            Self::History(e) => write!(f, "physical skeleton history: {e}"),
        }
    }
}
impl<C: fmt::Debug, H: fmt::Debug> std::error::Error for PhysicalPlayerSkeletonFinishError<C, H> {}

pub(crate) type PhysicalAssemblyWorld =
    SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;

/// Actual caller-selected records, not prebuilt board/skeleton definitions.
/// The Actor constructor projection uses its evaluated bind pose and actual
/// hierarchy. Other explicit callers may supply distinct mapping/pose hierarchies.
#[derive(Clone)]
pub(crate) struct PhysicalPlayerAssemblyInputs {
    pub(crate) cvars: crate::input_cvar_runtime::InputCvarReferences,
    pub(crate) vault: Arc<AttributeRuntimeVault>,
    pub(crate) hierarchy: Arc<AndaleHierarchy>,
    pub(crate) parameters: Arc<AndalePhysicsParameters>,
    pub(crate) pose_hierarchy: Arc<AndaleHierarchy>,
    pub(crate) pose_matrices: Arc<[AnimationMatrix]>,
    pub(crate) spawn: [f32; 16],
    pub(crate) hat_type: u32,
    pub(crate) bouncy_ragdoll: u8,
    pub(crate) is_human: u8,
}

/// Shared native Entity-equality projection, issued only by the real paired
/// construction below before either body is created. The retained input and
/// manager owner make this meaningful construction ownership, not an empty token.
/// Distinct constructions always allocate distinct owners, even for same assets.
pub(crate) struct PhysicalPlayerAssemblyOwner {
    /// PC mode state belongs to this same Actor physical allocation. Replacing
    /// character recipes does not create another player runtime or body owner.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) skate3: std::sync::Mutex<Option<Box<crate::skate3_native::physics::player::Skate3Player>>>,
    #[cfg(feature = "skate3-runtime")]
    pub(crate) skate3_exit: std::sync::Mutex<Option<crate::skate3_native::physics::player::exit::PlayerExit>>,
    /// Actual mAnimOutPhysIn+2b4, assigned by `SetPhysUpdateData82dfe688`.
    pub(crate) animation_physics_input: std::sync::Mutex<Option<Arc<std::sync::Mutex<crate::actor_construction::anim_out_phys_in::AnimOutPhysInOwner>>>>,
    pub(crate) actor_query: std::sync::Mutex<Option<Arc<crate::actor_construction::physical_mount::ActorPhysicalQueryBinding>>>,
    /// Host completion proof, distinct from the source suffix returning.
    pub(crate) skeleton_constructor_complete: std::sync::atomic::AtomicBool,
    prefix: std::sync::Mutex<Option<hilod_prefix::PhysicalHiLodPrefix>>,
    physics_debug_system: std::sync::Mutex<Option<Box<hilod_prefix::PhysicsDebugSystem>>>,
    tail: std::sync::Mutex<hilod_tail::PhysicalHiLodTail>,
    world_services: Option<Arc<crate::simulation_assembly_backend::constraint_session::world_services::PhysicalWorldServices>>,
    network_query: Option<Arc<crate::simulation_assembly_backend::constraint_session::world_services::PhysicalWorldServices>>,
    remaining_states: std::sync::Mutex<remaining_states::RemainingStates>,
    anim_out: std::sync::Mutex<Option<crate::physical_player_anim_out::PhysicalAnimOut>>,
    grab_spline: std::sync::Mutex<Option<crate::physical_player_grab_spline::PlayerGrabSpline>>,
    remaining: std::sync::Mutex<
        crate::physical_player_remaining_constructor_owners::RemainingConstructorOwners,
    >,
    handplant: std::sync::Mutex<Option<crate::physical_player_handplant::HandPlantManager>>,
    grind: std::sync::Mutex<Option<crate::physical_player_grind_manager::GrindManager>>,
    conditioners:
        std::sync::Mutex<Option<crate::physical_player_phys_out::conditioners::Conditioners>>,
    grind_air:
        std::sync::Mutex<Option<crate::physical_player_grind_air_adjust::GrindAirAdjustOwner>>,
    trajectories:
        std::sync::Mutex<crate::physical_player_trajectory_owners::PhysicalTrajectoryOwners>,
    footplant: std::sync::Mutex<crate::physical_player_footplant::PhysicalPlayerFootPlantOwner>,
    pumping: Arc<std::sync::Mutex<crate::physical_player_pumping::PhysicalPlayerPumpingOwner>>,
    pumping_attributes: std::sync::Mutex<crate::physical_player_pumping::attributes::PumpingVaultAttributes>,
    wipeout: std::sync::Mutex<crate::physical_player_wipeout::PhysicalPlayerWipeoutOwner>,
    state_changer:
        std::sync::Mutex<crate::physical_player_state_changer::PhysicalPlayerStateChangerOwner>,
    physics_state:
        std::sync::Mutex<crate::physical_player_physics_state::PhysicalPlayerPhysicsStateOwner>,
    skeleton_controller: std::sync::Mutex<crate::skeleton_controller::SkeletonController>,
    skateboard_controller: std::sync::Mutex<crate::skateboard_controller::SkateboardController>,
    prepare_velocity:
        std::sync::Mutex<crate::physical_player_prepare_velocity::PhysicalPlayerPrepareVelocity>,
    heading_attributes: crate::simulation_assembly_backend::constraint_session::physical_heading::runtime::HeadingAttributes,
    // Retained immutable global instances, with no eager tuning reads.
    jump_attributes: std::sync::Mutex<
        crate::skeleton_processed_contact::jump_events::SkeletonJumpVaultAttributes,
    >,
    reckoning:
        std::sync::Mutex<crate::physical_player_reckoning_owner::PhysicalPlayerReckoningOwner>,
    inputs: PhysicalPlayerAssemblyInputs,
    manager: SimulationManagerContactAuthority,
    // Host occurrence storage only until the explicit selected HiLOD output
    // construction stage succeeds. No component defaults are published here.
    phys_out: std::sync::Mutex<crate::physical_player_phys_out::PhysicalPlayerPhysOut>,
    surface_physics:
        std::sync::Mutex<Option<crate::skateboard_processed_surface::SkateboardProcessedSurface>>,
}

impl PhysicalPlayerAssemblyOwner {
    pub(crate) fn tail(&self) -> &std::sync::Mutex<hilod_tail::PhysicalHiLodTail> {
        &self.tail
    }
    pub(crate) fn world_services(&self) -> Option<&Arc<crate::simulation_assembly_backend::constraint_session::world_services::PhysicalWorldServices>>{
        self.world_services.as_ref()
    }
    pub(crate) fn network_query(&self) -> Option<&Arc<crate::simulation_assembly_backend::constraint_session::world_services::PhysicalWorldServices>>{
        self.network_query.as_ref()
    }
    pub(crate) fn remaining_states(&self) -> &std::sync::Mutex<remaining_states::RemainingStates> {
        &self.remaining_states
    }
    pub(crate) fn handplant(
        &self,
    ) -> &std::sync::Mutex<Option<crate::physical_player_handplant::HandPlantManager>> {
        &self.handplant
    }
    pub(crate) fn remaining(
        &self,
    ) -> &std::sync::Mutex<
        crate::physical_player_remaining_constructor_owners::RemainingConstructorOwners,
    > {
        &self.remaining
    }
    pub(crate) fn grind(
        &self,
    ) -> &std::sync::Mutex<Option<crate::physical_player_grind_manager::GrindManager>> {
        &self.grind
    }
    pub(crate) fn grind_air(
        &self,
    ) -> &std::sync::Mutex<Option<crate::physical_player_grind_air_adjust::GrindAirAdjustOwner>>
    {
        &self.grind_air
    }
    pub(crate) fn trajectories(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_trajectory_owners::PhysicalTrajectoryOwners> {
        &self.trajectories
    }
    pub(crate) fn footplant(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_footplant::PhysicalPlayerFootPlantOwner> {
        &self.footplant
    }
    pub(crate) fn pumping(
        &self,
    ) -> &Arc<std::sync::Mutex<crate::physical_player_pumping::PhysicalPlayerPumpingOwner>> {
        &self.pumping
    }
    pub(crate) fn pumping_attributes(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_pumping::attributes::PumpingVaultAttributes> {
        &self.pumping_attributes
    }
    pub(crate) fn prefix(&self) -> &std::sync::Mutex<Option<hilod_prefix::PhysicalHiLodPrefix>> {
        &self.prefix
    }
    pub(crate) fn anim_out(
        &self,
    ) -> &std::sync::Mutex<Option<crate::physical_player_anim_out::PhysicalAnimOut>> {
        &self.anim_out
    }
    pub(crate) fn physics_state(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_physics_state::PhysicalPlayerPhysicsStateOwner>
    {
        &self.physics_state
    }
    pub(crate) fn wipeout(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_wipeout::PhysicalPlayerWipeoutOwner> {
        &self.wipeout
    }
    pub(crate) fn state_changer(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_state_changer::PhysicalPlayerStateChangerOwner>
    {
        &self.state_changer
    }
    /// Checked host lifetime guard, not an implicit native removal/cvar override.
    pub(crate) fn require_controller_drives_removed(
        &self,
    ) -> Result<(), SimulationSkaterPoolError> {
        let controller = self
            .skateboard_controller
            .lock()
            .map_err(|_| SimulationSkaterPoolError::ControllerPoisoned)?;
        let slots = [controller.drive(0).is_some(), controller.drive(1).is_some()];
        if slots.iter().any(|present| *present) {
            return Err(SimulationSkaterPoolError::ControllerDrivesRemain { slots });
        }
        Ok(())
    }
    pub(crate) fn skateboard_controller(
        &self,
    ) -> &std::sync::Mutex<crate::skateboard_controller::SkateboardController> {
        &self.skateboard_controller
    }
    pub(crate) fn skeleton_controller(
        &self,
    ) -> &std::sync::Mutex<crate::skeleton_controller::SkeletonController> {
        &self.skeleton_controller
    }
    pub(crate) fn prepare_velocity(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_prepare_velocity::PhysicalPlayerPrepareVelocity>
    {
        &self.prepare_velocity
    }
    pub(crate) fn grab_spline(
        &self,
    ) -> &std::sync::Mutex<Option<crate::physical_player_grab_spline::PlayerGrabSpline>> {
        &self.grab_spline
    }
    pub(crate) fn heading_attributes(&self) -> &crate::simulation_assembly_backend::constraint_session::physical_heading::runtime::HeadingAttributes{
        &self.heading_attributes
    }
    pub(crate) fn jump_attributes(
        &self,
    ) -> &std::sync::Mutex<
        crate::skeleton_processed_contact::jump_events::SkeletonJumpVaultAttributes,
    > {
        &self.jump_attributes
    }
    pub(crate) fn reckoning(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_reckoning_owner::PhysicalPlayerReckoningOwner>
    {
        &self.reckoning
    }
    pub(crate) fn surface_physics(
        &self,
    ) -> &std::sync::Mutex<Option<crate::skateboard_processed_surface::SkateboardProcessedSurface>>
    {
        &self.surface_physics
    }
    pub(crate) fn phys_out(
        &self,
    ) -> &std::sync::Mutex<crate::physical_player_phys_out::PhysicalPlayerPhysOut> {
        &self.phys_out
    }
    pub(crate) fn conditioners(
        &self,
    ) -> &std::sync::Mutex<Option<crate::physical_player_phys_out::conditioners::Conditioners>>
    {
        &self.conditioners
    }
    pub(crate) fn inputs(&self) -> &PhysicalPlayerAssemblyInputs {
        &self.inputs
    }
    pub(crate) fn manager(&self) -> SimulationManagerContactAuthority {
        self.manager
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PhysicalPlayerAssemblyPhase {
    Fresh,
    SelectedPrepareHistory,
    SelectedProcessed,
    CompletePhysOut,
    SurfacePhysics,
    Conditioners,
    GrindAir,
    HandPlant,
    Grind,
    SelectedReckoning,
    OnboardTrajectory,
    OffboardTrajectory,
    SelectedPumping,
    AnimOut,
    GrabSpline,
    BoardDefinition,
    BoardRouting,
    BoardAdmission,
    BoardMembership,
    BoardTruckTransforms,
    BoardDrives,
    OptionalWheelDrives,
    HookDefinition,
    HookAdmission,
    HookConnection,
    BoardPlacement,
    BoardRegistration,
    BoardReady,
    SkeletonDefinition,
    SelectedStateChanger,
    SkeletonIk,
    SkeletonRouting,
    SkeletonAdmission,
    SkeletonMembership,
    SkeletonRegistration,
    SkeletonNormal,
    SkeletonTeleportReset,
    SkeletonReady,
    SkeletonDrives,
    SkeletonPlacement,
    SkeletonHistory,
    Complete,
    TearingDown,
    Removed,
}

/// Retains completed construction prefixes and the SAME exclusively selected
/// manager. It does not atomically prebuild/admit a pair or refund issued IDs.
pub(crate) struct PhysicalPlayerAssemblyConstruction<'world> {
    world: &'world mut PhysicalAssemblyWorld,
    state: PhysicalPlayerAssemblyConstructionState,
}

/// The unique in-progress constructor state, without a borrowed world.
/// Moving it never resets construction or retries a failed source stage.
pub(crate) struct PhysicalPlayerAssemblyConstructionState {
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    phase: PhysicalPlayerAssemblyPhase,
    board_started: bool,
    skeleton_started: bool,
    board_construction: Option<Box<SkateboardBodyConstruction>>,
    board_force_runtime: Option<Box<crate::skateboard_force_runtime::SkateboardForceRuntime>>,
    surface_attributes: Option<
        crate::skateboard_slide_collision_attributes::SkateboardSlideCollisionVaultAttributes,
    >,
    board_candidate: Option<SimulationSkaterAssemblyCandidate>,
    board: Option<SimulationSkaterAssemblyHandle>,
    board_drives: Option<SkateboardTruckDrives>,
    hook_construction: Option<SkateboardHookConstruction>,
    hook_session:
        Option<DynamicAssemblySession<SkateboardSimulationBackend, SkateboardHookDefinition>>,
    hook_candidate:
        Option<SimulationHookAssemblyCandidate<AssemblyPartMaterial, SkateboardHookDefinition>>,
    hook: Option<SimulationHookAssemblyHandle>,
    skeleton_construction: Option<Box<SkeletonAssemblyConstruction>>,
    skeleton_candidate: Option<SimulationSkeletonAssemblyCandidate>,
    skeleton: Option<SimulationSkaterAssemblyHandle>,
    skeleton_drive_attributes: Option<SkeletonDriveAttributes>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhysicalConstructionBindError {
    ForeignWorld,
}
impl std::fmt::Display for PhysicalConstructionBindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignWorld => f.write_str("physical construction belongs to another world"),
        }
    }
}
impl PhysicalPlayerAssemblyConstructionState {
    /// Reborrow the original manager. Rejection returns all state untouched.
    #[expect(
        clippy::result_large_err,
        reason = "return the unique retained state without cloning or reallocating on rejection"
    )]
    pub(crate) fn try_bind(
        self,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<PhysicalPlayerAssemblyConstruction<'_>, (PhysicalConstructionBindError, Self)> {
        if self.owner.manager() != world.physical_manager_authority() {
            return Err((PhysicalConstructionBindError::ForeignWorld, self));
        }
        Ok(PhysicalPlayerAssemblyConstruction { world, state: self })
    }
}

impl<'world> PhysicalPlayerAssemblyConstruction<'world> {
    /// Creates one unique meaningful owner BEFORE constructing either Body.
    /// No Entity flag decision is made here: execution samples at the actual
    /// native constructor call sites before using this selected world.
    pub(crate) fn new(
        world: &'world mut PhysicalAssemblyWorld,
        inputs: PhysicalPlayerAssemblyInputs,
    ) -> Self {
        let owner = Arc::new(PhysicalPlayerAssemblyOwner {
            #[cfg(feature = "skate3-runtime")]
            skate3: std::sync::Mutex::new(None),
            #[cfg(feature = "skate3-runtime")]
            skate3_exit: std::sync::Mutex::new(None),
            animation_physics_input: std::sync::Mutex::new(None),
            actor_query: std::sync::Mutex::new(None),
            skeleton_constructor_complete: std::sync::atomic::AtomicBool::new(false),
            prefix: std::sync::Mutex::new(None),
            physics_debug_system: std::sync::Mutex::new(None),
            tail: std::sync::Mutex::new(hilod_tail::PhysicalHiLodTail::from_sim_zeroed_allocation()),
            world_services: world.physical_world_services().cloned(),
            network_query: world.physical_world_services().cloned(),
            remaining_states: std::sync::Mutex::new(remaining_states::RemainingStates::default()),
            anim_out: std::sync::Mutex::new(None),
            grab_spline: std::sync::Mutex::new(None),
            remaining: std::sync::Mutex::new(crate::physical_player_remaining_constructor_owners::RemainingConstructorOwners::default()),
            handplant: std::sync::Mutex::new(None),
            grind: std::sync::Mutex::new(None),
            conditioners: std::sync::Mutex::new(None),
            grind_air: std::sync::Mutex::new(None),
            trajectories: std::sync::Mutex::new(crate::physical_player_trajectory_owners::PhysicalTrajectoryOwners::default()),
            footplant: std::sync::Mutex::new(crate::physical_player_footplant::PhysicalPlayerFootPlantOwner::new()),
            pumping: Arc::new(std::sync::Mutex::new(crate::physical_player_pumping::PhysicalPlayerPumpingOwner::new())),
            pumping_attributes: std::sync::Mutex::new(crate::physical_player_pumping::attributes::PumpingVaultAttributes::bind(&inputs.vault)),
            wipeout: std::sync::Mutex::new(crate::physical_player_wipeout::PhysicalPlayerWipeoutOwner::new()),
            state_changer: std::sync::Mutex::new(crate::physical_player_state_changer::PhysicalPlayerStateChangerOwner::new()),
            // Actor825ff2d8 selects Sim, whose category OR0x100 clears the HiLOD allocation.
            physics_state: std::sync::Mutex::new(crate::physical_player_physics_state::PhysicalPlayerPhysicsStateOwner::from_sim_zeroed_allocation()),
            skeleton_controller: std::sync::Mutex::new(crate::skeleton_controller::SkeletonController::new()),
            skateboard_controller: std::sync::Mutex::new(crate::skateboard_controller::SkateboardController::new()),
            prepare_velocity: std::sync::Mutex::new(crate::physical_player_prepare_velocity::PhysicalPlayerPrepareVelocity::uninitialized()),
            heading_attributes: crate::simulation_assembly_backend::constraint_session::physical_heading::runtime::HeadingAttributes::bind(&inputs.vault),
            jump_attributes: std::sync::Mutex::new(crate::skeleton_processed_contact::jump_events::SkeletonJumpVaultAttributes::bind_global(&inputs.vault)),
            reckoning: std::sync::Mutex::new(
                crate::physical_player_reckoning_owner::PhysicalPlayerReckoningOwner::new(),
            ),
            manager: world.physical_manager_authority(),
            inputs,
            phys_out: std::sync::Mutex::new(
                crate::physical_player_phys_out::PhysicalPlayerPhysOut::new(),
            ),
            surface_physics: std::sync::Mutex::new(None),
        });
        Self {
            world,
            state: PhysicalPlayerAssemblyConstructionState {
                board_force_runtime: None,
                surface_attributes: None,
                board_construction: Some(Box::new(
                    SkateboardBodyConstruction::new_for_physical_player(Arc::clone(&owner)),
                )),
                owner,
                phase: PhysicalPlayerAssemblyPhase::Fresh,
                board_started: false,
                skeleton_started: false,
                board_candidate: None,
                board: None,
                board_drives: None,
                hook_construction: None,
                hook_session: None,
                hook_candidate: None,
                hook: None,
                skeleton_construction: None,
                skeleton_candidate: None,
                skeleton: None,
                skeleton_drive_attributes: None,
            },
        }
    }

    /// Release the temporary world borrow, retaining every constructor prefix.
    /// This does not run teardown or turn a failed stage into a retryable one.
    pub(crate) fn into_retained(self) -> PhysicalPlayerAssemblyConstructionState {
        self.state
    }

    pub(crate) fn owner(&self) -> &Arc<PhysicalPlayerAssemblyOwner> {
        &self.state.owner
    }
    pub(crate) fn phase(&self) -> PhysicalPlayerAssemblyPhase {
        self.state.phase
    }
    pub(crate) fn world(&self) -> &PhysicalAssemblyWorld {
        self.world
    }
    pub(crate) fn board(&self) -> Option<&SimulationSkaterAssemblyHandle> {
        self.state.board.as_ref()
    }
    pub(crate) fn skeleton(&self) -> Option<&SimulationSkaterAssemblyHandle> {
        self.state.skeleton.as_ref()
    }

    /// Whole board assembly projection of 82b387a4..82b38918. Safe host
    /// failures keep earlier members/candidates and never replay this sequence.
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered retained construction with explicit failure ownership"
    )]
    pub(crate) fn try_construct_board(
        &mut self,
        services: &mut impl PhysicalPlayerAssemblyServices,
    ) -> Result<(), PhysicalPlayerAssemblyError> {
        use PhysicalPlayerAssemblyError as Error;
        use PhysicalPlayerAssemblyPhase as Phase;
        if self.state.board_started || self.state.phase != Phase::Fresh {
            return Err(Error::WrongPhase(self.state.phase));
        }
        self.state.board_started = true;
        *self
            .state
            .owner
            .prefix
            .lock()
            .map_err(|_| Error::HiLodPrefixPoisoned)? =
            Some(hilod_prefix::PhysicalHiLodPrefix::construct_in_sim_allocation());
        self.state
            .owner
            .physics_state()
            .lock()
            .map_err(|_| Error::HiLodPrefixPoisoned)?
            .initialize_constructor_scalars();
        // Selected HiLOD Reckoning3ff8/4008 precedes outer Skateboard413c.
        // Stable selected processed storage already exists in this SAME owner.
        // Omitted HiLOD components/full Processed construction are not implied.
        self.state.phase = Phase::SelectedPrepareHistory;
        self.state
            .owner
            .prepare_velocity()
            .lock()
            .map_err(|_| Error::PrepareHistoryPoisoned)?
            .initialize_constructor()
            .map_err(|_| Error::PrepareHistoryAlreadyInitialized)?;
        self.state
            .owner
            .tail()
            .lock()
            .map_err(|_| Error::HiLodPrefixPoisoned)?
            .initialize_early_line_results();
        self.state.phase = Phase::SelectedProcessed;
        self.state
            .owner
            .reckoning()
            .lock()
            .map_err(|_| Error::ReckoningPoisoned)?
            .processed()
            .lock()
            .map_err(|_| Error::ProcessedPoisoned)?
            .try_initialize_constructor_fields_in_sim_allocation()
            .map_err(|_| Error::ProcessedAlreadyInitialized)?;
        // HiLOD82e03f20 completes PhysOut and publishes +2ac after ProcessedPhysIn,
        // before Reckoning82e04008 and either physical Body. This selected host
        // route takes successful allocations; native NULL child behavior remains
        // represented by the complete constructor's allocation contract.
        self.state.phase = Phase::CompletePhysOut;
        self.state
            .owner
            .phys_out()
            .lock()
            .map_err(|_| Error::PhysOutPoisoned)?
            .try_construct_complete(|_| {
                Ok(crate::physical_player_phys_out::construct::PhysOutAllocation::Host)
            })
            .map_err(Error::PhysOut)?;
        // HiLOD82e03f58: all conditioners bind this same, completed PhysOut.
        self.state.phase = Phase::Conditioners;
        {
            let mut output = self
                .state
                .owner
                .phys_out()
                .lock()
                .map_err(|_| Error::PhysOutPoisoned)?;
            let mut slot = self
                .state
                .owner
                .conditioners
                .lock()
                .map_err(|_| Error::ConditionersPoisoned)?;
            *slot = Some(
                crate::physical_player_phys_out::conditioners::Conditioners::before_constructor(
                    &output,
                ),
            );
            slot.as_mut()
                .expect("retained conditioners")
                .try_construct(
                    &mut output,
                    &self.state.owner.inputs().vault,
                    &self.state.owner.inputs().cvars,
                    &mut |_| Ok(true),
                )
                .map_err(Error::Conditioners)?;
        }
        // HiLOD82e03f90: retain partial SurfacePhysics before initializing its materials.
        self.state.phase = Phase::SurfacePhysics;
        self.state.surface_attributes = Some(
            crate::skateboard_slide_collision_attributes::SkateboardSlideCollisionVaultAttributes::bind(&self.state.owner.inputs().vault));
        {
            let attributes = self
                .state
                .surface_attributes
                .as_mut()
                .expect("surface constructor provider");
            let mut surface = self
                .state
                .owner
                .surface_physics()
                .lock()
                .map_err(|_| Error::SurfacePhysicsPoisoned)?;
            *surface = Some(
                crate::skateboard_processed_surface::SkateboardProcessedSurface::bind(attributes),
            );
            surface
                .as_mut()
                .expect("retained source surface")
                .try_initialize_materials(attributes)
                .map_err(Error::SurfacePhysics)?;
        }
        // HiLOD82e03fc4: source allocation is retained across InitTargets failure.
        self.state.phase = Phase::GrindAir;
        {
            let processed = Arc::clone(
                self.state
                    .owner
                    .reckoning()
                    .lock()
                    .map_err(|_| Error::ReckoningPoisoned)?
                    .processed(),
            );
            let mut slot = self
                .state
                .owner
                .grind_air
                .lock()
                .map_err(|_| Error::GrindAirPoisoned)?;
            *slot = Some(
                crate::physical_player_grind_air_adjust::GrindAirAdjustOwner::constructor_prefix(
                    crate::physical_player_trajectory_owners::TrajectoryOwnerBindings::new(
                        &self.state.owner,
                        processed,
                    ),
                ),
            );
            slot.as_mut()
                .expect("retained GrindAirAdjust")
                .try_init_targets(
                    &self.state.owner.inputs().vault,
                    &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                )
                .map_err(Error::GrindAir)?;
        }
        self.state.phase = Phase::SelectedReckoning;
        self.state
            .owner
            .reckoning()
            .lock()
            .map_err(|_| Error::ReckoningPoisoned)?
            .try_construct(&self.state.owner.inputs().vault)
            .map_err(Error::Reckoning)?;
        // Native calls82e04030 and82e04068, before Pumping and either Body.
        // Bind the SAME Processed storage; these constructors launch no jobs.
        let processed = Arc::clone(
            self.state
                .owner
                .reckoning()
                .lock()
                .map_err(|_| Error::ReckoningPoisoned)?
                .processed(),
        );
        self.state.phase = Phase::OnboardTrajectory;
        {
            use crate::physical_player_trajectory_owners::{
                OnboardTrajectoryOwner, TrajectoryOwnerBindings,
            };
            self.state
                .owner
                .trajectories()
                .lock()
                .map_err(|_| Error::TrajectoryPoisoned)?
                .onboard = Some(OnboardTrajectoryOwner::new(
                TrajectoryOwnerBindings::new(&self.state.owner, Arc::clone(&processed)),
                true,
            ));
        }
        self.state.phase = Phase::OffboardTrajectory;
        {
            use crate::physical_player_trajectory_owners::{
                OffboardTrajectoryOwner, TrajectoryOwnerBindings,
            };
            self.state
                .owner
                .trajectories()
                .lock()
                .map_err(|_| Error::TrajectoryPoisoned)?
                .offboard = Some(OffboardTrajectoryOwner::new(
                TrajectoryOwnerBindings::new(&self.state.owner, processed),
                &self.state.owner.inputs().vault,
            ));
        }
        self.state.phase = Phase::HandPlant;
        {
            let mut slot = self
                .state
                .owner
                .handplant
                .lock()
                .map_err(|_| Error::HandPlantPoisoned)?;
            *slot = Some(crate::physical_player_handplant::HandPlantManager::new_sim(
                &self.state.owner.inputs().vault,
                || {
                    Some(
                        crate::physical_player_handplant::trajectory::SurfaceQuery::new(
                            self.world.prediction_owner().cloned(),
                        ),
                    )
                },
            ));
        }
        self.state.phase = Phase::SelectedPumping;
        self.state
            .owner
            .pumping()
            .lock()
            .map_err(|_| Error::PumpingPoisoned)?
            .try_construct(true)
            .map_err(Error::Pumping)?;
        // Actual AnimOut allocation/ctor82e040ec/40f8 precedes Skateboard.
        self.state.phase = Phase::AnimOut;
        *self
            .state
            .owner
            .anim_out
            .lock()
            .map_err(|_| Error::AnimOutPoisoned)? =
            Some(crate::physical_player_anim_out::PhysicalAnimOut::new());
        self.state.board_force_runtime = Some(Box::new(
            crate::skateboard_force_runtime::SkateboardForceRuntime::new(
                &self.state.owner.inputs().vault,
                self.state.owner.pumping(),
                self.state
                    .surface_attributes
                    .take()
                    .expect("earlier SurfacePhysics capability"),
            ),
        ));
        // Outer38b98/38ba4 follows its own deck Instance and precedes Body
        // allocation38bbc. This selected host construction takes the successful
        // allocation branch; it does not replace a native null result.
        self.state.board_force_runtime.as_mut().expect("actual outer board")
            .speed_wobble.try_construct(&self.state.owner.inputs().vault,
                crate::skateboard_force_runtime::speed_wobble::SkateboardSpeedWobbleAllocation::Allocated)
            .map_err(Error::SpeedWobble)?;
        // Selected Body ctor Reset82b382dc precedes GetPhysicsIsland82b382e8.
        // This commits only the represented CollisionInfo fields, not omitted
        // base/outer constructor work. Routing failure retains these writes.
        self.state
            .board_force_runtime
            .as_ref()
            .expect("same outer board owner")
            .contact_surface
            .lock()
            .map_err(|_| Error::BoardResetPoisoned)?
            .collision_state
            .reset();
        // SkateboardBody caches GetPhysicsIsland at 82b382e8 BEFORE Init
        // 82b387e8; Add82b38808 later uses that cached selection.
        self.state.phase = Phase::BoardRouting;
        Self::require_simulation_island(services)?;
        self.state.phase = Phase::BoardDefinition;
        self.state
            .board_construction
            .as_mut()
            .expect("fresh actual board owner")
            .try_construct()
            .map_err(Error::BoardDefinition)?;
        match SimulationSkaterAssemblyCandidate::try_from_construction(
            self.state
                .board_construction
                .take()
                .expect("completed board"),
        ) {
            Ok(mut candidate) => {
                candidate.retain_force_runtime(
                    self.state
                        .board_force_runtime
                        .take()
                        .expect("same Skateboard force owner"),
                );
                self.state.board_candidate = Some(candidate);
            }
            Err(failure) => {
                self.state.board_construction = Some(failure.construction);
                return Err(Error::BoardDefinition(failure.error));
            }
        }
        self.state.phase = Phase::BoardAdmission;
        match self.world.try_insert_board(
            self.state.board_candidate.take().expect("actual candidate"),
            4,
            f32::from_bits(0x3f7f_be77),
        ) {
            Ok(handle) => self.state.board = Some(handle),
            Err(failure) => {
                self.state.board_candidate = Some(failure.candidate);
                return Err(Error::Pool(failure.error));
            }
        }
        let board = self.state.board.as_ref().expect("admitted board");
        self.state.phase = Phase::BoardMembership;
        self.world
            .try_add_physical_body_membership(board, 4)
            .map_err(Error::Island)?;
        self.state.phase = Phase::BoardTruckTransforms;
        self.state.board_drives = Some(SkateboardTruckDrives::new(
            self.world
                .try_calc_constructed_board_truck_transforms(board)
                .map_err(Error::Board)?,
        ));
        self.state.phase = Phase::BoardDrives;
        self.world
            .try_initialize_constructed_board_truck_drives(
                board,
                self.state
                    .board_drives
                    .as_mut()
                    .expect("actual truck drive owner"),
            )
            .map_err(Error::Board)?;
        self.state.phase = Phase::OptionalWheelDrives;
        self.world
            .try_retain_constructed_board_truck_drives(board, &mut self.state.board_drives)
            .map_err(Error::Board)?;
        let value = self
            .world
            .try_constructed_board_optional_wheel_drive_byte(board)
            .map_err(Error::Board)?;
        if value != 0 {
            return Err(Error::OptionalWheelDrivesUnsupported { value });
        }
        self.state.phase = Phase::HookDefinition;
        self.state.hook_construction = Some(SkateboardHookConstruction::new());
        let construction = self
            .state
            .hook_construction
            .as_mut()
            .expect("actual hook construction");
        self.world
            .try_retain_board_hook_dynamics(board, &self.state.owner, construction.dynamics())
            .map_err(Error::BoardEntry)?;
        construction
            .try_prepare(
                &mut HostSkateboardTruckDriveNumeric,
                &mut HostSkateboardWheelMassNumeric,
            )
            .map_err(Error::HookDefinition)?;
        self.state.hook_session = Some(
            DynamicAssemblySession::try_new(
                Arc::clone(construction.template().expect("completed template")),
                vec![FRESH_VOLUME_TRANSFORM],
            )
            .map_err(Error::HookSession)?,
        );
        match SimulationHookAssemblyCandidate::try_new(
            self.state.hook_session.take().expect("actual hook session"),
        ) {
            Ok(candidate) => self.state.hook_candidate = Some(candidate),
            Err(failure) => {
                self.state.hook_session = Some(failure.session);
                return Err(Error::Hook(failure.error));
            }
        }
        self.state.phase = Phase::HookAdmission;
        match self.world.try_insert_hook(
            self.state
                .hook_candidate
                .take()
                .expect("actual hook candidate"),
        ) {
            Ok(handle) => self.state.hook = Some(handle),
            Err(failure) => {
                self.state.hook_candidate = Some(failure.candidate);
                return Err(Error::Hook(failure.error));
            }
        }
        self.state.phase = Phase::HookConnection;
        let hook = self.state.hook.as_ref().expect("admitted hook");
        // A capacity-null drive is a supported native result: the retained
        // optional slot and detach guard still belong to the actual session.
        self.world
            .try_connect_registered_hook(
                board,
                hook,
                construction.frames(),
                construction.dynamics(),
            )
            .map_err(Error::Pool)?;
        self.state.phase = Phase::BoardPlacement;
        self.world
            .try_set_registered_board_transform(
                board,
                hook,
                self.state.owner.inputs.spawn,
                services,
            )
            .map_err(Error::Pool)?;
        self.state.phase = Phase::BoardRegistration;
        self.world
            .try_register_physical_body_targets(board)
            .map_err(Error::Island)?;
        *self
            .state
            .owner
            .physics_debug_system
            .lock()
            .map_err(|_| Error::HiLodPrefixPoisoned)? =
            Some(hilod_prefix::PhysicsDebugSystem::allocate());
        self.world
            .try_mark_physical_board_completed(board)
            .map_err(Error::Island)?;
        self.world
            .try_finish_board_entry_constructor(board)
            .map_err(Error::BoardEntry)?;
        self.state.phase = Phase::BoardReady;
        Ok(())
    }

    fn require_simulation_island(
        services: &mut impl PhysicalPlayerAssemblyServices,
    ) -> Result<(), PhysicalPlayerAssemblyError> {
        let flags = services.current_entity_flags();
        if flags & 1 == 0 {
            return Err(PhysicalPlayerAssemblyError::VisualIslandUnsupported { flags });
        }
        Ok(())
    }

    /// Starts `SkeletonData` only AFTER the real board's post-registration
    /// completion. `RegisterSpyTarget` precedes normal properties and setup.
    pub(crate) fn try_construct_skeleton(
        &mut self,
        services: &mut impl PhysicalPlayerAssemblyServices,
        hat_enabled: &mut impl FnMut() -> bool,
    ) -> Result<(), PhysicalPlayerAssemblyError> {
        use PhysicalPlayerAssemblyError as Error;
        use PhysicalPlayerAssemblyPhase as Phase;
        if self.state.skeleton_started || self.state.phase != Phase::BoardReady {
            return Err(Error::WrongPhase(self.state.phase));
        }
        self.state.skeleton_started = true;
        self.state.phase = Phase::SkeletonDefinition;
        self.state.skeleton_construction = Some(Box::new(SkeletonAssemblyConstruction::new(
            Arc::clone(&self.state.owner),
        )));
        self.state.phase = Phase::SkeletonIk;
        self.world
            .try_construct_paired_skeleton_members(
                self.state
                    .skeleton_construction
                    .as_mut()
                    .expect("retained Skeleton before Init"),
                self.state.board.as_ref().expect("completed actual board"),
                &mut || services.current_entity_flags(),
            )
            .map_err(Error::SkeletonIk)?;
        // Native82e04204 follows the completed Skeleton child constructors;
        // Skeleton.Init then initializes the same retained Data and IK owners.
        self.state.phase = Phase::SelectedStateChanger;
        self.state
            .owner
            .state_changer()
            .lock()
            .map_err(|_| Error::StateChangerPoisoned)?
            .try_construct(&self.state.owner, true)
            .map_err(Error::StateChanger)?;
        self.state.phase = Phase::SkeletonDefinition;
        self.world.try_construct_physical_skeleton(
            self.state
                .skeleton_construction
                .as_mut()
                .expect("actual skeleton constructor"),
            hat_enabled,
        )?;
        match SimulationSkeletonAssemblyCandidate::try_from_construction(
            self.state
                .skeleton_construction
                .take()
                .expect("completed skeleton constructor"),
        ) {
            Ok(candidate) => self.state.skeleton_candidate = Some(candidate),
            Err(construction) => {
                self.state.skeleton_construction = Some(construction);
                return Err(Error::SkeletonHandoff);
            }
        }
        self.state.phase = Phase::SkeletonRouting;
        Self::require_simulation_island(services)?;
        self.state.phase = Phase::SkeletonAdmission;
        match self.world.try_insert_skeleton(
            self.state
                .skeleton_candidate
                .take()
                .expect("actual skeleton candidate"),
            -1.0,
        ) {
            Ok(handle) => self.state.skeleton = Some(handle),
            Err(failure) => {
                self.state.skeleton_candidate = Some(failure.candidate);
                return Err(Error::Pool(failure.error));
            }
        }
        let skeleton = self.state.skeleton.as_ref().expect("admitted skeleton");
        self.state.phase = Phase::SkeletonMembership;
        self.world
            .try_add_physical_body_membership(skeleton, 4)
            .map_err(Error::Island)?;
        // Native SyncCheckBody only locks/unlocks; exclusive world ownership
        // already serializes this host path, without a fabricated callback.
        self.state.phase = Phase::SkeletonRegistration;
        self.world
            .try_register_physical_body_targets(skeleton)
            .map_err(Error::Island)?;
        self.state.phase = Phase::SkeletonNormal;
        self.world
            .try_set_skeleton_pre_normal_properties(skeleton)
            .map_err(Error::SkeletonProperties)?;
        self.world
            .try_set_constructed_skeleton_normal(skeleton)
            .map_err(Error::SkeletonNormal)?;
        self.state.phase = Phase::SkeletonTeleportReset;
        self.world
            .try_teleport_reset_skeleton(skeleton)
            .map_err(Error::Pool)?;
        self.state.phase = Phase::SkeletonReady;
        Ok(())
    }

    /// Constructor drives, Set and initial physics-history path. Live cvar and
    /// history-input contracts are preserved; no hardcoded construction profile.
    /// Complete means this ASSEMBLY projection, not omitted IK/HiLOD services.
    #[expect(
        clippy::too_many_lines,
        reason = "ordered retained constructor stages preserve each failure prefix"
    )]
    pub(crate) fn try_finish_skeleton<C: SkeletonDriveCvars, H: SkeletonHistoryInput>(
        &mut self,
        cvars: &mut C,
        history: &mut H,
    ) -> Result<(), PhysicalPlayerSkeletonFinishError<C::Error, H::Error>> {
        use PhysicalPlayerAssemblyPhase as Phase;
        use PhysicalPlayerSkeletonFinishError as Error;
        if self.state.phase != Phase::SkeletonReady {
            return Err(Error::WrongPhase(self.state.phase));
        }
        let skeleton = self.state.skeleton.as_ref().expect("constructed skeleton");
        self.state.phase = Phase::SkeletonDrives;
        self.state.skeleton_drive_attributes = Some(SkeletonDriveAttributes::bind(
            &self.state.owner.inputs.vault,
        ));
        self.world
            .try_initialize_skeleton_drives(
                skeleton,
                self.state
                    .skeleton_drive_attributes
                    .as_mut()
                    .expect("actual drive provider"),
                &mut HostSkeletonDriveNumeric,
                &mut HostSkateboardWheelMassNumeric,
                cvars,
            )
            .map_err(Error::Drives)?;
        self.world
            .try_retain_skeleton_drive_attributes(
                skeleton,
                &mut self.state.skeleton_drive_attributes,
            )
            .map_err(Error::Pool)?;
        self.state.phase = Phase::SkeletonPlacement;
        self.world
            .try_initialize_skeleton_animation_buffers(skeleton)
            .map_err(Error::Pool)?;
        self.world
            .try_set_skeleton_pose_with_hooks(skeleton)
            .map_err(Error::Pool)?;
        self.state.phase = Phase::SkeletonHistory;
        self.world
            .try_initialize_physics_history(
                skeleton,
                history,
                &mut HostSkeletonPhysicsHistoryNumeric,
            )
            .map_err(Error::History)?;
        self.world
            .try_finish_skeleton_init(skeleton)
            .map_err(Error::Pool)?;
        self.state
            .owner
            .wipeout()
            .lock()
            .map_err(|_| Error::WipeoutPoisoned)?
            .try_construct(true)
            .map_err(Error::Wipeout)?;
        // Native GrindManager82e042bc follows Wipeout and precedes FootPlant.
        self.state.phase = Phase::Grind;
        {
            let processed = Arc::clone(
                self.state
                    .owner
                    .reckoning()
                    .lock()
                    .map_err(|_| Error::GrindPoisoned)?
                    .processed(),
            );
            let mut slot = self
                .state
                .owner
                .grind
                .lock()
                .map_err(|_| Error::GrindPoisoned)?;
            *slot = Some(
                crate::physical_player_grind_manager::GrindManager::new_sim_prefix(
                    crate::physical_player_grind_manager::GrindBindings::new(
                        &self.state.owner,
                        processed,
                        self.state.board.as_ref().expect("completed board"),
                    ),
                ),
            );
            slot.as_mut()
                .expect("retained GrindManager")
                .try_construct_children(&self.state.owner.inputs().cvars, &mut |_| true)
                .map_err(Error::Grind)?;
        }
        // Selected FootPlant ctor82e042f4.
        self.state
            .owner
            .footplant()
            .lock()
            .map_err(|_| Error::FootPlantPoisoned)?
            .try_construct(
                &self.state.owner,
                skeleton,
                &self.state.owner.inputs().vault,
                true,
            )
            .map_err(Error::FootPlant)?;
        // Actual PlayerGrabSpline82e0432c follows FootPlant, before controller.
        self.state.phase = Phase::GrabSpline;
        {
            let mut grab = self
                .state
                .owner
                .grab_spline
                .lock()
                .map_err(|_| Error::GrabPoisoned)?;
            *grab = Some(
                crate::physical_player_grab_spline::PlayerGrabSpline::new_prefix(
                    &self.state.owner,
                    self.world.grab_query_manager().cloned(),
                ),
            );
            grab.as_mut()
                .expect("retained grab-spline prefix")
                .try_finish()
                .map_err(Error::Grab)?;
        }
        self.state.phase = Phase::Complete;
        Ok(())
    }

    pub(crate) fn completed_world_mut(
        &mut self,
    ) -> Result<&mut PhysicalAssemblyWorld, PhysicalPlayerAssemblyPhase> {
        if self.state.phase != PhysicalPlayerAssemblyPhase::Complete {
            return Err(self.state.phase);
        }
        Ok(self.world)
    }

    /// Explicit cleanup, never automatic rollback/Drop. `HiLOD` destroys skeleton
    /// before board (82e01368/1394), and `SkeletonDrives` before `SkeletonBody`.
    /// Successful removals retain returned candidates/Body IDs here; a checked
    /// failure retains its prefix and allows cleanup, but not construction, retry.
    /// Unadmitted-candidate recovery is checked host policy, not native faults.
    pub(crate) fn try_teardown(&mut self) -> Result<(), PhysicalPlayerAssemblyError> {
        use PhysicalPlayerAssemblyError as Error;
        use PhysicalPlayerAssemblyPhase as Phase;
        if self.state.phase == Phase::Removed {
            return Ok(());
        }
        self.state
            .owner
            .require_controller_drives_removed()
            .map_err(Error::Pool)?;
        self.state.phase = Phase::TearingDown;
        if let Some(skeleton) = self.state.skeleton.as_ref() {
            if self
                .world
                .skeleton(skeleton)
                .map_err(Error::Pool)?
                .drive_owner()
                .is_some()
            {
                self.world
                    .try_release_skeleton_drives(skeleton)
                    .map_err(Error::Pool)?;
            }
            self.state.skeleton_candidate = Some(
                self.world
                    .try_remove_skeleton(skeleton)
                    .map_err(Error::Pool)?,
            );
            self.state.skeleton = None;
        }
        if let Some(candidate) = self.state.skeleton_candidate.as_mut() {
            self.world
                .try_detach_unadmitted_physical_skeleton(candidate)
                .map_err(Error::Pool)?;
        }
        if let Some(board) = self.state.board.as_ref() {
            self.world
                .try_remove_board_drives(board)
                .map_err(Error::Pool)?;
        }
        if let Some(hook) = self.state.hook.as_ref() {
            self.state.hook_candidate =
                Some(self.world.try_remove_hook(hook).map_err(Error::Hook)?);
            self.state.hook = None;
        }
        if let Some(candidate) = self.state.hook_candidate.as_mut() {
            self.world
                .try_detach_unadmitted_physical_hook(candidate)
                .map_err(Error::Hook)?;
        }
        if let Some(board) = self.state.board.as_ref() {
            self.state.board_candidate =
                Some(self.world.try_remove_board(board).map_err(Error::Pool)?);
            self.state.board = None;
        }
        if let Some(candidate) = self.state.board_candidate.as_mut() {
            self.world
                .try_detach_unadmitted_physical_board(candidate)
                .map_err(Error::Pool)?;
        }
        self.state.phase = Phase::Removed;
        Ok(())
    }
}

#[path = "physical_player_selected_animation_conditioning.rs"]
pub(crate) mod selected_animation_conditioning;
#[path = "physical_player_scoring_input.rs"]
pub(crate) mod scoring_input;
#[path = "../conditioning/physical_player_selected_ground_conditioning.rs"]
pub(crate) mod selected_ground_conditioning;

#[cfg(test)]
use crate::skateboard_spawn::SkateboardSpawnServices;
#[path = "physical_player_constructor_services.rs"]
pub(crate) mod constructor_services;

#[path = "physical_player_selected_constructor.rs"]
pub(crate) mod selected_constructor;

#[path = "../offboard/physical_player_landing_on_deck_state.rs"]
pub(crate) mod landing_on_deck_state;
pub(crate) use crate::simulation_assembly_backend::constraint_session::known_air_state;

#[path = "physical_player_controller_active.rs"]
pub(crate) mod controller_active;

#[path = "physical_player_controller_throw.rs"]
pub(crate) mod controller_throw;

#[path = "../offboard/physical_player_offboard_locomotion_state.rs"]
pub(crate) mod offboard_locomotion_state;
#[path = "../offboard/physical_player_offboard_pushing_state.rs"]
pub(crate) mod offboard_pushing_state;
