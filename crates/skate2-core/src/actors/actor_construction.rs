//! Retained selected `Actor` constructor prefix, 0x825fe850..0x825fec20.
//! Includes owned listener and unloaded graph construction; not a complete Actor.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected Actor prefix awaits native startup orchestration"
    )
)]

use crate::{
    action_graph_input_listener::{ActionGraphInputListener, ActionInputListenerError},
    actor_graph_construction::{
        ActorGraphAllocation, ActorGraphComponents, ActorGraphConstructionError,
        ActorGraphIntentError, ActorGraphInterface, ActorGraphRegistry,
    },
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    game_input_player::GameInputPlayer,
    game_input_player::recognizer::event_route::{GameInputEventRoute, GameInputRouteError},
    motion_graph_entity_authority::{ActorGraphAuthority, MotionGraphEntityAuthority},
    reckoning_numeric::{ReckoningNumeric, try_length},
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::{
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

#[cfg(test)]
#[path = "actor_construction_tests.rs"]
pub(crate) mod tests;

#[path = "../animation/graphs/action/actor_action_motion_intent_bridge.rs"]
pub(crate) mod action_motion_intent_bridge;
#[path = "../animation/graphs/action/actor_action_runtime_session.rs"]
pub(crate) mod action_runtime_session;
#[path = "actor_anim_out_phys_in.rs"]
pub(crate) mod anim_out_phys_in;
#[path = "actor_ai_control.rs"]
pub(crate) mod ai_control;
#[path = "actor_anim_out_phys_in_mount.rs"]
pub(crate) mod anim_out_phys_in_mount;
#[path = "../animation/runtime/actor_animation_mount.rs"]
pub(crate) mod animation_mount;
#[path = "../animation/runtime/actor_animation_phase1.rs"]
pub(crate) mod animation_phase1;
#[path = "../animation/runtime/actor_animation_start.rs"]
pub(crate) mod animation_start;
#[path = "../animation/runtime/actor_animation_body_session.rs"]
pub(crate) mod body_session;
#[cfg(feature = "skate3-runtime")]
#[path = "../animation/runtime/actor_skate3_gestures.rs"]
pub(crate) mod skate3_gestures;
#[path = "actor_breadcrumb_postupdate.rs"]
pub(crate) mod breadcrumb_postupdate;
#[path = "actor_breadcrumb_manager.rs"]
pub(crate) mod breadcrumbs;
#[path = "../animation/graphs/motion/generated_motion_character_gesture.rs"]
pub(crate) mod character_gesture;
#[path = "../animation/graphs/actor_graph_startup.rs"]
pub(crate) mod graph_startup;
#[path = "../animation/hands_face/actor_animation_hands_face_session.rs"]
pub(crate) mod hands_face_session;
#[path = "actor_input_generation.rs"]
pub(crate) mod input_generation;
#[path = "../animation/locomotion/actor_animation_locomotion_mid_session.rs"]
pub(crate) mod locomotion_mid_session;
#[path = "../animation/graphs/motion/callbacks/actor_motion_filter_intent_begin.rs"]
pub(crate) mod motion_filter_intent_begin;
#[path = "../animation/graphs/motion/callbacks/actor_motion_filter_intent_update.rs"]
pub(crate) mod motion_filter_intent_update;
#[path = "../animation/graphs/motion/callbacks/actor_motion_runtime_session.rs"]
pub(crate) mod motion_runtime_session;
#[path = "actor_physical_mount.rs"]
pub(crate) mod physical_mount;
#[path = "actor_physics_auxiliary.rs"]
pub(crate) mod physics_auxiliary;
#[path = "actor_reset_skater_animation_begin.rs"]
pub(crate) mod reset_skater_animation_begin;
#[path = "actor_selected_frame_session.rs"]
pub(crate) mod selected_frame_session;
#[path = "actor_teleport.rs"]
pub(crate) mod teleport;
#[path = "actor_update_input_flags.rs"]
pub(crate) mod update_input_flags;
#[path = "actor_chase_update.rs"]
pub(crate) mod chase_update;

const CHASE_CLASS: u64 = 0x1477_b0d5_343d_29fd;
const MIN_RADIUS: u64 = 0x09c7_ba07_59e1_3dde;
const MAX_RADIUS: u64 = 0x4cf3_9d0d_047c_8cc0;
const VELOCITY_SCALE: u64 = 0xfdcd_97d0_5887_5e31;

#[derive(Debug)]
pub(crate) enum ActorConstructionError<E = std::convert::Infallible> {
    Poisoned,
    GameplayUnconstructed,
    ActorFieldsUninitialized,
    TelemetryUnavailable,
    AlreadyAttempted,
    ChaseFormationUnbound,
    MissingChaseCollection,
    Attribute(AttributeRuntimeError),
    Allocation,
    PlayerIndexAssertion(u32),
    MatrixNanAssertion,
    PositionLengthAssertion,
    Numeric(E),
    ListenerPrefixIncomplete,
    Listener(ActionInputListenerError),
    ListenerRoute(GameInputRouteError),
    GraphPrefixIncomplete,
    Graph(ActorGraphConstructionError<ActorGraphRegistrationError>),
}
impl<E: fmt::Debug> fmt::Display for ActorConstructionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "Actor chase attribute: {error}"),
            Self::Numeric(error) => write!(f, "Actor position numeric: {error:?}"),
            Self::PlayerIndexAssertion(index) => write!(f, "Actor player index assertion: {index}"),
            Self::Listener(error) => write!(f, "Actor listener construction: {error:?}"),
            Self::ListenerRoute(error) => write!(f, "Actor listener registration: {error:?}"),
            Self::Graph(error) => write!(f, "Actor graph construction: {error}"),
            other => write!(f, "Actor constructor prefix: {other:?}"),
        }
    }
}

