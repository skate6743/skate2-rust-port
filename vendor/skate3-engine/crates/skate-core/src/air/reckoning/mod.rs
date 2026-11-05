//! Complete original TU3 Reckoning::UpdateAirStates82D8DBD8.
//! Shared Ground filters, frame owners and BodySpin history persist across states.
mod data;
mod math;
use crate::{
    air::{
        body_flip::{self, BodyFlipInput, BodyFlipState},
        body_spin::{self, BodySpinState},
    },
    math::Vector3,
    riding::{ground_orientation::GroundOrientation, reckoning_frames::ReckoningFrames},
};
pub use data::{AirState, Input, Settings};
pub use math::limit_angle as clamp_vector_within_max_angle;
use math::{angle, length, limit_angle, normalize, normalize_safe, rotate_heading, wrap};

pub fn update(
    orientation: &mut GroundOrientation,
    frames: &mut ReckoningFrames,
    body_spin: &mut BodySpinState,
    state: &mut AirState,
    settings: &Settings,
    input: &Input,
) {
    let old_up = lanes(orientation.up);
    orientation.dynamic_up = orientation.up;
    frames.target_lean_angle = 0.0;
    state.secondary_lean_angle = 0.0;
    let ground_normal = orientation
        .ground_filter
        .update(settings.ground_normal_smoothing, input.landing_normal);
    orientation.ground_normal = vector(ground_normal);

    state.spin_speed = if input.additive_spin {
        input.grind_adjusted_body_spin + input.target_spin
    } else if input.direct_spin {
        input.target_spin
    } else {
        body_spin::update(
            body_spin,
            &settings.body_spin,
            input.physical_body_spin,
            input.target_spin,
            true,
            u8::from(input.easy_body_spins),
        );
        body_spin.speed()
    };
    state.spin_angle = state
        .spin_speed
        .mul_add(f32::from_bits(0x3c88_8889), state.spin_angle);
    if state.flip_active {
        //A short call-boundary view, not a second retained owner of matrix1008.
        let mut flip = BodyFlipState {
            angle: state.flip_angle,
            speed: state.flip_speed,
            requested_speed: state.flip_requested_speed,
            spin_transform: state.spin_transform,
            combined_transform: frames.body_flip,
        };
        body_flip::update(
            &mut flip,
            &settings.body_flip,
            &BodyFlipInput {
                requested_speed: input.flip_request,
                spin_angle: state.spin_angle,
                normal: old_up,
                flip_axis: state.flip_axis,
                timestep: input.timestep,
                perfect_body_flips: input.perfect_body_flips,
            },
        );
        state.flip_angle = flip.angle;
        state.flip_speed = flip.speed;
        state.flip_requested_speed = flip.requested_speed;
        state.spin_transform = flip.spin_transform;
        frames.body_flip = flip.combined_transform;
    }

    let wrapped = wrap(angle(old_up, ground_normal));
    let maximum = wrapped * input.normal_blend;
    let target = if wrapped.abs() >= f32::from_bits(0x40c8_f61e) {
        if old_up[1].abs() < f32::from_bits(0x3f7d_70a4) {
            [0.0, 1.0, 0.0, 0.0]
        } else {
            [1.0, 0.0, 0.0, 0.0]
        }
    } else {
        ground_normal
    };
    let candidate = normalize(limit_angle(target, old_up, maximum));
    let max_delta = settings
        .max_up_angle_delta
        .evaluate(length(input.com_to_deck));
    let selected = if wrap(angle(candidate, old_up)).abs() > max_delta {
        limit_angle(candidate, old_up, max_delta)
    } else {
        candidate
    };
    let up = normalize_safe(selected, old_up);
    orientation.up_velocity = vector(std::array::from_fn(|i| up[i] - old_up[i]));
    orientation.up = vector(up);
    orientation.target = orientation.up;

    if !state.flip_active {
        frames.heading = rotate_heading(frames.heading, up, input.timestep * state.spin_speed);
    }
    //82D8C868 advances histories using retained control words, then publishes
    //the chosen up into each current-position slot; no filter reinitialization.
    orientation.slow_filter.filter(up);
    orientation.fast_filter.filter(up);
    orientation.slow_filter.publish_current(up);
    orientation.fast_filter.publish_current(up);
    frames.calculate_transform(up, ground_normal);
    frames.calculate_tilt(
        input.reverse_stance,
        &settings.tilt_vs_rotation,
        &settings.tilt_vs_slope,
    );
}
fn lanes(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
fn vector(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

///Handplant Reckoning82D8E1C0. Uses the ordinary persistent normal filters.
pub fn update_plant(
    orientation: &mut GroundOrientation,
    frames: &mut ReckoningFrames,
    body_spin: &mut BodySpinState,
    state: &mut AirState,
    settings: &Settings,
    up: [f32; 4],
    heading: [f32; 4],
    reverse: bool,
) {
    frames.heading = heading;
    body_spin::update(body_spin, &settings.body_spin, 0.0, 0.0, true, 0);
    let previous = lanes(orientation.up);
    orientation.dynamic_up = orientation.up;
    frames.target_lean_angle = 0.0;
    state.secondary_lean_angle = 0.0;
    let normal = normalize(
        orientation
            .ground_filter
            .update(settings.ground_normal_smoothing, up),
    );
    orientation.ground_normal = vector(normal);
    let up = normalize_safe(limit_angle(normal, previous, 1.0), previous);
    orientation.up_velocity = vector(std::array::from_fn(|i| up[i] - previous[i]));
    orientation.up = vector(up);
    orientation.target = orientation.up;
    orientation.slow_filter.filter(up);
    orientation.fast_filter.filter(up);
    orientation.slow_filter.publish_current(up);
    orientation.fast_filter.publish_current(up);
    frames.calculate_transform(up, normal);
    frames.calculate_tilt(reverse, &settings.tilt_vs_rotation, &settings.tilt_vs_slope);
}

#[cfg(test)]
mod tests;
