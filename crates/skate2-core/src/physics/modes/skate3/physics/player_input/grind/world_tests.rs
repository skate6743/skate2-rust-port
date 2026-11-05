use super::*;
use skate3_core::physics::{
    board_world::{
        WorldTriangle,
        query_metadata::{QueryMesh, QueryMetadata},
    },
    contact::RetailContactMaterial,
    drive_frames::RetailAffineTransform,
};

fn triangle(height: f32, fatness: f32) -> WorldTriangle {
    WorldTriangle::from_vertices(
        [
            Vector3::new(-2., height, -2.),
            Vector3::new(-2., height, 2.),
            Vector3::new(2., height, -2.),
        ],
        RetailContactMaterial {
            static_friction: 0.7,
            dynamic_friction: 0.4,
            restitution: 0.2,
        },
        0xdead_beef,
        0xe0,
        [1.; 3],
        fatness,
    )
    .unwrap()
}

fn world(entries: &[(f32, QueryPool, i32, u16)], flags: u32, fatness: f32) -> BoardWorld {
    let triangles: Vec<_> = entries.iter().map(|e| triangle(e.0, fatness)).collect();
    let meshes = entries
        .iter()
        .enumerate()
        .map(|(index, e)| QueryMesh {
            triangle_range: index..index + 1,
            local_to_world: RetailAffineTransform::IDENTITY,
            world_to_local: RetailAffineTransform::IDENTITY,
            local_bounds: Bounds::from_points(triangles[index].triangle.vertices).unwrap(),
            matching_group: e.2,
            rejection_flags: u32::MAX,
            geometry: 700,
            pool: e.1,
        })
        .collect();
    BoardWorld::with_query_metadata(
        triangles,
        QueryMetadata {
            packed_surfaces: entries.iter().map(|e| e.3).collect(),
            meshes,
            static_edges: vec![],
            island_flags: flags,
        },
    )
    .unwrap()
}

fn down() -> Probe {
    Probe {
        start: [-0.5, 1., -0.5, 0.],
        end: [-0.5, -1., -0.5, 0.],
        radius: 0.,
    }
}

#[test]
fn exact_packed_surface_not_render_tag_and_zero_rejection_mask() {
    let w = world(&[(0., QueryPool::Ground, 7, 0xf3a9)], 0, 0.);
    let hit = surface_probe(&w, [700, 7], 0, down()).unwrap().unwrap();
    assert_eq!(hit.packed_surface, 0xf3a9);
    assert_eq!(hit.fraction, 0.5);
    assert_eq!(hit.normal, [0., 1., 0., 0.]);
    //Actor2948 equals mesh geometry, but neither participates in this filter.
    assert!(surface_probe(&w, [7, 8], 0, down()).unwrap().is_none());
    assert!(
        surface_probe(&w, [0, u32::MAX], 0, down())
            .unwrap()
            .is_some()
    );
    let wildcard = world(&[(0., QueryPool::Ground, -1, 5)], 0, 0.);
    assert!(
        surface_probe(&wildcard, [0, 999], 0, down())
            .unwrap()
            .is_some()
    );
}

#[test]
fn pool_order_precedes_mesh_order_and_nearest_still_wins() {
    let w = world(
        &[
            (0., QueryPool::Island, -1, 1),
            (0., QueryPool::Ground, -1, 2),
            (0., QueryPool::Ground, -1, 3),
        ],
        0,
        0.,
    );
    assert_eq!(
        surface_probe(&w, [0; 2], 0, down())
            .unwrap()
            .unwrap()
            .packed_surface,
        2
    );
    let w = world(
        &[
            (0., QueryPool::Ground, -1, 2),
            (0.5, QueryPool::Island, -1, 1),
        ],
        0,
        0.,
    );
    assert_eq!(
        surface_probe(&w, [0; 2], 0, down())
            .unwrap()
            .unwrap()
            .packed_surface,
        1
    );
}

#[test]
fn conditional_pool_requires_exact_island_state_three() {
    for flags in [0, 1, 2, 3, 4, 7] {
        let w = world(&[(0., QueryPool::Conditional, -1, 1)], flags, 0.);
        assert_eq!(
            surface_probe(&w, [0; 2], 0, down()).unwrap().is_some(),
            flags == 3
        );
    }
}

#[test]
fn contact_fatness_does_not_move_the_query_hit() {
    let w = world(&[(0., QueryPool::Ground, -1, 1)], 0, 0.5);
    let hit = surface_probe(&w, [0; 2], 0, down()).unwrap().unwrap();
    assert_eq!(hit.fraction, 0.5);
    assert_eq!(hit.position[1], 0.);
}

