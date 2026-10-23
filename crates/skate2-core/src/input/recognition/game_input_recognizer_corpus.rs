//! Original assets and same supplied Vault through command-owned recognition.
//! Controlled samples/physics/sync and retained source cvars; no full Actor/native frame.
use crate::{
    actor_construction::{ActorChaseFormationOwner, ActorConstruction, ActorPrefixPhase},
    actor_graph_construction::ActorGraphIntents,
    attribute_runtime::AttributeRuntimeVault,
    game_input_manager::{GameInputCommandManager, GameInputSyncLevel},
    game_input_player::PlayerInputData,
    input_map::SelectedInputMap,
    input_pad_cache::PublishedInputPadSample,
    input_pattern_matcher::InputPatternStick,
    reckoning_numeric::HostReckoningNumeric,
};
use std::sync::{Arc, Mutex};
#[path = "../game_input_loaded_action_corpus.rs"]
pub(crate) mod loaded_action;
// Command dispatch uses the selected offline synchronization level.
struct ControlledCommandSync;
impl GameInputSyncLevel for ControlledCommandSync {
    type Error = std::convert::Infallible;
    fn sync_level(&mut self) -> Result<i32, Self::Error> {
        Ok(2)
    }
}
// Controlled pre-physics selection for this input corpus, not Actor physical admission.
struct DiagnosticPhysics;
impl crate::actor_construction::input_generation::ActorInputPhysics for DiagnosticPhysics {
    type Error = &'static str;
    fn physics_state(&mut self) -> Result<u32, Self::Error> {
        Ok(0)
    }
    fn effective_anim_z(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        Ok(crate::skeleton_root_transforms::SkeletonRootVector::known(
            [0.0, 0.0, 1.0, 0.0],
        ))
    }
    fn is_wiping_out(&mut self) -> Result<bool, Self::Error> {
        Err("no pending wipeout query expected")
    }
}

