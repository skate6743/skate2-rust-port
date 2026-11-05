//! Asset-independent Skate 3 physics over actual Skate 2 world geometry.
//!
//! The imported crate contains calculations only: no file I/O, engine startup,
//! rendering, or Skate 3 asset loader. Animation/player scheduling is a separate
//! integration boundary and is not replaced by this world adapter.
#[cfg(feature = "skate3-runtime")]
pub(crate) use crate::skate3_native::physics::player;

use crate::static_ground_pool::{ResidentGroundGeometry, StaticGroundPool};
pub use skate3_core::physics::contact::RetailContactMaterial as ContactMaterial;
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{
            BoardWorld, WorldTriangle,
            query_metadata::{Bounds, QueryMesh, QueryMetadata, QueryPool},
        },
        drive_frames::RetailAffineTransform,
    },
};

pub struct Skate3GroundWorld {
    world: BoardWorld,
    sources: Vec<ResidentGroundGeometry>,
    query_sources: Vec<usize>,
    material: ContactMaterial,
}

impl Skate3GroundWorld {
    /// Preserve geometry, packed surfaces, contact features, order and source
    /// ownership. The caller supplies its actual active contact material.
    ///
    /// # Errors
    /// Returns the original export error or the native Skate 3 validation error.
    pub fn try_new(pool: &StaticGroundPool, material: ContactMaterial) -> Result<Self, String> {
        Self::from_resident_geometry(
            pool.resident_geometry().map_err(|e| e.to_string())?,
            material,
        )
    }

    /// Consume already-exported resident geometry without decoding it again.
    ///
    /// # Errors
    /// Rejects empty/invalid collision without adding fallback terrain.
    pub fn from_resident_geometry(
        sources: Vec<ResidentGroundGeometry>,
        material: ContactMaterial,
    ) -> Result<Self, String> {
        let mut triangles = Vec::new();
        let mut packed_surfaces = Vec::new();
        let mut meshes = Vec::new();
        let mut query_sources = Vec::new();
        for (source_index, source) in sources.iter().enumerate() {
            if source.features.len() != source.triangles.len() {
                return Err(format!(
                    "Ground mesh {source_index} has incomplete native features"
                ));
            }
            let start = triangles.len();
            for (index, (triangle, feature)) in
                source.triangles.iter().zip(&source.features).enumerate()
            {
                let surface = triangle.surface_id.unwrap_or(0);
                // BoardWorld queries consume a packed RW surface, not S2's GP user tag.
                let tag = u32::from(surface);
                let points = triangle.vertices.map(|[x, y, z]| Vector3::new(x, y, z));
                triangles.push(
                    WorldTriangle::from_vertices(
                        points,
                        material,
                        tag,
                        feature.flags,
                        feature.edge_cosines,
                        0.,
                    )
                    .ok_or_else(|| {
                        format!("Skate 3 rejected ground mesh {source_index}, triangle {index}")
                    })?,
                );
                packed_surfaces.push(surface);
            }
            // Match the local Skate 3 adapter's 64-triangle broadphase leaves,
            // while retaining each leaf's actual Skate 2 source mesh identity.
            for leaf_start in (start..triangles.len()).step_by(64) {
                let end = (leaf_start + 64).min(triangles.len());
                let local_bounds = Bounds::from_points(
                    triangles[leaf_start..end]
                        .iter()
                        .flat_map(|triangle| triangle.triangle.vertices),
                )
                .ok_or("Empty ground leaf bounds")?;
                meshes.push(QueryMesh {
                    triangle_range: leaf_start..end,
                    local_to_world: RetailAffineTransform::IDENTITY,
                    world_to_local: RetailAffineTransform::IDENTITY,
                    local_bounds,
                    matching_group: -1,
                    rejection_flags: 0,
                    geometry: 0,
                    pool: QueryPool::Ground,
                });
                query_sources.push(source_index);
            }
        }
        if triangles.is_empty() {
            return Err("Skate 3 requires real resident Skate 2 collision geometry".into());
        }
        let world = BoardWorld::with_query_metadata(
            triangles,
            QueryMetadata {
                packed_surfaces,
                meshes,
                static_edges: Vec::new(),
                island_flags: 0,
            },
        )
        .map_err(str::to_owned)?;
        Ok(Self {
            world,
            sources,
            query_sources,
            material,
        })
    }

    /// Atomically rebuild after real streaming/source/material changes.
    ///
    /// # Errors
    /// Keeps the old world and all its source lifetimes on a failed replacement.
    pub fn try_refresh(
        &mut self,
        pool: &StaticGroundPool,
        material: ContactMaterial,
    ) -> Result<bool, String> {
        if self.material == material && pool.matches_resident_geometry(&self.sources) {
            return Ok(false);
        }
        let replacement = Self::try_new(pool, material)?;
        *self = replacement;
        Ok(true)
    }

    #[must_use]
    pub const fn world(&self) -> &BoardWorld {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut BoardWorld {
        &mut self.world
    }

    /// Query-metadata indices are not guessed object IDs or reused slot numbers.
    #[must_use]
    pub fn source_for_query_mesh(&self, mesh: usize) -> Option<&ResidentGroundGeometry> {
        self.sources.get(*self.query_sources.get(mesh)?)
    }
}
