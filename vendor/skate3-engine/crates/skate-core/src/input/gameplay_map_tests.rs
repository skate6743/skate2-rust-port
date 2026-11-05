//! Drives each mapped gameplay action ID the way `sdk.input.override_action`
//! does (one value in the 18-entry action vector) through the retail
//! Derived controller update and every listener producer, and checks the
//! intents it reaches. This is the evidence behind the action table in
//! `sdk/GENERAL_API.md`.
use super::*;
use crate::input::{
    controller::{DerivedControllerInput, MagnitudeHeldSettings},
    gameplay_gestures, grind_intentions, manual_intentions, offboard_intentions,
    riding_intentions::{self, PushPreferences},
    trick_intentions, wipeout_intentions,
};
use std::collections::BTreeSet;

const DT: f32 = 1.0 / 30.0;

/// Mirrors `modding::override_actions`: `values[id - 64] = value`.
fn overridden(overrides: &[(u32, f32)]) -> [f32; 18] {
    let mut values = [0.0; 18];
    for &(id, value) in overrides {
        values[(id - 64) as usize] = value;
    }
    values
}

/// One neutral tick, then one tick with the overrides; returns every intent
/// name the listener producers publish on the second tick.
fn intents(overrides: &[(u32, f32)]) -> BTreeSet<&'static str> {
    let settings = MagnitudeHeldSettings {
        attribute: Some(0.5),
        missing_attribute_value: 0.5,
    };
    let mut controller = DerivedControllerInput::from_words([0; 26]);
    controller.initialize();
    for values in [[0.0; 18], overridden(overrides)] {
        let mut map = GameplayActions::from_values(values);
        controller.update(&mut map, DT, false, false, &settings);
    }
    let mut names = BTreeSet::new();
    names.extend(
        riding_intentions::produce(&controller, 0, PushPreferences::default())
            .iter()
            .map(|i| i.name),
    );
    names.extend(
        trick_intentions::produce(&controller)
            .iter()
            .map(|i| i.name),
    );
    names.extend(
        offboard_intentions::produce_discrete(&controller, 0, false)
            .iter()
            .map(|i| i.name),
    );
    names.extend(
        gameplay_gestures::produce(&controller)
            .iter()
            .map(|i| i.name),
    );
    names.extend(
        manual_intentions::produce(&controller, 0)
            .iter()
            .map(|i| i.name),
    );
    names.extend(
        grind_intentions::produce(&controller)
            .iter()
            .map(|i| i.name),
    );
    names.extend(
        wipeout_intentions::produce(&controller, 0, 0x40 | 0x80 | 0x100)
            .iter()
            .map(|i| i.name),
    );
    names
}

fn added(overrides: &[(u32, f32)]) -> BTreeSet<&'static str> {
    let neutral = intents(&[]);
    intents(overrides).difference(&neutral).copied().collect()
}

fn assert_reaches(overrides: &[(u32, f32)], expected: &[&str]) {
    let got = added(overrides);
    for name in expected {
        assert!(
            got.contains(name),
            "{overrides:?} should publish {name}; got {got:?}"
        );
    }
}

#[test]
fn table_is_the_retail_registration_order() {
    for (index, action) in ACTIONS.iter().enumerate() {
        assert_eq!(action.id, 64 + index as u32);
        assert_eq!(action_by_key(action.key), Some(action));
    }
    // Each expression reads the native slot GameplayActions::from_pad uses.
    let slots = [
        "DPadU", "DPadD", "DPadL", "DPadR", "Start", "Back", "LStick", "RStick", "LBumper",
        "RBumper", "LTrigger", "RTrigger", "A", "B", "X", "Y", "LStickR", "LStickL", "LStickU",
        "LStickD", "RStickR", "RStickL", "RStickU", "RStickD",
    ];
    let pad_slots: [&[usize]; 18] = [
        &[16, 17],
        &[18, 19],
        &[6],
        &[20, 21],
        &[22, 23],
        &[7],
        &[10],
        &[11],
        &[8],
        &[9],
        &[0],
        &[1],
        &[2],
        &[3],
        &[14],
        &[15],
        &[12],
        &[13],
    ];
    for (action, expected) in ACTIONS.iter().zip(pad_slots) {
        let used: Vec<usize> = action
            .expression
            .split('-')
            .map(|name| slots.iter().position(|s| *s == name).unwrap())
            .collect();
        assert_eq!(used, expected, "{}", action.name);
    }
}

#[test]
fn neutral_input_publishes_no_button_intents() {
    let neutral = intents(&[]);
    for name in [
        "Brake",
        "Pushing",
        "Dismount",
        "GrabWorld",
        "ToggleOffBoardState",
        "OB_Jump",
    ] {
        assert!(!neutral.contains(name), "{name}");
    }
}

