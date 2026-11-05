//! Read-only host geometry for the alternate simulation, separate from native
//! trajectory queries and their 64-triangle result capacity.
use super::{
    StaticGroundPool, StaticGroundSlotSnapshot, mesh_trajectory_triangles_in_native_order,
};
use crate::{
    simulation_assembly_ground_triangle_topology::build_old_assembly_unit_features,
    simulation_static_arena_plan::SimulationStaticArenaPlan,
};
use skate2_formats::collision::{CollisionError, MeshTriangle, PreparedMeshTriangles};
use std::{collections::HashMap, sync::{Arc, OnceLock}};

/// Native contact features accompany each triangle. Reconstructing adjacency
/// from welded positions would lose authored edge/vertex and one-sided flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidentGroundFeatures {
    pub flags: u32,
    pub edge_cosines: [f32; 3],
}

#[derive(Debug)]
pub enum ResidentGroundExportError {
    Collision(CollisionError),
    Topology(String),
    TriangleOrder { cluster: usize, unit: usize },
}

impl std::fmt::Display for ResidentGroundExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collision(error) => error.fmt(formatter),
            Self::Topology(error) => write!(formatter, "host ground topology: {error}"),
            Self::TriangleOrder { cluster, unit } => write!(
                formatter,
                "host triangle order differs from native GP order at cluster {cluster}, unit {unit}"
            ),
        }
    }
}

impl std::error::Error for ResidentGroundExportError {}

impl From<CollisionError> for ResidentGroundExportError {
    fn from(error: CollisionError) -> Self {
        Self::Collision(error)
    }
}

/// One actual published ground mesh. Retaining the arena keeps the identity
/// valid even when streaming removes its original slot.
#[derive(Debug)]
pub struct ResidentGroundGeometry {
    pub slot: StaticGroundSlotSnapshot,
    pub triangles: Vec<MeshTriangle>,
    /// Same length and native order as `triangles`.
    pub features: Vec<ResidentGroundFeatures>,
    owner: Arc<SimulationStaticArenaPlan>,
}

/// Identity for immutable derived geometry. Holding the arena prevents an old
/// occurrence from comparing equal to a later allocation in the same slot.
#[cfg(feature = "skate3-runtime")]
#[derive(Clone)]
pub(crate) struct ResidentGroundIdentity {
    slot: StaticGroundSlotSnapshot,
    owner: Arc<SimulationStaticArenaPlan>,
}

#[cfg(feature = "skate3-runtime")]
impl ResidentGroundIdentity {
    pub(crate) fn matches(&self, source: &ResidentGroundGeometry) -> bool {
        self.slot == source.slot && Arc::ptr_eq(&self.owner, &source.owner)
    }
}

impl ResidentGroundGeometry {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn identity(&self) -> ResidentGroundIdentity {
        ResidentGroundIdentity { slot: self.slot, owner: Arc::clone(&self.owner) }
    }

    /// Slot indices alone can be reused by streaming. Compare backing ownership
    /// as well before reusing an alternate engine's cached collision scene.
    #[must_use]
    pub fn same_source(&self, other: &Self) -> bool {
        self.slot == other.slot && Arc::ptr_eq(&self.owner, &other.owner)
    }
}

impl StaticGroundPool {
    /// Compare retained source identity without decoding unchanged geometry.
    #[must_use]
    pub fn matches_resident_geometry(&self, sources: &[ResidentGroundGeometry]) -> bool {
        self.slots.len() == sources.len()
            && self.slots.iter().zip(sources).all(|(slot, source)| {
                Arc::ptr_eq(&slot.owner, &source.owner)
                    && source.slot
                        == StaticGroundSlotSnapshot {
                            operation_index: slot.operation_index,
                            root_ordinal: slot.root_ordinal,
                            collision_root_index: slot.collision_root_index,
                            mesh_index: slot.mesh_index,
                        }
            })
    }

    /// Decode every published ground mesh in retained slot and native KD order.
    /// This mod-only export leaves the stock query schedule and capacities intact.
    ///
    /// # Errors
    /// Returns the original bounds/KD/cluster error; a failed mesh is never omitted.
    pub fn resident_geometry(
        &self,
    ) -> Result<Vec<ResidentGroundGeometry>, ResidentGroundExportError> {
        self.resident_geometry_reusing(Vec::new())
    }

    /// Keep decoded immutable meshes while their actual arena and slot identity
    /// remain resident. Streaming order is taken from the current pool verbatim;
    /// departing or replaced arena occurrences are discarded.
    pub fn resident_geometry_reusing(
        &self,
        previous: Vec<ResidentGroundGeometry>,
    ) -> Result<Vec<ResidentGroundGeometry>, ResidentGroundExportError> {
        decode_resident_geometry(&self.resident_geometry_jobs(), previous)
    }

