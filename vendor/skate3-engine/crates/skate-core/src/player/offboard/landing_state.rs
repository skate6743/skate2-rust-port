//! State503, original TU3 82D4D418..82D4E100 (SHA431b8eba...0395a).
//! Manager owns the trajectory; this state owns alignment and landing decisions.
use super::{
    hippy_jump,
    landing_deck::{GRAVITY, Manager, STEP, Trajectory, UpdateOutput},
};
use crate::physics::board_motion_output::inverse_length_squared;
use crate::player::wipeout_state::{math::wrap_angle, orientation::projected_angle};
pub type Vector = [f32; 4];
const UP: Vector = [0., 1., 0., 0.];
const EPSILON: f32 = f32::from_bits(0x37800000);
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub minimum_auto_angle: f32,
    pub automatic_speed: f32,
    pub input_speed: f32,
    pub input_delta: f32,
    pub automatic_delta: f32,
    pub maximum_landing_speed: f32,
}
#[derive(Default, Debug)]
pub struct State {
    pub output: Option<UpdateOutput>,
    pub time_to_land: f32,
    pub dangerous: bool,
    pub near_deck: bool,
    pub turning: bool,
    pub hippy: bool,
    pub takeoff_frames: i32,
    pub spin_rate: f32,
    pub applied_spin: f32,
    pub transition_angle: f32,
    pub accumulated_spin: f32,
    pub half_turns: i32,
    pub next_half_turns: i32,
}
pub struct Entry {
    pub previous_category: u32,
    pub hippy: bool,
    pub strength: f32,
    pub board_position: Vector,
    pub com_position: Vector,
    pub com_velocity: Vector,
    pub board_velocity: Vector,
    pub up: Vector,
    pub hips_up: Vector,
    pub animation_right: Vector,
    pub reversed: bool,
}
impl State {
    ///82D4D418/530/668: same manager as BipedAir; preservation depends on category.
    pub fn enter(&mut self, manager: &mut Manager, input: Entry) {
        let preserve = input.previous_category == 500;
        if !preserve {
            manager.reset();
        }
        *self = Self::default();
        let right = input
            .animation_right
            .map(|v| if input.reversed { -v } else { v });
        self.transition_angle = wrap_angle(projected_angle(input.hips_up, right, UP));
        if input.hippy || !preserve {
            let velocity = if input.hippy {
                hippy_jump::calculate(hippy_jump::Input {
                    desired_height: (1. - input.strength) * 1.35 + input.strength * 1.65,
                    board_position: input.board_position,
                    centre_of_mass_position: input.com_position,
                    reference_up: input.up,
                    current_velocity: input.board_velocity,
                })
            } else {
                input.com_velocity
            };
            manager.trajectory_32 = Trajectory {
                position: input.com_position,
                velocity,
                acceleration: GRAVITY,
                duration: -1.,
            };
            manager.elapsed_160 = 0.;
            manager.trajectory_valid_164 = true;
            manager.force_257 = input.hippy;
            if input.hippy {
                self.takeoff_frames = 2;
            }
        } else {
            manager.elapsed_160 += STEP;
            manager.correct_trajectory(input.com_position);
        }
        self.hippy = input.hippy;
    }
    ///82D4D838..DAFC, after the manager update and before Skeleton82BDE428.
    pub fn align(
        &mut self,
        s: &Settings,
        board_forward: Vector,
        animation_forward: Vector,
        state_time: f32,
        com_velocity_y: f32,
        input_spin: f32,
    ) {
        let frames = self.time_to_land * 59.999996;
        let mut angle = projected_angle(board_forward, animation_forward, UP);
        if state_time < 0.28 {
            angle -= (1. - state_time * 3.5714285) * self.transition_angle;
        }
        angle = wrap_angle(self.spin_rate * frames + angle);
        self.turning = angle.abs() > std::f32::consts::FRAC_PI_2;
        if self.time_to_land > EPSILON {
            let rising = com_velocity_y > 0.;
            let input = if rising && self.hippy { input_spin } else { 0. };
            let (target, delta) = if input.abs() > EPSILON {
                (-s.input_speed * input, s.input_delta)
            } else {
                if rising {
                    //82E09C80 folds the wrapped angle modulo pi, not modulo 2pi.
                    angle = wrap_angle(angle);
                    if angle.abs() > std::f32::consts::FRAC_PI_2 {
                        angle = (angle.abs() - std::f32::consts::PI) * angle.signum();
                    }
                }
                let tolerance = s.minimum_auto_angle * 0.017453292;
                let target = if angle.abs() > tolerance {
                    let excess = if angle > 0. {
                        (angle - tolerance).max(0.)
                    } else {
                        (angle + tolerance).min(0.)
                    };
                    (self.spin_rate - excess / frames).clamp(-s.automatic_speed, s.automatic_speed)
                } else {
                    self.spin_rate
                };
                (target, s.automatic_delta)
            };
            self.spin_rate = target.clamp(self.spin_rate - delta, self.spin_rate + delta);
            self.applied_spin = self.spin_rate;
            if state_time < 0.28 {
                self.applied_spin = (-self.transition_angle).mul_add(0.059523813, self.spin_rate);
            }
        }
    }
    ///82D4DB10..DB74, called only after the skeleton update completes.
    pub fn advance_spin(&mut self, output: UpdateOutput) {
        self.accumulated_spin += self.applied_spin;
        self.half_turns = (self.accumulated_spin * 0.31830987) as i32;
        self.next_half_turns = self.half_turns;
        if self.accumulated_spin < 0. && self.applied_spin < 0. {
            self.next_half_turns -= 1;
        } else if self.accumulated_spin > 0. && self.applied_spin > 0. {
            self.next_half_turns += 1;
        }
        self.time_to_land = output.time_to_land;
        self.output = Some(output);
    }
    ///82D4DD80: actual physical toe-volume positions15/19, not IK targets.
    pub fn accurate_time(
        board_y: f32,
        board_velocity_y: f32,
        com_velocity_y: f32,
        toe_y: [f32; 2],
        wheel_contacts: u32,
    ) -> f32 {
        let feet = (toe_y[0] + toe_y[1]) * 0.5 - 0.15000001;
        let velocity = com_velocity_y - board_velocity_y;
        let displacement = board_y - feet;
        let gravity = if wheel_contacts != 0 { -9.8 } else { -4.9 };
        let discriminant = velocity * velocity + (displacement * gravity) * 2.;
        if discriminant > 0. {
            let root = discriminant * inverse_length_squared(discriminant, 2);
            (-velocity - root) / gravity
        } else {
            0.
        }
    }
    ///82D4DCBC..DD58; leaves near_deck untouched while rising.
    pub fn finish(&mut self, s: &Settings, com_velocity_y: f32) {
        if self.takeoff_frames > 0 {
            self.takeoff_frames -= 1;
        }
        self.dangerous = self.time_to_land < 0.07 && com_velocity_y < -s.maximum_landing_speed;
    }
    pub fn landing_half_turns(&self) -> i32 {
        if self.time_to_land >= 0.05 {
            self.half_turns
        } else {
            self.next_half_turns
        }
    }
    pub fn requests_board_flip(&self) -> bool {
        self.turning && self.time_to_land < 0.02
    }
}
#[cfg(test)]
mod tests;
