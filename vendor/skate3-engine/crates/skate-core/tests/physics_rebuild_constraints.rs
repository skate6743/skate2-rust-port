//! Independently specified mechanics and packed-boundary checks. These rows
//! are explicit test inputs, not generated-code outputs or parity fixtures.
use skate_core::physics::solver::packed::{self, Constraint, Drive, Joint, Reaction};

fn column(words: &mut [u32], byte_offset: usize, values: [f32; 4]) {
    for (slot, value) in words[byte_offset / 4..byte_offset / 4 + 4]
        .iter_mut()
        .zip(values)
    {
        *slot = value.to_bits();
    }
}

fn read(words: &[u32], byte_offset: usize) -> [f32; 4] {
    core::array::from_fn(|i| f32::from_bits(words[byte_offset / 4 + i]))
}

// Equal unit masses/inertias with origin anchors have inverse effective mass
// one half along each axis. The six impulse axes are the Cartesian basis.
fn origin_constraint() -> Constraint<96> {
    let mut words = [0; 96];
    for axis in 0..3 {
        let mut basis = [0.0; 4];
        basis[axis] = 1.0;
        column(&mut words, 192 + axis * 16, basis);
        column(&mut words, 240 + axis * 16, basis);
        let mut projection = basis;
        projection[axis] = 0.5;
        column(&mut words, 64 + axis * 32, projection);
        column(&mut words, 80 + axis * 32, projection);
        basis[3] = if axis == 0 { 1.0 } else { 0.0 };
        column(&mut words, 288 + axis * 16, basis);
        column(&mut words, 336 + axis * 16, basis);
    }
    Constraint {
        words,
        reaction_a: 0,
        reaction_b: 1,
    }
}

fn drive() -> Drive {
    let mut drive = origin_constraint();
    drive.words[11] = 1.0f32.to_bits();
    drive.words[15] = 1.0f32.to_bits();
    drive_limits(&mut drive, [1000.0; 3], [1000.0; 3]);
    drive
}

fn drive_limits(drive: &mut Drive, linear: [f32; 3], angular: [f32; 3]) {
    for (offset, value) in [
        (108, linear[0]),
        (124, linear[1]),
        (172, linear[2]),
        (140, angular[0]),
        (156, angular[1]),
        (188, angular[2]),
    ] {
        drive.words[offset / 4] = value.to_bits();
    }
}

fn reactions(linear_b: [f32; 3], angular_b: [f32; 3]) -> [Reaction; 2] {
    let mut states = [[0; 16]; 2];
    column(
        &mut states[1],
        0,
        [linear_b[0], linear_b[1], linear_b[2], 0.0],
    );
    column(
        &mut states[1],
        32,
        [angular_b[0], angular_b[1], angular_b[2], 0.0],
    );
    states
}

fn xyz(words: &[u32], offset: usize) -> [f32; 3] {
    read(words, offset)[..3].try_into().unwrap()
}

#[test]
fn joint_and_drive_equalize_origin_corrections_and_converge() {
    for is_drive in [false, true] {
        let mut joints: Vec<Joint> = if is_drive {
            vec![]
        } else {
            vec![origin_constraint()]
        };
        let mut drives = if is_drive { vec![drive()] } else { vec![] };
        let mut states = reactions([4.0, -8.0, 12.0], [-6.0, 10.0, -14.0]);
        packed::solve(&mut [], &mut joints, &mut drives, &mut states, 1);
        for body in states {
            assert_eq!(xyz(&body, 0), [2.0, -4.0, 6.0]);
            assert_eq!(xyz(&body, 32), [-3.0, 5.0, -7.0]);
        }
        let first = states;
        packed::solve(&mut [], &mut joints, &mut drives, &mut states, 5);
        assert_eq!(states, first);
    }
}

