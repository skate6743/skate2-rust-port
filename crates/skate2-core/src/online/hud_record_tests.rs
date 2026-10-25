use super::*;
use crate::ui::trick_backend::{DisplayEvent, TricksBackend};
use std::sync::Arc;

const KICKFLIP: u64 = 0x1234_5678_9abc_def0;

fn kickflip() -> TrickName {
    TrickName { index: 3, attrib_key: KICKFLIP, valid: true, label: Some(Arc::from("ID_TRICK_KICKFLIP")), accepts_spin: true, nollie: false, fakie_variant: None }
}

fn names(key: u64) -> Option<TrickName> {
    (key == KICKFLIP).then(kickflip)
}

/// A line: a kickflip begins a sequence in the air, spins, lands, the sequence completes, and a
/// big-trick context shows.
fn owner_frames() -> Vec<PhysicsFrame> {
    vec![
        PhysicsFrame::default(),
        PhysicsFrame { begun: Some(kickflip()), sequence_begun: true, aerial: true, spins: 1, multiplier: 1.0, ..Default::default() },
        PhysicsFrame { aerial: true, spins: 2, hud_score: 300, multiplier: 1.5, ..Default::default() },
        PhysicsFrame {
            air_finished: true,
            just_landed: true,
            landed_height: 2.5,
            ended_count: 1,
            hud_score: 500,
            multiplier: 1.5,
            context_entries: vec![Entry::Big { index: 5, achievement: 1, measurement: 1.25 }],
            ..Default::default()
        },
        PhysicsFrame { sequence_completed: true, finished_sequence_score: 750, finished_tricks: 2, finished_taps: 1, line_score: 750, line_seconds: 4.0, line_points: 20, ..Default::default() },
        PhysicsFrame { line_score: 750, line_seconds: 3.5, line_points: 20, ..Default::default() },
    ]
}

/// What a backend shows after taking `frames`, and every display event it raised.
fn run(frames: &[PhysicsFrame]) -> (TricksBackend, Vec<DisplayEvent>) {
    let mut backend = TricksBackend::default();
    let mut events = Vec::new();
    for frame in frames {
        backend.consume(frame, &mut events).unwrap();
    }
    (backend, events)
}

fn same_display(a: &TricksBackend, b: &TricksBackend) {
    assert_eq!(a.current, b.current);
    assert_eq!(a.total_tricks, b.total_tricks);
    assert_eq!(a.sequence_score, b.sequence_score);
    assert_eq!(a.tricks_in_sequence, b.tricks_in_sequence);
    assert_eq!(a.multiplier, b.multiplier);
    assert_eq!(a.line_score, b.line_score);
    assert_eq!(a.line_seconds, b.line_seconds);
    assert_eq!(a.line_points, b.line_points);
    assert_eq!((a.spins, a.flips, a.switched, a.fakie), (b.spins, b.flips, b.switched, b.fakie));
    assert_eq!((a.sequence_on, a.sequence_sketchy, a.wiped_out, a.converted), (b.sequence_on, b.sequence_sketchy, b.wiped_out, b.converted));
}

/// The owner's frames through the record, the follower and back, one local tick per owner tick,
/// leaving out the ticks in `lost`.
fn spectated(frames: &[PhysicsFrame], lost: &[usize]) -> Vec<PhysicsFrame> {
    let mut recorder = HudRecorder::default();
    let mut follower = HudFollower::default();
    let mut shown = Vec::new();
    for (tick, frame) in frames.iter().enumerate() {
        recorder.record(frame);
        if !lost.contains(&tick) {
            follower.pass(u32::try_from(tick).unwrap(), &recorder.words());
        }
        let steps = follower.take();
        shown.extend(followed_frames(&steps, follower.latest(), PhysicsFrame::default(), names));
    }
    shown
}

#[test]
fn a_spectator_shows_what_the_owner_shows() {
    let frames = owner_frames();
    let (owner, owner_events) = run(&frames);
    let (spectator, spectator_events) = run(&spectated(&frames, &[]));
    same_display(&owner, &spectator);
    assert_eq!(owner_events, spectator_events);
    assert!(owner_events.contains(&DisplayEvent::Start) && owner_events.contains(&DisplayEvent::EndSequence));
}

#[test]
fn a_lost_tick_delays_its_events_but_never_loses_or_repeats_them() {
    let frames = owner_frames();
    let (owner, owner_events) = run(&frames);
    for lost in 1..frames.len() - 1 {
        let (spectator, spectator_events) = run(&spectated(&frames, &[lost]));
        same_display(&owner, &spectator);
        let count = |events: &[DisplayEvent], event: &DisplayEvent| events.iter().filter(|e| *e == event).count();
        for event in &owner_events {
            assert_eq!(count(&spectator_events, event), count(&owner_events, event), "tick {lost} lost: {event:?}");
        }
    }
}

