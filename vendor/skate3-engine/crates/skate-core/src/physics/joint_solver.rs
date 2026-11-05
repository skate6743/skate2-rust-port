//! Semantic port of Skate 3 TU3's six-degree joint iteration.
//!
//! Skate 3 builds one 384-byte `JointJacobian` for each of the six skateboard
//! assembly joints. `rw::physics::Simulation::Solve` visits contacts, joints,
//! and drives in that order on each configured outer iteration. The joint loop
//! starts near `0x82AE2BC8`; the loop near `0x82AE2E78` is the drive family.
//!
//! Generated-code validation is withdrawn. All arithmetic and packed-lane
//! semantics require independent review against TU3 IDA before parity claims.

use crate::physics::rigid_body::RetailReactionCorrections;

pub mod tu3 {
    pub const SIMULATION_SOLVE: u32 = 0x82AE_27D0;
    pub const JOINT_SOLVE_LOOP: u32 = 0x82AE_2BC8;
    pub const DRIVE_SOLVE_LOOP: u32 = 0x82AE_2E78;
    pub const JOINT_BATCH_BUILD: u32 = 0x82AE_39D0;
    pub const JOINT_JACOBIAN_BUILD: u32 = 0x82AE_3BC8;
    pub const JACOBIAN_BYTES: usize = 384;
    pub const REACTION_BYTES: usize = 64;
}

const JACOBIAN_VECTOR_COUNT: usize = tu3::JACOBIAN_BYTES / 16;
const REACTION_VECTOR_COUNT: usize = tu3::REACTION_BYTES / 16;

/// Raw guest word order of TU3's 384-byte `JointJacobian`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct RetailJointJacobian {
    pub words: [u32; JACOBIAN_VECTOR_COUNT * 4],
}

/// Four raw vectors indexed by a rigid body's solver id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct RetailJointReactionBlock {
    pub words: [u32; REACTION_VECTOR_COUNT * 4],
}

impl RetailJointReactionBlock {
    /// Packs the two reaction vectors consumed by the joint-family branch.
    ///
    /// TU3 leaves reaction vectors one and three to the contact family's
    /// position/orientation correction.
    pub fn from_reactions(reactions: RetailReactionCorrections) -> Self {
        let mut words = [0; REACTION_VECTOR_COUNT * 4];
        words[0] = reactions.linear_displacement.x.to_bits();
        words[1] = reactions.linear_displacement.y.to_bits();
        words[2] = reactions.linear_displacement.z.to_bits();
        words[8] = reactions.angular_displacement.x.to_bits();
        words[9] = reactions.angular_displacement.y.to_bits();
        words[10] = reactions.angular_displacement.z.to_bits();
        Self { words }
    }

    /// Writes back only the two joint-owned velocity correction vectors.
    pub fn write_velocity_reactions(self, reactions: &mut RetailReactionCorrections) {
        reactions.linear_displacement.x = f32::from_bits(self.words[0]);
        reactions.linear_displacement.y = f32::from_bits(self.words[1]);
        reactions.linear_displacement.z = f32::from_bits(self.words[2]);
        reactions.angular_displacement.x = f32::from_bits(self.words[8]);
        reactions.angular_displacement.y = f32::from_bits(self.words[9]);
        reactions.angular_displacement.z = f32::from_bits(self.words[10]);
    }
}

/// One joint pass using TU3's fused vector operations and projector.
pub fn solve_joint_iteration(
    jacobian: &mut RetailJointJacobian,
    reaction_a: &mut RetailJointReactionBlock,
    reaction_b: &mut RetailJointReactionBlock,
) {
    super::solver::solve_joint_record(
        &mut jacobian.words,
        &mut reaction_a.words,
        &mut reaction_b.words,
    );
}

#[cfg(test)]
#[path = "tests/joint_solver.rs"]
mod tests;
