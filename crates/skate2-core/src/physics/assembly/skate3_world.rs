//! Retained host body identities and actual authored collision geometry for
//! the captured Skate 3 solver. No proxy body is integrated by both solvers.
use super::SimulationAssemblyConstraintSession;
use crate::{
    authored_collision_publication::AuthoredCollisionPublication,
    narrow_phase::{GpPrimitive, GpPrimitiveType},
    serialised_assembly_template::SerialisedAssemblyVolumeDataState,
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    world::{CollisionAggregateRef, CollisionArena, CollisionMesh},
};
use skate2_formats::collision::VolumeType;
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{PreparedTriangles, WorldTriangle},
        contact::RetailContactMaterial,
    },
};
use crate::skate3_native::{
    SolidBody, SolidCollider,
    rapier3d::prelude::{Pose, Rotation, SharedShape, Vector},
};
use std::sync::Arc;

#[derive(Clone)]
enum MeshOwner {
    Source(Arc<crate::simulation_normal_arena_plan::SimulationNormalArenaPlan>),
    Character(Arc<CollisionArena>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPool};
    fn transform() -> [f32; 16] {
        crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM
    }
    fn inertia() -> Arc<SimulationInertia> {
        Arc::new(SimulationInertia {
            inverse_mass: 0.5,
            inverse_tensor: [0.2, 0.3, 0.4],
            ..SimulationInertia::native_default()
        })
    }
    fn reaction(id: u64) -> crate::skate3_native::BodyReaction {
        crate::skate3_native::BodyReaction {
            id,
            linear_velocity_delta: [2., 0., 3.],
            angular_velocity_delta: [0., 1., 0.],
            position_delta: [0.1, 0.2, 0.3],
            orientation_delta: [0., std::f32::consts::FRAC_PI_2, 0.],
        }
    }

    #[test]
    fn host_body_ids_and_reactions_reject_reused_slots() {
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let old = pool
            .try_add_rigid_body(transform(), Some(&inertia()), 4)
            .unwrap()
            .unwrap();
        let old_observation = pool.try_observe(old).unwrap();
        let mut ids = HostBodies::default();
        let old_id = ids.id(old_observation).unwrap();
        pool.try_remove_rigid_body(old).unwrap();
        let new = pool
            .try_add_rigid_body(transform(), Some(&inertia()), 4)
            .unwrap()
            .unwrap();
        let new_observation = pool.try_observe(new).unwrap();
        assert_ne!(old_id, ids.id(new_observation).unwrap());
        let before = pool.try_drive_build_snapshot(new_observation).unwrap();
        assert!(
            pool.try_apply_external_reaction(ids.body(old_id).unwrap(), reaction(old_id))
                .is_err()
        );
        let after = pool.try_drive_build_snapshot(new_observation).unwrap();
        assert_eq!(before.center_of_mass, after.center_of_mass);
        assert_eq!(before.velocity, after.velocity);
        assert_eq!(before.quaternion, after.quaternion);
    }
    #[test]
    fn source_reaction_wakes_and_corrects_only_its_actual_host_body() {
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let body = pool
            .try_add_rigid_body(transform(), Some(&inertia()), 2)
            .unwrap()
            .unwrap();
        let other = pool
            .try_add_rigid_body(transform(), Some(&inertia()), 4)
            .unwrap()
            .unwrap();
        let body = pool.try_observe(body).unwrap();
        let other = pool.try_observe(other).unwrap();
        let untouched = pool.try_drive_build_snapshot(other).unwrap();
        pool.try_apply_external_reaction(body, reaction(1)).unwrap();
        let corrected = pool.try_drive_build_snapshot(body).unwrap();
        assert_eq!(corrected.state & 7, 4);
        assert_eq!(corrected.center_of_mass, [0.1, 0.2, 0.3]);
        assert_eq!(corrected.velocity, [2., 0., 3.]);
        assert_eq!(corrected.omega, [0., 1., 0.]);
        assert!(corrected.right[0].abs() < 0.0001 && (corrected.right[2] + 1.).abs() < 0.0001);
        let after = pool.try_drive_build_snapshot(other).unwrap();
        assert_eq!(untouched.center_of_mass, after.center_of_mass);
        assert_eq!(untouched.velocity, after.velocity);
        assert_eq!(untouched.quaternion, after.quaternion);
    }

