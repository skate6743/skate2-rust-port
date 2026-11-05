use super::{BUCKET_COUNT, ENTRY_CAPACITY, Intent, IntentMap, IntentName};
use crate::input::graph_intents::IntentMutation;

fn key(seed: u32) -> IntentName {
    IntentName([seed, 0, 0, 0, 0, 0])
}

#[test]
fn set_overwrites_and_remove_preserves_other_chain_entries() {
    let mut map = IntentMap::default();
    let first = key(1);
    let second = key(1 + BUCKET_COUNT as u32);
    assert!(
        map.insert(Intent {
            name: first,
            value: 1.0
        })
        .unwrap()
    );
    assert!(
        map.insert(Intent {
            name: second,
            value: 2.0
        })
        .unwrap()
    );
    assert!(!map.set(first, 7.0).unwrap());
    assert_eq!(
        map.entries()
            .find(|entry| entry.name == first)
            .unwrap()
            .value,
        7.0
    );
    assert!(map.remove(first));
    assert!(!map.remove(first));
    assert_eq!(map.len(), 1);
    assert_eq!(map.entries().next().unwrap().name, second);
}

#[test]
fn apply_uses_existing_mutation_protocol_and_capacity() {
    let mut map = IntentMap::default();
    let name = key(42);
    assert!(map.apply(name, IntentMutation::Set(3.0)).unwrap());
    assert!(!map.apply(name, IntentMutation::None).unwrap());
    assert!(!map.apply(name, IntentMutation::Set(4.0)).unwrap());
    assert_eq!(map.entries().next().unwrap().value, 4.0);
    assert!(map.apply(name, IntentMutation::Remove).unwrap());
    assert_eq!(map.len(), 0);
}

#[test]
fn insert_rejects_only_after_native_fixed_capacity() {
    let mut map = IntentMap::default();
    for seed in 0..ENTRY_CAPACITY as u32 {
        assert!(
            map.insert(Intent {
                name: key(seed),
                value: seed as f32
            })
            .is_ok()
        );
    }
    assert!(
        map.insert(Intent {
            name: key(ENTRY_CAPACITY as u32),
            value: 0.0
        })
        .is_err()
    );
}
