use super::*;
use crate::{
    attribute_runtime::tests::{collection, definition, graph, node, set_dependency_bytes},
    reckoning_numeric::{HostReckoningNumeric, host_dot3},
};

/// Facing +Z: the look-at frame is the identity.
fn phys_out(physics_state: u32) -> ActionInputPhysOut {
    facing(physics_state, [0.0, 0.0, 1.0, 0.0])
}
fn facing(physics_state: u32, anim_z: [f32; 4]) -> ActionInputPhysOut {
    ActionInputPhysOut {
        physics_state,
        effective_anim_z: SkeletonRootVector::known(anim_z),
    }
}
const OFF_BOARD: u32 = PHYSICS_STATE_OFFBOARD.cast_unsigned();
/// On board and unlatched, the off-board stick only writes its four values.
fn continuous<N: ReckoningNumeric>(
    input: &DerivedControllerInput,
    attributes: Option<&ActionInputFillAttributes>,
    cvars: &mut Cvars,
    numeric: &mut N,
    sink: &mut Sink,
) -> Result<(), ActionInputContinuousError<N::Error, &'static str, &'static str>> {
    try_continuous(
        input,
        &mut OffBoardStickLatch::default(),
        phys_out(0),
        attributes,
        cvars,
        numeric,
        sink,
    )
}

fn attributes() -> ActionInputFillAttributes {
    // Actual null collection is the native missing-pointer DefaultDataArea path.
    ActionInputFillAttributes::new(&Arc::new(graph(
        super::super::derived::INPUT_LISTENER_CLASS,
        vec![],
    )))
}
#[derive(Default)]
struct Cvars {
    calls: Vec<String>,
    fail: Option<usize>,
    triggers: bool,
    sprint_button: bool,
}
impl Cvars {
    fn call(&mut self, name: String) -> Result<(), &'static str> {
        if self.fail == Some(self.calls.len()) {
            return Err("cvar");
        }
        self.calls.push(name);
        Ok(())
    }
}
impl ActionInputFillCvars for Cvars {
    type Error = &'static str;
    fn capture_float(&mut self, key: ActionInputFloatCvar) -> Result<(), Self::Error> {
        self.call(format!("capture {key:?}"))
    }
    fn capture_boolean(&mut self, key: ActionInputBoolCvar) -> Result<(), Self::Error> {
        self.call(format!("capture {key:?}"))
    }
    fn float(&mut self, key: ActionInputFloatCvar) -> Result<f32, Self::Error> {
        self.call(format!("read {key:?}"))?;
        Ok(0.2)
    }
    fn boolean(&mut self, key: ActionInputBoolCvar) -> Result<bool, Self::Error> {
        self.call(format!("read {key:?}"))?;
        Ok(match key {
            ActionInputBoolCvar::UseTriggersForExplicitWipeout => self.triggers,
            ActionInputBoolCvar::UseReleaseForWipeout => false,
            ActionInputBoolCvar::UseSprintButtonInput => self.sprint_button,
        })
    }
}
#[derive(Default)]
struct Sink {
    writes: Vec<(String, u32)>,
    fail: Option<usize>,
}
impl ActionInputIntentSink for Sink {
    type Error = &'static str;
    fn insert(&mut self, name: &str, value: f32) -> Result<(), Self::Error> {
        if self.fail == Some(self.writes.len()) {
            return Err("insert");
        }
        self.writes.push((name.to_owned(), value.to_bits()));
        Ok(())
    }
}
impl Sink {
    fn names(&self) -> Vec<&str> {
        self.writes.iter().map(|(n, _)| n.as_str()).collect()
    }
    fn value(&self, key: &str) -> Option<f32> {
        self.writes
            .iter()
            .find(|(n, _)| n == key)
            .map(|(_, v)| f32::from_bits(*v))
    }
}
#[derive(Default)]
struct Numeric {
    dot: Vec<u32>,
    sqrt: Vec<u32>,
    reciprocal: Vec<u32>,
}
impl ReckoningNumeric for Numeric {
    type Error = std::convert::Infallible;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        let value = host_dot3(a, b);
        self.dot.push(value.to_bits());
        Ok(value)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.sqrt.push(value.to_bits());
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.reciprocal.push(value.to_bits());
        Ok(1.0 / value)
    }
    fn truncate_i32(&mut self, _value: f32) -> Result<i32, Self::Error> {
        panic!("Fill wraps with floating floor")
    }
}

