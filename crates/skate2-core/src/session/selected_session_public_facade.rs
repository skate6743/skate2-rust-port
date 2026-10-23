//! Opaque host-facing selected session over the one retained core owner.
//!
//! The controlled first-test path remains distinct from native command-3
//! spawn, registration peer/status, PROXY activation and restored camera output.
use super::{ControlledPlayerPolicy, SelectedSession};
use crate::{
    actor_construction::selected_tick::{
        SelectedQueuedTickOutcome, SelectedTickInputDelivery, SelectedTickNumerics,
        SelectedTickRequest, SelectedTickStage,
    },
    actor_face_playback::presentation::frame::{
        RootPositionWTransport,
        selected_runtime::{SelectedPresentationPolicy, SelectedPresentationStage},
    },
    character::{material_admission::CharacterMaterialAdmission, palette::CharacterPalette},
    collection_resident_streamer::CollectionId,
    game::{WorldStreamPoll, WorldStreamSpec},
    game_input_recognizer_runtime_services::GameInputRuntimeServices,
    gesture_trick_mapping::MappingAllocation,
    input_pad::device::PadPlatform,
    juice_trick_input::boot::{JuiceBootPlatform, JuiceHostOption},
    native_skater_random::{NativeRandomGate, SimulationCommandRandom},
    nis::manager::{FrozenFocus, NisCamera},
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    selected_session_marker::{SelectedSessionMarker, SelectedSessionMarkerSources},
    selected_session_terrain::SelectedSessionTerrainError,
    selected_sim_command_queue::{SelectedSimCommandQueue, SimCommandCounters},
    simulation_assembly_backend::constraint_session::{
        ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
        skateboard_ground::{SkateboardGroundFrameInput, SkateboardGroundFrameSchedule},
        update5_managers::GrindManagerFillPolicy,
    },
    simulation_environment_unlock_router::EnvironmentUnlockSeed,
    skateboard_attributes::SkateboardAttributes,
    world_stream_catalog::WorldStreamCatalog,
    world_stream_runtime::{WorldBoundsError, WorldStreamEffects},
};
use skate2_formats::mount::MountSet;
use std::{cell::RefCell, convert::Infallible, fmt::Debug, path::Path, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedLiveStage {
    BootPrefix,
    SourceSelections,
    ControlledPlayer,
    PushAttributes,
    RecoveryGraph,
    PhysicalPair,
    ControlledPresentation,
    SpawnAdjustment,
    TerrainStart,
    TerrainStream,
    TerrainResident,
    CameraRoster,
    InputBind,
    FramePrepare,
    DevicePoll,
    PreupdateInput,
    SessionMarker,
    QueuedFrame,
    Visual,
    /// The main DMO bank load, model registration and initial census.
    LivingWorldDmo,
    /// The career's challenge system.
    Career,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedLivePending {
    BootNotStarted,
    TerrainStream,
    TerrainResident,
    FrameNotReady,
    NoCompletedFrame,
    UnsourcedFramePolicy(&'static str),
    /// Native PlayerUI command-3 timing and matching peer/status are absent.
    NativeSpawnAndRegistration,
    /// Original enabled PROXY admission before BAM is not connected here.
    NativeProxyActivation,
    /// Only the camera producer's provisional projection is exposed below.
    RestoredCameraPresentation,
}

#[derive(Debug)]
pub enum SelectedLiveError {
    Pending(SelectedLivePending),
    AlreadyAttempted,
    AlreadyFailed(SelectedLiveStage),
    Failed {
        stage: SelectedLiveStage,
        detail: String,
    },
}
impl SelectedLiveError {
    fn failed(stage: SelectedLiveStage, cause: impl Debug) -> Self {
        Self::Failed {
            stage,
            detail: format!("{cause:?}"),
        }
    }
}
impl std::fmt::Display for SelectedLiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SelectedLiveError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedGestureAllocation {
    Host,
    Null,
}
impl From<SelectedGestureAllocation> for MappingAllocation {
    fn from(value: SelectedGestureAllocation) -> Self {
        match value {
            SelectedGestureAllocation::Host => Self::Host,
            SelectedGestureAllocation::Null => Self::Null,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedRootWPolicy {
    RequireKnown,
    ReadSkaterOpaque { bits: u32 },
}

#[derive(Clone, Copy, Debug)]
pub enum SelectedSpawnPolicy {
    /// Read the selected authored location from this session's retained assets.
    AuthoredSelectedLocation,
    ExplicitHostTransform([f32; 16]),
}

#[derive(Clone, Copy, Debug)]
pub enum SelectedGravityPolicy {
    /// Read the default skateboard physics layout from this same Vault owner.
    VaultSkateboardDefault,
    ExplicitHostVector([f32; 3]),
}
impl From<SelectedRootWPolicy> for RootPositionWTransport {
    fn from(value: SelectedRootWPolicy) -> Self {
        match value {
            SelectedRootWPolicy::RequireKnown => Self::RequireKnown,
            SelectedRootWPolicy::ReadSkaterOpaque { bits } => Self::ReadSkaterOpaque { bits },
        }
    }
}

/// Every value is supplied by the host. The passing original-data corpus uses
/// one explicit policy set; none of those values is a production default.
pub struct SelectedControlledBootPolicy {
    pub spawn_policy: SelectedSpawnPolicy,
    pub marquee: u64,
    pub actor_flags: u32,
    pub capsule_resource_allocated: bool,
    pub camera_set: String,
    pub terrain_catalog: WorldStreamCatalog,
    pub terrain_spec: WorldStreamSpec,
    pub unlock_seed: EnvironmentUnlockSeed,
    pub collection: CollectionId,
    pub dmo_update_in_parallel: bool,
    pub arena_userdata: u32,
    pub presentation_lod: u8,
    pub presentation_submission_capacity: usize,
    pub presentation_gather_capacity: usize,
    pub presentation_provider: u32,
    pub presentation_root_w: SelectedRootWPolicy,
    pub camera_submit_mask: u32,
    pub gravity_policy: SelectedGravityPolicy,
    pub command_start: SelectedCommandStartPolicy,
}

struct PendingBoot<T> {
    spawn: [f32; 16],
    marquee: u64,
    actor_flags: u32,
    capsule_resource_allocated: bool,
    thread: T,
    camera_set: String,
    presentation: SelectedPresentationPolicy,
    camera_submit_mask: u32,
    terrain_stream_complete: bool,
    terrain_requested: bool,
    command_start: SelectedCommandStartPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedBootProgress {
    TerrainStreamPending,
    TerrainResidentPending,
    /// `SecondaryLoad01` is still loading the main DMO bank.
    DmoBankPending,
    ControlledFrameReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedFrameNumerics {
    ExistingHostApproximation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedCommandStartPolicy {
    /// Explicit native static-field constructors; later reset/seed ownership
    /// is still a separate native scheduling question.
    NativeStaticFields,
}

/// Source-facing scheduler answers for one selected queued frame. These fields
/// mirror the passed corpus; the facade never substitutes its own values.
#[derive(Clone, Copy, Debug)]
pub struct SelectedQueuedFramePolicy {
    pub dt: f32,
    pub lod: i32,
    pub wait_for_bs_job: bool,
    pub can_simulate: bool,
    pub has_focus: bool,
    pub ground_group: u32,
    pub allow_multiple_writes: bool,
    pub spy_flags: u32,
    pub numerics: SelectedFrameNumerics,
    pub physics_flags3: bool,
    pub park_grind_preparation: bool,
    pub exclude_grind_fill: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedQueuedFrameOutcome {
    /// Frontend-only poll: world and simulation owners did not advance.
    Frontend,
    NoCommands {
        update_mask: u32,
        delivered: usize,
    },
    Completed {
        update_mask: u32,
        delivered: usize,
        missing_receivers: usize,
    },
}

/// Borrowed GPU inputs from a completed queued frame of this same session.
/// `provisional_clip_from_world` is the camera producer projection, not the
/// missing native CameraPresentation restore result.
pub struct SelectedVisualSnapshot<'a> {
    /// VDReader's recorded bail fields, restored on the replay timeline.
    pub hom_visual: Option<replay_session::BailVisual>,
    pub minimap: crate::ui::minimap::Frame,
    pub minimap_icons: &'a [crate::ui::minimap::Icon],
    pub marker_hud: crate::ui::phonelist::MarkerHudSnapshot,
    /// Ordered completed scoring updates since the preceding visual snapshot.
    pub scoring_hud_frames: Vec<crate::ui::trick_backend::PhysicsFrame>,
    pub scoring_hud_tuning: crate::ui::trick_display::DisplayTuning,
    /// VDWriter827ff8c8 message presence and amount, retained for this completed tick.
    pub out_of_boundary_amount: Option<f32>,
    /// Completed 60 Hz simulation ticks. This facade publishes whole ticks;
    /// presentation interpolation has no fractional tick in this path yet.
    pub simulation_tick: u32,
    pub stream_focus_position: [f32; 3],
    pub stream_focus_disabled: bool,
    pub stream_xml_enabled: bool,
    pub palette: &'a CharacterPalette,
    pub materials: Arc<std::sync::Mutex<CharacterMaterialAdmission>>,
    /// `SkaterPresentation::GetHipsPos` 0x827e3ef0 for the first player's skater, which
    /// lights it.
    pub skater_hips_position: [f32; 3],
    /// The first player's SkaterPresEntity world `m_BB`, min then max, as
    /// `SceneRenderView::Cull` 0x828501c0 leaves it; its `ShadowView::Cull` reads it.
    pub skater_world_bounds: [[f32; 3]; 2],
    /// The local skater's hips and world `m_BB` (a later party skater on its turn):
    /// `PostFX::RenderShadows_CSM` 0x82855888 fits the cascades' depth to
    /// `GetSkaterPresEntityFromIndex(GetLocalIndex())`, and `RenderMan::UpdateRenderParams`
    /// samples the sun map at its hips.
    pub local_skater_shadow: ([f32; 3], [[f32; 3]; 2]),
    pub provisional_clip_from_world: [f32; 16],
    /// Translation lanes of the same `CameraMan` matrix the clip was built
    /// from; `UpdateRenderParams` 0x82852780 publishes it as `g_vViewPos`.
    pub camera_world_position: [f32; 3],
    /// `CameraPresentation::GetNearPlane` and `GetFarPlane` of the same
    /// projection.
    pub camera_planes: [f32; 2],
    /// Z axis lanes of that matrix (the view direction), which
    /// `RenderShadows_CSM` centres each shadow cascade along.
    pub camera_forward: [f32; 3],
    pub camera_right: [f32; 3],
    pub camera_up: [f32; 3],
    /// Completed RAIN cycles packed by native WorldPresentation::DoSimRestore.
    pub traffic_light_status: [[f32; 4]; 2],
    pub camera_update_counter: u32,
    /// WorldPainter region keys at this camera, or why the lookup failed.
    pub world_painter: Result<WorldPainterKeys, String>,
    /// SkaterPresEntity `i_params[0].x` (+0x870) after DoSimRestore: the alpha
    /// the original character pixel shaders write.
    pub skater_opacity: f32,
    /// SkaterPresEntity `mIsGhosted` (+0x61a): `SkaterPresEntity::Update`
    /// 827e8730 then selects the `ghost` technique.
    pub skater_ghosted: bool,
    /// The skater entity's `m_EntityType`: `eEntTypSkater` (0x122), or
    /// `eEntTypNISChar` (0x822) while a NIS borrows it.
    pub skater_entity_type: u32,
    /// The spawned LivingWorld DMOs, in entity-map order.
    pub dynamic_objects: Vec<crate::SelectedDynamicObjectVisual>,
    pub vehicles: Vec<crate::SelectedVehicleVisual>,
    pub pedestrians: Vec<crate::SelectedPedestrianVisual>,
    /// `CrowdPresentation`'s imposter targets and imposters.
    pub crowd: crate::SelectedCrowdVisual,
    /// `ChaseArrowMan`'s arrows after this tick's front-end update.
    pub chase_arrows: Vec<crate::ui::chase_arrows::Arrow>,
    /// The running NIS's scene characters with their own renderables.
    pub scene_characters: Vec<SelectedSceneCharacterVisual<'a>>,
    /// `SkaterManager`'s skaters after the first, in player index order.
    pub party_skaters: Vec<SelectedSkaterVisual<'a>>,
    /// `VisualDirector`'s challenge indicator records for this completed tick
    /// (`VDWriter::_PrepareChallengeVisualIndicators` 0x827ff138), in a career.
    pub visual_indicators: Option<crate::challenge::visual_director::VisualIndicatorFrame>,
}

/// A pedestrian as `AudioPresentation::DoSimSubmit` 0x827fb2f8 reports it to the audio: its
/// `Pedestrian::GetStateOut`, `PedStateOut::GetAudioCharacter` 0x82e78490 and the distance
/// `GetPedestriansWithinRange` returned, with its pres pedestrian's local-to-world rows 2
/// (facing) and 3 (position), which `SFXCTL_PedestrianPhysics::UpdateParams` 0x8251a2d8 reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioPedestrian {
    /// `LWBasicEntity::GetID`.
    pub id: u32,
    /// `PedStateOut` `mState`, `mLength`, `mTopic` and `mTargetGreetCharacter`.
    pub state: u32,
    pub length: u32,
    pub topic: u32,
    pub target_greet_character: u32,
    /// The `PedStateOut` byte at 0x14: `mbRightFootDown` 0x10, `mbLeftFootDown` 0x20,
    /// `mbWasPlayerCollision` 0x40, `mbIsStopped` 0x80.
    pub flags: u8,
    pub audio_character: u32,
    pub distance: f32,
    /// `None` without a pres pedestrian.
    pub local_to_world: Option<[[f32; 4]; 2]>,
}

/// The front end's `AudioPresentation` (`SelectedLiveSession::audio_front_end`): the frame, the
/// camera's Z axis and translation, and `GlobalData`'s reverb, ambience and emitter keys.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioFrontEnd {
    pub frame: u32,
    pub camera_at: [f32; 3],
    pub camera_position: [f32; 3],
    pub reverb_key: u64,
    pub ambience_key: u64,
    pub emitter_key: u64,
}

/// One skater `AudioPresentation::DoSimSubmit` submits (`SelectedLiveSession::audio_skaters`):
/// its player ID, whether that is the local sim index, and its `PhysOut` values.
#[derive(Clone)]
pub struct AudioSkater {
    pub player_index: u32,
    pub local: bool,
    pub phys_out: crate::SkaterAudioPhysOut,
    /// `Actor::CanSeeSkater` 0x825ed758 of the local skater; false for the local skater itself
    /// or without one.
    pub can_see_player: bool,
    /// An AI skater's `AIOut_Audio` words (`Actor::GetAIOut` 0x825ed0e8).
    pub ai_audio: Option<[u32; 10]>,
}

/// A NIS scene character's renderable (`SceneCharacterManager`), posed by
/// its scene clip this presentation frame.
pub struct SelectedSceneCharacterVisual<'a> {
    pub palette: &'a CharacterPalette,
    /// A `SkaterPresEntity`'s skater materials.
    pub materials: Option<&'a CharacterMaterialAdmission>,
    /// A `ColorizedSkaterPresEntity`'s `i_colorize_red`/`i_colorize_blue`;
    /// its meshes take the living world effects, a Vehicle recipe's the
    /// vehicle effects.
    pub colours: Option<[[f32; 4]; 2]>,
    /// `SceneCharacterAnim::mHipsPos`, which the entity's `GetHipsPos`
    /// callback answers.
    pub hips_position: [f32; 3],
    /// The entity's world `m_BB`, min then max.
    pub world_bounds: [[f32; 3]; 2],
    /// The entity's `m_MatLtoW` (row vectors, the translation last).
    pub root: [f32; 16],
    /// Its `m_localBB`, min then max.
    pub local_bounds: [[f32; 3]; 2],
}

/// A `SkaterPresEntity` of a skater after the first, as `DoSimRestore` leaves it.
pub struct SelectedSkaterVisual<'a> {
    /// `mPlayerName` and `mPlayerId` while `mDrawPlayerName` is set.
    pub name_tag: Option<SkaterNameTag>,
    pub palette: &'a CharacterPalette,
    pub materials: Arc<std::sync::Mutex<CharacterMaterialAdmission>>,
    /// `SkaterPresentation::GetHipsPos` 0x827e3ef0.
    pub hips_position: [f32; 3],
    /// The entity's world `m_BB`, min then max.
    pub world_bounds: [[f32; 3]; 2],
    /// `i_params[0].x` (+0x870).
    pub opacity: f32,
    /// `mIsGhosted` (+0x61a).
    pub ghosted: bool,
    /// `m_EntityType`: `eEntTypSkater` (0x122).
    pub entity_type: u32,
    /// Its `m_Recipe` is the player's (`SkaterPresentation::GetPlayerRecipeID` 0x827dd270).
    pub players_recipe: bool,
    /// An online challenge's player that a player waiting in its pre-game lobby watches (PC
    /// port addition): drawn as a ghost at its `opacity`, with no deck shadow.
    pub lobby_ghost: bool,
}

/// The name `SkaterPresEntity::DrawName` (0x827d7fe0) draws over a skater: its constructor
/// (0x827e5740) copies the name of the online game's player whose peer the skater is and draws
/// it when that peer is not the local user's. `index` is the skater's index (`mPlayerId`, which
/// `DoSimRestore` sets), whose `kOnlineColours` entry colours it, as the player's row of the
/// leaderboard HUD is coloured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkaterNameTag {
    pub name: String,
    pub index: u8,
}

/// `VisualDirector::VD_WorldPainterInfo`, as
/// `VDWriter::_PrepareWorldPainterData` 0x827fd200 fills it: one
/// `LayerMan::GetAttribData` key per layer at the game camera's (x, z). A
/// zero key is a miss; `RenderMan::UpdateWorldPainterAttribs` 0x82848ba8
/// then blends toward the collection defaults.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorldPainterKeys {
    /// `eFog` (15): a `material_fog` collection.
    pub fog: u64,
    /// `eColorCube` (11): a `colour_cube` collection.
    pub colour_cube: u64,
    /// `eSky` (16): a `material_sky` collection.
    pub sky: u64,
    /// `eExposure` (17): a `rendering` auto-exposure collection.
    pub exposure: u64,
    /// Layer 18: a `post_fx` bloom collection.
    pub bloom: u64,
    /// `eDistricts` (4): a `worldpainter_district` collection.
    pub district: u64,
}

/// One car of `AudioPresentation::DoSimSubmit` 0x827fb2f8's `VehicleData` list: its
/// `tVehicleStateOut`, the range query's distance and its presentation entity's matrix.
#[derive(Clone, Copy, Debug)]
pub struct AudioVehicle {
    /// `tVehicleStateOut::GetID`.
    pub id: u32,
    /// `mAcceleration`, `mSpeed`.
    pub acceleration: f32,
    pub speed: f32,
    /// `GetVehiclesWithinRange`'s distance.
    pub distance: f32,
    /// `mHornState` (`eHornState`).
    pub horn_state: i32,
    /// `IsSkidding`.
    pub skidding: bool,
    /// `GetEngineKey` 0x82d1e6d8: the collection key at byte 8 of the car's
    /// `livingworld_vehicle_characteristics` layout.
    pub engine_key: u64,
    /// `cPresVehicle::GetLocalToWorldMatrix` rows; `None` until `CreatePresVehicle` has the car's
    /// recipe.
    pub matrix: Option<[[f32; 4]; 4]>,
}

/// A dynamic object as `AudioPresentation::DoSimSubmit` 0x827fb2f8 reports it to the audio
/// (`DynamicData`), with its `cPresDynamicObject` bounds centre, which
/// `SFXCTL_DynamicObjectPhysics::UpdateParams` 0x8253e4c8 reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioDynamicObject {
    /// `DynamicObjectStateOut::GetID`.
    pub id: u32,
    /// `GetVelocity`: the core data velocity (+0x90).
    pub velocity: [f32; 4],
    /// `grabbedObject`: the local skater grabs and its grab spline belongs to this object.
    pub grabbed: bool,
    /// `GetCharacteristic()->GetCollection()`: the `livingworld_dynamicobject_characteristics`
    /// key.
    pub characteristic_key: u64,
    /// `GetPartAngularVelocity(0)`: Part 0's `RigidBody::mOmega`; W is padding.
    pub angular_velocity: [f32; 4],
    /// Part 0's strongest contact: its `mCollisionForce` and `mContactAudioSurfaceType`. `None`
    /// without contacts, where the record has force 0 and is not on the ground.
    pub contact: Option<(f32, u32)>,
    /// |`GetTransform` row 1 . (0, 1, 0)| is not at least 0.3.
    pub on_side: bool,
    /// `PresentationEntity::GetBoundsCenter` 0x82e3dd30 (`m_globalBC`).
    pub bounds_center: [f32; 4],
}

/// Opaque owner. `T` is the caller's animation thread answer and `C` is the
/// caller's Juice connection type; neither is chosen by the core facade.
pub struct SelectedLiveSession<T, C> {
    inner: SelectedSession<T, C>,
    boot_attempted: bool,
    pending_boot: Option<PendingBoot<T>>,
    ready: bool,
    last_step_completed: bool,
    pause_menu: crate::ui::pause_menu::PauseMenuController,
    options_menu: crate::ui::options::Controller,
    /// `FrontEndState_SkateProfile` (FE state 45), which the `StartHub` pushes.
    skate_profile: crate::ui::skate_profile::Controller,
    main_menu_assets: Option<crate::ui::main_menu::Presentation>,
    main_menu: Option<crate::ui::main_menu::Controller>,
    stereoscopy: bool,
    main_menu_pending: bool,
    /// PC: Exit to Desktop was selected; the window closes once the fade covers.
    exit_to_desktop: bool,
    /// PC: what the main menu asked of the app's updater, not yet taken.
    update_request: Option<crate::ui::update_prompt::UpdateRequest>,
    map_menu: crate::ui::map_menu::MapMenuController,
    editor: Option<crate::edit_skater_session::EditSkaterSession>,
    /// The sound requests Edit Skater left when it closed.
    editor_sounds: Vec<crate::ui::FeSoundRequest>,
    frontend_fade: crate::ui::frontend_fade::FrontendFade,
    editor_enter_pending: bool,
    frontend_reveal_pending: bool,
    editor_outro_complete: bool,
    map_travel: Option<[f32; 16]>,
    map_travel_presentation_ready: bool,
    map_travel_dispatched: bool,
    map_travel_generation: Option<u32>,
    map_travel_ready_seen: bool,
    /// `SKATE2_TRACE_TRAVEL`: what the map travel last waited for.
    map_travel_gate: &'static str,
    completed_frames: u32,
    out_of_boundary_amount: Option<f32>,
    failed_stage: Option<SelectedLiveStage>,
    career: Option<career::CareerSession>,
    /// Host GetLocalTime provider, retained across game loads for statistics
    /// and HOMResultsData::Reset (0x825698b8). Supplied before the first career.
    host_calendar: Option<fn() -> [u16; 6]>,
    /// `DataSysMan`'s recorded frames by frame number, as the challenge replay recorder
    /// holds them.
    replay_frames: std::collections::BTreeMap<u32, Arc<replay_session::ReplayFrame>>,
    /// A replay-controlled NIS's `SceneReplayTimeController`.
    replay_clock: Option<replay_session::ReplayClock>,
    /// The presentations a replay-controlled NIS restored from the recorded frames.
    replay_restorer: crate::presentation_restore::PresentationRestorer,
    freeroam_replay: crate::challenge::replay::ReplaySystem,
    freeroam_replay_frame: u32,
    replay_editor: Option<replay_editor_session::ReplayEditorSession>,
    trick_guide: Option<trick_guide_session::TrickGuideSession>,
    trick_guide_pending: bool,
    replay_editor_pending: bool,
    replay_file_pending:Option<Vec<u8>>,
    replay_visual_focuses:Option<[crate::streaming::replay_focus::VisualFocus;2]>,
    /// The career's autosaves being written.
    career_saves: career::CareerSaves,
    /// New Career: the career starts when Edit Skater closes.
    career_after_editor: bool,
    /// Continue Career: the career starts with the next simulation update.
    career_start_pending: bool,
    /// New Career: the career's game load holds its updates until the skater has spawned at
    /// the start location.
    career_spawn_pending: bool,
    /// Free skate's front end and `sGameInfo` choices.
    freeskate: freeskate::FreeSkate,
    /// Party play's front end.
    party_play: party_play_session::PartyPlay,
    /// The profile's settings and music player.
    audio: music_player_session::ProfileAudio,
    /// The travel is a game start, whose skater stands on the board.
    map_travel_on_board: bool,
    /// `StateGameUnload` ran since the presentation last asked.
    game_unloaded: bool,
    /// The last HUD update of a challenge system that went outside a game unload, until the
    /// presentation takes it.
    ended_career_hud: Option<CareerHud>,
    /// The last game's `cChallengeDynamicHullManager::GameUnload` asked for unloads that the next
    /// game's load waits for.
    dynamic_hulls_unloading: bool,
    /// The world behind the main menu, with its AI skaters.
    frontend_world: frontend_world::FrontendWorld,
    /// `FrontEndManager::mMoviePlayer` with FE state 28, and the new career's intro states.
    movie: movie_session::MovieSession,
    /// The renderer's presentation stream, which `IsAreaLoaded` reads for the presentation focus.
    presentation_area: Option<crate::streaming::area_loaded::PresentationStreamArea>,
    /// The connection to the dedicated server and the online game.
    online: online_session::Online,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
#[allow(dead_code, clippy::struct_excessive_bools)]
pub(crate) struct ControlledWipeoutRecoveryObservation {
    pub(crate) time_in_state: f32,
    pub(crate) over: bool,
    pub(crate) time_settled: f32,
    pub(crate) prevent_respawn: bool,
    pub(crate) ignore_reset_button: bool,
    pub(crate) reset_ever_pressed: bool,
    pub(crate) enough_time_for_reset: bool,
    pub(crate) auto_teleport_pending: bool,
    pub(crate) time_until_teleport: f32,
    pub(crate) request_teleport: bool,
}

/// `SKATE2_TRACE_TRAVEL`: the gate a map travel waits at, when it changes.
fn trace_travel(last: &mut &'static str, gate: &'static str) {
    if *last != gate {
        *last = gate;
        if std::env::var_os("SKATE2_TRACE_TRAVEL").is_some() {
            eprintln!("map travel: waiting for {gate}");
        }
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// BAM_Pres carries navigation used by Simulation::RegArena kind23.
    /// Called between queued frames, with the installed visual collection owners.
    pub fn sync_presentation_navigation(&self, plans: &[crate::presentation::PresentationNavigationArena]) -> Result<(), String> {
        let world = self.inner.world.as_ref().ok_or("navigation world absent")?;
        let terrain = self.inner.terrain.as_ref().ok_or("navigation terrain absent")?
            .terrain_for(&world.world).map_err(|e|format!("navigation terrain: {e:?}"))?;
        terrain.assets.sync_presentation_navigation(plans, &terrain.mesh, |tile| {
            let world = world.world.lock().map_err(|_| "navigation physical world lock")?;
            let mut living = world.grind_services.living_world().lock()
                .map_err(|_| "navigation LivingWorld lock")?;
            let roster = living.pedestrian_entities.as_mut().ok_or("navigation pedestrian roster absent")?;
            for id in roster.ids() {
                let entity = roster.get_mut(id).ok_or("navigation pedestrian disappeared")?;
                entity.nav_rig.as_mut().ok_or("navigation pedestrian rig absent")?.mesh_info.release_tile(tile);
                entity.mesh_navigation.as_mut().ok_or("navigation pedestrian path absent")?.release_tile(tile);
            }
            Ok(())
        })
    }
    #[must_use]
    pub const fn native_gaps() -> [SelectedLivePending; 3] {
        [
            SelectedLivePending::NativeSpawnAndRegistration,
            SelectedLivePending::NativeProxyActivation,
            SelectedLivePending::RestoredCameraPresentation,
        ]
    }

    #[must_use]
    pub fn new(mounts: Arc<MountSet>, data_root: &Path) -> Self {
        Self {
            inner: SelectedSession::new(mounts, data_root),
            boot_attempted: false,
            pending_boot: None,
            ready: false,
            last_step_completed: false,
            pause_menu: crate::ui::pause_menu::PauseMenuController::default(),
            options_menu: Default::default(),
            skate_profile: Default::default(),
            main_menu_assets: None,
            main_menu: None,
            stereoscopy: false,
            main_menu_pending: false,
            exit_to_desktop: false,
            update_request: None,
            map_menu: crate::ui::map_menu::MapMenuController::default(),
            editor: None,
            editor_sounds: Vec::new(),
            frontend_fade: Default::default(),
            editor_enter_pending: false,
            frontend_reveal_pending: false,
            editor_outro_complete: false,
            map_travel: None,
            map_travel_presentation_ready: false,
            map_travel_dispatched: false,
            map_travel_generation: None,
            map_travel_ready_seen: false,
            map_travel_gate: "",
            completed_frames: 0,
            out_of_boundary_amount: None,
            failed_stage: None,
            career: None,
            host_calendar: None,
            replay_frames: std::collections::BTreeMap::new(),
            replay_clock: None,
            replay_restorer: crate::presentation_restore::PresentationRestorer::new(),
            freeroam_replay: Default::default(),
            freeroam_replay_frame: 0,
            replay_editor: None,
            trick_guide: None,
            trick_guide_pending: false,
            replay_editor_pending: false,
            replay_file_pending:None,
            replay_visual_focuses:None,
            career_saves: career::CareerSaves::default(),
            career_after_editor: false,
            career_start_pending: false,
            career_spawn_pending: false,
            freeskate: Default::default(),
            party_play: Default::default(),
            audio: Default::default(),
            map_travel_on_board: false,
            game_unloaded: false,
            ended_career_hud: None,
            dynamic_hulls_unloading: false,
            frontend_world: Default::default(),
            movie: Default::default(),
            presentation_area: None,
            online: Default::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn controlled_player_and_pair_present_for_test(&self) -> (bool, bool) {
        (self.inner.player.is_some(), self.inner.physical.is_some())
    }

    #[cfg(test)]
    pub(crate) fn controlled_initial_body_forces_for_test(&self) -> ([f32; 3], Vec<[f32; 3]>) {
        let world = self
            .inner
            .world
            .as_ref()
            .expect("selected world")
            .world
            .lock()
            .unwrap();
        let snapshot = world.backend().rigid_body_snapshot();
        let forces = snapshot
            .active_order
            .iter()
            .take(7)
            .map(|id| snapshot.slots[id.index() as usize].force)
            .collect();
        (snapshot.gravity, forces)
    }

    #[cfg(test)]
    pub(crate) fn controlled_action_observation_for_test(
        &self,
        pattern_name: &[u8],
    ) -> (u16, [Option<u32>; 3]) {
        let actor = self
            .inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor");
        let prefix = actor.prefix().unwrap();
        let action = prefix.graphs().unwrap().action().unwrap();
        let intents = action.intents.as_ref().expect("Action intents");
        let name = crate::actor_graph_construction::ActorGraphIntents::native_name;
        let bits = |label: &[u8]| intents.get(&name(label)).map(f32::to_bits);
        let state = action
            .controller
            .as_ref()
            .unwrap()
            .generated_action()
            .unwrap()
            .lock()
            .unwrap()
            .current_state;
        (
            state,
            [bits(b"Trick"), bits(pattern_name), bits(b"HoldPattern")],
        )
    }

    #[cfg(test)]
    pub(crate) fn controlled_motion_observation_for_test(
        &self,
        intent_name: &[u8],
    ) -> (u16, [Option<u32>; 3]) {
        let actor = self
            .inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor");
        let prefix = actor.prefix().unwrap();
        let motion = prefix.graphs().unwrap().motion().unwrap();
        let intents = motion.intents.as_ref().expect("Motion intents");
        let name = crate::actor_graph_construction::ActorGraphIntents::native_name;
        let bits = |label: &[u8]| intents.get(&name(label)).map(f32::to_bits);
        let state = motion
            .controller
            .as_ref()
            .unwrap()
            .generated_motion()
            .unwrap()
            .lock()
            .unwrap()
            .current_state;
        (state, [bits(b"Trick"), bits(intent_name), bits(b"Pushing")])
    }

    #[cfg(test)]
    pub(crate) fn controlled_tree_attribute_for_test(
        &self,
        name: &str,
    ) -> Option<skate2_formats::andale::AndaleRuntimeAttribute> {
        use crate::dynamic_state_graph_controller::DynamicStateGraphContext;
        let actor = self
            .inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor");
        let context = {
            let prefix = actor.prefix().unwrap();
            let wrapper = prefix
                .graphs()
                .unwrap()
                .motion()
                .unwrap()
                .controller
                .as_ref()
                .unwrap();
            wrapper.context().copy_for_behaviour()
        };
        actor
            .motion_runtime_tree_attribute(
                &context,
                crate::play_animation::FastString30::encode(name).unwrap(),
            )
            .unwrap()
    }

    #[cfg(test)]
    pub(crate) fn controlled_physical_state_for_test(
        &self,
    ) -> crate::physical_player_physics_state::CurrentPhysicalState {
        self.inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor")
            .cached_physical_allocation()
            .unwrap()
            .owner()
            .physics_state()
            .lock()
            .unwrap()
            .current
    }

    #[cfg(test)]
    pub(crate) fn controlled_wipeout_recovery_for_test(
        &self,
    ) -> Option<ControlledWipeoutRecoveryObservation> {
        let physical = self
            .inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor")
            .cached_physical_allocation()
            .unwrap();
        let owner = physical.owner();
        let state = owner.physics_state().lock().unwrap();
        let fields = &state.wipeout_ground.as_ref()?.fields;
        let observation = ControlledWipeoutRecoveryObservation {
            time_in_state: fields.time_in_state,
            over: fields.over,
            time_settled: fields.time_settled,
            prevent_respawn: fields.prevent_respawn,
            ignore_reset_button: fields.ignore_reset_button,
            reset_ever_pressed: fields.reset_ever_pressed,
            enough_time_for_reset: fields.enough_time_for_reset,
            auto_teleport_pending: fields.auto_teleport_pending,
            time_until_teleport: fields.time_until_teleport,
            request_teleport: fields.request_teleport,
        };
        Some(observation)
    }

    #[cfg(test)]
    pub(crate) fn controlled_teleporting_entry_for_test(
        &self,
    ) -> Option<(
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        bool,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        bool,
    )> {
        use crate::physical_player_assembly::remaining_states::{StateKind, StatePayload};
        let physical = self
            .inner
            .player
            .as_ref()?
            .selected
            .actor
            .as_ref()?
            .cached_physical_allocation()
            .ok()?;
        let owner = physical.owner();
        let state = owner.physics_state().lock().ok()?;
        if state.current != crate::physical_player_physics_state::CurrentPhysicalState::Teleporting
        {
            return None;
        }
        let prevent = state.wipeout_ground.as_ref()?.fields.prevent_respawn;
        let processed = Arc::clone(owner.reckoning().lock().ok()?.processed());
        let processed = processed.lock().ok()?;
        let fields = processed.physics_state_fields();
        let states = owner.remaining_states().lock().ok()?;
        let Some(StatePayload::Teleporting(teleporting)) = states
            .states
            .get(&StateKind::Teleporting)
            .filter(|state| state.published)
            .and_then(|state| state.value.as_ref())
        else {
            return None;
        };
        Some((
            fields.last,
            fields.last_category,
            fields.next,
            fields.current,
            prevent,
            teleporting.teleport_to_on_board,
            teleporting.data_acquired,
            teleporting.teleport_ready,
            teleporting.reset_transform.is_none(),
        ))
    }

    /// Independent native side effects of WipeoutGround.Exit and Update2's
    /// success-only request-clear suffix, sampled across one scripted frame.
    #[cfg(test)]
    pub(crate) fn controlled_wipeout_exit_effects_for_test(&self) -> (bool, u32) {
        let pair = self
            .inner
            .physical
            .as_ref()
            .expect("selected physical pair");
        let free_wheel = self
            .inner
            .world_handle()
            .expect("selected world")
            .lock()
            .unwrap()
            .board_construction(&pair.pair.board)
            .unwrap()
            .expect("selected board construction")
            .post_fields()
            .expect("constructed board")
            .free_wheel_enabled;
        let actor = self
            .inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor");
        let physical = actor.cached_physical_allocation().unwrap();
        let clears = physical
            .owner()
            .wipeout()
            .lock()
            .unwrap()
            .clear_calls_for_test();
        (free_wheel, clears)
    }

    #[cfg(test)]
    pub(crate) fn controlled_board_kinematics_for_test(&self) -> ([f32; 3], f32, f32) {
        self.diagnostic_board_kinematics()
            .expect("completed selected board kinematics")
    }

    #[cfg(test)]
    pub(crate) fn controlled_skater_position_for_test(&self) -> [f32; 3] {
        let skeleton = &self.inner.physical.as_ref().unwrap().pair.skeleton;
        let world = self.inner.world_handle().unwrap().lock().unwrap();
        world
            .skeleton(skeleton)
            .unwrap()
            .root_transforms()
            .animation_to_world()
            .columns()[3]
            .xyz
    }

    /// Observe the existing physics and board owners after one completed
    /// frame, without deriving a turn from the input sample.
    #[cfg(test)]
    pub(crate) fn controlled_board_turn_for_test(
        &self,
    ) -> Option<(f32, f32, f32, f32, f32, [f32; 16])> {
        use crate::skateboard_steering::calculate::{SteeringCalculationInput, SteeringScalar};
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let owner = physical.owner();
        let (turn, hard_turn, balance, spin) = {
            let reckoning = owner.reckoning().lock().ok()?;
            let processed = reckoning.processed().lock().ok()?;
            (
                processed.scalar(SteeringScalar::Turn).ok()?,
                processed.scalar(SteeringScalar::HardTurn).ok()?,
                processed.scalar(SteeringScalar::Balance).ok()?,
                processed.jump_events().spin()?,
            )
        };
        let tilt = owner
            .phys_out()
            .lock()
            .ok()?
            .try_camera_motion_output()
            .ok()?
            .deck_tilt;
        let board = &self.inner.physical.as_ref()?.pair.board;
        let world = self.inner.world_handle()?.lock().ok()?;
        let transform = world
            .board(board)
            .ok()?
            .try_live_part_transform_host(world.backend(), 6)
            .ok()?;
        Some((turn, hard_turn, balance, spin, tilt, transform))
    }

    #[cfg(test)]
    pub(crate) fn controlled_truck_turn_for_test(
        &self,
    ) -> Option<(Option<u32>, [f32; 4], Vec<[f32; 16]>)> {
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let flags = physical
            .owner()
            .reckoning()
            .lock()
            .ok()?
            .processed()
            .lock()
            .ok()?
            .known_flags_8c8();
        let board = &self.inner.physical.as_ref()?.pair.board;
        let mut world = self.inner.world_handle()?.lock().ok()?;
        let state = world
            .try_with_constructed_board_truck_drives(board, |drives, _| {
                drives.turn_state_for_test()
            })
            .ok()?;
        let parts = (0..7)
            .map(|part| {
                world
                    .board(board)
                    .ok()?
                    .try_live_part_transform_host(world.backend(), part)
                    .ok()
            })
            .collect::<Option<Vec<_>>>()?;
        Some((flags, state, parts))
    }

    #[cfg(test)]
    pub(crate) fn controlled_runout_request_for_test(&self) -> (bool, u32, bool, bool) {
        let actor = self
            .inner
            .player
            .as_ref()
            .unwrap()
            .selected
            .actor
            .as_ref()
            .unwrap();
        let physical = actor.cached_physical_allocation().unwrap();
        let runout = physical
            .owner()
            .phys_out()
            .lock()
            .unwrap()
            .try_state_output()
            .unwrap()
            .physics_requests_runout;
        let wipeout = physical.owner().wipeout().lock().unwrap();
        (
            runout,
            wipeout.try_request_count().unwrap(),
            wipeout
                .try_is_requested(crate::wipeout::WipeoutType::ALL[2])
                .unwrap(),
            wipeout
                .try_is_requested(crate::wipeout::WipeoutType::ALL[24])
                .unwrap(),
        )
    }

    #[cfg(test)]
    pub(crate) fn controlled_offboard_processed_flags_for_test(
        &self,
    ) -> (Option<u32>, Option<u32>) {
        let actor = self
            .inner
            .player
            .as_ref()
            .unwrap()
            .selected
            .actor
            .as_ref()
            .unwrap();
        let physical = actor.cached_physical_allocation().unwrap();
        let processed =
            std::sync::Arc::clone(physical.owner().reckoning().lock().unwrap().processed());
        let processed = processed.lock().unwrap();
        (
            processed.physics_state_fields().flags_8cc,
            processed.physics_state_fields().flags_8d0,
        )
    }

    #[cfg(test)]
    pub(crate) fn controlled_current_animation_for_test(
        &self,
    ) -> (
        Option<usize>,
        Option<crate::actor_animation_construction::blend_anim::BodyAnimationReference>,
    ) {
        let actor = self
            .inner
            .player
            .as_ref()
            .unwrap()
            .selected
            .actor
            .as_ref()
            .unwrap();
        let owner = actor.published_camera_animation().unwrap().unwrap();
        let animation = owner.lock().unwrap();
        (
            animation
                .initialization()
                .unwrap()
                .channels
                .as_ref()
                .unwrap()
                .root,
            animation.base().unwrap().body_root.clone(),
        )
    }

    #[cfg(test)]
    pub(crate) fn controlled_grab_busy_for_test(&self) -> Option<[i32; 2]> {
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let prefix = actor.prefix().ok()?;
        Some(prefix.graphs()?.motion()?.state.as_ref()?.grab_busy)
    }

    /// Source-chain snapshot for the original-data push replay. Values are
    /// retained owner lanes; no force or speed is inferred from button input.
    #[cfg(test)]
    pub(crate) fn controlled_push_chain_for_test(
        &self,
    ) -> Result<([u32; 8], (u32, u32), u32, Vec<[u32; 3]>), &'static str> {
        let actor = self
            .inner
            .player
            .as_ref()
            .ok_or("selected player")?
            .selected
            .actor
            .as_ref()
            .ok_or("selected Actor")?;
        let push = actor
            .prefix()
            .map_err(|_| "Actor prefix")?
            .graphs()
            .and_then(|graphs| graphs.motion())
            .and_then(|motion| motion.state.as_ref())
            .ok_or("retained Motion PushState")?
            .push
            .map(f32::to_bits);
        if push[1] == 0 && std::env::var_os("SKATE2_TRACE_PUSH_FORCE").is_some() {
            eprintln!(
                "push retained clip measurements: {:?}",
                actor
                    .push_shared_attributes()
                    .map_err(|_| "push shared attributes")?
            );
        }
        let physical = actor
            .cached_physical_allocation()
            .map_err(|_| "selected physical allocation")?;
        let processed = Arc::clone(
            physical
                .owner()
                .reckoning()
                .lock()
                .map_err(|_| "Reckoning lock")?
                .processed(),
        );
        let (flags, push_speed) = {
            let processed = processed.lock().map_err(|_| "Processed lock")?;
            let flags = processed.known_flags_8c4();
            let push_speed = processed
                .animation_attribute_fields()
                .push_speed
                .ok_or("Processed animation push speed940")?
                .to_bits();
            (flags, push_speed)
        };
        let pair = self
            .inner
            .physical
            .as_ref()
            .ok_or("selected physical pair")?;
        let queued_push = self
            .inner
            .world_handle()
            .ok_or("selected world")?
            .lock()
            .map_err(|_| "world lock")?
            .ground_force_queue_for_test(&pair.pair.board)
            .into_iter()
            .filter(|record| record.kind == 3)
            .map(|record| record.force.world_space_force.xyz.map(f32::to_bits))
            .collect();
        Ok((push, flags, push_speed, queued_push))
    }

    #[cfg(test)]
    pub(crate) fn controlled_jump_gate_for_test(
        &self,
    ) -> ((u32, u32), Option<(Option<bool>, Option<bool>)>) {
        use crate::physical_player_assembly::remaining_states::{StateKind, StatePayload};
        let actor = self
            .inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor");
        let physical = actor.cached_physical_allocation().unwrap();
        let owner = physical.owner();
        let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
        let flags = processed.lock().unwrap().known_flags_8c4();
        let states = owner.remaining_states().lock().unwrap();
        let ground_animation = states
            .states
            .get(&StateKind::GroundAnimation)
            .and_then(|state| state.value.as_ref())
            .and_then(|state| match state {
                StatePayload::GroundAnimation(state) => Some((
                    state.jump_info.as_ref().map(|info| info.do_jump),
                    state.jumped_this_frame,
                )),
                _ => None,
            });
        (flags, ground_animation)
    }

    #[cfg(test)]
    pub(crate) fn controlled_relative_stance_for_test(&self) -> u32 {
        self.inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .actor
            .as_ref()
            .expect("selected Actor")
            .retained_animation()
            .unwrap()
            .expect("retained animation")
            .lock()
            .unwrap()
            .player()
            .expect("SkaterAnim player")
            .relative_stance
    }

    /// Authored right-stick points from the same loaded matcher used by the
    /// controlled frame. The diagnostic window uses this for input replay.
    pub fn controlled_right_pattern_points(&self, name: &str) -> Vec<[f32; 2]> {
        self.inner
            .player
            .as_ref()
            .expect("selected player")
            .selected
            .input
            .original_right_pattern(name)
            .points()
            .iter()
            .rev()
            .copied()
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn controlled_right_pattern_points_for_test(&self, name: &str) -> Vec<[f32; 2]> {
        self.controlled_right_pattern_points(name)
    }

    /// Begin the controlled path with the real pad, Juice answer, Actor thread
    /// callback, and terrain source. Physical allocation collaborators arrive
    /// on a later poll, after this same world's terrain is resident.
    #[expect(
        clippy::too_many_arguments,
        reason = "distinct host services are deliberately explicit"
    )]
    pub fn try_begin_controlled<P, J, E>(
        &mut self,
        pad: &mut P,
        juice: &mut J,
        juice_ini: Option<&[u8]>,
        juice_option: JuiceHostOption,
        thread: T,
        effects: E,
        policy: SelectedControlledBootPolicy,
    ) -> Result<SelectedBootProgress, SelectedLiveError>
    where
        P: PadPlatform,
        J: JuiceBootPlatform<Connection = C>,
        E: WorldStreamEffects + Send + 'static,
        P::Error: Debug,
        J::Error: Debug,
    {
        use SelectedLiveStage as Stage;
        if self.boot_attempted {
            return Err(SelectedLiveError::AlreadyAttempted);
        }
        self.boot_attempted = true;
        let path = crate::edit_skater_profile::EditSkaterProfileStore::default_path()
            .map_err(|e| SelectedLiveError::failed(Stage::BootPrefix, e))?;
        let profile = crate::edit_skater_profile::EditSkaterProfileStore::open(path)
            .map_err(|e| SelectedLiveError::failed(Stage::BootPrefix, e))?;
        self.options_menu.load(profile.load_display_settings()
            .map_err(|e| SelectedLiveError::failed(Stage::BootPrefix, e))?);
        self.options_menu.load_lobby_chat(crate::ui::online_config::OnlineConfig::load().show_lobby_chat);
        self.audio.settings = profile.load_settings().map_err(|e| SelectedLiveError::failed(Stage::BootPrefix, e))?;
        self.audio.new_profile = profile.is_new_profile().map_err(|e| SelectedLiveError::failed(Stage::BootPrefix, e))?;
        let SelectedControlledBootPolicy {
            spawn_policy,
            marquee,
            actor_flags,
            capsule_resource_allocated,
            camera_set,
            terrain_catalog,
            terrain_spec,
            unlock_seed,
            collection,
            dmo_update_in_parallel,
            arena_userdata,
            presentation_lod,
            presentation_submission_capacity,
            presentation_gather_capacity,
            presentation_provider,
            presentation_root_w,
            camera_submit_mask,
            gravity_policy,
            command_start,
        } = policy;
        self.inner
            .try_boot_prefix(pad, juice, juice_ini, juice_option)
            .map_err(|e| {
                self.failed_stage = Some(Stage::BootPrefix);
                SelectedLiveError::failed(Stage::BootPrefix, e)
            })?;
        // Island construction installs Vault gravity before AddRigidBody
        // copies it into each new Active body's initial force. Frame setup is
        // too late: changing only the pool field would leave the first force
        // at Simulation::Initialize's -600 rather than the source selection.
        let gravity = match gravity_policy {
            SelectedGravityPolicy::ExplicitHostVector(vector) => vector,
            SelectedGravityPolicy::VaultSkateboardDefault => {
                let assets = self
                    .inner
                    .assets
                    .as_ref()
                    .expect("retained original assets");
                let layout = assets
                    .vault
                    .physics(SkateboardAttributes::DEFAULT_COLLECTION)
                    .try_physics_layout()
                    .map_err(|e| {
                        self.failed_stage = Some(Stage::SourceSelections);
                        SelectedLiveError::failed(Stage::SourceSelections, e)
                    })?;
                if layout.raw_bytes.len() < 12 {
                    self.failed_stage = Some(Stage::SourceSelections);
                    return Err(SelectedLiveError::failed(
                        Stage::SourceSelections,
                        "default skateboard gravity payload is short",
                    ));
                }
                std::array::from_fn(|axis| {
                    f32::from_be_bytes(
                        layout.raw_bytes[axis * 4..axis * 4 + 4]
                            .try_into()
                            .expect("length checked"),
                    )
                })
            }
        };
        self.inner
            .world
            .as_ref()
            .expect("booted selected world")
            .world
            .lock()
            .map_err(|_| {
                self.failed_stage = Some(Stage::SourceSelections);
                SelectedLiveError::failed(Stage::SourceSelections, "world lock poisoned")
            })?
            .backend_mut()
            .set_gravity(gravity);
        let spawn = match spawn_policy {
            SelectedSpawnPolicy::AuthoredSelectedLocation => {
                self.inner
                    .assets
                    .as_ref()
                    .expect("completed original assets")
                    .spawn
            }
            SelectedSpawnPolicy::ExplicitHostTransform(transform) => transform,
        };
        self.inner
            .try_select_terrain(
                terrain_catalog,
                terrain_spec,
                effects,
                unlock_seed,
                collection,
                dmo_update_in_parallel,
                arena_userdata,
            )
            .map_err(|e| {
                self.failed_stage = Some(Stage::TerrainStart);
                SelectedLiveError::failed(Stage::TerrainStart, e)
            })?;
        // GameModule::SecondaryLoad01 0x8272d550: LoadMainDMOBank.
        let living_world_dmo = {
            let assets = self
                .inner
                .assets
                .as_ref()
                .expect("completed original assets");
            crate::selected_living_world_dmo::SelectedLivingWorldDmo::try_new(
                Arc::clone(&assets.mounts),
                &assets.cvars,
            )
        }
        .map_err(|e| {
            self.failed_stage = Some(Stage::LivingWorldDmo);
            SelectedLiveError::failed(Stage::LivingWorldDmo, e)
        })?;
        self.inner.living_world_dmo = Some(living_world_dmo);
        self.pending_boot = Some(PendingBoot {
            spawn,
            marquee,
            actor_flags,
            capsule_resource_allocated,
            thread,
            camera_set,
            presentation: SelectedPresentationPolicy {
                lod: presentation_lod,
                submission_capacity: presentation_submission_capacity,
                gather_capacity: presentation_gather_capacity,
                provider: presentation_provider,
                root_w: presentation_root_w.into(),
            },
            camera_submit_mask,
            terrain_stream_complete: false,
            terrain_requested: false,
            command_start,
        });
        Ok(SelectedBootProgress::TerrainStreamPending)
    }

    /// One stream/resident poll per call. On completion, admit the existing
    /// camera/roster, bind this input owner, and prepare the existing frame.
    /// Earlier successful writes stay retained if a later gate fails.
    pub fn try_poll_controlled_boot<TE, ME>(
        &mut self,
        mapping: &mut impl FnMut(usize, usize, u32) -> Result<SelectedGestureAllocation, ME>,
        assertion: &mut impl FnMut(),
        hat_enabled: &mut impl FnMut() -> bool,
    ) -> Result<SelectedBootProgress, SelectedLiveError>
    where
        T: FnMut() -> Result<u64, TE>,
        TE: Debug,
        ME: Debug,
    {
        use SelectedLiveStage as Stage;
        if let Some(stage) = self.failed_stage {
            return Err(SelectedLiveError::AlreadyFailed(stage));
        }
        let pending = self.pending_boot.as_mut().ok_or(if self.ready {
            SelectedLiveError::AlreadyAttempted
        } else {
            SelectedLiveError::Pending(SelectedLivePending::BootNotStarted)
        })?;
        // SecondaryLoad01's bank update runs beside the world stream.
        let dmo_bank_loaded = self
            .inner
            .living_world_dmo
            .as_mut()
            .expect("started DMO bank")
            .try_poll()
            .map_err(|e| {
                self.failed_stage = Some(Stage::LivingWorldDmo);
                SelectedLiveError::failed(Stage::LivingWorldDmo, e)
            })?;
        let terrain = self
            .inner
            .terrain
            .as_mut()
            .expect("started selected terrain");
        if !pending.terrain_stream_complete {
            match terrain.try_poll_stream().map_err(|e| {
                self.failed_stage = Some(Stage::TerrainStream);
                SelectedLiveError::failed(Stage::TerrainStream, e)
            })? {
                WorldStreamPoll::Pending => return Ok(SelectedBootProgress::TerrainStreamPending),
                WorldStreamPoll::CompletedThisPoll => {
                    pending.terrain_stream_complete = true;
                }
            }
        }
        if !pending.terrain_requested {
            match terrain.try_request() {
                Ok(()) => {
                    pending.terrain_requested = true;
                    return Ok(SelectedBootProgress::TerrainResidentPending);
                }
                Err(SelectedSessionTerrainError::Bounds(WorldBoundsError::Pending)) => {
                    return Ok(SelectedBootProgress::TerrainStreamPending);
                }
                Err(e) => {
                    self.failed_stage = Some(Stage::TerrainResident);
                    return Err(SelectedLiveError::failed(Stage::TerrainResident, e));
                }
            }
        }
        let recipe_owner=&self.inner.world.as_ref().expect("retained world").living_world_recipes;
        let recipe_runtime=recipe_owner.lock().map_err(|e|SelectedLiveError::failed(Stage::TerrainResident,e))?;
        let graph_guard=self.inner.world.as_ref().expect("retained world").graphs.lock()
            .map_err(|e|SelectedLiveError::failed(Stage::TerrainResident,e))?;
        let mut prop_frame=crate::selected_living_world_recipes::props::PropRecipeFrame{
            graphs:&graph_guard,recipes:&recipe_runtime.recipes,avoid_volumes:&recipe_runtime.challenge_avoid_volumes,vault:&self.inner.assets.as_ref().expect("retained assets").vault};
        if !terrain.try_poll_with_props(Some(&mut prop_frame)).map_err(|e| {
            self.failed_stage = Some(Stage::TerrainResident);
            SelectedLiveError::failed(Stage::TerrainResident, e)
        })? {
            return Ok(SelectedBootProgress::TerrainResidentPending);
        }
        drop(prop_frame);
        drop(graph_guard);
        drop(recipe_runtime);
        if !dmo_bank_loaded {
            return Ok(SelectedBootProgress::DmoBankPending);
        }
        // The bank's RegArena model pass, once the World query owner (and its
        // empty DMO pool) exists.
        {
            let assets = self
                .inner
                .assets
                .as_ref()
                .expect("retained original assets");
            let mut world = self
                .inner
                .world
                .as_ref()
                .expect("retained world")
                .world
                .lock()
                .map_err(|_| {
                    self.failed_stage = Some(Stage::LivingWorldDmo);
                    SelectedLiveError::failed(Stage::LivingWorldDmo, "world lock poisoned")
                })?;
            self.inner
                .living_world_dmo
                .as_mut()
                .expect("started DMO bank")
                .try_register(&mut world, &assets.vault, &assets.cvars)
                .map_err(|e| {
                    self.failed_stage = Some(Stage::LivingWorldDmo);
                    SelectedLiveError::failed(Stage::LivingWorldDmo, e)
                })?;
        }
        let mut spawn = self
            .pending_boot
            .as_ref()
            .expect("retained host policies")
            .spawn;
        // The stream focus the loader moved to the spawn point before
        // SpawnPlayer's own adjustment.
        let stream_focus = [spawn[12], spawn[13], spawn[14]];
        let world_handle = &self.inner.world.as_ref().expect("retained world").world;
        let mut world = world_handle.lock().map_err(|_| {
            self.failed_stage = Some(Stage::SpawnAdjustment);
            SelectedLiveError::failed(Stage::SpawnAdjustment, "world lock poisoned")
        })?;
        let ground = terrain.ground_pool_for(world_handle).map_err(|e| {
            self.failed_stage = Some(Stage::SpawnAdjustment);
            SelectedLiveError::failed(Stage::SpawnAdjustment, e)
        })?;
        let adjusted = crate::skater_manager_adjust_spawn_position::try_adjust_spawn_position(
            &mut world,
            ground,
            &self
                .inner
                .assets
                .as_ref()
                .expect("retained original assets")
                .cvars,
            [spawn[12], spawn[13], spawn[14], spawn[15]],
        )
        .map_err(|e| {
            self.failed_stage = Some(Stage::SpawnAdjustment);
            SelectedLiveError::failed(Stage::SpawnAdjustment, e)
        })?;
        if let Some(translation) = adjusted {
            spawn[12..16].copy_from_slice(&translation);
        }
        drop(world);
        let pending = self.pending_boot.take().expect("retained host policies");
        let player_request = self
            .inner
            .try_prepare_controlled_player(
                ControlledPlayerPolicy::HostSpawnWhileRegistrationPending,
                spawn,
                pending.marquee,
                pending.actor_flags,
                pending.thread,
            )
            .map_err(|e| {
                self.failed_stage = Some(Stage::ControlledPlayer);
                SelectedLiveError::failed(Stage::ControlledPlayer, e)
            })?;
        let (playback, attributes) = {
            let selected = &self
                .inner
                .player
                .as_ref()
                .expect("prepared player")
                .selected;
            let animation = &selected.owners.animation;
            let playback = animation.body.first().ok_or_else(|| {
                self.failed_stage = Some(Stage::PushAttributes);
                SelectedLiveError::failed(Stage::PushAttributes, "Skater playback absent")
            })?;
            let attributes = animation.attributes.as_ref().ok_or_else(|| {
                self.failed_stage = Some(Stage::PushAttributes);
                SelectedLiveError::failed(Stage::PushAttributes, "animation attributes absent")
            })?;
            (Arc::clone(playback), Arc::clone(attributes))
        };
        // `SkaterAnimShared::Init` constructs regular then mongo push blocks
        // before `InitWipeoutRecoverGraph`, all from this same playback/pool.
        let push = {
            let playback = playback.lock().map_err(|_| {
                self.failed_stage = Some(Stage::PushAttributes);
                SelectedLiveError::failed(Stage::PushAttributes, "Skater playback lock poisoned")
            })?;
            let mut attributes = attributes.lock().map_err(|_| {
                self.failed_stage = Some(Stage::PushAttributes);
                SelectedLiveError::failed(
                    Stage::PushAttributes,
                    "animation attributes lock poisoned",
                )
            })?;
            crate::skater_push_shared_attributes::SkaterPushSharedAttributes::try_build(
                &playback,
                &mut attributes,
            )
            .map_err(|error| {
                self.failed_stage = Some(Stage::PushAttributes);
                SelectedLiveError::failed(Stage::PushAttributes, error)
            })?
        };
        let push = Arc::new(push);
        self.inner.push_shared_attributes = Some(Arc::clone(&push));
        let graph = {
            let playback = playback.lock().map_err(|_| {
                self.failed_stage = Some(Stage::RecoveryGraph);
                SelectedLiveError::failed(Stage::RecoveryGraph, "Skater playback lock poisoned")
            })?;
            let mut attributes = attributes.lock().map_err(|_| {
                self.failed_stage = Some(Stage::RecoveryGraph);
                SelectedLiveError::failed(
                    Stage::RecoveryGraph,
                    "animation attributes lock poisoned",
                )
            })?;
            crate::skater_wipeout_recovery_graph::SkaterWipeoutRecoveryGraph::try_build(
                &playback,
                &mut attributes,
            )
            .map_err(|e| {
                self.failed_stage = Some(Stage::RecoveryGraph);
                SelectedLiveError::failed(Stage::RecoveryGraph, e)
            })?
        };
        let graph = Arc::new(graph);
        self.inner.wipeout_recovery_graph = Some(Arc::clone(&graph));
        self.inner
            .try_finish_controlled_player(player_request)
            .map_err(|e| {
                self.failed_stage = Some(Stage::ControlledPlayer);
                SelectedLiveError::failed(Stage::ControlledPlayer, e)
            })?;
        let actor = Arc::clone(
            self.inner
                .player
                .as_ref()
                .expect("retained player")
                .selected
                .actor
                .as_ref()
                .expect("constructed Actor"),
        );
        actor
            .try_install_push_shared_attributes(push)
            .map_err(|e| {
                self.failed_stage = Some(Stage::PushAttributes);
                SelectedLiveError::failed(Stage::PushAttributes, e)
            })?;
        let handplant_lengths = {
            let playback = playback.lock().map_err(|_| {
                SelectedLiveError::failed(Stage::RecoveryGraph, "HandPlant playback lock")
            })?;
            crate::actor_construction::motion_runtime_session::handplant::measure(&playback)
                .map_err(|e| SelectedLiveError::failed(Stage::RecoveryGraph, e))?
        };
        *actor.handplant_shared.lock().map_err(|_| {
            SelectedLiveError::failed(Stage::RecoveryGraph, "HandPlant shared lock")
        })? = Some(Arc::new(handplant_lengths));
        actor
            .try_install_wipeout_recovery_graph(graph)
            .map_err(|e| {
                self.failed_stage = Some(Stage::RecoveryGraph);
                SelectedLiveError::failed(Stage::RecoveryGraph, e)
            })?;
        let mut translated = |size, align, flags| mapping(size, align, flags).map(Into::into);
        self.inner
            .try_continue_physical(
                &mut translated,
                &mut HostReckoningNumeric,
                pending.capsule_resource_allocated,
                assertion,
                hat_enabled,
            )
            .map_err(|e| {
                self.failed_stage = Some(Stage::PhysicalPair);
                SelectedLiveError::failed(Stage::PhysicalPair, e)
            })?;
        self.inner
            .try_continue_controlled_presentation::<TE>()
            .map_err(|e| {
                self.failed_stage = Some(Stage::ControlledPresentation);
                SelectedLiveError::failed(Stage::ControlledPresentation, e)
            })?;
        self.inner
            .try_admit_selected_camera_and_roster(&pending.camera_set)
            .map_err(|e| {
                self.failed_stage = Some(Stage::CameraRoster);
                SelectedLiveError::failed(Stage::CameraRoster, e)
            })?;
        let SelectedCommandStartPolicy::NativeStaticFields = pending.command_start;
        let queue = SelectedSimCommandQueue::construct_offline_command_fields(
            SimCommandCounters::from_static_initializer(),
        );
        let random =
            SimulationCommandRandom::construct_field(NativeRandomGate::production_shared());
        self.inner
            .try_bind_selected_input(queue, random)
            .map_err(|e| {
                self.failed_stage = Some(Stage::InputBind);
                SelectedLiveError::failed(Stage::InputBind, std::mem::discriminant(&e))
            })?;
        self.inner
            .try_prepare_selected_queued_frame(pending.presentation, pending.camera_submit_mask)
            .map_err(|e| {
                self.failed_stage = Some(Stage::FramePrepare);
                SelectedLiveError::failed(Stage::FramePrepare, e)
            })?;
        self.inner.spawn_policy = Some(super::party_skaters::SkaterSpawnPolicy {
            actor_flags: pending.actor_flags,
            capsule_resource_allocated: pending.capsule_resource_allocated,
            presentation: pending.presentation,
        });
        // GameModule::ResetGame8273a680 creates the game's session marker.
        let game_mode = {
            let world = self
                .inner
                .world
                .as_ref()
                .expect("retained world")
                .world
                .lock()
                .map_err(|_| {
                    self.failed_stage = Some(Stage::SessionMarker);
                    SelectedLiveError::failed(Stage::SessionMarker, "world lock poisoned")
                })?;
            world
                .physical_world_services()
                .ok_or_else(|| {
                    self.failed_stage = Some(Stage::SessionMarker);
                    SelectedLiveError::failed(Stage::SessionMarker, "world services absent")
                })?
                .game_info()
                .try_game_mode()
                .map_err(|e| {
                    self.failed_stage = Some(Stage::SessionMarker);
                    SelectedLiveError::failed(Stage::SessionMarker, e)
                })?
        };
        self.inner.session_marker = Some(SelectedSessionMarker::try_construct(game_mode, None).map_err(
            |e| {
                self.failed_stage = Some(Stage::SessionMarker);
                SelectedLiveError::failed(Stage::SessionMarker, e)
            },
        )?);
        // StateLoading::CharacterPrepareState exit 0x827601e0: the initial
        // LivingWorld census at the stream focus.
        {
            let cvars = &self
                .inner
                .assets
                .as_ref()
                .expect("retained original assets")
                .cvars;
            let world_owner = self.inner.world.as_ref().expect("retained world");
            let player = self.inner.player.as_mut().expect("retained player");
            let actor = player.selected.actor.as_ref().expect("constructed Actor");
            let owners = &mut player.selected.owners;
            let recipe_owner=Arc::clone(&owners.living_world_recipes);
            let mut recipe_runtime=recipe_owner.lock().map_err(|e|SelectedLiveError::failed(Stage::LivingWorldDmo,e))?;
            let graph_owner=Arc::clone(owners.graphs.as_ref().expect("graph owners"));
            let graph_guard=graph_owner.lock().map_err(|e|SelectedLiveError::failed(Stage::LivingWorldDmo,e))?;
            let mut recipes = crate::selected_living_world_recipes::SelectedRecipeFrame {
            challenge: None,
                triggers: &self.inner.terrain.as_ref().expect("retained terrain")
                    .terrain_for(&world_owner.world)
                    .map_err(|e| SelectedLiveError::failed(Stage::LivingWorldDmo,e))?.triggers,
                pedestrian_animation: crate::selected_living_world_recipes::pedestrian_animation::PedestrianSessionAnimation {
                    startup: &owners.pedestrian_animation, shared: &owners.animation, world: &world_owner.world,
                },
                runtime: &mut recipe_runtime,
                registry: owners
                    .recipe_registry
                    .as_mut()
                    .expect("mounted recipe registry"),
                requests: owners
                    .recipe_load
                    .as_mut()
                    .expect("mounted recipe requests")
                    .requests_mut(),
                mounts: &owners.mounts,
                vault: &owners.services.vault,
                graphs: &graph_guard,
                painter: &self
                    .inner
                    .terrain
                    .as_ref()
                    .expect("retained terrain")
                    .terrain_for(&world_owner.world)
                    .map_err(|error| {
                        self.failed_stage = Some(Stage::LivingWorldDmo);
                        SelectedLiveError::failed(Stage::LivingWorldDmo, error)
                    })?
                    .layers,
            };
            let mut world = world_owner.world.lock().map_err(|_| {
                self.failed_stage = Some(Stage::LivingWorldDmo);
                SelectedLiveError::failed(Stage::LivingWorldDmo, "world lock poisoned")
            })?;
            let local = world.local_skater_identity.clone().ok_or_else(|| {
                self.failed_stage = Some(Stage::LivingWorldDmo);
                SelectedLiveError::failed(
                    Stage::LivingWorldDmo,
                    "SkaterManager local identity absent",
                )
            })?;
            self.inner
                .living_world_dmo
                .as_mut()
                .expect("started DMO bank")
                .try_initial_census(&mut world, actor, &local, cvars, stream_focus, &mut recipes,
                    self.inner.terrain.as_ref().expect("retained terrain")
                        .ground_pool_for(&world_owner.world)
                        .map_err(|e|SelectedLiveError::failed(Stage::LivingWorldDmo,e))?,
                    Some(self.inner.bound_input.as_mut().expect("bound Simulation input").simulation_random_mut()))
                .map_err(|e| {
                    self.failed_stage = Some(Stage::LivingWorldDmo);
                    SelectedLiveError::failed(Stage::LivingWorldDmo, e)
                })?;
            // Native loading exit827582e8 publishes GameBackEnd.InGame only
            // after preparation and initial census have completed.
            local.backend().finish_loading()
                .map_err(|e| SelectedLiveError::failed(Stage::FramePrepare, e))?;
        }
        self.ready = true;
        self.pending_boot = None;
        Ok(SelectedBootProgress::ControlledFrameReady)
    }

    /// Tests every retained owner gate, rather than interpreting the broad
    /// `TerrainCameraInputAndFrameServices` boot hint as readiness.
    #[must_use]
    pub fn controlled_frame_ready(&self) -> bool {
        if !self.ready
            || self.failed_stage.is_some()
            || !self.inner.camera_complete
            || !self.inner.presentation_complete
        {
            return false;
        }
        let Some(world) = self.inner.world.as_ref() else {
            return false;
        };
        self.inner.player.is_some()
            && self.inner.physical.is_some()
            && self.inner.bound_input.is_some()
            && self.inner.session_marker.is_some()
            && self.inner.selected_tick.is_some()
            && self.inner.presentation_runtime.is_some()
            && self
                .inner
                .terrain
                .as_ref()
                .is_some_and(|terrain| terrain.ground_pool_for(&world.world).is_ok())
    }

    /// Poll/cache the supplied pad, enqueue against the same loader/game-info,
    /// then execute the existing `SelectedSessionFrame` exactly once. The host
    /// supplies every timing, focus, physics and parked-grind answer.
    pub fn try_tick<P, CE, TE>(
        &mut self,
        pad: &mut P,
        policy: SelectedQueuedFramePolicy,
        clamp: &mut impl FnMut() -> Result<i32, CE>,
    ) -> Result<SelectedQueuedFrameOutcome, SelectedLiveError>
    where
        P: PadPlatform,
        P::Error: Debug,
        CE: Debug,
        T: FnMut() -> Result<u64, TE>,
        TE: Debug,
    {
        self.last_step_completed = false;
        if let Some(stage) = self.failed_stage {
            return Err(SelectedLiveError::AlreadyFailed(stage));
        }
        let mut result = self.try_tick_once(pad, policy, clamp);
        // GameModule::DoGameUpdate (0x82730418) runs PlayerUI::Update on paused frames too:
        // CheckForPauseState (0x826477b0) fails, so the focus is cleared without simulating, and
        // the first unpaused update regains it (PlayerInputGainFocusData).
        if matches!(result, Ok(SelectedQueuedFrameOutcome::Frontend))
            && let Some(input) = self.inner.bound_input.as_mut()
            && let Err(e) = input.try_player_ui_prefix::<P>(false, false)
        {
            let stage = SelectedLiveStage::PreupdateInput;
            result = Err(SelectedLiveError::failed(stage, e));
        }
        if let Err(SelectedLiveError::Failed { stage, .. }) = &result {
            // Any reached device/preupdate/tick write is monotonic. A failed
            // owner is retained for inspection and cannot replay that frame.
            self.failed_stage = Some(*stage);
        }
        result
    }

    fn try_tick_once<P, CE, TE>(
        &mut self,
        pad: &mut P,
        policy: SelectedQueuedFramePolicy,
        clamp: &mut impl FnMut() -> Result<i32, CE>,
    ) -> Result<SelectedQueuedFrameOutcome, SelectedLiveError>
    where
        P: PadPlatform,
        P::Error: Debug,
        CE: Debug,
        T: FnMut() -> Result<u64, TE>,
        TE: Debug,
    {
        use SelectedLiveStage as Stage;
        if !self.controlled_frame_ready() {
            return Err(SelectedLiveError::Pending(
                SelectedLivePending::FrameNotReady,
            ));
        }
        if !policy.physics_flags3 {
            return Err(SelectedLiveError::Pending(
                SelectedLivePending::UnsourcedFramePolicy("physical schedule"),
            ));
        }
        if !policy.park_grind_preparation || !policy.exclude_grind_fill {
            return Err(SelectedLiveError::Pending(
                SelectedLivePending::UnsourcedFramePolicy("active grind schedule"),
            ));
        }
        let SelectedFrameNumerics::ExistingHostApproximation = policy.numerics;
        // FrontEndManager::Update (0x82640130): an in-game frame the front end does not pause adds
        // its 16 ms to the time played (statistic 19).
        if self.main_menu.is_none() && !self.main_menu_pending
            && !self.frontend_audio_state().is_some_and(|state| state.paused)
            && let Some(player) = self.inner.player.as_ref()
        {
            let owners = &player.selected.owners;
            owners.services.skaters.backend()
                .update_additive_statistic(19, f32::from_bits(0x3c83_126f), false, &owners.cvars)
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        }
        // Its ChaseArrowMan::Update (0x82631328) runs on every in-game frame, paused or not.
        if self.main_menu.is_none() && !self.main_menu_pending {
            self.try_update_chase_arrows().map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        }
        if self.career.as_mut().is_some_and(career::CareerSession::finish_map_challenge_load) {
            self.frontend_reveal_pending = true;
        }
        if let Some(world) = self.inner.world.as_ref() {
            let settings = &self.audio.settings;
            world
                .local
                .try_set_camera_profile_settings(settings.camera_angle, settings.invert_x, settings.invert_y, settings.grind_transparency)
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, format!("{e:?}")))?;
        }
        let fade_speed = self.inner.assets.as_ref().expect("ready assets").cvars
            .try_frontend_fade_speed()
            .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        self.apply_career_fe_background();
        self.frontend_fade.update(fade_speed);
        self.update_online()?;
        // TimerCallback polls once for both input groups. Frontend keeps its
        // PRES history alive while paused; SIM is drained but never enqueued.
        let input = self.inner.bound_input.as_mut().expect("ready input");
        input.poll_and_cache(pad)
            .map_err(|e| SelectedLiveError::failed(Stage::DevicePoll, e))?;
        // PartyPlayUI keeps the input on the turn's skater (mPlayerIndex).
        let loader = self
            .inner
            .party
            .iter()
            .map(|skater| &skater.loader)
            .find(|loader| loader.player_index() == input.player_index())
            .unwrap_or(&self.inner.player.as_ref().expect("ready player").selected.owners.loader);
        let has_sim_input = input.try_last_input(pad, loader)
            .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        let frontend_input = input.try_pause_menu_input::<P>()
            .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        let phone_requested=input.try_phone_options_input::<P>()
            .map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
        let hud_dpad = input.try_hud_dpad_input::<P>()
            .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        if self.replay_file_pending.is_some() {
            if self.frontend_fade.covered() {self.complete_replay_file_load()?;}
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if self.trick_guide_pending {
            if self.frontend_fade.covered() {self.trick_guide_pending=false;self.open_trick_guide()?;}
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if self.trick_guide.is_some() {
            self.update_trick_guide(frontend_input,policy.has_focus)?;
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if self.career.as_mut().is_some_and(career::CareerSession::take_hom_replay_editor) {
            self.open_hom_results_replay_editor()?;
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if self.replay_editor_pending {
            if self.frontend_fade.covered() {
                self.replay_editor_pending=false;
                self.open_replay_editor()?;
            }
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if self.replay_editor.is_some() {
            let replay_input=input.try_replay_editor_input::<P>()
                .map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
            self.update_replay_editor(&replay_input,policy.dt,policy.has_focus)?;
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        // StateLoading owns input through challenge/world preparation, including a paused NIS.
        let map_challenge_loading = self.career.as_ref().is_some_and(career::CareerSession::map_challenge_loading);
        let frontend_input = if map_challenge_loading { Default::default() } else { frontend_input };
        let main_challenge = self.career.as_ref().is_some_and(|career| career.main_challenge_active());
        let party_play = self.options_menu.visible() && self.party_play_mode()?;
        let options = self.audio.update_options(&mut self.options_menu, frontend_input, policy.has_focus,
            main_challenge, party_play)?;
        if options != music_player_session::OptionsOutcome::Hidden {
            if options == music_player_session::OptionsOutcome::Exited {
                // Options pops back with mbKeepPrevious: Show(true) runs MainMenu::OnGainFocus8264a018,
                // whose SetScreenState(5) is menu_picker SetValue(-1) then Update, back on row 0.
                if let Some(menu) = self.main_menu.as_mut() {
                    menu.selected = 0;
                } else if self.pause_menu.snapshot().visible {
                    // In game Options was pushed over the StartHub, which the pop enters anew.
                    self.pause_menu.reenter();
                }
            }
            // Net::ResumeGame (0x82791ca8): an online game simulates under Options as well.
            if !self.online.network_resumed {
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
        }
        let frontend_input = if options == music_player_session::OptionsOutcome::Hidden { frontend_input } else { Default::default() };
        // FE state 45 over the StartHub takes the front end's input, an online game simulating
        // under it.
        let profile_open = self.skate_profile.visible();
        if profile_open {
            self.update_skate_profile(frontend_input, policy.has_focus);
            if !self.online.network_resumed {
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
        }
        let frontend_input = if profile_open { Default::default() } else { frontend_input };
        // FrontEndManager::Update: FEMoviePlayer::Update, then FE state 28's input and update.
        if self.update_movie(frontend_input, policy.has_focus)? {
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        let frontend_input = self.update_online_menu(frontend_input, policy.has_focus)?;
        // CheckForOnlineHubMenu (0x82656428): no memcard save, no quit under way, CheckForPauseState
        // (no pausing state on top, the pause stack's owner INIT or MANUALCAM), no load, no online
        // loading screen (`mbDisableBackpackAndPauseMenu`), and the career's own refusals.
        let hub_can_open = self.completed_frames != 0 && !map_challenge_loading && !self.pause_menu.snapshot().visible
            && !self.main_menu_pending && !self.career_saves.busy() && !self.online_loading_screen()
            && self.career.as_ref().is_none_or(career::CareerSession::online_hub_allowed)
            && self.inner.bound_input.as_ref().is_some_and(|input| input.try_can_open_frontend().unwrap_or(false));
        let frontend_input = self.update_online_hub(frontend_input, policy.has_focus, hub_can_open)?;
        // A game the single player hub joined: once the fade covers, the game in play unloads
        // and the online game starts.
        if self.main_menu.is_none() && self.online.start.is_some() {
            if self.frontend_fade.covered() {
                self.online_enter_from_play()?;
            }
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        let hub_open = self.online_hub_open();
        // StateManagerFrontEnd::ReceiveInput (0x82656af8) gives an open popup the input, and the
        // HUD components none.
        let online_popup = self.online_popup_open();
        let input = self.inner.bound_input.as_mut().expect("ready input");
        if self.main_menu_pending {
            if self.frontend_fade.covered() {
                if self.pause_menu.take_doing_quit() {
                    self.start_hub_fade_to_exit()?;
                }
                self.game_unload()?;
                let mut menu=crate::ui::main_menu::Controller::new(false);
                menu.new_profile=Self::profile_is_new()?;
                self.main_menu=Some(menu);
                self.main_menu_pending=false;
                self.pause_menu.close();
                self.frontend_reveal_pending=true;
            }
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if let Some(menu)=self.main_menu.as_mut() {
            if !menu.input_paused {
                input.try_set_frontend_pause(true)
                    .map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
                menu.input_paused=true;
            }
            if menu.leaving && self.frontend_fade.covered() && !self.exit_to_desktop {
                let new_career = menu.new_profile;
                self.main_menu=None;
                self.try_leave_frontend_world()?;
                let input = self.inner.bound_input.as_mut().expect("ready input");
                input.try_set_frontend_pause(false)
                    .map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
                if let Some(game) = self.freeskate.start.take() {
                    self.start_freeskate(game)?;
                } else if let Some(start) = self.online.start.take() {
                    self.start_online_game(start)?;
                } else if let Some(game) = self.take_party_play_start() {
                    self.start_party_play(game)?;
                } else if new_career {
                    // EnterGame's flow message 28: StateMainMenu::NewCareerIntro, then
                    // RedaIntro, then NewCAC.
                    self.begin_new_career_intro();
                    return Ok(SelectedQueuedFrameOutcome::Frontend);
                } else {
                    // The career's start travels under the fade and reveals once its skater has landed.
                    self.enter_single_player()?;
                    self.career_start_pending = true;
                }
            } else {
                let assets=self.main_menu_assets.as_ref().expect("main menu assets admitted");
                let selection = menu.update(assets,policy.dt,frontend_input,policy.has_focus);
                if policy.has_focus && menu.settled && menu.intro_complete && !menu.leaving
                    && menu.extras.is_none() && menu.name_prompt.is_none()
                    && menu.screen == crate::ui::main_menu::Screen::MainMenu
                    && frontend_input.select && menu.selected == 5 {
                    self.options_menu.open(false, crate::ui::options::VideoScreen::MainMenu, &self.audio.settings);
                }
                match selection {
                    crate::ui::main_menu::Selection::SaveProfileName(name) => self.save_first_profile_name(&name),
                    crate::ui::main_menu::Selection::Online => {
                        menu.nickname_needed = !crate::ui::online_config::OnlineConfig::load().has_nickname();
                        self.online_sign_in();
                    }
                    crate::ui::main_menu::Selection::SaveOnlineNickname(nickname) => {
                        let nickname = nickname.trim().to_owned();
                        match (crate::ui::online_config::OnlineConfig { nickname, ..crate::ui::online_config::OnlineConfig::load() }).save() {
                            Ok(()) => {
                                menu.nickname_prompt = None;
                                menu.nickname_needed = false;
                                self.online_nickname_changed();
                            }
                            Err(e) => if let Some(prompt) = menu.nickname_prompt.as_mut() {
                                prompt.error = Some(format!("Could not save online-config.json: {e}"));
                            },
                        }
                    }
                    crate::ui::main_menu::Selection::None => {}
                    crate::ui::main_menu::Selection::OpenExtras => self.open_extras()?,
                    // PC: the fade covers, then the host closes the window (`exit_to_desktop_ready`).
                    crate::ui::main_menu::Selection::ExitToDesktop => {
                        menu.leaving=true;
                        self.exit_to_desktop=true;
                        self.frontend_fade.cover();
                    }
                    crate::ui::main_menu::Selection::InstallUpdate => self.update_request = Some(crate::ui::update_prompt::UpdateRequest::Install),
                    crate::ui::main_menu::Selection::Extras(action) => self.update_extras_action(action)?,
                    crate::ui::main_menu::Selection::ContinueCareer => {
                        menu.leaving=true;
                        self.start_single_player_census()?;
                        self.frontend_fade.cover();
                    }
                    crate::ui::main_menu::Selection::FreeSkate => self.open_freeskate_option()?,
                    crate::ui::main_menu::Selection::PartyPlay => self.open_party_play()?,
                    // FrontEndState_PartyPlay's A: EnableBackground fades over, then EnterGame.
                    crate::ui::main_menu::Selection::EnterPartyPlay(game) => {
                        menu.leaving=true;
                        self.party_play.start=Some(game);
                        self.frontend_fade.cover();
                    }
                    // FrontEndState_FreeSkateOption's A: EnableBackground fades over, then EnterGame.
                    crate::ui::main_menu::Selection::EnterFreeSkate(game) => {
                        menu.leaving=true;
                        self.freeskate.start=Some(game);
                        self.frontend_fade.cover();
                    }
                }
                self.update_freeskate_viewer(policy.dt)?;
                self.update_party_play_viewer(policy.dt)?;
                self.try_tick_frontend_world(policy, clamp)?;
                self.try_update_unowned_dynamic_hulls()?;
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
        }
        // A front-end skater loads while no simulation tick reads its presentation.
        if self.apply_pending_character()? {
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if self.editor_enter_pending {
            if self.frontend_fade.covered() {
                self.open_edit_skater()?;
                self.editor_enter_pending = false;
                self.frontend_reveal_pending = true;
            }
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if self.editor.is_some() {
            self.update_edit_skater(frontend_input,policy.has_focus,policy.dt)?;
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        if let Some(transform) = self.map_travel.filter(|_| !self.map_travel_dispatched) {
            if !self.frontend_fade.covered() {
                trace_travel(&mut self.map_travel_gate, "the fade");
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
            let mut transform = transform;
            let terrain = self.inner.terrain.as_mut().expect("ready terrain");
            terrain.try_advance_asset_release()
                .map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
            // A destination the simulation stream holds nothing at cannot be landed on (no number,
            // or another world's location, such as the training facility's, which the port does not
            // load): the travel ends where the skater is, which the focus holds.
            let destination = [transform[12], transform[13], transform[14]];
            if !terrain.landable(destination).map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))? {
                let here = terrain.focus_position()
                    .ok_or_else(|| SelectedLiveError::failed(Stage::TerrainResident, "travel destination outside the stream and no focus held"))?;
                eprintln!("travel: the simulation stream holds nothing at {destination:?}; the travel ends at {here:?}");
                if transform[..12].iter().any(|value| !value.is_finite()) {
                    transform[..12].copy_from_slice(&[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
                }
                transform[12..15].copy_from_slice(&here);
                transform[15] = 1.0;
                self.map_travel = Some(transform);
            }
            let position = [transform[12], transform[13], transform[14]];
            let ready = terrain.try_prepare_teleport(position)
                .map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
            if !ready {
                trace_travel(&mut self.map_travel_gate, "the location to stream");
            }
            if ready && self.dynamic_hulls_unloading {
                // GameModule::GameLoad (0x8273be50): once the focus is in, the dynamic hull manager
                // updates until the last game's hulls are all unloaded (IsAllUnloaded 0x82da1680).
                terrain.try_update_dynamic_hulls()
                    .map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
                if !terrain.dynamic_hulls_unloaded() {
                    trace_travel(&mut self.map_travel_gate, "the last game's dynamic hulls to unload");
                    return Ok(SelectedQueuedFrameOutcome::Frontend);
                }
                self.dynamic_hulls_unloading = false;
            }
            if ready && !career::try_dynamic_hull_state(self.career.as_mut(), self.freeskate.hulls.as_mut(), terrain.as_mut(), position)? {
                // The presentation answers again once it shows the hulls the loader registered.
                trace_travel(&mut self.map_travel_gate, "the dynamic hulls");
                self.map_travel_presentation_ready = false;
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
            if ready && !self.map_travel_presentation_ready {
                trace_travel(&mut self.map_travel_gate, "the presentation");
            }
            if ready && self.map_travel_presentation_ready {
                trace_travel(&mut self.map_travel_gate, "the teleport");
                self.apply_spawn_character_attributes()?;
                let input = self.inner.bound_input.as_mut().expect("ready input");
                let player = &self.inner.player.as_ref().expect("ready player").selected;
                let regular = player.owners.services.settings.try_stance_byte(input.player_index())
                    .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                let (generation, _) = player.actor.as_ref().expect("ready Actor").try_teleport_publication()
                    .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                self.map_travel_generation = Some(generation.wrapping_add(1));
                self.map_travel_ready_seen = false;
                let marker = self.inner.session_marker.as_mut().expect("ready session marker");
                if let Some(marker) = marker.marker.as_mut() { marker.clear_for_world_travel(); }
                marker.teleport.send_flow(crate::session_marker::player_ui::TeleportToPosition {
                    transform: crate::skeleton_root_transforms::SkeletonRootMatrix::known(transform),
                    foot_forward: u32::from(regular != 0),
                    on_board: self.map_travel_on_board,
                    session_marker: false,
                });
                // A game start's PlayerUI::ResetPlayer (0x828ad280), after its spawn command: the
                // new game's marker is not valid, so SetSessionMarker (0x828acf30, forced, on the
                // board) puts it at the spawn and cMsgSetMarker goes out for the UI's player. Party
                // Play's PartyPlayUI (0x828aeae0) is made without a marker.
                let mut spawn_marker = false;
                if self.map_travel_on_board
                    && let Some(session) = marker.marker.as_mut()
                    && session.get().is_none()
                {
                    let spawn = crate::skeleton_root_transforms::SkeletonRootMatrix::known(transform);
                    session.set_forced(&spawn, u32::from(regular != 0), true);
                    spawn_marker = true;
                }
                if spawn_marker {
                    if std::env::var_os("SKATE2_TRACE_SESSION_MARKER").is_some() {
                        eprintln!("session marker: set at the spawn {:?}", [transform[12], transform[13], transform[14]]);
                    }
                    let local = self.inner.world.as_ref().expect("ready world").world.lock()
                        .map_err(|_| SelectedLiveError::failed(Stage::SessionMarker, "world lock poisoned"))?
                        .local_skater_identity
                        .clone()
                        .ok_or_else(|| SelectedLiveError::failed(Stage::SessionMarker, "SkaterManager local identity absent"))?;
                    let player = i32::from_ne_bytes(input.player_index().to_ne_bytes());
                    local.try_challenge_broadcaster()
                        .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?
                        .send_marker(true, player)
                        .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                }
                self.map_menu.close();
                if let Some(career) = self.career.as_mut() {
                    career.close_challenge_map();
                }
                self.pause_menu.close();
                input.try_set_frontend_pause(false)
                    .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
                self.map_travel_dispatched = true;
            }
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        let round_syncing = self.online_round_syncing();
        let online_loading_screen = self.online_loading_screen();
        let nis_on_round_ticks = self.online_round_ticking();
        let input = self.inner.bound_input.as_mut().expect("ready input");
        if self.map_menu.snapshot().visible {
            // The career's map is FrontEndState_ChallengeMap: OnUpdate each front-end update,
            // ReceiveInput while it has focus.
            if let Some(career) = self.career.as_mut() {
                use crate::challenge::stuff_menu::{MapAction, MapInput};
                let map_input = policy.has_focus.then_some(MapInput {
                    up: frontend_input.up,
                    down: frontend_input.down,
                    select: frontend_input.select,
                    back: frontend_input.cancel,
                    switch_list: frontend_input.misc,
                });
                let action = career.update_challenge_map(map_input);
                if matches!(action, MapAction::TeleportChallenge(_) | MapAction::RetryChallenge(_) | MapAction::TeleportLocation(_)) {
                    // DismissAndTeleportCallback826750f0 sends the service's
                    // unspawn before fading/moving stream focus. The map is
                    // paused, so do not defer its physical cull to a SIM tick
                    // at the destination with the old terrain already gone.
                    self.apply_career_living_world()?;
                }
                let career = self.career.as_mut().expect("career map owner");
                let destination = match action {
                    MapAction::None => return Ok(SelectedQueuedFrameOutcome::Frontend),
                    MapAction::Close => {
                        career.close_challenge_map();
                        self.map_menu.close();
                        self.pause_menu.reenter();
                        return Ok(SelectedQueuedFrameOutcome::Frontend);
                    }
                    MapAction::TeleportChallenge(key) => career.prepare_challenge_map_travel(key, false),
                    MapAction::RetryChallenge(key) => career.prepare_challenge_map_travel(key, true),
                    MapAction::TeleportLocation(key) => self
                        .inner
                        .assets
                        .as_ref()
                        .expect("ready assets")
                        .map_catalog
                        .transform(key),
                };
                if let Some(transform) = destination {
                    self.frontend_fade.cover();
                    self.map_travel = Some(transform);
                    self.map_travel_presentation_ready = false;
                    self.map_travel_dispatched = false;
                    self.map_travel_on_board = false;
                } else {
                    // DoTeleport (0x8265f280) pops to the start hub's state 7 without a destination.
                    career.close_challenge_map();
                    self.map_menu.close();
                    self.pause_menu.close();
                    self.inner
                        .bound_input
                        .as_mut()
                        .expect("ready input")
                        .try_set_frontend_pause(false)
                        .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
                }
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
            if policy.has_focus {
                let assets = self.inner.assets.as_ref().expect("ready assets");
                match self.map_menu.update(frontend_input, &assets.map_catalog.locations) {
                    crate::ui::map_menu::MapMenuAction::None => {},
                    crate::ui::map_menu::MapMenuAction::Close => {
                        self.map_menu.close();
                        self.pause_menu.reenter();
                    }
                    crate::ui::map_menu::MapMenuAction::Teleport(key) => {
                        self.frontend_fade.cover();
                        self.map_travel = Some(assets.map_catalog.transform(key).ok_or_else(||
                            SelectedLiveError::failed(Stage::SessionMarker, "Map destination transform absent"))?);
                        self.map_travel_presentation_ready = false;
                        self.map_travel_dispatched = false;
                        self.map_travel_on_board = false;
                    },
                }
            }
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        // An online round's sync point: kPAUSE_STATE_ONLINE and Net::PauseGame hold the
        // simulation until every player has reached it.
        if round_syncing {
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        // SimController::Update1 (0x828a72c8) runs the NIS manager every
        // simulation update, paused or not; FrontEndState_NIS (FE state 2)
        // takes A (keycode 2) as SkipNIS.
        if let Some(career) = self.career.as_mut() {
            // A front-end state pushed over FrontEndState_NIS takes the input first.
            // An online game plays its scenes through: every console runs them over the same ticks.
            if policy.has_focus && frontend_input.select && career.nis.fe_state_active() && !career.photo_select_open() && !career.overlay_open()
                && self.online.game.is_none()
            {
                career.nis.fe_select();
            }
            // An online round's scenes update with its challenge ticks instead
            // (`update_online_round_nis`).
            if !nis_on_round_ticks {
                let world = self.inner.world.as_ref().map(|owner| &owner.world);
                let terrain = self.inner.terrain.as_deref();
                career
                    .nis
                    .update_simulation(&mut |start, end| nis_ground_line(world, terrain, start, end))
                    .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
            }
            for paused in career.take_nis_events() {
                input.try_set_nis_pause(paused)
                    .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
            }
            // Net::ResumeGame's mask: neither pause stops an online game's simulation.
            if (career.nis.is_sim_paused() || career.challenge_paused) && !self.online.network_resumed {
                // kPAUSE_STATE_NIS or kPAUSE_STATE_CHALLENGE: no simulation update; the scene
                // manager's presentation frame still runs.
                // A replay-controlled scene's local skater is the restored one
                // (`update_replay_view`): SkaterPresEntity::GetPosition asks the entity the
                // recorded frames restored.
                if career.nis.replay_clock().is_none() {
                    let local = self.inner.local_index_presentation().map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
                    career.nis.set_camera_anchor(local.and_then(|runtime| runtime.published_root()));
                    career.nis.set_camera_local_hips(local.and_then(|runtime| runtime.published_hips_position()));
                    career.nis.set_camera_local_hips_forward(local.and_then(|runtime| runtime.published_hips_forward()));
                    career.nis.set_local_bounds(local.and_then(|runtime| runtime.published_bounds()));
                }
                career.set_local_player(
                    self.inner
                        .player
                        .as_ref()
                        .and_then(|player| player.selected.owners.recipe_load.as_ref())
                        .and_then(|load| load.recipe().ok())
                        .map(|recipe| career::CareerSession::local_player_recipe(recipe)),
                );
                // The streamer controller while paused: the NIS's frozen
                // focus, else Sim_LocalPlayerStreamerController's (0x8277d4d0)
                // at the local Actor, each a MoveStreamFocus of every focus.
                // The terrain answers IsStreamFocusLoaded(3, GameLoad) for the
                // simulation focus; the presentation focus streams on the
                // renderer's side.
                let focus = match career.nis.stream_focus() {
                    Some(FrozenFocus::At(position)) => position,
                    Some(FrozenFocus::GameCamera) => {
                        self.inner
                            .world
                            .as_ref()
                            .ok_or_else(|| SelectedLiveError::failed(Stage::TerrainResident, "world absent"))?
                            .local
                            .try_camera_producer_projection()
                            .map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?
                            .camera_matrix
                            .columns()[3]
                            .xyz
                    }
                    None => local_skater_position(
                        self.inner.world.as_ref().map(|world| &world.world),
                        self.inner.player.as_ref().expect("ready player").selected.actor.as_deref().expect("ready Actor"),
                    )?,
                };
                let terrain = self.inner.terrain.as_mut().ok_or_else(|| SelectedLiveError::failed(Stage::TerrainResident, "terrain absent"))?;
                terrain.try_advance_asset_release().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
                terrain.try_update_focus(focus).map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
                let loaded = terrain.try_focus_loaded().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
                career.nis.set_stream_focus_loaded(loaded);
                let scene_dt = career.nis.update_presentation_begin(1.0 / 60.0);
                if replay_session::update_replay_view(career, &self.replay_frames, &mut self.replay_clock, &mut self.replay_restorer)? {
                    replay_session::restore_game_cloth(&mut self.inner, Some(career))?;
                }
                {
                    let world_owner = self.inner.world.as_ref();
                    let terrain = self.inner.terrain.as_ref();
                    // Presentation::AddLineTest with kICECameraRadius over the selected world's
                    // admitted ground (User_ICEWorldManager::HandleCollision).
                    career.nis.update_presentation_end(scene_dt, &mut |start, end, radius| {
                        let owner = world_owner.ok_or("NIS camera line: world absent")?;
                        let ground = terrain
                            .ok_or("NIS camera line: terrain absent")?
                            .ground_pool_for(&owner.world)
                            .map_err(|e| format!("NIS camera line: {e}"))?;
                        let mut world = owner.world.lock().map_err(|_| "NIS camera line: world lock poisoned")?;
                        let line = crate::collision_line_collection::CollisionLineInput {
                            start: crate::skeleton_root_transforms::SkeletonRootVector::known(start),
                            end: crate::skeleton_root_transforms::SkeletonRootVector::known(end),
                            fatness: radius,
                        };
                        let hit = world
                            .try_world_line_test(
                                ground,
                                line,
                                "User_ICEWorldManager::HandleCollision",
                                &mut crate::collision_line_numeric::HostCollisionLineNumeric,
                            )
                            .map_err(|e| format!("NIS camera line: {e:?}"))?;
                        Ok(hit.map(|hit| hit.line_parameter))
                    });
                }
                if let Some(assets) = self.inner.assets.as_ref() {
                    career.present_scene_characters(&assets.mounts, &assets.vault, &assets.cvars);
                }
                career.publish_borrowed_skaters(self.inner.presentation_runtime.as_mut(), &mut self.inner.ai_skaters, &mut self.inner.party).map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
                let input = self.inner.bound_input.as_mut().expect("ready input");
                for paused in career.take_nis_events() {
                    input.try_set_nis_pause(paused)
                        .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
                }
                if policy.has_focus {
                    if career.photo_select_open() {
                        career.photo_select_input(frontend_input);
                    } else if career.overlay_open() {
                        career.overlay_input(frontend_input);
                    }
                }
                if let Some(paused) = career.take_overlay_pause_change() {
                    input.try_set_frontend_pause(paused)
                        .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
                }
                self.try_update_career_paused()?;
                self.try_nis_breadcrumb_teleport()?;
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
        }
        // FrontEndState_Unlock's sFEStateInfo (0x82fdee48) pauses the game.
        // Its input is taken in the front end's update, which runs before
        // the simulation's: the A that closes it lets this tick simulate.
        if phone_requested && policy.has_focus && !self.pause_menu.snapshot().visible && !online_popup
            && input.try_can_open_frontend().map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?
            && self.inner.session_marker.as_ref().is_some_and(|m|m.hud.visible && !m.hud.phone_hidden && m.hud.permissions.is_some())
            && let Some(career)=self.career.as_mut().filter(|c|c.can_open_contacts()) {
            career.open_contacts(self.career_saves.busy());
        }
        if let Some(career)=self.career.as_mut(){career.set_contacts_saving(self.career_saves.busy());}
        let career_overlay_was_open = self.career.as_ref().is_some_and(career::CareerSession::overlay_open);
        if policy.has_focus
            && let Some(career) = self.career.as_mut()
            && career.overlay_open()
        {
            career.overlay_input(frontend_input);
        }
        if let Some(paused) = self.career.as_mut().and_then(career::CareerSession::take_overlay_pause_change) {
            input.try_set_frontend_pause(paused)
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        }
        let career_overlay_open = self.career.as_ref().is_some_and(career::CareerSession::overlay_open);
        if career_overlay_open {
            // ExecutePopState82d672f8 re-enters the parent from the front end,
            // even while that parent keeps FRONTEND paused. Annals' B queues
            // the results OnEnter; deliver it before retaining this frame.
            self.apply_career_outputs()?;
            // Net::ResumeGame (0x82791ca8): an online game simulates under the front end, as
            // under a contest's round results.
            if !self.online.network_resumed {
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
        }
        let input = self.inner.bound_input.as_mut().expect("ready input");
        if career_overlay_was_open && self.pause_menu.snapshot().visible {
            // ReceiveInput82656af8 delivers the key to the top state once.
            // PopState82d672f8 re-enters StartHub (OnEnter8265e4d0), opens a
            // fresh screen and waits for IntroComplete. Do not feed the Blog's
            // closing B into its parent, or release the parent's FRONTEND pause.
            self.pause_menu.reenter();
            input.try_set_frontend_pause(true)
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
            self.apply_career_outputs()?;
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        let was_paused = self.pause_menu.snapshot().visible;
        if !was_paused {
            let available=self.career.as_ref().map_or(&self.freeroam_replay,|career|career.replay()).num_frames_tracked()>=150;
            self.pause_menu.set_replay_available(available);
        }
        let can_open = input.try_can_open_frontend()
            .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?
            // CheckForOptions82656718 rejects the service/script-owned
            // mbDisableBackpackAndPauseMenu flag before opening StartHub, which the online
            // loading screen sets as well.
            && !self.career.as_ref().is_some_and(|career|career.system.world().pause_menu_disabled)
            && !online_loading_screen;
        // Keep simulation/publication running until its real registration callback
        // completes. Pausing a pending peer match would prevent the next retry.
        let player_registered = matches!(
            self.inner.player.as_ref().expect("ready player").selected.owners.loader.phase(),
            crate::player::PlayerLoadPhase::Loaded { .. }
        );
        self.pause_menu.set_main_challenge(self.career.as_ref().and_then(career::CareerSession::start_hub_main_challenge));
        self.pause_menu.set_leave_challenge(self.online.can_leave_challenge());
        // StateManagerFrontEnd::ReceiveInput 0x82656af8: nothing reaches the StartHub before its
        // screen's IntroComplete, and a front-end state over the game takes the input itself.
        let frontend_input = if self.pause_menu.awaiting_intro() || career_overlay_open {
            crate::ui::pause_menu::PauseMenuInput::default()
        } else {
            frontend_input
        };
        // StateManagerFrontEnd::ReceiveInput (0x82656af8) hands an open popup the input instead of
        // CheckForOptions.
        let popup_open = self.career.as_ref().is_some_and(career::CareerSession::popup_open);
        let pause = self.pause_menu.update(frontend_input,
            policy.has_focus && self.completed_frames != 0 && self.map_travel.is_none() && !map_challenge_loading
                && (was_paused || (can_open && player_registered && !popup_open && !hub_open)));
        if was_paused && pause.visible && !pause.jacked && policy.has_focus && frontend_input.select
            && self.pause_menu.selected_option() == crate::ui::pause_menu::PauseMenuOption::Map {
            self.map_menu.open();
            if let Some(career) = self.career.as_mut() {
                career.open_challenge_map();
            }
        }
        if was_paused && pause.visible && !pause.jacked && policy.has_focus && frontend_input.select
            && self.pause_menu.selected_option() == crate::ui::pause_menu::PauseMenuOption::Settings {
            // GameBackEnd::IsFreeSkateMode: a game without the career's challenge system.
            let video = if self.career.is_none() {
                crate::ui::options::VideoScreen::FreeSkate
            } else {
                crate::ui::options::VideoScreen::Game
            };
            self.options_menu.open(true, video, &self.audio.settings);
        }
        if was_paused && pause.visible && !pause.jacked && policy.has_focus && frontend_input.select
            && self.pause_menu.selected_option() == crate::ui::pause_menu::PauseMenuOption::Blog
            && let Some(career) = self.career.as_mut()
        {
            // IStartHubState::OnMenuSelect82674f28: PushState(SlappysBlog).
            let player = self.inner.player.as_ref().expect("ready player");
            let services = &player.selected.owners.services;
            let male = services.settings.try_character_is_male(0).unwrap_or(true);
            career.open_slappys_blog(false, male, services.profile.name().unwrap_or_default());
        }
        if was_paused && pause.visible && !pause.jacked && policy.has_focus && frontend_input.select
            && !self.pause_menu.awaiting_intro()
            && self.pause_menu.selected_option() == crate::ui::pause_menu::PauseMenuOption::Profile
            && pause.item_enabled(crate::ui::pause_menu::PauseMenuOption::Profile)
        {
            // IStartHubState::OnMenuSelect82674f28: mEntryScreenMode = eScreenMode_Offline, the
            // local peer, PushState(45).
            let player = self.inner.player.as_ref().expect("ready player");
            let decks = crate::skater_manager_local_identity::stats::profile_deck_count()
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
            let lists = player.selected.owners.services.skaters.backend().skate_profile_rows(decks)
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
            self.skate_profile.open(lists);
        }
        if was_paused != pause.visible {
            input.try_set_frontend_pause(pause.visible)
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
        }
        if !was_paused && pause.visible && !pause.jacked {
            // StartHub::OnEnter8265e548..e584 sends the default rate for one
            // tick. Keep this existing message owner's ordered dispatch alive
            // even though no simulation body runs while the frontend is open.
            let world_owner=self.inner.world.as_ref().expect("ready world");
            let assets=self.inner.assets.as_ref().expect("ready assets");
            let player=self.inner.player.as_ref().expect("ready player");
            let recipe_runtime=world_owner.living_world_recipes.lock()
                .map_err(|_|SelectedLiveError::failed(Stage::PreupdateInput,"recipe runtime lock poisoned"))?;
            let terrain_owner=self.inner.terrain.as_ref().expect("ready terrain");
            let terrain=terrain_owner.terrain_for(&world_owner.world)
                .map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
            let ground=terrain_owner.ground_pool_for(&world_owner.world)
                .map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
            let mut world=world_owner.world.lock()
                .map_err(|_|SelectedLiveError::failed(Stage::PreupdateInput,"world lock poisoned"))?;
            world.sim_rate.lock().map_err(|_|SelectedLiveError::failed(Stage::PreupdateInput,"sim rate lock poisoned"))?
                .send(crate::simulation_sim_rate::SetSimRate {
                    sim_time:crate::simulation_sim_rate::UPDATE_TIME,ticks_of_slow_motion:1,
                }).map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
            crate::selected_living_world_recipes::mood::SelectedMoodFrame {
                triggers:&terrain.triggers,runtime:&recipe_runtime,
                vault:&player.selected.owners.services.vault,painter:&terrain.layers,
            }.dispatch_game_sim(&mut world,&assets.cvars,ground)
                .map_err(|e|SelectedLiveError::failed(Stage::PreupdateInput,e))?;
        }
        if was_paused || pause.visible {
            self.update_start_hub(pause.hub)?;
            if self.pause_menu.take_leave_challenge_confirmed() {
                self.online_request_leave_challenge();
            }
            if was_paused && pause.visible && !pause.jacked && policy.has_focus && frontend_input.select
                && !self.pause_menu.awaiting_intro()
                && self.pause_menu.selected_option()==crate::ui::pause_menu::PauseMenuOption::TrickGuide
                && pause.item_enabled(crate::ui::pause_menu::PauseMenuOption::TrickGuide) {
                self.trick_guide_pending=true;self.frontend_fade.cover();
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
            if was_paused && pause.visible && !pause.jacked && policy.has_focus && frontend_input.select
                && !self.pause_menu.awaiting_intro()
                && self.pause_menu.selected_option()==crate::ui::pause_menu::PauseMenuOption::Replay
                && pause.item_enabled(crate::ui::pause_menu::PauseMenuOption::Replay) {
                self.replay_editor_pending=true;
                self.frontend_fade.cover();
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
            if self.pause_menu.take_retry_confirmed() {
                self.career_restart_main_challenge();
            }
            if was_paused && pause.visible && !pause.jacked && policy.has_focus && frontend_input.select
                && self.pause_menu.selected_option() == crate::ui::pause_menu::PauseMenuOption::EditSkater {
                self.editor_enter_pending = true;
                self.frontend_fade.cover();
            }
            // Net::ResumeGame (0x82791ca8): an online game simulates under the front end, the
            // skater without PlayerUI's focus.
            if !self.online.network_resumed {
                return Ok(SelectedQueuedFrameOutcome::Frontend);
            }
        }
        // gFEStateInfo (0x82fded80) rows 44, 68 and 69 pause as the StartHub's does: the single
        // player game does not simulate under the online hub and its screens.
        if hub_open && !self.online.network_resumed {
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        // ChallengeLoadState's kPAUSE_STATE_ONLINE: the round loads with the simulation paused.
        if self.online_round_loading() {
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        let frontend_input = if was_paused || pause.visible { Default::default() } else { frontend_input };
        let hud_dpad = if was_paused || pause.visible || hub_open || online_popup { Default::default() } else { hud_dpad };
        if policy.has_focus && !map_challenge_loading {
            self.online_vote_input(frontend_input);
            self.career_input(frontend_input, hud_dpad);
            if self.career.is_none() {self.queue_prop_input(hud_dpad);}
        }
        self.apply_prop_actions()?;
        if self.try_update_sign_up_choice_pause()? {
            return Ok(SelectedQueuedFrameOutcome::Frontend);
        }
        // FE state 8's QuitPopupCallback: QuittingToFE and Sk8AppMan::EnterFE.
        if self.take_career_quit_to_front_end() {
            self.request_main_menu()?;
        }

        if self.completed_frames != 0 {
            let profile_release = crate::diagnostics::frame_profile::span(
                crate::diagnostics::frame_profile::TICK_TERRAIN_RELEASE,
            );
            self.inner
                .terrain
                .as_mut()
                .expect("ready terrain")
                .try_advance_asset_release()
                .map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
            drop(profile_release);
            // Admit completed terrain loads between physics frames, before query borrows.
            let position = local_skater_position(
                self.inner.world.as_ref().map(|world| &world.world),
                self.inner.player.as_ref().expect("ready player").selected.actor.as_deref().expect("ready Actor"),
            )?;
            // An online game simulates under a scene (Net::ResumeGame), so the NIS's frozen
            // focus (a snapped scene's location, the restored stream position) streams here
            // rather than in the paused path, which reports it loaded back to the NIS.
            let nis_focus = self.career.as_ref().filter(|_| self.online.network_resumed).and_then(|career| career.nis.stream_focus());
            let focus = match nis_focus {
                Some(FrozenFocus::At(position)) => position,
                Some(FrozenFocus::GameCamera) => self
                    .inner
                    .world
                    .as_ref()
                    .ok_or_else(|| SelectedLiveError::failed(Stage::TerrainResident, "world absent"))?
                    .local
                    .try_camera_producer_projection()
                    .map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?
                    .camera_matrix
                    .columns()[3]
                    .xyz,
                None => self.map_travel.map_or(position, |matrix| [matrix[12], matrix[13], matrix[14]]),
            };
            let _profile_focus = crate::diagnostics::frame_profile::span(
                crate::diagnostics::frame_profile::TICK_TERRAIN_FOCUS,
            );
            let terrain = self.inner.terrain.as_mut().expect("ready terrain");
            terrain.try_update_focus(focus).map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
            if nis_focus.is_some() {
                let loaded = terrain.try_focus_loaded().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
                if let Some(career) = self.career.as_mut() {
                    career.nis.set_stream_focus_loaded(loaded);
                }
            }
        }
        if self.completed_frames == 30
            && self.inner.party.is_empty()
            && let Some(marquee) = std::env::var("SKATE2_DIAGNOSTIC_SPAWN_SKATER")
                .ok()
                .and_then(|text| u64::from_str_radix(text.trim_start_matches("0x"), 16).ok())
        {
            let mut spawn = self.inner.spawn.ok_or_else(|| SelectedLiveError::failed(Stage::ControlledPlayer, "spawn absent"))?;
            spawn[12] += 1.5;
            {
                let services = &self.inner.player.as_ref().expect("ready player").selected.owners.services;
                services
                    .settings
                    .try_create_offline_human(1, &services.profile, services.skaters.backend())
                    .map_err(|e| SelectedLiveError::failed(Stage::ControlledPlayer, e))?;
            }
            self.inner
                .try_spawn_party_skater(1, super::party_skaters::PartyCharacter::Marquee(marquee), spawn)
                .map_err(|e| SelectedLiveError::failed(Stage::ControlledPlayer, e))?;
            eprintln!("diagnostic spawned skater 1 ({marquee:#018x}) at {:?}", &spawn[12..15]);
        }
        // AISkaterManager in game: GetCameraReferenceMatrix 0x824ba008 is the game camera, and
        // the local player's skater is a recipe in use.
        if self.inner.ai.is_some() {
            let camera = self
                .inner
                .world
                .as_ref()
                .expect("ready world")
                .local
                .try_camera_producer_projection()
                .map_err(|e| SelectedLiveError::failed(Stage::QueuedFrame, e))?
                .camera_matrix;
            let recipe = self.inner.player.as_ref().expect("ready player").selected.owners.services.settings.try_character(0).ok();
            let living_world_recipes = Arc::clone(&self.inner.player.as_ref().expect("ready player").selected.owners.living_world_recipes);
            let party_play = self.party_play.game.is_some();
            let ai = self.inner.ai.as_mut().expect("checked session AI");
            ai.camera = std::array::from_fn(|row| {
                let column = camera.columns()[row];
                [column.xyz[0], column.xyz[1], column.xyz[2], column.w.unwrap_or(0.0)]
            });
            ai.party_play = party_play;
            let mut in_use: Vec<u64> = recipe.into_iter().collect();
            if let Ok(runtime) = living_world_recipes.lock() {
                in_use.extend(ai.recipes_used_by_living_world(|name| runtime.recipes.is_tracked(name)));
            }
            ai.set_recipes_in_use(in_use);
        }
        let world = self.inner.world.as_ref().expect("ready world");
        let assets = self.inner.assets.as_ref().expect("ready assets");
        let cvars = assets.cvars.clone();
        let vault = Arc::clone(&assets.vault);
        let (mut sync, mut feedback, overlay) = {
            let world = world.world.lock().map_err(|_| {
                SelectedLiveError::failed(Stage::QueuedFrame, "world lock poisoned")
            })?;
            let services = world
                .physical_world_services()
                .ok_or(SelectedLiveError::Pending(
                    SelectedLivePending::FrameNotReady,
                ))?;
            (
                Arc::clone(services.game_info()),
                world
                    .try_live_contact_feedback(&cvars)
                    .map_err(|e| SelectedLiveError::failed(Stage::QueuedFrame, e))?,
                world.debug_overlay.clone(),
            )
        };
        let online_boundaries = self.online_held_boundaries();
        let offboard_stick = self.offboard_stick_frames();
        let mut boundary_teleport = false;
        let input = self.inner.bound_input.as_mut().expect("ready input");
        let selected = &self.inner.player.as_ref().expect("ready player").selected;
        // PlayerUI::Update samples for its mPlayerIndex, which PartyPlayUI moves.
        let loader = self
            .inner
            .party
            .iter()
            .map(|skater| &skater.loader)
            .find(|loader| loader.player_index() == input.player_index())
            .unwrap_or(&selected.owners.loader);
        // Sk8AppMan::PreUpdate8272c4d8: without SIM input neither Flow nor
        // the input-gated GameModule update runs.
        let profile_marker = crate::diagnostics::frame_profile::span(
            crate::diagnostics::frame_profile::TICK_SESSION_MARKER,
        );
        if has_sim_input {
            let challenge = self.career.as_ref().map(career::CareerSession::session_marker_rules);
            // GameModule::DoGameUpdate (0x82730418): PlayerUI has the focus only while no popup is
            // open and PauseMan's top state lets the front end open (CheckForPauseState).
            let game_focus = policy.has_focus && !map_challenge_loading
                && !self.career.as_ref().is_some_and(career::CareerSession::popup_open)
                && input.try_can_open_frontend().map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
            let marker = self
                .inner
                .session_marker
                .as_mut()
                .expect("ready session marker");
            let terrain = self.inner.terrain.as_mut().expect("ready terrain");
            // 8272c590 Flow::Manager::Update: DoTeleport's MoveStreamFocus,
            // then SendTeleportMsg, for a request from an earlier frame.
            marker
                .teleport
                .try_pump_flow(|position| terrain.try_update_focus(position.xyz))
                .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
            let local = world
                .world
                .lock()
                .map_err(|_| {
                    SelectedLiveError::failed(Stage::SessionMarker, "world lock poisoned")
                })?
                .local_skater_identity
                .clone()
                .ok_or_else(|| {
                    SelectedLiveError::failed(
                        Stage::SessionMarker,
                        "SkaterManager local identity absent",
                    )
                })?;
            // GameModule::DoGameUpdate8273049c: SessionMarker::Update (+0x50).
            if let Some(session) = marker.marker.as_mut() {
                session
                    .try_update(&mut SelectedSessionMarkerSources {
                        local: &local,
                        world: &world.world,
                        terrain: &**terrain,
                        cvars: &cvars,
                        game_info: &sync,
                        input: &*input,
                        challenge,
                    })
                    .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
            }
            // GameUI::Update -> PlayerUI::Update828af9f8.
            if input
                .try_player_ui_prefix::<P>(policy.can_simulate, game_focus)
                .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?
            {
                // 828afa78: UpdateSessionMarker needs the marker and focus.
                if let Some(session) = marker.marker.as_mut().filter(|_| game_focus) {
                    let request = marker
                        .player_ui
                        .try_update(
                            session,
                            &mut SelectedSessionMarkerSources {
                                local: &local,
                                world: &world.world,
                                terrain: &**terrain,
                                cvars: &cvars,
                                game_info: &sync,
                                input: &*input,
                                challenge,
                            },
                        )
                        .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                    if let Some(request) = request {
                        marker.restore_props = true;
                        marker.teleport.send_flow(request);
                    }
                }
                // 828afa98: with a network query, the local skater's respawn transform outside
                // the game's boundaries sends it to its suggested breadcrumb.
                if let Some(boundaries) = online_boundaries.as_ref() {
                    let respawn = crate::session_marker::player_ui::PlayerUiSessionMarkerSource::respawn_transform(
                        &mut SelectedSessionMarkerSources {
                            local: &local,
                            world: &world.world,
                            terrain: &**terrain,
                            cvars: &cvars,
                            game_info: &sync,
                            input: &*input,
                            challenge,
                        },
                    )
                    .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                    boundary_teleport = !boundaries.is_position_ok(respawn.columns()[3].xyz, 0.0);
                }
                marker.hud = crate::ui::phonelist::MarkerHudSnapshot {
                    visible: policy.has_focus,
                    bumper_held: input
                        .try_session_marker_state(29)
                        .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?,
                    permissions: marker.marker.as_ref().map(|session| {
                        crate::ui::phonelist::MarkerPermissions {
                            use_allowed: session.is_use_allowed(),
                            set_allowed: session.is_set_allowed(),
                        }
                    }),
                    successful_sets: marker.player_ui.successful_sets,
                    ..marker.hud
                };
                input
                    .try_player_ui_sample_and_sync::<P>(loader, game_focus, offboard_stick.as_ref(), &sync)
                    .map_err(|e| SelectedLiveError::failed(Stage::PreupdateInput, e))?;
            }
        }
        drop(profile_marker);
        self.apply_prop_actions()?;
        if boundary_teleport && let Some(crumb) = self.local_suggested_breadcrumb()? {
            self.inner
                .session_marker
                .as_mut()
                .expect("ready session marker")
                .teleport
                .send_breadcrumb_teleport(crumb.transform, crate::session_marker::FootForward::from(!crumb.foot_forward_right), !crumb.offboard);
        }
        let input = self.inner.bound_input.as_mut().expect("ready input");
        // SimController::Update1 Dispatch828a739c: eEQGameSim reaches
        // PlayerUI before this frame's queued commands.
        let player_index = i32::from_ne_bytes(input.player_index().to_ne_bytes());
        let session_marker = self.inner.session_marker.as_mut().expect("ready session marker");
        session_marker
            .teleport
            .try_dispatch_game_sim(player_index, input, session_marker.marker.as_mut())
            .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
        let mut recognizer = GameInputRuntimeServices::new(&cvars, &vault, &overlay, None);
        let mut numeric = SelectedTickNumerics {
            input: HostReckoningNumeric,
            actor: HostReckoningNumeric,
            pre: HostGroundPreparationNumeric::default(),
            line: crate::collision_line_numeric::HostCollisionLineNumeric,
            vector: HostReckoningNumeric,
            rounding: HostReckoningSinCosNumeric,
            truck: crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
            collision: crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
        };
        let trace_frame = self.completed_frames.saturating_add(1);
        let trace = std::env::var_os("SKATE2_TRACE_ADJUSTED_POSE")
            .is_some()
            .then(|| {
                let bound = |name: &str, default| {
                    std::env::var(name)
                        .ok()
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(default)
                };
                (bound("SKATE2_TRACE_ADJUSTED_POSE_FROM", 1)
                    ..=bound("SKATE2_TRACE_ADJUSTED_POSE_THROUGH", 30))
                    .contains(&trace_frame)
            })
            .filter(|enabled| *enabled)
            .map(|_| RefCell::new(Vec::<String>::new()));
        let profile_queued = crate::diagnostics::frame_profile::span(
            crate::diagnostics::frame_profile::TICK_QUEUED_FRAME,
        );
        let result = self
            .inner
            .try_with_selected_queued_frame_challenge(
                self.career.as_mut().map(|career| career.system.world_mut()),
                |mut frame, ground, queued| {
                let board = frame.board;
                let skeleton = frame.skeleton;
                let presentation = frame.presentation;
                if trace_frame <= 2 {
                    if let Some(trace) = trace.as_ref() {
                        let board_parts: [Option<_>; 7] = std::array::from_fn(|index| {
                            frame.world.board(board).ok().and_then(|part| {
                                part.try_part_contact_spy_observation(frame.world.backend(), index)
                                    .ok()
                            })
                        });
                        let board_positions: [Option<[f32; 3]>; 7] =
                            std::array::from_fn(|index| {
                                frame.world.board(board).ok().and_then(|part| {
                                    part.try_live_part_transform_host(frame.world.backend(), index)
                                        .ok()
                                        .map(|matrix| [matrix[12], matrix[13], matrix[14]])
                                })
                            });
                        trace.borrow_mut().push(format!(
                            "adjusted-pose frame={trace_frame} board-part-bodies={board_parts:?} board-part-positions={board_positions:?}"
                        ));
                    }
                }
                let physical = trace.as_ref().and_then(|_| {
                    frame
                        .actor
                        .cached_physical_allocation()
                        .ok()
                        .map(|allocation| Arc::clone(allocation.owner()))
                });
                let mut observe_tick = |stage: SelectedTickStage,
                                        world: &crate::physical_player_assembly::PhysicalAssemblyWorld| {
                    let Some(trace) = trace.as_ref() else { return };
                    let state = physical.as_ref().and_then(|owner| {
                        owner.physics_state().try_lock().ok().map(|state| {
                            (state.current, state.frames_since_jump)
                        })
                    });
                    let processed = physical.as_ref().and_then(|owner| {
                        let reckoning = owner.reckoning().try_lock().ok()?;
                        let processed = reckoning.processed().try_lock().ok()?;
                        Some((
                            processed.physics_state_fields().num_wheels_in_contact,
                            processed.known_flags_8c4(),
                        ))
                    });
                    let wheels = world.board_construction(board).ok().flatten().and_then(|body| {
                        let lines = body.wheel_lines().try_lock().ok()?;
                        Some((
                            lines.distances,
                            lines.height,
                            body.post_fields().map(|post| post.time_since_wheels_touching),
                        ))
                    });
                    let board_part6 = world.board(board).ok().and_then(|part| {
                        part.try_live_part_transform_host(world.backend(), 6).ok()
                    });
                    let board_part0 = world.board(board).ok().and_then(|part| {
                        part.try_live_part_transform_host(world.backend(), 0).ok()
                    });
                    let skeleton_parts = matches!(stage, SelectedTickStage::AfterSolver).then(|| {
                        [1_usize, 23].map(|part| {
                            world.skeleton(skeleton).ok().and_then(|assembly| {
                                assembly
                                    .session()
                                    .try_live_part_transform_host(world.backend(), part)
                                    .ok()
                                    .map(|matrix| {
                                        (
                                            [matrix[12], matrix[13], matrix[14]],
                                            [matrix[4], matrix[5], matrix[6]],
                                        )
                                    })
                            })
                        })
                    });
                    let board_velocity = if trace_frame <= 2 {
                        world.board(board).ok().and_then(|part| {
                            crate::dynamic_assembly_session::skater_pool::board_entry::try_current_board_deck_velocity(
                                part,
                                world.backend(),
                            )
                            .ok()
                        })
                    } else {
                        None
                    };
                    // AfterSolver precedes post-solve/PhysOut fill, so this
                    // output belongs to the previous completed frame.
                    let phys_root = physical.as_ref().and_then(|owner| {
                        let output = owner.phys_out().try_lock().ok()?;
                        output.try_skeleton_output().ok().map(|skeleton| skeleton.skeleton_root_matrix)
                    });
                    let stage_name = match stage {
                        SelectedTickStage::BeforeActor => "BeforeActor",
                        SelectedTickStage::BeforePreSolve => "BeforePreSolve",
                        SelectedTickStage::BeforeSolver => "BeforeSolver",
                        SelectedTickStage::AfterSolver => "AfterSolver",
                    };
                    trace.borrow_mut().push(format!(
                        "adjusted-pose frame={trace_frame} stage={stage_name} state/jump={state:?} processed-wheels/flags={processed:?} wheel-distances/height/touch-time={wheels:?} board-part0={board_part0:?} board-part6={board_part6:?} skeleton-parts1-23-position/up={skeleton_parts:?} board-deck-velocity={board_velocity:?} physout-root(previous-postsolve)={phys_root:?}"
                    ));
                };
                let mut observe_presentation = |stage: SelectedPresentationStage,
                                                runtime: &crate::actor_face_playback::presentation::frame::selected_runtime::SelectedPresentationRuntime,
                                                submitted: &crate::actor_face_playback::presentation::frame::SubmittedPresentationFrame| {
                    if matches!(trace_frame, 1 | 30) || matches!(stage, SelectedPresentationStage::Produced | SelectedPresentationStage::Published) {
                        if let Some(trace) = trace.as_ref() {
                            let root = runtime.producer().root_position(submitted, 0).ok();
                            let hips = presentation.animation.hips.and_then(|bone| usize::try_from(bone).ok());
                            let replay_hips = (!matches!(stage, SelectedPresentationStage::Produced))
                                .then(|| hips.and_then(|bone| presentation.animation.replay.first()?.read().ok()?.1.get(bone).copied()))
                                .flatten();
                            let saved_hips = matches!(stage, SelectedPresentationStage::Corrected | SelectedPresentationStage::Finalized | SelectedPresentationStage::Published)
                                .then(|| presentation.animation.hips_transforms.try_lock().ok().and_then(|saved| saved[0]))
                                .flatten();
                            let final_hips = matches!(stage, SelectedPresentationStage::Finalized | SelectedPresentationStage::Published)
                                .then(|| hips.and_then(|bone| presentation.animation.interpolated.first()?.matrices().ok()??.get(bone).copied()))
                                .flatten();
                            let header_pose = matches!(stage, SelectedPresentationStage::Produced)
                                .then(|| {
                                    let view = runtime.producer().read_frame(submitted, 0).ok()?;
                                    let matrix = |base: usize| -> Option<[Option<f32>; 16]> {
                                        Some(std::array::from_fn(|lane| {
                                            let bytes: [u8; 4] = view.read_base
                                                .get(base + lane * 4..base + lane * 4 + 4)?
                                                .try_into().ok()?;
                                            Some(f32::from_bits(u32::from_be_bytes(bytes)))
                                        }))
                                    };
                                    Some((matrix(0)?, matrix(64)?))
                                })
                                .flatten();
                            let rows = if matches!(stage, SelectedPresentationStage::Published) {
                                runtime.palette().ok().and_then(|palette| {
                                    palette.mesh_rows(0, 0).ok().map(|mesh| {
                                        (mesh.generation, mesh.rows.get(..3).map(<[[f32; 4]]>::to_vec))
                                    })
                                })
                            } else { None };
                            trace.borrow_mut().push(format!(
                                "adjusted-pose frame={trace_frame} stage={stage:?} submitted-root={root:?} header-hips/board={header_pose:?} replay-hips={replay_hips:?} saved-hips={saved_hips:?} final-hips={final_hips:?} published-part0-mesh0-first-rows={rows:?}"
                            ));
                        }
                    }
                    Ok::<(), Infallible>(())
                };
                let tick_observer: Option<
                    &mut dyn FnMut(
                        SelectedTickStage,
                        &crate::physical_player_assembly::PhysicalAssemblyWorld,
                    ),
                > = if trace.is_some() {
                    Some(&mut observe_tick)
                } else {
                    None
                };
                let presentation_observer: Option<
                    &mut dyn FnMut(
                        SelectedPresentationStage,
                        &crate::actor_face_playback::presentation::frame::selected_runtime::SelectedPresentationRuntime,
                        &crate::actor_face_playback::presentation::frame::SubmittedPresentationFrame,
                    ) -> Result<(), Infallible>,
                > = if trace.is_some() {
                    Some(&mut observe_presentation)
                } else {
                    None
                };
                let outcome = frame.try_step_queued::<_, _, _, _, _, _, std::convert::Infallible>(
                    SelectedTickRequest {
                        input: queued,
                        dt: policy.dt,
                        lod: policy.lod,
                        wait_for_bs_job: policy.wait_for_bs_job,
                        physics: SkateboardGroundFrameInput {
                            schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                            ground_pool: ground,
                            ground_group: policy.ground_group,
                            pipeline_culling: None,
                            allow_multiple_writes: policy.allow_multiple_writes,
                            spy_flags: policy.spy_flags,
                        },
                        pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                        post_grind: GrindManagerFillPolicy::RequireNative,
                    },
                    &mut numeric,
                    &mut sync,
                    &mut recognizer,
                    &mut feedback,
                    clamp,
                    &mut HostReckoningNumeric,
                    tick_observer,
                    presentation_observer,
                );
                if outcome.is_err() {
                    if let Some(trace) = trace.as_ref() {
                        for line in trace.borrow().iter() { eprintln!("{line}"); }
                    }
                }
                if matches!(&outcome, Ok(SelectedQueuedTickOutcome::Completed(_)))
                    && matches!(trace_frame, 1 | 30)
                {
                    if let Some(trace) = trace.as_ref() {
                        use crate::actor_construction::anim_out_phys_in::pose_lanes::PoseBuffer;
                        let physical_hips = frame.animation.hierarchy.lock().ok().and_then(|db| {
                            db.parsed.as_ref()?.hierarchy.bone_names.iter().position(|name| {
                                name.eq_ignore_ascii_case("Hips")
                            })
                        });
                        let history = frame.world.skeleton(skeleton).ok().map(|assembly| {
                            let data = &assembly.state().binding.body().data;
                            let hips_part = physical_hips
                                .and_then(|bone| data.part_from_bone().get(bone).copied().flatten());
                            let mapped_23 = data.bone_ids()[23];
                            (
                                assembly.physics_history_frame_state().progress(),
                                hips_part.map(|part| (
                                    part.index(),
                                    assembly.physics_history().post_wipeout_record_transform(part.index()),
                                    data.volume_to_bone()[part.index()],
                                )),
                                (mapped_23, data.hierarchy().bone_names.get(mapped_23),
                                 assembly.physics_history().post_wipeout_record_transform(23)),
                                assembly.root_transforms().world_to_animation(),
                            )
                        });
                        let input_pose = physical_hips.and_then(|bone| {
                            let owner = frame.actor.published_animation_physics_input().ok()?;
                            let pose = owner.lock().ok()?;
                            Some((
                                bone,
                                pose.pose_matrix_typed(PoseBuffer::Global, bone).ok(),
                                pose.pose_matrix_typed(PoseBuffer::Local, bone).ok(),
                            ))
                        });
                        let presentation_hips = frame.presentation.animation.hips.and_then(|bone| {
                            usize::try_from(bone).ok()
                        });
                        let presentation_pose = presentation_hips.and_then(|bone| {
                            let replay = frame.presentation.animation.replay.first()?.read().ok()?.1;
                            let final_pose = frame.presentation.animation.interpolated.first()?
                                .matrices().ok()??;
                            Some((bone, replay.get(bone).copied(), final_pose.get(bone).copied()))
                        });
                        trace.borrow_mut().push(format!(
                            "adjusted-pose frame={trace_frame} completed-pose physics-hips/part23={history:?} animout-hips-global/local={input_pose:?} presentation-replay/final-hips={presentation_pose:?}"
                        ));
                    }
                }
                if trace_frame <= 2
                    && matches!(&outcome, Ok(SelectedQueuedTickOutcome::Completed(_)))
                {
                    if let Some(trace) = trace.as_ref() {
                        match frame.world.contact_spies() {
                            Ok(Some(spies)) => {
                                for (index, spy) in spies.records().iter().enumerate() {
                                    trace.borrow_mut().push(format!(
                                        "adjusted-pose frame={trace_frame} solved-contact[{index}] body-a={:?} body-b={:?} frame={:?} normal-force={:?} friction-force={:?} tag={:#x}",
                                        spy.body_a(),
                                        spy.body_b(),
                                        spy.frame(),
                                        spy.normal_force(),
                                        spy.friction_force(),
                                        spy.tag(),
                                    ));
                                }
                            }
                            Ok(None) => trace.borrow_mut().push(format!(
                                "adjusted-pose frame={trace_frame} solved-contacts=none"
                            )),
                            Err(error) => trace.borrow_mut().push(format!(
                                "adjusted-pose frame={trace_frame} solved-contacts-error={error:?}"
                            )),
                        }
                    }
                }
                outcome
            })
            .map_err(|e| SelectedLiveError::failed(Stage::QueuedFrame, e))?
            .map_err(|e| SelectedLiveError::failed(Stage::QueuedFrame, e))?;
        drop(profile_queued);
        match result {
            SelectedQueuedTickOutcome::NoCommands(report) => {
                Ok(SelectedQueuedFrameOutcome::NoCommands {
                    update_mask: report.update_mask,
                    delivered: report.delivered,
                })
            }
            SelectedQueuedTickOutcome::Completed(completed) => {
                let SelectedTickInputDelivery::Queued { commands, .. } = &completed.input else {
                    return Err(SelectedLiveError::failed(
                        Stage::QueuedFrame,
                        "immediate input in queued frame",
                    ));
                };
                self.try_restore_skater_render_opacity()
                    .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
                self.last_step_completed = true;
                self.complete_character_registration()?;
                self.inner
                    .try_complete_party_registration()
                    .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
                self.inner
                    .try_dispatch_ai_messages()
                    .map_err(|e| SelectedLiveError::failed(Stage::ControlledPlayer, e))?;
                self.completed_frames = self.completed_frames.saturating_add(1);
                if std::mem::take(&mut self.career_start_pending) {
                    self.start_career(true)?;
                }
                if self.career_after_editor && self.editor.is_none() && !self.editor_enter_pending
                    && matches!(self.inner.player.as_ref().expect("ready player").selected.owners.loader.phase(),
                        crate::player::PlayerLoadPhase::Loaded { .. })
                {
                    self.editor_enter_pending = true;
                }
                if !self.career_spawn_pending {
                    let _profile = crate::diagnostics::frame_profile::span(crate::diagnostics::frame_profile::CAREER);
                    self.try_update_prop_hud()?;
                    self.try_update_career()?;
                    // PC policy (2026-10-12): Hall of Meat stays at normal speed.
                    // Deliberately omit VDWriter::UpdateHoMSlowMotion (0x827fd448)
                    // and its one-tick SetSimRate request; recording/FX still run.
                    self.record_replay_frame()?;
                    self.online_tick()?;
                    self.update_online_dmos()?;
                    self.try_nis_breadcrumb_teleport()?;
                }
                if let Some(location) = self.career.as_mut().and_then(career::CareerSession::take_party_switch) {
                    self.switch_party_play(location)?;
                }
                if self.map_travel_dispatched {
                    if self.map_travel_ready_seen {
                        // CheckForTeleport82e01db8 consumes the PRIOR ready output
                        // in Update1. Keep both stream focuses pinned through that tick.
                        self.map_travel = None;
                        self.map_travel_dispatched = false;
                        self.map_travel_generation = None;
                        self.map_travel_ready_seen = false;
                        self.frontend_reveal_pending = !self.career.as_mut()
                            .is_some_and(career::CareerSession::map_challenge_teleport_complete);
                        self.career_spawn_pending = false;
                    } else {
                        let actor = self.inner.player.as_ref().expect("ready player").selected.actor
                            .as_ref().expect("ready Actor");
                        let (generation, reset) = actor.try_teleport_publication()
                            .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                        let physical = actor.cached_physical_allocation()
                            .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                        let output = physical.owner().phys_out().lock()
                            .map_err(|_| SelectedLiveError::failed(Stage::SessionMarker, "PhysOut lock poisoned"))?;
                        let published = output.try_completed_teleport()
                            .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, e))?;
                        self.map_travel_ready_seen = Some(generation) == self.map_travel_generation
                            && published.is_some_and(|transform| transform.columns() == reset.columns());
                        // A game start's skater has landed where SkaterManager::OnCommand case 3
                        // spawns a new Actor, whose breadcrumbs start over at the spawn.
                        if self.map_travel_ready_seen && self.map_travel_on_board {
                            let right = actor.try_current_foot_forward_with_fakie()
                                .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, format!("{e:?}")))?;
                            let crumbs = actor.retained_breadcrumbs()
                                .map_err(|e| SelectedLiveError::failed(Stage::SessionMarker, format!("{e:?}")))?;
                            if let Some(crumbs) = crumbs {
                                crumbs.lock()
                                    .map_err(|_| SelectedLiveError::failed(Stage::SessionMarker, "breadcrumb lock poisoned"))?
                                    .respawn(reset, right);
                            }
                        }
                    }
                }
                // EnterGame's party members are all loaded before StartGameModule, so the later
                // players spawn once the location loaded and the first player's skater is in.
                if self.map_travel.is_none() && !self.front_end_character_pending() {
                    if let Some((game, location)) = self.party_play.spawn.take() {
                        self.spawn_party_players(&game, location)?;
                    }
                }
                self.out_of_boundary_amount = self.inner.session_marker.as_mut()
                    .expect("ready session marker").player_ui.take_out_of_boundary_amount();
                // Sk8ChallengeSystem::Update: InvisibleBoundaryManager::Update's amount reaches the
                // VDWriter after PlayerUI's.
                if let Some(amount) = self.online_boundary_amount()? {
                    self.out_of_boundary_amount = Some(amount);
                }
                if let Some(trace) = trace {
                    if trace_frame <= 2 {
                        eprintln!(
                            "adjusted-pose frame={trace_frame} completed counts={:?} ground-report={:?}",
                            completed.counts, completed.report
                        );
                    }
                    for line in trace.into_inner() {
                        eprintln!("{line}");
                    }
                }
                Ok(SelectedQueuedFrameOutcome::Completed {
                    update_mask: commands.update_mask,
                    delivered: commands.delivered,
                    missing_receivers: commands.missing_receivers,
                })
            }
        }
    }

    /// Frontend state is available independently of completed simulation frames.
    #[must_use]
    pub fn pause_menu_snapshot(&self) -> crate::ui::pause_menu::PauseMenuSnapshot {
        self.pause_menu.snapshot()
    }
    /// The in-game front-end state on top, as `UpdateFEAudioState` reports it to the audio system:
    /// the pause menu's screens, else the career's; `None` in gameplay, which leaves the HUD
    /// stack's state, and in the main menu.
    #[must_use]
    pub fn frontend_audio_state(&self) -> Option<crate::ui::FrontendAudioState> {
        use crate::ui::FrontendAudioState as State;
        if self.trick_guide.is_some() {return Some(State::TRICK_GUIDE);}
        if let Some(editor)=&self.replay_editor {
            return Some(if editor.controller.browsing_footage() {State::REPLAY_FOOTAGE} else {State::REPLAY_EDITOR});
        }
        if let Some(reel)=self.main_menu.as_ref().and_then(|menu|menu.skate_reel.as_ref()) {
            use crate::ui::skate_reel::Page;
            if let Some(screen)=self.online_match_snapshot() {
                return Some(if screen.freeskate {State::UNRANKED_FREESKATE} else {State::UNRANKED_CUSTOM_MATCH});
            }
            // The Lobby Browser shows on FE state 69's screen.
            if self.lobby_browser_snapshot().is_some() {return Some(State::UNRANKED_CUSTOM_MATCH);}
            return Some(match reel.page {
                Page::Online=>State::ONLINE_OPTIONS,
                Page::General=>State::SKATE_REEL_GENERAL,
                Page::Footage=>State::REPLAY_FOOTAGE,
                Page::Photo(_)=>State::SKATE_REEL_PHOTO,
                Page::Replay=>State::REPLAY_EDITOR,
            });
        }
        if self.movie_state_open() {
            return Some(if self.movie_background_forced() { State::CREDITS } else { State::MOVIE_PLAYER });
        }
        if let Some(extras) = self.main_menu.as_ref().and_then(|menu| menu.extras.as_ref()) {
            return Some(if matches!(extras.state.page, crate::ui::extras::Page::Code { .. }) {
                State::KEYBOARD
            } else { State::EXTRAS });
        }
        if self.main_menu.is_some() {
            return None;
        }
        let pause = self.pause_menu.snapshot();
        if self.editor.is_some() {
            Some(State::CAC)
        } else if self.map_menu.snapshot().visible {
            Some(State::CHALLENGE_MAP)
        } else if self.audio.menu.visible() {
            Some(State::MUSIC_PLAYER)
        } else if self.options_menu.visible() {
            Some(State::OPTIONS)
        } else if self.skate_profile.visible() {
            Some(State::SKATE_PROFILE)
        } else if pause.jacked {
            Some(State::JACKED)
        } else if pause.visible {
            Some(State::START_HUB)
        } else if let Some(screen) = self.online_hub_match_snapshot() {
            Some(if screen.freeskate { State::UNRANKED_FREESKATE } else { State::UNRANKED_CUSTOM_MATCH })
        } else if self.online_hub_open() {
            Some(State::ONLINE_HUB)
        } else {
            self.career.as_ref().and_then(career::CareerSession::frontend_audio_state)
        }
    }
    /// `PauseMan::IsPaused(kPAUSE_FLAG_SIM)` (`0x824e4778`), which `SFXObj_Pause::UpdateParams`
    /// (`0x82532f48`) reads: the pause stack (`kPAUSE_STATE_FRONTEND`, `kPAUSE_STATE_NIS` from
    /// `NISManager::PauseSim` to `ResumeSim`), the career's `kPAUSE_STATE_CHALLENGE`, and the
    /// front-end state on top whose `gFEStateInfo` row pauses (`UpdateOnEnter` `0x82d663f0` pushes
    /// `kPAUSE_STATE_FRONTEND` for it). All of them carry `kPAUSE_FLAG_SIM`. The main menu holds
    /// the stack's `kPAUSE_STATE_FRONTEND` for its pinned player, which the native main menu rows
    /// do not, so there only the reported state counts.
    #[must_use]
    pub fn audio_paused(&self) -> bool {
        // Net::ResumeGame masks the front end's kPAUSE_STATE_FRONTEND in an online game.
        let frontend = !self.online.network_resumed && self.frontend_audio_state().is_some_and(|state| state.paused);
        if self.main_menu.is_some() {
            return frontend;
        }
        let stack = self.inner.bound_input.as_ref().is_some_and(|input| input.try_is_simulation_paused().unwrap_or(false));
        let challenge = self.career.as_ref().is_some_and(|career| career.challenge_paused);
        stack || challenge || frontend
    }
    /// `FrontEndManager::IsExitingToMainMenu`: Quit was confirmed and the main menu is not up yet.
    #[must_use]
    pub fn exiting_to_main_menu(&self) -> bool {
        self.main_menu_pending
    }
    /// PC: Exit to Desktop's fade covers the screen and no save is being written, so the host
    /// closes the window.
    #[must_use]
    pub fn exit_to_desktop_ready(&self) -> bool {
        self.exit_to_desktop && self.frontend_fade.covered() && !self.career_saves.busy()
    }
    /// PC: the app's updater as it stands, for the main menu's update popup. Once the new release
    /// is in place the game closes as Exit to Desktop does, and the app starts it again.
    pub fn set_update_status(&mut self, status: &crate::ui::update_prompt::UpdateStatus, current: &str) {
        let Some(menu) = self.main_menu.as_mut() else { return };
        menu.set_update_status(status, current);
        if matches!(menu.update_prompt, Some(crate::ui::update_prompt::UpdatePrompt::Installed { .. })) && !self.exit_to_desktop {
            menu.leaving = true;
            self.exit_to_desktop = true;
            self.frontend_fade.cover();
        }
    }
    /// PC: what the main menu asked of the app's updater since the last call.
    pub fn take_update_request(&mut self) -> Option<crate::ui::update_prompt::UpdateRequest> {
        self.update_request.take()
    }
    /// `AudioSystem::mbIsFE`: set by the constructor and `ExitGamePlay` 0x824ed990, cleared by
    /// `ExitFE` 0x825087c0 once the game load starts; the main menu and the new career's intro
    /// movies are the front end.
    #[must_use]
    pub fn audio_is_fe(&self) -> bool {
        self.replay_editor.is_none() && (self.main_menu.is_some() || self.new_career_intro_running())
    }
    /// `VDPresentationState::mOutData.bFadeInfo`/`sFadeInfo`: the challenge effects'
    /// `cMsgFadeAmount` this update (`cChallengeEffectsManager::Update` 0x82daa1a0), none without a
    /// fade running.
    #[must_use]
    pub fn audio_fade_amount(&self) -> Option<f32> {
        self.career.as_ref().and_then(career::CareerSession::fade_amount)
    }
    /// `AudioSystem::mNISFadeAmount`: the NIS transition's bloom to white, 0 without a career.
    #[must_use]
    pub fn audio_nis_fade_amount(&self) -> f32 {
        self.nis_bloom_to_white()
    }
    /// The NIS transition's bloom to white (the post-fx blocks' `bloomToWhiteValue`), 0 without
    /// a career.
    #[must_use]
    pub fn nis_bloom_to_white(&self) -> f32 {
        self.career.as_ref().map_or(0.0, |career| career.nis.bloom_to_white())
    }
    /// The `IAudioInterface::PlayNISSfx` ids the NIS's audio effect channel fired since the last
    /// call.
    pub fn take_nis_sfx(&mut self) -> Vec<i32> {
        self.career.as_mut().map_or_else(Vec::new, |career| career.nis.take_nis_sfx())
    }
    /// The challenge `AudioSystem::UpdatePresentationData` (`0x82504848`) reads from the audio
    /// presentation's `GlobalData`: the main challenge's type (`eChallengeTypes_Unknown` without
    /// one), host and large-arena flag, and the game mode.
    #[must_use]
    pub fn audio_challenge(&self) -> (i32, Option<i32>, bool, i32) {
        self.career.as_ref().map_or((52, None, false, 0), career::CareerSession::audio_challenge)
    }
    /// Whether the main challenge is one `UpdatePresentationData` (`0x82504848`) names: the
    /// street GVR, Danny Way's best trick or the mega jam.
    #[must_use]
    pub fn audio_challenge_names(&self) -> (bool, bool, bool) {
        self.career.as_ref().map_or((false, false, false), career::CareerSession::audio_challenge_names)
    }
    /// The player bits `AudioPresentation::DoSimSubmit` (`0x827fb2f8`) packs into `GlobalData`:
    /// the local skater's `PhysOut_Intents::mIsPlayerProvidingInput` (no without a local skater)
    /// and whether player 1's character is male (`CACRecord::IsCharacterMale`, male without a
    /// record).
    #[must_use]
    pub fn audio_player_flags(&self) -> (bool, bool) {
        let providing_input = self.audio_local_actor().and_then(|actor| {
            let physical = actor.cached_physical_allocation().ok()?;
            let output = physical.owner().phys_out().lock().ok()?;
            Some(output.try_camera_intents_output().ok()?.is_player_providing_input)
        });
        let is_male = self.inner.player.as_ref().and_then(|player| player.selected.owners.services.settings.try_character_is_male(0).ok());
        (providing_input.unwrap_or(false), is_male.unwrap_or(true))
    }
    /// `GlobalData.localRaceRanking`, which `AudioPresentation::DoSimSubmit` (`0x827fb2f8`) takes
    /// from `cChallengeRaceManager::GetPlayerRank` of the local skater's player: -1 outside a race.
    #[must_use]
    pub fn audio_local_race_ranking(&self) -> i32 {
        self.career.as_ref().map_or(-1, career::CareerSession::audio_local_race_ranking)
    }
    /// The quit to the main menu: its presentation loaded, the front end covered, then the
    /// game unloads.
    fn request_main_menu(&mut self) -> Result<(), SelectedLiveError> {
        if self.main_menu_assets.is_none() {
            self.main_menu_assets = Some(
                crate::ui::main_menu::Presentation::load(&self.inner.assets.as_ref().expect("ready assets").mounts)
                    .map_err(|e| SelectedLiveError::failed(SelectedLiveStage::Visual, e.to_string()))?,
            );
        }
        self.main_menu_pending = true;
        self.frontend_fade.cover();
        Ok(())
    }

    /// The front-end sound requests since the last call, in order.
    pub fn take_menu_sounds(&mut self) -> Vec<crate::ui::FeSoundRequest> {
        use crate::ui::FeSoundRequest;
        let mut sounds: Vec<_> =
            self.pause_menu.take_sounds().into_iter().map(FeSoundRequest::Play).collect();
        sounds.extend(self.take_career_sounds().into_iter().map(FeSoundRequest::Play));
        if let Some(menu) = self.main_menu.as_mut() {
            sounds.extend(menu.take_sounds().into_iter().map(FeSoundRequest::Play));
        }
        sounds.extend(self.options_menu.take_sounds().into_iter().map(FeSoundRequest::Play));
        sounds.extend(self.skate_profile.take_sounds().into_iter().map(FeSoundRequest::Play));
        sounds.extend(self.audio.menu.take_sounds().into_iter().map(FeSoundRequest::Play));
        sounds.append(&mut self.editor_sounds);
        if let Some(editor) = self.editor.as_mut() {
            sounds.extend(editor.frontend.take_sounds());
        }
        if let Some(editor)=self.replay_editor.as_mut() {sounds.extend(editor.controller.take_sounds());}
        if let Some(guide)=self.trick_guide.as_mut() {sounds.extend(guide.controller.take_sounds().into_iter().map(FeSoundRequest::Play));}
        sounds.extend(self.take_online_hub_sounds());
        sounds
    }
    /// Consumes TricksBackEnd.UpdateJackedOverlay8256db90's frontend request.
    pub fn request_jacked_overlay(&mut self)->Result<(),SelectedLiveError>{
        if self.inner.assets.as_ref().expect("ready assets").cvars.try_show_jacked_overlay()
            .map_err(|e|SelectedLiveError::failed(SelectedLiveStage::PreupdateInput,e))? {
            self.pause_menu.request_jacked();
        }
        Ok(())
    }
    #[must_use]
    pub fn map_menu_snapshot(&self) -> crate::ui::map_menu::MapMenuSnapshot {
        let mut snapshot = self.map_menu.snapshot();
        snapshot.career = self
            .career
            .as_ref()
            .and_then(career::CareerSession::challenge_map_view)
            .map(Arc::new);
        snapshot
    }
    #[must_use]
    pub fn map_locations(&self) -> &[crate::ui::map_menu::MapLocation] {
        self.inner.assets.as_ref().map_or(&[], |assets| assets.map_catalog.locations.as_slice())
    }
    /// MapIconManager826187b8 uses Actor position and Entity::GetHeading,
    /// whose Actor transform is GetEffectiveSkaterTransform825ed4c0.
    #[must_use]
    pub fn map_player_pose(&self) -> Option<([f32; 2], [f32; 2])> {
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let output = physical.owner().phys_out().lock().ok()?;
        let skeleton = output.try_skeleton_output().ok()?;
        let position = skeleton.skeleton_root_matrix.columns()[3].xyz;
        let heading = skeleton.effective_skeleton_root_matrix.columns()[2].xyz;
        Some(([position[0], position[2]], [heading[0], heading[2]]))
    }
    #[must_use]
    pub fn map_travel_position(&self) -> Option<[f32; 3]> {
        self.map_travel.filter(|_| !self.map_travel_dispatched && self.frontend_fade.covered())
            .map(|m| [m[12], m[13], m[14]])
    }
    pub fn acknowledge_map_travel_presentation_ready(&mut self) {
        if self.map_travel.is_some() { self.map_travel_presentation_ready = true; }
    }
    /// The renderer's presentation stream after its last streaming update.
    pub fn set_presentation_stream_area(&mut self, area: crate::streaming::area_loaded::PresentationStreamArea) {
        self.presentation_area = Some(area);
    }
    /// The mission hulls' presentation collections the hull loaders registered.
    pub fn hull_presentations(&self) -> Vec<crate::presentation::HullPresentation> {
        self.inner.terrain.as_ref().map(|terrain| terrain.hull_presentations()).unwrap_or_default()
    }
    pub fn map_travel_stream_policy(&self) -> Result<(bool, bool), SelectedLiveError> {
        let cvars = &self.inner.assets.as_ref().expect("ready assets").cvars;
        Ok((cvars.try_disable_stream_focus_movement().map_err(|e| SelectedLiveError::failed(SelectedLiveStage::TerrainResident, e))?,
            cvars.try_enable_stream_xml().map_err(|e| SelectedLiveError::failed(SelectedLiveStage::TerrainResident, e))?))
    }

    /// `XBox2ExecutionMan::TimerHz`: the rate the host timer wakes the game
    /// loop. Slow motion lowers it; every tick still simulates `kUpdateTime`.
    /// `None` before the World exists.
    #[must_use]
    pub fn sim_timer_hz(&self) -> Option<i32> {
        let world = self.inner.world_handle()?.lock().ok()?;
        let hz = world.sim_rate.lock().ok()?.timer_hz();
        Some(hz)
    }

    /// Read the current generated Action and Motion states and raw PhysOut
    /// physics-state ID from the Actor that completed the latest frame.
    /// This is diagnostic observation only; an incomplete or unavailable
    /// owner yields `None` without changing the retained graphs or output.
    #[must_use]
    pub fn diagnostic_state_ids(&self) -> Option<(u16, u16, i32)> {
        if !self.last_step_completed {
            return None;
        }
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let (action_controller, motion_controller) = {
            let prefix = actor.prefix().ok()?;
            let graphs = prefix.graphs()?;
            let action = graphs.action()?.controller.as_ref()?.generated_action()?;
            let motion = graphs.motion()?.controller.as_ref()?.generated_motion()?;
            (action, motion)
        };
        let action = action_controller.lock().ok()?.current_state;
        let motion = motion_controller.lock().ok()?.current_state;
        let physical = actor.cached_physical_allocation().ok()?;
        let output = physical.owner().phys_out().lock().ok()?;
        let physics = output.try_state_output().ok()?.physics_state;
        Some((action, motion, physics))
    }

    /// Read the completed frame's retained PhysOut board position, motion Z
    /// speed, and deck speed. This observes published output only; it does not
    /// estimate motion from the input sample or advance any simulation owner.
    #[must_use]
    pub fn diagnostic_board_kinematics(&self) -> Option<([f32; 3], f32, f32)> {
        if !self.last_step_completed {
            return None;
        }
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let output = physical.owner().phys_out().lock().ok()?;
        Some((
            output.try_camera_board_output().ok()?.deck_position.xyz,
            output.try_motion_z_speed().ok()?,
            output.try_motion_deck_speed().ok()?,
        ))
    }

    /// Published manual flag used by the camera subject (excluding kickturns).
    /// Diagnostic only, with no state mutation or inferred input state.
    #[must_use]
    pub fn diagnostic_manual_flags(&self) -> Option<(bool, bool)> {
        if !self.last_step_completed {
            return None;
        }
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let output = physical.owner().phys_out().lock().ok()?;
        Some((
            output.try_state_output().ok()?.is_manual,
            output.try_camera_intents_output().ok()?.is_kickturning,
        ))
    }

    /// Published offboard output: jumping, trajectory ready, holding board.
    /// Observe only; do not infer success from the scripted button sample.
    #[must_use]
    pub fn diagnostic_offboard_flags(&self) -> Option<(bool, bool, bool)> {
        if !self.last_step_completed {
            return None;
        }
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let output = physical.owner().phys_out().lock().ok()?;
        let offboard = output.try_offboard_output().ok()?;
        Some((
            offboard.is_jumping,
            offboard.trajectory_ready,
            offboard.is_holding_skateboard,
        ))
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's skaters from the latest completed frame.
    /// Observation only, like the diagnostics.
    #[must_use]
    pub fn audio_skaters(&self) -> Option<Vec<AudioSkater>> {
        if !self.last_step_completed {
            return None;
        }
        self.audio_roster()
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's skaters: the SkaterManager's Actors in its
    /// order, each with the `PhysOut` values `FillSkaterData` reads and whether its player ID is
    /// `GetLocalSimIndex`, then the online game's remote skaters. A remote skater the console
    /// follows is the local one (`Online.SwitchCamera`), and the console's own is not.
    fn audio_roster(&self) -> Option<Vec<AudioSkater>> {
        let local = self.inner.world.as_ref()?.world.lock().ok()?.local_skater_identity.clone()?;
        let local_index = local.try_local_sim_index().ok()?;
        let following = self.online_remote_is_audio_local();
        let local_position = self.audio_local_actor().and_then(|actor| actor.try_ai_position().ok());
        let mut skaters = Vec::new();
        for actor in local.try_scoring_roster().ok()? {
            let Some(player_index) = actor.prefix().ok().and_then(|prefix| prefix.fields().map(|fields| fields.player_index)) else {
                continue;
            };
            let player = i32::from_ne_bytes(player_index.to_ne_bytes());
            let is_local = player == local_index;
            let can_see_player = !is_local
                && local_position
                    .zip(actor.try_ai_position().ok())
                    .is_some_and(|(local, own)| crate::ai::actor::can_see_skater(own, local));
            let Ok(physical) = actor.cached_physical_allocation() else { continue };
            let Ok(output) = physical.owner().phys_out().lock() else { continue };
            let Ok(phys_out) = output.try_audio_phys_out() else { continue };
            let ai_audio = self.inner.ai.as_ref().and_then(|ai| ai.controller_audio(player));
            skaters.push(AudioSkater { player_index, local: is_local && !following, phys_out, can_see_player, ai_audio });
        }
        skaters.extend(self.online_audio_skaters());
        Some(skaters)
    }

    /// 1.02 `PlayerUI::Update` (8232f7a4..f82c): the frames `sub_826F7668` turns the left
    /// stick with while the skater `GetLocalActor` finds (with no ghost, `FindByIndex` of the
    /// PlayerUI's player) is off its board (`PhysOut_State::mIsOffboard`), not holding an
    /// object (`PhysOut_OffBoard::mIsGrabbingObject`) and not landing on its deck. The camera
    /// is `CameraPresentation::GetMatrix(true)` as the game shows it: an online game's view,
    /// else the game camera.
    fn offboard_stick_frames(
        &self,
    ) -> Option<crate::input_pad::device_manager::selected_runtime::player_sample::OffBoardStickFrames> {
        use crate::skeleton_root_transforms::SkeletonRootVector;
        let player = i32::from_ne_bytes(self.inner.bound_input.as_ref()?.player_index().to_ne_bytes());
        let local = self.inner.world.as_ref()?.world.lock().ok()?.local_skater_identity.clone()?;
        if !local.try_ghost_is_null().ok()? {
            return None;
        }
        let actor = local.try_find_by_index(player).ok()??;
        let physical = actor.cached_physical_allocation().ok()?;
        let skater = {
            let output = physical.owner().phys_out().lock().ok()?;
            let state = output.try_state_output().ok()?;
            if !state.is_offboard
                || output.try_offboard_output().ok()?.is_grabbing_object
                || state.physics_state == crate::physics_state_ids::PHYSICS_STATE_LANDINGONDECK
            {
                return None;
            }
            let rows = output.try_skeleton_output().ok()?.effective_skeleton_root_matrix.columns();
            [rows[0], rows[1], rows[2]]
        };
        let camera = self
            .online_view_camera()
            .and_then(|camera| camera.matrix)
            .or_else(|| self.game_camera_matrix())?;
        let row = |row: usize| SkeletonRootVector::known(std::array::from_fn(|lane| camera[row * 4 + lane]));
        Some(crate::input_pad::device_manager::selected_runtime::player_sample::OffBoardStickFrames {
            camera: [row(0), row(1), row(2)],
            skater,
        })
    }

    /// The game camera's matrix as Skate rows (the producer projection `try_visual_snapshot`
    /// uses), which `CameraPresentation::GetCurrentData` holds until a scene camera shows.
    pub(super) fn game_camera_matrix(&self) -> Option<[f32; 16]> {
        let camera = self.inner.world.as_ref()?.local.try_camera_producer_projection().ok()?;
        let c = camera.camera_matrix.columns();
        let [x, y, z, t] = [c[0].xyz, c[1].xyz, c[2].xyz, c[3].xyz];
        Some([x[0], x[1], x[2], 0.0, y[0], y[1], y[2], 0.0, z[0], z[1], z[2], 0.0, t[0], t[1], t[2], 1.0])
    }

    /// `CameraPresentation::GetMatrix(1)` for the audio microphones: the completed frame count,
    /// then the game camera matrix's Z axis and translation (the producer projection
    /// `try_visual_snapshot` uses). Reads only.
    #[must_use]
    pub fn audio_camera(&self) -> Option<(u32, [f32; 3], [f32; 3])> {
        if self.completed_frames == 0 {
            return None;
        }
        if let Some(matrix) = self.online_view_camera().and_then(|camera| camera.matrix) {
            return Some((self.completed_frames, [matrix[8], matrix[9], matrix[10]], [matrix[12], matrix[13], matrix[14]]));
        }
        let camera = self.inner.world.as_ref()?.local.try_camera_producer_projection().ok()?;
        let columns = camera.camera_matrix.columns();
        Some((self.completed_frames, columns[2].xyz, columns[3].xyz))
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's `GlobalData.reverbKey`: `LayerMan::GetAttribData`
    /// of layer 10 at the local skater's `PhysOut_Skeleton::mSkeletonHipsPos`, permuted `<0,6,0,0>`
    /// into (x, z), or at (0, 0) without a local skater. Reads only; `None` without a world.
    #[must_use]
    pub fn audio_reverb_key(&self) -> Option<u64> {
        self.audio_layer_key(10, self.audio_layer_point())
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's `GlobalData.ambienceKey`: `LayerMan::GetAttribData`
    /// of layer 7 (`eAudio_Ambience`) at the point [`Self::audio_reverb_key`] reads. Its region names
    /// hash to the `aud_wp_ambiences` collections. Reads only; `None` without a world.
    #[must_use]
    pub fn audio_ambience_key(&self) -> Option<u64> {
        self.audio_layer_key(7, self.audio_layer_point())
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's `GlobalData.emitterKey`: `LayerMan::GetAttribData`
    /// of layer 8 (`eAudio_Emitters`) at the point [`Self::audio_reverb_key`] reads: the location's
    /// `aud_wp_emitters` collection of random emitters. Reads only; `None` without a world.
    #[must_use]
    pub fn audio_emitter_key(&self) -> Option<u64> {
        self.audio_layer_key(8, self.audio_layer_point())
    }

    /// The local skater's `PhysOut_Skeleton::mSkeletonHipsPos` as the (x, z) `DoSimSubmit` reads
    /// the WorldPainter audio layers at; (0, 0), its zeroed position, without one.
    fn audio_layer_point(&self) -> [f32; 2] {
        if let Some(hips) = self.online_audio_local_hips() {
            return [hips[0], hips[2]];
        }
        if let Some(actor) = self.audio_local_actor()
            && let Ok(physical) = actor.cached_physical_allocation()
            && let Ok(output) = physical.owner().phys_out().lock()
            && let Ok(skeleton) = output.try_skeleton_output()
        {
            let hips = skeleton.skeleton_hips_pos.xyz;
            return [hips[0], hips[2]];
        }
        [0.0; 2]
    }

    /// `FillSkaterData`'s `PhysOut` values of the console's own skater.
    fn audio_own_record(&self) -> Option<crate::SkaterAudioPhysOut> {
        let actor = self.audio_local_actor()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let output = physical.owner().phys_out().lock().ok()?;
        output.try_audio_phys_out().ok()
    }

    /// `GlobalData`'s network-game bit (`AudioSystem::mbIsOnlineGame`): `sGameInfo.isNetworkGame`.
    #[must_use]
    pub fn audio_online_game(&self) -> bool {
        let Some(owner) = self.inner.world.as_ref() else { return false };
        let Ok(world) = owner.world.lock() else { return false };
        world.local_skater_identity.as_ref().is_some_and(|identity| identity.backend().try_is_network_game().unwrap_or(false))
    }

    /// The SkaterManager's Actor whose player ID is `GetLocalSimIndex`.
    fn audio_local_actor(&self) -> Option<std::sync::Arc<crate::actor_construction::ActorConstruction>> {
        let local = self.inner.world.as_ref()?.world.lock().ok()?.local_skater_identity.clone()?;
        let index = local.try_local_sim_index().ok()?;
        local.try_find_by_index(index).ok()?
    }

    /// `WorldPainter::LayerMan::GetAttribData` of an audio layer at an (x, z); `None` without a world.
    fn audio_layer_key(&self, layer: usize, point: [f32; 2]) -> Option<u64> {
        let world = self.inner.world.as_ref()?;
        let terrain = self.inner.terrain.as_ref()?.terrain_for(&world.world).ok()?;
        let layers = terrain.layers.lock().ok()?;
        layers.try_attrib_key(layer, point).ok()
    }

    /// What `AudioPresentation::DoSimSubmit` 0x827fb2f8 submits in the front end, whose simulation
    /// (`GameModule::Update` 0x8273edc8 runs it while `eFE_STATE_IN_FE`) has the first player and
    /// the ambient AI skaters but no cars, pedestrians or dynamic objects: the main menu's ICE
    /// camera as `CameraPresentation::GetMatrix(1)` (its Z axis and translation), and the
    /// WorldPainter audio layers at the local skater's hips, at (0, 0) until the front end's world
    /// runs. `None` outside the front end.
    #[must_use]
    pub fn audio_front_end(&self) -> Option<crate::AudioFrontEnd> {
        if self.main_menu.is_none() {
            return None;
        }
        let camera = self.main_menu_frame().ok()??.camera;
        let point = if self.frontend_world_running() { self.audio_layer_point() } else { [0.0; 2] };
        let key = |layer| self.audio_layer_key(layer, point).unwrap_or(0);
        Some(crate::AudioFrontEnd {
            frame: self.completed_frames,
            camera_at: [camera[8], camera[9], camera[10]],
            camera_position: [camera[12], camera[13], camera[14]],
            reverb_key: key(10),
            ambience_key: key(7),
            emitter_key: key(8),
        })
    }

    /// The amount `VDWriter::ReceiveOutOfBoundaryAmountMsg` 0x827fc718 kept for the last completed
    /// frame; `WriteData` 0x827ff8c8 records it only when not negative, then resets it to -1.
    #[must_use]
    pub fn out_of_boundary_amount(&self) -> Option<f32> {
        self.out_of_boundary_amount
    }

    /// The DMO `AudioPresentation::DoSimSubmit` 0x827fb2f8 marks `DynamicData.grabbedObject`: while
    /// the local skater's `PhysOut_OffBoard::mIsGrabbingObject` holds,
    /// `LWGrabbableObjectMan::GetGrabSplineOwnerObjectID` of `PhysOut_State::mGrabSplineDesc.mObjectID`,
    /// compared against each entry's `objectID`.
    #[must_use]
    pub fn audio_grabbed_dynamic_object(&self) -> Option<u32> {
        let actor = self.inner.player.as_ref()?.selected.actor.as_ref()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let spline = {
            let output = physical.owner().phys_out().lock().ok()?;
            if !output.try_offboard_output().ok()?.is_grabbing_object {
                return None;
            }
            output.try_state_output().ok()?.grab_spline_desc.object_id
        };
        let world = self.inner.world.as_ref()?.world.lock().ok()?;
        let living_world = world.grind_services.living_world().lock().ok()?;
        Some(living_world.dmo_objects.as_deref()?.grab_spline_owner(spline.cast_unsigned()))
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's pedestrians: `GetPedestriansWithinRange`
    /// 0x82e9ab88 within 50 m of the game camera's (x, z), at most 30 in the KD-tree's order,
    /// skipping those without an initialized core (`LWCommonCoreDataProxy::IsInitialized`).
    /// `GetAudioCharacter` reads `livingworld_models` layout+4 (`DefaultDataArea`'s 0 without
    /// one). Reads only; empty before a completed frame or without the pedestrian manager.
    #[must_use]
    pub fn audio_pedestrians(&self) -> Vec<AudioPedestrian> {
        let mut pedestrians = Vec::new();
        if self.completed_frames == 0 {
            return pedestrians;
        }
        let Some(world) = self.inner.world.as_ref() else { return pedestrians };
        let Ok(camera) = world.local.try_camera_producer_projection() else { return pedestrians };
        let position = camera.camera_matrix.columns()[3].xyz;
        let Ok(physical) = world.world.lock() else { return pedestrians };
        let Ok(living) = physical.grind_services.living_world().lock() else { return pedestrians };
        let (mut count, mut ids, mut distances) = (0, [0; 30], [0.0; 30]);
        if living.try_get_pedestrians_within_range([position[0], position[2]], 50.0, 30, &mut count, &mut ids, &mut distances).is_err() {
            return pedestrians;
        }
        let Some(roster) = living.pedestrian_entities.as_deref() else { return pedestrians };
        for (&id, &distance) in ids.iter().zip(&distances).take(count as usize) {
            let Some(entity) = roster.get(id) else { continue };
            let Some(core) = entity.core.as_ref() else { continue };
            let audio_character = entity
                .resources
                .as_ref()
                .and_then(|resources| resources.model.try_cached_layout_field(4, 4).ok().flatten())
                .and_then(|data| <[u8; 4]>::try_from(data.bytes()).ok())
                .map_or(0, u32::from_be_bytes);
            let presented = entity.animation.as_ref().is_some_and(|animation| animation.presentation.is_some());
            let out = &entity.state_out;
            pedestrians.push(AudioPedestrian {
                id: entity.id,
                state: out.state,
                length: out.length,
                topic: out.topic,
                target_greet_character: out.target_greet_character,
                flags: out.flags,
                audio_character,
                distance,
                local_to_world: presented.then_some([core.transform[2], core.transform[3]]),
            });
        }
        pedestrians
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's `VehicleData`: `GetVehiclesWithinRange`
    /// 0x82d29030 at the camera's (x, z), 40 m, at most ten cars, in the query's order (`GetVehicle`
    /// skips a missing id), with each car's `cLivingWorldPresEntityManager::GetPresVehicle` matrix.
    /// Reads only; empty without a world.
    #[must_use]
    pub fn audio_vehicles(&self) -> Vec<AudioVehicle> {
        let Some(owner) = self.inner.world.as_ref() else { return Vec::new() };
        let Ok(camera) = owner.local.try_camera_producer_projection() else { return Vec::new() };
        let position = camera.camera_matrix.columns()[3].xyz;
        let Ok(world) = owner.world.lock() else { return Vec::new() };
        let recipes = self.inner.player.as_ref().and_then(|player| player.selected.owners.living_world_recipes.lock().ok());
        let Ok(living) = world.grind_services.living_world().lock() else { return Vec::new() };
        let (mut count, mut ids, mut distances) = (0, [0; 10], [0.0; 10]);
        if living.try_get_vehicles_within_range([position[0], position[2]], 40.0, 10, &mut count, &mut ids, &mut distances).is_err() {
            return Vec::new();
        }
        let Some(vehicles) = living.grind_vehicle_entities.as_deref() else { return Vec::new() };
        ids.iter()
            .zip(distances)
            .take(count as usize)
            .filter_map(|(&id, distance)| {
                let vehicle = vehicles.get(id)?;
                let engine_key = vehicle
                    .characteristics
                    .try_cached_layout_field(8, 8)
                    .ok()
                    .flatten()
                    .map_or(0, |data| u64::from_be_bytes(data.bytes().try_into().expect("eight-byte key")));
                let presented = recipes.as_ref().is_some_and(|recipes| recipes.presentation.get(vehicle.core.recipe_id).is_some());
                let mut matrix = vehicle.core.transform;
                matrix[3][3] = 1.0;
                Some(AudioVehicle {
                    id: vehicle.id,
                    acceleration: vehicle.motion.acceleration,
                    speed: vehicle.motion.speed,
                    distance,
                    horn_state: vehicle.signals.horn as i32,
                    skidding: vehicle.motion.skidding,
                    engine_key,
                    matrix: presented.then_some(matrix),
                })
            })
            .collect()
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's `DynamicData`: `GetDynamicObjectsWithinRange`
    /// 0x82d3be78 at the local skater's `PhysOut_SystemReckoning::mCentreOfMass` (x, z), or the
    /// camera's without one, 30 m, at most 20, in the query's order (`GetDynamicObject` skips a
    /// missing id). A later Part 0 contact replaces the strongest unless its force is at most the
    /// strongest's (`fcmpu`/`ble`, so a NaN replaces). The bounds centre is `SetLocalToWorldMatrix`
    /// 0x827efd08's `m_globalBC` with the core `mTransform`: the RModel box centre, whose W
    /// `CalculateBoundsInfo` 0x827f2708 sets to 1. Reads only; empty without a world.
    #[must_use]
    pub fn audio_dynamic_objects(&self) -> Vec<AudioDynamicObject> {
        use crate::{
            physical_player_phys_out::PhysOutSystemVector,
            reckoning_numeric::{ReckoningVector, host_dot3},
        };
        let Some(owner) = self.inner.world.as_ref() else { return Vec::new() };
        let Ok(camera) = owner.local.try_camera_producer_projection() else { return Vec::new() };
        let mut centre = camera.camera_matrix.columns()[3].xyz;
        let mut grab_spline = None;
        if let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref())
            && let Ok(physical) = actor.cached_physical_allocation()
            && let Ok(output) = physical.owner().phys_out().lock()
        {
            if let Ok(centre_of_mass) = output.try_camera_system_vector(PhysOutSystemVector::CentreOfMass) {
                let [x, y, z, _] = centre_of_mass.operand();
                centre = [x, y, z];
            }
            if output.try_offboard_output().is_ok_and(|offboard| offboard.is_grabbing_object)
                && let Ok(state) = output.try_state_output()
            {
                grab_spline = Some(state.grab_spline_desc.object_id.cast_unsigned());
            }
        }
        let Ok(world) = owner.world.lock() else { return Vec::new() };
        let Ok(living) = world.grind_services.living_world().lock() else { return Vec::new() };
        let Some(objects) = living.dmo_objects.as_deref() else { return Vec::new() };
        let grabbed = grab_spline.map(|spline| objects.grab_spline_owner(spline));
        let Ok(found) = objects.try_objects_within_range([centre[0], centre[2]], 30.0, 20) else {
            return Vec::new();
        };
        found
            .into_iter()
            .filter_map(|(id, _)| {
                let object = objects.object(id)?;
                // AudioPresentation::DoSimSubmit 0x827fb2f8 hears the living world's objects only.
                if object.online_static {
                    return None;
                }
                let (_, angular_velocity) = world.try_dmo_collision_part_velocities(&object.body, 0).ok()?;
                let contact = object.part0_contacts.iter().reduce(|best, candidate| {
                    if candidate.collision_force <= best.collision_force { best } else { candidate }
                });
                let t = &object.transform;
                let up = host_dot3([t[4], t[5], t[6], t[7]], [0.0, 1.0, 0.0, 0.0]).abs();
                let (minimum, maximum) = (object.model.bbox_min, object.model.bbox_max);
                let box_centre: [f32; 3] = std::array::from_fn(|lane| (maximum[lane] + minimum[lane]) * 0.5);
                let bounds_center = std::array::from_fn(|lane| {
                    let x = t[lane] * box_centre[0];
                    let y = t[4 + lane].mul_add(box_centre[1], x);
                    let z = t[8 + lane].mul_add(box_centre[2], y);
                    t[12 + lane].mul_add(1.0, z)
                });
                Some(AudioDynamicObject {
                    id: object.entity_id,
                    velocity: object.velocity,
                    grabbed: grabbed == Some(object.entity_id),
                    characteristic_key: object.model.characteristic_collection,
                    angular_velocity,
                    contact: contact.map(|contact| (contact.collision_force, contact.audio_surface)),
                    on_side: !(up >= 0.3),
                    bounds_center,
                })
            })
            .collect()
    }

    /// Borrow only the palette/materials published by this session's last
    /// completed selected frame; couple them with its retained camera observer.
    pub fn try_visual_snapshot(&self) -> Result<SelectedVisualSnapshot<'_>, SelectedLiveError> {
        // Subscribe only when a frontend exists. Headless selected sessions
        // still score normally without accumulating unconsumed HUD packets.
        if let Some(tick) = self.inner.selected_tick.as_ref() {
            tick.scoring_hud_subscribed.set(true);
        }
        if !self.last_step_completed {
            return Err(SelectedLiveError::Pending(
                SelectedLivePending::NoCompletedFrame,
            ));
        }
        // An online game's scene camera, or the camera of the player Online.SwitchCamera follows.
        self.visual_snapshot(self.online_view_camera(), None)
    }

    /// A frame while a NIS holds the simulation paused: the world as the
    /// last completed update left it, seen through the ICE camera while
    /// `mICECameraActive`, else the game camera; with the stream focus a
    /// scene froze. A replay's LivingWorld entities are the shown frame's own:
    /// `UpdateLivingWorldPresentationEntities` (0x828133e0) keeps only the
    /// pedestrians, vehicles and dynamic objects the frame records, and
    /// `LivingWorldPresentation::DoSimRestore` (0x82813a90) runs the frame's
    /// recorded animation commands for the pedestrians' poses.
    pub fn try_nis_visual_snapshot(&self) -> Result<SelectedVisualSnapshot<'_>, SelectedLiveError> {
        if self.completed_frames == 0 {
            return Err(SelectedLiveError::Pending(
                SelectedLivePending::NoCompletedFrame,
            ));
        }
        let (camera, focus) = self.career_nis_camera();
        let mut visual = self.visual_snapshot(camera, focus)?;
        if let Some(frame) = self.replay_clock.and_then(|clock| self.replay_frames.get(&clock.current)) {
            visual.dynamic_objects = frame.world.dynamic_objects.clone();
            visual.vehicles = frame.world.vehicles.clone();
            visual.pedestrians = frame.world.pedestrians.clone();
            for pedestrian in &visual.pedestrians {
                pedestrian
                    .restore_replay_pose()
                    .map_err(|e| SelectedLiveError::failed(SelectedLiveStage::Visual, e))?;
            }
        }
        Ok(visual)
    }

    /// The crowd presentation's targets and imposters (`CrowdPresentation::RenderCrowdTargets`,
    /// `RenderCrowdImposters`).
    fn crowd_visual(&self) -> Result<crate::SelectedCrowdVisual, SelectedLiveError> {
        use SelectedLiveStage as Stage;
        let assets = self.inner.assets.as_ref().ok_or(SelectedLiveError::Pending(SelectedLivePending::FrameNotReady))?;
        let player = self.inner.player.as_ref().ok_or(SelectedLiveError::Pending(SelectedLivePending::FrameNotReady))?;
        let Some(actor) = player.selected.actor.as_ref() else {
            return Ok(crate::SelectedCrowdVisual::default());
        };
        let recipes = player.selected.owners.living_world_recipes.lock().map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
        recipes.crowd_visual(&assets.vault, &assets.cvars, actor).map_err(|e| SelectedLiveError::failed(Stage::Visual, e))
    }

    pub(super) fn recorded_world_visuals(&self)->Result<replay_session::RecordedWorld,SelectedLiveError> {
        use SelectedLiveStage as Stage;
        let cvars=&self.inner.assets.as_ref().ok_or_else(||SelectedLiveError::failed(Stage::Visual,"assets absent"))?.cvars;
            let world = self
                .inner
                .world
                .as_ref()
                .ok_or(SelectedLiveError::Pending(
                    SelectedLivePending::FrameNotReady,
                ))?
                .world
                .lock()
                .map_err(|_| SelectedLiveError::failed(Stage::Visual, "world lock poisoned"))?;
            let actor = self
                .inner
                .player
                .as_ref()
                .and_then(|player| player.selected.actor.as_ref())
                .ok_or_else(|| {
                    SelectedLiveError::failed(Stage::Visual, "DMO presentation Actor absent")
                })?;
            let assets = self
                .inner
                .assets
                .as_ref()
                .ok_or(SelectedLiveError::Pending(
                    SelectedLivePending::FrameNotReady,
                ))?;
            let dmos = crate::selected_living_world_dmo::dynamic_object_visuals(
                &world,
                actor,
                &assets.vault,
                cvars,
            )
            .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
            let recipes = &self
                .inner
                .player
                .as_ref()
                .expect("checked player")
                .selected
                .owners
                .living_world_recipes;
            let recipe_guard=recipes.lock().map_err(|e|SelectedLiveError::failed(Stage::Visual,e))?;
            let recipes=&recipe_guard.presentation;
            let vehicles = crate::selected_living_world_vehicle_visual::vehicle_visuals(
                &world,
                actor,
                &assets.vault,
                cvars,
                recipes,
            )
            .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
            let traffic_light_status = {
                let living = world
                    .grind_services
                    .living_world()
                    .lock()
                    .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
                let rain = living.rain.as_ref().ok_or_else(|| {
                    SelectedLiveError::failed(Stage::Visual, "world presentation RAIN owner absent")
                })?;
                rain.presentation_traffic_lights()
                    .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?
            };
            let pedestrians=crate::selected_living_world_recipes::pedestrian_animation::presentation::pedestrian_visuals(&world,actor,&assets.vault,cvars)
                .map_err(|e|SelectedLiveError::failed(Stage::Visual,e))?;
            Ok(replay_session::RecordedWorld { dynamic_objects:dmos, vehicles, pedestrians, traffic_lights:traffic_light_status })
    }

    fn visual_snapshot(&self, nis: Option<NisCamera>, focus: Option<FrozenFocus>) -> Result<SelectedVisualSnapshot<'_>, SelectedLiveError> {
        use SelectedLiveStage as Stage;
        let runtime =
            self.inner
                .presentation_runtime
                .as_ref()
                .ok_or(SelectedLiveError::Pending(
                    SelectedLivePending::FrameNotReady,
                ))?;
        let palette = runtime
            .palette()
            .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
        let materials = Arc::clone(
            runtime
                .materials()
                .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?,
        );
        let local = &self
            .inner
            .world
            .as_ref()
            .ok_or(SelectedLiveError::Pending(
                SelectedLivePending::FrameNotReady,
            ))?
            .local;
        let camera = match nis {
            Some(ice) => local.try_ice_camera_projection(ice.matrix, ice.fov, ice.near_plane),
            None => local.try_camera_producer_projection(),
        }
            .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
        let frozen_focus = focus.map(|focus| match focus {
            FrozenFocus::At(position) => position,
            FrozenFocus::GameCamera => camera.camera_matrix.columns()[3].xyz,
        });
        let cvars = &self
            .inner
            .assets
            .as_ref()
            .ok_or(SelectedLiveError::Pending(
                SelectedLivePending::FrameNotReady,
            ))?
            .cvars;
        let borrowed = self.career.as_ref().is_some_and(career::CareerSession::first_player_borrowed);
        // SkaterPresEntity::GetPosition (0x827d79c0) asks the entity's
        // callbacks: the scene's while it is borrowed.
        let skater_hips_position = if borrowed { runtime.borrowed_hips_position() } else { runtime.published_hips_position() }
            .ok_or_else(|| SelectedLiveError::failed(Stage::Visual, "published skater hips absent"))?;
        let skater_world_bounds = runtime.published_bounds()
            .ok_or_else(|| SelectedLiveError::failed(Stage::Visual, "published skater bounds absent"))?;
        let local_skater_shadow = match (self.online_spectated_shadow(), self.inner.local_presented_skater().map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?) {
            (Some(spectated), _) => spectated,
            (None, Some(skater)) => (
                skater.presentation_runtime.published_hips_position()
                    .ok_or_else(|| SelectedLiveError::failed(Stage::Visual, "local party skater hips absent"))?,
                skater.presentation_runtime.published_bounds()
                    .ok_or_else(|| SelectedLiveError::failed(Stage::Visual, "local party skater bounds absent"))?,
            ),
            (None, None) => (skater_hips_position, skater_world_bounds),
        };
        let render = runtime.render_opacity().ok_or_else(|| {
            SelectedLiveError::failed(Stage::Visual, "skater render opacity unpublished")
        })?;
        let replay_session::RecordedWorld {dynamic_objects,vehicles,pedestrians,traffic_lights:traffic_light_status} =
            self.recorded_world_visuals()?;
        Ok(SelectedVisualSnapshot {
            hom_visual: self.bail_visual(),
            minimap: self.try_minimap_frame().map_err(|e| SelectedLiveError::failed(Stage::Visual,e))?,
            minimap_icons: &self.inner.assets.as_ref().expect("ready assets").map_catalog.minimap,
            scoring_hud_frames: self.inner.selected_tick.as_ref()
                .map(|tick| std::mem::take(&mut *tick.scoring_hud_frames.borrow_mut()))
                .unwrap_or_default(),
            scoring_hud_tuning: cvars.try_trick_display_tuning()
                .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?,
            out_of_boundary_amount: self.out_of_boundary_amount,
            marker_hud: self
                .inner
                .session_marker
                .as_ref()
                .expect("ready marker")
                .hud,
            simulation_tick: self.completed_frames,
            stream_focus_position: frozen_focus
                .or_else(|| self.map_travel.map(|matrix| [matrix[12], matrix[13], matrix[14]]))
                .or_else(|| self.online_spectated_root())
                .or_else(|| runtime.published_root_position()).ok_or_else(|| {
                SelectedLiveError::failed(Stage::Visual, "published skater root absent")
            })?,
            stream_focus_disabled: cvars
                .try_disable_stream_focus_movement()
                .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?,
            stream_xml_enabled: cvars
                .try_enable_stream_xml()
                .map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?,
            palette,
            materials,
            skater_hips_position,
            skater_world_bounds,
            local_skater_shadow,
            provisional_clip_from_world: camera.clip_from_world,
            camera_world_position: camera.camera_matrix.columns()[3].xyz,
            camera_planes: [camera.near_plane, camera.far_plane],
            camera_forward: camera.camera_matrix.columns()[2].xyz,
            camera_right: camera.camera_matrix.columns()[0].xyz,
            camera_up: camera.camera_matrix.columns()[1].xyz,
            traffic_light_status,
            camera_update_counter: camera.camera_update_counter,
            world_painter: self.try_world_painter_keys(camera.camera_matrix.columns()[3].xyz),
            // ControlCharacterRenderable (0x82870fb0) retypes a borrowed
            // entity, clears mIsGhosted and sets i_params[0].x to 1.
            skater_opacity: if borrowed { 1.0 } else { render.opacity },
            skater_ghosted: render.ghosted && !borrowed,
            skater_entity_type: if borrowed { 0x822 } else { 0x122 },
            dynamic_objects,
            vehicles,
            pedestrians,
            crowd: self.crowd_visual()?,
            chase_arrows: self.try_chase_arrows().map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?,
            scene_characters: self.career.as_ref().map(career::CareerSession::scene_visuals).unwrap_or_default(),
            party_skaters: {
                let mut skaters = self.inner.party_skater_visuals().map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?;
                skaters.extend(self.inner.ai_skater_visuals().map_err(|e| SelectedLiveError::failed(Stage::Visual, e))?);
                skaters.extend(self.online_visuals());
                skaters
            },
            visual_indicators: self.visual_indicator_frame()?,
        })
    }

    /// `VDWriter::_PrepareChallengeVisualIndicators` around the local skater's
    /// `Actor::GetPosition`, with the first follow target's hips
    /// (`SkaterPresentation::GetIndexFromPlayerId`, `GetHipsPos`) for the trail.
    fn visual_indicator_frame(
        &self,
    ) -> Result<Option<crate::challenge::visual_director::VisualIndicatorFrame>, SelectedLiveError>
    {
        let Some(career) = self.career.as_ref() else {
            return Ok(None);
        };
        let Some(actor) = self
            .inner
            .player
            .as_ref()
            .and_then(|player| player.selected.actor.as_deref())
        else {
            return Ok(None);
        };
        let position =
            local_skater_position(self.inner.world.as_ref().map(|world| &world.world), actor)?;
        let mut frame = career.visual_indicators(position);
        frame.follow_target_hips = frame.follow_targets.first().and_then(|&player| {
            self.inner
                .party
                .iter()
                .chain(&self.inner.ai_skaters)
                .find(|skater| i32::try_from(skater.player_index()).is_ok_and(|index| index == player))
                .and_then(|skater| skater.presentation_runtime.published_hips_position())
        });
        Ok(Some(frame))
    }

    /// DoSimRestore render opacity for the completed frame's restored skater,
    /// from the camera producer the renderer draws with this frame.
    fn try_restore_skater_render_opacity(&mut self) -> Result<(), String> {
        let camera = self
            .inner
            .world
            .as_ref()
            .ok_or("selected world absent")?
            .local
            .try_camera_producer_projection()
            .map_err(|e| format!("{e:?}"))?;
        let vault = Arc::clone(
            &self
                .inner
                .player
                .as_ref()
                .ok_or("selected player absent")?
                .selected
                .owners
                .services
                .vault,
        );
        self.inner
            .presentation_runtime
            .as_mut()
            .ok_or("presentation runtime absent")?
            .try_restore_render_opacity(&vault, camera.camera_matrix.columns()[3].xyz)
            .map_err(|e| format!("{e:?}"))?;
        for skater in &mut self.inner.party {
            skater
                .presentation_runtime
                .try_restore_render_opacity(&vault, camera.camera_matrix.columns()[3].xyz)
                .map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }

    /// `_PrepareWorldPainterData` permutes the camera position `<0,6,0,0>`
    /// into (x, z) and queries the simulation's `LayerMan` for six layers.
    fn try_world_painter_keys(&self, camera: [f32; 3]) -> Result<WorldPainterKeys, String> {
        let world = self.inner.world.as_ref().ok_or("selected world absent")?;
        let terrain = self
            .inner
            .terrain
            .as_ref()
            .ok_or("selected terrain absent")?
            .terrain_for(&world.world)
            .map_err(|error| format!("{error:?}"))?;
        let layers = terrain
            .layers
            .lock()
            .map_err(|_| "WorldPainter LayerMan lock poisoned")?;
        let position = [camera[0], camera[2]];
        let key = |layer: usize| {
            layers
                .try_attrib_key(layer, position)
                .map_err(|error| error.to_string())
        };
        Ok(WorldPainterKeys {
            fog: key(15)?,
            colour_cube: key(11)?,
            sky: key(16)?,
            exposure: key(17)?,
            bloom: key(18)?,
            district: key(4)?,
        })
    }
}

/// `NISManager`'s ground queries: `World::LineCollisionTest` over the selected world's
/// admitted ground.
fn nis_ground_line(
    world: Option<&Arc<std::sync::Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>>,
    terrain: Option<&(dyn super::terrain_input::SelectedSessionTerrainSlot + Send)>,
    start: [f32; 4],
    end: [f32; 4],
) -> Result<Option<([f32; 3], [f32; 3])>, String> {
    let world = world.ok_or("NIS ground: world absent")?;
    let ground = terrain
        .ok_or("NIS ground: terrain absent")?
        .ground_pool_for(world)
        .map_err(|e| format!("NIS ground: {e}"))?;
    let mut world = world.lock().map_err(|_| "NIS ground: world lock poisoned")?;
    let line = crate::collision_line_collection::CollisionLineInput {
        start: crate::skeleton_root_transforms::SkeletonRootVector::known(start),
        end: crate::skeleton_root_transforms::SkeletonRootVector::known(end),
        fatness: 0.0,
    };
    let hit = world
        .try_world_line_test(
            ground,
            line,
            "NISManager::GetGroundPositionAndOrientationAtLocation",
            &mut crate::collision_line_numeric::HostCollisionLineNumeric,
        )
        .map_err(|e| format!("NIS ground: {e:?}"))?;
    Ok(hit.map(|hit| (hit.position.xyz, hit.normal.xyz)))
}

/// `Actor::GetPosition` (`0x825f9df0`) reads `PhysOut.mSkeleton+0x1a0`: the
/// translation of `mSkeletonRootMatrix`, not the hips or the deck.
/// `Sim_LocalPlayerStreamerController::UpdateStreamer` (`0x8277d4d0`) reads
/// `SkaterManager::GetLocal`: the skater at `mLocalIndex`, the selected one until Party
/// Play's `SetLocalIndex` moves it.
/// `TriggerMissionMusic` 0x82504f28's countdown `CacheCrowdPodInfos` 0x824fd6c0: the spectator
/// groups around the local skater.
fn crowd_pod_infos(
    world: &Arc<std::sync::Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>,
    selected: &crate::actor_construction::ActorConstruction,
) -> Vec<([f32; 4], i32)> {
    let Ok(position) = local_skater_position(Some(world), selected) else { return Vec::new() };
    let Ok(world) = world.lock() else { return Vec::new() };
    let Ok(living) = world.grind_services.living_world().lock() else { return Vec::new() };
    living.props.as_ref().and_then(|props| props.crowd_pod_infos([position[0], position[2]]).ok()).unwrap_or_default()
}

fn local_skater_position(
    world: Option<&Arc<std::sync::Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>>,
    selected: &crate::actor_construction::ActorConstruction,
) -> Result<[f32; 3], SelectedLiveError> {
    with_local_skater(world, selected, actor_position)
}

/// `Actor::GetBasePosition` (`0x825ed928`) of the same skater, the trigger tester position
/// `InvisibleBoundaryManager::Update` (`0x82da72f0`) measures: midway between the feet while it
/// wipes out (`PhysOut.mState` category 300), else `Actor::GetPosition`.
fn local_skater_base_position(
    world: Option<&Arc<std::sync::Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>>,
    selected: &crate::actor_construction::ActorConstruction,
) -> Result<[f32; 3], SelectedLiveError> {
    with_local_skater(world, selected, actor_base_position)
}

fn with_local_skater<T>(
    world: Option<&Arc<std::sync::Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>>,
    selected: &crate::actor_construction::ActorConstruction,
    read: impl FnOnce(&crate::actor_construction::ActorConstruction) -> Result<T, SelectedLiveError>,
) -> Result<T, SelectedLiveError> {
    use SelectedLiveStage as Stage;
    let identity = match world {
        Some(world) => world
            .lock()
            .map_err(|_| SelectedLiveError::failed(Stage::TerrainResident, "world lock poisoned"))?
            .local_skater_identity
            .clone(),
        None => None,
    };
    let local = match identity {
        Some(identity) => {
            let index = identity.try_local_index().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, format!("{e:?}")))?;
            identity.try_find_by_index(index).map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, format!("{e:?}")))?
        }
        None => None,
    };
    read(local.as_deref().unwrap_or(selected))
}

fn actor_position(actor: &crate::actor_construction::ActorConstruction) -> Result<[f32; 3], SelectedLiveError> {
    use SelectedLiveStage as Stage;
    let physical = actor.cached_physical_allocation().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
    let output = physical
        .owner()
        .phys_out()
        .lock()
        .map_err(|_| SelectedLiveError::failed(Stage::TerrainResident, "PhysOut lock poisoned"))?;
    let skeleton = output.try_skeleton_output().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
    Ok(skeleton.skeleton_root_matrix.columns()[3].xyz)
}

fn actor_base_position(actor: &crate::actor_construction::ActorConstruction) -> Result<[f32; 3], SelectedLiveError> {
    use SelectedLiveStage as Stage;
    let physical = actor.cached_physical_allocation().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
    let output = physical
        .owner()
        .phys_out()
        .lock()
        .map_err(|_| SelectedLiveError::failed(Stage::TerrainResident, "PhysOut lock poisoned"))?;
    let category = output.try_state_output().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?.physics_state_category;
    let skeleton = output.try_skeleton_output().map_err(|e| SelectedLiveError::failed(Stage::TerrainResident, e))?;
    if category != crate::physics_state_ids::PHYSICS_CATEGORY_WIPEOUT {
        return Ok(skeleton.skeleton_root_matrix.columns()[3].xyz);
    }
    let left = skeleton.skeleton_foot_left_pos.xyz;
    let right = skeleton.skeleton_foot_right_pos.xyz;
    Ok(std::array::from_fn(|i| (right[i] - left[i]).mul_add(0.5, left[i])))
}

#[path = "selected_session_career.rs"]
mod career;
pub use career::{CareerHud, EndOptionsView, FrontendArguments, PhotoSelectView, PressADialog};
#[path = "selected_session_scene_characters.rs"]
mod scene_characters;
#[path = "selected_session_edit_skater.rs"]
mod edit_skater;
#[path = "selected_session_freeskate.rs"]
mod freeskate;
#[path = "selected_session_frontend_world.rs"]
mod frontend_world;
#[path = "selected_session_freeskate_viewer.rs"]
mod freeskate_viewer;
#[path = "selected_session_party_play.rs"]
mod party_play_session;
#[path = "selected_session_online.rs"]
mod online_session;
pub use online_session::{MatchRequest, OnlineNotice, OnlinePlayer, SignIn};
#[path = "selected_session_music_player.rs"]
mod music_player_session;
#[path = "selected_session_presentation_submit.rs"]
mod presentation_submit;
#[path = "selected_session_replay.rs"]
mod replay_session;
#[path = "selected_session_replay_editor.rs"]
mod replay_editor_session;
#[path = "selected_session_trick_guide.rs"]
mod trick_guide_session;
#[path = "selected_session_skate_profile.rs"]
mod skate_profile_session;
pub use trick_guide_session::TrickGuideView;
pub use replay_session::{AudioFrame, BailVisual, SecurityPresentation};
#[path = "selected_session_movie.rs"]
mod movie_session;

#[path = "selected_session_minimap.rs"]
mod minimap;

#[path = "selected_session_props.rs"]
mod props;

#[path = "selected_session_extras.rs"]
mod extras_session;
