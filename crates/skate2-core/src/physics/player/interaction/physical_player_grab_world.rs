//! WorldInitialize827b125c..12a4 actual grab-query manager construction/publication.
use super::SimulationAssemblyConstraintSession;
use crate::physical_player_grab_spline::manager::GrabQueryManager;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct GrabQueryManagerSlot {
    attempted: bool,
    value: Option<Arc<GrabQueryManager>>,
}
#[derive(Debug)]
pub(crate) struct GrabManagerAlreadyAttempted;
impl<M, C> SimulationAssemblyConstraintSession<M, C> {
    /// Source allocation can return NULL; successful construction publishes its
    /// actual empty queues. Player construction never creates a private manager.
    pub(crate) fn try_initialize_grab_query_manager(
        &mut self,
        allocated: bool,
    ) -> Result<(), GrabManagerAlreadyAttempted> {
        if self.grab_query_manager.attempted {
            return Err(GrabManagerAlreadyAttempted);
        }
        self.grab_query_manager.attempted = true;
        if allocated {
            self.grab_query_manager.value = Some(Arc::new(GrabQueryManager::new(
                self.contacts.assembly_pool_authority(),
            )));
        }
        Ok(())
    }
    pub(crate) fn grab_query_manager(&self) -> Option<&Arc<GrabQueryManager>> {
        self.grab_query_manager.value.as_ref()
    }
}

impl<C> SimulationAssemblyConstraintSession<skate2_formats::collision::AssemblyPartMaterial, C> {
    /// World::Update1 827ae138, after all Update3 state jobs and before the
    /// island step. Uses existing entities and actual attached Part 0 bodies.
    pub(crate) fn try_update_grab_messages(
        &mut self,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grab_spline::manager::GrabMessageKind;
        use crate::physical_player_grind_manager::view::GrindError;
        let Some(manager) = self.grab_query_manager().cloned() else {
            return Ok(());
        };
        manager.update_deferred_messages(|message| {
            let kind = message.descriptor.object_type;
            let spline_id = message.descriptor.object_id as u32;
            match message.kind {
                GrabMessageKind::ApplyForce {
                    force,
                    position,
                    torque,
                } => {
                    // Manager827ac2b0/827ac370 only handle DMO kind 2.
                    if kind == 2 {
                        self.try_apply_grab_message_force(spline_id, force, Some(position))?;
                        self.try_apply_grab_message_force(spline_id, torque, None)?;
                    }
                }
                GrabMessageKind::NotifyLocalPlayer => match kind {
                    1 => {
                        // NotifyLocal827ac428 -> IVehicle+12882d1d1c8.
                        // Resolve ownership anew for each deferred message.
                        let owner = self.grind_services.living_world();
                        let mut living_world = owner.lock().map_err(|_| {
                            GrindError("LivingWorld vehicle notification lock poisoned".into())
                        })?;
                        let entity = living_world.vehicle_grabbables.owner(spline_id);
                        let vehicles = living_world
                            .grind_vehicle_entities
                            .as_deref_mut()
                            .ok_or_else(|| {
                                GrindError("vehicle notification has no entity manager".into())
                            })?;
                        if let Some(vehicle) = vehicles.get_mut(entity) {
                            vehicle.motion.local_player_grabbing = true;
                        }
                    }
                    2 => {
                        let owner = self.grind_services.living_world();
                        let mut living_world = owner.lock().map_err(|_| {
                            GrindError("LivingWorld grab message lock poisoned".into())
                        })?;
                        let objects = living_world.dmo_objects.as_deref_mut().ok_or_else(|| {
                            GrindError("DMO grab message has no entity manager".into())
                        })?;
                        let entity = objects.grab_spline_owner(spline_id);
                        if let Some(object) = objects.object_mut(entity) {
                            object.set_in_use_by_player(true);
                        }
                    }
                    _ => {}
                },
                GrabMessageKind::NotifyGrabbing => {
                    // 827ac568 only notifies vehicles, not DMOs.
                    if kind == 1 {
                        // NotifyGrabbing827ac568 -> IVehicle+12482d1d1b0.
                        let owner = self.grind_services.living_world();
                        let mut living_world = owner.lock().map_err(|_| {
                            GrindError("LivingWorld vehicle notification lock poisoned".into())
                        })?;
                        let entity = living_world.vehicle_grabbables.owner(spline_id);
                        let vehicles = living_world
                            .grind_vehicle_entities
                            .as_deref_mut()
                            .ok_or_else(|| {
                                GrindError("vehicle notification has no entity manager".into())
                            })?;
                        if let Some(vehicle) = vehicles.get_mut(entity) {
                            vehicle.motion.being_skitched = true;
                        }
                    }
                }
                GrabMessageKind::Activate | GrabMessageKind::Freeze => {}
            }
            Ok(())
        })
    }