/// Retained `ChaseMan` formation attribute fields.
/// Its constructor has a null collection; selected Reset binds `global`.
/// This does not construct/publish `ChaseMan`, its services, or its broadcaster; the camera's
/// threat reads go to the living world's own `ChaseMan`.
pub(crate) struct ActorChaseFormationOwner {
    vault: Arc<AttributeRuntimeVault>,
    attempted: bool,
    instance: Option<AttributeRuntimeInstance>,
    ready: bool,
    // Actual ctor82715878/88.
    num_chasers: u32,
    chase_flags: u8,
}
impl ActorChaseFormationOwner {
    pub(crate) fn new(vault: Arc<AttributeRuntimeVault>) -> Self {
        Self {
            vault,
            attempted: false,
            instance: None,
            ready: false,
            num_chasers: 0,
            chase_flags: 0x10,
        }
    }
    pub(crate) fn try_bind_selected_reset(&mut self) -> Result<(), ActorConstructionError> {
        if self.attempted {
            return Err(ActorConstructionError::AlreadyAttempted);
        }
        self.attempted = true;
        // Reset assigns before its non-null assertion (0x82702294/22a0).
        self.instance = Some(self.fresh_instance());
        if self
            .instance
            .as_ref()
            .is_none_or(AttributeRuntimeInstance::is_null)
        {
            return Err(ActorConstructionError::MissingChaseCollection);
        }
        // Reset827022d4 samples actual ChaseIsOn (numChasers !=0), then
        // clears the native flags. Keep this after the attribute assertion.
        if self.num_chasers != 0 {
            self.chase_flags |= 4;
        }
        self.chase_flags = (self.chase_flags & 0x67) | 0x10;
        self.ready = true;
        Ok(())
    }
    fn fresh_instance(&self) -> AttributeRuntimeInstance {
        self.vault
            .instance(CHASE_CLASS, skate2_formats::attrib::string_hash64("global"))
    }
    fn scalar<E>(&self, key: u64) -> Result<f32, ActorConstructionError<E>> {
        if !self.ready {
            return Err(ActorConstructionError::ChaseFormationUnbound);
        }
        let instance = self
            .instance
            .as_ref()
            .ok_or(ActorConstructionError::ChaseFormationUnbound)?;
        if instance.is_null() {
            return Ok(0.0);
        }
        let data = instance
            .try_get_data(key, 0)
            .map_err(ActorConstructionError::Attribute)?;
        let Some(data) = data else {
            return Ok(0.0);
        };
        let bytes = data.bytes();
        let bytes: [u8; 4] = bytes.try_into().map_err(|_| {
            ActorConstructionError::Attribute(AttributeRuntimeError::ScalarSize {
                size: bytes.len(),
            })
        })?;
        Ok(f32::from_bits(u32::from_be_bytes(bytes)))
    }
}