#[test]
fn neutral_has_seven_unconditional_controls_after_no_input_the_frame_and_one_late_length() {
    let mut sink = Sink::default();
    let mut cvars = Cvars::default();
    let mut numeric = Numeric::default();
    continuous(
        &DerivedControllerInput::new(),
        Some(&attributes()),
        &mut cvars,
        &mut numeric,
        &mut sink,
    )
    .unwrap();
    assert_eq!(
        sink.names(),
        [
            "NoInput",
            "OB_Steer",
            "OBL_Steer",
            "OB_Mag",
            "OBL_Mag",
            "OB_ObjectMvZ",
            "OB_ObjectMvX",
            "OB_ObjectMvRot"
        ]
    );
    // Matrix33FromLookAt's two normalizations, the stick, then the late length.
    let one = 1.0_f32.to_bits();
    assert_eq!(numeric.dot, [one, one, 0]);
    assert_eq!(numeric.sqrt, [one, one, 0]);
    assert!(numeric.reciprocal.is_empty());
    assert_eq!(sink.writes[0].1, one);
    // The unlatched squaring turns the stick's -0 side into +0.
    assert!(sink.writes[1..].iter().all(|(_, bits)| *bits == 0));
    assert_eq!(
        cvars.calls,
        [
            "capture NewPushAllowance",
            "capture UseTriggersForExplicitWipeout",
            "read UseTriggersForExplicitWipeout",
            "capture TimeToConsiderWipeout",
            "read TimeToConsiderWipeout",
            "capture UseReleaseForWipeout",
            "read UseReleaseForWipeout",
            "capture UseSprintButtonInput",
            "read UseSprintButtonInput"
        ]
    );
}

#[test]
fn both_pushes_preserve_duplicate_insertion_and_button_source_order() {
    let mut input = DerivedControllerInput::new();
    input.current.face_a = true;
    input.current.face_x = true;
    let mut sink = Sink::default();
    let mut cvars = Cvars::default();
    continuous(
        &input,
        Some(&attributes()),
        &mut cvars,
        &mut HostReckoningNumeric,
        &mut sink,
    )
    .unwrap();
    assert_eq!(
        &sink.names()[..9],
        [
            "WipeOutRecover",
            "ExitGrind",
            "NewPush",
            "RightPush",
            "Pushing",
            "LeftPush",
            "Pushing",
            "HandPlantOneFootRight",
            "HandPlantOneFootLeft"
        ]
    );
    assert!(!cvars.calls.iter().any(|s| s == "read NewPushAllowance"));
    assert_eq!(sink.value("OB_Jump"), Some(1.0));
    assert_eq!(sink.value("NoInput"), None);
    let mut refused = Sink {
        fail: Some(5),
        ..Sink::default()
    };
    assert!(matches!(
        continuous(
            &input,
            Some(&attributes()),
            &mut Cvars::default(),
            &mut HostReckoningNumeric,
            &mut refused
        ),
        Err(ActionInputContinuousError::Insert("insert"))
    ));
    assert_eq!(refused.writes, sink.writes[..5]);
}

#[test]
fn sticky_push_allowance_rereads_same_captured_cvar_and_bumper_blocks_gestures() {
    let mut input = DerivedControllerInput::new();
    input.current.face_a = true;
    input.current.face_x = true;
    input.previous = input.current;
    input.current.left_bumper = true;
    input.current.dpad_up = true;
    input.face_held = [0.1, 0.0, 0.3, 0.0];
    let mut cvars = Cvars::default();
    let mut sink = Sink::default();
    continuous(
        &input,
        Some(&attributes()),
        &mut cvars,
        &mut HostReckoningNumeric,
        &mut sink,
    )
    .unwrap();
    assert_eq!(
        cvars
            .calls
            .iter()
            .filter(|s| *s == "capture NewPushAllowance")
            .count(),
        1
    );
    assert_eq!(
        cvars
            .calls
            .iter()
            .filter(|s| *s == "read NewPushAllowance")
            .count(),
        2
    );
    assert_eq!(sink.value("NewPush"), Some(1.0));
    assert_eq!(sink.value("GestureUp"), None);
}

