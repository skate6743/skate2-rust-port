//! Mod solid providers for biped ground queries and moving-object classification.
use super::ground_query::{Bounds, EdgeBody, PrimaryEdges, Segment};
use skate3_core::{
    math::Vector3,
    player::offboard::ground_query::{Edge, Frame},
    player::offboard::ground_sync::{BoardLimits, Bounds as BoardBounds},
};
use skate_dynamics::SolidBody;

pub(crate) const VEHICLE_GROUP: u32 = 8;

/// Per-frame edge storage for `PrimaryEdges::Normal.vehicles`.
pub(crate) struct VehicleEdgeCache {
    bodies: Vec<EdgeBodyStorage>,
    segments: Vec<Segment>,
}

struct EdgeBodyStorage {
    frame: Frame,
    local_bounds: Bounds,
    segment_range: std::ops::Range<usize>,
}

impl VehicleEdgeCache {
    pub(crate) fn build(solids: &[(usize, SolidBody)]) -> Self {
        let mut cache = Self {
            bodies: Vec::new(),
            segments: Vec::new(),
        };
        for (_, solid) in solids {
            if solid.contact_group == VEHICLE_GROUP {
                cache.push_solid(solid);
            }
        }
        cache
    }

    pub(crate) fn vehicles(&self) -> Vec<EdgeBody<'_>> {
        self.bodies
            .iter()
            .map(|body| EdgeBody {
                local_to_world: body.frame,
                local_bounds: body.local_bounds,
                segments: &self.segments[body.segment_range.clone()],
            })
            .collect()
    }

    pub(crate) fn primary_edges<'a>(&'a self, vehicles: &'a [EdgeBody<'a>]) -> PrimaryEdges<'a> {
        PrimaryEdges::Normal {
            dynamic: &[],
            vehicles,
        }
    }

    pub(crate) fn with_primary_edges<R>(
        &self,
        f: impl FnOnce(PrimaryEdges<'_>) -> R,
    ) -> R {
        let vehicles = self.vehicles();
        f(self.primary_edges(&vehicles))
    }

    fn push_solid(&mut self, solid: &SolidBody) {
        let start = self.segments.len();
        for collider in &solid.colliders {
            let aabb = collider.shape.compute_aabb(&collider.pose);
            let min = vec3(aabb.mins.x, aabb.mins.y, aabb.mins.z);
            let max = vec3(aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
            let y = max.y;
            for (a, b) in [
                (Vector3::new(min.x, y, min.z), Vector3::new(max.x, y, min.z)),
                (Vector3::new(max.x, y, min.z), Vector3::new(max.x, y, max.z)),
                (Vector3::new(max.x, y, max.z), Vector3::new(min.x, y, max.z)),
                (Vector3::new(min.x, y, max.z), Vector3::new(min.x, y, min.z)),
            ] {
                let edge_min = Vector3::new(
                    a.x.min(b.x),
                    a.y.min(b.y),
                    a.z.min(b.z),
                );
                let edge_max = Vector3::new(
                    a.x.max(b.x),
                    a.y.max(b.y),
                    a.z.max(b.z),
                );
                self.segments.push(Segment {
                    edge: Edge { start: a, end: b },
                    local_bounds: Bounds {
                        min: edge_min,
                        max: edge_max,
                    },
                });
            }
        }
        if start == self.segments.len() {
            return;
        }
        let (min, max) = segment_bounds(&self.segments[start..]);
        self.bodies.push(EdgeBodyStorage {
            // Collider poses are already world-space; keep segments in world
            // coordinates and use the identity frame when transforming them.
            frame: Frame::IDENTITY,
            local_bounds: Bounds { min, max },
            segment_range: start..self.segments.len(),
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VehicleCandidate {
    pub id: u64,
}

pub(crate) fn probe_vehicle(
    solids: &[(usize, SolidBody)],
    position: [f32; 4],
) -> Option<VehicleCandidate> {
    let mut best = None;
    let mut best_distance = f32::MAX;
    for (_, solid) in solids {
        if solid.contact_group != VEHICLE_GROUP {
            continue;
        }
        let (min, max) = solid_world_aabb(solid);
        if position[0] < min.x - 1.0
            || position[0] > max.x + 1.0
            || position[2] < min.z - 1.0
            || position[2] > max.z + 1.0
        {
            continue;
        }
        let height = position[1] - max.y;
        if height < -0.6 || height > 1.2 {
            continue;
        }
        let center = Vector3::new((min.x + max.x) * 0.5, max.y, (min.z + max.z) * 0.5);
        let dx = position[0] - center.x;
        let dz = position[2] - center.z;
        let distance = dx * dx + dz * dz;
        if distance < best_distance {
            best_distance = distance;
            best = Some(VehicleCandidate { id: solid.id });
        }
    }
    best
}

pub(crate) fn classify_vehicle(
    solid: &SolidBody,
    bone: [f32; 4],
    bounds: BoardBounds,
    limits: BoardLimits,
) -> bool {
    if !is_moving(solid) {
        return false;
    }
    let (min, max) = solid_world_aabb(solid);
    let top = max.y;
    if bone[1] < top - 0.35 || bone[1] > top + 1.5 {
        return false;
    }
    let local = inverse_point(bounds.frame, Vector3::new(bone[0], bone[1], bone[2]));
    let margin = limits.margin;
    (0..3).all(|i| {
        local[i] + margin >= -bounds.extents[i] && local[i] - margin <= bounds.extents[i]
    }) && bone[0] >= min.x - margin
        && bone[0] <= max.x + margin
        && bone[2] >= min.z - margin
        && bone[2] <= max.z + margin
}

pub(crate) fn solid_by_id(solids: &[(usize, SolidBody)], id: u64) -> Option<&SolidBody> {
    solids
        .iter()
        .find(|(_, solid)| solid.id == id)
        .map(|(_, solid)| solid)
}

pub(crate) fn candidate_key(candidate: &VehicleCandidate) -> [u32; 2] {
    [
        (candidate.id & 0xffff_ffff) as u32,
        (candidate.id >> 32) as u32,
    ]
}

fn is_moving(solid: &SolidBody) -> bool {
    let v = solid.linvel;
    let w = solid.angvel;
    v.length_squared() > f32::from_bits(0x3a83126f)
        || w.length_squared() > f32::from_bits(0x3a83126f)
}

fn solid_world_aabb(solid: &SolidBody) -> (Vector3, Vector3) {
    let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for collider in &solid.colliders {
        let aabb = collider.shape.compute_aabb(&collider.pose);
        min = min_vec(min, vec3(aabb.mins.x, aabb.mins.y, aabb.mins.z));
        max = max_vec(max, vec3(aabb.maxs.x, aabb.maxs.y, aabb.maxs.z));
    }
    if !vector_finite(min) {
        let p = vec3(
            solid.center_of_mass.x,
            solid.center_of_mass.y,
            solid.center_of_mass.z,
        );
        return (
            Vector3::new(p.x - 1.0, p.y - 1.0, p.z - 1.0),
            Vector3::new(p.x + 1.0, p.y + 1.0, p.z + 1.0),
        );
    }
    (min, max)
}

fn segment_bounds(segments: &[Segment]) -> (Vector3, Vector3) {
    let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for segment in segments {
        min = min_vec(min, min_vec(segment.edge.start, segment.edge.end));
        max = max_vec(max, max_vec(segment.edge.start, segment.edge.end));
    }
    (min, max)
}

fn inverse_point(frame: [[f32; 4]; 4], point: Vector3) -> [f32; 4] {
    let right = Vector3::new(frame[0][0], frame[0][1], frame[0][2]);
    let up = Vector3::new(frame[1][0], frame[1][1], frame[1][2]);
    let forward = Vector3::new(frame[2][0], frame[2][1], frame[2][2]);
    let origin = Vector3::new(frame[3][0], frame[3][1], frame[3][2]);
    let delta = Vector3::new(
        point.x - origin.x,
        point.y - origin.y,
        point.z - origin.z,
    );
    [
        dot(right, delta),
        dot(up, delta),
        dot(forward, delta),
        0.0,
    ]
}

fn vec3(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z)
}

fn min_vec(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
}

fn max_vec(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
}

fn dot(a: Vector3, b: Vector3) -> f32 {
    a.x.mul_add(b.x, a.y.mul_add(b.y, a.z * b.z))
}

fn vector_finite(v: Vector3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}
