//! Native render submissions reconstructed from the presentation scene.
//!
//! The reference `SceneRenderView::Sort` gathers visible mesh contexts and
//! radix-sorts their 32-bit `InstanceSortKey`. This module expresses that
//! behavior as ordinary Rust data. It has no console graphics or CPU state.

use crate::presentation::{
    PresentationArena, PresentationInstance, PresentationModelMesh, PresentationScene,
};
use skate2_formats::shader::{ShaderManifest, ShaderTechnique};
use std::{collections::BTreeMap, fmt};

/// The four bytes of `cInstanceScope::InstanceSortKey` in reference order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RenderSortKey {
    pub sort_order: u8,
    pub effect_scope: u8,
    pub pass_count: u8,
    pub material_scope: u8,
}

impl RenderSortKey {
    /// Returns the unsigned key consumed by the reference radix sort.
    #[must_use]
    pub const fn packed(self) -> u32 {
        u32::from_be_bytes([
            self.sort_order,
            self.effect_scope,
            self.pass_count,
            self.material_scope,
        ])
    }
}

/// Stable indices into a runtime-owned presentation scene.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderSceneHandle {
    pub collection: usize,
    pub arena: usize,
    pub instance: usize,
    pub model: usize,
    pub mesh: usize,
    pub material: usize,
}

/// One reference-shaped mesh context ready for visibility and backend upload.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeRenderSubmission {
    pub sort_key: RenderSortKey,
    pub scene: RenderSceneHandle,
    pub manifest_name: String,
    pub technique_name: String,
    pub scope_name: String,
    pub transform: [f32; 16],
    pub island_count: usize,
}

/// A mesh context whose reference-selected effect asset is absent from the
/// supplied corpus. It remains part of runtime state and cannot be silently
/// assigned an approximate shader.
#[derive(Clone, Debug, PartialEq)]
pub struct DeferredRenderSubmission {
    pub scene: RenderSceneHandle,
    pub manifest_name: String,
    pub material_name: String,
    pub transform: [f32; 16],
    pub island_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRenderQueue {
    pub submissions: Vec<NativeRenderSubmission>,
    pub deferred: Vec<DeferredRenderSubmission>,
    pub effect_scopes: usize,
    pub material_scopes: usize,
}

/// Recreates the monotonically assigned effect/material scope identifiers.
#[derive(Debug, Default)]
pub struct RenderScopeRegistry {
    next_effect_scope: u32,
    next_material_scope: u32,
    effect_scopes: BTreeMap<String, u8>,
    material_scopes: BTreeMap<(String, u64, u64), u8>,
}

impl RenderScopeRegistry {
    fn effect_scope(&mut self, name: &str) -> u8 {
        if let Some(value) = self.effect_scopes.get(name) {
            return *value;
        }
        self.next_effect_scope = self.next_effect_scope.wrapping_add(1);
        let value = self.next_effect_scope.to_be_bytes()[3];
        self.effect_scopes.insert(name.to_owned(), value);
        value
    }

    fn material_scope(&mut self, scope: &str, class: u64, variant: u64) -> u8 {
        let identity = (scope.to_owned(), class, variant);
        if let Some(value) = self.material_scopes.get(&identity) {
            return *value;
        }
        self.next_material_scope = self.next_material_scope.wrapping_add(1);
        let value = self.next_material_scope.to_be_bytes()[3];
        self.material_scopes.insert(identity, value);
        value
    }

    #[must_use]
    pub fn effect_scope_count(&self) -> usize {
        self.effect_scopes.len()
    }

