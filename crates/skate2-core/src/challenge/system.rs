//! `Sk8::Challenge::Sk8ChallengeSystem`: owns the challenge data, the Challenge
//! port's event queue (EventSys queue 7), the progression and challenge Lua
//! states, and the challenge sub-systems.
//!
//! The Lua states and the rest of the system are separate fields, so natives
//! can receive the system data (`World`) while a VM runs; this stands in for
//! the native closures' `lightuserdata` upvalues.

use super::{
    activity::{self, ActivityManager},
    ai::{self, AiRequest, ChallengeAiManager},
    assets::{self, AssetManager},
    attrib_db::AttribDb,
    attrib_lua::{self, AttribHost, Pointers},
    audio_handlers, binder,
    callouts::{self, CalloutManager},
    cas_unlock_parts::CasUnlockPartDb,
    characters::{self, CharacterManager},
    contest, crowd,
    dynamic_hull::{self, DynamicHulls},
    dmo::{self, DmoManager},
    dpo_hud,
    effects::{self, EffectsManager},
    follow_the_leader::{self, FtlManager},
    race::{self, RaceManager},
    indicators::{self, IndicatorManager},
    map_icons::{self, MapIcons, PinchPoints},
    host::{Host, NullHost},
    hud::{CentralMessage, HudManager},
    keys, leaderboard_hud,
    living_world::{self, LivingWorldManager},
    lua::{Lua, LuaError, Value},
    mission::MissionFrame,
    mission_audio::{self, MissionAudio},
    natives, objective_groups,
    objective_hud::{self, ObjectiveHudComponent},
    objective_natives,
    objectives::{self, Closure, ObjectivesManager, PlayerState},
    online,
    players::{self, HighlightGroup, PlayersBackEnd},
    presentation::{self, PresentationDirector},
    replay::ReplaySystem,
    results,
    runtime::{self, PostOffice},
    services::{self, ServiceManager},
    signpost::{self, SignPost},
    signup::{self, SignUpManager},
    signup_hud::{self, SignUpHudComponent},
    signup_poster::{self, PosterAccept, PosterView},
    skate::{self, ChallengeScoring, SkateChallengeManager},
    start_hub,
    teleport::{self, TeleportManager},
    timers::{self, TimerManager},
    tricks::TrickGroupManager,
    turn_hud,
    tutorial_hud::{self, TutorialHudComponent},
    unlock_overlay::{self, UnlockOverlayView},
    unlockables::Unlockables,
    vd_results, voting,
};
use std::{
    collections::{HashSet, VecDeque},
    fmt,
    sync::Arc,
};

/// One field of a message's Lua table (`cMsg*_BuildMessageTable`).
#[derive(Clone, Debug, PartialEq)]
pub enum Field {
    Number(f32),
    Bool(bool),
    Text(String),
    /// `lua_pushlightuserdata` of a native object (a [`Pointers`] handle).
    ///
    /// [`Pointers`]: super::attrib_lua::Pointers
    Pointer(usize),
    /// A 64-bit key; scripts see `(float)(double)` of it.
    Key(u64),
    Nil,
}

/// A Hermes message as challenge scripts see it: the kind
/// (`stringhash32` of the class name) and the table its
/// `cMsg*_HandleMessage_LuaBinding` builds. `fields: None` is a message whose
/// binding has no table builder, which scripts receive as `nil`.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub kind: u32,
    pub fields: Option<Vec<(&'static str, Field)>>,
}

impl Message {
    #[must_use]
    pub fn new(class_name: &str, fields: Vec<(&'static str, Field)>) -> Self {
        Self { kind: keys::hash32(class_name), fields: Some(fields) }
    }
}

/// Challenge-port traffic in queue order: Hermes messages and EventSys events
/// share queue 7 (`EventSys::Flush` in `Simulation::PostUpdate` `0x8289ff24`).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// `cMsgMessageSent` (`0x4586298B`): a progression message key.
    MessageSent { message: u64 },
    /// `cMsgTerminateStateMachine` (`0x86D1A5E1`).
    TerminateStateMachine { machine: u64, autosave: bool },
    /// `cMsgChallengeFailed` (`0xCFCC4109`): `cSignPostManager::LoseChallenge` (`0x825e0f90`)
    /// and `AbandonChallenge` (`0x825e1028`) send the activity key and the `eFinishResult`
    /// (1 lose, 2 abandon).
    ChallengeFailed { activity: u64, result: i32 },
    /// `cMsgNISChallengeNotify`.
    NisChallengeNotify { notification: i32 },
    /// `cMsgHUDComplete`, `cMsgHUDContinue`, `cMsgHUDNo`, `cMsgHUDYes`.
    HudComplete,
    HudContinue,
    HudNo,
    HudYes,
    /// `cMsgSpeechEnded`: the speaking character (`eSk8Characters`) and
    /// speech id.
    SpeechEnded { character: i32, id: i32 },
    /// `cMsgTimerExpired`.
    TimerExpired { timer: i32 },
    /// `cMsgFMVFinishedPlaying`.
    FmvFinishedPlaying,
    /// `cMsgNISComplete` (`NISManager::EndNIS` `0x828a3200`):
    /// `cHighlightReelManager::OnNisCompleteMsg` (`0x82cd7af8`) and the scripts.
    NisComplete,
    /// `cMsgHUDIntroDone` (`0xBDE7CEA2`): `FrontEndState_SignUpPoster::
    /// OnIntroDone` (`0x826581a0`) when the poster's intro ends.
    HudIntroDone,
    /// Any other Hermes message with a Lua binding: only the challenge
    /// post office receives it.
    Challenge(Message),
    /// `cEventChangeState` (`0x828b5a30`): its destructor, run by the flush,
    /// enters the named state of a state graph.
    ChangeState { graph: usize, state: u32 },
    /// `cMsgTriggerEnterCollision`/`cMsgTriggerExitCollision` from
    /// `cTriggerVolumeMessageGroup::Update` (`0x82e24020`), once with the
    /// trigger's global id and once with its local id.
    TriggerCollision { enter: bool, volume: u64, player: i32 },
}

impl Event {
    /// The message the challenge post office routes for this event
    /// (`cMsg*_HandleMessage_LuaBinding` → `cLuaPostOffice::RouteMessage`).
    #[must_use]
    pub fn lua_message(&self) -> Option<Message> {
        #[expect(clippy::cast_precision_loss, reason = "lua_Number")]
        let message = match self {
            Self::MessageSent { message } => Message::new("cMsgMessageSent", vec![("AttribKey", Field::Number(*message as f64 as f32))]),
            Self::TerminateStateMachine { machine, autosave } => Message::new(
                "cMsgTerminateStateMachine",
                vec![("AttribKey", Field::Number(*machine as f64 as f32)), ("InitiateAutoSave", Field::Bool(*autosave))],
            ),
            // cMsgChallengeFailed_BuildMessageTable 0x828c08c8 pushes nil for Result.
            Self::ChallengeFailed { activity, .. } => Message::new(
                "cMsgChallengeFailed",
                vec![("ActivityKey", Field::Number(*activity as f64 as f32)), ("Result", Field::Nil)],
            ),
            Self::NisChallengeNotify { notification } => {
                Message::new("cMsgNISChallengeNotify", vec![("NotificationID", Field::Number(*notification as f32))])
            }
            Self::HudContinue => Message::new("cMsgHUDContinue", Vec::new()),
            Self::TimerExpired { timer } => Message::new("cMsgTimerExpired", vec![("TimerID", Field::Number(*timer as f32))]),
            Self::FmvFinishedPlaying => Message::new("cMsgFMVFinishedPlaying", vec![("FileName", Field::Nil)]),
            // cMsgSpeechEnded_BuildMessageTable 0x828b85e8 pushes nil for Character.
            Self::SpeechEnded { id, .. } => Message::new("cMsgSpeechEnded", vec![("Character", Field::Nil), ("id", Field::Number(*id as f32))]),
            Self::Challenge(message) => message.clone(),
            Self::TriggerCollision { enter, volume, player } => Message::new(
                if *enter { "cMsgTriggerEnterCollision" } else { "cMsgTriggerExitCollision" },
                vec![("VolumeID", Field::Number(*volume as f64 as f32)), ("PlayerID", Field::Number(*player as f32))],
            ),
            // BindAllMessageHandlersToLua (0x828e9..): these share the field-less
            // handler cMsgPresGameUnload_HandleMessage_LuaBinding (0x828cda38), whose
            // table builder (cMsgHUDContinue_BuildMessageTable 0x828ca2c0) is empty.
            Self::HudComplete => Message::new("cMsgHUDComplete", Vec::new()),
            Self::HudNo => Message::new("cMsgHUDNo", Vec::new()),
            Self::HudYes => Message::new("cMsgHUDYes", Vec::new()),
            Self::HudIntroDone => Message::new("cMsgHUDIntroDone", Vec::new()),
            Self::NisComplete => Message::new("cMsgNISComplete", Vec::new()),
            Self::ChangeState { .. } => return None,
        };
        Some(message)
    }
}

/// What `FrontEndState_Skate101` shows: the message's `TitleHALID` and
/// `TextHALID`, as `SetTitleHAL`/`SetDescriptionHAL` (`0x82691ad0`/
/// `0x82691a88`) copy them into 128-byte buffers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Skate101View {
    pub title: String,
    pub text: String,
}

