//! Pumping behavior82BAC1B0 and instance seeds82BAC528.
use crate::point_graph::PointGraph;
#[derive(Clone, Debug)]
pub struct Settings {
    pub amplify: PointGraph<8>,
    pub input_blend: f32,
    pub maximum_physics_pump: f32,
    pub new_pump_threshold: f32,
    pub blend_in: f32,
    pub blend_out: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct State {
    value: f32,
    filtered: f32,
    channel: Option<usize>,
    peak: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            value: 0.0,
            filtered: 0.0,
            channel: None,
            peak: 0.0,
        }
    }
}
pub struct Update {
    pub start: Option<usize>,
    pub influence: Option<(usize, f32)>,
}
impl State {
    pub fn update(&mut self, physics_pump: f32, occupied: [bool; 5], s: &Settings) -> Update {
        let previous = self.value;
        let input = physics_pump / s.maximum_physics_pump;
        let lower = if -input >= 0.0 { 0.0 } else { input };
        let input = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
        //82BAC2F8 is fmadds; preserve the single final rounding.
        self.filtered = input.mul_add(s.input_blend, (1.0 - s.input_blend) * self.filtered);
        self.value = s.amplify.evaluate(self.filtered);
        let start = if !(previous > s.new_pump_threshold) && self.value > s.new_pump_threshold {
            occupied.iter().position(|&exists| !exists)
        } else {
            None
        };
        if let Some(index) = start {
            self.channel = Some(index);
            self.peak = self.value;
        }
        let influence = if self.value >= previous && self.value >= self.peak {
            self.channel.map(|index| {
                self.peak = self.value;
                (index, self.value)
            })
        } else {
            None
        };
        Update { start, influence }
    }
}
