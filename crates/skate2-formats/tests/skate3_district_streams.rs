//! A linked Skate 3 data folder's district streams through the game's stream tables: its EB
//! archives mount, each district's Sim, Pres and Tex tables parse, the level-0 collections are
//! named on Skate 3's tiles, and every collection's SFIL parses with the asset count its map
//! gives. Set `SKATE3_DATA` to the folder holding `default.xex` and `data`.

use skate2_formats::{
    hash::rw_hash64_string,
    mount::{ArchiveSource, MountSet, MountedBig},
    stream::{AssetToc, CollectionMap, CollectionSpace, CompressedArenaHeader, StreamFile, skate3_high_detail_tile, tiled_high_detail_collection_filename},
    stream_xml::TileRecords,
};
use std::path::PathBuf;

const DISTRICTS: [&str; 3] = ["DIST_University", "DIST_DownTown", "DIST_Industrial"];

fn data() -> Option<PathBuf> {
    std::env::var_os("SKATE3_DATA").map(PathBuf::from)
}

#[test]
fn district_streams_parse_on_skate3_tiles() {
    let Some(root) = data() else { return };
    let mut mounts = MountSet::new();
    for district in DISTRICTS {
        let path = root.join("data").join("content").join(format!("world{district}.big"));
        mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("district archive mounts"));
    }
    for district in DISTRICTS {
        for kind in ["Sim", "Pres", "Tex"] {
            let base = format!("data\\content\\world\\stream\\{district}\\{district}_{kind}");
            let toc = AssetToc::parse(&mounts.read_path(&format!("{base}.xst")).unwrap()).unwrap();
            let map = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm")).unwrap()).unwrap();
            let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss")).unwrap()).unwrap();
            let tile = skate3_high_detail_tile(kind);
            let mut level0 = 0;
            let mut named = 0;
            for entry in space.collections.iter().filter(|entry| entry.lod == 0) {
                level0 += 1;
                let centre = [
                    (entry.geometry_min[0] + entry.geometry_max[0]) * 0.5,
                    0.0,
                    (entry.geometry_min[2] + entry.geometry_max[2]) * 0.5,
                ];
                if rw_hash64_string(&tiled_high_detail_collection_filename(kind, centre, tile)) == entry.id {
                    named += 1;
                } else if kind != "Tex" {
                    let name = map.collections.iter().find(|c| c.id == entry.id).map_or("?", |c| c.filename.as_str());
                    eprintln!("{district} {kind}: level-0 {name} at {centre:?} is not a tile");
                }
            }
            let mut assets = 0;
            for collection in &map.collections {
                let path = format!("data\\content\\world\\stream\\{district}\\{}", collection.filename);
                let path = if mounts.contains_path(&path) { path } else { format!("{path}.xsf") };
                let bytes = mounts.read_path(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
                let file = StreamFile::parse(&bytes, collection.asset_count).unwrap_or_else(|e| panic!("{path}: {e:?}"));
                for asset in &file.assets {
                    CompressedArenaHeader::parse(&bytes[asset.data_range.clone()]).unwrap_or_else(|e| panic!("{path}: {e:?}"));
                }
                assets += file.assets.len();
            }
            eprintln!(
                "{district} {kind}: toc {} assets, map {} collections, space {} ({} level 0, {} named on Skate 3 tiles), {} collection assets",
                toc.assets.len(),
                map.collections.len(),
                space.collections.len(),
                level0,
                named,
                assets
            );
            assert!(level0 > 0, "{district} {kind}: no level-0 collections");
            // A Tex collection's geometry reaches past its tile, so only Sim and Pres centres name it.
            if kind != "Tex" {
                assert!(named + 2 >= level0, "{district} {kind}: level-0 collections are named on Skate 3's tiles");
            }
        }
    }
}

