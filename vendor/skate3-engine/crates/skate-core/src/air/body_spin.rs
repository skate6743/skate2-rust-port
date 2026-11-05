//! Complete TU3 BodySpin::Update (82D8BF58) and history scan (82D8C2D8).
//! Cached curves are explicit inputs, not reconstructed from Skate 2 settings.
use crate::point_graph::PointGraph;

/// Native bytes +0..176. Preserves the unused +136 word and mode-byte padding.
/// Construction/air-entry lifecycle belongs to the recovered caller, not Default.
#[derive(Clone, Debug, PartialEq)]
pub struct BodySpinState {
    words: [u32; 44],
}

impl BodySpinState {
    ///82D8DA98..DAE0 initializes every BodySpin scalar and history entry.
    pub fn new() -> Self {
        Self { words: [0; 44] }
    }
    pub fn from_words(words: [u32; 44]) -> Self {
        assert!(
            words[42] < 30,
            "native derivative history index must be 0..30"
        );
        Self { words }
    }

    pub fn words(&self) -> &[u32; 44] {
        &self.words
    }

    /// +140, consumed by Reckoning::UpdateAirStates after BodySpin::Update.
    pub fn speed(&self) -> f32 {
        self.get(140)
    }

    fn get(&self, offset: usize) -> f32 {
        f32::from_bits(self.words[offset / 4])
    }

    fn set(&mut self, offset: usize, value: f32) {
        self.words[offset / 4] = value.to_bits();
    }
}

/// Native cached inputs at +176/+180, seven 80-byte curves, and 830BD300.
/// The vector threshold's producer must be supplied: no guessed retail default.
pub struct BodySpinSettings {
    pub derivative_floor: f32,
    pub acceleration_limit: f32,
    /// Native +184, +264, +344, +424, +504, +584, +664; x/y at +16/+48.
    pub curves: [PointGraph<8>; 7],
    pub input_fade_threshold: [f32; 4],
}

const STEP: f32 = f32::from_bits(0x3C88_8889); // 820849C8, fixed in these functions.

/// Registers f1/f2/r6/r7, including the native mode-byte publication.
pub fn update(
    state: &mut BodySpinState,
    settings: &BodySpinSettings,
    input: f32,
    auto_spin: f32,
    in_air: bool,
    mode: u8,
) {
    update_input(state, input, auto_spin, mode);
    if !in_air {
        finish_ground(state, input);
        return;
    }

    update_air(state, settings, input, auto_spin, mode);
}

/// Ground caller82D8C8F0 supplies auto_spin=0, in_air=false and mode=0.
/// This branch does not read any of the airborne BodySpin tuning curves.
pub fn update_ground(state: &mut BodySpinState, input: f32) {
    update_input(state, input, 0.0, 0);
    finish_ground(state, input);
}

fn update_input(state: &mut BodySpinState, input: f32, auto_spin: f32, mode: u8) {
    let delta = input - state.get(120);
    let derivative = delta.mul_add(1.5, state.get(124) * 0.0);
    state.set(164, auto_spin);
    state.set(120, input);
    state.words[43] = (state.words[43] & 0x00FF_FFFF) | ((mode as u32) << 24);
    state.set(124, derivative);
    let filtered = if derivative * input > 0.1 {
        derivative
    } else {
        0.0
    };
    let filtered = native_clamp(filtered, -1.0, 1.0);
    state.set(128, filtered);
    let index = state.words[42] as usize;
    state.set(index * 4, filtered);
    state.words[42] = ((index + 1) % 30) as u32;
}

fn finish_ground(state: &mut BodySpinState, input: f32) {
    state.set(152, 0.0);
    state.set(144, 0.0);
    state.set(140, 0.0);
    state.set(132, state.get(132).mul_add(0.8, input * 0.2));
}

fn update_air(
    state: &mut BodySpinState,
    settings: &BodySpinSettings,
    input: f32,
    auto_spin: f32,
    mode: u8,
) {
    let alternate = usize::from(mode == 0);
    if state.get(152) == 0.0 {
        calculate_history_derivative(state, &settings.curves[3 + alternate]);
    }
    let time = state.get(152);
    let normal_acceleration = settings.curves[alternate].evaluate(time);
    let auto_acceleration = settings.curves[2].evaluate(time);
    let derivative_gain = settings.curves[3 + alternate].evaluate(time);
    let candidate = derivative_gain * state.get(128);
    if candidate.abs() > state.get(148).abs() {
        state.set(148, candidate);
        state.set(156, time);
    }
    let next_time = time + STEP;
    state.set(152, next_time);
    // vcmpgtfp128. followed by CR6 all-lanes test, not a scalar dead zone.
    if settings
        .input_fade_threshold
        .iter()
        .all(|&v| v > state.get(120).abs())
    {
        let faded = state.get(132) * 0.96;
        state.set(132, faded);
        state.set(120, faded);
    } else {
        state.set(132, state.get(132).mul_add(0.8, input * 0.2));
    }
    let proportional = settings.curves[5 + alternate].evaluate(next_time) * state.get(120);
    state.set(144, proportional);
    if proportional * state.get(148) < 0.0 {
        state.set(148, 0.0);
    }
    let weight = state
        .get(148)
        .abs()
        .mul_add(1.0 - settings.derivative_floor, settings.derivative_floor);
    let mut target = -(weight * proportional);
    let mut acceleration = normal_acceleration;
    let old_speed = state.get(140);
    if auto_spin.abs() > f32::from_bits(0x3780_0000)
        && (old_speed * auto_spin > 0.0 || old_speed.abs() < 0.02)
    {
        acceleration = auto_acceleration;
        target = native_clamp(auto_spin, -2.0, 2.0);
    }
    let limited = select(
        settings.acceleration_limit - acceleration,
        acceleration,
        settings.acceleration_limit,
    );
    let (lower, upper) = if old_speed > 0.0 {
        (-limited, acceleration)
    } else {
        (-acceleration, limited)
    };
    let change = native_clamp(target - old_speed, lower, upper);
    let speed = old_speed + change;
    state.set(140, speed);
    // Both subtractions are present at 82D8C27C..284.
    state.set(160, (target - speed) - change);
}

/// 82D8C2D8 scans the 29 entries before the current write position.
/// The newest entry starts at negative one native step, with strict peak ties.
fn calculate_history_derivative(state: &mut BodySpinState, graph: &PointGraph<8>) {
    let end = state.words[42] as usize;
    let mut index = (end + 29) % 30;
    let mut time = 0.0;
    let mut peak: f32 = 0.0;
    state.set(148, 0.0);
    state.set(156, 0.0);
    while index != end {
        time -= STEP;
        let candidate = graph.evaluate(time) * state.get(index * 4);
        if candidate.abs() > peak.abs() {
            peak = candidate;
            state.set(156, time);
        }
        index = (index + 29) % 30;
    }
    state.set(148, peak);
}

fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

fn native_clamp(value: f32, lower: f32, upper: f32) -> f32 {
    let value = select(lower - value, lower, value);
    select(upper - value, value, upper)
}
