//! CalcManualEffect 82C047F0 control/scalar response and output construction.
//! The native angle-between-vectors measurement remains a required dependency.
use super::{
    angle,
    settings::{ManualMode, ManualSettings},
    state::ManualState,
};

#[derive(Clone, Copy, Debug)]
pub struct ManualInput {
    /// ProcessedPhysIn+2720 and +2620.
    pub balance: f32,
    pub flipped_controls: f32,
    /// +2664, +2616 and +2768. Noise time is independent of manual elapsed.
    pub procedural_noise_time: f32,
    pub absolute_speed: f32,
    pub animation_noise: f32,
    /// +2604; this controller does not replace it with a fixed 1/60.
    pub timestep: f32,
    /// The explicit CalcManualEffect argument; Ground callers pass false.
    pub powersliding: bool,
    /// Processed2468 bit29 selects BrakeTiltAngle and correction-force gates.
    pub braking: bool,
    /// Processed2472 bits27 and26, respectively. Physical axle identities are
    /// not assumed here; the signed balance gates are preserved directly.
    pub positive_balance_contact: bool,
    pub negative_balance_contact: bool,
    /// Processed2468 bit20 reverses the point-selection balance sign.
    pub reversed_point_selection: bool,
    /// Reference frame from board+292, matrix+752: first and third columns.
    pub reference_x: [f32; 4],
    pub reference_z: [f32; 4],
    /// Deck part6 transform's z-axis from 82585CB0, not an animation pose.
    pub deck_z: [f32; 4],
    /// Processed effective transform third column+224 and ground velocity+416.
    pub velocity_frame_z: [f32; 4],
    /// Legacy field name: this is Processed416, NOT angular velocity720.
    /// Native82C04C88 projects it into the effective frame;82C04E60..EC4
    /// emits -400 times this same translational velocity for correction.
    pub angular_velocity_world: [f32; 4],
    /// Front/back truck drive-frame translations (7840+48 and7904+48).
    ///82C0B9C0 steering rotates the bases and preserves these translations.
    pub correction_point_7888: [f32; 4],
    pub correction_point_7952: [f32; 4],
}

