//! Actual selected observer order and first-player callback prefix.
use super::{LocalIdentityError, SkaterManagerLocalIdentity};
use crate::{
    actor_construction::{ActorConstruction, ActorConstructionError},
    camera_session::{
        CameraStartupError, CameraStartupFields,
        producer_projection::{CameraProducerProjection, CameraProducerProjectionError},
    },
    hermes_challenge_queue::{
        ChallengeEventBroadcaster, ChallengeEventQueue, ChallengeQueueError, SkaterNotification,
    },
    physical_player_physics_state::PhysicalStateActorQuery,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SkaterObserverError {
    Identity(LocalIdentityError),
    Actor(ActorConstructionError),
    Queue(ChallengeQueueError),
    Camera(CameraStartupError),
    CameraProjection(CameraProducerProjectionError),
    Allocation,
    AlreadyRegistered,
    PendingWorkspaceObservers,
}
impl std::fmt::Display for SkaterObserverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identity(error) => write!(f, "observer identity: {error:?}"),
            Self::Actor(error) => write!(f, "observer Actor: {error:?}"),
            Self::Queue(error) => error.fmt(f),
            Self::Camera(error) => write!(f, "observer camera: {error:?}"),
            other => write!(f, "first-player observers: {other:?}"),
        }
    }
}

/// Census ctor827209f8 writes FE to actual object+A8. The observer subobject
/// is +4, so `OnCreated8271c170` `ORs1` at its+A4. No Census population is invented.
struct CensusNotificationFields {
    flags: Mutex<u8>,
}
#[derive(Clone)]
enum Observer {
    Census(Arc<CensusNotificationFields>),
    Challenge(Arc<ChallengeEventBroadcaster>),
    Camera(Arc<Mutex<CameraStartupFields>>),
}

pub(crate) struct SkaterObserverState {
    observers: Vec<Observer>,
    registration_attempted: bool,
    registered: bool,
    queue: Option<Arc<ChallengeEventQueue>>,
}
impl SkaterObserverState {
    /// SkaterManager825fb350/358/368: actual empty list and no dispatch.
    pub(crate) const fn construct_empty() -> Self {
        Self {
            observers: Vec::new(),
            registration_attempted: false,
            registered: false,
            queue: None,
        }
    }
    fn append(&mut self, observer: Observer) -> Result<(), SkaterObserverError> {
        // Fresh concrete endpoint allocation ensures source no-duplicate check.
        self.observers
            .try_reserve(1)
            .map_err(|_| SkaterObserverError::Allocation)?;
        self.observers.push(observer);
        Ok(())
    }
    fn snapshot(&self) -> Result<Vec<Observer>, SkaterObserverError> {
        if !self.registered {
            return Err(SkaterObserverError::PendingWorkspaceObservers);
        }
        let mut snapshot = Vec::new();
        snapshot
            .try_reserve_exact(self.observers.len())
            .map_err(|_| SkaterObserverError::Allocation)?;
        snapshot.extend(self.observers.iter().cloned());
        Ok(snapshot)
    }
}

fn actor_id(actor: &Arc<ActorConstruction>) -> Result<i32, SkaterObserverError> {
    let mut query = actor.as_ref();
    PhysicalStateActorQuery::player_id(&mut query).map_err(SkaterObserverError::Actor)
}

impl SkaterManagerLocalIdentity {
    /// Selected workspace callback producer: Census ctor82720a74, then AI.Init
    ///825c1298 (`GameInit`), then Camera.Init82584cdc. Queue is a retained earlier
    ///`EventSystem` owner, never an empty queue manufactured at notification.
    pub(crate) fn try_register_startup_observers(
        &self,
        queue: Arc<ChallengeEventQueue>,
    ) -> Result<(), SkaterObserverError> {
        let mut fields = self
            .fields
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?;
        let state = &mut fields.observers;
        if state.registration_attempted {
            return Err(SkaterObserverError::AlreadyRegistered);
        }
        state.registration_attempted = true;
        state.append(Observer::Census(Arc::new(CensusNotificationFields {
            flags: Mutex::new(0xfe),
        })))?;
        state.queue = Some(Arc::clone(&queue));
        state.append(Observer::Challenge(Arc::new(
            ChallengeEventBroadcaster::construct(queue),
        )))?;
        state.append(Observer::Camera(Arc::new(Mutex::new(
            CameraStartupFields::construct_notification_fields(),
        ))))?;
        state.registered = true;
        Ok(())
    }

