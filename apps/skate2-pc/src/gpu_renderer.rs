//! GPU foundation for the explicitly diagnostic original-geometry view.
//!
//! This shader is not a reconstruction of a Skate 2 material. Callers supply
//! transforms and retained decoded geometry; this module does not advance gameplay.
//! Backend baseline: wgpu 30.0.1, winit 0.29.15 (MSRV 1.65).
//! Official versioned manifests: <https://docs.rs/crate/wgpu/30.0.1/source/Cargo.toml>
//! and <https://docs.rs/crate/winit/0.29.15/source/Cargo.toml> .
//! DX12/Vulkan are native candidates; adapter selection is reported, not assumed.
//! Browser/Android event lifecycle and texture compression need separate validation.

use skate2_core::{texture::NativeTexture, ui::options::FrameRateCap};
use skate2_formats::{
    material::{
        PIXEL_FORMAT_A8R8G8B8, PIXEL_FORMAT_DXT1, PIXEL_FORMAT_DXT5, PIXEL_FORMAT_LIN_DXT5,
        PIXEL_FORMAT_R5G6B5,
        TextureType,
    },
    render::{
        NativeMeshGeometry, NativeVertexAttributeValues, PRIMITIVE_TYPE_TRIANGLE_LIST,
        VertexSemantic,
    },
};
use std::{
    error::Error,
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::util::DeviceExt;
use winit::window::Window;

pub mod character;
#[expect(
    dead_code,
    reason = "same-palette GPU binding awaits the production session/window caller"
)]
pub(crate) mod character_palette;
mod dynamic_object;
mod environment_shading;
mod fps;
mod frame_uploads;
pub(crate) mod marker_ui;
pub(crate) mod final_pass;
mod postfx;
mod gpu_profile;
mod main_target;
mod map_draw;
mod mesh_arena;
mod media_capture;
mod raster_state;
mod shadow_map;
mod texture_bc;
pub(crate) mod visual_indicator;
pub(crate) mod particles;
pub(crate) mod crowd;
pub(crate) mod deck_ao;
pub(crate) mod movie;
mod stereoscopy;

pub(crate) use dynamic_object::{
    DynamicObjectDraw, DynamicObjectEffect, DynamicObjectMeshInput, GpuDynamicObject,
    GpuDynamicObjectModel, VehicleDrawConstants, vehicle_bone_index,
};
pub(crate) use environment_shading::EnvironmentEffect;
pub(crate) use map_draw::{
    DiagnosticFragment, DiagnosticPipelineKey, FrameEnvironment, GpuMap, GpuMapBuilder,
    InstanceMaterial, MapEnvironment, MapFrameStats, WorldShadowGeometry, world_box_in_view,
};
pub(crate) use raster_state::RasterState;
pub(crate) use shadow_map::ShadowCascades;

pub type RenderResult<T> = Result<T, Box<dyn Error>>;

/// Owns uploaded bytes independently of the borrowed decoded arena.
#[derive(Clone)]
pub struct GpuMesh {
    vertices: Arc<wgpu::Buffer>,
    indices: Arc<wgpu::Buffer>,
    /// One indexed draw per original optimesh island.
    draws: Arc<[(Range<u32>, i32)]>,
    has_uv: bool,
    /// Original TEXCOORD1 (zero-padded to four lanes), TANGENT and
    /// TEXCOORD0.zw per vertex, `LIGHTING_STRIDE` bytes each. World uploads
    /// supply the native absent-semantic default for missing TEXCOORD1;
    /// a normal-only spotlight stream also occupies this buffer.
    lighting: Option<Arc<wgpu::Buffer>>,
    /// Available TEXCOORD1 lanes (2/4, or4 for the native world default).
    lightmap_components: u8,
    has_tangent: bool,
    /// NORMAL0 shares the unused tangent slot for normal-only world meshes.
    has_normal: bool,
    /// TEXCOORD0 decoded with four lanes (decalenvironment `uv_decal`) and
    /// carried in `lighting`.
    has_decal_uv: bool,
    /// Union of `island_bounds`; None when a non-empty island is unbounded.
    bounds: Option<map_draw::Bounds>,
    /// Positions each island's indices actually reference, aligned with `draws`.
    island_bounds: Arc<[Option<map_draw::Bounds>]>,
}

pub struct GpuObject {
    pub mesh: GpuMesh,
    uniform: wgpu::Buffer,
    central_clip: std::sync::Mutex<[f32;16]>,
    binding: wgpu::BindGroup,
    texture_binding: Option<Arc<wgpu::BindGroup>>,
    character: Option<character::CharacterBinding>,
    /// Left out of every pass (`RenderView::DetermineVisibilityFromGameState`
    /// `0x82829698`: an entity whose type the render types do not cover).
    pub(crate) hidden: bool,
}

/// Owns a validated original 2D or cube texture and all uploaded mips; the
/// view retains the texture. 2D textures also carry their one diagnostic
/// diffuse bind group.
#[derive(Clone)]
pub struct GpuTexture {
    view: Arc<wgpu::TextureView>,
    binding: Option<Arc<wgpu::BindGroup>>,
    cube: bool,
}

impl GpuTexture {
    pub(crate) fn is_cube(&self) -> bool {
        self.cube
    }
}

/// Everything presented in one frame: the retained map draw list first, then
/// individually bound objects such as the selected character and the spawned
/// LivingWorld DMOs, in material draw order.
#[derive(Default)]
pub struct GpuScene {
    pub(crate) stereoscopy: bool,
    pub(crate) stereo_planes: Option<[f32;2]>,
    pub(crate) career_photo: Option<u32>,
    pub(crate) replay_media: Option<crate::replay_media::Request>,
    pub(crate) capture_watermark: Option<marker_ui::DrawList>,
    pub(crate) postfx: Option<(std::sync::Arc<crate::render_postfx::Assets>, crate::render_postfx::Frame)>,
    pub(crate) world_ui: Option<marker_ui::WorldPanel>,
    /// `ParticlePresentation::Draw`'s quad lists, drawn after the world UI.
    pub(crate) particles: Option<particles::Frame>,
    /// `CrowdPresentation`'s imposter targets, drawn into the atlas after the shadow cascades,
    /// and its imposters, drawn after the world UI and before the particles.
    pub(crate) crowd: crowd::Frame,
    /// `RenderAOBoxes`' boxes for this frame's skaters, drawn after the uber pass.
    pub(crate) deck_ao: Option<deck_ao::DeckAoFrame>,
    /// `VIPresentation::Render`'s strips, drawn after the world UI, before PostFX.
    pub(crate) visual_indicators: Option<visual_indicator::Frame>,
    /// `SkaterPresEntity::DrawName`'s online names, drawn after the scene's meshes.
    pub(crate) online_names: Option<marker_ui::DrawList>,
    /// `ChaseArrowMan::Render`'s arrows, drawn before the APT levels.
    pub(crate) chase_arrows: Option<marker_ui::DrawList>,
    pub(crate) marker_ui: Option<marker_ui::DrawList>,
    pub(crate) fps_ui: Option<marker_ui::DrawList>,
    pub(crate) frontend_fade: Option<marker_ui::DrawList>,
    pub(crate) frontend_fade_alpha: f32,
    /// The NIS transition's bloom to white as a white cover, for frames without PostFX.
    pub(crate) nis_white: Option<marker_ui::DrawList>,
    pub(crate) nis_white_alpha: f32,
    /// `AptMovieIntegration::Render`'s movie and background this frame.
    pub(crate) movie: Option<movie::MovieDraw>,
    pub(crate) final_blit: Option<(final_pass::FisheyeParameters, GpuTexture)>,
    pub(crate) postfx_overlay: Option<GpuTexture>,
    pub(crate) out_of_boundary_amount: Option<f32>,
    pub(crate) map: Option<GpuMap>,
    pub objects: Vec<GpuObject>,
    /// This frame's DMOs in entity-map order; drawn only with a map, whose
    /// globals they read.
    pub(crate) dynamic_objects: Vec<DynamicObjectDraw>,
    /// When the simulation tick this frame first shows fell due; `None` repeats an image.
    pub(crate) tick_due: Option<Instant>,
    /// The client changed nothing since the previous frame; the window presents
    /// nothing for it.
    pub(crate) unchanged: bool,
}

/// Host present pacing for new simulation images without vsync: each is presented a fixed
/// time after its tick fell due, about the 95th percentile of the last second's due-to-ready
/// times, so consecutive images keep the fixed step's spacing whatever a tick or its
/// frame costs. A slower image presents when ready rather than holding every image back.
/// Under vsync the display's refresh already times the flip.
#[derive(Default)]
struct PresentPacing {
    recent: std::collections::VecDeque<Duration>,
}

impl PresentPacing {
    const IMAGES: usize = 60;
    /// The images slower than the spacing allowed in the window.
    const OUTLIERS: usize = 3;
    const MARGIN: Duration = Duration::from_micros(500);
    const LIMIT: Duration = Duration::from_millis(16);

    fn wait(&mut self, due: Instant) {
        let ready = Instant::now();
        if self.recent.len() == Self::IMAGES {
            self.recent.pop_front();
        }
        self.recent.push_back(ready.saturating_duration_since(due).min(Self::LIMIT));
        let mut sorted: Vec<Duration> = self.recent.iter().copied().collect();
        sorted.sort_unstable();
        let spacing = sorted[sorted.len().saturating_sub(Self::OUTLIERS + 1)];
        wait_until(due + spacing + Self::MARGIN);
    }
}

/// The Frame Rate Cap's 60 and 120, and Refresh Rate without VSync (the monitor's interval): a
/// frame starts one interval after the last presented frame
/// started, or at once when it is later. The wait comes before the client reads its clock, so
/// the moments the frames show keep the interval too.
#[derive(Default)]
struct FrameCap {
    /// When the last presented frame started.
    presented: Option<Instant>,
    /// When the frame being drawn started.
    started: Option<Instant>,
}

impl FrameCap {
    fn wait(&mut self, interval: Option<Duration>) {
        let mut start = Instant::now();
        if let (Some(interval), Some(presented)) = (interval, self.presented) {
            let target = presented + interval;
            if target > start {
                wait_until(target);
                start = target;
            }
        }
        self.started = Some(start);
    }

    fn presented(&mut self) {
        self.presented = self.started.take();
    }
}

/// Sleeps until a millisecond before `target`, then spins to it.
fn wait_until(target: Instant) {
    if let Some(rest) = target.checked_duration_since(Instant::now()) {
        if let Some(coarse) = rest.checked_sub(Duration::from_millis(1)) {
            std::thread::sleep(coarse);
        }
        while Instant::now() < target {
            std::hint::spin_loop();
        }
    }
}

/// A clip matrix for positions relative to `camera`: `clip_from_world` with
/// its translation column rebased there in f64, so `clip_from_camera * (p -
/// camera, 1)` is `clip_from_world * (p, 1)`. City coordinates are thousands
/// of metres from the origin; transformed directly, their f32 products round
/// by ~1e-4 and the projection (near 0.1) scales that by distance/near into
/// depth order: 11 mm at 3 m, 57 mm at 20 m near (-3500, -900), where the
/// camera-relative form stays under 0.6 mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CameraRelativeClip {
    pub(crate) clip_from_camera: [f32; 16],
    pub(crate) camera: [f32; 3],
}

impl CameraRelativeClip {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the f64 rebase narrows to the f32 GPU lanes"
    )]
    pub(crate) fn new(clip_from_world: &[f32; 16], camera: [f32; 3]) -> Self {
        let mut clip_from_camera = *clip_from_world;
        for row in 0..4 {
            let translation = (0..3).fold(f64::from(clip_from_world[12 + row]), |sum, axis| {
                f64::from(clip_from_world[axis * 4 + row]).mul_add(f64::from(camera[axis]), sum)
            });
            clip_from_camera[12 + row] = translation as f32;
        }
        Self {
            clip_from_camera,
            camera,
        }
    }
}