/// Every district Sim arena decodes with the game's arena decoder (the spots' start and waiting
/// locators are in their mission hulls, below, not in the districts).
#[test]
fn district_sim_arenas_decode() {
    let Some(root) = data() else { return };
    let mut mounts = MountSet::new();
    for district in DISTRICTS {
        let path = root.join("data").join("content").join(format!("world{district}.big"));
        mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("district archive mounts"));
    }
    for district in DISTRICTS {
        let base = format!("data\\content\\world\\stream\\{district}\\{district}_Sim");
        let map = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm")).unwrap()).unwrap();
        let mut decoded = 0;
        for collection in &map.collections {
            let path = format!("data\\content\\world\\stream\\{district}\\{}", collection.filename);
            let path = if mounts.contains_path(&path) { path } else { format!("{path}.xsf") };
            let bytes = mounts.read_path(&path).unwrap();
            let file = StreamFile::parse(&bytes, collection.asset_count).unwrap();
            for asset in &file.assets {
                skate2_formats::stream::decode_arena(&bytes[asset.data_range.clone()]).unwrap_or_else(|e| panic!("{path}: {e:?}"));
                decoded += 1;
            }
        }
        eprintln!("{district}: {decoded} Sim arenas decoded");
        assert!(decoded > 0);
    }
}

/// Each online S.K.A.T.E. spot's own mission stream (`missions.big`, the challenge's hull that
/// `LoadChallengeAssets` streams in, `data\content\missions\<name>`) decodes with the game's
/// arena decoder and holds the spot's start and waiting locators.
#[test]
fn skate_mission_hulls_decode_and_hold_their_locators() {
    let Some(root) = data() else { return };
    let mut mounts = MountSet::new();
    let path = root.join("data").join("content").join("missions.big");
    mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("missions archive mounts"));
    let spots = [
        ("skate_univ_01", ""), ("skate_univ_02", ""), ("skate_univ_03", ""),
        ("skate_dwtn_02", ""), ("skate_dwtn_03", ""), ("skate_dwtn_04", ""),
        ("skate_indu_01", "_01"), ("skate_indu_02", "_01"), ("skate_indu_03", "_01"),
    ];
    for (name, suffix) in spots {
        let folder = format!("data\\content\\missions\\{name}");
        let map = CollectionMap::parse(&mounts.read_path(&format!("{folder}\\{name}_Sim.xsm")).unwrap()).unwrap();
        let mut wanted = vec![format!("{name}_startlocator{suffix}"), format!("{name}_waitlocator{suffix}")];
        let mut decoded = 0;
        for collection in &map.collections {
            let path = format!("{folder}\\{}", collection.filename);
            let path = if mounts.contains_path(&path) { path } else { format!("{path}.xsf") };
            let bytes = mounts.read_path(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            let file = StreamFile::parse(&bytes, collection.asset_count).unwrap();
            for asset in &file.assets {
                let arena = skate2_formats::stream::decode_arena(&bytes[asset.data_range.clone()]).unwrap_or_else(|e| panic!("{path}: {e:?}"));
                decoded += 1;
                wanted.retain(|locator| {
                    let needle = locator.to_ascii_lowercase();
                    !arena.resources.iter().any(|resource| resource.windows(needle.len()).any(|window| window.eq_ignore_ascii_case(needle.as_bytes())))
                });
            }
        }
        eprintln!("{name}: {} collections, {decoded} arenas; locators not found: {wanted:?}", map.collections.len());
        assert!(wanted.is_empty(), "{name}: {wanted:?}");
    }
}

