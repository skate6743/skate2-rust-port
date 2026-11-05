//! Exercise the production vert-input adapter from raw controller samples.
use super::transition_action;
use skate3_core::input::{gameplay_map::GameplayActions, pad::Pad, xbox::{self, XboxState}};

fn sample(left_y: i16, triggers: [u8; 2], right_y: i16) -> f32 {
    let mut pad = Pad::new();
    pad.update(&xbox::convert(&XboxState {
        buttons: 0, triggers, left: [0, left_y], right: [0, right_y],
    }, 0));
    transition_action(&mut GameplayActions::from_pad(&pad))
}

#[test]
fn quarter_pipe_exit_uses_signed_forward_stick() {
    // Positive lean sends the launch away from the ramp's inward normal;
    // neutral/backward lean retains the return-to-transition trajectory.
    assert!(sample(32767, [0, 0], 0) > 0.99);
    assert!(sample(-32768, [0, 0], 0) < -0.99);
    assert_eq!(sample(0, [0, 0], 0), 0.0);
    assert_eq!(sample(4000, [0, 0], 0), 0.0); // native stick deadzone
    let partial = sample(20000, [0, 0], 0);
    assert!(partial > 0.25 && partial < 0.99);
}

#[test]
fn quarter_pipe_exit_is_independent_of_grabs_and_trick_stick() {
    for triggers in [[0, 0], [255, 0], [0, 255], [255, 255]] {
        for right_y in [-32768, 0, 32767] {
            for left_y in [-32768, 0, 32767] {
                assert_eq!(sample(left_y, triggers, right_y), sample(left_y, [0, 0], 0));
            }
        }
    }
}
