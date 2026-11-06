//! Live vehicle phases of Manager.Update1/2/UpdatePostTransforms. These borrow
//! the same EntityMan, RAIN, execution groups and collision world as spawning.
use super::{
    OtherVehicles, Vehicle, VehicleLookupError,
    execution::VehicleExecutionMan,
    skitch_perception::{BorrowedSkitchActors, SkitchActors},
};
use crate::{
    actor_construction::ActorConstruction,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        LivingWorldGrindServices, entities::EntityMan, vehicle_motion::signals::HonkingPedestrians,
    },
    living_world_mesh_store::MeshStore,
    living_world_rain::RainMan,
    native_skater_random::NativeSkaterRandom,
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct VehicleFrameError(pub(crate) String);
impl std::fmt::Display for VehicleFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
pub(crate) fn error(e: impl std::fmt::Debug) -> VehicleFrameError {
    VehicleFrameError(format!("{e:?}"))
}
fn need<T>(value: Option<T>, name: &'static str) -> Result<T, VehicleFrameError> {
    value.ok_or_else(|| error(name))
}
struct Pedestrians<'a>(&'a mut EntityMan<40, crate::living_world_pedestrian_entity::PedestrianConstruction>);
impl HonkingPedestrians for Pedestrians<'_> {
    type Error = VehicleFrameError;
    fn set_honking_vehicle_if_present(&mut self, id: u32, vehicle: u32) -> Result<(), Self::Error> {
        if let Some(pedestrian) = self.0.get_mut(id) {
            pedestrian.set_honking_vehicle(vehicle).map_err(error)?;
        }
        Ok(()) // GetPedestrian returned null, as in UpdateHornState82d18870.
    }
}

/// OnCullRequested82d23bf0 then DetachFromManagers82d26f70. Physics bodies
/// remain owned until the subsequent census Update destructor. No active
/// plugins/contained props are admitted by the regular-vehicle factory.
pub(crate) fn request_cull(
    id: u32,
    vehicles: &mut EntityMan<24, Vehicle>,
    execution: &mut VehicleExecutionMan,
    pending: &mut Vec<Vehicle>,
    rain: &mut RainMan,
    mesh: &MeshStore,
    random: &mut NativeSkaterRandom,
    cvars: &InputCvarReferences,
) -> Result<(), VehicleFrameError> {
    if pending.iter().any(|v| v.id == id) {
        return Ok(());
    }
    let (vehicle, others) = need(vehicles.split_entity_mut(id), "cull vehicle")?;
    vehicle
        .end_state_behaviours(
            rain,
            mesh,
            &mut OtherVehicles(others),
            cvars,
            random,
            &mut HostReckoningNumeric,
        )
        .map_err(error)?;
    let length = vehicle.length();
    vehicle
        .route
        .remove_from_road_network::<VehicleLookupError, _>(
            &mut vehicle.motion,
            id,
            length,
            rain,
            mesh,
        )
        .map_err(
            |e: crate::living_world_grind_services::vehicle_motion::driving::DrivingError<
                VehicleLookupError,
                crate::reckoning_numeric::ReckoningHostNumericError,
            >| error(e),
        )?;
    if rain.light_biased_vehicle_id == Some(id) {
        rain.clear_light_biased_vehicle();
    }
    execution
        .remove(vehicle.core.current_losim, id)
        .map_err(error)?;
    pending.push(need(vehicles.remove(id), "detached vehicle")?);
    Ok(())
}

/// PC port (Skate 3 physics): the first of the Skate 3 skater's solver contacts on this car's
/// collision Body (its Part 0), as the island contact the native reads: the skater's side's
/// velocity relative to the car, the contact point and the skater's Body.
fn skate3_vehicle_contact(
    world: &PhysicalAssemblyWorld,
    handle: &crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
    contacts: &[crate::pedestrian_collision::contacts::Skate3PedestrianContact],
    car_velocity: [f32; 4],
) -> Result<
    Option<([f32; 3], [f32; 3], Option<crate::simulation_assembly_backend::constraint_session::physical_island::SimulationIslandBodyHandle>)>,
    VehicleFrameError,
> {
    if contacts.is_empty() {
        return Ok(None);
    }
    let Some(body) = world.try_vehicle_grab_part(handle).map_err(error)?.and_then(|part| part.body) else {
        return Ok(None);
    };
    Ok(contacts.iter().find(|contact| contact.body == body).map(|contact| {
        (
            std::array::from_fn(|i| contact.velocity[i] - car_velocity[i]),
            contact.point,
            Some(contact.other.clone()),
        )
    }))
}

