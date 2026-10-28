//! Reached local Actor breadcrumb selector over the original mounted owners.
//! GetWipeoutSpotBreadCrumb 0x82ce1cb8, PopBreadCrumb 0x82ce2d80,
//! TestForLWRespawnContraints 0x82ce1320, GetLastResortBreadCrumb 0x82ce2598.

use super::{
    ActorBreadCrumbConstructorOwners, ActorBreadCrumbManager, ActorBreadCrumbMountError,
    ActorBreadCrumbOwnerError, BreadCrumbFloatAttribute, BreadCrumbSelectionSource,
    SkaterBreadCrumb,
    collision_navmesh::{BreadCrumbCollisionGateError, BreadCrumbNavMeshGateError},
};
use crate::{
    actor_construction::ActorConstruction,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningHostNumericError,
    simulation_assembly_backend::constraint_session::{
        living_world_respawn::LivingWorldRespawnQueryError,
        world_services::PhysicalWorldServicesError,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum LiveBreadCrumbSelectionError {
    Cvar(InputCvarError),
    Attribute(AttributeRuntimeError),
    Owner(ActorBreadCrumbOwnerError),
    Collision(BreadCrumbCollisionGateError<ReckoningHostNumericError>),
    NavMesh(BreadCrumbNavMeshGateError),
    LivingWorld(LivingWorldRespawnQueryError),
    FootForward(ActorBreadCrumbMountError),
    MissingWorldServices,
    Network(PhysicalWorldServicesError),
    MissingRingRadii,
}

/// The Actor constructor's weak pointers and attribute binding are copied as
/// references to their existing owners; no World, MeshStore, LW Manager, or
/// skater breadcrumb collection is allocated here.
pub(crate) struct LiveBreadCrumbSelectionSource<'a> {
    actor: &'a ActorConstruction,
    simulation: &'a Arc<Mutex<PhysicalAssemblyWorld>>,
    world: &'a mut PhysicalAssemblyWorld,
    ground: &'a StaticGroundPool,
    cvars: &'a InputCvarReferences,
    numeric: &'a mut HostCollisionLineNumeric,
    owners: ActorBreadCrumbConstructorOwners,
    attributes: AttributeRuntimeInstance,
    ring_radii: Option<(f32, f32)>, // pedestrian, then vehicle
}

impl<'a> LiveBreadCrumbSelectionSource<'a> {
    pub(crate) fn new(
        breadcrumbs: &ActorBreadCrumbManager,
        actor: &'a ActorConstruction,
        simulation: &'a Arc<Mutex<PhysicalAssemblyWorld>>,
        world: &'a mut PhysicalAssemblyWorld,
        ground: &'a StaticGroundPool,
        cvars: &'a InputCvarReferences,
        numeric: &'a mut HostCollisionLineNumeric,
    ) -> Self {
        Self {
            actor,
            simulation,
            world,
            ground,
            cvars,
            numeric,
            owners: breadcrumbs.constructor_owners.clone(),
            attributes: breadcrumbs.attributes.clone(),
            ring_radii: None,
        }
    }

    /// Native LW predicate tests network first: a network game accepts every
    /// crumb (0x82ce135c -> 0x82ce14ac `li r3, 1`). Offline, vehicle sphere,
    /// pedestrian sphere, then DMO capsule are delegated to the one retained
    /// World/LW bridge. The caller supplies native radii.
    fn try_living_world_accepts_transform(
        &mut self,
        transform: &SkeletonRootMatrix,
        pedestrian_radius: f32,
        vehicle_radius: f32,
    ) -> Result<bool, LiveBreadCrumbSelectionError> {
        use LiveBreadCrumbSelectionError as E;
        let services = self
            .world
            .physical_world_services()
            .ok_or(E::MissingWorldServices)?;
        if services.try_is_network_game().map_err(E::Network)? {
            return Ok(true);
        }
        let living = self
            .owners
            .try_verify_living_world_in_world(self.simulation, self.world)
            .map_err(E::Owner)?;
        let position = transform.columns()[3];
        let attributes = &self.attributes;
        self.world
            .try_offline_respawn_lw_constraints(
                &living,
                self.ground,
                position,
                vehicle_radius,
                pedestrian_radius,
                self.cvars,
                || {
                    let height = BreadCrumbFloatAttribute::CapsuleHeightForCollisionTest
                        .try_read(attributes)?;
                    let radius = BreadCrumbFloatAttribute::CapsuleRadiusForCollisionTest
                        .try_read(attributes)?;
                    let start = position.zip(
                        SkeletonRootVector::known([0.0, radius, 0.0, 0.0]),
                        |a, b| a + b,
                    );
                    // VMX 0x82ce1478/147c: VA63 of the second add is the
                    // first result, not the original translation vector.
                    let end = start.zip(
                        SkeletonRootVector::known([0.0, height, 0.0, 0.0]),
                        |a, b| a + b,
                    );
                    Ok(CollisionLineInput {
                        start,
                        end,
                        fatness: radius,
                    })
                },
            )
            .map_err(E::LivingWorld)
    }
}

impl BreadCrumbSelectionSource for LiveBreadCrumbSelectionSource<'_> {
    type Error = LiveBreadCrumbSelectionError;

    fn try_wipeout_spot(
        &mut self,
        breadcrumbs: &ActorBreadCrumbManager,
    ) -> Result<Option<SkaterBreadCrumb>, Self::Error> {
        use LiveBreadCrumbSelectionError as E;
        if !self.cvars.try_wipeout_spot_breadcrumb().map_err(E::Cvar)? {
            return Ok(None);
        }
        let orientation = breadcrumbs.last_known_orientation.columns();
        let position = breadcrumbs
            .last_known_actor_position
            .zip(SkeletonRootVector::known([0.0; 4]), |a, b| a + b);
        let candidate = SkeletonRootMatrix::from_columns([
            orientation[0],
            orientation[1],
            orientation[2],
            position,
        ]);
        self.owners
            .try_verify_world(self.simulation)
            .map_err(E::Owner)?;
        if !breadcrumbs
            .try_collision_constraints(&candidate, false, self.world, self.ground, self.numeric)
            .map_err(E::Collision)?
        {
            return Ok(None);
        }
        let owners = &self.owners;
        let simulation = self.simulation;
        let world = &*self.world;
        if !breadcrumbs
            .try_navmesh_constraints(&candidate, self.cvars, world, || {
                owners
                    .try_verify_in_world(simulation, world)
                    .map(|verified| verified.mesh_store)
            })
            .map_err(E::NavMesh)?
        {
            return Ok(None);
        }
        let pedestrian_radius = breadcrumbs
            .try_float_attribute(BreadCrumbFloatAttribute::MinDistToPedestriansForRespawn)
            .map_err(E::Attribute)?;
        let vehicle_radius = breadcrumbs
            .try_float_attribute(BreadCrumbFloatAttribute::MinDistToVehicleForRespawn)
            .map_err(E::Attribute)?;
        if !self.try_living_world_accepts_transform(
            &candidate,
            pedestrian_radius,
            vehicle_radius,
        )? {
            return Ok(None);
        }
        Ok(Some(SkaterBreadCrumb {
            transform: candidate,
            foot_forward_right: self
                .actor
                .try_current_foot_forward_with_fakie()
                .map_err(E::FootForward)?,
            offboard: false,
        }))
    }

    fn try_begin_ring(&mut self, breadcrumbs: &ActorBreadCrumbManager) -> Result<(), Self::Error> {
        use LiveBreadCrumbSelectionError as E;
        let pedestrian = breadcrumbs
            .try_float_attribute(BreadCrumbFloatAttribute::MinDistToPedestriansForRespawn)
            .map_err(E::Attribute)?;
        let vehicle = breadcrumbs
            .try_float_attribute(BreadCrumbFloatAttribute::MinDistToVehicleForRespawn)
            .map_err(E::Attribute)?;
        self.ring_radii = Some((pedestrian, vehicle));
        Ok(())
    }

    fn try_living_world_accepts(
        &mut self,
        breadcrumb: &SkaterBreadCrumb,
    ) -> Result<bool, Self::Error> {
        let (pedestrian, vehicle) = self
            .ring_radii
            .ok_or(LiveBreadCrumbSelectionError::MissingRingRadii)?;
        self.try_living_world_accepts_transform(&breadcrumb.transform, pedestrian, vehicle)
    }

    /// The newest ring crumb whatever the living world says; with an empty
    /// ring, `mSpawnPosition` with the Actor's `GetFootForwardWithFakie`, on
    /// the board.
    fn try_last_resort(
        &mut self,
        breadcrumbs: &ActorBreadCrumbManager,
    ) -> Result<SkaterBreadCrumb, Self::Error> {
        if let Some(newest) = breadcrumbs.newest() {
            return Ok(newest);
        }
        Ok(SkaterBreadCrumb {
            transform: breadcrumbs.spawn_position,
            foot_forward_right: self
                .actor
                .try_current_foot_forward_with_fakie()
                .map_err(LiveBreadCrumbSelectionError::FootForward)?,
            offboard: false,
        })
    }
}
