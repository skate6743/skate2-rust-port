//! Retained world-map draw list: one camera constant per frame, static
//! per-object instance records, and visibility culling before submission.
//!
//! Reference shape (debug `sk82_na_m.xex`):
//! - `Sk8::Render::Frustum::ComputeFrustum` 0x8282bd28 takes six planes from
//!   the combined view-projection matrix (w+x, w-x, w-y, w+y, w+z, w-z).
//! - `Sk8::Render::SceneRenderView::Cull` 0x828500a8 tests every presentation
//!   entity AABB against that frustum; only surviving entities cull further.
//! - `Sk8::WorldPresEntityOptimesh::CullMeshes_VMX` 0x827d0ae8 culls optimesh
//!   islands (`Frustum::Cull_VMX_NEW` 0x82825948), and
//!   `WorldPresEntityOptimesh::FillMtlSrtKeyListWithCulledKeys` 0x827e3068
//!   submits a mesh's sort key only when at least one island survived,
//!   keeping visible islands in ascending island order.
//!
//! This backend keeps that entity -> island order but tests against the exact
//! clip volume the rasterizer applies (wgpu's 0 <= z <= w near plane, not the
//! reference's w+z) with a conservative tolerance. A rejected mesh or island
//! could not have produced a fragment, so the presented image is unchanged.
//! The reference's anti-frustum occluders and minimum pixel-size/area rejection
//! (`Frustum::UpdateCullParams` 0x82853970) remove visible geometry and are
//! deliberately not applied. Submission order is the retained render-queue
//! order; nothing is re-sorted.

use super::{
    CameraRelativeClip, DEPTH_FORMAT, GpuMesh, GpuTexture, RenderResult,
    environment_shading::{EnvironmentEffect, EnvironmentPipelines, GLOBALS_SIZE},
    multisample,
    raster_state::{RasterKey, RasterState},
    shadow_map::ShadowCascades,
    uniform_bytes,
};
use crate::render_shadows::WorldShadowProjection;
use skate2_core::{
    presentation_anti_frustums::{AntiFrustumSet, AntiFrustums},
    presentation_scene_culling::IslandAreaCull,
};
use skate2_core::presentation_shadow_culling::WorldShadowBounds;
use skate2_formats::render::AxisAlignedBox;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use wgpu::util::DeviceExt;

/// Relative slack between this f64 test and the GPU's f32 vertex arithmetic.
/// f32 rounding contributes ~1e-7 of the summed term magnitudes; this is 100x.
const CLIP_TOLERANCE: f64 = 1e-5;

/// World-space effects take positions directly; their instance matrix is identity.
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Same 20-byte position/UV0 vertex records `upload_mesh` writes.
const MESH_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

/// `uniform_bytes` layout: four world-from-mesh columns, then the draw color.
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4
];
/// World-from-mesh and color (80 bytes), then the original effects' per-draw
/// material lanes (`InstanceMaterial`, 80 bytes).
const INSTANCE_STRIDE: wgpu::BufferAddress = 160;
const INSTANCE_BYTES: usize = 160;

/// Per-draw values the original effects read from their instance/material
/// scopes: `i_monoLightmap_Dot`, the three baseenvironment scalar channels
/// and `m_params[0]`. Diagnostic draws leave them zero.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct InstanceMaterial {
    pub(crate) lightmap_dot: [f32; 4],
    pub(crate) macro_overlay_uv_scale: [f32; 4],
    pub(crate) detail_normal_uv_scale: [f32; 4],
    pub(crate) macro_overlay_opacity: [f32; 4],
    pub(crate) m_params: [f32; 4],
}

impl InstanceMaterial {
    /// `sky.fx` lanes: `m_params[0]` in the `m_params` lane, `m_params[1]` in
    /// the macro-overlay-UV lane, and lane 7 `.x` set when the scope's
    /// `m_params` storage is the WorldPainter sky blend output.
    pub(crate) fn sky(params: [[f32; 4]; 2], blended: bool) -> Self {
        Self {
            lightmap_dot: [f32::from(u8::from(blended)), 0.0, 0.0, 0.0],
            macro_overlay_uv_scale: params[1],
            m_params: params[0],
            ..Self::default()
        }
    }

    fn bytes(&self) -> [u8; 80] {
        let mut bytes = [0; 80];
        let lanes = self
            .lightmap_dot
            .iter()
            .chain(&self.macro_overlay_uv_scale)
            .chain(&self.detail_normal_uv_scale)
            .chain(&self.macro_overlay_opacity)
            .chain(&self.m_params);
        for (destination, value) in bytes.chunks_exact_mut(4).zip(lanes) {
            destination.copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
}

/// Environment-lighting globals and resources shared by one map generation.
#[derive(Clone)]
pub(crate) struct MapEnvironment {
    pub(crate) resources: Arc<wgpu::BindGroup>,
    pub(crate) envattributes2: [f32; 4],
}

/// Values `UpdateRenderParams` recomputes every frame: `g_FogK1`,
/// `g_FogColour` and `material_sky/default` `m_params` from the WorldPainter
/// blends, and `g_vLightDir` from the sun-direction map; then the cascades
/// `RenderShadows_CSM` builds from that light.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FrameEnvironment {
    pub(crate) exposure: Option<f32>,
    pub(crate) fog_k1: [f32; 4],
    pub(crate) fog_colour: [f32; 4],
    pub(crate) sky_params: [[f32; 4]; 2],
    pub(crate) light_direction: [f32; 4],
    pub(crate) view_dot_light: [f32; 4],
    pub(crate) animation_time: [f32; 4],
    pub(crate) pca: [[f32; 4]; 7],
    pub(crate) traffic_lights: [[f32; 4]; 2],
    pub(crate) camera_axes: [[f32; 4]; 3],
    pub(crate) shadows: ShadowCascades,
    pub(crate) world_shadow: Option<WorldShadowProjection>,
    /// The main view's island area rule; None culls islands by the frustum only.
    pub(crate) scene_cull: Option<skate2_core::presentation_scene_culling::IslandAreaCullSettings>,
}

/// Axis-aligned bounds of positions exactly as the vertex buffer holds them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    min: [f64; 3],
    max: [f64; 3],
}

impl Bounds {
    fn union(&self, other: &Self) -> Self {
        Self {
            min: std::array::from_fn(|axis| self.min[axis].min(other.min[axis])),
            max: std::array::from_fn(|axis| self.max[axis].max(other.max[axis])),
        }
    }

