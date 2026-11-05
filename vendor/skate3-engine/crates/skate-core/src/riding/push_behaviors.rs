//! Push behavior state and coefficient calculations from Skate 3 TU3.
//! Animation-tree selection and graph transitions belong to the authored graph.
use super::push_animation::{PushAnimationCurves, PushBlendParameters};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PushClipMetrics {
    pub length: f32,
    pub begin_velocity: f32,
    pub end_velocity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PushAttributes {
    /// Native order: low-speed/high-strength, high-speed/high-strength,
    /// low-speed/low-strength, high-speed/low-strength.
    pub clips: [PushClipMetrics; 4],
    pub minimum_begin_velocity: f32,
    pub maximum_begin_velocity: f32,
    pub minimum_delta_velocity: f32,
    pub maximum_delta_velocity: f32,
}

impl PushAttributes {
    /// 82B954D8, after 82B95620 obtains each tree's length and its last
    /// HStr_Vel_B/LStr_Vel_B and Vel_E attributes. No fitted clip coefficients.
    pub fn from_clips(clips: [PushClipMetrics; 4]) -> Self {
        let begin = clips.map(|clip| clip.begin_velocity);
        let delta = clips.map(|clip| clip.end_velocity - clip.begin_velocity);
        Self {
            clips,
            minimum_begin_velocity: native_min(
                native_min(begin[0], begin[1]),
                native_min(begin[2], begin[3]),
            ),
            maximum_begin_velocity: native_max(
                native_max(begin[0], begin[1]),
                native_max(begin[2], begin[3]),
            ),
            minimum_delta_velocity: native_min(
                native_min(delta[0], delta[1]),
                native_min(delta[2], delta[3]),
            ),
            maximum_delta_velocity: native_max(
                native_max(delta[0], delta[1]),
                native_max(delta[2], delta[3]),
            ),
        }
    }

    /// ComputeTargetCoefsFromSpeedAndStrength, 82BAD260. The blend accounts
    /// for differing clip lengths; a linear velocity interpolation is wrong.
    pub fn target(&self, forward_speed: f32, strength: f32) -> PushBlendParameters {
        let speed = bounded(
            nonnegative(forward_speed),
            self.minimum_begin_velocity,
            self.maximum_begin_velocity,
        );
        let strength = bounded(
            strength,
            self.minimum_delta_velocity,
            self.maximum_delta_velocity,
        );
        let [slow_strong, fast_strong, slow_gentle, fast_gentle] = self.clips;
        let strong_speed = bounded(
            speed,
            slow_strong.begin_velocity,
            fast_strong.begin_velocity,
        );
        let gentle_speed = bounded(
            speed,
            slow_gentle.begin_velocity,
            fast_gentle.begin_velocity,
        );
        let strong = velocity_blend(
            strong_speed,
            slow_strong.length,
            slow_strong.begin_velocity,
            fast_strong.length,
            fast_strong.begin_velocity,
        );
        let gentle = velocity_blend(
            gentle_speed,
            slow_gentle.length,
            slow_gentle.begin_velocity,
            fast_gentle.length,
            fast_gentle.begin_velocity,
        );
        let strong_length = (1.0 - strong).mul_add(slow_strong.length, fast_strong.length * strong);
        let gentle_length = (1.0 - gentle).mul_add(slow_gentle.length, fast_gentle.length * gentle);
        let strong_end = blended_end(strong, slow_strong, fast_strong);
        let gentle_end = blended_end(gentle, slow_gentle, fast_gentle);
        let target_end = bounded(strength + speed, gentle_end, strong_end);
        let strength_blend = velocity_blend(
            target_end,
            gentle_length,
            gentle_end,
            strong_length,
            strong_end,
        );
        PushBlendParameters {
            hstr_vel_b: unit(strong),
            lstr_vel_b: unit(gentle),
            vel_e: unit(strength_blend),
        }
    }

    /// ComputeRepushDeadline::End, 82BADA90. Despite its graph name, this
    /// writes PushState.out_factor, not a time deadline or a graph transition.
    pub fn out_factor(
        &self,
        forward_speed: f32,
        strength: f32,
        speed_weight: f32,
        maximum_out: f32,
    ) -> f32 {
        let first_speed = self.minimum_begin_velocity + self.minimum_delta_velocity;
        let speed = bounded(
            nonnegative(forward_speed),
            first_speed,
            self.maximum_begin_velocity,
        );
        let strength = bounded(
            strength,
            self.minimum_delta_velocity,
            self.maximum_delta_velocity,
        );
        let strength_factor = unit(
            (strength - self.minimum_delta_velocity)
                / (self.maximum_delta_velocity - self.minimum_delta_velocity),
        );
        let speed_factor =
            unit((speed - first_speed) / (self.maximum_begin_velocity - first_speed));
        let combined = strength_factor.mul_add(1.0 - speed_weight, speed_factor * speed_weight);
        let lower = if -combined >= 0.0 { 0.0 } else { combined };
        native_min(maximum_out, lower)
    }
}

fn velocity_blend(
    speed: f32,
    low_length: f32,
    low_velocity: f32,
    high_length: f32,
    high_velocity: f32,
) -> f32 {
    let low_distance = low_length * low_velocity;
    let speed_distance = speed * low_length;
    let sum = high_length.mul_add(high_velocity, speed_distance);
    let numerator = speed_distance - low_distance;
    let denominator = -speed.mul_add(high_length, -(sum - low_distance));
    unit(numerator / denominator)
}
fn blended_end(coefficient: f32, low: PushClipMetrics, high: PushClipMetrics) -> f32 {
    let coefficient = unit(coefficient);
    let high_weight = coefficient * high.length;
    let low_weight = (1.0 - coefficient) * low.length;
    high.end_velocity
        .mul_add(high_weight, low.end_velocity * low_weight)
        / (high_weight + low_weight)
}

fn native_min(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { b } else { a }
}
fn native_max(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { a } else { b }
}
fn bounded(value: f32, minimum: f32, maximum: f32) -> f32 {
    native_min(maximum, native_max(minimum, value))
}
fn nonnegative(value: f32) -> f32 {
    if value >= 0.0 { value } else { 0.0 }
}
fn unit(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PushState {
    pub out_factor: f32,
    pub current_push_dv: f32,
    pub current: PushBlendParameters,
    pub target: PushBlendParameters,
    pub continue_push: bool,
}
impl PushState {
    /// InitPush::Begin, 82BAD5B8, preserves out_factor.
    pub fn initialize_push(&mut self) {
        self.continue_push = false;
        self.current_push_dv = 0.0;
        self.current = PushBlendParameters {
            hstr_vel_b: -1.0,
            lstr_vel_b: -1.0,
            vel_e: -1.0,
        };
        self.target = self.current;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirstPushStrength {
    pub simulated_held_seconds: f32,
    pub push_from_teleport: bool,
    pub first_push: bool,
    pub first_update: bool,
}
impl FirstPushStrength {
    /// Begin82BAD6E8: capture teleport eligibility once per activation.
    pub fn begin(time_since_teleport: f32, teleport_window: f32) -> Self {
        Self {
            simulated_held_seconds: 0.0,
            push_from_teleport: time_since_teleport < teleport_window,
            first_push: true,
            first_update: true,
        }
    }
    /// Update82BAD7A8: the teleport path evaluates the old simulated time
    /// before adding dt. A released first push cannot be resumed here.
    pub fn update(
        &mut self,
        state: &mut PushState,
        curves: &PushAnimationCurves,
        pushing: Option<f32>,
        forward_speed: f32,
        dt: f32,
    ) {
        if pushing.is_none() {
            self.first_push = false;
        }
        if self.first_push || self.push_from_teleport {
            let held = if self.push_from_teleport {
                let held = self.simulated_held_seconds;
                self.simulated_held_seconds = dt + self.simulated_held_seconds;
                held
            } else {
                pushing.unwrap_or(0.0)
            };
            let strength = curves.strength(held, nonnegative(forward_speed));
            if strength > state.current_push_dv {
                state.current_push_dv = strength;
            }
        }
        self.first_update = false;
    }
}

/// A snapshot of the authored MG intent list, not raw controller buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushIntents {
    pub pushing: Option<f32>,
    pub new_push: bool,
    /// Presence of the configured newPushName (RightPush or LeftPush in stock).
    pub configured_foot: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushCyclePhase {
    HoldingFirst,
    WaitingNext,
    HoldingNext,
    NextReleased,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PushCycle {
    pub phase: PushCyclePhase,
    pub next_push_dv: f32,
}
impl PushCycle {
    /// Begin82BADCD0.
    pub fn begin(
        state: &mut PushState,
        curves: &PushAnimationCurves,
        intents: PushIntents,
    ) -> Self {
        let holding = intents.pushing.is_some() && intents.configured_foot;
        state.continue_push = holding;
        Self {
            phase: if holding {
                PushCyclePhase::HoldingFirst
            } else {
                PushCyclePhase::WaitingNext
            },
            next_push_dv: curves.button_time_to_dv.evaluate(0.0),
        }
    }
    /// Update82BADE38. Separate phase checks intentionally allow the release
    /// and next-press path to run in one graph update, as in the recovered code.
    pub fn update(
        &mut self,
        state: &mut PushState,
        curves: &PushAnimationCurves,
        intents: PushIntents,
        forward_speed: f32,
        maximum_holding_acceleration: f32,
    ) {
        let speed = nonnegative(forward_speed);
        if self.phase == PushCyclePhase::HoldingFirst {
            if let Some(held) = intents.pushing {
                self.next_push_dv = curves.strength(held, speed);
                if self.next_push_dv > maximum_holding_acceleration {
                    self.next_push_dv = maximum_holding_acceleration;
                    state.current_push_dv = maximum_holding_acceleration;
                }
                if state.current_push_dv < self.next_push_dv {
                    state.current_push_dv = self.next_push_dv;
                }
            } else {
                self.phase = PushCyclePhase::WaitingNext;
                self.next_push_dv = curves.button_time_to_dv.evaluate(0.0);
                state.continue_push = false;
            }
        }
        if self.phase == PushCyclePhase::WaitingNext && intents.new_push && intents.configured_foot
        {
            self.phase = PushCyclePhase::HoldingNext;
        }
        if self.phase == PushCyclePhase::HoldingNext && !intents.configured_foot {
            self.phase = PushCyclePhase::NextReleased;
            state.current_push_dv = self.next_push_dv;
        }
        if self.phase == PushCyclePhase::HoldingNext {
            self.next_push_dv = curves.strength(intents.pushing.unwrap_or(0.0), speed);
            if state.current_push_dv < self.next_push_dv {
                state.current_push_dv = self.next_push_dv;
            }
            state.continue_push = true;
        }
    }
}
/// Physics outputs consumed by GetFootPos82595B28. All positions are world
/// space; deck axes come from the live reckoning transform, not a render pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PushFootFrame {
    pub left_foot: [f32; 4],
    pub right_foot: [f32; 4],
    pub deck_position: [f32; 4],
    pub deck_y: [f32; 4],
    pub deck_z: [f32; 4],
    pub skateboard_flipped: bool,
}
impl PushFootFrame {
    /// TU382595B28. Reuses the isolated, approximation-derived Xenon dot
    /// lowering; independent hardware arithmetic validation remains pending.
    pub fn out_distance(&self, right_foot: bool) -> [f32; 2] {
        let foot = if right_foot {
            self.right_foot
        } else {
            self.left_foot
        };
        let relative = std::array::from_fn(|i| foot[i] - self.deck_position[i]);
        let z = if self.skateboard_flipped {
            self.deck_z.map(|v| -v)
        } else {
            self.deck_z
        };
        [
            crate::physics::native_arithmetic::dot3(relative, z),
            crate::physics::native_arithmetic::dot3(relative, self.deck_y),
        ]
    }
}

#[cfg(test)]
#[path = "tests/push_behaviors.rs"]
mod tests;