    /// The current slots as [`decode_resident_geometry`] reads them, in streaming order: cheap
    /// handles to their arenas, bounds and shared triangle decodes, which another thread can take.
    #[must_use]
    pub fn resident_geometry_jobs(&self) -> Vec<ResidentGroundJob> {
        self.slots
            .iter()
            .map(|slot| ResidentGroundJob {
                slot: StaticGroundSlotSnapshot {
                    operation_index: slot.operation_index,
                    root_ordinal: slot.root_ordinal,
                    collision_root_index: slot.collision_root_index,
                    mesh_index: slot.mesh_index,
                },
                owner: Arc::clone(&slot.owner),
                bounds: slot.kd_bounds.clone(),
                triangle_decode: Arc::clone(&slot.triangle_decode),
            })
            .collect()
    }
}

/// One resident ground slot for [`decode_resident_geometry`]: its identity, arena, KD bounds and
/// the triangle decode it shares with the pool's own queries.
pub struct ResidentGroundJob {
    slot: StaticGroundSlotSnapshot,
    owner: Arc<SimulationStaticArenaPlan>,
    bounds: Result<([f32; 3], [f32; 3]), CollisionError>,
    triangle_decode: Arc<OnceLock<Result<PreparedMeshTriangles, CollisionError>>>,
}

/// Keep decoded immutable meshes while their actual arena and slot identity remain resident and
/// decode the rest, in the slots' order; departing or replaced arena occurrences are discarded.
///
/// # Errors
/// Returns the original bounds/KD/cluster error; a failed mesh is never omitted.
pub fn decode_resident_geometry(
    jobs: &[ResidentGroundJob],
    previous: Vec<ResidentGroundGeometry>,
) -> Result<Vec<ResidentGroundGeometry>, ResidentGroundExportError> {
    let mut previous: Vec<_> = previous.into_iter().map(Some).collect();
    // Retained meshes first, in slot order; the rest decode on the available cores.
    let mut slots: Vec<Option<ResidentGroundGeometry>> = jobs
        .iter()
        .map(|job| {
            previous.iter().position(|old| old.as_ref().is_some_and(|source|
                source.slot == job.slot && Arc::ptr_eq(&source.owner, &job.owner)))
                .and_then(|index| previous[index].take())
        })
        .collect();
    let reused = slots.iter().filter(|slot| slot.is_some()).count();
    let pending: Vec<usize> = (0..jobs.len()).filter(|&index| slots[index].is_none()).collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get).min(pending.len()).max(1);
    let decoded: Vec<(usize, Result<ResidentGroundGeometry, ResidentGroundExportError>)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let pending = &pending;
                scope.spawn(move || {
                    pending.iter().skip(worker).step_by(threads)
                        .map(|&index| (index, decode_job(&jobs[index])))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers.into_iter().flat_map(|worker| worker.join().expect("ground decode worker")).collect()
    });
    for (index, mesh) in decoded {
        slots[index] = Some(mesh?);
    }
    let result: Vec<ResidentGroundGeometry> = slots.into_iter().map(|slot| slot.expect("every slot decoded")).collect();
    crate::diagnostics::test_log::event("skate3_ground_decode",format_args!(
        "sources={} reused={reused} decoded={} threads={threads}",result.len(),result.len()-reused));
    Ok(result)
}

/// One slot's triangles and contact features, in native KD order.
fn decode_job(job: &ResidentGroundJob) -> Result<ResidentGroundGeometry, ResidentGroundExportError> {
    let &(minimum, maximum) = job.bounds.as_ref().map_err(Clone::clone)?;
    let mesh = &job.owner.asset().collision.meshes[job.slot.mesh_index];
    let triangle_start = std::time::Instant::now();
    let triangles = mesh_trajectory_triangles_in_native_order(
        mesh,
        &job.triangle_decode,
        minimum,
        maximum,
    )?;
    let triangle_ms = triangle_start.elapsed().as_secs_f64() * 1000.;
    let features_start = std::time::Instant::now();
    // The successful triangle export has already validated every
    // serialized cluster/unit in this same immutable mesh owner.
    let decoded = job.triangle_decode.get()
        .expect("successful triangle export initializes its decode")
        .as_ref().map_err(Clone::clone)?;
    let features = native_features(mesh, decoded, &triangles)?;
    let features_ms = features_start.elapsed().as_secs_f64() * 1000.;
    crate::diagnostics::test_log::event("skate3_ground_mesh_decode", format_args!(
        "slot={:?} triangles={} triangle_ms={triangle_ms:.3} features_ms={features_ms:.3}",
        job.slot, triangles.len()));
    Ok(ResidentGroundGeometry {
        slot: job.slot,
        triangles,
        features,
        owner: Arc::clone(&job.owner),
    })
}