#[test]
fn early_and_late_failures_write_no_intents_and_preserve_numeric_order() {
    let mut input = DerivedControllerInput::new();
    input.current.left_stick_x = 0.37;
    input.current.left_stick_y = 0.61;
    input.current.right_stick_x = 0.71;
    input.current.right_stick_y = 0.19;
    let mut numeric = Numeric::default();
    let mut sink = Sink::default();
    // 1.02 reads the global attributes only for the moving-object curves.
    assert!(matches!(
        continuous(&input, None, &mut Cvars::default(), &mut numeric, &mut sink),
        Err(ActionInputContinuousError::MissingGlobalAttributes)
    ));
    assert!(sink.writes.is_empty());
    let one = 1.0_f32.to_bits();
    assert_eq!(
        numeric.sqrt,
        [
            0.71_f32.mul_add(0.71, 0.19 * 0.19).to_bits(),
            0.61_f32.mul_add(0.61, 0.37 * 0.37).to_bits(),
            one,
            one,
            0.37_f32.mul_add(0.37, 0.61 * 0.61).to_bits()
        ]
    );
    assert_eq!(
        numeric.dot,
        [
            one,
            one,
            ((0.37_f32 * 0.37) + 0.0 * 0.0 + 0.61 * 0.61).to_bits()
        ]
    );
    assert_eq!(numeric.reciprocal.len(), 8);
    let class = super::super::derived::INPUT_LISTENER_CLASS;
    let mut vault = graph(class, vec![definition(OBJECT_ROTATE, 4, 144)]);
    collection(
        &mut vault,
        class,
        0xd7ed_bd36_2d7d_2152,
        0,
        None,
        vec![node(OBJECT_ROTATE, 0x40, 0)],
    );
    let mut numeric = Numeric::default();
    assert!(matches!(
        continuous(
            &input,
            Some(&ActionInputFillAttributes::new(&Arc::new(vault))),
            &mut Cvars::default(),
            &mut numeric,
            &mut sink
        ),
        Err(ActionInputContinuousError::Attribute(_))
    ));
    assert_eq!(numeric.sqrt.len(), 5);
    assert_eq!(numeric.reciprocal.len(), 12);
    assert!(sink.writes.is_empty());
}

#[test]
fn real_runtime_curve_layout_inheritance_and_full_key_select_payload() {
    let class = super::super::derived::INPUT_LISTENER_CLASS;
    let mut vault = graph(class, vec![definition(OBJECT_X, 4, 80)]);
    let mut bytes = vec![0; 256];
    for i in 0..8 {
        #[expect(clippy::cast_precision_loss, reason = "eight fixture points")]
        let x = i as f32;
        bytes[16 + 4 * i..20 + 4 * i].copy_from_slice(&x.to_bits().to_be_bytes());
        bytes[48 + 4 * i..52 + 4 * i].copy_from_slice(&(2.0 * x).to_bits().to_be_bytes());
    }
    set_dependency_bytes(&mut vault, 1, bytes);
    collection(
        &mut vault,
        class,
        99,
        0,
        Some(0),
        vec![node(OBJECT_X, 0x10, 0)],
    );
    collection(&mut vault, class, 0xd7ed_bd36_2d7d_2152, 99, None, vec![]);
    let attributes = ActionInputFillAttributes::new(&Arc::new(vault));
    assert_eq!(
        attributes.curve::<8>(OBJECT_X, 2.25).unwrap().to_bits(),
        4.5f32.to_bits()
    );
    assert_eq!(
        attributes
            .curve::<8>(OBJECT_X & 0xffff_ffff, 2.25)
            .unwrap()
            .to_bits(),
        0
    );
    assert_eq!(
        attributes.curve::<8>(OBJECT_X, f32::NAN).unwrap().to_bits(),
        14.0f32.to_bits()
    );
}

