//!82C07328: requested angular displacement becomes deck angular acceleration.
//! Keep both inverse-inertia transforms: cancellation would change rounding and
//! singular-matrix behavior. Ordinary PC reciprocal seeds are not Xenon parity.
use super::{Vector, math::*};
pub fn acceleration_delta(request: Vector, omega: Vector, inverse_inertia: [Vector; 3]) -> Vector {
    let [a, b, c] = inverse_inertia;
    let cofactor = [cross(b, c), cross(c, a), cross(a, b)];
    let determinant = dot(a, cofactor[0]);
    let inv_det = reciprocal(determinant, 2);
    let inverse: [Vector; 3] = std::array::from_fn(|i| {
        [
            cofactor[0][i] * inv_det,
            cofactor[1][i] * inv_det,
            cofactor[2][i] * inv_det,
            0.,
        ]
    });
    let dt = f32::from_bits(0x3c888889);
    let axis = unit(request, [0.; 4]);
    let residual = sub(request, scale(axis, dot(axis, scale(omega, dt))));
    let local = transform_direction(
        [inverse[0], inverse[1], inverse[2], [0.; 4]],
        scale(residual, reciprocal(dt, 2)),
    );
    transform_direction([a, b, c, [0.; 4]], scale(local, reciprocal(dt, 2)))
}
