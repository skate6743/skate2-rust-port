use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Fill,
    Component,
    State,
}

struct Services {
    events: Vec<Event>,
    vector: [u32; 4],
}

impl PreStateServices for Services {
    fn fill_packet_vtable_24(&mut self, packet: &mut PreStatePacket) {
        self.events.push(Event::Fill);
        packet.words[12..16].copy_from_slice(&self.vector);
    }

    fn update_component_1840_82d74270(&mut self) {
        self.events.push(Event::Component);
    }

    fn update_before_state_vtable_4(&mut self) {
        self.events.push(Event::State);
    }
}

#[test]
fn publishes_packet_vector_and_preserves_full_native_call_order() {
    let mut player = PreStatePlayerFields {
        frame_counter_1312: 9,
        component_1840_present: true,
    };
    let mut skeleton = PreStateSkeletonFields {
        predicted_position_16112: [0; 4],
        predicted_position_set_16416: false,
        nested_flag_3184: true,
    };
    let vector = [0x3F80_0000, 0xC000_0000, 0x4040_0000, 0];
    let mut services = Services {
        events: Vec::new(),
        vector,
    };

    run_pre_state(&mut player, &mut skeleton, &mut services);

    assert_eq!(player.frame_counter_1312, 10);
    assert_eq!(skeleton.predicted_position_16112, vector);
    assert!(skeleton.predicted_position_set_16416);
    assert!(!skeleton.nested_flag_3184);
    assert_eq!(
        services.events,
        [Event::Fill, Event::Component, Event::State]
    );
}

#[test]
fn absent_component_skips_only_82d74270_and_counter_wraps() {
    let mut player = PreStatePlayerFields {
        frame_counter_1312: u32::MAX,
        component_1840_present: false,
    };
    let mut skeleton = PreStateSkeletonFields {
        predicted_position_16112: [7; 4],
        predicted_position_set_16416: false,
        nested_flag_3184: true,
    };
    let mut services = Services {
        events: Vec::new(),
        vector: [1, 2, 3, 4],
    };

    run_pre_state(&mut player, &mut skeleton, &mut services);

    assert_eq!(player.frame_counter_1312, 0);
    assert_eq!(services.events, [Event::Fill, Event::State]);
    assert_eq!(skeleton.predicted_position_16112, [1, 2, 3, 4]);
}
