use super::{
    attribute_finalization::*,
    catalog::SCALAR_ATTRIBUTES,
    contact_events::*,
    extended_attributes::ExtendedAttributes,
    name::encode,
    process_attributes::process,
    scalar_attributes::{AnimationControlOutput, ScalarAttributeInputs},
};
use crate::{
    animation::output::{attributes::*, packet_reset::RESET_POSE},
    input::controller::ActionMap,
};

struct Map;
impl ActionMap for Map {
    fn value(&mut self, key: u32) -> f32 {
        match key {
            64 => 0.25,
            65 => -0.75,
            68 => 0.5,
            _ => panic!("Unexpected source action{key}"),
        }
    }
    fn state(&mut self, _: u32) -> u8 {
        panic!("Attribute finalization reads analog values only")
    }
}

#[test]
fn complete_attribute_packet_keeps_contact_timing_reset_and_finalization() {
    let mut fields = ScalarAttributeInputs::reset(8, u32::MAX);
    let mut extra = ExtendedAttributes::reset(0.0);
    let mut contacts = ContactEventState {
        bone: 0,
        push_speed: 0.0,
    };
    let mut cached = JumpAttributeState::new();
    let mut output = AnimationControlOutput {
        grind_name: encode(b""),
        flags: 0,
    };
    let names = [encode(b"Trajectory"), encode(b"RightToeBase")];
    let mut hierarchy = [RESET_POSE; 2];
    hierarchy[0][3] = [0.0, 0.0, 0.1, 0.0];
    // Deliberately unrelated foot position: native speed is trajectory/dt.
    hierarchy[1][3] = [20.0, 10.0, -40.0, 0.0];
    let pose = ContactEventPose {
        bone_names: &names,
        hierarchy: &hierarchy,
        trajectory_bone: 0,
        right_toe_bone: 1,
        timestep: 1.0 / 60.0,
    };
    let mut map = Map;
    let input = FinalizationInput {
        select_jump_extremes: true,
        low_jump_threshold: 0.25,
        high_jump_threshold: 0.75,
        allow_height_override: true,
        use_prepared_controls: false,
        external_impulse_active: false,
        animation_flags: 0x08000000,
    };
    let mut event = MotionGraphAttribute {
        name: encode(b"push_contact"),
        value: 0.0,
    }
    .to_animation();
    event.kind = 3;
    event.status = 4;
    event.payload = AttributePayload(std::array::from_fn(|i| {
        Some(if i < 5 {
            names[1].0[i]
        } else {
            1.0f32.to_bits()
        })
    }));
    process(
        &[event],
        &pose,
        &mut fields,
        &mut extra,
        &mut contacts,
        &mut cached,
        &mut output,
        input,
        Some(&mut map),
    )
    .unwrap();
    assert_eq!(contacts.bone, 1);
    assert!((contacts.push_speed - 6.0).abs() < 0.00001);
    assert_eq!(fields.flags2468 & 0x0f000000, 0x0e000000);
    assert_eq!(extra.flags2480 & 8, 8); //byte at start of a big-endian word
    assert_eq!(extra.jump_controls, [0.25, 0.5]);
    event.status = 2;
    contacts.push_speed = 7.0;
    process(
        &[event],
        &pose,
        &mut fields,
        &mut extra,
        &mut contacts,
        &mut cached,
        &mut output,
        input,
        Some(&mut map),
    )
    .unwrap();
    assert_eq!(contacts.push_speed, 7.0); //inactive event performs no writes
    let attributes: Vec<_> = SCALAR_ATTRIBUTES
        .iter()
        .map(|a| {
            MotionGraphAttribute {
                name: a.encoded_name,
                value: 0.5,
            }
            .to_animation()
        })
        .collect();
    process(
        &attributes,
        &pose,
        &mut fields,
        &mut extra,
        &mut contacts,
        &mut cached,
        &mut output,
        input,
        Some(&mut map),
    )
    .unwrap();
    assert_eq!(cached.prepared_controls, [0.25, -0.75]);
    assert_eq!(fields.flags2476 & 2, 2); //PlayerControlledPump
    assert_eq!(fields.flags2476 & 0x400000, 0); //retrieving/dropping clears GrabWorld
    let reset = ScalarAttributeInputs::reset(fields.flags2468, fields.flags2488);
    assert_eq!(reset.flags2468, 0x2008);
    assert_eq!(
        (
            reset.turn_scale,
            reset.magnitude_scale,
            reset.cadence_end_percent
        ),
        (1.0, 1.0, -1.0)
    );
}
