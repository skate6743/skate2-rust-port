use super::{checks::*, *};
use crate::{
    physics::skeleton_animation_record::IDENTITY, player::offboard::biped_air::recovered::State,
};
fn frame() -> Frame {
    Frame {
        flags_2468: 0,
        flags_2472: 0,
        flags_2476: 0,
        flags_2480: 0,
        flags_2484: 0,
        category: 500,
        timestep: DT,
        time_on_ground: 0.,
        speed: 0.,
        animation_up: [0., 1., 0., 0.],
        landing_angle: 0.,
        deck_velocity: [0.; 4],
        com_velocity: [0.; 4],
        jump_fix_frames: 0,
        deck: IDENTITY,
        input_board: IDENTITY,
        world_to_animation: IDENTITY,
        closing_velocity: [0.; 4],
        board_material_flags: 0,
        board_contact: false,
        wheel_contact: false,
        board_contact_normal: [0.; 4],
        opposing_contact: 0.,
        regions_force: [0.; 8],
        maximum_skater_force: 0.,
        vehicle_force: 0.,
        group_8: false,
        conflicting: false,
        compliant: false,
        highest_normal: [0.; 4],
        pose_error: [0.; 4],
        maximum_pose_error: 0.,
        flip_active: false,
        flip_requested_speed: 0.,
        system_up_y: 1.,
        grind_selected: false,
        grind_normal_valid: false,
        grind_normal: [0.; 4],
    }
}
fn mode() -> Mode {
    Mode {
        check_squash: true,
        check_bad_landing: false,
        ground_xz: 0.,
        bad_landing_scale: 0.,
    }
}
fn settings() -> Settings {
    Settings {
        skeleton_air: Thresholds {
            squash: 2.,
            displacement: 3.,
            body_contact: 4.,
            arm_contact: 5.,
        },
        offboard_air: Thresholds {
            squash: 6.,
            displacement: 7.,
            body_contact: 8.,
            arm_contact: 9.,
        },
        offboard_min_speed: 10.,
    }
}
#[test]
fn offboard_speed_is_strict_root_velocity_and_mode_always_changes() {
    let mut f = frame();
    f.maximum_pose_error = 100.;
    f.com_velocity = [100.; 4];
    let mut r = Requests::new();
    offboard_air(&mut r, &settings(), mode(), &f, [10., 0., 0., 900.]);
    assert_eq!(r.mode, 4);
    assert_eq!(r.count, 0); //Boundary equality, COM and W do not trigger.
    offboard_air(&mut r, &settings(), mode(), &f, [10.01, 0., 0., 0.]);
    assert_eq!(r.count, 1);
    assert!(r.reasons[18]);
}
#[test]
fn squash_displacement_and_force_are_exclusive() {
    let mut f = frame();
    f.maximum_pose_error = 10.;
    f.pose_error = [10., 0., 0., 0.];
    f.regions_force = [20.; 8];
    let mut r = Requests::new();
    skeleton_air(&mut r, &settings(), mode(), &f);
    assert_eq!(r.count, 1);
    assert!(r.reasons[18] && !r.reasons[1] && !r.reasons[0]);
    let mut m = mode();
    m.check_squash = false;
    skeleton_air(&mut r, &settings(), m, &f);
    assert_eq!(r.count, 2);
    assert!(r.reasons[1] && !r.reasons[0]);
    f.pose_error = [3., 0., 0., 100.]; //Equal displacement goes to regional forces.
    skeleton_air(&mut r, &settings(), m, &f);
    assert_eq!(r.count, 3);
    assert!(r.reasons[0]);
    assert_eq!(r.mode, 0); //8FEE8 never writes mode.
}
#[test]
fn offboard_uses_its_own_body_limit_and_stock_ob_arm_limit() {
    let mut f = frame();
    let mut r = Requests::new();
    f.regions_force[2] = 9.;
    offboard_air(&mut r, &settings(), mode(), &f, [11., 0., 0., 0.]);
    assert_eq!(r.count, 0);
    f.regions_force[2] = 9.01;
    offboard_air(&mut r, &settings(), mode(), &f, [11., 0., 0., 0.]);
    assert_eq!(r.count, 1);
    assert!(r.reasons[0]);
}
#[test]
fn unordered_displacement_retains_native_else_branch() {
    let mut f = frame();
    f.pose_error[0] = f32::NAN;
    let mut r = Requests::new();
    skeleton_air(&mut r, &settings(), mode(), &f);
    assert!(r.reasons[1]);
    assert_eq!(r.count, 1);
}
#[test]
fn air_post_does_not_deduplicate_repeated_request_31() {
    let mut s = State::default();
    s.result.valid_404 = true;
    s.result.velocity_288[1] = -1.;
    s.result.contact_velocity_320 = [0., 0., 21., 0.];
    s.flags_544_550[5] = true;
    s.time_remaining_444 = -1.;
    let mut r = Requests::new();
    s.post_physics(
        PostInput {
            flags_2484: 0,
            state_timer_2664: 0.,
            forward_224: [0., 0., 1., 0.],
            side_192: [1., 0., 0., 0.],
            input_2708: 0.,
            input_2704: 0.,
        },
        &settings(),
        &frame(),
        mode(),
        [0.; 4],
        &mut r,
    );
    assert_eq!(r.count, 3); //Downward flag549, planar overspeed, expired latch548.
    assert!(r.reasons[31]);
    assert_eq!(r.reasons.iter().filter(|&&v| v).count(), 1);
    assert_eq!(r.mode, 4);
}