#[test]
fn sticks_reach_steering_tricks_manuals_and_grinds() {
    assert_reaches(
        &[(64, 0.8)],
        &[
            "BodySpin",
            "KickTurn",
            "GrindBalanceX",
            "PhysGrindTranslation",
            "WipeoutControlX",
        ],
    );
    assert_reaches(&[(65, 0.8)], &["WipeoutControlY"]);
    assert_reaches(
        &[(67, 0.8)],
        &[
            "TweakX",
            "BoardAdjustAngle",
            "HandPlantTweakX",
            "OB_LookAtX",
        ],
    );
    assert_reaches(
        &[(68, 0.8)],
        &[
            "Manual",
            "TweakY",
            "HandPlantTweakY",
            "OB_LookAtY",
            "PhysGrindUpDown",
        ],
    );
    assert_reaches(&[(68, -0.95)], &["Manual", "ManualBrake"]);
}

#[test]
fn stick_clicks_and_full_triggers_together_request_a_bail() {
    let all = [(66, 1.0), (69, 1.0), (70, 1.0), (71, 1.0)];
    assert_reaches(&all, &["WipeOutRequest"]);
    for skip in 0..4 {
        let partial: Vec<_> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != skip)
            .map(|(_, a)| *a)
            .collect();
        assert!(!added(&partial).contains("WipeOutRequest"), "{partial:?}");
    }
    assert_reaches(&[(69, 1.0)], &["OB_DoAirBodyTweak"]);
}

#[test]
fn triggers_are_grabs_crouch_and_board_drop_or_throw() {
    assert_reaches(
        &[(70, 1.0)],
        &[
            "LeftGroundGrab",
            "LeftAirGrab",
            "Crouch",
            "OB_DropBoard",
            "OB_RetrieveBoard",
        ],
    );
    assert_reaches(
        &[(71, 1.0)],
        &[
            "RightGroundGrab",
            "RightAirGrab",
            "Crouch",
            "OB_ThrowBoard",
            "WipeOutPushOff",
        ],
    );
    // A partial trigger is an air grab only (ground grabs need exactly 1.0).
    let half = added(&[(70, 0.5)]);
    assert!(
        half.contains("LeftAirGrab")
            && !half.contains("LeftGroundGrab")
            && !half.contains("OB_DropBoard")
    );
}

#[test]
fn bumpers_grab_the_world_and_gate_dpad_and_board_toggle() {
    assert_reaches(
        &[(73, 1.0)],
        &[
            "GrabWorld",
            "DarkCatch",
            "NewDarkCatch",
            "OB_DoAirBodyTweak",
        ],
    );
    assert!(!added(&[(72, 1.0)]).contains("GrabWorld"));
    for bumper in [72, 73] {
        assert!(!added(&[(bumper, 1.0), (74, 1.0)]).contains("GestureUpStart"));
    }
    assert!(!added(&[(72, 1.0), (79, 1.0)]).contains("ToggleOffBoardState"));
    // Both bumpers on one new press: retail keeps LB and drops RB (bit 28).
    assert!(!added(&[(72, 1.0), (73, 1.0)]).contains("GrabWorld"));
}

#[test]
fn dpad_publishes_gesture_starts_and_holds() {
    for (id, direction) in [(74, "Up"), (75, "Down"), (76, "Left"), (77, "Right")] {
        let start = format!("Gesture{direction}Start");
        let held = format!("Gesture{direction}Held");
        let got = added(&[(id, 1.0)]);
        assert!(
            got.iter().any(|n| *n == start) && got.iter().any(|n| *n == held),
            "{id}: {got:?}"
        );
    }
}

#[test]
fn face_buttons_push_brake_dismount_and_leave_the_board() {
    assert_reaches(
        &[(78, 1.0)],
        &[
            "LeftPush",
            "Pushing",
            "NewPush",
            "HandPlantOneFootLeft",
            "OB_Jump",
            "WipeOutRecover",
        ],
    );
    assert_reaches(
        &[(79, 1.0)],
        &["ToggleOffBoardState", "NewToggleOffBoardState"],
    );
    assert_reaches(
        &[(80, 1.0)],
        &[
            "RightPush",
            "Pushing",
            "NewPush",
            "HandPlantOneFootRight",
            "OB_Sprint",
            "WipeOutRecover",
        ],
    );
    assert_reaches(
        &[(81, 1.0)],
        &[
            "Brake",
            "Dismount",
            "NewDismount",
            "DarkCatch",
            "HandPlantDismount",
        ],
    );
    // B held blocks the off-board sprint that A starts.
    assert!(!added(&[(80, 1.0), (81, 1.0)]).contains("OB_Sprint"));
}
