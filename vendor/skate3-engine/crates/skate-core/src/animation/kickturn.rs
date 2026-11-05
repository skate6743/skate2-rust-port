//! Original Skate 3 TU3 KickTurnSteering: constructor82BAE730,
//! Begin82BAE430 and Update82BAE4A8. Curves are Globals324 anim_motion/kickturn.
use crate::point_graph::PointGraph;

#[derive(Clone, Debug)]
pub struct Settings {
    pub spin: PointGraph<8>,
    pub height: PointGraph<8>,
}

/// Authored behavior fields28/32; constructor82BC7218 defaults both to0.5.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    pub max_height: f32,
    pub spin_scale: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    animation_length: f32,
    elapsed: f32,
    first_update: bool,
    previous_height: f32,
    last_nonzero_intent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Output {
    /// Published first, to the actual MotionGraph Balance attribute.
    pub balance: f32,
    pub spin: f32,
}

impl State {
    pub fn new() -> Self {
        Self {
            animation_length: 0.0,
            elapsed: 0.0,
            first_update: true,
            previous_height: 0.0,
            last_nonzero_intent: 0.0,
        }
    }

    pub fn begin(&mut self, settings: &Settings) {
        *self = Self::new();
        //82BAE480 seeds the unscaled curve value, before applying max_height.
        self.previous_height = settings.height.evaluate(0.0);
    }

    pub fn needs_animation_length(&self) -> bool {
        self.first_update
    }

    /// The caller first applies pending animation parameters, then reads the
    /// base animation tree's length. No time is added on that first update.
    pub fn capture_animation_length(&mut self, length: f32) {
        self.animation_length = length;
    }

    pub fn update(
        &mut self,
        dt: f32,
        intent: f32,
        parameters: Parameters,
        settings: &Settings,
    ) -> Output {
        if !self.first_update {
            self.elapsed += dt;
        }
        let phase = bound(self.elapsed / self.animation_length, 1.0);
        let spin_curve = settings.spin.evaluate(phase);
        let height_curve = settings.height.evaluate(phase);
        //82BAE660 fmadds, followed by separate82BAE670 fmuls. TU3 fixes
        //the blend coefficient at0.5; the S2 debug alternatives are absent.
        let sum = height_curve.mul_add(parameters.max_height, self.previous_height);
        let height = bound(sum * 0.5, parameters.max_height);
        self.previous_height = height;
        if intent != 0.0 {
            self.last_nonzero_intent = intent;
        }
        let sign = if self.last_nonzero_intent >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let output = Output {
            balance: -height,
            spin: (parameters.spin_scale * sign) * spin_curve,
        };
        self.first_update = false;
        output
    }
}

//Source fsel(-value,0,value), then fsel(max-lower,lower,max).
//Preserves source unordered handling: a NaN selects the upper bound.
fn bound(value: f32, maximum: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if maximum - lower >= 0.0 {
        lower
    } else {
        maximum
    }
}

#[cfg(test)]
#[path = "tests/kickturn.rs"]
mod tests;
