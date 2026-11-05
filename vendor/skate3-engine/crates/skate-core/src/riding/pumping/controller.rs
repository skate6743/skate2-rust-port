//! Recovered scalar/control flow of TU3 82D8F228 and 82D8F470.
//!
//! Production uses geometry::NativePumpingGeometry for the original vector
//! operations, refinement order and inverse-cosine kernel. The Ground adapter
//! supplies the current physical sample and the selected stock mode each tick.
use super::{settings::*, state::PumpingState};

/// Literal loaded by Ground Update 82D37EA0, independent of Processed+2604.
pub const GROUND_PUMPING_TIMESTEP: f32 = f32::from_bits(0x3c88_8889);

#[derive(Clone, Copy, Debug)]
pub struct PumpingSample {
    /// Ground caller: ProcessedPhysIn+112, ground context+1216, skeleton+11008.
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub com_to_deck_world: [f32; 4],
    /// Ground's signed-angle difference calculation supplies the absolute angle.
    pub deck_angle: f32,
    /// Ground caller extracts ProcessedPhysIn+2476 bit 1.
    pub intentional_pumping: u8,
}

/// Recovered numeric operations, implemented by geometry::NativePumpingGeometry.
/// Keeping this boundary explicit allows independent measurements to validate
/// the surrounding lifecycle without asserting numeric parity from mocks.
pub trait PumpingGeometry {
    type Error;
    /// 82D8F280..290: three-component COM-to-deck projection onto the normal.
    fn height(&mut self, normal: [f32; 4], com_to_deck: [f32; 4]) -> Result<f32, Self::Error>;
    /// 82D8F4A0..574: displacement divided by dt, then its length.
    fn speed(&mut self, previous: [f32; 4], current: [f32; 4], dt: f32)
    -> Result<f32, Self::Error>;
    /// 82D8F590..6AC: dot(cross(previous_normal, normal)/dt,
    /// safe_normalize(cross(position-previous_position, normal))).
    /// Preserve the recovered fused cross-product and refinement order.
    fn angular_speed(
        &mut self,
        previous_position: [f32; 4],
        previous_normal: [f32; 4],
        sample: &PumpingSample,
        dt: f32,
    ) -> Result<f32, Self::Error>;
    /// 82453298 called at 82D8F6EC, with previous normal.y clamped to [0,1].
    fn inclination_radians(&mut self, clamped_normal_y: f32) -> Result<f32, Self::Error>;
}

/// Executes the recovered lifecycle only when the required geometry succeeds.
/// An error leaves the already executed prefix visible, and must abort the
/// caller's simulation tick; it is never permission to publish partial forces.
pub fn update<G: PumpingGeometry>(
    state: &mut PumpingState,
    settings: &PumpingSettings,
    mode: PumpingMode,
    sample: PumpingSample,
    timestep: f32,
    geometry: &mut G,
) -> Result<(), G::Error> {
    state.pump_acceleration = 0.0;
    let height = geometry.height(sample.normal, sample.com_to_deck_world)?;
    if state.record_valid {
        state.intentional_pumping = sample.intentional_pumping;
        let change = height - state.previous_height;
        let rising_change = if change >= 0.0 { change } else { 0.0 };
        let retained = (1.0 - settings.height_change_damping) * state.smoothed_height_change;
        let smoothed = rising_change.mul_add(settings.height_change_damping, retained);
        state.smoothed_height_change = smoothed;
        // bge after fcmpu takes the unordered path too: do not replace with >=.
        let mut effect = if smoothed < settings.minimum_height_change {
            0.0
        } else {
            smoothed
        };
        state.pumping_time = if effect > 0.0 {
            state.pumping_time + timestep
        } else {
            0.0
        };
        if effect.abs() > settings.maximum_height_change {
            let sign = if effect >= 0.0 {
                if effect > 0.0 { 1.0 } else { 0.0 }
            } else {
                -1.0
            };
            effect = sign * settings.maximum_height_change;
        }
        effect = settings.pump_vs_time.evaluate(state.pumping_time) * effect;
        let pumping = calculate(state, settings, &sample, timestep, geometry)?;
        state.pumping = pumping;
        let factor = if pumping * effect > 0.0 {
            mode.acceleration_factor
        } else {
            mode.absorption_factor
        };
        let raw = (pumping * factor) * effect;
        let lower = -(mode.maximum_absorption_per_second * timestep);
        let upper = mode.maximum_acceleration_per_second * timestep;
        let lower_clamped = if lower - raw >= 0.0 { lower } else { raw };
        state.pump_acceleration = if upper - lower_clamped >= 0.0 {
            lower_clamped
        } else {
            upper
        };
    }
    state.previous_position = sample.position;
    state.previous_normal = sample.normal;
    state.record_valid = true;
    state.previous_height = height;
    Ok(())
}

/// Pumping slice of Ground Update 82D37EA8. Force assembly must follow only
/// after this succeeds; it has its own ProcessedPhysIn timestep for force scaling.
pub fn update_ground<G: PumpingGeometry>(
    state: &mut PumpingState,
    settings: &PumpingSettings,
    mode: PumpingMode,
    sample: PumpingSample,
    geometry: &mut G,
) -> Result<(), G::Error> {
    update(
        state,
        settings,
        mode,
        sample,
        GROUND_PUMPING_TIMESTEP,
        geometry,
    )
}

/// Scalar portion of Calculate, including output write/curve evaluation order.
/// Does not update the Update-owned cached pose, pump time or pump acceleration.
pub fn calculate<G: PumpingGeometry>(
    state: &mut PumpingState,
    settings: &PumpingSettings,
    sample: &PumpingSample,
    timestep: f32,
    geometry: &mut G,
) -> Result<f32, G::Error> {
    let speed = geometry.speed(state.previous_position, sample.position, timestep)?;
    let speed_effect = settings.pump_vs_speed.evaluate(speed);
    let angular_speed = geometry.angular_speed(
        state.previous_position,
        state.previous_normal,
        sample,
        timestep,
    )?;
    let measured = angular_speed * settings.angular_speed_damping;
    state.angular_speed =
        (1.0 - settings.angular_speed_damping).mul_add(state.angular_speed, measured);
    state.absorption = -(speed * state.angular_speed);
    let inclination = geometry.inclination_radians(unit(state.previous_normal[1]))?;
    // Stock 822F8B7C: 2/pi rounded to binary32. This is not host acos().
    let radians_to_quarter_turns = f32::from_bits(0x3f22_f983);
    let ground_angle = unit(radians_to_quarter_turns * inclination);
    state.ground_normal_absorption = settings.compression_vs_ground_angle.evaluate(ground_angle)
        * settings.ground_compression_scale;
    state.minimum_crouch = settings.min_crouch_vs_ground_angle.evaluate(ground_angle);
    state.deck_angle_absorption = settings
        .compression_vs_deck_angle
        .evaluate(sample.deck_angle * radians_to_quarter_turns);
    let deck_compression = settings.deck_compression_scale * state.deck_angle_absorption;
    let pumping = state.angular_speed * speed_effect;
    if deck_compression * deck_compression
        > state.ground_normal_absorption * state.ground_normal_absorption
    {
        state.ground_normal_absorption = deck_compression;
    }
    Ok(pumping)
}

fn unit(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}
