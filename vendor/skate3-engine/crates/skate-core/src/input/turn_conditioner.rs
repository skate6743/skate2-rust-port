//! Complete steering conditioner 82DEFF38, called by82DEFB30.
//! The settings decoder supplies global830CFDA4 slot204's verified layout.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Layout688/704/720, corresponding to conditioner216/180/144.
    pub filter_coefficients: [[f32; 4]; 3],
    /// Layout x304/y336, x240/y272, x32/y64, x384/y400.
    pub input_curve: PointGraph<8>,
    pub quickness_curve: PointGraph<8>,
    pub speed_curve: PointGraph<8>,
    pub smoothing_curve: PointGraph<4>,
    /// Layout768,868,872,896,900,904,908,912,924,928,932,936,940.
    pub parameters: [f32; 13],
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    /// Conditioner112..140, in native order.
    pub history: [f32; 8],
    /// Complete9-word filters at216,180,144; first4 words refresh every call.
    pub filters: [[f32; 9]; 3],
}

impl State {
    /// Steering-history portion of native reset82DEFAA0. Cached coefficients
    /// survive reset; the update refreshes them from the settings layout.
    pub fn reset_history(&mut self) {
        self.history.fill(0.0);
        for filter in &mut self.filters {
            filter[4..].fill(0.0);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub body_160: f32,
    pub body_176: f32,
    pub bundle_36_field_160: f32,
    pub bundle_32_field_264: f32,
    pub animation_152: u8,
    pub animation_156: u8,
}

/// PhysOut_Animation32/36/40/44/48/52/56/60, in native memory order.
pub type Output = [f32; 8];

fn clamp(value: f32, low: f32, high: f32) -> f32 {
    let value = if low - value >= 0.0 { low } else { value };
    if high - value >= 0.0 { value } else { high }
}

fn filter(state: &mut [f32; 9], coefficients: [f32; 4], input: f32) -> f32 {
    state[..4].copy_from_slice(&coefficients);
    let difference = input - state[4];
    let input_difference = input - state[5];
    let complement = 1.0 - state[3];
    state[5] = input;
    state[8] = input_difference.mul_add(state[3], state[8] * complement);
    state[6] = difference.mul_add(state[3], complement * state[6]);
    let term = (state[8] - state[7]).mul_add(state[2], state[1] * state[6]);
    let term = difference.mul_add(state[0], term);
    let result = term + state[4];
    state[7] = result - state[4];
    state[4] = result;
    result
}

pub fn update(state: &mut State, input: Input, settings: &Settings) -> Output {
    let p = settings.parameters;
    let sign = if input.animation_156 != 0 { -1.0 } else { 1.0 };
    let scaled = input.bundle_32_field_264 * sign;
    let speed_fraction = clamp(input.body_160 / p[11], 0.0, 1.0);
    let other_fraction = clamp((input.bundle_36_field_160 / p[10]) * sign, -1.0, 1.0);
    let base = settings.input_curve.evaluate(speed_fraction) * scaled;
    let attenuated = (-(1.0 - p[8])).mul_add(input.body_176, 1.0) * other_fraction;
    let target = attenuated.mul_add(p[9], (1.0 - p[9]) * base);
    let first = filter(
        &mut state.filters[0],
        settings.filter_coefficients[0],
        target,
    );
    let epsilon = f32::from_bits(0x3c23_d70a);
    let first = if first.abs() > epsilon { first } else { 0.0 };
    let second = filter(
        &mut state.filters[1],
        settings.filter_coefficients[1],
        target,
    );
    let second = if second.abs() > epsilon { second } else { 0.0 };
    let third = filter(
        &mut state.filters[2],
        settings.filter_coefficients[2],
        (target - second) / p[12],
    );
    let movement = if input.animation_152 != 0 {
        scaled
    } else {
        0.0
    };
    let difference = movement - state.history[0];
    state.history[0] = movement;
    state.history[2] = difference
        .abs()
        .mul_add(p[2], (1.0 - p[2]) * state.history[2]);
    let ratio = state.history[2] / p[1];
    let squared = ratio * ratio;
    let bounded = if squared - 1.0 >= 0.0 { 1.0 } else { squared };
    state.history[3] = clamp(bounded, state.history[3] - p[0], state.history[3] + p[0]);
    let quick = settings.quickness_curve.evaluate(state.history[3]);
    let quick = clamp(quick, 0.0, 1.0);
    let quick = settings.speed_curve.evaluate(input.body_160) * quick;
    let movement = if input.animation_152 != 0 {
        second
    } else {
        0.0
    };
    let difference = movement - state.history[1];
    state.history[1] = movement;
    state.history[5] = difference
        .abs()
        .mul_add(p[5], (1.0 - p[5]) * state.history[5]);
    let hold: f32 = if state.history[5] < p[3] { 1.0 } else { 0.0 };
    let alpha = (1.0 - hold).mul_add(p[7], p[6] * hold);
    let next = (1.0 - alpha).mul_add(state.history[4], alpha * hold);
    let difference = next - state.history[4];
    state.history[6] = clamp(difference, state.history[6] - p[4], state.history[6] + p[4]);
    state.history[4] = clamp(state.history[4] + state.history[6], 0.0, 1.0);
    let alpha = settings.smoothing_curve.evaluate(input.body_176);
    state.history[7] = (1.0 - alpha).mul_add(state.history[7], alpha * first);
    let turn = clamp(state.history[7], -1.0, 1.0);
    [
        turn,
        clamp(target, -1.0, 1.0),
        base,
        turn * sign,
        base * sign,
        quick,
        clamp(third, -1.0, 1.0),
        clamp(state.history[4], 0.0, 1.0),
    ]
}
