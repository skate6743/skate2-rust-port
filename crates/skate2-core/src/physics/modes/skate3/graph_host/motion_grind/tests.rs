use super::*;
mod conditions;
mod fixture;
mod routing;
use fixture::{Owner, blend, clip, height_dependent_blend};
use skate3_core::animation::{
    grind_control::FadeSettings, playback::TransitionSettings,
    playback_transition::PlaybackTransition, playback_tree::PlaybackTree,
};
use skate_data::state_graph::GraphAttribute;

fn attributes(pairs: &[(&str, &str)]) -> Vec<GraphAttribute> {
    pairs
        .iter()
        .map(|(name, text)| GraphAttribute {
            name: (*name).into(),
            text: (*text).into(),
            float_bits: 0,
            boolean_byte: 0,
        })
        .collect()
}
fn settings() -> Settings {
    Settings {
        height: [0.25, 1.0, 2.0],
        fade: FadeSettings {
            response: 1.0,
            input_scale: 1.0,
            acceleration: 0.25,
            maximum_step: 0.5,
        },
    }
}
fn physical() -> Physical {
    Physical {
        grinding: true,
        grind_name: encode(b"FS_50_50"),
        ground_axis: [1.0, 0.0, 0.0],
        board_axis: [0.0, 0.0, -1.0],
        ground_flag_273: false,
        animation_mirrored: false,
        height: 1.0,
        crouch: 0.75,
        twist: 0.0,
    }
}
#[test]
fn parser_preserves_authored_crouch_name_and_rejects_missing_fade_attributes() {
    let a = attributes(&[
        ("name", "ControlGrindCrouch"),
        ("driveDistToCom", "CustomHeight"),
    ]);
    assert_eq!(
        Operation::parse(&Attributes::new(&a)).unwrap(),
        Some(Operation::Crouch {
            height: encode(b"CustomHeight")
        })
    );
    let a = attributes(&[("name", "GrindControlFade")]);
    assert!(Operation::parse(&Attributes::new(&a)).is_err());
}
#[test]
fn creates_ordered_packets_not_intents_or_settable_parameters() {
    let mut a = Owner::new(blend());
    let mut state = State::default();
    a.intents.insert("retained", 0.5);
    let before = a.intents.clone();
    execute(
        &mut state,
        &Operation::Attributes,
        0,
        0.1,
        &settings(),
        &physical(),
        &mut a,
    )
    .unwrap();
    assert!(a.packets.is_empty());
    execute(
        &mut state,
        &Operation::Attributes,
        1,
        0.1,
        &settings(),
        &physical(),
        &mut a,
    )
    .unwrap();
    assert_eq!(
        a.packets
            .iter()
            .map(|p| (p.name, p.value))
            .collect::<Vec<_>>(),
        vec![
            (encode(b"FS_50_50"), 1.0),
            (encode(b"GrindFacingBackwards"), 1.0)
        ]
    );
    assert_eq!(a.intents, before);
    assert!(a.pending.entries().is_empty());
    let mut p = physical();
    p.grinding = false;
    execute(
        &mut state,
        &Operation::Attributes,
        1,
        0.1,
        &settings(),
        &p,
        &mut a,
    )
    .unwrap();
    assert_eq!(a.packets.len(), 2);
}
#[test]
fn crouch_begins_from_physical_height_updates_authored_parameter_and_ends_silently() {
    let mut a = Owner::new(blend());
    let mut state = State::default();
    let op = Operation::Crouch {
        height: encode(b"custom"),
    };
    assert!(execute(&mut state, &op, 1, 0.125, &settings(), &physical(), &mut a).is_err());
    execute(&mut state, &op, 0, 0.125, &settings(), &physical(), &mut a).unwrap();
    assert!(a.pending.entries().is_empty());
    execute(&mut state, &op, 1, 0.125, &settings(), &physical(), &mut a).unwrap();
    let entry = a.pending.entries()[0];
    assert_eq!(
        (entry.name, entry.value, entry.normalized, entry.sequence_id),
        (encode(b"custom"), 0.75, false, -1)
    );
    a.pending.clear();
    execute(&mut state, &op, 2, 0.125, &settings(), &physical(), &mut a).unwrap();
    assert!(a.pending.entries().is_empty());
}
#[test]
fn fade_begin_applies_height_before_endpoints_and_keeps_live_tree_effects() {
    let mut a = Owner::new(height_dependent_blend());
    let mut state = State::default();
    let op = Operation::Fade {
        height: encode(b"height"),
        twist: encode(b"twist"),
        intent: "balance".into(),
    };
    let mut p = physical();
    p.twist = 3.0;
    execute(&mut state, &op, 0, 0.1, &settings(), &p, &mut a).unwrap();
    assert_eq!(a.applications, 2);
    let f = state.fade.as_ref().unwrap();
    assert_eq!((f.minimum, f.maximum, f.value), (-2.0, 4.0, 3.0));
    assert_eq!(a.pending.entries().len(), 1);
    assert!(!a.pending.entries()[0].normalized);
    a.apply_parameters().unwrap();
    let mut out = skate3_core::animation::output::attributes::MotionGraphAttribute {
        name: encode(b"twist"),
        value: 0.0,
    }
    .to_animation();
    assert!(
        a.query_current_attribute(encode(b"twist"), 15, &mut out)
            .unwrap()
    );
    assert_eq!(f32::from_bits(out.payload.0[0].unwrap()), 3.0);
    execute(&mut state, &op, 1, 0.1, &settings(), &p, &mut a).unwrap();
    assert!(
        a.pending.entries().is_empty(),
        "first Update emitted a new value"
    );
}
#[test]
fn endpoint_miss_keeps_original_zero_or_partial_output() {
    let mut missing = Owner::new(clip(None, 0.0));
    assert_eq!(
        animation::endpoints(&mut missing, encode(b"twist")).unwrap(),
        [0.0, 0.0]
    );
    let mut partial = Owner::new(PlaybackTree::PhaseBlend(
        skate3_core::animation::phase_blend::PhaseBlend::new(
            encode(b"height"),
            vec![clip(Some(-2.0), 0.0), clip(None, 1.0)],
        )
        .unwrap(),
    ));
    assert_eq!(
        animation::endpoints(&mut partial, encode(b"twist")).unwrap(),
        [-2.0, -2.0]
    );
}
#[test]
fn endpoint_root_query_respects_sequence_vs_blend_without_forcing_destination() {
    for (kind, expected) in [(2, [-2.0, 4.0]), (4, [-9.0, -9.0])] {
        let transition = PlaybackTransition::new(
            clip(Some(-9.0), 0.0),
            blend(),
            TransitionSettings {
                kind,
                seconds: 0.5,
                under: 0,
                matching: 0,
                use_channels_from_weights: false,
            },
        );
        let mut a = Owner::new(PlaybackTree::Transition(transition));
        assert_eq!(
            animation::endpoints(&mut a, encode(b"twist")).unwrap(),
            expected
        );
    }
}
