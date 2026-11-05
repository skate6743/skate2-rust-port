//! Same Actor and Simulation owners for asynchronous body evaluation.
//! References: `DispatchBodyAnim825f1040` and `UpdatePhysics825ffd98`.
//! `GetPhysUpdateData82c96d58` remains a later consumer: its trajectory and
//! post-evaluation producers must run before its two pose copies.
use super::{ActorConstruction, ActorConstructionError, animation_mount::MountedActorAnimation};
use crate::{
    actor_animation_construction::initialization::{
        ActorAnimationInitializeSource,
        body_dispatch::{BodyAnimationDispatchSource, BodyDispatchError},
    },
    actor_face_playback::{ActorFaceError, FaceCommandManager},
    animation_job_owners::AnimationJobManager,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    simulation_animation_startup::SimulationAnimationStartup,
};
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[path = "actor_animation_body_session_corpus.rs"]
pub(crate) mod corpus;

#[derive(Debug)]
pub(crate) enum BodySessionError {
    Actor(ActorConstructionError),
    Animation(ActorFaceError),
    Cvar(InputCvarError),
    MissingJobManager,
    MissingWalking(&'static str),
    Physical(super::physical_mount::ActorPhysicalMountError),
    Output(
        crate::physical_player_phys_out::construct::ConcretePhysOutError<std::convert::Infallible>,
    ),
    Walking(super::motion_runtime_session::neutral_lifecycle::WalkingWorldError),
    Create(
        crate::actor_animation_construction::locomotion_create::CreateError<
            crate::reckoning_numeric::ReckoningHostNumericError,
            std::convert::Infallible,
        >,
    ),
    UnpublishedAnimation,
    ForeignAnimation,
    Lock,
}
impl std::fmt::Display for BodySessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "body animation Actor: {error}"),
            Self::Animation(error) => write!(f, "body animation startup: {error:?}"),
            Self::Cvar(error) => write!(f, "body animation cvar: {error:?}"),
            other => write!(f, "body animation: {other:?}"),
        }
    }
}

/// These are the real shared startup services. Cvar targets remain shared and
/// are sampled on every dispatch; no threshold is cached into this binding.
pub(crate) struct ActorBodyAnimationServices {
    commands: Arc<Mutex<FaceCommandManager>>,
    jobs: Arc<AnimationJobManager>,
    cvars: InputCvarReferences,
}
impl ActorBodyAnimationServices {
    pub(crate) fn try_from_startup(
        startup: &SimulationAnimationStartup,
        cvars: &InputCvarReferences,
    ) -> Result<Self, BodySessionError> {
        // Resolve before entering any buffered ACS lock: retained_job_manager
        // visits that same buffered owner while recovering its startup lease.
        let jobs = startup
            .retained_job_manager()
            .map_err(BodySessionError::Animation)?
            .ok_or(BodySessionError::MissingJobManager)?;
        Ok(Self {
            commands: Arc::clone(&startup.managers.command),
            jobs,
            cvars: cvars.clone(),
        })
    }
}
impl ActorAnimationInitializeSource for ActorBodyAnimationServices {
    type Error = BodySessionError;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_use_jobs_for_skater_anim_update()
            .map_err(BodySessionError::Cvar)
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        Ok(Arc::clone(&self.jobs))
    }
}
impl BodyAnimationDispatchSource for ActorBodyAnimationServices {
    fn blend_space_cvars(&self) -> Option<InputCvarReferences> {
        Some(self.cvars.clone())
    }
    fn body_command_manager(&mut self) -> Result<Arc<Mutex<FaceCommandManager>>, Self::Error> {
        Ok(Arc::clone(&self.commands))
    }
    fn blend_cull_threshold(&mut self, low: bool) -> Result<f32, Self::Error> {
        self.cvars
            .try_skater_blend_cull_threshold(low)
            .map_err(BodySessionError::Cvar)
    }
}
impl ActorConstruction {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_select_skate3_movement(&self, selected: bool) -> Result<(), BodySessionError> {
        self.checked_body_animation()?.lock().map_err(|_| BodySessionError::Lock)?
            .select_skate3_movement(selected);
        Ok(())
    }

    /// Actual retained phase2 animation calls; outer Actor physical checks are
    /// still the scheduler's responsibility.
    pub(crate) fn try_update_animation_phase2(
        &self,
        wait_for_bs_job: bool,
        cvars: &InputCvarReferences,
    ) -> Result<
        (),
        crate::actor_animation_construction::frame_phase::AnimationPhaseDispatchError<
            BodySessionError,
        >,
    > {
        use crate::actor_animation_construction::frame_phase::AnimationPhaseDispatchError as E;
        let animation = self
            .checked_body_animation()
            .map_err(|e| E::Dispatch(BodyDispatchError::Source(e)))?;
        animation
            .lock()
            .map_err(|_| E::Dispatch(BodyDispatchError::Source(BodySessionError::Lock)))?
            .try_animation_phase2(wait_for_bs_job, cvars)
            .map_err(E::Phase)
    }

