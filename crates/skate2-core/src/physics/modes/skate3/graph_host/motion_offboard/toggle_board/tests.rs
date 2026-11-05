use super::*;
use skate3_core::animation::{playback_clip::PlaybackClip, playback_tree::PlaybackTree};
use skate3_core::player::offboard::toggle_board::Phase;

fn animation() -> MotionAnimation {
    MotionAnimation::from_metadata(skate_data::animation_metadata::AnimationMetadata::parse(
        r#"{"version":1,"source_bank":"test-only","source_sha256":"0000000000000000000000000000000000000000000000000000000000000000","source_bytes":48,"clips":[],"unsupported_trees":[]}"#,
    ).unwrap())
}
fn physical() -> Physical {
    Physical {
        grabbing_object: false,
        holding_board: true,
        free_board: false,
        retrieval_blocked: false,
        retrieval_active: false,
        yaw_radians: 0.0,
        pitch_radians: 0.0,
    }
}
#[test]
fn dropping_terminal_tick_publishes_physical_attribute_and_animation_parameter() {
    let mut state = State::default();
    state.phase = Phase::DropPlaying;
    let mut animation = animation();
    execute(
        &mut state,
        &mut animation,
        BoardControls::default(),
        Some(physical()),
        Some(false),
        1,
    )
    .unwrap();
    assert_eq!(animation.motion_attributes.len(), 1);
    assert_eq!(animation.motion_attributes[0].name, encode(b"OB_DroppingBoard"));
    assert_eq!(animation.motion_attributes[0].value, 1.0);
    assert!(!animation.motion_intents.contains_key("OB_DroppingBoard"));
    assert_eq!(animation.pending_parameter(encode(b"OB_DroppingBoard")), Some(1.0));
    assert_eq!(state.phase, Phase::Idle);
}

#[test]
fn all_toggle_publications_reach_physics_attributes_and_animation_parameters() {
    let mut animation = animation();
    for name in ["OB_RetrievingBoard", "OB_DroppingBoard", "OB_RetrieveBoard"] {
        publish(&mut animation, name);
        let packet = animation.motion_attributes.last().unwrap();
        assert_eq!(packet.name, encode(name.as_bytes()));
        assert_eq!(packet.value, 1.0);
        assert_eq!(animation.pending_parameter(encode(name.as_bytes())), Some(1.0));
        assert!(!animation.motion_intents.contains_key(name));
    }
    //Yaw/pitch parameterize the real retrieval tree; native does not append
    //these to the physical MG attribute list.
    attribute(&mut animation, "yaw", 45.0);
    attribute(&mut animation, "pitch", -5.0);
    assert_eq!(animation.motion_attributes.len(), 3);
    assert_eq!(animation.pending_parameter(encode(b"yaw")), Some(45.0));
    assert_eq!(animation.pending_parameter(encode(b"pitch")), Some(-5.0));
    animation.begin_graph_update();
    assert!(animation.motion_attributes.is_empty(), "Recall pulse leaked to next frame");
}
#[test]
fn lifecycle_end_leaves_persistent_channel_and_begin_resets_state() {
    let mut animation = animation();
    animation.channels.insert(
        CHANNEL.into(),
        PlaybackTree::Clip {
            name: "test".into(),
            clip: PlaybackClip::new(301.0, 30.0, 1.0, 0, Vec::new()),
        },
        ChannelSettings {
            priority: 0,
            keep_alive: true,
            mirrored: false,
            speed: 1.0,
            blend_in: 0.0,
            blend_out: 0.0,
            hold_during_blend_in: false,
            hold_during_blend_out: false,
            use_attributes: false,
        },
    );
    animation.channels.advance(0.3, 0.0);
    assert!((animation.channels.elapsed(CHANNEL) - 0.3).abs() < 0.00001);
    assert_eq!(animation.channels.elapsed("missing"), 0.0);
    let mut state = State::default();
    state.phase = Phase::DropPlaying;
    execute(&mut state, &mut animation, BoardControls::default(), None, None, 2).unwrap();
    assert_eq!(state.phase, Phase::DropPlaying);
    execute(&mut state, &mut animation, BoardControls::default(), None, None, 0).unwrap();
    assert_eq!(state.phase, Phase::Idle);
    animation.channels.advance(1.0, 0.0);
    animation.channels.retire();
    assert!(animation.channels.has(CHANNEL));
}
