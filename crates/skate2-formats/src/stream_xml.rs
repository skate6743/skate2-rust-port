//! Authored collection neighborhoods: StreamTiles/StreamTile in the debug build.

use crate::{
    hash::rw_hash64_string,
    stream::{CollectionSpace, FOCUS_HALF_HEIGHT, FocusCollection, aabb_overlaps},
};
use quick_xml::{Reader, events::Event};
use std::collections::{BTreeMap, HashMap};

/// Parsed names are resolved against the owning simulation collection space.
#[derive(Clone, Debug, Default)]
pub struct StreamXml {
    neighborhoods: BTreeMap<u64, Vec<usize>>,
    /// The document's first tile's collections, when its center is one.
    first: Option<Vec<usize>>,
}

/// One `StreamTile` of a document: its `Center` and the `Tile`s it lists, in order.
struct StreamTile {
    center: [f32; 2],
    tiles: Vec<[f32; 2]>,
}

/// A document's tiles (generated parsers 0x82964ad0/0x82961a50). Unsupported or malformed
/// documents are reported, never treated as empty.
fn stream_tiles(bytes: &[u8]) -> Result<Vec<StreamTile>, &'static str> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    reader.config_mut().expand_empty_elements = true;
    let mut stack = Vec::<Vec<u8>>::new();
    let mut center = [0.0; 2];
    let mut tiles = Vec::new();
    let mut result = Vec::new();
    let mut root_seen = false;
    loop {
        match reader.read_event().map_err(|_| "malformed Stream XML")? {
            Event::Start(element) => {
                let name = element.name();
                match (stack.len(), name.as_ref()) {
                    (0, b"StreamTiles") if !root_seen => root_seen = true,
                    (1, b"StreamTile") => {
                        center = [0.0; 2];
                        tiles.clear();
                    }
                    (2, b"Center" | b"Tile") => {
                        let text = reader
                            .read_text(name)
                            .map_err(|_| "invalid Stream XML vector")?;
                        let text = std::str::from_utf8(text.as_ref())
                            .map_err(|_| "invalid Stream XML encoding")?;
                        let (x, z) = text.split_once(',').ok_or("invalid Stream XML vector")?;
                        let point = [
                            x.trim()
                                .parse::<f32>()
                                .map_err(|_| "invalid Stream XML X")?,
                            z.trim()
                                .parse::<f32>()
                                .map_err(|_| "invalid Stream XML Z")?,
                        ];
                        if !point.iter().all(|v| v.is_finite()) {
                            return Err("nonfinite Stream XML vector");
                        }
                        if name.as_ref() == b"Center" {
                            center = point;
                        } else {
                            tiles.push(point);
                        }
                        continue;
                    }
                    _ => return Err("unsupported Stream XML element"),
                }
                stack.push(name.as_ref().to_vec());
            }
            Event::End(element) => {
                if stack.pop().as_deref() != Some(element.name().as_ref()) {
                    return Err("unbalanced Stream XML");
                }
                if element.name().as_ref() == b"StreamTile" {
                    result.push(StreamTile { center, tiles: std::mem::take(&mut tiles) });
                }
            }
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            _ => return Err("unsupported Stream XML content"),
        }
    }
    if !root_seen || !stack.is_empty() {
        return Err("incomplete Stream XML");
    }
    Ok(result)
}