/// Wall-clock split of the most recent `render_frame`, plus map submission counts.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FrameDiagnostics {
    /// Completed asynchronous measurements, possibly from earlier frames.
    pub(crate) gpu: gpu_profile::Measurements,
    /// Waiting for the next swapchain image.
    pub(crate) acquire: Duration,
    /// CPU recording, culling and wgpu validation of the frame's commands.
    pub(crate) record: Duration,
    /// Queue submission and presentation.
    pub(crate) submit_present: Duration,
    pub(crate) dynamic_upload: Duration,
    pub(crate) dynamic_shadows: Duration,
    pub(crate) world_shadows: Duration,
    /// The objects pass, recorded on the render thread.
    pub(crate) scene: Duration,
    /// Selecting the map's islands for a changed view, on a render worker.
    pub(crate) map_prepare: Duration,
    /// Main-pass object and dynamic draws after the map.
    pub(crate) objects: Duration,
    /// Closing the objects pass, then waiting for the map's runs.
    pub(crate) pass_end: Duration,
    /// PostFX, final blit and overlays.
    pub(crate) post: Duration,
    /// Encoding the render thread's command buffers.
    pub(crate) finish: Duration,
    pub(crate) submit: Duration,
    pub(crate) present: Duration,
    pub(crate) world_shadow_islands: usize,
    pub(crate) dynamic_objects: usize,
    pub(crate) map: MapFrameStats,
    /// The on-screen frame counter's latest half-second value.
    pub(crate) fps_counter: u32,
}

/// A frame's commands on either side of the map's runs.
struct RecordedFrame {
    /// The shadow maps and the world UI panel's texture.
    before: wgpu::CommandBuffer,
    /// The objects pass, PostFX, the final blit and overlays.
    after: wgpu::CommandBuffer,
    /// The render thread's share of the frame's diagnostics.
    diagnostics: FrameDiagnostics,
}

pub struct GpuRenderer {
    media_capture: Option<media_capture::Target>,
    stereo_objects_active: bool,
    present_pacing: PresentPacing,
    frame_cap: FrameCap,
    /// For the monitor's refresh rate.
    window: Arc<Window>,
    /// The monitor's refresh interval and when it was read.
    refresh: Option<(Instant, Duration)>,
    display_settings: std::cell::Cell<skate2_core::ui::options::PcDisplaySettings>,
    /// The Resolution Scale (percent) the scene's depth, colour and PostFX targets are built
    /// at: 100 while a frame has no final blit to bring a scaled scene to the window, or when
    /// the GPU could not allocate `scale_requested`.
    resolution_scale: u8,
    /// The Resolution Scale last asked for, so a refused one is not retried every frame.
    scale_requested: u8,
    /// PC port change: PostFX stays on while a cutscene's own post-fx runs, whatever Video
    /// Settings says.
    cutscene_postfx: std::cell::Cell<bool>,
    mesh_arena: Arc<std::sync::Mutex<mesh_arena::MeshArena>>,
    gpu_profile: Option<gpu_profile::GpuProfile>,
    character_uploads: frame_uploads::FrameUploads,
    fps: fps::Fps,
    marker_ui: marker_ui::Pipeline,
    visual_indicators: visual_indicator::Pipelines,
    particles: particles::Pipelines,
    crowd: crowd::Pipelines,
    movie: std::cell::RefCell<movie::MoviePipeline>,
    final_pass: Option<final_pass::FinalPass>,
    /// `PostFX`'s main render target, which the video screens sample (`main_target.rs`).
    main_target: main_target::MainTarget,
    surface: wgpu::Surface<'static>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    /// Presentation antialiasing samples shared by every main-pass pipeline.
    sample_count: u32,
    depth: wgpu::TextureView,
    /// The same depth, depth aspect only, for the NIS grade's depth reads.
    depth_sample: wgpu::TextureView,
    stencil_sample: wgpu::TextureView,
    /// Multisampled color target resolved into the swapchain image; None at 1x.
    msaa_color: Option<wgpu::TextureView>,
    pipeline: wgpu::RenderPipeline,
    object_layout: Arc<wgpu::BindGroupLayout>,
    texture_layout: Arc<wgpu::BindGroupLayout>,
    textured_pipeline: wgpu::RenderPipeline,
    character_pipelines: character::CharacterPipelines,
    map_pipelines: Arc<map_draw::MapPipelines>,
    /// The `dynamicobject` effects the spawned DMOs draw with.
    dynamic_objects: Arc<dynamic_object::DynamicObjectPipelines>,
    /// `RenderShadows_CSM` targets and filters.
    shadows: shadow_map::ShadowMaps,
    camera: wgpu::Buffer,
    camera_binding: wgpu::BindGroup,
    sampler: Arc<wgpu::Sampler>,
    drawable: bool,
    capture_supported: bool,
    /// The surface's present modes, for the Frame Rate Cap.
    present_modes: Vec<wgpu::PresentMode>,
    /// `SKATE2_NO_VSYNC`: Unlimited, whatever Video Settings say.
    no_vsync: bool,
    /// The window has the keyboard focus.
    focused: bool,
    last_frame: FrameDiagnostics,
    /// The most map runs a frame encodes at once (`SKATE2_MAP_THREADS`).
    map_threads: usize,
}

