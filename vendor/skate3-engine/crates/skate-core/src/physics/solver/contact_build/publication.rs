//! Scaled Jacobian and restitution target publication, `82AE1590..82AE15F4`.
use super::ContactPreparation;
use crate::math::Vector3;

fn vector_row(words: &mut [u32; 64], row: usize, xyz: Vector3, carry: u32) {
    words[row * 4..row * 4 + 4].copy_from_slice(&[
        xyz.x.to_bits(),
        xyz.y.to_bits(),
        xyz.z.to_bits(),
        carry,
    ]);
}

fn float_row(words: &mut [u32; 64], row: usize, values: [f32; 4]) {
    words[row * 4..row * 4 + 4].copy_from_slice(&values.map(f32::to_bits));
}

pub(super) fn compile(prepared: &ContactPreparation, inverse_response: [f32; 3]) -> [u32; 64] {
    let mut words = [0; 64];
    for body in 0..2 {
        vector_row(
            &mut words,
            body,
            prepared.arms[body],
            prepared.reaction_ids[body],
        );
    }
    let axes = prepared.axes.map(|axis| [axis.x, axis.y, axis.z]);
    let carry = [
        prepared.combined_state_bit_8,
        prepared.static_friction_bits,
        prepared.dynamic_friction_bits,
    ];
    for component in 0..3 {
        let column = core::array::from_fn::<_, 3, _>(|axis| {
            axes[axis][component].mul_add(inverse_response[axis], 0.0)
        });
        vector_row(
            &mut words,
            2 + component,
            Vector3::new(column[0], column[1], column[2]),
            carry[component],
        );
    }
    // Row 5 is the newly cleared accumulated impulse, including its carry.
    let separation = prepared.separation_projection[0].mul_add(inverse_response[0], 0.0);
    let restitution = prepared.restitution_projection[0].mul_add(inverse_response[0], 0.0);
    let predicted = core::array::from_fn::<_, 3, _>(|axis| {
        prepared.predicted_separation_projection[axis].mul_add(inverse_response[axis], 0.0)
    });
    let mut normal_adjustment = if restitution >= 0.0 {
        0.0
    } else {
        separation + restitution
    };
    if separation >= 0.0 {
        normal_adjustment = restitution;
    }
    if 0.0 >= predicted[0] {
        normal_adjustment = 0.0;
    }
    float_row(
        &mut words,
        6,
        [
            predicted[0] - normal_adjustment,
            predicted[1] - 0.0,
            predicted[2] - 0.0,
            separation,
        ],
    );
    let identifiers = [
        prepared.body_ids[0],
        prepared.body_ids[1],
        prepared.contact_tag,
    ];
    for axis in 0..3 {
        vector_row(
            &mut words,
            7 + axis * 3,
            prepared.axes[axis],
            identifiers[axis],
        );
        let mut a = prepared.angular_response_a[axis];
        let mut b = prepared.angular_response_b[axis];
        if axis == 0 {
            a[3] = prepared.inverse_mass[0];
            b[3] = prepared.inverse_mass[1];
        }
        float_row(&mut words, 8 + axis * 3, a);
        float_row(&mut words, 9 + axis * 3, b);
    }
    words
}
