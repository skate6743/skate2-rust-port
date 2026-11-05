use skate_core::animation::{
    commands::{
        batch::{CompletedSetData, SetDataQueue},
        buffers::{BufferError, PoseBuffers},
    },
    output::{
        actor_packet::{
            self, ActorPacketFields, ActorPublicationEnvironment, ActorPublicationState,
            ExternalPhysicsInput, ExternalReset, PlayerPhysicsSettingsSource,
        },
        attributes::{AttributeName, MotionGraphAttribute, PacketAttributes},
        intents::{Intent, IntentCapacityExceeded, IntentMap, IntentName},
        motion_graph_packet::{GestureData, MotionGraphPacket, TrickData, TrickIdentifier},
        packet_reset::{AdditionalResetFields, RESET_POSE},
        physics_packet::{ActorPoseBuffers, PhysicsPosePacket, SkaterPublicationState},
        setup_physics::{
            self, AnimationPublication, IntentPublication, MotionGraphPublication,
            PhysicsInputPacket, PublicationError,
        },
    },
};
use std::num::NonZeroU16;

fn intent(id: u32, value: f32) -> Intent {
    Intent {
        name: IntentName([id, 0, 0, 0, 0, 0]),
        value,
    }
}
#[test]
fn intent_assignment_reverses_collisions_and_retains_first_duplicate_value() {
    let mut source = IntentMap::default();
    for entry in [
        intent(1, 1.0),
        intent(62, 2.0),
        intent(123, 3.0),
        intent(2, 4.0),
    ] {
        assert_eq!(source.insert(entry), Ok(true));
    }
    assert_eq!(source.insert(intent(62, 99.0)), Ok(false));
    assert_eq!(
        source.entries().map(|e| e.name.0[0]).collect::<Vec<_>>(),
        [123, 62, 1, 2]
    );
    let mut destination = IntentMap::default();
    destination.insert(intent(99, 99.0)).unwrap();
    destination.replace_from(&source);
    assert_eq!(
        destination
            .entries()
            .map(|e| (e.name.0[0], e.value))
            .collect::<Vec<_>>(),
        [(1, 1.0), (62, 2.0), (123, 3.0), (2, 4.0)]
    );
    assert_eq!(
        source.entries().map(|e| e.name.0[0]).collect::<Vec<_>>(),
        [123, 62, 1, 2]
    );
}
#[test]
fn intent_hash_wraps_all_six_words_and_capacity_allows_existing_keys() {
    let mut map = IntentMap::default();
    map.insert(intent(1, 7.0)).unwrap();
    let wrapped = Intent {
        name: IntentName([u32::MAX, 0, 0, 0, 0, 2]),
        value: -0.0,
    };
    map.insert(wrapped).unwrap();
    assert_eq!(map.entries().next().unwrap().name, wrapped.name);
    assert_eq!(map.entries().next().unwrap().value.to_bits(), 0x80000000);
    for id in 2..64 {
        map.insert(intent(id, id as f32)).unwrap();
    }
    assert_eq!(map.len(), 64);
    assert_eq!(map.insert(intent(1, 8.0)), Ok(false));
    assert_eq!(map.insert(intent(1000, 8.0)), Err(IntentCapacityExceeded));
    assert_eq!(map.len(), 64);
    map.clear();
    assert!(map.is_empty());
    assert_eq!(map.insert(wrapped), Ok(true));
}