/// Skate 3's clothing database (`cas_db_xfd1ca62f_000.recipe`) through the game's part database
/// reader: its slots and the parts' model and material ids, and how many Skate 2 shares.
#[test]
fn skate3_clothing_database_reads_as_the_games() {
    let Some(root) = data() else { return };
    let mut mounts = MountSet::new();
    let path = root.join("data").join("content").join("createacharacter.big");
    mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("clothing archive mounts"));
    let bytes = mounts.read_path("data\\content\\recipe\\createacharacter\\cas_db_xfd1ca62f_000.recipe").unwrap();
    let db = skate2_formats::cac_parts::CacPartDb::parse(&bytes).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("skate 3 parts: {}, strings: {}", db.parts.len(), db.strings.len());
    for (slot, name) in skate2_formats::cac_parts::SLOT_NAMES.iter().enumerate() {
        let range = db.slot_range(u8::try_from(slot).unwrap());
        // Native attribute ids: 15 Gender, 36 material name, 38 model name, 34 brand, 51 top type.
        let first = db.parts.get(range.start).map(|part| {
            [15, 38, 36, 34, 51].map(|attribute| db.attribute(part, attribute).unwrap_or("?").to_owned())
        });
        eprintln!("  {name}: {} parts, first {:?}", range.len(), first);
        for part in db.parts_in_slot(u8::try_from(slot).unwrap()) {
            assert_eq!(usize::from(part.slot), slot, "{name}: a part reads its own slot");
            let gender = db.attribute(part, 15).unwrap();
            assert!(matches!(gender, "male" | "female" | "" | "both" | "unisex"), "{name}: gender {gender:?}");
        }
    }
    if let Ok(skate2) = std::env::var("SKATE2_CAS_DB") {
        let skate2 = skate2_formats::cac_parts::CacPartDb::parse(&std::fs::read(skate2).unwrap()).unwrap();
        let shared = db.parts.iter().filter(|part| skate2.find(part.slot, part.model_id, part.material_id).is_some()).count();
        let merged = skate2.with_skate3(&db, &[0, 1, 5, 6, 11, 12, 13, 15, 17, 18, 20, 21, 22, 23, 25]).unwrap();
        let joined = (0..merged.parts.len()).filter(|&i| merged.is_skate3(i)).count();
        eprintln!("parts Skate 2 has too: {shared} of {}; merged {} ({joined} Skate 3), strings {}", db.parts.len(), merged.parts.len(), merged.strings.len());
        assert_eq!(merged.parts.len(), skate2.parts.len() + joined);
        for i in (0..merged.parts.len()).filter(|&i| merged.is_skate3(i)).take(3) {
            let part = &merged.parts[i];
            eprintln!("  joined {} {} {}", skate2_formats::cac_parts::SLOT_NAMES[usize::from(part.slot)], merged.attribute(part, 38).unwrap(), merged.attribute(part, 36).unwrap());
        }
    }
}

/// Every Skate 3 part Slappy's would list once joined (a clothing, accessory or board slot, used
/// in game) has its model in the joined master recipe, each LOD's geometry in Skate 3's
/// `createacharacter.big` where the recipe's asset requests look, and its thumbnail in
/// `fedynamic.big` where the movie's `GetThumbnailPath` looks.
#[test]
fn joined_skate3_clothing_has_its_models_and_thumbnails() {
    let (Some(root), Ok(skate2)) = (data(), std::env::var("SKATE2_CAS_DB")) else { return };
    let mut mounts = MountSet::new();
    for archive in ["content\\createacharacter.big", "big\\fedynamic.big"] {
        let path = root.join("data").join(archive.replace("\\", "/"));
        mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("archive mounts"));
    }
    let skate3 = skate2_formats::cac_parts::CacPartDb::parse(&mounts.read_path("data\\content\\recipe\\createacharacter\\cas_db_xfd1ca62f_000.recipe").unwrap()).unwrap();
    let skate2 = skate2_formats::cac_parts::CacPartDb::parse(&std::fs::read(skate2).unwrap()).unwrap();
    let merged = skate2.with_skate3(&skate3, &[0, 1, 5, 6, 11, 12, 13, 15, 17, 18, 20, 21, 22, 23, 25]).unwrap();
    let listed_slots = [5_u8, 6, 11, 12, 13, 15, 17, 18, 20, 21, 23, 25];
    let (mut parts, mut missing_models, mut missing_files, mut missing_thumbnails) = (0, Vec::new(), Vec::new(), Vec::new());
    for (index, part) in merged.parts.iter().enumerate() {
        if !merged.is_skate3(index) || !listed_slots.contains(&part.slot) || (part.used_in_game != 0 && part.used_in_game != 255) {
            continue;
        }
        parts += 1;
        let slot = skate2_formats::cac_parts::SLOT_NAMES[usize::from(part.slot)];
        let Some(model) = merged.master.components.iter().find(|c| c.name == slot).and_then(|c| c.models.iter().find(|m| m.asset_id == part.model_id)) else {
            missing_models.push(format!("{slot} {:016x}", part.model_id));
            continue;
        };
        for lod in &model.lods {
            let path = format!("data\\content\\createacharacter\\model\\{}\\{slot}\\0x{:016x}.rx2", merged.master.schema_name, lod.arena_asset_id);
            if !mounts.contains_path(&path) {
                missing_files.push(path);
            }
        }
        let gender = merged.attribute(part, 15).unwrap();
        let prefix = if gender == "male" || part.slot == 5 { "male\\" } else if gender == "female" { "female\\" } else { "" };
        let thumbnail = format!("data\\fe\\cas\\{prefix}{slot}\\{}_{}.rx2", merged.attribute(part, 38).unwrap(), merged.attribute(part, 36).unwrap()).to_ascii_lowercase();
        let alternatives = ["male\\", "female\\", ""].map(|other| thumbnail.replacen(&format!("cas\\{prefix}"), &format!("cas\\{other}"), 1));
        if !mounts.contains_path(&thumbnail) && !alternatives.iter().any(|path| mounts.contains_path(path)) {
            missing_thumbnails.push(format!("{slot}: {thumbnail}"));
        }
    }
    eprintln!("listed Skate 3 parts {parts}: models missing {}, geometry files missing {}, thumbnails missing {}", missing_models.len(), missing_files.len(), missing_thumbnails.len());
    eprintln!("  e.g. {:?} {:?} {:?}", missing_models.first(), missing_files.first(), missing_thumbnails.iter().take(3).collect::<Vec<_>>());
    let mut by_slot = std::collections::BTreeMap::<&str, usize>::new();
    for missing in &missing_thumbnails {
        *by_slot.entry(missing.split(':').next().unwrap()).or_default() += 1;
    }
    eprintln!("  thumbnails missing by slot: {by_slot:?}");
    assert!(missing_models.is_empty(), "every listed part's model is in the joined master");
    assert!(missing_files.is_empty(), "every listed part's geometry is in Skate 3's archive");
}

