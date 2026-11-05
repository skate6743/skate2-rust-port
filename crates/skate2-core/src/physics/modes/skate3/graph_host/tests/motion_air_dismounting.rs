use super::*;
use crate::skate3_native::{graph_host::motion::MotionHost, graph_runtime::{CompiledGraph, LoadedGraph}};
use skate3_core::{animation::{playback::PlaybackContext, output::attributes::AttributePayload}, graph::controller::{Frame, Host}};
use skate_data::{collections::Collections, state_graph::{StateGraph, binding::Binding}};

#[test]
#[ignore = "requires private stock graphs and animation banks"]
fn air_dismounting_stock_crash_node_3207_publishes_only_on_clip_marker() {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let source = StateGraph::load(&root.join("private/stock/data/state/MotionGraph_OnBoard.stategraph")).unwrap();
    let binding = Binding::from_graph(&source).unwrap();
    let runtime = CompiledGraph::from_binding(&binding).unwrap();
    let graph = LoadedGraph { source, binding, runtime };
    let mut host = MotionHost::from_graph(&graph, &Collections::load(&root).unwrap(),
        skate_data::animation_banks::AnimationBanks::load(&root).unwrap().metadata().unwrap(),
        PlaybackContext { is_switch: Some(false), is_mirrored: Some(false), board_available: Some(true),
            pro_skater: encode(b""), transition_override: None }).unwrap();
    // Controlled clip clock isolates this stock behavior from pose evaluation.
    host.animation.current = Some(PlaybackTree::Clip { name: "dismount-test".into(),
        clip: PlaybackClip::new(61.0, 60.0, 1.0, 0, vec![]) });
    host.animation.skater_animation_flags = Some(0x0802_0000);
    let frame = Frame { dt: 1.0 / 120.0, current: None, last: None, state_times: vec![] };
    host.allocate(3207, &frame);
    host.begin(3207, [0; 6], &frame);
    host.update(3207, [0; 6], &frame);
    assert!(host.errors.is_empty(), "{:?}", host.errors);
    assert_eq!(host.animation.air_dismount_revert_frames, None);
    assert_eq!(host.animation.skater_animation_flags, Some(0x0802_0000));
    host.animation.tree_attributes.push(AnimationAttribute { name: encode(b"airdismountrevert"),
        payload: AttributePayload([Some(0); 6]), begin_time: 0.0, end_time: 0.0,
        status: 0, kind: 0, sequence_id: -1 });
    host.animation.seek_current_fraction(0.5);
    host.update(3207, [0; 6], &frame);
    assert_eq!(host.animation.air_dismount_revert_frames.take(), Some(59));
    assert_eq!(host.animation.skater_animation_flags, Some(0x0812_0000));
    host.animation.skater_animation_flags = Some(0x0802_0000); // consumed by physics publication
    host.end(3207, [0; 6], &frame);
    assert_eq!(host.animation.air_dismount_revert_frames, None);
    host.begin(3207, [0; 6], &frame); // reentry captures current remaining duration
    host.update(3207, [0; 6], &frame);
    assert_eq!(host.animation.air_dismount_revert_frames.take(), Some(29));
    host.animation.tree_attributes.clear();
    host.animation.skater_animation_flags = Some(0x0802_0000);
    host.update(3207, [0; 6], &frame);
    assert_eq!(host.animation.air_dismount_revert_frames, None);
    assert_eq!(host.animation.skater_animation_flags, Some(0x0802_0000));
    assert!(host.errors.is_empty(), "{:?}", host.errors);
}
