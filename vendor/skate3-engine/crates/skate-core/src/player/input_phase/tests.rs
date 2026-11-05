use crate::animation::output::actor_packet::ExternalPhysicsInput;
use crate::input::animation_packet::AnimationPacketFields;

use super::publication::{
    effective_animation_transform, publish_external_physics, publish_physical_outputs,
    select_surface,
};
use super::*;

fn external(flags: u32) -> ExternalPhysicsInput {
    ExternalPhysicsInput {
        vectors: [[0; 4]; 10],
        flags,
    }
}

fn animation_publication() -> AnimationPacketFields {
    AnimationPacketFields {
        stance_byte: 0,
        timestep: 0.25,
        scalar_10388: 0.0,
        flags_10375_10496_10784: [0; 3],
        vector_10480: [0; 4],
        matrix_10704: [[0; 4]; 4],
        byte_10768: 0,
        truck_tightness: 0.0,
        scalar_10792: 0.0,
        flag_10371: 0,
    }
}

fn packet<'a>(
    publication: &'a AnimationPacketFields,
    external_physics: &'a ExternalPhysicsInput,
) -> AnimationInputPacket<'a> {
    AnimationInputPacket {
        publication,
        external_physics_10512: external_physics,
        flag_10369: 0,
        flag_10370: 0,
        flag_10373: 0,
        suppress_transition_10376: 0,
        use_external_physics_10688: 0,
        external_physics_flag_10689: 0,
        flag_10786: 0,
        flag_10787: 0,
        force_braking_10796: 0,
        vector_10816: [0; 4],
        vector_10832: [0; 4],
        vector_10848: [0; 4],
        vector_10864: [0; 4],
        vector_10880: [0; 4],
        vector_10896: [0; 4],
        scalar_10912: 0.0,
        scalar_10916: 0.0,
        state_variant_10928: 0,
        flags_10932: 0,
    }
}

#[derive(Default)]
struct Services {
    calls: Vec<&'static str>,
    manager_state_change: Option<(u32, u32)>,
    actor_input_available: bool,
}

impl InputPhaseServices for Services {
    type Error = ();

    fn update_pre_input_manager_82d81610(
        &mut self,
        _player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
    ) -> Result<(), Self::Error> {
        self.calls.push("manager");
        if let Some((state, category)) = self.manager_state_change {
            physical.state.state_16 = state;
            physical.state.category_12 = category;
        }
        Ok(())
    }

