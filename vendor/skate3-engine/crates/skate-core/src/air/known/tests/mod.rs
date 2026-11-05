use super::*;

mod mock;

use mock::{MockRuntime, frame, mode, output, reckoning, settings, state};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_01,
        "{actual} != {expected}"
    );
}

#[test]
fn init_trajectory_info_builds_stock_landing_values() {
    let mut state = state();
    let mut frame = frame();
    frame.selector_landing_normal_2656 = [0.0, 1.0, 0.0, 0.0];
    let mut settings = settings();
    settings.flip_scalar = 0.5;
    settings.min_target_heading_velocity_420 = 0.5;
    let mode = mode();
    let mut runtime = MockRuntime::new();
    runtime.prediction.trajectory.velocity = [3.0, 4.0, 0.0, 0.0];
    runtime.prediction.trajectory.scalar_48 = 9.0; // Independent query horizon.
    runtime.prediction.collision_time_48 = 2.0;
    runtime.prediction.collision_frame_128 = 3;
    runtime.contact = [9.0, 8.0, 7.0, 0.0];
    runtime.selector_vector = [6.0, 5.0, 4.0, 0.0];
    runtime.highest = ([1.0, 12.0, 3.0, 0.0], 0.75);

    init_trajectory_info(&mut state, &frame, &settings, &mode, &mut runtime);

    assert_eq!(state.trajectory_apex_96, runtime.highest.0);
    assert_eq!(state.time_to_apex_200, 0.75);
    assert_eq!(state.collision_position_112, runtime.contact);
    assert_eq!(state.selector_vector_128, runtime.selector_vector);
    assert_close(state.collision_normal_speed_176, 4.0);
    assert_eq!(state.landing_heading_80, [1.0, 0.0, 0.0, 0.0]);
    assert!(state.landing_heading_valid_209);
    assert_close(
        state.body_flip_target_speed_204,
        -core::f32::consts::FRAC_PI_2,
    );
}

#[test]
fn init_negative_collision_clears_speed_and_heading_validity() {
    let mut state = state();
    state.collision_normal_speed_176 = 99.0;
    state.landing_heading_valid_209 = true;
    let frame = frame();
    let settings = settings();
    let mode = mode();
    let mut runtime = MockRuntime::new();
    runtime.prediction.collision_time_48 = -1.0;

    init_trajectory_info(&mut state, &frame, &settings, &mode, &mut runtime);

    assert_eq!(state.collision_normal_speed_176, 0.0);
    assert_eq!(state.landing_heading_80, [0.0; 4]);
    assert!(!state.landing_heading_valid_209);
}

#[test]
fn enter_preserves_recovered_effect_order_and_initializes_heights() {
    let mut state = state();
    let mut frame = frame();
    frame.start_height_484 = 3.0;
    frame.velocity_400 = [-1.0, 0.0, 0.0, 0.0];
    frame.start_flip_reference_96 = [1.0, 0.0, 0.0, 0.0];
    let settings = settings();
    let mode = mode();
    let mut runtime = MockRuntime::new();
    runtime.board_y = 4.0;
    runtime.com = [0.0, 5.0, 0.0, 0.0];
    runtime.targeting_grind = true;
    runtime.closest = (11, [0.25, 0.0, 0.0, 0.0]);

    enter(&mut state, &frame, &settings, &mode, &mut runtime);

    assert_eq!(state.start_y_184, 3.0);
    assert_eq!(state.max_y_188, 4.0);
    assert_eq!(state.com_max_y_192, 5.0);
    assert_eq!(state.trajectory_index_216, 11);
    assert_eq!(state.trajectory_follow_offset_144, runtime.closest.1);
    assert!(state.targeting_grind_213);
    assert!(state.start_flipped_210);
    assert_eq!(
        &runtime.calls[..9],
        [
            "board.angular_only",
            "collision.enabled:true",
            "collision.state:7",
            "skeleton.truck_tilt:false",
            "skeleton.ik:true",
            "reckoning.reset_flip",
            "footplant.flag:false",
            "footplant.reset",
            "board.height",
        ]
    );
    assert!(runtime.calls.iter().any(|call| call == "grind.start"));
}

