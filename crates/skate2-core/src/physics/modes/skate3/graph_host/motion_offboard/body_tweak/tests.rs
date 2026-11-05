use super::*;

#[test]
fn native_air_tweak_release_latch_keeps_signed_axes_and_strict_counter() {
    let mut state = State::default();
    assert_eq!(state.gates(Some(1.0), Some(0.0), false, 0.0), (false, false));
    state.ticks = 80;
    assert!(!state.gates(Some(1.0), Some(0.0), false, 0.0).0);
    state.ticks = 81;
    assert!(state.gates(Some(1.0), Some(0.0), false, 0.0).0);
    state.begin();
    assert!(state.gates(Some(-1.0), Some(0.0), false, 0.0).0);
    state.begin();
    assert!(!state.gates(None, Some(0.0), false, 0.0).0);
    assert!(!state.released, "both native intent lookups must succeed to release");
    assert_eq!(state.gates(Some(0.0), Some(0.0), false, 4.0), (false, false));
    assert_eq!(state.gates(Some(0.0), Some(0.0), false, 4.01), (false, true));
    assert_eq!(state.gates(Some(1.0), Some(0.0), true, 0.0), (true, true));
}

#[test]
fn native_air_tweak_begin_preserves_filters_and_center_request_decays_them() {
    let mut state = State::default();
    state.filter(1.0, -1.0, false);
    assert_eq!(state.axes, [0.15, -0.15]);
    state.begin();
    assert_eq!(state.axes, [0.15, -0.15]);
    state.filter(-1.0, 1.0, true);
    assert_eq!(state.axes, [0.15 * 0.85, -0.15 * 0.85]);
    assert_eq!(state.ticks, 0);
    assert!(!state.released && !state.into_started && !state.cycle_started);
}

#[test]
#[ignore = "requires private stock OffBoard bank; graph leaf/pose regression, not gameplay acceptance"]
fn stock_air_tweak_channels_parameters_pose_requests_and_teardown() {
    use skate3_core::animation::playback_tree::{Evaluation, PoseCommand};
    use skate3_core::animation::playback::{PlaybackRequest, PlaybackService};
    use skate_data::animation_banks::AnimationBanks;
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT")
        .expect("Set SKATE3_ASSET_ROOT"));
    let banks = AnimationBanks::load(&root).unwrap();
    let evaluator = crate::skate3_native::animation_pose::PoseEvaluator::from_banks(&banks).unwrap();
    let settings = Settings {
        nb_cycle: "B_OBAIR_BODYTWEAK_NB_CYC".into(),
        nb_into: "B_OBAIR_BODYTWEAK_NB_INTO".into(),
        br_cycle: "B_OBAIR_BODYTWEAK_BR_CYC".into(),
        br_into: "B_OBAIR_BODYTWEAK_BR_INTO".into(),
    };
    for held in [false, true] {
        let mut animation = MotionAnimation::from_metadata(banks.metadata().unwrap());
        animation.skater_animation_flags = Some(0);
        animation.play(PlaybackRequest {
            animation: if held { "BR_STAND_0_CYC" } else { "NB_STAND_0_CYC" }.into(),
            speed: 1.0, start_time: 0.0,
            transition: TransitionSettings {
                kind: 1, seconds: 0.0, under: 0, matching: 0,
                use_channels_from_weights: false,
            },
        }).unwrap();
        let mut controls = Controls::default();
        let mut state = State::default();
        let mut physical = Physical { time_to_land: 2.0, scalar_92: 0.0, board_held: held };
        let mut poses = Vec::new();
        state.begin();
        animation.motion_intents.insert("OB_AirBodyTweakX", 0.0);
        animation.motion_intents.insert("OB_AirBodyTweakY", 0.0);
        state.update(&settings, &mut animation, &mut controls, physical).unwrap();
        assert!(!animation.channels.has(CHANNEL));
        for direction in [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]] {
            animation.motion_intents.insert("OB_AirBodyTweakX", direction[0]);
            animation.motion_intents.insert("OB_AirBodyTweakY", direction[1]);
            for _ in 0..90 {
                state.update(&settings, &mut animation, &mut controls, physical).unwrap();
                assert!(animation.pending_parameter(encode(b"bodytweakx")).is_some());
                assert!(animation.pending_parameter(encode(b"bodytweaky")).is_some());
                assert_ne!(animation.skater_animation_flags.unwrap() & 0x8000, 0);
                assert!(controls.seed_from_air_tweak);
                animation.apply_parameters().unwrap();
                animation.advance(1.0 / 60.0, 0.0);
                animation.refresh_tree_attributes().unwrap();
                let commands = animation.evaluate_pose(Evaluation {
                    cull_threshold: 0.01, update_history: true,
                }).unwrap();
                let pose = evaluator.evaluate(&commands).unwrap();
                assert!(pose.iter().all(|s| s.rotation.iter().chain(s.translation.iter())
                    .all(|v| v.is_finite())));
                if commands.iter().any(|c| matches!(c, PoseCommand::ChannelBlend { .. })) {
                    poses.push(pose);
                }
            }
        }
        assert!(state.into_started && state.cycle_started);
        assert!(poses.windows(2).any(|p| p[0].iter().zip(&p[1])
            .any(|(a, b)| a.rotation != b.rotation || a.translation != b.translation)));
        physical.time_to_land = 0.05;
        animation.skater_animation_flags = Some(0);
        state.update(&settings, &mut animation, &mut controls, physical).unwrap();
        assert_ne!(animation.skater_animation_flags.unwrap() & 0x10000, 0);
        state.end(&mut animation);
        for _ in 0..10 { animation.advance(1.0 / 60.0, 0.0); }
        assert!(!animation.channels.has(CHANNEL));
        assert!(controls.seed_from_air_tweak, "native End preserves Wipeout handoff latch");
    }
}
