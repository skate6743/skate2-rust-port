use super::*;
use skate3_core::player::offboard::grab_scene::Geometry;
use std::sync::Arc;
fn record(id: u32, y: f32, z: f32) -> Record {
    let a = [-1., y, z, 0.];
    let b = [1., y, z, 0.];
    let geometry = Arc::new(Geometry {
        id,
        points: vec![a, b],
        approach_vectors: vec![],
        word_60: 0,
    });
    let mut r = Record([0; 72], geometry);
    r.0[47] = 2;
    r.0[48] = id;
    r.0[49] = id;
    r.0[52] = id;
    r.set_vector(64, a);
    r.set_vector(80, b);
    r
}
#[test]
fn s3_validation_reprocesses_swap_with_next_hit_not_next_triplet() {
    let mut records = vec![record(1, 0., 1.), record(2, 0., 1.), record(3, 0., 1.)];
    //First null-assembly obstruction removes1. Next hit belongs to3: if the
    //implementation aligns to three hits or keeps original order, it rejects3.
    let hits = [
        Some(Hit {
            fraction: 0.5,
            assembly: None,
        }),
        Some(Hit {
            fraction: 0.5,
            assembly: Some(3),
        }),
        None,
        None,
        Some(Hit {
            fraction: 0.5,
            assembly: Some(2),
        }),
        None,
        None,
        None,
        None,
    ];
    validate(&mut records, &hits);
    assert_eq!(
        records
            .iter()
            .map(|r| r.descriptor().id)
            .collect::<Vec<_>>(),
        vec![3, 2]
    );
}
#[test]
fn nearest_ineligible_suppresses_farther_eligible() {
    let near = record(1, 0.6, 0.1);
    let far = record(2, 0., 2.);
    assert!(selection::best(&[far.clone()], [0.; 4]).is_some());
    assert!(selection::best(&[far, near], [0.; 4]).is_none());
}
#[test]
fn both_height_boundaries_are_exclusive() {
    for height in [-0.6, 0.6] {
        assert!(selection::best(&[record(1, height, 0.2)], [0.; 4]).is_none());
    }
}
#[test]
fn invalidation_preserves_retained_candidates_and_interactable_completion() {
    let mut owner = Owner::default();
    owner.validated.push(record(1, 0., 1.));
    owner.flags_12836 = 0xff;
    owner.interactable_result = Some(Some(44));
    owner.data_ready = [true, true];
    owner.invalidate();
    assert_eq!(owner.flags_12836, 0x3f);
    assert_eq!(owner.validated.len(), 1);
    assert_eq!(owner.interactable_result, Some(Some(44)));
    assert_eq!(owner.data_ready, [false, false]);
    owner.enter_reset();
    assert_eq!(owner.interactable_result, None);
}
#[test]
fn publication_consumes_ready_not_retained_data() {
    let mut owner = Owner::default();
    owner.data[0] = Some(record(1, 0., 1.));
    owner.data_ready[0] = true;
    owner.interactable_result = Some(Some(55));
    let first = owner.publish();
    assert!(first.records[0].is_some());
    assert_eq!(first.object, Some(Some(55)));
    assert!(owner.interactable_latched);
    let second = owner.publish();
    assert!(second.records[0].is_none());
    assert_eq!(second.object, None);
    assert!(owner.data[0].is_some());
}
