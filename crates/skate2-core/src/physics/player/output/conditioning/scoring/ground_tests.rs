use super::super::{
    collector_base::CollectorBase, score_holder::ScoreHolder, sequence::GlobalMetrics,
};
use super::*;
use std::sync::{Arc, Mutex};

fn setup() -> (GroundCollector, TrickDatabase, Arc<Mutex<ScoreHolder>>) {
    // No tuning approximation: these tests exercise already-announced tricks
    // and native exit ordering. Empty trick class yields no optional patterns.
    let vault = Arc::new(crate::attribute_runtime::tests::graph(
        super::super::scoring_database::TRICK_CLASS,
        vec![],
    ));
    let output = Arc::new(());
    let metrics = Arc::new(Mutex::new(GlobalMetrics::new(output.clone())));
    metrics.lock().unwrap().tick = 60;
    let mut holder = ScoreHolder::before_sequence(output.clone(), &vault);
    holder.construct_sequence(Some(metrics.clone()));
    holder.finish_constructor().unwrap();
    holder.sequence.as_mut().unwrap().flags_word = 0x8000_0000;
    let holder = Arc::new(Mutex::new(holder));
    let ground = GroundCollector::new(CollectorBase::new(
        output,
        Some(metrics),
        Some(holder.clone()),
        &vault,
    ));
    (ground, TrickDatabase::load(&vault).unwrap(), holder)
}

fn frame() -> GroundFrame<'static> {
    GroundFrame {
        enabled: &[true; 14],
        tick: 60,
        dt: 1.0 / 60.0,
        deck_position: super::super::ZERO,
        deck_speed: 4.0,
        vel_direction: super::super::ZERO,
        switched: false,
        fakie: false,
        is_reverting: false,
        revert_exited_normally: false,
        minimum_distance: 0.001,
        allow_slow_manuals: true,
        allow_slow_manuals_after_begin: true,
    }
}

fn announced(trick: &mut super::super::scorable::Scorable, id: i32, score: f32) {
    trick.trick_scoring_settings.is_valid = true;
    trick.trick_scoring_settings.scorable_id = id;
    trick.is_beginning_announced = true;
    trick.score = score;
}

#[test]
fn manual_release_banks_tracker_once_and_publishes_finished_length() {
    let (mut g, db, holder) = setup();
    announced(&mut g.manual_scorable, 2, 15.0);
    g.manual_tracker.in_state = true;
    g.manual_tracker.score = 42.0;
    g.manual_tracker.total_distance = 8.0;
    let mut out = Scoring2::new();
    assert_eq!(g.update_manual(&db, &frame(), &mut out).unwrap(), 0.0);
    assert!(out.just_finished_manual);
    assert_eq!(out.raw_finished_manual, 2);
    assert_eq!(out.finished_manual_length, 8.0);
    assert_eq!(g.manual_scorable.valid_id(), None);
    assert!(!g.manual_tracker.in_state);
    // Repeated idle update must not duplicate the bank or announcement.
    g.update_manual(&db, &frame(), &mut out).unwrap();
    let h = holder.lock().unwrap();
    assert_eq!(h.sequence_score, 42.0);
    assert_eq!(h.line_accumulator[2], 1);
    assert_eq!(h.tricks_ended.num_entries, 1);
    assert_eq!(h.sequence.as_ref().unwrap().trick_count, 1);
}

#[test]
fn cancellation_announces_without_banking_or_repeat_penalty() {
    let (mut g, db, holder) = setup();
    announced(&mut g.manual_scorable, 3, 15.0);
    g.manual_tracker.in_state = true;
    g.manual_tracker.score = 90.0;
    let mut out = Scoring2::new();
    g.exit(false, &db, &frame(), &mut out).unwrap();
    let h = holder.lock().unwrap();
    assert_eq!(h.sequence_score, 0.0);
    assert_eq!(h.line_accumulator[3], 0);
    assert_eq!(h.tricks_cancelled.num_entries, 1);
    assert_eq!(h.tricks_ended.num_entries, 0);
    assert!(!out.just_finished_manual);
}

#[test]
fn normal_revert_exit_banks_once_and_publishes_raw_end() {
    let (mut g, db, holder) = setup();
    announced(&mut g.revert_scorable, 5, 33.0);
    g.reverting = true;
    g.most_recent_animation_revert = 2;
    let mut out = Scoring2::new();
    assert_eq!(g.update_revert(&db, &frame(), &mut out).unwrap(), 33.0);
    assert_eq!(out.raw_finished_revert, 5);
    assert!(!g.reverting);
    assert_eq!(g.most_recent_animation_revert, 0);
    assert_eq!(g.update_revert(&db, &frame(), &mut out).unwrap(), 0.0);
    let h = holder.lock().unwrap();
    assert_eq!(h.sequence_score, 33.0);
    assert_eq!(h.line_accumulator[5], 1);
    assert_eq!(h.tricks_ended.num_entries, 1);
}

#[test]
fn seventh_manual_revert_breaks_manual_without_restart_next_tick() {
    let (mut g, db, holder) = setup();
    announced(&mut g.manual_scorable, 2, 15.0);
    g.base.animation_trick.bits = 0x0800_0000;
    g.manuals_are_connectors = true;
    g.manual_revert_count = 6;
    g.manual_tracker.in_state = true;
    g.manual_tracker.score = 42.0;
    let mut f = frame();
    f.revert_exited_normally = true;
    let mut out = Scoring2::new();
    g.update_manual(&db, &f, &mut out).unwrap();
    assert_eq!(g.manual_revert_count, 7);
    assert!(!g.manual_tracker.in_state);
    f.revert_exited_normally = false;
    g.update_manual(&db, &f, &mut out).unwrap();
    assert!(!g.manual_tracker.in_state);
    assert_eq!(holder.lock().unwrap().line_accumulator[2], 1);
}
