//! Mode4:82D7C0C0..C7F4. Secondary obstacle direction and primary impulse
//! intentionally have different dataflow, and the source compares signed angles.
use super::{PointGraph, Processed, Settings, Vector, controller, math::*};
use crate::player::wipeout_state::orientation::projected_angle;

pub(super) fn calculate(
    state: &controller::State,
    turn: &PointGraph<8>,
    settings: Settings,
    p: &Processed,
    forward: Vector,
) -> (Vector, Vector) {
    let reference_up = state.frame_output.frame[1]; //Biped144
    // Preserve the explicit zero multiply and fallback to the original reference.
    let blend = madd(reference_up, 1.0, scale(state.motion.frame_0[1], 0.0));
    let up = normalize_or(blend, reference_up);
    let planar = sub(p.velocity_912, scale(up, dot(up, p.velocity_912)));
    let backward = dot(planar, forward);
    let amount = select(-backward, 0.0, backward) - backward;
    let mut base = madd(forward, amount, planar);
    let mut secondary = base;
    let speed_squared = dot(base, base);
    if p.flags_2472 & 0x1000_0000 == 0 {
        let control = [p.raw_x_2692, 0.0, p.raw_z_2688, 0.0];
        if !(speed_squared >= 1.0) {
            base = super::super::contact_correction::clamp_length(madd(control, 1.0, base), 1.0);
            secondary = base;
        } else if state.contact.active && dot(control, control) > f32::from_bits(0x3f4f_5c28) {
            let angle = wrap_angle(projected_angle(control, base, up));
            if 45.0 * RADIANS > angle {
                secondary = scale(normalize(reject(control, up)), magnitude(speed_squared));
            } else if 90.0 * RADIANS > angle {
                let direction = normalize(reject(control, up));
                //C4E4 supplies45.0 exactly, with no radians conversion.
                secondary = scale(limit_angle(direction, base, 45.0), magnitude(speed_squared));
            }
        }
    }
    let launch_speed = magnitude(settings.jump_height * f32::from_bits(0x419c_cccd));
    let vertical = up[1] * launch_speed;
    let vertical = select(-vertical, 0.0, vertical);
    secondary = normalize_or(flatten(secondary), ZERO);
    let requested_turn = (turn.evaluate(state.motion.speed_704) * state.intent.steering) * RADIANS;
    //82D7C6C4 reads Biped+688; +672 is the unrelated edge-position vector.
    let turn_center = state.motion.angular_velocity_688;
    let window = f32::from_bits(0x3fdf_66f3);
    let angle = clamp(
        clamp(requested_turn, turn_center - window, turn_center + window) * 0.4,
        f32::from_bits(0xbe86_0a92),
        f32::from_bits(0x3e86_0a92),
    );
    let impulse = madd(base, settings.jump_speed_scalar, scale(up, vertical));
    (rotate(impulse, up, angle), secondary)
}
