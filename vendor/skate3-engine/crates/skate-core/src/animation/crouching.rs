//! Stock riding crouch/absorption and auto-pump, TU3 82BB7DA0/82BB8818.
use crate::point_graph::PointGraph;

#[derive(Clone, Debug)]
pub struct Settings {
    pub maximum_height: PointGraph<8>,
    pub absorption_upforce: PointGraph<8>,
    pub pump_maxspeed: PointGraph<8>,
    pub minimum_height: f32,
    pub maximum_ratio: f32,
    pub maximum_delta_delta: f32,
    pub maximum_delta: f32,
    pub input_blend: f32,
    pub pump_vertical_speed: f32,
    pub maximum_crouch_from_deck: f32,
    pub skateboard_damping: f32,
    pub maximum_force: f32,
    pub maximum_ground_force: f32,
    pub absorption_factor: f32,
    pub auto_pump: AutoPumpSettings,
}
#[derive(Clone, Debug)]
pub struct AutoPumpSettings {
    pub maximum_crouch: PointGraph<4>,
    pub sufficient_crouch: f32,
    pub standing_threshold: f32,
    pub rise_speed: f32,
    pub pump_speed: f32,
    pub potential_threshold: f32,
    pub potential_blend: f32,
    pub intent_magnitude_start: f32,
    pub intent_angle_region: f32,
    pub crouch_time: f32,
    pub crouch_speed: f32,
}
/// Resolved physical publication fields. Offsets stay explicit until the
/// producer's domain identity is verified; callers must publish actual values.
#[derive(Clone, Copy, Debug)]
pub struct Physical {
    pub body_84: f32,
    pub body_164: f32,
    pub body_188: f32,
    pub force_516: f32,
    pub ground_force_520: f32,
    pub minimum_crouch_528: f32,
    pub deck_angle_532: f32,
    pub animation_height_72: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Intents {
    pub auto_pump_angle: Option<f32>,
    pub auto_pump_magnitude: Option<f32>,
    pub crouch: Option<f32>,
    pub hard_turn_crouch: Option<f32>,
    pub manual: Option<f32>,
    /// SkaterMotionGraph interface+108, used by both manual-crouch branches.
    pub motion_flag_108: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct State {
    absorbed_velocity: f32,
    filtered_input: f32,
    pub fraction: f32,
    delta_fraction: f32,
    player_pumping: bool,
    auto_fraction: f32,
    auto_state: u32,
    auto_time: f32,
    previous_angle: f32,
    previous_magnitude: f32,
    filtered_potential: f32,
    pump_idle_time: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub height: f32,
    pub new_auto_pump: bool,
    pub player_controlled_pump: bool,
}
impl State {
    pub fn begin(physical: Physical, settings: &Settings) -> Self {
        let maximum = settings.maximum_height.evaluate(physical.body_164);
        let fraction = 1.0
            - unit(
                (physical.animation_height_72 - settings.minimum_height)
                    / (maximum - settings.minimum_height),
            );
        Self {
            absorbed_velocity: 0.0,
            filtered_input: 0.0,
            fraction,
            delta_fraction: 0.0,
            player_pumping: false,
            auto_fraction: 0.0,
            auto_state: 0,
            auto_time: 0.0,
            previous_angle: 0.0,
            previous_magnitude: 0.0,
            filtered_potential: 0.0,
            pump_idle_time: 0.5,
        }
    }
    pub fn update(&mut self, p: Physical, intents: Intents, dt: f32, s: &Settings) -> Output {
        let restoring = -s.absorption_upforce.evaluate(1.0 - self.fraction);
        let mut board = bound(-p.force_516 / s.maximum_force, -1.0, 1.0);
        self.absorbed_velocity += p.body_84;
        self.absorbed_velocity = s.skateboard_damping * self.absorbed_velocity;
        if self.absorbed_velocity < 0.0 {
            board = -board;
        }
        let ground = bound(-p.ground_force_520 / s.maximum_ground_force, -1.0, 1.0);
        let mut absorption = if board < 0.0 {
            if ground > 0.0 { ground + board } else { board }
        } else {
            maximum(board, ground)
        };
        if absorption < 0.0 {
            if !(absorption < restoring) {
                absorption = restoring;
            }
        } else {
            absorption += restoring;
        }
        let angle = intents.auto_pump_angle.unwrap_or(0.0);
        let magnitude = intents.auto_pump_magnitude.unwrap_or(0.0);
        let a = &s.auto_pump;
        let started = self.previous_angle.abs() < a.intent_angle_region
            && self.previous_magnitude < a.intent_magnitude_start
            && angle.abs() < a.intent_angle_region
            && magnitude > a.intent_magnitude_start;
        self.previous_angle = angle;
        self.previous_magnitude = magnitude;
        self.filtered_potential = p.body_188.mul_add(
            a.potential_blend,
            (1.0 - a.potential_blend) * self.filtered_potential,
        );
        let sufficient_potential = !(self.filtered_potential < a.potential_threshold);
        if started {
            self.auto_time = 0.0;
            self.auto_state = 1;
        }
        if self.auto_state == 1 && self.auto_time > a.crouch_time {
            self.auto_state = 2;
        }
        let new_auto_pump = (self.auto_state == 1 || self.auto_state == 2)
            && self.fraction > a.sufficient_crouch
            && sufficient_potential;
        if new_auto_pump {
            self.auto_state = 3;
        }
        if (self.auto_state == 2 || self.auto_state == 3)
            && !(self.fraction > a.standing_threshold + p.minimum_crouch_528)
        {
            self.auto_state = 0;
        }
        match self.auto_state {
            0 => self.auto_fraction = 0.0,
            1 => {
                self.auto_fraction = minimum(
                    a.maximum_crouch.evaluate(p.body_164),
                    a.crouch_speed + self.auto_fraction,
                );
                self.auto_time = minimum(a.crouch_time + 1.0, dt + self.auto_time);
            }
            2 => self.auto_fraction = nonnegative(self.auto_fraction - a.rise_speed),
            3 => self.auto_fraction = nonnegative(self.auto_fraction - a.pump_speed),
            _ => {}
        }
        let mut crouch = intents.crouch.unwrap_or(0.0);
        if !intents.motion_flag_108 {
            crouch = maximum(crouch, intents.hard_turn_crouch.unwrap_or(0.0));
        }
        if intents.motion_flag_108 && intents.manual.is_some_and(|value| value < 0.0) {
            crouch = maximum(crouch, f32::from_bits(0x3ecccccd));
        }
        let blend = unit(s.input_blend);
        self.filtered_input =
            maximum(crouch, self.auto_fraction).mul_add(blend, (1.0 - blend) * self.filtered_input);
        if self.filtered_input.abs() < f32::from_bits(0x3ba3d70a) {
            self.filtered_input = 0.0;
        }
        let mut delta = s.absorption_factor * absorption;
        if crouch > 0.0
            || self.auto_fraction > 0.0
            || (self.player_pumping && self.fraction > p.minimum_crouch_528)
        {
            self.player_pumping = true;
            delta = maximum(
                minimum(1.0, maximum(p.minimum_crouch_528, self.filtered_input)) - self.fraction,
                -s.maximum_delta,
            );
            self.pump_idle_time = 0.0;
        } else {
            self.player_pumping = false;
            // Native literal820849C8, deliberately not graph delta time.
            self.pump_idle_time += f32::from_bits(0x3c888889);
        }
        let player_controlled_pump = self.pump_idle_time < 0.5;
        let minimum_delta =
            -(s.pump_maxspeed.evaluate(1.0 - self.fraction) * s.pump_vertical_speed);
        let maximum_delta = s.pump_maxspeed.evaluate(self.fraction) * s.pump_vertical_speed;
        let delta = bound(delta, minimum_delta, maximum_delta);
        self.delta_fraction = bound(
            delta,
            self.delta_fraction - s.maximum_delta_delta,
            s.maximum_delta_delta + self.delta_fraction,
        );
        self.fraction = bound(
            self.fraction + self.delta_fraction,
            p.minimum_crouch_528,
            1.0,
        );
        let from_deck = s.maximum_crouch_from_deck * p.deck_angle_532;
        if from_deck > self.fraction {
            self.fraction = from_deck;
        }
        let height_ratio = unit((-s.maximum_ratio).mul_add(self.fraction, 1.0));
        let maximum_height = s.maximum_height.evaluate(p.body_164);
        Output {
            height: (maximum_height - s.minimum_height).mul_add(height_ratio, s.minimum_height),
            new_auto_pump,
            player_controlled_pump,
        }
    }
}
fn minimum(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { b } else { a }
}
fn maximum(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { a } else { b }
}
fn bound(v: f32, lo: f32, hi: f32) -> f32 {
    minimum(hi, maximum(lo, v))
}
fn nonnegative(v: f32) -> f32 {
    if -v >= 0.0 { 0.0 } else { v }
}
fn unit(v: f32) -> f32 {
    minimum(1.0, nonnegative(v))
}
