//! Retained notification fields of `GameCameraManager82e37c08`. This projection
//! does not claim completed Subject, `IrisCore`, rig or camera construction.
#[path = "../camera_subject_update.rs"]
pub(crate) mod subject_update;

#[path = "../camera_reference_points.rs"]
pub(crate) mod reference_points;

#[cfg(feature = "skate3-runtime")]
#[path = "camera_skate3.rs"]
mod skate3;
use crate::actor_construction::ActorConstruction;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum CameraStartupError {
    PendingCameraManConstruction,
    Selected(selected_runtime::CameraSelectedError),
    CameraSkaterDestroyed,
}

/// Actual constructor writes null Actor and empty `CameraMan`/graph lists before
/// first local notification. Subsequent camera work extends this same owner.
pub(crate) struct CameraStartupFields {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained null Actor awaits camera construction continuation"
        )
    )]
    actor: Option<Arc<ActorConstruction>>,
    camera_man_count: usize,
    graph_count: usize,
    retained_parameters: Option<Arc<std::sync::Mutex<crate::camera_attributes::CameraAttributes>>>,
    construction_state: Option<construction::CameraConstructionState>,
    selected_inputs: Option<selected_runtime::CameraSelectedInputs>,
    producer_snapshot_ready: bool,
    /// The projection of the last completed update: CameraPresentation keeps showing the
    /// frame it was last submitted while a local-skater change waits for the next update.
    completed_projection: Option<producer_projection::CameraProducerProjection>,
    /// `CameraPresentation::DoSimSubmit`'s record of the last completed update.
    completed_submission: Option<crate::presentation_restore::camera::CameraSubmission>,
    /// First CameraManager::mCameraList BasicCamera, distinct from CameraMan's
    /// working frame. Simulation consumers can read it before the first tick.
    simulation_camera_matrix: Option<crate::skeleton_root_transforms::SkeletonRootMatrix>,
    global: Option<selected_runtime::CameraGlobalCondition>,
    /// PC port (Skate 3 physics): the frame field of view the Skate 3 camera replaced, given
    /// back when the Skate 2 camera runs again.
    #[cfg(feature = "skate3-runtime")]
    skate2_field_of_view: Option<f32>,
}
impl CameraStartupFields {
    pub(crate) const fn construct_notification_fields() -> Self {
        Self {
            actor: None,
            camera_man_count: 0,
            graph_count: 0,
            retained_parameters: None,
            construction_state: None,
            selected_inputs: None,
            producer_snapshot_ready: false,
            completed_projection: None,
            completed_submission: None,
            simulation_camera_matrix: None,
            global: None,
            #[cfg(feature = "skate3-runtime")]
            skate2_field_of_view: None,
        }
    }
    /// Camera observer82581c08 forwards to folded825187c0, an actual blr.
    #[expect(clippy::unused_self, reason = "actual folded native observer callback")]
    pub(crate) fn skater_created(&self) {}
    pub(crate) fn try_simulation_camera_matrix(
        &self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, CameraStartupError> {
        self.simulation_camera_matrix
            .ok_or(CameraStartupError::Selected(
                selected_runtime::CameraSelectedError::MissingConstruction,
            ))
    }
    /// `OnLocalSkaterChanged82e3a758` first creates cameras, then `SetLocalSkater`.
    /// No Actor/PhysOut write may occur before that construction succeeds.
    pub(crate) fn local_changed(
        &mut self,
        actor: Option<&Arc<ActorConstruction>>,
    ) -> Result<(), CameraStartupError> {
        if self.selected_inputs.is_none() {
            return Err(CameraStartupError::PendingCameraManConstruction);
        }
        self.try_local_changed_selected(actor)
            .map_err(CameraStartupError::Selected)
    }
    /// `CameraManager::OnSkaterDestroyed` 0x82581c10 -> `GameCameraManager::OnSkaterDestroyed`
    /// 0x82e363e0: another skater leaves the cameras alone. The camera's own skater would take
    /// its CameraMan instances and the local skater with it (`DestroyCameraManInstances`,
    /// `SetLocalSkater(null)`), which is not ported and is refused.
    pub(crate) fn skater_destroyed(&self, actor: &ActorConstruction) -> Result<(), CameraStartupError> {
        if self.actor.as_ref().is_some_and(|camera| std::ptr::eq(Arc::as_ptr(camera), actor)) {
            return Err(CameraStartupError::CameraSkaterDestroyed);
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn actor(&self) -> Option<&Arc<ActorConstruction>> {
        self.actor.as_ref()
    }
    /// `GetCameraSkater82581bf8` is GameCameraManager+0xd0. When it is the
    /// caller, `GetCameraSkaterOpacity82e34960` returns 0.0 for an empty CameraMan
    /// list (+0x50 == +0x54), otherwise the first CameraMan+0x36c (UpdateRig).
    pub(crate) fn camera_skater_opacity(
        &self,
        actor: &ActorConstruction,
    ) -> Result<Option<f32>, CameraStartupError> {
        if !self
            .actor
            .as_ref()
            .is_some_and(|camera| std::ptr::eq(Arc::as_ptr(camera), actor))
        {
            return Ok(None);
        }
        if self.camera_man_count == 0 {
            return Ok(Some(0.0));
        }
        let man = self
            .construction_state
            .as_ref()
            .and_then(|state| state.camera_man.as_ref())
            .ok_or(CameraStartupError::PendingCameraManConstruction)?;
        let opacity = man
            .lock()
            .map_err(|_| {
                CameraStartupError::Selected(selected_runtime::CameraSelectedError::OwnerLock)
            })?
            .opacity;
        Ok(Some(opacity))
    }
}

#[path = "../camera_construction_probe.rs"]
pub(crate) mod construction;
#[path = "camera_producer_projection.rs"]
pub(crate) mod producer_projection;
#[path = "camera_selected_runtime.rs"]
pub(crate) mod selected_runtime;
