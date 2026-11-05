//! Original world-environment shading ported from the shipped effect source.
//!
//! `render.big` ships `data\render\shaders\final\*.updb` debug databases whose
//! `<file id="0">` is the preprocessed HLSL the programs were compiled from
//! (compiler 7645, `SHADER_FINAL`, `rwgDEVICE_XBOX2`). The WGSL below follows
//! `defaultenvironment.fx`, `alphatestdefaultenvironment.fx`,
//! `baseenvironment.fx`, `decalenvironment.fx`, `transparentenvironment.fx`,
//! `decalzbias.fx`, `sky.fx` and `proxyworld.fx` (`default` technique,
//! `defaultVS`/`defaultPS`) with
//! their helpers from `common_shadows.fxh`, `common_functions.fxh`,
//! `common_lighting.fxh`, `common_textures.fxh`, `environment_textures.fxh`,
//! `environment_lighting.fxh` and `postfx_functions.fxh`, in source order.
//! Literal arguments are the compiled call-site constants, not substitutes.
//!
//! Inputs and deviations (see docs/evidence/render-environment-shading):
//! - Output stays in the effects' own gamma-2 space (`MoveFromLinearSpace` is
//!   `sqrt`), so the surface is a non-sRGB target.
//! - `half` is evaluated as f32; Xenos `TRILINEARTHRESHOLD` has no wgpu
//!   equivalent; the manifests' anisotropy 4 is raised to the renderer's 16.
//! - `shadowTexture_CSM0` is the renderer's live atlas (`shadow_map`), read
//!   through the named `ShadowTextureSampler`. Adapters without clamp-to-border
//!   clamp to the edge texel instead of the zero border.
//! - Every pass keeps its manifest CULLMODE (FRONT) and decalzbias its
//!   DEPTHBIAS/SLOPESCALEDEPTHBIAS, through `raster_state`.

use super::{DEPTH_FORMAT, multisample, raster_state::RasterState};
use std::sync::Arc;

/// `GlobalShaderData` subset read by these effects, in WGSL `Globals` order,
/// then the WorldPainter-blended `material_sky/default` `m_params[0..2]`.
pub(crate) const GLOBALS_SIZE: u64 = 464;

/// TEXCOORD1 (lightmap UV, or baseenvironment `lm_norm`), TANGENT, then
/// TEXCOORD0.zw (decalenvironment `uv_decal.zw`, zero for two-lane UV0).
pub(super) const LIGHTING_STRIDE: wgpu::BufferAddress = 36;
const LIGHTING_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![12 => Float32x4, 13 => Float32x3, 14 => Float32x2];
/// Instance record lanes after world-from-mesh and color: lightmap selector,
/// macro-overlay UV scale, detail-normal UV scale, macro-overlay opacity and
/// `m_params[0]`.
const MATERIAL_ATTRIBUTES: [wgpu::VertexAttribute; 9] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 0,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 16,
        shader_location: 3,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 32,
        shader_location: 4,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 48,
        shader_location: 5,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 80,
        shader_location: 7,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 96,
        shader_location: 8,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 112,
        shader_location: 9,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 128,
        shader_location: 10,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 144,
        shader_location: 11,
    },
];

/// Which original effect a retained map draw uses.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum EnvironmentEffect {
    /// `defaultenvironment` (`environmentsimple.default`).
    Default,
    /// `alphatestdefaultenvironment` (`environmentsimple.alphatest`).
    AlphaTest,
    /// `transparentdefaultenvironment`: simple lighting, source-alpha blend.
    TransparentDefault,
    Advertisement,
    /// `tree`: two-sided foliage with native view-dependent lighting.
    Tree,
    /// `treeanimate`: authored three-axis wind, driven by simulation time.
    TreeAnimate,
    VertexAnimate,
    BaseIncandescent,
    ScrollIncandescent,
    TransparentIncandescent,
    ZBiasIncandescent,
    ZBiasIncandescentBacklit,
    /// `baseenvironment` (`environment.default`).
    Base,
    BaseTerrain,
    DecalTerrain,
    BuildingCeilingWorld,
    Water,
    FlowingWater,
    Ocean,
    TrafficLightOne,
    TrafficLightTwo,
    Glare,
    GodraySpotlight,
    /// `decalenvironment`: baseenvironment lighting over a CLAMP decal.
    Decal,
    /// `transparentenvironment`: alpha-blended lightmap and specular.
    Transparent,
    /// `sky` (`material_sky` variants).
    Sky,
    /// Skate 3 `sky` (`sky_defaultPS.fpo`): `Sky`'s inputs, the diffuse alone squared (no detail,
    /// tint or saturation), then the same sun table and tone.
    S3Sky,
    /// `decalzbias` (`material_decal`): baseenvironment's inputs without the
    /// detail, overlay or reflection terms, alpha-blended over the surface it
    /// is depth-biased toward the camera from.
    DecalZBias,
    /// `proxyworld` (`material_proxyworld/default`): the PROXY city's
    /// unlit diffuse with fog.
    ProxyWorld,
    /// Skate 3 `defaultenvironment` (`environmentsimple.default`): normal-mapped lightmap
    /// lighting and a fixed-direction highlight (`S3_LIT`).
    S3Default,
    /// Skate 3 `decalenvironment_simple`: `S3Default` over a CLAMP decal.
    S3DecalSimple,
    /// Skate 3 `decalenvironment_simple_tileable`: the same program over a WRAP decal.
    S3DecalSimpleTileable,
    /// Skate 3 `environmentdiffuse`: diffuse times the shadowed lightmap, short tone curve.
    S3EnvironmentDiffuse,
    /// Skate 3 `alphatestdefaultenvironment`.
    S3AlphaTest,
    /// Skate 3 `transparentdefaultenvironment`.
    S3TransparentDefault,
    /// Skate 3 `baseenvironment` (`environment.default`): `S3Default`'s lighting with the detail
    /// normal and the macro overlay (`S3_BASE`).
    S3Base,
    /// Skate 3 `decalenvironment`: `S3Base` over a CLAMP decal.
    S3Decal,
    /// Skate 3 `decalenvironment_tileable`: the same program over a WRAP decal.
    S3DecalTileable,
    /// Skate 3 `baseenvironmentreflective` (`environment.reflective`): `S3Base` plus the
    /// `environment` cube reflection (`S3_REFLECTION`).
    S3BaseReflective,
    /// Skate 3 `baseenvironmentreflective_simple`: `S3Default` plus the cube reflection.
    S3BaseReflectiveSimple,
    /// Skate 3 `transparentenvironmentreflective` (`environment.reflective_trans`): the shadowed
    /// lightmap, highlight and cube reflection on the interpolated normal, source-alpha blended.
    S3TransparentReflective,
    /// Skate 3 `tree`, `treeanimate` and `vertexanimate`: the lightmap's rgb with no chromaticity,
    /// `treeanimate` with its noise texture.
    S3Tree,
    S3TreeAnimate,
    S3VertexAnimate,
}

impl EnvironmentEffect {
    pub(crate) fn from_manifest(name: &str) -> Option<Self> {
        match name {
            "defaultenvironment" => Some(Self::Default),
            "alphatestdefaultenvironment" => Some(Self::AlphaTest),
            "transparentdefaultenvironment" => Some(Self::TransparentDefault),
            "advertisement" => Some(Self::Advertisement),
            "water" => Some(Self::Water),
            "flowingwater" => Some(Self::FlowingWater),
            "ocean" => Some(Self::Ocean),
            "trafficlight_one" => Some(Self::TrafficLightOne),
            "trafficlight_two" => Some(Self::TrafficLightTwo),
            "glare" => Some(Self::Glare),
            "godrayspotlight" => Some(Self::GodraySpotlight),
            "tree" => Some(Self::Tree),
            "treeanimate" => Some(Self::TreeAnimate),
            "vertexanimate" => Some(Self::VertexAnimate),
            // Final videoscreen VS/PS instruction bytes and manifest states
            // equal baseincandescent (only trailing container hashes differ).
            // Its i_diffuse is PostFX's main render target (main_target.rs).
            "baseincandescent" | "videoscreen" => Some(Self::BaseIncandescent),
            "scrollincandescent" => Some(Self::ScrollIncandescent),
            "transparentincandescent" => Some(Self::TransparentIncandescent),
            "zbiasincandescent" => Some(Self::ZBiasIncandescent),
            "zbiasincandescentbacklit" => Some(Self::ZBiasIncandescentBacklit),
            "baseenvironment" => Some(Self::Base),
            "baseterrain" => Some(Self::BaseTerrain),
            "decalterrain" => Some(Self::DecalTerrain),
            "building_ceiling_worldspace" => Some(Self::BuildingCeilingWorld),
            "decalenvironment" => Some(Self::Decal),
            "transparentenvironment" => Some(Self::Transparent),
            "sky" => Some(Self::Sky),
            "decalzbias" => Some(Self::DecalZBias),
            "proxyworld" => Some(Self::ProxyWorld),
            _ => None,
        }
    }

    /// PC port: a linked Skate 3 data folder's district (`skate2_core::skate3`) names Skate 3's
    /// effects, several under Skate 2's names with Skate 3's own programs; the rest are Skate 2's.
    pub(crate) fn from_skate3_manifest(name: &str) -> Option<Self> {
        Some(match name {
            "defaultenvironment" => Self::S3Default,
            "alphatestdefaultenvironment" => Self::S3AlphaTest,
            "transparentdefaultenvironment" => Self::S3TransparentDefault,
            "decalenvironment_simple" => Self::S3DecalSimple,
            "decalenvironment_simple_tileable" => Self::S3DecalSimpleTileable,
            "environmentdiffuse" => Self::S3EnvironmentDiffuse,
            "baseenvironment" => Self::S3Base,
            "decalenvironment" => Self::S3Decal,
            "decalenvironment_tileable" => Self::S3DecalTileable,
            "baseenvironmentreflective" => Self::S3BaseReflective,
            "baseenvironmentreflective_simple" => Self::S3BaseReflectiveSimple,
            "transparentenvironmentreflective" => Self::S3TransparentReflective,
            "tree" => Self::S3Tree,
            "treeanimate" => Self::S3TreeAnimate,
            "vertexanimate" => Self::S3VertexAnimate,
            "sky" => Self::S3Sky,
            other => return Self::from_manifest(other).or_else(|| Self::skate3_stand_in(other)),
        })
    }

    /// Skate 3 world effects with no ported program, drawn by the nearest family (logged once
    /// each).
    fn skate3_stand_in(name: &str) -> Option<Self> {
        let family = match name {
            "decal2environment" => Self::Decal,
            "oceanreflection" => Self::Ocean,
            "wateralpha" => Self::Water,
            "flowingwateralpha" => Self::FlowingWater,
            _ => return None,
        };
        static LOGGED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        if let Ok(mut logged) = LOGGED.lock()
            && !logged.iter().any(|n| n == name)
        {
            logged.push(name.to_owned());
            eprintln!("world effect {name} drawn as {family:?} (Skate 3 program not ported)");
        }
        Some(family)
    }

    /// Material channels in bind-group order; `environment` is the cube map.
    pub(crate) fn channels(self) -> &'static [&'static str] {
        match self {
            Self::Water => &[
                "diffuse",
                "specular",
                "normal",
                "normal2",
                "environment",
                "lightmap",
                "chromaticity",
            ],
            Self::FlowingWater => &[
                "diffuse",
                "specular",
                "normal",
                "environment",
                "lightmap",
                "chromaticity",
            ],
            Self::Ocean => &[
                "normal",
                "normal2",
                "macrooverlay",
                "environment",
                "lightmap",
                "chromaticity",
            ],
            Self::BuildingCeilingWorld => &[
                "diffuse",
                "specular",
                "normal",
                "diffuse2",
                "environment",
                "lightmap",
                "chromaticity",
            ],
            Self::BaseTerrain => &[
                "diffuse",
                "specular",
                "normal",
                "macrooverlay",
                "lightmap",
                "chromaticity",
                "diffuse2",
                "normal2",
                "specular2",
            ],
            // The final decalterrain program samples i_specular for BOTH layers.
            Self::DecalTerrain => &[
                "diffuse",
                "specular",
                "normal",
                "macrooverlay",
                "lightmap",
                "chromaticity",
                "diffuse2",
                "normal2",
                "decal",
            ],
            Self::Default
            | Self::AlphaTest
            | Self::TransparentDefault
            | Self::Advertisement
            | Self::Tree
            | Self::TreeAnimate
            | Self::VertexAnimate
            | Self::ScrollIncandescent
            | Self::BaseIncandescent
            | Self::TransparentIncandescent
            | Self::ZBiasIncandescent
            | Self::ZBiasIncandescentBacklit => &["diffuse", "lightmap", "chromaticity"],
            Self::Base => &[
                "diffuse",
                "specular",
                "normal",
                "detail",
                "macrooverlay",
                "environment",
                "lightmap",
                "chromaticity",
            ],
            Self::Decal => &[
                "diffuse",
                "specular",
                "normal",
                "detail",
                "macrooverlay",
                "environment",
                "lightmap",
                "chromaticity",
                "decal",
            ],
            Self::Transparent => &[
                "diffuse",
                "specular",
                "environment",
                "lightmap",
                "chromaticity",
            ],
            Self::S3Default => &["diffuse", "specular", "normal", "lightmap"],
            Self::S3DecalSimple | Self::S3DecalSimpleTileable => {
                &["diffuse", "specular", "normal", "lightmap", "decal"]
            }
            Self::S3EnvironmentDiffuse | Self::S3AlphaTest | Self::S3TransparentDefault => {
                &["diffuse", "lightmap"]
            }
            Self::S3Base => &["diffuse", "specular", "normal", "detail", "macrooverlay", "lightmap"],
            Self::S3Decal | Self::S3DecalTileable => {
                &["diffuse", "specular", "normal", "detail", "macrooverlay", "lightmap", "decal"]
            }
            Self::S3BaseReflective => &[
                "diffuse",
                "specular",
                "normal",
                "detail",
                "macrooverlay",
                "lightmap",
                "environment",
            ],
            Self::S3BaseReflectiveSimple => {
                &["diffuse", "specular", "normal", "lightmap", "environment"]
            }
            Self::S3TransparentReflective => &["diffuse", "specular", "lightmap", "environment"],
            Self::S3Tree | Self::S3VertexAnimate => &["diffuse", "lightmap"],
            Self::S3TreeAnimate => &["diffuse", "lightmap", "noise"],
            // Skate 3's sky names no detail; its default fills the slot the program never reads.
            Self::Sky | Self::S3Sky => &["diffuse", "detail", "specular"],
            Self::DecalZBias => &["diffuse", "specular", "normal", "lightmap", "chromaticity"],
            Self::ProxyWorld
            | Self::TrafficLightOne
            | Self::TrafficLightTwo
            | Self::Glare
            | Self::GodraySpotlight => &["diffuse"],
        }
    }

    /// Instance scalar channels `defaultVS` copies into `dataconstants`.
    pub(crate) fn scalars(self) -> &'static [&'static str] {
        match self {
            Self::Water | Self::FlowingWater | Self::TrafficLightOne | Self::TrafficLightTwo => &[],
            Self::Glare => &["glareScale"],
            Self::GodraySpotlight => &[],
            Self::Ocean => &["macroOverlayUVScale"],
            Self::BuildingCeilingWorld => &["floorHeightMeters", "floorCount"],
            Self::BaseTerrain | Self::DecalTerrain => &[
                "macroOverlayUVScale",
                "macroOverlayOpacity",
                "diffuse2UVScale",
            ],
            Self::ScrollIncandescent => &["uAnimationSpeed", "vAnimationSpeed"],
            Self::Default
            | Self::AlphaTest
            | Self::TransparentDefault
            | Self::Advertisement
            | Self::Tree
            | Self::TreeAnimate
            | Self::VertexAnimate
            | Self::BaseIncandescent
            | Self::TransparentIncandescent
            | Self::ZBiasIncandescent
            | Self::ZBiasIncandescentBacklit
            | Self::Transparent
            | Self::Sky
            | Self::S3Sky
            | Self::DecalZBias
            | Self::ProxyWorld
            | Self::S3Default
            | Self::S3DecalSimple
            | Self::S3DecalSimpleTileable
            | Self::S3EnvironmentDiffuse
            | Self::S3AlphaTest
            | Self::S3TransparentDefault
            | Self::S3BaseReflectiveSimple
            | Self::S3TransparentReflective
            | Self::S3Tree
            | Self::S3TreeAnimate
            | Self::S3VertexAnimate => &[],
            Self::Base
            | Self::Decal
            | Self::S3Base
            | Self::S3Decal
            | Self::S3DecalTileable
            | Self::S3BaseReflective => &[
                "macroOverlayUVScale",
                "detailNormalUVScale",
                "macroOverlayOpacity",
            ],
        }
    }

    /// `VS_IN` TEXCOORD1 width: `float2 LM` or `float4 lm_norm`, plus TANGENT.
    /// `sky.fx` and `proxyworld.fx` read only POSITION and TEXCOORD0.
    pub(crate) fn lighting_inputs(self) -> (u8, bool) {
        match self {
            Self::Default
            | Self::AlphaTest
            | Self::TransparentDefault
            | Self::Tree
            | Self::BaseIncandescent
            | Self::ScrollIncandescent
            | Self::TransparentIncandescent
            | Self::ZBiasIncandescent
            | Self::ZBiasIncandescentBacklit
            | Self::Water
            | Self::Ocean
            | Self::TrafficLightOne
            | Self::TrafficLightTwo => (2, false),
            Self::TreeAnimate | Self::VertexAnimate | Self::Advertisement => (4, false),
            Self::Base
            | Self::Decal
            | Self::DecalZBias
            | Self::BaseTerrain
            | Self::BuildingCeilingWorld
            | Self::FlowingWater
            | Self::DecalTerrain => (4, true),
            Self::Transparent => (4, false),
            Self::Sky | Self::S3Sky | Self::ProxyWorld | Self::Glare | Self::GodraySpotlight => (0, false),
            // Skate 3 `defaultVS`/`decalenvironment_simple` read `lm_norm` and the tangent;
            // `environmentdiffuse`, alphatest and transparent read two lanes of TEXCOORD1.
            Self::S3Default
            | Self::S3DecalSimple
            | Self::S3DecalSimpleTileable
            | Self::S3Base
            | Self::S3Decal
            | Self::S3DecalTileable
            | Self::S3BaseReflective
            | Self::S3BaseReflectiveSimple => (4, true),
            Self::S3EnvironmentDiffuse | Self::S3AlphaTest | Self::S3TransparentDefault => (2, false),
            // transparentenvironmentreflective_defaultVS: all four TEXCOORD1 lanes, no TANGENT.
            Self::S3TransparentReflective => (4, false),
            Self::S3Tree => (2, false),
            Self::S3TreeAnimate | Self::S3VertexAnimate => (4, false),
        }
    }

    /// decalenvironment reads `float4 uv_decal : TEXCOORD0`, zw the decal UV.
    pub(crate) fn reads_decal_uv(self) -> bool {
        matches!(
            self,
            Self::Decal
                | Self::DecalTerrain
                | Self::TrafficLightOne
                | Self::TrafficLightTwo
                | Self::S3DecalSimple
                | Self::S3DecalSimpleTileable
                | Self::S3Decal
                | Self::S3DecalTileable
        )
    }
}

