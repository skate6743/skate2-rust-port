//! Headless executable-embedded player over original Skate 2 world and rig owners.
use super::Platform as DisabledJuicePlatform;
use crate::{
    actor_face_playback::presentation::frame::{
        RootPositionWTransport,
        selected_runtime::SelectedPresentationPolicy,
    },
    collection_resident_streamer::CollectionId,
    game::{WorldStreamPoll, WorldStreamSpec},
    gesture_trick_mapping::MappingAllocation,
    input_pad::device::tests::Platform as PadPlatform,
    juice_trick_input::boot::JuiceHostOption,
    player::{DEFAULT_MARQUEE_ID, PlayerLoadPhase},
    reckoning_numeric::HostReckoningNumeric,
    selected_session_boot_owner::{ControlledPlayerPolicy, SelectedSession},
    simulation_environment_unlock_router::EnvironmentUnlockSeed,
    skateboard_attributes::SkateboardAttributes,
    world_stream_catalog::WorldStreamCatalog,
    world_stream_runtime::WorldStreamEffects,
};
use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Default)]
struct StreamCounts {
    high: usize,
    finalized: usize,
}
struct StreamObservations(Arc<Mutex<StreamCounts>>);
impl WorldStreamEffects for StreamObservations {
    fn add_high_collection(&mut self, _id: u64, _x: u32, _z: u32) {
        self.0.lock().unwrap().high += 1;
    }
    fn finalize_world_bounds(&mut self) {
        self.0.lock().unwrap().finalized += 1;
    }
}
#[expect(
    clippy::unnecessary_wraps,
    reason = "the selected thread provider's fallible callback signature is explicit"
)]
fn selected_thread() -> Result<u64, Infallible> {
    Ok(1)
}

#[test]
#[ignore = "requires original Skate 2 ROM; Skate 3 data stays in this test EXE"]
fn embedded_player_owns_actual_skate2_player_bodies() {
    // The PC game already gives its simulation worker 16 MiB. Match that
    // actual host boundary instead of Rust's smaller default test-worker stack.
    std::thread::Builder::new()
        .name("embedded-player-corpus".into())
        .stack_size(16 << 20)
        .spawn(|| run_embedded_player_corpus(false, false, None))
        .unwrap()
        .join()
        .unwrap();
}

#[expect(
    clippy::too_many_lines,
    reason = "one original-data proof retains all boot, stream, player and presentation owners"
)]
fn run_embedded_player_corpus(live: bool, populated: bool, trick_probe: Option<PlayerProbe>) {
    let root = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("original game root"));
    let mounts =
        Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
    let mut session =
        SelectedSession::<fn() -> Result<u64, Infallible>, u32>::new(Arc::clone(&mounts), &root);
    let mut pad = PadPlatform::new();
    let mut juice_platform = DisabledJuicePlatform::new();
    juice_platform.initialized = false;
    let juice_ini = match mounts.read_path("data\\juice\\Juice_m.ini") {
        Ok(bytes) => Some(bytes),
        Err(skate2_formats::mount::MountError::EntryNotFound(_)) => None,
        Err(error) => panic!("original optional Juice config: {error}"),
    };
    session
        .try_boot_prefix(
            &mut pad,
            &mut juice_platform,
            juice_ini.as_deref(),
            JuiceHostOption::DisableBeforeStart,
        )
        .unwrap();
    let assets = session.assets.as_ref().expect("one asset owner");
    assert!(Arc::ptr_eq(&assets.mounts, &mounts));
    let spawn = probe_spawn(assets.spawn,trick_probe);
    let vault = Arc::clone(&assets.vault);
    let cvars = assets.cvars.clone();
    let layout = vault
        .physics(SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    let gravity = std::array::from_fn(|axis| {
        f32::from_be_bytes(layout.raw_bytes[axis * 4..axis * 4 + 4].try_into().unwrap())
    });
    session
        .world_handle()
        .unwrap()
        .lock()
        .unwrap()
        .backend_mut()
        .set_gravity(gravity);
    session
        .try_continue_controlled_player(
            ControlledPlayerPolicy::HostSpawnWhileRegistrationPending,
            spawn,
            DEFAULT_MARQUEE_ID,
            1,
            selected_thread,
        )
        .unwrap();
    let player = session.player.as_ref().unwrap();
    assert!(matches!(
        player.selected.owners.loader.phase(),
        PlayerLoadPhase::Registering { .. }
    ));
    session
        .try_continue_physical(
            &mut |_, _, _| Ok::<_, Infallible>(MappingAllocation::Host),
            &mut HostReckoningNumeric,
            true,
            &mut || panic!("native spawn orthogonality assertion"),
            &mut || panic!("original hat0"),
        )
        .unwrap();
    session
        .try_continue_controlled_presentation::<Infallible>()
        .unwrap();

    let counts = Arc::new(Mutex::new(StreamCounts::default()));
    // Use the same authored-location -> Sim high-detail XSM lookup as the PC
    // frontend, so this fixture starts with the player's own terrain tile.
    let filename = skate2_formats::stream::high_detail_collection_filename(
        "Sim",
        [spawn[12], spawn[13], spawn[14]],
    );
    let map = skate2_formats::stream::CollectionMap::parse(
        &mounts
            .read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.xsm")
            .unwrap(),
    )
    .unwrap();
    let collection = map
        .collections
        .iter()
        .find(|entry| entry.filename == filename)
        .unwrap()
        .id;
    session
        .try_select_terrain(
            WorldStreamCatalog::new(),
            WorldStreamSpec::derive("BAM", None).unwrap(),
            StreamObservations(Arc::clone(&counts)),
            // Empty contexts/zero lock are the original constructor state in
            // the terrain corpus; this is explicit test policy here.
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 0,
                contexts_in_native_iteration_order: Vec::new(),
            },
            CollectionId(collection),
            false,
            0,
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        assert!(Instant::now() < deadline, "BAM stream timed out");
        match session.terrain.as_mut().unwrap().try_poll_stream().unwrap() {
            WorldStreamPoll::Pending => std::thread::yield_now(),
            WorldStreamPoll::CompletedThisPoll => break,
        }
    }
    assert!(counts.lock().unwrap().high > 0);
    assert_eq!(counts.lock().unwrap().finalized, 1);
    session.terrain.as_mut().unwrap().try_request().unwrap();
    loop {
        assert!(Instant::now() < deadline, "normal resident timed out");
        if session.terrain.as_mut().unwrap().try_poll().unwrap() {
            break;
        }
        std::thread::yield_now();
    }
    let world_handle = Arc::clone(session.world_handle().unwrap());
    // Verify support in the tile selected for the original frontend spawn.
    let at = {
        let world = world_handle.lock().unwrap();
        let pair = &session.physical.as_ref().unwrap().pair;
        let deck = world
            .board(&pair.board)
            .unwrap()
            .try_live_part_transform_host(world.backend(), 6)
            .unwrap();
        [deck[12], deck[13], deck[14]]
    };
    assert!(
        session
            .terrain
            .as_ref()
            .unwrap()
            .ground_pool_for(&world_handle)
            .unwrap()
            .published_volume_count()
            > 0
    );

    let ground = session
        .terrain
        .as_ref()
        .unwrap()
        .ground_pool_for(&world_handle)
        .unwrap();
    let support = ground
        .trajectory_sweep_sphere_segment(
            [at[0], at[1] + 5., at[2]],
            [at[0], at[1] - 20., at[2]],
            0.,
        )
        .unwrap();
    assert!(
        support.is_some(),
        "actual player spawn has no resident ground: {at:?}"
    );
    eprintln!("spawn={at:?} original ground={support:?}");
    if live {
        if populated {
            let mut bank = crate::selected_living_world_dmo::SelectedLivingWorldDmo::try_new(
                Arc::clone(&mounts),
                &cvars,
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(120);
            while !bank.try_poll().unwrap() {
                assert!(
                    Instant::now() < deadline,
                    "original main DMO bank timed out"
                );
                std::thread::yield_now();
            }
            bank.try_register(&mut world_handle.lock().unwrap(), &vault, &cvars)
                .unwrap();
            session.living_world_dmo = Some(bank);
        }
        run_live_player(&mut session, &mut pad, populated, trick_probe);
        return;
    }
    let physical = session.physical.as_ref().unwrap();
    let board = &physical.pair.board;
    let skeleton = &physical.pair.skeleton;
    let mut world = world_handle.lock().unwrap();
    let mut player = crate::skate3_bridge::player::Skate3Player::load(
        &mut world, ground, board, skeleton, "normal",
        None,
    )
    .expect("native player over original S2 owners");
    assert!(
        player.grind_primitive_count() > 0,
        "real Skate 2 spline sources were not bound to Skate 3"
    );
    let before = player.output().tick;
    eprintln!(
        "native initial tick={} state={} deck={:?} root={:?}",
        player.output().tick,
        player.output().state,
        player.output().board_transform[3],
        player.output().root_transform[3]
    );
    for _ in 0..3 {
        player.advance(&mut world, ground, 1. / 60., None).unwrap();
    }
    assert!(player.output().tick > before);
    //A running source player must wait through a terrain publication gap,
    //without replaying scoring/reset events or manufacturing replacement ground.
    let pending = crate::static_ground_pool::StaticGroundPool::try_new(
        crate::static_ground_pool::STATIC_GROUND_NATIVE_CAPACITY).unwrap();
    let paused=player.output().clone();
    for _ in 0..3 {
        player.advance(&mut world,&pending,1./60.,None).unwrap();
        assert!(!player.advanced_this_tick());
        assert_eq!(player.output().tick,paused.tick);
        assert_eq!(player.output().root_transform,paused.root_transform);
        assert_eq!(player.output().board_transform,paused.board_transform);
        assert!(player.completed_teleport().is_none());
    }
    player.advance(&mut world,ground,1./60.,None).unwrap();
    assert!(player.advanced_this_tick());
    assert_eq!(player.output().tick,paused.tick+1,"terrain recovery must not catch up skipped ticks");
    assert!(
        player
            .output()
            .world_bones
            .iter()
            .flatten()
            .flatten()
            .all(|v| v.is_finite())
    );
    let neutral = crate::input_pad::XboxPadSample {
        buttons: 0,
        left_trigger: 0,
        right_trigger: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    };
    for (camera_type, expected_shot) in [(1, "bl_high_chase"), (0, "bl_chase"), (1, "bl_high_chase")] {
        player.set_camera_type(camera_type);
        for _ in 0..90 {
            player.advance(&mut world, ground, 1. / 60., Some(neutral)).unwrap();
        }
        assert_eq!(player.camera_shot(), expected_shot, "original stock camera branch");
    }
    eprintln!(
        "native before Y tick={} state={} category={} wipeout={} deck={:?} root={:?} v={:?}",
        player.output().tick,
        player.output().state,
        player.output().category,
        player.output().wiping_out,
        player.output().board_transform[3],
        player.output().root_transform[3],
        player.output().board_velocity
    );
    player
        .advance(
            &mut world,
            ground,
            1. / 60.,
            Some(crate::input_pad::XboxPadSample {
                buttons: 1 << 15,
                ..neutral
            }),
        )
        .unwrap();
    let mut offboard = player.output().offboard;
    for _ in 0..90 {
        player
            .advance(&mut world, ground, 1. / 60., Some(neutral))
            .unwrap();
        offboard |= player.output().offboard;
    }
    assert!(
        offboard,
        "raw Y did not reach native offboard in the actual Skate 2 world"
    );
    let mirrored = world
        .board(board)
        .unwrap()
        .try_live_part_transform_host(world.backend(), 6)
        .unwrap();
    let native = crate::skate3_bridge::player::flatten(player.output().board_transform);
    for axis in [12, 13, 14] {
        assert!((mirrored[axis] - native[axis]).abs() < 0.0001);
    }
    player.release(&mut world).unwrap();
}

