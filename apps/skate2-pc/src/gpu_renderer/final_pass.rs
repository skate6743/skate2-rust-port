//! Original final colour blit, including actual image distortion and snow.
//! `PostFX::Blit_ToDeviceRT` 0x82831c18 and the shipped pixel programs at
//! 0x8212cda8 / 0x8212cff0. The existing scene renderer and texture uploader
//! retain ownership; this module adds the native sampled final target.
use super::{GpuTexture, RenderResult};
use crate::render_environment::with_vault;
use skate2_core::respawn_noise::{NOISE_TEXTURE_SIZE, RespawnNoise, create_noise_texture_rgba};
use skate2_formats::mount::MountSet;

/// `SetParameters` 0x8284bc74..bd24 reads `postfx/main` every render; the
/// mounted immutable Vault is sampled when the live view is initialized.
/// Native debug edits, ICE/replay overrides and challenge bloom are separate
/// presentation producers and are not synthesized by this freeroam reader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FisheyeParameters {
    pub(crate) warp: [f32; 4],
    pub(crate) vignette: [f32; 4],
    pub(crate) widescreen: bool,
    /// PC presentation adjustment, separate from original tuning: the lift without PostFX and
    /// Display Settings' Brightness.
    pub(crate) brightness: f32,
    /// PC: the scene is larger than the target (Resolution Scale above 100%), so each target
    /// pixel averages it rather than taking one sample. The renderer sets it from the sizes.
    pub(crate) supersample: bool,
}
impl FisheyeParameters {
    /// PC preference: identity sampling and a white lens mask. Noise and
    /// frontend fades retain their separate owners. The warp magnifies the image, so the game
    /// camera narrows instead while the fisheye is off ([`Self::edge_scale`]).
    pub(super) fn for_display(mut self, settings: skate2_core::ui::options::PcDisplaySettings) -> Self {
        if !settings.fisheye { self.warp = [0., 1., 0., 0.]; self.vignette = [0., 0.25, 0., 1.]; }
        // Explicit PC preferences: compensate the ungraded scene by +22%
        // (October 2), a further +15% (October 6), then +10% (October 12).
        // Applied once before the later HUD/fade/FPS passes.
        if !settings.postfx { self.brightness *= 1.22 * 1.15 * 1.10; }
        // Display Settings' Brightness, over either: at 100% the scene is as before.
        self.brightness *= f32::from(settings.brightness) / 100.;
        self
    }
    /// The warp's factor `y + x r^4` (r the UV radius) at the middle of each screen edge
    /// (r^2 = 0.25): with it the game camera's field of view reaches, undistorted, as far as the
    /// fisheye's view does (the projection is linear in the angle).
    pub(crate) fn edge_scale(&self) -> f32 {
        self.warp[0].mul_add(0.0625, self.warp[1])
    }
    pub(crate) fn load(mounts: &MountSet) -> RenderResult<Self> {
        let brightness = match std::env::var("SKATE2_BRIGHTNESS") {
            Ok(value) => value.parse::<f32>()?,
            // Native PostFX now supplies exposure and grading. The former
            // 1.08*1.10 compensation clipped its highlights after the LUT.
            Err(_) => 1.0,
        };
        if !brightness.is_finite() || brightness <= 0.0 {
            return Err("SKATE2_BRIGHTNESS must be a positive finite multiplier".into());
        }
        with_vault(mounts, |vault| {
            const CLASS: u64 = 0x46f4_3159_bc82_72ab;
            const COLLECTION: u64 = 0x316a_0393_1530_b5e6;
            // Preserve the source getter order: z, y, x.
            let [z] = vault.floats::<1>(CLASS, COLLECTION, 0xd35e_9a01_5df6_c989)?;
            let [y] = vault.floats::<1>(CLASS, COLLECTION, 0xc83e_225b_09fc_3d00)?;
            let [x] = vault.floats::<1>(CLASS, COLLECTION, 0x03d4_19b2_5e35_e90c)?;
            Ok(Self {
                warp: [x, y, z, 0.0],
                // VisualFX_Animation82847610 initializes this every render.
                // Blit_ToDeviceRT sets w to max(bloomCoefficient[0], 0).
                // Ordinary freeroam's no-positive-bloom branch leaves zero.
                vignette: [0.0, 0.25, 0.0, 0.0],
                widescreen: true,
                brightness,
                supersample: false,
            })
        })
    }
}

