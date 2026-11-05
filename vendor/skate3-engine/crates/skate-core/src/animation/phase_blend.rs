//! Andale PhaseBlend, TU3 82D24C40..82D25808; construction 82D1B67C.
use super::{
    clip_clock::AdvanceResult,
    output::attributes::{AnimationAttribute, AttributeName, MotionGraphAttribute},
    playback_attributes,
    playback_parameters::SettableAttribute,
    playback_tree::{Evaluation, PlaybackTree, PoseCommand},
};

#[derive(Clone, Debug)]
pub struct PhaseBlend {
    pub parameter: AttributeName,
    pub children: Vec<PlaybackTree>,
    order: Vec<usize>,
    values: Vec<f32>,
    left: usize,
    right: usize,
    weight: f32,
    parameter_value: f32,
    normalized: bool,
    cull_threshold: f32,
    pub length: f32,
    pub time: f32,
}
impl PhaseBlend {
    pub fn new(parameter: AttributeName, children: Vec<PlaybackTree>) -> Result<Self, String> {
        if children.len() < 2 {
            return Err("PhaseBlend requires at least two authored children".into());
        }
        let count = children.len();
        Ok(Self {
            parameter,
            length: children[0].length(),
            children,
            order: (0..count).collect(),
            values: vec![0.0; count],
            left: 0,
            right: 1,
            weight: 0.0,
            parameter_value: f32::MAX,
            normalized: true,
            cull_threshold: 0.0,
            time: 0.0,
        })
    }
    pub fn set_time(&mut self, time: f32) {
        let phase = time / self.length;
        let left_time = self.children[self.left].length() * phase;
        let right_time = self.children[self.right].length() * phase;
        self.children[self.left].set_time(left_time);
        self.children[self.right].set_time(right_time);
        self.time = time;
    }
    fn phase(&self) -> f32 {
        self.time
            / if -self.length >= 0.0 {
                1.0
            } else {
                self.length
            }
    }
    fn update_length(&mut self, phase: f32) {
        self.length = self.children[self.right].length().mul_add(
            self.weight,
            self.children[self.left].length() * (1.0 - self.weight),
        );
        self.set_time(self.length * phase);
    }
    pub fn set_speed(&mut self, speed: f32) {
        let phase = self.phase();
        for &index in &self.order {
            self.children[index].set_speed(speed);
        }
        self.update_length(phase);
    }
    pub fn advance(&mut self, dt: f32, phase: f32, property: &mut AdvanceResult) {
        let left_length = self.children[self.left].length();
        let right_length = self.children[self.right].length();
        let fraction = dt / self.length;
        self.children[self.left].advance(fraction * left_length, phase, property);
        let mut discarded = AdvanceResult {
            crossed_end: false,
            overshoot: -1.0,
            remaining_before_wrap: -1.0,
        };
        self.children[self.right].advance(fraction * right_length, phase, &mut discarded);
        let inverse_left = 1.0 / left_length;
        self.time = (self.children[self.left].time() * inverse_left) * self.length;
        property.remaining_before_wrap =
            (property.remaining_before_wrap * inverse_left) * self.length;
        property.overshoot = (inverse_left * self.length) * property.overshoot;
    }
    pub fn set_attributes(&mut self, attributes: &[SettableAttribute]) -> Result<bool, String> {
        let mut changed = false;
        for &index in &self.order {
            changed |= self.children[index].set_attributes(attributes)?;
        }
        if let Some(attribute) = attributes.iter().find(|a| a.name == self.parameter) {
            if attribute.normalized != self.normalized
                || !((attribute.value - self.parameter_value).abs() < f32::from_bits(0x38d1b717))
            {
                self.parameter_value = attribute.value;
                self.normalized = attribute.normalized;
                changed = true;
            }
        }
        if !changed {
            return Ok(false);
        }
        for &index in &self.order {
            let mut attribute = MotionGraphAttribute {
                name: self.parameter,
                value: 0.0,
            }
            .to_animation();
            // Native deliberately ignores success; partial output from nested
            // GetAttribute remains visible, and a complete miss retains zero.
            self.children[index].query_attribute(self.parameter, 15, &mut attribute)?;
            self.values[index] =
                f32::from_bits(attribute.payload.0[0].ok_or("Uninitialized PhaseBlend parameter")?);
        }
        for end in (1..self.order.len()).rev() {
            for i in 0..end {
                if self.values[self.order[i + 1]] < self.values[self.order[i]] {
                    self.order.swap(i, i + 1);
                }
            }
        }
        let minimum = self.values[self.order[0]];
        let maximum = self.values[*self.order.last().unwrap()];
        let target = if self.normalized {
            (maximum - minimum).mul_add(self.parameter_value, minimum)
        } else {
            self.parameter_value
        };
        let mut low = 0;
        let mut count = self.order.len();
        while count != 0 {
            let step = count / 2;
            let middle = low + step;
            if self.values[self.order[middle]] < target {
                low = middle + 1;
                count -= step + 1;
            } else {
                count = step;
            }
        }
        let upper = low.clamp(1, self.order.len() - 1);
        let left = self.order[upper - 1];
        let right = self.order[upper];
        let lower_value = self.values[left];
        let upper_value = self.values[right];
        let delta = upper_value - lower_value;
        let lower = if lower_value - target >= 0.0 {
            lower_value
        } else {
            target
        };
        let clamped = if upper_value - lower >= 0.0 {
            lower
        } else {
            upper_value
        };
        let weight = (clamped - lower_value) / if delta == 0.0 { 1.0 } else { delta };
        let lower = if -weight >= 0.0 { 0.0 } else { weight };
        self.weight = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
        let phase = self.phase();
        self.left = left;
        self.right = right;
        self.update_length(phase);
        Ok(true)
    }
    pub fn attributes(&self, mask: u32) -> Result<Vec<AnimationAttribute>, String> {
        if self.weight < self.cull_threshold {
            return self.children[self.left].attributes(mask);
        }
        if self.weight > 1.0 - self.cull_threshold {
            return self.children[self.right].attributes(mask);
        }
        playback_attributes::intersection(
            self.children[self.left].attributes(mask)?,
            &self.children[self.right].attributes(31)?,
            self.weight,
        )
    }
    pub fn query_attribute(
        &self,
        name: AttributeName,
        mask: u32,
        output: &mut AnimationAttribute,
    ) -> Result<bool, String> {
        if self.weight < self.cull_threshold {
            return self.children[self.left].query_attribute(name, mask, output);
        }
        if self.weight > 1.0 - self.cull_threshold {
            return self.children[self.right].query_attribute(name, mask, output);
        }
        let left = self.children[self.left].query_attribute(name, mask, output)?;
        let mut right = MotionGraphAttribute { name, value: 0.0 }.to_animation();
        let found_right = self.children[self.right].query_attribute(name, 31, &mut right)?;
        if left && found_right {
            playback_attributes::blend(output, &right, self.weight)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn evaluate(
        &mut self,
        parameters: Evaluation,
        enabled: bool,
        output: &mut Vec<PoseCommand>,
    ) -> Result<bool, String> {
        self.cull_threshold = parameters.cull_threshold;
        let c = parameters.cull_threshold;
        let lower = if c - self.weight >= 0.0 {
            c
        } else {
            self.weight
        };
        let upper = 1.0 - c;
        let clamped = if upper - lower >= 0.0 { lower } else { upper };
        let t = (clamped - c) / (-c).mul_add(2.0, 1.0);
        let square = t * t;
        let weight = if !(t > 0.5) && self.left == self.order[0] {
            (-t).mul_add(4.0, 4.0) * square
        } else if t > 0.5 && self.right == *self.order.last().unwrap() {
            let four = t * 4.0;
            (8.0 - four).mul_add(square, -four) + 1.0
        } else {
            self.weight
        };
        let left =
            self.children[self.left].evaluate(parameters, enabled && weight < 1.0, output)?;
        let right =
            self.children[self.right].evaluate(parameters, enabled && weight > 0.0, output)?;
        if left && right {
            output.push(PoseCommand::Blend { weight });
        }
        Ok(left || right)
    }
}
