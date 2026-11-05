//! PC weather sky, using the CC0 photographs credited in rain_sky/CREDITS.txt.
//! Two 4K panoramas replace flat noise; this is an authored mod, not retail parity.
use std::sync::OnceLock;

pub(super) struct Sky {
    pub texture: wgpu::Texture,
    uploaded: OnceLock<()>,
}

impl Sky {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            texture: device.create_texture(&wgpu::TextureDescriptor {
                label: Some("CC0 photographed weather skies"),
                size: wgpu::Extent3d { width: 4096, height: 1024, depth_or_array_layers: 2 },
                mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }),
            uploaded: OnceLock::new(),
        }
    }

    // Queue writes precede the first rain pass. Decode once per target creation,
    // then release CPU pixel memory. No image file, network or per-frame upload.
    pub fn upload(&self, queue: &wgpu::Queue) {
        self.uploaded.get_or_init(|| {
            let sources: [&[u8]; 2] = [
                include_bytes!("rain_sky/approaching_storm.jpg"),
                include_bytes!("rain_sky/quarry_cloudy.jpg"),
            ];
            for (layer, bytes) in sources.into_iter().enumerate() {
                let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
                let rgb = decoder.decode().expect("embedded CC0 cloud JPEG");
                let info = decoder.info().expect("embedded cloud dimensions");
                assert_eq!((info.width, info.height), (4096, 1024));
                assert_eq!(info.pixel_format, jpeg_decoder::PixelFormat::RGB24);
                let pixels: Vec<u8> = rgb.chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255]).collect();
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.texture, mip_level: 0,
                        origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &pixels,
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4096 * 4), rows_per_image: Some(1024) },
                    wgpu::Extent3d { width: 4096, height: 1024, depth_or_array_layers: 1 },
                );
            }
        });
    }
}