    /// Conservative world bounds under a column-major affine world-from-mesh
    /// matrix. Projective or non-finite matrices return None (never culled).
    fn transformed(&self, world_from_mesh: &[f32; 16]) -> Option<Self> {
        let m = world_from_mesh;
        let affine =
            m[3] == 0.0 && m[7] == 0.0 && m[11] == 0.0 && m[15].to_bits() == 1.0_f32.to_bits();
        if !affine || m.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        for (row, (lower, upper)) in min.iter_mut().zip(&mut max).enumerate() {
            let translation = f64::from(m[12 + row]);
            let (mut low, mut high, mut magnitude) = (translation, translation, translation.abs());
            for (column, (source_min, source_max)) in self.min.iter().zip(&self.max).enumerate() {
                let scale = f64::from(m[column * 4 + row]);
                let (at_min, at_max) = (scale * source_min, scale * source_max);
                low += at_min.min(at_max);
                high += at_min.max(at_max);
                magnitude += at_min.abs().max(at_max.abs());
            }
            // The GPU forms this product in f32; widen far beyond its rounding.
            let slack = magnitude * CLIP_TOLERANCE;
            *lower = low - slack;
            *upper = high + slack;
        }
        Some(Self { min, max })
    }
}

/// Accumulates the positions one original island's indices reference.
pub(super) struct BoundsBuilder {
    min: [f64; 3],
    max: [f64; 3],
    empty: bool,
    finite: bool,
}

impl Default for BoundsBuilder {
    fn default() -> Self {
        Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
            empty: true,
            finite: true,
        }
    }
}

impl BoundsBuilder {
    pub(super) fn include(&mut self, position: [f32; 3]) {
        self.empty = false;
        self.finite &= position.iter().all(|value| value.is_finite());
        for ((min, max), value) in self.min.iter_mut().zip(&mut self.max).zip(position) {
            *min = min.min(f64::from(value));
            *max = max.max(f64::from(value));
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.empty
    }

    /// None for an empty island or one with a non-finite position.
    pub(super) fn finish(self) -> Option<Bounds> {
        (!self.empty && self.finite).then_some(Bounds {
            min: self.min,
            max: self.max,
        })
    }
}

/// Mesh-level union: None when any non-empty island is unbounded.
pub(super) fn union_of_islands(islands: &[Option<Bounds>], all_bounded: bool) -> Option<Bounds> {
    if !all_bounded {
        return None;
    }
    islands
        .iter()
        .flatten()
        .copied()
        .reduce(|current, next| current.union(&next))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Containment {
    Outside,
    Intersecting,
    Inside,
}

/// The six homogeneous half-spaces the rasterizer keeps for one camera.
pub(super) struct ClipVolume {
    planes: [[f64; 4]; 6],
}

impl ClipVolume {
    /// Column-major clip-from-world: -w <= x <= w, -w <= y <= w, 0 <= z <= w.
    pub(super) fn new(clip_from_world: &[f32; 16]) -> Self {
        let rows: [[f64; 4]; 4] = std::array::from_fn(|row| {
            std::array::from_fn(|column| f64::from(clip_from_world[column * 4 + row]))
        });
        let plus = |left: [f64; 4], right: [f64; 4]| -> [f64; 4] {
            std::array::from_fn(|term| left[term] + right[term])
        };
        let minus = |left: [f64; 4], right: [f64; 4]| -> [f64; 4] {
            std::array::from_fn(|term| left[term] - right[term])
        };
        let [clip_x, clip_y, clip_z, clip_w] = rows;
        Self {
            planes: [
                plus(clip_w, clip_x),
                minus(clip_w, clip_x),
                plus(clip_w, clip_y),
                minus(clip_w, clip_y),
                clip_z,
                minus(clip_w, clip_z),
            ],
        }
    }

    /// Outside only when every point of the box is provably beyond one plane.
    /// A plane whose terms are not finite never rejects on its own.
    pub(super) fn classify(&self, bounds: &Bounds) -> Containment {
        let mut containment = Containment::Inside;
        for plane in &self.planes {
            let mut farthest = plane[3];
            let mut nearest = plane[3];
            let mut magnitude = plane[3].abs();
            for ((coefficient, min), max) in plane.iter().zip(&bounds.min).zip(&bounds.max) {
                let (low, high) = (coefficient * min, coefficient * max);
                farthest += low.max(high);
                nearest += low.min(high);
                magnitude += low.abs().max(high.abs());
            }
            let tolerance = magnitude * CLIP_TOLERANCE;
            if farthest < -tolerance {
                return Containment::Outside;
            }
            if nearest < tolerance {
                containment = Containment::Intersecting;
            }
        }
        containment
    }
}

/// `SceneRenderView::Cull` (0x828500a8) for one presentation entity: whether its world box, min
/// then max, reaches the view `clip_from_world` draws.
pub(crate) fn world_box_in_view(clip_from_world: &[f32; 16], bounds: [[f32; 3]; 2]) -> bool {
    let bounds = Bounds { min: bounds[0].map(f64::from), max: bounds[1].map(f64::from) };
    ClipVolume::new(clip_from_world).classify(&bounds) != Containment::Outside
}

/// A diagnostic draw's pixel program: the explicit solid colour, the raw
/// diffuse, or the diffuse with its source alpha test (reference byte).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum DiagnosticFragment {
    Solid,
    Textured,
    DiffuseAlpha(u8),
}

/// Diagnostic pipeline-cache identity: pixel program and the technique's
/// pass rasterizer state.
pub(crate) type DiagnosticPipelineKey = (DiagnosticFragment, RasterKey);

/// Map pipelines share one globals constant (group 0) and a material group
/// (group 1: the renderer's diffuse layout, or an original effect's sampler
/// set); per-object data is an instance record.
pub(super) struct MapPipelines {
    camera_layout: wgpu::BindGroupLayout,
    solid_layout: wgpu::PipelineLayout,
    textured_layout: wgpu::PipelineLayout,
    /// The diagnostic solid/diffuse programs.
    shader: wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    /// Diagnostic pipelines created later on workers must match the main pass.
    sample_count: u32,
    /// Per-pass default state (no culling, no bias) solid and diffuse draws.
    solid: Arc<wgpu::RenderPipeline>,
    textured: Arc<wgpu::RenderPipeline>,
    pub(super) environment: EnvironmentPipelines,
}

impl MapPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("map globals layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(GLOBALS_SIZE),
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("map solid/diffuse; not original material lighting"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let solid_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("map solid pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout)],
            immediate_size: 0,
        });
        let textured_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("map diffuse pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(texture_layout)],
            immediate_size: 0,
        });
        let solid = Arc::new(map_pipeline(
            device,
            &shader,
            &solid_layout,
            format,
            sample_count,
            "fs_main",
            RasterState::default(),
        ));
        let textured = Arc::new(map_pipeline(
            device,
            &shader,
            &textured_layout,
            format,
            sample_count,
            "fs_textured",
            RasterState::default(),
        ));
        let environment = EnvironmentPipelines::new(
            device,
            format,
            sample_count,
            &camera_layout,
            &MESH_ATTRIBUTES,
            INSTANCE_STRIDE,
        );
        Self {
            camera_layout,
            solid_layout,
            textured_layout,
            shader,
            format,
            sample_count,
            solid,
            textured,
            environment,
        }
    }

    /// The globals layout (group 0) every original effect reads.
    pub(super) fn camera_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_layout
    }

    /// The single per-frame globals constant shared by every map draw.
    pub(super) fn create_camera(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("map globals (clip-from-world first)"),
            size: GLOBALS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("map camera"),
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, binding)
    }

    /// One diagnostic pipeline with a technique's pass rasterizer state.
    /// `DiffuseAlpha`: source-reviewed environment shaders return unmodified
    /// diffuse alpha. `D3DDevice_SetRenderState_AlphaRef` 0x82c34c30
    /// multiplies the authored byte by 1/255. GREATEREQUAL keeps the threshold
    /// itself; rejected fragments must not write depth. Lighting remains the
    /// diagnostic path.
    pub(super) fn create_diagnostic_pipeline(
        &self,
        device: &wgpu::Device,
        fragment: DiagnosticFragment,
        raster: RasterState,
    ) -> Arc<wgpu::RenderPipeline> {
        let alpha_shader;
        let (shader, layout, entry_point) = match fragment {
            DiagnosticFragment::Solid => (&self.shader, &self.solid_layout, "fs_main"),
            DiagnosticFragment::Textured => (&self.shader, &self.textured_layout, "fs_textured"),
            DiagnosticFragment::DiffuseAlpha(reference) => {
                let source = format!(
                    "{SHADER}\n@fragment fn fs_alpha(input: VertexOutput) -> @location(0) vec4<f32> {{\nlet diffuse = textureSample(diffuse_texture, diffuse_sampler, input.uv);\nif (diffuse.a < {} * 0.0039215689) {{ discard; }}\nreturn vec4<f32>(diffuse.rgb, diffuse.a) * input.color;\n}}",
                    f32::from(reference)
                );
                alpha_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("original diffuse-alpha rejection; diagnostic lighting"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
                (&alpha_shader, &self.textured_layout, "fs_alpha")
            }
        };
        Arc::new(map_pipeline(
            device,
            shader,
            layout,
            self.format,
            self.sample_count,
            entry_point,
            raster,
        ))
    }
}

