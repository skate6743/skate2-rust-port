//! The PC side of `Sk8::VIPresentation`: `GameModule::OneOffLoadTexture`'s
//! `visual_indicators_sk82.rx2` (its visual indicator data become the ribbon
//! resource, `AddInstancedSplineInstance` 0x828050a8) and
//! `cTexturePackManager::LoadTextures`' `vitextures.rx2`, loaded once (on a
//! loader thread from the view's construction, as the native loads them with the
//! game); then each frame `VIPresentation::Update` from the tick's restored
//! `VDReaderData` and `Render`'s strips for the scene pass.

use crate::gpu_renderer::{
    CameraRelativeClip, GpuRenderer, RenderResult,
    visual_indicator::{Frame, Textures, Vertices},
};
use skate2_core::{
    challenge::{
        vi_presentation::{self, ViPresentation, ViewInputs},
        visual_director::VisualIndicatorFrame,
    },
    nis::manager::NisView,
    presentation_restore::camera::CameraView,
};
use skate2_formats::{
    mount::MountSet,
    pegasus::{VISUAL_INDICATOR_DATA_TYPE_ID, VisualIndicatorData},
    rw4::Arena as Rw4Arena,
};
use std::{sync::Arc, thread::JoinHandle};

/// The two files as the loader thread reads and parses them.
struct LoadedAssets {
    indicators: Vec<VisualIndicatorData>,
    textures: skate2_core::texture::TextureArena,
}

pub(crate) struct VisualIndicatorView {
    presentation: ViPresentation,
    textures: Textures,
    vertices: Vertices,
    /// The texture each technique's effect scope last bound.
    last: [Option<u64>; 2],
    /// The shown tick `VIPresentation::Update` last ran at.
    updated_tick: Option<u32>,
    /// The files' read on the loader thread, until a frame takes it in.
    loading: Option<JoinHandle<Result<LoadedAssets, String>>>,
    /// Taken in once; a failed load leaves the indicators undrawn.
    loaded: Option<bool>,
}

/// Reads and parses the two files.
fn load(mounts: &MountSet) -> RenderResult<LoadedAssets> {
    let bytes = mounts.read_path(vi_presentation::ASSET_PATH)?;
    let resources = Rw4Arena::split_file_resources(&bytes)?;
    let arena = Rw4Arena::parse(&resources)?;
    let mut indicators = Vec::new();
    for entry in arena
        .dictionary
        .iter()
        .filter(|entry| entry.type_id == VISUAL_INDICATOR_DATA_TYPE_ID)
    {
        let object = arena
            .object_bytes(&resources, entry)
            .ok_or("visual indicator data range is invalid")?;
        indicators.push(VisualIndicatorData::parse(object)?);
    }
    let bytes = mounts.read_path(vi_presentation::TEXTURE_PATH)?;
    let resources = Rw4Arena::split_file_resources(&bytes)?;
    let arena = Rw4Arena::parse(&resources)?;
    let textures = skate2_core::mounted_character_scene::own_texture_arena(&arena, &resources)?;
    Ok(LoadedAssets { indicators, textures })
}

impl VisualIndicatorView {
    pub(crate) fn new(mounts: Arc<MountSet>) -> Self {
        let loading = std::thread::Builder::new()
            .name("sk8 indicator load".to_owned())
            .spawn(move || load(&mounts).map_err(|error| error.to_string()))
            .ok();
        Self {
            presentation: ViPresentation::new(),
            textures: Textures::default(),
            vertices: Vertices::default(),
            last: [None; 2],
            updated_tick: None,
            loading,
            loaded: None,
        }
    }

    /// Whether the indicators can draw: the loader thread's files taken in once it has finished
    /// (read here when no thread could start).
    fn ready(&mut self, renderer: &GpuRenderer, mounts: &MountSet) -> bool {
        if self.loaded.is_none() {
            let assets = match self.loading.take() {
                Some(loading) if !loading.is_finished() => {
                    self.loading = Some(loading);
                    return false;
                }
                Some(loading) => loading.join().unwrap_or_else(|_| Err("the visual indicator loader panicked".to_owned())),
                None => load(mounts).map_err(|error| error.to_string()),
            };
            let loaded = assets.map_err(Into::into).and_then(|assets| self.take_in(renderer, &assets));
            if let Err(error) = &loaded {
                eprintln!("visual indicators not loaded ({error}); location ribbons stay undrawn");
            }
            self.loaded = Some(loaded.is_ok());
        }
        self.loaded == Some(true)
    }

    /// The indicator data's splines (`AddInstancedSplineInstance`), then the textures.
    fn take_in(&mut self, renderer: &GpuRenderer, assets: &LoadedAssets) -> RenderResult<()> {
        for data in &assets.indicators {
            self.presentation.add_instanced_splines(data);
        }
        self.textures = renderer.visual_indicator_textures(&assets.textures.textures)?;
        self.presentation.register_textures(self.textures.ids());
        Ok(())
    }

