//! Three consecutive selected host frames on the same original mounted owners.
//! Test-only child of `physical_player_ground_startup_corpus`.
use crate::{
    actor_animation_mount_corpus::OriginalPhysicalAnimationInputs,
    actor_construction::{
        ActorConstruction,
        selected_tick::{
            SelectedPlayerTick, SelectedQueuedTickOutcome, SelectedTickInputDelivery,
            SelectedTickNumerics, SelectedTickOwners, SelectedTickRequest, SelectedTickStage,
        },
    },
    attribute_runtime::AttributeRuntimeVault,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    game_input_recognizer_corpus::loaded_action::LoadedActionInputSession,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::HostReckoningNumeric,
    selected_session_frame::SelectedSessionFrame,
    simulation_assembly_backend::constraint_session::{
        ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
        update5_managers::GrindManagerFillPolicy,
    },
};
use std::sync::{Arc, Mutex};

/// Call on a fresh fully published Actor BEFORE `check_published`; root's separate
/// startup mode must provide contact capacity256 and retain all old fixture modes.
#[expect(
    clippy::too_many_arguments,
    reason = "one three-frame original-owner corpus with explicit retained services"
)]
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    actor: &ActorConstruction,
    animation: &mut OriginalPhysicalAnimationInputs,
    input_session: &mut LoadedActionInputSession,
    cvars: &InputCvarReferences,
    vault: &Arc<AttributeRuntimeVault>,
    services: &crate::player::attributes_runtime::PlayerAttributeServices,
    presentation: &crate::actor_face_playback::presentation::startup::SelectedPresentationStartup,
    scene: Arc<crate::character::CharacterScene>,
    camera_residency: Option<super::grind_corpus::RetainedGrindCorpus>,
) {
    check_mode(
        world,
        world_owner,
        board,
        skeleton,
        actor,
        animation,
        input_session,
        cvars,
        vault,
        services,
        presentation,
        scene,
        camera_residency,
        false,
        None,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "same original retained fixture owners"
)]
pub(super) fn check_wipeout(
    world: &mut PhysicalAssemblyWorld,
    world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    actor: &ActorConstruction,
    animation: &mut OriginalPhysicalAnimationInputs,
    input_session: &mut LoadedActionInputSession,
    cvars: &InputCvarReferences,
    vault: &Arc<AttributeRuntimeVault>,
    services: &crate::player::attributes_runtime::PlayerAttributeServices,
    presentation: &crate::actor_face_playback::presentation::startup::SelectedPresentationStartup,
    scene: Arc<crate::character::CharacterScene>,
    camera_residency: Option<super::grind_corpus::RetainedGrindCorpus>,
) {
    check_mode(
        world,
        world_owner,
        board,
        skeleton,
        actor,
        animation,
        input_session,
        cvars,
        vault,
        services,
        presentation,
        scene,
        camera_residency,
        true,
        None,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "same mounted owners including actual retained startup selection"
)]
pub(super) fn check_queued(
    world: &mut PhysicalAssemblyWorld,
    world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    actor: &ActorConstruction,
    animation: &mut OriginalPhysicalAnimationInputs,
    input_session: &mut LoadedActionInputSession,
    cvars: &InputCvarReferences,
    vault: &Arc<AttributeRuntimeVault>,
    players: &crate::actor_construction::selected_startup::SelectedPlayerStartupOwners,
    camera_residency: Option<super::grind_corpus::RetainedGrindCorpus>,
    policy: super::super::queued_input::QueuedRemapPolicy,
) {
    // This direct corpus bypasses the selected facade's SkaterAnimShared
    // initialization. Reuse its retained Skater playback and attribute pool
    // in the native push-then-recovery construction order before any tick.
    let playback = players
        .animation
        .body
        .first()
        .expect("selected Skater playback");
    let attributes = players
        .animation
        .attributes
        .as_ref()
        .expect("selected animation attributes");
    let push = {
        let playback = playback.lock().unwrap();
        let mut attributes = attributes.lock().unwrap();
        crate::skater_push_shared_attributes::SkaterPushSharedAttributes::try_build(
            &playback,
            &mut attributes,
        )
        .unwrap()
    };
    let recovery = {
        let playback = playback.lock().unwrap();
        let mut attributes = attributes.lock().unwrap();
        crate::skater_wipeout_recovery_graph::SkaterWipeoutRecoveryGraph::try_build(
            &playback,
            &mut attributes,
        )
        .unwrap()
    };
    actor
        .try_install_push_shared_attributes(Arc::new(push))
        .unwrap();
    actor
        .try_install_wipeout_recovery_graph(Arc::new(recovery))
        .unwrap();
    check_mode(
        world,
        world_owner,
        board,
        skeleton,
        actor,
        animation,
        input_session,
        cvars,
        vault,
        &players.services,
        players
            .presentation
            .as_ref()
            .expect("same presentation startup"),
        Arc::clone(players.recipe_load.as_ref().unwrap().scene().unwrap()),
        camera_residency,
        false,
        Some((players, policy)),
    );
}