#[test]
fn drive_applies_six_independent_symmetric_limits() {
    let mut drive = drive();
    drive_limits(&mut drive, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    column(&mut drive.words, 160, [8.0, -8.0, 0.25, 3.0]);
    column(&mut drive.words, 176, [-9.0, 9.0, -0.75, 6.0]);
    let mut states = reactions([0.0; 3], [0.0; 3]);
    packed::solve(
        &mut [],
        &mut [],
        std::slice::from_mut(&mut drive),
        &mut states,
        1,
    );
    assert_eq!(read(&drive.words, 32), [1.0, -2.0, 0.25, 1.0]);
    assert_eq!(read(&drive.words, 48), [-4.0, 5.0, -0.75, 1.0]);
    assert_eq!(xyz(&states[0], 0), [1.0, -2.0, 0.25]);
    assert_eq!(xyz(&states[1], 0), [-1.0, 2.0, -0.25]);
    assert_eq!(xyz(&states[0], 32), [-4.0, 5.0, -0.75]);
    assert_eq!(xyz(&states[1], 32), [4.0, -5.0, 0.75]);
}

#[test]
fn drive_softness_scales_accumulated_and_projected_response_before_target() {
    let mut drive = drive();
    column(&mut drive.words, 32, [2.0, 0.0, 0.0, 0.5]);
    column(&mut drive.words, 48, [4.0, 0.0, 0.0, 0.25]);
    column(&mut drive.words, 160, [1.0, 0.0, 0.0, 1000.0]);
    column(&mut drive.words, 176, [-1.0, 0.0, 0.0, 1000.0]);
    let mut states = reactions([2.0, 0.0, 0.0], [8.0, 0.0, 0.0]);
    packed::solve(
        &mut [],
        &mut [],
        std::slice::from_mut(&mut drive),
        &mut states,
        1,
    );
    assert_eq!(read(&drive.words, 32), [2.5, 0.0, 0.0, 0.5]);
    assert_eq!(read(&drive.words, 48), [1.0, 0.0, 0.0, 0.25]);
    assert_eq!(states[0][0], 0.5f32.to_bits());
    assert_eq!(states[1][0], 1.5f32.to_bits());
    assert_eq!(states[0][8], (-3.0f32).to_bits());
    assert_eq!(states[1][8], 11.0f32.to_bits());
}

#[test]
fn joint_limits_remove_inside_interval_and_correct_both_outside_sides() {
    let mut joint = origin_constraint();
    column(&mut joint.words, 160, [-1.0, -2.0, -3.0, -4.0]);
    column(&mut joint.words, 176, [1.0, 2.0, 3.0, 4.0]);
    for (offset, value) in [(108, -2.0f32), (124, -3.0), (140, 2.0), (156, 3.0)] {
        joint.words[offset / 4] = value.to_bits();
    }
    let mut states = reactions([6.0, -6.0, 0.0], [8.0, -8.0, 0.0]);
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut joint),
        &mut [],
        &mut states,
        1,
    );
    assert_eq!(xyz(&joint.words, 32), [2.0, -1.0, 0.0]);
    assert_eq!(xyz(&joint.words, 48), [2.0, -1.0, 0.0]);

    for direction in [-1.0, 1.0] {
        let mut joint = joint.clone();
        column(&mut joint.words, 32, [0.0; 4]);
        column(&mut joint.words, 48, [0.0; 4]);
        let mut states = reactions(
            [direction * 2.0, direction * 4.0, direction * 6.0],
            [0.0; 3],
        );
        packed::solve(
            &mut [],
            std::slice::from_mut(&mut joint),
            &mut [],
            &mut states,
            1,
        );
        assert_eq!(xyz(&joint.words, 32), [0.0; 3]);
    }
}

#[test]
fn off_center_linear_and_angular_candidates_share_incoming_reactions() {
    let mut joint = origin_constraint();
    column(&mut joint.words, 0, [0.0, 1.0, 0.0, 0.0]);
    column(&mut joint.words, 16, [0.0, -2.0, 0.0, 0.0]);
    joint.words[64 / 4] = 0.125f32.to_bits();
    joint.words[348 / 4] = 2.0f32.to_bits();
    let mut states = reactions([0.0; 3], [0.0, 0.0, -1.0]);
    states[0][10] = 1.0f32.to_bits();
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut joint),
        &mut [],
        &mut states,
        1,
    );
    assert_eq!(xyz(&joint.words, 32), [-0.125, 0.0, 0.0]);
    assert_eq!(xyz(&joint.words, 48), [0.0, 0.0, -1.0]);
    assert_eq!(xyz(&states[0], 0), [-0.125, 0.0, 0.0]);
    assert_eq!(xyz(&states[1], 0), [0.25, 0.0, 0.0]);
    assert_eq!(xyz(&states[0], 32), [0.0, 0.0, 0.125]);
    assert_eq!(xyz(&states[1], 32), [0.0, 0.0, 0.25]);
}

