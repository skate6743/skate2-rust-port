//! Camera triangle leaf82771D08 stores the authored face normal, clamps every
//! fraction and retains the first equal-distance hit in world traversal order.
use skate3_core::{camera::FatLineResult, math::Vector3,
    physics::{board_world::BoardWorld, triangle_query::{TriangleLineHit, triangle_segment}}};

pub(super) fn line(world: &BoardWorld, start: [f32; 4], end: [f32; 4], radius: f32)
    -> Result<FatLineResult, String> {
    if !radius.is_finite() || radius < 0.0 {
        return Err("Invalid stock camera collision radius".into());
    }
    let start = vector(start);
    let delta = Vector3::new(end[0] - start.x, end[1] - start.y, end[2] - start.z);
    let mut fraction = f32::MAX;
    let mut result = no_hit();
    for (_, entry) in world.line_candidates(start, vector(end), radius) {
        let mut hit = TriangleLineHit { position: Vector3::ZERO, normal: Vector3::ZERO,
            fraction: 0.0, volume_parameter: [0.0; 3] };
        // Zero triangle fatness and zero material exclusion mask are native.
        // Subject actor exclusion is preserved by BoardWorld's world-only data.
        if triangle_segment(&mut hit, start, delta, entry.triangle.vertices, radius, 0.0) {
            let lower = if -hit.fraction >= 0.0 { 0.0 } else { hit.fraction };
            let candidate = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
            if candidate < fraction {
                fraction = candidate;
                result = FatLineResult { position: lanes(hit.position),
                    normal: lanes(entry.triangle.feature.normal), fraction: candidate,
                    hit: 1, surface: entry.tag };
            }
        }
    }
    if let Some(external) = world.external_line(start, vector(end), radius) {
        let hit = external.hit.geometry;
        let candidate = hit.fraction.clamp(0.0, 1.0);
        if candidate < fraction {
            result = FatLineResult { position: lanes(hit.position), normal: lanes(hit.normal),
                fraction: candidate, hit: 1, surface: external.hit.tag };
        }
    }
    Ok(result)
}
pub(super) fn no_hit() -> FatLineResult {
    FatLineResult { position: [0.0; 4], normal: [0.0; 4], fraction: 0.0, hit: 0, surface: 0 }
}
fn vector(v: [f32; 4]) -> Vector3 { Vector3::new(v[0], v[1], v[2]) }
fn lanes(v: Vector3) -> [f32; 4] { [v.x, v.y, v.z, 0.0] }
