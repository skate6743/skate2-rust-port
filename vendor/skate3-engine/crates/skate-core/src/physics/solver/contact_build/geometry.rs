//! Contact arms and inverse-inertia response, `82AE11BC..82AE1588`.
use super::{ContactPreparation, input::ContactInput};
use crate::math::Vector3;

fn components(v: Vector3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn difference(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn cross(arm: Vector3, axis: Vector3) -> [f32; 3] {
    [
        (-arm.z).mul_add(axis.y, arm.y.mul_add(axis.z, 0.0)),
        (-arm.x).mul_add(axis.z, arm.z.mul_add(axis.x, 0.0)),
        (-arm.y).mul_add(axis.x, arm.x.mul_add(axis.y, 0.0)),
    ]
}

fn inertia_response(full: Vector3, split: Vector3, mass: f32, jacobian: [f32; 3]) -> [f32; 4] {
    let columns = [
        [full.x, full.y, full.z, mass],
        [full.y, split.y, split.z, mass],
        [full.z, split.z, split.x, mass],
    ];
    core::array::from_fn(|component| {
        let first = columns[0][component].mul_add(jacobian[0], 0.0);
        let second = columns[1][component].mul_add(jacobian[1], first);
        columns[2][component].mul_add(jacobian[2], second)
    })
}

fn project(axes: [Vector3; 3], value: Vector3) -> [f32; 3] {
    axes.map(|axis| {
        let x = axis.x.mul_add(value.x, 0.0);
        let xy = axis.y.mul_add(value.y, x);
        axis.z.mul_add(value.z, xy)
    })
}

pub(super) fn prepare(input: ContactInput, time_step: f32) -> ContactPreparation {
    let arms =
        core::array::from_fn(|body| difference(input.positions[body], input.bodies[body].center));
    let active = input.bodies.each_ref().map(|body| body.state & 4 == 4);
    let inverse_mass = core::array::from_fn(|body| {
        if active[body] {
            input.bodies[body].inverse_mass
        } else {
            0.0
        }
    });
    let jacobians: [[[f32; 3]; 3]; 2] = arms.map(|arm| input.axes.map(|axis| cross(arm, axis)));
    let responses: [[[f32; 4]; 3]; 2] = core::array::from_fn(|body| {
        let (full, split) = if active[body] {
            (
                input.bodies[body].inverse_inertia_full,
                input.bodies[body].inverse_inertia_split,
            )
        } else {
            (Vector3::ZERO, Vector3::ZERO)
        };
        jacobians[body].map(|jacobian| inertia_response(full, split, inverse_mass[body], jacobian))
    });
    let effective_mass = core::array::from_fn(|axis| {
        let products: [f32; 3] = core::array::from_fn(|component| {
            let a = jacobians[0][axis][component].mul_add(responses[0][axis][component], 0.0);
            jacobians[1][axis][component].mul_add(responses[1][axis][component], a)
        });
        (products[2] + (products[0] + inverse_mass[0])) + (products[1] + inverse_mass[1])
    });
    let point_acceleration = core::array::from_fn(|body| {
        if !active[body] {
            return Vector3::ZERO;
        }
        let arm = arms[body];
        let force = input.bodies[body].force_acceleration;
        let torque = input.bodies[body].torque_acceleration;
        Vector3::new(
            arm.z.mul_add(torque.y, (-arm.y).mul_add(torque.z, force.x)),
            arm.x.mul_add(torque.z, (-arm.z).mul_add(torque.x, force.y)),
            arm.y.mul_add(torque.x, (-arm.x).mul_add(torque.y, force.z)),
        )
    });
    let separation = difference(input.positions[1], input.positions[0]);
    let relative_acceleration = difference(point_acceleration[1], point_acceleration[0]);
    let scaled_velocity = components(input.relative_velocity).map(|v| time_step.mul_add(v, 0.0));
    let acceleration =
        components(relative_acceleration).map(|v| v.mul_add(time_step * time_step, 0.0));
    let predicted = core::array::from_fn::<_, 3, _>(|i| {
        scaled_velocity[i] + (components(separation)[i] + acceleration[i])
    });
    let restitution = scaled_velocity.map(|v| (-v).mul_add(input.restitution, 0.0));
    ContactPreparation {
        arms,
        axes: input.axes,
        active,
        inverse_mass,
        point_acceleration,
        angular_response_a: responses[0],
        angular_response_b: responses[1],
        effective_mass,
        separation_projection: project(input.axes, separation),
        restitution_projection: project(
            input.axes,
            Vector3::new(restitution[0], restitution[1], restitution[2]),
        ),
        predicted_separation_projection: project(
            input.axes,
            Vector3::new(predicted[0], predicted[1], predicted[2]),
        ),
        reaction_ids: input.bodies.each_ref().map(|body| body.reaction_id),
        body_ids: input.body_ids,
        static_friction_bits: input.static_friction_bits,
        dynamic_friction_bits: input.dynamic_friction_bits,
        contact_tag: input.contact_tag,
        combined_state_bit_8: (input.bodies[0].state | input.bodies[1].state) & 8,
    }
}
