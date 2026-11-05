//! LaunchSkateboard82D73AB0; S2 named counterpart82DCB3A8.
//! S3 replaces tunables with literals; energy cost is0.6, not ZIP's0.2.
use super::{V, length, reciprocal, scale, sub};
use crate::physics::{grind_contact::manager::Jumper, native_arithmetic::dot3};

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub velocity_400: V,
    pub position_112: V,
    /// Low-energy zero-balance side selection uses CURRENT1120, not cache96.
    pub current_point_1120: V,
    pub balance_2800: f32,
    pub geometry_side_jump: f32,
    pub vertical_jump: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub velocity: V,
    /// Complete retained cache, to replace the SAME owner updated by82D739D8.
    pub jumper: Jumper,
    pub flags_2476_to_or: u32,
}

pub fn launch(mut jumper: Jumper, input: Input) -> Output {
    let geometry = jumper.geometry;
    let mut normal = geometry.normal;
    let candidate = if matches!(jumper.family, 2 | 4) {
        let across = cross(normal, geometry.direction);
        let outward = if dot3(across, sub(input.position_112, geometry.point)) > 0. {
            across
        } else {
            scale(across, -1.)
        };
        let amount = if jumper.family == 4 { -0.1093 } else { 0.291 };
        core::array::from_fn(|i| normal[i] + outward[i] * amount)
    } else {
        core::array::from_fn(|i| geometry.upmost[i].mul_add(0.5, normal[i] * 0.5))
    };
    let magnitude = length(candidate);
    if magnitude > 0.1 {
        normal = scale(candidate, reciprocal(magnitude));
    }
    let planar = sub(
        input.velocity_400,
        scale(normal, dot3(input.velocity_400, normal)),
    );
    let vertical = input.vertical_jump * 1.03;
    let base: V = core::array::from_fn(|i| normal[i].mul_add(vertical, planar[i]));
    let across = cross(geometry.direction, normal);
    let mut balance = input.balance_2800;
    let low_energy = jumper.energy < 0.31;
    if low_energy {
        if balance < 0. {
            if balance - -0.42 >= 0. {
                balance = -0.42;
            }
        } else if balance > 0. {
            if !(balance - 0.42 >= 0.) {
                balance = 0.42;
            }
        } else {
            balance = if dot3(sub(input.position_112, input.current_point_1120), across) > 0. {
                0.42
            } else {
                -0.42
            };
        }
        jumper.cooldown = 10;
    }
    let side = if geometry.geometry_kind != 0 {
        input.geometry_side_jump
    } else {
        0.
    };
    let mut lateral = sub(
        scale(across, balance * 1.8),
        scale(geometry.high_side, side),
    );
    let magnitude = length(lateral);
    if magnitude > 1.8 {
        lateral = scale(lateral, 1.8 / magnitude);
    }
    let velocity = core::array::from_fn(|i| base[i] + lateral[i]);
    jumper.launched = true;
    let energy = jumper.energy - 0.6;
    jumper.energy = if -energy >= 0. { 0. } else { energy };
    Output {
        velocity,
        jumper,
        flags_2476_to_or: if low_energy { 0x0200_0000 } else { 0 },
    }
}

fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