#[test]
fn wipeout_unordered_ages_refuse_and_exact_trigger_edges_are_preserved() {
    let mut input = DerivedControllerInput::new();
    input.stick_edge_age = [f32::NAN, 0.0, 0.0, 0.0];
    let mut sink = Sink::default();
    continuous(
        &input,
        Some(&attributes()),
        &mut Cvars::default(),
        &mut HostReckoningNumeric,
        &mut sink,
    )
    .unwrap();
    assert_eq!(sink.value("WipeOutRequest"), None);
    input.stick_edge_age = [0.0, 0.0, f32::NAN, 0.0];
    let mut sink = Sink::default();
    continuous(
        &input,
        Some(&attributes()),
        &mut Cvars::default(),
        &mut HostReckoningNumeric,
        &mut sink,
    )
    .unwrap();
    assert_eq!(sink.value("WipeOutRequest"), None);
    input.current.left_trigger = 1.0;
    input.current.right_trigger = 1.0;
    input.current.left_stick_in = true;
    input.current.right_stick_in = true;
    let mut sink = Sink::default();
    continuous(
        &input,
        Some(&attributes()),
        &mut Cvars {
            triggers: true,
            ..Cvars::default()
        },
        &mut HostReckoningNumeric,
        &mut sink,
    )
    .unwrap();
    assert_eq!(sink.value("WipeOutRequest"), Some(1.0));
    assert_eq!(sink.value("OB_RetrieveBoard"), Some(1.0));
}

struct Juice {
    active: bool,
    calls: Vec<&'static str>,
}
impl ActionInputJuice for Juice {
    type Error = std::convert::Infallible;
    fn is_trick_active(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("active");
        Ok(self.active)
    }
    fn take_trick_name(&mut self) -> Result<String, Self::Error> {
        self.calls.push("take");
        self.active = false;
        Ok("Kickflip".to_owned())
    }
}
#[test]
fn same_listener_full_selected_fill_consumes_late_juice_before_failed_insertion() {
    use crate::{
        game_input_player::recognizer::event_route::GameInputEventRoute,
        motion_graph_entity_authority::ActorGraphAuthority,
    };
    let authority = ActorGraphAuthority::new("same owner");
    let (player, receiver, _manager) = super::super::tests::gameplay();
    let listener = Arc::new(std::sync::Mutex::new(
        ActionGraphInputListener::new(&authority, &player).unwrap(),
    ));
    let mut route = GameInputEventRoute::new();
    let _registration = route.try_construct_listener(&listener).unwrap();
    let mut listener = listener.lock().unwrap();
    let pattern = super::super::tests::pattern("Ollie");
    listener
        .receive(&super::super::tests::matched(&receiver, &pattern, 2))
        .unwrap();
    let mut sink = Sink {
        fail: Some(11),
        ..Sink::default()
    };
    let mut juice = Juice {
        active: true,
        calls: vec![],
    };
    assert!(matches!(
        listener.try_fill_selected(
            0.25,
            phys_out(0),
            Some(&attributes()),
            &mut Cvars::default(),
            &mut HostReckoningNumeric,
            &mut sink,
            &mut juice
        ),
        Err(ActionInputFillError::Insert("insert"))
    ));
    assert_eq!(juice.calls, ["active", "take"]);
    assert!(!juice.active);
    assert_eq!(&sink.names()[8..], ["Trick", "Ollie", "GestureSpeed"]);
    assert_eq!(listener.events().len(), 1);
    assert_eq!(
        listener.derived().unwrap().magnitude_held.map(f32::to_bits),
        [0; 2]
    );
}

fn off_board_fill(
    input: &DerivedControllerInput,
    latch: &mut OffBoardStickLatch,
    phys_out: ActionInputPhysOut,
    cvars: &mut Cvars,
) -> Sink {
    let mut sink = Sink::default();
    try_continuous(
        input,
        latch,
        phys_out,
        Some(&attributes()),
        cvars,
        &mut HostReckoningNumeric,
        &mut sink,
    )
    .unwrap();
    sink
}