fn map_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    sample_count: u32,
    fragment: &str,
    raster: RasterState,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        cache: None,
        label: Some("retained map original geometry"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            compilation_options: Default::default(),
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &MESH_ATTRIBUTES,
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_STRIDE,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRIBUTES,
                }),
            ],
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
        primitive: raster.primitive(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: raster.depth_bias(),
        }),
        multisample: multisample(sample_count),
        multiview_mask: None,
    })
}

struct MapObject {
    mesh: GpuMesh,
    pipeline: Arc<wgpu::RenderPipeline>,
    /// Group 1: diagnostic diffuse or an original effect's material samplers.
    texture: Option<Arc<wgpu::BindGroup>>,
    /// Vertex slot 2 for original effects (TEXCOORD1 and TANGENT).
    lighting: Option<Arc<wgpu::Buffer>>,
    /// World-space bounds; None is never culled.
    bounds: Option<Bounds>,
    /// World-space bounds per original island, aligned with `mesh.draws`.
    island_bounds: Arc<[Option<Bounds>]>,
    shadow: Option<WorldShadowGeometry>,
}

/// RenderShadowMesh82824440: position-only simple VS, existing mesh/instance
/// streams, no material PS or alpha test. World-space effects have identity
/// instances already; local-space diagnostic effects keep their own matrix.
pub(super) fn world_caster_pipeline(
    device: &wgpu::Device,
    camera: &wgpu::BindGroupLayout,
    depth: wgpu::DepthStencilState,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("WorldShadow simpleVS"),
        source: wgpu::ShaderSource::Wgsl(
            r"
struct Camera { clip: mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@vertex fn main(@location(0) position: vec3<f32>, @location(2) c0: vec4<f32>,
    @location(3) c1: vec4<f32>, @location(4) c2: vec4<f32>, @location(5) c3: vec4<f32>)
    -> @builtin(position) vec4<f32> {
    return camera.clip * (mat4x4<f32>(c0, c1, c2, c3) * vec4<f32>(position, 1.0));
}"
            .into(),
        ),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("WorldShadow simpleVS"),
        bind_group_layouts: &[Some(camera)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        cache: None,
        label: Some("WorldShadow simpleVS"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            compilation_options: Default::default(),
            module: &module,
            entry_point: Some("main"),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &MESH_ATTRIBUTES[..1],
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_STRIDE,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRIBUTES[..4],
                }),
            ],
        },
        fragment: None,
        primitive: RasterState::culled(super::raster_state::CullMode::Back).primitive(),
        depth_stencil: Some(depth),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
    })
}

/// The original entity box and RModel island table, alongside (not another
/// copy of) the existing GPU geometry. Proxies have none; sky contributes
/// entity bounds but has no caster islands.
#[derive(Clone)]
pub(crate) struct WorldShadowGeometry {
    pub(crate) entity: AxisAlignedBox,
    pub(crate) islands: Arc<[AxisAlignedBox]>,
    /// The RModel's `m_pIslandAreas`, aligned with `islands`; empty without one.
    pub(crate) areas: Arc<[[f32; 4]]>,
    pub(crate) range: std::ops::Range<usize>,
    /// Old-format WorldPresEntity's vtable has a no-op shadow cull and
    /// returns its full mesh list (82110f48+24/+34).
    pub(crate) cull_islands: bool,
    /// Sky ctor827d436c leaves its shadow-visible island count zero.
    pub(crate) casts: bool,
}

pub(super) struct WorldShadowDraws {
    pub(super) rows: [[f32; 4]; 3],
    draws: Vec<(usize, usize)>,
}
impl WorldShadowDraws {
    pub(super) fn draw_count(&self) -> usize {
        self.draws.len()
    }
}

/// Admits map objects in render-queue submission order.
pub(crate) struct GpuMapBuilder {
    device: Arc<wgpu::Device>,
    pipelines: Arc<MapPipelines>,
    objects: Vec<MapObject>,
    instances: Vec<u8>,
    shadow: Option<WorldShadowGeometry>,
    anti_frustums: Vec<Arc<AntiFrustumSet>>,
}

