//! Stock bindings plus production Host lifecycle: a split latch breaks this test.
use super::*;
use crate::skate3_native::graph_host::{motion_conditions::MotionCondition, motion_sliding::Operation};
use skate3_core::graph::conditions::PhysicalStateInputs;

fn host() -> MotionHost {
    use crate::skate3_native::graph_runtime::CompiledGraph;
    use skate_data::state_graph::{StateGraph, binding::Binding};
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let source =
        StateGraph::load(&root.join("private/stock/data/state/MotionGraph_OnBoard.stategraph"))
            .unwrap();
    let binding = Binding::from_graph(&source).unwrap();
    let runtime = CompiledGraph::from_binding(&binding).unwrap();
    MotionHost::from_graph(
        &LoadedGraph {
            source,
            binding,
            runtime,
        },
        &Collections::load(&root).unwrap(),
        skate_data::animation_banks::AnimationBanks::load(&root)
            .unwrap()
            .metadata()
            .unwrap(),
        PlaybackContext {
            is_switch: Some(false),
            is_mirrored: Some(false),
            board_available: Some(false),
            pro_skater: encode(b""),
            transition_override: None,
        },
    )
    .unwrap()
}
fn behavior(host: &MotionHost, predicate: impl Fn(&MotionOperation) -> bool) -> usize {
    host.remap
        .behaviors
        .iter()
        .position(|&id| predicate(&host.operations[id]))
        .expect("Required authored stock behavior is missing")
}
fn slide(host: &MotionHost, operation: Operation) -> usize {
    behavior(
        host,
        |v| matches!(v, MotionOperation::Slide(other) if *other == operation),
    )
}
fn frame() -> Frame {
    Frame {
        dt: 0.1,
        current: None,
        last: None,
        state_times: Vec::new(),
    }
}
fn physical(host: &mut MotionHost) {
    host.animation.skater_animation_flags = Some(0);
    host.condition_inputs.physical_state = Some(PhysicalStateInputs {
        category: 1,
        grinding: false,
        grind_name: String::new(),
    });
    host.physical = Some(MotionPhysical {
        turning: set_turning::Physical {
            field_32: 0.0,
            field_36: 0.0,
            field_52: 0.0,
            field_56: 0.0,
            field_60: 0.0,
            body_168: 2.0,
        },
        stance: (false, false),
        forward_speed: 2.0,
        time_since_teleport: 1.0,
        is_switch: false,
        foot_frame: Some(PushFootFrame {
            left_foot: [0.0; 4],
            right_foot: [0.0; 4],
            deck_position: [0.0; 4],
            deck_y: [0.0, 1.0, 0.0, 0.0],
            deck_z: [0.0, 0.0, 1.0, 0.0],
            skateboard_flipped: false,
        }),
    });
    host.fakie_physical = Some(skate3_core::animation::riding_fakie::Physical {
        category: 1,
        grind_state: 0,
        doing_trick: false,
        board_axis: [0.0, 0.0, 1.0, 0.0],
        deck_velocity: [2.0, 0.0, 0.0, 0.0],
        external_velocity: [0.0; 4],
        ground_projected_speed: 2.0,
    });
    host.native_physical = Some(crate::skate3_native::graph_host::motion_native::Physical {
        centre_of_mass_velocity: [0.0; 4],
        system_up: [0.0, 1.0, 0.0, 0.0],
        board_reckoning_z: [0.0, 0.0, 1.0, 0.0],
        board_reckoning: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    });
}
fn packet(host: &MotionHost, name: &[u8]) -> f32 {
    host.animation
        .motion_attributes
        .iter()
        .rev()
        .find(|a| a.name == encode(name))
        .expect("Missing physical packet attribute")
        .value
}