#[test]
fn exit_restores_velocity_before_selector_and_grind_cleanup() {
    let mut state = state();
    state.trajectory_index_216 = 0;
    let mut frame = frame();
    frame.next_physics_state_2500 = 100;
    frame.ground_normal_464 = [0.0, 1.0, 0.0, 0.0];
    let mut settings = settings();
    settings.landing_speed_scalar_vs_ground_normal_y_240 = mock::flat_graph(2.0);
    let mut reckoning = reckoning();
    reckoning.body_spin_angle_1568 = 2.0;
    reckoning.body_spin_speed_1572 = 3.0;
    let mut runtime = MockRuntime::new();
    runtime.restore_geometry = RestoreVelocityGeometry {
        tangential_velocity: [2.0, 0.0, 0.0, 0.0],
        landing_speed_curve_input: 0.0,
        landing_speed_blend_source: 0.5,
    };
    runtime.board_velocity = [0.0, 1.0, 0.0, 0.0];
    runtime.forward = [1.0, 0.0, 0.0, 0.0];

    exit(
        &mut state,
        &mut frame,
        &settings,
        &mut reckoning,
        &mut runtime,
    );

    assert_eq!(state.trajectory_index_216, 1);
    assert_eq!(runtime.written_board_velocity, [3.0, 1.0, 0.0, 0.0]);
    assert_eq!(frame.forward_speed_2612, 3.0);
    assert_eq!(reckoning.body_spin_speed_1572, 0.0);
    assert_eq!(reckoning.body_spin_angle_1568, 0.0);
    let reset = runtime
        .calls
        .iter()
        .position(|v| v == "reckoning.reset_flip")
        .unwrap();
    let write = runtime
        .calls
        .iter()
        .position(|v| v == "board.set_velocity")
        .unwrap();
    let selector = runtime
        .calls
        .iter()
        .position(|v| v == "selector.reset")
        .unwrap();
    let grind = runtime
        .calls
        .iter()
        .position(|v| v == "grind.started:false")
        .unwrap();
    assert!(reset < write && write < selector && selector < grind);
}

#[test]
fn update_runs_footplant_grind_and_publication_in_native_order() {
    let mut state = state();
    state.time_in_state_180 = 0.2;
    state.collision_time_196 = 0.25;
    state.reached_apex_208 = true;
    state.targeting_grind_213 = true;
    state.trajectory_index_216 = 4;
    let mut frame = frame();
    frame.flags_2480 = 0x0400_0000;
    frame.delta_time_2604 = 1.0 / 60.0;
    let settings = settings();
    let mode = mode();
    let reckoning = reckoning();
    let mut runtime = MockRuntime::new();
    runtime.grind_started = true;
    runtime.grind_adjusting = true;
    runtime.board_y = 7.0;
    runtime.com = [0.0, 8.0, 0.0, 0.0];

    update(
        &mut state,
        &frame,
        &settings,
        &mode,
        &reckoning,
        &mut runtime,
    );

    assert!(state.grind_air_adjust_activated_212);
    assert_eq!(state.trajectory_index_216, 5);
    assert_close(state.time_in_state_180, 0.2 + frame.delta_time_2604);
    assert_eq!(state.max_y_188, 7.0);
    assert_eq!(state.com_max_y_192, 8.0);
    let expected_bodies = ["3152", "3156", "3160", "3168", "3172", "3176"];
    let actual_bodies: Vec<_> = runtime
        .calls
        .iter()
        .filter_map(|call| call.strip_prefix("grind.disable:"))
        .collect();
    assert_eq!(actual_bodies, expected_bodies);
    let foot = runtime
        .calls
        .iter()
        .position(|v| v == "footplant.shift")
        .unwrap();
    let collision = runtime
        .calls
        .iter()
        .position(|v| v == "collision.update")
        .unwrap();
    let reckon = runtime
        .calls
        .iter()
        .position(|v| v == "reckoning.update_air")
        .unwrap();
    let skeleton = runtime
        .calls
        .iter()
        .position(|v| v == "skeleton.update_known_air")
        .unwrap();
    let head = runtime
        .calls
        .iter()
        .position(|v| v == "head.target:true")
        .unwrap();
    let steering = runtime
        .calls
        .iter()
        .position(|v| v == "board.steering_zero")
        .unwrap();
    assert!(
        foot < collision
            && collision < reckon
            && reckon < skeleton
            && skeleton < head
            && head < steering
    );
}

#[test]
fn trajectory_follow_uses_selector_2704_and_honors_locked_index() {
    let mut state = state();
    state.trajectory_index_216 = 0;
    state.time_in_state_180 = 2.0;
    let mut frame = frame();
    frame.flags_2468 = 0x0000_0008;
    let settings = settings();
    let mut runtime = MockRuntime::new();
    runtime.prediction.trajectory.position = [100.0, 0.0, 0.0, 0.0];
    runtime.selector_trajectory.position = [10.0, 0.0, 0.0, 0.0];

    update_trajectory_follow(&mut state, &frame, &settings, &runtime);

    assert_eq!(state.trajectory_index_216, 0);
    assert_eq!(state.target_com_position_160, [10.0, 0.0, 0.0, 0.0]);
}