/// The district proxy streams (`<District>_100_Proxy`), which Skate 3 registers together as one
/// city: the collections more than one of them holds.
#[test]
fn district_proxy_streams_overlap() {
    let Some(root) = data() else { return };
    let mut maps = Vec::new();
    for district in ["University", "Downtown", "Industrial"] {
        let path = root.join("data").join("content").join(format!("proxy{district}_100_Proxy.big"));
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("proxy archive mounts"));
        let stream = format!("{district}_100_Proxy");
        let base = format!("data\\content\\world\\stream\\{stream}\\{stream}_Pres");
        let map = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm")).unwrap()).unwrap();
        maps.push((district, map));
    }
    let mut shared = 0;
    for (index, (district, map)) in maps.iter().enumerate() {
        for entry in &map.collections {
            for (other, other_map) in &maps[index + 1..] {
                if let Some(twin) = other_map.collections.iter().find(|c| c.id == entry.id) {
                    shared += 1;
                    eprintln!(
                        "{district}/{other}: {:016x} {} (parent {:016x}, {} assets, {} bytes) | {} (parent {:016x}, {} assets, {} bytes)",
                        entry.id, entry.filename, entry.parent_id, entry.asset_count, entry.payload_size,
                        twin.filename, twin.parent_id, twin.asset_count, twin.payload_size
                    );
                }
            }
        }
    }
    eprintln!("{shared} shared; collections {:?}", maps.iter().map(|(d, m)| (*d, m.collections.len())).collect::<Vec<_>>());
}