    /// DynamicObject::ApplyLocalForce82d2a4f8 / ApplyTorque82d2a588(clamp=true).
    /// Re-resolve each native call; a removed owner or detached body is a no-op.
    fn try_apply_grab_message_force(
        &mut self,
        spline_id: u32,
        force: crate::skeleton_root_transforms::SkeletonRootVector,
        position: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::{GrindError, error};
        let (body, model) = {
            let owner = self.grind_services.living_world();
            let living_world = owner
                .lock()
                .map_err(|_| GrindError("LivingWorld grab force lock poisoned".into()))?;
            let objects = living_world
                .dmo_objects
                .as_deref()
                .ok_or_else(|| GrindError("DMO grab force has no entity manager".into()))?;
            let Some(object) = objects.object(objects.grab_spline_owner(spline_id)) else {
                trace_grab_force(format_args!("spline {spline_id}: no owner object"));
                return Ok(());
            };
            (object.body.clone(), Arc::clone(&object.model))
        };
        // Pool membership represents the source Body's island attachment.
        let pool = self
            .dmos
            .as_ref()
            .ok_or_else(|| GrindError("DMO grab force has no dynamic pool".into()))?;
        if pool.candidate(&body).is_none() {
            trace_grab_force(format_args!("spline {spline_id}: body not in the dynamic pool"));
            return Ok(());
        }
        let Some(body) = self
            .try_living_world_grab_part(&body)
            .map_err(error)?
            .and_then(|part| part.body)
        else {
            trace_grab_force(format_args!("spline {spline_id}: no part body"));
            return Ok(());
        };
        trace_grab_force(format_args!("spline {spline_id}: applied {force:?} at {position:?}"));
        let limit = model
            .try_max_angular_acceleration_while_moving()
            .map_err(error)?;
        if let Some(position) = position {
            self.backend
                .rigid_bodies
                .try_apply_grab_local_force(body, force, position, limit)
                .map_err(error)
        } else {
            self.backend
                .rigid_bodies
                .try_apply_grab_world_torque(body, force, limit)
                .map_err(error)
        }
    }

    /// World::Update1 827ae008 calls GrabSplineQueryManager::Update before
    /// Update0PhysicalGatherPrimitives. Uses this World's LivingWorld and DMO
    /// pool, with no independent query scene or substitute rigid body.
    pub(crate) fn try_update_grab_queries(
        &mut self,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        num: &mut crate::physical_player_grind_manager::math::GrindNum<'_>,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::{GrindError, error};
        let Some(manager) = self.grab_query_manager().cloned() else {
            return Ok(());
        };
        manager.update_deferred_queries(
            |request, output, num| {
                use crate::physical_player_grab_spline::manager::DeferredGrabQuery;
                if request.query_type() & 2 != 0 {
                    let DeferredGrabQuery::Radius {
                        position, radius, ..
                    } = request
                    else {
                        return Err(GrindError(
                            "vehicle box grab-query provider is not connected".into(),
                        ));
                    };
                    self.grind_services
                        .living_world()
                        .lock()
                        .map_err(|_| {
                            GrindError("LivingWorld vehicle grab query lock poisoned".into())
                        })?
                        .try_vehicle_grab_radius(
                            *position,
                            *radius,
                            output,
                            cvars,
                            &mut |body| self.try_vehicle_grab_part(body).map_err(error),
                            num,
                        )?;
                }
                if request.query_type() & 4 != 0 {
                    let DeferredGrabQuery::Box(request) = request else {
                        return Err(GrindError(
                            "DMO radius grab-query provider is not connected".into(),
                        ));
                    };
                    let owner = self.grind_services.living_world();
                    let living_world = owner
                        .lock()
                        .map_err(|_| GrindError("LivingWorld grab query lock poisoned".into()))?;
                    let objects = living_world
                        .dmo_objects
                        .as_deref()
                        .ok_or_else(|| GrindError("DMO grab query has no entity manager".into()))?;
                    objects.query_grab_splines(
                        &request.geometry,
                        output,
                        cvars,
                        &mut |body| self.try_living_world_grab_part(body).map_err(error),
                        num,
                    )?;
                }
                Ok(())
            },
            num,
        )?;
        // Manager::Update827b24b0 refreshes data after deferred box queries.
        manager.update_deferred_data(
            |output, num| {
                match output.descriptor_type {
                    1 => {
                        let owner = self.grind_services.living_world();
                        let living_world = owner.lock().map_err(|_| {
                            GrindError("LivingWorld vehicle grab data lock poisoned".into())
                        })?;
                        let _ = living_world.get_vehicle_grab_spline(
                            output.descriptor_object as u32,
                            output,
                            &mut |body| self.try_vehicle_grab_part(body).map_err(error),
                            num,
                        )?;
                    }
                    2 => {
                        let owner = self.grind_services.living_world();
                        let living_world = owner.lock().map_err(|_| {
                            GrindError("LivingWorld grab data lock poisoned".into())
                        })?;
                        let objects = living_world.dmo_objects.as_deref().ok_or_else(|| {
                            GrindError("DMO grab data has no entity manager".into())
                        })?;
                        let _ = objects.get_grab_spline(
                            output.descriptor_object as u32,
                            output,
                            &mut |body| self.try_living_world_grab_part(body).map_err(error),
                            num,
                        )?;
                    }
                    // GetGrabSpline827abff0 returns false for all other kinds.
                    _ => {}
                }
                Ok(())
            },
            num,
        )?;
        let living_world = self.grind_services.living_world();
        manager.update_deferred_interactable(self.world_line_queries.as_mut(), |body| {
            let living_world = living_world.lock().map_err(error)?;
            let objects = living_world
                .dmo_objects
                .as_deref()
                .ok_or_else(|| GrindError("interactable query has no DMO entity manager".into()))?;
            let id = objects.owner_of(body);
            Ok(objects
                .object(id)
                .filter(|object| !object.is_static())
                .map(|_| id))
        })
    }
}

/// SKATE2_TRACE_PUSHING_MOVE: each grab force message's outcome.
fn trace_grab_force(message: std::fmt::Arguments<'_>) {
    if std::env::var_os("SKATE2_TRACE_PUSHING_MOVE").is_some() {
        eprintln!("grab force: {message}");
    }
}
