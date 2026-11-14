//! Reusable mounted character/arena producers extracted from the existing PC host.
//! This preserves host decoding policy; it does not complete native asynchronous
//! asset registration or create presentation entity/model membership.
use crate::{
    character::{
        CharacterComponentAsset, CharacterLodAsset, CharacterModelAsset, CharacterScene,
        CharacterTextureAsset, CharacterRopaBinding, MISSING_TEXTURE,
    },
    presentation::{
        PresentationArena, PresentationInstance, PresentationMaterial, PresentationMaterialChannel,
        PresentationMaterialEffect, PresentationMaterialValue, PresentationMesh, PresentationModel,
        PresentationModelMesh,
    },
    texture::{NativeTexture, TextureArena},
};
use skate2_formats::{
    material::{
        MaterialChannelValue, R_MATERIAL_DATA_TYPE_ID, RenderMaterialData, TEXTURE_TYPE_ID, Texture,
    },
    mount::MountSet,
    pegasus::{
        INSTANCE_DATA_TYPE_ID, InstanceData, ObjectId, TABLE_OF_CONTENTS_TYPE_ID, TableOfContents,
    },
    recipe::{CharacterAssetKind, CharacterAssetRequest, CharacterRecipe},
    render::{
        GPU_BUFFER_RESOURCE_TYPE_ID, OptimizedRenderMeshData, R_MODEL_DATA_TYPE_ID,
        R_OPTI_MESH_DATA_TYPE_ID, RenderModelData,
    },
    rw4::Arena as Rw4Arena,
};
use std::collections::{BTreeMap, BTreeSet};
type ObjectIndexMap = BTreeMap<u32, usize>;
type MaterialSubreferenceMap = BTreeMap<u32, usize>;
/// Existing mounted host asset decoding, preserving serialized references.
///
/// # Errors
/// Returns the existing mount, parse, or unresolved asset-reference error.
pub fn own_character_scene(
    mounts: &MountSet,
    recipe: &CharacterRecipe,
) -> Result<CharacterScene, Box<dyn std::error::Error>> {
    own_character_scene_category(mounts, recipe, "marquee")
}

/// Shared character decoder for the original category asset namespaces.
/// Character::GetCategoryString82cfd288: marquee, livingworld, vehicle, ... .
pub fn own_character_scene_category(
    mounts: &MountSet,
    recipe: &CharacterRecipe,
    category: &str,
) -> Result<CharacterScene, Box<dyn std::error::Error>> {
    own_character_scene_with_geometry(mounts, recipe, category, &mut |_, _, _| Ok(()))
}

