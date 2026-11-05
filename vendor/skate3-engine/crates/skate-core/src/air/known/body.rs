//! Body flip and spin targets selected by TU3 KnownAir.

use super::{
    data::{
        KnownAirFrame, KnownAirModeSettings, KnownAirReckoningFields, KnownAirSettings,
        KnownAirState, Vector4,
    },
    runtime::KnownAirRuntime,
    trajectory::select,
};

const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3C8E_FA35); // 0x8206D110.
const RADIANS_TO_DEGREES: f32 = f32::from_bits(0x4265_2EE1); // 0x82084620.
const DIRECTION_EPSILON: f32 = f32::from_bits(0x3A83_126F); // 0x82063A48.
const PI: f32 = f32::from_bits(0x4049_0FDB); // 0x82060C44.
const ALIGNMENT_DELAY: f32 = f32::from_bits(0x3D03_126F); // 0x822F91E8.
const AUTO_SPIN_SCALE: f32 = f32::from_bits(0x3F99_999A); // 0x821659F4.
const FORTY_FIVE_DEGREES: f32 = f32::from_bits(0x4234_0000); // 0x82099018.
const HALF: f32 = f32::from_bits(0x3F00_0000); // 0x8209975C.
const REVERSE_SCALE: f32 = f32::from_bits(0x3BA3_D70A); // 0x82099920.
const DECELERATION_SCALE: f32 = f32::from_bits(0x3F40_0000); // 0x821814A0.

/// `PhysState_KnownAir::CalculateBodyFlipSpeed`, TU3 `0x82D35F40`.
pub fn calculate_body_flip_speed(
    state: &mut KnownAirState,
    frame: &KnownAirFrame,
    settings: &KnownAirSettings,
    mode: &KnownAirModeSettings,
    reckoning: &KnownAirReckoningFields,
    runtime: &mut impl KnownAirRuntime,
) -> f32 {
    if !state.body_flipping_211 {
        let flags = frame.flags_2468;
        let has_flip = flags & 0x0000_0020 != 0 || flags & 0x0000_0010 != 0;
        let has_grab = flags & 0x0000_0080 != 0 || flags & 0x0000_0040 != 0;
        if has_flip && has_grab && frame.flags_2488 & 0x0200_0000 == 0 {
            let threshold = settings
                .flip_start_collision_time_vs_normal_y
                .evaluate(reckoning.landing_normal_1152[1]);
            if state.collision_time_196 > threshold {
                runtime.begin_reckoning_body_flip((flags >> 7) & 1 != 0);
                state.body_flipping_211 = true;
            }
        }
    }

    if mode.perfect_body_flips_28 {
        return state.body_flip_target_speed_204;
    }
    let flags = frame.flags_2468;
    if flags & 0x0000_0020 != 0
        || flags & 0x0000_0010 != 0
        || state.time_in_state_180 < settings.body_flip_min_grab_time_fraction_456
    {
        state.body_flip_target_speed_204
    } else {
        0.0
    }
}

/// `PhysState_KnownAir::CalculateBodySpinSpeed`, TU3 `0x82D360B0`.
pub fn calculate_body_spin_speed(
    state: &KnownAirState,
    frame: &KnownAirFrame,
    settings: &KnownAirSettings,
    mode: &KnownAirModeSettings,
    reckoning: &KnownAirReckoningFields,
    runtime: &mut impl KnownAirRuntime,
) -> f32 {
    let scaled_input = frame.body_spin_input_2640 * settings.max_spin_speed_428;
    let manual_spin = scaled_input * DEGREES_TO_RADIANS;
    let mut result = if frame.flags_2472 & 0x1000_0000 != 0 {
        manual_spin
    } else {
        0.0
    };

    // Native tests the unmasked manual value, not `result`.
    if manual_spin == 0.0 && state.landing_heading_valid_209 && frame.flags_2484 & 0x0000_8000 == 0
    {
        let mut heading_axis = reckoning.heading_axis_1200;
        if ((frame.flags_2468 >> 20) & 1 != 0) != state.start_flipped_210 {
            heading_axis = flip_sign_bits(heading_axis);
        }

        let mut unwrapped_angle = 0.0;
        if runtime.dot3(reckoning.landing_normal_1152, state.landing_normal_64) > DIRECTION_EPSILON
        {
            unwrapped_angle = runtime.signed_angle_about_axis(
                heading_axis,
                state.landing_heading_80,
                reckoning.landing_normal_1152,
            );
            if frame.flags_2468 & 0x0010_0000 != 0 {
                unwrapped_angle += PI;
            }
        }

        let mut corrected_angle = runtime.wrap_angle_8258db98(unwrapped_angle);
        let heading_degrees = corrected_angle * RADIANS_TO_DEGREES;
        let max_adjust = settings
            .max_heading_adjust_vs_up_y_160
            .evaluate(frame.skater_up_544[1]);
        if heading_degrees.abs() > max_adjust {
            let wrapped_pi = runtime.wrap_angle_8258db98(PI);
            corrected_angle = runtime.wrap_angle_8258db98(wrapped_pi + corrected_angle);
        }

        let mut current_speed = reckoning.body_spin_speed_1572;
        let remaining = state.collision_time_196 - state.time_in_state_180;
        let alignment_time = select(remaining - ALIGNMENT_DELAY, remaining, ALIGNMENT_DELAY);
        let reciprocal_time = 1.0 / alignment_time;
        let mut target = (reciprocal_time * corrected_angle) * AUTO_SPIN_SCALE;

        if current_speed.abs() < DIRECTION_EPSILON {
            current_speed = 0.0;
        }
        if current_speed * target < 0.0 {
            let alternate = runtime.wrap_angle_8258db98(unwrapped_angle + PI);
            if FORTY_FIVE_DEGREES * DEGREES_TO_RADIANS > alternate.abs()
                && (reciprocal_time * alternate) * current_speed > 0.0
            {
                target = reciprocal_time * alternate;
            }
        }

        result = native_clamp(
            target,
            -mode.body_spin_speed_limit_56,
            mode.body_spin_speed_limit_56,
        );
        if result.abs() < settings.min_auto_body_speed_424 {
            result *= HALF;
        }
        if unwrapped_angle.abs() < settings.min_auto_body_speed_424 {
            result = (unwrapped_angle / frame.delta_time_2604) * HALF;
        }
        if result * current_speed < 0.0 {
            result *= REVERSE_SCALE;
        } else if current_speed.abs() > result.abs() {
            result *= DECELERATION_SCALE;
        }
    }

    result
}

fn flip_sign_bits(vector: Vector4) -> Vector4 {
    vector.map(|lane| f32::from_bits(lane.to_bits() ^ 0x8000_0000))
}

fn native_clamp(value: f32, lower: f32, upper: f32) -> f32 {
    let value = select(lower - value, lower, value);
    select(upper - value, value, upper)
}
