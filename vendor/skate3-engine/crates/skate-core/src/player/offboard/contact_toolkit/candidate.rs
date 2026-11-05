//! Candidate record82D82380 and slope-projected approaches82D84D38/82D85070.
use super::analyzer_math::{intersection, madd, tangent};
use super::profile::Profile;
use super::{Input, UP, Vector, ZERO, dot, length, sub};
#[derive(Clone, Copy, Debug)]
pub(super) struct Candidate {
    pub position: Vector,
    pub normal: Vector,
    pub direction: Vector,
    pub flags: u32,
    pub low: f32,
    pub high: f32,
    pub order: f32,
    pub segment: i32,
    pub kind: u32,
}
impl Default for Candidate {
    fn default() -> Self {
        Self {
            position: ZERO,
            normal: UP,
            direction: ZERO,
            flags: 0,
            low: 0.,
            high: 0.,
            order: 0.,
            segment: 0,
            kind: 0,
        }
    }
}
pub(super) struct Builder<'a> {
    pub input: Input,
    pub profile: &'a Profile,
    pub obstruction: f32,
    pub candidates: Vec<Candidate>,
}
impl Builder<'_> {
    pub fn add(
        &mut self,
        position: Vector,
        normal: Vector,
        direction: Vector,
        segment: i32,
        flags: u32,
        kind: u32,
    ) {
        let order = dot(self.input.forward, sub(position, self.input.position));
        let mut c = Candidate {
            position,
            normal,
            direction,
            segment,
            flags,
            kind,
            order,
            low: order,
            high: order,
        };
        if segment < 0 {
            c.low = 0.;
        } else {
            let s = self.profile.segments[segment as usize];
            if s.kind != 0 {
                let point = if s.kind == 2 { s.start } else { s.end };
                let distance = dot(self.input.forward, sub(point, self.input.position));
                if distance >= order {
                    c.high = distance;
                } else {
                    c.low = distance;
                }
            }
        }
        self.candidates.push(c);
    }
    pub fn point(&mut self, position: Vector, normal: Vector, segment: i32, flags: u32, kind: u32) {
        let delta = sub(position, self.input.position);
        self.add(
            position,
            normal,
            if length(delta) >= 0.02 {
                delta
            } else {
                self.input.forward
            },
            segment,
            flags,
            kind,
        );
    }
    fn local(&self, p: Vector) -> [f32; 2] {
        let d = sub(p, self.input.position);
        [dot(self.input.forward, d), dot(self.input.up, d)]
    }
    pub fn approach_up(&mut self, index: usize, flags: u32, slope: f32) {
        let p = self.local(self.profile.segments[index].end);
        let goal = [-1., (-1. - p[0]).mul_add(slope, p[1])];
        for i in (0..index).rev() {
            let s = self.profile.segments[i];
            let t = intersection(self.local(s.start), self.local(s.end), p, goal);
            if t < 0. {
                if i == 0 {
                    self.point(self.input.position, self.input.up, index as i32, flags, 1);
                    return;
                }
            } else if t <= 1. {
                let (position, normal) = if s.kind == 0 {
                    (madd(s.direction, s.length * t, s.start), s.normal)
                } else {
                    (s.start, self.next_normal(i))
                };
                if self.local(position)[0] <= 0.75 {
                    self.point(position, normal, index as i32, flags, 1);
                }
                return;
            }
        }
    }
    pub fn approach_down(&mut self, index: i32, flags: u32, slope: f32, kind: u32) {
        let point = if index >= 0 {
            self.profile.segments[index as usize].start
        } else {
            self.input.position
        };
        let p = self.local(point);
        let goal = [2.8, (2.8 - p[0]).mul_add(slope, p[1])];
        for i in (index + 1) as usize..self.profile.last_ground {
            let s = self.profile.segments[i];
            let t = intersection(self.local(s.start), self.local(s.end), p, goal);
            if t >= 0. && t <= 1. {
                if s.kind == 0 {
                    self.point(
                        madd(s.direction, s.length * t, s.start),
                        s.normal,
                        index,
                        flags,
                        kind,
                    );
                } else if self.local(s.end)[0] <= self.obstruction - 0.35 {
                    self.point(s.end, self.next_normal(i), index, flags, kind);
                }
                return;
            }
        }
    }
    pub fn advance(&mut self, mut distance: f32) {
        for i in 0..self.profile.last_ground {
            let s = self.profile.segments[i];
            if s.kind == 2 {
                distance += s.length / down_slope(self.input, s.length);
                continue;
            }
            distance -= s.length;
            if distance > 0. {
                continue;
            }
            if s.kind == 1 {
                self.point(s.end, self.next_normal(i), i as i32, 0, 9);
            } else {
                self.point(madd(s.direction, distance, s.end), s.normal, i as i32, 0, 9);
            }
            return;
        }
    }
    pub fn next_normal(&self, i: usize) -> Vector {
        if i == self.profile.last_ground - 1 {
            self.input.up
        } else {
            self.profile.segments[i + 1].normal
        }
    }
}
pub(super) fn up_slope(input: Input, height: f32) -> f32 {
    let (low, high) = if height > 0.4 { (35., 50.) } else { (15., 30.) };
    slope(input, low, high)
}
pub(super) fn down_slope(input: Input, height: f32) -> f32 {
    let (low, high) = if height > 0.4 { (30., 50.) } else { (15., 30.) };
    slope(input, low, high)
}
fn slope(input: Input, low: f32, high: f32) -> f32 {
    let speed = length(input.velocity).max(2.).min(6.);
    tangent((((speed - 2.) * (low - high)) * 0.25 + high) * 0.017453292)
}
pub(super) fn between_slope(input: Input, a: Vector, b: Vector) -> f32 {
    let d = sub(b, a);
    let x = dot(input.forward, d);
    if x < 0.001 {
        1000.
    } else {
        super::analyzer_math::reciprocal(x) * dot(input.up, d)
    }
}
