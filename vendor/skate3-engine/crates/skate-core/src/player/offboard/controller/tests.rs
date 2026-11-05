use super::state::{IDENTITY, UP, ZERO};
use super::*;
fn metrics() -> [Option<ClipMetric>; 3] {
    [
        Some(ClipMetric {
            translation_z: -2.0,
            end_time: 1.0,
        }),
        Some(ClipMetric {
            translation_z: -4.0,
            end_time: 1.0,
        }),
        Some(ClipMetric {
            translation_z: -8.0,
            end_time: 1.0,
        }),
    ]
}
fn graph<const N: usize>(value: f32) -> PointGraph<N> {
    PointGraph {
        x: std::array::from_fn(|i| i as f32),
        y: [value; N],
    }
}
fn settings() -> Settings {
    Settings {
        movement_intent: movement_intent::Settings {
            sprint_speed: graph(2.0),
            normal_speed: graph(1.0),
            sprint_blend: graph(0.0),
            sprint_time_cap: 1.0,
            slide_steering: graph(0.0),
        },
        movement_velocity: movement_velocity::Settings {
            slope_speed_scalar: graph(1.0),
            slope_mode_speed: graph(1.0),
            turn_vs_speed: graph(0.0),
            turn_delta_vs_speed: graph(1.0),
        },
        slide_vs_slope: graph(0.0),
        slide_vs_speed: graph(0.0),
    }
}
fn job() -> GroundJob {
    GroundJob {
        contact_position: ZERO,
        contact_normal: UP,
        support_frame: IDENTITY,
        target_position: ZERO,
        target_normal: UP,
        edge_position: ZERO,
        edge_normal: UP,
        flags: 1,
        support_id: 0,
        collision_displacements: [ZERO; 2],
        animation_motion: [0.0, 0.0, 1.0, 0.0],
        animation_velocity: ZERO,
        desired_direction: [0.0, 0.0, 1.0, 0.0],
        animation_position: ZERO,
        requested_duration: 1.0,
        mirrored: false,
        requested_phase: -1.0,
        override_duration: 1.0,
        animation_directed: false,
        movement: 1.0,
        steering: 0.0,
        sprint_pressed: false,
        suppress_lean: false,
        suppress_minimum: false,
        target_frame_present: false,
        edge_active: false,
        target_frame: IDENTITY,
        ignore_obstacle: false,
    }
}
#[test]
fn constructor_uses_clip_metrics_and_original_initial_fields() {
    let s = State::new(metrics());
    assert_eq!(
        s.thresholds.0,
        [0.01, 3.0, 6.0, f32::from_bits(0x5015_02f9)]
    );
    assert_eq!(s.motion.frame_0, IDENTITY);
    assert_eq!(s.surface.surface_normal, UP);
    assert_eq!(s.velocity_override_remaining_772, -1.0);
    assert_eq!(s.motion.target_scale_784, -1.0);
    assert_eq!(s.cadence.phase.duration, None);
}

