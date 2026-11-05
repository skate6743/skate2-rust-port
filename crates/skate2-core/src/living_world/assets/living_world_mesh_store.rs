//! Actual `MeshMan82ce7590`/`MeshStore82e60690` constructor ownership and the
//! streamed tile ownership and source triangle/cost queries.
use crate::{
    selected_simulation_asset_services::{SelectedAssetError, SelectedSimulationAssetServices},
    skeleton_root_transforms::SkeletonRootVector,
};
#[path = "../navigation/living_world_nav_mesh_asset.rs"]
pub(crate) mod asset;
#[path = "../navigation/living_world_pathogen.rs"]
pub(crate) mod pathogen;
use asset::{Cell, NavAssetError, NavMeshAsset};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshStoreError {
    OwnerLock,
    MissingConstruction,
    AlreadyConstructed,
    PendingTileAdmission { asset_id: u64 },
    PendingTileRemoval { asset_id: u64 },
    PendingNonemptyQuery,
    Asset(SelectedAssetError),
    Nav(NavAssetError),
    DuplicateTile(u64),
}
impl std::fmt::Display for MeshStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native mesh store boundary: {self:?}")
    }
}
impl std::error::Error for MeshStoreError {}

struct NavMeshTile {
    data: Arc<NavMeshAsset>,
    neighbours: [Option<u64>; 5],
}
struct MeshStoreFields {
    tile_map: BTreeMap<u64, NavMeshTile>,
    assets: Option<Arc<SelectedSimulationAssetServices>>,
    /// PC port: removed tiles, alive until LivingWorld's next frame releases them from every
    /// pedestrian (`take_released`).
    released: Vec<Arc<NavMeshAsset>>,
    /// Host continuation guard after an unreconstructed native operation. This
    /// is not a native field and never substitutes a failed lookup as a miss.
    pending: Option<MeshStoreError>,
}
pub(crate) struct MeshStore {
    fields: Mutex<MeshStoreFields>,
}
/// Actual null cell/handle pair from `GetMeshCell82e58704..70c`.
#[derive(Clone)]
pub(crate) enum MeshCell {
    Absent,
    Present {
        tile: Weak<NavMeshAsset>,
        cell: Cell,
    },
}
impl MeshCell {
    pub(crate) fn height_at<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, position: [f32; 4], numeric: &mut N,
    ) -> Result<Option<f32>, String>
    where N::Error: std::fmt::Debug,
    {
        let Self::Present { tile, cell } = self else { return Ok(None); };
        let Some(tile) = tile.upgrade() else { return Ok(None); };
        let points = tile.cell_points(cell.id).map_err(|e| format!("mesh height: {e:?}"))?;
        pathogen::triangle_height(position, points, numeric)
            .map_err(|e| format!("mesh height numeric: {e:?}"))
    }
    pub(crate) fn same(&self, other: &Self) -> bool {
        matches!((self,other),(Self::Present{tile:a,cell:ac},Self::Present{tile:b,cell:bc})
            if a.ptr_eq(b) && (ac.id&0x1fff_ffff)==(bc.id&0x1fff_ffff))
    }
    /// tMeshCell GetCost/GetCentroid retain the streamed handle's lifetime.
    pub(crate) fn cost_and_centroid(&self) -> Option<(f32, [f32; 4])> {
        match self {
            Self::Present { tile, cell } if tile.upgrade().is_some() => Some((
                cell.fixed_cost,
                [cell.centroid[0], cell.centroid[1], cell.centroid[2], 0.0],
            )),
            _ => None,
        }
    }
    pub(crate) fn is_valid(&self) -> bool {
        matches!(self, Self::Present { tile, .. } if tile.upgrade().is_some())
    }
    /// `GetCellScore82e517f0`: invalid cell returns literal10000000 at5182c.
    pub(crate) fn score(&self) -> f32 {
        self.score_with_threshold(0.0)
    }
    pub(crate) fn score_with_threshold(&self, threshold: f32) -> f32 {
        match self {
            Self::Absent => 10_000_000.0,
            Self::Present { tile, cell } => {
                if tile.upgrade().is_none() {
                    return 10_000_000.0;
                }
                cell_score(cell.fixed_cost, threshold)
            }
        }
    }
}
/// GetCellScore82e517f0, ordered fsel predicates including unordered inputs.
fn cell_score(cost: f32, threshold: f32) -> f32 {
    let scaled = if threshold - cost >= 0.0 {
        0.0
    } else {
        cost * 1000.0
    };
    if cost - f32::from_bits(0x3f7d70a4) >= 0.0 {
        10_000_000.0
    } else {
        scaled
    }
}
impl MeshStore {
    pub(crate) fn tile(&self, id: u64) -> Result<Option<Arc<NavMeshAsset>>, MeshStoreError> {
        let fields = self.fields.lock().map_err(|_| MeshStoreError::OwnerLock)?;
        if let Some(error) = fields.pending { return Err(error); }
        Ok(fields.tile_map.get(&id).map(|tile|Arc::clone(&tile.data)))
    }
    /// `MeshStore82e60690`: real fixed-map constructor then actual mutex.
    pub(crate) fn construct() -> Arc<Self> {
        Arc::new(Self {
            fields: Mutex::new(MeshStoreFields {
                tile_map: BTreeMap::new(),
                assets: None,
                released: Vec::new(),
                pending: None,
            }),
        })
    }
    pub(crate) fn bind_assets(
        &self,
        assets: Arc<SelectedSimulationAssetServices>,
    ) -> Result<(), MeshStoreError> {
        let mut fields = self.fields.lock().map_err(|_| MeshStoreError::OwnerLock)?;
        if fields.assets.is_some() {
            return Err(MeshStoreError::AlreadyConstructed);
        }
        fields.assets = Some(assets);
        Ok(())
    }
    /// AddNavMeshTile82e63968: registry winner, reciprocal links, then map insert.
    pub(crate) fn try_add_tile(&self, asset_id: u64) -> Result<(), MeshStoreError> {
        let mut fields = self.fields.lock().map_err(|_| MeshStoreError::OwnerLock)?;
        if let Some(error) = fields.pending {
            return Err(error);
        }
        let Some(assets) = fields.assets.as_ref() else {
            let error = MeshStoreError::PendingTileAdmission { asset_id };
            fields.pending = Some(error);
            return Err(error);
        };
        let Some(asset) = assets.try_inner(asset_id).map_err(MeshStoreError::Asset)? else {
            return Ok(());
        };
        if fields.tile_map.contains_key(&asset_id) {
            return Err(MeshStoreError::DuplicateTile(asset_id));
        }
        let data = Arc::new(NavMeshAsset::new(asset).map_err(MeshStoreError::Nav)?);
        let mut neighbours = [Some(asset_id), None, None, None, None];
        if let Some(tile) = data.nav_power() {
            // PC port: a Skate 3 tile's neighbours are the loaded tiles beside it on Skate 3's
            // grid, its border edges linked to theirs (`living_world_nav_power`).
            for (&id, neighbour) in &mut fields.tile_map {
                let Some(other) = neighbour.data.nav_power() else { continue };
                let Some(i) = tile.slot_of(other) else { continue };
                tile.stitch(i, other);
                neighbours[i] = Some(id);
                neighbour.neighbours[opposite(i)] = Some(asset_id);
            }
        } else {
            let ids = data.neighbour_ids().map_err(MeshStoreError::Nav)?;
            for i in 1..5 {
                // Native tests only the low 32 bits of the authored asset ID.
                if ids[i] as u32 == 0 {
                    continue;
                }
                if let Some(neighbour) = fields.tile_map.get_mut(&ids[i]) {
                    neighbours[i] = Some(ids[i]);
                    neighbour.neighbours[opposite(i)] = Some(asset_id);
                }
            }
        }
        fields
            .tile_map
            .insert(asset_id, NavMeshTile { data, neighbours });
        Ok(())
    }
    pub(crate) fn try_remove_tile(&self, asset_id: u64) -> Result<(), MeshStoreError> {
        let mut fields = self.fields.lock().map_err(|_| MeshStoreError::OwnerLock)?;
        if let Some(error) = fields.pending {
            return Err(error);
        }
        let Some(assets) = fields.assets.as_ref() else {
            let error = MeshStoreError::PendingTileRemoval { asset_id };
            fields.pending = Some(error);
            return Err(error);
        };
        if assets
            .try_inner(asset_id)
            .map_err(MeshStoreError::Asset)?
            .is_none()
        {
            return Ok(());
        }
        let Some(tile) = fields.tile_map.get(&asset_id) else {
            return Ok(());
        };
        let neighbours = tile.neighbours;
        for i in 1..5 {
            if let Some(neighbour) = neighbours[i].and_then(|id| fields.tile_map.get_mut(&id)) {
                neighbour.neighbours[opposite(i)] = None;
            }
        }
        // `MeshMan::RemoveNavMeshData` (`0x82ce6ac0`) has the PedestrianMan release the tile
        // (`ReleaseNavMeshTile`) before removing it. PC port: a Sim arena's removal (Skate 3's
        // tiles) cannot reach the pedestrians, so the tile's data stays alive until LivingWorld's
        // next frame releases it from them, before any pedestrian reads a path.
        if let Some(tile) = fields.tile_map.remove(&asset_id) {
            fields.released.push(tile.data);
        }
        Ok(())
    }
    /// The tiles removed since the last call, for the pedestrians to release.
    pub(crate) fn take_released(&self) -> Result<Vec<Arc<NavMeshAsset>>, MeshStoreError> {
        let mut fields = self.fields.lock().map_err(|_| MeshStoreError::OwnerLock)?;
        Ok(std::mem::take(&mut fields.released))
    }
    /// Actual `GetPosTileAssetId82e57ff8` empty iteration returns null ID, then
    /// `GetMeshCell82e580c8` map-find falls through to the null cell/handle pair.
    /// Position lanes are not compared when the retained map has no entries.
    pub(crate) fn try_get_cell(
        &self,
        position: SkeletonRootVector,
    ) -> Result<MeshCell, MeshStoreError> {
        self.try_get_cell_with_tolerance(position, f32::MAX)
    }
    /// Same native query with the caller's explicit vertical tolerance.
    pub(crate) fn try_get_cell_with_tolerance(
        &self,
        position: SkeletonRootVector,
        y_tolerance: f32,
    ) -> Result<MeshCell, MeshStoreError> {
        let fields = self.fields.lock().map_err(|_| MeshStoreError::OwnerLock)?;
        if let Some(error) = fields.pending {
            return Err(error);
        }
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static QUERIES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let trace = *TRACE.get_or_init(||std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some())
            && QUERIES.fetch_add(1,std::sync::atomic::Ordering::Relaxed)%600==0;
        if trace {std::eprintln!("pedestrian nav query position={:?} tiles={} ids={:?}",position.xyz,
            fields.tile_map.len(),fields.tile_map.keys().collect::<Vec<_>>());}
        // GetPosTileAssetId iterates sorted IDs and returns the first XZ match.
        // Do not search subsequent overlapping tiles if that tile has no cell.
        for tile in fields.tile_map.values() {
            // PC port: a Skate 3 tile still loaded as its district is left never stands in for
            // Skate 2's navigation at the same coordinates.
            if tile.data.nav_power().is_some() && !crate::skate3::district_active() {
                continue;
            }
            if !tile
                .data
                .contains_xz(position.xyz)
                .map_err(MeshStoreError::Nav)?
            {
                continue;
            }
            let found = tile.data.get_cell(position.xyz, y_tolerance).map_err(MeshStoreError::Nav)?;
            if trace {std::eprintln!("pedestrian nav matching tile cell={found:?}");}
            return Ok(
                match found {
                    Some(cell) => MeshCell::Present {
                        tile: Arc::downgrade(&tile.data),
                        cell,
                    },
                    None => MeshCell::Absent,
                },
            );
        }
        Ok(MeshCell::Absent)
    }
    /// Camera-used `RAINMan.PointOnRoadNetwork826f3a30` math after its caller
    /// captures the actual `MeshMan`'s store. Raw gates require ordered >0 and <550.
    pub(crate) fn try_point_on_road_network(
        &self,
        position: SkeletonRootVector,
    ) -> Result<bool, MeshStoreError> {
        let score = self.try_get_cell(position)?.score();
        Ok(score > 0.0 && score < 550.0)
    }
}
fn opposite(index: usize) -> usize {
    if index & 1 != 0 { index + 1 } else { index - 1 }
}

/// Selected actual `MeshMan` child fields; `PathSolver` Init and the global Solver
/// publication remain separate native services, not claimed by this projection.
pub(crate) struct MeshMan {
    pub(crate) store: Arc<MeshStore>,
    pub(crate) solver: pathogen::PathSolver,
    pub(crate) nav_validator: Mutex<crate::pedestrian_navigation::rig::NavRigBatch>,
}
impl MeshMan {
    /// Native FrameBegin82ce6c28 resets only the retained validator count.
    pub(crate) fn frame_begin(&self) -> Result<(), MeshStoreError> {
        self.nav_validator.lock().map_err(|_| MeshStoreError::OwnerLock)?.frame_begin();
        Ok(())
    }
    pub(crate) fn construct_fields() -> Self {
        Self {
            store: MeshStore::construct(),
            solver: pathogen::PathSolver::new(),
            nav_validator: Mutex::new(crate::pedestrian_navigation::rig::NavRigBatch::default()),
        }
    }
}

#[cfg(test)]
#[path = "living_world_mesh_store_tests.rs"]
mod tests;
