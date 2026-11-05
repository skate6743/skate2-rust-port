use super::*;

fn input(state: i32, category: i32) -> FilteredStateInput {
    FilteredStateInput {
        physics_state: state,
        physics_category: category,
        anything_in_contact: false,
        physics_surface_type: 0,
        wall_ride_exit: false,
        targeting_grind: false,
        offboard_has_landed: false,
        offboard_on_deck: false,
        grind: GrindState::default(),
        last_grind_distance: 0.0,
    }
}

#[test]
fn stairs_preserve_ground_then_contact_returns_air_to_ground() {
    let mut state = FilteredState::default();
    let mut stairs = input(100, 100);
    stairs.physics_surface_type = 8;
    assert_eq!(state.update(stairs).category, FilteredCategory::Ground);
    for _ in 0..9 {
        assert_eq!(
            state.update(input(200, 200)).category,
            FilteredCategory::Ground
        );
    }
    assert_eq!(
        state.update(input(200, 200)).category,
        FilteredCategory::Air
    );
    let mut touching = input(200, 200);
    touching.anything_in_contact = true;
    let output = state.update(touching);
    assert_eq!(output.category, FilteredCategory::Ground);
    assert_eq!(output.previous_category, FilteredCategory::Air);
    state.update(input(702, 700));
    assert_eq!(
        state.update(input(200, 200)).category,
        FilteredCategory::Air
    );
}

#[test]
fn nonspecific_keeps_grind_metadata_and_leaving_clears_only_publication() {
    let mut state = FilteredState::default();
    let mut grinding = input(403, 400);
    grinding.grind = GrindState {
        kind: 3,
        scorable_id: 21,
        name: encode(b"fs smith"),
        scoring_name: encode(b"smith"),
        on_front: true,
        crouch: 0.25,
        pathed_guid: 0x123456789abcdef0,
        local_guid: 0xfedcba9876543210,
    };
    let entered = state.update(grinding);
    assert_eq!(entered.grind, grinding.grind);
    let retained = state.update(input(701, 700));
    assert!(retained.grinding);
    assert_eq!(retained.grind, grinding.grind);
    let left = state.update(input(100, 100));
    assert!(!left.grinding);
    assert_eq!(left.grind, GrindState::default());
    assert_eq!(state.cached_grind, grinding.grind);
    state.reset();
    assert_eq!(state.cached_grind, GrindState::default());
}
