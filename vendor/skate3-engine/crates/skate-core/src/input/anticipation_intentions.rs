//! Right-stick anticipation publication, Fill825999F0:
//! angle/magnitude 82599AD0..9C4C, emissions 8259AD0C..AD40.
use super::{
    angle::left_stick_angle,
    controller::{DerivedControllerInput, magnitude},
    riding_intentions::RidingIntent,
};

pub fn produce(controller: &DerivedControllerInput) -> Vec<RidingIntent> {
    let words = controller.words();
    let x = f32::from_bits(words[9]);
    let y = f32::from_bits(words[10]);
    if x == 0.0 && y == 0.0 {
        return Vec::new();
    }
    // Unlike steering, these two records have no actor1908 bit gate.
    vec![
        RidingIntent {
            name: "AnticMag",
            value: magnitude(x.mul_add(x, y * y)),
        },
        RidingIntent {
            name: "AnticAngle",
            value: left_stick_angle(x, y),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neutral_removes_anticipation_and_tail_hold_selects_ollie_range() {
        assert!(produce(&DerivedControllerInput::from_words([0; 26])).is_empty());
        let mut words = [0; 26];
        words[10] = (-1.0f32).to_bits();
        let intents = produce(&DerivedControllerInput::from_words(words));
        assert!((intents[0].value - 1.0).abs() < 0.000001);
        assert!(intents[1].value.abs() <= 0.52);
        words[10] = 1.0f32.to_bits();
        assert!(
            produce(&DerivedControllerInput::from_words(words))[1]
                .value
                .abs()
                >= 2.62
        );
    }
}
