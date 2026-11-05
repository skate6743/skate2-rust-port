use super::ContactRecord;
use super::arithmetic::{dot, sub, vector};
use super::reduction::reduce;

/// Native TempContactBuffer state (+0/+4/+8/+12/+16, +128 records,
/// +12940/+12941/+12942 flags). Caller supplies the actual constructor settings.
/// Guest allocator/publication pointers are represented by the flush callback.
#[derive(Clone, Debug)]
pub struct ContactBuffer {
    pub count: u32,
    pub flushed: u32,
    pub capacity: u32,
    pub dropped: u32,
    pub distance_squared_threshold: f32,
    pub allow_flush: u8,
    pub deferred_reduction: u8,
    pub full: u8,
    pub records: [ContactRecord; 50],
}

impl ContactBuffer {
    /// Complete 82779D60. A full physical buffer with flushing disabled sets
    /// the full flag but does not increment dropped until a subsequent call.
    pub fn allocate(&mut self, sink: &mut impl FnMut(&[ContactRecord])) -> Option<usize> {
        assert!(self.count <= 50);
        if self.count.wrapping_add(self.flushed) >= self.capacity {
            if self.full == 0 {
                if self.deferred_reduction != 0 {
                    self.reduce();
                }
                if self.count.wrapping_add(self.flushed) >= self.capacity {
                    self.full = 1;
                }
            }
            if self.full != 0 {
                self.dropped = self.dropped.wrapping_add(1);
                return None;
            }
        }
        if self.count >= 50 {
            if self.allow_flush == 0 {
                self.full = 1;
                return None;
            }
            self.flush(sink);
        }
        let index = self.count as usize;
        self.count += 1;
        Some(index)
    }

    /// Complete 82779C18. Tests the most recently allocated record against all
    /// previous records. The native caller, not this function, decrements count.
    pub fn last_is_duplicate(&mut self) -> bool {
        if self.distance_squared_threshold < 0.0 || self.deferred_reduction != 0 {
            return false;
        }
        assert!(self.count > 0 && self.count <= 50);
        let current = &self.records[self.count as usize - 1];
        for previous in &self.records[..self.count as usize - 1] {
            if previous[3] != current[3] || previous[7] != current[7] {
                continue;
            }
            let a = sub(vector(previous, 0), vector(current, 0));
            let b = sub(vector(previous, 4), vector(current, 4));
            if self.distance_squared_threshold > dot(a, a)
                && dot(b, b) < self.distance_squared_threshold
                && f64::from(dot(vector(previous, 8), vector(current, 8)))
                    > f64::from_bits(0x3fee_147a_e147_ae14)
            {
                self.dropped = self.dropped.wrapping_add(1);
                return true;
            }
        }
        false
    }

    pub fn reduce(&mut self) {
        reduce(&mut self.records, &mut self.count);
    }

    /// Complete 82779E40; sink is the caller's 8277B428 publication operation.
    /// Flushed counts advance by the reduced count, irrespective of sink result.
    pub fn flush(&mut self, sink: &mut impl FnMut(&[ContactRecord])) {
        if self.count == 0 {
            return;
        }
        if self.deferred_reduction != 0 {
            self.reduce();
        }
        sink(&self.records[..self.count as usize]);
        self.flushed = self.flushed.wrapping_add(self.count);
        self.count = 0;
    }
}