pub(super) struct EnvironmentPipelines {
    simple_layout: wgpu::BindGroupLayout,
    base_layout: wgpu::BindGroupLayout,
    terrain_layout: wgpu::BindGroupLayout,
    building_layout: wgpu::BindGroupLayout,
    water_layouts: [wgpu::BindGroupLayout; 3],
    decal_layout: wgpu::BindGroupLayout,
    transparent_layout: wgpu::BindGroupLayout,
    sky_layout: wgpu::BindGroupLayout,
    decal_zbias_layout: wgpu::BindGroupLayout,
    proxy_world_layout: wgpu::BindGroupLayout,
    s3_lit_layout: wgpu::BindGroupLayout,
    s3_decal_layout: wgpu::BindGroupLayout,
    s3_unlit_layout: wgpu::BindGroupLayout,
    s3_tree_animate_layout: wgpu::BindGroupLayout,
    s3_base_layout: wgpu::BindGroupLayout,
    s3_base_decal_layout: wgpu::BindGroupLayout,
    /// baseenvironmentreflective, its `_simple` and transparentenvironmentreflective.
    s3_reflective_layouts: [wgpu::BindGroupLayout; 3],
    resources_layout: wgpu::BindGroupLayout,
    wrap_sampler: wgpu::Sampler,
    clamp_sampler: wgpu::Sampler,
    cube_sampler: wgpu::Sampler,
    sky_sampler: wgpu::Sampler,
    fresnel_sampler: wgpu::Sampler,
    shadow_sampler: wgpu::Sampler,
    pass_reset_sampler: wgpu::Sampler,
    default: Arc<wgpu::RenderPipeline>,
    alpha_test: Arc<wgpu::RenderPipeline>,
    transparent_default: Arc<wgpu::RenderPipeline>,
    advertisement: Arc<wgpu::RenderPipeline>,
    tree: Arc<wgpu::RenderPipeline>,
    tree_animate: Arc<wgpu::RenderPipeline>,
    vertex_animate: Arc<wgpu::RenderPipeline>,
    scroll_incandescent: Arc<wgpu::RenderPipeline>,
    incandescent: [Arc<wgpu::RenderPipeline>; 4],
    base: Arc<wgpu::RenderPipeline>,
    terrain: [Arc<wgpu::RenderPipeline>; 2],
    building: Arc<wgpu::RenderPipeline>,
    water: [Arc<wgpu::RenderPipeline>; 3],
    street: [Arc<wgpu::RenderPipeline>; 4],
    decal: Arc<wgpu::RenderPipeline>,
    transparent: Arc<wgpu::RenderPipeline>,
    sky: Arc<wgpu::RenderPipeline>,
    s3_sky: Arc<wgpu::RenderPipeline>,
    decal_zbias: Arc<wgpu::RenderPipeline>,
    proxy_world: Arc<wgpu::RenderPipeline>,
    s3_default: Arc<wgpu::RenderPipeline>,
    s3_decal_simple: Arc<wgpu::RenderPipeline>,
    s3_environment_diffuse: Arc<wgpu::RenderPipeline>,
    s3_alpha_test: Arc<wgpu::RenderPipeline>,
    s3_transparent_default: Arc<wgpu::RenderPipeline>,
    s3_base: Arc<wgpu::RenderPipeline>,
    s3_base_decal: Arc<wgpu::RenderPipeline>,
    s3_reflective: [Arc<wgpu::RenderPipeline>; 3],
    s3_tree: Arc<wgpu::RenderPipeline>,
    s3_tree_animate: Arc<wgpu::RenderPipeline>,
    s3_vertex_animate: Arc<wgpu::RenderPipeline>,
}

fn texture_entry(
    binding: u32,
    dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: dimension,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn sampler(
    device: &wgpu::Device,
    label: &str,
    address: wgpu::AddressMode,
    filter: wgpu::FilterMode,
    mipmap: wgpu::MipmapFilterMode,
    anisotropy_clamp: u16,
) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: address,
        address_mode_v: address,
        address_mode_w: address,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: mipmap,
        anisotropy_clamp,
        ..Default::default()
    })
}

