use super::{DEVICE_SLOTS, HISTORY_CAPACITY, HistoryRecord, PadHistory};
use crate::input::pad::Pad;

fn pads() -> [Pad; DEVICE_SLOTS] {
    [Pad::new(), Pad::new(), Pad::new(), Pad::new()]
}
fn batch(seed: f32) -> [HistoryRecord; DEVICE_SLOTS] {
    [
        HistoryRecord::new(&[seed]),
        HistoryRecord::new(&[seed + 1.0]),
        HistoryRecord::new(&[seed + 2.0]),
        HistoryRecord::new(&[seed + 3.0]),
    ]
}

#[test]
fn drains_newest_batch_once_for_all_devices() {
    let mut history = PadHistory::new();
    history.publish(&batch(10.0));
    history.publish(&batch(20.0));
    let mut output = pads();
    assert!(history.drain_to_latest(&mut output));
    for (pad, expected) in output.iter().zip([20.0_f32, 21.0, 22.0, 23.0]) {
        assert_eq!(pad.count(), 1);
        assert_eq!(pad.records()[0][0], expected.to_bits());
        assert_eq!(pad.records()[0][3], 1, "only the final batch advances Pad");
    }
    assert!(!history.drain_to_latest(&mut output));
}

#[test]
fn producer_wraps_at_full_lap_without_overflow_policy() {
    let mut history = PadHistory::new();
    for i in 0..HISTORY_CAPACITY - 1 {
        history.publish(&batch(i as f32));
    }
    assert_eq!(history.write_index(), 0);
    let mut output = pads();
    assert!(history.drain_to_latest(&mut output));
    history.publish(&batch(99.0));
    assert_eq!(history.write_index(), 1);
    assert!(history.drain_to_latest(&mut output));
    let mut full = PadHistory::new();
    for i in 0..HISTORY_CAPACITY {
        full.publish(&batch(i as f32));
    }
    assert_eq!(full.write_index(), 1);
    assert!(!full.drain_to_latest(&mut output));
}

#[test]
fn empty_publish_advances_once() {
    let mut history = PadHistory::new();
    history.publish(&batch(1.0));
    history.publish(&[]);
    assert_eq!(history.write_index(), 3);
}

#[test]
fn reused_batch_retains_unregistered_slots() {
    let mut history = PadHistory::new();
    history.batches[1] = batch(40.0);
    let mut output = pads();
    history.publish(&[HistoryRecord::new(&[99.0])]);
    assert!(history.drain_to_latest(&mut output));
    assert_eq!(output[0].records()[0][0], 99.0_f32.to_bits());
    assert_eq!(output[1].records()[0][0], 41.0_f32.to_bits());
    assert_eq!(output[2].records()[0][0], 42.0_f32.to_bits());
    assert_eq!(output[3].records()[0][0], 43.0_f32.to_bits());
}

#[test]
fn clear_count_preserves_visibility_contract() {
    let mut record = HistoryRecord::new(&[3.5, 4.5]);
    record.clear_count();
    assert_eq!(record.count(), 0);
    assert_eq!(record.values(), &[]);
    assert_eq!(&record.values[..2], &[3.5, 4.5]);
}
