use super::*;

#[test]
#[ignore = "requires private stock graphs and animation banks"]
fn stock_deck_angles_copy_completed_output_only_on_begin() {
    use crate::skate3_native::graph_runtime::CompiledGraph;
    use skate_data::state_graph::{StateGraph, binding::Binding};
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
    let behavior = host.remap.behaviors.iter().position(|&id| matches!(
        host.operations[id], MotionOperation::SetDeckPitchAndYaw { .. }
    )).expect("stock graph must contain the reported behavior");
    let op = host.remap.behaviors[behavior];
    assert_eq!(host.operations[op], MotionOperation::SetDeckPitchAndYaw {
        yaw: encode(b"skateyaw"), pitch: encode(b"skatepitch"),
    });
    let frame = Frame { dt: 1.0 / 60.0, current: None, last: None, state_times: vec![] };
    let handle = host.allocate(behavior, &frame);
    assert!(host.execute(behavior, &frame, 0).is_err());
    let mut output = skate3_core::player::input_phase::SkeletonOutputFields::default();
    output.publish_deck_angles([1.0, 0.5, 1.0, 0.0], true);
    let values = [output.deck_yaw_536, output.deck_pitch_540];
    host.deck_yaw_pitch = Some(values);
    host.execute(behavior, &frame, 0).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"skateyaw")), Some(values[0]));
    assert_eq!(host.animation.pending_parameter(encode(b"skatepitch")), Some(values[1]));
    host.deck_yaw_pitch = Some([9.0, 8.0]);
    host.execute(behavior, &frame, 1).unwrap();
    host.execute(behavior, &frame, 2).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"skateyaw")), Some(values[0]));
    assert_eq!(host.animation.pending_parameter(encode(b"skatepitch")), Some(values[1]));
    host.release(handle);
    host.allocate(behavior, &frame);
    host.execute(behavior, &frame, 0).unwrap();
    assert_eq!(host.animation.pending_parameter(encode(b"skateyaw")), Some(9.0));
    assert_eq!(host.animation.pending_parameter(encode(b"skatepitch")), Some(8.0));
}