    /// `VIPresentation::Update` once per shown tick, as the native presents one frame per tick
    /// (whole ticks, so `dT` is 1/60 a tick), the frames between only facing their camera; then
    /// `Render`.
    #[expect(
        clippy::too_many_arguments,
        reason = "one frame's camera, records and scene state"
    )]
    pub(crate) fn frame(
        &mut self,
        renderer: &GpuRenderer,
        mounts: &MountSet,
        indicators: &VisualIndicatorFrame,
        streamed: &[Arc<VisualIndicatorData>],
        view: &CameraView,
        clip_from_world: &[f32; 16],
        camera: &CameraRelativeClip,
        shown: (u32, f32),
        nis: Option<&NisView>,
    ) -> Option<Frame> {
        if !self.ready(renderer, mounts) {
            return None;
        }
        let row = |axis: [f32; 3], w: f32| [axis[0], axis[1], axis[2], w];
        let camera_rows = [
            row(view.right, 0.0),
            row(view.up, 0.0),
            row(view.forward, 0.0),
            row(view.position, 1.0),
        ];
        let clip_rows =
            std::array::from_fn(|r| std::array::from_fn(|c| clip_from_world[c * 4 + r]));
        let inputs = ViewInputs::new(camera_rows, clip_rows);
        // SceneManager::IsInNIS: a scene handle exists.
        let in_nis = nis.is_some_and(|nis| nis.scene.is_some());
        let hidden = nis.is_some_and(|nis| nis.vis_indicators_hidden);
        self.presentation.sync_streamed_splines(streamed);
        if self.updated_tick == Some(shown.0) {
            self.presentation.face_camera(&inputs);
        } else {
            self.updated_tick = Some(shown.0);
            self.presentation
                .update(indicators, &inputs, (shown.0, 0.0), in_nis, hidden);
        }
        let draws = self.presentation.render(indicators, in_nis);
        if std::env::var_os("SKATE2_TRACE_VISUAL_INDICATORS").is_some() {
            static LAST: std::sync::Mutex<(usize, usize, usize)> = std::sync::Mutex::new((usize::MAX, usize::MAX, usize::MAX));
            static FRAMES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let periodic = FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 120 == 0;
            let now = (indicators.location_ribbons.len(), draws.len(), indicators.race_gates.len());
            if let Ok(mut last) = LAST.lock()
                && (*last != now || (periodic && !draws.is_empty()))
            {
                eprintln!(
                    "visual indicators: {} location ribbons, {} race gates, {} draws, camera {:?}",
                    now.0, now.2, now.1, view.position
                );
                if periodic && let Some(gate) = indicators.race_gates.first() {
                    eprintln!("visual indicators: gate next={} finish={} transform={:?} half={:?} spline={:?}",
                        gate.is_next, gate.is_finish, gate.transform, gate.half_lengths, gate.style.spline_asset);
                }
                if let Some(first) = draws.first() {
                    let point = first.layers.first().and_then(|layer| layer.first()).map(|cp| {
                        std::array::from_fn::<f32, 3, _>(|r| {
                            (0..4).map(|c| first.world[r][c] * cp[c]).sum::<f32>()
                        })
                    });
                    eprintln!(
                        "visual indicators: first textures {:?} loaded {:?} intensity {:?} clip {:?} layers {} world {:?} cp0 {:?}",
                        first.textures,
                        first.textures.map(|t| t.map(|id| id.is_some_and(|id| self.textures.ids().any(|loaded| loaded == id)))),
                        first.intensity,
                        first.clip_values,
                        first.layers.len(),
                        first.world,
                        point
                    );
                    if let Some(cp) = first.layers.first().and_then(|layer| layer.get(4)) {
                        let world: [f32; 3] =
                            std::array::from_fn(|r| (0..4).map(|c| first.world[r][c] * cp[c]).sum::<f32>());
                        let relative = [
                            world[0] - camera.camera[0],
                            world[1] - camera.camera[1],
                            world[2] - camera.camera[2],
                            1.0,
                        ];
                        let clip: [f32; 4] = std::array::from_fn(|r| {
                            (0..4).map(|c| camera.clip_from_camera[c * 4 + r] * relative[c]).sum::<f32>()
                        });
                        eprintln!(
                            "visual indicators: cp4 world {world:?} ndc {:?} w {}",
                            [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]],
                            clip[3]
                        );
                    }
                    if let Some(layer) = first.layers.first() {
                        eprintln!(
                            "visual indicators: layer0 cp[0..8] {:?} vertices {:?} count {}",
                            &layer[..layer.len().min(6)],
                            first.vertices,
                            first.vertex_count
                        );
                    }
                }
                *last = now;
            }
        }
        renderer.visual_indicator_frame(
            &draws,
            camera,
            &self.textures,
            &mut self.vertices,
            &mut self.last,
        )
    }
}
