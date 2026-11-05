//! Biped movement producer82D7CF50, steering merge82D7D5A0 and slide82D7D608.
//! Original TU3; state/input offsets are documented at the host boundary.
use crate::point_graph::PointGraph;
mod math;
use math::*;
pub type Vector = [f32; 4];

pub struct Settings {
    /// Biped320 <- physics_biped350, x+16/y+32.
    pub sprint_speed: PointGraph<4>,
    /// Biped5D0 <- physics_biped90, x+16/y+80.
    pub normal_speed: PointGraph<16>,
    /// Biped3F0 <- physics_biped1C0, x+16/y+48; header+8 is cap.
    pub sprint_blend: PointGraph<8>,
    pub sprint_time_cap: f32,
    /// Biped440 <- physics_biped300, x+16/y+48.
    pub slide_steering: PointGraph<8>,
}

/// Retained outputs and timers; native Reset82D7B1C0 initializes these to zero.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub speed: f32,              //756
    pub steering: f32,           //760
    pub sprint_time: f32,        //764
    pub sprint_grace: f32,       //768
    pub secondary_speed: f32,    //776
    pub original_steering: f32,  //780
    pub obstacle_centered: bool, //712
    pub edge_aligned: bool,      //713
    pub edge_target: Vector,     //672
}

pub struct Input {
    pub flags: u32,              //input176
    pub suppress_minimum: bool,  //input318
    pub magnitude: f32,          //input308
    pub steering: f32,           //input312
    pub sprint_pressed: bool,    //input316
    pub edge_active: bool,       //input353; also disables sprint
    pub ignore_obstacle: bool,   //input432
    pub direction: Vector,       //input256
    pub edge_tangent: Vector,    //input400
    pub edge_point: Vector,      //input416
    pub right: Vector,           //Biped0
    pub up: Vector,              //Biped16
    pub forward: Vector,         //Biped32
    pub position: Vector,        //Biped48
    pub obstacle: bool,          //Biped708
    pub obstacle_normal: Vector, //Biped400
    pub sliding: bool,           //Biped710
    pub slide_velocity: Vector,  //Biped528
}

impl State {
    /// Once per original physical tick; source uses literal1/60, not graph dt.
    pub fn update(&mut self, settings: &Settings, input: &Input) {
        let minimum = if !input.suppress_minimum && input.flags & 8 != 0 {
            0.5
        } else {
            0.0
        };
        let magnitude = select(minimum - input.magnitude, minimum, input.magnitude);
        let dt = f32::from_bits(0x3c88_8889);
        self.sprint_grace = if input.sprint_pressed {
            f32::from_bits(0x3e99_999a)
        } else {
            self.sprint_grace - dt
        };
        let sprint = (self.sprint_grace > 0.0 || input.sprint_pressed) && !input.edge_active;
        self.sprint_time = clamp(
            self.sprint_time + if sprint { dt } else { -dt },
            0.0,
            settings.sprint_time_cap,
        );
        let fast = settings.sprint_speed.evaluate(magnitude);
        let normal = settings.normal_speed.evaluate(magnitude);
        let blend = if sprint {
            settings.sprint_blend.evaluate(self.sprint_time)
        } else {
            0.0
        };
        self.speed = (1.0 - blend).mul_add(normal, blend * fast);
        self.secondary_speed = 0.0;
        self.steering = input.steering;
        self.original_steering = input.steering;
        self.edge_aligned = false;
        if input.obstacle && !input.ignore_obstacle && !(self.speed <= 0.0) {
            let opposite = scale(input.obstacle_normal, -1.0);
            let desired = wrap_angle(signed_angle(
                opposite,
                input.direction,
                [0.0, 1.0, 0.0, 0.0],
            ));
            let degrees = if self.obstacle_centered { 55.0 } else { 25.0 };
            self.obstacle_centered = degrees * f32::from_bits(0x3c8e_fa35) > desired.abs();
            let facing = wrap_angle(signed_angle(opposite, input.forward, [0.0, 1.0, 0.0, 0.0]));
            if !(facing.abs() >= HALF_PI) {
                let correction = if self.obstacle_centered {
                    (facing * f32::from_bits(0xbf22_f983)) * 0.5
                } else {
                    (-facing).mul_add(
                        f32::from_bits(0x3f22_f983),
                        if desired <= 0.0 { -1.0 } else { 1.0 },
                    ) * 0.5
                };
                self.merge_steering(clamp(correction, -1.0, 1.0));
                if self.obstacle_centered {
                    self.speed = 0.0;
                }
            }
        } else {
            self.obstacle_centered = false;
            if input.sliding {
                self.apply_slide(settings, input);
            } else if input.edge_active && !(self.speed <= 0.0) {
                let direction = if input.magnitude == 0.0 {
                    input.forward
                } else {
                    input.direction
                };
                let tangent = if dot(input.edge_tangent, direction) >= 0.0 {
                    input.edge_tangent
                } else {
                    scale(input.edge_tangent, -1.0)
                };
                self.edge_target = madd(tangent, 0.5, input.edge_point);
                let delta = sub(self.edge_target, input.position);
                if !(dot(horizontal(direction), horizontal(delta)) <= f32::from_bits(0x3f66_6666)) {
                    self.edge_aligned = true;
                    let normalized = normalize_or(delta, input.forward);
                    self.steering = clamp(dot(normalized, input.right) * 0.5, -1.0, 1.0);
                }
            }
        }
    }

    ///82D7D5A0: opposite signs replace; same signs retain larger magnitude.
    fn merge_steering(&mut self, correction: f32) {
        if !(self.steering * correction >= 0.0) {
            self.steering = correction;
        } else {
            let sign = if correction <= 0.0 { -1.0 } else { 1.0 };
            let old = self.steering * sign;
            let new = correction * sign;
            self.steering = select(old - new, old, new) * sign;
        }
    }

    fn apply_slide(&mut self, settings: &Settings, input: &Input) {
        let angle = wrap_angle(projected_angle(
            input.slide_velocity,
            input.forward,
            input.up,
        ));
        let original = self.steering;
        self.speed = (1.0 - original.abs()) * self.speed;
        let sign = select(angle, 1.0, -1.0);
        if !(original * sign <= 0.0) {
            let limit = 45.0 * f32::from_bits(0x3c8e_fa35);
            let weight = (-angle).mul_add(sign, limit) / limit;
            self.steering = clamp(weight, 0.0, 1.0) * original;
        }
        let correction = settings
            .slide_steering
            .evaluate(sign * (angle * f32::from_bits(0x3ea2_f983)));
        self.steering = clamp((-correction).mul_add(sign, self.steering), -1.0, 1.0);
        if !(angle.abs() >= HALF_PI) {
            self.speed += length(input.slide_velocity);
        }
    }
}

#[cfg(test)]
mod tests;
