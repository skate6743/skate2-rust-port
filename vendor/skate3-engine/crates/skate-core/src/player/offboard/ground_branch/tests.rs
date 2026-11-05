use super::*;
fn input() -> BranchInput {
    BranchInput {
        movement_velocity_480: [0.0, 5.0, 0.0, 0.0],
        output_velocity_512: [0.0, 6.0, 0.0, 0.0],
        right_0: [1.0, 0.0, 0.0, 0.0],
        position_48: [0.0; 4],
        contact_point_96: [0.0, 0.0, 1.0, 0.0],
        contact_flags_176: 2,
        suppressed_317: false,
    }
}
#[test]
fn both_velocities_must_clear_projected_threshold() {
    assert!(select_alternate(input()));
    let mut i = input();
    i.output_velocity_512[1] = 4.0;
    assert!(!select_alternate(i));
    let mut i = input();
    i.movement_velocity_480[1] = 4.0;
    assert!(!select_alternate(i));
}
#[test]
fn contact_distance_suppression_and_contact_bit_gate_branch() {
    let mut i = input();
    i.contact_point_96 = [0.0, 0.0, 0.04, 0.0];
    assert!(!select_alternate(i));
    let mut i = input();
    i.suppressed_317 = true;
    assert!(!select_alternate(i));
    let mut i = input();
    i.contact_flags_176 = 1;
    assert!(!select_alternate(i));
    let mut i = input();
    i.contact_point_96 = [1.0, 0.0, 0.0, 0.0];
    assert!(!select_alternate(i));
}
#[test]
fn reversed_contact_direction_reverses_velocity_requirement() {
    let mut i = input();
    i.contact_point_96[2] = -1.0;
    assert!(!select_alternate(i));
    let mut i = input();
    i.contact_point_96[2] = -1.0;
    i.movement_velocity_480[1] = -5.0;
    i.output_velocity_512[1] = -6.0;
    assert!(select_alternate(i));
}
#[test]
fn alternate_integrates_gravity_before_position_and_frame_output() {
    let mut velocity = [1.0, 0.0, 0.0, 0.0];
    let mut speed = 0.0;
    let mut position = [0.0; 4];
    let right = [1.0, 0.0, 0.0, 0.0];
    let up = [0.0, 1.0, 0.0, 0.0];
    let forward = [0.0, 0.0, 1.0, 0.0];
    let mut output = FrameOutput {
        frame: [right, up, forward, position],
        velocity: [0.0; 4],
    };
    integrate_alternate(
        &mut velocity,
        &mut speed,
        &mut position,
        &mut output,
        forward,
        up,
        up,
    );
    assert_eq!(velocity[1], f32::from_bits(0xc11c_cccd) * DT);
    assert_eq!(position[1], velocity[1] * DT);
    assert!(speed > 1.0);
    for i in 0..3 {
        assert!((output.frame[3][i] - position[i]).abs() < 1e-6);
    }
}
