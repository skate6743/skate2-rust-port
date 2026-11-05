//! Contacts for every board primitive against the host triangle world.
//! Rust owns geometry storage and traversal. Recovered query8277B720,
//! triangle fixup82AD3130, material combine82763078 and contact retention
//! determine the physical result, in world-triangle then moving-volume order.
mod broadphase;
mod query_index;
pub mod query_metadata;
use crate::math::Vector3;
pub use broadphase::{VOLUME_QUERY_SCALE, VOLUME_QUERY_STEP, VolumeMotion, volume_query_bounds};
use broadphase::{conservative_bounds, primitive_bounds};
use query_metadata::{Bounds, QueryMetadata};

use super::{
    board::BodyId,
    board_runtime::BoardRuntime,
    board_step::{BoardCollision, CollisionBody},
    collision::{Sphere, Triangle, WorldContactSettings},
    contact::{RetailContactInput, RetailContactMaterial, combine_contact_materials},
    triangle_query::{TriangleLineHit, triangle_segment},
    world_contact::{
        ContactBuffer, ContactPrimitive, ContactRecord, primitive_triangle_world_contacts,
        triangle_from_volume,
    },
};

/// Retail surface type 12 (`(tag >> 7) & 31`): water.
pub const fn is_water_tag(tag: u32) -> bool {
    tag & 0xF80 == 0x600
}

/// Water with geometry within this depth under the surface (at a given point)
/// is shallow: solid there, like retail's University fountain channel, where
/// the bailed skater lies on the water. Deeper water is not solid: bodies sink
/// in and float (Aletown canal). Project-chosen depth
/// (docs/hails-additions/09-water.md).
pub const FLOAT_DEPTH: f32 = 0.5;

#[derive(Clone, Copy, Debug)]
pub struct WorldTriangle {
    pub triangle: Triangle,
    pub material: RetailContactMaterial,
    pub tag: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldLineHit {
    pub geometry: TriangleLineHit,
    pub tag: u32,
}

impl WorldTriangle {
    /// Host geometry preparation, with explicit adjacency and sidedness.
    /// Uses the native Volume constructor and its authored winding. Invalid
    /// host geometry is rejected without substituting a repaired triangle.
    pub fn from_vertices(
        vertices: [Vector3; 3],
        material: RetailContactMaterial,
        tag: u32,
        flags: u32,
        edge_cosines: [f32; 3],
        fatness: f32,
    ) -> Option<Self> {
        if vertices
            .iter()
            .any(|v| !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite())
        {
            return None;
        }
        let triangle = triangle_from_volume(vertices, fatness, edge_cosines, flags);
        let normal = triangle.feature.normal;
        if triangle
            .edge_lengths
            .iter()
            .any(|&v| !v.is_finite() || v <= 0.0)
            || !normal.x.is_finite()
            || !normal.y.is_finite()
            || !normal.z.is_finite()
            || normal == Vector3::ZERO
        {
            return None;
        }
        Some(Self {
            triangle,
            material,
            tag,
        })
    }
}

/// Caller-resolved TempContactBuffer settings. Retention changes solver rows,
/// so its thresholds/capacity remain explicit instead of using tuned defaults.
#[derive(Clone, Copy, Debug)]
pub struct ContactRetentionSettings {
    pub capacity: u32,
    pub duplicate_distance_squared: f32,
    pub deferred_reduction: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct WheelWorldSettings {
    /// Sphere shared by wheel definitions in82C0AA78.
    pub radius: f32,
    pub material: RetailContactMaterial,
    pub query: WorldContactSettings,
    pub retention: ContactRetentionSettings,
}

/// One enabled moving primitive, with its owning solver body and material.
#[derive(Clone, Copy, Debug)]
pub struct BoardWorldVolume {
    /// Native serialized VolumeData+212 (Part+92), not Volume+84.
    pub collision_group: u32,
    pub body: CollisionBody,
    pub primitive: ContactPrimitive,
    /// Owning body's rates; 82777E70 sweeps the query box over them.
    pub motion: VolumeMotion,
    pub material: RetailContactMaterial,
}

/// Host-supplied moving geometry. The core remains independent of Rapier and
/// queries the same solid shapes as the host's rigid-body contact bridge.
#[derive(Clone, Copy, Debug)]
pub struct ExternalLineHit {
    pub hit: WorldLineHit,
    pub surface: u16,
    pub geometry_id: u32,
    pub frame: [[f32; 4]; 4],
}
pub trait ExternalQueries: Send + Sync {
    fn line(&self, start: Vector3, end: Vector3, radius: f32) -> Option<ExternalLineHit>;
    fn nearby(&self, center: Vector3, radius: f32) -> Vec<[Vector3; 3]>;
}

/// Immutable canonical geometry and its matching per-triangle bounds.
/// Private storage keeps cached bounds tied to the exact supplied triangles.
pub struct PreparedTriangles {
    triangles: Vec<WorldTriangle>,
    triangle_bounds: Vec<Bounds>,
}

impl PreparedTriangles {
    pub fn new(triangles: Vec<WorldTriangle>) -> Self {
        let triangle_bounds: Vec<_> = triangles
            .iter()
            .map(|t| {
                Bounds::from_points(t.triangle.vertices).unwrap_or(Bounds {
                    min: Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                    max: Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
                })
            })
            .collect();
        Self {
            triangles,
            triangle_bounds,
        }
    }

