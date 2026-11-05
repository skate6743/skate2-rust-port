//! Concrete CURRENT BoardRuntime adaptation of AddWorldForce82C03F98.
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{
        board::BodyId,
        board_runtime::BoardRuntime,
        point_force::{RetailForceAccumulator, accumulate_point_force},
    },
};

/// Immediate deck acceleration/torque accumulation. Unlike queued board forces,
/// this has no capacity, attribution tag or stock force-point Y offset.
pub(crate) fn apply_world_force(board: &mut BoardRuntime, force: [f32; 4], point: [f32; 4]) {
    let deck = &mut board.bodies_mut()[BodyId::Deck.index()];
    apply_to_deck(deck, force, point);
}

pub(super) fn apply_to_deck(
    deck: &mut skate3_core::physics::assembly::BodySnapshot,
    force: [f32; 4],
    point: [f32; 4],
) {
    let rates = &mut deck.rates;
    let arm = Vector3::new(
        point[0] - rates.position.x,
        point[1] - rates.position.y,
        point[2] - rates.position.z,
    );
    // The shared helper's input is already world-space here, so its basis is
    // identity. Preserve its native cross/tensor accumulation order.
    let next = accumulate_point_force(
        RetailForceAccumulator {
            force_acceleration: rates.force_acceleration,
            torque_acceleration: rates.torque_acceleration,
            cool_down: rates.cool_down,
        },
        Vector3::new(force[0], force[1], force[2]),
        arm,
        Basis3 {
            columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        deck.inertia.inverse_mass,
        rates.world_inverse_inertia,
    );
    rates.force_acceleration = next.force_acceleration;
    rates.torque_acceleration = next.torque_acceleration;
    rates.cool_down = next.cool_down;
}
