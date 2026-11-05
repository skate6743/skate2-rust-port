//! Complete TU3 SpeedWobble::CalculateSpeedWobble (82C0DBE8).
//! UpdateSpeedWobble 82C03638 supplies the selected-mode attributes and body
//! inputs; Ground Update 82D38800 applies the returned tilt before truck targets.
use crate::{point_graph::PointGraph, trigonometry::sin};

#[derive(Clone, Copy, Debug)]
pub struct SpeedWobbleSettings {
    /// Layout arrays +0/+32, +64/+96, +128/+160, +192/+224.
    pub frequency_time: PointGraph<8>,
    pub frequency_speed: PointGraph<8>,
    pub amplitude_time: PointGraph<8>,
    pub amplitude_speed: PointGraph<8>,
    /// Layout +256 through +284, in native order.
    pub tightness_threshold: f32,
    pub time_range: f32,
    pub speed_range: f32,
    pub frequency_scale: f32,
    pub amplitude_scale: f32,
    pub crouch_threshold: f32,
    pub height_min: f32,
    pub height_max: f32,
}

/// Fields +16..+44, including untouched +20 and opaque/reset-only +24/+28.
/// The active flag is a byte; retain its other three packed bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeedWobbleState(pub [u32; 8]);

impl SpeedWobbleState {
    /// The same six-float/one-byte reset used in Ground Enter 82D376D0.
    pub fn reset(&mut self) {
        for index in [0, 2, 3, 4, 5, 6] {
            self.0[index] = 0;
        }
        self.0[7] &= 0x00ff_ffff;
    }

    fn active(&self) -> bool {
        self.0[7] >> 24 != 0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpeedWobbleInput {
    pub tilt: f32,
    /// Signed speed, ProcessedPhysIn +2612. This routine does not take abs().
    pub speed: f32,
    pub center_of_mass_height: f32,
    pub truck_tightness: f32,
    /// Resolved mode Hash_77AFCE78FE1206CA / Hash_5B57F2CCCCEEF430.
    pub activation_threshold: f32,
    pub amplitude_multiplier: f32,
}

pub fn calculate(
    state: &mut SpeedWobbleState,
    settings: &SpeedWobbleSettings,
    input: SpeedWobbleInput,
) -> f32 {
    let height = (input.center_of_mass_height - settings.height_min)
        / (settings.height_max - settings.height_min);
    let crouch = unit(1.0 - height);
    let threshold = crouch.mul_add(
        settings.crouch_threshold,
        settings.tightness_threshold * input.truck_tightness,
    ) + input.activation_threshold;
    if !state.active() && input.speed > threshold {
        state.reset();
        state.0[7] |= 0x0100_0000;
    }
    if !state.active() {
        return input.tilt;
    }

    let time = f32::from_bits(state.0[0]);
    let time_fraction = unit(time / settings.time_range);
    let speed_fraction = unit((input.speed - threshold) / settings.speed_range);
    let frequency = (settings.frequency_time.evaluate(time_fraction)
        * settings.frequency_speed.evaluate(speed_fraction))
        * settings.frequency_scale;
    let amplitude = settings.amplitude_speed.evaluate(speed_fraction)
        * settings.amplitude_time.evaluate(time_fraction);
    let phase = frequency.mul_add(f32::from_bits(0x3DD6_7751), f32::from_bits(state.0[4]));
    let scale = (settings.amplitude_scale * amplitude) * input.amplitude_multiplier;
    let displacement = sin(phase) * scale;
    state.0[0] = (time + f32::from_bits(0x3C88_8889)).to_bits();
    state.0[4] = phase.to_bits();
    state.0[5] = amplitude.to_bits();
    state.0[6] = displacement.to_bits();
    let result = (displacement + input.tilt) * 0.5;
    // This frame still returns the computed tilt when speed drops below threshold.
    if input.speed < threshold {
        state.reset();
    }
    result
}

fn unit(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}
