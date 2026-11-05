//! Typed animation allocations. Slots identify whole allocations, never addresses.
use crate::animation::output::{NativeMatrix, Sqt};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) fn new_owner() -> usize {
    static NEXT_OWNER: AtomicUsize = AtomicUsize::new(0);
    NEXT_OWNER
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .expect("animation storage identity exhausted")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Buffer<T> {
    slot: usize,
    owner: usize,
    marker: PhantomData<T>,
}
impl<T: Copy> Buffer<T> {
    pub fn at(self, first_bone: usize) -> BoneSlice<T> {
        BoneSlice {
            buffer: self,
            first_bone,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoneSlice<T> {
    pub buffer: Buffer<T>,
    pub first_bone: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferError {
    UnknownBuffer,
    RangeOutsideAllocation,
    UninitializedBone {
        bone: usize,
    },
    /// The native copy helper's partially overlapping behavior is not recovered.
    UnsupportedPartialOverlap,
}
pub struct BoneBuffers<T> {
    owner: usize,
    allocations: Vec<Vec<Option<T>>>,
}
impl<T: Copy> Default for BoneBuffers<T> {
    fn default() -> Self {
        Self {
            owner: new_owner(),
            allocations: Vec::new(),
        }
    }
}
impl<T: Copy> BoneBuffers<T> {
    /// Allocate exactly the requested extent without inventing a bind pose.
    pub fn allocate(&mut self, count: usize) -> Buffer<T> {
        self.insert(vec![None; count])
    }
    pub fn initialized(&mut self, values: impl IntoIterator<Item = T>) -> Buffer<T> {
        self.insert(values.into_iter().map(Some).collect())
    }
    fn insert(&mut self, allocation: Vec<Option<T>>) -> Buffer<T> {
        let slot = self.allocations.len();
        self.allocations.push(allocation);
        Buffer {
            slot,
            owner: self.owner,
            marker: PhantomData,
        }
    }
    fn range(
        &self,
        source: BoneSlice<T>,
        count: usize,
    ) -> Result<std::ops::Range<usize>, BufferError> {
        if source.buffer.owner != self.owner {
            return Err(BufferError::UnknownBuffer);
        }
        let allocation = self
            .allocations
            .get(source.buffer.slot)
            .ok_or(BufferError::UnknownBuffer)?;
        let end = source
            .first_bone
            .checked_add(count)
            .ok_or(BufferError::RangeOutsideAllocation)?;
        if end > allocation.len() {
            return Err(BufferError::RangeOutsideAllocation);
        }
        Ok(source.first_bone..end)
    }
    pub fn read(&self, source: BoneSlice<T>, count: usize) -> Result<Vec<T>, BufferError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let range = self.range(source, count)?;
        range
            .map(|bone| {
                self.allocations[source.buffer.slot][bone]
                    .ok_or(BufferError::UninitializedBone { bone })
            })
            .collect()
    }
    pub(crate) fn copy(
        &mut self,
        source: BoneSlice<T>,
        destination: BoneSlice<T>,
        count: usize,
    ) -> Result<(), BufferError> {
        if count == 0 {
            return Ok(());
        }
        let from = self.range(source, count)?;
        let to = self.range(destination, count)?;
        if source.buffer.slot == destination.buffer.slot {
            if from == to {
                return Ok(());
            }
            if from.start < to.end && to.start < from.end {
                return Err(BufferError::UnsupportedPartialOverlap);
            }
        }
        // Copy validity alongside the value: native SET does not initialize
        // unwritten bones. Such exports remain unreadable, rather than zero-filled.
        for (from, to) in from.zip(to) {
            self.allocations[destination.buffer.slot][to] =
                self.allocations[source.buffer.slot][from];
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct PoseBuffers {
    pub sqt: BoneBuffers<Sqt>,
    pub matrices: BoneBuffers<NativeMatrix>,
}
