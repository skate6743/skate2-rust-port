//! Andale BlendSpace82D22DA0..82D24B90. Authored simplex planes, original
//! scalar selection82D23A50 and phase-coupled child clocks; no fitted metric.
use super::{
    AdvanceResult, AnimationAttribute, AttributeName, Evaluation, PlaybackTree, PoseCommand,
    SettableAttribute,
};
use crate::animation::{output::attributes::MotionGraphAttribute, playback_attributes};

#[derive(Clone, Debug)]
pub struct Simplex {
    pub children: Vec<usize>,
    pub vertices: Vec<Vec<f32>>,
    pub normals: Vec<Vec<f32>>,
    pub scales: Vec<f32>,
}
impl Simplex {
    ///82D23898: signed distances to the opposite planes, scaled by inverse height.
    pub fn coordinates(&self, point: &[f32]) -> Vec<f32> {
        (0..self.children.len())
            .map(|i| {
                let anchor = &self.vertices[(i + 1) % self.children.len()];
                let mut even = 0.0;
                let mut odd = 0.0;
                let pairs = point.len() / 2;
                for j in 0..pairs {
                    even = self.normals[i][j * 2].mul_add(point[j * 2] - anchor[j * 2], even);
                    odd = self.normals[i][j * 2 + 1]
                        .mul_add(point[j * 2 + 1] - anchor[j * 2 + 1], odd);
                }
                let mut dot = even + odd;
                if point.len() % 2 != 0 {
                    let j = point.len() - 1;
                    dot += self.normals[i][j] * (point[j] - anchor[j]);
                }
                dot * self.scales[i]
            })
            .collect()
    }
    ///82D23530/82D23080: successive coordinate-plane slices, retaining edge
    ///enumeration order. On an empty slice choose its nearest remaining vertex.
    fn project(&self, point: &[f32]) -> Vec<f32> {
        let mut matrix = self.vertices.clone();
        let mut projected = Vec::with_capacity(point.len());
        for &target in point {
            let count = matrix.len() - 1;
            let mut sliced = Vec::new();
            'edges: for i in 0..count {
                for j in i + 1..matrix.len() {
                    if sliced.len() == count {
                        break 'edges;
                    }
                    let a = &matrix[i];
                    let b = &matrix[j];
                    if (a[0] < target) == (b[0] < target) {
                        continue;
                    }
                    let delta = b[0] - a[0];
                    if delta.abs() <= f32::from_bits(0x3727_c5ac) {
                        //8219B100
                        sliced.push(a[1..].to_vec());
                        if sliced.len() < count {
                            sliced.push(b[1..].to_vec());
                        }
                    } else {
                        let t = (target - a[0]) / delta;
                        sliced.push(
                            a[1..]
                                .iter()
                                .zip(&b[1..])
                                .map(|(&a, &b)| (b - a).mul_add(t, a))
                                .collect(),
                        );
                    }
                }
            }
            if sliced.is_empty() {
                let mut nearest = 0;
                let mut distance = f32::MAX;
                for (i, row) in matrix.iter().enumerate() {
                    let d = (row[0] - target) * (row[0] - target);
                    if d < distance {
                        distance = d;
                        nearest = i;
                    }
                }
                projected.extend_from_slice(&matrix[nearest]);
                break;
            }
            projected.push(target);
            while sliced.len() < count {
                sliced.push(sliced.last().unwrap().clone());
            }
            matrix = sliced;
        }
        projected
    }
}