/// Identity belongs to this retained constructor owner. The manager is a host
/// binding, not evidence that native `PhysicsWorld` dependencies are constructed.
pub(crate) struct ActorIdentity {
    gameplay: Arc<Mutex<GameInputPlayer>>,
    manager: SimulationManagerContactAuthority,
}
impl ActorIdentity {
    pub(crate) fn gameplay(&self) -> &Arc<Mutex<GameInputPlayer>> {
        &self.gameplay
    }
    pub(crate) const fn manager(&self) -> SimulationManagerContactAuthority {
        self.manager
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActorPrefixPhase {
    Unattempted,
    ChaseeReset,
    ActorFields,
    MatrixValidation,
    ListenerRequired,
    ListenerConstructing,
    GraphRequired,
    GraphConstructing,
    GraphFacadesConstructed,
    AnimationSelecting,
    AnimationConstructing,
    AnimationConstructorReturned,
    AnimationInitializing,
    AnimationInitialized,
    AnimationPhysicsInputConstructing,
    AnimationPhysicsInputConstructed,
    PhysicalConstructing,
    PhysicalConstructorReturned,
}

/// Optional fields distinguish untouched storage from completed native writes.
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct source member publications must survive partial constructor failures"
)]
pub(crate) struct ActorConstructorPrefix {
    phase: ActorPrefixPhase,
    interfaces: Vec<u32>,
    cached_interfaces_null: bool,
    /// Sole completed Chasee owner. Scalar Options below hold only a failed
    /// construction prefix and are consumed when native ChaseeReset completes.
    pub(crate) chasee: Option<crate::living_world_chase::ChaseeState>,
    flank: Option<i32>,
    min_radius: Option<f32>,
    max_radius: Option<f32>,
    block_radius: Option<f32>,
    velocity_scale: Option<f32>,
    chase_timers: Option<[f32; 2]>,
    own_chase: Option<AttributeRuntimeInstance>,
    group_end_reason: Option<i32>,
    fields: Option<ActorInitialFields>,
    // Actual owned allocation survives late constructor failure. Publication
    // into the source Actor member is a distinct milestone after registrations.
    listener: Option<Arc<Mutex<ActionGraphInputListener<ActorIdentity>>>>,
    listener_published: bool,
    graphs: Option<ActorGraphComponents<ActorIdentity>>,
    graph_interfaces: [Option<Arc<()>>; 3],
    action_graph_published: bool,
    motion_graph_published: bool,
    animation: Option<Arc<Mutex<animation_mount::MountedActorAnimation>>>,
    animation_interfaces: [Option<Arc<()>>; 3],
    animation_published: bool,
    external_player_reset: Option<animation_mount::ExternalPlayerResetStorage>,
    physics_auxiliary: Option<physics_auxiliary::ActorPhysicsAuxiliary>,
    animation_start: Option<animation_start::ActorAnimationStart>,
    animation_physics_input: Option<Arc<Mutex<anim_out_phys_in::AnimOutPhysInOwner>>>,
    animation_physics_input_published: bool,
    physical_allocation: Option<Arc<physical_mount::ActorPhysicalAllocation>>,
    physical_interface: Option<Arc<physical_mount::ActorPhysicalAllocation>>,
    // Native Actor+200, distinct from the early registered allocation. Only
    // the complete HiLOD constructor return may publish this cached owner.
    physical_cached: Option<Arc<physical_mount::ActorPhysicalAllocation>>,
    breadcrumbs_owner: Option<Arc<Mutex<breadcrumbs::ActorBreadCrumbManager>>>,
    // Only source-initialized upper seven bits of Actor+258 are represented.
    input_generation_flags: Option<u32>,
    /// Online, a pinned skater still gestures: the input fill runs and its gesture intents stay.
    pinned_gestures: bool,
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent native constructor bits and null-member projections are not one exclusive state"
)]
pub(crate) struct ActorInitialFields {
    pub(crate) player_index: u32,
    pub(crate) flags: u32,
    pub(crate) time_since_fade_in: f32,
    pub(crate) time_since_fade_out: f32,
    pub(crate) set_transform_count: u32,
    pub(crate) teleport_count: u32,
    /// Native RIGHT enum; retain the semantic value without guessing its ABI.
    pub(crate) last_foot_forward_right: bool,
    pub(crate) protector_invalid: bool,
    pub(crate) protector: crate::living_world_chase::TargetableId,
    pub(crate) sound_level: f32,
    pub(crate) sound_timer: f32,
    pub(crate) grind_sound_timer: f32,
    /// High seven bits clear; low 25 bits retain unknown prior storage.
    pub(crate) packed_known_zero_mask: u32,
    pub(crate) action_graph_null: bool,
    pub(crate) motion_graph_null: bool,
    pub(crate) animation_null: bool,
    pub(crate) breadcrumbs_null: bool,
    pub(crate) ai_controller_null: bool,
}
impl ActorConstructorPrefix {
    pub(crate) const fn phase(&self) -> ActorPrefixPhase {
        self.phase
    }
    pub(crate) fn interface_names(&self) -> &[u32] {
        &self.interfaces
    }
    pub(crate) fn fields(&self) -> Option<&ActorInitialFields> {
        self.fields.as_ref()
    }
    pub(crate) fn fields_mut(&mut self) -> Option<&mut ActorInitialFields> {
        self.fields.as_mut()
    }
    pub(crate) fn graphs(&self) -> Option<&ActorGraphComponents<ActorIdentity>> {
        self.graphs.as_ref()
    }
    fn register<E>(&mut self, name: u32) -> Result<(), ActorConstructionError<E>> {
        self.interfaces
            .try_reserve(1)
            .map_err(|_| ActorConstructionError::Allocation)?;
        let index = self.interfaces.partition_point(|old| *old <= name);
        self.interfaces.insert(index, name);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActorGraphRegistrationError {
    ForeignEntity,
    DuplicateInterface(u32),
    Allocation,
}

/// Borrows the existing Entity registry and retains each actual graph occurrence.
struct ActorGraphRegistration<'a> {
    authority: &'a ActorGraphAuthority<ActorIdentity>,
    state: &'a mut ActorConstructorPrefix,
}
impl ActorGraphRegistry<ActorIdentity> for ActorGraphRegistration<'_> {
    type Error = ActorGraphRegistrationError;

