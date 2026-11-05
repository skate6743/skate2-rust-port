//! Biped cadence producer82D80720 and phase helpers82D7AA18..82D7AFD0.
//! Original TU3, SHA256431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
//! Embed this state once in Biped; callers supply actual movement/animation input.
//! PC floating-point arithmetic is not proven bit-exact Xenon arithmetic.

const DT: f32 = f32::from_bits(0x3c888889);
const EPSILON: f32 = f32::from_bits(0x3a83126f);
type Vector = [f32; 3];

/// Retained Biped720..736; duration is unwritten by constructor82D7AFD8.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BipedPhase {
    pub phase: f32,
    pub rate: f32,
    pub target: f32,
    pub duration: Option<f32>,
    pub forward_target: bool,
}

impl Default for BipedPhase {
    fn default() -> Self {
        Self {
            phase: 0.0,
            rate: 0.0,
            target: -1.0,
            duration: None,
            forward_target: false,
        }
    }
}

/// This is part of the physical Biped owner, not a MotionGraph phase clock.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BipedCadence {
    pub phase: BipedPhase,
    /// Original enum716. Values above every threshold retain the previous value.
    pub locomotion_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CadenceThresholds(pub [f32; 4]);

impl CadenceThresholds {
    ///82D7AFD8: metrics come from actual walk/run/sprint AnimTransZ attributes.
    pub fn from_clip_speeds(walk: f32, run: f32, sprint: f32) -> Self {
        Self([
            f32::from_bits(0x3c23d70a),
            (run + walk) * 0.5,
            (sprint + run) * 0.5,
            f32::from_bits(0x501502f9),
        ])
    }
}

/// Offsets identify original Biped or job-input fields until their upstream
/// physical names are recovered. No field may be filled from a guessed proxy.
#[derive(Clone, Copy, Debug)]
pub struct CadenceInput {
    pub motion_512: Vector,
    pub motion_reference_272: Vector,
    pub reject_axis_400: Vector,
    pub reject_enabled_708: bool,
    pub up_144: Vector,
    pub frame_rows_0_16_32: [Vector; 3],
    pub frame_position_48: Vector,
    pub animation_motion_224: Vector,
    pub requested_duration_288: f32,
    pub requested_phase_296: f32,
    pub suppress_adjustment_353: bool,
    pub contact_flags_176: u32,
    pub contact_point_96: Vector,
}

impl BipedCadence {
    ///82D80720. Called after movement stage82D80548, before common publication.
    pub fn update(&mut self, input: &CadenceInput, thresholds: CadenceThresholds) {
        let mut motion = subtract(input.motion_512, input.motion_reference_272);
        if input.reject_enabled_708 {
            motion = reject(motion, input.reject_axis_400);
        }
        let mut magnitude = length(reject(motion, input.up_144));
        for (index, threshold) in thresholds.0.into_iter().enumerate() {
            if magnitude <= threshold {
                self.locomotion_index = index as u32;
                break;
            }
        }
        if !(input.requested_phase_296 < 0.0) {
            self.phase
                .request(input.requested_phase_296, input.requested_duration_288);
        } else if self.locomotion_index == 0 {
            if !(self.phase.target >= 0.0) {
                self.phase.stop();
            }
        } else {
            let animation = input.animation_motion_224;
            let horizontal = [animation[0], 0.0, animation[2]];
            let horizontal_length = length(horizontal);
            if horizontal_length > EPSILON {
                //82D80A58..AA8 transposes rows, then accumulates x, y, z.
                let local = input.frame_rows_0_16_32.map(|row| {
                    row[2].mul_add(motion[2], row[1].mul_add(motion[1], row[0] * motion[0]))
                });
                let projection = (1.0 / horizontal_length) * dot(local, horizontal);
                magnitude = square_root(local[1].mul_add(local[1], projection * projection));
            }
            let animation_length = length(animation);
            self.phase.rate = if animation_length < EPSILON {
                0.0
            } else {
                magnitude / animation_length
            };
            self.phase.target = -1.0;
            if !input.reject_enabled_708
                && !input.suppress_adjustment_353
                && input.contact_flags_176 & 0x180 != 0
            {
                let time = (length(subtract(input.contact_point_96, input.frame_position_48))
                    - 0.2)
                    / magnitude;
                let lower = self.phase.rate * 0.4;
                let upper = self.phase.rate * 2.5;
                if time > f32::from_bits(0x3c088889) {
                    if input.contact_flags_176 & 0x100 != 0 {
                        let target = if self.phase.phase > 0.37 && self.phase.phase <= 0.87 {
                            0.9
                        } else {
                            0.4
                        };
                        self.phase.adjust_targets(target, target, time, upper);
                    } else {
                        self.phase.adjust_contact(time, lower, upper);
                    }
                }
            }
        }
        self.phase.advance();
    }
}

