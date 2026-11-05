use super::*;
use skate3_core::physics::{
    board_world::{
        BoardWorld, WorldTriangle,
        query_metadata::{Bounds, QueryMesh, QueryMetadata, QueryPool},
    },
    collision::TriangleFeature,
    contact::RetailContactMaterial,
};

fn world(height: f32, width: f32, depth: f32, ceiling: bool, top: bool) -> BoardWorld {
    let mut triangles = Vec::new();
    let mut quad = |p: [[f32; 3]; 4]| {
        for ids in [[0, 1, 2], [0, 2, 3]] {
            triangles.push(
                WorldTriangle::from_vertices(
                    ids.map(|i| Vector3::new(p[i][0], p[i][1], p[i][2])),
                    RetailContactMaterial {
                        static_friction: 0.8,
                        dynamic_friction: 0.6,
                        restitution: 0.,
                    },
                    0,
                    TriangleFeature::ONE_SIDED,
                    [1.; 3],
                    0.,
                )
                .unwrap(),
            );
        }
    };
    quad([
        [-20., 0., -20.],
        [-20., 0., 20.],
        [20., 0., 20.],
        [20., 0., -20.],
    ]);
    quad([
        [-width, 0., 0.6],
        [-width, height, 0.6],
        [width, height, 0.6],
        [width, 0., 0.6],
    ]);
    if top {
        quad([
            [-width, height, 0.6],
            [-width, height, 0.6 + depth],
            [width, height, 0.6 + depth],
            [width, height, 0.6],
        ]);
    }
    if ceiling {
        quad([
            [-width, height + 1.2, 0.3],
            [width, height + 1.2, 0.3],
            [width, height + 1.2, 3.],
            [-width, height + 1.2, 3.],
        ]);
    }
    let count = triangles.len();
    BoardWorld::with_query_metadata(
        triangles,
        QueryMetadata {
            packed_surfaces: vec![0; count],
            static_edges: vec![],
            island_flags: 0,
            meshes: vec![QueryMesh {
                geometry: 1, rejection_flags: 0,
                triangle_range: 0..count,
                pool: QueryPool::Ground,
                matching_group: 0,
                local_bounds: Bounds {
                    min: Vector3::new(-20., -1., -20.),
                    max: Vector3::new(20., height + 2., 20.),
                },
                local_to_world: RetailAffineTransform::IDENTITY,
                world_to_local: RetailAffineTransform::IDENTITY,
            }],
        },
    )
    .unwrap()
}
#[test]
fn reachable_ledge_needs_width_support_and_headroom() {
    let probe = |w| ledge::find(&w, Vec3::ZERO, Vec3::Z);
    let good = probe(world(2.2, 2., 3., false, true)).expect("reachable ledge");
    assert!((good.anchor.y - 2.225).abs() < 0.002);
    assert!(probe(world(4., 2., 3., false, true)).is_none(), "too high");
    assert!(probe(world(2.2, 2., 3., false, false)).is_none(), "no top");
    assert!(
        probe(world(2.2, 0.2, 3., false, true)).is_none(),
        "too narrow for hands"
    );
    assert!(
        probe(world(2.2, 2., 0.4, false, true)).is_none(),
        "no standing support"
    );
    assert!(probe(world(2.2, 2., 3., true, true)).is_none(), "ceiling");
    assert!(ledge::find(&world(2.2, 2., 3., false, true), Vec3::ZERO, -Vec3::Z).is_none());
}

#[test]
fn default_course_has_a_reachable_test_block() {
    let world = super::super::ground::world(RetailContactMaterial {
        static_friction: 0.8,
        dynamic_friction: 0.6,
        restitution: 0.,
    });
    assert!(
        ledge::find(
            &world,
            Vec3::new(-4.4, super::super::ground::FLOOR_HEIGHT, 0.),
            Vec3::NEG_X
        )
        .is_some()
    );
}

#[test]
#[ignore = "requires private stock graphs and authored climbing clips"]
fn hybrid_climb_holds_for_second_press_and_resumes_walking() {
    run_climb(false, false);
}

#[test]
#[ignore = "requires private stock graphs and authored climbing clips"]
fn climbing_with_a_dropped_board_preserves_free_board_physics() {
    run_climb(true, false);
}

#[test]
#[ignore = "requires private stock graphs and authored climbing clips"]
fn turning_away_during_reach_fades_out_and_lands_normally() {
    run_climb(false, true);
}