#[test]
fn shared_reactions_use_full_inertia_tensors_and_separate_arms() {
    let mut drive = drive();
    column(&mut drive.words, 0, [1.0, 2.0, 3.0, 0.0]);
    column(&mut drive.words, 16, [-1.0, 1.0, 2.0, 0.0]);
    column(&mut drive.words, 288, [2.0, 0.25, 0.5, 1.0]);
    column(&mut drive.words, 304, [0.25, 3.0, 0.75, 0.0]);
    column(&mut drive.words, 320, [0.5, 0.75, 4.0, 0.0]);
    column(&mut drive.words, 336, [1.0, -0.5, 0.0, 1.0]);
    column(&mut drive.words, 352, [-0.5, 2.0, 0.25, 0.0]);
    column(&mut drive.words, 368, [0.0, 0.25, 3.0, 0.0]);
    column(&mut drive.words, 160, [2.0, 3.0, 4.0, 1000.0]);
    column(&mut drive.words, 176, [5.0, 6.0, 7.0, 1000.0]);
    let mut states = reactions([0.0; 3], [0.0; 3]);
    packed::solve(
        &mut [],
        &mut [],
        std::slice::from_mut(&mut drive),
        &mut states,
        1,
    );
    assert_eq!(xyz(&states[0], 32), [13.0, 29.5, 32.0]);
    assert_eq!(xyz(&states[1], 32), [4.0, -27.0, -9.5]);
}

#[test]
fn packed_carry_lanes_update_but_pose_only_blocks_and_records_survive() {
    let mut drive = drive();
    for (offset, carry) in [
        (204, 7.0f32),
        (220, 11.0),
        (236, 13.0),
        (300, 2.0),
        (316, 5.0),
        (332, 7.0),
        (348, 3.0),
        (364, 13.0),
        (380, 17.0),
    ] {
        drive.words[offset / 4] = carry.to_bits();
    }
    column(&mut drive.words, 160, [2.0, 3.0, 4.0, 1000.0]);
    column(&mut drive.words, 176, [5.0, 6.0, 7.0, 1000.0]);
    let original = drive.words;
    let mut states = reactions([0.0; 3], [0.0; 3]);
    states[0][3] = 17.0f32.to_bits();
    states[1][3] = 19.0f32.to_bits();
    states[0][11] = 11.0f32.to_bits();
    states[1][11] = 23.0f32.to_bits();
    for state in &mut states {
        state[4..8].copy_from_slice(&[0x12345678; 4]);
        state[12..16].copy_from_slice(&[0x87654321; 4]);
    }
    packed::solve(
        &mut [],
        &mut [],
        std::slice::from_mut(&mut drive),
        &mut states,
        1,
    );
    assert_eq!(states[0][3], 215.0f32.to_bits());
    assert_eq!(states[1][3], (-278.0f32).to_bits());
    assert_eq!(states[0][11], 100.0f32.to_bits());
    assert_eq!(states[1][11], (-189.0f32).to_bits());
    for state in states {
        assert_eq!(&state[4..8], &[0x12345678; 4]);
        assert_eq!(&state[12..16], &[0x87654321; 4]);
    }
    assert_eq!(&drive.words[..8], &original[..8]);
    assert_eq!(&drive.words[16..], &original[16..]);
}

#[test]
fn zero_strength_drive_and_static_body_response_are_distinct() {
    let mut drive = drive();
    drive_limits(&mut drive, [0.0; 3], [0.0; 3]);
    let mut states = reactions([8.0; 3], [6.0; 3]);
    let before = states;
    packed::solve(
        &mut [],
        &mut [],
        std::slice::from_mut(&mut drive),
        &mut states,
        2,
    );
    assert_eq!(states, before);

    let mut drive = origin_constraint();
    drive.words[84..96].fill(0); // Body B supplies no dynamic response.
    let mut states = reactions([8.0, 0.0, 0.0], [0.0; 3]);
    let fixed = states[1];
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut drive),
        &mut [],
        &mut states,
        1,
    );
    assert_eq!(states[1], fixed);
    assert_eq!(states[0][0], 4.0f32.to_bits());
}