impl BipedPhase {
    fn request(&mut self, target: f32, mut duration: f32) {
        if duration > 0.1 {
            duration -= 0.1;
        }
        if self.target == target && self.duration == Some(duration) {
            return;
        }
        let delta = target - self.phase;
        self.target = target;
        self.duration = Some(duration);
        self.forward_target = true;
        self.rate = if delta < 0.0 {
            (delta + 1.0) / duration
        } else {
            delta / duration
        };
    }

    fn stop(&mut self) {
        let mut target = 0.0;
        let mut best = f32::from_bits(0x7149f2ca);
        for candidate in [0.0, 0.5] {
            let raw = candidate - self.phase;
            let wrapped = wrap(0.0, raw + 1.0, 1.0);
            let distance = if raw.abs() < wrapped.abs() {
                raw.abs()
            } else {
                wrapped.abs()
            };
            if distance < best {
                target = candidate;
                best = distance;
            }
        }
        self.forward_target = false;
        self.duration = Some(0.2);
        self.target = target;
        let distance = (target - self.phase).abs();
        self.rate = select(distance - (1.0 - distance), 1.0 - distance, distance) * 5.0;
    }

    ///82D7AD90: this fixed retail phase step is also used by Biped Air.
    pub fn advance(&mut self) {
        if self.target < 0.0 {
            let next = self.rate.mul_add(DT, self.phase);
            self.phase = next - next.floor();
        } else if self.forward_target {
            if self.phase == self.target {
                return;
            }
            let goal = if self.target < self.phase {
                self.target + 1.0
            } else {
                self.target
            };
            let old = self.phase;
            let next = self.rate.mul_add(DT, old);
            self.phase = if old < goal && next >= goal {
                self.target
            } else {
                next - next.floor()
            };
        } else {
            let old = self.phase;
            let mut next = old + wrap(-0.5, self.target - old, 0.5);
            let step = self.rate * DT;
            if old > next + step {
                next = old - step;
            } else if old < next - step {
                next = old + step;
            }
            self.phase = wrap(0.0, next, 1.0);
        }
    }

    ///82D7AB48, with82D7AA18's two candidate rates when neither wraps to1.
    fn adjust_targets(&mut self, a: f32, b: f32, time: f32, upper: f32) {
        let da = wrap(0.0, a - self.phase, 1.0);
        let db = wrap(0.0, b - self.phase, 1.0);
        self.rate = if da >= 1.0 {
            clamp(db / time, 0.0, upper)
        } else if db >= 1.0 {
            clamp(da / time, 0.0, upper)
        } else {
            let inverse = 1.0 / time;
            let a = da * inverse;
            let b = db * inverse;
            select_rate(a, b, 0.0, upper, select(a - b, b, a))
        };
    }

    ///82D7AC30 selects the legal contact target closest to the current rate.
    fn adjust_contact(&mut self, time: f32, lower: f32, upper: f32) {
        let inverse = 1.0 / time;
        let a = wrap(0.0, 0.45 - self.phase, 1.0) * inverse;
        let b = wrap(0.0, 0.95 - self.phase, 1.0) * inverse;
        let preferred = if (self.rate - a).abs() < (self.rate - b).abs() {
            a
        } else {
            b
        };
        self.rate = select_rate(a, b, lower, upper, preferred);
    }
}

fn select_rate(a: f32, b: f32, lower: f32, upper: f32, both: f32) -> f32 {
    if a >= lower && a <= upper {
        if b >= lower && b <= upper { both } else { a }
    } else if b >= lower && b <= upper {
        b
    } else {
        let da = if a < lower { lower - a } else { a - upper };
        let db = if b < lower { lower - b } else { b - upper };
        clamp(if da < db { a } else { b }, lower, upper)
    }
}

///82D7AF18: closed endpoints and signed floor count, not rem_euclid.
fn wrap(lower: f32, value: f32, upper: f32) -> f32 {
    let count = if value < lower {
        ((upper - value) / (upper - lower)).floor() as i32
    } else if value > upper {
        (((value - lower) / (upper - lower)).floor() as i32).wrapping_neg()
    } else {
        0
    };
    clamp((upper - lower).mul_add(count as f32, value), lower, upper)
}

fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn clamp(value: f32, lower: f32, upper: f32) -> f32 {
    let value = select(lower - value, lower, value);
    select(upper - value, value, upper)
}
fn subtract(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn reject(vector: Vector, axis: Vector) -> Vector {
    let projection = dot(axis, vector);
    std::array::from_fn(|i| vector[i] - axis[i] * projection)
}
fn length(vector: Vector) -> f32 {
    square_root(dot(vector, vector))
}
fn square_root(squared: f32) -> f32 {
    // Independent PC seed. Preserve both original refinement steps and zero select.
    let mut inverse = squared.sqrt().recip();
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.0), inverse);
    }
    let result = squared * inverse;
    if squared == 0.0 { 0.0 } else { result }
}

#[cfg(test)]
#[path = "cadence/tests.rs"]
mod tests;
