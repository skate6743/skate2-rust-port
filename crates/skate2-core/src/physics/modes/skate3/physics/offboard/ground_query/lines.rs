use super::{
    Bounds, Mesh, Scene,
    transform::{self, sub},
};
use skate3_core::{
    math::Vector3,
    physics::triangle_query::{TriangleLineHit, triangle_segment},
    player::offboard::ground_query::{GroundQueryPacket, LineHit},
};
pub(super) fn query(
    scene: &Scene<'_>,
    packet: &GroundQueryPacket,
) -> Result<[Option<LineHit>; 7], &'static str> {
    let pools = [
        scene.ground_pool,
        scene.island_pool,
        if scene.island_flags == 3 {
            scene.conditional_pool
        } else {
            &[]
        },
    ];
    query_pools(pools, packet)
}

pub(super) fn query_pools(
    pools: [&[Mesh<'_>]; 3],
    packet: &GroundQueryPacket,
) -> Result<[Option<LineHit>; 7], &'static str> {
    for pool in pools {
        for mesh in pool {
            if mesh.triangles.len() != mesh.surfaces.len() {
                return Err("Biped query mesh lacks exact per-triangle packed surfaces");
            }
        }
    }
    let mut output = [None; 7];
    for (index, line) in packet.lines.iter().enumerate() {
        if !line.radius.is_finite() || line.radius < 0. {
            return Err("Invalid Biped line radius");
        }
        let delta = sub(line.end, line.start);
        let threshold = f32::from_bits(0x37800000);
        if !(delta.x.abs() > threshold || delta.y.abs() > threshold || delta.z.abs() > threshold) {
            continue;
        }
        let low = |a: f32, b: f32| a.min(b) - line.radius;
        let high = |a: f32, b: f32| a.max(b) + line.radius;
        let bounds = Bounds {
            min: Vector3::new(
                low(line.start.x, line.end.x),
                low(line.start.y, line.end.y),
                low(line.start.z, line.end.z),
            ),
            max: Vector3::new(
                high(line.start.x, line.end.x),
                high(line.start.y, line.end.y),
                high(line.start.z, line.end.z),
            ),
        };
        let mut nearest = f32::MAX;
        for pool in pools {
            for mesh in pool {
                if !transform::matches(packet.context.matching_id_2952, mesh.matching_group)
                    || !transform::overlaps(
                        transform::bounds(mesh.world_to_local, bounds),
                        mesh.local_bounds,
                    )
                {
                    continue;
                }
                for (triangle, &packed_surface) in mesh.triangles.iter().zip(mesh.surfaces) {
                    let vertices = triangle
                        .triangle
                        .vertices
                        .map(|p| transform::point(mesh.local_to_world, p));
                    let mut hit = TriangleLineHit {
                        position: Vector3::ZERO,
                        normal: Vector3::ZERO,
                        fraction: 0.,
                        volume_parameter: [0.; 3],
                    };
                    if triangle_segment(&mut hit, line.start, delta, vertices, line.radius, 0.)
                    {
                        // Native outer leaf uses fsel, including its unordered choice.
                        let lower = if -hit.fraction >= 0. {
                            0.
                        } else {
                            hit.fraction
                        };
                        let fraction = if 1. - lower >= 0. { lower } else { 1. };
                        if fraction < nearest {
                            nearest = fraction;
                            output[index] = Some(LineHit {
                                position: hit.position,
                                face_normal: transform::face(vertices),
                                fraction,
                                packed_surface,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(output)
}
