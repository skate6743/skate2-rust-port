//! Explicit original-map inspection, separate from the unfinished skating runtime.
use crate::{
    gpu_renderer::{
        DiagnosticFragment, DiagnosticPipelineKey, EnvironmentEffect, FrameEnvironment, GpuMap,
        GpuMesh, GpuRenderer, GpuResourceBuilder, GpuScene, GpuTexture, InstanceMaterial,
        MapEnvironment, RasterState, RenderResult, WorldShadowGeometry,
    },
    render_environment::EnvironmentLighting,
    render_shadows::ShadowCasterInputs,
    render_window::RenderWindowClient,
    render_world_painter::WorldPainterBlend,
};
use skate2_core::{
    WorldPainterKeys,
    player::selected_spawn_location,
    presentation::{PresentationMaterial, PresentationMaterialValue, PresentationScene},
    render::{NativeRenderQueue, RenderScopeRegistry},
    shader_texture_table::ShaderTextureTable,
    texture::{ResidentTextureHandle, TextureScene},
};
use skate2_formats::mount::MountSet;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Instant,
};
use winit::{
    event::{ElementState, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

pub(crate) fn run() -> RenderResult<()> {
    let root = std::env::var_os("SKATE2_GAME_ROOT")
        .ok_or("set SKATE2_GAME_ROOT to the external original game directory")?;
    let mounts = crate::mount_reference_archives(std::path::Path::new(&root))?;
    crate::verify_world_stream_probe_identity(&mounts)?;
    let mut client = Inspection::load(&mounts)?;
    eprintln!(
        "INSPECTION ONLY: {} original mesh contexts, {} deferred effects; original default/alphatest/base/decal/transparent/decalzbias environment and sky shading where every channel resolves, diagnostic unlit diffuse elsewhere, location fog/sky (no WorldPainter keys without the simulation), inspection camera, no skating simulation. Arrow keys move target; PageUp/PageDown change view distance; Escape closes.",
        client.queue.submissions.len(),
        client.queue.deferred.len()
    );
    let args: Vec<_> = std::env::args().collect();
    let option = |name: &str| -> RenderResult<Option<&str>> {
        args.iter()
            .position(|a| a == name)
            .map(|i| {
                args.get(i + 1)
                    .map(String::as_str)
                    .ok_or_else(|| format!("{name} requires a value").into())
            })
            .transpose()
    };
    let vector = |name: &str| -> RenderResult<Option<[f32; 3]>> {
        option(name)?
            .map(|text| {
                let values = text
                    .split(',')
                    .map(str::parse::<f32>)
                    .collect::<Result<Vec<_>, _>>()?;
                let point: [f32; 3] = values
                    .try_into()
                    .map_err(|_| format!("{name} requires x,y,z"))?;
                if !point.iter().all(|v| v.is_finite()) {
                    return Err(format!("{name} requires finite coordinates").into());
                }
                Ok(point)
            })
            .transpose()
    };
    if let Some(target) = vector("--render-target")? {
        client.focus = target;
    }
    client.inspection_eye = vector("--render-eye")?;
    if client.inspection_eye.is_some_and(|eye| {
        let d: [f32; 3] = std::array::from_fn(|i| eye[i] - client.focus[i]);
        d[0] * d[0] + d[2] * d[2] < 1e-8
    }) {
        return Err("inspection eye must have a horizontal separation from target".into());
    }
    let options = crate::render_window::RenderWindowOptions {
        frame_limit: option("--render-frames")?.map(str::parse).transpose()?,
        capture: option("--render-capture")?.map(Into::into),
        title: None,
        hold_at_completion: false,
    };
    crate::render_window::run_with_options(client, options)
}

pub(crate) struct Inspection {
    scene: Option<PresentationScene>,
    world_probes: crate::render_irradiance::WorldProbes,
    /// The resident arenas' emitter waypoint blocks, kept while a map build holds the scene.
    emitter_waypoints: Vec<Arc<skate2_core::world_particles::EmitterWaypointData>>,
    textures: Option<TextureScene>,
    streaming: Option<crate::city_streaming::CityStreaming>,
    /// The hull loaders' registered presentation collections, as the session last gave them.
    hull_presentations: Vec<skate2_core::presentation::HullPresentation>,
    /// The additional world the session holds loaded, as it last gave it.
    additional_world: Option<String>,
    proxy_textures: Option<crate::proxy_texture_world::ProxyTextures>,
    queue: NativeRenderQueue,
    focus: [f32; 3],
    distance: f32,
    keys: BTreeSet<KeyCode>,
    previous: Instant,
    /// Clip-from-world and camera world position from the live camera.
    external_view: Option<([f32; 16], [f32; 3])>,
    /// Explicit inspection-only viewpoint for reproducible material captures.
    inspection_eye: Option<[f32; 3]>,
    /// The local skater's hips (`SkaterPresentation::GetHipsPos`), where
    /// `UpdateRenderParams` samples the sun map; the camera without one.
    sun_sample_position: Option<[f32; 3]>,
    /// The camera and skater `RenderShadows_CSM` places its cascades from;
    /// without them no cascade has a caster.
    shadow_inputs: Option<ShadowCasterInputs>,
    frontend_csm: Option<crate::render_shadows::CsmAttributes>,
    /// The live frame's WorldPainter keys (or why the lookup failed).
    painter_keys: Option<Result<WorldPainterKeys, String>>,
    painter: WorldPainterBlend,
    painter_previous: Option<Instant>,
    painter_error_reported: bool,
    frame_environment: FrameEnvironment,
    animation_clock: crate::render_animation_clock::WorldAnimationClock,
    /// The front end scene's time step this frame, which drives the clock in place of the
    /// simulation's ticks.
    scene_delta: Option<f32>,
    simulation_tick: u32,
    /// The fraction of the way to `simulation_tick` the frame shows.
    simulation_fraction: f32,
    traffic_light_status: [[f32; 4]; 2],
    camera_axes: Option<[[f32; 3]; 3]>,
    water_animation: crate::render_water_animation::WaterAnimation,
    diagnostic_techniques: DiagnosticTechniques,
    environment: Arc<EnvironmentLighting>,
    /// `RenderMan::m_RenderLocation`'s world location (`None`: the default one).
    render_location: Option<u64>,
    /// The environment as loaded, at the default location.
    loaded_environment: Option<Arc<EnvironmentLighting>>,
    /// The game's and the downloadable content's vaults, read for the content's vault paths
    /// listed, and the loaded environment with their collections added, which a location of the
    /// content's names and which stays in place after it.
    location_vaults: Option<(Vec<String>, Arc<skate2_core::challenge::attrib_db::AttribDb>, Arc<EnvironmentLighting>)>,
    environment_resources: Option<Arc<wgpu::BindGroup>>,
    // Keys use archive collection identity, never the moving scene index.
    mesh_cache: BTreeMap<(u64, usize, usize), GpuMesh>,
    texture_cache: BTreeMap<(bool, u64, usize, usize), GpuTexture>,
    material_cache: BTreeMap<MaterialKey, Arc<wgpu::BindGroup>>,
    diagnostic_pipelines: BTreeMap<DiagnosticPipelineKey, Arc<wgpu::RenderPipeline>>,
    pending_gpu: Option<std::sync::mpsc::Receiver<Result<(MapBuild, GpuMap), String>>>,
    retire_gpu: Option<std::sync::mpsc::Sender<RetiredMap>>,
    media_focus:Option<[f32;3]>,
}
impl Inspection {
    pub(crate) fn navigation(&self) -> Vec<skate2_core::presentation::PresentationNavigationArena> {
        self.scene.as_ref().map_or_else(Vec::new, |scene| scene.dependencies.iter()
            .chain(&scene.collections).flat_map(|collection| collection.navigation.iter().cloned()).collect())
    }
    pub(crate) fn light_probe_at(&mut self, position: [f32;3]) -> skate2_formats::irradiance::Coefficients {
        // kbRenderEnableWorldSHProbes82f6f840 has native enabled default.
        self.world_probes.sample(position, self.environment.use_world_sh_probes, self.environment.default_irradiance)
    }
    pub(crate) fn frame_environment(&self)->FrameEnvironment { self.frame_environment }
    /// The presented time the last frame advanced the world's animation by.
    pub(crate) fn animation_delta(&self)->f32 { self.animation_clock.last_delta() }
    pub(crate) fn emitter_waypoints(&self)->&[Arc<skate2_core::world_particles::EmitterWaypointData>] { &self.emitter_waypoints }
    /// Resolve the original boot asset from the existing retained texture owner.
    pub(crate) fn vignette_texture(&mut self, renderer: &GpuRenderer) -> RenderResult<GpuTexture> {
        self.postfx_texture(renderer, "vignette_gradient.Texture")
    }
    /// PostFX::SetTexturePointers 82828840 resolves the shipped x-ray plate.
    pub(crate) fn xray_texture(&mut self, renderer: &GpuRenderer) -> RenderResult<GpuTexture> {
        self.postfx_texture(renderer, "xray.Texture")
    }
    pub(crate) fn postfx_texture(&mut self, renderer: &GpuRenderer, name: &str) -> RenderResult<GpuTexture> {
        let guid=skate2_formats::hash::rw_hash64_string(name);
        let scene=self.scene.as_ref().ok_or("map scene is still loading")?;
        for collection in scene.resident_collections() {
            for (arena_index,arena) in collection.arenas.iter().enumerate() {
                if let Some((texture_index,texture))=arena.textures.iter().enumerate().find(|(_,t)|t.guid==guid) {
                    let key=(false,collection.id,arena_index,texture_index);
                    if let Some(cached)=self.texture_cache.get(&key) {return Ok(cached.clone());}
                    let gpu=renderer.upload_native_texture(texture)?;
                    self.texture_cache.insert(key,gpu.clone());
                    return Ok(gpu);
                }
            }
        }
        Err(format!("original boot {name} is not resident").into())
    }
    /// The inspection scene starts from the selected spawn location's map.
    /// The caller retains the same mounted archives for the skating session.
    pub(crate) fn load(mounts: &MountSet) -> RenderResult<Self> {
        // Decode once and retain for the window lifetime. This selected inventory
        // admission is explicitly not native Presentation registration or streaming.
        let scene = crate::load_runtime_presentation_scene(&mounts, selected_spawn_location())?;
        let textures = crate::load_runtime_texture_scene(&mounts, selected_spawn_location())?;
        Self::from_assets(mounts, scene, textures, EnvironmentLighting::load(mounts)?)
    }
    /// Same material/texture/queue/upload owner for original nonstreamed worlds.
    pub(crate) fn from_assets(mounts: &MountSet, mut scene: PresentationScene,
        textures: TextureScene, environment: EnvironmentLighting) -> RenderResult<Self> {
        let texture_counts = scene.bind_resident_textures(&textures);
        eprintln!("original retained texture bindings: {texture_counts:?}");
        let mut channel_counts = BTreeMap::<String, [usize; 2]>::new();
        for channel in scene
            .resident_collections()
            .flat_map(|c| &c.arenas)
            .flat_map(|a| &a.materials)
            .flat_map(|m| &m.channels)
        {
            if let PresentationMaterialValue::Raster {
                resident_texture, ..
            } = channel.value
            {
                let counts = channel_counts
                    .entry(channel.shader_input.clone())
                    .or_default();
                counts[0] += 1;
                counts[1] += usize::from(resident_texture.is_some());
            }
        }
        eprintln!("original material channel [total, resident]: {channel_counts:?}");
        let (queue, diagnostic_techniques) = build_render_state(&scene, mounts)?;
        eprintln!(
            "original environment lighting: Vault g_vLightDir={:?} (sun-direction map requested={}, sampled per frame at the skater hips) g_envattributes[2]={:?}; WorldPainter blends over {} material_fog / {} material_sky collections, location fallback fog={:016x} sky={:016x}",
            environment.light_direction(),
            environment.sun_direction_map_requested,
            environment.exposure(),
            environment.fogs.len(),
            environment.skies.len(),
            environment.location_fog_key,
            environment.location_sky_key
        );
        let painter = WorldPainterBlend::new(&environment);
        let focus = scene.focus;
        // Settled defaults for any draw before the first `frame`; the retained
        // blend state itself is untouched until then.
        let frame_environment = {
            let mut preview = painter;
            preview.update(
                &environment,
                0.0,
                None,
                environment.light_direction_at(focus),
            )
        };
        let mut world_probes = crate::render_irradiance::WorldProbes::default();
        world_probes.sync(&scene);
        let emitter_waypoints = resident_emitter_waypoints(&scene);
        Ok(Self {
            world_probes,
            emitter_waypoints,
            scene: Some(scene),
            textures: Some(textures),
            streaming: None,
            hull_presentations: Vec::new(),
            additional_world: None,
            proxy_textures: None,
            queue,
            focus,
            distance: 45.0,
            keys: BTreeSet::new(),
            previous: Instant::now(),
            external_view: None,
            inspection_eye: None,
            sun_sample_position: None,
            shadow_inputs: None,
            frontend_csm: None,
            painter_keys: None,
            painter,
            painter_previous: None,
            painter_error_reported: false,
            frame_environment,
            animation_clock: Default::default(),
            scene_delta: None,
            simulation_tick: 0,
            simulation_fraction: 0.0,
            traffic_light_status: [[0.; 4]; 2],
            camera_axes: None,
            water_animation: Default::default(),
            diagnostic_techniques,
            environment: Arc::new(environment),
            render_location: None,
            loaded_environment: None,
            location_vaults: None,
            environment_resources: None,
            mesh_cache: BTreeMap::new(),
            texture_cache: BTreeMap::new(),
            material_cache: BTreeMap::new(),
            diagnostic_pipelines: BTreeMap::new(),
            pending_gpu: None,
            retire_gpu: None,
            media_focus:None,
        })
    }
    /// The live camera: clip-from-world, the `CameraMan` translation the
    /// original publishes as `g_vViewPos`, the WorldPainter keys the
    /// VisualDirector sampled at that camera, and the camera and local
    /// skater the sun map and the shadow cascades use.
    pub(crate) fn set_world_shader_state(&mut self, traffic: [[f32; 4]; 2], axes: [[f32; 3]; 3]) {
        self.traffic_light_status = traffic;
        self.camera_axes = Some(axes);
    }
    pub(crate) fn set_external_view(
        &mut self,
        clip: [f32; 16],
        view_position: [f32; 3],
        painter_keys: Result<WorldPainterKeys, String>,
        shadow_inputs: ShadowCasterInputs,
        (simulation_tick, fraction): (u32, f32),
    ) {
        self.external_view = Some((clip, view_position));
        self.painter_keys = Some(painter_keys);
        self.sun_sample_position = Some(shadow_inputs.skater_hips);
        self.shadow_inputs = Some(shadow_inputs);
        self.frontend_csm = None;
        self.simulation_tick = simulation_tick;
        self.simulation_fraction = fraction;
    }
    /// The retained Vault environment inputs (`m_params`, fog, sun).
    pub(crate) fn environment(&self) -> &Arc<EnvironmentLighting> {
        &self.environment
    }
    /// `RenderMan::UpdateRenderParams` 0x82852780 each frame: a world render location other than
    /// the one shown (`GetWorldRenderLocation`, the session's `cMsgSetWorldRenderLocation` key)
    /// changes `m_RenderLocation`, read through every vault the game has loaded.
    pub(crate) fn set_world_render_location(&mut self, mounts: &MountSet, location: Option<u64>) -> RenderResult<()> {
        if self.render_location == location {
            return Ok(());
        }
        let loaded = Arc::clone(self.loaded_environment.get_or_insert_with(|| Arc::clone(&self.environment)));
        let vaults = skate2_core::dlc::vaults(mounts);
        if location.is_some() && self.location_vaults.as_ref().is_none_or(|(read, ..)| *read != vaults) {
            let db = crate::vault_database(mounts)?;
            let extended = Arc::new(loaded.with_vault_collections(mounts, &db)?);
            self.location_vaults = Some((vaults, db, extended));
        }
        let base = self.location_vaults.as_ref().map_or(loaded, |(.., extended)| Arc::clone(extended));
        let located = match (location, self.location_vaults.as_ref()) {
            (Some(key), Some((_, db, _))) => base.with_world_render_location(db, key)?.map(Arc::new),
            _ => None,
        };
        eprintln!("render location: {}", location.map_or_else(|| "default".to_owned(), |key| format!(
            "{key:016x}{}", if located.is_some() { "" } else { " (no render_locations collection: the default)" })));
        self.environment = located.unwrap_or(base);
        self.render_location = location;
        Ok(())
    }
    pub(crate) fn enable_city_streaming(
        &mut self,
        mounts: std::sync::Arc<MountSet>,
        catalog: skate2_core::world_stream_catalog::WorldStreamCatalog,
        proxy: crate::proxy_world::SharedProxyWorld,
    ) {
        self.streaming = Some(crate::city_streaming::CityStreaming::new(
            mounts, catalog, proxy,
        ));
    }
    /// The session's registered hull presentation collections for the next city update.
    pub(crate) fn set_hull_presentations(&mut self, hulls: Vec<skate2_core::presentation::HullPresentation>) {
        self.hull_presentations = hulls;
    }
    /// The additional world the session holds loaded, whose files the next city update selects
    /// from.
    pub(crate) fn set_additional_world(&mut self, world: Option<String>) {
        self.additional_world = world;
    }
    /// The additional world the city streaming may still hold collections of.
    pub(crate) fn additional_world_held(&self) -> Option<String> {
        self.streaming.as_ref()?.additional_world().map(str::to_owned)
    }
    /// The additional world the city streaming selects from.
    pub(crate) fn additional_world_selected(&self) -> Option<String> {
        self.streaming.as_ref()?.selected_world().map(str::to_owned)
    }
    /// The presentation stream for the session's `IsAreaLoaded`; none while a streaming
    /// generation holds the scene.
    pub(crate) fn presentation_stream_area(&self) -> Option<skate2_core::streaming::area_loaded::PresentationStreamArea> {
        self.streaming.as_ref()?.presentation_area(self.scene.as_ref()?)
    }
    pub(crate) fn replay_stream_focuses(&self)->Option<[skate2_core::streaming::replay_focus::VisualFocus;2]> {
        self.streaming.as_ref()?.replay_focuses(self.scene.as_ref()?,self.textures.as_ref()?)
    }
    /// A saved frame must use the requested map generation, including its
    /// completed CPU load and GPU upload, rather than the previous live draw.
    pub(crate) fn media_scene_ready(&self)->bool {
        self.pending_gpu.is_none()&&self.streaming.as_ref().is_none_or(|s|!s.is_pending())
            &&self.scene.as_ref().is_some_and(|s|self.media_focus.is_none_or(|p|s.focus==p))
            &&self.textures.as_ref().is_some_and(|s|self.media_focus.is_none_or(|p|s.focus==p))
    }
    pub(crate) fn update_city_streaming(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        visual: &skate2_core::SelectedVisualSnapshot<'_>,
    ) -> RenderResult<()> {
        self.media_focus=(!visual.stream_focus_disabled).then_some(visual.stream_focus_position);
        self.update_city_streaming_at(renderer, scene, visual.stream_focus_position,
            visual.stream_focus_disabled, visual.stream_xml_enabled)
    }
    /// Loose-world CAS retains location lighting and the camera's FE cascades.
    pub(crate) fn set_loose_world_view(&mut self, clip: [f32;16], position: [f32;3], axes: [[f32;3];3],
        caster: ShadowCasterInputs, csm: crate::render_shadows::CsmAttributes) {
        self.external_view=Some((clip,position));
        self.camera_axes=Some(axes);
        self.sun_sample_position=Some(position);
        self.shadow_inputs=Some(caster);
        self.frontend_csm=Some(csm);
          self.painter_keys=None;
      }
    /// `RenderShadows_CSM` 0x82855888 in the front end's scene: its cascades and the world pass
    /// (82856d54..828576bc, ungated) from the scene camera, over the presentation's local
    /// skater, as in any NIS without ICE shadow channels (`m_NISooScale`/`m_NISooDirOfset`
    /// keep the `m_CSMAttribs` values `Shadows_CSM_Init` 0x8283ee34/eef8 copied).
    pub(crate) fn set_frontend_shadow_casters(&mut self,casters:ShadowCasterInputs){
        self.shadow_inputs=Some(casters);
    }
    pub(crate) fn set_frontend_world_view(&mut self,clip:[f32;16],position:[f32;3],axes:[[f32;3];3],scene_delta:f32){
        self.scene_delta=Some(scene_delta);
        self.external_view=Some((clip,position));
        self.camera_axes=Some(axes);
        self.sun_sample_position=Some(position);
        self.shadow_inputs=None;
        self.frontend_csm=None;
        self.painter_keys=None;
    }

    /// Reuse the retained city CPU/GPU loader without changing the view camera.
    pub(crate) fn prepare_teleport(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene,
        position: [f32; 3], disabled: bool, xml_enabled: bool,
    ) -> RenderResult<bool> {
        if disabled { return Err("Map travel cannot move a disabled stream focus".into()); }
        self.update_city_streaming_at(renderer, scene, position, disabled, xml_enabled)?;
        // An additional world's location can select no texture collection: its Pres files carry
        // its textures.
        Ok(self.pending_gpu.is_none()
            && self.streaming.as_ref().is_some_and(|stream| !stream.is_pending())
            && self.scene.as_ref().is_some_and(|scene| scene.focus == position && !scene.collections.is_empty())
            && self.textures.as_ref().is_some_and(|textures| textures.focus == position
                && (!textures.collections.is_empty()
                    || self.streaming.as_ref().is_some_and(|stream| stream.texture_selection_empty(position, xml_enabled)))))
    }

    fn update_city_streaming_at(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene,
        position: [f32; 3], disabled: bool, xml_enabled: bool,
    ) -> RenderResult<()> {
        if let Some(pending) = &self.pending_gpu {
            match pending.try_recv() {
                Ok(result) => {
                    self.pending_gpu = None;
                    let (build, replacement) = result?;
                    let Some(current) = scene.map.replace(replacement) else {
                        return Err("city swap lost current map objects".into());
                    };
                    let retired = RetiredMap {
                        map: current,
                        queue: std::mem::take(&mut self.queue),
                        techniques: std::mem::take(&mut self.diagnostic_techniques),
                    };
                    self.install_map_build(build);
                    // Destruction of thousands of old bindings also belongs off-frame.
                    if let Some(retire) = self.retire_gpu.take() {
                        retire
                            .send(retired)
                            .map_err(|_| "city GPU retirement worker stopped")?;
                    }
                    return Ok(());
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(()),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err("city GPU builder stopped without completion".into());
                }
            }
        }
        let Some(streaming) = self.streaming.as_mut() else {
            return Ok(());
        };
        let Some((queue, techniques)) = streaming.update(
            &mut self.scene,
            &mut self.textures,
            position,
            disabled,
            xml_enabled,
            &self.hull_presentations,
            self.additional_world.as_deref(),
        )?
        else {
            return Ok(());
        };
        let mut build = self.take_map_build(renderer, queue, techniques)?;
        let gpu = renderer.resource_builder();
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        let (retire, retired) = std::sync::mpsc::channel::<RetiredMap>();
        std::thread::Builder::new()
            .name("city-gpu-builder".into())
            .spawn(move || {
                let result = build
                    .build(&gpu)
                    .map(|map| (build, map))
                    .map_err(|error| error.to_string());
                if send.send(result).is_ok() {
                    // Channel closes on shutdown/error, so this worker cannot outlive its owner.
                    if let Ok(old) = retired.recv() {
                        drop(old);
                    }
                }
            })?;
        self.pending_gpu = Some(receive);
        self.retire_gpu = Some(retire);
        Ok(())
    }
}
impl RenderWindowClient for Inspection {
    fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<GpuScene> {
        if self.proxy_textures.is_none() {
            if let Some(streaming) = &mut self.streaming {
                let scene = self.scene.as_mut().ok_or("map scene is still loading")?;
                self.proxy_textures =
                    Some(streaming.activate_proxy_world(renderer.texture_uploader(), scene)?);
                // The first draw list includes the registered proxies.
                (self.queue, self.diagnostic_techniques) =
                    build_render_state(scene, &streaming.mounts)?;
            }
        }
        let queue = std::mem::take(&mut self.queue);
        let techniques = std::mem::take(&mut self.diagnostic_techniques);
        let mut build = self.take_map_build(renderer, queue, techniques)?;
        let map = build.build(&renderer.resource_builder())?;
        self.install_map_build(build);
        Ok(GpuScene {
            stereoscopy: false,
            stereo_planes: None,
            career_photo: None,
            replay_media: None,
            capture_watermark: None,
            postfx: None,
            world_ui:None,
            particles: None,
            crowd: Default::default(),
            deck_ao: None,
            visual_indicators: None,
            online_names: None,
            chase_arrows: None,
            marker_ui: None,
            fps_ui: None,
            frontend_fade: None,
            frontend_fade_alpha: 0.,
            nis_white: None,
            nis_white_alpha: 0.,
            movie: None,
            final_blit: None,
            postfx_overlay: None,
            out_of_boundary_amount: None,
            map: Some(map),
            objects: Vec::new(),
            dynamic_objects: Vec::new(),
            tick_due: None,
            unchanged: false,
        })
    }
    fn window_event(&mut self, event: &WindowEvent) -> RenderResult<()> {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if let PhysicalKey::Code(code) = event.physical_key {
                if event.state == ElementState::Pressed {
                    self.keys.insert(code);
                } else {
                    self.keys.remove(&code);
                }
            }
        }
        if matches!(event, WindowEvent::Focused(false)) {
            self.keys.clear();
        }
        Ok(())
    }
    fn frame(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
    ) -> RenderResult<()> {
        match self.scene_delta.take() {
            Some(delta) => self.animation_clock.advance_scene(delta),
            None => self.animation_clock.update(self.simulation_tick, self.simulation_fraction),
        };
        self.water_animation
            .update(self.animation_clock.last_delta());
        let (clip, view_position) = if let Some(view) = self.external_view {
            view
        } else {
            // Inspection camera timing only; never supplies a simulation dt.
            let step = now.duration_since(self.previous).as_secs_f32().min(0.1) * 20.0;
            self.previous = now;
            for (key, axis, sign) in [
                (KeyCode::ArrowLeft, 0, -1.0),
                (KeyCode::ArrowRight, 0, 1.0),
                (KeyCode::ArrowUp, 2, -1.0),
                (KeyCode::ArrowDown, 2, 1.0),
            ] {
                if self.keys.contains(&key) {
                    self.focus[axis] += sign * step;
                }
            }
            if self.keys.contains(&KeyCode::PageUp) {
                self.distance = (self.distance - step).max(2.0);
            }
            if self.keys.contains(&KeyCode::PageDown) {
                self.distance += step;
            }
            self.camera(renderer.dimensions())
        };
        // `UpdateRenderParams` dT: presentation frame time, not a sim step.
        let dt = self
            .painter_previous
            .map_or(0.0, |previous| now.duration_since(previous).as_secs_f32());
        self.painter_previous = Some(now);
        let keys = match &self.painter_keys {
            Some(Ok(keys)) => Some(keys),
            Some(Err(error)) => {
                if !self.painter_error_reported {
                    self.painter_error_reported = true;
                    eprintln!(
                        "WorldPainter key lookup failed ({error}); using the render location's fog/sky references"
                    );
                }
                None
            }
            None => None,
        };
        // UpdateWorldPainterAttribs 0x82848c08: the WorldPainter keys only with the location's
        // use_worldpainter, else its references.
        let keys = keys.filter(|_| self.environment.use_worldpainter);
        let light_direction = self
            .environment
            .light_direction_at(self.sun_sample_position.unwrap_or(view_position));
        // `RenderShadows_CSM` 0x82855888 runs right after `UpdateRenderParams`.
        let shadows = self
            .shadow_inputs
            .as_ref()
            .map_or_else(Default::default, |inputs| {
                if let Some(attributes) = &self.frontend_csm {
                    crate::render_shadows::frontend_cascades(light_direction, attributes, inputs)
                } else {
                    crate::render_shadows::cascades(light_direction, &self.environment.csm, inputs)
                }
            });
        self.frame_environment = FrameEnvironment {
            exposure: renderer.postfx_exposure(),
            scene_cull: Some(self.environment.scene_cull),
            shadows,
            animation_time: [self.animation_clock.seconds(), 0., 0., 0.],
            pca: self.water_animation.globals(),
            traffic_lights: self.traffic_light_status,
            camera_axes: self
                .camera_axes
                .unwrap_or_else(|| {
                    let z = normalize(std::array::from_fn(|i| view_position[i] - self.focus[i]));
                    let x = normalize([z[2], 0., -z[0]]);
                    let y = [
                        z[1] * x[2] - z[2] * x[1],
                        z[2] * x[0] - z[0] * x[2],
                        z[0] * x[1] - z[1] * x[0],
                    ];
                    [x, y, z]
                })
                .map(|v| [v[0], v[1], v[2], 0.]),
            view_dot_light: self.environment.view_dot_light(
                // UpdateRenderParams82852870/82852d94 uses the actual camera
                // Z axis, independently of whether a shadow caster exists.
                // The frontend loading focus equals its final camera position:
                // subtracting those positions normalized zero and poisoned
                // foliage lighting. CAS and ICE already publish their axes.
                self.camera_axes.map_or_else(
                    || self.shadow_inputs.as_ref().map_or_else(
                        || normalize(std::array::from_fn(|i| view_position[i] - self.focus[i])),
                        |inputs| inputs.camera_forward,
                    ),
                    |axes| axes[2],
                ),
            ),
            world_shadow: self
                .shadow_inputs
                .as_ref()
                .filter(|_| self.environment.world_shadow.enabled)
                .map(|inputs| {
                    crate::render_shadows::WorldShadowProjection::new(
                        light_direction,
                        &self.environment.world_shadow,
                        inputs,
                    )
                }),
            ..self
                .painter
                .update(&self.environment, dt, keys, light_direction)
        };
        let postfx=self.painter.postfx(&self.environment,dt,keys,now);
        scene.postfx=Some((self.environment.postfx.clone(),postfx));
        // Object transforms and colors are fixed per generation; camera and
        // WorldPainter blends change, uploaded once per frame.
        scene
            .map
            .as_mut()
            .ok_or("map draw list not initialized")?
            .set_view(clip, view_position, self.frame_environment);
        Ok(())
    }
}
impl Inspection {
    /// Inspection clip-from-world and its eye position.
    fn camera(&self, (width, height): (u32, u32)) -> ([f32; 16], [f32; 3]) {
        let eye = self.inspection_eye.unwrap_or([
            self.focus[0] + self.distance * 0.7,
            self.focus[1] + self.distance * 0.5,
            self.focus[2] + self.distance * 0.7,
        ]);
        let z = normalize(std::array::from_fn(|i| eye[i] - self.focus[i]));
        let x = normalize([z[2], 0.0, -z[0]]);
        let y = [
            z[1] * x[2] - z[2] * x[1],
            z[2] * x[0] - z[0] * x[2],
            z[0] * x[1] - z[1] * x[0],
        ];
        let view = [
            x[0],
            y[0],
            z[0],
            0.0,
            x[1],
            y[1],
            z[1],
            0.0,
            x[2],
            y[2],
            z[2],
            0.0,
            -dot(x, eye),
            -dot(y, eye),
            -dot(z, eye),
            1.0,
        ];
        // Pixel dimensions become a diagnostic floating-point projection ratio.
        #[expect(
            clippy::cast_precision_loss,
            reason = "GPU projection uses f32 pixel dimensions"
        )]
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let scale = 1.0 / (60.0_f32.to_radians() * 0.5).tan();
        let (near, far) = (0.1, 2000.0);
        let projection = [
            scale / aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            scale,
            0.0,
            0.0,
            0.0,
            0.0,
            far / (near - far),
            -1.0,
            0.0,
            0.0,
            near * far / (near - far),
            0.0,
        ];
        (multiply(projection, view), eye)
    }
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    v.map(|x| x / length)
}
fn multiply(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    std::array::from_fn(|i| {
        let row = i % 4;
        let column = i / 4;
        (0..4).map(|k| a[k * 4 + row] * b[column * 4 + k]).sum()
    })
}
fn color(index: usize) -> [f32; 4] {
    let palette = [
        [0.48, 0.55, 0.62, 1.0],
        [0.65, 0.59, 0.48, 1.0],
        [0.38, 0.49, 0.42, 1.0],
        [0.62, 0.48, 0.43, 1.0],
    ];
    palette[index % palette.len()]
}
/// None when the effect's vertex shader already consumes world-space positions.
fn world_from_mesh(submission: &skate2_core::render::NativeRenderSubmission) -> Option<[f32; 16]> {
    if WORLD_SPACE_EFFECTS.contains(&submission.manifest_name.as_str()) {
        None
    } else {
        Some(submission.transform)
    }
}