    /// Preserve chunk order, including repeated geometry and chunk boundaries
    /// that do not coincide with the world's query leaves.
    pub fn concatenate(chunks: &[std::sync::Arc<Self>]) -> Self {
        let count = chunks.iter().map(|chunk| chunk.triangles.len()).sum();
        let mut triangles = Vec::with_capacity(count);
        let mut triangle_bounds = Vec::with_capacity(count);
        for chunk in chunks {
            triangles.extend_from_slice(&chunk.triangles);
            triangle_bounds.extend_from_slice(&chunk.triangle_bounds);
        }
        Self {
            triangles,
            triangle_bounds,
        }
    }

    pub fn triangles(&self) -> &[WorldTriangle] {
        &self.triangles
    }
    pub fn triangle_bounds(&self) -> &[Bounds] {
        &self.triangle_bounds
    }
}

/// World geometry remains in supplied order. Each query reuses output storage;
/// the returned contacts are valid until the next mutable call.
pub struct BoardWorld {
    triangles: Vec<WorldTriangle>,
    external: Option<std::sync::Arc<dyn ExternalQueries>>,
    triangle_bounds: Vec<Bounds>,
    query_metadata: Option<QueryMetadata>,
    query_index: query_index::QueryIndex,
    maximum_fatness: f32,
    maximum_triangle_margin: f32,
    /// Indices of water triangles, for `water_surface_at`.
    water: Vec<usize>,
    contacts: Vec<BoardCollision>,
    buffer: ContactBuffer,
}

/// Host construction measurements only; no simulation step uses these values.
#[derive(Clone, Copy, Debug)]
pub struct WorldConstructionTimings {
    pub validation: std::time::Duration,
    pub triangle_data: std::time::Duration,
    pub query_index: std::time::Duration,
}

impl BoardWorld {
    pub fn new(triangles: Vec<WorldTriangle>) -> Self {
        Self::from_prepared(PreparedTriangles::new(triangles))
    }