pub(crate) fn original_input_cvars() -> (
    crate::input_cvar_runtime::InputCvarManager,
    crate::input_cvar_runtime::InputCvarReferences,
) {
    let refs = crate::input_cvar_runtime::InputCvarReferences::new_static_initialized();
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let bytes = std::fs::read(root.join("data").join("startup.ini")).unwrap();
    manager.try_deserialize_startup(Some(&bytes)).unwrap();
    (manager, refs)
}
fn sample_from_point(point: [f32; 2]) -> PlayerInputData {
    let mut values = [0.0; 24];
    // Authored recognizer XY -> gameplay axes; UpdateRecognizer negates Y.
    for (value, positive, negative) in [(point[0], 20, 21), (-point[1], 22, 23)] {
        if value >= 0.0 {
            values[positive] = value;
        } else {
            values[negative] = -value;
        }
    }
    PlayerInputData {
        sample: PublishedInputPadSample { count: 24, values },
        allow_sensor: false,
        has_focus: true,
    }
}
fn step(
    manager: &mut GameInputCommandManager,
    gameplay: &mut crate::gameplay_input_update_manager::GameplayInputUpdateManager<
        crate::actor_construction::ActorIdentity,
    >,
    receiver: u32,
    point: [f32; 2],
    services: &mut ControlledCommandSync,
    runtime: &mut crate::game_input_recognizer_runtime_services::GameInputRuntimeServices<'_>,
    overlay: &crate::debug_overlay::DebugOverlaySlot,
) {
    let sample = sample_from_point(point);
    crate::game_input_frame_session::try_deliver_frame(
        manager,
        gameplay,
        crate::game_input_frame_session::GameInputFrameRequest {
            input: &sample,
            receiver_id: receiver,
            update_mask: 1,
            add_to_crc: true,
        },
        &mut HostReckoningNumeric,
        services,
        runtime,
        overlay,
    )
    .unwrap();
    assert_eq!(gameplay.route_mut().pending(), 0);
}
#[expect(
    clippy::too_many_lines,
    reason = "one retained original gesture through match, held and release"
)]
pub(crate) fn check(
    vault: &Arc<AttributeRuntimeVault>,
    world: &Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>,
    spawn: [f32; 16],
    cvars: crate::input_cvar_runtime::InputCvarReferences,
) -> (
    ActorConstruction,
    crate::actor_animation_mount_corpus::OriginalPhysicalAnimationInputs,
    loaded_action::LoadedActionInputSession,
) {
    let manager_authority = world.lock().unwrap().physical_manager_authority();
    let cvar_refs = cvars;
    let (animation_startup, mut animation_source) =
        crate::simulation_animation_startup::corpus::load_original_startup_with_cvars_and_source(
            &cvar_refs,
        );
    let faces = crate::actor_animation_mount_corpus::load_tutorial_face();
    let mounts = crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus();
    let input = mounts.read_path("data\\config\\input.cfg").unwrap();
    let map = Arc::new(
        SelectedInputMap::parse_with_input_events(std::str::from_utf8(&input).unwrap()).unwrap(),
    );
    let mut gameplay = crate::gameplay_input_update_manager::GameplayInputUpdateManager::new();
    gameplay
        .try_construct(|path| mounts.read_path(&path.replace('/', "\\")))
        .unwrap();
    let original = Arc::clone(&gameplay.loaded_patterns(InputPatternStick::Right)[0].patterns()[0]);
    let original = &original;
    assert_eq!(original.name(), "Ollie");
    assert_eq!(original.points().len(), 2);
    let start = original.points()[1];
    let end = original.points()[0];
    // Independent original layout read proves this is the supplied actual Vault,
    // not a fresh default-only provider. The matcher reads it on the wait step.
    let instance = vault.instance(0x7f08_6875_d621_c88c, InputPatternStick::Right.collection());
    let layout = instance
        .try_cached_layout(0x30)
        .unwrap()
        .expect("original right recognizer layout");
    let waiting = layout.bytes()[0x21];
    assert!(
        waiting > 0,
        "original recognizer must admit the one waiting step"
    );
    let mut manager = GameInputCommandManager::new();
    gameplay.try_reset(&map, vault, &mut manager).unwrap();
    let player = Arc::clone(gameplay.player(0).unwrap());
    let receiver_id = 10;
    let overlay = world.lock().unwrap().debug_overlay.clone();
    let mut runtime = crate::game_input_recognizer_runtime_services::GameInputRuntimeServices::new(
        &cvar_refs, vault, &overlay, None,
    );
    let mut services = ControlledCommandSync;
    let mut formation = ActorChaseFormationOwner::new(Arc::clone(vault));
    formation.try_bind_selected_reset().unwrap();
    let actor = ActorConstruction::new(
        Arc::clone(&player),
        manager_authority,
        1,
        crate::skeleton_root_transforms::SkeletonRootMatrix::known(spawn),
    )
    .unwrap();
    actor
        .try_construct_prefix(&formation, &mut HostReckoningNumeric)
        .unwrap();
    let authority = actor.authority_for_listener().unwrap();
    assert_eq!(authority.entity_key().manager(), manager_authority);
    assert!(Arc::ptr_eq(authority.entity_key().gameplay(), &player));
    let listener = actor.try_construct_listener(gameplay.route_mut()).unwrap();
    assert!(listener.lock().unwrap().registration().is_some());
    actor.try_construct_graphs().unwrap();
    assert!(Arc::ptr_eq(
        &actor.retained_listener().unwrap().unwrap(),
        &listener
    ));
    let fill_attributes =
        crate::action_graph_input_listener::fill::ActionInputFillAttributes::new(vault);
    let mut cvars = cvar_refs.fill_provider();
    let mut juice = crate::juice_trick_input::JuiceTrickInput::new_static_initialized();
    step(
        &mut manager,
        &mut gameplay,
        receiver_id,
        start,
        &mut services,
        &mut runtime,
        &overlay,
    );
    {
        // Actual map clear/Fill/event-clear path, with controlled physics state.
        // Live labels come from original startup.ini; no outer tick is claimed.
        actor
            .try_generate_action_intents(
                0.25,
                Some(&fill_attributes),
                &mut cvars,
                &mut HostReckoningNumeric,
                &mut juice,
                &mut DiagnosticPhysics,
            )
            .unwrap();
        let state = listener.lock().unwrap();
        let derived = state.derived().unwrap();
        assert_eq!(
            derived.previous,
            crate::controller_input::RawControllerInput::default()
        );
        // The original sparse command codec quantizes stick axes to signed
        // bytes before the listener sees them (Pack82a8c070/Unpack82a8c330).
        let quantized = [start[0], -start[1]].map(|value| {
            let integer = (value * 127.0).trunc();
            if integer == 0.0 {
                0.0
            } else {
                integer * f32::from_bits(0x3c01_0204)
            }
        });
        assert_eq!(
            derived.current.right_stick_x.to_bits(),
            quantized[0].to_bits()
        );
        assert_eq!(
            derived.current.right_stick_y.to_bits(),
            quantized[1].to_bits()
        );
        assert_eq!(
            derived.magnitude_held.map(f32::to_bits),
            [0, 0.25_f32.to_bits()]
        );
    }
    step(
        &mut manager,
        &mut gameplay,
        receiver_id,
        start,
        &mut services,
        &mut runtime,
        &overlay,
    );
    step(
        &mut manager,
        &mut gameplay,
        receiver_id,
        [0.0, 0.0],
        &mut services,
        &mut runtime,
        &overlay,
    );
    step(
        &mut manager,
        &mut gameplay,
        receiver_id,
        end,
        &mut services,
        &mut runtime,
        &overlay,
    );
    assert!(
        listener
            .lock()
            .unwrap()
            .events()
            .iter()
            .filter_map(|event| event.pattern())
            .any(|event| Arc::ptr_eq(&event.pattern, original)),
        "original Ollie must reach the same Actor during manager Update"
    );
    step(
        &mut manager,
        &mut gameplay,
        receiver_id,
        end,
        &mut services,
        &mut runtime,
        &overlay,
    );
    step(
        &mut manager,
        &mut gameplay,
        receiver_id,
        [0.0, 0.0],
        &mut services,
        &mut runtime,
        &overlay,
    );
    let state = listener.lock().unwrap();
    assert!(state.actor().is_same_issuance(&authority.entity_handle()));
    let event = state
        .events()
        .iter()
        .filter_map(|event| event.pattern())
        .find(|event| Arc::ptr_eq(&event.pattern, original))
        .unwrap();
    let speed = event.speed;
    assert!(
        !state.held_pending(),
        "Ollie does not arm a flip held candidate"
    );
    drop(state);
    actor
        .try_generate_action_intents(
            0.25,
            Some(&fill_attributes),
            &mut cvars,
            &mut HostReckoningNumeric,
            &mut juice,
            &mut DiagnosticPhysics,
        )
        .unwrap();
    {
        let prefix = actor.prefix().unwrap();
        assert_eq!(prefix.phase(), ActorPrefixPhase::GraphFacadesConstructed);
        let graphs = prefix.graphs().unwrap();
        let action = graphs.action().unwrap();
        let motion = graphs.motion().unwrap();
        for wrapper in [
            action.controller.as_ref().unwrap(),
            motion.controller.as_ref().unwrap(),
        ] {
            assert!(
                wrapper
                    .context()
                    .entity()
                    .unwrap()
                    .is_same_issuance(&authority.entity_handle())
            );
            assert!(wrapper.is_unloaded());
        }
        assert!(
            !action
                .controller
                .as_ref()
                .unwrap()
                .controller_identity()
                .is_same_issuance(motion.controller.as_ref().unwrap().controller_identity())
        );
        assert!(action.is_complete() && motion.is_complete());
        let intents = action.intents.as_ref().unwrap();
        for (name, bits) in [
            ("Trick", 1.0_f32.to_bits()),
            ("Ollie", 1.0_f32.to_bits()),
            ("GestureSpeed", speed.to_bits()),
        ] {
            assert_eq!(
                intents
                    .get(&ActorGraphIntents::native_name(name.as_bytes()))
                    .map(f32::to_bits),
                Some(bits)
            );
        }
        assert!(
            intents
                .get(&ActorGraphIntents::native_name(b"HoldPattern"))
                .is_none()
        );
        assert_eq!(motion.intents.as_ref().unwrap().len(), 0);
        assert_eq!(motion.filtered_intents.as_ref().unwrap().len(), 0);
        assert!(motion.state.as_ref().unwrap().attributes().is_empty());
    }
    // The actual generation call consumes the queued events after successful Fill.
    assert!(listener.lock().unwrap().events().is_empty());
    assert!(listener.lock().unwrap().registration().is_some());
    let animation = actor
        .try_construct_animation_prefix(
            world,
            vault,
            &mut AnimationSelection(animation_startup.managers.clone()),
        )
        .unwrap();
    assert_eq!(
        animation.lock().unwrap().inputs().face_name.as_deref(),
        Some(&b"tutorial_male"[..])
    );
    assert!(Arc::ptr_eq(
        &animation.lock().unwrap().inputs().physics_world,
        world
    ));
    assert!(matches!(actor.try_publish_animation_constructor(), Err(crate::actor_construction::animation_mount::ActorAnimationMountError::ConstructorIncomplete)));
    eprintln!(
        "original gameplay manager: nine native players, codec-fed Ollie/hold/release samples, supplied Vault waiting={waiting}, actual per-player recognizer/events then shared Flush -> SAME Actor listener and ActionGraph intents PASS; controlled command samples and schedule, not full frame"
    );
    crate::actor_animation_mount_corpus::finish(
        &actor,
        vault,
        &faces,
        &cvar_refs,
        &mut animation_source,
    );
    let physical_animation = crate::actor_animation_mount_corpus::initialize(
        &actor,
        &animation_startup,
        &cvar_refs,
        crate::actor_construction::input_generation::runtime::ActorInputRuntimeServices {
            attributes: fill_attributes,
            cvars,
            juice,
        },
    );
    let mut graph_owners =
        crate::actor_construction::graph_startup::ActorGraphStartupOwners::try_new_with_motion(
            Arc::clone(vault),
            cvar_refs.clone(),
        )
        .unwrap();
    graph_owners.try_enable_player(&actor, &cvar_refs).unwrap();
    assert!(actor.try_load_action_graph(&graph_owners).unwrap());
    assert!(actor.try_load_motion_graph(&graph_owners).unwrap());
    eprintln!(
        "SAME Actor registered Action/Motion interfaces -> actual startup enablement -> full generated constructor loads; graph callbacks and Tick remain pending"
    );
    (
        actor,
        physical_animation,
        loaded_action::LoadedActionInputSession {
            commands: manager,
            gameplay,
        },
    )
}

