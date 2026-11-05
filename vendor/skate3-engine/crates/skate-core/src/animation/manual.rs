//! SetManualAngle Begin82BA8C48/Update82BA8C60. Animation selection only;
//! physical balance is separately emitted by the authored AttachIntent node.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// anim_motion/manual layout1216..1276, native eight-point evaluator.
    pub balance: PointGraph<8>,
    /// Layout1800/1804. Limits are per graph update, not per second.
    pub velocity_limit: f32,
    pub acceleration_limit: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub angle: f32,
    pub velocity: f32,
}

impl State {
    pub fn begin(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, manual: Option<f32>, settings: &Settings) -> f32 {
        // The native lookup begins with zero; an absent intent leaves it zero.
        let manual = manual.unwrap_or(0.0);
        let target = settings.balance.evaluate(manual.abs());
        let target = if manual >= -0.0 { target } else { -target };
        let desired_velocity = clamp(target - self.angle, settings.velocity_limit);
        let acceleration = clamp(
            desired_velocity - self.velocity,
            settings.acceleration_limit,
        );
        self.velocity += acceleration;
        self.angle += self.velocity;
        self.angle
    }
}

fn clamp(value: f32, limit: f32) -> f32 {
    // Preserve the source fsubs/fsel selection, including signed zero.
    let lower = if -limit - value >= -0.0 {
        -limit
    } else {
        value
    };
    if limit - lower >= -0.0 { lower } else { limit }
}