    #[test]
    fn zero_source_reaction_keeps_an_untouched_body_frozen() {
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let body = pool
            .try_add_rigid_body(transform(), Some(&inertia()), 2)
            .unwrap()
            .unwrap();
        let body = pool.try_observe(body).unwrap();
        let before = pool.try_drive_build_snapshot(body).unwrap();
        pool.try_apply_external_reaction(
            body,
            crate::skate3_native::BodyReaction {
                id: 1,
                linear_velocity_delta: [0.; 3],
                angular_velocity_delta: [0.; 3],
                position_delta: [0.; 3],
                orientation_delta: [0.; 3],
            },
        )
        .unwrap();
        let after = pool.try_drive_build_snapshot(body).unwrap();
        assert_eq!(after.state & 7, 2);
        assert_eq!(before.center_of_mass, after.center_of_mass);
        assert_eq!(before.quaternion, after.quaternion);
        assert_eq!(before.velocity, after.velocity);
        assert_eq!(before.omega, after.omega);
    }
}
/// Placements within a millimetre (and the matching rotation lanes) are the same:
/// a pushed or dragged prop is rebuilt, a resting one is not.
pub(crate) fn same_prop_placement(a: &[f32; 16], b: &[f32; 16]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() <= 1.0e-3)
}

/// One model-space triangle of an authored collision mesh with its native GP
/// contact features. A rigid placement keeps every flag and edge cosine.
#[derive(Clone, Copy)]
struct LocalTriangle {
    vertices: [[f32; 3]; 3],
    surface: u32,
    flags: u32,
    edge_cosines: [f32; 3],
}

const IDENTITY_TRANSFORM: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

fn place(transform: &[f32; 16], [x, y, z]: [f32; 3]) -> Vector3 {
    let lane = |row: usize| {
        transform[row] * x + transform[4 + row] * y + transform[8 + row] * z + transform[12 + row]
    };
    Vector3::new(lane(0), lane(1), lane(2))
}

fn native_triangle(
    triangle: &LocalTriangle,
    transform: &[f32; 16],
    material: RetailContactMaterial,
) -> Option<WorldTriangle> {
    WorldTriangle::from_vertices(
        triangle.vertices.map(|vertex| place(transform, vertex)),
        material,
        triangle.surface,
        triangle.flags,
        triangle.edge_cosines,
        0.,
    )
}

fn material_bits(material: RetailContactMaterial) -> [u32; 3] {
    [material.static_friction, material.dynamic_friction, material.restitution].map(f32::to_bits)
}

/// A moving body whose ground, wheel and skeleton contacts come from its
/// authored collision mesh: native world-space triangles at its CURRENT
/// placement, in native mesh order, as the static world holds its own.
pub(crate) struct Skate3MeshBody {
    pub(crate) id: u64,
    pub(crate) triangles: Arc<PreparedTriangles>,
}

/// This tick's moving bodies, and the ones among them that carry a mesh.
#[derive(Default)]
pub(crate) struct Skate3Bodies {
    pub(crate) solids: Vec<SolidBody>,
    pub(crate) meshes: Vec<Skate3MeshBody>,
}

