use super::*;
use crate::physics::skeleton_animation_record::IDENTITY;
use crate::point_graph::PointGraph;
#[test]
fn handplant_air_check_preserves_contact_threshold_and_real_impact_bails() {
    let mut f = input();
    f.category = 600;
    f.closing_velocity = [0.2, 0.0, 0.0, 0.0];
    let mut s = settings();
    s.air.xz_trick = 0.1;
    s.air.y_trick = 0.6;
    let mut foot = Requests::new();
    check_plant(&mut foot, &s, &f);
    assert!(foot.reasons[2]);
    //HandPlant82D4C530 calls Air(false); a coping contact is not by itself
    //the sideways-board danger branch that enables the tiny trick limits.
    let mut hand = Requests::new();
    check_air(&mut hand, &s, &mode(), &f, false);
    assert_eq!(hand.count, 0);
    f.closing_velocity = [11.0, 0.0, 0.0, 0.0];
    check_air(&mut hand, &s, &mode(), &f, false);
    assert!(hand.reasons[2]);
}
fn input() -> Frame {
    Frame {
        flags_2468: 0,
        flags_2472: 0,
        flags_2476: 0,
        flags_2480: 0,
        flags_2484: 0,
        category: 100,
        timestep: 1.0 / 60.0,
        time_on_ground: 1.0,
        speed: 0.0,
        animation_up: [0.0, 1.0, 0.0, 0.0],
        landing_angle: 0.0,
        deck_velocity: [0.0; 4],
        com_velocity: [0.0; 4],
        jump_fix_frames: 1000,
        deck: IDENTITY,
        input_board: IDENTITY,
        world_to_animation: IDENTITY,
        closing_velocity: [0.0; 4],
        board_material_flags: 0,
        board_contact: true,
        wheel_contact: true,
        board_contact_normal: [0.0, 1.0, 0.0, 0.0],
        opposing_contact: 0.0,
        regions_force: [0.0; 8],
        maximum_skater_force: 0.0,
        vehicle_force: 0.0,
        group_8: false,
        conflicting: false,
        compliant: false,
        highest_normal: [0.0, 1.0, 0.0, 0.0],
        pose_error: [0.0; 4],
        maximum_pose_error: 0.0,
        flip_active: false,
        flip_requested_speed: 0.0,
        system_up_y: 1.0,
        grind_selected: false,
        grind_normal_valid: false,
        grind_normal: [0.0, 1.0, 0.0, 0.0],
    }
}
fn mode() -> Mode {
    Mode {
        check_squash: true,
        check_bad_landing: true,
        ground_xz: 10.0,
        bad_landing_scale: 1.0,
    }
}
fn settings() -> Settings {
    Settings {
        ground: GroundSettings {
            vehicle_scalar: 1.0,
            vehicle_contact: 100.0,
            skitch_contact: 100.0,
            skitch_scalar: 1.0,
            skitch_arms_scalar: 1.0,
            skater_scalar: 1.0,
            max_squash: 100.0,
            max_squash_coffin: 100.0,
            max_displacement: 100.0,
            max_contact: 100.0,
            max_arm_contact: 100.0,
            max_deck_error: 1.0,
            opposing_contact: 1.0,
            y_acceleration: 10.0,
            light_dmo_scalar: 1.0,
            player_scalar: 1.0,
            ai_scalar: 1.0,
            skitch_acc_scalar: 1.0,
            balance_total: 100.0,
            balance_min_speed: 1.0,
            balance_base: 1.0,
        },
        air: AirSettings {
            xz_trick: 10.0,
            y_trick: 10.0,
            xz_acceleration: 10.0,
            y_acceleration: 10.0,
            max_squash: 100.0,
            max_displacement: 100.0,
            max_contact: 100.0,
            max_arm_contact: 100.0,
            body_flip_scalar: 1.0,
            body_flip_acc_scalar: 1.0,
            light_dmo_scalar: 1.0,
            ignore_danger_frames: 10,
            max_landing_speed: 6.0,
            max_stairs_speed: 8.0,
            max_grind_speed: 5.0,
            max_landing_angle: PointGraph {
                x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
                y: [1.0; 8],
            },
        },
        lean_contact_y: -1.0,
    }
}
fn request_input() -> RequestInput {
    RequestInput {
        flags_2468: 0,
        flags_2476: 0,
        flags_2480: 0,
        flags_2484: 0,
        animation_up_y: 1.0,
        category: 100,
    }
}
#[test]
fn wipeout_ground_priority_and_repeated_force_change_runout_selection() {
    let mut state = Requests::new();
    let mut f = input();
    let s = settings();
    f.closing_velocity = [11.0, 0.0, 0.0, 0.0];
    check_ground(&mut state, &s, &mode(), &f);
    assert_eq!(state.count, 1);
    assert!(state.reasons[2]);
    assert!(state.requests_runout(&request_input()));
    assert!(!state.requests_wipeout(&request_input()));
    state.clear_after_selection();
    f.maximum_pose_error = 101.0;
    f.pose_error = [101.0, 0.0, 0.0, 0.0];
    f.regions_force[0] = 101.0;
    check_ground(&mut state, &s, &mode(), &f);
    assert_eq!(state.count, 3);
    assert!(state.reasons[18] && state.reasons[2] && state.reasons[0]);
    assert!(!state.reasons[1]); //The initial squash/displacement/force chain is exclusive.
    assert!(!state.requests_runout(&request_input()));
    assert!(state.requests_wipeout(&request_input()));
    state.clear_after_selection();
    f.maximum_pose_error = 0.0;
    f.pose_error = [0.0; 4];
    check_ground(&mut state, &s, &mode(), &f);
    //Force is requested once by the main check and again after excessive closing speed.
    assert_eq!(state.count, 3);
    assert_eq!(state.reasons.iter().filter(|v| **v).count(), 2);
}
#[test]
fn wipeout_air_landing_uses_selected_velocity_and_preserves_impact_value() {
    let mut state = Requests::new();
    let mut f = input();
    let s = settings();
    f.deck_velocity = [0.0, -2.0, 0.0, 0.0];
    f.com_velocity = [0.0, -7.0, 0.0, 0.0];
    check_air(&mut state, &s, &mode(), &f, false);
    assert_eq!(state.count, 0);
    check_air(&mut state, &s, &mode(), &f, true);
    assert_eq!(state.count, 1);
    assert!(state.reasons[6]);
    assert_eq!(state.values[6], 7.0);
    state.clear_after_selection();
    f.grind_selected = true;
    f.grind_normal_valid = true;
    f.grind_normal = [1.0, 0.0, 0.0, 0.0];
    check_air(&mut state, &s, &mode(), &f, true);
    assert_eq!(state.count, 0); //The selector's actual normal replaces the board normal.
    f.grind_normal_valid = false;
    check_air(&mut state, &s, &mode(), &f, true);
    assert_eq!(state.count, 1);
    assert!(state.reasons[8]);
    assert!(!state.reasons[6]);
}
#[test]
fn wipeout_sensitive_conditions_count_both_triggers_and_clear_only_requests() {
    let mut state = Requests::new();
    let mut f = input();
    f.flags_2476 = 1 << 26;
    f.flags_2472 = 1 << 6;
    f.compliant = true;
    check_air(&mut state, &settings(), &mode(), &f, false);
    assert_eq!(state.count, 2);
    assert!(state.reasons[21]);
    assert_eq!(state.reasons.iter().filter(|v| **v).count(), 1);
    state.balance = 3.0;
    state.cooldown = 0.2;
    state.clear_after_selection();
    assert_eq!(state.count, 0);
    assert_eq!(state.balance, 3.0);
    assert_eq!(state.cooldown, 0.2);
    assert_eq!(state.mode, 2);
    assert_eq!(state.contact_frames, 3);
    state.reset_systems();
    assert_eq!(state.mode, 0);
    assert_eq!(state.balance, 3.0);
    assert_eq!(state.cooldown, 0.2);
}
#[test]
fn wipeout_cooldown_has_native_initial_and_teleport_lifetimes() {
    let mut state = Requests::new();
    state.initialize_player();
    assert_eq!(state.cooldown.to_bits(), 0x3d23d70a);
    let mut f = input();
    f.maximum_pose_error = 101.0;
    check_air(&mut state, &settings(), &mode(), &f, false);
    assert_eq!(state.count, 0);
    assert_eq!(state.contact_frames, 3);
    assert!(state.cooldown > 0.0);
    state.teleport();
    assert_eq!(state.cooldown.to_bits(), 0x3ecccccd);
    state.reset_systems();
    assert_eq!(state.cooldown.to_bits(), 0x3ecccccd);
    state.cooldown = 0.001;
    check_ground(&mut state, &settings(), &mode(), &f);
    assert!(state.cooldown < 0.0);
    assert_eq!(state.count, 0);
    check_ground(&mut state, &settings(), &mode(), &f);
    assert_eq!(state.count, 1);
    assert!(state.reasons[18]);
}

