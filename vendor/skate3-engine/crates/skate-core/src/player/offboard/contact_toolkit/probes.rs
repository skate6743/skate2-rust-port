//! Original descriptors82D856C8 and query geometry82D811C8.
use super::{Vector, cross};
use crate::air::trajectory::{QueryRequest, Trajectory};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub position: Vector,        // toolkit0
    pub forward: Vector,         //16
    pub up: Vector,              //32
    pub right: Vector,           //48
    pub velocity: Vector,        //64
    pub animation_up: Vector,    //80
    pub animation_right: Vector, //96
}
impl Input {
    pub fn from_vectors(v: [Vector; 7]) -> Self {
        Self {
            position: v[0],
            forward: v[1],
            up: v[2],
            right: v[3],
            velocity: v[4],
            animation_up: v[5],
            animation_right: v[6],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineProbe {
    pub start: Vector,
    pub end: Vector,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Descriptor {
    pub line: LineProbe,
    pub forward_index: usize,
    pub reverse_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeLayout {
    pub trajectories: [LineProbe; 3],
    pub secondary: Vec<Descriptor>,
    pub primary: Vec<Descriptor>,
}
#[derive(Clone, Debug)]
pub struct Batch {
    pub input: Input,
    pub matching_group: i32,
    /// Original request mesh184 rejection mask. Not a surface-material mask.
    pub mesh_reject_mask: u32,
    pub trajectories: [QueryRequest; 3],
    pub lines: Vec<LineProbe>,
    pub secondary: Vec<Descriptor>,
    pub primary: Vec<Descriptor>,
}
fn probe(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, radius: f32) -> LineProbe {
    LineProbe {
        start: [x0, y0, z0, 0.],
        end: [x1, y1, z1, 0.],
        radius,
    }
}
impl ProbeLayout {
    pub fn stock() -> Self {
        // Live TU3 data8303744C = 3f4ccccd (0.8); not a tuned foot height.
        let trajectories = [
            probe(0., 1.2, 0., 0., -0.8, 0., 0.01),
            probe(-0.12, 0.8, 0., -0.17, -0.2, 0., 0.03),
            probe(0.12, 0.8, 0., 0.17, -0.2, 0., 0.03),
        ];
        let mut secondary = Vec::new();
        let mut index = 0;
        let mut z = 0.037500001_f32;
        loop {
            secondary.push(Descriptor {
                line: probe(0., 0.8, z, 0., -0.8, z, 0.),
                forward_index: index,
                reverse_index: None,
            });
            index += 1;
            z += 0.075000003;
            if !(z < 1.7624999) {
                break;
            }
        }
        let mut primary = Vec::new();
        for i in 0..12 {
            // Preserve the source's two products and subtraction.
            let fraction = i as f32 * 0.090909094;
            let height = fraction * 0.8 - (1. - fraction) * 0.8;
            let reverse_index = if height < 0. { Some(index + 1) } else { None };
            primary.push(Descriptor {
                line: probe(0., height, 0., 0., height, 1.8, 0.),
                forward_index: index,
                reverse_index,
            });
            index += if reverse_index.is_some() { 2 } else { 1 };
        }
        for i in 0..3 {
            let height = (i as f32 * 2. + 1.) * 0.099999994 + 0.8;
            primary.push(Descriptor {
                line: probe(0., height, 0.099999994, 0., height, 1.6999999, 0.099999994),
                forward_index: index,
                reverse_index: None,
            });
            index += 1;
        }
        Self {
            trajectories,
            secondary,
            primary,
        }
    }
    pub fn prepare(&self, input: Input, matching_group: i32) -> Batch {
        let surface = [input.right, input.up, input.forward, input.position];
        // 82D81298..12E4 computes animation_right x animation_up.
        let animated = [
            input.animation_right,
            input.animation_up,
            cross(input.animation_right, input.animation_up),
            input.position,
        ];
        let trajectories = std::array::from_fn(|i| {
            let line = transform(
                self.trajectories[i],
                if i == 0 { surface } else { animated },
            );
            QueryRequest {
                trajectory: Trajectory {
                    position: line.start,
                    velocity: super::sub(line.end, line.start),
                    acceleration: [0.; 4],
                    duration: 1.,
                },
                radius: line.radius,
                start_error: 0.,
                end_error: 0.,
            }
        });
        let mut lines = Vec::new();
        let mut prepare = |descriptors: &[Descriptor]| {
            descriptors
                .iter()
                .map(|d| {
                    let line = transform(d.line, surface);
                    lines.push(line);
                    if d.reverse_index.is_some() {
                        lines.push(LineProbe {
                            start: line.end,
                            end: line.start,
                            radius: line.radius,
                        });
                    }
                    Descriptor { line, ..*d }
                })
                .collect()
        };
        let secondary = prepare(&self.secondary);
        let primary = prepare(&self.primary);
        Batch {
            input,
            matching_group,
            mesh_reject_mask: 0x6000,
            trajectories,
            lines,
            secondary,
            primary,
        }
    }
}
fn transform(line: LineProbe, frame: [Vector; 4]) -> LineProbe {
    let point = |v: Vector| {
        std::array::from_fn(|i| {
            frame[2][i].mul_add(
                v[2],
                frame[1][i].mul_add(v[1], frame[0][i].mul_add(v[0], frame[3][i])),
            )
        })
    };
    LineProbe {
        start: point(line.start),
        end: point(line.end),
        radius: line.radius,
    }
}
