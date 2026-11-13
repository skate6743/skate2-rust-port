//! Static stock assets only. Registration follows package source occurrence
//! order; native octree order is retained within each registered section.
#[path = "host.rs"]
mod host;
use super::{octree::{Bounds, Octree}, spline};
use skate3_core::physics::grind_contact::Primitive;
use skate_data::skate_map::SkateMap;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SourceIdentity {
    pub stream_file: String,
    pub asset_id: String,
    pub section_index: u64,
    pub section_offset: u64,
}
struct Asset {
    #[cfg(any())]
    source: Option<SourceIdentity>,
    bounds: Bounds,
    indices: Vec<usize>,
    tree: Octree,
}
pub(crate) struct StaticProvider {
    primitives: Vec<Primitive>,
    metadata: Vec<spline::PrimitiveMetadata>,
    #[cfg(any())]
    rail_guids: Vec<[u64; 2]>,
    assets: Vec<Asset>,
    #[cfg(any())]
    source_for_primitive: Vec<usize>,
    #[cfg(any())]
    source_rail_indices: Vec<u64>,
}

impl StaticProvider {
    /// WMET preserves source section identities even when spline IDs repeat.
    /// Missing provenance is an explicit conversion prerequisite, not a ray miss.
    pub fn new(map: Option<&SkateMap>) -> Result<Self, String> {
        let Some(map) = map else {
            return Ok(Self { primitives: vec![], metadata: vec![], #[cfg(any())] rail_guids: vec![], assets: vec![], #[cfg(any())] source_for_primitive: vec![], #[cfg(any())] source_rail_indices: vec![] });
        };
        if map.rails.is_empty() {
            return Ok(Self { primitives: vec![], metadata: vec![], #[cfg(any())] rail_guids: vec![], assets: vec![], #[cfg(any())] source_for_primitive: vec![], #[cfg(any())] source_rail_indices: vec![] });
        }
        if map.rails.iter().all(|rail| rail.native.is_none()) { return Self::authored(&map.rails); }
        let mut metadata = map.extensions.iter().filter(|e| e.tag == *b"WMET");
        let extension = metadata.next().ok_or("Stock grind provider requires WMET source identity metadata")?;
        if extension.schema != 1 || metadata.next().is_some() { return Err("Expected exactly one WMET schema 1 extension".into()); }
        let manifest: Value = serde_json::from_slice(&extension.payload).map_err(|e| format!("Invalid WMET: {e}"))?;
        if manifest["grind_coordinate_policy"]["mode"].as_str() != Some("world_space") {
            return Err("Static grind provider requires verified world_space spline payloads".into());
        }
        let records = manifest["grind_splines"].as_array().ok_or("WMET grind_splines array missing")?;
        if records.len() != map.rails.len() { return Err("WMET/package rail count mismatch".into()); }
        let bytes = spline::build(Some(map))?;
        let (primitives, metadata) = spline::decoded_from_blob(&bytes)?;
        let word = |at: usize| u32::from_be_bytes(bytes[at..at+4].try_into().unwrap());
        let mut grouped: Vec<(SourceIdentity, Vec<usize>)> = Vec::new();
        let mut source_index = HashMap::new();
        let mut spatial_bounds = Vec::with_capacity(primitives.len());
        #[cfg(any())]
        let mut source_for_primitive = Vec::with_capacity(primitives.len());
        #[cfg(any())]
        let mut source_rail_indices = Vec::with_capacity(primitives.len());
        #[cfg(any())]
        let mut rail_guids = Vec::with_capacity(map.rails.len());
        let mut ordinal = 0;
        for (rail, (record, package_rail)) in records.iter().zip(&map.rails).enumerate() {
            if package_rail.native.is_none() { return Err(format!("Stock rail {} lacks native cubic payload", package_rail.name)); }
            let source = SourceIdentity {
                stream_file: string(record, "stream_file")?, asset_id: string(record, "asset_id")?,
                section_index: number(record, "section_index")?, section_offset: number(record, "section_offset")?,
            };
            let source_rail = number(record, "rail_index")?;
            let expected_name = format!("{}_{}_{}", source.asset_id, source.section_index, source_rail);
            if package_rail.name != expected_name { return Err(format!("Stock rail provenance/name mismatch: {}", package_rail.name)); }
            let header = 16+rail*32;
            let first = word(header+20) as usize;
            let last = word(header+24) as usize;
            let count = (last-first)/144+1;
            let spline_id = ((word(header) as u64)<<32)|word(header+4) as u64;
            let type_signature = ((word(header+8) as u64)<<32)|word(header+12) as u64;
            #[cfg(any())]
            rail_guids.push([spline_id, type_signature]);
            if parse_id(record, "spline_id")? != spline_id || number(record, "segment_count")? != count as u64
                || parse_id(record, "type_signature")? != type_signature
                || number(record, "flags")? != word(header+16) as u64
                || number(record, "trailing_word")? != word(header+28) as u64
                || record["closed"].as_bool() != Some(package_rail.closed) {
                return Err(format!("Stock rail native identity/count mismatch: {}", package_rail.name));
            }
            let asset = *source_index.entry(source.clone()).or_insert_with(|| {
                let index = grouped.len(); grouped.push((source, Vec::new())); index
            });
            for segment in 0..count {
                let at = first+144*segment;
                let bounds = Bounds {
                    min: std::array::from_fn(|i| f32::from_bits(word(at+80+i*4))),
                    max: std::array::from_fn(|i| f32::from_bits(word(at+96+i*4))),
                };
                spatial_bounds.push(bounds.identity_transformed());
                grouped[asset].1.push(ordinal);
                #[cfg(any())]
                source_for_primitive.push(asset);
                #[cfg(any())]
                source_rail_indices.push(source_rail);
                ordinal+=1;
            }
        }
        let assets = grouped.into_iter().map(|(_source, indices)| {
            let bounds = indices.iter().map(|&i| spatial_bounds[i]).reduce(Bounds::union)
                .ok_or("Empty stock grind source section")?.padded();
            let tree = Octree::new(bounds, indices.iter().map(|&i| spatial_bounds[i]).collect())?;
            Ok(Asset { #[cfg(any())] source: Some(_source), bounds, indices, tree })
        }).collect::<Result<_, String>>()?;
        Ok(Self { primitives, metadata, #[cfg(any())] rail_guids, assets, #[cfg(any())] source_for_primitive, #[cfg(any())] source_rail_indices })
    }

    /// Explicit host-authored polylines have no retail source section identity.
    pub fn authored(rails: &[skate_data::skate_map::Rail]) -> Result<Self, String> {
        let bytes = spline::build_rails(rails)?;
        let (primitives, metadata) = spline::decoded_from_blob(&bytes)?;
        #[cfg(any())]
        let rail_guids = (0..rails.len()).map(|i| {
            let at = 16 + i * 32;
            [u64::from_be_bytes(bytes[at..at+8].try_into().unwrap()),
             u64::from_be_bytes(bytes[at+8..at+16].try_into().unwrap())]
        }).collect();
        let authored_bounds: Vec<_> = primitives.iter().map(|p| Bounds {
            min: std::array::from_fn(|i| p.start[i].min(p.end[i])),
            max: std::array::from_fn(|i| p.start[i].max(p.end[i])),
        }).collect();
        let assets = if let Some(bounds) = authored_bounds.iter().copied().reduce(Bounds::union) {
            let bounds = bounds.padded();
            vec![Asset {
                #[cfg(any())]
                source: Some(SourceIdentity { stream_file: String::new(), asset_id: "host-authored".into(), section_index: 0, section_offset: 0 }),
                bounds, indices: (0..primitives.len()).collect(),
                tree: Octree::new(bounds, authored_bounds.clone())?,
            }]
        } else { vec![] };
        #[cfg(any())]
        let source_rail_indices = primitives.iter().map(|p| p.owner - 1).collect();
        Ok(Self { #[cfg(any())] source_for_primitive: vec![0; primitives.len()], primitives, metadata, #[cfg(any())] rail_guids, assets, #[cfg(any())] source_rail_indices })
    }

    pub fn primitives(&self) -> &[Primitive] { &self.primitives }

    pub fn metadata(&self, primitive: usize) -> Option<&spline::PrimitiveMetadata> {
        self.metadata.get(primitive)
    }

    /// Resolve contact's map-local header handle without confusing it with GUID.
    #[cfg(any())]
    pub fn spline_guids(&self, owner: u64) -> Option<[u64; 2]> {
        let rail = usize::try_from(owner.checked_sub(1)?).ok()?;
        self.rail_guids.get(rail).copied()
    }

    #[cfg(any())]
    pub fn source_rail_index(&self, primitive: usize) -> Option<u64> {
        self.source_rail_indices.get(primitive).copied()
    }

    #[cfg(any())]
    pub fn source(&self, primitive: usize) -> Option<&SourceIdentity> {
        self.source_for_primitive.get(primitive).and_then(|&asset| self.assets[asset].source.as_ref())
    }

    /// S3 82C1EAD8 static pass only: query gate, ordered asset overlap and
    /// native per-asset octree traversal, at most forty original vector indices.
    /// Does not implement or pretend to query unavailable moving providers.
    pub fn query(&self, min: [f32; 3], max: [f32; 3]) -> Result<Vec<usize>, String> {
        if (0..3).any(|i| !min[i].is_finite() || !max[i].is_finite() || min[i] > max[i]) {
            return Err("Invalid grind query bounds".into());
        }
        let query = Bounds { min, max };
        let delta: [f32; 3] = std::array::from_fn(|i| min[i]-max[i]);
        let square = (delta[0]*delta[0]+delta[1]*delta[1])+delta[2]*delta[2];
        if !(square > f32::from_bits(0x3780_0000)) { return Ok(vec![]); }
        let mut result = Vec::new();
        for asset in &self.assets {
            if !asset.bounds.overlaps(query) { continue; }
            for local in asset.tree.query(query, 40-result.len()) { result.push(asset.indices[local]); }
            if result.len() == 40 { break; }
        }
        Ok(result)
    }

    /// `GrindData::Implementation::BoxTest` (82C1EAD8) as its edge users read it: the chord of
    /// each primitive in the box (82C1E568: D -> (A+B)+(C+D)), in [`Self::query`] order. The
    /// host registers the props' splines after the world's, the native static then dynamic
    /// provider order.
    pub fn box_test(&self, min: [f32; 3], max: [f32; 3]) -> Result<Vec<&Primitive>, String> {
        Ok(self.query(min, max)?.into_iter().map(|index| &self.primitives[index]).collect())
    }
}

fn string(value: &Value, key: &str) -> Result<String, String> {
    value[key].as_str().map(str::to_owned).ok_or_else(|| format!("WMET rail missing string {key}"))
}
fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key].as_u64().ok_or_else(|| format!("WMET rail missing unsigned {key}"))
}
fn parse_id(value: &Value, key: &str) -> Result<u64, String> {
    let value = string(value, key)?;
    u64::from_str_radix(value.strip_prefix("0x").unwrap_or(&value), 16)
        .map_err(|_| format!("Invalid WMET {key}"))
}