/// Lets the same load retain collision data before its decoded resource buffers
/// are released. The NPC path must not reread every geometry archive for SIM.
pub(crate) fn own_character_scene_with_geometry(
    mounts: &MountSet,
    recipe: &CharacterRecipe,
    category: &str,
    retain_geometry: &mut impl FnMut(
        &CharacterAssetRequest,
        &Rw4Arena,
        &[Vec<u8>; 5],
    ) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<CharacterScene, Box<dyn std::error::Error>> {
    let requests = recipe.asset_requests(category);
    let morph =
        crate::character_morph::CharacterMorph::load(mounts, recipe.customizations.as_ref())?;
    let mut geometry_requests = BTreeMap::<u64, &CharacterAssetRequest>::new();
    for request in requests
        .iter()
        .filter(|request| request.kind == CharacterAssetKind::Geometry)
    {
        if geometry_requests
            .insert(request.source_asset_id, request)
            .is_some()
        {
            return Err(format!(
                "ambiguous character geometry arena ID {:016x}",
                request.source_asset_id
            )
            .into());
        }
    }

    let mut components = Vec::with_capacity(recipe.components.len());
    for component in &recipe.components {
        let mut models = Vec::with_capacity(component.models.len());
        for model in &component.models {
            let mut lods = Vec::with_capacity(model.lods.len());
            for lod in &model.lods {
                let request = geometry_requests
                    .get(&lod.arena_asset_id)
                    .ok_or("recipe LOD has no geometry request")?;
                let bytes = mounts.read_path(&request.entry_path)?;
                let mut resources = Rw4Arena::split_file_resources(&bytes)?;
                let arena = Rw4Arena::parse(&resources)?;
                morph.apply(&arena, &mut resources)?;
                retain_geometry(request, &arena, &resources)?;
                let owned = own_presentation_arena(&arena, &resources)?;
                let mut ropa_effects = Vec::new();
                let mut ropa_bindings=Vec::new();
                for instance in &owned.instances {
                    let Some(model_index)=instance.render_model else {continue;};
                    let Some(model) = owned.models.get(model_index)
                    else {
                        continue;
                    };
                    let Some(mesh) = model
                        .meshes
                        .first()
                        .and_then(|m| owned.meshes.get(m.mesh_index))
                    else {
                        continue;
                    };
                    let entry = arena
                        .dictionary
                        .get(mesh.dictionary_index as usize)
                        .ok_or("cloth mesh dictionary entry")?;
                    let data = OptimizedRenderMeshData::parse(
                        arena
                            .object_bytes(&resources, entry)
                            .ok_or("cloth mesh bytes")?,
                    )?;
                    // InitCloth827d7b30 follows optimized mesh+0x24 to material,
                    // then tests material.flags (+4)&1. The decompiler's RMesh
                    // type incorrectly labels this pointer as mesh helper.
                    if mesh.material.and_then(|i| owned.materials.get(i))
                        .is_some_and(|material| material.flags & 1 != 0) {
                        let path = format!("data\\ropa2\\0x{:x}.r2b", instance.guid);
                        // PC port: a linked Skate 3's clothing keeps its cloth in its own
                        // archive (`crate::skate3::cloth_effect`).
                        let read = match mounts.read_path(&path) {
                            Err(skate2_formats::mount::MountError::EntryNotFound(missing)) => crate::skate3::cloth_effect(&path)
                                .ok_or(skate2_formats::mount::MountError::EntryNotFound(missing)),
                            read => read,
                        };
                        match read {
                            Ok(bytes) => {
                                let effect=std::sync::Arc::new(skate2_formats::ropa::RopaEffect::parse(&bytes)?);
                                let vertex_particle_indices=data.cloth_particle_indices(&arena,&resources)?;
                                let driver_particle_indices=data.cloth_driver_particle_indices(&arena,&resources)?;
                                if vertex_particle_indices.len()!=mesh.geometry.positions.len(){return Err("cloth vertex mapping differs from presentation geometry".into());}
                                let mut matrix_palette_offsets=vec![0u16;vertex_particle_indices.len()];
                                let mut matrix_palette=Vec::new();
                                for (order,model_mesh) in model.meshes.iter().enumerate(){
                                    let part_mesh=owned.meshes.get(model_mesh.mesh_index).ok_or("cloth model mesh")?;
                                    let part_entry=arena.dictionary.get(part_mesh.dictionary_index as usize).ok_or("cloth remap mesh dictionary")?;
                                    let part=OptimizedRenderMeshData::parse(arena.object_bytes(&resources,part_entry).ok_or("cloth remap mesh bytes")?)?;
                                    if order!=0 {
                                        // Full decoded index-buffer order, without draw range filtering.
                                        for &vertex in &part_mesh.geometry.indices {
                                            *matrix_palette_offsets.get_mut(vertex as usize).ok_or("cloth coord vertex exceeds shared buffer")?=matrix_palette.len() as u16;
                                        }
                                    }
                                    matrix_palette.extend_from_slice(&part.remap_indices);
                                }
                                ropa_effects.push(instance.guid);
                                ropa_bindings.push(CharacterRopaBinding{model_id:instance.guid,model_index,
                                    mesh_index:model.meshes[0].mesh_index,effect,vertex_particle_indices,driver_particle_indices,matrix_palette_offsets,matrix_palette});
                            },
                            Err(skate2_formats::mount::MountError::EntryNotFound(_)) => {}
                            Err(e) => return Err(e.into()),
                        }
                    }
                }
                if owned.instances.is_empty() || owned.models.is_empty() {
                    return Err(format!(
                        "character LOD arena {:?} has no instance/model graph",
                        request.entry_path
                    )
                    .into());
                }
                lods.push(CharacterLodAsset {
                    level: lod.level,
                    asset_id: lod.asset_id,
                    arena_asset_id: lod.arena_asset_id,
                    loaded_asset_id: request.loaded_asset_id,
                    entry_path: request.entry_path.clone(),
                    material_instances: lod.material_instances.clone(),
                    ropa_effects: Some(ropa_effects),
                    ropa_bindings,
                    arena: owned,
                });
            }
            models.push(CharacterModelAsset {
                asset_id: model.asset_id,
                lods,
            });
        }
        components.push(CharacterComponentAsset {
            name: component.name.clone(),
            models,
        });
    }

    let unique_texture_requests = requests
        .iter()
        .filter(|request| request.kind == CharacterAssetKind::Texture)
        .map(|request| (request.entry_path.as_str(), request))
        .collect::<BTreeMap<_, _>>();
    let mut textures = Vec::with_capacity(unique_texture_requests.len());
    for request in unique_texture_requests.values() {
        let bytes = mounts.read_path(&request.entry_path)?;
        let resources = Rw4Arena::split_file_resources(&bytes)?;
        let arena = Rw4Arena::parse(&resources)?;
        let mut owned = own_texture_arena(&arena, &resources)?.textures;
        if owned.len() != 1 {
            return Err(format!(
                "character texture arena {:?} contains {} textures",
                request.entry_path,
                owned.len()
            )
            .into());
        }
        if owned[0].guid != request.source_asset_id {
            return Err(format!(
                "character texture {:?} TOC GUID {:016x} disagrees with source ID {:016x}",
                request.entry_path, owned[0].guid, request.source_asset_id
            )
            .into());
        }
        textures.push(CharacterTextureAsset {
            source_asset_id: request.source_asset_id,
            loaded_asset_id: request.loaded_asset_id,
            entry_path: request.entry_path.clone(),
            texture: owned.remove(0),
        });
    }

    let mut scene = CharacterScene {
        customizations: recipe.customizations.clone(),
        schema_name: recipe.schema_name.clone(),
        category: recipe.category,
        components,
        textures,
    };
    // PC port: Skate 2's head under a linked Skate 3 data folder's tops.
    crate::skate3::head_fit::fit_heads(&mut scene.components);
    // Gameplay clone3 normally receives the already rebound resident CAC copy.
    // This host reloads original arena bytes, so perform that same binding on
    // its private decoded resources before publication, including clone3 CAC.
    if matches!(recipe.clone_type, 1 | 2) || recipe.customizations.is_some() {
        rebind_character_materials(&mut scene);
    }
    add_missing_texture(mounts, &mut scene)?;
    Ok(scene)
}

/// The arena holding `TextureMan`'s boot textures.
const BOOT_TEXTURES: &str = "data\\content\\textures\\globaltextures.rx2";

/// A channel naming a texture the entity never loaded (`ExtractAssetList` 0x82d0c348 requests
/// only the recipe materials' textures, and the rebind keeps a channel its material names
/// nothing for) has no raster: `cModelInstance::Resolve` 0x82853e70 hands
/// `AddPegasusMaterialToScope` 0x82e17bb0 the boot [`MISSING_TEXTURE`] to bind in its place. The
/// scene keeps that texture beside its own when any channel needs it.
fn add_missing_texture(
    mounts: &MountSet,
    scene: &mut CharacterScene,
) -> Result<(), Box<dyn std::error::Error>> {
    let loaded: BTreeSet<u64> = scene
        .textures
        .iter()
        .map(|texture| texture.source_asset_id)
        .collect();
    let unbound = scene
        .components
        .iter()
        .flat_map(|component| &component.models)
        .flat_map(|model| &model.lods)
        .flat_map(|lod| &lod.arena.materials)
        .flat_map(|material| material.channels.iter().skip(2))
        .any(|channel| match channel.value {
            PresentationMaterialValue::Raster { guid, .. } => {
                channel.flags & 2 == 0 && !loaded.contains(&guid)
            }
            _ => false,
        });
    if !unbound {
        return Ok(());
    }
    let bytes = mounts.read_path(BOOT_TEXTURES)?;
    let resources = Rw4Arena::split_file_resources(&bytes)?;
    let arena = Rw4Arena::parse(&resources)?;
    let guid = skate2_formats::hash::rw_hash64_string(MISSING_TEXTURE);
    let texture = own_texture_asset(&arena, &resources, guid)?;
    scene.textures.push(CharacterTextureAsset {
        source_asset_id: guid,
        loaded_asset_id: guid,
        entry_path: BOOT_TEXTURES.to_owned(),
        texture,
    });
    Ok(())
}

/// Existing mounted host asset decoding, preserving serialized references.
///
/// # Errors
/// Returns the existing mount, parse, or unresolved asset-reference error.
pub fn own_texture_arena(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
) -> Result<TextureArena, Box<dyn std::error::Error>> {
    own_texture_arena_matching(arena, resources, |_| true)
}

/// Decode one exact TOC asset using the same ownership/level decoder as whole
/// texture arenas. Boot textures can share an arena with CPU-only surfaces.
pub fn own_texture_asset(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    guid: u64,
) -> Result<NativeTexture, Box<dyn std::error::Error>> {
    let mut owned = own_texture_arena_matching(arena, resources, |id| id == guid)?;
    if owned.textures.len() != 1 {
        return Err(format!("texture asset {guid:016x} must have exactly one TOC owner").into());
    }
    Ok(owned.textures.remove(0))
}

fn own_texture_arena_matching(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    select: impl Fn(u64) -> bool,
) -> Result<TextureArena, Box<dyn std::error::Error>> {
    let mut identities = BTreeMap::new();
    for entry in &arena.dictionary {
        if entry.type_id != TABLE_OF_CONTENTS_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or("texture TOC range is invalid")?;
        for item in TableOfContents::parse(bytes)?.entries {
            let target = require_local_entry(item.object, "texture TOC entry")?;
            identities.insert(target, (item.guid, item.name));
        }
    }
    let mut textures = Vec::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != TEXTURE_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or("texture object range is invalid")?;
        let texture = Texture::parse(bytes)?;
        let dictionary_index = u32::try_from(entry_index)?;
        let (guid, name) = identities
            .remove(&dictionary_index)
            .ok_or("texture object has no TOC identity")?;
        if !select(guid) {
            continue;
        }
        let data_entry = arena
            .local_entry(texture.data)
            .filter(|target| target.type_id == GPU_BUFFER_RESOURCE_TYPE_ID)
            .ok_or("texture payload link is invalid")?;
        let encoded_data = arena
            .object_bytes(resources, data_entry)
            .ok_or("texture payload range is invalid")?
            .to_vec();
        let linear_levels = texture.decode_levels(&encoded_data)?;
        textures.push(NativeTexture {
            dictionary_index,
            guid,
            name,
            texture_type: texture.texture_type,
            endian: texture.endian,
            width: texture.width,
            height: texture.height,
            depth: texture.depth,
            mip_levels: texture.mip_levels,
            pixel_format: texture.pixel_format,
            packed_mips: texture.packed_mips,
            mip_data_offset: texture.mip_data_offset,
            encoded_data,
            linear_levels,
        });
    }
    if !identities.is_empty() {
        return Err("texture TOC contains unresolved object identities".into());
    }
    Ok(TextureArena {
        id: arena.id,
        textures,
    })
}