/// Read-only comparison with the actual GPU skin formula and original terrain.
/// This diagnoses alignment without changing source physics or moving the mesh.
fn audit_rendered_character(
    palette: &crate::character::palette::CharacterPalette,
    output: &crate::skate3_native::FrameOutput,
    ground: &crate::static_ground_pool::StaticGroundPool,
) {
    use skate2_formats::render::{
        NativeVertexAttributeValues as Values, VertexSemantic as Semantic,
    };
    for (name, bone) in output.bone_names.iter().zip(&output.world_bones) {
        if [
            "LEFTFOOT",
            "RIGHTFOOT",
            "LEFTTOEBASE",
            "RIGHTTOEBASE",
            "HIPS",
            "HEAD",
            "RIGHTHAND",
        ]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
        {
            eprintln!(
                "render alignment source {name}: position={:?}",
                &bone[3][..3]
            );
        }
    }
    for (part_index, part) in palette.skinning().parts.iter().enumerate() {
        let component = &palette.scene().components[part.component_index];
        let asset = &component.models[part.model_index].lods[part.lod_index];
        let model_index = asset
            .arena
            .instances
            .iter()
            .find_map(|i| i.render_model)
            .unwrap();
        for (mesh_index, mesh) in asset.arena.models[model_index].meshes.iter().enumerate() {
            let geometry = &asset.arena.meshes[mesh.mesh_index].geometry;
            let bytes = |semantic| {
                geometry.attributes.iter().find_map(|a| {
                    if a.semantic != semantic {
                        return None;
                    }
                    if let Values::UnsignedByte4(v) = &a.values {
                        Some(v.as_slice())
                    } else {
                        None
                    }
                })
            };
            let (Some(weights), Some(indices)) = (
                bytes(Semantic::BlendWeight(0)),
                bytes(Semantic::BlendIndices(0)),
            ) else {
                continue;
            };
            let rows = palette.mesh_rows(part_index, mesh_index).unwrap().rows;
            let mut minimum = [f32::INFINITY; 3];
            let mut maximum = [f32::NEG_INFINITY; 3];
            let mut sole = [0.; 3];
            for ((p, w), indices) in geometry.positions.iter().zip(weights).zip(indices) {
                let mut w = w.map(|v| f32::from(v) / 255.);
                w[3] = 1. - (w[0] + w[1] + w[2]);
                let p = [p[0] * 2., p[1] * 2. + 0.8, p[2] * 2., 1.];
                let point: [f32; 3] = std::array::from_fn(|axis| {
                    (0..4)
                        .map(|influence| {
                            let row = rows[usize::from(indices[influence]) * 3 + axis];
                            w[influence] * (0..4).map(|i| p[i] * row[i]).sum::<f32>()
                        })
                        .sum()
                });
                if point[1] < minimum[1] {
                    sole = point;
                }
                for axis in 0..3 {
                    minimum[axis] = minimum[axis].min(point[axis]);
                    maximum[axis] = maximum[axis].max(point[axis]);
                }
            }
            let floor = ground
                .trajectory_sweep_sphere_segment(
                    [sole[0], sole[1] + 5., sole[2]],
                    [sole[0], sole[1] - 5., sole[2]],
                    0.,
                )
                .unwrap();
            eprintln!(
                "render alignment mesh {} part={part_index} mesh={mesh_index}: bounds={minimum:?}..{maximum:?} sole={sole:?} floor={:?}",
                component.name,
                floor.map(|h| h.position[1])
            );
        }
    }
}

#[test]
#[ignore = "requires original Skate 2 ROM; runs the queued simulation headlessly"]
fn skate3_selected_frame_drives_camera_pose_and_raw_offboard_input() {
    std::thread::Builder::new()
        .name("skate3-selected-frame".into())
        .stack_size(16 << 20)
        .spawn(|| run_embedded_player_corpus(true, false, None))
        .unwrap()
        .join()
        .unwrap();
}

