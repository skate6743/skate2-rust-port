//! Per-frame DMO pool update and the DMO line collection.
//!
//! `DynamicPool::BeginUpdate` 0x827c93f0 empties the mesh collection, then
//! for every assembly in pool order and every Part in order hands the Part
//! volume and `Part::GetTransform` to `DynamicPool::AddVolume` 0x827c86d0; a
//! refused add abandons the walk AND the base `AssemblyPool::BeginUpdate`.
//! The serialized half runs later in the frame, once the body pool has its
//! frame identity; no body moves between the two (bodies integrate only in
//! the solver).
//! `AddVolume` skips a disabled volume (unless `kIgnoreVolumeEnableFlag`) or
//! a primitive, recurses into a `SimpleMappedArray`, and publishes a
//! `ClusteredMesh` through `ClusteredMeshData::Reset` 0x827c8378: the forward
//! transform is the Part transform itself, its cofactor inverse and the mesh
//! KD bounds. The base `EndUpdate` 0x82e0bb10 and the serialized volume update
//! run through `try_update_serialised_assembly_pool` (Begin, bounds job, End).

use super::{MemberContent, SimulationDmoAssemblyHandle, SimulationDmoAssemblyPool, SourceBackend};
use crate::{
    collision_line_collection::{
        CollisionLineCollectionError, CollisionLineCollectionMesh, CollisionLineInput,
        try_publish_collection_line,
    },
    collision_mesh_line::{CollisionMeshLineNearest, CollisionMeshOccurrence},
    dynamic_assembly_session::{
        character_publication::CharacterPublicationError,
        source_parts::DynamicAssemblySourcePartError,
    },
    dynamic_clustered_mesh_template::{
        DynamicClusteredMeshTemplateError, resolve_aggregate_target,
    },
    living_world_grind_services::character_models::ModelCollisionGeometry,
    physics::{TrajectoryCollisionWorld, TrajectoryGroundTriangle, TrajectorySweepHit},
    reckoning_numeric::ReckoningNumeric,
    serialised_assembly_bbox::SerialisedAssemblyPartBBoxNumeric,
    serialised_assembly_create::{
        PreparedSerialisedAssembly, SerialisedAssemblyLiveAssemblyInput,
        pool_update::{
            AssemblyPoolEntryUpdateError, AssemblyPoolUpdateEntry,
            SerialisedAssemblyPoolUpdateError, SerialisedAssemblyPoolUpdateInput,
            try_update_assembly_pool_entries,
        },
    },
    simulation_assembly_backend::source::SimulationAssemblySourceError,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    static_ground_pool::{
        TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY, native_clamp_line_parameter,
        trajectory_line_has_extent, triangle_normal,
    },
    world::{
        CollisionAggregateRef, CollisionArena, CollisionMesh, inverse_collision_affine_reset,
        transform_collision_aabb,
    },
};
use skate2_formats::collision::{
    CollisionError, Volume, VolumeType, sphere_segment_triangle_intersection, triangle_aabb_overlap,
};
use std::{fmt, sync::Arc};

/// One `ClusteredMeshData` slot of the DMO collection.
#[derive(Clone)]
pub(crate) struct DmoMeshSlot {
    owner: SimulationDmoAssemblyHandle,
    backing: MeshArenaOwner,
    mesh_index: usize,
    forward: [f32; 16],
    inverse: [f32; 16],
    kd_bounds: Result<([f32; 3], [f32; 3]), CollisionError>,
}

/// Retain actual geometry backing; constructed models do not invent an arena
/// plan or serialized object ID merely to enter the common mesh collection.
#[derive(Clone)]
enum MeshArenaOwner {
    Source(Arc<SimulationNormalArenaPlan>),
    Constructed(Arc<CollisionArena>),
}
impl MeshArenaOwner {
    fn arena(&self) -> &CollisionArena {
        match self {
            Self::Source(plan) => &plan.static_plan().asset().collision,
            Self::Constructed(arena) => arena,
        }
    }
}

