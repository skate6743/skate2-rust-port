//! TU3 queued point-force application, 0x82C037DC..0x82C03908.
use crate::math::{Basis3, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailForceAccumulator {
    pub force_acceleration: Vector3,
    pub torque_acceleration: Vector3,
    pub cool_down: u32,
}

/// Apply one queued force using the native multiply / fused-add order.
/// `application_point_body` already includes the collection's force-point Y
/// offset (added by the queue consumer at 0x82C037EC).
pub fn accumulate_point_force(
    mut accumulator: RetailForceAccumulator,
    force_world: Vector3,
    application_point_body: Vector3,
    deck_basis: Basis3,
    inverse_mass: f32,
    world_inverse_inertia: Basis3,
) -> RetailForceAccumulator {
    // 0x82C03850: vmulfp; 0x82C03878..0x82C03894: scalar adds.
    accumulator.force_acceleration.x += force_world.x * inverse_mass;
    accumulator.force_acceleration.y += force_world.y * inverse_mass;
    accumulator.force_acceleration.z += force_world.z * inverse_mass;
    let arm = multiply_basis(deck_basis, application_point_body);
    // 0x82C0387C / 0x82C038B0 / 0x82C038B4: multiply, fused negative
    // multiply-add, permutation. Fusing the positive term instead differs.
    let torque = Vector3::new(
        (-arm.z).mul_add(force_world.y, arm.y * force_world.z),
        (-arm.x).mul_add(force_world.z, arm.z * force_world.x),
        (-arm.y).mul_add(force_world.x, arm.x * force_world.y),
    );
    let angular = multiply_basis(world_inverse_inertia, torque);
    accumulator.torque_acceleration.x += angular.x;
    accumulator.torque_acceleration.y += angular.y;
    accumulator.torque_acceleration.z += angular.z;
    accumulator.cool_down = 0;
    accumulator
}

fn multiply_basis(basis: Basis3, v: Vector3) -> Vector3 {
    // Native multiplies column 0, then fuses column 1, then column 2.
    let lane = |i: usize| {
        basis.columns[2][i].mul_add(
            v.z,
            basis.columns[1][i].mul_add(v.y, basis.columns[0][i] * v.x),
        )
    };
    Vector3::new(lane(0), lane(1), lane(2))
}
