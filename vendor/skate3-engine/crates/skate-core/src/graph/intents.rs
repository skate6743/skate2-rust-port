//! Canonical ActionGraph/MotionGraph intent identity.
//! TU3 FastString constructor823C3BF8 uses six words of82382B50 encoding.
//! Storage is ours; aliases share one value and the most recent write wins.
use crate::animation::playback_parameters::intent_key;
use std::collections::BTreeMap;

/// Decoded names cannot bypass native case folding, NUL termination or the
/// 36-byte limit on insert, read or removal. Kept separate from five-word
/// animation attribute names and graph time tags.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IntentMap {
    values: BTreeMap<[u32; 6], f32>,
}

impl IntentMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: f32) -> Option<f32> {
        self.values.insert(intent_key(name), value)
    }

    pub fn get(&self, name: &str) -> Option<&f32> {
        self.values.get(&intent_key(name))
    }

    pub fn remove(&mut self, name: &str) -> Option<f32> {
        self.values.remove(&intent_key(name))
    }

    pub fn contains_key(&self, name: &str) -> bool {
        self.values.contains_key(&intent_key(name))
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
