//! `cChallengeDMOManager`'s calls into the LivingWorld dynamic object managers: the culls and
//! locator changes of a challenge's `tDMOResetInfo`s and their unwinding, the challenge DMO
//! groups' locators and their reset.
use super::SelectedRecipeFrame;
use crate::{
    challenge::dmo::reset_type,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        LivingWorldGrindServices,
        dmo_locators::{DynamicObjectLocatorMan, LocatorData},
        dmo_objects::{DmoCullEvent, DmoObjectMan, FrameLocatorServices, bbox_volume},
    },
    narrow_phase::{GpPrimitive, rw_overlap},
    physical_player_assembly::PhysicalAssemblyWorld,
    selected_terrain_startup::dmo::ChallengeDmoChunks,
    world_painter_layer::WorldPainterLayerMan,
};
use skate2_formats::collision::{Volume, VolumeType};

/// A trigger instance `cTriggerVolumeManager::GetTriggerInstance` (0x82e21cc0) found: its
/// `m_uiGuid` and what `TriggerVolumeUtils::IsColldingWithTriggerVolume` (0x82e213b8) tests
/// against, its collision model's bounding volume at `m_TransformMatrix` (none without a model,
/// which nothing collides with).
pub(crate) struct ChallengeTrigger {
    pub(crate) guid: u64,
    bounds: Option<(Volume, [f32; 16])>,
}

impl ChallengeTrigger {
    /// `IsColldingWithTriggerVolume(volume, transform, trigger, false)`.
    fn collides(&self, volume: &Volume, transform: [f32; 16]) -> bool {
        let Some((trigger_volume, trigger_transform)) = self.bounds.as_ref() else {
            return false;
        };
        GpPrimitive::from_volume(volume, Some(transform))
            .zip(GpPrimitive::from_volume(trigger_volume, Some(*trigger_transform)))
            .and_then(|(volume, trigger)| rw_overlap(&volume, &trigger, 0.0))
            .unwrap_or(false)
    }

    /// The locator walks' test: `PhysicsUtility::CreateCollisionSphere(0.1)` (0x82e4af08,
    /// `SphereVolume::Initialize` 0x82ad9438: identity rows, group and surface 0, flags 1) at the
    /// transform.
    fn holds_locator(&self, transform: &[f32; 16]) -> bool {
        let sphere = Volume {
            transform: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 0.0,
            ],
            vtable_offset: VolumeType::Sphere as u32,
            shape_words: [0; 3],
            radius: 0.1,
            group_id: 0,
            surface_id: 0,
            flags: 1,
        };
        self.collides(&sphere, *transform)
    }
}

/// `DynamicObject::RequestExplicitCull` (0x82d36390) as a walk calls it, with the managers.
type Cull<'c> = dyn FnMut(&mut DmoObjectMan, &mut DynamicObjectLocatorMan, u32) -> Result<(), String> + 'c;