#[test]
#[ignore = "requires original S2 road and embedded S3 player; actual bail camera controls"]
fn skate3_marker_offboard_return_after_bail_keeps_world_position() {
    std::thread::Builder::new().name("marker-after-bail".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::MarkerReturnAfterBail)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original S2 road and embedded S3 player; actual bail camera controls"]
fn skate3_bail_uses_native_camera_relative_left_stick() {
    std::thread::Builder::new().name("source-bail-controls".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::BailControls)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original S2 road and embedded S3 player; actual left-stick plant release"]
fn skate3_footplant_stick_steers_release_and_back_pops_higher() {
    std::thread::Builder::new().name("source-footplant-steering".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::FootplantSteering)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 ROM; checks the native source-player admission boundary"]
fn skate3_player_waits_for_resident_terrain_before_admission() {
    std::thread::Builder::new().name("source-terrain-admission".into()).stack_size(16 << 20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::TerrainAdmission)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 ROM and its original main DMO bank"]
fn skate3_selected_frame_with_main_dmo_bank_keeps_original_world_running() {
    std::thread::Builder::new()
        .name("skate3-populated-world".into())
        .stack_size(16 << 20)
        .spawn(|| run_embedded_player_corpus(true, true, None))
        .unwrap()
        .join()
        .unwrap();
}

#[test]
#[ignore = "requires original Skate 2 road; uses embedded source movement assets"]
fn skate3_darkcatch_uses_authored_animation_and_original_hud() {
    std::thread::Builder::new().name("source-darkcatch".into()).stack_size(16 << 20)
        .spawn(|| { for foot in [0,1] { run_embedded_player_corpus(true, false, Some(PlayerProbe::Darkcatch(foot))); } })
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original S2 road; full native manual-release score and HUD publications"]
fn skate3_manual_hold_and_release_preserves_finished_score() {
    std::thread::Builder::new().name("manual-banking".into()).stack_size(16 << 20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::ManualBanking)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original S2 road; actual trigger-held grabs and completed score/mission frames"]
fn skate3_grabs_bank_on_landing_in_native_hud_and_mission_facts() {
    std::thread::Builder::new().name("grab-banking".into()).stack_size(16 << 20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::GrabBanking)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original S2 road and embedded S3 controller; repeated native actor teleports"]
fn skate3_repeated_host_spawns_keep_profile_requested_foot_board_and_heading() {
    std::thread::Builder::new().name("spawn-stance".into()).stack_size(16 << 20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::TeleportStance)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 ROM; Skate 3 data stays in this test EXE"]
fn skate3_offboard_left_stick_circle_preserves_motion_and_arm_targets() {
    std::thread::Builder::new().name("offboard-circle".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::OffboardCircle)))
        .unwrap().join().unwrap();
}

#[derive(Clone, Copy)]
enum PlayerProbe { OnlineResetSupersession, OffboardMarkerMovement, OffboardMarkerDrift(bool), GestureCatalogs, MarkerBanked, MarkerPushAway, MarkerPushImmediate, MarkerRelocation, OffboardCircle, SmallOffboardCircle(bool), MarkerReturnAfterBail, BailControls, FootplantSteering, GrabBanking, TeleportStance, ManualBanking, TerrainAdmission, Darkcatch(u32), Footplant, Boneless }

#[test]
#[ignore = "requires original S2 road; source ground-grab boneless/fastplant state602"]
fn skate3_ground_boneless_and_fastplants_use_authored_takeoff() {
    std::thread::Builder::new().name("source-boneless".into()).stack_size(16 << 20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::Boneless)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 road; source one-foot air footplants"]
fn skate3_air_footplants_use_authored_pose_and_native_foot_ik() {
    std::thread::Builder::new().name("source-footplant".into()).stack_size(16 << 20)
        .spawn(|| run_embedded_player_corpus(true,false,Some(PlayerProbe::Footplant)))
        .unwrap().join().unwrap();
}

fn run_live_player(
    session: &mut SelectedSession<fn() -> Result<u64, Infallible>, u32>,
    pad: &mut PadPlatform,
    populated: bool,
    trick_probe: Option<PlayerProbe>,
) {
    use crate::{
        actor_construction::selected_tick::{
            SelectedQueuedTickOutcome, SelectedTickNumerics, SelectedTickRequest,
        },
        game::CommonBackEndSeed,
        native_skater_random::{NativeRandomGate, SimulationCommandRandom},
        selected_sim_command_queue::{SelectedSimCommandQueue, SimCommandCounters},
        simulation_assembly_backend::constraint_session::{
            ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
            skateboard_ground::{SkateboardGroundFrameInput, SkateboardGroundFrameSchedule},
            update5_managers::GrindManagerFillPolicy,
        },
        ui::gameplay::{Difficulty, GameplaySettings},
    };
    // Match the facade's real SkaterAnimShared boot, including the push/recovery
    // owners used by the original character/gesture graph. This direct fixture
    // does not pass through SelectedLiveSession's initialization stages.
    let primary = session.player.as_ref().unwrap();
    let animation = &primary.selected.owners.animation;
    let playback = animation.body.first().unwrap().lock().unwrap();
    let mut attributes = animation.attributes.as_ref().unwrap().lock().unwrap();
    let push = Arc::new(crate::skater_push_shared_attributes::SkaterPushSharedAttributes::try_build(&playback, &mut attributes).unwrap());
    let recovery = Arc::new(crate::skater_wipeout_recovery_graph::SkaterWipeoutRecoveryGraph::try_build(&playback, &mut attributes).unwrap());
    let actor = primary.selected.actor.as_ref().unwrap();
    actor.try_install_push_shared_attributes(Arc::clone(&push)).unwrap();
    actor.try_install_wipeout_recovery_graph(Arc::clone(&recovery)).unwrap();
    drop(attributes);
    drop(playback);
    session.push_shared_attributes = Some(push);
    session.wipeout_recovery_graph = Some(recovery);
    session
        .try_admit_selected_camera_and_roster(CommonBackEndSeed::debug_reference().camera_set)
        .unwrap();
    let queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    let random =
        SimulationCommandRandom::construct_field(NativeRandomGate::from_static_initializer());
    session
        .try_bind_selected_input(queue, random)
        .unwrap_or_else(|_| panic!("production input bind"));
    session
        .try_prepare_selected_queued_frame(
            SelectedPresentationPolicy {
                lod: 0,
                submission_capacity: 65536,
                gather_capacity: 65536,
                provider: 0,
                root_w: RootPositionWTransport::ReadSkaterOpaque { bits: 0 },
            },
            3,
        )
        .unwrap();
    let mut test_spawn = probe_spawn(session.assets.as_ref().unwrap().spawn,trick_probe);
    if populated {
        let spawn = session.assets.as_ref().unwrap().spawn;
        // Match CharacterPrepareState's actual exit, including the initial
        // census and GameBackEnd.InGame publication. Merely registering the
        // main bank leaves an empty loading-world fixture.
        session
            .try_with_selected_queued_frame(|mut frame, ground, queued| {
                let local = frame.world.local_skater_identity.clone().unwrap();
                let debug_spawn = frame.services.vault.instance(0xff19_68a5_c467_bcf9, 0xf97b_9086_caa5_0c53)
                    .try_cached_layout_field(121, 1).unwrap().map(|field| field.bytes().to_vec());
                eprintln!("original census gates: spawn={} ghost={} network={} debug_spawn={debug_spawn:?}",
                    frame.cvars.try_enable_spawn().unwrap(), local.backend().try_ghost_enabled().unwrap(),
                    local.backend().try_is_network_game().unwrap());
                {
                    let owner = frame.world.grind_services.living_world();
                    let living = owner.lock().unwrap();
                    let locators = living.dmo_locators.as_deref().unwrap();
                    let nearby = locators.passive_locators_within_range(
                        [spawn[12], spawn[14]], 10000., 256,
                        &mut crate::living_world_grind_services::dmo_objects::FrameLocatorServices,
                    ).unwrap();
                    eprintln!("original DMO tuning: spawn_dynamic={} maximum={}",
                        frame.cvars.try_enable_spawn_dynamic_objects().unwrap(),
                        frame.cvars.try_max_dynamic_objects().unwrap());
                    for (id, distance) in nearby.iter().take(8) {
                        let locator = locators.locator(*id).unwrap();
                        eprintln!("actual nearby locator {id:016x}: distance={distance} position={:?} flags={:#x} user={} model_registered={}",
                            &locator.transform[12..15], locator.flags, locator.user_object_id,
                            living.dmo_models.model(locator.model_id).is_some());
                    }
                    // This partial-session fixture streams one tile; the PC
                    // frontend streams additional nearby locator/RAIN chunks.
                    // Start near an actual supported object of this tile rather
                    // than counting bank registration as a populated-world test.
                    let (id, support) = nearby.iter().find_map(|(id, _)| {
                        let locator = locators.locator(*id)?;
                        if locator.flags & crate::living_world_grind_services::dmo_locators::LOCATOR_ENABLED == 0
                            || living.dmo_models.model(locator.model_id).is_none() { return None; }
                        let p = &locator.transform[12..15];
                        let support = ground.trajectory_sweep_sphere_segment(
                            [p[0] + 3., p[1] + 5., p[2]],
                            [p[0] + 3., p[1] - 20., p[2]], 0.).unwrap()?;
                        Some((*id, support))
                    }).expect("no supported authored locator in the real streamed tile");
                    test_spawn[12] = support.position[0];
                    test_spawn[13] = support.position[1] + 0.5;
                    test_spawn[14] = support.position[2];
                    eprintln!("populated fixture near actual locator {id:016x}: target={:?}", &test_spawn[12..15]);
                }
                frame.actor.try_publish_teleport(
                    &crate::skeleton_root_transforms::SkeletonRootMatrix::known(test_spawn), true).unwrap();
                frame
                    .living_world
                    .as_deref_mut()
                    .unwrap()
                    .try_initial_census(
                        frame.world,
                        frame.actor,
                        &local,
                        frame.cvars,
                        [test_spawn[12], test_spawn[13], test_spawn[14]],
                        frame.living_world_recipes.as_mut().unwrap(),
                        ground,
                        Some(queued.random),
                    )
                    .unwrap_or_else(|error| panic!("original initial census: {error:?}"));
                local.backend().finish_loading().unwrap();
            })
            .unwrap();
    }
    let world_handle = Arc::clone(session.world_handle().unwrap());
    let cvars = session.assets.as_ref().unwrap().cvars.clone();
    let vault = Arc::clone(&session.assets.as_ref().unwrap().vault);
    let (mut sync, mut feedback, overlay) = {
        let world = world_handle.lock().unwrap();
        (
            Arc::clone(world.physical_world_services().unwrap().game_info()),
            world.try_live_contact_feedback(&cvars).unwrap(),
            world.debug_overlay.clone(),
        )
    };
    let mut recognizer =
        crate::game_input_recognizer_runtime_services::GameInputRuntimeServices::new(
            &cvars, &vault, &overlay, None,
        );
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
    let mut clamp = || Err::<i32, _>("native hands must not run for the source player");
    let settings = GameplaySettings {
        skate3: true,
        difficulty: Difficulty::Normal,
        ..Default::default()
    };
    let neutral = crate::input_pad::XboxPadSample {
        buttons: 0,
        left_trigger: 0,
        right_trigger: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    };
    // skate3_city_prop_kicker_has_no_lip (keyed by its thread, not a PlayerProbe).
    if std::thread::current().name() == Some("prop-lip") {
        include!("skate3_prop_lip_corpus.rs");
    }
    if matches!(trick_probe,Some(PlayerProbe::TerrainAdmission)) {
        session.try_with_selected_queued_frame(|frame,ground,_queued| {
            assert!(ground.published_volume_count()>0);
            let pending = crate::static_ground_pool::StaticGroundPool::try_new(
                crate::static_ground_pool::STATIC_GROUND_NATIVE_CAPACITY).unwrap();
            let physical = frame.actor.cached_physical_allocation().unwrap();
            let owner = Arc::clone(physical.owner());
            let skaters = [crate::actor_construction::selected_tick::SelectedTickSkater {
                board:frame.board,skeleton:frame.skeleton,actor:frame.actor,animation:frame.animation,
            }];
            frame.tick.configure_skate3(settings,Some(neutral));
            for _ in 0..3 {
                frame.tick.test_reconcile_skate3(frame.world,&pending,&skaters,0,&frame.services.settings).unwrap();
                assert!(frame.tick.skate3_output().is_none(),"source instantiated without terrain");
                assert!(owner.skate3.lock().unwrap().is_none());
            }
            frame.tick.test_reconcile_skate3(frame.world,ground,&skaters,0,&frame.services.settings).unwrap();
            assert!(frame.tick.skate3_output().is_some(),"source did not acquire the now-ready world");
            assert!(owner.skate3.lock().unwrap().is_some());
            assert!(Arc::ptr_eq(&owner,frame.actor.cached_physical_allocation().unwrap().owner()),
                "terrain admission replaced the native Skate2 character owner");
            crate::diagnostics::test_log::event("terrain_admission_audit",format_args!(
                "pending_attempts=3 deferred=true resumed=true same_skate2_actor=true"));
        }).unwrap();
        return;
    }
    if matches!(trick_probe, Some(PlayerProbe::OnlineResetSupersession)) {
        include!("skate3_online_reset_corpus.rs");
    }
    if matches!(trick_probe, Some(PlayerProbe::OffboardMarkerMovement)) {
        include!("skate3_offboard_marker_corpus.rs");
        return;
    }
    if matches!(trick_probe, Some(PlayerProbe::OffboardMarkerDrift(_))) {
        include!("skate3_offboard_marker_drift_corpus.rs");
        return;
    }
    let mut saw_offboard = false;
    let mut walking_start = None;
    let mut final_root = [0.; 4];
    let mut saw_world_bodies = false;
    let mut saw_native_gesture = false;
    let mut saw_source_trick = false;
    let mut saw_hud_trick = false;
    let mut saw_hud_points = false;
    let mut saw_audio_surface = false;
    let mut saw_bail = false;
    let mut saw_bail_audio_event = false;
    let mut previous_audio_bail = false;
    let mut teleport_target = test_spawn;
    teleport_target[12] += 0.3;
    // Both original spawns may start above their road. Let the native bodies
    // settle before issuing a grounded flick; the populated locator starts
    // higher than the regular frontend spawn.
    if matches!(trick_probe, Some(PlayerProbe::GrabBanking)) {
        let mut reports=Vec::new();
        let mut failures=Vec::new();
        for source in [false,true] {
            for right in [false,true] {
                for held_on_landing in [false,true] {
                    let selected=GameplaySettings {skate3:if source {true}else{Default::default()},..settings};
                    let actor=session.player.as_ref().unwrap().selected.actor.as_ref().unwrap();
                    actor.try_spawn_foot_forward_stance(1).unwrap();
                    actor.try_publish_teleport(&crate::skeleton_root_transforms::SkeletonRootMatrix::known(test_spawn),true).unwrap();
                    let mut peak=0;
                    let mut finished=0;
                    let mut mission_finished=0;
                    let mut grabbed=false;
                    let mut wiped=false;
                    let mut banked=0i32;
                    let mut backend=crate::ui::trick_backend::TricksBackend::default();
                    for step in 0..240 {
                        let holding=(125..if held_on_landing{210}else{150}).contains(&step);
                        let sample=crate::input_pad::XboxPadSample {
                            buttons:if (80..100).contains(&step){0x1000}else{0},
                            right_y:if (110..120).contains(&step){-i16::MAX}else if (120..122).contains(&step){i16::MAX}else{0},
                            left_trigger:if holding && !right {255}else{0},
                            right_trigger:if holding && right {255}else{0},..neutral};
                        pad.scripted_controller_zero=Some(sample);
                        let input=session.bound_input.as_mut().unwrap(); input.poll_and_cache(pad).unwrap();
                        assert!(input.try_preupdate_enqueue(pad,&session.player.as_ref().unwrap().selected.owners.loader,true,true,&sync).unwrap());
                        let raw=input.raw_controller();
                        let (hud,mission)=session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                            frame.tick.configure_skate3(selected,raw);
                            frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                                input:queued,dt:1./60.,lod:0,wait_for_bs_job:true,
                                physics:SkateboardGroundFrameInput {schedule:SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                                    ground_pool:ground,ground_group:0,pipeline_culling:None,allow_multiple_writes:true,spy_flags:4},
                                pre_grind:GroundPreparationGrindPolicy::RunRecoveredPrefix,post_grind:GrindManagerFillPolicy::RequireNative,
                            },&mut numeric,&mut sync,&mut recognizer,&mut feedback,&mut clamp,&mut HostReckoningNumeric,None,None)
                                .unwrap_or_else(|e|panic!("grab source={source} right={right} held={held_on_landing} tick={step}: {e:?}"));
                            let physical=frame.actor.cached_physical_allocation().unwrap();
                            let hud=frame.world.try_scoring_hud_frame(physical.owner()).unwrap();
                            let output=physical.owner().phys_out().lock().unwrap();
                            let mission=output.try_mission_frame(0,true,true).unwrap();
                            (hud,mission)
                        }).unwrap();
                        grabbed |= mission.tricks_ended.iter().any(|t|t.trick_type==4);
                        peak=peak.max(hud.hud_score);
                        if step>65 {wiped |= hud.just_wiped_out;}
                        banked+=mission.banked_point_change;
                        if hud.sequence_completed && !hud.just_wiped_out && !hud.interrupted {
                            finished=finished.max(hud.finished_sequence_score);
                        }
                        if let Some(sequence)=&mission.sequence_completed {
                            if sequence.successfully {mission_finished=mission_finished.max(sequence.score);}
                        }
                        let mut events=Vec::new();backend.consume(&hud,&mut events).unwrap();
                        if hud.begun.is_some() || hud.ended_count>0 || hud.sequence_completed || hud.just_wiped_out || step%20==0 {
                            eprintln!("grab106 source={source} right={right} held={held_on_landing} step={step} begun={:?} hud={} backend={} finished={} completed={} success={} banked={} wipeout={} air={} events={events:?}",
                                hud.begun,hud.hud_score,backend.sequence_score,hud.finished_sequence_score,hud.sequence_completed,
                                mission.sequence_completed_successfully,mission.banked_point_change,hud.just_wiped_out,hud.aerial);
                        }
                    }
                    let report=format!("source={source} right={right} held={held_on_landing} grabbed={grabbed} peak={peak} finished={finished} mission={mission_finished} banked={banked} wiped={wiped}");
                    eprintln!("grab106 result {report}");
                    if !grabbed || peak==0 || finished==0 || mission_finished!=finished || wiped {failures.push(report.clone());}
                    reports.push(report);
                }
            }
        }
        assert!(failures.is_empty(),"grab landing failures: {failures:#?}");
        assert_eq!(reports.len(),8);
        return;
    }
    if matches!(trick_probe, Some(PlayerProbe::TeleportStance | PlayerProbe::MarkerBanked | PlayerProbe::MarkerPushAway | PlayerProbe::MarkerPushImmediate | PlayerProbe::MarkerRelocation)) {
        let push_away=matches!(trick_probe,Some(PlayerProbe::MarkerPushAway | PlayerProbe::MarkerPushImmediate));
        let immediate=matches!(trick_probe,Some(PlayerProbe::MarkerPushImmediate));
        let relocation=matches!(trick_probe,Some(PlayerProbe::MarkerRelocation));
        if push_away {session.try_with_selected_queued_frame(|frame,_,_| {
            frame.actor.try_set_pinned(false).unwrap();
            frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
        }).unwrap();}
        // Construct the selected source owner through its ordinary queued tick
        // before asking it for fixture terrain or previous-position queries.
        for _ in 0..2 {
            pad.scripted_controller_zero=Some(neutral);
            let input=session.bound_input.as_mut().unwrap();input.poll_and_cache(pad).unwrap();
            assert!(input.try_preupdate_enqueue(pad,&session.player.as_ref().unwrap().selected.owners.loader,true,true,&sync).unwrap());
            let raw=input.raw_controller();
            session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                frame.tick.configure_skate3(settings,raw);
                frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                    input:queued,dt:1./60.,lod:0,wait_for_bs_job:true,
                    physics:SkateboardGroundFrameInput {schedule:SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool:ground,ground_group:0,pipeline_culling:None,allow_multiple_writes:true,spy_flags:4},
                    pre_grind:GroundPreparationGrindPolicy::RunRecoveredPrefix,post_grind:GrindManagerFillPolicy::RequireNative,
                },&mut numeric,&mut sync,&mut recognizer,&mut feedback,&mut clamp,&mut HostReckoningNumeric,None,None).unwrap();
            }).unwrap();
        }
        let mut reports=Vec::new();
        let mut failures=Vec::new();
        for regular in [1u8,0] {
            // Both normal and switch markers, with explicit offboard and reboard resets.
            for (case,(foot,on_board,reverse)) in [(1,true,false),(0,true,true),
                (1,false,false),(0,false,true),(0,true,false),(1,true,true)].into_iter().enumerate() {
                let mut target=test_spawn;
                if reverse { for column in [0,2] { for row in 0..3 {target[column*4+row] = -target[column*4+row];} } }
                let mut prior=[0.;4];
                session.try_with_selected_queued_frame(|frame,_,_| {
                    prior=frame.tick.skate3_output().unwrap().board_transform[3];
                    if relocation && case%2==0 {
                        let offsets=[[20.,0.],[-20.,0.],[0.,20.],[0.,-20.],[10.,0.],[-10.,0.],
                            [0.,10.],[0.,-10.],[5.,0.],[-5.,0.],[0.,5.],[0.,-5.]];
                        let (point,height)=offsets.into_iter().find_map(|offset| {
                            let point=[target[12]+offset[0],target[13],target[14]+offset[1]];
                            let height=frame.tick.skate3_ground_height(point).ok()?;
                            if (height-(target[13]-0.15)).abs()>2. {return None;}
                            for corner in [[0.7,0.7],[-0.7,0.7],[0.7,-0.7],[-0.7,-0.7]] {
                                let other=frame.tick.skate3_ground_height([point[0]+corner[0],point[1],point[2]+corner[1]]).ok()?;
                                if (height-other).abs()>0.1 {return None;}
                            }
                            Some((point,height))
                        }).expect("marker relocation fixture needs a second resident surface at least five metres away");
                        target[12]=point[0];target[14]=point[2];target[13]=height+0.15;
                        eprintln!("marker126 resident destination={point:?} floor={height}");
                    }
                    frame.services.settings.try_set_stance_byte(0,regular).unwrap();
                    frame.actor.try_spawn_foot_forward_stance(foot).unwrap();
                    frame.actor.try_publish_teleport(&crate::skeleton_root_transforms::SkeletonRootMatrix::known(target),on_board).unwrap();
                }).unwrap();
                let mut reactivated=false;
                let mut departure=None;
                for step in 0..if push_away && on_board {160}else{100} {
                    pad.scripted_controller_zero=Some(crate::input_pad::XboxPadSample {
                        buttons:if push_away && on_board && (immediate || step>=100) {1<<12}else{0},..neutral
                    });
                    let input=session.bound_input.as_mut().unwrap();
                    input.poll_and_cache(pad).unwrap();
                    assert!(input.try_preupdate_enqueue(pad,&session.player.as_ref().unwrap().selected.owners.loader,true,true,&sync).unwrap());
                    let raw=input.raw_controller();
                    session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                        frame.tick.configure_skate3(settings,raw);
                        frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                            input:queued,dt:1./60.,lod:0,wait_for_bs_job:true,
                            physics:SkateboardGroundFrameInput {schedule:SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                                ground_pool:ground,ground_group:0,pipeline_culling:None,allow_multiple_writes:true,spy_flags:4},
                            pre_grind:GroundPreparationGrindPolicy::RunRecoveredPrefix,post_grind:GrindManagerFillPolicy::RequireNative,
                        },&mut numeric,&mut sync,&mut recognizer,&mut feedback,&mut clamp,&mut HostReckoningNumeric,None,None)
                            .unwrap_or_else(|e|panic!("spawn regular={regular} case={case} step={step}: {e:?}"));
                        let physical=frame.actor.cached_physical_allocation().unwrap();
                        let hud=frame.world.try_scoring_hud_frame(physical.owner()).unwrap();
                        reactivated |= hud.reactivated;
                        {
                            let out=frame.tick.skate3_output().unwrap();
                            for position in [out.root_transform[3],out.board_transform[3]] {
                                let distance=(0..3).map(|i|(position[i]-target[12+i]).powi(2)).sum::<f32>().sqrt();
                                let prior_distance=(0..3).map(|i|(position[i]-prior[i]).powi(2)).sum::<f32>().sqrt();
                                assert!(step>=if immediate {8}else{100} || distance.min(prior_distance)<5.,"transient spawn displacement regular={regular} case={case} step={step} position={position:?} target={target:?}");
                            }
                        }
                        if matches!(trick_probe,Some(PlayerProbe::MarkerBanked)) && on_board && (step<12 || step%10==0) {
                            eprintln!("banked112 regular={regular} case={case} step={step} state={} gaps={:?}",
                                frame.tick.skate3_output().unwrap().state,frame.tick.skate3_spawn_wheel_gaps());
                        }
                        if step<8 {
                            let out=frame.tick.skate3_output().unwrap();
                            eprintln!("spawn106 phase regular={regular} case={case} step={step} foot={:?} state={} offboard={} {}",
                                frame.tick.skate3_spawn_stance(),out.state,out.offboard,frame.tick.skate3_input_trace());
                        }
                        if immediate && step==1 {
                            assert_eq!(frame.tick.skate3_spawn_stance(),(foot,i32::from(regular==0)),"reset stance before ordinary pushing regular={regular} case={case}");
                        }
                        if push_away && on_board && step>=if immediate {0}else{99} {
                            let out=frame.tick.skate3_output().unwrap();
                            if step==if immediate {0}else{99} {departure=Some(out.board_transform[3]);}
                            if step%5==0 || step==159 {
                                eprintln!("marker118 regular={regular} case={case} step={step} board={:?} state={} gaps={:?} {}",
                                    out.board_transform[3],out.state,frame.tick.skate3_spawn_wheel_gaps(),frame.tick.skate3_input_trace());
                            }
                            if step==159 {
                                let before=departure.unwrap();
                                let position=out.board_transform[3];
                                let distance=((position[0]-before[0]).powi(2)+(position[2]-before[2]).powi(2)).sqrt();
                                if out.offboard || out.wiping_out || !distance.is_finite() || distance<0.5 {
                                    failures.push(format!("push away failed regular={regular} case={case} distance={distance} state={} before={before:?} after={position:?}",out.state));
                                }
                            }
                        }
                        if step==99 {
                            let out=frame.tick.skate3_output().unwrap();
                            let (actual_foot,natural)=frame.tick.skate3_spawn_stance();
                            let pending=frame.actor.try_skate3_teleport_publication().unwrap().3;
                            let heading=if on_board {out.board_transform[2]} else {
                                out.root_transform[2].map(|v|if out.gameplay.stance[1]{-v}else{v})
                            };
                            let heading_dot=(heading[0]*target[8]+heading[2]*target[10]) /
                                (heading[0]*heading[0]+heading[2]*heading[2]).sqrt();
                            let report=format!("regular={regular} case={case} foot={foot}->{actual_foot} natural={natural} board={on_board}->{} state={} pending={pending} reactivated={reactivated} heading_dot={heading_dot} root={:?} target={:?}",
                                !out.offboard,out.state,out.root_transform,target);
                            eprintln!("spawn106 {report}");
                            if on_board {
                                let gaps=frame.tick.skate3_spawn_wheel_gaps().unwrap();
                                eprintln!("spawn107 wheels regular={regular} case={case} gaps_m={gaps:?} contacts={:?}",out.physical.collision.wheel_contact_3296_3299);
                                // A diagnostic regression limit of5mm, not a change
                                // to collision tolerances or a parity claim.
                                if gaps.iter().any(|gap|!gap.is_finite() || *gap < -0.005) {
                                    failures.push(format!("wheel penetration {report} gaps={gaps:?}"));
                                }
                            }
                            if (!immediate && actual_foot!=foot) || natural!=i32::from(regular==0) || out.offboard==on_board || pending || !reactivated || (on_board && heading_dot<0.95) {
                                failures.push(report.clone());
                            }
                            reports.push(report);
                        }
                    }).unwrap();
                }
            }
        }
        assert!(failures.is_empty(),"spawn failures: {failures:#?}");
        assert_eq!(reports.len(),12);
        return;
    }
    if matches!(trick_probe, Some(PlayerProbe::ManualBanking)) {
        let mut reports = Vec::new();
        for mode in [0, 2] {
            world_handle
                .lock()
                .unwrap()
                .local_skater_identity
                .as_ref()
                .unwrap()
                .backend()
                .game_info()
                .try_set_game_mode(mode)
                .unwrap();
            for source in [false, true] {
                session
                    .player
                    .as_ref()
                    .unwrap()
                    .selected
                    .actor
                    .as_ref()
                    .unwrap()
                    .try_publish_teleport(
                        &crate::skeleton_root_transforms::SkeletonRootMatrix::known(test_spawn),
                        true,
                    )
                    .unwrap();
                let selected = GameplaySettings {
                    difficulty: Difficulty::Normal,
                    skate3: if source {
                        true
                    } else {
                        Default::default()
                    },
                    ..Default::default()
                };
                let mut saw_manual = false;
                let mut peak = 0;
                let mut finished = 0;
                let mut backend = crate::ui::trick_backend::TricksBackend::default();
                let mut trace = Vec::new();
                for step in 0..340 {
                    let sample = crate::input_pad::XboxPadSample {
                        buttons: if (32..92).contains(&step) { 0x1000 } else { 0 },
                        // Actual human pad sample at a native manual Begin.
                        right_x: if (112..232).contains(&step) { 848 } else { 0 },
                        right_y: if (112..232).contains(&step) {
                            -23169
                        } else {
                            0
                        },
                        ..neutral
                    };
                    pad.scripted_controller_zero = Some(sample);
                    let input = session.bound_input.as_mut().unwrap();
                    input.poll_and_cache(pad).unwrap();
                    assert!(
                        input
                            .try_preupdate_enqueue(
                                pad,
                                &session.player.as_ref().unwrap().selected.owners.loader,
                                true,
                                true,
                                &sync
                            )
                            .unwrap()
                    );
                    let raw = input.raw_controller();
                    let hud=session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                        frame.tick.configure_skate3(selected,raw);
                        frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                            input:queued,dt:1./60.,lod:0,wait_for_bs_job:true,
                            physics:SkateboardGroundFrameInput {
                                schedule:SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                                ground_pool:ground,ground_group:0,pipeline_culling:None,allow_multiple_writes:true,spy_flags:4 },
                            pre_grind:GroundPreparationGrindPolicy::RunRecoveredPrefix,
                            post_grind:GrindManagerFillPolicy::RequireNative,
                        },&mut numeric,&mut sync,&mut recognizer,&mut feedback,&mut clamp,&mut HostReckoningNumeric,None,None)
                            .unwrap_or_else(|e|panic!("manual source={source} mode={mode} tick={step}: {e:?}"));
                        let physical=frame.actor.cached_physical_allocation().unwrap();
                        frame.world.try_scoring_hud_frame(physical.owner()).unwrap()
                    }).unwrap();
                    saw_manual |= hud.manual_length > 0.;
                    if saw_manual {
                        peak = peak.max(hud.hud_score);
                    }
                    if saw_manual
                        && hud.sequence_completed
                        && !hud.just_wiped_out
                        && !hud.interrupted
                    {
                        finished = finished.max(hud.finished_sequence_score);
                    }
                    let mut events = Vec::new();
                    backend.consume(&hud, &mut events).unwrap();
                    if step % 30 == 0
                        || hud.begun.is_some()
                        || hud.sequence_completed
                        || hud.ended_count > 0
                    {
                        trace.push(format!("tick={step} manual={} live={} finished={} complete={} wiped={} interrupted={} backend={} events={events:?}",
                            hud.manual_length,hud.hud_score,hud.finished_sequence_score,hud.sequence_completed,hud.just_wiped_out,hud.interrupted,backend.sequence_score));
                    }
                }
                eprintln!(
                    "MANUAL_BANK source={source} mode={mode} manual={saw_manual} peak={peak} finished={finished} trace={trace:?}"
                );
                reports.push((source, mode, saw_manual, peak, finished));
            }
        }
        assert!(
            reports.iter().all(|r| r.2 && r.3 > 0 && r.4 > 0),
            "manual hold/release missing native award: {reports:?}"
        );
        return;
    }
    if matches!(trick_probe,Some(PlayerProbe::GestureCatalogs)) {
        session.try_with_selected_queued_frame(|frame,_,_| {
            frame.actor.try_set_pinned(false).unwrap();
            frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
        }).unwrap();
        let mut observed=[false;4];
        let mut mission_gesture=[false;4];
        let mut performed=[false;4];
        for step in 0..960 {
            let variant=step/240;let local=step%240;
            let source=variant<2;
            let set=if variant%2==0 {skate2_formats::skater_profile::GestureSet::Skate3}else{skate2_formats::skater_profile::GestureSet::Skate2};
            let sample=crate::input_pad::XboxPadSample {buttons:if (60..140).contains(&local) {1}else{0},..neutral};
            pad.scripted_controller_zero=Some(sample);
            let input=session.bound_input.as_mut().unwrap();input.poll_and_cache(pad).unwrap();
            assert!(input.try_preupdate_enqueue(pad,&session.player.as_ref().unwrap().selected.owners.loader,true,true,&sync).unwrap());
            let raw=input.raw_controller();
            session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                frame.tick.gesture_preferences=Some(skate2_formats::skater_profile::PcGestureSettings {selected:set,skate3:[36,17,8,29]});
                frame.tick.configure_skate3(GameplaySettings {skate3:if source {true}else{Default::default()},..settings},raw);
                frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                    input:queued,dt:1./60.,lod:0,wait_for_bs_job:true,
                    physics:SkateboardGroundFrameInput {
                        schedule:SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool:ground,ground_group:0,pipeline_culling:None,allow_multiple_writes:true,spy_flags:4,
                    },pre_grind:GroundPreparationGrindPolicy::RunRecoveredPrefix,post_grind:GrindManagerFillPolicy::RequireNative,
                },&mut numeric,&mut sync,&mut recognizer,&mut feedback,&mut clamp,&mut HostReckoningNumeric,None,None).unwrap();
                if local>60&&local<180 {observed[variant]|=frame.actor.test_gesture_active();}
                let physical=frame.actor.cached_physical_allocation().unwrap();
                let output=physical.owner().phys_out().lock().unwrap();
                let mission=output.try_mission_frame(0,true,true).unwrap();
                if local>60 && local<180 {
                    mission_gesture[variant]|=mission.current_gesture!=35;
                    performed[variant]|=mission.performed_gesture.is_some();
                    if mission.current_gesture!=35 {eprintln!("gesture119 variant={variant} local={local} current={} performed={:?}",mission.current_gesture,mission.performed_gesture);}
                }

                if let Some(output)=frame.tick.skate3_output() {assert!(output.world_bones.iter().flatten().flatten().all(|v|v.is_finite()));}
            }).unwrap();
        }
        assert!(observed.into_iter().all(|active|active),"actual Actor failed a physics/catalog combination: {observed:?}");
        assert!(mission_gesture.into_iter().all(|active|active),"gesture did not reach actual mission facts: {mission_gesture:?}");
        assert!(performed.into_iter().all(|active|active),"gesture performance event missing: {performed:?}");
        return;
    }
    if matches!(trick_probe,Some(PlayerProbe::OffboardCircle | PlayerProbe::SmallOffboardCircle(_))) {
        session.try_with_selected_queued_frame(|frame,_,_| {
            frame.actor.try_set_pinned(false).unwrap();
            frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
        }).unwrap();
        let mut observed=0;
        let small=matches!(trick_probe,Some(PlayerProbe::SmallOffboardCircle(_)));
        let dropped=matches!(trick_probe,Some(PlayerProbe::SmallOffboardCircle(true)));
        let mut dropped_frames=0;
        for step in 0usize..if small {980}else{420} {
            let angle=(step as f32-80.)*std::f32::consts::TAU/180.;
            let magnitude=if small {[9000.,10000.,12000.,16000.,26000.][((step.saturating_sub(80))/180).min(4)]}else{26000.};
            let sample=crate::input_pad::XboxPadSample {
                buttons:if step==40 {1<<15}else{0},
                left_trigger:if dropped && step==65 {255}else{0},
                left_x:if step>=80 {(angle.sin()*magnitude) as i16}else{0},
                left_y:if step>=80 {(angle.cos()*magnitude) as i16}else{0},..neutral
            };
            pad.scripted_controller_zero=Some(sample);
            let input=session.bound_input.as_mut().unwrap();input.poll_and_cache(pad).unwrap();
            assert!(input.try_preupdate_enqueue(pad,&session.player.as_ref().unwrap().selected.owners.loader,true,true,&sync).unwrap());
            let raw=input.raw_controller();
            session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                frame.tick.configure_skate3(settings,raw);
                frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                    input:queued,dt:1./60.,lod:0,wait_for_bs_job:true,
                    physics:SkateboardGroundFrameInput {
                        schedule:SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool:ground,ground_group:0,pipeline_culling:None,allow_multiple_writes:true,spy_flags:4,
                    },pre_grind:GroundPreparationGrindPolicy::RunRecoveredPrefix,post_grind:GrindManagerFillPolicy::RequireNative,
                },&mut numeric,&mut sync,&mut recognizer,&mut feedback,&mut clamp,&mut HostReckoningNumeric,None,None).unwrap();
                let output=frame.tick.skate3_output().unwrap();
                assert!(output.world_bones.iter().flatten().flatten().all(|v|v.is_finite()));
                observed+=usize::from(output.offboard);
                dropped_frames+=usize::from(output.offboard && output.physical.off_board.flag_311==0);
                if step%3==0 {eprintln!("offboard110 step={step} raw={:?} {}",[sample.left_x,sample.left_y],frame.tick.skate3_offboard_motion_trace());}
            }).unwrap();
        }
        assert!(observed>if small {900}else{350},"circle input unexpectedly left the actual offboard state");
        if dropped {assert!(dropped_frames>850,"LT fixture did not actually drop the board: {dropped_frames}");}
        return;
    }
    if matches!(trick_probe,Some(PlayerProbe::BailControls | PlayerProbe::MarkerReturnAfterBail)) {
        let marker_probe=matches!(trick_probe,Some(PlayerProbe::MarkerReturnAfterBail));
        let mut marker_reached=false;
        let mut entered=false;
        let mut checked=[false;4];
        let mut bail_frames=0;
        for step in 0..220 {
            if marker_probe && step==110 {
                assert!(entered,"marker fixture must first enter a real bail");
                session.try_with_selected_queued_frame(|frame,_,_| {
                    frame.actor.try_spawn_foot_forward_stance(1).unwrap();
                    frame.actor.try_publish_teleport(
                        &crate::skeleton_root_transforms::SkeletonRootMatrix::known(test_spawn),false).unwrap();
                }).unwrap();
            }
            let axis_index=(bail_frames/3)%4;
            let direction=[[0,i16::MAX],[0,-i16::MAX],[-i16::MAX,0],[i16::MAX,0]][axis_index];
            let requesting=(100..104).contains(&step);
            let sample=crate::input_pad::XboxPadSample {
                buttons:if requesting {(1<<6)|(1<<7)}
                    else if marker_probe && (110..150).contains(&step) {257}
                    else if (32..65).contains(&step){0x1000}else{0},
                left_trigger:if requesting {255}else{0},right_trigger:if requesting {255}else{0},
                right_y:if (70..85).contains(&step){-i16::MAX}else if (85..88).contains(&step){i16::MAX}else{0},
                left_x:if entered && !marker_probe {direction[0]}else{0},
                left_y:if entered && !marker_probe {direction[1]}else{0},..neutral
            };
            pad.scripted_controller_zero=Some(sample);
            let input=session.bound_input.as_mut().unwrap();input.poll_and_cache(pad).unwrap();
            assert!(input.try_preupdate_enqueue(pad,&session.player.as_ref().unwrap().selected.owners.loader,true,true,&sync).unwrap());
            let raw=input.raw_controller();assert_eq!(raw,Some(sample));
            session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                frame.tick.configure_skate3(settings,raw);
                frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                    input:queued,dt:1./60.,lod:0,wait_for_bs_job:true,
                    physics:SkateboardGroundFrameInput {
                        schedule:SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool:ground,ground_group:0,pipeline_culling:None,allow_multiple_writes:true,spy_flags:4,
                    },pre_grind:GroundPreparationGrindPolicy::RunRecoveredPrefix,post_grind:GrindManagerFillPolicy::RequireNative,
                },&mut numeric,&mut sync,&mut recognizer,&mut feedback,&mut clamp,&mut HostReckoningNumeric,None,None).unwrap();
                let output=frame.tick.skate3_output().unwrap();
                if marker_probe && output.state==300 {
                    // The session marker may be used during a source bail.
                    let physical=frame.actor.cached_physical_allocation().unwrap();
                    let native=physical.owner().phys_out().lock().unwrap();
                    let state=native.try_state_output().unwrap();
                    assert!(state.is_wiping_out && state.can_leave_wipeout,"bail blocks the session marker: step={step} can_leave={}",state.can_leave_wipeout);
                }
                if marker_probe && step>=110 {
                    for (label,position) in [("root",output.root_transform[3]),("board",output.board_transform[3])] {
                        let distance=(0..3).map(|i|(position[i]-test_spawn[12+i]).powi(2)).sum::<f32>().sqrt();
                        assert!(distance<5.,"marker returned {label} outside its destination: step={step} state={} position={position:?} distance={distance}",output.state);
                    }
                    marker_reached |= output.offboard && !frame.actor.try_skate3_teleport_publication().unwrap().3;
                }
                if output.state==300 {
                    let (control,axes)=frame.tick.skate3_bail_control_probe();
                    if entered && !marker_probe {
                        let axes=axes.expect("actual State83 bail camera remap");
                        assert!(axes.iter().any(|v|v.abs()>0.1));
                        for i in 0..2 {assert!((control[i]-axes[i]).abs()<0.00001,"bail intent was inverted or remapped twice: {control:?} vs {axes:?}");}
                        checked[axis_index]=true;bail_frames+=1;
                        eprintln!("bail108 step={step} raw={direction:?} axes={axes:?} control={control:?}");
                    }
                    entered=true;
                }
            }).unwrap();
            if !marker_probe && checked.into_iter().all(|v|v){break;}
        }
        if marker_probe {assert!(marker_reached,"offboard marker reset never completed on the actual native Actor");}
        else {assert!(entered && checked.into_iter().all(|v|v),"real bail did not exercise every stick direction: {checked:?}");}
        return;
    }
    if matches!(trick_probe, Some(PlayerProbe::FootplantSteering | PlayerProbe::Footplant | PlayerProbe::Boneless)) {
        let steering=matches!(trick_probe,Some(PlayerProbe::FootplantSteering));
        let boneless=matches!(trick_probe,Some(PlayerProbe::Boneless));
        let count=if steering {5} else {4};
        let mut performed=vec![false;count];
        let mut named=vec![false;count];
        let mut foot_out=vec![false;count];
        let mut entered=vec![false;count];
        let mut released=vec![false;count];
        let mut labelled=vec![false;count];
        let mut last_planting=false;
        let mut plant_ticks=0;
        let mut apex=vec![f32::NEG_INFINITY;count];
        let mut plant_controls=vec![[0.;2];count];
        let mut launches=vec![[0.;4];count];
        let mut remapped=vec![false;count];
        for step in 0..count*300 {
            let variant=step/300;
            let local=step%300;
            if local==0 {
                last_planting=false;
                plant_ticks=0;
                session.player.as_ref().unwrap().selected.actor.as_ref().unwrap().try_publish_teleport(
                    &crate::skeleton_root_transforms::SkeletonRootMatrix::known(test_spawn),true).unwrap();
            }
            let grab=(100..190).contains(&local);
            let one_foot=(105..190).contains(&local);
            let input_variant=if steering {0}else{variant};
            // A timed pullback can redirect forward release upward; holding
            // indefinitely can swing past vertical. Other directions stay held.
            let steer=if steering && last_planting && (variant!=1 || plant_ticks==1) {
                [[0,0],[0,-i16::MAX],[0,i16::MAX],[-i16::MAX,0],[i16::MAX,0]][variant]
            }else{[0,0]};
            // Stock inputs: LT/RT choose grab; A/X choose the free foot.
            let sample=crate::input_pad::XboxPadSample {
                left_trigger: if grab && input_variant<2 {255} else {0},
                right_trigger: if grab && input_variant>=2 {255} else {0},
                buttons: if one_foot {1 << if input_variant%2==0 {12} else {14}}
                    else if steering && (32..65).contains(&local) {0x1000}else{0},
                right_y: if !boneless && (70..85).contains(&local) {-i16::MAX}
                    else if !boneless && (85..88).contains(&local) {i16::MAX} else {0},
                left_x:if steering {steer[0]} else {0},
                left_y:steer[1],
                ..neutral
            };
            pad.scripted_controller_zero=Some(sample);
            let input=session.bound_input.as_mut().unwrap();
            input.poll_and_cache(pad).unwrap();
            assert!(input.try_preupdate_enqueue(pad,&session.player.as_ref().unwrap().selected.owners.loader,
                true,true,&sync).unwrap());
            let raw=input.raw_controller();
            assert_eq!(raw,Some(sample));
            session.try_with_selected_queued_frame(|mut frame,ground,queued| {
                frame.tick.configure_skate3(settings,raw);
                frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                    input: queued, dt: 1./60., lod: 0, wait_for_bs_job: true,
                    physics: SkateboardGroundFrameInput {
                        schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool: ground, ground_group: 0, pipeline_culling: None,
                        allow_multiple_writes: true, spy_flags: 4,
                    },
                    pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                    post_grind: GrindManagerFillPolicy::RequireNative,
                }, &mut numeric, &mut sync, &mut recognizer, &mut feedback,
                    &mut clamp, &mut HostReckoningNumeric, None, None)
                    .unwrap_or_else(|error|panic!("source footplant variant={variant} step={local}: {error:?}"));
                let output=frame.tick.skate3_output().unwrap();
                if steering {
                    last_planting=output.state==601;
                    plant_ticks+=u32::from(last_planting);
                    entered[variant] |= last_planting;
                    released[variant] |= entered[variant] && (200..=202).contains(&output.state);
                    if released[variant] && (200..=202).contains(&output.state) {
                        apex[variant]=apex[variant].max(output.centre_of_mass[1]);
                    }
                    if last_planting {
                        let (adjust,launch,axes)=frame.tick.skate3_plant_control_probe();
                        if adjust.iter().any(|v|v.abs()>0.) {plant_controls[variant]=adjust;}
                        remapped[variant] |= axes.is_some();
                        launches[variant]=launch;
                        eprintln!("plant108 variant={variant} step={local} stick={steer:?} adjust={adjust:?} axes={axes:?} release={launch:?}");
                    }
                }
                if boneless {
                    if local==99 {assert_eq!(output.state,100,"ground-grab fixture did not settle");}
                    entered[variant] |= output.state==602;
                    performed[variant] |= output.state==602;
                    released[variant] |= entered[variant] && (200..=202).contains(&output.state);
                    if let Some(trick)=output.gameplay.trick {
                        named[variant] |= skate3_core::scoring::catalog::IDENTIFIERS.iter().any(|(name,_,_)|
                            (name.contains("boneless") || name.contains("fastplant") || name.contains("nocomply"))
                            && skate3_core::animation::skeleton_input::name::encode(name.as_bytes())==trick);
                    }
                    if [99,105,120,135,150,165,180,210].contains(&local) {
                        eprintln!("source boneless variant={variant} step={local} state={} {}",output.state,frame.tick.skate3_boneless_trace());
                    }
                    return;
                }
                if local==99 { assert!((200..=202).contains(&output.state),"footplant grab fixture was not airborne: {}",output.state); }
                if (105..220).contains(&local) {
                    performed[variant] |= output.physical.air.flag_447!=0;
                    foot_out[variant] |= output.gameplay.feet.within_deck_box.into_iter().any(|within|!within);
                    if let Some(trick)=output.gameplay.trick {
                        named[variant] |= skate3_core::scoring::catalog::IDENTIFIERS.iter().any(|(name,_,_)|
                            name.contains("_footplant") && skate3_core::animation::skeleton_input::name::encode(name.as_bytes())==trick);
                    }
                }
                if [99,105,120,135,150,165,180].contains(&local) {
                    eprintln!("source footplant variant={variant} step={local} state={} foot_out={:?} air={}/{}/{} {}",
                        output.state,output.gameplay.feet.within_deck_box,output.physical.air.flag_447,
                        output.physical.air.footplant_contact_time_208,output.physical.air.footplant_duration_212,
                        frame.tick.skate3_footplant_trace());
                }
            }).unwrap();
        }
        if steering {
            eprintln!("plant108 result entered={entered:?} released={released:?} remapped={remapped:?} controls={plant_controls:?} launches={launches:?} apex={apex:?}");
            assert!(entered.iter().all(|v|*v) && released.iter().all(|v|*v),"actual plant/air lifecycle missing");
            assert!(remapped.iter().all(|v|*v),"native State83 camera remap missing");
            assert!(plant_controls[1..].iter().all(|v|v.iter().any(|x|x.abs()>0.)),"left stick never reached BodyAdjustX/Z");
            assert!(launches[1][1]>launches[0][1],"pulling back failed to raise the source release: {launches:?}");
            assert!(apex[1]>apex[0],"timed back pull failed to raise actual flight apex: {apex:?}");
            for i in 1..count {
                assert!((launches[i][0]-launches[0][0]).abs()+(launches[i][2]-launches[0][2]).abs()>0.0001,"steering left no horizontal release change");
            }
            return;
        }
        if boneless {
            eprintln!("source boneless results entered={entered:?} released={released:?} named={named:?}");
            assert!(entered[..count].iter().all(|v|*v),"source state602 was not entered: {entered:?}");
            assert!(released[..count].iter().all(|v|*v),"source state602 never launched into air: {released:?}");
        }
        eprintln!("source footplant results performed={performed:?} named={named:?} foot_out={foot_out:?}");
        assert!(performed[..count].iter().all(|v|*v),"source footplant contact never completed: {performed:?}");
        assert!(named[..count].iter().all(|v|*v),"source footplants never published authored scoring: {named:?}");
        return;
    }
    if let Some(PlayerProbe::Darkcatch(foot)) = trick_probe {
        session.player.as_ref().unwrap().selected.actor.as_ref().unwrap().try_spawn_foot_forward_stance(foot).unwrap();
        let mut named = false;
        let mut inverted = false;
        let mut labelled = false;
        let labels = crate::ui::gameplay::skate3_trick_strings().unwrap();
        for step in 0..280 {
            // A flip ends diagonally upward; a horizontal-only endpoint is the
            // authored Switch gesture (verified by the source AG diagnostics).
            let sample = crate::input_pad::XboxPadSample {
                buttons: if (100..155).contains(&step) { 1 << 9 } else { 0 },
                right_y: if (70..85).contains(&step) { -i16::MAX }
                    else if (85..88).contains(&step) { i16::MAX } else { 0 },
                right_x: if (85..88).contains(&step) { i16::MAX } else { 0 },
                ..neutral
            };
            pad.scripted_controller_zero = Some(sample);
            let input = session.bound_input.as_mut().unwrap();
            input.poll_and_cache(pad).unwrap();
            assert!(input.try_preupdate_enqueue(pad, &session.player.as_ref().unwrap().selected.owners.loader,
                true, true, &sync).unwrap());
            let raw = input.raw_controller();
            assert_eq!(raw,Some(sample));
            session.try_with_selected_queued_frame(|mut frame, ground, queued| {
                frame.tick.configure_skate3(settings, raw);
                frame.try_step_queued::<_,_,_,_,_,_,Infallible>(SelectedTickRequest {
                    input: queued, dt: 1./60., lod: 0, wait_for_bs_job: true,
                    physics: SkateboardGroundFrameInput {
                        schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool: ground, ground_group: 0, pipeline_culling: None,
                        allow_multiple_writes: true, spy_flags: 4,
                    },
                    pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                    post_grind: GrindManagerFillPolicy::RequireNative,
                }, &mut numeric, &mut sync, &mut recognizer, &mut feedback,
                    &mut clamp, &mut HostReckoningNumeric, None, None)
                    .unwrap_or_else(|error|panic!("source darkcatch foot={foot} step={step}: {error:?}"));
                let output = frame.tick.skate3_output().unwrap();
                if [70,85,90,100,110,120,140].contains(&step) {
                    eprintln!("source darkcatch control foot={foot} step={step} {}", frame.tick.skate3_input_trace());
                }
                if let Some(trick) = output.gameplay.trick {
                    let name = skate3_core::scoring::catalog::IDENTIFIERS.iter().find_map(|(name,_,_)|
                        (skate3_core::animation::skeleton_input::name::encode(name.as_bytes()) == trick).then_some(*name));
                    let dark = name.is_some_and(|name|name.contains("_darkcatch"));
                    named |= dark;
                    inverted |= dark && output.board_transform[1][1] < 0.
                        && output.gameplay.feet.within_deck_box.into_iter().any(|within|within);
                    if step % 5 == 0 { eprintln!("source darkcatch pose foot={foot} step={step} name={name:?} up={} feet={:?}",
                        output.board_transform[1][1],output.gameplay.feet.within_deck_box); }
                }
                let owner = frame.actor.cached_physical_allocation().unwrap();
                let hud = frame.world.try_scoring_hud_frame(owner.owner()).unwrap();
                // Darkcatch converts the already-started flip; the original
                // HUD deliberately emits converted when no new trick began.
                for trick in hud.begun.iter().chain(&hud.converted) {
                    eprintln!("source darkcatch HUD foot={foot} step={step} begun={} name={trick:?}", hud.begun.is_some());
                    if trick.valid && let Some(key) = trick.label.as_ref() {
                        // The original HUD carries a localization ID, then its
                        // text renderer resolves that ID through the merged table.
                        let hash = skate2_formats::localization::string_hash(key.as_bytes());
                        if let Some((_, units)) = labels.iter().find(|(id,_)|*id == hash) {
                            let text = String::from_utf16(units).unwrap();
                            labelled |= text.contains("Dark Catch");
                            eprintln!("source darkcatch localized HUD foot={foot} step={step} text={text:?}");
                        }
                    }
                }
            }).unwrap();
        }
        assert!(named, "source darkcatch animation not selected (leading foot {foot})");
        assert!(inverted, "source darkcatch did not catch an inverted board (leading foot {foot})");
        assert!(labelled, "source darkcatch did not reach original named HUD (leading foot {foot})");
        return;
    }
    for tick in 0..450 {
        if tick == 210 {
            session
                .player
                .as_ref()
                .unwrap()
                .selected
                .actor
                .as_ref()
                .unwrap()
                .try_publish_teleport(
                    &crate::skeleton_root_transforms::SkeletonRootMatrix::known(teleport_target),
                    false,
                )
                .unwrap();
        }
        pad.scripted_controller_zero = Some(crate::input_pad::XboxPadSample {
            buttons: if tick == 180 { 1 << 15 } else if (310..320).contains(&tick) { 0x4000 } else if (80..100).contains(&tick) { 0x1000 } else { 0 },
            left_y: if tick >= 235 { i16::MAX } else { 0 },
            right_y: if (110..120).contains(&tick) { -i16::MAX } else if (120..122).contains(&tick) { i16::MAX } else { 0 },
            ..neutral
        });
        let input = session.bound_input.as_mut().unwrap();
        input.poll_and_cache(pad).unwrap();
        assert!(
            input
                .try_preupdate_enqueue(
                    pad,
                    &session.player.as_ref().unwrap().selected.owners.loader,
                    true,
                    true,
                    &sync
                )
                .unwrap()
        );
        let raw = input.raw_controller();
        let (completed, output) = session
            .try_with_selected_queued_frame(|mut frame, ground, queued| {
                frame.tick.configure_skate3(
                    if tick >= 250 {
                        GameplaySettings {
                            difficulty: [
                                Difficulty::Skate2Default,
                                Difficulty::Easy,
                                Difficulty::Normal,
                                Difficulty::Hardcore,
                                Difficulty::Motorized,
                                Difficulty::EasyMotorized,
                            ][((tick - 250) / 10).min(5)],
                            ..settings
                        }
                    } else {
                        settings
                    },
                    raw,
                );
                let completed = frame
                    .try_step_queued::<_, _, _, _, _, _, Infallible>(
                        SelectedTickRequest {
                            input: queued,
                            dt: 1. / 60.,
                            lod: 0,
                            wait_for_bs_job: true,
                            physics: SkateboardGroundFrameInput {
                                schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                                ground_pool: ground,
                                ground_group: 0,
                                pipeline_culling: None,
                                allow_multiple_writes: true,
                                spy_flags: 4,
                            },
                            pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                            post_grind: GrindManagerFillPolicy::RequireNative,
                        },
                        &mut numeric,
                        &mut sync,
                        &mut recognizer,
                        &mut feedback,
                        &mut clamp,
                        &mut HostReckoningNumeric,
                        None,
                        None,
                    )
                    .unwrap_or_else(|error| panic!("source frame {tick}: {error:?}"));
                let output = frame
                    .tick
                    .skate3_output()
                    .expect("active source player")
                    .clone();
                assert_eq!(output.tick, (tick + 1) as u64, "one native source step per admitted world tick");
                saw_world_bodies |= frame.tick.skate3_world_body_count() > 0;
                saw_source_trick |= output.gameplay.trick.is_some();
                let physical = frame.actor.cached_physical_allocation().unwrap();
                if populated && [0,59,179].contains(&tick) {
                    let audit=frame.tick.skate3_grab_audit(frame.world).unwrap();
                    eprintln!("actual native2 grab scene tick={tick} objects={} authored_records={} eligible_mesh_hits={}",audit.0,audit.1,audit.2);
                    assert!(audit.0>0 && audit.1>0 && audit.2>0,"populated source grab scene has no real usable native objects: {audit:?}");
                }
                {
                    let observer=physical.owner().phys_out().lock().unwrap();
                    let state=observer.try_state_output().unwrap();
                    let offboard=observer.try_offboard_output().unwrap();
                    assert_eq!(state.interactable_object_id,output.gameplay.interactable_object.0);
                    assert_eq!(offboard.has_valid_interactable_object,output.gameplay.interactable_object.1);
                    assert_eq!(offboard.is_grabbing_object,output.physical.off_board.flag_304!=0);
                    assert_eq!(state.grab_spline_desc.object_type,output.physical.state.counter_36 as i32);
                    assert_eq!(state.grab_spline_desc.object_id,output.physical.state.skitch_value_40 as i32);
                }
                let audio = physical.owner().phys_out().lock().unwrap().try_audio_phys_out().unwrap();
                saw_bail |= output.wiping_out;
                saw_bail_audio_event |= audio.is_wiping_out && !previous_audio_bail;
                previous_audio_bail = audio.is_wiping_out;

                saw_audio_surface |= audio.num_wheels_in_contact > 0 && audio.wheel_audio_surface_type.iter().any(|&surface| surface != 0);
                let hud = frame.world.try_scoring_hud_frame(physical.owner()).unwrap();
                saw_hud_points |= hud.hud_score > 0 || hud.finished_sequence_score > 0;
                if tick == 449 { assert!(hud.begun.is_none(), "a completed trick event leaked into later walking frames"); }

                if hud.begun.as_ref().is_some_and(|trick| trick.valid && trick.label.is_some()) {
                    saw_hud_trick = true;
                    eprintln!("native source trick reaches Skate 2 HUD: tick={tick} begun={:?} score={}", hud.begun, hud.hud_score);
                }

                let published = frame.presentation_runtime.published_root().unwrap();
                let solved = crate::skate3_bridge::player::flatten(output.root_transform);
                // Native Skate 2 transports a geometric affine W and decodes
                // the root rotation from its normal presentation quaternion.
                for axis in [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14] {
                    assert!((published[axis] - solved[axis]).abs() < 0.00001);
                }
                assert!(frame.presentation_runtime
                    .submitted_frame(frame.presentation, &vault).unwrap().is_some(),
                    "Skate 3 mode bypassed the native Skate 2 presentation/replay writer");
                if tick % 60 == 0 {
                    // Require a fresh cloth step rather than an early return
                    // for clothing already published on this simulation tick.
                    frame.presentation_runtime.set_cloth_tick(
                        frame.tick.presentation_tick().wrapping_add(1));
                    frame.presentation_runtime.restore_game_cloth()
                        .expect("native clothing restoration after a menu/replay return");
                }
                {
                    let input = frame.actor.published_animation_physics_input().unwrap();
                    let input = input.lock().unwrap();
                    if input.gesturing && !saw_native_gesture {
                        eprintln!("native Skate 2 gesture reached physics input: tick={tick} gesture={}", input.gesture);
                        saw_native_gesture = true;
                    }
                }
                if tick == 144 {
                    audit_rendered_character(frame.presentation_runtime.palette().unwrap(), &output, ground);
                }
                assert!(frame.presentation_runtime.palette().unwrap().generation() > 0);
                let palette = frame.presentation_runtime.palette().unwrap();
                for part in 0..palette.skinning().parts.len() {
                    assert!(
                        palette
                            .cloth_matrices(part)
                            .unwrap()
                            .iter()
                            .flatten()
                            .all(|v| v.is_finite()),
                        "non-finite source pose reached the actual Skate 2 skinned/cloth matrices"
                    );
                }
                (completed, output)
            })
            .unwrap();
        let SelectedQueuedTickOutcome::Completed(completed) = completed else {
            panic!("no admitted tick");
        };
        assert!(
            completed.before.is_none() && completed.hands.is_some() && completed.after.is_none(),
            "Skate 2 character animation must run while its physical solver is excluded"
        );
        if !populated {
            assert_eq!(
                completed.report.processed_assemblies, 0,
                "local S2 collision solve ran"
            );
        }
        let world = world_handle.lock().unwrap();
        let actor = session
            .player
            .as_ref()
            .unwrap()
            .selected
            .actor
            .as_ref()
            .unwrap();
        let owner = Arc::clone(actor.cached_physical_allocation().unwrap().owner());
        let output2 = owner.phys_out().lock().unwrap();
        let (centre, _) = output2.try_system_centre_and_damped_velocity().unwrap();
        assert_eq!(centre.xyz, output.centre_of_mass[..3]);
        let skeleton = output2.try_skeleton_output().unwrap();
        for (name, actual) in [
            ("Neck1", skeleton.skeleton_head_pos.xyz),
            ("Hips", skeleton.skeleton_hips_pos.xyz),
            ("LeftToeBase", skeleton.skeleton_foot_left_pos.xyz),
            ("RightToeBase", skeleton.skeleton_foot_right_pos.xyz),
        ] {
            let source = output
                .skeleton_parts
                .iter()
                .find(|p| p.bone_name.eq_ignore_ascii_case(name))
                .unwrap();
            assert_eq!(actual, source.body.transform[3][..3]);
            assert!(
                [0, 2]
                    .iter()
                    .all(|&axis| (actual[axis] - output.root_transform[3][axis]).abs() < 5.),
                "world census received a stale physical {name} origin"
            );
        }
        for axis in [0, 2] {
            assert!(
                (centre.xyz[axis] - output.root_transform[3][axis]).abs() < 5.,
                "world census received an animation-local centre instead of the source world-space publication"
            );
        }
        if populated && [0, 59, 119, 121, 209].contains(&tick) {
            eprintln!(
                "census observer tick={tick} flags={:#x} filtered={:?} centre={:?}",
                actor.try_input_generation_flags().unwrap(),
                output2
                    .try_filtered_output()
                    .unwrap()
                    .filtered_physics_state,
                output.centre_of_mass
            );
        }
        if tick == 210 {
            assert_eq!(
                actor.try_input_generation_flags().unwrap() & 0x2000_0000,
                0,
                "source reset completed but Actor still blocks the world census"
            );
            assert_eq!(
                output2
                    .try_completed_teleport()
                    .unwrap()
                    .unwrap()
                    .known_lanes()
                    .unwrap(),
                teleport_target,
                "source teleport did not acknowledge the real Actor reset"
            );
        }
        assert_eq!(
            output2
                .try_skeleton_output()
                .unwrap()
                .skeleton_root_matrix
                .known_lanes()
                .unwrap(),
            crate::skate3_bridge::player::flatten(output.root_transform)
        );
        let camera = world
            .local_skater_identity
            .as_ref()
            .unwrap()
            .try_camera_producer_projection()
            .unwrap();
        assert_eq!(camera.camera_update_counter, output.tick as u32);
        assert!(camera.view_from_world.iter().all(|v| v.is_finite()));
        saw_offboard |= output.offboard;
        if tick == 235 {
            walking_start = Some(output.root_transform[3]);
        }
        final_root = output.root_transform[3];
    }
    if populated {
        // This fixture deliberately starts at an authored object's locator.
        // It bails against the real object before the scripted grounded flick;
        // validate the collision/bail consumers here. The separate road fixture
        // validates trick recognition, points and rolling surfaces.
        assert!(saw_bail, "populated object collision never reached native bail physics");
        assert!(saw_bail_audio_event, "source bail never reached the original Skate 2 audio event");
    } else {
        assert!(saw_source_trick, "raw onboard flick never reached source trick recognition");
        assert!(saw_hud_trick, "source trick never reached the original Skate 2 named trick HUD");
        assert!(saw_hud_points, "the original Skate 2 score output never received points from source movement");
        assert!(saw_audio_surface, "source riding never published a contacting audio surface to Skate 2");
    }
    assert!(
        saw_offboard,
        "raw hardware Y did not reach source offboard through the selected frame"
    );
    if populated {
        assert!(
            saw_world_bodies,
            "main-bank census did not publish any actual world bodies"
        );
    }
    let start = walking_start.unwrap();
    let moved = ((final_root[0] - start[0]).powi(2) + (final_root[2] - start[2]).powi(2)).sqrt();
    assert!(
        moved > 0.1,
        "native offboard input did not move the player: {start:?} -> {final_root:?}"
    );
    if !populated {
        // Match PartyPlayLoadingState's real player rows and second Actor.
        // This catches indexing the primary skater after SetLocalIndex(1).
        let services = &session.player.as_ref().unwrap().selected.owners.services;
        services.settings.try_reset_players().unwrap();
        services.apply_marquee(DEFAULT_MARQUEE_ID, 0).unwrap();
        services.settings.try_set_character(0, DEFAULT_MARQUEE_ID).unwrap();
        services.settings.try_create_party_human(0, crate::player::NO_PEER_ID, b"Player 1").unwrap();
        services.settings.try_set_controller_index(0, 0).unwrap();
        services.apply_marquee(DEFAULT_MARQUEE_ID, 1).unwrap();
        services.settings.try_set_character(1, DEFAULT_MARQUEE_ID).unwrap();
        services.settings.try_create_party_human(1, 1, b"Player 2").unwrap();
        services.settings.try_set_controller_index(1, 1).unwrap();
        session.spawn_policy = Some(crate::selected_session_boot_owner::party_skaters::SkaterSpawnPolicy {
            actor_flags: 1,
            capsule_resource_allocated: true,
            presentation: SelectedPresentationPolicy {
                lod: 0, submission_capacity: 65536, gather_capacity: 65536,
                provider: 0, root_w: RootPositionWTransport::ReadSkaterOpaque { bits: 0 },
            },
        });
        let mut party_spawn = session.assets.as_ref().unwrap().spawn;
        party_spawn[14] -= 5.;
        session.try_spawn_party_skater::<Infallible>(1,
            crate::selected_session_boot_owner::party_skaters::PartyCharacter::Marquee(DEFAULT_MARQUEE_ID),
            party_spawn).unwrap();
        let local = world_handle.lock().unwrap().local_skater_identity.clone().unwrap();
        local.backend().game_info().try_set_game_mode(2).unwrap();
        local.backend().finish_loading().unwrap();
        for tick in 0..240 {
            // The preceding six-profile scenario now really auto-pushes and
            // leaves player0 moving at over8m/s. Its dormant native solver can
            // hit scenery before selection. Establish this separate walking
            // acquisition case with the real marker reset, not that unrelated
            // previous path or a forged physical state/velocity.
            if tick == 80 {
                session.player.as_ref().unwrap().selected.actor.as_ref().unwrap()
                    .try_publish_teleport(
                        &crate::skeleton_root_transforms::SkeletonRootMatrix::known(test_spawn), true)
                    .unwrap();
            }
            let selected = if tick < 80 || tick >= 160 { 1 } else { 0 };
            let using_source = !(80..140).contains(&tick);
            if tick % 80 == 0 || tick == 140 {
                let actor = if selected == 0 { session.player.as_ref().unwrap().selected.actor.as_ref().unwrap() }
                    else { &session.party[0].actor };
                let physical = actor.cached_physical_allocation().unwrap();
                let output = physical.owner().phys_out().lock().unwrap();
                eprintln!("party selected={selected} before handoff offboard={} tick={tick}", output.try_state_output().unwrap().is_offboard);
                if tick == 140 { assert!(output.try_state_output().unwrap().is_offboard,
                    "active native S2 Y input never completed the offboard acquisition fixture"); }
                local.try_set_local_index(selected).unwrap();
                crate::session_marker::teleport::PlayerUiTeleportReceiver::set_player(
                    session.bound_input.as_mut().unwrap(), selected).unwrap();
            }
            let active_sample = crate::input_pad::XboxPadSample {
                buttons: if (selected == 1 && tick % 80 == 30) || tick == 85 { 1 << 15 } else { 0 },
                left_y: if tick % 80 >= 40 { i16::MAX } else { 0 },
                ..neutral
            };
            let inactive_sample = crate::input_pad::XboxPadSample { left_y: -i16::MAX, ..neutral };
            pad.scripted_controller_zero = Some(if selected == 0 { active_sample } else { inactive_sample });
            pad.scripted_controller_one = Some(if selected == 1 { active_sample } else { inactive_sample });
            let input = session.bound_input.as_mut().unwrap();
            input.poll_and_cache(pad).unwrap();
            let loader = if selected == 0 { &session.player.as_ref().unwrap().selected.owners.loader }
                else { &session.party[0].loader };
            assert!(input.try_preupdate_enqueue(pad, loader,
                true, true, &sync).unwrap());
            let raw = input.raw_controller();
            assert_eq!(raw, Some(active_sample), "a different party controller supplied the source packet");
            session.try_with_selected_queued_frame(|mut frame, ground, queued| {
                assert_eq!(frame.local, selected as usize);
                // Exercise real native S2 input/animation/physics on player 0
                // before enabling the source solver while that actor is walking.
                frame.tick.configure_skate3(if using_source { settings } else { GameplaySettings::default() }, raw);
                frame.try_step_queued::<_, _, _, _, _, _, Infallible>(SelectedTickRequest {
                    input: queued, dt: 1. / 60., lod: 0, wait_for_bs_job: true,
                    physics: SkateboardGroundFrameInput {
                        schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool: ground, ground_group: 0, pipeline_culling: None,
                        allow_multiple_writes: true, spy_flags: 4,
                    },
                    pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                    post_grind: GrindManagerFillPolicy::RequireNative,
                }, &mut numeric, &mut sync, &mut recognizer, &mut feedback,
                &mut clamp, &mut HostReckoningNumeric, None, None)
                    .unwrap_or_else(|error| panic!("party native frame {tick}, player {selected}: {error:?}"));
                let output = frame.tick.skate3_output();
                assert_eq!(output.is_some(), using_source);
                assert_eq!(frame.tick.skate3_world_body_count(), 0, "Party Play exported another player's bodies as collidable objects");
                // The source's first output is its explicit 702 teleport
                // publication. The next state must resume the walking actor.
                if tick == 141 { let output = output.as_ref().unwrap();
                    assert!(output.offboard, "acquiring the walking primary actor remounted it: state={}", output.state); }
                let primary = frame.actor.cached_physical_allocation().unwrap();
                let secondary = frame.others[0].actor.cached_physical_allocation().unwrap();
                if tick == 84 {
                    let output=primary.owner().phys_out().lock().unwrap();
                    let state=output.try_state_output().unwrap();
                    assert_eq!(state.physics_state,100,"party native dismount precondition was not grounded: {state:?}");
                    assert!(!state.is_wiping_out);
                }
                if [85, 86, 130].contains(&tick) {
                    let prefix = frame.actor.prefix().unwrap();
                    let graphs = prefix.graphs().unwrap();
                    let action = graphs.action().unwrap();
                    let motion = graphs.motion().unwrap();
                    let name = crate::actor_graph_construction::ActorGraphIntents::native_name;
                    let action_state = action.controller.as_ref().unwrap().generated_action().unwrap().lock().unwrap().current_state;
                    let motion_state = motion.controller.as_ref().unwrap().generated_motion().unwrap().lock().unwrap().current_state;
                    let toggle = action.intents.as_ref().unwrap().get(&name(b"NewToggleOffBoardState"));
                    let offboard = motion.intents.as_ref().unwrap().get(&name(b"OffBoard"));
                    drop(prefix);
                    let physical = primary.owner().phys_out().lock().unwrap();
                    eprintln!("native S2 admission tick={tick} AG={action_state} MG={motion_state} toggle={toggle:?} offboard_intent={offboard:?} physical={:?}",
                        physical.try_state_output().unwrap());
                }
                assert_eq!(primary.owner().skate3.lock().unwrap().is_some(), selected == 0 && using_source);
                assert_eq!(secondary.owner().skate3.lock().unwrap().is_some(), selected == 1 && using_source);
                if let Some(output) = output {
                let presented = if selected == 0 { frame.presentation_runtime.published_root() }
                    else { frame.others[0].presentation_runtime.published_root() }.unwrap();
                let solved = crate::skate3_bridge::player::flatten(output.root_transform);
                for axis in [12, 13, 14] { assert!((presented[axis] - solved[axis]).abs() < 0.00001); }
                }
            }).unwrap();
        }
    }
}