#[test]
fn a_gap_hiding_an_end_and_a_start_ends_first() {
    // The sequence completes, then a new trick begins one; the tick between is lost.
    let frames = vec![
        PhysicsFrame::default(),
        PhysicsFrame { begun: Some(kickflip()), sequence_begun: true, ..Default::default() },
        PhysicsFrame { ended_count: 1, ..Default::default() },
        PhysicsFrame { sequence_completed: true, finished_sequence_score: 100, finished_tricks: 1, ..Default::default() },
        PhysicsFrame { begun: Some(kickflip()), sequence_begun: true, ..Default::default() },
    ];
    let shown = spectated(&frames, &[3]);
    let (spectator, _) = run(&shown);
    assert!(spectator.sequence_on, "the new sequence is the one running");
    assert_eq!(spectator.current, Some(kickflip()));
    let ends = shown.iter().position(|frame| frame.sequence_completed).unwrap();
    let starts = shown.iter().rposition(|frame| frame.sequence_begun && frame.begun.is_some() && !frame.sequence_completed).unwrap();
    assert!(ends < starts);
}

#[test]
fn following_starts_from_the_state_then_not_the_events_before() {
    let mut recorder = HudRecorder::default();
    recorder.record(&PhysicsFrame { begun: Some(kickflip()), sequence_begun: true, hud_score: 40, multiplier: 2.0, ..Default::default() });
    let mut follower = HudFollower::default();
    follower.pass(10, &recorder.words());
    let shown = followed_frames(&follower.take(), follower.latest(), PhysicsFrame::default(), names);
    assert_eq!(shown.len(), 1);
    assert!(shown[0].begun.is_none() && !shown[0].sequence_begun);
    assert_eq!((shown[0].hud_score, shown[0].multiplier), (40, 2.0));
}

#[test]
fn a_tick_without_a_record_holds_the_newest_values() {
    let mut recorder = HudRecorder::default();
    recorder.record(&PhysicsFrame { line_score: 900, line_seconds: 2.0, ..Default::default() });
    let mut follower = HudFollower::default();
    follower.pass(0, &recorder.words());
    follower.take();
    let local = PhysicsFrame { simulation_rate: 1.0 / 60.0, line_max_points: 50, ..Default::default() };
    let shown = followed_frames(&[], follower.latest(), local, names);
    assert_eq!(shown.len(), 1);
    assert_eq!((shown[0].line_score, shown[0].line_seconds, shown[0].line_max_points), (900, 2.0, 50));
    assert_eq!(shown[0].simulation_rate, 1.0 / 60.0);
}

#[test]
fn the_local_commands_ride_the_first_frame_only() {
    use crate::ui::trick_backend::BackendCommand;
    let frames = owner_frames();
    let mut recorder = HudRecorder::default();
    let mut follower = HudFollower::default();
    for (tick, frame) in frames.iter().enumerate().take(4) {
        recorder.record(frame);
        follower.pass(u32::try_from(tick).unwrap(), &recorder.words());
    }
    let local = PhysicsFrame { backend_commands: vec![BackendCommand::SetIsJacked(true)], ..Default::default() };
    let shown = followed_frames(&follower.take(), follower.latest(), local, names);
    assert!(shown.len() >= 2);
    assert_eq!(shown[0].backend_commands.len(), 1);
    assert!(shown[1..].iter().all(|frame| frame.backend_commands.is_empty()));
}

fn begun_with(trick: u64) -> Option<TrickName> {
    let mut words = HudRecord::default().words();
    let mut follower = HudFollower::default();
    follower.pass(0, &words);
    words[word::TRICK] = u32::try_from(trick >> 32).unwrap();
    words[word::TRICK + 1] = u32::try_from(trick & 0xffff_ffff).unwrap();
    words[word::COUNTS] = u32::from_be_bytes([1, 0, 0, 0]);
    follower.pass(1, &words);
    followed_frames(&follower.take(), follower.latest(), PhysicsFrame::default(), names).into_iter().find_map(|frame| frame.begun)
}

#[test]
fn a_trick_the_database_does_not_hold_shows_no_name() {
    assert_eq!(begun_with(KICKFLIP), Some(kickflip()));
    // The scoring's own "no valid trick" shows as the original shows it.
    assert_eq!(begun_with(0), Some(TrickName { index: -1, ..Default::default() }));
    for unknown in [1, KICKFLIP + 1, 0xdead_beef, u64::MAX] {
        assert_eq!(begun_with(unknown), None, "{unknown:x}");
    }
}

#[test]
fn an_invalid_trick_of_the_owners_scoring_travels_as_none() {
    let mut recorder = HudRecorder::default();
    recorder.record(&PhysicsFrame { begun: Some(TrickName { index: -1, attrib_key: 77, ..Default::default() }), ..Default::default() });
    assert_eq!(HudRecord::from_words(&recorder.words()).unwrap().trick, 0);
}

