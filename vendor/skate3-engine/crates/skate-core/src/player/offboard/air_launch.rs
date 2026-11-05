//! Original Biped launch82D7BA78. This is separate from onboard LaunchInfo.
//! Produces the full packet before Sync replaces position32 with its body point.
use super::controller;
use crate::point_graph::PointGraph;
mod jump;
pub(super) mod math;
mod packet;
use math::*;
pub use packet::Packet;
#[cfg(test)]
mod tests;
pub type Vector = [f32; 4];

/// Actual physics_biped scalar fields, validated by the stock loader.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Biped796 <- layout896 (0x380), JumpSpeedScalar.
    pub jump_speed_scalar: f32,
    /// Biped792 <- layout900 (0x384), JumpHeight.
    pub jump_height: f32,
}
/// A per-call snapshot from the canonical processed input; no persistent copy.
///Source1120 is a point and1136 is the axis used by mode1's rejection.
///Absence means the host has no source-backed departure geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepartureGeometry {
    pub point_1120: Vector,
    pub axis_1136: Vector,
}
#[derive(Clone, Copy, Debug)]
pub struct Processed {
    pub board_position_112: Vector,
    pub forward_224: Vector,
    pub up_544: Vector,
    pub position_592: Vector,
    pub velocity_608: Vector,
    pub velocity_912: Vector,
    pub departure_geometry: Option<DepartureGeometry>,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub previous_state_2504: u32,
    pub current_state_2508: u32,
    pub current_category_2512: u32,
    pub previous_category_2516: u32,
    pub raw_x_2692: f32,
    pub raw_z_2688: f32,
}
///82D7BA90..BB38, including the current/previous selector used by its callers.
pub fn mode(p: &Processed, current: bool) -> u8 {
    let (state, category) = if current {
        (p.current_state_2508, p.current_category_2512)
    } else {
        (p.previous_state_2504, p.previous_category_2516)
    };
    match category {
        100 => 0,
        400 => 1,
        200 => 2,
        500 if p.flags_2480 & 0x80 != 0 || state == 503 => 3,
        500 if p.flags_2476 & 0x80000 != 0 => 4,
        500 => 5,
        _ => 6,
    }
}
/// Borrow the existing controller state and its movement_velocity.turn_vs_speed.
/// Fields80/116/117 retain the caller's packet unless the original mode writes
/// them. Common scalars, both velocities, position, up and forward always write.
pub fn produce(
    packet: &mut Packet,
    state: &controller::State,
    turn_vs_speed: &PointGraph<8>,
    settings: Settings,
    p: &Processed,
    current: bool,
) -> Result<(), &'static str> {
    let selected_mode = mode(p, current);
    //Validate the host contract before ANY packet field changes. No numerical
    //fallback or inferred geometry is substituted for an unavailable producer.
    let departure =
        if selected_mode == 1 {
            Some(p.departure_geometry.ok_or(
                "Offboard launch mode1 requires source-backed departure geometry1120/1136",
            )?)
        } else {
            None
        };
    let forward = normalize_or(flatten(p.forward_224), ZERO);
    packet.up_48 = p.up_544;
    packet.forward_64 = forward;
    packet.scalar_96 = f32::from_bits(0x3db2_b8c2);
    packet.scalar_100 = f32::from_bits(0x3f5f_66f3);
    packet.scalar_104 = f32::from_bits(0x3f32_b8c2);
    packet.kind_108 = 1;
    packet.kind_112 = 0;
    packet.position_32 = p.position_592;
    let mut velocity = p.velocity_608;
    let mut secondary = velocity;
    match selected_mode {
        0 => {
            //BC84..BCC0 is a boost, not a conventional [-3,3] clamp.
            let lower = select(velocity[1] - 3.0, velocity[1], 3.0);
            velocity[1] = select((velocity[1] + 3.0) - lower, lower, velocity[1] + 3.0);
            packet.board_position_80 = p.board_position_112;
            packet.has_board_position_116 = true;
            packet.position_32 = madd(velocity, DT, packet.position_32);
        }
        1 => {
            let departure = departure.expect("Mode1 geometry validated before packet writes");
            let delta = sub(p.position_592, departure.point_1120);
            let projected = sub(
                delta,
                scale(departure.axis_1136, dot(delta, departure.axis_1136)),
            );
            let normal = normalize_or(flatten(projected), ZERO);
            velocity = add(madd(normal, 2.0, velocity), UP);
            packet.position_32 = madd(velocity, DT, packet.position_32);
            packet.kind_108 = 6;
        }
        3 => {
            let side = cross(UP, velocity);
            let speed = length(side);
            if !(speed <= f32::from_bits(0x3a83_126f)) {
                let offset = if p.flags_2476 & 4 != 0 { 0.75 } else { -0.75 };
                velocity = madd(side, offset / speed, velocity);
            }
        }
        4 => {
            (velocity, secondary) = jump::calculate(state, turn_vs_speed, settings, p, forward);
            packet.kind_108 = 6;
            packet.kind_112 = 3;
        }
        5 => {
            let horizontal = flatten(velocity);
            if !(length(horizontal) >= 1.875) {
                velocity = scale(normalize_or(horizontal, forward), 2.5);
                velocity[1] = select(1.0 - velocity[1], 1.0, velocity[1]);
                packet.scalar_96 = f32::from_bits(0x3f06_0a92);
            } else {
                velocity = scale(p.velocity_912, 0.75);
                velocity[1] = clamp(velocity[1], -10.0, 5.0);
            }
            packet.kind_108 = 6;
        }
        2 | 6 => {}
        _ => unreachable!("mode returns only the seven native cases"),
    }
    packet.velocity_0 = velocity;
    packet.secondary_velocity_16 = secondary;
    Ok(())
}
