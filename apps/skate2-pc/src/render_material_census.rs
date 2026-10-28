//! Explicit offline census: production decoders and Vault bindings, one
//! collection at a time. No window, shader substitution or simulation changes.
//! `SKATE2_CENSUS_WORLD` names a downloadable content world of the packages in
//! `SKATE2_DLC_DIR`, whose Pres stream the census reads in BAM's place.
use crate::gpu_renderer::{EnvironmentEffect, RenderResult};
use skate2_core::presentation::{PresentationMaterialEffect, PresentationMaterialValue};
use skate2_formats::stream::{CollectionMap, FocusCollection};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Entry {
    materials: usize,
    channels: BTreeMap<String, usize>,
    effect: Option<PresentationMaterialEffect>,
    sample_collection: String,
}

/// Per stream asset type (the `.xst` entry of each payload block): its arenas, the world
/// instances `RenderQueue::build` submits from them, how many draw a `dynamicobject` effect,
/// and how many draw a raster nothing in the stream holds.
fn report_asset_types(
    mounts: &skate2_formats::mount::MountSet,
    map: &CollectionMap,
    streams: &crate::render_streams::RenderStreams,
    held: &BTreeSet<u64>,
) -> RenderResult<()> {
    #[derive(Default, Debug)]
    struct Row {
        blocks: usize,
        instances: usize,
        dynamic_object: usize,
        short: usize,
        sample: String,
    }
    let mut rows = BTreeMap::<u32, Row>::new();
    for row in &map.collections {
        let (collection, stream) = streams.entry(row.id).ok_or("census collection absent")?;
        let xsf = mounts.read_path(&stream.collection_path(collection))?;
        let payload = xsf.get(collection.payload_offset as usize..).ok_or("census payload offset")?;
        let file = skate2_formats::stream::CollectionPayload::parse(payload, collection.asset_count)?;
        let toc = skate2_formats::stream::AssetToc::parse(&mounts.read_path(&stream.asset_toc_path())?)?;
        for block in &file.assets {
            let asset_type = toc
                .assets
                .iter()
                .find(|entry| entry.id == block.id)
                .map_or(u32::MAX, |entry| entry.asset_type);
            let decoded = skate2_formats::stream::decode_arena(&payload[block.data_range.clone()])?;
            let arena = skate2_formats::rw4::Arena::parse(&decoded.resources)?;
            let owned = skate2_core::mounted_character_scene::own_presentation_arena(&arena, &decoded.resources)?;
            let entry = rows.entry(asset_type).or_default();
            entry.blocks += 1;
            for instance in &owned.instances {
                let Some(model) = instance.render_model else { continue };
                entry.instances += 1;
                let materials: Vec<_> = owned.models[model]
                    .meshes
                    .iter()
                    .filter_map(|mesh| mesh.material.or(owned.meshes[mesh.mesh_index].material))
                    .map(|index| &owned.materials[index])
                    .collect();
                if materials.iter().any(|material| {
                    material.effect.as_ref().is_some_and(|effect| effect.serialized_name.starts_with("dynamicobject"))
                }) {
                    entry.dynamic_object += 1;
                }
                if materials.iter().any(|material| {
                    material.channels.iter().skip(2).any(|channel| match channel.value {
                        PresentationMaterialValue::Raster { guid, .. } => guid & !1 != 0 && !held.contains(&(guid & !1)),
                        PresentationMaterialValue::ScalarConstant(_) => false,
                    })
                }) {
                    entry.short += 1;
                    if entry.sample.is_empty() {
                        entry.sample = format!("{} ({})", instance.name, row.filename);
                    }
                }
            }
        }
    }
    for (asset_type, row) in &rows {
        std::println!(
            "asset type {asset_type:08x}: {} blocks, {} world instances, {} dynamicobject, {} short; first short {}",
            row.blocks,
            row.instances,
            row.dynamic_object,
            row.short,
            row.sample
        );
    }
    Ok(())
}

fn census_focus(index: usize, id: u64) -> FocusCollection {
    FocusCollection {
        index,
        id,
        position: [0.; 3],
        distance_xz: 0.,
        stream_order: 0,
    }
}

