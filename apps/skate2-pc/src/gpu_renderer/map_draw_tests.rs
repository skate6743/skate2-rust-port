use super::{
    Bounds, BoundsBuilder, ClipVolume, Containment, DiagnosticFragment, EnvironmentEffect,
    FrameEnvironment, IDENTITY, INSTANCE_ATTRIBUTES, INSTANCE_STRIDE, InstanceMaterial,
    MESH_ATTRIBUTES, MapEnvironment, MapPipelines, ShadowCascades, pack_globals, union_of_islands,
};
use crate::gpu_renderer::{
    DEPTH_FORMAT, GpuResourceBuilder, GpuTexture, RasterState, create_texture_bindings, raster_state::CullMode,
    uniform_bytes,
};
use skate2_core::texture::NativeTexture;
use skate2_formats::{
    material::{LinearTextureLevel, PIXEL_FORMAT_A8R8G8B8, TextureEndian, TextureType},
    render::{
        DrawIndexedParameters, NativeMeshGeometry, NativeVertexAttribute,
        NativeVertexAttributeValues, PRIMITIVE_TYPE_TRIANGLE_LIST, VertexSemantic,
    },
};
use std::sync::Arc;

#[test]
fn combined_island_draws_preserve_exact_ordered_index_and_instance_stream() {
    let draws = vec![
        (0, 0..3, 10),
        (0, 3..9, 10),   // safe adjacent triangles
        (0, 12..15, 10), // culled gap must remain a gap
        (0, 15..18, 11), // different base vertex
        (1, 18..21, 11), // different instance/material
        (1, 21..24, 11),
        (1, 24..26, 11),
        (1, 26..29, 11), // incomplete list boundary
        (1, 29..32, 11),
    ];
    let merged: Vec<_> = super::island_runs(draws.clone().into_iter(), true).collect();
    assert_eq!(merged.len(), 6);
    assert_eq!(merged[0], (0, 0..9, 10, 2));
    assert_eq!(merged[3], (1, 18..24, 11, 2));
    let reference: Vec<_> = draws
        .iter()
        .flat_map(|(instance, range, base)| {
            range.clone().map(move |index| (*instance, index, *base))
        })
        .collect();
    let actual: Vec<_> = merged
        .iter()
        .flat_map(|(instance, range, base, _)| {
            range.clone().map(move |index| (*instance, index, *base))
        })
        .collect();
    assert_eq!(actual, reference);
    assert_eq!(merged.iter().map(|d| d.3).sum::<usize>(), draws.len());
    assert_eq!(super::island_runs(draws.into_iter(), false).count(), 9);
}

#[test]
fn draw_runs_cut_between_objects_and_keep_every_draw_in_order() {
    let visible: Vec<(u32, usize)> = [(0, 3), (1, 2), (2, 1), (3, 4), (4, 1), (5, 1), (6, 2)]
        .into_iter()
        .flat_map(|(object, islands)| (0..islands).map(move |island| (object, island)))
        .collect();
    for count in 1..=8 {
        let runs = super::draw_runs(&visible, count);
        assert!(!runs.is_empty() && runs.len() <= count);
        assert_eq!(runs.first().map(|run| run.start), Some(0));
        assert_eq!(runs.last().map(|run| run.end), Some(visible.len()));
        for pair in runs.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
            assert!(pair[0].start < pair[0].end);
            assert_ne!(visible[pair[0].end - 1].0, visible[pair[1].start].0);
        }
    }
    assert_eq!(super::draw_runs(&visible, 1), vec![0..visible.len()]);
    assert_eq!(super::draw_runs(&visible, 4).len(), 4);
    assert_eq!(super::draw_runs(&[], 4), vec![0..0]);
}

/// Right-handed camera at the origin looking down -Z, WebGPU 0..1 depth.
fn perspective() -> [f32; 16] {
    let (near, far) = (0.1_f32, 100.0_f32);
    let scale = 1.0 / 30.0_f32.to_radians().tan();
    [
        scale,
        0.0,
        0.0,
        0.0,
        0.0,
        scale,
        0.0,
        0.0,
        0.0,
        0.0,
        far / (near - far),
        -1.0,
        0.0,
        0.0,
        near * far / (near - far),
        0.0,
    ]
}