impl GpuMapBuilder {
    pub(super) fn new(
        device: Arc<wgpu::Device>,
        pipelines: Arc<MapPipelines>,
        capacity: usize,
    ) -> Self {
        Self {
            device,
            pipelines,
            objects: Vec::with_capacity(capacity),
            instances: Vec::with_capacity(capacity.saturating_mul(INSTANCE_BYTES)),
            shadow: None,
            anti_frustums: Vec::new(),
        }
    }

    pub(crate) fn set_shadow_geometry(&mut self, geometry: Option<WorldShadowGeometry>) {
        self.shadow = geometry;
    }

    /// The registered arenas' occluders, in the order the view selects from them.
    pub(crate) fn set_anti_frustums(&mut self, sets: Vec<Arc<AntiFrustumSet>>) {
        self.anti_frustums = sets;
    }

    /// Explicitly solid diagnostic color. `world_from_mesh` None means the
    /// effect already takes world-space positions. `pipeline` is the
    /// technique's diagnostic pipeline; None keeps the per-pass default state.
    pub(crate) fn push_solid(
        &mut self,
        mesh: &GpuMesh,
        world_from_mesh: Option<&[f32; 16]>,
        color: [f32; 4],
        pipeline: Option<&Arc<wgpu::RenderPipeline>>,
    ) {
        let pipeline = Arc::clone(pipeline.unwrap_or(&self.pipelines.solid));
        let record = DrawRecord {
            world_from_mesh,
            color,
            pipeline,
            texture: None,
            lighting: None,
            material: InstanceMaterial::default(),
            cull: true,
        };
        self.push(mesh, record);
    }

    /// Uses actual UV0 and a caller-resolved diffuse texture. `pipeline` is
    /// the technique's diagnostic pipeline (its source alpha test when the
    /// technique enables one, and its rasterizer state); None keeps the
    /// per-pass default diffuse pipeline.
    pub(crate) fn push_textured(
        &mut self,
        mesh: &GpuMesh,
        world_from_mesh: Option<&[f32; 16]>,
        texture: &GpuTexture,
        pipeline: Option<&Arc<wgpu::RenderPipeline>>,
    ) -> RenderResult<()> {
        if !mesh.has_uv {
            return Err("textured mesh lacks supported original UV0".into());
        }
        let binding = texture
            .binding
            .as_ref()
            .ok_or("diagnostic diffuse cannot be a cube map")?;
        let record = DrawRecord {
            world_from_mesh,
            color: [1.0; 4],
            pipeline: Arc::clone(pipeline.unwrap_or(&self.pipelines.textured)),
            texture: Some(Arc::clone(binding)),
            lighting: None,
            material: InstanceMaterial::default(),
            cull: true,
        };
        self.push(mesh, record);
        Ok(())
    }

    /// One draw through a ported original effect. These effects take
    /// world-space positions (`i_matWorld` is declared but unused), so the
    /// instance matrix is identity. Fails when the mesh lacks the effect's
    /// TEXCOORD1 width, TANGENT or four-lane TEXCOORD0; an effect that reads
    /// no TEXCOORD1 (`proxyworld`) binds no lighting stream.
    pub(crate) fn push_environment(
        &mut self,
        mesh: &GpuMesh,
        effect: EnvironmentEffect,
        material_textures: Arc<wgpu::BindGroup>,
        material: InstanceMaterial,
    ) -> RenderResult<()> {
        self.push_environment_transform(mesh, effect, material_textures, material, None)
    }

    pub(crate) fn push_environment_transform(
        &mut self,
        mesh: &GpuMesh,
        effect: EnvironmentEffect,
        material_textures: Arc<wgpu::BindGroup>,
        material: InstanceMaterial,
        world_from_mesh: Option<&[f32; 16]>,
    ) -> RenderResult<()> {
        let (components, tangent) = effect.lighting_inputs();
        let needs_normal = effect == EnvironmentEffect::GodraySpotlight;
        let lighting = if components == 0 && !needs_normal {
            None
        } else {
            Some(Arc::clone(
                mesh.lighting
                    .as_ref()
                    .ok_or("mesh has no original TEXCOORD1 stream")?,
            ))
        };
        if !mesh.has_uv {
            return Err("mesh lacks supported original UV0".into());
        }
        if mesh.lightmap_components < components {
            return Err("mesh TEXCOORD1 is narrower than the effect input".into());
        }
        if tangent && !mesh.has_tangent {
            return Err("mesh lacks the effect's TANGENT input".into());
        }
        if needs_normal && !mesh.has_normal {
            return Err("spotlight mesh lacks original NORMAL0 stream".into());
        }
        if effect.reads_decal_uv() && !mesh.has_decal_uv {
            return Err("mesh TEXCOORD0 lacks the decal UV lanes".into());
        }
        let record = DrawRecord {
            world_from_mesh,
            color: [1.0; 4],
            pipeline: self.pipelines.environment.pipeline(effect),
            texture: Some(material_textures),
            lighting,
            material,
            cull: true,
        };
        self.push(mesh, record);
        Ok(())
    }

    /// One `sky.fx` draw. The shader offsets model-space positions by
    /// `g_vViewPos`, and `SkyPresEntity` 0x827d4258 marks every island visible
    /// and re-centres itself on the camera (`Update` 0x827d4480), so the
    /// static bounds are meaningless: sky draws are never culled.
    pub(crate) fn push_sky(
        &mut self,
        mesh: &GpuMesh,
        effect: EnvironmentEffect,
        material_textures: Arc<wgpu::BindGroup>,
        material: InstanceMaterial,
    ) -> RenderResult<()> {
        if !mesh.has_uv {
            return Err("sky mesh lacks supported original UV0".into());
        }
        let record = DrawRecord {
            world_from_mesh: None,
            color: [1.0; 4],
            pipeline: self.pipelines.environment.pipeline(effect),
            texture: Some(material_textures),
            lighting: None,
            material,
            cull: false,
        };
        self.push(mesh, record);
        Ok(())
    }