impl GpuRenderer {
    /// PostFX, fisheye and the Frame Rate Cap from Video Settings; the next frame applies them.
    /// Whether the window shows fullscreen (Display Settings' Fullscreen).
    pub(crate) fn fullscreen(&self) -> bool {
        self.display_settings.get().fullscreen
    }
    pub(crate) fn set_display_settings(&self, settings: skate2_core::ui::options::PcDisplaySettings) {
        self.display_settings.set(settings);
    }
    /// A cutscene's post-fx is active (`mICEPostfxActive`, `Scene::Init` to `Scene::Purge`).
    /// Both ends fall under the NIS fades, so turning PostFX on for it does not show.
    pub(crate) fn set_cutscene_postfx(&self, active: bool) {
        self.cutscene_postfx.set(active);
    }
    fn effective_display_settings(&self) -> skate2_core::ui::options::PcDisplaySettings {
        let mut settings = self.display_settings.get();
        settings.postfx |= self.cutscene_postfx.get();
        settings
    }
    pub(crate) fn postfx_exposure(&self)->Option<f32> {
        Some(if self.effective_display_settings().postfx { self.final_pass.as_ref().map_or(1., |p|p.exposure()) } else { 1. })
    }
    fn frame_rate_cap(&self) -> FrameRateCap {
        if self.no_vsync {
            FrameRateCap::Unlimited
        } else {
            self.display_settings.get().frame_rate_cap
        }
    }
    /// Display Settings' VSync, off with `SKATE2_NO_VSYNC`.
    fn vsync_setting(&self) -> bool {
        !self.no_vsync && self.display_settings.get().vsync
    }
    /// Whether the present waits for the refresh: VSync, while the window has the focus. Without
    /// it, `FrameCap` waits the refresh instead: once another window covers this one, its FIFO
    /// swapchain can stop taking frames, and a frame held in the present holds the simulation,
    /// whose next tick waits for this frame to take the last one (online, long enough for the
    /// server to drop the game as frozen).
    fn vsync(&self) -> bool {
        self.vsync_setting() && self.focused
    }
    /// Whether the window has the keyboard focus.
    pub(crate) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    /// Before a frame starts: the Frame Rate Cap's wait at 60 and 120, and at Refresh Rate
    /// without VSync the monitor's interval. With VSync the present waits for the refresh;
    /// while the window is unfocused, no frame starts sooner than a refresh after the last.
    pub(crate) fn wait_for_frame_cap(&mut self) {
        let cap = match self.frame_rate_cap() {
            FrameRateCap::Fps60 => Some(Duration::from_secs(1) / 60),
            FrameRateCap::Fps120 => Some(Duration::from_secs(1) / 120),
            FrameRateCap::RefreshRate if !self.vsync() => Some(self.refresh_interval()),
            FrameRateCap::Unlimited | FrameRateCap::RefreshRate => None,
        };
        let interval = if self.vsync_setting() && !self.vsync() {
            let refresh = self.refresh_interval();
            Some(cap.map_or(refresh, |cap| cap.max(refresh)))
        } else {
            cap
        };
        self.frame_cap.wait(interval);
    }
    /// The monitor's refresh interval, read again each second as the window can move to another
    /// monitor; 60 Hz when the monitor does not say.
    fn refresh_interval(&mut self) -> Duration {
        let now = Instant::now();
        if let Some((read, interval)) = self.refresh {
            if now.duration_since(read) < Duration::from_secs(1) {
                return interval;
            }
        }
        let hertz = self
            .window
            .current_monitor()
            .and_then(|monitor| monitor.refresh_rate_millihertz())
            .filter(|&millihertz| millihertz >= 1000)
            .map_or(60.0, |millihertz| f64::from(millihertz) / 1000.0);
        let interval = Duration::from_secs_f64(1.0 / hertz);
        self.refresh = Some((now, interval));
        interval
    }
    pub async fn new(window: Arc<Window>) -> RenderResult<Self> {
        let size = window.inner_size();
        // Every backend and FXC, which ships with Windows, for DX12 shaders: wgpu's automatic
        // choice takes any dxcompiler.dll on the PATH, and an old one without dxil.dll fails every
        // shader. WGPU_BACKEND, WGPU_DX12_COMPILER and the other wgpu variables still override.
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc;
        let instance = wgpu::Instance::new(descriptor.with_env());
        let surface = instance.create_surface(Arc::clone(&window))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| format!("no compatible GPU adapter: {error}"))?;
        let info = adapter.get_info();
        eprintln!("diagnostic GPU adapter: {info:?}");
        crate::panic_report::set_gpu(format!("{} ({:?}, driver {} {})", info.name, info.backend, info.driver, info.driver_info));
        let profile_features = if std::env::var_os("SKATE2_FRAME_PROFILE").is_some()
            && std::env::var_os("SKATE2_GPU_PROFILE").is_some()
        {
            // The stamps go between passes, on the command encoder.
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
        } else {
            wgpu::Features::empty()
        };
        // Shadows_CSM_Init8283ecf8..ed44 resolves to G16R16, not G16R16F.
        // Native wgpu extensions are required to render to this normalized format.
        let shadow_extensions = wgpu::Features::TEXTURE_FORMAT_16BIT_NORM
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        let shadow_caps = adapter.get_texture_format_features(wgpu::TextureFormat::Rg16Unorm);
        let shadow_features = if adapter.features().contains(shadow_extensions)
            && shadow_caps.allowed_usages.contains(wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING)
            && shadow_caps.flags.contains(wgpu::TextureFormatFeatureFlags::FILTERABLE)
        { shadow_extensions } else { wgpu::Features::empty() };
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    experimental_features: Default::default(),
                    memory_hints: Default::default(),
                    trace: Default::default(),
                    label: Some("original geometry diagnostic"),
                    // Clamp-to-border serves `ShadowTextureSampler`'s BORDER mode.
                    required_features: adapter.features()
                        & (wgpu::Features::TEXTURE_COMPRESSION_BC
                            | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
                            | shadow_features
                            | profile_features),
                    required_limits: wgpu::Limits::default(),
                },
            )
            .await?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let caps = surface.get_capabilities(&adapter);
        // The original effects end in `MoveFromLinearSpace` (sqrt) and write
        // gamma-2 values straight to the target; no hardware sRGB encode.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|format| {
                matches!(
                    format,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
            .ok_or("original gamma-2 output requires a non-sRGB 8-bit surface")?;
        let capture_supported = caps.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let present_modes = caps.present_modes.clone();
        // `SKATE2_NO_VSYNC` forces VSync off and Unlimited, for measuring renderer headroom.
        // Otherwise VSync until the profile's Display Settings load; `render_frame` follows them.
        let no_vsync = std::env::var_os("SKATE2_NO_VSYNC").is_some();
        let present_mode = present_mode(&present_modes, !no_vsync);
        eprintln!("diagnostic present mode: {present_mode:?}");
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | if capture_supported {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            // defaultenvironment writes alpha 0 (the original reuses the
            // backbuffer alpha); presentation must not composite with it.
            alpha_mode: if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
                wgpu::CompositeAlphaMode::Opaque
            } else {
                *caps
                    .alpha_modes
                    .first()
                    .ok_or("surface has no alpha modes")?
            },
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        // Presentation-only enhancement: 4x MSAA when both the surface and depth
        // formats support it on this device (SKATE2_NO_MSAA restores 1x).
        let requested = if std::env::var_os("SKATE2_NO_MSAA").is_some() {
            1
        } else {
            4
        };
        let supported = |format: wgpu::TextureFormat| {
            format
                .guaranteed_format_features(device.features())
                .flags
                .sample_count_supported(requested)
                && adapter
                    .get_texture_format_features(format)
                    .flags
                    .sample_count_supported(requested)
        };
        let sample_count = if supported(format) && supported(DEPTH_FORMAT) {
            requested
        } else {
            1
        };
        eprintln!("diagnostic MSAA samples: {sample_count}");
        let (depth, depth_sample, stencil_sample) = depth_view(&device, &config, sample_count);
        let msaa_color = msaa_view(&device, &config, sample_count);
        let object_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("diagnostic object layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let (texture_layout, sampler) = create_texture_bindings(&device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("diagnostic solid/diffuse; not original material lighting"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = create_pipeline(
            &device,
            &shader,
            format,
            sample_count,
            &[Some(&object_layout)],
            "fs_main",
        );
        let textured_pipeline = create_pipeline(
            &device,
            &shader,
            format,
            sample_count,
            &[Some(&object_layout), Some(&texture_layout)],
            "fs_textured",
        );
        let character_pipelines = character::CharacterPipelines::new(
            &device,
            format,
            sample_count,
            &object_layout,
            &texture_layout,
        );
        let map_pipelines = Arc::new(map_draw::MapPipelines::new(
            &device,
            format,
            sample_count,
            &texture_layout,
        ));
        let (camera, camera_binding) = map_pipelines.create_camera(&device);
        let dynamic_object_layout = Arc::new(dynamic_object::object_layout(&device));
        let shadows = shadow_map::ShadowMaps::new(
            &device,
            character_pipelines.row_layout(),
            &dynamic_object_layout,
        );
        let dynamic_objects = Arc::new(dynamic_object::DynamicObjectPipelines::new(
            &device,
            format,
            sample_count,
            map_pipelines.camera_layout(),
            dynamic_object_layout,
            shadows.atlas(),
            shadows.world_map(),
            shadows.world_rows(),
        ));
        let fps = fps::Fps::new(&device, format);
        let marker_ui = marker_ui::Pipeline::new(&device, &queue, format, sample_count);
        let visual_indicators = visual_indicator::Pipelines::new(&device, format, sample_count);
        let particles = particles::Pipelines::new(&device, format, sample_count);
        let crowd = crowd::Pipelines::new(&device, format, sample_count, &object_layout, &texture_layout);
        let movie = std::cell::RefCell::new(movie::MoviePipeline::new(&device, format));
        let main_target = main_target::MainTarget::new(&device, format);
        let gpu_profile = gpu_profile::GpuProfile::new(&device, &queue);
        Ok(Self {
            character_uploads: frame_uploads::FrameUploads::new(&device),
            present_pacing: PresentPacing::default(),
            frame_cap: FrameCap::default(),
            window,
            refresh: None,
            mesh_arena: Arc::new(std::sync::Mutex::new(mesh_arena::MeshArena::default())),
            gpu_profile,
            fps,
            marker_ui,
            visual_indicators,
            particles,
            crowd,
            movie,
            final_pass: None,
            main_target,
            stereo_objects_active: false,
            display_settings: Default::default(),
            resolution_scale: 100,
            scale_requested: 100,
            cutscene_postfx: Default::default(),
            surface,
            device,
            queue,
            config,
            sample_count,
            depth,
            depth_sample,
            stencil_sample,
            msaa_color,
            pipeline,
            object_layout: Arc::new(object_layout),
            texture_layout: Arc::new(texture_layout),
            textured_pipeline,
            character_pipelines,
            map_pipelines,
            dynamic_objects,
            shadows,
            camera,
            camera_binding,
            sampler: Arc::new(sampler),
            media_capture: None,
            drawable: size.width != 0 && size.height != 0,
            capture_supported,
            present_modes,
            no_vsync,
            focused: true,
            last_frame: FrameDiagnostics::default(),
            map_threads: render_workers(),
        })
    }

    /// Timing split and map counts of the most recently recorded frame.
    pub(crate) fn last_frame(&self) -> FrameDiagnostics {
        self.last_frame
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.drawable = width != 0 && height != 0;
        if self.drawable {
            if let Some(capture)=&mut self.media_capture {
                capture.window_size=(width,height);
                let mut config=self.config.clone();config.width=width;config.height=height;
                self.surface.configure(&self.device,&config);
                return;
            }
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.rebuild_scene_targets();
        }
    }

    /// The scene's size at `percent` of the render size, its longer side within the device's
    /// largest texture.
    fn scene_size(&self, percent: u8) -> (u32, u32) {
        let (width, height) = (self.config.width, self.config.height);
        if percent == 100 {
            return (width, height);
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        let scale = (f64::from(percent) / 100.0).min(f64::from(limit) / f64::from(width.max(height)));
        let side = |length: u32| ((f64::from(length) * scale).round() as u32).clamp(1, limit);
        (side(width), side(height))
    }

    /// The scene's depth, MSAA colour and final-pass targets at `resolution_scale`; a scale the
    /// GPU cannot allocate falls back to 100%.
    fn rebuild_scene_targets(&mut self) {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.build_scene_targets();
        if let Some(error) = pollster::block_on(scope.pop()) {
            if self.resolution_scale != 100 {
                eprintln!("resolution scale {}% does not fit this GPU ({error}); using 100%", self.resolution_scale);
                self.resolution_scale = 100;
                self.build_scene_targets();
            }
        }
    }

    fn build_scene_targets(&mut self) {
        let (width, height) = self.scene_size(self.resolution_scale);
        let config = wgpu::SurfaceConfiguration { width, height, ..self.config.clone() };
        (self.depth, self.depth_sample, self.stencil_sample) = depth_view(&self.device, &config, self.sample_count);
        self.msaa_color = msaa_view(&self.device, &config, self.sample_count);
        if let Some(final_pass) = &mut self.final_pass {
            final_pass.resize(&self.device, width, height, &self.depth_sample, &self.stencil_sample, &self.main_target);
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn upload_mesh(&self, geometry: &NativeMeshGeometry) -> RenderResult<GpuMesh> {
        self.resource_builder().upload_mesh(geometry)
    }

    /// Matrix is column-major clip-from-local, with WebGPU depth range 0..1.
    pub fn create_object(&self, mesh: GpuMesh, matrix: &[f32; 16], color: &[f32; 4]) -> GpuObject {
        self.resource_builder().create_object(mesh, matrix, color)
    }

    pub fn update_object(&self, object: &GpuObject, matrix: &[f32; 16], color: &[f32; 4]) {
        *object.central_clip.lock().expect("object projection lock") = *matrix;
        let bytes = uniform_bytes(matrix, color);
        if object.character.is_some() {
            // Character uniforms have this single update entrypoint. Keep all
            // their writes in the same ordered staging stream as their palette.
            self.character_uploads.write(&object.uniform, 0, &bytes);
        } else {
            self.queue.write_buffer(&object.uniform, 0, &bytes);
        }
    }

    /// Uses actual UV0 and a caller-resolved diffuse texture. This is diagnostic
    /// texture color, without the original lightmaps, fog, shadows or tone mapping.
    pub fn create_textured_object(
        &self,
        mesh: GpuMesh,
        matrix: &[f32; 16],
        color: &[f32; 4],
        texture: &GpuTexture,
    ) -> RenderResult<GpuObject> {
        self.resource_builder()
            .create_textured_object(mesh, matrix, color, texture)
    }

    /// Share this device/queue with retained resident texture builders.
    pub(crate) fn texture_uploader(&self) -> GpuTextureUploader {
        self.resource_builder().texture_uploader()
    }

    /// One `dynamicobject` material: `i_diffuse`, `i_specular`, `i_normal`,
    /// `i_detail`, the `i_environment` cube, `m_params[0..2]` and
    /// `i_detailNormalUVScale`.
    pub(crate) fn create_dynamic_object_material(
        &self,
        textures: [&GpuTexture; 5],
        m_params: [[f32; 4]; 2],
        detail_normal_uv_scale: [f32; 4],
    ) -> RenderResult<Arc<wgpu::BindGroup>> {
        Ok(Arc::new(self.dynamic_objects.material(
            &self.device,
            textures,
            [
                m_params[0],
                m_params[1],
                [0.; 4],
                [0.; 4],
                [0.; 4],
                [0.; 4],
                [0.; 4],
                [0.; 4],
            ],
            detail_normal_uv_scale,
        )?))
    }
    pub(crate) fn create_vehicle_material(
        &self,
        textures: [&GpuTexture; 5],
        params: [[f32; 4]; 8],
        char_params: [f32; 4],
    ) -> RenderResult<Arc<wgpu::BindGroup>> {
        Ok(Arc::new(self.dynamic_objects.material(
            &self.device,
            textures,
            params,
            char_params,
        )?))
    }

    /// Uploads one DMO RModel's meshes; `bounds` is its bounding box.
    pub(crate) fn upload_dynamic_object_model(
        &self,
        meshes: Vec<DynamicObjectMeshInput<'_>>,
        bounds: [[f32; 3]; 2],
    ) -> RenderResult<Arc<GpuDynamicObjectModel>> {
        Ok(Arc::new(dynamic_object::upload_model(
            &self.device,
            meshes,
            bounds,
        )?))
    }

    /// One spawned DMO's constant over a shared model.
    pub(crate) fn create_dynamic_object(
        &self,
        model: Arc<GpuDynamicObjectModel>,
    ) -> Arc<GpuDynamicObject> {
        Arc::new(self.dynamic_objects.create_object(&self.device, model))
    }

    pub fn upload_native_texture(&self, source: &NativeTexture) -> RenderResult<GpuTexture> {
        self.resource_builder().upload_native_texture(source)
    }

    pub fn render(&mut self, scene: &GpuScene) -> RenderResult<()> {
        self.render_frame(scene, None)
    }

    /// A frame that draws nothing (a minimized window, an unchanged scene): its staged
    /// uploads still go to the GPU, so the staging memory comes back and the queue's pending
    /// writes and dropped resources are released rather than gathering until the next draw.
    pub fn submit_uploads(&mut self) {
        let uploads = self.character_uploads.finish();
        self.queue.submit(uploads);
        let _ = self.device.poll(wgpu::PollType::Poll);
    }

    /// Capture uses the same rendered surface; no alternate scene/pipeline.
    pub fn render_capture(&mut self, scene: &GpuScene, path: &std::path::Path) -> RenderResult<()> {
        let (width, height, rgba) = self.capture_pixels(scene, false)?;
        write_bmp(path, width, height, width * 4, &rgba, false)
    }

    /// ScreenshotBackEnd::GrabScreenshot 0x82562588 reads the post-effect scene,
    /// before FE overlays, at its full dimensions. The blog also needs its thumbnail.
    pub(crate) fn save_career_photo(&mut self, scene: &GpuScene, slot: u32) -> RenderResult<()> {
        let (width, height, rgba) = self.capture_pixels(scene, true)?;
        crate::career_photos::save(slot, width, height, &rgba)
    }

    pub(crate) fn save_replay_media(&mut self,scene:&GpuScene,request:&crate::replay_media::Request)->RenderResult<String> {
        let (width,height,rgba)=self.capture_pixels(scene,true)?;
        crate::replay_media::save_photo(request,width,height,&rgba)
    }

    pub(crate) fn capture_pixels(&mut self, scene: &GpuScene, before_ui: bool) -> RenderResult<(u32, u32, Vec<u8>)> {
        if !self.drawable {
            return Err("cannot capture a minimized surface".into());
        }
        if !self.capture_supported && self.media_capture.is_none() {
            return Err("surface does not support COPY_SRC capture".into());
        }
        if !matches!(
            self.config.format,
            wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
                | wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
        ) {
            return Err("capture requires an RGBA8/BGRA8 surface".into());
        }
        let row = self
            .config
            .width
            .checked_mul(4)
            .ok_or("capture row overflow")?;
        let stride =
            row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("same-pipeline screenshot"),
            size: u64::from(stride) * u64::from(self.config.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        self.render_frame(scene, Some((&buffer, stride, before_ui)))?;
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;
        let mapped = buffer.slice(..).get_mapped_range()?;
        let bgra = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut rgba = Vec::with_capacity((row * self.config.height) as usize);
        for source in mapped.chunks_exact(stride as usize).take(self.config.height as usize) {
            for pixel in source[..row as usize].chunks_exact(4) {
                if bgra { rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]); }
                else { rgba.extend_from_slice(pixel); }
            }
        }
        drop(mapped);
        buffer.unmap();
        Ok((self.config.width, self.config.height, rgba))
    }

    fn render_frame(
        &mut self,
        scene: &GpuScene,
        capture: Option<(&wgpu::Buffer, u32, bool)>,
    ) -> RenderResult<()> {
        if !self.drawable {
            self.submit_uploads();
            return Ok(());
        }
        // PC Resolution Scale: the scene renders at the scale only where a final blit brings it to
        // the window; a frame without one draws straight into the window, at 100%.
        let wanted = if scene.final_blit.is_some() {
            self.display_settings.get().resolution_scale.clamp(*skate2_core::ui::options::RESOLUTION_SCALES.start(),
                *skate2_core::ui::options::RESOLUTION_SCALES.end())
        } else {
            100
        };
        if wanted != self.scale_requested {
            self.scale_requested = wanted;
            self.resolution_scale = wanted;
            self.rebuild_scene_targets();
        }
        if let Some((_,vignette))=&scene.final_blit {
            if self.final_pass.is_none() {
                let (width, height) = self.scene_size(self.resolution_scale);
                self.final_pass=Some(final_pass::FinalPass::new(&self.device,&self.queue,self.config.format,width,height,vignette,
                    &self.depth_sample,&self.stencil_sample,scene.postfx_overlay.as_ref().ok_or("PostFX x-ray texture absent")?,self.sample_count>1,&self.main_target)?);
            }
        }
        let acquire_start = Instant::now();
        let settings = self.effective_display_settings();
        // Frozen scenes preview exposure immediately, without changing any
        // material gains. Cached owners upload only when the value changes.
        for object in &scene.objects {
            if let Some(character) = &object.character { character.apply_display_exposure(&self.character_uploads, settings.postfx); }
        }
        let mode = present_mode(&self.present_modes, self.vsync());
        if mode != self.config.present_mode {
            self.config.present_mode = mode;
            let mut config=self.config.clone();
            if let Some(capture)=&self.media_capture {(config.width,config.height)=capture.window_size;}
            self.surface.configure(&self.device, &config);
        }
        let frame = acquire(&self.surface)?;
        let record_start = Instant::now();
        let render_texture=self.media_capture.as_ref().map_or_else(||frame.texture.clone(),|c|c.texture.clone());
        let view=render_texture.create_view(&Default::default());
        let stereo=scene.stereoscopy && settings.postfx && scene.postfx.is_some() && scene.final_blit.is_some();
        let eyes=if stereo {
            let map=scene.map.as_ref().ok_or("stereo camera absent")?;
            let planes=scene.stereo_planes.ok_or("stereo camera planes absent")?;
            vec![Some(map.stereo_eye(planes,0)?),Some(map.stereo_eye(planes,1)?)]
        } else { vec![None] };
        let mut diagnostics=FrameDiagnostics::default();
        let mut map_wait=Duration::ZERO;
        let mut submit=Duration::ZERO;
        for eye in eyes {
        // The map's selection and runs encode on the render workers while this thread
        // records the frame's other commands.
        let runs = MapRuns {
            device: Arc::clone(&self.device),
            camera: self.camera_binding.clone(),
            target: self
                .msaa_color
                .as_ref()
                .unwrap_or(scene_color(
                    self.final_pass.as_ref(),
                    scene,
                    settings,
                    &view,
                ))
                .clone(),
            depth: self.depth.clone(),
            threads: self.map_threads,
        };
        self.marker_ui.set_eye(&self.queue,eye.as_ref());
        self.visual_indicators.set_eye(&self.queue,eye.as_ref());
        self.particles.set_eye(&self.queue,eye.as_ref());
        self.crowd.set_eye(&self.queue,eye.as_ref());
        if eye.is_some() || self.stereo_objects_active {
            for object in &scene.objects {
                let central=*object.central_clip.lock().map_err(|_| "object projection lock")?;
                let clip=eye.as_ref().map_or(central,|e|e.transform(central));
                let bytes:Vec<_>=clip.into_iter().flat_map(f32::to_ne_bytes).collect();
                self.character_uploads.write(&object.uniform,0,&bytes);
            }
        }
        let mut map = None;
        let (recorded, recorded_end) = rayon::in_place_scope(|scope| {
            if let Some(source) = &scene.map {
                let (runs, map) = (&runs, &mut map);
                scope.spawn(move |_| *map = Some(runs.encode(source,eye.as_ref())));
            }
            let recorded = self.record_frame(scene, settings, &render_texture, &view, if eye.is_some_and(|e|e.field==0) {None} else {capture}, eye.as_ref());
            // While the map's runs finish, the submissions the GPU has completed release
            // their resources here rather than in this frame's submit.
            let _ = self.device.poll(wgpu::PollType::Poll);
            (recorded, Instant::now())
        });
        map_wait += recorded_end.elapsed();
        let RecordedFrame {
            before,
            after,
            diagnostics: mut recorded_diagnostics,
        } = recorded?;
        let mut buffers = vec![before];
        if let Some(map) = map {
            buffers.extend(map.buffers);
            recorded_diagnostics.map = map.stats;
            recorded_diagnostics.map_prepare = map.selection;
        }
        buffers.push(after);
        let field_submit = Instant::now();
        // Preserve every constant update (including overlapping fog/exposure
        // writes) in order, before shadow and scene draws consume the buffers.
        let uploads = self.character_uploads.finish();
        self.queue.submit(uploads.into_iter().chain(buffers));
        if let Some(final_pass)=&mut self.final_pass {final_pass.after_submit();}
        self.shadows.submitted();
        if let Some(profile) = &mut self.gpu_profile {
            profile.submitted();
        }
        submit += field_submit.elapsed();
        diagnostics=recorded_diagnostics;
        }
        self.stereo_objects_active=stereo;
        if let Some(capture)=&self.media_capture {
            self.queue.submit(Some(capture.preview(&self.device,&self.queue,&frame.texture.create_view(&Default::default()))));
        }
        let record_duration=record_start.elapsed();
        let submit_start=Instant::now();
        if let Some(due) = scene.tick_due.filter(|_| mode != wgpu::PresentMode::Fifo) {
            self.present_pacing.wait(due);
        }
        let present_start = Instant::now();
        self.queue.present(frame);
        self.frame_cap.presented();
        let present = present_start.elapsed();
        self.fps.presented(&self.queue);
        self.last_frame = FrameDiagnostics {
            acquire: record_start.duration_since(acquire_start),
            record: record_duration.saturating_sub(submit),
            submit_present: submit_start.elapsed(),
            pass_end: diagnostics.pass_end + map_wait,
            submit,
            present,
            fps_counter: self.fps.value,
            ..diagnostics
        };
        Ok(())
    }

    /// The frame's commands on either side of the map's runs, which the render workers
    /// encode meanwhile.
    fn record_frame(
        &mut self,
        scene: &GpuScene,
        settings: skate2_core::ui::options::PcDisplaySettings,
        frame: &wgpu::Texture,
        view: &wgpu::TextureView,
        capture: Option<(&wgpu::Buffer, u32, bool)>,
        eye: Option<&stereoscopy::Eye>,
    ) -> RenderResult<RecordedFrame> {
        self.shadows.begin_frame();
        let gpu = self
            .gpu_profile
            .as_mut()
            .map_or_else(Default::default, |profile| profile.begin(&self.device));
        if let Some(map) = &scene.map {
            // The only per-frame map upload: every map draw shares these globals.
            self.queue
                .write_buffer(&self.camera, 0, &map.globals_for_eye((!settings.postfx).then_some(1.),eye));
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("diagnostic frame"),
            });
        // `RenderModeScene` 0x82857b40 renders both cascades before the scene.
        let (shadows, camera) = scene.map.as_ref().map_or_else(
            || (ShadowCascades::default(), [0.0; 3]),
            |map| (*map.shadows(), map.camera()),
        );
        // DMOs read the map's globals: without a map none draws or casts.
        let dynamic_objects: &[DynamicObjectDraw] = if scene.map.is_some() {
            &scene.dynamic_objects
        } else {
            &[]
        };
        let upload_start = Instant::now();
        for draw in dynamic_objects {
            draw.upload(&self.queue, camera);
        }
        let dynamic_upload = upload_start.elapsed();
        let dynamic_shadow_start = Instant::now();
        // `ShadowView::Cull` 0x82850ac0 per listed LivingWorld entity.
        let dynamic_casts = dynamic_objects
            .iter()
            .map(|draw| {
                crate::render_shadows::caster_cascades(
                    &shadows.rows,
                    draw.position(),
                    draw.world_bounds(),
                    camera,
                )
            })
            .collect::<Vec<_>>();
        if let Some(profile) = &self.gpu_profile {
            profile.stamp(&mut encoder, 0);
        }
        self.shadows.record(
            &self.queue,
            &mut encoder,
            &shadows,
            &scene.objects,
            shadow_map::DynamicCasters {
                draws: dynamic_objects,
                casts: &dynamic_casts,
            },
            camera,
        );
        let dynamic_shadows = dynamic_shadow_start.elapsed();
        if let Some(profile) = &self.gpu_profile {
            profile.stamp(&mut encoder, 1);
        }
        let world_shadow_start = Instant::now();
        let world_shadow_islands = if let Some(map) = &scene.map {
            self.shadows
                .record_world(&self.device, &self.queue, &mut encoder, map)?
        } else {
            0
        };
        let world_shadows = world_shadow_start.elapsed();
        if let Some(profile) = &self.gpu_profile {
            profile.stamp(&mut encoder, 2);
        }
        // `RenderCrowdTargets` 0x82857b90 follows the shadow cascades.
        self.crowd.record_targets(&mut encoder, &scene.crowd);
        if let Some(panel)=&scene.world_ui{self.marker_ui.record_panel(&mut encoder,panel);}
        let finish_start = Instant::now();
        let before = encoder.finish();
        let mut finish = finish_start.elapsed();
        let scene_start = Instant::now();
        let scene_view = scene_color(self.final_pass.as_ref(), scene, settings, view);
        // With MSAA the samples resolve into the swapchain image and are
        // then discarded; the overlay and capture read the resolved image.
        let target = self.msaa_color.as_ref().unwrap_or(scene_view);
        // A map's runs clear the scene targets; the individually bound objects
        // continue them.
        let cleared = scene.map.is_some();
        let object_draws;
        let pass_end_start;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("diagnostic frame objects"),
            });
        {
            let (resolve_target, store) = match &self.msaa_color {
                Some(_) => (Some(scene_view), wgpu::StoreOp::Discard),
                None => (None, wgpu::StoreOp::Store),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label: Some("original geometry diagnostic"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    depth_slice: None,
                    view: target,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: if cleared {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(SCENE_CLEAR)
                        },
                        store,
                    },
                })],
                depth_stencil_attachment: Some(scene_depth(&self.depth, !cleared)),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let objects_start = Instant::now();
            // SceneRenderView::Sort82839b38 stable-sorts every mesh context by
            // `InstanceSortKey`, whose first byte is the material draw order
            // (skater 9, hair 10); RenderMeshes82850670 then draws single-pass
            // meshes in that order and every multi-pass mesh pass-major, in the
            // same order, each only for the passes it has. The hair and a
            // ghosted skater part are multi-pass: every pass 0 runs before any
            // pass 1, and the hair's passes follow the skater's in each. The
            // effect/material scope bytes are not modeled here; equal draw
            // orders keep part order.
            // DMO default/ghost meshes (draw order8 or1) join the same stable
            // sort after the objects.
            let mut objects = scene.objects.iter().filter(|object| !object.hidden).collect::<Vec<_>>();
            objects.sort_by_key(|object| {
                object
                    .character
                    .as_ref()
                    .map_or(0, |character| character.draw_order)
            });
            let mut ordered = objects
                .iter()
                .map(|&object| {
                    let order = object
                        .character
                        .as_ref()
                        .map_or(0, |character| character.draw_order);
                    (order, SceneDraw::Object(object))
                })
                .chain(dynamic_objects.iter().flat_map(|draw| {
                    (0..draw.mesh_count())
                        .map(move |mesh| (draw.draw_order(mesh), SceneDraw::Dynamic(draw, mesh)))
                }))
                .collect::<Vec<_>>();
            ordered.sort_by_key(|(order, _)| *order);
            let characters = &self.character_pipelines;
            let technique = |object: &GpuObject| {
                object
                    .character
                    .as_ref()
                    .map(|character| characters.technique(character))
            };
            for (_, item) in &ordered {
                let object = match *item {
                    SceneDraw::Object(object) => object,
                    SceneDraw::Dynamic(draw, mesh) => {
                        if self.dynamic_objects.pass_count(draw, mesh) == 1 {
                            self.dynamic_objects.draw(
                                &mut pass,
                                &self.camera_binding,
                                draw,
                                mesh,
                                0,
                            );
                        }
                        continue;
                    }
                };
                match technique(object) {
                    Some([pipeline]) => pass.set_pipeline(pipeline),
                    Some(_) => continue,
                    None if object.texture_binding.is_some() => {
                        pass.set_pipeline(&self.textured_pipeline);
                    }
                    None => pass.set_pipeline(&self.pipeline),
                }
                draw_object(&mut pass, object);
            }
            let pass_count = ordered
                .iter()
                .map(|(_, item)| match *item {
                    SceneDraw::Object(object) => technique(object).map_or(0, <[_]>::len),
                    SceneDraw::Dynamic(draw, mesh) => self.dynamic_objects.pass_count(draw, mesh),
                })
                .max()
                .unwrap_or(0);
            for index in 0..pass_count {
                // RenderMeshes82850670: all multi-pass meshes in sorted order,
                // pass-major. DMO ghost meshes share this ordering with skaters.
                for (_, item) in &ordered {
                    let object = match *item {
                        SceneDraw::Object(object) => object,
                        SceneDraw::Dynamic(draw, mesh) => {
                            let count = self.dynamic_objects.pass_count(draw, mesh);
                            if count > 1 && index < count {
                                self.dynamic_objects.draw(
                                    &mut pass,
                                    &self.camera_binding,
                                    draw,
                                    mesh,
                                    index,
                                );
                            }
                            continue;
                        }
                    };
                    let Some(passes) = technique(object).filter(|passes| passes.len() > 1) else {
                        continue;
                    };
                    if let Some(pipeline) = passes.get(index) {
                        pass.set_pipeline(pipeline);
                        draw_object(&mut pass, object);
                    }
                }
            }
            // SceneRenderView::Render 0x828508f8 draws each visible entity's name after the meshes.
            if let Some(names) = &scene.online_names {
                self.marker_ui.draw_world(&mut pass, names);
            }
            // AptRenderingIntegration::Render3D, before PostFX and the flat UI.
            if let Some(panel)=&scene.world_ui{self.marker_ui.draw_panel(&mut pass,panel);}
            // `RenderCrowdImposters` 0x82857d04, after Render3D.
            self.crowd.draw_imposters(&mut pass, &scene.crowd);
            // ParticlePresentation::Draw at 0x82857d14, after Render3D and the crowd imposters.
            if let Some(frame) = &scene.particles {
                self.particles.draw(&mut pass, frame);
            }
            // RenderMan::RenderModeScene calls VIPresentation::Render at 0x82857d2c, after
            // Render3D and the particles, before PostFX::End.
            if let Some(frame) = &scene.visual_indicators {
                self.visual_indicators.draw(&mut pass, frame);
            }
            object_draws = objects_start.elapsed();
            pass_end_start = Instant::now();
        }
        let pass_end = pass_end_start.elapsed();
        let scene_time = scene_start.elapsed();
        let post_start = Instant::now();
        if let Some(profile) = &self.gpu_profile {
            profile.stamp(&mut encoder, 3);
        }
        if let Some((parameters,_))=&scene.final_blit {
            if let Some((assets,frame))=scene.postfx.as_ref().filter(|_| settings.postfx) {
                self.final_pass.as_mut().expect("initialized final pass").record_postfx(&self.device,&self.queue,&mut encoder,assets,frame,scene.deck_ao.as_ref(),eye)?;
            } else if let (Some((assets,frame)),Some(deck_ao),None)=(scene.postfx.as_ref(),scene.deck_ao.as_ref(),eye) {
                // The deck shadow belongs to the scene, so turning PostFX off keeps it.
                self.final_pass.as_mut().expect("initialized final pass").record_deck_ao(&self.device,&self.queue,&mut encoder,assets,frame,deck_ao);
            }
            if !eye.is_some_and(|e|e.field==0) {
            // ApplyFastUber 0x82846ca4 resolves the graded frame into the main target, which
            // the next frame's video screens sample.
            let final_pass = self.final_pass.as_ref().expect("initialized final pass");
            self.main_target.record(&mut encoder, final_pass.main_target_source());
            let mut parameters = *parameters;
            // Blit_ToDeviceRT 0x82831d5c..84: g_VignetteUVOffset.w takes the
            // first bloom coefficient when it is positive.
            if let Some((_,frame))=scene.postfx.as_ref().filter(|_| settings.postfx) {
                if let Some(hom) = frame.hom { parameters.vignette[1] = hom.vignette; }
                parameters.vignette[3]=crate::render_postfx_math::bloom_coefficients(frame.bloom[0])[0].max(0.);
                // SetParameters 0x8284bffc..c068: the ICE warp is the vault's
                // times fisheyeWarpMultiplier w, plus (0, 1 - w, 0, 0).
                if let Some(ice)=&frame.ice {
                    let w=ice.fisheye;
                    let [x,y,z,a]=parameters.warp;
                    parameters.warp=[x*w,y.mul_add(w,1.-w),z*w,a*w];
                }
            }
            let mut parameters = parameters.for_display(settings);
            parameters.supersample = self.scene_size(self.resolution_scale).0 > self.config.width;
            self.final_pass.as_mut().expect("initialized final pass").record(&self.queue,&mut encoder,view,&parameters,scene.out_of_boundary_amount,0.0,false)?;
            // Without PostFX its bloom to white has no pass to lower the bloom threshold in:
            // the NIS fades show as a white cover of the same value over the scene.
            if !settings.postfx && let Some(white) = &scene.nis_white {
                self.marker_ui.draw(&mut encoder, view, white);
            }
            }
        }
        if !eye.is_some_and(|e|e.field==0) {
        if let Some((buffer, stride, true)) = capture {
            let captured=if let Some(watermark)=scene.capture_watermark.as_ref().filter(|_|settings.postfx) {
                let texture=self.device.create_texture(&wgpu::TextureDescriptor {
                    label:Some("Replay media watermark target"),
                    size:wgpu::Extent3d {width:self.config.width,height:self.config.height,depth_or_array_layers:1},
                    mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:self.config.format,
                    usage:wgpu::TextureUsages::COPY_DST|wgpu::TextureUsages::COPY_SRC|wgpu::TextureUsages::RENDER_ATTACHMENT,view_formats:&[],
                });
                encoder.copy_texture_to_texture(frame.as_image_copy(),texture.as_image_copy(),texture.size());
                self.marker_ui.draw(&mut encoder,&texture.create_view(&Default::default()),watermark);
                Some(texture)
            } else {None};
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: captured.as_ref().unwrap_or(frame),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(self.config.height),
                    },
                },
                wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        // RenderModeScene82857ea4 -> FE.Render2D -> AptRendering.Render2D
        // 82631bc8..bd0 draws layer5 AFTER PostFX.Apply82857dec. Left-focus
        // HUD has a 3D projection, but is NOT the Render3D world-menu quad.
        // Applying scene fisheye/noise here again bends and darkens the HUD.
        let size = (self.config.width, self.config.height);
        // AptRenderingIntegration::Render2D (0x82631b70): a movie not in the foreground
        // renders before the APT levels, one in the foreground after them.
        // FrontEndManager::Render2D (0x82633be0) draws the chase arrows before
        // AptIntegration::Render2D.
        if let Some(arrows) = &scene.chase_arrows { self.marker_ui.draw(&mut encoder, view, arrows); }
        if let Some(movie) = scene.movie.filter(|movie| !movie.foreground) {
            self.movie.borrow().draw(&self.queue, &mut encoder, view, &movie, size);
        }
        if let Some(ui) = &scene.marker_ui { self.marker_ui.draw(&mut encoder, view, ui); }
        // FrontEndManager::RenderBackground82630dc0 covers the scene and APT.
        if let Some(fade) = &scene.frontend_fade { self.marker_ui.draw(&mut encoder, view, fade); }
        if let Some(movie) = scene.movie.filter(|movie| movie.foreground) {
            self.movie.borrow().draw(&self.queue, &mut encoder, view, &movie, size);
        }
        if self.display_settings.get().fps_counter {
            if let Some(fps) = &scene.fps_ui {
                self.marker_ui.draw(&mut encoder, view, fps);
            } else {
                self.fps.draw(&mut encoder, view, self.config.width, self.config.height);
            }
        }
        }
        if let Some(profile) = &self.gpu_profile {
            profile.stamp(&mut encoder, 4);
            profile.resolve(&mut encoder);
        }
        if let Some((buffer, stride, false)) = capture {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: frame,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(self.config.height),
                    },
                },
                wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let post = post_start.elapsed();
        let finish_start = Instant::now();
        let after = encoder.finish();
        finish += finish_start.elapsed();
        Ok(RecordedFrame {
            before,
            after,
            diagnostics: FrameDiagnostics {
                gpu,
                dynamic_upload,
                dynamic_shadows,
                world_shadows,
                scene: scene_time,
                objects: object_draws,
                pass_end,
                post,
                finish,
                world_shadow_islands,
                dynamic_objects: dynamic_objects.len(),
                ..FrameDiagnostics::default()
            },
        })
    }
}