#[test]
#[ignore = "requires the original stock graph, banks and collections (not in the repository)"]
fn stock_slide_lifecycle_shares_turning_latch_and_captured_stance() {
    let mut host = host();
    physical(&mut host);
    let frame = frame();
    let power = slide(&host, Operation::Update);
    let create = slide(&host, Operation::Create { right: true });
    let candidate = slide(&host, Operation::Candidate);
    let active = slide(&host, Operation::IsPowerSliding);
    let manual = slide(&host, Operation::ManualAttribute);
    let turning = behavior(&host, |v| matches!(v, MotionOperation::SetTurning(_)));
    let grab = host
        .remap
        .hooks
        .iter()
        .position(|&id| {
            matches!(
                host.operations[id],
                MotionOperation::Hook(crate::skate3_native::graph_host::motion_hooks::MotionHook::GrabSlide {
                    right: true
                })
            )
        })
        .expect("Authored GrabSlide hook missing");
    for fakie in [false, true] {
        host.slide_latch.reset();
        host.animation.motion_intents.clear();
        //A pre-graph snapshot must not override the current animation flag.
        host.physical.as_mut().unwrap().stance.0 = !fakie;
        host.animation.skater_animation_flags = Some((fakie as u32) << 29);
        for id in [power, candidate, active, create, turning, manual] {
            host.allocate(id, &frame);
        }
        host.begin(candidate, [0; 6], &frame);
        host.begin(turning, [0; 6], &frame);
        host.update(power, [0; 6], &frame); // native ground-entry history capture
        let (intent, start, right, value) = if fakie {
            ("LeftSlide", "LeftSlideStart", false, -0.7)
        } else {
            ("RightSlide", "RightSlideStart", true, 0.7)
        };
        host.animation.motion_intents.insert(intent.into(), value);
        host.animation.motion_intents.insert(start.into(), 1.0);
        host.update(power, [0; 6], &frame);
        assert!(host.slide_latch.start(right));
        host.update(turning, [0; 6], &frame);
        assert!(matches!(host.instances[turning], Instance::Turning(s) if s.mode == right as u32));
        assert_eq!(host.slide_latch.captured_fakie(), fakie);
        host.hook(grab, &frame);
        host.begin(create, [0; 6], &frame);
        host.begin(active, [0; 6], &frame);
        assert!(host.is_power_sliding);
        host.update(create, [0; 6], &frame);
        let expected_slide = if fakie { 1.0 - 0.7 } else { 0.7 };
        assert!((packet(&host, b"slide") - expected_slide).abs() < 1.0e-6);
        assert_eq!(packet(&host, b"turn"), host.turning.override_turn);
        // Current stance changes after entry; the captured side remains authoritative.
        host.animation.skater_animation_flags = Some(((!fakie) as u32) << 29);
        host.update(turning, [0; 6], &frame);
        assert_eq!(host.slide_latch.captured_fakie(), fakie);
        host.animation.motion_intents.insert("Manual".into(), -0.25);
        host.update(manual, [0; 6], &frame);
        assert_eq!(packet(&host, b"balance"), -0.25);
        host.animation.motion_intents.remove(intent);
        host.animation.motion_intents.remove(start);
        for _ in 0..4 {
            host.update(power, [0; 6], &frame);
        }
        assert!(
            MotionCondition::ShouldLeaveSlide { right: true }
                .evaluate(&host, &frame)
                .unwrap()
        );
        host.update(turning, [0; 6], &frame);
        assert!(matches!(host.instances[turning], Instance::Turning(s) if s.mode == 2));
        host.end(active, [0; 6], &frame);
        host.end(candidate, [0; 6], &frame);
        assert!(!host.is_power_sliding && !host.slide_latch.candidate_enabled());
        assert!(host.errors.is_empty(), "{:?}", host.errors);
    }
}

#[test]
#[ignore = "requires the original stock graph, banks and collections (not in the repository)"]
fn stock_slide_deceleration_uses_ground_frame_and_resets_on_entry() {
    let mut host = host();
    physical(&mut host);
    let frame = frame();
    let decel = slide(&host, Operation::Deceleration);
    let spin = slide(&host, Operation::Spin);
    host.allocate(decel, &frame);
    host.begin(decel, [0; 6], &frame);
    host.fakie_physical.as_mut().unwrap().deck_velocity = [0.0, 2.0, 0.0, 0.0];
    host.update(decel, [0; 6], &frame);
    let value = |host: &MotionHost| match host.instances[decel] {
        Instance::SlideDeceleration(value) => value,
        _ => panic!("Wrong deceleration instance"),
    };
    assert_eq!(value(&host), 0.0); // motion parallel to ground up is discarded
    host.native_physical.as_mut().unwrap().board_reckoning = [
        [0.0, 1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [100.0, 200.0, 300.0, 1.0],
    ];
    host.update(decel, [0; 6], &frame);
    let first = value(&host);
    assert!(first > 0.0 && first.is_finite()); // same motion is now across the ground
    host.update(decel, [0; 6], &frame);
    assert!(value(&host) > first);
    host.end(decel, [0; 6], &frame);
    host.begin(decel, [0; 6], &frame);
    assert_eq!(value(&host), 0.0);
    host.update(decel, [0; 6], &frame);
    assert_eq!(value(&host), first);
    host.begin(spin, [0; 6], &frame);
    host.update(spin, [0; 6], &frame);
    host.end(spin, [0; 6], &frame);
    // Both are animation parameters, so neither creates a physical packet scalar.
    assert!(host.animation.motion_attributes.is_empty());
    assert!(host.errors.is_empty(), "{:?}", host.errors);
}

#[test]
fn slide_direction_preserves_native_unordered_threshold_branch() {
    assert_eq!(
        crate::skate3_native::graph_host::motion_sliding::direction(
            [f32::NAN, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            false
        ),
        1.0
    );
}

#[test]
#[ignore = "requires installed stock graph assets; compile-only during bail work"]
fn bail_reset_restores_saved_stance_in_the_real_graph_dispatch() {
    use crate::skate3_native::graph_host::motion_reset::Operation as Reset;
    let mut host = host();
    let reset = behavior(&host, |op| matches!(op, MotionOperation::ResetAnimation(Reset::SkaterAnimation)));
    let stance = behavior(&host, |op| matches!(op, MotionOperation::ResetAnimation(Reset::GivenStance)));
    for natural in [0, 1] {
        for requested in [0, 1] {
            host.animation.skater_animation_flags = Some(0xf80a_0000);
            host.animation.natural_stance = natural;
            host.animation.relative_stance = 1;
            host.animation.requested_stance = requested;
            host.begin(reset, [0; 6], &frame());
            assert_eq!(host.animation.requested_stance, requested);
            assert_eq!(host.animation.skater_animation_flags, Some(0x0802_0000));
            host.begin(stance, [0; 6], &frame());
            assert_eq!(host.animation.requested_stance, 0);
            let backwards = natural == requested;
            assert_eq!(host.playback_context.is_mirrored, Some(backwards));
            assert_eq!(host.animation.relative_stance, requested);
            assert!(host.errors.is_empty(), "{:?}", host.errors);
        }
    }
}
