//! Borrowed scene adapter for original Biped query82764AB0/82770650/82C1EAD8.
//! All geometry and live transforms belong to the canonical collision scene.
mod edges;
mod lines;
mod transform;
mod world;
use skate3_core::{
    math::Vector3,
    physics::board_world::WorldTriangle,
    player::offboard::ground_query::{
        Edge, EdgeSearch, Frame, GroundQueryPacket, GroundQueryScene, LineHit,
    },
};
pub use world::with_world_scene;
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: Vector3,
    pub max: Vector3,
}
/// A borrowed mesh from one canonical source pool, in canonical mesh order.
pub struct Mesh<'a> {
    pub local_to_world: Frame,
    pub world_to_local: Frame,
    pub local_bounds: Bounds,
    pub matching_group: i32,
    pub triangles: &'a [WorldTriangle],
    /// Exact packed source surface, one entry per triangle; never inferred from tag.
    pub surfaces: &'a [u16],
}
#[derive(Clone, Copy)]
pub struct Segment {
    pub edge: Edge,
    pub local_bounds: Bounds,
}
pub struct EdgeBody<'a> {
    pub local_to_world: Frame,
    pub local_bounds: Bounds,
    pub segments: &'a [Segment],
}
pub struct AlternateRecord<'a> {
    pub enabled: bool,
    /// Original record44 and48, selected by context2948 bit1.
    pub choices: [Option<(&'a EdgeBody<'a>, i32)>; 2],
}
pub enum PrimaryEdges<'a> {
    /// Original826C0630: dynamic provider, then vehicle provider.
    Normal {
        dynamic: &'a [EdgeBody<'a>],
        vehicles: &'a [EdgeBody<'a>],
    },
    Alternate(&'a [AlternateRecord<'a>]),
}
pub struct IndexedEdgeBody<'a> {
    pub id: u32,
    pub disabled: bool,
    pub body: &'a EdgeBody<'a>,
}
/// Ephemeral view only: owns no collision geometry, bodies, or cached observations.
pub struct Scene<'a> {
    pub ground_pool: &'a [Mesh<'a>],
    pub island_pool: &'a [Mesh<'a>],
    pub conditional_pool: &'a [Mesh<'a>],
    pub island_flags: u32,
    /// The grind data `GrindData::BoxTest` (82C1EAD8) walks before its moving providers: the
    /// world's and the props' grind spline segments.
    pub grind: Option<&'a crate::skate3_native::grind_world::StaticProvider>,
    pub static_edges: &'a [Segment],
    pub primary_edges: PrimaryEdges<'a>,
    pub indexed_edges: &'a [IndexedEdgeBody<'a>],
}
impl GroundQueryScene for Scene<'_> {
    type Error = &'static str;
    fn edge_candidates(&mut self, search: &EdgeSearch) -> Result<Vec<Edge>, Self::Error> {
        edges::candidates(self, search)
    }
    fn query_lines(
        &mut self,
        packet: &GroundQueryPacket,
    ) -> Result<[Option<LineHit>; 7], Self::Error> {
        lines::query(self, packet)
    }
}

#[cfg(any())]
mod tests;
