//! `DataSysMan::SubmitFrame` 0x8277bca0's record of a completed tick for the
//! presentation's restore: `CameraPresentation`, `SkaterPresentation`,
//! `LivingWorldPresentation` and `SimPresentation`'s `DoSimSubmit`.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage};
use crate::{
    living_world_grind_services::LivingWorldGrindServices,
    living_world_rain::geometry::Transform,
    presentation_restore::{
        PresentationSubmission,
        living_world::{EntityReplay, LivingWorldSubmission, ObjectReplay, VehicleReplay},
    },
};
use std::sync::Arc;

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, e.to_string())
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The completed tick's submission: the game camera, every skater the tick
    /// published (the first player's, then the later and AI skaters') and the
    /// simulation's update rate. `None` before a tick completes.
    ///
    /// # Errors
    /// An owner the submission reads is missing or its lock is poisoned.
    pub fn presentation_submission(
        &mut self,
    ) -> Result<Option<PresentationSubmission>, SelectedLiveError> {
        if !self.last_step_completed {
            return Ok(None);
        }
        let borrowed = self
            .career
            .as_ref()
            .is_some_and(super::career::CareerSession::first_player_borrowed);
        let inner = &mut self.inner;
        let world = inner.world.as_ref().ok_or_else(|| error("world absent"))?;
        let mut camera = world.local.try_camera_submission().map_err(error)?;
        let (sim_rate, living_world) = {
            let world = world
                .world
                .lock()
                .map_err(|_| error("world lock poisoned"))?;
            let sim_rate = world
                .sim_rate
                .lock()
                .map_err(|_| error("sim rate lock poisoned"))?
                .current_sim_update_rate();
            let living_world = world
                .grind_services
                .living_world()
                .lock()
                .map_err(|_| error("living world lock poisoned"))?;
            (sim_rate, living_world_submission(&living_world))
        };
        let vault = Arc::clone(
            &inner
                .assets
                .as_ref()
                .ok_or_else(|| error("assets absent"))?
                .vault,
        );
        let mut skaters = Vec::new();
        // A NIS that borrows the first player's entity poses its palette itself.
        if !borrowed
            && let Some(runtime) = inner.presentation_runtime.as_mut()
            && let Some(presentation) = inner
                .player
                .as_ref()
                .and_then(|player| player.selected.owners.presentation.as_ref())
            && let Some(frame) = runtime
                .submitted_frame(presentation, &vault)
                .map_err(error)?
        {
            skaters.push(frame);
        }
        for skater in inner.party.iter_mut().chain(inner.ai_skaters.iter_mut()) {
            if skater.presentation_runtime.borrowed() { continue; }
            if let Some(frame) = skater
                .presentation_runtime
                .submitted_frame(&skater.presentation, &vault)
                .map_err(error)?
            {
                skaters.push(frame);
            }
        }
        skaters.extend(self.online_submitted_frames());
        // PC port addition, Skate During Turn Wait: the local game camera, before another player's
        // takes its place, for the corner the local skater's own view draws in.
        let pip_camera = self.online_turn_wait_preview().then(|| camera.clone());
        self.online_camera_submission(&mut camera);
        Ok(Some(PresentationSubmission {
            camera,
            pip_camera,
            skaters,
            living_world,
            tick: self.completed_frames,
            sim_rate,
        }))
    }

    /// Whether the presentation's restore steps the skaters' cloth each frame
    /// (`ClothUpdate` on the presentation side) instead of each tick.
    pub fn set_presentation_cloth(&mut self, presentation: bool) {
        let inner = &mut self.inner;
        if let Some(runtime) = inner.presentation_runtime.as_mut() {
            runtime.set_cloth_by_presentation(presentation);
        }
        for skater in inner.party.iter_mut().chain(inner.ai_skaters.iter_mut()) {
            skater
                .presentation_runtime
                .set_cloth_by_presentation(presentation);
        }
    }
}

/// `LivingWorldPresentation::DoSimSubmit` 0x8280e8b0: the pedestrians, vehicles and
/// dynamic objects in their managers' order, under the IDs their visuals draw.
fn living_world_submission(owner: &LivingWorldGrindServices) -> LivingWorldSubmission {
    let rows = |transform: &Transform| -> [f32; 16] {
        std::array::from_fn(|index| transform[index / 4][index % 4])
    };
    let pedestrians = owner
        .pedestrian_entities
        .as_deref()
        .map(|roster| {
            roster
                .ids()
                .into_iter()
                .filter_map(|id| {
                    let core = roster.get(id)?.core.as_ref()?;
                    Some(EntityReplay::new(id, rows(&core.transform)))
                })
                .collect()
        })
        .unwrap_or_default();
    let vehicles = owner
        .grind_vehicle_entities
        .as_deref()
        .map(|roster| {
            roster
                .ids()
                .into_iter()
                .filter_map(|id| {
                    let vehicle = roster.get(id)?;
                    Some(VehicleReplay::new(
                        id,
                        rows(&vehicle.core.transform),
                        vehicle.wheels.spin_angle,
                        vehicle.wheels.turn_angle,
                        vehicle.route.lights,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    // A dynamic object's one part is the object's own transform.
    let objects = owner
        .dmo_objects
        .as_deref()
        .map(|objects| {
            objects
                .ids()
                .into_iter()
                .filter_map(|id| objects.object(id))
                .map(|object| {
                    ObjectReplay::new(object.entity_id, object.transform, object.transform)
                })
                .collect()
        })
        .unwrap_or_default();
    LivingWorldSubmission {
        pedestrians,
        vehicles,
        objects,
    }
}