fn graph() -> MotionGraphPacket {
    MotionGraphPacket {
        trick: TrickData {
            identifiers: [TrickIdentifier([7; 6]); 2],
            word48: 8,
            scalars52_56: [9.0, 10.0],
        },
        gesture: GestureData {
            gesture: 11,
            flag_bytes: [1, 2, 3, 255],
        },
        wipeout_gesture: [12.0, 13.0],
    }
}
fn packet() -> PhysicsInputPacket {
    PhysicsInputPacket {
        pose: PhysicsPosePacket {
            bone_count: 1,
            hierarchy: vec![[[77.0; 4]; 4]],
            local: vec![[[88.0; 4]; 4]],
            timestep: 0.0,
            foot_surface_ids: [42; 2],
            flags: u32::MAX,
            board_flipped: false,
            mirrored: false,
            riding_switch: false,
            riding_fakie: false,
            weight_forwards: false,
            regular_stance: false,
            air_dismount_revert_frames: 0,
        },
        reset: AdditionalResetFields {
            compression: 99.0,
            foot_ik_influence: [99.0; 2],
            next_step_position_valid: true,
            actor_flag_1904_bit23: false,
            actor_flag_1908_bit2: false,
            external_impulse_active: false,
            external_physics_input_active: false,
            externally_controlled: false,
            prevent_manual_respawn: false,
            ignore_respawn_reset_button: 0,
            force_braking: false,
            truck_tightness: 99.0,
            wheel_hardness: 99.0,
            auxiliary_vectors: [[99.0; 4]; 6],
            requested_physics_mode: 99,
        },
        motion_graph: graph(),
        intents: IntentMap::default(),
        attributes: PacketAttributes::default(),
        actor: ActorPacketFields {
            external_impulse: [0; 4],
            external_physics: ExternalPhysicsInput {
                vectors: [[0; 4]; 10],
                flags: 0x01abcdef,
            },
            external_reset: ExternalReset {
                transform: [[0; 4]; 4],
                byte64: 0,
            },
            actor_flag_1904_bit29: false,
        },
    }
}
fn actor() -> ActorPublicationState {
    ActorPublicationState {
        flags1904: (1 << 23) | (1 << 25) | (1 << 27) | (1 << 28) | (1 << 29) | (1 << 30) | 1,
        flags1908: 4,
        external_controller_present: false,
        external_impulse: [0x7fc12345, 0x80000000, 1, 0xff],
        external_physics: ExternalPhysicsInput {
            vectors: std::array::from_fn(|i| [i as u32; 4]),
            flags: 0xaa765432,
        },
        external_reset: ExternalReset {
            transform: [[0x80000000; 4]; 4],
            byte64: 254,
        },
        player_index: 3,
    }
}
struct Environment {
    calls: Vec<&'static str>,
    scene: Option<i32>,
    selector: u8,
    missing_scene: bool,
    missing_mode: bool,
}
struct Settings {
    fixed: bool,
    missing_mode: bool,
}
impl PlayerPhysicsSettingsSource for Settings {
    type Error = &'static str;
    fn truck_tightness(&mut self, index: u32) -> Result<f32, Self::Error> {
        assert!(!self.fixed);
        assert_eq!(index, 3);
        Ok(0.2)
    }
    fn wheel_hardness(&mut self, index: u32) -> Result<f32, Self::Error> {
        assert!(!self.fixed);
        assert_eq!(index, 3);
        Ok(0.8)
    }
    fn requested_physics_mode(&mut self, index: u32) -> Result<u32, Self::Error> {
        assert_eq!(index, 3);
        if self.missing_mode {
            Err("mode unavailable")
        } else {
            Ok(4)
        }
    }
}
impl ActorPublicationEnvironment for Environment {
    type Error = &'static str;
    type Settings = Settings;
    fn current_scene_mode(&mut self) -> Result<Option<i32>, Self::Error> {
        self.calls.push("scene");
        if self.missing_scene {
            Err("scene service unavailable")
        } else {
            Ok(self.scene)
        }
    }
    fn ignore_respawn_reset_button(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("global-byte");
        Ok(0xfe)
    }
    fn actor_subobject56_slot16(
        &mut self,
        actor: &mut ActorPublicationState,
    ) -> Result<u8, Self::Error> {
        self.calls.push("actor56");
        assert_eq!(actor.flags1904 & ((1 << 25) | (1 << 28)), 0);
        Ok(self.selector)
    }
    fn capture_physics_settings(&mut self) -> Result<Settings, Self::Error> {
        self.calls.push("settings-base");
        Ok(Settings {
            fixed: self.selector != 0,
            missing_mode: self.missing_mode,
        })
    }
}
fn environment() -> Environment {
    Environment {
        calls: vec![],
        scene: Some(19),
        selector: 0,
        missing_scene: false,
        missing_mode: false,
    }
}
#[test]
fn external_input_copy_transfers_each_high_flag_and_preserves_every_low_flag() {
    for bit in 0..32 {
        let mut destination = ExternalPhysicsInput {
            vectors: [[7; 4]; 10],
            flags: 1 << bit,
        };
        let source = ExternalPhysicsInput {
            vectors: [[0x7fc12345; 4]; 10],
            flags: 0,
        };
        destination.copy_from(&source);
        assert_eq!(destination.flags, if bit < 25 { 1 << bit } else { 0 });
        assert_eq!(destination.vectors, source.vectors);
        let source = ExternalPhysicsInput {
            flags: 1 << bit,
            ..source
        };
        destination.flags = 0;
        destination.copy_from(&source);
        assert_eq!(destination.flags, if bit >= 25 { 1 << bit } else { 0 });
    }
}
#[test]
fn actor_publication_copies_masked_payload_consumes_only_two_latches_and_repeats() {
    let mut packet = packet();
    let mut actor = actor();
    let initial = actor.flags1904;
    let mut environment = environment();
    actor_packet::publish(
        &mut actor,
        &mut packet.pose,
        &mut packet.reset,
        &mut packet.actor,
        &mut environment,
    )
    .unwrap();
    assert_eq!(
        environment.calls,
        ["scene", "global-byte", "settings-base", "actor56"]
    );
    assert_eq!(packet.actor.external_physics.flags, 0xababcdef);
    assert_eq!(
        packet.actor.external_physics.vectors,
        actor.external_physics.vectors
    );
    assert_eq!(packet.actor.external_reset, actor.external_reset);
    assert_eq!(packet.actor.external_impulse, actor.external_impulse);
    assert!(packet.reset.actor_flag_1904_bit23 && packet.reset.actor_flag_1908_bit2);
    assert!(packet.reset.external_impulse_active && packet.reset.external_physics_input_active);
    assert!(packet.actor.actor_flag_1904_bit29 && packet.reset.prevent_manual_respawn);
    assert!(!packet.reset.externally_controlled && !packet.reset.force_braking);
    assert_eq!(packet.pose.flags, u32::MAX);
    assert_eq!(actor.flags1904, initial & !((1 << 25) | (1 << 28)));
    assert_eq!(packet.reset.ignore_respawn_reset_button, 254);
    assert_eq!(
        (packet.reset.truck_tightness, packet.reset.wheel_hardness),
        (0.2, 0.8)
    );
    assert_eq!(packet.reset.requested_physics_mode, 4);
    actor_packet::publish(
        &mut actor,
        &mut packet.pose,
        &mut packet.reset,
        &mut packet.actor,
        &mut environment,
    )
    .unwrap();
    assert_eq!(packet.pose.flags & (1 << 24), 0);
    assert!(!packet.reset.prevent_manual_respawn);
    assert!(packet.reset.external_impulse_active && packet.actor.actor_flag_1904_bit29);
}
#[test]
fn scene_query_short_circuits_and_only_modes19_20_enable_flag() {
    for controller in [false, true] {
        for braking in [false, true] {
            for mode in [None, Some(18), Some(19), Some(20), Some(21)] {
                let mut p = packet();
                let mut a = actor();
                let mut e = environment();
                a.external_controller_present = controller;
                if braking {
                    a.flags1904 |= 1 << 26;
                }
                e.scene = mode;
                e.selector = 128;
                actor_packet::publish(&mut a, &mut p.pose, &mut p.reset, &mut p.actor, &mut e)
                    .unwrap();
                assert_eq!(e.calls.contains(&"scene"), !controller && !braking);
                assert_eq!(
                    p.pose.flags & (1 << 22) != 0,
                    !controller && !braking && matches!(mode, Some(19 | 20))
                );
                assert_eq!(p.reset.externally_controlled, controller);
                assert_eq!(p.reset.force_braking, braking);
                assert_eq!(p.reset.truck_tightness.to_bits(), 0x3f333333);
                assert_eq!(p.reset.wheel_hardness.to_bits(), 0x3f333333);
                assert_eq!(p.reset.requested_physics_mode, 4);
            }
        }
    }
}
#[test]
fn missing_mode_retains_both_branch_scalars_without_reading_unused_settings() {
    for fixed in [false, true] {
        let mut p = packet();
        let mut a = actor();
        let mut e = environment();
        e.selector = u8::from(fixed);
        e.missing_mode = true;
        assert_eq!(
            actor_packet::publish(&mut a, &mut p.pose, &mut p.reset, &mut p.actor, &mut e),
            Err("mode unavailable")
        );
        let expected = if fixed { (0.7, 0.7) } else { (0.2, 0.8) };
        assert_eq!((p.reset.truck_tightness, p.reset.wheel_hardness), expected);
        assert_eq!(p.reset.requested_physics_mode, 99);
        assert_eq!(
            e.calls,
            ["scene", "global-byte", "settings-base", "actor56"]
        );
    }
}

