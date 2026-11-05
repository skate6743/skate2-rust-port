//! Actual static scene swept triangles. Assembly identity comes exclusively
//! from registered physical associations, never the mesh/triangle numeric ID.
use super::{Hit, Line, Scene};
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::query_metadata::QueryPool,
        triangle_query::{TriangleLineHit, triangle_segment},
    },
};
pub(super) fn line(scene: &Scene<'_>, line: Line) -> Result<Option<Hit>, String> {
    if line.radius < 0.0
        || !line.radius.is_finite()
        || line
            .start
            .into_iter()
            .chain(line.end)
            .any(|v| !v.is_finite())
    {
        return Err("Invalid grab validation swept line".into());
    }
    let delta = std::array::from_fn::<_, 4, _>(|i| line.end[i] - line.start[i]);
    if !delta[..3]
        .iter()
        .any(|v| v.abs() > f32::from_bits(0x37800000))
    {
        return Ok(None);
    }
    let metadata = scene.world.query_metadata().map_err(str::to_owned)?;
    let bounds = scene
        .world
        .line_candidate_bounds(xyz(line.start), xyz(line.end), line.radius);
    let meshes = scene
        .world
        .candidate_mesh_indices(bounds)
        .map_err(str::to_owned)?;
    let candidates: Vec<_> = scene
        .world
        .line_candidates(xyz(line.start), xyz(line.end), line.radius)
        .map(|(index, _)| index)
        .collect();
    let mut nearest = f32::MAX;
    let mut result = None;
    for (bit, pool) in [
        (1, QueryPool::Ground),
        (2, QueryPool::Island),
        (4, QueryPool::Conditional),
    ] {
        if line.source_pool_mask & bit == 0 {
            continue;
        }
        if pool == QueryPool::Conditional && metadata.island_flags != 3 {
            continue;
        }
        for &mesh_index in &meshes {
            let mesh = &metadata.meshes[mesh_index];
            if mesh.pool != pool
                || mesh.rejection_flags & line.reject_flags != 0
                || !(line.group == -1
                    || mesh.matching_group == -1
                    || line.group == mesh.matching_group)
            {
                continue;
            }
            let first = candidates.partition_point(|&i| i < mesh.triangle_range.start);
            let last = candidates.partition_point(|&i| i < mesh.triangle_range.end);
            for &index in &candidates[first..last] {
                let mut hit = TriangleLineHit {
                    position: Vector3::ZERO,
                    normal: Vector3::ZERO,
                    fraction: 0.0,
                    volume_parameter: [0.; 3],
                };
                let vertices = scene.world.triangles()[index].triangle.vertices;
                if !triangle_segment(
                    &mut hit,
                    xyz(line.start),
                    xyz(delta),
                    vertices,
                    line.radius,
                    0.0,
                ) {
                    continue;
                }
                let lower = if -hit.fraction >= 0.0 {
                    0.0
                } else {
                    hit.fraction
                };
                let fraction = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
                if fraction < nearest {
                    nearest = fraction;
                    result = Some(Hit {
                        fraction,
                        assembly: scene.registry.assembly(mesh.geometry),
                    });
                }
            }
        }
        // Same pool order and strict nearest comparison as the host World.
        // The host snapshot uses its existing ClusteredMesh line kernel.
        if let Some(host)=&scene.registry.host_lines {
            if let Some(hit)=host.line(Line { source_pool_mask: bit, ..line })? {
                if hit.fraction < nearest { nearest=hit.fraction; result=Some(hit); }
            }
        }
    }
    Ok(result)
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
