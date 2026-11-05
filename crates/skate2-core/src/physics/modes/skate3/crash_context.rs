//! Infrequent, allowlisted snapshots: no player names, addresses or lobby credentials.
use crate::skate3_native::math_host::*;
use skate3_core::player::state::PhysicalStateId;

type Transition = (u64, PhysicalStateId, PhysicalStateId);
// Recording a transition copies three scalars. Formatting happens once per second.
static EVENTS: std::sync::Mutex<([Option<Transition>; 64], usize, u64)> =
    std::sync::Mutex::new(([None; 64], 0, 0));

pub(crate) fn requested(tick: u64, from: PhysicalStateId, to: PhysicalStateId) {
    if let Ok(mut events) = EVENTS.try_lock() {
        let index = events.1;
        events.0[index] = Some((tick, from, to));
        events.1 = (index + 1) % 64;
        events.2 += 1;
    }
}