impl StreamXml {
    /// Generated parsers 0x82964ad0/0x82961a50; collection resolution and
    /// last-definition-wins insertion at 0x824e0830/0x824e0ba8.
    /// Unsupported or malformed documents are reported, never treated as empty.
    pub fn parse(
        bytes: &[u8],
        suffix: &str,
        space: &CollectionSpace,
    ) -> Result<Self, &'static str> {
        Self::parse_tiles(bytes, suffix, space, false)
    }

    /// [`Self::parse`] for a linked Skate 3 data folder's documents: Skate 3's
    /// `ParseXmlStreamTile` (0x8247a760 in its title update 3) reads a tile's
    /// first 64 neighbours with no capacity assertion, a repeated one taken once.
    pub fn parse_skate3(
        bytes: &[u8],
        suffix: &str,
        space: &CollectionSpace,
    ) -> Result<Self, &'static str> {
        Self::parse_tiles(bytes, suffix, space, true)
    }

    fn parse_tiles(
        bytes: &[u8],
        suffix: &str,
        space: &CollectionSpace,
        skate3: bool,
    ) -> Result<Self, &'static str> {
        let neighbours = if skate3 { 64 } else { 16 };
        let mut result = Self::default();
        for (at, tile) in stream_tiles(bytes)?.into_iter().enumerate() {
            let id = collection_id(suffix, tile.center);
            // Native rejects an unavailable center; unavailable neighbors are skipped.
            let neighborhood = match space.collections.iter().position(|entry| entry.id == id) {
                Some(index) => {
                    let mut indices = vec![index];
                    for point in tile.tiles.iter().take(neighbours) {
                        let tile_id = collection_id(suffix, *point);
                        if let Some(index) = space
                            .collections
                            .iter()
                            .position(|entry| entry.id == tile_id)
                        {
                            // The native capacity assertion precedes duplicate detection.
                            if !skate3 && indices.len() >= 16 {
                                return Err("Stream XML exceeds native collection capacity");
                            }
                            if !indices.contains(&index) {
                                indices.push(index);
                            }
                        }
                    }
                    Some(indices)
                }
                None => None,
            };
            if at == 0 {
                result.first.clone_from(&neighborhood);
            }
            if let Some(indices) = neighborhood {
                result.neighborhoods.insert(id, indices);
            }
        }
        Ok(result)
    }

    /// `tStreamFocus::Update` 0x824e1770: a focus's own stream XML (`tStreamFocus::LoadStreamXml`
    /// 0x824d7bb8, `data\stream\<name>.xml`) is the document's first tile alone, its center
    /// first, and only when that center is a collection (`ParseXmlStreamTile` 0x824e0830).
    #[must_use]
    pub fn first_tile(&self) -> Option<&[usize]> {
        self.first.as_deref()
    }

    /// cStreamFile::FindCollections (0x824df068): find the nearest hull within
    /// the 5 x 8000 x 5 box, then select its authored neighborhood. Missing
    /// matches fall back to the ordinary radius query. Sort before applying cap.
    #[must_use]
    pub fn focus_collections(
        &self,
        space: &CollectionSpace,
        focus: [f32; 3],
        radius: f32,
        cap: usize,
    ) -> Vec<FocusCollection> {
        let nearest = space.focus_collections(focus, 5.0, 1);
        let Some(indices) = nearest
            .first()
            .filter(|entry| entry.distance_xz < 1.0e9)
            .and_then(|entry| self.neighborhoods.get(&entry.id))
        else {
            return space.focus_collections(focus, radius, cap);
        };
        let mut selected = indices
            .iter()
            .map(|&index| space.focus_collection(index, focus))
            .collect::<Vec<_>>();
        selected.sort_by(|a, b| a.distance_xz.total_cmp(&b.distance_xz));
        selected.truncate(cap);
        selected
    }
}

/// How many collections a Skate 3 stream record holds (`0x8247c888`'s 384-byte record, 64 from
/// +120, the count at +376).
const RECORD_CAPACITY: usize = 64;

/// What `cAssetStreamSystem::FindCollections` (0x8247b3a0 in Skate 3's title update 3) keeps of a
/// texture focus's selection: its nearest four.
pub const SKATE3_TEXTURE_FOCUS_COLLECTIONS: usize = 4;

/// Half the side of the box a Skate 3 stream record gathers its collections in on x and z: the
/// stream type's extent (`flt_8225185C`: 200 for Pres, 100 for Sim and Tex) less 5 m
/// (`0x8247cb80`).
#[must_use]
pub fn skate3_record_extent(suffix: &str) -> f32 {
    let extent = if suffix.eq_ignore_ascii_case("Pres") { 200.0 } else { 100.0 };
    extent - 5.0
}