impl LivingWorldGrindServices {
    /// World.Update3 -> ResolveContactSpyList827b3a90 ->
    /// cLivingWorldCollisionBody.GetContacts82e50e60 -> Vehicle.OnEntityCollision
    /// 82d1d390. The callback returns true, so only the FIRST contact of this
    /// body's existing contact vector is consumed, including non-skater hits.
    pub(crate) fn vehicle_resolve_contacts(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), VehicleFrameError> {
        use crate::living_world_grind_services::vehicle_motion::VehicleCharacteristics;
        use crate::simulation_assembly_backend::constraint_session::physical_island::{
            SimulationIslandBodyHandle, contact_owners::SimulationContactEntityOwner,
        };
        let vehicles = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        for id in vehicles.ids() {
            let vehicle = need(vehicles.get_mut(id), "contact vehicle")?;
            let collision = need(vehicle.collision.as_mut(), "vehicle collision")?;
            let Some(handle) = collision.attached_assembly_handle().cloned() else {
                continue;
            };
            let island = world
                .try_get_island_collisions(&SimulationIslandBodyHandle::Vehicle(handle.clone()))
                .map_err(error)?
                .first()
                .map(|c| {
                    (
                        *c.relative_velocity(),
                        *c.position(),
                        c.other_body().cloned(),
                    )
                });
            // PC port (Skate 3 physics): that skater is the Skate 3 solver's, whose contacts with
            // this car's Body the world records apart from the island's; its first is the car's.
            let island_missing = island.is_none();
            let first = match island {
                Some(contact) => Some(contact),
                None => skate3_vehicle_contact(world, &handle, &self.skate3_contacts, vehicle.core.velocity)?,
            };
            let Some((relative, position, other)) = first else {
                continue;
            };
            if vehicle.motion.parked {
                let threshold = vehicle
                    .driver
                    .scalar(0x5434_7592_1fd9_e04a)
                    .map_err(error)?;
                let speed = crate::reckoning_numeric::try_length(
                    [relative[0], relative[1], relative[2], 0.],
                    &mut HostReckoningNumeric,
                )
                .map_err(error)?;
                if speed > threshold {
                    vehicle.motion.parking_timer = 0.;
                    vehicle.motion.alarm_timer = 0.;
                    vehicle.motion.alarming = true;
                }
            }
            let Some(other) = other else { continue };
            let SimulationContactEntityOwner::Physical(owner) = world
                .island_body_owner_for_contacts(&other)
                .map_err(error)?
            else {
                continue;
            };
            // SetSkaterCollisionPartFlag is the original no-op virtual for cars.
            collision.colliding_with_skater = true;
            let query = owner.actor_query.lock().map_err(error)?;
            let player = need(query.as_ref(), "contact Actor")?
                .try_player_index()
                .map_err(error)?;
            // PPC slw uses six shift bits and returns zero for shifts >=32.
            vehicle.skater_collision_flags |= 1u32.checked_shl(player & 63).unwrap_or(0);
            let delta = std::array::from_fn::<_, 4, _>(|i| {
                if i < 3 {
                    position[i] - vehicle.core.transform[3][i]
                } else {
                    0.
                }
            });
            // Entity virtual +10 is GetVelocity, NOT GetFacing.
            if HostReckoningNumeric
                .dot3(delta, vehicle.core.velocity)
                .map_err(error)?
                > 0.
            {
                vehicle.motion.force_stopping = true;
            }
            // SKATE2_TRACE_VEHICLE_SKATER: each car the skater touches, and whether it stops.
            if std::env::var_os("SKATE2_TRACE_VEHICLE_SKATER").is_some() {
                eprintln!("vehicle {id} touches skater: skate3={} delta={delta:?} force_stopping={}",
                    island_missing, vehicle.motion.force_stopping);
            }
        }
        Ok(())
    }
    /// UpdateOnRoadPoints8271b4e8's skater half. One local Actor in this
    /// selected session; every point is tested against the actual MeshMan.
    pub(crate) fn vehicle_skater_road_points(
        &mut self,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        network_game: bool,
    ) -> Result<(), VehicleFrameError> {
        use crate::living_world_rain::RainPoint;
        use crate::reckoning_numeric::ReckoningVector;
        if network_game {
            return Ok(());
        }
        let mesh = self.try_mesh_store().map_err(error)?;
        let rain = need(self.rain.as_mut(), "RAIN")?;
        rain.skater_points.clear();
        let physical = actor.cached_physical_allocation().map_err(error)?;
        let output = physical.owner().phys_out().lock().map_err(error)?;
        if !output
            .try_vehicle_avoidance(cvars.try_vehicle_avoidance_disabled().map_err(error)?)
            .map_err(error)?
        {
            return Ok(());
        }
        let wiping = output.try_state_output().map_err(error)?.is_wiping_out;
        let mut add = |position:crate::skeleton_root_transforms::SkeletonRootVector|->Result<(),VehicleFrameError> {
            if rain.try_point_on_road_network(&mesh,position).map_err(error)? {
                rain.skater_points.push(RainPoint {position:position.operand(),radius:[0.;4],compulsory_stop:wiping,pedestrian_id:-1});
            }
            Ok(())
        };
        add(output
            .try_camera_board_output()
            .map_err(error)?
            .deck_position)?;
        if wiping {
            let s = output.try_skeleton_output().map_err(error)?;
            for p in [
                s.skeleton_hips_pos,
                s.skeleton_head_pos,
                s.skeleton_foot_left_pos,
                s.skeleton_foot_right_pos,
            ] {
                add(p)?;
            }
        }
        Ok(())
    }