    fn push(&mut self, mesh: &GpuMesh, record: DrawRecord<'_>) {
        let DrawRecord {
            world_from_mesh,
            color,
            pipeline,
            texture,
            lighting,
            material,
            cull,
        } = record;
        let (bounds, island_bounds) = match world_from_mesh {
            _ if !cull => (None, mesh.draws.iter().map(|_| None).collect::<Arc<[_]>>()),
            None => (mesh.bounds, Arc::clone(&mesh.island_bounds)),
            Some(matrix) => (
                mesh.bounds.and_then(|bounds| bounds.transformed(matrix)),
                mesh.island_bounds
                    .iter()
                    .map(|island| island.and_then(|bounds| bounds.transformed(matrix)))
                    .collect::<Arc<[_]>>(),
            ),
        };
        self.instances
            .extend_from_slice(&uniform_bytes(world_from_mesh.unwrap_or(&IDENTITY), &color));
        self.instances.extend_from_slice(&material.bytes());
        self.objects.push(MapObject {
            mesh: mesh.clone(),
            pipeline,
            texture,
            lighting,
            bounds,
            island_bounds,
            shadow: self.shadow.clone(),
        });
    }

    /// `environment` carries the lighting globals and resources the original
    /// effects read; None only when no draw uses them.
    pub(crate) fn finish(
        self,
        clip_from_world: [f32; 16],
        environment: Option<MapEnvironment>,
    ) -> GpuMap {
        let mut contents = self.instances;
        if contents.is_empty() {
            // Zero-sized vertex buffers cannot be bound; nothing draws from it.
            contents.extend_from_slice(&uniform_bytes(&IDENTITY, &[0.0; 4]));
            contents.extend_from_slice(&InstanceMaterial::default().bytes());
        }
        let instances = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("retained map world-from-mesh/color/material instances"),
                contents: &contents,
                usage: wgpu::BufferUsages::VERTEX,
            });
        GpuMap {
            objects: self.objects,
            anti_frustums: self.anti_frustums,
            instances,
            clip_from_world,
            view_position: [0.0; 4],
            frame: FrameEnvironment::default(),
            environment,
            visible: OnceLock::new(),
            world_shadow: OnceLock::new(),
            shadow_commands: Mutex::new(None),
            rain_surface: Arc::default(),
            rain_location: String::new(),
            rain_gathered: false,
        }
    }
}

/// One retained draw as the builder's public entry points assemble it.
struct DrawRecord<'a> {
    world_from_mesh: Option<&'a [f32; 16]>,
    color: [f32; 4],
    pipeline: Arc<wgpu::RenderPipeline>,
    texture: Option<Arc<wgpu::BindGroup>>,
    lighting: Option<Arc<wgpu::Buffer>>,
    material: InstanceMaterial,
    /// False only for draws positioned relative to the camera (the sky).
    cull: bool,
}

/// Per-frame map submission counts for diagnostics.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MapFrameStats {
    pub(crate) objects: usize,
    pub(crate) drawn_objects: usize,
    pub(crate) drawn_islands: usize,
    pub(crate) draw_calls: usize,
}
impl MapFrameStats {
    /// Another run's draws of the same map.
    pub(super) fn add(&mut self, run: Self) {
        self.objects = run.objects;
        self.drawn_objects += run.drawn_objects;
        self.drawn_islands += run.drawn_islands;
        self.draw_calls += run.draw_calls;
    }
}

/// Merge only consecutive, complete triangle lists of the SAME object and
/// base vertex. Never bridge a culled index gap or change instance/material
/// order. All map pipelines use RasterState's TriangleList topology.
fn island_runs(
    draws: impl Iterator<Item = (u32, std::ops::Range<u32>, i32)>,
    merge: bool,
) -> impl Iterator<Item = (u32, std::ops::Range<u32>, i32, usize)> {
    let mut draws = draws.peekable();
    std::iter::from_fn(move || {
        let (instance, mut range, base) = draws.next()?;
        let mut islands = 1;
        if merge && (range.end - range.start) % 3 == 0 {
            while let Some((next_instance, next_range, next_base)) = draws.peek() {
                if *next_instance != instance
                    || *next_base != base
                    || range.end != next_range.start
                    || (next_range.end - next_range.start) % 3 != 0
                {
                    break;
                }
                range.end = next_range.end;
                islands += 1;
                draws.next();
            }
        }
        Some((instance, range, base, islands))
    })
}
fn merge_islands() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("SKATE2_SEPARATE_ISLAND_DRAWS").is_none())
}

/// An object's pipeline and material binds, against one island draw.
const OBJECT_ENCODE_COST: usize = 4;

/// `visible` cut into at most `count` runs of about equal encoding work, each ending
/// where an object's draws end, so every run draws whole objects in the same order.
pub(super) fn draw_runs(visible: &[(u32, usize)], count: usize) -> Vec<std::ops::Range<usize>> {
    let starts_object = |index: usize| index == 0 || visible[index - 1].0 != visible[index].0;
    let total: usize = (0..visible.len())
        .map(|index| {
            1 + if starts_object(index) {
                OBJECT_ENCODE_COST
            } else {
                0
            }
        })
        .sum();
    let count = count.max(1);
    let mut runs = Vec::with_capacity(count);
    let (mut start, mut cost) = (0, 0);
    for index in 0..visible.len() {
        let object = starts_object(index);
        if object
            && index > start
            && runs.len() + 1 < count
            && cost * count >= total * (runs.len() + 1)
        {
            runs.push(start..index);
            start = index;
        }
        cost += 1 + if object { OBJECT_ENCODE_COST } else { 0 };
    }
    runs.push(start..visible.len());
    runs
}

/// What the map's draws read: shared by the threads that encode its runs.
pub(super) struct MapDrawSource<'a> {
    objects: &'a [MapObject],
    instances: &'a wgpu::Buffer,
    environment: Option<&'a wgpu::BindGroup>,
}

