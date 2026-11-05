use super::*;

#[test]
#[ignore = "requires private stock graphs and animation banks"]
fn stock_jump_into_seeks_once_per_entry_and_consumes_misses() {
    use crate::skate3_native::graph_runtime::CompiledGraph;
    use skate3_core::animation::playback::{PlaybackRequest, PlaybackService, TransitionSettings};
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
    host.animation.skater_animation_flags = Some(0);
    let behavior = host.remap.behaviors.iter().position(|&id| matches!(
        &host.operations[id], MotionOperation::StockGameplay(
            super::super::motion_stock_gameplay::Operation::JumpInto { attribute }
        ) if *attribute == encode(b"manualInto")
    )).unwrap();
    let frame = Frame { dt: 1.0 / 60.0, current: None, last: None, state_times: vec![] };
    let request = PlaybackRequest {
        animation: "B_HARDFLIP_G".into(), speed: 1.0, start_time: 0.0,
        transition: TransitionSettings { kind: 1, seconds: 0.0, under: 0,
            matching: 0, use_channels_from_weights: false },
    };
    let expected = host.animation.build_tree("B_HARDFLIP_G").unwrap()
        .attribute(encode(b"manualInto"), 31).unwrap().unwrap().begin_time;
    assert!(expected.is_finite(), "stock manualInto marker must have a finite begin time");
    for _ in 0..2 {
        host.animation.play(request.clone()).unwrap();
        let handle = host.allocate(behavior, &frame);
        host.execute(behavior, &frame, 0).unwrap();
        assert_eq!(host.animation.current_time().unwrap(), 0.0);
        host.execute(behavior, &frame, 1).unwrap();
        assert_eq!(host.animation.current_time().unwrap(), expected);
        host.animation.advance(frame.dt, 0.0);
        let advanced = host.animation.current_time().unwrap();
        host.execute(behavior, &frame, 1).unwrap();
        assert_eq!(host.animation.current_time().unwrap(), advanced);
        host.execute(behavior, &frame, 2).unwrap();
        host.release(handle);
        assert!(host.animation.motion_attributes.is_empty());
    }
    // A first-update miss also consumes the native instance latch.
    let op = host.remap.behaviors[behavior];
    host.operations[op] = MotionOperation::StockGameplay(
        super::super::motion_stock_gameplay::Operation::JumpInto {
            attribute: encode(b"missing-marker"),
        },
    );
    host.animation.play(request).unwrap();
    host.allocate(behavior, &frame);
    host.execute(behavior, &frame, 1).unwrap();
    assert!(matches!(host.instances[behavior], Instance::JumpInto { first_update: false }));
    host.operations[op] = MotionOperation::StockGameplay(
        super::super::motion_stock_gameplay::Operation::JumpInto { attribute: encode(b"manualInto") },
    );
    host.execute(behavior, &frame, 1).unwrap();
    assert_eq!(host.animation.current_time().unwrap(), 0.0);
}
