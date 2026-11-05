use super::*;

#[test]
fn removal_preserves_survivor_order_and_reuses_entries_and_nodes() {
    let mut tree = GrindDataOctree::try_new(32, bounds(-100.0, 100.0)).unwrap();
    // Many visits exceed the fixed entry capacity unless slots are reusable.
    for _ in 0..100 {
        for index in 0..20 {
            let p = -95.0 + index as f32;
            tree.try_insert(index, bounds(p, p + 0.1)).unwrap();
        }
        tree.try_insert(20, bounds(-200.0, 200.0)).unwrap(); // root stuck chain
        let mut expected = query(&tree, bounds(-300.0, 300.0));
        for index in (0..21).step_by(2).chain((1..21).step_by(2)) {
            tree.try_remove(index).unwrap();
            expected.retain(|entry| *entry != index);
            assert_eq!(query(&tree, bounds(-300.0, 300.0)), expected);
        }
        assert!(tree.boxes.iter().all(Option::is_none));
    }
}

fn bounds(minimum: f32, maximum: f32) -> OctreeBounds {
    OctreeBounds {
        minimum: [minimum, minimum, minimum, 0.0],
        maximum: [maximum, maximum, maximum, 0.0],
    }
}
fn query(tree: &GrindDataOctree, bbox: OctreeBounds) -> Vec<usize> {
    let mut query = tree.try_query(bbox).unwrap();
    let mut results = Vec::new();
    while let Some(index) = query.try_next().unwrap() {
        results.push(index);
    }
    results
}

