use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    GetType(StateBinding),
    Exit(StateCall),
    Enter(StateCall),
    Hold,
    LetGo,
}

struct Recorder {
    reported_type: PhysicalStateId,
    events: Vec<Event>,
}

#[derive(Clone)]
struct SharedRecorder {
    reported_type: PhysicalStateId,
    events: Rc<RefCell<Vec<Event>>>,
}

impl PhysicalStateCalls for SharedRecorder {
    fn get_type(&mut self, state: StateBinding) -> PhysicalStateId {
        self.events.borrow_mut().push(Event::GetType(state));
        self.reported_type
    }

    fn exit(&mut self, call: StateCall) {
        self.events.borrow_mut().push(Event::Exit(call));
    }

    fn enter(&mut self, call: StateCall) {
        self.events.borrow_mut().push(Event::Enter(call));
    }
}

impl SkateboardControllerActions for SharedRecorder {
    fn hold_skateboard(&mut self) {
        self.events.borrow_mut().push(Event::Hold);
    }

    fn let_go_of_skateboard(&mut self) {
        self.events.borrow_mut().push(Event::LetGo);
    }
}

impl PhysicalStateCalls for Recorder {
    fn get_type(&mut self, state: StateBinding) -> PhysicalStateId {
        self.events.push(Event::GetType(state));
        self.reported_type
    }

    fn exit(&mut self, call: StateCall) {
        self.events.push(Event::Exit(call));
    }

    fn enter(&mut self, call: StateCall) {
        self.events.push(Event::Enter(call));
    }
}

impl SkateboardControllerActions for Recorder {
    fn hold_skateboard(&mut self) {
        self.events.push(Event::Hold);
    }

    fn let_go_of_skateboard(&mut self) {
        self.events.push(Event::LetGo);
    }
}

fn seeded_data() -> StateChangeData {
    StateChangeData {
        player: PlayerStateChangeFields {
            word_1312: 0x1312_1312,
            previous_category_latch_1336: 400,
            scalar_1344: 13.44,
        },
        processed: ProcessedStateChangeFields {
            flags_2480: 0,
            requested_state_2500: 0x2500_2500,
            previous_state_2504: 0x2504_2504,
            current_state_2508: 0x2508_2508,
            current_category_2512: 0x2512_2512,
            previous_category_2516: 0x2516_2516,
            previous_category_latch_2520: 0x2520_2520,
            word_2564: 0x2564_2564,
            scalar_2664: 26.64,
        },
        skateboard_controller: SkateboardControllerFields {
            word_444: 444,
            state_448: 0,
            system_on_452: false,
        },
    }
}

fn recorder(reported_type: PhysicalStateId) -> Recorder {
    Recorder {
        reported_type,
        events: Vec::new(),
    }
}

#[test]
fn exits_old_pointer_before_switching_and_enters_new_pointer_afterward() {
    let old = StateBinding::new(PhysicalStateId::PhysicsGround);
    let new = StateBinding::new(PhysicalStateId::PhysicsAir);
    let mut lifecycle = PhysicalPlayerStateLifecycle::new(old.state);
    let mut data = seeded_data();
    let mut states = recorder(old.state);
    let mut controller = recorder(old.state);

    assert_eq!(
        lifecycle
            .set_physics_state(new.state as u32, &mut data, &mut states, &mut controller)
            .unwrap(),
        new
    );
    assert_eq!(
        states.events,
        [
            Event::GetType(old),
            Event::Exit(StateCall {
                state: old,
                active: old,
            }),
            Event::Enter(StateCall {
                state: new,
                active: new,
            }),
        ]
    );
    assert_eq!(lifecycle.active(), new);
}

