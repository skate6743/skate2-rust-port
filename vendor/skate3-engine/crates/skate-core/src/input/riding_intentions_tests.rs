use super::*;
#[test]
fn right_bumper_publishes_held_grab_world_without_actor_gate() {
    for (previous, current, expected) in [
        (0, 1 << 28, true),
        (1 << 28, 1 << 28, true),
        (1 << 28, 0, false),
        (0, 1 << 29, false),
    ] {
        for actor_flags in [0, u32::MAX] {
            let output = produce(
                &snapshot(previous, current, 0.5),
                actor_flags,
                PushPreferences::default(),
            );
            let values: Vec<_> = output.iter().filter(|i| i.name == "GrabWorld").collect();
            assert_eq!(values.len(), usize::from(expected));
            if expected {
                assert_eq!(values[0].value, 1.0);
            }
        }
    }
}

#[test]
fn crouch_preserves_zero_presence_and_uses_maximum_trigger() {
    let mut words = [0u32; 26];
    words[8] = 1.0f32.to_bits();
    let values = produce(
        &DerivedControllerInput::from_words(words),
        1,
        PushPreferences::default(),
    );
    assert_eq!(
        values.iter().find(|i| i.name == "Crouch").unwrap().value,
        0.0
    );
    words[11] = 0.4f32.to_bits();
    words[12] = 0.7f32.to_bits();
    words[7] = 0.5f32.to_bits();
    let values = produce(
        &DerivedControllerInput::from_words(words),
        0,
        PushPreferences::default(),
    );
    assert_eq!(
        values.iter().find(|i| i.name == "Crouch").unwrap().value,
        0.7
    );
    assert_eq!(
        values.iter().find(|i| i.name == "KickTurn").unwrap().value,
        0.5
    );
    assert_ne!(values.iter().find(|i| i.name == "Turn").unwrap().value, 0.5);
}
fn snapshot(previous: u32, current: u32, held: f32) -> DerivedControllerInput {
    let mut words = [0; 26];
    words[6] = previous;
    words[13] = current;
    words[20] = held.to_bits();
    words[22] = held.to_bits();
    DerivedControllerInput::from_words(words)
}
#[test]
fn held_push_continues_after_new_push_allowance_expires() {
    let button = 1 << 21;
    let first = produce(&snapshot(0, button, 0.5), 0, PushPreferences::default());
    assert!(first.iter().any(|i| i.name == "NewPush"));
    let held = produce(
        &snapshot(button, button, 0.5),
        0,
        PushPreferences::default(),
    );
    assert_eq!(
        held.iter().map(|i| i.name).collect::<Vec<_>>(),
        ["RightPush", "Pushing"]
    );
    assert!(
        produce(
            &snapshot(0, button, 0.0),
            1 << 11,
            PushPreferences::default()
        )
        .is_empty()
    );
}
#[test]
fn dual_push_gate_requires_both_feet_and_brake_uses_actor_mode() {
    assert!(
        !produce(
            &snapshot(0, 1 << 21, 0.0),
            1 << 6,
            PushPreferences::default()
        )
        .is_empty()
    );
    assert!(
        produce(
            &snapshot(0, (1 << 21) | (1 << 23), 0.0),
            1 << 6,
            PushPreferences::default()
        )
        .is_empty()
    );
    let brake = snapshot(0, 1 << 20, 0.0);
    assert_eq!(
        produce(&brake, 0, PushPreferences::default())[0].name,
        "Brake"
    );
    assert!(produce(&brake, 1 << 7, PushPreferences::default()).is_empty());
}

#[test]
fn powerslide_queries_are_separate_from_signed_continuous_values() {
    let intents = |x: f32, y: f32, flags| {
        let mut words = [0; 26];
        words[7] = x.to_bits();
        words[8] = y.to_bits();
        produce(
            &DerivedControllerInput::from_words(words),
            flags,
            PushPreferences::default(),
        )
    };
    //8259A430..A50C: lateral steering is outside both start windows.
    for x in [-1.0, 1.0] {
        let values = intents(x, 0.0, 0);
        assert!(!values.iter().any(|i| i.name.ends_with("SlideStart")));
        assert!(values.iter().any(|i| i.name == "Turn"));
    }
    //Diagonal rearward queries have independent, overlapping continuous values.
    for (x, start) in [(0.6, "RightSlideStart"), (-0.6, "LeftSlideStart")] {
        let values = intents(x, -0.8, 0);
        assert!(values.iter().any(|i| i.name == start && i.value == 1.0));
        assert!(
            values
                .iter()
                .any(|i| i.name == "LeftSlide" && i.value < 0.0)
        );
        assert!(
            values
                .iter()
                .any(|i| i.name == "RightSlide" && i.value > 0.0)
        );
        let inhibited_push = intents(x, -0.8, 1 << 11);
        for value in values.iter().filter(|i| i.name.contains("Slide")) {
            assert!(inhibited_push.contains(value));
        }
        assert!(
            !intents(x, -0.8, 1 << 8)
                .iter()
                .any(|i| i.name.contains("Slide"))
        );
    }
    for (x, y) in [(0.0, 0.0), (0.0, 1.0), (0.0, -1.0), (0.3, -0.4)] {
        let values = intents(x, y, 0);
        assert!(!values.iter().any(|i| i.name.ends_with("SlideStart")));
    }
    assert!(
        !intents(0.3, -0.4, 0)
            .iter()
            .any(|i| i.name.contains("Slide"))
    );
}
