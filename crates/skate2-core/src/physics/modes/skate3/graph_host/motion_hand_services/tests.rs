use super::{Hand, HandServices, Operation};

#[test]
fn nested_hand_lifetimes_preserve_other_owners() {
    let mut state = HandServices::default();
    let bs = Operation::HandBusy(Hand::Backside);
    let fs = Operation::HandBusy(Hand::Frontside);
    for operation in [bs, fs, bs] { state.apply(operation, 0); }
    state.apply(bs, 1);
    assert_eq!(state.busy_hands, [2, 1]);
    state.apply(bs, 2);
    assert_eq!(state.busy_hands, [1, 1]);
    state.apply(fs, 2);
    assert_eq!(state.busy_hands, [1, 0]);
    state.apply(bs, 2);
    assert_eq!(state.busy_hands, [0, 0]);
}

#[test]
fn retail_counter_stores_wrap_without_saturating() {
    let mut state = HandServices::default();
    let bs = Operation::HandBusy(Hand::Backside);
    state.apply(bs, 2);
    assert_eq!(state.busy_hands, [u32::MAX, 0]);
    state.apply(bs, 0);
    assert_eq!(state.busy_hands, [0, 0]);
}

#[test]
fn maintain_shove_is_an_update_latch_not_a_reference_count() {
    let mut state = HandServices::default();
    let operation = Operation::MaintainShove;
    assert!(!state.keep_shove_channels);
    assert!(!state.apply(operation, 0));
    assert!(!state.keep_shove_channels);
    for _ in 0..2 { assert!(!state.apply(operation, 1)); }
    assert!(state.keep_shove_channels);
    assert!(state.apply(operation, 2));
    assert!(!state.keep_shove_channels);
    assert!(!state.apply(operation, 1));
    assert!(state.keep_shove_channels);
}

#[test]
fn hand_selection_uses_exact_stock_comparison() {
    assert_eq!(Hand::from_name("bs"), Hand::Backside);
    for name in ["fs", "BS", "", "both"] {
        assert_eq!(Hand::from_name(name), Hand::Frontside);
    }
}

#[test]
fn maintain_end_fades_only_antic_through_existing_animation_runtime() {
    use crate::skate3_native::graph_host::motion_animation::MotionAnimation;
    use skate3_core::animation::{
        channel_playback::ChannelSettings, playback_clip::PlaybackClip,
        playback_tree::PlaybackTree,
    };
    let metadata = skate_data::animation_metadata::AnimationMetadata::parse(
        r#"{"version":1,"source_bank":"test-only","source_sha256":"0000000000000000000000000000000000000000000000000000000000000000","source_bytes":48,"clips":[],"unsupported_trees":[]}"#,
    ).unwrap();
    let mut animation = MotionAnimation::from_metadata(metadata);
    let settings = ChannelSettings {
        priority: 0, keep_alive: true, mirrored: false, speed: 1.0,
        blend_in: 0.0, hold_during_blend_in: false, blend_out: 0.5,
        hold_during_blend_out: false, use_attributes: false,
    };
    for name in ["SkitchAntic", "Shove"] {
        animation.channels.insert(name.into(), PlaybackTree::Clip {
            name: name.into(),
            clip: PlaybackClip::new(301.0, 30.0, 1.0, 0, Vec::new()),
        }, settings);
    }
    let mut state = HandServices::default();
    state.execute(Operation::MaintainShove, 1, &mut animation);
    state.execute(Operation::MaintainShove, 2, &mut animation);
    assert!(!state.keep_shove_channels);
    assert!(animation.channels.has("SkitchAntic"));
    animation.channels.advance(0.25, 0.0);
    animation.channels.retire();
    assert!(animation.channels.has("SkitchAntic"));
    animation.channels.advance(0.25, 0.0);
    animation.channels.retire();
    assert!(!animation.channels.has("SkitchAntic"));
    assert!(animation.channels.has("Shove"));
    // Repeating End with no antic must leave unrelated live channels intact.
    state.execute(Operation::MaintainShove, 2, &mut animation);
    assert!(animation.channels.has("Shove"));
}
