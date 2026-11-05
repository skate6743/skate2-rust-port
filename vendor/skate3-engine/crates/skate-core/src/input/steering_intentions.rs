//! Complete local Turn/HardTurn/HardTurnCrouch emission dataflow in 825999F0.
//! Other intention outputs and the subsequent action/motion graphs are separate.
use super::{angle::left_stick_angle, controller::magnitude};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SteeringIntentions {
    /// Absence is retained: native does not emit a zero Turn intention.
    pub turn: Option<f32>,
    pub hard_turn: Option<f32>,
    pub hard_turn_crouch: Option<f32>,
}

/// Consumes the current record after 825992D8 and actor+1908 captured at
/// 82599A38. No history field affects these three local emission sites.
pub fn produce(left: [f32; 2], actor_flags_1908: u32) -> SteeringIntentions {
    let [x, y] = left;
    let angle = left_stick_angle(x, y);
    let absolute = angle.abs();
    let length = if x == 0.0 && y == 0.0 {
        0.0
    } else {
        magnitude(y.mul_add(y, x * x))
    };
    let signed_length = if x >= 0.0 { length } else { -length };
    let turn = if y > 0.0 {
        if absolute > f32::from_bits(0x40433333) {
            0.0
        } else if absolute > f32::from_bits(0x40166666) {
            ((f32::from_bits(0x40433333) - absolute) * f32::from_bits(0x3fb6db6d)) * signed_length
        } else {
            signed_length
        }
    } else {
        x
    };
    let active = length > f32::from_bits(0x3f666666)
        && !(absolute > f32::from_bits(0x3fb1eb85))
        && !(absolute < f32::from_bits(0x3f68f5c3));
    let hard = (absolute - f32::from_bits(0x3fb1eb85)) * f32::from_bits(0xc0055556);
    let minimum = f32::from_bits(0x3a83126f);
    let hard = if hard - minimum >= 0.0 { hard } else { minimum };
    let hard = if angle >= 0.0 { hard } else { -hard };
    let allowed = actor_flags_1908 & 1 == 0;
    SteeringIntentions {
        turn: (turn != 0.0 && allowed).then_some(turn),
        hard_turn: (active && allowed).then_some(hard),
        hard_turn_crouch: (active && allowed).then_some((hard * f32::from_bits(0x3f59999a)).abs()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_steering_preserves_signed_turn_and_hard_turn_symmetry() {
        let left = produce([-0.75, 0.0], 0);
        let right = produce([0.75, 0.0], 0);

        assert_eq!(left.turn, Some(-0.75));
        assert_eq!(right.turn, Some(0.75));
        assert_eq!(left.hard_turn, right.hard_turn.map(|value| -value));
        assert_eq!(left.hard_turn_crouch, right.hard_turn_crouch);
    }

    #[test]
    fn steering_gate_suppresses_all_three_emissions_without_changing_input() {
        let gated = produce([-0.75, 0.0], 1);
        assert_eq!(
            gated,
            SteeringIntentions {
                turn: None,
                hard_turn: None,
                hard_turn_crouch: None,
            }
        );
    }
}