/// Selected diagnostic backend answers, not a reconstructed `PlayersBackEnd`.
struct AnimationSelection(crate::actor_face_playback::FaceAnimationManagers);
impl crate::actor_construction::animation_mount::ActorAnimationSelection for AnimationSelection {
    type Error = &'static str;
    fn local_player_index(&mut self) -> Result<u32, Self::Error> {
        Ok(0)
    }
    fn is_network_game(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn ghost_system_enabled(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn marquee_id(&mut self, _: u32) -> Result<u64, Self::Error> {
        Ok(crate::player::DEFAULT_MARQUEE_ID)
    }
    fn character_is_male(&mut self, _: u32) -> Result<bool, Self::Error> {
        Err("marquee branch must not query CAC sex")
    }
    fn command_manager(
        &mut self,
    ) -> Result<Arc<Mutex<crate::actor_face_playback::FaceCommandManager>>, Self::Error> {
        Ok(Arc::clone(&self.0.command))
    }
    fn playback_manager(
        &mut self,
    ) -> Result<Arc<Mutex<crate::actor_face_playback::FacePlaybackManager>>, Self::Error> {
        Ok(Arc::clone(&self.0.playback))
    }
    fn database_manager(
        &mut self,
    ) -> Result<Arc<Mutex<crate::actor_face_playback::FaceDatabaseManager>>, Self::Error> {
        Ok(Arc::clone(&self.0.database))
    }
}
