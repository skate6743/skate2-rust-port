//! Original82D78EE8 and its82D60B98/82D60C80,82D609E0,82D60B00 arithmetic.
//! Uses the existing Trajectory evaluations; no alternative trajectory owner.
use super::{GRAVITY, Input, Manager, STEP, Settings, Trajectory, Vector};
use crate::physics::board_motion_output::inverse_length_squared;
use crate::player::wipeout_state::math::{add, dot, length, madd, reciprocal, scale, sub};

pub(super) fn frames(time: f32) -> i32 {
    (time * f32::from_bits(0x426f_ffff)) as i32
}
fn square_root(value: f32) -> f32 {
    if value == 0. {
        0.
    } else {
        value * inverse_length_squared(value, 2)
    }
}

///82D60C80 ->82D60B98. No invented linear/zero-acceleration fallback.
///The one-root wrapper requires positive time; the two-root wrapper does not.
fn greatest_plane_time(t: Trajectory, point: Vector, normal: Vector) -> Option<f32> {
    let a = dot(normal, scale(t.acceleration, reciprocal(2.)));
    let b = dot(normal, t.velocity);
    let c = dot(normal, t.position) - dot(point, normal);
    let discriminant = b * b - (4. * a) * c;
    if discriminant < 0. {
        None
    } else if discriminant > 0. {
        let root = square_root(discriminant);
        let inverse = reciprocal(2. * a);
        Some((inverse * (-b + root)).max(inverse * (-b - root)))
    } else {
        let time = reciprocal(2. * a) * -b;
        (time > 0.).then_some(time)
    }
}

pub(super) fn adjust(t: &mut Trajectory, frame: i32, delta: Vector, maximum: f32) {
    if frame <= 0 {
        return;
    }
    let correction = scale(delta, reciprocal(frame as f32 * STEP));
    let factor = if dot(correction, correction) > maximum * maximum {
        maximum / length(correction)
    } else {
        1.
    };
    t.velocity = madd(correction, factor, t.velocity);
}
///82D60B00 leaves duration untouched.
pub(super) fn shift(t: &mut Trajectory, frame: i32) {
    let time = frame as f32 * STEP;
    let position = t.position_at(time);
    t.velocity = t.velocity_at(time);
    t.position = position;
}

impl Manager {
    ///82D78EE8. Failed intersection clears only can-land; other fields retain.
    pub(super) fn consider_board(
        &mut self,
        p: &Input,
        settings: &Settings,
        position: Vector,
        velocity: Vector,
    ) {
        let airborne_board = p.wheel_contacts_2556 == 0;
        let mut board_velocity = p.board_velocity_400;
        if airborne_board {
            board_velocity[1] = 0.;
        }
        let relative = Trajectory {
            position,
            velocity: sub(velocity, board_velocity),
            acceleration: if airborne_board {
                scale(GRAVITY, 0.5)
            } else {
                GRAVITY
            },
            duration: -1.,
        };
        let target = madd(
            p.up_544,
            settings.approximate_com_height,
            p.board_position_112,
        );
        let Some(time) = greatest_plane_time(relative, target, p.up_544).filter(|t| *t > 0.) else {
            self.can_land_256 = false;
            return;
        };
        //822F8BD4=9.8;82060C50=2. Min-speed time is clamped to [.75t,1.3t].
        let distance = length(sub(position, target));
        let optimal = square_root(reciprocal(f32::from_bits(0x411c_cccd)) * (2. * distance));
        let proposed_time = (time * 1.3).min((time * 0.75).max(optimal));
        self.proposed_96 = Trajectory {
            position,
            velocity: add(
                sub(
                    scale(sub(target, position), reciprocal(proposed_time)),
                    scale(scale(GRAVITY, 0.5), proposed_time),
                ),
                p.board_velocity_400,
            ),
            acceleration: GRAVITY,
            duration: -1.,
        };
        self.ik_offset_176 = sub(target, relative.position_at(time));
        self.ik_offset_176[1] = 0.;
        self.time_to_land_244 = time;
        self.proposed_time_248 = proposed_time;
    }
}