    fn register_graph_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<ActorIdentity>,
        kind: ActorGraphInterface,
        component: &Arc<()>,
    ) -> Result<(), Self::Error> {
        if !entity.is_same_issuance(&self.authority.entity_handle()) {
            return Err(ActorGraphRegistrationError::ForeignEntity);
        }
        let name = kind.source_name();
        if self.state.interfaces.binary_search(&name).is_ok() {
            return Err(ActorGraphRegistrationError::DuplicateInterface(name));
        }
        self.state
            .register::<std::convert::Infallible>(name)
            .map_err(|_| ActorGraphRegistrationError::Allocation)?;
        let slot = match kind {
            ActorGraphInterface::Action => 0,
            ActorGraphInterface::Motion => 1,
            ActorGraphInterface::SkaterMotion => 2,
        };
        self.state.graph_interfaces[slot] = Some(Arc::clone(component));
        Ok(())
    }
}

pub(crate) struct ActorConstruction {
    // Host capability to the shared selected globals, not a native Actor member.
    pub(crate) handplant_shared: Mutex<Option<Arc<[f32; 3]>>>,
    action_services: Mutex<Option<Arc<action_runtime_session::ActionRuntimeServices>>>,
    // Host bridge to the one SkaterManager-owned recovery graph, not a second
    // native Actor member or another copy of the animation databases.
    wipeout_recovery_graph:
        Mutex<Option<Arc<crate::skater_wipeout_recovery_graph::SkaterWipeoutRecoveryGraph>>>,
    // Same SkaterManager SkaterAnimShared initialization as recovery graph.
    push_shared_attributes:
        Mutex<Option<Arc<crate::skater_push_shared_attributes::SkaterPushSharedAttributes>>>,
    authority: ActorGraphAuthority<ActorIdentity>,
    flags: u32,
    player_index: u32,
    spawn: SkeletonRootMatrix,
    state: Arc<Mutex<ActorConstructorPrefix>>,
}
impl ActorConstruction {
    pub(crate) fn new(
        gameplay: Arc<Mutex<GameInputPlayer>>,
        manager: SimulationManagerContactAuthority,
        flags: u32,
        spawn: SkeletonRootMatrix,
    ) -> Result<Self, ActorConstructionError> {
        // Host ownership prerequisite, not a relocated native ID assertion.
        let player_index = {
            let player = gameplay
                .lock()
                .map_err(|_| ActorConstructionError::Poisoned)?;
            if !player.selected_construction_complete() {
                return Err(ActorConstructionError::GameplayUnconstructed);
            }
            player.player_index()
        };
        Ok(Self {
            authority: ActorGraphAuthority::new(ActorIdentity { gameplay, manager }),
            handplant_shared: Mutex::new(None),
            action_services: Mutex::new(None),
            wipeout_recovery_graph: Mutex::new(None),
            push_shared_attributes: Mutex::new(None),
            flags,
            player_index,
            spawn,
            state: Arc::new(Mutex::new(ActorConstructorPrefix {
                phase: ActorPrefixPhase::Unattempted,
                interfaces: Vec::new(),
                cached_interfaces_null: false,
                chasee: None,
                flank: None,
                min_radius: None,
                max_radius: None,
                block_radius: None,
                velocity_scale: None,
                chase_timers: None,
                own_chase: None,
                group_end_reason: None,
                fields: None,
                listener: None,
                listener_published: false,
                graphs: None,
                graph_interfaces: [None, None, None],
                action_graph_published: false,
                motion_graph_published: false,
                animation: None,
                animation_start: None,
                animation_physics_input: None,
                animation_physics_input_published: false,
                physical_allocation: None,
                physical_interface: None,
                physical_cached: None,
                breadcrumbs_owner: None,
                animation_interfaces: [None, None, None],
                animation_published: false,
                external_player_reset: None,
                physics_auxiliary: None,
                input_generation_flags: None,
                pinned_gestures: false,
            })),
        })
    }
    pub(crate) fn prefix(
        &self,
    ) -> Result<MutexGuard<'_, ActorConstructorPrefix>, ActorConstructionError> {
        self.state
            .lock()
            .map_err(|_| ActorConstructionError::Poisoned)
    }
    pub(crate) fn authority_for_listener(
        &self,
    ) -> Result<&ActorGraphAuthority<ActorIdentity>, ActorConstructionError> {
        if !matches!(
            self.prefix()?.phase,
            ActorPrefixPhase::ListenerRequired
                | ActorPrefixPhase::ListenerConstructing
                | ActorPrefixPhase::GraphRequired
                | ActorPrefixPhase::GraphConstructing
                | ActorPrefixPhase::GraphFacadesConstructed
        ) {
            return Err(ActorConstructionError::ListenerPrefixIncomplete);
        }
        Ok(&self.authority)
    }

    /// Retains the actual partial listener before source registration effects.
    /// The selected host allocation has no native null-allocation success path;
    /// native allocator parity and the remainder of Actor construction remain
    /// separate. There is no automatic registration rollback or Drop teardown.
    pub(crate) fn try_construct_listener(
        &self,
        route: &mut GameInputEventRoute<ActorIdentity>,
    ) -> Result<Arc<Mutex<ActionGraphInputListener<ActorIdentity>>>, ActorConstructionError> {
        let mut state = self.prefix()?;
        match state.phase {
            ActorPrefixPhase::ListenerRequired => {}
            ActorPrefixPhase::ListenerConstructing
            | ActorPrefixPhase::GraphRequired
            | ActorPrefixPhase::GraphConstructing
            | ActorPrefixPhase::GraphFacadesConstructed => {
                return Err(ActorConstructionError::AlreadyAttempted);
            }
            _ => return Err(ActorConstructionError::ListenerPrefixIncomplete),
        }
        state.phase = ActorPrefixPhase::ListenerConstructing;
        let listener = Arc::new(Mutex::new(
            ActionGraphInputListener::new(&self.authority, self.authority.entity_key().gameplay())
                .map_err(ActorConstructionError::Listener)?,
        ));
        state.listener = Some(Arc::clone(&listener));
        route
            .try_construct_listener(&listener)
            .map_err(ActorConstructionError::ListenerRoute)?;
        // Actor+1f0 store825fec58 is after all three native registrations.
        state.listener_published = true;
        state.phase = ActorPrefixPhase::GraphRequired;
        Ok(listener)
    }

    /// Includes a failed constructor's retained allocation and cleanup token.
    pub(crate) fn retained_listener(
        &self,
    ) -> Result<Option<Arc<Mutex<ActionGraphInputListener<ActorIdentity>>>>, ActorConstructionError>
    {
        Ok(self.prefix()?.listener.as_ref().map(Arc::clone))
    }

    /// Source Actor 0x825fec5c..0x825fecc8. This ends before animation,
    /// physical construction, graph loading and final Actor publication.
    pub(crate) fn try_construct_graphs(&self) -> Result<(), ActorConstructionError> {
        self.try_construct_graphs_with_allocations([ActorGraphAllocation::Host; 4])
    }

    /// Explicit selected outcomes for Action outer/wrapper then Motion
    /// outer/wrapper allocations. Host ownership is not native allocator parity.
    pub(crate) fn try_construct_graphs_with_allocations(
        &self,
        allocations: [ActorGraphAllocation; 4],
    ) -> Result<(), ActorConstructionError> {
        let mut state = self.prefix()?;
        match state.phase {
            ActorPrefixPhase::GraphRequired => {}
            ActorPrefixPhase::GraphConstructing | ActorPrefixPhase::GraphFacadesConstructed => {
                return Err(ActorConstructionError::AlreadyAttempted);
            }
            _ => return Err(ActorConstructionError::GraphPrefixIncomplete),
        }
        state.phase = ActorPrefixPhase::GraphConstructing;
        let mut graphs = ActorGraphComponents::new(&self.authority);
        let result = (|| {
            let mut registry = ActorGraphRegistration {
                authority: &self.authority,
                state: &mut state,
            };
            graphs.try_construct_action_with_allocations(
                &mut registry,
                allocations[0],
                allocations[1],
            )?;
            // +1f4 is published only after Action's constructor returns.
            registry
                .state
                .fields
                .as_mut()
                .expect("Actor fields precede graphs")
                .action_graph_null = graphs.action().is_none();
            registry.state.action_graph_published = true;
            graphs.try_construct_skater_motion_with_allocations(
                &mut registry,
                allocations[2],
                allocations[3],
            )?;
            // +1f8 denotes ISkaterMotionGraph, distinct from base IMotionGraph.
            registry
                .state
                .fields
                .as_mut()
                .expect("Actor fields precede graphs")
                .motion_graph_null = graphs.motion().is_none();
            registry.state.motion_graph_published = true;
            Ok(())
        })();
        // Retain all allocations and registrations even after a late failure.
        state.graphs = Some(graphs);
        result.map_err(ActorConstructionError::Graph)?;
        state.phase = ActorPrefixPhase::GraphFacadesConstructed;
        Ok(())
    }
    pub(crate) fn try_construct_prefix<N: ReckoningNumeric>(
        &self,
        formation: &ActorChaseFormationOwner,
        numeric: &mut N,
    ) -> Result<(), ActorConstructionError<N::Error>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActorConstructionError::Poisoned)?;
        if state.phase != ActorPrefixPhase::Unattempted {
            return Err(ActorConstructionError::AlreadyAttempted);
        }
        state.phase = ActorPrefixPhase::ChaseeReset;
        // +14/+18/+1c cached interface null writes precede the first Add.
        state.cached_interfaces_null = true;
        // Source interfaces use _IName function identities, not name hashes.
        for name in [0x825e_be38, 0x825e_be28, 0x825e_be68, 0x825e_be58] {
            state.register(name)?;
        }
        state.flank = Some(-1);
        state.min_radius = Some(formation.scalar(MIN_RADIUS)?);
        state.max_radius = Some(formation.scalar(MAX_RADIUS)?);
        let velocity = formation.scalar(VELOCITY_SCALE)?;
        state.block_radius = state.min_radius;
        state.velocity_scale = Some(velocity);
        state.chase_timers = Some([0.0; 2]);
        // A fresh instance is constructed for Chasee, not copied from ChaseMan.
        state.own_chase = Some(formation.fresh_instance());
        state.group_end_reason = Some(-1);
        let mut chasee=crate::living_world_chase::ChaseeState::new(
            state.min_radius.take().expect("assigned before completion"),
            state.max_radius.take().expect("assigned before completion"),
            state.velocity_scale.take().expect("assigned before completion"));
        chasee.block_radius=state.block_radius.take().expect("assigned before completion");
        let [scared,nearby]=state.chase_timers.take().expect("assigned before completion");
        chasee.scared_timer=scared;chasee.nearby_event_timer=nearby;
        chasee.group_end_reason=state.group_end_reason.take().expect("assigned before completion");
        state.chasee=Some(chasee);
        state.register(0x825e_be78)?;
        state.phase = ActorPrefixPhase::ActorFields;
        let index = self.player_index;
        state.fields = Some(ActorInitialFields {
            player_index: index,
            flags: self.flags,
            time_since_fade_in: 0.0,
            time_since_fade_out: f32::MAX,
            set_transform_count: 0,
            teleport_count: 0,
            last_foot_forward_right: true,
            protector_invalid: true,
            protector: crate::living_world_chase::TargetableId::INVALID,
            sound_level: 0.0,
            sound_timer: 0.0,
            grind_sound_timer: 0.0,
            packed_known_zero_mask: 0xfe00_0000,
            action_graph_null: true,
            motion_graph_null: true,
            animation_null: true,
            breadcrumbs_null: true,
            ai_controller_null: true,
        });
        state.input_generation_flags = Some(0);
        if index > 8 {
            return Err(ActorConstructionError::PlayerIndexAssertion(index));
        }
        state.phase = ActorPrefixPhase::MatrixValidation;
        validate_spawn(&self.spawn, numeric)?;
        state.phase = ActorPrefixPhase::ListenerRequired;
        Ok(())
    }
}

