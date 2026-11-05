//! Original loose CAS world, kept separate from the retained freeroam scene.
//! LoadLooseWorld827e3510 registers texture arena before geometry arena.
use crate::{
    gpu_renderer::{
        CameraRelativeClip, GpuRenderer, GpuScene, RenderResult, character::CharacterGhostState,
        character_palette::SelectedCharacterGpuBinding,
    },
    render_environment::EnvironmentLighting,
    render_inspection::Inspection,
    render_window::RenderWindowClient,
    selected_character_material,
};
use skate2_core::{
    edit_skater_character::EditSkaterCharacter,
    ui::edit_skater_presentation::{EditSkaterAnimation, EditSkaterPresentationAttributes},
};
use skate2_core::{
    mounted_character_scene::{own_presentation_arena, own_texture_arena},
    presentation::{PresentationCollection, PresentationScene},
    streaming::StreamFocusState,
    texture::{TextureCollection, TextureScene},
};
use skate2_formats::{mount::MountSet, rw4::Arena};
use std::{sync::Arc, time::Instant};

/// Owns the loose-world draw list independently of retained gameplay owners.
pub(crate) struct EditSkaterRoomView {
    room: Inspection,
    lighting: crate::render_irradiance::SkaterLighting,
    attributes: EditSkaterPresentationAttributes,
    animation: EditSkaterAnimation,
    bindings: Vec<SelectedCharacterGpuBinding>,
    recipe: Option<Arc<skate2_formats::recipe::CharacterRecipe>>,
    previous: Instant,
    male: bool,
    csm: std::collections::BTreeMap<u64, crate::render_shadows::CsmAttributes>,
    local_bounds: Option<[[f32; 3]; 2]>,
}
impl EditSkaterRoomView {
    pub(crate) fn load(mounts: &MountSet, male: bool, now: Instant) -> RenderResult<Self> {
        let assets = EditSkaterRoom::load(mounts)?;
        let room = Inspection::from_assets(
            mounts,
            assets.scene,
            assets.textures,
            EnvironmentLighting::load_location(mounts, CAS_RENDER_LOCATION)?,
        )?;
        Ok(Self {
            lighting: crate::render_irradiance::SkaterLighting::new(room.environment().default_irradiance),
            room,
            attributes: EditSkaterPresentationAttributes::load(mounts)?,
            animation: EditSkaterAnimation::load(
                mounts,
                male,
                if male { "CAC_male" } else { "CAC_female" },
            )?,
            bindings: vec![],
            recipe: None,
            previous: now,
            male,
            csm: Default::default(),
            local_bounds: None,
        })
    }
    pub(crate) fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<GpuScene> {
        self.room.initialize(renderer)
    }
    pub(crate) fn update(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        mounts: &MountSet,
        now: Instant,
        character: &mut EditSkaterCharacter,
        male: bool,
        body_key: u64,
        camera_key: u64,
        gesture: Option<u32>,
        near: f32,
    ) -> RenderResult<()> {
        if male != self.male {
            self.animation = EditSkaterAnimation::load(
                mounts,
                male,
                if male { "CAC_male" } else { "CAC_female" },
            )?;
            self.male = male;
        }
        self.animation.set_skate3_gesture_preview(gesture)?;
        let body = self.attributes.body(body_key)?;
        let camera = self.attributes.camera(camera_key)?;
        let projection = self.attributes.project_camera(&camera, near)?;
        let dt = now.saturating_duration_since(self.previous).as_secs_f32();
        let root = self.animation
            .update(dt, &body, &self.attributes, &mut character.palette)?;
        self.previous = now;
        if !self.recipe.as_ref().is_some_and(|r| Arc::ptr_eq(r, &character.recipe)) {
            // SkaterPresEntity constructor827e5a30..5b2c lowers local bounds
            // by one metre; SceneRenderView::Cull828501c0 adds LtoW translation.
            let bounds = character.palette.scene().level_bounds(0)?
                .ok_or("CAS LOD0 bounds absent")?;
            let mut bounds = [std::array::from_fn(|i|bounds.minimum[i]),
                std::array::from_fn(|i|bounds.maximum[i])];
            bounds[0][1] -= 1.;
            bounds[1][1] -= 1.;
            self.local_bounds = Some(bounds);
        }
        let hips = [root[12], root[13], root[14]];
        let bounds = self.local_bounds.ok_or("CAS retained bounds absent")?
            .map(|v|std::array::from_fn(|i|v[i] + hips[i]));
        if !self.csm.contains_key(&camera.postfx_csm_collection) {
            self.csm.insert(camera.postfx_csm_collection,
                EnvironmentLighting::load_frontend_csm(mounts, camera.postfx_csm_collection)?);
        }
        let clip = crate::live_skating_view::window_clip_from_world(
            projection.clip_from_world,
            renderer.dimensions(),
        );
        self.room
            .set_loose_world_view(clip, camera.position, projection.axes,
                crate::render_shadows::ShadowCasterInputs {
                    camera_position: camera.position, camera_forward: projection.axes[2],
                    skater_hips: hips, skater_bounds: bounds,
                }, self.csm[&camera.postfx_csm_collection]);
        self.room.frame(renderer, scene, now)?;
        self.lighting.update_frontend(self.room.light_probe_at(hips), dt);
        let relative = CameraRelativeClip::new(&clip, camera.position);
        let ghost = CharacterGhostState {
            opacity: 1.0,
            ghosted: false,
        };
        if !self
            .recipe
            .as_ref()
            .is_some_and(|r| Arc::ptr_eq(r, &character.recipe))
        {
            if let Some(path) = std::env::var_os("SKATE2_DIAGNOSTIC_CAS_DUMP") {
                use std::fmt::Write;
                let mut output = format!("clip={clip:?}\nrelative={relative:?}\nbones={:?}\nbody={body:?}\ncamera={camera:?}\ncustom={:?}\n",
                    self.animation.hierarchy().bone_names, character.recipe.customizations);
                for (part, mesh) in character.materials.meshes() {
                    let view = character.materials.view(&character.palette, part, mesh)?;
                    let geometry = &view.geometry.geometry;
                    writeln!(output, "mesh={part},{mesh},{}", view.effect)?;
                    writeln!(output, "rows={:?}", character.palette.mesh_rows(part, mesh)?.rows)?;
                    writeln!(output, "positions={:?}", geometry.positions)?;
                    writeln!(output, "indices={:?}", geometry.indices)?;
                    for a in &geometry.attributes {
                        writeln!(output, "{:?}={:?}", a.semantic, a.values)?;
                    }
                }
                std::fs::write(path, output)?;
            }
            let created = selected_character_material::create_objects(
                renderer,
                mounts,
                &character.palette,
                &character.materials,
                &relative,
                ghost,
                self.room.environment(),
                self.room.frame_environment(),
                    self.lighting.coefficients,
            )?;
            if created.is_empty() {
                return Err("CAS material admitted no GPU objects".into());
            }
            self.bindings.clear();
            scene.objects.clear();
            for (binding, object) in created {
                self.bindings.push(binding);
                scene.objects.push(object);
            }
            self.recipe = Some(Arc::clone(&character.recipe));
        } else {
            if self.bindings.len() != scene.objects.len() {
                return Err("CAS GPU object ownership mismatch".into());
            }
            for (binding, object) in self.bindings.iter().zip(&mut scene.objects) {
                selected_character_material::update_object(
                    renderer,
                    binding,
                    object,
                    &character.palette,
                    &character.materials,
                    &relative,
                    ghost,
                    self.room.environment(),
                    self.room.frame_environment(),
                    self.lighting.coefficients,
                )?;
            }
        }
        Ok(())
    }
}