fn bounds(min: [f64; 3], max: [f64; 3]) -> Bounds {
    Bounds { min, max }
}

fn clip_point(clip: &[f32; 16], point: [f64; 3]) -> [f64; 4] {
    std::array::from_fn(|row| {
        point
            .iter()
            .enumerate()
            .map(|(column, value)| f64::from(clip[column * 4 + row]) * value)
            .sum::<f64>()
            + f64::from(clip[12 + row])
    })
}

fn rasterizer_keeps(clip: [f64; 4]) -> bool {
    let [clip_x, clip_y, clip_z, clip_w] = clip;
    -clip_w <= clip_x
        && clip_x <= clip_w
        && -clip_w <= clip_y
        && clip_y <= clip_w
        && 0.0 <= clip_z
        && clip_z <= clip_w
}

#[test]
fn classifies_inside_outside_and_straddling_boxes() {
    let volume = ClipVolume::new(&perspective());
    let ahead = bounds([-1.0, -1.0, -11.0], [1.0, 1.0, -9.0]);
    assert_eq!(volume.classify(&ahead), Containment::Inside);
    let behind = bounds([-1.0, -1.0, 5.0], [1.0, 1.0, 9.0]);
    assert_eq!(volume.classify(&behind), Containment::Outside);
    let beyond_far = bounds([-1.0, -1.0, -300.0], [1.0, 1.0, -200.0]);
    assert_eq!(volume.classify(&beyond_far), Containment::Outside);
    let far_left = bounds([-500.0, -1.0, -11.0], [-400.0, 1.0, -9.0]);
    assert_eq!(volume.classify(&far_left), Containment::Outside);
    let straddling_left = bounds([-100.0, -1.0, -11.0], [0.0, 1.0, -9.0]);
    assert_eq!(volume.classify(&straddling_left), Containment::Intersecting);
    let around_camera = bounds([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);
    assert_eq!(volume.classify(&around_camera), Containment::Intersecting);
}

#[test]
fn rejected_boxes_contain_no_point_the_rasterizer_keeps() {
    let clip = perspective();
    let volume = ClipVolume::new(&clip);
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        f64::from(u32::try_from(state >> 40).expect("24 random bits")) / f64::from(1_u32 << 24)
    };
    let mut outside = 0;
    for _ in 0..3000 {
        let center = [
            next() * 300.0 - 150.0,
            next() * 300.0 - 150.0,
            next() * 300.0 - 150.0,
        ];
        let half = [next() * 40.0, next() * 40.0, next() * 40.0];
        let tested = bounds(
            std::array::from_fn(|axis| center[axis] - half[axis]),
            std::array::from_fn(|axis| center[axis] + half[axis]),
        );
        let containment = volume.classify(&tested);
        outside += usize::from(containment == Containment::Outside);
        for step in 0..216 {
            let fraction = |digit: u32| f64::from(digit) / 5.0;
            let (i, j, k) = (step % 6, step / 6 % 6, step / 36);
            let point = [
                tested.min[0] + (tested.max[0] - tested.min[0]) * fraction(i),
                tested.min[1] + (tested.max[1] - tested.min[1]) * fraction(j),
                tested.min[2] + (tested.max[2] - tested.min[2]) * fraction(k),
            ];
            let kept = rasterizer_keeps(clip_point(&clip, point));
            if kept {
                assert_ne!(
                    containment,
                    Containment::Outside,
                    "{tested:?} culled {point:?}"
                );
            } else {
                assert_ne!(
                    containment,
                    Containment::Inside,
                    "{tested:?} lost {point:?}"
                );
            }
        }
    }
    assert!(
        outside > 1000,
        "the sample must exercise rejection: {outside}"
    );
}

#[test]
fn non_finite_planes_never_reject() {
    let behind = bounds([-1.0, -1.0, 5.0], [1.0, 1.0, 9.0]);
    let volume = ClipVolume::new(&[f32::NAN; 16]);
    assert_ne!(volume.classify(&behind), Containment::Outside);
    let unbounded = bounds([f64::NEG_INFINITY; 3], [f64::INFINITY; 3]);
    assert_ne!(
        ClipVolume::new(&perspective()).classify(&unbounded),
        Containment::Outside
    );
}