/// Requests from the challenge system to the game and presentation layers,
/// drained by the session after each challenge update or flush.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// `Online.OnlineChallengeComplete` (`0x8289abc0`): `MESSAGE_ChallengeComplete` with the
    /// local player's result (`StateOnlineGame::mMyGameResult`).
    OnlineChallengeComplete { ranking: i32, score: i32, time: f32, players_choice: bool, finish_reason: i32 },
    /// `Online.LeaveGame` (`0x8289aaa0`): `MESSAGE_ChallengeLeaveGame`, the challenge cleaned up.
    OnlineChallengeCleanupDone,
    /// `Online.OnlineChallengeRoundSyncPoint` (`0x8289adb8`): `MESSAGE_ChallengeSyncPoint` for the
    /// round at the named location.
    OnlineRoundSyncPoint { round: i32, location: String },
    /// `FrontEnd.CloseOnlineContinueRetryDialog` (`0x82896c58`): `PopState` of FE state 8.
    CloseOnlineContinueRetryDialog,
    /// `Sk8ChallengeSystem::InitiateAutoSave` (`0x825a66c0`) → `cMsgInitAutosave`.
    AutoSave,
    /// A progression message the port does not present yet.
    Unpresented { message: String, kind: i32 },
    /// `NISManager::RunBasicNISDefinition` (`0x828a8a70`) of a `tNISPlaybackDefinition`: its
    /// scene, `IgnoreTeleportToLastBreadcrumb` and location override (position, direction), the
    /// completion transition, whether the camera holds at the end, whether the NIS can be
    /// skipped and whether the streamer is restored on completion.
    RunNisDefinition {
        scene: String,
        ignore_breadcrumb: bool,
        transition: i32,
        hold_camera: bool,
        skip_enabled: bool,
        location: Option<[[f32; 3]; 2]>,
        restore_streamer: bool,
    },
    /// `NISManager::ResetActorIDs` (`0x828a24a8`).
    ResetNisActorIds,
    /// `CentralHUD::ShowChallengeCompleteMessage` (`0x826bc210`, screen state 6) or
    /// `ShowChallengeFailedMessage` (`0x826bc290`, state 7) of the HAL message.
    ChallengeResultMessage { hal_id: String, failed: bool },
    /// `cMsgChallengeCamera` to the camera presentation (`ReceiveChallengeCameraMsg`
    /// `0x827ef9b8`): while active its matrix is the presented camera.
    ChallengeCamera { active: bool, matrix: [f32; 16] },
    /// `NISManager::PreloadNISList` (`0x828a6998`): the scenes' names.
    PreloadNisList(Vec<String>),
    /// `NISManager::UnloadAnyPreloadedNISs` (`0x828a2e40`).
    UnloadPreloadedNis,
    /// `ChallengeAudioManager::PlayMusic`: `IAudioInterface::TriggerMissionMusic` of the
    /// trigger (`eSk8MissionMusicTrigger`) for the challenge.
    TriggerMissionMusic { trigger: i32, challenge: u64 },
    /// `NISManager::AddActorID` (`0x828a8648`).
    AddNisActorId(i32),
    /// `cTriggerVolumeManager::ResetTriggerVolumeCollisionState(float)` (`0x82e21dd8`): the
    /// first trigger instance whose global or local id, as a float, is the volume's leaves every
    /// tester's previous collisions (`cTriggerVolumeMessageGroup::ResetTriggerVolumeState`
    /// `0x82e22b30`), so a tester still inside enters it again.
    ResetTriggerVolumeState { volume: f32 },
    /// `HandleShowDialogEvent` (message type 18).
    ShowDialog { message: String, title: String, text: String, yes_no: bool },
    /// Message type 10: `ShowHUDComponent`.
    ShowHudComponent { component: String, show: bool },
    /// Message type 15: `FrontEndState_MoviePlayer` with the `fe_movie` key of the
    /// message's `FMV_ID` (`cSignPostUtil::GetFMVMovieKeyFromName` `0x825ca340`).
    PlayMovie { message: String, movie: String, key: u64 },
    /// Message type 2: `NISManager::RunBasicNIS`.
    PlayNis { message: String, scene: String, transition: i32 },
    /// `CentralHUD::ShowCentralMessage`.
    CentralMessage(CentralMessage),
    /// `IAudioInterface::PlayMissionSpeech(character, id, param)` for an `eSk8Characters`
    /// value and speech id (`ChallengeAudioManager::PlayNPCSpeech` `0x82da1bb0` passes param 0,
    /// `PlaySpeech` `0x82da1b98` its own); the audio system answers with `cMsgSpeechEnded`.
    PlayNpcSpeech { character: i32, id: i32, param: i32 },
    /// `cMsgPlayCameraManCsisEvent` (`0x2B009311`) on the audio broadcaster:
    /// `CameraManCsisEventHandler` (`0x824ff7a0`) plays the trigger through
    /// `PlayCameraManSpeech` (`0x824fc3a0`) with the message's parameters, all unset.
    PlayCameraManCsisEvent { function: &'static str },
    /// `cMsgAbandonChallenge` at the audio system's handler on the challenge broadcaster,
    /// `AudioSystem::QuitChallenge` (`0x824ff478`).
    QuitChallengeMusic,
    /// `ChallengeAudioManager::RestartChallenge` (`0x82da1c48`):
    /// `IAudioInterface::RestartChallenge(isRetry)`.
    RestartChallengeAudio { retry: bool },
    /// `cChallengeTeleportManager::TeleportPlayerNow` →
    /// `ITeleportManager::TeleportPlayer`: move a player to a named location's
    /// spawn point `pos_id` at once; with `set_session_marker` the local
    /// player's session marker moves there too (`cMsgSetSessionMarker`).
    TeleportPlayer { player: i32, location: String, pos_id: i32, set_session_marker: bool },
    /// Message type 12, messages 256/257: `cMsgSpawnBigBlack` (and his map
    /// icon) or its removal.
    BigBlack { spawn: bool },
    /// `FrontEnd.OpenHOMResults`: `PushState(9)`, the Hall of Meat results.
    OpenHomResults,
    /// `cCrowdPositionBuffer::DispatchCrowdPositions`: `cMsgCrowdBufferChanged` on the
    /// presentation broadcaster; the presentation reads [`World::crowds`]' buffer as it takes it
    /// (`Presentation::ReceiveCrowdBufferChanged`, `CrowdPresentation::Update`).
    CrowdBufferChanged,
    /// `FrontEnd.ShowYesNoPopup` (no options: `ID_COMMON_NO` and `ID_COMMON_YES`) or
    /// `ShowGenericAttributePopup`: `FrontEndManager::OpenPopup` with the popup's id.
    ShowPopup { id: i32, title: String, description: String, options: Option<Vec<String>> },
    /// `FrontEnd.ClosePopup`: `FrontEndManager::ClosePopup` of the popup with this id.
    ClosePopup { id: i32 },
    /// `Teleport.ToWorldFileLocation`: `cTeleportManager::TeleportLocalToWorldLocation` to the
    /// location; `world_location` is the `fe_locations` key and `world_file` its world's file,
    /// none for the main world.
    TeleportLocalToWorldLocation { world_location: u64, world_file: Option<String>, location: String },
    /// `SignUpHUD`: the D-pad-up prompt's challenges (names), empty when it
    /// hides.
    SignUpPrompt { challenges: Vec<String> },
    /// `SignUpHUD::UpdateDPadUp` with several challenges: the choice popup
    /// (`MultipleSignupsCallback` answers it).
    SignUpChoice { options: Vec<String> },
    /// `FrontEndState_SignUpPoster::OpenPoster` for a challenge (name).
    OpenSignUpPoster { challenge: String },
    /// `FrontEndState_SignUpPoster::OpenCASPoster`.
    OpenCreateASpotPoster,
    /// `cMsgPinPlayer`: hold the local player in place while signing up.
    PinPlayer { pin: bool },
    /// `ITeleportManager::TeleportLocalPlayerStartChallenge`.
    TeleportStartChallenge { challenge: u64 },
    /// `cChallengeTeleportManager::TeleportLocalPlayerToChallengeEndPoint` (`0x82da30b8`):
    /// `cTeleportManager::TeleportLocalPlayer(location, onboard false, largeLoadRadius false)`
    /// (`0x828a92a8`).
    TeleportToChallengeEndPoint { location: String },
    /// `cTeleportManager::TeleportLocalPlayerToLastSessionMarker` (`0x828a9b18`): the game
    /// module's session marker as a session-marker `StateMessage_TeleportToPosition`.
    TeleportLocalPlayerToSessionMarker,
    /// `cMsgOpenUnlockOverlay`: `FrontEndState_Unlock` (FE state 10) over the
    /// completion results; [`ChallengeSystem::unlock_overlay_continue`]
    /// answers it.
    OpenUnlockOverlay,
    /// Message type 5: `cMsgSkate101Show` to the front end, which pushes
    /// `FrontEndState_Skate101` (FE state 79) with the message's title and
    /// text; its A answers with `cMsgHUDContinue`.
    ShowSkate101(Skate101View),
    /// Presentation events 30/31, as [`World::map_icons`] takes them: the icon
    /// held under `GetHashValue32(icon)` goes (`RemoveChallengeMapIcon`
    /// `0x82e06c38`); with a location, `MapBackEndManager::AddChallengeObjectIcon`
    /// (`0x825781f8`) adds the named `fe_map_icons` icon there.
    ChallengeMapIcon { icon: String, location: Option<String> },
    /// `cChallengeDynamicHullManager::SetDynamicHullState`: a dynamic-hull
    /// challenge's new state, for the session's trace.
    DynamicHullState { challenge: String, state: String },
    /// `cChallengeTeleportManager::SetSessionMarker` (`0x82da6e40`):
    /// `cMsgSetSessionMarker` at the local player's respawn transform
    /// (`Actor::CalcRespawnTransform`) and foot forward.
    SetSessionMarker,
    /// `SessionMarker.ClearSessionMarker` (`0x82da9350`): `cMsgClearSessionMarker` on the
    /// teleport manager's broadcaster.
    ClearSessionMarker,
    /// `StatsBackEnd::GrantAchievement` (`0x8255f580`): `Net::GrantAchievement`
    /// once per achievement (`mAchievementsInfo[a].mCompleted`).
    GrantAchievement { achievement: i32, time_delay: i32 },
    /// ServiceManager's original statistics and service achievements.
    ServiceStatistics { service: i32, paid: bool, pools: Option<(i32, i32)> },
    TrackHomStatistics {bail:super::mission::HomBail,enabled:bool},
    /// `NISManager::IgnoreTeleportToBreadcrumbOnNextNIS`.
    IgnoreTeleportToBreadcrumbOnNextNis,
    /// `cMsgOpenSponsorOverlay`: `FrontEndState_Sponsor` (FE state 11).
    OpenSponsorOverlay { sponsor: i32 },
    /// `FrontEndState_Unlock::SaveAndClose` with coverage unlocked: the
    /// coverage screen (FE state 61).
    OpenCoverageScreen,
    /// A Lua handler raised an error (the native would assert).
    ScriptError { context: String, error: String },
    /// `cMsgSetSimRate` (`0x3FBF5E43`) on the simulation broadcaster: the
    /// sim time of an update and the ticks of slow motion.
    SetSimRate { sim_time: f32, ticks: i32 },
    /// `cMsgSetPlayer` (`0x48D42C2F`) on the simulation broadcaster:
    /// `PartyPlayUI::SetPlayer` (`0x828aa4c0`) moves the input to the player.
    SetPlayer { player: i32 },
    /// `ISkaterManager::SetLocalIndex`: the camera's and HUD's skater.
    SetLocalIndex { player: i32 },
    /// `Actor::mFlags` `0x4000000` of an AI-driven skater, set or cleared at once.
    PinActor { player: i32, pin: bool },
    /// `TransitionManager::EndTransition` (`0x8289ed58`).
    EndTransition,
    /// FE state 38 (`FrontEndState_PressA`, screen 64) for a player's turn (1-based)
    /// and round; that player's A answers with `cMsgHUDComplete`.
    OpenPressADialog { player_turn: i32, round: i32 },
    /// `CentralHUD::ShowModifiedCentralMessage` (`0x826bc050`).
    ModifiedCentralMessage(CentralMessage),
    /// `cPathedHullLoader::LoadHull` of the asset manager's loader; the hull's
    /// completion answers through `assets::on_complete`.
    LoadHull { hull: u32, file_name: String, register_on_load: bool, high_priority: bool, asset_flags: u32 },
    /// `cPathedHullLoader::UnloadHull`, answered through `assets::on_complete`.
    UnloadHull { hull: u32 },
    /// `cPathedHullLoader::LoadHull` of the dynamic hull manager's own loader (the challenge
    /// content directory, load flags 7, file names clamped to 30 bytes): registered on load at
    /// normal priority; with `callback` the loader's `OnComplete` answers through
    /// [`dynamic_hull::on_complete`].
    LoadDynamicHull { hull: u32, file_name: String, callback: bool, asset_flags: u32 },
    /// `cPathedHullLoader::UnloadHull(fileName, nullptr)` of the dynamic hull manager's loader.
    UnloadDynamicHull { hull: u32 },
    /// `FrontEnd.OpenContinueRetryDialog`'s `PushState(8)`: `FrontEndState_EndChallengeOptions`
    /// over [`World::results`].
    OpenEndChallengeOptions(results::EndChallengeOptions),
    /// `ChallengeAIManager`'s requests of the AI skaters.
    Ai(AiRequest),
    /// A message on the audio system's broadcaster.
    MissionAudio(MissionAudio),
    /// `cChallengePauseManager::PauseSimulation`/`UnpauseSimulation` (`0x825b12f8`/
    /// `0x825b12a0`): `PauseMan::Push`/`Pull(kPAUSE_STATE_CHALLENGE)`.
    ChallengePause { pause: bool },
    /// `NISManager::KillNIS` (`0x828a2f88`): while a NIS runs, `cMsgNISUnloadAll` to the
    /// presentation.
    KillNis,
    /// `FrontEnd.OpenPhotoSelection`'s `PushState(5)`: `FrontEndState_PhotoChallenge` over
    /// `count` photos, FE state 8's win state set for after it.
    OpenPhotoSelection { count: i32 },
    /// `cMsgEnterRealTime` on the presentation thread's port: `Presentation::EnterRealTime`
    /// (`0x827f1130`) gives the restore a real-time clock again, ending a replay's scrub.
    EnterRealTime,
    /// `NISManager::RunReplayNIS` (`0x828a3c70`) from `cHighlightGroup::PlayHighlight` or a
    /// presentation group's Replay event: the scene played over the replay recorder's frames
    /// `start` to `end`.
    RunReplayNis {
        scene: String,
        start: u32,
        end: u32,
        hold_camera: bool,
        skip_enabled: bool,
        focus_player: i32,
        /// `NISPlaybackOverrides::useLocationValues` with `locationPosition` and
        /// `locationDirection`.
        location: Option<[[f32; 3]; 2]>,
        /// `useStreamPositionOnReturn` with `streamPosition`.
        stream_position: Option<[f32; 3]>,
        /// `eNISSpecialFlags`. A Hall of Meat highlight also selects the PC's full-speed HoM
        /// policy, rather than a camera scene's name.
        special_flags: i32,
    },
}