#[test]
fn publishes_exact_state_history_and_resets_transition_fields() {
    let mut lifecycle = PhysicalPlayerStateLifecycle::new(PhysicalStateId::PhysicsGround);
    let mut data = seeded_data();
    let mut states = recorder(PhysicalStateId::PhysicsGround);
    let mut controller = recorder(PhysicalStateId::PhysicsGround);

    lifecycle
        .set_physics_state(
            PhysicalStateId::PhysicsAir as u32,
            &mut data,
            &mut states,
            &mut controller,
        )
        .unwrap();

    assert_eq!(data.player.word_1312, 0);
    assert_eq!(data.player.scalar_1344.to_bits(), 0.0f32.to_bits());
    assert_eq!(data.processed.scalar_2664.to_bits(), 0.0f32.to_bits());
    assert_eq!(data.processed.word_2564, 0);
    assert_eq!(data.processed.previous_state_2504, 100);
    assert_eq!(data.processed.previous_category_2516, 100);
    assert_eq!(data.processed.requested_state_2500, 200);
    assert_eq!(data.processed.current_state_2508, 200);
    assert_eq!(data.processed.current_category_2512, 200);
    assert_eq!(data.player.previous_category_latch_1336, 100);
    assert_eq!(data.processed.previous_category_latch_2520, 100);

    states.reported_type = PhysicalStateId::PhysicsAir;
    data.player.previous_category_latch_1336 = 700;
    lifecycle
        .set_physics_state(
            PhysicalStateId::KnownAir as u32,
            &mut data,
            &mut states,
            &mut controller,
        )
        .unwrap();
    assert_eq!(data.processed.previous_state_2504, 200);
    assert_eq!(data.processed.previous_category_2516, 200);
    assert_eq!(data.processed.current_state_2508, 201);
    assert_eq!(data.processed.current_category_2512, 200);
    assert_eq!(data.player.previous_category_latch_1336, 700);
    assert_eq!(data.processed.previous_category_latch_2520, 700);
}

#[test]
fn every_native_state_selects_its_exact_owned_pointer_slot() {
    let expected = [
        (PhysicalStateId::PhysicsGround, 1696),
        (PhysicalStateId::SlideGround, 1700),
        (PhysicalStateId::RevertGround, 1704),
        (PhysicalStateId::GroundAnimation, 1708),
        (PhysicalStateId::Skitching, 1772),
        (PhysicalStateId::FollowPath, 1760),
        (PhysicalStateId::PhysicsAir, 1712),
        (PhysicalStateId::KnownAir, 1720),
        (PhysicalStateId::PhysicsAirSecondary, 1716),
        (PhysicalStateId::WipeoutGround, 1748),
        (PhysicalStateId::GrindBoardslide, 1728),
        (PhysicalStateId::GrindFiftyFifty, 1732),
        (PhysicalStateId::GrindTipslide, 1736),
        (PhysicalStateId::GrindFiveO, 1740),
        (PhysicalStateId::GrindBackslash, 1744),
        (PhysicalStateId::GrindDarkslide, 1724),
        (PhysicalStateId::BipedGround, 1764),
        (PhysicalStateId::BipedAir, 1768),
        (PhysicalStateId::OffBoardPushing, 1776),
        (PhysicalStateId::LandingOnDeck, 1792),
        (PhysicalStateId::HandPlant, 1780),
        (PhysicalStateId::FootPlant, 1784),
        (PhysicalStateId::Boneless, 1788),
        (PhysicalStateId::Sleeping, 1692),
        (PhysicalStateId::Nonspecific, 1752),
        (PhysicalStateId::Teleporting, 1756),
    ];

    for (requested, owner_offset) in expected {
        let mut lifecycle = PhysicalPlayerStateLifecycle::new(PhysicalStateId::Sleeping);
        let mut data = seeded_data();
        let mut states = recorder(PhysicalStateId::Sleeping);
        let mut controller = recorder(PhysicalStateId::Sleeping);
        let selected = lifecycle
            .set_physics_state(requested as u32, &mut data, &mut states, &mut controller)
            .unwrap();
        assert_eq!(
            selected,
            StateBinding {
                state: requested,
                owner_offset
            }
        );
        assert_eq!(
            states.events.last(),
            Some(&Event::Enter(StateCall {
                state: selected,
                active: selected,
            }))
        );
    }
}

#[test]
fn invalid_ids_fail_before_callbacks_writes_or_pointer_changes() {
    let mut lifecycle = PhysicalPlayerStateLifecycle::new(PhysicalStateId::PhysicsGround);
    let active_before = lifecycle.active();
    let mut data = seeded_data();
    let data_before = data;
    let mut states = recorder(PhysicalStateId::PhysicsGround);
    let mut controller = recorder(PhysicalStateId::PhysicsGround);

    assert_eq!(
        lifecycle.set_physics_state(203, &mut data, &mut states, &mut controller),
        Err(UnknownPhysicalState(203))
    );
    assert_eq!(lifecycle.active(), active_before);
    assert_eq!(data, data_before);
    assert!(states.events.is_empty());
    assert!(controller.events.is_empty());
}

