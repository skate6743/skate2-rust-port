//! Original TU3 Biped movement82D7EDC8 and leaves800C0/80EB0/80CE0.
//! S3 SHA256431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
//! The controller owns these fields once; this stage makes no collision queries.

mod approach;
mod math;
mod publication;
mod support;
#[cfg(test)]
mod tests;

pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];
use math::*;

/// Only fields written by this stage. Integrate these into the one Biped owner;
/// velocity480, correction576/711 and frame0 must not become stale shadow copies.
#[derive(Clone, Debug)]
pub struct GroundMotionState {
    pub frame_0: Frame,
    pub published_frame_64: Frame,
    pub previous_support_frame_192: Frame,
    pub support_velocity_256: Vector,
    pub predicted_support_velocity_272: Vector,
    pub previous_support_velocity_288: Vector,
    pub support_acceleration_304: Vector,
    pub filtered_local_acceleration_320: Vector,
    pub support_yaw_336: f32,
    pub previous_support_yaw_340: f32,
    pub predicted_support_yaw_344: f32,
    pub support_speed_348: f32,
    pub support_id_352: u32,
    pub velocity_480: Vector,
    pub correction_576: Vector,
    pub target_frame_608: Frame,
    pub angular_velocity_688: f32,
    pub speed_704: f32,
    pub correction_enabled_711: bool,
    pub support_velocity_removed_715: bool,
    pub target_scale_784: f32,
}

/// Actual job values and external-controller fields, never query placeholders.
#[derive(Clone, Copy, Debug)]
pub struct GroundMotionInput {
    pub contact_position_0: Vector,
    pub contact_frame_32: Frame,
    pub contact_target_96: Vector,
    pub contact_normal_112: Vector,
    pub contact_flags_176: u32,
    pub contact_id_180: u32,
    pub animation_motion_224: Vector,
    pub animation_motion_240: Vector,
    pub requested_duration_288: f32,
    pub mirrored_292: bool,
    pub animation_directed_304: bool,
    pub target_frame_present_352: bool,
    pub target_frame_368: Frame,
    /// Externally produced Biped frame128..176; read by800C0 and angle limiting.
    pub reference_frame_128: Frame,
    /// Contact correction producer384 (not the job's frame384).
    pub contact_displacement_384: Vector,
    pub velocity_addition_528: Vector,
    pub desired_up_544: Vector,
    pub correction_target_592: Vector,
    pub obstacle_target_672: Vector,
    pub obstacle_enabled_713: bool,
}

///82D7EDC8. Order: support history, approach, displacement/heading, publication.
pub fn update(state: &mut GroundMotionState, input: &GroundMotionInput) {
    let support_delta = support::update(state, input);
    let old_position = state.frame_0[3];
    state.frame_0[3] = ZERO;
    let (step, height) = approach::calculate(state, input, old_position, support_delta);
    let displacement = add(ZERO, step);
    let displacement = add(displacement, height);
    let displacement = add(displacement, input.contact_displacement_384);
    let displacement = madd(input.velocity_addition_528, DT, displacement);
    let displacement = madd(state.predicted_support_velocity_272, DT, displacement);
    let angle = (state.angular_velocity_688 + state.predicted_support_yaw_344) * DT;
    let axis = state.frame_0[1];
    state.frame_0 = state.frame_0.map(|row| rotate(row, axis, angle));
    state.frame_0[0] = normalize(cross(input.desired_up_544, state.frame_0[2]));
    state.frame_0[1] = input.desired_up_544;
    state.frame_0[2] = normalize(cross(state.frame_0[0], input.desired_up_544));
    state.frame_0 = crate::physics::skeleton_root::orthonormalize(state.frame_0);
    state.frame_0[3] = add(old_position, displacement);
    publication::update(state, displacement);
}