pub(super) struct FinalPass {
    postfx: super::postfx::PostFx,
    standard: wgpu::RenderPipeline,
    noisy: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    binding: wgpu::BindGroup,
    scene_texture: wgpu::Texture,
    scene: wgpu::TextureView,
    /// `scene` as the main target's copy reads it.
    main_source: super::main_target::MainTargetSource,
    noise_texture: wgpu::TextureView,
    vignette: GpuTexture,
    overlay: GpuTexture,
    clamp: wgpu::Sampler,
    repeat: wgpu::Sampler,
    format: wgpu::TextureFormat,
    noise: RespawnNoise,
    multisampled: bool,
    /// PC weather's pass, made for the scene format of the first weather frame, and what a
    /// new one needs.
    weather: Option<(super::weather::Weather, wgpu::TextureFormat, bool)>,
    weather_depth: wgpu::TextureView,
    weather_stencil: wgpu::TextureView,
    weather_size: (u32, u32),
}
impl FinalPass {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        vignette: &GpuTexture,
        depth: &wgpu::TextureView,
        stencil: &wgpu::TextureView,
        overlay: &GpuTexture,
        multisampled: bool,
        main_target: &super::main_target::MainTarget,
    ) -> RenderResult<Self> {
        if vignette.cube {
            return Err("native vignette_gradient.Texture must be 2D".into());
        }
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("original final-blit constants"),
            size: 8 * 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for binding in 1..=3 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        for binding in 4..=5 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("original final blit"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("original final blit"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("basictex_fisheye and basictex_fisheye_noise"),
            source: wgpu::ShaderSource::Wgsl(include_str!("final_pass.wgsl").into()),
        });
        let pipeline = |entry_point| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some(entry_point),
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
                    entry_point: Some(entry_point),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
            })
        };
        let standard = pipeline("fs_standard");
        let noisy = pipeline("fs_noise");
        let sampler = |mode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("original final-blit linear sampler"),
                address_mode_u: mode,
                address_mode_v: mode,
                address_mode_w: mode,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
        };
        let clamp = sampler(wgpu::AddressMode::ClampToEdge);
        let repeat = sampler(wgpu::AddressMode::Repeat);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("CreateNoiseTexture_ARGB32"),
            size: wgpu::Extent3d {
                width: NOISE_TEXTURE_SIZE,
                height: NOISE_TEXTURE_SIZE,
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
            texture.as_image_copy(),
            &create_noise_texture_rgba()?,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(NOISE_TEXTURE_SIZE * 4),
                rows_per_image: Some(NOISE_TEXTURE_SIZE),
            },
            texture.size(),
        );
        let noise_texture = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let scene_texture = scene_target(device, format, width, height);
        let scene = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let main_source = main_target.source(device, &scene, width, height);
        let binding = binding(
            device,
            &layout,
            &uniform,
            [&scene, &noise_texture, &vignette.view],
            [&clamp, &repeat],
        );
        Ok(Self {
            postfx: super::postfx::PostFx::new(device,format,width,height,depth,stencil,&overlay.view,multisampled),
            standard,
            noisy,
            layout,
            uniform,
            binding,
            scene_texture,
            scene,
            main_source,
            noise_texture,
            vignette: vignette.clone(),
            overlay: overlay.clone(),
            clamp,
            repeat,
            format,
            noise: RespawnNoise::default(),
            multisampled,
            weather: None,
            weather_depth: depth.clone(),
            weather_stencil: stencil.clone(),
            weather_size: (width, height),
        })
    }

    pub(super) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32, depth: &wgpu::TextureView, stencil: &wgpu::TextureView,
        main_target: &super::main_target::MainTarget) {
        let exposure=self.postfx.exposure;
        self.postfx=super::postfx::PostFx::new(device,self.format,width,height,depth,stencil,&self.overlay.view,self.multisampled);
        self.postfx.exposure=exposure;
        self.scene_texture = scene_target(device, self.format, width, height);
        self.scene = self.scene_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.weather = None;
        self.weather_depth = depth.clone();
        self.weather_stencil = stencil.clone();
        self.weather_size = (width, height);
        self.main_source = main_target.source(device, &self.scene, width, height);
        self.binding = binding(
            device,
            &self.layout,
            &self.uniform,
            [&self.scene, &self.noise_texture, &self.vignette.view],
            [&self.clamp, &self.repeat],
        );
    }

    pub(super) fn scene_view(&self,postfx:bool) -> &wgpu::TextureView {
        if postfx {self.postfx.scene_view()} else {&self.scene}
    }

    /// The texture this frame's graded scene ends in, as the main target copies it.
    pub(super) fn main_target_source(&self) -> &super::main_target::MainTargetSource {
        &self.main_source
    }

    /// PC weather over the scene (the PostFX scene with PostFX on), before the native bloom,
    /// cinematic flashes and colour grading.
    pub(super) fn record_weather(&mut self,device:&wgpu::Device,queue:&wgpu::Queue,
        encoder:&mut wgpu::CommandEncoder,map:&super::GpuMap,frame:super::weather::Frame,postfx:bool)->RenderResult<()> {
        let texture=if postfx {self.postfx.scene_texture()} else {&self.scene_texture};
        let view=if postfx {self.postfx.scene_view()} else {&self.scene};
        let format=texture.format();
        if !self.weather.as_ref().is_some_and(|(_,made,made_postfx)| *made==format && *made_postfx==postfx) {
            let (width,height)=self.weather_size;
            let effect=super::weather::Weather::new(device,format,width,height,view,&self.weather_depth,&self.weather_stencil,self.multisampled);
            self.weather=Some((effect,format,postfx));
        }
        let (effect,_,_)=self.weather.as_ref().expect("weather pass made");
        effect.record(device,queue,encoder,map,frame)?;
        encoder.copy_texture_to_texture(effect.texture.as_image_copy(),texture.as_image_copy(),texture.size());
        Ok(())
    }

    pub(super) fn exposure(&self)->f32 {self.postfx.exposure}
    pub(super) fn after_submit(&mut self){self.postfx.after_submit();}
    #[expect(clippy::too_many_arguments, reason = "the frame's PostFX inputs")]
    pub(super) fn record_postfx(&mut self,device:&wgpu::Device,queue:&wgpu::Queue,
        encoder:&mut wgpu::CommandEncoder,assets:&crate::render_postfx::Assets,frame:&crate::render_postfx::Frame,
        deck_ao:Option<&super::deck_ao::DeckAoFrame>,eye:Option<&super::stereoscopy::Eye>)->RenderResult<()> {
        self.postfx.record(device,queue,encoder,&self.scene,self.format,assets,frame,deck_ao,eye)
    }

    /// `RenderAOBoxes` on the scene a frame drew without PostFX.
    pub(super) fn record_deck_ao(&mut self,device:&wgpu::Device,queue:&wgpu::Queue,encoder:&mut wgpu::CommandEncoder,
        assets:&crate::render_postfx::Assets,frame:&crate::render_postfx::Frame,deck_ao:&super::deck_ao::DeckAoFrame) {
        self.postfx.record_deck_ao(device,queue,encoder,&self.scene,self.format,assets,frame,deck_ao);
    }

    /// One call for each submitted scene frame; phase/RNG advance only when
    /// the native final-noise branch is selected, never during resize.
    #[expect(
        clippy::too_many_arguments,
        reason = "explicit native final-blit inputs"
    )]
    pub(super) fn record(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        parameters: &FisheyeParameters,
        boundary: Option<f32>,
        white_noise: f32,
        paused: bool,
    ) -> RenderResult<()> {
        let noise = self.noise.frame(boundary, white_noise, paused)?;
        let uv_transform = if parameters.widescreen {
            [1.0, 1.0, 0.0, 0.0]
        } else {
            [0.75, 1.0, 0.125, 0.0]
        };
        let values = [
            parameters.warp,
            parameters.vignette,
            noise.map_or([0.0; 4], |n| n.scroll),
            noise.map_or([1.0, 0.0, 0.0, 0.0], |n| n.fade),
            noise.map_or([0.0; 4], |n| n.dot1),
            noise.map_or([0.0; 4], |n| n.dot2),
            uv_transform,
            [parameters.brightness, if parameters.supersample { 1.0 } else { 0.0 }, rain_desaturation(), 0.0],
        ];
        let bytes: Vec<u8> = values
            .into_iter()
            .flatten()
            .flat_map(f32::to_ne_bytes)
            .collect();
        queue.write_buffer(&self.uniform, 0, &bytes);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("Blit_ToDeviceRT"),
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
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(if noise.is_some() {
            &self.noisy
        } else {
            &self.standard
        });
        pass.set_bind_group(0, &self.binding, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

fn scene_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("final-blit scene colour"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
}

/// PC weather: how far the rain greys the final image; none for any other weather.
fn rain_desaturation() -> f32 {
    match crate::weather::current() {
        crate::weather::Mode::Drizzle => 0.16,
        crate::weather::Mode::Rain => 0.74,
        crate::weather::Mode::Storm => 1.,
        _ => 0.,
    }
}

fn binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    textures: [&wgpu::TextureView; 3],
    samplers: [&wgpu::Sampler; 2],
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("original final-blit inputs"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(textures[0]),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(textures[1]),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(textures[2]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(samplers[0]),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(samplers[1]),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn display_preferences_gain_and_lens_are_independent() {
        let base = super::FisheyeParameters { warp: [0.2, 0.9, 0.3, 0.],
            vignette: [0., 0.25, 0., 0.], widescreen: true, brightness: 1., supersample: false };
        for postfx in [false, true] {
            for fisheye in [false, true] {
                let actual = base.for_display(skate2_core::ui::options::PcDisplaySettings {
                    postfx, fisheye, ..Default::default() });
                assert_eq!(actual.brightness, if postfx { 1. } else { 1.22 * 1.15 * 1.10 });
                assert_eq!(actual.warp, if fisheye { base.warp } else { [0., 1., 0., 0.] });
                assert_eq!(base.edge_scale(), 0.2f32.mul_add(0.0625, 0.9));
            }
        }
        assert_eq!(base.brightness, 1.); // retained source values never compound
        for (brightness, postfx, expected) in [(75, true, 0.75), (150, true, 1.5), (150, false, 1.22 * 1.15 * 1.10 * 1.5)] {
            let actual = base.for_display(skate2_core::ui::options::PcDisplaySettings { postfx, brightness, ..Default::default() });
            assert_eq!(actual.brightness, expected);
        }
    }
    /// Uses wgpu's production Naga validation and backend compilation for
    /// both native branches, without consuming the shared gameplay RNG.
    #[test]
    fn original_fisheye_and_noise_shaders_compile() {
        let instance = wgpu::Instance::default();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default()))
        else {
            eprintln!("no adapter; final-blit GPU shader check skipped");
            return;
        };
        let (device, _) = pollster::block_on(adapter.request_device(&Default::default()))
            .expect("final-blit test device");
        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("original final-blit validation"),
            source: wgpu::ShaderSource::Wgsl(include_str!("final_pass.wgsl").into()),
        });
        for entry_point in ["fs_standard", "fs_noise"] {
            let _pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some(entry_point),
                layout: None,
                vertex: wgpu::VertexState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    compilation_options: Default::default(),
                    module: &shader,
                    entry_point: Some(entry_point),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
            });
        }
        assert!(
            pollster::block_on(error_scope.pop()).is_none(),
            "original final-blit shader failed GPU validation"
        );
    }
}