    fn reset_processed_input_82bf9ef0(
        &mut self,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error> {
        self.calls.push("reset");
        *output = ProcessedPhysicsInput::default();
        Ok(())
    }

    fn actor_query_slot_56(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("actor56");
        Ok(56)
    }

    fn actor_query_slot_44(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("actor44");
        Ok(44)
    }

    fn reset_player_probe_82d7a330(
        &mut self,
        player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
    ) -> Result<(), Self::Error> {
        self.calls.push("probe_reset");
        player.probe.byte_104 = 0;
        Ok(())
    }

    fn check_teleport_82db88c8(
        &mut self,
        _player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
        _output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error> {
        self.calls.push("teleport");
        Ok(())
    }

    fn actor_input_available_slot_4(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("actor_input");
        Ok(self.actor_input_available)
    }

    fn transition_action_825903c8(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("transition");
        Ok(2.0)
    }

    fn calculate_ground_position_82c02840(
        &mut self,
        _physical: &PhysicalPlayerInput,
    ) -> Result<RawVector, Self::Error> {
        self.calls.push("ground_position");
        Ok([2.0f32, 4.0, 6.0, 0.0].map(f32::to_bits))
    }

    fn prepare_board_toolkit_82c013f0(
        &mut self,
        _player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
        _output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error> {
        self.calls.push("board_toolkit");
        Ok(())
    }

    fn process_skeleton_82bd8918(
        &mut self,
        _packet: &AnimationInputPacket<'_>,
        _physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error> {
        self.calls.push("skeleton");
        output.flags_2472 |= (1 << 21) | (1 << 23);
        output.flags_2476 |= 1 << 22;
        output.spin_input_2672 = 2.0;
        output.crouch_2776 = 3.0;
        Ok(())
    }

    fn update_grind_manager_82d8a828(
        &mut self,
        _player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
        _output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error> {
        self.calls.push("grind");
        Ok(())
    }
}

#[test]
fn complete_phase_keeps_native_service_order_and_persistent_counters() {
    let publication = animation_publication();
    let external_physics = external(0);
    let packet = packet(&publication, &external_physics);
    let mut player = PlayerInputState {
        manager_1856_counter_320: 2,
        manager_1852_flag_256: 9,
        grounded_frames_1300: 6,
        update_count_1316: 42,
        frames_since_teleport_1328: 2,
        ground_history_frames_1304: 1,
        time_since_last_input_1348: 1.0,
        time_on_ground_1352: 1.0,
        previous_spin_input_1360: 1.0,
        previous_crouch_1364: 1.0,
        previous_ground_position_1200: [1.0f32, 2.0, 3.0, 0.0].map(f32::to_bits),
        damped_ground_delta_1248: [0; 4],
        probe: ProbeFields {
            byte_104: 7,
            ..ProbeFields::default()
        },
        left_line_test_1536: LineTestFields {
            surface: 123,
            valid: 2,
            ..LineTestFields::default()
        },
        ..PlayerInputState::default()
    };
    let mut physical = PhysicalPlayerInput::default();
    physical.state.category_12 = 100;
    physical.state.state_16 = 104;
    physical.state.skitch_value_40 = 77;
    physical.state.signed_ground_step_84 = 1;
    physical.surface_default_mode = 1;
    physical.ground.scalar_276 = 0.0;
    physical.ground.scalar_292 = 0.5;
    physical.ground.scalar_296 = 0.75;
    let mut output = ProcessedPhysicsInput::default();
    let mut services = Services {
        actor_input_available: true,
        ..Services::default()
    };

    process_input(
        &mut player,
        &mut physical,
        &packet,
        &mut output,
        &mut services,
    )
    .unwrap();

    assert_eq!(
        services.calls,
        [
            "manager",
            "reset",
            "actor56",
            "actor44",
            "probe_reset",
            "teleport",
            "actor_input",
            "transition",
            "ground_position",
            "board_toolkit",
            "skeleton",
            "grind",
        ]
    );
    assert_eq!(player.manager_1856_counter_320, 1);
    assert_eq!(player.manager_1852_flag_256, 0);
    assert_eq!(output.actor_query_2948, 56);
    assert_eq!(output.actor_query_2952, 44);
    assert_eq!(output.probe_1792.byte_104, 7);
    assert_eq!(player.probe.byte_104, 0);
    assert_eq!(player.grounded_frames_1300, 7);
    assert_eq!(player.update_count_1316, 42);
    assert_eq!(output.update_count_2568, 42);
    assert_eq!(output.time_on_ground_2752, 1.25);
    assert_ne!(output.flags_2468 & (1 << 13), 0);
    assert_eq!(output.transition_2636, 1.0);
    assert_eq!(player.skitch_timer_1376, 1.75);
    assert_eq!(output.skitch_value_2592, 77);
    assert_eq!(output.ground_timer_2848, 0.25);
    assert_eq!(output.secondary_ground_timer_2852, 0.5);
    assert_eq!(
        output.vectors_464_480_496_512_528[1].map(f32::from_bits),
        [2.0, 4.0, 6.0, 0.0]
    );
    assert_eq!(
        output.vectors_464_480_496_512_528[2].map(f32::from_bits),
        [2.0, 4.0, 6.0, 0.0]
    );
    for (actual, expected) in output.vectors_464_480_496_512_528[3]
        .map(f32::from_bits)
        .into_iter()
        .zip([0.2, 0.4, 0.6, 0.0])
    {
        assert!((actual - expected).abs() < 1e-6);
    }
    assert_eq!(
        player.ground_delta_1232.map(f32::from_bits),
        [1.0, 2.0, 3.0, 0.0]
    );
    assert_eq!(output.time_since_last_input_2748, 1.25);
    assert_eq!(output.prepared_jump_704.map(f32::from_bits), [0.0; 4]);
    assert_eq!(output.spin_same_direction_frames_2580, 1);
    assert_eq!(output.crouch_delta_2780, 2.0);
    assert_eq!(output.left_surface_2596, 123);
}

#[test]
fn manager_call_precedes_category_capture_but_state_number_is_already_captured() {
    let publication = animation_publication();
    let external_physics = external(0);
    let packet = packet(&publication, &external_physics);
    let mut player = PlayerInputState::default();
    let mut physical = PhysicalPlayerInput::default();
    physical.state.state_16 = 11;
    physical.state.category_12 = 100;
    physical.surface_default_mode = 1;
    physical.skeleton.anim_to_world_11920[3] = [7, 8, 9, 10];
    let mut output = ProcessedPhysicsInput::default();
    let mut services = Services {
        manager_state_change: Some((22, 500)),
        ..Services::default()
    };

    process_input(
        &mut player,
        &mut physical,
        &packet,
        &mut output,
        &mut services,
    )
    .unwrap();

    assert_eq!(output.state_2504, 11);
    assert_eq!(output.state_2508, 11);
    assert_eq!(output.category_2512, 500);
    assert_eq!(output.category_2516, 500);
    assert_eq!(player.current_ground_position_1216, [7, 8, 9, 10]);
    assert!(!services.calls.contains(&"ground_position"));
    assert_eq!(player.time_off_board_1368.to_bits(), 0x3c88_8889);
}

#[test]
fn physical_publication_uses_low_flag_bits_and_stance_wheel_permutation() {
    let publication = animation_publication();
    let external_physics = external(0);
    let mut packet = packet(&publication, &external_physics);
    packet.flags_10932 = (1 << 31) | (1 << 29) | (1 << 22);
    packet.flag_10369 = 2;
    let mut physical = PhysicalPlayerInput::default();
    physical.ground.flag_317 = 3;
    physical.air.flag_446 = 2;
    physical.off_board.scalar_32 = f32::NAN;
    physical.collision.wheel_contact_3296_3299 = [1, 0, 0, 0];
    let mut output = ProcessedPhysicsInput::default();
    output.flags_2468 &= !(1 << 20);

    publish_physical_outputs(&physical, &packet, &mut output);
    assert_ne!(output.flags_2476 & (1 << 6), 0);
    assert_eq!(output.flags_2476 & (1 << 5), 0);
    assert_ne!(output.flags_2476 & (1 << 4), 0);
    assert_ne!(output.flags_2488 & (1 << 24), 0);
    assert_eq!(output.flags_2480 & 1, 0);
    assert_ne!(output.flags_2476 & (1 << 27), 0);
    assert_eq!(output.flags_2476 & 1, 0);
    assert_eq!(output.flags_2476 & (1 << 17), 0);
    assert_eq!(output.flags_2472 & (1 << 27), 0);
    assert_ne!(output.flags_2472 & (1 << 26), 0);
    assert_ne!(output.flags_2472 & (1 << 25), 0);
    assert_eq!(output.flags_2472 & (1 << 24), 0);

    output.flags_2468 |= 1 << 20;
    publish_physical_outputs(&physical, &packet, &mut output);
    assert_ne!(output.flags_2472 & (1 << 27), 0);
    assert_eq!(output.flags_2472 & (1 << 26), 0);
    assert_eq!(output.flags_2472 & (1 << 25), 0);
    assert_ne!(output.flags_2472 & (1 << 24), 0);
}

#[test]
fn external_physics_fallback_uses_cached_trajectory_and_preserves_packet_high_bits() {
    let publication = animation_publication();
    let mut incoming = external((1 << 31) | (1 << 25));
    incoming.vectors = [[0x100; 4]; 10];
    let mut packet = packet(&publication, &incoming);
    packet.use_external_physics_10688 = 1;
    packet.external_physics_flag_10689 = 3;
    let mut player = PlayerInputState::default();
    for index in 5..10 {
        player.external_physics_cache_1008.vectors[index] = [index as u32; 4];
    }
    player.external_physics_cache_1008.flags = 1 << 27;
    let mut output = ProcessedPhysicsInput::default();
    output.external_physics_1616.flags = 0x1234;

    publish_external_physics(&mut player, &packet, &mut output);

    assert_ne!(output.flags_2472 & (1 << 29), 0);
    assert_ne!(output.flags_2472 & (1 << 28), 0);
    assert_eq!(output.external_physics_1616.vectors[4], [0x100; 4]);
    assert_eq!(output.external_physics_1616.vectors[5], [5; 4]);
    assert_eq!(output.external_physics_1616.vectors[9], [9; 4]);
    assert_ne!(output.external_physics_1616.flags & (1 << 31), 0);
    assert_ne!(output.external_physics_1616.flags & (1 << 27), 0);
    assert_ne!(output.external_physics_1616.flags & (1 << 26), 0);
    assert_eq!(output.external_physics_1616.flags & 0x01ff_ffff, 0x1234);
}

#[test]
fn surface_selector_and_effective_transform_match_recovered_switches() {
    let publication = animation_publication();
    let external_physics = external(0);
    let mut packet = packet(&publication, &external_physics);
    let mut player = PlayerInputState::default();
    player.state_variants_1408[2].surface_override_enabled_60 = 1;
    packet.state_variant_10928 = 2;
    let mut output = ProcessedPhysicsInput::default();

    select_surface(&player, &packet, 13, &mut output).unwrap();
    assert_eq!(output.surface_mode_2540, 2);
    assert_eq!(output.state_variant_ref_2548.unwrap().offset, 0x5a0);
    assert_eq!(output.surface_primary_ref_2544.unwrap().offset, 40);
    assert_eq!(output.surface_secondary_ref_2552.unwrap().offset, 116);

    player.state_variants_1408[2].surface_override_enabled_60 = 0;
    select_surface(&player, &packet, 6, &mut output).unwrap();
    assert_eq!(output.surface_mode_2540, 3);
    packet.state_variant_10928 = 5;
    assert_eq!(select_surface(&player, &packet, 1, &mut output), Err(5));

    let matrix = [
        [
            1.0f32.to_bits(),
            (-2.0f32).to_bits(),
            0.0f32.to_bits(),
            4.0f32.to_bits(),
        ],
        [5.0f32.to_bits(); 4],
        [
            6.0f32.to_bits(),
            7.0f32.to_bits(),
            8.0f32.to_bits(),
            9.0f32.to_bits(),
        ],
        [10.0f32.to_bits(); 4],
    ];
    let flipped = effective_animation_transform(matrix, 1 << 2);
    assert_eq!(flipped[0][0], (-1.0f32).to_bits());
    assert_eq!(flipped[0][1], 2.0f32.to_bits());
    assert_eq!(flipped[1], matrix[1]);
    assert_eq!(flipped[2][0], (-6.0f32).to_bits());
    assert_eq!(flipped[3], matrix[3]);
}

#[test]
fn physical_vector_publication_uses_the_restored_native_offsets() {
    let publication = animation_publication();
    let external = external(0);
    let packet = packet(&publication, &external);
    let mut physical = PhysicalPlayerInput::default();
    physical.board_reckoning_side_176 = [176; 4];
    physical.reckoning.vector_16 = [16; 4];
    physical.reckoning.vector_64 = [64; 4];
    physical.reckoning.vector_96 = [96; 4];
    physical.physics.vector_16 = [116; 4];
    physical.off_board.vector_64 = [164; 4];
    physical.air.vector_160 = [160; 4];
    let mut output = ProcessedPhysicsInput::default();
    publish_physical_outputs(&physical, &packet, &mut output);
    assert_eq!(
        output.vectors_544_560_592_608,
        [[96; 4], [176; 4], [64; 4], [16; 4]]
    );
    assert_eq!(output.vectors_720_784_800_816_832_864[3], [116; 4]);
    assert_eq!(output.vectors_880_896_912_928_944[2], [164; 4]);
    physical.air.use_air_reckoning_452 = 1;
    publish_physical_outputs(&physical, &packet, &mut output);
    assert_eq!(output.vectors_544_560_592_608[3], [160; 4]);
}