#[test]
fn rounded_edge_hit_publishes_authored_face_not_sweep_normal() {
    let w = world(&[(0., QueryPool::Ground, -1, 1)], 0, 0.);
    let probe = Probe {
        start: [-2.0005, 0.01, -1., 0.],
        end: [-2.0005, -0.01, -1., 0.],
        radius: 0.001,
    };
    let mut leaf = TriangleLineHit {
        position: Vector3::ZERO,
        normal: Vector3::ZERO,
        fraction: 0.,
        volume_parameter: [0.; 3],
    };
    assert!(triangle_segment(
        &mut leaf,
        vector(probe.start),
        sub(vector(probe.end), vector(probe.start)),
        w.triangles()[0].triangle.vertices,
        probe.radius,
        0.
    ));
    assert!(leaf.normal.x.abs() > 0.1);
    let hit = surface_probe(&w, [0; 2], 5, probe).unwrap().unwrap();
    assert_eq!(hit.normal, [0., 1., 0., 0.]);
    assert_eq!(hit.position, packed(leaf.position));
}

#[test]
fn exact_outer_bounds_reject_leaf_endpoint_tolerance() {
    let w = world(&[(0., QueryPool::Ground, -1, 1)], 0, 0.);
    let probe = Probe {
        end: [-0.5, 0.000001, -0.5, 0.],
        ..down()
    };
    let mut leaf = TriangleLineHit {
        position: Vector3::ZERO,
        normal: Vector3::ZERO,
        fraction: 0.,
        volume_parameter: [0.; 3],
    };
    assert!(triangle_segment(
        &mut leaf,
        vector(probe.start),
        sub(vector(probe.end), vector(probe.start)),
        w.triangles()[0].triangle.vertices,
        0.,
        0.
    ));
    assert!(surface_probe(&w, [0; 2], 0, probe).unwrap().is_none());
}

#[test]
fn native_short_line_gate_and_force_exit_share_matching_contract() {
    let w = world(&[(0., QueryPool::Ground, 9, 1)], 0, 0.);
    let half = f32::from_bits(0x3780_0000) * 0.5;
    let probe = Probe {
        start: [-0.5, half, -0.5, 0.],
        end: [-0.5, -half, -0.5, 0.],
        radius: 0.001,
    };
    assert!(surface_probe(&w, [0, 9], 5, probe).unwrap().is_none());
    let p = down();
    let line = ForceExitProbe {
        start: p.start,
        end: p.end,
    };
    assert!(force_exit_line(&w, [9, 8], line).unwrap().is_none());
    assert_eq!(
        force_exit_line(&w, [0, 9], line).unwrap().unwrap().normal,
        [0., 1., 0., 0.]
    );
}

#[test]
fn triangle_bounds_gate_is_required_even_inside_a_larger_mesh() {
    let triangles = vec![triangle(0., 0.), triangle(2., 0.)];
    let local_bounds =
        Bounds::from_points(triangles.iter().flat_map(|t| t.triangle.vertices)).unwrap();
    let w = BoardWorld::with_query_metadata(
        triangles,
        QueryMetadata {
            packed_surfaces: vec![1, 2],
            meshes: vec![QueryMesh {
                triangle_range: 0..2,
                local_to_world: RetailAffineTransform::IDENTITY,
                world_to_local: RetailAffineTransform::IDENTITY,
                local_bounds,
                matching_group: -1,
                rejection_flags: 0,
                geometry: 0,
                pool: QueryPool::Ground,
            }],
            static_edges: vec![],
            island_flags: 0,
        },
    )
    .unwrap();
    let probe = Probe {
        end: [-0.5, 0.000001, -0.5, 0.],
        ..down()
    };
    assert!(surface_probe(&w, [0; 2], 0, probe).unwrap().is_none());
}

#[test]
fn both_side_collector_does_not_replace_one_sided_thin_leaf() {
    let w = world(&[(0., QueryPool::Ground, -1, 1)], 0, 0.);
    let d = down();
    assert!(
        surface_probe(
            &w,
            [0; 2],
            0,
            Probe {
                start: d.end,
                end: d.start,
                radius: 0.
            }
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn missing_metadata_and_invalid_descriptors_are_errors_not_misses() {
    assert!(surface_probe(&BoardWorld::new(vec![]), [0; 2], 0, down()).is_err());
    let w = world(&[], 0, 0.);
    assert!(surface_probe(&w, [0; 2], 7, down()).is_err());
    assert!(
        surface_probe(
            &w,
            [0; 2],
            0,
            Probe {
                radius: -0.1,
                ..down()
            }
        )
        .is_err()
    );
    assert!(
        surface_probe(
            &w,
            [0; 2],
            0,
            Probe {
                start: [f32::NAN; 4],
                ..down()
            }
        )
        .is_err()
    );
    assert!(surface_probe(&w, [0; 2], 0, down()).unwrap().is_none());
}

#[test]
fn outer_fsel_clamps_fraction_without_reconstructing_position() {
    assert_eq!(clamp_fraction(-0.000001), 0.);
    assert_eq!(clamp_fraction(1.000001), 1.);
    assert_eq!(clamp_fraction(f32::NAN), 1.);
    assert_eq!(clamp_fraction(0.25), 0.25);
}
