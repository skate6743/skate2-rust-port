//! Native controller history owner (`8296D0D0`, `8296D1F0`, `82699230`).
//!
//! The cache is thirty frame batches. Each batch contains four 100-byte
//! records, one per registered device. A record contains a value count and
//! up to 24 native float values. Startup is read=0 and write=1.

use super::pad::Pad;

pub const HISTORY_CAPACITY: usize = 30;
pub const DEVICE_SLOTS: usize = 4;
pub const MAX_VALUES: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistoryRecord {
    values: [f32; MAX_VALUES],
    count: usize,
}

impl HistoryRecord {
    pub fn new(values: &[f32]) -> Self {
        assert!(values.len() <= MAX_VALUES);
        let mut record = Self {
            values: [0.0; MAX_VALUES],
            count: values.len(),
        };
        record.values[..values.len()].copy_from_slice(values);
        record
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn values(&self) -> &[f32] {
        &self.values[..self.count]
    }
    /// Failed device polls clear publication count while retaining value words.
    pub fn clear_count(&mut self) {
        self.count = 0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PadHistory {
    batches: [[HistoryRecord; DEVICE_SLOTS]; HISTORY_CAPACITY],
    read: usize,
    write: usize,
}

impl PadHistory {
    /// Matches `826986A8`: zeroed storage, read index 0, write index 1.
    pub fn new() -> Self {
        let empty = HistoryRecord::new(&[]);
        Self {
            batches: [[empty; DEVICE_SLOTS]; HISTORY_CAPACITY],
            read: 0,
            write: 1,
        }
    }
    pub fn read_index(&self) -> usize {
        self.read
    }
    pub fn write_index(&self) -> usize {
        self.write
    }

    /// `8296D0D0` copies the registered-device prefix into one frame batch,
    /// then advances the producer once. Uncopied slots retain their contents;
    /// an empty poll still advances the producer.
    pub fn publish(&mut self, records: &[HistoryRecord]) {
        assert!(records.len() <= DEVICE_SLOTS);
        self.batches[self.write][..records.len()].copy_from_slice(records);
        self.write = (self.write + 1) % HISTORY_CAPACITY;
    }

    /// `82699230` drains to the newest available batch and updates all Pads
    /// exactly once with that batch.
    pub fn drain_to_latest(&mut self, pads: &mut [Pad; DEVICE_SLOTS]) -> bool {
        if self.is_empty() {
            return false;
        }
        let mut latest = self.read;
        while (self.read + 1) % HISTORY_CAPACITY != self.write {
            self.read = (self.read + 1) % HISTORY_CAPACITY;
            latest = self.read;
        }
        for (pad, record) in pads.iter_mut().zip(self.batches[latest].iter()) {
            pad.update(record.values());
        }
        true
    }

    fn is_empty(&self) -> bool {
        (self.read + 1) % HISTORY_CAPACITY == self.write
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
