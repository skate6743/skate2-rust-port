//! PC port addition, Skate During Turn Wait (Online Settings): while another player's turn of an
//! online challenge shows, the local skater's own game camera draws into a corner at the left of
//! the screen, just above the trick display and over the HUD, so nothing of the watched player's
//! HUD covers it.
//!
//! The corner is a second view of the same frame, as stereoscopy's second eye is
//! (`stereoscopy::Eye`): one correction takes every clip-from-local the main view's producers
//! made to the corner camera's, so the map's selection and globals, the skaters, the DMOs, the
//! particles, the crowd and the world UI all follow it. It draws into targets of its own at the
//! corner's size with the main pass's pipelines (the surface format and the frame's samples),
//! then a small pass copies it into the corner. It shows the scene as `PostFX` Off does: no bloom,
//! grading or lens. The light, the fog's origin (`g_vViewPos`) and the shadow cascades stay the
//! main camera's, which the corner only nears when the two cameras are far apart.
use super::{RenderResult, stereoscopy};

/// The corner's width, a share of the window's; its height keeps the window's aspect.
const WIDTH_SHARE: f64 = 0.3;
/// Its distance from the window's left edge (and from the top or bottom edge where it would
/// reach one), a share of the window's height.
const MARGIN_SHARE: f64 = 0.03;
/// The top of the bottom-left trick display (`hud2/trickdisplay2`'s trick names, three of them
/// stacked as they animate), a share of the HUD's 16:9 height from its top: 642 of 1080 lines,
/// measured through the marker UI's own projection. The corner sits above it, so the watched
/// player's trick names, multiplier and line score stay clear; the notices that pop up over the
/// names (a gap's, a big trick's) go under the corner, which draws over the HUD.
const TRICK_DISPLAY_TOP: f64 = 0.594;
/// The gap between the corner and the trick display, a share of the window's height.
const GAP_SHARE: f64 = 0.01;
/// The frame around it, in pixels at a 1080-line window.
const BORDER_PIXELS: f64 = 3.0;

/// The local game camera the corner shows, for the window's aspect (`window_clip_from_world`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PipView {
    pub(crate) clip_from_world: [f32; 16],
    /// Near, then far.
    pub(crate) planes: [f32; 2],
}

/// The corner's image and frame within a window, in pixels from its top left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Layout {
    /// The image: x, y, width, height.
    pub(super) image: [u32; 4],
    /// The image with its frame around it.
    pub(super) outer: [u32; 4],
}

impl Layout {
    /// The corner of a `width` x `height` window; none in a window too small to hold one.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "pixel counts of a window"
    )]
    pub(super) fn of(width: u32, height: u32) -> Option<Self> {
        let (w, h) = (f64::from(width), f64::from(height));
        let image_width = (w * WIDTH_SHARE).round();
        let image_height = (image_width * h / w).round();
        let margin = (h * MARGIN_SHARE).round();
        let border = (BORDER_PIXELS * h / 1080.0).round().max(1.0);
        let outer_width = image_width + 2.0 * border;
        let outer_height = image_height + 2.0 * border;
        if image_width < 16.0
            || image_height < 16.0
            || margin + outer_width > w
            || margin + outer_height > h
        {
            return None;
        }
        // The HUD keeps a 16:9 stage: a window taller than that squeezes it toward the middle
        // (`marker_ui::project`), a wider one only narrows it.
        let squeeze = (w / h / (16.0 / 9.0)).min(1.0);
        let trick_display_top = h * 0.5 + (TRICK_DISPLAY_TOP - 0.5) * h * squeeze;
        let bottom = (trick_display_top - (h * GAP_SHARE).round()).round();
        let x = margin;
        let y = (bottom - outer_height).max(margin);
        let pixels = |value: f64| value as u32;
        Some(Self {
            image: [
                pixels(x + border),
                pixels(y + border),
                pixels(image_width),
                pixels(image_height),
            ],
            outer: [
                pixels(x),
                pixels(y),
                pixels(outer_width),
                pixels(outer_height),
            ],
        })
    }

    pub(super) const fn image_size(&self) -> (u32, u32) {
        (self.image[2], self.image[3])
    }
}

fn multiply(a: [f64; 16], b: [f64; 16]) -> [f64; 16] {
    std::array::from_fn(|i| (0..4).map(|k| a[k * 4 + i % 4] * b[i / 4 * 4 + k]).sum())
}

/// The second eye that draws `pip` where the main view draws `main` (both clip-from-world):
/// `clip_pip * inverse(clip_main)` takes any clip-from-local the main view made to the corner's.
pub(super) fn eye(main: [f32; 16], pip: &PipView) -> RenderResult<stereoscopy::Eye> {
    let correction = multiply(
        pip.clip_from_world.map(f64::from),
        stereoscopy::inverse(main.map(f64::from))?,
    );
    Ok(stereoscopy::Eye {
        correction,
        clip: pip.clip_from_world,
        field: 1,
        planes: pip.planes,
    })
}

