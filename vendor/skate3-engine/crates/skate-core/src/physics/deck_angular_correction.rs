//! Deck angular corrections from TU3 82C07000 and 82C075B8.
//! These add angular acceleration before the shared solve. They never set yaw
//! or orientation directly. Both native fixed-step conversions are retained.
use super::{
    board_motion_output::{dot, inverse_length_squared},
    native_arithmetic::reciprocal_estimate,
    rigid_body::RetailBodyRates,
};
use crate::math::{Basis3, Vector3};
const STEP: f32 = f32::from_bits(0x3c88_8889);
const NORMAL_MINIMUM: f32 = f32::from_bits(0x3586_37bd); //82F826F8 ->830BD350.

///82C07328 subtracts the current angular displacement along the requested
///axis, without07000's directional clamps, then performs the same tensor and
///fixed-step conversion as075B8. The thrown-board controller calls this leaf.
pub fn apply_axis_displacement(body: &mut RetailBodyRates, requested: Vector3) {
    let squared = dot(requested, requested);
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    let direction = if length > NORMAL_MINIMUM {
        scale(requested, inverse)
    } else {
        Vector3::ZERO
    };
    let existing = scale(
        direction,
        dot(direction, scale(body.angular_velocity, STEP)),
    );
    apply_angular_displacement(body, subtract(requested, existing));
}

/// 82C07000 first limits the requested angular displacement against the
/// displacement already supplied by angular velocity along that same axis.
pub fn apply_limited_displacement(body: &mut RetailBodyRates, requested: Vector3) {
    let squared = dot(requested, requested);
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    let direction = if length > NORMAL_MINIMUM {
        scale(requested, inverse)
    } else {
        Vector3::ZERO
    };
    let existing = scale(
        direction,
        dot(direction, scale(body.angular_velocity, STEP)),
    );
    let remainder = subtract(requested, existing);
    let remainder = if dot(remainder, requested) < 0.0 {
        Vector3::ZERO
    } else {
        remainder
    };
    let selected = if dot(existing, requested) < 0.0 {
        requested
    } else {
        remainder
    };
    apply_angular_displacement(body, selected);
}

/// 82C075B8. The inverse of the current world inverse-inertia tensor converts
/// displacement/step to torque; another step division and the original tensor
/// convert it to angular acceleration. Do not algebraically cancel the tensor
/// pair: its rounded cofactors and ordered products are observable.
pub fn apply_angular_displacement(body: &mut RetailBodyRates, displacement: Vector3) {
    let tensor = body.world_inverse_inertia;
    let inverse = invert_symmetric(tensor);
    let inverse_step = reciprocal_refined(STEP);
    let target_rate = scale(displacement, inverse_step);
    let momentum = multiply(inverse, target_rate);
    let torque = scale(momentum, reciprocal_refined(STEP));
    let acceleration = multiply(tensor, torque);
    body.torque_acceleration = Vector3::new(
        body.torque_acceleration.x + acceleration.x,
        body.torque_acceleration.y + acceleration.y,
        body.torque_acceleration.z + acceleration.z,
    );
    // Native accumulator +172 shares its vector with the cooldown word.
    body.cool_down = 0;
}

