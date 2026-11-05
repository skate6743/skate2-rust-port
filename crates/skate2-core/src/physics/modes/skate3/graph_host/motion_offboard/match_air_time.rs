//! Native MatchAirTime: factory82BCA308, Begin82BB9F48,
//! Update82BBA070, instance82BBA390. End is the empty82B61BB8.
use super::MotionAnimation;
use skate3_core::animation::{
    playback_parameters::{AttributeSink, SettableAttribute},
    skeleton_input::name::encode,
};

#[derive(Default)]
pub(crate) struct State {
    cadence_start: f32,
    duration: f32,
    translation: [f32; 4],
    first_update: bool,
}
impl State {
    pub fn begin(&mut self, cadence: f32, duration: f32, translation: [f32; 4]) {
        self.first_update = true;
        self.cadence_start = cadence;
        self.duration = duration;
        self.translation = translation;
    }

    pub fn update(&mut self, animation: &mut MotionAnimation,
        remaining: f32, duration: f32, translation: [f32; 4]) {
        if let Some(fraction) = self.seek_fraction(remaining, duration) {
            animation.synchronize_air_time(fraction);
        }
        self.translation = bounded_translation(translation);
        self.duration = duration;
        for (name, value) in [
            ("CadenceStartPercent", self.cadence_start),
            ("AnimTime", self.duration),
            ("AnimTransX", self.translation[0]),
            ("AnimTransY", self.translation[1]),
            ("AnimTransZ", self.translation[2]),
        ] {
            //82BBA2C0..378: r6=0, r7=-1 for each v80 SetAttribute.
            animation.set_attribute(SettableAttribute {
                name: encode(name.as_bytes()), value, normalized: false, sequence_id: -1,
            });
        }
        self.first_update = false;
    }

    fn seek_fraction(&self, remaining: f32, duration: f32) -> Option<f32> {
        //82BBA16C skips the first update;184 is ble (unordered falls through).
        if self.first_update || duration <= 0.0 { return None; }
        let progress = 1.0 - remaining / duration;
        let lower = if -progress >= 0.0 { 0.0 } else { progress };
        Some(if 1.0 - lower >= 0.0 { lower } else { 1.0 })
    }
}

fn bounded_translation(vector: [f32; 4]) -> [f32; 4] {
    //82BBA218 vmsum3, two rsqrt refinements, zero selection. Independent
    //PC seed, not a claim of bit-exact Xenon reciprocal-square-root emulation.
    let squared = (vector[0] * vector[0] + vector[1] * vector[1]) + vector[2] * vector[2];
    let mut inverse = squared.sqrt().recip();
    for _ in 0..2 {
        let correction = (-squared).mul_add(inverse * inverse, 1.0);
        inverse = (inverse * 0.5).mul_add(correction, inverse);
    }
    let length = if squared == 0.0 { 0.0 } else { squared * inverse };
    if length > 0.01 {
        let bounded = if length - 2.0 >= 0.0 { 2.0 } else { length };
        //82BBA2A0 scales all four lanes. Published scalar parameters are XYZ.
        vector.map(|v| v * (bounded / length))
    } else { vector }
}

#[cfg(any())]
#[path = "match_air_time/tests.rs"]
mod tests;