impl EnvironmentPipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        globals_layout: &wgpu::BindGroupLayout,
        mesh_attributes: &[wgpu::VertexAttribute],
        instance_stride: wgpu::BufferAddress,
    ) -> Self {
        use wgpu::TextureViewDimension::{Cube, D2};
        // Opt-in receiver diagnosis, folded out of normal shaders. Raw view
        // separates atlas/depth failures from native normal-map shadow gating.
        let receiver_view = match std::env::var("SKATE2_SHADOW_RECEIVER_VIEW").as_deref() {
            Ok("raw") => 1,
            Ok("normal") => 2,
            _ => 0,
        };
        let common = COMMON.replace("const SHADOW_RECEIVER_VIEW:u32=0u;",
            &format!("const SHADOW_RECEIVER_VIEW:u32={receiver_view}u;"));
        let simple_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("defaultenvironment material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                sampler_entry(3),
                sampler_entry(4),
            ],
        });
        let base_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("baseenvironment material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                texture_entry(3, D2),
                texture_entry(4, D2),
                texture_entry(5, Cube),
                texture_entry(6, D2),
                texture_entry(7, D2),
                sampler_entry(8),
                sampler_entry(9),
                sampler_entry(10),
            ],
        });
        let terrain_entries: Vec<_> = (0..9)
            .map(|index| texture_entry(index, D2))
            .chain([sampler_entry(9), sampler_entry(10)])
            .collect();
        let terrain_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain material"),
            entries: &terrain_entries,
        });
        let building_entries: Vec<_> = (0..7)
            .map(|index| texture_entry(index, if index == 4 { Cube } else { D2 }))
            .chain([sampler_entry(7), sampler_entry(8), sampler_entry(9)])
            .collect();
        let building_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("building ceiling material"),
            entries: &building_entries,
        });
        let water_effects = [
            EnvironmentEffect::Water,
            EnvironmentEffect::FlowingWater,
            EnvironmentEffect::Ocean,
        ];
        let water_layouts = water_effects.map(|effect| {
            let count = effect.channels().len() as u32;
            let entries: Vec<_> = effect
                .channels()
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    texture_entry(index as u32, if *name == "environment" { Cube } else { D2 })
                })
                .chain((count..count + 3).map(sampler_entry))
                .collect();
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("native water material"),
                entries: &entries,
            })
        });
        let decal_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("decalenvironment material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                texture_entry(3, D2),
                texture_entry(4, D2),
                texture_entry(5, Cube),
                texture_entry(6, D2),
                texture_entry(7, D2),
                texture_entry(8, D2),
                sampler_entry(9),
                sampler_entry(10),
                sampler_entry(11),
            ],
        });
        let transparent_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("transparentenvironment material"),
                entries: &[
                    texture_entry(0, D2),
                    texture_entry(1, D2),
                    texture_entry(2, Cube),
                    texture_entry(3, D2),
                    texture_entry(4, D2),
                    sampler_entry(5),
                    sampler_entry(6),
                    sampler_entry(7),
                ],
            });
        let sky_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                sampler_entry(3),
                sampler_entry(4),
            ],
        });
        let decal_zbias_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("decalzbias material"),
                entries: &[
                    texture_entry(0, D2),
                    texture_entry(1, D2),
                    texture_entry(2, D2),
                    texture_entry(3, D2),
                    texture_entry(4, D2),
                    sampler_entry(5),
                    sampler_entry(6),
                ],
            });
        let proxy_world_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("proxyworld material"),
                entries: &[texture_entry(0, D2), sampler_entry(1)],
            });
        let s3_lit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skate 3 defaultenvironment material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                texture_entry(3, D2),
                sampler_entry(4),
                sampler_entry(5),
            ],
        });
        let s3_decal_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skate 3 decalenvironment_simple material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                texture_entry(3, D2),
                texture_entry(4, D2),
                sampler_entry(5),
                sampler_entry(6),
                sampler_entry(7),
            ],
        });
        let s3_unlit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skate 3 environmentdiffuse material"),
            entries: &[texture_entry(0, D2), texture_entry(1, D2), sampler_entry(2), sampler_entry(3)],
        });
        let s3_tree_animate_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skate 3 treeanimate material"),
            entries: &[
                texture_entry(0, D2),
                texture_entry(1, D2),
                texture_entry(2, D2),
                sampler_entry(3),
                sampler_entry(4),
                sampler_entry(5),
            ],
        });
        let s3_base_entries: Vec<_> = (0..6)
            .map(|index| texture_entry(index, D2))
            .chain([sampler_entry(6), sampler_entry(7)])
            .collect();
        let s3_base_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skate 3 baseenvironment material"),
            entries: &s3_base_entries,
        });
        let s3_base_decal_entries: Vec<_> = (0..7)
            .map(|index| texture_entry(index, D2))
            .chain([sampler_entry(7), sampler_entry(8), sampler_entry(9)])
            .collect();
        let s3_base_decal_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skate 3 decalenvironment material"),
            entries: &s3_base_decal_entries,
        });
        // Each reflective effect's channels end with the `environment` cube.
        let s3_reflective_layouts = [
            EnvironmentEffect::S3BaseReflective,
            EnvironmentEffect::S3BaseReflectiveSimple,
            EnvironmentEffect::S3TransparentReflective,
        ]
        .map(|effect| {
            let count = effect.channels().len() as u32;
            let entries: Vec<_> = (0..count)
                .map(|index| texture_entry(index, if index + 1 == count { Cube } else { D2 }))
                .chain((count..count + 3).map(sampler_entry))
                .collect();
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Skate 3 reflective environment material"),
                entries: &entries,
            })
        });
        let resources_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("environment global resources"),
            entries: &[
                texture_entry(0, D2),
                sampler_entry(1),
                texture_entry(2, D2),
                sampler_entry(3),
            ],
        });
        // Manifest sampler states: diffuse-family ANISOTROPIC/LINEAR mip with
        // default WRAP; lightmap/chromaticity/decal add CLAMP; the cube map is
        // CLAMP, LINEAR min/mag and POINT mip.
        let wrap_sampler = sampler(
            device,
            "environment diffuse-family wrap",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::MipmapFilterMode::Linear,
            16,
        );
        let clamp_sampler = sampler(
            device,
            "environment lightmap/chromaticity clamp",
            wgpu::AddressMode::ClampToEdge,
            wgpu::FilterMode::Linear,
            wgpu::MipmapFilterMode::Linear,
            16,
        );
        let cube_sampler = sampler(
            device,
            "environment reflection cube",
            wgpu::AddressMode::ClampToEdge,
            wgpu::FilterMode::Linear,
            wgpu::MipmapFilterMode::Nearest,
            1,
        );
        // sky.xml `i_diffuse`/`i_detail`: ADDRESSU WRAP, ADDRESSV CLAMP,
        // ANISOTROPIC min/mag, LINEAR mip; `i_specular` is the CLAMP sampler.
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sky gradient/detail wrap-u clamp-v"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        // `g_fresnelLookupTexture` has no sampler_state block, so
        // `cFxParser::EndElement("Pass")` 0x82e1fb60..fbc4 binds the pass's
        // reset definition: WRAP, LINEAR min/mag, POINT mip.
        let fresnel_sampler = sampler(
            device,
            "fresnel lookup pass-default sampler",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::MipmapFilterMode::Nearest,
            1,
        );
        // EndElement 0x82e1f8e8..f954 gives `shadowTexture_CSM0` the named
        // `ShadowTextureSampler` (`AddDefaultSamplerStates` 0x82e18cbc..8d08):
        // BORDER u/v with a zero border, WRAP w, LINEAR min/mag, no mip.
        let border = device
            .features()
            .contains(wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER);
        let shadow_address = if border {
            wgpu::AddressMode::ClampToBorder
        } else {
            wgpu::AddressMode::ClampToEdge
        };
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ShadowTextureSampler"),
            address_mode_u: shadow_address,
            address_mode_v: shadow_address,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            border_color: border.then_some(wgpu::SamplerBorderColor::TransparentBlack),
            ..Default::default()
        });
        // proxyworld.xml has no sampler block, so `EndElement("Pass")`
        // 0x82e1fb64..fbc4 gives `i_diffuse` the slot's reset
        // `cSamplerStateDefinition` (0x82e1fd70..fdac): WRAP u/v/w, LINEAR
        // mag/min, POINT mip, anisotropy 1, LOD bias 0.
        let pass_reset_sampler = sampler(
            device,
            "proxyworld i_diffuse pass-reset sampler",
            wgpu::AddressMode::Repeat,
            wgpu::FilterMode::Linear,
            wgpu::MipmapFilterMode::Nearest,
            1,
        );
        let simple_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("defaultenvironment pipeline layout"),
                bind_group_layouts: &[Some(globals_layout), Some(&simple_layout), Some(&resources_layout)],
                immediate_size: 0,
            });
        let base_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("baseenvironment pipeline layout"),
            bind_group_layouts: &[Some(globals_layout), Some(&base_layout), Some(&resources_layout)],
            immediate_size: 0,
        });
        let default_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("defaultenvironment.fx"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{common}{DEFAULT_ENVIRONMENT}{}",
                    include_str!("simple_materials.wgsl")
                )
                .into(),
            ),
        });
        let terrain_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("terrain pipeline"),
                bind_group_layouts: &[Some(globals_layout), Some(&terrain_layout), Some(&resources_layout)],
                immediate_size: 0,
            });
        let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("baseterrain/decalterrain final programs"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{common}{ENVIRONMENT_VS}{}",
                    include_str!("terrain_material.wgsl")
                )
                .into(),
            ),
        });
        let building_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("building ceiling pipeline"),
                bind_group_layouts: &[Some(globals_layout), Some(&building_layout), Some(&resources_layout)],
                immediate_size: 0,
            });
        let building_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("building_ceiling_worldspace final programs"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{common}{ENVIRONMENT_VS}{SPECULAR}{}",
                    include_str!("building_material.wgsl")
                )
                .into(),
            ),
        });
        let base_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("baseenvironment.fx"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{BASE_BINDINGS}{SPECULAR}{ENVIRONMENT_VS}{BASE_ENVIRONMENT}")
                    .into(),
            ),
        });
        let decal_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("decalenvironment pipeline layout"),
                bind_group_layouts: &[Some(globals_layout), Some(&decal_layout), Some(&resources_layout)],
                immediate_size: 0,
            });
        let decal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("decalenvironment.fx"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{common}{DECAL_BINDINGS}{SPECULAR}{ENVIRONMENT_VS}{BASE_ENVIRONMENT}{DECAL_ENVIRONMENT}"
                )
                .into(),
            ),
        });
        let transparent_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("transparentenvironment pipeline layout"),
                bind_group_layouts: &[Some(globals_layout), Some(&transparent_layout), Some(&resources_layout)],
                immediate_size: 0,
            });
        let transparent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("transparentenvironment.fx"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{TRANSPARENT_BINDINGS}{SPECULAR}{TRANSPARENT_ENVIRONMENT}").into(),
            ),
        });
        let sky_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky pipeline layout"),
            bind_group_layouts: &[Some(globals_layout), Some(&sky_layout)],
            immediate_size: 0,
        });
        let sky_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky.fx"),
            source: wgpu::ShaderSource::Wgsl(format!("{common}{SKY_VS}{SKY_PS}").into()),
        });
        let s3_sky_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skate 3 sky.fx"),
            source: wgpu::ShaderSource::Wgsl(format!("{common}{SKY_VS}{S3_SKY_PS}").into()),
        });
        let decal_zbias_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("decalzbias pipeline layout"),
                bind_group_layouts: &[Some(globals_layout), Some(&decal_zbias_layout), Some(&resources_layout)],
                immediate_size: 0,
            });
        let decal_zbias_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("decalzbias.fx"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{DECAL_ZBIAS_BINDINGS}{ENVIRONMENT_VS}{DECAL_ZBIAS}").into(),
            ),
        });
        let proxy_world_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("proxyworld pipeline layout"),
                bind_group_layouts: &[Some(globals_layout), Some(&proxy_world_layout)],
                immediate_size: 0,
            });
        let proxy_world_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("proxyworld.fx"),
            source: wgpu::ShaderSource::Wgsl(format!("{common}{PROXY_WORLD}").into()),
        });
        let s3_pipeline_layout = |label, layout: &wgpu::BindGroupLayout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(globals_layout), Some(layout), Some(&resources_layout)],
                immediate_size: 0,
            })
        };
        let s3_lit_pipeline_layout = s3_pipeline_layout("Skate 3 defaultenvironment pipeline", &s3_lit_layout);
        let s3_decal_pipeline_layout =
            s3_pipeline_layout("Skate 3 decalenvironment_simple pipeline", &s3_decal_layout);
        let s3_unlit_pipeline_layout =
            s3_pipeline_layout("Skate 3 environmentdiffuse pipeline", &s3_unlit_layout);
        let s3_tree_animate_pipeline_layout =
            s3_pipeline_layout("Skate 3 treeanimate pipeline", &s3_tree_animate_layout);
        let s3_base_pipeline_layout = s3_pipeline_layout("Skate 3 baseenvironment pipeline", &s3_base_layout);
        let s3_base_decal_pipeline_layout =
            s3_pipeline_layout("Skate 3 decalenvironment pipeline", &s3_base_decal_layout);
        let s3_lit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate 3 defaultenvironment"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{S3_LIT_BINDINGS}{ENVIRONMENT_VS}{S3_LIGHTMAP}{S3_LIT}{S3_DEFAULT_PS}").into(),
            ),
        });
        let s3_decal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate 3 decalenvironment_simple"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{S3_DECAL_BINDINGS}{ENVIRONMENT_VS}{S3_LIGHTMAP}{S3_LIT}{S3_DECAL_VS}{S3_DECAL}")
                    .into(),
            ),
        });
        let s3_base_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate 3 baseenvironment"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{S3_BASE_BINDINGS}{ENVIRONMENT_VS}{S3_LIGHTMAP}{S3_BASE}{S3_BASE_PS}").into(),
            ),
        });
        let s3_base_decal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate 3 decalenvironment"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{common}{S3_BASE_DECAL_BINDINGS}{ENVIRONMENT_VS}{S3_LIGHTMAP}{S3_BASE}{S3_DECAL_VS}{S3_BASE_DECAL_PS}"
                )
                .into(),
            ),
        });
        let s3_unlit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate 3 environmentdiffuse, alphatest and transparent default"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{S3_UNLIT_BINDINGS}{S3_LIGHTMAP}{S3_UNLIT}").into(),
            ),
        });
        let s3_tree_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate 3 tree and vertexanimate"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{S3_UNLIT_BINDINGS}{S3_FOLIAGE}{S3_TREE}").into(),
            ),
        });
        let s3_tree_animate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate 3 treeanimate"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{S3_TREE_ANIMATE_BINDINGS}{S3_FOLIAGE}{S3_TREE_ANIMATE}").into(),
            ),
        });
        let s3_reflective_shaders = [
            (
                "Skate 3 baseenvironmentreflective",
                format!(
                    "{common}{S3_BASE_REFLECTIVE_BINDINGS}{ENVIRONMENT_VS}{S3_LIGHTMAP}{S3_BASE}{S3_REFLECTION}{S3_BASE_REFLECTIVE_PS}"
                ),
            ),
            (
                "Skate 3 baseenvironmentreflective_simple",
                format!(
                    "{common}{S3_REFLECTIVE_SIMPLE_BINDINGS}{ENVIRONMENT_VS}{S3_LIGHTMAP}{S3_LIT}{S3_REFLECTION}{S3_REFLECTIVE_SIMPLE_PS}"
                ),
            ),
            (
                "Skate 3 transparentenvironmentreflective",
                format!(
                    "{common}{S3_TRANSPARENT_REFLECTIVE_BINDINGS}{S3_LIGHTMAP}{S3_REFLECTION}{S3_TRANSPARENT_REFLECTIVE}"
                ),
            ),
        ]
        .map(|(label, source)| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        });
        let s3_reflective_pipeline_layouts = s3_reflective_layouts
            .each_ref()
            .map(|layout| s3_pipeline_layout("Skate 3 reflective environment pipeline", layout));
        // Every ported `default` pass states CULLMODE=FRONT.
        let target = |write_mask| EffectTarget {
            format,
            sample_count,
            write_mask,
            blend: None,
            raster: RasterState::FRONT,
            mesh_attributes,
            instance_stride,
            lighting: true,
            depth_write: true,
        };
        let street_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native traffic lights and glare"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{common}{}", include_str!("street_material.wgsl")).into(),
            ),
        });
        let street = std::array::from_fn(|index| {
            let mut state = target(if index >= 2 {
                wgpu::ColorWrites::ALL
            } else {
                wgpu::ColorWrites::COLOR
            });
            state.raster = RasterState::default();
            if index >= 2 {
                let add = wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                };
                state.blend = Some(wgpu::BlendState {
                    color: add,
                    alpha: add,
                });
                state.depth_write = false;
                state.lighting = index != 2;
                if index == 3 {
                    state.raster = RasterState::FRONT;
                }
            }
            Arc::new(effect_pipeline(
                device,
                &street_shader,
                &proxy_world_pipeline_layout,
                [
                    ["trafficOneVS", "trafficPS"],
                    ["trafficTwoVS", "trafficPS"],
                    ["glareVS", "glarePS"],
                    ["spotlightVS", "spotlightPS"],
                ][index],
                state,
            ))
        });
        let water = std::array::from_fn(|index| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("native water pipeline"),
                bind_group_layouts: &[Some(globals_layout), Some(&water_layouts[index]), Some(&resources_layout)],
                immediate_size: 0,
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("native water final program"),
                source: wgpu::ShaderSource::Wgsl(water_source(water_effects[index]).into()),
            });
            Arc::new(effect_pipeline(
                device,
                &shader,
                &layout,
                ["waterVS", ["waterPS", "flowingPS", "oceanPS"][index]],
                target(wgpu::ColorWrites::ALL),
            ))
        });
        Self {
            water,
            street,
            water_layouts,
            building: Arc::new(effect_pipeline(
                device,
                &building_shader,
                &building_pipeline_layout,
                ["defaultVS", "buildingPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            advertisement: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["advertisementVS", "advertisementPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            terrain: ["baseTerrainPS", "decalTerrainPS"].map(|fragment| {
                Arc::new(effect_pipeline(
                    device,
                    &terrain_shader,
                    &terrain_pipeline_layout,
                    ["terrainVS", fragment],
                    target(wgpu::ColorWrites::ALL),
                ))
            }),
            vertex_animate: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["treeAnimateVS", "vertexAnimatePS"],
                EffectTarget {
                    raster: RasterState::default(),
                    ..target(wgpu::ColorWrites::COLOR)
                },
            )),
            scroll_incandescent: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["scrollIncandescentVS", "incandescentPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            incandescent: std::array::from_fn(|index| {
                let mut state = target(if index == 2 {
                    wgpu::ColorWrites::COLOR
                } else {
                    wgpu::ColorWrites::ALL
                });
                if index == 1 || index == 2 {
                    let additive = wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    };
                    state.blend = Some(wgpu::BlendState {
                        color: additive,
                        alpha: additive,
                    });
                }
                if index >= 2 {
                    state.raster = RasterState {
                        depth_bias: -0.0001,
                        slope_scale: 1.0,
                        ..RasterState::FRONT
                    };
                }
                Arc::new(effect_pipeline(
                    device,
                    &default_shader,
                    &simple_pipeline_layout,
                    [
                        "defaultVS",
                        [
                            "incandescentPS",
                            "incandescentAlphaPS",
                            "incandescentAlphaPS",
                            "incandescentBacklitPS",
                        ][index],
                    ],
                    state,
                ))
            }),
            tree_animate: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["treeAnimateVS", "treePS"],
                EffectTarget {
                    raster: RasterState::default(),
                    ..target(wgpu::ColorWrites::COLOR)
                },
            )),
            transparent_default: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["defaultVS", "transparentDefaultPS"],
                EffectTarget {
                    blend: Some(wgpu::BlendState {
                        color: SOURCE_ALPHA_BLEND,
                        alpha: SOURCE_ALPHA_BLEND,
                    }),
                    ..target(wgpu::ColorWrites::ALL)
                },
            )),
            tree: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["treeVS", "treePS"],
                EffectTarget {
                    raster: RasterState::default(),
                    ..target(wgpu::ColorWrites::ALL)
                },
            )),
            default: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["defaultVS", "defaultPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            // alphatestdefaultenvironment: COLORWRITE=RGB, ALPHAREF=30 GREATEREQUAL.
            alpha_test: Arc::new(effect_pipeline(
                device,
                &default_shader,
                &simple_pipeline_layout,
                ["defaultVS", "alphatestPS"],
                target(wgpu::ColorWrites::COLOR),
            )),
            base: Arc::new(effect_pipeline(
                device,
                &base_shader,
                &base_pipeline_layout,
                ["defaultVS", "defaultPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            // decalenvironment.xml `default` pass: only CULLMODE=FRONT, so
            // opaque with depth write and every channel written.
            decal: Arc::new(effect_pipeline(
                device,
                &decal_shader,
                &decal_pipeline_layout,
                ["decalVS", "decalPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            // transparentenvironment.xml `default` pass: ALPHABLEND with
            // SRCALPHA/INVSRCALPHA ADD (alpha the same), ALPHATEST
            // GREATEREQUAL 16 (in the program), ZWRITEENABLE, COLORWRITE=RGB.
            transparent: Arc::new(effect_pipeline(
                device,
                &transparent_shader,
                &transparent_pipeline_layout,
                ["defaultVS", "defaultPS"],
                EffectTarget {
                    blend: Some(wgpu::BlendState {
                        color: SOURCE_ALPHA_BLEND,
                        alpha: SOURCE_ALPHA_BLEND,
                    }),
                    ..target(wgpu::ColorWrites::COLOR)
                },
            )),
            // sky.xml `default` pass: only CULLMODE=FRONT; depth on.
            sky: Arc::new(effect_pipeline(
                device,
                &sky_shader,
                &sky_pipeline_layout,
                ["defaultVS", "defaultPS"],
                EffectTarget {
                    lighting: false,
                    ..target(wgpu::ColorWrites::ALL)
                },
            )),
            // Skate 3's sky: the same pass and inputs as Skate 2's.
            s3_sky: Arc::new(effect_pipeline(
                device,
                &s3_sky_shader,
                &sky_pipeline_layout,
                ["defaultVS", "defaultPS"],
                EffectTarget {
                    lighting: false,
                    ..target(wgpu::ColorWrites::ALL)
                },
            )),
            // decalzbias.xml `default` pass: ALPHABLEND with SRCALPHA/
            // INVSRCALPHA ADD (alpha the same), ALPHATEST GREATEREQUAL 16 (in
            // the program), ZENABLE/ZWRITEENABLE, COLORWRITE=RGB, CULLMODE=FRONT,
            // DEPTHBIAS=-0.0001, SLOPESCALEDEPTHBIAS=1.0.
            decal_zbias: Arc::new(effect_pipeline(
                device,
                &decal_zbias_shader,
                &decal_zbias_pipeline_layout,
                ["defaultVS", "defaultPS"],
                EffectTarget {
                    blend: Some(wgpu::BlendState {
                        color: SOURCE_ALPHA_BLEND,
                        alpha: SOURCE_ALPHA_BLEND,
                    }),
                    raster: RasterState {
                        depth_bias: -0.0001,
                        slope_scale: 1.0,
                        ..RasterState::FRONT
                    },
                    ..target(wgpu::ColorWrites::COLOR)
                },
            )),
            // proxyworld.xml `default` pass: only CULLMODE=FRONT, so opaque
            // with depth write and every channel written; no TEXCOORD1.
            proxy_world: Arc::new(effect_pipeline(
                device,
                &proxy_world_shader,
                &proxy_world_pipeline_layout,
                ["defaultVS", "defaultPS"],
                EffectTarget {
                    lighting: false,
                    ..target(wgpu::ColorWrites::ALL)
                },
            )),
            s3_default: Arc::new(effect_pipeline(
                device,
                &s3_lit_shader,
                &s3_lit_pipeline_layout,
                ["defaultVS", "defaultPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            s3_decal_simple: Arc::new(effect_pipeline(
                device,
                &s3_decal_shader,
                &s3_decal_pipeline_layout,
                ["decalVS", "decalPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            s3_environment_diffuse: Arc::new(effect_pipeline(
                device,
                &s3_unlit_shader,
                &s3_unlit_pipeline_layout,
                ["diffuseVS", "diffusePS"],
                target(wgpu::ColorWrites::ALL),
            )),
            // As Skate 2's alphatestdefaultenvironment: COLORWRITE=RGB, ALPHAREF=30 GREATEREQUAL.
            s3_alpha_test: Arc::new(effect_pipeline(
                device,
                &s3_unlit_shader,
                &s3_unlit_pipeline_layout,
                ["defaultVS", "alphatestPS"],
                target(wgpu::ColorWrites::COLOR),
            )),
            s3_transparent_default: Arc::new(effect_pipeline(
                device,
                &s3_unlit_shader,
                &s3_unlit_pipeline_layout,
                ["defaultVS", "transparentPS"],
                EffectTarget {
                    blend: Some(wgpu::BlendState {
                        color: SOURCE_ALPHA_BLEND,
                        alpha: SOURCE_ALPHA_BLEND,
                    }),
                    ..target(wgpu::ColorWrites::ALL)
                },
            )),
            s3_base: Arc::new(effect_pipeline(
                device,
                &s3_base_shader,
                &s3_base_pipeline_layout,
                ["defaultVS", "defaultPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            s3_base_decal: Arc::new(effect_pipeline(
                device,
                &s3_base_decal_shader,
                &s3_base_decal_pipeline_layout,
                ["decalVS", "decalPS"],
                target(wgpu::ColorWrites::ALL),
            )),
            // Both baseenvironmentreflective techniques bind baseenvironment's states
            // (0x8307DDA0, 0x8307DE80, 0x830782E0); transparentenvironmentreflective's blend
            // 0x8307DEEC is SRCALPHA/INVSRCALPHA ADD with ALPHATEST GREATEREQUAL 16 (in the program)
            // and depth 0x8307DDA8 writes Z.
            s3_reflective: std::array::from_fn(|index| {
                let state = if index == 2 {
                    EffectTarget {
                        blend: Some(wgpu::BlendState {
                            color: SOURCE_ALPHA_BLEND,
                            alpha: SOURCE_ALPHA_BLEND,
                        }),
                        ..target(wgpu::ColorWrites::ALL)
                    }
                } else {
                    target(wgpu::ColorWrites::ALL)
                };
                Arc::new(effect_pipeline(
                    device,
                    &s3_reflective_shaders[index],
                    &s3_reflective_pipeline_layouts[index],
                    ["defaultVS", "defaultPS"],
                    state,
                ))
            }),
            s3_reflective_layouts,
            s3_lit_layout,
            s3_decal_layout,
            s3_unlit_layout,
            s3_tree_animate_layout,
            s3_base_layout,
            s3_base_decal_layout,
            s3_tree: Arc::new(effect_pipeline(
                device,
                &s3_tree_shader,
                &s3_unlit_pipeline_layout,
                ["treeVS", "treePS"],
                EffectTarget {
                    raster: RasterState::default(),
                    ..target(wgpu::ColorWrites::ALL)
                },
            )),
            s3_tree_animate: Arc::new(effect_pipeline(
                device,
                &s3_tree_animate_shader,
                &s3_tree_animate_pipeline_layout,
                ["treeAnimateVS", "treeAnimatePS"],
                EffectTarget {
                    raster: RasterState::default(),
                    ..target(wgpu::ColorWrites::COLOR)
                },
            )),
            s3_vertex_animate: Arc::new(effect_pipeline(
                device,
                &s3_tree_shader,
                &s3_unlit_pipeline_layout,
                ["vertexAnimateVS", "vertexAnimatePS"],
                EffectTarget {
                    raster: RasterState::default(),
                    ..target(wgpu::ColorWrites::COLOR)
                },
            )),
            simple_layout,
            base_layout,
            terrain_layout,
            building_layout,
            decal_layout,
            transparent_layout,
            sky_layout,
            decal_zbias_layout,
            proxy_world_layout,
            resources_layout,
            wrap_sampler,
            clamp_sampler,
            cube_sampler,
            sky_sampler,
            fresnel_sampler,
            shadow_sampler,
            pass_reset_sampler,
        }
    }

    pub(super) fn pipeline(&self, effect: EnvironmentEffect) -> Arc<wgpu::RenderPipeline> {
        Arc::clone(match effect {
            EnvironmentEffect::Default => &self.default,
            EnvironmentEffect::AlphaTest => &self.alpha_test,
            EnvironmentEffect::TransparentDefault => &self.transparent_default,
            EnvironmentEffect::Advertisement => &self.advertisement,
            EnvironmentEffect::Tree => &self.tree,
            EnvironmentEffect::TreeAnimate => &self.tree_animate,
            EnvironmentEffect::VertexAnimate => &self.vertex_animate,
            EnvironmentEffect::ScrollIncandescent => &self.scroll_incandescent,
            EnvironmentEffect::BaseIncandescent => &self.incandescent[0],
            EnvironmentEffect::TransparentIncandescent => &self.incandescent[1],
            EnvironmentEffect::ZBiasIncandescent => &self.incandescent[2],
            EnvironmentEffect::ZBiasIncandescentBacklit => &self.incandescent[3],
            EnvironmentEffect::Base => &self.base,
            EnvironmentEffect::BaseTerrain => &self.terrain[0],
            EnvironmentEffect::BuildingCeilingWorld => &self.building,
            EnvironmentEffect::Water => &self.water[0],
            EnvironmentEffect::FlowingWater => &self.water[1],
            EnvironmentEffect::Ocean => &self.water[2],
            EnvironmentEffect::TrafficLightOne => &self.street[0],
            EnvironmentEffect::TrafficLightTwo => &self.street[1],
            EnvironmentEffect::Glare => &self.street[2],
            EnvironmentEffect::GodraySpotlight => &self.street[3],
            EnvironmentEffect::DecalTerrain => &self.terrain[1],
            EnvironmentEffect::Decal => &self.decal,
            EnvironmentEffect::Transparent => &self.transparent,
            EnvironmentEffect::Sky => &self.sky,
            EnvironmentEffect::S3Sky => &self.s3_sky,
            EnvironmentEffect::DecalZBias => &self.decal_zbias,
            EnvironmentEffect::ProxyWorld => &self.proxy_world,
            EnvironmentEffect::S3Default => &self.s3_default,
            EnvironmentEffect::S3DecalSimple | EnvironmentEffect::S3DecalSimpleTileable => {
                &self.s3_decal_simple
            }
            EnvironmentEffect::S3EnvironmentDiffuse => &self.s3_environment_diffuse,
            EnvironmentEffect::S3AlphaTest => &self.s3_alpha_test,
            EnvironmentEffect::S3TransparentDefault => &self.s3_transparent_default,
            EnvironmentEffect::S3Base => &self.s3_base,
            EnvironmentEffect::S3Decal | EnvironmentEffect::S3DecalTileable => &self.s3_base_decal,
            EnvironmentEffect::S3BaseReflective => &self.s3_reflective[0],
            EnvironmentEffect::S3BaseReflectiveSimple => &self.s3_reflective[1],
            EnvironmentEffect::S3TransparentReflective => &self.s3_reflective[2],
            EnvironmentEffect::S3Tree => &self.s3_tree,
            EnvironmentEffect::S3TreeAnimate => &self.s3_tree_animate,
            EnvironmentEffect::S3VertexAnimate => &self.s3_vertex_animate,
        })
    }

    /// One material's samplers, in `EnvironmentEffect::channels` order.
    pub(super) fn material(
        &self,
        device: &wgpu::Device,
        effect: EnvironmentEffect,
        views: &[&wgpu::TextureView],
    ) -> wgpu::BindGroup {
        let mut entries: Vec<wgpu::BindGroupEntry<'_>> = views
            .iter()
            .zip(0_u32..)
            .map(|(view, binding)| wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        let (layout, samplers) = match effect {
            EnvironmentEffect::Water
            | EnvironmentEffect::FlowingWater
            | EnvironmentEffect::Ocean => (
                &self.water_layouts[match effect {
                    EnvironmentEffect::Water => 0,
                    EnvironmentEffect::FlowingWater => 1,
                    _ => 2,
                }],
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.cube_sampler],
            ),
            EnvironmentEffect::BuildingCeilingWorld => (
                &self.building_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.cube_sampler],
            ),
            EnvironmentEffect::BaseTerrain | EnvironmentEffect::DecalTerrain => (
                &self.terrain_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler],
            ),
            EnvironmentEffect::BaseIncandescent
            | EnvironmentEffect::ScrollIncandescent
            | EnvironmentEffect::TransparentIncandescent
            | EnvironmentEffect::ZBiasIncandescent
            | EnvironmentEffect::ZBiasIncandescentBacklit => (
                &self.simple_layout,
                vec![&self.pass_reset_sampler, &self.clamp_sampler],
            ),
            EnvironmentEffect::Default
            | EnvironmentEffect::AlphaTest
            | EnvironmentEffect::TransparentDefault
            | EnvironmentEffect::Advertisement
            | EnvironmentEffect::Tree
            | EnvironmentEffect::TreeAnimate
            | EnvironmentEffect::VertexAnimate => (
                &self.simple_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler],
            ),
            EnvironmentEffect::Base => (
                &self.base_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.cube_sampler],
            ),
            EnvironmentEffect::Decal => (
                &self.decal_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.cube_sampler],
            ),
            EnvironmentEffect::Transparent => (
                &self.transparent_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.cube_sampler],
            ),
            EnvironmentEffect::Sky | EnvironmentEffect::S3Sky => (
                &self.sky_layout,
                vec![&self.sky_sampler, &self.clamp_sampler],
            ),
            EnvironmentEffect::DecalZBias => (
                &self.decal_zbias_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler],
            ),
            EnvironmentEffect::TrafficLightOne
            | EnvironmentEffect::TrafficLightTwo
            | EnvironmentEffect::GodraySpotlight => {
                (&self.proxy_world_layout, vec![&self.wrap_sampler])
            }
            EnvironmentEffect::Glare => (&self.proxy_world_layout, vec![&self.pass_reset_sampler]),
            EnvironmentEffect::ProxyWorld => {
                (&self.proxy_world_layout, vec![&self.pass_reset_sampler])
            }
            EnvironmentEffect::S3Default => {
                (&self.s3_lit_layout, vec![&self.wrap_sampler, &self.clamp_sampler])
            }
            EnvironmentEffect::S3DecalSimple => (
                &self.s3_decal_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.clamp_sampler],
            ),
            EnvironmentEffect::S3DecalSimpleTileable => (
                &self.s3_decal_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.wrap_sampler],
            ),
            EnvironmentEffect::S3EnvironmentDiffuse
            | EnvironmentEffect::S3AlphaTest
            | EnvironmentEffect::S3TransparentDefault => {
                (&self.s3_unlit_layout, vec![&self.wrap_sampler, &self.clamp_sampler])
            }
            EnvironmentEffect::S3Base => {
                (&self.s3_base_layout, vec![&self.wrap_sampler, &self.clamp_sampler])
            }
            EnvironmentEffect::S3Decal => (
                &self.s3_base_decal_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.clamp_sampler],
            ),
            EnvironmentEffect::S3DecalTileable => (
                &self.s3_base_decal_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.wrap_sampler],
            ),
            // Samplers 0x8307E020/E030 (WRAP), E01C (the CLAMP lightmap) and E038 (the cube:
            // CLAMP, LINEAR, POINT mip).
            EnvironmentEffect::S3BaseReflective
            | EnvironmentEffect::S3BaseReflectiveSimple
            | EnvironmentEffect::S3TransparentReflective => (
                &self.s3_reflective_layouts[match effect {
                    EnvironmentEffect::S3BaseReflective => 0,
                    EnvironmentEffect::S3BaseReflectiveSimple => 1,
                    _ => 2,
                }],
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.cube_sampler],
            ),
            // Samplers 0x8307E020 (diffuse WRAP) and E01C (lightmap CLAMP), 0x82cf0e18; treeanimate
            // adds E070 for `i_noise`: WRAP, LINEAR min/mag, POINT mip, anisotropy 1 (0x82cee170).
            EnvironmentEffect::S3Tree | EnvironmentEffect::S3VertexAnimate => {
                (&self.s3_unlit_layout, vec![&self.wrap_sampler, &self.clamp_sampler])
            }
            EnvironmentEffect::S3TreeAnimate => (
                &self.s3_tree_animate_layout,
                vec![&self.wrap_sampler, &self.clamp_sampler, &self.pass_reset_sampler],
            ),
        };
        let first_sampler = u32::try_from(views.len()).unwrap_or(u32::MAX);
        entries.extend(
            samplers
                .into_iter()
                .zip(first_sampler..)
                .map(|(sampler, binding)| wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::Sampler(sampler),
                }),
        );
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("original environment material"),
            layout,
            entries: &entries,
        })
    }

    /// Fresnel table (256x1 RGBA8) and the renderer's CSM atlas.
    pub(super) fn resources(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        fresnel_texels: &[u8],
        shadow_atlas: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let texture = |label, width: u32, texels: &[u8]| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                texels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        };
        let fresnel = texture("g_fresnelLookupTexture", 256, fresnel_texels);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment global resources"),
            layout: &self.resources_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fresnel),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.fresnel_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(shadow_atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                },
            ],
        })
    }
}