/// The corner's scene targets: the main pass's format and samples at the corner's size.
pub(super) struct Targets {
    size: (u32, u32),
    samples: u32,
    format: wgpu::TextureFormat,
    /// The resolved image the corner shows.
    pub(super) colour: wgpu::TextureView,
    /// The multisampled image the scene draws into, resolved into `colour`; none at 1x.
    pub(super) msaa: Option<wgpu::TextureView>,
    pub(super) depth: wgpu::TextureView,
    binding: wgpu::BindGroup,
}

impl Targets {
    /// Whether these targets serve a corner of `size` drawn with `samples` into `format`.
    pub(super) fn fit(&self, size: (u32, u32), samples: u32, format: wgpu::TextureFormat) -> bool {
        self.size == size && self.samples == samples && self.format == format
    }
}

const SHADER: &str = r"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
// The image's x, y, width and height in the window, in pixels.
@group(0) @binding(2) var<uniform> rect: vec4<f32>;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(positions[i], 0.0, 1.0);
}
@fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (p.xy - rect.xy) / rect.zw;
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return vec4<f32>(0.02, 0.02, 0.02, 1.0);
    }
    return vec4<f32>(textureSampleLevel(image, image_sampler, uv, 0.0).rgb, 1.0);
}
";

/// The pass that copies the corner's image into the window, with its frame.
pub(super) struct Composite {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    rect: wgpu::Buffer,
    format: wgpu::TextureFormat,
}

impl Composite {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skate During Turn Wait corner"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skate During Turn Wait corner"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skate During Turn Wait corner"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Skate During Turn Wait corner"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: &shader,
                entry_point: Some("fs"),
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
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Skate During Turn Wait corner"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let rect = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Skate During Turn Wait corner rect"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            layout,
            sampler,
            rect,
            format,
        }
    }

    /// The corner's scene targets at `size`, drawn with `samples`.
    pub(super) fn targets(&self, device: &wgpu::Device, size: (u32, u32), samples: u32) -> Targets {
        let texture = |label: &str,
                       sample_count: u32,
                       format: wgpu::TextureFormat,
                       usage: wgpu::TextureUsages| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let colour = texture(
            "Skate During Turn Wait corner image",
            1,
            self.format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let msaa = (samples > 1).then(|| {
            texture(
                "Skate During Turn Wait corner samples",
                samples,
                self.format,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
        });
        let depth = texture(
            "Skate During Turn Wait corner depth",
            samples,
            super::DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skate During Turn Wait corner"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&colour),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.rect.as_entire_binding(),
                },
            ],
        });
        Targets {
            size,
            samples,
            format: self.format,
            colour,
            msaa,
            depth,
            binding,
        }
    }

    /// The corner's image and frame drawn over `view` at `layout`.
    #[expect(clippy::cast_precision_loss, reason = "pixel counts of a window")]
    pub(super) fn draw(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        targets: &Targets,
        layout: Layout,
    ) {
        let rect: Vec<u8> = layout
            .image
            .iter()
            .flat_map(|&value| (value as f32).to_ne_bytes())
            .collect();
        queue.write_buffer(&self.rect, 0, &rect);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("Skate During Turn Wait corner"),
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
        let [x, y, width, height] = layout.outer;
        pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);
        pass.set_scissor_rect(x, y, width, height);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &targets.binding, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_corner_sits_left_above_the_trick_display_at_the_window_aspect() {
        let layout = Layout::of(1920, 1080).unwrap();
        assert_eq!(layout.image, [35, 304, 576, 324]);
        assert_eq!(layout.outer, [32, 301, 582, 330]);
        // Its frame ends 11 lines above the trick names' top (642 of 1080).
        assert_eq!(layout.outer[1] + layout.outer[3], 642 - 11);
        let wide = Layout::of(2560, 1080).unwrap();
        assert_eq!(wide.image_size(), (768, 324));
        assert_eq!(wide.outer[1] + wide.outer[3], 631);
        let monitor = Layout::of(2560, 1440).unwrap();
        assert_eq!(monitor.outer[1] + monitor.outer[3], 841);
        // A 16:10 window squeezes the HUD toward the middle, and the trick display with it.
        let tall = Layout::of(1920, 1200).unwrap();
        assert_eq!(tall.outer[1] + tall.outer[3], 690);
        assert!(Layout::of(40, 30).is_none());
    }

    #[test]
    fn the_correction_takes_the_main_clip_to_the_corner_one() {
        let main: [f32; 16] = [
            1.2, 0.0, 0.0, 0.0, 0.0, 2.1, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, -30.0, 4.0, -900.0, -899.0,
        ];
        let pip = PipView {
            clip_from_world: [
                0.9, 0.1, 0.0, 0.0, 0.0, 1.6, 0.2, 0.2, 0.3, 0.0, 1.0, 1.0, 12.0, -3.0, -880.0,
                -879.5,
            ],
            planes: [0.1, 1000.0],
        };
        let eye = eye(main, &pip).unwrap();
        let corrected = eye.transform(main);
        for (got, want) in corrected.iter().zip(pip.clip_from_world) {
            assert!((got - want).abs() < 1e-3, "{corrected:?}");
        }
    }
}
