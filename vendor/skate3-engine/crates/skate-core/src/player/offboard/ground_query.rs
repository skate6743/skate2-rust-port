//! Original TU3 Biped ground geometry:82D32808,82C20728,82C20C08,82D31620.
//! Scene enumeration/collision is supplied explicitly by the canonical host.
//! The host must consume the previous Sync query during the next PreUpdate.
mod consume;
mod geometry;
mod math;
mod packet;
use crate::math::Vector3;
pub use consume::{consume_geometry, interpret_hits};
pub use geometry::{closest_point, edge_search, select_edge};
pub use packet::prepare_packet;
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub right: Vector3,
    pub up: Vector3,
    pub forward: Vector3,
    pub position: Vector3,
}
impl Frame {
    pub const IDENTITY: Self = Self {
        right: Vector3::new(1., 0., 0.),
        up: Vector3::new(0., 1., 0.),
        forward: Vector3::new(0., 0., 1.),
        position: Vector3::ZERO,
    };
}
#[derive(Clone, Copy, Debug)]
pub struct QueryContext {
    pub selection_flags_2948: u32,
    pub matching_id_2952: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub start: Vector3,
    pub end: Vector3,
}
#[derive(Clone, Copy, Debug)]
pub struct EdgeSearch {
    pub min: Vector3,
    pub max: Vector3,
    pub frame: Frame,
    pub context: QueryContext,
    pub narrow_forward: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct EdgeSelection {
    pub edge: Edge,
    pub closest: Vector3,
}
#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub start: Vector3,
    pub end: Vector3,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct GroundQueryPacket {
    pub context: QueryContext,
    pub center: Vector3,
    pub up: Vector3,
    pub tangent: Vector3,
    pub lines: [Line; 7],
}
impl GroundQueryPacket {
    ///82764AB0: Ground pool,21888 pool, and21892 only on island.flags==3.
    pub const SOURCE_POOL_MASK: u8 = 7;
    ///8276D510 adds no facing rejection; triangle winding rules still apply.
    pub const FACING_FLAGS: u8 = 3;
    pub const MESH_EXCLUSION_MASK: u32 = 0;
    pub const MATERIAL_EXCLUSION_MASK: [u32; 4] = [0; 4];
}
#[derive(Clone, Copy, Debug)]
pub struct LineHit {
    pub position: Vector3,
    pub face_normal: Vector3,
    pub fraction: f32,
    pub packed_surface: u16,
}
///Complete fields read by82D31620; not the unrelated general grind classifier.
#[derive(Clone, Copy, Debug)]
pub struct GroundGeometry {
    pub frame: Frame,
    pub kind: u32,
    pub flag26: bool,
    pub flag27: bool,
    pub flag28: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct ConsumeInput {
    pub frame_80: Frame,
    pub contact_position_192: Vector3,
    pub contact_flags_368: u32,
    pub reach_364: f32,
    pub previous_input_up_416: Vector3,
}
#[derive(Clone, Copy, Debug)]
pub struct GroundAdjustment {
    pub state_752: bool,
    pub state_753: bool,
    pub state_754: bool,
    pub frame_768: Frame,
    pub input_up_416: Vector3,
}
///The runtime implements actual authored edge/scene-body queries, never cached
///observations. Provider ordering, cap40, transforms and context filtering are
///specified in OFFBOARD_BIPED_GROUND_QUERY_HANDOFF.md.
pub trait GroundQueryScene {
    type Error;
    fn edge_candidates(&mut self, search: &EdgeSearch) -> Result<Vec<Edge>, Self::Error>;
    ///Return actual seven query results. Preserve source pool ordering, group
    ///matching, fatness0, decoded face normals and strict nearest-hit ties.
    fn query_lines(
        &mut self,
        packet: &GroundQueryPacket,
    ) -> Result<[Option<LineHit>; 7], Self::Error>;
}
#[cfg(test)]
mod tests;