/// Effects whose shipped `defaultVS` microcode (render.big `.vpo`) writes
/// oPos as four dp4 of the fetched position against g_matVP (c0-c3), with no
/// instance matrix: the instance transform never reaches their vertices.
/// treeanimate and vertexanimate add a world-space wave offset first (`P =
/// In.pos + move`), which remains unimplemented. Every other effect keeps
/// its staged instance-space inspection.
const WORLD_SPACE_EFFECTS: [&str; 29] = [
    "advertisement",
    "alphatestdefaultenvironment",
    "baseenvironment",
    "baseincandescent",
    "baseterrain",
    "building_ceiling_tangentspace",
    "building_ceiling_worldspace",
    "building_cuberoom_tangentspace",
    "decalenvironment",
    "decalterrain",
    "decalzbias",
    "decalzbiasbacklit",
    "defaultenvironment",
    "flowingwater",
    "ocean",
    "proxyworld",
    "scrollincandescent",
    "trafficlight_one",
    "trafficlight_two",
    "transparentdefaultenvironment",
    "transparentenvironment",
    "transparentincandescent",
    "tree",
    "treeanimate",
    "vertexanimate",
    "videoscreen",
    "water",
    "zbiasincandescent",
    "zbiasincandescentbacklit",
];

/// What a diagnostic draw reproduces of one manifest technique: the source
/// diffuse-alpha test (reviewed programs only) and its pass rasterizer state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DiagnosticTechnique {
    pub(crate) alpha_reference: Option<u8>,
    pub(crate) raster: RasterState,
}