fn validate_spawn<N: ReckoningNumeric>(
    spawn: &SkeletonRootMatrix,
    numeric: &mut N,
) -> Result<(), ActorConstructionError<N::Error>> {
    // Native self equality rejects NaN only. It does not require finite bases,
    // orthogonality, or a homogeneous W; W is never consumed by these checks.
    let mut all_rows = true;
    for column in spawn.columns() {
        let row = column.xyz.iter().all(|value| !value.is_nan());
        all_rows &= row;
    }
    if !all_rows {
        return Err(ActorConstructionError::MatrixNanAssertion);
    }
    let xyz = spawn.columns()[3].xyz;
    // Scratch W is unobserved by dot3, never stored or reported as native W.
    let length = try_length([xyz[0], xyz[1], xyz[2], 0.0], numeric)
        .map_err(ActorConstructionError::Numeric)?;
    if f32::from_bits(0x47c3_5000).partial_cmp(&length) != Some(std::cmp::Ordering::Greater) {
        return Err(ActorConstructionError::PositionLengthAssertion);
    }
    Ok(())
}

#[path = "../animation/runtime/actor_animation_player_services.rs"]
pub(crate) mod player_animation;

#[path = "actor_physical_animation_inputs.rs"]
pub(crate) mod physical_animation_inputs;

