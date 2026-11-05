//! Lazy field access to the actual retained `WorldPainter` layer record.
//! Serialized object IDs resolve within the same decoded arena allocation.
use crate::{
    simulation_inner_registry_asset::{InnerAssetError, SimulationInnerRegistryAsset},
    simulation_normal_arena_plan::SimulationRegistryAssetKind,
};
use skate2_formats::pegasus::ObjectId;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerAssetError {
    Inner(InnerAssetError),
    WrongAssetKind,
    Truncated { required: usize, actual: usize },
    NonlocalObject(u32),
    MissingObject(u32),
    ObjectBytes(u32),
    NodeIndex,
    NullDictionary,
    DictionaryIndex { index: u16, total: u32 },
}
impl From<InnerAssetError> for LayerAssetError {
    fn from(value: InnerAssetError) -> Self {
        Self::Inner(value)
    }
}

#[derive(Clone)]
pub(crate) struct WorldPainterLayerAsset {
    pub(crate) asset: Arc<SimulationInnerRegistryAsset>,
}
impl WorldPainterLayerAsset {
    pub(crate) fn try_new(
        asset: Arc<SimulationInnerRegistryAsset>,
    ) -> Result<Self, LayerAssetError> {
        if asset.kind != SimulationRegistryAssetKind::WorldPainter {
            return Err(LayerAssetError::WrongAssetKind);
        }
        Ok(Self { asset })
    }

    fn quad_tree(&self) -> Result<Option<&[u8]>, LayerAssetError> {
        self.object(0)
    }

    /// `tWorldPainterLayerData::m_pDictionary` (+4); `Fixup` 0x829c7858
    /// resolves both +0 and +4 with `Arena::IdToObject`.
    fn dictionary(&self) -> Result<Option<&[u8]>, LayerAssetError> {
        self.object(4)
    }

    fn object(&self, field: usize) -> Result<Option<&[u8]>, LayerAssetError> {
        let data = self.asset.bytes()?;
        let object = word(data, field)?;
        referenced_object(&self.asset, object)
    }

    /// Register/Unregister dereference the quad tree and read both full vectors
    /// before computing a tile ID or inspecting the layer GUID.
    #[expect(
        clippy::type_complexity,
        reason = "Optional native bounds are the full minimum and maximum four-lane vectors"
    )]
    pub(crate) fn registration_bounds(
        &self,
    ) -> Result<Option<([f32; 4], [f32; 4])>, LayerAssetError> {
        let Some(bytes) = self.quad_tree()? else {
            return Ok(None);
        };
        Ok(Some((vector(bytes, 0)?, vector(bytes, 16)?)))
    }
    /// Raw `ld +8`/`cmpd` at82ceb8e8..8f8 compares the complete 64-bit GUID.
    pub(crate) fn layer_guid(&self) -> Result<u64, LayerAssetError> {
        let bytes = self.asset.bytes()?;
        Ok((u64::from(word(bytes, 8)?) << 32) | u64::from(word(bytes, 12)?))
    }
    /// `GetTOCEntry` reads this only after tile and selected layer are nonnull.
    #[cfg(test)]
    pub(crate) fn quad_node_count(&self) -> Result<Option<u32>, LayerAssetError> {
        self.quad_tree()?.map(|bytes| word(bytes, 32)).transpose()
    }

    /// `WorldPainter::DoQuadTreeLookup` 0x82cea8b8 as `GetAttribData` calls
    /// it: None for a NULL tree, zero nodes, or the native -1 miss.
    pub(crate) fn dictionary_lookup(
        &self,
        point: [f32; 2],
    ) -> Result<Option<u16>, LayerAssetError> {
        match self.quad_tree()? {
            Some(tree) if word(tree, 32)? != 0 => quad_tree_lookup(tree, point),
            _ => Ok(None),
        }
    }

    /// PC port: the 64-bit dictionary entry for a lookup index, which a linked Skate 3
    /// district's layer holds as its region's collection key, low word first (TU3 0x82c0eac0).
    pub(crate) fn region_key(&self, index: u16) -> Result<u64, LayerAssetError> {
        dictionary_region_key(
            self.dictionary()?.ok_or(LayerAssetError::NullDictionary)?,
            index,
        )
    }

    /// The palette region word for a dictionary lookup index.
    pub(crate) fn region_word(&self, index: u16) -> Result<u32, LayerAssetError> {
        dictionary_region_word(
            self.dictionary()?.ok_or(LayerAssetError::NullDictionary)?,
            index,
        )
    }
}

