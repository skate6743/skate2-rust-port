use super::world_contact::{FeaturePrism, MaximumFeature, find_feature_intersection_prism};

const NORMAL: [u32; 4] = [0, 1.0f32.to_bits(), 0, 0];
const SENTINEL: u32 = 0xdead_beef;

fn vector(value: [f32; 4]) -> [u32; 4] {
    value.map(f32::to_bits)
}
fn point(x: f32, y: f32, z: f32) -> MaximumFeature {
    let mut feature = [0; 144];
    feature[0] = 100;
    feature[136..140].copy_from_slice(&vector([x, y, z, 1.0]));
    feature
}
fn segment(start: [f32; 3], end: [f32; 3]) -> MaximumFeature {
    let mut feature = [0; 144];
    feature[0] = 100;
    feature[140] = 1;
    let delta: [f32; 3] = std::array::from_fn(|i| end[i] - start[i]);
    let length = delta.iter().map(|v| v * v).sum::<f32>().sqrt();
    feature[4..8].copy_from_slice(&vector([start[0], start[1], start[2], 1.0]));
    feature[8..12].copy_from_slice(&vector([
        delta[0] / length,
        delta[1] / length,
        delta[2] / length,
        0.0,
    ]));
    feature[16..20].fill(length.to_bits());
    feature
}
fn face(vertices: &[[f32; 2]], height: f32) -> MaximumFeature {
    let mut feature = [0; 144];
    feature[0] = 100;
    feature[140] = vertices.len() as u32;
    feature[132..136].copy_from_slice(&NORMAL);
    for (edge, &start) in vertices.iter().enumerate() {
        let end = vertices[(edge + 1) % vertices.len()];
        let line = segment([start[0], height, start[1]], [end[0], height, end[1]]);
        let offset = 4 + edge * 16;
        feature[offset..offset + 16].copy_from_slice(&line[4..20]);
        let dx = f32::from_bits(line[8]);
        let dz = f32::from_bits(line[10]);
        feature[offset + 8..offset + 12].copy_from_slice(&vector([dz, 0.0, -dx, 0.0]));
    }
    feature
}
fn square(x: f32, z: f32, height: f32) -> MaximumFeature {
    face(
        &[
            [x - 1.0, z - 1.0],
            [x + 1.0, z - 1.0],
            [x + 1.0, z + 1.0],
            [x - 1.0, z + 1.0],
        ],
        height,
    )
}
fn query(a: &mut MaximumFeature, b: &mut MaximumFeature) -> FeaturePrism {
    let mut result = [SENTINEL; 136];
    result[128..132].copy_from_slice(&NORMAL);
    result[133] = 0;
    assert_eq!(
        find_feature_intersection_prism(&mut result, a, b, NORMAL),
        1
    );
    result
}
fn assert_point(output: &FeaturePrism, side: usize, index: usize, expected: [f32; 3]) {
    for axis in 0..3 {
        let actual = f32::from_bits(output[side * 64 + index * 4 + axis]);
        assert!(
            (actual - expected[axis]).abs() < 2.0e-5,
            "side={side} point={index} axis={axis}: {actual} != {}",
            expected[axis]
        );
    }
}
fn assert_untouched_tail(output: &FeaturePrism) {
    let count = output[132] as usize;
    assert!(output[count * 4..64].iter().all(|&v| v == SENTINEL));
    assert!(output[64 + count * 4..128].iter().all(|&v| v == SENTINEL));
    assert_eq!(&output[134..136], &[SENTINEL; 2]);
}

#[test]
fn point_point_copies_both_points_and_preserves_prism_state() {
    let mut a = point(2.0, 3.0, 4.0);
    let mut b = point(-3.0, 8.0, 6.0);
    let output = query(&mut a, &mut b);
    assert_eq!(output[132], 1);
    assert_eq!(&output[..4], &a[136..140]);
    assert_eq!(&output[64..68], &b[136..140]);
    assert_eq!(&output[128..132], &NORMAL);
    assert_eq!(output[133], 0);
    assert_eq!((a[0], b[0]), (100, 100));
    assert_untouched_tail(&output);
}

