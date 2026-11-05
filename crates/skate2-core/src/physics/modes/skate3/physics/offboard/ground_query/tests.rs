use super::*;
use skate3_core::player::offboard::ground_query::QueryContext;
#[test]
fn indexed_edges_sort_ids_and_preserve_shared_capacity() {
    let bounds = Bounds {
        min: Vector3::new(-1., -1., -1.),
        max: Vector3::new(1., 1., 1.),
    };
    let segments = [Segment {
        edge: Edge {
            start: Vector3::ZERO,
            end: Vector3::new(1., 0., 0.),
        },
        local_bounds: bounds,
    }];
    let first = EdgeBody {
        local_to_world: Frame::IDENTITY,
        local_bounds: bounds,
        segments: &segments,
    };
    let second = EdgeBody {
        local_to_world: Frame {
            position: Vector3::new(0., 0.25, 0.),
            ..Frame::IDENTITY
        },
        local_bounds: bounds,
        segments: &segments,
    };
    let entries = [
        IndexedEdgeBody {
            id: 9,
            disabled: false,
            body: &first,
        },
        IndexedEdgeBody {
            id: 2,
            disabled: false,
            body: &second,
        },
    ];
    let static_edges = vec![segments[0]; 39];
    let mut scene = Scene {
        ground_pool: &[],
        island_pool: &[],
        conditional_pool: &[],
        island_flags: 0,
        static_edges: &static_edges,
        primary_edges: PrimaryEdges::Normal {
            dynamic: &[],
            vehicles: &[],
        },
        indexed_edges: &entries,
    };
    let search = EdgeSearch {
        min: bounds.min,
        max: bounds.max,
        frame: Frame::IDENTITY,
        context: QueryContext {
            selection_flags_2948: 0,
            matching_id_2952: -1,
        },
        narrow_forward: false,
    };
    let result = scene.edge_candidates(&search).unwrap();
    assert_eq!(result.len(), 40);
    assert_eq!(result[39].start.y, 0.25);
}
#[test]
fn decoded_face_uses_world_vertices_and_inclusive_bounds() {
    let face = transform::face([
        Vector3::ZERO,
        Vector3::new(0., 0., 2.),
        Vector3::new(3., 0., 0.),
    ]);
    assert!((face.y - 1.).abs() < 1e-6);
    let a = Bounds {
        min: Vector3::ZERO,
        max: Vector3::new(1., 1., 1.),
    };
    let b = Bounds {
        min: Vector3::new(1., 1., 1.),
        max: Vector3::new(2., 2., 2.),
    };
    assert!(transform::overlaps(a, b));
    assert!(transform::matches(-1, 3));
    assert!(!transform::matches(2, 3));
}