/// Existing mounted host asset decoding, preserving serialized references.
///
/// # Errors
/// Returns the existing mount, parse, or unresolved asset-reference error.
pub fn own_presentation_arena(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
) -> Result<PresentationArena, Box<dyn std::error::Error>> {
    let mut owned = PresentationArena {
        id: arena.id,
        textures: if arena
            .dictionary
            .iter()
            .any(|entry| entry.type_id == TEXTURE_TYPE_ID)
        {
            own_texture_arena(arena, resources)?.textures
        } else {
            Vec::new()
        },
        materials: Vec::new(),
        meshes: Vec::new(),
        models: Vec::new(),
        instances: Vec::new(),
        anti_frustums: Vec::new(),
        emitter_waypoints: Vec::new(),
        visual_indicators: Vec::new(),
    };
    let material_entries = own_presentation_materials(arena, resources, &mut owned)?;
    let mesh_entries = own_presentation_meshes(arena, resources, &material_entries, &mut owned)?;
    let model_entries = own_presentation_models(
        arena,
        resources,
        &mesh_entries,
        &material_entries,
        &mut owned,
    )?;
    own_presentation_instances(arena, resources, &model_entries, &mut owned)?;
    for entry in arena.dictionary.iter().filter(|entry| {
        entry.type_id == crate::presentation_anti_frustums::ANTI_FRUSTUM_DATA_TYPE_ID
    }) {
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} anti-frustum data range is invalid", arena.id))?;
        owned.anti_frustums.push(std::sync::Arc::new(
            crate::presentation_anti_frustums::AntiFrustumSet::parse(bytes)?,
        ));
    }
    for entry in arena.dictionary.iter().filter(|entry| {
        entry.type_id == crate::world_particles::EMITTER_WAYPOINT_DATA_TYPE_ID
    }) {
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} emitter waypoint data range is invalid", arena.id))?;
        owned.emitter_waypoints.push(std::sync::Arc::new(
            crate::world_particles::EmitterWaypointData::parse(bytes)?,
        ));
    }
    owned.visual_indicators = own_visual_indicators(arena, resources);
    Ok(owned)
}