#[test]
fn auto_spin_uses_recovered_alignment_chain_and_manual_gate() {
    let mut state = state();
    state.landing_heading_valid_209 = true;
    state.landing_normal_64 = [0.0, 1.0, 0.0, 0.0];
    state.landing_heading_80 = [1.0, 0.0, 0.0, 0.0];
    state.collision_time_196 = 1.0;
    let mut frame = frame();
    frame.delta_time_2604 = 1.0 / 60.0;
    frame.skater_up_544 = [0.0, 1.0, 0.0, 0.0];
    let mut settings = settings();
    settings.max_heading_adjust_vs_up_y_160 = mock::flat_graph(180.0);
    settings.min_auto_body_speed_424 = 0.01;
    let mode = mode();
    let reckoning = reckoning();
    let mut runtime = MockRuntime::new();
    runtime.signed_angle = 0.5;

    let automatic =
        calculate_body_spin_speed(&state, &frame, &settings, &mode, &reckoning, &mut runtime);
    assert_close(automatic, 0.6);

    frame.body_spin_input_2640 = 1.0;
    frame.flags_2472 = 0;
    runtime.calls.clear();
    let masked_manual =
        calculate_body_spin_speed(&state, &frame, &settings, &mode, &reckoning, &mut runtime);
    assert_eq!(masked_manual, 0.0);
    assert!(!runtime.calls.iter().any(|v| v == "math.signed_angle"));
}

#[test]
fn body_flip_starts_only_after_stock_flag_and_curve_gates() {
    let mut state = state();
    state.collision_time_196 = 2.0;
    state.body_flip_target_speed_204 = -3.0;
    let mut frame = frame();
    frame.flags_2468 = 0x20 | 0x80;
    let mut settings = settings();
    settings.flip_start_collision_time_vs_normal_y = mock::flat_graph(1.0);
    let mode = mode();
    let reckoning = reckoning();
    let mut runtime = MockRuntime::new();

    let speed = calculate_body_flip_speed(
        &mut state,
        &frame,
        &settings,
        &mode,
        &reckoning,
        &mut runtime,
    );

    assert!(state.body_flipping_211);
    assert_eq!(speed, -3.0);
    assert!(
        runtime
            .calls
            .iter()
            .any(|v| v == "reckoning.begin_flip:true")
    );

    state.body_flipping_211 = false;
    frame.flags_2488 = 0x0200_0000;
    runtime.calls.clear();
    calculate_body_flip_speed(
        &mut state,
        &frame,
        &settings,
        &mode,
        &reckoning,
        &mut runtime,
    );
    assert!(!state.body_flipping_211);
    assert!(runtime.calls.is_empty());
}

#[test]
fn post_physics_sets_apex_then_requests_upside_down_wipeout() {
    let mut state = state();
    state.landing_normal_64 = [0.0, 1.0, 0.0, 0.0];
    let mut frame = frame();
    frame.skater_up_544 = [0.0, -1.0, 0.0, 0.0];
    let mode = mode();
    let wipeout_settings = KnownAirWipeoutSettings {
        air_falling_min_up_y_260: 0.0,
        air_falling_max_angle_264: 0.0,
    };
    let mut request = KnownAirWipeoutRequest {
        requested_35: false,
        scalar_116: 9.0,
        counter_200: 4,
    };
    let mut runtime = MockRuntime::new();
    runtime.board_velocity = [0.0, -2.0, 0.0, 0.0];

    update_post_physics(
        &mut state,
        &frame,
        &mode,
        &wipeout_settings,
        &mut request,
        &mut runtime,
    );

    assert!(state.reached_apex_208);
    assert!(request.requested_35);
    assert_eq!(request.scalar_116, 0.0);
    assert_eq!(request.counter_200, 5);
    assert_eq!(runtime.calls[0], "board.velocity");
    assert_eq!(runtime.calls[1], "wipeout.check:true");
}

#[test]
fn output_writes_all_samples_and_preserves_conditional_fields() {
    let mut state = state();
    state.trajectory_index_216 = 2;
    state.collision_position_112 = [1.0, 0.0, 0.0, 0.0];
    state.landing_normal_64 = [1.0, 0.0, 0.0, 0.0];
    state.max_y_188 = 7.0;
    state.start_y_184 = 2.0;
    state.collision_time_196 = 3.0;
    state.time_in_state_180 = 1.0;
    let frame = frame();
    let mut output = output();
    output.locked_trajectory_velocity_valid_452 = true;
    output.locked_trajectory_velocity_160 = [9.0; 4];
    let mut runtime = MockRuntime::new();
    runtime.prediction.trajectory.position = [0.0; 4];
    runtime.prediction.trajectory.velocity = [60.0, 0.0, 0.0, 0.0];
    runtime.selector_trajectory.position = [999.0, 0.0, 0.0, 0.0];
    runtime.selector_com = [0.0; 4];

    fill_physics_output(&state, &frame, &mut output, &mut runtime);

    assert_eq!(output.locked_trajectory_velocity_160, [9.0; 4]);
    assert!(output.locked_trajectory_velocity_valid_452);
    assert_eq!(output.jump_height_200, 5.0);
    assert_eq!(output.time_until_collision_184, 2.0);
    assert_eq!(output.trajectory_position_64[0], 2.0);
    assert_eq!(output.trajectory_plane_samples_336[0], 0.0);
    assert_close(output.trajectory_plane_samples_336[24], 24.0);
    assert!(output.known_air_valid_437);
}
