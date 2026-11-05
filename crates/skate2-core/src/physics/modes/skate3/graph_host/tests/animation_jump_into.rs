use super::*;

fn owner(tree: PlaybackTree) -> MotionAnimation {
    // Synthetic independent fixture; no claim that this is stock animation data.
    let metadata = AnimationMetadata::parse(r#"{
        "version":1,"source_bank":"synthetic-test",
        "source_sha256":"0000000000000000000000000000000000000000000000000000000000000000",
        "source_bytes":48,"clips":[],"unsupported_trees":[]
    }"#).unwrap();
    let mut animation = MotionAnimation::from_metadata(metadata);
    animation.current = Some(tree);
    animation
}

fn clip(begin: f32, blend_value: f32) -> PlaybackTree {
    PlaybackTree::Clip {
        name: "synthetic-manual-takeoff".into(),
        clip: PlaybackClip::new(61.0, 60.0, 1.0, 0, vec![
            ClipAttribute {
                name: encode(b"manualInto"), kind: 0, begin, end: begin + 0.1,
                payload: vec![17.0f32.to_bits()],
            },
            ClipAttribute {
                name: encode(b"height"), kind: 0, begin: -1.0, end: -1.0,
                payload: vec![blend_value.to_bits()],
            },
        ]),
    }
}

#[test]
fn jump_into_queries_inactive_marker_begin_and_resets_clip_history() {
    let mut animation = owner(clip(0.625, 0.0));
    let name = encode(b"manualInto");
    animation.refresh_tree_attributes().unwrap();
    assert!(!animation.tree_attributes().iter().any(|a| a.name == name));
    animation.jump_into(name).unwrap();
    assert_eq!(animation.current_time().unwrap(), 0.625);
    let PlaybackTree::Clip { clip, .. } = animation.current.as_ref().unwrap() else { panic!() };
    assert_eq!(clip.clock.previous_time, 0.625);
    assert_eq!(clip.clock.loops_since_evaluation, 0);
    assert!(animation.motion_attributes.is_empty());
    assert!(animation.settable.entries().is_empty());
}

#[test]
fn jump_into_applies_pending_blend_before_reading_marker_time() {
    let tree = PlaybackTree::PhaseBlend(PhaseBlend::new(
        encode(b"height"), vec![clip(0.25, 0.0), clip(0.75, 1.0)],
    ).unwrap());
    let mut animation = owner(tree);
    animation.set_attribute(SettableAttribute {
        name: encode(b"height"), value: 1.0, normalized: false, sequence_id: -1,
    });
    animation.jump_into(encode(b"manualInto")).unwrap();
    assert_eq!(animation.current_time().unwrap(), 0.75);
    assert!(animation.settable.entries().is_empty());
    assert!(animation.motion_attributes.is_empty());
}

#[test]
fn jump_into_missing_marker_preserves_time_without_emitting_packets() {
    let mut animation = owner(clip(0.625, 0.0));
    animation.advance(0.125, 0.0);
    animation.jump_into(encode(b"missing-marker")).unwrap();
    assert_eq!(animation.current_time().unwrap(), 0.125);
    assert!(animation.motion_attributes.is_empty());
}
