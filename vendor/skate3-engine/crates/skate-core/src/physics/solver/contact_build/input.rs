//! Decodes the uncompiled contact copied by the native contact producer.
use crate::math::Vector3;

pub(super) struct BodyInput {
    pub center: Vector3,
    pub inverse_inertia_full: Vector3,
    pub inverse_inertia_split: Vector3,
    pub inverse_mass: f32,
    pub state: u32,
    pub force_acceleration: Vector3,
    pub torque_acceleration: Vector3,
    pub reaction_id: u32,
}

pub(super) struct ContactInput {
    pub positions: [Vector3; 2],
    pub axes: [Vector3; 3],
    pub relative_velocity: Vector3,
    pub restitution: f32,
    pub bodies: [BodyInput; 2],
    pub body_ids: [u32; 2],
    pub static_friction_bits: u32,
    pub dynamic_friction_bits: u32,
    pub contact_tag: u32,
}

fn vector(words: &[u32; 64], row: usize) -> Vector3 {
    Vector3::new(
        f32::from_bits(words[row * 4]),
        f32::from_bits(words[row * 4 + 1]),
        f32::from_bits(words[row * 4 + 2]),
    )
}

impl ContactInput {
    pub fn decode(words: &[u32; 64]) -> Self {
        Self {
            positions: [vector(words, 0), vector(words, 1)],
            axes: core::array::from_fn(|axis| vector(words, 2 + axis)),
            relative_velocity: vector(words, 5),
            restitution: f32::from_bits(words[11]),
            bodies: core::array::from_fn(|body| BodyInput {
                center: vector(words, 6 + body),
                inverse_inertia_full: vector(words, 8 + body),
                inverse_inertia_split: vector(words, 10 + body),
                inverse_mass: f32::from_bits(words[(8 + body) * 4 + 3]),
                state: words[(10 + body) * 4 + 3],
                force_acceleration: vector(words, 12 + body),
                torque_acceleration: vector(words, 14 + body),
                reaction_id: words[(6 + body) * 4 + 3],
            }),
            body_ids: [words[3], words[7]],
            static_friction_bits: words[15],
            dynamic_friction_bits: words[19],
            contact_tag: words[23],
        }
    }
}
