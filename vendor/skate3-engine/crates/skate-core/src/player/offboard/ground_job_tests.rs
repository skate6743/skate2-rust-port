use super::*;
use crate::player::offboard::{contact_toolkit::IDENTITY, ground_query};

fn input() -> Input {
    Input {
        contact: ContactSnapshot {
            readiness: 30,
            prefix: ContactPrefix::reset(),
        },
        frame: IDENTITY,
        previous_state: 500,
        third_line_position: None,
        frames_since_teleport: 20,
        processed_position: [2.; 4],
        processed_velocity: [60.; 4],
        controls: GroundInputOutput {
            state_708: 0.7,
            state_712: -0.2,
            state_656: [1., 0., 2., 0.],
        },
        collision_displacements: [[3.; 4], [4.; 4]],
        animation_motion: [5.; 4],
        animation_velocity: [6.; 4],
        requested_duration: 0.8,
        requested_phase: 0.3,
        override_duration: 0.4,
        flags_2472: 0x1000_0000,
        flags_2476: 4,
        flags_2480: 0x80,
        flags_2484: 0x20000,
        flags_2488: 0x0800_0000,
    }
}
fn run(contact: &mut ContactPrefix, timer: &mut f32, input: Input) -> Prepared {
    prepare(contact, timer, input, |i| {
        Ok::<_, ()>(ground_query::consume_geometry(i, None))
    })
    .unwrap()
}

#[test]
fn ready_copies_whole_prefix_not_processed_flags_or_wheel_support() {
    let mut i = input();
    i.contact.prefix = ContactPrefix {
        position: [1., 2., 3., 0.],
        normal: [0.6, 0.8, 0., 0.],
        support_frame: [[7.; 4]; 4],
        target_position: [8.; 4],
        target_normal: [9.; 4],
        edge_position: [10.; 4],
        edge_normal: [11.; 4],
        scalar_160: 12.,
        kind_164: 13,
        distance_168: 14.,
        distance_172: 15.,
        flags_176: 0x31,
        support_180: 16,
    };
    let mut retained = ContactPrefix::reset();
    let mut timer = 0.;
    let out = run(&mut retained, &mut timer, i);
    assert_eq!(retained, i.contact.prefix);
    assert_eq!(out.job.flags, 0x31);
    assert_eq!(out.job.support_id, 16);
    assert_eq!(out.job.contact_normal, retained.normal);
    assert_eq!(out.job.support_frame, retained.support_frame);
    assert_eq!(out.job.target_position, retained.target_position);
    assert_eq!(out.job.target_normal, retained.target_normal);
    assert_eq!(out.job.edge_position, retained.edge_position);
    assert_eq!(out.job.edge_normal, retained.edge_normal);
    assert_eq!(timer, STEP);
}

#[test]
fn unready_native_third_line_changes_only_three_fields_and_keeps_timer() {
    let mut retained = ContactPrefix::reset();
    retained.support_180 = 42;
    retained.distance_172 = 17.;
    retained.target_position = [9.; 4];
    let old = retained;
    let mut i = input();
    i.contact.readiness = 0;
    i.third_line_position = Some([0., -2., 0., 0.]);
    let mut timer = 0.;
    run(&mut retained, &mut timer, i);
    assert_eq!(
        retained,
        ContactPrefix {
            flags_176: 1,
            position: [0., -2., 0., 0.],
            normal: i.frame[1],
            ..old
        }
    );
    assert_eq!(timer, STEP); //r27 remains1 on the unready branch.
}

#[test]
fn no_third_line_or_previous_501_does_not_invent_support() {
    for previous in [500, 501] {
        let mut i = input();
        i.contact.readiness = -1;
        i.previous_state = previous;
        i.third_line_position = if previous == 501 { Some([7.; 4]) } else { None };
        let mut retained = ContactPrefix::reset();
        let before = retained;
        let mut timer = 0.;
        run(&mut retained, &mut timer, i);
        assert_eq!(retained, before);
        assert_eq!(timer, STEP);
    }
}

#[test]
fn ready_reset_packet_is_not_treated_as_unready_or_stale_contact() {
    let mut i = input();
    i.third_line_position = Some([8.; 4]);
    let mut retained = ContactPrefix::reset();
    retained.flags_176 = 0x39;
    retained.position = [9.; 4];
    let mut timer = 2.;
    run(&mut retained, &mut timer, i);
    assert_eq!(retained, ContactPrefix::reset());
    assert_eq!(timer, 0.);
}

