use super::*;

fn settings() -> LandingSettings {
    let graph = PointGraph {
        x: [0.0, 0.3, 0.6, 1.0],
        y: [1.0; 4],
    };
    LandingSettings {
        manual_blend: graph,
        grind_blend: graph,
        coffin_height: graph,
        minimum_height: 0.5,
        maximum_velocity: 8.0,
        manual_damping: 17.5,
        manual_spring: 60.0,
        ground_minimum_compression_time: 0.05,
        ground_damping: 6.0,
        ground_spring: 60.0,
        grind_animation_target_time: 0.15,
        grind_damping: 10.0,
        grind_spring: 60.0,
        grind_target_delta: 0.01,
        desired_com_height: 0.85,
        coffin_time: 0.04,
        coffin_maximum_velocity: 5.0,
        coffin_blend_frames: 7.0,
        coffin_base_height: 0.2,
    }
}
#[test]
fn ground_landing_uses_physical_height_velocity_and_preserves_release_order() {
    let mut adjustment = LandingAdjustment::default();
    let mut offset = SkateboardOffset::default();
    let mut input = LandingInput {
        filtered_state: 2,
        flags_2468: 0,
        flags_2472: 0,
        flags_2476: 0,
        balance: 0.0,
        physical_com_velocity_along_up: -2.0,
        physical_com_height: 0.8,
        animation_com_height: 1.0,
    };
    adjustment.update(input, &settings(), &mut offset);
    assert!(!adjustment.active);
    input.filtered_state = 1;
    input.flags_2476 = 0x1000_0000;
    adjustment.update(input, &settings(), &mut offset);
    assert!(adjustment.active);
    assert_eq!(adjustment.kind, 0);
    assert!((adjustment.velocity + 1.75).abs() < 1e-6);
    assert!((offset.transform[3][1] - 0.205).abs() < 1e-6);
    assert_eq!(offset.height_frames, 15.0);
    assert!(offset.height_refreshed);
    input.filtered_state = 2;
    adjustment.update(input, &settings(), &mut offset);
    assert!(!adjustment.active);
    assert_eq!(adjustment.time, STEP + STEP);
    assert_eq!(adjustment.previous_filtered_state, 2);
}
