//! Type11's authored metric and first-update selection: 82D26CC0/82D26FF8.
//! Native construction clears transition/reselect bits. No exposed operation
//! on this tree enables them; ordinary playback therefore latches one child.
use super::super::{output::attributes::AttributeName, playback_parameters::SettableAttribute};
use super::PlaybackTree;

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: AttributeName,
    pub mode: u32,
    pub weight: f32,
    pub minimum: f32,
    pub maximum: f32,
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub name: String,
    pub values: Vec<f32>,
    pub tree: PlaybackTree,
}
#[derive(Clone, Debug)]
pub struct SelectionSpace {
    pub parameters: Vec<Parameter>,
    pub candidates: Vec<Candidate>,
    pub selected: Option<usize>,
    values: Vec<Option<f32>>,
    speed: f32,
    requested_time: f32,
}
impl SelectionSpace {
    pub fn new(parameters: Vec<Parameter>, candidates: Vec<Candidate>) -> Result<Self, String> {
        if parameters.len() > 10
            || candidates.is_empty()
            || candidates
                .iter()
                .any(|c| c.values.len() != parameters.len())
        {
            return Err("Invalid native selection space dimensions".into());
        }
        let values = vec![None; parameters.len()];
        Ok(Self {
            parameters,
            candidates,
            selected: None,
            values,
            speed: 1.0,
            requested_time: 0.0,
        })
    }
    pub fn current(&self) -> Option<&PlaybackTree> {
        self.selected.map(|i| &self.candidates[i].tree)
    }
    pub fn current_mut(&mut self) -> Option<&mut PlaybackTree> {
        self.selected.map(|i| &mut self.candidates[i].tree)
    }
    pub fn set_time(&mut self, time: f32) {
        self.requested_time = time;
        if let Some(child) = self.current_mut() {
            child.set_time(time);
        }
    }
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
        if let Some(child) = self.current_mut() {
            child.set_speed(speed);
        }
    }
    pub fn select(&mut self, attributes: &[SettableAttribute]) -> Result<(), String> {
        for attribute in attributes {
            for (parameter, value) in self.parameters.iter().zip(&mut self.values) {
                if parameter.name == attribute.name {
                    *value = Some(attribute.value);
                }
            }
        }
        if self.selected.is_none() {
            let values = self
                .values
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    value.ok_or_else(|| {
                        format!("SelectionSpace parameter {} has no source producer", i)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut best = f32::MAX;
            let mut selected = None;
            for (i, candidate) in self.candidates.iter().enumerate() {
                let distance = distance(&self.parameters, &values, &candidate.values);
                if distance < best {
                    best = distance;
                    selected = Some(i);
                }
            }
            let chosen = selected
                .ok_or_else(|| "SelectionSpace has no finite native minimum".to_string())?;
            //82D26EE4..F88 resolves the first child record with the chosen name.
            self.selected = self
                .candidates
                .iter()
                .position(|c| c.name == self.candidates[chosen].name);
        }
        Ok(())
    }
    pub fn set_attributes(&mut self, attributes: &[SettableAttribute]) -> Result<bool, String> {
        self.select(attributes)?;
        let speed = self.speed;
        if let Some(child) = self.current_mut() {
            child.set_attributes(attributes)?;
            child.set_speed(speed);
        }
        Ok(true)
    }
}

/// Original 82D26FF8 uses two accumulators and FMA for pairs, then a separate
/// multiply/multiply/add for an odd tail. Do not reorder into a generic sum.
pub fn distance(parameters: &[Parameter], values: &[f32], candidate: &[f32]) -> f32 {
    let mut even = 0.0f32;
    let mut odd = 0.0f32;
    let pairs = parameters.len() / 2 * 2;
    for i in (0..pairs).step_by(2) {
        let d = delta(&parameters[i], values[i], candidate[i]);
        even = (parameters[i].weight * d).mul_add(d, even);
        let d = delta(&parameters[i + 1], values[i + 1], candidate[i + 1]);
        odd = (parameters[i + 1].weight * d).mul_add(d, odd);
    }
    let sum = odd + even;
    if pairs < parameters.len() {
        let d = delta(&parameters[pairs], values[pairs], candidate[pairs]);
        sum + (parameters[pairs].weight * d) * d
    } else {
        sum + 0.0
    }
}
fn delta(parameter: &Parameter, mut value: f32, mut candidate: f32) -> f32 {
    let range = parameter.maximum - parameter.minimum;
    if range > f32::from_bits(0x3780_0000) {
        candidate /= range;
        value /= range;
    }
    match parameter.mode {
        1 if value > candidate => (1.0 - value) + candidate,
        2 => {
            let difference = (value - candidate).abs();
            let wrapped = 1.0 - difference;
            if difference - wrapped >= 0.0 {
                wrapped
            } else {
                difference
            }
        }
        _ => candidate - value,
    }
}