impl DmoMeshSlot {
    pub(crate) fn owner(&self) -> &SimulationDmoAssemblyHandle {
        &self.owner
    }
    #[cfg(all(test,feature = "skate3-runtime"))]
    pub(crate) fn skate3_probe_lines(&self) -> Result<Vec<([f32;4],[f32;4])>,String> {
        let (minimum,maximum)=self.kd_bounds.as_ref().map_err(|e|e.to_string())?;
        let (minimum,maximum)=transform_collision_aabb(*minimum,*maximum,self.forward);
        let mut lines=Vec::new();
        for axis in 0..3 { for a in 1..5 { for b in 1..5 {
            let mut start=[0.;4];let mut end=[0.;4];
            for i in 0..3 {
                let t=if i==(axis+1)%3 {a as f32/5.} else {b as f32/5.};
                start[i]=(maximum[i]-minimum[i]).mul_add(t,minimum[i]);end[i]=start[i];
            }
            start[axis]=minimum[axis]-0.25;end[axis]=maximum[axis]+0.25;
            lines.push((start,end));
        }}}
        Ok(lines)
    }
}

#[derive(Debug)]
pub(crate) enum SimulationDmoPoolUpdateError<N> {
    /// `AddVolume` asserts `meshCount < mCapacity` (DynamicPool.cpp line 179).
    MeshCapacity,
    ForeignBackend,
    PoisonedDefinition,
    PartCount,
    MissingRoot(usize),
    Character {
        assembly: usize,
        source: CharacterPublicationError,
    },
    MissingMesh {
        mesh_index: usize,
    },
    Aggregate(DynamicClusteredMeshTemplateError),
    Part(DynamicAssemblySourcePartError),
    Source(SimulationAssemblySourceError),
    Update(SerialisedAssemblyPoolUpdateError<N>),
}

impl<N: fmt::Debug> fmt::Display for SimulationDmoPoolUpdateError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DMO pool update: {self:?}")
    }
}

impl<N: fmt::Debug> std::error::Error for SimulationDmoPoolUpdateError<N> {}

/// The mesh-collection walk shared by every Part of one assembly.
struct MeshWalk<'a> {
    capacity: usize,
    arena: &'a CollisionArena,
    backing: &'a MeshArenaOwner,
    owner: &'a SimulationDmoAssemblyHandle,
    transform: [f32; 16],
    ignore_volume_enable: bool,
}