fn probe_spawn(default:[f32;16],probe:Option<PlayerProbe>)->[f32;16] {
    if matches!(probe,Some(PlayerProbe::MarkerBanked | PlayerProbe::MarkerPushAway | PlayerProbe::MarkerPushImmediate | PlayerProbe::MarkerRelocation)) {
        [0.6310481,-0.07590369,-0.77202123,0.,0.10443655,0.99445415,-0.0124067245,0.,
            0.76868147,-0.07279799,0.6354755,0.,1192.4199,208.65027,-1213.2676,0.]
    }else{default}
}
#[test]
#[ignore="requires original Skate 2 collision at the player's logged banked marker"]
fn skate3_banked_marker_wheels_settle_above_actual_collision() {
    std::thread::Builder::new().name("banked-marker".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::MarkerBanked))).unwrap().join().unwrap();
}

#[test]
#[ignore="requires original Skate 2 world and embedded gesture bank; both animation catalogs on both physical solvers"]
fn skate3_gestures_use_actual_actor_on_both_physical_solvers() {
    std::thread::Builder::new().name("gesture-catalogs".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::GestureCatalogs))).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 corpus and embedded Skate 3 bank"]
fn skate3_banked_marker_pushes_away_without_sticking() {
    std::thread::Builder::new().name("marker-push-away".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::MarkerPushAway))).unwrap().join().unwrap();
}
#[test]
#[ignore = "requires original Skate 2 corpus and embedded Skate 3 bank"]
fn skate3_small_stick_offboard_circle_carrying_board() {
    std::thread::Builder::new().name("small-circle-held".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::SmallOffboardCircle(false)))).unwrap().join().unwrap();
}
#[test]
#[ignore = "requires original Skate 2 corpus and embedded Skate 3 bank"]
fn skate3_small_stick_offboard_circle_dropped_board() {
    std::thread::Builder::new().name("small-circle-dropped".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::SmallOffboardCircle(true)))).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 corpus and embedded Skate 3 bank"]