impl<'a> MapDrawSource<'a> {
    /// Records visible objects and islands in retained submission order.
    pub(super) fn record_draws(
        &self,
        pass: &mut impl wgpu::util::RenderEncoder<'a>,
        camera: &'a wgpu::BindGroup,
        visible: &[(u32, usize)],
    ) -> MapFrameStats {
        let mut stats = MapFrameStats {
            objects: self.objects.len(),
            ..MapFrameStats::default()
        };
        pass.set_bind_group(0, Some(camera), &[]);
        if let Some(environment) = self.environment {
            pass.set_bind_group(2, Some(environment), &[]);
        }
        pass.set_vertex_buffer(1, Some(self.instances.slice(..)));
        // wgpu 0.19 tracks every set it replays, a repeated one too: bind what changes.
        // A new pipeline binds its material again.
        let mut bound_pipeline: Option<&Arc<wgpu::RenderPipeline>> = None;
        let mut bound_texture: Option<&Arc<wgpu::BindGroup>> = None;
        let mut bound_vertices: Option<&Arc<wgpu::Buffer>> = None;
        let mut bound_indices: Option<&Arc<wgpu::Buffer>> = None;
        let mut bound_lighting: Option<&Arc<wgpu::Buffer>> = None;
        let mut previous_object = None;
        let objects = self.objects;
        let draws = visible.iter().map(|&(instance, island)| {
            let (range, base) = &objects[instance as usize].mesh.draws[island];
            (instance, range.clone(), *base)
        });
        for (instance, range, base, islands) in island_runs(draws, merge_islands()) {
            let object = &objects[instance as usize];
            if previous_object != Some(instance) {
                previous_object = Some(instance);
                stats.drawn_objects += 1;
                if !bound_pipeline.is_some_and(|pipeline| Arc::ptr_eq(pipeline, &object.pipeline)) {
                    pass.set_pipeline(&object.pipeline);
                    bound_pipeline = Some(&object.pipeline);
                    bound_texture = None;
                }
                if let Some(texture) = &object.texture
                    && !bound_texture.is_some_and(|bound| Arc::ptr_eq(bound, texture))
                {
                    pass.set_bind_group(1, Some(texture), &[]);
                    bound_texture = Some(texture);
                }
                if !bound_vertices.is_some_and(|buffer| Arc::ptr_eq(buffer, &object.mesh.vertices))
                {
                    pass.set_vertex_buffer(0, Some(object.mesh.vertices.slice(..)));
                    bound_vertices = Some(&object.mesh.vertices);
                }
                if let Some(lighting) = &object.lighting {
                    if !bound_lighting.is_some_and(|buffer| Arc::ptr_eq(buffer, lighting)) {
                        pass.set_vertex_buffer(2, Some(lighting.slice(..)));
                        bound_lighting = Some(lighting);
                    }
                }
                if !bound_indices.is_some_and(|buffer| Arc::ptr_eq(buffer, &object.mesh.indices)) {
                    pass.set_index_buffer(object.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    bound_indices = Some(&object.mesh.indices);
                }
            }
            stats.drawn_islands += islands;
            stats.draw_calls += 1;
            pass.draw_indexed(range, base, instance..instance + 1);
        }
        stats
    }
}

struct CachedShadowCommands {
    selection: Vec<(usize, usize)>,
    camera: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    format: wgpu::TextureFormat,
    commands: Arc<wgpu::RenderBundle>,
}

/// One immutable generation of map draws. Only the camera changes per frame.
pub(crate) struct GpuMap {
    objects: Vec<MapObject>,
    instances: wgpu::Buffer,
    clip_from_world: [f32; 16],
    /// `g_vViewPos` (`UpdateRenderParams` 0x82852780): camera translation.
    view_position: [f32; 4],
    frame: FrameEnvironment,
    environment: Option<MapEnvironment>,
    /// Geometry is immutable for this map generation. Redraws between camera
    /// updates can reuse exactly the same ordered selection.
    visible: OnceLock<Vec<(u32, usize)>>,
    /// `AntiFrustumManager`'s occluders from the registered arenas.
    anti_frustums: Vec<Arc<AntiFrustumSet>>,
    world_shadow: OnceLock<WorldShadowDraws>,
    shadow_commands: Mutex<Option<CachedShadowCommands>>,
    /// PC weather: the rain's world model of this map generation (built only while a weather is
    /// picked) with its location.
    pub(crate) rain_surface: super::rain::LoadingSurface,
    pub(crate) rain_location: String,
    /// The rain's world model was gathered for this map generation (a weather was picked).
    pub(crate) rain_gathered: bool,
}

impl GpuMap {
    /// PC weather: this frame's weather inputs, or none for Vanilla.
    pub(super) fn weather_frame(&self, eye: Option<&super::stereoscopy::Eye>) -> Option<super::weather::Frame> {
        let mode = crate::weather::current();
        if mode == crate::weather::Mode::Vanilla {
            return None;
        }
        Some(super::weather::Frame {
            clip: eye.map_or(self.clip_from_world, |e| e.clip),
            camera: self.camera(),
            axes: self.frame.camera_axes,
            clock: self.frame.animation_time[0],
            fog: self.frame.fog_colour,
            mode,
        })
    }
    pub(super) fn world_shadow_draws(&self) -> RenderResult<&WorldShadowDraws> {
        if let Some(selected) = self.world_shadow.get() {
            return Ok(selected);
        }
        // Cache only a successful selection; errors remain observable.
        let selected = self.select_world_shadow_draws()?;
        let _ = self.world_shadow.set(selected);
        Ok(self.world_shadow.get().expect("installed shadow selection"))
    }

    fn select_world_shadow_draws(&self) -> RenderResult<WorldShadowDraws> {
        let mut result = WorldShadowDraws {
            rows: [[0.0; 4]; 3],
            draws: Vec::new(),
        };
        let Some(projection) = self.frame.world_shadow else {
            return Ok(result);
        };
        let frustum = projection.frustum()?;
        let mut bounds = WorldShadowBounds::default();
        let mut admitted = false;
        for (object_index, object) in self.objects.iter().enumerate() {
            let Some(shadow) = &object.shadow else {
                continue;
            };
            if frustum.try_cull(&shadow.entity)? == 0.0 {
                continue;
            }
            admitted = true;
            bounds.include(&shadow.entity);
            if !shadow.casts {
                continue;
            }
            if !shadow.cull_islands {
                result
                    .draws
                    .extend((0..object.mesh.draws.len()).map(|i| (object_index, i)));
                continue;
            }
            for (island, bbox) in shadow.islands[shadow.range.clone()].iter().enumerate() {
                if frustum.try_cull(bbox)? != 0.0 {
                    result.draws.push((object_index, island));
                }
            }
        }
        // No entity means a cleared, uncovered map; avoid publishing native
        // empty-bounds infinities to WGSL. No geometry is admitted by this.
        if admitted {
            result.rows = projection.fitted_rows(bounds);
        }
        Ok(result)
    }

