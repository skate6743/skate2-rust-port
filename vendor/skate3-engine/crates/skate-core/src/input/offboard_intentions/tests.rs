use super::*;

fn values(words: [u32; 26], flags: u32, blocked: bool) -> Vec<OffboardIntent> {
    produce_discrete(&DerivedControllerInput::from_words(words), flags, blocked)
}
fn has(output: &[OffboardIntent], name: &str) -> bool {
    output.iter().any(|intent| intent.name == name)
}
fn toggle_words(previous: bool, timer: f32) -> [u32; 26] {
    let mut words = [0; 26];
    words[6] = if previous { 1 << 22 } else { 0 };
    words[13] = 1 << 22;
    words[23] = timer.to_bits();
    words
}

#[test]
fn toggle_rise_and_short_hold_are_distinct_from_continuous_hold() {
    let new = "NewToggleOffBoardState";
    let held = "ToggleOffBoardState";
    assert!(has(&values(toggle_words(false, 9.0), 0, false), new));
    assert!(has(&values(toggle_words(true, 0.02), 0, false), new));
    let boundary = values(toggle_words(true, f32::from_bits(0x3cf5_c28f)), 0, false);
    assert!(!has(&boundary, new));
    assert!(has(&boundary, held));
    let nan_timer = values(toggle_words(true, f32::NAN), 0, false);
    assert!(has(&nan_timer, new));
    let mut released = toggle_words(true, 0.0);
    released[13] = 0;
    let released = values(released, 0, false);
    assert!(!has(&released, new));
    assert!(!has(&released, held));
}

#[test]
fn physical_and_modifier_gates_block_both_but_actor_gate_only_blocks_new() {
    let words = toggle_words(false, 0.0);
    let disabled = values(words, 1 << 10, false);
    assert!(!has(&disabled, "NewToggleOffBoardState"));
    assert!(has(&disabled, "ToggleOffBoardState"));
    let mut modifier = words;
    modifier[13] |= 1 << 29;
    for output in [values(words, 0, true), values(modifier, 0, false)] {
        assert!(!has(&output, "NewToggleOffBoardState"));
        assert!(!has(&output, "ToggleOffBoardState"));
    }
}

#[test]
fn y_remains_toggle_for_stock_offboard_mount_not_board_recall() {
    //S3 listener8259A604..A6C0 has no physical-state input. The stock
    //OffBoard/OBGround/Default graph consumes both of these for remount.
    let output = values(toggle_words(false, 0.0), 0, false);
    assert!(has(&output, "NewToggleOffBoardState"));
    assert!(has(&output, "ToggleOffBoardState"));
    assert!(!has(&output, "OB_RetrieveBoard"));
}

#[test]
fn jump_requires_new_button_and_does_not_inherit_toggle_gates() {
    let mut words = [0; 26];
    words[13] = (1 << 23) | (1 << 29);
    words[22] = 0.01f32.to_bits();
    assert!(has(&values(words, u32::MAX, true), "OB_Jump"));
    words[6] = 1 << 23;
    assert!(!has(&values(words, 0, false), "OB_Jump"));
}

#[test]
fn full_trigger_edges_publish_board_actions_together_and_do_not_repeat() {
    let mut words = [0; 26];
    words[11] = 0.99f32.to_bits();
    assert!(!has(&values(words, 0, false), "OB_RetrieveBoard"));
    words[11] = 1.0f32.to_bits();
    words[12] = 1.0f32.to_bits();
    let output = values(words, u32::MAX, true);
    let board: Vec<_> = output
        .iter()
        .filter(|i| i.name.ends_with("Board"))
        .map(|i| i.name)
        .collect();
    assert_eq!(board, ["OB_DropBoard", "OB_ThrowBoard", "OB_RetrieveBoard"]);
    words[4] = words[11];
    words[5] = words[12];
    assert!(!has(&values(words, 0, false), "OB_RetrieveBoard"));
    words[5] = 0;
    let right_only = values(words, 0, false);
    assert!(!has(&right_only, "OB_DropBoard"));
    assert!(has(&right_only, "OB_ThrowBoard"));
    assert!(has(&right_only, "OB_RetrieveBoard"));
}

