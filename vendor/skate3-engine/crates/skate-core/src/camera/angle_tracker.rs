//! TU3 angular-tracking candidates; exceptional wrap conversion is unavailable.
use super::{ScalarTracker, ScalarTrackerParameters};

/// Angular state has the same six-word layout as the scalar tracker. Angles
/// and angular rates use radians; effective settings are explicitly supplied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngleTracker(pub ScalarTracker);

impl AngleTracker {
    pub fn update(&mut self, dt: f32, target: f32, parameters: ScalarTrackerParameters) {
        let state = &mut self.0;
        let position = state.position;
        state.target = target;
        let delta = normalize_angle(target - position);
        if parameters.overshoot_zeroes_velocity && state.velocity * delta < 0.0 {
            state.velocity *= 0.0;
        }
        let velocity = state.velocity;
        let inverse_dt = 1.0 / dt;
        state.acceleration = inverse_dt.mul_add(delta, -velocity) * inverse_dt;
        let magnitude = delta.abs();
        let fraction = if magnitude < parameters.delta_umbra {
            1.0
        } else if magnitude < parameters.delta_penumbra {
            1.0 - (magnitude - parameters.delta_umbra)
                / (parameters.delta_penumbra - parameters.delta_umbra)
        } else {
            0.0
        };
        if state.acceleration * delta > 0.0 {
            state.acceleration_clamp = super::tracker::interpolate(
                parameters.acceleration_clamp_min,
                parameters.acceleration_clamp_max,
                fraction,
            );
            let magnitude = state.acceleration.abs();
            if magnitude > state.acceleration_clamp {
                state.acceleration = (state.acceleration / magnitude) * state.acceleration_clamp;
            }
        }
        state.velocity = state.acceleration.mul_add(dt, velocity);
        let magnitude = state.velocity.abs();
        if magnitude > parameters.speed_clamp {
            state.velocity = (state.velocity / magnitude) * parameters.speed_clamp;
        }
        // Native normalizes each intermediate separately. Combining these into
        // a single wrap, or fusing the position addition, changes boundary cases.
        let next = normalize_angle(normalize_angle(state.velocity * dt) + position);
        state.smoothing = super::tracker::interpolate(
            parameters.smoothing_min,
            parameters.smoothing_max,
            fraction,
        );
        let displacement = normalize_angle(next - position);
        let smoothed = normalize_angle(displacement * (1.0 - state.smoothing));
        state.position = normalize_angle(smoothed + position);
    }
}

/// Candidate 8258DB98 wrapping sequence. Its fast interval is [-pi, pi), and
/// its reduction uses truncation followed by one correction, not rem_euclid.
pub fn normalize_angle(angle: f32) -> f32 {
    const PI: f32 = f32::from_bits(0x40490fdb);
    const NEG_PI: f32 = f32::from_bits(0xc0490fdb);
    const INV_TAU: f32 = f32::from_bits(0x3e22f983);
    const TAU: f32 = f32::from_bits(0x40c90fdb);
    if angle >= NEG_PI && angle < PI {
        return angle;
    }
    let turns = angle * INV_TAU;
    // Ordinary finite truncation follows the native fctiwz operation. Its
    // exceptional/overflow results are unresolved; do not borrow host rules.
    if !turns.is_finite() || turns < i32::MIN as f32 || turns as f64 > i32::MAX as f64 {
        panic!(
            "Skate 3 exceptional angle integer conversion is unavailable: independent implementation required"
        );
    }
    let count = turns as i32;
    let count = count as f32;
    let reduced = -((count as f64).mul_add(TAU as f64, -(angle as f64))) as f32;
    if reduced >= PI {
        reduced - TAU
    } else if reduced < NEG_PI {
        reduced + TAU
    } else {
        reduced
    }
}
