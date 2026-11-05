use super::*;
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{
            BoardWorld, WorldTriangle,
            query_metadata::{Bounds, QueryMesh, QueryMetadata, QueryPool},
        },
        contact::RetailContactMaterial,
        drive_frames::RetailAffineTransform,
    },
    player::offboard::contact_toolkit::{Input, ProbeLayout, Scene},
};
fn plane(rejection_flags: u32) -> BoardWorld {
    let vertices = [
        Vector3::new(-10., 0., -10.),
        Vector3::new(-10., 0., 10.),
        Vector3::new(10., 0., 0.),
    ];
    let material = RetailContactMaterial {
        static_friction: 0.,
        dynamic_friction: 0.,
        restitution: 0.,
    };
    let triangle = WorldTriangle::from_vertices(vertices, material, 999, 0, [0.; 3], 0.).unwrap();
    BoardWorld::with_query_metadata(
        vec![triangle],
        QueryMetadata {
            packed_surfaces: vec![17],
            meshes: vec![QueryMesh {
                triangle_range: 0..1,
                local_to_world: RetailAffineTransform::IDENTITY,
                world_to_local: RetailAffineTransform::IDENTITY,
                local_bounds: Bounds::from_points(vertices).unwrap(),
                matching_group: -1,
                rejection_flags,
                geometry: 73,
                pool: QueryPool::Ground,
            }],
            static_edges: vec![],
            island_flags: 0,
        },
    )
    .unwrap()
}
fn input() -> Input {
    Input::from_vectors([
        [0.; 4],
        [0., 0., 1., 0.],
        [0., 1., 0., 0.],
        [1., 0., 0., 0.],
        [0.; 4],
        [0., 1., 0., 0.],
        [1., 0., 0., 0.],
    ])
}
#[test]
fn actual_world_hit_preserves_mesh_identity_and_packed_surface() {
    let world = plane(0);
    let scene = StaticScene::new(&world).unwrap();
    let result = scene
        .execute(&ProbeLayout::stock().prepare(input(), -1))
        .unwrap();
    assert!(result.trajectories[0].valid());
    assert_eq!(result.trajectories[0].geometry, 73);
    assert_eq!(result.trajectories[0].surface, 17);
    assert!((result.trajectories[0].landing_normal[1] - 1.).abs() < 1e-5);
    assert_eq!(result.lines.len(), 44);
}

#[test]
fn nonfinite_unused_lane_preserves_sweep_hits_but_invalid_xyz_still_fails() {
    let world = plane(0);
    let scene = StaticScene::new(&world).unwrap();
    let mut batch = ProbeLayout::stock().prepare(input(), -1);
    let expected = scene.execute(&batch).unwrap();
    // Reported crash: finite spatial endpoints, start.w=inf, end.w=NaN.
    for request in &mut batch.trajectories {
        request.trajectory.position[3] = f32::INFINITY;
        request.trajectory.velocity[3] = f32::NEG_INFINITY;
    }
    for line in &mut batch.lines {
        line.start[3] = f32::INFINITY;
        line.end[3] = f32::NAN;
    }
    let actual = scene.execute(&batch).unwrap();
    assert_eq!(actual.trajectories, expected.trajectories);
    assert_eq!(actual.lines, expected.lines);
    assert_eq!(actual.edges, expected.edges);
    for axis in 0..3 {
        let mut invalid = batch.clone();
        invalid.lines[0].start[axis] = f32::NAN;
        assert!(scene.execute(&invalid).is_err());
        invalid.lines[0].start[axis] = 0.;
        invalid.lines[0].end[axis] = f32::INFINITY;
        assert!(scene.execute(&invalid).is_err());
    }
    for radius in [-1., f32::NAN, f32::INFINITY] {
        let mut invalid = batch.clone();
        invalid.lines[0].radius = radius;
        assert!(scene.execute(&invalid).is_err());
    }
}
#[test]
fn mesh_filter_is_not_a_board_flag_or_material_tag() {
    let world = plane(0x2000);
    let scene = StaticScene::new(&world).unwrap();
    let result = scene
        .execute(&ProbeLayout::stock().prepare(input(), -1))
        .unwrap();
    assert!(!result.trajectories[0].valid());
    assert!(result.lines.iter().all(Option::is_none));
}

