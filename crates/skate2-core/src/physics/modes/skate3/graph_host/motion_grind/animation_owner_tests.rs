use super::*;
use crate::skate3_native::graph_host::motion_grind::animation::endpoints;
use skate3_core::animation::{
    output::attributes::MotionGraphAttribute,
    phase_blend::PhaseBlend,
    playback_clip::{ClipAttribute, PlaybackClip},
    playback_parameters::{AttributeSink, SettableAttribute},
    playback_tree::PlaybackTree,
    skeleton_input::name::encode,
};
use skate_data::animation_metadata::AnimationMetadata;

fn owner() -> MotionAnimation {
    let metadata = AnimationMetadata::parse(
        r#"{
        "version":1,"source_bank":"synthetic-test",
        "source_sha256":"0000000000000000000000000000000000000000000000000000000000000000",
        "source_bytes":48,"clips":[],"unsupported_trees":[]
    }"#,
    )
    .unwrap();
    let mut owner = MotionAnimation::from_metadata(metadata);
    let clip = |value: f32| PlaybackTree::Clip {
        name: "independent-grind-test".into(),
        clip: PlaybackClip::new(
            61.0,
            60.0,
            1.0,
            0,
            vec![ClipAttribute {
                name: encode(b"twist"),
                kind: 0,
                begin: -1.0,
                end: -1.0,
                payload: vec![value.to_bits()],
            }],
        ),
    };
    owner.current = Some(PlaybackTree::PhaseBlend(
        PhaseBlend::new(encode(b"twist"), vec![clip(-2.0), clip(4.0)]).unwrap(),
    ));
    owner
}

#[test]
fn actual_motion_animation_adapter_uses_live_queue_root_and_packet_owner() {
    let mut owner = owner();
    owner.motion_intents.insert("unchanged", 0.25);
    // The probe must replace this queued attribute, not append a duplicate
    // that an actual PhaseBlend would read first and use for both endpoints.
    owner.set_attribute(SettableAttribute {
        name: encode(b"twist"),
        value: 0.5,
        normalized: true,
        sequence_id: -1,
    });
    assert_eq!(
        endpoints(&mut owner, encode(b"twist")).unwrap(),
        [-2.0, 4.0]
    );
    assert_eq!(owner.pending_parameter(encode(b"twist")), None);
    let mut out = MotionGraphAttribute {
        name: encode(b"twist"),
        value: 0.0,
    }
    .to_animation();
    assert!(
        owner
            .query_current_attribute(encode(b"twist"), 15, &mut out)
            .unwrap()
    );
    assert_eq!(f32::from_bits(out.payload.0[0].unwrap()), 4.0);
    Animation::emit_packet(&mut owner, encode(b"GrindFacingForwards"), 1.0);
    assert_eq!(owner.motion_attributes.len(), 1);
    assert_eq!(owner.motion_intents.len(), 1);
    assert_eq!(owner.motion_intents.get("unchanged"), Some(&0.25));
}

#[test]
fn actual_owner_missing_tree_is_an_error_not_fabricated_endpoints() {
    let mut owner = owner();
    owner.current = None;
    assert!(endpoints(&mut owner, encode(b"twist")).is_err());
}
