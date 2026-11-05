//! Grind Post82D43098 -> Wipeout82D90898; S2 82D85E78 ->82DDF248.
//! Unlike Ground, no cooldown/vehicle/squash/balance checks are added here.
use crate::{
    physics::{board_motion_output::inverse_length_squared, native_arithmetic::dot3},
    player::wipeout::{Frame, Requests, regional_force},
    trigonometry,
};

/// Required physics_wipeout/default layout values. No borrowed Ground/Air
/// thresholds and no default-value fallback. Shared stock wiring supplies these.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub max_arm_contact_164: f32,
    pub max_body_contact_168: f32,
    pub xz_acceleration_204: f32,
    pub max_displacement_208: f32,
    pub max_angular_deck_error_212: f32,
}

pub fn check(state: &mut Requests, settings: Settings, frame: &Frame) {
    state.mode = 5; // Preserve balance, timers and existing requests.
    if dot3(frame.pose_error, frame.pose_error)
        > settings.max_displacement_208 * settings.max_displacement_208
    {
        state.request(1, 0.0);
    } else if regional_force(
        frame,
        settings.max_body_contact_168,
        settings.max_arm_contact_164,
    ) {
        state.request(0, 0.0);
    }
    //82D90AB8. Closing-velocity checks still run after displacement/contact.
    if frame.board_contact {
        let v = frame.closing_velocity;
        let m = frame.world_to_animation;
        let local: [f32; 4] =
            core::array::from_fn(|i| m[2][i].mul_add(v[2], m[1][i].mul_add(v[1], m[0][i] * v[0])));
        let square = dot3(
            [local[0], 0.0, local[2], local[3]],
            [local[0], 0.0, local[2], local[3]],
        );
        let length = if square == 0.0 {
            0.0
        } else {
            square * inverse_length_squared(square, 2)
        };
        if length > settings.xz_acceleration_204 || local[1].abs() > 100.0 {
            state.request(2, 0.0);
            if regional_force(frame, 1.0, 20.0) {
                state.request(0, 0.0);
            }
        }
    }
    //82D909B0 always checks all three corresponding deck/Input axes.
    let cosine = dot3(frame.deck[0], frame.input_board[0])
        .min(dot3(frame.deck[1], frame.input_board[1]))
        .min(dot3(frame.deck[2], frame.input_board[2]));
    if trigonometry::acos(cosine.max(-1.0).min(1.0)) > settings.max_angular_deck_error_212 {
        state.request(3, 0.0);
    }
}
