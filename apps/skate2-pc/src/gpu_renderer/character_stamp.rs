//! livingworld_stamp final programs, rather than the older embedded UPDB text.
//! Source binding: cPresPedestrian::BindConstants8280b5d0. See pedestrian
//! rendering evidence2026-09-30 for final-program differences and limitations.
use super::*;

pub(crate) struct Inputs {
    pub material: [[f32; 4]; 7],
    pub irradiance: [[f32; 4]; 9],
    pub attributes: [[f32; 4]; 3],
    pub light: [f32; 4],
    pub csm: [[f32; 4]; 4],
    pub camera: [f32; 3],
    pub material_lod: f32,
}
impl Inputs {
    fn bytes(&self) -> [u8; 30 * 16] {
        self.bytes_custom([[0.; 4]; 5])
    }
    fn bytes_custom(&self, custom: [[f32; 4]; 5]) -> [u8; 30 * 16] {
        let camera = [self.camera[0], self.camera[1], self.camera[2], self.material_lod];
        // PC weather: overcast keeps the ambient SH and takes the direct key and rim sunlight.
        let mut material = self.material;
        let mut light = self.light;
        let cloud = crate::weather::cloud_cover();
        if cloud > 0. {
            for index in [1, 2, 3, 6] {
                for value in &mut material[index][..3] {
                    *value *= 1. - cloud;
                }
            }
            light[3] = cloud;
        }
        let lanes = material
            .iter()
            .chain(&self.irradiance)
            .chain(&self.attributes)
            .chain(std::iter::once(&light))
            .chain(&self.csm)
            .chain(std::iter::once(&camera))
            .chain(custom.iter())
            .flatten();
        let mut data = [0; 30 * 16];
        for (bytes, value) in data.chunks_exact_mut(4).zip(lanes) {
            bytes.copy_from_slice(&value.to_ne_bytes());
        }
        data
    }
}
pub(in crate::gpu_renderer) struct Binding {
    exposure: ExposureOverride,
    pub(in crate::gpu_renderer) group: wgpu::BindGroup,
    constants: wgpu::Buffer,
    pub(in crate::gpu_renderer) cas: bool,
}
impl Binding {
    pub(super) fn apply_display_exposure(&self, uploads: &super::super::frame_uploads::FrameUploads, enabled: bool) {
        // Stamp.material[7], sh[9], attributes[2].z.
        self.exposure.apply(uploads, &self.constants, (7 + 9 + 2) * 16 + 8, enabled);
    }
}
pub(super) struct Pipelines {
    layout: wgpu::BindGroupLayout,
    normal_sampler: wgpu::Sampler,
    surface_sampler: wgpu::Sampler,
    shadow_sampler: wgpu::Sampler,
    world_sampler: wgpu::Sampler,
    default: wgpu::RenderPipeline,
    ghost: [wgpu::RenderPipeline; 2],
    plain: wgpu::RenderPipeline,
    plain_ghost: [wgpu::RenderPipeline; 2],
    character: wgpu::RenderPipeline,
    character_ghost: [wgpu::RenderPipeline; 2],
    hair: [wgpu::RenderPipeline; 3],
    hair_ropa: [wgpu::RenderPipeline; 3],
    cas: wgpu::RenderPipeline,
    cas_ghost: [wgpu::RenderPipeline; 2],
    cas_hair: [wgpu::RenderPipeline; 3],
    /// livingworld_stamp's `crowd` technique (programs 0 and 4), which
    /// `CrowdImposterTarget::RenderPresEntity` 0x827e9298 selects for the imposter atlas.
    crowd: wgpu::RenderPipeline,
}
pub(super) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0=>Float32x3,1=>Float32x2,2=>Uint8x4,3=>Uint8x4,4=>Float32x3,5=>Float32x3];
pub(super) fn vectors(
    geometry: &NativeMeshGeometry,
) -> Result<(&[[f32; 3]], &[[f32; 3]]), CharacterGpuError> {
    let vector = |semantic| {
        let value = attribute(geometry, semantic)?;
        match &value.values {
            NativeVertexAttributeValues::Float3(values) => Ok(values.as_slice()),
            _ => Err(CharacterGpuError::UnsupportedAttributeFormat(
                semantic,
                value.original_format,
            )),
        }
    };
    Ok((
        vector(VertexSemantic::Tangent(0))?,
        vector(VertexSemantic::Other { usage: 7, index: 0 })?,
    ))
}
impl Pipelines {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        objects: &wgpu::BindGroupLayout,
        textures: &wgpu::BindGroupLayout,
        rows: &wgpu::BindGroupLayout,
    ) -> Self {
        let uniform = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("livingworld_stamp constants, normal and actual shadow maps"),
            entries: &[
                uniform(0),
                texture(1),
                sampler(2),
                texture(3),
                texture(4),
                sampler(5),
                uniform(6),
                sampler(7),
                texture(8),
                sampler(9),
                texture(10),
                texture(11),
            ],
        });
        // No sampler declarations in the manifest: original pass reset is wrap,
        // linear min/mag, point mip. Shadows use their original clamp sampler.
        let normal_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let world_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        // defaultcharacter/defaulthair manifests override both diffuse and
        // normal to ANISOTROPIC4 with LINEAR mip; alpha keeps pass defaults.
        let surface_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        let border = device
            .features()
            .contains(wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER);
        let address = if border {
            wgpu::AddressMode::ClampToBorder
        } else {
            wgpu::AddressMode::ClampToEdge
        };
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: address,
            address_mode_v: address,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            border_color: border.then_some(wgpu::SamplerBorderColor::TransparentBlack),
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("character, hair, CAS and livingworld_stamp GPU programs"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{SKINNING}{SHADER}{}{}", surface::SHADER, cas::SHADER).into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("livingworld_stamp"),
            bind_group_layouts: &[Some(objects), Some(textures), Some(rows), Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |pass| {
            character_pipeline(
                device,
                &shader,
                &pipeline_layout,
                format,
                sample_count,
                pass,
            )
        };
        Self {
            crowd: pipeline(CharacterPass {
                label: "livingworld_stamp crowd",
                vertex: "vs_stamp",
                fragment: "fs_stamp_crowd",
                cull: CullMode::None,
                ..DEFAULT_PASS
            }),
            default: pipeline(CharacterPass {
                label: "livingworld_stamp default",
                vertex: "vs_stamp",
                fragment: "fs_stamp",
                cull: CullMode::None,
                ..DEFAULT_PASS
            }),
            plain: pipeline(CharacterPass {
                label: "defaultlivingworld final",
                vertex: "vs_plain",
                fragment: "fs_plain",
                ..DEFAULT_PASS
            }),
            plain_ghost: [
                pipeline(CharacterPass {
                    vertex: "vs_plain",
                    fragment: "fs_plain",
                    ..GHOST_DEPTH
                }),
                pipeline(CharacterPass {
                    vertex: "vs_plain",
                    fragment: "fs_plain_ghost",
                    ..GHOST_BLEND
                }),
            ],
            ghost: [
                pipeline(CharacterPass {
                    vertex: "vs_stamp",
                    fragment: "fs_stamp",
                    ..GHOST_DEPTH
                }),
                pipeline(CharacterPass {
                    vertex: "vs_stamp",
                    fragment: "fs_stamp_ghost",
                    ..GHOST_BLEND
                }),
            ],
            character: pipeline(CharacterPass {
                vertex: "vs_stamp",
                fragment: "fs_surface",
                ..DEFAULT_PASS
            }),
            character_ghost: [
                pipeline(CharacterPass {
                    vertex: "vs_stamp",
                    fragment: "fs_surface",
                    ..GHOST_DEPTH
                }),
                pipeline(CharacterPass {
                    vertex: "vs_stamp",
                    fragment: "fs_surface_ghost",
                    ..GHOST_BLEND
                }),
            ],
            hair: HAIR_PASSES.map(|pass| {
                pipeline(CharacterPass {
                    vertex: "vs_surface_hair",
                    fragment: "fs_surface_hair",
                    ..pass
                })
            }),
            hair_ropa: HAIR_PASSES.map(|pass| {
                pipeline(CharacterPass {
                    label: "defaulthair_ropa final",
                    vertex: "vs_surface_hair",
                    fragment: "fs_surface_hair_ropa",
                    ..pass
                })
            }),
            cas: pipeline(CharacterPass {
                vertex: "vs_cas",
                fragment: "fs_cas",
                ..DEFAULT_PASS
            }),
            cas_ghost: [
                pipeline(CharacterPass {
                    vertex: "vs_cas",
                    fragment: "fs_cas",
                    ..GHOST_DEPTH
                }),
                pipeline(CharacterPass {
                    vertex: "vs_cas",
                    fragment: "fs_cas_ghost",
                    ..GHOST_BLEND
                }),
            ],
            cas_hair: HAIR_PASSES.map(|pass| {
                pipeline(CharacterPass {
                    vertex: "vs_cas",
                    fragment: "fs_cas",
                    ..pass
                })
            }),
            layout,
            normal_sampler,
            surface_sampler,
            shadow_sampler,
            world_sampler,
        }
    }
    pub(super) fn crowd_technique(&self) -> &[wgpu::RenderPipeline] {
        std::slice::from_ref(&self.crowd)
    }
    pub(super) fn technique(&self, ghosted: bool) -> &[wgpu::RenderPipeline] {
        if ghosted {
            &self.ghost
        } else {
            std::slice::from_ref(&self.default)
        }
    }
    pub(super) fn cas_technique(
        &self,
        family: CharacterShaderFamily,
        ghosted: bool,
    ) -> &[wgpu::RenderPipeline] {
        if family == CharacterShaderFamily::Hair {
            &self.cas_hair
        } else if ghosted {
            &self.cas_ghost
        } else {
            std::slice::from_ref(&self.cas)
        }
    }
    pub(super) fn surface_technique(
        &self,
        family: CharacterShaderFamily,
        ghosted: bool,
    ) -> &[wgpu::RenderPipeline] {
        match family {
            CharacterShaderFamily::LivingWorld if ghosted => &self.plain_ghost,
            CharacterShaderFamily::LivingWorld => std::slice::from_ref(&self.plain),
            CharacterShaderFamily::Hair => &self.hair,
            CharacterShaderFamily::HairRopa => &self.hair_ropa,
            CharacterShaderFamily::Character | CharacterShaderFamily::Ropa if ghosted => &self.character_ghost,
            CharacterShaderFamily::Character | CharacterShaderFamily::Ropa => std::slice::from_ref(&self.character),
            _ => self.technique(ghosted),
        }
    }
}
impl GpuRenderer {
    pub(crate) fn bind_living_world_lighting(
        &self,
        object: &mut GpuObject,
        diffuse: &GpuTexture,
        inputs: &Inputs,
    ) -> RenderResult<()> {
        if object
            .character
            .as_ref()
            .is_none_or(|c| c.family != CharacterShaderFamily::LivingWorld)
        {
            return Err("livingworld binding on other effect".into());
        }
        // Plain entrypoints never sample the normal/alpha slots. Reuse the
        // resident diffuse solely to satisfy the shared bind-group layout.
        self.bind_character_lighting(object, diffuse, None, inputs)
    }
    pub(crate) fn bind_character_stamp(
        &self,
        object: &mut GpuObject,
        normal: &GpuTexture,
        inputs: &Inputs,
    ) -> RenderResult<()> {
        if object
            .character
            .as_ref()
            .is_none_or(|c| c.family != CharacterShaderFamily::LivingWorldStamp)
        {
            return Err("stamp binding on other effect".into());
        }
        self.bind_character_lighting(object, normal, None, inputs)
    }
    pub(super) fn bind_character_lighting(
        &self,
        object: &mut GpuObject,
        normal: &GpuTexture,
        alpha: Option<&GpuTexture>,
        inputs: &Inputs,
    ) -> RenderResult<()> {
        self.bind_character_lighting_custom(object, normal, alpha, None, None, inputs, None)
    }
    pub(super) fn bind_character_lighting_custom(
        &self,
        object: &mut GpuObject,
        normal: &GpuTexture,
        alpha: Option<&GpuTexture>,
        decal: Option<&GpuTexture>,
        decal2: Option<&GpuTexture>,
        inputs: &Inputs,
        custom: Option<[[f32; 4]; 5]>,
    ) -> RenderResult<()> {
        if normal.cube {
            return Err("livingworld_stamp normal must be 2D".into());
        }
        let character = object
            .character
            .as_mut()
            .ok_or(CharacterGpuError::NotCharacterObject)?;
        if alpha.is_some_and(|t| t.cube) {
            return Err("character alpha must be 2D".into());
        }
        let constants = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("character original constants"),
                contents: &inputs.bytes_custom(custom.unwrap_or([[0.; 4]; 5])),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let pipelines = &self.character_pipelines.stamp;
        let normal_sampler = if custom.map(|c| c[4][0] != 4.).unwrap_or(matches!(
            character.family,
            CharacterShaderFamily::Character | CharacterShaderFamily::Hair | CharacterShaderFamily::HairRopa
        )) {
            &pipelines.surface_sampler
        } else {
            &pipelines.normal_sampler
        };
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("livingworld_stamp original bindings"),
            layout: &pipelines.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: constants.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&normal.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(self.shadows.atlas()),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(self.shadows.world_map()),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&pipelines.shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.shadows.world_rows().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&pipelines.world_sampler),
                },
                // Non-hair entrypoints never sample binding8. Reuse an actual
                // resident texture to satisfy the shared layout, no new raster.
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&alpha.unwrap_or(normal).view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::Sampler(&pipelines.normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::TextureView(&decal.unwrap_or(normal).view),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: wgpu::BindingResource::TextureView(&decal2.unwrap_or(normal).view),
                },
            ],
        });
        character.stamp = Some(Binding {
            exposure: ExposureOverride::new(inputs.attributes[2][2]),
            group,
            constants,
            cas: custom.is_some(),
        });
        Ok(())
    }
    pub(crate) fn update_character_stamp(
        &self,
        object: &GpuObject,
        inputs: &Inputs,
    ) -> RenderResult<()> {
        let binding = object
            .character
            .as_ref()
            .and_then(|c| c.stamp.as_ref())
            .ok_or("pedestrian stamp binding absent")?;
        self.character_uploads
            .write(&binding.constants, 0, &inputs.bytes());
        binding.exposure.supplied(inputs.attributes[2][2]);
        Ok(())
    }
    pub(crate) fn update_character_cas_constants(
        &self,
        object: &GpuObject,
        inputs: &Inputs,
        custom: [[f32; 4]; 5],
    ) -> RenderResult<()> {
        let binding = object
            .character
            .as_ref()
            .and_then(|c| c.stamp.as_ref())
            .ok_or("CAS lighting binding absent")?;
        if !binding.cas {
            return Err("CAS constants on other character".into());
        }
        self.character_uploads
            .write(&binding.constants, 0, &inputs.bytes_custom(custom));
        binding.exposure.supplied(inputs.attributes[2][2]);
        Ok(())
    }
}

