//! NavMesh2 Fixup829c7a20 / GetMeshCell82e580c8 over retained arena bytes.
//! Offsets are native fields, not a second navigation representation; a linked Skate 3 data
//! folder's NavPower record is read into its own cells (`nav_power`).
use crate::{
    simulation_inner_registry_asset::SimulationInnerRegistryAsset,
    simulation_normal_arena_plan::{NAV_POWER_DATA_TYPE_ID, SimulationRegistryAssetKind},
    world_painter_layer_asset::{
        LayerAssetError, quad_tree_lookup, referenced_object, vector, word,
    },
};
use std::sync::Arc;

#[path = "living_world_nav_power.rs"]
pub(crate) mod nav_power;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavAssetError {
    Data(LayerAssetError),
    WrongKind,
    Index { index: u32, count: u32 },
    TooManyCandidates(u32),
    NullDictionary,
}
impl From<LayerAssetError> for NavAssetError {
    fn from(value: LayerAssetError) -> Self {
        Self::Data(value)
    }
}
pub(crate) struct NavMeshAsset {
    asset: Arc<SimulationInnerRegistryAsset>,
    nav_power: Option<nav_power::NavPowerTile>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cell {
    pub(crate) id: u32,
    pub(crate) centroid: [f32; 3],
    pub(crate) fixed_cost: f32,
}
impl NavMeshAsset {
    pub(crate) fn asset_id(&self) -> u64 {
        self.asset.id
    }
    pub(crate) fn nav_power(&self) -> Option<&nav_power::NavPowerTile> {
        self.nav_power.as_ref()
    }
    pub(crate) fn cell(&self, id: u32) -> Result<Cell, NavAssetError> {
        let id = id & 0x1fff_ffff;
        if let Some(tile) = &self.nav_power {
            return tile.cell(id);
        }
        let data = self.record(0x58, 0x64, 56, id)?;
        Ok(Cell {
            id,
            centroid: xyz(data, 0x24)?,
            fixed_cost: f32::from_bits(word(data, 0x30)?),
        })
    }
    pub(crate) fn cell_edges(&self, id: u32) -> Result<[u32; 3], NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.cell_edges(id & 0x1fff_ffff);
        }
        let data = self.record(0x58, 0x64, 56, id & 0x1fff_ffff)?;
        Ok([word(data, 12)?, word(data, 16)?, word(data, 20)?])
    }
    pub(crate) fn cell_neighbours(&self, id: u32) -> Result<[u32; 3], NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.cell_neighbours(id & 0x1fff_ffff);
        }
        let data = self.record(0x58, 0x64, 56, id & 0x1fff_ffff)?;
        Ok([word(data, 24)?, word(data, 28)?, word(data, 32)?])
    }
    pub(crate) fn cell_address(&self, id: u32) -> Result<usize, NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.cell_address(id & 0x1fff_ffff);
        }
        Ok(self.record(0x58, 0x64, 56, id & 0x1fff_ffff)?.as_ptr() as usize)
    }
    pub(crate) fn edge_cells(&self, id: u32) -> Result<[u32; 2], NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.edge_cells(id);
        }
        let data = self.record(0x54, 0x60, 44, id)?;
        Ok([word(data, 8)?, word(data, 12)?])
    }
    pub(crate) fn edge_points(&self, id: u32) -> Result<[[f32; 4]; 2], NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.edge_points(id);
        }
        let data = self.record(0x54, 0x60, 44, id)?;
        let a = self.point(word(data, 0)?)?;
        let b = self.point(word(data, 4)?)?;
        Ok([[a[0], a[1], a[2], 0.], [b[0], b[1], b[2], 0.]])
    }
    pub(crate) fn edge_address(&self, id: u32) -> Result<usize, NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.edge_address(id);
        }
        Ok(self.record(0x54, 0x60, 44, id)?.as_ptr() as usize)
    }
    pub(crate) fn edge_line(&self, id: u32) -> Result<([f32; 4], f32), NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.edge_line(id);
        }
        let data = self.record(0x54, 0x60, 44, id)?;
        let normal = xyz(data, 28)?;
        Ok((
            [normal[0], normal[1], normal[2], 0.],
            f32::from_bits(word(data, 40)?),
        ))
    }
    pub(crate) fn cell_points(&self, id: u32) -> Result<[[f32; 4]; 3], NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return tile.cell_points(id & 0x1fff_ffff);
        }
        let data = self.record(0x58, 0x64, 56, id & 0x1fff_ffff)?;
        let mut result = [[0.; 4]; 3];
        for (i, out) in result.iter_mut().enumerate() {
            let p = self.point(word(data, i * 4)?)?;
            *out = [p[0], p[1], p[2], 0.];
        }
        Ok(result)
    }
    pub(crate) fn new(asset: Arc<SimulationInnerRegistryAsset>) -> Result<Self, NavAssetError> {
        if asset.kind != SimulationRegistryAssetKind::NavMesh {
            return Err(NavAssetError::WrongKind);
        }
        if asset.object_type().map_err(|e| NavAssetError::Data(e.into()))? == NAV_POWER_DATA_TYPE_ID {
            let bytes = asset.bytes().map_err(|e| NavAssetError::Data(e.into()))?;
            let tile = nav_power::NavPowerTile::read(bytes)?;
            return Ok(Self { asset, nav_power: Some(tile) });
        }
        let result = Self { asset, nav_power: None };
        word(result.bytes()?, 0x6c)?;
        Ok(result)
    }
    fn bytes(&self) -> Result<&[u8], NavAssetError> {
        self.asset
            .bytes()
            .map_err(|e| NavAssetError::Data(e.into()))
    }
    pub(crate) fn neighbour_ids(&self) -> Result<[u64; 5], NavAssetError> {
        let bytes = self.bytes()?;
        let mut ids = [0; 5];
        for (i, id) in ids.iter_mut().enumerate() {
            *id = (u64::from(word(bytes, i * 8)?) << 32) | u64::from(word(bytes, i * 8 + 4)?);
        }
        Ok(ids)
    }
    pub(crate) fn contains_xz(&self, p: [f32; 3]) -> Result<bool, NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return Ok(tile.contains_xz(p));
        }
        let bytes = self.bytes()?;
        let min = vector(bytes, 0x30)?;
        let max = vector(bytes, 0x40)?;
        Ok(p[0] >= min[0] && max[0] >= p[0] && p[2] >= min[2] && max[2] >= p[2])
    }
    fn record(
        &self,
        count_at: usize,
        pointer_at: usize,
        stride: usize,
        index: u32,
    ) -> Result<&[u8], NavAssetError> {
        let bytes = self.bytes()?;
        let count = word(bytes, count_at)?;
        if index >= count {
            return Err(NavAssetError::Index { index, count });
        }
        let start = word(bytes, pointer_at)? as usize + index as usize * stride;
        bytes
            .get(start..start + stride)
            .ok_or(NavAssetError::Data(LayerAssetError::Truncated {
                required: start + stride,
                actual: bytes.len(),
            }))
    }
    fn point(&self, id: u32) -> Result<[f32; 3], NavAssetError> {
        xyz(self.record(0x50, 0x5c, 12, id)?, 0)
    }
    pub(crate) fn get_cell(
        &self,
        p: [f32; 3],
        y_tolerance: f32,
    ) -> Result<Option<Cell>, NavAssetError> {
        if let Some(tile) = &self.nav_power {
            return Ok(tile.get_cell(p, y_tolerance));
        }
        let bytes = self.bytes()?;
        let Some(tree) = referenced_object(&self.asset, word(bytes, 0x68)?)? else {
            return Ok(None);
        };
        let Some(index) = quad_tree_lookup(tree, [p[0], p[2]])? else {
            return Ok(None);
        };
        if index == u16::MAX {
            return Ok(None);
        }
        let dictionary = referenced_object(&self.asset, word(bytes, 0x6c)?)?
            .ok_or(NavAssetError::NullDictionary)?;
        let total = word(dictionary, 0)?;
        if u32::from(index) >= total {
            return Err(LayerAssetError::DictionaryIndex { index, total }.into());
        }
        let slot = word(dictionary, 12)? as usize + usize::from(index) * 8;
        let start = word(dictionary, slot)? as usize;
        let count = word(dictionary, slot + 4)?;
        if count > 64 {
            return Err(NavAssetError::TooManyCandidates(count));
        }
        let mut best: Option<Cell> = None;
        let mut distance = f32::MAX;
        for i in 0..count {
            let id = word(dictionary, start + i as usize * 4)?;
            let cell = self.record(0x58, 0x64, 56, id & 0x1fff_ffff)?;
            let points = [
                self.point(word(cell, 0)?)?,
                self.point(word(cell, 4)?)?,
                self.point(word(cell, 8)?)?,
            ];
            if !point_in_triangle_xz(p, points) {
                continue;
            }
            let centroid = xyz(cell, 0x24)?;
            let delta = (centroid[1] - p[1]).abs();
            if best.is_none() || delta < distance {
                best = Some(Cell {
                    id,
                    centroid,
                    fixed_cost: f32::from_bits(word(cell, 0x30)?),
                });
                distance = delta;
            }
        }
        Ok(if distance < y_tolerance { best } else { None })
    }
}
fn xyz(bytes: &[u8], at: usize) -> Result<[f32; 3], NavAssetError> {
    Ok([
        f32::from_bits(word(bytes, at)?),
        f32::from_bits(word(bytes, at + 4)?),
        f32::from_bits(word(bytes, at + 8)?),
    ])
}