#[test]
fn affine_transform_bounds_every_transformed_corner() {
    let source = bounds([-1.0, -2.0, -3.0], [4.0, 5.0, 6.0]);
    // Column-major: 90 degrees about Y, then translate by (10, 20, 30).
    let matrix = [
        0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 10.0, 20.0, 30.0, 1.0,
    ];
    let world = source.transformed(&matrix).expect("affine");
    for corner in 0..8 {
        let local = [
            if corner & 1 == 0 {
                source.min[0]
            } else {
                source.max[0]
            },
            if corner & 2 == 0 {
                source.min[1]
            } else {
                source.max[1]
            },
            if corner & 4 == 0 {
                source.min[2]
            } else {
                source.max[2]
            },
        ];
        let transformed = clip_point(&matrix, local);
        for (axis, value) in transformed.into_iter().take(3).enumerate() {
            assert!(world.min[axis] <= value && value <= world.max[axis]);
        }
    }
    assert!((world.min[0] - 7.0).abs() < 1e-3 && (world.max[0] - 16.0).abs() < 1e-3);
    assert!((world.min[2] - 26.0).abs() < 1e-3 && (world.max[2] - 31.0).abs() < 1e-3);
}

#[test]
fn projective_or_non_finite_transforms_are_never_culled() {
    let source = bounds([0.0; 3], [1.0; 3]);
    let mut projective = [0.0; 16];
    for index in [0, 5, 10, 15] {
        projective[index] = 1.0;
    }
    projective[11] = -1.0;
    assert!(source.transformed(&projective).is_none());
    let mut affine = projective;
    affine[11] = 0.0;
    assert!(source.transformed(&affine).is_some());
    affine[12] = f32::INFINITY;
    assert!(source.transformed(&affine).is_none());
}

#[test]
fn island_bounds_reject_empty_and_non_finite_positions() {
    let empty = BoundsBuilder::default();
    assert!(empty.is_empty());
    assert!(empty.finish().is_none());
    let mut non_finite = BoundsBuilder::default();
    non_finite.include([0.0, f32::NAN, 0.0]);
    assert!(!non_finite.is_empty());
    assert!(non_finite.finish().is_none());
    let mut island = BoundsBuilder::default();
    island.include([1.0, -2.0, 3.0]);
    island.include([-4.0, 5.0, 0.5]);
    assert_eq!(
        island.finish(),
        Some(bounds([-4.0, -2.0, 0.5], [1.0, 5.0, 3.0]))
    );
}

#[test]
fn mesh_bounds_are_the_island_union_unless_one_is_unbounded() {
    let islands = [
        Some(bounds([0.0; 3], [1.0; 3])),
        None,
        Some(bounds([-2.0, 0.5, 0.5], [0.5, 3.0, 0.75])),
    ];
    assert_eq!(
        union_of_islands(&islands, true),
        Some(bounds([-2.0, 0.0, 0.0], [1.0, 3.0, 1.0]))
    );
    assert_eq!(union_of_islands(&islands, false), None);
}

fn lanes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|word| f32::from_ne_bytes(word.try_into().expect("f32 lane")))
        .collect()
}

#[test]
fn instance_records_match_the_uniform_byte_layout() {
    let record =
        uniform_bytes(&[0.0; 16], &[0.0; 4]).len() + InstanceMaterial::default().bytes().len();
    assert_eq!(u64::try_from(record).expect("record size"), INSTANCE_STRIDE);
    for (index, attribute) in INSTANCE_ATTRIBUTES.iter().enumerate() {
        assert_eq!(attribute.offset, 16 * u64::try_from(index).expect("index"));
        assert_eq!(attribute.format, wgpu::VertexFormat::Float32x4);
    }
    assert_eq!(MESH_ATTRIBUTES[1].offset, 12);
}

#[test]
fn material_lanes_follow_world_and_color_in_shader_order() {
    let lane = |first: u8| {
        std::array::from_fn(|index| f32::from(first + u8::try_from(index).expect("lane")))
    };
    let material = InstanceMaterial {
        lightmap_dot: lane(1),
        macro_overlay_uv_scale: lane(5),
        detail_normal_uv_scale: lane(9),
        macro_overlay_opacity: lane(13),
        m_params: lane(17),
    };
    assert_eq!(
        lanes(&material.bytes()),
        (1_u8..=20).map(f32::from).collect::<Vec<_>>()
    );
}