/// Exact 8296EC98 dependency: degeneracy gates, normalization, acos and signed
/// cross-product selection. There is deliberately no host atan2/acos fallback.
pub trait ManualAngleMeasurement {
    type Error;
    fn angle_between(
        &mut self,
        deck_z: [f32; 4],
        reference_z: [f32; 4],
        reference_x: [f32; 4],
    ) -> Result<f32, Self::Error>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ManualError<E> {
    Angle(angle::AngleError),
    Measurement(E),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManualEffect {
    /// Native result+0, applied with ApplyAngularDisplacementToDeck.
    pub angular_displacement: [f32; 4],
    /// Result+16/+32 are explicitly zeroed by this function.
    pub force_world: [f32; 4],
    pub force_point_body: [f32; 4],
    /// Result+48/+64, plus result+80/+81 status bytes.
    pub corrective_force_world: [f32; 4],
    pub corrective_point_body: [f32; 4],
    pub correction_active: bool,
    pub opposing_motion_without_correction: bool,
}

/// An unavailable measurement/conversion aborts the tick. Already executed
/// scalar-state writes remain visible, but no effect is returned for publication.
pub fn calculate<G: ManualAngleMeasurement>(
    state: &mut ManualState,
    settings: &ManualSettings,
    mode: ManualMode,
    input: &ManualInput,
    geometry: &mut G,
) -> Result<ManualEffect, ManualError<G::Error>> {
    let mut effect = ManualEffect {
        angular_displacement: [0.0; 4],
        force_world: [0.0; 4],
        force_point_body: [0.0; 4],
        corrective_force_world: [0.0; 4],
        corrective_point_body: [0.0; 4],
        correction_active: false,
        opposing_motion_without_correction: false,
    };
    let mut output_scale = 1.0;
    if input.balance == 0.0 {
        state.reset();
    } else {
        let norm = |value| angle::normalize(value).map_err(ManualError::Angle);
        let gains = if input.powersliding {
            settings.powerslide
        } else {
            settings.manual
        };
        let scaled_balance = input.balance * input.flipped_controls;
        let sign = if scaled_balance > 0.0 { 1.0 } else { -1.0 };
        let signed_target = scaled_balance.abs().mul_add(0.75, 0.25) * sign;
        let started = if state.elapsed == 0.0 { 0.0 } else { 1.0 };
        let degrees_to_radians = f32::from_bits(0x3c8e_fa35);
        let base_angle =
            norm((settings.maximum_tilt_degrees * signed_target) * degrees_to_radians)?;
        let target = if input.braking {
            norm((settings.brake_tilt_degrees * signed_target) * degrees_to_radians)?
        } else {
            let phase = (input.procedural_noise_time * settings.procedural_noise_frequency)
                * f32::from_bits(0x40c9_0fdb);
            let noise = crate::trigonometry::sin(phase) * settings.procedural_noise_scale;
            let speed_scale = settings.noise_vs_speed.evaluate(input.absolute_speed);
            let animated = input.animation_noise * settings.animation_noise_scale;
            let disturbance = animated.mul_add(input.flipped_controls, noise);
            let noise_angle = norm(speed_scale * disturbance)?;
            norm(noise_angle + base_angle)?
        };
        if state.elapsed == 0.0 {
            state.angular_correction = norm(settings.start_torque_scale * target)?;
        }
        state.target_angle = target;
        let measured = geometry
            .angle_between(input.deck_z, input.reference_z, input.reference_x)
            .map_err(ManualError::Measurement)?;
        let measured = norm(measured)?;
        let angle_change = measured - state.measured_angle;
        state.measured_angle = measured;
        let error = clamp_symmetric(norm(target - measured)?, settings.maximum_angle_error);
        let weighted_error = error * f32::from_bits(0x3d4c_cccd);
        state.filtered_angle_error = state
            .filtered_angle_error
            .mul_add(f32::from_bits(0x3f73_3333), weighted_error);
        let derivative = clamp_symmetric(
            (angle_change * started) * gains.derivative,
            settings.derivative_limit,
        );
        let wheel_scale = if input.positive_balance_contact != input.negative_balance_contact {
            0.5
        } else {
            1.0
        };
        let wrong_contact = (!input.positive_balance_contact && input.balance > 0.0)
            || (!input.negative_balance_contact && input.balance < 0.0);
        if wrong_contact {
            state.angular_correction =
                (settings.torque_bleed_without_contact * state.angular_correction) * wheel_scale;
            output_scale = settings.torque_scale_without_contact * wheel_scale;
        } else {
            state.angular_correction = ((state.angular_correction + derivative)
                + state.filtered_angle_error * gains.integral)
                + error * gains.proportional;
        }
        apply_correction(&mut effect, input, mode, target, measured);
        state.elapsed = input.timestep + state.elapsed;
    }
    let correction = state.angular_correction * output_scale;
    effect.angular_displacement = input.reference_x.map(|component| (-component) * correction);
    Ok(effect)
}

fn apply_correction(
    effect: &mut ManualEffect,
    input: &ManualInput,
    mode: ManualMode,
    target: f32,
    measured: f32,
) {
    let frame = input.velocity_frame_z;
    let velocity = input.angular_velocity_world;
    let local_z = frame[2].mul_add(
        velocity[2],
        frame[1].mul_add(velocity[1], frame[0] * velocity[0]),
    );
    if !input.braking || !(0.0 > local_z * input.balance) {
        return;
    }
    if !mode.corrective_force_enabled {
        effect.opposing_motion_without_correction = true;
        return;
    }
    if !(local_z.abs() > mode.correction_angular_speed_threshold) {
        return;
    }
    let radians_to_degrees = f32::from_bits(0x4265_2ee1);
    let error_degrees = (target * radians_to_degrees) - (measured * radians_to_degrees);
    if !(error_degrees.abs() < 1.0) {
        return;
    }
    let alternate = if input.reversed_point_selection {
        input.balance < 0.0
    } else {
        input.balance > 0.0
    };
    effect.corrective_point_body = if alternate {
        input.correction_point_7888
    } else {
        input.correction_point_7952
    };
    effect.corrective_force_world = velocity.map(|component| (component * -1.0) * 400.0);
    effect.correction_active = true;
}

fn clamp_symmetric(value: f32, bound: f32) -> f32 {
    let lower = -bound;
    let lower_clamped = if lower - value >= 0.0 { lower } else { value };
    if bound - lower_clamped >= 0.0 {
        lower_clamped
    } else {
        bound
    }
}