/// `GameModule::Update`'s `FrontEndManager::Update(0x682)`: the HUD update's time.
pub(crate) const FRONT_END_UPDATE_DT: u32 = 0x682;

/// Everything except the Lua states; natives receive this.
pub struct World {
    pub db: Arc<AttribDb>,
    pub queue: VecDeque<Event>,
    pub outputs: Vec<Output>,
    /// `StatsBackEnd::UpdateStats(item, value, force)` calls of the challenge code
    /// (`IncrementPointsPoolPoints`' cash credits, the receipt's purchases), for the session's
    /// back end.
    pub stat_updates: Vec<(usize, f32, bool)>,
    pub signpost: SignPost,
    /// `Sk8ChallengeSystem::GameModeFlags`; 4 is the career.
    pub game_mode_flags: u32,
    /// `kEnableProgression` (cvar).
    pub enable_progression: bool,
    progression_enabled: bool,
    start_signpost_next_frame: bool,
    pub activities: ActivityManager,
    pub post_office: PostOffice,
    /// `cMessageHandler::Attached`, per handler collection.
    pub attached: HashSet<u64>,
    /// `ILuaStateHolder::SetClosureData`: the activity whose handler runs.
    pub closure: Option<usize>,
    pub pointers: Pointers,
    pub hud: HudManager,
    /// The `objective` HUD component (`ObjectiveHUD`).
    pub objective_hud: ObjectiveHudComponent,
    /// The `tutorialinstruction` HUD component (`TutorialInstructionHUD`).
    pub tutorial_hud: TutorialHudComponent,
    /// The `signup` HUD component (`SignUpHUD`).
    pub signup_hud: SignUpHudComponent,
    /// `PlayersBackEnd::IsPlayerStanceRegular` for the local player, set by
    /// the session.
    pub stance_regular: bool,
    /// `FrontEndManager::IsStanceTrickGesturesFlipped` (`0x82614648`: the
    /// trick display's stance is not regular exactly when the player is
    /// regular), set by the session.
    pub stance_gestures_flipped: bool,
    /// `cChallengeHudTracker`: HUD groups created by an activity.
    pub hud_owners: Vec<(usize, u32)>,
    /// `StateManagerFrontEnd::mbDisableBackpackAndPauseMenu`.
    pub pause_menu_disabled: bool,
    pub indicators: IndicatorManager,
    /// `ChallengeFTLManager`.
    pub follow_the_leader: FtlManager,
    /// `cChallengeRaceManager`.
    pub race: RaceManager,
    /// `MapIconManager`'s icons and `MapBackEndManager`'s lists of them.
    pub map_icons: MapIcons,
    /// `cSignPostPinchMan`'s pinch points.
    pub pinch_points: PinchPoints,
    /// `Simulation::mTick`.
    pub tick: u32,
    /// `CommonBackEnd::sGameInfo`'s `startChallengeDuringLoad` and `isReplayingChallenge`.
    pub start_challenge_during_load: bool,
    pub replaying_challenge: bool,
    /// `cChallengeBEManager::mGotoAllowsSignup`.
    pub goto_allows_signup: bool,
    /// `Flow::Manager::IsActive(STATE_Loading)`: a map travel or a map challenge's load.
    pub flow_loading: bool,
    /// `BE::SaveLoadManager::IsMemcardActive`: a profile save is being written.
    pub memcard_active: bool,
    /// The skater manager's local player index.
    pub local_player: i32,
    pub presentation: PresentationDirector,
    pub assets: AssetManager,
    pub effects: EffectsManager,
    /// `VDWriter`'s challenge success and failure times.
    pub vd_results: vd_results::ResultTimes,
    pub teleport: TeleportManager,
    pub timers: TimerManager,
    pub objectives: ObjectivesManager,
    /// The objective whose condition is running (`tObjectiveClosureData`).
    pub objective_closure: Option<Closure>,
    /// Objective vars tables to drop from the objectives' Lua state.
    pub objective_vars_cleared: Vec<u64>,
    /// The local player's state for the objective natives, set each frame.
    pub player: PlayerState,
    /// Every player's state, by player index, where each player's objectives read their own
    /// player's (an online freeskate activity's, from every player's facts; PC port: the original
    /// read each skater's own output); none otherwise.
    pub player_states: Vec<(i32, PlayerState)>,
    /// `kIsHoMChallengeEnabled` (default true).
    pub hom_enabled: bool,
    /// `kObjectiveNotificationWindow` seconds (default 1).
    pub objective_notification_window: f32,
    /// `kSkaterStopThreshold` m/s (default 1.2).
    pub stop_threshold: f32,
    /// `cLocationManager` orientations by `GetHashValue32(name)`, from the
    /// loaded location data.
    pub locations: std::collections::HashMap<u32, [f32; 16]>,
    /// `cLocationManager::DefaultLocationPosistion`.
    pub default_location: [f32; 3],
    /// The running game's answers to DMO and collision queries.
    pub host: Box<dyn Host>,
    pub dmos: DmoManager,
    pub trick_groups: TrickGroupManager,
    pub signup: SignUpManager,
    /// `BackEndManager` module 5, the profile's unlocks.
    pub unlockables: Unlockables,
    /// `CASSimManager::mUnlockPartDB`, set by the session from the loaded
    /// part database.
    pub cas_unlock_parts: CasUnlockPartDb,
    /// `Sk8ChallengeSystem::mDynamicHullManager`'s states.
    pub dynamic_hulls: DynamicHulls,
    /// `FEBackEnd::Data.mBootFlowExecuted`, set once the main menu's front
    /// end state starts (`StateMainMenu::MainFEState` `0x82778448`).
    pub boot_flow_executed: bool,
    /// `CommonBackEnd::sGameInfo.gameMode`, set by the session.
    pub game_mode: i32,
    /// `CommonBackEnd::sGameInfo.additionalWorldLocation`, the world location (`fe_locations`
    /// key) of the additional world loaded, set by the session.
    pub additional_world_location: Option<u64>,
    /// The players back end, installed by the session.
    pub players: Option<Arc<dyn PlayersBackEnd>>,
    /// The network game an online challenge runs in, installed by the session.
    pub online: Option<super::players::OnlineChallenge>,
    /// The skater manager's skaters' completed outputs this update, in index order.
    pub skaters: Vec<MissionFrame>,
    /// `Sk8ChallengeSystem::SkateChallengeManager`.
    pub skate: SkateChallengeManager,
    /// `cTrickPatternManager`'s statics and `Sk8ChallengeSystem::ScoringManager`, shared
    /// with the skaters' scoring.
    pub scoring: Arc<std::sync::Mutex<ChallengeScoring>>,
    /// The Simulation's RNG, lent by the session while the system updates.
    pub(crate) random: Option<crate::native_skater_random::SimulationCommandRandom>,
    /// `Sk8ChallengeSystem`'s replay system (`DataSystem::DataMan`).
    pub replay: ReplaySystem,
    /// `cHighlightReelManager::HighlightGroups`.
    pub highlight_groups: Vec<HighlightGroup>,
    pub highlight_group_ids: u32,
    /// `ChallengeHOMManager::mWipeoutScore`.
    pub hom_wipeout_score: i32,
    /// The leaderboard, ABD, timer and throw-down message HUD components.
    pub turn_huds: turn_hud::TurnHuds,
    /// `FrontEndState_LeaderBoard` (FE state 27).
    pub results: results::LeaderboardState,
    /// `Sk8ChallengeSystem::VotingManager`.
    pub voting: voting::VotingManager,
    /// `Sk8ChallengeSystem::AIManager`.
    pub(crate) ai_manager: ChallengeAiManager,
    /// The AI singletons, installed by the session.
    pub(crate) ai_link: Option<ai::AiLink>,
    /// `Sk8ChallengeSystem::CharacterManager`.
    pub characters: CharacterManager,
    /// `Sk8ChallengeSystem::LivingWorldManager`.
    pub living_world: LivingWorldManager,
    /// `Sk8ChallengeSystem::SequenceConnectorManager`.
    pub(crate) sequence_connector: super::sequence_connector::SequenceConnectorManager,
    /// `Sk8ChallengeSystem::CalloutManager`.
    pub callouts: CalloutManager,
    /// `Sk8ChallengeSystem::CrowdManager` and the `cCrowdPositionBuffer` it fills.
    pub crowds: crowd::CrowdManager,
    /// `Sk8ChallengeSystem::ServiceManager`.
    pub services: ServiceManager,
    pub films: super::film::FilmManager,
    /// `Sk8ChallengeSystem::CompetitorsManager`.
    pub competitors: super::competitors::CompetitorsManager,
    /// `Sk8ChallengeSystem::HOMManager`.
    pub hom: super::hom::HomManager,
    pub hom_stats: std::sync::Arc<std::sync::Mutex<super::hom::HomStats>>,
    pub hom_local_calendar:Option<fn()->[u16;6]>,
    pub hom_location:String,
    pub hom_location_key:u64,
    /// The teleporters' sign-up HUD data and the scripts' popup ids.
    pub teleporters: super::teleporter::Teleporters,
    /// `Sk8ChallengeSystem::OtsDiscoveryManager`.
    pub(crate) ots_discovery: super::ots_discovery::OtsDiscoveryManager,
    /// The WorldPainter keys the film and service managers answer.
    pub world_painter: super::world_painter::WorldPainterKeys,
}