    #[must_use]
    pub fn material_scope_count(&self) -> usize {
        self.material_scopes.len()
    }
}

impl NativeRenderQueue {
    /// Builds and stably sorts the currently resident renderable world meshes.
    ///
    /// The default technique is selected by name when present, matching the
    /// reference material-manager path. Otherwise the manifest's first
    /// technique remains its parser-selected initial technique.
    ///
    /// # Errors
    ///
    /// Returns [`RenderQueueError`] for an invalid scene link or absent effect
    /// manifest/technique/binding.
    pub fn build(
        scene: &PresentationScene,
        manifests: &BTreeMap<String, ShaderManifest>,
        scopes: &mut RenderScopeRegistry,
    ) -> Result<Self, RenderQueueError> {
        let mut submissions = Vec::new();
        let mut deferred = Vec::new();
        for (collection_index, collection) in scene.resident_collections().enumerate() {
            for (arena_index, arena) in collection.arenas.iter().enumerate() {
                for (instance_index, instance) in arena.instances.iter().enumerate() {
                    let Some(model_index) = instance.render_model else {
                        continue;
                    };
                    let model = arena.models.get(model_index).ok_or(
                        RenderQueueError::InvalidSceneLink {
                            kind: "instance model",
                            index: model_index,
                        },
                    )?;
                    for model_mesh in &model.meshes {
                        match prepare_mesh_submission(
                            arena,
                            instance,
                            model_mesh,
                            RenderSceneHandle {
                                collection: collection_index,
                                arena: arena_index,
                                instance: instance_index,
                                model: model_index,
                                mesh: model_mesh.mesh_index,
                                material: 0,
                            },
                            manifests,
                            scopes,
                        )? {
                            PreparedRenderSubmission::Native(submission) => {
                                submissions.push(submission);
                            }
                            PreparedRenderSubmission::Deferred(submission) => {
                                deferred.push(submission);
                            }
                        }
                    }
                }
            }
        }
        // Rust's slice sort is stable, retaining scene insertion order for
        // identical keys just as the reference stable LSD radix pass does.
        submissions.sort_by_key(|submission| submission.sort_key.packed());
        Ok(Self {
            submissions,
            deferred,
            effect_scopes: scopes.effect_scope_count(),
            material_scopes: scopes.material_scope_count(),
        })
    }
}

enum PreparedRenderSubmission {
    Native(NativeRenderSubmission),
    Deferred(DeferredRenderSubmission),
}

fn prepare_mesh_submission(
    arena: &PresentationArena,
    instance: &PresentationInstance,
    model_mesh: &PresentationModelMesh,
    mut scene: RenderSceneHandle,
    manifests: &BTreeMap<String, ShaderManifest>,
    scopes: &mut RenderScopeRegistry,
) -> Result<PreparedRenderSubmission, RenderQueueError> {
    let mesh =
        arena
            .meshes
            .get(model_mesh.mesh_index)
            .ok_or(RenderQueueError::InvalidSceneLink {
                kind: "model mesh",
                index: model_mesh.mesh_index,
            })?;
    // cModelInstance::Resolve follows the model entry's mesh pointer and reads
    // the material at mesh offset 0x24. For tROptiMeshData that is the
    // mesh-owned material field; the model table's second pointer is unused.
    let material_index = mesh.material.ok_or(RenderQueueError::MissingMaterial)?;
    scene.material = material_index;
    let material =
        arena
            .materials
            .get(material_index)
            .ok_or(RenderQueueError::InvalidSceneLink {
                kind: "mesh material",
                index: material_index,
            })?;
    let effect = material
        .effect
        .as_ref()
        .ok_or(RenderQueueError::MissingEffect)?;
    let binding = effect
        .binding
        .as_ref()
        .ok_or_else(|| RenderQueueError::UnboundEffect {
            material: effect.serialized_name.clone(),
        })?;
    let Some(manifest) = manifests.get(&binding.manifest_name) else {
        // PC port: a linked Skate 3 district's world effects ship no FX manifest: they are compiled
        // classes (`<name>_effect`, `InitializePrograms`) whose `default` technique draws one pass,
        // which the presentation's Skate 3 programs draw.
        if crate::skate3::linked().is_some() && SKATE3_COMPILED_WORLD_EFFECTS.contains(&binding.manifest_name.as_str()) {
            let class = effect.attribute_class_id.ok_or_else(|| RenderQueueError::UnboundEffect { material: effect.serialized_name.clone() })?;
            let variant = effect.material_variant_id.ok_or_else(|| RenderQueueError::UnboundEffect { material: effect.serialized_name.clone() })?;
            let sort_order = u8::try_from(binding.draw_order).map_err(|_| RenderQueueError::InvalidDrawOrder(binding.draw_order))?;
            let scope_name = format!("{}_default", binding.manifest_name);
            let sort_key = RenderSortKey {
                sort_order,
                effect_scope: scopes.effect_scope(&scope_name),
                pass_count: 1,
                material_scope: scopes.material_scope(&scope_name, class, variant),
            };
            return Ok(PreparedRenderSubmission::Native(NativeRenderSubmission {
                sort_key,
                scene,
                manifest_name: binding.manifest_name.clone(),
                technique_name: "default".to_owned(),
                scope_name,
                transform: instance.transform,
                island_count: mesh.geometry.draws.len(),
            }));
        }
        // Any other effect Skate 2 does not ship defers, as the manifest-less `default` effect
        // does, instead of failing the queue.
        let skate3_effect = crate::skate3::linked().is_some() && crate::skate3::note_missing_effect(&binding.manifest_name);
        if binding.manifest_name != "default" && !skate3_effect {
            return Err(RenderQueueError::MissingManifest {
                manifest: binding.manifest_name.clone(),
                material: effect.serialized_name.clone(),
            });
        }
        return Ok(PreparedRenderSubmission::Deferred(
            DeferredRenderSubmission {
                scene,
                manifest_name: binding.manifest_name.clone(),
                material_name: effect.serialized_name.clone(),
                transform: instance.transform,
                island_count: mesh.geometry.draws.len(),
            },
        ));
    };
    let technique = default_technique(manifest)
        .ok_or_else(|| RenderQueueError::MissingTechnique(binding.manifest_name.clone()))?;
    let scope_name = format!("{}_{}", binding.manifest_name, technique.name);
    let class = effect
        .attribute_class_id
        .ok_or_else(|| RenderQueueError::UnboundEffect {
            material: effect.serialized_name.clone(),
        })?;
    let variant = effect
        .material_variant_id
        .ok_or_else(|| RenderQueueError::UnboundEffect {
            material: effect.serialized_name.clone(),
        })?;
    let sort_order = u8::try_from(binding.draw_order)
        .map_err(|_| RenderQueueError::InvalidDrawOrder(binding.draw_order))?;
    let pass_count = u8::try_from(technique.passes.len())
        .map_err(|_| RenderQueueError::InvalidPassCount(technique.passes.len()))?;
    let sort_key = RenderSortKey {
        sort_order,
        effect_scope: scopes.effect_scope(&scope_name),
        pass_count,
        material_scope: scopes.material_scope(&scope_name, class, variant),
    };
    Ok(PreparedRenderSubmission::Native(NativeRenderSubmission {
        sort_key,
        scene,
        manifest_name: binding.manifest_name.clone(),
        technique_name: technique.name.clone(),
        scope_name,
        transform: instance.transform,
        island_count: mesh.geometry.draws.len(),
    }))
}

/// PC port: Skate 3's world effects with no FX manifest whose compiled class draws one `default`
/// pass, which the presentation ports (`environment_shading`'s Skate 3 programs and stand-ins).
const SKATE3_COMPILED_WORLD_EFFECTS: [&str; 12] = [
    "environmentdiffuse",
    "decalenvironment_simple",
    "decalenvironment_simple_tileable",
    "decalenvironment_tileable",
    "baseenvironmentreflective",
    "baseenvironmentreflective_simple",
    "transparentenvironmentreflective",
    "decal2environment",
    "oceanreflection",
    "wateralpha",
    "flowingwateralpha",
    "treeanimate",
];

fn default_technique(manifest: &ShaderManifest) -> Option<&ShaderTechnique> {
    manifest
        .techniques
        .iter()
        .find(|technique| technique.name == "default")
        .or_else(|| manifest.techniques.first())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderQueueError {
    InvalidSceneLink { kind: &'static str, index: usize },
    MissingMaterial,
    MissingEffect,
    UnboundEffect { material: String },
    MissingManifest { manifest: String, material: String },
    MissingTechnique(String),
    InvalidDrawOrder(i32),
    InvalidPassCount(usize),
}

impl fmt::Display for RenderQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid native render queue: {self:?}")
    }
}

impl std::error::Error for RenderQueueError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        presentation::{
            PresentationArena, PresentationCollection, PresentationInstance, PresentationMaterial,
            PresentationMaterialBinding, PresentationMaterialEffect, PresentationMesh,
            PresentationModel, PresentationModelMesh, PresentationScene,
        },
        streaming::StreamFocusState,
    };
    use skate2_formats::{
        render::{AxisAlignedBox, NativeMeshGeometry},
        shader::ShaderTechnique,
    };

    #[test]
    fn packs_reference_instance_sort_key_bytes() {
        assert_eq!(
            RenderSortKey {
                sort_order: 3,
                effect_scope: 4,
                pass_count: 2,
                material_scope: 9,
            }
            .packed(),
            0x0304_0209
        );
    }

    #[test]
    fn builds_and_sorts_native_mesh_contexts() {
        let mut scene = sample_scene();
        scene.collections[0].arenas[0].materials[0]
            .effect
            .as_mut()
            .unwrap()
            .binding
            .as_mut()
            .unwrap()
            .draw_order = 7;
        let manifests = BTreeMap::from([(
            "baseenvironment".to_owned(),
            ShaderManifest {
                version: "0.01.0".to_owned(),
                vertex_programs: Vec::new(),
                pixel_programs: Vec::new(),
                techniques: vec![ShaderTechnique {
                    name: "default".to_owned(),
                    passes: Vec::new(),
                }],
            },
        )]);
        let mut scopes = RenderScopeRegistry::default();
        let queue = NativeRenderQueue::build(&scene, &manifests, &mut scopes).unwrap();
        assert_eq!(queue.submissions.len(), 1);
        assert!(queue.deferred.is_empty());
        assert_eq!(queue.submissions[0].sort_key.sort_order, 7);
        assert_eq!(queue.submissions[0].sort_key.effect_scope, 1);
        assert_eq!(queue.submissions[0].sort_key.material_scope, 1);
        assert_eq!(queue.submissions[0].scope_name, "baseenvironment_default");
    }

    #[test]
    fn preserves_missing_reference_default_as_deferred_work() {
        let mut scene = sample_scene();
        let effect = scene.collections[0].arenas[0].materials[0]
            .effect
            .as_mut()
            .unwrap();
        effect.serialized_name = "decal.default".to_owned();
        effect.binding.as_mut().unwrap().manifest_name = "default".to_owned();
        let mut scopes = RenderScopeRegistry::default();
        let queue = NativeRenderQueue::build(&scene, &BTreeMap::new(), &mut scopes).unwrap();
        assert!(queue.submissions.is_empty());
        assert_eq!(queue.deferred.len(), 1);
        assert_eq!(queue.deferred[0].material_name, "decal.default");
        assert_eq!(queue.deferred[0].scene.mesh, 0);
        assert_eq!(queue.effect_scopes, 0);
        assert_eq!(queue.material_scopes, 0);
    }

    fn sample_scene() -> PresentationScene {
        let effect = PresentationMaterialEffect {
            serialized_name: "environment.default".to_owned(),
            attribute_class: Some("material_environment".to_owned()),
            attribute_class_id: Some(1),
            material_variant: Some("default".to_owned()),
            material_variant_id: Some(2),
            binding: Some(PresentationMaterialBinding {
                manifest_name: "baseenvironment".to_owned(),
                draw_order: 3,
            }),
        };
        PresentationScene {
            dependencies: Vec::new(),
            loose: Vec::new(),
            hulls: Vec::new(),
            proxies: Vec::new(),
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: StreamFocusState::new([0.0; 3], false),
            collections: vec![PresentationCollection {
                light_probes: Vec::new(),
        navigation: Vec::new(),
                id: 1,
                name: "cell".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![PresentationArena {
                    textures: Vec::new(),
                    id: 2,
                    materials: vec![PresentationMaterial {
                        flags: 0,
                        effect: Some(effect),
                        channels: Vec::new(),
                    }],
                    meshes: vec![PresentationMesh {
                        dictionary_index: 0,
                        bounding_box: AxisAlignedBox {
                            minimum: [0.0; 4],
                            maximum: [1.0; 4],
                        },
                        material: Some(0),
                        skin_palette: Vec::new(),
                        geometry: NativeMeshGeometry {
                            positions: vec![[0.0; 3]; 3],
                            attributes: Vec::new(),
                            indices: vec![0, 1, 2],
                            draws: Vec::new(),
                            position_format: 0,
                        },
                    }],
                    models: vec![PresentationModel {
                        dictionary_index: 0,
                        bounding_box: AxisAlignedBox {
                            minimum: [0.0; 4],
                            maximum: [1.0; 4],
                        },
                        island_bounding_boxes: Vec::new(),
                        island_areas: Vec::new(),
                        total_bone_count: 0,
                        named_bone_count: 0,
                        inverse_bind_pose: Vec::new(),
                        bone_names: Vec::new(),
                        meshes: vec![PresentationModelMesh {
                            mesh_index: 0,
                            material: Some(0),
                        }],
                    }],
                    instances: vec![PresentationInstance {
                        guid: 3,
                        local_guid: 4,
                        name: "instance".to_owned(),
                        transform: [0.0; 16],
                        bounding_box_min: [0.0; 3],
                        bounding_box_max: [1.0; 3],
                        render_model: Some(0),
                    }],
                    anti_frustums: Vec::new(),
                    emitter_waypoints: Vec::new(),
                    visual_indicators: Vec::new(),
                }],
            }],
        }
    }
}