const SHADER: &str = r"
struct Object { world_to_clip:mat4x4<f32>, color:vec4<f32> }
struct Stamp { material:array<vec4<f32>,7>, sh:array<vec4<f32>,9>, attributes:array<vec4<f32>,3>,
    light:vec4<f32>, csm:array<vec4<f32>,4>, camera_lod:vec4<f32>, custom:array<vec4<f32>,5> }
struct WorldShadow {rows:array<vec4<f32>,3>}
@group(0) @binding(0) var<uniform> object:Object;
@group(1) @binding(0) var diffuse:texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler:sampler;
@group(2) @binding(0) var<uniform> palette:Palette;
@group(3) @binding(0) var<uniform> stamp:Stamp;
@group(3) @binding(1) var normal_map:texture_2d<f32>;
@group(3) @binding(2) var normal_sampler:sampler;
@group(3) @binding(3) var shadow_csm:texture_2d<f32>;
@group(3) @binding(4) var shadow_world:texture_2d<f32>;
@group(3) @binding(5) var shadow_sampler:sampler;
@group(3) @binding(6) var<uniform> world_shadow:WorldShadow;
@group(3) @binding(7) var world_sampler:sampler;
// Centroid: with MSAA an edge pixel whose centre lies off the triangle keeps its attributes
// inside it, so silhouettes never sample beyond the surface (texture, normal, shadow lookup).
struct Output { @builtin(position) @invariant position:vec4<f32>,@location(0) @interpolate(perspective, centroid) uv:vec4<f32>,
    @location(1) @interpolate(perspective, centroid) tangent:vec3<f32>,@location(2) @interpolate(perspective, centroid) binormal:vec3<f32>,
    @location(3) @interpolate(perspective, centroid) normal:vec3<f32>,@location(4) @interpolate(perspective, centroid) relative:vec3<f32>,
    @location(5) @interpolate(perspective, centroid) fog:vec4<f32> }