#[test]
fn canonical_world_reaches_completed_analyzer_without_substitute_support() {
    let world = plane(0);
    let scene = StaticScene::new(&world).unwrap();
    let mut owner = Owner::default();
    assert!(owner.refresh().is_none());
    owner.submit(input(), -1, &scene).unwrap();
    let completed = owner.refresh().unwrap();
    assert_eq!(completed.prefix.flags_176 & 1, 1);
    assert_eq!(completed.prefix.support_180, 73);
    assert!(completed.prefix.position[1].abs() < 1e-5);
    assert!(completed.prefix.normal[1] > 0.99);
    assert!(completed.samples.original_ground_count > 0);
    assert!(!owner.has_pending());

    // Move the query outside the authored collision mesh. Previous support
    // cannot be retained as a fabricated surface after the real query misses.
    let mut airborne = input();
    airborne.position[1] = 10.;
    owner.submit(airborne, -1, &scene).unwrap();
    let missed = owner.refresh().unwrap();
    assert_eq!(missed.prefix.flags_176 & 1, 0);
    assert_eq!(missed.prefix.support_180, 0);
    assert!(missed.results.trajectories.iter().all(|hit| !hit.valid()));
}

#[test]
fn toolkit_refuses_world_without_authored_query_metadata() {
    let unannotated = BoardWorld::new(Vec::new());
    assert!(StaticScene::new(&unannotated).is_err());
}

#[test]
fn embedded_static_rwcm_hits_distinct_actor_query_ids() {
    let mut map = skate_data::skate_map::SkateMap::parse(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../maps/format-demo.skate"
    )))
    .unwrap();
    map.geometry.collision.clear();
    map.extensions.push(skate_data::skate_map::Extension {
        tag: *b"RWCM",
        schema: 1,
        payload: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../skate-data/tests/fixtures/retail-collision.rwcmset"
        ))
        .to_vec(),
    });
    let world = crate::skate3_native::skate_world::collision_world(
        &map,
        RetailContactMaterial {
            static_friction: 0.,
            dynamic_friction: 0.,
            restitution: 1.,
        },
    )
    .unwrap();
    assert!(
        world
            .query_metadata()
            .unwrap()
            .meshes
            .iter()
            .all(|mesh| mesh.matching_group == -1 && mesh.geometry == 0)
    );
    let tri = &world.triangles()[1].triangle;
    let [a, b, c] = tri.vertices;
    let centre = Vector3::new(
        (a.x + b.x + c.x) / 3.,
        (a.y + b.y + c.y) / 3.,
        (a.z + b.z + c.z) / 3.,
    );
    let normal = tri.feature.normal;
    let line = skate3_core::player::offboard::ground_query::Line {
        start: Vector3::new(
            centre.x + normal.x,
            centre.y + normal.y,
            centre.z + normal.z,
        ),
        end: Vector3::new(
            centre.x - normal.x,
            centre.y - normal.y,
            centre.z - normal.z,
        ),
        radius: 0.,
    };
    let scene = StaticScene::new(&world).unwrap();
    //Native static registration uses mesh matchingID-1, not packed unit group0x1234.
    for actor in [0, 0x1234, 73] {
        assert!(
            scene.lines(&[line], actor).unwrap()[0].is_some(),
            "Static RWCM unit group incorrectly filtered actor{actor}"
        );
    }
}

#[test]
fn indexed_clusters_preserve_rejection_identity_surface_and_pool_ties() {
    let source = plane(0);
    let mut triangles = Vec::new();
    let mut metadata = source.query_metadata().unwrap().clone();
    metadata.meshes.clear();
    metadata.packed_surfaces.clear();
    for i in 0..256 {
        let shift = if (100..=102).contains(&i) {
            0.
        } else {
            1000. + i as f32 * 30.
        };
        let original = source.triangles()[0];
        let vertices = original
            .triangle
            .vertices
            .map(|v| Vector3::new(v.x + shift, v.y, v.z));
        triangles.push(
            WorldTriangle::from_vertices(vertices, original.material, 999, 0, [0.; 3], 0.).unwrap(),
        );
        let mut mesh = source.query_metadata().unwrap().meshes[0].clone();
        mesh.triangle_range = i..i + 1;
        mesh.local_bounds = Bounds::from_points(vertices).unwrap();
        mesh.geometry = 7000 + i as u32;
        mesh.rejection_flags = if i == 100 { 0x2000 } else { 0 };
        mesh.pool = if i == 102 {
            QueryPool::Island
        } else {
            QueryPool::Ground
        };
        metadata.meshes.push(mesh);
        metadata.packed_surfaces.push(i as u16);
    }
    let world = BoardWorld::with_query_metadata(triangles, metadata).unwrap();
    let scene = StaticScene::new(&world).unwrap();
    let result = scene
        .execute(&ProbeLayout::stock().prepare(input(), -1))
        .unwrap();
    assert!(result.trajectories[0].valid());
    assert_eq!(result.trajectories[0].geometry, 7101);
    assert_eq!(result.trajectories[0].surface, 101);
    for hit in result.lines.iter().flatten() {
        assert_eq!(hit.geometry, 7101);
        assert_eq!(hit.surface, 101);
    }
}