    fn from_prepared(prepared: PreparedTriangles) -> Self {
        let PreparedTriangles {
            triangles,
            triangle_bounds,
        } = prepared;
        let maximum_fatness = triangles
            .iter()
            .map(|t| {
                if t.triangle.fatness.is_finite() && t.triangle.fatness >= 0. {
                    t.triangle.fatness
                } else {
                    f32::INFINITY
                }
            })
            .fold(0., f32::max);
        // The current thin-triangle leaf accepts barycentric weights down to
        // -epsilon. Two negative weights can extend an axis by 2*epsilon*span.
        let maximum_triangle_margin = triangle_bounds
            .iter()
            .map(|b| {
                let span = (b.max.x - b.min.x)
                    .max(b.max.y - b.min.y)
                    .max(b.max.z - b.min.z);
                span * (2. * broadphase::THIN_MARGIN)
            })
            .fold(0., f32::max);
        let water = (0..triangles.len())
            .filter(|&i| is_water_tag(triangles[i].tag))
            .collect();
        Self {
            water,
            triangles,
            external: None,
            triangle_bounds,
            query_metadata: None,
            query_index: query_index::QueryIndex::default(),
            maximum_fatness,
            maximum_triangle_margin,
            contacts: Vec::new(),
            buffer: ContactBuffer {
                count: 0,
                flushed: 0,
                capacity: 0,
                dropped: 0,
                distance_squared_threshold: 0.0,
                allow_flush: 1,
                deferred_reduction: 0,
                full: 0,
                records: [[0; 64]; 50],
            },
        }
    }

    /// Whether the water at `point` on its surface is shallow: some geometry
    /// lies within `FLOAT_DEPTH` under it (a channel, a ledge, a bank).
    pub fn water_shallow_at(&self, point: Vector3) -> bool {
        let start = Vector3::new(point.x, point.y - 0.001, point.z);
        let end = Vector3::new(point.x, point.y - FLOAT_DEPTH, point.z);
        !matches!(self.query_thin_line(start, end), Ok(None))
    }

    pub fn with_query_metadata(
        triangles: Vec<WorldTriangle>,
        metadata: QueryMetadata,
    ) -> Result<Self, &'static str> {
        metadata.validate(&triangles)?;
        let mut world = Self::new(triangles);
        world.query_index = query_index::QueryIndex::new(&metadata.meshes);
        world.query_metadata = Some(metadata);
        Ok(world)
    }

    /// The same validated construction order, with separate measurements for
    /// streamed-world diagnostics. Query/contact arithmetic remains unchanged.
    pub fn with_query_metadata_profiled(
        triangles: Vec<WorldTriangle>,
        metadata: QueryMetadata,
    ) -> Result<(Self, WorldConstructionTimings), &'static str> {
        let start = std::time::Instant::now();
        metadata.validate(&triangles)?;
        let validation = start.elapsed();
        let start = std::time::Instant::now();
        let mut world = Self::new(triangles);
        let triangle_data = start.elapsed();
        let start = std::time::Instant::now();
        world.query_index = query_index::QueryIndex::new(&metadata.meshes);
        let query_index = start.elapsed();
        world.query_metadata = Some(metadata);
        Ok((
            world,
            WorldConstructionTimings {
                validation,
                triangle_data,
                query_index,
            },
        ))
    }

