use skate_data::skate_map::SkateMap;

#[test]
fn presentation_loading_does_not_relax_playable_map_validation() {
    let mut data = fixture(1);
    let counts = 8 + 4 + 4 + "Fixture".len() + (4 + 12) * 4;
    data[counts + 16..counts + 24].fill(0); // No collision or rails.
    let needle: Vec<u8> = [0u32, 1, 2]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    let indices = data.windows(12).position(|w| w == needle).unwrap();
    data.truncate(indices + 12);
    assert!(
        SkateMap::parse(&data)
            .unwrap_err()
            .contains("requires triangle collision")
    );
    let render = SkateMap::parse_render_only(&data).unwrap();
    assert_eq!(render.geometry.indices, [0, 1, 2]);
    assert!(render.geometry.collision.is_empty());
    data[indices..indices + 4].copy_from_slice(&99u32.to_le_bytes());
    assert!(
        SkateMap::parse_render_only(&data)
            .unwrap_err()
            .contains("indices")
    );
    assert!(
        SkateMap::parse_render_only(&fixture(1))
            .unwrap_err()
            .contains("non-presentation")
    );
}

fn u(b: &mut Vec<u8>, value: u32) {
    b.extend(value.to_le_bytes());
}
fn f(b: &mut Vec<u8>, value: f32) {
    u(b, value.to_bits());
}
fn s(b: &mut Vec<u8>, value: &str) {
    u(b, value.len() as u32);
    b.extend(value.as_bytes());
}
fn fs(b: &mut Vec<u8>, values: &[f32]) {
    for &v in values {
        f(b, v);
    }
}
fn fixture(version: u8) -> Vec<u8> {
    let mut b = format!("SKATE{version:02}\0").into_bytes();
    u(&mut b, 0x12345678);
    s(&mut b, "Fixture");
    fs(&mut b, &[2., 0., 3., 1.]);
    fs(&mut b, &[0.; 9]);
    fs(&mut b, &[0., 12., 0.]);
    if version >= 3 {
        fs(&mut b, &[17., 0.]);
    }
    if version >= 6 {
        fs(&mut b, &[0.; 31]);
    }
    for n in [1, 1, 3, 3, 1, 1] {
        u(&mut b, n);
    }
    if version >= 4 {
        u(&mut b, 0);
    }
    if version >= 7 {
        u(&mut b, 1);
    }
    if version >= 8 {
        u(&mut b, 1);
    }
    let material_start = b.len();
    s(&mut b, "Surface");
    u(&mut b, 1);
    fs(&mut b, &[0.6, 0.1, 1., 0.5, 0.25, 0.8, 0.]);
    u(&mut b, 1);
    u(&mut b, 0);
    f(&mut b, 1.);
    if version >= 2 {
        for n in [0, 0, 0, 1] {
            u(&mut b, n);
        }
        f(&mut b, 0.4);
        for n in [42, 4, 7] {
            u(&mut b, n);
        }
    }
    if version >= 13 {
        u(&mut b, 1);
    }
    if version >= 12 {
        u(&mut b, 0);
    }
    if version >= 15 {
        let material = b.split_off(material_start);
        u(&mut b, material.len() as u32);
        stored(&mut b, &material, 2);
    }
    s(&mut b, "Pixel");
    for n in [1, 1, 1] {
        u(&mut b, n);
    }
    if version >= 9 {
        stored(&mut b, &[255, 128, 64, 255], version % 3);
    } else {
        u(&mut b, 4);
        b.extend([255, 128, 64, 255]);
    }
    let points = [[0., 0., 0.], [0., 0., 1.], [1., 0., 0.]];
    let mut vertices = Vec::new();
    for p in points {
        fs(&mut vertices, &p);
        fs(&mut vertices, &[0., 1., 0., 0., 0., 0., 0.]);
        u(&mut vertices, 1);
        if version >= 12 {
            fs(&mut vertices, &[0., 0.]);
            vertices.extend([0, 0, 127, 127]);
        }
    }
    let mut indices = Vec::new();
    for n in [0, 1, 2] {
        u(&mut indices, n);
    }
    let mut collision = Vec::new();
    for p in points {
        fs(&mut collision, &p);
    }
    u(&mut collision, 99);
    u(&mut collision, 1);
    if version >= 11 {
        collision.extend([26, 26, 26, 0]);
    }
    for bytes in [vertices, indices, collision] {
        if version >= 9 {
            stored(&mut b, &bytes, version % 3);
        } else {
            b.extend(bytes);
        }
    }
    s(&mut b, "Rail");
    u(&mut b, 0);
    if version >= 10 {
        u(&mut b, 0);
    }
    u(&mut b, 2);
    fs(&mut b, &[0., 1., 0., 2., 1., 0.]);
    if version >= 7 {
        s(&mut b, "Spot");
        u(&mut b, 1);
        fs(
            &mut b,
            &[
                0., 2., 0., 0., -1., 0., 1., 0.8, 0.5, 100., 10., 0.1, 0.95, 0.8,
            ],
        );
    }
    if version >= 8 {
        s(&mut b, "Route");
        u(&mut b, 1);
        u(&mut b, 2);
        fs(&mut b, &[4., 3.]);
        u(&mut b, 2);
        fs(&mut b, &[0., 0., 0., 3., 0., 0.]);
    }
    if version >= 12 {
        u(&mut b, 1);
        b.extend(b"WMET");
        u(&mut b, 1);
        u(&mut b, 2);
        stored(&mut b, b"{}", version % 3);
    }
    b
}
fn stored(b: &mut Vec<u8>, data: &[u8], method: u8) {
    use std::io::Write;
    let bytes = match method {
        0 => data.to_vec(),
        1 => {
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        }
        2 => zstd::stream::encode_all(data, 1).unwrap(),
        _ => unreachable!(),
    };
    u(b, method as u32);
    u(b, bytes.len() as u32);
    b.extend(bytes);
}
#[test]
fn reads_all_documented_versions_without_moving_geometry() {
    for version in 1..=15 {
        let map = SkateMap::parse(&fixture(version)).unwrap();
        assert_eq!(map.version, version);
        assert_eq!(map.spawn, [2., 0., 3.]);
        assert_eq!(map.heading, 1.);
        assert_eq!(map.geometry.vertices[2].position, [1., 0., 0.]);
        assert_eq!(map.geometry.collision[0].surface, 99);
        assert_eq!(map.textures[0].rgba, [255, 128, 64, 255]);
        assert_eq!(map.materials[0].audio, if version == 1 { 3 } else { 42 });
        assert_eq!(map.materials[0].physics, if version == 1 { 1 } else { 4 });
        assert_eq!(map.rails[0].points.len(), 2);
        assert_eq!(map.lights.len(), usize::from(version >= 7));
        assert_eq!(map.routes.len(), usize::from(version >= 8));
        if version >= 12 {
            assert_eq!(map.extensions[0].payload, b"{}");
            assert_eq!(
                map.geometry.vertices[0].tangent_frame,
                Some([0, 0, 127, 127])
            );
        }
    }
}