#[test]
fn globals_pack_in_wgsl_struct_order() {
    let clip = std::array::from_fn(|index| f32::from(u8::try_from(index).expect("lane")));
    let view = [16.0, 17.0, 18.0, 0.0];
    let lighting = [
        [20.0, 21.0, 22.0, 23.0],
        [24.0, 25.0, 26.0, 27.0],
        [28.0, 29.0, 30.0, 31.0],
        [32.0, 33.0, 34.0, 35.0],
    ];
    let shadows = [
        [36.0, 37.0, 38.0, 39.0],
        [40.0, 41.0, 42.0, 43.0],
        [44.0, 45.0, 46.0, 47.0],
        [48.0, 49.0, 50.0, 51.0],
    ];
    let sky = [[52.0, 53.0, 54.0, 55.0], [56.0, 57.0, 58.0, 59.0]];
    let packed = lanes(&pack_globals(&clip, &view, &lighting, &shadows, &sky));
    assert_eq!(packed.len(), 60);
    assert_eq!(&packed[..16], &clip);
    assert_eq!(&packed[16..20], &view);
    assert_eq!(
        &packed[20..36],
        lighting
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
            .as_slice()
    );
    assert_eq!(
        &packed[36..52],
        shadows
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
            .as_slice()
    );
    assert_eq!(
        &packed[52..],
        sky.iter().flatten().copied().collect::<Vec<_>>().as_slice()
    );
}

#[test]
fn sky_instance_lanes_carry_both_params_and_the_blend_flag() {
    let params = [[0.75, 1.6, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0]];
    let blended = lanes(&InstanceMaterial::sky(params, true).bytes());
    assert_eq!(blended[0].to_bits(), 1.0_f32.to_bits());
    assert_eq!(&blended[4..8], &params[1]);
    assert_eq!(&blended[16..20], &params[0]);
    let own = lanes(&InstanceMaterial::sky(params, false).bytes());
    assert_eq!(own[0].to_bits(), 0.0_f32.to_bits());
}

fn texel_texture(texture_type: TextureType, faces: u32) -> NativeTexture {
    NativeTexture {
        dictionary_index: 0,
        guid: 1,
        name: "validation texel".to_owned(),
        texture_type,
        endian: TextureEndian::None,
        width: 1,
        height: 1,
        depth: 1,
        mip_levels: 1,
        pixel_format: PIXEL_FORMAT_A8R8G8B8,
        packed_mips: false,
        mip_data_offset: 0,
        encoded_data: Vec::new(),
        linear_levels: (0..faces)
            .map(|face| LinearTextureLevel {
                level: 0,
                face,
                width: 1,
                height: 1,
                width_in_blocks: 1,
                height_in_blocks: 1,
                bytes_per_block: 4,
                row_pitch: 4,
                data: vec![128; 4],
            })
            .collect(),
    }
}

fn lit_triangle() -> NativeMeshGeometry {
    let attribute = |semantic, values| NativeVertexAttribute {
        semantic,
        original_format: 0,
        values,
    };
    NativeMeshGeometry {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        attributes: vec![
            // decalenvironment `uv_decal`: surface UV in xy, decal UV in zw.
            attribute(
                VertexSemantic::TextureCoordinate(0),
                NativeVertexAttributeValues::Float4(vec![[0.0, 0.0, 0.5, 0.5]; 3]),
            ),
            attribute(
                VertexSemantic::TextureCoordinate(1),
                NativeVertexAttributeValues::Float4(vec![[0.5, -0.0, 0.0, 0.0]; 3]),
            ),
            attribute(
                VertexSemantic::Tangent(0),
                NativeVertexAttributeValues::Float3(vec![[1.0, 0.0, 0.0]; 3]),
            ),
        ],
        indices: vec![0, 1, 2],
        draws: vec![DrawIndexedParameters {
            primitive_type: PRIMITIVE_TYPE_TRIANGLE_LIST,
            base_vertex_index: 0,
            start_index: 0,
            index_count: 3,
        }],
        position_format: 0,
    }
}

