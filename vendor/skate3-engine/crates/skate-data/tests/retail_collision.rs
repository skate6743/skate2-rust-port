use skate_data::retail_collision::visit_clusters;

const FIXTURE: &[u8] = include_bytes!("fixtures/retail-collision.rwcmset");

#[test]
fn compressed_offsets_zero_extend_across_the_sign_boundary() {
    let mut bytes = FIXTURE.to_vec();
    // Locate the second fixture mesh, which uses 16-bit offsets.
    let mut at = 12;
    let mut mesh = 0;
    for _ in 0..2 {
        let name_len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4 + name_len;
        let size = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        mesh = at + 4;
        at = mesh + size;
    }
    let cluster = mesh + 160;
    let base = [-40000_i32, -32768, i32::MAX - 2];
    for axis in 0..3 {
        bytes[cluster + 16 + axis * 4..cluster + 20 + axis * 4]
            .copy_from_slice(&base[axis].to_be_bytes());
    }
    for (i, raw) in [0x7fff_u16, 0x8000, 0xffff].into_iter().enumerate() {
        for axis in 0..3 {
            bytes[cluster + 28 + i * 6 + axis * 2..cluster + 30 + i * 6 + axis * 2]
                .copy_from_slice(&raw.to_be_bytes());
        }
    }
    visit_clusters(&bytes, |name, triangles| {
        if name == "compression-1" {
            assert_eq!(
                triangles[0].points,
                [
                    [-1808.25, -0.25, 536870912.],
                    [-1808., 0., 536870912.],
                    [6383.75, 8191.75, 536870912.],
                ]
            );
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn preserves_all_vertex_encodings_and_little_endian_unit_ids() {
    let mut clusters = 0;
    let total = visit_clusters(FIXTURE, |name, triangles| {
        assert_eq!(name, format!("compression-{clusters}"));
        assert_eq!(triangles.len(), 1);
        let t = triangles[0];
        let x = clusters as f32 * 10.;
        assert_eq!(t.points, [[x, 0., 0.], [x, 0., 1.], [x + 1., 0., 0.]]);
        assert_eq!(t.edges, Some([0x20, 0x42, 0x9a]));
        assert_eq!(t.surface, 0x4321);
        assert_eq!(t.group, 0x1234);
        assert!(t.one_sided);
        clusters += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(total, 3);
    assert_eq!(clusters, 3);
}

/// Removes the surface ID from the first unit of fixture mesh `index`: clears
/// unit flag 0x80 and shortens the unit stream by the 2-byte ID (its last field).
fn strip_surface(bytes: &mut [u8], index: usize) {
    let be16 = |b: &[u8], at: usize| u16::from_be_bytes(b[at..at + 2].try_into().unwrap());
    let be32 = |b: &[u8], at: usize| u32::from_be_bytes(b[at..at + 4].try_into().unwrap()) as usize;
    let mut at = 12;
    let mut mesh = 0;
    for _ in 0..=index {
        let name_len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4 + name_len;
        let size = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        mesh = at + 4;
        at = mesh + size;
    }
    assert_eq!(bytes[mesh + 63], 2, "fixture surface IDs are 2 bytes");
    let cluster = mesh + be32(bytes, mesh + be32(bytes, mesh + 52));
    let unit_start = (usize::from(be16(bytes, cluster + 4)) + 1) * 16;
    assert_ne!(bytes[cluster + unit_start] & 0x80, 0);
    bytes[cluster + unit_start] &= !0x80;
    let length = be16(bytes, cluster + 2) - 2;
    bytes[cluster + 2..cluster + 4].copy_from_slice(&length.to_be_bytes());
}

#[test]
fn skips_meshes_without_any_surface_ids() {
    let mut bytes = FIXTURE.to_vec();
    strip_surface(&mut bytes, 1);
    let mut names = Vec::new();
    let total = visit_clusters(&bytes, |name, triangles| {
        assert!(triangles.iter().all(|t| t.has_surface));
        names.push(name.to_owned());
        Ok(())
    })
    .unwrap();
    assert_eq!(names, ["compression-0", "compression-2"]);
    assert_eq!(total, 2);
}

#[test]
fn rejects_every_truncated_prefix_and_trailing_bytes() {
    for end in 0..FIXTURE.len() {
        assert!(
            visit_clusters(&FIXTURE[..end], |_, _| Ok(())).is_err(),
            "prefix {end}"
        );
    }
    let mut bytes = FIXTURE.to_vec();
    bytes.push(0);
    assert!(visit_clusters(&bytes, |_, _| Ok(())).is_err());
}

#[test]
#[ignore = "requires SKATE_RWCM_TEST_PATH pointing to a private archive"]
fn private_archive_decodes_all_clusters() {
    let path = std::env::var("SKATE_RWCM_TEST_PATH").unwrap();
    let bytes = std::fs::read(path).unwrap();
    let count = visit_clusters(&bytes, |_, _| Ok(())).unwrap();
    assert!(count > 0);
    eprintln!("RWCM triangles decoded: {count}");
}