/// An arena's `tVisualIndicatorData` (`cProcessArenaAsset::ProcessVisualIndicatorData`
/// 0x82d5f5a0), in dictionary order. Data that does not decode is logged and left out rather
/// than failing the arena's other assets.
pub fn own_visual_indicators(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
) -> Vec<std::sync::Arc<skate2_formats::pegasus::VisualIndicatorData>> {
    arena
        .dictionary
        .iter()
        .filter(|entry| entry.type_id == skate2_formats::pegasus::VISUAL_INDICATOR_DATA_TYPE_ID)
        .filter_map(|entry| {
            let data = arena
                .object_bytes(resources, entry)
                .ok_or_else(|| "range is invalid".to_owned())
                .and_then(|bytes| {
                    skate2_formats::pegasus::VisualIndicatorData::parse(bytes)
                        .map_err(|error| format!("{error:?}"))
                });
            data.map_err(|error| eprintln!("arena {:08x} visual indicator data: {error}", arena.id))
                .ok()
                .map(std::sync::Arc::new)
        })
        .collect()
}

fn own_presentation_materials(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    owned: &mut PresentationArena,
) -> Result<MaterialSubreferenceMap, Box<dyn std::error::Error>> {
    let sections = arena.sections(resources)?;
    let mut material_targets = BTreeMap::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != R_MATERIAL_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} material range is invalid", arena.id))?;
        let data = RenderMaterialData::parse(bytes)?;
        let dictionary_index = u32::try_from(entry_index)?;
        for (material_index, material) in data.materials.iter().copied().enumerate() {
            let object_offset = data
                .material_object_offset(material_index)
                .ok_or("material object offset overflow")?;
            material_targets.insert((dictionary_index, object_offset), owned.materials.len());
            let source_channels = data.channels_for(material);
            let effect = source_channels.iter().find_map(|channel| {
                if channel.shader_input != "AttribulatorMaterialName" {
                    return None;
                }
                let MaterialChannelValue::Raster { stream_name, .. } = &channel.value else {
                    return None;
                };
                Some(PresentationMaterialEffect::from_serialized_name(
                    stream_name,
                ))
            });
            owned.materials.push(PresentationMaterial {
                flags: material.flags,
                effect,
                channels: source_channels.iter().map(own_material_channel).collect(),
            });
        }
    }
    let mut subreferences = BTreeMap::new();
    for (index, subreference) in sections.subreferences.iter().enumerate() {
        if subreference.object.section_index() != 0 {
            continue;
        }
        let key = (
            subreference.object.entry_index(),
            subreference.object_offset,
        );
        if let Some(&material_index) = material_targets.get(&key) {
            subreferences.insert(u32::try_from(index)?, material_index);
        }
    }
    Ok(subreferences)
}