fn animation(initialized: bool) -> (CompletedSetData, ActorPoseBuffers, SkaterPublicationState) {
    let mut buffers = PoseBuffers::default();
    let hierarchy = if initialized {
        buffers.matrices.initialized([[[1.0; 4]; 4]])
    } else {
        buffers.matrices.allocate(1)
    };
    let local = buffers.matrices.initialized([[[2.0; 4]; 4]]);
    let completed = SetDataQueue::new(buffers, NonZeroU16::new(1).unwrap())
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    (
        completed,
        ActorPoseBuffers {
            hierarchy: hierarchy.at(0),
            local: local.at(0),
        },
        SkaterPublicationState {
            orientation_bit31: false,
            mirrored: false,
            riding_fakie: true,
            weight_on_nose: true,
            relative_stance: 1,
            natural_stance: 0,
            request_bit16: true,
            request_bit15: true,
            air_dismount_revert_requested: true,
            air_dismount_revert_frames: 12,
            signal: None,
        },
    )
}
#[test]
fn composition_overwrites_reset_poses_then_merges_graph_lists_and_binds_owned_packet() {
    let (completed, poses, mut state) = animation(true);
    let mut p = packet();
    let mut a = actor();
    let mut e = environment();
    let mut mg = graph();
    mg.gesture.gesture = 987;
    let mut intents = IntentMap::default();
    intents.insert(intent(1, 3.0)).unwrap();
    let attrs = [MotionGraphAttribute {
        name: AttributeName([1; 5]),
        value: 4.0,
    }];
    let tree = [MotionGraphAttribute {
        name: AttributeName([1; 5]),
        value: 5.0,
    }
    .to_animation()];
    let binding = setup_physics::compose(
        &mut p,
        AnimationPublication {
            state: &mut state,
            completed: &completed,
            poses: &poses,
            signal_name: b"signup",
            tree_attributes: &tree,
        },
        MotionGraphPublication {
            state: &mg,
            intents: IntentPublication::MotionGraph(&intents),
            attributes: &attrs,
        },
        &mut a,
        &mut e,
    )
    .unwrap();
    let published = binding.packet();
    assert_eq!(published.pose.hierarchy, [[[1.0; 4]; 4]]);
    assert_eq!(published.pose.local, [[[2.0; 4]; 4]]);
    assert_eq!(published.pose.flags, 0x19ffffff);
    assert_eq!(published.reset.compression, 0.5);
    assert_eq!(published.pose.timestep.to_bits(), 0x3c888889);
    assert_eq!(published.motion_graph.gesture.gesture, 987);
    assert_eq!(
        published
            .attributes
            .entries()
            .iter()
            .map(|a| a.payload.0[0])
            .collect::<Vec<_>>(),
        [Some(4.0f32.to_bits()), Some(5.0f32.to_bits())]
    );
    assert_eq!(published.intents.entries().next().unwrap().value, 3.0);
    assert!(!state.request_bit16 && !state.request_bit15 && !state.air_dismount_revert_requested);
    assert_eq!(mg.gesture.gesture, 987);
}
#[test]
fn composition_runtime_failure_retains_prior_phases_but_does_not_bind() {
    let (completed, poses, mut state) = animation(true);
    let mut p = packet();
    let mut a = actor();
    let mut e = environment();
    let mg = graph();
    p.intents.insert(intent(1, 3.0)).unwrap();
    p.intents.insert(intent(62, 4.0)).unwrap();
    e.missing_scene = true;
    let error = setup_physics::compose(
        &mut p,
        AnimationPublication {
            state: &mut state,
            completed: &completed,
            poses: &poses,
            signal_name: b"",
            tree_attributes: &[],
        },
        MotionGraphPublication {
            state: &mg,
            intents: IntentPublication::RetainPacket,
            attributes: &[],
        },
        &mut a,
        &mut e,
    )
    .err()
    .unwrap();
    assert_eq!(
        error,
        PublicationError::RuntimeService("scene service unavailable")
    );
    assert_eq!(e.calls, ["scene"]);
    assert_eq!(p.pose.local, [[[2.0; 4]; 4]]);
    assert_eq!(p.actor.external_physics.flags, 0xababcdef);
    assert_eq!(
        p.intents.entries().map(|e| e.name.0[0]).collect::<Vec<_>>(),
        [62, 1]
    );
    assert_ne!(a.flags1904 & ((1 << 25) | (1 << 28)), 0);
    assert!(!state.request_bit16);
}
#[test]
fn missing_pose_stops_after_reset_before_graph_or_runtime_phases() {
    let (completed, poses, mut state) = animation(false);
    let mut p = packet();
    let mut a = actor();
    let mut e = environment();
    let mut mg = graph();
    mg.gesture.gesture = 999;
    let error = setup_physics::compose(
        &mut p,
        AnimationPublication {
            state: &mut state,
            completed: &completed,
            poses: &poses,
            signal_name: b"",
            tree_attributes: &[],
        },
        MotionGraphPublication {
            state: &mg,
            intents: IntentPublication::RetainPacket,
            attributes: &[],
        },
        &mut a,
        &mut e,
    )
    .err()
    .unwrap();
    assert!(matches!(
        error,
        PublicationError::Pose(BufferError::UninitializedBone { .. })
    ));
    assert_eq!(p.pose.local, [RESET_POSE]);
    assert_eq!(p.motion_graph.gesture.gesture, 11);
    assert!(e.calls.is_empty());
    assert!(state.request_bit16);
}