#[test]
fn teleport_signed_guard_and_fixed_step_prediction_match_preupdate() {
    for (frames, suppressed) in [(19, true), (20, false), (u32::MAX, true)] {
        let mut i = input();
        i.frames_since_teleport = frames;
        i.contact.prefix.flags_176 = 0x38;
        let mut retained = ContactPrefix::reset();
        let out = run(&mut retained, &mut 0., i);
        assert_eq!(out.job.suppress_minimum, suppressed);
        assert_eq!(retained.flags_176 & 8 == 0, suppressed);
        let expected = 60_f32.mul_add(f32::from_bits(0x3c88_8889), 2.);
        assert_eq!(out.job.animation_position, [expected; 4]);
        assert_eq!(out.job.collision_displacements, i.collision_displacements);
        assert_eq!(out.job.requested_phase, 0.3);
        assert_eq!(out.job.override_duration, 0.4);
        assert_eq!(out.job.desired_direction, i.controls.state_656);
        assert!(out.job.mirrored && out.job.sprint_pressed && out.job.suppress_lean);
        assert!(out.job.animation_directed && out.job.ignore_obstacle);
    }
}

#[test]
fn geometry_changes_job_normal_without_overwriting_retained_prefix() {
    let mut i = input();
    i.contact.prefix.normal = [0.6, 0.8, 0., 0.];
    i.contact.prefix.flags_176 = 0x30;
    let mut retained = ContactPrefix::reset();
    let mut count = 0;
    let out = prepare(&mut retained, &mut 0., i, |input| {
        count += 1;
        assert_eq!(input.reach_364, 1.0e10);
        Ok::<_, ()>(ground_query::consume_geometry(
            input,
            Some(ground_query::GroundGeometry {
                frame: ground_query::Frame::IDENTITY,
                kind: 2,
                flag26: true,
                flag27: false,
                flag28: false,
            }),
        ))
    })
    .unwrap();
    assert_eq!(count, 1);
    assert_eq!(retained.normal, [0.6, 0.8, 0., 0.]);
    assert_eq!(out.job.contact_normal, [0., 1., 0., 0.]);
    assert!(out.geometry.state_754);
    assert!(out.job.target_frame_present);
    assert!(!out.job.edge_active);
}

#[test]
fn geometry_failure_is_not_converted_to_a_ground_job() {
    assert!(
        prepare(&mut ContactPrefix::reset(), &mut 0., input(), |_| {
            Err::<GroundAdjustment, _>("real query failed")
        })
        .is_err()
    );
}

fn motion() -> super::super::controller::GroundResult {
    super::super::controller::GroundResult {
        physical_frame: IDENTITY,
        animation_frame: IDENTITY,
        surface_frame: IDENTITY,
        velocity: [0.; 4],
        position: [0.; 4],
        angular_velocity: 0.,
        alternate: false,
        sliding: false,
    }
}

#[test]
fn sync_corrects_only_positive_contact_height_and_preserves_result() {
    for (height, expected) in [(0.2, 0.2), (-0.2, 0.)] {
        let mut state = super::super::ground_entry::State::default();
        let mut contact = ContactPrefix::reset();
        contact.flags_176 = 1;
        contact.position[1] = height;
        let result = motion();
        assert_eq!(sync_frames(&mut state, contact, result), IDENTITY);
        assert_eq!(state.frame_80[3][1], expected);
        assert_eq!(result.physical_frame, IDENTITY);
    }
}

#[test]
fn sync_finishes_enter_angle_unwind_before_publishing_animation_axes() {
    let mut state = super::super::ground_entry::State::default();
    state.flags_144_to_150[6] = true;
    state.duration_180 = STEP;
    state.angle_172 = 1.;
    state.angular_velocity_176 = 3.;
    let frame = sync_frames(&mut state, ContactPrefix::reset(), motion());
    assert!(!state.flags_144_to_150[6]);
    assert_eq!(state.duration_180, 0.);
    let (sin, cos) = crate::trigonometry::sin_cos(0.9);
    assert!((frame[2][0] - sin).abs() < 1.0e-6);
    assert!((frame[2][2] - cos).abs() < 1.0e-6);
}
