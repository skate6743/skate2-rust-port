//! APT triangle/text submission over the existing native texture uploader.
use super::{GpuRenderer, GpuScene, RenderResult};
use skate2_core::texture::NativeTexture;
use std::{collections::BTreeMap, ops::Range, sync::Arc};
use wgpu::util::DeviceExt;

pub(crate) struct Batch {
    pub texture: Option<String>,
    pub wrapped: bool,
    /// clip.xyzw, UV, RGBA. Original colour transform is applied to the
    /// element/font colour BEFORE texture sampling (AptTextureUnit::Draw).
    pub vertices: Vec<[f32; 10]>,
}
pub(crate) struct DrawList {
    vertices: wgpu::Buffer,
    draws: Vec<(Range<u32>, Arc<wgpu::BindGroup>)>,
}
pub(crate) struct WorldPanel {
    view: wgpu::TextureView,
    texture: Arc<wgpu::BindGroup>,
    contents: Option<DrawList>,
    quad: Option<DrawList>,
}
pub(super) struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    world_pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    samplers: [wgpu::Sampler; 2],
    white: Arc<wgpu::BindGroup>,
    eye: super::stereoscopy::Correction,
}
pub(crate) struct Textures {
    bindings: BTreeMap<String, [Arc<wgpu::BindGroup>; 2]>,
}
impl Textures {
    pub(crate) fn extend(&mut self,other:Self){self.bindings.extend(other.bindings);}
    pub(crate) fn contains_key(&self, key: &str) -> bool { self.bindings.contains_key(key) }
}
impl Pipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, samples:u32) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("APT texture"),
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
        let samplers = [wgpu::AddressMode::ClampToEdge, wgpu::AddressMode::Repeat].map(|mode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("APT sampler"),
                address_mode_u: mode,
                address_mode_v: mode,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            })
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("APT"),
            source: wgpu::ShaderSource::Wgsl(include_str!("marker_ui.wgsl").into()),
        });
        let eye=super::stereoscopy::Correction::new(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("APT"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let world_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:Some("APT world eye"), bind_group_layouts:&[Some(&layout),Some(&eye.layout)],immediate_size:0,
        });
        let create = |world| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("APT original geometry and fonts"),
            layout: Some(if world {&world_layout} else {&pipeline_layout}),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some(if world {"vs_world"} else {"vs"}),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 40,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x2,2=>Float32x4],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: if world { Some("fs_world") } else { Some("fs") },
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: if world {Some(wgpu::DepthStencilState {
                format:super::DEPTH_FORMAT,depth_write_enabled:Some(true),depth_compare:Some(wgpu::CompareFunction::LessEqual),
                stencil:Default::default(),bias:Default::default(),
            })}else{None},
            multisample: wgpu::MultisampleState {count:if world{samples}else{1},..Default::default()},
            multiview_mask: None,
        });
        let pipeline=create(false);
        let world_pipeline=create(true);
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("APT solid fill"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[255; 4],
        );
        let white = Arc::new(bind(
            device,
            &layout,
            &samplers[0],
            &texture.create_view(&Default::default()),
        ));
        Self {
            pipeline,
            world_pipeline,
            layout,
            samplers,
            white,
            eye,
        }
    }
    pub(super) fn set_eye(&self,queue:&wgpu::Queue,eye:Option<&super::stereoscopy::Eye>) { self.eye.update(queue,eye); }
    pub(super) fn record_panel(&self,encoder:&mut wgpu::CommandEncoder,panel:&WorldPanel){
        {
            let _pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label:Some("APT layer1 render to texture"),
                color_attachments:&[Some(wgpu::RenderPassColorAttachment{depth_slice:None,view:&panel.view,resolve_target:None,
                    ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),store:wgpu::StoreOp::Store}})],
                depth_stencil_attachment:None,timestamp_writes:None,occlusion_query_set:None,
            });
        }
        if let Some(contents)=&panel.contents{self.draw(encoder,&panel.view,contents);}
    }
    pub(super) fn draw_panel<'a>(&'a self,pass:&mut wgpu::RenderPass<'a>,panel:&'a WorldPanel){
        if let Some(list)=&panel.quad{self.draw_world(pass,list);}
    }
    /// A clip-space list drawn into the scene, tested against its depth and writing it.
    pub(super) fn draw_world<'a>(&'a self,pass:&mut wgpu::RenderPass<'a>,list:&'a DrawList){
        pass.set_pipeline(&self.world_pipeline);
        pass.set_bind_group(1,&self.eye.binding,&[]);
        pass.set_vertex_buffer(0,list.vertices.slice(..));
        for (range,binding) in &list.draws{pass.set_bind_group(0,binding.as_ref(),&[]);pass.draw(range.clone(),0..1);}
    }
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        list: &DrawList,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("APT HUD"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, list.vertices.slice(..));
        for (range, binding) in &list.draws {
            pass.set_bind_group(0, binding.as_ref(), &[]);
            pass.draw(range.clone(), 0..1);
        }
    }
}
fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("APT texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
impl GpuRenderer {
    /// DownsampleForScreenshot828326e0: 64/128 x64, relative to the 640x360
    /// movie viewport, centered on rendering/skate_watermark's authored point.
    /// This overlay belongs to the capture targets only, not the FE preview.
    pub(crate) fn replay_watermark(&self,texture:&super::GpuTexture,position:[f32;2],pro:bool)->RenderResult<Option<DrawList>> {
        let binding=Arc::new(bind(&self.device,&self.marker_ui.layout,&self.marker_ui.samplers[0],&texture.view));
        let textures=Textures {bindings:BTreeMap::from([("watermark".into(),[binding.clone(),binding])])};
        let scale=[if pro {128.} else {64.}/640.,64./360.];
        let translate=position.map(|v|v*2.-1.);
        let vertices=[(-1.,-1.,0.,1.),(1.,-1.,1.,1.),(-1.,1.,0.,0.),(-1.,1.,0.,0.),(1.,-1.,1.,1.),(1.,1.,1.,0.)]
            .map(|(x,y,u,v)|[x*scale[0]+translate[0],y*scale[1]+translate[1],0.,1.,u,v,1.,1.,1.,1.]).to_vec();
        self.upload_ui_batches(&textures,vec![Batch {texture:Some("watermark".into()),wrapped:false,vertices}])
    }
    /// The frontend APT offscreen, its quad, then the secondary quad with its texture.
    pub(crate) fn update_world_ui(&self,scene:&mut GpuScene,textures:&Textures,batches:Vec<Batch>,quad:Vec<[f32;10]>,
        secondary:Option<(String,Vec<[f32;10]>)>)->RenderResult<()> {
        if scene.world_ui.is_none(){
            let texture=self.device.create_texture(&wgpu::TextureDescriptor{
                label:Some("Original frontend APT offscreen1280x720"),
                size:wgpu::Extent3d{width:1280,height:720,depth_or_array_layers:1},mip_level_count:1,sample_count:1,
                dimension:wgpu::TextureDimension::D2,format:self.config.format,
                usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::TEXTURE_BINDING,view_formats:&[],
            });
            let view=texture.create_view(&Default::default());
            let texture=Arc::new(bind(&self.device,&self.marker_ui.layout,&self.marker_ui.samplers[0],&view));
            scene.world_ui=Some(WorldPanel{view,texture,contents:None,quad:None});
        }
        let panel=scene.world_ui.as_mut().expect("allocated panel");
        panel.contents=self.upload_ui_batches(textures,batches)?;
        let texture=panel.texture.clone();
        let mut bindings=BTreeMap::from([("frontend".into(),[texture.clone(),texture])]);
        let mut quads=vec![Batch{texture:Some("frontend".into()),wrapped:false,vertices:quad}];
        if let Some((key,vertices))=secondary {
            let binding=textures.bindings.get(&key).ok_or_else(||format!("UI texture not uploaded: {key}"))?.clone();
            bindings.insert(key.clone(),binding);
            quads.push(Batch{texture:Some(key),wrapped:false,vertices});
        }
        panel.quad=self.upload_ui_batches(&Textures{bindings},quads)?;
        Ok(())
    }
    pub(crate) fn upload_ui_textures<'a>(
        &self,
        textures: impl Iterator<Item = &'a NativeTexture>,
    ) -> RenderResult<Textures> {
        let mut bindings = BTreeMap::new();
        for t in textures {
            let gpu = self.upload_native_texture(t)?;
            let pair = std::array::from_fn(|i| {
                Arc::new(bind(
                    &self.device,
                    &self.marker_ui.layout,
                    &self.marker_ui.samplers[i],
                    &gpu.view,
                ))
            });
            bindings.insert(t.name.to_ascii_lowercase(), pair);
        }
        Ok(Textures { bindings })
    }
    pub(crate) fn update_marker_ui(
        &self,
        scene: &mut GpuScene,
        textures: &Textures,
        batches: Vec<Batch>,
    ) -> RenderResult<()> {
        scene.marker_ui = self.upload_ui_batches(textures, batches)?;
        Ok(())
    }
    pub(crate) fn render_fps(&self) -> u32 { self.fps.value }
    pub(crate) fn set_frontend_fade(&self, scene: &mut GpuScene, alpha: f32) -> RenderResult<()> {
        if scene.frontend_fade_alpha == alpha { return Ok(()); }
        scene.frontend_fade = self.cover(0., alpha)?;
        scene.frontend_fade_alpha = alpha;
        Ok(())
    }
    /// The NIS transition's bloom to white as a white cover of its value, which draws only
    /// while PostFX is off (with PostFX the bloom itself shows).
    pub(crate) fn set_nis_white(&self, scene: &mut GpuScene, alpha: f32) -> RenderResult<()> {
        if scene.nis_white_alpha.to_bits() == alpha.to_bits() { return Ok(()); }
        scene.nis_white = self.cover(1., alpha)?;
        scene.nis_white_alpha = alpha;
        Ok(())
    }
    /// The screen fades as a black cover of their value, with PostFX or without.
    pub(crate) fn set_fade_black(&self, scene: &mut GpuScene, alpha: f32) -> RenderResult<()> {
        if scene.fade_black_alpha.to_bits() == alpha.to_bits() { return Ok(()); }
        scene.fade_black = self.cover(0., alpha)?;
        scene.fade_black_alpha = alpha;
        Ok(())
    }
    /// A full-screen quad of grey `level` at `alpha`; none while it is clear.
    fn cover(&self, level: f32, alpha: f32) -> RenderResult<Option<DrawList>> {
        if alpha == 0. { return Ok(None); }
        let vertices = [(-1.,-1.), (1.,-1.), (-1.,1.),
            (-1.,1.), (1.,-1.), (1.,1.)].map(|(x,y)|
            [x,y,0.,1.,0.,0.,level,level,level,alpha]).to_vec();
        self.upload_ui_batches(&Textures { bindings: BTreeMap::new() },
            vec![Batch { texture: None, wrapped: false, vertices }])
    }
    pub(crate) fn upload_ui_batches(
        &self, textures: &Textures, batches: Vec<Batch>,
    ) -> RenderResult<Option<DrawList>> {
        let mut bytes = Vec::new();
        let mut draws = Vec::new();
        let mut count = 0;
        for batch in batches {
            if batch.vertices.is_empty() {
                continue;
            }
            let binding = if let Some(name) = batch.texture {
                textures
                    .bindings
                    .get(&name.to_ascii_lowercase())
                    .ok_or_else(|| format!("UI texture not uploaded: {name}"))?
                    [usize::from(batch.wrapped)]
                .clone()
            } else {
                self.marker_ui.white.clone()
            };
            let start = count;
            for vertex in batch.vertices {
                for lane in vertex {
                    bytes.extend_from_slice(&lane.to_le_bytes());
                }
                count += 1;
            }
            draws.push((start..count, binding));
        }
        Ok(if bytes.is_empty() {
            None
        } else {
            Some(DrawList {
                vertices: self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("APT frame vertices"),
                        contents: &bytes,
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                draws,
            })
        })
    }
}