impl World {
    pub fn send(&mut self, event: Event) {
        self.queue.push_back(event);
    }
    pub fn output(&mut self, output: Output) {
        if let Output::BigBlack { spawn } = &output {
            characters::op(self, characters::LivingWorldOp::BigBlack(*spawn));
        }
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career output: {output:?}");
        }
        self.outputs.push(output);
    }
}

impl AttribHost for World {
    fn attrib_db(&self) -> &AttribDb {
        &self.db
    }
    fn pointers(&mut self) -> &mut Pointers {
        &mut self.pointers
    }
    fn data_providers(&self, activity: usize) -> Vec<(u64, u64)> {
        self.activities.data_providers(activity)
    }
}

/// `IActivitySubController::RunMainActivity` of the ported sub-controllers,
/// in `SubControllers` order.
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    ai::run_main_activity(world, activity);
    assets::run_main_activity(world, activity);
    characters::run_main_activity(world, activity);
    signup::run_main_activity(world);
    super::sequence_connector::run_main_activity(world, activity);
    living_world::run_main_activity(world, activity);
    map_icons::run_main_activity(world, activity);
    crowd::run_main_activity(world, activity);
    dmo::run_main_activity(world, activity);
    let key = world.activities.activities[activity].key;
    // cSignPostManager::RunMainActivity825d7e08: a replay already sent OnChallengeRetry.
    if !world.replaying_challenge {
        signpost::check_state_triggers(world, 3, key);
    }
    services::run_main_activity(world);
}

/// `IActivitySubController::CompleteChallenge` of the ported sub-controllers.
pub(crate) fn complete_challenge_sub_controllers(world: &mut World, activity: usize, is_main: bool) {
    ai::complete_challenge(world, activity, is_main);
    indicators::complete_challenge(world, activity, is_main);
    characters::complete_challenge(world, activity, is_main);
    // cChallengeBEManager::CompleteChallenge825b7920 clears this for every completed activity.
    world.replaying_challenge = false;
    follow_the_leader::complete_challenge(world, is_main);
    signup::complete_challenge(world);
    // cChallengeSimManager::CompleteChallenge 0x82da78e8: a main challenge other than a Flow
    // (type 44) whose local data has ClearSessionMarkerOnExit clears the session marker.
    if is_main && signup::challenge_type(world, activity) != 44 && clears_session_marker_on_exit(world, activity) {
        world.output(Output::ClearSessionMarker);
    }
    living_world::complete_challenge(world, is_main);
    map_icons::complete_challenge(world);
    teleport::complete_challenge(world, activity);
    dmo::complete_challenge(world, activity, is_main);
    super::hom::complete_challenge(world, activity);
    services::complete_challenge(world, is_main);
    if is_main {
        // cChallengeEffectsManager::CompleteChallenge 0x82da84d0.
        world.pause_menu_disabled = false;
        world.hud.set_visible(true, super::hud::VISIBILITY_CHALLENGE_EFFECTS);
        if world.effects.params[effects::FADE_OUT].active {
            world.effects.params[effects::FADE_OUT] = effects::EffectParams::default();
            #[expect(clippy::cast_precision_loss, reason = "enum value")]
            let effect = Field::Number(effects::FADE_OUT as f32);
            world.send(Event::Challenge(Message::new("cMsgEffectFinished", vec![("Effect", effect)])));
        }
        // cChallengeAssetManager::CompleteChallenge 0x82e0f8d8.
        if world.game_mode_flags & 8 == 0 {
            assets::unload(world, activity);
        }
    }
}

/// `challenge_local_data.ClearSessionMarkerOnExit`.
const CLEAR_SESSION_MARKER_ON_EXIT: u64 = 0x533b_aabd_fe3b_8a70;

fn clears_session_marker_on_exit(world: &World, activity: usize) -> bool {
    world.activities.activities[activity]
        .local_data
        .and_then(|(class, key)| world.db.attribute(class, key, CLEAR_SESSION_MARKER_ON_EXIT))
        .and_then(|attribute| attribute.bool(0))
        .unwrap_or(false)
}

/// `IActivitySubController::ShutdownActivity` of the ported sub-controllers.
pub(crate) fn shutdown_activity_sub_controllers(world: &mut World, activity: usize) {
    // cChallengeHudTracker::ShutdownActivity 0x82dac598: release owned groups.
    let owned: Vec<u32> = world.hud_owners.iter().filter(|(a, _)| *a == activity).map(|(_, h)| *h).collect();
    world.hud_owners.retain(|(a, _)| *a != activity);
    for hash in owned {
        world.hud.delete_group_hash(hash);
    }
    // cChallengePresentationDirector::ShutdownActivity 0x82e074f0.
    let key = world.activities.activities[activity].key;
    presentation::remove_groups(world, key);
    // cChallengeTimerManager::ShutdownActivity 0x82dac4c8.
    world.timers.shutdown_activity(activity);
    // cChallengeObjectivesManager::ShutdownActivity 0x82d4dd48.
    objectives::shutdown_activity(world, activity);
    indicators::shutdown_activity(world, activity);
    super::sequence_connector::shutdown_activity(world, activity);
    skate::shutdown_activity(world, activity);
    players::shutdown_activity(world, activity);
    ai::shutdown_activity(world, activity);
    dmo::shutdown_activity(world, activity);
    callouts::shutdown_activity(world, activity);
    crowd::shutdown_activity(world, activity);
    assets::shutdown_activity(world, activity);
    super::competitors::shutdown_activity(world, activity);
}

/// What `Mission.GetTitle()` reads (`MissionCmdTranslator::sMethod_GetTitle`
/// `0x8260e208`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MissionTitle {
    /// The main challenge's `challenges` title, through `FormatStringCustom`
    /// (its `#` format prefix makes `SkateAptString` show the result as is).
    Formatted(String),
    /// The main challenge has no title: `"#Missing Instance Title..."`.
    Missing,
    /// No main challenge, and `GetChallenge(sGameInfo.challengeKey)` is no online free skate
    /// challenge (none offline): `"ID_FREESKATE_CHALLENGE_LOBBY"`.
    #[default]
    Lobby,
    /// No main challenge in an online free skate game (`ChallengeType` 22):
    /// `"ID_FREESKATE_LOBBY"`.
    FreeskateLobby,
}

/// The challenge system's answers to `Sk8::SessionMarker`'s rules, each as (set, use).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkerRules {
    /// `GetMainActivity`'s challenge data `AllowSessionMarkersToBeSet` / `AllowSessionMarkersToBeUsed`
    /// (`VerifyChallengeOkToSet` 0x828ab5c0, `VerifyChallengeOkToUse` 0x828ab6d0); none without a
    /// main activity.
    pub main_activity: Option<(bool, bool)>,
    /// `cChallengeTeleportManager`'s set / use session marker override lists are empty.
    pub no_overrides: (bool, bool),
    /// `GetMessageSetting("Set Session Marker Visible", true)` / `("Use Session Marker Visible",
    /// true)` (`VerifyProgressionOkToSet` 0x828ab840, `VerifyProgressionOkToUse` 0x828ab940).
    pub visible: (bool, bool),
    /// `cChallengeSignUpManager::IsSigningUp` (`0x82da1ab0`).
    pub signing_up: bool,
    pub hom_allows_use: bool,
}

pub struct ChallengeSystem {
    pub(crate) progression: Lua<World>,
    pub(crate) challenge: Lua<World>,
    /// The objectives manager's own state (`"ObjectiveSystem"`).
    pub(crate) objectives: Lua<World>,
    pub(crate) world: World,
}