/// D3DBLEND_SRCALPHA / D3DBLEND_INVSRCALPHA with D3DBLENDOP_ADD.
pub(super) const SOURCE_ALPHA_BLEND: wgpu::BlendComponent = wgpu::BlendComponent {
    src_factor: wgpu::BlendFactor::SrcAlpha,
    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
    operation: wgpu::BlendOperation::Add,
};

#[derive(Clone, Copy)]
struct EffectTarget<'a> {
    format: wgpu::TextureFormat,
    sample_count: u32,
    write_mask: wgpu::ColorWrites,
    /// None: the pass leaves ALPHABLENDENABLE at its false default.
    blend: Option<wgpu::BlendState>,
    raster: RasterState,
    mesh_attributes: &'a [wgpu::VertexAttribute],
    instance_stride: wgpu::BufferAddress,
    /// Vertex slot 2 (TEXCOORD1/TANGENT); the sky reads no lighting stream.
    lighting: bool,
    depth_write: bool,
}

/// `entry_points`: the vertex and fragment programs.
fn effect_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    [vertex, fragment]: [&str; 2],
    target: EffectTarget<'_>,
) -> wgpu::RenderPipeline {
    let buffers = [
        Some(wgpu::VertexBufferLayout {
            array_stride: 20,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: target.mesh_attributes,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: target.instance_stride,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &MATERIAL_ATTRIBUTES,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: LIGHTING_STRIDE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &LIGHTING_ATTRIBUTES,
        }),
    ];
    let buffers = if target.lighting {
        &buffers[..]
    } else {
        &buffers[..2]
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        cache: None,
        label: Some(fragment),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            compilation_options: Default::default(),
            module: shader,
            entry_point: Some(vertex),
            buffers,
        },
        fragment: Some(wgpu::FragmentState {
            compilation_options: Default::default(),
            module: shader,
            entry_point: Some(fragment),
            targets: &[Some(wgpu::ColorTargetState {
                format: target.format,
                blend: target.blend,
                write_mask: target.write_mask,
            })],
        }),
        primitive: target.raster.primitive(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(target.depth_write),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: target.raster.depth_bias(),
        }),
        multisample: multisample(target.sample_count),
        multiview_mask: None,
    })
}