/// Every loaded (manifest, technique) pair's diagnostic state.
pub(crate) type DiagnosticTechniques = BTreeMap<(String, String), DiagnosticTechnique>;

pub(crate) fn build_render_state(
    scene: &PresentationScene,
    mounts: &MountSet,
) -> RenderResult<(NativeRenderQueue, DiagnosticTechniques)> {
    let names = scene
        .resident_collections()
        .flat_map(|c| &c.arenas)
        .flat_map(|a| &a.materials)
        .filter_map(|m| m.effect.as_ref())
        .filter_map(|e| e.binding.as_ref())
        .map(|b| b.manifest_name.clone())
        .collect::<BTreeSet<_>>();
    let manifests = crate::load_native_shader_manifests(&mounts, &names)?;
    let queue = NativeRenderQueue::build(&scene, &manifests, &mut RenderScopeRegistry::default())?;
    let mut techniques = DiagnosticTechniques::new();
    for (name, manifest) in &manifests {
        for technique in &manifest.techniques {
            let mut passes = Vec::with_capacity(technique.passes.len());
            for (index, pass) in technique.passes.iter().enumerate() {
                let states = pass
                    .states
                    .iter()
                    .map(|s| (s.name.as_str(), s.value.as_str()));
                passes.push(RasterState::from_pass_states(states).ok_or_else(|| {
                    format!(
                        "{name}/{} pass {index}: rasterizer state the original parser rejects",
                        technique.name
                    )
                })?);
            }
            // One diagnostic draw cannot reproduce passes that differ (hair,
            // visual indicators); those keep the per-pass default state.
            let raster = match passes.split_first() {
                Some((first, rest)) if rest.iter().all(|pass| pass == first) => *first,
                _ => RasterState::default(),
            };
            // These three original UPDB pixel shaders explicitly restore
            // diffuse.a after lighting/fog/tone mapping. Do not assume other
            // shaders do so.
            let mut alpha_reference = None;
            if matches!(
                name.as_str(),
                "tree" | "treeanimate" | "alphatestdefaultenvironment"
            ) && technique.passes.len() == 1
            {
                let pass = &technique.passes[0];
                let state = |name: &str| {
                    pass.states
                        .iter()
                        .find(|s| s.name == name)
                        .map(|s| s.value.as_str())
                };
                if state("ALPHATESTENABLE") == Some("true")
                    && state("ALPHAFUNC") == Some("GREATEREQUAL")
                    && state("ALPHABLEND") == Some("false")
                {
                    alpha_reference = Some(
                        state("ALPHAREF")
                            .ok_or("alpha-tested material lacks ALPHAREF")?
                            .parse::<u8>()?,
                    );
                }
            }
            techniques.insert(
                (name.clone(), technique.name.clone()),
                DiagnosticTechnique {
                    alpha_reference,
                    raster,
                },
            );
        }
    }
    Ok((queue, techniques))
}

