//! Serialization of the uncompiled TU3 256-byte contact, in guest word order.
use super::contact::{RetailContact, RetailContactWorkspace};
use crate::math::Vector3;

pub(super) fn encode(c: RetailContact) -> [u32; 64] {
    let mut words = [0; 64];
    let header = [
        lanes(c.position_on_a, c.body_a_id),
        lanes(c.position_on_b, c.body_b_id),
        lanes(c.normal, c.restitution.to_bits()),
        lanes(c.tangent_0, c.static_friction.to_bits()),
        lanes(c.tangent_1, c.dynamic_friction.to_bits()),
        lanes(c.relative_velocity, c.tag),
    ];
    for (i, v) in header.into_iter().enumerate() {
        words[i * 4..i * 4 + 4].copy_from_slice(&v);
    }
    for (i, (a, b)) in workspace(c.body_a_workspace)
        .into_iter()
        .zip(workspace(c.body_b_workspace))
        .enumerate()
    {
        words[24 + i * 8..28 + i * 8].copy_from_slice(&a);
        words[28 + i * 8..32 + i * 8].copy_from_slice(&b);
    }
    words
}
fn lanes(v: Vector3, w: u32) -> [u32; 4] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits(), w]
}
fn workspace(w: RetailContactWorkspace) -> [[u32; 4]; 5] {
    [
        lanes(w.center_of_mass, w.reaction_id),
        lanes(w.inverse_inertia_full, w.inverse_mass.to_bits()),
        lanes(w.inverse_inertia_split, w.state),
        lanes(w.force_acceleration, w.kinetic_energy.to_bits()),
        lanes(w.torque_acceleration, w.cool_down),
    ]
}