/// Which stream holds each raster the census stream references but does not hold: every
/// downloadable content world's and mission's Pres stream, and BAM's Pres and Tex streams. A
/// GUID with its top bits cleared is looked for too.
fn report_holders(
    mounts: &skate2_formats::mount::MountSet,
    held: &BTreeSet<u64>,
    referenced: &BTreeMap<String, BTreeSet<u64>>,
) -> RenderResult<()> {
    let missing: BTreeSet<u64> = referenced
        .values()
        .flatten()
        .copied()
        .filter(|guid| !held.contains(guid))
        .collect();
    let mut sources = Vec::<(String, BTreeSet<u64>)>::new();
    let mut streams_to_read: Vec<(String, String)> = ["DLC_Community_Center", "DLC_School", "DLC_Parkade", "DLC_Rob_Fantasy_Plaza", "DLC_MaloofMoneyCup"]
        .map(|world| (skate2_core::game::WORLD_STREAM_PATH.to_owned(), world.to_owned()))
        .to_vec();
    let missions = BTreeSet::<String>::from_iter(mounts.dlc_archives().iter().flat_map(|archive| {
        archive
            .archive()
            .entries
            .iter()
            .filter_map(|entry| match &entry.key {
                skate2_formats::big::BigEntryKey::Name(name) => {
                    let rest = name.strip_prefix("data\\content\\missions\\")?;
                    let (folder, file) = rest.split_once('\\')?;
                    (file == format!("{folder}_Pres.xsm")).then(|| folder.to_owned())
                }
                skate2_formats::big::BigEntryKey::Hash(_) => None,
            })
            .collect::<Vec<_>>()
    }));
    streams_to_read.extend(missions.into_iter().map(|folder| ("data\\content\\missions".to_owned(), folder)));
    for (base, name) in &streams_to_read {
        let paths = skate2_core::game_stream::StreamPaths::derive(
            base,
            name,
            skate2_core::game_stream::StreamKind::Presentation,
        )
        .map_err(|e| format!("{e:?}"))?;
        let Ok(bytes) = mounts.read_path(&format!("{}.xsm", paths.manifest_stem)) else { continue };
        let map = CollectionMap::parse(&bytes)?;
        let streams = crate::render_streams::RenderStreams::single(
            crate::render_streams::RenderStreamFile::from_paths(std::sync::Arc::new(map.clone()), &paths),
        );
        let mut ids = BTreeSet::new();
        for (index, row) in map.collections.iter().enumerate() {
            match crate::load_presentation_collection(mounts, &streams, &census_focus(index, row.id)) {
                Ok(collection) => ids.extend(collection.arenas.iter().flat_map(|a| &a.textures).map(|t| t.guid & !1)),
                Err(error) => std::eprintln!("holders: {name} {}: {error}", row.filename),
            }
        }
        std::eprintln!("holders: {name} read");
        sources.push((name.clone(), ids));
    }
    let bam = CollectionMap::parse(&mounts.read_path(r"data\content\world\stream\BAM\BAM_Pres.xsm")?)?;
    let streams = crate::render_streams::RenderStreams::single(crate::render_streams::RenderStreamFile::bam(bam.clone(), "Pres"));
    let mut ids = BTreeSet::new();
    for (index, row) in bam.collections.iter().enumerate() {
        let collection = crate::load_presentation_collection(mounts, &streams, &census_focus(index, row.id))?;
        ids.extend(collection.arenas.iter().flat_map(|a| &a.textures).map(|t| t.guid & !1));
    }
    std::eprintln!("holders: BAM_Pres read");
    sources.push(("BAM_Pres".to_owned(), ids));
    let tex = CollectionMap::parse(&mounts.read_path(r"data\content\world\stream\BAM\BAM_Tex.xsm")?)?;
    let streams = crate::render_streams::RenderStreams::single(crate::render_streams::RenderStreamFile::bam(tex.clone(), "Tex"));
    let mut ids = BTreeSet::new();
    for (index, row) in tex.collections.iter().enumerate() {
        let collection = crate::load_texture_collection(mounts, &streams, &census_focus(index, row.id))?;
        ids.extend(collection.arenas.iter().flat_map(|a| &a.textures).map(|t| t.guid & !1));
    }
    std::eprintln!("holders: BAM_Tex read");
    sources.push(("BAM_Tex".to_owned(), ids));
    for (name, ids) in &sources {
        let prefixed = ids.iter().filter(|&&id| id >> 48 == 0x4000).count();
        std::println!("holders: {name} holds {} textures, {prefixed} with the 4000 prefix", ids.len());
    }
    for guid in &missing {
        let unprefixed = guid & 0x0000_ffff_ffff_ffff;
        let by = |key: u64| sources.iter().filter(|(_, ids)| ids.contains(&key)).map(|(n, _)| n.as_str()).collect::<Vec<_>>();
        std::println!("missing {guid:016x}: held by {:?}; {unprefixed:016x} held by {:?}", by(*guid), by(unprefixed));
    }
    Ok(())
}