/// Stable texture identity across generations: archive collection identity
/// for retained textures, asset GUID for PROXY textures.
// Fields only order and compare material cache keys.
#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum TextureIdentity {
    Resident(bool, u64, usize, usize),
    Proxy(u64),
    /// `PostFX`'s main render target.
    MainTarget,
    /// A stand-in for a channel the material leaves out ([`TextureResolver::absent`]).
    Absent(&'static str),
    /// A resident 2D texture uploaded as a cube ([`TextureResolver::as_cube`]).
    ResidentCube(bool, u64, usize, usize),
}

/// One original-effect material sampler set, by effect and channel textures.
type MaterialKey = (EnvironmentEffect, Vec<TextureIdentity>);

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum TextureSource {
    Proxy(u64),
    Presentation(ResidentTextureHandle),
    High(ResidentTextureHandle),
}

/// `AddPegasusMaterialToScope` 0x82e17bb0 `LightMapDot`: a channel whose name
/// starts with "li" binds `i_monoLightmap_Dot` to entry `image_channel & 3`.
const LIGHTMAP_DOT: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.33, 0.34, 0.33, 0.0],
];

/// Resolves a material channel GUID through the shader texture table's
/// active slot to one uploaded texture; each retained texture uploads once.
struct TextureResolver<'a> {
    renderer: &'a GpuResourceBuilder,
    scene: &'a PresentationScene,
    textures: &'a TextureScene,
    table: ShaderTextureTable<TextureSource>,
    proxy: &'a BTreeMap<u64, GpuTexture>,
    cache: &'a mut BTreeMap<(bool, u64, usize, usize), GpuTexture>,
    uploaded: BTreeMap<TextureSource, Result<(TextureIdentity, GpuTexture), String>>,
    used: BTreeSet<(bool, u64, usize, usize)>,
    uploads: usize,
    absent: BTreeMap<&'static str, GpuTexture>,
    cubes: BTreeMap<TextureSource, (TextureIdentity, GpuTexture)>,
}

