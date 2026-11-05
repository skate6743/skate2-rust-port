//! Runtime-owned presentation world assembled from native decoded geometry.

use crate::streaming::StreamFocusState;
use crate::texture::{ResidentTextureHandle, TextureScene};
use skate2_formats::{
    attrib::string_hash64,
    render::{AxisAlignedBox, NativeMeshGeometry},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationScene {
    pub location_name: String,
    pub focus: [f32; 3],
    pub stream_focus: StreamFocusState,
    pub collections: Vec<PresentationCollection>,
    /// Shared ancestor collections retained while at least one selected child needs them.
    pub dependencies: Vec<PresentationCollection>,
    /// Loose arenas `GameModule::OneOffLoadTexture` 0x82735730 registers with
    /// Presentation once (`LooseAssetHelper::LoadAssets` 0x8272cdb8 ->
    /// `SendPresRegArenaMsg`); resident for the session, outside stream focus.
    pub loose: Vec<PresentationCollection>,
    /// The mission hulls' presentation collections the hull loaders registered
    /// ([`HullPresentation`]), outside stream focus.
    pub hulls: Vec<PresentationCollection>,
    /// The registered PROXY collections (`proxy_world_manager`): the
    /// low-detail city wherever no high collection is registered. Loaded once
    /// and shared between generations, never edited.
    pub proxies: Vec<Arc<PresentationCollection>>,
}

/// A mission hull's presentation collections, as a `cHullLoader` hands them to the
/// presentation (`InitiateCollectionRegistration` `0x82eecd50` →
/// `cAssetStreamManager::NotifyCollectionReg`): the hull's loader ID, its clamped file name
/// (the stream `data\content\missions\<name>\<name>_Pres`) and its collection IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HullPresentation {
    pub hull: u32,
    pub file_name: String,
    pub collections: Vec<u64>,
    /// The hull's simulation collections (`<name>_Sim`), which `NotifyCollectionReg`
    /// (`0x82d5c8c0`) registers with the presentation as well: of them the presentation
    /// takes the visual indicator data.
    pub sim_collections: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationCollection {
    /// Registered hull probes, in asset publication order (SHLightingMan).
    pub light_probes: Vec<(u64, Arc<[skate2_formats::irradiance::LightProbe]>)>,
    pub id: u64,
    pub name: String,
    pub position: [f32; 3],
    pub distance_xz: f32,
    pub arenas: Vec<PresentationArena>,
    /// NavMesh2 arenas travel in BAM_Pres but publish Simulation kind23.
    /// Retain their existing decode through the matching collection lifetime.
    pub navigation: Vec<PresentationNavigationArena>,
}

#[derive(Clone, Debug)]
pub struct PresentationNavigationArena(pub Arc<crate::simulation_normal_arena_plan::SimulationNormalArenaPlan>);
impl PartialEq for PresentationNavigationArena {
    fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.0, &other.0) }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationArena {
    pub id: u32,
    pub textures: Vec<crate::texture::NativeTexture>,
    pub materials: Vec<PresentationMaterial>,
    pub meshes: Vec<PresentationMesh>,
    pub models: Vec<PresentationModel>,
    pub instances: Vec<PresentationInstance>,
    /// The arena's `tAntiFrustumData` occluders, in dictionary order.
    pub anti_frustums: Vec<Arc<crate::presentation_anti_frustums::AntiFrustumSet>>,
    /// The arena's `tEmitterWayPointData` blocks (`RegArena` asset type 0x1d), in dictionary
    /// order.
    pub emitter_waypoints: Vec<Arc<crate::world_particles::EmitterWaypointData>>,
    /// The arena's `tVisualIndicatorData` (`RegArena` asset type 0x11, which
    /// `ProcessVisualIndicatorData` 0x82d5f5a0 adds to the presentation registry from any
    /// arena): the streamed splines of `VIPresentation::AddStreamedSplineInstance`, in
    /// dictionary order.
    pub visual_indicators: Vec<Arc<skate2_formats::pegasus::VisualIndicatorData>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationMesh {
    pub dictionary_index: u32,
    pub bounding_box: AxisAlignedBox,
    pub material: Option<usize>,
    pub skin_palette: Vec<u16>,
    pub geometry: NativeMeshGeometry,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationModel {
    pub dictionary_index: u32,
    pub bounding_box: AxisAlignedBox,
    /// Original RModel island AABBs, flattened in mesh/island order.
    /// WorldPresEntityOptimesh827d41dc/423c uses these for shadow culling.
    pub island_bounding_boxes: Vec<AxisAlignedBox>,
    /// Original RModel `m_pIslandAreas`, in the same order; empty without one.
    pub island_areas: Vec<[f32; 4]>,
    pub total_bone_count: u16,
    pub named_bone_count: u16,
    pub inverse_bind_pose: Vec<[f32; 16]>,
    pub bone_names: Vec<String>,
    pub meshes: Vec<PresentationModelMesh>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationModelMesh {
    pub mesh_index: usize,
    pub material: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationMaterial {
    pub flags: u32,
    pub effect: Option<PresentationMaterialEffect>,
    pub channels: Vec<PresentationMaterialChannel>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationMaterialEffect {
    pub serialized_name: String,
    pub attribute_class: Option<String>,
    pub attribute_class_id: Option<u64>,
    pub material_variant: Option<String>,
    pub material_variant_id: Option<u64>,
    pub binding: Option<PresentationMaterialBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationMaterialBinding {
    pub manifest_name: String,
    pub draw_order: i32,
}

impl PresentationMaterialEffect {
    /// Splits the Attribulator material name using the reference
    /// `GetAttribulatorKeyFromAttrMaterialName` rule.
    #[must_use]
    pub fn from_serialized_name(name: &str) -> Self {
        let (attribute_class, material_variant) = name
            .split_once('.')
            .map_or((None, None), |(class, variant)| {
                (Some(format!("material_{class}")), Some(variant.to_owned()))
            });
        let attribute_class_id = attribute_class.as_deref().map(string_hash64);
        let material_variant_id = material_variant.as_deref().map(string_hash64);
        Self {
            serialized_name: name.to_owned(),
            attribute_class,
            attribute_class_id,
            material_variant,
            material_variant_id,
            binding: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationMaterialChannel {
    pub shader_input: String,
    pub flags: u16,
    pub image_channel: u16,
    pub value: PresentationMaterialValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PresentationMaterialValue {
    Raster {
        guid: u64,
        stream_name: String,
        resident_texture: Option<ResidentTextureHandle>,
    },
    ScalarConstant([f32; 4]),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MaterialTextureBindingCounts {
    pub raster_channels: usize,
    pub metadata_channels: usize,
    pub sampled_channels: usize,
    pub zero_guids: usize,
    pub resident: usize,
    pub unresolved: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresentationInstance {
    pub guid: u64,
    pub local_guid: u64,
    pub name: String,
    pub transform: [f32; 16],
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub render_model: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PresentationSceneCounts {
    pub arenas: usize,
    pub materials: usize,
    pub material_effects: usize,
    pub attributed_material_effects: usize,
    pub bound_material_effects: usize,
    pub material_channels: usize,
    pub raster_channels: usize,
    pub meshes: usize,
    pub models: usize,
    pub instances: usize,
    pub render_instances: usize,
    pub vertices: usize,
    pub vertex_attributes: usize,
    pub vertex_attribute_values: usize,
    pub indices: usize,
    pub draw_packets: usize,
}

impl PresentationScene {
    /// Stable draw/texture handle order: spatial collections, then ancestors,
    /// then loose arenas, then mission hulls, then registered proxies.
    pub fn resident_collections(&self) -> impl Iterator<Item = &PresentationCollection> {
        self.collections
            .iter()
            .chain(&self.dependencies)
            .chain(&self.loose)
            .chain(&self.hulls)
            .chain(self.proxies.iter().map(Arc::as_ref))
    }

    /// The editable collections; the shared proxies are excluded.
    pub fn resident_collections_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut PresentationCollection> {
        self.collections
            .iter_mut()
            .chain(&mut self.dependencies)
            .chain(&mut self.loose)
            .chain(&mut self.hulls)
    }

    pub fn resident_collection(&self, index: usize) -> Option<&PresentationCollection> {
        if index < self.collections.len() {
            return self.collections.get(index);
        }
        let index = index - self.collections.len();
        if index < self.dependencies.len() {
            return self.dependencies.get(index);
        }
        let index = index - self.dependencies.len();
        if index < self.loose.len() {
            return self.loose.get(index);
        }
        let index = index - self.loose.len();
        if index < self.hulls.len() {
            return self.hulls.get(index);
        }
        self.proxies.get(index - self.hulls.len()).map(Arc::as_ref)
    }

    #[must_use]
    pub fn current_collection(&self) -> Option<&PresentationCollection> {
        let current_id = self.stream_focus.current_id()?;
        self.collections
            .iter()
            .find(|collection| collection.id == current_id)
    }

    #[must_use]
    pub fn counts(&self) -> PresentationSceneCounts {
        let mut counts = PresentationSceneCounts::default();
        for collection in self.resident_collections() {
            counts.arenas += collection.arenas.len();
            for arena in &collection.arenas {
                counts.materials += arena.materials.len();
                counts.material_effects += arena
                    .materials
                    .iter()
                    .filter(|material| material.effect.is_some())
                    .count();
                counts.attributed_material_effects += arena
                    .materials
                    .iter()
                    .filter(|material| {
                        material
                            .effect
                            .as_ref()
                            .is_some_and(|effect| effect.attribute_class.is_some())
                    })
                    .count();
                counts.bound_material_effects += arena
                    .materials
                    .iter()
                    .filter(|material| {
                        material
                            .effect
                            .as_ref()
                            .is_some_and(|effect| effect.binding.is_some())
                    })
                    .count();
                counts.material_channels += arena
                    .materials
                    .iter()
                    .map(|material| material.channels.len())
                    .sum::<usize>();
                counts.raster_channels += arena
                    .materials
                    .iter()
                    .flat_map(|material| &material.channels)
                    .filter(|channel| {
                        matches!(channel.value, PresentationMaterialValue::Raster { .. })
                    })
                    .count();
                counts.meshes += arena.meshes.len();
                counts.models += arena.models.len();
                counts.instances += arena.instances.len();
                counts.render_instances += arena
                    .instances
                    .iter()
                    .filter(|instance| instance.render_model.is_some())
                    .count();
                counts.vertices += arena
                    .meshes
                    .iter()
                    .map(|mesh| mesh.geometry.positions.len())
                    .sum::<usize>();
                counts.vertex_attributes += arena
                    .meshes
                    .iter()
                    .map(|mesh| mesh.geometry.attributes.len())
                    .sum::<usize>();
                counts.vertex_attribute_values += arena
                    .meshes
                    .iter()
                    .flat_map(|mesh| &mesh.geometry.attributes)
                    .map(|attribute| attribute.values.len())
                    .sum::<usize>();
                counts.indices += arena
                    .meshes
                    .iter()
                    .map(|mesh| mesh.geometry.indices.len())
                    .sum::<usize>();
                counts.draw_packets += arena
                    .meshes
                    .iter()
                    .map(|mesh| mesh.geometry.draws.len())
                    .sum::<usize>();
            }
        }
        counts
    }

    pub fn bind_resident_textures(
        &mut self,
        textures: &TextureScene,
    ) -> MaterialTextureBindingCounts {
        let texture_index = textures.resident_texture_index();
        let mut counts = MaterialTextureBindingCounts::default();
        for channel in self
            .resident_collections_mut()
            .flat_map(|collection| &mut collection.arenas)
            .flat_map(|arena| &mut arena.materials)
            .flat_map(|material| &mut material.channels)
        {
            let PresentationMaterialValue::Raster {
                guid,
                resident_texture,
                ..
            } = &mut channel.value
            else {
                continue;
            };
            counts.raster_channels += 1;
            if matches!(
                channel.shader_input.as_str(),
                "Name" | "AttribulatorMaterialName"
            ) {
                counts.metadata_channels += 1;
                *resident_texture = None;
                continue;
            }
            counts.sampled_channels += 1;
            *resident_texture = texture_index.get(guid).copied();
            if *guid == 0 {
                counts.zero_guids += 1;
            } else if resident_texture.is_some() {
                counts.resident += 1;
            } else {
                counts.unresolved += 1;
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::{NativeTexture, TextureArena, TextureCollection};
    use skate2_formats::material::{TextureEndian, TextureType};
    use skate2_formats::render::NativeMeshGeometry;
    use skate2_formats::stream::FocusCollection;

    fn test_focus() -> StreamFocusState {
        let mut focus = StreamFocusState::new([0.0; 3], false);
        focus.reconcile(&[FocusCollection {
            index: 0,
            id: 7,
            position: [0.0; 3],
            distance_xz: 0.0,
            stream_order: 0,
        }]);
        focus
    }

    fn raster_channel(shader_input: &str, guid: u64) -> PresentationMaterialChannel {
        PresentationMaterialChannel {
            shader_input: shader_input.to_owned(),
            flags: 0,
            image_channel: u16::MAX,
            value: PresentationMaterialValue::Raster {
                guid,
                stream_name: shader_input.to_owned(),
                resident_texture: None,
            },
        }
    }

    #[test]
    fn splits_attribulator_effect_names_like_the_reference() {
        assert_eq!(
            PresentationMaterialEffect::from_serialized_name("environment.default"),
            PresentationMaterialEffect {
                serialized_name: "environment.default".to_owned(),
                attribute_class: Some("material_environment".to_owned()),
                attribute_class_id: Some(string_hash64("material_environment")),
                material_variant: Some("default".to_owned()),
                material_variant_id: Some(string_hash64("default")),
                binding: None,
            }
        );
        assert_eq!(
            PresentationMaterialEffect::from_serialized_name("model_default").attribute_class,
            None
        );
    }

    #[test]
    fn counts_runtime_owned_geometry() {
        let scene = PresentationScene {
            dependencies: Vec::new(),
            loose: Vec::new(),
            hulls: Vec::new(),
            proxies: Vec::new(),
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: test_focus(),
            collections: vec![PresentationCollection {
                light_probes: Vec::new(),
                navigation: Vec::new(),
                id: 7,
                name: "cell".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![PresentationArena {
                    textures: Vec::new(),
                    id: 9,
                    materials: Vec::new(),
                    meshes: vec![PresentationMesh {
                        dictionary_index: 0,
                        bounding_box: AxisAlignedBox {
                            minimum: [0.0; 4],
                            maximum: [1.0; 4],
                        },
                        material: None,
                        skin_palette: Vec::new(),
                        geometry: NativeMeshGeometry {
                            positions: vec![[0.0; 3]; 3],
                            attributes: Vec::new(),
                            indices: vec![0, 1, 2],
                            draws: Vec::new(),
                            position_format: 0,
                        },
                    }],
                    models: Vec::new(),
                    instances: Vec::new(),
                    anti_frustums: Vec::new(),
                    emitter_waypoints: Vec::new(),
                    visual_indicators: Vec::new(),
                }],
            }],
        };
        assert_eq!(scene.current_collection().unwrap().name, "cell");
        assert_eq!(scene.counts().vertices, 3);
        assert_eq!(scene.counts().indices, 3);
        let mut scene = scene;
        let mut parent = scene.collections[0].clone();
        parent.id = 8;
        scene.dependencies.push(parent);
        assert_eq!(scene.current_collection().unwrap().id, 7);
        assert_eq!(scene.resident_collection(0).unwrap().id, 7);
        assert_eq!(scene.resident_collection(1).unwrap().id, 8);
        assert!(scene.resident_collection(2).is_none());
        assert_eq!(scene.counts().vertices, 6);
        let mut loose = scene.collections[0].clone();
        loose.id = 9;
        scene.loose.push(loose);
        assert_eq!(scene.resident_collection(1).unwrap().id, 8);
        assert_eq!(scene.resident_collection(2).unwrap().id, 9);
        assert!(scene.resident_collection(3).is_none());
        assert_eq!(
            scene
                .resident_collections()
                .map(|collection| collection.id)
                .collect::<Vec<_>>(),
            [7, 8, 9]
        );
        assert_eq!(scene.counts().vertices, 9);
        let mut proxy = scene.collections[0].clone();
        proxy.id = 10;
        scene.proxies.push(Arc::new(proxy));
        assert_eq!(scene.resident_collection(2).unwrap().id, 9);
        assert_eq!(scene.resident_collection(3).unwrap().id, 10);
        assert!(scene.resident_collection(4).is_none());
        assert_eq!(scene.resident_collections_mut().count(), 3);
        assert_eq!(scene.counts().vertices, 12);
    }

    #[test]
    fn binds_only_sampled_material_channels_to_resident_textures() {
        let focus = test_focus();
        let mut scene = PresentationScene {
            dependencies: Vec::new(),
            loose: Vec::new(),
            hulls: Vec::new(),
            proxies: Vec::new(),
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: focus.clone(),
            collections: vec![PresentationCollection {
                light_probes: Vec::new(),
                navigation: Vec::new(),
                id: 7,
                name: "cell".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![PresentationArena {
                    textures: Vec::new(),
                    id: 9,
                    materials: vec![PresentationMaterial {
                        flags: 0,
                        effect: Some(PresentationMaterialEffect::from_serialized_name(
                            "environment.default",
                        )),
                        channels: vec![
                            raster_channel("diffuse", 11),
                            raster_channel("normal", 12),
                            raster_channel("Name", 11),
                        ],
                    }],
                    meshes: Vec::new(),
                    models: Vec::new(),
                    instances: Vec::new(),
                    anti_frustums: Vec::new(),
                    emitter_waypoints: Vec::new(),
                    visual_indicators: Vec::new(),
                }],
            }],
        };
        let textures = TextureScene {
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: focus,
            collections: vec![TextureCollection {
                id: 7,
                name: "textures".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![TextureArena {
                    id: 10,
                    textures: vec![NativeTexture {
                        dictionary_index: 0,
                        guid: 11,
                        name: "resident".to_owned(),
                        texture_type: TextureType::TwoDimensional,
                        endian: TextureEndian::None,
                        width: 1,
                        height: 1,
                        depth: 1,
                        mip_levels: 1,
                        pixel_format: 0,
                        packed_mips: false,
                        mip_data_offset: 0,
                        encoded_data: Vec::new(),
                        linear_levels: Vec::new(),
                    }],
                }],
            }],
        };
        let bindings = scene.bind_resident_textures(&textures);
        assert_eq!(bindings.raster_channels, 3);
        assert_eq!(bindings.metadata_channels, 1);
        assert_eq!(bindings.sampled_channels, 2);
        assert_eq!(bindings.resident, 1);
        assert_eq!(bindings.unresolved, 1);
    }
}
