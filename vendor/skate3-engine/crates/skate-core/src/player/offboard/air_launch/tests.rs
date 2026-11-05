use super::*;
fn processed() -> Processed {
    Processed {
        board_position_112: [7.0, 8.0, 9.0, 2.0],
        forward_224: [0.0, 0.0, 1.0, 0.0],
        up_544: UP,
        position_592: [10.0, 20.0, 30.0, 1.0],
        velocity_608: [0.0; 4],
        velocity_912: [0.0; 4],
        departure_geometry: None,
        flags_2472: 0,
        flags_2476: 0,
        flags_2480: 0,
        previous_state_2504: 500,
        current_state_2508: 500,
        current_category_2512: 500,
        previous_category_2516: 500,
        raw_x_2692: 0.0,
        raw_z_2688: 0.0,
    }
}
fn curve() -> PointGraph<8> {
    PointGraph {
        x: [0., 1., 2., 3., 4., 5., 6., 7.],
        y: [90.; 8],
    }
}
#[test]
fn mode4_turn_clamp_reads_angular_velocity_not_edge_position() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.velocity_912 = [0., 0., 4., 0.];
    let mut state = controller::State::new([None; 3]);
    state.motion.angular_velocity_688 = 4.0;
    state.intent.edge_target[0] = -100.0;
    let mut first = Packet::initialized(0.0);
    produce(&mut first, &state, &curve(), settings(), &p, true).unwrap();
    state.intent.edge_target[0] = 100.0;
    let mut second = Packet::initialized(0.0);
    produce(&mut second, &state, &curve(), settings(), &p, true).unwrap();
    //82D7C6C4 loads Biped+688, not the separate edge-position vector+672.
    assert_eq!(first.velocity_0, second.velocity_0);
    state.motion.angular_velocity_688 = -4.0;
    let mut opposite = Packet::initialized(0.0);
    produce(&mut opposite, &state, &curve(), settings(), &p, true).unwrap();
    assert_ne!(second.velocity_0, opposite.velocity_0);
}
fn settings() -> Settings {
    Settings {
        jump_speed_scalar: 0.8,
        jump_height: 0.9,
    }
}
fn run(p: Processed) -> Packet {
    let mut packet = Packet::initialized(f32::NAN);
    produce(
        &mut packet,
        &controller::State::new([None; 3]),
        &curve(),
        settings(),
        &p,
        true,
    )
    .unwrap();
    packet
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 2.0e-5, "{a} != {b}");
}
fn vector(a: Vector, b: Vector) {
    for i in 0..4 {
        near(a[i], b[i]);
    }
}
#[test]
fn all_seven_dispatch_modes_and_previous_state_selector() {
    let mut p = processed();
    for (category, expected) in [(100, 0), (400, 1), (200, 2), (700, 6)] {
        p.current_category_2512 = category;
        assert_eq!(mode(&p, true), expected);
    }
    p.current_category_2512 = 500;
    assert_eq!(mode(&p, true), 5);
    p.flags_2476 = 0x80000;
    assert_eq!(mode(&p, true), 4);
    p.flags_2480 = 0x80;
    assert_eq!(mode(&p, true), 3);
    p.flags_2480 = 0;
    p.current_state_2508 = 503;
    assert_eq!(mode(&p, true), 3);
    p.previous_category_2516 = 200;
    assert_eq!(mode(&p, false), 2);
}
#[test]
fn initializer_preserves_only_unwritten_scalar104() {
    let p = Packet::initialized(123.0);
    assert_eq!(p.scalar_104, 123.0);
    assert_eq!(p.up_48, UP);
    assert_eq!(p.board_position_80, ZERO);
    assert!(!p.flag_117);
}
#[test]
fn passive_modes_publish_common_fields_and_preserve_caller_fields() {
    for category in [200, 700] {
        let mut p = processed();
        p.current_category_2512 = category;
        p.velocity_608 = [1., 2., 3., 4.];
        p.forward_224 = [0., 99., 2., 6.];
        let mut packet = Packet::initialized(f32::NAN);
        packet.board_position_80 = [4.; 4];
        packet.has_board_position_116 = true;
        packet.flag_117 = true;
        produce(
            &mut packet,
            &controller::State::new([None; 3]),
            &curve(),
            settings(),
            &p,
            true,
        )
        .unwrap();
        assert_eq!(packet.velocity_0, p.velocity_608);
        assert_eq!(packet.secondary_velocity_16, p.velocity_608);
        vector(packet.forward_64, [0., 0., 1., 3.]);
        assert_eq!(packet.position_32, p.position_592);
        assert_eq!(packet.scalar_104.to_bits(), 0x3f32_b8c2);
        assert_eq!(packet.board_position_80, [4.; 4]);
        assert!(packet.has_board_position_116 && packet.flag_117);
    }
}
#[test]
fn riding_launch_boost_is_not_a_clamp_and_advances_all_lanes() {
    for (y, want) in [
        (-10., -7.),
        (-2., 1.),
        (0., 3.),
        (2., 3.),
        (3., 3.),
        (8., 8.),
    ] {
        let mut p = processed();
        p.current_category_2512 = 100;
        p.velocity_608 = [2., y, 4., 6.];
        let packet = run(p);
        assert_eq!(packet.velocity_0[1], want);
        assert_eq!(packet.secondary_velocity_16, p.velocity_608);
        assert_eq!(packet.board_position_80, p.board_position_112);
        assert!(packet.has_board_position_116);
        near(packet.position_32[1], want.mul_add(DT, 20.));
        near(packet.position_32[3], 6.0f32.mul_add(DT, 1.));
    }
}
#[test]
fn mode1_projects_away_from_supplied_axis_before_side_impulse() {
    let mut p = processed();
    p.current_category_2512 = 400;
    p.position_592 = [3., 50., 4., 0.];
    p.departure_geometry = Some(DepartureGeometry {
        point_1120: [0.; 4],
        axis_1136: UP,
    });
    let packet = run(p);
    vector(packet.velocity_0, [1.2, 1., 1.6, 0.]);
    assert_eq!(packet.kind_108, 6);
    vector(packet.secondary_velocity_16, ZERO);
}
#[test]
fn mode3_sideways_sign_and_stationary_guard() {
    let mut p = processed();
    p.current_state_2508 = 503;
    assert_eq!(run(p).velocity_0, ZERO);
    p.velocity_608 = [0., 1., 2., 0.];
    near(run(p).velocity_0[0], -0.75);
    p.flags_2476 = 4;
    near(run(p).velocity_0[0], 0.75);
    assert_eq!(run(p).position_32, p.position_592);
}
#[test]
fn mode5_speeds_either_side_and_fast_vertical_caps() {
    let mut p = processed();
    p.velocity_608 = [0., 0., 1.874, 0.];
    p.velocity_912 = [4., 20., 8., 12.];
    let slow = run(p);
    vector(slow.velocity_0, [0., 1., 2.5, 0.]);
    assert_eq!(slow.scalar_96.to_bits(), 0x3f06_0a92);
    //BF84 compares the refined magnitude, not the raw input component.
    //Use a value unambiguously above1.875 for the fast/capping contract.
    p.velocity_608[2] = 1.876;
    let fast = run(p);
    vector(fast.velocity_0, [3., 5., 6., 9.]);
    assert_eq!(fast.scalar_96.to_bits(), 0x3db2_b8c2);
    p.velocity_912[1] = -20.;
    assert_eq!(run(p).velocity_0[1], -10.);
}
#[test]
fn mode5_pc_refinement_at_raw_1875_is_below_comparison_threshold() {
    //Independent Single/FMA evaluation of S3 BF34..BF88 with our documented
    //PC seed: square3.515625; seed3F088889; two refinements3F088888;
    //multiply gives3FEFFFFF, one ULP below threshold3FF00000.
    //This is a regression for our PC arithmetic, NOT Xenon estimate parity.
    let mut p = processed();
    p.velocity_608 = [0., 0., 1.875, 0.];
    p.velocity_912 = [4., 20., 8., 12.];
    assert_eq!(length(p.velocity_608).to_bits(), 0x3fef_ffff);
    let packet = run(p);
    vector(packet.velocity_0, [0., 1., 2.5, 0.]);
    assert_eq!(packet.scalar_96.to_bits(), 0x3f06_0a92);
}
#[test]
fn mode5_stationary_uses_flattened_forward_including_w() {
    let mut p = processed();
    p.forward_224 = [0., 7., 2., 4.];
    vector(run(p).velocity_0, [0., 1., 2.5, 5.]);
}
#[test]
fn jump_stationary_control_and_impulse_use_separate_stock_scalars() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.raw_x_2692 = 0.6;
    p.raw_z_2688 = 0.8;
    let packet = run(p);
    near(packet.velocity_0[0], 0.48);
    near(packet.velocity_0[2], 0.64);
    near(packet.velocity_0[1], (0.9f32 * 19.6).sqrt());
    // Host geometric representation deliberately excludes native scratch W.
    // XYZ stock impulse assertions above remain unchanged.
    assert_eq!(packet.velocity_0[3], 0.);
    vector(packet.secondary_velocity_16, [0.6, 0., 0.8, 0.]);
    assert_eq!((packet.kind_108, packet.kind_112), (6, 3));
}
#[test]
fn jump_processed_gate_suppresses_stationary_stick_impulse() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.flags_2472 = 0x1000_0000;
    p.raw_x_2692 = 1.;
    let packet = run(p);
    assert_eq!(packet.velocity_0[0], 0.);
    assert_eq!(packet.secondary_velocity_16, ZERO);
}
#[test]
fn jump_backward_velocity_is_removed_before_control() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.flags_2472 = 0x1000_0000;
    p.velocity_912 = [0., -9., -3., 0.];
    let packet = run(p);
    near(packet.velocity_0[2], 0.);
    assert_eq!(packet.secondary_velocity_16, ZERO);
}
#[test]
fn obstacle_positive_sixty_degree_branch_keeps_raw_45_argument() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.velocity_912 = [0., 0., 2., 0.];
    p.raw_x_2692 = -0.8660254;
    p.raw_z_2688 = 0.5;
    let mut state = controller::State::new([None; 3]);
    state.contact.active = true;
    let mut packet = Packet::initialized(0.);
    produce(&mut packet, &state, &curve(), settings(), &p, true).unwrap();
    near(packet.secondary_velocity_16[0], -0.8660254);
    near(packet.secondary_velocity_16[2], 0.5);
    // Secondary changes do not replace the base vector used for primary impulse.
    near(packet.velocity_0[0], 0.);
    near(packet.velocity_0[2], 1.6);
}
#[test]
fn obstacle_signed_negative_angle_and_over_ninety_branch() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.velocity_912 = [0., 0., 2., 0.];
    let mut state = controller::State::new([None; 3]);
    state.contact.active = true;
    let mut packet = Packet::initialized(0.);
    p.raw_x_2692 = 1.;
    produce(&mut packet, &state, &curve(), settings(), &p, true).unwrap();
    vector(packet.secondary_velocity_16, [1., 0., 0., 0.]);
    p.raw_x_2692 = -1.;
    p.raw_z_2688 = -0.1;
    produce(&mut packet, &state, &curve(), settings(), &p, true).unwrap();
    vector(packet.secondary_velocity_16, [0., 0., 1., 0.]);
}
#[test]
fn jump_slope_frame_reduces_vertical_impulse_by_up_y() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.flags_2472 = 0x1000_0000;
    let mut state = controller::State::new([None; 3]);
    state.frame_output.frame[1] = [0.6, 0.8, 0., 0.];
    let mut packet = Packet::initialized(0.);
    produce(&mut packet, &state, &curve(), settings(), &p, true).unwrap();
    let lift = (0.9f32 * 19.6).sqrt() * 0.8;
    near(packet.velocity_0[0], lift * 0.6);
    near(packet.velocity_0[1], lift * 0.8);
    assert_eq!(packet.up_48, UP); //Common output retains processed544.
}
#[test]
fn steering_uses_live760_then_angular_velocity_window_and_final_angle_cap() {
    let mut p = processed();
    p.flags_2476 = 0x80000;
    p.flags_2472 = 0x1000_0000;
    p.velocity_912 = [0., 0., 2., 0.];
    let mut state = controller::State::new([None; 3]);
    state.intent.steering = 1.;
    state.intent.original_steering = -1.;
    let mut packet = Packet::initialized(0.);
    produce(&mut packet, &state, &curve(), settings(), &p, true).unwrap();
    let (sin, cos) = crate::trigonometry::sin_cos(f32::from_bits(0x3e86_0a92));
    near(packet.velocity_0[0], 1.6 * sin);
    near(packet.velocity_0[2], 1.6 * cos);
    //82D7C6C4: the previous fixture incorrectly populated Biped+672.
    state.motion.angular_velocity_688 = -3.;
    produce(&mut packet, &state, &curve(), settings(), &p, true).unwrap();
    assert!(packet.velocity_0[0] < 0.);
}
#[test]
fn missing_departure_geometry_fails_before_any_packet_write() {
    let mut p = processed();
    p.previous_category_2516 = 400;
    let mut packet = Packet::initialized(123.);
    packet.velocity_0 = [9.; 4];
    packet.position_32 = [8.; 4];
    let before = packet;
    let result = produce(
        &mut packet,
        &controller::State::new([None; 3]),
        &curve(),
        settings(),
        &p,
        false,
    );
    assert_eq!(
        result,
        Err("Offboard launch mode1 requires source-backed departure geometry1120/1136")
    );
    assert_eq!(packet, before);
}
#[test]
fn absent_departure_geometry_is_valid_outside_selected_mode1() {
    let mut p = processed();
    p.previous_category_2516 = 400;
    p.current_category_2512 = 200;
    p.velocity_608 = [1., 2., 3., 4.];
    let mut packet = Packet::initialized(0.);
    produce(
        &mut packet,
        &controller::State::new([None; 3]),
        &curve(),
        settings(),
        &p,
        true,
    )
    .unwrap();
    assert_eq!(packet.velocity_0, p.velocity_608);
}

#[test]
fn geometric_launch_rotation_does_not_turn_x_into_a_fourth_velocity() {
    let v = [-2.0225198, 4.1746917, 0.7100913, 0.];
    let rotated = super::math::rotate(v, [0., 1., 0., 0.], 0.);
    assert_eq!(rotated, v);
    for angle in [-0.2, 0.2] {
        let rotated = super::math::rotate(v, [0., 1., 0., 0.], angle);
        assert_eq!(rotated[3], 0.);
        let (s, c) = crate::trigonometry::sin_cos(angle);
        assert!((rotated[0] - (v[0] * c + v[2] * s)).abs() < 1e-6);
        assert_eq!(rotated[1], v[1]);
        assert!((rotated[2] - (-v[0] * s + v[2] * c)).abs() < 1e-6);
    }
}