@vertex fn vs_stamp(@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,
    @location(2) stored_weights:vec4<u32>,@location(3) stored_indices:vec4<u32>,
    @location(4) tangent:vec3<f32>,@location(5) binormal:vec3<f32>)->Output {
    return surface_vertex(position,uv,stored_weights,stored_indices,tangent,binormal);
}
fn surface_vertex(position:vec3<f32>,uv:vec2<f32>,stored_weights:vec4<u32>,stored_indices:vec4<u32>,
    tangent:vec3<f32>,binormal:vec3<f32>)->Output {
    var weights=vec4<f32>(stored_weights)/255.;weights.w=1.-dot(vec3<f32>(1.),weights.xyz);
    let indices=stored_indices*3u;
    let a=blend_row(indices,weights,0u).xyz;let b=blend_row(indices,weights,1u).xyz;let c=blend_row(indices,weights,2u).xyz;
    let t=vec3<f32>(dot(tangent,a),dot(tangent,b),dot(tangent,c));
    let n=vec3<f32>(dot(binormal,a),dot(binormal,b),dot(binormal,c));
    let world=skin_world(position,stored_weights,stored_indices);
    let k=palette.living_world[2];let amount=pow(clamp(length(world.xyz)*k.x+k.y,0.,1.),k.z);
    return Output(object.world_to_clip*world,vec4<f32>(uv,0.,0.),t,n,cross(t,n),world.xyz,
        palette.living_world[3]*amount+vec4<f32>(0.,0.,0.,1.));
}
fn shadow_uv(world:vec4<f32>)->vec3<f32>{
    let first=vec2<f32>(dot(stamp.csm[0],world),dot(stamp.csm[2],world));
    let second=first*stamp.csm[1].xy+stamp.csm[1].zw;
    var uv=vec2<f32>(-1.);
    if max(abs(second.x),abs(second.y))<.99 {uv=second*vec2<f32>(.25,-.5)+vec2<f32>(.75,.5);}
    if max(abs(first.x),abs(first.y))<.99 {uv=first*vec2<f32>(.25,-.5)+vec2<f32>(.25,.5);}
    return vec3<f32>(uv,dot(stamp.csm[3],world));
}
fn tap(uv:vec2<f32>,depth:f32)->f32{
    let moments=textureSample(shadow_csm,shadow_sampler,uv).xy;
    return select(0.,1.,depth>=moments.x && moments.y!=0.);
}
fn csm_shadow_parameters(world:vec4<f32>,bias:f32,radius:f32)->f32{
    let coord=shadow_uv(world);let depth=coord.z-bias;
    let u=radius/1024.;let v=radius/512.;
    var sum=tap(coord.xy+vec2<f32>(-u,-v),depth);
    sum+=tap(coord.xy+vec2<f32>(0.,-v),depth);sum+=tap(coord.xy+vec2<f32>(u,-v),depth);
    sum+=tap(coord.xy+vec2<f32>(-u,0.),depth);sum+=tap(coord.xy,depth);sum+=tap(coord.xy+vec2<f32>(u,0.),depth);
    sum+=tap(coord.xy+vec2<f32>(-u,v),depth);sum+=tap(coord.xy+vec2<f32>(0.,v),depth);sum+=tap(coord.xy+vec2<f32>(u,v),depth);
    return 1.-sum*.111111112;
}
fn csm_shadow(world:vec4<f32>)->f32{
    return csm_shadow_parameters(world,stamp.attributes[0].x,stamp.attributes[0].z);
}
fn world_shadow_value(world:vec4<f32>)->f32{
    let p=vec2<f32>(dot(world_shadow.rows[0],world),dot(world_shadow.rows[1],world));
    let depth=dot(world_shadow.rows[2],world);
    let moments=textureSample(shadow_world,world_sampler,p*vec2<f32>(.5,-.5)+vec2<f32>(.5)).xy;
    return select(0.,1.,max(abs(p.x),abs(p.y))<.99)*clamp(moments.y*select(0.,1.,moments.x>=depth),0.,1.);
}
// Final defaultlivingworld_defaultPS pc18..23, unlike its older UPDB text,
// multiplies gain by m_params[5].w and saturate(worldShadow+m_params[7].w).
struct PlainOutput { @builtin(position) @invariant position:vec4<f32>,
    @location(0) @interpolate(perspective, centroid) uv:vec2<f32>,@location(1) @interpolate(perspective, centroid) relative:vec3<f32>,@location(2) @interpolate(perspective, centroid) fog:vec4<f32> }