#[test]
fn absent_world_uv_uses_native_fetch_defaults_without_masking_broken_streams() {
    let mut mesh = lit_triangle();
    mesh.attributes
        .retain(|a| a.semantic != VertexSemantic::TextureCoordinate(1));
    let stream = super::super::lighting_stream(&mesh, true).unwrap().unwrap();
    for vertex in stream.bytes.chunks_exact(36) {
        assert_eq!(&lanes(vertex)[..4], &[0., 0., 0., 1.]);
    }
    assert!(
        super::super::lighting_stream(&mesh, false)
            .unwrap()
            .is_none()
    );
    mesh.attributes.push(NativeVertexAttribute {
        semantic: VertexSemantic::TextureCoordinate(1),
        original_format: 0,
        values: NativeVertexAttributeValues::Float4(vec![[0.; 4]; 2]),
    });
    assert!(
        super::super::lighting_stream(&mesh, true)
            .unwrap()
            .is_none()
    );
    let authored = super::super::lighting_stream(&lit_triangle(), true)
        .unwrap()
        .unwrap();
    assert_eq!(lanes(&authored.bytes)[1].to_bits(), (-0.0_f32).to_bits());
}

/// Builds every map pipeline, a cube texture, the environment resources and
/// one material per ported effect (sky included) plus a culled, depth-biased
/// alpha-tested diagnostic pipeline, then records and submits one frame that
/// switches diagnostic -> original pipelines, all inside a validation error
/// scope. Skipped when no adapter/device is available.
#[test]
fn ported_effects_validate_and_draw_on_an_available_adapter() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        eprintln!("no wgpu adapter; GPU validation skipped");
        return;
    };
    let descriptor = wgpu::DeviceDescriptor {
        experimental_features: Default::default(),
        memory_hints: Default::default(),
        trace: Default::default(),
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&descriptor)) else {
        eprintln!("no wgpu device; GPU validation skipped");
        return;
    };
    let (device, queue) = (Arc::new(device), Arc::new(queue));
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let (texture_layout, sampler) = create_texture_bindings(&device);
    let pipelines = Arc::new(MapPipelines::new(&device, format, 1, &texture_layout));
    let builder = GpuResourceBuilder {
        mesh_arena: Arc::new(std::sync::Mutex::new(Default::default())),
        device: Arc::clone(&device),
        queue: Arc::clone(&queue),
        object_layout: Arc::new(device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[],
            },
        )),
        texture_layout: Arc::new(texture_layout),
        sampler: Arc::new(sampler),
        map_pipelines: Arc::clone(&pipelines),
        shadow_atlas: Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("CSM atlas stand-in"),
                    size: wgpu::Extent3d {
                        width: 2,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rg16Float,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        ),
        main_target: GpuTexture {
            view: Arc::new(
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("main target stand-in"),
                        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            ),
            binding: None,
            cube: false,
        },
    };
    let first_mesh = builder
        .upload_mesh(&lit_triangle())
        .expect("first lit mesh");
    let mut relative = lit_triangle();
    for index in &mut relative.indices {
        *index += 1;
    }
    relative.draws[0].base_vertex_index = -1;
    let mesh = builder
        .upload_mesh(&relative)
        .expect("lit mesh with signed base");
    assert!(Arc::ptr_eq(&first_mesh.vertices, &mesh.vertices));
    assert!(Arc::ptr_eq(&first_mesh.indices, &mesh.indices));
    assert_eq!(mesh.draws[0].0.start, first_mesh.draws[0].0.start + 3);
    assert_eq!(mesh.draws[0].1, first_mesh.draws[0].1 + 2);
    // Character creation replaces positions with skinned vertices. It must
    // retain local draw offsets despite earlier shared map allocations.
    let dedicated = builder
        .upload_mesh_dedicated(&relative)
        .expect("character admission");
    assert!(!Arc::ptr_eq(&dedicated.vertices, &mesh.vertices));
    assert_eq!(dedicated.draws[0].0.start, relative.draws[0].start_index);
    assert_eq!(dedicated.draws[0].1, -1);
    let flat = builder
        .upload_native_texture(&texel_texture(TextureType::TwoDimensional, 1))
        .expect("2D texel");
    let cube = builder
        .upload_native_texture(&texel_texture(TextureType::Cube, 6))
        .expect("cube texel");
    assert!(
        builder
            .create_environment_material(EnvironmentEffect::Default, &[&cube, &flat, &flat])
            .is_err()
    );
    let mut map = builder.map_builder(10);
    map.push_solid(&mesh, None, [1.0; 4], None);
    map.push_textured(&mesh, None, &flat, None)
        .expect("diagnostic diffuse");
    let biased = builder.create_diagnostic_pipeline(
        DiagnosticFragment::DiffuseAlpha(16),
        RasterState {
            cull: CullMode::Front,
            depth_bias: -0.0001,
            slope_scale: 1.0,
        },
    );
    map.push_textured(&mesh, None, &flat, Some(&biased))
        .expect("culled, biased, alpha-tested diagnostic diffuse");
    let mut spotlight_geometry = lit_triangle();
    spotlight_geometry
        .attributes
        .retain(|a| a.semantic != VertexSemantic::TextureCoordinate(1));
    for attribute in &mut spotlight_geometry.attributes {
        if attribute.semantic == VertexSemantic::Tangent(0) {
            attribute.semantic = VertexSemantic::Normal(0);
        }
    }
    let spotlight_mesh = builder
        .upload_mesh(&spotlight_geometry)
        .expect("native normal-only mesh");
    assert!(spotlight_mesh.has_normal);
    assert!(!spotlight_mesh.has_tangent);
    assert_eq!(spotlight_mesh.lightmap_components, 0);
    let mut no_lightmap_geometry = lit_triangle();
    no_lightmap_geometry
        .attributes
        .retain(|a| a.semantic != VertexSemantic::TextureCoordinate(1));
    let no_lightmap_mesh = builder
        .upload_world_mesh(&no_lightmap_geometry)
        .expect("native missing UV default");
    let effects = [
        EnvironmentEffect::GodraySpotlight,
        EnvironmentEffect::Default,
        EnvironmentEffect::AlphaTest,
        EnvironmentEffect::TransparentDefault,
        EnvironmentEffect::Advertisement,
        EnvironmentEffect::Water,
        EnvironmentEffect::FlowingWater,
        EnvironmentEffect::Ocean,
        EnvironmentEffect::TrafficLightOne,
        EnvironmentEffect::TrafficLightTwo,
        EnvironmentEffect::Glare,
        EnvironmentEffect::Tree,
        EnvironmentEffect::TreeAnimate,
        EnvironmentEffect::VertexAnimate,
        EnvironmentEffect::ScrollIncandescent,
        EnvironmentEffect::BaseIncandescent,
        EnvironmentEffect::TransparentIncandescent,
        EnvironmentEffect::ZBiasIncandescent,
        EnvironmentEffect::ZBiasIncandescentBacklit,
        EnvironmentEffect::Base,
        EnvironmentEffect::BaseTerrain,
        EnvironmentEffect::BuildingCeilingWorld,
        EnvironmentEffect::DecalTerrain,
        EnvironmentEffect::Decal,
        EnvironmentEffect::Transparent,
        EnvironmentEffect::DecalZBias,
        EnvironmentEffect::ProxyWorld,
    ];
    for effect in effects {
        let textures = effect
            .channels()
            .iter()
            .map(|&channel| {
                if channel == "environment" {
                    &cube
                } else {
                    &flat
                }
            })
            .collect::<Vec<_>>();
        let material = builder
            .create_environment_material(effect, &textures)
            .expect("material");
        let selected_mesh = if effect == EnvironmentEffect::GodraySpotlight {
            &spotlight_mesh
        } else if matches!(
            effect,
            EnvironmentEffect::BaseIncandescent | EnvironmentEffect::Transparent
        ) {
            &no_lightmap_mesh
        } else {
            &mesh
        };
        map.push_environment(selected_mesh, effect, material, InstanceMaterial::default())
            .expect("original effect draw");
    }
    let sky_material = builder
        .create_environment_material(EnvironmentEffect::Sky, &[&flat, &flat, &flat])
        .expect("sky material");
    map.push_sky(
        &mesh,
        sky_material,
        InstanceMaterial::sky([[0.75, 1.6, 0.0, 0.0], [1.0; 4]], true),
    )
    .expect("sky draw");
    let resources = builder
        .create_environment_resources(&[255; 1024])
        .expect("environment resources");
    let mut map = map.finish(
        IDENTITY,
        Some(MapEnvironment {
            resources,
            envattributes2: [1.0, 0.0, 12.0, 0.1],
        }),
    );
    map.set_view(
        IDENTITY,
        [0.0, 0.0, -1.0],
        FrameEnvironment {
            exposure: Some(1.0),
            fog_k1: [0.004, -0.2, 1.4, 0.0],
            fog_colour: [0.0, 0.0, 0.0, -1.0],
            sky_params: [[0.75, 1.6, 0.0, 0.0], [1.0; 4]],
            light_direction: [0.0, 0.0, 1.0, 0.0],
            view_dot_light: [1.0, 0.1, 0.2, 0.0],
            animation_time: [0.25, 0., 0., 0.],
            pca: crate::render_water_animation::WaterAnimation::default().globals(),
            traffic_lights: [[2., 0., 0., 0.], [0., 0., 2., 2.]],
            camera_axes: [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]],
            shadows: ShadowCascades::default(),
            world_shadow: None,
            scene_cull: None,
        },
    );
    let (camera, camera_binding) = pipelines.create_camera(&device);
    queue.write_buffer(&camera, 0, &map.globals_bytes());
    let target = |format| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    let color = target(format);
    let depth = target(DEPTH_FORMAT);
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    // The selection is made once per view and recorded straight into a pass.
    let visible = map.visible().to_vec();
    assert_eq!(visible, map.select_visible());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: &color,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        let stats = map
            .draw_source()
            .record_draws(&mut pass, &camera_binding, &visible);
        // Three diagnostics, the ported effects and sky.
        assert_eq!(stats.drawn_objects, effects.len() + 4);
    }
    // A redraw keeps exactly the same ordered selection. Fog changes do not
    // invalidate geometry; changed camera/projection inputs do.
    map.world_shadow_draws().unwrap();
    let mut frame = map.frame;
    frame.fog_colour = [0.25; 4];
    map.set_view(IDENTITY, [0., 0., -1.], frame);
    assert_eq!(map.visible.get(), Some(&visible));
    assert!(map.world_shadow.get().is_some());
    let mut moved = IDENTITY;
    moved[12] = 100.;
    map.set_view(moved, [100., 0., -1.], frame);
    assert!(map.visible.get().is_none());
    assert_eq!(map.visible(), map.select_visible().as_slice());
    assert!(map.world_shadow.get().is_some()); // Its projection is still None.
    frame.world_shadow = Some(crate::render_shadows::WorldShadowProjection {
        preliminary: [[0.; 4]; 3],
        settings: skate2_core::presentation_shadow_culling::WorldShadowCullSettings {
            minimum_pixels: 1.,
            extra_radius: 0.,
        },
    });
    map.set_view(moved, [100., 0., -1.], frame);
    assert!(map.world_shadow.get().is_none());
    // Shadow command reuse ignores mutable uniform contents, but must retain
    // the exact ordered island list and pipeline/binding identity.
    let caster = super::world_caster_pipeline(
        &device,
        pipelines.camera_layout(),
        wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        },
    );
    let mut selected = super::WorldShadowDraws {
        rows: [[0.; 4]; 3],
        draws: vec![(0, 0), (1, 0)],
    };
    let casters =
        map.prepare_world_shadow(&device, &camera_binding, &caster, DEPTH_FORMAT, &selected);
    selected.rows[0][0] = 1.;
    assert!(Arc::ptr_eq(
        &casters,
        &map.prepare_world_shadow(&device, &camera_binding, &caster, DEPTH_FORMAT, &selected)
    ));
    selected.draws.reverse();
    assert!(!Arc::ptr_eq(
        &casters,
        &map.prepare_world_shadow(&device, &camera_binding, &caster, DEPTH_FORMAT, &selected)
    ));
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            multiview_mask: None,
            label: Some("cached world shadow validation"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_stencil_reference(255);
        pass.execute_bundles(std::iter::once(casters.as_ref()));
    }
    queue.submit(Some(encoder.finish()));
    let error = pollster::block_on(error_scope.pop());
    assert!(error.is_none(), "{error:?}");
}