#[test]
fn malformed_records_are_dropped() {
    let good = HudRecorder::default().words();
    assert!(HudRecord::from_words(&good).is_some());
    let broken = |change: &dyn Fn(&mut Vec<u32>)| {
        let mut words = good.clone();
        change(&mut words);
        HudRecord::from_words(&words).is_none()
    };
    assert!(broken(&|w| w.truncate(FIXED_WORDS - 1)));
    assert!(broken(&|w| w.push(0)));
    assert!(broken(&|w| w.resize(MAX_WORDS + 2, 0)));
    assert!(broken(&|w| w[word::FLAGS] = 1 << 6));
    assert!(broken(&|w| w[word::SPINS] = word_of(MAX_SPINS + 1)));
    assert!(broken(&|w| w[word::FLIPS] = word_of(i32::MIN)));
    assert!(broken(&|w| w[word::MULTIPLIER] = f32::NAN.to_bits()));
    assert!(broken(&|w| w[word::MULTIPLIER] = (-1.0f32).to_bits()));
    assert!(broken(&|w| w[word::LINE_SECONDS] = f32::INFINITY.to_bits()));
    assert!(broken(&|w| w[word::LINE_POINTS] = word_of(i32::MAX)));
    assert!(broken(&|w| w[word::FINISHED_COUNTS] = (1 << 16) | 2));
    assert!(broken(&|w| w[word::LANDED_DROP] = f32::NAN.to_bits()));
    assert!(broken(&|w| w.extend([BIG | 15, 0])));
    assert!(broken(&|w| w.extend([BIG | (2 << 8), 0])));
    assert!(broken(&|w| w.extend([BIG | 0x1_0000, 0])));
    assert!(broken(&|w| w.extend([BIG, f32::NAN.to_bits()])));
    assert!(broken(&|w| w.extend([ENTITY | 68, 0])));
    assert!(broken(&|w| w.extend([ENTITY | 3, 1])));
    assert!(broken(&|w| w.extend([BIG | ENTITY, 0])));
    assert!(broken(&|w| w.extend([0, 0])));
    assert!(!broken(&|w| w.extend([BIG | (1 << 8) | 14, 3.0f32.to_bits(), ENTITY | 67, 0])));
}

#[test]
fn a_dropped_record_counts_as_lost() {
    let frames = owner_frames();
    let mut recorder = HudRecorder::default();
    let mut follower = HudFollower::default();
    let mut shown = Vec::new();
    for (tick, frame) in frames.iter().enumerate() {
        recorder.record(frame);
        let mut words = recorder.words();
        if tick == 2 {
            words[word::FLAGS] = u32::MAX;
        }
        follower.pass(u32::try_from(tick).unwrap(), &words);
        shown.extend(followed_frames(&follower.take(), follower.latest(), PhysicsFrame::default(), names));
    }
    let (owner, _) = run(&frames);
    let (spectator, _) = run(&shown);
    same_display(&owner, &spectator);
}

#[test]
fn the_sender_holds_its_values_to_what_a_receiver_takes() {
    let mut recorder = HudRecorder::default();
    recorder.record(&PhysicsFrame {
        spins: i32::MAX,
        flips: i32::MIN,
        multiplier: f32::NAN,
        line_seconds: f32::INFINITY,
        line_points: i32::MIN,
        sequence_completed: true,
        finished_tricks: 1,
        finished_taps: 5,
        air_finished: true,
        landed_height: f32::NAN,
        context_entries: vec![
            Entry::Big { index: 40, achievement: 0, measurement: 1.0 },
            Entry::Entity { entity_type: 9, is_entity: true, count: 3, context_type: 0 },
            Entry::Big { index: 2, achievement: 1, measurement: f32::INFINITY },
        ],
        ..Default::default()
    });
    let record = HudRecord::from_words(&recorder.words()).expect("a sender's own record is never refused");
    assert_eq!(record.contexts, vec![Entry::Big { index: 2, achievement: 1, measurement: 0.0 }]);
    assert!(record.finished_taps <= record.finished_tricks);
}

#[test]
fn counts_wrap() {
    let mut recorder = HudRecorder::default();
    let mut follower = HudFollower::default();
    follower.pass(0, &recorder.words());
    follower.take();
    let mut begun = 0;
    for tick in 1..=600 {
        recorder.record(&PhysicsFrame { begun: Some(kickflip()), ..Default::default() });
        follower.pass(tick, &recorder.words());
        begun += followed_frames(&follower.take(), follower.latest(), PhysicsFrame::default(), names).iter().filter(|frame| frame.begun.is_some()).count();
    }
    assert_eq!(begun, 600);
}