fn run_climb(drop_board: bool, miss: bool) {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let assets = skate_data::GameAssets::load(&root).unwrap();
    let graphs = crate::skate3_native::graph_runtime::StockGraphs::load(&root, &assets).unwrap();
    let mut physics =
        Skate3PhysicsState::load_with_difficulty(&root, None, crate::skate3_native::difficulty::Difficulty::Normal)
            .unwrap();
    let mut skater = Skate3ControllerState::load(&root, &graphs, &physics, "normal").unwrap();
    let mut camera = crate::skate3_native::camera::CameraRuntime::load(&root).unwrap();
    let mut controls = PlayerControls::default();
    let mut input = crate::skate3_native::input::ControllerInput::default();
    // Dismount through the real stock controller and graphs before testing.
    for tick in 0..360 {
        input.sample_raw_for_test(skate3_core::input::xbox::XboxState {
            buttons: if tick == 120 { 0x8000 } else { 0 },
            triggers: if drop_board && tick == 180 {
                [255, 0]
            } else {
                [0; 2]
            },
            left: [0; 2],
            right: [0; 2],
        });
        controls.update(
            &mut input.player_actions(),
            physics.settings.step.simulation.time_step,
            physics.settings.input_magnitude_threshold,
            skater.player_input.physical.scoring.capabilities_204,
        );
        super::super::frame::advance(
            &mut physics,
            &mut skater,
            &mut controls,
            &graphs,
            &mut input.player_actions(),
            true,
            &mut camera,
        )
        .unwrap();
    }
    assert_eq!(skater.player_state.current(), PhysicalStateId::BipedGround);
    assert_eq!(
        skater.skateboard_controller.fields.state_448 == 1,
        !drop_board
    );
    physics.world = world(2.2, 4., 8., false, true);
    // Put the real skeleton's feet at the fixture origin before entering Biped.
    let c = &skater.climbing.clips.as_ref().unwrap().reach;
    let g: Vec<_> = skater
        .climbing
        .indices
        .iter()
        .map(|&i| matrix(skater.render_pose[i]))
        .collect();
    let root_matrix = Mat4::from_translation(-c.feet(&g) - Vec3::Z * 2.);
    let pose = skater.render_pose.clone();
    publish_pose(&physics, &mut skater, root_matrix, pose).unwrap();
    super::super::player_state::resume_after_climb(&mut physics, &mut skater).unwrap();
    controls.offboard_direction = Some([0., 0., 1., 0.]);
    let button = |controls: &mut PlayerControls, down: bool, previous: bool| {
        let mut words = *controls.controller.words();
        words[13] = u32::from(down) << 23;
        words[6] = u32::from(previous) << 23;
        controls.controller =
            skate3_core::input::controller::DerivedControllerInput::from_words(words);
    };
    approach::advance(&physics, &mut skater, &controls);
    let mut saw_air = false;
    let mut saw_reach = false;
    let mut saw_ground_reach = false;
    for tick in 0..180 {
        input.sample_raw_for_test(skate3_core::input::xbox::XboxState {
            buttons: if tick >= 33 && tick < 120 { 0x4000 } else { 0 },
            triggers: [0; 2],
            left: [0, 24000],
            right: [0; 2],
        });
        controls.update(
            &mut input.player_actions(),
            physics.settings.step.simulation.time_step,
            physics.settings.input_magnitude_threshold,
            skater.player_input.physical.scoring.capabilities_204,
        );
        controls.offboard_direction = Some([0., 0., if miss && tick >= 45 { -1. } else { 1. }, 0.]);
        super::super::frame::advance(
            &mut physics,
            &mut skater,
            &mut controls,
            &graphs,
            &mut input.player_actions(),
            true,
            &mut camera,
        )
        .unwrap();
        saw_air |= skater.player_state.current() == PhysicalStateId::BipedAir;
        saw_reach |= skater.climbing.approach.is_some();
        saw_ground_reach |= skater.player_state.current() == PhysicalStateId::BipedGround
            && skater
                .climbing
                .approach
                .as_ref()
                .is_some_and(|a| a.weight > 0.02);
        if !miss && [15, 25, 30, 35, 40, 45, 50, 55].contains(&tick) {
            capture(&skater, &format!("00_jump_{tick}"));
        }
        if skater
            .climbing
            .active
            .as_ref()
            .is_some_and(|a| a.phase == Phase::Hang)
        {
            break;
        }
    }
    assert!(saw_air, "must use stock airborne jump before catching");
    assert!(saw_ground_reach, "arms must start reaching during the run");
    if miss {
        assert!(saw_reach, "miss scenario must begin a reach first");
        assert!(skater.climbing.active.is_none());
        assert!(skater.climbing.approach.is_none());
        assert_eq!(skater.player_state.current(), PhysicalStateId::BipedGround);
        return;
    }
    assert!(
        skater.climbing.active.is_some(),
        "stock jump failed to catch"
    );
    assert_eq!(skater.climbing.active.as_ref().unwrap().phase, Phase::Hang);
    let c = &skater.climbing.clips.as_ref().unwrap().reach;
    let g: Vec<_> = skater
        .climbing
        .indices
        .iter()
        .map(|&i| matrix(skater.render_pose[i]))
        .collect();
    let ledge = skater.climbing.active.as_ref().unwrap().ledge;
    let root = matrix(skater.animated_skeleton.roots.animation_to_world);
    for (i, name) in ["LEFTHAND", "RIGHTHAND"].into_iter().enumerate() {
        let hand = root * g[c.index(name)];
        let palm = hand.transform_point3(contacts::PALM);
        assert!(
            palm.distance(ledge.palms[i]) < 0.015,
            "{name}: palm={palm:?} target={:?}",
            ledge.palms[i]
        );
        assert!(hand.y_axis.truncate().dot(-ledge.normals[i]) > 0.99);
        let lower = g[c.index(if i == 0 {
            "LEFTFOREARM"
        } else {
            "RIGHTFOREARM"
        })];
        let relative = Transform::from_matrix(lower.inverse() * g[c.index(name)]);
        let axis = relative.translation.normalize();
        let q = relative.rotation;
        assert!(
            Vec3::new(q.x, q.y, q.z).dot(axis).abs() < 0.01,
            "forearm roll must not invert the wrist: {name} {q:?}"
        );
    }
    let hips = matrix(skater.animated_skeleton.roots.animation_to_world)
        .transform_point3(g[c.index("HIPS")].w_axis.truncate());
    assert!(
        hips.z < 0.45,
        "hanging hips must stay outside the wall: {hips:?}"
    );
    capture(&skater, "02_hanging");
    button(&mut controls, false, true);
    advance(&mut physics, &mut skater, &controls, &mut camera).unwrap();
    button(&mut controls, true, false);
    advance(&mut physics, &mut skater, &controls, &mut camera).unwrap();
    assert_eq!(
        skater.climbing.active.as_ref().unwrap().phase,
        Phase::Mantle
    );
    button(&mut controls, false, true);
    for tick in 0..260 {
        if !advance(&mut physics, &mut skater, &controls, &mut camera).unwrap() {
            break;
        }
        assert!(
            skater
                .render_pose
                .iter()
                .flatten()
                .flatten()
                .all(|v| v.is_finite())
        );
        if [30, 90, 150, 210].contains(&tick) {
            capture(&skater, &format!("03_mantle_{tick}"));
        }
    }
    assert!(skater.climbing.active.is_none());
    assert_eq!(skater.player_state.current(), PhysicalStateId::BipedGround);
    let landing = matrix(skater.animated_skeleton.roots.animation_to_world)
        .w_axis
        .truncate();
    capture(&skater, "04_handoff");
    input.sample_raw_for_test(skate3_core::input::xbox::XboxState {
        buttons: 0,
        triggers: [0; 2],
        left: [0; 2],
        right: [0; 2],
    });
    controls.offboard_direction = Some([0.; 4]);
    for tick in 0..120 {
        controls.update(
            &mut input.player_actions(),
            physics.settings.step.simulation.time_step,
            physics.settings.input_magnitude_threshold,
            skater.player_input.physical.scoring.capabilities_204,
        );
        super::super::frame::advance(
            &mut physics,
            &mut skater,
            &mut controls,
            &graphs,
            &mut input.player_actions(),
            false,
            &mut camera,
        )
        .unwrap();
        assert_eq!(
            skater.player_state.current(),
            PhysicalStateId::BipedGround,
            "unexpected state after handoff at tick {tick}"
        );
    }
    let end = matrix(skater.animated_skeleton.roots.animation_to_world)
        .w_axis
        .truncate();
    assert!(
        end.distance(landing) < 0.1,
        "handoff drifted: {landing:?} -> {end:?}"
    );
    assert_eq!(skater.player_state.current(), PhysicalStateId::BipedGround);
    capture(&skater, "05_standing");
    for _ in 0..60 {
        input.sample_raw_for_test(skate3_core::input::xbox::XboxState {
            buttons: 0,
            triggers: [0; 2],
            left: [0, 16000],
            right: [0; 2],
        });
        controls.update(
            &mut input.player_actions(),
            physics.settings.step.simulation.time_step,
            physics.settings.input_magnitude_threshold,
            skater.player_input.physical.scoring.capabilities_204,
        );
        controls.offboard_direction = Some([0., 0., 0.5, 0.]);
        super::super::frame::advance(
            &mut physics,
            &mut skater,
            &mut controls,
            &graphs,
            &mut input.player_actions(),
            true,
            &mut camera,
        )
        .unwrap();
    }
    let walked = matrix(skater.animated_skeleton.roots.animation_to_world)
        .w_axis
        .truncate();
    assert!(
        walked.z > end.z + 0.2,
        "walking did not resume: {end:?} -> {walked:?}"
    );
    assert!(
        (walked.y - 2.2).abs() < 0.15,
        "walking left the ledge: {walked:?}"
    );
    assert_eq!(skater.player_state.current(), PhysicalStateId::BipedGround);
}

fn capture(skater: &Skate3ControllerState, name: &str) {
    if skater.skateboard_controller.fields.state_448 != 1 {
        return;
    }
    if let Some(path) = std::env::var_os("SKATE_CLIMB_CAPTURE") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join(format!("{name}.json")),
            serde_json::to_vec(&serde_json::json!({
                "root":skater.animated_skeleton.roots.animation_to_world,
                "names":skater.animation.evaluator.frames.bone_names,
                "pose":skater.render_pose,
                "board_state":skater.skateboard_controller.fields.state_448,
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