/// PC port, a linked Skate 3 world's stream file of one type: the records Skate 3 (title update 3)
/// selects a focus's collections from, in place of Skate 2's spatial query and neighbourhoods.
///
/// `cStreamFile::Update` (0x8247ce58) registers each level-0 collection of the XSS once
/// (`0x8247c888`: its hull, and the hull's centre), and takes the tile size and its centres'
/// offset from the first one's hull on x. `0x8247cb80` then gives each its record: every
/// registered collection whose hull overlaps the box at its centre, [`skate3_record_extent`] out
/// on x and z and 8000 m on y. The world's stream XML (`cStreamFile::LoadStreamXml` 0x8247dbe0,
/// Pres and Sim only) replaces the records of the tiles it lists ([`Self::apply_stream_xml`]). A
/// focus takes the record of the tile it stands in alone ([`Self::focus_collections`]): where no
/// collection is, nothing.
#[derive(Clone, Debug, Default)]
pub struct TileRecords {
    suffix: String,
    /// The tile size and its centres' offset; none without a level-0 collection to take them
    /// from.
    tile: Option<(i32, i32)>,
    /// The registered collections' indices in the XSS, by ID.
    registered: HashMap<u64, usize>,
    /// Each registered collection's record: the XSS indices of the collections it selects.
    records: HashMap<u64, Vec<usize>>,
}

impl TileRecords {
    /// The records of `space`, a stream file of type `suffix` (`Pres`, `Sim`, `Tex`), before any
    /// stream XML.
    #[must_use]
    pub fn new(space: &CollectionSpace, suffix: &str) -> Self {
        let mut order = Vec::new();
        let mut registered = HashMap::new();
        let mut tile = None;
        for (index, entry) in space.collections.iter().enumerate() {
            if entry.lod != 0 || registered.contains_key(&entry.id) {
                continue;
            }
            registered.insert(entry.id, index);
            order.push(index);
            if tile.is_none() {
                tile = tile_of(entry.hull_min[0], entry.hull_max[0]);
            }
        }
        let extent = skate3_record_extent(suffix);
        let records = order
            .iter()
            .map(|&index| {
                let entry = &space.collections[index];
                let center: [f32; 3] = std::array::from_fn(|axis| entry.hull_min[axis] + (entry.hull_max[axis] - entry.hull_min[axis]) * 0.5);
                let min = [center[0] - extent, center[1] - FOCUS_HALF_HEIGHT, center[2] - extent];
                let max = [center[0] + extent, center[1] + FOCUS_HALF_HEIGHT, center[2] + extent];
                let held = order
                    .iter()
                    .copied()
                    .filter(|&other| aabb_overlaps(space.collections[other].hull_min, space.collections[other].hull_max, min, max))
                    .take(RECORD_CAPACITY)
                    .collect();
                (entry.id, held)
            })
            .collect();
        Self { suffix: suffix.to_owned(), tile, registered, records }
    }

    /// A stream XML's tiles (`data\stream\<world>_<type>.xml`), as `cStreamFile::Update` reads
    /// them once loaded: each tile whose centre is a registered collection gets the record of the
    /// centre, then of the first 64 tiles it lists those that are registered, each once
    /// (`ParseXmlStreamTile` 0x8247a760, which tells repeats apart by the IDs' low words); a later
    /// tile of the same centre replaces it. A tile whose centre is none changes nothing.
    ///
    /// # Errors
    ///
    /// A malformed or unsupported document, which changes no record.
    pub fn apply_stream_xml(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        for tile in stream_tiles(bytes)? {
            let center = collection_id(&self.suffix, tile.center);
            if !self.registered.contains_key(&center) {
                continue;
            }
            let mut ids = vec![center];
            for point in tile.tiles.iter().take(RECORD_CAPACITY) {
                let id = collection_id(&self.suffix, *point);
                if self.registered.contains_key(&id) && !ids.iter().any(|&held| held & 0xffff_ffff == id & 0xffff_ffff) {
                    ids.push(id);
                }
            }
            ids.truncate(RECORD_CAPACITY);
            let held = ids.iter().map(|id| self.registered[id]).collect();
            self.records.insert(center, held);
        }
        Ok(())
    }

