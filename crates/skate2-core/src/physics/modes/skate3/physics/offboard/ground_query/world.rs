//! Ephemeral adapter over the canonical BoardWorld metadata. Geometry is borrowed.
use super::{Bounds, Frame, IndexedEdgeBody, Mesh, PrimaryEdges, Scene, Segment};
use skate3_core::player::offboard::ground_query::{
    EdgeSearch, GroundQueryPacket, GroundQueryScene, LineHit,
};
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{BoardWorld, query_metadata::QueryPool},
        drive_frames::RetailAffineTransform,
    },
    player::offboard::ground_query::Edge,
};

/// Static mesh candidates use the same ZIP hierarchy as the other world queries.
/// The existing ground leaf still owns transforms, exact bounds gates and hits.
pub struct WorldScene<'a> {
    world: &'a BoardWorld,
    edges: Scene<'a>,
}
impl GroundQueryScene for WorldScene<'_> {
    type Error = &'static str;
    fn edge_candidates(&mut self, search: &EdgeSearch) -> Result<Vec<Edge>, Self::Error> {
        self.edges.edge_candidates(search)
    }
    fn query_lines(
        &mut self,
        packet: &GroundQueryPacket,
    ) -> Result<[Option<LineHit>; 7], Self::Error> {
        let metadata = self.world.query_metadata()?;
        let triangles = self.world.triangles();
        let mut candidates = Vec::new();
        for line in &packet.lines {
            if !line.radius.is_finite() || line.radius < 0. {
                return Err("Invalid Biped line radius");
            }
            candidates.extend(
                self.world
                    .candidate_mesh_indices(self.world.line_candidate_bounds(
                        line.start,
                        line.end,
                        line.radius,
                    ))?,
            );
        }
        candidates.sort_unstable();
        candidates.dedup();
        let mut pools: [Vec<Mesh<'_>>; 3] = std::array::from_fn(|_| Vec::new());
        for index in candidates {
            let source = &metadata.meshes[index];
            let pool = match source.pool {
                QueryPool::Ground => 0,
                QueryPool::Island => 1,
                QueryPool::Conditional => 2,
            };
            pools[pool].push(Mesh {
                local_to_world: frame(source.local_to_world),
                world_to_local: frame(source.world_to_local),
                local_bounds: Bounds {
                    min: source.local_bounds.min,
                    max: source.local_bounds.max,
                },
                matching_group: source.matching_group,
                triangles: &triangles[source.triangle_range.clone()],
                surfaces: &metadata.packed_surfaces[source.triangle_range.clone()],
            });
        }
        super::lines::query_pools(
            [
                &pools[0],
                &pools[1],
                if metadata.island_flags == 3 {
                    &pools[2]
                } else {
                    &[]
                },
            ],
            packet,
        )
    }
}

/// Caller supplies actual dynamic-provider views from the same live scene owner.
/// Empty providers mean the authored scene actually has none, never a failed lookup.
pub fn with_world_scene<T>(
    world: &BoardWorld,
    primary_edges: PrimaryEdges<'_>,
    indexed_edges: &[IndexedEdgeBody<'_>],
    operation: impl FnOnce(&mut WorldScene<'_>) -> Result<T, &'static str>,
) -> Result<T, &'static str> {
    let metadata = world.query_metadata()?;
    let edges: Vec<_> = metadata
        .static_edges
        .iter()
        .map(|s| Segment {
            edge: Edge {
                start: s.start,
                end: s.end,
            },
            local_bounds: Bounds {
                min: s.local_bounds.min,
                max: s.local_bounds.max,
            },
        })
        .collect();
    let edges = Scene {
        ground_pool: &[],
        island_pool: &[],
        conditional_pool: &[],
        island_flags: metadata.island_flags,
        static_edges: &edges,
        primary_edges,
        indexed_edges,
    };
    let mut scene = WorldScene { world, edges };
    operation(&mut scene)
}
fn frame(f: RetailAffineTransform) -> Frame {
    let v = |a: [f32; 3]| Vector3::new(a[0], a[1], a[2]);
    Frame {
        right: v(f.basis.columns[0]),
        up: v(f.basis.columns[1]),
        forward: v(f.basis.columns[2]),
        position: f.translation,
    }
}