    /// The Challenge queue (queue 7) the registered broadcaster fills.
    pub(crate) fn try_challenge_queue(&self) -> Result<Option<Arc<ChallengeEventQueue>>, LocalIdentityError> {
        Ok(self.fields.lock().map_err(|_| LocalIdentityError::Poisoned)?.observers.queue.clone())
    }

    /// Borrow the broadcaster already registered by Challenge.GameInit. Chase
    /// producers must share its queue, port and mutex with skater notifications.
    pub(crate) fn try_challenge_broadcaster(&self)
        -> Result<Arc<ChallengeEventBroadcaster>, SkaterObserverError> {
        self.observer_snapshot()?.into_iter().find_map(|observer| match observer {
            Observer::Challenge(broadcaster) => Some(broadcaster),
            _ => None,
        }).ok_or(SkaterObserverError::PendingWorkspaceObservers)
    }

    /// Explicit selected capability capture on the SAME registered observer.
    /// No roster lock is retained across camera construction or notifications.
    pub(crate) fn try_capture_selected_camera(
        &self,
        inputs: crate::camera_session::selected_runtime::CameraSelectedInputs,
    ) -> Result<(), SkaterObserverError> {
        let observers = self.observer_snapshot()?;
        let camera = observers
            .into_iter()
            .find_map(|observer| match observer {
                Observer::Camera(camera) => Some(camera),
                _ => None,
            })
            .ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .try_capture_selected(inputs)
            .map_err(|source| SkaterObserverError::Camera(CameraStartupError::Selected(source)))
    }

    /// Scheduled selected producer; no completed full-camera frame is implied.
    pub(crate) fn try_update_selected_camera(
        &self,
        dt: f32,
        authority: crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
        services: &mut crate::simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices<'_>,
    ) -> Result<(), SkaterObserverError> {
        let observers = self.observer_snapshot()?;
        let camera = observers
            .into_iter()
            .find_map(|observer| match observer {
                Observer::Camera(camera) => Some(camera),
                _ => None,
            })
            .ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .try_update_selected_prefix(dt, authority, services)
            .map_err(|source| SkaterObserverError::Camera(CameraStartupError::Selected(source)))
    }
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_publish_skate3_camera(&self, output: &crate::skate3_native::FrameOutput,
        owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>, dt: f32)
        -> Result<(), SkaterObserverError> {
        let camera = self.observer_snapshot()?.into_iter().find_map(|observer| match observer {
            Observer::Camera(camera) => Some(camera), _ => None,
        }).ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera.lock().map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .try_publish_skate3_camera(output, owner, dt).map_err(SkaterObserverError::CameraProjection)
    }
    /// Read the same retained camera observer after a successful selected tick.
    /// This returns a provisional producer projection, not restored presentation.
    pub(crate) fn try_set_camera_profile_settings(
        &self,
        camera_type: i32,
        invert_x: bool,
        invert_y: bool,
        grind_detector: bool,
    ) -> Result<(), SkaterObserverError> {
        let camera = self.observer_snapshot()?.into_iter().find_map(|observer| match observer {
            Observer::Camera(camera) => Some(camera),
            _ => None,
        });
        if let Some(camera) = camera {
            camera
                .lock()
                .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
                .set_profile_settings(camera_type, invert_x, invert_y, grind_detector);
        }
        Ok(())
    }
    pub(crate) fn try_camera_producer_projection(
        &self,
    ) -> Result<CameraProducerProjection, SkaterObserverError> {
        let camera = self
            .observer_snapshot()?
            .into_iter()
            .find_map(|observer| match observer {
                Observer::Camera(camera) => Some(camera),
                _ => None,
            })
            .ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .try_provisional_camera_projection()
            .map_err(SkaterObserverError::CameraProjection)
    }