/// PointInTris_XZ82e4e1a0. Preserve the raw VMX multiply then VNMSUB order;
/// edges are inclusive and either winding works. No invented epsilon.
pub(crate) fn point_in_triangle_xz(p: [f32; 3], triangle: [[f32; 3]; 3]) -> bool {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[2] - b[2]];
    let cross = |a: [f32; 2], b: [f32; 2]| -(a[0].mul_add(b[1], -(a[1] * b[0])));
    let [a, b, c] = triangle;
    let first = cross(sub(b, a), sub(p, a)) * cross(sub(b, a), sub(c, a));
    let second = cross(sub(c, b), sub(p, b)) * cross(sub(c, b), sub(a, b));
    let third = cross(sub(a, c), sub(p, c)) * cross(sub(a, c), sub(b, c));
    first >= 0.0 && second >= 0.0 && third >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_uses_xz_inclusive_edges_and_both_windings() {
        let a = [0., 5., 0.];
        let b = [2., 6., 0.];
        let c = [0., 7., 2.];
        for triangle in [[a, b, c], [a, c, b]] {
            assert!(point_in_triangle_xz([1., -999., 1.], triangle));
            assert!(point_in_triangle_xz(a, triangle));
            assert!(!point_in_triangle_xz([1.1, 0., 1.1], triangle));
            assert!(!point_in_triangle_xz([f32::NAN, 0., 0.], triangle));
        }
    }
}