fn write_bmp(
    path: &std::path::Path,
    width: u32,
    height: u32,
    stride: u32,
    pixels: &[u8],
    bgra: bool,
) -> RenderResult<()> {
    use std::io::Write;
    let image_size = width
        .checked_mul(height)
        .and_then(|size| size.checked_mul(4))
        .ok_or("BMP size overflow")?;
    let file_size = image_size.checked_add(54).ok_or("BMP file overflow")?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(b"BM")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(&[0; 4])?;
    file.write_all(&54_u32.to_le_bytes())?;
    file.write_all(&40_u32.to_le_bytes())?;
    file.write_all(&i32::try_from(width)?.to_le_bytes())?;
    file.write_all(&(-i32::try_from(height)?).to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&32_u16.to_le_bytes())?;
    file.write_all(&0_u32.to_le_bytes())?;
    file.write_all(&image_size.to_le_bytes())?;
    file.write_all(&[0; 16])?;
    for row in pixels
        .chunks_exact(usize::try_from(stride)?)
        .take(usize::try_from(height)?)
    {
        let row = &row[..usize::try_from(width)? * 4];
        if bgra {
            file.write_all(row)?;
        } else {
            for pixel in row.chunks_exact(4) {
                file.write_all(&[pixel[2], pixel[1], pixel[0], pixel[3]])?;
            }
        }
    }
    Ok(())
}

