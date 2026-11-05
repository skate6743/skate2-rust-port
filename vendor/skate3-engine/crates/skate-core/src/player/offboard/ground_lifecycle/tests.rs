use super::*;
use crate::physics::skeleton_animation_record::IDENTITY;
use crate::player::{
    offboard::ground_entry::State,
    wipeout::{Frame, Requests},
};
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
fn settings() -> CollisionSettings {
    CollisionSettings {
        vehicle_scalar: 0.35,
        vehicle_contact: 6.0,
        maximum_displacement: 0.3,
        maximum_arm_contact: 20.0,
        maximum_body_contact: 20.0,
        minimum_speed: 0.0,
        maximum_squash: 0.6,
        special_scalar: 1.0,
    }
}
fn collision(f: &Frame) -> CollisionInput<'_> {
    CollisionInput {
        shared: f,
        skeleton_velocity_16336: [1.0, 0.0, 0.0, 0.0],
        contact_flag_4072: false,
        contact_force_4056: 0.0,
    }
}
fn post(f: &Frame) -> PostInput<'_> {
    PostInput {
        collision: collision(f),
        processed_flags_2484: 0,
        processed_velocity_608: [0.0; 4],
        skeleton_displacement_16288: [0.0; 4],
        skeleton_displacement_16304: [0.0; 4],
        ground_kind_356: 0,
    }
}
#[test]
fn duplicate_ground_triggers_increment_count_and_keep_shared_requests() {
    let f = input();
    let mut p = post(&f);
    p.processed_flags_2484 = 0x400;
    let mut s = State::default();
    s.flags_144_to_150[6] = true;
    s.flags_144_to_150[2] = true;
    s.flags_144_to_150[1] = true;
    let mut q = Requests::new();
    q.request(3, 2.0);
    post_physics(&mut s, &mut q, &settings(), &p);
    assert_eq!(q.count, 4);
    assert!(q.reasons[31] && q.reasons[28] && q.reasons[3]);
    assert_eq!(q.values[3], 2.0);
    assert_eq!(q.mode, 4);
}
#[test]
fn displacement_requires_persistent_ticks_then_clears_when_contact_settles() {
    let f = input();
    let mut p = post(&f);
    p.skeleton_displacement_16304 = [0.0, 0.5, 0.0, 0.0];
    let mut s = State::default();
    let mut q = Requests::new();
    for _ in 0..2 {
        post_physics(&mut s, &mut q, &settings(), &p);
    }
    assert_eq!(q.count, 0);
    post_physics(&mut s, &mut q, &settings(), &p);
    assert!(q.reasons[32]);
    p.skeleton_displacement_16304 = [0.0; 4];
    post_physics(&mut s, &mut q, &settings(), &p);
    assert_eq!(s.elapsed_168, 0.0);
}
#[test]
fn opposite_displacement_triggers_only_beyond_speed_scaled_threshold() {
    let f = input();
    let mut p = post(&f);
    p.processed_flags_2484 = 0x400;
    p.processed_velocity_608 = [2.0, 0.0, 0.0, 0.0];
    let mut s = State::default();
    let mut q = Requests::new();
    p.skeleton_displacement_16304 = [-0.02, 0.0, 0.0, 0.0];
    post_physics(&mut s, &mut q, &settings(), &p);
    assert_eq!(q.count, 0);
    p.skeleton_displacement_16304[0] = -0.03;
    post_physics(&mut s, &mut q, &settings(), &p);
    assert!(q.reasons[32]);
}
#[test]
fn vehicle_scalar_applies_even_below_vehicle_request_force() {
    let mut f = input();
    f.maximum_pose_error = 0.3;
    let mut i = collision(&f);
    i.contact_flag_4072 = true;
    i.contact_force_4056 = 0.0;
    let mut q = Requests::new();
    check_collision(&mut q, &settings(), &i);
    assert!(q.reasons[18]);
    assert!(!q.reasons[7]);
}
#[test]
fn collision_priority_selects_squash_before_displacement_and_force() {
    let mut f = input();
    f.maximum_pose_error = 1.0;
    f.pose_error = [1.0, 0.0, 0.0, 0.0];
    f.regions_force = [100.0; 8];
    let mut q = Requests::new();
    check_collision(&mut q, &settings(), &collision(&f));
    assert!(q.reasons[18]);
    assert!(!q.reasons[1] && !q.reasons[0]);
}
fn publication() -> PublicationInput {
    PublicationInput {
        ground_flags_752_to_754: [false; 3],
        contact_flags_368: 0,
        contact_position_192: [3.0, 0.0, 4.0, 0.0],
        query_position_816: [0.0; 4],
        processed_flags_2476: 0,
        ground_kind_356: 4,
        ground_scalar_360: 0.7,
        motion_vector_1040: [2.0, 3.0, 4.0, 5.0],
        motion_up_864: [0.0, 1.0, 0.0, 2.0],
        hand_flags: [[true, false, false], [true, true, false]],
    }
}
#[test]
fn output_gates_distance_and_preserves_projection_w_lane() {
    let mut s = State::default();
    s.distance_164 = 0.0;
    let mut i = publication();
    let o = publish(&s, &i);
    assert_eq!(o.offboard_distance_116, 1e10);
    assert!(o.offboard_flag_330);
    assert_eq!(o.animation_vector_144, [2.0, 0.0, 4.0, -1.0]);
    assert_eq!(o.offboard_hand_flags_306_307, [true, false]);
    i.ground_flags_752_to_754[0] = true;
    i.contact_flags_368 = 9;
    let o = publish(&s, &i);
    assert_eq!(o.offboard_distance_116, 5.0);
    assert!(o.offboard_flag_329);
    assert!(!o.offboard_flag_330);
}
#[test]
fn exit_calls_real_refresh_before_clearing_and_releasing() {
    struct Service {
        calls: Vec<u8>,
        active: bool,
    }
    impl ExitServices for Service {
        fn refresh_toolkit_contacts(&mut self) {
            self.calls.push(1)
        }
        fn clear_toolkit_exit_fields(&mut self) {
            self.calls.push(2)
        }
        fn ground_geometry_active(&self) -> bool {
            self.active
        }
        fn release_ground_geometry(&mut self) {
            self.calls.push(3)
        }
        fn clear_ground_geometry_active(&mut self) {
            self.calls.push(4);
            self.active = false
        }
    }
    let mut service = Service {
        calls: vec![],
        active: true,
    };
    exit(&mut service);
    assert_eq!(service.calls, [1, 2, 3, 4]);
    service.calls.clear();
    exit(&mut service);
    assert_eq!(service.calls, [1, 2]);
}
