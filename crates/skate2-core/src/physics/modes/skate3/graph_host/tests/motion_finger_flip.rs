use super::*;

#[test]
#[ignore = "requires private stock graphs and animation banks"]
fn stock_finger_flip_crash_node_3080_lifecycle_and_intent_source() {
    use crate::skate3_native::graph_runtime::CompiledGraph;
    use skate_data::state_graph::{StateGraph, binding::Binding};
    use super::super::motion_stock_gameplay::Operation;
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let source = StateGraph::load(&root.join(
        "private/stock/data/state/MotionGraph_OnBoard.stategraph",
    )).unwrap();
    let binding = Binding::from_graph(&source).unwrap();
    let runtime = CompiledGraph::from_binding(&binding).unwrap();
    let graph = LoadedGraph { source, binding, runtime };
    let mut host = MotionHost::from_graph(
        &graph, &Collections::load(&root).unwrap(),
        skate_data::animation_banks::AnimationBanks::load(&root).unwrap().metadata().unwrap(),
        PlaybackContext { is_switch: Some(false), is_mirrored: Some(false),
            board_available: Some(true), pro_skater: encode(b""), transition_override: None },
    ).unwrap();
    let behavior = 3080;
    let MotionOperation::StockGameplay(Operation::FingerFlipOut { grab_intent }) =
        &host.operations[host.remap.behaviors[behavior]] else {
            panic!("The reported stock behavior must bind FingerFlipOut");
        };
    let intent = grab_intent.clone();
    assert!(!intent.is_empty() && !intent.contains('$'), "authored intent must be resolved");
    let frame = Frame { dt: 0.125, current: None, last: None, state_times: vec![] };
    host.allocate(behavior, &frame);
    host.execute(behavior, &frame, 0).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"Grabbing")), None);

    // Raw AG input cannot substitute for the MG intent read by virtual16.
    host.action_intents.insert(&intent, 1.0);
    host.execute(behavior, &frame, 1).unwrap();
    let released = host.animation.pending_parameter(encode(b"Grabbing")).unwrap();
    assert!((0.0..1.0).contains(&released));
    // Native membership test treats zero as present and reverses the timer.
    host.animation.motion_intents.insert(&intent, 0.0);
    host.execute(behavior, &frame, 1).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"Grabbing")), Some(1.0));
    host.animation.motion_intents.remove(&intent);
    for _ in 0..6 { host.execute(behavior, &frame, 1).unwrap(); }
    assert_eq!(host.animation.pending_parameter(encode(b"Grabbing")), Some(0.0));
    host.execute(behavior, &frame, 2).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"Grabbing")), Some(0.0));
    // Reentry resets private time, but Begin/End do not publish a parameter.
    host.execute(behavior, &frame, 0).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"Grabbing")), Some(0.0));
    host.execute(behavior, &frame, 1).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"Grabbing")), Some(released));

    let second = host.remap.behaviors.iter().enumerate().find_map(|(id, &op)| {
        (id != behavior && matches!(host.operations[op],
            MotionOperation::StockGameplay(Operation::FingerFlipOut { .. }))).then_some(id)
    }).expect("stock graph has separate finger-flip exit nodes");
    host.allocate(second, &frame);
    host.execute(second, &frame, 0).unwrap();
    host.execute(second, &frame, 1).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"Grabbing")), Some(released));
    host.execute(behavior, &frame, 1).unwrap();
    assert!(host.animation.pending_parameter(encode(b"Grabbing")).unwrap() < released);
    assert!(host.animation.motion_attributes.is_empty(), "writes a tree parameter, not a physics packet");
}
