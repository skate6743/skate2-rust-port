use super::*;

fn input() -> InvestigationInput {
    InvestigationInput {
        start: [0.; 4],
        end: [0., 0., 2., 0.],
        reference: [4., 5., 1., 0.],
        optional_probe: None,
        deck_center_to_truck: 0.243,
    }
}
fn hit(y: f32, fraction: f32, surface: u32) -> ProbeHit {
    ProbeHit {
        position: [0.243, y, 1., 0.],
        normal: UP,
        fraction,
        packed_surface: surface,
    }
}

#[test]
fn seven_descriptors_preserve_native_index_and_world_down_probe() {
    let mut value = input();
    value.optional_probe = Some([2., 3., 4., 0.]);
    let plan = prepare(value).unwrap();
    assert_eq!(plan.probes.len(), 7);
    assert_eq!(plan.center, [0., 0., 1., 0.]);
    assert_eq!(plan.probes[0].start, [0.09, 0.04, 1., 0.]);
    assert_eq!(plan.probes[4].start, [0., 0.04, 1., 0.]);
    assert_eq!(plan.probes[5].start, [0.09, 0.09, 1., 0.]);
    assert_eq!(plan.probes[5].radius.to_bits(), 0x3a83_126f);
    assert_eq!(
        plan.probes[6],
        Probe {
            start: [2., 3., 4., 0.],
            end: [2., -3., 4., 0.],
            radius: 0.
        }
    );
    let mut called = Vec::new();
    investigate(value, |i, _| {
        called.push(i);
        Ok::<_, ()>(None)
    })
    .unwrap();
    assert_eq!(called, [0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn raised_cross_probe_not_center_probe_blocks_geometry() {
    let plan = prepare(input()).unwrap();
    let mut hits = [None; 7];
    hits[4] = Some(hit(0., 0.5, 17));
    let result = resolve(&plan, &hits, 0);
    assert_eq!(result.kind, GeometryType::ThinRail);
    assert_eq!((result.audio_surface, result.physics_surface), (17, 4));
    hits[5] = Some(hit(0., 0.5, 17));
    let result = resolve(&plan, &hits, 0);
    assert_eq!(result.kind, GeometryType::Impossible);
    assert_ne!(result.flags & GrindSurface::BLOCKED_CROSS_SECTION, 0);
    assert_eq!(result.flags & GrindSurface::INVALID, 0);
}

#[test]
fn strict_coarse_boundary_high_side_and_surface_codes() {
    let plan = prepare(input()).unwrap();
    let mut hits = [None; 7];
    hits[1] = Some(hit(0., 0.5, 23 | (8 << 7)));
    hits[3] = Some(hit(-0.1, 0.65, 9));
    let result = resolve(&plan, &hits, 0);
    assert_eq!(result.kind, GeometryType::FatRail);
    assert_eq!(result.high_side[0], -1.);
    assert_eq!((result.audio_surface, result.physics_surface), (23, 8));
    assert_ne!(result.flags & GrindSurface::STAIR, 0);
    hits[3].as_mut().unwrap().fraction = f32::from_bits(0.65f32.to_bits() - 1);
    assert_eq!(resolve(&plan, &hits, 0).kind, GeometryType::Ledge);
}

#[test]
fn curb_requires_two_coarse_normals_and_strict_height_ranges() {
    let plan = prepare(input()).unwrap();
    let mut hits = [None; 7];
    hits[0] = Some(hit(0., 0.5, 17));
    hits[2] = Some(hit(0., 0.5, 17));
    hits[3] = Some(hit(-0.22, 0.9, 17));
    let result = resolve(&plan, &hits, 0);
    assert_ne!(result.flags & GrindSurface::CURB, 0);
    assert_eq!((result.audio_surface, result.physics_surface), (4, 2));
    hits[3].as_mut().unwrap().position[1] = -0.2;
    assert_eq!(resolve(&plan, &hits, 0).flags & GrindSurface::CURB, 0);
    hits[3].as_mut().unwrap().position[1] = -0.22;
    hits[3].as_mut().unwrap().normal = [0., 0.99, 0., 0.];
    assert_eq!(resolve(&plan, &hits, 0).flags & GrindSurface::CURB, 0);
}

#[test]
fn optional_query_flags_use_seventh_result_and_strict_drop_test() {
    let mut value = input();
    value.optional_probe = Some([0., 2., 1., 0.]);
    let plan = prepare(value).unwrap();
    let mut hits = [None; 7];
    hits[6] = Some(ProbeHit {
        position: [1., -1., 1., 0.],
        normal: [1., 0., 0., 0.],
        fraction: 0.5,
        packed_surface: 0,
    });
    let result = resolve(&plan, &hits, 0);
    assert_ne!(result.flags & GrindSurface::OPTIONAL_NORMAL_TEST, 0);
    assert_ne!(result.flags & GrindSurface::OPTIONAL_DROP_TEST, 0);
    hits[6].as_mut().unwrap().position[1] = -0.99;
    assert_eq!(
        resolve(&plan, &hits, 0).flags & GrindSurface::OPTIONAL_DROP_TEST,
        0
    );
}

#[test]
fn output_reuse_preserves_unassigned_bits_and_native_blocked_latch() {
    let plan = prepare(input()).unwrap();
    let old = GrindSurface::INVALID | GrindSurface::CURB | GrindSurface::BLOCKED_CROSS_SECTION | 7;
    let result = resolve(&plan, &[None; 7], old);
    assert_eq!(result.flags & 7, 7);
    assert_ne!(result.flags & GrindSurface::BLOCKED_CROSS_SECTION, 0);
    assert_eq!(
        result.flags & (GrindSurface::INVALID | GrindSurface::CURB),
        0
    );
}

#[test]
fn vertical_query_uses_native_no_submission_result() {
    let mut value = input();
    value.end = [0., 2., 0., 0.];
    let result = investigate(value, |_, _| -> Result<Option<ProbeHit>, ()> {
        panic!("native gate should not submit")
    })
    .unwrap();
    assert_ne!(result.flags & GrindSurface::INVALID, 0);
    assert_eq!((result.audio_surface, result.physics_surface), (3, 1));
    assert_eq!(result.kind, GeometryType::Impossible);
    assert_eq!(result.tilted_upmost_normal[2], -1.);
}

#[test]
fn tilted_normal_is_distinct_from_unrotated_upmost() {
    let rail = [0., 0., 1., 0.];
    assert_eq!(orientation::tilted_normal(rail, UP, [2., 4.]), UP);
    let tilted = orientation::tilted_normal(rail, UP, [0., 0.]);
    assert_eq!(tilted[1], -1.);
    assert_eq!(tilted[0], 0.);
}