#[path = "actor_player_startup.rs"]
pub(crate) mod player_startup;

#[path = "../session/selected_player_startup.rs"]
pub(crate) mod selected_startup;

#[path = "../session/selected_player_tick.rs"]
pub(crate) mod selected_tick;

#[path = "actor_presentation_pose_source.rs"]
pub(crate) mod presentation_pose_source;

impl ActorConstruction {
    pub(crate) fn try_install_push_shared_attributes(
        &self,
        attributes: Arc<crate::skater_push_shared_attributes::SkaterPushSharedAttributes>,
    ) -> Result<(), ActorConstructionError> {
        let mut retained = self
            .push_shared_attributes
            .lock()
            .map_err(|_| ActorConstructionError::Poisoned)?;
        if let Some(current) = retained.as_ref() {
            return if Arc::ptr_eq(current, &attributes) {
                Ok(())
            } else {
                Err(ActorConstructionError::AlreadyAttempted)
            };
        }
        *retained = Some(attributes);
        Ok(())
    }

    pub(crate) fn push_shared_attributes(
        &self,
    ) -> Result<
        Arc<crate::skater_push_shared_attributes::SkaterPushSharedAttributes>,
        crate::generated_motion_load::runtime::MotionUpdateError,
    > {
        use crate::generated_motion_load::runtime::MotionUpdateError as E;
        self.push_shared_attributes
            .lock()
            .map_err(|_| E::Lock)?
            .clone()
            .ok_or(E::MissingOwner("SkaterManager push attributes"))
    }