fn own_material_channel(
    channel: &skate2_formats::material::MaterialChannel,
) -> PresentationMaterialChannel {
    let value = match &channel.value {
        MaterialChannelValue::Raster {
            guid, stream_name, ..
        } => PresentationMaterialValue::Raster {
            guid: *guid,
            stream_name: stream_name.clone(),
            resident_texture: None,
        },
        MaterialChannelValue::ScalarConstant(value) => {
            PresentationMaterialValue::ScalarConstant(*value)
        }
    };
    PresentationMaterialChannel {
        shader_input: channel.shader_input.clone(),
        flags: channel.flags,
        image_channel: channel.image_channel,
        value,
    }
}

fn own_presentation_meshes(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    material_entries: &MaterialSubreferenceMap,
    owned: &mut PresentationArena,
) -> Result<ObjectIndexMap, Box<dyn std::error::Error>> {
    let mut mesh_entries = BTreeMap::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != R_OPTI_MESH_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} render mesh range is invalid", arena.id))?;
        let data = OptimizedRenderMeshData::parse(bytes)?;
        let dictionary_index = u32::try_from(entry_index)?;
        mesh_entries.insert(dictionary_index, owned.meshes.len());
        owned.meshes.push(PresentationMesh {
            dictionary_index,
            bounding_box: data.bounding_box,
            material: data
                .material
                .map(|id| resolve_material_reference(id, material_entries))
                .transpose()?,
            skin_palette: data.remap_indices.clone(),
            geometry: data.decode_geometry(arena, resources)?,
        });
    }
    Ok(mesh_entries)
}

