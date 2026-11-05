//! Remaining numerical ground corrections from TU3 82D38800 and its callees.
//! These operate on actual physical vectors; phase/counter gates stay in the
//! grounded state owner and all body writes target the shared board solver.
use crate::{
    math::Vector3,
    physics::{
        board_motion_output::{dot, inverse_length_squared, length},
        native_arithmetic::reciprocal_estimate,
    },
};

///82D38838..AC: length of Skeleton11008, with two reciprocal-square-root refinements.
pub fn center_of_mass_height(com_to_deck_world: [f32; 4]) -> f32 {
    length(xyz(com_to_deck_world))
}

pub fn dot_product(a: [f32; 4], b: [f32; 4]) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}

/// 82D38E18..82D38EA8: normalize the force with two refinements, reject
/// lengths no greater than the live native epsilon, then dot with velocity.
/// Velocity deliberately remains unnormalized.
pub fn collision_force_projection(force: [f32; 4], velocity: [f32; 4]) -> f32 {
    let squared = dot(xyz(force), xyz(force));
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    let direction = if length > f32::from_bits(0x3586_37bd) {
        force.map(|v| v * inverse)
    } else {
        [0.0; 4]
    };
    dot(xyz(direction), xyz(velocity))
}

/// 82D37048: edge direction is divided by its computed length. The native
/// code uses two reciprocal refinements after its two square-root refinements;
/// it does not replace that sequence with direct inverse-length scaling.
pub fn edge_direction(start: [f32; 4], end: [f32; 4]) -> [f32; 4] {
    let delta = subtract(end, start);
    let inverse = reciprocal_refined(length(xyz(delta)));
    delta.map(|v| v * inverse)
}

/// 82D370F8: positive-Y perpendicular to an edge. The degenerate result is
/// native world X (82139A10), not world Y or an arbitrary orthogonal vector.
pub fn edge_up(direction: [f32; 4]) -> [f32; 4] {
    let first = cross([0., 1., 0., 0.], direction);
    let perpendicular = cross(first, direction);
    let magnitude = length(xyz(perpendicular));
    if magnitude <= 0.0 {
        return [1., 0., 0., 0.];
    }
    let signed_length = if perpendicular[1] < 0.0 {
        -magnitude
    } else {
        magnitude
    };
    let inverse = reciprocal_refined(signed_length);
    perpendicular.map(|v| v * inverse)
}

/// 82D395AC..82D396AC: push away from the obstructing edge and upward.
/// Inputs are Processed1312,1328 and the actual deck position112.
pub fn hang_force(edge_start: [f32; 4], edge_end: [f32; 4], deck: [f32; 4]) -> [f32; 4] {
    let direction = edge_direction(edge_start, edge_end);
    let up = edge_up(direction);
    let side = cross(up, direction);
    let side = if dot(xyz(subtract(deck, edge_start)), xyz(side)) < 0.0 {
        side.map(|v| v * -1.0)
    } else {
        side
    };
    let lift = [0., 160., 0., 0.];
    std::array::from_fn(|i| side[i].mul_add(50.0, lift[i]))
}

/// 82D39950..82D399FC: signed deck-Z cross deck-Y, scaled by the native
/// displacement0.007. The caller already checked halfpipe/contact geometry.
pub fn wheel_catch_displacement(deck_y: [f32; 4], deck_z: [f32; 4]) -> [f32; 4] {
    let signed_z = if deck_z[1] > 0.0 {
        deck_z
    } else {
        deck_z.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000))
    };
    cross(signed_z, deck_y).map(|v| v * f32::from_bits(0x3be5_6042))
}

/// 82D39B04..82D39B8C: only captured X/Z are restored. Current position Y
/// appears in both terms, so no vertical pinning correction is introduced.
pub fn pinning_velocity(position: [f32; 4], captured_x: f32, captured_z: f32, dt: f32) -> [f32; 4] {
    let target = [captured_x, position[1], captured_z, 0.0];
    let inverse_dt = reciprocal_refined(dt);
    subtract(target, position).map(|v| v * inverse_dt)
}

pub fn scale_to_magnitude(vector: [f32; 4], squared: f32, magnitude: f32) -> [f32; 4] {
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    // 82D394D8 divides50 by the computed length with scalar fdivs.
    let factor = magnitude / length;
    vector.map(|v| v * factor)
}

fn reciprocal_refined(value: f32) -> f32 {
    let estimate = reciprocal_estimate(value);
    let first = estimate.mul_add((-estimate).mul_add(value, 1.0), estimate);
    first.mul_add((-first).mul_add(value, 1.0), first)
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn subtract(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrections_preserve_native_axes_and_units() {
        let force = hang_force([0.; 4], [0., 0., 2., 0.], [1., 0., 0., 0.]);
        assert!((force[0] - 50.).abs() < 0.0001);
        assert_eq!(force[1], 160.);
        assert_eq!(edge_up([0., 1., 0., 0.]), [1., 0., 0., 0.]);
        assert_eq!(
            collision_force_projection([0., 0., 4., 0.], [0., 0., -3., 0.]),
            -3.
        );
        assert_eq!(collision_force_projection([0.; 4], [1., 2., 3., 0.]), 0.);
        let pin = pinning_velocity([4., 9., 8., 0.], 3., 10., 1. / 60.);
        assert!((pin[0] + 60.).abs() < 0.00001);
        assert_eq!(pin[1], 0.);
        assert!((pin[2] - 120.).abs() < 0.00002);
        assert!(wheel_catch_displacement([0., 0., 1., 0.], [0., 1., 0., 0.])[0] > 0.);
    }
}

/// ApplyWorldSpaceForceToDeck82C03F98. Its moment arm is measured from the
/// physical deck part origin returned by82585CB0, not the body's COM origin.
pub fn apply_world_force(
    body: &mut crate::physics::assembly::BodySnapshot,
    deck_part_position: Vector3,
    force: Vector3,
    point: Vector3,
) {
    let arm = Vector3::new(
        point.x - deck_part_position.x,
        point.y - deck_part_position.y,
        point.z - deck_part_position.z,
    );
    let torque = cross([arm.x, arm.y, arm.z, 0.], [force.x, force.y, force.z, 0.]);
    let inverse = body.inertia.inverse_mass;
    body.rates.force_acceleration.x += force.x * inverse;
    body.rates.force_acceleration.y += force.y * inverse;
    body.rates.force_acceleration.z += force.z * inverse;
    let columns = body.rates.world_inverse_inertia.columns;
    let angular = |i: usize| {
        columns[2][i].mul_add(
            torque[2],
            columns[1][i].mul_add(torque[1], columns[0][i] * torque[0]),
        )
    };
    body.rates.torque_acceleration.x += angular(0);
    body.rates.torque_acceleration.y += angular(1);
    body.rates.torque_acceleration.z += angular(2);
    body.rates.cool_down = 0;
}
