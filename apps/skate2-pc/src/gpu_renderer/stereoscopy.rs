//! Stereoscopy::SetPinch82838e30 and ViewCamera::SetPerspectiveMatrixRadians82840770.
//! Two real geometry views, composed by stereo_uberPostFX_PS8212e1a8.
use super::RenderResult;
use wgpu::util::DeviceExt;

pub(super) const IDENTITY: [f64; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];
#[derive(Clone, Copy)]
pub(super) struct Eye {
    pub correction: [f64; 16],
    pub clip: [f32; 16],
    pub field: u8,
    pub planes: [f32; 2],
}
fn multiply(a: [f64; 16], b: [f64; 16]) -> [f64; 16] {
    std::array::from_fn(|i| (0..4).map(|k| a[k * 4 + i % 4] * b[i / 4 * 4 + k]).sum())
}
pub(super) fn inverse(m: [f64; 16]) -> RenderResult<[f64; 16]> {
    let mut rows: [[f64; 8]; 4] = std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            if c < 4 {
                m[c * 4 + r]
            } else {
                if c - 4 == r { 1. } else { 0. }
            }
        })
    });
    for c in 0..4 {
        let p = (c..4)
            .max_by(|&a, &b| rows[a][c].abs().total_cmp(&rows[b][c].abs()))
            .unwrap();
        if rows[p][c] == 0. || !rows[p][c].is_finite() {
            return Err("stereo projection is singular".into());
        }
        rows.swap(c, p);
        let divisor = rows[c][c];
        for value in &mut rows[c] {
            *value /= divisor;
        }
        for r in 0..4 {
            if r != c {
                let n = rows[r][c];
                for k in 0..8 {
                    rows[r][k] -= n * rows[c][k];
                }
            }
        }
    }
    Ok(std::array::from_fn(|i| rows[i % 4][4 + i / 4]))
}
impl Eye {
    pub fn new(
        clip: [f32; 16],
        axes: [[f32; 4]; 3],
        camera: [f32; 3],
        planes: [f32; 2],
        field: u8,
    ) -> RenderResult<Self> {
        let mut camera_to_world = IDENTITY;
        for c in 0..3 {
            for r in 0..3 {
                camera_to_world[c * 4 + r] = f64::from(axes[c][r]);
            }
        }
        for r in 0..3 {
            camera_to_world[12 + r] = f64::from(camera[r]);
        }
        let projection = multiply(clip.map(f64::from), camera_to_world);
        // Native PINCH82feffd0=1 degree, EYE_DIST82feffd4 bits3d23d6ef.
        // SetPinch computes its trigonometry as float then stores double matrices.
        let angle = 1f32 * 0.017453292f32 * if field == 0 { -1. } else { 1. };
        let (s, c) = angle.sin_cos();
        let (s, c) = (f64::from(s), f64::from(c));
        let x = f64::from(f32::from_bits(0x3d23d6ef)) * if field == 0 { -1. } else { 1. };
        let eye = [c, 0., s, 0., 0., 1., 0., 0., -s, 0., c, 0., x, 0., 0., 1.];
        // P*E*P^-1 lets each existing clip-from-local producer keep its camera
        // rebase, skinning and world transforms; the eye projection is applied once.
        let correction = multiply(multiply(projection, eye), inverse(projection)?);
        let clip = multiply(correction, clip.map(f64::from)).map(|v| v as f32);
        Ok(Self {
            correction,
            clip,
            field,
            planes,
        })
    }
    pub fn transform(&self, matrix: [f32; 16]) -> [f32; 16] {
        multiply(self.correction, matrix.map(f64::from)).map(|v| v as f32)
    }
}

/// Projection of already generated world UI / spline vertices. Overlay UI uses
/// its separate unmodified vertex entrypoint and remains screen aligned.
pub(super) struct Correction {
    pub layout: wgpu::BindGroupLayout,
    pub binding: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}
impl Correction {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stereo eye projection"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bytes: Vec<_> = IDENTITY
            .into_iter()
            .flat_map(|v| (v as f32).to_ne_bytes())
            .collect();
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("stereo eye projection"),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stereo eye projection"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            layout,
            binding,
            uniform,
        }
    }
    pub fn update(&self, queue: &wgpu::Queue, eye: Option<&Eye>) {
        let bytes: Vec<_> = eye
            .map_or(IDENTITY, |e| e.correction)
            .into_iter()
            .flat_map(|v| (v as f32).to_ne_bytes())
            .collect();
        queue.write_buffer(&self.uniform, 0, &bytes);
    }
}
