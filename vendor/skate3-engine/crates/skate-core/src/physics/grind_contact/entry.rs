//! TU3 82D86F00 airborne entry and 82D872B8 impact admission.
//! The caller retains this result until Grind::PreUpdate82D40AF8 consumes it.
use super::{V, admission::engagement_slope_sine, arithmetic, dot3};
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct Input<'a> {
    pub valid: bool,
    pub kind: u32,
    pub category: u32,
    pub previous_state_2504: u32,
    pub speed: f32,
    pub balance_2720: f32,
    pub direction: V,
    pub normal: V,
    pub up: V,
    pub board_velocity: V,
    pub air_velocity: V,
    pub surface_kind: u32,
    pub high_side: V,
    /// physics_grinds/VertEngagementHelpVsVelY, X144/Y160.
    pub vertical_help: &'a PointGraph<4>,
    /// Stock wipeout collection layout256, checked by82D872B8.
    pub max_delta: f32,
    pub flags: u32,
    pub previous_entry_velocity: V,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub valid: bool,
    pub flags: u32,
    pub entry_velocity: V,
    pub impact_speed: f32,
    /// Wipeout reason offsets minus20. Each request also sets input2468 bit18.
    pub wipeout_reasons: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Engagement {
    pub tipslide_frames: u32,
}

impl Engagement {
    ///82D86DE8: retain flags/velocity when invalid; reset impact every update.
    pub fn update(&mut self, input: Input<'_>) -> Output {
        self.tipslide_frames = if input.valid
            && input.kind == 2
            && matches!(input.previous_state_2504, 701 | 403)
            && input.speed > 1.8
        {
            8
        } else {
            let next = self.tipslide_frames.wrapping_sub(1);
            if (next as i32) < 0 { 0 } else { next }
        };
        let mut out = Output {
            valid: input.valid,
            flags: (input.flags & !0x0200_0000)
                | if self.tipslide_frames > 0 {
                    0x0200_0000
                } else {
                    0
                },
            entry_velocity: input.previous_entry_velocity,
            impact_speed: 0.,
            wipeout_reasons: Vec::new(),
        };
        if !out.valid {
            return out;
        }
        out.flags &= !0x4000_0000;
        if !matches!(input.category, 100 | 200) {
            return out;
        }
        if dot3(input.up, input.direction).abs() > 0.5 {
            out.valid = false;
            if input.category == 200 {
                out.wipeout_reasons.push(13);
            }
            return out;
        }
        out.flags |= 0x4000_0000;
        let initial = if input.category == 100 {
            input.board_velocity
        } else {
            input.air_velocity
        };
        out.entry_velocity = if input.category == 100 {
            grounded_velocity(
                input.kind,
                input.direction,
                input.board_velocity,
                input.balance_2720,
                input.speed,
                input.vertical_help,
            )
        } else {
            airborne_velocity(
                input.kind,
                input.direction,
                input.normal,
                input.air_velocity,
            )
        };
        let delta = core::array::from_fn(|i| out.entry_velocity[i] - initial[i]);
        out.impact_speed = arithmetic::square_root(dot3(delta, delta));
        // MeasureEngagement runs for BOTH ground and air accepted entries.
        if out.impact_speed > input.max_delta {
            out.wipeout_reasons.push(9);
        }
        let along = dot3(input.board_velocity, input.direction);
        let mut transverse =
            core::array::from_fn(|i| input.direction[i] * along - input.board_velocity[i]);
        transverse[1] = 0.;
        if dot3(transverse, transverse) > 49.
            && (input.surface_kind == 0 || dot3(input.board_velocity, input.high_side) > 0.)
        {
            out.wipeout_reasons.push(14);
        }
        if !out.wipeout_reasons.is_empty() {
            out.valid = false;
        }
        out
    }
}

pub fn grounded_velocity(
    kind: u32,
    direction: V,
    velocity: V,
    balance: f32,
    speed: f32,
    vertical_help: &PointGraph<4>,
) -> V {
    let base = match kind {
        1 => 0.1,
        2 | 4 if balance != 0. && speed < 1.45 => 1.,
        2 => 0.4,
        0 | 3 | 4 | 5 => 0.2,
        _ => 0.,
    };
    let help = vertical_help.evaluate(engagement_slope_sine(direction, velocity));
    let amount = help.mul_add(1. - base, base);
    // Native fsel on (1-amount), including unordered selecting1.
    let amount = if 1. - amount >= 0. { amount } else { 1. };
    let along = dot3(velocity, direction);
    core::array::from_fn(|i| (direction[i] * along).mul_add(amount, velocity[i] * (1. - amount)))
}

pub fn airborne_velocity(kind: u32, direction: V, normal: V, velocity: V) -> V {
    let across = cross(normal, direction);
    let removed = across.map(|v| v * dot3(velocity, across));
    let amount = match kind {
        0 | 3 => 0.4,
        1 | 5 => 0.2,
        2 | 4 => 0.8,
        _ => 0.0,
    };
    core::array::from_fn(|i| {
        (velocity[i] - removed[i]).mul_add(amount, velocity[i] * (1.0 - amount))
    })
}

/// Native reason indices: byte33 steep entry,29 excessive correction,34
/// excessive horizontal impact. Retain both impact requests when both fire.
pub fn airborne_rejections(
    direction: V,
    up: V,
    board_velocity: V,
    air_velocity: V,
    corrected: V,
    surface_kind: u32,
    surface_side: V,
    max_delta: f32,
) -> Vec<usize> {
    if dot3(up, direction).abs() > 0.5 {
        return vec![13];
    }
    let delta = core::array::from_fn(|i| corrected[i] - air_velocity[i]);
    let mut reasons = Vec::new();
    if arithmetic::square_root(dot3(delta, delta)) > max_delta {
        reasons.push(9);
    }
    let along = dot3(board_velocity, direction);
    let mut transverse = core::array::from_fn(|i| direction[i] * along - board_velocity[i]);
    transverse[1] = 0.0;
    if dot3(transverse, transverse) > 49.0
        && (surface_kind == 0 || dot3(board_velocity, surface_side) > 0.0)
    {
        reasons.push(14);
    }
    reasons
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.0,
    ]
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod tests;