fn triangle(y: f32) -> WorldTriangle {
    use skate3_core::physics::contact::RetailContactMaterial;
    WorldTriangle::from_vertices(
        [
            Vector3::new(-2., y, -2.),
            Vector3::new(-2., y, 2.),
            Vector3::new(2., y, -2.),
        ],
        RetailContactMaterial {
            static_friction: 0.,
            dynamic_friction: 0.,
            restitution: 0.,
        },
        123,
        0,
        [0.; 3],
        0.,
    )
    .unwrap()
}
fn packet() -> GroundQueryPacket {
    use skate3_core::player::offboard::ground_query::Line;
    GroundQueryPacket {
        context: QueryContext {
            selection_flags_2948: 0,
            matching_id_2952: 7,
        },
        center: Vector3::ZERO,
        up: Vector3::new(0., 1., 0.),
        tangent: Vector3::new(0., 0., 1.),
        lines: [Line {
            start: Vector3::new(-0.5, 2., -0.5),
            end: Vector3::new(-0.5, -2., -0.5),
            radius: 0.,
        }; 7],
    }
}
fn mesh<'a>(triangles: &'a [WorldTriangle], surfaces: &'a [u16], matching_group: i32) -> Mesh<'a> {
    Mesh {
        triangles,
        surfaces,
        matching_group,
        local_to_world: Frame::IDENTITY,
        world_to_local: Frame::IDENTITY,
        local_bounds: Bounds {
            min: Vector3::new(-2., -2., -2.),
            max: Vector3::new(2., 2., 2.),
        },
    }
}
#[test]
fn actual_triangle_queries_preserve_pool_ties_group_filter_and_conditional_gate() {
    let triangles = [triangle(0.)];
    let upper = [triangle(1.)];
    let ground = [mesh(&triangles, &[11], 7)];
    let island = [mesh(&triangles, &[22], -1)];
    let conditional = [mesh(&upper, &[33], 7)];
    let mut scene = Scene {
        ground_pool: &ground,
        island_pool: &island,
        conditional_pool: &conditional,
        island_flags: 0,
        static_edges: &[],
        primary_edges: PrimaryEdges::Normal {
            dynamic: &[],
            vehicles: &[],
        },
        indexed_edges: &[],
    };
    let p = packet();
    let hit = scene.query_lines(&p).unwrap()[0].unwrap();
    assert_eq!(hit.packed_surface, 11);
    assert_eq!(hit.fraction, 0.5);
    assert!(hit.face_normal.y > 0.99);
    let mut different = p;
    different.context.matching_id_2952 = 8;
    assert_eq!(
        scene.query_lines(&different).unwrap()[0]
            .unwrap()
            .packed_surface,
        22
    );
    scene.island_flags = 3;
    assert_eq!(
        scene.query_lines(&p).unwrap()[0].unwrap().packed_surface,
        33
    );
}
#[test]
fn canonical_world_bridge_requires_metadata_and_queries_authored_surfaces() {
    use skate3_core::physics::{
        board_world::{
            BoardWorld,
            query_metadata::{Bounds as WorldBounds, QueryMesh, QueryMetadata, QueryPool},
        },
        drive_frames::RetailAffineTransform,
    };
    let unannotated = BoardWorld::new(vec![triangle(0.)]);
    let empty = || PrimaryEdges::Normal {
        dynamic: &[],
        vehicles: &[],
    };
    assert!(
        with_world_scene(&unannotated, empty(), &[], |scene| scene
            .query_lines(&packet()))
        .is_err()
    );
    let world = BoardWorld::with_query_metadata(
        vec![triangle(0.)],
        QueryMetadata {
            packed_surfaces: vec![0x400],
            island_flags: 0,
            static_edges: vec![],
            meshes: vec![QueryMesh {
                triangle_range: 0..1,
                local_to_world: RetailAffineTransform::IDENTITY,
                world_to_local: RetailAffineTransform::IDENTITY,
                local_bounds: WorldBounds {
                    min: Vector3::new(-2., 0., -2.),
                    max: Vector3::new(2., 0., 2.),
                },
                matching_group: 7,
                pool: QueryPool::Ground,
                rejection_flags: 0,
                geometry: 73,
            }],
        },
    )
    .unwrap();
    let hits =
        with_world_scene(&world, empty(), &[], |scene| scene.query_lines(&packet())).unwrap();
    assert_eq!(hits[0].unwrap().packed_surface, 0x400);
}

#[test]
fn indexed_world_bridge_retains_group_pool_and_conditional_selection() {
    use skate3_core::physics::{
        board_world::{
            BoardWorld,
            query_metadata::{Bounds as WorldBounds, QueryMesh, QueryMetadata, QueryPool},
        },
        drive_frames::RetailAffineTransform,
    };
    let mut triangles = Vec::new();
    let mut meshes = Vec::new();
    for i in 0..256 {
        let original = triangle(if i == 102 { 1. } else { 0. });
        let shift = if (100..=102).contains(&i) {
            0.
        } else {
            1000. + i as f32 * 10.
        };
        let vertices = original
            .triangle
            .vertices
            .map(|v| Vector3::new(v.x + shift, v.y, v.z));
        triangles.push(
            WorldTriangle::from_vertices(vertices, original.material, 123, 0, [0.; 3], 0.).unwrap(),
        );
        meshes.push(QueryMesh {
            triangle_range: i..i + 1,
            local_to_world: RetailAffineTransform::IDENTITY,
            world_to_local: RetailAffineTransform::IDENTITY,
            local_bounds: WorldBounds::from_points(vertices).unwrap(),
            matching_group: if i == 100 { 8 } else { -1 },
            rejection_flags: 0,
            geometry: i as u32,
            pool: match i {
                101 => QueryPool::Island,
                102 => QueryPool::Conditional,
                _ => QueryPool::Ground,
            },
        });
    }
    let mut metadata = QueryMetadata {
        packed_surfaces: (0..256).collect(),
        meshes,
        static_edges: vec![],
        island_flags: 0,
    };
    for (flags, expected) in [(0, 101), (3, 102)] {
        metadata.island_flags = flags;
        let world = BoardWorld::with_query_metadata(triangles.clone(), metadata.clone()).unwrap();
        let hits = with_world_scene(
            &world,
            PrimaryEdges::Normal {
                dynamic: &[],
                vehicles: &[],
            },
            &[],
            |scene| scene.query_lines(&packet()),
        )
        .unwrap();
        assert!(
            hits.iter()
                .all(|hit| hit.unwrap().packed_surface == expected)
        );
    }
}