/// Globals, global resources and the shared helper functions.
pub(super) const COMMON: &str = r"
const SHADOW_RECEIVER_VIEW:u32=0u;
fn shadow_receiver_view(shadow:f32,n_dot_l:f32)->vec4<f32>{
    // Red identifies precisely the native nDotL<=0 bypass inside shadow.
    if SHADOW_RECEIVER_VIEW==2u && n_dot_l<=0. && shadow<0.99 {
        return vec4<f32>(1.,0.,0.,1.);
    }
    return vec4<f32>(vec3<f32>(shadow),1.);
}
struct Globals {
    // g_matVP rebased at g_vViewPos (`CameraRelativeClip`).
    clip_from_camera: mat4x4<f32>,
    view_position: vec4<f32>,
    light_direction: vec4<f32>,
    fog_colour: vec4<f32>,
    fog_k1: vec4<f32>,
    envattributes2: vec4<f32>,
    csm_row0: array<vec4<f32>, 2>,
    csm_row1: vec4<f32>,
    csm_row2: vec4<f32>,
    sky_params: array<vec4<f32>, 2>,
    view_dot_light: vec4<f32>,
    animation_time: vec4<f32>,
    pca: array<vec4<f32>, 7>,
    traffic_lights: array<vec4<f32>,2>,
    camera_axes: array<vec4<f32>,3>,
}
@group(0) @binding(0) var<uniform> globals: Globals;
@group(2) @binding(0) var fresnel_lookup: texture_2d<f32>;
@group(2) @binding(1) var fresnel_sampler: sampler;
@group(2) @binding(2) var shadow_csm0: texture_2d<f32>;
@group(2) @binding(3) var shadow_sampler: sampler;

// Every shipped world program writes oPos with the same four dp4 of the
// fetched position against g_matVP (c0-c3), so coincident vertices of
// different effects get the same depth; each output position is @invariant.
// The product runs on the offset from the camera, so f32 rounding stays
// relative to the view distance instead of to the city coordinates.
fn world_clip(position: vec3<f32>) -> vec4<f32> {
    return globals.clip_from_camera * vec4<f32>(position - globals.view_position.xyz, 1.0);
}

// postfx_functions.fxh:21 ComputeFogFactor (matV argument unused).
fn compute_fog_factor(position: vec3<f32>) -> vec4<f32> {
    let view_distance = length(position - globals.view_position.xyz);
    var f1 = clamp(view_distance * globals.fog_k1.x + globals.fog_k1.y, 0.0, 1.0);
    f1 = pow(f1, globals.fog_k1.z);
    return globals.fog_colour * f1 + vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

// postfx_functions.fxh:51 tonemap.
fn tonemap(x: vec4<f32>) -> vec4<f32> {
    var ret = clamp(vec4<f32>(1.0) - x, vec4<f32>(0.0), vec4<f32>(1.0));
    ret = ret * ret;
    ret = max(x * 0.25 + 0.75, vec4<f32>(1.0)) - ret;
    return ret * 0.5;
}

// postfx_functions.fxh:77 PS_Fog_Bloom_Tone, then MoveFromLinearSpace (sqrt).
fn fog_bloom_tone(final_colour: vec4<f32>, fog: vec4<f32>, exposure: f32) -> vec4<f32> {
    let tone = tonemap((final_colour * fog.a + fog) * exposure);
    return sqrt(tone);
}

// common_shadows.fxh:14 CSM_Shadow as the shipped programs compiled it
// (defaultenvironment_defaultPS.fpo pc 9-14, and the alphatest, base, decal and
// transparent programs alike): y scales by -0.5 around 0.5, filling the
// 1024x512 atlas height; the .updb source text's (-0.25, 0.75) did not ship.
// The first cascade wins overlap because its assignment runs last; no
// perspective divide.
fn csm_shadow(world: vec4<f32>) -> vec3<f32> {
    let scale_offset_guard = vec4<f32>(0.25, -0.5, 0.75, 0.99);
    var uv = vec3<f32>(-1.0, -1.0, 0.0);
    let first = vec2<f32>(dot(globals.csm_row0[0], world), dot(globals.csm_row1, world));
    uv.z = dot(globals.csm_row2, world);
    let second = first * globals.csm_row0[1].xy + globals.csm_row0[1].zw;
    var dist = vec2<f32>(max(abs(first.x), abs(first.y)), max(abs(second.x), abs(second.y)));
    dist -= vec2<f32>(scale_offset_guard.w);
    if (dist.y < 0.0) {
        uv = vec3<f32>(second * scale_offset_guard.xy + vec2<f32>(scale_offset_guard.z, 0.5), uv.z);
    }
    if (dist.x < 0.0) {
        uv = vec3<f32>(first * scale_offset_guard.xy + vec2<f32>(0.25, 0.5), uv.z);
    }
    return uv;
}

// common_shadows.fxh:41 CSM_ShadowMap_PS: 1 is fully lit.
fn csm_shadow_map(world: vec4<f32>, bias: vec3<f32>) -> f32 {
    let light_space = csm_shadow(world);
    let moments = textureSample(shadow_csm0, shadow_sampler, light_space.xy).xy;
    let in_front = step(light_space.z - bias.x, moments.x);
    let visibility = clamp(in_front + (1.0 - moments.y), 0.0, 1.0);
    // PC weather: overcast light is diffuse, without a hard solar shadow.
    if globals.animation_time.w > 0.0 { return mix(visibility, 1.0, globals.animation_time.w); }
    return visibility;
}

// environment_lighting.fxh:96 GetShadowedLightMap with
// GetLightmapValue_SingleTap (dot with the channel selector, then squared).
fn shadowed_lightmap(
    lightmap_sample: vec4<f32>,
    chromaticity: vec4<f32>,
    lightmap_dot: vec4<f32>,
    multiplier: vec4<f32>,
    shadow_value: vec4<f32>,
    shadow_ramp: f32,
    shadow_inv_lerp: f32
) -> vec4<f32> {
    let gamma = dot(lightmap_sample, lightmap_dot);
    let lm_val = gamma * gamma;
    let interpolator = clamp((lm_val - shadow_value.a) * shadow_ramp, 0.0, 1.0) * (1.0 - shadow_inv_lerp);
    var lm_col = vec4<f32>(lm_val, lm_val, lm_val, 1.0) * chromaticity * multiplier;
    lm_col = vec4<f32>(mix(lm_col.rgb, shadow_value.rgb, vec3<f32>(interpolator)), lm_col.a);
    let lm_shadow_val = min(lm_val, shadow_value.a);
    lm_col.a = mix(lm_shadow_val, lm_val, shadow_inv_lerp);
    return lm_col;
}

// common_lighting.fxh:6 phong_specular.
fn phong_specular(normal: vec3<f32>, view: vec3<f32>, light: vec3<f32>, power: f32) -> f32 {
    let reflected = -reflect(light, normal);
    let specular = pow(clamp(dot(reflected, view), 0.0, 1.0), power);
    if globals.animation_time.w > 0.0 { return specular * (1.0 - globals.animation_time.w); }
    return specular;
}

// environment_lighting.fxh:154 GetTangentLight.
fn tangent_light(n: vec3<f32>, tbn_light_pos: vec3<f32>, actual_light: vec3<f32>) -> f32 {
    let norm_z = normalize(tbn_light_pos).z;
    var light = tbn_light_pos;
    light = vec3<f32>(light.xy * sign(actual_light.xy * light.xy), light.z);
    return dot(n, light) / norm_z;
}

// Final decalzbias / terrain programs calculate Fresnel analytically. Their
// older UPDB text incorrectly calls GetFresnel_TextureLookup: no such fetch
// exists in the final programs. Constants are (power8, minimum1, maximum5).
fn calculated_specular_term(
    light: vec3<f32>, normal: vec3<f32>, view_dir: vec3<f32>, masks: vec3<f32>, luminosity: f32
) -> vec3<f32> {
    let fresnel=environment_fresnel(view_dir,normal);
    let power=10.0+masks.g*290.0;
    let calculated=masks.r*phong_specular(normal,view_dir,light,power)
        *vec3<f32>(18.,15.,10.5)*clamp(luminosity-0.1,0.0,1.0);
    return calculated*fresnel;
}
fn environment_fresnel(view_dir: vec3<f32>,normal: vec3<f32>) -> f32 {
    let facing=1.0-clamp(dot(view_dir,normal),0.0,1.0);
    let squared=facing*facing;
    let fourth=squared*squared;
    return 1.0+4.0*(fourth*fourth);
}
";

const DEFAULT_ENVIRONMENT: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_lightmap: texture_2d<f32>;
@group(1) @binding(2) var i_chromaticity: texture_2d<f32>;
@group(1) @binding(3) var wrap_sampler: sampler;
@group(1) @binding(4) var clamp_sampler: sampler;

struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) world: vec3<f32>,
    @location(2) fog: vec4<f32>,
    @location(3) @interpolate(flat) lightmap_channel: vec4<f32>,
    @location(4) @interpolate(flat) material: vec4<f32>,
    @location(5) geometric_normal: vec3<f32>,
}

// defaultenvironment.fx:10 / alphatestdefaultenvironment.fx:10 defaultVS.
@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(8) macro_overlay_uv_scale: vec4<f32>,
    @location(9) detail_normal_uv_scale: vec4<f32>,
    @location(10) macro_overlay_opacity: vec4<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm: vec4<f32>,
    @location(13) tangent: vec3<f32>
) -> PsIn {
    let p = vec4<f32>(position, 1.0);
    var output: PsIn;
    output.world = p.xyz;
    output.position = world_clip(position);
    output.uv = vec4<f32>(uv, lm.xy);
    output.fog = compute_fog_factor(p.xyz);
    output.lightmap_channel = lightmap_dot;
    output.material = m_params;
    return output;
}

fn default_lit(input: PsIn) -> vec4<f32> {
    var diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    diffuse = vec4<f32>(diffuse.rgb * diffuse.rgb, diffuse.a);
    let shadow_inv_lerp = csm_shadow_map(vec4<f32>(input.world, 1.0), vec3<f32>(-0.005, 10.0, 25.0));
    if SHADOW_RECEIVER_VIEW!=0u {return shadow_receiver_view(shadow_inv_lerp,1.);}
    let lightmap = shadowed_lightmap(
        textureSample(i_lightmap, clamp_sampler, input.uv.zw),
        textureSample(i_chromaticity, clamp_sampler, input.uv.zw),
        input.lightmap_channel,
        vec4<f32>(4.5, 4.5, 4.5, 1.5),
        vec4<f32>(0.07, 0.09, 0.1, 0.04),
        50.0,
        shadow_inv_lerp
    );
    return vec4<f32>(diffuse.rgb * lightmap.rgb, diffuse.a);
}

// defaultenvironment.fx:21: final alpha is overwritten with 0.
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let out_colour = default_lit(input);
    let tone = fog_bloom_tone(out_colour * input.material.y, input.fog, globals.envattributes2.x);
    return vec4<f32>(tone.rgb, 0.0);
}

// alphatestdefaultenvironment.fx:21: alpha keeps diffuse alpha, then the
// pass alpha test (GREATEREQUAL 30) rejects before any depth write.
@fragment fn alphatestPS(input: PsIn) -> @location(0) vec4<f32> {
    let out_colour = default_lit(input);
    if (out_colour.a < 30.0 * 0.0039215689) {
        discard;
    }
    let tone = fog_bloom_tone(out_colour * input.material.y, input.fog, globals.envattributes2.x);
    return vec4<f32>(tone.rgb, out_colour.a);
}
";

const BASE_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_detail: texture_2d<f32>;
@group(1) @binding(4) var i_macrooverlay: texture_2d<f32>;
@group(1) @binding(5) var i_environment: texture_cube<f32>;
@group(1) @binding(6) var i_lightmap: texture_2d<f32>;
@group(1) @binding(7) var i_chromaticity: texture_2d<f32>;
@group(1) @binding(8) var wrap_sampler: sampler;
@group(1) @binding(9) var clamp_sampler: sampler;
@group(1) @binding(10) var cube_sampler: sampler;
";

/// baseenvironment's channels in the same order, then `i_decal`.
const DECAL_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_detail: texture_2d<f32>;
@group(1) @binding(4) var i_macrooverlay: texture_2d<f32>;
@group(1) @binding(5) var i_environment: texture_cube<f32>;
@group(1) @binding(6) var i_lightmap: texture_2d<f32>;
@group(1) @binding(7) var i_chromaticity: texture_2d<f32>;
@group(1) @binding(8) var i_decal: texture_2d<f32>;
@group(1) @binding(9) var wrap_sampler: sampler;
@group(1) @binding(10) var clamp_sampler: sampler;
@group(1) @binding(11) var cube_sampler: sampler;
";

/// Shared by every module that declares `i_environment` and `cube_sampler`.
const SPECULAR: &str = r"
// environment_lighting.fxh:207 GetSpecularTerm with the compiled call
// arguments every environment effect passes: fresnel (8, 1, 5), threshold
// 0.1, tint (18, 15, 10.5), power scale 290, offset 10, reflection scale
// 1.5, ramp (0.3, 4, 1.5). GetFresnel_TextureLookup, GetReflectionTerm
// (xy of the reflection negated), GetSpecPower and GetCalculatedSpecular
// inline in source order, with analytic Fresnel from the FINAL FPO (the
// older UPDB calls a lookup texture that none of these final programs bind).
fn specular_term(
    light: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    masks: vec3<f32>,
    luminosity: f32
) -> vec3<f32> {
    let fresnel = environment_fresnel(view_dir,normal);
    let reflection_luminosity = 0.3 * clamp(4.0 * (masks.b - 1.5), 0.0, 1.0);
    var reflect_vector = reflect(view_dir, normal);
    reflect_vector = vec3<f32>(-reflect_vector.xy, reflect_vector.z);
    let reflection = textureSample(i_environment, cube_sampler, reflect_vector).rgb
        * mix(luminosity, 1.0, reflection_luminosity);
    let reflected = masks.b * reflection * 1.5;
    let power = 10.0 + masks.g * 290.0;
    let calculated = masks.r * phong_specular(normal, view_dir, light, power)
        * vec3<f32>(18.0, 15.0, 10.5) * clamp(luminosity - 0.1, 0.0, 1.0);
    return (calculated + reflected) * fresnel;
}
";

/// baseenvironment's vertex program. decalenvironment reuses it, and
/// decalzbias.fx `defaultVS` is the same body without `dataconstants`, which
/// its pixel program never reads.
const ENVIRONMENT_VS: &str = r"
struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) binormal: vec3<f32>,
    @location(4) world: vec3<f32>,
    @location(5) fog: vec4<f32>,
    @location(6) @interpolate(flat) lightmap_channel: vec4<f32>,
    @location(7) @interpolate(flat) data_constants: vec4<f32>,
    @location(8) @interpolate(flat) material: vec4<f32>,
    // decalenvironment `Decal : TEXCOORD5`; baseenvironment leaves it zero.
    @location(9) decal: vec2<f32>,
    @location(10) @interpolate(flat) material1: vec4<f32>,
}

