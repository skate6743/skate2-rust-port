use super::*;
use skate3_core::point_graph::PointGraph;

fn state() -> GrindInputState {
    let constant = |value| PointGraph {
        x: [0., 1., 2., 3.],
        y: [value; 4],
    };
    GrindInputState {
        previous_state: 0,
        engagement_counter: 0,
        cooldown: 0,
        disabled: false,
        suppressed: false,
        elapsed: 0.,
        friction_vs_time: 0.,
        previous_velocity: [0; 4],
        grind_history: 0,
        secondary_history: 0,
        grounded_frames: 0,
        air_frames: 0,
        low_wheel_frames: 0,
        previous_proximity: false,
        previous_air_target: false,
        previous_direction: [0.; 4],
        gravity_timer: 0.,
        balance: balance::BalanceState::default(),
        engagement: entry::Engagement::default(),
        control: control::Control::default(),
        jumper: manager::Jumper::default(),
        investigation: GrindInvestigationFields::default(),
        settings: settings::Settings {
            friction: constant(0.8),
            slope_threshold: constant(1.),
            vertical_help: constant(0.5),
            gravity_vertical: constant(2.),
            gravity_linear: constant(0.5),
            exit_lean: PointGraph {
                x: [0., 1., 2., 3., 4., 5., 6., 7.],
                y: [0.; 8],
            },
            truck_to_wheel: 0.095,
            deck_to_truck: 0.243,
            test_above: 0.03,
            test_below: 0.2,
            max_impact: 8.,
        },
    }
}