#[test]
fn sprint_uses_competing_held_timers_and_axes_preserve_zero_presence() {
    let mut words = [0; 26];
    words[20] = 0.01f32.to_bits();
    let output = values(words, u32::MAX, true);
    assert!(has(&output, "OB_Sprint"));
    assert!(!has(&output, "OB_LookAtX"));
    assert!(!has(&output, "OB_LookAtY"));
    assert!(has(&output, "OB_AirBodyTweakX"));
    assert!(has(&output, "OB_AirBodyTweakY"));
    words[21] = 0.01f32.to_bits();
    assert!(!has(&values(words, 0, false), "OB_Sprint"));
    words[9] = (-0.5f32).to_bits();
    words[13] = 1 << 30;
    let output = values(words, 0, false);
    assert!(has(&output, "OB_DoAirBodyTweak"));
    assert_eq!(
        output
            .iter()
            .find(|i| i.name == "OB_LookAtX")
            .unwrap()
            .value,
        -0.5
    );
    assert_eq!(
        output
            .iter()
            .find(|i| i.name == "OB_AirBodyTweakX")
            .unwrap()
            .value,
        -0.5
    );
}

fn analog(x: f32, z: f32, forward: [f32; 4], correction: Option<[f32; 4]>) -> [OffboardIntent; 4] {
    let mut words = [0; 26];
    words[7] = x.to_bits();
    words[8] = z.to_bits();
    produce_analog(
        &DerivedControllerInput::from_words(words),
        AnalogObservation {
            effective_skeleton_z: forward,
            biped_correction: correction,
        },
    )
}

#[test]
fn movement_preserves_signed_projection_and_raw_axes() {
    let output = analog(0.75, -0.5, [0.0, 8.0, 3.0, 0.0], None);
    assert!((output[0].value + 0.5).abs() < 1e-6);
    assert_eq!(output[1].value, -0.5);
    assert_eq!(output[2].value, 0.75);
    assert!((output[3].value - (0.75f32 * 0.75 + 0.25).sqrt()).abs() < 1e-6);
    let mirrored = analog(0.75, -0.5, [0.0, 0.0, -3.0, 0.0], None);
    assert!((mirrored[0].value - 0.5).abs() < 1e-6);
}

#[test]
fn obstacle_correction_stops_direct_approach_but_preserves_tangent_motion() {
    let forward = [0.0, 0.0, 1.0, 0.0];
    let wall = Some([0.0, 0.0, -1.0, 0.0]);
    assert_eq!(analog(0.0, 1.0, forward, wall)[0].value, 0.0);
    let tangent = analog(0.6, 0.8, [1.0, 0.0, 0.0, 0.0], wall);
    assert!((tangent[0].value - 0.6).abs() < 1e-6);
    assert!((analog(0.0, -0.5, forward, wall)[0].value + 0.5).abs() < 1e-6);
    //The source intentionally does not normalize the supplied correction vector.
    assert!((analog(0.0, 1.0, forward, Some([0.0, 0.0, -0.5, 0.0]))[0].value - 0.75).abs() < 1e-6);
}

#[test]
fn zero_and_tiny_headings_use_original_length_threshold() {
    assert_eq!(analog(0.0, 1.0, [0.0; 4], None)[0].value, 0.0);
    assert_eq!(
        analog(0.0, 1.0, [0.0, 0.0, 0.5e-6, 0.0], None)[0].value,
        0.0
    );
    assert!((analog(0.0, 1.0, [0.0, 0.0, 2.0e-6, 0.0], None)[0].value - 1.0).abs() < 1e-6);
    let zero = analog(0.0, 0.0, [0.0, 0.0, 1.0, 0.0], None);
    assert!(zero.iter().all(|intent| intent.value == 0.0));
}
