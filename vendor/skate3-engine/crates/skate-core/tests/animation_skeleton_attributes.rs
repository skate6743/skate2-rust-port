use skate_core::animation::{
    output::attributes::{AnimationAttribute, AttributeName, AttributePayload},
    skeleton_input::{
        catalog::{self, EVENT_COMPARISONS, SCALAR_ATTRIBUTES},
        name::encode,
        scalar_attributes::{self, AnimationControlOutput, DispatchError, ScalarAttributeInputs},
    },
};

fn input() -> ScalarAttributeInputs {
    ScalarAttributeInputs {
        flags2468: 0x1000,
        flags2472: 0x1000,
        flags2476: 0x1000,
        flags2484: 0x1000,
        flags2488: 0x1000,
        board_adjust: 999,
        balance: 2.0,
        spin: 3.0,
        body_spin: 4.0,
        brake: 5.0,
        turn: 6.0,
        turn_scale: 7.0,
        magnitude_scale: 8.0,
        animation_end_com: [1.0, 2.0, 3.0, -0.0],
        animation_translation: [4.0, 5.0, 6.0, 9.0],
        animation_time: 10.0,
        animation_physics_blend_seconds: 11.0,
        cadence_end_percent: 12.0,
        raw_turn: 13.0,
        hard_turn: 14.0,
        slide: 15.0,
    }
}
fn output() -> AnimationControlOutput {
    AnimationControlOutput {
        grind_name: AttributeName([99; 5]),
        flags: 0x1000,
    }
}
fn attribute(name: &[u8], value: Option<f32>) -> AnimationAttribute {
    AnimationAttribute {
        name: encode(name),
        payload: AttributePayload([value.map(f32::to_bits), None, None, None, None, None]),
        begin_time: 123.0,
        end_time: -99.0,
        status: 0,
        kind: 0,
        sequence_id: 5,
    }
}
fn apply(
    name: &[u8],
    value: Option<f32>,
    input: &mut ScalarAttributeInputs,
    output: &mut AnimationControlOutput,
) {
    scalar_attributes::dispatch_attribute(&attribute(name, value), input, output).unwrap();
}

#[test]
fn native_name_encoding_is_base38_case_insensitive_chunked_and_nul_terminated() {
    assert_eq!(encode(b"A").0, [11 * 79_235_168, 0, 0, 0, 0]);
    assert_eq!(
        encode(b"a0_zZz").0[0],
        11 * 79_235_168 + 2_085_136 + 37 * 54_872 + 36 * 1_444 + 36 * 38 + 36
    );
    assert_eq!(encode(b"ABCDEFg").0[1], 17 * 79_235_168);
    assert_eq!(encode(b"Turn"), encode(b"tUrN"));
    assert_eq!(encode(b"Turn\0Brake"), encode(b"Turn"));
    assert_eq!(
        encode(b"123456123456123456123456123456ignored"),
        encode(b"123456123456123456123456123456")
    );
    assert_eq!(
        encode(&[255]).0[0],
        (-48i32 as u32).wrapping_mul(79_235_168)
    );
    assert_ne!(encode(b"push_contact"), encode(b"PushContact"));
}

#[test]
fn complete_dispatch_identity_catalog_retains_direct_and_event_comparisons() {
    assert_eq!(SCALAR_ATTRIBUTES.len(), 151);
    assert_eq!(EVENT_COMPARISONS.len(), 5);
    assert!(
        SCALAR_ATTRIBUTES
            .windows(2)
            .all(|pair| pair[0].comparison_site < pair[1].comparison_site)
    );
    assert_eq!(
        catalog::lookup(encode(b"Spin")).unwrap().comparison_site,
        0x82bdb004
    );
    assert_eq!(
        catalog::lookup(encode(b"IsWipeoutPushOffImpulse"))
            .unwrap()
            .comparison_site,
        0x82bdb148
    );
    assert_eq!(
        catalog::lookup(encode(b"WantsLeftAirGrab"))
            .unwrap()
            .comparison_site,
        0x82bdb5fc
    );
    assert_eq!(
        catalog::lookup(encode(b"WantsRightAirGrab"))
            .unwrap()
            .comparison_site,
        0x82bdb614
    );
    assert_eq!(
        catalog::lookup(encode(b"jump")).unwrap().comparison_site,
        0x82bdb7e8
    );
    assert!(catalog::lookup(encode(b"push_contact")).is_none());
}