/// A mesh body's triangles at the exact placement they were built at.
struct PlacedMesh {
    id: u64,
    transform: [u32; 16],
    material: [u32; 3],
    triangles: Arc<PreparedTriangles>,
    generation: u64,
}
impl MeshOwner {
    fn arena(&self) -> &CollisionArena {
        match self {
            Self::Source(owner) => &owner.static_plan().asset().collision,
            Self::Character(owner) => owner,
        }
    }
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Source(a), Self::Source(b)) => Arc::ptr_eq(a, b),
            (Self::Character(a), Self::Character(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
struct CachedMesh {
    owner: MeshOwner,
    index: usize,
    shape: SharedShape,
    /// Model-space native triangles, decoded on first use by a mesh body.
    local: Option<Arc<[LocalTriangle]>>,
    generation: u64,
}

/// Monotonically issued IDs retain the allocation occurrence, not only a pool
/// slot. Removing and reusing a slot can never redirect an old reaction.
#[derive(Default)]
pub(crate) struct HostBodies {
    next: u64,
    bodies: Vec<(SimulationRigidBodyObservation, u64)>,
    meshes: Vec<CachedMesh>,
    placed: Vec<PlacedMesh>,
    mesh_generation: u64,
}
impl HostBodies {
    pub(crate) fn body(&self, id: u64) -> Option<SimulationRigidBodyObservation> {
        self.bodies
            .iter()
            .find(|(_, candidate)| *candidate == id)
            .map(|(body, _)| *body)
    }
    fn id(&mut self, body: SimulationRigidBodyObservation) -> Result<u64, String> {
        if let Some((_, id)) = self.bodies.iter().find(|(candidate, _)| *candidate == body) {
            return Ok(*id);
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Skate 3 body ID exhausted")?;
        // MovingQueries uses 31 bits of the host identity in its geometry ID.
        if self.next > 0x7fff_ffff {
            return Err("Skate 3 moving-query ID exhausted".into());
        }
        self.bodies.push((body, self.next));
        Ok(self.next)
    }
    fn mesh(&mut self, owner: MeshOwner, index: usize) -> Result<SharedShape, String> {
        if let Some(entry) = self
            .meshes
            .iter_mut()
            .find(|entry| entry.index == index && entry.owner.same(&owner))
        {
            entry.generation = self.mesh_generation;
            return Ok(entry.shape.clone());
        }
        let mesh = owner
            .arena()
            .meshes
            .get(index)
            .ok_or("Missing host collision mesh")?;
        let shape = mesh_shape(mesh)?;
        self.meshes.push(CachedMesh {
            owner,
            index,
            shape: shape.clone(),
            local: None,
            generation: self.mesh_generation,
        });
        Ok(shape)
    }
    /// The mesh's model-space native triangles and GP features, decoded once
    /// per retained mesh. Each must also form a valid native triangle.
    fn local(&mut self, owner: MeshOwner, index: usize) -> Result<Arc<[LocalTriangle]>, String> {
        self.mesh(owner.clone(), index)?;
        let entry = self
            .meshes
            .iter_mut()
            .find(|entry| entry.index == index && entry.owner.same(&owner))
            .ok_or("Missing host collision mesh")?;
        if let Some(local) = &entry.local {
            return Ok(Arc::clone(local));
        }
        let mesh = owner
            .arena()
            .meshes
            .get(index)
            .ok_or("Missing host collision mesh")?;
        let (triangles, features) =
            crate::static_ground_pool::transformed_mesh_ground(mesh, &IDENTITY_TRANSFORM)
                .map_err(|error| error.to_string())?;
        if triangles.is_empty() || triangles.len() != features.len() {
            return Err("Host mesh has no complete native features".into());
        }
        let local: Arc<[LocalTriangle]> = triangles
            .iter()
            .zip(&features)
            .map(|(triangle, feature)| {
                let local = LocalTriangle {
                    vertices: triangle.vertices,
                    surface: u32::from(triangle.surface_id.unwrap_or(0)),
                    flags: feature.flags,
                    edge_cosines: feature.edge_cosines,
                };
                let valid = feature.edge_cosines.iter().all(|value| value.is_finite())
                    && native_triangle(&local, &IDENTITY_TRANSFORM, RetailContactMaterial {
                        static_friction: 0., dynamic_friction: 0., restitution: 0.,
                    }).is_some();
                valid.then_some(local).ok_or("Host mesh holds an invalid native triangle")
            })
            .collect::<Result<_, _>>()?;
        entry.local = Some(Arc::clone(&local));
        Ok(local)
    }
    /// A mesh body's world-space native triangles at `transform`. A body
    /// resting at bit-identical placement keeps the triangles it already has;
    /// a moving one is re-placed from the retained model-space decode.
    fn placed(
        &mut self,
        id: u64,
        transform: &[f32; 16],
        meshes: &[Arc<[LocalTriangle]>],
        material: RetailContactMaterial,
    ) -> Result<Arc<PreparedTriangles>, String> {
        let key = transform.map(f32::to_bits);
        let material_key = material_bits(material);
        if let Some(entry) = self
            .placed
            .iter_mut()
            .find(|entry| entry.id == id && entry.transform == key && entry.material == material_key)
        {
            entry.generation = self.mesh_generation;
            return Ok(Arc::clone(&entry.triangles));
        }
        let triangles = meshes
            .iter()
            .flat_map(|mesh| mesh.iter())
            .map(|triangle| {
                native_triangle(triangle, transform, material).ok_or("Invalid placed host mesh triangle")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let triangles = Arc::new(PreparedTriangles::new(triangles));
        self.placed.retain(|entry| entry.id != id);
        self.placed.push(PlacedMesh {
            id,
            transform: key,
            material: material_key,
            triangles: Arc::clone(&triangles),
            generation: self.mesh_generation,
        });
        Ok(triangles)
    }
}

fn mesh_shape(mesh: &CollisionMesh) -> Result<SharedShape, String> {
    if mesh.triangles.is_empty() {
        return Err("Host aggregate has no decoded collision triangles".into());
    }
    let mut vertices = Vec::with_capacity(mesh.triangles.len() * 3);
    let mut indices = Vec::with_capacity(mesh.triangles.len());
    for triangle in &mesh.triangles {
        let start = u32::try_from(vertices.len()).map_err(|e| e.to_string())?;
        vertices.extend(triangle.vertices.map(Vector::from_array));
        indices.push([start, start + 1, start + 2]);
    }
    SharedShape::trimesh(vertices, indices).map_err(|e| format!("Host collision trimesh: {e}"))
}
/// A Skate 2 collision group in Skate 3's numbering. TU3 has no camera or phantom groups and adds
/// its own (15, 16, 19, 20), so its living world groups sit two lower: a pedestrian's world and
/// skater bodies are 11 and 13 in Skate 2 (`InitCollisionBodies` 0x82e99df0) but 9 and 11 in TU3
/// (0x82e268c8), vehicles 10 and 8 (0x82d28eb8, 0x82c37458), dynamic objects 14/16 and 12/14
/// (0x82d3aa28, 0x82c4e7b8). Under this map the two `InitCullingTable` pair tables (Skate 2
/// 0x827b8c10, TU3 0x82765ef0) agree on every pair but group 17's, which TU3 moved from the board
/// to the ragdoll.
fn skate3_group(group: u32) -> u32 {
    match group {
        0..=4 | 7 | 17 | 18 => group,
        6 => 5,
        10..=16 => group - 2,
        _ => 1,
    }
}
fn pose(transform: [f32; 16]) -> Result<Pose, String> {
    if transform.iter().any(|v| !v.is_finite()) {
        return Err("Non-finite host collider transform".into());
    }
    let q = crate::world::simulation_quaternion_from_orthonormalized_matrix_host(transform);
    let rotation = Rotation::from_xyzw(q[0], q[1], q[2], q[3]).normalize();
    Ok(Pose::from_parts(
        Vector::new(transform[12], transform[13], transform[14]),
        rotation,
    ))
}
/// `round_triangles`: keep a Triangle's fatness (a riderless deck's kicktails),
/// as every other primitive does; a plain triangle is paper thin.
fn primitive_collider(
    record: &SerialisedAssemblyVolumeDataState,
    round_triangles: bool,
) -> Result<SolidCollider, String> {
    let gp = GpPrimitive::from_volume(&record.volume, Some(record.transform))
        .ok_or("Unsupported host primitive")?;
    let center = Vector::from_array(gp.points[0]);
    let (shape, rotation) = match gp.primitive_type {
        GpPrimitiveType::Sphere => (SharedShape::ball(gp.fatness), Rotation::IDENTITY),
        GpPrimitiveType::Capsule | GpPrimitiveType::Cylinder => {
            let axis = Vector::from_array(gp.edge_directions[0]);
            if !axis.is_finite() || axis.length_squared() == 0. {
                return Err("Invalid host capsule/cylinder axis".into());
            }
            let rotation = Rotation::from_rotation_arc(Vector::Y, axis.normalize());
            let shape = if gp.primitive_type == GpPrimitiveType::Capsule {
                SharedShape::capsule_y(gp.dimensions[0], gp.fatness)
            } else {
                SharedShape::round_cylinder(gp.dimensions[0], gp.dimensions[1], gp.fatness)
            };
            (shape, rotation)
        }
        GpPrimitiveType::Box => {
            let mut transform = [0.; 16];
            transform[15] = 1.;
            for c in 0..3 {
                for r in 0..3 {
                    transform[c * 4 + r] = gp.face_normals[c][r];
                }
            }
            let q = pose(transform)?.rotation;
            (
                SharedShape::round_cuboid(
                    gp.dimensions[0],
                    gp.dimensions[1],
                    gp.dimensions[2],
                    gp.fatness,
                ),
                q,
            )
        }
        GpPrimitiveType::Triangle if round_triangles => (
            SharedShape::round_triangle(
                Vector::from_array(gp.points[0]),
                Vector::from_array(gp.points[1]),
                Vector::from_array(gp.points[2]),
                gp.fatness,
            ),
            Rotation::IDENTITY,
        ),
        GpPrimitiveType::Triangle => (
            SharedShape::triangle(
                Vector::from_array(gp.points[0]),
                Vector::from_array(gp.points[1]),
                Vector::from_array(gp.points[2]),
            ),
            Rotation::IDENTITY,
        ),
    };
    // Triangle vertices already occupy world space.
    let translation = if gp.primitive_type == GpPrimitiveType::Triangle {
        Vector::ZERO
    } else {
        center
    };
    Ok(SolidCollider {
        shape,
        pose: Pose::from_parts(translation, rotation),
        friction: record.material.dynamic_friction,
    })
}

impl<C> SimulationAssemblyConstraintSession<skate2_formats::collision::AssemblyPartMaterial, C> {
    /// Clone admitted metadata; Arc owners retain the original collision geometry.
    pub(crate) fn skate3_grab_meshes(&self) -> (
        Vec<crate::simulation_dmo_assembly_part_session::dynamic_pool::DmoMeshSlot>,
        Vec<crate::simulation_dmo_assembly_part_session::dynamic_pool::DmoMeshSlot>, u32,
    ) {
        let dmos=self.dmos.as_ref().map_or_else(Vec::new,|p|p.skate3_grab_meshes());
        let (vehicles,flags)=self.world_line_queries.as_ref().map_or_else(||(Vec::new(),1),
            |q|(q.vehicle.skate3_grab_meshes(),q.skate3_island_flags()));
        (dmos,vehicles,flags)
    }
    pub(crate) fn try_refresh_skate3_sources(&mut self) -> Result<(), String> {
        if let Some(skaters) = self.skaters.as_mut() {
            skaters
                .try_refresh(&self.backend)
                .map_err(|e| format!("{e:?}"))?;
        }
        if let Some(pedestrians) = self.pedestrians.as_mut() {
            pedestrians
                .try_refresh(&self.backend)
                .map_err(|e| format!("{e:?}"))?;
        }
        if let Some(dmos) = self.dmos.as_mut() {
            dmos.try_update_serialized(
                &self.backend,
                &mut crate::serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric,
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(queries) = self.world_line_queries.as_mut() {
            queries
                .vehicle
                .try_update_serialized(
                    &self.backend,
                    &mut crate::serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric,
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    /// A DMO whose enabled aggregates are all authored meshes on ONE rigid
    /// body: that body index and the meshes. Every Object Dropper prop (and
    /// most city DMOs) authors such a mesh plus a box/capsule set over the
    /// same bounds. The mesh is the ride surface; the stacked boxes leave
    /// millimetre seams that stop a Skate 3 wheel dead (the "lip").
    fn skate3_mesh_plan(
        owner: &MeshOwner,
        records: &[SerialisedAssemblyVolumeDataState],
    ) -> Option<(u16, Vec<usize>)> {
        let mut body = None;
        let mut meshes = Vec::new();
        for record in records
            .iter()
            .filter(|record| record.enabled && record.volume_type == VolumeType::Aggregate)
        {
            if body.is_some_and(|body| body != record.rigid_body_index) {
                return None;
            }
            body = Some(record.rigid_body_index);
            match crate::dynamic_clustered_mesh_template::resolve_aggregate_target(
                owner.arena(),
                record.volume.shape_words[0],
            ) {
                Ok(CollisionAggregateRef::Mesh(index)) => meshes.push(index),
                _ => return None,
            }
        }
        Some((body?, meshes))
    }
    /// This tick's moving bodies. DMOs with an authored mesh stay ordinary
    /// dynamic solver bodies (the board and skater push them), but carry only
    /// that mesh, and their board/skeleton/wheel contacts and line queries use
    /// its native triangles at the body's current placement (`meshes`).
    pub(crate) fn try_skate3_bodies(
        &self,
        host: &mut HostBodies,
        collide_other_skaters: bool,
        material: RetailContactMaterial,
    ) -> Result<Skate3Bodies, String> {
        host.mesh_generation = host
            .mesh_generation
            .checked_add(1)
            .ok_or("Host mesh generation exhausted")?;
        let mut solids = Vec::new();
        let mut meshes = Vec::new();
        let mut append = |publication: AuthoredCollisionPublication<'_>| {
            self.append_skate3_bodies(
                host,
                &mut solids,
                publication.records(),
                publication.rigid_bodies(),
                publication.group(),
                None,
                |part| match publication {
                    AuthoredCollisionPublication::Character(p) => {
                        p.part_mesh_owner(part).map(MeshOwner::Character)
                    }
                    _ => None,
                },
            )?;
            Ok::<(), String>(())
        };
        // Match the host's admitted pair policy: Party Play's Own pool keeps
        // each player's board/skeleton together and lets different players pass.
        if collide_other_skaters && let Some(skaters) = self.skaters.as_ref() {
            for publication in skaters.publications() {
                append(publication.into())?;
            }
        }
        if let Some(pedestrians) = &self.pedestrians {
            for publication in pedestrians.publications() {
                append(publication.into())?;
            }
        }
        if let Some(dmos) = &self.dmos {
            for publication in dmos.character_publications() {
                append(publication.into())?;
            }
        }
        if let Some(queries) = &self.world_line_queries {
            for publication in queries.vehicle.character_publications() {
                append(publication.into())?;
            }
        }
        if let Some(dmos) = &self.dmos {
            for (_, publication) in dmos.entity_publications() {
                let owner = MeshOwner::Source(Arc::clone(publication.owner()));
                let records = publication.volume_data().records();
                // The mesh decodes once per retained mesh; a mesh that cannot
                // supply native triangles keeps the full authored set instead.
                let plan = Self::skate3_mesh_plan(&owner, records).and_then(|(body, indices)| {
                    let locals = indices
                        .iter()
                        .map(|&index| host.local(owner.clone(), index))
                        .collect::<Result<Vec<_>, _>>()
                        .ok()?;
                    let index = body;
                    let body = *publication.rigid_bodies().get(usize::from(body))?;
                    Some((index, body, locals))
                });
                self.append_skate3_bodies(
                    host,
                    &mut solids,
                    records,
                    publication.rigid_bodies(),
                    publication.header().group,
                    plan.as_ref().map(|(index, _, _)| *index),
                    |_| Some(owner.clone()),
                )?;
                let (Some((_, body, locals)), Some(first)) = (plan, records.first()) else {
                    continue;
                };
                // Externally driven: not a Skate 3 body at all.
                if self.backend.rigid_bodies.is_externally_driven(body) {
                    continue;
                }
                let id = host.id(body)?;
                if !solids.iter().any(|solid| solid.id == id) {
                    continue;
                }
                // Match the mesh collider: the first volume record's forward transform.
                let triangles = host.placed(id, &first.transform, &locals, material)?;
                meshes.push(Skate3MeshBody { id, triangles });
            }
        }
        host.meshes
            .retain(|entry| entry.generation == host.mesh_generation);
        host.placed
            .retain(|entry| entry.generation == host.mesh_generation);
        Ok(Skate3Bodies { solids, meshes })
    }
    fn append_skate3_bodies(
        &self,
        host: &mut HostBodies,
        solids: &mut Vec<SolidBody>,
        records: &[SerialisedAssemblyVolumeDataState],
        bodies: &[SimulationRigidBodyObservation],
        group: u32,
        mesh_body: Option<u16>,
        arena: impl Fn(usize) -> Option<MeshOwner>,
    ) -> Result<(), String> {
        for record in records {
            if !record.enabled {
                continue;
            }
            // A mesh body carries only its mesh (see skate3_mesh_plan).
            if mesh_body == Some(record.rigid_body_index)
                && record.volume_type != VolumeType::Aggregate
            {
                continue;
            }
            let body = *bodies
                .get(usize::from(record.rigid_body_index))
                .ok_or("Invalid host volume body")?;
            if self.backend.rigid_bodies.is_externally_driven(body) {
                continue;
            }
            let collider = if record.volume_type == VolumeType::Aggregate {
                let owner = arena(usize::from(record.rigid_body_index))
                    .ok_or("Missing host aggregate owner")?;
                let target = crate::dynamic_clustered_mesh_template::resolve_aggregate_target(
                    owner.arena(),
                    record.volume.shape_words[0],
                )
                .map_err(|e| e.to_string())?;
                let CollisionAggregateRef::Mesh(index) = target else {
                    return Err("Host aggregate requires its native mapped-array expansion".into());
                };
                let shape = host.mesh(owner, index)?;
                // Match GetClusteredMesh827ce4e8: the first volume record owns
                // the assembly forward transform, not the wrapper's local TM.
                let forward = records.first().ok_or("Empty host assembly")?.transform;
                SolidCollider {
                    shape,
                    pose: pose(forward)?,
                    friction: record.material.dynamic_friction,
                }
            } else if record.volume_type == VolumeType::Null {
                continue;
            } else {
                primitive_collider(record, false)?
            };
            let id = host.id(body)?;
            if let Some(solid) = solids.iter_mut().find(|solid| solid.id == id) {
                solid.colliders.push(collider);
                continue;
            }
            let state = self.backend.try_external_body_snapshot(body)?;
            let inertia = self
                .backend
                .rigid_bodies
                .try_external_principal_inertia(body)?;
            let rotation = Rotation::from_xyzw(
                state.quaternion[0],
                state.quaternion[1],
                state.quaternion[2],
                state.quaternion[3],
            )
            .normalize();
            let center = Vector::from_array(state.center_of_mass);
            solids.push(SolidBody {
                id,
                pose: Pose::from_parts(center, rotation),
                center_of_mass: center,
                inertia_rotation: rotation,
                inverse_mass: state.inverse_mass,
                inverse_inertia: Vector::from_array(inertia),
                linvel: Vector::from_array(state.velocity),
                angvel: Vector::from_array(state.omega),
                contact_group: skate3_group(group),
                colliders: vec![collider],
            });
        }
        Ok(())
    }
    pub(crate) fn try_apply_skate3_reactions(
        &mut self,
        host: &HostBodies,
        reactions: Vec<crate::skate3_native::BodyReaction>,
    ) -> Result<(), String> {
        for reaction in reactions {
            let body = host
                .body(reaction.id)
                .ok_or("Unbound Skate 3 body reaction")?;
            self.backend
                .rigid_bodies
                .try_apply_external_reaction(body, reaction)?;
        }
        Ok(())
    }
}
