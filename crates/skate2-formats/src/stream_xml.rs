//! Authored collection neighborhoods: StreamTiles/StreamTile in the debug build.

use crate::{
    hash::rw_hash64_string,
    stream::{CollectionSpace, FocusCollection},
};
use quick_xml::{Reader, events::Event};
use std::collections::BTreeMap;

/// Parsed names are resolved against the owning simulation collection space.
#[derive(Clone, Debug, Default)]
pub struct StreamXml {
    neighborhoods: BTreeMap<u64, Vec<usize>>,
    /// The document's first tile's collections, when its center is one.
    first: Option<Vec<usize>>,
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
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        reader.config_mut().expand_empty_elements = true;
        let mut stack = Vec::<Vec<u8>>::new();
        let mut center = [0.0; 2];
        let mut tiles = Vec::new();
        let mut result = Self::default();
        let mut root_seen = false;
        let mut first_seen = false;
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
                        let id = collection_id(suffix, center);
                        // Native rejects an unavailable center; unavailable neighbors are skipped.
                        let neighborhood = match space.collections.iter().position(|entry| entry.id == id) {
                            Some(index) => {
                                let mut indices = vec![index];
                                for point in tiles.iter().take(neighbours) {
                                    let tile_id = collection_id(suffix, *point);
                                    if let Some(index) = space
                                        .collections
                                        .iter()
                                        .position(|entry| entry.id == tile_id)
                                    {
                                        // The native capacity assertion precedes duplicate detection.
                                        if !skate3 && indices.len() >= 16 {
                                            return Err(
                                                "Stream XML exceeds native collection capacity",
                                            );
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
                        if !first_seen {
                            first_seen = true;
                            result.first.clone_from(&neighborhood);
                        }
                        if let Some(indices) = neighborhood {
                            result.neighborhoods.insert(id, indices);
                        }
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
