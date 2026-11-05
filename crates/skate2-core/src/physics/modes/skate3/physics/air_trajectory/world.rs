//! Original trajectory collision leaf82771D08 and nearby triangle collector
//!82772028, across the host's world-format boundary. BoardWorld stores static
//!world-space triangles, so their source geometry transform is identity.
use skate3_core::{
    air::trajectory::SurfaceHit,
    math::Vector3,
    physics::{
        board_world::BoardWorld,
        triangle_query::{TriangleLineHit, triangle_segment},
    },
};
type Vector = [f32; 4];
const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];

pub(super) fn line(
    world: &BoardWorld,
    start: Vector,
    end: Vector,
    radius: f32,
) -> Result<Option<SurfaceHit>, String> {
    if !radius.is_finite() || radius < 0.0 || start.into_iter().chain(end).any(|v| !v.is_finite()) {
        return Err("Non-finite trajectory collision request or invalid radius".into());
    }
    let start = vec3(start);
    let delta = Vector3::new(end[0] - start.x, end[1] - start.y, end[2] - start.z);
    let mut nearest = f32::MAX;
    let mut result = None;
    for (index, entry) in world.line_candidates(start, vec3(end), radius) {
        let mut hit = TriangleLineHit {
            position: Vector3::ZERO,
            normal: Vector3::ZERO,
            fraction: 0.0,
            volume_parameter: [0.0; 3],
        };
        if triangle_segment(&mut hit, start, delta, entry.triangle.vertices, radius, 0.0) {
            let lower = if -hit.fraction >= 0.0 {
                0.0
            } else {
                hit.fraction
            };
            let fraction = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
            if fraction < nearest {
                nearest = fraction;
                result = Some(SurfaceHit {
                    position: lanes(hit.position),
                    normal: lanes(entry.triangle.feature.normal),
                    transform: IDENTITY,
                    surface: entry.tag,
                    geometry: u32::try_from(index).map_err(|_| "World geometry index overflow")?,
                });
            }
        }
    }
    if let Some(external) = world.external_line(start, vec3(end), radius) {
        let hit = external.hit.geometry;
        if hit.fraction.clamp(0.0, 1.0) < nearest {
            result = Some(SurfaceHit {
                position: lanes(hit.position), normal: lanes(hit.normal),
                transform: external.frame, surface: external.hit.tag,
                geometry: external.geometry_id,
            });
        }
    }
    Ok(result)
}
pub(super) fn nearby(
    world: &BoardWorld,
    center: Vector,
    radius: f32,
) -> Result<Vec<[Vector; 3]>, String> {
    if !radius.is_finite() || radius < 0.0 {
        return Err("Invalid nearby-trajectory triangle radius".into());
    }
    let minimum = std::array::from_fn(|i| center[i] - radius);
    let maximum = std::array::from_fn(|i| center[i] + radius);
    let mut triangles = Vec::with_capacity(64);
    for (_, entry) in world.line_candidates(vec3(center), vec3(center), radius) {
        let vertices = entry.triangle.vertices.map(lanes);
        if triangle_box(vertices, minimum, maximum) {
            triangles.push(vertices);
            if triangles.len() == 64 {
                break;
            }
        }
    }
    if triangles.len() < 64 {
        for vertices in world.external_nearby(vec3(center), radius) {
            let vertices = vertices.map(lanes);
            if triangle_box(vertices, minimum, maximum) {
                triangles.push(vertices);
                if triangles.len() == 64 { break; }
            }
        }
    }
    Ok(triangles)
}
///82761698 full separating-axis triangle/AABB predicate: nine edge-axis
///crosses, three box axes and the face plane. No broadphase-only substitution.
fn triangle_box(vertices: [Vector; 3], minimum: Vector, maximum: Vector) -> bool {
    let center: Vector = std::array::from_fn(|i| (minimum[i] + maximum[i]) * 0.5);
    let half: Vector = std::array::from_fn(|i| (maximum[i] - minimum[i]) * 0.5);
    let v = vertices.map(|p| sub(p, center));
    let edges = [sub(v[1], v[0]), sub(v[2], v[1]), sub(v[0], v[2])];
    let axes = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    for edge in edges {
        for axis in axes {
            if separated(v, cross(edge, axis), half) {
                return false;
            }
        }
    }
    for axis in axes {
        if separated(v, axis, half) {
            return false;
        }
    }
    !separated(v, cross(edges[0], edges[1]), half)
}
fn separated(vertices: [Vector; 3], axis: Vector, half: Vector) -> bool {
    let projections = vertices.map(|v| dot(v, axis));
    let min = projections[0].min(projections[1]).min(projections[2]);
    let max = projections[0].max(projections[1]).max(projections[2]);
    let radius = (half[0] * axis[0].abs() + half[1] * axis[1].abs()) + half[2] * axis[2].abs();
    min > radius || max < -radius
}
fn dot(a: Vector, b: Vector) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.0,
    ]
}
fn vec3(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn lanes(v: Vector3) -> Vector {
    [v.x, v.y, v.z, 0.0]
}