fn create_texture_bindings(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::Sampler) {
    let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("original diffuse diagnostic layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("diagnostic diffuse repeat/linear; not resolved original sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        // Presentation enhancement; wgpu clamps to 16 and uses 1 on hardware
        // without anisotropic filtering.
        anisotropy_clamp: 16,
        ..Default::default()
    });
    (texture_layout, sampler)
}

/// The same GPU allocation service used by immediate and streamed textures.
/// Clones retain the device/queue, never a second renderer or surface.
#[derive(Clone)]
pub(crate) struct GpuTextureUploader {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture_layout: Arc<wgpu::BindGroupLayout>,
    sampler: Arc<wgpu::Sampler>,
}

impl GpuTextureUploader {
    /// Uploads the existing endian-normalized row-major mip blocks exactly once.
    /// Cube faces become array layers in D3D face order (+X, -X, +Y, -Y, +Z,
    /// -Z), the order both the Xbox 360 surface and wgpu cube views use.
    pub fn upload_native_texture(&self, source: &NativeTexture) -> RenderResult<GpuTexture> {
        let (mut format, block, block_bytes) = native_texture_layout(source)?;
        // Original APT slider skins include 16x2 BC3 images. wgpu requires
        // block-aligned BC allocations; expand only these exceptional images
        // to RGBA without padding/resizing their logical dimensions or UVs.
        let expand_bc = block == 4 && (source.width % 4 != 0 || source.height % 4 != 0);
        if expand_bc { format = wgpu::TextureFormat::Rgba8Unorm; }
        let cube = source.texture_type == TextureType::Cube;
        if block == 4 && !expand_bc
            && !self
                .device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
        {
            return Err("adapter lacks original BC texture compression support".into());
        }
        validate_texture_levels(
            source,
            block,
            block_bytes,
            self.device.limits().max_texture_dimension_2d,
        )?;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&source.name),
            size: wgpu::Extent3d {
                width: source.width,
                height: source.height,
                depth_or_array_layers: if cube { 6 } else { 1 },
            },
            mip_level_count: source.mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for level in &source.linear_levels {
            let expanded;
            let (data, row_pitch) = if expand_bc {
                expanded = texture_bc::expand(
                    level,
                    matches!(source.pixel_format, PIXEL_FORMAT_DXT5 | PIXEL_FORMAT_LIN_DXT5),
                )?;
                (expanded.as_slice(), level.width.checked_mul(4).ok_or("BC expansion pitch overflow")?)
            } else if source.pixel_format == PIXEL_FORMAT_R5G6B5 {
                expanded = expand_rgb565(&level.data);
                (
                    expanded.as_slice(),
                    level
                        .row_pitch
                        .checked_mul(2)
                        .ok_or("RGB565 upload pitch overflow")?,
                )
            } else {
                (level.data.as_slice(), level.row_pitch)
            };
            // Compressed mip copies use physical block-rounded dimensions.
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level.level,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: level.face,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_pitch),
                    rows_per_image: Some(if expand_bc { level.height } else { level.height_in_blocks }),
                },
                wgpu::Extent3d {
                    width: if expand_bc { level.width } else { level.width_in_blocks * block },
                    height: if expand_bc { level.height } else { level.height_in_blocks * block },
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(if cube {
                wgpu::TextureViewDimension::Cube
            } else {
                wgpu::TextureViewDimension::D2
            }),
            ..Default::default()
        });
        // One diffuse binding per 2D texture, shared by every object that
        // samples it, instead of a new view and bind group per draw object.
        let binding = (!cube).then(|| {
            Arc::new(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("actual diffuse texture diagnostic"),
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }))
        });
        Ok(GpuTexture {
            view: Arc::new(view),
            binding,
            cube,
        })
    }
}

