//! Shared TU3 constraint iteration (`0x82AE27D0`).
//!
//! Constraint families must exchange reactions on every pass. Solving each
//! family to completion independently changes the physical result.
use super::{
    contact_solver::RetailContactJacobian, drive_solver::RetailDriveRows,
    joint_solver::RetailJointJacobian, rigid_body::RetailReactionCorrections,
};
mod contact;
pub mod contact_build;
mod drive;
mod joint;
pub mod packed;
mod packing;

pub(crate) fn compile_contact(record: &mut [u32; 64], time_step: f32) {
    contact_build::build(record, time_step);
}

pub(crate) fn solve_joint_record(record: &mut [u32], a: &mut [u32], b: &mut [u32]) {
    joint::solve(record, a, b);
}

/// Host indices replace the two guest reaction pointers stored in a Jacobian.
#[derive(Clone, Debug)]
pub struct JointConstraint {
    pub jacobian: RetailJointJacobian,
    pub reaction_a: usize,
    pub reaction_b: usize,
}

/// Runs the retail contact â†’ joint â†’ drive ordering. The caller clears the
/// reaction workspace and builds fresh rows once per simulation step.
pub fn solve_constraints(
    contacts: &mut [RetailContactJacobian],
    joints: &mut [JointConstraint],
    drives: &mut [RetailDriveRows],
    reactions: &mut [RetailReactionCorrections],
    iterations: u32,
) {
    let mut c: Vec<_> = contacts.iter().map(packing::contact).collect();
    let mut j: Vec<_> = joints
        .iter()
        .map(|j| packed::Joint {
            words: j.jacobian.words,
            reaction_a: j.reaction_a,
            reaction_b: j.reaction_b,
        })
        .collect();
    let mut d: Vec<_> = drives.iter().map(packing::drive).collect();
    let mut r: Vec<_> = reactions.iter().copied().map(packing::reaction).collect();
    packed::solve(&mut c, &mut j, &mut d, &mut r, iterations);
    for (contact, native) in contacts.iter_mut().zip(c) {
        contact.words = native.words;
    }
    for (joint, native) in joints.iter_mut().zip(j) {
        joint.jacobian.words = native.words;
    }
    for (drive, native) in drives.iter_mut().zip(d) {
        drive.accumulated_linear_impulse = packing::read_xyz(&native.words, 2);
        drive.accumulated_angular_impulse = packing::read_xyz(&native.words, 3);
    }
    for (reaction, native) in reactions.iter_mut().zip(r) {
        *reaction = packing::unpack_reaction(native);
    }
}
