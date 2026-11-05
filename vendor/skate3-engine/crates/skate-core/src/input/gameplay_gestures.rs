//! Skate 3 TU3 gameplay D-pad gestures, Fill helper 8259BF50.
//! These are emote intents, independent of trick-stick gesture recognition.
use super::{controller::DerivedControllerInput, riding_intentions::RidingIntent};

pub fn produce(controller: &DerivedControllerInput) -> Vec<RidingIntent> {
    let words = controller.words();
    let previous = words[6];
    let current = words[13];
    if current & ((1 << 29) | (1 << 28)) != 0 {
        return Vec::new();
    }
    let directions = [
        (26, "GestureDownStart", "GestureDownHeld"),
        (25, "GestureLeftStart", "GestureLeftHeld"),
        (24, "GestureRightStart", "GestureRightHeld"),
        (27, "GestureUpStart", "GestureUpHeld"),
    ];
    let mut out = Vec::with_capacity(8);
    // The native producer emits all starts before held intents.
    for (bit, start, _) in directions {
        if current & (1 << bit) != 0 && previous & (1 << bit) == 0 {
            out.push(RidingIntent {
                name: start,
                value: 1.,
            });
        }
    }
    for (bit, _, held) in directions {
        if current & (1 << bit) != 0 {
            out.push(RidingIntent {
                name: held,
                value: 1.,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn intents(previous: u32, current: u32) -> Vec<&'static str> {
        let mut words = [0; 26];
        words[6] = previous;
        words[13] = current;
        produce(&DerivedControllerInput::from_words(words))
            .into_iter()
            .map(|i| i.name)
            .collect()
    }
    #[test]
    fn gameplay_gestures_preserve_direction_edges_holds_and_release() {
        for (bit, name) in [(27, "Up"), (26, "Down"), (25, "Left"), (24, "Right")] {
            let pressed = intents(0, 1 << bit);
            assert_eq!(
                pressed,
                [format!("Gesture{name}Start"), format!("Gesture{name}Held")]
            );
            assert_eq!(intents(1 << bit, 1 << bit), [format!("Gesture{name}Held")]);
            assert!(intents(1 << bit, 0).is_empty());
        }
    }
    #[test]
    fn bumpers_suppress_gestures_without_synthesizing_a_new_edge_on_release() {
        for bumper in [1 << 28, 1 << 29] {
            assert!(intents(0, bumper | (1 << 27)).is_empty());
            assert_eq!(intents(bumper | (1 << 27), 1 << 27), ["GestureUpHeld"]);
        }
        assert_eq!(
            intents(0, 0x0f00_0000),
            [
                "GestureDownStart",
                "GestureLeftStart",
                "GestureRightStart",
                "GestureUpStart",
                "GestureDownHeld",
                "GestureLeftHeld",
                "GestureRightHeld",
                "GestureUpHeld"
            ]
        );
    }
}
