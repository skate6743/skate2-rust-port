//! Real owner/helper integration with private stock construction. Synthetic
//! requests isolate latch semantics; these are not a recorded failure replay.
use super::{Skate3PhysicsState, Skate3ControllerState};
use crate::skate3_native::physics::skeleton_input_runtime::SkeletonOwners;
use skate3_core::player::offboard::board_possession::lifecycle::Effects as _;

fn stock() -> (Skate3PhysicsState, Skate3ControllerState) {
    let root = std::env::var_os("SKATE3_ASSET_ROOT").expect("set SKATE3_ASSET_ROOT");
    let root = std::path::Path::new(&root);
    let assets = skate_data::GameAssets::load(root).unwrap();
    let graphs = crate::skate3_native::graph_runtime::StockGraphs::load(root, &assets).unwrap();
    let physics = Skate3PhysicsState::load(root).unwrap();
    let skater = Skate3ControllerState::load(root, &graphs, &physics, "normal").unwrap();
    (physics, skater)
}

#[test]
#[ignore = "requires private stock skater, graph and collection assets"]
fn canonical_correction_consumes_once_and_retains_suppressed_requests() {
    let (_, mut skater) = stock();
    let body = &mut skater.skeleton;
    let correction = &mut skater.skeleton_output.correction;
    let up = [0.0, 1.0, 0.0, 0.0];
    correction.observe_board([0.25, 0.0, 0.0, 0.0], [0.0; 4]);
    correction.pending = true;
    let before = body.record.pose;
    let rates = body.bodies().map(|b| b.rates);
    correction.apply(body, up, false, 1 << 18, 0);
    assert!(correction.pending);
    assert_eq!(body.record.pose, before);
    correction.apply(body, up, true, 0, 0);
    assert!(correction.pending);
    assert_eq!(body.record.pose, before);
    correction.apply(body, up, false, 0, 0);
    assert!(!correction.pending);
    assert_eq!(body.record.pose[0], before[0]);
    for part in 1..24 {
        assert_eq!(body.record.pose[part][3][0], before[part][3][0] + 0.25);
    }
    assert_eq!(body.bodies().map(|b| b.rates), rates);
    let consumed = body.record.pose;
    // No producer write: a later tick cannot resurrect the consumed request.
    correction.apply(body, up, false, 0, 0);
    assert_eq!(body.record.pose, consumed);
    correction.pending = true;
    correction.observe_board([2.0, 0.0, 0.0, 0.0], [0.0; 4]);
    correction.apply(body, up, true, 0, 1 << 30);
    assert!(
        !correction.pending,
        "special wipeout clears even when too large to move"
    );
    assert_eq!(body.record.pose, consumed);
}

#[test]
#[ignore = "requires private stock skater, graph and collection assets"]
fn physical_pose_reset_clears_the_canonical_correction_request() {
    let (physics, mut skater) = stock();
    skater.skeleton_output.correction.pending = true;
    let mut owners = SkeletonOwners {
        animated: &mut skater.animated_skeleton,
        body: &mut skater.skeleton,
        drives: &mut skater.skeleton_drives,
        ik: &mut skater.foot_ik,
        animation_input: &mut skater.animation_input,
        correction: &mut skater.skeleton_output.correction,
        pose_errors: &mut skater.pose_errors,
    };
    skater
        .skeleton_input
        .reset_physical_pose(&mut owners, physics.settings.step.simulation);
    assert!(!skater.skeleton_output.correction.pending);
}

#[test]
#[ignore = "requires private stock skater, graph and collection assets"]
fn ground_entry_restores_stock_materials_and_volumes_after_release() {
    let (mut physics, mut skater) = stock();
    let standard = [
        physics.settings.standard_wheel_material,
        physics.settings.truck_material,
        physics.settings.deck_material,
    ];
    let p = &skater.player_input.processed;
    skater.player_input.toolkit = Some(
        skate3_core::physics::board_toolkit::BoardToolkit::from_board(
            &physics.board,
            p.flags_2468,
            p.scalar_2612,
            p.vectors_464_480_496_512_528[0].map(f32::from_bits),
            [0.0, 1.0, 0.0, 0.0],
        ),
    );
    {
        let mut effects = skater.board_possession_live.effects(
            &mut physics,
            &mut skater.ground_lifecycle.board_animated_290,
            skater.player_input.processed.timestep_2604,
        );
        effects.released_board();
        effects.collision_volumes(false);
        effects.enable_animation_soft();
    }
    skater.board_possession_live.publish_volumes(&mut physics);
    assert!(physics.board_wiping_out);
    assert_eq!(physics.board.collision_group(), 7);
    super::enter(&mut physics, &mut skater).unwrap();
    assert!(!physics.board_wiping_out);
    assert_eq!(physics.board.collision_group(), 4);
    assert_eq!(
        [
            physics.settings.wheel_material,
            physics.settings.truck_material,
            physics.settings.deck_material
        ],
        standard
    );
    let volumes = &skater.board_possession_live.volumes;
    assert!(volumes.deck && volumes.trucks && volumes.wheels);
    assert!(volumes.deck_children.iter().all(|&enabled| enabled));
    assert!(physics.settings.truck_collisions);
    assert!(
        physics
            .settings
            .deck_geometry
            .children
            .iter()
            .all(|c| c.collision_enabled)
    );
    assert_eq!(skater.ground_lifecycle.board_animated_290, 0);
    assert_eq!(
        physics.board.hook().drive.dynamics,
        [0, 0, 0, 2, 0, 0, 0, 2]
    );
}
