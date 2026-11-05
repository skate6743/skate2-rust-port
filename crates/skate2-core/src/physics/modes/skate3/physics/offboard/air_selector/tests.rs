use super::*;
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{
            WorldTriangle,
            query_metadata::{Bounds, QueryMesh, QueryMetadata, QueryPool},
        },
        contact::RetailContactMaterial,
        drive_frames::RetailAffineTransform,
    },
    player::offboard::{air_selector::TrajectoryResult, ground_query::Line},
    point_graph::PointGraph,
};
fn world(pools: &[(QueryPool, i32, u16)], island_flags: u32) -> BoardWorld {
    let vertices = [
        Vector3::new(-20., 0., -20.),
        Vector3::new(-20., 0., 20.),
        Vector3::new(20., 0., 0.),
    ];
    let material = RetailContactMaterial {
        static_friction: 0.6,
        dynamic_friction: 0.4,
        restitution: 0.1,
    };
    let triangles = pools
        .iter()
        .map(|_| WorldTriangle::from_vertices(vertices, material, 999, 0, [0.; 3], 0.).unwrap())
        .collect();
    let meshes = pools
        .iter()
        .enumerate()
        .map(|(i, &(pool, group, _))| QueryMesh {
            triangle_range: i..i + 1,
            local_to_world: RetailAffineTransform::IDENTITY,
            world_to_local: RetailAffineTransform::IDENTITY,
            local_bounds: Bounds::from_points(vertices).unwrap(),
            matching_group: group,
            rejection_flags: 0x6000,
            geometry: i as u32,
            pool,
        })
        .collect();
    BoardWorld::with_query_metadata(
        triangles,
        QueryMetadata {
            packed_surfaces: pools.iter().map(|p| p.2).collect(),
            meshes,
            static_edges: vec![],
            island_flags,
        },
    )
    .unwrap()
}
fn context() -> Context {
    Context {
        selection_flags_2948: 0,
        matching_group_2952: -1,
        up_544: [0., 1., 0., 0.],
        forward_224: [0., 0., 1., 0.],
    }
}
fn owner() -> AirSelector {
    AirSelector::new(Settings {
        deck_center_to_truck: 0.3,
        query: core::Settings {
            height: 0.9,
            sphere_radius: 0.3,
            start_index: 0,
        },
        blend: PointGraph {
            x: [0.; 8],
            y: [1.; 8],
        },
    })
}
fn packet() -> Packet {
    let mut p = Packet::initialized(0.);
    p.position_32 = [0., 2., 0., 0.];
    p.velocity_0 = [0., 0., 3., 0.];
    p.forward_64 = [0., 0., 1., 0.];
    p
}
#[test]
fn six_real_lines_keep_native_pool_ties_and_exact_surfaces() {
    let world = world(
        &[(QueryPool::Ground, 2, 0x411), (QueryPool::Island, 2, 0x722)],
        0,
    );
    let scene = StaticScene::new(&world).unwrap();
    let line = Line {
        start: Vector3::new(0., 1., 0.),
        end: Vector3::new(0., -1., 0.),
        radius: 0.,
    };
    let hits = scene.lines(&[line; 6], 2).unwrap();
    assert_eq!(hits.len(), 6);
    for hit in hits {
        let hit = hit.unwrap();
        assert_eq!(hit.packed_surface, 0x411);
        assert!((hit.fraction - 0.5).abs() < 1e-5);
    }
    assert!(
        scene
            .lines(&[line; 6], 3)
            .unwrap()
            .iter()
            .all(Option::is_none)
    );
    //Mesh0x6000 does not reject these lines; unlike trajectory their mask is0.
}
#[test]
fn conditional_pool_requires_exact_island_three() {
    let line = Line {
        start: Vector3::new(0., 1., 0.),
        end: Vector3::new(0., -1., 0.),
        radius: 0.,
    };
    for flags in [0, 1, 2, 3, 7] {
        let world = world(&[(QueryPool::Conditional, -1, 0x845)], flags);
        let result = StaticScene::new(&world)
            .unwrap()
            .lines(&[line], -1)
            .unwrap();
        assert_eq!(result[0].is_some(), flags == 3);
    }
}
#[test]
fn line_rejects_nonfinite_input_without_synthesizing_a_hit() {
    let world = world(&[(QueryPool::Ground, -1, 1)], 0);
    let scene = StaticScene::new(&world).unwrap();
    let line = Line {
        start: Vector3::new(f32::NAN, 1., 0.),
        end: Vector3::ZERO,
        radius: 0.,
    };
    assert!(scene.lines(&[line], -1).is_err());
    assert!(scene.lines(&[], -1).unwrap().is_empty());
}
#[test]
fn synchronous_results_remain_staged_until_native_consume() {
    let world = world(&[(QueryPool::Ground, -1, 1)], 0);
    let mut owner = owner();
    let mut out = TrajectoryResult::default();
    owner
        .launch(&world, packet(), [0., -9.8, 0., 0.], context())
        .unwrap();
    assert!(owner.core.sampling.pending_8492);
    owner.sample(0, core::DT, &mut out);
    assert!(!out.valid_404);
    assert_eq!(out.position_272, packet().position_32);
    assert_eq!(owner.consume(&world, &[], context(), 0.4).unwrap(), Some(0));
    assert!(!owner.core.sampling.pending_8492);
    assert!(owner.core.sampling.preinitialized_8494);
    //The actual mask0x6000 rejects the plane in trajectory requests.
    assert!(!owner.core.selected_candidate.valid_120);
    assert!(owner.requery(&world, context(), 0, 0).unwrap());
    assert!(owner.core.requery_pending_8499);
    owner.consume(&world, &[], context(), 0.4).unwrap();
    assert!(!owner.core.requery_pending_8499);
    assert!(owner.core.sampling.restart_allowed_8493);
    assert!(!owner.core.sampling.preinitialized_8494);
}
#[test]
fn failed_world_submission_preserves_owner_and_never_returns_fake_completion() {
    let world = BoardWorld::new(vec![]);
    let mut owner = owner();
    assert!(
        owner
            .launch(&world, packet(), [0., -9.8, 0., 0.], context())
            .is_err()
    );
    assert!(!owner.core.sampling.pending_8492);
    assert!(owner.completed_launch.is_none());
    assert!(owner.completed_requery.is_none());
}
#[test]
fn launch_and_requery_share_slot_zero_and_consume_in_native_order() {
    let world = world(&[(QueryPool::Ground, -1, 1)], 0);
    let mut owner = owner();
    owner
        .launch(&world, packet(), [0., -9.8, 0., 0.], context())
        .unwrap();
    assert!(owner.requery(&world, context(), 0, 0).unwrap());
    assert!(owner.core.sampling.pending_8492 && owner.core.requery_pending_8499);
    assert!(owner.completed_launch.is_some() && owner.completed_requery.is_some());
    assert!(!owner.requery(&world, context(), 0, 0).unwrap());
    assert_eq!(owner.consume(&world, &[], context(), 0.3).unwrap(), Some(0));
    assert!(!owner.core.sampling.pending_8492 && !owner.core.requery_pending_8499);
    assert!(owner.completed_launch.is_none() && owner.completed_requery.is_none());
    assert!(owner.core.sampling.restart_allowed_8493);
}
#[test]
fn native_suppression_returns_before_pending_host_completion() {
    let world = world(&[(QueryPool::Ground, -1, 1)], 0);
    let mut owner = owner();
    owner
        .launch(&world, packet(), [0., -9.8, 0., 0.], context())
        .unwrap();
    assert!(!owner.requery(&world, context(), 0x10000000, 0).unwrap());
    assert!(!owner.requery(&world, context(), 0, 0x400000).unwrap());
    assert!(owner.core.sampling.pending_8492);
}