fn native_texture_layout(source: &NativeTexture) -> RenderResult<(wgpu::TextureFormat, u32, u32)> {
    if !matches!(
        source.texture_type,
        TextureType::TwoDimensional | TextureType::Cube
    ) || source.depth != 1
    {
        return Err("only actual single-layer 2D and cube textures are admitted".into());
    }
    match source.pixel_format {
        PIXEL_FORMAT_A8R8G8B8 => Ok((wgpu::TextureFormat::Bgra8Unorm, 1, 4)),
        PIXEL_FORMAT_DXT1 => Ok((wgpu::TextureFormat::Bc1RgbaUnorm, 4, 8)),
        PIXEL_FORMAT_DXT5 | PIXEL_FORMAT_LIN_DXT5 => Ok((wgpu::TextureFormat::Bc3RgbaUnorm, 4, 16)),
        // IDA renderengine::PixelFormat names 0x28280144 R5G6B5.
        // wgpu has no packed565 sampled format. Validate original 2-byte
        // texels, then expand to the nearest 8-bit UNORM values for upload.
        PIXEL_FORMAT_R5G6B5 => Ok((wgpu::TextureFormat::Rgba8Unorm, 1, 2)),
        format => Err(format!("unsupported original texture format {format:#x}").into()),
    }
}

fn expand_rgb565(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len() * 2);
    for word in bytes.chunks_exact(2) {
        // The existing texture decoder already normalizes Xbox surface endian.
        let pixel = u16::from_le_bytes([word[0], word[1]]);
        result.extend_from_slice(&[
            (((pixel >> 11) * 255 + 15) / 31) as u8,
            ((((pixel >> 5) & 63) * 255 + 31) / 63) as u8,
            (((pixel & 31) * 255 + 15) / 31) as u8,
            255,
        ]);
    }
    result
}

fn validate_texture_levels(
    source: &NativeTexture,
    block: u32,
    block_bytes: u32,
    max_dimension: u32,
) -> RenderResult<()> {
    // `Texture::decode_levels` emits every face of a level, sorted by
    // (level, face): six square faces per level for cube textures.
    let faces: u32 = if source.texture_type == TextureType::Cube {
        6
    } else {
        1
    };
    if source.width == 0
        || source.height == 0
        || source.width > max_dimension
        || source.height > max_dimension
        || (faces == 6 && source.width != source.height)
        || source.mip_levels == 0
        || source.mip_levels > 32 - source.width.max(source.height).leading_zeros()
        || source.linear_levels.len() != usize::try_from(source.mip_levels * faces)?
    {
        return Err("invalid original texture dimensions/mip/face count".into());
    }
    // Edge blocks are valid original data, including sub-block base levels.
    // The uploader expands those bases to RGBA for wgpu, without resizing.
    for (index, level) in source.linear_levels.iter().enumerate() {
        let index = u32::try_from(index)?;
        let (mip, face) = (index / faces, index % faces);
        let width = (source.width >> mip).max(1);
        let height = (source.height >> mip).max(1);
        let blocks_x = width.div_ceil(block);
        let blocks_y = height.div_ceil(block);
        let row = blocks_x
            .checked_mul(block_bytes)
            .ok_or("texture pitch overflow")?;
        let size = usize::try_from(row)?
            .checked_mul(usize::try_from(blocks_y)?)
            .ok_or("texture payload size overflow")?;
        if level.level != mip
            || level.face != face
            || level.width != width
            || level.height != height
            || level.width_in_blocks != blocks_x
            || level.height_in_blocks != blocks_y
            || level.bytes_per_block != block_bytes
            || level.row_pitch != row
            || level.data.len() != size
        {
            return Err(
                format!("invalid original texture mip {mip} face {face} layout/payload").into(),
            );
        }
    }
    Ok(())
}

fn create_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    sample_count: u32,
    bindings: &[Option<&wgpu::BindGroupLayout>],
    fragment: &str,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("diagnostic pipeline layout"),
        bind_group_layouts: bindings,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        cache: None,
        label: Some("diagnostic indexed original geometry"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            compilation_options: Default::default(),
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 20,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 12,
                        shader_location: 1,
                    },
                ],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            compilation_options: Default::default(),
            module: shader,
            entry_point: Some(fragment),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: multisample(sample_count),
        multiview_mask: None,
    })
}

// XBox2GraphicsMan.Init8282c3d8 selects PIXELFORMAT_D24S8. Stencil must be
// present and writable for the original DMO ghost technique's second pass.
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// Main-pass pipelines share one sample count; alpha-to-coverage stays off so
/// alpha-tested foliage keeps its authored discard threshold unchanged.
fn multisample(sample_count: u32) -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count: sample_count,
        mask: !0,
        alpha_to_coverage_enabled: false,
    }
}

#[cfg(test)]
mod record_tests {
    use super::uniform_bytes;

    #[test]
    fn records_preserve_float_bits() {
        let matrix = std::array::from_fn(|index| {
            f32::from_bits(0x7fc0_0001 + u32::try_from(index).expect("matrix index"))
        });
        let color = [0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY];
        let expected: Vec<_> = matrix
            .iter()
            .chain(color.iter())
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        assert_eq!(
            uniform_bytes(&matrix, &color).as_slice(),
            expected.as_slice()
        );
    }
}

/// One main-pass draw in material draw order.
enum SceneDraw<'a> {
    Object(&'a GpuObject),
    /// A DMO and one of its meshes.
    Dynamic(&'a DynamicObjectDraw, usize),
}

/// Binds one object's groups and buffers and issues its original draws; the
/// caller has already selected the pass pipeline.
/// The former linear clear on the sRGB surface, now in the gamma-2 output space
/// (sqrt of 0.025/0.035/0.05).
const SCENE_CLEAR: wgpu::Color = wgpu::Color {
    r: 0.158_113_883,
    g: 0.187_082_869,
    b: 0.223_606_798,
    a: 1.0,
};

/// The view the scene draws into: the final pass's scene texture, which its blit then
/// reads, or else the swapchain image.
fn scene_color<'a>(
    final_pass: Option<&'a final_pass::FinalPass>,
    scene: &GpuScene,
    settings: skate2_core::ui::options::PcDisplaySettings,
    view: &'a wgpu::TextureView,
) -> &'a wgpu::TextureView {
    if scene.final_blit.is_some() {
        final_pass
            .expect("initialized final pass")
            .scene_view(settings.postfx && scene.postfx.is_some())
    } else {
        view
    }
}

/// The scene's depth/stencil, which the frame's first scene pass clears:
/// `RenderModeScene` 0x82857c50 clears the original scene depth/stencil target,
/// and DMO ghost pass 1 writes stencil 255.
fn scene_depth(
    view: &wgpu::TextureView,
    clear: bool,
) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: Some(wgpu::Operations {
            load: if clear {
                wgpu::LoadOp::Clear(1.0)
            } else {
                wgpu::LoadOp::Load
            },
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: Some(wgpu::Operations {
            load: if clear {
                wgpu::LoadOp::Clear(0)
            } else {
                wgpu::LoadOp::Load
            },
            store: wgpu::StoreOp::Store,
        }),
    }
}

/// The runs a frame cuts the map's draws into (`SKATE2_MAP_THREADS`), and the render
/// worker pool that encodes them: one thread more, for the job that selects and cuts
/// them. A larger pool only spins.
fn render_workers() -> usize {
    let threads = std::env::var("SKATE2_MAP_THREADS")
        .ok()
        .and_then(|threads| threads.parse().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map_or(1, |threads| (threads.get() / 4).clamp(1, 4))
        });
    // Built once per process; a renderer made again keeps the first pool.
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(threads + 1)
        .thread_name(|index| format!("render worker {index}"))
        .build_global();
    threads
}

/// The fewest draws a map run takes: a thread for fewer costs more than it saves.
const MAP_RUN_DRAWS: usize = 500;

/// The scene targets and threads a frame encodes the map's runs with.
struct MapRuns {
    device: Arc<wgpu::Device>,
    camera: wgpu::BindGroup,
    target: wgpu::TextureView,
    depth: wgpu::TextureView,
    threads: usize,
}

/// The map's share of a frame, encoded on the render workers.
struct EncodedMap {
    /// The runs' command buffers in draw order.
    buffers: Vec<wgpu::CommandBuffer>,
    stats: MapFrameStats,
    /// Selecting the view's islands, when the view changed.
    selection: Duration,
}

impl MapRuns {
    /// The view's selection cut into runs between objects, each recorded and encoded
    /// on a render worker into a pass and command buffer of its own; the first clears
    /// the scene targets, even with nothing selected.
    fn encode(&self, map: &GpuMap, eye:Option<&stereoscopy::Eye>) -> EncodedMap {
        let selection_start = Instant::now();
        let per_eye=eye.map(|e|map.select_visible_for(&e.clip));
        let visible = per_eye.as_deref().unwrap_or_else(||map.visible());
        let selection = selection_start.elapsed();
        let source = map.draw_source();
        let count = self.threads.min(visible.len().div_ceil(MAP_RUN_DRAWS));
        let runs = map_draw::draw_runs(visible, count);
        let encode = |index: usize, run: Range<usize>| {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("map run"),
                });
            let stats = {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    multiview_mask: None,
                    label: Some("map run"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        depth_slice: None,
                        view: &self.target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if index == 0 {
                                wgpu::LoadOp::Clear(SCENE_CLEAR)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(scene_depth(&self.depth, index == 0)),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                source.record_draws(&mut pass, &self.camera, &visible[run])
            };
            (encoder.finish(), stats)
        };
        let encode = &encode;
        let mut encoded: Vec<_> = runs.iter().map(|_| None).collect();
        rayon::scope(|scope| {
            for ((index, run), slot) in runs.iter().enumerate().zip(&mut encoded) {
                let run = run.clone();
                scope.spawn(move |_| *slot = Some(encode(index, run)));
            }
        });
        let mut stats = MapFrameStats::default();
        let buffers = encoded
            .into_iter()
            .flatten()
            .map(|(buffer, run)| {
                stats.add(run);
                buffer
            })
            .collect();
        EncodedMap {
            buffers,
            stats,
            selection,
        }
    }
}

/// Why a frame could not take the swapchain's next image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceUnavailable {
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

impl std::fmt::Display for SurfaceUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "swapchain image unavailable: {self:?}")
    }
}

impl Error for SurfaceUnavailable {}

/// The swapchain's next image; a suboptimal one still draws.
fn acquire(surface: &wgpu::Surface<'_>) -> Result<wgpu::SurfaceTexture, SurfaceUnavailable> {
    match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(texture)
        | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Ok(texture),
        wgpu::CurrentSurfaceTexture::Timeout => Err(SurfaceUnavailable::Timeout),
        wgpu::CurrentSurfaceTexture::Occluded => Err(SurfaceUnavailable::Occluded),
        wgpu::CurrentSurfaceTexture::Outdated => Err(SurfaceUnavailable::Outdated),
        wgpu::CurrentSurfaceTexture::Lost => Err(SurfaceUnavailable::Lost),
        wgpu::CurrentSurfaceTexture::Validation => Err(SurfaceUnavailable::Validation),
    }
}