impl SelectedRecipeFrame<'_> {
    /// `cTriggerVolumeManager::GetTriggerInstance` (0x82e21cc0) of a `tTriggerVolumeInstanceID`,
    /// as the float the native converts it to.
    pub(crate) fn challenge_trigger(&self, volume: u64) -> Result<Option<ChallengeTrigger>, String> {
        let triggers = self.triggers.lock().map_err(|_| "challenge trigger lock")?;
        let id = crate::skate3::id_float(volume);
        let Some(record) = triggers
            .try_find_volume_id(id)
            .map_err(|e| format!("challenge trigger lookup: {e:?}"))?
        else {
            return Ok(None);
        };
        Ok(Some(ChallengeTrigger {
            guid: record.trigger_guids()?.0,
            bounds: record.trigger_bounding_volume()?,
        }))
    }

    /// Runs `body` over the dynamic object and locator managers with `RequestExplicitCull` at
    /// hand: a culled object's children and monitors get the frame's services, as a census
    /// cull's do.
    pub(crate) fn with_dmo_culls<T>(
        &self,
        living: &mut LivingWorldGrindServices,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        painter: &WorldPainterLayerMan,
        body: impl FnOnce(&mut DmoObjectMan, &mut DynamicObjectLocatorMan, &mut Cull<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let actors = world
            .entity_roster
            .as_ref()
            .ok_or("challenge DMO cull Actor roster absent")?
            .snapshot()
            .map_err(|e| format!("challenge DMO cull Actor roster: {e:?}"))?;
        let avoid = {
            let triggers = self.triggers.lock().map_err(|_| "challenge DMO cull triggers lock")?;
            self.runtime.challenge_avoid_volumes.resolve(&triggers)?
        };
        let mut random = living.random.take().ok_or("challenge DMO cull Manager RNG absent")?;
        let result = (|| {
            let (mut owners, objects, locators) =
                crate::pedestrian_graph::backend::frame::FrameOwners::split(living)?;
            let mut cull = |objects: &mut DmoObjectMan, locators: &mut DynamicObjectLocatorMan, id: u32| {
                objects
                    .try_request_explicit_cull_with_lifecycle(
                        id,
                        locators,
                        &mut FrameLocatorServices,
                        |objects, locators, event| match event {
                            DmoCullEvent::BeforeCull(id) => owners.before_dmo_cull(
                                world, objects, locators, id, cvars, painter, self.vault,
                                &mut random, &actors, &avoid,
                            ),
                            DmoCullEvent::Monitor { entity, monitor } => {
                                if let Some(pedestrian) = owners.pedestrians.get_mut(monitor) {
                                    pedestrian.monitored_object_being_culled(entity, world, objects, cvars)?;
                                }
                                Ok(())
                            }
                        },
                    )
                    .map_err(|e| format!("challenge DMO cull: {e:?}"))
            };
            body(objects, locators, &mut cull)
        })();
        living.random = Some(random);
        result
    }

    /// `RunDMOOperations`' `tDMOResetInfo` on its trigger: Save first serializes the volume's
    /// locators (`SerializeLocatorsInTriggerVolume` 0x82d35ba0), returned for the DMO manager;
    /// Reset, Clear and Save cull every dynamic object intersecting the volume
    /// (`LWComponentMan<DynamicObject>::RequestCullWithinTriggerVolume(trigger, false)`
    /// 0x82d36b18: in entity map order, `LWBasicEntity::IsIntersectingTriggerVolume` 0x82e4c138 of
    /// the model's bounding volume at `GetBBoxTransform` of the target transform); then Reset
    /// resets the volume's locators (`ResetLocatorsInTriggerVolume` 0x82d3c928) and Clear and Save
    /// disable them (`SetLocatorsInTriggerVolumeEnabled(false)` 0x82d32380).
    #[expect(clippy::too_many_arguments, reason = "the managers and services one operation reads")]
    pub(crate) fn try_dmo_reset(
        &self,
        living: &mut LivingWorldGrindServices,
        world: &mut PhysicalAssemblyWorld,
        kind: u32,
        trigger: &ChallengeTrigger,
        chunks: &ChallengeDmoChunks,
        cvars: &InputCvarReferences,
        painter: &WorldPainterLayerMan,
    ) -> Result<Option<Vec<LocatorData>>, String> {
        if !matches!(kind, reset_type::RESET | reset_type::CLEAR | reset_type::SAVE) {
            return Ok(None);
        }
        let mut services = chunks.services();
        self.with_dmo_culls(living, world, cvars, painter, |objects, locators, cull| {
            let saved = if kind == reset_type::SAVE {
                Some(
                    locators
                        .serialize_locators_in_trigger_volume(|t| trigger.holds_locator(t), &mut services)
                        .map_err(|e| format!("challenge DMO save: {e:?}"))?,
                )
            } else {
                None
            };
            for id in objects.ids() {
                let Some(object) = objects.object(id) else {
                    continue;
                };
                let transform = object.model.bbox_transform(&object.target_transform);
                if trigger.collides(&bbox_volume(&object.model), transform) {
                    cull(objects, locators, id)?;
                }
            }
            if kind == reset_type::RESET {
                locators
                    .reset_locators_in_trigger_volume(
                        |t| trigger.holds_locator(t),
                        |locators, user| cull(objects, locators, user),
                        &mut services,
                    )
                    .map_err(|e| format!("challenge DMO reset: {e:?}"))?;
            } else {
                locators
                    .set_locators_in_trigger_volume_enabled(false, |t| trigger.holds_locator(t), &mut services)
                    .map_err(|e| format!("challenge DMO clear: {e:?}"))?;
            }
            Ok(saved)
        })
    }

    /// `ResetChallengeDMOs` (0x825b06a0): `ResetStreamedLocatorData` (0x82d2de58) of each
    /// challenge DMO group, each locator's object culled and the locator back at its initial
    /// stream transform.
    pub(crate) fn try_reset_challenge_dmos(
        &self,
        living: &mut LivingWorldGrindServices,
        world: &mut PhysicalAssemblyWorld,
        chunks: &ChallengeDmoChunks,
        cvars: &InputCvarReferences,
        painter: &WorldPainterLayerMan,
    ) -> Result<(), String> {
        let mut services = chunks.services();
        self.with_dmo_culls(living, world, cvars, painter, |objects, locators, cull| {
            for &group in &chunks.groups {
                locators
                    .reset_streamed_locator_data(group, |locators, user| cull(objects, locators, user), &mut services)
                    .map_err(|e| format!("challenge DMO group {group:016x} reset: {e:?}"))?;
            }
            Ok(())
        })
    }
}

/// `UnwindDMOOperations`' undo on its trigger: a Clear enables the volume's locators again
/// (`SetLocatorsInTriggerVolumeEnabled(true)`), a Save restores its saved locators
/// (`RestoreLocatorsInSerializedList` 0x82d3b650).
pub(crate) fn try_unwind_dmo_reset(
    living: &mut LivingWorldGrindServices,
    kind: u32,
    trigger: &ChallengeTrigger,
    saved: Option<Vec<LocatorData>>,
    chunks: &ChallengeDmoChunks,
) -> Result<(), String> {
    let locators = living.dmo_locators.as_deref_mut().ok_or("challenge DMO locators absent")?;
    let mut services = chunks.services();
    if kind == reset_type::CLEAR {
        locators
            .set_locators_in_trigger_volume_enabled(true, |t| trigger.holds_locator(t), &mut services)
            .map_err(|e| format!("challenge DMO unwind: {e:?}"))?;
    } else if let Some(saved) = saved {
        locators
            .restore_saved_locators(&saved, &mut services)
            .map_err(|e| format!("challenge DMO restore: {e:?}"))?;
    }
    Ok(())
}

/// `RunDMOOperations`' `AddStreamedLocatorData(group, 0, false, false, treatAsStatic)`
/// (0x82d3d548) of each challenge DMO group, whose locators the census then spawns from.
pub(crate) fn try_add_challenge_dmos(
    living: &mut LivingWorldGrindServices,
    chunks: &ChallengeDmoChunks,
    treat_as_static: bool,
) -> Result<(), String> {
    let locators = living.dmo_locators.as_deref_mut().ok_or("challenge DMO locators absent")?;
    let mut services = chunks.services();
    for &group in &chunks.groups {
        locators
            .add_streamed_locator_data(group, 0, false, false, treat_as_static, &mut services)
            .map_err(|e| format!("challenge DMO group {group:016x}: {e:?}"))?;
    }
    Ok(())
}