#[test]
fn wipeout_ground_animation_keeps_additional_checks_during_cooldown() {
    let mut state = Requests::new();
    state.teleport();
    let mut f = input();
    f.regions_force[0] = 101.0;
    f.opposing_contact = 1.5;
    check_ground_animation(&mut state, &settings(), &mode(), &f, 1.0);
    assert_eq!(state.count, 2);
    assert!(state.reasons[0] && state.reasons[20]);
    state.clear_after_selection();
    check_ground_animation(&mut state, &settings(), &mode(), &f, 2.0);
    assert_eq!(state.count, 1);
    assert!(state.reasons[0]);
    assert!(!state.reasons[20]);
}

#[test]
fn plant_observation_boundary_keeps_contact_rotation_and_duplicate_force_triggers() {
    let mut f = input();
    let mut s = settings();
    s.air.max_contact = 2.;
    s.air.xz_trick = 1.;
    s.air.y_trick = 1.;
    f.regions_force[0] = 3.;
    f.closing_velocity = [4., 0., 0., 0.];
    // World X becomes local Y. Both source impact branches use this actual
    // matrix; two force triggers must remain two calls to the request ledger.
    f.world_to_animation = [
        [0., 1., 0., 0.],
        [-1., 0., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let mut full = Requests::new();
    check_plant(&mut full, &s, &f);
    let mut input = PlantInput {
        maximum_pose_error: f.maximum_pose_error,
        pose_error: f.pose_error,
        regions_force: f.regions_force,
        contact: Some(PlantContact {
            closing_velocity: f.closing_velocity,
            world_to_animation: f.world_to_animation,
        }),
    };
    let mut events = Vec::new();
    check_plant_input(&s, &input, |reason, value| events.push((reason, value)));
    assert_eq!(events, [(0, 0.), (2, 0.), (0, 0.)]);
    assert_eq!(full.count, 3);
    assert_eq!(full.mode, 3);
    input.contact = None;
    events.clear();
    check_plant_input(&s, &input, |reason, value| events.push((reason, value)));
    assert_eq!(events, [(0, 0.)]);
}