    /// Full metadata validation is retained; only immutable triangle bounds
    /// have been prepared earlier. Aggregate query settings use the same
    /// supplied-order reductions as ordinary construction.
    pub fn with_prepared_query_metadata_profiled(
        prepared: PreparedTriangles,
        metadata: QueryMetadata,
    ) -> Result<(Self, WorldConstructionTimings), &'static str> {
        let start = std::time::Instant::now();
        metadata.validate(prepared.triangles())?;
        let validation = start.elapsed();
        let start = std::time::Instant::now();
        let mut world = Self::from_prepared(prepared);
        let triangle_data = start.elapsed();
        let start = std::time::Instant::now();
        world.query_index = query_index::QueryIndex::new(&metadata.meshes);
        let query_index = start.elapsed();
        world.query_metadata = Some(metadata);
        Ok((
            world,
            WorldConstructionTimings {
                validation,
                triangle_data,
                query_index,
            },
        ))
    }

    /// Capture matching geometry/bounds from an already constructed world.
    pub fn prepared_triangle_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> Option<PreparedTriangles> {
        Some(PreparedTriangles {
            triangles: self.triangles.get(range.clone())?.to_vec(),
            triangle_bounds: self.triangle_bounds.get(range)?.to_vec(),
        })
    }

    pub fn triangle_bounds(&self) -> &[Bounds] {
        &self.triangle_bounds
    }

    pub fn query_metadata(&self) -> Result<&QueryMetadata, &'static str> {
        self.query_metadata
            .as_ref()
            .ok_or("Canonical world has no authored query metadata")
    }

    /// Replace only static terrain. The host already published moving-body
    /// queries for this tick; streaming must not disconnect those snapshots.
    pub fn replace_static_geometry(&mut self, mut replacement: Self) {
        replacement.external = self.external.take();
        *self = replacement;
    }

    pub fn set_external_queries(&mut self, queries: Option<std::sync::Arc<dyn ExternalQueries>>) {
        self.external = queries;
    }
    pub fn external_line(
        &self,
        start: Vector3,
        end: Vector3,
        radius: f32,
    ) -> Option<ExternalLineHit> {
        self.external.as_ref()?.line(start, end, radius)
    }
    pub fn external_nearby(&self, center: Vector3, radius: f32) -> Vec<[Vector3; 3]> {
        self.external
            .as_ref()
            .map_or_else(Vec::new, |q| q.nearby(center, radius))
    }

    pub fn triangles(&self) -> &[WorldTriangle] {
        &self.triangles
    }

    /// Board wheel segments use zero query radius; rounded world triangles
    /// still select the native swept branch through their own fatness.
    pub fn query_thin_line(
        &self,
        start: Vector3,
        end: Vector3,
    ) -> Result<Option<WorldLineHit>, &'static str> {
        self.query_swept_line(start, end, 0.0)
    }

    /// Ordered nearest-hit traversal for wheel and camera queries. The host
    /// owns traversal; each candidate uses the complete TU3 triangle dispatcher.
    pub fn query_swept_line(
        &self,
        start: Vector3,
        end: Vector3,
        radius: f32,
    ) -> Result<Option<WorldLineHit>, &'static str> {
        if !radius.is_finite() || radius < 0.0 {
            return Err("line query radius must be finite and nonnegative");
        }
        let direction = Vector3::new(end.x - start.x, end.y - start.y, end.z - start.z);
        let mut nearest: Option<WorldLineHit> =
            self.external_line(start, end, radius).map(|h| h.hit);
        for (_, entry) in self.line_candidates(start, end, radius) {
            let mut geometry = TriangleLineHit {
                position: Vector3::ZERO,
                normal: Vector3::ZERO,
                fraction: 0.0,
                volume_parameter: [0.0; 3],
            };
            if triangle_segment(
                &mut geometry,
                start,
                direction,
                entry.triangle.vertices,
                radius,
                entry.triangle.fatness,
            ) {
                if nearest
                    .as_ref()
                    .is_none_or(|hit| geometry.fraction < hit.geometry.fraction)
                {
                    nearest = Some(WorldLineHit {
                        geometry,
                        tag: entry.tag,
                    });
                }
            }
        }
        Ok(nearest)
    }

    /// Sphere-only convenience for callers querying wheels. The game uses
    /// query_primitives with all enabled deck/truck/wheel children.
    pub fn query(
        &mut self,
        board: &BoardRuntime,
        settings: WheelWorldSettings,
    ) -> &[BoardCollision] {
        let poses = board.part_transforms();
        let volumes: Vec<_> = BodyId::ORDER[..4]
            .iter()
            .filter_map(|&id| {
                let body = &board.bodies()[id.index()];
                (body.state_flags != 1).then_some(BoardWorldVolume {
                    collision_group: board.collision_group(),
                    body: CollisionBody::Board(id),
                    primitive: ContactPrimitive::Sphere(Sphere {
                        center: poses[id.index()].translation,
                        radius: settings.radius,
                    }),
                    motion: VolumeMotion::of(&body.rates),
                    material: settings.material,
                })
            })
            .collect();
        self.query_primitives(&volumes, settings.query, settings.retention)
    }

    /// World-space shapes come from the live part poses and authored children.
    /// Cluster bounds include shape radii and the maximum predictive padding.
    /// Candidate pairs retain world-triangle then moving-volume order.
    pub fn query_primitives(
        &mut self,
        volumes: &[BoardWorldVolume],
        query: WorldContactSettings,
        retention: ContactRetentionSettings,
    ) -> &[BoardCollision] {
        self.contacts.clear();
        self.buffer.count = 0;
        self.buffer.flushed = 0;
        self.buffer.dropped = 0;
        self.buffer.full = 0;
        self.buffer.capacity = retention.capacity;
        self.buffer.distance_squared_threshold = retention.duplicate_distance_squared;
        self.buffer.deferred_reduction = u8::from(retention.deferred_reduction);
        let padding =
            if query.volume_padding.is_finite() && query.maximum_separating_distance.is_finite() {
                query.volume_padding.max(0.) + query.maximum_separating_distance.max(0.)
            } else {
                f32::INFINITY
            };
        // Cluster preselection only: a conservative superset of every pair the
        // per-volume test below can keep.
        let preselect_bounds: Vec<_> = volumes
            .iter()
            .map(|v| {
                primitive_bounds(v.primitive)
                    .map(|b| conservative_bounds(b, padding + self.maximum_fatness))
            })
            .collect();
        // 82777E70: the swept, scaled box 8277BC58 tests every triangle against.
        let volume_bounds: Vec<_> = volumes
            .iter()
            .map(|v| {
                volume_query_bounds(v.primitive, v.motion, VOLUME_QUERY_STEP, VOLUME_QUERY_SCALE)
            })
            .collect();
        let bounds: Option<Vec<_>> = preselect_bounds
            .iter()
            .zip(&volume_bounds)
            .map(|(p, v)| match (p, v) {
                (Some(p), Some(v)) => Some(Bounds {
                    min: Vector3::new(
                        p.min.x.min(v.min.x),
                        p.min.y.min(v.min.y),
                        p.min.z.min(v.min.z),
                    ),
                    max: Vector3::new(
                        p.max.x.max(v.max.x),
                        p.max.y.max(v.max.y),
                        p.max.z.max(v.max.z),
                    ),
                }),
                _ => None,
            })
            .collect();
        let bounds = bounds
            .and_then(|b| Bounds::from_points(b.iter().flat_map(|b| [b.min, b.max])))
            .map(|b| b.expanded(padding));
        let ranges = self.candidate_ranges(bounds);
        let mut output = std::mem::take(&mut self.contacts);
        let mut publish = |records: &[ContactRecord]| {
            output.extend(records.iter().map(collision_from_record));
        };
        'triangles: for index in ranges.into_iter().flatten() {
            let entry = self.triangles[index];
            for (volume, volume_bounds) in volumes.iter().zip(&volume_bounds) {
                // GroundPipeline8277BC58 checks the group/surface exclusion
                // table before running the geometric dispatcher. Its stock
                // initializer82767D60 sets only (7,12) and (16,12).
                if matches!(volume.collision_group, 7 | 16) && is_water_tag(entry.tag) {
                    continue;
                }
                // 8277BC58 BE94..BF38: unpadded triangle vertex box against the
                // volume box, for every triangle. A non-finite box keeps the pair.
                if volume_bounds.is_some_and(|b| !self.triangle_bounds[index].overlaps(b)) {
                    continue;
                }
                let Some(manifold) = primitive_triangle_world_contacts(
                    volume.primitive,
                    entry.triangle,
                    volume.motion.linear_velocity,
                    query,
                ) else {
                    continue;
                };
                let material = combine_contact_materials(volume.material, entry.material);
                for pair in &manifold.points[..manifold.count] {
                    let contact = RetailContactInput {
                        position_on_a: pair.a,
                        position_on_b: pair.b,
                        normal: manifold.normal,
                        restitution: material.restitution,
                        static_friction: material.static_friction,
                        dynamic_friction: material.dynamic_friction,
                        tag: entry.tag,
                    };
                    let Some(slot) = self.buffer.allocate(&mut publish) else {
                        self.buffer.flush(&mut publish);
                        break 'triangles;
                    };
                    self.buffer.records[slot] = retention_record(volume.body, contact);
                    //8277C23C removes the most recent record on rejection.
                    if self.buffer.last_is_duplicate() {
                        self.buffer.count -= 1;
                    }
                }
            }
        }
        if self.buffer.full == 0 {
            self.buffer.flush(&mut publish);
        }
        self.contacts = output;
        &self.contacts
    }

    pub fn contacts(&self) -> &[BoardCollision] {
        &self.contacts
    }

    /// `water_surface_at`, but only where the water is at least `min_depth`
    /// deep: no solid (or other) geometry within `min_depth` under the surface
    /// at the point's XZ. Shallow water over a floor is not something to float in.
    pub fn deep_water_surface_at(
        &self,
        point: Vector3,
        above: f32,
        max_depth: f32,
        min_depth: f32,
    ) -> Option<f32> {
        let surface = self.water_surface_at(point, above, max_depth)?;
        let start = Vector3::new(point.x, surface - 0.001, point.z);
        let end = Vector3::new(point.x, surface - min_depth, point.z);
        matches!(self.query_thin_line(start, end), Ok(None)).then_some(surface)
    }

    /// Height of the water surface over `point`, when the point lies at most
    /// `max_depth` below it (or less than `above` over it). Water surfaces are
    /// the type-12 triangles, interpolated at the point's XZ; the nearest
    /// surface at or above the point wins.
    pub fn water_surface_at(&self, point: Vector3, above: f32, max_depth: f32) -> Option<f32> {
        let mut best: Option<f32> = None;
        for &index in &self.water {
            let [a, b, c] = self.triangles[index].triangle.vertices;
            let det = (b.x - a.x) * (c.z - a.z) - (c.x - a.x) * (b.z - a.z);
            if det.abs() < 1e-8 {
                continue;
            }
            let u = ((point.x - a.x) * (c.z - a.z) - (c.x - a.x) * (point.z - a.z)) / det;
            let v = ((b.x - a.x) * (point.z - a.z) - (point.x - a.x) * (b.z - a.z)) / det;
            if u < 0. || v < 0. || u + v > 1. {
                continue;
            }
            let height = a.y + u * (b.y - a.y) + v * (c.y - a.y);
            let depth = height - point.y;
            if depth > -above && depth <= max_depth && best.is_none_or(|h| height < h) {
                best = Some(height);
            }
        }
        best
    }

    pub fn dropped_contacts(&self) -> u32 {
        self.buffer.dropped
    }
}