    /// `CameraPresentation::DoSimSubmit`'s record of the camera observer's last
    /// completed update.
    pub(crate) fn try_camera_submission(
        &self,
    ) -> Result<crate::presentation_restore::camera::CameraSubmission, SkaterObserverError> {
        let camera = self
            .observer_snapshot()?
            .into_iter()
            .find_map(|observer| match observer {
                Observer::Camera(camera) => Some(camera),
                _ => None,
            })
            .ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .completed_camera_submission()
            .ok_or(SkaterObserverError::CameraProjection(
                crate::camera_session::producer_projection::CameraProducerProjectionError::NoCompletedSelectedUpdate,
            ))
    }

    /// The same observer's projection of the ICE camera a NIS drives.
    pub(crate) fn try_ice_camera_projection(
        &self,
        matrix: Option<[f32; 16]>,
        fov: f32,
        near_plane: f32,
    ) -> Result<CameraProducerProjection, SkaterObserverError> {
        let camera = self
            .observer_snapshot()?
            .into_iter()
            .find_map(|observer| match observer {
                Observer::Camera(camera) => Some(camera),
                _ => None,
            })
            .ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .try_ice_camera_projection(matrix, fov, near_plane)
            .map_err(SkaterObserverError::CameraProjection)
    }

    /// LivingWorld reads CameraManager::mCameraList[0], including before the
    /// first CameraMan update. This is not the guarded render projection.
    pub(crate) fn try_simulation_camera_matrix(
        &self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, SkaterObserverError> {
        let camera = self
            .observer_snapshot()?
            .into_iter()
            .find_map(|observer| match observer {
                Observer::Camera(camera) => Some(camera),
                _ => None,
            })
            .ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .try_simulation_camera_matrix()
            .map_err(SkaterObserverError::Camera)
    }

    /// `Simulation::GetCameraManager` for UpdateSkaterOpacity825f1368: the same
    /// retained camera observer answers `GetCameraSkater` and its opacity.
    pub(crate) fn try_camera_skater_opacity(
        &self,
        actor: &ActorConstruction,
    ) -> Result<Option<f32>, SkaterObserverError> {
        let camera = self
            .observer_snapshot()?
            .into_iter()
            .find_map(|observer| match observer {
                Observer::Camera(camera) => Some(camera),
                _ => None,
            })
            .ok_or(SkaterObserverError::PendingWorkspaceObservers)?;
        camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .camera_skater_opacity(actor)
            .map_err(SkaterObserverError::Camera)
    }

    fn observer_snapshot(&self) -> Result<Vec<Observer>, SkaterObserverError> {
        self.fields
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .observers
            .snapshot()
    }

    /// Created Notify8260036c, followed by actual `UpdateLocalPlayer825ffef8`.
    pub(crate) fn try_notify_first_actor(
        &self,
        actor: &Arc<ActorConstruction>,
    ) -> Result<(), SkaterObserverError> {
        for observer in self.observer_snapshot()? {
            match observer {
                Observer::Census(census) => {
                    *census.flags.lock().map_err(|_| {
                        SkaterObserverError::Identity(LocalIdentityError::Poisoned)
                    })? |= 1;
                }
                Observer::Challenge(broadcaster) => broadcaster
                    .send(SkaterNotification::Created, actor_id(actor)?)
                    .map_err(SkaterObserverError::Queue)?,
                Observer::Camera(camera) => camera
                    .lock()
                    .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
                    .skater_created(),
            }
        }
        self.try_update_local_player()
    }

    /// `DestroyPlayer`'s Destroyed notification (observer vtable+8): Census's is the folded
    /// empty function, `ChallengeAIManager::OnSkaterDestroyed` 0x825b84e0 sends the player's
    /// Destroyed message and `CameraManager::OnSkaterDestroyed` 0x82581c10 hands the Actor to
    /// its `GameCameraManager`.
    pub(crate) fn try_notify_destroyed_actor(
        &self,
        actor: &Arc<ActorConstruction>,
    ) -> Result<(), SkaterObserverError> {
        for observer in self.observer_snapshot()? {
            match observer {
                Observer::Census(_) => {}
                Observer::Challenge(broadcaster) => broadcaster
                    .send(SkaterNotification::Destroyed, actor_id(actor)?)
                    .map_err(SkaterObserverError::Queue)?,
                Observer::Camera(camera) => camera
                    .lock()
                    .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
                    .skater_destroyed(actor)
                    .map_err(SkaterObserverError::Camera)?,
            }
        }
        Ok(())
    }

    /// `SetLocalIndex` 0x82600380: with the ghost system on, an offline game's
    /// index 0 is the ghost's 8; then `UpdateLocalPlayer`.
    pub(crate) fn try_set_local_index(&self, index: i32) -> Result<(), SkaterObserverError> {
        let ghost = self
            .backend()
            .try_ghost_enabled()
            .map_err(SkaterObserverError::Identity)?;
        let network = self
            .backend()
            .try_is_network_game()
            .map_err(SkaterObserverError::Identity)?;
        if ghost && index >= 0 && network {
            return Err(SkaterObserverError::Identity(LocalIdentityError::PendingNetworkReset));
        }
        let index = if ghost && index == 0 { 8 } else { index };
        self.fields
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .local_index = index;
        self.try_update_local_player()
    }

    /// `UpdateLocalPlayer` 0x825ffef8: the local Actor is `FindByIndex(mLocalIndex)`;
    /// a change notifies LocalChanged and drops the recorder.
    pub(crate) fn try_update_local_player(&self) -> Result<(), SkaterObserverError> {
        // Old local capture precedes FindByIndex; publish the returned SAME Arc
        // before notification. Do not retain the manager lock across callbacks.
        let (old, index) = {
            let fields = self
                .fields
                .lock()
                .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?;
            (fields.local.clone(), fields.local_index)
        };
        let local = self
            .try_find_by_index(index)
            .map_err(SkaterObserverError::Identity)?;
        self.fields
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .local
            .clone_from(&local);
        let unchanged = match (&old, &local) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        if !unchanged {
            for observer in self.observer_snapshot()? {
                match observer {
                    Observer::Census(_) => {} // vtable+12 -> actual folded blr
                    Observer::Challenge(broadcaster) => {
                        let id = match &local {
                            Some(actor) => actor_id(actor)?,
                            None => -1,
                        };
                        broadcaster
                            .send(SkaterNotification::LocalChanged, id)
                            .map_err(SkaterObserverError::Queue)?;
                    }
                    Observer::Camera(camera) => camera
                        .lock()
                        .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
                        .local_changed(local.as_ref())
                        .map_err(SkaterObserverError::Camera)?,
                }
            }
            // UpdateLocalPlayer825fff4c..74: only after all callbacks, capture
            // the retained recorder, delete if present, then publish null.
            // This selected owner has admitted no recorder allocation.
            let mut fields = self
                .fields
                .lock()
                .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?;
            if let Some(recorder) = fields.recorder.take() {
                match recorder {}
            }
            fields.recorder = None;
        }
        Ok(())
    }

    pub(crate) fn try_local_actor(
        &self,
    ) -> Result<Option<Arc<ActorConstruction>>, LocalIdentityError> {
        Ok(self
            .fields
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?
            .local
            .clone())
    }

    /// Actual retained queue/callback fields, for scoped original-data validation.
    #[cfg(test)]
    pub(crate) fn observer_evidence(
        &self,
    ) -> Result<(u8, Arc<ChallengeEventQueue>, bool), SkaterObserverError> {
        let fields = self
            .fields
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?;
        let state = &fields.observers;
        let [
            Observer::Census(census),
            Observer::Challenge(_),
            Observer::Camera(camera),
        ] = state.observers.as_slice()
        else {
            return Err(SkaterObserverError::PendingWorkspaceObservers);
        };
        let flags = *census
            .flags
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?;
        let camera_null = camera
            .lock()
            .map_err(|_| SkaterObserverError::Identity(LocalIdentityError::Poisoned))?
            .actor()
            .is_none();
        Ok((
            flags,
            Arc::clone(state.queue.as_ref().expect("registered Challenge queue")),
            camera_null,
        ))
    }
}

impl crate::actor_construction::animation_phase1::SkaterOpacityCamera
    for SkaterManagerLocalIdentity
{
    fn camera_skater_opacity(&self, actor: &ActorConstruction) -> Result<Option<f32>, String> {
        self.try_camera_skater_opacity(actor)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "skater_manager_observers_tests.rs"]
mod tests;
