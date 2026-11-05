//! Semantic builder outputs → native compiled-record layout.
use super::{packed, *};
use crate::math::Vector3;
use crate::physics::rigid_body::RetailPackedWorldInverseInertia;

pub(super) fn contact(c: &RetailContactJacobian) -> packed::Contact {
    packed::Contact {
        words: *c.words(),
        reaction_a: c.reaction_index_a,
        reaction_b: c.reaction_index_b,
    }
}

pub(super) fn drive(d: &RetailDriveRows) -> packed::Drive {
    let mut words = [0; 96];
    vector(&mut words, 0, d.arm_a, 0.0);
    vector(&mut words, 1, d.arm_b, 0.0);
    row(
        &mut words,
        2,
        d.accumulated_linear_impulse,
        d.linear_softness,
    );
    row(
        &mut words,
        3,
        d.accumulated_angular_impulse,
        d.angular_softness,
    );
    for component in 0..3 {
        row(
            &mut words,
            4 + component * 2,
            core::array::from_fn(|r| {
                xyz(d.linear_axes[r])[component] * d.linear_inverse_effective_mass[r]
            }),
            0.0,
        );
        row(
            &mut words,
            5 + component * 2,
            core::array::from_fn(|r| {
                xyz(d.angular_axes[r])[component] * d.angular_inverse_effective_mass[r]
            }),
            0.0,
        );
        vector(&mut words, 12 + component, d.linear_axes[component], 0.0);
        vector(&mut words, 15 + component, d.angular_axes[component], 0.0);
    }
    row(
        &mut words,
        10,
        d.linear_target_impulse,
        d.linear_maximum_impulse[2],
    );
    row(
        &mut words,
        11,
        d.angular_target_impulse,
        d.angular_maximum_impulse[2],
    );
    words[6 * 4 + 3] = d.linear_maximum_impulse[0].to_bits();
    words[7 * 4 + 3] = d.linear_maximum_impulse[1].to_bits();
    words[8 * 4 + 3] = d.angular_maximum_impulse[0].to_bits();
    words[9 * 4 + 3] = d.angular_maximum_impulse[1].to_bits();
    for (index, body) in [(18, d.frame_a_body), (21, d.frame_b_body)] {
        if body.state & 4 != 0 {
            let columns = inertia_columns(body.world_inverse_inertia);
            for (i, column) in columns.into_iter().enumerate() {
                vector(
                    &mut words,
                    index + i,
                    column,
                    if i == 0 { body.inverse_mass } else { 0.0 },
                );
            }
        }
    }
    packed::Drive {
        words,
        reaction_a: d.frame_a_body.reaction_index,
        reaction_b: d.frame_b_body.reaction_index,
    }
}

pub(super) fn reaction(r: RetailReactionCorrections) -> packed::Reaction {
    let mut words = [0; 16];
    for (i, v) in [
        r.linear_displacement,
        r.position_displacement,
        r.angular_displacement,
        r.orientation_displacement,
    ]
    .into_iter()
    .enumerate()
    {
        vector(&mut words, i, v, 0.0);
    }
    words
}
pub(super) fn unpack_reaction(words: packed::Reaction) -> RetailReactionCorrections {
    let v = |i| {
        let v = read_xyz(&words, i);
        Vector3::new(v[0], v[1], v[2])
    };
    RetailReactionCorrections {
        linear_displacement: v(0),
        position_displacement: v(1),
        angular_displacement: v(2),
        orientation_displacement: v(3),
    }
}
pub(super) fn read_xyz(words: &[u32], i: usize) -> [f32; 3] {
    core::array::from_fn(|j| f32::from_bits(words[4 * i + j]))
}
fn row(words: &mut [u32], i: usize, v: [f32; 3], w: f32) {
    words[4 * i..4 * i + 4].copy_from_slice(&[
        v[0].to_bits(),
        v[1].to_bits(),
        v[2].to_bits(),
        w.to_bits(),
    ]);
}
fn vector(words: &mut [u32], i: usize, v: Vector3, w: f32) {
    row(words, i, xyz(v), w);
}
fn xyz(v: Vector3) -> [f32; 3] {
    [v.x, v.y, v.z]
}
fn inertia_columns(i: RetailPackedWorldInverseInertia) -> [Vector3; 3] {
    [
        Vector3::new(i.full.x, i.full.y, i.full.z),
        Vector3::new(i.full.y, i.split.y, i.split.z),
        Vector3::new(i.full.z, i.split.z, i.split.x),
    ]
}
