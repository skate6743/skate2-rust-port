//! Retained GPU implementation of GenerateBloomBuffers82832c68 and colour
//! cube composition. Native bloom retained; the six lens reflections are
//! intentionally omitted, a PC presentation choice (2026-10-12).
use super::RenderResult;
use crate::{
    render_postfx::{Assets, DEFAULT_CUBE, Frame},
    render_postfx_math as math,
};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

// Slots16..23 formerly carried lens-reflection parameters. Leave the remaining
// shared shader layout stable for normal, cinematic and HoM composition.
type Constants = [[f32; 4]; 33];

// PostFX::Init 0x8284a908..944 and SetRects 0x82823050: the original
// scene domain is 1152x640. InitBuffers_Xbox360 0x8283ea58 derives its
// bloom surfaces from that domain, independently of the display size.
// Keep the scene/depth at the requested PC resolution, but preserve the
// original normalized bloom footprint when the window or render scale changes.
const BLOOM_REFERENCE_WIDTH: u32 = 1152;
const BLOOM_REFERENCE_HEIGHT: u32 = 640;

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}
impl Target {
    fn new(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("original PostFX surface"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self {
            texture,
            view,
            width,
            height,
        }
    }
}

#[cfg(test)]
#[path = "postfx_tests.rs"]
mod tests;
struct Stage {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    binding: wgpu::BindGroup,
}
impl Stage {
    fn record(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        constants: &Constants,
    ) {
        let bytes: Vec<u8> = constants
            .iter()
            .flatten()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        queue.write_buffer(&self.uniform, 0, &bytes);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("original PostFX stage"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.draw(0..3, 0..1);
    }
}
struct Readback {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
    frame: u64,
}
pub(super) struct PostFx {
    scene: Target,
    targets: Vec<Target>,
    stages: Vec<Stage>,
    cube: wgpu::Texture,
    readbacks: [Readback; 3],
    copy_slot: Option<usize>,
    serial: u64,
    pub(super) exposure: f32,
    luminance: Option<f32>,
    current_cube: u64,
    target_cube: u64,
    /// The current and target cubes' texels, held while they blend: `UpdateColourCube` keeps the
    /// cube resources it blends whatever the next scene's assets hold (a downloadable content
    /// world's cube stays current over the front end's).
    current_texels: Option<std::sync::Arc<[u8]>>,
    target_texels: Option<std::sync::Arc<[u8]>>,
    cube_blend: f32,
    cube_pixels: Vec<u8>,
    /// Current cube, target cube and blend of the texels last uploaded.
    uploaded_cube: Option<(u64, u64, f32)>,
    exposure_time: f32,
    /// `Tonemap`'s static `currentExposure`, taken from the exposure on its first call.
    online_exposure: Option<f32>,
    last_update: Option<std::time::Instant>,
    /// `m_FXMatrixR/G/B_A` (near) and `_B` (far) as the last render left them.
    grades: Option<[[[f32; 4]; 3]; 2]>,
    /// Presentation time since the grades were last written.
    grade_time: f32,
    diagnostic_view: u8,
    /// `RenderAOBoxes` after the uber or visual FX pass.
    deck_ao: super::deck_ao::DeckAo,
}
impl PostFx {
    /// `depth` is the scene's depth aspect, which the NIS grade reads.
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        depth: &wgpu::TextureView,
        stencil: &wgpu::TextureView,
        overlay: &wgpu::TextureView,
        multisampled: bool,
    ) -> Self {
        let scene = Target::new(device, width, height, format);
        // 0/1 original quarter;2/3 original sixteenth;4/5 original32x16;
        // 6 combine;7 base-bloom copy;8 full-resolution cinematic intermediate.
        let quarter = (BLOOM_REFERENCE_WIDTH / 4, BLOOM_REFERENCE_HEIGHT / 4);
        let sixteenth = (BLOOM_REFERENCE_WIDTH / 16, BLOOM_REFERENCE_HEIGHT / 16);
        let targets = vec![
            quarter,
            quarter,
            sixteenth,
            sixteenth,
            (32, 16),
            (32, 16),
            quarter,
            quarter,
            (width, height),
        ]
        .into_iter()
        .map(|(w, h)| Target::new(device, w, h, wgpu::TextureFormat::Rgba8Unorm))
        .collect::<Vec<_>>();
        let cube = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("original32x32x32 colour cube"),
            size: wgpu::Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let cube_view = cube.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("PostFX linear clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for i in 1..4 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: if i == 3 {
                        wgpu::TextureViewDimension::D3
                    } else {
                        wgpu::TextureViewDimension::D2
                    },
                    multisampled: false,
                },
                count: None,
            });
        }
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled,
            },
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 6, visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 7, visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint,
                view_dimension: wgpu::TextureViewDimension::D2, multisampled }, count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding:8,visibility:wgpu::ShaderStages::FRAGMENT,
            ty:wgpu::BindingType::Texture {sample_type:wgpu::TextureSampleType::Float {filterable:true},
                view_dimension:wgpu::TextureViewDimension::D2,multisampled:false},count:None,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PostFX depth, stencil and original x-ray"), entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PostFX depth, stencil and original x-ray"), bind_group_layouts: &[Some(&layout)], immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("native PostFX shaders"),
            source: wgpu::ShaderSource::Wgsl(if multisampled {
                include_str!("postfx.wgsl").replace("texture_depth_2d", "texture_depth_multisampled_2d")
                    .replace("texture_2d<u32>", "texture_multisampled_2d<u32>").into()
            } else {
                include_str!("postfx.wgsl").into()
            }),
        });
        let mut stages = Vec::new();
        for (name, a, b) in [
            ("extract", &scene.view, &scene.view),
            ("downsample", &targets[0].view, &scene.view),
            ("downsample", &targets[2].view, &scene.view),
            ("blur9", &targets[0].view, &scene.view),
            ("blur9", &targets[2].view, &scene.view),
            ("blur9", &targets[4].view, &scene.view),
            ("blur9", &targets[3].view, &scene.view),
            ("blur9", &targets[5].view, &scene.view),
            ("combined", &targets[1].view, &targets[2].view),
            ("bloom_copy", &targets[6].view, &scene.view),
            ("grade", &scene.view, &targets[7].view),
            ("visual", &scene.view, &targets[7].view),
            ("cinematic_grade", &targets[8].view, &targets[7].view),
            ("inspect", &scene.view, &scene.view),
            ("inspect", &targets[7].view, &scene.view),
            ("stereo_grade", &scene.view, &targets[7].view),
        ] {
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(name),
                size: 33 * 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: false,
            });
            let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(name),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&cube_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                    wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(overlay) },
                    wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(stencil) },
                    wgpu::BindGroupEntry {binding:8,resource:wgpu::BindingResource::TextureView(if name=="stereo_grade" {&targets[8].view} else {&scene.view})},
                ],
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some(name),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some(name),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: if name == "grade" || name == "cinematic_grade" || name == "stereo_grade" || name == "inspect" {
                            format
                        } else {
                            wgpu::TextureFormat::Rgba8Unorm
                        },
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
            });
            stages.push(Stage {
                pipeline,
                uniform,
                binding,
            });
        }
        let readbacks = std::array::from_fn(|_| Readback {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("delayed PostFX luminance"),
                size: 256 * 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            state: Arc::new(AtomicU8::new(0)),
            frame: 0,
        });
        let deck_ao = super::deck_ao::DeckAo::new(device, [format, wgpu::TextureFormat::Rgba8Unorm], depth, stencil, multisampled);
        Self {
            deck_ao,
            scene,
            targets,
            stages,
            cube,
            readbacks,
            copy_slot: None,
            serial: 0,
            exposure: 1.,
            luminance: None,
            current_cube: 0,
            target_cube: 0,
            current_texels: None,
            target_texels: None,
            cube_blend: 1.,
            cube_pixels: vec![0; 32 * 32 * 32 * 4],
            uploaded_cube: None,
            exposure_time: 0.,
            online_exposure: None,
            last_update: None,
            grades: None,
            grade_time: 0.,
            diagnostic_view: match std::env::var("SKATE2_POSTFX_VIEW").as_deref() {
                Ok("scene") => 1,
                Ok("bloom") => 2,
                _ => 0,
            },
        }
    }
    pub(super) fn scene_view(&self) -> &wgpu::TextureView {
        &self.scene.view
    }
    fn update_cube(
        &mut self,
        queue: &wgpu::Queue,
        assets: &Assets,
        frame: &Frame,
        dt: f32,
    ) -> RenderResult<()> {
        let key = if assets.cubes.contains_key(&frame.cube) {
            frame.cube
        } else if self.current_cube != 0 {
            self.current_cube
        } else {
            DEFAULT_CUBE
        };
        let initial = self.current_cube == 0;
        if initial {
            let texels = assets.cubes.get(&key).ok_or("PostFX initial cube absent")?;
            self.current_cube = key;
            self.target_cube = key;
            self.current_texels = Some(std::sync::Arc::clone(texels));
            self.target_texels = Some(std::sync::Arc::clone(texels));
            self.cube_blend = 1.;
        } else if self.cube_blend >= 1. && key != self.current_cube {
            self.target_texels = Some(std::sync::Arc::clone(assets.cubes.get(&key).ok_or("PostFX target cube absent")?));
            self.target_cube = key;
            self.cube_blend = 0.;
        } else if self.cube_blend >= 1. {
            return Ok(());
        }
        let a = std::sync::Arc::clone(self.target_texels.as_ref().ok_or("PostFX target cube absent")?);
        let b = std::sync::Arc::clone(self.current_texels.as_ref().ok_or("PostFX current cube absent")?);
        // VMXBlend2 uses vcuxwfp8, multiply+fmadd, then vcfpuxws8
        // (truncate). Preserve byte quantization before trilinear sampling.
        let t = (self.cube_blend + dt / 6.).min(1.);
        // A redraw of the same presentation update (dt 0) blends to the same
        // texels already uploaded.
        if self.uploaded_cube == Some((self.current_cube, self.target_cube, t)) {
            return Ok(());
        }
        self.uploaded_cube = Some((self.current_cube, self.target_cube, t));
        for (i, out) in self.cube_pixels.iter_mut().enumerate() {
            *out = if i % 4 == 3 {
                255
            } else {
                ((b[i] as f32 / 256.).mul_add(1. - t, a[i] as f32 / 256. * t) * 256.)
                    .clamp(0., 255.) as u8
            };
        }
        queue.write_texture(
            self.cube.as_image_copy(),
            &self.cube_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(128),
                rows_per_image: Some(32),
            },
            self.cube.size(),
        );
        // BLEND_INC82ff0234=1/360 per60Hz presentation tick. Keep its six
        // second duration when PC rendering runs faster than the original.
        self.cube_blend = t;
        if self.cube_blend >= 1. {
            self.current_cube = self.target_cube;
            self.current_texels = self.target_texels.clone();
        }
        Ok(())
    }
    /// With PostFX off the scene is drawn straight into `output`, where `RenderAOBoxes` still
    /// darkens it: the camera planes give `g_DOFCameraInfoX/Y`, and no bloom lightens the boxes.
    #[expect(clippy::too_many_arguments, reason = "RenderAOBoxes' inputs")]
    pub(super) fn record_deck_ao(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        assets: &Assets,
        frame: &Frame,
        deck_ao: &super::deck_ao::DeckAoFrame,
    ) {
        let Some([near_plane, far_plane]) = frame.camera_planes else { return };
        let depth = far_plane - near_plane;
        let dof = [-(far_plane * near_plane / depth), far_plane / depth];
        self.deck_ao.record(device, queue, encoder, output, format, deck_ao, assets.deck_ao, 0.0, dof);
    }

    #[expect(clippy::too_many_arguments, reason = "the frame's PostFX inputs")]
    pub(super) fn record(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        assets: &Assets,
        frame: &Frame,
        deck_ao: Option<&super::deck_ao::DeckAoFrame>,
        eye: Option<&super::stereoscopy::Eye>,
    ) -> RenderResult<()> {
        // Each submit's device maintenance maps the readbacks the GPU has finished.
        let mut newest = None;
        for read in &mut self.readbacks {
            if self.serial.saturating_sub(read.frame) >= 2
                && read.state.load(Ordering::Acquire) == 2
            {
                if newest.is_none_or(|serial| read.frame > serial) {
                    let bytes = read.buffer.slice(..).get_mapped_range()?;
                    self.luminance = math::luminance(&bytes, 256);
                    newest = Some(read.frame);
                }
                read.buffer.unmap();
                read.state.store(0, Ordering::Release);
            }
            if read.state.load(Ordering::Acquire) == 3 {
                return Err("PostFX luminance GPU readback failed".into());
            }
        }
        // Inspection publishes dt once per completed presentation update.
        // Uncapped redraws reuse that scene: consuming its dt again makes
        // exposure and the six-second cube blend run at render-FPS speed.
        let advanced = self.last_update.is_none_or(|last| frame.updated_at > last);
        let dt = if advanced {
            self.last_update = Some(frame.updated_at);
            frame.dt
        } else {
            0.
        };
        self.exposure_time += dt;
        let step = if frame.cinematic { 1. / 30. } else { 1. / 60. };
        // PostFX::Tonemap 0x82829248 adapts once per rendered frame from that frame's
        // luminance, so a long frame does not catch up: repeated steps on one (late)
        // reading overshoot and swing the exposure.
        let stepped = self.luminance.is_some() && self.exposure_time >= step;
        if let Some(lum) = self.luminance.filter(|_| stepped) {
            self.exposure = math::adapt(self.exposure, lum, frame.exposure);
            self.exposure_time -= step;
        }
        self.exposure_time = self.exposure_time.min(step);
        // PostFX::Tonemap 0x828292a0..c0: while the ICE post-fx is active a
        // positive tonemapValue replaces the published exposure, which the
        // next adaptation then starts from.
        if let Some(ice) = frame.ice
            && ice.tonemap > 0.
        {
            self.exposure = ice.tonemap;
        }
        // 0x828292d8..9338: on the front end's Online menu the exposure is the static
        // currentExposure, which then steps 0.05 a frame down to 0.59; elsewhere it follows
        // the exposure.
        let current = *self.online_exposure.get_or_insert(self.exposure);
        if frame.fe_online_menu {
            self.exposure = current;
            if stepped {
                let next = current - 0.05;
                self.online_exposure = Some(if next < 0.59 { 0.59 } else { next });
            }
        } else {
            self.online_exposure = Some(self.exposure);
        }
        self.update_cube(queue, assets, frame, dt)?;
        if self.serial % 240 == 0 && std::env::var_os("SKATE2_TRACE_POSTFX").is_some() {
            eprintln!("postfx frame={} exposure={} luminance={:?} input={frame:?} current_cube={:016x} target_cube={:016x} blend={} matrix={:?}",
                self.serial, self.exposure, self.luminance, self.current_cube,
                self.target_cube, self.cube_blend, assets.matrix);
        }
        let [threshold, saturation, coarse, medium, fine] = frame.bloom;
        // m_RadialBlur: 0 outside ICE (SetParameters 0x8284bc58).
        let radial = frame.ice.map_or(0., |ice| ice.radial_blur);
        let mut c = [[0.; 4]; 33];
        c[9] = math::bloom_coefficients(threshold);
        c[10..13].copy_from_slice(&math::saturation_matrix(1. + saturation));
        // GenerateBloomBuffers 0x82832c68: offsets use main2's native
        // dimensions. Its quad UV shift is the Debug attribute divided by
        // those dimensions (the shipped attribute is zero). Xenos integer
        // pixel centers require subtracting half a destination texel from
        // WGPU's half-integer centers before applying that UV shift.
        c[24] = [
            1. / BLOOM_REFERENCE_WIDTH as f32,
            1. / BLOOM_REFERENCE_HEIGHT as f32,
            assets.bloom_sample_offset / BLOOM_REFERENCE_WIDTH as f32
                - 0.5 / self.targets[0].width as f32,
            assets.bloom_sample_offset / BLOOM_REFERENCE_HEIGHT as f32
                - 0.5 / self.targets[0].height as f32,
        ];
        self.stages[0].record(queue, encoder, &self.targets[0].view, &c);
        // Subsequent downsample and Blur9 quads explicitly add half a
        // destination texel; that cancels the pixel-center conversion.
        c[24] = [
            1. / self.targets[0].width as f32,
            1. / self.targets[0].height as f32,
            0.,
            0.,
        ];
        self.stages[1].record(queue, encoder, &self.targets[2].view, &c);
        c[24] = [
            1. / self.targets[2].width as f32,
            1. / self.targets[2].height as f32,
            0.,
            0.,
        ];
        self.stages[2].record(queue, encoder, &self.targets[4].view, &c);
        for (stage, source, target, vertical, multiply) in [
            (3, 0, 1, false, 1.),
            (4, 2, 3, false, medium),
            (5, 4, 5, false, coarse),
            (6, 3, 2, true, 1.),
            (7, 5, 4, true, coarse),
        ] {
            c[..9].copy_from_slice(&math::blur_samples(self.targets[source].width));
            c[13] = [multiply, 0., 0., 0.];
            // PfxHelper::Blur9 0x82830c60..78: (direction, 0, m_RadialBlur).
            c[14] = if vertical {
                [0., 1., 0., radial]
            } else {
                [1., 0., 0., radial]
            };
            self.stages[stage].record(queue, encoder, &self.targets[target].view, &c);
        }
        c[..9].copy_from_slice(&math::blur_samples(self.targets[1].height));
        c[13] = [fine, 1., if coarse > 1. { 1. / coarse } else { 1. }, 0.];
        // Blur9_combined3 0x8282955c: (0, 0, 0, m_RadialBlur).
        c[14] = [0., 0., 0., radial];
        // Combined and the base-bloom copy use RenderQuad(nullptr), hence no native UV
        // shift. Both destinations have the original quarter dimensions.
        c[24][2] = -0.5 / self.targets[6].width as f32;
        c[24][3] = -0.5 / self.targets[6].height as f32;
        self.stages[8].record(queue, encoder, &self.targets[6].view, &c);
        // PC visual exception: omit the six colored reflections
        // from native 82130038. Keep its base-bloom resample so normal glow
        // reaches both gameplay and cinematic composition with the same phase.
        self.stages[9].record(queue, encoder, &self.targets[7].view, &c);
        // ICE: ApplyFX_NEW's near (A) and far (B) grades replace the vault's. The grades are
        // PostFX members: a frame whose VisualFX_Animation wrote neither (its challenge fade
        // branch) keeps the last ones.
        let rewritten = match frame.ice {
            Some(ice) => Some(ice.matrices),
            None => frame.tazer.then_some([[[-1.,0.,0.,1.],[0.,-1.,0.,1.],[0.,0.,-1.,1.]];2])
                .or_else(|| frame.hom.map(|hom| hom.matrices))
                // VisualFX_Animation82847610's normal branch writes both
                // authored grades. ICELite selects the cinematic pass without
                // activating ICE PostFX; it still needs the normal far grade.
                .or_else(|| (!frame.keep_matrices).then_some([assets.matrix,assets.hom.normal_far])),
        };
        // PostFX::Apply 0x8284c92c..0x8284cafc: a positive first bloom coefficient (a
        // threshold below zero) moves the members toward the identity by at most 1, in place,
        // each render: once over freshly written grades, and once per 60 Hz presentation step
        // over kept ones.
        let moves = if let Some(grades) = rewritten {
            self.grades = Some(grades);
            self.grade_time = 0.;
            1
        } else {
            self.grade_time += dt;
            let mut moves = 0;
            while self.grade_time >= step {
                self.grade_time -= step;
                moves += 1;
            }
            moves
        };
        let mut grades = self.grades.unwrap_or([assets.matrix; 2]);
        if c[9][0] > 0. {
            let white = c[9][0].min(1.);
            let keep = 1. - white;
            for _ in 0..moves {
                for grade in &mut grades {
                    for (axis, row) in grade.iter_mut().enumerate() {
                        for (lane, value) in row.iter_mut().enumerate() {
                            *value = value.mul_add(keep, if lane == axis { white } else { 0. });
                        }
                    }
                }
            }
        }
        self.grades = Some(grades);
        c[10..13].copy_from_slice(&grades[0]);
        c[27..30].copy_from_slice(&grades[1]);
        let depth_inputs = frame.ice.map(|ice| (ice.lerp, ice.planes))
            .or_else(|| frame.hom.map(|hom| (hom.lerp, hom.planes)))
            .or_else(|| eye.map(|e|(assets.hom.normal_distance,e.planes)))
            .or_else(|| frame.camera_planes.map(|planes|(assets.hom.normal_distance,planes)));
        c[26] = [frame.ice.map_or(0., |ice| ice.scanline), 0., 0., 0.];
        c[32] = [frame.hom.map_or(0., |hom| hom.overlay), if frame.ice.is_some() { 0. } else { 1. }, 0., 0.];
        if let Some((lerp, planes)) = depth_inputs {
            // SetParameters' tail 0x8284c6e8..0x8284c77c: g_NISLerp_Mult
            // (-1/(F-N), 1/(F-N)) and g_NISLerp_Add (F/(F-N), N * -1/(F-N)).
            let [n, f] = lerp;
            let range = f - n;
            let mult = -1. / range;
            c[30] = [mult, 1. / range, f / range, mult * n];
            // 0x8284bd50..dd8: g_DOFCameraInfoX -(fn/(f-n)), Y f/(f-n).
            let [near_plane, far_plane] = planes;
            let depth = far_plane - near_plane;
            c[31] = [-(far_plane * near_plane / depth), far_plane / depth, 0., 0.];
        }
        c[15][0] = if frame.cinematic { 1.41412354 } else { 1.41 };
        c[25] = frame.final_mult;
        // ApplyVisualFX 0x82846914 / ApplyFastUber 0x82846c74: RenderAOBoxes into the target
        // the pass just wrote, with this frame's `g_DOFCameraInfoX/Y`.
        let boxes = deck_ao.filter(|_| depth_inputs.is_some());
        let (bloom, dof) = (c[9][0], [c[31][0], c[31][1]]);
        if let Some(eye)=eye {
            if eye.field==0 {
                self.stages[11].record(queue,encoder,&self.targets[8].view,&c);
                if let Some(boxes) = boxes {
                    self.deck_ao.record(device, queue, encoder, &self.targets[8].view, wgpu::TextureFormat::Rgba8Unorm, boxes, assets.deck_ao, bloom, dof);
                }
            }
            else {
                c[15][0]=1.41;
                self.stages[15].record(queue,encoder,output,&c);
                if let Some(boxes) = boxes {
                    self.deck_ao.record(device, queue, encoder, output, format, boxes, assets.deck_ao, bloom, dof);
                }
            }
        } else if frame.cinematic {
            if depth_inputs.is_none() {
                return Err("cinematic PostFX frame without grade inputs".into());
            }
            self.stages[11].record(queue, encoder, &self.targets[8].view, &c);
            if let Some(boxes) = boxes {
                self.deck_ao.record(device, queue, encoder, &self.targets[8].view, wgpu::TextureFormat::Rgba8Unorm, boxes, assets.deck_ao, bloom, dof);
            }
            self.stages[12].record(queue, encoder, output, &c);
        } else {
            self.stages[10].record(queue, encoder, output, &c);
            if let Some(boxes) = boxes {
                self.deck_ao.record(device, queue, encoder, output, format, boxes, assets.deck_ao, bloom, dof);
            }
        }
        if self.diagnostic_view != 0 && !eye.is_some_and(|e|e.field==0) {
            self.stages[12 + self.diagnostic_view as usize].record(queue, encoder, output, &c);
        }
        // Exposure adapts once per presentation update, from the newest luminance
        // read back: the frames drawn between updates need not read theirs.
        let slot = self.serial as usize % 3;
        if advanced && self.readbacks[slot].state.load(Ordering::Acquire) == 0 {
            encoder.copy_texture_to_buffer(
                self.targets[4].texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &self.readbacks[slot].buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(16),
                    },
                },
                self.targets[4].texture.size(),
            );
            self.readbacks[slot].frame = self.serial;
            self.copy_slot = Some(slot);
        }
        self.serial += 1;
        Ok(())
    }
    pub(super) fn after_submit(&mut self) {
        if let Some(slot) = self.copy_slot.take() {
            let read = &self.readbacks[slot];
            let state = read.state.clone();
            state.store(1, Ordering::Release);
            read.buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| {
                    state.store(if r.is_ok() { 2 } else { 3 }, Ordering::Release)
                });
        }
    }
}
