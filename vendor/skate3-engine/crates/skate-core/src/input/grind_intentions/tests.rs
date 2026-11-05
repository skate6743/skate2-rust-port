use super::*;

fn values(x: f32, rx: f32, ry: f32) -> Vec<(&'static str, f32)> {
    let mut words = [0; 26];
    words[7] = x.to_bits();
    words[9] = rx.to_bits();
    words[10] = ry.to_bits();
    produce(&DerivedControllerInput::from_words(words))
        .into_iter()
        .map(|i| (i.name, i.value))
        .collect()
}
#[test]
fn all_four_s3_emissions_preserve_order_and_sign() {
    assert_eq!(
        values(0.25, 0.5, -0.75),
        vec![
            ("GrindBalanceX", -0.25),
            ("PhysGrindTranslation", 0.75),
            ("PhysGrindStabilityNudge", 0.25),
            ("PhysGrindUpDown", -0.75),
        ]
    );
}
#[test]
fn translation_saturates_both_directions_and_cancellation_omits_only_it() {
    assert_eq!(values(0.75, 0.75, 0.0)[1], ("PhysGrindTranslation", 1.0));
    assert_eq!(values(-0.75, -0.75, 0.0)[1], ("PhysGrindTranslation", -1.0));
    assert_eq!(
        values(0.5, -0.5, 0.0),
        vec![("GrindBalanceX", -0.5), ("PhysGrindStabilityNudge", 0.5),]
    );
    assert!(values(0.0, -0.0, 0.0).is_empty());
}
#[test]
fn left_only_translation_distinguishes_s3_from_s2() {
    assert_eq!(values(0.5, 0.0, 0.0)[1], ("PhysGrindTranslation", 0.5));
}