fn context() -> PreContext {
    PreContext {
        board: [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
        air_counter: 20,
        tip_state: 0,
        air_targeting_grind_9653: false,
        balance_2720: 0.,
        translation_2796: 0.,
        stability_nudge_2800: 0.,
        up_down_2804: 0.,
        grab_min_height_2808: 0.,
    }
}

#[derive(Default)]
struct TestHost {
    events: Vec<&'static str>,
    fail: bool,
}
impl Host for TestHost {
    fn apply_material_mode(&mut self, mode: MaterialMode) -> Result<(), String> {
        self.events.push(match mode {
            MaterialMode::Grind => "grind",
            MaterialMode::Standard => "standard",
            MaterialMode::Unchanged => "unchanged",
        });
        if self.fail {
            Err("material failure".into())
        } else {
            Ok(())
        }
    }
    fn surface_probe(
        &mut self,
        _: &BoardWorld,
        _: [u32; 2],
        _: usize,
        _: Probe,
    ) -> Result<Option<ProbeHit>, String> {
        self.events.push("surface");
        if self.fail {
            Err("query failure".into())
        } else {
            Ok(None)
        }
    }
    fn force_exit_line(
        &mut self,
        _: &BoardWorld,
        _: [u32; 2],
        _: balance::ForceExitProbe,
    ) -> Result<Option<balance::ForceExitHit>, String> {
        self.events.push("exit");
        if self.fail {
            Err("query failure".into())
        } else {
            Ok(None)
        }
    }
}

#[test]
fn material_selection_uses_previous_air_target_and_preserves_skipped_history() {
    let mut s = state();
    let mut p = ProcessedPhysicsInput::default();
    p.state_2508 = 201;
    p.category_2512 = 200;
    assert_eq!(s.material_mode(&p, false, true), MaterialMode::Standard);
    p.state_2508 = 300;
    assert_eq!(s.material_mode(&p, true, false), MaterialMode::Unchanged);
    assert!(s.previous_air_target);
    p.state_2508 = 100;
    p.category_2512 = 100;
    assert_eq!(s.material_mode(&p, false, false), MaterialMode::Grind);
    assert_eq!(s.material_mode(&p, false, false), MaterialMode::Standard);
}

#[test]
fn permission_keeps_flicker_weights_strict_threshold_and_signed_decrement() {
    let mut s = state();
    let mut p = ProcessedPhysicsInput::default();
    p.state_2508 = 400;
    s.permission(&p, 20);
    assert_eq!(s.engagement_counter, 49);
    p.state_2508 = 403;
    s.permission(&p, 20);
    assert_eq!(s.engagement_counter, 68);
    s.engagement_counter = 152;
    s.permission(&p, 20);
    assert!(!s.suppressed);
    s.engagement_counter = 153;
    s.permission(&p, 20);
    assert!(s.suppressed && s.disabled);
    assert_eq!(s.cooldown, 90);
    assert_eq!(history::decrement(0), 0);
    assert_eq!(history::decrement(u32::MAX), 0);
    assert_eq!(history::decrement(0x8000_0000), 0x7fff_ffff);
}

#[test]
fn constructor_reset_does_not_reinitialize_child_history_or_jump_energy() {
    let mut s = state();
    s.jumper.energy = 0.6;
    s.balance.elapsed = 1.;
    s.secondary_history = 15;
    s.investigation.flags_1516 = u32::MAX;
    s.reset();
    assert_eq!(s.investigation, GrindInvestigationFields::default());
    assert_eq!(s.jumper.energy, 0.6);
    assert_eq!(s.balance.elapsed, 1.);
    assert_eq!(s.secondary_history, 15);
}

#[test]
fn empty_authored_world_completes_active_grind_without_error_stub() {
    let mut s = state();
    let mut p = ProcessedPhysicsInput::default();
    p.state_2508 = 403;
    p.category_2512 = 400;
    p.timestep_2604 = 0.02;
    p.grind_words_2532_2536 = [3, 0];
    let provider = StaticProvider::new(None).unwrap();
    let world = BoardWorld::new(vec![]);
    let mut host = TestHost::default();
    let pending = s
        .pre_update(&p, &provider, &world, context(), &mut host)
        .unwrap();
    let c = context();
    let post = PostContext {
        board: c.board,
        balance_2720: c.balance_2720,
        translation_2796: c.translation_2796,
        stability_nudge_2800: c.stability_nudge_2800,
        up_down_2804: c.up_down_2804,
        grab_min_height_2808: c.grab_min_height_2808,
    };
    let result = s
        .post_update(&mut p, &world, pending, post, &mut host)
        .unwrap();
    assert!(!p.grind.valid_1488);
    assert_eq!(p.grind.friction_1496, 0.8);
    assert_eq!(p.grind.normal_1152, [0; 4]);
    assert_eq!(host.events, ["grind"]);
    assert!(result.wipeout_reasons.is_empty());
    assert_eq!(s.jumper.family as u32, 3);
}

#[test]
fn producer_failure_is_not_a_query_miss() {
    let mut s = state();
    let p = ProcessedPhysicsInput::default();
    let provider = StaticProvider::new(None).unwrap();
    let world = BoardWorld::new(vec![]);
    let mut host = TestHost {
        fail: true,
        ..TestHost::default()
    };
    assert!(
        s.pre_update(&p, &provider, &world, context(), &mut host)
            .is_err()
    );
    assert_eq!(s.grounded_frames, 0);
}

#[test]
fn full_surface_publication_keeps_separate_normals_materials_and_offsets() {
    let surface = GrindSurface {
        center: [1.; 4],
        far_points: [[2.; 4], [3.; 4]],
        upmost_normal: [4.; 4],
        direction: [5.; 4],
        high_side: [6.; 4],
        normal_limits: [7., 8.],
        kind: grind_surface::GeometryType::Ledge,
        audio_surface: 9,
        physics_surface: 10,
        flags: 0x1000_0000,
        tilted_upmost_normal: [11.; 4],
    };
    let mut fields = GrindInvestigationFields::default();
    publication::surface(&mut fields, &surface);
    assert_eq!(fields.center_1360, raw([1.; 4]));
    assert_eq!(fields.far_points_1376_1392, [raw([2.; 4]), raw([3.; 4])]);
    assert_eq!(fields.upmost_normal_1408, raw([4.; 4]));
    assert_eq!(fields.audio_surface_1468, 9);
    assert_eq!(fields.physics_surface_1472, 10);
    assert_eq!(fields.normal_1152, [0; 4]);
}