/// One rigidly placed mesh (a PC Object Dropper prop): the same native-order
/// triangles and contact features as resident ground, decoded in model space
/// where the GP features are authored, then carried by `transform` (column
/// lanes 0/4/8 and translation 12, as `transform_point` uses). A rigid motion
/// keeps every edge cosine and flag, so the features need no re-derivation.
#[cfg(feature = "skate3-runtime")]
pub(crate) fn transformed_mesh_ground(
    mesh: &crate::world::CollisionMesh,
    transform: &[f32; 16],
) -> Result<(Vec<MeshTriangle>, Vec<ResidentGroundFeatures>), ResidentGroundExportError> {
    let tree = mesh.clustered_mesh.kd_tree(&mesh.object_bytes)?;
    let minimum: [f32; 3] = std::array::from_fn(|axis| tree.bounding_box_min[axis]);
    let maximum: [f32; 3] = std::array::from_fn(|axis| tree.bounding_box_max[axis]);
    let decode = std::sync::OnceLock::new();
    let mut triangles = mesh_trajectory_triangles_in_native_order(mesh, &decode, minimum, maximum)?;
    let decoded = decode
        .get()
        .expect("successful triangle export initializes its decode")
        .as_ref()
        .map_err(Clone::clone)?;
    let features = native_features(mesh, decoded, &triangles)?;
    for triangle in &mut triangles {
        triangle.vertices = triangle.vertices.map(|[x, y, z]| {
            std::array::from_fn(|row| {
                transform[row] * x + transform[4 + row] * y + transform[8 + row] * z + transform[12 + row]
            })
        });
    }
    Ok((triangles, features))
}

fn native_features(
    mesh: &crate::world::CollisionMesh,
    decoded: &PreparedMeshTriangles,
    triangles: &[MeshTriangle],
) -> Result<Vec<ResidentGroundFeatures>, ResidentGroundExportError> {
    let (clusters, units) = decoded.cluster_units();
    #[cfg(test)]
    let audit_full_gp = std::env::var_os("SKATE2_AUDIT_GROUND_FEATURES").is_some();
    // The numeric oracle independently reparses the original bytes, so a
    // mismatched retained decode cannot validate itself through the GP check.
    #[cfg(test)]
    let audit_decode = if audit_full_gp {
        Some(PreparedMeshTriangles::new(
            &mesh.clustered_mesh,
            &mesh.object_bytes,
        )?)
    } else {
        None
    };
    // Only lookups are performed: output order remains the incoming KD order.
    let mut batches = HashMap::new();
    let mut last_unit = None;
    let mut primitive = 0;
    let mut features = Vec::with_capacity(triangles.len());
    for triangle in triangles {
        let key = (triangle.cluster_index, triangle.unit_stream_offset);
        let batch = match batches.entry(key) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let cluster = &clusters[key.0];
                let unit = units[key.0]
                    .binary_search_by_key(&key.1, |unit| unit.stream_offset)
                    .map_err(|_| ResidentGroundExportError::TriangleOrder {
                        cluster: key.0,
                        unit: key.1,
                    })?;
                let batch = build_old_assembly_unit_features(
                    &mesh.clustered_mesh,
                    key.0,
                    cluster,
                    &units[key.0][unit],
                    super::STATIC_GROUND_IDENTITY_TRANSFORM,
                )
                .map_err(|error| ResidentGroundExportError::Topology(format!("{error:?}")))?;
                #[cfg(test)]
                if audit_full_gp {
                    let (original_clusters, original_units) = audit_decode
                        .as_ref()
                        .expect("requested original-byte GP oracle")
                        .cluster_units();
                    let original_unit = original_units[key.0]
                        .binary_search_by_key(&key.1, |unit| unit.stream_offset)
                        .map_err(|_| ResidentGroundExportError::TriangleOrder {
                            cluster: key.0,
                            unit: key.1,
                        })?;
                    let oracle = crate::simulation_assembly_ground_triangle_topology::build_old_assembly_unit_gp(
                    &mesh.clustered_mesh,key.0,&original_clusters[key.0],&original_units[key.0][original_unit],
                    super::STATIC_GROUND_IDENTITY_TRANSFORM,0,
                ).map_err(|error|ResidentGroundExportError::Topology(format!("{error:?}")))?;
                    let expected: Vec<_> = oracle.iter().map(|record| record.primitive()).collect();
                    assert_eq!(
                        batch.len(),
                        expected.len(),
                        "native feature primitive count"
                    );
                    for (actual, expected) in batch.iter().zip(expected) {
                        assert_eq!(
                            actual.points.map(|point| point.map(f32::to_bits)),
                            expected.points.map(|point| point.map(f32::to_bits)),
                            "native feature winding/transform"
                        );
                        assert_eq!(
                            actual.edge_data.map(f32::to_bits),
                            expected.edge_data.map(f32::to_bits),
                            "native feature edge bits"
                        );
                        assert_eq!(actual.flags, expected.flags, "native feature flags");
                    }
                }
                entry.insert(batch)
            }
        };
        // Each KD unit visit emits its complete triangle/quad prefix. Repeated
        // visits retain their original duplicates and restart that same prefix.
        if last_unit != Some(key) || primitive == batch.len() {
            primitive = 0;
        }
        let record = batch
            .get(primitive)
            .filter(|record| record.points == triangle.vertices)
            .ok_or(ResidentGroundExportError::TriangleOrder {
                cluster: key.0,
                unit: key.1,
            })?;
        features.push(ResidentGroundFeatures {
            flags: record.flags,
            edge_cosines: record.edge_data,
        });
        last_unit = Some(key);
        primitive += 1;
    }
    Ok(features)
}
