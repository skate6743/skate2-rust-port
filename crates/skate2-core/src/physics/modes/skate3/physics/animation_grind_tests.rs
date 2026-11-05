//! Independent publication tests; these do not simulate physical producers.
use super::*;
use skate3_core::physics::filtered_state::FilteredCategory;

#[test]
fn encoded_name_boundary_is_lossless_and_rejects_malformed_names() {
    for text in [
        "",
        "BF_50_50",
        "FS_BLUNT",
        "BS_NOSESLIDE",
        "0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    ] {
        let name = encode(text.as_bytes());
        assert_eq!(encode(name_text(name).unwrap().as_bytes()), name);
    }
    assert!(name_text(AttributeName([u32::MAX, 0, 0, 0, 0])).is_err());
    assert!(name_text(AttributeName([0, 79_235_168, 0, 0, 0])).is_err());
}

#[test]
fn completed_output_fields_reach_both_graph_boundaries_without_proxies() {
    let mut p = PhysicalPlayerInput::default();
    p.filtered_state_0 = FilteredCategory::Grind as u32;
    p.skateboard.vector_80 = [1.0f32, 2.0, 3.0, 4.0].map(f32::to_bits);
    let board_forward = [5.0, 6.0, 7.0];
    p.ground.flag_273 = 2;
    p.skeleton.twist_504 = -0.7;
    p.grinds.words_136_140[0] = 4;
    p.grinds.trick_out_240 = 2;
    p.grinds.animation_chromosome_268[0] = 1;
    p.grinds.dropping_in_324 = 3;
    p.air.flag_443 = 5;
    p.air.scalar_184 = 0.15;
    let filtered = FilteredStateOutput {
        category: FilteredCategory::Grind,
        previous_category: FilteredCategory::Air,
        grinding: true,
        grind: GrindState {
            name: encode(b"BF_50_50"),
            crouch: 0.3,
            ..GrindState::default()
        },
        last_grind_distance: 0.0,
    };
    let (state, grind, conditions) =
        observations(&p, Some(&filtered), 0.8, true, board_forward).unwrap();
    assert!(state.grinding);
    assert_eq!(encode(state.grind_name.as_bytes()), filtered.grind.name);
    assert_eq!(grind.grind_name, filtered.grind.name);
    assert_eq!(grind.ground_axis, [1.0, 2.0, 3.0]);
    assert_eq!(grind.board_axis, [5.0, 6.0, 7.0]);
    assert!(grind.ground_flag_273 && grind.animation_mirrored);
    assert_eq!((grind.height, grind.crouch, grind.twist), (0.8, 0.3, -0.7));
    use motion_grind::conditions::Condition;
    for condition in [
        Condition::BluntingBackslash,
        Condition::Approach { forwards: true },
        Condition::TrickOutType { value: 2 },
        Condition::LandingIntoGrind,
        Condition::DroppingIn,
    ] {
        assert!(condition.evaluate(&conditions));
    }
    // No physical state/contact/holding-board proxies were supplied above.
    assert_eq!(p.state.state_16, 0);
    assert_eq!(p.grinds.flag_318, 0);
    assert_eq!(p.off_board.flag_304, 0);
    assert!(observations(&p, None, 0.8, true, board_forward).is_err());
    p.filtered_state_0 = FilteredCategory::Air as u32;
    assert!(observations(&p, Some(&filtered), 0.8, true, board_forward).is_err());
}

#[test]
fn initial_reset_is_not_an_invented_grind_publication() {
    let (state, grind, _) =
        observations(&PhysicalPlayerInput::default(), None, 0.0, false, [0.0; 3]).unwrap();
    assert!(!state.grinding && !grind.grinding);
    assert_eq!(state.category, 0);
    assert_eq!(state.grind_name, "");
    assert_eq!(grind.crouch, 0.0);
}
