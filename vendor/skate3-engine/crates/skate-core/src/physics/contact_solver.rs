//! TU3 ContactBatchBuild and the shared Horus contact solver.
//! Typed contacts are encoded only at the compiled-row boundary. Contact
//! iteration has a readable direct-source implementation; the native builder's
//! reciprocal-estimate dependency remains unavailable.
use super::{contact::RetailContact, rigid_body::RetailReactionCorrections};
use crate::math::Vector3;

pub mod tu3 {
    pub const CONTACT_BATCH_BUILD: u32 = 0x82AE_10C8;
    pub const ITERATIVE_CONSTRAINT_SOLVER: u32 = 0x82AE_27D0;
}
pub const ACTIVE_BODY: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailContactJacobian {
    pub(crate) words: [u32; 64],
    pub reaction_index_a: usize,
    pub reaction_index_b: usize,
}
impl RetailContactJacobian {
    /// Native compiled layout, for inspection and independent comparisons.
    pub fn words(&self) -> &[u32; 64] {
        &self.words
    }
    pub fn accumulated_impulse(&self) -> [f32; 4] {
        self.lanes(5)
    }
    /// Weighted velocity targets in xyz and position-only target in w.
    pub fn target_impulse(&self) -> [f32; 4] {
        self.lanes(6)
    }
    pub fn static_friction(&self) -> f32 {
        f32::from_bits(self.words[15])
    }
    pub fn dynamic_friction(&self) -> f32 {
        f32::from_bits(self.words[19])
    }
    pub fn angular_response_a(&self) -> [Vector3; 3] {
        core::array::from_fn(|i| self.xyz(8 + i * 3))
    }
    pub fn angular_response_b(&self) -> [Vector3; 3] {
        core::array::from_fn(|i| self.xyz(9 + i * 3))
    }
    fn lanes(&self, vector: usize) -> [f32; 4] {
        core::array::from_fn(|i| f32::from_bits(self.words[vector * 4 + i]))
    }
    fn xyz(&self, vector: usize) -> Vector3 {
        let v = self.lanes(vector);
        Vector3::new(v[0], v[1], v[2])
    }
}

/// Requests native contact construction. The unresolved reciprocal estimate
/// currently stops this call explicitly; callers must not treat compiled-row
/// solver tests as evidence that this producer is ready for gameplay.
/// Each input body workspace must include forces accumulated for this tick.
pub fn build_contact_jacobian(contact: RetailContact, time_step: f32) -> RetailContactJacobian {
    assert!(time_step.is_finite() && time_step > 0.0);
    let mut words = super::contact_layout::encode(contact);
    super::solver::compile_contact(&mut words, time_step);
    RetailContactJacobian {
        words,
        reaction_index_a: contact.body_a_workspace.reaction_id as usize,
        reaction_index_b: contact.body_b_workspace.reaction_id as usize,
    }
}

/// Contact-only entry point uses the same kernel and reaction layout as the
/// complete contact -> joint -> drive iteration. Friction reads the preceding
/// normal impulse, before this pass publishes its new normal impulse.
pub fn solve_contact_jacobians(
    contacts: &mut [RetailContactJacobian],
    reactions: &mut [RetailReactionCorrections],
    maximum_iterations: u32,
) {
    super::solver::solve_constraints(contacts, &mut [], &mut [], reactions, maximum_iterations);
}
pub fn solve_contact_jacobian_iteration(
    contacts: &mut [RetailContactJacobian],
    reactions: &mut [RetailReactionCorrections],
) {
    solve_contact_jacobians(contacts, reactions, 1);
}

#[cfg(test)]
#[path = "tests/contact_solver.rs"]
mod tests;