impl TextureResolver<'_> {
    /// A sampled channel the material leaves out (some downloadable content world and city
    /// surfaces have no normal or specular map). The original binds nothing for it
    /// (`DispatchParamsHelper` skips a null `cTextureParam`), so the sampler keeps whatever
    /// texture an earlier draw left there and the surface is still lit and shadowed. The port
    /// stands in a flat normal map or an unlit specular mask (PC); any other absent channel
    /// keeps the diagnostic path.
    /// A 2D texture where the effect samples a cube. Some downloadable content materials bind a
    /// plain texture to `environment` (DLC_School's `Suburbs_roadway_crosswalk`: the 64x64
    /// `arrow` texture). The original has no type check: its cube fetch reads the 2D fetch
    /// constant, the same image for every face, and the surface stays lit and shadowed. The
    /// port uploads the square image as all six faces (PC); a non-square one stays refused.
    fn as_cube(&mut self, guid: u64) -> RenderResult<Result<(TextureIdentity, GpuTexture), String>> {
        let Some(source) = self
            .table
            .find(guid)
            .and_then(|(_, slot)| slot.active)
            .or_else(|| {
                self.table
                    .find(skate2_formats::hash::rw_hash64_string("missingtexture.Texture"))
                    .and_then(|(_, slot)| slot.active)
            })
        else {
            return Ok(Err(format!("GUID {guid:016x} absent")));
        };
        if let Some(found) = self.cubes.get(&source) {
            return Ok(Ok(found.clone()));
        }
        let (high, handle) = match source {
            TextureSource::Proxy(_) => return Ok(Err("PROXY texture bound as a cube".to_owned())),
            TextureSource::Presentation(handle) => (false, handle),
            TextureSource::High(handle) => (true, handle),
        };
        let texture = if high {
            self.textures.texture(handle)
        } else {
            self.scene
                .resident_collection(handle.collection_index)
                .and_then(|c| c.arenas.get(handle.arena_index))
                .and_then(|a| a.textures.get(handle.texture_index))
        }
        .ok_or("bound texture handle lost its retained owner")?;
        if texture.width != texture.height {
            return Ok(Err(format!("{}x{} texture bound as a cube", texture.width, texture.height)));
        }
        let collection_id = if high {
            self.textures.collections[handle.collection_index].id
        } else {
            self.scene
                .resident_collection(handle.collection_index)
                .ok_or("texture collection owner absent")?
                .id
        };
        let mut cube = texture.clone();
        cube.texture_type = skate2_formats::material::TextureType::Cube;
        cube.linear_levels = texture
            .linear_levels
            .iter()
            .flat_map(|level| {
                (0..6).map(move |face| skate2_formats::material::LinearTextureLevel { face, ..level.clone() })
            })
            .collect();
        let identity = TextureIdentity::ResidentCube(high, collection_id, handle.arena_index, handle.texture_index);
        let result = self
            .renderer
            .upload_native_texture(&cube)
            .map(|gpu| (identity, gpu))
            .map_err(|error| error.to_string());
        if let Ok(pair) = &result {
            self.cubes.insert(source, pair.clone());
        }
        Ok(result)
    }

    fn absent(&mut self, name: &str) -> RenderResult<Option<(TextureIdentity, GpuTexture)>> {
        let (key, bgra): (&'static str, [u8; 4]) = match name {
            "normal" | "normal2" => ("normal", [255, 128, 128, 255]),
            "specular" | "specular2" => ("specular", [0, 0, 0, 255]),
            _ => return Ok(None),
        };
        if !self.absent.contains_key(key) {
            let texture = self.renderer.upload_native_texture(&solid_texture(key, bgra))?;
            self.absent.insert(key, texture);
        }
        Ok(Some((TextureIdentity::Absent(key), self.absent[key].clone())))
    }

    /// Outer error: retained ownership broke. Inner error: why this channel
    /// cannot be decoded. A missing referenced GUID uses the original boot
    /// asset (TextureMan::AddMaterial8282c838). An entirely absent channel is
    /// different: DispatchParamsHelper skips a null cTextureParam.
    fn resolve(
        &mut self,
        guid: u64,
    ) -> RenderResult<Result<(TextureIdentity, GpuTexture), String>> {
        let source = self
            .table
            .find(guid)
            .and_then(|(_, slot)| slot.active)
            .or_else(|| {
                self.table
                    .find(skate2_formats::hash::rw_hash64_string(
                        "missingtexture.Texture",
                    ))
                    .and_then(|(_, slot)| slot.active)
            })
            .ok_or_else(|| {
                format!("GUID {guid:016x} absent and boot missingtexture.Texture is not retained")
            })?;
        let (high, handle) = match source {
            TextureSource::Proxy(guid) => {
                return Ok(self
                    .proxy
                    .get(&guid)
                    .map(|texture| (TextureIdentity::Proxy(guid), texture.clone()))
                    .ok_or_else(|| "PROXY texture lost retained owner".to_owned()));
            }
            TextureSource::Presentation(handle) => (false, handle),
            TextureSource::High(handle) => (true, handle),
        };
        if let Some(result) = self.uploaded.get(&source) {
            return Ok(result.clone());
        }
        let texture = if high {
            self.textures.texture(handle)
        } else {
            self.scene
                .resident_collection(handle.collection_index)
                .and_then(|c| c.arenas.get(handle.arena_index))
                .and_then(|a| a.textures.get(handle.texture_index))
        }
        .ok_or("bound texture handle lost its retained owner")?;
        let collection_id = if high {
            self.textures.collections[handle.collection_index].id
        } else {
            self.scene
                .resident_collection(handle.collection_index)
                .ok_or("texture collection owner absent")?
                .id
        };
        let key = (
            high,
            collection_id,
            handle.arena_index,
            handle.texture_index,
        );
        self.used.insert(key);
        let identity = TextureIdentity::Resident(key.0, key.1, key.2, key.3);
        let result = if let Some(cached) = self.cache.get(&key) {
            Ok((identity, cached.clone()))
        } else {
            self.uploads += 1;
            self.renderer
                .upload_native_texture(texture)
                .map(|gpu| {
                    self.cache.insert(key, gpu.clone());
                    (identity, gpu)
                })
                .map_err(|error| error.to_string())
        };
        self.uploaded.insert(source, result.clone());
        Ok(result)
    }
}