    pub(crate) fn vehicle_reset_skitch_flags(&mut self) -> Result<(), VehicleFrameError> {
        let entities = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        for id in entities.ids() {
            let v = need(entities.get_mut(id), "vehicle")?;
            v.being_skitched_previous_frame = v.motion.being_skitched;
            v.motion.being_skitched = false;
            v.local_player_grabbing_previous_frame = v.motion.local_player_grabbing;
            v.motion.local_player_grabbing = false;
        }
        Ok(())
    }
    pub(crate) fn vehicle_entity_road_points(
        &mut self,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        network_game: bool,
    ) -> Result<(), VehicleFrameError> {
        if network_game {
            return Ok(());
        }
        let rain = need(self.rain.as_mut(), "RAIN")?;
        rain.entity_points.clear();
        // Manager.AddAllEntitiesRAINPoints826f5218 visits pedestrians first,
        // in EntityMan hash order. Vehicle/prop virtuals are no-ops, then DMOs.
        let pedestrians=need(self.pedestrian_entities.as_deref(), "pedestrians")?;
        for id in pedestrians.ids(){
            let pedestrian=need(pedestrians.get(id), "RAIN pedestrian")?;
            if crate::pedestrian_graph::conditions::entity_condition("IsOnRoad",pedestrian)
                .map_err(error)?!=Some(true){continue;}
            let collision=need(pedestrian.collision.as_ref(), "RAIN pedestrian collision")?;
            // GetLength82e4fec0 reads the model's Z dimension (zero without a model).
            let radius=pedestrian.model.as_ref().map_or(0.,|model|model.dimensions[2])*0.5;
            let core=need(pedestrian.core.as_ref(), "RAIN pedestrian core")?;
            // Pedestrian82e91c88 uses Colliding; BigBlack82e92508 forces a stop.
            rain.entity_points.push(crate::living_world_rain::RainPoint{
                position:core.target_transform[3],radius:[radius;4],
                compulsory_stop:pedestrian.big_black.is_some() || collision.colliding,
                pedestrian_id:id as i32,
            });
        }
        need(self.dmo_objects.as_deref(), "DMOs")?.add_road_points(rain, vault)
    }
    pub(crate) fn request_vehicle_cull(
        &mut self,
        id: u32,
        cvars: &InputCvarReferences,
    ) -> Result<(), VehicleFrameError> {
        let mesh = self.try_mesh_store().map_err(error)?;
        request_cull(
            id,
            need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?,
            &mut self.vehicle_execution,
            &mut self.vehicle_culls,
            need(self.rain.as_mut(), "RAIN")?,
            &mesh,
            need(self.random.as_mut(), "Manager RNG")?,
            cvars,
        )
    }

    pub(crate) fn process_vehicle_culls(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), VehicleFrameError> {
        let mesh = self.try_mesh_store().map_err(error)?;
        let pedestrians = need(self.pedestrian_entities.as_deref_mut(), "pedestrians")?;
        let rain = need(self.rain.as_mut(), "RAIN")?;
        let factory = need(self.vehicle_factory.as_mut(), "vehicle factory")?;
        while let Some(vehicle) = self.vehicle_culls.first_mut() {
            // Vehicle destructor82d25138 repeats road removal, then collision,
            // core, passage, splines, BasicEntity and finally factory slot.
            vehicle
                .destroy_detached::<crate::reckoning_numeric::ReckoningHostNumericError>(
                    pedestrians,
                    rain,
                    &mesh,
                    factory,
                    &mut self.vehicle_collideables,
                    &mut self.vehicle_grabbables,
                    world,
                )
                .map_err(error)?;
            self.vehicle_culls.remove(0);
        }
        Ok(())
    }

