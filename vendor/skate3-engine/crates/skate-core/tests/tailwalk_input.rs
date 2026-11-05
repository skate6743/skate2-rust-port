//! Input-publication check; does not substitute for the full airborne replay.
use skate_core::input::{
    controller::{DerivedControllerInput, MagnitudeHeldSettings},
    gameplay_map::GameplayActions,
    pad::Pad,
    riding_intentions::{self, PushPreferences},
    trick_intentions,
    xbox::{self, XboxState},
};

#[test]
fn alternate_double_push_route_publishes_required_action_graph_inputs() {
    let mut pad = Pad::new();
    let mut controller = DerivedControllerInput::from_words([0; 26]);
    controller.initialize();
    let settings = MagnitudeHeldSettings {
        attribute: Some(0.5),
        missing_attribute_value: 0.5,
    };
    for buttons in [0, 0x5000] {
        pad.update(&xbox::convert(
            &XboxState {
                buttons,
                triggers: [255, 0],
                left: [0, 0],
                right: [0, 32767],
            },
            0,
        ));
        controller.update(
            &mut GameplayActions::from_pad(&pad),
            1. / 60.,
            false,
            false,
            &settings,
        );
        let trick = trick_intentions::produce(&controller);
        assert!(trick.iter().any(|i| i.name == "LeftAirGrab"));
        assert!(
            trick
                .iter()
                .any(|i| i.name == "BoardAdjustMag" && i.value > 0.)
        );
        assert!(
            trick
                .iter()
                .any(|i| i.name == "BoardAdjustAngle" && i.value.abs() >= 2.355)
        );
        let push = riding_intentions::produce(&controller, 0, PushPreferences::default());
        for name in ["LeftPush", "RightPush"] {
            assert_eq!(push.iter().any(|i| i.name == name), buttons == 0x5000);
        }
    }
    // The actor's double-push suppression remains a live graph dependency.
    let blocked = riding_intentions::produce(&controller, 1 << 6, PushPreferences::default());
    assert!(
        !blocked
            .iter()
            .any(|i| matches!(i.name, "LeftPush" | "RightPush"))
    );
}

#[test]
fn tailwalk_b_button_publishes_dismount_hold_and_press_edge() {
    let mut pad = Pad::new();
    pad.update(&xbox::convert(
        &XboxState {
            buttons: 0x2000,
            triggers: [255, 0],
            left: [0, 0],
            right: [0, 32767],
        },
        0,
    ));
    let mut controller = DerivedControllerInput::from_words([0; 26]);
    controller.initialize();
    controller.update(
        &mut GameplayActions::from_pad(&pad),
        1. / 60.,
        false,
        false,
        &MagnitudeHeldSettings {
            attribute: Some(0.5),
            missing_attribute_value: 0.5,
        },
    );
    // TU3 Fill8259AA20..AA34 publishes Dismount when this bit is set.
    assert_ne!(controller.words()[13] & (1 << 20), 0);
    let trick = trick_intentions::produce(&controller);
    let riding = riding_intentions::produce(&controller, 0, PushPreferences::default());
    assert!(trick.iter().any(|i| i.name == "Dismount" && i.value == 1.));
    assert!(
        trick
            .iter()
            .any(|i| i.name == "NewDismount" && i.value == 1.)
    );
    assert!(!riding.iter().any(|i| i.name == "Dismount"));
    assert!(
        !riding
            .iter()
            .any(|i| matches!(i.name, "LeftPush" | "RightPush"))
    );
    for (buttons, held, edge) in [
        (0x2000, true, false),
        (0, false, false),
        (0x2000, true, true),
        (0x0200, false, false),
    ] {
        pad.update(&xbox::convert(
            &XboxState {
                buttons,
                triggers: [255, 0],
                left: [0, 0],
                right: [0, 32767],
            },
            0,
        ));
        controller.update(
            &mut GameplayActions::from_pad(&pad),
            1. / 60.,
            false,
            false,
            &MagnitudeHeldSettings {
                attribute: Some(0.5),
                missing_attribute_value: 0.5,
            },
        );
        let intents = trick_intentions::produce(&controller);
        assert_eq!(intents.iter().any(|i| i.name == "Dismount"), held);
        assert_eq!(intents.iter().any(|i| i.name == "NewDismount"), edge);
    }
}