    /// The collection naming the tile `focus` stands in (`0x8247d9b0`): `c<type>_<x>_<z>_high` at
    /// the tile's centre, from the position truncated toward zero, a negative one on a tile's edge
    /// counting in the tile below; none without a tile size.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // The reference converts with fctiwz.
    pub fn tile_collection(&self, focus: [f32; 3]) -> Option<u64> {
        let (size, offset) = self.tile?;
        let low = offset.wrapping_sub(size / 2);
        let center = |at: f32| {
            let at = at as i32;
            let cell = at.wrapping_sub(low).checked_div(size)?;
            let cell = if at < low { cell.wrapping_sub(1) } else { cell };
            Some(cell.wrapping_mul(size).wrapping_add(offset))
        };
        let (x, z) = (center(focus[0])?, center(focus[2])?);
        Some(rw_hash64_string(&format!("c{}_{x}_{z}_high", self.suffix)))
    }

    /// `cAssetStreamSystem::FindCollections` (0x8247b3a0) on this stream file at `focus`: the
    /// record of the tile it stands in (`0x8247dab0`), nearest first by the square of the distance
    /// on x and z (a stable sort), a texture focus keeping its nearest
    /// [`SKATE3_TEXTURE_FOCUS_COLLECTIONS`]; none where the tile is no collection. `space` is the
    /// XSS the records were made from.
    #[must_use]
    pub fn focus_collections(&self, space: &CollectionSpace, focus: [f32; 3]) -> Vec<FocusCollection> {
        let Some(record) = self.tile_collection(focus).and_then(|tile| self.records.get(&tile)) else {
            return Vec::new();
        };
        let square = |entry: &FocusCollection| {
            let x = focus[0] - entry.position[0];
            let z = focus[2] - entry.position[2];
            x * x + z * z
        };
        let mut selected: Vec<FocusCollection> = record
            .iter()
            .filter(|&&index| index < space.collections.len())
            .map(|&index| space.focus_collection(index, focus))
            .collect();
        selected.sort_by(|left, right| square(left).total_cmp(&square(right)));
        if self.suffix.eq_ignore_ascii_case("Tex") {
            selected.truncate(SKATE3_TEXTURE_FOCUS_COLLECTIONS);
        }
        selected
    }
}

/// The tile size and its centres' offset `cStreamFile::Update` takes from a level-0 collection's
/// hull on x: the width of the truncated bounds, and half of it less the truncated maximum's
/// magnitude modulo the width (unsigned); none for no width (a division by zero natively).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn tile_of(min: f32, max: f32) -> Option<(i32, i32)> {
    let (min, max) = (min as i32, max as i32);
    let width = max.wrapping_sub(min);
    let size = (if width < 0 { min.wrapping_sub(max) } else { width }) as u32;
    if size == 0 {
        return None;
    }
    let offset = (size >> 1).wrapping_sub(max.unsigned_abs() % size);
    Some((size as i32, offset as i32))
}