    /// UpdateLOSStates82d21cf0, separated from execution so the Manager can
    /// preserve the native cross-manager phase ordering82715a10.
    pub(crate) fn vehicle_update_los_states(
        &mut self, center: [f32; 4],
    ) -> Result<(), VehicleFrameError> {
        let entities = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        let mut numeric = HostReckoningNumeric;
        // UpdateLOSStates82d21cf0; collision OnLOSimSwitch82e50580 returns true.
        for id in entities.ids() {
            let vehicle = need(entities.get_mut(id), "vehicle")?;
            let level = vehicle.core.test_losim(center, &vehicle.attrib_cache, &mut numeric)
                .map_err(error)?;
            let previous = vehicle.core.current_losim;
            if level != previous {
                self.vehicle_execution.remove(previous, id).map_err(error)?;
                vehicle.core.current_losim = level;
                self.vehicle_execution.add(level, id).map_err(error)?;
            }
        }
        Ok(())
    }

    /// Vehicle portion of UpdateEntitiesPreFrame. Execution groups have
    /// already advanced once at the Manager-level phase boundary.
    pub(crate) fn vehicle_update_preframe(
        &mut self, center: [f32; 4], actor: &ActorConstruction,
        cvars: &InputCvarReferences,
    ) -> Result<(), VehicleFrameError> {
        let mesh = self.try_mesh_store().map_err(error)?;
        let entities = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        let mut numeric = HostReckoningNumeric;
        let mut targets = Vec::new();
        self.vehicle_execution
            .collect_preframe(entities, center, &mut targets, &mut numeric)
            .map_err(error)?;
        let actors = [actor];
        let mut actors = BorrowedSkitchActors { actors: &actors };
        let root_position = actors.position(0).map_err(error)?;
        VehicleExecutionMan::update_positional_data(
            entities,
            &targets,
            Some(root_position),
            &mut numeric,
        )
        .map_err(error)?;
        let rain = need(self.rain.as_mut(), "RAIN")?;
        let random = need(self.random.as_mut(), "Manager RNG")?;
        let mut pedestrians =
            Pedestrians(need(self.pedestrian_entities.as_deref_mut(), "pedestrians")?);
        for target in targets {
            let (vehicle, others) = need(
                entities.split_entity_mut(target.entity),
                "execution vehicle",
            )?;
            let mut others = OtherVehicles(others);
            // BasicEntity.PreStateGraphUpdate only services a requested plugin
            // removal. Regular vehicles have none; PostStateGraphUpdate is
            // exclusively the optional debug-text history82e62858.
            vehicle
                .pre_state_graph_vehicle_suffix(
                    target.dt,
                    rain,
                    &mut others,
                    &mut actors,
                    &mut pedestrians,
                    cvars,
                    &mut numeric,
                )
                .map_err(error)?;
            vehicle
                .update_state(
                    target.dt,
                    rain,
                    &mesh,
                    &mut others,
                    cvars,
                    random,
                    &mut numeric,
                )
                .map_err(error)?;
        }
        Ok(())
    }

    /// UpdateTargetRAINInfo and UpdateTargetTransforms follow all entity
    /// preframe tasks, not just the vehicle tasks82715ad0/82715ae8.
    pub(crate) fn vehicle_update_targets(
        &mut self, cvars: &InputCvarReferences, dt: f32,
    ) -> Result<(), VehicleFrameError> {
        let mesh = self.try_mesh_store().map_err(error)?;
        let entities = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        let rain = need(self.rain.as_mut(), "RAIN")?;
        let random = need(self.random.as_mut(), "Manager RNG")?;
        let mut numeric = HostReckoningNumeric;
        // Source map order, every frame; not the behaviour group's timestep.
        for id in entities.ids() {
            let mut cull = false;
            {
                let (vehicle, others) = need(entities.split_entity_mut(id), "route vehicle")?;
                vehicle
                    .update_target_rain_info(
                        dt,
                        rain,
                        &mesh,
                        &mut OtherVehicles(others),
                        &mut |_| {
                            cull = true;
                            Ok::<_, VehicleLookupError>(())
                        },
                        &mut numeric,
                    )
                    .map_err(error)?;
            }
            if cull {
                request_cull(
                    id,
                    entities,
                    &mut self.vehicle_execution,
                    &mut self.vehicle_culls,
                    rain,
                    &mesh,
                    random,
                    cvars,
                )?;
            }
        }
        for id in entities.ids() {
            let mut cull = false;
            need(entities.get_mut(id), "target vehicle")?
                .update_target_transform(
                    rain,
                    &mesh,
                    &mut |_| {
                        cull = true;
                        Ok::<_, VehicleLookupError>(())
                    },
                    &mut numeric,
                )
                .map_err(error)?;
            if cull {
                request_cull(
                    id,
                    entities,
                    &mut self.vehicle_execution,
                    &mut self.vehicle_culls,
                    rain,
                    &mesh,
                    random,
                    cvars,
                )?;
            }
        }
        Ok(())
    }