    /// Phase3 advances time/attributes before body Dispatch on this Actor.
    pub(crate) fn try_update_animation_phase3(
        &self,
        dt: f32,
        services: &mut ActorBodyAnimationServices,
        world: &mut dyn super::motion_runtime_session::neutral_lifecycle::ForcePhysicsWriter,
    ) -> Result<
        (),
        crate::actor_animation_construction::frame_phase::AnimationPhaseDispatchError<
            BodySessionError,
        >,
    > {
        use crate::actor_animation_construction::frame_phase::AnimationPhaseDispatchError as E;
        let animation = self
            .checked_body_animation()
            .map_err(|e| E::Dispatch(BodyDispatchError::Source(e)))?;
        // Native825f4410 asserts IPhysical before Was/AddTime/Toggle.
        let _physical = self
            .cached_physical_allocation()
            .map_err(|e| E::Dispatch(BodyDispatchError::Source(BodySessionError::Physical(e))))?;
        let needs_create = animation
            .lock()
            .map_err(|_| E::Dispatch(BodyDispatchError::Source(BodySessionError::Lock)))?
            .try_animation_phase3_prefix(dt)
            .map_err(E::Phase)?;
        if needs_create {
            let continuation = self
                .try_create_walking_step(dt, &animation, services, world)
                .map_err(|e| E::Dispatch(BodyDispatchError::Source(e)))?;
            // Launch locks the same animation when publishing its real future.
            // No animation/Actor/PhysOut guard survives the mutation helper.
            world
                .launch_walking_steps(
                    &animation,
                    continuation.first,
                    continuation.count,
                    continuation.world_only,
                )
                .map_err(|e| {
                    E::Dispatch(BodyDispatchError::Source(BodySessionError::Walking(e)))
                })?;
        }
        animation
            .lock()
            .map_err(|_| E::Dispatch(BodyDispatchError::Source(BodySessionError::Lock)))?
            .try_animation_phase3_suffix(services)
    }

    /// Actor825f44f8..45ac: separate guarded PhysOut reads in original order.
    fn try_create_walking_step(
        &self,
        dt: f32,
        animation: &Arc<Mutex<MountedActorAnimation>>,
        services: &mut ActorBodyAnimationServices,
        world: &mut dyn super::motion_runtime_session::neutral_lifecycle::ForcePhysicsWriter,
    ) -> Result<
        crate::actor_animation_construction::locomotion_create::StepLaunchContinuation,
        BodySessionError,
    > {
        use BodySessionError as E;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let world_only = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_state_output()
            .map_err(E::Output)?
            .collide_with_world_only;
        let proxy = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::Output)?
            .is_proxy_in_collision;
        let velocity = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_camera_system_vector(
                crate::physical_player_phys_out::PhysOutSystemVector::CentreOfMassVelocity,
            )
            .map_err(E::Output)?;
        let displacement = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::Output)?
            .skeleton_hook_to_hips;
        let mg_attributes = {
            let actor = self.prefix().map_err(E::Actor)?;
            let state = actor
                .graphs()
                .and_then(|g| g.motion())
                .and_then(|g| g.state.as_ref())
                .ok_or(E::MissingWalking("Motion graph attributes"))?;
            state
                .attributes()
                .iter()
                .map(|a| crate::actor_graph_construction::ActorMgAttribute {
                    name: a.name,
                    value: a.value,
                })
                .collect::<Vec<_>>()
        };
        let mut guard = animation.lock().map_err(|_| E::Lock)?;
        let attributes = guard
            .base()
            .ok_or(E::MissingWalking("animation attribute owner"))?
            .tree_attributes
            .occupied_slots()
            .iter()
            .map(|a| {
                a.as_ref()
                    .cloned()
                    .ok_or(E::MissingWalking("occupied animation attribute"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let overlay = world.walking_debug_overlay().map_err(E::Walking)?;
        guard
            .try_create_locomotion_trajectory(
                dt,
                &attributes,
                &mg_attributes,
                displacement,
                velocity,
                proxy,
                world_only,
                &services.cvars,
                &overlay,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            )
            .map_err(E::Create)
    }

    pub(super) fn checked_body_animation(
        &self,
    ) -> Result<Arc<Mutex<MountedActorAnimation>>, BodySessionError> {
        let state = self.prefix().map_err(BodySessionError::Actor)?;
        if !state.animation_published {
            return Err(BodySessionError::UnpublishedAnimation);
        }
        let animation = state
            .animation
            .as_ref()
            .ok_or(BodySessionError::UnpublishedAnimation)?;
        let occurrence = state.animation_interfaces[2]
            .as_ref()
            .ok_or(BodySessionError::ForeignAnimation)?;
        if !Arc::ptr_eq(
            occurrence,
            animation
                .lock()
                .map_err(|_| BodySessionError::Lock)?
                .occurrence(),
        ) {
            return Err(BodySessionError::ForeignAnimation);
        }
        Ok(Arc::clone(animation))
    }
    /// Dispatch queues writes into the same Initialize-time local/global buffers.
    /// It must remain separate from the later physics synchronization boundary.
    pub(crate) fn try_dispatch_body_frame(
        &self,
        services: &mut ActorBodyAnimationServices,
    ) -> Result<(), BodyDispatchError<BodySessionError>> {
        let animation = self
            .checked_body_animation()
            .map_err(BodyDispatchError::Source)?;
        animation
            .lock()
            .map_err(|_| BodyDispatchError::Source(BodySessionError::Lock))?
            .try_dispatch_body_animation(services)
    }
    /// `UpdatePhysics825ffea8` synchronizes this Actor before its update-mask test.
    /// This entry intentionally does not claim the subsequent `SetUpPhysics` call.
    pub(crate) fn try_sync_body_frame(
        &self,
        services: &mut ActorBodyAnimationServices,
    ) -> Result<(), BodyDispatchError<BodySessionError>> {
        let animation = self
            .checked_body_animation()
            .map_err(BodyDispatchError::Source)?;
        animation
            .lock()
            .map_err(|_| BodyDispatchError::Source(BodySessionError::Lock))?
            .try_sync_body_animation(services)
    }
}
