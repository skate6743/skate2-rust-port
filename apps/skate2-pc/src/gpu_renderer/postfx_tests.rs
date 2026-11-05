use super::*;
use std::collections::BTreeMap;

#[test]
fn gpu_native_cube_channels_and_bloom_composition() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("PostFX GPU adapter");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("NIS grade depth"),
        size: wgpu::Extent3d { width: 64, height: 32, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24PlusStencil8,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth = depth.create_view(&wgpu::TextureViewDescriptor {
        aspect: wgpu::TextureAspect::DepthOnly,
        ..Default::default()
    });
    let stencil = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: 64, height: 32, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Stencil8, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[],
    }).create_view(&Default::default());
    let overlay = Target::new(&device,64,32,wgpu::TextureFormat::Rgba8Unorm);
    let mut post = PostFx::new(&device, wgpu::TextureFormat::Rgba8Unorm, 64, 32, &depth, &stencil, &overlay.view, false);
    if let Some(error) = pollster::block_on(error_scope.pop()) {
        panic!("{error}");
    }
    let mut error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let output = Target::new(&device, 64, 32, wgpu::TextureFormat::Rgba8Unorm);
    let mut neutral = Vec::new();
    for z in 0..32 {
        for y in 0..32 {
            for x in 0..32 {
                neutral.extend_from_slice(&[
                    ((31 - y) * 255 / 31) as u8,
                    (x * 255 / 31) as u8,
                    (z * 255 / 31) as u8,
                    255,
                ]);
            }
        }
    }
    let mut assets = Assets {
        cubes: BTreeMap::from([(DEFAULT_CUBE, neutral.into())]),
        blooms: BTreeMap::new(),
        exposures: BTreeMap::new(),
        location: [0; 3],
        matrix: math::colour_matrix(1., 0.5, 1., [0.; 3], [1.; 3]),
        hom: Default::default(),
        bloom_sample_offset: 0.,
        watermark_position: [0.;2],
        deck_ao: [0.; 4],
    };
    for (cinematic, bloom, expected) in [
        (false, [1., 0., 0., 0., 0.], [72i32, 144, 216]),
        (true, [1., 0., 0., 0., 0.], [72, 144, 216]),
        // Uniform input: extraction rounds to [5,36,88]; medium blur
        // gives [2,18,44], fine + medium gives [3,27,66]. The original
        // 1.41 composition then gives ~[75,171,255] before LUT byte error.
        // Positive bloom must brighten red/green and saturate blue only.
        (false, [0.02, 0., 0., 0.5, 0.25], [75, 171, 255]),
        (true, [0.02, 0., 0., 0.5, 0.25], [75, 171, 255]),
    ] {
        let frame = Frame {
            bloom,
            exposure: [0.25, 0.1, 2., 0.15],
            cube: DEFAULT_CUBE,
            dt: 1. / 60.,
            updated_at: std::time::Instant::now(),
            cinematic, camera_planes: None,
            final_mult: [1.; 4],
            keep_matrices: false, tazer: false, hom: None,
            // Default ICE values: both grades the identity, nothing rolled.
            ice: cinematic.then(|| crate::render_postfx::IceInputs {
                radial_blur: 0.,
                scanline: 0.,
                fisheye: 1.,
                matrices: [math::colour_matrix(1., 0.5, 1., [0.; 3], [1.; 3]); 2],
                lerp: [40., 150.],
                tonemap: 0.,
                planes: [0.2, 1000.],
            }),
            fe_online_menu: false,
            luminance_scale: 1.0,
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label: Some("known gamma-space scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    depth_slice: None,
                    view: post.scene_view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.4,
                            b: 0.6,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        post.record(&device, &queue, &mut encoder, &output.view, wgpu::TextureFormat::Rgba8Unorm, &assets, &frame, None, None)
            .unwrap();
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * 32,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            output.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(32),
                },
            },
            output.texture.size(),
        );
        queue.submit(Some(encoder.finish()));
        post.after_submit();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            panic!("{error}");
        }
        error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pixels = read.slice(..).get_mapped_range().unwrap();
        for pixel in pixels.chunks_exact(4) {
            for (actual, expected) in pixel[..3].iter().zip(expected) {
                assert!(
                    (*actual as i32 - expected).abs() <= 2,
                    "cinematic={cinematic}, bloom={bloom:?}: {pixel:?}"
                );
            }
        }
        drop(pixels);
        read.unmap();
    }
    // A retained scene is rendered multiple times between 60Hz updates.
    // Neither exposure feedback nor an in-progress LUT transition may consume
    // its presentation delta again on those extra redraws.
    assets.cubes.insert(1, assets.cubes[&DEFAULT_CUBE].clone());
    let frame = Frame {
        bloom: [1., 0., 0., 0., 0.],
        exposure: [0.25, 0.1, 2., 0.15],
        cube: 1,
        dt: 1. / 60.,
        updated_at: std::time::Instant::now(),
        cinematic: false, camera_planes: None,
        final_mult: [1.; 4],
        keep_matrices: false, tazer: false, hom: None,
        ice: None, fe_online_menu: false, luminance_scale: 1.0,
    };
    post.exposure = 1.;
    post.exposure_time = 0.;
    post.luminance = Some(0.);
    let mut first = None;
    for _ in 0..4 {
        let mut encoder = device.create_command_encoder(&Default::default());
        post.record(&device, &queue, &mut encoder, &output.view, wgpu::TextureFormat::Rgba8Unorm, &assets, &frame, None, None).unwrap();
        queue.submit(Some(encoder.finish()));
        post.after_submit();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let state = (post.exposure, post.cube_blend);
        if first.is_none() {
            assert_eq!(post.exposure, math::adapt(1., post.luminance.unwrap(), frame.exposure));
        }
        assert_eq!(*first.get_or_insert(state), state);
    }
    assert!((post.cube_blend - 1. / 360.).abs() < 1e-7);
    assert_ne!(post.exposure, 1.);
    assert!(pollster::block_on(error_scope.pop()).is_none());
}