/// A 1x1 A8R8G8B8 texture of one colour, in the uploader's BGRA byte order.
fn solid_texture(name: &str, bgra: [u8; 4]) -> skate2_core::texture::NativeTexture {
    skate2_core::texture::NativeTexture {
        dictionary_index: 0,
        guid: 0,
        name: format!("absent {name}"),
        texture_type: skate2_formats::material::TextureType::TwoDimensional,
        endian: skate2_formats::material::TextureEndian::None,
        width: 1,
        height: 1,
        depth: 1,
        mip_levels: 1,
        pixel_format: skate2_formats::material::PIXEL_FORMAT_A8R8G8B8,
        packed_mips: false,
        mip_data_offset: 0,
        encoded_data: bgra.to_vec(),
        linear_levels: vec![skate2_formats::material::LinearTextureLevel {
            level: 0,
            face: 0,
            width: 1,
            height: 1,
            width_in_blocks: 1,
            height_in_blocks: 1,
            bytes_per_block: 4,
            row_pitch: 4,
            data: bgra.to_vec(),
        }],
    }
}

/// Per-generation original-effect material bindings, reused across
/// generations by texture identity.
struct MaterialBindings<'a> {
    renderer: &'a GpuResourceBuilder,
    cache: &'a mut BTreeMap<MaterialKey, Arc<wgpu::BindGroup>>,
    used: BTreeSet<MaterialKey>,
}

/// The original effect's full channel set, instance scalars and `m_params`
/// for one material, following `AddPegasusMaterialToScope` 0x82e17bb0
/// (channels from index 2; flag-bit-1 scalars bound as `i_<name>`) and
/// `cMaterialScope::Resolve` 0x82e17a98. Inner Err when any input the effect
/// samples is absent: that draw keeps the diagnostic path instead of the
/// original's stale previous-draw texture.
fn environment_material(
    effect: EnvironmentEffect,
    material: &PresentationMaterial,
    main_target: Option<&GpuTexture>,
    lighting: &EnvironmentLighting,
    resolver: &mut TextureResolver<'_>,
    bindings: &mut MaterialBindings<'_>,
) -> RenderResult<Result<(Arc<wgpu::BindGroup>, InstanceMaterial), String>> {
    let channels = material.channels.get(2..).unwrap_or_default();
    // Each channel binds its hashed name in order, so a repeated name's last
    // occurrence is the one the shader sees.
    let find = |name: &str| channels.iter().rev().find(|c| c.shader_input == name);
    let mut identities = Vec::with_capacity(effect.channels().len());
    let mut textures = Vec::with_capacity(effect.channels().len());
    for &name in effect.channels() {
        // cModelInstance::Resolve 0x82853f90..0x82854028 sets a videoscreen_default scope's
        // i_diffuse after the material's channels: the main target, whatever the diffuse binds.
        if name == "diffuse"
            && let Some(texture) = main_target
        {
            identities.push(TextureIdentity::MainTarget);
            textures.push(texture.clone());
            continue;
        }
        let guid = match find(name).map(|c| &c.value) {
            Some(PresentationMaterialValue::Raster { guid, .. }) => *guid,
            Some(PresentationMaterialValue::ScalarConstant(_)) => {
                return Ok(Err(format!("{effect:?}: {name} is a scalar")));
            }
            None => {
                let Some((identity, texture)) = resolver.absent(name)? else {
                    return Ok(Err(format!("{effect:?}: no {name} channel")));
                };
                identities.push(identity);
                textures.push(texture);
                continue;
            }
        };
        match resolver.resolve(guid)? {
            Ok((identity, texture)) => {
                let (identity, texture) = if name == "environment" && !texture.is_cube() {
                    match resolver.as_cube(guid)? {
                        Ok(cube) => cube,
                        Err(reason) => return Ok(Err(format!("{effect:?}: {name} {reason}"))),
                    }
                } else {
                    (identity, texture)
                };
                identities.push(identity);
                textures.push(texture);
            }
            Err(reason) => return Ok(Err(format!("{effect:?}: {name} {reason}"))),
        }
    }
    let class_variant = material
        .effect
        .as_ref()
        .and_then(|e| e.attribute_class_id.zip(e.material_variant_id));
    if effect == EnvironmentEffect::Sky {
        let Some((params, blended)) =
            class_variant.and_then(|(class, variant)| lighting.sky_material_params(class, variant))
        else {
            return Ok(Err(format!("{effect:?}: m_params[0..2] unresolved")));
        };
        return Ok(material_binding(effect, identities, &textures, bindings)?
            .map(|binding| (binding, InstanceMaterial::sky(params, blended))));
    }
    // proxyworld.fx reads `i_diffuse` and `m_params[0]` alone.
    if matches!(
        effect,
        EnvironmentEffect::Glare
            | EnvironmentEffect::GodraySpotlight
            | EnvironmentEffect::TrafficLightOne
            | EnvironmentEffect::TrafficLightTwo
    ) {
        let mut instance = InstanceMaterial::default();
        if effect == EnvironmentEffect::Glare {
            let Some(PresentationMaterialValue::ScalarConstant(scale)) =
                find("glareScale").map(|c| &c.value)
            else {
                return Ok(Err("Glare: glareScale absent".into()));
            };
            instance.macro_overlay_uv_scale = *scale;
        } else if effect != EnvironmentEffect::GodraySpotlight {
            let Some(params) =
                class_variant.and_then(|(class, variant)| lighting.material_params(class, variant))
            else {
                return Ok(Err(format!("{effect:?}: m_params unresolved")));
            };
            instance.m_params = params;
        }
        return Ok(material_binding(effect, identities, &textures, bindings)?
            .map(|binding| (binding, instance)));
    }
    if effect == EnvironmentEffect::ProxyWorld {
        let Some(m_params) =
            class_variant.and_then(|(class, variant)| lighting.material_params(class, variant))
        else {
            return Ok(Err(format!("{effect:?}: m_params unresolved")));
        };
        let instance = InstanceMaterial {
            m_params,
            ..InstanceMaterial::default()
        };
        return Ok(material_binding(effect, identities, &textures, bindings)?
            .map(|binding| (binding, instance)));
    }
    let Some(lightmap_dot) = channels
        .iter()
        .rev()
        .find(|c| c.shader_input.starts_with("li"))
        .map(|c| LIGHTMAP_DOT[usize::from(c.image_channel & 3)])
    else {
        return Ok(Err(format!("{effect:?}: no lightmap selector")));
    };
    let mut scalars = [[0.0; 4]; 3];
    for (value, &name) in scalars.iter_mut().zip(effect.scalars()) {
        match find(name).map(|c| &c.value) {
            Some(PresentationMaterialValue::ScalarConstant(lanes)) => *value = *lanes,
            _ => return Ok(Err(format!("{effect:?}: no {name} scalar"))),
        }
    }
    let Some(m_params) =
        class_variant.and_then(|(class, variant)| lighting.material_params(class, variant))
    else {
        return Ok(Err(format!("{effect:?}: m_params unresolved")));
    };
    if matches!(
        effect,
        EnvironmentEffect::TreeAnimate | EnvironmentEffect::VertexAnimate
    ) {
        let Some(params) = class_variant
            .and_then(|(class, variant)| lighting.material_parameter_array(class, variant))
            .filter(|params| params.len() >= 3)
        else {
            return Ok(Err(format!("{effect:?}: m_params[0..3] unresolved")));
        };
        // These two instance lanes are unused scalar channels in treeanimate.
        scalars[0] = params[1];
        scalars[1] = params[2];
    }
    if matches!(
        effect,
        EnvironmentEffect::BaseTerrain | EnvironmentEffect::DecalTerrain
    ) {
        let Some(params) = class_variant
            .and_then(|(class, variant)| lighting.material_parameter_array(class, variant))
            .filter(|params| params.len() >= 2)
        else {
            return Ok(Err(format!("{effect:?}: m_params[0..2] unresolved")));
        };
        let [overlay_scale, opacity, second_scale] = scalars;
        // The final terrain shaders have no detail-normal scalar use. Reuse
        // that otherwise unused instance vector for the authored blend powers.
        scalars = [
            [overlay_scale[0], second_scale[0], 0., 0.],
            params[1],
            opacity,
        ];
    }
    if matches!(
        effect,
        EnvironmentEffect::Water | EnvironmentEffect::FlowingWater | EnvironmentEffect::Ocean
    ) {
        let count = if effect == EnvironmentEffect::Ocean {
            3
        } else {
            4
        };
        let Some(params) = class_variant
            .and_then(|(class, variant)| lighting.material_parameter_array(class, variant))
            .filter(|params| params.len() >= count)
        else {
            return Ok(Err(format!(
                "{effect:?}: water material parameters unresolved"
            )));
        };
        let third = if count == 3 { scalars[0] } else { params[3] };
        scalars = [params[1], params[2], third];
    }
    let binding = match material_binding(effect, identities, &textures, bindings)? {
        Ok(binding) => binding,
        Err(reason) => return Ok(Err(reason)),
    };
    let [
        macro_overlay_uv_scale,
        detail_normal_uv_scale,
        macro_overlay_opacity,
    ] = scalars;
    Ok(Ok((
        binding,
        InstanceMaterial {
            lightmap_dot,
            macro_overlay_uv_scale,
            detail_normal_uv_scale,
            macro_overlay_opacity,
            m_params,
        },
    )))
}

