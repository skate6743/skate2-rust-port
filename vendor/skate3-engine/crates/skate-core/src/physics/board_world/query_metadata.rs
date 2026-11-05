//! Canonical authored scene metadata; independent of rendering tags and actors.
use super::WorldTriangle;
use crate::{math::Vector3, physics::drive_frames::RetailAffineTransform};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryPool {
    Ground,
    Island,
    Conditional,
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: Vector3,
    pub max: Vector3,
}
impl Bounds {
    pub fn expanded(self, padding: f32) -> Self {
        Self {
            min: Vector3::new(
                self.min.x - padding,
                self.min.y - padding,
                self.min.z - padding,
            ),
            max: Vector3::new(
                self.max.x + padding,
                self.max.y + padding,
                self.max.z + padding,
            ),
        }
    }
    pub fn overlaps(self, other: Self) -> bool {
        !(self.max.x < other.min.x
            || self.min.x > other.max.x
            || self.max.y < other.min.y
            || self.min.y > other.max.y
            || self.max.z < other.min.z
            || self.min.z > other.max.z)
    }
    pub fn from_points(points: impl IntoIterator<Item = Vector3>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        if !finite(first) {
            return None;
        }
        let mut bounds = Self {
            min: first,
            max: first,
        };
        for p in points {
            if !finite(p) {
                return None;
            }
            bounds.min = Vector3::new(
                bounds.min.x.min(p.x),
                bounds.min.y.min(p.y),
                bounds.min.z.min(p.z),
            );
            bounds.max = Vector3::new(
                bounds.max.x.max(p.x),
                bounds.max.y.max(p.y),
                bounds.max.z.max(p.z),
            );
        }
        Some(bounds)
    }
    fn contains(self, p: Vector3) -> bool {
        finite(p)
            && p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }
    pub(super) fn valid(self) -> bool {
        finite(self.min)
            && finite(self.max)
            && self.min.x <= self.max.x
            && self.min.y <= self.max.y
            && self.min.z <= self.max.z
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EdgeSegment {
    pub start: Vector3,
    pub end: Vector3,
    pub local_bounds: Bounds,
}

#[derive(Clone, Debug)]
pub struct QueryMesh {
    pub triangle_range: Range<usize>,
    pub local_to_world: RetailAffineTransform,
    pub world_to_local: RetailAffineTransform,
    pub local_bounds: Bounds,
    pub matching_group: i32,
    /// S3 mesh184: trajectory rejection mask, separate from surface/material.
    pub rejection_flags: u32,
    /// S3 mesh176 support/body identity; static registration uses0. This is
    /// separate from the host metadata index and authored mesh/unit names.
    pub geometry: u32,
    pub pool: QueryPool,
}

#[derive(Clone, Debug)]
pub struct QueryMetadata {
    /// Exact authored uint16 codes, in canonical triangle order. Low7 material,
    ///next5 category, upper4 flags; never synthesized from WorldTriangle::tag.
    pub packed_surfaces: Vec<u16>,
    pub meshes: Vec<QueryMesh>,
    /// Explicit level query edges; triangulation diagonals are not query edges.
    pub static_edges: Vec<EdgeSegment>,
    pub island_flags: u32,
}
impl QueryMetadata {
    pub(super) fn validate(&self, triangles: &[WorldTriangle]) -> Result<(), &'static str> {
        if self.packed_surfaces.len() != triangles.len() {
            return Err("Query surface count must equal canonical triangle count");
        }
        let mut next = 0;
        for mesh in &self.meshes {
            if mesh.triangle_range.start != next
                || mesh.triangle_range.end <= next
                || mesh.triangle_range.end > triangles.len()
            {
                return Err("Query meshes must partition canonical triangles in supplied order");
            }
            //BoardWorld collision currently owns world-space static triangles.
            //Reject an unsupported scene transform rather than move query-only geometry.
            if mesh.local_to_world != RetailAffineTransform::IDENTITY
                || mesh.world_to_local != RetailAffineTransform::IDENTITY
            {
                return Err("BoardWorld static query meshes require identity transforms");
            }
            if !mesh.local_bounds.valid() {
                return Err("Invalid query mesh bounds");
            }
            if triangles[mesh.triangle_range.clone()]
                .iter()
                .flat_map(|entry| entry.triangle.vertices)
                .any(|p| !mesh.local_bounds.contains(p))
            {
                return Err("Query mesh bounds exclude canonical triangle geometry");
            }
            next = mesh.triangle_range.end;
        }
        if next != triangles.len() {
            return Err("Query metadata leaves canonical triangles unassigned");
        }
        for edge in &self.static_edges {
            if !edge.local_bounds.valid()
                || !edge.local_bounds.contains(edge.start)
                || !edge.local_bounds.contains(edge.end)
            {
                return Err("Invalid authored query edge");
            }
        }
        Ok(())
    }
}
fn finite(p: Vector3) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
}
