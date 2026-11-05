//! Solid dual-world boundary. Native local bones participate in BoardStep;
//! remote actor mirrors participate in Rapier. Never solve the local actor in
//! both worlds, and never feed reactions back to another peer's body.
//!
//! A moving body with an authored collision mesh (Object Dropper props, city
//! DMOs) answers line and nearby-triangle queries with that mesh's native
//! triangles at its current placement, exactly as static ground answers them:
//! the same triangle dispatcher, GP features and packed surface. Other bodies
//! answer through their solid Parry shapes.

use crate::skate3_native::math_host::*;
use skate3_core::{math::Vector3, physics::{
    board_world::{ExternalLineHit, ExternalQueries, PreparedTriangles, WorldLineHit, query_metadata::Bounds},
    triangle_query::{TriangleLineHit, triangle_segment},
}};
use skate_dynamics::{SolidBody, solid::{collider_triangles, sweep_sphere}};
use std::sync::Arc;

fn xyz(v: Vector3) -> [f32;3] { [v.x,v.y,v.z] }
fn native(v: [f32;3]) -> Vector3 { Vector3::new(v[0],v[1],v[2]) }

/// Nearby-triangle answers per query. Every consumer keeps at most 64 after
/// its own exact triangle/box test, so this only bounds a pathological query.
const NEARBY_CAPACITY: usize = 512;

fn body_frame(body: &SolidBody) -> [[f32; 4]; 4] {
    let q = body.pose.rotation;
    Mat4::from_rotation_translation(
        Quat::from_xyzw(q.x,q.y,q.z,q.w), Vec3::from_array(body.pose.translation.to_array()))
        .to_cols_array_2d()
}
fn geometry_id(id: u64) -> u32 { 0x8000_0000 | (id as u32 & 0x7fff_ffff) }

/// One moving body's authored mesh: world-space native triangles at the
/// body's current placement, with their union bounds.
#[derive(Clone)]
pub(crate) struct MeshQuery {
    pub id: u64,
    pub frame: [[f32; 4]; 4],
    /// The body's linear velocity (contacts use the volume's relative motion).
    pub velocity: Vector3,
    pub triangles: Arc<PreparedTriangles>,
    pub bounds: Bounds,
}
impl MeshQuery {
    pub(crate) fn new(body: &SolidBody, triangles: Arc<PreparedTriangles>) -> Option<Self> {
        let bounds = Bounds::from_points(
            triangles.triangle_bounds().iter().flat_map(|bounds| [bounds.min, bounds.max]))?;
        Some(Self { id: body.id, frame: body_frame(body), velocity: native(body.linvel.to_array()), triangles, bounds })
    }
    /// Native triangles whose bounds overlap `bounds`, in native order.
    pub(crate) fn candidates(&self, bounds: Bounds)
        -> impl Iterator<Item = &skate3_core::physics::board_world::WorldTriangle> {
        let overlaps = self.bounds.overlaps(bounds);
        self.triangles.triangles().iter().zip(self.triangles.triangle_bounds())
            .filter(move |(_, triangle)| overlaps && triangle.overlaps(bounds))
            .map(|(triangle, _)| triangle)
    }
}

pub(crate) struct MovingQueries {
    /// Bodies answered through their solid Parry shapes.
    solids: Vec<SolidBody>,
    meshes: Vec<MeshQuery>,
}
impl MovingQueries {
    /// `meshes` name bodies of `bodies`; those answer only through their mesh.
    pub(crate) fn new(bodies: Vec<SolidBody>, meshes: Vec<MeshQuery>) -> Self {
        let solids = bodies.into_iter().filter(|body| !meshes.iter().any(|mesh| mesh.id == body.id)).collect();
        Self { solids, meshes }
    }
}
impl ExternalQueries for MovingQueries {
    fn line(&self, start: Vector3, end: Vector3, radius: f32) -> Option<ExternalLineHit> {
        let mut nearest = sweep_sphere(&self.solids, xyz(start), xyz(end), radius).and_then(|(id, c)| {
            let body = self.solids.iter().find(|b| b.id == id)?;
            Some(ExternalLineHit {
                hit: WorldLineHit { geometry: TriangleLineHit {
                    position: native(c.point_b.to_array()), normal: native(c.normal.to_array()),
                    fraction: c.time_of_impact, volume_parameter: [0.;3],
                }, tag: 0 }, surface: 0, geometry_id: geometry_id(id),
                frame: body_frame(body),
            })
        });
        if self.meshes.is_empty() || !radius.is_finite() || radius < 0. { return nearest; }
        let Some(segment) = Bounds::from_points([start, end]) else { return nearest };
        // Thin-triangle leaves accept slightly negative barycentrics; pad as
        // the static world's line candidates do.
        let segment = segment.expanded(radius + 0.01);
        let direction = Vector3::new(end.x - start.x, end.y - start.y, end.z - start.z);
        for mesh in &self.meshes {
            for entry in mesh.candidates(segment) {
                let mut geometry = TriangleLineHit {
                    position: Vector3::ZERO, normal: Vector3::ZERO, fraction: 0., volume_parameter: [0.; 3],
                };
                if triangle_segment(&mut geometry, start, direction, entry.triangle.vertices, radius, entry.triangle.fatness)
                    && nearest.as_ref().is_none_or(|hit| geometry.fraction < hit.hit.geometry.fraction)
                {
                    nearest = Some(ExternalLineHit {
                        hit: WorldLineHit { geometry, tag: entry.tag },
                        surface: entry.tag as u16,
                        geometry_id: geometry_id(mesh.id),
                        frame: mesh.frame,
                    });
                }
            }
        }
        nearest
    }
    fn nearby(&self, center: Vector3, radius: f32) -> Vec<[Vector3;3]> {
        let point = Vec3::from_array(xyz(center));
        if !point.is_finite() || !radius.is_finite() || radius < 0. { return Vec::new(); }
        let mut triangles = Vec::new();
        let within = |min: Vec3, max: Vec3| point.distance_squared(point.clamp(min, max)) <= radius * radius;
        let query = Bounds { min: center, max: center }.expanded(radius);
        for mesh in &self.meshes {
            for entry in mesh.candidates(query) {
                let verts = entry.triangle.vertices.map(|v| Vec3::new(v.x, v.y, v.z));
                if within(verts[0].min(verts[1]).min(verts[2]), verts[0].max(verts[1]).max(verts[2])) {
                    triangles.push(entry.triangle.vertices);
                    if triangles.len() >= NEARBY_CAPACITY { return triangles; }
                }
            }
        }
        for body in &self.solids {
            for collider in &body.colliders {
                let aabb = collider.shape.compute_aabb(&collider.pose);
                if !within(Vec3::from_array(aabb.mins.to_array()), Vec3::from_array(aabb.maxs.to_array())) { continue; }
                for t in collider_triangles(collider) {
                    let verts = t.map(Vec3::from_array);
                    if within(verts[0].min(verts[1]).min(verts[2]), verts[0].max(verts[1]).max(verts[2])) {
                        triangles.push(t.map(native));
                        if triangles.len() >= NEARBY_CAPACITY { return triangles; }
                    }
                }
            }
        }
        triangles
    }
}
