//! The mission hulls' presentation collections the simulation's hull loaders registered
//! (`cHullLoader::InitiateCollectionRegistration` 0x82eecd50 → `NotifyCollectionPresReg`):
//! each hull's `cPres_Global` collection, read from the hull's own stream
//! (`data\content\missions\<hull>\<hull>_Pres`) and drawn with the world, as
//! `WorldPresentation::RegArena` registers any presentation collection's render instances.
//! `NotifyCollectionReg` registers the hull's simulation collections with the presentation
//! too: of their arenas the presentation keeps the visual indicator data.
use skate2_core::{
    challenge::assets::CONTENT_STREAM_DIRECTORY,
    game_stream::{StreamKind, StreamPaths},
    mounted_character_scene::{own_presentation_arena, own_visual_indicators},
    presentation::{HullPresentation, PresentationArena, PresentationCollection, PresentationScene},
};
use skate2_formats::{
    mount::MountSet,
    rw4::Arena as Rw4Arena,
    stream::{CollectionMap, CollectionPayload, decode_arena},
};
use std::{error::Error, sync::Arc};

/// The scene's hull collections become the registered ones, in the loaders' order: a collection
/// still registered keeps its decode, a new one is read. Returns whether any changed.
pub(crate) fn sync(scene: &mut PresentationScene, mounts: &MountSet, hulls: &[HullPresentation]) -> Result<bool, Box<dyn Error>> {
    let mut held = std::mem::take(&mut scene.hulls);
    let mut registered = Vec::new();
    let mut changed = false;
    for hull in hulls {
        let paths = StreamPaths::derive(CONTENT_STREAM_DIRECTORY, &hull.file_name, StreamKind::Presentation)
            .map_err(|e| format!("hull {} presentation stream path: {e:?}", hull.file_name))?;
        let mut map = None;
        for &id in &hull.collections {
            if let Some(index) = held.iter().position(|collection| collection.id == id) {
                registered.push(held.swap_remove(index));
                continue;
            }
            if map.is_none() {
                map = Some(CollectionMap::parse(&mounts.read_path(&format!("{}.xsm", paths.manifest_stem))?)?);
            }
            let map = map.as_ref().ok_or("hull presentation manifest absent")?;
            let collection = load_collection(mounts, &paths, map, id)?;
            if std::env::var_os("SKATE2_TRACE_HULLS").is_some() {
                let instances = collection.arenas.iter().flat_map(|arena| &arena.instances).filter(|instance| instance.render_model.is_some());
                let (count, min, max) = instances.fold((0, [f32::MAX; 3], [f32::MIN; 3]), |(n, lo, hi), instance| {
                    (n + 1, std::array::from_fn(|i| lo[i].min(instance.bounding_box_min[i])), std::array::from_fn(|i| hi[i].max(instance.bounding_box_max[i])))
                });
                eprintln!("hull presentation: {} {id:016x} {count} render instances, bounds {min:?}..{max:?}", hull.file_name);
            }
            registered.push(collection);
            changed = true;
        }
        if hull.sim_collections.iter().all(|id| held.iter().any(|collection| collection.id == *id)) {
            for &id in &hull.sim_collections {
                if let Some(index) = held.iter().position(|collection| collection.id == id) {
                    registered.push(held.swap_remove(index));
                }
            }
            continue;
        }
        let paths = StreamPaths::derive(CONTENT_STREAM_DIRECTORY, &hull.file_name, StreamKind::Simulation)
            .map_err(|e| format!("hull {} simulation stream path: {e:?}", hull.file_name))?;
        let map = CollectionMap::parse(&mounts.read_path(&format!("{}.xsm", paths.manifest_stem))?)?;
        for &id in &hull.sim_collections {
            if let Some(index) = held.iter().position(|collection| collection.id == id) {
                registered.push(held.swap_remove(index));
                continue;
            }
            registered.push(load_visual_indicators(mounts, &paths, &map, id)?);
            changed = true;
        }
    }
    changed |= !held.is_empty();
    scene.hulls = registered;
    Ok(changed)
}