/// Shared Arena::IdToObject projection for retained world-painter and nav
/// records. The original arena allocation owns both referenced data objects.
pub(crate) fn referenced_object(
    asset: &SimulationInnerRegistryAsset,
    object: u32,
) -> Result<Option<&[u8]>, LayerAssetError> {
    // Actual null object reference; other section values remain unsupported.
    if object == 0 {
        return Ok(None);
    }
    let id = ObjectId(object);
    if id.section_index() != 0 {
        return Err(LayerAssetError::NonlocalObject(object));
    }
    let decoded = &asset.plan.static_plan().asset().decoded;
    let entry = decoded
        .arena
        .local_entry(id)
        .ok_or(LayerAssetError::MissingObject(object))?;
    Ok(Some(
        decoded
            .arena
            .object_bytes(&decoded.resources.resources, entry)
            .ok_or(LayerAssetError::ObjectBytes(object))?,
    ))
}

/// `DoQuadTreeLookup` over one serialized `tWorldPainterQuadTreeData`: root
/// centre (+0) and half width (+16) vectors, `m_uiNumNodes` (+32) and
/// `m_pRootNode` (+36, relative to the tree; `Fixup` 0x829c6408). A `tNode`
/// is four signed child indices then `m_DictionaryLookup`, ten bytes. The
/// hint-node argument is never read.
pub(crate) fn quad_tree_lookup(
    tree: &[u8],
    point: [f32; 2],
) -> Result<Option<u16>, LayerAssetError> {
    let root = usize::try_from(word(tree, 36)?).map_err(|_| LayerAssetError::NodeIndex)?;
    let mut centre = vector(tree, 0)?;
    let mut half_width = vector(tree, 16)?;
    let mut index: i16 = 0;
    loop {
        // Loop head compares (dx, dy, 0, 0) against (hx, hy, 0, 0).
        let offset = [point[0] - centre[0], point[1] - centre[1], 0.0, 0.0];
        if !in_bounds(offset, [half_width[0], half_width[1], 0.0, 0.0]) {
            return Ok(None);
        }
        let node = usize::try_from(i32::from(index) * 10)
            .ok()
            .and_then(|offset| root.checked_add(offset))
            .ok_or(LayerAssetError::NodeIndex)?;
        let mut children = [0_u16; 4];
        for (child, value) in children.iter_mut().enumerate() {
            *value = half_word(tree, node + child * 2)?;
        }
        // `lhz`/`cmplwi 0xFFFF`: an unsigned first child marks a leaf.
        if children[0] == 0xffff {
            return half_word(tree, node + 8).map(Some);
        }
        // `vmulfp` by `half` (0.5, 0.5, 0, 0), then each child centre is
        // `vmaddfp(half_width, quad[i], centre)`.
        half_width = std::array::from_fn(|lane| half_width[lane] * HALF[lane]);
        let centres: [[f32; 4]; 4] = std::array::from_fn(|child| {
            std::array::from_fn(|lane| half_width[lane].mul_add(QUAD[child][lane], centre[lane]))
        });
        // `InBounds4Way` 0x82cea420 rebuilds each offset as (dx, dy, 0, 0)
        // but tests it against the full child half-width vector.
        let inside = centres.map(|child| {
            in_bounds(
                [point[0] - child[0], point[1] - child[1], 0.0, 0.0],
                half_width,
            )
        });
        let chosen = if inside[0] {
            0
        } else if let Some(child) = (1..4).find(|&child| inside[child]) {
            child
        } else {
            return Ok(None);
        };
        // `lhz` into r29, sign-extended by `extsh` at the loop head.
        index = i16::from_be_bytes(children[chosen].to_be_bytes());
        centre = centres[chosen];
    }
}

