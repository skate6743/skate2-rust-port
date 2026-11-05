use super::*;
use crate::{physics::skeleton_animation_record::IDENTITY, point_graph::PointGraph};

#[test]
fn solved_volume_is_converted_back_to_bone_and_preserves_unmapped_animation() {
    let mut forward = [IDENTITY; 24];
    forward[1][3][0] = 2.0;
    let mut parents = [None; 24];
    parents[1] = Some(2);
    let output = SkeletonOutput {
        bone_indices: core::array::from_fn(|i| i),
        geometry: Geometry::new(parents, &forward).unwrap(),
        board_bones: board::BoneIndices {
            front_truck: 25,
            back_truck: 26,
            front_left_wheel: 27,
            front_right_wheel: 28,
            back_left_wheel: 29,
            back_right_wheel: 30,
        },
        board_settings: board::Settings {
            truck_tilt_scalar: 1.0,
            truck_tilt_max_angle: 1.0,
            truck_tilt_wobble_scalar: 1.0,
            truck_displacement_max: 1.0,
        },
    };
    let mut physical = [IDENTITY; 24];
    physical[1][3][0] = 12.0;
    physical[2][3][0] = 7.0;
    let mut world_to_anim = IDENTITY;
    world_to_anim[3][0] = -3.0;
    let mut globals = [IDENTITY; 36];
    let mut locals = [IDENTITY; 36];
    locals[35][3][1] = 9.0;
    let mut bodies = [IDENTITY; 6];
    bodies[0][3][0] = -1.0;
    bodies[1][3][0] = 1.0;
    bodies[2][3][0] = 1.0;
    bodies[3][3][0] = -1.0;
    output
        .publish(
            Input {
                physical_parts: &physical,
                world_to_animation: &world_to_anim,
                board: board::Input {
                    bodies: &bodies,
                    skeleton_board: &physical[0],
                    truck_frames: &[IDENTITY; 2],
                    deck_wobble_tilt: 0.0,
                    deck_wobble_squish: 0.0,
                    average_compression: [0.0; 2],
                },
            },
            &mut globals,
            &mut locals,
        )
        .unwrap();
    assert!((globals[1][3][0] - 7.0).abs() < 1e-6);
    assert!((locals[1][3][0] - 3.0).abs() < 1e-6);
    assert_eq!(locals[35][3][1], 9.0);
}

#[test]
fn wobble_emits_final_sample_before_expiry_and_selects_real_phase_curves() {
    let curve = |value| PointGraph {
        x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        y: [value; 8],
    };
    let settings = wobble::Settings {
        takeoff_tilt: curve(2.0),
        landing_tilt: curve(3.0),
        takeoff_squish: curve(4.0),
        landing_squish: curve(5.0),
        maximum_time: 0.0,
    };
    let mut state = wobble::Wobble::default();
    assert!(!state.update(&settings).sampled);
    state.trigger(true, true);
    let final_sample = state.update(&settings);
    assert!(final_sample.sampled);
    assert!(!final_sample.remains_active);
    assert_eq!(final_sample.tilt, -3.0);
    assert_eq!(final_sample.squish, 5.0);
    assert_eq!(state.time, 0.0);
    assert!(!state.update(&settings).sampled);
    state.trigger(false, false);
    assert_eq!(state.update(&settings).tilt, 2.0);
}

#[test]
fn wobble_rotates_about_deck_forward_then_compresses_along_new_up() {
    let mut board = IDENTITY;
    board[3] = [4.0, 5.0, 6.0, 0.0];
    wobble::apply(
        wobble::Output {
            sampled: true,
            tilt: core::f32::consts::FRAC_PI_2,
            squish: 0.25,
            remains_active: true,
        },
        &mut board,
    );
    assert!((board[0][0]).abs() < 2e-6);
    assert!((board[0][1] - 1.0).abs() < 2e-6);
    assert!((board[1][0] + 1.0).abs() < 2e-6);
    assert_eq!(&board[2][..3], &[0.0, 0.0, 1.0]);
    assert!((board[3][0] - 3.75).abs() < 2e-6);
    assert!((board[3][1] - 5.0).abs() < 2e-6);
    assert_eq!(board[3][2], 6.0);
}