#[test]
fn a_turn_latches_the_world_stick_and_runs_until_it_is_faced() {
    let mut input = DerivedControllerInput::new();
    input.current.left_stick_x = 0.5;
    input.current.left_stick_y = -0.5;
    let mut latch = OffBoardStickLatch::default();
    // Behind the skater the side saturates; unlatched it is squared, sign kept.
    let first = off_board_fill(
        &input,
        &mut latch,
        phys_out(OFF_BOARD),
        &mut Cvars::default(),
    );
    assert_eq!(first.value("OB_Steer"), Some(-1.0));
    assert_eq!(first.value("OBL_Steer"), Some(-3.0625));
    assert_eq!(first.value("OB_Mag"), Some(0.0));
    assert_eq!(first.value("OB_MagHeld"), None);
    assert!(latch.latched);
    assert_eq!([latch.direction[0], latch.direction[2]], [0.5, -0.5]);
    // Latched: no squaring, and a full run while the turn lasts.
    let second = off_board_fill(
        &input,
        &mut latch,
        phys_out(OFF_BOARD),
        &mut Cvars::default(),
    );
    assert_eq!(second.value("OBL_Steer"), Some(-1.75));
    assert_eq!(second.value("OB_Mag"), Some(1.0));
    assert_eq!(second.value("OB_MagHeld"), Some(1.0));
    // Released, the latched world direction keeps turning and running.
    input.current.left_stick_x = 0.0;
    input.current.left_stick_y = 0.0;
    let released = off_board_fill(
        &input,
        &mut latch,
        phys_out(OFF_BOARD),
        &mut Cvars::default(),
    );
    assert_eq!(released.value("OB_Steer"), Some(-1.0));
    assert_eq!(released.value("OB_Mag"), Some(1.0));
    assert!(latch.latched);
    // Facing it ends the turn: this Fill still runs, the next one stops.
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let faced = facing(OFF_BOARD, [s, 0.0, -s, 0.0]);
    let sink = off_board_fill(&input, &mut latch, faced, &mut Cvars::default());
    assert!(sink.value("OB_Steer").unwrap().abs() < 0.2);
    assert_eq!(sink.value("OB_Mag"), Some(1.0));
    assert!(!latch.latched);
    let sink = off_board_fill(&input, &mut latch, faced, &mut Cvars::default());
    assert_eq!(sink.value("OB_Mag"), Some(0.0));
}

#[test]
fn b_held_off_board_walks_one_of_eight_ways_and_never_steers() {
    let directions = [
        (0.0, 1.0, "OB_WalkForward"),
        (0.7, 0.7, "OB_WalkForwardLeft"),
        (1.0, 0.0, "OB_WalkLeft"),
        (0.7, -0.7, "OB_WalkBackwardLeft"),
        (0.0, -1.0, "OB_WalkBackward"),
        (-0.7, -0.7, "OB_WalkBackwardRight"),
        (-1.0, 0.0, "OB_WalkRight"),
        (-0.7, 0.7, "OB_WalkForwardRight"),
    ];
    for (x, y, walk) in directions {
        let mut input = DerivedControllerInput::new();
        input.current.left_stick_x = x;
        input.current.left_stick_y = y;
        input.current.face_b = true;
        input.face_held[1] = 0.1;
        let mut latch = OffBoardStickLatch {
            direction: [1.0, 0.0, 0.0, 0.0],
            latched: true,
        };
        let sink = off_board_fill(
            &input,
            &mut latch,
            phys_out(OFF_BOARD),
            &mut Cvars::default(),
        );
        // PickOffBoardWalkDirection inserts before every other intent.
        assert_eq!(sink.writes[0], (walk.to_owned(), 1.0_f32.to_bits()));
        let walks = sink
            .names()
            .into_iter()
            .filter(|n| n.starts_with("OB_Walk"));
        assert_eq!(walks.count(), 1);
        assert_eq!(sink.value("OB_Steer"), Some(0.0));
        assert_eq!(sink.value("OBL_Mag"), Some(-1.0));
        assert_eq!(latch, OffBoardStickLatch::default());
    }
    let mut input = DerivedControllerInput::new();
    input.current.left_stick_x = 0.1;
    input.current.left_stick_y = 0.1;
    input.current.face_b = true;
    let no_walk = |sink: &Sink| !sink.names().iter().any(|n| n.starts_with("OB_Walk"));
    let mut latch = OffBoardStickLatch::default();
    let sink = off_board_fill(
        &input,
        &mut latch,
        phys_out(OFF_BOARD),
        &mut Cvars::default(),
    );
    assert!(no_walk(&sink));
    input.current.left_stick_y = 1.0;
    let running = phys_out(PHYSICS_STATE_OFFBOARD_LOCOMOTION.cast_unsigned());
    let sink = off_board_fill(&input, &mut latch, running, &mut Cvars::default());
    assert!(no_walk(&sink));
}

