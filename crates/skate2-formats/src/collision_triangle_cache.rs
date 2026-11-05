//! Reusable immutable decode for the eager triangle-query backend.
use super::{
    ClusteredMesh, CollisionError, KdLeafRange, MeshCluster, MeshTriangle, MeshUnit,
    append_leaf_triangles,
};

/// Owned cluster/unit decode. Construct only when the original query reaches
/// its eager decode stage; retaining it does not change traversal or hit order.
#[derive(Debug)]
pub struct PreparedMeshTriangles {
    mesh: ClusteredMesh,
    clusters: Vec<MeshCluster>,
    units: Vec<Vec<MeshUnit>>,
}
impl PreparedMeshTriangles {
    /// Uses exactly the existing all-cluster then all-unit validation order.
    /// # Errors
    /// Returns the same decode error as the eager triangle decoder.
    pub fn new(mesh: &ClusteredMesh, bytes: &[u8]) -> Result<Self, CollisionError> {
        let clusters = mesh.clusters(bytes)?;
        let units = clusters
            .iter()
            .map(|cluster| cluster.units(mesh))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            mesh: mesh.clone(),
            clusters,
            units,
        })
    }
    /// Borrow this immutable mesh's already validated cluster and unit decode.
    /// Both slices retain serialized order. A consumer must use the same mesh
    /// owner; this does not admit reuse across resident mesh occurrences.
    #[must_use]
    pub fn cluster_units(&self) -> (&[MeshCluster], &[Vec<MeshUnit>]) {
        (&self.clusters, &self.units)
    }

    /// Expands ranges with the original helper: no sorting or deduplication.
    /// # Errors
    /// Returns the original selected range/unit/vertex validation error.
    pub fn triangles_for_leaf_ranges(
        &self,
        ranges: &[KdLeafRange],
    ) -> Result<Vec<MeshTriangle>, CollisionError> {
        let shift = if self.mesh.maximum_cluster_count <= 0x1_0000 {
            16
        } else {
            20
        };
        let mask = (1_u32 << shift) - 1;
        let mut triangles = Vec::new();
        for range in ranges {
            append_leaf_triangles(
                &self.mesh,
                &self.clusters,
                &self.units,
                *range,
                shift,
                mask,
                &mut triangles,
            )?;
        }
        Ok(triangles)
    }
}