// baseenvironment.fx:40 defaultVS; i_matWorld is declared but unused.
// decalenvironment.fx defaultVS is the same program plus `Out.Decal`.
fn environment_vs(
    position: vec3<f32>,
    uv: vec2<f32>,
    lightmap_dot: vec4<f32>,
    macro_overlay_uv_scale: vec4<f32>,
    detail_normal_uv_scale: vec4<f32>,
    macro_overlay_opacity: vec4<f32>,
    m_params: vec4<f32>,
    lm_norm: vec4<f32>,
    tangent: vec3<f32>
) -> PsIn {
    let p = vec4<f32>(position, 1.0);
    var output: PsIn;
    var signs = clamp(vec2<f32>(65535.0, 65535.0) * lm_norm.xy, vec2<f32>(0.0), vec2<f32>(1.0));
    signs = signs * 2.0 - 1.0;
    let xy_len = min(0.999, dot(lm_norm.zw, lm_norm.zw));
    output.normal = vec3<f32>(lm_norm.zw, signs.y * sqrt(1.0 - xy_len));
    output.tangent = tangent;
    output.binormal = signs.x * cross(output.normal, output.tangent);
    output.world = position;
    output.position = world_clip(position);
    output.uv = vec4<f32>(uv, abs(lm_norm.xy));
    output.fog = compute_fog_factor(p.xyz);
    output.data_constants = vec4<f32>(macro_overlay_uv_scale.x, detail_normal_uv_scale.x, macro_overlay_opacity.x, 0.0);
    output.lightmap_channel = lightmap_dot;
    output.material = m_params;
    output.decal = vec2<f32>(0.0);
    return output;
}

@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(8) macro_overlay_uv_scale: vec4<f32>,
    @location(9) detail_normal_uv_scale: vec4<f32>,
    @location(10) macro_overlay_opacity: vec4<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm_norm: vec4<f32>,
    @location(13) tangent: vec3<f32>
) -> PsIn {
    return environment_vs(
        position,
        uv,
        lightmap_dot,
        macro_overlay_uv_scale,
        detail_normal_uv_scale,
        macro_overlay_opacity,
        m_params,
        lm_norm,
        tangent
    );
}
";

const BASE_ENVIRONMENT: &str = r"
// environment_textures.fxh:41 ApplyOverlay returns the darken branch.
fn apply_overlay(overlay: vec3<f32>, base: vec3<f32>) -> vec3<f32> {
    return 2.0 * base * overlay;
}

// baseenvironment.fx:61 defaultPS from the sampled surface colour on, which
// decalenvironment.fx defaultPS shares: `surface` is the linear diffuse
// (after the decal for decalenvironment) before `ApplyOverlay`. The decal
// program's `half4` arguments to `ApplyOverlay`/`GetDiffuseTerm` truncate to
// the same `half3` overloads, and its `TransformToWorldFromTangent` body is
// `TransformToWorldFromTangent_normalize`'s.
fn environment_tone(input: PsIn, surface: vec3<f32>) -> vec4<f32> {
    let overlay_sample = textureSample(i_macrooverlay, wrap_sampler, input.uv.xy * input.data_constants.x).rgb;
    let overlay = clamp((overlay_sample - vec3<f32>(0.5)) * input.data_constants.z + vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(1.0));
    let spec_masks = apply_overlay(overlay, textureSample(i_specular, wrap_sampler, input.uv.xy).xyz);
    let view_dir = normalize(globals.view_position.xyz - input.world);

    let detail = 2.0 * textureSample(i_detail, wrap_sampler, input.uv.xy * input.data_constants.y).xyz - 1.0;
    let tangent_normal = 2.0 * textureSample(i_normal, wrap_sampler, input.uv.xy).xyz - 1.0;
    let nd = normalize(tangent_normal + vec3<f32>(detail.x, detail.y, 0.0));
    // half3x3(tangent, binormal, normal) has those as rows.
    let world_normal = normalize(nd.x * input.tangent + nd.y * input.binormal + nd.z * input.normal);

    let n_dot_l = dot(world_normal, globals.light_direction.xyz);
    let csm = csm_shadow_map(vec4<f32>(input.world, 1.0), vec3<f32>(-0.005, 10.0, 25.0));
    let shadow_inv_lerp = select(1.0, csm, n_dot_l > 0.0);
    if SHADOW_RECEIVER_VIEW!=0u {return shadow_receiver_view(csm,n_dot_l);}
    let lightmap = shadowed_lightmap(
        textureSample(i_lightmap, clamp_sampler, input.uv.zw),
        textureSample(i_chromaticity, clamp_sampler, input.uv.zw),
        input.lightmap_channel,
        vec4<f32>(4.5, 4.5, 4.5, 1.5),
        vec4<f32>(0.07, 0.09, 0.1, 0.04),
        50.0,
        shadow_inv_lerp
    );

    let light = globals.light_direction.xyz;
    let tangent_space_light = normalize(vec3<f32>(
        dot(input.tangent, light),
        dot(input.binormal, light),
        dot(input.normal, light)
    ));
    let kd = tangent_light(nd, vec3<f32>(0.64, 0.64, 0.420465), tangent_space_light);
    let diffuse_colour = apply_overlay(overlay, surface);
    let diff_term = lightmap.rgb * kd;
    let spec_term = specular_term(light, world_normal, view_dir, spec_masks, lightmap.a);

    let out_colour = vec4<f32>(diff_term * diffuse_colour + spec_term, 1.0);
    return fog_bloom_tone(out_colour * input.material.y, input.fog, globals.envattributes2.x);
}

// baseenvironment.fx:61 defaultPS; GetDiffuseTextureValue squares rgb only.
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    let tone = environment_tone(input, diffuse.rgb * diffuse.rgb);
    return vec4<f32>(tone.rgb, 1.0);
}
";

const DECAL_ENVIRONMENT: &str = r"
// decalenvironment.fx defaultVS: `float4 uv_decal : TEXCOORD0`, xy the
// surface UV and zw copied to `Out.Decal`.
@vertex fn decalVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(8) macro_overlay_uv_scale: vec4<f32>,
    @location(9) detail_normal_uv_scale: vec4<f32>,
    @location(10) macro_overlay_opacity: vec4<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm_norm: vec4<f32>,
    @location(13) tangent: vec3<f32>,
    @location(14) uv_decal_zw: vec2<f32>
) -> PsIn {
    var output = environment_vs(
        position,
        uv,
        lightmap_dot,
        macro_overlay_uv_scale,
        detail_normal_uv_scale,
        macro_overlay_opacity,
        m_params,
        lm_norm,
        tangent
    );
    output.decal = uv_decal_zw;
    return output;
}

// decalenvironment.fx defaultPS: environment_textures.fxh:32 GetDecalTexture
// (rgb squared, alpha kept), :37 ApplyDecal lerp(base, decal.rgb, decal.a)
// before ApplyOverlay; the final alpha is overwritten with 0.
@fragment fn decalPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    let decal = textureSample(i_decal, clamp_sampler, input.decal);
    let surface = mix(diffuse.rgb * diffuse.rgb, decal.rgb * decal.rgb, vec3<f32>(decal.a));
    let tone = environment_tone(input, surface);
    return vec4<f32>(tone.rgb, 0.0);
}
";

const TRANSPARENT_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_environment: texture_cube<f32>;
@group(1) @binding(3) var i_lightmap: texture_2d<f32>;
@group(1) @binding(4) var i_chromaticity: texture_2d<f32>;
@group(1) @binding(5) var wrap_sampler: sampler;
@group(1) @binding(6) var clamp_sampler: sampler;
@group(1) @binding(7) var cube_sampler: sampler;
";

const TRANSPARENT_ENVIRONMENT: &str = r"
struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world: vec3<f32>,
    @location(3) fog: vec4<f32>,
    @location(4) @interpolate(flat) lightmap_channel: vec4<f32>,
    @location(5) @interpolate(flat) material: vec4<f32>,
}

// transparentenvironment.fx defaultVS: baseenvironment's `lm_norm` normal
// decode without TANGENT; i_matWorld is declared but unused.
@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm_norm: vec4<f32>
) -> PsIn {
    let p = vec4<f32>(position, 1.0);
    var output: PsIn;
    var signs = clamp(vec2<f32>(65535.0, 65535.0) * lm_norm.xy, vec2<f32>(0.0), vec2<f32>(1.0));
    signs = signs * 2.0 - 1.0;
    let xy_len = min(0.999, dot(lm_norm.zw, lm_norm.zw));
    output.normal = vec3<f32>(lm_norm.zw, signs.y * sqrt(1.0 - xy_len));
    output.world = position;
    output.position = world_clip(position);
    output.uv = vec4<f32>(uv, abs(lm_norm.xy));
    output.fog = compute_fog_factor(p.xyz);
    output.lightmap_channel = lightmap_dot;
    output.material = m_params;
    return output;
}

// transparentenvironment.fx defaultPS. The interpolated normal is used as
// is (never renormalized); GetDiffuseTerm(cLightMap, 1) is the lightmap
// colour; outcolor = (half4(diffTerm, 1) * cDiffuse) * cDiffuse.a +
// half4(specTerm, 0), so its alpha is diffuse.a squared, and the returned
// alpha is that outcolor.a. The pass alpha test (GREATEREQUAL 16) runs on it
// after every texture fetch.
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    var diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    diffuse = vec4<f32>(diffuse.rgb * diffuse.rgb, diffuse.a);
    let masks = textureSample(i_specular, wrap_sampler, input.uv.xy).xyz;
    let view_dir = normalize(globals.view_position.xyz - input.world);
    let normal = input.normal;
    let n_dot_l = dot(normal, globals.light_direction.xyz);
    let csm = csm_shadow_map(vec4<f32>(input.world, 1.0), vec3<f32>(-0.005, 10.0, 25.0));
    let shadow_inv_lerp = select(1.0, csm, n_dot_l > 0.0);
    let lightmap = shadowed_lightmap(
        textureSample(i_lightmap, clamp_sampler, input.uv.zw),
        textureSample(i_chromaticity, clamp_sampler, input.uv.zw),
        input.lightmap_channel,
        vec4<f32>(4.5, 4.5, 4.5, 1.5),
        vec4<f32>(0.07, 0.09, 0.1, 0.04),
        50.0,
        shadow_inv_lerp
    );
    let spec_term = specular_term(globals.light_direction.xyz, normal, view_dir, masks, lightmap.a);
    let out_colour = (vec4<f32>(lightmap.rgb, 1.0) * diffuse) * diffuse.a + vec4<f32>(spec_term, 0.0);
    if (out_colour.a < 16.0 * 0.0039215689) {
        discard;
    }
    let tone = fog_bloom_tone(out_colour * input.material.y, input.fog, globals.envattributes2.x);
    return vec4<f32>(tone.rgb, out_colour.a);
}
";

/// `sky.fx` defaultVS, which Skate 2's and Skate 3's skies share (`sky_defaultVS.vpo` differs only
/// in naming c4 `i_worldPos` for `g_vViewPos`), with the bindings both pixel programs read.
const SKY_VS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_detail: texture_2d<f32>;
@group(1) @binding(2) var i_specular: texture_2d<f32>;
@group(1) @binding(3) var sky_sampler: sampler;
@group(1) @binding(4) var clamp_sampler: sampler;

struct PsIn {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) v_pos: vec3<f32>,
    @location(2) @interpolate(flat) params0: vec4<f32>,
    @location(3) @interpolate(flat) params1: vec4<f32>,
}

// sky.fx defaultVS: model-space positions offset by g_vViewPos; i_matWorld
// is unused. The offset position relative to the camera is the model-space
// position itself. `m_params` is the material scope's; a scope that resolves
// to material_sky/default reads the WorldPainter sky blend output.
@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) blended: vec4<f32>,
    @location(8) params1: vec4<f32>,
    @location(11) params0: vec4<f32>
) -> PsIn {
    var output: PsIn;
    output.position = globals.clip_from_camera * vec4<f32>(position, 1.0);
    // PC weather: the sky mesh sits at a finite depth; under weather it moves to the far plane
    // so the weather pass reads it as sky, not scenery.
    if globals.animation_time.w > 0.0 { output.position.z = output.position.w; }
    output.uv = uv;
    output.v_pos = position;
    let use_blend = blended.x != 0.0;
    output.params0 = select(params0, globals.sky_params[0], use_blend);
    output.params1 = select(params1, globals.sky_params[1], use_blend);
    return output;
}
";

const SKY_PS: &str = r"
// sky.fx defaultPS, then PS_Bloom_Tone (tonemap, MoveFromLinearSpace).
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let sky_detail = textureSample(i_detail, sky_sampler, input.uv).rgb;
    var sky_gradient = textureSample(i_diffuse, sky_sampler, input.uv).rgb;
    let grey = dot(vec3<f32>(0.3, 0.59, 0.11), sky_gradient);
    sky_gradient = mix(vec3<f32>(grey), sky_gradient, vec3<f32>(input.params1.w)) * input.params1.xyz;
    var skycolour = sky_gradient + sky_detail - vec3<f32>(0.5);
    skycolour = skycolour * skycolour;
    let dot_pl = clamp(dot(normalize(input.v_pos), globals.light_direction.xyz), 0.0, 1.0);
    let sin_a = sqrt(1.0 - dot_pl * dot_pl);
    let s = sin_a / input.params0.x;
    // tex1D: the one-row sun colour table along u.
    var sun = textureSample(i_specular, clamp_sampler, vec2<f32>(s, 0.0));
    sun = vec4<f32>(sun.rgb * sun.rgb, sun.a);
    // PC weather: clouds take the solar disc before bloom spreads its light.
    if globals.animation_time.w > 0.0 { sun = vec4<f32>(sun.rgb * (1.0 - globals.animation_time.w), sun.a); }
    let out_colour = vec4<f32>(skycolour + sun.rgb / clamp(sun.a + 0.01, 0.0, 1.0), 1.0);
    let tone = sqrt(tonemap(out_colour * input.params0.y * globals.envattributes2.x));
    return vec4<f32>(tone.rgb, 1.0);
}
";

/// Skate 3 `sky_defaultPS.fpo` (shaders_final.big), translated from the shipped microcode: the
/// diffuse alone, squared (pc 3, 8), with no detail fetch and no tint or saturation (it binds
/// `m_params` c4[1], reading only .x and .y); the sun table (pc 9-15) and the tone (pc 16-26)
/// are Skate 2's.
const S3_SKY_PS: &str = r"
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, sky_sampler, input.uv).rgb;
    let skycolour = diffuse * diffuse;
    let dot_pl = clamp(dot(normalize(input.v_pos), globals.light_direction.xyz), 0.0, 1.0);
    let sin_a = sqrt(1.0 - dot_pl * dot_pl);
    let s = sin_a / input.params0.x;
    var sun = textureSample(i_specular, clamp_sampler, vec2<f32>(s, 0.0));
    sun = vec4<f32>(sun.rgb * sun.rgb, sun.a);
    // PC weather: clouds take the solar disc before bloom spreads its light.
    if globals.animation_time.w > 0.0 { sun = vec4<f32>(sun.rgb * (1.0 - globals.animation_time.w), sun.a); }
    // PC port: under Skate 2's PostFX the disc keeps Skate 3's on-screen brightness.
    if globals.animation_time.z > 0.0 { sun = vec4<f32>(sun.rgb * globals.animation_time.z, sun.a); }
    let out_colour = vec4<f32>(skycolour + sun.rgb / clamp(sun.a + 0.01, 0.0, 1.0), 1.0);
    let tone = sqrt(tonemap(out_colour * input.params0.y * globals.envattributes2.x));
    return vec4<f32>(tone.rgb, 1.0);
}
";

/// decalzbias's five samplers: the diffuse family WRAP, the lightmap and
/// chromaticity CLAMP (decalzbias.xml `default` pass).
const DECAL_ZBIAS_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_lightmap: texture_2d<f32>;
@group(1) @binding(4) var i_chromaticity: texture_2d<f32>;
@group(1) @binding(5) var wrap_sampler: sampler;
@group(1) @binding(6) var clamp_sampler: sampler;
";

