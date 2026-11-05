use super::*;
use crate::skate3_native::input::platform::{DeviceError, DevicePacket};
use skate3_core::input::xbox::XboxState;

impl ControllerInput {
    /// Feed test packets through the production conversion, cache, history and Pad.
    /// Only the operating-system read is supplied by the test.
    pub(crate) fn sample_raw_for_test(&mut self, state: XboxState) {
        self.collect([
            Ok(DevicePacket {
                number: self.publications as u32 + 1,
                state,
                subtype: 1,
            }),
            Err(DeviceError::Disconnected),
            Err(DeviceError::Disconnected),
            Err(DeviceError::Disconnected),
        ]);
        assert!(self.publish_actions());
    }
}

fn packet(number: u32, buttons: u16, left: [i16; 2]) -> Result<DevicePacket, DeviceError> {
    Ok(DevicePacket {
        number,
        state: XboxState {
            buttons,
            triggers: [0; 2],
            left,
            right: [0; 2],
        },
        subtype: 1,
    })
}

#[test]
fn raw_packets_reach_stock_actions_without_losing_device_identity() {
    let mut input = ControllerInput::default();
    input.collect([
        packet(1, 0x1000, [32767, 0]),
        packet(2, 0x2000, [-32768, 0]),
        packet(3, 0x4000, [0, 0]),
        packet(4, 0x8000, [0, 0]),
    ]);
    assert!(input.publish_actions());
    assert!((input.mapped_actions[0][0] - 1.0).abs() < 0.000001);
    assert!((input.mapped_actions[1][0] + 1.0).abs() < 0.000001);
    assert_eq!(input.mapped_actions[0][16], 1.0); // GP_AFace
    assert_eq!(input.mapped_actions[1][17], 1.0); // GP_BFace
    assert_eq!(input.mapped_actions[2][14], 1.0); // GP_XFace
    assert_eq!(input.mapped_actions[3][15], 1.0); // GP_YFace
    assert_eq!(input.packet_numbers, [Some(1), Some(2), Some(3), Some(4)]);
}

#[test]
fn unchanged_packet_numbers_do_not_suppress_publication_and_empty_drain_does_not_advance() {
    let mut input = ControllerInput::default();
    for _ in 0..4 {
        input.collect(std::array::from_fn(|_| packet(7, 0x1000, [0; 2])));
        assert!(input.publish_actions());
    }
    assert_eq!(input.publications, 4);
    assert_eq!(input.consumed_batches, 4);
    assert_ne!(input.pads[0].records()[12][1] & 0x01000000, 0);
    let prior = input.pads.clone();
    assert!(!input.publish_actions());
    assert_eq!(input.pads, prior);
}

#[test]
fn disconnect_publishes_zero_count_and_reconnect_reinitializes_pad_records() {
    let mut input = ControllerInput::default();
    input.collect(std::array::from_fn(|_| packet(1, 0x1000, [32767, 0])));
    input.publish_actions();
    input.collect(std::array::from_fn(|_| Err(DeviceError::Disconnected)));
    input.publish_actions();
    assert_eq!(input.mapped_actions, [[0.0; 18]; 4]);
    assert!(input.pads.iter().all(|pad| pad.count() == 0));
    input.collect(std::array::from_fn(|_| packet(1, 0, [0; 2])));
    input.publish_actions();
    assert_eq!(input.mapped_actions, [[0.0; 18]; 4]);
    assert_eq!(input.pads[0].records()[12], [0, 0, 0, 1]);
}

#[test]
fn alternate_guitar_subtype_uses_verified_conversion_mask_and_errors_stay_visible() {
    let mut input = ControllerInput::default();
    let special = DevicePacket {
        number: 1,
        state: XboxState {
            buttons: 0,
            triggers: [255; 2],
            left: [0; 2],
            right: [-32768, 32767],
        },
        subtype: 7,
    };
    input.collect([
        Ok(special),
        Err(DeviceError::Capabilities(5)),
        Err(DeviceError::Disconnected),
        Err(DeviceError::Disconnected),
    ]);
    input.publish_actions();
    assert_eq!(input.mapped_actions[0][6..8], [0.0; 2]);
    assert_eq!(input.mapped_actions[0][3..5], [0.0; 2]);
    assert_eq!(
        input.status[1],
        ControllerStatus::Unavailable(DeviceError::Capabilities(5))
    );
    assert_eq!(input.mapped_actions[1], [0.0; 18]);
}

#[test]
fn tick_input_is_the_single_fixed_tick_snapshot() {
    let mut input = ControllerInput::default();
    input.collect(std::array::from_fn(|_| packet(1, 0, [19661, 0])));
    assert!(input.publish_actions());

    let first = input.tick_input();
    assert_eq!(first.tick(), 1);
    assert!(first.controller_available());
    let mut first_actions = first.actions();
    let first_turn = first_actions.value(64);

    // A second consumer reads the same published tick without touching the
    // platform cache or remapping the controller.
    let second = input.tick_input();
    let mut second_actions = second.actions();
    assert_eq!(second.tick(), first.tick());
    assert_eq!(second_actions.value(64), first_turn);
}

#[test]
fn unavailable_controllers_publish_zero_gameplay_actions() {
    let mut input = ControllerInput::default();
    input.collect(std::array::from_fn(|_| packet(1, 0x1000, [32767, 0])));
    assert!(input.publish_actions());
    assert!(input.tick_input().controller_available());

    input.collect(std::array::from_fn(|_| Err(DeviceError::Disconnected)));
    assert!(input.publish_actions());
    let snapshot = input.tick_input();
    assert!(!snapshot.controller_available());
    assert_eq!(*snapshot.actions().values(), [0.0; 18]);
}
