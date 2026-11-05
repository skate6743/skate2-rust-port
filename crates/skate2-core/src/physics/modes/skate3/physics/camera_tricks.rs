use super::*;
#[test]
#[ignore = "requires private stock graphs and animation assets"]
fn tricks_select_air_camera_without_chasing_the_board_upward() {
    for flick in [[0, 32767], [-32767, 16384], [32767, 16384]] {
        replay(flick);
    }
}
fn replay(flick: [i16; 2]) {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
    let assets = skate_data::GameAssets::load(&root).unwrap();
    let graphs = crate::skate3_native::graph_runtime::StockGraphs::load(&root, &assets).unwrap();
    let mut physics =
        Skate3PhysicsState::load_with_difficulty(&root, None, crate::skate3_native::difficulty::Difficulty::Normal)
            .unwrap();
    let mut skater = Skate3ControllerState::load(&root, &graphs, &physics, "normal").unwrap();
    let mut camera = crate::skate3_native::camera::CameraRuntime::load(&root).unwrap();
    let mut input = crate::skate3_native::input::ControllerInput::default();
    let mut controls = super::super::PlayerControls::load(&root).unwrap();
    let mut saw_air = false;
    let mut saw_air_shot = false;
    let mut max_air_height = 0.0_f32;
    for tick in 0..300 {
        input.sample_raw_for_test(skate3_core::input::xbox::XboxState {
            buttons: 0,
            triggers: [0; 2],
            left: [0; 2],
            right: if (120..150).contains(&tick) {
                [0, -32767]
            } else if (150..153).contains(&tick) {
                flick
            } else {
                [0; 2]
            },
        });
        let mut actions = input.player_actions();
        controls.update(
            &mut actions,
            physics.settings.step.simulation.time_step,
            physics.settings.input_magnitude_threshold,
            skater.player_input.physical.scoring.capabilities_204,
        );
        controls.publish_gestures(
            physics.animation_profile.physics_mode,
            skater.player_input.physical.state.state_16,
        );
        super::super::frame::advance(
            &mut physics,
            &mut skater,
            &mut controls,
            &graphs,
            &mut actions,
            true,
            &mut camera,
        )
        .unwrap();
        let air = skater.player_input.physical.air.known_air_valid_437 != 0;
        if air {
            saw_air = true;
            saw_air_shot |= camera.selected_shot() == "low_ollie";
            max_air_height = max_air_height.max(camera.frame.unwrap().position[1]);
            assert_eq!(
                skater.player_input.physical.air.flag_441, 0,
                "ordinary flicks must not require body-flip mode"
            );
        }
    }
    assert!(saw_air, "flick {flick:?} did not launch");
    assert!(saw_air_shot, "flick {flick:?} stayed in the chase shot");
    assert!(
        max_air_height < 2.8,
        "camera chased the board upward: flick={flick:?} height={max_air_height}"
    );
    assert_eq!(
        camera.selected_shot(),
        "bl_high_chase",
        "normal camera must resume after landing"
    );
    eprintln!("flick={flick:?}, max camera height={max_air_height}");
}