/// The region word `GetAttribData` compares against the palette, from one
/// serialized `tWorldPainterDictionaryData`: `m_pEntryTable` (+12) holds
/// eight-byte slots whose first word is an entry offset (both relative to the
/// dictionary; `Fixup` 0x829c63a0); the entry's first word is the region.
/// Native asserts (and continues) when `index >= m_NumTotalEntries` (+0);
/// that is surfaced as an error here.
pub(crate) fn dictionary_region_word(
    dictionary: &[u8],
    index: u16,
) -> Result<u32, LayerAssetError> {
    let total = word(dictionary, 0)?;
    if u32::from(index) >= total {
        return Err(LayerAssetError::DictionaryIndex { index, total });
    }
    let table = usize::try_from(word(dictionary, 12)?).map_err(|_| LayerAssetError::NodeIndex)?;
    let slot = table + usize::from(index) * 8;
    let entry = usize::try_from(word(dictionary, slot)?).map_err(|_| LayerAssetError::NodeIndex)?;
    word(dictionary, entry)
}

/// PC port: [`dictionary_region_word`]'s entry read whole, as Skate 3's key.
pub(crate) fn dictionary_region_key(
    dictionary: &[u8],
    index: u16,
) -> Result<u64, LayerAssetError> {
    let total = word(dictionary, 0)?;
    if u32::from(index) >= total {
        return Err(LayerAssetError::DictionaryIndex { index, total });
    }
    let table = usize::try_from(word(dictionary, 12)?).map_err(|_| LayerAssetError::NodeIndex)?;
    let slot = table + usize::from(index) * 8;
    let entry = usize::try_from(word(dictionary, slot)?).map_err(|_| LayerAssetError::NodeIndex)?;
    // TU3 0x82c0eac0 returns `(entry[1] << 32) | entry[0]`: the key's low word comes first.
    Ok((u64::from(word(dictionary, entry + 4)?) << 32) | u64::from(word(dictionary, entry)?))
}

/// `WorldPainter::quad` (dynamic initializer 0x82f89fe0): child centre
/// directions in child order.
const QUAD: [[f32; 4]; 4] = [
    [-1.0, -1.0, 0.0, 0.0],
    [-1.0, 1.0, 0.0, 0.0],
    [1.0, -1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0, 0.0],
];
/// `WorldPainter::half` (dynamic initializer 0x82f8a080).
const HALF: [f32; 4] = [0.5, 0.5, 0.0, 0.0];

/// `vcmpbfp.` all-in-bounds: every lane satisfies `-b <= a <= b`; NaN fails.
fn in_bounds(value: [f32; 4], bound: [f32; 4]) -> bool {
    value
        .iter()
        .zip(&bound)
        .all(|(value, bound)| *value <= *bound && *value >= -*bound)
}

pub(crate) fn word(bytes: &[u8], offset: usize) -> Result<u32, LayerAssetError> {
    let data = bytes
        .get(offset..offset + 4)
        .ok_or(LayerAssetError::Truncated {
            required: offset + 4,
            actual: bytes.len(),
        })?;
    Ok(u32::from_be_bytes(
        data.try_into().expect("four-byte field"),
    ))
}
fn half_word(bytes: &[u8], offset: usize) -> Result<u16, LayerAssetError> {
    let data = bytes
        .get(offset..offset + 2)
        .ok_or(LayerAssetError::Truncated {
            required: offset + 2,
            actual: bytes.len(),
        })?;
    Ok(u16::from_be_bytes([data[0], data[1]]))
}
pub(crate) fn vector(bytes: &[u8], offset: usize) -> Result<[f32; 4], LayerAssetError> {
    Ok([
        f32::from_bits(word(bytes, offset)?),
        f32::from_bits(word(bytes, offset + 4)?),
        f32::from_bits(word(bytes, offset + 8)?),
        f32::from_bits(word(bytes, offset + 12)?),
    ])
}

#[cfg(test)]
#[path = "world_painter_layer_asset_tests.rs"]
pub(crate) mod tests;
