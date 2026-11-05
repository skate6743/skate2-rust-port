//! Runtime-owned character graph assembled from a decoded recipe and RX2 assets.
//!
//! This mirrors the domain boundary built by `CharacterEntity::BuildTInstanceArray`
//! and `PresentationEntity::InitFromRecipe`: recipe components own model/LOD
//! selections, each loaded LOD owns a Pegasus instance and render arena, and
//! texture requests remain identified separately from the serialized source ID.

use crate::{
    presentation::{PresentationArena, PresentationMaterialValue},
    texture::NativeTexture,
};
use skate2_formats::{
    hash::{rw_hash32_string, rw_hash64_string},
    recipe::CharacterMaterialInstance,
    render::AxisAlignedBox,
};
use std::{collections::BTreeMap, fmt, sync::Arc};
#[path = "../rendering/character_palette_upload.rs"]
pub mod palette;

/// `TextureMan`'s boot stand-in for a texture nothing loaded (`ResolveMissingTexture` 0x8282c7c8).
pub const MISSING_TEXTURE: &str = "missingtexture.Texture";

pub type CharacterMatrix = [f32; 16];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterHierarchy {
    pub bone_names: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterSkinningPart {
    pub component_index: usize,
    pub model_index: usize,
    pub lod_index: usize,
    pub bone_remap: Vec<usize>,
    pub fallback_bone_count: usize,
    pub packed_matrix_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterSkinningState {
    pub lod_level: u8,
    pub hierarchy_inverse_bind_pose: Arc<[CharacterMatrix]>,
    pub parts: Vec<CharacterSkinningPart>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterSkinningError {
    MissingLodZero,
    MissingDefaultBone(String),
    MissingRenderInstance,
    MissingRenderModel(usize),
    BoneDataCountMismatch { names: usize, matrices: usize },
    MissingMesh(usize),
    SkinPaletteIndexOutsideModel { index: usize, bone_count: usize },
    HierarchyIndexOutsidePose { index: usize, pose_count: usize },
    PackedMatrixCountMismatch { expected: usize, actual: usize },
    MissingSkinningPart(usize),
}

impl fmt::Display for CharacterSkinningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid character skinning graph: {self:?}")
    }
}

impl std::error::Error for CharacterSkinningError {}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterScene {
    pub customizations: Option<skate2_formats::recipe::CharacterCustomizations>,
    pub schema_name: String,
    pub category: u32,
    pub components: Vec<CharacterComponentAsset>,
    pub textures: Vec<CharacterTextureAsset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterComponentAsset {
    pub name: String,
    pub models: Vec<CharacterModelAsset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterModelAsset {
    pub asset_id: u64,
    pub lods: Vec<CharacterLodAsset>,
}

/// Original first-mesh InitCloth candidate and its complete retained source.
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterRopaBinding {
    /// Presentation instance GUID; names the original ropa2/0xGUID.r2b.
    pub model_id:u64,
    pub model_index:usize,
    pub mesh_index:usize,
    pub effect:std::sync::Arc<skate2_formats::ropa::RopaEffect>,
    /// One unnormalized position-W particle index per source vertex.
    pub vertex_particle_indices:Vec<u32>,
    /// Skinning destination driver indices (CoordIndex.X, or position W).
    pub driver_particle_indices:Vec<u32>,
    /// InitSkinParams per-source-vertex offset into concatenated mesh palettes.
    pub matrix_palette_offsets:Vec<u16>,
    /// Original optimized mesh remap tables concatenated in model mesh order.
    pub matrix_palette:Vec<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterLodAsset {
    pub level: u8,
    pub asset_id: u64,
    pub arena_asset_id: u64,
    pub loaded_asset_id: u64,
    pub entry_path: String,
    pub material_instances: Vec<CharacterMaterialInstance>,
    /// Native InitCloth candidates whose original .r2b asset exists. None means
    /// the producer did not establish cloth admission; an empty list is proven.
    pub ropa_effects: Option<Vec<u64>>,
    pub ropa_bindings:Vec<CharacterRopaBinding>,
    pub arena: PresentationArena,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterTextureAsset {
    pub source_asset_id: u64,
    pub loaded_asset_id: u64,
    pub entry_path: String,
    pub texture: NativeTexture,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CharacterSceneCounts {
    pub components: usize,
    pub model_selections: usize,
    pub lods: usize,
    pub arenas: usize,
    pub instances: usize,
    pub render_models: usize,
    pub meshes: usize,
    pub skin_palette_entries: usize,
    pub total_bones: usize,
    pub named_bone_slots: usize,
    pub inverse_bind_matrices: usize,
    pub materials: usize,
    pub raster_channels: usize,
    pub resolved_texture_channels: usize,
    pub metadata_channels: usize,
    pub unresolved_sampled_channels: usize,
    pub constant_channels: usize,
    pub textures: usize,
}

impl CharacterScene {
    #[must_use]
    pub fn counts(&self) -> CharacterSceneCounts {
        let texture_index = self.texture_index();
        let mut counts = CharacterSceneCounts {
            components: self.components.len(),
            textures: self.textures.len(),
            ..CharacterSceneCounts::default()
        };
        for model in self
            .components
            .iter()
            .flat_map(|component| &component.models)
        {
            counts.model_selections += 1;
            for lod in &model.lods {
                counts.lods += 1;
                counts.arenas += 1;
                counts.instances += lod.arena.instances.len();
                counts.render_models += lod.arena.models.len();
                counts.meshes += lod.arena.meshes.len();
                counts.skin_palette_entries += lod
                    .arena
                    .meshes
                    .iter()
                    .map(|mesh| mesh.skin_palette.len())
                    .sum::<usize>();
                counts.materials += lod.arena.materials.len();
                for channel in lod
                    .arena
                    .materials
                    .iter()
                    .flat_map(|material| &material.channels)
                {
                    match channel.value {
                        PresentationMaterialValue::Raster { guid, .. } => {
                            counts.raster_channels += 1;
                            if texture_index.contains_key(&guid) {
                                counts.resolved_texture_channels += 1;
                            } else if matches!(
                                channel.shader_input.as_str(),
                                "AttribulatorMaterialName" | "Name"
                            ) {
                                counts.metadata_channels += 1;
                            } else {
                                counts.unresolved_sampled_channels += 1;
                            }
                        }
                        PresentationMaterialValue::ScalarConstant(_) => {
                            counts.constant_channels += 1;
                        }
                    }
                }
                for render_model in &lod.arena.models {
                    counts.total_bones += usize::from(render_model.total_bone_count);
                    counts.named_bone_slots += usize::from(render_model.named_bone_count);
                    counts.inverse_bind_matrices += render_model.inverse_bind_pose.len();
                }
            }
        }
        counts
    }

    /// Indexes the inner RX2 texture GUID used by material raster channels.
    /// The character asset manager separately keeps each request's hashed
    /// `loaded_asset_id`; the serialized material link uses the source GUID.
    #[must_use]
    pub fn texture_index(&self) -> BTreeMap<u64, usize> {
        self.textures
            .iter()
            .enumerate()
            .map(|(index, texture)| (texture.source_asset_id, index))
            .collect()
    }

    #[must_use]
    pub fn texture(&self, source_asset_id: u64) -> Option<&CharacterTextureAsset> {
        self.textures
            .iter()
            .find(|texture| texture.source_asset_id == source_asset_id)
    }

    /// The texture a material channel naming `source_asset_id` binds: the scene's own, else the
    /// boot [`MISSING_TEXTURE`] the load keeps for a channel naming a texture nothing loaded
    /// (`AddPegasusMaterialToScope` 0x82e17bb0 binds it for a channel without a raster).
    #[must_use]
    pub fn bound_texture_index(&self, source_asset_id: u64) -> Option<usize> {
        let position = |id| {
            self.textures
                .iter()
                .position(|texture| texture.source_asset_id == id)
        };
        position(source_asset_id).or_else(|| position(rw_hash64_string(MISSING_TEXTURE)))
    }

    /// `PresentationEntity::CalculateBoundsInfo` 0x827f2708: the first part's
    /// render-model box, widened lane by lane (x, y, z) by every later part
    /// at `lod_level`. The entity passes its LOD 0 model instances.
    ///
    /// # Errors
    ///
    /// Returns [`CharacterSkinningError`] when a part has no render model.
    pub fn level_bounds(
        &self,
        lod_level: u8,
    ) -> Result<Option<AxisAlignedBox>, CharacterSkinningError> {
        let mut bounds: Option<AxisAlignedBox> = None;
        for lod in self
            .components
            .iter()
            .flat_map(|component| &component.models)
            .flat_map(|model| &model.lods)
            .filter(|lod| lod.level == lod_level)
        {
            let part = lod_render_model(lod)?.bounding_box;
            let Some(union) = bounds.as_mut() else {
                bounds = Some(part);
                continue;
            };
            for lane in 0..3 {
                if union.minimum[lane] > part.minimum[lane] {
                    union.minimum[lane] = part.minimum[lane];
                }
                if part.maximum[lane] > union.maximum[lane] {
                    union.maximum[lane] = part.maximum[lane];
                }
            }
        }
        Ok(bounds)
    }

    /// SceneRenderView::UpdateEntityLOD82826a10 and recipe UpdateLOD828219a8.
    /// Character/vehicle callers supply their actual squared Vault distances.
    /// LOD2's distance is retained by native but never selects a fourth level.
    pub fn distance_lod(
        &self,
        position: [f32; 4],
        camera: [f32; 4],
        thresholds: [f32; 3],
    ) -> Result<u8, CharacterSkinningError> {
        let delta = std::array::from_fn(|i| position[i] - camera[i]);
        let distance = crate::reckoning_numeric::host_dot4(delta, delta);
        // First BGE accepts only ordered >=; second BLT rejects only <.
        let mut level = if distance >= thresholds[0] {
            if distance < thresholds[1] { 1_u8 } else { 2_u8 }
        } else {
            0_u8
        };
        while !self
            .components
            .iter()
            .flat_map(|c| &c.models)
            .flat_map(|m| &m.lods)
            .any(|lod| lod.level == level)
        {
            level = level
                .checked_sub(1)
                .ok_or(CharacterSkinningError::MissingLodZero)?;
        }
        Ok(level)
    }

    /// Reconstructs `RemapBones` and `FillInvBPBuffer` for all parts selected
    /// at one character LOD. Missing model bones use the named default bone,
    /// matching the reference's `Hips` fallback. Both look the default bone up
    /// only for a model bone the hierarchy lacks (`RemapBones` 0x827f8138,
    /// `FillInvBPBuffer` 0x827f8338), so a hierarchy without it still serves
    /// a model whose bones it all has.
    ///
    /// `FillInvBPBuffer` 0x827f8338 writes every part's matrices in
    /// `m_vLODMdlInstances` order, so a bone several models name keeps the
    /// last writer's. `BuildTInstanceArray` 0x82d088a8 builds that order by
    /// walking the component map, keyed by the pooled component name and
    /// compared by its `RwHash32Buffer` (`less` 0x82590cc0, unsigned). The
    /// returned parts keep recipe order.
    ///
    /// # Errors
    ///
    /// Returns [`CharacterSkinningError`] for a missing fallback, invalid arena
    /// ownership, or inconsistent serialized bone arrays.
    pub fn build_skinning_state(
        &self,
        lod_level: u8,
        hierarchy: &CharacterHierarchy,
        default_bone_name: &str,
    ) -> Result<CharacterSkinningState, CharacterSkinningError> {
        self.build_skinning_state_without(lod_level, hierarchy, default_bone_name, None)
    }

    /// [`Self::build_skinning_state`] without one component's models, which
    /// `PresentationEntity::RemoveComponentInstances` 0x827fa6e8 takes out of
    /// every LOD's instances before `InitSkinning` and `FillInvBPBuffer` run.
    ///
    /// # Errors
    ///
    /// As [`Self::build_skinning_state`].
    pub fn build_skinning_state_without(
        &self,
        lod_level: u8,
        hierarchy: &CharacterHierarchy,
        default_bone_name: &str,
        removed_component: Option<usize>,
    ) -> Result<CharacterSkinningState, CharacterSkinningError> {
        let fallback = || {
            hierarchy
                .bone_names
                .iter()
                .position(|name| name.eq_ignore_ascii_case(default_bone_name))
                .ok_or_else(|| {
                    CharacterSkinningError::MissingDefaultBone(default_bone_name.to_owned())
                })
        };
        let mut hierarchy_inverse_bind_pose = vec![identity_matrix(); hierarchy.bone_names.len()];
        let mut parts = Vec::new();
        for (component_index, component) in self.components.iter().enumerate() {
            if removed_component == Some(component_index) {
                continue;
            }
            for (model_index, model) in component.models.iter().enumerate() {
                for (lod_index, lod) in model.lods.iter().enumerate() {
                    if lod.level != lod_level {
                        continue;
                    }
                    let render_model = lod_render_model(lod)?;
                    if render_model.bone_names.len() != render_model.inverse_bind_pose.len() {
                        return Err(CharacterSkinningError::BoneDataCountMismatch {
                            names: render_model.bone_names.len(),
                            matrices: render_model.inverse_bind_pose.len(),
                        });
                    }
                    let mut fallback_bone_count = 0;
                    let bone_remap = render_model
                        .bone_names
                        .iter()
                        .map(|name| {
                            match hierarchy
                                .bone_names
                                .iter()
                                .position(|candidate| candidate.eq_ignore_ascii_case(name))
                            {
                                Some(index) => Ok(index),
                                None => {
                                    fallback_bone_count += 1;
                                    if crate::skate3::linked().is_some() {
                                        crate::skate3::note_unmatched_bone(name, &hierarchy.bone_names);
                                    }
                                    fallback()
                                }
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    parts.push(CharacterSkinningPart {
                        component_index,
                        model_index,
                        lod_index,
                        bone_remap,
                        fallback_bone_count,
                        packed_matrix_count: usize::from(render_model.total_bone_count),
                    });
                }
            }
        }
        let mut fill_order = (0..parts.len()).collect::<Vec<_>>();
        fill_order.sort_by_key(|&index| {
            rw_hash32_string(&self.components[parts[index].component_index].name)
        });
        for index in fill_order {
            let part = &parts[index];
            let component = &self.components[part.component_index];
            let lod = &component.models[part.model_index].lods[part.lod_index];
            let render_model = lod_render_model(lod)?;
            for ((&hierarchy_index, &matrix), name) in
                part.bone_remap.iter().zip(&render_model.inverse_bind_pose).zip(&render_model.bone_names)
            {
                // PC port: a bone the hierarchy lacks (a linked Skate 3 data folder's clothing)
                // stands on the default bone without giving it its bind pose, which every part
                // shares.
                if !hierarchy.bone_names[hierarchy_index].eq_ignore_ascii_case(name) {
                    continue;
                }
                hierarchy_inverse_bind_pose[hierarchy_index] = matrix;
            }
        }
        Ok(CharacterSkinningState {
            lod_level,
            hierarchy_inverse_bind_pose: hierarchy_inverse_bind_pose.into(),
            parts,
        })
    }

    /// Reconstructs `UpdateBoneTransforms` for one skinning part. The input is
    /// the hierarchy-ordered result of global-pose times inverse-bind, as built
    /// by `PoseWithMatMult`. The returned matrices are in the packed
    /// mesh-palette order consumed by the model instance, before its
    /// local-to-world upload transform is applied.
    ///
    /// # Errors
    ///
    /// Returns [`CharacterSkinningError`] for stale graph indices, bad palette
    /// entries, a short animated pose, or a packed-count disagreement.
    pub fn pack_skin_matrices(
        &self,
        state: &CharacterSkinningState,
        part_index: usize,
        skinning_pose: &[CharacterMatrix],
    ) -> Result<Vec<CharacterMatrix>, CharacterSkinningError> {
        let part = state
            .parts
            .get(part_index)
            .ok_or(CharacterSkinningError::MissingSkinningPart(part_index))?;
        let lod =
            &self.components[part.component_index].models[part.model_index].lods[part.lod_index];
        let model = lod_render_model(lod)?;
        let mut packed = Vec::with_capacity(part.packed_matrix_count);
        for model_mesh in &model.meshes {
            let mesh = lod
                .arena
                .meshes
                .get(model_mesh.mesh_index)
                .ok_or(CharacterSkinningError::MissingMesh(model_mesh.mesh_index))?;
            for &model_bone_index in &mesh.skin_palette {
                let model_bone_index = usize::from(model_bone_index);
                let &hierarchy_index = part.bone_remap.get(model_bone_index).ok_or(
                    CharacterSkinningError::SkinPaletteIndexOutsideModel {
                        index: model_bone_index,
                        bone_count: part.bone_remap.len(),
                    },
                )?;
                packed.push(*skinning_pose.get(hierarchy_index).ok_or(
                    CharacterSkinningError::HierarchyIndexOutsidePose {
                        index: hierarchy_index,
                        pose_count: skinning_pose.len(),
                    },
                )?);
            }
        }
        if packed.len() != part.packed_matrix_count {
            return Err(CharacterSkinningError::PackedMatrixCountMismatch {
                expected: part.packed_matrix_count,
                actual: packed.len(),
            });
        }
        Ok(packed)
    }
}

fn lod_render_model(
    lod: &CharacterLodAsset,
) -> Result<&crate::presentation::PresentationModel, CharacterSkinningError> {
    let model_index = lod
        .arena
        .instances
        .iter()
        .find_map(|instance| instance.render_model)
        .ok_or(CharacterSkinningError::MissingRenderInstance)?;
    lod.arena
        .models
        .get(model_index)
        .ok_or(CharacterSkinningError::MissingRenderModel(model_index))
}

fn identity_matrix() -> CharacterMatrix {
    [
        1.0, 0.0, 0.0, 0.0, // x
        0.0, 1.0, 0.0, 0.0, // y
        0.0, 0.0, 1.0, 0.0, // z
        0.0, 0.0, 0.0, 1.0, // w
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::{PresentationInstance, PresentationMesh, PresentationModelMesh};
    use skate2_formats::render::{AxisAlignedBox, NativeMeshGeometry};

    fn filled_matrix(value: f32) -> CharacterMatrix {
        [value; 16]
    }

    fn matrix_bits(matrix: CharacterMatrix) -> [u32; 16] {
        matrix.map(f32::to_bits)
    }

    #[test]
    fn counts_owned_component_lod_and_skeleton_graph() {
        let scene = CharacterScene {
            customizations: None,
            schema_name: "tutorial_male".to_owned(),
            category: 0,
            components: vec![CharacterComponentAsset {
                name: "SkateBoard".to_owned(),
                models: vec![CharacterModelAsset {
                    asset_id: 1,
                    lods: vec![CharacterLodAsset {
                        level: 0,
                        asset_id: 2,
                        arena_asset_id: 3,
                        loaded_asset_id: 4,
                        entry_path: "board.rx2".to_owned(),
                        material_instances: Vec::new(),
                        ropa_effects: None,
                        ropa_bindings:Vec::new(),
                        arena: PresentationArena {
                            textures: Vec::new(),
                            id: 5,
                            materials: Vec::new(),
                            meshes: Vec::new(),
                            models: vec![crate::presentation::PresentationModel {
                                dictionary_index: 0,
                                bounding_box: AxisAlignedBox {
                                    minimum: [0.0; 4],
                                    maximum: [1.0; 4],
                                },
                                island_bounding_boxes: Vec::new(),
                                island_areas: Vec::new(),
                                total_bone_count: 2,
                                named_bone_count: 3,
                                inverse_bind_pose: vec![[0.0; 16]; 3],
                                bone_names: vec!["Root".to_owned(); 3],
                                meshes: Vec::new(),
                            }],
                            instances: Vec::new(),
                            anti_frustums: Vec::new(),
                            emitter_waypoints: Vec::new(),
                            visual_indicators: Vec::new(),
                        },
                    }],
                }],
            }],
            textures: Vec::new(),
        };

        assert_eq!(
            scene.counts(),
            CharacterSceneCounts {
                components: 1,
                model_selections: 1,
                lods: 1,
                arenas: 1,
                instances: 0,
                render_models: 1,
                meshes: 0,
                skin_palette_entries: 0,
                total_bones: 2,
                named_bone_slots: 3,
                inverse_bind_matrices: 3,
                materials: 0,
                raster_channels: 0,
                resolved_texture_channels: 0,
                metadata_channels: 0,
                unresolved_sampled_channels: 0,
                constant_channels: 0,
                textures: 0,
            }
        );
    }

    #[test]
    fn remaps_named_bones_and_packs_mesh_palette() {
        let bounds = AxisAlignedBox {
            minimum: [0.0; 4],
            maximum: [1.0; 4],
        };
        let arena = PresentationArena {
            textures: Vec::new(),
            id: 5,
            materials: Vec::new(),
            meshes: vec![PresentationMesh {
                dictionary_index: 1,
                bounding_box: bounds,
                material: None,
                skin_palette: vec![1, 0],
                geometry: NativeMeshGeometry {
                    positions: Vec::new(),
                    attributes: Vec::new(),
                    indices: Vec::new(),
                    draws: Vec::new(),
                    position_format: 0,
                },
            }],
            models: vec![crate::presentation::PresentationModel {
                dictionary_index: 2,
                bounding_box: bounds,
                island_bounding_boxes: Vec::new(),
                island_areas: Vec::new(),
                total_bone_count: 2,
                named_bone_count: 2,
                inverse_bind_pose: vec![filled_matrix(10.0), filled_matrix(20.0)],
                bone_names: vec!["Arm".to_owned(), "Missing".to_owned()],
                meshes: vec![PresentationModelMesh {
                    mesh_index: 0,
                    material: None,
                }],
            }],
            instances: vec![PresentationInstance {
                guid: 0,
                local_guid: 0,
                name: String::new(),
                transform: identity_matrix(),
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                render_model: Some(0),
            }],
            anti_frustums: Vec::new(),
            emitter_waypoints: Vec::new(),
            visual_indicators: Vec::new(),
        };
        let scene = CharacterScene {
            customizations: None,
            schema_name: "test".to_owned(),
            category: 0,
            components: vec![CharacterComponentAsset {
                name: "Arm".to_owned(),
                models: vec![CharacterModelAsset {
                    asset_id: 1,
                    lods: vec![CharacterLodAsset {
                        level: 0,
                        asset_id: 2,
                        arena_asset_id: 3,
                        loaded_asset_id: 4,
                        entry_path: "arm.rx2".to_owned(),
                        material_instances: Vec::new(),
                        ropa_effects: None,
                        ropa_bindings:Vec::new(),
                        arena,
                    }],
                }],
            }],
            textures: Vec::new(),
        };
        let hierarchy = CharacterHierarchy {
            bone_names: vec!["hips".to_owned(), "arm".to_owned()],
        };

        let state = scene.build_skinning_state(0, &hierarchy, "Hips").unwrap();
        assert_eq!(state.parts[0].bone_remap, [1, 0]);
        assert_eq!(state.parts[0].fallback_bone_count, 1);
        assert_eq!(
            matrix_bits(state.hierarchy_inverse_bind_pose[0]),
            matrix_bits(filled_matrix(20.0))
        );
        assert_eq!(
            matrix_bits(state.hierarchy_inverse_bind_pose[1]),
            matrix_bits(filled_matrix(10.0))
        );
        let packed = scene
            .pack_skin_matrices(&state, 0, &[filled_matrix(1.0), filled_matrix(2.0)])
            .unwrap();
        assert_eq!(matrix_bits(packed[0]), matrix_bits(filled_matrix(1.0)));
        assert_eq!(matrix_bits(packed[1]), matrix_bits(filled_matrix(2.0)));
    }

    fn one_bone_component(name: &str, inverse_bind: CharacterMatrix) -> CharacterComponentAsset {
        let bounds = AxisAlignedBox {
            minimum: [0.0; 4],
            maximum: [1.0; 4],
        };
        CharacterComponentAsset {
            name: name.to_owned(),
            models: vec![CharacterModelAsset {
                asset_id: 1,
                lods: vec![CharacterLodAsset {
                    level: 0,
                    asset_id: 2,
                    arena_asset_id: 3,
                    loaded_asset_id: 4,
                    entry_path: format!("{name}.rx2"),
                    material_instances: Vec::new(),
                    ropa_effects: None,
                        ropa_bindings:Vec::new(),
                    arena: PresentationArena {
                        textures: Vec::new(),
                        id: 5,
                        materials: Vec::new(),
                        meshes: Vec::new(),
                        models: vec![crate::presentation::PresentationModel {
                            dictionary_index: 0,
                            bounding_box: bounds,
                            island_bounding_boxes: Vec::new(),
                            island_areas: Vec::new(),
                            total_bone_count: 1,
                            named_bone_count: 1,
                            inverse_bind_pose: vec![inverse_bind],
                            bone_names: vec!["RightArmTwist".to_owned()],
                            meshes: Vec::new(),
                        }],
                        instances: vec![PresentationInstance {
                            guid: 0,
                            local_guid: 0,
                            name: String::new(),
                            transform: identity_matrix(),
                            bounding_box_min: [0.0; 3],
                            bounding_box_max: [0.0; 3],
                            render_model: Some(0),
                        }],
                        anti_frustums: Vec::new(),
                        emitter_waypoints: Vec::new(),
                        visual_indicators: Vec::new(),
                    },
                }],
            }],
        }
    }

    #[test]
    fn shared_bone_keeps_the_last_component_in_name_hash_order() {
        // RwHash32 "OuterTorso" 0x09410273 < "Pants" 0x4dbff91d, so Pants
        // is written last although the recipe lists it first.
        let scene = CharacterScene {
            customizations: None,
            schema_name: "test".to_owned(),
            category: 0,
            components: vec![
                one_bone_component("Pants", filled_matrix(1.0)),
                one_bone_component("OuterTorso", filled_matrix(2.0)),
            ],
            textures: Vec::new(),
        };
        let hierarchy = CharacterHierarchy {
            bone_names: vec!["Hips".to_owned(), "RightArmTwist".to_owned()],
        };
        let state = scene.build_skinning_state(0, &hierarchy, "Hips").unwrap();
        let components = state.parts.iter().map(|part| part.component_index);
        assert!(components.eq([0, 1]));
        assert_eq!(
            matrix_bits(state.hierarchy_inverse_bind_pose[1]),
            matrix_bits(filled_matrix(1.0))
        );
    }

    #[test]
    fn a_removed_component_neither_skins_nor_fills_its_bones() {
        let scene = CharacterScene {
            customizations: None,
            schema_name: "test".to_owned(),
            category: 0,
            components: vec![
                one_bone_component("Pants", filled_matrix(1.0)),
                one_bone_component("OuterTorso", filled_matrix(2.0)),
            ],
            textures: Vec::new(),
        };
        let hierarchy = CharacterHierarchy {
            bone_names: vec!["Hips".to_owned(), "RightArmTwist".to_owned()],
        };
        let state = scene
            .build_skinning_state_without(0, &hierarchy, "Hips", Some(0))
            .unwrap();
        let components = state.parts.iter().map(|part| part.component_index);
        assert!(components.eq([1]));
        assert_eq!(
            matrix_bits(state.hierarchy_inverse_bind_pose[1]),
            matrix_bits(filled_matrix(2.0))
        );
    }

    #[test]
    fn a_pedestrian_that_hides_its_skateboard_keeps_it_out_at_every_lod() {
        let two_lods = |name: &str, inverse_bind| {
            let mut component = one_bone_component(name, inverse_bind);
            let mut far = component.models[0].lods[0].clone();
            far.level = 1;
            component.models[0].lods.push(far);
            component
        };
        let scene = Arc::new(CharacterScene {
            customizations: None,
            schema_name: "test".to_owned(),
            category: 0,
            components: vec![
                two_lods("Pants", filled_matrix(1.0)),
                two_lods("SkateBoard", filled_matrix(2.0)),
            ],
            textures: Vec::new(),
        });
        let hierarchy = CharacterHierarchy {
            bone_names: vec!["Hips".to_owned(), "RightArmTwist".to_owned()],
        };
        let components = |palette: &palette::CharacterPalette| {
            palette
                .skinning()
                .parts
                .iter()
                .map(|part| part.component_index)
                .collect::<Vec<_>>()
        };
        let shown =
            palette::CharacterPalette::pedestrian(Arc::clone(&scene), &hierarchy, 0, false).unwrap();
        assert_eq!(components(&shown), [0, 1]);
        let mut hidden =
            palette::CharacterPalette::pedestrian(Arc::clone(&scene), &hierarchy, 0, true).unwrap();
        assert_eq!(components(&hidden), [0]);
        assert!(hidden.select_pedestrian_lod(&hierarchy, 1).unwrap());
        assert_eq!(components(&hidden), [0]);
        let mut replayed = palette::CharacterPalette::from_replay_skinning(
            Arc::clone(&scene),
            hidden.skinning().clone(),
        )
        .unwrap();
        assert!(replayed.select_pedestrian_lod(&hierarchy, 0).unwrap());
        assert_eq!(components(&replayed), [0]);
    }
}

#[path = "../rendering/character_material_admission.rs"]
pub mod material_admission;
