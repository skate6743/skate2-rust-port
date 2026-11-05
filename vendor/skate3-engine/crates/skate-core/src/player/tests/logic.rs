use super::*;
use crate::player::{
    lifecycle::{
        PhysicalStateCalls, PlayerStateChangeFields, ProcessedStateChangeFields,
        SkateboardControllerActions, SkateboardControllerFields, StateCall, StateChangeData,
    },
    selector::{
        ProcessedStateInput,
        conditions::{BoardBodyState, SkeletonAnimationState, TwoStageThresholds},
    },
    state::PhysicalStateId as State,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    GetType(State),
    Exit(State),
    Enter(State),
}

struct States {
    events: Rc<RefCell<Vec<Event>>>,
}

impl PhysicalStateCalls for States {
    fn get_type(&mut self, state: StateBinding) -> State {
        self.events.borrow_mut().push(Event::GetType(state.state));
        state.state
    }

    fn exit(&mut self, call: StateCall) {
        self.events.borrow_mut().push(Event::Exit(call.state.state));
    }

    fn enter(&mut self, call: StateCall) {
        self.events
            .borrow_mut()
            .push(Event::Enter(call.state.state));
    }
}

struct Controller;

impl SkateboardControllerActions for Controller {
    fn hold_skateboard(&mut self) {}
    fn let_go_of_skateboard(&mut self) {}
}

fn input() -> StateSelectionInput {
    let thresholds = TwoStageThresholds {
        field_856_primary: 1.0,
        field_856_secondary: 1.0,
        field_7692: 1.0,
    };
    StateSelectionInput {
        processed: ProcessedStateInput {
            grind_type_1248: 6,
            grind_candidate_1488: false,
            field_1776: 0,
            flags_2468: 0,
            flags_2472: 0,
            flags_2476: 0,
            flags_2480: 0,
            flags_2484: 0,
            flags_2488: 0,
            category_2512: 100,
            wheel_contact_count_2556: 0,
            field_2572: 0,
            state_timer_2664: 0.0,
            field_2732: 0.0,
            field_2744: 0.0,
            trajectory_collision_time_2772: 0.0,
            grind_investigation_flags_1516: 0,
        },
        skateboard_contact_count_869: 0,
        board_body: BoardBodyState {
            field_856: 0.0,
            field_7692: 0.0,
        },
        skeleton: SkeletonAnimationState {
            mode_16420: 1,
            back_chain_lane_12656: 0.0,
            threshold_800: 1.0,
        },
        skitching_off_ground: thresholds,
        normal_off_ground: thresholds,
    }
}

fn change_data() -> StateChangeData {
    StateChangeData {
        player: PlayerStateChangeFields {
            word_1312: 1,
            previous_category_latch_1336: 0,
            scalar_1344: 1.0,
        },
        processed: ProcessedStateChangeFields {
            flags_2480: 0,
            requested_state_2500: 0,
            previous_state_2504: 0,
            current_state_2508: 100,
            current_category_2512: 100,
            previous_category_2516: 0,
            previous_category_latch_2520: 0,
            word_2564: 1,
            scalar_2664: 1.0,
        },
        skateboard_controller: SkateboardControllerFields {
            word_444: 0,
            state_448: 0,
            system_on_452: false,
        },
    }
}

fn staging() -> PhysicsOutputStaging {
    PhysicsOutputStaging {
        bytes_20_through_53: [0xaa; 34],
        scalars_56_through_188: [1.0; 34],
        word_200: 7,
    }
}

#[test]
fn unchanged_state_calls_get_type_once_and_still_clears_output_staging() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut logic = PhysicalPlayerLogic::new(State::PhysicsGround);
    let mut data = change_data();
    let mut output = staging();
    let mut states = States {
        events: events.clone(),
    };
    let result = logic
        .run(
            &input(),
            &mut data,
            &mut output,
            &mut states,
            &mut Controller,
        )
        .unwrap();

    assert!(!result.changed);
    assert_eq!(*events.borrow(), [Event::GetType(State::PhysicsGround)]);
    assert_eq!(logic.frames_since_jump_1304, 0);
    assert_eq!(output.bytes_20_through_53, [0; 34]);
    assert_eq!(output.scalars_56_through_188, [0.0; 34]);
    assert_eq!(output.word_200, 0);
}

#[test]
fn state_edge_runs_native_transition_and_sets_100() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut logic = PhysicalPlayerLogic::new(State::PhysicsGround);
    let mut frame = input();
    frame.processed.flags_2480 = 0x1000;
    let mut data = change_data();
    let mut output = staging();
    let mut states = States {
        events: events.clone(),
    };
    let result = logic
        .run(&frame, &mut data, &mut output, &mut states, &mut Controller)
        .unwrap();

    assert!(result.changed);
    assert_eq!(result.selected.state, State::LandingOnDeck);
    assert_eq!(logic.frames_since_jump_1304, 100);
    assert_eq!(
        *events.borrow(),
        [
            Event::GetType(State::PhysicsGround),
            Event::GetType(State::PhysicsGround),
            Event::Exit(State::PhysicsGround),
            Event::Enter(State::LandingOnDeck),
        ]
    );
    assert_eq!(data.processed.current_state_2508, 503);
}
