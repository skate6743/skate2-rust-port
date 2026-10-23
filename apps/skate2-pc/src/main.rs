#[macro_use]
mod diagnostic_output;
mod behaviour_graph_corpus;
mod dmo_hull_resident_probe;
mod global_location_corpus;
mod kd_line_corpus_probe;
mod motion_graph_typed_corpus;
mod play_animation_corpus;
mod recipe_mapping_corpus;
mod robs_park_resident_probe;
mod simulation_collection_runtime_probe;
mod simulation_normal_runtime_probe;
mod simulation_resident_router_probe;
mod state_graph_corpus;
mod state_graph_registry_catalog;
mod state_graph_runtime_corpus;
mod subtitle_manager;
mod juice_platform;
#[cfg(windows)]
mod windows_pad_platform;
mod local_calendar;
mod sdl_pads;
#[cfg(not(windows))]
mod sdl_pad_platform;
#[cfg(windows)]
#[expect(
    dead_code,
    reason = "selected device events await the production session owner"
)]
mod windows_selected_input;

use simulation_collection_runtime_probe::{
    NotificationRuntime as SimulationNotificationCorpusRuntime,
    Runtime as SimulationCollectionCorpusRuntime,
};
use simulation_normal_runtime_probe::SimulationNormalCorpusRuntime;
use skate2_core::animation::{AnimationSkeleton, LocalPose, apply_inverse_bind, multiply_matrices};
use skate2_core::catalog_resident_load_factory::{
    CatalogResidentCollectionLoadFactory, CollectionAssetBuilderSessionFactory,
    CollectionRequestIdAllocator, CollectionRequestIdLease,
};
use skate2_core::character::{CharacterHierarchy, CharacterScene};
use skate2_core::collection_load_pump::LoadedAssetCollection;
use skate2_core::collection_resident_streamer::{
    CollectionId, ResidentCollectionLoadDriver, ResidentCollectionLoadFactory,
};
use skate2_core::decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE};
use skate2_core::dmo_bank_processed_asset::{
    DmoBankProcessedArenaAsset, DmoBankProcessedArenaAssetBuilderSessions,
};
use skate2_core::dynamic_clustered_mesh_template::{
    DynamicClusteredMeshTemplate, DynamicClusteredMeshTemplatePart,
};
use skate2_core::game::{
    CompletedTextureStream, DEBUG_REFERENCE_TEXTURE_STREAM_DEFAULT,
    SECONDARY_LOAD_SUBTITLE_BIG_PATH, WorldStreamBackend, WorldStreamLoads, WorldStreamPoll,
    WorldStreamSpec,
};
use skate2_core::game_stream::{
    GameStreamState, InnerStreamBackend, ManifestOnlyStream, StreamKind,
};
use skate2_core::grind::{
    GrindAsset, GrindBounds, GrindDeckTuning, GrindGeometryCalculation, GrindGeometryTuning,
    GrindPairFilterTuning, GrindPrimitive, LivingWorldGrindBody, LivingWorldGrindScene,
    PreparedGrindGeometry, calculate_grind_geometry, consider_grind_primitive,
    merge_grind_geometry_collision_batches, prepare_grind_geometry,
    transform_dynamic_grind_primitive,
};
use skate2_core::location_arena_adapter::{
    LocationArenaRegistration, LocationArenaRegistrationAdapter, LocationArenaRuntimeSource,
    PreparedLocationArena,
};
use skate2_core::location_arena_plan::{LocationArenaOperation, LocationArenaPlan};
use skate2_core::mounted_character_scene::{
    own_character_scene, own_presentation_arena, own_texture_arena, require_local_entry,
    resolve_local_entry,
};
use skate2_core::mounted_stream_backend::MountedStreamBackend;
use skate2_core::physics::{
    AnimationPhysicsInput, AnimationPhysicsTuning, AnimationTrickData, BallisticTrajectory,
    ContactAnimationAttribute, GroundJumpTuning, GroundStateTuning, JumpPhysicsCurves,
    MotionGraphOutput, PHYSICS_STEP_SECONDS, PointGraph, ProcessedPhysicsInput,
    ScalarAnimationAttribute, TrajectoryPredictionQuery, TrajectoryPredictionTuning,
    TrajectorySelectionTuning, TrajectoryWallRideTuning, TrickFlag, predict_ballistic_trajectory,
};
use skate2_core::player::{
    DEFAULT_MARQUEE_ID, LOCAL_PLAYER_INDEX, MarqueeDefinition, NO_PEER_ID, PlayerLoaderSlot,
};
use skate2_core::presentation::{
    PresentationCollection, PresentationMaterialBinding, PresentationMaterialEffect,
    PresentationMaterialValue, PresentationScene,
};
use skate2_core::render::{NativeRenderQueue, RenderScopeRegistry};
use skate2_core::simulation_collision_asset::{
    SimulationCollisionAsset, SimulationCollisionAssetBuilderSessions,
    SimulationCollisionAssetSessionError,
};
use skate2_core::simulation_dmo_arena_adapter::{
    PreparedSimulationDmoArena, SimulationDmoArenaDispatch, SimulationDmoArenaRegistrationAdapter,
    SimulationDmoLoadPolicy,
};
use skate2_core::simulation_dmo_arena_plan::{
    SimulationDmoArenaPlan, SimulationDmoDataHandle, SimulationDmoLocatorOperation,
};
use skate2_core::simulation_normal_arena_plan::{
    SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRecordHandle,
    SimulationRecordSourceKind, SimulationRegistryAssetKind,
};
use skate2_core::simulation_pointer_arena_adapter::{
    PreparedSimulationPointerArena, PreparedSimulationPointerArenaBank,
    SimulationPointerArenaDispatch, SimulationPointerArenaRegistrationAdapter,
};
use skate2_core::simulation_registry_arena_adapter::{
    PreparedSimulationRegistryArena, SimulationRegistryArenaDispatch,
    SimulationRegistryArenaRegistrationAdapter,
};
use skate2_core::streaming::{
    CollectionRequestKind, CollectionRequestQueue, StreamFocusState, StreamFocusTransition,
};
use skate2_core::texture::{TextureCollection, TextureScene};
use skate2_core::world::{
    CollisionAggregateRef, CollisionArena, CollisionCollection, CollisionInstance, CollisionMesh,
    CollisionModel, CollisionPrimitiveSet, CollisionRoot, CollisionScene,
    CollisionSimplePrimitiveSet, DynamicCollisionBody, DynamicCollisionPart,
    DynamicObjectAdmissionPlan, DynamicObjectAdmissionRejection, DynamicObjectEntityAllocation,
    DynamicObjectEntityAllocator, DynamicObjectPriorityContext, DynamicObjectPriorityIndex,
    DynamicObjectPriorityWeights, LivingWorldEntityIdAllocator, LivingWorldEntityLifetime,
    OwnedDynamicCollisionBody, PrioritizedDynamicCollisionBody, compose_collision_affine,
    dynamic_grind_geometry_batch, dynamic_object_priority_index,
    dynamic_object_reference_cull_order, plan_dynamic_object_admission_from_world,
};
use skate2_core::world_stream_adapter::WorldStreamAdapter;
use skate2_core::world_stream_catalog::{
    CatalogCounts, CollectionRegistrationSnapshot, WorldStreamCatalog,
};
use skate2_core::world_stream_runtime::{
    TextureStreamGate, WorldStreamEffects, WorldStreamRuntime,
};
use skate2_formats::{
    andale::{
        AndaleAnimationController, AndaleCompressionHeader, AndaleDatabase,
        AndalePhaseBlendPoseSample, AndaleRuntimeAttribute,
    },
    attrib::{
        AttributeChunkFile, AttributeCollectionLoadData, AttributeDataFile, AttributeDefinition,
        AttributeExport, AttributePointerKind, AttributePointerLocation, AttributePointerValue,
        AttributeVaultMetadata, string_hash64,
    },
    big::{BigEntryKey, djb2},
    collision::{
        ASSEMBLY_DEFINITION_TYPE_ID, ASSEMBLY_TYPE_ID, Assembly, AssemblyDefinition,
        AssemblyPartDefinition, ClusteredMesh, KdTreeMappedArray, MeshUnit,
        ObjectType as CollisionObjectType, SimpleMappedArray, VertexCompression, Volume,
        VolumeType,
    },
    hash::sk8_hash_value_32,
    location::LocationTable,
    material::{
        MaterialChannelValue, R_MATERIAL_DATA_TYPE_ID, RenderMaterialData, TEXTURE_TYPE_ID,
        Texture, TextureEndian, TextureType,
    },
    mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    pegasus::{
        C_MODEL_DATA_TYPE_ID, CollisionModelData, CollisionModelMeshes, DMO_DATA_TYPE_ID, DmoData,
        EMBEDDED_DATA_TYPE_ID, EmbeddedData, INSTANCE_DATA_TYPE_ID, INSTANCE_DMO_ASSET_TYPE_ID,
        InstanceData, LOCATION_DESC_DATA_TYPE_ID, LocationDesc, LocationDescData, ObjectId,
        SPLINE_DATA_TYPE_ID, SplineData, SubLocationDesc, TABLE_OF_CONTENTS_TYPE_ID,
        TOC_INSTANCE_SUBREFERENCE_TYPE_ID, TOC_LOCATION_DESC_SUBREFERENCE_TYPE_ID,
        TOC_SPLINE_SUBREFERENCE_TYPE_ID, TOC_TRIGGER_INSTANCE_SUBREFERENCE_TYPE_ID,
        TRIGGER_INSTANCE_DATA_TYPE_ID, TableOfContents, TriggerInstanceData,
    },
    recipe::{CharacterAssetKind, CharacterRecipe},
    render::{
        GPU_BUFFER_RESOURCE_TYPE_ID, INDEX_BUFFER_TYPE_ID, IndexBuffer, MESH_HELPER_TYPE_ID,
        MeshHelper, NativeMeshGeometry, NativeVertexAttributeValues, OptimizedRenderMeshData,
        R_MESH_DATA_TYPE_ID, R_MODEL_DATA_TYPE_ID, R_OPTI_MESH_DATA_TYPE_ID, RenderMeshData,
        RenderModelData, VERTEX_BUFFER_TYPE_ID, VERTEX_DESCRIPTOR_TYPE_ID, VertexBuffer,
        VertexDescriptor, VertexSemantic,
    },
    rw4::{Arena as Rw4Arena, ArenaSections},
    shader::ShaderManifest,
    stream::{
        AssetToc, CollectionMap, CollectionSpace, CompressedArenaHeader,
        DEFAULT_PRESENTATION_FOCUS_COLLECTIONS, DEFAULT_SIMULATION_FOCUS_COLLECTIONS,
        DEFAULT_TEXTURE_FOCUS_COLLECTIONS, StreamAssetBlock, StreamFile, TEXTURE_FOCUS_RADIUS,
        WORLD_FOCUS_RADIUS, decode_arena, high_detail_collection_filename,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    io,
    path::PathBuf,
    process::ExitCode,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

const SKATER_HIGH_BLEND_CULL_THRESHOLD: f32 = 0.1;
const LIVING_WORLD_DMO_CHARACTERISTICS_CLASS_ID: u64 = 0xCCB7_C068_68EB_1C87;
const LIVING_WORLD_DMO_PRIORITY_REFERENCE_KEY: u64 = 0x1A3F_B63D_FDAE_0C4F;

mod city_streaming;
mod hull_presentation;
mod dmo_render;
mod gpu_renderer;
mod career_photos;
mod replay_media;
mod media_encoder;
mod gpu_log;
mod panic_report;
mod game_root;
mod game_update;
mod input_fuzz;
mod live_skating_client;
mod live_skating_view;
mod movie_playback;
mod edit_skater_room;
mod game_audio;
mod marker_ui;
#[path = "proxy_kind0_resident_builder_draft_20260923.rs"]
mod proxy_kind0_resident_builder;
#[path = "proxy_shared_loaded_arena_draft_20260923.rs"]
mod proxy_shared_loaded_arena;
mod proxy_texture_world;
mod proxy_world;
mod render_animation_clock;
mod render_environment;
mod render_irradiance;
mod render_inspection;
mod render_material_census;
mod render_shadows;
mod render_water_animation;
mod render_window;
mod render_world_painter;
mod render_postfx;
mod render_postfx_math;
mod vehicle_render;
mod pedestrian_render;
mod visual_indicator_view;
mod particle_view;

/// A bundle's app is started in `/`, where the game cannot keep its files, so a launch from the
/// bundle (Finder, LaunchServices) starts the game in its own folder instead. Started from
/// anywhere else, a terminal or `cargo run`, the folder it starts in is already the game's.
#[cfg(target_os = "macos")]
fn enter_bundle_data_folder() {
    let Some(executable) = std::env::current_exe().ok() else { return };
    if !executable.to_string_lossy().contains("/Contents/MacOS/") {
        return;
    }
    let Some(home) = std::env::var_os("HOME") else { return };
    let data = std::path::Path::new(&home).join("Library/Application Support/Skate 2");
    if let Err(error) = std::fs::create_dir_all(&data).and_then(|()| std::env::set_current_dir(&data)) {
        eprintln!("the game's folder {} cannot be used: {error}", data.display());
    }
}

fn main() -> ExitCode {
    // Before anything reads a relative path: a bundle's launch starts in `/`.
    #[cfg(target_os = "macos")]
    enter_bundle_data_folder();
    panic_report::install();
    gpu_log::install();
    // This executable's build (its build script's secret): its server and its client meet only
    // the same build.
    let build = skate2_net::crypto::Build::from_hex(env!("SKATE2_BUILD_SECRET")).unwrap_or(skate2_net::crypto::Build::DEVELOPMENT);
    build.install();
    if env::args().any(|argument| argument == "--update") {
        return if game_update::update_from_command_line() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }
    if let Some(position) = env::args().position(|argument| argument == "--server") {
        let bind = env::args().nth(position + 1).filter(|argument| !argument.starts_with("--"));
        let Some(bind) = skate2_net::server::parse_bind(bind.as_deref()) else {
            eprintln!("--server takes ip:port, ip or :port");
            return ExitCode::FAILURE;
        };
        game_update::announce_newer_release();
        return match skate2_net::server::run(&skate2_net::server::ServerConfig { bind, build }) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("dedicated server failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    // A player's launch (no arguments, or only `--changelog`): the game, quiet, asking for the
    // game's folder when it is not known yet and showing a fatal error in a message box.
    {
        let player = game_update::player_launch();
        if player || env::args().any(|argument| argument == "--live-skating") {
            skate2_core::diagnostics::set_quiet(player || env::args().any(|argument| argument == "--quiet"));
            skate2_core::diagnostics::read_trace::install();
            let failed = |error: String| {
                std::eprintln!("controlled skating test failed: {error}");
                if player {
                    let gpu = panic_report::gpu().map(|gpu| format!("\n\nGPU: {gpu}")).unwrap_or_default();
                    game_root::show_error(&format!("Skate 2 stopped because of an error:\n\n{error}{gpu}"));
                }
                ExitCode::FAILURE
            };
            // macOS: winit takes NSApplication as its principal class, which the folder dialog
            // below would take first, so the event loop is made before that dialog.
            #[cfg(target_os = "macos")]
            render_window::prepare_event_loop();
            return match game_root::resolve() {
                Ok(Some(root)) => match game_update::play(&root) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(error) => failed(error),
                },
                Ok(None) => ExitCode::SUCCESS,
                Err(error) => failed(error),
            };
        }
    }
    if env::args().any(|argument| argument == "--material-census") {
        return match render_material_census::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("material census failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if env::args().any(|argument| argument == "--render-inspection") {
        return match render_inspection::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("render inspection failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if env::args().any(|argument| argument == "--check-data") {
        return check_data_root();
    }
    let probe_robs_park_resident =
        env::args().any(|argument| argument == "--probe-robs-park-resident");
    if probe_robs_park_resident || env::args().any(|argument| argument == "--probe-world-stream") {
        return match probe_world_stream(probe_robs_park_resident) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("world-stream probe failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let inventory_sim_resident =
        env::args().any(|argument| argument == "--inventory-bam-sim-resident");
    let inventory_sim =
        inventory_sim_resident || env::args().any(|argument| argument == "--inventory-bam-sim");
    let inventory_sim_kd_line = kd_line_corpus_probe::requested(env::args());
    let inventory_pres = env::args().any(|argument| argument == "--inventory-bam-pres");
    let inventory_render = env::args().any(|argument| argument == "--inventory-render-assets");
    let probe_archives_explicit = env::args().any(|argument| argument == "--probe-archives");
    if inventory_sim
        || inventory_sim_kd_line
        || inventory_pres
        || inventory_render
        || probe_archives_explicit
    {
        return probe_archives(ArchiveProbeOptions {
            inventory_sim,
            inventory_sim_resident,
            inventory_sim_kd_line,
            inventory_pres,
            inventory_render,
            probe_archives_explicit,
        });
    }

    eprintln!(
        "the playable runtime is not implemented yet; use --check-data, --probe-world-stream, --probe-robs-park-resident, --inventory-bam-sim-resident, --inventory-bam-sim-kd-line, or --probe-archives"
    );
    ExitCode::FAILURE
}

const WORLD_STREAM_PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const DEBUG_REFERENCE_ENVIRONMENT_VERSION: &[u8] = b"515600\r\n";

type ProbeResult<T> = Result<T, Box<dyn Error>>;
type ProbeInnerBackend = MountedStreamBackend<WorldStreamCatalog>;
type ProbeRuntime = WorldStreamRuntime<DebugReferenceTextureGate, ProbeWorldStreamEffects>;
type ProbeAdapter = WorldStreamAdapter<ProbeInnerBackend, ProbeRuntime>;
type ProbeStreamFile = <ProbeAdapter as WorldStreamBackend>::StreamFile;

#[derive(Clone)]
struct DebugReferenceTextureGate {
    enabled: Arc<AtomicBool>,
    events: Arc<Mutex<Vec<ProbeWorldStreamEvent>>>,
}

impl TextureStreamGate for DebugReferenceTextureGate {
    fn texture_stream_enabled(
        &mut self,
    ) -> Result<bool, skate2_core::world_stream_runtime::TextureStreamGateError> {
        let enabled = self.enabled.load(Ordering::Acquire);
        if let Ok(mut events) = self.events.lock() {
            events.push(ProbeWorldStreamEvent::TextureGateSample { enabled });
        }
        Ok(enabled)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ProbeWorldStreamEvent {
    AddHigh { id: u64, x: u32, z: u32 },
    TextureGateSample { enabled: bool },
    FinalBounds,
}

#[derive(Clone)]
struct ProbeWorldStreamEffects {
    events: Arc<Mutex<Vec<ProbeWorldStreamEvent>>>,
}

impl ProbeWorldStreamEffects {
    fn snapshot(&self) -> ProbeResult<Vec<ProbeWorldStreamEvent>> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| probe_error("world-stream observation event log was poisoned"))
    }
}

impl WorldStreamEffects for ProbeWorldStreamEffects {
    fn add_high_collection(&mut self, id: u64, x: u32, z: u32) {
        if let Ok(mut events) = self.events.lock() {
            events.push(ProbeWorldStreamEvent::AddHigh { id, x, z });
        }
    }

    fn finalize_world_bounds(&mut self) {
        if let Ok(mut events) = self.events.lock() {
            events.push(ProbeWorldStreamEvent::FinalBounds);
        }
    }
}

#[derive(Clone, Copy)]
struct ExpectedManifest {
    kind: StreamKind,
    stamp: u64,
    collections: usize,
    assets: usize,
    spatial: usize,
    manifest_stem: &'static str,
    collection_space_path: &'static str,
    stream_xml_path: &'static str,
    xml_bytes: Option<usize>,
}

const EXPECTED_MANIFESTS: [ExpectedManifest; 3] = [
    ExpectedManifest {
        kind: StreamKind::Presentation,
        stamp: 0x0000_0000_9546_2c2e,
        collections: 285,
        assets: 16_292,
        spatial: 284,
        manifest_stem: "data\\content\\world\\stream\\BAM\\BAM_Pres",
        collection_space_path: "data\\content\\world\\stream\\BAM\\BAM_Pres.xss",
        stream_xml_path: "data\\stream\\BAM_Pres.xml",
        xml_bytes: Some(175_633),
    },
    ExpectedManifest {
        kind: StreamKind::Simulation,
        stamp: 0x0000_0000_9532_ee28,
        collections: 284,
        assets: 2_135,
        spatial: 284,
        manifest_stem: "data\\content\\world\\stream\\BAM\\BAM_Sim",
        collection_space_path: "data\\content\\world\\stream\\BAM\\BAM_Sim.xss",
        stream_xml_path: "data\\stream\\BAM_Sim.xml",
        xml_bytes: Some(172_291),
    },
    ExpectedManifest {
        kind: StreamKind::Texture,
        stamp: 0x0000_0000_9539_8e81,
        collections: 338,
        assets: 5_240,
        spatial: 338,
        manifest_stem: "data\\content\\world\\stream\\BAM\\BAM_Tex",
        collection_space_path: "data\\content\\world\\stream\\BAM\\BAM_Tex.xss",
        stream_xml_path: "data\\stream\\BAM_Tex.xml",
        xml_bytes: None,
    },
];

#[derive(Clone, Copy)]
struct PublicationExpectation {
    files_by_kind: [usize; 3],
    unique_ids: usize,
}

#[derive(Clone, Copy)]
struct UnloadPollExpectation {
    raw_file_state: u32,
    catalog: CatalogCounts,
    publication: PublicationExpectation,
}

// Keeping the one outer-poll loop, completed-owner checks, and diagnostic
// reverse unload adjacent makes accidental extra scheduling calls auditable.
#[allow(clippy::too_many_lines)]
fn probe_world_stream(probe_robs_park_resident: bool) -> ProbeResult<()> {
    let root = env::var_os("SKATE2_GAME_ROOT")
        .map(PathBuf::from)
        .ok_or_else(|| probe_error("SKATE2_GAME_ROOT is not set"))?;
    let mounts = Arc::new(mount_reference_archives(&root)?);
    verify_world_stream_probe_identity(&mounts)?;

    let catalog = WorldStreamCatalog::new();
    let inner = MountedStreamBackend::new(Arc::clone(&mounts), catalog.clone())?;
    let gate_enabled = Arc::new(AtomicBool::new(DEBUG_REFERENCE_TEXTURE_STREAM_DEFAULT));
    let events = Arc::new(Mutex::new(Vec::new()));
    let gate = DebugReferenceTextureGate {
        enabled: Arc::clone(&gate_enabled),
        events: Arc::clone(&events),
    };
    let effects = ProbeWorldStreamEffects {
        events: Arc::clone(&events),
    };
    let effect_observer = effects.clone();
    let runtime = WorldStreamRuntime::new(catalog.clone(), gate, effects);
    let mut adapter = WorldStreamAdapter::new(inner, runtime);
    let mut loads = WorldStreamLoads::debug_reference();
    let spec = WorldStreamSpec::derive("BAM", None)?;
    let handle = loads.start(spec)?;

    let deadline = Instant::now() + WORLD_STREAM_PROBE_TIMEOUT;
    let mut compressed_states = Vec::new();
    loop {
        if Instant::now() >= deadline {
            return Err(probe_error(format!(
                "world-stream load exceeded the host probe deadline in outer state {}",
                loads.state(handle)?.raw()
            )));
        }
        let state = loads.state(handle)?.raw();
        if compressed_states.last().copied() != Some(state) {
            compressed_states.push(state);
        }
        let outcome = loads.poll(handle, &mut adapter)?;
        match outcome {
            WorldStreamPoll::Pending => thread::yield_now(),
            WorldStreamPoll::CompletedThisPoll => {
                require_probe(
                    state == 7,
                    format!("completion pulse came from outer state {state}, not state 7"),
                )?;
                break;
            }
        }
    }
    let terminal_state = loads.state(handle)?.raw();
    if compressed_states.last().copied() != Some(terminal_state) {
        compressed_states.push(terminal_state);
    }
    require_probe(
        compressed_states == [1, 2, 3, 4, 5, 6, 7, 0],
        format!(
            "compressed outer-state trace was {compressed_states:?}, expected 1 through 7 then terminal 0"
        ),
    )?;
    require_probe(
        loads.poll(handle, &mut adapter)? == WorldStreamPoll::Pending
            && loads.state(handle)?.raw() == 0,
        "one extra dormant poll did not remain raw state 0 and Pending",
    )?;

    let completed = loads.take_completed(handle)?;
    require_probe(
        loads.is_empty(),
        "completed BAM record was not moved out once",
    )?;
    let (_, completed_spec, presentation, simulation, texture) = completed.into_parts();
    require_probe(
        completed_spec.basename() == "BAM"
            && completed_spec.stream_path() == "data\\content\\world\\stream",
        "completed record did not retain the derived BAM stream identity",
    )?;
    let texture = match texture {
        CompletedTextureStream::Loaded(file) => file,
        CompletedTextureStream::Disabled => {
            return Err(probe_error(
                "debug-reference live texture gate did not produce the Texture stream",
            ));
        }
    };

    let presentation_ids = validate_loaded_manifest(
        &catalog,
        &presentation,
        EXPECTED_MANIFESTS[StreamKind::Presentation as usize],
    )?;
    let presentation_processed = presentation
        .processed_octree_records()
        .ok_or_else(|| probe_error("missing Presentation processed-prefix order"))?;
    let simulation_ids = validate_loaded_manifest(
        &catalog,
        &simulation,
        EXPECTED_MANIFESTS[StreamKind::Simulation as usize],
    )?;
    let texture_ids = validate_loaded_manifest(
        &catalog,
        &texture,
        EXPECTED_MANIFESTS[StreamKind::Texture as usize],
    )?;
    let authored_ids: BTreeSet<_> = presentation_ids
        .iter()
        .chain(&simulation_ids)
        .chain(&texture_ids)
        .copied()
        .collect();
    require_probe(
        authored_ids.len() == 906,
        format!(
            "the three supplied BAM XSS tables contain {} unique IDs, expected 906",
            authored_ids.len()
        ),
    )?;

    let loaded_counts = CatalogCounts {
        files: 3,
        collections: 907,
        asset_entries: 23_667,
        asset_references: 23_667,
        collection_spaces: 3,
        stream_xml_files: 2,
    };
    require_catalog_counts(&catalog, loaded_counts, "completed load")?;
    adapter = validate_publications(
        adapter,
        PublicationExpectation {
            files_by_kind: [1, 1, 1],
            unique_ids: 906,
        },
        Some(&authored_ids),
        "completed load",
    )?;

    let observed_events = effect_observer.snapshot()?;
    let mut expected_events = Vec::with_capacity(presentation_processed.len() + 2);
    for record in presentation_processed {
        expected_events.push(ProbeWorldStreamEvent::AddHigh {
            id: record.metadata.id,
            x: add_high_coordinate_word(record.metadata.position_words[0])?,
            z: add_high_coordinate_word(record.metadata.position_words[2])?,
        });
    }
    expected_events.push(ProbeWorldStreamEvent::TextureGateSample { enabled: true });
    expected_events.push(ProbeWorldStreamEvent::FinalBounds);
    require_probe(
        observed_events == expected_events,
        format!(
            "ordered world-stream effects differed: observed {} events, expected 286",
            observed_events.len()
        ),
    )?;
    require_probe(
        gate_enabled.load(Ordering::Acquire),
        "debug-reference texture gate changed after its enabled sample",
    )?;

    println!(
        "BAM world-stream load: states {compressed_states:?}; catalog 3 files / 907 collections / 23667 XST references / 23667 unique asset IDs; 906 published XSS IDs"
    );
    println!(
        "effects: 284 Presentation AddHighCollection observations; final bounds observed once"
    );

    if probe_robs_park_resident {
        robs_park_resident_probe::run(Arc::clone(&mounts), catalog.clone())?;
    }

    adapter = unload_probe_file(
        adapter,
        texture,
        &catalog,
        "Texture",
        &texture_unload_expectations(),
    )?;
    adapter = unload_probe_file(
        adapter,
        simulation,
        &catalog,
        "Simulation",
        &simulation_unload_expectations(),
    )?;
    adapter = unload_probe_file(
        adapter,
        presentation,
        &catalog,
        "Presentation",
        &presentation_unload_expectations(),
    )?;
    require_catalog_counts(&catalog, empty_catalog_counts(), "final unload")?;
    adapter = validate_publications(
        adapter,
        PublicationExpectation {
            files_by_kind: [0, 0, 0],
            unique_ids: 0,
        },
        None,
        "final unload",
    )?;
    drop(adapter);

    println!("diagnostic unload: Texture -> Simulation -> Presentation, three polls each");
    if probe_robs_park_resident {
        println!(
            "PASS: BAM manifests and the Rob's Park resident diagnostic completed before manifest teardown"
        );
    } else {
        println!(
            "PASS: this proves BAM manifest loading and local/global publication ownership only; it does not load XSF or Rob's Park cell payloads"
        );
    }
    Ok(())
}

fn mount_reference_archives(root: &std::path::Path) -> ProbeResult<MountSet> {
    let mut mounts = MountSet::new();
    for spec in &SKATE2_ARCHIVES {
        let path = spec.path_under(root);
        if !path.is_file() {
            continue;
        }
        mounts.push(MountedBig::open(path, spec.use_external_header)?);
    }
    require_probe(
        !mounts.archives().is_empty(),
        "no Skate 2 archive tables were available under SKATE2_GAME_ROOT",
    )?;
    Ok(mounts)
}

fn verify_world_stream_probe_identity(mounts: &MountSet) -> ProbeResult<()> {
    let version = mounts.read_path("data\\content\\world\\environmentVersion.txt")?;
    require_probe(
        version == DEBUG_REFERENCE_ENVIRONMENT_VERSION,
        format!(
            "environmentVersion.txt bytes were {version:?}, expected {DEBUG_REFERENCE_ENVIRONMENT_VERSION:?}"
        ),
    )?;
    let locations =
        LocationTable::parse(&mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc")?)?;
    require_probe(
        locations.locations.len() == 787,
        format!(
            "BAM_Sim.loc contains {} locations, expected 787",
            locations.locations.len()
        ),
    )?;
    let robs_park = locations
        .find("A_RobsPark")
        .ok_or_else(|| probe_error("BAM_Sim.loc has no A_RobsPark identity"))?;
    let position_bits = robs_park.position().map(f32::to_bits);
    let expected_position_bits = [
        588.8617_f32.to_bits(),
        (-54.231_647_f32).to_bits(),
        996.5048_f32.to_bits(),
    ];
    require_probe(
        robs_park.name == "A_RobsPark"
            && robs_park.category.as_deref() == Some("DIST_Waterfront")
            && position_bits == expected_position_bits,
        format!(
            "A_RobsPark identity differed: name {:?}, category {:?}, position bits {position_bits:08x?}",
            robs_park.name, robs_park.category
        ),
    )
}

fn validate_loaded_manifest(
    catalog: &WorldStreamCatalog,
    file: &ProbeStreamFile,
    expected: ExpectedManifest,
) -> ProbeResult<BTreeSet<u64>> {
    require_probe(
        file.kind() == expected.kind && file.stream_file_state().raw() == 4,
        format!(
            "{:?} file was not retained as loaded raw state 4",
            expected.kind
        ),
    )?;
    let snapshot = catalog
        .file(file.stream_file_identity().as_ref())?
        .ok_or_else(|| probe_error(format!("missing {:?} catalog file", expected.kind)))?;
    let map = snapshot
        .collection_map
        .as_deref()
        .ok_or_else(|| probe_error(format!("missing {:?} XSM", expected.kind)))?;
    let toc = snapshot
        .asset_toc
        .as_deref()
        .ok_or_else(|| probe_error(format!("missing {:?} XST", expected.kind)))?;
    let space = snapshot
        .collection_space
        .as_deref()
        .ok_or_else(|| probe_error(format!("missing {:?} XSS", expected.kind)))?;
    let paths = snapshot.paths.as_ref();
    let expected_map_path = format!("{}.xsm", expected.manifest_stem);
    let expected_toc_path = format!("{}.xst", expected.manifest_stem);
    require_probe(
        snapshot.kind == expected.kind
            && paths.collection_directory == "data\\content\\world\\stream\\BAM\\"
            && paths.manifest_stem == expected.manifest_stem
            && paths.collection_space == expected.collection_space_path
            && paths.stream_xml == expected.stream_xml_path
            && paths.collection_map() == expected_map_path
            && paths.asset_toc() == expected_toc_path
            && map.version == 3
            && map.stream_format == 2
            && map.stamp == expected.stamp
            && map.collections.len() == expected.collections
            && toc.version == 3
            && toc.stamp == expected.stamp
            && toc.assets.len() == expected.assets
            && space.version == 3
            && space.stamp == expected.stamp
            && space.collections.len() == expected.spatial
            && snapshot.collection_ids.len() == expected.collections
            && snapshot.asset_ids.len() == expected.assets
            && snapshot.collections_active
            && snapshot.assets_active
            && snapshot.stream_xml.as_deref().map(<[u8]>::len) == expected.xml_bytes,
        format!("{:?} manifest metadata/count mismatch", expected.kind),
    )?;
    require_probe(
        space.collections.iter().all(|entry| entry.lod == 0),
        format!("{:?} XSS contains a nonzero LOD entry", expected.kind),
    )?;
    let ids: BTreeSet<_> = space.collections.iter().map(|entry| entry.id).collect();
    require_probe(
        ids.len() == expected.spatial,
        format!("{:?} XSS contains duplicate IDs", expected.kind),
    )?;
    let processed = file
        .processed_octree_storage()
        .ok_or_else(|| probe_error(format!("missing {:?} processed XSS", expected.kind)))?;
    require_probe(
        processed.raw_logical_len() as usize == expected.spatial
            && processed.accepted_prefix().len() == expected.spatial
            && processed.opaque_tail_len() == 0,
        format!("{:?} raw/accepted processed XSS mismatch", expected.kind),
    )?;
    Ok(ids)
}

// The processed position words are the midpoint values installed at
// `0x824e0ff4..0x824e104c`. `AddHighCollection` receives
// `floor(position_axis + 0.5)` as signed i32 bits at `0x824e10ec..0x824e110c`.
// The mounted BAM values are ordinary finite coordinates; exceptional PPC
// conversion parity remains outside this probe just as it does in the adapter.
#[allow(clippy::cast_possible_truncation)]
fn add_high_coordinate_word(position_word: u32) -> ProbeResult<u32> {
    let coordinate = (f32::from_bits(position_word) + 0.5).floor();
    require_probe(
        coordinate.is_finite() && (-2_147_483_648.0..2_147_483_648.0).contains(&coordinate),
        format!("unsupported AddHigh coordinate word {position_word:08x}"),
    )?;
    Ok(u32::from_ne_bytes((coordinate as i32).to_ne_bytes()))
}

fn validate_publications(
    adapter: ProbeAdapter,
    expected: PublicationExpectation,
    exact_ids: Option<&BTreeSet<u64>>,
    stage: &str,
) -> ProbeResult<ProbeAdapter> {
    let (inner, runtime) = adapter.into_parts();
    let mut ids = BTreeSet::new();
    let mut total_records = 0usize;
    for (index, kind) in [
        StreamKind::Presentation,
        StreamKind::Simulation,
        StreamKind::Texture,
    ]
    .into_iter()
    .enumerate()
    {
        let files = runtime.published_files(kind)?;
        require_probe(
            files.len() == expected.files_by_kind[index],
            format!(
                "{stage}: {:?} published-file count was {}, expected {}",
                kind,
                files.len(),
                expected.files_by_kind[index]
            ),
        )?;
        for file in files {
            for record in file.records.accepted_prefix() {
                total_records += 1;
                ids.insert(record.metadata.id);
            }
        }
    }
    require_probe(
        ids.len() == expected.unique_ids && total_records == expected.unique_ids,
        format!(
            "{stage}: publication exposed {total_records} records / {} unique IDs, expected {}",
            ids.len(),
            expected.unique_ids
        ),
    )?;
    if let Some(exact_ids) = exact_ids {
        require_probe(
            &ids == exact_ids,
            format!("{stage}: published ID set did not equal the authored XSS set"),
        )?;
    }
    for id in &ids {
        require_probe(
            runtime.published_record(*id)?.is_some(),
            format!("{stage}: published file record {id:016x} was absent from the ID map"),
        )?;
    }
    Ok(WorldStreamAdapter::new(inner, runtime))
}

fn unload_probe_file(
    mut adapter: ProbeAdapter,
    mut file: ProbeStreamFile,
    catalog: &WorldStreamCatalog,
    label: &str,
    expectations: &[UnloadPollExpectation; 3],
) -> ProbeResult<ProbeAdapter> {
    adapter.request_stream_file_unload(&mut file)?;
    require_probe(
        file.stream_file_state().raw() == 5,
        format!("{label} unload request did not publish raw state 5"),
    )?;
    for (index, expected) in expectations.iter().copied().enumerate() {
        let state = adapter.poll_stream_file(&mut file)?;
        require_probe(
            state.raw() == expected.raw_file_state,
            format!(
                "{label} unload poll {} returned raw {}, expected {}",
                index + 1,
                state.raw(),
                expected.raw_file_state
            ),
        )?;
        require_catalog_counts(
            catalog,
            expected.catalog,
            &format!("{label} poll {}", index + 1),
        )?;
        adapter = validate_publications(
            adapter,
            expected.publication,
            None,
            &format!("{label} poll {}", index + 1),
        )?;
    }
    require_probe(
        !file.is_globally_published() && file.processed_octree_storage().is_none(),
        format!("{label} retained publication/storage after its third unload poll"),
    )?;
    println!("unloaded {label}: raw states [5, 5, 6]");
    Ok(adapter)
}

fn require_catalog_counts(
    catalog: &WorldStreamCatalog,
    expected: CatalogCounts,
    stage: &str,
) -> ProbeResult<()> {
    let actual = catalog.counts()?;
    require_probe(
        actual == expected,
        format!("{stage}: catalog counts were {actual:?}, expected {expected:?}"),
    )
}

const fn empty_catalog_counts() -> CatalogCounts {
    CatalogCounts {
        files: 0,
        collections: 0,
        asset_entries: 0,
        asset_references: 0,
        collection_spaces: 0,
        stream_xml_files: 0,
    }
}

const fn texture_unload_expectations() -> [UnloadPollExpectation; 3] {
    [
        UnloadPollExpectation {
            raw_file_state: 5,
            catalog: CatalogCounts {
                files: 3,
                collections: 907,
                asset_entries: 23_667,
                asset_references: 23_667,
                collection_spaces: 3,
                stream_xml_files: 2,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 1, 1],
                unique_ids: 906,
            },
        },
        UnloadPollExpectation {
            raw_file_state: 5,
            catalog: CatalogCounts {
                files: 3,
                collections: 569,
                asset_entries: 18_427,
                asset_references: 18_427,
                collection_spaces: 3,
                stream_xml_files: 2,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 1, 1],
                unique_ids: 906,
            },
        },
        UnloadPollExpectation {
            raw_file_state: 6,
            catalog: CatalogCounts {
                files: 2,
                collections: 569,
                asset_entries: 18_427,
                asset_references: 18_427,
                collection_spaces: 2,
                stream_xml_files: 2,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 1, 0],
                unique_ids: 568,
            },
        },
    ]
}

const fn simulation_unload_expectations() -> [UnloadPollExpectation; 3] {
    [
        UnloadPollExpectation {
            raw_file_state: 5,
            catalog: CatalogCounts {
                files: 2,
                collections: 569,
                asset_entries: 18_427,
                asset_references: 18_427,
                collection_spaces: 2,
                stream_xml_files: 2,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 1, 0],
                unique_ids: 568,
            },
        },
        UnloadPollExpectation {
            raw_file_state: 5,
            catalog: CatalogCounts {
                files: 2,
                collections: 285,
                asset_entries: 16_292,
                asset_references: 16_292,
                collection_spaces: 2,
                stream_xml_files: 2,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 1, 0],
                unique_ids: 568,
            },
        },
        UnloadPollExpectation {
            raw_file_state: 6,
            catalog: CatalogCounts {
                files: 1,
                collections: 285,
                asset_entries: 16_292,
                asset_references: 16_292,
                collection_spaces: 1,
                stream_xml_files: 1,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 0, 0],
                unique_ids: 284,
            },
        },
    ]
}

const fn presentation_unload_expectations() -> [UnloadPollExpectation; 3] {
    [
        UnloadPollExpectation {
            raw_file_state: 5,
            catalog: CatalogCounts {
                files: 1,
                collections: 285,
                asset_entries: 16_292,
                asset_references: 16_292,
                collection_spaces: 1,
                stream_xml_files: 1,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 0, 0],
                unique_ids: 284,
            },
        },
        UnloadPollExpectation {
            raw_file_state: 5,
            catalog: CatalogCounts {
                files: 1,
                collections: 0,
                asset_entries: 0,
                asset_references: 0,
                collection_spaces: 1,
                stream_xml_files: 1,
            },
            publication: PublicationExpectation {
                files_by_kind: [1, 0, 0],
                unique_ids: 284,
            },
        },
        UnloadPollExpectation {
            raw_file_state: 6,
            catalog: empty_catalog_counts(),
            publication: PublicationExpectation {
                files_by_kind: [0, 0, 0],
                unique_ids: 0,
            },
        },
    ]
}

fn require_probe(condition: bool, message: impl Into<String>) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(probe_error(message))
    }
}

fn probe_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

#[cfg(test)]
mod world_stream_probe_tests {
    use super::*;

    #[test]
    fn debug_reference_texture_gate_reads_live_value_per_sample() {
        let enabled = Arc::new(AtomicBool::new(true));
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut gate = DebugReferenceTextureGate {
            enabled: Arc::clone(&enabled),
            events: Arc::clone(&events),
        };

        assert!(gate.texture_stream_enabled().unwrap());
        enabled.store(false, Ordering::Release);
        assert!(!gate.texture_stream_enabled().unwrap());
        assert_eq!(
            *events.lock().unwrap(),
            [
                ProbeWorldStreamEvent::TextureGateSample { enabled: true },
                ProbeWorldStreamEvent::TextureGateSample { enabled: false },
            ]
        );
    }

    #[test]
    fn add_high_conversion_preserves_signed_words_and_rounding() {
        assert_eq!(add_high_coordinate_word(100.0f32.to_bits()).unwrap(), 100);
        assert_eq!(
            add_high_coordinate_word((-100.0f32).to_bits()).unwrap(),
            u32::from_ne_bytes((-100_i32).to_ne_bytes())
        );
        assert_eq!(add_high_coordinate_word(100.5f32.to_bits()).unwrap(), 101);
    }

    #[test]
    fn main_dmo_manifest_is_created_only_by_its_one_shot_load_request() {
        let mut session = MountedDmoManifestLoadSession::new(Arc::new(MountSet::new())).unwrap();
        assert!(session.manifest.is_none());

        dmo_hull_resident_probe::DmoManifestLoadSession::begin_load(&mut session).unwrap();
        assert_eq!(
            session.manifest().unwrap().state(),
            GameStreamState::BEGIN_MAP_READ
        );
        assert!(dmo_hull_resident_probe::DmoManifestLoadSession::begin_load(&mut session).is_err());
    }
}

#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // These are independent opt-in CLI diagnostics.
struct ArchiveProbeOptions {
    inventory_sim: bool,
    inventory_sim_resident: bool,
    inventory_sim_kd_line: bool,
    inventory_pres: bool,
    inventory_render: bool,
    probe_archives_explicit: bool,
}

fn probe_archives(options: ArchiveProbeOptions) -> ExitCode {
    let ArchiveProbeOptions {
        inventory_sim,
        inventory_sim_resident,
        inventory_sim_kd_line,
        inventory_pres,
        inventory_render,
        probe_archives_explicit,
    } = options;
    let Some(root) = env::var_os("SKATE2_GAME_ROOT").map(PathBuf::from) else {
        eprintln!("SKATE2_GAME_ROOT is not set");
        return ExitCode::FAILURE;
    };

    let mut opened = 0_usize;
    let mut missing = 0_usize;
    let mut entries = 0_usize;
    let mut mounts = MountSet::new();
    for spec in SKATE2_ARCHIVES {
        let path = spec.path_under(&root);
        match MountedBig::open(&path, spec.use_external_header) {
            Ok(mounted) => {
                opened += 1;
                entries += mounted.archive().entries.len();
                println!(
                    "{:02} {:?}: {} entries ({:?})",
                    spec.index,
                    spec.group,
                    mounted.archive().entries.len(),
                    mounted.archive().format
                );
                mounts.push(mounted);
            }
            Err(error) => {
                missing += 1;
                eprintln!("{:02} {:?}: {error}", spec.index, spec.group);
            }
        }
    }
    println!("mounted {opened} archive tables with {entries} entries; {missing} unavailable");

    let mounts = Arc::new(mounts);

    let kd_line_only = inventory_sim_kd_line
        && !inventory_sim
        && !inventory_pres
        && !inventory_render
        && !probe_archives_explicit;
    if kd_line_only {
        if let Err(error) = kd_line_corpus_probe::run(&mounts) {
            eprintln!("BAM Simulation KD line public corpus: {error}");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    probe_bam(&mounts);
    if let Err(error) = probe_default_player_load(&mounts) {
        eprintln!("default local-player load probe: {error}");
        return ExitCode::FAILURE;
    }
    if inventory_sim_kd_line && let Err(error) = kd_line_corpus_probe::run(&mounts) {
        eprintln!("BAM Simulation KD line public corpus: {error}");
        return ExitCode::FAILURE;
    }
    if inventory_render {
        inventory_render_assets(&mounts);
    }
    let dmo_model_ids = if inventory_sim {
        let ids = match inventory_bam_sim(&mounts) {
            Ok(ids) => ids,
            Err(error) => {
                eprintln!("BAM simulation inventory: {error}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(error) = probe_runtime_collision_scene(&mounts, "A_RobsPark") {
            eprintln!("runtime collision scene: {error}");
            return ExitCode::FAILURE;
        }
        if inventory_sim_resident {
            if let Err(error) = simulation_resident_router_probe::run(&mounts) {
                eprintln!("BAM Simulation resident router inventory: {error}");
                return ExitCode::FAILURE;
            }
        }
        Some(ids)
    } else {
        None
    };
    if inventory_pres {
        if let Err(error) = inventory_bam_pres(&mounts, dmo_model_ids.as_ref()) {
            eprintln!("BAM presentation inventory: {error}");
            return ExitCode::FAILURE;
        }
        if let Err(error) = probe_runtime_presentation_scene(&mounts, "A_RobsPark") {
            eprintln!("runtime presentation scene: {error}");
            return ExitCode::FAILURE;
        }
    }

    if opened == 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn inventory_render_assets(mounts: &MountSet) {
    let mut extensions = BTreeMap::<String, usize>::new();
    let mut candidates = Vec::new();
    let mut program_assets = BTreeSet::new();
    let mut attribute_db_assets = Vec::new();
    let mut shaders = ShaderInventory::default();
    for mounted in mounts.archives() {
        for entry in &mounted.archive().entries {
            let BigEntryKey::Name(name) = &entry.key else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            let extension = lower
                .rsplit_once('.')
                .map_or("<none>", |(_, extension)| extension);
            *extensions.entry(extension.to_owned()).or_default() += 1;
            if matches!(extension, "fpo" | "vpo") {
                program_assets.insert(lower.clone());
            }
            if mounted
                .data_path()
                .file_name()
                .is_some_and(|filename| filename.eq_ignore_ascii_case("db.big"))
            {
                attribute_db_assets.push(name.clone());
            }
            if lower.contains("shader")
                || lower.contains("effect")
                || matches!(
                    extension,
                    "fx" | "fxb" | "fxo" | "psh" | "vsh" | "xpu" | "xvu"
                )
            {
                candidates.push(format!("{}::{name}", mounted.data_path().display()));
            }
            if lower.starts_with("data\\render\\shaders\\") && extension == "xml" {
                match mounted
                    .read_path(name)
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| {
                        ShaderManifest::parse(&bytes).map_err(|error| error.to_string())
                    }) {
                    Ok(manifest) => {
                        inventory_shader_manifest(&manifest, &lower, name, mounted, &mut shaders);
                    }
                    Err(error) => shaders.failures.push(format!("{name}: {error}")),
                }
            }
        }
    }
    println!("named archive extensions: {extensions:?}");
    println!("render asset candidates: {}", candidates.len());
    println!("attribute database assets: {attribute_db_assets:?}");
    println!(
        "shader manifests: {} {:?}; {} vertex programs / {} bytes, {} pixel programs / {} bytes, {} techniques / {} passes, {} render states, {} samplers / {} sampler states; {} missing program assets; versions {:?}; failures {:?}",
        shaders.manifests,
        shaders.sets,
        shaders.vertex_programs,
        shaders.vertex_program_bytes,
        shaders.pixel_programs,
        shaders.pixel_program_bytes,
        shaders.techniques,
        shaders.passes,
        shaders.render_states,
        shaders.samplers,
        shaders.sampler_states,
        shaders.missing_program_assets,
        shaders.versions,
        shaders.failures,
    );
    println!("shader render states: {:?}", shaders.render_state_values);
    println!("shader sampler states: {:?}", shaders.sampler_state_values);
    println!("shader sampler names: {:?}", shaders.sampler_names);
    println!(
        "unreferenced shader programs: {:?}",
        program_assets
            .difference(&shaders.referenced_program_assets)
            .collect::<Vec<_>>()
    );
    probe_material_attribute_database(mounts);
}

#[derive(Debug, Default)]
struct CharacterRequestSummary {
    raw_references: usize,
    geometry_requests: usize,
    texture_requests: usize,
    texture_toc_matches: usize,
    unique_paths: BTreeMap<String, CharacterAssetKind>,
}

fn validate_character_requests(
    mounts: &MountSet,
    recipe: Option<&CharacterRecipe>,
    texture_toc: &AssetToc,
) -> Result<CharacterRequestSummary, Box<dyn std::error::Error>> {
    let requests = recipe
        .map(|recipe| recipe.asset_requests("marquee"))
        .unwrap_or_default();
    let archive_paths = mounts
        .archives()
        .iter()
        .flat_map(|mounted| &mounted.archive().entries)
        .filter_map(|entry| match &entry.key {
            BigEntryKey::Name(name) => Some(name.to_ascii_lowercase()),
            BigEntryKey::Hash(_) => None,
        })
        .collect::<BTreeSet<_>>();
    let missing_asset_paths = requests
        .iter()
        .filter(|request| !archive_paths.contains(&request.entry_path.to_ascii_lowercase()))
        .map(|request| request.entry_path.clone())
        .collect::<Vec<_>>();
    let texture_loaded_ids = texture_toc
        .assets
        .iter()
        .map(|asset| asset.id)
        .collect::<BTreeSet<_>>();
    let geometry_requests = requests
        .iter()
        .filter(|request| request.kind == CharacterAssetKind::Geometry)
        .count();
    let texture_requests = requests.len() - geometry_requests;
    let texture_toc_matches = requests
        .iter()
        .filter(|request| {
            request.kind == CharacterAssetKind::Texture
                && texture_loaded_ids.contains(&request.loaded_asset_id)
        })
        .count();
    if !missing_asset_paths.is_empty() {
        return Err(
            format!("missing default character arena paths: {missing_asset_paths:?}").into(),
        );
    }
    if texture_toc_matches != texture_requests {
        return Err(format!(
            "only {texture_toc_matches}/{texture_requests} default character texture IDs match marquee_texture.toc"
        )
        .into());
    }
    let unique_paths = requests
        .iter()
        .map(|request| (request.entry_path.clone(), request.kind))
        .collect::<BTreeMap<_, _>>();
    Ok(CharacterRequestSummary {
        raw_references: recipe.map_or(0, |recipe| recipe.referenced_asset_ids().len()),
        geometry_requests,
        texture_requests,
        texture_toc_matches,
        unique_paths,
    })
}

#[derive(Debug, Default)]
struct CharacterAssetStats {
    arena_bytes: usize,
    arena_objects: usize,
    object_types: BTreeMap<u32, usize>,
    models: usize,
    model_meshes: usize,
    total_bones: usize,
    named_bones: usize,
    inverse_bind_matrices: usize,
    unique_bone_names: BTreeSet<String>,
    meshes: usize,
    skin_palette_entries: usize,
    vertices: u64,
    vertex_attributes: usize,
    indices: u64,
    draw_packets: usize,
    materials: usize,
    material_channels: usize,
    textures: usize,
    texture_levels: usize,
    texture_linear_bytes: u64,
}

fn decode_character_assets(
    mounts: &MountSet,
    requests: &BTreeMap<String, CharacterAssetKind>,
) -> Result<CharacterAssetStats, Box<dyn std::error::Error>> {
    let mut stats = CharacterAssetStats::default();
    for path in requests.keys() {
        let bytes = mounts.read_path(path)?;
        stats.arena_bytes += bytes.len();
        let resources = Rw4Arena::split_file_resources(&bytes)?;
        let arena = Rw4Arena::parse(&resources)?;
        stats.arena_objects += arena.dictionary.len();
        for object in &arena.dictionary {
            *stats.object_types.entry(object.type_id).or_default() += 1;
            let object_bytes = arena
                .object_bytes(&resources, object)
                .ok_or("character arena dictionary object is outside its resource")?;
            match object.type_id {
                R_MODEL_DATA_TYPE_ID => {
                    let model = RenderModelData::parse(object_bytes)?;
                    stats.models += 1;
                    stats.model_meshes += model.meshes.len();
                    stats.total_bones += usize::from(model.total_bone_count);
                    stats.named_bones += usize::from(model.bone_count);
                    stats.inverse_bind_matrices += model.inverse_bind_pose.len();
                    stats.unique_bone_names.extend(model.bone_names);
                }
                R_OPTI_MESH_DATA_TYPE_ID => {
                    let mesh = OptimizedRenderMeshData::parse(object_bytes)?;
                    let geometry = mesh.decode_geometry(&arena, &resources).map_err(|error| {
                        let buffers = character_mesh_buffer_summary(&arena, &resources, &mesh)
                            .unwrap_or_else(|debug_error| debug_error.to_string());
                        format!(
                            "character mesh in {path:?} failed native decoding: {error}; {mesh:?}; {buffers}"
                        )
                    })?;
                    stats.meshes += 1;
                    stats.skin_palette_entries += mesh.remap_indices.len();
                    stats.vertices += u64::try_from(geometry.positions.len())?;
                    stats.vertex_attributes += geometry.attributes.len();
                    stats.indices += u64::try_from(geometry.indices.len())?;
                    stats.draw_packets += geometry.draws.len();
                }
                R_MATERIAL_DATA_TYPE_ID => {
                    let materials = RenderMaterialData::parse(object_bytes)?;
                    stats.materials += materials.materials.len();
                    stats.material_channels += materials.channels.len();
                }
                TEXTURE_TYPE_ID => {
                    let texture = Texture::parse(object_bytes)?;
                    let data_entry = arena
                        .local_entry(texture.data)
                        .ok_or("character texture data is not a local arena object")?;
                    let encoded = arena
                        .object_bytes(&resources, data_entry)
                        .ok_or("character texture data range is invalid")?;
                    let levels = texture.decode_levels(encoded)?;
                    stats.textures += 1;
                    stats.texture_levels += levels.len();
                    stats.texture_linear_bytes += levels
                        .iter()
                        .map(|level| u64::try_from(level.data.len()))
                        .sum::<Result<u64, _>>()?;
                }
                _ => {}
            }
        }
    }
    Ok(stats)
}

fn schema_path_count(mounts: &MountSet, schema_name: &str) -> usize {
    let schema_path_fragment = schema_name.to_ascii_lowercase();
    mounts
        .archives()
        .iter()
        .flat_map(|mounted| &mounted.archive().entries)
        .filter(|entry| {
            matches!(&entry.key, BigEntryKey::Name(name) if name.to_ascii_lowercase().contains(&schema_path_fragment))
        })
        .count()
}

fn validate_default_character_scene(
    scene: &CharacterScene,
) -> Result<(), Box<dyn std::error::Error>> {
    let counts = scene.counts();
    if counts.instances != counts.lods || counts.render_models != counts.lods {
        return Err("default character LODs do not each own one instance/model".into());
    }
    if counts.skin_palette_entries != counts.total_bones {
        return Err("default character packed skin palettes disagree with model totals".into());
    }
    if counts.inverse_bind_matrices != counts.named_bone_slots {
        return Err("default character inverse-bind and named-bone counts disagree".into());
    }
    if counts.unresolved_sampled_channels != 0 {
        return Err("default character has unresolved sampled material channels".into());
    }
    Ok(())
}

fn probe_default_character_skinning(
    mounts: &MountSet,
    scene: &CharacterScene,
) -> Result<(), Box<dyn std::error::Error>> {
    let database_bytes = mounts.read_path("data\\anim\\SkaterPresentationDB.abin")?;
    let database = AndaleDatabase::parse(&database_bytes)?;
    let full_skeleton = AnimationSkeleton::from_andale(&database.hierarchy)?;
    let raw_bind_pose = database
        .poses
        .first()
        .ok_or("full skater presentation database has no bind pose")?
        .sample_raw_sqt(&database.hierarchy)?;
    if let Some((bone, transform)) = raw_bind_pose.iter().enumerate().find(|(_, transform)| {
        transform
            .scale
            .iter()
            .chain(&transform.rotation)
            .chain(&transform.translation)
            .any(|component| !component.is_finite())
    }) {
        return Err(format!(
            "raw bind pose bone {bone} {:?} is non-finite: {transform:?}",
            database.hierarchy.bone_names[bone]
        )
        .into());
    }
    let bind_pose = LocalPose::from_andale(&raw_bind_pose);
    let bind_global_pose = full_skeleton.global_matrices(&bind_pose)?;
    let trick = build_named_trick_pose(mounts, &full_skeleton, bind_pose)?;

    let hierarchy = CharacterHierarchy {
        bone_names: database.hierarchy.bone_names.clone(),
    };
    let lod_levels = scene
        .components
        .iter()
        .flat_map(|component| &component.models)
        .flat_map(|model| &model.lods)
        .map(|lod| lod.level)
        .collect::<BTreeSet<_>>();
    let mut level_summaries = Vec::new();
    let mut bind_validation = Vec::new();
    for level in lod_levels {
        let state = scene.build_skinning_state(level, &hierarchy, "Hips")?;
        let skinned_pose = apply_inverse_bind(
            &trick.physics_input.global_pose,
            &state.hierarchy_inverse_bind_pose,
        )?;
        let mut packed_matrices = 0;
        let mut fallback_bones = 0;
        for (part_index, part) in state.parts.iter().enumerate() {
            packed_matrices += scene
                .pack_skin_matrices(&state, part_index, &skinned_pose)?
                .len();
            fallback_bones += part.fallback_bone_count;
        }
        // Each model's own inverse binds for the bones its meshes skin with.
        // The merged buffer keeps FillInvBPBuffer's last writer per bone,
        // which can be another model's stale matrix (tutorial_male: Pants'
        // right arm twists), exactly as the original skins with it.
        let mut mapped_bones = BTreeSet::new();
        let mut bind_error = 0.0_f32;
        for part in &state.parts {
            let component = &scene.components[part.component_index];
            let lod = &component.models[part.model_index].lods[part.lod_index];
            let model = lod
                .arena
                .instances
                .iter()
                .find_map(|instance| instance.render_model)
                .and_then(|index| lod.arena.models.get(index))
                .ok_or("skinning part lost its render model")?;
            for model_mesh in &model.meshes {
                let mesh = lod
                    .arena
                    .meshes
                    .get(model_mesh.mesh_index)
                    .ok_or("render model mesh is absent")?;
                for &model_bone in &mesh.skin_palette {
                    let model_bone = usize::from(model_bone);
                    let bone = *part
                        .bone_remap
                        .get(model_bone)
                        .ok_or("skin palette entry outside its model")?;
                    // A bone the hierarchy lacks fell back to Hips.
                    if !hierarchy.bone_names[bone]
                        .eq_ignore_ascii_case(&model.bone_names[model_bone])
                    {
                        continue;
                    }
                    mapped_bones.insert(bone);
                    bind_error = bind_error.max(matrix_identity_error(multiply_matrices(
                        bind_global_pose[bone],
                        model.inverse_bind_pose[model_bone],
                    )));
                }
            }
        }
        bind_validation.push((level, mapped_bones.len(), bind_error));
        level_summaries.push((level, state.parts.len(), packed_matrices, fallback_bones));
    }
    println!(
        "native trick presentation pose {:?}: {} raw bind SQT bones + {} body SQT bones across {} named parts -> {} paired animation/physics matrices at {} Hz -> processed compression {}/gravity {}, jump tuning {:?} -> {} packed skin matrices; trick flag {}, runtime attributes {:?}, controller leaves {:?}, duration {}, high-threshold non-endpoint leaves {:?}/attributes {:?}",
        trick.name,
        raw_bind_pose.len(),
        trick.body_bones,
        trick.body_parts,
        trick.physics_input.global_pose.len(),
        1.0 / PHYSICS_STEP_SECONDS,
        trick.processed_input.compression,
        trick.processed_input.gravity,
        trick.physics_tuning,
        level_summaries
            .iter()
            .map(|summary| summary.2)
            .sum::<usize>(),
        trick.physics_input.trick.flags.contains(&TrickFlag::Trick),
        trick.runtime_attributes,
        trick.weighted_clips,
        trick.duration_seconds,
        trick.non_endpoint_weighted_clips,
        trick.non_endpoint_attributes,
    );
    report_physics_tuning(mounts)?;
    println!(
        "bind-pose matrix validation (LOD, mapped bones, maximum global*inverse identity error): {bind_validation:?}"
    );
    if bind_validation.iter().any(|validation| validation.2 > 0.05) {
        return Err("authored bind pose and model inverse-bind matrices disagree".into());
    }
    report_default_andale_database(database_bytes.len(), &database, &level_summaries)?;
    probe_skater_animation_databases(mounts)?;
    Ok(())
}

struct TrickPoseProbe {
    name: String,
    body_bones: usize,
    body_parts: usize,
    physics_input: AnimationPhysicsInput,
    processed_input: ProcessedPhysicsInput,
    physics_tuning: AnimationPhysicsTuning,
    runtime_attributes: Vec<RuntimeAttributeSummary>,
    weighted_clips: Vec<(String, f32)>,
    duration_seconds: f32,
    non_endpoint_weighted_clips: Vec<(String, f32)>,
    non_endpoint_attributes: Vec<RuntimeAttributeSummary>,
}

fn build_named_trick_pose(
    mounts: &MountSet,
    full_skeleton: &AnimationSkeleton,
    mut presentation_pose: LocalPose,
) -> Result<TrickPoseProbe, Box<dyn std::error::Error>> {
    let body_bytes = mounts.read_path("data\\anim\\OnBoard.abin")?;
    let body_database = AndaleDatabase::parse(&body_bytes)?;
    let body_skeleton = AnimationSkeleton::from_andale(&body_database.hierarchy)?;
    // Exact endpoint coordinates select the left-angle, normal-compression
    // child through the same clip attributes consumed by PhaseBlend.
    let endpoint_inputs = [("angle", -1.0), ("compression", 1.0)];
    let sample = body_database.sample_phase_blend_pose_sqt(
        "b_antic_360shuvit_cyc",
        &body_database.hierarchy,
        0.0,
        &endpoint_inputs,
        SKATER_HIGH_BLEND_CULL_THRESHOLD,
    )?;
    let name = sample
        .weighted_clips
        .first()
        .ok_or("phase-blend endpoint produced no clip")?
        .name
        .clone();
    let runtime_attributes = body_database.sample_phase_blend_attributes(
        "b_antic_360shuvit_cyc",
        0.0,
        0.0,
        false,
        &endpoint_inputs,
        SKATER_HIGH_BLEND_CULL_THRESHOLD,
    )?;
    let non_endpoint_inputs = [("angle", -0.75), ("compression", 1.0)];
    let non_endpoint = body_database.sample_phase_blend_pose_sqt(
        "b_antic_360shuvit_cyc",
        &body_database.hierarchy,
        0.25,
        &non_endpoint_inputs,
        SKATER_HIGH_BLEND_CULL_THRESHOLD,
    )?;
    let non_endpoint_attributes = body_database.sample_phase_blend_attributes(
        "b_antic_360shuvit_cyc",
        0.25,
        0.24,
        false,
        &non_endpoint_inputs,
        SKATER_HIGH_BLEND_CULL_THRESHOLD,
    )?;
    validate_body_pose(&non_endpoint, body_database.hierarchy.bone_names.len())?;
    let body_sqt = sample.transforms;
    let body_bones = body_skeleton
        .parts
        .iter()
        .map(|part| part.bones.len())
        .sum::<usize>();
    if body_bones != body_sqt.len() {
        return Err("named body-part ranges do not cover the sampled trick pose".into());
    }
    presentation_pose.overlay_matching_parts(
        full_skeleton,
        &body_skeleton,
        &LocalPose::from_andale(&body_sqt),
    )?;
    let local_matrices = full_skeleton.local_matrices(&presentation_pose)?;
    let global_matrices = full_skeleton.global_matrices(&presentation_pose)?;
    if global_matrices
        .iter()
        .flatten()
        .any(|component| !component.is_finite())
    {
        return Err("named trick produced a non-finite presentation matrix".into());
    }
    let (physics_input, processed_input, physics_tuning) = process_trick_physics(
        mounts,
        full_skeleton,
        &body_database,
        &name,
        &runtime_attributes,
        &local_matrices,
        &global_matrices,
    )?;
    Ok(TrickPoseProbe {
        name,
        body_bones,
        body_parts: body_skeleton.parts.len(),
        physics_input,
        processed_input,
        physics_tuning,
        runtime_attributes: runtime_attribute_summary(&runtime_attributes),
        weighted_clips: sample
            .weighted_clips
            .into_iter()
            .map(|clip| (clip.name, clip.weight))
            .collect(),
        duration_seconds: sample.duration_seconds,
        non_endpoint_weighted_clips: non_endpoint
            .weighted_clips
            .into_iter()
            .map(|clip| (clip.name, clip.weight))
            .collect(),
        non_endpoint_attributes: runtime_attribute_summary(&non_endpoint_attributes),
    })
}

fn process_trick_physics(
    mounts: &MountSet,
    full_skeleton: &AnimationSkeleton,
    body_database: &AndaleDatabase,
    name: &str,
    runtime_attributes: &[AndaleRuntimeAttribute],
    local_matrices: &[[f32; 16]],
    global_matrices: &[[f32; 16]],
) -> Result<
    (
        AnimationPhysicsInput,
        ProcessedPhysicsInput,
        AnimationPhysicsTuning,
    ),
    Box<dyn std::error::Error>,
> {
    let mut physics_input = AnimationPhysicsInput::reset(full_skeleton.parents.len());
    physics_input.set_animation_pose(global_matrices, local_matrices)?;
    configure_contact_context(&mut physics_input, full_skeleton, body_database);
    let mut motion_graph = MotionGraphOutput {
        trick: AnimationTrickData::named_trick(name),
        ..MotionGraphOutput::default()
    };
    motion_graph.drain_into(&mut physics_input);
    append_runtime_animation_attributes(&mut physics_input, runtime_attributes)?;
    if !motion_graph.trick.is_empty() || physics_input.trick.name.as_deref() != Some(name) {
        return Err("motion-graph trick was not transferred as one-shot physics input".into());
    }
    let physics_tuning = load_animation_physics_tuning(mounts)?;
    let mut processed_input = ProcessedPhysicsInput::reset();
    processed_input.process_animation_handoff(&physics_input, physics_tuning);
    Ok((physics_input, processed_input, physics_tuning))
}

type RuntimeAttributeSummary = (String, f32, f32, u8, Option<f32>);

fn runtime_attribute_summary(
    attributes: &[AndaleRuntimeAttribute],
) -> Vec<RuntimeAttributeSummary> {
    attributes
        .iter()
        .map(|attribute| {
            (
                attribute.name.clone(),
                attribute.begin_seconds,
                attribute.end_seconds,
                attribute.status,
                attribute.scalar_value(),
            )
        })
        .collect()
}

fn validate_body_pose(
    sample: &AndalePhaseBlendPoseSample,
    expected_bones: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let has_invalid_component = sample
        .transforms
        .iter()
        .flat_map(|transform| {
            transform
                .scale
                .iter()
                .chain(&transform.rotation)
                .chain(&transform.translation)
        })
        .any(|component| !component.is_finite());
    if sample.transforms.len() != expected_bones || has_invalid_component {
        return Err("non-endpoint phase blend produced an invalid body pose".into());
    }
    Ok(())
}

fn append_runtime_animation_attributes(
    physics_input: &mut AnimationPhysicsInput,
    attributes: &[AndaleRuntimeAttribute],
) -> Result<(), Box<dyn std::error::Error>> {
    for attribute in attributes {
        if let Some(value) = attribute.scalar_value() {
            physics_input
                .scalar_attributes
                .push(ScalarAnimationAttribute::new(&attribute.name, value));
        } else if let Some(contact) = attribute.contact_value()? {
            physics_input
                .contact_attributes
                .push(ContactAnimationAttribute::new(
                    &attribute.name,
                    contact.bone_name,
                    contact.value,
                ));
        }
    }
    Ok(())
}

fn configure_contact_context(
    physics_input: &mut AnimationPhysicsInput,
    full_skeleton: &AnimationSkeleton,
    body_database: &AndaleDatabase,
) {
    physics_input.trajectory_bone_index = full_skeleton.trajectory_bone;
    physics_input.push_reference_bone_name = body_database
        .physics_parameters
        .first()
        .and_then(|parameters| parameters.bones.get(19))
        .map(|bone| bone.name.clone());
}

fn matrix_identity_error(matrix: [f32; 16]) -> f32 {
    matrix
        .iter()
        .enumerate()
        .map(|(index, &component)| {
            let expected = f32::from(index % 5 == 0);
            (component - expected).abs()
        })
        .fold(0.0_f32, f32::max)
}

fn report_default_andale_database(
    byte_count: usize,
    database: &AndaleDatabase,
    level_summaries: &[(u8, usize, usize, usize)],
) -> Result<(), Box<dyn std::error::Error>> {
    let record_summaries = database
        .records
        .iter()
        .map(|record| {
            Ok((
                record.header.data_type,
                record.header.codec,
                record.header.size,
                record.header.name.decode()?,
            ))
        })
        .collect::<Result<Vec<_>, skate2_formats::andale::AndaleError>>()?;
    println!(
        "Andale skater database: {} bytes, file {:?}, records {:?}; hierarchy {} bones / {} parts / trajectory {}, first {:?}",
        byte_count,
        database.file_header.name.decode()?,
        record_summaries,
        database.hierarchy.bone_names.len(),
        database.hierarchy.parts.len(),
        database.hierarchy.has_trajectory,
        database
            .hierarchy
            .bone_names
            .iter()
            .take(12)
            .collect::<Vec<_>>(),
    );
    println!(
        "runtime hierarchy skinning levels (level, parts, packed matrices, fallback bones): {level_summaries:?}"
    );
    println!(
        "Andale hierarchy parts (name, bones, offset, animated, name index): {:?}",
        database
            .hierarchy
            .parts
            .iter()
            .map(|part| (
                &part.name,
                part.bone_count,
                part.bone_offset,
                part.is_animated,
                part.name_index,
            ))
            .collect::<Vec<_>>()
    );
    println!(
        "Andale pose parts (name: flags/header/data/padding bytes): {:?}",
        database.poses.first().map(|pose| {
            pose.parts
                .iter()
                .zip(&database.hierarchy.parts)
                .map(|(pose_part, hierarchy_part)| {
                    let compression = match &pose_part.compression {
                        AndaleCompressionHeader::RawDiscretize(raw) => {
                            format!(
                                "masks={:x?}:stride={}:last={}:formats={:02x?}",
                                raw.channel_masks,
                                raw.frame_stride,
                                raw.last_bone_index,
                                raw.format_flags,
                            )
                        }
                        AndaleCompressionHeader::Vbr(header) => format!(
                            "vbr:f{}:c{}",
                            header.frame_count,
                            header.channel_blocks.len()
                        ),
                        AndaleCompressionHeader::Unknown { codec, bytes } => {
                            format!("{codec:08x}:{}", bytes.len())
                        }
                    };
                    format!(
                        "{}:{:08x}/{compression}/{}/{}/{}",
                        hierarchy_part.name,
                        pose_part.header,
                        pose_part.compression_header.len(),
                        pose_part.compressed_data.len(),
                        pose_part.padding.len(),
                    )
                })
                .collect::<Vec<_>>()
        })
    );
    Ok(())
}

const SKATER_ANIMATION_DATABASE_PATHS: [&str; 9] = [
    "data\\anim\\SkaterFacePres.abin",
    "data\\anim\\SkaterFaceSimPres.abin",
    "data\\anim\\SkaterLeftHandPres.abin",
    "data\\anim\\SkaterRightHandPres.abin",
    "data\\anim\\SkaterBodyPres.abin",
    "data\\anim\\OffBoard.abin",
    "data\\anim\\OnBoard.abin",
    "data\\anim\\SkaterLeftHand.abin",
    "data\\anim\\SkaterRightHand.abin",
];

fn probe_skater_animation_databases(mounts: &MountSet) -> Result<(), Box<dyn std::error::Error>> {
    let mut summaries = Vec::with_capacity(SKATER_ANIMATION_DATABASE_PATHS.len());
    for path in SKATER_ANIMATION_DATABASE_PATHS {
        let bytes = mounts.read_path(path)?;
        let database = AndaleDatabase::parse(&bytes)?;
        report_andale_controller_graph(path, &database);
        let record_types =
            database
                .records
                .iter()
                .fold(BTreeMap::<u32, usize>::new(), |mut counts, record| {
                    *counts.entry(record.header.data_type).or_default() += 1;
                    counts
                });
        let clip_codecs =
            database
                .clips
                .iter()
                .fold(BTreeMap::<u32, usize>::new(), |mut counts, clip| {
                    *counts.entry(clip.codec).or_default() += 1;
                    counts
                });
        let looping_clips = database.clips.iter().filter(|clip| clip.looping()).count();
        let clip_duration_range = database
            .clips
            .iter()
            .map(skate2_formats::andale::AndaleClip::duration_seconds)
            .fold(None, |range: Option<(f32, f32)>, duration| {
                Some(range.map_or((duration, duration), |(minimum, maximum)| {
                    (minimum.min(duration), maximum.max(duration))
                }))
            });
        let sample_clip_names = database
            .clips
            .iter()
            .take(8)
            .map(|clip| clip.name.clone())
            .collect::<Vec<_>>();
        let first_vbr_header =
            database
                .poses
                .iter()
                .flat_map(|pose| &pose.parts)
                .find_map(|part| match &part.compression {
                    AndaleCompressionHeader::Vbr(header) => Some((
                        header.frame_count,
                        header.constant_channel_map_run_count,
                        header.constant_palette.len(),
                        header.frame_block_sizes.clone(),
                        header
                            .channel_blocks
                            .iter()
                            .map(|block| {
                                (
                                    block.animated_entry_count,
                                    block.constant_entry_count,
                                    block.component_count,
                                )
                            })
                            .collect::<Vec<_>>(),
                    )),
                    AndaleCompressionHeader::RawDiscretize(_)
                    | AndaleCompressionHeader::Unknown { .. } => None,
                });
        let constant_vbr_sqt_bones = constant_vbr_sqt_bone_count(&database);
        report_andale_contact_attributes(path, &database)?;
        report_vbr_frame_block_failures(path, &database);
        report_named_onboard_clip(path, &database)?;
        summaries.push((
            path,
            bytes.len(),
            database.hierarchy.bone_names.len(),
            database.hierarchy.parts.len(),
            database.clips.len(),
            database.poses.len(),
            looping_clips,
            clip_duration_range,
            clip_codecs,
            sample_clip_names,
            record_types,
            (
                first_vbr_header,
                constant_vbr_sqt_bones,
                vbr_clip_frame_layout(&database),
            ),
        ));
    }
    println!(
        "supporting skater Andale databases (path, bytes, bones, parts, clips, poses, looping, duration range, clip codecs, sample clips, record types, [first VBR header summary, constant VBR SQT bones, clip VBR frame blocks/bytes/trailing]): {summaries:?}"
    );
    Ok(())
}

fn report_andale_contact_attributes(
    path: &str,
    database: &AndaleDatabase,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut contacts = BTreeMap::<(String, String, u32), usize>::new();
    for attribute in database.clips.iter().flat_map(|clip| &clip.attributes) {
        let Some(contact) = attribute.value.contact_value()? else {
            continue;
        };
        *contacts
            .entry((
                attribute.name.clone(),
                contact.bone_name,
                contact.value.to_bits(),
            ))
            .or_default() += 1;
    }
    if !contacts.is_empty() {
        let readable = contacts
            .into_iter()
            .map(|((name, bone, value), count)| (name, bone, f32::from_bits(value), count))
            .collect::<Vec<_>>();
        println!("Andale contact attributes {path}: {readable:?}");
    }
    Ok(())
}

fn report_andale_controller_graph(path: &str, database: &AndaleDatabase) {
    if database.controllers.is_empty() && database.physics_parameters.is_empty() {
        return;
    }
    let controller_names = database
        .controllers
        .iter()
        .map(AndaleAnimationController::name)
        .collect::<BTreeSet<_>>();
    let available_names = database
        .clips
        .iter()
        .map(|clip| clip.name.as_str())
        .chain(controller_names.iter().copied())
        .collect::<BTreeSet<_>>();
    let referenced_names = database
        .controllers
        .iter()
        .flat_map(AndaleAnimationController::child_names)
        .collect::<BTreeSet<_>>();
    let unresolved = referenced_names
        .difference(&available_names)
        .copied()
        .collect::<Vec<_>>();
    let roots = controller_names
        .difference(&referenced_names)
        .copied()
        .collect::<Vec<_>>();
    let type_counts = database.controllers.iter().fold(
        BTreeMap::<&str, usize>::new(),
        |mut counts, controller| {
            let kind = match controller {
                AndaleAnimationController::BlendSpace(_) => "blend-space",
                AndaleAnimationController::PhaseBlend(_) => "phase-blend",
                AndaleAnimationController::ConstructionSelector(_) => "construction-selector",
                AndaleAnimationController::Sequence(_) => "sequence",
                AndaleAnimationController::Selector(_) => "selector",
            };
            *counts.entry(kind).or_default() += 1;
            counts
        },
    );
    let physics_bones = database
        .physics_parameters
        .iter()
        .map(|parameters| parameters.bones.len())
        .sum::<usize>();
    let physics_bone_names = database
        .physics_parameters
        .iter()
        .flat_map(|parameters| parameters.bones.iter().map(|bone| bone.name.as_str()))
        .collect::<Vec<_>>();
    let trick_path = find_controller_path(database, "r_antic_360shuvit_l_0_cyc");
    println!(
        "Andale controller graph {path}: {} typed nodes {type_counts:?}, {} edges, {} roots (first {:?}), {} unresolved references (first {:?}), {} physics bones {physics_bone_names:?}, named trick path {:?}",
        database.controllers.len(),
        referenced_names.len(),
        roots.len(),
        roots.iter().take(8).collect::<Vec<_>>(),
        unresolved.len(),
        unresolved.iter().take(8).collect::<Vec<_>>(),
        physics_bones,
        trick_path,
    );
    if let Some(path) = &trick_path {
        let path_details = path
            .iter()
            .filter_map(|name| {
                database
                    .controllers
                    .iter()
                    .find(|controller| controller.name() == name)
            })
            .map(|controller| describe_andale_controller(database, controller))
            .collect::<Vec<_>>();
        println!("Andale named trick controller route details: {path_details:?}");
    }
}

fn describe_andale_controller(
    database: &AndaleDatabase,
    controller: &AndaleAnimationController,
) -> String {
    match controller {
        AndaleAnimationController::BlendSpace(value) => format!(
            "{} blend-space animations={:?}",
            value.name, value.animation_names
        ),
        AndaleAnimationController::PhaseBlend(value) => format!(
            "{} phase-blend attribute={:?} children={:?} child coordinates={:?}",
            value.name,
            value.blend_attribute,
            value.children,
            value
                .children
                .iter()
                .map(|child| collect_static_scalar_values(database, child, &value.blend_attribute))
                .collect::<Vec<_>>()
        ),
        AndaleAnimationController::ConstructionSelector(value) => format!(
            "{} construction attribute={:?} options={:?}",
            value.name, value.construction_attribute, value.options
        ),
        AndaleAnimationController::Sequence(value) => {
            format!("{} sequence children={:?}", value.name, value.children)
        }
        AndaleAnimationController::Selector(value) => {
            format!("{} selector children={:?}", value.name, value.children)
        }
    }
}

fn collect_static_scalar_values(
    database: &AndaleDatabase,
    name: &str,
    attribute: &str,
) -> Vec<f32> {
    if let Some(clip) = database.clips.iter().find(|clip| clip.name == name) {
        return clip
            .attributes
            .iter()
            .filter(|value| {
                value.name == attribute && value.begin.to_bits() == (-1.0_f32).to_bits()
            })
            .filter_map(skate2_formats::andale::AndaleAttribute::scalar_value)
            .collect();
    }
    let Some(controller) = database
        .controllers
        .iter()
        .find(|controller| controller.name() == name)
    else {
        return Vec::new();
    };
    let mut values = controller
        .child_names()
        .into_iter()
        .flat_map(|child| collect_static_scalar_values(database, child, attribute))
        .collect::<Vec<_>>();
    values.sort_by(f32::total_cmp);
    values.dedup_by(|left, right| left.to_bits() == right.to_bits());
    values
}

fn find_controller_path(database: &AndaleDatabase, target: &str) -> Option<Vec<String>> {
    let controllers = database
        .controllers
        .iter()
        .map(|controller| (controller.name(), controller))
        .collect::<BTreeMap<_, _>>();
    let referenced = database
        .controllers
        .iter()
        .flat_map(AndaleAnimationController::child_names)
        .collect::<BTreeSet<_>>();
    let roots = controllers
        .keys()
        .copied()
        .filter(|name| !referenced.contains(name))
        .chain(controllers.keys().copied())
        .collect::<Vec<_>>();
    for root in roots {
        let mut visiting = BTreeSet::new();
        if let Some(path) = find_controller_path_from(root, target, &controllers, &mut visiting) {
            return Some(path);
        }
    }
    None
}

fn find_controller_path_from(
    name: &str,
    target: &str,
    controllers: &BTreeMap<&str, &AndaleAnimationController>,
    visiting: &mut BTreeSet<String>,
) -> Option<Vec<String>> {
    if !visiting.insert(name.to_owned()) {
        return None;
    }
    let controller = controllers.get(name)?;
    for child in controller.child_names() {
        if child == target {
            return Some(vec![name.to_owned(), target.to_owned()]);
        }
        if controllers.contains_key(child) {
            if let Some(mut path) = find_controller_path_from(child, target, controllers, visiting)
            {
                path.insert(0, name.to_owned());
                return Some(path);
            }
        }
    }
    visiting.remove(name);
    None
}

fn report_named_onboard_clip(
    path: &str,
    database: &AndaleDatabase,
) -> Result<(), skate2_formats::andale::AndaleError> {
    if path.ends_with("OnBoard.abin") {
        report_named_vbr_clip(database, "r_antic_360shuvit_l_0_cyc")?;
    }
    Ok(())
}

fn report_named_vbr_clip(
    database: &AndaleDatabase,
    name: &str,
) -> Result<(), skate2_formats::andale::AndaleError> {
    let Some(clip) = database.clips.iter().find(|clip| clip.name == name) else {
        return Ok(());
    };
    let mut summaries = Vec::with_capacity(clip.parts.len());
    let mut pose = Vec::new();
    for (part_index, part) in clip.parts.iter().enumerate() {
        let sqt = part.sample_vbr_sqt_interpolated(0, 1, 0.0)?;
        let quaternion_norm_range = sqt
            .iter()
            .map(|transform| {
                transform
                    .rotation
                    .iter()
                    .map(|component| component * component)
                    .sum::<f32>()
                    .sqrt()
            })
            .fold((f32::MAX, f32::MIN), |range, length| {
                (range.0.min(length), range.1.max(length))
            });
        summaries.push((
            part_index,
            part.part_name_index,
            sqt.len(),
            sqt.first().copied(),
            quaternion_norm_range,
        ));
        pose.extend(sqt);
    }
    let first_named_bones = database
        .hierarchy
        .bone_names
        .iter()
        .zip(&pose)
        .take(4)
        .collect::<Vec<_>>();
    println!(
        "named VBR clip {name:?} blended frame 0: {} SQT bones / {} hierarchy bones, first named bones {first_named_bones:?}, parts {summaries:?}",
        pose.len(),
        database.hierarchy.bone_names.len()
    );
    Ok(())
}

fn constant_vbr_sqt_bone_count(database: &AndaleDatabase) -> usize {
    database
        .poses
        .iter()
        .flat_map(|pose| &pose.parts)
        .filter_map(skate2_formats::andale::AndaleAnimationPart::constant_vbr_sqt)
        .map(|sqt| sqt.len())
        .sum()
}

fn report_vbr_frame_block_failures(path: &str, database: &AndaleDatabase) {
    let failures = vbr_frame_block_failures(database);
    if !failures.is_empty() {
        eprintln!(
            "VBR frame-prefix exceptions in {path} (first eight): {:?}",
            failures.iter().take(8).collect::<Vec<_>>()
        );
    }
}

fn vbr_frame_block_failures(database: &AndaleDatabase) -> Vec<String> {
    let mut failures = Vec::new();
    for clip in &database.clips {
        for (part_index, part) in clip.parts.iter().enumerate() {
            let (AndaleCompressionHeader::Vbr(header), Some(prelude)) =
                (&part.compression, &part.vbr_prelude)
            else {
                continue;
            };
            for (block_index, range) in prelude.frame_blocks.iter().enumerate() {
                if let Err(error) =
                    header.decode_frame_block(prelude, &part.compressed_data, block_index)
                {
                    let prefix_end = (range.start + 16).min(range.end);
                    failures.push(format!(
                        "{} part {part_index} block {block_index} prefix {:02x?}: {error:?}",
                        clip.name,
                        &part.compressed_data[range.start..prefix_end]
                    ));
                }
            }
        }
    }
    failures
}

fn vbr_clip_frame_layout(
    database: &AndaleDatabase,
) -> (usize, usize, usize, usize, usize, usize, usize) {
    database
        .clips
        .iter()
        .flat_map(|clip| &clip.parts)
        .filter_map(|part| match (&part.compression, &part.vbr_prelude) {
            (AndaleCompressionHeader::Vbr(header), Some(prelude)) => Some((part, header, prelude)),
            _ => None,
        })
        .fold(
            (
                0_usize, 0_usize, 0_usize, 0_usize, 0_usize, 0_usize, 0_usize,
            ),
            |summary, (part, header, prelude)| {
                let (valid_blocks, coefficient_bytes) = prelude
                    .frame_blocks
                    .iter()
                    .enumerate()
                    .filter_map(|(index, _)| {
                        header
                            .decode_frame_block(prelude, &part.compressed_data, index)
                            .ok()
                            .filter(|block| {
                                header
                                    .unpack_frame_block_coefficients(prelude, block)
                                    .is_ok()
                            })
                    })
                    .fold((0_usize, 0_usize), |counts, block| {
                        (counts.0 + 1, counts.1 + block.coefficient_payload.len())
                    });
                (
                    summary.0 + prelude.frame_blocks.len(),
                    summary.1
                        + prelude
                            .frame_blocks
                            .iter()
                            .map(std::ops::Range::len)
                            .sum::<usize>(),
                    summary.2 + prelude.trailing_bytes.len(),
                    summary.3 + valid_blocks,
                    summary.4 + coefficient_bytes,
                    summary.5.max(
                        prelude
                            .used_table
                            .iter()
                            .map(|byte| usize::from(byte & 0x0f != 0) + usize::from(byte >> 4 != 0))
                            .sum::<usize>(),
                    ),
                    summary.6
                        + usize::from(
                            prelude
                                .used_table
                                .iter()
                                .map(|byte| {
                                    usize::from(byte & 0x0f != 0) + usize::from(byte >> 4 != 0)
                                })
                                .sum::<usize>()
                                > usize::from(u8::MAX),
                        ) * prelude.frame_blocks.len(),
                )
            },
        )
}

fn report_character_asset_graph(assets: &CharacterAssetStats, scene: Option<&CharacterScene>) {
    println!(
        "character arenas: {} bytes / {} objects {:?}",
        assets.arena_bytes, assets.arena_objects, assets.object_types,
    );
    println!(
        "native character assets: {} models / {} mesh links / {} total bones / {} named bone slots / {} inverse-bind matrices / {} unique bone names, {} meshes / {} skin-palette entries / {} vertices / {} attribute arrays / {} indices / {} draws, {} materials / {} channels, {} textures / {} level surfaces / {} linear bytes",
        assets.models,
        assets.model_meshes,
        assets.total_bones,
        assets.named_bones,
        assets.inverse_bind_matrices,
        assets.unique_bone_names.len(),
        assets.meshes,
        assets.skin_palette_entries,
        assets.vertices,
        assets.vertex_attributes,
        assets.indices,
        assets.draw_packets,
        assets.materials,
        assets.material_channels,
        assets.textures,
        assets.texture_levels,
        assets.texture_linear_bytes,
    );
    println!(
        "runtime-owned character graph: {:?}",
        scene.map(CharacterScene::counts)
    );
}

fn probe_default_player_load(mounts: &MountSet) -> Result<(), Box<dyn std::error::Error>> {
    let definition = load_default_marquee_definition(mounts)?;
    let recipe_path = definition.binary_recipe_path();
    let recipe_bytes = mounts.read_path(&recipe_path).ok();
    let schema_asset_id = definition.schema_asset_id();
    let model_toc = AssetToc::parse(&mounts.read_path("marquee_model.toc")?)?;
    let preloaded_schema = model_toc
        .assets
        .iter()
        .any(|asset| asset.id == schema_asset_id);
    if recipe_bytes.is_none() && !preloaded_schema {
        return Err(format!(
            "default schema {schema_asset_id:016x} is neither a loose recipe nor in marquee_model.toc"
        )
        .into());
    }
    let recipe = recipe_bytes
        .as_deref()
        .map(CharacterRecipe::parse)
        .transpose()?;
    if recipe
        .as_ref()
        .is_some_and(|recipe| recipe.schema_name != definition.schema_name)
    {
        return Err("default marquee definition and recipe schema names disagree".into());
    }
    let texture_toc = AssetToc::parse(&mounts.read_path("marquee_texture.toc")?)?;
    let requests = validate_character_requests(mounts, recipe.as_ref(), &texture_toc)?;
    let assets = decode_character_assets(mounts, &requests.unique_paths)?;
    let character_scene = recipe
        .as_ref()
        .map(|recipe| own_character_scene(mounts, recipe))
        .transpose()?;
    if let Some(scene) = &character_scene {
        validate_default_character_scene(scene)?;
        probe_default_character_skinning(mounts, scene)?;
    }
    let schema_path_fragment = definition.schema_name.to_ascii_lowercase();
    let schema_paths = schema_path_count(mounts, &schema_path_fragment);
    let mut loader = PlayerLoaderSlot::new(LOCAL_PLAYER_INDEX);
    loader.set_marquee(definition.id, NO_PEER_ID);
    let load = loader.begin_marquee_load(&definition, 1)?;
    let attributes_pending = loader.character_loaded(1, 0xFFFF_0000_0000_0001, 2)?;
    println!(
        "default local-player marquee: {:016x} {:?}, schema asset {:016x}, recipe {:?} loose bytes {:?}, parsed {:?}",
        definition.id,
        definition.schema_name,
        schema_asset_id,
        recipe_path,
        recipe_bytes.as_ref().map(Vec::len),
        recipe.as_ref().map(|recipe| (
            recipe.version,
            recipe.category,
            recipe.memory_category,
            recipe.clone_type,
            recipe.model_count(),
            recipe.lod_count(),
            recipe.material_instance_count(),
            recipe.material_count(),
            recipe.shader_parameter_count(),
            recipe.packed_size,
        )),
    );
    println!(
        "components {:?}; {} raw references -> {} geometry + {} textures ({} unique), all paths present, texture TOC IDs {}/{}",
        recipe.as_ref().map(|recipe| recipe
            .components
            .iter()
            .map(|part| &part.name)
            .collect::<Vec<_>>()),
        requests.raw_references,
        requests.geometry_requests,
        requests.texture_requests,
        requests.unique_paths.len(),
        requests.texture_toc_matches,
        requests.texture_requests,
    );
    report_character_asset_graph(&assets, character_scene.as_ref());
    println!(
        "TOCs model/texture {}/{}, {} schema paths, preloaded {}; protocol dry-run {:?} -> {:?} (native attribute services required)",
        model_toc.assets.len(),
        texture_toc.assets.len(),
        schema_paths,
        preloaded_schema,
        load,
        attributes_pending
    );
    Ok(())
}

fn character_mesh_buffer_summary(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    mesh: &OptimizedRenderMeshData,
) -> Result<String, Box<dyn std::error::Error>> {
    let descriptor_id = mesh.vertex_descriptor.ok_or("missing vertex descriptor")?;
    let helper_id = mesh.mesh_helper.ok_or("missing mesh helper")?;
    let index_id = mesh.index_buffer.ok_or("missing index buffer")?;
    let descriptor_entry = arena
        .local_entry(descriptor_id)
        .ok_or("vertex descriptor is not local")?;
    let helper_entry = arena
        .local_entry(helper_id)
        .ok_or("mesh helper is not local")?;
    let index_entry = arena
        .local_entry(index_id)
        .ok_or("index buffer is not local")?;
    let descriptor = VertexDescriptor::parse(
        arena
            .object_bytes(resources, descriptor_entry)
            .ok_or("invalid vertex descriptor range")?,
    )?;
    let helper = MeshHelper::parse(
        arena
            .object_bytes(resources, helper_entry)
            .ok_or("invalid mesh helper range")?,
    )?;
    let position = descriptor
        .position_element()
        .ok_or("missing position declaration")?;
    let stride = *descriptor
        .stream_strides()?
        .get(&position.stream)
        .ok_or("missing position stride")?;
    let vertex_id = *helper
        .vertex_buffers
        .get(usize::from(position.stream))
        .ok_or("missing position buffer")?;
    let vertex_entry = arena
        .local_entry(vertex_id)
        .ok_or("vertex buffer is not local")?;
    let vertex = VertexBuffer::parse(
        arena
            .object_bytes(resources, vertex_entry)
            .ok_or("invalid vertex buffer range")?,
    )?;
    let index = IndexBuffer::parse(
        arena
            .object_bytes(resources, index_entry)
            .ok_or("invalid index buffer range")?,
    )?;
    let index_object = arena
        .object_bytes(resources, index_entry)
        .ok_or("invalid index buffer range")?;
    let index_data_entry = arena
        .local_entry(index.data)
        .ok_or("index data is not local")?;
    let index_data = arena
        .object_bytes(resources, index_data_entry)
        .ok_or("invalid index data range")?;
    let big_endian = index.decode_indices(index_data)?;
    let vertex_count = vertex.buffer_size / u32::from(stride);
    let in_range_be = big_endian
        .iter()
        .filter(|value| **value < vertex_count)
        .count();
    let little_endian_max = if index.index_depth == 16 {
        index_data
            .chunks_exact(2)
            .take(usize::try_from(index.index_count)?)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .max()
            .map(u32::from)
    } else {
        index_data
            .chunks_exact(4)
            .take(usize::try_from(index.index_count)?)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("four bytes")))
            .max()
    };
    Ok(format!(
        "arena resources {:?} at {:?}; position {:?}, stride {stride}, vertex bytes {} ({vertex_count} vertices), index common {:08x}, depth {} count {} in-range BE {in_range_be}, max BE {:?}/LE {:?}, data entry {:?}, raw {:02x?}",
        arena.resource_descriptor,
        arena.resource_offsets,
        position,
        vertex.buffer_size,
        u32::from_be_bytes(index_object[..4].try_into().expect("index common word")),
        index.index_depth,
        index.index_count,
        big_endian.iter().max(),
        little_endian_max,
        index_data_entry,
        &index_data[..index_data.len().min(32)],
    ))
}

fn load_default_marquee_definition(
    mounts: &MountSet,
) -> Result<MarqueeDefinition, Box<dyn std::error::Error>> {
    const CHARACTERS_MARQUEE_CLASS_ID: u64 = 0x91BC_A669_3EFC_9AA7;

    if string_hash64("characters_marquee") != CHARACTERS_MARQUEE_CLASS_ID {
        return Err("characters_marquee class hash disagrees with executable".into());
    }
    let vlt = mounts.read_path("data\\db\\skater.vlt")?;
    let bin = mounts.read_path("data\\db\\skater.bin")?;
    let vault = AttributeVaultMetadata::parse(&vlt)?;
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()])?;
    let buffers = [vlt.as_slice(), bin.as_slice()];
    let database_export = vault
        .exports
        .iter()
        .find(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Database)
        .ok_or("missing attribute database export")?;
    let database = database_export.database_load_data(&vlt)?;
    let type_names_slot = database_export
        .data_offset
        .checked_add(12)
        .ok_or("attribute type-name slot overflow")?;
    let (type_names_dependency, type_names_offset) =
        attribute_direct_pointer(&pointers, 0, type_names_slot)
            .ok_or("attribute type-name pointer is null")?;
    let type_names = attribute_string_sequence(
        &buffers,
        type_names_dependency,
        type_names_offset,
        usize::try_from(database.type_count)?,
    )
    .ok_or("attribute type-name table is invalid")?;
    let classes = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Class)
        .map(|export| (export.object_id, export))
        .collect::<BTreeMap<_, _>>();
    let collections = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Collection)
        .map(|export| {
            let collection = export.collection_load_data(&vlt)?;
            Ok((
                (collection.class_id, collection.collection_id),
                (export, collection),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, skate2_formats::attrib::AttributeChunkError>>()?;
    let class = classes
        .get(&CHARACTERS_MARQUEE_CLASS_ID)
        .ok_or("missing characters_marquee class")?;
    let (collection_export, _collection) = collections
        .get(&(CHARACTERS_MARQUEE_CLASS_ID, DEFAULT_MARQUEE_ID))
        .ok_or("missing default characters_marquee collection")?;
    let definitions = class.class_definitions(&buffers, &pointers)?;
    let name = definitions
        .iter()
        .find(|definition| definition.key == string_hash64("name"))
        .or_else(|| {
            definitions
                .iter()
                .find(|definition| definition.offset == 0 && definition.size == 4)
        })
        .ok_or("characters_marquee has no layout-offset-zero name definition")?;
    let name_type = type_names
        .iter()
        .find(|type_name| string_hash64(type_name) == name.type_id)
        .map_or("<unknown>", String::as_str);
    if name_type != "EA::Reflection::Text" || name.offset != 0 {
        return Err("unexpected characters_marquee name layout".into());
    }
    // GetSkaterName reads mLayoutPtr directly rather than calling GetNode.
    // Follow CollectionLoadData::mLayout and then the name pointer at layout
    // offset zero exactly as the generated accessor does.
    let layout_pointer_slot = collection_export
        .data_offset
        .checked_add(40)
        .ok_or("default marquee layout slot overflow")?;
    let (layout_dependency, layout_offset) =
        attribute_direct_pointer(&pointers, 0, layout_pointer_slot)
            .ok_or("default marquee layout pointer is null")?;
    let name_pointer_offset = layout_offset
        .checked_add(u64::from(name.offset))
        .ok_or("default marquee name pointer offset overflow")?;
    let (string_dependency, string_offset) =
        attribute_direct_pointer_u64(&pointers, layout_dependency, name_pointer_offset)
            .ok_or("default marquee layout name pointer is null")?;
    let schema_name = attribute_string(&buffers, string_dependency, string_offset)
        .ok_or("default marquee name string is invalid")?;
    Ok(MarqueeDefinition {
        id: DEFAULT_MARQUEE_ID,
        schema_name,
    })
}

fn attribute_string_sequence(
    buffers: &[&[u8]],
    dependency_index: u16,
    offset: u64,
    count: usize,
) -> Option<Vec<String>> {
    let bytes = *buffers.get(usize::from(dependency_index))?;
    let mut tail = bytes.get(usize::try_from(offset).ok()?..)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let end = tail.iter().position(|byte| *byte == 0)?;
        values.push(std::str::from_utf8(&tail[..end]).ok()?.to_owned());
        tail = tail.get(end + 1..)?;
    }
    Some(values)
}

fn probe_material_attribute_database(mounts: &MountSet) {
    let attribute_bin = mounts.read_path("data\\db\\skater.bin").ok();
    let generated_material_classes = attribute_bin
        .as_deref()
        .map(generated_material_class_names)
        .unwrap_or_default();
    for path in ["data\\db\\skater.vlt", "data\\db\\skater.bin"] {
        let Ok(bytes) = mounts.read_path(path) else {
            continue;
        };
        let strings = ascii_strings(&bytes, 4);
        let material_strings = strings
            .iter()
            .filter(|value| {
                let lower = value.to_ascii_lowercase();
                lower.contains("material_")
                    || lower.contains("baseenvironment")
                    || lower.contains("environment.default")
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        let chunks = std::path::Path::new(path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("vlt"))
            .then(|| AttributeChunkFile::parse(&bytes));
        let vault = std::path::Path::new(path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("vlt"))
            .then(|| AttributeVaultMetadata::parse(&bytes));
        let data_file = std::path::Path::new(path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("bin"))
            .then(|| AttributeDataFile::parse(&bytes));
        let parsed_vault = vault.as_ref().and_then(|vault| vault.as_ref().ok());
        let export_types = attribute_export_type_summary(parsed_vault);
        let pointer_kinds = attribute_pointer_kind_summary(parsed_vault);
        let pointer_selectors = parsed_vault.map(|vault| {
            vault
                .pointer_fixups
                .iter()
                .filter(|fixup| fixup.kind == AttributePointerKind::SelectTargetBase)
                .collect::<Vec<_>>()
        });
        let resolved_pointers = parsed_vault
            .zip(attribute_bin.as_deref())
            .map(|(vault, bin)| {
                vault
                    .resolve_pointers(&[bytes.len(), bin.len()])
                    .map(|pointers| attribute_pointer_resolution_summary(&pointers))
            });
        let load_summary = parsed_vault.map(|vault| attribute_load_summary(&bytes, vault));
        let material_class_matches = parsed_vault
            .map(|vault| match_material_class_names(vault, &generated_material_classes));
        let chunk_summary = attribute_chunk_summary(chunks.as_ref());
        let pointer_probe = attribute_pointer_probe(&bytes, chunks.as_ref());
        let next_section_probe = attribute_next_section_probe(&bytes);
        println!(
            "attribute database {path}: {} bytes, header {:02x?}, chunks {:?}, vault {:?}, export types {:?}, load summary {:?}, material class matches {:?}, pointer kinds {:?}, pointer selectors {:?}, resolved pointers {:?}, pointer probe {:02x?}, data file {:?}, next-section probe {:02x?}, {} ASCII strings, material/effect strings {:?}",
            bytes.len(),
            &bytes[..bytes.len().min(32)],
            chunk_summary,
            vault.as_ref().map(|vault| vault.as_ref().map(|vault| (
                vault.version,
                &vault.dependencies,
                vault.exports.len(),
                vault.pointer_fixups.len(),
                &vault.data,
            ))),
            export_types,
            load_summary,
            material_class_matches,
            pointer_kinds,
            pointer_selectors,
            resolved_pointers,
            pointer_probe,
            data_file.as_ref().map(|data| data.as_ref().map(|data| (
                &data.string_table,
                data.strings.len(),
                &data.object_data,
            ))),
            next_section_probe,
            strings.len(),
            material_strings,
        );
    }
}

fn attribute_export_type_summary(
    vault: Option<&AttributeVaultMetadata>,
) -> Option<BTreeMap<u64, (usize, u32, u32, u64)>> {
    vault.map(|vault| {
        let mut types = BTreeMap::<u64, (usize, u32, u32, u64)>::new();
        for export in &vault.exports {
            let aggregate =
                types
                    .entry(export.type_id)
                    .or_insert((0, export.data_size, export.data_size, 0));
            aggregate.0 += 1;
            aggregate.1 = aggregate.1.min(export.data_size);
            aggregate.2 = aggregate.2.max(export.data_size);
            aggregate.3 += u64::from(export.data_size);
        }
        types
    })
}

fn attribute_pointer_kind_summary(
    vault: Option<&AttributeVaultMetadata>,
) -> Option<BTreeMap<AttributePointerKind, usize>> {
    vault.map(|vault| {
        let mut kinds = BTreeMap::<AttributePointerKind, usize>::new();
        for fixup in &vault.pointer_fixups {
            *kinds.entry(fixup.kind).or_default() += 1;
        }
        kinds
    })
}

#[derive(Debug, Default)]
struct AttributePointerResolutionSummary {
    location_dependencies: BTreeMap<u16, usize>,
    direct_dependencies: BTreeMap<u16, usize>,
    nulls: usize,
    exports: usize,
}

fn attribute_pointer_resolution_summary(
    pointers: &BTreeMap<skate2_formats::attrib::AttributePointerLocation, AttributePointerValue>,
) -> AttributePointerResolutionSummary {
    let mut summary = AttributePointerResolutionSummary::default();
    for (location, value) in pointers {
        *summary
            .location_dependencies
            .entry(location.dependency_index)
            .or_default() += 1;
        match value {
            AttributePointerValue::Null => summary.nulls += 1,
            AttributePointerValue::Direct {
                dependency_index, ..
            } => {
                *summary
                    .direct_dependencies
                    .entry(*dependency_index)
                    .or_default() += 1;
            }
            AttributePointerValue::Export { .. } => summary.exports += 1,
        }
    }
    summary
}

fn attribute_chunk_summary(
    chunks: Option<&Result<AttributeChunkFile, skate2_formats::attrib::AttributeChunkError>>,
) -> Option<Result<BTreeMap<String, [usize; 2]>, &skate2_formats::attrib::AttributeChunkError>> {
    chunks.map(|chunks| {
        chunks.as_ref().map(|file| {
            let mut summary = BTreeMap::<String, [usize; 2]>::new();
            for chunk in &file.chunks {
                let totals = summary
                    .entry(String::from_utf8_lossy(&chunk.tag).into_owned())
                    .or_default();
                totals[0] += 1;
                totals[1] += chunk.size;
            }
            summary
        })
    })
}

fn generated_material_class_names(bytes: &[u8]) -> BTreeSet<String> {
    const PREFIX: &str = "Attrib::Gen::ClassRefSpec_";
    ascii_strings(bytes, PREFIX.len())
        .into_iter()
        .filter_map(|value| value.strip_prefix(PREFIX).map(str::to_owned))
        .filter(|value| value.starts_with("material_"))
        .collect()
}

fn match_material_class_names(
    vault: &AttributeVaultMetadata,
    names: &BTreeSet<String>,
) -> BTreeMap<String, u64> {
    let class_ids = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Class)
        .map(|export| export.object_id)
        .collect::<BTreeSet<_>>();
    names
        .iter()
        .filter_map(|name| {
            let id = string_hash64(name);
            class_ids.contains(&id).then(|| (name.clone(), id))
        })
        .collect()
}

#[derive(Debug, Default)]
struct AttributeLoadSummary {
    database_class_count: u32,
    database_type_count: u32,
    classes: usize,
    class_id_mismatches: usize,
    definitions: u64,
    collection_reserve: u64,
    collections: usize,
    collection_class_count: usize,
    parented_collections: usize,
    entries: u64,
}

fn attribute_load_summary(
    bytes: &[u8],
    vault: &AttributeVaultMetadata,
) -> Result<AttributeLoadSummary, skate2_formats::attrib::AttributeChunkError> {
    let mut summary = AttributeLoadSummary::default();
    let mut collection_classes = BTreeSet::new();
    for export in &vault.exports {
        match export.kind() {
            skate2_formats::attrib::AttributeExportKind::Database => {
                let database = export.database_load_data(bytes)?;
                summary.database_class_count = database.class_count;
                summary.database_type_count = database.type_count;
            }
            skate2_formats::attrib::AttributeExportKind::Class => {
                let class = export.class_load_data(bytes)?;
                summary.classes += 1;
                summary.class_id_mismatches += usize::from(class.class_id != export.object_id);
                summary.definitions += u64::from(class.definition_count);
                summary.collection_reserve += u64::from(class.collection_reserve);
            }
            skate2_formats::attrib::AttributeExportKind::Collection => {
                let collection = export.collection_load_data(bytes)?;
                summary.collections += 1;
                collection_classes.insert(collection.class_id);
                summary.parented_collections += usize::from(collection.parent_id != 0);
                summary.entries += u64::from(collection.entry_count);
            }
            skate2_formats::attrib::AttributeExportKind::Unknown(_) => {}
        }
    }
    summary.collection_class_count = collection_classes.len();
    Ok(summary)
}

fn attribute_pointer_probe<'a>(
    bytes: &'a [u8],
    chunks: Option<&Result<AttributeChunkFile, skate2_formats::attrib::AttributeChunkError>>,
) -> Option<(&'a [u8], &'a [u8])> {
    let chunk = chunks?
        .as_ref()
        .ok()?
        .chunks
        .iter()
        .find(|chunk| &chunk.tag == b"PtrN")?;
    Some((
        &bytes[chunk.payload.start..(chunk.payload.start + 64).min(chunk.payload.end)],
        &bytes[chunk.payload.end.saturating_sub(32)..chunk.payload.end],
    ))
}

fn attribute_next_section_probe(bytes: &[u8]) -> Option<&[u8]> {
    let value = bytes.get(4..8)?;
    let offset = u32::from_be_bytes([value[0], value[1], value[2], value[3]]) as usize;
    bytes.get(offset.saturating_sub(16)..(offset + 32).min(bytes.len()))
}

fn ascii_strings(bytes: &[u8], minimum_length: usize) -> Vec<String> {
    let mut strings = Vec::new();
    let mut start = None;
    for (index, byte) in bytes.iter().copied().chain([0]).enumerate() {
        if (0x20..=0x7e).contains(&byte) {
            start.get_or_insert(index);
        } else if let Some(string_start) = start.take()
            && index - string_start >= minimum_length
        {
            strings.push(String::from_utf8_lossy(&bytes[string_start..index]).into_owned());
        }
    }
    strings
}

#[derive(Default)]
struct ShaderInventory {
    manifests: usize,
    sets: BTreeMap<String, usize>,
    versions: BTreeMap<String, usize>,
    vertex_programs: usize,
    vertex_program_bytes: u64,
    pixel_programs: usize,
    pixel_program_bytes: u64,
    techniques: usize,
    passes: usize,
    render_states: usize,
    samplers: usize,
    sampler_states: usize,
    missing_program_assets: usize,
    render_state_values: BTreeMap<(String, String), usize>,
    sampler_state_values: BTreeMap<(String, String), usize>,
    sampler_names: BTreeMap<String, usize>,
    referenced_program_assets: BTreeSet<String>,
    failures: Vec<String>,
}

fn inventory_shader_manifest(
    manifest: &ShaderManifest,
    lower_path: &str,
    path: &str,
    mounted: &MountedBig,
    inventory: &mut ShaderInventory,
) {
    inventory.manifests += 1;
    *inventory
        .versions
        .entry(manifest.version.clone())
        .or_default() += 1;
    let set = if lower_path.contains("\\debug\\") {
        "debug"
    } else if lower_path.contains("\\final\\") {
        "final"
    } else {
        "other"
    };
    *inventory.sets.entry(set.to_owned()).or_default() += 1;
    let directory = path
        .rsplit_once('\\')
        .map_or("", |(directory, _)| directory);
    for program in &manifest.vertex_programs {
        inventory.vertex_programs += 1;
        inventory.vertex_program_bytes += u64::from(program.size);
        let program_path = format!("{directory}\\{}", program.name);
        inventory
            .referenced_program_assets
            .insert(program_path.to_ascii_lowercase());
        inventory.missing_program_assets +=
            usize::from(mounted.archive().find_name(&program_path).is_none());
    }
    for program in &manifest.pixel_programs {
        inventory.pixel_programs += 1;
        inventory.pixel_program_bytes += u64::from(program.size);
        let program_path = format!("{directory}\\{}", program.name);
        inventory
            .referenced_program_assets
            .insert(program_path.to_ascii_lowercase());
        inventory.missing_program_assets +=
            usize::from(mounted.archive().find_name(&program_path).is_none());
    }
    inventory.techniques += manifest.techniques.len();
    for technique in &manifest.techniques {
        inventory.passes += technique.passes.len();
        for pass in &technique.passes {
            inventory.render_states += pass.states.len();
            for state in &pass.states {
                *inventory
                    .render_state_values
                    .entry((state.name.clone(), state.value.clone()))
                    .or_default() += 1;
            }
            inventory.samplers += pass.samplers.len();
            for sampler in &pass.samplers {
                *inventory
                    .sampler_names
                    .entry(sampler.name.clone())
                    .or_default() += 1;
                inventory.sampler_states += sampler.states.len();
                for state in &sampler.states {
                    *inventory
                        .sampler_state_values
                        .entry((state.name.clone(), state.value.clone()))
                        .or_default() += 1;
                }
            }
        }
    }
}

#[derive(Default)]
struct PresentationInventory {
    collections: usize,
    rw4_arenas: usize,
    rw4_objects: usize,
    models: usize,
    model_meshes: usize,
    meshes: usize,
    optimized_meshes: usize,
    vertices: u64,
    decoded_positions: u64,
    decoded_vertex_attributes: u64,
    decoded_vertex_attribute_values: u64,
    vertex_descriptors: usize,
    vertex_elements: usize,
    vertex_buffers: usize,
    vertex_buffer_bytes: u64,
    index_buffers: usize,
    indices: u64,
    decoded_indices: u64,
    index_buffer_bytes: u64,
    mesh_helpers: usize,
    draw_packets: usize,
    draw_indices: u64,
    gpu_resources: usize,
    material_blocks: usize,
    materials: usize,
    material_channels: usize,
    material_references: usize,
    resolved_material_references: usize,
    unresolved_material_references: usize,
    material_reference_sections: BTreeMap<u16, usize>,
    material_subreference_types: BTreeMap<u32, usize>,
    material_subreference_base_types: BTreeMap<u32, usize>,
    raster_channels: usize,
    constant_channels: usize,
    textures: usize,
    instance_data: usize,
    instances: usize,
    instance_local_guids: BTreeSet<u64>,
    embedded_data: usize,
    embedded_reference_groups: [usize; 4],
    embedded_reference_sections: BTreeMap<(usize, u16), usize>,
    embedded_local_target_types: BTreeMap<(usize, u32), usize>,
    embedded_subreference_types: BTreeMap<(usize, u32), usize>,
    embedded_subreference_base_types: BTreeMap<(usize, u32), usize>,
    embedded_external_arena_ids: BTreeMap<(usize, u32), usize>,
    embedded_reference_samples: Vec<String>,
    raster_local_toc_guid_matches: usize,
    raster_local_toc_guid_misses: usize,
    raster_texture_asset_guid_matches: usize,
    raster_texture_asset_guid_misses: usize,
    raster_texture_toc_guid_matches: usize,
    raster_texture_toc_guid_misses: usize,
    raster_zero_guids: usize,
    raster_empty_stream_names: usize,
    raster_stream_names: BTreeSet<String>,
    raster_binding_by_shader_input: BTreeMap<String, [usize; 3]>,
    raster_binding_by_image_channel: BTreeMap<u16, [usize; 3]>,
    raster_reference_samples: BTreeMap<String, (u64, String, u16)>,
    texture_asset_types: BTreeMap<u32, usize>,
    texture_stream_collections: usize,
    texture_stream_arenas: usize,
    texture_stream_objects: usize,
    texture_stream_toc_entries: usize,
    texture_stream_object_types: BTreeMap<u32, usize>,
    texture_toc_object_types: BTreeMap<u32, usize>,
    texture_toc_link_sections: BTreeMap<u16, usize>,
    texture_toc_target_types: BTreeMap<u32, usize>,
    texture_stream_texture_types: BTreeMap<TextureType, usize>,
    texture_stream_endian_modes: BTreeMap<TextureEndian, usize>,
    texture_stream_texture_formats: BTreeMap<u32, usize>,
    texture_stream_mip_levels: BTreeMap<u32, usize>,
    texture_stream_packed_mips: BTreeMap<bool, usize>,
    texture_stream_mip_data_offsets: BTreeMap<u32, usize>,
    texture_stream_zero_offset_mip_tails: usize,
    texture_stream_expected_payload_matches: usize,
    texture_stream_expected_payload_deltas: BTreeMap<i64, usize>,
    texture_stream_decoded_levels: usize,
    texture_stream_decoded_bytes: u64,
    texture_stream_decode_failures: usize,
    local_links: usize,
    external_links: usize,
    failures: BTreeMap<&'static str, usize>,
    object_types: BTreeMap<u32, usize>,
    vertex_formats: BTreeMap<u32, usize>,
    vertex_semantic_formats: BTreeMap<(u8, u8, u32), usize>,
    vertex_methods: BTreeMap<u8, usize>,
    vertex_element_types: BTreeMap<u8, usize>,
    vertex_element_classes: BTreeMap<u32, usize>,
    vertex_streams: BTreeMap<u16, usize>,
    non_finite_vertex_attributes: BTreeMap<(VertexSemantic, u32), [usize; 2]>,
    non_finite_vertex_samples: BTreeMap<(VertexSemantic, u32), Vec<String>>,
    non_finite_vertex_owners: BTreeSet<String>,
    position_formats: BTreeMap<u32, usize>,
    primitive_types: BTreeMap<u32, usize>,
    material_flags: BTreeMap<u32, usize>,
    channel_flags: BTreeMap<u16, usize>,
    shader_inputs: BTreeMap<String, usize>,
    material_effect_names: BTreeMap<String, usize>,
    texture_types: BTreeMap<TextureType, usize>,
    texture_formats: BTreeMap<u32, usize>,
}

fn inventory_bam_pres(
    mounts: &MountSet,
    dmo_model_ids: Option<&BTreeSet<u64>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = "data\\content\\world\\stream\\BAM\\BAM_Pres";
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let mut inventory = PresentationInventory::default();
    let (texture_asset_ids, texture_toc_guids) =
        inventory_texture_asset_indices(mounts, &mut inventory)?;
    for collection in &collections.collections {
        let path = format!(
            "data\\content\\world\\stream\\BAM\\{}.xsf",
            collection.filename
        );
        let bytes = mounts.read_path(&path)?;
        let file = StreamFile::parse(&bytes, collection.asset_count)?;
        inventory.collections += 1;
        for block in &file.assets {
            let decoded = decode_arena(&bytes[block.data_range.clone()])?;
            let arena = Rw4Arena::parse(&decoded.resources)?;
            let sections = arena.sections(&decoded.resources)?;
            inventory.rw4_arenas += 1;
            inventory.rw4_objects += arena.dictionary.len();
            for entry in &arena.dictionary {
                *inventory.object_types.entry(entry.type_id).or_default() += 1;
                if entry.type_id == GPU_BUFFER_RESOURCE_TYPE_ID {
                    inventory.gpu_resources += 1;
                }
                let Some(object) = arena.object_bytes(&decoded.resources, entry) else {
                    record_presentation_failure(&mut inventory, "RW4 object range");
                    continue;
                };
                inventory_presentation_object(
                    &arena,
                    &sections,
                    entry.type_id,
                    object,
                    &mut inventory,
                );
            }
            inventory_presentation_geometry(
                &arena,
                &decoded.resources,
                (&collection.filename, block.id),
                &mut inventory,
            );
            inventory_material_texture_links(
                &arena,
                &decoded.resources,
                &texture_asset_ids,
                &texture_toc_guids,
                &mut inventory,
            );
            inventory_mesh_material_links(&arena, &decoded.resources, &mut inventory);
        }
    }
    print_presentation_inventory(&inventory);
    finish_presentation_inventory(mounts, &inventory, dmo_model_ids)
}

fn print_presentation_inventory(inventory: &PresentationInventory) {
    println!(
        "BAM presentation inventory: {} collections, {} arenas, {} objects; {} models / {} model meshes, {} regular + {} optimized meshes / {} derived vertices / {} decoded positions; {} descriptors / {} elements, {} vertex buffers / {} bytes, {} index buffers / {} indices / {} decoded / {} bytes, {} helpers, {} draw packets / {} draw indices, {} GPU resources; {} material blocks / {} materials / {} channels ({} raster, {} constant), {} textures; raster GUIDs local TOC {} matched / {} missed, texture ATOC {} matched / {} missed, texture inner TOC {} matched / {} missed, {} zero; {} unique / {} empty stream names; raster bindings [texture TOC, miss, zero] by shader {:?}, by image channel {:?}; texture asset types {:?}; {} local links, {} external links; failures {:?}; position formats {:?}; vertex formats {:?}; primitive types {:?}; material flags {:?}; channel flags {:?}; shader inputs {:?}; texture types {:?}; texture formats {:?}; object types {:?}",
        inventory.collections,
        inventory.rw4_arenas,
        inventory.rw4_objects,
        inventory.models,
        inventory.model_meshes,
        inventory.meshes,
        inventory.optimized_meshes,
        inventory.vertices,
        inventory.decoded_positions,
        inventory.vertex_descriptors,
        inventory.vertex_elements,
        inventory.vertex_buffers,
        inventory.vertex_buffer_bytes,
        inventory.index_buffers,
        inventory.indices,
        inventory.decoded_indices,
        inventory.index_buffer_bytes,
        inventory.mesh_helpers,
        inventory.draw_packets,
        inventory.draw_indices,
        inventory.gpu_resources,
        inventory.material_blocks,
        inventory.materials,
        inventory.material_channels,
        inventory.raster_channels,
        inventory.constant_channels,
        inventory.textures,
        inventory.raster_local_toc_guid_matches,
        inventory.raster_local_toc_guid_misses,
        inventory.raster_texture_asset_guid_matches,
        inventory.raster_texture_asset_guid_misses,
        inventory.raster_texture_toc_guid_matches,
        inventory.raster_texture_toc_guid_misses,
        inventory.raster_zero_guids,
        inventory.raster_stream_names.len(),
        inventory.raster_empty_stream_names,
        inventory.raster_binding_by_shader_input,
        inventory.raster_binding_by_image_channel,
        inventory.texture_asset_types,
        inventory.local_links,
        inventory.external_links,
        inventory.failures,
        inventory.position_formats,
        inventory.vertex_formats,
        inventory.primitive_types,
        inventory.material_flags,
        inventory.channel_flags,
        inventory.shader_inputs,
        inventory.texture_types,
        inventory.texture_formats,
        inventory.object_types,
    );
}

fn inventory_material_attribute_bindings(
    mounts: &MountSet,
    effect_names: &BTreeMap<String, usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let vlt = mounts.read_path("data\\db\\skater.vlt")?;
    let bin = mounts.read_path("data\\db\\skater.bin")?;
    let vault = AttributeVaultMetadata::parse(&vlt)?;
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()])?;
    let buffers = [vlt.as_slice(), bin.as_slice()];
    let classes = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Class)
        .map(|export| (export.object_id, export))
        .collect::<BTreeMap<_, _>>();
    let collections = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Collection)
        .map(|export| {
            let collection = export.collection_load_data(&vlt)?;
            Ok((
                (collection.class_id, collection.collection_id),
                (export, collection),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, skate2_formats::attrib::AttributeChunkError>>()?;

    let mut attributed_count = 0;
    let mut matched = 0;
    let mut legacy = BTreeMap::new();
    let mut missing = BTreeMap::new();
    let mut manifests = BTreeMap::new();
    let mut draw_orders = BTreeMap::new();
    let mut unresolved_attributes = BTreeMap::new();
    for (name, count) in effect_names {
        let effect = PresentationMaterialEffect::from_serialized_name(name);
        let Some((class_id, variant_id)) =
            effect.attribute_class_id.zip(effect.material_variant_id)
        else {
            legacy.insert(name.clone(), *count);
            continue;
        };
        attributed_count += count;
        let Some(class) = classes.get(&class_id) else {
            missing.insert(name.clone(), *count);
            continue;
        };
        let Some((collection_export, collection)) = collections.get(&(class_id, variant_id)) else {
            missing.insert(name.clone(), *count);
            continue;
        };
        matched += count;
        match resolve_material_attributes(
            class,
            collection_export,
            collection,
            &classes,
            &collections,
            &buffers,
            &pointers,
        ) {
            Ok(resolved) => {
                manifests.insert(name.clone(), resolved.manifest_name);
                draw_orders.insert(name.clone(), resolved.draw_order);
            }
            Err(reason) => {
                unresolved_attributes.insert(format!("{name} ({reason})"), *count);
            }
        }
    }
    println!(
        "BAM material attribute bindings: {matched} / {attributed_count} attributed materials match VLT class+collection keys; manifests {manifests:?}; draw orders {draw_orders:?}; unresolved attributes {unresolved_attributes:?}; legacy {legacy:?}; missing {missing:?}",
    );
    Ok(())
}

fn bind_runtime_material_attributes(
    mounts: &MountSet,
    scene: &mut PresentationScene,
) -> Result<usize, Box<dyn std::error::Error>> {
    // CreateEffectListFromPegasusMaterialUnSkinned82e20948 falls back to
    // material_defaultenvironment/default when the material has no resolvable
    // Attribulator collection. GetAttribulatorKeyFromAttrMaterialName82e13310
    // cannot split legacy names such as model_default. Resolve that world-only
    // case here, preserving the serialized name for diagnostics. The normal
    // Vault lookup below supplies the real effect and draw order; no shader or
    // render state is guessed. Character/vehicle material parsing is unchanged.
    for effect in scene
        .resident_collections_mut()
        .flat_map(|collection| &mut collection.arenas)
        .flat_map(|arena| &mut arena.materials)
        .filter_map(|material| material.effect.as_mut())
    {
        if effect
            .attribute_class_id
            .zip(effect.material_variant_id)
            .is_none()
        {
            let serialized_name = std::mem::take(&mut effect.serialized_name);
            *effect =
                PresentationMaterialEffect::from_serialized_name("defaultenvironment.default");
            effect.serialized_name = serialized_name;
        }
    }
    let keys = scene
        .resident_collections()
        .flat_map(|collection| &collection.arenas)
        .flat_map(|arena| &arena.materials)
        .filter_map(|material| material.effect.as_ref())
        .filter_map(|effect| effect.attribute_class_id.zip(effect.material_variant_id))
        .collect::<BTreeSet<_>>();
    let bindings = load_material_attribute_bindings(mounts, &keys)?;
    let mut bound = 0;
    for effect in scene
        .resident_collections_mut()
        .flat_map(|collection| &mut collection.arenas)
        .flat_map(|arena| &mut arena.materials)
        .filter_map(|material| material.effect.as_mut())
    {
        let Some(key) = effect.attribute_class_id.zip(effect.material_variant_id) else {
            continue;
        };
        effect.binding = Some(
            bindings
                .get(&key)
                .ok_or("runtime material has no attribute binding")?
                .clone(),
        );
        bound += 1;
    }
    Ok(bound)
}

/// Resolved bindings are a pure function of the mounted Vault and key, so each
/// key is resolved once per mount instead of re-reading the whole Vault.
fn load_material_attribute_bindings(
    mounts: &MountSet,
    keys: &BTreeSet<(u64, u64)>,
) -> Result<BTreeMap<(u64, u64), PresentationMaterialBinding>, Box<dyn std::error::Error>> {
    type Cache = BTreeMap<(usize, u64, u64), PresentationMaterialBinding>;
    static CACHE: std::sync::Mutex<Cache> = std::sync::Mutex::new(BTreeMap::new());
    let mount = std::ptr::from_ref(mounts) as usize;
    let mut cache = CACHE.lock().map_err(|_| "material binding cache poisoned")?;
    let missing = keys
        .iter()
        .copied()
        .filter(|&(class, collection)| !cache.contains_key(&(mount, class, collection)))
        .collect::<BTreeSet<_>>();
    if !missing.is_empty() {
        for ((class, collection), binding) in resolve_material_attribute_bindings(mounts, &missing)? {
            cache.insert((mount, class, collection), binding);
        }
    }
    keys.iter()
        .map(|&(class, collection)| {
            cache
                .get(&(mount, class, collection))
                .map(|binding| ((class, collection), binding.clone()))
                .ok_or_else(|| "material binding cache lost a resolved key".into())
        })
        .collect()
}

/// `skater.vlt` and `skater.bin` of a mount set, read once: every material binding resolves
/// from them, and reading them again for each new material (a remote skater's, a pedestrian's)
/// would read the archive on the render thread.
fn skater_vault(mounts: &MountSet) -> Result<std::sync::Arc<(Vec<u8>, Vec<u8>)>, Box<dyn std::error::Error>> {
    type Vaults = BTreeMap<usize, std::sync::Arc<(Vec<u8>, Vec<u8>)>>;
    static VAULTS: std::sync::Mutex<Vaults> = std::sync::Mutex::new(BTreeMap::new());
    let mount = std::ptr::from_ref(mounts) as usize;
    let mut vaults = VAULTS.lock().map_err(|_| "skater vault cache poisoned")?;
    if let Some(bytes) = vaults.get(&mount) {
        return Ok(std::sync::Arc::clone(bytes));
    }
    let bytes = std::sync::Arc::new((mounts.read_path("data\\db\\skater.vlt")?, mounts.read_path("data\\db\\skater.bin")?));
    vaults.insert(mount, std::sync::Arc::clone(&bytes));
    Ok(bytes)
}

fn resolve_material_attribute_bindings(
    mounts: &MountSet,
    keys: &BTreeSet<(u64, u64)>,
) -> Result<BTreeMap<(u64, u64), PresentationMaterialBinding>, Box<dyn std::error::Error>> {
    let bytes = skater_vault(mounts)?;
    let (vlt, bin) = (bytes.0.as_slice(), bytes.1.as_slice());
    let vault = AttributeVaultMetadata::parse(vlt)?;
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()])?;
    let buffers = [vlt, bin];
    let classes = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Class)
        .map(|export| (export.object_id, export))
        .collect::<BTreeMap<_, _>>();
    let collections = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Collection)
        .map(|export| {
            let collection = export.collection_load_data(vlt)?;
            Ok((
                (collection.class_id, collection.collection_id),
                (export, collection),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, skate2_formats::attrib::AttributeChunkError>>()?;
    let mut bindings = BTreeMap::new();
    for &(class_id, collection_id) in keys {
        let class = classes
            .get(&class_id)
            .ok_or_else(|| format!("missing material class {class_id:016x}"))?;
        let (collection_export, collection) =
            collections.get(&(class_id, collection_id)).ok_or_else(|| {
                format!("missing material collection {class_id:016x}:{collection_id:016x}")
            })?;
        let binding = resolve_material_attributes(
            class,
            collection_export,
            collection,
            &classes,
            &collections,
            &buffers,
            &pointers,
        )
        .map_err(|reason| {
            format!("material collection {class_id:016x}:{collection_id:016x}: {reason}")
        })?;
        bindings.insert((class_id, collection_id), binding);
    }
    Ok(bindings)
}

type AttributeCollectionMap<'a> =
    BTreeMap<(u64, u64), (&'a AttributeExport, AttributeCollectionLoadData)>;

fn load_animation_physics_tuning(
    mounts: &MountSet,
) -> Result<AnimationPhysicsTuning, Box<dyn std::error::Error>> {
    const USE_ENDPOINTS_KEY: u64 = 0xB52E_6EE4_F322_A9F4;
    const LOWER_ENDPOINT_KEY: u64 = 0x982F_499E_C977_35A9;
    const UPPER_ENDPOINT_KEY: u64 = 0xA1F6_5BB8_C4D1_BA8B;

    let values = load_attribute_values(
        mounts,
        "anim_motion",
        "jumping",
        &[USE_ENDPOINTS_KEY, LOWER_ENDPOINT_KEY, UPPER_ENDPOINT_KEY],
    )?;
    Ok(AnimationPhysicsTuning {
        use_jump_strength_endpoints: values[0].as_deref().is_some_and(|value| value == [1]),
        jump_strength_lower_endpoint: optional_be_f32(
            values[1].as_deref(),
            "invalid lower jump endpoint",
        )?,
        jump_strength_upper_endpoint: optional_be_f32(
            values[2].as_deref(),
            "invalid upper jump endpoint",
        )?,
    })
}

fn load_ground_state_tuning(
    mounts: &MountSet,
) -> Result<GroundStateTuning, Box<dyn std::error::Error>> {
    // Exact keys reconstructed from ConditionIsOffGround at 0x82dd78d8.
    const CLEAR_GROUND_HEIGHT_KEY: u64 = 0x0FC2_2CB4_43A7_63AC;
    const RECENT_CONTACT_HEIGHT_KEY: u64 = 0x8E8B_8588_435B_6FE8;
    const CONTACT_GRACE_SECONDS_KEY: u64 = 0xC6E6_E6BD_74DF_FB65;
    let values = load_attribute_values(
        mounts,
        "physics_airstates",
        "default",
        &[
            CLEAR_GROUND_HEIGHT_KEY,
            RECENT_CONTACT_HEIGHT_KEY,
            CONTACT_GRACE_SECONDS_KEY,
        ],
    )?;
    decode_ground_state_tuning_values(
        values[0].as_deref(),
        values[1].as_deref(),
        values[2].as_deref(),
    )
    .map_err(Into::into)
}

fn decode_ground_state_tuning_values(
    clear_ground_height: Option<&[u8]>,
    recent_contact_ground_height: Option<&[u8]>,
    wheel_contact_grace_seconds: Option<&[u8]>,
) -> Result<GroundStateTuning, &'static str> {
    Ok(GroundStateTuning {
        clear_ground_height: optional_be_f32(clear_ground_height, "invalid clear-ground height")?,
        recent_contact_ground_height: optional_be_f32(
            recent_contact_ground_height,
            "invalid recent-contact height",
        )?,
        wheel_contact_grace_seconds: optional_be_f32(
            wheel_contact_grace_seconds,
            "invalid wheel-contact grace time",
        )?,
    })
}

fn load_jump_physics_curves(
    mounts: &MountSet,
) -> Result<JumpPhysicsCurves, Box<dyn std::error::Error>> {
    let values = load_attribute_values(
        mounts,
        "physics_jump",
        "default",
        &[
            string_hash64("VerticalResponse"),
            string_hash64("JumpYScalarVsGroundNormalY"),
            string_hash64("JumpSpeedScalarVsAngle"),
            string_hash64("MinHeightVsSpeed"),
            string_hash64("MaxHeightVsSpeed"),
        ],
    )?;
    Ok(JumpPhysicsCurves {
        vertical_response: decode_point_graph(required_attribute(&values, 0)?, 0)?,
        jump_y_scalar_vs_ground_normal_y: decode_point_graph(required_attribute(&values, 1)?, 16)?,
        jump_speed_scalar_vs_angle: decode_point_graph(required_attribute(&values, 2)?, 16)?,
        min_height_vs_speed: decode_point_graph(required_attribute(&values, 3)?, 0)?,
        max_height_vs_speed: decode_point_graph(required_attribute(&values, 4)?, 0)?,
    })
}

fn load_ground_jump_tuning(
    mounts: &MountSet,
) -> Result<GroundJumpTuning, Box<dyn std::error::Error>> {
    // Exact keys reconstructed from Skateboard::Toolkit_CalcGroundJump at
    // 0x82b315a8. Their semantic names follow the operations consuming them;
    // the original field names are not present as plaintext in this build.
    const HEIGHT_CURVE_REFERENCE_SPEED_KEY: u64 = 0x97CD_B6C3_13CD_0A1B;
    const LOW_SPEED_JUMP_HEIGHT_KEY: u64 = 0xA35E_E331_9500_4033;
    const MINIMUM_JUMP_HEIGHT_KEY: u64 = 0xBB9E_9BAF_BA1B_29E6;
    const MAXIMUM_JUMP_HEIGHT_KEY: u64 = 0x0B24_54C6_6641_0860;
    const USE_VERTICAL_RESPONSE_CURVE_KEY: u64 = 0xC1A0_16BA_4CA8_8480;
    const MINIMUM_VERTICAL_RESPONSE_KEY: u64 = 0x8D13_4B78_61D8_8C9A;
    const MAXIMUM_HORIZONTAL_COMPENSATION_KEY: u64 = 0xBE6C_6A3E_D7C6_D148;
    const SIDEWAYS_ADJUSTMENT_SCALE_KEY: u64 = 0x09A6_C205_C73D_D180;
    const FORWARD_ADJUSTMENT_SCALE_KEY: u64 = 0x4EDD_BD69_72EE_9C56;
    let values = load_attribute_values(
        mounts,
        "physics_jump",
        "default",
        &[
            HEIGHT_CURVE_REFERENCE_SPEED_KEY,
            LOW_SPEED_JUMP_HEIGHT_KEY,
            MINIMUM_JUMP_HEIGHT_KEY,
            MAXIMUM_JUMP_HEIGHT_KEY,
            USE_VERTICAL_RESPONSE_CURVE_KEY,
            MINIMUM_VERTICAL_RESPONSE_KEY,
            MAXIMUM_HORIZONTAL_COMPENSATION_KEY,
            SIDEWAYS_ADJUSTMENT_SCALE_KEY,
            FORWARD_ADJUSTMENT_SCALE_KEY,
        ],
    )?;
    Ok(GroundJumpTuning {
        height_curve_reference_speed: optional_be_f32(
            values[0].as_deref(),
            "invalid jump-height reference speed",
        )?,
        low_speed_jump_height: optional_be_f32(
            values[1].as_deref(),
            "invalid low-speed jump height",
        )?,
        minimum_jump_height: optional_be_f32(values[2].as_deref(), "invalid minimum jump height")?,
        maximum_jump_height: optional_be_f32(values[3].as_deref(), "invalid maximum jump height")?,
        use_vertical_response_curve: values[4].as_deref().is_some_and(|value| value == [1]),
        minimum_vertical_response: optional_be_f32(
            values[5].as_deref(),
            "invalid minimum vertical response",
        )?,
        maximum_horizontal_compensation: optional_be_f32(
            values[6].as_deref(),
            "invalid maximum horizontal compensation",
        )?,
        sideways_adjustment_scale: optional_be_f32(
            values[7].as_deref(),
            "invalid sideways jump-adjustment scale",
        )?,
        forward_adjustment_scale: optional_be_f32(
            values[8].as_deref(),
            "invalid forward jump-adjustment scale",
        )?,
    })
}

#[allow(clippy::too_many_lines)]
fn load_trajectory_selection_tuning(
    mounts: &MountSet,
) -> Result<TrajectorySelectionTuning, Box<dyn std::error::Error>> {
    // `FindBestGrind` constructs this 64-bit key directly. Its generated
    // schema name is absent from the PDB and from the executable's strings.
    const GRIND_MINIMUM_ANGLE_IMPROVEMENT_VS_HEIGHT_DELTA_KEY: u64 = 0x7DFC_D6F1_D65B_425C;
    let values = load_attribute_values(
        mounts,
        "physics_trajectory",
        "default",
        &[
            string_hash64("LandingTimeBonus"),
            string_hash64("LandingComScalarVsSlope"),
            string_hash64("LandingForceScalarVsDPVelNorm"),
            string_hash64("ConeAngleZVsSpeed"),
            string_hash64("SpeedFactorMin"),
            string_hash64("SpeedFactorMax"),
            string_hash64("ScoreMiddleBonus"),
            string_hash64("ScoreLandingForce"),
            string_hash64("ScoreLandingDirection"),
            string_hash64("ScoreSurface_Unrideable"),
            string_hash64("ScoreSurface_DontAlign"),
            string_hash64("MaxTrajectoryAdjust"),
            string_hash64("MinTimeForTrajectoryToBeValid"),
            string_hash64("MinNormalDeltaForSecondPass"),
            string_hash64("ConeAngleXSecondPass"),
            string_hash64("ConeAngleZSecondPass"),
            string_hash64("GrindPenaltyVsDistToGrind"),
            string_hash64("ScoreGrind"),
            string_hash64("GrindDetectionRadius"),
            string_hash64("ScoreTransition"),
            string_hash64("MinTimeAfterApexForTransition"),
            string_hash64("GrindTipScalar"),
            string_hash64("GrindOffset"),
            string_hash64("GrindMaxSpeedSqrIntoLedge"),
            string_hash64("GrindMaxSpeedSqrIntoGrind"),
            string_hash64("GrindMaxSpeedDownOntoGrind"),
            string_hash64("GrindLockToTipDist"),
            string_hash64("GrindLockToCentreDist"),
            string_hash64("GrindLockLedgeScalar2"),
            string_hash64("GrindLockLedgeScalar"),
            string_hash64("GrindLockLedgeLowSideScalar2"),
            string_hash64("GrindLockLedgeLowSideScalar"),
            string_hash64("GrindLandingVelScalar"),
            string_hash64("GrindLandingMaxAngle"),
            string_hash64("GrindAdjustMaxAngle"),
            GRIND_MINIMUM_ANGLE_IMPROVEMENT_VS_HEIGHT_DELTA_KEY,
        ],
    )?;
    let scalar =
        |index: usize, message: &'static str| optional_be_f32(values[index].as_deref(), message);
    Ok(TrajectorySelectionTuning {
        landing_time_bonus: decode_point_graph(required_attribute(&values, 0)?, 16)?,
        landing_centre_of_mass_scalar_vs_slope: decode_point_graph(
            required_attribute(&values, 1)?,
            16,
        )?,
        grind_penalty_vs_distance_to_grind: decode_point_graph(
            required_attribute(&values, 16)?,
            16,
        )?,
        landing_force_scalar_vs_dot_velocity_normal: decode_point_graph(
            required_attribute(&values, 2)?,
            0,
        )?,
        cone_angle_z_vs_speed: decode_point_graph(required_attribute(&values, 3)?, 0)?,
        speed_factor_minimum: scalar(4, "invalid minimum trajectory speed factor")?,
        speed_factor_maximum: scalar(5, "invalid maximum trajectory speed factor")?,
        score_middle_bonus: scalar(6, "invalid middle trajectory score")?,
        score_landing_force: scalar(7, "invalid landing-force trajectory score")?,
        score_landing_direction: scalar(8, "invalid landing-direction trajectory score")?,
        score_grind: scalar(17, "invalid grind trajectory score")?,
        score_transition: scalar(19, "invalid transition trajectory score")?,
        score_surface_unrideable: scalar(9, "invalid unrideable-surface trajectory score")?,
        score_surface_do_not_align: scalar(10, "invalid do-not-align surface trajectory score")?,
        grind_detection_radius: scalar(18, "invalid grind-detection radius")?,
        grind_tip_scalar: scalar(21, "invalid grind tip scalar")?,
        grind_offset: scalar(22, "invalid grind offset")?,
        grind_maximum_speed_squared_into_ledge: scalar(
            23,
            "invalid grind maximum speed squared into ledge",
        )?,
        grind_maximum_speed_squared_into_grind: scalar(
            24,
            "invalid grind maximum speed squared into grind",
        )?,
        grind_maximum_speed_down_onto_grind: scalar(25, "invalid grind maximum downward speed")?,
        grind_lock_to_tip_distance: scalar(26, "invalid grind tip lock distance")?,
        grind_lock_to_centre_distance: scalar(27, "invalid grind centre lock distance")?,
        grind_lock_ledge_scalar_squared: scalar(28, "invalid ledge lock squared scalar")?,
        grind_lock_ledge_scalar: scalar(29, "invalid ledge lock scalar")?,
        grind_lock_ledge_low_side_scalar_squared: scalar(
            30,
            "invalid low-side ledge lock squared scalar",
        )?,
        grind_lock_ledge_low_side_scalar: scalar(31, "invalid low-side ledge lock scalar")?,
        grind_landing_velocity_scalar: scalar(32, "invalid grind landing velocity scalar")?,
        grind_landing_maximum_angle: scalar(33, "invalid grind landing maximum angle")?,
        grind_adjustment_maximum_angle: scalar(34, "invalid grind adjustment maximum angle")?,
        grind_minimum_angle_improvement_vs_height_delta: decode_point_graph(
            required_attribute(&values, 35)?,
            16,
        )?,
        maximum_trajectory_adjustment: scalar(11, "invalid maximum trajectory adjustment")?,
        minimum_valid_collision_time: scalar(12, "invalid minimum trajectory time")?,
        minimum_time_after_apex_for_transition: scalar(
            20,
            "invalid transition apex-time threshold",
        )?,
        minimum_normal_delta_for_second_pass: scalar(13, "invalid second-pass normal delta")?,
        cone_angle_x_second_pass: scalar(14, "invalid second-pass X cone angle")?,
        cone_angle_z_second_pass: scalar(15, "invalid second-pass Z cone angle")?,
    })
}

fn load_trajectory_prediction_tuning(
    mounts: &MountSet,
) -> Result<TrajectoryPredictionTuning, Box<dyn std::error::Error>> {
    let trajectory_values = load_attribute_values(
        mounts,
        "physics_trajectory",
        "default",
        &[
            string_hash64("TrajectoryMaxTime"),
            string_hash64("TrajectoryMaxErrorStart"),
            string_hash64("TrajectoryMaxErrorEnd"),
        ],
    )?;
    let reckoning_values = load_attribute_values(
        mounts,
        "physics_reckoning",
        "default",
        &[string_hash64("TrajectoryRadius")],
    )?;
    Ok(TrajectoryPredictionTuning {
        radius: optional_be_f32(
            reckoning_values[0].as_deref(),
            "invalid trajectory prediction radius",
        )?,
        maximum_time: optional_be_f32(
            trajectory_values[0].as_deref(),
            "invalid maximum trajectory time",
        )?,
        maximum_error_start: optional_be_f32(
            trajectory_values[1].as_deref(),
            "invalid initial trajectory prediction error",
        )?,
        maximum_error_end: optional_be_f32(
            trajectory_values[2].as_deref(),
            "invalid final trajectory prediction error",
        )?,
    })
}

fn load_trajectory_wall_ride_tuning(
    mounts: &MountSet,
) -> Result<TrajectoryWallRideTuning, Box<dyn std::error::Error>> {
    // These two physics_feet keys are preserved from the executable because
    // their generated schema names are absent from the available PDB types.
    const MAXIMUM_COLLISION_NORMAL_Y_KEY: u64 = 0xBABA_9160_6102_AF9A;
    const MAXIMUM_GROUND_WALL_NORMAL_DOT_KEY: u64 = 0x90C4_E79C_BFBA_ED5E;

    let trajectory_values = load_attribute_values(
        mounts,
        "physics_trajectory",
        "default",
        &[
            string_hash64("WallRideBoost"),
            string_hash64("WallRideTestDist"),
            string_hash64("WallRideMinHeightFromGround"),
            string_hash64("WallRideAngleToAllowLanding"),
            string_hash64("ScoreWallrideHeight"),
        ],
    )?;
    let feet_values = load_attribute_values(
        mounts,
        "physics_feet",
        "default",
        &[
            MAXIMUM_COLLISION_NORMAL_Y_KEY,
            MAXIMUM_GROUND_WALL_NORMAL_DOT_KEY,
        ],
    )?;
    Ok(TrajectoryWallRideTuning {
        maximum_collision_normal_y: optional_be_f32(
            feet_values[0].as_deref(),
            "invalid wallride collision-normal threshold",
        )?,
        maximum_ground_wall_normal_dot: optional_be_f32(
            feet_values[1].as_deref(),
            "invalid wallride ground-normal threshold",
        )?,
        boost_vs_height: decode_point_graph(required_attribute(&trajectory_values, 0)?, 16)?,
        test_distance: optional_be_f32(
            trajectory_values[1].as_deref(),
            "invalid wallride test distance",
        )?,
        minimum_height_from_ground: optional_be_f32(
            trajectory_values[2].as_deref(),
            "invalid minimum wallride height",
        )?,
        maximum_landing_slope: optional_be_f32(
            trajectory_values[3].as_deref(),
            "invalid wallride landing-slope threshold",
        )?,
        score_per_height: optional_be_f32(
            trajectory_values[4].as_deref(),
            "invalid wallride height score",
        )?,
    })
}

fn required_attribute(values: &[Option<Vec<u8>>], index: usize) -> Result<&[u8], &'static str> {
    values
        .get(index)
        .and_then(Option::as_deref)
        .ok_or("required physics attribute uses a null value")
}

fn decode_point_graph<const N: usize>(
    bytes: &[u8],
    prefix_bytes: usize,
) -> Result<PointGraph<N>, &'static str> {
    let values = bytes
        .get(prefix_bytes..prefix_bytes + N * 8)
        .ok_or("point graph attribute is truncated")?;
    let mut x = [0.0; N];
    let mut y = [0.0; N];
    for index in 0..N {
        x[index] = f32::from_be_bytes(
            values[index * 4..index * 4 + 4]
                .try_into()
                .map_err(|_| "invalid point graph x value")?,
        );
        let y_offset = (N + index) * 4;
        y[index] = f32::from_be_bytes(
            values[y_offset..y_offset + 4]
                .try_into()
                .map_err(|_| "invalid point graph y value")?,
        );
    }
    Ok(PointGraph { x, y })
}

fn report_physics_tuning(mounts: &MountSet) -> Result<(), Box<dyn std::error::Error>> {
    let ground = load_ground_state_tuning(mounts)?;
    let jump = load_jump_physics_curves(mounts)?;
    let ground_jump = load_ground_jump_tuning(mounts)?;
    let trajectory = load_trajectory_selection_tuning(mounts)?;
    let grind_deck = load_grind_deck_tuning(mounts)?;
    let grind_geometry = load_grind_geometry_tuning(mounts)?;
    let trajectory_prediction = load_trajectory_prediction_tuning(mounts)?;
    let wall_ride = load_trajectory_wall_ride_tuning(mounts)?;
    println!("grounded physics-state tuning: {ground:?}");
    println!("ground-jump physics curves: {jump:?}");
    println!("ground-jump scalar tuning: {ground_jump:?}");
    println!("trajectory-selection tuning: {trajectory:?}");
    println!("grind-deck tuning: {grind_deck:?}");
    println!("grind-geometry tuning: {grind_geometry:?}");
    println!("trajectory-prediction tuning: {trajectory_prediction:?}");
    println!("trajectory-wallride tuning: {wall_ride:?}");
    Ok(())
}

fn load_grind_deck_tuning(
    mounts: &MountSet,
) -> Result<GrindDeckTuning, Box<dyn std::error::Error>> {
    // `AnalyzeAndAdjustTrajectory` constructs both keys directly. Their
    // generated schema names are absent from the PDB and executable strings.
    const DECK_GRIND_EXTENT_A_KEY: u64 = 0xC94D_6509_1DCB_828B;
    const DECK_GRIND_EXTENT_B_KEY: u64 = 0x49F4_29AE_2E66_F063;
    let values = load_attribute_values(
        mounts,
        "physicsdeck",
        "default",
        &[DECK_GRIND_EXTENT_A_KEY, DECK_GRIND_EXTENT_B_KEY],
    )?;
    let extent_a = optional_be_f32(values[0].as_deref(), "invalid first deck grind extent")?;
    let extent_b = optional_be_f32(values[1].as_deref(), "invalid second deck grind extent")?;
    Ok(GrindDeckTuning {
        contact_footprint_radius: extent_a.mul_add(0.5, extent_b * 0.5),
    })
}

fn load_grind_geometry_tuning(
    mounts: &MountSet,
) -> Result<GrindGeometryTuning, Box<dyn std::error::Error>> {
    const GEOMETRY_COLLISION_OFFSET_KEY: u64 = 0xCBFE_FFEA_12F4_CC27;

    let values = load_attribute_values(
        mounts,
        "physics_grinds",
        "default",
        &[GEOMETRY_COLLISION_OFFSET_KEY],
    )?;
    Ok(GrindGeometryTuning {
        collision_offset: optional_be_f32(
            values[0].as_deref(),
            "invalid grind geometry collision offset",
        )?,
        ..GrindGeometryTuning::default()
    })
}

fn load_dynamic_object_census_circles(
    mounts: &MountSet,
) -> Result<[LivingWorldCensusCircle; 2], Box<dyn std::error::Error>> {
    let values = load_attribute_values(
        mounts,
        "livingworld_census_ranges",
        "dynamicobjects",
        &[
            string_hash64("MinSpeed_CensusCircleInfo"),
            string_hash64("MaxSpeed_CensusCircleInfo"),
        ],
    )?;
    Ok([
        decode_living_world_census_circle(required_attribute(&values, 0)?)?,
        decode_living_world_census_circle(required_attribute(&values, 1)?)?,
    ])
}

fn load_dynamic_object_priority_weights(
    mounts: &MountSet,
) -> Result<DynamicObjectPriorityWeights, Box<dyn std::error::Error>> {
    const LIVING_WORLD_DEFAULT_COLLECTION_ID: u64 = 0x20F0_4D29_5272_EE46;

    let values = load_attribute_values_by_id(
        mounts,
        string_hash64("livingworld"),
        LIVING_WORLD_DEFAULT_COLLECTION_ID,
        &[
            string_hash64("WeightFactor_CameraPlane"),
            string_hash64("WeightFactor_InUse"),
            string_hash64("WeightFactor_Priority"),
            string_hash64("WeightFactor_Safety"),
            string_hash64("WeightFactor_Distance"),
        ],
        "livingworld default",
    )?;
    let value =
        |index: usize, error: &'static str| optional_be_f32(values[index].as_deref(), error);
    Ok(DynamicObjectPriorityWeights {
        camera_plane: value(0, "invalid DMO camera-plane weight")?,
        in_use: value(1, "invalid DMO in-use weight")?,
        priority: value(2, "invalid DMO priority weight")?,
        safety: value(3, "invalid DMO safety weight")?,
        distance: value(4, "invalid DMO distance weight")?,
    })
}

fn load_attribute_values(
    mounts: &MountSet,
    class_name: &str,
    collection_name: &str,
    keys: &[u64],
) -> Result<Vec<Option<Vec<u8>>>, Box<dyn std::error::Error>> {
    load_attribute_values_by_id(
        mounts,
        string_hash64(class_name),
        string_hash64(collection_name),
        keys,
        &format!("{class_name}.{collection_name}"),
    )
}

fn load_attribute_values_by_id(
    mounts: &MountSet,
    class_id: u64,
    collection_id: u64,
    keys: &[u64],
    context: &str,
) -> Result<Vec<Option<Vec<u8>>>, Box<dyn std::error::Error>> {
    let vlt = mounts.read_path("data\\db\\skater.vlt")?;
    let bin = mounts.read_path("data\\db\\skater.bin")?;
    let vault = AttributeVaultMetadata::parse(&vlt)?;
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()])?;
    let buffers = [vlt.as_slice(), bin.as_slice()];
    let class = vault
        .exports
        .iter()
        .find(|export| {
            export.kind() == skate2_formats::attrib::AttributeExportKind::Class
                && export.object_id == class_id
        })
        .ok_or_else(|| format!("missing {context} attribute class {class_id:016x}"))?;
    let collections = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Collection)
        .map(|export| {
            let collection = export.collection_load_data(&vlt)?;
            Ok((
                (collection.class_id, collection.collection_id),
                (export, collection),
            ))
        })
        .collect::<Result<AttributeCollectionMap<'_>, skate2_formats::attrib::AttributeChunkError>>(
        )?;
    let (collection_export, collection) =
        collections
            .get(&(class_id, collection_id))
            .ok_or_else(|| {
                let available = collections
                    .keys()
                    .filter_map(|&(candidate_class, candidate_collection)| {
                        (candidate_class == class_id).then_some(candidate_collection)
                    })
                    .collect::<Vec<_>>();
                format!(
                    "missing {context} attribute collection {collection_id:016x}; available {available:016x?}"
                )
            })?;
    let definitions = class.class_definitions(&buffers, &pointers)?;
    keys.iter()
        .map(|&key| {
            let definition = definitions
                .iter()
                .find(|definition| definition.key == key)
                .ok_or_else(|| format!("missing {context} attribute key {key:016x}"))?;
            resolve_attribute_value(
                &buffers,
                &pointers,
                &collections,
                class,
                collection_export,
                collection,
                definition,
            )
            .map_err(|error| -> Box<dyn std::error::Error> {
                format!("{context} attribute {key:016x}: {error}").into()
            })
        })
        .collect()
}

fn resolve_attribute_value(
    buffers: &[&[u8]],
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
    collections: &AttributeCollectionMap<'_>,
    class: &AttributeExport,
    collection_export: &AttributeExport,
    collection: &AttributeCollectionLoadData,
    definition: &AttributeDefinition,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    if let Some(value) = resolve_inline_attribute_value(collection, definition, collections)? {
        return Ok(Some(value));
    }
    let Some((dependency, offset)) = resolve_attribute_location(
        class,
        collection_export,
        collection,
        definition,
        collections,
        pointers,
    )?
    else {
        return Ok(None);
    };
    let bytes = attribute_bytes(buffers, dependency, offset, usize::from(definition.size))
        .ok_or("attribute value is outside its dependency")?;
    Ok(Some(bytes.to_vec()))
}

fn load_dmo_priority_indices(
    mounts: &MountSet,
    model_attribute_pairs: &BTreeMap<u64, (u64, u64)>,
) -> Result<BTreeMap<u64, i32>, Box<dyn std::error::Error>> {
    let vlt = mounts.read_path("data\\db\\skater.vlt")?;
    let bin = mounts.read_path("data\\db\\skater.bin")?;
    let vault = AttributeVaultMetadata::parse(&vlt)?;
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()])?;
    let buffers = [vlt.as_slice(), bin.as_slice()];
    let collections = vault
        .exports
        .iter()
        .filter(|export| export.kind() == skate2_formats::attrib::AttributeExportKind::Collection)
        .map(|export| {
            let collection = export.collection_load_data(&vlt)?;
            Ok((
                (collection.class_id, collection.collection_id),
                (export, collection),
            ))
        })
        .collect::<Result<AttributeCollectionMap<'_>, skate2_formats::attrib::AttributeChunkError>>(
        )?;
    let characteristics_class = vault
        .exports
        .iter()
        .find(|export| {
            export.kind() == skate2_formats::attrib::AttributeExportKind::Class
                && export.object_id == LIVING_WORLD_DMO_CHARACTERISTICS_CLASS_ID
        })
        .ok_or("missing DMO characteristics class")?;
    let priority_class_id = string_hash64("livingworld_dynamicobject_priority");
    let priority_class = vault
        .exports
        .iter()
        .find(|export| {
            export.kind() == skate2_formats::attrib::AttributeExportKind::Class
                && export.object_id == priority_class_id
        })
        .ok_or("missing DMO priority class")?;
    let characteristics_definition = characteristics_class
        .class_definitions(&buffers, &pointers)?
        .into_iter()
        .find(|definition| definition.key == LIVING_WORLD_DMO_PRIORITY_REFERENCE_KEY)
        .ok_or("missing DMO priority-reference definition")?;
    let priority_definition = priority_class
        .class_definitions(&buffers, &pointers)?
        .into_iter()
        .find(|definition| definition.key == string_hash64("PriorityIndex"))
        .ok_or("missing DMO PriorityIndex definition")?;
    let mut priorities = BTreeMap::new();
    for (&model_id, &(_, characteristics_collection_id)) in model_attribute_pairs {
        let (characteristics_export, characteristics_collection) = collections
            .get(&(
                LIVING_WORLD_DMO_CHARACTERISTICS_CLASS_ID,
                characteristics_collection_id,
            ))
            .ok_or("missing DMO characteristics collection")?;
        let reference = resolve_attribute_value(
            &buffers,
            &pointers,
            &collections,
            characteristics_class,
            characteristics_export,
            characteristics_collection,
            &characteristics_definition,
        )?
        .ok_or("DMO characteristics have no priority reference")?;
        let priority_collection_id = u64::from_be_bytes(
            reference
                .get(..8)
                .ok_or("truncated DMO priority reference")?
                .try_into()?,
        );
        let (priority_export, priority_collection) = collections
            .get(&(priority_class_id, priority_collection_id))
            .ok_or("missing DMO priority collection")?;
        let priority = resolve_attribute_value(
            &buffers,
            &pointers,
            &collections,
            priority_class,
            priority_export,
            priority_collection,
            &priority_definition,
        )?
        .ok_or("DMO priority collection has no PriorityIndex")?;
        priorities.insert(
            model_id,
            i32::from_be_bytes(priority.as_slice().try_into()?),
        );
    }
    Ok(priorities)
}

fn resolve_inline_attribute_value(
    collection: &AttributeCollectionLoadData,
    definition: &AttributeDefinition,
    collections: &AttributeCollectionMap<'_>,
) -> Result<Option<Vec<u8>>, &'static str> {
    let mut current = collection;
    loop {
        if let Some(entry) = current
            .entries
            .iter()
            .find(|entry| entry.key == definition.key)
        {
            // Collection node flag 0x40 stores values of up to four bytes in
            // the node's pointer word itself. The word is serialized in the
            // vault's big-endian byte order; it is not a pointer fixup.
            if let Some(bytes) = entry.inline_value_word() {
                let length = usize::from(definition.size);
                return bytes
                    .get(..length)
                    .map(|value| Some(value.to_vec()))
                    .ok_or("inline attribute is larger than its storage word");
            }
            return Ok(None);
        }
        if current.parent_id == 0 {
            return Ok(None);
        }
        let parent = collections
            .get(&(current.class_id, current.parent_id))
            .ok_or("missing parent")?;
        current = &parent.1;
    }
}

fn optional_be_f32(value: Option<&[u8]>, error: &'static str) -> Result<f32, &'static str> {
    value.map_or(Ok(0.0), |bytes| {
        Ok(f32::from_be_bytes(bytes.try_into().map_err(|_| error)?))
    })
}

#[cfg(test)]
mod attribute_tuning_tests {
    use super::*;

    #[test]
    fn decodes_reference_ground_state_inline_float_words() {
        // `physics_airstates.default` stores these three values directly in
        // flag-0x40 collection-node pointer words in the supplied reference
        // Vault. Keep them as serialized big-endian bytes at this boundary.
        let tuning = decode_ground_state_tuning_values(
            Some(&0x3DF5_C28F_u32.to_be_bytes()),
            Some(&0x3D75_C28F_u32.to_be_bytes()),
            Some(&0x3E4C_CCCD_u32.to_be_bytes()),
        )
        .expect("reference inline floats should decode");

        assert_eq!(tuning.clear_ground_height.to_bits(), 0x3DF5_C28F);
        assert_eq!(tuning.recent_contact_ground_height.to_bits(), 0x3D75_C28F);
        assert_eq!(tuning.wheel_contact_grace_seconds.to_bits(), 0x3E4C_CCCD);
    }
}

fn resolve_material_attributes(
    class: &AttributeExport,
    collection_export: &AttributeExport,
    collection: &AttributeCollectionLoadData,
    classes: &BTreeMap<u64, &AttributeExport>,
    collections: &AttributeCollectionMap<'_>,
    buffers: &[&[u8]],
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
) -> Result<PresentationMaterialBinding, &'static str> {
    let manifest = resolve_material_manifest(
        class,
        collection_export,
        collection,
        collections,
        buffers,
        pointers,
    )?;
    let draw_order = resolve_material_draw_order(
        class,
        collection_export,
        collection,
        classes,
        collections,
        buffers,
        pointers,
    )?;
    Ok(PresentationMaterialBinding {
        manifest_name: manifest,
        draw_order,
    })
}

fn resolve_material_manifest(
    class: &AttributeExport,
    collection_export: &AttributeExport,
    collection: &AttributeCollectionLoadData,
    collections: &AttributeCollectionMap<'_>,
    buffers: &[&[u8]],
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
) -> Result<String, &'static str> {
    let definition = class
        .class_definitions(buffers, pointers)
        .map_err(|_| "invalid definitions")?
        .into_iter()
        .find(|definition| definition.key == string_hash64("effect"))
        .ok_or("no effect definition")?;
    let Some((dependency, offset)) = resolve_attribute_location(
        class,
        collection_export,
        collection,
        &definition,
        collections,
        pointers,
    )?
    else {
        return Ok("default".to_owned());
    };
    let Some((string_dependency, string_offset)) =
        attribute_direct_pointer_u64(pointers, dependency, offset)
    else {
        // CreateEffectListFromMaterialKey initializes the name to this literal
        // and only replaces it when the attribute pointer is non-null.
        return Ok("default".to_owned());
    };
    attribute_string(buffers, string_dependency, string_offset).ok_or("invalid effect string")
}

fn resolve_material_draw_order(
    class: &AttributeExport,
    collection_export: &AttributeExport,
    collection: &AttributeCollectionLoadData,
    classes: &BTreeMap<u64, &AttributeExport>,
    collections: &AttributeCollectionMap<'_>,
    buffers: &[&[u8]],
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
) -> Result<i32, &'static str> {
    let Some(definition) = class
        .class_definitions(buffers, pointers)
        .map_err(|_| "invalid definitions")?
        .into_iter()
        .find(|definition| definition.key == string_hash64("DrawOrder"))
    else {
        return Ok(0);
    };
    let Some((dependency, offset)) = resolve_attribute_location(
        class,
        collection_export,
        collection,
        &definition,
        collections,
        pointers,
    )?
    else {
        return Ok(0);
    };
    let reference = attribute_bytes(buffers, dependency, offset, 16).ok_or("invalid DrawOrder")?;
    let class_id = u64::from_be_bytes(
        reference[0..8]
            .try_into()
            .map_err(|_| "invalid DrawOrder")?,
    );
    let collection_id = u64::from_be_bytes(
        reference[8..16]
            .try_into()
            .map_err(|_| "invalid DrawOrder")?,
    );
    if class_id == 0 || collection_id == 0 {
        return Ok(0);
    }

    let draw_class = classes.get(&class_id).ok_or("missing DrawOrder class")?;
    let (draw_collection_export, draw_collection) = collections
        .get(&(class_id, collection_id))
        .ok_or("missing DrawOrder collection")?;
    let guid_definition = draw_class
        .class_definitions(buffers, pointers)
        .map_err(|_| "invalid DrawOrder definitions")?
        .into_iter()
        .find(|definition| definition.key == string_hash64("GUID"))
        .ok_or("no DrawOrder GUID definition")?;
    let Some((guid_dependency, guid_offset)) = resolve_attribute_location(
        draw_class,
        draw_collection_export,
        draw_collection,
        &guid_definition,
        collections,
        pointers,
    )?
    else {
        return Ok(0);
    };
    let guid = attribute_bytes(buffers, guid_dependency, guid_offset, 4)
        .ok_or("invalid DrawOrder GUID")?;
    Ok(i32::from_be_bytes(
        guid.try_into().map_err(|_| "invalid DrawOrder GUID")?,
    ))
}

fn resolve_attribute_location(
    class: &AttributeExport,
    collection_export: &AttributeExport,
    collection: &AttributeCollectionLoadData,
    definition: &AttributeDefinition,
    collections: &AttributeCollectionMap<'_>,
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
) -> Result<Option<(u16, u64)>, &'static str> {
    // Collection::GetNode searches the collection's explicit hash table and
    // every parent table first. Only when that chain is exhausted does it
    // consult the original collection's class layout table. This precedence
    // matters when a parent overrides a layout/static class default.
    let mut current_export = collection_export;
    let mut current = collection;
    loop {
        if let Some(location) =
            resolve_collection_entry_location(current_export, current, definition, pointers)?
        {
            return Ok(Some(location));
        }
        if current.parent_id == 0 {
            break;
        }
        let parent = collections
            .get(&(current.class_id, current.parent_id))
            .ok_or("missing parent")?;
        current_export = parent.0;
        current = &parent.1;
    }

    Ok(resolve_class_default_attribute_location(
        class,
        collection_export,
        definition,
        pointers,
    ))
}

fn resolve_collection_entry_location(
    collection_export: &AttributeExport,
    collection: &AttributeCollectionLoadData,
    definition: &AttributeDefinition,
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
) -> Result<Option<(u16, u64)>, &'static str> {
    let Some(entry_index) = collection
        .entries
        .iter()
        .position(|entry| entry.key == definition.key)
    else {
        return Ok(None);
    };
    let entries_start = collection_export
        .data_offset
        .checked_add(48)
        .and_then(|offset| {
            u32::from(collection.types_length)
                .checked_mul(8)
                .and_then(|types_size| offset.checked_add(types_size))
        })
        .ok_or("collection entry offset overflow")?;
    let pointer_slot = entries_start
        .checked_add(
            u32::try_from(entry_index)
                .map_err(|_| "collection entry index overflow")?
                .checked_mul(16)
                .ok_or("collection entry offset overflow")?,
        )
        .and_then(|offset| offset.checked_add(8))
        .ok_or("collection entry offset overflow")?;
    // Node::GetPointer: an inline node (0x40) keeps its value in the node's own
    // pointer word, so the attribute lives at that slot, as in the core
    // AttributeRuntime inline branch. Only a stored-pointer node dereferences it.
    if collection.entries[entry_index].node_flags
        & skate2_formats::attrib::INLINE_COLLECTION_VALUE_FLAG
        != 0
    {
        return Ok(Some((0, u64::from(pointer_slot))));
    }
    attribute_direct_pointer(pointers, 0, pointer_slot)
        .map(Some)
        .ok_or("collection entry has no direct value pointer")
}

fn resolve_class_default_attribute_location(
    class: &AttributeExport,
    collection_export: &AttributeExport,
    definition: &AttributeDefinition,
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
) -> Option<(u16, u64)> {
    if definition.flags & 0x10 != 0 {
        let static_pointer_slot = class.data_offset.checked_add(24)?;
        let (dependency, offset) = attribute_direct_pointer(pointers, 0, static_pointer_slot)?;
        Some((
            dependency,
            offset.checked_add(u64::from(definition.offset))?,
        ))
    } else if definition.flags & 2 != 0 {
        let layout_pointer_slot = collection_export.data_offset.checked_add(40)?;
        let (dependency, offset) = attribute_direct_pointer(pointers, 0, layout_pointer_slot)?;
        Some((
            dependency,
            offset.checked_add(u64::from(definition.offset))?,
        ))
    } else {
        None
    }
}

fn attribute_direct_pointer(
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
    dependency_index: u16,
    offset: u32,
) -> Option<(u16, u64)> {
    match pointers.get(&AttributePointerLocation {
        dependency_index,
        offset,
    })? {
        AttributePointerValue::Direct {
            dependency_index,
            offset,
        } => Some((*dependency_index, *offset)),
        _ => None,
    }
}

fn attribute_direct_pointer_u64(
    pointers: &BTreeMap<AttributePointerLocation, AttributePointerValue>,
    dependency_index: u16,
    offset: u64,
) -> Option<(u16, u64)> {
    attribute_direct_pointer(pointers, dependency_index, u32::try_from(offset).ok()?)
}

fn attribute_string(buffers: &[&[u8]], dependency_index: u16, offset: u64) -> Option<String> {
    let bytes = *buffers.get(usize::from(dependency_index))?;
    let tail = bytes.get(usize::try_from(offset).ok()?..)?;
    let end = tail.iter().position(|byte| *byte == 0)?;
    std::str::from_utf8(&tail[..end]).ok().map(str::to_owned)
}

fn attribute_bytes<'a>(
    buffers: &[&'a [u8]],
    dependency_index: u16,
    offset: u64,
    length: usize,
) -> Option<&'a [u8]> {
    let bytes = *buffers.get(usize::from(dependency_index))?;
    let start = usize::try_from(offset).ok()?;
    let end = start.checked_add(length)?;
    bytes.get(start..end)
}

fn finish_presentation_inventory(
    mounts: &MountSet,
    inventory: &PresentationInventory,
    dmo_model_ids: Option<&BTreeSet<u64>>,
) -> Result<(), Box<dyn std::error::Error>> {
    inventory_material_attribute_bindings(mounts, &inventory.material_effect_names)?;
    println!(
        "BAM material raster reference samples: {:?}",
        inventory.raster_reference_samples
    );
    println!(
        "BAM mesh material references: {} total / {} exact inline / {} unresolved; sections {:?}; subreference types {:?}; base object types {:?}",
        inventory.material_references,
        inventory.resolved_material_references,
        inventory.unresolved_material_references,
        inventory.material_reference_sections,
        inventory.material_subreference_types,
        inventory.material_subreference_base_types,
    );
    println!(
        "BAM vertex declarations: semantic formats {:?}; methods {:?}; element types {:?}; element classes {:?}; streams {:?}",
        inventory.vertex_semantic_formats,
        inventory.vertex_methods,
        inventory.vertex_element_types,
        inventory.vertex_element_classes,
        inventory.vertex_streams,
    );
    println!(
        "BAM native vertex attributes: {} arrays / {} values",
        inventory.decoded_vertex_attributes, inventory.decoded_vertex_attribute_values,
    );
    println!(
        "BAM material effect names: {:?}",
        inventory.material_effect_names
    );
    println!(
        "BAM non-finite vertex attributes: {:?}; samples {:?}; owners {:?}",
        inventory.non_finite_vertex_attributes,
        inventory.non_finite_vertex_samples,
        inventory.non_finite_vertex_owners,
    );
    println!(
        "BAM texture stream: {} collections / {} arenas / {} objects / {} TOC entries; {} level surfaces / {} linear bytes / {} decode failures; texture types {:?}; endian modes {:?}; formats {:?}; arena types {:?}; TOC object types {:?}; link sections {:?}; target types {:?}",
        inventory.texture_stream_collections,
        inventory.texture_stream_arenas,
        inventory.texture_stream_objects,
        inventory.texture_stream_toc_entries,
        inventory.texture_stream_decoded_levels,
        inventory.texture_stream_decoded_bytes,
        inventory.texture_stream_decode_failures,
        inventory.texture_stream_texture_types,
        inventory.texture_stream_endian_modes,
        inventory.texture_stream_texture_formats,
        inventory.texture_stream_object_types,
        inventory.texture_toc_object_types,
        inventory.texture_toc_link_sections,
        inventory.texture_toc_target_types,
    );
    println!(
        "BAM texture mips: levels {:?}; packed {:?}; data offsets {:?}; {} zero-offset packed bases; {} expected payload matches / deltas {:?}",
        inventory.texture_stream_mip_levels,
        inventory.texture_stream_packed_mips,
        inventory.texture_stream_mip_data_offsets,
        inventory.texture_stream_zero_offset_mip_tails,
        inventory.texture_stream_expected_payload_matches,
        inventory.texture_stream_expected_payload_deltas,
    );
    println!(
        "BAM embedded data: {} records; groups {:?}; reference sections {:?}; local target types {:?}; subreference types {:?}; subreference base types {:?}; external arena IDs {:?}; samples {:?}",
        inventory.embedded_data,
        inventory.embedded_reference_groups,
        inventory.embedded_reference_sections,
        inventory.embedded_local_target_types,
        inventory.embedded_subreference_types,
        inventory.embedded_subreference_base_types,
        inventory.embedded_external_arena_ids,
        inventory.embedded_reference_samples,
    );
    println!(
        "BAM presentation instances: {} containers / {} instances / {} local GUIDs; DMO model-ID matches {:?}",
        inventory.instance_data,
        inventory.instances,
        inventory.instance_local_guids.len(),
        dmo_model_ids.map(|ids| inventory.instance_local_guids.intersection(ids).count()),
    );
    if inventory.failures.is_empty() {
        Ok(())
    } else {
        Err("presentation objects or links failed validation".into())
    }
}

fn inventory_texture_asset_indices(
    mounts: &MountSet,
    inventory: &mut PresentationInventory,
) -> Result<(BTreeSet<u64>, BTreeSet<u64>), Box<dyn std::error::Error>> {
    let assets =
        AssetToc::parse(&mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Tex.xst")?)?;
    let asset_ids = assets.assets.iter().map(|asset| asset.id).collect();
    for asset in &assets.assets {
        *inventory
            .texture_asset_types
            .entry(asset.asset_type)
            .or_default() += 1;
    }
    let toc_guids = collect_texture_stream_toc_guids(
        mounts,
        "data\\content\\world\\stream\\BAM\\BAM_Tex.xsm",
        "data\\content\\world\\stream\\BAM",
        inventory,
    )?;
    Ok((asset_ids, toc_guids))
}

fn inventory_presentation_object(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    type_id: u32,
    bytes: &[u8],
    inventory: &mut PresentationInventory,
) {
    match type_id {
        R_MODEL_DATA_TYPE_ID => inventory_render_model(arena, bytes, inventory),
        R_MATERIAL_DATA_TYPE_ID => inventory_render_material(bytes, inventory),
        R_MESH_DATA_TYPE_ID => inventory_render_mesh(arena, bytes, inventory),
        R_OPTI_MESH_DATA_TYPE_ID => inventory_optimized_mesh(arena, bytes, inventory),
        VERTEX_DESCRIPTOR_TYPE_ID => inventory_vertex_descriptor(bytes, inventory),
        VERTEX_BUFFER_TYPE_ID => inventory_vertex_buffer(arena, bytes, inventory),
        INDEX_BUFFER_TYPE_ID => inventory_index_buffer(arena, bytes, inventory),
        MESH_HELPER_TYPE_ID => inventory_mesh_helper(arena, bytes, inventory),
        TEXTURE_TYPE_ID => inventory_texture(arena, bytes, inventory),
        INSTANCE_DATA_TYPE_ID => match InstanceData::parse(bytes) {
            Ok(data) => {
                inventory.instance_data += 1;
                inventory.instances += data.instances.len();
                inventory.instance_local_guids.extend(
                    data.instances
                        .into_iter()
                        .map(|instance| instance.local_guid),
                );
            }
            Err(_) => record_presentation_failure(inventory, "instance data parse"),
        },
        EMBEDDED_DATA_TYPE_ID => {
            inventory_presentation_embedded_data(arena, sections, bytes, inventory);
        }
        _ => {}
    }
}

fn inventory_presentation_embedded_data(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    bytes: &[u8],
    inventory: &mut PresentationInventory,
) {
    let Ok(data) = EmbeddedData::parse(bytes) else {
        record_presentation_failure(inventory, "embedded data parse");
        return;
    };
    inventory.embedded_data += 1;
    for (group_index, group) in data.reference_groups.iter().enumerate() {
        inventory.embedded_reference_groups[group_index] += group.len();
        for reference in group {
            inventory_presentation_embedded_reference(
                arena,
                sections,
                group_index,
                reference.key,
                reference.object,
                inventory,
            );
        }
    }
}

fn inventory_presentation_embedded_reference(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    group_index: usize,
    key: u32,
    id: ObjectId,
    inventory: &mut PresentationInventory,
) {
    let section = id.section_index();
    *inventory
        .embedded_reference_sections
        .entry((group_index, section))
        .or_default() += 1;
    let description = match section {
        0 => {
            if let Some(entry) = arena.local_entry(id) {
                *inventory
                    .embedded_local_target_types
                    .entry((group_index, entry.type_id))
                    .or_default() += 1;
                format!("local type {:08x}", entry.type_id)
            } else {
                record_presentation_failure(inventory, "embedded local reference");
                "invalid local reference".to_owned()
            }
        }
        1 => "null arena".to_owned(),
        2 => {
            if let Some(subreference) = usize::try_from(id.entry_index())
                .ok()
                .and_then(|index| sections.subreferences.get(index))
            {
                *inventory
                    .embedded_subreference_types
                    .entry((group_index, subreference.type_id))
                    .or_default() += 1;
                let base_type = arena
                    .local_entry(subreference.object)
                    .map(|entry| entry.type_id);
                if let Some(base_type) = base_type {
                    *inventory
                        .embedded_subreference_base_types
                        .entry((group_index, base_type))
                        .or_default() += 1;
                }
                let valid_base = arena
                    .local_entry(subreference.object)
                    .is_some_and(|entry| subreference.object_offset < entry.size);
                if !valid_base {
                    record_presentation_failure(inventory, "embedded subreference");
                }
                format!(
                    "subref type {:08x}, base {:?}+{:x}",
                    subreference.type_id, base_type, subreference.object_offset
                )
            } else {
                record_presentation_failure(inventory, "embedded subreference");
                "invalid subreference".to_owned()
            }
        }
        _ => {
            if let Some(arena_id) = sections.external_arena_id(section) {
                *inventory
                    .embedded_external_arena_ids
                    .entry((group_index, arena_id))
                    .or_default() += 1;
                format!("external arena {arena_id:08x}")
            } else {
                record_presentation_failure(inventory, "embedded external reference");
                "invalid external reference".to_owned()
            }
        }
    };
    if inventory.embedded_reference_samples.len() < 24 {
        inventory.embedded_reference_samples.push(format!(
            "group {group_index}, key {key:08x}, id {:08x}: {description}",
            id.0
        ));
    }
}

fn inventory_render_model(arena: &Rw4Arena, bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(model) = RenderModelData::parse(bytes) else {
        record_presentation_failure(inventory, "render model parse");
        return;
    };
    inventory.models += 1;
    inventory.model_meshes += model.meshes.len();
    for mesh in model.meshes {
        validate_presentation_link(
            arena,
            mesh.mesh,
            &[R_MESH_DATA_TYPE_ID, R_OPTI_MESH_DATA_TYPE_ID],
            inventory,
            "model mesh link",
        );
    }
}

fn inventory_render_material(bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(data) = RenderMaterialData::parse(bytes) else {
        record_presentation_failure(inventory, "render material parse");
        return;
    };
    inventory.material_blocks += 1;
    inventory.materials += data.materials.len();
    inventory.material_channels += data.channels.len();
    for material in data.materials {
        *inventory.material_flags.entry(material.flags).or_default() += 1;
    }
    for channel in data.channels {
        *inventory.channel_flags.entry(channel.flags).or_default() += 1;
        let shader_input = channel.shader_input;
        if shader_input == "AttribulatorMaterialName"
            && let MaterialChannelValue::Raster { stream_name, .. } = &channel.value
        {
            *inventory
                .material_effect_names
                .entry(stream_name.clone())
                .or_default() += 1;
        }
        *inventory.shader_inputs.entry(shader_input).or_default() += 1;
        match channel.value {
            MaterialChannelValue::Raster { .. } => inventory.raster_channels += 1,
            MaterialChannelValue::ScalarConstant(_) => inventory.constant_channels += 1,
        }
    }
}

fn inventory_texture(arena: &Rw4Arena, bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(texture) = Texture::parse(bytes) else {
        record_presentation_failure(inventory, "texture parse");
        return;
    };
    inventory.textures += 1;
    *inventory
        .texture_types
        .entry(texture.texture_type)
        .or_default() += 1;
    *inventory
        .texture_formats
        .entry(texture.pixel_format)
        .or_default() += 1;
    if texture.width == 0 || texture.height == 0 || texture.depth == 0 {
        record_presentation_failure(inventory, "zero texture dimension");
    }
    validate_presentation_link(
        arena,
        texture.data,
        &[GPU_BUFFER_RESOURCE_TYPE_ID],
        inventory,
        "texture data link",
    );
}

fn inventory_render_mesh(arena: &Rw4Arena, bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(mesh) = RenderMeshData::parse(bytes) else {
        record_presentation_failure(inventory, "render mesh parse");
        return;
    };
    inventory.meshes += 1;
    validate_presentation_link(
        arena,
        mesh.vertex_descriptor,
        &[VERTEX_DESCRIPTOR_TYPE_ID],
        inventory,
        "mesh vertex descriptor link",
    );
    validate_presentation_link(
        arena,
        mesh.mesh_helper,
        &[MESH_HELPER_TYPE_ID],
        inventory,
        "mesh helper link",
    );
    validate_optional_presentation_link(
        arena,
        mesh.index_buffer,
        INDEX_BUFFER_TYPE_ID,
        inventory,
        "mesh index buffer link",
    );
    validate_optional_presentation_link(
        arena,
        mesh.vertex_buffer,
        VERTEX_BUFFER_TYPE_ID,
        inventory,
        "mesh vertex buffer link",
    );
}

fn inventory_optimized_mesh(arena: &Rw4Arena, bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(mesh) = OptimizedRenderMeshData::parse(bytes) else {
        record_presentation_failure(inventory, "optimized render mesh parse");
        return;
    };
    inventory.optimized_meshes += 1;
    for (link, expected, failure) in [
        (
            mesh.vertex_descriptor,
            VERTEX_DESCRIPTOR_TYPE_ID,
            "optimized mesh vertex descriptor link",
        ),
        (
            mesh.mesh_helper,
            MESH_HELPER_TYPE_ID,
            "optimized mesh helper link",
        ),
        (
            mesh.index_buffer,
            INDEX_BUFFER_TYPE_ID,
            "optimized mesh index buffer link",
        ),
        (
            mesh.vertex_buffer,
            VERTEX_BUFFER_TYPE_ID,
            "optimized mesh vertex buffer link",
        ),
    ] {
        validate_optional_presentation_link(arena, link, expected, inventory, failure);
    }
}

fn inventory_vertex_descriptor(bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(descriptor) = VertexDescriptor::parse(bytes) else {
        record_presentation_failure(inventory, "vertex descriptor parse");
        return;
    };
    inventory.vertex_descriptors += 1;
    inventory.vertex_elements += descriptor.elements.len();
    for element in descriptor.elements {
        *inventory.vertex_formats.entry(element.format).or_default() += 1;
        *inventory
            .vertex_semantic_formats
            .entry((element.usage, element.usage_index, element.format))
            .or_default() += 1;
        *inventory.vertex_methods.entry(element.method).or_default() += 1;
        *inventory
            .vertex_element_types
            .entry(element.element_type)
            .or_default() += 1;
        *inventory
            .vertex_element_classes
            .entry(element.element_class)
            .or_default() += 1;
        *inventory.vertex_streams.entry(element.stream).or_default() += 1;
    }
}

fn inventory_vertex_buffer(arena: &Rw4Arena, bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(buffer) = VertexBuffer::parse(bytes) else {
        record_presentation_failure(inventory, "vertex buffer parse");
        return;
    };
    inventory.vertex_buffers += 1;
    inventory.vertex_buffer_bytes += u64::from(buffer.buffer_size);
    validate_presentation_link(
        arena,
        buffer.data,
        &[GPU_BUFFER_RESOURCE_TYPE_ID],
        inventory,
        "vertex buffer data link",
    );
}

fn inventory_index_buffer(arena: &Rw4Arena, bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(buffer) = IndexBuffer::parse(bytes) else {
        record_presentation_failure(inventory, "index buffer parse");
        return;
    };
    inventory.index_buffers += 1;
    inventory.indices += u64::from(buffer.index_count);
    inventory.index_buffer_bytes += u64::from(buffer.aligned_buffer_size);
    validate_presentation_link(
        arena,
        buffer.data,
        &[GPU_BUFFER_RESOURCE_TYPE_ID],
        inventory,
        "index buffer data link",
    );
}

fn inventory_mesh_helper(arena: &Rw4Arena, bytes: &[u8], inventory: &mut PresentationInventory) {
    let Ok(helper) = MeshHelper::parse(bytes) else {
        record_presentation_failure(inventory, "mesh helper parse");
        return;
    };
    inventory.mesh_helpers += 1;
    for link in helper.index_buffers {
        validate_presentation_link(
            arena,
            link,
            &[INDEX_BUFFER_TYPE_ID],
            inventory,
            "helper index buffer link",
        );
    }
    for link in helper.vertex_buffers {
        validate_presentation_link(
            arena,
            link,
            &[VERTEX_BUFFER_TYPE_ID],
            inventory,
            "helper vertex buffer link",
        );
    }
}

fn inventory_presentation_geometry(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    owner: (&str, u64),
    inventory: &mut PresentationInventory,
) {
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != R_OPTI_MESH_DATA_TYPE_ID {
            continue;
        }
        let Some(bytes) = arena.object_bytes(resources, entry) else {
            record_presentation_failure(inventory, "optimized mesh object range");
            continue;
        };
        let Ok(mesh) = OptimizedRenderMeshData::parse(bytes) else {
            continue;
        };
        let Ok(geometry) = mesh.decode_geometry(arena, resources) else {
            record_presentation_failure(inventory, "native mesh decode");
            continue;
        };
        if !geometry
            .positions
            .iter()
            .flatten()
            .all(|component| component.is_finite())
        {
            record_presentation_failure(inventory, "non-finite position");
            continue;
        }
        inventory.vertices += geometry.positions.len() as u64;
        inventory.decoded_positions += geometry.positions.len() as u64;
        for attribute in &geometry.attributes {
            let samples = non_finite_attribute_samples(&attribute.values);
            if samples.is_empty() {
                continue;
            }
            let key = (attribute.semantic, attribute.original_format);
            let counts = inventory
                .non_finite_vertex_attributes
                .entry(key)
                .or_default();
            counts[0] += 1;
            counts[1] += samples.len();
            inventory
                .non_finite_vertex_samples
                .entry(key)
                .or_default()
                .extend(samples.into_iter().take(8));
            inventory.non_finite_vertex_owners.insert(format!(
                "collection={} asset=0x{:016x} arena=0x{:08x} entry={entry_index} offset=0x{:08x}",
                owner.0, owner.1, arena.id, entry.object_offset,
            ));
        }
        inventory.decoded_vertex_attributes += geometry.attributes.len() as u64;
        inventory.decoded_vertex_attribute_values += geometry
            .attributes
            .iter()
            .map(|attribute| attribute.values.len() as u64)
            .sum::<u64>();
        inventory.decoded_indices += geometry.indices.len() as u64;
        *inventory
            .position_formats
            .entry(geometry.position_format)
            .or_default() += 1;
        for draw in geometry.draws {
            inventory.draw_packets += 1;
            inventory.draw_indices += u64::from(draw.index_count);
            *inventory
                .primitive_types
                .entry(draw.primitive_type)
                .or_default() += 1;
        }
    }
}

fn non_finite_attribute_samples(values: &NativeVertexAttributeValues) -> Vec<String> {
    let mut samples = Vec::new();
    let mut record = |vertex: usize, component: usize, value: f32| {
        if !value.is_finite() {
            samples.push(format!("v{vertex}.c{component}=0x{:08x}", value.to_bits()));
        }
    };
    match values {
        NativeVertexAttributeValues::Float2(values) => {
            for (vertex, value) in values.iter().enumerate() {
                for (component, value) in value.iter().copied().enumerate() {
                    record(vertex, component, value);
                }
            }
        }
        NativeVertexAttributeValues::Float3(values) => {
            for (vertex, value) in values.iter().enumerate() {
                for (component, value) in value.iter().copied().enumerate() {
                    record(vertex, component, value);
                }
            }
        }
        NativeVertexAttributeValues::Float4(values) => {
            for (vertex, value) in values.iter().enumerate() {
                for (component, value) in value.iter().copied().enumerate() {
                    record(vertex, component, value);
                }
            }
        }
        NativeVertexAttributeValues::UnsignedByte4(_)
        | NativeVertexAttributeValues::SignedShort2(_) => {}
    }
    samples
}

fn inventory_material_texture_links(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    texture_asset_ids: &BTreeSet<u64>,
    texture_toc_guids: &BTreeSet<u64>,
    inventory: &mut PresentationInventory,
) {
    let mut toc_guids = BTreeSet::new();
    for entry in &arena.dictionary {
        if entry.type_id != TABLE_OF_CONTENTS_TYPE_ID {
            continue;
        }
        let Some(bytes) = arena.object_bytes(resources, entry) else {
            continue;
        };
        let Ok(toc) = TableOfContents::parse(bytes) else {
            continue;
        };
        toc_guids.extend(toc.entries.into_iter().map(|item| item.guid));
    }
    for entry in &arena.dictionary {
        if entry.type_id != R_MATERIAL_DATA_TYPE_ID {
            continue;
        }
        let Some(bytes) = arena.object_bytes(resources, entry) else {
            continue;
        };
        let Ok(materials) = RenderMaterialData::parse(bytes) else {
            continue;
        };
        for channel in materials.channels {
            let shader_input = channel.shader_input;
            let image_channel = channel.image_channel;
            let MaterialChannelValue::Raster {
                guid, stream_name, ..
            } = channel.value
            else {
                continue;
            };
            if stream_name.is_empty() {
                inventory.raster_empty_stream_names += 1;
            } else {
                inventory.raster_stream_names.insert(stream_name.clone());
            }
            if guid == 0 {
                inventory
                    .raster_reference_samples
                    .entry(format!("{shader_input}:zero"))
                    .or_insert((guid, stream_name.clone(), image_channel));
                inventory.raster_zero_guids += 1;
                inventory
                    .raster_binding_by_shader_input
                    .entry(shader_input)
                    .or_default()[2] += 1;
                inventory
                    .raster_binding_by_image_channel
                    .entry(image_channel)
                    .or_default()[2] += 1;
                continue;
            }
            if toc_guids.contains(&guid) {
                inventory.raster_local_toc_guid_matches += 1;
            } else {
                inventory.raster_local_toc_guid_misses += 1;
            }
            if texture_asset_ids.contains(&guid) {
                inventory.raster_texture_asset_guid_matches += 1;
            } else {
                inventory.raster_texture_asset_guid_misses += 1;
            }
            if texture_toc_guids.contains(&guid) {
                inventory
                    .raster_reference_samples
                    .entry(format!("{shader_input}:texture-toc"))
                    .or_insert((guid, stream_name.clone(), image_channel));
                inventory.raster_texture_toc_guid_matches += 1;
                inventory
                    .raster_binding_by_shader_input
                    .entry(shader_input)
                    .or_default()[0] += 1;
                inventory
                    .raster_binding_by_image_channel
                    .entry(image_channel)
                    .or_default()[0] += 1;
            } else {
                inventory
                    .raster_reference_samples
                    .entry(format!("{shader_input}:miss"))
                    .or_insert((guid, stream_name.clone(), image_channel));
                inventory.raster_texture_toc_guid_misses += 1;
                inventory
                    .raster_binding_by_shader_input
                    .entry(shader_input)
                    .or_default()[1] += 1;
                inventory
                    .raster_binding_by_image_channel
                    .entry(image_channel)
                    .or_default()[1] += 1;
            }
        }
    }
}

fn inventory_mesh_material_links(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    inventory: &mut PresentationInventory,
) {
    let Ok(sections) = arena.sections(resources) else {
        return;
    };
    let mut references = Vec::new();
    for entry in &arena.dictionary {
        let Some(bytes) = arena.object_bytes(resources, entry) else {
            continue;
        };
        match entry.type_id {
            R_MODEL_DATA_TYPE_ID => {
                if let Ok(model) = RenderModelData::parse(bytes) {
                    references.extend(model.meshes.into_iter().filter_map(|mesh| mesh.material));
                }
            }
            R_OPTI_MESH_DATA_TYPE_ID => {
                if let Ok(mesh) = OptimizedRenderMeshData::parse(bytes) {
                    references.extend(mesh.material);
                }
            }
            _ => {}
        }
    }
    for id in references {
        inventory.material_references += 1;
        *inventory
            .material_reference_sections
            .entry(id.section_index())
            .or_default() += 1;
        if resolve_inline_material(arena, &sections, resources, id, inventory) {
            inventory.resolved_material_references += 1;
        } else {
            inventory.unresolved_material_references += 1;
        }
    }
}

fn resolve_inline_material(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    resources: &[Vec<u8>; 5],
    id: ObjectId,
    inventory: &mut PresentationInventory,
) -> bool {
    let Some(subreference) = (id.section_index() == 2)
        .then(|| usize::try_from(id.entry_index()).ok())
        .flatten()
        .and_then(|index| sections.subreferences.get(index))
    else {
        return false;
    };
    *inventory
        .material_subreference_types
        .entry(subreference.type_id)
        .or_default() += 1;
    let Some(base) = arena.local_entry(subreference.object) else {
        return false;
    };
    *inventory
        .material_subreference_base_types
        .entry(base.type_id)
        .or_default() += 1;
    if base.type_id != R_MATERIAL_DATA_TYPE_ID {
        return false;
    }
    arena
        .object_bytes(resources, base)
        .and_then(|bytes| RenderMaterialData::parse(bytes).ok())
        .and_then(|data| data.material_at_object_offset(subreference.object_offset))
        .is_some()
}

fn collect_texture_stream_toc_guids(
    mounts: &MountSet,
    collection_map_path: &str,
    collection_directory: &str,
    inventory: &mut PresentationInventory,
) -> Result<BTreeSet<u64>, Box<dyn std::error::Error>> {
    let collections = CollectionMap::parse(&mounts.read_path(collection_map_path)?)?;
    let mut guids = BTreeSet::new();
    for collection in &collections.collections {
        inventory.texture_stream_collections += 1;
        let path = format!("{collection_directory}\\{}.xsf", collection.filename);
        let bytes = mounts.read_path(&path)?;
        let file = StreamFile::parse(&bytes, collection.asset_count)?;
        for block in &file.assets {
            let decoded = decode_arena(&bytes[block.data_range.clone()])?;
            let arena = Rw4Arena::parse(&decoded.resources)?;
            inventory.texture_stream_arenas += 1;
            inventory.texture_stream_objects += arena.dictionary.len();
            for entry in &arena.dictionary {
                *inventory
                    .texture_stream_object_types
                    .entry(entry.type_id)
                    .or_default() += 1;
                if entry.type_id == TEXTURE_TYPE_ID {
                    inventory_texture_stream_level(&arena, &decoded.resources, entry, inventory);
                }
                if entry.type_id != TABLE_OF_CONTENTS_TYPE_ID {
                    continue;
                }
                let object = arena
                    .object_bytes(&decoded.resources, entry)
                    .ok_or("texture TOC object is outside its resource")?;
                for item in TableOfContents::parse(object)?.entries {
                    inventory.texture_stream_toc_entries += 1;
                    *inventory
                        .texture_toc_object_types
                        .entry(item.object_type)
                        .or_default() += 1;
                    *inventory
                        .texture_toc_link_sections
                        .entry(item.object.section_index())
                        .or_default() += 1;
                    if let Some(target) = arena.local_entry(item.object) {
                        *inventory
                            .texture_toc_target_types
                            .entry(target.type_id)
                            .or_default() += 1;
                    }
                    guids.insert(item.guid);
                }
            }
        }
    }
    Ok(guids)
}

fn inventory_texture_stream_level(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    entry: &skate2_formats::rw4::DictionaryEntry,
    inventory: &mut PresentationInventory,
) {
    let Some(bytes) = arena.object_bytes(resources, entry) else {
        inventory.texture_stream_decode_failures += 1;
        return;
    };
    let Ok(texture) = Texture::parse(bytes) else {
        inventory.texture_stream_decode_failures += 1;
        return;
    };
    *inventory
        .texture_stream_texture_types
        .entry(texture.texture_type)
        .or_default() += 1;
    *inventory
        .texture_stream_endian_modes
        .entry(texture.endian)
        .or_default() += 1;
    *inventory
        .texture_stream_mip_levels
        .entry(texture.mip_levels)
        .or_default() += 1;
    *inventory
        .texture_stream_packed_mips
        .entry(texture.packed_mips)
        .or_default() += 1;
    *inventory
        .texture_stream_mip_data_offsets
        .entry(texture.mip_data_offset)
        .or_default() += 1;
    *inventory
        .texture_stream_texture_formats
        .entry(texture.pixel_format)
        .or_default() += 1;
    let Some(payload) = arena
        .local_entry(texture.data)
        .and_then(|target| arena.object_bytes(resources, target))
    else {
        inventory.texture_stream_decode_failures += 1;
        return;
    };
    let mip_offset = usize::try_from(texture.mip_data_offset).ok();
    if texture.mip_levels > 1 && mip_offset == Some(0) {
        inventory.texture_stream_zero_offset_mip_tails += 1;
    }
    if let Some(expected) = expected_texture_payload_size(texture) {
        if expected == payload.len() {
            inventory.texture_stream_expected_payload_matches += 1;
        } else if let (Ok(actual), Ok(expected)) =
            (i64::try_from(payload.len()), i64::try_from(expected))
        {
            *inventory
                .texture_stream_expected_payload_deltas
                .entry(actual - expected)
                .or_default() += 1;
        }
    }
    match texture.decode_levels(payload) {
        Ok(levels) => {
            inventory.texture_stream_decoded_levels += levels.len();
            inventory.texture_stream_decoded_bytes += levels
                .iter()
                .map(|level| level.data.len() as u64)
                .sum::<u64>();
        }
        Err(_) => inventory.texture_stream_decode_failures += 1,
    }
}

fn expected_texture_payload_size(texture: Texture) -> Option<usize> {
    let bytes_per_block = match texture.pixel_format {
        skate2_formats::material::PIXEL_FORMAT_A8R8G8B8 => 4_u32,
        skate2_formats::material::PIXEL_FORMAT_DXT1 => 8,
        skate2_formats::material::PIXEL_FORMAT_DXT5 => 16,
        skate2_formats::material::PIXEL_FORMAT_R5G6B5 => 2,
        _ => return None,
    };
    let block_extent = if matches!(
        texture.pixel_format,
        skate2_formats::material::PIXEL_FORMAT_DXT1 | skate2_formats::material::PIXEL_FORMAT_DXT5
    ) {
        4
    } else {
        1
    };
    let faces = match texture.texture_type {
        TextureType::TwoDimensional => 1_usize,
        TextureType::Cube => 6,
        _ => return None,
    };
    let tail_level = texture
        .width
        .ilog2()
        .min(texture.height.ilog2())
        .saturating_sub(4)
        .min(texture.mip_levels - 1);
    let first_independent_level = u32::from(tail_level != 0);
    let independent_size =
        (first_independent_level..tail_level).try_fold(0_usize, |total, level| {
            let width = (texture.width >> level).max(1).div_ceil(block_extent);
            let height = (texture.height >> level).max(1).div_ceil(block_extent);
            let aligned_width = width.checked_add(31)? & !31;
            let aligned_height = height.checked_add(31)? & !31;
            let surface = aligned_width
                .checked_mul(aligned_height)?
                .checked_mul(bytes_per_block)?
                .checked_add(4095)?
                & !4095;
            total.checked_add(usize::try_from(surface).ok()?.checked_mul(faces)?)
        })?;
    let tail_width = (texture.width >> tail_level).max(1).div_ceil(block_extent);
    let tail_height = (texture.height >> tail_level).max(1).div_ceil(block_extent);
    let tail_surface = (tail_width.checked_add(31)? & !31)
        .checked_mul(tail_height.checked_add(31)? & !31)?
        .checked_mul(bytes_per_block)?
        .checked_add(4095)?
        & !4095;
    usize::try_from(texture.mip_data_offset)
        .ok()?
        .checked_add(independent_size)?
        .checked_add(usize::try_from(tail_surface).ok()?.checked_mul(faces)?)
}

fn validate_presentation_link(
    arena: &Rw4Arena,
    id: ObjectId,
    expected_types: &[u32],
    inventory: &mut PresentationInventory,
    failure: &'static str,
) {
    if id.section_index() != 0 {
        inventory.external_links += 1;
        return;
    }
    match arena.local_entry(id) {
        Some(entry) if expected_types.contains(&entry.type_id) => inventory.local_links += 1,
        _ => record_presentation_failure(inventory, failure),
    }
}

fn validate_optional_presentation_link(
    arena: &Rw4Arena,
    id: Option<ObjectId>,
    expected_type: u32,
    inventory: &mut PresentationInventory,
    failure: &'static str,
) {
    if let Some(id) = id {
        validate_presentation_link(arena, id, &[expected_type], inventory, failure);
    }
}

fn record_presentation_failure(inventory: &mut PresentationInventory, failure: &'static str) {
    *inventory.failures.entry(failure).or_default() += 1;
}

#[derive(Default)]
struct SimulationInventory {
    collections: usize,
    xsf_bytes: u64,
    asset_blocks: usize,
    stream_assets_by_id: BTreeMap<u64, BTreeSet<String>>,
    rw4_arenas: usize,
    rw4_objects: usize,
    volumes: usize,
    mapped_arrays: usize,
    mapped_volumes: usize,
    primitive_probe_hits: usize,
    primitive_probe_misses: usize,
    primitive_fat_probe_hits: usize,
    primitive_fat_probe_misses: usize,
    invalid_primitive_parameters: usize,
    meshes: usize,
    clusters: usize,
    kd_branches: u64,
    kd_entries: u64,
    vertices: usize,
    units: usize,
    triangles: usize,
    instances: usize,
    instance_guids: BTreeSet<u64>,
    instance_local_guids: BTreeSet<u64>,
    instance_data_types: BTreeMap<u32, usize>,
    instance_guids_by_data_type: BTreeMap<u32, BTreeSet<u64>>,
    instance_subreferences: usize,
    splines: usize,
    spline_segments: usize,
    vertical_grind_primitives: usize,
    grind_candidate_probe_hits: usize,
    grind_candidate_probe_misses: usize,
    grind_candidate_miss_samples: Vec<GrindPrimitive>,
    spline_subreferences: usize,
    spline_instance_links: usize,
    embedded_data: usize,
    embedded_grind_splines: usize,
    embedded_grab_splines: usize,
    dmo_data: usize,
    dmos: usize,
    dmo_nonzero_serialized_instance_words: usize,
    dmo_local_guids: BTreeSet<u64>,
    dmo_instance_guids: BTreeSet<u64>,
    dmo_model_local_guids: BTreeSet<u64>,
    dmo_model_names: BTreeSet<String>,
    dmo_model_names_by_id: BTreeMap<u64, BTreeSet<String>>,
    collision_models: usize,
    location_descriptors: usize,
    sub_location_descriptors: usize,
    location_subreferences: usize,
    trigger_instances: usize,
    trigger_subreferences: usize,
    toc_entries: usize,
    toc_entries_by_guid: BTreeMap<u64, BTreeSet<String>>,
    local_object_links: usize,
    subreference_links: usize,
    null_arena_links: usize,
    external_arena_links: usize,
    resolved_external_links: usize,
    unavailable_external_links: usize,
    failures: BTreeMap<&'static str, usize>,
    object_types: BTreeMap<u32, usize>,
    compression_modes: BTreeMap<&'static str, usize>,
    mapped_volume_types: BTreeMap<u32, usize>,
    toc_types: BTreeMap<u32, usize>,
    object_link_sections: BTreeMap<u16, usize>,
    arena_dictionaries: BTreeMap<u32, Vec<u32>>,
    pending_external_links: Vec<PendingExternalLink>,
    location_plan: LocationPlanInventory,
    location_plan_edges: LocationPlanEdges,
    location_runtime: LocationRuntimeInventory,
    pointer_plan: SimulationPointerPlanInventory,
    registry_runtime: SimulationRegistryRuntimeInventory,
    dmo_plan: SimulationDmoPlanInventory,
    dmo_plan_edges: SimulationDmoPlanEdges,
    dmo_runtime: SimulationDmoRuntimeInventory,
    pointer_raw_containers: BTreeMap<(u64, u32, u32), usize>,
    pointer_processed_containers: BTreeMap<(u64, u32, u32), usize>,
}

const SIMULATION_POINTER_FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const SIMULATION_POINTER_FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LocationPlanInventory {
    collections: usize,
    arenas: usize,
    containers: usize,
    arenas_with_container: usize,
    zero_location_containers: usize,
    locations: usize,
    sublocations: usize,
    string_count: usize,
    signature: u64,
}

impl Default for LocationPlanInventory {
    fn default() -> Self {
        let mut signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(&mut signature, b"SKATE2/BAM_SIM/LOCATION_KIND21_PLAN\0V1\0");
        Self {
            collections: 0,
            arenas: 0,
            containers: 0,
            arenas_with_container: 0,
            zero_location_containers: 0,
            locations: 0,
            sublocations: 0,
            string_count: 0,
            signature,
        }
    }
}

const EXPECTED_LOCATION_PLAN: LocationPlanInventory = LocationPlanInventory {
    collections: 284,
    arenas: 2_135,
    containers: 107,
    arenas_with_container: 107,
    zero_location_containers: 83,
    locations: 53,
    sublocations: 0,
    string_count: 0,
    signature: 0x024d_e466_0f5f_c535,
};

#[derive(Debug, Default)]
struct LocationPlanEdges {
    container_ids: BTreeSet<u64>,
    location_ids: BTreeSet<u64>,
    location_names: BTreeSet<String>,
    location_hash_names: BTreeMap<u32, String>,
    source_spans: BTreeSet<(u64, u32, u64, u64)>,
    source_resources: BTreeSet<u32>,
    locations_per_container: BTreeMap<usize, usize>,
    source_span_aliases: usize,
    zero_container_ids: usize,
    zero_location_ids: usize,
    duplicate_container_ids: usize,
    duplicate_location_ids: usize,
    duplicate_location_names: usize,
    location_hash_collisions: usize,
    named_locations: usize,
    described_locations: usize,
    named_sublocations: usize,
    described_sublocations: usize,
    maximum_containers_per_arena: usize,
    maximum_locations_per_container: usize,
    maximum_sublocations_per_container: usize,
    minimum_source_span: Option<usize>,
    maximum_source_span: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LocationRuntimeInventory {
    collections: usize,
    arenas: usize,
    containers: usize,
    empty_containers: usize,
    descriptors: usize,
    priority_samples: usize,
    high_priority: usize,
    low_priority: usize,
    registration_tokens: usize,
    removals: usize,
    deregistration_priority_samples: usize,
    source_matches: usize,
    source_mismatches: usize,
    reg_signature: u64,
    dreg_signature: u64,
}

impl Default for LocationRuntimeInventory {
    fn default() -> Self {
        let mut reg_signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(
            &mut reg_signature,
            b"SKATE2/BAM_SIM/LOCATION_KIND21_RUNTIME_REG\0V1\0",
        );
        simulation_pointer_fnv_bytes(&mut reg_signature, b"S");
        simulation_pointer_fnv_u32(&mut reg_signature, 0x4c4f_4331);
        let mut dreg_signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(
            &mut dreg_signature,
            b"SKATE2/BAM_SIM/LOCATION_KIND21_RUNTIME_DREG\0V1\0",
        );
        simulation_pointer_fnv_bytes(&mut dreg_signature, b"S");
        Self {
            collections: 0,
            arenas: 0,
            containers: 0,
            empty_containers: 0,
            descriptors: 0,
            priority_samples: 0,
            high_priority: 0,
            low_priority: 0,
            registration_tokens: 0,
            removals: 0,
            deregistration_priority_samples: 0,
            source_matches: 0,
            source_mismatches: 0,
            reg_signature,
            dreg_signature,
        }
    }
}

const EXPECTED_LOCATION_RUNTIME: LocationRuntimeInventory = LocationRuntimeInventory {
    collections: 284,
    arenas: 2_135,
    containers: 107,
    empty_containers: 83,
    descriptors: 53,
    priority_samples: 53,
    high_priority: 27,
    low_priority: 26,
    registration_tokens: 2_135,
    removals: 53,
    deregistration_priority_samples: 0,
    source_matches: 53,
    source_mismatches: 0,
    reg_signature: 0x2c83_b407_a844_d816,
    dreg_signature: 0x8b32_aa3e_9943_123b,
};

#[derive(Debug, Default)]
struct LocationCorpusRuntimeSource {
    registry: BTreeMap<u64, skate2_core::location_arena_plan::LocationDataHandle>,
    resolutions: Vec<u64>,
    priority_samples: usize,
}

impl LocationArenaRuntimeSource for LocationCorpusRuntimeSource {
    fn resolve_location_data(
        &mut self,
        asset_id: u64,
    ) -> Option<skate2_core::location_arena_plan::LocationDataHandle> {
        self.resolutions.push(asset_id);
        self.registry.get(&asset_id).cloned()
    }

    fn high_priority_raw(&mut self) -> u8 {
        let high = self.priority_samples % 2 == 0;
        self.priority_samples += 1;
        u8::from(high)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationDmoPlanInventory {
    collections: usize,
    arenas: usize,
    collections_with_kind31: usize,
    arenas_with_kind31: usize,
    containers: usize,
    records: usize,
    signature: u64,
}

impl Default for SimulationDmoPlanInventory {
    fn default() -> Self {
        let mut signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(&mut signature, b"SKATE2/BAM_SIM/KIND31\0V1\0");
        Self {
            collections: 0,
            arenas: 0,
            collections_with_kind31: 0,
            arenas_with_kind31: 0,
            containers: 0,
            records: 0,
            signature,
        }
    }
}

const EXPECTED_SIMULATION_DMO_PLAN: SimulationDmoPlanInventory = SimulationDmoPlanInventory {
    collections: 284,
    arenas: 2_135,
    collections_with_kind31: 99,
    arenas_with_kind31: 815,
    containers: 815,
    records: 3_591,
    signature: 0x1c41_b205_7433_ff8a,
};

#[derive(Debug, Default)]
struct SimulationDmoPlanEdges {
    collection_ordinals: BTreeSet<u32>,
    container_ids: BTreeSet<u64>,
    record_ids: BTreeSet<u64>,
    local_id_counts: BTreeMap<u64, usize>,
    instance_id_counts: BTreeMap<u64, usize>,
    identity_triples: BTreeSet<(u64, u64, u64)>,
    source_spans: BTreeSet<(u64, u32, u64, u64)>,
    source_span_aliases: usize,
    one_record_containers: usize,
    minimum_record_count: Option<usize>,
    maximum_record_count: Option<usize>,
    minimum_span: Option<usize>,
    maximum_span: Option<usize>,
    zero_guid: usize,
    zero_local_guid: usize,
    zero_instance_guid: usize,
    nonzero_serialized_words: usize,
    empty_names: usize,
    header_types: BTreeMap<u32, usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationDmoRuntimeInventory {
    operations: usize,
    policy_samples: usize,
    reg_living: usize,
    reg_challenge: usize,
    reg_registry: usize,
    dreg_living: usize,
    dreg_challenge: usize,
    dreg_registry: usize,
    prepared_source_matches: usize,
    dreg_source_matches: usize,
    source_mismatches: usize,
    reg_signature: u64,
    dreg_signature: u64,
}

impl Default for SimulationDmoRuntimeInventory {
    fn default() -> Self {
        let mut reg_signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(
            &mut reg_signature,
            b"SKATE2/BAM_SIM/KIND31_RUNTIME_REG\0V1\0",
        );
        simulation_pointer_fnv_bytes(&mut reg_signature, b"S");
        simulation_pointer_fnv_u32(&mut reg_signature, 7);
        simulation_pointer_fnv_u32(&mut reg_signature, 2);
        let mut dreg_signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(
            &mut dreg_signature,
            b"SKATE2/BAM_SIM/KIND31_RUNTIME_DREG\0V1\0",
        );
        simulation_pointer_fnv_bytes(&mut dreg_signature, b"S");
        simulation_pointer_fnv_u32(&mut dreg_signature, 7);
        Self {
            operations: 0,
            policy_samples: 0,
            reg_living: 0,
            reg_challenge: 0,
            reg_registry: 0,
            dreg_living: 0,
            dreg_challenge: 0,
            dreg_registry: 0,
            prepared_source_matches: 0,
            dreg_source_matches: 0,
            source_mismatches: 0,
            reg_signature,
            dreg_signature,
        }
    }
}

const EXPECTED_SIMULATION_DMO_RUNTIME: SimulationDmoRuntimeInventory =
    SimulationDmoRuntimeInventory {
        operations: 815,
        policy_samples: 815,
        reg_living: 407,
        reg_challenge: 408,
        reg_registry: 408,
        dreg_living: 0,
        dreg_challenge: 815,
        dreg_registry: 815,
        prepared_source_matches: 815,
        dreg_source_matches: 815,
        source_mismatches: 0,
        reg_signature: 0x16f4_eda4_f913_2804,
        dreg_signature: 0x70d8_1a75_f975_2756,
    };

#[derive(Clone, Debug)]
struct SimulationDmoRuntimeMetadata {
    global_index: u32,
    asset_id: u64,
    arena_asset_id: u64,
    resource_index: u32,
    source_span_start: u64,
    source_span_end: u64,
    source: SimulationDmoDataHandle,
}

#[derive(Debug, Default)]
struct SimulationDmoRuntimeState {
    inventory: SimulationDmoRuntimeInventory,
    current: Vec<SimulationDmoRuntimeMetadata>,
    registration_index: usize,
    deregistration_index: usize,
}

#[derive(Clone, Debug)]
struct SimulationDmoRuntimePolicy {
    state: Arc<Mutex<SimulationDmoRuntimeState>>,
}

#[derive(Clone, Debug)]
struct SimulationDmoRuntimeDispatch {
    state: Arc<Mutex<SimulationDmoRuntimeState>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationPointerPlanInventory {
    collections: usize,
    arenas: usize,
    toc_entries: usize,
    dmo_instances: usize,
    trigger_instances: usize,
    raw_dmo_containers: usize,
    raw_dmo_instances: usize,
    raw_trigger_containers: usize,
    raw_trigger_instances: usize,
    scope_signature: u64,
    operation_signature: u64,
    unified_signature: u64,
}

impl Default for SimulationPointerPlanInventory {
    fn default() -> Self {
        Self {
            collections: 0,
            arenas: 0,
            toc_entries: 0,
            dmo_instances: 0,
            trigger_instances: 0,
            raw_dmo_containers: 0,
            raw_dmo_instances: 0,
            raw_trigger_containers: 0,
            raw_trigger_instances: 0,
            scope_signature: SIMULATION_POINTER_FNV_OFFSET,
            operation_signature: SIMULATION_POINTER_FNV_OFFSET,
            unified_signature: SIMULATION_POINTER_FNV_OFFSET,
        }
    }
}

const EXPECTED_SIMULATION_POINTER_PLAN: SimulationPointerPlanInventory =
    SimulationPointerPlanInventory {
        collections: 284,
        arenas: 2_135,
        toc_entries: 62_810,
        dmo_instances: 0,
        trigger_instances: 10,
        raw_dmo_containers: 0,
        raw_dmo_instances: 0,
        raw_trigger_containers: 9,
        raw_trigger_instances: 10,
        scope_signature: 0xd53a_b862_08c6_d53a,
        operation_signature: 0x8cd3_0815_7ff7_0b30,
        unified_signature: 0xf9ea_7069_6c82_a99b,
    };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationRegistryRuntimeInventory {
    collections: usize,
    arenas: usize,
    operations: usize,
    world_painter: usize,
    nav_mesh: usize,
    rain: usize,
    ai_path: usize,
    waypoint: usize,
    spatial_map: usize,
    registrations: usize,
    deregistrations: usize,
    waypoint_polarity_mismatches: usize,
    sequence_mismatches: usize,
    reg_signature: u64,
    dreg_signature: u64,
}

impl Default for SimulationRegistryRuntimeInventory {
    fn default() -> Self {
        let mut reg_signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(
            &mut reg_signature,
            b"SKATE2/BAM_SIM/SIMULATION_ID_ONLY_RUNTIME_REG\0V1\0",
        );
        let mut dreg_signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(
            &mut dreg_signature,
            b"SKATE2/BAM_SIM/SIMULATION_ID_ONLY_RUNTIME_DREG\0V1\0",
        );
        Self {
            collections: 0,
            arenas: 0,
            operations: 0,
            world_painter: 0,
            nav_mesh: 0,
            rain: 0,
            ai_path: 0,
            waypoint: 0,
            spatial_map: 0,
            registrations: 0,
            deregistrations: 0,
            waypoint_polarity_mismatches: 0,
            sequence_mismatches: 0,
            reg_signature,
            dreg_signature,
        }
    }
}

const EXPECTED_SIMULATION_REGISTRY_RUNTIME: SimulationRegistryRuntimeInventory =
    SimulationRegistryRuntimeInventory {
        collections: 284,
        arenas: 2_135,
        operations: 7_426,
        world_painter: 6_122,
        nav_mesh: 0,
        rain: 75,
        ai_path: 101,
        waypoint: 95,
        spatial_map: 1_033,
        registrations: 7_426,
        deregistrations: 7_426,
        waypoint_polarity_mismatches: 0,
        sequence_mismatches: 0,
        reg_signature: 0x757b_00e4_7ce5_f273,
        dreg_signature: 0xb55a_e554_79c6_7d26,
    };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationRegistryCorpusEvent {
    registration: bool,
    kind: SimulationRegistryAssetKind,
    asset_id: u64,
    waypoint_streamed: Option<bool>,
}

#[derive(Debug, Default)]
struct SimulationRegistryCorpusDispatch {
    events: Vec<SimulationRegistryCorpusEvent>,
}

impl SimulationRegistryCorpusDispatch {
    fn push(
        &mut self,
        registration: bool,
        kind: SimulationRegistryAssetKind,
        asset_id: u64,
        waypoint_streamed: Option<bool>,
    ) {
        self.events.push(SimulationRegistryCorpusEvent {
            registration,
            kind,
            asset_id,
            waypoint_streamed,
        });
    }
}

impl SimulationRegistryArenaDispatch for SimulationRegistryCorpusDispatch {
    fn register_streamed_tile(&mut self, asset_id: u64) {
        self.push(
            true,
            SimulationRegistryAssetKind::WorldPainter,
            asset_id,
            None,
        );
    }

    fn unregister_streamed_tile(&mut self, asset_id: u64) {
        self.push(
            false,
            SimulationRegistryAssetKind::WorldPainter,
            asset_id,
            None,
        );
    }

    fn add_nav_mesh_data(&mut self, asset_id: u64) {
        self.push(true, SimulationRegistryAssetKind::NavMesh, asset_id, None);
    }

    fn remove_nav_mesh_data(&mut self, asset_id: u64) {
        self.push(false, SimulationRegistryAssetKind::NavMesh, asset_id, None);
    }

    fn add_rain_data(&mut self, asset_id: u64) {
        self.push(true, SimulationRegistryAssetKind::Rain, asset_id, None);
    }

    fn remove_rain_data(&mut self, asset_id: u64) {
        self.push(false, SimulationRegistryAssetKind::Rain, asset_id, None);
    }

    fn add_ai_path_asset(&mut self, asset_id: u64) {
        self.push(true, SimulationRegistryAssetKind::AiPath, asset_id, None);
    }

    fn remove_ai_path_asset(&mut self, asset_id: u64) {
        self.push(false, SimulationRegistryAssetKind::AiPath, asset_id, None);
    }

    fn register_waypoint_data(&mut self, asset_id: u64, streamed: bool) {
        self.push(
            true,
            SimulationRegistryAssetKind::Waypoint,
            asset_id,
            Some(streamed),
        );
    }

    fn deregister_waypoint_data(&mut self, asset_id: u64, streamed: bool) {
        self.push(
            false,
            SimulationRegistryAssetKind::Waypoint,
            asset_id,
            Some(streamed),
        );
    }

    fn add_spatial_map_asset(&mut self, asset_id: u64) {
        self.push(
            true,
            SimulationRegistryAssetKind::SpatialMap,
            asset_id,
            None,
        );
    }

    fn remove_spatial_map_asset(&mut self, asset_id: u64) {
        self.push(
            false,
            SimulationRegistryAssetKind::SpatialMap,
            asset_id,
            None,
        );
    }
}

const EXPECTED_DMO_POINTER_PLAN: SimulationPointerPlanInventory = SimulationPointerPlanInventory {
    collections: 1,
    arenas: 125,
    toc_entries: 287,
    dmo_instances: 93,
    trigger_instances: 0,
    raw_dmo_containers: 1,
    raw_dmo_instances: 93,
    raw_trigger_containers: 0,
    raw_trigger_instances: 0,
    scope_signature: 0x3142_cd8e_7499_1ba3,
    operation_signature: 0xc769_707c_fc80_f14f,
    unified_signature: 0x4151_adc6_8002_1965,
};

#[derive(Debug, Default)]
struct LivingWorldModelInventory {
    model_toc_assets: usize,
    dmo_model_toc_matches: BTreeSet<u64>,
    rx2_files: usize,
    hashed_model_entries: usize,
    non_arena_hashed_entries: usize,
    xml_recipe_companions: usize,
    character_recipes: usize,
    recipe_models: usize,
    recipe_lods: usize,
    recipe_asset_requests: usize,
    recipe_unique_asset_paths: usize,
    resolved_recipe_asset_paths: usize,
    recipe_dmo_id_matches: BTreeSet<u64>,
    recipe_schemas: BTreeMap<String, usize>,
    arenas: usize,
    instances: usize,
    instances_with_embedded_data: usize,
    dmo_model_id_matches: usize,
    dmo_model_path_matches: BTreeSet<String>,
    dmo_any_path_matches: BTreeSet<String>,
    dmo_model_name_path_matches: BTreeSet<String>,
    embedded_records: usize,
    grind_spline_references: usize,
    resolved_grind_splines: usize,
    native_grind_assets: usize,
    native_grind_primitives: usize,
    grind_splines_per_body: BTreeMap<usize, usize>,
    grind_reference_sections: BTreeMap<u16, usize>,
    grind_subreference_types: BTreeMap<u32, usize>,
    grind_base_types: BTreeMap<u32, usize>,
    grind_model_schemas: BTreeSet<String>,
    failures: BTreeMap<&'static str, usize>,
}

#[derive(Debug, Default)]
struct DmoModelArenaInventory {
    entries: usize,
    entries_by_archive: BTreeMap<String, usize>,
    standalone_arenas: usize,
    compressed_arenas: usize,
    instances: usize,
    dmo_instances: usize,
    instance_data_types: BTreeMap<u32, usize>,
    object_types: BTreeMap<u32, usize>,
    dmo_models: BTreeMap<u64, String>,
    dmo_model_bounds: BTreeMap<u64, ([f32; 3], [f32; 3])>,
    dmo_attribute_pairs: BTreeMap<u64, (u64, u64)>,
    dmo_priority_indices: BTreeMap<u64, i32>,
    dmo_grind_assets: BTreeMap<u64, Vec<GrindAsset>>,
    dmo_collision_parts: BTreeMap<u64, Vec<DynamicCollisionPart>>,
    dmo_clustered_mesh_templates: BTreeMap<u64, Arc<DynamicClusteredMeshTemplate>>,
    clustered_mesh_template_parts: usize,
    clustered_mesh_template_leaves: usize,
    collision_template_parts: usize,
    collision_template_primitives: usize,
    collision_template_volume_types: BTreeMap<u32, usize>,
    collision_template_triangles: usize,
    local_collision_models: usize,
    local_collision_geometry_objects: usize,
    collision_model_references: usize,
    collision_model_reference_sections: BTreeMap<u16, usize>,
    collision_model_external_arenas: BTreeSet<u32>,
    collision_ownership_errors: BTreeSet<String>,
    assembly_target_types: BTreeMap<u32, usize>,
    assembly_definitions: usize,
    assembly_part_definitions: usize,
    assembly_part_payload_signature: u64,
    instances_with_embedded_data: usize,
    embedded_records: usize,
    grind_spline_references: usize,
    resolved_grind_splines: usize,
    native_grind_primitives: usize,
    grind_splines_per_body: BTreeMap<usize, usize>,
    grind_reference_sections: BTreeMap<u16, usize>,
    grind_subreference_types: BTreeMap<u32, usize>,
    grind_base_types: BTreeMap<u32, usize>,
    failures: BTreeMap<&'static str, usize>,
    read_failures: usize,
}

#[derive(Debug, Default)]
struct RuntimeDmoGrindInventory {
    resident_locators: usize,
    resolved_model_locators: usize,
    census_spawn_radius: f32,
    census_candidate_locators: usize,
    resident_grind_bodies: usize,
    resident_grind_assets: usize,
    resident_grind_primitives: usize,
    resident_collision_bodies: usize,
    resident_collision_parts: usize,
    resident_collision_primitives: usize,
    resident_clustered_mesh_templates: usize,
    resident_clustered_mesh_template_parts: usize,
    resident_clustered_mesh_template_leaves: usize,
    missing_model_ids: BTreeSet<u64>,
}

#[derive(Clone, Debug)]
struct RuntimeDmoLocator {
    locator_id: u64,
    model_id: u64,
    priority_index: i32,
    transform: [f32; 16],
    model_bounds: ([f32; 3], [f32; 3]),
    grind_assets: Vec<GrindAsset>,
    collision_parts: Vec<DynamicCollisionPart>,
    /// Structural model ownership only. Live `Part::GetTransform` placement is
    /// not derivable from the locator matrix and is intentionally absent.
    collision_template: Option<Arc<DynamicClusteredMeshTemplate>>,
}

impl RuntimeDmoLocator {
    #[allow(dead_code)]
    fn retained_collision_template(&self) -> Option<&Arc<DynamicClusteredMeshTemplate>> {
        self.collision_template.as_ref()
    }

    fn collision_body(&self) -> DynamicCollisionBody {
        DynamicCollisionBody {
            model_id: self.model_id,
            transform: self.transform,
            bounding_box_min: self.model_bounds.0,
            bounding_box_max: self.model_bounds.1,
            parts: self.collision_parts.clone(),
        }
    }
}

#[derive(Debug)]
struct RuntimeDmoCatalog {
    census: LivingWorldCensusCircle,
    priority_weights: DynamicObjectPriorityWeights,
    locators: Vec<RuntimeDmoLocator>,
}

#[allow(dead_code)]
struct RuntimeDmoAdmissionSnapshot<'a> {
    priority_context: DynamicObjectPriorityContext,
    safety_index: i32,
    is_static: bool,
    lowest_pool_object: Option<DynamicObjectPriorityIndex>,
    live_dynamic_objects: &'a [PrioritizedDynamicCollisionBody],
    live_vehicles: &'a [OwnedDynamicCollisionBody],
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
struct RuntimeDmoEntity {
    allocation: DynamicObjectEntityAllocation,
    locator_id: u64,
    collision: PrioritizedDynamicCollisionBody,
    lifetime: LivingWorldEntityLifetime,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeDmoSpawnError {
    Admission(DynamicObjectAdmissionRejection),
    EntityStoreFull,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
struct RuntimeDmoCensus {
    allocator: DynamicObjectEntityAllocator,
    entities: Vec<RuntimeDmoEntity>,
    pending_culls: Vec<DynamicObjectEntityAllocation>,
}

#[allow(dead_code)]
impl RuntimeDmoCensus {
    fn live_collision_snapshot(&self) -> Vec<PrioritizedDynamicCollisionBody> {
        self.entities
            .iter()
            .filter(|entity| entity.lifetime == LivingWorldEntityLifetime::Active)
            .map(|entity| entity.collision.clone())
            .collect()
    }

    fn request_cull(&mut self, object_id: u32) -> bool {
        let Some(entity) = self.entities.iter_mut().find(|entity| {
            entity.allocation.object_id == object_id
                && entity.lifetime == LivingWorldEntityLifetime::Active
        }) else {
            return false;
        };
        if !entity.lifetime.request_cull() {
            return false;
        }
        self.pending_culls.push(entity.allocation);
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn admit_and_spawn(
        &mut self,
        catalog: &RuntimeDmoCatalog,
        locator: &RuntimeDmoLocator,
        priority_context: DynamicObjectPriorityContext,
        safety_index: i32,
        is_static: bool,
        lowest_pool_object: Option<DynamicObjectPriorityIndex>,
        live_vehicles: &[OwnedDynamicCollisionBody],
        entity_ids: &mut LivingWorldEntityIdAllocator,
        mut request_vehicle_cull: impl FnMut(u32),
    ) -> Result<(u32, DynamicObjectAdmissionPlan), RuntimeDmoSpawnError> {
        let live_dynamic_objects = self.live_collision_snapshot();
        let plan = catalog
            .admission_for_snapshot(
                locator,
                &RuntimeDmoAdmissionSnapshot {
                    priority_context,
                    safety_index,
                    is_static,
                    lowest_pool_object,
                    live_dynamic_objects: &live_dynamic_objects,
                    live_vehicles,
                },
            )
            .map_err(RuntimeDmoSpawnError::Admission)?;

        for object_id in &plan.overlapping_dynamic_objects {
            self.request_cull(*object_id);
        }
        for object_id in &plan.overlapping_vehicles {
            request_vehicle_cull(*object_id);
        }
        if let Some(object_id) = plan.lowest_priority_object {
            self.request_cull(object_id);
        }

        let allocation = self
            .allocator
            .allocate(entity_ids)
            .ok_or(RuntimeDmoSpawnError::EntityStoreFull)?;
        let mut priority = dynamic_object_priority_index(
            locator.priority_index,
            safety_index,
            is_static,
            [
                locator.transform[12],
                locator.transform[13],
                locator.transform[14],
            ],
            priority_context,
            catalog.priority_weights,
        );
        priority.object_id = allocation.object_id;
        self.entities.push(RuntimeDmoEntity {
            allocation,
            locator_id: locator.locator_id,
            collision: PrioritizedDynamicCollisionBody {
                priority,
                body: locator.collision_body(),
            },
            lifetime: LivingWorldEntityLifetime::Active,
        });
        Ok((allocation.object_id, plan))
    }

    fn finish_cull_update(&mut self, reference_entity_store_base: u32) -> Option<Vec<u32>> {
        let destruction_order =
            dynamic_object_reference_cull_order(&self.pending_culls, reference_entity_store_base)?;
        for object_id in &destruction_order {
            let entity = self
                .entities
                .iter_mut()
                .find(|entity| entity.allocation.object_id == *object_id)?;
            if !entity.lifetime.finish_cull_update() || !self.allocator.free(entity.allocation) {
                return None;
            }
        }
        self.entities
            .retain(|entity| entity.lifetime != LivingWorldEntityLifetime::Destroyed);
        self.pending_culls.clear();
        Some(destruction_order)
    }
}

impl RuntimeDmoCatalog {
    fn census_candidates(&self, center: [f32; 3], initial_spawn: bool) -> Vec<&RuntimeDmoLocator> {
        let radius_squared = self.census.spawn_radius * self.census.spawn_radius;
        let mut candidates = self
            .locators
            .iter()
            .filter_map(|locator| {
                let delta_x = locator.transform[12] - center[0];
                let delta_z = locator.transform[14] - center[2];
                let distance_squared = delta_x.mul_add(delta_x, delta_z * delta_z);
                let inside_outer_radius = distance_squared <= radius_squared;
                let outside_inner_radius =
                    initial_spawn || distance_squared.sqrt() >= self.census.no_spawn_radius;
                (inside_outer_radius && outside_inner_radius).then_some((distance_squared, locator))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.locator_id.cmp(&right.1.locator_id))
        });
        candidates
            .into_iter()
            .take(256)
            .map(|(_, locator)| locator)
            .collect()
    }

    fn scene_for<'a>(
        locators: impl IntoIterator<Item = &'a RuntimeDmoLocator>,
    ) -> (LivingWorldGrindScene, Vec<DynamicCollisionBody>) {
        let mut scene = LivingWorldGrindScene::default();
        let mut collision_bodies = Vec::new();
        for locator in locators {
            if !locator.grind_assets.is_empty() {
                scene.dynamic_objects.push(LivingWorldGrindBody {
                    transform: locator.transform,
                    grind_assets: locator.grind_assets.clone(),
                });
            }
            if !locator.collision_parts.is_empty() {
                collision_bodies.push(locator.collision_body());
            }
        }
        (scene, collision_bodies)
    }

    #[allow(dead_code)]
    fn admission_for_snapshot(
        &self,
        locator: &RuntimeDmoLocator,
        snapshot: &RuntimeDmoAdmissionSnapshot<'_>,
    ) -> Result<DynamicObjectAdmissionPlan, DynamicObjectAdmissionRejection> {
        let candidate_priority = dynamic_object_priority_index(
            locator.priority_index,
            snapshot.safety_index,
            snapshot.is_static,
            [
                locator.transform[12],
                locator.transform[13],
                locator.transform[14],
            ],
            snapshot.priority_context,
            self.priority_weights,
        );
        plan_dynamic_object_admission_from_world(
            candidate_priority,
            &locator.collision_body(),
            snapshot.lowest_pool_object,
            snapshot.live_dynamic_objects,
            snapshot.live_vehicles,
        )
    }

    fn first_self_colliding_resident(
        &self,
        tuning: &GrindGeometryTuning,
    ) -> Option<(&RuntimeDmoLocator, GrindPrimitive)> {
        self.locators.iter().find_map(|locator| {
            let collision_body = locator.collision_body();
            locator
                .grind_assets
                .iter()
                .flat_map(|asset| &asset.primitives)
                .find_map(|primitive| {
                    let transformed =
                        transform_dynamic_grind_primitive(*primitive, locator.transform);
                    #[allow(clippy::manual_midpoint)]
                    let midpoint = std::array::from_fn(|axis| {
                        (transformed.start_point[axis] + transformed.end_point[axis]) * 0.5
                    });
                    let PreparedGrindGeometry::Query(setup) =
                        prepare_grind_geometry(&transformed, midpoint, tuning)?
                    else {
                        return None;
                    };
                    dynamic_grind_geometry_batch(
                        std::slice::from_ref(&collision_body),
                        &setup.batch,
                    )
                    .into_iter()
                    .any(|collision| collision.hit)
                    .then_some((locator, *primitive))
                })
        })
    }
}

#[cfg(test)]
mod runtime_dmo_catalog_tests {
    use super::*;

    fn catalog_locator(locator_id: u64, position: [f32; 3]) -> RuntimeDmoLocator {
        let mut transform = [0.0; 16];
        transform[0] = 1.0;
        transform[5] = 1.0;
        transform[10] = 1.0;
        transform[15] = 1.0;
        transform[12] = position[0];
        transform[13] = position[1];
        transform[14] = position[2];
        RuntimeDmoLocator {
            locator_id,
            model_id: locator_id,
            priority_index: 1,
            transform,
            model_bounds: ([0.0; 3], [0.0; 3]),
            grind_assets: Vec::new(),
            collision_parts: Vec::new(),
            collision_template: None,
        }
    }

    fn test_catalog(locators: Vec<RuntimeDmoLocator>) -> RuntimeDmoCatalog {
        RuntimeDmoCatalog {
            census: LivingWorldCensusCircle {
                no_spawn_radius: 0.0,
                spawn_radius: 60.0,
                cull_radius: 75.0,
                shift_distance_along_velocity: 0.0,
                speed: 0.0,
            },
            priority_weights: DynamicObjectPriorityWeights {
                camera_plane: 1.0,
                in_use: 1.0,
                priority: 1.0,
                safety: 1.0,
                distance: 1.0,
            },
            locators,
        }
    }

    fn add_unit_sphere(locator: &mut RuntimeDmoLocator) {
        locator.model_bounds = ([-1.0; 3], [1.0; 3]);
        locator.collision_parts = vec![DynamicCollisionPart {
            transform: catalog_locator(0, [0.0; 3]).transform,
            volumes: vec![Volume {
                transform: catalog_locator(0, [0.0; 3]).transform,
                vtable_offset: VolumeType::Sphere as u32,
                shape_words: [0; 3],
                radius: 1.0,
                group_id: 0,
                surface_id: 0,
                flags: 0,
            }],
            triangles: Vec::new(),
        }];
    }

    fn empty_collision_arena() -> CollisionArena {
        CollisionArena {
            id: 1,
            grind_assets: Vec::new(),
            aggregate_roots: Vec::new(),
            primitive_sets: Vec::new(),
            simple_primitive_sets: Vec::new(),
            meshes: Vec::new(),
            collision_models: Vec::new(),
            instances: Vec::new(),
        }
    }

    #[test]
    fn runtime_locators_share_retained_template_ownership_without_changing_flattened_bodies() {
        let arena = Arc::new(empty_collision_arena());
        let template = Arc::new(
            DynamicClusteredMeshTemplate::try_new(Arc::clone(&arena), Vec::new(), false).unwrap(),
        );
        let mut first = catalog_locator(1, [2.0, 3.0, 4.0]);
        let mut second = catalog_locator(2, [5.0, 6.0, 7.0]);
        first.collision_template = Some(Arc::clone(&template));
        second.collision_template = Some(Arc::clone(&template));
        add_unit_sphere(&mut first);

        assert!(Arc::ptr_eq(template.arena_owner(), &arena));
        assert!(Arc::ptr_eq(
            first.retained_collision_template().unwrap(),
            second.retained_collision_template().unwrap()
        ));
        let body = first.collision_body();
        assert_eq!(
            body.transform.map(f32::to_bits),
            first.transform.map(f32::to_bits)
        );
        assert_eq!(body.parts, first.collision_parts);
    }

    #[test]
    fn dmo_template_part_mapping_preserves_order_metadata_and_rejects_bad_provenance() {
        let mut arena = empty_collision_arena();
        let volume = |dictionary_index| CollisionRoot {
            dictionary_index,
            volume: Volume {
                transform: catalog_locator(0, [0.0; 3]).transform,
                vtable_offset: VolumeType::Aggregate as u32,
                shape_words: [0; 3],
                radius: 0.0,
                group_id: 0,
                surface_id: 0,
                flags: 0,
            },
            aggregate: None,
        };
        arena.aggregate_roots = vec![volume(9), volume(7)];
        let first_matrix = std::array::from_fn(|index| {
            f32::from_bits(0x3f00_0000 + u32::try_from(index).unwrap())
        });
        let second_matrix = std::array::from_fn(|index| {
            f32::from_bits(0x4000_0000 + u32::try_from(index).unwrap())
        });
        let parts = [
            AssemblyPartDefinition {
                volume: ObjectId(7),
                inertia: skate2_formats::collision::AssemblyPartInertia {
                    inverse_tensor: [0.0; 4],
                    inverse_mass: 0.0,
                    spherical: 0.0,
                    max_velocity: 0.0,
                    max_omega: 0.0,
                    linear_drag: 0.0,
                    angular_drag: 0.0,
                },
                inverse_body_transform: first_matrix,
                material: skate2_formats::collision::AssemblyPartMaterial {
                    static_friction: 0.0,
                    dynamic_friction: 0.0,
                    restitution: 0.0,
                },
                has_inverse_body_transform: 0x8000_0001,
                is_static_body: 0x7fff_ffff,
            },
            AssemblyPartDefinition {
                volume: ObjectId(9),
                inertia: skate2_formats::collision::AssemblyPartInertia {
                    inverse_tensor: [0.0; 4],
                    inverse_mass: 0.0,
                    spherical: 0.0,
                    max_velocity: 0.0,
                    max_omega: 0.0,
                    linear_drag: 0.0,
                    angular_drag: 0.0,
                },
                inverse_body_transform: second_matrix,
                material: skate2_formats::collision::AssemblyPartMaterial {
                    static_friction: 0.0,
                    dynamic_friction: 0.0,
                    restitution: 0.0,
                },
                has_inverse_body_transform: 2,
                is_static_body: 3,
            },
        ];

        let mapped = dmo_clustered_mesh_template_parts(&arena, 0x55, &parts).unwrap();
        assert_eq!(mapped[0].root_index, 1);
        assert_eq!(mapped[1].root_index, 0);
        assert_eq!(
            mapped[0].inverse_body_transform.map(f32::to_bits),
            first_matrix.map(f32::to_bits)
        );
        assert_eq!(mapped[0].has_inverse_body_transform, 0x8000_0001);
        assert_eq!(mapped[0].is_static_body, 0x7fff_ffff);
        assert_eq!(
            mapped[1].inverse_body_transform.map(f32::to_bits),
            second_matrix.map(f32::to_bits)
        );
        assert_eq!(mapped[1].has_inverse_body_transform, 2);
        assert_eq!(mapped[1].is_static_body, 3);

        let mut external = parts[0].clone();
        external.volume = ObjectId((1 << 22) | 7);
        assert!(dmo_clustered_mesh_template_parts(&arena, 0x55, &[external]).is_err());
        let mut missing = parts[0].clone();
        missing.volume = ObjectId(11);
        assert!(dmo_clustered_mesh_template_parts(&arena, 0x55, &[missing]).is_err());
        arena.aggregate_roots.push(volume(7));
        assert!(dmo_clustered_mesh_template_parts(&arena, 0x55, &parts[..1]).is_err());
    }

    #[test]
    fn dmo_template_publication_requires_all_three_part_counts_to_match() {
        assert_eq!(validate_dmo_assembly_part_counts(0x55, 2, 2, 2), Ok(()));
        assert!(validate_dmo_assembly_part_counts(0x55, 1, 2, 2).is_err());
        assert!(validate_dmo_assembly_part_counts(0x55, 2, 2, 1).is_err());
    }

    #[test]
    fn dmo_collision_model_resolution_rejects_missing_invalid_and_duplicate_owners() {
        let mut arena = empty_collision_arena();
        assert!(unique_dmo_collision_model(&arena, 0x55).is_err());

        let instance = |collision_model| CollisionInstance {
            guid: 1,
            local_guid: 0x55,
            name: String::new(),
            transform: catalog_locator(0, [0.0; 3]).transform,
            bounding_box_min: [0.0; 3],
            bounding_box_max: [0.0; 3],
            collision_model,
        };
        arena.instances.push(instance(None));
        assert!(unique_dmo_collision_model(&arena, 0x55).is_err());
        arena.instances[0].collision_model = Some(1);
        assert!(unique_dmo_collision_model(&arena, 0x55).is_err());

        arena.collision_models.push(CollisionModel {
            dictionary_index: 7,
            bounding_root: None,
            collision_roots: Vec::new(),
            assembly_object_id: None,
        });
        arena.instances[0].collision_model = Some(0);
        assert_eq!(
            unique_dmo_collision_model(&arena, 0x55).unwrap(),
            &arena.collision_models[0]
        );
        arena.instances.push(instance(Some(0)));
        assert!(unique_dmo_collision_model(&arena, 0x55).is_err());
    }

    #[test]
    fn census_candidates_use_horizontal_distance_and_include_radius_boundary() {
        let catalog = test_catalog(vec![
            catalog_locator(1, [60.0, 1_000_000.0, 0.0]),
            catalog_locator(2, [10.0, -1_000_000.0, 0.0]),
            catalog_locator(3, [60.01, 0.0, 0.0]),
        ]);

        let ids = catalog
            .census_candidates([0.0, 0.0, 0.0], true)
            .into_iter()
            .map(|locator| locator.locator_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, [2, 1]);
    }

    #[test]
    fn census_candidates_are_nearest_first_and_capped_at_256() {
        let locators = (0_u16..300)
            .map(|index| {
                catalog_locator(u64::from(index), [f32::from(300 - index) * 0.1, 0.0, 0.0])
            })
            .collect();
        let catalog = test_catalog(locators);

        let candidates = catalog.census_candidates([0.0, 0.0, 0.0], true);
        assert_eq!(candidates.len(), 256);
        assert_eq!(
            candidates.first().map(|locator| locator.locator_id),
            Some(299)
        );
        assert_eq!(
            candidates.last().map(|locator| locator.locator_id),
            Some(44)
        );
    }

    #[test]
    fn noninitial_census_excludes_only_strictly_inside_inner_radius() {
        let mut catalog = test_catalog(vec![
            catalog_locator(1, [9.99, 0.0, 0.0]),
            catalog_locator(2, [10.0, 0.0, 0.0]),
            catalog_locator(3, [10.01, 0.0, 0.0]),
        ]);
        catalog.census.no_spawn_radius = 10.0;

        let ids = catalog
            .census_candidates([0.0; 3], false)
            .into_iter()
            .map(|locator| locator.locator_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, [2, 3]);
        assert_eq!(catalog.census_candidates([0.0; 3], true).len(), 3);
    }

    #[test]
    fn catalog_candidate_uses_live_physics_owners_for_admission() {
        let mut candidate = catalog_locator(1, [0.0; 3]);
        add_unit_sphere(&mut candidate);
        candidate.priority_index = 10;
        let catalog = test_catalog(vec![candidate]);
        let priority_context = DynamicObjectPriorityContext {
            simulation_center: [0.0; 3],
            camera_position: [0.0; 3],
            camera_heading: [1.0, 0.0, 0.0],
        };
        let live_priority = DynamicObjectPriorityIndex {
            object_id: 7,
            priority: 10.0,
            safety: 0.0,
            other_factors: 0.0,
            multiplier: 1.0,
        };
        let live_body = catalog.locators[0].collision_body();
        let live_dynamic_objects = vec![PrioritizedDynamicCollisionBody {
            priority: live_priority,
            body: live_body.clone(),
        }];
        let live_vehicles = vec![OwnedDynamicCollisionBody {
            object_id: 20,
            body: live_body,
        }];

        assert_eq!(
            catalog.admission_for_snapshot(
                &catalog.locators[0],
                &RuntimeDmoAdmissionSnapshot {
                    priority_context,
                    safety_index: 0,
                    is_static: false,
                    lowest_pool_object: None,
                    live_dynamic_objects: &live_dynamic_objects,
                    live_vehicles: &live_vehicles,
                },
            ),
            Err(DynamicObjectAdmissionRejection::OverlapPriority {
                blocking_object_id: 7,
            })
        );

        let mut higher_candidate = catalog.locators[0].clone();
        higher_candidate.priority_index = 11;
        let plan = catalog
            .admission_for_snapshot(
                &higher_candidate,
                &RuntimeDmoAdmissionSnapshot {
                    priority_context,
                    safety_index: 0,
                    is_static: false,
                    lowest_pool_object: None,
                    live_dynamic_objects: &live_dynamic_objects,
                    live_vehicles: &live_vehicles,
                },
            )
            .expect("higher-priority overlapping locator should be admitted");
        assert_eq!(plan.overlapping_dynamic_objects, [7]);
        assert_eq!(plan.overlapping_vehicles, [20]);
    }

    #[test]
    fn census_spawns_after_ordered_cull_requests_and_reuses_only_destroyed_slots() {
        let mut first = catalog_locator(1, [0.0; 3]);
        add_unit_sphere(&mut first);
        first.priority_index = 1;
        let mut replacement = catalog_locator(2, [0.0; 3]);
        add_unit_sphere(&mut replacement);
        replacement.priority_index = 11;
        let mut third = catalog_locator(3, [10.0, 0.0, 0.0]);
        add_unit_sphere(&mut third);
        third.priority_index = 1;
        let catalog = test_catalog(vec![first, replacement, third]);
        let context = DynamicObjectPriorityContext {
            simulation_center: [0.0; 3],
            camera_position: [0.0; 3],
            camera_heading: [1.0, 0.0, 0.0],
        };
        let mut ids = LivingWorldEntityIdAllocator::default();
        let mut census = RuntimeDmoCensus::default();

        let (first_id, _) = census
            .admit_and_spawn(
                &catalog,
                &catalog.locators[0],
                context,
                0,
                false,
                None,
                &[],
                &mut ids,
                |_| {},
            )
            .expect("first DMO should spawn");
        assert_eq!(first_id, 0x0500_0001);
        assert_eq!(census.entities[0].allocation.entity_slot, 0);

        let vehicle_body = catalog.locators[0].collision_body();
        let mut vehicle_culls = Vec::new();
        let (replacement_id, plan) = census
            .admit_and_spawn(
                &catalog,
                &catalog.locators[1],
                context,
                0,
                false,
                None,
                &[OwnedDynamicCollisionBody {
                    object_id: 20,
                    body: vehicle_body,
                }],
                &mut ids,
                |object_id| vehicle_culls.push(object_id),
            )
            .expect("higher-priority replacement should spawn");
        assert_eq!(plan.overlapping_dynamic_objects, [first_id]);
        assert_eq!(vehicle_culls, [20]);
        assert_eq!(replacement_id, 0x0500_0002);
        assert_eq!(census.entities[1].allocation.entity_slot, 1);
        assert_eq!(census.live_collision_snapshot().len(), 1);

        assert_eq!(census.finish_cull_update(0), Some(vec![first_id]));
        assert_eq!(census.entities.len(), 1);
        let (third_id, _) = census
            .admit_and_spawn(
                &catalog,
                &catalog.locators[2],
                context,
                0,
                false,
                None,
                &[],
                &mut ids,
                |_| {},
            )
            .expect("freed DMO slot should be reusable");
        assert_eq!(third_id, 0x0500_0003);
        assert_eq!(census.entities[1].allocation.entity_slot, 0);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LivingWorldCensusCircle {
    no_spawn_radius: f32,
    spawn_radius: f32,
    cull_radius: f32,
    shift_distance_along_velocity: f32,
    speed: f32,
}

fn decode_living_world_census_circle(
    bytes: &[u8],
) -> Result<LivingWorldCensusCircle, &'static str> {
    let value = |offset: usize| {
        bytes
            .get(offset..offset + 4)
            .ok_or("truncated living-world census circle")?
            .try_into()
            .map(f32::from_be_bytes)
            .map_err(|_| "invalid living-world census circle")
    };
    Ok(LivingWorldCensusCircle {
        no_spawn_radius: value(0)?,
        spawn_radius: value(4)?,
        cull_radius: value(8)?,
        shift_distance_along_velocity: value(12)?,
        speed: value(16)?,
    })
}

struct PendingExternalLink {
    arena_id: u32,
    entry_index: u32,
    expected_types: Vec<u32>,
    failure_stage: &'static str,
}

fn simulation_pointer_fnv_bytes(signature: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *signature ^= u64::from(byte);
        *signature = signature.wrapping_mul(SIMULATION_POINTER_FNV_PRIME);
    }
}

fn simulation_pointer_fnv_u32(signature: &mut u64, value: u32) {
    simulation_pointer_fnv_bytes(signature, &value.to_be_bytes());
}

fn simulation_pointer_fnv_u64(signature: &mut u64, value: u64) {
    simulation_pointer_fnv_bytes(signature, &value.to_be_bytes());
}

fn hash_location_string(
    signature: &mut u64,
    value: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    match value {
        None => simulation_pointer_fnv_bytes(signature, &[0]),
        Some(value) => {
            simulation_pointer_fnv_bytes(signature, &[1]);
            simulation_pointer_fnv_u32(signature, u32::try_from(value.len())?);
            simulation_pointer_fnv_bytes(signature, value.as_bytes());
        }
    }
    Ok(())
}

fn add_location_plan_collection(
    totals: &mut LocationPlanInventory,
    collection_ordinal: usize,
    collection_id: u64,
    collection_name: &str,
    asset_count: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    totals.collections += 1;
    simulation_pointer_fnv_bytes(&mut totals.signature, b"C");
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(collection_ordinal)?);
    simulation_pointer_fnv_u64(&mut totals.signature, collection_id);
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(collection_name.len())?);
    simulation_pointer_fnv_bytes(&mut totals.signature, collection_name.as_bytes());
    simulation_pointer_fnv_u32(&mut totals.signature, asset_count);
    Ok(())
}

fn add_location_arena_plan(
    inventory: &mut SimulationInventory,
    collection_ordinal: usize,
    arena_ordinal: usize,
    plan: &LocationArenaPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    let collection_ordinal = u32::try_from(collection_ordinal)?;
    let arena_ordinal = u32::try_from(arena_ordinal)?;
    let asset = plan.normal_plan().static_plan().asset();
    let totals = &mut inventory.location_plan;
    totals.arenas += 1;
    simulation_pointer_fnv_bytes(&mut totals.signature, b"A");
    simulation_pointer_fnv_u32(&mut totals.signature, collection_ordinal);
    simulation_pointer_fnv_u32(&mut totals.signature, arena_ordinal);
    simulation_pointer_fnv_u64(&mut totals.signature, asset.decoded.asset_id);
    simulation_pointer_fnv_u32(&mut totals.signature, asset.decoded.arena.id);
    simulation_pointer_fnv_u32(
        &mut totals.signature,
        u32::try_from(plan.operations().len())?,
    );
    inventory.location_plan_edges.maximum_containers_per_arena = inventory
        .location_plan_edges
        .maximum_containers_per_arena
        .max(plan.operations().len());
    totals.arenas_with_container += usize::from(!plan.operations().is_empty());

    for (operation_index, operation) in plan.operations().iter().enumerate() {
        let source = plan
            .source_handle(operation_index)
            .ok_or("Location plan lost a whole-object source")?;
        let parsed = LocationDescData::parse(source.bytes())?;
        if source.location_count() != parsed.locations.len() {
            return Err("Location source record count changed after planning".into());
        }
        for (location_index, location) in parsed.locations.iter().enumerate() {
            let handle = source
                .location_handle(location_index)
                .ok_or("Location plan lost an exact record source")?;
            if handle.bytes().len() != 128 || handle.name()?.as_deref() != location.name.as_deref()
            {
                return Err("Location record handle disagrees with its retained object".into());
            }
        }
        add_location_plan_operation(
            totals,
            &mut inventory.location_plan_edges,
            asset.decoded.asset_id,
            operation,
            source.bytes(),
        )?;
    }
    Ok(())
}

fn add_location_plan_operation(
    totals: &mut LocationPlanInventory,
    edges: &mut LocationPlanEdges,
    arena_asset_id: u64,
    operation: &LocationArenaOperation,
    source_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    totals.containers += 1;
    simulation_pointer_fnv_bytes(&mut totals.signature, b"K");
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_toc_index);
    simulation_pointer_fnv_u64(&mut totals.signature, operation.asset_id);
    simulation_pointer_fnv_u32(&mut totals.signature, LOCATION_DESC_DATA_TYPE_ID);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.raw_object_id);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.toc_dictionary_index);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_dictionary_index);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_resource_index);
    simulation_pointer_fnv_u64(&mut totals.signature, operation.source_span_start);
    simulation_pointer_fnv_u64(&mut totals.signature, operation.source_span_end);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.string_count);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.locations_offset);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.sub_locations_offset);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.string_list_offset);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.location_count);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.sub_location_count);

    if source_bytes.len() != usize::try_from(operation.source_object_len)? {
        return Err("Location source span length changed".into());
    }
    let parsed = LocationDescData::parse(source_bytes)?;
    if parsed.locations.len() != usize::try_from(operation.location_count)?
        || parsed.sub_locations.len() != usize::try_from(operation.sub_location_count)?
        || parsed.string_count != operation.string_count
        || parsed.locations_offset != operation.locations_offset
        || parsed.sub_locations_offset != operation.sub_locations_offset
        || parsed.string_list_offset != operation.string_list_offset
    {
        return Err("Location parsed descriptor changed after planning".into());
    }

    totals.zero_location_containers += usize::from(parsed.locations.is_empty());
    totals.locations += parsed.locations.len();
    totals.sublocations += parsed.sub_locations.len();
    totals.string_count += usize::try_from(parsed.string_count)?;
    edges.maximum_locations_per_container = edges
        .maximum_locations_per_container
        .max(parsed.locations.len());
    edges.maximum_sublocations_per_container = edges
        .maximum_sublocations_per_container
        .max(parsed.sub_locations.len());
    *edges
        .locations_per_container
        .entry(parsed.locations.len())
        .or_default() += 1;
    edges
        .source_resources
        .insert(operation.source_resource_index);
    edges.minimum_source_span = Some(
        edges
            .minimum_source_span
            .map_or(source_bytes.len(), |current| {
                current.min(source_bytes.len())
            }),
    );
    edges.maximum_source_span = Some(
        edges
            .maximum_source_span
            .map_or(source_bytes.len(), |current| {
                current.max(source_bytes.len())
            }),
    );
    edges.zero_container_ids += usize::from(operation.asset_id == 0);
    edges.duplicate_container_ids += usize::from(!edges.container_ids.insert(operation.asset_id));
    edges.source_span_aliases += usize::from(!edges.source_spans.insert((
        arena_asset_id,
        operation.source_resource_index,
        operation.source_span_start,
        operation.source_span_end,
    )));

    for (index, location) in parsed.locations.iter().enumerate() {
        add_location_descriptor(totals, edges, index, location)?;
    }

    for (index, sublocation) in parsed.sub_locations.iter().enumerate() {
        add_sublocation_descriptor(totals, edges, index, sublocation)?;
    }
    Ok(())
}

fn hash_location_transform_and_box(
    signature: &mut u64,
    transform: &[f32; 16],
    minimum: &[f32; 4],
    maximum: &[f32; 4],
) {
    for value in transform.iter().chain(minimum).chain(maximum) {
        simulation_pointer_fnv_u32(signature, value.to_bits());
    }
}

fn add_location_descriptor(
    totals: &mut LocationPlanInventory,
    edges: &mut LocationPlanEdges,
    index: usize,
    location: &LocationDesc,
) -> Result<(), Box<dyn std::error::Error>> {
    simulation_pointer_fnv_bytes(&mut totals.signature, b"D");
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(index)?);
    hash_location_transform_and_box(
        &mut totals.signature,
        &location.transform,
        &location.bounding_box.minimum,
        &location.bounding_box.maximum,
    );
    simulation_pointer_fnv_u32(&mut totals.signature, location.sub_location_count);
    simulation_pointer_fnv_u32(
        &mut totals.signature,
        location.sub_locations_offset.unwrap_or(u32::MAX),
    );
    hash_location_string(&mut totals.signature, location.name.as_deref())?;
    hash_location_string(&mut totals.signature, location.description.as_deref())?;
    simulation_pointer_fnv_u64(&mut totals.signature, location.guid);

    edges.named_locations += usize::from(location.name.is_some());
    edges.described_locations += usize::from(location.description.is_some());
    edges.zero_location_ids += usize::from(location.guid == 0);
    edges.duplicate_location_ids += usize::from(!edges.location_ids.insert(location.guid));
    if let Some(name) = &location.name {
        edges.duplicate_location_names += usize::from(!edges.location_names.insert(name.clone()));
        let hash = sk8_hash_value_32(name.as_bytes());
        if let Some(existing) = edges.location_hash_names.insert(hash, name.clone())
            && existing != *name
        {
            edges.location_hash_collisions += 1;
        }
    }
    Ok(())
}

fn add_sublocation_descriptor(
    totals: &mut LocationPlanInventory,
    edges: &mut LocationPlanEdges,
    index: usize,
    sublocation: &SubLocationDesc,
) -> Result<(), Box<dyn std::error::Error>> {
    simulation_pointer_fnv_bytes(&mut totals.signature, b"d");
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(index)?);
    hash_location_transform_and_box(
        &mut totals.signature,
        &sublocation.transform,
        &sublocation.bounding_box.minimum,
        &sublocation.bounding_box.maximum,
    );
    hash_location_string(&mut totals.signature, sublocation.name.as_deref())?;
    hash_location_string(&mut totals.signature, sublocation.description.as_deref())?;
    edges.named_sublocations += usize::from(sublocation.name.is_some());
    edges.described_sublocations += usize::from(sublocation.description.is_some());
    Ok(())
}

fn finish_location_plan(
    inventory: &mut SimulationInventory,
) -> Result<(), Box<dyn std::error::Error>> {
    let totals = &mut inventory.location_plan;
    simulation_pointer_fnv_bytes(&mut totals.signature, b"Z");
    for count in [
        totals.collections,
        totals.arenas,
        totals.containers,
        totals.arenas_with_container,
        totals.zero_location_containers,
        totals.locations,
        totals.sublocations,
        totals.string_count,
    ] {
        simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(count)?);
    }
    let edges = &inventory.location_plan_edges;
    if *totals != EXPECTED_LOCATION_PLAN
        || edges.container_ids.len() != 107
        || edges.location_ids.len() != 53
        || edges.location_names.len() != 53
        || edges.location_hash_names.len() != 53
        || edges.source_spans.len() != 107
        || edges.source_resources != BTreeSet::from([0])
        || edges.locations_per_container
            != BTreeMap::from([(0, 83), (1, 8), (2, 8), (3, 5), (4, 1), (5, 2)])
        || edges.source_span_aliases != 0
        || edges.zero_container_ids != 0
        || edges.zero_location_ids != 0
        || edges.duplicate_container_ids != 0
        || edges.duplicate_location_ids != 0
        || edges.duplicate_location_names != 0
        || edges.location_hash_collisions != 0
        || edges.named_locations != 53
        || edges.described_locations != 53
        || edges.named_sublocations != 0
        || edges.described_sublocations != 0
        || edges.maximum_containers_per_arena != 1
        || edges.maximum_locations_per_container != 5
        || edges.maximum_sublocations_per_container != 0
        || edges.minimum_source_span != Some(32)
        || edges.maximum_source_span != Some(778)
        || inventory.location_descriptors != totals.locations
        || inventory.sub_location_descriptors != totals.sublocations
    {
        return Err(
            format!("Location kind-21 corpus mismatch: {totals:?}, edges {edges:?}").into(),
        );
    }
    Ok(())
}

fn add_location_runtime_collection(
    totals: &mut LocationRuntimeInventory,
    collection_ordinal: usize,
    collection_id: u64,
    collection_name: &str,
    arena_count: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    totals.collections += 1;
    for (signature, tag) in [
        (&mut totals.reg_signature, b"C".as_slice()),
        (&mut totals.dreg_signature, b"c".as_slice()),
    ] {
        simulation_pointer_fnv_bytes(signature, tag);
        simulation_pointer_fnv_u32(signature, u32::try_from(collection_ordinal)?);
        simulation_pointer_fnv_u64(signature, collection_id);
        simulation_pointer_fnv_u32(signature, u32::try_from(collection_name.len())?);
        simulation_pointer_fnv_bytes(signature, collection_name.as_bytes());
        simulation_pointer_fnv_u32(signature, arena_count);
    }
    Ok(())
}

fn hash_location_runtime_arena(
    signature: &mut u64,
    tag: u8,
    collection_ordinal: usize,
    arena_ordinal: usize,
    prepared: &PreparedLocationArena,
) -> Result<(), Box<dyn std::error::Error>> {
    let asset = prepared.plan().normal_plan().static_plan().asset();
    simulation_pointer_fnv_bytes(signature, &[tag]);
    simulation_pointer_fnv_u32(signature, u32::try_from(collection_ordinal)?);
    simulation_pointer_fnv_u32(signature, u32::try_from(arena_ordinal)?);
    simulation_pointer_fnv_u64(signature, asset.decoded.asset_id);
    simulation_pointer_fnv_u32(signature, asset.decoded.arena.id);
    simulation_pointer_fnv_u32(signature, u32::try_from(prepared.container_count())?);
    Ok(())
}

fn hash_location_runtime_container(
    signature: &mut u64,
    tag: u8,
    operation: LocationArenaOperation,
) {
    simulation_pointer_fnv_bytes(signature, &[tag]);
    simulation_pointer_fnv_u64(signature, operation.asset_id);
    simulation_pointer_fnv_u32(signature, 21);
    simulation_pointer_fnv_u32(signature, LOCATION_DESC_DATA_TYPE_ID);
    simulation_pointer_fnv_u32(signature, operation.source_toc_index);
    simulation_pointer_fnv_u32(signature, operation.raw_object_id);
    simulation_pointer_fnv_u32(signature, operation.toc_dictionary_index);
    simulation_pointer_fnv_u32(signature, operation.source_dictionary_index);
    simulation_pointer_fnv_u32(signature, operation.source_resource_index);
    simulation_pointer_fnv_u64(signature, operation.source_span_start);
    simulation_pointer_fnv_u64(signature, operation.source_span_end);
    simulation_pointer_fnv_u32(signature, operation.location_count);
}

fn location_record_guid(
    record: &skate2_core::location_arena_plan::LocationRecordHandle,
) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(u64::from_be_bytes(
        record
            .bytes()
            .get(112..120)
            .ok_or("Location record lost its GUID")?
            .try_into()?,
    ))
}

fn hash_location_runtime_descriptor_prefix(
    signature: &mut u64,
    tag: u8,
    descriptor_index: usize,
    container_id: u64,
    arena_asset_id: u64,
    record: &skate2_core::location_arena_plan::LocationRecordHandle,
    name_hash: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    simulation_pointer_fnv_bytes(signature, &[tag]);
    simulation_pointer_fnv_u32(signature, u32::try_from(descriptor_index)?);
    simulation_pointer_fnv_u64(signature, container_id);
    simulation_pointer_fnv_u64(signature, arena_asset_id);
    simulation_pointer_fnv_u32(signature, u32::try_from(record.resource_index())?);
    simulation_pointer_fnv_u64(signature, u64::try_from(record.resource_offset())?);
    simulation_pointer_fnv_u32(signature, name_hash);
    simulation_pointer_fnv_u64(signature, location_record_guid(record)?);
    Ok(())
}

fn publish_location_runtime_sources(
    adapter: &mut LocationArenaRegistrationAdapter<LocationCorpusRuntimeSource>,
    prepared: &PreparedLocationArena,
) -> Result<(), Box<dyn std::error::Error>> {
    for container_index in 0..prepared.container_count() {
        let operation = prepared
            .operation(container_index)
            .ok_or("prepared Location operation disappeared")?;
        let source = prepared
            .planned_source(container_index)
            .ok_or("prepared Location source disappeared")?;
        if adapter
            .runtime_mut()
            .registry
            .insert(operation.asset_id, source)
            .is_some()
        {
            return Err(format!(
                "Location corpus repeats live container {:016x}",
                operation.asset_id
            )
            .into());
        }
    }
    Ok(())
}

fn register_location_runtime_arena(
    totals: &mut LocationRuntimeInventory,
    adapter: &mut LocationArenaRegistrationAdapter<LocationCorpusRuntimeSource>,
    collection_ordinal: usize,
    arena_ordinal: usize,
    prepared: &PreparedLocationArena,
) -> Result<LocationArenaRegistration, Box<dyn std::error::Error>> {
    totals.arenas += 1;
    totals.registration_tokens += 1;
    hash_location_runtime_arena(
        &mut totals.reg_signature,
        b'A',
        collection_ordinal,
        arena_ordinal,
        prepared,
    )?;
    let asset_id = prepared
        .plan()
        .normal_plan()
        .static_plan()
        .asset()
        .decoded
        .asset_id;
    let resolution_start = adapter.runtime().resolutions.len();
    let sample_start = adapter.runtime().priority_samples;
    let mut expected_entries = Vec::new();
    for container_index in 0..prepared.container_count() {
        let operation = prepared
            .operation(container_index)
            .ok_or("prepared Location operation disappeared")?;
        let source = prepared
            .planned_source(container_index)
            .ok_or("prepared Location source disappeared")?;
        totals.containers += 1;
        totals.empty_containers += usize::from(source.location_count() == 0);
        hash_location_runtime_container(&mut totals.reg_signature, b'K', operation);
        for descriptor_index in 0..source.location_count() {
            let record = source
                .location_handle(descriptor_index)
                .ok_or("prepared Location descriptor disappeared")?;
            let Some(name) = record.name_bytes()? else {
                continue;
            };
            let name_hash = sk8_hash_value_32(name);
            let sample_ordinal = totals.priority_samples;
            let high_priority = sample_ordinal % 2 == 0;
            hash_location_runtime_descriptor_prefix(
                &mut totals.reg_signature,
                b'D',
                descriptor_index,
                operation.asset_id,
                asset_id,
                &record,
                name_hash,
            )?;
            simulation_pointer_fnv_u32(&mut totals.reg_signature, u32::try_from(sample_ordinal)?);
            simulation_pointer_fnv_bytes(&mut totals.reg_signature, &[u8::from(high_priority), 1]);
            totals.descriptors += 1;
            totals.priority_samples += 1;
            totals.high_priority += usize::from(high_priority);
            totals.low_priority += usize::from(!high_priority);
            expected_entries.push((record, name_hash, u8::from(high_priority)));
        }
    }
    let token = adapter.register(prepared)?;
    let expected_resolutions = (0..prepared.container_count())
        .map(|index| {
            prepared
                .operation(index)
                .expect("prepared Location operation disappeared")
                .asset_id
        })
        .collect::<Vec<_>>();
    if adapter.runtime().resolutions[resolution_start..] != expected_resolutions
        || adapter.runtime().priority_samples - sample_start != expected_entries.len()
    {
        return Err("Location registration runtime call order changed".into());
    }
    for (record, name_hash, high_priority) in expected_entries {
        let matches = adapter
            .entries()
            .iter()
            .filter(|entry| {
                entry.name_hash() == name_hash
                    && entry.high_priority_raw() == high_priority
                    && entry.streamed_location()
                    && entry.descriptor().is_same_backing_record(&record)
            })
            .count();
        if matches != 1 {
            return Err("Location registration did not publish one exact record".into());
        }
    }
    Ok(token)
}

fn deregister_location_runtime_arena(
    totals: &mut LocationRuntimeInventory,
    adapter: &mut LocationArenaRegistrationAdapter<LocationCorpusRuntimeSource>,
    collection_ordinal: usize,
    arena_ordinal: usize,
    prepared: &PreparedLocationArena,
    token: LocationArenaRegistration,
) -> Result<(), Box<dyn std::error::Error>> {
    hash_location_runtime_arena(
        &mut totals.dreg_signature,
        b'a',
        collection_ordinal,
        arena_ordinal,
        prepared,
    )?;
    let asset_id = prepared
        .plan()
        .normal_plan()
        .static_plan()
        .asset()
        .decoded
        .asset_id;
    let resolution_start = adapter.runtime().resolutions.len();
    let sample_start = adapter.runtime().priority_samples;
    let mut removed_records = Vec::new();
    for container_index in 0..prepared.container_count() {
        let operation = prepared
            .operation(container_index)
            .ok_or("prepared Location operation disappeared")?;
        let source = adapter
            .runtime()
            .registry
            .get(&operation.asset_id)
            .ok_or("Location registry lost a live corpus source")?;
        hash_location_runtime_container(&mut totals.dreg_signature, b'k', operation);
        for descriptor_index in 0..source.location_count() {
            let record = source
                .location_handle(descriptor_index)
                .ok_or("fresh Location descriptor disappeared")?;
            let Some(name) = record.name_bytes()? else {
                continue;
            };
            let name_hash = sk8_hash_value_32(name);
            let planned = prepared
                .planned_source(container_index)
                .and_then(|planned| planned.location_handle(descriptor_index))
                .ok_or("planned Location descriptor disappeared")?;
            let matches = record.is_same_backing_record(&planned);
            hash_location_runtime_descriptor_prefix(
                &mut totals.dreg_signature,
                b'd',
                descriptor_index,
                operation.asset_id,
                asset_id,
                &record,
                name_hash,
            )?;
            simulation_pointer_fnv_bytes(&mut totals.dreg_signature, &[u8::from(matches)]);
            totals.source_matches += usize::from(matches);
            totals.source_mismatches += usize::from(!matches);
            removed_records.push(record);
        }
    }
    adapter.deregister(token)?;
    let expected_resolutions = (0..prepared.container_count())
        .map(|index| {
            prepared
                .operation(index)
                .expect("prepared Location operation disappeared")
                .asset_id
        })
        .collect::<Vec<_>>();
    if adapter.runtime().resolutions[resolution_start..] != expected_resolutions {
        return Err("Location deregistration runtime call order changed".into());
    }
    totals.deregistration_priority_samples += adapter.runtime().priority_samples - sample_start;
    for record in removed_records {
        if adapter
            .entries()
            .iter()
            .any(|entry| entry.descriptor().is_same_backing_record(&record))
        {
            return Err("Location deregistration retained an exact corpus record".into());
        }
        totals.removals += 1;
    }
    for container_index in 0..prepared.container_count() {
        let operation = prepared
            .operation(container_index)
            .ok_or("prepared Location operation disappeared")?;
        let removed = adapter.runtime_mut().registry.remove(&operation.asset_id);
        if removed.is_none() {
            return Err("Location registry source disappeared before unpublication".into());
        }
    }
    Ok(())
}

fn finish_location_runtime(
    totals: &mut LocationRuntimeInventory,
    adapter: &LocationArenaRegistrationAdapter<LocationCorpusRuntimeSource>,
) -> Result<(), Box<dyn std::error::Error>> {
    simulation_pointer_fnv_bytes(&mut totals.reg_signature, b"Z");
    for count in [
        totals.collections,
        totals.arenas,
        totals.containers,
        totals.empty_containers,
        totals.descriptors,
        totals.priority_samples,
        totals.high_priority,
        totals.low_priority,
        totals.registration_tokens,
        totals.descriptors,
    ] {
        simulation_pointer_fnv_u32(&mut totals.reg_signature, u32::try_from(count)?);
    }
    simulation_pointer_fnv_bytes(&mut totals.dreg_signature, b"z");
    for count in [
        totals.collections,
        totals.arenas,
        totals.containers,
        totals.empty_containers,
        totals.descriptors,
        totals.removals,
        totals.deregistration_priority_samples,
        totals.source_mismatches,
        adapter.entries().len(),
        adapter.live_registration_count(),
    ] {
        simulation_pointer_fnv_u32(&mut totals.dreg_signature, u32::try_from(count)?);
    }
    if *totals != EXPECTED_LOCATION_RUNTIME
        || !adapter.entries().is_empty()
        || adapter.live_registration_count() != 0
        || !adapter.runtime().registry.is_empty()
        || adapter.runtime().resolutions.len() != totals.containers * 2
        || adapter.runtime().priority_samples != totals.priority_samples
        || adapter.fault().is_some()
    {
        return Err(
            format!("Location runtime corpus mismatch: {totals:?}, adapter {adapter:?}").into(),
        );
    }
    Ok(())
}

fn hash_simulation_dmo_runtime_event(
    signature: &mut u64,
    tag: u8,
    metadata: &SimulationDmoRuntimeMetadata,
    value: u32,
) {
    simulation_pointer_fnv_bytes(signature, &[tag]);
    simulation_pointer_fnv_u32(signature, metadata.global_index);
    simulation_pointer_fnv_u64(signature, metadata.asset_id);
    simulation_pointer_fnv_u32(signature, value);
    simulation_pointer_fnv_u64(signature, metadata.arena_asset_id);
    simulation_pointer_fnv_u32(signature, metadata.resource_index);
    simulation_pointer_fnv_u64(signature, metadata.source_span_start);
    simulation_pointer_fnv_u64(signature, metadata.source_span_end);
}

impl SimulationDmoLoadPolicy for SimulationDmoRuntimePolicy {
    fn load_dmo_locator_from_stream(&mut self) -> bool {
        let mut state = self.state.lock().expect("DMO runtime oracle poisoned");
        let metadata = state.current[state.registration_index].clone();
        let enabled = metadata.global_index % 2 == 0;
        hash_simulation_dmo_runtime_event(
            &mut state.inventory.reg_signature,
            b'F',
            &metadata,
            u32::from(enabled),
        );
        state.inventory.policy_samples += 1;
        state.registration_index += 1;
        enabled
    }
}

impl SimulationDmoArenaDispatch for SimulationDmoRuntimeDispatch {
    fn add_living_world_locator_data(&mut self, asset_id: u64, userdata_raw: u32) {
        let mut state = self.state.lock().expect("DMO runtime oracle poisoned");
        let metadata = state.current[state.registration_index - 1].clone();
        assert_eq!(asset_id, metadata.asset_id);
        hash_simulation_dmo_runtime_event(
            &mut state.inventory.reg_signature,
            b'L',
            &metadata,
            userdata_raw,
        );
        state.inventory.reg_living += 1;
    }

    fn add_challenge_dmos(&mut self, asset_id: u64, userdata_raw: u32) {
        let mut state = self.state.lock().expect("DMO runtime oracle poisoned");
        let metadata = state.current[state.registration_index - 1].clone();
        assert_eq!(asset_id, metadata.asset_id);
        hash_simulation_dmo_runtime_event(
            &mut state.inventory.reg_signature,
            b'C',
            &metadata,
            userdata_raw,
        );
        state.inventory.reg_challenge += 1;
    }

    fn register_dmo_locator(&mut self, source: &SimulationDmoDataHandle, asset_id: u64) {
        let mut state = self.state.lock().expect("DMO runtime oracle poisoned");
        let metadata = state.current[state.registration_index - 1].clone();
        assert_eq!(asset_id, metadata.asset_id);
        assert!(source.is_same_backing_object(&metadata.source));
        hash_simulation_dmo_runtime_event(&mut state.inventory.reg_signature, b'R', &metadata, 7);
        state.inventory.reg_registry += 1;
    }

    fn remove_living_world_locator_data(&mut self, asset_id: u64, userdata_raw: u32) {
        let mut state = self.state.lock().expect("DMO runtime oracle poisoned");
        let metadata = state.current[state.deregistration_index].clone();
        assert_eq!(asset_id, metadata.asset_id);
        hash_simulation_dmo_runtime_event(
            &mut state.inventory.dreg_signature,
            b'D',
            &metadata,
            userdata_raw,
        );
        hash_simulation_dmo_runtime_event(
            &mut state.inventory.dreg_signature,
            b'l',
            &metadata,
            userdata_raw,
        );
        state.inventory.dreg_living += 1;
        state.deregistration_index += 1;
    }

    fn remove_challenge_dmos(&mut self, asset_id: u64, userdata_raw: u32) {
        let mut state = self.state.lock().expect("DMO runtime oracle poisoned");
        let metadata = state.current[state.deregistration_index].clone();
        assert_eq!(asset_id, metadata.asset_id);
        hash_simulation_dmo_runtime_event(
            &mut state.inventory.dreg_signature,
            b'D',
            &metadata,
            userdata_raw,
        );
        hash_simulation_dmo_runtime_event(
            &mut state.inventory.dreg_signature,
            b'c',
            &metadata,
            userdata_raw,
        );
        state.inventory.dreg_challenge += 1;
    }

    fn deregister_dmo_locator(&mut self, source: &SimulationDmoDataHandle, asset_id: u64) {
        let mut state = self.state.lock().expect("DMO runtime oracle poisoned");
        let metadata = state.current[state.deregistration_index].clone();
        assert_eq!(asset_id, metadata.asset_id);
        if source.is_same_backing_object(&metadata.source) {
            state.inventory.dreg_source_matches += 1;
        } else {
            state.inventory.source_mismatches += 1;
        }
        hash_simulation_dmo_runtime_event(&mut state.inventory.dreg_signature, b'r', &metadata, 7);
        state.inventory.dreg_registry += 1;
        state.deregistration_index += 1;
    }
}

fn exercise_simulation_dmo_runtime(
    state: &Arc<Mutex<SimulationDmoRuntimeState>>,
    plan: &Arc<SimulationDmoArenaPlan>,
) -> Result<(), Box<dyn std::error::Error>> {
    let prepared = PreparedSimulationDmoArena::try_new(Arc::clone(plan))?;
    let mut metadata = Vec::new();
    metadata.try_reserve_exact(plan.operations().len())?;
    {
        let state = state.lock().expect("DMO runtime oracle poisoned");
        let base = state.inventory.operations;
        drop(state);
        for (operation_index, operation) in plan.operations().iter().enumerate() {
            let source = plan
                .source_handle(operation_index)
                .ok_or("Simulation DMO runtime plan lost its source")?;
            metadata.push(SimulationDmoRuntimeMetadata {
                global_index: u32::try_from(
                    base.checked_add(operation_index)
                        .ok_or("Simulation DMO runtime global operation index overflowed")?,
                )?,
                asset_id: operation.asset_id,
                arena_asset_id: plan.normal_plan().static_plan().asset().decoded.asset_id,
                resource_index: operation.source_resource_index,
                source_span_start: operation.source_span_start,
                source_span_end: operation.source_span_end,
                source,
            });
        }
    }
    {
        let mut state = state.lock().expect("DMO runtime oracle poisoned");
        state.inventory.operations += metadata.len();
        state.inventory.prepared_source_matches += metadata.len();
        state.current = metadata;
        state.registration_index = 0;
        state.deregistration_index = 0;
    }
    let policy = SimulationDmoRuntimePolicy {
        state: Arc::clone(state),
    };
    let dispatch = SimulationDmoRuntimeDispatch {
        state: Arc::clone(state),
    };
    let mut adapter = SimulationDmoArenaRegistrationAdapter::new(policy, dispatch);
    let registration = adapter.register(&prepared, 7)?;
    adapter.deregister(registration, 7)?;
    let state = state.lock().expect("DMO runtime oracle poisoned");
    if state.registration_index != state.current.len()
        || state.deregistration_index != state.current.len()
        || adapter.live_registration_count() != 0
    {
        return Err("Simulation DMO runtime adapter did not consume its complete arena".into());
    }
    Ok(())
}

fn finish_simulation_dmo_runtime(
    state: &mut SimulationDmoRuntimeState,
) -> Result<(), Box<dyn std::error::Error>> {
    let inventory = &mut state.inventory;
    simulation_pointer_fnv_bytes(&mut inventory.reg_signature, b"Z");
    simulation_pointer_fnv_u32(
        &mut inventory.reg_signature,
        u32::try_from(inventory.operations)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.reg_signature,
        u32::try_from(inventory.reg_living)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.reg_signature,
        u32::try_from(inventory.reg_challenge)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.reg_signature,
        u32::try_from(inventory.reg_registry)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.reg_signature,
        u32::try_from(inventory.prepared_source_matches)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.reg_signature,
        u32::try_from(inventory.source_mismatches)?,
    );
    simulation_pointer_fnv_bytes(&mut inventory.dreg_signature, b"Z");
    simulation_pointer_fnv_u32(
        &mut inventory.dreg_signature,
        u32::try_from(inventory.operations)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.dreg_signature,
        u32::try_from(inventory.dreg_living)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.dreg_signature,
        u32::try_from(inventory.dreg_challenge)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.dreg_signature,
        u32::try_from(inventory.dreg_registry)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.dreg_signature,
        u32::try_from(inventory.dreg_source_matches)?,
    );
    simulation_pointer_fnv_u32(
        &mut inventory.dreg_signature,
        u32::try_from(inventory.source_mismatches)?,
    );
    if *inventory != EXPECTED_SIMULATION_DMO_RUNTIME {
        return Err(format!("Simulation DMO runtime corpus mismatch: {inventory:?}").into());
    }
    Ok(())
}

fn add_simulation_dmo_collection(
    totals: &mut SimulationDmoPlanInventory,
    collection_ordinal: usize,
    collection_id: u64,
    collection_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    totals.collections += 1;
    simulation_pointer_fnv_bytes(&mut totals.signature, b"C");
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(collection_ordinal)?);
    simulation_pointer_fnv_u64(&mut totals.signature, collection_id);
    simulation_pointer_fnv_bytes(&mut totals.signature, collection_name.as_bytes());
    simulation_pointer_fnv_bytes(&mut totals.signature, &[0]);
    Ok(())
}

fn add_simulation_dmo_plan(
    inventory: &mut SimulationInventory,
    collection_ordinal: usize,
    arena_ordinal: usize,
    plan: &SimulationDmoArenaPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    let collection_ordinal = u32::try_from(collection_ordinal)?;
    let arena_ordinal = u32::try_from(arena_ordinal)?;
    let asset = plan.normal_plan().static_plan().asset();
    let totals = &mut inventory.dmo_plan;
    totals.arenas += 1;
    simulation_pointer_fnv_bytes(&mut totals.signature, b"A");
    simulation_pointer_fnv_u32(&mut totals.signature, collection_ordinal);
    simulation_pointer_fnv_u32(&mut totals.signature, arena_ordinal);
    simulation_pointer_fnv_u64(&mut totals.signature, asset.decoded.asset_id);
    simulation_pointer_fnv_u32(&mut totals.signature, asset.decoded.arena.id);

    if !plan.operations().is_empty() {
        totals.arenas_with_kind31 += 1;
        if inventory
            .dmo_plan_edges
            .collection_ordinals
            .insert(collection_ordinal)
        {
            totals.collections_with_kind31 += 1;
        }
    }

    for (operation_index, operation) in plan.operations().iter().enumerate() {
        let source = plan
            .source_handle(operation_index)
            .ok_or("Simulation DMO plan lost a whole-object source")?;
        add_simulation_dmo_operation(
            totals,
            &mut inventory.dmo_plan_edges,
            asset.decoded.asset_id,
            operation,
            source.bytes(),
        )?;
    }
    Ok(())
}

fn add_simulation_dmo_operation(
    totals: &mut SimulationDmoPlanInventory,
    edges: &mut SimulationDmoPlanEdges,
    arena_asset_id: u64,
    operation: &SimulationDmoLocatorOperation,
    source_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    totals.containers += 1;
    totals.records += usize::try_from(operation.locator_count)?;
    simulation_pointer_fnv_bytes(&mut totals.signature, b"O");
    simulation_pointer_fnv_u32(&mut totals.signature, operation.toc_dictionary_index);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_toc_index);
    simulation_pointer_fnv_u64(&mut totals.signature, operation.asset_id);
    simulation_pointer_fnv_u32(&mut totals.signature, DMO_DATA_TYPE_ID);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.raw_object_id);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_dictionary_index);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_resource_index);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_object_offset);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.source_dictionary_size);
    simulation_pointer_fnv_u64(&mut totals.signature, operation.source_span_start);
    simulation_pointer_fnv_u64(&mut totals.signature, operation.source_span_end);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.locator_count);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.header_type_id);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.string_count);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.dmos_offset);
    simulation_pointer_fnv_u32(&mut totals.signature, operation.string_list_offset);

    if source_bytes.len() != usize::try_from(operation.source_object_len)? {
        return Err("Simulation DMO source span length changed".into());
    }
    let parsed = DmoData::parse(source_bytes)?;
    if parsed.dmos.len() != usize::try_from(operation.locator_count)?
        || parsed.type_id != operation.header_type_id
        || parsed.string_count != operation.string_count
        || parsed.dmos_offset != operation.dmos_offset
        || parsed.string_list_offset != operation.string_list_offset
    {
        return Err("Simulation DMO parsed descriptor changed after planning".into());
    }

    if !edges.container_ids.insert(operation.asset_id) {
        return Err("duplicate kind-31 container GUID in BAM Simulation".into());
    }
    if !edges.source_spans.insert((
        arena_asset_id,
        operation.source_resource_index,
        operation.source_span_start,
        operation.source_span_end,
    )) {
        edges.source_span_aliases += 1;
    }
    let span = source_bytes.len();
    edges.minimum_span = Some(edges.minimum_span.map_or(span, |value| value.min(span)));
    edges.maximum_span = Some(edges.maximum_span.map_or(span, |value| value.max(span)));
    let record_count = parsed.dmos.len();
    edges.minimum_record_count = Some(
        edges
            .minimum_record_count
            .map_or(record_count, |value| value.min(record_count)),
    );
    edges.maximum_record_count = Some(
        edges
            .maximum_record_count
            .map_or(record_count, |value| value.max(record_count)),
    );
    edges.one_record_containers += usize::from(record_count == 1);
    *edges.header_types.entry(parsed.type_id).or_default() += 1;

    for (record_index, record) in parsed.dmos.iter().enumerate() {
        let relative_offset = operation
            .dmos_offset
            .checked_add(
                u32::try_from(record_index)?
                    .checked_mul(128)
                    .ok_or("Simulation DMO inline record offset multiplication overflowed")?,
            )
            .ok_or("Simulation DMO inline record offset overflowed")?;
        simulation_pointer_fnv_bytes(&mut totals.signature, b"R");
        simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(record_index)?);
        simulation_pointer_fnv_u32(&mut totals.signature, relative_offset);
        simulation_pointer_fnv_u64(&mut totals.signature, record.guid);
        simulation_pointer_fnv_u64(&mut totals.signature, record.local_guid);
        simulation_pointer_fnv_u64(&mut totals.signature, record.instance_guid);
        simulation_pointer_fnv_u32(&mut totals.signature, record.name_offset);

        if !edges.record_ids.insert(record.guid) {
            return Err("duplicate kind-31 record GUID in BAM Simulation".into());
        }
        *edges.local_id_counts.entry(record.local_guid).or_default() += 1;
        *edges
            .instance_id_counts
            .entry(record.instance_guid)
            .or_default() += 1;
        if !edges
            .identity_triples
            .insert((record.guid, record.local_guid, record.instance_guid))
        {
            return Err("duplicate kind-31 record identity triple in BAM Simulation".into());
        }
        edges.zero_guid += usize::from(record.guid == 0);
        edges.zero_local_guid += usize::from(record.local_guid == 0);
        edges.zero_instance_guid += usize::from(record.instance_guid == 0);
        edges.nonzero_serialized_words += usize::from(record.serialized_instance_word != 0);
        edges.empty_names += usize::from(record.name.is_empty());
    }
    Ok(())
}

fn finish_simulation_dmo_plan_signature(
    totals: &mut SimulationDmoPlanInventory,
) -> Result<(), Box<dyn std::error::Error>> {
    simulation_pointer_fnv_bytes(&mut totals.signature, b"Z");
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(totals.collections)?);
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(totals.arenas)?);
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(totals.containers)?);
    simulation_pointer_fnv_u32(&mut totals.signature, u32::try_from(totals.records)?);
    Ok(())
}

fn validate_simulation_dmo_plan(
    inventory: &SimulationInventory,
) -> Result<(), Box<dyn std::error::Error>> {
    let totals = inventory.dmo_plan;
    let edges = &inventory.dmo_plan_edges;
    let local_duplicates = repeated_key_stats(&edges.local_id_counts);
    let instance_duplicates = repeated_key_stats(&edges.instance_id_counts);
    if totals != EXPECTED_SIMULATION_DMO_PLAN
        || inventory.dmo_data != totals.containers
        || inventory.dmos != totals.records
        || edges.collection_ordinals.len() != 99
        || edges.container_ids.len() != 815
        || edges.record_ids.len() != 3_591
        || edges.local_id_counts.len() != 3_579
        || local_duplicates != (2, 12, 12)
        || edges.instance_id_counts.len() != 99
        || instance_duplicates != (88, 3_492, 227)
        || edges.identity_triples.len() != 3_591
        || edges.source_spans.len() != 815
        || edges.source_span_aliases != 0
        || edges.one_record_containers != 134
        || edges.minimum_record_count != Some(1)
        || edges.maximum_record_count != Some(21)
        || edges.minimum_span != Some(382)
        || edges.maximum_span != Some(7_871)
        || edges.zero_guid != 0
        || edges.zero_local_guid != 0
        || edges.zero_instance_guid != 0
        || edges.nonzero_serialized_words != 3_591
        || edges.empty_names != 0
        || edges.header_types != BTreeMap::from([(0, 815)])
    {
        return Err(format!(
            "Simulation kind-31 plan corpus mismatch: {totals:?}, edges {edges:?}, local duplicates {local_duplicates:?}, instance duplicates {instance_duplicates:?}"
        )
        .into());
    }
    Ok(())
}

fn repeated_key_stats(counts: &BTreeMap<u64, usize>) -> (usize, usize, usize) {
    counts
        .values()
        .fold((0, 0, 1), |(keys, extras, maximum), &count| {
            (
                keys + usize::from(count > 1),
                extras + count.saturating_sub(1),
                maximum.max(count),
            )
        })
}

fn add_simulation_pointer_collection(
    totals: &mut SimulationPointerPlanInventory,
    collection_name: &str,
) {
    totals.collections += 1;
    for signature in [&mut totals.scope_signature, &mut totals.unified_signature] {
        simulation_pointer_fnv_bytes(signature, b"C");
        simulation_pointer_fnv_bytes(signature, collection_name.as_bytes());
        simulation_pointer_fnv_bytes(signature, &[0]);
    }
}

fn add_simulation_registry_runtime_collection(
    totals: &mut SimulationRegistryRuntimeInventory,
    collection_name: &str,
) {
    totals.collections += 1;
    for signature in [&mut totals.reg_signature, &mut totals.dreg_signature] {
        simulation_pointer_fnv_bytes(signature, b"C");
        simulation_pointer_fnv_bytes(signature, collection_name.as_bytes());
        simulation_pointer_fnv_bytes(signature, &[0]);
    }
}

fn hash_simulation_registry_runtime_arena(signature: &mut u64, asset_id: u64, arena_id: u32) {
    simulation_pointer_fnv_bytes(signature, b"A");
    simulation_pointer_fnv_u64(signature, asset_id);
    simulation_pointer_fnv_u32(signature, arena_id);
}

fn hash_simulation_registry_runtime_event(
    signature: &mut u64,
    event: SimulationRegistryCorpusEvent,
) {
    simulation_pointer_fnv_bytes(signature, b"O");
    simulation_pointer_fnv_u32(signature, event.kind as u32);
    simulation_pointer_fnv_u64(signature, event.asset_id);
    simulation_pointer_fnv_bytes(signature, &[event.waypoint_streamed.map_or(0xff, u8::from)]);
}

fn expected_simulation_registry_event(
    operation: &skate2_core::simulation_registry_arena_adapter::PreparedSimulationRegistryOperation,
    registration: bool,
) -> SimulationRegistryCorpusEvent {
    SimulationRegistryCorpusEvent {
        registration,
        kind: operation.kind(),
        asset_id: operation.asset_id(),
        waypoint_streamed: (operation.kind() == SimulationRegistryAssetKind::Waypoint)
            .then_some(registration),
    }
}

fn exercise_simulation_registry_runtime(
    totals: &mut SimulationRegistryRuntimeInventory,
    adapter: &mut SimulationRegistryArenaRegistrationAdapter<SimulationRegistryCorpusDispatch>,
    prepared: &PreparedSimulationRegistryArena,
) -> Result<(), Box<dyn std::error::Error>> {
    let asset = prepared.plan().static_plan().asset();
    totals.arenas += 1;
    totals.operations += prepared.operation_count();
    for operation in prepared.operations() {
        match operation.kind() {
            SimulationRegistryAssetKind::WorldPainter => totals.world_painter += 1,
            SimulationRegistryAssetKind::NavMesh => totals.nav_mesh += 1,
            SimulationRegistryAssetKind::Rain => totals.rain += 1,
            SimulationRegistryAssetKind::AiPath => totals.ai_path += 1,
            SimulationRegistryAssetKind::Waypoint => totals.waypoint += 1,
            SimulationRegistryAssetKind::SpatialMap => totals.spatial_map += 1,
        }
    }
    hash_simulation_registry_runtime_arena(
        &mut totals.reg_signature,
        asset.decoded.asset_id,
        asset.decoded.arena.id,
    );
    hash_simulation_registry_runtime_arena(
        &mut totals.dreg_signature,
        asset.decoded.asset_id,
        asset.decoded.arena.id,
    );

    let registration_start = adapter.dispatch().events.len();
    let registration = adapter.register(prepared)?;
    let registration_events = &adapter.dispatch().events[registration_start..];
    totals.registrations += registration_events.len();
    if registration_events.len() != prepared.operation_count() {
        totals.sequence_mismatches += registration_events
            .len()
            .abs_diff(prepared.operation_count());
    }
    for (index, event) in registration_events.iter().copied().enumerate() {
        if prepared
            .operations()
            .get(index)
            .is_none_or(|operation| event != expected_simulation_registry_event(operation, true))
        {
            totals.sequence_mismatches += 1;
        }
        if event.kind == SimulationRegistryAssetKind::Waypoint
            && event.waypoint_streamed != Some(true)
        {
            totals.waypoint_polarity_mismatches += 1;
        }
        hash_simulation_registry_runtime_event(&mut totals.reg_signature, event);
    }

    let deregistration_start = adapter.dispatch().events.len();
    adapter.deregister(registration)?;
    let deregistration_events = &adapter.dispatch().events[deregistration_start..];
    totals.deregistrations += deregistration_events.len();
    if deregistration_events.len() != prepared.operation_count() {
        totals.sequence_mismatches += deregistration_events
            .len()
            .abs_diff(prepared.operation_count());
    }
    for (index, event) in deregistration_events.iter().copied().enumerate() {
        if prepared
            .operations()
            .get(index)
            .is_none_or(|operation| event != expected_simulation_registry_event(operation, false))
        {
            totals.sequence_mismatches += 1;
        }
        if event.kind == SimulationRegistryAssetKind::Waypoint
            && event.waypoint_streamed != Some(false)
        {
            totals.waypoint_polarity_mismatches += 1;
        }
        hash_simulation_registry_runtime_event(&mut totals.dreg_signature, event);
    }
    Ok(())
}

fn add_simulation_pointer_raw_container(
    inventory: &mut SimulationInventory,
    asset_id: u64,
    dictionary_index: u32,
    raw_type: u32,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let count = if raw_type == INSTANCE_DATA_TYPE_ID {
        let records = InstanceData::record_table(bytes)?;
        if records.type_id() != INSTANCE_DMO_ASSET_TYPE_ID {
            return Ok(());
        }
        inventory.pointer_plan.raw_dmo_containers += 1;
        inventory.pointer_plan.raw_dmo_instances += records.len();
        records.len()
    } else if raw_type == TRIGGER_INSTANCE_DATA_TYPE_ID {
        let records = TriggerInstanceData::record_table(bytes)?;
        inventory.pointer_plan.raw_trigger_containers += 1;
        inventory.pointer_plan.raw_trigger_instances += records.len();
        records.len()
    } else {
        return Ok(());
    };
    if inventory
        .pointer_raw_containers
        .insert((asset_id, raw_type, dictionary_index), count)
        .is_some()
    {
        return Err("duplicate raw pointer-container identity".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn hash_simulation_pointer_operation(
    signature: &mut u64,
    kind: u32,
    dispatch_id: u64,
    primary_id: u64,
    local_id: u64,
    toc_index: u32,
    dictionary_index: u32,
    inline_index: u32,
    inline_offset: u32,
    raw_type: u32,
    raw_id: u32,
) {
    simulation_pointer_fnv_bytes(signature, b"O");
    simulation_pointer_fnv_u32(signature, kind);
    simulation_pointer_fnv_u64(signature, dispatch_id);
    simulation_pointer_fnv_u64(signature, primary_id);
    simulation_pointer_fnv_u64(signature, local_id);
    simulation_pointer_fnv_u32(signature, toc_index);
    simulation_pointer_fnv_u32(signature, dictionary_index);
    simulation_pointer_fnv_u32(signature, inline_index);
    simulation_pointer_fnv_u32(signature, inline_offset);
    simulation_pointer_fnv_u32(signature, raw_type);
    simulation_pointer_fnv_u32(signature, raw_id);
}

#[allow(clippy::too_many_lines)] // Keep the frozen pointer descriptor gate auditable together.
fn add_simulation_pointer_plan(
    inventory: &mut SimulationInventory,
    plan: &SimulationNormalArenaPlan,
) -> Result<(), Box<dyn std::error::Error>> {
    let totals = &mut inventory.pointer_plan;
    let asset = plan.static_plan().asset();
    let arena = &asset.decoded.arena;
    let resources = &asset.decoded.resources.resources;
    totals.arenas += 1;
    for signature in [&mut totals.scope_signature, &mut totals.unified_signature] {
        simulation_pointer_fnv_bytes(signature, b"A");
        simulation_pointer_fnv_u64(signature, asset.decoded.asset_id);
        simulation_pointer_fnv_u32(signature, arena.id);
    }
    let toc = arena
        .dictionary
        .iter()
        .find(|entry| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
        .map(|entry| {
            arena
                .object_bytes(resources, entry)
                .ok_or("Simulation pointer-plan TOC lies outside retained storage")
                .and_then(|bytes| {
                    TableOfContents::parse(bytes)
                        .map_err(|_| "Simulation pointer-plan TOC is malformed")
                })
        })
        .transpose()?;
    totals.toc_entries += toc.as_ref().map_or(0, |toc| toc.entries.len());

    for (operation_index, operation) in plan.operations().iter().enumerate() {
        let descriptor = match *operation {
            SimulationNormalOperation::DmoInstance {
                asset_id,
                local_guid,
                source_toc_index,
                source_dictionary_index,
                source_inline_index,
                source_record_offset,
                raw_object_id,
            } => {
                totals.dmo_instances += 1;
                (
                    11,
                    asset_id,
                    asset_id,
                    local_guid,
                    source_toc_index,
                    source_dictionary_index,
                    source_inline_index,
                    source_record_offset,
                    INSTANCE_DATA_TYPE_ID,
                    raw_object_id,
                    SimulationRecordSourceKind::DmoInstance,
                )
            }
            SimulationNormalOperation::TriggerInstance {
                asset_id,
                primary_guid,
                source_toc_index,
                source_dictionary_index,
                source_inline_index,
                source_record_offset,
                raw_object_id,
            } => {
                totals.trigger_instances += 1;
                (
                    16,
                    asset_id,
                    primary_guid,
                    asset_id,
                    source_toc_index,
                    source_dictionary_index,
                    source_inline_index,
                    source_record_offset,
                    TRIGGER_INSTANCE_DATA_TYPE_ID,
                    raw_object_id,
                    SimulationRecordSourceKind::TriggerInstance,
                )
            }
            _ => continue,
        };
        let (
            kind,
            dispatch_id,
            primary_id,
            local_id,
            toc_index,
            dictionary_index,
            inline_index,
            inline_offset,
            raw_type,
            raw_id,
            expected_source_kind,
        ) = descriptor;
        let toc_entry = toc
            .as_ref()
            .and_then(|toc| toc.entries.get(toc_index as usize))
            .ok_or("pointer operation indexes a missing TOC entry")?;
        let dictionary = arena
            .dictionary
            .get(dictionary_index as usize)
            .ok_or("pointer operation indexes a missing dictionary entry")?;
        let source = plan
            .record_source(operation_index)
            .ok_or("pointer operation has no retained record source")?;
        let object_bytes = arena
            .object_bytes(resources, dictionary)
            .ok_or("pointer operation dictionary object is outside retained storage")?;
        let (derived_offset, inline_bytes, decoded_primary, decoded_local) = if kind == 11 {
            let records = InstanceData::record_table(object_bytes)?;
            if records.type_id() != INSTANCE_DMO_ASSET_TYPE_ID {
                return Err("kind-11 source is not an Instance_DMO container".into());
            }
            let record = records
                .record(inline_index as usize)
                .ok_or("kind-11 inline index is outside its record table")?;
            (
                record.object_offset(),
                record.bytes(),
                record.guid(),
                record.local_guid(),
            )
        } else {
            let records = TriggerInstanceData::record_table(object_bytes)?;
            let record = records
                .record(inline_index as usize)
                .ok_or("kind-16 inline index is outside its record table")?;
            (
                record.object_offset(),
                record.bytes(),
                record.guid(),
                record.local_guid(),
            )
        };
        if toc_entry.object_type != raw_type
            || toc_entry.object.0 != raw_id
            || toc_entry.object.section_index() != 0
            || toc_entry.object.entry_index() != dictionary_index
            || dictionary.type_id != raw_type
            || source.kind() != expected_source_kind
            || source.bytes() != inline_bytes
            || derived_offset != inline_offset
            || decoded_primary != primary_id
            || decoded_local != local_id
        {
            return Err("Simulation pointer operation disagrees with retained source".into());
        }
        *inventory
            .pointer_processed_containers
            .entry((asset.decoded.asset_id, raw_type, dictionary_index))
            .or_default() += 1;
        for signature in [
            &mut totals.operation_signature,
            &mut totals.unified_signature,
        ] {
            hash_simulation_pointer_operation(
                signature,
                kind,
                dispatch_id,
                primary_id,
                local_id,
                toc_index,
                dictionary_index,
                inline_index,
                inline_offset,
                raw_type,
                raw_id,
            );
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct SimulationPointerCorpusDispatch {
    dmo_model_adds: usize,
    dmo_registrations: usize,
    dmo_model_removes: usize,
    dmo_deregistrations: usize,
    trigger_adds: usize,
    trigger_removes: usize,
    source_identity_mismatches: usize,
    dmo_call_order_mismatches: usize,
    dmo_sequence_mismatches: usize,
    distinct_adjacent_trigger_pairs: usize,
    pending_dmo_model_call: Option<(bool, u64)>,
    pending_dmo_sources: Vec<(SimulationRecordHandle, u64)>,
    expected_dmo_ids: Vec<(u64, u64)>,
    expected_dmo_count: usize,
    next_dmo_registration: usize,
    next_dmo_remove: usize,
    pending_trigger_sources: Vec<SimulationRecordHandle>,
    next_trigger_remove: usize,
}

impl SimulationPointerCorpusDispatch {
    fn with_dmo_capacity(capacity: usize) -> Result<Self, Box<dyn Error>> {
        let mut dispatch = Self::default();
        dispatch.pending_dmo_sources.try_reserve_exact(capacity)?;
        Ok(dispatch)
    }

    fn with_expected_dmos(expected_dmo_ids: Vec<(u64, u64)>) -> Result<Self, Box<dyn Error>> {
        let mut dispatch = Self::with_dmo_capacity(expected_dmo_ids.len())?;
        dispatch.expected_dmo_count = expected_dmo_ids.len();
        dispatch.expected_dmo_ids = expected_dmo_ids;
        Ok(dispatch)
    }

    fn is_quiescent(&self) -> bool {
        self.pending_dmo_model_call.is_none()
            && self.pending_dmo_sources.is_empty()
            && self.expected_dmo_ids.is_empty()
            && self.next_dmo_remove == 0
            && self.pending_trigger_sources.is_empty()
            && self.next_trigger_remove == 0
    }
}

impl SimulationPointerArenaDispatch for SimulationPointerCorpusDispatch {
    fn add_dmo_model(&mut self, asset_id: u64) {
        self.dmo_model_adds += 1;
        if self
            .pending_dmo_model_call
            .replace((true, asset_id))
            .is_some()
        {
            self.dmo_call_order_mismatches += 1;
        }
    }

    fn register_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64) {
        let primary_id = u64::from_be_bytes(source.bytes()[96..104].try_into().unwrap());
        let local_id = u64::from_be_bytes(source.bytes()[104..112].try_into().unwrap());
        if self.pending_dmo_model_call.take() != Some((true, asset_id))
            || source.bytes().len() != 160
            || primary_id != asset_id
        {
            self.dmo_call_order_mismatches += 1;
        }
        if self.expected_dmo_ids.get(self.next_dmo_registration) != Some(&(primary_id, local_id)) {
            self.dmo_sequence_mismatches += 1;
        }
        self.next_dmo_registration += 1;
        self.pending_dmo_sources.push((source.clone(), asset_id));
        self.dmo_registrations += 1;
    }

    fn remove_dmo_model(&mut self, asset_id: u64) {
        self.dmo_model_removes += 1;
        if self
            .pending_dmo_model_call
            .replace((false, asset_id))
            .is_some()
        {
            self.dmo_call_order_mismatches += 1;
        }
    }

    fn deregister_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64) {
        if self.pending_dmo_model_call.take() != Some((false, asset_id)) {
            self.dmo_call_order_mismatches += 1;
        }
        if self
            .pending_dmo_sources
            .get(self.next_dmo_remove)
            .is_none_or(|(expected_source, expected_id)| {
                *expected_id != asset_id || !expected_source.is_same_backing_record(source)
            })
        {
            self.source_identity_mismatches += 1;
        }
        let primary_id = u64::from_be_bytes(source.bytes()[96..104].try_into().unwrap());
        let local_id = u64::from_be_bytes(source.bytes()[104..112].try_into().unwrap());
        if self.expected_dmo_ids.get(self.next_dmo_remove) != Some(&(primary_id, local_id)) {
            self.dmo_sequence_mismatches += 1;
        }
        self.next_dmo_remove += 1;
        self.dmo_deregistrations += 1;
        if self.next_dmo_remove == self.pending_dmo_sources.len() {
            self.pending_dmo_sources.clear();
            self.expected_dmo_ids.clear();
            self.next_dmo_remove = 0;
        }
    }

    fn add_trigger_volume(&mut self, source: &SimulationRecordHandle) {
        if self
            .pending_trigger_sources
            .last()
            .is_some_and(|previous| !previous.is_same_backing_record(source))
        {
            self.distinct_adjacent_trigger_pairs += 1;
        }
        self.pending_trigger_sources.push(source.clone());
        self.trigger_adds += 1;
    }

    fn remove_trigger_volume(&mut self, source: &SimulationRecordHandle) {
        if self
            .pending_trigger_sources
            .get(self.next_trigger_remove)
            .is_none_or(|expected| !expected.is_same_backing_record(source))
        {
            self.source_identity_mismatches += 1;
        }
        self.next_trigger_remove += 1;
        self.trigger_removes += 1;
        if self.next_trigger_remove == self.pending_trigger_sources.len() {
            self.pending_trigger_sources.clear();
            self.next_trigger_remove = 0;
        }
    }
}

#[allow(clippy::too_many_lines)] // One complete corpus traversal owns its cross-checks.
fn inventory_bam_sim(mounts: &Arc<MountSet>) -> Result<BTreeSet<u64>, Box<dyn std::error::Error>> {
    let base = "data\\content\\world\\stream\\BAM\\BAM_Sim";
    let assets = AssetToc::parse(&mounts.read_path(&format!("{base}.xst"))?)?;
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let mut inventory = SimulationInventory::default();
    let dmo_runtime = Arc::new(Mutex::new(SimulationDmoRuntimeState::default()));
    let mut pointer_runtime =
        SimulationPointerArenaRegistrationAdapter::new(SimulationPointerCorpusDispatch::default());
    let mut registry_runtime = SimulationRegistryArenaRegistrationAdapter::new(
        SimulationRegistryCorpusDispatch::default(),
    );
    let mut location_runtime =
        LocationArenaRegistrationAdapter::new(LocationCorpusRuntimeSource::default());
    let mut collection_runtime = SimulationCollectionCorpusRuntime::try_new()?;
    let mut notification_runtime = SimulationNotificationCorpusRuntime::try_new()?;
    let mut normal_runtime = SimulationNormalCorpusRuntime::try_new()?;
    for (collection_ordinal, collection) in collections.collections.iter().enumerate() {
        normal_runtime.add_collection(
            collection_ordinal,
            collection.id,
            &collection.filename,
            collection.asset_count,
        )?;
        add_location_plan_collection(
            &mut inventory.location_plan,
            collection_ordinal,
            collection.id,
            &collection.filename,
            collection.asset_count,
        )?;
        add_location_runtime_collection(
            &mut inventory.location_runtime,
            collection_ordinal,
            collection.id,
            &collection.filename,
            collection.asset_count,
        )?;
        add_simulation_pointer_collection(&mut inventory.pointer_plan, &collection.filename);
        add_simulation_registry_runtime_collection(
            &mut inventory.registry_runtime,
            &collection.filename,
        );
        add_simulation_dmo_collection(
            &mut inventory.dmo_plan,
            collection_ordinal,
            collection.id,
            &collection.filename,
        )?;
        let path = format!(
            "data\\content\\world\\stream\\BAM\\{}.xsf",
            collection.filename
        );
        let bytes = mounts.read_path(&path)?;
        inventory.collections += 1;
        inventory.xsf_bytes += u64::try_from(bytes.len())?;
        let file = StreamFile::parse(&bytes, collection.asset_count)?;
        inventory.asset_blocks += file.assets.len();
        let mut location_registrations = Vec::new();
        location_registrations.try_reserve_exact(file.assets.len())?;
        let mut collection_plans = Vec::new();
        collection_plans.try_reserve_exact(file.assets.len())?;
        for (arena_ordinal, block) in file.assets.iter().enumerate() {
            inventory
                .stream_assets_by_id
                .entry(block.id)
                .or_default()
                .insert(path.clone());
            let Some(metadata) = assets.assets.iter().find(|asset| asset.id == block.id) else {
                record_failure(&mut inventory, "missing ATOC asset");
                continue;
            };
            let Ok(resources) = decode_arena(&bytes[block.data_range.clone()]) else {
                record_failure(&mut inventory, "arena decode");
                continue;
            };
            let Ok(arena) = Rw4Arena::parse(&resources.resources) else {
                record_failure(&mut inventory, "RW4 arena");
                continue;
            };
            let Ok(sections) = arena.sections(&resources.resources) else {
                record_failure(&mut inventory, "RW4 arena sections");
                continue;
            };
            let subreference_kinds = classify_scene_subreferences(
                &arena,
                &sections,
                &resources.resources,
                &mut inventory,
            );
            inventory.rw4_arenas += 1;
            inventory.rw4_objects += arena.dictionary.len();
            inventory.arena_dictionaries.insert(
                arena.id,
                arena.dictionary.iter().map(|entry| entry.type_id).collect(),
            );
            for (dictionary_index, entry) in arena.dictionary.iter().enumerate() {
                *inventory.object_types.entry(entry.type_id).or_default() += 1;
                let Some(object) = arena.object_bytes(&resources.resources, entry) else {
                    record_failure(&mut inventory, "RW4 object range");
                    continue;
                };
                add_simulation_pointer_raw_container(
                    &mut inventory,
                    block.id,
                    u32::try_from(dictionary_index)?,
                    entry.type_id,
                    object,
                )?;
                inventory_sim_collision(&arena, &sections, entry.type_id, object, &mut inventory);
                inventory_sim_pegasus(
                    &arena,
                    &sections,
                    &subreference_kinds,
                    entry.type_id,
                    object,
                    &mut inventory,
                );
            }
            let collision_asset = SimulationCollisionAsset::from_decoded(DecodedArenaAsset {
                asset_id: block.id,
                asset_type: metadata.asset_type,
                asset_resource_descriptors: metadata.resources,
                resources,
                arena,
            })?;
            let plan = Arc::new(SimulationNormalArenaPlan::from_collision_asset(
                collision_asset,
            )?);
            collection_plans.push(Arc::clone(&plan));
            normal_runtime.exercise(collection_ordinal, arena_ordinal, &plan)?;
            let prepared_registry = PreparedSimulationRegistryArena::try_new(Arc::clone(&plan))?;
            exercise_simulation_registry_runtime(
                &mut inventory.registry_runtime,
                &mut registry_runtime,
                &prepared_registry,
            )?;
            drop(prepared_registry);
            let location_plan = Arc::new(LocationArenaPlan::from_normal_plan(Arc::clone(&plan))?);
            add_location_arena_plan(
                &mut inventory,
                collection_ordinal,
                arena_ordinal,
                &location_plan,
            )?;
            let prepared_location = PreparedLocationArena::try_new(Arc::clone(&location_plan))?;
            publish_location_runtime_sources(&mut location_runtime, &prepared_location)?;
            let location_registration = register_location_runtime_arena(
                &mut inventory.location_runtime,
                &mut location_runtime,
                collection_ordinal,
                arena_ordinal,
                &prepared_location,
            )?;
            let dmo_plan = Arc::new(SimulationDmoArenaPlan::from_normal_plan(Arc::clone(&plan))?);
            add_simulation_dmo_plan(&mut inventory, collection_ordinal, arena_ordinal, &dmo_plan)?;
            exercise_simulation_dmo_runtime(&dmo_runtime, &dmo_plan)?;
            drop(dmo_plan);
            drop(location_plan);
            add_simulation_pointer_plan(&mut inventory, &plan)?;
            let weak_plan = Arc::downgrade(&plan);
            let prepared = PreparedSimulationPointerArena::try_new(Arc::clone(&plan))?;
            let registration = pointer_runtime.register(&prepared, 8)?;
            pointer_runtime.deregister(registration, 8)?;
            drop(prepared);
            drop(plan);
            location_registrations.push((
                arena_ordinal,
                prepared_location,
                location_registration,
                weak_plan,
            ));
        }
        let notification_plans = collection_plans.clone();
        collection_runtime.exercise(
            collection_ordinal,
            collection.id,
            &collection.filename,
            collection.asset_count,
            collection.payload_size,
            collection_plans,
        )?;
        notification_runtime.exercise(
            collection_ordinal,
            collection.id,
            &collection.filename,
            collection.asset_count,
            collection.payload_size,
            notification_plans,
        )?;
        for (arena_ordinal, prepared, registration, weak_plan) in location_registrations {
            deregister_location_runtime_arena(
                &mut inventory.location_runtime,
                &mut location_runtime,
                collection_ordinal,
                arena_ordinal,
                &prepared,
                registration,
            )?;
            drop(prepared);
            if weak_plan.upgrade().is_some() {
                return Err("arena adapters retained a plan after matched deregistration".into());
            }
        }
        if !location_runtime.entries().is_empty() || location_runtime.live_registration_count() != 0
        {
            return Err("Location runtime did not finish its collection teardown".into());
        }
    }
    finish_location_plan(&mut inventory)?;
    finish_location_runtime(&mut inventory.location_runtime, &location_runtime)?;
    let collection_runtime_inventory = collection_runtime.finish()?;
    let notification_runtime_inventory = notification_runtime.finish()?;
    let normal_runtime_inventory = normal_runtime.finish()?;
    finish_simulation_dmo_plan_signature(&mut inventory.dmo_plan)?;
    validate_simulation_dmo_plan(&inventory)?;
    {
        let mut runtime = dmo_runtime.lock().expect("DMO runtime oracle poisoned");
        finish_simulation_dmo_runtime(&mut runtime)?;
        inventory.dmo_runtime = runtime.inventory;
    }
    resolve_external_links(&mut inventory);
    print_simulation_inventory(&inventory);
    println!(
        "Simulation pointer-plan corpus: {:?}",
        inventory.pointer_plan
    );
    println!(
        "Simulation ID-only runtime corpus: {:?}",
        inventory.registry_runtime
    );
    println!("Simulation unified normal runtime corpus: {normal_runtime_inventory:?}");
    println!("Simulation outer collection runtime corpus: {collection_runtime_inventory:?}");
    println!("Simulation notification router corpus: {notification_runtime_inventory:?}");
    println!(
        "Location kind-21 plan corpus: {:?}",
        inventory.location_plan
    );
    println!(
        "Location kind-21 runtime corpus: {:?}",
        inventory.location_runtime
    );
    println!("Simulation kind-31 plan corpus: {:?}", inventory.dmo_plan);
    println!(
        "Simulation kind-31 runtime corpus: {:?}",
        inventory.dmo_runtime
    );
    if inventory.pointer_plan != EXPECTED_SIMULATION_POINTER_PLAN {
        return Err(format!(
            "Simulation pointer-plan corpus mismatch: expected {:?}, got {:?}",
            EXPECTED_SIMULATION_POINTER_PLAN, inventory.pointer_plan
        )
        .into());
    }
    if inventory.pointer_raw_containers != inventory.pointer_processed_containers {
        return Err(format!(
            "Simulation raw/processed pointer containers disagree: raw {:?}, processed {:?}",
            inventory.pointer_raw_containers, inventory.pointer_processed_containers
        )
        .into());
    }
    if registry_runtime.live_registration_count() != 0
        || inventory.registry_runtime != EXPECTED_SIMULATION_REGISTRY_RUNTIME
    {
        return Err(format!(
            "Simulation ID-only runtime corpus mismatch: expected {:?}, got {:?}",
            EXPECTED_SIMULATION_REGISTRY_RUNTIME, inventory.registry_runtime
        )
        .into());
    }
    let pointer_dispatch = pointer_runtime.dispatch();
    if pointer_runtime.live_registration_count() != 0
        || pointer_runtime.orphaned_source_count() != 0
        || pointer_dispatch.dmo_model_adds != 0
        || pointer_dispatch.dmo_registrations != 0
        || pointer_dispatch.dmo_model_removes != 0
        || pointer_dispatch.dmo_deregistrations != 0
        || pointer_dispatch.trigger_adds != 10
        || pointer_dispatch.trigger_removes != 10
        || pointer_dispatch.source_identity_mismatches != 0
        || pointer_dispatch.dmo_call_order_mismatches != 0
        || pointer_dispatch.dmo_sequence_mismatches != 0
        || pointer_dispatch.next_dmo_registration != 0
        || pointer_dispatch.distinct_adjacent_trigger_pairs != 1
        || !pointer_dispatch.is_quiescent()
    {
        return Err(
            format!("Simulation pointer runtime corpus mismatch: {pointer_dispatch:?}").into(),
        );
    }
    println!("Simulation pointer runtime corpus: {pointer_dispatch:?}");
    let dmo_pointer_runtime = verify_dmo_bank_pointer_runtime(mounts)?;
    println!(
        "dedicated DMO pointer runtime: {:?}; normal kinds {:?}; asset ID signature {:016x}; {:?}",
        dmo_pointer_runtime.plan,
        dmo_pointer_runtime.normal_kind_counts,
        dmo_pointer_runtime.asset_id_signature,
        dmo_pointer_runtime.dispatch,
    );
    print_dmo_model_inventories(mounts, &inventory)?;
    Ok(inventory.dmo_instance_guids)
}

#[derive(Debug)]
struct DmoPointerRuntimeGate {
    plan: SimulationPointerPlanInventory,
    normal_kind_counts: BTreeMap<u32, usize>,
    asset_id_signature: u64,
    dispatch: SimulationPointerCorpusDispatch,
}

#[derive(Clone, Copy, Debug)]
struct DmoProbeRequestIdError;

impl std::fmt::Display for DmoProbeRequestIdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("dedicated DMO probe request ID is already live")
    }
}

impl Error for DmoProbeRequestIdError {}

struct DmoProbeRequestIdLease {
    live: Arc<AtomicBool>,
}

impl CollectionRequestIdLease for DmoProbeRequestIdLease {
    fn raw(&self) -> i32 {
        0
    }
}

impl Drop for DmoProbeRequestIdLease {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
    }
}

struct DmoProbeRequestIds {
    live: Arc<AtomicBool>,
}

impl CollectionRequestIdAllocator for DmoProbeRequestIds {
    type Lease = DmoProbeRequestIdLease;
    type Error = DmoProbeRequestIdError;

    fn acquire(&mut self) -> Result<Self::Lease, Self::Error> {
        self.live
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| DmoProbeRequestIdError)?;
        Ok(DmoProbeRequestIdLease {
            live: Arc::clone(&self.live),
        })
    }
}

#[derive(Clone)]
struct DmoProbeLoadedAssets {
    assets: Arc<Mutex<Vec<DmoBankProcessedArenaAsset>>>,
}

impl DmoProbeLoadedAssets {
    fn with_capacity(capacity: usize) -> Result<Self, Box<dyn Error>> {
        let mut assets = Vec::new();
        assets.try_reserve_exact(capacity)?;
        Ok(Self {
            assets: Arc::new(Mutex::new(assets)),
        })
    }
}

impl LoadedAssetCollection<DmoBankProcessedArenaAsset> for DmoProbeLoadedAssets {
    fn add_asset(&mut self, asset: DmoBankProcessedArenaAsset) {
        self.assets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(asset);
    }
}

type DmoManifestStream = ManifestOnlyStream<
    <ProbeInnerBackend as InnerStreamBackend>::Handle,
    <ProbeInnerBackend as InnerStreamBackend>::Error,
>;

struct MountedDmoManifestLoadSession {
    mounts: Arc<MountSet>,
    catalog: WorldStreamCatalog,
    backend: ProbeInnerBackend,
    manifest: Option<DmoManifestStream>,
    collection_claimed: bool,
}

impl MountedDmoManifestLoadSession {
    fn new(mounts: Arc<MountSet>) -> Result<Self, Box<dyn Error>> {
        let catalog = WorldStreamCatalog::new();
        let backend = MountedStreamBackend::new(Arc::clone(&mounts), catalog.clone())?;
        Ok(Self {
            mounts,
            catalog,
            backend,
            manifest: None,
            collection_claimed: false,
        })
    }

    fn manifest(&self) -> Result<&DmoManifestStream, Box<dyn Error>> {
        self.manifest
            .as_ref()
            .ok_or_else(|| "DMO manifest load was not started".into())
    }

    fn manifest_mut(&mut self) -> Result<&mut DmoManifestStream, Box<dyn Error>> {
        self.manifest
            .as_mut()
            .ok_or_else(|| "DMO manifest load was not started".into())
    }
}

impl dmo_hull_resident_probe::DmoManifestLoadSession for MountedDmoManifestLoadSession {
    fn catalog(&self) -> &WorldStreamCatalog {
        &self.catalog
    }

    fn begin_load(&mut self) -> Result<(), Box<dyn Error>> {
        if self.manifest.is_some() {
            return Err("DMO manifest load was requested more than once".into());
        }
        self.manifest = Some(ManifestOnlyStream::load(
            "data\\content\\world\\dmo",
            "DMO",
            StreamKind::Presentation,
        )?);
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.manifest
            .as_ref()
            .is_some_and(|manifest| manifest.state() == GameStreamState::LOADED)
    }

    fn update_load(&mut self) -> Result<(), Box<dyn Error>> {
        let manifest = self
            .manifest
            .as_mut()
            .ok_or("DMO manifest load was not started")?;
        manifest.poll(&mut self.backend)?;
        if let Some(stage) = manifest.reference_read_failure() {
            return Err(format!("DMO manifest read stalled at {stage:?}").into());
        }
        Ok(())
    }

    fn loaded_collection(&mut self) -> Result<CollectionRegistrationSnapshot, Box<dyn Error>> {
        if !self.is_loaded() {
            return Err("DMO manifest collection was requested before load completion".into());
        }
        if self.collection_claimed {
            return Err("DMO manifest collection was requested more than once".into());
        }
        let counts = self.catalog.counts()?;
        if counts.files != 1
            || counts.collections != 1
            || counts.asset_entries != 125
            || counts.asset_references != 125
            || counts.collection_spaces != 0
            || counts.stream_xml_files != 0
        {
            return Err(format!("DMO manifest catalog counts differ: {counts:?}").into());
        }
        let collection = self
            .catalog
            .collection(0xe878_78ef_d7f5_0339)?
            .ok_or("DMO manifest did not register cPres_Global")?;
        verify_dmo_processed_resident(&self.mounts, &self.catalog, &collection)?;
        self.collection_claimed = true;
        Ok(collection)
    }

    fn unload(&mut self) -> Result<(), Box<dyn Error>> {
        self.manifest_mut()?
            .request_unload()
            .map_err(|error| format!("DMO manifest unload request failed: {error:?}"))?;
        let deadline = Instant::now() + WORLD_STREAM_PROBE_TIMEOUT;
        while !self.manifest()?.is_released() {
            if Instant::now() >= deadline {
                return Err(format!(
                    "DMO manifest unload timed out in nested state {}",
                    self.manifest()?.state().raw()
                )
                .into());
            }
            let manifest = self
                .manifest
                .as_mut()
                .ok_or("DMO manifest load was not started")?;
            manifest.poll(&mut self.backend)?;
            thread::yield_now();
        }
        Ok(())
    }
}

fn verify_dmo_bank_catalog_session(mounts: &Arc<MountSet>) -> Result<(), Box<dyn Error>> {
    let mut subtitle_manager = subtitle_manager::MountedSubtitleManager::new(Arc::clone(mounts));
    subtitle_manager.load_big_subtitle_file(SECONDARY_LOAD_SUBTITLE_BIG_PATH)?;
    let subtitle_archives = subtitle_manager.archives();
    if subtitle_archives.archive_count() != 1
        || subtitle_archives.entry_count(0) != Some(288)
        || subtitle_archives.backing_len(0) != Some(46_984)
    {
        return Err(format!(
            "secondary subtitle BIG differed: {} archives, {:?} entries, {:?} bytes",
            subtitle_archives.archive_count(),
            subtitle_archives.entry_count(0),
            subtitle_archives.backing_len(0)
        )
        .into());
    }
    let mut session = MountedDmoManifestLoadSession::new(Arc::clone(mounts))?;
    dmo_hull_resident_probe::run(Arc::clone(mounts), &mut session)?;
    let counts = session.catalog.counts()?;
    if counts.files != 0
        || counts.collections != 0
        || counts.asset_entries != 0
        || counts.asset_references != 0
        || counts.collection_spaces != 0
        || counts.stream_xml_files != 0
    {
        return Err(format!("DMO manifest catalog did not unload cleanly: {counts:?}").into());
    }
    let global_locations = global_location_corpus::MountedGlobalLocationCorpus::load(mounts)?;
    let global_summary = global_locations.summary();
    if global_summary.files != 18
        || global_summary.containers != 18
        || global_summary.locations != 744
        || global_summary.sub_locations != 2_419
        || global_summary.normal_operations != 0
        || global_summary.registered_entries != 744
        || global_summary.live_registrations != 18
        || global_summary.allocated_assets != 18
        || global_locations.infos().len() != 18
        || global_locations.infos().values().any(|info| !info.loaded)
        || global_locations.registration_count() != 18
        || global_locations.adapter_entry_count() != 744
        || global_locations.activation_is_pending()
    {
        return Err(format!(
            "global Location corpus differed: {global_summary:?}, infos={}, registrations={}, adapter_entries={}",
            global_locations.infos().len(),
            global_locations.registration_count(),
            global_locations.adapter_entry_count()
        )
        .into());
    }
    let recipe_summary = recipe_mapping_corpus::load(mounts)?;
    if !recipe_summary.matches_shipped_default() {
        return Err(format!("RecipeMan shipped mapping differed: {recipe_summary:?}").into());
    }
    let behaviour_summary = behaviour_graph_corpus::load(mounts)?;
    if !behaviour_summary.matches_shipped() {
        return Err(
            format!("living-world BehaviourGraph corpus differed: {behaviour_summary:?}").into(),
        );
    }
    let state_graph_summary = state_graph_corpus::load(mounts)?;
    if !state_graph_summary.matches_shipped() {
        return Err(format!("compiled StateGraph corpus differed: {state_graph_summary:?}").into());
    }
    let state_runtime_summary = state_graph_runtime_corpus::load(mounts)?;
    if !state_runtime_summary.matches_shipped() {
        return Err(format!(
            "constructed StateGraph runtime corpus differed: {state_runtime_summary:?}"
        )
        .into());
    }
    let play_animation_summary = play_animation_corpus::load(mounts)?;
    if !play_animation_summary.matches_shipped() {
        return Err(format!(
            "constructed PlayAnimation stored-data corpus differed: {play_animation_summary:?}"
        )
        .into());
    }
    let mounted_motion_graph = motion_graph_typed_corpus::load(mounts, &play_animation_summary)?;
    let motion_graph_typed_summary = mounted_motion_graph.summary();
    if !motion_graph_typed_summary.matches_shipped() {
        return Err(format!(
            "typed MotionGraph construction corpus differed: {motion_graph_typed_summary:?}"
        )
        .into());
    }
    verify_retained_motion_graph(&mounted_motion_graph)?;
    subtitle_manager.purge();
    if subtitle_manager.archives().archive_count() != 0 {
        return Err("subtitle manager retained archives after explicit probe purge".into());
    }
    println!(
        "secondary load corpus: retained 288-entry private subtitle BIG; DMO manifest + resident/deferred path reached raw state 10 and unloaded cleanly; 18 global Location arenas registered 744 locations / 2419 sublocations with zero normal-Simulation operations; RecipeMan built exact 24-group / 54-entry shipped mapping and 15 / 0 / 10 queues from skater.vlt; parsed and constructed 10 compiled BehaviourGraph descriptors / 39 expressions / 41 conditions / 100 nodes / 5147 bytes; parsed, constructed, resolved, and structurally finalized 36 compiled StateGraphs / 2681 states / 2457 transitions / 3561 expressions / 8375 opaque registered leaves / 18 exact safe misses / 1017704 bytes; logical MotionGraph_OnBoard.xml selection reached its exact normalized compiled sibling only through forced Dynamic fallback, retained the constructed 1354-state arena, 815 HasIntent / 780 WillExpire / 807 PlayAnimation typed leaves plus 2902 registered opaque leaves, routed all 165 direct Params to PlayAnimation, and completed exact structural Process order; immutable preflight reported 1173 Conditions / 1662 Behaviours / 67 Hooks unsupported across 167 exact kind/name selector keys without invoking a controller or dynamic condition/lifecycle callback; PlayAnimation stored data retained exact source-order digests; no typed activation or frame tick claimed; clean subtitle purge"
    );
    Ok(())
}

fn verify_retained_motion_graph(
    mounted: &motion_graph_typed_corpus::MountedProcessedMotionGraph,
) -> Result<(), Box<dyn Error>> {
    if mounted.logical_request() != "state/MotionGraph_OnBoard.xml"
        || mounted.cache_key().normalized_path() != r"data\state\MotionGraph_OnBoard.xml"
        || mounted.cache_key().hash() != 0x07d5_e6bc
        || mounted.compiled_path() != r"data\state\MotionGraph_OnBoard.stategraph"
        || mounted.capability_report().leaf_count != 5_304
        || mounted.capability_report().diagnostics.len() != 2_902
    {
        return Err("retained MotionGraph route/refusal identity differed".into());
    }
    Ok(())
}

fn verify_dmo_processed_resident(
    mounts: &Arc<MountSet>,
    catalog: &WorldStreamCatalog,
    collection: &CollectionRegistrationSnapshot,
) -> Result<(), Box<dyn Error>> {
    let mut ordinary_sessions = SimulationCollisionAssetBuilderSessions::new(catalog.clone());
    if ordinary_sessions.begin_session(collection).unwrap_err()
        != SimulationCollisionAssetSessionError::WrongStreamKind(StreamKind::Presentation)
    {
        return Err("ordinary Simulation builder unexpectedly admitted the DMO bank".into());
    }
    drop(ordinary_sessions);

    let loaded_assets = DmoProbeLoadedAssets::with_capacity(125)?;
    let observer = loaded_assets.clone();
    let request_live = Arc::new(AtomicBool::new(false));
    let dmo_sessions = DmoBankProcessedArenaAssetBuilderSessions::new(catalog.clone());
    let mut factory = CatalogResidentCollectionLoadFactory::new(
        Arc::clone(mounts),
        catalog.clone(),
        DmoProbeRequestIds {
            live: Arc::clone(&request_live),
        },
        dmo_sessions,
    );
    let wake = factory.decompression_kick();
    let mut driver = factory.begin(
        CollectionId(collection.id),
        i32::try_from(collection.asset_count)?,
        collection.clone(),
        loaded_assets,
    )?;
    let deadline = Instant::now() + WORLD_STREAM_PROBE_TIMEOUT;
    while !driver.is_complete() {
        if Instant::now() >= deadline {
            return Err("dedicated DMO resident construction timed out".into());
        }
        let _ = wake.request();
        driver.update()?;
        thread::yield_now();
    }
    if driver.loaded_size() != 3_069_087 || driver.highest_stored_block() != 597_388 {
        return Err(format!(
            "dedicated DMO resident byte totals differ: {}/{}",
            driver.loaded_size(),
            driver.highest_stored_block()
        )
        .into());
    }
    let snapshot = driver
        .mounted_stream_snapshot()
        .ok_or("dedicated DMO resident driver has no mounted snapshot")?;
    if snapshot.queued_chunks != 0 || snapshot.retained_chunks != 0 {
        return Err(format!("dedicated DMO resident driver retained chunks: {snapshot:?}").into());
    }
    drop(driver);
    drop(factory);
    if request_live.load(Ordering::Acquire) {
        return Err("dedicated DMO request-ID lease survived its driver".into());
    }

    let file = catalog
        .file(&collection.owner)?
        .ok_or("DMO manifest owner disappeared during resident construction")?;
    let assets = observer
        .assets
        .lock()
        .map_err(|_| "dedicated DMO resident assets mutex was poisoned")?;
    if assets.len() != 125
        || assets.len() != file.asset_ids.len()
        || assets
            .iter()
            .zip(file.asset_ids.iter())
            .any(|(asset, id)| asset.plan().static_plan().asset().decoded.asset_id != *id)
    {
        return Err("dedicated DMO resident assets differ from physical XST order".into());
    }
    verify_dmo_processed_asset_shapes(&assets)?;
    drop(assets);
    drop(observer);
    Ok(())
}

fn verify_dmo_processed_asset_shapes(
    assets: &[DmoBankProcessedArenaAsset],
) -> Result<(), Box<dyn Error>> {
    let (operation_count, dmo_operation_count) = dmo_processed_operation_counts(assets);
    let mut serialised_assembly_count = 0_usize;
    let mut serialised_assembly_part_count = 0_usize;
    let mut serialised_assembly_joint_count = 0_usize;
    let mut serialised_assembly_drive_count = 0_usize;
    let mut serialised_assembly_joint_count_mismatches = 0_usize;
    let mut serialised_assembly_drive_count_mismatches = 0_usize;
    let mut serialised_assembly_volume_count = 0_usize;
    let mut serialised_assembly_update_volume_count = 0_usize;
    let mut serialised_assembly_create_update_selection_mismatches = 0_usize;
    let retained_asset_resource_descriptor_signature =
        dmo_retained_asset_resource_descriptor_signature(assets);
    let mut serialised_assembly_signature = SIMULATION_POINTER_FNV_OFFSET;
    let mut serialised_assembly_inertia_source_signature = SIMULATION_POINTER_FNV_OFFSET;
    let mut serialised_assembly_update_signature = SIMULATION_POINTER_FNV_OFFSET;
    simulation_pointer_fnv_bytes(
        &mut serialised_assembly_signature,
        b"SKATE2/DMO/SERIALISED_ASSEMBLY_SOURCE_CORPUS\0V2\0",
    );
    simulation_pointer_fnv_bytes(
        &mut serialised_assembly_inertia_source_signature,
        b"SKATE2/DMO/SERIALISED_ASSEMBLY_INERTIA_SOURCE_CORPUS\0V1\0",
    );
    simulation_pointer_fnv_bytes(
        &mut serialised_assembly_update_signature,
        b"SKATE2/DMO/SERIALISED_ASSEMBLY_UPDATE_SELECTION_CORPUS\0V1\0",
    );
    for asset in assets {
        let summary = asset.serialised_assembly_source_summary();
        serialised_assembly_count += summary.assembly_count;
        serialised_assembly_part_count += summary.part_count;
        serialised_assembly_joint_count += summary.joint_count;
        serialised_assembly_drive_count += summary.drive_count;
        serialised_assembly_joint_count_mismatches += summary.joint_count_mismatches;
        serialised_assembly_drive_count_mismatches += summary.drive_count_mismatches;
        serialised_assembly_volume_count += summary.volume_count;
        serialised_assembly_update_volume_count += summary.update_volume_count;
        serialised_assembly_create_update_selection_mismatches +=
            summary.create_update_selection_mismatches;
        simulation_pointer_fnv_u32(
            &mut serialised_assembly_signature,
            u32::try_from(summary.assembly_count)?,
        );
        simulation_pointer_fnv_u32(
            &mut serialised_assembly_signature,
            u32::try_from(summary.part_count)?,
        );
        simulation_pointer_fnv_u32(
            &mut serialised_assembly_signature,
            u32::try_from(summary.volume_count)?,
        );
        simulation_pointer_fnv_u64(&mut serialised_assembly_signature, summary.signature);
        simulation_pointer_fnv_u32(
            &mut serialised_assembly_inertia_source_signature,
            u32::try_from(summary.part_count)?,
        );
        simulation_pointer_fnv_u64(
            &mut serialised_assembly_inertia_source_signature,
            summary.inertia_source_signature,
        );
        simulation_pointer_fnv_u32(
            &mut serialised_assembly_update_signature,
            u32::try_from(summary.update_volume_count)?,
        );
        simulation_pointer_fnv_u64(
            &mut serialised_assembly_update_signature,
            summary.update_signature,
        );
    }
    if operation_count != 93
        || dmo_operation_count != 93
        || serialised_assembly_count != 93
        || serialised_assembly_part_count != 93
        || serialised_assembly_joint_count != 0
        || serialised_assembly_drive_count != 0
        || serialised_assembly_joint_count_mismatches != 0
        || serialised_assembly_drive_count_mismatches != 0
        || serialised_assembly_volume_count != 679
        || serialised_assembly_update_volume_count != 679
        || serialised_assembly_create_update_selection_mismatches != 0
        || retained_asset_resource_descriptor_signature != 0x14cc_c0e3_4801_1f23
        || serialised_assembly_signature != 0x2de1_9d33_48a1_226b
        || serialised_assembly_inertia_source_signature != 0xce61_e8a4_64f9_ccdd
        || serialised_assembly_update_signature != 0xeac3_e52c_1327_0389
    {
        return Err(format!(
            "dedicated DMO resident plan/Assembly counts differ: {operation_count}/{dmo_operation_count}/{serialised_assembly_count}/{serialised_assembly_part_count}/{serialised_assembly_joint_count}/{serialised_assembly_drive_count}/{serialised_assembly_volume_count}/{serialised_assembly_update_volume_count}, {serialised_assembly_joint_count_mismatches} Joint and {serialised_assembly_drive_count_mismatches} Drive count mismatches, {serialised_assembly_create_update_selection_mismatches} Create/Update occurrence mismatches, retained XST descriptor signature {retained_asset_resource_descriptor_signature:016x}, Create signature {serialised_assembly_signature:016x}, inertia-source signature {serialised_assembly_inertia_source_signature:016x}, Update signature {serialised_assembly_update_signature:016x}"
        )
        .into());
    }
    println!(
        "dedicated DMO resident Assembly source: {serialised_assembly_count} templates / {serialised_assembly_part_count} Parts / {serialised_assembly_joint_count} Joints / {serialised_assembly_drive_count} Drives / {serialised_assembly_joint_count_mismatches} Joint and {serialised_assembly_drive_count_mismatches} Drive count mismatches / {serialised_assembly_volume_count} Create-selected / {serialised_assembly_update_volume_count} Update-selected Volumes / {serialised_assembly_create_update_selection_mismatches} positional mismatches / retained XST descriptor signature {retained_asset_resource_descriptor_signature:016x} / Create signature {serialised_assembly_signature:016x} / inertia-source signature {serialised_assembly_inertia_source_signature:016x} / Update signature {serialised_assembly_update_signature:016x}"
    );
    Ok(())
}

fn dmo_retained_asset_resource_descriptor_signature(assets: &[DmoBankProcessedArenaAsset]) -> u64 {
    let mut signature = SIMULATION_POINTER_FNV_OFFSET;
    simulation_pointer_fnv_bytes(
        &mut signature,
        b"SKATE2/DMO/RETAINED_XST_RESOURCE_DESCRIPTORS\0V1\0",
    );
    for asset in assets {
        let decoded = &asset.plan().static_plan().asset().decoded;
        simulation_pointer_fnv_u64(&mut signature, decoded.asset_id);
        for descriptor in decoded.asset_resource_descriptors {
            simulation_pointer_fnv_u32(&mut signature, descriptor.size);
            simulation_pointer_fnv_u32(&mut signature, descriptor.alignment);
        }
    }
    signature
}

fn dmo_processed_operation_counts(assets: &[DmoBankProcessedArenaAsset]) -> (usize, usize) {
    let operations = assets.iter().flat_map(|asset| asset.plan().operations());
    let operation_count = operations.clone().count();
    let dmo_operation_count = operations
        .filter(|operation| matches!(operation, SimulationNormalOperation::DmoInstance { .. }))
        .count();
    (operation_count, dmo_operation_count)
}

#[allow(clippy::too_many_lines)] // Keep the strict bank provenance and runtime gate adjacent.
fn verify_dmo_bank_pointer_runtime(
    mounts: &Arc<MountSet>,
) -> Result<DmoPointerRuntimeGate, Box<dyn Error>> {
    const BASE: &str = "data\\content\\world\\dmo\\DMO\\DMO_Pres";
    const COLLECTION_ID: u64 = 0xe878_78ef_d7f5_0339;
    const PARENT_ID: u64 = 0xe171_e1ab_fedb_69ed;
    const STAMP: u64 = 0x0000_0000_94d1_4dc9;

    verify_dmo_bank_catalog_session(mounts)?;
    let assets = AssetToc::parse(&mounts.read_path(&format!("{BASE}.xst"))?)?;
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{BASE}.xsm"))?)?;
    let [collection] = collections.collections.as_slice() else {
        return Err("dedicated DMO bank does not contain exactly one collection".into());
    };
    if collections.version != 3
        || collections.stamp != STAMP
        || collections.stream_format != 2
        || assets.version != 3
        || assets.stamp != STAMP
        || collection.id != COLLECTION_ID
        || collection.parent_id != PARENT_ID
        || collection.payload_offset != 0x80
        || collection.payload_size != 0x2f_3180
        || collection.asset_count != 125
        || collection.filename != "cPres_Global"
        || assets.assets.len() != 125
    {
        return Err(
            "dedicated DMO bank manifest identity differs from the reference corpus".into(),
        );
    }
    let mut metadata_ids = BTreeSet::new();
    if !assets
        .assets
        .iter()
        .all(|asset| metadata_ids.insert(asset.id))
    {
        return Err("dedicated DMO XST contains duplicate asset IDs".into());
    }

    let path = "data\\content\\world\\dmo\\DMO\\cPres_Global.xsf";
    let bytes = mounts.read_path(path)?;
    let stream = StreamFile::parse(&bytes, collection.asset_count)?;
    if bytes.len() != 0x2f_3200
        || stream.version != 3
        || stream.stamp != STAMP
        || stream.header_size != collection.payload_offset
        || stream.assets.len() != 125
        || usize::try_from(collection.payload_offset)?
            .checked_add(usize::try_from(collection.payload_size)?)
            != Some(bytes.len())
    {
        return Err("dedicated DMO XSF identity differs from the reference corpus".into());
    }
    if assets
        .assets
        .iter()
        .zip(&stream.assets)
        .any(|(metadata, block)| metadata.id != block.id)
        || stream.assets.first().map(|block| block.id) != Some(0x91d7_2407_5ae4_4972)
        || stream.assets.last().map(|block| block.id) != Some(0xd309_6f13_cf1c_0aab)
    {
        return Err("dedicated DMO XST order differs from physical XSF block order".into());
    }
    let stride_bytes = stream.assets.iter().try_fold(0_u32, |sum, block| {
        sum.checked_add(block.stride)
            .ok_or("dedicated DMO XSF stride total overflowed")
    })?;
    if stride_bytes != collection.payload_size {
        return Err("dedicated DMO XSF strides do not cover its declared payload".into());
    }

    let mut inventory = SimulationInventory::default();
    add_simulation_pointer_collection(&mut inventory.pointer_plan, &collection.filename);
    let mut prepared = Vec::new();
    let mut plan_owners = Vec::new();
    prepared.try_reserve_exact(stream.assets.len())?;
    plan_owners.try_reserve_exact(stream.assets.len())?;
    let mut asset_id_signature = SIMULATION_POINTER_FNV_OFFSET;
    let mut stored_bytes = 0_u64;
    let mut maximum_stored = 0_u32;
    let mut serialized_bytes = 0_u64;
    let mut maximum_serialized = 0_u32;
    let mut resource_bytes = [0_u64; 5];
    let mut normal_kind_counts = BTreeMap::<u32, usize>::new();
    let mut expected_dmo_ids = Vec::new();
    expected_dmo_ids.try_reserve_exact(93)?;

    for (metadata, block) in assets.assets.iter().zip(&stream.assets) {
        let payload = bytes
            .get(block.data_range.clone())
            .ok_or("dedicated DMO block range lies outside its XSF")?;
        if metadata.asset_type != WORLD_STREAM_ARENA_ASSET_TYPE
            || payload.len() != usize::try_from(block.stored_size)?
        {
            return Err("dedicated DMO block metadata is not a streamed arena".into());
        }
        let expected_serialized = metadata.resources.iter().try_fold(0_u32, |sum, resource| {
            sum.checked_add(resource.size)
                .ok_or("dedicated DMO XST resource-size sum overflowed")
        })?;
        if metadata.serialized_size != expected_serialized {
            return Err("dedicated DMO XST serialized/resource size disagrees".into());
        }
        stored_bytes = stored_bytes
            .checked_add(u64::from(block.stored_size))
            .ok_or("dedicated DMO stored-byte total overflowed")?;
        maximum_stored = maximum_stored.max(block.stored_size);
        serialized_bytes = serialized_bytes
            .checked_add(u64::from(metadata.serialized_size))
            .ok_or("dedicated DMO serialized-byte total overflowed")?;
        maximum_serialized = maximum_serialized.max(metadata.serialized_size);
        let resources = decode_arena(payload)?;
        for (index, (resource, descriptor)) in resources
            .resources
            .iter()
            .zip(&metadata.resources)
            .enumerate()
        {
            if resource.len() != usize::try_from(descriptor.size)? {
                return Err(format!(
                    "dedicated DMO asset {:016x} resource {index} size disagrees",
                    block.id
                )
                .into());
            }
            resource_bytes[index] = resource_bytes[index]
                .checked_add(u64::from(descriptor.size))
                .ok_or("dedicated DMO resource-byte total overflowed")?;
        }
        let arena = Rw4Arena::parse(&resources.resources)?;
        for (dictionary_index, entry) in arena.dictionary.iter().enumerate() {
            let object = arena
                .object_bytes(&resources.resources, entry)
                .ok_or("dedicated DMO dictionary object lies outside retained storage")?;
            add_simulation_pointer_raw_container(
                &mut inventory,
                block.id,
                u32::try_from(dictionary_index)?,
                entry.type_id,
                object,
            )?;
        }
        simulation_pointer_fnv_u64(&mut asset_id_signature, block.id);
        let collision_asset = SimulationCollisionAsset::from_decoded(DecodedArenaAsset {
            asset_id: block.id,
            asset_type: metadata.asset_type,
            asset_resource_descriptors: metadata.resources,
            resources,
            arena,
        })?;
        let plan = Arc::new(SimulationNormalArenaPlan::from_collision_asset(
            collision_asset,
        )?);
        for operation in plan.operations() {
            *normal_kind_counts
                .entry(operation.simulation_kind())
                .or_default() += 1;
            if let SimulationNormalOperation::DmoInstance {
                asset_id,
                local_guid,
                ..
            } = *operation
            {
                expected_dmo_ids.push((asset_id, local_guid));
            }
        }
        add_simulation_pointer_plan(&mut inventory, &plan)?;
        plan_owners.push(Arc::downgrade(&plan));
        prepared.push(PreparedSimulationPointerArena::try_new(plan)?);
    }

    if stored_bytes != 3_069_087
        || maximum_stored != 597_388
        || serialized_bytes != 7_117_012
        || maximum_serialized != 1_382_292
        || resource_bytes != [945_452, 0, 6_171_560, 0, 0]
        || asset_id_signature != 0x9a86_1845_e82e_e292
        || inventory.pointer_plan != EXPECTED_DMO_POINTER_PLAN
        || normal_kind_counts != BTreeMap::from([(11, 93)])
        || inventory.pointer_raw_containers != inventory.pointer_processed_containers
    {
        return Err(format!(
            "dedicated DMO pointer-plan corpus mismatch: {:?}, kinds {normal_kind_counts:?}, stored {stored_bytes}/{maximum_stored}, serialized {serialized_bytes}/{maximum_serialized}, resources {resource_bytes:?}, ID signature {asset_id_signature:016x}",
            inventory.pointer_plan
        )
        .into());
    }
    let prepared_bank = PreparedSimulationPointerArenaBank::try_from_prepared(&prepared)?;
    if prepared_bank.arena_count() != 125 || prepared_bank.operation_count() != 93 {
        return Err(format!("dedicated DMO prepared bank differs: {prepared_bank:?}").into());
    }

    let mut disabled = SimulationPointerArenaRegistrationAdapter::new(
        SimulationPointerCorpusDispatch::with_dmo_capacity(0)?,
    );
    let disabled_registration = disabled.register_bank(&prepared_bank, 2)?;
    disabled.deregister_bank(disabled_registration, 2)?;
    let disabled_dispatch = disabled.dispatch();
    if disabled.live_registration_count() != 0
        || disabled.orphaned_source_count() != 0
        || disabled.live_bank_registration_count() != 0
        || disabled.orphaned_bank_count() != 0
        || disabled_dispatch.dmo_model_adds != 0
        || disabled_dispatch.dmo_registrations != 0
        || disabled_dispatch.dmo_model_removes != 0
        || disabled_dispatch.dmo_deregistrations != 0
        || disabled_dispatch.trigger_adds != 0
        || disabled_dispatch.trigger_removes != 0
        || !disabled_dispatch.is_quiescent()
    {
        return Err(format!(
            "disabled dedicated DMO pointer runtime produced effects: {disabled_dispatch:?}"
        )
        .into());
    }
    drop(disabled);

    let mut runtime = SimulationPointerArenaRegistrationAdapter::new(
        SimulationPointerCorpusDispatch::with_expected_dmos(expected_dmo_ids)?,
    );
    let registration = runtime.register_bank(&prepared_bank, 10)?;
    drop(prepared);
    drop(prepared_bank);
    if plan_owners.iter().any(|owner| owner.upgrade().is_none()) {
        return Err("dedicated DMO runtime released a live plan before deregistration".into());
    }
    runtime.deregister_bank(registration, 10)?;
    if runtime.live_registration_count() != 0
        || runtime.orphaned_source_count() != 0
        || runtime.live_bank_registration_count() != 0
        || runtime.orphaned_bank_count() != 0
        || plan_owners.iter().any(|owner| owner.upgrade().is_some())
    {
        return Err("dedicated DMO runtime retained a plan after matched teardown".into());
    }
    let dispatch = runtime.dispatch();
    if dispatch.dmo_model_adds != 93
        || dispatch.dmo_registrations != 93
        || dispatch.dmo_model_removes != 93
        || dispatch.dmo_deregistrations != 93
        || dispatch.trigger_adds != 0
        || dispatch.trigger_removes != 0
        || dispatch.source_identity_mismatches != 0
        || dispatch.dmo_call_order_mismatches != 0
        || dispatch.dmo_sequence_mismatches != 0
        || dispatch.expected_dmo_count != 93
        || dispatch.next_dmo_registration != dispatch.expected_dmo_count
        || dispatch.next_dmo_remove != 0
        || !dispatch.is_quiescent()
    {
        return Err(format!("dedicated DMO pointer runtime mismatch: {dispatch:?}").into());
    }

    Ok(DmoPointerRuntimeGate {
        plan: inventory.pointer_plan,
        normal_kind_counts,
        asset_id_signature,
        dispatch: runtime.dispatch().clone(),
    })
}

fn print_dmo_model_inventories(
    mounts: &MountSet,
    inventory: &SimulationInventory,
) -> Result<(), Box<dyn Error>> {
    let named_attribute_vaults = named_attribute_vaults(mounts);
    println!("mounted named attribute vaults: {named_attribute_vaults:?}");
    let model_inventory = inventory_living_world_model_arenas(
        mounts,
        &inventory.dmo_instance_guids,
        &inventory.dmo_model_local_guids,
        &inventory.dmo_model_names,
    );
    print_living_world_model_inventory(&model_inventory);
    let hashed = inventory_dmo_model_arenas(mounts);
    println!(
        "hashed model arenas: {} entries {:?} / {} standalone + {} streamed arenas / {} instances by discriminator {:?} / {} Instance_DMO / {} read failures",
        hashed.entries,
        hashed.entries_by_archive,
        hashed.standalone_arenas,
        hashed.compressed_arenas,
        hashed.instances,
        hashed.instance_data_types,
        hashed.dmo_instances,
        hashed.read_failures,
    );
    let dedicated = inventory_dmo_stream_model_arenas(mounts);
    let dedicated_ids = dedicated
        .dmo_models
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let resolved = dedicated_ids
        .intersection(&inventory.dmo_instance_guids)
        .count();
    let missing = inventory
        .dmo_instance_guids
        .difference(&dedicated_ids)
        .map(|id| {
            (
                format!("{id:016x}"),
                inventory
                    .dmo_model_names_by_id
                    .get(id)
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let unused = dedicated_ids
        .difference(&inventory.dmo_instance_guids)
        .map(|id| (format!("{id:016x}"), dedicated.dmo_models[id].clone()))
        .collect::<BTreeMap<_, _>>();
    validate_dmo_collision_template_corpus(&dedicated)?;
    println!(
        "dedicated DMO stream: {} assets {:?} / {} arenas / {} Instance_DMO records and {} unique sim-model IDs; {} embedded bodies / {} records / {} grind refs / {} exact resolutions with {} native primitives, splines per body {:?}, sections {:?}, subreference types {:?}, base types {:?}; collision {} refs by section {:?} to {} external arenas, {} local models / {} local geometry objects, {} templates / {} parts / {} primitives (volume types {:?}, {} triangles), {} retained clustered-mesh templates / {} authored parts / {} mesh leaves, {} assembly definitions / {} part payloads / signature {:016x}, assembly targets {:?}, ownership errors {:?}; object types {:?}; {resolved}/{} BAM locator model IDs resolve, {} missing {:?}, {} bank models unused {:?}; failures {:?} / {} read failures",
        dedicated.entries,
        dedicated.entries_by_archive,
        dedicated.compressed_arenas,
        dedicated.dmo_instances,
        dedicated.dmo_models.len(),
        dedicated.instances_with_embedded_data,
        dedicated.embedded_records,
        dedicated.grind_spline_references,
        dedicated.resolved_grind_splines,
        dedicated.native_grind_primitives,
        dedicated.grind_splines_per_body,
        dedicated.grind_reference_sections,
        dedicated.grind_subreference_types,
        dedicated.grind_base_types,
        dedicated.collision_model_references,
        dedicated.collision_model_reference_sections,
        dedicated.collision_model_external_arenas.len(),
        dedicated.local_collision_models,
        dedicated.local_collision_geometry_objects,
        dedicated.dmo_collision_parts.len(),
        dedicated.collision_template_parts,
        dedicated.collision_template_primitives,
        dedicated.collision_template_volume_types,
        dedicated.collision_template_triangles,
        dedicated.dmo_clustered_mesh_templates.len(),
        dedicated.clustered_mesh_template_parts,
        dedicated.clustered_mesh_template_leaves,
        dedicated.assembly_definitions,
        dedicated.assembly_part_definitions,
        dedicated.assembly_part_payload_signature,
        dedicated.assembly_target_types,
        dedicated.collision_ownership_errors,
        dedicated.object_types,
        inventory.dmo_instance_guids.len(),
        missing.len(),
        missing,
        unused.len(),
        unused,
        dedicated.failures,
        dedicated.read_failures,
    );
    print_dmo_attribute_inventory(mounts, &dedicated);
    Ok(())
}

fn named_attribute_vaults(mounts: &MountSet) -> BTreeSet<String> {
    mounts
        .archives()
        .iter()
        .flat_map(|archive| &archive.archive().entries)
        .filter_map(|entry| match &entry.key {
            BigEntryKey::Name(name) if name.to_ascii_lowercase().ends_with(".vlt") => {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

fn validate_dmo_collision_template_corpus(
    dedicated: &DmoModelArenaInventory,
) -> Result<(), Box<dyn Error>> {
    let retained_shape_matches = dedicated
        .dmo_clustered_mesh_templates
        .values()
        .all(|template| template.parts().len() == 1 && template.leaves().len() == 1);
    if dedicated.dmo_collision_parts.len() != 93
        || dedicated.collision_template_parts != 93
        || dedicated.collision_template_primitives != 14_451
        || dedicated.collision_template_triangles != 13_865
        || dedicated.dmo_clustered_mesh_templates.len() != 93
        || dedicated.clustered_mesh_template_parts != 93
        || dedicated.clustered_mesh_template_leaves != 93
        || dedicated.assembly_definitions != 93
        || dedicated.assembly_part_definitions != 93
        || dedicated.assembly_part_payload_signature != 0x6e38_d215_339b_f897
        || !retained_shape_matches
        || !dedicated.collision_ownership_errors.is_empty()
    {
        return Err(format!(
            "dedicated DMO collision-template corpus mismatch: flattened {}/{}/{} primitives/{} triangles, retained {}/{}/{} templates/parts/leaves, PartDefinitions {}/{} signature {:016x}, ownership errors {:?}",
            dedicated.dmo_collision_parts.len(),
            dedicated.collision_template_parts,
            dedicated.collision_template_primitives,
            dedicated.collision_template_triangles,
            dedicated.dmo_clustered_mesh_templates.len(),
            dedicated.clustered_mesh_template_parts,
            dedicated.clustered_mesh_template_leaves,
            dedicated.assembly_definitions,
            dedicated.assembly_part_definitions,
            dedicated.assembly_part_payload_signature,
            dedicated.collision_ownership_errors,
        )
        .into());
    }
    Ok(())
}

fn print_dmo_attribute_inventory(mounts: &MountSet, dedicated: &DmoModelArenaInventory) {
    let attribute_classes = dedicated.dmo_attribute_pairs.values().fold(
        BTreeMap::new(),
        |mut counts, &(class_id, _)| {
            *counts.entry(format!("{class_id:016x}")).or_insert(0_usize) += 1;
            counts
        },
    );
    let attribute_samples = dedicated
        .dmo_attribute_pairs
        .iter()
        .take(8)
        .map(|(model_id, &(class_id, collection_id))| {
            let name = dedicated.dmo_models[model_id].clone();
            (
                name.clone(),
                format!("{:016x}", string_hash64(&name)),
                format!("{class_id:016x}/{collection_id:016x}"),
            )
        })
        .collect::<Vec<_>>();
    let attribute_class_vaults = [
        "data\\db\\skater.vlt",
        "data\\db\\ChallengeCore.vlt",
        "data\\db\\progressioncore.vlt",
    ]
    .into_iter()
    .filter(|path| {
        mounts
            .read_path(path)
            .ok()
            .and_then(|bytes| AttributeVaultMetadata::parse(&bytes).ok())
            .is_some_and(|vault| {
                vault.exports.iter().any(|export| {
                    export.kind() == skate2_formats::attrib::AttributeExportKind::Class
                        && export.object_id == LIVING_WORLD_DMO_CHARACTERISTICS_CLASS_ID
                })
            })
    })
    .collect::<Vec<_>>();
    let priority_indices =
        dedicated
            .dmo_priority_indices
            .values()
            .fold(BTreeMap::new(), |mut counts, &priority| {
                *counts.entry(priority).or_insert(0_usize) += 1;
                counts
            });
    println!(
        "dedicated DMO serialized attribute classes {attribute_classes:?}; resolved characteristics class found in {attribute_class_vaults:?}; first model (name hash, serialized class/collection) pairs {attribute_samples:?}"
    );
    println!("dedicated DMO resolved priority indices {priority_indices:?}");
}

fn inventory_dmo_stream_model_arenas(mounts: &MountSet) -> DmoModelArenaInventory {
    let mut totals = DmoModelArenaInventory::default();
    for channel in ["Pres", "Sim", "Tex"] {
        let base = format!("data\\content\\world\\dmo\\DMO\\DMO_{channel}");
        let Some(collections) = mounts
            .read_path(&format!("{base}.xsm"))
            .ok()
            .and_then(|bytes| CollectionMap::parse(&bytes).ok())
        else {
            continue;
        };
        for collection in collections.collections {
            let path = format!(
                "data\\content\\world\\dmo\\DMO\\{}.xsf",
                collection.filename
            );
            let Ok(bytes) = mounts.read_path(&path) else {
                totals.read_failures += 1;
                continue;
            };
            let Ok(stream) = StreamFile::parse(&bytes, collection.asset_count) else {
                continue;
            };
            for block in stream.assets {
                totals.entries += 1;
                *totals
                    .entries_by_archive
                    .entry(channel.to_owned())
                    .or_default() += 1;
                let Ok(decoded) = decode_arena(&bytes[block.data_range]) else {
                    continue;
                };
                totals.compressed_arenas += 1;
                inventory_dmo_arena_resources(&decoded.resources, &mut totals);
            }
        }
    }
    match load_dmo_priority_indices(mounts, &totals.dmo_attribute_pairs) {
        Ok(priorities) => totals.dmo_priority_indices = priorities,
        Err(_) => {
            *totals
                .failures
                .entry("DMO priority attributes")
                .or_default() += 1;
        }
    }
    totals
}

fn inventory_dmo_arena_resources(resources: &[Vec<u8>; 5], totals: &mut DmoModelArenaInventory) {
    let Ok(arena) = Rw4Arena::parse(resources) else {
        return;
    };
    let Ok(sections) = arena.sections(resources) else {
        *totals.failures.entry("DMO arena sections").or_default() += 1;
        return;
    };
    let mut spline_data = BTreeMap::new();
    let mut embedded_data = BTreeMap::new();
    let mut assemblies = BTreeMap::new();
    let mut assembly_definitions = BTreeMap::new();
    let mut instance_data = Vec::new();
    let local_collision_models_before = totals.local_collision_models;
    for (index, object) in arena.dictionary.iter().enumerate() {
        let Some(bytes) = arena.object_bytes(resources, object) else {
            *totals.failures.entry("DMO object range").or_default() += 1;
            continue;
        };
        let Ok(index) = u32::try_from(index) else {
            *totals.failures.entry("DMO dictionary index").or_default() += 1;
            continue;
        };
        *totals.object_types.entry(object.type_id).or_default() += 1;
        if object.type_id == C_MODEL_DATA_TYPE_ID {
            totals.local_collision_models += 1;
        }
        if CollisionObjectType::from_type_id(object.type_id).is_some() {
            totals.local_collision_geometry_objects += 1;
        }
        match object.type_id {
            SPLINE_DATA_TYPE_ID => match SplineData::parse(bytes) {
                Ok(data) => {
                    spline_data.insert(index, data);
                }
                Err(_) => *totals.failures.entry("DMO spline data").or_default() += 1,
            },
            EMBEDDED_DATA_TYPE_ID => match EmbeddedData::parse(bytes) {
                Ok(data) => {
                    totals.embedded_records += 1;
                    embedded_data.insert(index, data);
                }
                Err(_) => *totals.failures.entry("DMO embedded data").or_default() += 1,
            },
            INSTANCE_DATA_TYPE_ID => match InstanceData::parse(bytes) {
                Ok(data) => instance_data.push(data),
                Err(_) => *totals.failures.entry("DMO instance data").or_default() += 1,
            },
            ASSEMBLY_TYPE_ID => match Assembly::parse(bytes) {
                Ok(assembly) => {
                    assemblies.insert(index, assembly);
                }
                Err(_) => *totals.failures.entry("DMO assembly").or_default() += 1,
            },
            ASSEMBLY_DEFINITION_TYPE_ID => match AssemblyDefinition::parse_in_resource(
                &resources[object.resource_index],
                object.object_offset,
            ) {
                Ok(definition) => {
                    record_dmo_assembly_definition(arena.id, index, &definition, totals);
                    assembly_definitions.insert(index, definition);
                }
                Err(_) => {
                    *totals
                        .failures
                        .entry("DMO assembly definition")
                        .or_default() += 1;
                }
            },
            _ => {}
        }
    }
    let collision_arena = own_dmo_collision_arena(
        &arena,
        resources,
        totals.local_collision_models > local_collision_models_before,
        totals,
    );
    let objects = DmoArenaInventoryObjects {
        arena: &arena,
        sections: &sections,
        spline_data: &spline_data,
        embedded_data: &embedded_data,
        assemblies: &assemblies,
        assembly_definitions: &assembly_definitions,
        collision_arena: collision_arena.as_ref(),
    };
    inventory_dmo_instances(&objects, instance_data, totals);
}

fn record_dmo_assembly_definition(
    arena_id: u32,
    dictionary_index: u32,
    definition: &AssemblyDefinition,
    totals: &mut DmoModelArenaInventory,
) {
    if totals.assembly_definitions == 0 {
        totals.assembly_part_payload_signature = SIMULATION_POINTER_FNV_OFFSET;
        simulation_pointer_fnv_bytes(
            &mut totals.assembly_part_payload_signature,
            b"SKATE2/DMO/PART_DEFINITION_PAYLOAD\0V1\0",
        );
    }
    totals.assembly_definitions += 1;
    totals.assembly_part_definitions += definition.parts.len();
    simulation_pointer_fnv_u32(&mut totals.assembly_part_payload_signature, arena_id);
    simulation_pointer_fnv_u32(
        &mut totals.assembly_part_payload_signature,
        dictionary_index,
    );
    simulation_pointer_fnv_u32(
        &mut totals.assembly_part_payload_signature,
        definition.part_count,
    );
    for part in &definition.parts {
        simulation_pointer_fnv_u32(&mut totals.assembly_part_payload_signature, part.volume.0);
        for value in part.inertia.inverse_tensor {
            simulation_pointer_fnv_u32(
                &mut totals.assembly_part_payload_signature,
                value.to_bits(),
            );
        }
        for value in [
            part.inertia.inverse_mass,
            part.inertia.spherical,
            part.inertia.max_velocity,
            part.inertia.max_omega,
            part.inertia.linear_drag,
            part.inertia.angular_drag,
        ] {
            simulation_pointer_fnv_u32(
                &mut totals.assembly_part_payload_signature,
                value.to_bits(),
            );
        }
        for value in part.inverse_body_transform {
            simulation_pointer_fnv_u32(
                &mut totals.assembly_part_payload_signature,
                value.to_bits(),
            );
        }
        for value in [
            part.material.static_friction,
            part.material.dynamic_friction,
            part.material.restitution,
        ] {
            simulation_pointer_fnv_u32(
                &mut totals.assembly_part_payload_signature,
                value.to_bits(),
            );
        }
        simulation_pointer_fnv_u32(
            &mut totals.assembly_part_payload_signature,
            part.has_inverse_body_transform,
        );
        simulation_pointer_fnv_u32(
            &mut totals.assembly_part_payload_signature,
            part.is_static_body,
        );
    }
}

struct DmoArenaInventoryObjects<'a> {
    arena: &'a Rw4Arena,
    sections: &'a ArenaSections,
    spline_data: &'a BTreeMap<u32, SplineData>,
    embedded_data: &'a BTreeMap<u32, EmbeddedData>,
    assemblies: &'a BTreeMap<u32, Assembly>,
    assembly_definitions: &'a BTreeMap<u32, AssemblyDefinition>,
    collision_arena: Option<&'a Arc<CollisionArena>>,
}

fn inventory_dmo_instances(
    objects: &DmoArenaInventoryObjects<'_>,
    instance_data: Vec<InstanceData>,
    totals: &mut DmoModelArenaInventory,
) {
    for data in instance_data {
        *totals.instance_data_types.entry(data.type_id).or_default() += data.instances.len();
        totals.instances += data.instances.len();
        totals.dmo_instances +=
            usize::from(data.type_id == INSTANCE_DMO_ASSET_TYPE_ID) * data.instances.len();
        for instance in data.instances {
            if data.type_id == INSTANCE_DMO_ASSET_TYPE_ID {
                totals.dmo_models.insert(
                    instance.local_guid,
                    model_name_from_serialized_instance(&instance.name)
                        .unwrap_or_else(|| instance.name.clone()),
                );
                totals.dmo_attribute_pairs.insert(
                    instance.local_guid,
                    (instance.class_key, instance.instance_key),
                );
                totals.dmo_model_bounds.insert(
                    instance.local_guid,
                    (
                        instance.bounding_box_min[..3]
                            .try_into()
                            .expect("three bounds"),
                        instance.bounding_box_max[..3]
                            .try_into()
                            .expect("three bounds"),
                    ),
                );
                inventory_dmo_collision_reference(
                    objects.sections,
                    instance.collision_model,
                    totals,
                );
                inventory_dmo_collision_template(objects, instance.local_guid, totals);
            }
            let Some(embedded_id) = instance.embedded_data else {
                continue;
            };
            totals.instances_with_embedded_data += 1;
            if embedded_id.section_index() != 0 {
                *totals.failures.entry("DMO embedded section").or_default() += 1;
                continue;
            }
            let Some(embedded) = objects.embedded_data.get(&embedded_id.entry_index()) else {
                *totals.failures.entry("DMO embedded target").or_default() += 1;
                continue;
            };
            totals.grind_spline_references += embedded.grind_splines().len();
            if !embedded.grind_splines().is_empty() {
                *totals
                    .grind_splines_per_body
                    .entry(embedded.grind_splines().len())
                    .or_default() += 1;
            }
            for reference in embedded.grind_splines() {
                inventory_dmo_grind_reference(
                    objects.arena,
                    objects.sections,
                    objects.spline_data,
                    instance.local_guid,
                    reference.object,
                    totals,
                );
            }
        }
    }
}

fn own_dmo_collision_arena(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    has_collision_models: bool,
    totals: &mut DmoModelArenaInventory,
) -> Option<Arc<CollisionArena>> {
    has_collision_models
        .then(|| own_collision_arena(arena, resources))
        .and_then(|result| {
            result
                .map(Arc::new)
                .map_err(|error| {
                    totals.collision_ownership_errors.insert(error.to_string());
                })
                .ok()
        })
}

fn inventory_dmo_collision_reference(
    sections: &ArenaSections,
    collision_model: Option<ObjectId>,
    totals: &mut DmoModelArenaInventory,
) {
    let Some(collision_model) = collision_model else {
        return;
    };
    totals.collision_model_references += 1;
    *totals
        .collision_model_reference_sections
        .entry(collision_model.section_index())
        .or_default() += 1;
    if let Some(arena_id) = sections.external_arena_id(collision_model.section_index()) {
        totals.collision_model_external_arenas.insert(arena_id);
    }
}

fn inventory_dmo_collision_template(
    objects: &DmoArenaInventoryObjects<'_>,
    model_id: u64,
    totals: &mut DmoModelArenaInventory,
) {
    let Some(arena_owner) = objects.collision_arena else {
        return;
    };
    let arena = arena_owner.as_ref();
    let model = match unique_dmo_collision_model(arena, model_id) {
        Ok(model) => model,
        Err(error) => {
            totals.collision_ownership_errors.insert(error);
            return;
        }
    };
    if let Some(assembly_id) = model.assembly_object_id.map(ObjectId)
        && assembly_id.section_index() == 0
        && let Some(entry) = usize::try_from(assembly_id.entry_index())
            .ok()
            .and_then(|index| objects.arena.dictionary.get(index))
    {
        *totals
            .assembly_target_types
            .entry(entry.type_id)
            .or_default() += 1;
    }
    let (root_indices, template_parts) =
        match dmo_collision_root_plan(objects, arena, model, model_id) {
            Ok(plan) => plan,
            Err(error) => {
                totals.collision_ownership_errors.insert(error);
                return;
            }
        };
    let mut parts = Vec::with_capacity(root_indices.len());
    for root_index in root_indices {
        let Some(root) = arena.aggregate_roots.get(root_index) else {
            return;
        };
        let Some(part) = dynamic_collision_part(arena, root) else {
            totals.collision_ownership_errors.insert(format!(
                "model {model_id:016x} has unsupported collision aggregate"
            ));
            return;
        };
        record_dmo_collision_part(&part, totals);
        parts.push(part);
    }
    totals.collision_template_parts += parts.len();
    totals.dmo_collision_parts.insert(model_id, parts);
    if let Some(template_parts) = template_parts {
        record_dmo_clustered_mesh_template(arena_owner, model_id, template_parts, totals);
    }
}

fn unique_dmo_collision_model(
    arena: &CollisionArena,
    model_id: u64,
) -> Result<&CollisionModel, String> {
    let mut matches = arena
        .instances
        .iter()
        .filter(|instance| instance.local_guid == model_id);
    let instance = matches
        .next()
        .ok_or_else(|| format!("model {model_id:016x} has no owned collision instance"))?;
    if matches.next().is_some() {
        return Err(format!(
            "model {model_id:016x} has ambiguous owned collision instances"
        ));
    }
    let model_index = instance.collision_model.ok_or_else(|| {
        format!("model {model_id:016x} collision instance has no collision model")
    })?;
    arena.collision_models.get(model_index).ok_or_else(|| {
        format!(
            "model {model_id:016x} collision instance references missing collision model {model_index}"
        )
    })
}

fn dmo_collision_root_plan(
    objects: &DmoArenaInventoryObjects<'_>,
    arena: &CollisionArena,
    model: &CollisionModel,
    model_id: u64,
) -> Result<(Vec<usize>, Option<Vec<DynamicClusteredMeshTemplatePart>>), String> {
    let Some(assembly_id) = model.assembly_object_id.map(ObjectId) else {
        // Preserve the pre-existing flattened diagnostic path for ordinary
        // collision models. It cannot supply native AssemblyPartDefinition
        // metadata, so it deliberately does not fabricate a dynamic template.
        return Ok((model.collision_roots.clone(), None));
    };
    if assembly_id.section_index() != 0 {
        return Err(format!(
            "model {model_id:016x} has external runtime assembly {:08x}",
            assembly_id.0
        ));
    }
    let assembly = objects
        .assemblies
        .get(&assembly_id.entry_index())
        .ok_or_else(|| format!("model {model_id:016x} has no runtime assembly"))?;
    if assembly.definition.section_index() != 0 {
        return Err(format!(
            "model {model_id:016x} has external assembly definition {:08x}",
            assembly.definition.0
        ));
    }
    let definition = objects
        .assembly_definitions
        .get(&assembly.definition.entry_index())
        .ok_or_else(|| {
            format!(
                "model {model_id:016x} has no assembly definition {:08x}",
                assembly.definition.0
            )
        })?;
    validate_dmo_assembly_part_counts(
        model_id,
        assembly.part_count,
        definition.part_count,
        definition.parts.len(),
    )?;
    let template_parts = dmo_clustered_mesh_template_parts(arena, model_id, &definition.parts)?;
    let root_indices = template_parts.iter().map(|part| part.root_index).collect();
    Ok((root_indices, Some(template_parts)))
}

fn record_dmo_clustered_mesh_template(
    arena: &Arc<CollisionArena>,
    model_id: u64,
    parts: Vec<DynamicClusteredMeshTemplatePart>,
    totals: &mut DmoModelArenaInventory,
) {
    match DynamicClusteredMeshTemplate::try_new(Arc::clone(arena), parts, false) {
        Ok(template) => {
            if totals.dmo_clustered_mesh_templates.contains_key(&model_id) {
                totals.collision_ownership_errors.insert(format!(
                    "model {model_id:016x} has duplicate clustered-mesh templates"
                ));
                return;
            }
            let part_count = template.parts().len();
            let leaf_count = template.leaves().len();
            totals
                .dmo_clustered_mesh_templates
                .insert(model_id, Arc::new(template));
            totals.clustered_mesh_template_parts += part_count;
            totals.clustered_mesh_template_leaves += leaf_count;
        }
        Err(error) => {
            totals.collision_ownership_errors.insert(format!(
                "model {model_id:016x} clustered-mesh template: {error}"
            ));
        }
    }
}

fn validate_dmo_assembly_part_counts(
    model_id: u64,
    assembly_count: u32,
    definition_count: u32,
    parsed_count: usize,
) -> Result<(), String> {
    let parsed_count = u32::try_from(parsed_count).map_err(|_| {
        format!("model {model_id:016x} parsed assembly part count does not fit u32")
    })?;
    if assembly_count != definition_count || definition_count != parsed_count {
        return Err(format!(
            "model {model_id:016x} assembly part counts differ: runtime {assembly_count}, definition {definition_count}, parsed {parsed_count}"
        ));
    }
    Ok(())
}

fn dmo_clustered_mesh_template_parts(
    arena: &CollisionArena,
    model_id: u64,
    parts: &[AssemblyPartDefinition],
) -> Result<Vec<DynamicClusteredMeshTemplatePart>, String> {
    parts
        .iter()
        .map(|part| {
            if part.volume.section_index() != 0 {
                return Err(format!(
                    "model {model_id:016x} has external part volume {:08x}",
                    part.volume.0
                ));
            }
            let mut matches = arena
                .aggregate_roots
                .iter()
                .enumerate()
                .filter(|(_, root)| root.dictionary_index == part.volume.entry_index());
            let Some((root_index, _)) = matches.next() else {
                return Err(format!(
                    "model {model_id:016x} has no part volume {:08x}",
                    part.volume.0
                ));
            };
            if matches.next().is_some() {
                return Err(format!(
                    "model {model_id:016x} has ambiguous part volume {:08x}",
                    part.volume.0
                ));
            }
            Ok(DynamicClusteredMeshTemplatePart {
                root_index,
                inverse_body_transform: part.inverse_body_transform,
                has_inverse_body_transform: part.has_inverse_body_transform,
                is_static_body: part.is_static_body,
            })
        })
        .collect()
}

fn record_dmo_collision_part(part: &DynamicCollisionPart, totals: &mut DmoModelArenaInventory) {
    for volume in &part.volumes {
        *totals
            .collision_template_volume_types
            .entry(volume.vtable_offset)
            .or_default() += 1;
    }
    totals.collision_template_triangles += part.triangles.len();
    totals.collision_template_primitives += part.volumes.len() + part.triangles.len();
}

fn dynamic_collision_part(
    arena: &CollisionArena,
    root: &CollisionRoot,
) -> Option<DynamicCollisionPart> {
    let mut volumes = Vec::new();
    let mut triangles = Vec::new();
    collect_dynamic_collision_aggregate(
        arena,
        root.aggregate?,
        identity_matrix(),
        &mut volumes,
        &mut triangles,
    )?;
    Some(DynamicCollisionPart {
        transform: root.volume.transform,
        volumes,
        triangles,
    })
}

fn collect_dynamic_collision_aggregate(
    arena: &CollisionArena,
    aggregate: CollisionAggregateRef,
    transform: [f32; 16],
    volumes: &mut Vec<Volume>,
    triangles: &mut Vec<skate2_formats::collision::MeshTriangle>,
) -> Option<()> {
    match aggregate {
        CollisionAggregateRef::PrimitiveSet(index) => {
            let set = arena.primitive_sets.get(index)?;
            collect_dynamic_collision_volumes(arena, &set.volumes, transform, volumes, triangles)
        }
        CollisionAggregateRef::SimplePrimitiveSet(index) => {
            let set = arena.simple_primitive_sets.get(index)?;
            collect_dynamic_collision_volumes(arena, &set.volumes, transform, volumes, triangles)
        }
        CollisionAggregateRef::Mesh(index) => {
            let mesh = arena.meshes.get(index)?;
            triangles.extend(mesh.triangles.iter().cloned().map(|mut triangle| {
                triangle.vertices = triangle
                    .vertices
                    .map(|point| transform_collision_point(point, transform));
                triangle
            }));
            Some(())
        }
    }
}

fn collect_dynamic_collision_volumes(
    arena: &CollisionArena,
    source: &[Volume],
    parent_transform: [f32; 16],
    volumes: &mut Vec<Volume>,
    triangles: &mut Vec<skate2_formats::collision::MeshTriangle>,
) -> Option<()> {
    for source_volume in source {
        let transform = compose_collision_affine(parent_transform, source_volume.transform);
        if let Some(target) = source_volume.aggregate_object_id().map(ObjectId) {
            collect_dynamic_collision_aggregate(
                arena,
                collision_aggregate_by_dictionary_index(arena, target.entry_index())?,
                transform,
                volumes,
                triangles,
            )?;
        } else {
            let mut volume = source_volume.clone();
            volume.transform = transform;
            volumes.push(volume);
        }
    }
    Some(())
}

fn collision_aggregate_by_dictionary_index(
    arena: &CollisionArena,
    dictionary_index: u32,
) -> Option<CollisionAggregateRef> {
    arena
        .primitive_sets
        .iter()
        .position(|set| set.dictionary_index == dictionary_index)
        .map(CollisionAggregateRef::PrimitiveSet)
        .or_else(|| {
            arena
                .simple_primitive_sets
                .iter()
                .position(|set| set.dictionary_index == dictionary_index)
                .map(CollisionAggregateRef::SimplePrimitiveSet)
        })
        .or_else(|| {
            arena
                .meshes
                .iter()
                .position(|mesh| mesh.dictionary_index == dictionary_index)
                .map(CollisionAggregateRef::Mesh)
        })
}

fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn transform_collision_point(point: [f32; 3], transform: [f32; 16]) -> [f32; 3] {
    std::array::from_fn(|component| {
        transform[8 + component].mul_add(
            point[2],
            transform[4 + component].mul_add(
                point[1],
                transform[component].mul_add(point[0], transform[12 + component]),
            ),
        )
    })
}

fn inventory_dmo_grind_reference(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    spline_data: &BTreeMap<u32, SplineData>,
    model_id: u64,
    reference: ObjectId,
    totals: &mut DmoModelArenaInventory,
) {
    *totals
        .grind_reference_sections
        .entry(reference.section_index())
        .or_default() += 1;
    if reference.section_index() != 2 {
        *totals.failures.entry("DMO grind section").or_default() += 1;
        return;
    }
    let Some(subreference) = usize::try_from(reference.entry_index())
        .ok()
        .and_then(|index| sections.subreferences.get(index))
    else {
        *totals.failures.entry("DMO grind subreference").or_default() += 1;
        return;
    };
    *totals
        .grind_subreference_types
        .entry(subreference.type_id)
        .or_default() += 1;
    if subreference.object.section_index() != 0 {
        *totals.failures.entry("DMO grind base section").or_default() += 1;
        return;
    }
    let Some(base_entry) = arena
        .dictionary
        .get(usize::try_from(subreference.object.entry_index()).unwrap_or(usize::MAX))
    else {
        *totals.failures.entry("DMO grind base").or_default() += 1;
        return;
    };
    *totals
        .grind_base_types
        .entry(base_entry.type_id)
        .or_default() += 1;
    let Some(splines) = spline_data.get(&subreference.object.entry_index()) else {
        *totals.failures.entry("DMO grind spline data").or_default() += 1;
        return;
    };
    let Some(header_index) = splines.spline_index_at_object_offset(subreference.object_offset)
    else {
        *totals.failures.entry("DMO grind offset").or_default() += 1;
        return;
    };
    totals.resolved_grind_splines += 1;
    let Some(asset) = GrindAsset::from_spline_header(
        model_id,
        subreference.object.entry_index(),
        splines,
        header_index,
    ) else {
        *totals.failures.entry("DMO native grind asset").or_default() += 1;
        return;
    };
    totals.native_grind_primitives += asset.primitives.len();
    totals
        .dmo_grind_assets
        .entry(model_id)
        .or_default()
        .push(asset);
}

fn inventory_dmo_model_arenas(mounts: &MountSet) -> DmoModelArenaInventory {
    let mut totals = DmoModelArenaInventory::default();
    for mounted in mounts.archives() {
        let archive_name = mounted
            .data_path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<archive>")
            .to_owned();
        for entry in &mounted.archive().entries {
            if !matches!(entry.key, BigEntryKey::Hash(_)) {
                continue;
            }
            totals.entries += 1;
            *totals
                .entries_by_archive
                .entry(archive_name.clone())
                .or_default() += 1;
            let Ok(bytes) = mounted.read_entry(entry) else {
                totals.read_failures += 1;
                continue;
            };
            let resources = if let Ok(resources) = Rw4Arena::split_file_resources(&bytes) {
                totals.standalone_arenas += 1;
                resources
            } else if let Ok(decoded) = decode_arena(&bytes) {
                totals.compressed_arenas += 1;
                decoded.resources
            } else {
                continue;
            };
            inventory_dmo_arena_resources(&resources, &mut totals);
        }
    }
    totals
}

fn inventory_living_world_model_arenas(
    mounts: &MountSet,
    dmo_model_ids: &BTreeSet<u64>,
    dmo_model_local_guids: &BTreeSet<u64>,
    dmo_model_names: &BTreeSet<String>,
) -> LivingWorldModelInventory {
    let mut totals = LivingWorldModelInventory::default();
    for toc_path in ["livingworld_model.toc", "vehicle_model.toc"] {
        match mounts
            .read_path(toc_path)
            .ok()
            .and_then(|bytes| AssetToc::parse(&bytes).ok())
        {
            Some(toc) => {
                totals.model_toc_assets += toc.assets.len();
                totals.dmo_model_toc_matches.extend(
                    toc.assets
                        .iter()
                        .map(|asset| asset.id)
                        .filter(|id| dmo_model_ids.contains(id)),
                );
            }
            None => *totals.failures.entry("living-world model TOC").or_default() += 1,
        }
    }
    let recipe_paths =
        index_living_world_recipe_paths(mounts, dmo_model_ids, dmo_model_local_guids, &mut totals);
    for mounted in mounts.archives() {
        let hash_archive_name = mounted
            .data_path()
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.eq_ignore_ascii_case("livingworld.big"));
        for entry in &mounted.archive().entries {
            if let BigEntryKey::Name(path) = &entry.key
                && dmo_model_ids
                    .iter()
                    .any(|id| path.contains(&format!("{id:016x}")))
            {
                totals.dmo_any_path_matches.insert(path.clone());
            }
            if let BigEntryKey::Name(path) = &entry.key {
                let lower = path.to_ascii_lowercase();
                if dmo_model_names
                    .iter()
                    .any(|name| lower.contains(&name.to_ascii_lowercase()))
                {
                    totals.dmo_model_name_path_matches.insert(path.clone());
                }
            }
            let label = match &entry.key {
                BigEntryKey::Name(path) if path.to_ascii_lowercase().ends_with(".rx2") => {
                    totals.rx2_files += 1;
                    if dmo_model_ids
                        .iter()
                        .any(|id| path.contains(&format!("{id:016x}")))
                    {
                        totals.dmo_model_path_matches.insert(path.clone());
                    }
                    path.clone()
                }
                BigEntryKey::Hash(hash) if hash_archive_name.is_some() => {
                    totals.hashed_model_entries += 1;
                    let archive = hash_archive_name.unwrap_or_default();
                    recipe_paths.get(hash).map_or_else(
                        || format!("{archive}::{hash:08x}"),
                        |path| format!("{archive}::{hash:08x} [{path}]"),
                    )
                }
                _ => continue,
            };
            let Ok(bytes) = mounted.read_entry(entry) else {
                *totals.failures.entry("RX2 read").or_default() += 1;
                continue;
            };
            inventory_model_arena(
                &bytes,
                &label,
                dmo_model_ids,
                dmo_model_local_guids,
                &mut totals,
            );
        }
    }
    totals
}

fn index_living_world_recipe_paths(
    mounts: &MountSet,
    dmo_model_ids: &BTreeSet<u64>,
    dmo_model_local_guids: &BTreeSet<u64>,
    totals: &mut LivingWorldModelInventory,
) -> BTreeMap<u32, String> {
    let mut request_paths = BTreeMap::new();
    let Some(mounted) = mounts.archives().iter().find(|mounted| {
        mounted
            .data_path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("livingworld.big"))
    }) else {
        return request_paths;
    };
    for entry in &mounted.archive().entries {
        let Ok(bytes) = mounted.read_entry(entry) else {
            continue;
        };
        let Ok(recipe) = CharacterRecipe::parse(&bytes) else {
            continue;
        };
        totals.character_recipes += 1;
        totals.recipe_models += recipe.model_count();
        totals.recipe_lods += recipe.lod_count();
        *totals
            .recipe_schemas
            .entry(recipe.schema_name.clone())
            .or_default() += 1;
        totals.recipe_dmo_id_matches.extend(
            recipe
                .referenced_asset_ids()
                .into_iter()
                .filter(|id| dmo_model_ids.contains(id) || dmo_model_local_guids.contains(id)),
        );
        let category = if recipe.category == 2 {
            "vehicle"
        } else {
            "livingworld"
        };
        for request in recipe.asset_requests(category) {
            totals.recipe_asset_requests += 1;
            let hash = djb2(request.entry_path.as_bytes());
            if mounted.archive().find_hash(hash).is_some() {
                totals.resolved_recipe_asset_paths += 1;
            }
            if let Some(previous) = request_paths.insert(hash, request.entry_path.clone())
                && previous != request.entry_path
            {
                *totals
                    .failures
                    .entry("recipe path hash collision")
                    .or_default() += 1;
            }
        }
    }
    totals.recipe_unique_asset_paths = request_paths.len();
    request_paths
}

fn inventory_model_arena(
    bytes: &[u8],
    label: &str,
    dmo_model_ids: &BTreeSet<u64>,
    dmo_model_local_guids: &BTreeSet<u64>,
    totals: &mut LivingWorldModelInventory,
) {
    let Ok(resources) = Rw4Arena::split_file_resources(bytes) else {
        inventory_non_arena_model(bytes, label, totals);
        return;
    };
    let Ok(arena) = Rw4Arena::parse(&resources) else {
        *totals.failures.entry("RX2 arena").or_default() += 1;
        return;
    };
    let Ok(sections) = arena.sections(&resources) else {
        *totals.failures.entry("RX2 sections").or_default() += 1;
        return;
    };
    totals.arenas += 1;

    let mut spline_data = BTreeMap::new();
    let mut embedded_data = BTreeMap::new();
    let mut instance_data = Vec::new();
    for (index, object) in arena.dictionary.iter().enumerate() {
        let Some(object_bytes) = arena.object_bytes(&resources, object) else {
            *totals.failures.entry("RX2 object range").or_default() += 1;
            continue;
        };
        let Ok(index) = u32::try_from(index) else {
            *totals.failures.entry("RX2 dictionary index").or_default() += 1;
            continue;
        };
        match object.type_id {
            SPLINE_DATA_TYPE_ID => match SplineData::parse(object_bytes) {
                Ok(data) => {
                    spline_data.insert(index, data);
                }
                Err(_) => *totals.failures.entry("RX2 spline data").or_default() += 1,
            },
            EMBEDDED_DATA_TYPE_ID => match EmbeddedData::parse(object_bytes) {
                Ok(data) => {
                    totals.embedded_records += 1;
                    embedded_data.insert(index, data);
                }
                Err(_) => *totals.failures.entry("RX2 embedded data").or_default() += 1,
            },
            INSTANCE_DATA_TYPE_ID => match InstanceData::parse(object_bytes) {
                Ok(data) => instance_data.push(data),
                Err(_) => *totals.failures.entry("RX2 instance data").or_default() += 1,
            },
            _ => {}
        }
    }

    for data in instance_data {
        totals.instances += data.instances.len();
        for instance in data.instances {
            let model_id_match = dmo_model_local_guids.contains(&instance.local_guid)
                || [instance.guid, instance.class_key, instance.instance_key]
                    .into_iter()
                    .any(|id| dmo_model_ids.contains(&id));
            totals.dmo_model_id_matches += usize::from(model_id_match);
            let Some(embedded_id) = instance.embedded_data else {
                continue;
            };
            totals.instances_with_embedded_data += 1;
            if embedded_id.section_index() != 0 {
                *totals
                    .failures
                    .entry("RX2 instance embedded section")
                    .or_default() += 1;
                continue;
            }
            let Some(embedded) = embedded_data.get(&embedded_id.entry_index()) else {
                *totals
                    .failures
                    .entry("RX2 instance embedded target")
                    .or_default() += 1;
                continue;
            };
            totals.grind_spline_references += embedded.grind_splines().len();
            if !embedded.grind_splines().is_empty() {
                if let Some(schema) = living_world_model_schema(label) {
                    totals.grind_model_schemas.insert(schema.to_owned());
                }
                *totals
                    .grind_splines_per_body
                    .entry(embedded.grind_splines().len())
                    .or_default() += 1;
            }
            for reference in embedded.grind_splines() {
                inventory_model_grind_reference(
                    &arena,
                    &sections,
                    &spline_data,
                    instance.local_guid,
                    reference.object,
                    totals,
                );
            }
        }
    }
}

fn inventory_non_arena_model(bytes: &[u8], label: &str, totals: &mut LivingWorldModelInventory) {
    if !label.contains(".big::") {
        *totals.failures.entry("RX2 resource split").or_default() += 1;
        return;
    }
    totals.non_arena_hashed_entries += 1;
    if CharacterRecipe::parse(bytes).is_ok() {
        return;
    }
    if bytes.starts_with(b"<?xml") {
        totals.xml_recipe_companions += 1;
    } else {
        *totals
            .failures
            .entry("unclassified hashed model entry")
            .or_default() += 1;
    }
}

fn inventory_model_grind_reference(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    spline_data: &BTreeMap<u32, SplineData>,
    instance_local_guid: u64,
    reference: ObjectId,
    totals: &mut LivingWorldModelInventory,
) {
    *totals
        .grind_reference_sections
        .entry(reference.section_index())
        .or_default() += 1;
    if reference.section_index() != 2 {
        *totals.failures.entry("RX2 grind section").or_default() += 1;
        return;
    }
    let Ok(subreference_index) = usize::try_from(reference.entry_index()) else {
        *totals.failures.entry("RX2 grind subreference").or_default() += 1;
        return;
    };
    let Some(subreference) = sections.subreferences.get(subreference_index) else {
        *totals.failures.entry("RX2 grind subreference").or_default() += 1;
        return;
    };
    *totals
        .grind_subreference_types
        .entry(subreference.type_id)
        .or_default() += 1;
    if subreference.object.section_index() != 0 {
        *totals.failures.entry("RX2 grind base section").or_default() += 1;
        return;
    }
    let Some(base_entry) = arena
        .dictionary
        .get(usize::try_from(subreference.object.entry_index()).unwrap_or(usize::MAX))
    else {
        *totals.failures.entry("RX2 grind base").or_default() += 1;
        return;
    };
    *totals
        .grind_base_types
        .entry(base_entry.type_id)
        .or_default() += 1;
    if base_entry.type_id != SPLINE_DATA_TYPE_ID {
        *totals.failures.entry("RX2 grind base type").or_default() += 1;
        return;
    }
    let Some(splines) = spline_data.get(&subreference.object.entry_index()) else {
        *totals.failures.entry("RX2 grind base").or_default() += 1;
        return;
    };
    let Some(header_index) = splines.spline_index_at_object_offset(subreference.object_offset)
    else {
        *totals.failures.entry("RX2 grind offset").or_default() += 1;
        return;
    };
    totals.resolved_grind_splines += 1;
    let Some(asset) = GrindAsset::from_spline_header(
        instance_local_guid,
        subreference.object.entry_index(),
        splines,
        header_index,
    ) else {
        *totals.failures.entry("RX2 native grind asset").or_default() += 1;
        return;
    };
    totals.native_grind_assets += 1;
    totals.native_grind_primitives += asset.primitives.len();
}

fn living_world_model_schema(label: &str) -> Option<&str> {
    label
        .split_once("\\model\\")?
        .1
        .split_once('\\')
        .map(|(schema, _)| schema)
}

fn print_living_world_model_inventory(inventory: &LivingWorldModelInventory) {
    println!(
        "living-world model TOC: {} assets / {} DMO model-ID matches; model corpus: {} named RX2 + {} livingworld hashed entries ({} non-arena: {} recipes + {} XML companions / {} schemas / {} models / {} LODs / {} asset requests, {} unique paths / {} resolved) / {} arenas, {} instances ({} with embedded data), {} embedded records, {} grind refs / {} exact resolutions / {} native assets with {} primitives, splines per body {:?}, grind-model schemas {:?}, reference sections {:?}, subreference types {:?}, base types {:?}; recipe DMO ID matches {:?}; {} DMO model-ID field matches / {} RX2 path matches / {} any named path matches / {} model-name path matches {:?}; failures {:?}",
        inventory.model_toc_assets,
        inventory.dmo_model_toc_matches.len(),
        inventory.rx2_files,
        inventory.hashed_model_entries,
        inventory.non_arena_hashed_entries,
        inventory.character_recipes,
        inventory.xml_recipe_companions,
        inventory.recipe_schemas.len(),
        inventory.recipe_models,
        inventory.recipe_lods,
        inventory.recipe_asset_requests,
        inventory.recipe_unique_asset_paths,
        inventory.resolved_recipe_asset_paths,
        inventory.arenas,
        inventory.instances,
        inventory.instances_with_embedded_data,
        inventory.embedded_records,
        inventory.grind_spline_references,
        inventory.resolved_grind_splines,
        inventory.native_grind_assets,
        inventory.native_grind_primitives,
        inventory.grind_splines_per_body,
        inventory.grind_model_schemas,
        inventory.grind_reference_sections,
        inventory.grind_subreference_types,
        inventory.grind_base_types,
        inventory.recipe_dmo_id_matches,
        inventory.dmo_model_id_matches,
        inventory.dmo_model_path_matches.len(),
        inventory.dmo_any_path_matches.len(),
        inventory.dmo_model_name_path_matches.len(),
        inventory.dmo_model_name_path_matches,
        inventory.failures,
    );
}

struct RuntimeDmoProbeResults {
    active_scene: LivingWorldGrindScene,
    inventory: RuntimeDmoGrindInventory,
    priority_weights: DynamicObjectPriorityWeights,
    resident_priority_indices: BTreeMap<i32, usize>,
    probe: RuntimeDmoGrindProbe,
    trajectory_hit_count: usize,
    geometry_hit_count: usize,
    geometry: Option<GrindGeometryCalculation>,
}

fn prepare_runtime_dmo_probe(
    mounts: &MountSet,
    world: &CollisionScene,
    templates: &DmoModelArenaInventory,
) -> Result<RuntimeDmoProbeResults, Box<dyn std::error::Error>> {
    let (catalog, inventory) = load_runtime_dmo_catalog(mounts, world.focus, templates)?;
    let retained_shape_matches = catalog.locators.iter().all(|locator| {
        locator
            .collision_template
            .as_ref()
            .is_some_and(|template| template.parts().len() == 1 && template.leaves().len() == 1)
    });
    if inventory.resident_locators != 249
        || inventory.resolved_model_locators != 249
        || inventory.resident_collision_bodies != 249
        || inventory.resident_collision_parts != 249
        || inventory.resident_collision_primitives != 35_626
        || inventory.resident_clustered_mesh_templates != 249
        || inventory.resident_clustered_mesh_template_parts != 249
        || inventory.resident_clustered_mesh_template_leaves != 249
        || !retained_shape_matches
        || !inventory.missing_model_ids.is_empty()
    {
        return Err(format!("runtime DMO retained-template corpus mismatch: {inventory:?}").into());
    }
    let priority_weights = catalog.priority_weights;
    let resident_priority_indices =
        catalog
            .locators
            .iter()
            .fold(BTreeMap::new(), |mut counts, locator| {
                *counts.entry(locator.priority_index).or_insert(0) += 1;
                counts
            });
    let census_candidates = catalog.census_candidates(world.focus, true);
    if !census_candidates.is_empty() {
        return Err(
            "runtime DMO census has candidates but the shared living-world entity runtime is not connected"
                .into(),
        );
    }
    let (active_scene, _) = RuntimeDmoCatalog::scene_for(census_candidates);
    let tuning = load_grind_geometry_tuning(mounts)?;
    let (probe_locator, source_primitive) = catalog
        .first_self_colliding_resident(&tuning)
        .ok_or("resident DMO catalog has no self-colliding grind primitive")?;
    let (probe_scene, collision_bodies) =
        RuntimeDmoCatalog::scene_for(std::iter::once(probe_locator));
    let probe = probe_first_runtime_dmo_grind(world, &probe_scene, source_primitive)?;
    let trajectory_hit_count =
        trajectory_grinds_around_with_living_world(world, &probe_scene, probe.midpoint, [2.0; 3])
            .len();
    let (geometry_hit_count, geometry) =
        probe_first_runtime_dmo_grind_geometry(world, mounts, &collision_bodies, &probe)?;
    Ok(RuntimeDmoProbeResults {
        active_scene,
        inventory,
        priority_weights,
        resident_priority_indices,
        probe,
        trajectory_hit_count,
        geometry_hit_count,
        geometry,
    })
}

fn probe_runtime_collision_scene(
    mounts: &MountSet,
    location_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = load_runtime_collision_scene(mounts, location_name)?;
    let counts = scene.counts();
    let dmo_templates = inventory_dmo_stream_model_arenas(mounts);
    let dmo = prepare_runtime_dmo_probe(mounts, &scene, &dmo_templates)?;
    let start = [scene.focus[0], scene.focus[1] + 50.0, scene.focus[2]];
    let end = [scene.focus[0], scene.focus[1] - 200.0, scene.focus[2]];
    let hits = scene.thin_segment_hits(start, end)?;
    let prediction_tuning = load_trajectory_prediction_tuning(mounts)?;
    let trajectory = BallisticTrajectory {
        initial_position: start,
        initial_velocity: [0.0, -10.0, 0.0],
        gravity: [0.0, -9.8, 0.0],
        duration: prediction_tuning.maximum_time,
    };
    let trajectory_prediction = predict_ballistic_trajectory(
        TrajectoryPredictionQuery {
            trajectory,
            radius: prediction_tuning.radius,
            maximum_error_start: prediction_tuning.maximum_error_start,
            maximum_error_end: prediction_tuning.maximum_error_end,
            calculate_ground_normal: true,
        },
        &mut scene,
    )?;
    let selection_tuning = load_trajectory_selection_tuning(mounts)?;
    let landing_position = trajectory.position_at_time(trajectory_prediction.collision_time);
    let grind_radius = [selection_tuning.grind_detection_radius; 3];
    let landing_grinds =
        grinds_around_with_living_world(&scene, &dmo.active_scene, landing_position, grind_radius);
    let first_grind_probe = probe_first_runtime_grind(&scene, &dmo.active_scene, grind_radius)?;
    let first_trajectory_grind_probe = trajectory_grinds_around_with_living_world(
        &scene,
        &dmo.active_scene,
        first_grind_probe.midpoint,
        grind_radius,
    );
    let first_grind_geometry = probe_static_grind_geometry(
        &scene,
        mounts,
        first_grind_probe.primitive,
        first_grind_probe.midpoint,
    )?;
    let current_collection = scene
        .current_collection()
        .ok_or("runtime scene lost its current collection")?;
    let collection_names = collision_collection_names(&scene);
    println!(
        "runtime collision scene {} at {:?}: current {}, selected {:?}; {:?}; {} arenas, {} roots, {} primitive sets / {} primitives, {} meshes / {} triangles, {} models / {} instances ({} collision-bound), {} grind assets / {} grind primitives; vertical probe {} hits, nearest {:?}; native ballistic prediction {:?}; landing grind query {} hits, authored-rail probe {} raw / {} trajectory-filtered hits, static geometry {:?}",
        scene.location_name,
        scene.focus,
        current_collection.name,
        collection_names,
        counts,
        counts.arenas,
        counts.aggregate_roots,
        counts.primitive_sets,
        counts.primitives,
        counts.meshes,
        counts.triangles,
        counts.collision_models,
        counts.instances,
        counts.collision_instances,
        counts.grind_assets,
        counts.grind_primitives,
        hits.len(),
        hits.first(),
        trajectory_prediction,
        landing_grinds.len(),
        first_grind_probe.nearby.len(),
        first_trajectory_grind_probe.len(),
        first_grind_geometry,
    );
    println!(
        "runtime DMO catalog: {:?}; authored spawn radius {:.1}, priority weights {:?}, resident PriorityIndex distribution {:?}, Rob's Park active pool 0 bodies (exact empty census); single-resident decoder probe {} combined hits / {} trajectory-filtered hits; resident DMO collision batch {} hits / geometry {:?}",
        dmo.inventory,
        dmo.inventory.census_spawn_radius,
        dmo.priority_weights,
        dmo.resident_priority_indices,
        dmo.probe.nearby.len(),
        dmo.trajectory_hit_count,
        dmo.geometry_hit_count,
        dmo.geometry,
    );
    verify_runtime_collision_round_trip(
        &mut scene,
        mounts,
        &dmo.active_scene,
        &first_grind_probe,
        grind_radius,
        start,
        end,
    )?;
    Ok(())
}

fn verify_runtime_collision_round_trip(
    scene: &mut CollisionScene,
    mounts: &MountSet,
    living_world: &LivingWorldGrindScene,
    first_grind_probe: &RuntimeGrindProbe,
    grind_radius: [f32; 3],
    start: [f32; 3],
    end: [f32; 3],
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = scene.thin_segment_hits(start, end)?;
    let first_trajectory_grind_probe = trajectory_grinds_around_with_living_world(
        scene,
        living_world,
        first_grind_probe.midpoint,
        grind_radius,
    );
    let original_focus = scene.focus;
    let adjacent_focus = [
        original_focus[0] + 200.0,
        original_focus[1],
        original_focus[2],
    ];
    let outward = move_runtime_collision_scene(scene, mounts, adjacent_focus)?;
    let adjacent_names = collision_collection_names(scene);
    let adjacent_counts = scene.counts();
    let returned = move_runtime_collision_scene(scene, mounts, original_focus)?;
    let returned_hits = scene.thin_segment_hits(start, end)?;
    let returned_grinds = grinds_around_with_living_world(
        scene,
        living_world,
        first_grind_probe.midpoint,
        grind_radius,
    );
    let returned_trajectory_grinds = trajectory_grinds_around_with_living_world(
        scene,
        living_world,
        first_grind_probe.midpoint,
        grind_radius,
    );
    if hits.first() != returned_hits.first() {
        return Err("round-trip stream movement changed the Rob's Park spawn hit".into());
    }
    if first_grind_probe.nearby != returned_grinds {
        return Err("round-trip stream movement changed the authored grind query".into());
    }
    if first_trajectory_grind_probe != returned_trajectory_grinds {
        return Err("round-trip stream movement changed the trajectory grind query".into());
    }
    println!(
        "runtime stream round trip: outward {:?} -> {:?} / {:?}; return {:?}; restored nearest {:?}",
        request_kinds(&outward),
        adjacent_names,
        adjacent_counts,
        request_kinds(&returned),
        returned_hits.first()
    );
    Ok(())
}

fn load_runtime_dmo_catalog(
    mounts: &MountSet,
    focus: [f32; 3],
    templates: &DmoModelArenaInventory,
) -> Result<(RuntimeDmoCatalog, RuntimeDmoGrindInventory), Box<dyn std::error::Error>> {
    let [minimum_speed_census, maximum_speed_census] = load_dynamic_object_census_circles(mounts)?;
    let priority_weights = load_dynamic_object_priority_weights(mounts)?;
    if minimum_speed_census != maximum_speed_census {
        return Err("stationary and maximum-speed DMO census circles differ".into());
    }
    let base = "data\\content\\world\\stream\\BAM\\BAM_Sim";
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    let selected = space.focus_collections(
        focus,
        WORLD_FOCUS_RADIUS,
        DEFAULT_SIMULATION_FOCUS_COLLECTIONS,
    );
    let mut locators = Vec::new();
    let mut totals = RuntimeDmoGrindInventory {
        census_spawn_radius: minimum_speed_census.spawn_radius,
        ..RuntimeDmoGrindInventory::default()
    };
    let census_radius_squared = minimum_speed_census.spawn_radius.powi(2);
    for selected_collection in selected {
        let collection = collections
            .collections
            .iter()
            .find(|entry| entry.id == selected_collection.id)
            .ok_or_else(|| format!("missing DMO collection {:016x}", selected_collection.id))?;
        let path = format!(
            "data\\content\\world\\stream\\BAM\\{}.xsf",
            collection.filename
        );
        let bytes = mounts.read_path(&path)?;
        let stream = StreamFile::parse(&bytes, collection.asset_count)?;
        for block in stream.assets {
            let decoded = decode_arena(&bytes[block.data_range])?;
            let arena = Rw4Arena::parse(&decoded.resources)?;
            for object in &arena.dictionary {
                if object.type_id != DMO_DATA_TYPE_ID {
                    continue;
                }
                let object_bytes = arena
                    .object_bytes(&decoded.resources, object)
                    .ok_or("resident DMO object range is invalid")?;
                for locator in DmoData::parse(object_bytes)?.dmos {
                    add_runtime_dmo_locator(
                        &locator,
                        focus,
                        census_radius_squared,
                        templates,
                        &mut locators,
                        &mut totals,
                    )?;
                }
            }
        }
    }
    Ok((
        RuntimeDmoCatalog {
            census: minimum_speed_census,
            priority_weights,
            locators,
        },
        totals,
    ))
}

fn add_runtime_dmo_locator(
    locator: &skate2_formats::pegasus::Dmo,
    focus: [f32; 3],
    census_radius_squared: f32,
    templates: &DmoModelArenaInventory,
    locators: &mut Vec<RuntimeDmoLocator>,
    totals: &mut RuntimeDmoGrindInventory,
) -> Result<(), Box<dyn std::error::Error>> {
    totals.resident_locators += 1;
    let census_delta_x = locator.transform[12] - focus[0];
    let census_delta_z = locator.transform[14] - focus[2];
    if census_delta_x.mul_add(census_delta_x, census_delta_z * census_delta_z)
        <= census_radius_squared
    {
        totals.census_candidate_locators += 1;
    }
    if !templates.dmo_models.contains_key(&locator.instance_guid) {
        totals.missing_model_ids.insert(locator.instance_guid);
        return Ok(());
    }
    totals.resolved_model_locators += 1;
    let priority_index = *templates
        .dmo_priority_indices
        .get(&locator.instance_guid)
        .ok_or("resolved DMO model has no PriorityIndex")?;
    let model_bounds = *templates
        .dmo_model_bounds
        .get(&locator.instance_guid)
        .ok_or("resolved DMO model has no bounding box")?;
    let collision_template = templates
        .dmo_clustered_mesh_templates
        .get(&locator.instance_guid)
        .cloned()
        .ok_or("resolved DMO model has no retained clustered-mesh template")?;
    totals.resident_clustered_mesh_templates += 1;
    totals.resident_clustered_mesh_template_parts += collision_template.parts().len();
    totals.resident_clustered_mesh_template_leaves += collision_template.leaves().len();
    let collision_parts = templates
        .dmo_collision_parts
        .get(&locator.instance_guid)
        .cloned()
        .unwrap_or_default();
    if !collision_parts.is_empty() {
        totals.resident_collision_bodies += 1;
        totals.resident_collision_parts += collision_parts.len();
        totals.resident_collision_primitives += collision_parts
            .iter()
            .map(|part| part.volumes.len() + part.triangles.len())
            .sum::<usize>();
    }
    let grind_assets = templates
        .dmo_grind_assets
        .get(&locator.instance_guid)
        .cloned()
        .unwrap_or_default();
    if !grind_assets.is_empty() {
        totals.resident_grind_bodies += 1;
        totals.resident_grind_assets += grind_assets.len();
        totals.resident_grind_primitives += grind_assets
            .iter()
            .map(|asset| asset.primitives.len())
            .sum::<usize>();
    }
    locators.push(RuntimeDmoLocator {
        locator_id: locator.guid,
        model_id: locator.instance_guid,
        priority_index,
        transform: locator.transform,
        model_bounds,
        grind_assets,
        collision_parts,
        collision_template: Some(collision_template),
    });
    Ok(())
}

struct RuntimeDmoGrindProbe {
    midpoint: [f32; 3],
    primitive: GrindPrimitive,
    nearby: Vec<GrindPrimitive>,
    body_transform: [f32; 16],
}

fn probe_first_runtime_dmo_grind(
    world: &CollisionScene,
    living_world: &LivingWorldGrindScene,
    source_primitive: GrindPrimitive,
) -> Result<RuntimeDmoGrindProbe, Box<dyn std::error::Error>> {
    let body = living_world
        .dynamic_objects
        .first()
        .ok_or("resident DMO catalog probe contains no grind bodies")?;
    let mut transformed = transform_dynamic_grind_primitive(source_primitive, body.transform);
    transformed.is_vertical = living_world.legacy_flags.dynamic_objects_are_vertical;
    #[allow(clippy::manual_midpoint)]
    let midpoint: [f32; 3] = std::array::from_fn(|axis| {
        (transformed.start_point[axis] + transformed.end_point[axis]) * 0.5
    });
    let query = GrindBounds {
        minimum: midpoint.map(|value| value - 2.0),
        maximum: midpoint.map(|value| value + 2.0),
    };
    let hits =
        world.grind_box_test_with_living_world(query.minimum, query.maximum, false, living_world);
    if hits.is_empty() {
        return Err("combined runtime grind query returned no DMO candidates".into());
    }
    if !hits.contains(&transformed) {
        return Err("combined runtime grind query omitted its source DMO primitive".into());
    }
    Ok(RuntimeDmoGrindProbe {
        midpoint,
        primitive: transformed,
        nearby: hits,
        body_transform: body.transform,
    })
}

fn probe_first_runtime_dmo_grind_geometry(
    world: &CollisionScene,
    mounts: &MountSet,
    collision_bodies: &[DynamicCollisionBody],
    probe: &RuntimeDmoGrindProbe,
) -> Result<(usize, Option<GrindGeometryCalculation>), Box<dyn std::error::Error>> {
    let tuning = load_grind_geometry_tuning(mounts)?;
    collision_bodies
        .iter()
        .find(|body| {
            body.model_id == probe.primitive.source_asset_id
                && body
                    .transform
                    .into_iter()
                    .zip(probe.body_transform)
                    .all(|(left, right)| left.to_bits() == right.to_bits())
        })
        .ok_or("source DMO grind body has no matching collision body")?;
    let dmo_hit_count = match prepare_grind_geometry(&probe.primitive, probe.midpoint, &tuning) {
        Some(PreparedGrindGeometry::Query(setup)) => {
            dynamic_grind_geometry_batch(collision_bodies, &setup.batch)
                .into_iter()
                .filter(|collision| collision.hit)
                .count()
        }
        _ => 0,
    };
    if dmo_hit_count == 0 {
        return Err("source DMO collision body missed its grind-geometry batch".into());
    }
    let geometry = calculate_grind_geometry(&probe.primitive, probe.midpoint, &tuning, |batch| {
        let ground = world.static_grind_geometry_batch(batch)?;
        let dmo = dynamic_grind_geometry_batch(collision_bodies, batch);
        Ok::<_, skate2_formats::collision::CollisionError>(merge_grind_geometry_collision_batches(
            [&ground, &dmo],
        ))
    })?;
    Ok((dmo_hit_count, geometry))
}

fn collision_collection_names(scene: &CollisionScene) -> Vec<String> {
    scene
        .collections
        .iter()
        .map(|collection| collection.name.clone())
        .collect()
}

fn trajectory_grinds_around_with_living_world(
    scene: &CollisionScene,
    living_world: &LivingWorldGrindScene,
    midpoint: [f32; 3],
    radius: [f32; 3],
) -> Vec<GrindPrimitive> {
    scene.trajectory_grind_box_test_with_living_world(
        std::array::from_fn(|axis| midpoint[axis] - radius[axis]),
        std::array::from_fn(|axis| midpoint[axis] + radius[axis]),
        midpoint,
        GrindPairFilterTuning::default(),
        living_world,
    )
}

fn grinds_around_with_living_world(
    scene: &CollisionScene,
    living_world: &LivingWorldGrindScene,
    midpoint: [f32; 3],
    radius: [f32; 3],
) -> Vec<GrindPrimitive> {
    scene.grind_box_test_with_living_world(
        std::array::from_fn(|axis| midpoint[axis] - radius[axis]),
        std::array::from_fn(|axis| midpoint[axis] + radius[axis]),
        false,
        living_world,
    )
}

struct RuntimeGrindProbe {
    midpoint: [f32; 3],
    primitive: GrindPrimitive,
    nearby: Vec<GrindPrimitive>,
}

fn probe_first_runtime_grind(
    scene: &CollisionScene,
    living_world: &LivingWorldGrindScene,
    radius: [f32; 3],
) -> Result<RuntimeGrindProbe, Box<dyn std::error::Error>> {
    let first = scene
        .collections
        .iter()
        .flat_map(|collection| &collection.arenas)
        .flat_map(|arena| &arena.grind_assets)
        .flat_map(|asset| &asset.primitives)
        .find(|primitive| !primitive.is_vertical)
        .copied()
        .ok_or("runtime simulation scene contains no authored grind primitives")?;
    #[allow(clippy::manual_midpoint)]
    let midpoint: [f32; 3] =
        std::array::from_fn(|axis| (first.start_point[axis] + first.end_point[axis]) * 0.5);
    let results = grinds_around_with_living_world(scene, living_world, midpoint, radius);
    if results.is_empty() {
        return Err("authored grind primitive was absent from its own box query".into());
    }
    if !results.contains(&first) {
        return Err("authored grind query did not return its source primitive".into());
    }
    Ok(RuntimeGrindProbe {
        midpoint,
        primitive: first,
        nearby: results,
    })
}

fn probe_static_grind_geometry(
    scene: &CollisionScene,
    mounts: &MountSet,
    primitive: GrindPrimitive,
    midpoint: [f32; 3],
) -> Result<Option<GrindGeometryCalculation>, Box<dyn std::error::Error>> {
    let tuning = load_grind_geometry_tuning(mounts)?;
    Ok(calculate_grind_geometry(
        &primitive,
        midpoint,
        &tuning,
        |batch| scene.static_grind_geometry_batch(batch),
    )?)
}

fn request_kinds(transition: &StreamFocusTransition) -> Vec<(CollectionRequestKind, u64)> {
    transition
        .requests
        .iter()
        .map(|request| (request.kind, request.id))
        .collect()
}

fn probe_runtime_presentation_scene(
    mounts: &MountSet,
    location_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = load_runtime_presentation_scene(mounts, location_name)?;
    // `NativeRenderQueue::build` walks every resident collection.
    let required_manifests = scene
        .resident_collections()
        .flat_map(|collection| &collection.arenas)
        .flat_map(|arena| &arena.materials)
        .filter_map(|material| material.effect.as_ref())
        .filter_map(|effect| effect.binding.as_ref())
        .map(|binding| binding.manifest_name.clone())
        .collect::<BTreeSet<_>>();
    let manifests = load_native_shader_manifests(mounts, &required_manifests)?;
    print_default_effect_mesh_uses(&scene);
    let mut render_scopes = RenderScopeRegistry::default();
    let initial_render_queue = NativeRenderQueue::build(&scene, &manifests, &mut render_scopes)?;
    let initial_render_signature = native_render_queue_signature(&initial_render_queue);
    let current = scene
        .current_collection()
        .ok_or("runtime presentation scene lost its current collection")?;
    let names = scene
        .collections
        .iter()
        .map(|collection| collection.name.as_str())
        .collect::<Vec<_>>();
    println!(
        "runtime presentation scene {} at {:?}: current {}, selected {:?}; {:?}",
        scene.location_name,
        scene.focus,
        current.name,
        names,
        scene.counts()
    );
    let initial_counts = scene.counts();
    let initial_signature = presentation_scene_signature(&scene);
    let initial_focus = scene.focus;
    let adjacent_focus = [initial_focus[0] + 200.0, initial_focus[1], initial_focus[2]];
    let outward = move_runtime_presentation_scene(&mut scene, mounts, adjacent_focus)?;
    let adjacent_render_queue = NativeRenderQueue::build(&scene, &manifests, &mut render_scopes)?;
    let adjacent_names = scene
        .collections
        .iter()
        .map(|collection| collection.name.clone())
        .collect::<Vec<_>>();
    let adjacent_counts = scene.counts();
    let returning = move_runtime_presentation_scene(&mut scene, mounts, initial_focus)?;
    let returned_render_queue = NativeRenderQueue::build(&scene, &manifests, &mut render_scopes)?;
    if scene.counts() != initial_counts || presentation_scene_signature(&scene) != initial_signature
    {
        return Err("presentation stream round trip did not restore the owned scene".into());
    }
    if native_render_queue_signature(&returned_render_queue) != initial_render_signature {
        return Err("presentation stream round trip changed the native render queue".into());
    }
    println!(
        "runtime presentation stream round trip: outward {:?} -> {:?} / {:?}; return {:?}; restored {:?}",
        outward
            .requests
            .iter()
            .map(|request| (request.kind, request.id))
            .collect::<Vec<_>>(),
        adjacent_names,
        adjacent_counts,
        returning
            .requests
            .iter()
            .map(|request| (request.kind, request.id))
            .collect::<Vec<_>>(),
        initial_counts,
    );
    print_native_render_queue_summary(
        &initial_render_queue,
        &adjacent_render_queue,
        &returned_render_queue,
    );
    let texture_scene = probe_runtime_texture_scene(mounts, location_name)?;
    let binding_counts = scene.bind_resident_textures(&texture_scene);
    for channel in scene
        .collections
        .iter()
        .flat_map(|collection| &collection.arenas)
        .flat_map(|arena| &arena.materials)
        .flat_map(|material| &material.channels)
    {
        let PresentationMaterialValue::Raster {
            guid,
            resident_texture: Some(handle),
            ..
        } = &channel.value
        else {
            continue;
        };
        if texture_scene
            .texture(*handle)
            .is_none_or(|texture| texture.guid != *guid)
        {
            return Err("resident material texture handle is inconsistent".into());
        }
    }
    println!("runtime material texture bindings: {binding_counts:?}");
    Ok(())
}

fn print_default_effect_mesh_uses(scene: &PresentationScene) {
    let mut counts = BTreeMap::<String, usize>::new();
    let mut samples = BTreeMap::<String, String>::new();
    for collection in &scene.collections {
        for arena in &collection.arenas {
            for instance in &arena.instances {
                let Some(model_index) = instance.render_model else {
                    continue;
                };
                let Some(model) = arena.models.get(model_index) else {
                    continue;
                };
                for model_mesh in &model.meshes {
                    let Some(mesh) = arena.meshes.get(model_mesh.mesh_index) else {
                        continue;
                    };
                    let Some(material_index) = mesh.material else {
                        continue;
                    };
                    let Some(effect) = arena
                        .materials
                        .get(material_index)
                        .and_then(|material| material.effect.as_ref())
                    else {
                        continue;
                    };
                    if effect
                        .binding
                        .as_ref()
                        .map(|binding| binding.manifest_name.as_str())
                        != Some("default")
                    {
                        continue;
                    }
                    *counts.entry(effect.serialized_name.clone()).or_default() += 1;
                    samples
                        .entry(effect.serialized_name.clone())
                        .or_insert_with(|| {
                            format!(
                                "collection={}, arena={:08x}, instance={:?}, model={}({}), mesh={}({}), model_material={:?}, mesh_material={material_index}",
                                collection.name,
                                arena.id,
                                instance.name,
                                model_index,
                                model.dictionary_index,
                                model_mesh.mesh_index,
                                mesh.dictionary_index,
                                model_mesh.material,
                            )
                        });
                }
            }
        }
    }
    println!("runtime null-effect mesh uses: {counts:?}; samples {samples:?}");
}

fn print_native_render_queue_summary(
    initial: &NativeRenderQueue,
    adjacent: &NativeRenderQueue,
    returned: &NativeRenderQueue,
) {
    let islands = |queue: &NativeRenderQueue| {
        queue
            .submissions
            .iter()
            .map(|submission| submission.island_count)
            .sum::<usize>()
    };
    println!(
        "native render queue: initial {} mesh contexts / {} islands / {} deferred / {} effect scopes / {} material scopes; adjacent {} / {} islands / {} deferred; restored {} / {} islands / {} deferred",
        initial.submissions.len(),
        islands(initial),
        initial.deferred.len(),
        initial.effect_scopes,
        initial.material_scopes,
        adjacent.submissions.len(),
        islands(adjacent),
        adjacent.deferred.len(),
        returned.submissions.len(),
        islands(returned),
        returned.deferred.len(),
    );
}

fn load_native_shader_manifests(
    mounts: &MountSet,
    required: &BTreeSet<String>,
) -> Result<BTreeMap<String, ShaderManifest>, Box<dyn std::error::Error>> {
    let mut manifests = BTreeMap::new();
    for mounted in mounts.archives() {
        for entry in &mounted.archive().entries {
            let BigEntryKey::Name(path) = &entry.key else {
                continue;
            };
            let lower = path.to_ascii_lowercase();
            if !lower.starts_with("data\\render\\shaders\\final\\")
                || !std::path::Path::new(&lower)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
            {
                continue;
            }
            let filename = lower
                .rsplit_once('\\')
                .map_or(lower.as_str(), |(_, name)| name);
            let manifest_name = filename
                .strip_suffix(".xml")
                .ok_or("invalid manifest path")?;
            let manifest = ShaderManifest::parse(&mounted.read_path(path)?)?;
            if manifests
                .insert(manifest_name.to_owned(), manifest)
                .is_some()
            {
                return Err(format!("duplicate final shader manifest {manifest_name}").into());
            }
        }
    }
    // External-header BIG files expose only hashes. The reference opens an FX
    // manifest by virtual path, so probe every VLT-selected effect name through
    // the mount table even when the archive cannot enumerate its source name.
    for manifest_name in required {
        if manifests.contains_key(manifest_name) {
            continue;
        }
        let path = format!("data\\render\\shaders\\final\\{manifest_name}.xml");
        if let Ok(bytes) = mounts.read_path(&path) {
            manifests.insert(manifest_name.clone(), ShaderManifest::parse(&bytes)?);
        }
    }
    Ok(manifests)
}

type NativeRenderSignature = (
    u32,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    String,
    String,
);

type DeferredRenderSignature = (usize, usize, usize, usize, usize, usize, String, String);

fn native_render_queue_signature(
    queue: &NativeRenderQueue,
) -> (Vec<NativeRenderSignature>, Vec<DeferredRenderSignature>) {
    let submissions = queue
        .submissions
        .iter()
        .map(|submission| {
            (
                submission.sort_key.packed(),
                submission.scene.collection,
                submission.scene.arena,
                submission.scene.instance,
                submission.scene.model,
                submission.scene.mesh,
                submission.scene.material,
                submission.manifest_name.clone(),
                submission.technique_name.clone(),
            )
        })
        .collect();
    let deferred = queue
        .deferred
        .iter()
        .map(|submission| {
            (
                submission.scene.collection,
                submission.scene.arena,
                submission.scene.instance,
                submission.scene.model,
                submission.scene.mesh,
                submission.scene.material,
                submission.manifest_name.clone(),
                submission.material_name.clone(),
            )
        })
        .collect();
    (submissions, deferred)
}

type PresentationMeshSignature = (u64, u32, u32, usize, usize, usize, u64);

#[derive(Debug, PartialEq, Eq)]
struct PresentationSceneSignature {
    meshes: Vec<PresentationMeshSignature>,
    materials: Vec<PresentationMaterialSignature>,
}

#[derive(Debug, PartialEq, Eq)]
struct PresentationMaterialSignature {
    collection_id: u64,
    arena_id: u32,
    material_index: usize,
    serialized_name: Option<String>,
    manifest_name: Option<String>,
    draw_order: Option<i32>,
}

fn presentation_scene_signature(scene: &PresentationScene) -> PresentationSceneSignature {
    let meshes = scene
        .collections
        .iter()
        .flat_map(|collection| {
            collection.arenas.iter().flat_map(move |arena| {
                arena.meshes.iter().map(move |mesh| {
                    (
                        collection.id,
                        arena.id,
                        mesh.dictionary_index,
                        mesh.geometry.positions.len(),
                        mesh.geometry.attributes.len(),
                        mesh.geometry.indices.len(),
                        presentation_geometry_signature(&mesh.geometry),
                    )
                })
            })
        })
        .collect();
    let materials = scene
        .collections
        .iter()
        .flat_map(|collection| {
            collection.arenas.iter().flat_map(move |arena| {
                arena
                    .materials
                    .iter()
                    .enumerate()
                    .map(move |(material_index, material)| {
                        let effect = material.effect.as_ref();
                        PresentationMaterialSignature {
                            collection_id: collection.id,
                            arena_id: arena.id,
                            material_index,
                            serialized_name: effect.map(|value| value.serialized_name.clone()),
                            manifest_name: effect
                                .and_then(|value| value.binding.as_ref())
                                .map(|binding| binding.manifest_name.clone()),
                            draw_order: effect
                                .and_then(|value| value.binding.as_ref())
                                .map(|binding| binding.draw_order),
                        }
                    })
            })
        })
        .collect();
    PresentationSceneSignature { meshes, materials }
}

fn presentation_geometry_signature(geometry: &NativeMeshGeometry) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for position in &geometry.positions {
        for component in position {
            hash = fnv1a64_continue(hash, &component.to_bits().to_le_bytes());
        }
    }
    for attribute in &geometry.attributes {
        let (usage, index) = vertex_semantic_identity(attribute.semantic);
        hash = fnv1a64_continue(hash, &[usage, index]);
        hash = fnv1a64_continue(hash, &attribute.original_format.to_le_bytes());
        match &attribute.values {
            NativeVertexAttributeValues::Float2(values) => {
                for value in values.iter().flatten() {
                    hash = fnv1a64_continue(hash, &value.to_bits().to_le_bytes());
                }
            }
            NativeVertexAttributeValues::Float3(values) => {
                for value in values.iter().flatten() {
                    hash = fnv1a64_continue(hash, &value.to_bits().to_le_bytes());
                }
            }
            NativeVertexAttributeValues::Float4(values) => {
                for value in values.iter().flatten() {
                    hash = fnv1a64_continue(hash, &value.to_bits().to_le_bytes());
                }
            }
            NativeVertexAttributeValues::UnsignedByte4(values) => {
                for value in values {
                    hash = fnv1a64_continue(hash, value);
                }
            }
            NativeVertexAttributeValues::SignedShort2(values) => {
                for value in values.iter().flatten() {
                    hash = fnv1a64_continue(hash, &value.to_le_bytes());
                }
            }
        }
    }
    for index in &geometry.indices {
        hash = fnv1a64_continue(hash, &index.to_le_bytes());
    }
    for draw in &geometry.draws {
        hash = fnv1a64_continue(hash, &draw.primitive_type.to_le_bytes());
        hash = fnv1a64_continue(hash, &draw.base_vertex_index.to_le_bytes());
        hash = fnv1a64_continue(hash, &draw.start_index.to_le_bytes());
        hash = fnv1a64_continue(hash, &draw.index_count.to_le_bytes());
    }
    hash
}

const fn vertex_semantic_identity(semantic: VertexSemantic) -> (u8, u8) {
    match semantic {
        VertexSemantic::Position(index) => (0, index),
        VertexSemantic::BlendWeight(index) => (1, index),
        VertexSemantic::BlendIndices(index) => (2, index),
        VertexSemantic::Normal(index) => (3, index),
        VertexSemantic::TextureCoordinate(index) => (5, index),
        VertexSemantic::Tangent(index) => (6, index),
        VertexSemantic::Sample(index) => (13, index),
        VertexSemantic::Other { usage, index } => (usage, index),
    }
}

fn probe_runtime_texture_scene(
    mounts: &MountSet,
    location_name: &str,
) -> Result<TextureScene, Box<dyn std::error::Error>> {
    let mut scene = load_runtime_texture_scene(mounts, location_name)?;
    let initial_counts = scene.counts();
    let initial_signature = texture_scene_signature(&scene);
    let current = scene
        .current_collection()
        .ok_or("runtime texture scene lost its current collection")?;
    println!(
        "runtime texture scene {} at {:?}: current {}; selected {:?}; {:?}",
        scene.location_name,
        scene.focus,
        current.name,
        scene
            .collections
            .iter()
            .map(|collection| collection.name.as_str())
            .collect::<Vec<_>>(),
        initial_counts,
    );
    let initial_focus = scene.focus;
    let adjacent_focus = [initial_focus[0] + 150.0, initial_focus[1], initial_focus[2]];
    let outward = move_runtime_texture_scene(&mut scene, mounts, adjacent_focus)?;
    let adjacent_names = scene
        .collections
        .iter()
        .map(|collection| collection.name.clone())
        .collect::<Vec<_>>();
    let adjacent_counts = scene.counts();
    let returning = move_runtime_texture_scene(&mut scene, mounts, initial_focus)?;
    if scene.counts() != initial_counts || texture_scene_signature(&scene) != initial_signature {
        return Err("texture stream round trip did not restore the owned scene".into());
    }
    println!(
        "runtime texture stream round trip: outward {:?} -> {:?} / {:?}; return {:?}; restored {:?}",
        request_kinds(&outward),
        adjacent_names,
        adjacent_counts,
        request_kinds(&returning),
        initial_counts,
    );
    Ok(scene)
}

type TextureSignature = (u64, u32, u64, u32, u32, u32, usize, u64, usize, u64);

fn texture_scene_signature(scene: &TextureScene) -> Vec<TextureSignature> {
    scene
        .collections
        .iter()
        .flat_map(|collection| {
            collection.arenas.iter().flat_map(move |arena| {
                arena.textures.iter().map(move |texture| {
                    (
                        collection.id,
                        arena.id,
                        texture.guid,
                        texture.width,
                        texture.height,
                        texture.pixel_format,
                        texture.encoded_data.len(),
                        fnv1a64(&texture.encoded_data),
                        texture.linear_levels.len(),
                        texture
                            .linear_levels
                            .iter()
                            .fold(0xcbf2_9ce4_8422_2325, |hash, level| {
                                let hash = fnv1a64_continue(hash, &level.level.to_le_bytes());
                                let hash = fnv1a64_continue(hash, &level.face.to_le_bytes());
                                let hash = fnv1a64_continue(hash, &level.width.to_le_bytes());
                                let hash = fnv1a64_continue(hash, &level.height.to_le_bytes());
                                fnv1a64_continue(hash, &level.data)
                            }),
                    )
                })
            })
        })
        .collect()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    fnv1a64_continue(0xcbf2_9ce4_8422_2325, bytes)
}

fn fnv1a64_continue(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn load_runtime_texture_scene(
    mounts: &MountSet,
    location_name: &str,
) -> Result<TextureScene, Box<dyn std::error::Error>> {
    let locations =
        LocationTable::parse(&mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc")?)?;
    let location = locations
        .find(location_name)
        .ok_or_else(|| format!("unknown BAM location {location_name}"))?;
    let focus = location.position();
    load_runtime_texture_scene_at(mounts,location_name,focus)
}
fn load_runtime_texture_scene_at(mounts:&MountSet,location_name:&str,focus:[f32;3])->Result<TextureScene,Box<dyn std::error::Error>>{
    let base = "data\\content\\world\\stream\\BAM\\BAM_Tex";
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    let selected = space.focus_collections(
        focus,
        TEXTURE_FOCUS_RADIUS,
        DEFAULT_TEXTURE_FOCUS_COLLECTIONS,
    );
    let mut stream_focus = StreamFocusState::new(focus, false);
    let transition = stream_focus.reconcile(&selected);
    let mut queue = CollectionRequestQueue::default();
    queue.apply_transition(&transition);
    let mut runtime_collections = Vec::with_capacity(selected.len());
    while let Some(request) = queue.take_next_load() {
        let focus_collection = selected
            .iter()
            .find(|entry| entry.id == request.id)
            .ok_or_else(|| format!("unknown texture request {:016x}", request.id))?;
        runtime_collections.push(load_texture_collection(
            mounts,
            &collections,
            focus_collection,
        )?);
    }
    runtime_collections.sort_by(|left, right| left.distance_xz.total_cmp(&right.distance_xz));
    Ok(TextureScene {
        location_name: location_name.into(),
        focus,
        stream_focus,
        collections: runtime_collections,
    })
}

fn load_texture_collection(
    mounts: &MountSet,
    collections: &CollectionMap,
    focus_collection: &skate2_formats::stream::FocusCollection,
) -> Result<TextureCollection, Box<dyn std::error::Error>> {
    let collection = collections
        .collections
        .iter()
        .find(|entry| entry.id == focus_collection.id)
        .ok_or_else(|| format!("missing texture collection {:016x}", focus_collection.id))?;
    let path = format!(
        "data\\content\\world\\stream\\BAM\\{}.xsf",
        collection.filename
    );
    let xsf = mounts.read_path(&path)?;
    let file = StreamFile::parse(&xsf, collection.asset_count)?;
    let mut arenas = Vec::with_capacity(file.assets.len());
    for block in &file.assets {
        let decoded = decode_arena(&xsf[block.data_range.clone()])?;
        let arena = Rw4Arena::parse(&decoded.resources)?;
        arenas.push(own_texture_arena(&arena, &decoded.resources)?);
    }
    Ok(TextureCollection {
        id: focus_collection.id,
        name: collection.filename.clone(),
        position: focus_collection.position,
        distance_xz: focus_collection.distance_xz,
        arenas,
    })
}

fn move_runtime_texture_scene(
    scene: &mut TextureScene,
    mounts: &MountSet,
    position: [f32; 3],
) -> Result<StreamFocusTransition, Box<dyn std::error::Error>> {
    if !scene.stream_focus.move_to(position, false) {
        return Err("texture focus move was rejected".into());
    }
    let base = "data\\content\\world\\stream\\BAM\\BAM_Tex";
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    let selected = space.focus_collections(
        position,
        TEXTURE_FOCUS_RADIUS,
        DEFAULT_TEXTURE_FOCUS_COLLECTIONS,
    );
    move_runtime_texture_selection(scene, mounts, position, &collections, &selected)
}

fn move_runtime_texture_selection(
    scene: &mut TextureScene,
    mounts: &MountSet,
    position: [f32; 3],
    collections: &CollectionMap,
    selected: &[skate2_formats::stream::FocusCollection],
) -> Result<StreamFocusTransition, Box<dyn std::error::Error>> {
    scene.stream_focus.move_to(position, false);
    let transition = scene.stream_focus.reconcile(&selected);
    let mut queue = CollectionRequestQueue::default();
    queue.apply_transition(&transition);
    while let Some(request) = queue.take_next_unload() {
        let index = scene
            .collections
            .iter()
            .position(|collection| collection.id == request.id)
            .ok_or("texture unload targets an absent collection")?;
        scene.collections.remove(index);
    }
    while let Some(request) = queue.take_next_load() {
        let focus_collection = selected
            .iter()
            .find(|entry| entry.id == request.id)
            .ok_or("texture load targets an absent focus entry")?;
        scene.collections.push(load_texture_collection(
            mounts,
            collections,
            focus_collection,
        )?);
    }
    for collection in &mut scene.collections {
        let selected_entry = selected
            .iter()
            .find(|entry| entry.id == collection.id)
            .ok_or("resident texture collection left the focus set")?;
        collection.position = selected_entry.position;
        collection.distance_xz = selected_entry.distance_xz;
    }
    let order = skate2_formats::stream::order_like_selection(&mut scene.collections, |collection| collection.id, selected);
    report_tie_order(order, "texture", position);
    if !order.matches {
        return Err("resident texture collections disagree with focus selection".into());
    }
    scene.focus = position;
    Ok(transition)
}

/// Resident collections at equal distances put back in the focus selection's order, where a
/// sort by distance alone failed the reconcile check and ended the game.
fn report_tie_order(order: skate2_formats::stream::SelectionOrder, kind: &str, position: [f32; 3]) {
    if order.ties_reordered > 0 {
        eprintln!("controlled stream: {} {kind} collection pair(s) at equal distances kept in the selection's order at {position:?}", order.ties_reordered);
    }
}

fn load_runtime_presentation_scene(
    mounts: &MountSet,
    location_name: &str,
) -> Result<PresentationScene, Box<dyn std::error::Error>> {
    let locations =
        LocationTable::parse(&mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc")?)?;
    let location = locations
        .find(location_name)
        .ok_or_else(|| format!("unknown BAM location {location_name}"))?;
    let focus = location.position();
    load_runtime_presentation_scene_at(mounts,location_name,focus)
}
fn load_runtime_presentation_scene_at(mounts:&MountSet,location_name:&str,focus:[f32;3])->Result<PresentationScene,Box<dyn std::error::Error>>{
    let base = "data\\content\\world\\stream\\BAM\\BAM_Pres";
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    let selected = space.focus_collections(
        focus,
        WORLD_FOCUS_RADIUS,
        DEFAULT_PRESENTATION_FOCUS_COLLECTIONS,
    );
    let mut stream_focus = StreamFocusState::new(focus, false);
    let transition = stream_focus.reconcile(&selected);
    let mut queue = CollectionRequestQueue::default();
    queue.apply_transition(&transition);
    let mut requests = Vec::with_capacity(selected.len());
    if let Some(request) = queue.take_best_load() {
        requests.push(request);
    }
    while let Some(request) = queue.take_next_load() {
        requests.push(request);
    }
    if requests.len() != selected.len() {
        return Err("initial presentation queue did not dispatch the focus set".into());
    }
    let dependencies = load_presentation_dependencies(mounts, &collections, &selected, Vec::new())?;
    let mut runtime_collections = Vec::with_capacity(selected.len());
    for request in requests {
        let focus_collection = selected
            .iter()
            .find(|entry| entry.id == request.id)
            .ok_or_else(|| format!("unknown presentation request {:016x}", request.id))?;
        runtime_collections.push(load_presentation_collection(
            mounts,
            &collections,
            focus_collection,
        )?);
    }
    runtime_collections.sort_by(|left, right| left.distance_xz.total_cmp(&right.distance_xz));
    let mut scene = PresentationScene {
        dependencies,
        loose: load_loose_presentation_assets(mounts)?,
        // The hull loaders' collections, which the city streamer registers.
        hulls: Vec::new(),
        // Registered by the city streamer once the PROXY world activates.
        proxies: Vec::new(),
        location_name: location_name.into(),
        focus,
        stream_focus,
        collections: runtime_collections,
    };
    bind_runtime_material_attributes(mounts, &mut scene)?;
    Ok(scene)
}

/// `GameModule::OneOffLoadTexture` 0x82735730 `LooseAssetHelper::Add` order
/// for the sky: `DIST_skybox_Textures.rx2`, then `DIST_skybox.rx2`, whose
/// "skybox"-named model becomes the `SkyPresEntity`
/// (`WorldPresentation::AddRenderInstance` 0x827e4f00). `LoadAssets`
/// 0x8272cdb8 turns '/' into '\' and loads each file as one arena.
///
/// The native next two loads (827357f8/8273580c) supply distant ocean geometry
/// and textures. AddRenderInstance827e4f00 gives these ordinary world entities;
/// only the sky and occlusion test receive special entity types.
/// `material_test.rx2` remains omitted: its only instance is never rendered.
const LOOSE_PRESENTATION_ASSETS: [&str; 4] = [
    "data\\content\\world\\models\\DIST_skybox_Textures.rx2",
    "data\\content\\world\\models\\DIST_skybox.rx2",
    "data\\content\\world\\models\\DIST_Water_Textures.rx2",
    "data\\content\\world\\models\\DIST_Water.rx2",
];

/// Mandatory global textures have their own owner. An optional sky/material
/// failure must not discard the vignette, HoM overlay or replay watermarks.
fn load_loose_presentation_assets(
    mounts: &MountSet,
) -> Result<Vec<PresentationCollection>, Box<dyn std::error::Error>> {
    let boot = load_boot_presentation_collection(mounts)
        .map_err(|error| format!("original boot texture inventory: {error}"))?;
    let mut collections = vec![boot];
    match load_loose_presentation_collection(mounts) {
        Ok(collection) => collections.push(collection),
        Err(error) => {
            eprintln!(
                "loose presentation assets not loaded ({error}); the sky dome and distant mountains stay absent"
            );
        }
    }
    Ok(collections)
}

fn load_loose_presentation_collection(
    mounts: &MountSet,
) -> Result<PresentationCollection, Box<dyn std::error::Error>> {
    let mut arenas = Vec::with_capacity(LOOSE_PRESENTATION_ASSETS.len());
    for path in LOOSE_PRESENTATION_ASSETS {
        let bytes = mounts.read_path(path)?;
        let resources = Rw4Arena::split_file_resources(&bytes)?;
        let arena = Rw4Arena::parse(&resources)?;
        arenas.push(
            own_presentation_arena(&arena, &resources)
                .map_err(|error| format!("{path}: {error}"))?,
        );
    }
    let keys = arenas
        .iter()
        .flat_map(|arena| &arena.materials)
        .filter_map(|material| material.effect.as_ref())
        .filter_map(|effect| effect.attribute_class_id.zip(effect.material_variant_id))
        .collect::<BTreeSet<_>>();
    load_material_attribute_bindings(mounts, &keys)?;
    Ok(PresentationCollection {
        light_probes: Vec::new(),
        navigation: Vec::new(),
        // Host cache identity only; distinct from every stream collection GUID.
        id: string_hash64(LOOSE_PRESENTATION_ASSETS[1]),
        name: "OneOffLoadTexture loose assets".to_owned(),
        position: [0.0; 3],
        distance_xz: 0.0,
        arenas,
    })
}

fn load_boot_presentation_collection(
    mounts: &MountSet,
) -> Result<PresentationCollection, Box<dyn std::error::Error>> {
    // TextureMan::AddMaterial8282c838 / ResolveMissingTexture8282c7c8 use
    // the boot name configured by Sk8AppMan::InitBoot82740d4c. Retain the
    // original assets as a resident collection; no generated replacements.
    // The arena's CPU-only sun map continues through SunDirectionMap.
    const PATH: &str = "data\\content\\textures\\globaltextures.rx2";
    let bytes = mounts.read_path(PATH)?;
    let resources = Rw4Arena::split_file_resources(&bytes)?;
    let arena = Rw4Arena::parse(&resources)?;
    let missing = skate2_core::mounted_character_scene::own_texture_asset(
        &arena,
        &resources,
        skate2_formats::hash::rw_hash64_string("missingtexture.Texture"),
    )?;
    // PostFX::SetTexturePointers828287cc resolves this same boot inventory.
    let vignette = skate2_core::mounted_character_scene::own_texture_asset(
        &arena, &resources,
        skate2_formats::hash::rw_hash64_string("vignette_gradient.Texture"),
    )?;
    // PostFX::SetTexturePointers82828840: the HoM overlay is another
    // registered texture in this same original boot arena.
    let xray = skate2_core::mounted_character_scene::own_texture_asset(
        &arena, &resources,
        skate2_formats::hash::rw_hash64_string("xray.Texture"),
    )?;
    // SetWatermarkStatus8255d7f0 -> SetWatermarkTexture82828610 resolves
    // these from the same registered boot arena for replay/photo capture.
    // Keep both choices resident with the other global PostFX textures.
    let mut boot_textures = vec![missing, vignette, xray];
    for name in ["logo_watermark.Texture", "logo_watermark_pro.Texture"] {
        boot_textures.push(skate2_core::mounted_character_scene::own_texture_asset(
            &arena, &resources, skate2_formats::hash::rw_hash64_string(name),
        ).map_err(|error| format!("{name}: {error}"))?);
    }
    // PostFX::SetTexturePointers 0x828288bc..0x8282890c: the deck AO texture, which the
    // original leaves null when the arena lacks it.
    if let Ok(texture) = skate2_core::mounted_character_scene::own_texture_asset(
        &arena, &resources, skate2_formats::hash::rw_hash64_string("board_occlusion.Texture"),
    ) {
        boot_textures.push(texture);
    }
    let boot_arena = skate2_core::presentation::PresentationArena {
        id: arena.id,
        textures: boot_textures,
        materials: Vec::new(),
        meshes: Vec::new(),
        models: Vec::new(),
        instances: Vec::new(),
        anti_frustums: Vec::new(),
        emitter_waypoints: Vec::new(),
    };
    Ok(PresentationCollection {
        light_probes: Vec::new(),
        navigation: Vec::new(),
        // Host cache identity only; distinct from every stream collection GUID.
        id: string_hash64(PATH),
        name: "Original boot textures".to_owned(),
        position: [0.0; 3],
        distance_xz: 0.0,
        arenas: vec![boot_arena],
    })
}

/// Source ancestor ordering comes from the existing LoadCollection planner.
/// This retains visual inventories; it does not synthesize native callbacks.
fn load_presentation_dependencies(
    mounts: &MountSet,
    collections: &CollectionMap,
    selected: &[skate2_formats::stream::FocusCollection],
    mut retained: Vec<PresentationCollection>,
) -> Result<Vec<PresentationCollection>, Box<dyn std::error::Error>> {
    use skate2_core::{
        collection_parent_manager::CollectionParentManager,
        collection_resident_streamer::CollectionId,
    };
    let mut planner = CollectionParentManager::new(proxy_texture_world::Planning);
    for row in &collections.collections {
        planner.add_parent(CollectionId(row.id), CollectionId(row.parent_id));
    }
    let spatial: BTreeSet<_> = selected.iter().map(|row| row.id).collect();
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for child in selected {
        for request in planner.plan_load(CollectionId(child.id))? {
            let id = request.id.0;
            if spatial.contains(&id) || !seen.insert(id) {
                continue;
            }
            if let Some(index) = retained.iter().position(|row| row.id == id) {
                result.push(retained.remove(index));
            } else {
                let dependency = skate2_formats::stream::FocusCollection {
                    id,
                    // Ancestors are dependencies, not additional spatial queries.
                    position: child.position,
                    distance_xz: child.distance_xz,
                    ..*child
                };
                result.push(load_presentation_collection(
                    mounts,
                    collections,
                    &dependency,
                )?);
            }
        }
    }
    Ok(result)
}

fn load_presentation_collection(
    mounts: &MountSet,
    collections: &CollectionMap,
    focus_collection: &skate2_formats::stream::FocusCollection,
) -> Result<PresentationCollection, Box<dyn std::error::Error>> {
    let collection = collections
        .collections
        .iter()
        .find(|entry| entry.id == focus_collection.id)
        .ok_or_else(|| {
            format!(
                "missing presentation collection {:016x}",
                focus_collection.id
            )
        })?;
    let path = format!(
        "data\\content\\world\\stream\\BAM\\{}.xsf",
        collection.filename
    );
    let xsf = mounts.read_path(&path)?;
    let payload = xsf.get(collection.payload_offset as usize..)
        .ok_or("presentation collection payload offset")?;
    let file = skate2_formats::stream::CollectionPayload::parse(payload, collection.asset_count)?;
    let metadata = AssetToc::parse(&mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Pres.xst")?)?;
    let mut navigation = Vec::new();
    let mut light_probes = Vec::new();
    let mut arenas = Vec::with_capacity(file.assets.len());
    for block in &file.assets {
        let decoded = decode_arena(&payload[block.data_range.clone()])?;
        let arena = Rw4Arena::parse(&decoded.resources)?;
        if std::env::var_os("SKATE2_TRACE_CITY_TEXTURES").is_some() {
            let textures = arena
                .dictionary
                .iter()
                .filter(|e| e.type_id == skate2_formats::material::TEXTURE_TYPE_ID)
                .count();
            if textures != 0 {
                eprintln!("Pres arena {:08x}: texture objects={textures}", arena.id);
            }
        }
        for entry in &arena.dictionary {
            if entry.type_id != skate2_formats::pegasus::TABLE_OF_CONTENTS_TYPE_ID {continue;}
            let toc = skate2_formats::pegasus::TableOfContents::parse(
                arena.object_bytes(&decoded.resources, entry).ok_or("hull TOC object range")?)?;
            for item in toc.entries {
                if item.object_type != skate2_formats::irradiance::TYPE_ID {continue;}
                let probe_entry = arena.local_entry(item.object)
                    .filter(|e|e.type_id == skate2_formats::irradiance::TYPE_ID)
                    .ok_or("hull irradiance TOC reference")?;
                let bytes = arena.object_bytes(&decoded.resources, probe_entry)
                    .ok_or("hull irradiance object range")?;
                // RegAssets827f1430 passes the registered object's TOC GUID,
                // not the containing XSF arena block ID.
                light_probes.push((item.guid, Arc::from(skate2_formats::irradiance::parse(bytes)?)));
            }
        }
        arenas.push(own_presentation_arena(&arena, &decoded.resources)?);
        if arena.dictionary.iter().any(|entry| entry.type_id == skate2_core::simulation_normal_arena_plan::NAV_MESH_2_DATA_TYPE_ID) {
            let entry = metadata.assets.iter().find(|entry| entry.id == block.id)
                .ok_or("presentation navigation XST asset absent")?;
            let asset = SimulationCollisionAsset::from_decoded(DecodedArenaAsset {
                asset_id: block.id, asset_type: entry.asset_type,
                asset_resource_descriptors: entry.resources, resources: decoded, arena,
            })?;
            navigation.push(skate2_core::presentation::PresentationNavigationArena(Arc::new(
                SimulationNormalArenaPlan::from_collision_asset(asset)?)));
        }
    }
    Ok(PresentationCollection {
                light_probes,
        navigation,
        id: focus_collection.id,
        name: collection.filename.clone(),
        position: focus_collection.position,
        distance_xz: focus_collection.distance_xz,
        arenas,
    })
}

fn move_runtime_presentation_scene(
    scene: &mut PresentationScene,
    mounts: &MountSet,
    position: [f32; 3],
) -> Result<StreamFocusTransition, Box<dyn std::error::Error>> {
    if !scene.stream_focus.move_to(position, false) {
        return Err("presentation focus move was rejected".into());
    }
    let base = "data\\content\\world\\stream\\BAM\\BAM_Pres";
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    let selected = space.focus_collections(
        position,
        WORLD_FOCUS_RADIUS,
        DEFAULT_PRESENTATION_FOCUS_COLLECTIONS,
    );
    move_runtime_presentation_selection(scene, mounts, position, &collections, &selected)
}

fn move_runtime_presentation_selection(
    scene: &mut PresentationScene,
    mounts: &MountSet,
    position: [f32; 3],
    collections: &CollectionMap,
    selected: &[skate2_formats::stream::FocusCollection],
) -> Result<StreamFocusTransition, Box<dyn std::error::Error>> {
    let dependencies = load_presentation_dependencies(
        mounts,
        collections,
        selected,
        std::mem::take(&mut scene.dependencies),
    )?;
    scene.stream_focus.move_to(position, false);
    let transition = scene.stream_focus.reconcile(&selected);
    let mut queue = CollectionRequestQueue::default();
    queue.apply_transition(&transition);
    while let Some(request) = queue.take_next_unload() {
        let index = scene
            .collections
            .iter()
            .position(|collection| collection.id == request.id)
            .ok_or_else(|| {
                format!(
                    "presentation unload requested for absent collection {:016x}",
                    request.id
                )
            })?;
        scene.collections.remove(index);
    }
    while let Some(request) = if scene.collections.is_empty() {
        queue.take_best_load()
    } else {
        queue.take_next_load()
    } {
        let focus_collection = selected
            .iter()
            .find(|entry| entry.id == request.id)
            .ok_or_else(|| {
                format!(
                    "presentation load requested for absent focus entry {:016x}",
                    request.id
                )
            })?;
        scene.collections.push(load_presentation_collection(
            mounts,
            collections,
            focus_collection,
        )?);
    }
    for collection in &mut scene.collections {
        let selected_entry = selected
            .iter()
            .find(|entry| entry.id == collection.id)
            .ok_or_else(|| {
                format!(
                    "resident presentation collection {:016x} left the focus set",
                    collection.id
                )
            })?;
        collection.position = selected_entry.position;
        collection.distance_xz = selected_entry.distance_xz;
    }
    let order = skate2_formats::stream::order_like_selection(&mut scene.collections, |collection| collection.id, selected);
    report_tie_order(order, "presentation", position);
    if !order.matches {
        let resident: Vec<_> = scene.collections.iter().map(|collection| (collection.id, collection.distance_xz)).collect();
        let expected: Vec<_> = selected.iter().map(|entry| (entry.id, entry.distance_xz)).collect();
        return Err(format!("resident presentation collections disagree with focus selection at {position:?}: {resident:x?} vs {expected:x?}").into());
    }
    scene.dependencies = dependencies;
    scene.focus = position;
    bind_runtime_material_attributes(mounts, scene)?;
    Ok(transition)
}

fn load_runtime_collision_scene(
    mounts: &MountSet,
    location_name: &str,
) -> Result<CollisionScene, Box<dyn std::error::Error>> {
    let base = "data\\content\\world\\stream\\BAM\\BAM_Sim";
    let locations = LocationTable::parse(&mounts.read_path(&format!("{base}.loc"))?)?;
    let location = locations
        .find(location_name)
        .ok_or_else(|| format!("unknown BAM location {location_name}"))?;
    let focus = location.position();
    let expected_current_name = high_detail_collection_filename("Sim", focus);
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    let focus_collections = space.focus_collections(
        focus,
        WORLD_FOCUS_RADIUS,
        DEFAULT_SIMULATION_FOCUS_COLLECTIONS,
    );
    let mut stream_focus = StreamFocusState::new(focus, false);
    let initial_transition = stream_focus.reconcile(&focus_collections);
    if initial_transition
        .requests
        .iter()
        .any(|request| request.kind != CollectionRequestKind::Load)
    {
        return Err("initial stream-focus transition contained a non-load request".into());
    }
    let mut request_queue = CollectionRequestQueue::default();
    request_queue.apply_transition(&initial_transition);
    let mut dispatched_loads = Vec::with_capacity(focus_collections.len());
    if let Some(request) = request_queue.take_best_load() {
        dispatched_loads.push(request);
    }
    while let Some(request) = request_queue.take_next_load() {
        dispatched_loads.push(request);
    }
    if !request_queue.pending_unloads().is_empty()
        || dispatched_loads.len() != focus_collections.len()
    {
        return Err(
            "initial simulation request queue did not dispatch the complete focus set".into(),
        );
    }
    let current_id = initial_transition
        .current_id
        .ok_or_else(|| format!("no simulation collection overlaps {location_name}"))?;
    let current_collection_name = collections
        .collections
        .iter()
        .find(|entry| entry.id == current_id)
        .map(|entry| entry.filename.clone())
        .ok_or_else(|| format!("missing collection ID {current_id:016x}"))?;
    if current_collection_name != expected_current_name {
        return Err(format!(
            "stream-grid current collection {expected_current_name} disagrees with spatial selection {current_collection_name}"
        )
        .into());
    }

    let mut runtime_collections = Vec::with_capacity(focus_collections.len());
    for request in dispatched_loads {
        let focus_collection = focus_collections
            .iter()
            .find(|entry| entry.id == request.id)
            .ok_or_else(|| format!("request for unknown focus collection {:016x}", request.id))?;
        runtime_collections.push(load_collision_collection(
            mounts,
            &collections,
            focus_collection,
        )?);
    }
    runtime_collections.sort_by(|left, right| left.distance_xz.total_cmp(&right.distance_xz));
    Ok(CollisionScene {
        location_name: location.name.clone(),
        focus,
        stream_focus,
        collections: runtime_collections,
    })
}

fn load_collision_collection(
    mounts: &MountSet,
    collections: &CollectionMap,
    focus_collection: &skate2_formats::stream::FocusCollection,
) -> Result<CollisionCollection, Box<dyn std::error::Error>> {
    let collection = collections
        .collections
        .iter()
        .find(|entry| entry.id == focus_collection.id)
        .ok_or_else(|| format!("missing collection ID {:016x}", focus_collection.id))?;
    let xsf_path = format!(
        "data\\content\\world\\stream\\BAM\\{}.xsf",
        collection.filename
    );
    let xsf = mounts.read_path(&xsf_path)?;
    let file = StreamFile::parse(&xsf, collection.asset_count)?;
    let mut arenas = Vec::with_capacity(file.assets.len());
    for block in &file.assets {
        let decoded = decode_arena(&xsf[block.data_range.clone()])?;
        let arena = Rw4Arena::parse(&decoded.resources)?;
        arenas.push(own_collision_arena(&arena, &decoded.resources)?);
    }
    Ok(CollisionCollection {
        id: focus_collection.id,
        name: collection.filename.clone(),
        position: focus_collection.position,
        distance_xz: focus_collection.distance_xz,
        arenas,
    })
}

fn move_runtime_collision_scene(
    scene: &mut CollisionScene,
    mounts: &MountSet,
    position: [f32; 3],
) -> Result<StreamFocusTransition, Box<dyn std::error::Error>> {
    if !scene.stream_focus.move_to(position, false) {
        return Err("simulation focus move was rejected".into());
    }
    let base = "data\\content\\world\\stream\\BAM\\BAM_Sim";
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    let selected = space.focus_collections(
        position,
        WORLD_FOCUS_RADIUS,
        DEFAULT_SIMULATION_FOCUS_COLLECTIONS,
    );
    let transition = scene.stream_focus.reconcile(&selected);
    let mut queue = CollectionRequestQueue::default();
    queue.apply_transition(&transition);

    while let Some(request) = queue.take_next_unload() {
        let index = scene
            .collections
            .iter()
            .position(|collection| collection.id == request.id)
            .ok_or_else(|| format!("unload requested for absent collection {:016x}", request.id))?;
        scene.collections.remove(index);
    }
    while let Some(request) = if scene.collections.is_empty() {
        queue.take_best_load()
    } else {
        queue.take_next_load()
    } {
        let focus_collection = selected
            .iter()
            .find(|entry| entry.id == request.id)
            .ok_or_else(|| format!("load requested for absent focus entry {:016x}", request.id))?;
        scene.collections.push(load_collision_collection(
            mounts,
            &collections,
            focus_collection,
        )?);
    }

    for collection in &mut scene.collections {
        let selected_entry = selected
            .iter()
            .find(|entry| entry.id == collection.id)
            .ok_or_else(|| {
                format!(
                    "resident collection {:016x} left the focus set",
                    collection.id
                )
            })?;
        collection.position = selected_entry.position;
        collection.distance_xz = selected_entry.distance_xz;
    }
    let order = skate2_formats::stream::order_like_selection(&mut scene.collections, |collection| collection.id, &selected);
    report_tie_order(order, "simulation", position);
    if !order.matches {
        return Err("resident simulation collections disagree with focus selection".into());
    }
    scene.focus = position;
    Ok(transition)
}

fn own_collision_arena(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
) -> Result<CollisionArena, Box<dyn std::error::Error>> {
    let mut owned = CollisionArena {
        id: arena.id,
        grind_assets: Vec::new(),
        aggregate_roots: Vec::new(),
        primitive_sets: Vec::new(),
        simple_primitive_sets: Vec::new(),
        meshes: Vec::new(),
        collision_models: Vec::new(),
        instances: Vec::new(),
    };
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != SPLINE_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} spline data range is invalid", arena.id))?;
        let dictionary_index = u32::try_from(entry_index)?;
        let data = SplineData::parse(bytes)?;
        owned.grind_assets.push(GrindAsset::from_spline_data(
            u64::from(arena.id),
            dictionary_index,
            &data,
        ));
    }
    let geometry_entries = own_collision_geometry(arena, resources, &mut owned)?;
    link_collision_roots(
        arena.id,
        &mut owned.aggregate_roots,
        &geometry_entries.primitive_sets,
        &geometry_entries.simple_primitive_sets,
        &geometry_entries.meshes,
    )?;
    let model_entries =
        own_collision_models(arena, resources, &geometry_entries.roots, &mut owned)?;
    own_collision_instances(arena, resources, &model_entries, &mut owned)?;
    Ok(owned)
}

type ObjectIndexMap = BTreeMap<u32, usize>;

struct CollisionGeometryEntries {
    roots: ObjectIndexMap,
    primitive_sets: ObjectIndexMap,
    simple_primitive_sets: ObjectIndexMap,
    meshes: ObjectIndexMap,
}

fn own_collision_geometry(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    owned: &mut CollisionArena,
) -> Result<CollisionGeometryEntries, Box<dyn std::error::Error>> {
    let mut root_entries = BTreeMap::new();
    let mut primitive_set_entries = BTreeMap::new();
    let mut simple_primitive_set_entries = BTreeMap::new();
    let mut mesh_entries = BTreeMap::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        let Some(bytes) = arena.object_bytes(resources, entry) else {
            return Err(format!("arena {:08x} object range is invalid", arena.id).into());
        };
        let dictionary_index = u32::try_from(entry_index)?;
        match CollisionObjectType::from_type_id(entry.type_id) {
            Some(CollisionObjectType::Volume) => {
                root_entries.insert(dictionary_index, owned.aggregate_roots.len());
                owned.aggregate_roots.push(CollisionRoot {
                    dictionary_index,
                    volume: Volume::parse(bytes)?,
                    aggregate: None,
                });
            }
            Some(CollisionObjectType::KdTreeMappedArray) => {
                let mapped = KdTreeMappedArray::parse(bytes)?;
                primitive_set_entries.insert(dictionary_index, owned.primitive_sets.len());
                owned.primitive_sets.push(CollisionPrimitiveSet {
                    dictionary_index,
                    bounding_box_min: mapped.bounding_box_min[..3].try_into().unwrap(),
                    bounding_box_max: mapped.bounding_box_max[..3].try_into().unwrap(),
                    volumes: mapped.volumes(bytes)?,
                    mapped_array: mapped,
                    object_bytes: bytes.to_vec(),
                });
            }
            Some(CollisionObjectType::SimpleMappedArray) => {
                let mapped = SimpleMappedArray::parse(bytes)?;
                simple_primitive_set_entries
                    .insert(dictionary_index, owned.simple_primitive_sets.len());
                owned
                    .simple_primitive_sets
                    .push(CollisionSimplePrimitiveSet {
                        dictionary_index,
                        bounding_box_min: mapped.bounding_box_min[..3].try_into().unwrap(),
                        bounding_box_max: mapped.bounding_box_max[..3].try_into().unwrap(),
                        volumes: mapped.volumes(bytes)?,
                    });
            }
            Some(CollisionObjectType::ClusteredMesh) => {
                let mesh = ClusteredMesh::parse(bytes)?;
                mesh_entries.insert(dictionary_index, owned.meshes.len());
                let minimum = mesh.bounding_box_min[..3].try_into().unwrap();
                let maximum = mesh.bounding_box_max[..3].try_into().unwrap();
                let ranges = mesh
                    .kd_tree(bytes)?
                    .leaf_ranges_for_aabb(bytes, minimum, maximum)?;
                owned.meshes.push(CollisionMesh {
                    dictionary_index,
                    bounding_box_min: minimum,
                    bounding_box_max: maximum,
                    triangles: mesh.triangles_for_leaf_ranges(bytes, &ranges)?,
                    clustered_mesh: mesh,
                    object_bytes: bytes.to_vec(),
                });
            }
            _ => {}
        }
    }
    Ok(CollisionGeometryEntries {
        roots: root_entries,
        primitive_sets: primitive_set_entries,
        simple_primitive_sets: simple_primitive_set_entries,
        meshes: mesh_entries,
    })
}

fn link_collision_roots(
    arena_id: u32,
    roots: &mut [CollisionRoot],
    primitive_set_entries: &ObjectIndexMap,
    simple_primitive_set_entries: &ObjectIndexMap,
    mesh_entries: &ObjectIndexMap,
) -> Result<(), Box<dyn std::error::Error>> {
    for root in roots {
        let Some(id) = root.volume.aggregate_object_id().map(ObjectId) else {
            continue;
        };
        let entry_index = require_local_entry(id, "aggregate")?;
        root.aggregate = if let Some(index) = primitive_set_entries.get(&entry_index) {
            Some(CollisionAggregateRef::PrimitiveSet(*index))
        } else if let Some(index) = simple_primitive_set_entries.get(&entry_index) {
            Some(CollisionAggregateRef::SimplePrimitiveSet(*index))
        } else if let Some(index) = mesh_entries.get(&entry_index) {
            Some(CollisionAggregateRef::Mesh(*index))
        } else {
            return Err(format!(
                "arena {:08x} aggregate root {} targets missing entry {}",
                arena_id, root.dictionary_index, entry_index
            )
            .into());
        };
    }
    Ok(())
}

fn own_collision_models(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    root_entries: &ObjectIndexMap,
    owned: &mut CollisionArena,
) -> Result<ObjectIndexMap, Box<dyn std::error::Error>> {
    let mut model_entries = BTreeMap::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        if entry.type_id != C_MODEL_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} collision model range is invalid", arena.id))?;
        let dictionary_index = u32::try_from(entry_index)?;
        let data = CollisionModelData::parse(bytes)?;
        let bounding_root = data
            .bounding_volume
            .map(|id| resolve_local_entry(id, root_entries, "collision-model bounding root"))
            .transpose()?;
        let (collision_roots, assembly_object_id) = match data.meshes {
            CollisionModelMeshes::Assembly(id) => (Vec::new(), Some(id.0)),
            CollisionModelMeshes::Meshes(meshes) => (
                meshes
                    .into_iter()
                    .map(|mesh| {
                        resolve_local_entry(mesh.mesh, root_entries, "collision-model mesh root")
                    })
                    .collect::<Result<_, _>>()?,
                None,
            ),
        };
        model_entries.insert(dictionary_index, owned.collision_models.len());
        owned.collision_models.push(CollisionModel {
            dictionary_index,
            bounding_root,
            collision_roots,
            assembly_object_id,
        });
    }
    Ok(model_entries)
}

fn own_collision_instances(
    arena: &Rw4Arena,
    resources: &[Vec<u8>; 5],
    model_entries: &ObjectIndexMap,
    owned: &mut CollisionArena,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in &arena.dictionary {
        if entry.type_id != INSTANCE_DATA_TYPE_ID {
            continue;
        }
        let bytes = arena
            .object_bytes(resources, entry)
            .ok_or_else(|| format!("arena {:08x} instance data range is invalid", arena.id))?;
        for instance in InstanceData::parse(bytes)?.instances {
            let collision_model = instance
                .collision_model
                .map(|id| resolve_local_entry(id, model_entries, "instance collision model"))
                .transpose()?;
            owned.instances.push(CollisionInstance {
                guid: instance.guid,
                local_guid: instance.local_guid,
                name: instance.name,
                transform: instance.transform,
                bounding_box_min: instance.bounding_box_min[..3].try_into().unwrap(),
                bounding_box_max: instance.bounding_box_max[..3].try_into().unwrap(),
                collision_model,
            });
        }
    }
    Ok(())
}

fn print_simulation_inventory(inventory: &SimulationInventory) {
    println!(
        "BAM Sim corpus: {} collections / {} bytes, {} asset blocks, {} RW4 arenas, {} objects",
        inventory.collections,
        inventory.xsf_bytes,
        inventory.asset_blocks,
        inventory.rw4_arenas,
        inventory.rw4_objects
    );
    println!(
        "BAM Sim collision: {} wrapper volumes, {} mapped arrays / {} inline volumes, {} thin + {} fat primitive probe hits / {} thin + {} fat misses / {} invalid parameters, {} meshes, {} clusters, {} KD branches / {} entries, {} vertices, {} units / {} triangles; {} failures",
        inventory.volumes,
        inventory.mapped_arrays,
        inventory.mapped_volumes,
        inventory.primitive_probe_hits,
        inventory.primitive_fat_probe_hits,
        inventory.primitive_probe_misses,
        inventory.primitive_fat_probe_misses,
        inventory.invalid_primitive_parameters,
        inventory.meshes,
        inventory.clusters,
        inventory.kd_branches,
        inventory.kd_entries,
        inventory.vertices,
        inventory.units,
        inventory.triangles,
        inventory.failures.values().sum::<usize>()
    );
    println!(
        "BAM Sim scene: {} instances / {} exact TOC subreferences, {} splines / {} segments ({} vertical; {} candidate probes / {} misses) / {} exact TOC subreferences / {} instance links, {} locations / {} sublocations / {} exact TOC subreferences, {} triggers / {} exact TOC subreferences, {} collision models, {} TOC entries; links: {} local, {} subreference, {} null-arena, {} external-arena ({} resolved, {} target arenas unavailable)",
        inventory.instances,
        inventory.instance_subreferences,
        inventory.splines,
        inventory.spline_segments,
        inventory.vertical_grind_primitives,
        inventory.grind_candidate_probe_hits,
        inventory.grind_candidate_probe_misses,
        inventory.spline_subreferences,
        inventory.spline_instance_links,
        inventory.location_descriptors,
        inventory.sub_location_descriptors,
        inventory.location_subreferences,
        inventory.trigger_instances,
        inventory.trigger_subreferences,
        inventory.collision_models,
        inventory.toc_entries,
        inventory.local_object_links,
        inventory.subreference_links,
        inventory.null_arena_links,
        inventory.external_arena_links,
        inventory.resolved_external_links,
        inventory.unavailable_external_links
    );
    println!(
        "BAM Sim embedded: {} records, {} grind-spline refs, {} grab-spline refs",
        inventory.embedded_data, inventory.embedded_grind_splines, inventory.embedded_grab_splines
    );
    println!(
        "BAM Sim DMO: {} containers / {} locators / {} locator-local GUIDs / {} sim-model IDs / {} render-model IDs decoded from names / {} nonzero serialized instance words",
        inventory.dmo_data,
        inventory.dmos,
        inventory.dmo_local_guids.len(),
        inventory.dmo_instance_guids.len(),
        inventory.dmo_model_local_guids.len(),
        inventory.dmo_nonzero_serialized_instance_words,
    );
    print_simulation_dmo_joins(inventory);
    print_simulation_inventory_details(inventory);
}

fn print_simulation_dmo_joins(inventory: &SimulationInventory) {
    println!(
        "BAM Sim instance/DMO joins: {} primary GUID matches / {} local GUID matches / {} external stream-asset matches {:?} / {} external TOC GUID matches {:?} / {} model-local TOC GUID matches {:?}",
        inventory
            .instance_guids
            .intersection(&inventory.dmo_instance_guids)
            .count(),
        inventory
            .instance_local_guids
            .intersection(&inventory.dmo_instance_guids)
            .count(),
        inventory
            .stream_assets_by_id
            .keys()
            .filter(|id| inventory.dmo_instance_guids.contains(id))
            .count(),
        inventory
            .stream_assets_by_id
            .iter()
            .filter(|(id, _)| inventory.dmo_instance_guids.contains(id))
            .collect::<BTreeMap<_, _>>(),
        inventory
            .toc_entries_by_guid
            .keys()
            .filter(|guid| inventory.dmo_instance_guids.contains(guid))
            .count(),
        inventory
            .toc_entries_by_guid
            .iter()
            .filter(|(guid, _)| inventory.dmo_instance_guids.contains(guid))
            .collect::<BTreeMap<_, _>>(),
        inventory
            .toc_entries_by_guid
            .keys()
            .filter(|guid| inventory.dmo_model_local_guids.contains(guid))
            .count(),
        inventory
            .toc_entries_by_guid
            .iter()
            .filter(|(guid, _)| inventory.dmo_model_local_guids.contains(guid))
            .collect::<BTreeMap<_, _>>(),
    );
    println!(
        "BAM Sim instance-data discriminators: {:?}; DMO external-GUID matches by discriminator: {:?}",
        inventory.instance_data_types,
        inventory
            .instance_guids_by_data_type
            .iter()
            .map(|(type_id, guids)| (
                format!("{type_id:08x}"),
                guids.intersection(&inventory.dmo_instance_guids).count()
            ))
            .collect::<BTreeMap<_, _>>(),
    );
    let dmo_instances = inventory
        .instance_guids_by_data_type
        .get(&INSTANCE_DMO_ASSET_TYPE_ID);
    println!(
        "BAM Sim Instance_DMO ({INSTANCE_DMO_ASSET_TYPE_ID:08x}): {} instances / {} locator model-ID joins",
        inventory
            .instance_data_types
            .get(&INSTANCE_DMO_ASSET_TYPE_ID)
            .copied()
            .unwrap_or_default(),
        dmo_instances.map_or(0, |guids| guids
            .intersection(&inventory.dmo_instance_guids)
            .count()),
    );
}

fn print_simulation_inventory_details(inventory: &SimulationInventory) {
    if !inventory.grind_candidate_miss_samples.is_empty() {
        println!(
            "BAM Sim grind candidate miss samples: {:?}",
            inventory.grind_candidate_miss_samples
        );
    }
    for (stage, count) in &inventory.failures {
        println!("  failure {stage}: {count}");
    }
    println!("BAM Sim RW4 type counts:");
    for (type_id, count) in &inventory.object_types {
        println!("  {type_id:08x}: {count}");
    }
    println!(
        "BAM Sim cluster compression: {:?}",
        inventory.compression_modes
    );
    println!(
        "BAM Sim mapped volume types: {:?}",
        inventory.mapped_volume_types
    );
    println!(
        "BAM Sim object-link sections: {:?}",
        inventory.object_link_sections
    );
    println!("BAM Sim TOC types: {:?}", inventory.toc_types);
}

fn inventory_sim_collision(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    type_id: u32,
    bytes: &[u8],
    totals: &mut SimulationInventory,
) {
    match CollisionObjectType::from_type_id(type_id) {
        Some(CollisionObjectType::Volume) => match Volume::parse(bytes) {
            Ok(volume) => {
                totals.volumes += 1;
                validate_object_link(
                    arena,
                    sections,
                    volume.aggregate_object_id().map(ObjectId),
                    &[0x0008_0004, 0x0008_0006],
                    "aggregate volume link",
                    totals,
                );
            }
            Err(_) => record_failure(totals, "volume header"),
        },
        Some(CollisionObjectType::KdTreeMappedArray) => {
            inventory_kd_tree_mapped_array(bytes, totals);
        }
        Some(CollisionObjectType::ClusteredMesh) => inventory_clustered_mesh(bytes, totals),
        _ => {}
    }
}

fn inventory_kd_tree_mapped_array(bytes: &[u8], totals: &mut SimulationInventory) {
    let Ok(mapped) = KdTreeMappedArray::parse(bytes) else {
        record_failure(totals, "mapped-array header");
        return;
    };
    totals.mapped_arrays += 1;
    let Ok(volumes) = mapped.volumes(bytes) else {
        record_failure(totals, "mapped-array volumes");
        return;
    };
    totals.mapped_volumes += volumes.len();
    for volume in &volumes {
        *totals
            .mapped_volume_types
            .entry(volume.vtable_offset)
            .or_default() += 1;
        match probe_primitive_volume(volume) {
            Some((thin_hit, fat_hit)) => {
                if thin_hit {
                    totals.primitive_probe_hits += 1;
                } else {
                    totals.primitive_probe_misses += 1;
                }
                if fat_hit {
                    totals.primitive_fat_probe_hits += 1;
                } else {
                    totals.primitive_fat_probe_misses += 1;
                }
            }
            None if matches!(
                volume.volume_type(),
                Some(VolumeType::Sphere | VolumeType::Box | VolumeType::Cylinder)
            ) =>
            {
                totals.invalid_primitive_parameters += 1;
            }
            None => {}
        }
    }
    let Ok(tree) = mapped.kd_tree(bytes) else {
        record_failure(totals, "mapped-array KD header");
        return;
    };
    totals.kd_branches += u64::from(tree.branch_node_count);
    totals.kd_entries += u64::from(tree.entry_count);
    let minimum = [
        mapped.bounding_box_min[0],
        mapped.bounding_box_min[1],
        mapped.bounding_box_min[2],
    ];
    let maximum = [
        mapped.bounding_box_max[0],
        mapped.bounding_box_max[1],
        mapped.bounding_box_max[2],
    ];
    match mapped.candidate_volume_indices_for_aabb(bytes, minimum, maximum) {
        Ok(indices) if indices.len() == volumes.len() => {}
        Ok(_) | Err(_) => record_failure(totals, "mapped-array KD full-bounds query"),
    }
}

fn probe_primitive_volume(volume: &Volume) -> Option<(bool, bool)> {
    let extent = match volume.volume_type()? {
        VolumeType::Sphere => volume.radius,
        VolumeType::Box => f32::from_bits(volume.shape_words[0]),
        VolumeType::Cylinder => f32::from_bits(volume.shape_words[1]),
        _ => return None,
    };
    let raw_axis = [
        volume.transform[0],
        volume.transform[1],
        volume.transform[2],
    ];
    let axis_length = raw_axis
        .into_iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if !extent.is_finite() || extent <= 0.0 || !axis_length.is_finite() || axis_length <= 0.0 {
        return None;
    }
    let axis = raw_axis.map(|value| value / axis_length);
    let center = [
        volume.transform[12],
        volume.transform[13],
        volume.transform[14],
    ];
    if !center.into_iter().all(f32::is_finite) {
        return None;
    }
    let distance = extent + extent.mul_add(0.25, 0.25);
    let start = std::array::from_fn(|index| center[index] - axis[index] * distance);
    let delta = axis.map(|value| value * distance * 2.0);
    Some((
        volume.thin_segment_intersection(start, delta).is_some(),
        volume
            .segment_intersection_with_fatness(start, delta, 0.2)
            .is_some(),
    ))
}

fn inventory_sim_pegasus(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    subreference_kinds: &[Option<u32>],
    type_id: u32,
    bytes: &[u8],
    totals: &mut SimulationInventory,
) {
    match type_id {
        SPLINE_DATA_TYPE_ID => {
            inventory_spline_data(arena, sections, subreference_kinds, bytes, totals);
        }
        INSTANCE_DATA_TYPE_ID => inventory_instance_data(arena, sections, bytes, totals),
        EMBEDDED_DATA_TYPE_ID => inventory_embedded_data(arena, sections, bytes, totals),
        DMO_DATA_TYPE_ID => inventory_dmo_data(bytes, totals),
        LOCATION_DESC_DATA_TYPE_ID => match LocationDescData::parse(bytes) {
            Ok(data) => {
                totals.location_descriptors += data.locations.len();
                totals.sub_location_descriptors += data.sub_locations.len();
                for location in &data.locations {
                    if data.sub_locations_for(location).is_none() {
                        record_failure(totals, "location sublocation span");
                    }
                }
            }
            Err(_) => record_failure(totals, "location descriptor data"),
        },
        TRIGGER_INSTANCE_DATA_TYPE_ID => match TriggerInstanceData::parse(bytes) {
            Ok(data) => {
                totals.trigger_instances += data.instances.len();
                for trigger in data.instances {
                    validate_object_link(
                        arena,
                        sections,
                        trigger.collision_model,
                        &[C_MODEL_DATA_TYPE_ID],
                        "trigger collision-model link",
                        totals,
                    );
                }
            }
            Err(_) => record_failure(totals, "trigger instance data"),
        },
        C_MODEL_DATA_TYPE_ID => match CollisionModelData::parse(bytes) {
            Ok(data) => {
                totals.collision_models += 1;
                validate_object_link(
                    arena,
                    sections,
                    data.bounding_volume,
                    &[0x0008_0001],
                    "collision-model bounding-volume link",
                    totals,
                );
                match data.meshes {
                    CollisionModelMeshes::Assembly(id) => {
                        validate_object_link(
                            arena,
                            sections,
                            Some(id),
                            &[],
                            "collision-model assembly link",
                            totals,
                        );
                    }
                    CollisionModelMeshes::Meshes(meshes) => {
                        for mesh in meshes {
                            validate_object_link(
                                arena,
                                sections,
                                Some(mesh.mesh),
                                &[0x0008_0001],
                                "collision-model mesh link",
                                totals,
                            );
                        }
                    }
                }
            }
            Err(_) => record_failure(totals, "collision model data"),
        },
        TABLE_OF_CONTENTS_TYPE_ID => match TableOfContents::parse(bytes) {
            Ok(table) => {
                totals.toc_entries += table.entries.len();
                for entry in table.entries {
                    inventory_toc_identity(&entry, totals);
                    *totals.toc_types.entry(entry.object_type).or_default() += 1;
                    validate_toc_subreference_kind(
                        entry.object,
                        entry.object_type,
                        subreference_kinds,
                        totals,
                    );
                    validate_object_link(
                        arena,
                        sections,
                        Some(entry.object),
                        &[entry.object_type],
                        "TOC object link",
                        totals,
                    );
                }
            }
            Err(_) => record_failure(totals, "table of contents"),
        },
        _ => {}
    }
}

fn inventory_toc_identity(
    entry: &skate2_formats::pegasus::TocEntry,
    totals: &mut SimulationInventory,
) {
    totals
        .toc_entries_by_guid
        .entry(entry.guid)
        .or_default()
        .insert(entry.name.clone());
}

fn inventory_embedded_data(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    bytes: &[u8],
    totals: &mut SimulationInventory,
) {
    let Ok(data) = EmbeddedData::parse(bytes) else {
        record_failure(totals, "embedded data");
        return;
    };
    totals.embedded_data += 1;
    totals.embedded_grind_splines += data.grind_splines().len();
    totals.embedded_grab_splines += data.grab_splines().len();
    for reference in data.grind_splines() {
        validate_object_link(
            arena,
            sections,
            Some(reference.object),
            &[TOC_SPLINE_SUBREFERENCE_TYPE_ID],
            "embedded grind-spline link",
            totals,
        );
    }
    for reference in data.grab_splines() {
        validate_object_link(
            arena,
            sections,
            Some(reference.object),
            &[],
            "embedded grab-spline link",
            totals,
        );
    }
}

fn inventory_dmo_data(bytes: &[u8], totals: &mut SimulationInventory) {
    let Ok(data) = DmoData::parse(bytes) else {
        record_failure(totals, "DMO data");
        return;
    };
    totals.dmo_data += 1;
    totals.dmos += data.dmos.len();
    for dmo in data.dmos {
        totals.dmo_nonzero_serialized_instance_words +=
            usize::from(dmo.serialized_instance_word != 0);
        totals.dmo_local_guids.insert(dmo.local_guid);
        totals.dmo_instance_guids.insert(dmo.instance_guid);
        match dmo_model_ids_from_name(&dmo.name) {
            Some((model_local_guid, external_model_id))
                if external_model_id == dmo.instance_guid =>
            {
                totals.dmo_model_local_guids.insert(model_local_guid);
            }
            Some(_) => record_failure(totals, "DMO name external model ID"),
            None => record_failure(totals, "DMO name model IDs"),
        }
        if let Some(model_name) = model_name_from_serialized_instance(&dmo.name) {
            totals.dmo_model_names.insert(model_name.clone());
            totals
                .dmo_model_names_by_id
                .entry(dmo.instance_guid)
                .or_default()
                .insert(model_name);
        } else {
            record_failure(totals, "DMO model name");
        }
    }
}

fn model_name_from_serialized_instance(name: &str) -> Option<String> {
    let (prefix, _) = name.split_once("_0x")?;
    let model_end = prefix
        .rfind("_DMO")
        .or_else(|| prefix.rfind("_LW"))
        .unwrap_or(prefix.len());
    Some(prefix[..model_end].to_owned())
}

fn dmo_model_ids_from_name(name: &str) -> Option<(u64, u64)> {
    let identity_chain = name.split_once("::")?.0;
    let mut ids = identity_chain.rsplit(':');
    let external_model_id = u64::from_str_radix(ids.next()?.strip_prefix("0x")?, 16).ok()?;
    let model_local_guid = u64::from_str_radix(ids.next()?.strip_prefix("0x")?, 16).ok()?;
    Some((model_local_guid, external_model_id))
}

fn inventory_instance_data(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    bytes: &[u8],
    totals: &mut SimulationInventory,
) {
    let Ok(data) = InstanceData::parse(bytes) else {
        record_failure(totals, "instance data");
        return;
    };
    *totals.instance_data_types.entry(data.type_id).or_default() += data.instances.len();
    totals.instances += data.instances.len();
    for instance in data.instances {
        totals.instance_guids.insert(instance.guid);
        totals.instance_local_guids.insert(instance.local_guid);
        totals
            .instance_guids_by_data_type
            .entry(data.type_id)
            .or_default()
            .insert(instance.guid);
        validate_object_link(
            arena,
            sections,
            instance.render_model,
            &[0x00eb_0001],
            "instance render-model link",
            totals,
        );
        validate_object_link(
            arena,
            sections,
            instance.collision_model,
            &[C_MODEL_DATA_TYPE_ID],
            "instance collision-model link",
            totals,
        );
        validate_object_link(
            arena,
            sections,
            instance.embedded_data,
            &[],
            "instance embedded-data link",
            totals,
        );
    }
}

fn inventory_spline_data(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    subreference_kinds: &[Option<u32>],
    bytes: &[u8],
    totals: &mut SimulationInventory,
) {
    let Ok(data) = SplineData::parse(bytes) else {
        record_failure(totals, "spline data");
        return;
    };
    totals.splines += data.splines.len();
    totals.spline_segments += data.segments.len();
    let grind_asset = GrindAsset::from_spline_data(0, 0, &data);
    for primitive in &grind_asset.primitives {
        if primitive.is_vertical {
            totals.vertical_grind_primitives += 1;
            continue;
        }
        let delta = std::array::from_fn::<_, 3, _>(|axis| {
            primitive.end_point[axis] - primitive.start_point[axis]
        });
        let horizontal_length = delta[0].hypot(delta[2]);
        let plane_normal = [
            delta[2] / horizontal_length,
            0.0,
            -delta[0] / horizontal_length,
        ];
        #[allow(clippy::manual_midpoint)]
        let midpoint: [f32; 3] = std::array::from_fn(|axis| {
            (primitive.start_point[axis] + primitive.end_point[axis]) * 0.5
        });
        let trajectory = BallisticTrajectory {
            initial_position: std::array::from_fn(|axis| midpoint[axis] + plane_normal[axis]),
            initial_velocity: plane_normal.map(|component| -component),
            gravity: [0.0; 3],
            duration: 4.0,
        };
        if consider_grind_primitive(&trajectory, 0.2, -0.2, *primitive, 0).is_some() {
            totals.grind_candidate_probe_hits += 1;
        } else {
            totals.grind_candidate_probe_misses += 1;
            if totals.grind_candidate_miss_samples.len() < 8 {
                totals.grind_candidate_miss_samples.push(*primitive);
            }
        }
    }
    for spline in &data.splines {
        if spline.instance.0 != 0 {
            validate_object_link(
                arena,
                sections,
                Some(spline.instance),
                &[],
                "spline instance link",
                totals,
            );
            if validate_toc_subreference_kind(
                spline.instance,
                TOC_INSTANCE_SUBREFERENCE_TYPE_ID,
                subreference_kinds,
                totals,
            ) {
                totals.spline_instance_links += 1;
            }
        }
        for offset in [spline.head_offset, spline.tail_offset]
            .into_iter()
            .flatten()
        {
            if data.segment_at_object_offset(offset).is_none() {
                record_failure(totals, "spline segment link");
            }
        }
    }
    for segment in &data.segments {
        if segment
            .spline_offset
            .is_none_or(|offset| data.spline_at_object_offset(offset).is_none())
        {
            record_failure(totals, "segment spline link");
        }
        for offset in [segment.previous_offset, segment.next_offset]
            .into_iter()
            .flatten()
        {
            if data.segment_at_object_offset(offset).is_none() {
                record_failure(totals, "segment neighbour link");
            }
        }
    }
}

fn classify_scene_subreferences(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    resources: &[Vec<u8>; 5],
    totals: &mut SimulationInventory,
) -> Vec<Option<u32>> {
    let mut targets = BTreeMap::new();
    for (entry_index, entry) in arena.dictionary.iter().enumerate() {
        let Some(bytes) = arena.object_bytes(resources, entry) else {
            continue;
        };
        let Ok(entry_index) = u32::try_from(entry_index) else {
            continue;
        };
        if entry.type_id == INSTANCE_DATA_TYPE_ID
            && let Ok(data) = InstanceData::parse(bytes)
        {
            for index in 0..data.instances.len() {
                if let Some(offset) = data.instance_object_offset(index) {
                    targets.insert((entry_index, offset), TOC_INSTANCE_SUBREFERENCE_TYPE_ID);
                }
            }
        }
        if entry.type_id == SPLINE_DATA_TYPE_ID
            && let Ok(data) = SplineData::parse(bytes)
        {
            for index in 0..data.splines.len() {
                if let Some(offset) = data.spline_object_offset(index) {
                    targets.insert((entry_index, offset), TOC_SPLINE_SUBREFERENCE_TYPE_ID);
                }
            }
        }
        if entry.type_id == LOCATION_DESC_DATA_TYPE_ID
            && let Ok(data) = LocationDescData::parse(bytes)
        {
            for index in 0..data.locations.len() {
                if let Some(offset) = data.location_object_offset(index) {
                    targets.insert(
                        (entry_index, offset),
                        TOC_LOCATION_DESC_SUBREFERENCE_TYPE_ID,
                    );
                }
            }
        }
        if entry.type_id == TRIGGER_INSTANCE_DATA_TYPE_ID
            && let Ok(data) = TriggerInstanceData::parse(bytes)
        {
            for index in 0..data.instances.len() {
                if let Some(offset) = data.instance_object_offset(index) {
                    targets.insert(
                        (entry_index, offset),
                        TOC_TRIGGER_INSTANCE_SUBREFERENCE_TYPE_ID,
                    );
                }
            }
        }
    }
    sections
        .subreferences
        .iter()
        .map(|subreference| {
            let kind = (subreference.object.section_index() == 0)
                .then(|| {
                    targets
                        .get(&(
                            subreference.object.entry_index(),
                            subreference.object_offset,
                        ))
                        .copied()
                })
                .flatten();
            match kind {
                Some(TOC_INSTANCE_SUBREFERENCE_TYPE_ID) => totals.instance_subreferences += 1,
                Some(TOC_SPLINE_SUBREFERENCE_TYPE_ID) => totals.spline_subreferences += 1,
                Some(TOC_LOCATION_DESC_SUBREFERENCE_TYPE_ID) => {
                    totals.location_subreferences += 1;
                }
                Some(TOC_TRIGGER_INSTANCE_SUBREFERENCE_TYPE_ID) => {
                    totals.trigger_subreferences += 1;
                }
                _ => {}
            }
            kind
        })
        .collect()
}

fn validate_toc_subreference_kind(
    id: ObjectId,
    expected_type: u32,
    kinds: &[Option<u32>],
    totals: &mut SimulationInventory,
) -> bool {
    if id.section_index() != 2 {
        return false;
    }
    let kind = usize::try_from(id.entry_index())
        .ok()
        .and_then(|index| kinds.get(index))
        .copied()
        .flatten();
    if matches!(
        expected_type,
        TOC_INSTANCE_SUBREFERENCE_TYPE_ID
            | TOC_SPLINE_SUBREFERENCE_TYPE_ID
            | TOC_LOCATION_DESC_SUBREFERENCE_TYPE_ID
            | TOC_TRIGGER_INSTANCE_SUBREFERENCE_TYPE_ID
    ) && kind != Some(expected_type)
    {
        record_failure(totals, "typed TOC subreference");
    }
    kind == Some(expected_type)
}

fn validate_object_link(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    id: Option<ObjectId>,
    expected_types: &[u32],
    failure_stage: &'static str,
    totals: &mut SimulationInventory,
) {
    let Some(id) = id else { return };
    *totals
        .object_link_sections
        .entry(id.section_index())
        .or_default() += 1;
    if id.section_index() == 2 {
        validate_subreference_link(arena, sections, id, expected_types, failure_stage, totals);
        return;
    }
    if id.section_index() == 1 {
        totals.null_arena_links += 1;
        return;
    }
    if id.section_index() != 0 {
        totals.external_arena_links += 1;
        if let Some(arena_id) = sections.external_arena_id(id.section_index()) {
            totals.pending_external_links.push(PendingExternalLink {
                arena_id,
                entry_index: id.entry_index(),
                expected_types: expected_types.to_vec(),
                failure_stage,
            });
        }
        return;
    }
    let Some(entry) = arena.local_entry(id) else {
        if totals
            .failures
            .get(failure_stage)
            .copied()
            .unwrap_or_default()
            < 3
        {
            eprintln!(
                "{failure_stage}: local id {:08x} has no dictionary entry ({} entries)",
                id.0,
                arena.dictionary.len()
            );
        }
        record_failure(totals, failure_stage);
        return;
    };
    totals.local_object_links += 1;
    if !expected_types.is_empty() && !expected_types.contains(&entry.type_id) {
        if totals
            .failures
            .get(failure_stage)
            .copied()
            .unwrap_or_default()
            < 3
        {
            eprintln!(
                "{failure_stage}: id {:08x} resolved to {:08x}, expected one of {:?}",
                id.0, entry.type_id, expected_types
            );
        }
        record_failure(totals, failure_stage);
    }
}

fn validate_subreference_link(
    arena: &Rw4Arena,
    sections: &ArenaSections,
    id: ObjectId,
    expected_types: &[u32],
    failure_stage: &'static str,
    totals: &mut SimulationInventory,
) {
    totals.subreference_links += 1;
    let Some(subreference) = usize::try_from(id.entry_index())
        .ok()
        .and_then(|index| sections.subreferences.get(index))
    else {
        record_failure(totals, failure_stage);
        return;
    };
    if subreference.type_id != 0
        && !expected_types.is_empty()
        && !expected_types.contains(&subreference.type_id)
    {
        if totals
            .failures
            .get(failure_stage)
            .copied()
            .unwrap_or_default()
            < 3
        {
            eprintln!(
                "{failure_stage}: subreference {:08x} has type {:08x}, expected {:?}; base {:08x}+{:x}",
                id.0,
                subreference.type_id,
                expected_types,
                subreference.object.0,
                subreference.object_offset
            );
        }
        record_failure(totals, failure_stage);
    }
    if subreference.object.section_index() != 0 {
        return;
    }
    let Some(base) = arena.local_entry(subreference.object) else {
        record_failure(totals, failure_stage);
        return;
    };
    if subreference.object_offset >= base.size {
        record_failure(totals, failure_stage);
    }
}

fn resolve_external_links(totals: &mut SimulationInventory) {
    let pending = std::mem::take(&mut totals.pending_external_links);
    for link in pending {
        let Some(dictionary) = totals.arena_dictionaries.get(&link.arena_id) else {
            totals.unavailable_external_links += 1;
            continue;
        };
        let Some(type_id) = usize::try_from(link.entry_index)
            .ok()
            .and_then(|index| dictionary.get(index))
            .copied()
        else {
            record_failure(totals, link.failure_stage);
            continue;
        };
        totals.resolved_external_links += 1;
        if !link.expected_types.is_empty() && !link.expected_types.contains(&type_id) {
            record_failure(totals, link.failure_stage);
        }
    }
}

fn inventory_clustered_mesh(bytes: &[u8], totals: &mut SimulationInventory) {
    let Ok(mesh) = ClusteredMesh::parse(bytes) else {
        record_failure(totals, "mesh header");
        return;
    };
    totals.meshes += 1;
    if let Ok(tree) = mesh.kd_tree(bytes) {
        totals.kd_branches += u64::from(tree.branch_node_count);
        totals.kd_entries += u64::from(tree.entry_count);
        if tree.branch_nodes(bytes).is_err() {
            record_failure(totals, "KD branch nodes");
        }
        let minimum = [
            mesh.bounding_box_min[0],
            mesh.bounding_box_min[1],
            mesh.bounding_box_min[2],
        ];
        let maximum = [
            mesh.bounding_box_max[0],
            mesh.bounding_box_max[1],
            mesh.bounding_box_max[2],
        ];
        match tree.leaf_ranges_for_aabb(bytes, minimum, maximum) {
            Ok(ranges) => {
                let queried_entries = ranges
                    .iter()
                    .map(|range| u64::from(range.unit_count))
                    .sum::<u64>();
                if queried_entries != u64::from(tree.entry_count) {
                    record_failure(totals, "KD full-bounds query");
                }
                if mesh.triangles_for_leaf_ranges(bytes, &ranges).is_err() {
                    record_failure(totals, "KD leaf unit decode");
                }
            }
            Err(_) => record_failure(totals, "KD full-bounds query"),
        }
    } else {
        record_failure(totals, "KD header");
    }
    let Ok(clusters) = mesh.clusters(bytes) else {
        record_failure(totals, "cluster table/data");
        return;
    };
    totals.clusters += clusters.len();
    for cluster in clusters {
        let mode = match cluster.compression {
            VertexCompression::Uncompressed => "uncompressed",
            VertexCompression::Unsigned16 => "unsigned16",
            VertexCompression::Signed32 => "signed32",
        };
        *totals.compression_modes.entry(mode).or_default() += 1;
        totals.vertices += cluster.vertices.len();
        let tolerance = mesh.vertex_compression_granularity.abs() * 2.0 + 0.01;
        let bad_vertex = cluster.vertices.iter().find(|vertex| {
            vertex
                .position(&mesh)
                .into_iter()
                .enumerate()
                .any(|(axis, value)| {
                    !value.is_finite()
                        || value < mesh.bounding_box_min[axis] - tolerance
                        || value > mesh.bounding_box_max[axis] + tolerance
                })
        });
        if let Some(vertex) = bad_vertex {
            let stage = match cluster.compression {
                VertexCompression::Uncompressed => "vertex bounds uncompressed",
                VertexCompression::Unsigned16 => "vertex bounds unsigned16",
                VertexCompression::Signed32 => "vertex bounds signed32",
            };
            if totals.failures.get(stage).copied().unwrap_or_default() < 3 {
                eprintln!(
                    "{stage}: {:?} -> {:?}, bbox {:?}..{:?}, granularity {}",
                    vertex,
                    vertex.position(&mesh),
                    mesh.bounding_box_min,
                    mesh.bounding_box_max,
                    mesh.vertex_compression_granularity
                );
            }
            record_failure(totals, stage);
        }
        let Ok(units) = cluster.units(&mesh) else {
            record_failure(totals, "cluster units");
            continue;
        };
        totals.units += units.len();
        totals.triangles += units
            .iter()
            .map(|unit| unit.triangle_indices().len())
            .sum::<usize>();
    }
}

fn record_failure(totals: &mut SimulationInventory, stage: &'static str) {
    *totals.failures.entry(stage).or_default() += 1;
}

fn probe_bam(mounts: &MountSet) {
    if let Ok(version) = mounts.read_path("data\\content\\world\\environmentVersion.txt") {
        println!(
            "world environment version: {}",
            String::from_utf8_lossy(&version).trim()
        );
    }
    match mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc") {
        Ok(bytes) => match LocationTable::parse(&bytes) {
            Ok(table) => {
                println!("BAM locations: {}", table.locations.len());
                if let Some(location) = table.find("A_RobsPark") {
                    println!("A_RobsPark position: {:?}", location.position());
                }
            }
            Err(error) => eprintln!("BAM location table: {error}"),
        },
        Err(error) => eprintln!("BAM location table: {error}"),
    }
    for stream in ["Pres", "Sim"] {
        if let Err(error) = probe_bam_stream(mounts, stream) {
            eprintln!("BAM {stream}: {error}");
        }
    }
}

fn probe_bam_stream(mounts: &MountSet, stream: &str) -> Result<(), Box<dyn std::error::Error>> {
    let base = format!("data\\content\\world\\stream\\BAM\\BAM_{stream}");
    let assets = AssetToc::parse(&mounts.read_path(&format!("{base}.xst"))?)?;
    let collections = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm"))?)?;
    let space = CollectionSpace::parse(&mounts.read_path(&format!("{base}.xss"))?)?;
    println!(
        "BAM {stream}: {} assets, {} collections, {} spatial cells; stamps {:016x}/{:016x}/{:016x}",
        assets.assets.len(),
        collections.collections.len(),
        space.collections.len(),
        assets.stamp,
        collections.stamp,
        space.stamp
    );

    let Some(cell) = space.collections.first() else {
        return Ok(());
    };
    let Some(collection) = collections
        .collections
        .iter()
        .find(|collection| collection.id == cell.id)
    else {
        return Ok(());
    };
    println!(
        "BAM {stream} first cell: {}, hull {:?}..{:?}",
        collection.filename, cell.hull_min, cell.hull_max
    );
    let collection_path = format!(
        "data\\content\\world\\stream\\BAM\\{}.xsf",
        collection.filename
    );
    let bytes = mounts.read_path(&collection_path)?;
    let file = StreamFile::parse(&bytes, collection.asset_count)?;
    println!(
        "BAM {stream} first cell blocks: {} ({} bytes)",
        file.assets.len(),
        bytes.len()
    );
    for block in &file.assets {
        probe_stream_asset(&assets, &bytes, block);
    }
    Ok(())
}

fn probe_stream_asset(assets: &AssetToc, bytes: &[u8], block: &StreamAssetBlock) {
    let toc = assets.assets.iter().find(|asset| asset.id == block.id);
    let payload = &bytes[block.data_range.clone()];
    let arena = CompressedArenaHeader::parse(payload);
    let decoded = decode_arena(payload);
    println!(
        "  asset {:016x}: block {}, toc {:?}, arena {:?}, resources {:?}",
        block.id,
        block.stored_size,
        toc.map(|asset| (asset.asset_type, asset.serialized_size)),
        arena.ok().map(|header| header.segments[0]),
        decoded.as_ref().ok().map(|arena| {
            arena.resources.each_ref().map(|resource| {
                (
                    resource.len(),
                    resource
                        .get(..8.min(resource.len()))
                        .unwrap_or(&[])
                        .to_vec(),
                )
            })
        })
    );
    let Ok(decoded) = decoded else { return };
    let Ok(rw4) = Rw4Arena::parse(&decoded.resources) else {
        return;
    };
    let types: Vec<_> = rw4
        .dictionary
        .iter()
        .map(|entry| {
            (
                entry.type_index,
                format!("{:08x}", entry.type_id),
                entry.resource_index,
                entry.object_range.clone(),
            )
        })
        .collect();
    println!(
        "    RW4 arena {:08x}: {} objects, version {:?}/{:?}, types {:?}",
        rw4.id,
        rw4.dictionary.len(),
        String::from_utf8_lossy(&rw4.file_header.major_version),
        String::from_utf8_lossy(&rw4.file_header.minor_version),
        types
    );
    for entry in &rw4.dictionary {
        let Some(bytes) = rw4.object_bytes(&decoded.resources, entry) else {
            continue;
        };
        probe_collision_object(entry.type_id, bytes);
    }
}

fn probe_collision_object(type_id: u32, bytes: &[u8]) {
    match CollisionObjectType::from_type_id(type_id) {
        Some(CollisionObjectType::Volume) => {
            if let Ok(volume) = Volume::parse(bytes) {
                println!(
                    "      collision volume: radius {}, group {}, surface {}, flags {:08x}",
                    volume.radius, volume.group_id, volume.surface_id, volume.flags
                );
            }
        }
        Some(CollisionObjectType::ClusteredMesh) => {
            if let Ok(mesh) = ClusteredMesh::parse(bytes) {
                println!(
                    "      clustered mesh: bbox {:?}..{:?}, {} clusters, {} units, KD {:08x}, data {:08x}",
                    mesh.bounding_box_min,
                    mesh.bounding_box_max,
                    mesh.cluster_count,
                    mesh.unit_count,
                    mesh.kd_tree_offset,
                    mesh.cluster_data_offset
                );
                if let (Ok(tree), Ok(cluster)) = (mesh.kd_tree(bytes), mesh.first_cluster(bytes)) {
                    let positions: Vec<_> = cluster
                        .vertices
                        .iter()
                        .take(4)
                        .map(|vertex| vertex.position(&mesh))
                        .collect();
                    let units = cluster.units(&mesh).unwrap_or_default();
                    let triangles: Vec<_> =
                        units.iter().flat_map(MeshUnit::triangle_indices).collect();
                    let first_triangle = triangles.first().and_then(|indices| {
                        Some([
                            cluster
                                .vertices
                                .get(usize::from(indices[0]))?
                                .position(&mesh),
                            cluster
                                .vertices
                                .get(usize::from(indices[1]))?
                                .position(&mesh),
                            cluster
                                .vertices
                                .get(usize::from(indices[2]))?
                                .position(&mesh),
                        ])
                    });
                    println!(
                        "        KD: {} branches, {} entries; cluster {:?}, {} vertices, {} normals, {} units/{} triangles; first vertices {:?}; first triangle {:?}",
                        tree.branch_node_count,
                        tree.entry_count,
                        cluster.compression,
                        cluster.vertex_count,
                        cluster.normal_count,
                        cluster.unit_count,
                        triangles.len(),
                        positions,
                        first_triangle
                    );
                    if let Some(points) = first_triangle {
                        let center: [f32; 3] = std::array::from_fn(|axis| {
                            (points[0][axis] + points[1][axis] + points[2][axis]) / 3.0
                        });
                        let downward = mesh
                            .thin_segment_intersections(
                                bytes,
                                [center[0], center[1] + 10.0, center[2]],
                                [center[0], center[1] - 10.0, center[2]],
                            )
                            .unwrap_or_default();
                        let upward = mesh
                            .thin_segment_intersections(
                                bytes,
                                [center[0], center[1] - 10.0, center[2]],
                                [center[0], center[1] + 10.0, center[2]],
                            )
                            .unwrap_or_default();
                        println!(
                            "        reference-thin centroid ray: {} down / {} up hits",
                            downward.len(),
                            upward.len()
                        );
                    }
                } else {
                    eprintln!(
                        "        collision substructure parse: KD {:?}, cluster {:?}",
                        mesh.kd_tree(bytes).err(),
                        mesh.first_cluster(bytes).err()
                    );
                }
            }
        }
        _ => {}
    }
}

fn check_data_root() -> ExitCode {
    let Some(root) = env::var_os("SKATE2_GAME_ROOT").map(PathBuf::from) else {
        eprintln!("SKATE2_GAME_ROOT is not set");
        return ExitCode::FAILURE;
    };

    let required = ["data", "data/audio", "data/big", "data/content", "data/db"];
    let missing: Vec<_> = required
        .iter()
        .map(|relative| root.join(relative))
        .filter(|path| !path.exists())
        .collect();

    if missing.is_empty() {
        println!("recognized Skate 2 data root: {}", root.display());
        ExitCode::SUCCESS
    } else {
        for path in missing {
            eprintln!("missing: {}", path.display());
        }
        ExitCode::FAILURE
    }
}

mod selected_character_material;