#[test]
#[ignore = "research probe"]
fn probe_district_stream_lods() {
    let Some(root) = data() else { return };
    let mut mounts = MountSet::new();
    for district in DISTRICTS {
        let path = root.join("data").join("content").join(format!("world{district}.big"));
        mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("district archive mounts"));
    }
    for district in DISTRICTS {
        for kind in ["Pres", "Tex"] {
            let base = format!("data\\content\\world\\stream\\{district}\\{district}_{kind}");
            let map = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm")).unwrap()).unwrap();
            let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss")).unwrap()).unwrap();
            let mut lods: std::collections::BTreeMap<u8, (usize, f32, f32, Vec<String>)> = std::collections::BTreeMap::new();
            for entry in &space.collections {
                let size = (entry.hull_max[0] - entry.hull_min[0]).max(entry.hull_max[2] - entry.hull_min[2]);
                let name = map.collections.iter().find(|c| c.id == entry.id).map_or("?".to_owned(), |c| c.filename.clone());
                let row = lods.entry(entry.lod).or_insert((0, f32::MAX, 0.0, Vec::new()));
                row.0 += 1;
                row.1 = row.1.min(size);
                row.2 = row.2.max(size);
                if row.3.len() < 4 {
                    row.3.push(name);
                }
            }
            eprintln!("{district} {kind}: {} in space, {} in map", space.collections.len(), map.collections.len());
            for (lod, (count, min, max, names)) in lods {
                eprintln!("  lod {lod}: {count} collections, hull {min}..{max} m, e.g. {names:?}");
            }
            let unspaced: Vec<&str> = map.collections.iter().filter(|c| !space.collections.iter().any(|e| e.id == c.id)).map(|c| c.filename.as_str()).take(6).collect();
            eprintln!("  map-only e.g. {unspaced:?}");
            let mut parents: std::collections::BTreeMap<u64, usize> = std::collections::BTreeMap::new();
            for c in &map.collections {
                *parents.entry(c.parent_id).or_default() += 1;
            }
            for (parent, count) in &parents {
                let name = map.collections.iter().find(|c| c.id == *parent).map_or("-", |c| c.filename.as_str());
                eprintln!("  parent {parent:016x} ({name}): {count} children");
            }
            for c in map.collections.iter().filter(|c| c.filename.to_ascii_lowercase().contains("global")) {
                eprintln!("  {}: id {:016x} parent {:016x} assets {} bytes {}", c.filename, c.id, c.parent_id, c.asset_count, c.payload_size);
            }
            let toc = AssetToc::parse(&mounts.read_path(&format!("{base}.xst")).unwrap()).unwrap();
            let samples: Vec<&skate2_formats::stream::CollectionMapEntry> = map
                .collections
                .iter()
                .filter(|c| c.filename.to_ascii_lowercase().contains("global"))
                .chain(map.collections.iter().filter(|c| !c.filename.to_ascii_lowercase().contains("global")).take(1))
                .collect();
            for collection in samples {
                let path = format!("data\\content\\world\\stream\\{district}\\{}", collection.filename);
                let path = if mounts.contains_path(&path) { path } else { format!("{path}.xsf") };
                let bytes = mounts.read_path(&path).unwrap();
                let file = StreamFile::parse(&bytes, collection.asset_count).unwrap();
                let mut types: std::collections::BTreeMap<u32, (usize, usize)> = std::collections::BTreeMap::new();
                for asset in &file.assets {
                    let kind = toc.assets.iter().find(|a| a.id == asset.id).map_or(0, |a| a.asset_type);
                    let row = types.entry(kind).or_default();
                    row.0 += 1;
                    row.1 += usize::from(asset.id & (1 << 62) != 0);
                }
                let first: Vec<String> = file.assets.iter().take(3).map(|a| format!("{:016x}", a.id)).collect();
                eprintln!("  {} assets by type (count, low-res bit 62): {types:x?} e.g. {first:?}", collection.filename);
            }
        }
    }
}

#[test]
#[ignore = "research probe"]
fn probe_world_misc() {
    let Some(root) = data() else { return };
    {
        let big = MountedBig::open_source(ArchiveSource::File(root.join("data").join("big").join("db.big"))).unwrap();
        for entry in &big.archive().entries {
            let skate2_formats::big::BigEntryKey::Name(name) = &entry.key else { continue };
            if name.to_ascii_lowercase().ends_with(".txt") {
                let out = std::path::Path::new(&std::env::var("SKATE3_DUMP").unwrap()).join(name.rsplit(['\\', '/']).next().unwrap());
                std::fs::write(&out, big.read_path(name).unwrap()).unwrap();
            }
        }
    }
    for folder in ["content", "big"] {
        for file in std::fs::read_dir(root.join("data").join(folder)).unwrap().flatten() {
            let path = file.path();
            if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("big")) {
                continue;
            }
            let Ok(big) = MountedBig::open_source(ArchiveSource::File(path.clone())) else { continue };
            for entry in &big.archive().entries {
                let skate2_formats::big::BigEntryKey::Name(name) = &entry.key else { continue };
                let lower = name.to_ascii_lowercase();
                if lower.contains("sky") || lower.contains("water") || lower.contains("ocean") || lower.contains("world\\models") || lower.contains("dist_") && lower.ends_with(".rx2") {
                    eprintln!("{}: {name} {} bytes", path.file_name().unwrap().to_string_lossy(), entry.unpacked_size);
                }
            }
        }
    }
}