fn collection_id(suffix: &str, point: [f32; 2]) -> u64 {
    rw_hash64_string(&format!(
        "c{suffix}_{:.0}_{:.0}_high",
        (point[0] + 0.5).floor(),
        (point[1] + 0.5).floor()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::CollectionSpaceEntry;

    fn space() -> CollectionSpace {
        CollectionSpace {
            version: 3,
            stamp: 1,
            collections: [100.0, 300.0, 900.0]
                .into_iter()
                .map(|x| CollectionSpaceEntry {
                    id: collection_id("Sim", [x, 100.0]),
                    lod: 0,
                    hull_min: [x - 100.0, -100.0, 0.0, 1.0],
                    hull_max: [x + 100.0, 100.0, 200.0, 1.0],
                    geometry_min: [0.0; 4],
                    geometry_max: [0.0; 4],
                })
                .collect(),
        }
    }

    #[test]
    fn authored_neighbors_override_radius_and_missing_centers_fall_back() {
        let space = space();
        let xml = StreamXml::parse(br#"<?xml version="1.0"?><StreamTiles>
          <StreamTile><Center>100,100</Center><Tile>900,100</Tile><Tile>900,100</Tile><Tile>1500,100</Tile></StreamTile>
        </StreamTiles>"#, "Sim", &space).unwrap();
        let selected = xml.focus_collections(&space, [100.0, 0.0, 100.0], 195.0, 4);
        assert_eq!(selected.iter().map(|e| e.index).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(
            xml.focus_collections(&space, [100.0, 0.0, 100.0], 195.0, 1)
                .len(),
            1
        );
        assert_eq!(
            xml.focus_collections(&space, [300.0, 0.0, 100.0], 195.0, 4),
            space.focus_collections([300.0, 0.0, 100.0], 195.0, 4)
        );
        let empty = StreamXml::parse(b"<StreamTiles/>", "Sim", &space).unwrap();
        assert!(empty.neighborhoods.is_empty());
        assert!(StreamXml::parse(b"<StreamTiles><StreamTile>", "Sim", &space).is_err());
    }

    #[test]
    fn a_focus_takes_the_first_tile_only_when_its_center_is_a_collection() {
        let space = space();
        let xml = StreamXml::parse(br#"<StreamTiles>
          <StreamTile><Center>300,100</Center><Tile>100,100</Tile><Tile>700,100</Tile><Tile>900,100</Tile></StreamTile>
          <StreamTile><Center>100,100</Center><Tile>900,100</Tile></StreamTile>
        </StreamTiles>"#, "Sim", &space).unwrap();
        assert_eq!(xml.first_tile(), Some(&[1, 0, 2][..]));
        let missing_center = StreamXml::parse(br#"<StreamTiles>
          <StreamTile><Center>500,100</Center><Tile>100,100</Tile></StreamTile>
          <StreamTile><Center>100,100</Center><Tile>900,100</Tile></StreamTile>
        </StreamTiles>"#, "Sim", &space).unwrap();
        assert_eq!(missing_center.first_tile(), None);
    }

    fn tiles(suffix: &str, centers: &[[f32; 2]]) -> CollectionSpace {
        CollectionSpace {
            version: 3,
            stamp: 1,
            collections: centers
                .iter()
                .map(|&[x, z]| CollectionSpaceEntry {
                    id: collection_id(suffix, [x, z]),
                    lod: 0,
                    hull_min: [x - 50.0, -10.0, z - 50.0, 1.0],
                    hull_max: [x + 50.0, 10.0, z + 50.0, 1.0],
                    geometry_min: [0.0; 4],
                    geometry_max: [0.0; 4],
                })
                .collect(),
        }
    }

    #[test]
    fn skate3_records_hold_the_tiles_round_each_until_the_xml_replaces_them() {
        let row: Vec<[f32; 2]> = [-250.0, -150.0, -50.0, 50.0, 150.0, 250.0, 350.0].iter().map(|&x| [x, 50.0]).collect();
        let space = tiles("Pres", &row);
        let mut records = TileRecords::new(&space, "Pres");
        let xs = |selected: &[FocusCollection]| selected.iter().map(|entry| entry.position[0]).collect::<Vec<_>>();
        // The tile at x 50 holds the tiles whose hulls reach within 195 m of its centre, nearest
        // the focus first.
        assert_eq!(xs(&records.focus_collections(&space, [60.0, 0.0, 40.0])), [50.0, 150.0, -50.0, 250.0, -150.0]);
        assert_eq!(records.focus_collections(&space, [160.0, 0.0, 40.0]).len(), 5);
        // Positions truncate toward zero; a negative one on a tile's edge counts in the tile below.
        assert_eq!(records.tile_collection([-0.5, 0.0, 40.0]), Some(collection_id("Pres", [50.0, 50.0])));
        assert_eq!(records.tile_collection([-1.0, 0.0, 40.0]), Some(collection_id("Pres", [-50.0, 50.0])));
        assert_eq!(records.tile_collection([-100.0, 0.0, 40.0]), Some(collection_id("Pres", [-150.0, 50.0])));
        // A tile with no collection selects nothing.
        assert!(records.focus_collections(&space, [1000.0, 0.0, 40.0]).is_empty());
        records
            .apply_stream_xml(
                br#"<StreamTiles>
          <StreamTile><Center>50, 50</Center><Tile>350, 50</Tile><Tile>350, 50</Tile><Tile>950, 50</Tile><Tile>50, 50</Tile></StreamTile>
          <StreamTile><Center>650, 50</Center><Tile>50, 50</Tile></StreamTile>
        </StreamTiles>"#,
            )
            .unwrap();
        assert_eq!(xs(&records.focus_collections(&space, [60.0, 0.0, 40.0])), [50.0, 350.0]);
        assert_eq!(records.focus_collections(&space, [160.0, 0.0, 40.0]).len(), 5);
        assert!(records.apply_stream_xml(b"<StreamTiles><StreamTile>").is_err());
        assert_eq!(xs(&records.focus_collections(&space, [60.0, 0.0, 40.0])), [50.0, 350.0]);
    }

    #[test]
    fn skate3_texture_records_keep_the_four_nearest() {
        let axis = [-100.0, 0.0, 100.0];
        let grid: Vec<[f32; 2]> = axis.iter().flat_map(|&x| axis.iter().map(move |&z| [x, z])).collect();
        let space = tiles("Tex", &grid);
        let records = TileRecords::new(&space, "Tex");
        // Tiles centred on multiples of 100: the first hull's maximum, -50, puts the centres 0 past
        // them.
        assert_eq!(records.tile_collection([30.0, 0.0, -30.0]), Some(collection_id("Tex", [0.0, 0.0])));
        let selected = records.focus_collections(&space, [30.0, 0.0, 20.0]);
        assert_eq!(
            selected.iter().map(|entry| [entry.position[0], entry.position[2]]).collect::<Vec<_>>(),
            [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0], [100.0, 100.0]]
        );
    }

    #[test]
    fn later_definition_replaces_earlier_neighborhood() {
        let space = space();
        let xml = StreamXml::parse(b"<StreamTiles><StreamTile><Center>100,100</Center><Tile>900,100</Tile></StreamTile><StreamTile><Center>100,100</Center></StreamTile></StreamTiles>", "Sim", &space).unwrap();
        assert_eq!(
            xml.focus_collections(&space, [100.0, 0.0, 100.0], 195.0, 4)
                .len(),
            1
        );
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn original_sim_xml_selects_spawn_and_moving_neighborhoods() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let archives = std::fs::read_dir(root.join("data/big"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "big"))
            .map(|p| crate::mount::MountedBig::open(p, true).unwrap())
            .collect::<Vec<_>>();
        let read = |path: &str| {
            archives
                .iter()
                .find_map(|archive| archive.read_path(path).ok())
                .expect(path)
        };
        let bytes = read("data/stream/BAM_Sim.xml");
        let world =
            crate::mount::MountedBig::open(root.join("data/content/worldbam.big"), false).unwrap();
        let space = CollectionSpace::parse(
            &world
                .read_path("data/content/world/stream/BAM/BAM_Sim.xss")
                .unwrap(),
        )
        .unwrap();
        let xml = StreamXml::parse(&bytes, "Sim", &space).unwrap();
        assert!(!xml.neighborhoods.is_empty());
        for position in [
            [588.8617, -54.231647, 996.5048],
            [701.0, -54.231647, 996.5048],
        ] {
            let nearest = space.focus_collections(position, 5.0, 1);
            let indices = xml
                .neighborhoods
                .get(&nearest[0].id)
                .expect("authored neighborhood");
            let selected = xml.focus_collections(&space, position, 195.0, 4);
            assert!(!selected.is_empty());
            assert!(selected.iter().all(|entry| indices.contains(&entry.index)));
            println!(
                "position={position:?} authored={} selected={:?}",
                indices.len(),
                selected
                    .iter()
                    .map(|e| (e.id, e.position))
                    .collect::<Vec<_>>()
            );
        }
        println!(
            "XML bytes={} valid neighborhoods={}",
            bytes.len(),
            xml.neighborhoods.len()
        );
    }
}
