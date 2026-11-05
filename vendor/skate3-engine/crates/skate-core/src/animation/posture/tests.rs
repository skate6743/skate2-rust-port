use super::{PendingPosture, PosturePose};

#[test]
fn native_default_stays_disabled_even_after_request() {
    let mut state = PendingPosture::default();
    assert_eq!(state.profile(), 0);
    assert!(!state.is_pending());
    state.set_requested(true);
    let tree = state.apply::<_, ()>(7, |_, _| panic!("disabled profile wrapped"));
    assert_eq!(tree, Ok(7));
    assert!(state.is_pending());
}

#[test]
fn profile_values_follow_registration_not_alphabetical_order() {
    for (value, name) in [
        (1, "POSTURE_STIFF_POSE"),
        (2, "POSTURE_SLOUCH_POSE"),
        (3, "POSTURE_BUFF_POSE"),
    ] {
        assert_eq!(PosturePose::from_profile(value).unwrap().name(), name);
    }
    for value in [0, 4, u32::MAX] {
        assert_eq!(PosturePose::from_profile(value), None);
    }
}

#[test]
fn failed_construction_keeps_request_for_fallback() {
    let mut state = PendingPosture::default();
    state.set_profile(2);
    state.set_requested(true);
    assert_eq!(
        state.apply((), |_, _| Err("missing pose")),
        Err("missing pose")
    );
    assert!(state.is_pending());
    let tree = state
        .apply::<_, ()>(Vec::new(), |mut tree, pose| {
            tree.push(pose);
            Ok(tree)
        })
        .unwrap();
    assert_eq!(tree, [PosturePose::Slouch]);
    assert!(!state.is_pending());
}

#[test]
fn synchronous_begins_each_wrap_without_changing_previous_tree() {
    let mut state = PendingPosture::default();
    state.set_profile(1);
    let mut trees = Vec::new();
    for requested in [true, true, false] {
        state.set_requested(requested);
        trees.push(
            state
                .apply::<_, ()>(None, |_, pose| Ok(Some(pose)))
                .unwrap(),
        );
    }
    assert_eq!(
        trees,
        [Some(PosturePose::Stiff), Some(PosturePose::Stiff), None]
    );
    assert!(!state.is_pending());
}

#[test]
fn false_overwrites_unconsumed_request_and_refresh_does_not_request() {
    let mut state = PendingPosture::default();
    state.set_requested(true);
    state.set_requested(false);
    state.set_profile(3);
    assert_eq!(state.selected_pose(), None);
    state.set_requested(true);
    assert_eq!(state.selected_pose(), Some(PosturePose::Buff));
    state.set_profile(99);
    assert_eq!(state.selected_pose(), None);
    assert!(state.is_pending());
    state.set_profile(1);
    assert_eq!(state.selected_pose(), Some(PosturePose::Stiff));
}