#[test]
fn family_order_and_body_indices_share_reactions_between_constraints() {
    let mut joint = origin_constraint();
    let mut drive = drive();
    drive.reaction_a = 1;
    drive.reaction_b = 2;
    let mut states = [[0; 16]; 3];
    states[1][0] = 4.0f32.to_bits();
    states[2][0] = 8.0f32.to_bits();
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut joint),
        std::slice::from_mut(&mut drive),
        &mut states,
        1,
    );
    assert_eq!(
        [states[0][0], states[1][0], states[2][0]],
        [2.0f32, 5.0, 5.0].map(f32::to_bits)
    );
    assert_eq!(drive.words[8], 3.0f32.to_bits());

    let mut reversed = origin_constraint();
    reversed.reaction_a = 1;
    reversed.reaction_b = 0;
    let mut states = reactions([4.0, 0.0, 0.0], [0.0; 3]);
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut reversed),
        &mut [],
        &mut states,
        1,
    );
    assert_eq!(states[0][0], 2.0f32.to_bits());
    assert_eq!(states[1][0], 2.0f32.to_bits());
    assert_eq!(reversed.words[8], (-2.0f32).to_bits());
}

#[test]
fn zero_iterations_preserve_all_packed_words() {
    let mut joints = [origin_constraint()];
    let mut drives = [drive()];
    let original_joint = joints[0].words;
    let original_drive = drives[0].words;
    let mut states = reactions([2.0, 3.0, 4.0], [5.0, 6.0, 7.0]);
    let original_states = states;
    packed::solve(&mut [], &mut joints, &mut drives, &mut states, 0);
    assert_eq!(states, original_states);
    assert_eq!(joints[0].words, original_joint);
    assert_eq!(drives[0].words, original_drive);
}

#[test]
fn nonorthogonal_angular_axes_and_rotated_linear_axes_keep_column_direction() {
    let mut joint = origin_constraint();
    for (axis, values) in [
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
    ]
    .into_iter()
    .enumerate()
    {
        column(&mut joint.words, 192 + axis * 16, values);
    }
    column(&mut joint.words, 64, [0.0, 0.0, 0.5, 0.0]);
    column(&mut joint.words, 96, [0.5, 0.0, 0.0, 0.0]);
    column(&mut joint.words, 128, [0.0, 0.5, 0.0, 0.0]);
    column(&mut joint.words, 240, [1.0, 0.0, 0.0, 0.0]);
    column(&mut joint.words, 256, [1.0, 1.0, 0.0, 0.0]);
    column(&mut joint.words, 272, [0.0, 1.0, 1.0, 0.0]);
    column(&mut joint.words, 80, [0.5, 0.25, 0.0, 0.0]);
    column(&mut joint.words, 112, [0.0, 0.25, 0.25, 0.0]);
    column(&mut joint.words, 144, [0.0, 0.0, 0.25, 0.0]);
    let mut states = reactions([6.0, 2.0, 4.0], [2.0, 4.0, 6.0]);
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut joint),
        &mut [],
        &mut states,
        1,
    );
    assert_eq!(xyz(&joint.words, 32), [1.0, 2.0, 3.0]);
    assert_eq!(xyz(&joint.words, 48), [1.0, 1.5, 2.5]);
    assert_eq!(xyz(&states[0], 0), [3.0, 1.0, 2.0]);
    assert_eq!(xyz(&states[0], 32), [2.5, 4.0, 2.5]);
    assert_eq!(xyz(&states[1], 32), [-0.5, 0.0, 3.5]);
}

#[test]
fn fused_projection_retains_exact_product_cancellation() {
    let mut joint = origin_constraint();
    // (1 + 2^-23) * (1 - 2^-23) - 1 = -2^-46. A rounded multiply
    // followed by addition would erase this independently specified residual.
    joint.words[16] = f32::from_bits(0x3f800001).to_bits();
    joint.words[8] = (-1.0f32).to_bits();
    let mut states = reactions([f32::from_bits(0x3f7ffffe), 0.0, 0.0], [0.0; 3]);
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut joint),
        &mut [],
        &mut states,
        1,
    );
    assert_eq!(joint.words[8], 0xa8800000);
}

#[test]
fn joint_accumulator_carry_is_projected_instead_of_preserved_as_softness() {
    let mut joint = origin_constraint();
    column(&mut joint.words, 32, [0.0, 0.0, 0.0, 2.0]);
    column(&mut joint.words, 48, [0.0, 0.0, 0.0, 3.0]);
    column(&mut joint.words, 160, [0.0, 0.0, 0.0, -4.0]);
    column(&mut joint.words, 176, [0.0, 0.0, 0.0, 4.0]);
    let mut states = reactions([0.0; 3], [0.0; 3]);
    packed::solve(
        &mut [],
        std::slice::from_mut(&mut joint),
        &mut [],
        &mut states,
        1,
    );
    assert_eq!(read(&joint.words, 32), [0.0; 4]);
    assert_eq!(read(&joint.words, 48), [0.0; 4]);
}
