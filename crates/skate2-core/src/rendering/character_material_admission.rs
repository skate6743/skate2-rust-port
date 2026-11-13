//! Selected clone3 material binding over the finalized scene and same palette.
use super::{
    CharacterScene,
    palette::{CharacterPalette, CharacterPaletteIdentity},
};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    input_cvar_runtime::InputCvarReferences,
    player::mounted_recipe::finalize::FinalizedRecipeScene,
    presentation::{PresentationMaterialValue, PresentationMesh},
    texture::NativeTexture,
};
use skate2_formats::attrib::string_hash64;
use std::sync::Arc;
#[path = "skater/skater_presentation_ropa_state.rs"]
pub(crate) mod ropa;

#[derive(Debug)]
pub enum CharacterMaterialError {
    AlreadyBound,
    EntityReplacementPending,
    ForeignOwner,
    Missing(&'static str),
    UnsupportedEffect(String),
    MaterialMetadata,
    Attribute {
        class: u64,
        collection: u64,
        reason: String,
    },
    Finalization(String),
    Ropa(String),
    EnabledClothPending,
    ClothUpdatePending,
}
impl std::fmt::Display for CharacterMaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyBound => f.write_str("selected material owner is already bound"),
            Self::EntityReplacementPending => f.write_str(
                "presentation header requires entity replacement outside selected admission",
            ),
            Self::ForeignOwner => {
                f.write_str("material source differs from finalized scene/palette")
            }
            Self::Missing(s) => write!(f, "selected material missing {s}"),
            Self::UnsupportedEffect(s) => {
                write!(f, "selected material effect {s:?} is unsupported")
            }
            Self::MaterialMetadata => {
                f.write_str("selected material requires known nonempty channel1 metadata")
            }
            Self::Attribute {
                class,
                collection,
                reason,
            } => write!(
                f,
                "material attribute {class:016x}/{collection:016x}: {reason}"
            ),
            Self::Finalization(s) => write!(f, "material finalized membership: {s}"),
            Self::Ropa(s) => write!(f, "material ropa state: {s}"),
            Self::ClothUpdatePending => f.write_str(
                "non-disabled cloth requires its live Update before repeated GPU publication",
            ),
            Self::EnabledClothPending => f.write_str(
                "native InitCloth must complete before material GPU skinning publication",
            ),
        }
    }
}
impl std::error::Error for CharacterMaterialError {}
#[derive(Clone)]
struct Entry {
    part: usize,
    mesh: usize,
    render_model: usize,
    mesh_index: usize,
    texture: usize,
    /// defaulthair's `i_alpha` texture.
    alpha: Option<usize>,
    cas_textures: [Option<usize>; 3],
    effect: String,
    material_key: (u64, u64),
}
/// Owns no geometry, texture copy or independent palette. Public methods only
/// expose checked borrowed inputs to the actual GPU consumer.
pub struct CharacterMaterialAdmission {
    scene: Arc<CharacterScene>,
    palette: CharacterPaletteIdentity,
    entries: Vec<Entry>,
    ropa: ropa::PresentationRopaState,
    entity_header: Option<(i64, bool)>,
    has_live_cloth_effects: Option<bool>,
    cloth: crate::skater_ropa_runtime::SkaterRopaRuntime,
    cvars: InputCvarReferences,
    vault: Arc<AttributeRuntimeVault>,
    /// PC port: customization colours set after the scene was built (`set_colours`), which the
    /// meshes take in place of the scene's own.
    colours: Option<Vec<skate2_formats::recipe::customizations::Colour>>,
}
pub struct CharacterMaterialView<'a> {
    /// Prepared CPU stream uses the entity transform instead of skin matrices.
    pub cpu_rows: Option<&'a [[f32;4]]>,
    /// GetCustomColour82cff8c8: base, highlight; untouched fourth lanes are one.
    pub colours: [[f32; 4]; 2],
    pub tattoos: [[f32; 4]; 2],
    pub normal: Option<&'a NativeTexture>,
    pub decal: Option<&'a NativeTexture>,
    pub decal2: Option<&'a NativeTexture>,
    pub geometry: &'a PresentationMesh,
    pub texture: &'a NativeTexture,
    /// The `alpha` channel defaulthair samples as `i_alpha`; None for the
    /// body effects, which declare no such sampler.
    pub alpha: Option<&'a NativeTexture>,
    pub effect: &'a str,
    /// The material's Attribulator class and variant
    /// (`GetAttribulatorKeyFromAttrMaterialName`), whose `DrawOrder` is the
    /// first `InstanceSortKey` byte (`CreateEffectListFromMaterialKey`
    /// 0x82e203f8).
    pub material_key: (u64, u64),
    pub ropa_vertex_params_x: Option<f32>,
}
#[cfg(test)]
#[path = "character_material_admission_tests.rs"]
mod tests;
impl CharacterMaterialAdmission {
    #[expect(
        clippy::too_many_lines,
        reason = "checked same-owner model, effect and texture admission"
    )]
    pub(crate) fn try_new(
        finalized: &FinalizedRecipeScene,
        palette: &CharacterPalette,
        vault: Arc<AttributeRuntimeVault>,
        cvars: InputCvarReferences,
    ) -> Result<Self, CharacterMaterialError> {
        use CharacterMaterialError as E;
        let scene = finalized
            .scene()
            .map_err(|e| E::Finalization(e.to_string()))?;
        if !Arc::ptr_eq(scene, palette.scene()) {
            return Err(E::ForeignOwner);
        }
        let visible = finalized
            .instances(palette.skinning().lod_level)
            .map_err(|e| E::Finalization(e.to_string()))?
            .collect::<Vec<_>>();
        let lod_zero = finalized
            .instances(0)
            .map_err(|e| E::Finalization(e.to_string()))?
            .collect::<Vec<_>>();
        let mut has_live_cloth_effects = Some(false);
        for (lod, instance) in finalized
            .ordered_instances()
            .map_err(|e| E::Finalization(e.to_string()))?
        {
            if !lod_zero.iter().any(|other| std::ptr::eq(*other, instance)) {
                continue;
            }
            match &lod.ropa_effects {
                Some(effects) if effects.contains(&instance.guid) => {
                    has_live_cloth_effects = Some(true)
                }
                None if has_live_cloth_effects != Some(true) => has_live_cloth_effects = None,
                _ => {}
            }
        }
        let mut entries = Vec::new();
        for (part_index, part) in palette.skinning().parts.iter().enumerate() {
            let lod = &scene.components[part.component_index].models[part.model_index].lods
                [part.lod_index];
            let mut instances = lod
                .arena
                .instances
                .iter()
                .filter(|i| visible.iter().any(|v| std::ptr::eq(*v, *i)));
            let instance = instances
                .next()
                .ok_or(E::Missing("finalized palette part instance"))?;
            if instances.next().is_some() {
                return Err(E::Missing("unambiguous finalized instance"));
            }
            let render_model = instance
                .render_model
                .ok_or(E::Missing("finalized render model"))?;
            let model = lod
                .arena
                .models
                .get(render_model)
                .ok_or(E::Missing("render model"))?;
            // Existing palette admission currently uses its first render model.
            // Do not silently bind different geometry while that route is retained.
            if lod.arena.instances.iter().find_map(|i| i.render_model) != Some(render_model) {
                return Err(E::ForeignOwner);
            }
            for (mesh_index, link) in model.meshes.iter().enumerate() {
                let mesh = lod
                    .arena
                    .meshes
                    .get(link.mesh_index)
                    .ok_or(E::Missing("model mesh"))?;
                // Resolve82853ef0/ef4 reads table[i].mesh then mesh+0x24.
                let material_index = mesh.material.ok_or(E::Missing("mesh material"))?;
                let material = lod
                    .arena
                    .materials
                    .get(material_index)
                    .ok_or(E::Missing("material"))?;
                let name = match material.channels.get(1).map(|c| &c.value) {
                    Some(PresentationMaterialValue::Raster { stream_name, .. })
                        if !stream_name.is_empty() =>
                    {
                        stream_name
                    }
                    _ => return Err(E::MaterialMetadata),
                };
                validate_material_name(name)?;
                let key =
                    crate::presentation::PresentationMaterialEffect::from_serialized_name(name);
                let class = key.attribute_class_id.ok_or(E::MaterialMetadata)?;
                let collection = key.material_variant_id.ok_or(E::MaterialMetadata)?;
                // PC port: a linked Skate 3 data folder's clothing names `material_character`
                // collections Skate 2 lacks; each reads as its Skate 2 counterpart throughout
                // (`crate::skate3`).
                let collection = match crate::skate3::character_material_alias(class, collection) {
                    Some(alias) if vault.instance(class, collection).is_null() => alias,
                    _ => collection,
                };
                let instance = vault.instance(class, collection);
                if instance.is_null() {
                    return Err(E::Missing(
                        "authored material collection; fallback admission pending",
                    ));
                }
                let text = instance
                    .try_get_text(string_hash64("effect"), 0)
                    .map_err(|e| E::Attribute {
                        class,
                        collection,
                        reason: e.to_string(),
                    })?;
                let effect = text
                    .as_ref()
                    .map_or(Ok("default"), |v| std::str::from_utf8(v.bytes()))
                    .map_err(|_| E::MaterialMetadata)?;
                if !matches!(
                    effect,
                    "defaultcharacter"
                        | "defaulthair"
                        | "defaulthair_ropa"
                        | "defaultcharacter_ropa"
                        | "cacstamp_cloth"
                        | "cacstamp_cloth_ropa"
                        | "cacstamp_skin"
                        | "cacstamp_face"
                        | "cacstamp_shift"
                        | "cac_hair"
                        | "cac_hair_ropa"
                        | "defaultlivingworld"
                ) {
                    return Err(E::UnsupportedEffect(effect.to_owned()));
                }
                // Scope writes are ordered; the final matching binding wins.
                let guid = diffuse_guid(material)?;
                let texture = scene
                    .bound_texture_index(guid)
                    .ok_or(E::Missing("original diffuse texture residency"))?;
                // defaulthair.fx defaultPS also samples `i_alpha`, bound from
                // the material's "alpha" channel the same way.
                let alpha = if matches!(
                    effect,
                    "defaulthair"
                        | "defaulthair_ropa"
                        | "cacstamp_skin"
                        | "cacstamp_face"
                        | "cacstamp_shift"
                        | "cac_hair"
                        | "cac_hair_ropa"
                ) {
                    match raster_guid(material, "alpha", "alpha channel", "raster alpha binding") {
                        Ok(guid) => Some(
                            scene
                                .bound_texture_index(guid)
                                .ok_or(E::Missing("original alpha texture residency"))?,
                        ),
                        // PC port: as a CAS map (below), a linked Skate 3 data folder's clothing
                        // can leave its alpha map out.
                        Err(E::Missing(_)) if crate::skate3::linked().is_some() => {
                            crate::skate3::note_missing_cas_channel(effect, "alpha");
                            None
                        }
                        Err(error) => return Err(error),
                    }
                } else {
                    None
                };
                let mut cas_textures = [None; 3];
                if effect.starts_with("cac") || matches!(effect, "defaultcharacter" | "defaulthair" | "defaulthair_ropa" | "defaultcharacter_ropa") {
                    for (slot, name) in ["normal", "decal", "decal2"].into_iter().enumerate() {
                        if (slot > 0 && (!effect.starts_with("cac") || effect.starts_with("cac_hair"))) || (slot == 2 && effect == "cacstamp_face") {
                            continue;
                        }
                        let guid = match raster_guid(material, name, "CAS texture channel", "CAS texture raster") {
                            Ok(guid) => guid,
                            // PC port: a linked Skate 3 data folder's clothing, read through
                            // Skate 2's `material_character`, can leave out a map Skate 2's
                            // program samples; the renderer draws it as none
                            // (`crate::skate3::note_missing_cas_channel`).
                            Err(E::Missing(_)) if crate::skate3::linked().is_some() => {
                                crate::skate3::note_missing_cas_channel(effect, name);
                                continue;
                            }
                            Err(error) => return Err(error),
                        };
                        cas_textures[slot] = Some(
                            scene
                                .bound_texture_index(guid)
                                .ok_or(E::Missing("original CAS texture residency"))?,
                        );
                    }
                }
                entries.push(Entry {
                    part: part_index,
                    mesh: mesh_index,
                    render_model,
                    mesh_index: link.mesh_index,
                    texture,
                    alpha,
                    cas_textures,
                    effect: effect.to_owned(),
                    material_key: (class, collection),
                });
            }
        }
        let cloth=crate::skater_ropa_runtime::SkaterRopaRuntime::new(palette.scene(),palette,&vault).map_err(E::Ropa)?;
        if has_live_cloth_effects.is_some(){has_live_cloth_effects=Some(!cloth.is_empty());}
        Ok(Self {
            scene: Arc::clone(scene),
            palette: palette.identity(),
            entries,
            ropa: ropa::PresentationRopaState::default(),
            entity_header: None,
            has_live_cloth_effects,
            cloth,
            vault,
            cvars,
            colours: None,
        })
    }
    pub(crate) fn try_construct_from_header(
        &mut self,
        header: &[u8],
    ) -> Result<(), CharacterMaterialError> {
        use CharacterMaterialError as E;
        let peer = i64::from_be_bytes(
            header
                .get(160..168)
                .ok_or(E::Missing("frame peer"))?
                .try_into()
                .expect("eight bytes"),
        );
        let flags = u32::from_be_bytes(
            header
                .get(192..196)
                .ok_or(E::Missing("frame locality"))?
                .try_into()
                .expect("four bytes"),
        );
        let identity = (peer, flags & 0x8000 != 0);
        // UpdatePresEntity827e61e8 replaces the entity only for another peer; a skater
        // that stops or starts being the local one keeps its entity.
        if self.entity_header.is_some_and(|old| old.0 != identity.0) {
            return Err(E::EntityReplacementPending);
        }
        if self.ropa.is_attempted() {
            return self
                .ropa
                .vertex_x()
                .map(|_| ())
                .map_err(|e| E::Ropa(e.to_string()))
                .and_then(|()| self.require_disabled_cloth());
        }
        self.entity_header = Some(identity);
        self.ropa
            .try_construct_with_effects(
                &self.scene.schema_name,
                flags & 0x8000 != 0,
                // UpdatePresEntity827e6298..62a8 uses ld + cmpdi -1.
                // Hex-Rays incorrectly narrows this test to LODWORD(peerId).
                peer != -1,
                &self.vault,
                &self.cvars,
                self.has_live_cloth_effects,
            )
            .map_err(|e| match e {
                ropa::PresentationRopaError::EnabledClothPending => E::EnabledClothPending,
                other => E::Ropa(other.to_string()),
            })?;
        self.require_disabled_cloth()
    }
    /// `SceneCharacterManager::CreateCharacterRenderable` (`0x82871150`)
    /// constructs a scene character's `SkaterPresEntity` with
    /// `isClothDisabled` = !`kNISCharacterClothOn` (default off), so its
    /// cloth is disabled whatever the entity's locality.
    pub(crate) fn try_construct_scene_character(&mut self) -> Result<(), CharacterMaterialError> {
        self.ropa
            .try_construct(&self.scene.schema_name, false, true, &self.vault, &self.cvars)
            .map_err(|e| CharacterMaterialError::Ropa(e.to_string()))?;
        self.require_disabled_cloth()
    }
    /// `SceneCharacterManager::InstanceFrozenSkater` (`0x82874f90`): the frozen copy is
    /// a new renderable of the local player's recipe (`GetPlayerRecipeID`),
    /// `CreateCharacterRenderable` (`0x82871150`) with `clothDisabled` set, which the
    /// `SkaterPresEntity` constructor (`0x827e5740`) builds non-local without cloth, so its
    /// ROPA meshes are GPU-skinned on the copy's pose. It binds this entity's materials.
    pub(crate) fn try_frozen_copy(&self) -> Result<Self, CharacterMaterialError> {
        let mut copy = Self {
            scene: Arc::clone(&self.scene),
            palette: self.palette.clone(),
            entries: self.entries.clone(),
            ropa: ropa::PresentationRopaState::default(),
            entity_header: None,
            has_live_cloth_effects: Some(false),
            cloth: crate::skater_ropa_runtime::SkaterRopaRuntime::without_effects(),
            cvars: self.cvars.clone(),
            vault: Arc::clone(&self.vault),
            colours: self.colours.clone(),
        };
        copy.try_construct_scene_character()?;
        Ok(copy)
    }
    /// CACPresEntity827ea540 supplies isClothDisabled=true to its base.
    /// It uses the same base BindConstants and then forces GPU skinning.
    pub(crate) fn try_construct_cas(&mut self) -> Result<(), CharacterMaterialError> {
        self.ropa
            .try_construct(
                &self.scene.schema_name,
                false,
                true,
                &self.vault,
                &self.cvars,
            )
            .map_err(|e| CharacterMaterialError::Ropa(e.to_string()))?;
        self.require_disabled_cloth()
    }
    fn require_disabled_cloth(&self) -> Result<(), CharacterMaterialError> {
        match self.ropa.cloth_disabled() {
            Some(true) => Ok(()),
            Some(false) if self.ropa.has_no_live_effects() => Ok(()),
            Some(false) if !self.cloth.is_empty() => Ok(()),
            Some(false) => Err(CharacterMaterialError::ClothUpdatePending),
            None => Err(CharacterMaterialError::Missing(
                "constructed presentation entity",
            )),
        }
    }
    /// # Errors
    /// Rejects foreign palettes, absent mesh bindings or incomplete ropa state.
    pub fn view<'a>(
        &'a self,
        palette: &CharacterPalette,
        part: usize,
        mesh: usize,
    ) -> Result<CharacterMaterialView<'a>, CharacterMaterialError> {
        use CharacterMaterialError as E;
        if self.palette != palette.identity() || !Arc::ptr_eq(&self.scene, palette.scene()) {
            return Err(E::ForeignOwner);
        }
        self.require_disabled_cloth()?;
        let entry = self
            .entries
            .iter()
            .find(|e| e.part == part && e.mesh == mesh)
            .ok_or(E::Missing("mesh binding"))?;
        let indices = &palette.skinning().parts[part];
        let lod = &self.scene.components[indices.component_index].models[indices.model_index].lods
            [indices.lod_index];
        let model = &lod.arena.models[entry.render_model];
        if model.meshes[mesh].mesh_index != entry.mesh_index {
            return Err(E::ForeignOwner);
        }
        let recipe_model =
            &self.scene.components[indices.component_index].models[indices.model_index];
        let mut colours = [[1.0; 4]; 2];
        // PC port: colours set since the scene was built stand for its own (`set_colours`).
        let custom_colours = self
            .colours
            .as_deref()
            .or_else(|| self.scene.customizations.as_ref().map(|custom| custom.colours.as_slice()));
        if let Some(custom_colours) = custom_colours {
            // The chosen LOD material vector is the same ordered material map
            // consumed by RebindToMaterials. Each instance maps one RModel mesh.
            if let Some(material) = lod
                .material_instances
                .get(mesh)
                .and_then(|m| m.materials.first())
            {
                if let Some(colour) = custom_colours.iter().find(|c| {
                    c.model_id == recipe_model.asset_id && c.material_id == material.asset_id
                }) {
                    for axis in 0..3 {
                        colours[0][axis] = f32::from_bits(colour.base_rgb_bits[axis]);
                        colours[1][axis] = f32::from_bits(colour.highlight_rgb_bits[axis]);
                    }
                }
            }
        }
        let mut tattoos = [[8.; 4]; 2];
        if let Some(custom) = &self.scene.customizations {
            for &[part, index] in &custom.embedded {
                let slot = match part {
                    0 => Some((0, 0)),
                    1 => Some((0, 1)),
                    2 => Some((0, 2)),
                    3 => Some((0, 3)),
                    4 => Some((1, 1)),
                    5 => Some((1, 0)),
                    _ => None,
                };
                if let Some((row, lane)) = slot {
                    tattoos[row][lane] = f32::from(index.min(8));
                }
            }
        }
        let cpu=if self.ropa.cloth_disabled()==Some(false) {
            self.cloth.view(part,mesh)
        }else{None};
        Ok(CharacterMaterialView {
            cpu_rows:cpu.map(|(_,rows)|rows),
            colours,
            tattoos,
            normal: entry.cas_textures[0].map(|i| &self.scene.textures[i].texture),
            decal: entry.cas_textures[1].map(|i| &self.scene.textures[i].texture),
            decal2: entry.cas_textures[2].map(|i| &self.scene.textures[i].texture),
            geometry: cpu.map_or(&lod.arena.meshes[entry.mesh_index],|(geometry,_)|geometry),
            texture: &self.scene.textures[entry.texture].texture,
            alpha: entry.alpha.map(|index| &self.scene.textures[index].texture),
            effect: &entry.effect,
            material_key: entry.material_key,
            ropa_vertex_params_x: if entry.effect.ends_with("_ropa") {
                Some(self.ropa.vertex_x().map_err(|e| E::Ropa(e.to_string()))?)
            } else {
                None
            },
        })
    }
    /// PC port: Edit Skater's change of colours alone (a colour row's, the colour picker's): the
    /// customization colours the meshes take from here on, the scene and its GPU objects kept.
    pub(crate) fn set_colours(&mut self, colours: Vec<skate2_formats::recipe::customizations::Colour>) {
        self.colours = Some(colours);
    }
    pub fn meshes(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.entries.iter().map(|e| (e.part, e.mesh))
    }
    pub(crate) fn update_cloth(&mut self,palette:&CharacterPalette,hips:[f32;16],inverse:[f32;16],
        world:[f32;16],clock:crate::skater_ropa_runtime::ClothClock,wipeout:u32)->Result<(),CharacterMaterialError>{
        if self.ropa.cloth_disabled()==Some(false)&&!self.ropa.has_no_live_effects()&&!self.cloth.is_empty(){
            self.cloth.update(palette,hips,inverse,world,clock,wipeout,&self.cvars).map_err(CharacterMaterialError::Ropa)?;
            self.ropa.cloth_initialized();
        }
        Ok(())
    }
    fn cloth_live(&self)->bool{
        self.ropa.cloth_disabled()==Some(false)&&!self.ropa.has_no_live_effects()&&!self.cloth.is_empty()
    }
    /// Whether the presentation's restore has yet to step the cloth to `frame`.
    pub(crate) fn cloth_needs_step(&self,frame:u32)->bool{
        self.cloth_live()&&self.cloth.needs_step(frame)
    }
    /// The step of `update_cloth`, without publishing.
    pub(crate) fn step_cloth(&mut self,palette:&CharacterPalette,hips:[f32;16],inverse:[f32;16],
        clock:crate::skater_ropa_runtime::ClothClock,wipeout:u32)->Result<(),CharacterMaterialError>{
        if self.cloth_live(){
            self.cloth.step(palette,hips,inverse,clock,wipeout,&self.cvars).map_err(CharacterMaterialError::Ropa)?;
        }
        Ok(())
    }
    /// The step of the entity a NIS borrows, by `time`.
    pub(crate) fn step_borrowed_cloth(
        &mut self,
        palette: &CharacterPalette,
        hips: [f32; 16],
        inverse: [f32; 16],
        time: crate::skater_ropa_runtime::ClothTime,
    ) -> Result<(), CharacterMaterialError> {
        if self.cloth_live() {
            self.cloth
                .step_borrowed(palette, hips, inverse, time, &self.cvars)
                .map_err(CharacterMaterialError::Ropa)?;
        }
        Ok(())
    }
    /// The step of a skater a replay-controlled scene restores, by the replay's `clock`;
    /// `wipeout` is the restored frame's `mFramesInWipeOut`.
    pub(crate) fn step_restored_cloth(
        &mut self,
        palette: &CharacterPalette,
        hips: [f32; 16],
        inverse: [f32; 16],
        clock: crate::skater_ropa_runtime::ClothClock,
        wipeout: u32,
    ) -> Result<(), CharacterMaterialError> {
        if self.cloth_live() {
            self.cloth
                .step_restored(palette, hips, inverse, clock, wipeout, &self.cvars)
                .map_err(CharacterMaterialError::Ropa)?;
        }
        Ok(())
    }
    /// Publishes the cloth `fraction` of the way from the step before the last to the last.
    pub(crate) fn publish_cloth(&mut self,palette:&CharacterPalette,world:[f32;16],fraction:f32)
        ->Result<(),CharacterMaterialError>{
        if self.cloth_live(){
            self.cloth.publish(palette,world,fraction).map_err(CharacterMaterialError::Ropa)?;
            self.ropa.cloth_initialized();
        }
        Ok(())
    }
    /// Actual retained constructor decision; absent until the header is consumed.
    #[must_use]
    pub fn cloth_disabled(&self) -> Option<bool> {
        self.ropa.cloth_disabled()
    }
}
fn diffuse_guid(
    material: &crate::presentation::PresentationMaterial,
) -> Result<u64, CharacterMaterialError> {
    raster_guid(
        material,
        "diffuse",
        "diffuse channel",
        "raster diffuse binding",
    )
}
/// The last channel named `name` from index 2 (scope writes are ordered, so
/// the final binding wins), which must be a sampled raster.
fn raster_guid(
    material: &crate::presentation::PresentationMaterial,
    name: &str,
    missing: &'static str,
    not_raster: &'static str,
) -> Result<u64, CharacterMaterialError> {
    use CharacterMaterialError as E;
    let channel = material
        .channels
        .iter()
        .skip(2)
        .rfind(|c| c.shader_input == name)
        .ok_or(E::Missing(missing))?;
    match &channel.value {
        PresentationMaterialValue::Raster { guid, .. } if channel.flags & 2 == 0 => Ok(*guid),
        _ => Err(E::Missing(not_raster)),
    }
}
fn validate_material_name(name: &str) -> Result<(), CharacterMaterialError> {
    let (class, variant) = name
        .split_once('.')
        .ok_or(CharacterMaterialError::MaterialMetadata)?;
    // Native asserts suffix strlen>1. Checked selected host admission stays
    // within both native scratch buffers and refuses embedded C terminators.
    if name.as_bytes().contains(&0) || class.len() > 246 || !(2..=255).contains(&variant.len()) {
        return Err(CharacterMaterialError::MaterialMetadata);
    }
    Ok(())
}