#[derive(Debug)]
pub struct ChallengeError(pub String);

impl fmt::Display for ChallengeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "challenge system: {}", self.0)
    }
}

impl std::error::Error for ChallengeError {}

impl From<LuaError> for ChallengeError {
    fn from(e: LuaError) -> Self {
        Self(e.to_string())
    }
}

fn print_sink(line: &str) {
    if std::env::var_os("SKATE2_TRACE_CAREER_LUA").is_some() {
        eprintln!("career lua: {line}");
    }
}

impl ChallengeSystem {
    /// `Sk8ChallengeSystem` construction, `cActivityManager::Init` and
    /// `cSignPostManager::Init` (`0x825e3b98`) for a career game
    /// (`GameLoad(gameTypeFlags = 4)`). `challenge_global` is
    /// `data\script/challenge/global.lua`, run by `PerformChallengeBinding`.
    pub fn new_career(db: Arc<AttribDb>, challenge_global: &[u8]) -> Result<Self, ChallengeError> {
        let mut progression = Lua::new();
        progression.print = Some(print_sink);
        signpost::bind(&mut progression);
        let signpost = SignPost::new(&db);
        let activities = ActivityManager::new(&db);
        let unlockables = Unlockables::new(&db);
        let mut system = Self {
            progression,
            challenge: Lua::new(),
            objectives: Lua::new(),
            world: World {
                db,
                queue: VecDeque::new(),
                outputs: Vec::new(),
                stat_updates: Vec::new(),
                signpost,
                game_mode_flags: 4,
                enable_progression: false,
                progression_enabled: false,
                start_signpost_next_frame: false,
                activities,
                post_office: PostOffice::default(),
                attached: HashSet::new(),
                closure: None,
                pointers: Pointers::default(),
                hud: HudManager::default(),
                objective_hud: ObjectiveHudComponent::default(),
                tutorial_hud: TutorialHudComponent::default(),
                signup_hud: SignUpHudComponent::default(),
                stance_regular: true,
                stance_gestures_flipped: false,
                hud_owners: Vec::new(),
                pause_menu_disabled: false,
                indicators: IndicatorManager::default(),
                follow_the_leader: FtlManager::default(),
                race: RaceManager::default(),
                map_icons: MapIcons::default(),
                pinch_points: PinchPoints::default(),
                tick: 0,
                start_challenge_during_load: false,
                replaying_challenge: false,
                goto_allows_signup: true,
                flow_loading: false,
                memcard_active: false,
                local_player: 0,
                presentation: PresentationDirector::default(),
                assets: AssetManager::default(),
                effects: EffectsManager::default(),
                vd_results: vd_results::ResultTimes::default(),
                teleport: TeleportManager::default(),
                timers: TimerManager::default(),
                objectives: ObjectivesManager::default(),
                objective_closure: None,
                objective_vars_cleared: Vec::new(),
                player: PlayerState::default(),
                player_states: Vec::new(),
                hom_enabled: true,
                objective_notification_window: 1.0,
                stop_threshold: 1.2,
                locations: std::collections::HashMap::new(),
                default_location: [0.0; 3],
                host: Box::new(NullHost),
                // cChallengeDMOManager::GameLoad 0x825b03f8: career (4).
                dmos: DmoManager::new(true),
                trick_groups: TrickGroupManager::default(),
                signup: SignUpManager::default(),
                unlockables,
                cas_unlock_parts: CasUnlockPartDb::default(),
                dynamic_hulls: DynamicHulls::default(),
                boot_flow_executed: false,
                game_mode: players::GM_SINGLE_PLAYER,
                additional_world_location: None,
                players: None,
                skaters: Vec::new(),
                skate: SkateChallengeManager::default(),
                scoring: Arc::default(),
                random: None,
                replay: ReplaySystem::default(),
                highlight_groups: Vec::new(),
                highlight_group_ids: 0,
                hom_wipeout_score: 0,
                turn_huds: turn_hud::TurnHuds::default(),
                results: results::LeaderboardState::default(),
                voting: voting::VotingManager::default(),
                ai_manager: ChallengeAiManager::default(),
                ai_link: None,
                characters: CharacterManager::default(),
                living_world: LivingWorldManager::default(),
                sequence_connector: super::sequence_connector::SequenceConnectorManager::default(),
                callouts: CalloutManager::default(),
                crowds: crowd::CrowdManager::default(),
                services: ServiceManager::default(),
                films: super::film::FilmManager::default(),
                competitors: super::competitors::CompetitorsManager::default(),
                hom: super::hom::HomManager::default(),
                hom_stats: Default::default(),
                hom_local_calendar:None,hom_location:"ID_COMMON_NEW_SAN_VANELONA".into(),hom_location_key:u64::MAX,
                teleporters: super::teleporter::Teleporters::default(),
                ots_discovery: super::ots_discovery::OtsDiscoveryManager::default(),
                world_painter: super::world_painter::WorldPainterKeys::default(),
                online: None,
            },
        };
        signup::init(&mut system.world);
        super::film::init(&mut system.world);
        characters::init(&mut system.world);
        dynamic_hull::init(&mut system.world);
        super::hom::init(&mut system.world);
        super::ots_discovery::init(&mut system.world);
        system.create_challenge_state(challenge_global)?;
        system.create_objectives_state()?;
        system.world.signpost.game_type_flags = 4;
        signpost::init(&mut system.world);
        Ok(system)
    }

    /// `cLuaStateManager::CreateState` (`0x82aae998`) for the challenge state
    /// (`Lua::Manager`): libraries, a silent `print`, the module bindings,
    /// `global.lua`, the attribute metatables and the message exporters.
    fn create_challenge_state(&mut self, challenge_global: &[u8]) -> Result<(), ChallengeError> {
        let lua = &mut self.challenge;
        lua.print = Some(print_sink);
        let mut list = natives::natives();
        list.extend(signpost::natives());
        list.extend(skate::natives());
        list.extend(players::natives());
        list.extend(ai::natives());
        list.extend(indicators::natives());
        list.extend(follow_the_leader::natives());
        list.extend(race::natives());
        list.extend(leaderboard_hud::natives());
        list.extend(results::natives());
        list.extend(voting::natives());
        list.extend(dpo_hud::natives());
        list.extend(dmo::natives());
        list.extend(map_icons::natives());
        list.extend(living_world::natives());
        list.extend(mission_audio::natives());
        list.extend(contest::natives());
        list.extend(callouts::natives());
        list.extend(crowd::natives());
        list.extend(online::natives());
        list.extend(objective_groups::natives());
        list.extend(assets::natives());
        list.extend(services::natives());
        list.extend(teleport::natives());
        list.extend(super::competitors::natives());
        list.extend(super::hom::natives());
        list.extend(super::teleporter::natives());
        binder::bind_modules(lua, &natives::MODULES, &list);
        if !challenge_global.is_empty() {
            let main = lua.load_source(challenge_global, "data\\script/challenge/global.lua")?;
            lua.call(&mut self.world, &main, &[])?;
        }
        attrib_lua::bind_metatables(lua);
        binder::bind_message_exporters(lua, &natives::message_exporters());
        Ok(())
    }