fn invert_symmetric(tensor: Basis3) -> Basis3 {
    let [a, b, c] = tensor.columns.map(|[x, y, z]| Vector3::new(x, y, z));
    let first = cross(b, c);
    let second = cross(c, a);
    let third = cross(a, b);
    let inverse_det = reciprocal_refined(dot(a, first));
    Basis3 {
        columns: [
            [first.x, second.x, third.x].map(|x| x * inverse_det),
            [first.y, second.y, third.y].map(|x| x * inverse_det),
            [first.z, second.z, third.z].map(|x| x * inverse_det),
        ],
    }
}
fn reciprocal_refined(value: f32) -> f32 {
    let estimate = reciprocal_estimate(value);
    let first = estimate.mul_add((-estimate).mul_add(value, 1.0), estimate);
    first.mul_add((-first).mul_add(value, 1.0), first)
}
fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    )
}
fn multiply(matrix: Basis3, v: Vector3) -> Vector3 {
    let lane = |i: usize| {
        matrix.columns[2][i].mul_add(
            v.z,
            matrix.columns[1][i].mul_add(v.y, matrix.columns[0][i] * v.x),
        )
    };
    Vector3::new(lane(0), lane(1), lane(2))
}
fn scale(v: Vector3, x: f32) -> Vector3 {
    Vector3::new(v.x * x, v.y * x, v.z * x)
}
fn subtract(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::rigid_body::RetailQuaternion;
    fn body() -> RetailBodyRates {
        let identity = Basis3 {
            columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        RetailBodyRates {
            orientation: RetailQuaternion::IDENTITY,
            basis: identity,
            world_inverse_inertia: Basis3 {
                columns: [[3.0, 0.2, 0.1], [0.2, 2.0, 0.3], [0.1, 0.3, 1.0]],
            },
            position: Vector3::ZERO,
            linear_velocity: Vector3::ZERO,
            angular_velocity: Vector3::ZERO,
            force_acceleration: Vector3::ZERO,
            torque_acceleration: Vector3::ZERO,
            kinetic_energy: 0.0,
            cool_down: 3,
        }
    }
    #[test]
    fn thrown_board_correction_brakes_axis_overshoot_without_touching_other_motion() {
        let mut state = body();
        state.angular_velocity = Vector3::new(4., 3., 5.);
        apply_axis_displacement(&mut state, Vector3::new(0., 0.02, 0.));
        assert!((state.torque_acceleration.y + 108.).abs() < 0.0001);
        assert!(state.torque_acceleration.x.abs() < 0.0001);
        assert!(state.torque_acceleration.z.abs() < 0.0001);
        assert_eq!(state.angular_velocity, Vector3::new(4., 3., 5.));
        assert_eq!(state.cool_down, 0);
        let mut reverse = body();
        reverse.angular_velocity = Vector3::new(0., -3., 0.);
        apply_axis_displacement(&mut reverse, Vector3::new(0., 0.02, 0.));
        assert!((reverse.torque_acceleration.y - 252.).abs() < 0.0001);
    }
    #[test]
    fn correction_enters_accumulator_and_respects_existing_angular_motion() {
        let request = Vector3::new(0.02, 0.01, -0.03);
        let mut state = body();
        apply_angular_displacement(&mut state, request);
        for (actual, want) in [
            (state.torque_acceleration.x, 72.0),
            (state.torque_acceleration.y, 36.0),
            (state.torque_acceleration.z, -108.0),
        ] {
            assert!((actual - want).abs() < 0.0001);
        }
        assert_eq!(state.orientation, RetailQuaternion::IDENTITY);
        assert_eq!(state.cool_down, 0);
        let mut overshoot = body();
        overshoot.angular_velocity = Vector3::new(0.0, 3.0, 0.0);
        apply_limited_displacement(&mut overshoot, Vector3::new(0.0, 0.02, 0.0));
        assert_eq!(overshoot.torque_acceleration, Vector3::ZERO);
        let mut against = body();
        against.angular_velocity = Vector3::new(0.0, -3.0, 0.0);
        apply_limited_displacement(&mut against, Vector3::new(0.0, 0.02, 0.0));
        assert!((against.torque_acceleration.y - 72.0).abs() < 0.0001);
        let mut ground = body();
        ground.basis = Basis3 {
            columns: [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]],
        };
        apply_ground_body_torque(&mut ground);
        assert!((ground.torque_acceleration.x + 1.815).abs() < 0.00001);
        assert!((ground.torque_acceleration.y + 0.121).abs() < 0.00001);
        assert!((ground.torque_acceleration.z + 0.0605).abs() < 0.00001);
        assert_eq!(ground.force_acceleration, Vector3::ZERO);
    }
}

/// Ground board update82D389DC..82D38C98. Skateboard+16 is the cached deck
/// rigid body (ctor82C01278); this torque therefore enters the same accumulator.
/// The inline pow calculation has fixed inputs10 and-1. Retain its two
/// coefficient trees and reciprocal refinements instead of replacing it by.1.
pub fn apply_ground_body_torque(body: &mut RetailBodyRates) {
    let local = Vector3::new(0.0, ground_torque_scalar(), 0.0);
    let world = multiply(body.basis, local);
    let acceleration = multiply(body.world_inverse_inertia, world);
    body.torque_acceleration = Vector3::new(
        acceleration.x + body.torque_acceleration.x,
        acceleration.y + body.torque_acceleration.y,
        acceleration.z + body.torque_acceleration.z,
    );
    body.cool_down = 0;
}
fn ground_torque_scalar() -> f32 {
    // Positive fixed input10 has exponent3 and mantissa1.25. Thus the native
    // vlog estimate followed by floor selects3; its sign/special-value masks
    // cannot select another branch for these immutable instruction inputs.
    let x = 0.25_f32;
    let square = x * x;
    let cube = x * square;
    let fourth = square * square;
    let log = [
        0x3fb8aa0e, 0xbf389e52, 0x3ef5162d, 0xbeb1d204, 0x3e77adbd, 0xbe0cd4fb, 0x3d5541c6,
        0xbc188b0b,
    ]
    .map(f32::from_bits);
    let low = cube.mul_add(log[3], square.mul_add(log[2], x.mul_add(log[1], log[0])));
    let high = cube.mul_add(log[7], square.mul_add(log[6], x.mul_add(log[5], log[4])));
    let exponent = (-x).mul_add(fourth.mul_add(high, low), -3.0);
    let integral = exponent.floor();
    let fraction = exponent - integral;
    let square = fraction * fraction;
    let cube = fraction * square;
    let fourth = square * square;
    let exp = [
        0x3f800000, 0xbf317218, 0x3e75fded, 0xbd6357ca, 0x3c1d8c54, 0xbaae1854, 0x391aa7d7,
        0xb7364261,
    ]
    .map(f32::from_bits);
    let low = cube.mul_add(
        exp[3],
        square.mul_add(exp[2], fraction.mul_add(exp[1], exp[0])),
    );
    let high = cube.mul_add(
        exp[7],
        square.mul_add(exp[6], fraction.mul_add(exp[5], exp[4])),
    );
    let polynomial = fourth.mul_add(high, low);
    //vexptefp sees an integral exponent here, so its power of two is exact.
    let power_of_two = f32::from_bits(((127 + integral as i32) as u32) << 23);
    (power_of_two * reciprocal_refined(polynomial)) * f32::from_bits(0xc0c1_999a)
}