const DECAL_ZBIAS: &str = r"
// decalzbias.fx defaultPS. GetNormalMapValue is 2*tex-1 without
// normalization and feeds GetTangentLight as is; the world normal is
// normalize(mul(n, tbn)). outcolor = half4(diffTerm, 1)*cDiffuse +
// half4(specTerm, 0) and returncolour.a is cDiffuse.a, which the pass alpha
// test (GREATEREQUAL 16) rejects on after every texture fetch.
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    var diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    diffuse = vec4<f32>(diffuse.rgb * diffuse.rgb, diffuse.a);
    let masks = textureSample(i_specular, wrap_sampler, input.uv.xy).xyz;
    let normal = 2.0 * textureSample(i_normal, wrap_sampler, input.uv.xy).xyz - 1.0;
    // half3x3(tangent, binormal, normal) has those as rows.
    let world_normal = normalize(normal.x * input.tangent + normal.y * input.binormal + normal.z * input.normal);
    let view_dir = normalize(globals.view_position.xyz - input.world);
    let n_dot_l = dot(world_normal, globals.light_direction.xyz);
    let csm = csm_shadow_map(vec4<f32>(input.world, 1.0), vec3<f32>(-0.005, 10.0, 25.0));
    let shadow_inv_lerp = select(1.0, csm, n_dot_l > 0.0);
    let lightmap = shadowed_lightmap(
        textureSample(i_lightmap, clamp_sampler, input.uv.zw),
        textureSample(i_chromaticity, clamp_sampler, input.uv.zw),
        input.lightmap_channel,
        vec4<f32>(4.5, 4.5, 4.5, 1.5),
        vec4<f32>(0.07, 0.09, 0.1, 0.04),
        50.0,
        shadow_inv_lerp
    );
    let light = globals.light_direction.xyz;
    let tangent_space_light = normalize(vec3<f32>(
        dot(input.tangent, light),
        dot(input.binormal, light),
        dot(input.normal, light)
    ));
    let kd = tangent_light(normal, vec3<f32>(0.64, 0.64, 0.420465), tangent_space_light);
    let diff_term = lightmap.rgb * kd;
    let spec_term = calculated_specular_term(light, world_normal, view_dir, masks, lightmap.a);
    let out_colour = vec4<f32>(diff_term, 1.0) * diffuse + vec4<f32>(spec_term, 0.0);
    if (diffuse.a < 16.0 * 0.0039215689) {
        discard;
    }
    let tone = fog_bloom_tone(out_colour * input.material.y, input.fog, globals.envattributes2.x);
    return vec4<f32>(tone.rgb, diffuse.a);
}
";

/// Skate 3's simple world programs, translated from the shipped microcode
/// (`data\render\shaders\final\*_default{VS,PS}.?po`; Skate 3 ships no `.updb` source). Material
/// channels bind in `EnvironmentEffect::channels` order, then the samplers.
const S3_LIT_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_lightmap: texture_2d<f32>;
@group(1) @binding(4) var wrap_sampler: sampler;
@group(1) @binding(5) var clamp_sampler: sampler;
";

const S3_DECAL_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_lightmap: texture_2d<f32>;
@group(1) @binding(4) var i_decal: texture_2d<f32>;
@group(1) @binding(5) var wrap_sampler: sampler;
@group(1) @binding(6) var clamp_sampler: sampler;
@group(1) @binding(7) var decal_sampler: sampler;
";

const S3_UNLIT_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_lightmap: texture_2d<f32>;
@group(1) @binding(2) var wrap_sampler: sampler;
@group(1) @binding(3) var clamp_sampler: sampler;
";

/// Every Skate 3 world program: the lightmap's four taps at (+-0.5, +-0.5) texels averaged and
/// squared, min'd with the lit fraction plus (0.05, 0.09, 0.13); the lit programs' colour before
/// fog (`S3Lighting`) and their fog, tone and square root with m_params.y.
const S3_LIGHTMAP: &str = r"
fn s3_lightmap(uv: vec2<f32>) -> vec3<f32> {
    let half_texel = 0.5 / vec2<f32>(textureDimensions(i_lightmap));
    let taps = textureSample(i_lightmap, clamp_sampler, uv + half_texel).rgb
        + textureSample(i_lightmap, clamp_sampler, uv + vec2<f32>(-half_texel.x, half_texel.y)).rgb
        + textureSample(i_lightmap, clamp_sampler, uv - half_texel).rgb
        + textureSample(i_lightmap, clamp_sampler, uv + vec2<f32>(half_texel.x, -half_texel.y)).rgb;
    let average = taps * 0.25;
    return average * average;
}

fn s3_shadowed_lightmap(world: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let shadow = csm_shadow_map(vec4<f32>(world, 1.0), vec3<f32>(-0.005, 10.0, 25.0));
    return min(vec3<f32>(shadow) + vec3<f32>(0.05, 0.09, 0.13), s3_lightmap(uv));
}

struct S3Lighting {
    colour: vec3<f32>,
    lit: vec3<f32>,
    world_normal: vec3<f32>,
    view_dir: vec3<f32>,
    specular: vec3<f32>,
}

fn s3_tone(input: PsIn, colour: vec3<f32>) -> vec4<f32> {
    return fog_bloom_tone(vec4<f32>(colour, 1.0) * input.material.y, input.fog, globals.envattributes2.x);
}
";

/// `defaultenvironment_defaultPS` after its albedo.
const S3_LIT: &str = r"
// The tangent-space normal against the light's side of the tangent and binormal
// (0.58, 0.62, 0.39), times 2.3956 and the shadowed lightmap; a highlight off the fixed
// direction (-0.14, 0.5, 0.9), tinted (2.1, 1.8, 1.5) and scaled by the green shadowed lightmap
// and the specular map's x, its power 10 + 290 y.
fn s3_lighting(input: PsIn, albedo: vec3<f32>) -> S3Lighting {
    let specular = textureSample(i_specular, wrap_sampler, input.uv.xy).xyz;
    let n = 2.0 * textureSample(i_normal, wrap_sampler, input.uv.xy).xyz - 1.0;
    let lit = s3_shadowed_lightmap(input.world, input.uv.zw);
    let light = globals.light_direction.xyz;
    let facing = 0.58 * sign(dot(input.tangent, light)) * n.x
        + 0.62 * sign(dot(input.binormal, light)) * n.y
        + 0.39 * n.z;
    let world_normal = normalize(n.x * input.tangent + n.y * input.binormal + n.z * input.normal);
    let view_dir = normalize(globals.view_position.xyz - input.world);
    let reflected = reflect(vec3<f32>(-0.14, 0.5, 0.9), world_normal);
    let highlight = pow(clamp(dot(view_dir, -reflected), 0.0, 1.0), 10.0 + specular.y * 290.0);
    let colour = 2.3956184 * facing * lit * albedo
        + highlight * vec3<f32>(2.1, 1.8, 1.5) * lit.g * specular.x;
    return S3Lighting(colour, lit, world_normal, view_dir, specular);
}

fn s3_lit(input: PsIn, albedo: vec3<f32>) -> vec4<f32> {
    return s3_tone(input, s3_lighting(input, albedo).colour);
}
";

/// `defaultenvironment_defaultPS`: diffuse squared; the alpha lane is never written.
const S3_DEFAULT_PS: &str = r"
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy).rgb;
    return vec4<f32>(s3_lit(input, diffuse * diffuse).rgb, 0.0);
}
";

/// The decal effects' `defaultVS`: baseenvironment's frame plus `uv_decal.zw`.
const S3_DECAL_VS: &str = r"
@vertex fn decalVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(8) macro_overlay_uv_scale: vec4<f32>,
    @location(9) detail_normal_uv_scale: vec4<f32>,
    @location(10) macro_overlay_opacity: vec4<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm_norm: vec4<f32>,
    @location(13) tangent: vec3<f32>,
    @location(14) uv_decal_zw: vec2<f32>
) -> PsIn {
    var output = environment_vs(
        position,
        uv,
        lightmap_dot,
        macro_overlay_uv_scale,
        detail_normal_uv_scale,
        macro_overlay_opacity,
        m_params,
        lm_norm,
        tangent
    );
    output.decal = uv_decal_zw;
    return output;
}
";

/// `decalenvironment_simple(_tileable)_defaultPS`: lerp(diffuse^2, decal^2, decal.a) into
/// `S3_LIT`. The two programs are byte-identical; the tileable effect wraps its decal.
const S3_DECAL: &str = r"
@fragment fn decalPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy).rgb;
    let decal = textureSample(i_decal, decal_sampler, input.decal);
    let albedo = mix(diffuse * diffuse, decal.rgb * decal.rgb, vec3<f32>(decal.a));
    return vec4<f32>(s3_lit(input, albedo).rgb, 0.0);
}
";

const S3_BASE_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_detail: texture_2d<f32>;
@group(1) @binding(4) var i_macrooverlay: texture_2d<f32>;
@group(1) @binding(5) var i_lightmap: texture_2d<f32>;
@group(1) @binding(6) var wrap_sampler: sampler;
@group(1) @binding(7) var clamp_sampler: sampler;
";

const S3_BASE_DECAL_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_detail: texture_2d<f32>;
@group(1) @binding(4) var i_macrooverlay: texture_2d<f32>;
@group(1) @binding(5) var i_lightmap: texture_2d<f32>;
@group(1) @binding(6) var i_decal: texture_2d<f32>;
@group(1) @binding(7) var wrap_sampler: sampler;
@group(1) @binding(8) var clamp_sampler: sampler;
@group(1) @binding(9) var decal_sampler: sampler;
";

/// `baseenvironment_defaultPS` after its albedo (`S3_LIT` with these differences): the
/// tangent-space normal plus (2 detail.xy - 1, 0) at uv * detailNormalUVScale, normalized before
/// the facing term; the albedo times the macro overlay sat((macro - 0.5) opacity + 0.5) at
/// uv * macroOverlayUVScale, which `decalenvironment` lerps from 1 by `overlay_weight`.
const S3_BASE: &str = r"
fn s3_base_lighting(input: PsIn, albedo: vec3<f32>, overlay_weight: f32) -> S3Lighting {
    let uv = input.uv.xy;
    let macro_sample = textureSample(i_macrooverlay, wrap_sampler, uv * input.data_constants.x).rgb;
    let detail = textureSample(i_detail, wrap_sampler, uv * input.data_constants.y).xy;
    let specular = textureSample(i_specular, wrap_sampler, uv).xyz;
    let normal_sample = textureSample(i_normal, wrap_sampler, uv).xyz;
    let overlay = clamp(
        (macro_sample - vec3<f32>(0.5)) * input.data_constants.z + vec3<f32>(0.5),
        vec3<f32>(0.0),
        vec3<f32>(1.0)
    );
    let tint = mix(vec3<f32>(1.0), overlay, overlay_weight);
    let n = normalize(2.0 * normal_sample - 1.0 + vec3<f32>(2.0 * detail - 1.0, 0.0));
    let lit = s3_shadowed_lightmap(input.world, input.uv.zw);
    let light = globals.light_direction.xyz;
    let facing = 0.58 * sign(dot(input.tangent, light)) * n.x
        + 0.62 * sign(dot(input.binormal, light)) * n.y
        + 0.39 * n.z;
    let world_normal = normalize(n.x * input.tangent + n.y * input.binormal + n.z * input.normal);
    let view_dir = normalize(globals.view_position.xyz - input.world);
    let reflected = reflect(vec3<f32>(-0.14, 0.5, 0.9), world_normal);
    let highlight = pow(clamp(dot(view_dir, -reflected), 0.0, 1.0), 10.0 + specular.y * 290.0);
    let colour = 2.3956184 * facing * lit * tint * albedo
        + highlight * vec3<f32>(2.1, 1.8, 1.5) * lit.g * specular.x;
    return S3Lighting(colour, lit, world_normal, view_dir, specular);
}

fn s3_base(input: PsIn, albedo: vec3<f32>, overlay_weight: f32) -> vec4<f32> {
    return s3_tone(input, s3_base_lighting(input, albedo, overlay_weight).colour);
}
";

const S3_BASE_PS: &str = r"
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy).rgb;
    return vec4<f32>(s3_base(input, diffuse * diffuse, 1.0).rgb, 1.0);
}
";

/// `decalenvironment(_tileable)_defaultPS`: lerp(diffuse^2, decal^2, decal.a), the overlay
/// weighted by max(1 - decal.a, embeddedDecal) (the vertex program's `oC4.w`; the instance's
/// selector lane carries `embeddedDecal`, which no Skate 3 program reads as a selector).
const S3_BASE_DECAL_PS: &str = r"
@fragment fn decalPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy).rgb;
    let decal = textureSample(i_decal, decal_sampler, input.decal);
    let albedo = mix(diffuse * diffuse, decal.rgb * decal.rgb, vec3<f32>(decal.a));
    let overlay_weight = max(1.0 - decal.a, input.lightmap_channel.x);
    return vec4<f32>(s3_base(input, albedo, overlay_weight).rgb, 0.0);
}
";

const S3_BASE_REFLECTIVE_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_detail: texture_2d<f32>;
@group(1) @binding(4) var i_macrooverlay: texture_2d<f32>;
@group(1) @binding(5) var i_lightmap: texture_2d<f32>;
@group(1) @binding(6) var i_environment: texture_cube<f32>;
@group(1) @binding(7) var wrap_sampler: sampler;
@group(1) @binding(8) var clamp_sampler: sampler;
@group(1) @binding(9) var cube_sampler: sampler;
";

const S3_REFLECTIVE_SIMPLE_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_normal: texture_2d<f32>;
@group(1) @binding(3) var i_lightmap: texture_2d<f32>;
@group(1) @binding(4) var i_environment: texture_cube<f32>;
@group(1) @binding(5) var wrap_sampler: sampler;
@group(1) @binding(6) var clamp_sampler: sampler;
@group(1) @binding(7) var cube_sampler: sampler;
";

const S3_TRANSPARENT_REFLECTIVE_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_specular: texture_2d<f32>;
@group(1) @binding(2) var i_lightmap: texture_2d<f32>;
@group(1) @binding(3) var i_environment: texture_cube<f32>;
@group(1) @binding(4) var wrap_sampler: sampler;
@group(1) @binding(5) var clamp_sampler: sampler;
@group(1) @binding(6) var cube_sampler: sampler;
";

/// The reflection the three reflective programs add (`baseenvironmentreflective_defaultPS` pc
/// 57-65 and 77-90): `i_environment` at the view vector reflected about the normal, xy negated,
/// times the green shadowed lightmap raised toward 1 by 0.3 sat(4 mask - 2.6), the specular map's
/// z mask and 1.5.
const S3_REFLECTION: &str = r"
fn s3_reflection(normal: vec3<f32>, view_dir: vec3<f32>, lit_green: f32, mask: f32) -> vec3<f32> {
    let reflected = reflect(view_dir, normal);
    let environment = textureSample(i_environment, cube_sampler, vec3<f32>(-reflected.xy, reflected.z)).rgb;
    let luminosity = 0.3 * clamp(4.0 * mask - 2.6, 0.0, 1.0) * (1.0 - lit_green) + lit_green;
    return environment * luminosity * mask * 1.5;
}
";

/// `baseenvironmentreflective_defaultPS`: `baseenvironment`'s program (the full overlay) plus
/// `S3_REFLECTION` on the detailed normal; pc 99 writes alpha with both export masks, the
/// constant 1.
const S3_BASE_REFLECTIVE_PS: &str = r"
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy).rgb;
    let lighting = s3_base_lighting(input, diffuse * diffuse, 1.0);
    let reflection = s3_reflection(lighting.world_normal, lighting.view_dir, lighting.lit.g, lighting.specular.z);
    return vec4<f32>(s3_tone(input, lighting.colour + reflection).rgb, 1.0);
}
";

