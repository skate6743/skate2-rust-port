//! Native solver records with host indices replacing guest reaction pointers.
use super::{contact, drive, joint};

#[derive(Clone, Debug)]
pub struct Constraint<const WORDS: usize> {
    pub words: [u32; WORDS],
    pub reaction_a: usize,
    pub reaction_b: usize,
}

pub type Contact = Constraint<64>;
pub type Joint = Constraint<96>;
pub type Drive = Constraint<96>;
pub type Reaction = [u32; 16];

/// Complete contact → joint → drive loops of HorusPipeline (82AE27D0).
/// Contact position corrections remain distinct from velocity-producing
/// reactions; all families see preceding writes during the same iteration.
pub fn solve(
    contacts: &mut [Contact],
    joints: &mut [Joint],
    drives: &mut [Drive],
    reactions: &mut [Reaction],
    iterations: u32,
) {
    validate(contacts, reactions.len());
    validate(joints, reactions.len());
    validate(drives, reactions.len());
    for _ in 0..iterations {
        pass(contacts, reactions, contact::solve);
        pass(joints, reactions, joint::solve);
        pass(drives, reactions, drive::solve);
    }
}

fn validate<const N: usize>(constraints: &[Constraint<N>], count: usize) {
    for c in constraints {
        assert_ne!(c.reaction_a, c.reaction_b);
        assert!(c.reaction_a < count && c.reaction_b < count);
    }
}

fn pass<const N: usize>(
    constraints: &mut [Constraint<N>],
    reactions: &mut [Reaction],
    kernel: fn(&mut [u32], &mut [u32], &mut [u32]),
) {
    for c in constraints {
        let (a, b) = if c.reaction_a < c.reaction_b {
            let (left, right) = reactions.split_at_mut(c.reaction_b);
            (&mut left[c.reaction_a], &mut right[0])
        } else {
            let (left, right) = reactions.split_at_mut(c.reaction_a);
            (&mut right[0], &mut left[c.reaction_b])
        };
        kernel(&mut c.words, a, b);
    }
}