    pub(crate) fn belongs_to_physics_manager(
        &self,
        manager: SimulationManagerContactAuthority,
    ) -> bool {
        self.authority.entity_key().manager() == manager
    }

    pub(crate) fn try_install_wipeout_recovery_graph(
        &self,
        graph: Arc<crate::skater_wipeout_recovery_graph::SkaterWipeoutRecoveryGraph>,
    ) -> Result<(), ActorConstructionError> {
        let mut retained = self
            .wipeout_recovery_graph
            .lock()
            .map_err(|_| ActorConstructionError::Poisoned)?;
        if let Some(current) = retained.as_ref() {
            return if Arc::ptr_eq(current, &graph) {
                Ok(())
            } else {
                Err(ActorConstructionError::AlreadyAttempted)
            };
        }
        *retained = Some(graph);
        Ok(())
    }

    pub(crate) fn wipeout_recovery_graph(
        &self,
    ) -> Result<
        Arc<crate::skater_wipeout_recovery_graph::SkaterWipeoutRecoveryGraph>,
        crate::generated_motion_load::runtime::MotionUpdateError,
    > {
        use crate::generated_motion_load::runtime::MotionUpdateError as E;
        self.wipeout_recovery_graph
            .lock()
            .map_err(|_| E::Lock)?
            .clone()
            .ok_or(E::MissingOwner("SkaterManager recovery graph"))
    }
}

#[path = "actor_presentation_frame_source.rs"]
pub(crate) mod presentation_frame_source;