fn skate3_banked_marker_accepts_push_held_during_return() {
    std::thread::Builder::new().name("marker-held-push".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::MarkerPushImmediate))).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 corpus and embedded Skate 3 bank"]
fn skate3_banked_marker_relocates_with_pending_queries() {
    std::thread::Builder::new().name("marker-query-relocation".into()).stack_size(16<<20)
        .spawn(||run_embedded_player_corpus(true,false,Some(PlayerProbe::MarkerRelocation))).unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 map and embedded player assets"]
fn skate3_offboard_marker_return_does_not_drift() {
    for walking in [false, true] {
        std::thread::Builder::new().name("offboard-marker-drift".into()).stack_size(16 << 20)
            .spawn(move || run_embedded_player_corpus(true, false, Some(PlayerProbe::OffboardMarkerDrift(walking))))
            .unwrap().join().unwrap();
    }
}

#[test]
#[ignore = "requires original Skate 2 map and embedded player assets"]
fn skate3_offboard_marker_return_allows_walking() {
    std::thread::Builder::new().name("offboard-marker-movement".into()).stack_size(16 << 20)
        .spawn(|| run_embedded_player_corpus(true, false, Some(PlayerProbe::OffboardMarkerMovement)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 map and embedded player assets"]
fn skate3_online_map_reset_supersedes_unfinished_offboard_continuation() {
    std::thread::Builder::new().name("online-reset-supersession".into()).stack_size(16 << 20)
        .spawn(|| run_embedded_player_corpus(true, false, Some(PlayerProbe::OnlineResetSupersession)))
        .unwrap().join().unwrap();
}

#[test]
#[ignore = "requires original Skate 2 map and embedded player assets"]
fn skate3_city_prop_kicker_has_no_lip() {
    std::thread::Builder::new().name("prop-lip".into()).stack_size(16 << 20)
        .spawn(|| run_embedded_player_corpus(true, true, None))
        .unwrap().join().unwrap();
}
