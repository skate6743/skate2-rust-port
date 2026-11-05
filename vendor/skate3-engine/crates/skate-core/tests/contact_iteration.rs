//! Independent analytical cases for compiled rows. These exercise the solver
//! without pretending the still-unavailable contact builder has been validated.
use skate_core::physics::solver::packed::{self, Contact, Reaction};

fn row(words: &mut [u32], index: usize, value: [f32; 4]) {
    for (i, value) in value.into_iter().enumerate() {
        words[index * 4 + i] = value.to_bits();
    }
}
fn read(words: &[u32], index: usize) -> [f32; 4] {
    core::array::from_fn(|i| f32::from_bits(words[index * 4 + i]))
}
fn contact(target: [f32; 4], inverse_mass: f32) -> Contact {
    let mut words = [0; 64];
    row(&mut words, 2, [1.0, 0.0, 0.0, 0.0]);
    row(&mut words, 3, [0.0, 1.0, 0.0, 0.5]);
    row(&mut words, 4, [0.0, 0.0, 1.0, 0.25]);
    row(&mut words, 6, target);
    row(&mut words, 7, [1.0, 0.0, 0.0, 0.0]);
    row(&mut words, 10, [0.0, 1.0, 0.0, 0.0]);
    row(&mut words, 13, [0.0, 0.0, 1.0, 0.0]);
    row(&mut words, 8, [0.0, 0.0, 0.0, inverse_mass]);
    Contact {
        words,
        reaction_a: 0,
        reaction_b: 1,
    }
}
fn once(contact: &mut Contact, reactions: &mut [Reaction; 2]) {
    packed::solve(
        core::slice::from_mut(contact),
        &mut [],
        &mut [],
        reactions,
        1,
    );
}

#[test]
fn first_normal_impulse_does_not_enable_friction_in_the_same_pass() {
    let mut contact = contact([2.0, 1.0, -1.0, 0.5], 1.0);
    let mut reactions = [[0; 16]; 2];
    once(&mut contact, &mut reactions);
    assert_eq!(read(&contact.words, 5), [2.0, 0.0, 0.0, 0.5]);
    assert_eq!(&read(&reactions[0], 0)[..3], &[2.0, 0.0, 0.0]);
    assert_eq!(&read(&reactions[0], 1)[..3], &[0.5, 0.0, 0.0]);
    assert_eq!(reactions[1], [0; 16]);
}

#[test]
fn static_boundary_is_inclusive_but_sliding_uses_dynamic_limit() {
    for (target, expected) in [(1.0, 1.0), (-1.0, -1.0), (1.125, 0.5), (-1.125, -0.5)] {
        let mut contact = contact([0.0, target, 0.0, 0.0], 0.0);
        row(&mut contact.words, 5, [2.0, 0.0, 0.0, 0.0]);
        once(&mut contact, &mut [[0; 16]; 2]);
        assert_eq!(read(&contact.words, 5), [2.0, expected, 0.0, 0.0]);
    }
}

#[test]
fn angular_motion_at_contact_feeds_the_normal_row() {
    let mut contact = contact([0.0; 4], 0.0);
    row(&mut contact.words, 0, [0.0, 1.0, 0.0, 0.0]);
    row(&mut contact.words, 8, [0.0, 0.0, -1.0, 0.0]);
    let mut reactions = [[0; 16]; 2];
    row(&mut reactions[0], 2, [0.0, 0.0, 2.0, 0.0]);
    once(&mut contact, &mut reactions);
    assert_eq!(read(&contact.words, 5), [2.0, 0.0, 0.0, 0.0]);
    assert_eq!(read(&reactions[0], 2), [0.0; 4]);
}

#[test]
fn later_contact_reads_the_reaction_from_the_previous_contact() {
    let mut contacts = [
        contact([2.0, 0.0, 0.0, 0.0], 1.0),
        contact([3.0, 0.0, 0.0, 0.0], 1.0),
    ];
    let mut reactions = [[0; 16]; 2];
    packed::solve(&mut contacts, &mut [], &mut [], &mut reactions, 1);
    assert_eq!(read(&contacts[0].words, 5)[0], 2.0);
    assert_eq!(read(&contacts[1].words, 5)[0], 1.0);
    assert_eq!(read(&reactions[0], 0)[0], 3.0);
}

#[test]
fn body_b_receives_opposite_linear_and_angular_changes() {
    let mut contact = contact([2.0, 0.0, 0.0, 0.5], 1.0);
    row(&mut contact.words, 9, [0.0, 0.0, 3.0, 0.25]);
    let mut reactions = [[0; 16]; 2];
    once(&mut contact, &mut reactions);
    assert_eq!(&read(&reactions[1], 0)[..3], &[-0.5, 0.0, 0.0]);
    assert_eq!(&read(&reactions[1], 1)[..3], &[-0.125, 0.0, 0.0]);
    assert_eq!(&read(&reactions[1], 2)[..3], &[0.0, 0.0, -6.0]);
    assert_eq!(&read(&reactions[1], 3)[..3], &[0.0, 0.0, -1.5]);
}

#[test]
fn ground_contact_reaches_joint_then_drive_in_one_iteration() {
    // Three equal-mass bodies at their centers. Ground first gives body0 a
    // correction of2. The joint equalizes0/1 to1; the drive then equalizes1/2
    // to0.5. Solving families from independent snapshots cannot produce this.
    let mut ground = contact([2.0, 0.0, 0.0, 0.0], 1.0);
    ground.reaction_b = 3;
    let mut words = [0; 96];
    row(&mut words, 4, [0.5, 0.0, 0.0, 0.0]);
    row(&mut words, 12, [1.0, 0.0, 0.0, 0.0]);
    row(&mut words, 18, [0.0, 0.0, 0.0, 1.0]);
    row(&mut words, 21, [0.0, 0.0, 0.0, 1.0]);
    let mut joint = packed::Joint {
        words,
        reaction_a: 0,
        reaction_b: 1,
    };
    row(&mut words, 2, [0.0, 0.0, 0.0, 1.0]);
    words[6 * 4 + 3] = 10.0f32.to_bits();
    let mut drive = packed::Drive {
        words,
        reaction_a: 1,
        reaction_b: 2,
    };
    let mut reactions = [[0; 16]; 4];
    packed::solve(
        core::slice::from_mut(&mut ground),
        core::slice::from_mut(&mut joint),
        core::slice::from_mut(&mut drive),
        &mut reactions,
        1,
    );
    assert_eq!(
        reactions.map(|r| f32::from_bits(r[0])),
        [1.0, 0.5, 0.5, 0.0]
    );
    assert_eq!(read(&ground.words, 5)[0], 2.0);
    assert_eq!(read(&joint.words, 2)[0], -1.0);
    assert_eq!(read(&drive.words, 2)[0], -0.5);
}
