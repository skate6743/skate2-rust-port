use super::*;
use skate3_core::animation::playback_parameters::SettableAttributes;
#[test]
fn biped_updates_the_existing_phase_only_during_update() {
    let mut phase = 0.25;
    biped_cadence(Some(0.8), Some(&mut phase), 0);
    assert_eq!(phase, 0.25);
    biped_cadence(Some(0.8), Some(&mut phase), 1);
    assert_eq!(phase, 0.8);
    biped_cadence(None, Some(&mut phase), 1);
    biped_cadence(Some(0.1), Some(&mut phase), 2);
    assert_eq!(phase, 0.8);
}
#[test]
fn match_captures_begin_and_republishes_without_consuming_it() {
    let mut state = MatchCadence::default();
    let mut sink = SettableAttributes::default();
    state.begin(Some(0.73), true);
    assert!(state.pending);
    state.update(Some(&mut sink));
    assert!(!state.pending);
    let value = sink.entries()[0];
    assert_eq!(value.name, encode(b"CadenceStartPercent"));
    assert_eq!(value.value, 0.73);
    assert!(!value.normalized);
    assert_eq!(value.sequence_id, -1);
    sink.clear();
    state.end();
    state.update(Some(&mut sink));
    assert_eq!(sink.entries()[0].value, 0.73);
}
#[test]
fn absent_components_preserve_retained_capture_and_pending_flag() {
    let mut state = MatchCadence::default();
    state.begin(Some(0.4), true);
    state.begin(Some(0.9), false);
    state.begin(None, true);
    state.update(None);
    assert_eq!(state.captured_phase, 0.4);
    assert!(state.pending);
    state.end();
    assert!(state.pending);
}
#[test]
fn native_initial_capture_is_zero_before_any_begin() {
    let mut state = MatchCadence::default();
    let mut sink = SettableAttributes::default();
    state.update(Some(&mut sink));
    assert_eq!(sink.entries()[0].value, 0.0);
}
#[test]
fn locomotion_comparison_does_not_clamp_unknown_physical_values() {
    for expected in 0..4 {
        let condition = LocoState { expected };
        for actual in 0..5 {
            assert_eq!(condition.evaluate(actual), expected == actual);
        }
    }
}
#[test]
fn missing_locomotion_config_is_not_silently_stand() {
    assert!(LocoState::parse(&Attributes::new(&[])).is_err());
}