fn own_presentation_models(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    mesh_entries: &ObjectIndexMap,
    material_entries: &MaterialSubreferenceMap,
    owned: &mut PresentationArena,
) -> Result<ObjectIndexMap, Box<dyn std::error::Error>> {
    let mut model_entries = BTreeMap::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != R_MODEL_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} render model range is invalid", arena.id))?;
        let data = RenderModelData::parse(bytes)?;
        let meshes = data
            .meshes
            .into_iter()
            .map(|mesh| {
                Ok(PresentationModelMesh {
                    mesh_index: resolve_local_entry(mesh.mesh, mesh_entries, "render-model mesh")?,
                    material: mesh
                        .material
                        .map(|id| resolve_material_reference(id, material_entries))
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let dictionary_index = u32::try_from(entry_index)?;
        model_entries.insert(dictionary_index, owned.models.len());
        owned.models.push(PresentationModel {
            dictionary_index,
            bounding_box: data.bounding_box,
            island_bounding_boxes: data.island_bounding_boxes,
            island_areas: data.island_areas,
            total_bone_count: data.total_bone_count,
            named_bone_count: data.bone_count,
            inverse_bind_pose: data.inverse_bind_pose,
            bone_names: data.bone_names,
            meshes,
        });
    }
    Ok(model_entries)
}

fn own_presentation_instances(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    model_entries: &ObjectIndexMap,
    owned: &mut PresentationArena,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in &arena.dictionary {
        if entry.type_id != INSTANCE_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} instance data range is invalid", arena.id))?;
        for instance in InstanceData::parse(bytes)?.instances {
            let render_model = instance
                .render_model
                .map(|id| resolve_local_entry(id, model_entries, "instance render model"))
                .transpose()?;
            owned.instances.push(PresentationInstance {
                guid: instance.guid,
                local_guid: instance.local_guid,
                name: instance.name,
                transform: instance.transform,
                bounding_box_min: instance.bounding_box_min[..3].try_into().unwrap(),
                bounding_box_max: instance.bounding_box_max[..3].try_into().unwrap(),
                render_model,
            });
        }
    }
    Ok(())
}

/// Existing mounted host asset decoding, preserving serialized references.
///
/// # Errors
/// Returns the existing mount, parse, or unresolved asset-reference error.
pub fn require_local_entry(
    id: ObjectId,
    relationship: &str,
) -> Result<u32, Box<dyn std::error::Error>> {
    if id.section_index() != 0 {
        return Err(format!(
            "{relationship} uses nonlocal object id {:08x} (section {})",
            id.0,
            id.section_index()
        )
        .into());
    }
    Ok(id.entry_index())
}

/// Existing mounted host asset decoding, preserving serialized references.
///
/// # Errors
/// Returns the existing mount, parse, or unresolved asset-reference error.
pub fn resolve_local_entry(
    id: ObjectId,
    entries: &BTreeMap<u32, usize>,
    relationship: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let entry_index = require_local_entry(id, relationship)?;
    entries
        .get(&entry_index)
        .copied()
        .ok_or_else(|| format!("{relationship} targets missing entry {entry_index}").into())
}

/// Existing mounted host asset decoding, preserving serialized references.
///
/// # Errors
/// Returns the existing mount, parse, or unresolved asset-reference error.
pub fn resolve_material_reference(
    id: ObjectId,
    entries: &MaterialSubreferenceMap,
) -> Result<usize, Box<dyn std::error::Error>> {
    if id.section_index() != 2 {
        return Err(format!(
            "material reference {:08x} uses section {}, expected subreference section 2",
            id.0,
            id.section_index()
        )
        .into());
    }
    entries
        .get(&id.entry_index())
        .copied()
        .ok_or_else(|| format!("material reference {:08x} is unresolved", id.0).into())
}

/// RebindComponentToMaterial82d08fd8 over private clone1/2 arena ownership.
fn rebind_character_materials(scene: &mut CharacterScene) {
    // Native rebind visits every shader channel by name, retaining channels
    // absent from the selected material and replacing only texture values.
    for component in &mut scene.components {
        for model in &mut component.models {
            for lod in &mut model.lods {
                let Some(material) = lod
                    .material_instances
                    .first()
                    .and_then(|i| i.materials.first())
                else {
                    continue;
                };
                let mut targets = Vec::new();
                for instance in lod
                    .arena
                    .instances
                    .iter()
                    .filter(|i| i.guid == lod.asset_id)
                {
                    if let Some(model) = instance.render_model.and_then(|i| lod.arena.models.get(i))
                    {
                        for link in &model.meshes {
                            if let Some(index) = link.material.or_else(|| {
                                lod.arena
                                    .meshes
                                    .get(link.mesh_index)
                                    .and_then(|m| m.material)
                            }) {
                                if !targets.contains(&index) {
                                    targets.push(index);
                                }
                            }
                        }
                    }
                }
                for target in targets
                    .into_iter()
                    .filter_map(|index| lod.arena.materials.get(index).map(|_| index))
                    .collect::<Vec<_>>()
                {
                    let target = &mut lod.arena.materials[target];
                    for channel in &mut target.channels {
                        let Some(param) = material
                            .shader_parameters
                            .iter()
                            .find(|p| p.channel_name == channel.shader_input)
                        else {
                            continue;
                        };
                        if let crate::presentation::PresentationMaterialValue::Raster {
                            guid,
                            resident_texture,
                            ..
                        } = &mut channel.value
                        {
                            *guid = param.asset_id;
                            *resident_texture = None;
                            // Native RebindComponentToMaterial82d0936c changes
                            // the raster pointer only; retain serialized StreamName.
                        }
                    }
                }
            }
        }
    }
}
