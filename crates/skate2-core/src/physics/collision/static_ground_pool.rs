//! Owned static-ground publication for processed Environment instances.
//!
//! `WorldStreamerView::AddVolume` (`0x827c8578`) walks an Environment
//! instance's collision-model roots forward and sends each raw volume to
//! `GroundPool::AddVolume` (`0x827c84e0`) with identity placement. Removal
//! (`0x827c9088` -> `0x827c8e60`) walks the same roots forward, removes the
//! first pointer-identical slot, and fills the hole from the final slot.
//! This module reconstructs that bounded lower seam. Unlock/challenge state,
//! batch synchronization, assembly-backed models, and native allocation
//! failure remain separate boundaries.

use crate::{
    physics::{TrajectoryGroundTriangle, TrajectorySweepHit},
    simulation_static_arena_plan::{SimulationStaticArenaPlan, SimulationStaticOperation},
    world::CollisionAggregateRef,
};
use skate2_formats::collision::PreparedMeshTriangles;
use skate2_formats::collision::{
    CollisionError, LeafClusterCache, MeshTriangle, MeshTriangleHit, sphere_segment_triangle_intersection,
    triangle_aabb_overlap,
};
use std::{
    error::Error,
    fmt,
    sync::{Arc, OnceLock},
};
type TriangleDecodeCache = OnceLock<Result<PreparedMeshTriangles, CollisionError>>;

/// Identity placement constructed by `WorldStreamerView::AddVolume`.
pub const STATIC_GROUND_IDENTITY_TRANSFORM: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 0.0,
];

/// Fixed mesh-record capacity passed by `World::World` to `GroundPool`.
pub const STATIC_GROUND_NATIVE_CAPACITY: usize = 128;

const TRAJECTORY_LINE_MINIMUM_DELTA: f32 = f32::from_bits(0x3780_0000);
pub(crate) const TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY: usize = 64;

/// A completely validated Environment source retained independently of the
/// resident collection wrapper.
///
/// This is intentionally not `Clone`: the future unlock registry owns one
/// logical source record, while individual pool slots take their own internal
/// lifetime holds.
#[derive(Debug)]
pub struct StaticEnvironmentSource {
    owner: Arc<SimulationStaticArenaPlan>,
    operation_index: usize,
    source_resource_index: usize,
    source_record_start: usize,
    instance_index: usize,
    roots: Vec<StaticEnvironmentRoot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StaticEnvironmentRoot {
    collision_root_index: usize,
    mesh_index: usize,
}

impl StaticEnvironmentSource {
    /// Validates one kind-10 operation and retains its complete arena owner.
    ///
    /// A missing collision model or an ordinary model with zero roots is a
    /// valid no-op source. Assembly models and non-mesh aggregate roots remain
    /// explicitly unsupported instead of being projected into guessed state.
    ///
    /// # Errors
    ///
    /// Returns a typed error before constructing a source when an index or
    /// supported root relationship is invalid.
    pub fn from_operation(
        owner: Arc<SimulationStaticArenaPlan>,
        operation_index: usize,
    ) -> Result<Self, StaticGroundSourceError> {
        let (source_resource_index, source_record_start, instance_index) =
            match owner.operations().get(operation_index) {
                Some(SimulationStaticOperation::Environment {
                    source_resource_index,
                    source_record_start,
                    instance_index,
                    ..
                }) => (
                    *source_resource_index,
                    *source_record_start,
                    *instance_index,
                ),
                Some(SimulationStaticOperation::Grind { .. }) => {
                    return Err(StaticGroundSourceError::NotEnvironment { operation_index });
                }
                None => return Err(StaticGroundSourceError::MissingOperation { operation_index }),
            };
        let collision = &owner.asset().collision;
        let instance = collision.instances.get(instance_index).ok_or(
            StaticGroundSourceError::MissingInstance {
                operation_index,
                instance_index,
            },
        )?;
        let mut roots = Vec::new();
        if let Some(model_index) = instance.collision_model {
            let model = collision.collision_models.get(model_index).ok_or(
                StaticGroundSourceError::MissingCollisionModel {
                    operation_index,
                    model_index,
                },
            )?;
            if let Some(object_id) = model.assembly_object_id {
                return Err(StaticGroundSourceError::AssemblyModel {
                    operation_index,
                    model_index,
                    object_id,
                });
            }
            roots
                .try_reserve_exact(model.collision_roots.len())
                .map_err(|_| StaticGroundSourceError::RootStorageAllocation {
                    operation_index,
                    root_count: model.collision_roots.len(),
                })?;
            for (root_ordinal, &collision_root_index) in model.collision_roots.iter().enumerate() {
                let root = collision.aggregate_roots.get(collision_root_index).ok_or(
                    StaticGroundSourceError::MissingCollisionRoot {
                        operation_index,
                        root_ordinal,
                        collision_root_index,
                    },
                )?;
                let mesh_index = match root.aggregate {
                    Some(CollisionAggregateRef::Mesh(mesh_index)) => mesh_index,
                    aggregate => {
                        return Err(StaticGroundSourceError::UnsupportedCollisionRoot {
                            operation_index,
                            root_ordinal,
                            collision_root_index,
                            aggregate,
                        });
                    }
                };
                if collision.meshes.get(mesh_index).is_none() {
                    return Err(StaticGroundSourceError::MissingMesh {
                        operation_index,
                        root_ordinal,
                        mesh_index,
                    });
                }
                roots.push(StaticEnvironmentRoot {
                    collision_root_index,
                    mesh_index,
                });
            }
        }
        Ok(Self {
            owner,
            operation_index,
            source_resource_index,
            source_record_start,
            instance_index,
            roots,
        })
    }

    #[must_use]
    pub fn guid(&self) -> u64 {
        self.owner.asset().collision.instances[self.instance_index].guid
    }

    /// Key used by `UnlockableLocationMan`'s fixed Environment map.
    #[must_use]
    pub fn local_guid(&self) -> u64 {
        self.owner.asset().collision.instances[self.instance_index].local_guid
    }

    #[must_use]
    pub const fn operation_index(&self) -> usize {
        self.operation_index
    }

    #[must_use]
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Compares native-pointer-equivalent backing-record identity without
    /// exposing a host pointer. Repeated TOC aliases of one record compare
    /// equal; equal values at different retained byte starts do not.
    #[must_use]
    pub fn is_same_backing_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.source_resource_index == other.source_resource_index
            && self.source_record_start == other.source_record_start
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticGroundSourceError {
    MissingOperation {
        operation_index: usize,
    },
    NotEnvironment {
        operation_index: usize,
    },
    MissingInstance {
        operation_index: usize,
        instance_index: usize,
    },
    MissingCollisionModel {
        operation_index: usize,
        model_index: usize,
    },
    AssemblyModel {
        operation_index: usize,
        model_index: usize,
        object_id: u32,
    },
    RootStorageAllocation {
        operation_index: usize,
        root_count: usize,
    },
    MissingCollisionRoot {
        operation_index: usize,
        root_ordinal: usize,
        collision_root_index: usize,
    },
    UnsupportedCollisionRoot {
        operation_index: usize,
        root_ordinal: usize,
        collision_root_index: usize,
        aggregate: Option<CollisionAggregateRef>,
    },
    MissingMesh {
        operation_index: usize,
        root_ordinal: usize,
        mesh_index: usize,
    },
}

impl fmt::Display for StaticGroundSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid static Environment source: {self:?}")
    }
}

impl Error for StaticGroundSourceError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticGroundPoolCreateError {
    StorageAllocation { capacity: usize },
}

impl fmt::Display for StaticGroundPoolCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "cannot allocate static ground pool: {self:?}")
    }
}

impl Error for StaticGroundPoolCreateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticGroundCapacityError {
    pub capacity: usize,
    pub occupied: usize,
    pub requested: usize,
}

impl fmt::Display for StaticGroundCapacityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "static ground pool capacity {} cannot hold {} occupied plus {} requested roots",
            self.capacity, self.occupied, self.requested
        )
    }
}

impl Error for StaticGroundCapacityError {}

#[derive(Debug)]
struct StaticGroundSlot {
    triangle_decode: Arc<TriangleDecodeCache>,
    // Shares this slot's immutable arena lifetime; removal drops decoded clusters.
    cluster_decode: LeafClusterCache,
    owner: Arc<SimulationStaticArenaPlan>,
    operation_index: usize,
    root_ordinal: usize,
    collision_root_index: usize,
    mesh_index: usize,
    kd_bounds: Result<([f32; 3], [f32; 3]), CollisionError>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained line-query inverse awaits the live World query owner"
        )
    )]
    inverse: [f32; 16],
}

/// Immutable view of one actually published CPU ground mesh. The pool borrow
/// pins slot order and backing bytes; saved KD-bound failures remain lazy.
pub(crate) struct StaticGroundAssemblyMesh<'a> {
    pub(crate) mesh: &'a crate::world::CollisionMesh,
    pub(crate) kd_bounds: &'a Result<([f32; 3], [f32; 3]), CollisionError>,
    pub(crate) cluster_decode: &'a LeafClusterCache,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticGroundSlotSnapshot {
    pub operation_index: usize,
    pub root_ordinal: usize,
    pub collision_root_index: usize,
    pub mesh_index: usize,
}

