use super::*;
use crate::point_graph::PointGraph;
fn curves() -> PushAnimationCurves {
    let linear = PointGraph {
        x: std::array::from_fn(|i| i as f32 / 7.0),
        y: std::array::from_fn(|i| i as f32),
    };
    let constant = PointGraph {
        x: linear.x,
        y: [1.0; 8],
    };
    PushAnimationCurves {
        button_time_max: constant,
        button_time_to_dv: linear,
        blend_speed_over_frames: constant,
        blend_acc_over_frames: constant,
    }
}
fn state() -> PushState {
    let zero = PushBlendParameters {
        hstr_vel_b: 0.0,
        lstr_vel_b: 0.0,
        vel_e: 0.0,
    };
    PushState {
        out_factor: 0.375,
        current_push_dv: 0.0,
        current: zero,
        target: zero,
        continue_push: false,
    }
}
fn clips() -> PushAttributes {
    PushAttributes::from_clips([
        PushClipMetrics {
            length: 1.0,
            begin_velocity: 0.0,
            end_velocity: 4.0,
        },
        PushClipMetrics {
            length: 2.0,
            begin_velocity: 4.0,
            end_velocity: 8.0,
        },
        PushClipMetrics {
            length: 1.0,
            begin_velocity: 0.0,
            end_velocity: 1.0,
        },
        PushClipMetrics {
            length: 2.0,
            begin_velocity: 4.0,
            end_velocity: 5.0,
        },
    ])
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.000002, "{a} != {b}");
}

#[test]
fn unequal_clip_lengths_require_distance_weighted_velocity_blend() {
    let target = clips().target(2.0, 2.5);
    // At coefficient1/3, equal weighted durations make the begin velocity2.
    // The strong/gentle end velocities are6 and3, so their midpoint is4.5.
    near(target.hstr_vel_b, 1.0 / 3.0);
    near(target.lstr_vel_b, 1.0 / 3.0);
    near(target.vel_e, 0.5);
}
#[test]
fn authored_bounds_cap_begin_speed_and_strength_before_blending() {
    let attributes = clips();
    assert_eq!(attributes.minimum_begin_velocity, 0.0);
    assert_eq!(attributes.maximum_begin_velocity, 4.0);
    assert_eq!(attributes.minimum_delta_velocity, 1.0);
    assert_eq!(attributes.maximum_delta_velocity, 4.0);
    assert_eq!(
        attributes.target(-5.0, -3.0),
        PushBlendParameters {
            hstr_vel_b: 0.0,
            lstr_vel_b: 0.0,
            vel_e: 0.0
        }
    );
    assert_eq!(
        attributes.target(10.0, 20.0),
        PushBlendParameters {
            hstr_vel_b: 1.0,
            lstr_vel_b: 1.0,
            vel_e: 1.0
        }
    );
}
#[test]
fn out_factor_combines_normalized_speed_strength_then_caps_at_stock_limit() {
    near(clips().out_factor(2.5, 2.5, 0.4, 0.659), 0.5);
    assert_eq!(clips().out_factor(50.0, 50.0, 0.4, 0.659), 0.659);
    assert_eq!(clips().out_factor(-20.0, -20.0, 0.4, 0.659), 0.0);
}
#[test]
fn init_push_preserves_out_factor_and_marks_all_blends_uninitialized() {
    let mut state = state();
    state.current_push_dv = 4.0;
    state.continue_push = true;
    state.initialize_push();
    assert_eq!(state.out_factor, 0.375);
    assert_eq!(state.current_push_dv, 0.0);
    assert!(!state.continue_push);
    assert_eq!(state.current, state.target);
    assert_eq!(state.current.vel_e, -1.0);
}
#[test]
fn teleport_push_evaluates_previous_held_time_before_advancing() {
    let mut first = FirstPushStrength::begin(0.1, 0.5);
    let mut state = state();
    first.update(&mut state, &curves(), None, 0.0, 0.25);
    assert_eq!(state.current_push_dv, 0.0);
    assert_eq!(first.simulated_held_seconds, 0.25);
    assert!(!first.first_push && !first.first_update);
    first.update(&mut state, &curves(), None, 0.0, 0.25);
    near(state.current_push_dv, 1.75);
}
#[test]
fn first_push_release_latches_until_fresh_activation() {
    let mut first = FirstPushStrength::begin(0.5, 0.5);
    assert!(!first.push_from_teleport);
    let mut state = state();
    first.update(&mut state, &curves(), Some(0.25), 0.0, 0.1);
    let strength = state.current_push_dv;
    first.update(&mut state, &curves(), None, 0.0, 0.1);
    first.update(&mut state, &curves(), Some(1.0), 0.0, 0.1);
    assert_eq!(state.current_push_dv, strength);
}
#[test]
fn holding_first_cap_can_reduce_strength_but_next_hold_is_uncapped() {
    let mut state = state();
    state.current_push_dv = 6.0;
    let mut cycle = PushCycle::begin(
        &mut state,
        &curves(),
        PushIntents {
            pushing: Some(1.0),
            configured_foot: true,
            new_push: false,
        },
    );
    cycle.update(
        &mut state,
        &curves(),
        PushIntents {
            pushing: Some(1.0),
            configured_foot: true,
            new_push: false,
        },
        0.0,
        4.0,
    );
    assert_eq!(state.current_push_dv, 4.0);
    cycle.update(&mut state, &curves(), PushIntents::default(), 0.0, 4.0);
    assert!(!state.continue_push);
    cycle.update(
        &mut state,
        &curves(),
        PushIntents {
            pushing: Some(1.0),
            configured_foot: true,
            new_push: true,
        },
        0.0,
        4.0,
    );
    assert_eq!(state.current_push_dv, 7.0);
    assert_eq!(cycle.phase, PushCyclePhase::HoldingNext);
    state.current_push_dv = 9.0;
    cycle.update(&mut state, &curves(), PushIntents::default(), 0.0, 4.0);
    assert_eq!(state.current_push_dv, 7.0);
    assert_eq!(cycle.phase, PushCyclePhase::NextReleased);
    // Release does not clear the continuation set in the previous next-hold.
    assert!(state.continue_push);
}
#[test]
fn released_first_and_new_push_checks_can_cascade_in_one_update() {
    let mut state = state();
    let mut cycle = PushCycle::begin(
        &mut state,
        &curves(),
        PushIntents {
            pushing: Some(0.25),
            configured_foot: true,
            new_push: false,
        },
    );
    cycle.update(
        &mut state,
        &curves(),
        PushIntents {
            pushing: None,
            configured_foot: true,
            new_push: true,
        },
        0.0,
        4.0,
    );
    assert_eq!(cycle.phase, PushCyclePhase::HoldingNext);
    assert!(state.continue_push);
    assert_eq!(cycle.next_push_dv, 0.0);
}
#[test]
fn out_distance_uses_live_foot_and_deck_axes_with_only_z_flipped() {
    let mut frame = PushFootFrame {
        left_foot: [11.0, 22.0, 33.0, 0.0],
        right_foot: [14.0, 25.0, 36.0, 0.0],
        deck_position: [10.0, 20.0, 30.0, 0.0],
        deck_y: [0.0, 1.0, 0.0, 0.0],
        deck_z: [1.0, 0.0, 0.0, 0.0],
        skateboard_flipped: false,
    };
    assert_eq!(frame.out_distance(false), [1.0, 2.0]);
    assert_eq!(frame.out_distance(true), [4.0, 5.0]);
    frame.skateboard_flipped = true;
    assert_eq!(frame.out_distance(true), [-4.0, 5.0]);
}
