use super::*;
const FRAME: Frame = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0., 0., 0., 1.],
];
fn motion() -> CompletedMotion {
    CompletedMotion {
        contact_position_192: [0., 2., 0., 1.],
        contact_flags_368: 1,
        physical_frame_848: FRAME,
        animation_frame_912: FRAME,
        vector_976: [1.; 4],
        vector_992: [2.; 4],
        vector_1008: [3.; 4],
        vector_1040: [0.; 4],
        vector_1056: [4.; 4],
        launch_1076: false,
        flag_1077: true,
    }
}
fn processed() -> Processed {
    Processed {
        frame_192: FRAME,
        position_592: [0.; 4],
        flags_2476: 0,
        flags_2480: 0,
        flags_2484: 0,
        flags_2488: 0,
        elapsed_2664: 0.,
        value_2852: 0.,
        query_context: ground_query::QueryContext {
            selection_flags_2948: 0,
            matching_id_2952: 0,
        },
    }
}
#[test]
fn explicit_jump_suppression_does_not_suppress_controller_launch() {
    let mut state = State::default();
    let mut m = motion();
    let mut p = processed();
    p.flags_2476 = 0x80000;
    assert!(wants_air(&state, &m, &p));
    for mask in [0x20000, 0x80, 0x100] {
        p.flags_2480 = mask;
        assert!(!wants_air(&state, &m, &p));
        m.launch_1076 = true;
        assert!(wants_air(&state, &m, &p));
        m.launch_1076 = false;
    }
    p.flags_2480 = 0;
    state.flags_144_to_150[6] = true;
    assert!(!wants_air(&state, &m, &p));
    m.contact_flags_368 = 0;
    p.elapsed_2664 = 0.05;
    assert!(!wants_air(&state, &m, &p));
    p.elapsed_2664 = f32::from_bits(0.05f32.to_bits() + 1);
    assert!(wants_air(&state, &m, &p));
}
#[test]
fn board_bounds_flattens_y_but_retains_scaled_w() {
    let mut frame = FRAME;
    frame[2] = [0., 9., 2., 6.];
    let b = board_bounds(frame, [99., 3., 4., 55.], [1., 2., 5., 8.]);
    assert!((b.frame[2][3] - 3.).abs() < 1e-5);
    assert!((b.frame[3][3] - 28.).abs() < 1e-4);
    assert_eq!(b.frame[3][1], 3.);
    assert_eq!(b.extents, [1., 2., 5., 8.]);
    frame[2] = [0., 7., 0., 8.];
    assert_eq!(
        board_bounds(frame, [0.; 4], [0.; 4]).frame[2],
        [0., 0., 1., 0.]
    );
}
struct Recorder {
    p: Processed,
    events: Vec<&'static str>,
    point: Vector,
    bone_frame: Frame,
    toolkit: Option<ToolkitInput>,
    found: bool,
    accepted: bool,
    edge: bool,
}
impl Recorder {
    fn new() -> Self {
        Self {
            p: processed(),
            events: vec![],
            point: [0.; 4],
            bone_frame: FRAME,
            toolkit: None,
            found: false,
            accepted: false,
            edge: false,
        }
    }
}
impl ground_query::GroundQueryScene for Recorder {
    type Error = ();
    fn edge_candidates(
        &mut self,
        s: &ground_query::EdgeSearch,
    ) -> Result<Vec<ground_query::Edge>, ()> {
        self.events.push("edges");
        assert!(s.narrow_forward);
        Ok(if self.edge {
            vec![ground_query::Edge {
                start: crate::math::Vector3::new(-0.2, 0., 0.),
                end: crate::math::Vector3::new(0.2, 0., 0.),
            }]
        } else {
            vec![]
        })
    }
    fn query_lines(
        &mut self,
        _: &ground_query::GroundQueryPacket,
    ) -> Result<[Option<ground_query::LineHit>; 7], ()> {
        panic!("Sync must not consume its newly submitted queries")
    }
}
impl Services for Recorder {
    type Launch = ();
    type Candidate = [u32; 2];
    fn processed(&self) -> Processed {
        self.p
    }
    fn prepare_biped_launch(&mut self) {
        self.events.push("prepare");
        self.point = [2., 3., 4., 90.];
    }
    fn skeleton_point_10960(&self) -> Vector {
        self.point
    }
    fn launch_air(&mut self, _: (), point: Vector) {
        self.events.push("air");
        assert_eq!(point, [2., 3., 4., 1.]);
    }
    fn update_skeleton(&mut self, frame: Frame, _: Vector) {
        self.events.push("skeleton");
        self.bone_frame = frame;
        // Exercises the required fresh processed read after this callback.
        self.p.flags_2476 = 0x400000;
    }
    fn board_settings(&self) -> BoardSettings {
        BoardSettings {
            extent_0: [2.; 4],
            extent_16: [3.; 4],
            offset_32: [0.; 4],
            angle_436: 10.,
            angle_440: 20.,
            margin_444: 0.2,
            angle_452: 30.,
            angle_456: 40.,
        }
    }
    fn board_flags_12836(&self) -> u8 {
        0x40
    }
    fn probe_board(&mut self, _: Vector) -> Option<[u32; 2]> {
        self.events.push("probe");
        self.found.then_some([7, 11])
    }
    fn candidate_key_188(&self, c: &[u32; 2]) -> [u32; 2] {
        *c
    }
    fn bone_23_position(&mut self) -> Vector {
        self.events.push("bone23");
        [0.; 4]
    }
    fn classify_board(&mut self, _: &[u32; 2], _: Vector, b: Bounds, l: BoardLimits) -> bool {
        self.events.push("classify");
        assert_eq!(b.extents, [1.9; 4]);
        assert!((l.angle_a - 30. * f32::from_bits(0x3c8efa35)).abs() < 1e-6);
        self.accepted
    }
    fn query_board(&mut self, _: Vector, b: Bounds, l: BoardLimits) {
        self.events.push("board_query");
        assert_eq!(b.extents, [3.; 4]);
        assert!((l.angle_a - 40. * f32::from_bits(0x3c8efa35)).abs() < 1e-6);
    }
    fn bind_board(&mut self, key: [u32; 2]) {
        self.events.push("bind");
        assert_eq!(key, [7, 11]);
    }
    fn commit_bound_board(&mut self) {
        self.events.push("bound");
    }
    fn commit_free_board(&mut self, _: Frame) {
        self.events.push("free");
    }
    fn reset_board(&mut self) {
        self.events.push("reset");
    }
    fn update_toolkit(&mut self, p: ToolkitInput) {
        self.events.push("toolkit");
        self.toolkit = Some(p);
    }
    fn deck_half_wheelbase(&self) -> f32 {
        0.3
    }
    fn submit_ground_query(&mut self, _: ground_query::GroundQueryPacket) {
        self.events.push("submit");
    }
}
#[test]
fn launch_then_skeleton_then_board_then_toolkit_then_edges() {
    let mut s = State::default();
    let mut m = motion();
    m.launch_1076 = true;
    let mut r = Recorder::new();
    r.found = true;
    r.accepted = true;
    assert_eq!(sync(&mut s, &m, &mut r), Ok(false));
    assert_eq!(
        r.events,
        [
            "prepare",
            "air",
            "skeleton",
            "probe",
            "bone23",
            "classify",
            "board_query",
            "bind",
            "bound",
            "toolkit",
            "edges"
        ]
    );
    assert_eq!(s.frame_80[3], [0., 2., 0., 1.]);
    assert_eq!([s.counter_152, s.counter_156], [7, 11]);
    assert!(s.flags_144_to_150[0] && s.flags_144_to_150[2] && s.flags_144_to_150[3]);
    assert_eq!(
        r.toolkit.unwrap().0,
        [
            s.frame_80[3],
            [3.; 4],
            [2.; 4],
            [1.; 4],
            [0.; 4],
            FRAME[1],
            FRAME[0]
        ]
    );
}
#[test]
fn failed_probe_still_updates_board_and_uses_free_action() {
    let mut s = State::default();
    let mut r = Recorder::new();
    sync(&mut s, &motion(), &mut r).unwrap();
    assert_eq!(
        r.events,
        [
            "skeleton",
            "probe",
            "board_query",
            "free",
            "toolkit",
            "edges"
        ]
    );
}
#[test]
fn last_spin_tick_still_rotates_and_landing_gate_clears_without_query() {
    let mut s = State::default();
    s.flags_144_to_150[6] = true;
    s.flags_144_to_150[4] = true;
    s.duration_180 = 0.001;
    s.angle_172 = 1.;
    s.angular_velocity_176 = 99.;
    let mut r = Recorder::new();
    r.p.elapsed_2664 = 0.6;
    sync(&mut s, &motion(), &mut r).unwrap();
    assert!(!s.flags_144_to_150[6]);
    assert!(!s.flags_144_to_150[4]);
    assert_eq!(s.duration_180, 0.);
    assert_eq!(s.angle_172, 0.9);
    assert!(r.bone_frame[2][0] > 0.7);
    assert_eq!(r.events, ["skeleton", "toolkit", "edges"]);
}
#[test]
fn selected_edge_submits_next_frame_packet_without_consuming_it() {
    let mut s = State::default();
    let mut r = Recorder::new();
    r.edge = true;
    let mut m = motion();
    m.contact_position_192 = FRAME[3];
    assert_eq!(sync(&mut s, &m, &mut r), Ok(true));
    assert_eq!(
        &r.events[r.events.len() - 3..],
        ["toolkit", "edges", "submit"]
    );
}
