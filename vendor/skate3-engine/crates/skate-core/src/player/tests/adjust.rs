use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    Condition,
    BoardFlag,
    Adjust(bool, u32),
    State,
    Selector,
    Finish(u32),
}

struct Services {
    condition: u32,
    board_flag: bool,
    selector: u32,
    events: Vec<Event>,
}

impl AdjustServices for Services {
    fn board_condition_82c02138(&mut self) -> u32 {
        self.events.push(Event::Condition);
        self.condition
    }

    fn controller_mode_one_board_flag_868(&mut self) -> bool {
        self.events.push(Event::BoardFlag);
        self.board_flag
    }

    fn adjust_skeleton_82bd80d8(&mut self, board_flag: bool, timestep: f32) {
        self.events
            .push(Event::Adjust(board_flag, timestep.to_bits()));
    }

    fn update_current_state_post_physics_vtable_40(&mut self) {
        self.events.push(Event::State);
    }

    fn player_output_selector_82db9100(&mut self) -> u32 {
        self.events.push(Event::Selector);
        self.selector
    }

    fn finish_skeleton_adjustment_82bd83e0(&mut self, selector: u32) {
        self.events.push(Event::Finish(selector));
    }
}

#[test]
fn publishes_board_condition_and_preserves_post_solver_call_order() {
    let mut fields = AdjustFields {
        processed_word_2512: 100,
        timestep_2604: f32::from_bits(0x3c88_8889),
        controller_state_448: 1,
        output_flag_42: false,
        output_scalar_144: 7.0,
        output_counter_200: u32::MAX,
    };
    let mut services = Services {
        condition: 1,
        board_flag: true,
        selector: 19,
        events: Vec::new(),
    };

    run_adjust(&mut fields, &mut services);

    assert!(fields.output_flag_42);
    assert_eq!(fields.output_scalar_144.to_bits(), 0);
    assert_eq!(fields.output_counter_200, 0);
    assert_eq!(
        services.events,
        [
            Event::Condition,
            Event::BoardFlag,
            Event::Adjust(true, 0x3c88_8889),
            Event::State,
            Event::Selector,
            Event::Finish(19),
        ]
    );
}

#[test]
fn state_500_and_other_controller_modes_skip_only_the_native_branches() {
    let mut fields = AdjustFields {
        processed_word_2512: 500,
        timestep_2604: 0.25,
        controller_state_448: 4,
        output_flag_42: false,
        output_scalar_144: 7.0,
        output_counter_200: 8,
    };
    let mut services = Services {
        condition: 1,
        board_flag: true,
        selector: 3,
        events: Vec::new(),
    };

    run_adjust(&mut fields, &mut services);

    assert!(!fields.output_flag_42);
    assert_eq!(fields.output_scalar_144, 7.0);
    assert_eq!(fields.output_counter_200, 8);
    assert_eq!(
        services.events,
        [
            Event::Condition,
            Event::Adjust(false, 0.25_f32.to_bits()),
            Event::State,
            Event::Selector,
            Event::Finish(3),
        ]
    );
}