#[test]
fn source_masks_have_overlapping_children_and_midpoint_ties_choose_low() {
    let parent = bounds(-10.0, 10.0);
    let (high_min, low_max) = parent.split_planes();
    assert_eq!(
        high_min.map(f32::to_bits),
        [-2.0_f32, -2.0, -2.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(
        low_max.map(f32::to_bits),
        [2.0_f32, 2.0, 2.0, 0.0].map(f32::to_bits)
    );
    let (child, selected) = parent.containing_child(bounds(-1.0, 1.0)).unwrap();
    assert_eq!(child, 0);
    assert_eq!(
        selected.maximum.map(f32::to_bits),
        low_max.map(f32::to_bits)
    );
    assert!(parent.containing_child(bounds(-2.0, 2.0)).is_none());
    assert!(!parent.in_corner(bounds(-2.0, 2.0)));
    assert!(parent.in_corner(bounds(-2.0, 1.0)));
}

#[test]
fn rounded_subtractions_choose_high_when_algebraic_center_comparison_ties() {
    let parent = bounds(f32::from_bits(0x4a00_0001), f32::from_bits(0x4a00_0032));
    let candidate = bounds(f32::from_bits(0x4a00_0018), f32::from_bits(0x4a00_001d));
    let (high_min, low_max) = parent.split_planes();
    let lower = candidate.minimum[0] - high_min[0];
    let upper = low_max[0] - candidate.maximum[0];
    assert_eq!(lower.to_bits(), 0x3f40_0000);
    assert_eq!(upper.to_bits(), 0x3e80_0000);
    assert_eq!(
        (candidate.minimum[0] + candidate.maximum[0]).to_bits(),
        (high_min[0] + low_max[0]).to_bits()
    );
    assert_eq!(parent.containing_child(candidate).unwrap().0, 7);
}

#[test]
fn child_leaf_heads_reverse_then_stuck_head_runs_last_even_below_cap() {
    let mut tree = GrindDataOctree::try_new(6, bounds(-10.0, 10.0)).unwrap();
    for (index, bbox) in [
        bounds(-9.0, -8.0),
        bounds(8.0, 9.0),
        bounds(-3.0, 3.0),
        bounds(-8.0, -7.0),
        bounds(7.0, 8.0),
        bounds(-4.0, 4.0),
    ]
    .into_iter()
    .enumerate()
    {
        tree.try_insert(index, bbox).unwrap();
    }
    assert_eq!(query(&tree, bounds(-10.0, 10.0)), [4, 1, 3, 0, 5, 2]);
    assert_eq!(query(&tree, bounds(-8.5, -7.5)), [3, 0]);
}

#[test]
fn fourth_corner_entry_splits_and_redistribution_reverses_the_old_chain() {
    let mut tree = GrindDataOctree::try_new(4, bounds(-10.0, 10.0)).unwrap();
    for index in 0..3 {
        tree.try_insert(index, bounds(-9.0, -8.0)).unwrap();
    }
    assert_eq!(query(&tree, bounds(-10.0, 10.0)), [2, 1, 0]);
    tree.try_insert(3, bounds(-9.0, -8.0)).unwrap();
    assert_eq!(query(&tree, bounds(-10.0, 10.0)), [0, 1, 2, 3]);
    assert_eq!(
        tree.nodes.iter().filter(|slot| slot.node.is_some()).count(),
        2
    );
}

#[test]
fn leaf_results_precede_queued_child_nodes_and_child_nodes_are_lifo() {
    let mut tree = GrindDataOctree::try_new(9, bounds(-10.0, 10.0)).unwrap();
    for index in 0..4 {
        tree.try_insert(index, bounds(-9.0, -8.0)).unwrap();
    }
    for index in 4..8 {
        tree.try_insert(index, bounds(8.0, 9.0)).unwrap();
    }
    tree.try_insert(8, bounds(-3.0, 3.0)).unwrap();
    assert_eq!(
        query(&tree, bounds(-10.0, 10.0)),
        [8, 4, 5, 6, 7, 0, 1, 2, 3]
    );
}

#[test]
fn nan_separation_is_not_rejection_and_w_is_retained_but_not_compared() {
    let mut tree = GrindDataOctree::try_new(
        2,
        OctreeBounds {
            minimum: [-10.0, -10.0, -10.0, -0.0],
            maximum: [10.0, 10.0, 10.0, -0.0],
        },
    )
    .unwrap();
    assert_eq!(tree.extent.minimum[3].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(tree.extent.maximum[3].to_bits(), 0.0_f32.to_bits());
    let mut first = bounds(-9.0, -8.0);
    first.minimum[3] = f32::from_bits(0x7fc1_2345);
    first.maximum[3] = f32::NEG_INFINITY;
    tree.try_insert(0, first).unwrap();
    tree.try_insert(1, bounds(8.0, 9.0)).unwrap();
    assert_eq!(tree.boxes[0].unwrap().minimum[3].to_bits(), 0x7fc1_2345);
    assert_eq!(query(&tree, bounds(f32::NAN, f32::NAN)), [1, 0]);
    assert_eq!(query(&tree, bounds(-9.0, -8.0)), [0]);
    assert!(
        bounds(f32::NAN, f32::NAN)
            .containing_child(bounds(0.0, 1.0))
            .is_some()
    );
}

#[test]
fn constructor_fsel_uses_subtraction_unordered_selection_and_full_w() {
    let tree = GrindDataOctree::try_new(
        0,
        OctreeBounds {
            minimum: [f32::NAN, -2.0, -3.0, 4.0],
            maximum: [f32::NAN, 2.0, 3.0, 8.0],
        },
    )
    .unwrap();
    assert!(tree.extent.minimum[0].is_nan());
    assert_eq!(
        tree.extent.minimum[1..]
            .iter()
            .copied()
            .map(f32::to_bits)
            .collect::<Vec<_>>(),
        [-3.0_f32, -3.0, 6.0].map(f32::to_bits)
    );
    assert!(query(&tree, bounds(-1.0, 1.0)).is_empty());
}

#[test]
fn failed_insert_does_not_fabricate_entries_or_truncate_sentinel_indexes() {
    assert!(matches!(
        GrindDataOctree::try_new(65_536, bounds(-1.0, 1.0)),
        Err(GrindOctreeError::EntryCapacity(65_536))
    ));
    let mut tree = GrindDataOctree::try_new(1, bounds(-1.0, 1.0)).unwrap();
    assert_eq!(
        tree.try_insert(1, bounds(0.0, 1.0)),
        Err(GrindOctreeError::EntryIndex(1))
    );
    tree.try_insert(0, bounds(-1.0, 1.0)).unwrap();
    assert_eq!(
        tree.try_insert(0, bounds(-0.5, 0.5)),
        Err(GrindOctreeError::DuplicateEntry(0))
    );
    assert_eq!(query(&tree, bounds(-1.0, 1.0)), [0]);
}