    /// `cLuaStateManager::CreateState` for the objectives' state: the
    /// objective modules, `InitVars`, the metatables and the exporters.
    fn create_objectives_state(&mut self) -> Result<(), ChallengeError> {
        let lua = &mut self.objectives;
        lua.print = Some(print_sink);
        binder::bind_modules(lua, &objective_natives::MODULES, &objective_natives::natives());
        let init = lua.load_source(objective_natives::INIT_VARS.as_bytes(), objective_natives::INIT_VARS)?;
        lua.call(&mut self.world, &init, &[])?;
        attrib_lua::bind_metatables(lua);
        binder::bind_message_exporters(lua, &natives::message_exporters());
        Ok(())
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn take_outputs(&mut self) -> Vec<Output> {
        std::mem::take(&mut self.world.outputs)
    }

    /// The requests not taken yet.
    #[must_use]
    pub fn outputs(&self) -> &[Output] {
        &self.world.outputs
    }

    /// `Sk8ChallengeSystem::SetEnableProgression` (`0x825a2d98`).
    pub fn set_enable_progression(&mut self, enabled: bool) {
        self.world.enable_progression = enabled;
    }

    /// `Sk8ChallengeSystem::Update` (`0x825a3f10`) for the ported parts: the
    /// activity manager (pending resets, sub-controller updates), effects, the
    /// progression manager, then the `kEnableProgression` toggle that starts
    /// the session one frame later.
    pub fn update(&mut self, loading: bool, dt: f32) {
        // cActivityManager::Update 0x82cdba20: pending resets, the global activities' box
        // tests while a game is loaded, then the sub-controllers in construction order.
        self.deliver_terminations();
        for activity in activity::take_pending_resets(&mut self.world) {
            shutdown_activity_sub_controllers(&mut self.world, activity);
            runtime::reset(&mut self.challenge, &mut self.world, activity);
        }
        if self.world.activities.game_loaded {
            activity::perform_bbox_tests(&mut self.world);
            self.deliver_terminations();
        }
        assets::update(&mut self.world);
        characters::update(&mut self.world);
        ai::update(&mut self.world);
        signup::update(&mut self.world);
        teleport::update(&mut self.world);
        timers::update(&mut self.world);
        objectives::update(&mut self.objectives, &mut self.world);
        crowd::update(&mut self.world);
        super::hom::update(&mut self.world);
        super::ots_discovery::update(&mut self.world);
        super::film::update(&mut self.world);
        voting::update(&mut self.world);
        race::update(&mut self.world);
        dmo::update(&mut self.world);
        dynamic_hull::update(&mut self.world);
        effects::update(&mut self.world, dt);
        follow_the_leader::update(&mut self.world);
        signpost::update(&mut self.progression, &mut self.world, loading);
        // Physics::IsHallOfMeatEffectEnabled 0x827acbd0 reads the "Hom Available" message
        // setting (default 1) on every scoring update.
        let hom_available = self.world.signpost.message_setting_value("Hom Available").is_none_or(|value| value != 0);
        // Physics::IsScoringMultiplierActive 0x827accc8 reads the main challenge's ChallengeType:
        // Photo (6), BestTrick (8), OwnTheSpot (9), MegaRamp (26), CreateASpot (35) and
        // CreateASpot_Setup (36) score without the multiplier.
        let multiplier_active = self.world.activities.main
            .is_none_or(|main| !matches!(signup::challenge_type(&self.world, main), 6 | 8 | 9 | 26 | 35 | 36));
        if let Ok(mut scoring) = self.world.scoring.lock() {
            scoring.hom_available = hom_available;
            scoring.multiplier_active = multiplier_active;
        }
        if self.world.progression_enabled == self.world.enable_progression {
            if self.world.start_signpost_next_frame {
                self.world.start_signpost_next_frame = false;
                signpost::start_game_session(&mut self.world);
                activity::reevaluate_prereq_activities(&mut self.world);
            }
        } else {
            self.world.progression_enabled = self.world.enable_progression;
            activity::reevaluate_prereq_activities(&mut self.world);
            if self.world.progression_enabled && self.world.game_mode_flags == 4 {
                self.world.start_signpost_next_frame = true;
            }
        }
        self.deliver_terminations();
        // The next update's Simulation::SimUpdate1 (0x8289f508) submits the crowds' line batches
        // before its physics, for its own Update to read.
        crowd::pre_update_submit(&mut self.world);
    }

    /// The front end's half of the frame: `HUDManager2::Update` reports groups
    /// that finished loading (`OnHUDEvent` → `cMsgHUDLoaded`).
    /// `cActivityManager::GameLoad(4)` (`0x82cd92c0`): the sub-controllers'
    /// `GameLoad`, then `ReevaluatePrereqActivities`. Runs once the locations
    /// and the profile's progression are loaded.
    pub fn game_load(&mut self) {
        activity::game_load(&mut self.world, 4);
    }

    /// `Sk8ChallengeSystem::GameLoad` 0x825a6618 in Party Play: `GenerateGameModeSignature`
    /// gives 16, so `cActivityManager::GameLoad(16)` skips the prerequisite pass and no
    /// SignPost session starts.
    pub fn game_load_party_play(&mut self) {
        self.world.game_mode_flags = 16;
        activity::game_load(&mut self.world, 16);
    }

    /// `Sk8ChallengeSystem::GameLoad` 0x825a6618 in a network game: `GenerateGameModeSignature`
    /// (0x825a4228) gives 8.
    pub fn game_load_network(&mut self) {
        self.world.game_mode_flags = 8;
        activity::game_load(&mut self.world, 8);
    }

    /// `cChallengeDynamicHullManager::GameUnload` (`0x82da6470`), as `Sk8ChallengeSystem::
    /// GameUnload` and `GameModule::GameLoad` run it: a loaded game's hulls are unloaded.
    pub fn game_unload_dynamic_hulls(&mut self) {
        dynamic_hull::game_unload(&mut self.world);
    }

    /// `cActivityManager::GameUnload` (`0x82cda420`) as the career's challenge system goes: the
    /// asset manager's `GameUnload` (`0x82e0f8c8`) unloads the main activity's assets, every
    /// running activity is terminated and reset, and the front end's next `Update` shuts the reset
    /// ones down, unloading their assets.
    pub fn game_unload_activities(&mut self) {
        services::game_unload(&mut self.world);
        // cChallengeBEManager::GameUnload825b7858.
        self.world.replaying_challenge = false;
        if let Some(main) = self.world.activities.main {
            assets::unload(&mut self.world, main);
        }
        activity::game_unload(&mut self.world);
        self.deliver_terminations();
        for activity in activity::take_pending_resets(&mut self.world) {
            shutdown_activity_sub_controllers(&mut self.world, activity);
            runtime::reset(&mut self.challenge, &mut self.world, activity);
        }
    }

    /// `cChallengeDynamicHullManager::SpotTestAgainstPoint` (`0x82db4318`) at the stream focus,
    /// from `StateLoading::DynamicHullState` (`0x82763be8`).
    pub fn spot_test_dynamic_hulls(&mut self, point: [f32; 3]) {
        dynamic_hull::spot_test_against_point(&mut self.world, point);
    }

    /// `Sk8ChallengeSystem::GameLoad` (`0x825a6618`) of a free-skate game
    /// (`GenerateGameModeSignature` 2) for the dynamic hull manager, the sub-controller the
    /// port runs in free skate: `GameModeFlags` 2 and the manager's `GameLoad(2)`.
    pub fn game_load_free_skate_dynamic_hulls(&mut self) {
        self.world.game_mode_flags = 2;
        dynamic_hull::game_load(&mut self.world, 2);
    }

    /// `Sk8ChallengeSystem::GameLoad` (`0x825a6618`) of a network game for the dynamic hull
    /// manager: `GenerateGameModeSignature` (`0x825a4228`) gives 8, under which the manager's
    /// `GameLoad` (`0x82da48e8`) loads no game, so no gate's hull loads.
    pub fn game_load_network_dynamic_hulls(&mut self) {
        self.world.game_mode_flags = 8;
        dynamic_hull::game_load(&mut self.world, 8);
    }

    /// The dynamic hull manager's `Update` (`0x82db4dd0`) with the local skater's tester base
    /// (`None` while its tester is not valid), as free skate runs it.
    pub fn update_dynamic_hulls(&mut self, tester_base: Option<[f32; 3]>) {
        self.world.player.tester_base = tester_base;
        dynamic_hull::update(&mut self.world);
    }

    /// The dynamic hull loader's `OnComplete` for a hull loaded with a callback.
    pub fn dynamic_hull_load_complete(&mut self, hull: u32) {
        dynamic_hull::on_complete(&mut self.world, hull);
    }

    /// Diagnostic cash, added as `cSignPostManager::AddCash(cash, 0)` (`0x825dd7a0`) adds it.
    pub fn diagnostic_add_cash(&mut self, cash: i32) {
        super::signpost_progress::add_cash(&mut self.world, cash, 0);
    }

    /// A diagnostic `Progression.ActivateChallengeByTitle(title)`, run in the progression state
    /// as a progression script runs it (`cSignPostManager::ActivateChallenge` `0x825e02c8`).
    ///
    /// # Errors
    ///
    /// Returns [`ChallengeError`] when the call fails.
    pub fn diagnostic_activate_challenge(&mut self, title: &str) -> Result<(), ChallengeError> {
        let source = format!("Progression.ActivateChallengeByTitle({title:?})");
        let call = self.progression.load_source(source.as_bytes(), "diagnostic")?;
        self.progression.call(&mut self.world, &call, &[])?;
        Ok(())
    }

    /// A diagnostic abandon of the main challenge, as the pause menu's `cMsgAbandonChallenge`
    /// without quitting. False without a main challenge.
    pub fn diagnostic_abandon(&mut self) -> bool {
        if self.world.activities.main.is_none() {
            return false;
        }
        start_hub::abandon(&mut self.world, false);
        true
    }

    /// A diagnostic start of the named global activity, as `ReevaluateGlobalActivtyBBoxes`
    /// (`0x82cdb260`) runs a full global activity or `PerformBBoxTests` (`0x82cdaf00`) one whose
    /// box the skater entered: its state graph runs, then the sub-controllers'
    /// `RunGlobalActivity`. False for an unknown name.
    pub fn diagnostic_run_global(&mut self, name: &str) -> bool {
        let Some(activity) = self.world.activities.activities.iter().position(|a| a.name == name) else { return false };
        activity::run_global_activity(&mut self.world, activity);
        true
    }

    /// A diagnostic `cMsgTeleporterSignUp` of the named teleporter, as its sign-up HUD's D-pad up
    /// sends it. False for an unknown name.
    pub fn diagnostic_teleporter_sign_up(&mut self, name: &str) -> bool {
        let Some(activity) = self.world.activities.activities.iter().position(|a| a.name == name) else { return false };
        let key = self.world.activities.activities[activity].key;
        self.world.send(Event::Challenge(Message::new("cMsgTeleporterSignUp", vec![("ActivityKey", Field::Key(key))])));
        true
    }

    /// The main challenge's name.
    #[must_use]
    pub fn main_challenge_name(&self) -> Option<String> {
        self.world.activities.main.map(|activity| self.world.activities.activities[activity].name.clone())
    }

    /// A diagnostic sign-up of the named challenge, as its sign-up volume's D-pad up would
    /// (`PlayerTriggerSignUp` without the start teleport). False for an unknown name.
    pub fn diagnostic_sign_up(&mut self, name: &str) -> bool {
        // The diagnostic may bypass progression, but must respect the AI release/load lifetime
        // that CanStartMainChallenge82cd8798 checks before an actual D-pad sign-up.
        if !activity::can_start_main_challenge(&self.world) { return false; }
        // A leading '@' signs up as the phone list's throwdowns and Pick-a-Pro do: a teleport start.
        let (name, teleport_start) = name.strip_prefix('@').map_or((name, false), |name| (name, true));
        let Some(activity) = self.world.activities.activities.iter().position(|a| a.name == name) else { return false };
        signup::player_trigger_sign_up(&mut self.world, activity, teleport_start);
        true
    }

    /// Shows the sign-up prompt for the named challenges for a diagnostic run, as if the skater
    /// stood in all their sign-up volumes.
    pub fn diagnostic_sign_up_prompt(&mut self, names: &[&str]) -> bool {
        let activities = &self.world.activities.activities;
        let Some(prompt) = names.iter().map(|name| activities.iter().position(|a| a.name == *name)).collect::<Option<Vec<_>>>() else {
            return false;
        };
        signup::diagnostic_prompt(&mut self.world, prompt);
        true
    }

    /// A diagnostic step through the main challenge: the first active objective of each of its
    /// groups made complete as `Objectives.ForceObjectiveComplete` does
    /// (`cChallengeObjectiveGroup::ForceComplete` 0x82d4f480). Returns the objectives forced.
    pub fn diagnostic_force_active_objectives(&mut self) -> Vec<String> {
        let Some(main) = self.world.activities.main else { return Vec::new() };
        let targets: Vec<(u32, usize, u64)> = self
            .world
            .objectives
            .groups
            .iter()
            .filter(|g| g.activity == main)
            .filter_map(|g| {
                let index = g.objectives.iter().position(|o| o.state == objectives::Status::Active)?;
                Some((g.id, index, g.objectives[index].key))
            })
            .collect();
        targets
            .into_iter()
            .map(|(group, index, key)| {
                objectives::force_complete(&mut self.world, group, index);
                objectives::objective_name(&self.world.db, key).unwrap_or_default()
            })
            .collect()
    }

    /// A diagnostic step through the main challenge: the first active objective of each of its
    /// groups completed as its completion script would complete it
    /// ([`objectives::diagnostic_complete`], highlights included). Returns the objectives completed.
    pub fn diagnostic_complete_active_objectives(&mut self) -> Vec<String> {
        let Some(main) = self.world.activities.main else { return Vec::new() };
        let targets: Vec<(u32, usize, u64)> = self
            .world
            .objectives
            .groups
            .iter()
            .filter(|g| g.activity == main)
            .filter_map(|g| {
                let index = g.objectives.iter().position(|o| o.state == objectives::Status::Active)?;
                Some((g.id, index, g.objectives[index].key))
            })
            .collect();
        targets
            .into_iter()
            .map(|(group, index, key)| {
                objectives::diagnostic_complete(&mut self.world, group, index);
                objectives::objective_name(&self.world.db, key).unwrap_or_default()
            })
            .collect()
    }

    /// [`MarkerRules`] for the current challenge state.
    #[must_use]
    pub fn session_marker_rules(&self) -> MarkerRules {
        let world = &self.world;
        let allows = |activity: usize, name: &str| {
            let key = world.activities.activities[activity].key;
            world.db.attribute(keys::class::CHALLENGES, key, keys::hash64(name)).and_then(|a| a.bool(0)).unwrap_or(false)
        };
        let visible = |name: &str| world.signpost.message_setting_value(name).is_none_or(|value| value != 0);
        MarkerRules {
            main_activity: world
                .activities
                .main
                .map(|main| (allows(main, "AllowSessionMarkersToBeSet"), allows(main, "AllowSessionMarkersToBeUsed"))),
            no_overrides: (
                world.teleport.set_session_marker_overrides.is_empty(),
                world.teleport.use_session_marker_overrides.is_empty(),
            ),
            visible: (visible("Set Session Marker Visible"), visible("Use Session Marker Visible")),
            signing_up: signup::is_signing_up(world),
            hom_allows_use: world.hom.allow_marker,
        }
    }

    /// `cActivityManager::StartMainChallenge` 0x82cda588, as `StateLoading::StartChallengeState`
    /// 0x8274f800 runs it for `eLoadingType_StartNewPartyPlayChallenge`.
    pub fn start_main_challenge(&mut self, key: u64) {
        activity::start_main_challenge(&mut self.world, key);
    }

    /// `StateLoading::PartyPlayLoadState` 0x82770ea8's `LoadChallengeAssets(challengeKey, true,
    /// false, 0)` (`cChallengeAssetManager` 0x82e123b0).
    pub fn load_challenge_assets(&mut self, key: u64) {
        if let Some(activity) = self.world.activities.find(key) {
            assets::load_challenge_assets(&mut self.world, activity, true, false, 0);
        }
    }

    /// `GameModule::GameLoad` (`0x8273be50`)'s network game `LoadChallengeAssets(challengeKey,
    /// true, false, 7)` (`0x8273cc44`): asset flags 7 register the hulls' DMOs with
    /// `Dmo::Registration` (`RegDMOArena` 0x828a0420), whose locators `SpawnStatic` places.
    pub fn load_online_challenge_assets(&mut self, key: u64) {
        if let Some(activity) = self.world.activities.find(key) {
            assets::load_challenge_assets(&mut self.world, activity, true, false, 7);
        }
    }

    /// `TeleportLocalPlayerStartChallenge` (`0x828a9678`): `sGameInfo.startChallengeDuringLoad`.
    pub(crate) fn start_challenge_during_load(&mut self) {
        self.world.start_challenge_during_load = true;
    }

    /// `MapBackEndManager::TeleportToChallenge` (`0x8256dc10`): replay notification before
    /// loading, or an active Own the Spot's direct start. The caller has resolved `Location`.
    /// The selected key lives in the session's loading state until `StartChallengeState`.
    pub(crate) fn prepare_map_challenge(&mut self, key: u64, retry: bool) -> bool {
        let Some(activity) = self.world.activities.find(key) else { return false };
        if retry {
            self.world.replaying_challenge = true;
            // cSignPostManager::OnChallengeRetry825d7f90.
            signpost::check_state_triggers(&mut self.world, 7, key);
        }
        let start = retry || signup::challenge_type(&self.world, activity) == 9;
        if start {
            self.world.start_challenge_during_load = true;
        }
        start
    }

    /// `StateLoading::StartChallengeState` (`0x8274f800`): select AI before the hull load;
    /// count four ready updates, run DMO operations, clear the loading request, then start.
    /// `LoadChallengeAssets(key, true, false, 0)` is confirmed by raw PPC at 8274f950..64.
    /// The host hull loader and AI recipe loader continue in the session's normal frame pump.
    pub(crate) fn update_map_challenge_load(&mut self, key: u64, ready_updates: &mut u32) -> Result<bool, String> {
        let activity = self.world.activities.find(key).ok_or_else(|| format!("map challenge absent: {key:#018x}"))?;
        let ai_ready = if ai::is_activity_an_ai_challenge(&self.world, activity) {
            ai::select_characters(&mut self.world, activity, false)?;
            ai::are_all_assets_loaded(&self.world)
        } else {
            true
        };
        assets::load_challenge_assets(&mut self.world, activity, true, false, 0);
        assets::update(&mut self.world);
        if !assets::are_hulls_loaded(&self.world, activity) || !ai_ready {
            return Ok(false);
        }
        *ready_updates += 1;
        if *ready_updates <= 3 {
            return Ok(false);
        }
        dmo::run_dmo_operations(&mut self.world, activity, true);
        self.world.start_challenge_during_load = false;
        activity::start_main_challenge(&mut self.world, key);
        Ok(true)
    }

    /// `cChallengeAssetManager::IsAssetLoadComplete(challengeKey)` 0x82e0f990, which
    /// `PartyPlayLoadState` waits for: no load, or one past its notify.
    #[must_use]
    pub fn is_asset_load_complete(&self, key: u64) -> bool {
        self.world.activities.find(key).is_none_or(|activity| assets::is_load_complete(&self.world, activity))
    }

    /// `cChallengeAssetManager::IsAllUnloadComplete` (0x82e0f400): every load unloaded.
    #[must_use]
    pub fn is_all_unload_complete(&self) -> bool {
        assets::is_all_unload_complete(&self.world)
    }

    /// `FrontEndManager::StartInGame` (`0x82640450`), its HUD and SignPost
    /// parts: the `main` group (central, minimap, trickdisplay, homscoring,
    /// online_chyron) and, outside Party Play, `phonelist`, each made active
    /// when created; trickdisplay, minimap, online_chyron and phonelist
    /// shown, homscoring hidden, and the "Minimap Visible" setting set.
    pub fn start_in_game(&mut self, party_play: bool) {
        let hud = &mut self.world.hud;
        if !hud.group_exists("main") {
            hud.create_group("main", &["central", "minimap", "trickdisplay", "homscoring", "online_chyron"]);
            hud.add_group_to_active("main");
        }
        if !party_play && !hud.group_exists("phonelist") {
            hud.create_group("phonelist", &["phonelist"]);
            hud.add_group_to_active("phonelist");
        }
        for component in ["trickdisplay", "minimap", "online_chyron"] {
            hud.show("main", component, true);
        }
        self.world.signpost.set_message_setting_value("Minimap Visible", 1);
        self.world.hud.show("main", "homscoring", false);
        if !party_play {
            self.world.hud.show("phonelist", "phonelist", true);
        }
    }

    /// `cActivityManager::LoadingReset` (`0x82cd8728`, from
    /// `StateLoading::WorldPrepareState`): the sub-controllers' `LoadingReset`
    /// once the world is in.
    pub fn loading_reset(&mut self) {
        signup::loading_reset(&mut self.world);
    }

    /// The D-pad-up press (`SignUpHUD::UpdateInput(1)`) and release (0x4000). True when the
    /// several-challenges popup opened.
    pub fn sign_up_dpad_up(&mut self) -> bool {
        signup::dpad_up(&mut self.world)
    }
    pub fn sign_up_dpad_released(&mut self) {
        signup::dpad_released(&mut self.world);
    }
    /// `SignUpHUD::MultipleSignupsCallback`: the chosen option, or a cancel.
    pub fn sign_up_choose(&mut self, option: Option<usize>) {
        signup::choose(&mut self.world, option);
    }
    /// What the sign-up poster's screen reads for the selected challenge, which `OpenPoster`
    /// (`0x82658240`) keeps as the poster's `mKey`.
    pub fn sign_up_poster_view(&mut self) -> Option<PosterView> {
        let activity = self.world.signup.selected?;
        self.world.signup.poster = self.world.activities.activities[activity].key;
        Some(signup_poster::view(&mut self.world, activity))
    }
    /// `FrontEndState_SignUpPoster::ReceiveInput` accept (keycode 2).
    pub fn sign_up_accept(&mut self) -> PosterAccept {
        signup_poster::accept(&mut self.world)
    }
    /// The poster's decline (keycode 3).
    pub fn sign_up_decline(&mut self) {
        signup_poster::decline(&mut self.world);
    }
    /// `StateMainMenu::MainFEState`: the boot flow has run.
    pub fn set_boot_flow_executed(&mut self) {
        self.world.boot_flow_executed = true;
    }

    /// What the unlock overlay's screen reads (`FEUnlock` natives).
    #[must_use]
    pub fn unlock_overlay_view(&self) -> UnlockOverlayView {
        unlock_overlay::view(&self.world)
    }

    /// The unlock overlay's A (`FrontEndState_Unlock::ReceiveInput`
    /// `0x826595c0`) once the boot flow has run: `SaveAndClose` (`0x82659468`)
    /// sends `cMsgHUDContinue`, or moves to the coverage screen when coverage
    /// was unlocked; leaving the overlay (`OnLeave` `0x82652990`) resets the
    /// cash awards (`ResetCashAwards` `0x825c44c8`) and the completion results.
    /// The native takes the press only after its cash count animation.
    pub fn unlock_overlay_continue(&mut self) {
        if self.world.signpost.completion.coverage_unlocked {
            self.world.output(Output::OpenCoverageScreen);
        } else {
            self.world.send(Event::HudContinue);
        }
        self.world.signpost.progress.cash_earned = [0; 7];
        self.world.signpost.completion.clear();
    }

    /// What the sponsor overlay's screen reads for `state`; `selected` is FE state 11's
    /// `mSelectedSponsorIndex`.
    #[must_use]
    pub fn sponsor_overlay(&self, state: i32, selected: &mut usize) -> super::sponsor_overlay::SponsorOverlayHud {
        super::sponsor_overlay::hud(&self.world, state, selected)
    }

    /// The sponsor overlay's `NextSelected` for `state` with the picked sponsor `selected`.
    pub fn sponsor_next_selected(&mut self, state: i32, selected: usize) {
        super::sponsor_overlay::next_selected(&mut self.world, state, selected);
    }

    /// Diagnostics: `cSignPostManager::CompleteChallenge(name)` (`0x825e3098`).
    pub fn diagnostic_complete_challenge(&mut self, name: &str) {
        super::signpost::complete_challenge_by_name(&mut self.world, name);
    }

    /// Diagnostics: a progression message sent, after a text message setting stored.
    pub fn diagnostic_send_message(&mut self, setting: Option<(&str, &str)>, message: &str) {
        super::sponsor_overlay::diagnostic_send_message(&mut self.world, setting, message);
    }

    /// `HUDManager2::Update` (`0x82cee4f0`) for the ported components, after
    /// the frame's challenge messages are delivered.
    pub fn update_hud(&mut self) {
        let changed = std::mem::take(&mut self.world.hud.active_groups_changed);
        objective_hud::update(&mut self.world, changed);
        tutorial_hud::update(&mut self.world, changed);
        signup_hud::update(&mut self.world, changed);
        turn_hud::update(&mut self.world, changed, FRONT_END_UPDATE_DT);
    }

    pub fn update_frontend(&mut self) {
        map_icons::update_frontend(&mut self.world);
        for name in self.world.hud.take_loaded_groups() {
            let hash = keys::hash_value32(&name);
            #[expect(clippy::cast_precision_loss, reason = "(float)(int)")]
            let field = Field::Number(hash.cast_signed() as f32);
            self.world.send(Event::Challenge(Message::new("cMsgHUDLoaded", vec![("GroupHash", field)])));
        }
    }

    /// `EventSys::Flush(queue 7)`: deliver until the queue is empty, including
    /// events queued during delivery. A Hermes message reaches the native
    /// handlers in construction order (teleport manager, progression manager,
    /// presentation director, objectives manager), then the Lua post office.
    pub fn flush(&mut self) {
        loop {
            self.deliver_terminations();
            let Some(event) = self.world.queue.pop_front() else { break };
            if let Event::ChangeState { graph, state } = event {
                runtime::change_state_event(&mut self.challenge, &mut self.world, graph, state);
                continue;
            }
            // Sk8ChallengeSystem's own handler, which its constructor registers.
            start_hub::receive(&mut self.world, &event);
            teleport::receive(&mut self.world, &event);
            players::receive(&mut self.world, &event);
            signpost::receive(&mut self.progression, &mut self.world, &event);
            presentation::receive(&mut self.world, &event);
            super::sequence_connector::receive(&mut self.world, &event);
            objectives::receive(&mut self.objectives, &mut self.world, &event);
            callouts::receive(&mut self.world, &event);
            super::ots_discovery::receive(&mut self.world, &event);
            super::hom::receive(&mut self.world, &event);
            // cMsgHUDIntroDone, which no other handler takes.
            signup::receive(&mut self.world, &event);
            // The Visual Director's VDWriter and the audio system, on the same broadcaster.
            vd_results::receive(&mut self.world, &event);
            audio_handlers::receive(&mut self.world, &event);
            services::receive(&mut self.world, &event);
            if let Some(message) = event.lua_message() {
                runtime::route_message(&mut self.challenge, &mut self.world, &message);
            }
        }
        // Native cLuaStateManager::CreateState (0x82aae998, call at
        // 0x82aaeadc) stops automatic GC; its arena is reset separately.
        // A full sweep of all three host heaps after every event flush has no
        // native counterpart. Retain the host's existing allocation-triggered
        // cleanup at this safe boundary rather than scanning every live object
        // during ordinary updates (including an empty flush).
        self.progression.maybe_collect();
        self.challenge.maybe_collect();
        self.objectives.maybe_collect();
    }

    /// FE state 8 as the photo selection's `PhotoDone` opens it.
    #[must_use]
    pub fn photo_end_challenge_options(&self, win: bool) -> results::EndChallengeOptions {
        results::photo_end_options(&self.world, win)
    }

    /// `Simulation::PostUpdate` (`0x8289fd88`)'s challenge half:
    /// `cHighlightReelManager::HighlightUpdate` (`0x82cd6be0`).
    pub fn post_update(&mut self) {
        players::highlight_update(&mut self.world);
    }

    /// `VDManager::DoSimSubmit` (`0x82800120`)'s challenge success and failure info of this
    /// simulation update ([`vd_results::write`]).
    pub fn submit_result_info(&mut self) -> vd_results::ResultInfo {
        vd_results::write(&mut self.world)
    }

    /// `DataSysMan::SubmitFrame` (`0x8277bca0`) into the replay recorder's index: the frame of
    /// this update's tick (`Simulation::mTick`, as `Time.GetVisualGameTick` reads it). Its
    /// number, whether it was stored and the frames it let go.
    pub fn store_replay_frame(&mut self) -> (u32, bool, Vec<u32>) {
        let tick = self.world.tick;
        let (stored, dropped) = self.world.replay.store_frame(tick);
        (tick, stored, dropped)
    }

    /// The replay recorder's index (`DataSystem::DataMan`).
    pub fn replay_mut(&mut self) -> &mut ReplaySystem {
        &mut self.world.replay
    }

    /// The replay recorder's index (`DataSystem::DataMan`).
    #[must_use]
    pub fn replay(&self) -> &ReplaySystem {
        &self.world.replay
    }

    /// Hermes message from the game/presentation side onto the Challenge port.
    pub fn post(&mut self, event: Event) {
        self.world.send(event);
    }

    /// The progression `print`/error trace, for diagnostics.
    #[must_use]
    pub fn global(&self, name: &str) -> Value {
        self.progression.get_global(name)
    }

    /// The objectives of the main challenge's groups: (name, status).
    #[must_use]
    pub fn objective_summary(&self) -> Vec<(String, objectives::Status)> {
        let Some(main) = self.world.activities.main else { return Vec::new() };
        self.world
            .objectives
            .groups
            .iter()
            .filter(|g| g.activity == main)
            .flat_map(|g| g.objectives.iter())
            .map(|o| (objectives::objective_name(&self.world.db, o.key).unwrap_or_default(), o.state))
            .collect()
    }

    /// The running main challenge (`cActivityManager::MainActivity`).
    #[must_use]
    pub fn main_challenge(&self) -> Option<String> {
        self.world.activities.main.map(|a| self.world.activities.activities[a].name.clone())
    }

    /// `cActivityManager::RestartMainChallenge(playerID, isRetry)` (`0x82cd8880`): each
    /// sub-controller's `RestartChallenge` (of these the living world, DMO and audio managers';
    /// the presentation director's is not ported), then outside a retry
    /// `cStateGraph::RestartMission` (`0x82ce4f30`): a local `cMsgRestartChallenge` to the main
    /// activity's graph.
    pub fn restart_main_challenge(&mut self, player: i32, retry: bool) {
        let Some(main) = self.world.activities.main else { return };
        living_world::restart_challenge(&mut self.world, main);
        dmo::restart_challenge(&mut self.world, main);
        self.world.output(Output::RestartChallengeAudio { retry });
        if !retry {
            #[expect(clippy::cast_precision_loss, reason = "the message's (float) field")]
            let message = Message::new("cMsgRestartChallenge", vec![("PlayerID", Field::Number(player as f32))]);
            runtime::handle_local_message(&mut self.challenge, &mut self.world, main, &message);
        }
    }

    /// `cActivityManager::GetMainChallenge`: whether a main challenge runs.
    #[must_use]
    pub const fn has_main_challenge(&self) -> bool {
        self.world.activities.main.is_some()
    }

    /// `Mission.GetTitle()` (`MissionCmdTranslator::sMethod_GetTitle`
    /// `0x8260e208`) for the main challenge.
    #[must_use]
    pub fn mission_title(&self) -> MissionTitle {
        let Some(a) = self.world.activities.main else { return MissionTitle::Lobby };
        let key = self.world.activities.activities[a].key;
        self.world
            .db
            .attribute(keys::class::CHALLENGES, key, objective_hud::CHALLENGE_TITLE)
            .and_then(|a| a.text(0))
            .map_or(MissionTitle::Missing, |t| MissionTitle::Formatted(t.to_owned()))
    }

    /// `NIS.GetProInfo()` (`NISCmdTranslator::sMethod_GetProInfo`
    /// `0x8260d840`): the main challenge's pro by its name
    /// (`fp_u3_tutorial03` Way, `04` Haslam, `01` Dill, `02` Rodriguez,
    /// any other Carroll) as `NISBackEnd`'s name, hometown and stance
    /// labels. Without a main challenge, or with a nonzero
    /// `challenge_global_data.ChallengeType`, the literal placeholders.
    #[must_use]
    pub fn pro_info(&self) -> [String; 3] {
        let placeholder = || ["#Bad Pro".into(), "#Brooklyn, NY".into(), "#Goofy".into()];
        let Some(a) = self.world.activities.main else { return placeholder() };
        if super::signup::challenge_type(&self.world, a) != 0 {
            return placeholder();
        }
        let pro = match self.world.activities.activities[a].name.as_str() {
            "fp_u3_tutorial03" => 7,
            "fp_u3_tutorial04" => 8,
            "fp_u3_tutorial01" => 4,
            "fp_u3_tutorial02" => 1,
            _ => 5,
        };
        let pro = &crate::nis::back_end::PROS[pro];
        let stance = if pro.goofy { "ID_CHEAT_PRODETAIL_GOOFY" } else { "ID_CHEAT_PRODETAIL_REGULAR" };
        [pro.name.into(), pro.hometown.into(), stance.into()]
    }

    /// The main challenge's current state name.
    #[must_use]
    pub fn main_challenge_state(&self) -> Option<String> {
        let a = self.world.activities.main?;
        self.world.activities.graph(a).current_name().map(str::to_owned)
    }

    #[must_use]
    pub fn state_machine_names(&self) -> Vec<String> {
        self.world.signpost.running_machine_names(&self.world.db)
    }

    /// `cMsgMessageSent` kind, for tests and traces.
    #[must_use]
    pub fn message_sent_kind() -> u32 {
        keys::hash32("cMsgMessageSent")
    }
}