#[test]
fn offboard_entry_chooses_held_or_free_mode_from_native_flag_bits() {
    let mut held_lifecycle = PhysicalPlayerStateLifecycle::new(PhysicalStateId::PhysicsGround);
    let mut held_data = seeded_data();
    held_data.skateboard_controller.state_448 = 4;
    let mut held_states = recorder(PhysicalStateId::PhysicsGround);
    let mut held_actions = recorder(PhysicalStateId::PhysicsGround);
    held_lifecycle
        .set_physics_state(
            PhysicalStateId::BipedGround as u32,
            &mut held_data,
            &mut held_states,
            &mut held_actions,
        )
        .unwrap();
    assert_eq!(held_actions.events, [Event::Hold]);
    assert_eq!(held_data.skateboard_controller.word_444, 0);
    assert_eq!(held_data.skateboard_controller.state_448, 1);
    assert!(held_data.skateboard_controller.system_on_452);

    let mut free_lifecycle = PhysicalPlayerStateLifecycle::new(PhysicalStateId::PhysicsGround);
    let mut free_data = seeded_data();
    free_data.processed.flags_2480 = 0x80;
    free_data.skateboard_controller.state_448 = 1;
    let mut free_states = recorder(PhysicalStateId::PhysicsGround);
    let mut free_actions = recorder(PhysicalStateId::PhysicsGround);
    free_lifecycle
        .set_physics_state(
            PhysicalStateId::OffBoardPushing as u32,
            &mut free_data,
            &mut free_states,
            &mut free_actions,
        )
        .unwrap();
    assert_eq!(free_actions.events, [Event::LetGo]);
    assert_eq!(free_data.skateboard_controller.word_444, 0);
    assert_eq!(free_data.skateboard_controller.state_448, 2);
    assert!(free_data.skateboard_controller.system_on_452);
}

#[test]
fn onboard_entry_runs_stop_controller_before_state_callbacks() {
    let mut lifecycle = PhysicalPlayerStateLifecycle::new(PhysicalStateId::BipedGround);
    let mut data = seeded_data();
    data.skateboard_controller = SkateboardControllerFields {
        word_444: 99,
        state_448: 1,
        system_on_452: true,
    };
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut states = SharedRecorder {
        reported_type: PhysicalStateId::BipedGround,
        events: events.clone(),
    };
    let mut actions = states.clone();

    lifecycle
        .set_physics_state(
            PhysicalStateId::PhysicsGround as u32,
            &mut data,
            &mut states,
            &mut actions,
        )
        .unwrap();

    assert_eq!(data.skateboard_controller.word_444, 0);
    assert_eq!(data.skateboard_controller.state_448, 0);
    assert!(!data.skateboard_controller.system_on_452);
    let old = StateBinding::new(PhysicalStateId::BipedGround);
    let new = StateBinding::new(PhysicalStateId::PhysicsGround);
    assert_eq!(
        *events.borrow(),
        [
            Event::LetGo,
            Event::GetType(old),
            Event::Exit(StateCall {
                state: old,
                active: old,
            }),
            Event::Enter(StateCall {
                state: new,
                active: new,
            }),
        ]
    );
}

#[test]
fn an_already_running_offboard_controller_is_left_untouched() {
    let mut lifecycle = PhysicalPlayerStateLifecycle::new(PhysicalStateId::BipedGround);
    let mut data = seeded_data();
    data.processed.flags_2480 = 0x180;
    data.skateboard_controller = SkateboardControllerFields {
        word_444: 99,
        state_448: 1,
        system_on_452: true,
    };
    let mut states = recorder(PhysicalStateId::BipedGround);
    let mut actions = recorder(PhysicalStateId::BipedGround);

    lifecycle
        .set_physics_state(
            PhysicalStateId::BipedAir as u32,
            &mut data,
            &mut states,
            &mut actions,
        )
        .unwrap();

    assert!(actions.events.is_empty());
    assert_eq!(
        data.skateboard_controller,
        SkateboardControllerFields {
            word_444: 99,
            state_448: 1,
            system_on_452: true,
        }
    );
}