fn draw_object<'a>(pass: &mut wgpu::RenderPass<'a>, object: &'a GpuObject) {
    if let Some(character) = &object.character {
        // Original character/livingworld manifests: pass STENCILREF=255.
        pass.set_stencil_reference(255);
        pass.set_bind_group(2, &character.binding, &[]);
        if let Some(alpha) = &character.alpha {
            pass.set_bind_group(3, alpha, &[]);
        }
        if let Some(stamp) = &character.stamp {
            pass.set_bind_group(3, &stamp.group, &[]);
        }
    }
    if let Some(texture) = &object.texture_binding {
        pass.set_bind_group(1, texture.as_ref(), &[]);
    }
    pass.set_bind_group(0, &object.binding, &[]);
    pass.set_vertex_buffer(0, object.mesh.vertices.slice(..));
    pass.set_index_buffer(object.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
    for (range, base) in object.mesh.draws.iter() {
        pass.draw_indexed(range.clone(), *base, 0..1);
    }
}

fn uniform_bytes(matrix: &[f32; 16], color: &[f32; 4]) -> [u8; 80] {
    // Fixed GPU layout: preserve each float's exact bytes without allocating
    // a temporary Vec for every map object on every completed frame.
    let mut bytes = [0; 80];
    for (destination, value) in bytes.chunks_exact_mut(4).zip(matrix.iter().chain(color)) {
        destination.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

/// The surface mode for VSync: on, each frame waits for the refresh (Fifo); off, frames present
/// at once (Immediate, else Mailbox), the Frame Rate Cap's after `FrameCap`'s wait. Mailbox is
/// not uncapped once the surface is reconfigured: on the NVIDIA Vulkan driver it then runs at
/// most two frames per refresh.
fn present_mode(supported: &[wgpu::PresentMode], vsync: bool) -> wgpu::PresentMode {
    if vsync {
        wgpu::PresentMode::Fifo
    } else {
        [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox]
            .into_iter()
            .find(|mode| supported.contains(mode))
            .unwrap_or(wgpu::PresentMode::Fifo)
    }
}

/// The depth attachment and its depth-only view, which the NIS grade
/// (`visualFX_PS`'s `depth_texture`) reads.
fn depth_view(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("diagnostic depth"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let sample = texture.create_view(&wgpu::TextureViewDescriptor {
        aspect: wgpu::TextureAspect::DepthOnly,
        ..Default::default()
    });
    let stencil = texture.create_view(&wgpu::TextureViewDescriptor {
        aspect: wgpu::TextureAspect::StencilOnly,
        ..Default::default()
    });
    (texture.create_view(&wgpu::TextureViewDescriptor::default()), sample, stencil)
}

fn msaa_view(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> Option<wgpu::TextureView> {
    (sample_count > 1).then(|| {
        render_target(
            device,
            config,
            sample_count,
            config.format,
            "multisampled color",
        )
    })
}

fn render_target(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

const SHADER: &str = r"
struct Object { clip_from_local: mat4x4<f32>, color: vec4<f32> }
@group(0) @binding(0) var<uniform> object: Object;
@group(1) @binding(0) var diffuse_texture: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;
struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = object.clip_from_local * vec4<f32>(position, 1.0);
    output.uv = uv;
    return output;
}
@fragment fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> { return object.color; }
@fragment fn fs_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse = textureSample(diffuse_texture, diffuse_sampler, input.uv);
    // Diagnostic display on the gamma-2 (non-sRGB) surface the original
    // effects write: raw diffuse, no lighting or tone map.
    return diffuse * object.color;
}
";

#[cfg(test)]
mod tests {
    use super::{CameraRelativeClip, native_texture_layout, validate_texture_levels};
    use skate2_core::texture::NativeTexture;
    use skate2_formats::material::{
        LinearTextureLevel, PIXEL_FORMAT_DXT1, TextureEndian, TextureType,
    };

    #[test]
    fn camera_relative_clip_matches_the_world_clip() {
        let clip = [
            0.8, 0.1, 0.2, 0.3, -0.1, 1.2, 0.05, 0.02, 0.6, -0.2, 1.00005, 1.0, 2800.0, -400.0,
            3500.0, 3480.0,
        ];
        let camera = [-3500.25, 1.75, -900.5];
        let relative = CameraRelativeClip::new(&clip, camera);
        assert_eq!(relative.camera, camera);
        assert_eq!(&relative.clip_from_camera[..12], &clip[..12]);
        let point = [-3490.0_f64, 2.5, -895.0];
        for row in 0..4 {
            let world = (0..3).fold(f64::from(clip[12 + row]), |sum, axis| {
                sum + f64::from(clip[axis * 4 + row]) * point[axis]
            });
            let local = (0..3).fold(
                f64::from(relative.clip_from_camera[12 + row]),
                |sum, axis| {
                    sum + f64::from(clip[axis * 4 + row]) * (point[axis] - f64::from(camera[axis]))
                },
            );
            assert!(
                (world - local).abs() < 1e-3,
                "row {row}: {world} vs {local}"
            );
        }
    }

    #[test]
    fn rgb565_upload_uses_normalized_endian_rgb_and_opaque_alpha() {
        assert_eq!(
            super::expand_rgb565(&[0, 0, 0, 0xf8, 0xe0, 7, 31, 0, 255, 255]),
            [
                0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255
            ]
        );
        for bits in 0..=u16::MAX {
            let expanded = super::expand_rgb565(&bits.to_le_bytes());
            for (i, original, maximum) in [
                (0, bits >> 11, 31),
                (1, (bits >> 5) & 63, 63),
                (2, bits & 31, 31),
            ] {
                let error = (f64::from(expanded[i]) / 255.0
                    - f64::from(original) / f64::from(maximum))
                .abs();
                assert!(error <= 0.5 / 255.0);
            }
        }
    }

    fn bc_mip_chain() -> NativeTexture {
        NativeTexture {
            dictionary_index: 0,
            guid: 1,
            name: "synthetic BC tail".to_owned(),
            texture_type: TextureType::TwoDimensional,
            endian: TextureEndian::EightInSixteen,
            width: 4,
            height: 4,
            depth: 1,
            mip_levels: 3,
            pixel_format: PIXEL_FORMAT_DXT1,
            packed_mips: true,
            mip_data_offset: 0,
            encoded_data: Vec::new(),
            linear_levels: (0..3)
                .map(|level| LinearTextureLevel {
                    level,
                    face: 0,
                    width: 4 >> level,
                    height: 4 >> level,
                    width_in_blocks: 1,
                    height_in_blocks: 1,
                    bytes_per_block: 8,
                    row_pitch: 8,
                    data: vec![0; 8],
                })
                .collect(),
        }
    }

    #[test]
    fn admits_sub_block_mips_without_resizing_original_uv_domain() {
        let source = bc_mip_chain();
        assert_eq!(
            native_texture_layout(&source).unwrap(),
            (wgpu::TextureFormat::Bc1RgbaUnorm, 4, 8)
        );
        validate_texture_levels(&source, 4, 8, 8192).unwrap();
        assert_eq!(source.linear_levels[2].width, 1);
        assert_eq!(source.linear_levels[2].data.len(), 8);
    }

    #[test]
    fn admits_cube_faces_in_level_face_order_only() {
        let mut source = bc_mip_chain();
        source.texture_type = TextureType::Cube;
        assert!(native_texture_layout(&source).is_ok());
        // A 2D-shaped record set is not a cube.
        assert!(validate_texture_levels(&source, 4, 8, 8192).is_err());
        source.linear_levels = (0..3)
            .flat_map(|level| (0..6).map(move |face| (level, face)))
            .map(|(level, face)| LinearTextureLevel {
                face,
                ..bc_mip_chain().linear_levels[usize::try_from(level).unwrap()].clone()
            })
            .collect();
        validate_texture_levels(&source, 4, 8, 8192).unwrap();
        source.linear_levels.swap(1, 2);
        assert!(validate_texture_levels(&source, 4, 8, 8192).is_err());
        source.linear_levels.swap(1, 2);
        source.height = 8;
        assert!(validate_texture_levels(&source, 4, 8, 8192).is_err());
        source.texture_type = TextureType::ThreeDimensional;
        assert!(native_texture_layout(&source).is_err());
    }

    #[test]
    fn admits_sub_block_base_for_rgba_upload_without_relaxing_payload_checks() {
        let mut source = bc_mip_chain();
        source.width = 2;
        source.height = 2;
        source.mip_levels = 2;
        source.linear_levels.remove(0);
        for level in &mut source.linear_levels { level.level -= 1; }
        validate_texture_levels(&source, 4, 8, 8192).unwrap();
        source.linear_levels[0].data.pop();
        assert!(validate_texture_levels(&source, 4, 8, 8192).is_err());
    }

    #[test]
    fn rejects_misplaced_faces_missing_mips_and_truncated_blocks() {
        let mut source = bc_mip_chain();
        source.linear_levels[1].face = 1;
        assert!(validate_texture_levels(&source, 4, 8, 8192).is_err());
        source.linear_levels[1].face = 0;
        source.linear_levels[2].data.pop();
        assert!(validate_texture_levels(&source, 4, 8, 8192).is_err());
        source.linear_levels.pop();
        assert!(validate_texture_levels(&source, 4, 8, 8192).is_err());
    }
}

/// Cloneable creation-only device context. Workers never own or configure the surface.
#[derive(Clone)]
pub(crate) struct GpuResourceBuilder {
    mesh_arena: Arc<std::sync::Mutex<mesh_arena::MeshArena>>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    object_layout: Arc<wgpu::BindGroupLayout>,
    texture_layout: Arc<wgpu::BindGroupLayout>,
    sampler: Arc<wgpu::Sampler>,
    map_pipelines: Arc<map_draw::MapPipelines>,
    /// The renderer's CSM atlas, bound as `shadowTexture_CSM0`.
    shadow_atlas: Arc<wgpu::TextureView>,
    /// `PostFX`'s main render target, the `videoscreen` materials' `i_diffuse`.
    main_target: GpuTexture,
}
impl GpuRenderer {
    pub(crate) fn resource_builder(&self) -> GpuResourceBuilder {
        GpuResourceBuilder {
            mesh_arena: Arc::clone(&self.mesh_arena),
            device: self.device.clone(),
            queue: self.queue.clone(),
            object_layout: self.object_layout.clone(),
            texture_layout: self.texture_layout.clone(),
            sampler: self.sampler.clone(),
            map_pipelines: self.map_pipelines.clone(),
            shadow_atlas: Arc::clone(self.shadows.atlas()),
            main_target: self.main_target.texture(),
        }
    }
}
impl GpuResourceBuilder {
    /// `PostFX`'s main render target, which `cModelInstance::Resolve` binds as the
    /// `i_diffuse` of a `videoscreen_default` scope.
    pub(crate) fn main_target(&self) -> GpuTexture {
        self.main_target.clone()
    }

    pub fn upload_mesh(&self, geometry: &NativeMeshGeometry) -> RenderResult<GpuMesh> {
        self.upload_mesh_storage(geometry, true, false)
    }

    /// World shader fetches supply (0,0,0,1) for an absent declaration
    /// element: D3D::PatchVertexShaderToMatchVertexDeclaration82c43eb0..ebc.
    pub(crate) fn upload_world_mesh(&self, geometry: &NativeMeshGeometry) -> RenderResult<GpuMesh> {
        self.upload_mesh_storage(geometry, true, true)
    }

    /// Character upload replaces the vertex stream with a different stride.
    /// Its draw offsets must remain local to that dedicated replacement.
    pub(super) fn upload_mesh_dedicated(
        &self,
        geometry: &NativeMeshGeometry,
    ) -> RenderResult<GpuMesh> {
        self.upload_mesh_storage(geometry, false, false)
    }

    fn upload_mesh_storage(
        &self,
        geometry: &NativeMeshGeometry,
        shared: bool,
        world_vertex_defaults: bool,
    ) -> RenderResult<GpuMesh> {
        if geometry.positions.is_empty() || geometry.indices.is_empty() {
            return Err("cannot upload empty mesh".into());
        }
        let mut draws = Vec::with_capacity(geometry.draws.len());
        let mut island_bounds = Vec::with_capacity(geometry.draws.len());
        let mut all_islands_bounded = true;
        for draw in &geometry.draws {
            if draw.primitive_type != PRIMITIVE_TYPE_TRIANGLE_LIST {
                return Err("diagnostic backend requires original triangle-list draws".into());
            }
            let end = draw
                .start_index
                .checked_add(draw.index_count)
                .ok_or("index range overflow")?;
            let indices = geometry
                .indices
                .get(usize::try_from(draw.start_index)?..usize::try_from(end)?)
                .ok_or("draw outside original index buffer")?;
            // Island culling bounds come from the positions this draw actually
            // references, collected during the existing index admission pass.
            let mut island = map_draw::BoundsBuilder::default();
            for &index in indices {
                let vertex = i64::from(index) + i64::from(draw.base_vertex_index);
                if vertex < 0 || usize::try_from(vertex)? >= geometry.positions.len() {
                    return Err("draw base vertex references missing position".into());
                }
                island.include(geometry.positions[usize::try_from(vertex)?]);
            }
            let empty = island.is_empty();
            let bounds = island.finish();
            all_islands_bounded &= empty || bounds.is_some();
            island_bounds.push(bounds);
            draws.push((draw.start_index..end, draw.base_vertex_index));
        }
        let uv = geometry
            .attributes
            .iter()
            .find(|a| a.semantic == VertexSemantic::TextureCoordinate(0));
        let uv = uv.and_then(|attribute| match &attribute.values {
            NativeVertexAttributeValues::Float2(values) => Some(values.clone()),
            NativeVertexAttributeValues::Float3(values) => {
                Some(values.iter().map(|v| [v[0], v[1]]).collect())
            }
            NativeVertexAttributeValues::Float4(values) => {
                Some(values.iter().map(|v| [v[0], v[1]]).collect())
            }
            _ => None,
        });
        if uv
            .as_ref()
            .is_some_and(|values| values.len() != geometry.positions.len())
        {
            return Err("UV0 count differs from original positions".into());
        }
        let mut vertices = Vec::with_capacity(geometry.positions.len() * 20);
        for (index, position) in geometry.positions.iter().enumerate() {
            vertices.extend(position.iter().flat_map(|v| v.to_ne_bytes()));
            // Unused padding for the solid pipeline; textured admission rejects missing UV.
            let value = uv.as_ref().map_or([0.0; 2], |values| values[index]);
            vertices.extend(value.iter().flat_map(|v| v.to_ne_bytes()));
        }
        let indices = geometry
            .indices
            .iter()
            .flat_map(|v| v.to_ne_bytes())
            .collect::<Vec<_>>();
        let lighting = lighting_stream(geometry, world_vertex_defaults)?;
        let allocation = if shared {
            self.mesh_arena
                .lock()
                .map_err(|_| "GPU geometry arena poisoned")?
                .upload(
                    &self.device,
                    &self.queue,
                    &vertices,
                    &indices,
                    lighting.as_ref().map(|stream| stream.bytes.as_slice()),
                )?
        } else {
            None
        };
        let (vertex_buffer, index_buffer, lighting_buffer) = if let Some(allocation) = allocation {
            for (range, base) in &mut draws {
                range.start = range
                    .start
                    .checked_add(allocation.index_base)
                    .ok_or("packed index start overflow")?;
                range.end = range
                    .end
                    .checked_add(allocation.index_base)
                    .ok_or("packed index end overflow")?;
                *base = base
                    .checked_add(allocation.vertex_base)
                    .ok_or("packed base vertex overflow")?;
            }
            (allocation.vertices, allocation.indices, allocation.lighting)
        } else {
            let upload = |label, bytes: &[u8], usage| {
                Arc::new(
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(label),
                            contents: bytes,
                            usage,
                        }),
                )
            };
            (
                upload("original positions", &vertices, wgpu::BufferUsages::VERTEX),
                upload("original indices", &indices, wgpu::BufferUsages::INDEX),
                lighting.as_ref().map(|stream| {
                    upload(
                        "original TEXCOORD1/TANGENT",
                        &stream.bytes,
                        wgpu::BufferUsages::VERTEX,
                    )
                }),
            )
        };
        Ok(GpuMesh {
            vertices: vertex_buffer,
            indices: index_buffer,
            draws: draws.into(),
            has_uv: uv.is_some(),
            lightmap_components: lighting.as_ref().map_or(0, |stream| stream.components),
            has_tangent: lighting.as_ref().is_some_and(|stream| stream.tangent),
            has_normal: lighting.as_ref().is_some_and(|stream| stream.normal),
            has_decal_uv: lighting.as_ref().is_some_and(|stream| stream.decal_uv),
            lighting: lighting_buffer,
            bounds: map_draw::union_of_islands(&island_bounds, all_islands_bounded),
            island_bounds: island_bounds.into(),
        })
    }
    pub fn create_object(&self, mesh: GpuMesh, matrix: &[f32; 16], color: &[f32; 4]) -> GpuObject {
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("diagnostic transform/color"),
                contents: &uniform_bytes(matrix, color),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diagnostic object"),
            layout: &self.object_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        GpuObject {
            central_clip: std::sync::Mutex::new(*matrix),
            mesh,
            uniform,
            binding,
            texture_binding: None,
            character: None,
            hidden: false,
        }
    }
    /// See `MapPipelines::create_diagnostic_pipeline`; map draws only.
    pub(crate) fn create_diagnostic_pipeline(
        &self,
        fragment: DiagnosticFragment,
        raster: RasterState,
    ) -> Arc<wgpu::RenderPipeline> {
        self.map_pipelines
            .create_diagnostic_pipeline(&self.device, fragment, raster)
    }
    pub fn create_textured_object(
        &self,
        mesh: GpuMesh,
        matrix: &[f32; 16],
        color: &[f32; 4],
        texture: &GpuTexture,
    ) -> RenderResult<GpuObject> {
        if !mesh.has_uv {
            return Err("textured mesh lacks supported original UV0".into());
        }
        let binding = texture
            .binding
            .as_ref()
            .ok_or("diagnostic diffuse cannot be a cube map")?;
        let mut object = self.create_object(mesh, matrix, color);
        object.texture_binding = Some(Arc::clone(binding));
        Ok(object)
    }
    /// `g_fresnelLookupTexture` plus the CSM atlas, shared by one map
    /// generation's original-effect draws.
    pub(crate) fn create_environment_resources(
        &self,
        fresnel_texels: &[u8],
    ) -> RenderResult<Arc<wgpu::BindGroup>> {
        if fresnel_texels.len() != 256 * 4 {
            return Err("fresnel lookup must be 256 RGBA8 texels".into());
        }
        Ok(Arc::new(self.map_pipelines.environment.resources(
            &self.device,
            &self.queue,
            fresnel_texels,
            &self.shadow_atlas,
        )))
    }
    /// One material's sampler set in `EnvironmentEffect::channels` order.
    /// Only the `environment` channel may be (and must be) a cube map.
    pub(crate) fn create_environment_material(
        &self,
        effect: EnvironmentEffect,
        textures: &[&GpuTexture],
    ) -> RenderResult<Arc<wgpu::BindGroup>> {
        let channels = effect.channels();
        if textures.len() != channels.len() {
            return Err("environment material channel count differs from its effect".into());
        }
        for (texture, channel) in textures.iter().zip(channels) {
            if texture.cube != (*channel == "environment") {
                return Err(
                    format!("environment channel {channel} has the wrong texture type").into(),
                );
            }
        }
        let views = textures
            .iter()
            .map(|texture| &*texture.view)
            .collect::<Vec<_>>();
        Ok(Arc::new(self.map_pipelines.environment.material(
            &self.device,
            effect,
            &views,
        )))
    }
    pub(crate) fn texture_uploader(&self) -> GpuTextureUploader {
        GpuTextureUploader {
            device: Arc::clone(&self.device),
            queue: Arc::clone(&self.queue),
            texture_layout: Arc::clone(&self.texture_layout),
            sampler: Arc::clone(&self.sampler),
        }
    }
    pub fn upload_native_texture(&self, source: &NativeTexture) -> RenderResult<GpuTexture> {
        self.texture_uploader().upload_native_texture(source)
    }
    /// Starts one immutable map generation; objects keep admission order.
    pub(crate) fn map_builder(&self, capacity: usize) -> GpuMapBuilder {
        GpuMapBuilder::new(
            Arc::clone(&self.device),
            Arc::clone(&self.map_pipelines),
            capacity,
        )
    }
}

/// Vertex slot 2 contents for the original effects' TEXCOORD1, TANGENT and
/// TEXCOORD0.zw.
struct LightingStream {
    bytes: Vec<u8>,
    components: u8,
    tangent: bool,
    normal: bool,
    decal_uv: bool,
}

/// TEXCOORD1 keeps its decoded bits (half-float -0.0
/// survives for the baseenvironment `lm_norm` sign encoding); missing lanes,
/// a missing TANGENT and the zw of a two-lane TEXCOORD0 are zero and gated by
/// `components`/`tangent`/`decal_uv`. World meshes with no TEXCOORD1 declaration
/// use the native shader-patch default (0,0,0,1), not an invented UV mapping.
fn lighting_stream(
    geometry: &NativeMeshGeometry,
    world_vertex_defaults: bool,
) -> RenderResult<Option<LightingStream>> {
    let find = |semantic: VertexSemantic| {
        geometry
            .attributes
            .iter()
            .find(|attribute| attribute.semantic == semantic)
    };
    let vertex_count = geometry.positions.len();
    let normal = if find(VertexSemantic::Tangent(0)).is_none() {
        find(VertexSemantic::Normal(0)).and_then(|attribute| match &attribute.values {
            NativeVertexAttributeValues::Float3(values) => Some(values.clone()),
            NativeVertexAttributeValues::Float4(values) => {
                Some(values.iter().map(|v| [v[0], v[1], v[2]]).collect())
            }
            _ => None,
        })
    } else {
        None
    };
    let (lightmap, components): (Vec<[f32; 4]>, u8) =
        match find(VertexSemantic::TextureCoordinate(1)).map(|a| &a.values) {
            Some(NativeVertexAttributeValues::Float2(values)) => {
                (values.iter().map(|v| [v[0], v[1], 0.0, 0.0]).collect(), 2)
            }
            Some(NativeVertexAttributeValues::Float4(values)) => (values.clone(), 4),
            // Missing semantic: original fetch patch uses swizzle 0001.
            // Do not apply this to a PRESENT but unsupported/malformed input.
            None if world_vertex_defaults => (vec![[0., 0., 0., 1.]; vertex_count], 4),
            None if normal.is_some() => (vec![[0.; 4]; vertex_count], 0),
            _ => return Ok(None),
        };
    let tangent = find(VertexSemantic::Tangent(0)).and_then(|attribute| match &attribute.values {
        NativeVertexAttributeValues::Float3(values) => Some(values.clone()),
        NativeVertexAttributeValues::Float4(values) => {
            Some(values.iter().map(|v| [v[0], v[1], v[2]]).collect())
        }
        _ => None,
    });
    let decal_uv =
        find(VertexSemantic::TextureCoordinate(0)).and_then(|attribute| match &attribute.values {
            NativeVertexAttributeValues::Float4(values) => {
                Some(values.iter().map(|v| [v[2], v[3]]).collect::<Vec<_>>())
            }
            _ => None,
        });
    // A stream that does not cover every position is not admitted; the mesh
    // still uploads and its draws keep the diagnostic path.
    if lightmap.len() != vertex_count
        || tangent
            .as_ref()
            .is_some_and(|values| values.len() != vertex_count)
        || decal_uv
            .as_ref()
            .is_some_and(|values| values.len() != vertex_count)
        || normal
            .as_ref()
            .is_some_and(|values| values.len() != vertex_count)
    {
        return Ok(None);
    }
    let stride = usize::try_from(environment_shading::LIGHTING_STRIDE)?;
    let mut bytes = Vec::with_capacity(vertex_count * stride);
    for (index, lanes) in lightmap.iter().enumerate() {
        let tangent = tangent
            .as_ref()
            .or(normal.as_ref())
            .map_or([0.0; 3], |values| values[index]);
        let decal = decal_uv.as_ref().map_or([0.0; 2], |values| values[index]);
        for value in lanes.iter().chain(&tangent).chain(&decal) {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    Ok(Some(LightingStream {
        bytes,
        components,
        tangent: tangent.is_some(),
        normal: normal.is_some(),
        decal_uv: decal_uv.is_some(),
    }))
}