    /// ResetCollisionFlags82e4c6c8: vehicle's part-report virtuals at
    /// 823269a0+0c/+18 are blr825187c0; only CollidingWithSkater is reset.
    pub(crate) fn vehicle_reset_collision_flags(&mut self) -> Result<(), VehicleFrameError> {
        let entities = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        for id in entities.ids() {
            if let Some(collision) = need(entities.get_mut(id), "vehicle")?.collision.as_mut() {
                collision.colliding_with_skater = false;
            }
        }
        Ok(())
    }
    pub(crate) fn vehicle_update_collisions(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        dt: f32,
    ) -> Result<(), VehicleFrameError> {
        let entities = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        // Native Pre/PostCollisionUpdate vehicle virtuals are blr. Its actual
        // cLivingWorldCollision.UpdateCollisions82e4c808 calls the shared drive.
        for id in entities.ids() {
            let vehicle = need(entities.get(id), "collision vehicle")?;
            need(vehicle.collision.as_ref(), "vehicle collision")?
                .update_collision_transforms(&vehicle.core, world, dt, cvars)
                .map_err(error)?;
        }
        Ok(())
    }
    pub(crate) fn vehicle_post_transforms(
        &mut self,world:&mut PhysicalAssemblyWorld,cvars:&InputCvarReferences,dt:f32,
    )->Result<(),VehicleFrameError>{
        let entities=need(self.grind_vehicle_entities.as_deref_mut(),"vehicles")?;
        // ProcessLineTestBatch82d1add0, ReceiveGroundTests82d17718. All three
        // nonzero-parameter hits are required; partial hits change nothing.
        for (id, hits) in self.vehicle_ground_results.drain(..) {
            let [Some(a), Some(b), Some(c)] = hits else {
                continue;
            };
            let Some(vehicle) = entities.get_mut(id) else {
                continue;
            };
            let mut n = HostReckoningNumeric;
            let z = crate::reckoning_numeric::try_normalize_unsafe(
                std::array::from_fn::<_, 4, _>(|i| b[i] - c[i]),
                &mut n,
            )
            .map_err(error)?;
            let edge = std::array::from_fn::<_, 4, _>(|i| b[i] - a[i]);
            let y = crate::reckoning_numeric::try_normalize_unsafe(
                crate::reckoning_numeric::cross_full4(edge, z),
                &mut n,
            )
            .map_err(error)?;
            let x = crate::reckoning_numeric::try_normalize_unsafe(
                crate::reckoning_numeric::cross_full4(y, z),
                &mut n,
            )
            .map_err(error)?;
            vehicle.core.target_transform = [x, y, z, std::array::from_fn(|i| (a[i] + c[i]) * 0.5)];
        }
        let rain = need(self.rain.as_mut(), "RAIN")?;
        for id in entities.ids() {
            use crate::living_world_grind_services::vehicle_motion::VehicleCharacteristics;
            let v = need(entities.get_mut(id), "skitch vehicle")?;
            if v.being_skitched_previous_frame && !v.motion.being_skitched {
                v.motion.skitch_helper_timer = v
                    .characteristics
                    .scalar(0x4727_cf78_5ef7_35c8)
                    .map_err(error)?;
            } else {
                v.motion.skitch_helper_timer -= dt;
                if v.motion.skitch_helper_timer < 0. {
                    v.motion.skitch_helper_timer = -1.;
                }
            }
            if v.motion.local_player_grabbing && !v.local_player_grabbing_previous_frame {
                rain.light_biased_vehicle_id = Some(id);
                rain.vehicle_light_bias_registered = true;
            } else if !v.motion.local_player_grabbing && v.local_player_grabbing_previous_frame {
                rain.clear_light_biased_vehicle();
            }
        }
        Ok(())
    }
    /// Manager.UpdateTransforms826f52a8 calls this after pedestrian transforms
    /// and before DMO transforms, after the earlier line/skitch phase.
    pub(crate) fn vehicle_update_transforms(&mut self,dt:f32)->Result<(),VehicleFrameError>{
        let entities=need(self.grind_vehicle_entities.as_deref_mut(),"vehicles")?;
        for id in entities.ids() {
            let vehicle = need(entities.get_mut(id), "transform vehicle")?;
            vehicle
                .core
                .update_transform(dt, &mut HostReckoningNumeric)
                .map_err(error)?;
        }
        Ok(())
    }
    pub(crate) fn vehicle_update_wheels(&mut self, dt: f32) -> Result<(), VehicleFrameError> {
        let entities = need(self.grind_vehicle_entities.as_deref_mut(), "vehicles")?;
        for id in entities.ids() {
            let vehicle = need(entities.get_mut(id), "wheel vehicle")?;
            vehicle
                .wheels
                .update(
                    dt,
                    vehicle.core.velocity,
                    vehicle.core.target_transform,
                    vehicle.motion.skidding,
                    vehicle.motion.backing_up,
                    &mut HostReckoningNumeric,
                )
                .map_err(error)?;
        }
        Ok(())
    }

