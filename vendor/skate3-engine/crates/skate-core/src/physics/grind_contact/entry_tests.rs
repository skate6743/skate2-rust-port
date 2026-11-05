use super::*;

const HELP: PointGraph<4> = PointGraph {
    x: [-1., 0., 0.5, 1.],
    y: [0.; 4],
};
fn input() -> Input<'static> {
    Input {
        valid: true,
        kind: 0,
        category: 200,
        previous_state_2504: 200,
        speed: 4.,
        balance_2720: 0.,
        direction: [1., 0., 0., 0.],
        normal: [0., 1., 0., 0.],
        up: [0., 1., 0., 0.],
        board_velocity: [4., -2., 3., 0.],
        air_velocity: [4., -2., 3., 0.],
        surface_kind: 0,
        high_side: [0., 0., 1., 0.],
        vertical_help: &HELP,
        max_delta: 100.,
        flags: 0,
        previous_entry_velocity: [9.; 4],
    }
}

#[test]
fn ground_and_air_use_different_velocity_projections() {
    let mut state = Engagement::default();
    let air = state.update(input());
    let ground = state.update(Input {
        category: 100,
        ..input()
    });
    assert!((air.entry_velocity[1] + 2.).abs() < 0.00001);
    assert!((air.entry_velocity[2] - 1.8).abs() < 0.00001);
    assert!((ground.entry_velocity[1] + 1.6).abs() < 0.00001);
    assert!((ground.entry_velocity[2] - 2.4).abs() < 0.00001);
}

#[test]
fn steep_ground_entry_rejects_without_air_wipeout() {
    let steep = Input {
        up: [1., 0., 0., 0.],
        ..input()
    };
    let mut state = Engagement::default();
    let air = state.update(steep);
    assert!(!air.valid);
    assert_eq!(air.wipeout_reasons, [13]);
    let ground = state.update(Input {
        category: 100,
        ..steep
    });
    assert!(!ground.valid);
    assert!(ground.wipeout_reasons.is_empty());
    assert_eq!(ground.flags & 0x4000_0000, 0);
}

#[test]
fn both_impact_requests_survive_first_rejection_including_on_ground() {
    let mut state = Engagement::default();
    let result = state.update(Input {
        category: 100,
        max_delta: 0.,
        board_velocity: [4., 0., 8., 0.],
        ..input()
    });
    assert!(!result.valid);
    assert_eq!(result.wipeout_reasons, [9, 14]);
}

#[test]
fn tipslide_latch_counts_down_during_invalid_investigations() {
    let mut state = Engagement::default();
    let result = state.update(Input {
        kind: 2,
        previous_state_2504: 403,
        ..input()
    });
    assert_ne!(result.flags & 0x0200_0000, 0);
    assert_eq!(state.tipslide_frames, 8);
    for remaining in (0..8).rev() {
        let result = state.update(Input {
            valid: false,
            ..input()
        });
        assert_eq!(state.tipslide_frames, remaining);
        assert_eq!(result.entry_velocity, [9.; 4]);
        assert_eq!(result.impact_speed, 0.);
    }
    state.update(Input {
        valid: false,
        ..input()
    });
    assert_eq!(state.tipslide_frames, 0);
}

#[test]
fn coping_and_entry_slope_dead_zones_are_distinct() {
    let velocity = [1., 0.005, 0., 0.];
    let tangent = [1., 0., 0., 0.];
    assert_eq!(
        super::super::admission::approach_slope_sine(tangent, velocity),
        0.
    );
    assert!((engagement_slope_sine(tangent, velocity) - 1.).abs() < 0.00001);
}