/// One hull simulation collection as the presentation registers it: each asset's arena holds
/// only its visual indicator data (`RegArena` asset type 0x11).
fn load_visual_indicators(mounts: &MountSet, paths: &StreamPaths, map: &CollectionMap, id: u64) -> Result<PresentationCollection, Box<dyn Error>> {
    let entry = map.collections.iter().find(|entry| entry.id == id).ok_or_else(|| format!("missing hull simulation collection {id:016x}"))?;
    let xsf = mounts.read_path(&format!("{}{}.xsf", paths.collection_directory, entry.filename))?;
    let payload = xsf.get(entry.payload_offset as usize..).ok_or("hull simulation collection payload offset")?;
    let file = CollectionPayload::parse(payload, entry.asset_count)?;
    let mut arenas = Vec::with_capacity(file.assets.len());
    for block in &file.assets {
        let decoded = decode_arena(&payload[block.data_range.clone()])?;
        let arena = Rw4Arena::parse(&decoded.resources)?;
        arenas.push(PresentationArena {
            id: arena.id,
            textures: Vec::new(),
            materials: Vec::new(),
            meshes: Vec::new(),
            models: Vec::new(),
            instances: Vec::new(),
            anti_frustums: Vec::new(),
            emitter_waypoints: Vec::new(),
            visual_indicators: own_visual_indicators(&arena, &decoded.resources),
        });
    }
    Ok(PresentationCollection {
        light_probes: Vec::new(),
        navigation: Vec::new(),
        id,
        name: entry.filename.clone(),
        position: [0.0; 3],
        distance_xz: 0.0,
        arenas,
    })
}

/// One hull collection: each asset's arena decoded as the city's presentation collections are,
/// with the irradiance probes its table of contents registers.
fn load_collection(mounts: &MountSet, paths: &StreamPaths, map: &CollectionMap, id: u64) -> Result<PresentationCollection, Box<dyn Error>> {
    let entry = map.collections.iter().find(|entry| entry.id == id).ok_or_else(|| format!("missing hull presentation collection {id:016x}"))?;
    let xsf = mounts.read_path(&format!("{}{}.xsf", paths.collection_directory, entry.filename))?;
    let payload = xsf.get(entry.payload_offset as usize..).ok_or("hull presentation collection payload offset")?;
    let file = CollectionPayload::parse(payload, entry.asset_count)?;
    let mut light_probes = Vec::new();
    let mut arenas = Vec::with_capacity(file.assets.len());
    for block in &file.assets {
        let decoded = decode_arena(&payload[block.data_range.clone()])?;
        let arena = Rw4Arena::parse(&decoded.resources)?;
        for dictionary in &arena.dictionary {
            if dictionary.type_id != skate2_formats::pegasus::TABLE_OF_CONTENTS_TYPE_ID {
                continue;
            }
            let toc = skate2_formats::pegasus::TableOfContents::parse(
                arena.object_bytes(&decoded.resources, dictionary).ok_or("hull TOC object range")?,
            )?;
            for item in toc.entries {
                if item.object_type != skate2_formats::irradiance::TYPE_ID {
                    continue;
                }
                let probe = arena
                    .local_entry(item.object)
                    .filter(|e| e.type_id == skate2_formats::irradiance::TYPE_ID)
                    .ok_or("hull irradiance TOC reference")?;
                let bytes = arena.object_bytes(&decoded.resources, probe).ok_or("hull irradiance object range")?;
                light_probes.push((item.guid, Arc::from(skate2_formats::irradiance::parse(bytes)?)));
            }
        }
        arenas.push(own_presentation_arena(&arena, &decoded.resources)?);
    }
    Ok(PresentationCollection {
        light_probes,
        navigation: Vec::new(),
        id,
        name: entry.filename.clone(),
        position: [0.0; 3],
        distance_xz: 0.0,
        arenas,
    })
}