/// One material's sampler bind group, reused across generations by texture
/// identity.
fn material_binding(
    effect: EnvironmentEffect,
    identities: Vec<TextureIdentity>,
    textures: &[GpuTexture],
    bindings: &mut MaterialBindings<'_>,
) -> RenderResult<Result<Arc<wgpu::BindGroup>, String>> {
    let key = (effect, identities);
    bindings.used.insert(key.clone());
    if let Some(binding) = bindings.cache.get(&key) {
        return Ok(Ok(Arc::clone(binding)));
    }
    let views = textures.iter().collect::<Vec<_>>();
    Ok(
        match bindings
            .renderer
            .create_environment_material(effect, &views)
        {
            Ok(binding) => {
                bindings.cache.insert(key, Arc::clone(&binding));
                Ok(binding)
            }
            Err(error) => Err(error.to_string()),
        },
    )
}

fn resident_emitter_waypoints(scene: &PresentationScene) -> Vec<Arc<skate2_core::world_particles::EmitterWaypointData>> {
    scene
        .resident_collections()
        .flat_map(|collection| collection.arenas.iter())
        .flat_map(|arena| arena.emitter_waypoints.iter().cloned())
        .collect()
}

/// CPU/GPU replacement owners live on one worker; active draws remain on the window thread.
struct MapBuild {
    scene: Option<PresentationScene>,
    textures: Option<TextureScene>,
    queue: NativeRenderQueue,
    diagnostic_techniques: DiagnosticTechniques,
    proxy_textures: BTreeMap<u64, GpuTexture>,
    environment: Arc<EnvironmentLighting>,
    environment_resources: Option<Arc<wgpu::BindGroup>>,
    mesh_cache: BTreeMap<(u64, usize, usize), GpuMesh>,
    texture_cache: BTreeMap<(bool, u64, usize, usize), GpuTexture>,
    material_cache: BTreeMap<MaterialKey, Arc<wgpu::BindGroup>>,
    diagnostic_pipelines: BTreeMap<DiagnosticPipelineKey, Arc<wgpu::RenderPipeline>>,
    clip: [f32; 16],
    view_position: [f32; 3],
    frame: FrameEnvironment,
}
impl Inspection {
    fn take_map_build(
        &mut self,
        renderer: &GpuRenderer,
        queue: NativeRenderQueue,
        diagnostic_techniques: DiagnosticTechniques,
    ) -> RenderResult<MapBuild> {
        let proxy_textures = self
            .proxy_textures
            .as_ref()
            .map(|proxy| {
                proxy
                    .entries()
                    .map(|(guid, texture)| (guid, texture.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let (clip, view_position) = self
            .external_view
            .unwrap_or_else(|| self.camera(renderer.dimensions()));
        Ok(MapBuild {
            clip,
            view_position,
            frame: self.frame_environment,
            scene: self.scene.take(),
            textures: self.textures.take(),
            queue,
            diagnostic_techniques,
            proxy_textures,
            environment: Arc::clone(&self.environment),
            environment_resources: self.environment_resources.take(),
            mesh_cache: std::mem::take(&mut self.mesh_cache),
            texture_cache: std::mem::take(&mut self.texture_cache),
            material_cache: std::mem::take(&mut self.material_cache),
            diagnostic_pipelines: std::mem::take(&mut self.diagnostic_pipelines),
        })
    }
    fn install_map_build(&mut self, build: MapBuild) {
        if let Some(scene) = &build.scene {
            self.world_probes.sync(scene);
            self.emitter_waypoints = resident_emitter_waypoints(scene);
        }
        self.scene = build.scene;
        self.textures = build.textures;
        self.queue = build.queue;
        self.diagnostic_techniques = build.diagnostic_techniques;
        self.environment_resources = build.environment_resources;
        self.mesh_cache = build.mesh_cache;
        self.texture_cache = build.texture_cache;
        self.material_cache = build.material_cache;
        self.diagnostic_pipelines = build.diagnostic_pipelines;
    }
}
impl MapBuild {
    fn build(&mut self, renderer: &GpuResourceBuilder) -> RenderResult<GpuMap> {
        let scene = self.scene.as_ref().ok_or("map scene is still loading")?;
        let textures = self
            .textures
            .as_ref()
            .ok_or("map textures are still loading")?;
        let mut used_meshes = BTreeSet::new();
        let mut mesh_uploads = 0usize;
        // Project the currently retained inventory through the original
        // shader table's low/high selection. This is not a replacement for
        // native Presentation registration or its retained arena lifetime.
        let mut texture_table = ShaderTextureTable::new();
        for guid in self.proxy_textures.keys() {
            texture_table
                .add_texture(*guid, TextureSource::Proxy(*guid))
                .map_err(|e| format!("PROXY shader texture table: {e:?}"))?;
        }
        for (collection_index, collection) in scene.resident_collections().enumerate() {
            for (arena_index, arena) in collection.arenas.iter().enumerate() {
                for (texture_index, texture) in arena.textures.iter().enumerate() {
                    let handle = ResidentTextureHandle {
                        collection_index,
                        arena_index,
                        texture_index,
                    };
                    texture_table
                        .add_texture(texture.guid, TextureSource::Presentation(handle))
                        .map_err(|e| format!("presentation shader texture table: {e:?}"))?;
                }
            }
        }
        for (guid, handle) in textures.resident_texture_index() {
            texture_table
                .add_texture(guid, TextureSource::High(handle))
                .map_err(|error| format!("resident shader texture table: {error:?}"))?;
        }
        let mut resolver = TextureResolver {
            renderer,
            scene,
            textures,
            table: texture_table,
            proxy: &self.proxy_textures,
            cache: &mut self.texture_cache,
            uploaded: BTreeMap::new(),
            used: BTreeSet::new(),
            uploads: 0,
            absent: BTreeMap::new(),
            cubes: BTreeMap::new(),
        };
        let mut bindings = MaterialBindings {
            renderer,
            cache: &mut self.material_cache,
            used: BTreeSet::new(),
        };
        let mut missing = BTreeMap::<String, usize>::new();
        let mut original_fallbacks = BTreeMap::<String, usize>::new();
        let mut original = BTreeMap::<EnvironmentEffect, usize>::new();
        let mut diagnostic_effects = BTreeMap::<String, usize>::new();
        let mut occlusion_test_skipped = 0usize;
        let mut textured = 0;
        let mut alpha_tested = 0;
        // One retained draw per submission, in the queue's stable sort order.
        let mut map = renderer.map_builder(self.queue.submissions.len());
        let mut shadow_models = BTreeMap::new();
        for submission in &self.queue.submissions {
            let source = submission.scene;
            let collection = scene
                .resident_collection(source.collection)
                .ok_or("draw collection owner absent")?;
            let arena = &collection.arenas[source.arena];
            // `WorldPresentation::AddRenderInstance` 0x827e4f00: a new-format
            // instance (`GetRenderInstanceType` 0x827d0a28: first mesh-table
            // material non-NULL) whose bone-name list starts "testCu"
            // (`StringnCompare(.., "testCube1_1000", 6)`) becomes the
            // `OccTestPresEntity`, which is never added to a render view.
            let model = &arena.models[source.model];
            let new_format = model
                .meshes
                .first()
                .is_some_and(|mesh| mesh.material.is_some());
            if new_format
                && model
                    .bone_names
                    .first()
                    .is_some_and(|name| name.starts_with("testCu"))
            {
                occlusion_test_skipped += 1;
                continue;
            }
            let mesh = &arena.meshes[source.mesh];
            let mesh_key = (collection.id, source.arena, source.mesh);
            used_meshes.insert(mesh_key);
            if !self.mesh_cache.contains_key(&mesh_key) {
                self.mesh_cache
                    .insert(mesh_key, renderer.upload_world_mesh(&mesh.geometry)?);
                mesh_uploads += 1;
            }
            let gpu = &self.mesh_cache[&mesh_key];
            let transform = world_from_mesh(submission);
            let material = &arena.materials[source.material];
            // AddRenderInstance827e510c uses the FIRST mesh's pass order to
            // register this entity in WorldShadowView. Proxies stay scene-only.
            let first = model.meshes.first().ok_or("shadow model has no meshes")?;
            let first_material = first.material.or(arena.meshes[first.mesh_index].material);
            let first_order = first_material
                .and_then(|i| arena.materials[i].effect.as_ref())
                .and_then(|effect| effect.binding.as_ref())
                .map(|binding| binding.draw_order);
            let is_sky = new_format
                && model
                    .bone_names
                    .first()
                    .is_some_and(|name| name.starts_with("skybox"));
            let shadow = if first_order == Some(self.environment.proxy_draw_order) {
                None
            } else {
                let key = (collection.id, source.arena, source.model);
                if !shadow_models.contains_key(&key) {
                    shadow_models.insert(
                        key,
                        (
                            skate2_core::presentation_shadow_culling::world_entity_bounds(
                                model.bounding_box,
                            )?,
                            Arc::<[_]>::from(model.island_bounding_boxes.clone()),
                            Arc::<[_]>::from(model.island_areas.clone()),
                        ),
                    );
                }
                let (entity, islands, areas) = &shadow_models[&key];
                let mut offset = 0;
                let mut found = false;
                for entry in &model.meshes {
                    if entry.mesh_index == source.mesh {
                        found = true;
                        break;
                    }
                    offset += arena.meshes[entry.mesh_index].geometry.draws.len();
                }
                let end = offset + mesh.geometry.draws.len();
                if !found || (new_format && !is_sky && end > islands.len()) {
                    return Err("world shadow RModel island range absent".into());
                }
                Some(WorldShadowGeometry {
                    entity: *entity,
                    islands: Arc::clone(islands),
                    areas: Arc::clone(areas),
                    range: offset..end,
                    cull_islands: new_format && !is_sky,
                    casts: !is_sky,
                })
            };
            map.set_shadow_geometry(shadow);
            // Ported original effects (`default` technique) with every input
            // they read; anything else keeps the diagnostic path below.
            if let Some(effect) = EnvironmentEffect::from_manifest(&submission.manifest_name)
                .filter(|_| submission.technique_name == "default")
            {
                // cModelInstance::Resolve 0x82853f90: the videoscreen fx's default technique
                // samples PostFX's main render target.
                let main_target = (submission.manifest_name == "videoscreen")
                    .then(|| renderer.main_target());
                let routed = match environment_material(
                    effect,
                    material,
                    main_target.as_ref(),
                    &self.environment,
                    &mut resolver,
                    &mut bindings,
                )? {
                    Ok((binding, instance)) if effect == EnvironmentEffect::Sky => map
                        .push_sky(gpu, binding, instance)
                        .map_err(|error| format!("{effect:?}: {error}")),
                    Ok((binding, instance)) if effect == EnvironmentEffect::GodraySpotlight => map
                        .push_environment_transform(
                            gpu,
                            effect,
                            binding,
                            instance,
                            Some(&submission.transform),
                        )
                        .map_err(|error| format!("{effect:?}: {error}")),
                    Ok((binding, instance)) => map
                        .push_environment(gpu, effect, binding, instance)
                        .map_err(|error| format!("{effect:?}: {error}")),
                    Err(reason) => Err(reason),
                };
                match routed {
                    Ok(()) => {
                        *original.entry(effect).or_default() += 1;
                        continue;
                    }
                    Err(reason) => {
                        if std::env::var_os("SKATE2_MATERIAL_AUDIT").is_some() {
                            std::eprintln!(
                                "material fallback source={source:?} scope={} reason={reason} channels={:?} vertex_streams={:?}",
                                submission.scope_name,
                                material.channels,
                                mesh.geometry
                                    .attributes
                                    .iter()
                                    .map(|a| (a.semantic, a.original_format))
                                    .collect::<Vec<_>>()
                            );
                        }
                        *original_fallbacks.entry(reason).or_default() += 1;
                    }
                }
            }
            *diagnostic_effects
                .entry(submission.manifest_name.clone())
                .or_default() += 1;
            // The technique's source alpha test and pass rasterizer state
            // (CULLMODE, DEPTHBIAS, SLOPESCALEDEPTHBIAS).
            let technique = *self
                .diagnostic_techniques
                .get(&(
                    submission.manifest_name.clone(),
                    submission.technique_name.clone(),
                ))
                .ok_or("diagnostic draw technique absent from its loaded manifest")?;
            let mut pipeline = |fragment: DiagnosticFragment| {
                Arc::clone(
                    self.diagnostic_pipelines
                        .entry((fragment, technique.raster.key()))
                        .or_insert_with(|| {
                            renderer.create_diagnostic_pipeline(fragment, technique.raster)
                        }),
                )
            };
            // AddPegasusMaterialToScope82e17d0c hashes the serialized name
            // with kInstanceHash; the channel is "diffuse", while the shader's
            // scoped sampler is named "i_diffuse".
            let diffuse = material
                .channels
                .iter()
                .find(|c| c.shader_input == "diffuse");
            let reason = match diffuse.map(|c| &c.value) {
                Some(PresentationMaterialValue::Raster { guid, .. }) => {
                    match resolver.resolve(*guid)? {
                        Ok((_, texture)) => {
                            // Exact original diffuse channel and UV0; lighting
                            // and other material channels are not applied.
                            let fragment = technique.alpha_reference.map_or(
                                DiagnosticFragment::Textured,
                                DiagnosticFragment::DiffuseAlpha,
                            );
                            let diffuse_pipeline = pipeline(fragment);
                            match map.push_textured(
                                gpu,
                                transform.as_ref(),
                                &texture,
                                Some(&diffuse_pipeline),
                            ) {
                                Ok(()) => {
                                    if technique.alpha_reference.is_some() {
                                        alpha_tested += 1;
                                    }
                                    textured += 1;
                                    continue;
                                }
                                Err(error) => error.to_string(),
                            }
                        }
                        Err(reason) => {
                            format!("{}: diffuse {reason}", submission.manifest_name)
                        }
                    }
                }
                Some(PresentationMaterialValue::ScalarConstant(_)) => {
                    format!("{}: diffuse scalar", submission.manifest_name)
                }
                None => format!("{}: no diffuse channel", submission.manifest_name),
            };
            *missing.entry(reason).or_default() += 1;
            let solid_pipeline = pipeline(DiagnosticFragment::Solid);
            map.push_solid(
                gpu,
                transform.as_ref(),
                color(source.material),
                Some(&solid_pipeline),
            );
        }
        let TextureResolver {
            used: used_textures,
            uploads: texture_uploads,
            ..
        } = resolver;
        let used_materials = bindings.used;
        // Deferred submissions (effect resolved to the literal "default") are
        // never drawn; count them so a lost effect binding is visible.
        let deferred = self.queue.deferred.len();
        eprintln!(
            "original environment shading draws={original:?}; ported-effect draws kept diagnostic by reason={original_fallbacks:?}; diagnostic draws by effect={diagnostic_effects:?}; occlusion-test instances not rendered={occlusion_test_skipped}; deferred default-effect submissions={deferred}"
        );
        eprintln!(
            "inspection diffuse draws={textured}, source diffuse-alpha tested={alpha_tested}; explicitly solid diagnostic draws by reason={missing:?}"
        );
        // Keep only resources used by this generation. Existing submitted GPU
        // work retains its own references; roaming cannot grow this cache forever.
        self.mesh_cache.retain(|key, _| used_meshes.contains(key));
        self.texture_cache
            .retain(|key, _| used_textures.contains(key));
        self.material_cache
            .retain(|key, _| used_materials.contains(key));
        eprintln!(
            "city GPU reuse: mesh_uploads={mesh_uploads} texture_uploads={texture_uploads} retained_meshes={} retained_textures={} retained_materials={}",
            self.mesh_cache.len(),
            self.texture_cache.len(),
            self.material_cache.len()
        );
        let resources = if let Some(resources) = &self.environment_resources {
            Arc::clone(resources)
        } else {
            let resources =
                renderer.create_environment_resources(&self.environment.fresnel_lookup())?;
            self.environment_resources = Some(Arc::clone(&resources));
            resources
        };
        map.set_anti_frustums(
            scene
                .resident_collections()
                .flat_map(|collection| collection.arenas.iter())
                .flat_map(|arena| arena.anti_frustums.iter().cloned())
                .collect(),
        );
        let mut map = map.finish(
            self.clip,
            Some(MapEnvironment {
                resources,
                envattributes2: self.environment.exposure(),
            }),
        );
        map.set_view(self.clip, self.view_position, self.frame);
        Ok(map)
    }
}

// Fields are retained for destruction, never inspected by the worker.
#[allow(dead_code)]
struct RetiredMap {
    map: GpuMap,
    queue: NativeRenderQueue,
    techniques: DiagnosticTechniques,
}