#[derive(Clone, Debug)]
pub struct BlendSpace {
    pub parameters: Vec<AttributeName>,
    pub children: Vec<PlaybackTree>,
    pub simplexes: Vec<Simplex>,
    values: Vec<f32>,
    current: usize,
    weights: Vec<f32>,
    pub time: f32,
}
impl BlendSpace {
    pub fn new(
        parameters: Vec<AttributeName>,
        children: Vec<PlaybackTree>,
        simplexes: Vec<Simplex>,
    ) -> Result<Self, String> {
        let d = parameters.len();
        if d == 0
            || d > 4
            || children.len() < d + 1
            || simplexes.is_empty()
            || simplexes.iter().any(|s| {
                s.children.len() != d + 1
                    || s.vertices.len() != d + 1
                    || s.normals.len() != d + 1
                    || s.scales.len() != d + 1
                    || s.children.iter().any(|&i| i >= children.len())
                    || s.vertices
                        .iter()
                        .chain(&s.normals)
                        .any(|v| v.len() != d || v.iter().any(|v| !v.is_finite()))
                    || s.scales.iter().any(|v| !v.is_finite())
            })
        {
            return Err("Invalid authored BlendSpace topology".into());
        }
        let mut weights = vec![0.; d + 1];
        weights[0] = 1.;
        Ok(Self {
            parameters,
            children,
            simplexes,
            values: vec![0.; d],
            current: 0,
            weights,
            time: 0.,
        })
    }
    pub fn active_weights(&self) -> impl Iterator<Item = (usize, f32)> + '_ {
        self.simplexes[self.current]
            .children
            .iter()
            .copied()
            .zip(self.weights.iter().copied())
    }
    pub fn length(&self) -> f32 {
        self.active_weights()
            .fold(0., |sum, (i, w)| w.mul_add(self.children[i].length(), sum))
    }
    pub fn set_time(&mut self, time: f32) {
        let phase = time / self.length();
        for &i in &self.simplexes[self.current].children {
            let child = &mut self.children[i];
            child.set_time(phase * child.length());
        }
        //82D24AB0 updates child clocks; the cached time is written by Advance.
    }
    pub fn set_speed(&mut self, speed: f32) {
        for child in &mut self.children {
            child.set_speed(speed);
        }
    }
    pub fn advance(&mut self, dt: f32, phase: f32, property: &mut AdvanceResult) {
        let length = self.length();
        let indices = &self.simplexes[self.current].children;
        let first = indices[0];
        let first_length = self.children[first].length();
        for (slot, &i) in indices.iter().enumerate() {
            let child = &mut self.children[i];
            let mut result = AdvanceResult {
                crossed_end: false,
                overshoot: -1.,
                remaining_before_wrap: -1.,
            };
            child.advance((child.length() * (1. / length)) * dt, phase, &mut result);
            if slot == 0 {
                *property = result;
            }
        }
        let scale = length / first_length;
        property.overshoot *= scale;
        property.remaining_before_wrap *= scale;
        self.time = self.children[first].time() * scale;
    }
    pub fn set_attributes(&mut self, attributes: &[SettableAttribute]) -> Result<bool, String> {
        let mut changed = false;
        for attribute in attributes {
            if let Some(i) = self.parameters.iter().position(|&n| n == attribute.name) {
                if !attribute.value.is_finite() {
                    return Err("Nonfinite BlendSpace parameter".into());
                }
                self.values[i] = attribute.value;
                changed = true;
            }
        }
        if !changed {
            return Ok(false);
        }
        let mut selected = None;
        let mut closest = f32::MAX;
        for (index, simplex) in self.simplexes.iter().enumerate() {
            let mut weights = simplex.coordinates(&self.values);
            if weights.iter().all(|&w| w >= 0.) {
                selected = Some((index, weights));
                break;
            }
            weights = simplex.coordinates(&simplex.project(&self.values));
            for w in &mut weights {
                if *w < 0. {
                    *w = 0.;
                }
            }
            normalize(&mut weights)?;
            let mut distance = 0.;
            for (j, &target) in self.values.iter().enumerate() {
                let projected = simplex
                    .vertices
                    .iter()
                    .zip(&weights)
                    .fold(0., |sum, (v, &w)| v[j].mul_add(w, sum));
                distance = (projected - target).mul_add(projected - target, distance);
            }
            if distance < closest {
                closest = distance;
                selected = Some((index, weights));
            }
        }
        let (index, mut weights) = selected.ok_or("BlendSpace has no finite projection")?;
        for w in &mut weights {
            *w = w.clamp(0., 1.);
        }
        normalize(&mut weights)?;
        if index != self.current {
            let old = &self.children[self.simplexes[self.current].children[0]];
            let phase = old.time() / old.length();
            for &i in &self.simplexes[index].children {
                let child = &mut self.children[i];
                child.set_time(phase * child.length());
            }
            self.current = index;
        }
        self.weights = weights;
        Ok(true)
    }
    pub fn attributes(&self, mask: u32) -> Result<Vec<AnimationAttribute>, String> {
        let indices = &self.simplexes[self.current].children;
        let mut output = self.children[indices[0]].attributes(mask)?;
        for a in &mut output {
            playback_attributes::scale(a, self.weights[0])?;
        }
        for (slot, &i) in indices.iter().enumerate().skip(1) {
            let right = self.children[i].attributes(mask)?;
            let mut next = 0;
            let mut combined = Vec::new();
            for mut a in output {
                while next < right.len() && right[next].name.0 < a.name.0 {
                    next += 1;
                }
                if let Some(b) = right.get(next).filter(|b| b.name == a.name) {
                    playback_attributes::add_weighted(&mut a, b, self.weights[slot])?;
                    combined.push(a);
                }
            }
            output = combined;
        }
        Ok(output)
    }
    pub fn query_attribute(
        &self,
        name: AttributeName,
        mask: u32,
        output: &mut AnimationAttribute,
    ) -> Result<bool, String> {
        //82D24580 indexes the first d+1 authored children directly (unlike
        //GetAttributes, which follows the active simplex). Preserve that distinction.
        if !self.children[0].query_attribute(name, mask, output)? {
            return Ok(false);
        }
        playback_attributes::scale(output, self.weights[0])?;
        for i in 1..self.weights.len() {
            let mut right = MotionGraphAttribute { name, value: 0. }.to_animation();
            if !self.children[i].query_attribute(name, 15, &mut right)? {
                return Ok(false);
            }
            playback_attributes::add_weighted(output, &right, self.weights[i])?;
        }
        Ok(true)
    }
    pub fn evaluate(
        &mut self,
        parameters: Evaluation,
        enabled: bool,
        output: &mut Vec<PoseCommand>,
    ) -> Result<bool, String> {
        if !enabled {
            return Ok(false);
        }
        for &i in &self.simplexes[self.current].children {
            if !self.children[i].evaluate(parameters, true, output)? {
                return Err("BlendSpace child produced no pose".into());
            }
        }
        output.push(PoseCommand::WeightedBlend {
            weights: self.weights.clone(),
        });
        Ok(true)
    }
}
fn normalize(weights: &mut [f32]) -> Result<(), String> {
    let sum: f32 = weights.iter().sum();
    if !sum.is_finite() || sum <= 0. {
        return Err("Degenerate BlendSpace weights".into());
    }
    let inverse = 1. / sum;
    for w in weights {
        *w *= inverse;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{playback_clip::PlaybackClip, skeleton_input::name::encode};
    fn triangle() -> Simplex {
        Simplex {
            children: vec![0, 1, 2],
            vertices: vec![vec![0., 0.], vec![1., 0.], vec![0., 1.]],
            normals: vec![vec![-1., -1.], vec![1., 0.], vec![0., 1.]],
            scales: vec![1.; 3],
        }
    }
    #[test]
    fn bump_space_planes_and_original_outside_projection() {
        let s = triangle();
        assert_eq!(s.coordinates(&[0.25, 0.5]), vec![0.25, 0.25, 0.5]);
        assert_eq!(s.project(&[2., 0.5]), vec![1., 0.]);
        assert_eq!(s.project(&[0.5, 2.]), vec![0.5, 0.5]);
        assert_eq!(s.project(&[-1., -1.]), vec![0., 0.]);
    }
    #[test]
    fn bump_space_clocks_share_phase_and_keep_all_weighted_poses() {
        let children = (1..=3)
            .map(|i| PlaybackTree::Clip {
                name: format!("CLIP{i}"),
                clip: PlaybackClip::new((i * 30 + 1) as f32, 30., 1., 0, Vec::new()),
            })
            .collect();
        let x = encode(b"X");
        let y = encode(b"Y");
        let mut tree = BlendSpace::new(vec![x, y], children, vec![triangle()]).unwrap();
        tree.set_attributes(&[
            SettableAttribute {
                name: x,
                value: 0.25,
                normalized: false,
                sequence_id: -1,
            },
            SettableAttribute {
                name: y,
                value: 0.5,
                normalized: false,
                sequence_id: -1,
            },
        ])
        .unwrap();
        let length = tree.length();
        tree.set_time(length * 0.2);
        let mut property = AdvanceResult {
            crossed_end: false,
            overshoot: -1.,
            remaining_before_wrap: -1.,
        };
        tree.advance(length * 0.1, 0., &mut property);
        for child in &tree.children {
            assert!((child.time() / child.length() - 0.3).abs() < 1e-6);
        }
        let mut commands = Vec::new();
        tree.evaluate(
            Evaluation {
                cull_threshold: 0.01,
                update_history: true,
            },
            true,
            &mut commands,
        )
        .unwrap();
        assert_eq!(commands.len(), 4);
        assert_eq!(
            commands.last(),
            Some(&PoseCommand::WeightedBlend {
                weights: vec![0.25, 0.25, 0.5]
            })
        );
    }
}
