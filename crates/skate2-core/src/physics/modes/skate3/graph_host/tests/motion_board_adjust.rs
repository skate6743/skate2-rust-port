use super::*;

#[test]
#[ignore = "requires private stock graphs and animation banks"]
fn board_adjust_stock_release_keeps_the_authored_air_blend() {
    use crate::skate3_native::graph_runtime::CompiledGraph;
    use skate_data::state_graph::{StateGraph, binding::Binding};
    use skate3_core::animation::playback_tree::{Evaluation, PoseCommand};
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let source = StateGraph::load(&root.join("private/stock/data/state/MotionGraph_OnBoard.stategraph")).unwrap();
    let binding = Binding::from_graph(&source).unwrap();
    let runtime = CompiledGraph::from_binding(&binding).unwrap();
    let graph = LoadedGraph { source, binding, runtime };
    let mut host = MotionHost::from_graph(&graph, &Collections::load(&root).unwrap(),
        skate_data::animation_banks::AnimationBanks::load(&root).unwrap().metadata().unwrap(),
        PlaybackContext { is_switch: Some(false), is_mirrored: Some(false),
            board_available: Some(true), pro_skater: encode(b""), transition_override: None }).unwrap();
    host.animation.skater_animation_flags = Some(0);
    let find = |predicate: &dyn Fn(&MotionOperation) -> bool| {
        host.remap.behaviors.iter().position(|&op| predicate(&host.operations[op])).unwrap()
    };
    let adjust = find(&|op| matches!(op, MotionOperation::Play(p) if p.animation == "BLEND_ADJUST_RIGHT"));
    let air = find(&|op| matches!(op, MotionOperation::Play(p) if p.animation == "B_AIR_CYC" && p.transition.kind == 2 && p.transition.under == 1));
    let filters = ["RightBoardAdjustMag", "RightBoardAdjustAngle"].map(|name|
        find(&|op| matches!(op, MotionOperation::IntentFilter(p) if p.intent == name && p.settings.blend_out == Some(0.08))));
    let frame = Frame { dt: 1.0 / 60.0, current: None, last: None, state_times: vec![] };
    for id in filters { host.allocate(id, &frame); host.execute(id, &frame, 0).unwrap(); }
    host.allocate(adjust, &frame);
    host.execute(adjust, &frame, 0).unwrap();
    for _ in 0..60 {
        host.animation.advance(frame.dt, 0.0);
        host.animation.motion_intents.insert("RightBoardAdjustMag", 0.9);
        host.animation.motion_intents.insert("RightBoardAdjustAngle", 0.6);
        for id in filters { host.execute(id, &frame, 1).unwrap(); }
        host.execute(adjust, &frame, 1).unwrap();
        host.animation.apply_parameters().unwrap();
    }
    host.animation.motion_intents.remove("RightBoardAdjustMag");
    host.animation.motion_intents.remove("RightBoardAdjustAngle");
    let mut reached_exit = false;
    for _ in 0..120 {
        host.animation.advance(frame.dt, 0.0);
        for id in filters { host.execute(id, &frame, 1).unwrap(); }
        host.execute(adjust, &frame, 1).unwrap();
        host.animation.apply_parameters().unwrap();
        if ["BoardAdjustMag", "BoardAdjustAngle"].iter().all(|name|
            host.animation.filtered_intents.get(name).unwrap().abs() < 0.4) {
            reached_exit = true;
            break;
        }
    }
    assert!(reached_exit);
    host.animation.refresh_tree_attributes().unwrap();
    host.execute(adjust, &frame, 2).unwrap();
    for id in filters { host.execute(id, &frame, 2).unwrap(); }
    host.allocate(air, &frame);
    host.execute(air, &frame, 0).unwrap();
    host.animation.apply_parameters().unwrap();
    let evaluation = Evaluation { cull_threshold: 0.0, update_history: false };
    let commands = host.animation.evaluate_pose(evaluation).unwrap();
    assert!(matches!(commands.last(), Some(PoseCommand::Blend { weight }) if *weight == 0.0), "new air tree must not replace the outgoing pose: {commands:?}");
    let mut previous = 0.0;
    for _ in 0..12 {
        host.animation.advance(frame.dt, 0.0);
        let commands = host.animation.evaluate_pose(evaluation).unwrap();
        let Some(PoseCommand::Blend { weight }) = commands.last() else { panic!("missing authored 0.2s blend") };
        assert!(*weight >= previous && *weight <= 1.0);
        previous = *weight;
    }
    assert!(previous > 0.999);
}