#[test]
fn composed_mg_and_tree_attributes_reach_concrete_skeleton_steering_consumers() {
    use skate_core::animation::skeleton_input::{
        name::encode,
        scalar_attributes::{self, AnimationControlOutput, ScalarAttributeInputs},
    };
    let (completed, poses, mut state) = animation(true);
    let mut p = packet();
    let mut a = actor();
    let mut e = environment();
    let mg = graph();
    let graph_attributes = [
        MotionGraphAttribute {
            name: encode(b"Turn"),
            value: 0.25,
        },
        MotionGraphAttribute {
            name: encode(b"PushContact"),
            value: 0.0,
        },
    ];
    let tree_attributes = [
        MotionGraphAttribute {
            name: encode(b"Turn"),
            value: -0.75,
        }
        .to_animation(),
        MotionGraphAttribute {
            name: encode(b"Spin"),
            value: 0.5,
        }
        .to_animation(),
    ];
    let binding = setup_physics::compose(
        &mut p,
        AnimationPublication {
            state: &mut state,
            completed: &completed,
            poses: &poses,
            signal_name: b"signup",
            tree_attributes: &tree_attributes,
        },
        MotionGraphPublication {
            state: &mg,
            intents: IntentPublication::RetainPacket,
            attributes: &graph_attributes,
        },
        &mut a,
        &mut e,
    )
    .unwrap();
    let mut processed = ScalarAttributeInputs {
        flags2468: 0,
        flags2472: 0,
        flags2476: 0,
        flags2484: 0,
        flags2488: 0,
        board_adjust: 0,
        balance: 0.0,
        spin: 0.0,
        body_spin: 0.0,
        brake: 0.0,
        turn: 0.0,
        turn_scale: 1.0,
        magnitude_scale: 1.0,
        animation_end_com: [0.0; 4],
        animation_translation: [0.0; 4],
        animation_time: 0.0,
        animation_physics_blend_seconds: 0.0,
        cadence_end_percent: 0.0,
        raw_turn: 0.0,
        hard_turn: 0.0,
        slide: 0.0,
    };
    let mut animation_output = AnimationControlOutput {
        grind_name: AttributeName([0; 5]),
        flags: 0,
    };
    scalar_attributes::dispatch_supported_packet(&binding, &mut processed, &mut animation_output)
        .unwrap();
    assert_eq!(processed.turn, 0.75); // Later tree value wins, then native negation.
    assert_eq!(processed.spin, -0.5);
    assert_eq!(processed.flags2488, 1 << 29); // Scalar marker, no strength calculation.
    assert_eq!(binding.packet().attributes.entries().len(), 4);
    assert_eq!(
        binding.packet().attributes.entries()[0].payload.0[0],
        Some(0.25f32.to_bits())
    );
}