#[test]
fn steering_and_push_markers_write_their_distinct_consumed_fields_without_thresholds() {
    let mut i = input();
    let mut o = output();
    for name in [
        b"NoInput".as_slice(),
        b"NewAutoPump",
        b"ManualBrake",
        b"Carving",
        b"PushContact",
        b"PushSpeed",
    ] {
        apply(name, None, &mut i, &mut o);
    }
    apply(b"Brake", Some(-2.5), &mut i, &mut o);
    apply(b"Turn", Some(0.25), &mut i, &mut o);
    apply(b"Spin", Some(-0.75), &mut i, &mut o);
    assert_eq!((i.brake, i.turn, i.spin), (-2.5, -0.25, 0.75));
    assert_eq!(i.flags2468, 0xa0001000);
    assert_eq!(i.flags2472, 0x00801000);
    assert_eq!(i.flags2488, 0x20801000);
    assert_eq!(i.flags2476, 0x1000);
    assert_eq!(o.flags, 0x60001000);
    assert_eq!(i.balance, 2.0);
    // A false/zero scalar is still a presence marker in these native branches.
    apply(b"PushContact", Some(0.0), &mut i, &mut o);
    assert_eq!(i.flags2488, 0x20801000);
}

#[test]
fn repeated_attributes_preserve_list_order_and_do_not_clear_unrelated_state() {
    let mut i = input();
    let mut o = output();
    apply(b"SlideBoard", None, &mut i, &mut o);
    apply(b"GrindNose", None, &mut i, &mut o);
    apply(b"GrindFacingForwards", None, &mut i, &mut o);
    apply(b"GrindFacingBackwards", None, &mut i, &mut o);
    apply(b"BoardAdjustLeft", None, &mut i, &mut o);
    apply(b"BoardAdjustDown", None, &mut i, &mut o);
    apply(b"Turn", Some(1.0), &mut i, &mut o);
    apply(b"TURN", Some(-0.0), &mut i, &mut o);
    assert_eq!(o.grind_name, encode(b"GrindNose"));
    assert_eq!(o.flags, 0x1000);
    assert_eq!(i.board_adjust, 4);
    assert_eq!(i.turn.to_bits(), 0);
    assert_eq!(i.brake, 5.0);
    assert_eq!(i.flags2468, 0x1000);
}

#[test]
fn com_translation_and_scale_handlers_preserve_other_lanes_and_float_signs() {
    let mut i = input();
    let mut o = output();
    apply(b"AnimEndCOMY", Some(-7.0), &mut i, &mut o);
    apply(b"AnimTransZ", Some(8.0), &mut i, &mut o);
    apply(b"TurnScale", Some(0.0), &mut i, &mut o);
    apply(b"MagScale", Some(-3.0), &mut i, &mut o);
    apply(b"RawTurn", Some(f32::from_bits(0x7fc12345)), &mut i, &mut o);
    assert_eq!(i.animation_end_com[..3], [1.0, -7.0, 3.0]);
    assert_eq!(i.animation_end_com[3].to_bits(), 0x80000000);
    assert_eq!(i.animation_translation, [4.0, 5.0, 8.0, 9.0]);
    assert_eq!((i.turn_scale, i.magnitude_scale), (0.0, -3.0));
    assert_eq!(i.raw_turn.to_bits(), 0xffc12345);
}

#[test]
fn scalar_kind_gate_ignores_timing_status_but_never_treats_active_events_as_scalars() {
    let mut i = input();
    let mut o = output();
    let original = i;
    for kind in [1, 2, 127, 128, 255] {
        let mut a = attribute(b"Turn", Some(0.75));
        a.kind = kind;
        a.status = 255;
        scalar_attributes::dispatch_attribute(&a, &mut i, &mut o).unwrap();
        assert_eq!(i, original);
    }
    for status in [0, 1, 2, 16, 128] {
        let mut a = attribute(b"Turn", None);
        a.kind = 3;
        a.status = status;
        scalar_attributes::dispatch_attribute(&a, &mut i, &mut o).unwrap();
        assert_eq!(i, original);
    }
    for status in [4, 8, 12, 255] {
        let mut a = attribute(b"unknown-event", None);
        a.kind = 3;
        a.status = status;
        assert!(matches!(
            scalar_attributes::dispatch_attribute(&a, &mut i, &mut o),
            Err(DispatchError::EventConsumerUnavailable { .. })
        ));
    }
    let mut a = attribute(b"Turn", Some(2.0));
    a.status = 255;
    scalar_attributes::dispatch_attribute(&a, &mut i, &mut o).unwrap();
    assert_eq!(i.turn, -2.0);
}

#[test]
fn unported_known_handlers_and_uninitialized_scalars_fail_without_writes() {
    let mut i = input();
    let mut o = output();
    let original = i;
    let original_o = o;
    assert!(
        matches!(scalar_attributes::dispatch_attribute(&attribute(b"MinJump", Some(0.8)), &mut i, &mut o),
        Err(DispatchError::KnownScalarUnavailable { attribute }) if attribute.comparison_site == 0x82bdb71c)
    );
    assert!(matches!(
        scalar_attributes::dispatch_attribute(&attribute(b"Turn", None), &mut i, &mut o),
        Err(DispatchError::UninitializedScalar { .. })
    ));
    scalar_attributes::dispatch_attribute(&attribute(b"unknown-name", None), &mut i, &mut o)
        .unwrap();
    assert_eq!(i, original);
    assert_eq!(o, original_o);
}