#[test]
fn walk_sectors_split_at_odd_eighths_and_unordered_falls_to_backward_left() {
    assert_eq!(walk_direction(0.0), "OB_WalkBackward");
    assert_eq!(
        walk_direction(f32::from_bits(0x3ec9_0fda)),
        "OB_WalkBackward"
    );
    assert_eq!(
        walk_direction(f32::from_bits(0x3ec9_0fdb)),
        "OB_WalkBackwardLeft"
    );
    assert_eq!(walk_direction(PI), "OB_WalkForward");
    assert_eq!(
        walk_direction(f32::from_bits(0x40bc_7edc)),
        "OB_WalkBackwardRight"
    );
    assert_eq!(
        walk_direction(f32::from_bits(0x40bc_7edd)),
        "OB_WalkBackward"
    );
    assert_eq!(walk_direction(f32::NAN), "OB_WalkBackwardLeft");
}

#[test]
fn running_needs_the_stick_past_0_7_unless_sprinting_or_already_running() {
    let stick_length =
        |y: f32| length(0.0_f32.mul_add(0.0, y * y), &mut HostReckoningNumeric).unwrap();
    let run = |sink: &Sink| sink.value("OBL_Mag").unwrap().to_bits();
    let mut input = DerivedControllerInput::new();
    input.current.left_stick_y = 0.85;
    let full = stick_length(0.85);
    let sink = off_board_fill(
        &input,
        &mut OffBoardStickLatch::default(),
        phys_out(OFF_BOARD),
        &mut Cvars::default(),
    );
    assert_eq!(
        run(&sink),
        (((0.85 - RUN_START) * full) * RUN_RANGE).to_bits()
    );
    assert_eq!(
        sink.value("OB_Mag").unwrap().to_bits(),
        (full * ((0.85 - WALK_START) * WALK_RANGE)).to_bits()
    );
    input.current.left_stick_y = 0.5;
    let half = stick_length(0.5);
    let sink = off_board_fill(
        &input,
        &mut OffBoardStickLatch::default(),
        phys_out(OFF_BOARD),
        &mut Cvars::default(),
    );
    assert_eq!(run(&sink), 0);
    assert!(sink.value("OB_Mag").unwrap() > 0.0);
    input.face_held[0] = 0.1;
    let mut sprinting = Cvars {
        sprint_button: true,
        ..Cvars::default()
    };
    let sink = off_board_fill(
        &input,
        &mut OffBoardStickLatch::default(),
        phys_out(OFF_BOARD),
        &mut sprinting,
    );
    assert_eq!(sink.value("OB_Sprint"), Some(1.0));
    assert_eq!(run(&sink), half.to_bits());
    input.face_held[0] = 0.0;
    let running = phys_out(PHYSICS_STATE_OFFBOARD_LOCOMOTION.cast_unsigned());
    let sink = off_board_fill(
        &input,
        &mut OffBoardStickLatch::default(),
        running,
        &mut Cvars::default(),
    );
    assert_eq!(run(&sink), half.to_bits());
    input.face_held[1] = 0.1;
    let sink = off_board_fill(
        &input,
        &mut OffBoardStickLatch::default(),
        running,
        &mut Cvars::default(),
    );
    assert_eq!(run(&sink), (-1.0_f32).to_bits());
}

#[test]
fn back_on_board_a_stale_latch_still_steers_once_but_no_longer_runs() {
    let mut latch = OffBoardStickLatch {
        direction: [0.5, 0.0, -0.5, 0.0],
        latched: true,
    };
    let sink = off_board_fill(
        &DerivedControllerInput::new(),
        &mut latch,
        phys_out(0),
        &mut Cvars::default(),
    );
    assert_eq!(sink.value("OB_Steer"), Some(-1.0));
    assert_eq!(sink.value("OB_Mag"), Some(0.0));
}
