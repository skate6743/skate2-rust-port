//! Scene boundary for the recovered BipedToolkit probe layout. The pending
//! packet owns observations; the world's geometry and acceleration tree stay shared.
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{BoardWorld, query_metadata::QueryPool},
        drive_frames::RetailAffineTransform,
        triangle_query::{TriangleLineHit, triangle_segment},
    },
};

#[derive(Clone, Copy)]
pub(crate) struct Probe {
    pub start: [f32; 4],
    pub end: [f32; 4],
    pub radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Hit {
    pub geometry: TriangleLineHit,
    pub packed_surface: u16,
    pub mesh_index: usize,
    pub support_frame: RetailAffineTransform,
}

fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
pub(crate) fn query(world: &BoardWorld, probe: Probe, matching_id: u32) -> Result<Option<Hit>, String> {
    let metadata = world.query_metadata().map_err(str::to_owned)?;
    let start = xyz(probe.start);
    let end = xyz(probe.end);
    let delta = Vector3::new(end.x - start.x, end.y - start.y, end.z - start.z);
    let mut nearest: Option<Hit> = None;
    // Accelerate the canonical identity-transform map representation. Authored
    // transformed meshes retain the same geometry path without incorrect culling.
    let candidates: Vec<_> = if metadata
        .meshes
        .iter()
        .all(|m| m.local_to_world == RetailAffineTransform::IDENTITY)
    {
        world
            .line_candidates(start, end, probe.radius)
            .map(|(i, _)| i)
            .collect()
    } else {
        (0..world.triangles().len()).collect()
    };
    for pool in [QueryPool::Ground, QueryPool::Island, QueryPool::Conditional] {
        if pool == QueryPool::Conditional && metadata.island_flags != 3 {
            continue;
        }
        for (mesh_index, mesh) in metadata.meshes.iter().enumerate() {
            if mesh.pool != pool
                || !(mesh.matching_group == -1 || mesh.matching_group == matching_id as i32)
            {
                continue;
            }
            let first = candidates.partition_point(|&i| i < mesh.triangle_range.start);
            let last = candidates.partition_point(|&i| i < mesh.triangle_range.end);
            for &index in &candidates[first..last] {
                let triangle = &world.triangles()[index].triangle;
                let frame = mesh.local_to_world;
                let vertices = triangle.vertices.map(|v| {
                    let t = frame.translation;
                    let b = frame.basis.columns;
                    Vector3::new(
                        b[2][0].mul_add(v.z, b[1][0].mul_add(v.y, b[0][0].mul_add(v.x, t.x))),
                        b[2][1].mul_add(v.z, b[1][1].mul_add(v.y, b[0][1].mul_add(v.x, t.y))),
                        b[2][2].mul_add(v.z, b[1][2].mul_add(v.y, b[0][2].mul_add(v.x, t.z))),
                    )
                });
                let mut geometry = TriangleLineHit {
                    position: Vector3::ZERO,
                    normal: Vector3::ZERO,
                    fraction: 0.,
                    volume_parameter: [0.; 3],
                };
                if triangle_segment(
                    &mut geometry,
                    start,
                    delta,
                    vertices,
                    probe.radius,
                    triangle.fatness,
                ) && nearest
                    .as_ref()
                    .is_none_or(|h| geometry.fraction < h.geometry.fraction)
                {
                    nearest = Some(Hit {
                        geometry,
                        packed_surface: metadata.packed_surfaces[index],
                        mesh_index,
                        support_frame: frame,
                    });
                }
            }
        }
    }
    Ok(nearest)
}
