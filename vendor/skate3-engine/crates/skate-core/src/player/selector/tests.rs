use super::conditions::{BoardBodyState, SkeletonAnimationState, TwoStageThresholds};
use super::{ProcessedStateInput, StateSelectionInput, StateSelector};
use crate::player::state::PhysicalStateId as State;

fn input(category: u32) -> StateSelectionInput {
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
            category_2512: category,
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
        skitching_off_ground: thresholds(),
        normal_off_ground: thresholds(),
    }
}

const fn thresholds() -> TwoStageThresholds {
    TwoStageThresholds {
        field_856_primary: 1.0,
        field_856_secondary: 1.0,
        field_7692: 1.0,
    }
}

fn calculate(selector: &mut StateSelector, state: State, input: &StateSelectionInput) -> State {
    selector.calculate(state, input)
}

#[test]
fn teleport_request_has_priority_and_fallthrough_sets_the_native_request_byte() {
    let mut selector = StateSelector::default();
    let mut frame = input(100);
    frame.processed.flags_2472 = 0x4_0000;
    assert_eq!(
        calculate(&mut selector, State::PhysicsGround, &frame),
        State::Teleporting
    );

    frame.processed.flags_2472 = 0;
    frame.processed.category_2512 = 200;
    selector.air_frames = 300;
    assert_eq!(
        calculate(&mut selector, State::PhysicsAir, &frame),
        State::PhysicsAir
    );
    assert!(selector.request_teleport);
}

#[test]
fn all_six_grind_types_use_the_exact_native_state_mapping() {
    let expected = [
        State::GrindFiftyFifty,
        State::GrindBoardslide,
        State::GrindTipslide,
        State::GrindFiveO,
        State::GrindBackslash,
        State::GrindDarkslide,
    ];
    for (grind_type, expected) in expected.into_iter().enumerate() {
        let mut selector = StateSelector::default();
        let mut frame = input(100);
        frame.processed.grind_candidate_1488 = true;
        frame.processed.grind_type_1248 = grind_type as u32;
        assert_eq!(
            calculate(&mut selector, State::PhysicsGround, &frame),
            expected
        );
    }
}

#[test]
fn nonspecific_uses_exact_two_three_and_collision_counter_thresholds() {
    let mut free = StateSelector::default();
    let frame = input(700);
    assert_eq!(
        calculate(&mut free, State::Nonspecific, &frame),
        State::Nonspecific
    );
    assert_eq!(
        calculate(&mut free, State::Nonspecific, &frame),
        State::Nonspecific
    );
    assert_eq!(
        calculate(&mut free, State::Nonspecific, &frame),
        State::PhysicsAir
    );

    let mut two_wheels = StateSelector::default();
    let mut frame = input(700);
    frame.skateboard_contact_count_869 = 2;
    for _ in 0..2 {
        assert_eq!(
            calculate(&mut two_wheels, State::Nonspecific, &frame),
            State::Nonspecific
        );
    }
    assert_eq!(
        calculate(&mut two_wheels, State::Nonspecific, &frame),
        State::PhysicsGround
    );

    let mut collision = StateSelector::default();
    let mut frame = input(700);
    frame.processed.flags_2468 = 0x1_0000;
    for _ in 0..60 {
        assert_eq!(
            calculate(&mut collision, State::Nonspecific, &frame),
            State::Nonspecific
        );
    }
    assert_eq!(
        calculate(&mut collision, State::Nonspecific, &frame),
        State::PhysicsGround
    );
}

#[test]
fn nonspecific_short_collision_threshold_comes_from_flags_2484_bit_21() {
    let mut selector = StateSelector::default();
    let mut frame = input(700);
    frame.processed.flags_2468 = 0x1_0000;
    frame.processed.flags_2484 = 0x20_0000;
    for _ in 0..30 {
        assert_eq!(
            calculate(&mut selector, State::Nonspecific, &frame),
            State::Nonspecific
        );
    }
    assert_eq!(
        calculate(&mut selector, State::Nonspecific, &frame),
        State::PhysicsGround
    );
}