/// Fixed-capacity owner for the currently published static ground roots.
pub struct StaticGroundPool {
    capacity: usize,
    slots: Vec<StaticGroundSlot>,
    /// Added meshes decode their triangles on a background thread.
    warm_decodes: bool,
}

impl fmt::Debug for StaticGroundPool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticGroundPool")
            .field("capacity", &self.capacity)
            .field("published_volume_count", &self.slots.len())
            .field("slots", &self.snapshot())
            .finish()
    }
}

impl StaticGroundPool {
    /// Runs the original no-spatial-index line job on these published slots.
    /// This is the ground child only; World default queries also require the
    /// selected island's actual dynamic and conditional vehicle collections.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "ground query awaits the live World query owner")
    )]
    pub(crate) fn try_run_line_test<N>(
        &self,
        input: crate::collision_line_collection::CollisionLineInput,
        numeric: &mut N,
    ) -> Result<
        crate::collision_mesh_line::CollisionMeshLineNearest<()>,
        crate::collision_line_collection::CollisionLineCollectionError<N::Error>,
    >
    where
        N: crate::simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric
            + crate::reckoning_numeric::ReckoningNumeric,
    {
        let meshes = self.slots.iter().enumerate().map(|(index, slot)| {
            let mesh = &slot.owner.asset().collision.meshes[slot.mesh_index];
            crate::collision_line_collection::CollisionLineCollectionMesh {
                occurrence: crate::collision_mesh_line::CollisionMeshOccurrence {
                    mesh: &mesh.clustered_mesh,
                    bytes: &mesh.object_bytes,
                    forward: &STATIC_GROUND_IDENTITY_TRANSFORM,
                    inverse: &slot.inverse,
                    owner: None,
                    slot: index,
                },
                local_bounds: &slot.kd_bounds,
            }
        });
        let mut result = crate::collision_mesh_line::CollisionMeshLineNearest::new();
        crate::collision_line_collection::try_publish_collection_line(
            meshes,
            input,
            &mut result,
            numeric,
        )?;
        Ok(result)
    }

    /// `BoxTest<...,1>` consumes the current mesh array, without an octree.
    /// This includes the source's swap-with-last order after removals.
    pub(crate) fn assembly_ground_meshes(
        &self,
    ) -> impl Iterator<Item = StaticGroundAssemblyMesh<'_>> {
        self.slots.iter().map(|slot| StaticGroundAssemblyMesh {
            mesh: &slot.owner.asset().collision.meshes[slot.mesh_index],
            kd_bounds: &slot.kd_bounds,
            cluster_decode: &slot.cluster_decode,
        })
    }

    /// Allocates the complete host slot array up front, matching the native
    /// constructor's fixed-storage lifetime. The native world passes
    /// [`STATIC_GROUND_NATIVE_CAPACITY`]; an explicit value remains accepted
    /// for smaller tests and host injection.
    ///
    /// # Errors
    ///
    /// Returns a typed host allocation error without constructing a pool.
    pub fn try_new(capacity: usize) -> Result<Self, StaticGroundPoolCreateError> {
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(capacity)
            .map_err(|_| StaticGroundPoolCreateError::StorageAllocation { capacity })?;
        Ok(Self {
            capacity,
            slots,
            warm_decodes: false,
        })
    }

    /// From now on, each added mesh decodes its triangles on a background
    /// thread: the same decode the first trajectory query near it would run on
    /// the simulation thread. A query that arrives first waits for that decode.
    pub fn warm_decodes_in_background(&mut self) {
        self.warm_decodes = true;
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn published_volume_count(&self) -> usize {
        self.slots.len()
    }

    /// Publishes every source root in collision-model order.
    ///
    /// Capacity is host-preflighted before mutation because native overflow
    /// behavior is not yet recovered. Within the reserved capacity this call
    /// performs no allocation and retains repeated roots exactly.
    ///
    /// # Errors
    ///
    /// Returns a typed capacity error without mutation when all source roots
    /// do not fit in the preallocated pool.
    pub fn add(
        &mut self,
        source: &StaticEnvironmentSource,
    ) -> Result<(), StaticGroundCapacityError> {
        let requested = source.roots.len();
        let Some(final_count) = self.slots.len().checked_add(requested) else {
            return Err(StaticGroundCapacityError {
                capacity: self.capacity,
                occupied: self.slots.len(),
                requested,
            });
        };
        if final_count > self.capacity {
            return Err(StaticGroundCapacityError {
                capacity: self.capacity,
                occupied: self.slots.len(),
                requested,
            });
        }
        self.append_reserved(source);
        Ok(())
    }

    /// Preflights and publishes several sources without a capacity-partial
    /// result. Source and root order are retained.
    ///
    /// # Errors
    ///
    /// Returns a typed capacity error before mutation when the complete batch
    /// does not fit.
    pub fn add_all(
        &mut self,
        sources: &[StaticEnvironmentSource],
    ) -> Result<(), StaticGroundCapacityError> {
        let requested = sources.iter().try_fold(0_usize, |total, source| {
            total.checked_add(source.roots.len())
        });
        let Some(requested) = requested else {
            return Err(StaticGroundCapacityError {
                capacity: self.capacity,
                occupied: self.slots.len(),
                requested: usize::MAX,
            });
        };
        let Some(final_count) = self.slots.len().checked_add(requested) else {
            return Err(StaticGroundCapacityError {
                capacity: self.capacity,
                occupied: self.slots.len(),
                requested,
            });
        };
        if final_count > self.capacity {
            return Err(StaticGroundCapacityError {
                capacity: self.capacity,
                occupied: self.slots.len(),
                requested,
            });
        }
        for source in sources {
            self.append_reserved(source);
        }
        Ok(())
    }

    fn append_reserved(&mut self, source: &StaticEnvironmentSource) {
        let first = self.slots.len();
        for (root_ordinal, root) in source.roots.iter().enumerate() {
            let mesh = &source.owner.asset().collision.meshes[root.mesh_index];
            // Reset8378 copies identity forward placement then derives this
            // cofactor inverse before the mesh's KD bounds are copied. The
            // native inverse's fourth lanes are not canonical identity lanes.
            let inverse =
                crate::world::inverse_collision_affine_reset(STATIC_GROUND_IDENTITY_TRANSFORM);
            let kd_bounds = mesh.clustered_mesh.kd_tree(&mesh.object_bytes).map(|tree| {
                (
                    std::array::from_fn(|axis| tree.bounding_box_min[axis]),
                    std::array::from_fn(|axis| tree.bounding_box_max[axis]),
                )
            });
            self.slots.push(StaticGroundSlot {
                triangle_decode: Arc::new(OnceLock::new()),
                cluster_decode: LeafClusterCache::default(),
                owner: Arc::clone(&source.owner),
                operation_index: source.operation_index,
                root_ordinal,
                collision_root_index: root.collision_root_index,
                mesh_index: root.mesh_index,
                kd_bounds,
                inverse,
            });
        }
        if self.warm_decodes {
            let owner = &source.owner;
            for slot in &self.slots[first..] {
                warm_decode(Arc::clone(owner), slot.mesh_index, Arc::clone(&slot.triangle_decode));
            }
        }
    }

    /// Removes each source root in forward order.
    ///
    /// Every root search selects the first pointer-equivalent occurrence and
    /// fills its hole from the final slot. A missing occurrence is a native
    /// no-op. Removal is deliberately source-based rather than token-based:
    /// deregistration reaches this path even after an unlock-suppressed add.
    pub fn remove(&mut self, source: &StaticEnvironmentSource) {
        for root in &source.roots {
            if let Some(index) = self.slots.iter().position(|slot| {
                Arc::ptr_eq(&slot.owner, &source.owner)
                    && slot.collision_root_index == root.collision_root_index
            }) {
                self.slots.swap_remove(index);
            }
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<StaticGroundSlotSnapshot> {
        self.slots
            .iter()
            .map(|slot| StaticGroundSlotSnapshot {
                operation_index: slot.operation_index,
                root_ordinal: slot.root_ordinal,
                collision_root_index: slot.collision_root_index,
                mesh_index: slot.mesh_index,
            })
            .collect()
    }

    /// Runs the static-ground half of the native trajectory fat-line query.
    ///
    /// Ground slots have identity placement and no collection octree, so the
    /// reference walks the current slot array in order, culls each mesh's
    /// local KD bounds against the expanded line bounds, and retains only a
    /// strictly nearer triangle hit.
    ///
    /// # Errors
    ///
    /// Returns the first collision-format error encountered while reading a
    /// published mesh's KD tree, clusters, units, or vertices.
    pub fn trajectory_sweep_sphere_segment(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<TrajectorySweepHit>, CollisionError> {
        self.trajectory_sweep_sphere_segment_with_cache(start, end, radius, |mesh, cache| {
            mesh_trajectory_hits_in_native_order(mesh, cache, start, end, radius)
        })
    }

    /// Collects the static-ground triangles overlapping one trajectory query
    /// box in native slot, KD-range, unit, and primitive order.
    ///
    /// The reference functor owns a fixed 64-triangle cache. Filling entry 64
    /// does not stop traversal; the next accepted triangle observes the full
    /// cache and terminates the collection without replacing an entry.
    /// Native traversal and cache order are preserved for valid mesh data. The
    /// overlap predicate is the recovered 13-axis source-level SAT; exact
    /// Xenon fused projection rounding at a separator boundary and exceptional
    /// VMX values remain outside this scalar host boundary. Eager safe-host
    /// decoding can also report malformed unvisited data.
    ///
    /// # Errors
    ///
    /// Returns the first collision-format error encountered while reading an
    /// admitted published mesh. The returned vector is failure-atomic.
    pub fn trajectory_ground_triangles_overlapping_aabb(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
    ) -> Result<Vec<TrajectoryGroundTriangle>, CollisionError> {
        self.trajectory_ground_triangles_overlapping_aabb_with_cache(
            minimum,
            maximum,
            mesh_trajectory_triangles_in_native_order,
        )
    }

    #[cfg(test)]
    fn trajectory_ground_triangles_overlapping_aabb_with(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        mut query: impl FnMut(
            &crate::world::CollisionMesh,
            [f32; 3],
            [f32; 3],
        ) -> Result<Vec<MeshTriangle>, CollisionError>,
    ) -> Result<Vec<TrajectoryGroundTriangle>, CollisionError> {
        self.trajectory_ground_triangles_overlapping_aabb_with_cache(
            minimum,
            maximum,
            |mesh, _, minimum, maximum| query(mesh, minimum, maximum),
        )
    }
    #[cfg(test)]
    fn trajectory_sweep_sphere_segment_with(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
        mut query: impl FnMut(
            &crate::world::CollisionMesh,
        ) -> Result<Vec<MeshTriangleHit>, CollisionError>,
    ) -> Result<Option<TrajectorySweepHit>, CollisionError> {
        self.trajectory_sweep_sphere_segment_with_cache(start, end, radius, |mesh, _| query(mesh))
    }
    fn trajectory_ground_triangles_overlapping_aabb_with_cache(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        mut query_mesh: impl FnMut(
            &crate::world::CollisionMesh,
            &TriangleDecodeCache,
            [f32; 3],
            [f32; 3],
        ) -> Result<Vec<MeshTriangle>, CollisionError>,
    ) -> Result<Vec<TrajectoryGroundTriangle>, CollisionError> {
        let (model_minimum, model_maximum) = identity_transform_aabb(minimum, maximum);
        let mut result = Vec::with_capacity(TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY);

        for slot in &self.slots {
            let mesh = &slot.owner.asset().collision.meshes[slot.mesh_index];
            let &(tree_minimum, tree_maximum) = slot.kd_bounds.as_ref().map_err(Clone::clone)?;
            let overlaps = (0..3).all(|axis| {
                !(tree_minimum[axis] > model_maximum[axis]
                    || model_minimum[axis] > tree_maximum[axis])
            });
            if !overlaps {
                continue;
            }

            for triangle in query_mesh(mesh, &slot.triangle_decode, model_minimum, model_maximum)? {
                if !triangle_aabb_overlap(minimum, maximum, triangle.vertices) {
                    continue;
                }
                if result.len() >= TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY {
                    return Ok(result);
                }
                result.push(TrajectoryGroundTriangle {
                    vertices: triangle.vertices,
                    surface_id: triangle.surface_id.map_or(0, u32::from),
                });
            }
        }
        Ok(result)
    }

    fn trajectory_sweep_sphere_segment_with_cache(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
        mut sweep_mesh: impl FnMut(
            &crate::world::CollisionMesh,
            &TriangleDecodeCache,
        ) -> Result<Vec<MeshTriangleHit>, CollisionError>,
    ) -> Result<Option<TrajectorySweepHit>, CollisionError> {
        let delta: [f32; 3] = std::array::from_fn(|axis| end[axis] - start[axis]);
        if !trajectory_line_has_extent(delta) {
            return Ok(None);
        }

        let line_minimum: [f32; 3] =
            std::array::from_fn(|axis| start[axis].min(end[axis]) - radius);
        let line_maximum: [f32; 3] =
            std::array::from_fn(|axis| start[axis].max(end[axis]) + radius);
        let (line_minimum, line_maximum) = identity_transform_aabb(line_minimum, line_maximum);
        let mut nearest_parameter = f32::MAX;
        let mut nearest = None;

        for slot in &self.slots {
            let mesh = &slot.owner.asset().collision.meshes[slot.mesh_index];
            let &(tree_minimum, tree_maximum) = slot.kd_bounds.as_ref().map_err(Clone::clone)?;
            let overlaps = (0..3).all(|axis| {
                !(tree_minimum[axis] > line_maximum[axis]
                    || line_minimum[axis] > tree_maximum[axis])
            });
            if !overlaps {
                continue;
            }

            for hit in sweep_mesh(mesh, &slot.triangle_decode)? {
                let parameter = native_clamp_line_parameter(hit.intersection.line_parameter);
                if parameter < nearest_parameter {
                    nearest_parameter = parameter;
                    nearest = Some(TrajectorySweepHit {
                        position: hit.intersection.position,
                        normal: triangle_normal(hit.triangle.vertices),
                        surface_id: hit.triangle.surface_id.map_or(0, u32::from),
                    });
                }
            }
        }
        Ok(nearest)
    }
}

#[allow(clippy::manual_midpoint)] // Native order is `(minimum + maximum) * 0.5`.
fn identity_transform_aabb(minimum: [f32; 3], maximum: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    // The no-octree path still calls `AABBox::Transform` with the record's
    // inverse matrix. Ground records carry identity, but the reference first
    // reconstructs center and half-extents, so retain its finite rounding.
    let center: [f32; 3] = std::array::from_fn(|axis| (minimum[axis] + maximum[axis]) * 0.5);
    let half: [f32; 3] = std::array::from_fn(|axis| (maximum[axis] - minimum[axis]) * 0.5);
    (
        std::array::from_fn(|axis| center[axis] - half[axis]),
        std::array::from_fn(|axis| center[axis] + half[axis]),
    )
}

pub(crate) fn trajectory_line_has_extent(delta: [f32; 3]) -> bool {
    delta
        .into_iter()
        .any(|component| component.abs() > TRAJECTORY_LINE_MINIMUM_DELTA)
}

pub(crate) fn native_clamp_line_parameter(parameter: f32) -> f32 {
    let lower = if -parameter >= 0.0 { 0.0 } else { parameter };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

type WarmDecode = (Arc<SimulationStaticArenaPlan>, usize, Arc<TriangleDecodeCache>);

/// Queues one mesh for the shared background decoder, which works through
/// meshes in the order they were added. Without the decoder thread, the first
/// query decodes as before.
fn warm_decode(owner: Arc<SimulationStaticArenaPlan>, mesh_index: usize, cache: Arc<TriangleDecodeCache>) {
    static DECODER: OnceLock<Option<std::sync::Mutex<std::sync::mpsc::Sender<WarmDecode>>>> =
        OnceLock::new();
    let decoder = DECODER.get_or_init(|| {
        let (sender, receiver) = std::sync::mpsc::channel::<WarmDecode>();
        std::thread::Builder::new()
            .name("sk8 ground decode".to_owned())
            .spawn(move || {
                for (owner, mesh_index, cache) in receiver {
                    let mesh = &owner.asset().collision.meshes[mesh_index];
                    cache.get_or_init(|| {
                        PreparedMeshTriangles::new(&mesh.clustered_mesh, &mesh.object_bytes)
                    });
                }
            })
            .ok()
            .map(|_| std::sync::Mutex::new(sender))
    });
    if let Some(sender) = decoder.as_ref().and_then(|sender| sender.lock().ok()) {
        let _ = sender.send((owner, mesh_index, cache));
    }
}

fn mesh_trajectory_hits_in_native_order(
    mesh: &crate::world::CollisionMesh,
    cache: &TriangleDecodeCache,
    start: [f32; 3],
    end: [f32; 3],
    radius: f32,
) -> Result<Vec<MeshTriangleHit>, CollisionError> {
    let tree = mesh.clustered_mesh.kd_tree(&mesh.object_bytes)?;
    let mut query = tree.line_range_query(&mesh.object_bytes, start, end, radius)?;
    let mut ranges = Vec::new();
    while let Some(range) = query.next_range()? {
        ranges.push(range);
    }
    let triangles = cache
        .get_or_init(|| PreparedMeshTriangles::new(&mesh.clustered_mesh, &mesh.object_bytes))
        .as_ref()
        .map_err(Clone::clone)?
        .triangles_for_leaf_ranges(&ranges)?;
    let delta = std::array::from_fn(|axis| end[axis] - start[axis]);
    Ok(triangles
        .into_iter()
        .filter_map(|triangle| {
            sphere_segment_triangle_intersection(start, delta, radius, triangle.vertices).map(
                |intersection| MeshTriangleHit {
                    triangle,
                    intersection,
                },
            )
        })
        .collect())
}

fn mesh_trajectory_triangles_in_native_order(
    mesh: &crate::world::CollisionMesh,
    cache: &TriangleDecodeCache,
    minimum: [f32; 3],
    maximum: [f32; 3],
) -> Result<Vec<MeshTriangle>, CollisionError> {
    let tree = mesh.clustered_mesh.kd_tree(&mesh.object_bytes)?;
    let mut query = tree.aabb_range_query(&mesh.object_bytes, minimum, maximum)?;
    let mut ranges = Vec::new();
    while let Some(range) = query.next_range()? {
        ranges.push(range);
    }

    // Safe-host boundary: the range vector above exhausts the complete KD
    // iterator, then this decoder eagerly validates all serialized clusters
    // and units before expanding the visited native-ordered ranges. Native
    // code interleaves range and unit decoding and may stop at a 65th accepted
    // triangle, so a malformed later KD branch or unvisited cluster/unit can
    // have different error timing.
    cache
        .get_or_init(|| PreparedMeshTriangles::new(&mesh.clustered_mesh, &mesh.object_bytes))
        .as_ref()
        .map_err(Clone::clone)?
        .triangles_for_leaf_ranges(&ranges)
}

pub(crate) fn triangle_normal(vertices: [[f32; 3]; 3]) -> [f32; 3] {
    // `UnitGetOverlappingGPInstances` recomputes this face normal from the
    // forward-transformed vertices. Ground placement is identity. Xenon
    // `vrsqrte`/Newton-refinement bit parity remains a hardware boundary.
    let first: [f32; 3] = std::array::from_fn(|axis| vertices[1][axis] - vertices[0][axis]);
    let second: [f32; 3] = std::array::from_fn(|axis| vertices[2][axis] - vertices[0][axis]);
    let normal = [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ];
    let squared_length = normal.into_iter().map(|value| value * value).sum::<f32>();
    if squared_length > 0.0 {
        let inverse_length = squared_length.sqrt().recip();
        normal.map(|value| value * inverse_length)
    } else {
        [0.0; 3]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
        world::{CollisionArena, CollisionInstance, CollisionMesh, CollisionModel, CollisionRoot},
    };
    use skate2_formats::{
        collision::{ClusteredMesh, MeshTriangle, MeshTriangleHit, TriangleHit, Volume},
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };

    fn volume() -> Volume {
        Volume {
            transform: [0.0; 16],
            vtable_offset: 0,
            shape_words: [0; 3],
            radius: 0.0,
            group_id: 0,
            surface_id: u32::MAX,
            flags: 1,
        }
    }

    fn mesh(dictionary_index: u32) -> CollisionMesh {
        CollisionMesh {
            dictionary_index,
            bounding_box_min: [0.0; 3],
            bounding_box_max: [0.0; 3],
            triangles: Vec::new(),
            clustered_mesh: ClusteredMesh::parse(&[0; 96]).unwrap(),
            object_bytes: Vec::new(),
        }
    }

    fn query_mesh(dictionary_index: u32, minimum: [f32; 3], maximum: [f32; 3]) -> CollisionMesh {
        const KD_OFFSET: usize = 0x60;
        let mut bytes = vec![0_u8; KD_OFFSET + 48];
        bytes[48..52].copy_from_slice(&u32::try_from(KD_OFFSET).unwrap().to_be_bytes());
        for (base, values) in [
            (0, [minimum[0], minimum[1], minimum[2], 0.0]),
            (16, [maximum[0], maximum[1], maximum[2], 0.0]),
            (KD_OFFSET + 16, [minimum[0], minimum[1], minimum[2], 0.0]),
            (KD_OFFSET + 32, [maximum[0], maximum[1], maximum[2], 0.0]),
        ] {
            for (index, value) in values.into_iter().enumerate() {
                let offset = base + index * 4;
                bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
            }
        }
        CollisionMesh {
            dictionary_index,
            bounding_box_min: minimum,
            bounding_box_max: maximum,
            triangles: Vec::new(),
            clustered_mesh: ClusteredMesh::parse(&bytes).unwrap(),
            object_bytes: bytes,
        }
    }

    fn triangle_query_mesh(dictionary_index: u32, surface_id: u16) -> CollisionMesh {
        const CLUSTER_TABLE_OFFSET: usize = 0x60;
        const KD_OFFSET: usize = 0x80;
        const CLUSTER_OFFSET: usize = 0xc0;
        const CLUSTER_SIZE: usize = 16 + 3 * 16 + 6;
        let mut bytes = vec![0_u8; CLUSTER_OFFSET + CLUSTER_SIZE];
        {
            let mut write_words = |start: usize, values: [f32; 4]| {
                for (index, value) in values.into_iter().enumerate() {
                    let offset = start + index * 4;
                    bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
                }
            };
            write_words(0, [-2.0, -2.0, -1.0, 0.0]);
            write_words(16, [2.0, 2.0, 1.0, 0.0]);
            write_words(KD_OFFSET + 16, [-2.0, -2.0, -1.0, 0.0]);
            write_words(KD_OFFSET + 32, [2.0, 2.0, 1.0, 0.0]);
            for (index, vertex) in [
                [-1.0, -1.0, 0.0, 0.0],
                [1.0, -1.0, 0.0, 0.0],
                [-1.0, 1.0, 0.0, 0.0],
            ]
            .into_iter()
            .enumerate()
            {
                write_words(CLUSTER_OFFSET + 16 + index * 16, vertex);
            }
        }

        bytes[40..44].copy_from_slice(&1_u32.to_be_bytes());
        bytes[48..52].copy_from_slice(&u32::try_from(KD_OFFSET).unwrap().to_be_bytes());
        bytes[52..56].copy_from_slice(&u32::try_from(CLUSTER_TABLE_OFFSET).unwrap().to_be_bytes());
        bytes[56..60].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        bytes[63] = 2;
        bytes[64..68].copy_from_slice(&1_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&1_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&1_u32.to_be_bytes());
        bytes[76..80].copy_from_slice(&1_u32.to_be_bytes());
        let allocation_size = u32::try_from(bytes.len()).unwrap();
        bytes[80..84].copy_from_slice(&allocation_size.to_be_bytes());
        bytes[86..88].copy_from_slice(&surface_id.to_be_bytes());
        bytes[CLUSTER_TABLE_OFFSET..CLUSTER_TABLE_OFFSET + 4]
            .copy_from_slice(&u32::try_from(CLUSTER_OFFSET).unwrap().to_be_bytes());
        bytes[KD_OFFSET + 8..KD_OFFSET + 12].copy_from_slice(&1_u32.to_be_bytes());

        let cluster = &mut bytes[CLUSTER_OFFSET..];
        cluster[0..2].copy_from_slice(&1_u16.to_be_bytes());
        cluster[2..4].copy_from_slice(&6_u16.to_be_bytes());
        cluster[4..6].copy_from_slice(&3_u16.to_be_bytes());
        cluster[6..8].copy_from_slice(&3_u16.to_be_bytes());
        cluster[8..10].copy_from_slice(&u16::try_from(CLUSTER_SIZE).unwrap().to_be_bytes());
        cluster[10] = 3;
        cluster[12] = 0;
        cluster[64..70].copy_from_slice(&[0x81, 0, 1, 2, 0x34, 0x12]);

        CollisionMesh {
            dictionary_index,
            bounding_box_min: [-2.0, -2.0, -1.0],
            bounding_box_max: [2.0, 2.0, 1.0],
            triangles: Vec::new(),
            clustered_mesh: ClusteredMesh::parse(&bytes).unwrap(),
            object_bytes: bytes,
        }
    }

    fn query_source(asset_id: u64, mesh: CollisionMesh) -> StaticEnvironmentSource {
        let owner = plan(
            asset_id,
            vec![CollisionRoot {
                dictionary_index: 0,
                volume: volume(),
                aggregate: Some(CollisionAggregateRef::Mesh(0)),
            }],
            vec![mesh],
            vec![CollisionModel {
                dictionary_index: 0,
                bounding_root: None,
                collision_roots: vec![0],
                assembly_object_id: None,
            }],
            vec![CollisionInstance {
                guid: asset_id,
                local_guid: asset_id,
                name: String::new(),
                transform: [0.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                collision_model: Some(0),
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 0,
                instance_index: 0,
            }],
        );
        StaticEnvironmentSource::from_operation(owner, 0).unwrap()
    }

    fn query_hit(parameter: f32, surface_id: u16) -> MeshTriangleHit {
        MeshTriangleHit {
            triangle: MeshTriangle {
                cluster_index: 0,
                unit_stream_offset: 0,
                vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                group_id: None,
                surface_id: Some(surface_id),
            },
            intersection: TriangleHit {
                line_parameter: parameter,
                position: [0.0, 0.0, parameter],
                normal: [0.0, 0.0, -1.0],
                barycentric: [1.0, 0.0, 0.0],
            },
        }
    }

    fn query_triangle(surface_id: Option<u16>) -> MeshTriangle {
        MeshTriangle {
            cluster_index: 0,
            unit_stream_offset: 0,
            vertices: [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, 1.0, 0.0]],
            group_id: None,
            surface_id,
        }
    }

    fn plan(
        asset_id: u64,
        roots: Vec<CollisionRoot>,
        meshes: Vec<CollisionMesh>,
        models: Vec<CollisionModel>,
        instances: Vec<CollisionInstance>,
        operations: Vec<SimulationStaticOperation>,
    ) -> Arc<SimulationStaticArenaPlan> {
        let descriptor = ResourceDescriptor {
            size: 0,
            alignment: 1,
        };
        let arena = Arena {
            file_header: FileHeader {
                magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                is_big_endian: true,
                pointer_size_bits: 32,
                pointer_alignment: 4,
                unused: 0,
                major_version: *b"454\0",
                minor_version: *b"000\0",
                build_number: 0,
            },
            id: 9,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 1,
            virtual_size: 0,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor; 5],
            resources_used: [descriptor; 5],
            resource_offsets: [0; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            SimulationCollisionAsset {
                decoded: DecodedArenaAsset::from_parts_for_test(
                    asset_id,
                    WORLD_STREAM_ARENA_ASSET_TYPE,
                    ArenaResources {
                        resources: std::array::from_fn(|_| Vec::new()),
                    },
                    arena,
                ),
                collision: CollisionArena {
                    id: 9,
                    grind_assets: Vec::new(),
                    aggregate_roots: roots,
                    primitive_sets: Vec::new(),
                    simple_primitive_sets: Vec::new(),
                    meshes,
                    collision_models: models,
                    instances,
                },
            },
            operations,
        ))
    }

    pub(crate) fn environment_plan_for_test(
        asset_id: u64,
        local_guid: u64,
        root_indexes: &[usize],
    ) -> Arc<SimulationStaticArenaPlan> {
        let roots = root_indexes
            .iter()
            .enumerate()
            .map(|(index, &mesh_index)| CollisionRoot {
                dictionary_index: u32::try_from(index).unwrap(),
                volume: volume(),
                aggregate: Some(CollisionAggregateRef::Mesh(mesh_index)),
            })
            .collect::<Vec<_>>();
        let meshes = (0..root_indexes.len())
            .map(|index| mesh(u32::try_from(index).unwrap()))
            .collect();
        plan(
            asset_id,
            roots,
            meshes,
            vec![CollisionModel {
                dictionary_index: 4,
                bounding_root: None,
                collision_roots: (0..root_indexes.len()).collect(),
                assembly_object_id: None,
            }],
            vec![CollisionInstance {
                guid: 0x1111,
                local_guid,
                name: "environment".to_owned(),
                transform: [7.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [1.0; 3],
                collision_model: Some(0),
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 1,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            }],
        )
    }

    fn environment_plan(asset_id: u64, root_indexes: &[usize]) -> Arc<SimulationStaticArenaPlan> {
        environment_plan_for_test(asset_id, 0x2222, root_indexes)
    }

    #[test]
    fn assembly_ground_view_retains_backing_meshes_and_current_swap_remove_order() {
        let owners = [1, 2, 3].map(|id| environment_plan(id, &[0]));
        let sources = owners
            .each_ref()
            .map(|owner| StaticEnvironmentSource::from_operation(Arc::clone(owner), 0).unwrap());
        let mut pool = StaticGroundPool::try_new(3).unwrap();
        for source in &sources {
            pool.add(source).unwrap();
        }
        for (view, owner) in pool.assembly_ground_meshes().zip(&owners) {
            assert!(std::ptr::eq(
                view.mesh,
                &raw const owner.asset().collision.meshes[0]
            ));
            // This fixture intentionally has no KD bytes: merely borrowing
            // the view must retain, not eagerly surface, the saved error.
            assert!(view.kd_bounds.is_err());
        }
        pool.remove(&sources[0]);
        let remaining = pool.assembly_ground_meshes().collect::<Vec<_>>();
        assert_eq!(remaining.len(), 2);
        assert!(std::ptr::eq(
            remaining[0].mesh,
            &raw const owners[2].asset().collision.meshes[0]
        ));
        assert!(std::ptr::eq(
            remaining[1].mesh,
            &raw const owners[1].asset().collision.meshes[0]
        ));
    }

    #[test]
    fn source_validates_kind_indexes_and_supported_mesh_roots() {
        let owner = environment_plan(1, &[0]);
        assert_eq!(
            StaticEnvironmentSource::from_operation(Arc::clone(&owner), 1).unwrap_err(),
            StaticGroundSourceError::MissingOperation { operation_index: 1 }
        );

        let grind = plan(
            2,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![SimulationStaticOperation::Grind {
                source_toc_index: 0,
                source_dictionary_index: 0,
                grind_asset_index: 0,
            }],
        );
        assert_eq!(
            StaticEnvironmentSource::from_operation(grind, 0).unwrap_err(),
            StaticGroundSourceError::NotEnvironment { operation_index: 0 }
        );

        let unsupported = plan(
            3,
            vec![CollisionRoot {
                dictionary_index: 2,
                volume: volume(),
                aggregate: None,
            }],
            Vec::new(),
            vec![CollisionModel {
                dictionary_index: 4,
                bounding_root: None,
                collision_roots: vec![0],
                assembly_object_id: None,
            }],
            vec![CollisionInstance {
                guid: 1,
                local_guid: 2,
                name: String::new(),
                transform: [0.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                collision_model: Some(0),
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            }],
        );
        assert!(matches!(
            StaticEnvironmentSource::from_operation(unsupported, 0),
            Err(StaticGroundSourceError::UnsupportedCollisionRoot { .. })
        ));
    }

    #[test]
    fn source_identity_tracks_backing_record_across_toc_and_dictionary_aliases() {
        let base = environment_plan(1, &[]);
        let asset = base.asset().clone();
        let operation = |source_toc_index, source_dictionary_index, source_record_start| {
            SimulationStaticOperation::Environment {
                source_toc_index,
                source_dictionary_index,
                source_inline_index: 0,
                source_resource_index: 2,
                source_record_start,
                instance_index: 0,
            }
        };
        let owner = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            asset,
            vec![
                operation(0, 3, 0x120),
                operation(1, 3, 0x120),
                operation(2, 9, 0x120),
                operation(3, 3, 0x1c0),
            ],
        ));
        let direct = StaticEnvironmentSource::from_operation(Arc::clone(&owner), 0).unwrap();
        let repeated_toc = StaticEnvironmentSource::from_operation(Arc::clone(&owner), 1).unwrap();
        let dictionary_alias =
            StaticEnvironmentSource::from_operation(Arc::clone(&owner), 2).unwrap();
        let distinct_record = StaticEnvironmentSource::from_operation(owner, 3).unwrap();

        assert!(direct.is_same_backing_record(&repeated_toc));
        assert!(direct.is_same_backing_record(&dictionary_alias));
        assert!(!direct.is_same_backing_record(&distinct_record));
    }

    #[test]
    fn source_reports_missing_and_assembly_models() {
        let missing_model = plan(
            4,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![CollisionInstance {
                guid: 1,
                local_guid: 2,
                name: String::new(),
                transform: [0.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                collision_model: Some(0),
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            }],
        );
        assert_eq!(
            StaticEnvironmentSource::from_operation(missing_model, 0).unwrap_err(),
            StaticGroundSourceError::MissingCollisionModel {
                operation_index: 0,
                model_index: 0,
            }
        );

        let assembly = plan(
            5,
            Vec::new(),
            Vec::new(),
            vec![CollisionModel {
                dictionary_index: 4,
                bounding_root: None,
                collision_roots: Vec::new(),
                assembly_object_id: Some(0x1234),
            }],
            vec![CollisionInstance {
                guid: 1,
                local_guid: 2,
                name: String::new(),
                transform: [0.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                collision_model: Some(0),
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            }],
        );
        assert_eq!(
            StaticEnvironmentSource::from_operation(assembly, 0).unwrap_err(),
            StaticGroundSourceError::AssemblyModel {
                operation_index: 0,
                model_index: 0,
                object_id: 0x1234,
            }
        );
    }

    #[test]
    fn source_reports_missing_root_and_mesh() {
        let missing_root = plan(
            6,
            Vec::new(),
            Vec::new(),
            vec![CollisionModel {
                dictionary_index: 4,
                bounding_root: None,
                collision_roots: vec![3],
                assembly_object_id: None,
            }],
            vec![CollisionInstance {
                guid: 1,
                local_guid: 2,
                name: String::new(),
                transform: [0.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                collision_model: Some(0),
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            }],
        );
        assert_eq!(
            StaticEnvironmentSource::from_operation(missing_root, 0).unwrap_err(),
            StaticGroundSourceError::MissingCollisionRoot {
                operation_index: 0,
                root_ordinal: 0,
                collision_root_index: 3,
            }
        );

        let missing_mesh = plan(
            7,
            vec![CollisionRoot {
                dictionary_index: 2,
                volume: volume(),
                aggregate: Some(CollisionAggregateRef::Mesh(3)),
            }],
            Vec::new(),
            vec![CollisionModel {
                dictionary_index: 4,
                bounding_root: None,
                collision_roots: vec![0],
                assembly_object_id: None,
            }],
            vec![CollisionInstance {
                guid: 1,
                local_guid: 2,
                name: String::new(),
                transform: [0.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                collision_model: Some(0),
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            }],
        );
        assert_eq!(
            StaticEnvironmentSource::from_operation(missing_mesh, 0).unwrap_err(),
            StaticGroundSourceError::MissingMesh {
                operation_index: 0,
                root_ordinal: 0,
                mesh_index: 3,
            }
        );
    }

    #[test]
    fn add_is_forward_identity_and_capacity_failure_is_atomic() {
        let source =
            StaticEnvironmentSource::from_operation(environment_plan(1, &[0, 1]), 0).unwrap();
        assert_eq!(source.guid(), 0x1111);
        assert_eq!(source.local_guid(), 0x2222);
        assert_eq!(source.root_count(), 2);
        assert_eq!(STATIC_GROUND_NATIVE_CAPACITY, 128);
        assert_eq!(
            STATIC_GROUND_IDENTITY_TRANSFORM.map(f32::to_bits),
            [
                0x3f80_0000,
                0,
                0,
                0,
                0,
                0x3f80_0000,
                0,
                0,
                0,
                0,
                0x3f80_0000,
                0,
                0,
                0,
                0,
                0,
            ]
        );
        assert_ne!(
            STATIC_GROUND_IDENTITY_TRANSFORM.map(f32::to_bits),
            source.owner.asset().collision.instances[0]
                .transform
                .map(f32::to_bits)
        );

        let mut too_small = StaticGroundPool::try_new(1).unwrap();
        assert_eq!(
            too_small.add(&source),
            Err(StaticGroundCapacityError {
                capacity: 1,
                occupied: 0,
                requested: 2,
            })
        );
        assert_eq!(too_small.published_volume_count(), 0);

        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&source).unwrap();
        assert_eq!(
            pool.snapshot(),
            [
                StaticGroundSlotSnapshot {
                    operation_index: 0,
                    root_ordinal: 0,
                    collision_root_index: 0,
                    mesh_index: 0,
                },
                StaticGroundSlotSnapshot {
                    operation_index: 0,
                    root_ordinal: 1,
                    collision_root_index: 1,
                    mesh_index: 1,
                },
            ]
        );
    }

    #[test]
    fn repeated_add_and_forward_remove_use_pointer_first_swap_last() {
        let first_owner = environment_plan(1, &[0, 1]);
        let equal_but_distinct_owner = environment_plan(1, &[0, 1]);
        assert_eq!(*first_owner, *equal_but_distinct_owner);
        assert!(!Arc::ptr_eq(&first_owner, &equal_but_distinct_owner));
        let first = StaticEnvironmentSource::from_operation(Arc::clone(&first_owner), 0).unwrap();
        let distinct =
            StaticEnvironmentSource::from_operation(Arc::clone(&equal_but_distinct_owner), 0)
                .unwrap();
        let mut pool = StaticGroundPool::try_new(6).unwrap();
        pool.add(&first).unwrap();
        pool.add(&first).unwrap();
        pool.add(&distinct).unwrap();
        assert_eq!(pool.published_volume_count(), 6);

        pool.remove(&first);
        assert_eq!(pool.published_volume_count(), 4);
        assert_eq!(
            pool.slots
                .iter()
                .map(|slot| {
                    (
                        Arc::ptr_eq(&slot.owner, &first_owner),
                        Arc::ptr_eq(&slot.owner, &equal_but_distinct_owner),
                        slot.collision_root_index,
                    )
                })
                .collect::<Vec<_>>(),
            [
                (false, true, 1),
                (false, true, 0),
                (true, false, 0),
                (true, false, 1),
            ]
        );
        pool.remove(&first);
        assert_eq!(
            pool.slots
                .iter()
                .map(|slot| {
                    (
                        Arc::ptr_eq(&slot.owner, &equal_but_distinct_owner),
                        slot.collision_root_index,
                    )
                })
                .collect::<Vec<_>>(),
            [(true, 1), (true, 0)]
        );
        let before_missing_remove = pool.snapshot();
        pool.remove(&first);
        assert_eq!(pool.snapshot(), before_missing_remove);
    }

    #[test]
    fn add_all_capacity_failure_preserves_existing_order() {
        let existing =
            StaticEnvironmentSource::from_operation(environment_plan(1, &[0]), 0).unwrap();
        let first =
            StaticEnvironmentSource::from_operation(environment_plan(2, &[0, 1]), 0).unwrap();
        let second =
            StaticEnvironmentSource::from_operation(environment_plan(3, &[0, 1]), 0).unwrap();
        let mut pool = StaticGroundPool::try_new(4).unwrap();
        pool.add(&existing).unwrap();
        let before = pool.snapshot();

        assert_eq!(
            pool.add_all(&[first, second]),
            Err(StaticGroundCapacityError {
                capacity: 4,
                occupied: 1,
                requested: 4,
            })
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn suppressed_source_removal_uses_pointer_identity_not_add_tokens() {
        let owner = environment_plan(1, &[0]);
        let admitted = StaticEnvironmentSource::from_operation(Arc::clone(&owner), 0).unwrap();
        let suppressed = StaticEnvironmentSource::from_operation(owner, 0).unwrap();
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&admitted).unwrap();

        pool.remove(&suppressed);

        assert_eq!(pool.published_volume_count(), 0);
    }

    #[test]
    fn source_and_slots_retain_owner_until_final_removal() {
        let owner = environment_plan(1, &[0]);
        let weak = Arc::downgrade(&owner);
        let source = StaticEnvironmentSource::from_operation(Arc::clone(&owner), 0).unwrap();
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        drop(owner);
        assert!(weak.upgrade().is_some());
        pool.remove(&source);
        assert!(weak.upgrade().is_some());
        drop(source);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn absent_collision_model_is_a_valid_noop_source_and_remove_is_infallible() {
        let owner = plan(
            1,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![CollisionInstance {
                guid: 1,
                local_guid: 2,
                name: String::new(),
                transform: [0.0; 16],
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                collision_model: None,
            }],
            vec![SimulationStaticOperation::Environment {
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_resource_index: 0,
                source_record_start: 20,
                instance_index: 0,
            }],
        );
        let source = StaticEnvironmentSource::from_operation(owner, 0).unwrap();
        let mut pool = StaticGroundPool::try_new(0).unwrap();
        pool.add(&source).unwrap();
        pool.remove(&source);
        assert_eq!(pool.published_volume_count(), 0);
    }

    #[test]
    fn trajectory_sweep_rejects_a_non_overlapping_slot_before_the_mesh_provider() {
        let source = query_source(1, query_mesh(7, [10.0; 3], [11.0; 3]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        let mut calls = 0;

        let result = pool
            .trajectory_sweep_sphere_segment_with([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25, |_| {
                calls += 1;
                Ok(vec![query_hit(0.5, 7)])
            })
            .unwrap();

        assert_eq!(result, None);
        assert_eq!(calls, 0);
    }

    #[test]
    fn trajectory_sweep_retains_identity_aabb_transform_rounding_before_cull() {
        let lower = 16_777_216.0_f32;
        let upper = 16_777_218.0_f32;
        let (transformed_minimum, transformed_maximum) =
            identity_transform_aabb([lower, -1.0, 0.0], [upper, 1.0, 0.0]);
        assert_eq!(transformed_minimum[0].to_bits(), lower.to_bits() - 1);
        assert_eq!(transformed_maximum[0].to_bits(), lower.to_bits());

        let source = query_source(1, query_mesh(7, [upper, -2.0, -1.0], [upper, 2.0, 1.0]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        let mut calls = 0;

        let result = pool
            .trajectory_sweep_sphere_segment_with(
                [lower, -1.0, 0.0],
                [upper, 1.0, 0.0],
                0.0,
                |_| {
                    calls += 1;
                    Ok(vec![query_hit(0.5, 7)])
                },
            )
            .unwrap();

        assert_eq!(result, None);
        assert_eq!(calls, 0);
    }

    #[test]
    fn trajectory_sweep_uses_the_native_strict_short_line_threshold() {
        assert!(!trajectory_line_has_extent([0.0; 3]));
        assert!(!trajectory_line_has_extent([
            TRAJECTORY_LINE_MINIMUM_DELTA,
            0.0,
            0.0,
        ]));
        assert!(!trajectory_line_has_extent([
            f32::from_bits(0x3760_0000),
            0.0,
            0.0,
        ]));
        assert!(trajectory_line_has_extent([
            f32::from_bits(TRAJECTORY_LINE_MINIMUM_DELTA.to_bits() + 1),
            0.0,
            0.0,
        ]));
    }

    #[test]
    fn trajectory_sweep_native_clamp_maps_nan_and_signed_zero_before_ordering() {
        assert_eq!(
            native_clamp_line_parameter(f32::NAN).to_bits(),
            1.0_f32.to_bits()
        );
        assert_eq!(
            native_clamp_line_parameter(-0.0).to_bits(),
            0.0_f32.to_bits()
        );
        assert_eq!(
            native_clamp_line_parameter(0.0).to_bits(),
            0.0_f32.to_bits()
        );

        let source = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        let signed_zero = pool
            .trajectory_sweep_sphere_segment_with([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25, |_| {
                Ok(vec![query_hit(-0.0, 11), query_hit(0.0, 22)])
            })
            .unwrap()
            .unwrap();
        assert_eq!(signed_zero.surface_id, 11);

        let after_nan = pool
            .trajectory_sweep_sphere_segment_with([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25, |_| {
                Ok(vec![query_hit(f32::NAN, 11), query_hit(0.5, 22)])
            })
            .unwrap()
            .unwrap();
        assert_eq!(after_nan.surface_id, 22);
    }

    #[test]
    fn public_trajectory_sweep_short_line_reads_no_published_mesh_data() {
        let source = query_source(1, mesh(1));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();

        assert_eq!(
            pool.trajectory_sweep_sphere_segment(
                [0.0, 0.0, 0.0],
                [TRAJECTORY_LINE_MINIMUM_DELTA, 0.0, 0.0],
                0.25,
            ),
            Ok(None)
        );
    }

    #[test]
    fn trajectory_sweep_bbox_negated_separation_admits_unordered_axes() {
        let source = query_source(
            1,
            query_mesh(7, [f32::NAN, -1.0, -1.0], [f32::NAN, 1.0, 1.0]),
        );
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        let mut calls = 0;

        let result = pool
            .trajectory_sweep_sphere_segment_with([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25, |_| {
                calls += 1;
                Ok(vec![query_hit(0.5, 7)])
            })
            .unwrap();

        assert_eq!(calls, 1);
        assert_eq!(result.unwrap().surface_id, 7);
    }

    #[test]
    fn trajectory_sweep_replaces_only_with_a_strictly_nearer_slot_hit() {
        let first = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let second = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&first).unwrap();
        pool.add(&second).unwrap();

        let result = pool
            .trajectory_sweep_sphere_segment_with([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25, |mesh| {
                Ok(vec![if mesh.dictionary_index == 1 {
                    query_hit(0.75, 11)
                } else {
                    query_hit(0.25, 22)
                }])
            })
            .unwrap()
            .unwrap();

        assert_eq!(result.surface_id, 22);
        assert_eq!(result.position[2].to_bits(), 0.25_f32.to_bits());
    }

    #[test]
    fn public_trajectory_sweep_reads_the_published_clustered_mesh() {
        let source = query_source(1, triangle_query_mesh(1, 0x1234));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();

        let result = pool
            .trajectory_sweep_sphere_segment([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25)
            .unwrap()
            .unwrap();

        assert_eq!(result.surface_id, 0x1234);
        assert_eq!(result.normal.map(f32::to_bits), [0, 0, 0x3f80_0000]);
        assert!(result.position[2].abs() < 1.0e-6);
    }

    #[test]
    fn trajectory_sweep_keeps_the_first_equal_parameter_hit() {
        let first = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let second = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&first).unwrap();
        pool.add(&second).unwrap();

        let result = pool
            .trajectory_sweep_sphere_segment_with([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25, |mesh| {
                Ok(vec![query_hit(
                    0.5,
                    u16::try_from(mesh.dictionary_index).unwrap(),
                )])
            })
            .unwrap()
            .unwrap();

        assert_eq!(result.surface_id, 1);
    }

    #[test]
    fn trajectory_sweep_keeps_first_intra_mesh_equal_and_uses_gp_face_normal() {
        let source = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();

        let result = pool
            .trajectory_sweep_sphere_segment_with([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25, |_| {
                Ok(vec![query_hit(0.5, 11), query_hit(0.5, 22)])
            })
            .unwrap()
            .unwrap();

        assert_eq!(result.surface_id, 11);
        assert_eq!(result.normal.map(f32::to_bits), [0, 0, 0x3f80_0000]);
    }

    #[test]
    fn trajectory_sweep_observes_swap_remove_slot_order() {
        let removed = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let middle = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let last = query_source(3, query_mesh(3, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(3).unwrap();
        pool.add(&removed).unwrap();
        pool.add(&middle).unwrap();
        pool.add(&last).unwrap();
        pool.remove(&removed);
        let mut call_order = Vec::new();

        let result = pool
            .trajectory_sweep_sphere_segment_with([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25, |mesh| {
                call_order.push(mesh.dictionary_index);
                Ok(vec![query_hit(
                    0.5,
                    u16::try_from(mesh.dictionary_index).unwrap(),
                )])
            })
            .unwrap()
            .unwrap();

        assert_eq!(call_order, [3, 2]);
        assert_eq!(result.surface_id, 3);
    }

    #[test]
    fn trajectory_sweep_discards_an_earlier_hit_when_a_later_slot_errors() {
        let first = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let second = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&first).unwrap();
        pool.add(&second).unwrap();
        let before = pool.snapshot();
        let mut calls = 0;

        let result = pool.trajectory_sweep_sphere_segment_with(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, -2.0],
            0.25,
            |_| {
                calls += 1;
                if calls == 1 {
                    Ok(vec![query_hit(0.5, 11)])
                } else {
                    Err(CollisionError::NoClusters)
                }
            },
        );

        assert_eq!(result, Err(CollisionError::NoClusters));
        assert_eq!(calls, 2);
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn trajectory_ground_box_empty_and_broad_rejected_slots_return_no_triangles() {
        let empty = StaticGroundPool::try_new(0).unwrap();
        assert_eq!(
            empty.trajectory_ground_triangles_overlapping_aabb([-1.0; 3], [1.0; 3]),
            Ok(Vec::new())
        );

        let source = query_source(1, query_mesh(7, [10.0; 3], [11.0; 3]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        let mut calls = 0;
        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with([-1.0; 3], [1.0; 3], |_, _, _| {
                calls += 1;
                Ok(vec![query_triangle(Some(7))])
            })
            .unwrap();

        assert!(result.is_empty());
        assert_eq!(calls, 0);
    }

    #[test]
    fn trajectory_ground_box_uses_identity_transform_rounding_before_cull() {
        let lower = 16_777_216.0_f32;
        let upper = 16_777_218.0_f32;
        let source = query_source(1, query_mesh(7, [upper, -2.0, -1.0], [upper, 2.0, 1.0]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        let mut calls = 0;

        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with(
                [lower, -1.0, 0.0],
                [upper, 1.0, 0.0],
                |_, _, _| {
                    calls += 1;
                    Ok(vec![query_triangle(Some(7))])
                },
            )
            .unwrap();

        assert!(result.is_empty());
        assert_eq!(calls, 0);
    }

    #[test]
    fn trajectory_ground_box_final_sat_uses_the_original_world_bounds() {
        let lower = f32::from_bits(0x5efd_d828);
        let upper = f32::from_bits(0x5f03_d6f1);
        let world_minimum = [lower, -1.0, -1.0];
        let world_maximum = [upper, 1.0, 1.0];
        let (model_minimum, model_maximum) = identity_transform_aabb(world_minimum, world_maximum);
        assert_eq!(model_minimum[0].to_bits(), 0x5efd_d827);
        assert_eq!(model_maximum[0].to_bits(), 0x5f03_d6f0);
        let triangle = MeshTriangle {
            vertices: [
                [model_minimum[0], -0.5, 0.0],
                [model_minimum[0], 0.5, 0.0],
                [model_minimum[0], 0.0, 0.5],
            ],
            ..query_triangle(Some(7))
        };
        assert!(triangle_aabb_overlap(
            world_minimum,
            world_maximum,
            triangle.vertices
        ));
        assert!(!triangle_aabb_overlap(
            model_minimum,
            model_maximum,
            triangle.vertices
        ));

        let source = query_source(
            1,
            query_mesh(
                7,
                [model_minimum[0], -2.0, -2.0],
                [model_maximum[0], 2.0, 2.0],
            ),
        );
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with(
                world_minimum,
                world_maximum,
                |_, actual_minimum, actual_maximum| {
                    assert_eq!(
                        actual_minimum.map(f32::to_bits),
                        model_minimum.map(f32::to_bits)
                    );
                    assert_eq!(
                        actual_maximum.map(f32::to_bits),
                        model_maximum.map(f32::to_bits)
                    );
                    Ok(vec![triangle.clone()])
                },
            )
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].surface_id, 7);
    }

    #[test]
    fn trajectory_ground_box_cull_admits_touching_and_unordered_axes() {
        let touching = query_source(1, query_mesh(1, [1.0, -1.0, -1.0], [2.0, 1.0, 1.0]));
        let unordered = query_source(
            2,
            query_mesh(2, [f32::NAN, -1.0, -1.0], [f32::NAN, 1.0, 1.0]),
        );
        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&touching).unwrap();
        pool.add(&unordered).unwrap();
        let mut calls = Vec::new();

        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with([-1.0; 3], [1.0; 3], |mesh, _, _| {
                calls.push(mesh.dictionary_index);
                Ok(Vec::new())
            })
            .unwrap();

        assert!(result.is_empty());
        assert_eq!(calls, [1, 2]);
    }

    #[test]
    fn public_trajectory_ground_box_validates_an_invalid_box_only_for_an_admitted_mesh() {
        let empty = StaticGroundPool::try_new(0).unwrap();
        assert_eq!(
            empty.trajectory_ground_triangles_overlapping_aabb([1.0; 3], [-1.0; 3]),
            Ok(Vec::new())
        );

        let source = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        assert_eq!(
            pool.trajectory_ground_triangles_overlapping_aabb([1.0; 3], [-1.0; 3]),
            Err(CollisionError::InvalidBoundingBox)
        );
    }

    #[test]
    fn trajectory_ground_box_keeps_slot_order_and_observes_swap_remove() {
        let removed = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let middle = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let last = query_source(3, query_mesh(3, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(3).unwrap();
        pool.add(&removed).unwrap();
        pool.add(&middle).unwrap();
        pool.add(&last).unwrap();
        pool.remove(&removed);
        let mut calls = Vec::new();

        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with([-1.0; 3], [1.0; 3], |mesh, _, _| {
                calls.push(mesh.dictionary_index);
                Ok(vec![query_triangle(Some(
                    u16::try_from(mesh.dictionary_index).unwrap(),
                ))])
            })
            .unwrap();

        assert_eq!(calls, [3, 2]);
        assert_eq!(
            result
                .into_iter()
                .map(|triangle| triangle.surface_id)
                .collect::<Vec<_>>(),
            [3, 2]
        );
    }

    #[test]
    fn trajectory_ground_box_stores_the_64th_and_continues_without_a_65th() {
        let first = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let second = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&first).unwrap();
        pool.add(&second).unwrap();
        let mut calls = Vec::new();

        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with([-1.0; 3], [1.0; 3], |mesh, _, _| {
                calls.push(mesh.dictionary_index);
                if mesh.dictionary_index == 1 {
                    Ok(vec![
                        query_triangle(Some(1));
                        TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY
                    ])
                } else {
                    Ok(vec![MeshTriangle {
                        vertices: [[10.0, 10.0, 0.0], [11.0, 10.0, 0.0], [10.0, 11.0, 0.0]],
                        ..query_triangle(Some(2))
                    }])
                }
            })
            .unwrap();

        assert_eq!(result.len(), TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY);
        assert_eq!(calls, [1, 2]);
    }

    #[test]
    fn trajectory_ground_box_stops_on_the_65th_accepted_triangle() {
        let first = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let second = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let third = query_source(3, query_mesh(3, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(3).unwrap();
        pool.add(&first).unwrap();
        pool.add(&second).unwrap();
        pool.add(&third).unwrap();
        let mut calls = Vec::new();

        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with([-1.0; 3], [1.0; 3], |mesh, _, _| {
                calls.push(mesh.dictionary_index);
                Ok(if mesh.dictionary_index == 1 {
                    vec![query_triangle(Some(1)); TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY]
                } else {
                    vec![query_triangle(Some(
                        u16::try_from(mesh.dictionary_index).unwrap(),
                    ))]
                })
            })
            .unwrap();

        assert_eq!(result.len(), TRAJECTORY_GROUND_TRIANGLE_CACHE_CAPACITY);
        assert!(result.iter().all(|triangle| triangle.surface_id == 1));
        assert_eq!(calls, [1, 2]);
    }

    #[test]
    fn public_trajectory_ground_box_reads_native_order_and_surface_tags() {
        const CLUSTER_OFFSET: usize = 0xc0;

        let tagged = query_source(1, triangle_query_mesh(1, 0x1234));
        let mut absent_mesh = triangle_query_mesh(2, 0xabcd);
        absent_mesh.object_bytes[CLUSTER_OFFSET + 2..CLUSTER_OFFSET + 4]
            .copy_from_slice(&4_u16.to_be_bytes());
        absent_mesh.object_bytes[CLUSTER_OFFSET + 64] = 0x01;
        let absent = query_source(2, absent_mesh);
        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&tagged).unwrap();
        pool.add(&absent).unwrap();

        let result = pool
            .trajectory_ground_triangles_overlapping_aabb([-0.25, -0.25, -0.1], [0.25, 0.25, 0.1])
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].surface_id, 0x1234);
        assert_eq!(result[1].surface_id, 0);
        assert_eq!(
            result[0].vertices,
            [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, 1.0, 0.0],]
        );
    }

    #[test]
    fn trajectory_ground_box_exact_sat_rejects_a_broad_phase_false_positive() {
        let source = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();

        let result = pool
            .trajectory_ground_triangles_overlapping_aabb_with([-0.25; 3], [0.25; 3], |_, _, _| {
                Ok(vec![MeshTriangle {
                    vertices: [[1.0, 1.0, 0.0], [2.0, 1.0, 0.0], [1.0, 2.0, 0.0]],
                    ..query_triangle(Some(7))
                }])
            })
            .unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn trajectory_ground_box_discards_earlier_triangles_when_a_later_slot_errors() {
        let first = query_source(1, query_mesh(1, [-2.0; 3], [2.0; 3]));
        let second = query_source(2, query_mesh(2, [-2.0; 3], [2.0; 3]));
        let mut pool = StaticGroundPool::try_new(2).unwrap();
        pool.add(&first).unwrap();
        pool.add(&second).unwrap();
        let before = pool.snapshot();
        let mut calls = 0;

        let result = pool.trajectory_ground_triangles_overlapping_aabb_with(
            [-1.0; 3],
            [1.0; 3],
            |_, _, _| {
                calls += 1;
                if calls == 1 {
                    Ok(vec![query_triangle(Some(7))])
                } else {
                    Err(CollisionError::NoClusters)
                }
            },
        );

        assert_eq!(result, Err(CollisionError::NoClusters));
        assert_eq!(calls, 2);
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn prepared_triangles_preserve_duplicate_range_order_and_selected_errors() {
        use skate2_formats::collision::KdLeafRange;
        let mesh = triangle_query_mesh(1, 0x1234);
        let prepared =
            PreparedMeshTriangles::new(&mesh.clustered_mesh, &mesh.object_bytes).unwrap();
        let range = KdLeafRange {
            encoded_start: 0,
            unit_count: 1,
        };
        let ranges = [range, range];
        let baseline = mesh
            .clustered_mesh
            .triangles_for_leaf_ranges(&mesh.object_bytes, &ranges)
            .unwrap();
        assert_eq!(
            prepared.triangles_for_leaf_ranges(&ranges).unwrap(),
            baseline
        );
        assert_eq!(baseline.len(), 2);
        let bad = [KdLeafRange {
            encoded_start: 1,
            unit_count: 1,
        }];
        assert_eq!(
            prepared.triangles_for_leaf_ranges(&bad),
            mesh.clustered_mesh
                .triangles_for_leaf_ranges(&mesh.object_bytes, &bad)
        );
    }

    #[test]
    fn triangle_decode_cache_is_lazy_shared_and_drops_with_slot() {
        let source = query_source(1, triangle_query_mesh(1, 0x1234));
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        assert!(pool.slots[0].triangle_decode.get().is_none());
        assert!(
            pool.trajectory_sweep_sphere_segment([20.0; 3], [21.0; 3], 0.1)
                .unwrap()
                .is_none()
        );
        assert!(pool.slots[0].triangle_decode.get().is_none());
        let first = pool
            .trajectory_sweep_sphere_segment([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25)
            .unwrap();
        let cached = pool.slots[0]
            .triangle_decode
            .get()
            .unwrap()
            .as_ref()
            .unwrap() as *const _;
        let second = pool
            .trajectory_sweep_sphere_segment([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(
            cached,
            pool.slots[0]
                .triangle_decode
                .get()
                .unwrap()
                .as_ref()
                .unwrap() as *const _
        );
        pool.remove(&source);
        pool.add(&source).unwrap();
        assert!(pool.slots[0].triangle_decode.get().is_none());
    }

    #[test]
    fn prepared_decode_keeps_malformed_unit_error_even_for_empty_ranges() {
        let mut mesh = triangle_query_mesh(1, 0x1234);
        mesh.object_bytes[0xc0 + 64] = 0x0f;
        let baseline = mesh
            .clustered_mesh
            .triangles_for_leaf_ranges(&mesh.object_bytes, &[])
            .unwrap_err();
        assert_eq!(
            PreparedMeshTriangles::new(&mesh.clustered_mesh, &mesh.object_bytes).unwrap_err(),
            baseline
        );
        let source = query_source(1, mesh);
        let mut pool = StaticGroundPool::try_new(1).unwrap();
        pool.add(&source).unwrap();
        for _ in 0..2 {
            assert_eq!(
                pool.trajectory_sweep_sphere_segment([0.0, 0.0, 2.0], [0.0, 0.0, -2.0], 0.25)
                    .unwrap_err(),
                baseline
            );
        }
        assert!(pool.slots[0].triangle_decode.get().unwrap().is_err());
    }
}