#[test]
fn tiny_velocity_delta_keeps_ground_publication_finite() {
    let mut controller = Controller::new(settings(), metrics());
    controller.state.motion.velocity_480 = [0.0, 1.0e-21, 1.0, 0.0];
    controller.state.motion.speed_704 = 1.0;
    let result = controller.step_ground(&job());
    assert!(result.position.into_iter().all(f32::is_finite));
    assert!(result.velocity.into_iter().all(f32::is_finite));
    assert!(
        result
            .physical_frame
            .into_iter()
            .flatten()
            .all(f32::is_finite)
    );
    assert!(
        result
            .animation_frame
            .into_iter()
            .flatten()
            .all(f32::is_finite)
    );
    assert!(
        controller
            .state
            .surface
            .lean
            .into_iter()
            .all(f32::is_finite)
    );
    assert!(controller.state.cadence.phase.phase.is_finite());
}
#[test]
fn absent_clip_queries_have_original_zero_speed() {
    let s = State::new([None; 3]);
    assert_eq!(s.thresholds.0[1], 0.0);
    assert_eq!(s.thresholds.0[2], 0.0);
}
#[test]
fn reset_preserves_phase_and_correction_vectors_only() {
    let mut s = State::new(metrics());
    s.cadence.phase.phase = 0.75;
    s.cadence.phase.duration = Some(2.0);
    s.motion.correction_576 = [1.0; 4];
    s.correction_target_592 = [2.0; 4];
    s.motion.correction_enabled_711 = true;
    s.motion.velocity_480 = [3.0; 4];
    s.intent.speed = 9.0;
    s.reset();
    assert_eq!(s.cadence.phase.phase, 0.75);
    assert_eq!(s.cadence.phase.duration, Some(2.0));
    assert_eq!(s.motion.correction_576, [1.0; 4]);
    assert_eq!(s.correction_target_592, [2.0; 4]);
    assert!(!s.motion.correction_enabled_711);
    assert_eq!(s.motion.velocity_480, ZERO);
    assert_eq!(s.intent.speed, 0.0);
}
fn placement(current: u32, previous: u32) -> PlacementInput {
    PlacementInput {
        frame: IDENTITY,
        velocity: [3.0, 0.0, 4.0, 0.0],
        body_position: [1.0, 2.0, 3.0, 0.0],
        current_state: current,
        previous_state: previous,
        previous_frame: IDENTITY,
    }
}
#[test]
fn landing_preserves_eligible_correction_from_air_target() {
    let mut s = State::new(metrics());
    s.correction_target_592 = [0.0, 0.0, 2.0, 0.0];
    s.place(placement(500, 501));
    assert!(s.motion.correction_enabled_711);
    assert_eq!(s.motion.correction_576, [0.0, 0.0, -2.0, 0.0]);
    assert_eq!(s.position_368, [1.0, 2.0, 3.0, 0.0]);
    assert_eq!(s.motion.speed_704, 5.0);
}
#[test]
fn air_placement_clears_correction_without_resetting_movement_timers() {
    let mut s = State::new(metrics());
    s.intent.sprint_time = 4.0;
    s.correction_target_592 = [1.0; 4];
    s.motion.correction_enabled_711 = true;
    s.place(placement(501, 500));
    assert_eq!(s.intent.sprint_time, 4.0);
    assert_eq!(s.correction_target_592, ZERO);
    assert!(!s.motion.correction_enabled_711);
}
#[test]
fn other_placement_keeps_existing_correction() {
    let mut s = State::new(metrics());
    s.motion.correction_576 = [1.0; 4];
    s.motion.correction_enabled_711 = true;
    s.place(placement(600, 500));
    assert_eq!(s.motion.correction_576, [1.0; 4]);
    assert!(s.motion.correction_enabled_711);
}
#[test]
fn full_normal_dispatch_moves_canonical_velocity_and_publishes() {
    let mut c = Controller::new(settings(), metrics());
    let result = c.step_ground(&job());
    assert!(!result.alternate);
    assert!(c.state.motion.velocity_480[2] > 0.0);
    assert!(result.physical_frame[3][2] > 0.0);
    assert_eq!(result.velocity, c.state.frame_output.velocity);
    assert_eq!(result.position, c.state.position_368);
}
#[test]
fn alternate_dispatch_skips_cadence_and_position_but_exports() {
    let mut c = Controller::new(settings(), metrics());
    let mut j = job();
    j.flags = 2;
    j.target_position = [0.0, 0.0, 1.0, 0.0];
    c.state.motion.velocity_480 = [0.0, 5.0, 0.0, 0.0];
    c.state.frame_output.velocity = [0.0, 5.0, 0.0, 0.0];
    c.state.position_368 = [9.0; 4];
    c.state.cadence.phase.phase = 0.75;
    let r = c.step_ground(&j);
    assert!(r.alternate);
    assert_eq!(c.state.position_368, [9.0; 4]);
    assert_eq!(c.state.cadence.phase.phase, 0.75);
    assert!(c.state.motion.velocity_480[1] < 5.0);
}
#[test]
fn exporter_degenerate_frame_is_identity() {
    assert_eq!(build_frame(UP, UP), IDENTITY);
    let f = build_frame([0.0, 2.0, 0.0, 0.0], [0.0, 0.0, 3.0, 0.0]);
    for axis in 0..3 {
        for lane in 0..3 {
            assert!((f[axis][lane] - IDENTITY[axis][lane]).abs() < 1e-6);
        }
    }
    assert_eq!(f[3], ZERO);
}