    /// Exact selected caster commands; light rows still upload every frame.
    /// This map generation owns immutable mesh/instance buffers and the cache.
    pub(super) fn prepare_world_shadow(
        &self,
        device: &wgpu::Device,
        camera: &wgpu::BindGroup,
        pipeline: &wgpu::RenderPipeline,
        format: wgpu::TextureFormat,
        selected: &WorldShadowDraws,
    ) -> Arc<wgpu::RenderBundle> {
        self.prepare_shadow_commands(device, camera, pipeline, format, selected, &self.shadow_commands)
    }
    fn prepare_shadow_commands(
        &self,
        device: &wgpu::Device,
        camera: &wgpu::BindGroup,
        pipeline: &wgpu::RenderPipeline,
        format: wgpu::TextureFormat,
        selected: &WorldShadowDraws,
        cache: &Mutex<Option<CachedShadowCommands>>,
    ) -> Arc<wgpu::RenderBundle> {
        let mut cached = cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(previous) = cached.as_ref() {
            if previous.selection == selected.draws
                && previous.camera == *camera
                && previous.pipeline == *pipeline
                && previous.format == format
            {
                return Arc::clone(&previous.commands);
            }
        }
        let mut pass = device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
            label: Some("unchanged ordered world shadow casters"),
            color_formats: &[],
            depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                format,
                depth_read_only: false,
                stencil_read_only: false,
            }),
            sample_count: 1,
            multiview: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_vertex_buffer(1, self.instances.slice(..));
        let mut vertices = None;
        let mut indices = None;
        let draws = selected.draws.iter().map(|&(object, island)| {
            let (range, base) = &self.objects[object].mesh.draws[island];
            (object as u32, range.clone(), *base)
        });
        for (instance, range, base, _) in island_runs(draws, merge_islands()) {
            let object = &self.objects[instance as usize];
            if !vertices.is_some_and(|buffer| Arc::ptr_eq(buffer, &object.mesh.vertices)) {
                pass.set_vertex_buffer(0, object.mesh.vertices.slice(..));
                vertices = Some(&object.mesh.vertices);
            }
            if !indices.is_some_and(|buffer| Arc::ptr_eq(buffer, &object.mesh.indices)) {
                pass.set_index_buffer(object.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                indices = Some(&object.mesh.indices);
            }
            pass.draw_indexed(range, base, instance..instance + 1);
        }
        let commands = Arc::new(pass.finish(&wgpu::RenderBundleDescriptor {
            label: Some("ordered world shadow casters"),
        }));
        *cached = Some(CachedShadowCommands {
            selection: selected.draws.clone(),
            camera: camera.clone(),
            pipeline: pipeline.clone(),
            format,
            commands: Arc::clone(&commands),
        });
        commands
    }
    /// Column-major clip-from-world with WebGPU depth range 0..1, and the
    /// camera world position the original effects read as `g_vViewPos`.
    /// `frame` is this frame's `UpdateRenderParams` fog and sky output.
    pub(crate) fn set_view(
        &mut self,
        clip_from_world: [f32; 16],
        view_position: [f32; 3],
        frame: FrameEnvironment,
    ) {
        if self.clip_from_world != clip_from_world {
            self.visible.take();
        }
        if self.frame.world_shadow != frame.world_shadow {
            self.world_shadow.take();
        }
        self.clip_from_world = clip_from_world;
        self.view_position = [view_position[0], view_position[1], view_position[2], 0.0];
        self.frame = frame;
    }

    pub(super) fn stereo_eye(&self, planes:[f32;2], field:u8) -> RenderResult<super::stereoscopy::Eye> {
        super::stereoscopy::Eye::new(self.clip_from_world,self.frame.camera_axes,self.camera(),planes,field)
    }

    /// This frame's shadow cascades.
    pub(super) fn shadows(&self) -> &ShadowCascades {
        &self.frame.shadows
    }

    /// `g_vViewPos`, the camera every GPU position is relative to.
    pub(super) fn camera(&self) -> [f32; 3] {
        [
            self.view_position[0],
            self.view_position[1],
            self.view_position[2],
        ]
    }

    /// The per-frame globals. The matrix is camera-relative
    /// (`CameraRelativeClip`); the diagnostic shaders read it and `g_vViewPos`.
    pub(super) fn globals_bytes(&self) -> [u8; 464] {
        self.globals_bytes_with_exposure(None)
    }
    pub(super) fn globals_bytes_with_exposure(&self, exposure_override: Option<f32>) -> [u8; 464] {
        self.globals_for_eye(exposure_override, None)
    }
    pub(super) fn globals_for_eye(&self, exposure_override:Option<f32>, eye:Option<&super::stereoscopy::Eye>) -> [u8;464] {
        let mut envattributes2 = self
            .environment
            .as_ref()
            .map_or([0.0; 4], |environment| environment.envattributes2);
        if let Some(exposure)=self.frame.exposure {envattributes2[0]=exposure;}
        if let Some(exposure)=exposure_override {envattributes2[0]=exposure;}
        let lighting = [
            self.frame.light_direction,
            self.frame.fog_colour,
            self.frame.fog_k1,
            envattributes2,
        ];
        let base = pack_globals(
            &CameraRelativeClip::new(&eye.map_or(self.clip_from_world,|e|e.clip), self.camera()).clip_from_camera,
            &self.view_position,
            &lighting,
            &self.frame.shadows.globals(),
            &self.frame.sky_params,
        );
        let mut bytes = [0; 464];
        bytes[..240].copy_from_slice(&base);
        // PC weather: W carries the cloud cover and Y the storm's gust (both unused by the
        // original shaders, zero for Vanilla).
        let mut animation_time = self.frame.animation_time;
        if crate::weather::current() != crate::weather::Mode::Vanilla {
            use crate::weather::Precipitation;
            let cloud = crate::weather::cloud_cover();
            if cloud > 0. {
                animation_time[3] = cloud;
            }
            let rain = if crate::weather::visible() { crate::weather::current().rain() } else { 0. };
            let phase = (crate::weather::rain_clock() + 3.5).rem_euclid(17.);
            let gust = if (4.0..11.0).contains(&phase) { ((phase - 4.) / 7. * std::f32::consts::PI).sin().powi(2) } else { 0. };
            animation_time[1] = rain * (0.20 + 0.80 * gust);
        }
        for (dst, value) in bytes[240..].chunks_exact_mut(4).zip(
            self.frame
                .view_dot_light
                .into_iter()
                .chain(animation_time)
                .chain(self.frame.pca.into_iter().flatten())
                .chain(self.frame.traffic_lights.into_iter().flatten())
                .chain(self.frame.camera_axes.into_iter().flatten()),
        ) {
            dst.copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }

    /// This view's selection: the visible objects' islands in submission order, kept
    /// while the view's matrix is unchanged. Map generations own immutable
    /// geometry/materials, so streaming retirement drops it with its generation.
    pub(super) fn visible(&self) -> &[(u32, usize)] {
        self.visible.get_or_init(|| self.select_visible())
    }

    /// The map's draws, for the threads that encode its runs.
    pub(super) fn draw_source(&self) -> MapDrawSource<'_> {
        MapDrawSource {
            objects: &self.objects,
            instances: &self.instances,
            environment: self
                .environment
                .as_ref()
                .map(|environment| &*environment.resources),
        }
    }