pub(crate) const CAS_RENDER_LOCATION: u64 = 0x664b_0d04_8078_fe31;
pub(crate) const TEXTURES: &str = "data/content/world/models/DIST_petes_Textures.rx2";
pub(crate) const GEOMETRY: &str = "data/content/world/models/DIST_petes.rx2";

pub(crate) struct EditSkaterRoom {
    pub(crate) scene: PresentationScene,
    pub(crate) textures: TextureScene,
}

impl EditSkaterRoom {
    pub(crate) fn load(mounts: &MountSet) -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_paths(mounts,TEXTURES,GEOMETRY)
    }
    pub(crate) fn load_paths(mounts:&MountSet,textures_path:&str,geometry_path:&str)->Result<Self,Box<dyn std::error::Error>> {
        let bytes = mounts.read_path(textures_path)?;
        let resources = Arena::split_file_resources(&bytes)?;
        let arena = Arena::parse(&resources)?;
        let textures = own_texture_arena(&arena, &resources)?;
        let bytes = mounts.read_path(geometry_path)?;
        let resources = Arena::split_file_resources(&bytes)?;
        let arena = Arena::parse(&resources)?;
        let geometry = own_presentation_arena(&arena, &resources)?;
        if geometry.instances.is_empty() || geometry.models.is_empty() {
            return Err(format!("{geometry_path} contains no renderable original room instances").into());
        }
        // These are host collection envelopes, not invented native stream
        // collections: native loose-world instances retain their RX2 transforms.
        let origin = [0.0; 3];
        let mut scene = PresentationScene {
            location_name: geometry_path.to_owned(),
            focus: origin,
            stream_focus: StreamFocusState::new(origin, true),
            collections: vec![],
            dependencies: vec![],
            proxies: vec![],
            hulls: vec![],
            loose: vec![PresentationCollection {
                light_probes: Vec::new(),
                id: u64::from(geometry.id),
                name: geometry_path.to_owned(),
                position: origin,
                distance_xz: 0.0,
                arenas: vec![geometry],
                navigation: vec![],
            }],
        };
        // Loose-world registration uses the same native material manager as
        // the streamed world (CreateEffectListFromPegasusMaterialUnSkinned
        // 82e20948). Owning RX2 bytes alone leaves every effect unbound.
        // Resolve original Vault effects/draw orders before queue admission.
        crate::bind_runtime_material_attributes(mounts, &mut scene)?;
        let textures = TextureScene {
            location_name: textures_path.to_owned(),
            focus: origin,
            stream_focus: StreamFocusState::new(origin, true),
            collections: vec![TextureCollection {
                id: u64::from(textures.id),
                name: textures_path.to_owned(),
                position: origin,
                distance_xz: 0.0,
                arenas: vec![textures],
            }],
        };
        Ok(Self { scene, textures })
    }
}