/// `baseenvironmentreflective_simple_defaultPS`: `defaultenvironment`'s program plus
/// `S3_REFLECTION` (pc 44-65, 66-81); alpha the constant 1 as above (pc 88).
const S3_REFLECTIVE_SIMPLE_PS: &str = r"
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy).rgb;
    let lighting = s3_lighting(input, diffuse * diffuse);
    let reflection = s3_reflection(lighting.world_normal, lighting.view_dir, lighting.lit.g, lighting.specular.z);
    return vec4<f32>(s3_tone(input, lighting.colour + reflection).rgb, 1.0);
}
";

/// `transparentenvironmentreflective`: no normal, detail or overlay map. The vertex program
/// unpacks the geometric normal from TEXCOORD1.zw, z = (2 sat(65535 lm.y) - 1)
/// sqrt(1 - min(|zw|^2, 0.999)). The pixel program lights diffuse^2 by the shadowed lightmap and
/// the diffuse alpha, adds the fixed-direction highlight and `S3_REFLECTION` off that
/// interpolated, unnormalized normal (pc 21-29, 45-58), and writes alpha diffuse.a^2 (pc 68).
const S3_TRANSPARENT_REFLECTIVE: &str = r"
struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world: vec3<f32>,
    @location(3) fog: vec4<f32>,
    @location(4) @interpolate(flat) material: vec4<f32>,
}

@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm_norm: vec4<f32>
) -> PsIn {
    var output: PsIn;
    output.position = world_clip(position);
    output.uv = vec4<f32>(uv, abs(lm_norm.xy));
    let sign_z = clamp(lm_norm.y * 65535.0, 0.0, 1.0) * 2.0 - 1.0;
    let xy_len = min(dot(lm_norm.zw, lm_norm.zw), 0.999);
    output.normal = vec3<f32>(lm_norm.zw, sign_z * sqrt(1.0 - xy_len));
    output.world = position;
    output.fog = compute_fog_factor(position);
    output.material = m_params;
    return output;
}

@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    let specular = textureSample(i_specular, wrap_sampler, input.uv.xy).xyz;
    let lit = s3_shadowed_lightmap(input.world, input.uv.zw);
    let view_dir = normalize(globals.view_position.xyz - input.world);
    let normal = input.normal;
    let reflection = s3_reflection(normal, view_dir, lit.g, specular.z);
    let reflected = reflect(vec3<f32>(-0.14, 0.5, 0.9), normal);
    let highlight = pow(clamp(dot(view_dir, -reflected), 0.0, 1.0), 10.0 + specular.y * 290.0);
    let colour = diffuse.rgb * diffuse.rgb * lit * diffuse.a + reflection
        + highlight * vec3<f32>(2.1, 1.8, 1.5) * lit.g * specular.x;
    let alpha = diffuse.a * diffuse.a;
    if (alpha < 16.0 * 0.0039215689) {
        discard;
    }
    return vec4<f32>(s3_tone(input, colour).rgb, alpha);
}
";

/// `environmentdiffuse`, `alphatestdefaultenvironment` and `transparentdefaultenvironment`:
/// diffuse squared times the shadowed lightmap and m_params.y, then fog.
const S3_UNLIT: &str = r"
struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) world: vec3<f32>,
    @location(2) fog: vec4<f32>,
    @location(3) @interpolate(flat) material: vec4<f32>,
}

fn s3_unlit_vs(position: vec3<f32>, uv: vec2<f32>, lightmap_uv: vec2<f32>, m_params: vec4<f32>) -> PsIn {
    var output: PsIn;
    output.position = world_clip(position);
    output.world = position;
    output.uv = vec4<f32>(uv, lightmap_uv);
    output.fog = compute_fog_factor(position);
    output.material = m_params;
    return output;
}

// alphatest/transparentdefaultenvironment_defaultVS: TEXCOORD1's two lanes as they are.
@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    return s3_unlit_vs(position, uv, lm.xy, m_params);
}

// environmentdiffuse_defaultVS: |TEXCOORD1.xy|, the lanes keeping baseenvironment's sign packing.
@vertex fn diffuseVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    return s3_unlit_vs(position, uv, abs(lm.xy), m_params);
}

fn s3_unlit_colour(input: PsIn) -> vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    let colour = diffuse.rgb * diffuse.rgb * s3_shadowed_lightmap(input.world, input.uv.zw);
    return vec4<f32>(colour * input.material.y * input.fog.a + input.fog.rgb, diffuse.a);
}

// environmentdiffuse_defaultPS: the short tone curve 0.5 (1 - sat(1 - x)^2); alpha never written.
@fragment fn diffusePS(input: PsIn) -> @location(0) vec4<f32> {
    let fogged = s3_unlit_colour(input).rgb;
    let shoulder = clamp(vec3<f32>(1.0) - fogged * globals.envattributes2.x, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(sqrt(vec3<f32>(0.5) - 0.5 * shoulder * shoulder), 0.0);
}

// alphatestdefaultenvironment_defaultPS: the full tone curve and the diffuse alpha, which the
// pass rejects below 30 before the depth write.
@fragment fn alphatestPS(input: PsIn) -> @location(0) vec4<f32> {
    let colour = s3_unlit_colour(input);
    if (colour.a < 30.0 * 0.0039215689) {
        discard;
    }
    return vec4<f32>(sqrt(tonemap(vec4<f32>(colour.rgb * globals.envattributes2.x, 1.0)).rgb), colour.a);
}

// transparentdefaultenvironment_defaultPS: the same program, blended by its alpha.
@fragment fn transparentPS(input: PsIn) -> @location(0) vec4<f32> {
    let colour = s3_unlit_colour(input);
    return vec4<f32>(sqrt(tonemap(vec4<f32>(colour.rgb * globals.envattributes2.x, 1.0)).rgb), colour.a);
}
";

/// `tree`, `treeanimate` and `vertexanimate` (`tree_defaultPS` pc 3-19 and its siblings): one tap of
/// the lightmap's rgb, squared, raised to a floor and scaled by `g_ViewDotLight.x`, times the
/// albedo; fog, tone and square root over the diffuse alpha, which each pass tests >= 30 (blend
/// states 0x8307DEF0 / 0x8307DE84).
const S3_FOLIAGE: &str = r"
fn s3_foliage(lightmap_uv: vec2<f32>, minimum: f32, albedo: vec3<f32>, alpha: f32, fog: vec4<f32>) -> vec4<f32> {
    if (alpha < 30.0 * 0.0039215689) {
        discard;
    }
    let lightmap = textureSample(i_lightmap, clamp_sampler, lightmap_uv).rgb;
    let light = max(lightmap * lightmap, vec3<f32>(minimum)) * globals.view_dot_light.x;
    let tone = fog_bloom_tone(vec4<f32>(light * albedo, 1.0), fog, globals.envattributes2.x);
    return vec4<f32>(tone.rgb, alpha);
}

// treeanimate/vertexanimate_defaultVS: TEXCOORD1.zw - 0.5 and their sum - 1 weight three sines,
// each phase reduced to [-pi, pi], of amplitude weight (m_params[0].w, m_params[1].w,
// m_params[2].w).
fn s3_sway_weight(lm: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(lm.zw, lm.z + lm.w) - vec3<f32>(0.5, 0.5, 1.0);
}

fn s3_sway(weight: vec3<f32>, phase: vec3<f32>, amplitude: vec3<f32>) -> vec3<f32> {
    let reduced = fract(phase * 0.159154937 + vec3<f32>(0.5)) * 6.28318548 - vec3<f32>(3.14159274);
    return sin(reduced) * weight * amplitude;
}
";

const S3_TREE: &str = r"
struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) fog: vec4<f32>,
    @location(2) @interpolate(flat) material: vec4<f32>,
}

// tree_defaultVS: the fetched world position, TEXCOORD0 and |TEXCOORD1.xy|.
@vertex fn treeVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    var output: PsIn;
    output.position = world_clip(position);
    output.uv = vec4<f32>(uv, abs(lm.xy));
    output.fog = compute_fog_factor(position);
    output.material = m_params;
    return output;
}

// tree_defaultPS: the squared diffuse times m_params[0].y, lit no lower than g_ViewDotLight.y.
@fragment fn treePS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    let albedo = diffuse.rgb * diffuse.rgb * input.material.y;
    return s3_foliage(input.uv.zw, globals.view_dot_light.y, albedo, diffuse.a, input.fog);
}

// vertexanimate_defaultVS: the sway at phase (g_fAnimationTime.x - weight) m_params[1].xyz +
// m_params[2].xyz.
@vertex fn vertexAnimateVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(8) params1: vec4<f32>,
    @location(9) params2: vec4<f32>,
    @location(11) params0: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    let weight = s3_sway_weight(lm);
    let phase = (globals.animation_time.xxx - weight) * params1.xyz + params2.xyz;
    let moved = position + s3_sway(weight, phase, vec3<f32>(params0.w, params1.w, params2.w));
    var output: PsIn;
    output.position = world_clip(moved);
    output.uv = vec4<f32>(uv, abs(lm.xy));
    output.fog = compute_fog_factor(moved);
    output.material = params0;
    return output;
}

// vertexanimate_defaultPS: no lighting floor and no m_params.
@fragment fn vertexAnimatePS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    return s3_foliage(input.uv.zw, 0.0, diffuse.rgb * diffuse.rgb, diffuse.a, input.fog);
}
";

const S3_TREE_ANIMATE_BINDINGS: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var i_lightmap: texture_2d<f32>;
@group(1) @binding(2) var i_noise: texture_2d<f32>;
@group(1) @binding(3) var wrap_sampler: sampler;
@group(1) @binding(4) var clamp_sampler: sampler;
@group(1) @binding(5) var noise_sampler: sampler;
";

const S3_TREE_ANIMATE: &str = r"
struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) fog: vec4<f32>,
    @location(2) noise_uv: vec2<f32>,
}

// treeanimate_defaultVS: the sway at phase (g_fAnimationTime.x - weight) m_params[1].xyz + weight
// m_params[2].xyz; the noise coordinate is TEXCOORD0 less the x and z sway times
// g_fAnimationTime.y.
@vertex fn treeAnimateVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(8) params1: vec4<f32>,
    @location(9) params2: vec4<f32>,
    @location(11) params0: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    let weight = s3_sway_weight(lm);
    let phase = (globals.animation_time.xxx - weight) * params1.xyz + weight * params2.xyz;
    let sway = s3_sway(weight, phase, vec3<f32>(params0.w, params1.w, params2.w));
    let moved = position + sway;
    var output: PsIn;
    output.position = world_clip(moved);
    output.uv = vec4<f32>(uv, abs(lm.xy));
    output.fog = compute_fog_factor(moved);
    output.noise_uv = uv - sway.xz * globals.animation_time.y;
    return output;
}

// treeanimate_defaultPS: the squared diffuse times the squared i_noise, without m_params.
@fragment fn treeAnimatePS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, wrap_sampler, input.uv.xy);
    let noise = textureSample(i_noise, noise_sampler, input.noise_uv).rgb;
    let albedo = diffuse.rgb * diffuse.rgb * noise * noise;
    return s3_foliage(input.uv.zw, globals.view_dot_light.y, albedo, diffuse.a, input.fog);
}
";

const PROXY_WORLD: &str = r"
@group(1) @binding(0) var i_diffuse: texture_2d<f32>;
@group(1) @binding(1) var pass_reset_sampler: sampler;

struct PsIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fog: vec4<f32>,
    @location(2) @interpolate(flat) material: vec4<f32>,
}

// proxyworld.fx:10 defaultVS as proxyworld_defaultVS.vpo compiled it: oPos is
// the four dp4 of the fetched world position against g_matVP, the UV passes
// through, and ComputeFogFactor runs per vertex (c4 g_vViewPos, c5 g_FogK1,
// c6 g_FogColour); the fog alpha is 1 + g_FogColour.a * f1.
@vertex fn defaultVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(11) m_params: vec4<f32>
) -> PsIn {
    var output: PsIn;
    output.position = world_clip(position);
    output.uv = uv;
    output.fog = compute_fog_factor(position);
    output.material = m_params;
    return output;
}

// proxyworld.fx:23 defaultPS as proxyworld_defaultPS.fpo compiled it:
// GetDiffuseTextureValue squares rgb, all four lanes scale by m_params[0].y
// (c3.y), then PS_Fog_Bloom_Tone with g_envattributes[2].x (c2.x); every
// output lane is the square root.
@fragment fn defaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse = textureSample(i_diffuse, pass_reset_sampler, input.uv);
    let out_colour = vec4<f32>(diffuse.rgb * diffuse.rgb, diffuse.a);
    return fog_bloom_tone(out_colour * input.material.y, input.fog, globals.envattributes2.x);
}
";

#[cfg(test)]
#[path = "environment_material_tests.rs"]
mod material_tests;

#[cfg(test)]
mod tests {
    use super::{LIGHTING_ATTRIBUTES, LIGHTING_STRIDE, MATERIAL_ATTRIBUTES};

    #[test]
    fn material_lanes_follow_the_world_and_color_instance_lanes() {
        for (index, attribute) in (0_u32..).zip(&MATERIAL_ATTRIBUTES[..4]) {
            assert_eq!(attribute.offset,16*u64::from(index));
            assert_eq!(attribute.shader_location,2+index);
        }
        for (index, attribute) in (0_u32..).zip(&MATERIAL_ATTRIBUTES[4..]) {
            assert_eq!(attribute.offset, 80 + 16 * u64::from(index));
            assert_eq!(attribute.shader_location, 7 + index);
            assert_eq!(attribute.format, wgpu::VertexFormat::Float32x4);
        }
        assert_eq!(LIGHTING_ATTRIBUTES[1].offset, 16);
        assert_eq!(LIGHTING_ATTRIBUTES[2].offset, 28);
        assert_eq!(LIGHTING_ATTRIBUTES[2].shader_location, 14);
        assert_eq!(LIGHTING_STRIDE, 36);
    }
}

/// Select only the functions/textures present in each final shader program.
fn water_source(effect: EnvironmentEffect) -> String {
    use std::fmt::Write;
    let mut source = COMMON.to_owned();
    for (binding, name) in effect.channels().iter().enumerate() {
        let dimension = if *name == "environment" { "cube" } else { "2d" };
        writeln!(
            source,
            "@group(1) @binding({binding}) var i_{name}: texture_{dimension}<f32>;"
        )
        .unwrap();
    }
    for (offset, name) in ["wrap_sampler", "clamp_sampler", "cube_sampler"]
        .iter()
        .enumerate()
    {
        let binding = effect.channels().len() + offset;
        writeln!(source, "@group(1) @binding({binding}) var {name}: sampler;").unwrap();
    }
    writeln!(
        source,
        "const IS_OCEAN: bool = {};",
        effect == EnvironmentEffect::Ocean
    )
    .unwrap();
    let mut body = include_str!("water_material.wgsl").to_owned();
    for (name, keep) in [
        ("PCA", effect != EnvironmentEffect::FlowingWater),
        ("SURFACE", effect != EnvironmentEffect::Ocean),
        ("WATER", effect == EnvironmentEffect::Water),
        ("FLOW", effect == EnvironmentEffect::FlowingWater),
        ("OCEAN", effect == EnvironmentEffect::Ocean),
    ] {
        if !keep {
            let start = body.find(&format!("// __{name}_BEGIN__")).unwrap();
            let end = body.find(&format!("// __{name}_END__")).unwrap();
            body.replace_range(start..end, "");
        }
    }
    source.push_str(&body);
    source
}