/// A Skate 3 online game's own stream XML (`data\\stream\\<name>.xml`, `miscload.big`) on its
/// district's Sim tiles: Skate 3's `ParseXmlStreamTile` takes up to 64 neighbours, where Skate 2's
/// asserts at 16; a race's area holds more than Skate 2 allows.
#[test]
fn skate3_online_stream_xml_reads_on_its_district() {
    let Some(root) = data() else { return };
    let mut mounts = MountSet::new();
    mounts.push(MountedBig::open_source(ArchiveSource::File(root.join("data").join("big").join("miscload.big"))).expect("miscload mounts"));
    let mut spaces = Vec::new();
    for district in DISTRICTS {
        let path = root.join("data").join("content").join(format!("world{district}.big"));
        mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("district archive mounts"));
        let base = format!("data\\content\\world\\stream\\{district}\\{district}_Sim");
        spaces.push(CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss")).unwrap()).unwrap());
    }
    let mut read = 0;
    let mut beyond_skate2 = 0;
    for name in ["race_dwtn_02", "race_univ_01", "race_indu_01", "ots_indu_02", "hom_univ_01_sp", "skate_univ_01"] {
        let Ok(bytes) = mounts.read_path(&format!("data\\stream\\{name}.xml")) else { continue };
        // Its district: the space whose tiles hold its first tile's centre.
        let Some(tile) = spaces.iter().find_map(|space| skate2_formats::stream_xml::StreamXml::parse_skate3(&bytes, "Sim", space).ok()?.first_tile().map(<[usize]>::len)) else {
            continue;
        };
        read += 1;
        assert!(tile <= 65, "{name}: {tile} collections");
        beyond_skate2 += usize::from(tile > 16);
        eprintln!("{name}: {tile} collections");
    }
    assert!(read > 0, "no online game's stream XML read");
    eprintln!("{read} read, {beyond_skate2} past Skate 2's capacity");
}

/// Each district's Pres and Sim stream XML (`data\\stream\\<world>_<type>.xml`, `miscload.big`)
/// over its stream, as Skate 3 selects (`TileRecords`): a focus at a tile's centre takes that
/// tile first, and the Pres tiles take more than Skate 2's 16 collections, reaching past its
/// 195 m box.
#[test]
fn district_stream_xml_records_reach_past_skate2s_box() {
    let Some(root) = data() else { return };
    let mut mounts = MountSet::new();
    mounts.push(MountedBig::open_source(ArchiveSource::File(root.join("data").join("big").join("miscload.big"))).expect("miscload mounts"));
    for (district, world) in DISTRICTS.into_iter().zip(["dist_university", "dist_downtown", "dist_industrial"]) {
        let path = root.join("data").join("content").join(format!("world{district}.big"));
        mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("district archive mounts"));
        for kind in ["Pres", "Sim"] {
            let space = CollectionSpace::parse(&mounts.read_path(&format!("data\\content\\world\\stream\\{district}\\{district}_{kind}.xss")).unwrap()).unwrap();
            let mut records = TileRecords::new(&space, kind);
            records.apply_stream_xml(&mounts.read_path(&format!("data\\stream\\{world}_{kind}.xml")).unwrap()).unwrap();
            let (mut most, mut reach) = (0, 0.0_f32);
            for entry in space.collections.iter().filter(|entry| entry.lod == 0) {
                let centre = [(entry.hull_min[0] + entry.hull_max[0]) * 0.5, 0.0, (entry.hull_min[2] + entry.hull_max[2]) * 0.5];
                let selected = records.focus_collections(&space, centre);
                assert_eq!(selected.first().map(|first| first.id), Some(entry.id), "{district} {kind}: a tile's selection starts with the tile");
                most = most.max(selected.len());
                reach = selected.iter().map(|collection| collection.distance_xz).fold(reach, f32::max);
            }
            eprintln!("{district} {kind}: at most {most} collections, reaching {reach:.0} m");
            if kind == "Pres" {
                assert!(most > 16 && reach > 195.0, "{district}: the stream XML reaches past Skate 2's selection");
            }
        }
    }
}