    /// The objects are culled in chunks across the thread pool; the chunks' selections
    /// join in submission order.
    fn select_visible(&self) -> Vec<(u32, usize)> {
        self.select_visible_for(&self.clip_from_world)
    }
    pub(super) fn select_visible_for(&self, clip: &[f32;16]) -> Vec<(u32,usize)> {
        use rayon::prelude::*;
        let volume = ClipVolume::new(clip);
        let area_cull = self.island_area_cull_for(clip);
        let occluders = self.occluders_for(clip);
        let chunks: Vec<Vec<(u32, usize)>> = self
            .objects
            .par_chunks(SELECTION_CHUNK)
            .enumerate()
            .map(|(chunk, objects)| {
                let first = u32::try_from(chunk * SELECTION_CHUNK).unwrap_or(u32::MAX);
                let mut visible = Vec::new();
                for (instance, object) in (first..).zip(objects) {
                    select_object(
                        instance,
                        object,
                        &volume,
                        area_cull.as_ref(),
                        &occluders,
                        &mut visible,
                    );
                }
                visible
            })
            .collect();
        chunks.concat()
    }

    /// `Frustum::UpdateCullParams` 0x82853970's anti-frustums for this view; a
    /// degenerate view selects none.
    fn occluders_for(&self, clip:&[f32;16]) -> AntiFrustums {
        if self.anti_frustums.is_empty() {
            return AntiFrustums::default();
        }
        let rows = std::array::from_fn(|row| std::array::from_fn(|column| clip[column * 4 + row]));
        let [x, y, z, _] = self.view_position;
        AntiFrustums::try_select(self.anti_frustums.iter().map(AsRef::as_ref), [x, y, z], rows)
            .unwrap_or_default()
    }

    /// `WorldPresEntityOptimesh::CullMeshes_VMX` 0x827d0ae8's island area rule
    /// for this view, from the frame's settings and camera Z axis. Without it,
    /// or for a degenerate view, islands cull by the frustum alone.
    fn island_area_cull_for(&self, clip:&[f32;16]) -> Option<IslandAreaCull> {
        let settings = self.frame.scene_cull?;
        let rows = std::array::from_fn(|row| std::array::from_fn(|column| clip[column * 4 + row]));
        let [x, y, z, _] = self.frame.camera_axes[2];
        IslandAreaCull::try_new(rows, [x, y, z], settings).ok()
    }
}

/// Objects culled per selection chunk on the thread pool.
const SELECTION_CHUNK: usize = 256;

/// `SceneRenderView::Cull` for one object: the islands this view keeps, appended in
/// submission order.
fn select_object(
    instance: u32,
    object: &MapObject,
    volume: &ClipVolume,
    area_cull: Option<&IslandAreaCull>,
    occluders: &AntiFrustums,
    visible: &mut Vec<(u32, usize)>,
) {
    let containment = object
        .bounds
        .map_or(Containment::Intersecting, |bounds| volume.classify(&bounds));
    if containment == Containment::Outside {
        return;
    }
    // `SceneRenderView::Cull` 0x828500a8 tests the entity box first.
    if object
        .shadow
        .as_ref()
        .is_some_and(|shadow| occluders.occludes(&shadow.entity))
    {
        return;
    }
    let islands = object.shadow.as_ref().filter(|shadow| shadow.cull_islands);
    let areas = area_cull.zip(islands.filter(|shadow| !shadow.areas.is_empty()));
    for (index, (_, island)) in object
        .mesh
        .draws
        .iter()
        .zip(object.island_bounds.iter())
        .enumerate()
    {
        if containment == Containment::Intersecting
            && island.is_some_and(|bounds| volume.classify(&bounds) == Containment::Outside)
        {
            continue;
        }
        if let Some((cull, shadow)) = areas {
            let model_island = shadow.range.start + index;
            if let Some(bounds) = shadow.islands.get(model_island)
                && !cull
                    .keeps(bounds, &shadow.areas, model_island)
                    .unwrap_or(true)
            {
                continue;
            }
        }
        if let Some(shadow) = islands
            && !occluders.is_empty()
            && shadow
                .islands
                .get(shadow.range.start + index)
                .is_some_and(|bounds| occluders.occludes(bounds))
        {
            continue;
        }
        visible.push((instance, index));
    }
}

/// WGSL `Globals` order (`environment_shading`): the camera-relative clip
/// matrix, `g_vViewPos`, then `g_vLightDir`, `g_FogColour`, `g_FogK1` and
/// `g_envattributes[2]`, then `CSM_Mat_Row0[0..2]`, `Row1`, `Row2`, then the
/// blended sky `m_params`.
fn pack_globals(
    clip_from_camera: &[f32; 16],
    view_position: &[f32; 4],
    lighting: &[[f32; 4]; 4],
    shadows: &[[f32; 4]; 4],
    sky_params: &[[f32; 4]; 2],
) -> [u8; 240] {
    let values = clip_from_camera
        .iter()
        .chain(view_position)
        .chain(lighting.iter().flatten())
        .chain(shadows.iter().flatten())
        .chain(sky_params.iter().flatten());
    let mut bytes = [0; 240];
    for (destination, value) in bytes.chunks_exact_mut(4).zip(values) {
        destination.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

const SHADER: &str = r"
// The first two `Globals` members: the camera-relative clip matrix and
// g_vViewPos.
struct Camera { clip_from_camera: mat4x4<f32>, view_position: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var diffuse_texture: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;
// @invariant like the original effects' shared g_matVP dp4 sequence; a
// world-space draw's identity instance matrix leaves the position exact, and
// its offset from the camera matches the environment effects' `world_clip`.
struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
}
@vertex fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_column0: vec4<f32>,
    @location(3) world_column1: vec4<f32>,
    @location(4) world_column2: vec4<f32>,
    @location(5) world_column3: vec4<f32>,
    @location(6) color: vec4<f32>
) -> VertexOutput {
    let world_from_mesh = mat4x4<f32>(world_column0, world_column1, world_column2, world_column3);
    let world = world_from_mesh * vec4<f32>(position, 1.0);
    var output: VertexOutput;
    output.position = camera.clip_from_camera * vec4<f32>(world.xyz - camera.view_position.xyz, 1.0);
    output.uv = uv;
    output.color = color;
    return output;
}
@fragment fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> { return input.color; }
@fragment fn fs_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse = textureSample(diffuse_texture, diffuse_sampler, input.uv);
    // Diagnostic display on the gamma-2 (non-sRGB) surface the original
    // effects write: raw diffuse, no lighting or tone map.
    return diffuse * input.color;
}
";

#[cfg(test)]
#[path = "map_draw_tests.rs"]
mod tests;
