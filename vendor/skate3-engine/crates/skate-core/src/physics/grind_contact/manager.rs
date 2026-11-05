//! Manager leaves recovered from original TU3, independent of host storage.
use super::{V, arithmetic, dot3, scale, sub};
use crate::point_graph::PointGraph;

///82D87460, after CalcNormalizedGrindDir82D37048.
pub fn directed_tangent(start: V, end: V, velocity: V, previous: V) -> V {
    let delta = sub(end, start);
    let mut direction = scale(
        delta,
        arithmetic::reciprocal(arithmetic::square_root(dot3(delta, delta))),
    );
    let speed = dot3(direction, velocity);
    if speed < 0. {
        direction = scale(direction, -1.);
    }
    if speed.abs() < 0.1 && dot3(previous, direction) < -0.9 {
        direction = scale(direction, -1.);
    }
    direction
}

#[derive(Clone, Copy, Debug)]
pub struct GeometryInput {
    pub valid: bool,
    pub family: u32,
    pub current_state: u32,
    pub category: u32,
    pub air_frames: u32,
    pub geometry_kind: u32,
    pub geometry_flags: u32,
    pub far_points: [V; 2],
    pub upmost: V,
    pub high_side: V,
    pub point: V,
    pub direction: V,
    pub board_position: V,
    pub board_forward: V,
    pub velocity: V,
    pub deck_to_truck: f32,
    pub previous_exit_angle: f32,
    pub previous_exit_direction: V,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct GeometryOutput {
    pub valid: bool,
    pub family: u32,
    pub flags: u32,
}

///82D880D0, with the retained one-truck exclusion owned by the investigator.
pub fn tweak_geometry(i: GeometryInput, avoid_five_o_frames: &mut u32) -> GeometryOutput {
    let mut o = GeometryOutput {
        valid: i.valid,
        family: i.family,
        flags: i.flags,
    };
    if i.geometry_kind == 3
        || i.family == 2
            && i.geometry_flags & 0x1000_0000 != 0
            && i.category != 400
            && i.air_frames < 20
        || i.family == 1 && i.geometry_flags & 0x1000_0000 != 0
    {
        o.valid = false;
    }
    if o.valid && is_backslash(&i) {
        o.family = 4;
    }
    if o.valid
        && i.previous_exit_angle > 0.
        && i.geometry_kind != 2
        && dot3(i.previous_exit_direction, i.high_side) < 0.
    {
        o.valid = false;
    }
    if o.valid && i.geometry_kind != 0 {
        let mut transverse = sub(
            scale(i.direction, dot3(i.direction, i.velocity)),
            i.velocity,
        );
        transverse[1] = 0.;
        if dot3(transverse, transverse) > 12.25 && dot3(i.velocity, i.high_side) < 0. {
            o.valid = false;
        }
    }
    // Source executes this branch even if an earlier check rejected validity.
    if o.family == 3 && i.geometry_kind == 2 {
        if (*avoid_five_o_frames as i32) > 0 {
            o.valid = false;
        } else {
            let hanging = if o.flags & 0x2000_0000 != 0 {
                scale(i.board_forward, -1.)
            } else {
                i.board_forward
            };
            if dot3(hanging, i.high_side) > 0. {
                let vertical = dot3(hanging, i.upmost);
                o.flags = (o.flags & !0x1000_0000) | if vertical < 0.19 { 0x1000_0000 } else { 0 };
                if vertical < 0. {
                    o.valid = false;
                    *avoid_five_o_frames = 30;
                }
            }
        }
    }
    o
}

///82D883E0: height DIFFERENCE chooses the high-side sample, not two positive heights.
fn is_backslash(i: &GeometryInput) -> bool {
    if i.family != 2 || i.current_state == 402 || i.geometry_flags & 0x8000_0000 != 0 {
        return false;
    }
    let offsets = i.far_points.map(|p| sub(p, i.point));
    let heights = offsets.map(|v| dot3(v, i.upmost));
    if (heights[0] - heights[1]).abs() <= i.deck_to_truck * 0.1 {
        return false;
    }
    let high = if heights[0] > heights[1] {
        offsets[0]
    } else {
        offsets[1]
    };
    dot3(sub(i.board_position, i.point), high) > 0.
}

///82D8ACF0 / S2 ManageCopingAssistance82DDBC48. Update, then decrement, even
///on the activation frame. FrictionVsTime is a different pre-update producer.
pub fn gravity_relief(
    timer: &mut f32,
    valid: bool,
    category: u32,
    tangent: V,
    velocity: V,
    dt: f32,
    vertical: &PointGraph<4>,
    linear: &PointGraph<4>,
) -> f32 {
    if valid && category == 100 && super::admission::engagement_slope_sine(tangent, velocity) > 0.85
    {
        *timer = vertical.evaluate(velocity[1]) * linear.evaluate(dot3(tangent, velocity).abs());
    }
    let next = *timer - dt;
    *timer = if -next >= 0. { 0. } else { next };
    *timer
}

#[derive(Clone, Copy, Debug)]
pub struct JumpGeometry {
    pub geometry_kind: u32,
    pub high_side: V,
    pub normal: V,
    pub direction: V,
    pub upmost: V,
    pub point: V,
}

#[derive(Clone, Copy, Debug)]
pub struct Jumper {
    pub launched: bool,
    pub cooldown: u32,
    pub family: u32,
    pub energy: f32,
    pub geometry: JumpGeometry,
}

impl Default for Jumper {
    ///82D8A318; original vector constants82139A10/82139A20 checked in memory.
    fn default() -> Self {
        Self {
            launched: false,
            cooldown: 0,
            family: 3,
            energy: 1.,
            geometry: JumpGeometry {
                geometry_kind: 0,
                high_side: [1., 0., 0., 0.],
                normal: [0., 1., 0., 0.],
                direction: [1., 0., 0., 0.],
                upmost: [0., 1., 0., 0.],
                point: [0.; 4],
            },
        }
    }
}

impl Jumper {
    ///82D739D8: invalid investigation retains geometry. Return a flag to OR,
    ///not a replacement for the complete processed flags2476 word.
    pub fn update(&mut self, geometry: Option<JumpGeometry>, published_family: u32) -> u32 {
        self.launched = false;
        let next = self.cooldown.wrapping_sub(1);
        self.cooldown = if (next as i32) < 0 { 0 } else { next };
        if let Some(geometry) = geometry {
            self.geometry = geometry;
        }
        if published_family != u32::MAX {
            self.family = published_family;
        }
        let energy = self.energy + 0.0035;
        self.energy = if 1. - energy >= 0. { energy } else { 1. };
        if self.cooldown > 0 { 0x0200_0000 } else { 0 }
    }
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
