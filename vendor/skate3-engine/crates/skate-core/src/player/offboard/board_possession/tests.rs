use super::{lifecycle::*, *};
use crate::point_graph::PointGraph;
#[derive(Default)]
struct Recorder {
    calls: Vec<&'static str>,
    velocities: Vec<Vector>,
}
impl Effects for Recorder {
    fn enable_animation_soft(&mut self) {
        self.calls.push("soft");
    }
    fn enable_animation_angular_only(&mut self) {
        self.calls.push("angular");
    }
    fn disable_animation(&mut self) {
        self.calls.push("off");
    }
    fn disable_linear_drive(&mut self) {
        self.calls.push("linear_off");
    }
    fn standard_board(&mut self) {
        self.calls.push("standard");
    }
    fn released_board(&mut self) {
        self.calls.push("released");
    }
    fn collision_volumes(&mut self, e: bool) {
        self.calls
            .push(if e { "collision_on" } else { "collision_off" });
    }
    fn clear_alignment(&mut self) {
        self.calls.push("unalign");
    }
    fn alignment(&mut self, _: Alignment) {
        self.calls.push("align");
    }
    fn velocity(&mut self, v: Vector) {
        self.calls.push("velocity");
        self.velocities.push(v);
    }
    fn position(&mut self, _: Vector) {
        self.calls.push("position");
    }
    fn hook_frame(&mut self, _: Frame) {
        self.calls.push("hook");
    }
    fn target_position_velocity(&mut self, _: Vector) {
        self.calls.push("target_velocity");
    }
    fn torque(&mut self, _: Vector) {
        self.calls.push("torque");
    }
}
fn settings() -> Settings {
    let constant = |v| PointGraph {
        x: [0., 1., 2., 3., 4., 5., 6., 7.],
        y: [v; 8],
    };
    Settings {
        hide_distance: 30.,
        hide_offset: 1000.,
        return_distance: 30.,
        mounted_return_distance: 5.,
        mounting_time: 0.1,
        retrieval_time: constant(1.),
        retrieval_weight: constant(0.5),
        throw_pitch: 18.,
        throw_velocity: constant(1.),
        throw_target_pitch: 20.,
        throw_pitch_scalar: 0.1,
        throw_roll_scalar: 0.1,
        throw_yaw_scalar: 0.2,
    }
}
fn observation() -> Observation {
    Observation {
        processed: Processed {
            board_frame_64: IDENTITY,
            player_frame_192: IDENTITY,
            position_592: [0.; 4],
            velocity_912: [0.; 4],
            direction_400: [0., 0., 1., 0.],
            hide_direction_464: [0., 0., 1., 0.],
            flags_2476: 0,
            flags_2480: 0,
            flags_2488: 0,
        },
        board_collision_flags_872: 0,
        board_state_840: 0,
        hand_contacts: [false; 2],
        physical_hand_positions: [[0.; 4]; 2],
        animation_board_frame_12624: IDENTITY,
        animation_hand_frames: [IDENTITY; 2],
        attachment_frame_0: IDENTITY,
    }
}
fn fields(state: u32) -> SkateboardControllerFields {
    SkateboardControllerFields {
        state_448: state,
        word_444: 0,
        system_on_452: true,
    }
}
#[test]
fn drop_runs_same_tick_first_throw_and_exactly_eight_batches() {
    let (mut s, mut f, mut o, mut e) = (
        State::default(),
        fields(1),
        observation(),
        Recorder::default(),
    );
    o.processed.flags_2476 = 0x3000;
    s.update(&mut f, &o, &settings(), &mut e);
    assert_eq!((f.state_448, f.word_444), (2, 7));
    assert_eq!(
        &e.calls[..5],
        &["off", "released", "collision_on", "unalign", "velocity"]
    );
    for _ in 0..8 {
        s.update(&mut f, &o, &settings(), &mut e);
    }
    assert_eq!(e.calls.iter().filter(|x| **x == "torque").count(), 24);
    assert_eq!(f.word_444, 0);
}
#[test]
fn recall_wins_over_distance_hide_and_mount_clamps_duration_down() {
    let (mut s, mut f, mut o, mut e) = (
        State::default(),
        fields(2),
        observation(),
        Recorder::default(),
    );
    o.processed.board_frame_64[3] = [100., 0., 0., 7.];
    o.processed.flags_2480 = 0x80000;
    s.update_state(&mut f, &o, &settings(), &mut e);
    assert_eq!(f.state_448, 4);
    assert_eq!(s.retrieval.duration_196, 0.1);
    assert!((s.retrieval.initial_0[3][0] - 5.).abs() < 1e-5);
    assert_ne!(s.retrieval.initial_0[3][3], 0.);
    assert_eq!(
        e.calls,
        vec!["collision_off", "unalign", "angular", "position"]
    );
}
#[test]
fn returning_contact_precedes_completion_but_mount_ignores_contact() {
    let (mut s, mut f, mut o, mut e) = (
        State::default(),
        fields(4),
        observation(),
        Recorder::default(),
    );
    s.retrieval.progress_200 = 1.;
    o.hand_contacts[1] = true;
    s.update_state(&mut f, &o, &settings(), &mut e);
    assert_eq!(f.state_448, 2);
    f.state_448 = 4;
    o.processed.flags_2480 = 0x80000;
    e.calls.clear();
    s.update_state(&mut f, &o, &settings(), &mut e);
    assert_eq!(f.state_448, 1);
    assert_eq!(
        e.calls,
        vec!["velocity", "soft", "standard", "collision_on"]
    );
}
#[test]
fn unordered_progress_completes_and_only_selected_hand_is_armed() {
    let (mut s, mut f, o, mut e) = (
        State::default(),
        fields(4),
        observation(),
        Recorder::default(),
    );
    s.retrieval.progress_200 = f32::NAN;
    s.update_state(&mut f, &o, &settings(), &mut e);
    assert_eq!(s.selected_hand_424, 1);
    assert_eq!(s.hands[0].dynamics, [[0, 0, 0, 2]; 2]);
    assert_eq!(s.hands[1].dynamics, [[0x4395ffff, 0, 0x468c9fff, 2]; 2]);
}
#[test]
fn stop_preserves_throw_word_from_release_and_off_controller_is_inert() {
    let (mut s, mut f, mut o, mut e) = (
        State::default(),
        fields(1),
        observation(),
        Recorder::default(),
    );
    o.processed.flags_2476 = 0x1000;
    s.stop(&mut f, &o, &settings(), &mut e);
    assert_eq!(f.word_444, 8);
    assert_eq!(f.state_448, 0);
    assert!(!f.system_on_452);
    e.calls.clear();
    s.update(&mut f, &o, &settings(), &mut e);
    s.stop(&mut f, &o, &settings(), &mut e);
    assert!(e.calls.is_empty());
    assert_eq!(f.word_444, 8);
}
#[test]
fn angular_request_rejects_orthogonal_omega_and_retains_singular_behavior() {
    let tensor = [[2., 0., 0., 0.], [0., 3., 0., 0.], [0., 0., 5., 0.]];
    let a = angular::acceleration_delta([0.1, 0., 0., 0.], [2., 99., 0., 0.], tensor);
    assert!((a[0] - 240.).abs() < 0.01);
    assert_eq!(a[1], 0.);
    let singular = angular::acceleration_delta([0.1, 0., 0., 0.], [0.; 4], [[0.; 4]; 3]);
    assert!(singular[..3].iter().any(|v| v.is_nan()));
}
#[test]
fn fill_uses_hidden_initial_position_and_zero_axis_quadrant() {
    let mut s = State::default();
    let o = observation();
    s.retrieval.initial_0[3] = [1., 0., 0., 0.];
    let result = fill(&fields(3), &s, &o.processed, IDENTITY);
    assert_eq!(result.angle_36.to_bits(), 0x3fc90fdb);
    assert!(result.free_312 && result.hiding_321);
    assert_eq!(math::quadrant_angle(-0., 0.).to_bits(), 0xbfc90fdb);
}