    /// AddGroundTestsToBatch82d1d938: HiFi only, three corners, vertical +/-2m.
    /// Same World BF_Ground kernel, serialized host dispatch; results are
    /// retained until native ProcessLineTestBatch's post-physics position.
    pub(crate) fn vehicle_ground_tests(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &crate::static_ground_pool::StaticGroundPool,
    ) -> Result<(), VehicleFrameError> {
        let _profile = crate::diagnostics::frame_profile::span(13);
        use crate::{
            collision_line_collection::CollisionLineInput,
            collision_line_numeric::HostCollisionLineNumeric,
            physical_player_grind_manager::view::{lanes, root},
        };
        self.vehicle_ground_results.clear();
        let entities = need(self.grind_vehicle_entities.as_deref(), "vehicles")?;
        let (queries, dmos) = world.world_line_owners();
        let queries = need(queries, "world line queries")?;
        let dmos = need(dmos, "DMO query pool")?;
        for id in entities.ids() {
            let vehicle = need(entities.get(id), "ground-test vehicle")?;
            let collision = need(vehicle.collision.as_ref(), "vehicle collision")?;
            let Some(dimensions) = collision.model_dimensions() else {
                continue;
            };
            if vehicle.core.current_losim != 0 {
                continue;
            }
            let m = vehicle.core.target_transform;
            let half = dimensions.map(|v| v * 0.5);
            let forward: [f32; 4] = m[2].map(|v| (v * 0.5) * half[2]);
            let side: [f32; 4] = m[0].map(|v| v * half[0]);
            let front: [f32; 4] = std::array::from_fn(|i| m[3][i] + forward[i]);
            let rear: [f32; 4] = std::array::from_fn(|i| m[3][i] - forward[i]);
            let points: [[f32; 4]; 3] = [
                std::array::from_fn(|i| front[i] + side[i]),
                std::array::from_fn(|i| front[i] - side[i]),
                std::array::from_fn(|i| rear[i] - side[i]),
            ];
            let batch = queries
                .try_acquire_line_batch(dmos, "LWVehicleMan::Init", 1)
                .map_err(error)?;
            queries.try_begin(&batch).map_err(error)?;
            for point in points {
                let mut start = point;
                start[1] += 2.;
                let mut end = point;
                end[1] -= 2.;
                queries
                    .try_add_line(
                        &batch,
                        CollisionLineInput {
                            start: root(start),
                            end: root(end),
                            fatness: 0.,
                        },
                    )
                    .map_err(error)?;
            }
            queries
                .try_end(&batch, ground, dmos, &mut HostCollisionLineNumeric)
                .map_err(error)?;
            queries.try_sync(&batch).map_err(error)?;
            let mut hits = [None; 3];
            for (i, hit) in hits.iter_mut().enumerate() {
                *hit = queries
                    .try_get_result(&batch, i as u32)
                    .map_err(error)?
                    .filter(|h| h.line_parameter != 0.)
                    .map(|h| lanes(h.position));
            }
            queries.try_release(&batch).map_err(error)?;
            self.vehicle_ground_results.push((id, hits));
        }
        Ok(())
    }
}
