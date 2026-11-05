//! Static host for original S3 82C20728 and 82E0AAD0 line batches.
//! Both use scene mask7, matching ID2952, side flags3, rejection mask0 and
//! a null surface-exclusion table. Metadata is mandatory, never render tags.
use skate3_core::{
    air::trajectory::grind_surface::{Probe, ProbeHit},
    math::Vector3,
    physics::{
        board_world::{
            BoardWorld,
            query_metadata::{Bounds, QueryPool},
        },
        grind_contact::balance::{ForceExitHit, ForceExitProbe},
        triangle_query::{TriangleLineHit, triangle_segment},
    },
};

#[cfg(any())]
#[path = "world_tests.rs"]
mod tests;

///82C205D8/82C20728: descriptor5 has radius .001; the others have radius0.
/// The descriptor supplies its radius, so both thin and swept leaves execute.
pub(crate) fn surface_probe(
    world: &BoardWorld,
    actor: [u32; 2],
    index: usize,
    probe: Probe,
) -> Result<Option<ProbeHit>, String> {
    if index >= 7 {
        return Err(format!(
            "Grind surface descriptor {index} is outside native batch7"
        ));
    }
    line(world, actor[1] as i32, probe)
}

///82D86C88 passes processed2952 into82E0AAD0, not actor identity2948.
pub(crate) fn force_exit_line(
    world: &BoardWorld,
    actor: [u32; 2],
    probe: ForceExitProbe,
) -> Result<Option<ForceExitHit>, String> {
    Ok(line(
        world,
        actor[1] as i32,
        Probe {
            start: probe.start,
            end: probe.end,
            radius: 0.,
        },
    )?
    .map(|hit| ForceExitHit { normal: hit.normal }))
}

fn line(world: &BoardWorld, matching: i32, probe: Probe) -> Result<Option<ProbeHit>, String> {
    let metadata = world.query_metadata().map_err(str::to_owned)?;
    if !probe.radius.is_finite() || probe.radius < 0. {
        return Err("Invalid grind line radius".into());
    }
    let start = vector(probe.start);
    let end = vector(probe.end);
    let bounds = Bounds::from_points([start, end])
        .ok_or("Non-finite grind line endpoints")?
        .expanded(probe.radius);
    if ![
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err("Grind line bounds overflow".into());
    }
    let delta = sub(end, start);
    //82770650's component gate precedes dispatch, including rounded lines.
    let threshold = f32::from_bits(0x3780_0000);
    if !(delta.x.abs() > threshold || delta.y.abs() > threshold || delta.z.abs() > threshold) {
        return Ok(None);
    }
    //The index is conservative; native exact gates below remain authoritative.
    let candidates = world
        .candidate_mesh_indices(Some(bounds))
        .map_err(str::to_owned)?;
    //Current BoardWorld applies its stored per-triangle bounds filter after
    //cluster culling. Collect once in canonical order, then restrict each mesh
    //to its candidate slice instead of walking its entire triangle range.
    let triangle_candidates = world
        .line_candidates(start, end, probe.radius)
        .collect::<Vec<_>>();
    let mut nearest = f32::MAX;
    let mut output = None;
    //82764AB0 mask7 and8276EC40 aggregate nearest: ties retain earlier pools.
    for pool in [QueryPool::Ground, QueryPool::Island, QueryPool::Conditional] {
        if pool == QueryPool::Conditional && metadata.island_flags != 3 {
            continue;
        }
        for &mesh_index in &candidates {
            let mesh = &metadata.meshes[mesh_index];
            if mesh.pool != pool
                || !(matching == -1 || mesh.matching_group == -1 || matching == mesh.matching_group)
                || !bounds.overlaps(mesh.local_bounds)
            {
                continue;
            }
            //Request rejection mask0 deliberately accepts every rejection_flags
            //value; null exclusion table accepts every authored packed code.
            //BoardWorld validates identity transforms and world-space vertices.
            let first = triangle_candidates
                .partition_point(|&(index, _)| index < mesh.triangle_range.start);
            let last =
                triangle_candidates.partition_point(|&(index, _)| index < mesh.triangle_range.end);
            for &(triangle_index, triangle) in &triangle_candidates[first..last] {
                let vertices = triangle.triangle.vertices;
                let triangle_bounds =
                    Bounds::from_points(vertices).ok_or("Non-finite authored grind triangle")?;
                //82ACA170 decoder's per-volume AABB gate; no leaf tolerance pad.
                if !bounds.overlaps(triangle_bounds) {
                    continue;
                }
                let mut hit = TriangleLineHit {
                    position: Vector3::ZERO,
                    normal: Vector3::ZERO,
                    fraction: 0.,
                    volume_parameter: [0.; 3],
                };
                //827719B8 explicitly passes zero TRIANGLE fatness, irrespective
                //of the contact triangle's stored fatness. Not BoardWorld.query.
                if !triangle_segment(&mut hit, start, delta, vertices, probe.radius, 0.) {
                    continue;
                }
                let fraction = clamp_fraction(hit.fraction);
                if fraction < nearest {
                    nearest = fraction;
                    output = Some(ProbeHit {
                        fraction,
                        position: packed(hit.position),
                        //Decoded volume+16, NOT the rounded leaf's contact normal.
                        normal: packed(face(vertices)),
                        packed_surface: u32::from(metadata.packed_surfaces[triangle_index]),
                    });
                }
            }
        }
    }
    Ok(output)
}

fn vector(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn packed(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.]
}
fn sub(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

//827719B8's ordered fsel choices, including the unordered upper choice.
fn clamp_fraction(fraction: f32) -> f32 {
    let lower = if -fraction >= 0. { 0. } else { fraction };
    if 1. - lower >= 0. { lower } else { 1. }
}

//82ACA4E0..550, from decoded authored winding, with two rsqrt refinements.
//As in current core native_arithmetic, the PC seed is sqrt().recip(); this
//preserves host arithmetic policy, not bit-exact Xenon estimate emulation.
fn face([a, b, c]: [Vector3; 3]) -> Vector3 {
    let u = sub(b, a);
    let v = sub(c, a);
    let n = Vector3::new(
        (-u.z).mul_add(v.y, u.y * v.z),
        (-u.x).mul_add(v.z, u.z * v.x),
        (-u.y).mul_add(v.x, u.x * v.y),
    );
    let q = n.z.mul_add(n.z, n.y.mul_add(n.y, n.x * n.x));
    let mut r = q.sqrt().recip();
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-q).mul_add(r * r, 1.), r);
    }
    Vector3::new(n.x * r, n.y * r, n.z * r)
}
