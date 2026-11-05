//!82770B40/82771D08 mesh traversal and82770E00/82772028 nearby collection.
//!Authored map pools plus the host's immutable solid moving-geometry provider.
use skate3_core::{
    air::trajectory::{QueryRequest, QueryResult, SurfaceHit, query_trajectory},
    math::Vector3,
    physics::{
        board_world::{
            BoardWorld,
            query_metadata::{QueryMetadata, QueryPool},
        },
        triangle_query::{TriangleLineHit, triangle_segment},
    },
    player::offboard::contact_toolkit::{
        Batch, LineHit, LineProbe, QueryResults, Scene, Vector, query_sweep,
    },
};

pub struct StaticScene<'a> {
    world: &'a BoardWorld,
    metadata: &'a QueryMetadata,
}
impl<'a> StaticScene<'a> {
    ///82C20728/82764AB0: caller-sized lines, poolmask7, facing3, no mesh or
    ///material exclusion. In particular Air submits SIX lines, Ground seven.
    pub(crate) fn lines(
        &self,
        requests: &[skate3_core::player::offboard::ground_query::Line],
        matching_group: i32,
    ) -> Result<Vec<Option<skate3_core::player::offboard::ground_query::LineHit>>, &'static str>
    {
        use skate3_core::player::offboard::ground_query::LineHit as GroundHit;
        let mut output: Vec<Option<GroundHit>> = vec![None; requests.len()];
        for pool in self.pools() {
            for (request, destination) in requests.iter().zip(&mut output) {
                let probe = LineProbe {
                    start: lanes(request.start),
                    end: lanes(request.end),
                    radius: request.radius,
                };
                if let Some(hit) = self.line(pool, probe, matching_group, 0)? {
                    if destination
                        .as_ref()
                        .is_none_or(|old| hit.fraction < old.fraction)
                    {
                        *destination = Some(GroundHit {
                            position: vec3(hit.position),
                            face_normal: vec3(hit.normal),
                            fraction: hit.fraction,
                            packed_surface: hit.surface,
                        });
                    }
                }
            }
        }
        Ok(output)
    }
    /// 82764CF8 pool mask 7; 82770910 walks the actual accelerated trajectory.
    /// 8276EE88 retains the first pool on equal contact times. Callers supply
    /// the actor matching group and the original per-request mesh rejection mask.
    pub(crate) fn trajectory(
        &self,
        request: QueryRequest,
        matching_group: i32,
        mesh_reject_mask: u32,
    ) -> Result<QueryResult, &'static str> {
        let t = request.trajectory;
        if !t.duration.is_finite()
            || t.duration <= 0.
            || !request.radius.is_finite()
            || request.radius <= 0.
            || !request.start_error.is_finite()
            || request.start_error <= 0.
            || !request.end_error.is_finite()
            || request.end_error <= 0.
            || t.position
                .into_iter()
                .chain(t.velocity)
                .chain(t.acceleration)
                .any(|value| !value.is_finite())
        {
            return Err("Invalid offboard accelerated-trajectory request");
        }
        let mut nearest = QueryResult::miss();
        for pool in self.pools() {
            let result = query_trajectory(
                request,
                |start, end, radius| {
                    self.line(
                        pool,
                        LineProbe { start, end, radius },
                        matching_group,
                        mesh_reject_mask,
                    )
                    .map(|hit| {
                        hit.map(|h| SurfaceHit {
                            position: h.position,
                            normal: h.normal,
                            transform: h.mesh_frame,
                            surface: h.surface as u32,
                            geometry: h.geometry,
                        })
                    })
                },
                |center, radius| self.nearby(pool, center, radius, matching_group),
            )?;
            if result.valid() && (!nearest.valid() || result.contact_time < nearest.contact_time) {
                nearest = result;
            }
        }
        Ok(nearest)
    }
    pub fn new(world: &'a BoardWorld) -> Result<Self, &'static str> {
        let metadata = world.query_metadata()?;
        Ok(Self { world, metadata })
    }
    fn pools(&self) -> impl Iterator<Item = QueryPool> {
        [
            Some(QueryPool::Ground),
            Some(QueryPool::Island),
            (self.metadata.island_flags == 3).then_some(QueryPool::Conditional),
        ]
        .into_iter()
        .flatten()
    }
    fn line(
        &self,
        pool: QueryPool,
        probe: LineProbe,
        group: i32,
        reject: u32,
    ) -> Result<Option<LineHit>, &'static str> {
        if !probe.radius.is_finite()
            || probe.radius < 0.
            // The native packet has four SIMD lanes, but static collision and
            // query_sweep's dot3/length contract consume XYZ only. Lane 3 can
            // carry non-finite intermediate values (e.g. inf - inf) without
            // making the spatial segment invalid. Never relax XYZ/radius checks.
            || probe.start[..3].iter().chain(&probe.end[..3]).any(|x| !x.is_finite())
        {
            // Temporary audit-only error detail: preserve the rejected producer
            // packet and caller chain. Invalid spatial values are never substituted.
            crate::skate3_native::trace::error!(
                "OFFBOARD_INVALID_SWEEP start={:?} end={:?} radius={} start_bits={:08x?} end_bits={:08x?} radius_bits={:08x} group={} reject={:08x}",
                probe.start,
                probe.end,
                probe.radius,
                probe.start.map(f32::to_bits),
                probe.end.map(f32::to_bits),
                probe.radius.to_bits(),
                group,
                reject,
            );
            return Err("Invalid offboard swept-line request");
        }
        let delta = sub(probe.end, probe.start);
        if !delta[..3]
            .iter()
            .any(|x| x.abs() > f32::from_bits(0x37800000))
        {
            return Ok(None);
        }
        let start = vec3(probe.start);
        let direction = vec3(delta);
        let mut nearest = f32::MAX;
        let mut output = None;
        let bounds = self
            .world
            .line_candidate_bounds(start, vec3(probe.end), probe.radius);
        let candidates: Vec<_> = self
            .world
            .line_candidates(start, vec3(probe.end), probe.radius)
            .map(|(index, _)| index)
            .collect();
        for mesh_index in self.world.candidate_mesh_indices(bounds)? {
            let mesh = &self.metadata.meshes[mesh_index];
            if mesh.pool != pool
                || !matches(group, mesh.matching_group)
                || mesh.rejection_flags & reject != 0
            {
                continue;
            }
            let first = candidates.partition_point(|&i| i < mesh.triangle_range.start);
            let last = candidates.partition_point(|&i| i < mesh.triangle_range.end);
            for &index in &candidates[first..last] {
                let vertices = self.world.triangles()[index].triangle.vertices;
                let mut hit = TriangleLineHit {
                    position: Vector3::ZERO,
                    normal: Vector3::ZERO,
                    fraction: 0.,
                    volume_parameter: [0.; 3],
                };
                if !triangle_segment(&mut hit, start, direction, vertices, probe.radius, 0.) {
                    continue;
                }
                // 82771D08 retains the authored packed surface, including water.
                // State-specific handling belongs to ProcessOutput82DB8120.
                let lower = if -hit.fraction >= 0. {
                    0.
                } else {
                    hit.fraction
                };
                let fraction = if 1. - lower >= 0. { lower } else { 1. };
                if fraction < nearest {
                    nearest = fraction;
                    let transform = mesh.local_to_world;
                    output = Some(LineHit {
                        position: lanes(hit.position),
                        normal: face(vertices.map(lanes)),
                        fraction,
                        surface: self.metadata.packed_surfaces[index],
                        geometry: mesh.geometry,
                        mesh_frame: [
                            vector4(transform.basis.columns[0]),
                            vector4(transform.basis.columns[1]),
                            vector4(transform.basis.columns[2]),
                            lanes(transform.translation),
                        ],
                    });
                }
            }
        }
        if pool == QueryPool::Ground
            && let Some(hit) = self.world.external_line(start, vec3(probe.end), probe.radius)
        {
            if hit.hit.geometry.fraction < nearest {
                output = Some(LineHit {
                    position: lanes(hit.hit.geometry.position),
                    normal: lanes(hit.hit.geometry.normal),
                    fraction: hit.hit.geometry.fraction,
                    surface: hit.surface,
                    geometry: hit.geometry_id,
                    mesh_frame: hit.frame,
                });
            }
        }
        Ok(output)
    }
    fn nearby(
        &self,
        pool: QueryPool,
        center: Vector,
        radius: f32,
        group: i32,
    ) -> Result<Vec<[Vector; 3]>, &'static str> {
        let mut output = if pool == QueryPool::Ground {
            self.world
                .external_nearby(vec3(center), radius)
                .into_iter()
                .take(64)
                .map(|triangle| triangle.map(lanes))
                .collect::<Vec<_>>()
        } else {
            Vec::with_capacity(64)
        };
        if output.len() >= 64 { return Ok(output); }
        //82770E00 does not apply the trajectory mesh rejection mask here.
        let bounds = self
            .world
            .line_candidate_bounds(vec3(center), vec3(center), radius);
        let candidates: Vec<_> = self
            .world
            .line_candidates(vec3(center), vec3(center), radius)
            .map(|(index, _)| index)
            .collect();
        for mesh_index in self.world.candidate_mesh_indices(bounds)? {
            let mesh = &self.metadata.meshes[mesh_index];
            if mesh.pool != pool || !matches(group, mesh.matching_group) {
                continue;
            }
            let first = candidates.partition_point(|&i| i < mesh.triangle_range.start);
            let last = candidates.partition_point(|&i| i < mesh.triangle_range.end);
            for &index in &candidates[first..last] {
                let vertices = self.world.triangles()[index].triangle.vertices.map(lanes);
                if triangle_box(vertices, center, radius) {
                    output.push(vertices);
                    if output.len() == 64 {
                        return Ok(output);
                    }
                }
            }
        }
        Ok(output)
    }
}
impl Scene for StaticScene<'_> {
    type Error = &'static str;
    fn execute(&self, batch: &Batch) -> Result<QueryResults, Self::Error> {
        let mut trajectories = [QueryResult::miss(); 3];
        let mut lines = vec![None; batch.lines.len()];
        for pool in self.pools() {
            //8276EE88 chooses strict smallest nonnegative time, retaining the
            //first pool on ties. Normal refinement belongs to that same pool.
            for (request, output) in batch.trajectories.iter().zip(&mut trajectories) {
                let start = request.trajectory.position;
                let end = std::array::from_fn(|i| start[i] + request.trajectory.velocity[i]);
                let result = query_sweep(
                    LineProbe {
                        start,
                        end,
                        radius: request.radius,
                    },
                    |probe| self.line(pool, probe, batch.matching_group, batch.mesh_reject_mask),
                    |center, radius| self.nearby(pool, center, radius, batch.matching_group),
                )?;
                if result.valid() && (!output.valid() || result.contact_time < output.contact_time)
                {
                    *output = result;
                }
            }
            for (probe, output) in batch.lines.iter().zip(&mut lines) {
                if let Some(hit) =
                    self.line(pool, *probe, batch.matching_group, batch.mesh_reject_mask)?
                {
                    if output
                        .as_ref()
                        .is_none_or(|old: &LineHit| hit.fraction < old.fraction)
                    {
                        *output = Some(hit);
                    }
                }
            }
        }
        Ok(QueryResults {
            trajectories,
            lines,
            edges: self.edges(batch),
        })
    }
}
impl StaticScene<'_> {
    /// 82D81610 bounds and 82C1EAD8 ordinary static-edge traversal/capacity.
    fn edges(&self, batch: &Batch) -> Vec<[Vector; 2]> {
        let input = batch.input;
        let center: Vector = std::array::from_fn(|i| {
            input.up[i].mul_add(
                0.29999998,
                input.forward[i].mul_add(0.89999998, input.position[i]),
            )
        });
        let extent: Vector = std::array::from_fn(|i| {
            input.forward[i].abs().mul_add(
                0.89999998,
                input.up[i]
                    .abs()
                    .mul_add(1.1, input.right[i].abs() * 0.037500001),
            )
        });
        self.metadata
            .static_edges
            .iter()
            .filter(|edge| {
                let min = lanes(edge.local_bounds.min);
                let max = lanes(edge.local_bounds.max);
                (0..3).all(|i| center[i] - extent[i] <= max[i] && min[i] <= center[i] + extent[i])
            })
            .take(40)
            .map(|edge| [lanes(edge.start), lanes(edge.end)])
            .collect()
    }
}
fn vector4(v: [f32; 3]) -> Vector {
    [v[0], v[1], v[2], 0.]
}
fn matches(a: i32, b: i32) -> bool {
    a == -1 || b == -1 || a == b
}
fn vec3(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn lanes(v: Vector3) -> Vector {
    [v.x, v.y, v.z, 0.]
}
fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.,
    ]
}
fn face([a, b, c]: [Vector; 3]) -> Vector {
    skate3_core::player::offboard::contact_toolkit::triangle_normal([a, b, c])
}
///82761698: full edge-cross-axis, coordinate-axis, and face-plane separation.
///AABB overlap alone must not select a face for normal refinement.
fn triangle_box(vertices: [Vector; 3], center: Vector, radius: f32) -> bool {
    let v = vertices.map(|p| sub(p, center));
    let edges = [sub(v[1], v[0]), sub(v[2], v[1]), sub(v[0], v[2])];
    let axes = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]];
    let separated = |axis: Vector| {
        let p = v.map(|x| dot(x, axis));
        let extent = axis[2].abs().mul_add(
            radius,
            axis[1].abs().mul_add(radius, axis[0].abs() * radius),
        );
        p[0].min(p[1]).min(p[2]) > extent || p[0].max(p[1]).max(p[2]) < -extent
    };
    for edge in edges {
        for axis in axes {
            if separated(cross(edge, axis)) {
                return false;
            }
        }
    }
    for axis in axes {
        if separated(axis) {
            return false;
        }
    }
    !separated(cross(edges[0], edges[1]))
}
