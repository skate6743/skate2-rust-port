use super::*;
use crate::player::selector::{
    ProcessedStateInput,
    conditions::{BoardBodyState, SkeletonAnimationState, TwoStageThresholds},
};

fn input(flags_2484: u32, flags_2476: u32, flags_2480: u32) -> StateSelectionInput {
    let thresholds = TwoStageThresholds {
        field_856_primary: 1.,
        field_856_secondary: 1.,
        field_7692: 1.,
    };
    StateSelectionInput {
        processed: ProcessedStateInput {
            grind_type_1248: 0,
            grind_candidate_1488: false,
            grind_investigation_flags_1516: 0,
            field_1776: 0,
            flags_2468: 0,
            flags_2472: 0,
            flags_2476,
            flags_2480,
            flags_2484,
            flags_2488: 0,
            category_2512: 500,
            wheel_contact_count_2556: 0,
            field_2572: 0,
            state_timer_2664: 0.,
            field_2732: 0.,
            field_2744: 0.,
            trajectory_collision_time_2772: 1.,
        },
        skateboard_contact_count_869: 0,
        board_body: BoardBodyState {
            field_856: 0.,
            field_7692: 0.,
        },
        skeleton: SkeletonAnimationState {
            mode_16420: 0,
            back_chain_lane_12656: 0.,
            threshold_800: 1.,
        },
        normal_off_ground: thresholds,
        skitching_off_ground: thresholds,
    }
}

#[test]
fn biped_air_uses_published_air_flag_not_board_on_ground_animation_flag() {
    for animation_board_on_ground in [0, 1] {
        let mut selector = StateSelector::default();
        let airborne = input(0x1_0000 | animation_board_on_ground, 0x80, 0);
        assert_eq!(
            selector.calculate(PhysicalStateId::BipedAir, &airborne),
            PhysicalStateId::BipedAir
        );
        let landed = input(animation_board_on_ground, 0x80, 0);
        assert_eq!(
            selector.calculate(PhysicalStateId::BipedAir, &landed),
            PhysicalStateId::BipedGround
        );
    }
}

#[test]
fn air_to_ground_does_not_oscillate_while_published_air_flag_remains_set() {
    let mut selector = StateSelector::default();
    let airborne = input(0x1_0000, 0x8000, 0);
    let mut state = PhysicalStateId::BipedGround;
    for _ in 0..60 {
        state = selector.calculate(state, &airborne);
        assert_eq!(state, PhysicalStateId::BipedAir);
    }
    state = selector.calculate(state, &input(0, 0x8000, 0));
    assert_eq!(state, PhysicalStateId::BipedGround);
}

#[test]
fn actual_mount_and_land_on_board_requests_keep_their_native_precedence() {
    let mut selector = StateSelector::default();
    assert_eq!(
        selector.calculate(PhysicalStateId::BipedAir, &input(0x1_0000, 0, 0)),
        PhysicalStateId::PhysicsAir
    );
    assert_eq!(
        selector.calculate(PhysicalStateId::BipedAir, &input(0x1_0000, 0x80, 0x10)),
        PhysicalStateId::LandingOnDeck
    );
}

#[test]
fn airborne_dismount_reads_high_byte_gate_in_both_onboard_air_states() {
    for current in [PhysicalStateId::PhysicsAir, PhysicalStateId::KnownAir] {
        let mut selector = StateSelector::default();
        for board_on_ground in [0, 1] {
            let allowed = input(board_on_ground, 0x80, 0);
            assert_eq!(
                selector.calculate(current, &allowed),
                PhysicalStateId::BipedAir
            );
            let blocked = input(0x0100_0000 | board_on_ground, 0x80, 0);
            assert_ne!(
                selector.calculate(current, &blocked),
                PhysicalStateId::BipedAir
            );
        }
    }
}