/// Retention reads only contact header geometry, body IDs and material/tag.
/// BoardStep builds the actual body workspaces after this frame's force queue
/// has been applied, preventing stale acceleration copies in solver contacts.
fn retention_record(id: CollisionBody, contact: RetailContactInput) -> ContactRecord {
    let mut record = [0; 64];
    for (offset, v) in [
        (0, contact.position_on_a),
        (4, contact.position_on_b),
        (8, contact.normal),
    ] {
        record[offset..offset + 3].copy_from_slice(&[v.x, v.y, v.z].map(f32::to_bits));
    }
    record[3] = id.contact_id();
    record[7] = u32::MAX;
    record[11] = contact.restitution.to_bits();
    record[15] = contact.static_friction.to_bits();
    record[19] = contact.dynamic_friction.to_bits();
    record[23] = contact.tag;
    record
}

fn collision_from_record(record: &ContactRecord) -> BoardCollision {
    let f = |i| f32::from_bits(record[i]);
    let vector = |i| Vector3::new(f(i), f(i + 1), f(i + 2));
    BoardCollision {
        body_a: CollisionBody::from_contact_id(record[3]),
        body_b: CollisionBody::StaticWorld,
        contact: RetailContactInput {
            position_on_a: vector(0),
            position_on_b: vector(4),
            normal: vector(8),
            restitution: f(11),
            static_friction: f(15),
            dynamic_friction: f(19),
            tag: record[23],
        },
    }
}

#[cfg(test)]
#[path = "tests/board_world.rs"]
mod tests;

#[cfg(test)]
#[path = "board_world/tests.rs"]
mod broadphase_tests;