@vertex fn vs_plain(@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,
    @location(2) weights:vec4<u32>,@location(3) indices:vec4<u32>)->PlainOutput {
    let world=skin_world(position,weights,indices);
    let k=palette.living_world[2];let amount=pow(clamp(length(world.xyz)*k.x+k.y,0.,1.),k.z);
    return PlainOutput(object.world_to_clip*world,uv,world.xyz,
        palette.living_world[3]*amount+vec4<f32>(0.,0.,0.,1.));
}
fn plain_colour(input:PlainOutput)->vec4<f32>{
    let sampled=textureSample(diffuse,normal_sampler,input.uv).xyz;
    var colour=sampled*sampled;
    if colour.y<.001225 {colour=colour.x*palette.living_world[0].xyz+colour.z*palette.living_world[1].xyz;}
    let parameters=palette.living_world[4];
    let world=vec4<f32>(input.relative+stamp.camera_lod.xyz,1.);
    let gain=(parameters.x*parameters.z)*clamp(world_shadow_value(world)+parameters.w,0.,1.);
    let x=(colour*(gain*input.fog.a)+input.fog.xyz)*parameters.y;
    let low=clamp(vec3<f32>(1.)-x,vec3<f32>(0.),vec3<f32>(1.));
    let mapped=(max(x*.25+vec3<f32>(.75),vec3<f32>(1.))-low*low)*.5;
    return vec4<f32>(sqrt(abs(mapped)),object.color.a);
}
@fragment fn fs_plain(input:PlainOutput)->@location(0) vec4<f32>{return plain_colour(input);}
@fragment fn fs_plain_ghost(input:PlainOutput)->@location(0) vec4<f32>{
    let colour=plain_colour(input);if object.color.a<.0039215689 {discard;}return colour;
}
fn irradiance(n:vec3<f32>)->vec3<f32>{
    var value=stamp.sh[0].xyz;
    value+=stamp.sh[1].xyz*n.x;value+=stamp.sh[2].xyz*n.y;value+=stamp.sh[3].xyz*n.z;
    value+=stamp.sh[4].xyz*(n.x*n.z);value+=stamp.sh[5].xyz*(n.z*n.y);value+=stamp.sh[6].xyz*(n.y*n.x);
    value+=stamp.sh[7].xyz*(3.*n.z*n.z-1.);value+=stamp.sh[8].xyz*(n.x*n.x-n.y*n.y);
    return max(value,vec3<f32>(.036,.040,.042));
}
fn stamp_colour(input:Output)->vec4<f32>{
    let sample=textureSample(diffuse,normal_sampler,input.uv.xy);
    var colour=sample.xyz*sample.xyz;
    if colour.y<.001225 {colour=colour.x*palette.living_world[0].xyz+colour.z*palette.living_world[1].xyz;}
    let normal=textureSample(normal_map,normal_sampler,input.uv.xy).xyz*2.-vec3<f32>(1.);
    let n=normalize(normal.x*input.tangent+normal.y*input.binormal+normal.z*input.normal);
    let view=normalize(-input.relative);let light=stamp.light.xyz;
    let rim=normalize(light*vec3<f32>(-1.,.2,-1.)-view);
    let world=vec4<f32>(input.relative+stamp.camera_lod.xyz,1.);
    let world_shadow=world_shadow_value(world);let shadow=csm_shadow(world)*world_shadow;
    let facing=1.-clamp(dot(n,view),0.,1.);
    let key_fresnel=pow(facing,stamp.material[1].w);let rim_fresnel=pow(facing,stamp.material[0].z);
    let key_diffuse=clamp(dot(n,light),0.,1.)*stamp.material[1].xyz*shadow;
    let rim_diffuse=clamp(dot(n,rim)*rim_fresnel,0.,1.)*stamp.material[6].xyz;
    let ambient=irradiance(n)*stamp.material[5].w;
    let key_spec=pow(clamp(dot(view,reflect(-light,n)),0.,1.),stamp.material[2].w)*shadow;
    let rim_spec=pow(clamp(dot(view,reflect(-rim,n)),0.,1.),stamp.material[3].w);
    let spec=clamp((key_spec*stamp.material[2].xyz*key_fresnel+rim_spec*stamp.material[3].xyz*rim_fresnel)*normal.y,vec3<f32>(0.),vec3<f32>(1.));
    let lit=colour*(key_diffuse+ambient+rim_diffuse)+spec;
    let far=colour*.3*clamp(world_shadow+.1,0.,1.);
    let blended=lit+(far-lit)*stamp.camera_lod.w;
    let x=(blended*(stamp.material[0].y*input.fog.a)+input.fog.xyz)*stamp.attributes[2].z;
    let low=clamp(vec3<f32>(1.)-x,vec3<f32>(0.),vec3<f32>(1.));
    let mapped=(max(x*.25+vec3<f32>(.75),vec3<f32>(1.))-low*low)*.5;
    return vec4<f32>(sqrt(mapped),object.color.a);
}
@fragment fn fs_stamp(input:Output)->@location(0) vec4<f32>{return stamp_colour(input);}
@fragment fn fs_stamp_ghost(input:Output)->@location(0) vec4<f32>{
    let colour=stamp_colour(input);
    if object.color.a<.0039215689 {discard;}return colour;
}
// Final livingworld_stamp_crowdPS: the default lighting with only the CSM self-shadow, no
// colorize, LOD blend, gain, fog or tone; alpha marks a texel's colorize green as 0, else 1.
fn stamp_crowd_colour(input:Output)->vec4<f32>{
    let sample=textureSample(diffuse,normal_sampler,input.uv.xy);
    let colour=sample.xyz*sample.xyz;
    let normal=textureSample(normal_map,normal_sampler,input.uv.xy).xyz*2.-vec3<f32>(1.);
    let n=normalize(normal.x*input.tangent+normal.y*input.binormal+normal.z*input.normal);
    let view=normalize(-input.relative);let light=stamp.light.xyz;
    let rim=normalize(light*vec3<f32>(-1.,.2,-1.)-view);
    let shadow=csm_shadow(vec4<f32>(input.relative+stamp.camera_lod.xyz,1.));
    let facing=1.-clamp(dot(n,view),0.,1.);
    let key_fresnel=pow(facing,stamp.material[1].w);let rim_fresnel=pow(facing,stamp.material[0].z);
    let key_diffuse=clamp(dot(n,light),0.,1.)*stamp.material[1].xyz*shadow;
    let rim_diffuse=clamp(dot(n,rim)*rim_fresnel,0.,1.)*stamp.material[6].xyz;
    let ambient=irradiance(n)*stamp.material[5].w;
    let key_spec=pow(clamp(dot(view,reflect(-light,n)),0.,1.),stamp.material[2].w)*shadow;
    let rim_spec=pow(clamp(dot(view,reflect(-rim,n)),0.,1.),stamp.material[3].w);
    let spec=clamp((key_spec*stamp.material[2].xyz*key_fresnel+rim_spec*stamp.material[3].xyz*rim_fresnel)*normal.y,vec3<f32>(0.),vec3<f32>(1.));
    return vec4<f32>(colour*(key_diffuse+ambient+rim_diffuse)+spec,trunc(colour.y+.998775));
}
@fragment fn fs_stamp_crowd(input:Output)->@location(0) vec4<f32>{return stamp_crowd_colour(input);}
";

#[cfg(test)]
mod tests {
    #[test]
    fn the_programs_validate() {
        let source = format!("{}{}{}{}", super::SKINNING, super::SHADER, super::surface::SHADER, super::cas::SHADER);
        let module = wgpu::naga::front::wgsl::parse_str(&source).expect("livingworld_stamp WGSL");
        wgpu::naga::valid::Validator::new(wgpu::naga::valid::ValidationFlags::all(), wgpu::naga::valid::Capabilities::all())
            .validate(&module)
            .expect("livingworld_stamp WGSL validation");
    }
}