impl MeshWalk<'_> {
    /// `AddVolume` 0x827c86d0; `Ok(false)` is its capacity refusal.
    fn try_add<N>(
        &self,
        meshes: &mut Vec<DmoMeshSlot>,
        volume: &Volume,
        aggregate: Option<CollisionAggregateRef>,
    ) -> Result<bool, SimulationDmoPoolUpdateError<N>> {
        if (volume.flags & 1 == 0 && !self.ignore_volume_enable)
            || volume.volume_type() != Some(VolumeType::Aggregate)
        {
            return Ok(true);
        }
        match aggregate {
            Some(CollisionAggregateRef::SimplePrimitiveSet(index)) => {
                let set = self
                    .arena
                    .simple_primitive_sets
                    .get(index)
                    .ok_or(SimulationDmoPoolUpdateError::MissingMesh { mesh_index: index })?;
                for child in &set.volumes {
                    let nested = if child.volume_type() == Some(VolumeType::Aggregate) {
                        Some(
                            resolve_aggregate_target(self.arena, child.shape_words[0])
                                .map_err(SimulationDmoPoolUpdateError::Aggregate)?,
                        )
                    } else {
                        None
                    };
                    if !self.try_add::<N>(meshes, child, nested)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Some(CollisionAggregateRef::Mesh(mesh_index)) => {
                if meshes.len() >= self.capacity {
                    return Err(SimulationDmoPoolUpdateError::MeshCapacity);
                }
                let mesh = self
                    .arena
                    .meshes
                    .get(mesh_index)
                    .ok_or(SimulationDmoPoolUpdateError::MissingMesh { mesh_index })?;
                let kd_bounds = mesh.clustered_mesh.kd_tree(&mesh.object_bytes).map(|tree| {
                    (
                        std::array::from_fn(|axis| tree.bounding_box_min[axis]),
                        std::array::from_fn(|axis| tree.bounding_box_max[axis]),
                    )
                });
                meshes.push(DmoMeshSlot {
                    owner: self.owner.clone(),
                    backing: self.backing.clone(),
                    mesh_index,
                    forward: self.transform,
                    inverse: inverse_collision_affine_reset(self.transform),
                    kd_bounds,
                });
                Ok(meshes.len() < self.capacity)
            }
            Some(CollisionAggregateRef::PrimitiveSet(_)) | None => Ok(true),
        }
    }
}

impl SimulationDmoAssemblyPool {
    /// `DynamicPool::BeginUpdate` 0x827c93f0's mesh half: the collection is
    /// emptied and every Part volume of every member is offered to
    /// `AddVolume` at its `Part::GetTransform`. `Ok(false)` is a capacity
    /// refusal, after which native skips the base `AssemblyPool::BeginUpdate`.
    pub(crate) fn try_rebuild_meshes<N>(
        &mut self,
        backend: &SourceBackend,
        ignore_volume_enable: bool,
    ) -> Result<bool, SimulationDmoPoolUpdateError<N>> {
        if backend.authority() != self.backend {
            return Err(SimulationDmoPoolUpdateError::ForeignBackend);
        }
        self.meshes.clear();
        for member in &self.members {
            let transforms = member.content.world_part_transforms(backend);
            match &member.content {
                MemberContent::Source { candidate, .. } => {
                    let template = candidate.parts.owner.template();
                    let backing = MeshArenaOwner::Source(Arc::clone(template.owner()));
                    let arena = backing.arena();
                    for (part, transform) in template.parts().iter().zip(transforms) {
                        let root = &arena.aggregate_roots[part.root_index];
                        let walk = MeshWalk {
                            capacity: self.capacity,
                            arena,
                            backing: &backing,
                            owner: &member.handle,
                            transform,
                            ignore_volume_enable,
                        };
                        if !walk.try_add::<N>(&mut self.meshes, &root.volume, root.aggregate)? {
                            return Ok(false);
                        }
                    }
                }
                MemberContent::Character { candidate, .. } => {
                    let definition = candidate
                        .assembly
                        .definition
                        .lock()
                        .map_err(|_| SimulationDmoPoolUpdateError::PoisonedDefinition)?;
                    if definition.parts.len() != transforms.len() {
                        return Err(SimulationDmoPoolUpdateError::PartCount);
                    }
                    for (part, transform) in definition.parts.iter().zip(transforms) {
                        match &part.geometry {
                            // AddVolume returns immediately for primitive Parts.
                            ModelCollisionGeometry::Primitive(_) => {}
                            ModelCollisionGeometry::VehicleMappedArray {
                                source,
                                root,
                                extra_box,
                            } => {
                                // Fresh enabled SimpleMappedArray: original model then extra box,
                                // both at Part.GetTransform, with no composed child transform.
                                let backing = MeshArenaOwner::Constructed(Arc::clone(source));
                                let arena = backing.arena();
                                let root = arena
                                    .aggregate_roots
                                    .get(*root)
                                    .ok_or(SimulationDmoPoolUpdateError::MissingRoot(*root))?;
                                let walk = MeshWalk {
                                    capacity: self.capacity,
                                    arena,
                                    backing: &backing,
                                    owner: &member.handle,
                                    transform,
                                    ignore_volume_enable,
                                };
                                if !walk.try_add::<N>(
                                    &mut self.meshes,
                                    &root.volume,
                                    root.aggregate,
                                )? {
                                    return Ok(false);
                                }
                                if !walk.try_add::<N>(&mut self.meshes, extra_box, None)? {
                                    return Ok(false);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(true)
    }

    /// The base `AssemblyPool::BeginUpdate` (serialized volumes), its bounds
    /// job and `EndUpdate` 0x82e0bb10. The returned ordinals are EndUpdate's
    /// active list. Requires the body pool's current frame.
    pub(crate) fn try_update_serialized<N: SerialisedAssemblyPartBBoxNumeric>(
        &mut self,
        backend: &SourceBackend,
        numeric: &mut N,
    ) -> Result<Vec<usize>, SimulationDmoPoolUpdateError<N::Error>> {
        if backend.authority() != self.backend {
            return Err(SimulationDmoPoolUpdateError::ForeignBackend);
        }
        if self.members.is_empty() {
            return Ok(Vec::new());
        }
        let mut staged = Vec::new();
        for member in &self.members {
            staged.push(match &member.content {
                MemberContent::Source { candidate, .. } => {
                    let template = candidate.parts.owner.template();
                    let inputs = candidate
                        .parts
                        .parts
                        .try_source_create_parts(backend, template, &candidate.part_user_data)
                        .map_err(SimulationDmoPoolUpdateError::Part)?;
                    let flags = template
                        .update_volume_sources()
                        .iter()
                        .map(|source| source.volume.flags)
                        .collect::<Vec<_>>();
                    Some((inputs, flags))
                }
                MemberContent::Character { .. } => None,
            });
        }
        let mut entries = Vec::new();
        for (member, staged) in self.members.iter_mut().zip(&staged) {
            match &mut member.content {
                MemberContent::Source {
                    candidate,
                    publication,
                } => {
                    let template = candidate.parts.owner.template();
                    let context = backend
                        .try_source_context(template.owner())
                        .map_err(SimulationDmoPoolUpdateError::Source)?;
                    let (parts, flags) = staged.as_ref().expect("source entry staged above");
                    entries.push(AssemblyPoolUpdateEntry::Source {
                        input: SerialisedAssemblyPoolUpdateInput {
                            assembly: publication,
                            template,
                            inertia_owner: context.inertia_owner,
                            parts,
                            volume_flags: flags,
                            live: SerialisedAssemblyLiveAssemblyInput {
                                group: candidate.group,
                                culling_table: candidate.culling_table.as_ref(),
                            },
                        },
                        image: context.image_domain,
                    });
                }
                MemberContent::Character {
                    candidate,
                    publication,
                } => {
                    entries.push(AssemblyPoolUpdateEntry::Character {
                        assembly: publication,
                        owner: &candidate.assembly,
                        backend,
                    });
                }
            }
        }
        let active =
            try_update_assembly_pool_entries(&mut entries, backend.collision_body_pool(), numeric)
                .map_err(|error| match error {
                    AssemblyPoolEntryUpdateError::Source(error) => {
                        SimulationDmoPoolUpdateError::Update(error)
                    }
                    AssemblyPoolEntryUpdateError::Character { assembly, source } => {
                        SimulationDmoPoolUpdateError::Character { assembly, source }
                    }
                })?;
        drop(entries);
        for member in &mut self.members {
            member.bounds = match &member.content {
                MemberContent::Source { publication, .. } => publication.header().aabb,
                MemberContent::Character { publication, .. } => publication.bounds(),
            };
        }
        Ok(active)
    }

    /// Serialized publications in pool order (`AssemblyPool::Begin..End`).
    pub(crate) fn publications(&self) -> impl Iterator<Item = &PreparedSerialisedAssembly> {
        self.members
            .iter()
            .filter_map(|member| member.content.source().map(|(_, publication)| publication))
    }

    /// `publications` with each one's owning entity ID, in the same pool order.
    pub(crate) fn entity_publications(
        &self,
    ) -> impl Iterator<Item = (Option<u32>, &PreparedSerialisedAssembly)> {
        self.members.iter().filter_map(|member| {
            member
                .content
                .source()
                .map(|(candidate, publication)| (candidate.entity_id(), publication))
        })
    }

    /// The DMO line collection: every published mesh with its forward and
    /// inverse Part transforms and KD bounds (`BF_Dmo` line batches).
    pub(crate) fn try_line_query<N>(
        &self,
        input: CollisionLineInput,
        numeric: &mut N,
    ) -> Result<
        CollisionMeshLineNearest<SimulationDmoAssemblyHandle>,
        CollisionLineCollectionError<N::Error>,
    >
    where
        N: AssemblyGroundNumeric + ReckoningNumeric,
    {
        DmoMeshSlot::try_snapshot_line(&self.meshes, input, numeric)
    }

    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn skate3_grab_meshes(&self) -> Vec<DmoMeshSlot> { self.meshes.clone() }

    /// `DynamicPool::Remove` 0x827c9300..93e4: every mesh the assembly owns,
    /// each replaced by the last slot and the same index rechecked.
    pub(crate) fn remove_owned_meshes(&mut self, owner: &SimulationDmoAssemblyHandle) {
        let mut index = 0;
        while index < self.meshes.len() {
            if self.meshes[index].owner.same_as(owner) {
                self.meshes.swap_remove(index);
            } else {
                index += 1;
            }
        }
    }
}

/// `fma(z, p.z, fma(y, p.y, fma(x, p.x, w)))`: the affine `vmaddfp` chain the
/// per-mesh `LineTest` 0x827c01bc..0250 applies to the line endpoints.
fn transform_point(transform: &[f32; 16], point: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let x = transform[axis].mul_add(point[0], transform[12 + axis]);
        let y = transform[4 + axis].mul_add(point[1], x);
        transform[8 + axis].mul_add(point[2], y)
    })
}

impl DmoMeshSlot {
    pub(crate) fn try_snapshot_line<N: AssemblyGroundNumeric + ReckoningNumeric>(
        slots: &[Self], input: CollisionLineInput, numeric: &mut N,
    ) -> Result<CollisionMeshLineNearest<SimulationDmoAssemblyHandle>, CollisionLineCollectionError<N::Error>> {
        let meshes = slots.iter().enumerate().map(|(index, slot)| {
            let mesh = slot.mesh();
            CollisionLineCollectionMesh {
                occurrence: CollisionMeshOccurrence {
                    mesh: &mesh.clustered_mesh,
                    bytes: &mesh.object_bytes,
                    forward: &slot.forward,
                    inverse: &slot.inverse,
                    owner: Some(slot.owner.clone()),
                    slot: index,
                },
                local_bounds: &slot.kd_bounds,
            }
        });
        let mut result = CollisionMeshLineNearest::new();
        try_publish_collection_line(meshes, input, &mut result, numeric)?;
        Ok(result)
    }
    fn mesh(&self) -> &CollisionMesh {
        &self.backing.arena().meshes[self.mesh_index]
    }

    /// Collection-level culling: the world box through the slot inverse
    /// (`AABBoxTemplate::Transform`) against the mesh KD bounds.
    fn local_box(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
    ) -> Result<Option<([f32; 3], [f32; 3])>, CollisionError> {
        let (local_minimum, local_maximum) =
            transform_collision_aabb(minimum, maximum, self.inverse);
        let &(tree_minimum, tree_maximum) = self.kd_bounds.as_ref().map_err(Clone::clone)?;
        let overlaps = (0..3).all(|axis| {
            !(tree_minimum[axis] > local_maximum[axis] || local_minimum[axis] > tree_maximum[axis])
        });
        Ok(overlaps.then_some((local_minimum, local_maximum)))
    }
}

/// The DynamicPool mesh collection behind `LineTest<TrajectoryLineTestFunctor>`
/// 0x827c1300/0x827c01a0 and `BoxTest<TrajectoryBoxTestFunctor>` 0x827c1548:
/// queries move into mesh space through each slot's inverse, KD units are
/// instanced through the forward transform (`UnitGetOverlappingGPInstances`),
/// and the world-space tests of the ground collection follow.
pub(crate) struct DmoMeshGeometry<'a>(pub(crate) &'a SimulationDmoAssemblyPool);

impl TrajectoryCollisionWorld for DmoMeshGeometry<'_> {
    type Error = CollisionError;

    fn sweep_sphere_segment(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<TrajectorySweepHit>, Self::Error> {
        let delta: [f32; 3] = std::array::from_fn(|axis| end[axis] - start[axis]);
        if !trajectory_line_has_extent(delta) {
            return Ok(None);
        }
        let line_minimum: [f32; 3] =
            std::array::from_fn(|axis| start[axis].min(end[axis]) - radius);
        let line_maximum: [f32; 3] =
            std::array::from_fn(|axis| start[axis].max(end[axis]) + radius);
        let mut nearest_parameter = f32::MAX;
        let mut nearest = None;
        for slot in &self.0.meshes {
            if slot.local_box(line_minimum, line_maximum)?.is_none() {
                continue;
            }
            let mesh = slot.mesh();
            let local_start = transform_point(&slot.inverse, start);
            let local_end = transform_point(&slot.inverse, end);
            let tree = mesh.clustered_mesh.kd_tree(&mesh.object_bytes)?;
            let mut query =
                tree.line_range_query(&mesh.object_bytes, local_start, local_end, radius)?;
            let mut ranges = Vec::new();
            while let Some(range) = query.next_range()? {
                ranges.push(range);
            }
            for triangle in mesh
                .clustered_mesh
                .triangles_for_leaf_ranges(&mesh.object_bytes, &ranges)?
            {
                let vertices = triangle
                    .vertices
                    .map(|vertex| transform_point(&slot.forward, vertex));
                let Some(hit) =
                    sphere_segment_triangle_intersection(start, delta, radius, vertices)
                else {
                    continue;
                };
                let parameter = native_clamp_line_parameter(hit.line_parameter);
                if parameter < nearest_parameter {
                    nearest_parameter = parameter;
                    nearest = Some(TrajectorySweepHit {
                        position: hit.position,
                        normal: triangle_normal(vertices),
                        surface_id: triangle.surface_id.map_or(0, u32::from),
                    });
                }
            }
        }
        Ok(nearest)
    }

    fn triangles_overlapping_aabb(
        &mut self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        _: usize,
    ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error> {
        let mut result = Vec::with_capacity(TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY);
        for slot in &self.0.meshes {
            let Some((local_minimum, local_maximum)) = slot.local_box(minimum, maximum)? else {
                continue;
            };
            let mesh = slot.mesh();
            let tree = mesh.clustered_mesh.kd_tree(&mesh.object_bytes)?;
            let mut query =
                tree.aabb_range_query(&mesh.object_bytes, local_minimum, local_maximum)?;
            let mut ranges = Vec::new();
            while let Some(range) = query.next_range()? {
                ranges.push(range);
            }
            for triangle in mesh
                .clustered_mesh
                .triangles_for_leaf_ranges(&mesh.object_bytes, &ranges)?
            {
                let vertices = triangle
                    .vertices
                    .map(|vertex| transform_point(&slot.forward, vertex));
                if !triangle_aabb_overlap(minimum, maximum, vertices) {
                    continue;
                }
                if result.len() >= TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY {
                    return Ok(result);
                }
                result.push(TrajectoryGroundTriangle {
                    vertices,
                    surface_id: triangle.surface_id.map_or(0, u32::from),
                });
            }
        }
        Ok(result)
    }
}