#[test]
fn point_segment_selects_before_interior_and_after_tags_in_both_orders() {
    for (x, projected, tag) in [
        (-1.0, 0.0, 101),
        (0.0, 0.0, 100),
        (1.0, 1.0, 100),
        (2.0, 2.0, 100),
        (3.0, 2.0, 99),
    ] {
        for reverse in [false, true] {
            let mut line = segment([0.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
            let mut p = point(x, 2.0, 0.0);
            let output = if reverse {
                query(&mut p, &mut line)
            } else {
                query(&mut line, &mut p)
            };
            assert_eq!(output[132], 1);
            assert_point(&output, usize::from(reverse), 0, [projected, 0.0, 0.0]);
            assert_point(&output, usize::from(!reverse), 0, [x, 2.0, 0.0]);
            assert_eq!(line[0], tag);
            assert_eq!(p[0], 100);
            assert_untouched_tail(&output);
        }
    }
}

#[test]
fn point_face_projects_and_clamps_to_boundary_in_both_orders() {
    for (z, projected, tag) in [(0.0, 0.0, 100), (-2.0, -1.0, 102)] {
        for reverse in [false, true] {
            let mut polygon = square(0.0, 0.0, 0.0);
            let mut p = point(0.0, 2.0, z);
            let output = if reverse {
                query(&mut p, &mut polygon)
            } else {
                query(&mut polygon, &mut p)
            };
            assert_eq!(output[132], 1);
            assert_point(&output, usize::from(reverse), 0, [0.0, 0.0, projected]);
            assert_point(&output, usize::from(!reverse), 0, [0.0, 2.0, z]);
            assert_eq!(polygon[0], tag);
            assert_untouched_tail(&output);
        }
    }
}

#[test]
fn segment_segment_covers_crossing_overlap_and_disjoint_endpoints() {
    let mut a = segment([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    let mut b = segment([0.0, 1.0, -1.0], [0.0, 1.0, 1.0]);
    let crossing = query(&mut a, &mut b);
    assert_eq!(crossing[132], 1);
    assert_point(&crossing, 0, 0, [0.0, 0.0, 0.0]);
    assert_point(&crossing, 1, 0, [0.0, 1.0, 0.0]);
    let mut b = segment([0.0, 1.0, 0.0], [2.0, 1.0, 0.0]);
    let overlap = query(&mut a, &mut b);
    assert_eq!(overlap[132], 2);
    for side in 0..2 {
        assert_point(&overlap, side, 0, [0.0, side as f32, 0.0]);
        assert_point(&overlap, side, 1, [1.0, side as f32, 0.0]);
    }
    let mut b = segment([3.0, 1.0, 0.0], [4.0, 1.0, 0.0]);
    let disjoint = query(&mut a, &mut b);
    assert_eq!(disjoint[132], 1);
    assert_point(&disjoint, 0, 0, [1.0, 0.0, 0.0]);
    assert_point(&disjoint, 1, 0, [3.0, 1.0, 0.0]);
}

#[test]
fn segment_face_keeps_clipped_two_point_manifold_in_both_orders() {
    for reverse in [false, true] {
        let mut polygon = square(0.0, 0.0, 0.0);
        let mut line = segment([-2.0, 1.0, 0.0], [2.0, 1.0, 0.0]);
        // For this winding, -Y cross edge points into the clipping polygon.
        let direction = vector([0.0, -1.0, 0.0, 0.0]);
        let mut output = [SENTINEL; 136];
        output[128..132].copy_from_slice(&direction);
        output[133] = 0;
        let (a, b) = if reverse {
            (&mut line, &mut polygon)
        } else {
            (&mut polygon, &mut line)
        };
        assert_eq!(
            find_feature_intersection_prism(&mut output, a, b, direction),
            1
        );
        assert_eq!(output[132], 2);
        for (index, x) in [-1.0, 1.0].into_iter().enumerate() {
            assert_point(&output, usize::from(reverse), index, [x, 0.0, 0.0]);
            assert_point(&output, usize::from(!reverse), index, [x, 1.0, 0.0]);
        }
        assert_untouched_tail(&output);
    }
}

#[test]
fn quad_triangle_emits_edge_pairs_then_inside_vertex_with_stable_swapped_order() {
    let expected = [[1.0, -0.5], [1.0, 0.5], [0.5, 1.0], [0.0, 1.0], [0.0, -0.5]];
    for reverse in [false, true] {
        let mut quad = square(0.0, 0.0, 0.0);
        let mut triangle = face(&[[0.0, -0.5], [2.0, -0.5], [0.0, 1.5]], 1.0);
        let output = if reverse {
            query(&mut triangle, &mut quad)
        } else {
            query(&mut quad, &mut triangle)
        };
        assert_eq!(output[132], 5);
        for (index, [x, z]) in expected.into_iter().enumerate() {
            assert_point(&output, usize::from(reverse), index, [x, 0.0, z]);
            assert_point(&output, usize::from(!reverse), index, [x, 1.0, z]);
        }
        assert_eq!(output[133], 0);
        assert_eq!((quad[0], triangle[0]), (100, 100));
        assert_untouched_tail(&output);
    }
}

#[test]
fn quad_quad_orders_crossings_before_both_containment_passes() {
    let mut a = square(0.0, 0.0, 0.0);
    let mut b = square(1.0, 0.5, 1.0);
    let output = query(&mut a, &mut b);
    assert_eq!(output[132], 4);
    for (index, [x, z]) in [[1.0, -0.5], [0.0, 1.0], [1.0, 1.0], [0.0, -0.5]]
        .into_iter()
        .enumerate()
    {
        assert_point(&output, 0, index, [x, 0.0, z]);
        assert_point(&output, 1, index, [x, 1.0, z]);
    }
    assert_untouched_tail(&output);
}

#[test]
fn coincident_quad_boundaries_keep_native_duplicate_edge_intersections() {
    let mut a = square(0.0, 0.0, 0.0);
    let mut b = square(0.0, 0.0, 1.0);
    let output = query(&mut a, &mut b);
    assert_eq!(output[132], 8);
    for index in 0..8 {
        assert_eq!(f32::from_bits(output[index * 4]).abs(), 1.0);
        assert_eq!(f32::from_bits(output[index * 4 + 2]).abs(), 1.0);
    }
    assert_untouched_tail(&output);
}

#[test]
fn general_triangle_containment_returns_b_vertices_without_duplicates() {
    let vertices = [[-1.0, -1.0], [1.0, -1.0], [0.0, 1.0]];
    let mut a = face(&vertices, 0.0);
    let mut b = face(&vertices, 1.0);
    let output = query(&mut a, &mut b);
    assert_eq!(output[132], 3);
    for (index, [x, z]) in vertices.into_iter().enumerate() {
        assert_point(&output, 0, index, [x, 0.0, z]);
        assert_point(&output, 1, index, [x, 1.0, z]);
    }
    assert_untouched_tail(&output);
}

#[test]
fn general_triangle_crossings_form_the_six_corner_intersection() {
    let mut a = face(&[[-2.0, -1.0], [2.0, -1.0], [0.0, 2.0]], 0.0);
    let mut b = face(&[[-2.0, 1.0], [0.0, -2.0], [2.0, 1.0]], 1.0);
    let output = query(&mut a, &mut b);
    assert_eq!(output[132], 6);
    let expected = [
        [-4.0 / 3.0, 0.0],
        [-2.0 / 3.0, -1.0],
        [2.0 / 3.0, -1.0],
        [4.0 / 3.0, 0.0],
        [2.0 / 3.0, 1.0],
        [-2.0 / 3.0, 1.0],
    ];
    for [x, z] in expected {
        assert!((0..6).any(|index| {
            (f32::from_bits(output[index * 4]) - x).abs() < 2.0e-5
                && (f32::from_bits(output[index * 4 + 2]) - z).abs() < 2.0e-5
        }));
    }
    for index in 0..6 {
        let x = f32::from_bits(output[index * 4]);
        let z = f32::from_bits(output[index * 4 + 2]);
        assert_point(&output, 1, index, [x, 1.0, z]);
    }
    assert_untouched_tail(&output);
}

#[test]
fn disjoint_faces_fall_back_to_closest_features_and_correct_normal() {
    let mut a = face(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], 0.0);
    let mut b = face(&[[3.0, 3.0], [4.0, 3.0], [3.0, 4.0]], 0.0);
    let output = query(&mut a, &mut b);
    assert_eq!(output[132], 1);
    assert_point(&output, 0, 0, [0.5, 0.0, 0.5]);
    assert_point(&output, 1, 0, [3.0, 0.0, 3.0]);
    assert_eq!((a[0], b[0]), (4, 1));
    assert_eq!(output[133], 1);
    assert!((f32::from_bits(output[128]) - std::f32::consts::FRAC_1_SQRT_2).abs() < 2.0e-5);
    assert!((f32::from_bits(output[130]) - std::f32::consts::FRAC_1_SQRT_2).abs() < 2.0e-5);
    assert_untouched_tail(&output);
}

#[test]
fn specialized_quad_rejection_reaches_parallel_edge_fallback() {
    let mut a = square(0.0, 0.0, 0.0);
    let mut b = square(3.0, 0.0, 0.0);
    let output = query(&mut a, &mut b);
    assert_eq!(output[132], 2);
    for (index, z) in [-1.0, 1.0].into_iter().enumerate() {
        assert_point(&output, 0, index, [1.0, 0.0, z]);
        assert_point(&output, 1, index, [2.0, 0.0, z]);
    }
    assert_eq!((a[0], b[0]), (4, 8));
    assert_eq!(output[133], 1);
    assert_eq!(&output[128..132], &vector([1.0, 0.0, 0.0, 0.0]));
    assert_untouched_tail(&output);
}

#[test]
fn specialized_containment_handles_each_face_as_the_enclosed_feature() {
    for triangle_is_enclosed in [false, true] {
        for reverse in [false, true] {
            let (mut quad, mut triangle, expected) = if triangle_is_enclosed {
                (
                    square(0.0, 0.0, 0.0),
                    face(&[[-0.5, -0.5], [0.5, -0.5], [0.0, 0.5]], 1.0),
                    vec![[-0.5, -0.5], [0.5, -0.5], [0.0, 0.5]],
                )
            } else {
                (
                    square(0.0, 0.0, 0.0),
                    face(&[[-5.0, -3.0], [5.0, -3.0], [0.0, 5.0]], 1.0),
                    vec![[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]],
                )
            };
            let output = if reverse {
                query(&mut triangle, &mut quad)
            } else {
                query(&mut quad, &mut triangle)
            };
            assert_eq!(output[132] as usize, expected.len());
            for (index, [x, z]) in expected.into_iter().enumerate() {
                assert_point(&output, usize::from(reverse), index, [x, 0.0, z]);
                assert_point(&output, usize::from(!reverse), index, [x, 1.0, z]);
                assert_eq!(output[index * 4 + 3], 1.0f32.to_bits());
                assert_eq!(output[64 + index * 4 + 3], 1.0f32.to_bits());
            }
            assert_eq!(output[133], 0);
            assert_untouched_tail(&output);
        }
    }
}