#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "same original tick loop with explicit controlled transition policy"
)]
fn check_mode(
    world: &mut PhysicalAssemblyWorld,
    world_owner: &Arc<Mutex<PhysicalAssemblyWorld>>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    actor: &ActorConstruction,
    animation: &mut OriginalPhysicalAnimationInputs,
    input_session: &mut LoadedActionInputSession,
    cvars: &InputCvarReferences,
    vault: &Arc<AttributeRuntimeVault>,
    services: &crate::player::attributes_runtime::PlayerAttributeServices,
    presentation: &crate::actor_face_playback::presentation::startup::SelectedPresentationStartup,
    scene: Arc<crate::character::CharacterScene>,
    camera_residency: Option<super::grind_corpus::RetainedGrindCorpus>,
    wipeout_probe: bool,
    queued_players: Option<(
        &crate::actor_construction::selected_startup::SelectedPlayerStartupOwners,
        super::super::queued_input::QueuedRemapPolicy,
    )>,
) {
    // Retain the original pattern owner and authored order; the separate
    // controlled command corpus established this same Kickflip/held sequence.
    let script = queued_players
        .as_ref()
        .is_some_and(|(_, policy)| {
            matches!(
                policy,
                super::super::queued_input::QueuedRemapPolicy::ScriptedKickflip
            )
        })
        .then(|| {
            let pattern = input_session.original_right_pattern("Kickflip");
            let points = pattern.points().iter().rev().copied().collect::<Vec<_>>();
            assert!(points.len() >= 2);
            // This fresh recognizer has no prior candidate to expire. Its
            // waiting byte limits dwell between pattern points, not neutral
            // ticks before the first point. The mounted neutral prefix reaches
            // Ground.IdleTurning before frame 6; two points beginning there
            // match while its Trick transition is still reachable.
            let start = 6;
            assert!(
                start + points.len() + 1 < 120,
                "authored Kickflip points/held/release exceed queued 120-tick budget"
            );
            (pattern, points, start)
        });
    let scripted_y = queued_players.as_ref().is_some_and(|(_, policy)| {
        matches!(
            policy,
            super::super::queued_input::QueuedRemapPolicy::ScriptedY
        )
    });
    let scripted_y_frame = 32;
    // Optional original-owner continuation after the first completed
    // OffBoard frame. Zero preserves the original boundary proof.
    let y_followup_frames = std::env::var("SKATE2_Y_FOLLOWUP_FRAMES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut first_y_offboard_frame = None;
    let mut scripted_y_seen = false;
    let mut scripted_y_boundary = false;
    // Post-Fill Action intents prove the authored pattern name, not the exact
    // pattern Arc or pre-Fill Held message identity; those require a separate
    // read-only observation between Flush and Actor Fill.
    let scripted_listener = script
        .as_ref()
        .map(|_| actor.retained_listener().unwrap().unwrap());
    let mut scripted_match_seen = false;
    let mut scripted_release_completed = false;
    let mut queued_input = queued_players.map(|(players, policy)| {
        super::super::queued_input::QueuedInputCorpus::new(players, policy)
    });
    let settings = &services.settings;
    let mut presentation_probe =
        crate::actor_face_playback::presentation::frame::corpus::SelectedFrameProbe::new(
            presentation,
            scene,
        );
    let owner = Arc::clone(actor.cached_physical_allocation().unwrap().owner());
    let overlay = world.debug_overlay.clone();
    actor
        .try_construct_physics_auxiliary()
        .expect("one-time physics auxiliaries");
    let mut tick = SelectedPlayerTick::new();
    let selected_camera = camera_residency.is_some();
    if selected_camera {
        // Explicit selected physics flags3 and submit flags3 corpus input.
        // This does not certify the omitted SimUpdate2 manager calls.
        tick.select_camera_subsequence(3);
    }
    let mut bail = wipeout_probe.then(super::super::wipeout_live::WipeoutProbe::default);
    if wipeout_probe {
        tick.request_transition_for_test(crate::simulation_assembly_backend::constraint_session::physical_state::ground_to_air::GroundDestination::Wipeout);
    }
    let grind_residency = camera_residency
        .unwrap_or_else(|| super::grind_corpus::mount(&world.grind_services, cvars));
    // Physics and camera now borrow the SAME actual normal-arena Environment
    // admission; do not reopen and separately publish only the fixture mesh.
    let ground = grind_residency.ground_pool();
    assert!(ground.published_volume_count() > 0);
    // Existing selected query constructor now consumes source-sampled capacities.
    // This test still does not certify the enclosing whole World.Init schedule.
    let query_config = crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig::try_sample_selected(cvars, world.island_flags()).unwrap();
    world
        .try_initialize_world_line_queries(query_config, false)
        .expect("one-time retained query managers");
    let layout = vault
        .physics(crate::skateboard_attributes::SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    world.backend_mut().set_gravity(std::array::from_fn(|axis| {
        f32::from_be_bytes(layout.raw_bytes[axis * 4..axis * 4 + 4].try_into().unwrap())
    }));
    let sample = crate::game_input_player::PlayerInputData {
        sample: crate::input_pad_cache::PublishedInputPadSample {
            count: 24,
            values: [0.0; 24],
        },
        allow_sensor: false,
        has_focus: true,
    };
    let mut recognizer =
        crate::game_input_recognizer_runtime_services::GameInputRuntimeServices::new(
            cvars, vault, &overlay, None,
        );
    let mut sync = Arc::clone(world.physical_world_services().unwrap().game_info());
    let mut feedback = world.try_live_contact_feedback(cvars).unwrap();
    let mut numeric = SelectedTickNumerics {
        input: HostReckoningNumeric,
        actor: HostReckoningNumeric,
        pre: HostGroundPreparationNumeric::default(),
        line: crate::collision_line_numeric::HostCollisionLineNumeric,
        vector: HostReckoningNumeric,
        rounding: crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        truck: crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
        collision: crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
    };
    // This selected High/LOD0 test must not request the unmounted clamp service.
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let dt = 1.0_f32 / 60.0;
    // Fixture budget includes ordinary Ground ticks before source selection.
    // The 120-tick queued budget is explicit harness policy; Y-only follow-up
    // extends its headroom without changing native dt/tuning or other modes.
    let frame_budget = if scripted_y {
        120usize.saturating_add(y_followup_frames)
    } else if queued_input.is_some() {
        120
    } else if wipeout_probe {
        8
    } else {
        3
    };
    for frame in 0..frame_budget {
        if scripted_y && frame == scripted_y_frame {
            let before_y = owner.physics_state().lock().unwrap().current();
            assert!(
                matches!(
                    before_y,
                    crate::physical_player_physics_state::CurrentPhysicalState::Ground
                        | crate::physical_player_physics_state::CurrentPhysicalState::GroundAnimation
                ),
                "raw Y must be injected after natural mounted ground settle; got {before_y:?}"
            );
        }
        let mut submitted = 0;
        let mut observer = |stage: SelectedTickStage, world: &PhysicalAssemblyWorld| {
            let label = match stage {
                SelectedTickStage::BeforeActor => "before Actor",
                SelectedTickStage::BeforePreSolve => "before pre-solve",
                SelectedTickStage::BeforeSolver => "before solver",
                SelectedTickStage::AfterSolver => "after solver",
            };
            check_board_numeric_state(world, board, frame, label);
            if let Some(probe) = bail.as_mut() {
                probe.observe(frame, stage, world, &owner);
            }
            match stage {
                SelectedTickStage::BeforeSolver => {
                    let trajectories = owner.trajectories().lock().unwrap();
                    let selector = trajectories.onboard.as_ref().unwrap();
                    submitted = if selector.just_started_query {
                        usize::try_from(selector.num_trajectories).unwrap()
                    } else {
                        0
                    };
                }
                SelectedTickStage::AfterSolver if submitted != 0 => {
                    let trajectories = owner.trajectories().lock().unwrap();
                    let selector = trajectories.onboard.as_ref().unwrap();
                    assert!(
                        selector.queries[..submitted]
                            .iter()
                            .all(|future| future.ready),
                        "frame {frame}: submitted futures must be harvested before the solver returns"
                    );
                    eprintln!(
                        "frame {frame}: {submitted} actual retained trajectory futures ready after first harvest"
                    );
                }
                _ => {}
            }
        };
        let request = SelectedTickRequest {
            input: crate::game_input_frame_session::GameInputFrameRequest {
                input: &sample, receiver_id: 10, update_mask: 1, add_to_crc: true,
            },
            dt, lod: 0, wait_for_bs_job: true,
            physics: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                ground_pool: ground, ground_group: 0, pipeline_culling: None,
                allow_multiple_writes: true, spy_flags: 4,
            },
            pre_grind: GroundPreparationGrindPolicy::ParkedByUser,
            post_grind: GrindManagerFillPolicy::ExcludeForFirstSkatingTest,
        };
        let result = if let Some(driver) = queued_input.as_mut() {
            if frame == 0 {
                // A real empty mailbox must not visit Actor or physics stages.
                // Existing mounted handles/owners are supplied, so eager owner
                // work cannot hide behind a fabricated no-op test collaborator.
                let mut callbacks = 0;
                let mut no_work_observer = |_: SelectedTickStage, _: &PhysicalAssemblyWorld| {
                    callbacks += 1;
                };
                let skipped = SelectedSessionFrame {
                    tick: &mut tick,
                    world_owner,
                    world,
                    board,
                    skeleton,
                    actor,
                    animation,
                    input: input_session,
                    cvars,
                    settings,
                    services,
                    presentation,
                    presentation_runtime: presentation_probe.runtime_mut(),
                    others: Vec::new(),
                    local: 0,
                    challenge_scoring: None,
                    living_world: None,
                    living_world_recipes: None,
                    ai: None,
                }
                .try_step_queued::<_, _, _, _, _, _, std::convert::Infallible>(
                    request.with_input(driver.tick_input()),
                    &mut numeric,
                    &mut sync,
                    &mut recognizer,
                    &mut feedback,
                    &mut clamp,
                    &mut HostReckoningNumeric,
                    Some(&mut no_work_observer),
                    None,
                )
                .unwrap_or_else(|error| panic!("empty queue admission: {error:?}"));
                assert!(
                    matches!(skipped,SelectedQueuedTickOutcome::NoCommands(ref report) if report.update_mask==0 && report.delivered==0 && report.missing_receivers==0)
                );
                assert_eq!(callbacks, 0);
                driver.assert_rng_index(0);
            }
            if let Some((_, points, start)) = &script {
                let point = if frame < *start {
                    [0.0, 0.0]
                } else if frame < *start + points.len() {
                    points[frame - *start]
                } else if frame == *start + points.len() {
                    *points.last().unwrap()
                } else {
                    [0.0, 0.0]
                };
                driver.set_scripted_point(point);
            }
            if scripted_y {
                driver.set_scripted_y(frame == scripted_y_frame);
            }
            driver.poll_and_enqueue(&sync);
            if scripted_y {
                driver.assert_scripted_y_mapped(frame == scripted_y_frame);
            }
            if let Some((pattern, points, start)) = &script {
                let point = if frame < *start {
                    [0.0, 0.0]
                } else if frame < *start + points.len() {
                    points[frame - *start]
                } else if frame == *start + points.len() {
                    *points.last().unwrap()
                } else {
                    [0.0, 0.0]
                };
                driver.assert_scripted_mapped_point(point, pattern.squared_tolerance());
            }
            presentation_probe.check_with(
                actor,
                services,
                presentation,
                cvars,
                false,
                |runtime, presentation_numeric, presentation_observer| {
                    SelectedSessionFrame {
                        tick: &mut tick,
                        world_owner,
                        world,
                        board,
                        skeleton,
                        actor,
                        animation,
                        input: input_session,
                        cvars,
                        settings,
                        services,
                        presentation,
                        presentation_runtime: runtime,
                        others: Vec::new(),
                        local: 0,
                        challenge_scoring: None,
                        living_world: None,
                        living_world_recipes: None,
                        ai: None,
                    }
                    .try_step_queued(
                        request.with_input(driver.tick_input()),
                        &mut numeric,
                        &mut sync,
                        &mut recognizer,
                        &mut feedback,
                        &mut clamp,
                        presentation_numeric,
                        Some(&mut observer),
                        Some(presentation_observer),
                    )
                    .map(|outcome| match outcome {
                        SelectedQueuedTickOutcome::Completed(completed) => *completed,
                        SelectedQueuedTickOutcome::NoCommands(_) => {
                            panic!("accepted offline batch must reach selected tick")
                        }
                    })
                    .map_err(|error| format!("{error:?}"))
                },
            )
        } else {
            tick.try_step(
                SelectedTickOwners {
                    world_owner,
                    world,
                    board,
                    skeleton,
                    actor,
                    animation,
                    others: Vec::new(),
                    local: 0,
                    challenge_scoring: None,
                    input: input_session,
                    cvars,
                    settings,
                    living_world: None,
                    living_world_recipes: None,
                    ai: None,
                },
                request,
                &mut numeric,
                &mut sync,
                &mut recognizer,
                &mut feedback,
                &mut clamp,
                Some(&mut observer),
            )
            .map_err(|error| format!("{error:?}"))
        };
        if scripted_y && frame >= scripted_y_frame {
            let prefix = actor.prefix().unwrap();
            let graphs = prefix.graphs().unwrap();
            let name = crate::actor_graph_construction::ActorGraphIntents::native_name;
            let action = graphs.action().unwrap();
            let action_controller = action
                .controller
                .as_ref()
                .unwrap()
                .generated_action()
                .unwrap();
            let action_locked = action_controller.lock().unwrap();
            let action_state = action_locked.current_state;
            let action_gate = [70_usize, 127, 96, 130].map(|index| {
                (
                    action_locked.condition_active[index / 32] & (1 << (index % 32)) != 0,
                    action_locked.condition_values[index],
                )
            });
            drop(action_locked);
            let motion = graphs.motion().unwrap();
            let motion_controller = motion
                .controller
                .as_ref()
                .unwrap()
                .generated_motion()
                .unwrap();
            let motion_controller = motion_controller.lock().unwrap();
            let motion_state = motion_controller.current_state;
            let motion_wipeout_gate = [2_usize, 3, 4, 5].map(|index| {
                (
                    motion_controller.condition_active[index / 32] & (1 << (index % 32)) != 0,
                    motion_controller.condition_results[index],
                )
            });
            drop(motion_controller);
            let action_new = action
                .intents
                .as_ref()
                .unwrap()
                .get(&name(b"NewToggleOffBoardState"));
            let motion_dismount = motion.intents.as_ref().unwrap().get(&name(b"OB_Dismount"));
            let motion_offboard = motion.intents.as_ref().unwrap().get(&name(b"OffBoard"));
            let physical_state = owner.physics_state().lock().unwrap().current();
            let anim_flags = owner.anim_out.lock().unwrap().as_ref().unwrap().flags_bits;
            let processed = owner.reckoning().lock().unwrap().processed().clone();
            let processed = processed.lock().unwrap();
            let processed_flags = (
                processed.known_flags_8c4(),
                processed.physics_state_fields().flags_8cc,
                processed.physics_state_fields().flags_8d0,
                processed.physics_state_fields().flags_8d4,
            );
            drop(processed);
            eprintln!(
                "raw Y frame={frame} preassert result={:?} Action={action_state} physical={physical_state:?} Motion={motion_state} OB_Dismount={motion_dismount:?} OffBoard={motion_offboard:?} AnimOut.flags={anim_flags:#x} Processed.flags={processed_flags:?} Motion c2/c3/c4/c5={motion_wipeout_gate:?} gate c70/c127/c96/c130={action_gate:?}",
                result.as_ref().err()
            );
            if frame == scripted_y_frame {
                assert_eq!(action_new.map(f32::to_bits), Some(1.0_f32.to_bits()));
                scripted_y_seen = true;
                if action_state != 8 {
                    let error = result
                        .as_ref()
                        .err()
                        .expect("Action8 absent requires source error");
                    // The first reached source boundary is Action Pre31's
                    // fourth, lazy IsGrabbingObject condition. No Motion or
                    // physical dismount can be claimed before it returns.
                    assert!(
                        error.contains("PendingCallback { index: 130, address: 2194255600 }"),
                        "unexpected Y boundary before Action8: {error}"
                    );
                    assert_eq!(
                        action_gate,
                        [
                            (true, Some(true)),
                            (true, Some(false)),
                            (true, Some(false)),
                            (false, None)
                        ]
                    );
                    assert_eq!(action_state, 94);
                    assert!(motion_dismount.is_none() && motion_offboard.is_none());
                    scripted_y_boundary = true;
                    break;
                }
                assert_eq!(action_state, 8, "raw Y must select TransToOffBoard8");
                assert_eq!(motion_dismount.map(f32::to_bits), Some(0.0_f32.to_bits()));
                assert_eq!(motion_offboard.map(f32::to_bits), Some(0.0_f32.to_bits()));
            }
            eprintln!(
                "raw Y frame={frame} Action={action_state} Motion={motion_state} OB_Dismount={motion_dismount:?} OffBoard={motion_offboard:?} AnimOut.flags={anim_flags:#x} physical={physical_state:?} error={:?}",
                result.as_ref().err()
            );
            if let Err(error) = result.as_ref() {
                assert!(
                    scripted_y_seen,
                    "pending boundary preceded selected Y evidence"
                );
                assert!(
                    error.contains("Pending") || error.contains("pending"),
                    "first selected Y boundary must be an explicit source-pending callback: {error}"
                );
                scripted_y_boundary = true;
                break;
            }
            if physical_state
                == crate::physical_player_physics_state::CurrentPhysicalState::OffBoard
            {
                first_y_offboard_frame.get_or_insert(frame);
            }
            if let Some(first) = first_y_offboard_frame {
                // Bound the follow-up from the first OffBoard frame even after
                // the source transitions to Wipeout and recovery states.
                if frame >= first.saturating_add(y_followup_frames) {
                    scripted_y_boundary = true;
                    break;
                }
            }
        }
        let completed = result.unwrap_or_else(|error| {
            let controller = actor
                .prefix()
                .unwrap()
                .graphs()
                .unwrap()
                .action()
                .unwrap()
                .controller
                .as_ref()
                .unwrap()
                .generated_action()
                .unwrap();
            let state = controller.lock().unwrap();
            panic!(
                "frame {frame} selected tick: {error}; Action state {} active {:?}",
                state.current_state, state.current_behaviours
            );
        });
        if selected_camera && frame < 3 {
            // Same mounted camera observer after the completed selected tick.
            // This probes typed current/previous W and retained render
            // binding/cvar selection; no diagnostic camera is substituted.
            let camera = world
                .local_skater_identity
                .as_ref()
                .unwrap()
                .try_camera_producer_projection()
                .unwrap_or_else(|error| panic!("frame {frame} producer snapshot: {error:?}"));
            assert_eq!(
                camera.camera_update_counter,
                u32::try_from(frame + 1).unwrap()
            );
            assert_eq!(
                camera.near_plane.to_bits(),
                cvars.try_camera_near_clip_distance().unwrap().to_bits()
            );
            assert_eq!(camera.far_plane.to_bits(), 7700.0_f32.to_bits());
            assert!(!camera.rendering_fov_override);
            eprintln!(
                "frame {frame} camera W bits current={:?} previous={:?}",
                (*camera.camera_matrix.columns()).map(|column| column.w.map(f32::to_bits)),
                (*camera.camera_previous_matrix.columns()).map(|column| column.w.map(f32::to_bits))
            );
        }
        match &completed.input {
            SelectedTickInputDelivery::Immediate(input) => assert!(matches!(
                input.dispatch,
                crate::game_input_manager::GameInputDispatch::Delivered
            )),
            SelectedTickInputDelivery::Queued {
                commands,
                flushed_events,
            } => {
                assert_eq!(commands.update_mask, 3);
                // Initial gained-focus kind2 precedes ordinary kind1; later frames
                // preserve focus and submit only kind1.
                assert_eq!(commands.delivered, if frame == 0 { 2 } else { 1 });
                assert_eq!(commands.missing_receivers, 0);
                eprintln!("queued input: {flushed_events} flushed events");
                queued_input
                    .as_ref()
                    .unwrap()
                    .assert_rng_index(u32::try_from(frame + 2).unwrap());
            }
        }
        assert_eq!(input_session.gameplay.route_mut().pending(), 0);
        if let Some((_, points, start)) = &script {
            let listener = actor.retained_listener().unwrap().unwrap();
            assert!(
                Arc::ptr_eq(&listener, scripted_listener.as_ref().unwrap()),
                "frame {frame} listener stage: queued tick changed the Actor listener"
            );
            assert!(
                Arc::ptr_eq(
                    listener.lock().unwrap().player(),
                    input_session.gameplay.player(0).unwrap()
                ),
                "frame {frame} listener stage: queued tick changed player 0"
            );
            let prefix = actor.prefix().unwrap();
            let graphs = prefix.graphs().unwrap();
            let action = graphs.action().unwrap();
            let intents = action.intents.as_ref().unwrap();
            let name = crate::actor_graph_construction::ActorGraphIntents::native_name;
            if (*start..*start + points.len()).contains(&frame)
                && intents.get(&name(b"Kickflip")).map(f32::to_bits) == Some(1.0_f32.to_bits())
            {
                assert_eq!(
                    intents.get(&name(b"Trick")).map(f32::to_bits),
                    Some(1.0_f32.to_bits()),
                    "frame {frame} Action stage: Kickflip requires real Trick intent"
                );
                let selected = action
                    .controller
                    .as_ref()
                    .unwrap()
                    .generated_action()
                    .unwrap();
                assert_eq!(
                    selected.lock().unwrap().current_state,
                    52,
                    "frame {frame} Action stage: matched Kickflip must select authored state"
                );
                scripted_match_seen = true;
            }
            if frame == *start + points.len() - 1 {
                assert!(
                    scripted_match_seen,
                    "frame {frame} Action stage: authored raw-pad Kickflip never matched"
                );
            }
            if frame == *start + points.len() {
                assert_eq!(
                    intents.get(&name(b"HoldPattern")).map(f32::to_bits),
                    Some(1.0_f32.to_bits()),
                    "frame {frame} Action stage: held endpoint did not reach this Actor"
                );
            }
            if frame == *start + points.len() + 1 {
                assert!(
                    intents.get(&name(b"HoldPattern")).is_none(),
                    "frame {frame} Action stage: raw-pad release retained HoldPattern"
                );
                let selected = action
                    .controller
                    .as_ref()
                    .unwrap()
                    .generated_action()
                    .unwrap();
                assert_eq!(
                    selected.lock().unwrap().current_state,
                    94,
                    "frame {frame} Action stage: release must return to neutral94"
                );
                scripted_release_completed = true;
            }
        }
        let before = completed.before.as_ref().expect("ground pre-solve completed");
        let hands = completed.hands.as_ref().expect("hands/face published");
        let after = completed.after.as_ref().expect("ground animation completed");
        assert!(Arc::ptr_eq(&before.update1.owner, &owner));
        assert!(Arc::ptr_eq(&before.update3.owner, &owner));
        assert_eq!(
            before.grind_policy,
            GroundPreparationGrindPolicy::ParkedByUser
        );
        assert_eq!(completed.report.processed_assemblies, 2);
        for pose in [
            &hands.left,
            &hands.right,
            &hands.face,
        ] {
            assert!(
                !pose.read().unwrap().1.is_empty(),
                "LOD0 must publish synchronized presentation inputs"
            );
        }
        assert!(Arc::ptr_eq(&after.owner, &owner));
        assert!(
            after
                .animation
                .matches_output(&owner.phys_out().lock().unwrap())
        );
        check_board_numeric_state(world, board, frame, "after selected tick");
        crate::actor_face_playback::presentation::write::corpus::check(
            actor,
            presentation.animation.offsets.as_ref().unwrap(),
        );
        if queued_input.is_none() {
            presentation_probe.check(
                actor,
                services,
                presentation,
                cvars,
                &mut animation.body_services,
            );
        }
        if let Some(probe) = bail.as_mut() {
            probe.after_tick(frame, &owner);
        }
        eprintln!(
            "selected consecutive frame {frame} completed production input -> Actor -> pre-solve -> solve -> post-solve; {:?}",
            completed.counts
        );
        if bail
            .as_ref()
            .is_some_and(super::super::wipeout_live::WipeoutProbe::is_complete)
        {
            break;
        }
    }
    if script.is_some() {
        assert!(
            scripted_match_seen && scripted_release_completed,
            "scripted Kickflip match and release must both complete through post-solve"
        );
    }
    if scripted_y {
        assert!(
            scripted_y_seen && scripted_y_boundary,
            "selected Y replay must prove input/Action/Motion intents and reach an explicit boundary"
        );
    }
    if let Some(probe) = bail.as_ref() {
        probe.finish();
    }
    grind_residency.finish(&world.grind_services, cvars);
}

fn check_board_numeric_state(
    world: &PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    frame: usize,
    stage: &str,
) {
    let assembly = world.board(board).unwrap();
    for part in 0..7 {
        let transform = assembly
            .try_live_part_transform_typed(world.backend(), part)
            .unwrap();
        let observation = assembly
            .try_part_contact_spy_observation(world.backend(), part)
            .unwrap();
        let velocity = world
            .backend()
            .collision_body_pool()
            .try_contact_spy_linear_velocity(observation)
            .unwrap();
        assert!(
            transform
                .columns()
                .iter()
                .all(|column| column.xyz.iter().all(|lane| lane.is_finite()))
                && velocity[..3].iter().all(|lane| lane.is_finite()),
            "frame {frame} {stage} board part {part}: transform {transform:?}, velocity {velocity:?}"
        );
    }
}
