//! GroundAnimation's Toolkit_CalcGroundJump, original TU3 82D93618.
//! Stock graph values and live animation/physical inputs determine launch.
mod math;
mod types;
use crate::{
    air::state::{AirMath, PhysicsAirMath},
    trigonometry::acos,
};
use math::*;
pub use types::{GroundJump, GroundJumpInput, GroundJumpMode, GroundJumpSettings};

pub fn calculate(
    input: GroundJumpInput,
    mode: GroundJumpMode,
    s: &GroundJumpSettings,
) -> GroundJump {
    // TU3 82D93618 launches only with the jump flag (Processed+2468 bit 22). It has no hippy
    // branch: a hippy jump's strength comes from Skeleton::ProcessAnimAttributes (+2624) and the
    // board keeps the ordinary jump-mode heights.
    if input.flags_2468 & 0x0040_0000 == 0 {
        return GroundJump::default();
    }
    let normal = input.filtered_ground_normal;
    let y_scalar = s.y_scalar_vs_normal_y.evaluate(normal[1]);
    let current = planar(input.current_velocity, normal);
    let mut prepared = planar(input.prepared_velocity, normal);
    if dot(sub(current, prepared), sub(current, prepared)) > 7.0 {
        let agreement = vector_clamp(dot(normalize(current), normalize(prepared)), 0.0, 1.0);
        let fraction = acos(agreement) * reciprocal(f32::from_bits(0x4049_0fdb));
        let bound = length(prepared) * s.speed_scalar_vs_angle.evaluate(fraction);
        prepared = AirMath.clamp_vector_within_max_length(current, bound);
    }
    let mut velocity = if input.flags_2488 & 0x1000_0000 != 0 {
        let mut value = madd(input.effective_forward, 2.3, prepared);
        value[1] += 1.0;
        value
    } else {
        let minimum = if input.flags_2484 & 0x800 != 0 {
            mode.minimum_height_64
        } else {
            mode.minimum_height_68
        };
        let speed_fraction = clamp(input.surface_speed / s.speed_response_max_speed, 0.0, 1.0);
        let low = s.minimum_height_vs_speed.evaluate(speed_fraction).mul_add(
            minimum - s.absolute_minimum_height,
            s.absolute_minimum_height,
        );
        let high = s.maximum_height_vs_speed.evaluate(speed_fraction).mul_add(
            mode.maximum_height - s.absolute_minimum_height,
            s.absolute_minimum_height,
        );
        let height = low.mul_add(1.0 - input.jump_strength, high * input.jump_strength);
        let current_height = dot(
            sub(
                input.animation_com_position,
                input.ground_reference_position,
            ),
            input.reference_up,
        );
        let remaining = maximum(height - current_height, 0.0);
        let square = (remaining * input.gravity_y) * -2.0;
        let speed = square_root(square);
        let angle = acos(vector_clamp(normal[1], -1.0, 1.0));
        let fraction = clamp(angle * f32::from_bits(0x3f22_f983), 0.0, 1.0);
        let response = maximum(s.vertical_response.evaluate(fraction), s.minimum_scalar);
        madd(normal, speed * response, prepared)
    };
    let bonus = clamp((y_scalar - 1.0) * velocity[1], 0.0, s.maximum_y_bonus);
    velocity[1] += bonus;
    let projected = dot(velocity, normal);
    let side = normalize(cross(UP, input.forward));
    let x = -((input.jump_controls[0] * s.adjust_x_factor) * projected);
    let z = (input.jump_controls[1] * s.adjust_z_factor) * projected;
    let correction = madd(side, x, scale(cross(side, UP), z));
    GroundJump {
        velocity: add(velocity, correction),
        scalar_16: 0.0,
        active: true,
    }
}