#[test]
fn v15_texture_references_preserve_order_and_reject_forward_or_wrong_size() {
    let mut data = fixture(15);
    let header_counts = 8 + 4 + 4 + "Fixture".len() + 49 * 4;
    data[header_counts + 4..header_counts + 8].copy_from_slice(&3u32.to_le_bytes());
    let mut needle = Vec::new();
    s(&mut needle, "Pixel");
    let start = data
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap();
    let metadata_end = start + needle.len() + 12;
    let texture_end = metadata_end + 8 + 4; // fixture v15 uses raw RGBA
    let mut references = Vec::new();
    for source in [0, 1] {
        references.extend_from_slice(&data[start..metadata_end]);
        u(&mut references, 11);
        u(&mut references, 4);
        u(&mut references, source);
    }
    data.splice(texture_end..texture_end, references);
    let map = SkateMap::parse(&data).unwrap();
    assert_eq!(map.textures.len(), 3);
    assert!(map.textures.iter().all(|t| t.rgba == [255, 128, 64, 255]));
    let reference = texture_end + (metadata_end - start) + 8;
    data[reference..reference + 4].copy_from_slice(&1u32.to_le_bytes());
    assert!(
        SkateMap::parse(&data)
            .unwrap_err()
            .contains("forward texture reference")
    );
    data[reference..reference + 4].copy_from_slice(&0u32.to_le_bytes());
    let width = texture_end + needle.len();
    data[width..width + 4].copy_from_slice(&2u32.to_le_bytes());
    assert!(
        SkateMap::parse(&data)
            .unwrap_err()
            .contains("reference size mismatch")
    );
}
#[test]
fn rejects_every_truncated_prefix_and_trailing_data() {
    let data = fixture(8);
    for end in 0..data.len() {
        assert!(
            SkateMap::parse(&data[..end]).is_err(),
            "accepted truncation at {end}"
        );
    }
    let mut data = data;
    data.push(0);
    assert!(SkateMap::parse(&data).is_err());
}
#[test]
fn rejects_future_versions_endianness_nonfinite_and_huge_counts() {
    let data = fixture(8);
    let mut bad = data.clone();
    bad[5] = b'9';
    bad[6] = b'9';
    assert!(SkateMap::parse(&bad).unwrap_err().contains("version"));
    let mut bad = data.clone();
    bad[8] = 0;
    assert!(SkateMap::parse(&bad).unwrap_err().contains("endian"));
    let mut bad = data.clone();
    bad[23..27].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(SkateMap::parse(&bad).unwrap_err().contains("Non-finite"));
    let mut bad = data;
    bad[219..223].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(SkateMap::parse(&bad).is_err());
}
#[test]
fn rejects_bad_vertex_and_material_references() {
    let data = fixture(1);
    // Locate the known three-index sequence immediately before collision data.
    let mut needle = Vec::new();
    for v in [0u32, 1, 2] {
        u(&mut needle, v);
    }
    let offset = data.windows(12).position(|w| w == needle).unwrap();
    let mut bad = data.clone();
    bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
    assert!(SkateMap::parse(&bad).unwrap_err().contains("indices"));
    let mut bad = data;
    bad[offset - 4..offset].copy_from_slice(&0u32.to_le_bytes());
    assert!(SkateMap::parse(&bad).unwrap_err().contains("reference"));
}