#[test]
fn ground_to_landing_on_deck_takes_the_native_noop_call_branch() {
    let mut selector = StateSelector::default();
    let mut frame = input(100);
    frame.processed.flags_2480 = 0x1000;
    let state = selector.calculate(State::PhysicsGround, &frame);
    assert_eq!(state, State::LandingOnDeck);
}

#[test]
fn revert_normal_exit_sets_the_recovered_state_changer_byte() {
    let mut selector = StateSelector::default();
    let frame = input(100);
    assert_eq!(
        calculate(&mut selector, State::RevertGround, &frame),
        State::PhysicsGround
    );
    assert!(selector.revert_exited_normally);
}

#[test]
fn known_air_transition_order_matches_the_tu3_tree() {
    let mut selector = StateSelector::default();
    let mut frame = input(200);
    frame.processed.flags_2480 = 0x200_0000;
    assert_eq!(
        calculate(&mut selector, State::KnownAir, &frame),
        State::FootPlant
    );

    frame.processed.flags_2480 = 0;
    frame.processed.flags_2476 = 0x80;
    frame.processed.trajectory_collision_time_2772 = 0.04;
    assert_eq!(
        calculate(&mut selector, State::KnownAir, &frame),
        State::WipeoutGround
    );
}

#[test]
fn physics_air_collision_resolves_before_the_grind_candidate_path() {
    let mut selector = StateSelector {
        post_grind_jump_counter: 11,
        ..StateSelector::default()
    };
    let mut frame = input(200);
    frame.processed.state_timer_2664 = 0.1;
    frame.processed.flags_2468 = 0x1_0000;
    frame.processed.grind_candidate_1488 = true;
    frame.processed.grind_type_1248 = 0;
    assert_eq!(
        calculate(&mut selector, State::PhysicsAir, &frame),
        State::PhysicsGround
    );
}

#[test]
fn skitch_exit_countdown_is_loaded_to_ten_then_saturates() {
    let mut selector = StateSelector::default();
    let mut frame = input(100);
    frame.processed.flags_2476 = 0x20_0000;
    assert_eq!(
        calculate(&mut selector, State::Skitching, &frame),
        State::Skitching
    );
    assert_eq!(selector.skitch_exit_countdown, 10);

    frame.processed.flags_2476 = 0;
    let _ = calculate(&mut selector, State::PhysicsGround, &frame);
    assert_eq!(selector.skitch_exit_countdown, 9);
}

#[test]
fn check_for_grind_rejects_both_native_flags() {
    for rejected in [0x2_0000, 0x400_0000] {
        let mut selector = StateSelector::default();
        let mut frame = input(100);
        frame.processed.grind_candidate_1488 = true;
        frame.processed.grind_type_1248 = 0;
        frame.processed.flags_2480 = rejected;
        assert_eq!(
            calculate(&mut selector, State::PhysicsGround, &frame),
            State::PhysicsGround
        );
    }
}

#[test]
fn skeleton_animation_predicate_uses_mode_and_sign_cleared_lane() {
    use super::conditions::is_skateboard_animated;

    assert!(is_skateboard_animated(SkeletonAnimationState {
        mode_16420: 2,
        back_chain_lane_12656: 0.0,
        threshold_800: 100.0,
    }));
    assert!(is_skateboard_animated(SkeletonAnimationState {
        mode_16420: 0,
        back_chain_lane_12656: -2.0,
        threshold_800: 1.0,
    }));
    assert!(!is_skateboard_animated(SkeletonAnimationState {
        mode_16420: 1,
        back_chain_lane_12656: -2.0,
        threshold_800: 1.0,
    }));
    assert!(is_skateboard_animated(SkeletonAnimationState {
        mode_16420: 0,
        back_chain_lane_12656: f32::NAN,
        threshold_800: 1.0,
    }));
}