pub(crate) fn run() -> RenderResult<()> {
    let root = std::env::var_os("SKATE2_GAME_ROOT").ok_or("set SKATE2_GAME_ROOT")?;
    let mounts = crate::mount_reference_archives(std::path::Path::new(&root))?;
    if let Some(dir) = std::env::var_os("SKATE2_DLC_DIR") {
        for path in std::fs::read_dir(dir)?.flatten().map(|entry| entry.path()) {
            let Ok(package) = skate2_formats::stfs::StfsPackage::open(&path) else { continue };
            for entry in package.root_files().filter(|entry| entry.name.to_ascii_lowercase().ends_with(".big")) {
                mounts.push_dlc(skate2_formats::mount::MountedBig::open_source(
                    skate2_formats::mount::ArchiveSource::Package(package.file(entry)?),
                )?);
            }
        }
    }
    let world = std::env::var("SKATE2_CENSUS_WORLD").ok();
    let (map, streams) = match world.as_deref() {
        Some(world) => {
            let paths = skate2_core::game_stream::StreamPaths::derive(
                skate2_core::game::WORLD_STREAM_PATH,
                world,
                skate2_core::game_stream::StreamKind::Presentation,
            )
            .map_err(|e| format!("{e:?}"))?;
            let map = CollectionMap::parse(&mounts.read_path(&format!("{}.xsm", paths.manifest_stem))?)?;
            let file = crate::render_streams::RenderStreamFile::from_paths(std::sync::Arc::new(map.clone()), &paths);
            (map, crate::render_streams::RenderStreams::single(file))
        }
        None => {
            let map =
                CollectionMap::parse(&mounts.read_path(r"data\content\world\stream\BAM\BAM_Pres.xsm")?)?;
            let file = crate::render_streams::RenderStreamFile::bam(map.clone(), "Pres");
            (map, crate::render_streams::RenderStreams::single(file))
        }
    };
    let mut entries = BTreeMap::<String, Entry>::new();
    let mut lightmap_mips = BTreeMap::<u32, usize>::new();
    let mut lightmap_guids = BTreeSet::new();
    // Every texture the stream's collections hold and each channel's referenced rasters, by
    // GUID without its low-copy bit, as the shader texture table pairs them (AddTexture).
    let mut held = BTreeSet::new();
    let mut referenced = BTreeMap::<String, BTreeSet<u64>>::new();
    // Every material's own Vault key: one effect name can carry several.
    let mut material_keys = BTreeMap::<(String, Option<(u64, u64)>), usize>::new();
    // Each held texture's format and size, by GUID without its low-copy bit, and the formats'
    // counts; SKATE2_CENSUS_TEXTURES names materials (label substring) whose channels to print.
    let mut texture_formats = BTreeMap::<u64, (u32, u32, u32, String)>::new();
    let texture_label = std::env::var("SKATE2_CENSUS_TEXTURES").ok();
    let mut labelled_channels = BTreeMap::<String, Vec<(String, u64)>>::new();
    // Each material's collection, effect, first channel's stream name and sampled rasters.
    let mut material_rasters = Vec::<(String, String, String, Vec<(String, u64)>)>::new();
    // SKATE2_CENSUS_LABELS (effect names, comma separated): those effects' materials by name,
    // with their Vault key and scalar channels.
    let label_effects: Vec<String> = std::env::var("SKATE2_CENSUS_LABELS")
        .map(|list| list.split(',').map(str::to_owned).collect())
        .unwrap_or_default();
    let mut labelled = BTreeMap::<(String, String), (usize, Option<(u64, u64)>, Vec<String>, String)>::new();
    // Environment materials leaving out a normal or specular channel their effect samples.
    let mut absent = BTreeMap::<(String, String, Vec<&str>), usize>::new();
    for (index, row) in map.collections.iter().enumerate() {
        let focus = FocusCollection {
            index,
            id: row.id,
            position: [0.; 3],
            distance_xz: 0.,
            stream_order: 0,
        };
        let collection = crate::load_presentation_collection(&mounts, &streams, &focus)?;
        for arena in &collection.arenas {
            held.extend(arena.textures.iter().map(|texture| texture.guid & !1));
            for texture in &arena.textures {
                texture_formats
                    .entry(texture.guid & !1)
                    .or_insert((texture.pixel_format, texture.width, texture.height, texture.name.clone()));
            }
            for material in &arena.materials {
                for channel in material.channels.iter().skip(2) {
                    if let PresentationMaterialValue::Raster { guid, .. } = channel.value {
                        referenced.entry(channel.shader_input.clone()).or_default().insert(guid & !1);
                    }
                }
                let rasters = material
                    .channels
                    .iter()
                    .skip(2)
                    .filter_map(|channel| match channel.value {
                        PresentationMaterialValue::Raster { guid, .. } => Some((channel.shader_input.clone(), guid & !1)),
                        PresentationMaterialValue::ScalarConstant(_) => None,
                    })
                    .collect();
                let label = material.channels.first().map_or_else(String::new, |channel| match &channel.value {
                    PresentationMaterialValue::Raster { stream_name, .. } => stream_name.clone(),
                    PresentationMaterialValue::ScalarConstant(_) => String::new(),
                });
                if texture_label.as_deref().is_some_and(|wanted| label.contains(wanted)) {
                    labelled_channels.entry(label.clone()).or_insert_with(|| {
                        material
                            .channels
                            .iter()
                            .skip(2)
                            .filter_map(|channel| match channel.value {
                                PresentationMaterialValue::Raster { guid, .. } => Some((channel.shader_input.clone(), guid & !1)),
                                PresentationMaterialValue::ScalarConstant(_) => None,
                            })
                            .collect()
                    });
                }
                let effect_name = material.effect.as_ref().map_or_else(|| "<no effect>".to_owned(), |e| e.serialized_name.clone());
                if effect_name.starts_with("environment.") {
                    let missing: Vec<&str> = ["normal", "specular"]
                        .into_iter()
                        .filter(|name| !material.channels.iter().any(|channel| channel.shader_input == *name))
                        .collect();
                    if !missing.is_empty() {
                        *absent.entry((effect_name.clone(), label.clone(), missing)).or_default() += 1;
                    }
                }
                if label_effects.contains(&effect_name) {
                    let scalars = material
                        .channels
                        .iter()
                        .skip(2)
                        .filter_map(|channel| match channel.value {
                            PresentationMaterialValue::ScalarConstant(value) => Some(format!("{}={value:?}", channel.shader_input)),
                            PresentationMaterialValue::Raster { .. } => None,
                        })
                        .collect();
                    let entry = labelled.entry((effect_name.clone(), label.clone())).or_insert_with(|| {
                        (0, material.effect.as_ref().and_then(|e| e.attribute_class_id.zip(e.material_variant_id)), scalars, row.filename.clone())
                    });
                    entry.0 += 1;
                }
                material_rasters.push((row.filename.clone(), effect_name, label, rasters));
                let effect = material.effect.as_ref();
                *material_keys
                    .entry((
                        effect.map_or_else(|| "<no effect>".to_owned(), |e| e.serialized_name.clone()),
                        effect.and_then(|e| e.attribute_class_id.zip(e.material_variant_id)),
                    ))
                    .or_default() += 1;
                let name = material
                    .effect
                    .as_ref()
                    .map_or("<no effect>", |e| &e.serialized_name);
                let entry = entries.entry(name.to_owned()).or_default();
                entry.materials += 1;
                entry.effect = material.effect.clone();
                if entry.sample_collection.is_empty() {
                    entry.sample_collection = row.filename.clone();
                }
                for channel in material.channels.iter().skip(2) {
                    *entry
                        .channels
                        .entry(channel.shader_input.clone())
                        .or_default() += 1;
                    if channel.shader_input == "lightmap" {
                        if let PresentationMaterialValue::Raster { guid, .. } = channel.value {
                            lightmap_guids.insert(guid);
                            if let Some(texture) = arena.textures.iter().find(|t| t.guid == guid) {
                                *lightmap_mips.entry(texture.mip_levels).or_default() += 1;
                            }
                        }
                    }
                }
            }
        }
        std::eprintln!(
            "material census {}/{}: {}",
            index + 1,
            map.collections.len(),
            row.filename
        );
    }
    // As bind_runtime_material_attributes: a key neither the game's vault nor the content's
    // holds takes the native default.
    let known = crate::vault_collection_keys(&mounts)?;
    let db = crate::vault_database(&mounts)?;
    let source = |key: (u64, u64)| {
        if known.contains(&key) {
            "game"
        } else if db.has_collection(key.0, key.1) {
            "content"
        } else {
            "fallback"
        }
    };
    let keys = entries
        .values()
        .filter_map(|e| e.effect.as_ref())
        .filter_map(|e| e.attribute_class_id.zip(e.material_variant_id))
        .filter(|&key| source(key) != "fallback")
        .collect::<BTreeSet<_>>();
    let bindings = crate::load_material_attribute_bindings(&mounts, &keys)?;
    // The environment a downloadable content world draws with: the content's collections added.
    let lighting = crate::render_environment::EnvironmentLighting::load(&mounts)?.with_vault_collections(&mounts, &db)?;
    std::println!("material\tmanifest\tported\tsource\tm_params\tcount\tsample_collection\tchannels");
    for (name, entry) in entries {
        let key = entry.effect.as_ref().and_then(|e| e.attribute_class_id.zip(e.material_variant_id));
        let manifest = key
            .and_then(|key| bindings.get(&key))
            .map_or("<native default fallback>", |b| b.manifest_name.as_str());
        std::println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}",
            name,
            manifest,
            EnvironmentEffect::from_manifest(manifest).is_some(),
            key.map_or("legacy", source),
            key.is_some_and(|(class, variant)| lighting.material_params(class, variant).is_some()),
            entry.materials,
            entry.sample_collection,
            entry.channels
        );
    }
    for ((name, key), count) in &material_keys {
        let source = key.map_or("legacy", source);
        if source != "game" {
            std::println!(
                "key {name} {}: {count} materials, {source}",
                key.map_or_else(|| "-".to_owned(), |(class, variant)| format!("{class:016x}/{variant:016x}"))
            );
        }
    }
    for (channel, guids) in &referenced {
        let missing: Vec<_> = guids.iter().filter(|guid| !held.contains(guid)).collect();
        std::println!(
            "channel {channel}: {} rasters referenced, {} held by no collection of the stream{:x?}",
            guids.len(),
            missing.len(),
            missing.iter().take(8).collect::<Vec<_>>()
        );
    }
    let mut format_counts = BTreeMap::<u32, usize>::new();
    for (format, ..) in texture_formats.values() {
        *format_counts.entry(*format).or_default() += 1;
    }
    std::println!("texture formats {format_counts:x?}");
    for (label, channels) in &labelled_channels {
        std::println!("textures of \"{label}\"");
        for (channel, guid) in channels {
            match texture_formats.get(guid) {
                Some((format, width, height, name)) => {
                    std::println!("   {channel}: {guid:016x} format {format:#x} {width}x{height} {name}");
                }
                None => std::println!("   {channel}: {guid:016x} held by no collection"),
            }
        }
    }
    for ((effect, label, missing), count) in &absent {
        std::println!("absent {effect} \"{label}\": {count} materials without {missing:?}");
    }
    for ((effect, label), (count, key, scalars, collection)) in &labelled {
        let params = key.and_then(|(class, variant)| lighting.material_params_pair(class, variant));
        std::println!(
            "label {effect} \"{label}\": {count} materials in {collection}; key {}; m_params {params:?}; {scalars:?}",
            key.map_or_else(|| "-".to_owned(), |(class, variant)| format!("{class:016x}/{variant:016x}"))
        );
    }
    // The materials whose sampled rasters nothing holds: AddMaterial 0x8282c838 binds the boot
    // missing texture to those channels.
    let mut short = BTreeMap::<(String, String, Vec<String>), (usize, BTreeSet<String>)>::new();
    for (collection, effect, label, rasters) in &material_rasters {
        let missing: Vec<String> = rasters
            .iter()
            .filter(|(_, guid)| *guid != 0 && !held.contains(guid))
            .map(|(channel, guid)| format!("{channel}={guid:016x}"))
            .collect();
        if !missing.is_empty() {
            let entry = short.entry((effect.clone(), label.clone(), missing)).or_default();
            entry.0 += 1;
            entry.1.insert(collection.clone());
        }
    }
    for ((effect, label, missing), (count, collections)) in &short {
        std::println!(
            "short {effect} \"{label}\": {count} materials, missing {missing:?}, in {} collections (first {})",
            collections.len(),
            collections.first().map_or("", String::as_str)
        );
    }
    if std::env::var_os("SKATE2_CENSUS_HOLDERS").is_some() {
        report_holders(&mounts, &held, &referenced)?;
    }
    if std::env::var_os("SKATE2_CENSUS_TYPES").is_some() {
        report_asset_types(&mounts, &map, &streams, &held)?;
    }
    if world.is_some() {
        return Ok(());
    }
    let sampled =
        crate::load_runtime_texture_scene(&mounts, skate2_core::player::selected_spawn_location())?;
    let mut sampled_lightmaps = BTreeMap::<(u32, u32, u32), usize>::new();
    for texture in sampled
        .collections
        .iter()
        .flat_map(|c| &c.arenas)
        .flat_map(|a| &a.textures)
    {
        if lightmap_guids.contains(&texture.guid) {
            *sampled_lightmaps
                .entry((texture.width, texture.height, texture.mip_levels))
                .or_default() += 1;
        }
    }
    std::println!("arena-local lightmap mip-level histogram: {lightmap_mips:?}");
    std::println!("spawn-region lightmap (width,height,mips) histogram: {sampled_lightmaps:?}");
    Ok(())
}
