use super::{ConstMgIntent, TimeMgIntent};
use crate::input::graph_intents::IntentMutation;

#[test]
fn constant_keeps_begin_value_for_one_frame_then_removes() {
    let mut intent = ConstMgIntent::new(2.5, false);
    assert_eq!(intent.begin(), IntentMutation::Set(2.5));
    assert_eq!(intent.update(), IntentMutation::None);
    assert_eq!(intent.update(), IntentMutation::Remove);
}

#[test]
fn constant_on_update_does_not_republish() {
    let mut intent = ConstMgIntent::new(-1.0, true);
    assert_eq!(intent.begin(), IntentMutation::Set(-1.0));
    assert_eq!(intent.update(), IntentMutation::None);
    assert_eq!(intent.update(), IntentMutation::None);
}

#[test]
fn constant_end_removes_without_changing_instance_state() {
    let mut intent = ConstMgIntent::new(2.5, false);
    intent.begin();
    let previous = intent;
    assert_eq!(intent.end(), IntentMutation::Remove);
    assert_eq!(intent, previous);
}

#[test]
fn time_uses_presence_and_accumulates_instance_clock() {
    let mut intent = TimeMgIntent::default();
    assert_eq!(intent.update(Some(0.0), 0.25), IntentMutation::Set(0.25));
    assert_eq!(intent.update(Some(99.0), 0.5), IntentMutation::Set(0.75));
    assert_eq!(intent.update(None, 0.5), IntentMutation::Remove);
    assert_eq!(intent.elapsed, 0.0);
}

#[test]
fn time_end_removes_without_resetting_instance_clock() {
    let mut intent = TimeMgIntent::default();
    assert_eq!(intent.update(Some(0.0), 0.25), IntentMutation::Set(0.25));
    assert_eq!(intent.end(), IntentMutation::Remove);
    assert_eq!(intent.elapsed, 0.25);
}
