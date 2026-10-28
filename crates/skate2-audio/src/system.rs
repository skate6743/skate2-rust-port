//! `Sk8::Audio::AudioSystem` (0x824e64b8-0x8250a300): the `Update` order, the HUD and menu sound
//! path (`PlayMenuSound` 0x82504dd0 -> `cMsgPlayMenuSound` -> `PlayMenuSoundImp` 0x8250c738 ->
//! `PlayFEHudSplice` 0x824e7b28 -> `UpdateFEHudSplices` 0x824fea50), the menu flange voice, the MAIN
//! mixer state objects that produce the menu volumes, `GlobalEffects` (`crate::global_effects`),
//! and the PLAYER and COLLISION state managers (`crate::player`, `crate::collision`).
//!
//! Of the fourteen MAIN SFX objects, `SFXObj_Master`, `SFXObj_VU`, `SFXObj_Challenge`,
//! `SFXObj_Menu` and `SFXObj_NIS` write the only inputs that reach `SFXObj_Master`'s outputs 0, 1
//! and 10 (`mMenuVol`, `mFESFXVol`, `mMenuFlangeLevel`) in `MixMapSK8.mxb`, `SFXObj_Pause`
//! writes the inputs that silence the world objects' outputs while the game is paused, and
//! `SFXObj_Reverb` (`crate::reverb`) runs the environment reverbs and flanges. `SFXObj_Bloom`,
//! `SFXObj_HOM`, `SFXObj_Jitter`, `SFXObj_NIS`'s volumes, the Hall of Meat bank and the
//! `Sk8_moments.csi` blooms and logos are in `crate::main_objects`; `SFXObj_Speech` and the
//! speaking flags come from `crate::speech`, as do the scenes' speech and posts and the mission
//! speech behind the `IAudioInterface` calls below, with `SFXObj_CameraMan` and
//! `SFXObj_Announcer` placing the cameraman's and announcers' voices; the music system,
//! `SFXObj_Music` and its `SFXCTL_MusicPos` are `crate::music`. Replay-controlled scenes
//! restore the recorded world audio, playback speed and direction through the same objects.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::aems::{self, Aems, ClassObject};
use crate::collision::CollisionManager;
use crate::global_effects::{self, GlobalEffects};
use crate::main_objects::{self, Bloom, Hom, HomBank, Jitter, Moments, NisVolumes};
use crate::movie::{self, MovieAudio, MovieSound};
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, DynamicMixer, MixerHost, PresetType};
use crate::player::physics::{PhysicsInputs, WHEEL_SOFT};
use crate::player::pos3d::{CameraMatrix, MicrophoneMan};
use crate::player::skateboard::GrainFiles;
use crate::player::{CollisionInfo, Cvars, PlayerFrame, PlayerManager, SfxCtx};
use crate::presentation::{PhysOutView, SkaterData};
use crate::random::RwRandom;
use crate::reverb::Reverb;
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::{AudioCore, PluginRef, VoiceId};
use crate::splicer::{SpliceId, SpliceManager, SplicePlayParams, SpliceUpdateParams};
use crate::vault::Vault;
use crate::world::environment::EnvironmentManager;
use crate::world::traffic::TrafficManager;
use crate::world::pedestrian::PedestrianManager;
use crate::world::emitter::EmitterManager;
use crate::world::emitter_system::{self, EmitterFiles, EmitterIo, Sk8EmitterSystem};
use crate::world::crowd::{CrowdEnv, CrowdManager, CrowdPodInfo};
use crate::world::dynamic_object::DynamicObjectManager;
use crate::world::speaker::{SpeakerManager, SPEAKER_MUSIC};
use crate::world::whoosh::WhooshManager;
use crate::world::object_instance::ObjectInstanceManager;
use crate::world::nis_character::NisCharacterManager;
use crate::speech::{self, GameValues, NisSpeechObject, Sk8SpeechSystem, SpeakerObject, SpeechCtx, SpeechGameState, SpeechMessage, SpeechObjects};
use crate::music::{self, MusicIo, MusicSystem};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[path = "system_contest.rs"]
mod contest;
pub use contest::{mission_sfx, module_status};

/// `SPLICE_TYPE_FE`: `Sk82_FE.bnk`.
pub const SPLICE_FE: usize = 25;
/// `SPLICE_TYPE_MENU`: `sk8_menu.bnk`.
pub const SPLICE_MENU: usize = 26;
/// The Hall of Meat bank's splice type.
pub const SPLICE_HOM: usize = 7;
/// `MAP_CREATE_PARAMS::NumMixStates` (`InitializeMixer` 0x824ed830).
pub const MIX_STATES: usize = 13;
/// `__real_38000100` (1/32767).
const Q15_SCALE: f32 = f32::from_bits(0x3800_0100);

/// `m_CurNumStates` of the thirteen state managers after `EnterGamePlay`, on the first
/// `EnterWorld`: Main 1, Player 2, Environment 1, Collision 10, TrafficCar 4, Pedestrian 15,
/// Emitter/Crowd/DynamicObject 5, Speaker 4, Whoosh 3, ObjectInstance/NISCharacter 5.
pub const STATE_COUNTS: [i32; MIX_STATES] = [1, 2, 1, 10, 4, 15, 5, 5, 5, 4, 3, 5, 5];

/// SFX ids the MAIN state creates (`Setup(0x7FFF)` over the classes registered for state 0).
const MAIN_SFX_OBJECTS: [u32; 14] = [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
/// SFXCTL ids the MAIN state creates: `SFXCTL_MusicPos` (`crate::music::sfx`).
const MAIN_SFX_CONTROLS: [u32; 1] = [0];
const SFX_ANNOUNCER: u32 = 0;
const SFX_MASTER: u32 = 2;
const SFX_CAMERA_MAN: u32 = 3;
const SFX_REVERB: u32 = 5;
const SFX_NIS: u32 = 6;
const SFX_PAUSE: u32 = 7;
const SFX_SPEECH: u32 = 8;
const SFX_BLOOM: u32 = 9;
const SFX_VU: u32 = 10;
const SFX_CHALLENGE: u32 = 11;
const SFX_HOM: u32 = 12;
const SFX_MENU: u32 = 13;
const SFX_JITTER: u32 = 14;

/// `eSk8HUDState` values the MAIN objects test.
pub mod hud_state {
    pub const IDLE: i32 = 0x00;
    pub const FREE_SKATE: i32 = 0x01;
    pub const REPLAY2: i32 = 0x0C;
    pub const TRICK_TUTORIAL: i32 = 0x12;
    pub const CAC: i32 = 0x13;
    pub const MUSIC_PLAYER: i32 = 0x1E;
    pub const SIGN_UP_POSTER: i32 = 0x20;
}

/// `Sk8::Scene::eSceneType` values `SFXObj_NIS::GetNISType` distinguishes.
pub mod scene {
    pub const NONE: i32 = 0x00;
    pub const RACE_START: i32 = 0x03;
    pub const HIGHLIGHT: i32 = 0x04;
    pub const PHOTO_INTRO: i32 = 0x05;
    pub const PHOTO_OUTRO: i32 = 0x06;
    pub const SPOT_INTRO: i32 = 0x0C;
    pub const SPOT_OUTRO: i32 = 0x0D;
    pub const PRO_CHALLENGE_INTRO: i32 = 0x0F;
    pub const PRO_CHALLENGE_OUTRO: i32 = 0x10;
    pub const STORY: i32 = 0x12;
    pub const TUTORIAL_INTRO: i32 = 0x13;
    pub const TUTORIAL_OUTRO_SUCCESS: i32 = 0x14;
    pub const TUTORIAL_OUTRO_FAILURE: i32 = 0x15;
    pub const FILM: i32 = 0x17;
    pub const ONE_UP: i32 = 0x18;
    pub const BEST_TRICK: i32 = 0x19;
    pub const GOTO: i32 = 0x1A;
}

/// `eChallengeTypes_Unknown`.
pub const CHALLENGE_UNKNOWN: i32 = 0x34;
/// `GM_PartyPlay`.
pub const GM_PARTY_PLAY: i32 = 2;

/// `Sk8::Audio::tSceneInfo` (`mCurrentSceneInfo`): the playing scene's type (0 = none), its
/// `eNISSpecialFlags`, whether it controls a replay, and its length.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SceneInfo {
    pub scene_type: i32,
    pub special_flags: i32,
    pub is_replay_nis: bool,
    pub scene_length: f32,
}

/// One `fe` attribute collection (class 0x5831CB95F3E90598, `skater.vlt`), keyed by its
/// `Attrib::StringToLowerCaseKey`. `FrontEndManager::RegisterAudioEnums` 0x8261b820 registers
/// each as the Apt global `FE_SOUND_<NAME>` holding `name`.
#[derive(Debug, Clone, PartialEq)]
pub struct FeSound {
    /// Layout word 0.
    pub name: String,
    /// Layout word 1.
    pub volume: f32,
    /// Layout word 2: splice in `sk8_menu.bnk`, played when positive.
    pub menu_splice: i32,
    /// Layout word 3: splice in `Sk82_FE.bnk`, played when non-zero.
    pub fe_splice: i32,
    /// Attribute 0x845A1005_2522A2FB: splice in the Hall of Meat bank.
    pub hom_splice: u32,
    /// Attribute 0xA4080FB6_5880E3C2: the `Sk8_moments.csi` logo, `ownedit_logo` for 4 and
    /// `killedit_logo` otherwise.
    pub logo_type: i32,
    /// Attribute 0xBF45D439_FAC71A2E: route through the menu flange voice.
    pub flange: bool,
    /// `HOMExpireTime` in milliseconds.
    pub hom_expire_time: i32,
}

/// What the MAIN objects read from the rest of the game each update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameAudioState {
    /// `ProfileBackEnd::tSettings` sliders (`BackEndManager::mBEModules[7]` +0x78/+0x7C/+0x80).
    pub sfx_volume: f32,
    pub speech_volume: f32,
    pub music_volume: f32,
    /// `AudioSystem::mSk8HUDState`.
    pub hud_state: i32,
    /// The music state `UpdateFEAudioState` 0x82d67948 hands `ChangeFEMusicState` 0x82656f78: the
    /// top front-end state's `gFEStateInfo` row. The music system remaps it and posts
    /// `cMsgFEMusicStateChange` when it changes (`AudioSystem::fe_music_state`).
    pub fe_music_state: i32,
    /// `PauseMan::IsPaused(1)`.
    pub paused: bool,
    /// `FrontEndManager::IsExitingToMainMenu`.
    pub exiting_to_main_menu: bool,
    /// `AudioSystem::mChallengeType`.
    pub challenge_type: i32,
    /// `AudioSystem::mCurrentChallenge`'s host (its layout word 0, an `eSk8Characters`) and its
    /// large-arena flag (`GetCurrentChallengeLargeArena`); none without a current challenge.
    pub challenge_host: Option<i32>,
    pub challenge_large_arena: bool,
    /// `AudioSystem::mGameMode` (`Sk8::BE::CommonBackEnd::GameMode`).
    pub game_mode: i32,
    /// `AudioSystem::mNISFadeAmount`, which `BloomToWhiteTransitionEffect::Update` 0x827d55c0
    /// writes with the post-fx `bloomToWhiteValue`; it keeps its last value.
    pub nis_fade_amount: f32,
    /// `Presentation::IsInReplay` and `mAudioPresentation.mObject.mGameSpeed`; `None` when no
    /// `Presentation` exists.
    pub presentation: Option<(bool, f32)>,
    pub online_game: bool,
    pub mega_challenge: bool,
    /// `AudioSystem::mbInOwnTheSpot`.
    pub in_own_the_spot: bool,
    /// Whether the main challenge is `c_3a_01_street_gvr` (`mbIsGVR`) or
    /// `m_3a_03_besttrick_danny` (`mbIsDannyWayJam`).
    pub gvr: bool,
    pub danny_way_jam: bool,
    /// `GlobalData`'s player bits: the local skater's `PhysOut_Intents::mIsPlayerProvidingInput`
    /// and whether player 1's character is male.
    pub player_providing_input: bool,
    pub player_is_male: bool,
    pub is_fe: i32,
    /// The trick tutorial front-end state's flag at +0x38 when that state exists.
    pub trick_tutorial_state: Option<i32>,
    /// `ProfileBackEnd`'s `tMusicPlayer::mMusicMode` (`music::music_mode`) and
    /// `tProfileData::mNewProfile`.
    pub music_mode: i32,
    pub new_profile: bool,
}

impl Default for GameAudioState {
    /// `AudioSystem`'s constructor values and the default profile settings (`tSettings::Default`).
    fn default() -> Self {
        Self {
            sfx_volume: 1.0,
            speech_volume: 1.0,
            music_volume: 1.0,
            hud_state: hud_state::IDLE,
            fe_music_state: 0,
            paused: false,
            exiting_to_main_menu: false,
            challenge_type: CHALLENGE_UNKNOWN,
            challenge_host: None,
            challenge_large_arena: false,
            game_mode: 0,
            nis_fade_amount: 0.0,
            presentation: Some((false, 1.0)),
            online_game: false,
            mega_challenge: false,
            in_own_the_spot: false,
            gvr: false,
            danny_way_jam: false,
            player_providing_input: false,
            player_is_male: true,
            is_fe: 0,
            trick_tutorial_state: None,
            music_mode: music::music_mode::WORLD,
            new_profile: false,
        }
    }
}

/// What `AudioSystem::Update` reads from the running world (`Presentation`, `Simulation`) each
/// update. Without one the update takes the front-end branch (time controller frame 0x1FFFFFF):
/// only the MAIN state updates and no microphone moves.
#[derive(Debug, Clone, Default)]
pub struct WorldAudioState {
    /// Recorded ChaseMan.GetAudioState8270fe90 / GlobalData count and flags.
    pub chase: (u32, u8),
    pub bodyguard: bool,
    /// `DataSystem::GetCurrentRestoredFrame`.
    pub frame: u32,
    /// `Simulation` +0x10, which `CSTATE_Collision::Attach` keeps to pick the oldest collision.
    pub sim_tick: u32,
    /// `CameraPresentation::GetMatrix(1)`.
    pub camera: CameraMatrix,
    /// `AudioPresentation`: the skaters, `mGameSpeed`, `mIsRewinding`.
    pub player: PlayerFrame,
    /// `VDPresentationState::mOutData.bBail` and `fBailSeverity`.
    pub bail: bool,
    pub bail_severity: f32,
    /// `VDPresentationState::mOutData.bOutOfBounds` and `fOutOfBoundsAmount`, which
    /// `VDReader::ReadData` 0x827fdd78 sets on the frames `VDWriter::WriteData` recorded an amount.
    pub out_of_bounds: bool,
    pub out_of_bounds_amount: f32,
    /// `VDPresentationState::mOutData.bFadeInfo` and `sFadeInfo.mNormalizedFadeValue`: the
    /// `cMsgFadeAmount` `VDWriter::WriteData` 0x827ff8c8 recorded for the frame, if any.
    pub fade_info: Option<f32>,
    /// AudioPresentation GlobalData bit 22: bail in progress AND effect enabled.
    pub hom_bail: bool,
    /// `AudioPresentation::GlobalData` `timesOneFive`, `timesTwo`, `timesThree`.
    pub times: [bool; 3],
    /// `GlobalData.localRaceRanking`: `cChallengeRaceManager::GetPlayerRank` of the local skater's
    /// player (`DoSimSubmit` 0x827fb2f8), -1 without a local skater.
    pub local_race_ranking: i32,
    /// `AudioSystem::GetReverbKey` 0x824f1df8: the additional location's reverb key while one is
    /// loaded (`SetAdditionalLocationKey`), else `GlobalData.reverbKey`, which
    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8 reads from `WorldPainter` layer 10 at the local
    /// skater's x and z; 0 keeps the current reverb.
    pub reverb_key: u64,
    /// `GlobalData.emitterKey`, which `DoSimSubmit` reads from `WorldPainter` layer 8
    /// (`eAudio_Emitters`) at the same point: the location's `aud_wp_emitters` collection of
    /// random emitters; 0 is none.
    pub emitter_key: u64,
    /// `AudioSystem::GetAmbienceKey` 0x824f1cb8: the additional location's ambience key while one
    /// is loaded, else `GlobalData.ambienceKey`, which `DoSimSubmit` reads from `WorldPainter`
    /// layer 7 at the same point; 0 is no ambience bed.
    pub ambience_key: u64,
    /// `AudioPresentation` `mVehicleCount`/`mVehicle`: the cars `DoSimSubmit` 0x827fb2f8 found
    /// around the camera, with their presentation matrices.
    pub vehicles: Vec<crate::world::traffic::VehicleData>,
    /// `AudioPresentation` `mPedestrianCount`/`mPedestrian`: the pedestrians `DoSimSubmit` found
    /// around the camera, with their pres pedestrians' matrices.
    pub pedestrians: Vec<crate::world::pedestrian::PedestrianData>,
    /// `AudioPresentation` `mDynamicCount`/`mDynamicObject`: the dynamic objects `DoSimSubmit`
    /// found within 30 m of the local skater, with their presentation bounds centres.
    pub dynamic_objects: Vec<crate::world::dynamic_object::DynamicData>,
    /// `AudioPresentation` `mSpacialInstanceCount`/`mSpacialInstance`: the world instances
    /// `DoSimSubmit` found within 10 m of the local skater, with their whoosh keys.
    pub spacial_instances: Vec<crate::world::object_instance::SpacialInstanceData>,
    /// The files the DYNAMICOBJECT swap banks can load, by the paths `SFXObj_Moving` asks for.
    pub dmo_swap_files: Arc<crate::world::dynamic_object::DmoSwapFiles>,
}

/// `Sk8::Audio::tFEHudSplice`.
#[derive(Debug, Clone, Copy)]
struct FeHudSplice {
    key: u64,
    bank: i32,
    splice: i32,
    active: Option<SpliceId>,
    volume: f32,
    looping: bool,
    play: bool,
    flange: bool,
}

impl Default for FeHudSplice {
    fn default() -> Self {
        Self { key: 0, bank: SPLICE_MENU as i32, splice: 0, active: None, volume: 1.0, looping: false, play: false, flange: false }
    }
}

/// `eAssetStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    Empty,
    Loading,
    Loaded,
    Unloading,
}

/// What `TriggerMissionMusicImp` 0x824fee80 reads of the challenge (`Attrib::Gen::challenges` of
/// its key): the key, its global data's `ChallengeType` (19 without a key) and its
/// `AudioRaceTuning` collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionMusic {
    pub challenge: u64,
    pub challenge_type: i32,
    pub race_tuning: u64,
}

#[derive(Debug, Clone, Copy)]
enum AudioMessage {
    PlayMenuSound { key: u64, looping: bool },
    StopMenuSound { key: u64 },
    /// `cMsgPlayMissionSpeech`.
    PlayMissionSpeech { character: i32, id: i32, additional_parameter: i32 },
    LeaderboardOverall { character: i32 },
    /// `cMsgPlayCameraManCsisEvent` with unset parameters.
    PlayCameraManCsisEvent { function: &'static str },
    /// `cMsgTriggerMissionMusic`.
    TriggerMissionMusic { trigger: i32, music: MissionMusic },
    /// `cMsgPlayMissionSFX`.
    PlayMissionSfx { sfx: i32, mission: MissionMusic },
}

/// `SFXObj_Master`'s fields.
#[derive(Debug, Clone, Copy, Default)]
struct MasterObject {
    io: Option<DMixIoId>,
    master_music_vol: i32,
    master_sfx_vol: i32,
    master_speech_vol: i32,
    idle_count: f32,
    was_in_replay_editor: bool,
    menu_vol: i32,
    fe_sfx_vol: i32,
    pause_speech_slider_vol: i32,
    pause_music_slider_vol: i32,
    killed_logo_vol: i32,
    killed_bloom_vol: i32,
    owned_logo_vol: i32,
    owned_bloom_vol: i32,
    menu_flange_level: i32,
    movie_volume: i32,
    loading_screen_volume: i32,
}

/// `delayIdle`, `startCount` and `endCount` (0x82fd9410-0x82fd9418).
const DELAY_IDLE: bool = true;
const IDLE_START_COUNT: f32 = 0.8;
const IDLE_END_COUNT: f32 = 0.5;

fn clamp_q15(value: i32) -> i32 {
    value.clamp(0, 0x7FFF)
}

fn flag(value: bool) -> i32 {
    if value { 0x7FFF } else { 0 }
}

struct Host<'a> {
    main_objects: &'a [u32],
}

impl MixerHost for Host<'_> {
    fn state_count(&mut self, state: usize) -> i32 {
        STATE_COUNTS.get(state).copied().unwrap_or(0)
    }

    /// `AudioSystem::ConnectDMixIO`: MAIN objects and controls exist as created; the other states'
    /// objects exist for every instance their managers created.
    fn connect_dmixio(&mut self, id: u32) -> bool {
        let state = (id >> 16) & 0xFF;
        if state != 0 {
            return true;
        }
        let sfx = (id >> 4) & 0x7F;
        let instance = (id & 0xFFFF) >> 11;
        let created: &[u32] = match id & 0xE000_0000 {
            0x4000_0000 => self.main_objects,
            0x6000_0000 => &MAIN_SFX_CONTROLS,
            _ => &[],
        };
        instance == 0 && created.contains(&sfx)
    }
}

/// The parts of `Sk8::Audio::AudioSystem` and `AudioCore` the HUD and menu sounds run through.
pub struct AudioSystem {
    pub core: AudioCore,
    pub splicer: SpliceManager,
    pub mixer: DynamicMixer,
    pub state: GameAudioState,
    master_sub_mix: PluginRef,
    global_effects: GlobalEffects,
    /// The `Sk8SpeechSystem` singleton (`CreateAudioSingletons` 0x8272e4c0 constructs it after
    /// `GlobalEffects`).
    speech: Sk8SpeechSystem,
    /// The `MusicSystem` singleton (constructed and initialized after the speech system) and
    /// `AudioCore::GetMusicMasterSubMix`.
    music: MusicSystem,
    music_master_sub_mix: PluginRef,
    flange_voice: Option<VoiceId>,
    /// AudioSystem constructor8250e39c: eSk8Character_Max.
    last_contest_leader: i32,
    fe_sounds: HashMap<u64, FeSound>,
    fe_hud_splices: [FeHudSplice; 10],
    messages: VecDeque<AudioMessage>,
    initialized: bool,
    in_game_data_loaded: bool,
    update_toggle: i32,
    prev_delta_time_1: f32,
    prev_delta_time_2: f32,
    next_hom_time: f32,
    hom_bank: HomBank,
    /// AudioSystem::mIsHallOfMeat, latched by UpdatePresentationData.
    is_hall_of_meat: bool,
    chase_audio: (u32, u8),
    was_in_no_skate_zone: bool,
    /// `HOM_Set_1.bnk` to `HOM_Set_5.bnk` (`sHOMBankNames`), which `UpdateHOMBankStatus` loads.
    hom_bank_files: Vec<Arc<[u8]>>,
    /// `mbPausedForMovie` (+0x529).
    paused_for_movie: bool,
    /// The playing movie's `rw::movie::AudioRenderer_Rwac`.
    movie: Option<MovieAudio>,
    /// `mpKilledIt` and `mpOwnedIt`.
    moments: Moments,
    b321_go: bool,
    b321_go_count: i32,
    master: MasterObject,
    nis_io: Option<DMixIoId>,
    /// `SFXObj_NIS`'s volumes.
    nis_volumes: NisVolumes,
    pause_io: Option<DMixIoId>,
    /// `SFXObj_Pause +0x1C`: set while unpaused; the first paused frame clears it instead of
    /// raising input 2.
    pause_pending: bool,
    /// `SFXObj_Speech`.
    speech_io: Option<DMixIoId>,
    /// `SFXObj_Announcer` and `SFXObj_CameraMan`, and what their `ProcessUpdate` last read.
    announcer_io: Option<DMixIoId>,
    camera_man_io: Option<DMixIoId>,
    announcer_object: SpeakerObject,
    camera_man_object: SpeakerObject,
    /// `mCurrentSceneInfo`.
    current_scene: SceneInfo,
    /// `mNISPost`: the post the scene queued.
    nis_post: i32,
    /// `mbIsPaused` (+0x2D) and `mTimeSinceUnPaused` (+0x544, ms), which `SFXObj_Pause` keeps.
    is_paused: bool,
    time_since_unpaused: i32,
    /// `mFEMusicState` (+0xDF0, `eFEMusicState_Max` from the constructor) and whether
    /// `mCurrentMenu` (+0x618) is `eCurrentMenu_Pause`, which only
    /// `ReceiveFEMusicStateChangeMsg` 0x82508378 writes.
    fe_music_state: i32,
    current_menu_pause: bool,
    /// `mbIsPlayerProvidingInput` (+0x53C), `mAwayTime` (+0x540, ms) and `mIsMale` (+0x62B),
    /// which `UpdatePresentationData` keeps.
    player_providing_input: bool,
    away_time: i32,
    is_male: bool,
    reverb: Reverb,
    bloom: Bloom,
    vu_io: Option<DMixIoId>,
    challenge_io: Option<DMixIoId>,
    pick_photo_screen: bool,
    hom: Hom,
    menu_io: Option<DMixIoId>,
    jitter: Jitter,
    /// The world this update runs in, `None` in the front end.
    pub world: Option<WorldAudioState>,
    /// The `Audio_*` cvars.
    pub cvars: Cvars,
    vault: Vault,
    /// `rw::math::Random`, seeded from the clock as `Sk8AppMan::InitApp` seeds it from `mftb`.
    rng: RwRandom,
    microphones: MicrophoneMan,
    players: Option<PlayerManager>,
    collisions: Option<CollisionManager>,
    environment: Option<EnvironmentManager>,
    traffic: Option<TrafficManager>,
    pedestrians: Option<PedestrianManager>,
    emitters: Option<EmitterManager>,
    /// `mpEmitterSystem`: the emitter volumes, the random emitters and their swap slots.
    emitter_system: Sk8EmitterSystem,
    crowds: Option<CrowdManager>,
    dynamic_objects: Option<DynamicObjectManager>,
    speakers: Option<SpeakerManager>,
    whooshes: Option<WhooshManager>,
    object_instances: Option<ObjectInstanceManager>,
    nis_characters: Option<NisCharacterManager>,
    /// `PlayCollisionSound` calls made while the PLAYER objects update.
    collision_requests: Vec<CollisionInfo>,
    /// `grains.big`, which the skateboards' `LoadDataForSurface` loads from.
    grains: GrainFiles,
    /// Csis and the AEMS banks.
    pub aems: Aems,
    /// `mbIsTimesOneFive`, `mbIsTimesTwo`, `mbIsTimesThree`.
    times: [bool; 3],
    /// `mpUtility`, the `C_foley_utility` `LoadFoleyData` 0x824f0bf0 creates after the foley banks:
    /// its module deals the cloth foley variations.
    utility: Option<ClassObject>,
    /// `mbInOwnTheSpot` (+0x528), which `PlayMissionSFXImp`'s Own the Spot sets and nothing
    /// clears.
    in_own_the_spot: bool,
    /// `mContestId` (+0x620) with its challenge type: the loaded contest, 0 without one.
    contest_id: u64,
    contest_type: i32,
    /// `mContestKey` (+0x98): the challenge whose crowd banks load.
    contest_key: u64,
    /// `mCrowdsLoadStatus` (+0x8C) and `mVehiclesLoadStatus` (+0x90) (`eModuleLoadStatus`).
    crowds_load: i32,
    vehicles_load: i32,
    /// `mCrowdBanksLoaded` (+0x7A5), by `eCrowdSFX` size.
    crowd_banks_loaded: [bool; 9],
    /// `mbPlayArenaOnLoad` (+0x628).
    play_arena_on_load: bool,
    /// `mpCrowdPodInfos` (+0xA0): the spectator groups the last countdown found.
    crowd_pod_infos: [CrowdPodInfo; 30],
    /// `mCurrentEmitterState` (+0x630, `eEmitterStates`), `mbEmittersSetUp` (+0x634) and
    /// `mbNISEmittersSetUp` (+0x635).
    current_emitter_state: u32,
    emitters_set_up: bool,
    nis_emitters_set_up: bool,
    /// `mCondScore` (+0x55C) and `mCondPrevScore` (+0x560): the local skater's announcer score
    /// and the one before it, which `UpdatePresentationData` keeps.
    cond_score: i32,
    cond_prev_score: i32,
}

/// The Csis interfaces (`LoadCsisEventSystem` order) and AEMS banks (`AudioAssetManager` load
/// order) `EnterWorld` loads.
#[derive(Default)]
pub struct AemsFiles {
    pub interfaces: Vec<(String, Vec<u8>)>,
    pub banks: Vec<(String, Arc<[u8]>)>,
    /// The emitter data files and `emitters.big`'s banks (`Sk8EmitterSystem::LoadGameFiles`).
    pub emitters: EmitterFiles,
    /// The banks `AudioAssetManager::AsyncLoadAt` loads by name: the traffic banks of
    /// `vehicle.big` and `emitters2.big`'s `car_alarms.abk` (`LoadVehiclesData`), and the crowd
    /// banks of `crowds.big` (`LoadCrowdsData`).
    pub asset_files: Vec<(String, Arc<[u8]>)>,
}

impl AudioSystem {
    /// `AudioCore::InitializeAudioCore` 0x82a7f094: the mastering voice (`CreateMasteringVoice`
    /// 0x82a7d570: stage 255, `SubMix` "Master", `SampleCapture`, `Dac`, six channels), the music
    /// master voice and the `SpliceManager`; then the `GlobalEffects`, speech and music singletons
    /// (`CreateAudioSingletons`).
    ///
    /// # Panics
    /// When the mastering or music master voice cannot be created (the core starts empty, so they
    /// always can).
    pub fn new() -> Self {
        let mut core = AudioCore::new();
        let master = core
            .create_voice(
                255,
                "Mastering",
                &[
                    PlugInConfig { kind: PlugInKind::SubMix { name: "Master".into() }, output_channels: 6 },
                    PlugInConfig { kind: PlugInKind::SampleCapture, output_channels: 6 },
                    PlugInConfig { kind: PlugInKind::Dac, output_channels: 6 },
                ],
            )
            .expect("mastering voice");
        let master_sub_mix = PluginRef { voice: master, index: 0 };
        let music_master_sub_mix = music::create_music_master_voice(&mut core, master_sub_mix).expect("music master voice");
        let global_effects = GlobalEffects::new(&mut core, master_sub_mix);
        let mut aems = Aems::new(Some(master_sub_mix));
        aems.env_send_sub_mix = Some(global_effects.reverb_sub_mix());
        let speech = Sk8SpeechSystem::new(&mut core, master_sub_mix, global_effects.reverb_sub_mix(), speech::time64_seed());
        let mut rng = RwRandom::new(clock_seed());
        let mut music = MusicSystem::new();
        music.initialize(&mut core, &mut rng);
        Self {
            aems,
            core,
            splicer: SpliceManager::new(),
            mixer: DynamicMixer::new(),
            // Silent until the first update hands it the profile's volumes (PC port change): the
            // game reads SKATER.P after the audio system starts.
            state: GameAudioState { sfx_volume: 0.0, speech_volume: 0.0, music_volume: 0.0, ..GameAudioState::default() },
            master_sub_mix,
            global_effects,
            speech,
            music,
            music_master_sub_mix,
            flange_voice: None,
            last_contest_leader: 100,
            fe_sounds: HashMap::new(),
            fe_hud_splices: [FeHudSplice::default(); 10],
            messages: VecDeque::new(),
            initialized: false,
            in_game_data_loaded: false,
            update_toggle: 0,
            prev_delta_time_1: 0.0,
            prev_delta_time_2: 0.0,
            next_hom_time: 0.0,
            hom_bank: HomBank::default(),
            is_hall_of_meat: false,
            chase_audio: (0, 0),
            was_in_no_skate_zone: false,
            hom_bank_files: Vec::new(),
            paused_for_movie: false,
            movie: None,
            moments: Moments::default(),
            b321_go: false,
            b321_go_count: 0,
            master: MasterObject::default(),
            nis_io: None,
            nis_volumes: NisVolumes::default(),
            pause_io: None,
            pause_pending: true,
            speech_io: None,
            announcer_io: None,
            camera_man_io: None,
            announcer_object: SpeakerObject::default(),
            camera_man_object: SpeakerObject::default(),
            current_scene: SceneInfo::default(),
            nis_post: 0,
            is_paused: false,
            time_since_unpaused: 0,
            fe_music_state: music::fe_state::MAX,
            current_menu_pause: false,
            player_providing_input: false,
            away_time: 0,
            is_male: false,
            reverb: Reverb::new(None),
            bloom: Bloom::new(None),
            vu_io: None,
            challenge_io: None,
            pick_photo_screen: false,
            hom: Hom::new(None),
            menu_io: None,
            jitter: Jitter::new(None),
            in_own_the_spot: false,
            contest_id: 0,
            contest_type: 0,
            contest_key: 0,
            crowds_load: module_status::NO_HEADER,
            vehicles_load: module_status::NO_HEADER,
            crowd_banks_loaded: [false; 9],
            play_arena_on_load: false,
            crowd_pod_infos: [CrowdPodInfo::default(); 30],
            current_emitter_state: 0,
            emitters_set_up: false,
            nis_emitters_set_up: false,
            cond_score: 0,
            cond_prev_score: 0,
            world: None,
            cvars: Cvars::default(),
            times: [false; 3],
            utility: None,
            vault: Vault::default(),
            rng,
            microphones: MicrophoneMan::default(),
            players: None,
            collisions: None,
            environment: None,
            traffic: None,
            pedestrians: None,
            emitters: None,
            emitter_system: Sk8EmitterSystem::new(),
            crowds: None,
            dynamic_objects: None,
            speakers: None,
            whooshes: None,
            object_instances: None,
            nis_characters: None,
            collision_requests: Vec::new(),
            grains: GrainFiles::new(),
        }
    }

    /// The host's files the stream pools read (`rw::core::filesys`) and resident loads use.
    pub fn set_stream_files(&mut self, files: Arc<dyn crate::rwac::stream_pool::StreamFiles>) {
        self.core.stream_pools.set_files(files);
    }

    /// The splicer banks `GetHOMBank` 0x824e6a08 names, in `sHOMBankNames` order
    /// (`main_objects::HOM_BANK_NAMES` in `data\audio\splicercollision.big`).
    pub fn set_hom_banks(&mut self, banks: Vec<Arc<[u8]>>) {
        self.hom_bank_files = banks;
    }

    /// `EnterWorld` 0x82504450: `Initialize` (with `SecondaryLoad`'s speech projects and resident
    /// NIS and post samples), `LoadInGameData` 0x824fdfe8 (the two front-end banks, the menu flange
    /// voice, the splicer banks of `LoadSkateboardData` 0x824ee4b8 and `LoadCollisionData`
    /// 0x824ee348 in `world_banks` as (name, `SPLICE_TYPE`, bytes), the front-end posts,
    /// `mInGameDataLoaded`) and `InitializeMixer` 0x824ed830; then the front end's `EnterFE` and
    /// the music of its boot screens (`MusicSystem::boot`). `vault` holds `skater.vlt`; `grains`
    /// the `.grain` files by lower-case name.
    ///
    /// # Errors
    /// When a bank does not parse.
    pub fn enter_world(
        &mut self,
        vault: Vault,
        fe_bank: Arc<[u8]>,
        menu_bank: Arc<[u8]>,
        world_banks: Vec<(String, usize, Arc<[u8]>)>,
        mix_map: Vec<u8>,
        fe_sounds: HashMap<u64, FeSound>,
        grains: GrainFiles,
        aems_files: AemsFiles,
    ) -> Result<(), String> {
        self.fe_sounds = fe_sounds;
        self.vault = vault;
        self.grains = grains;
        for (name, bytes) in &aems_files.interfaces {
            if !self.aems.subscribe_interface(name, bytes) {
                return Err(format!("{name}: not a Csis interface"));
            }
        }
        // `LoadBigFileHeaders` 0x824e6740: the traffic and crowd banks can load.
        self.emitter_system.assets_mut().add_files(aems_files.asset_files.into_iter().map(|(name, bytes)| (name.to_lowercase(), bytes)));
        self.vehicles_load = module_status::EMPTY;
        self.crowds_load = module_status::EMPTY;
        let mut emitter_files = Some(aems_files.emitters);
        for (name, bytes) in aems_files.banks {
            self.aems.add_module_bank(&mut self.core, &name, bytes).ok_or_else(|| format!("{name}: not an AEMS bank"))?;
            // `Initialize` sets up the emitter system after `emitter_utility.abk`, before
            // `LoadInGameData`'s banks.
            if name.eq_ignore_ascii_case("emitter_utility.abk") {
                if let Some(files) = emitter_files.take() {
                    self.emitter_system.initialize(&mut self.aems, files);
                }
            }
        }
        if let Some(files) = emitter_files {
            self.emitter_system.initialize(&mut self.aems, files);
        }
        self.utility = Some(self.aems.create_object("c_foley_utility", Vec::new()));
        self.speech.set_files(self.core.stream_pools.files());
        // `Initialize`: `mAwayTime` starts at 1, then `SpeechSystem::Initialize`'s stream pool and
        // the context systems.
        self.away_time = 1;
        Sk8SpeechSystem::create_stream_pool(&mut self.core);
        self.speech.create_context_systems(clock_seed(), speech::game_time(self.core.system_time));
        self.speech.load_projects("english");
        self.initialized = true;
        self.splicer.load_bank(fe_bank, SPLICE_FE).map_err(|e| format!("Sk82_FE.bnk: {e:?}"))?;
        self.splicer.load_bank(menu_bank, SPLICE_MENU).map_err(|e| format!("sk8_menu.bnk: {e:?}"))?;
        self.create_menu_flange_voice();
        for (name, slot, bytes) in world_banks {
            self.splicer.load_bank(bytes, slot).map_err(|e| format!("{name}: {e:?}"))?;
        }
        self.speech.load_early_resident_data();
        // `SecondaryLoad` 0x824fe178: the traffic banks load on the next update.
        self.load_vehicles_data(true);
        self.in_game_data_loaded = true;
        self.initialize_mixer(mix_map);
        self.music.find_main_menu_play_list(&self.vault);
        self.with_music(|music, io| music.boot(io));
        Ok(())
    }

    /// `AudioPresentation::FillSkaterData` 0x827f4c48 against this system's vault.
    #[must_use]
    pub fn fill_skater_data(
        &self,
        view: &PhysOutView,
        is_local: bool,
        player_id: i32,
        can_see_player: bool,
        ai_audio: Option<&[u32; 10]>,
    ) -> SkaterData {
        crate::presentation::fill_skater_data(view, &self.vault, is_local, player_id, can_see_player, ai_audio)
    }

    /// `CreateMenuFlangeVoice` 0x824f1ea8: stage 5, `SubMix` "Menu Flange Voice" and two `Send`s;
    /// send 2 goes to the master mix, send 1 to `GlobalEffects`' flange 1 (patch 0x4000).
    fn create_menu_flange_voice(&mut self) {
        let voice = self.core.create_voice(
            5,
            "Menu Flange Voice",
            &[
                PlugInConfig { kind: PlugInKind::SubMix { name: "Menu Flange Voice".into() }, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::Send, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::Send, output_channels: 6 },
            ],
        );
        if let Some(voice) = voice {
            self.core.connect_send(PluginRef { voice, index: 2 }, Some(self.master_sub_mix));
            self.core.connect_send(PluginRef { voice, index: 1 }, Some(self.global_effects.flange_sub_mix(global_effects::FLANGE_PATCH_1)));
            self.flange_voice = Some(voice);
        }
    }

    /// `InitializeMixer`: the MAIN state's objects exist before the map connects to them;
    /// `SFXObj_Reverb::InitSFX` sets up the reverbs and flanges.
    fn initialize_mixer(&mut self, mix_map: Vec<u8>) {
        let mut host = Host { main_objects: &MAIN_SFX_OBJECTS };
        self.mixer.init_map(mix_map, MIX_STATES, &mut host);
        let io = |sfx: u32| self.mixer.map().and_then(|map| map.dmixio(0x4000_0000 | (sfx << 4)));
        self.master.io = io(SFX_MASTER);
        self.nis_io = io(SFX_NIS);
        self.pause_io = io(SFX_PAUSE);
        self.speech_io = io(SFX_SPEECH);
        self.announcer_io = io(SFX_ANNOUNCER);
        self.camera_man_io = io(SFX_CAMERA_MAN);
        self.vu_io = io(SFX_VU);
        self.challenge_io = io(SFX_CHALLENGE);
        self.menu_io = io(SFX_MENU);
        self.reverb = Reverb::new(io(SFX_REVERB));
        self.bloom = Bloom::new(io(SFX_BLOOM));
        self.hom = Hom::new(io(SFX_HOM));
        self.jitter = Jitter::new(io(SFX_JITTER));
        self.music.set_sfx_object(&self.vault, self.mixer.map_mut());
        if let Some(map) = self.mixer.map_mut() {
            // SFXObj_Announcer::InitSFX 0x82528468 and SFXObj_CameraMan::InitSFX 0x82528748 turn
            // their outputs on.
            for io in [self.announcer_io, self.camera_man_io].into_iter().flatten() {
                map.set_output_enabled(io, true);
            }
            self.reverb.init_sfx(&mut self.core, map, &self.vault, &mut self.global_effects);
            self.jitter.init_sfx(&self.vault, map);
        }
        let map = self.mixer.map();
        self.players = Some(PlayerManager::new(&self.vault, map, &self.grains));
        self.collisions = Some(CollisionManager::new(&self.vault, map));
        self.environment = Some(EnvironmentManager::new(&self.vault, map));
        self.traffic = Some(TrafficManager::new(&self.vault, map));
        self.pedestrians = Some(PedestrianManager::new(&self.vault, map));
        self.emitters = Some(EmitterManager::new(&self.vault, map));
        self.crowds = Some(CrowdManager::new(&self.vault, map));
        self.dynamic_objects = Some(DynamicObjectManager::new(&self.vault, map));
        self.speakers = Some(SpeakerManager::new(&self.vault, map));
        self.whooshes = Some(WhooshManager::new(&self.vault, map));
        self.object_instances = Some(ObjectInstanceManager::new(&self.vault, map));
        self.nis_characters = Some(NisCharacterManager::new(&self.vault, map));
    }

    /// `PlayMenuSound` 0x82504dd0: posts `cMsgPlayMenuSound` to the audio event broadcaster.
    pub fn play_menu_sound(&mut self, key: u64, looping: bool) {
        self.messages.push_back(AudioMessage::PlayMenuSound { key, looping });
    }

    /// `StopMenuSound` 0x82504e20.
    pub fn stop_menu_sound(&mut self, key: u64) {
        self.messages.push_back(AudioMessage::StopMenuSound { key });
    }

    /// `FE::GlobalFEPlaySound` 0x826150b0: plays only keys whose `fe` collection names a sound.
    pub fn global_fe_play_sound(&mut self, key: u64, looping: bool) {
        if let Some(sound) = self.fe_sounds.get(&key) {
            if sound.fe_splice != 0 || sound.menu_splice != 0 || sound.hom_splice != 0 || sound.logo_type != 0 {
                self.play_menu_sound(key, looping);
            }
        }
    }

    /// `FE::GlobalFEStopSound` 0x826151a0: the same gate, then `StopMenuSound`.
    pub fn global_fe_stop_sound(&mut self, key: u64) {
        if let Some(sound) = self.fe_sounds.get(&key) {
            if sound.fe_splice != 0 || sound.menu_splice != 0 || sound.hom_splice != 0 || sound.logo_type != 0 {
                self.stop_menu_sound(key);
            }
        }
    }

    pub fn fe_sound(&self, key: u64) -> Option<&FeSound> {
        self.fe_sounds.get(&key)
    }

    /// `PlayNISSfx` 0x82504540, from an `ICEUserChannelAudioEffect` command 2 (`FireEvent`
    /// 0x826d6648): the killed-it and owned-it blooms. Ids from 100 are `PlayLoadScreenData`'s
    /// trick-book load screen samples, which the port has no load screen for.
    pub fn play_nis_sfx(&mut self, id: i32) {
        self.moments.play_nis_sfx(id, self.master.killed_bloom_vol, self.master.owned_bloom_vol, &mut self.aems);
    }

    /// The speech system with the context it plays in.
    fn with_speech<R>(&mut self, f: impl FnOnce(&mut Sk8SpeechSystem, &mut SpeechCtx<'_>) -> R) -> R {
        let reverb_sub_mix = self.global_effects.reverb_sub_mix();
        let Self { core, vault, rng, master_sub_mix, speech, .. } = self;
        let mut ctx = SpeechCtx { core, vault, rng, master_sub_mix: *master_sub_mix, reverb_sub_mix };
        f(speech, &mut ctx)
    }

    /// `QueueNISPost` 0x824f1158 (`IAudioInterface` +0x58): the scene's post (`mNISPost`) queued
    /// as `eSk8Character_SFXPost`'s NIS sample.
    pub fn queue_nis_post(&mut self, id: i32) {
        self.nis_post = id;
        self.with_speech(|speech, ctx| speech.queue_nis_speech(speech::CHARACTER_SFX_POST, id, ctx));
    }

    /// `PlayNISPost` 0x824f11c0 (+0x5C): `PlayNISSpeech` of `mNISPost`.
    pub fn play_nis_post(&mut self) {
        let id = self.nis_post;
        self.with_speech(|speech, ctx| speech.play_nis_speech(speech::CHARACTER_SFX_POST, id, ctx));
    }

    /// `IsPostQueued` 0x824f1228 (+0x60): `IsNISSpeechQueued` of `mNISPost`.
    #[must_use]
    pub fn is_post_queued(&self) -> bool {
        self.speech.is_nis_speech_queued(speech::CHARACTER_SFX_POST, self.nis_post, &self.core)
    }

    /// Speech ids from 5000 are resident NIS samples, which play offline outside Party Play.
    fn plays_nis_sample(&self, id: i32) -> bool {
        id >= 5000 && !self.state.online_game && self.state.game_mode != GM_PARTY_PLAY
    }

    /// `QueueNISSpeech` 0x824f1080 (+0x64): a NIS sample (`Sk8SpeechSystem::QueueNISSpeech`).
    pub fn queue_nis_speech(&mut self, character: i32, id: i32) {
        if self.plays_nis_sample(id) {
            self.with_speech(|speech, ctx| speech.queue_nis_speech(character, id, ctx));
        }
    }

    /// `PlayNISSpeech` 0x8250c6a0 (+0x68): a NIS sample (`Sk8SpeechSystem::PlayNISSpeech`) or,
    /// below id 5000, the line as mission speech from a scene (`PlayMissionSpeech` with
    /// `fromNIS`).
    pub fn play_nis_speech(&mut self, character: i32, id: i32) {
        if id < 5000 {
            self.with_speech(|speech, ctx| speech.play_scene_mission_speech(character, id, ctx));
        } else if self.plays_nis_sample(id) {
            self.with_speech(|speech, ctx| speech.play_nis_speech(character, id, ctx));
        }
    }

    /// `IsSpeechQueued` 0x824f10e8 (+0x6C): a line that is no NIS sample counts as queued.
    #[must_use]
    pub fn is_speech_queued(&self, character: i32, id: i32) -> bool {
        if !self.plays_nis_sample(id) {
            return true;
        }
        self.speech.is_nis_speech_queued(character, id, &self.core)
    }

    /// `NotifyNISPlaybackStart` 0x824fe540 (+0x74, `Scene::Init`): the scene's info, then
    /// `CheckForRoundStart` 0x824f24b0, which sets up a contest's emitters (`mContestId`) and has
    /// none to set up without one.
    pub fn notify_nis_playback_start(&mut self, scene_type: i32, special_flags: i32, is_replay_nis: bool, scene_length: f32) {
        self.current_scene = SceneInfo { scene_type, special_flags, is_replay_nis, scene_length };
        self.check_for_round_start(self.current_scene);
    }

    /// `NotifyNISPlaybackFinish` 0x824fe558 (+0x78, `Scene::Purge`): the music resumes
    /// (`MusicTrigger(ALL_RESUME)`), the speech system finishes the scene's samples
    /// (`FinishNIS`), the scene info clears and the moments' blooms are released.
    pub fn notify_nis_playback_finish(&mut self) {
        self.music.music_trigger(music::trigger::RESUME);
        self.with_speech(|speech, ctx| speech.finish_nis(ctx));
        self.current_scene = SceneInfo::default();
        self.moments.release(&mut self.aems);
    }

    /// `PlayMissionSpeech` 0x82504ed0 (+0x3C, `ChallengeAudioManager::PlaySpeech` 0x82da1b98):
    /// posts `cMsgPlayMissionSpeech`, which the next update's group A handles
    /// (`PlayMissionSpeechMsg` 0x8250d0a8).
    pub fn leaderboard_overall(&mut self, character: i32) {
        self.messages.push_back(AudioMessage::LeaderboardOverall { character });
    }

    pub fn play_mission_speech(&mut self, character: i32, id: i32, additional_parameter: i32) {
        self.messages.push_back(AudioMessage::PlayMissionSpeech { character, id, additional_parameter });
    }

    /// `ChallengeFailedHandler` 0x82505740's `cMsgPlayCameraManCsisEvent` with unset parameters
    /// on the audio broadcaster, which the next update's group A handles
    /// (`CameraManCsisEventHandler` 0x824ff7a0).
    pub fn play_camera_man_csis_event(&mut self, function: &'static str) {
        self.messages.push_back(AudioMessage::PlayCameraManCsisEvent { function });
    }

    /// `TriggerMissionMusic` 0x82504f28 (+0x34, `ChallengeAudioManager::PlayMusic`): posts
    /// `cMsgTriggerMissionMusic`, which the next update's group A hands `TriggerMissionMusicImp`
    /// (`TriggerMissionMusicMsg` 0x82504fb8). Its countdown's `CacheCrowdPodInfos` is the host's
    /// (`cache_crowd_pod_infos`).
    pub fn trigger_mission_music(&mut self, trigger: i32, music: MissionMusic) {
        self.messages.push_back(AudioMessage::TriggerMissionMusic { trigger, music });
    }

    /// `QuitChallenge` 0x824ff478, the `cMsgAbandonChallenge` handler `AddDelayedMessageHandlers`
    /// 0x82508858 puts on the challenge broadcaster.
    pub fn quit_challenge(&mut self) {
        self.music.quit_challenge();
    }

    /// `RestartChallenge` 0x824ff320 (+0x84, `ChallengeAudioManager::RestartChallenge`
    /// 0x82da1c48): a loaded contest (`mContestId`) resets the emitters' contest state;
    /// otherwise a Point-to-Point, Death or Spot race (`mChallengeType`) restarts its music.
    pub fn restart_challenge(&mut self, _retry: bool) {
        if self.contest_id != 0 {
            self.restart_contest();
        } else if matches!(self.state.challenge_type, 0x12..=0x14) {
            self.music.restart_race();
        }
    }

    /// The speech system's messages to the challenge system (`cMsgSpeechStarted`,
    /// `cMsgSpeechEnded`) and the front end (`cMsgCallSpeechStarted`, `cMsgCallSpeechEnded`)
    /// since the last call, in order.
    pub fn take_speech_messages(&mut self) -> Vec<SpeechMessage> {
        self.speech.take_messages()
    }

    /// `AudioCore::GetSystemTime` 0x82a7dae0: the RWAC system's time, which the rendered blocks
    /// advance.
    #[must_use]
    pub fn system_time(&self) -> f64 {
        self.core.system_time
    }

    /// `mCurrentSceneInfo`.
    #[must_use]
    pub fn current_scene(&self) -> SceneInfo {
        self.current_scene
    }

    /// `NotifyPickPhotoScreen` 0x824e6b24 (`SFXObj_Challenge::NotifyPickPhotoScreen` 0x825187e8):
    /// challenge input 5 holds while the game speed is not positive.
    pub fn notify_pick_photo_screen(&mut self) {
        self.pick_photo_screen = true;
    }

    /// `PauseForMovie` 0x824f0a18: the music stream paused (`XMPOverrideBackgroundMusic` has no
    /// user music to override) and `mbPausedForMovie` (NIS input 1).
    pub fn pause_for_movie(&mut self) {
        self.music.pause_for_movie(true, &mut self.core, self.master_sub_mix);
        self.paused_for_movie = true;
    }

    /// `ResumeAfterMovie` 0x824f0a98: after a movie pause, the music stream resumes unless the
    /// music is off.
    pub fn resume_after_movie(&mut self) {
        if self.paused_for_movie {
            self.music.pause_for_movie(false, &mut self.core, self.master_sub_mix);
            self.paused_for_movie = false;
        }
    }

    /// A movie's `AudioRenderer_Rwac` (constructor 0x82b8cc00, `Init` 0x82b8c678), replacing any
    /// other; false when its voice cannot be made.
    pub fn movie_init(&mut self, sample_rate: u32, channels: u8, sound: MovieSound) -> bool {
        self.movie_shutdown();
        self.movie = MovieAudio::new(&mut self.core, self.master_sub_mix, sample_rate, channels, sound);
        self.movie.is_some()
    }

    /// The movie renderer's `Begin` (0x82b8d320).
    pub fn movie_begin(&mut self) {
        if let Some(movie) = self.movie.as_mut() {
            movie.begin(&mut self.core);
        }
    }

    /// The movie renderer's `SetVolume` (0x82b8c988).
    pub fn movie_set_volume(&mut self, volume: f32) {
        if let Some(movie) = self.movie.as_mut() {
            movie.set_volume(&mut self.core, volume);
        }
    }

    /// The movie renderer's `GetRenderingPosition` (0x82b8c868) in milliseconds; none without a
    /// movie.
    #[must_use]
    pub fn movie_rendering_position(&self) -> Option<f32> {
        self.movie.as_ref().map(|movie| movie.rendering_position(&self.core))
    }

    /// The movie renderer's `Stop` and destruction.
    pub fn movie_shutdown(&mut self) {
        if let Some(movie) = self.movie.take() {
            movie.shutdown(&mut self.core);
        }
    }

    /// `GetMovieVolume` 0x824e6808: `mMovieVolume` (master output 8, `SetMovieVolume`) over
    /// 32767.
    #[must_use]
    pub fn movie_volume(&self) -> f32 {
        self.master.movie_volume as f32 * Q15_SCALE
    }

    /// `SFXObj_NIS::GetNISVol` 0x82547e80 for the current scene, which
    /// `Sk8SpeechSystem::UpdateVoiceInfo` reads.
    #[must_use]
    pub fn nis_vol(&self) -> i32 {
        self.nis_volumes.nis_vol(self.nis_type())
    }

    /// `SFXObj_NIS`'s volumes as its last `ProcessUpdate` read them.
    #[must_use]
    pub fn nis_volumes(&self) -> &NisVolumes {
        &self.nis_volumes
    }

    /// The `MusicSystem` singleton: its state, cvars and the song `UpdateChyron` names.
    pub fn music(&mut self) -> &mut MusicSystem {
        &mut self.music
    }

    /// `MusicSystem::SetDefaultMode` 0x824f61e0 (`music::mode`), as `FE::MusicPlayerManager::
    /// SwitchMusicMode` 0x82629fc8 and `RevertMusicMode` 0x8262a088 map the profile's music mode.
    pub fn set_music_default_mode(&mut self, mode: i32) {
        self.with_music(|music, io| music.set_default_mode(mode, io));
    }

    /// `MusicSystem::IPodTrigger` 0x824f6d98 as `FE::MusicPlayerManager` sends it (`Play`
    /// 0x82639b00, `PlayPlaylist` 0x82629cd0, `sMethod_Stop` 0x82637ea0): a `music::ipod_trigger`
    /// for `song` with no playlist and resume node -1.
    pub fn ipod_trigger(&mut self, trigger: i32, song: u32) {
        self.with_music(|music, io| music.ipod_trigger(trigger, song as i32, 0, -1, io));
    }

    /// The profile's current music player playlist (its first `mNumSongs` songs) and
    /// `mCurrentSongIndex`, before the update; `UpdateIPodState` steps through it.
    pub fn set_music_player_playlist(&mut self, songs: &[u32], current_song_index: i32) {
        self.music.set_music_player_playlist(songs, current_song_index);
    }

    /// `mCurrentSongIndex` after the update, for the profile.
    #[must_use]
    pub fn music_player_song_index(&self) -> i32 {
        self.music.music_player_song_index()
    }

    /// `MusicSystem::GetCurrentMusicPlayerSongID` 0x824e9628.
    #[must_use]
    pub fn current_music_player_song_id(&self) -> u32 {
        self.music.current_music_player_song_id()
    }

    /// The new iPod song `MusicSystem::UpdateChyron` 0x825069f0 displayed since the last call.
    pub fn take_chyron(&mut self) -> Option<u32> {
        self.music.chyron.take()
    }

    /// `MusicPlayerCmdTranslator::sMethod_GetDefaultSongList` 0x826381a0: `MusicCore::
    /// GetMusicPlayerSongIDFromIndex` for each index below `GetNumMusicPlayerSongs`.
    #[must_use]
    pub fn music_player_song_ids(&self) -> Vec<u32> {
        (0..self.music.num_music_player_songs()).map(MusicSystem::music_player_song_id_from_index).collect()
    }

    /// `MusicCore::GetSongInfo` 0x82a80480 for the "IPOD" list (`music::music_player_song_info`):
    /// artist, title and album, with `localize` the localizer's string for a string id.
    pub fn music_player_song_info(song: u32, localize: impl FnMut(u32) -> Option<String>) -> (String, String, String) {
        music::music_player_song_info(song, localize)
    }

    /// `PlayMenuSoundImp` 0x8250c738.
    fn play_menu_sound_imp(&mut self, key: u64, looping: bool) {
        let Some(sound) = self.fe_sounds.get(&key).cloned() else { return };
        if sound.fe_splice != 0 {
            self.play_fe_hud_splice(key, SPLICE_FE as i32, sound.fe_splice, sound.volume, looping, sound.flange);
        }
        if sound.menu_splice > 0 {
            self.play_fe_hud_splice(key, SPLICE_MENU as i32, sound.menu_splice, sound.volume, looping, sound.flange);
        }
        if sound.hom_splice != 0 && !(self.next_hom_time > 0.0) && self.hom_bank.status == AssetStatus::Loaded {
            self.play_fe_hud_splice(key, SPLICE_HOM as i32, sound.hom_splice as i32, sound.volume, looping, sound.flange);
            self.next_hom_time = sound.hom_expire_time as f32 * f32::from_bits(0x3a83_126f);
        }
        if sound.logo_type != 0 {
            main_objects::play_logo(sound.logo_type, sound.volume, self.master.killed_logo_vol, self.master.owned_logo_vol, &mut self.aems);
        }
        self.check_menu_key(key);
    }

    /// `StopMenuSoundImp` 0x824f21d0. The Hall of Meat branch compares the menu splice index,
    /// as natively.
    fn stop_menu_sound_imp(&mut self, key: u64) {
        let Some(sound) = self.fe_sounds.get(&key).cloned() else { return };
        for index in 0..self.fe_hud_splices.len() {
            let slot = self.fe_hud_splices[index];
            let matches = (sound.fe_splice != 0 && slot.bank == SPLICE_FE as i32 && slot.splice == sound.fe_splice)
                || (sound.menu_splice != 0 && slot.bank == SPLICE_MENU as i32 && slot.splice == sound.menu_splice)
                || (sound.hom_splice != 0 && slot.bank == SPLICE_HOM as i32 && slot.splice == sound.menu_splice);
            if matches {
                if let Some(splice) = slot.active {
                    self.splicer.release_splice(&mut self.core, splice);
                }
                let slot = &mut self.fe_hud_splices[index];
                slot.active = None;
                slot.play = false;
                slot.looping = false;
                let finished = slot.key;
                self.notify_menu_sound_finished(finished);
            }
        }
    }

    /// The countdown keys `CheckMenuKey` 0x824e7c20 and `NotifyMenuSoundFinished` 0x824e7cc0 watch.
    fn is_321_go_key(key: u64) -> bool {
        matches!(key as u32 as i32, -788_600_351 | -58_664_700 | -562_789_192 | -367_558_467 | -1_008_060_802)
    }

    fn check_menu_key(&mut self, key: u64) {
        if Self::is_321_go_key(key) {
            self.b321_go = true;
            self.b321_go_count = 0;
        }
    }

    fn notify_menu_sound_finished(&mut self, key: u64) {
        if Self::is_321_go_key(key) {
            self.b321_go_count = 60;
        }
    }

    /// `PlayFEHudSplice` 0x824e7b28: needs `mInGameDataLoaded`; takes the first slot that is
    /// neither waiting to play nor playing.
    fn play_fe_hud_splice(&mut self, key: u64, bank: i32, splice: i32, volume: f32, looping: bool, flange: bool) {
        if !self.in_game_data_loaded {
            return;
        }
        if let Some(slot) = self.fe_hud_splices.iter_mut().find(|slot| !slot.play && slot.active.is_none()) {
            *slot = FeHudSplice { key, bank, splice, active: None, volume, looping, play: true, flange };
        }
    }

    fn fe_volume(&self, bank: i32) -> f32 {
        let volume = if bank == SPLICE_MENU as i32 || bank == SPLICE_HOM as i32 { self.master.menu_vol } else { self.master.fe_sfx_vol };
        volume as f32 * Q15_SCALE
    }

    /// `UpdateFEHudSplices` 0x824fea50.
    fn update_fe_hud_splices(&mut self, dt: f32) {
        for index in 0..self.fe_hud_splices.len() {
            let slot = self.fe_hud_splices[index];
            if slot.play {
                self.fe_hud_splices[index].play = false;
                let sub_mix = match self.flange_voice {
                    Some(voice) if slot.flange => PluginRef { voice, index: 0 },
                    _ => self.master_sub_mix,
                };
                let bank = usize::from(slot.bank as i8 as u8);
                let splice = self.splicer.create_splice(&mut self.core, bank, slot.splice, sub_mix);
                self.fe_hud_splices[index].active = splice;
                let update = SpliceUpdateParams { vol: slot.volume * self.fe_volume(slot.bank), pitch: 1.0, az: 0.0, delta: 0.0, spread: 1.0, time_stretch: 1.0 };
                if let Some(splice) = splice {
                    self.splicer.play_splice(&mut self.core, splice, SplicePlayParams::default(), &update);
                }
            } else if let Some(splice) = slot.active {
                if self.splicer.is_playing(splice) {
                    let mut update = SpliceUpdateParams { vol: slot.volume * self.fe_volume(slot.bank), pitch: 1.0, az: 0.0, delta: dt, spread: 1.0, time_stretch: 1.0 };
                    self.splicer.update_splice(&mut self.core, splice, &mut update);
                } else {
                    self.splicer.release_splice(&mut self.core, splice);
                    self.fe_hud_splices[index].active = None;
                    if slot.looping {
                        self.play_fe_hud_splice(slot.key, slot.bank, slot.splice, slot.volume, true, false);
                    } else {
                        self.notify_menu_sound_finished(slot.key);
                    }
                }
            }
        }
        self.update_menu_flange_voice();
    }

    /// `UpdateMenuFlangeVoice` 0x824f2100: the flange send follows `mMenuFlangeLevel`.
    fn update_menu_flange_voice(&mut self) {
        if let Some(voice) = self.flange_voice {
            self.core.set_attribute(PluginRef { voice, index: 1 }, 0, self.master.menu_flange_level as f32 * Q15_SCALE);
        }
    }

    fn set_input(&mut self, io: Option<DMixIoId>, index: usize, value: i32) {
        if let (Some(io), Some(map)) = (io, self.mixer.map_mut()) {
            map.set_input(io, index, value);
        }
    }

    /// `SFXObj_Master::UpdateParams` 0x8252a918.
    fn update_master(&mut self, t: f32) {
        let state = self.state;
        let io = self.master.io;
        let in_replay = state.presentation.is_some_and(|(replay, _)| replay);
        self.set_input(io, 0, flag(in_replay));
        let slider = |value: f32| fctiwz(value * 32767.0);
        self.master.master_music_vol = slider(state.music_volume);
        self.set_input(io, 1, clamp_q15(self.master.master_music_vol));
        self.master.master_sfx_vol = slider(state.sfx_volume);
        self.set_input(io, 2, clamp_q15(self.master.master_sfx_vol));
        self.master.master_speech_vol = slider(state.speech_volume);
        self.set_input(io, 3, clamp_q15(self.master.master_speech_vol));
        let speed = state.presentation.map_or(1.0, |(_, speed)| speed);
        let scaled = speed * 32767.0;
        let floor = if 100.0 - scaled >= 0.0 { 100.0 } else { scaled };
        let slow = fctiwz(if 32767.0 - floor >= 0.0 { floor } else { 32767.0 });
        let mut fast = 0;
        if speed > 1.0 {
            let excess = (speed - 1.0) * 8191.75;
            let excess = if -excess >= 0.0 { 0.0 } else { excess };
            fast = fctiwz(if 32767.0 - excess >= 0.0 { excess } else { 32767.0 });
        }
        self.set_input(io, 4, clamp_q15(slow));
        self.set_input(io, 5, clamp_q15(fast));
        self.set_input(io, 12, flag(speed < 1.0));
        let replay_editor = state.hud_state == hud_state::REPLAY2;
        self.set_input(io, 13, flag(replay_editor));
        if DELAY_IDLE {
            if replay_editor {
                if !self.master.was_in_replay_editor {
                    self.master.idle_count = IDLE_START_COUNT;
                }
            } else if self.master.was_in_replay_editor {
                self.master.idle_count = IDLE_END_COUNT;
            }
            self.master.was_in_replay_editor = replay_editor;
        }
        if self.master.idle_count < 0.0 {
            self.set_input(io, 6, flag(in_replay && speed == 0.0));
        } else {
            self.set_input(io, 6, 0x7FFF);
            self.master.idle_count -= t;
        }
        let music_mode = self.music.current_mode();
        self.set_input(io, 7, flag(music_mode == music::mode::CHASE));
        self.set_input(io, 8, if music_mode == music::mode::CHASE { 0x3FFF } else { 0 });
        self.set_input(io, 9, flag(music_mode == music::mode::CHALLENGE));
        let living_world = self.speech.is_a_living_world_character_speaking();
        self.set_input(io, 11, flag(living_world));
        let main_cast = self.speech.is_a_main_cast_character_speaking();
        self.set_input(io, 10, flag(main_cast));
    }

    /// `SFXObj_Master::ProcessUpdate` 0x8252af30 (`GetVolumeFromMixer` = `SndBase::GetDMixOutput`
    /// 0x82a7dd78 with `DMX_VOL`).
    fn process_master(&mut self) {
        let io = self.master.io;
        let volume = |index: i32| io.and_then(|io| self.mixer.map().map(|map| map.output(io, index, PresetType::Volume))).unwrap_or(0);
        let (menu, fe, killed_logo, killed_bloom, owned_logo, owned_bloom, pause_speech, pause_music, movie, loading, flange) =
            (volume(0), volume(1), volume(4), volume(5), volume(6), volume(7), volume(2), volume(3), volume(8), volume(9), volume(10));
        let master = &mut self.master;
        master.menu_vol = menu;
        master.fe_sfx_vol = fe;
        master.killed_logo_vol = killed_logo;
        master.killed_bloom_vol = killed_bloom;
        master.owned_logo_vol = owned_logo;
        master.owned_bloom_vol = owned_bloom;
        master.pause_speech_slider_vol = pause_speech;
        master.pause_music_slider_vol = pause_music;
        master.movie_volume = movie;
        master.loading_screen_volume = loading;
        master.menu_flange_level = flange;
    }

    /// `SFXObj_NIS::UpdateParams` 0x82547ab8 with `GetNISType` 0x82532b58.
    fn update_nis(&mut self) {
        let io = self.nis_io;
        for index in [0, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
            self.set_input(io, index, 0);
        }
        let slot = match self.nis_type() {
            0 => Some(2),
            1 => Some(3),
            2 => Some(4),
            3 => Some(5),
            4 => Some(6),
            5 => Some(0),
            6 => Some(7),
            7 => Some(8),
            8 => Some(10),
            9 => Some(11),
            _ => None,
        };
        if let Some(slot) = slot {
            self.set_input(io, slot, 0x7FFF);
        }
        self.set_input(io, 1, flag(self.paused_for_movie));
        self.set_input(io, 9, flag(self.is_loading()));
    }

    /// `SFXObj_NIS::GetNISType`: the `eNISType` for the current scene (10 when there is none).
    fn nis_type(&self) -> i32 {
        let film = [7, 0x27, 0x28, 0x29, 0x2E, 0x30];
        let best_trick = [8, 0xF, 2];
        let challenge = self.state.challenge_type;
        let replay = self.current_scene.is_replay_nis;
        match self.current_scene.scene_type {
            scene::HIGHLIGHT if film.contains(&challenge) => 7,
            scene::HIGHLIGHT if best_trick.contains(&challenge) => 6,
            scene::HIGHLIGHT | scene::PRO_CHALLENGE_OUTRO => 1,
            scene::RACE_START => 2,
            scene::BEST_TRICK => if replay { 6 } else { 3 },
            scene::FILM => if replay { 7 } else { 3 },
            scene::ONE_UP => if replay { 1 } else { 3 },
            scene::SPOT_INTRO | scene::PHOTO_INTRO | scene::GOTO => 3,
            scene::STORY | scene::PRO_CHALLENGE_INTRO => 5,
            scene::PHOTO_OUTRO => 0,
            scene::SPOT_OUTRO => 4,
            scene::TUTORIAL_INTRO | scene::TUTORIAL_OUTRO_FAILURE => 8,
            scene::TUTORIAL_OUTRO_SUCCESS => 9,
            _ => 10,
        }
    }

    /// `AudioSystem::IsLoading` 0x82504498.
    fn is_loading(&self) -> bool {
        if self.state.hud_state == hud_state::TRICK_TUTORIAL {
            self.state.trick_tutorial_state == Some(1)
        } else {
            self.state.hud_state == 0x51
        }
    }

    /// `SFXObj_Pause::UpdateParams` 0x82532f48. `AudioCore::StartSampleCapture` (`+0x50`) never
    /// runs. It also keeps `mbIsPaused` (the simulation paused or loading, with a `Presentation`)
    /// and zeroes `mTimeSinceUnPaused` as the pause ends.
    fn update_pause(&mut self) {
        let io = self.pause_io;
        let state = self.state;
        let is_paused = state.presentation.is_some() && (state.paused || self.is_loading());
        if self.is_paused && !is_paused {
            self.time_since_unpaused = 0;
        }
        self.is_paused = is_paused;
        let in_replay = state.presentation.is_some_and(|(replay, _)| replay);
        let paused = state.presentation.is_some()
            && state.paused
            && !in_replay
            && self.current_scene.scene_type == 0
            && !self.is_loading();
        let mut inputs = [0, flag(state.hud_state == hud_state::SIGN_UP_POSTER), 0];
        if paused {
            if self.current_menu_pause {
                inputs[0] = 0x7FFF;
            } else if self.pause_pending {
                self.pause_pending = false;
            } else {
                inputs[2] = 0x7FFF;
            }
        } else {
            self.pause_pending = true;
        }
        if state.exiting_to_main_menu {
            inputs[0] = 0x7FFF;
        }
        for (index, value) in inputs.into_iter().enumerate() {
            self.set_input(io, index, value);
        }
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            static LAST: std::sync::Mutex<Option<[i32; 9]>> = std::sync::Mutex::new(None);
            let traced = [
                i32::from(state.paused),
                i32::from(self.is_paused),
                inputs[0],
                inputs[1],
                inputs[2],
                self.current_scene.scene_type,
                self.nis_type(),
                state.hud_state,
                self.fe_music_state,
            ];
            if let Ok(mut last) = LAST.lock()
                && *last != Some(traced)
            {
                *last = Some(traced);
                let [sim, paused, fade, poster, drop, scene, nis, hud, music] = traced;
                eprintln!("{elapsed:9.3} audio pause sim {sim} paused {paused} inputs {fade} {poster} {drop} scene {scene} nis {nis} hud {hud} music {music}");
            }
        }
    }

    /// `SFXObj_Speech::UpdateParams` 0x825333a0.
    fn update_speech_object(&mut self) {
        if let Some(map) = self.mixer.map_mut() {
            speech::update_speech_object(map, self.speech_io, Some(&self.speech));
        }
    }

    /// `SFXObj_VU::UpdateParams` 0x8253ba48: `AudioCore::GetMusicVU`/`GetSpeechVU` read 0 with no
    /// music or speech voice, and both rate-limited values start at 0, so both inputs stay 0.
    fn update_vu(&mut self) {
        let io = self.vu_io;
        self.set_input(io, 0, 0);
        self.set_input(io, 1, 0);
    }

    /// `SFXObj_Challenge::UpdateParams` 0x8253c128.
    fn update_challenge(&mut self) {
        let io = self.challenge_io;
        let challenge = self.state.challenge_type;
        self.set_input(io, 1, flag(matches!(challenge, 8 | 0xF | 2)));
        self.set_input(io, 0, flag(matches!(challenge, 7 | 0x27 | 0x28 | 0x29 | 0x2E | 0x30)));
        self.set_input(io, 4, flag(matches!(challenge, 6 | 0x25 | 0x26 | 0x2F)));
        self.set_input(io, 2, flag(self.state.online_game));
        self.set_input(io, 3, flag(challenge != CHALLENGE_UNKNOWN && self.b321_go));
        let mut photo = 0;
        if self.pick_photo_screen {
            if let Some((_, speed)) = self.state.presentation {
                if speed <= 0.0 {
                    photo = 0x7FFF;
                } else {
                    self.pick_photo_screen = false;
                }
            }
        }
        self.set_input(io, 5, photo);
        self.set_input(io, 6, flag(self.state.mega_challenge));
    }

    /// `SFXObj_Menu::UpdateParams` 0x8254f1e0.
    fn update_menu(&mut self) {
        let io = self.menu_io;
        let hud = self.state.hud_state;
        let tutorial = hud == hud_state::TRICK_TUTORIAL && self.state.trick_tutorial_state.is_some_and(|value| value != 1);
        self.set_input(io, 0, flag(tutorial));
        self.set_input(io, 1, flag(hud == hud_state::CAC));
        self.set_input(io, 3, flag(hud == hud_state::MUSIC_PLAYER));
        self.set_input(io, 2, flag(self.state.is_fe > 0));
    }

    /// `CSTATEMGR_Base::UpdateParams` for the MAIN state (attached in `CSTATEMGR_Main::Initialize`):
    /// its control, then the objects in creation order.
    fn main_update_params(&mut self, t: f32) {
        let music_volume = self.emitter_system.current_music_volume().copied();
        self.update_speaker_object(self.announcer_io, speech::CHANNEL_ANNOUNCER);
        let Self { music, mixer, vault, microphones, players, times, .. } = self;
        if let (Some(map), Some(players)) = (mixer.map_mut(), players.as_mut()) {
            music.update_object(t, music_volume, *times, vault, microphones, &mut players.shared, map);
        }
        self.update_master(t);
        self.update_speaker_object(self.camera_man_io, speech::CHANNEL_CAMERA_MAN);
        let reverb_key = self.world.as_ref().map_or(0, |world| world.reverb_key);
        let local_board = self.world.as_ref().and_then(|world| world.player.skaters.iter().find(|skater| skater.local)).map(|skater| (skater.board_pos, skater.board_look));
        if let Some(map) = self.mixer.map_mut() {
            let presentation = self.state.presentation.is_some();
            self.reverb.update_params(t, presentation, reverb_key, local_board, &self.emitter_system, &mut self.core, map, &self.vault, &mut self.global_effects);
        }
        self.update_nis();
        self.update_pause();
        self.update_speech_object();
        self.update_bloom();
        self.update_vu();
        self.update_challenge();
        self.update_hom();
        self.update_menu();
        if let Some(map) = self.mixer.map_mut() {
            self.jitter.update_params(&mut self.rng, map);
        }
    }

    /// `SFXObj_Announcer::UpdateParams` 0x825284f0 / `SFXObj_CameraMan::UpdateParams` 0x825287d0:
    /// input 0 while the voices instance's stream 0 actually plays the object's channel.
    fn update_speaker_object(&mut self, io: Option<DMixIoId>, channel: i32) {
        let speaking = self.speech.voices_speaking_on_channel(channel);
        self.set_input(io, 0, flag(speaking));
    }

    /// `SFXObj_Announcer::ProcessUpdate` 0x82515218 (also `SFXObj_CameraMan`'s): azimuth 0,
    /// pitch 1, volume 2, the low pass (frequency 3) and high pass (frequency 4), and the wet
    /// volume 5.
    fn speaker_object(map: &crate::nicotine::MixMap, io: Option<DMixIoId>) -> SpeakerObject {
        let Some(io) = io else { return SpeakerObject::default() };
        SpeakerObject {
            azimuth: map.output(io, 0, PresetType::Azimuth),
            pitch: map.output(io, 1, PresetType::Pitch),
            vol: map.output(io, 2, PresetType::Volume),
            high_pass: map.output(io, 4, PresetType::Frequency),
            low_pass: map.output(io, 3, PresetType::Frequency),
            wet_vol: map.output(io, 5, PresetType::Volume),
        }
    }

    /// `SFXObj_Bloom::UpdateParams` 0x82549fa0.
    fn update_bloom(&mut self) {
        let fade_info = self.world.as_ref().and_then(|world| world.fade_info);
        let (scene_type, nis_fade_amount) = (self.current_scene.scene_type, self.state.nis_fade_amount);
        if let Some(map) = self.mixer.map_mut() {
            self.bloom.update_params(scene_type, fade_info, nis_fade_amount, map, &mut self.aems);
        }
    }

    /// `SFXObj_HOM::UpdateParams` 0x8253c598.
    fn update_hom(&mut self) {
        let hall_of_meat = self.is_hall_of_meat;
        let (hud_state, special_flags) = (self.state.hud_state, self.current_scene.special_flags);
        if let Some(map) = self.mixer.map_mut() {
            self.hom.update_params(hall_of_meat, hud_state, special_flags, map);
        }
    }

    /// `UpdateHOMBankStatus` 0x824f1650 over the host's `HOM_Set_<n>.bnk` files.
    fn update_hom_bank_status(&mut self) {
        let hall_of_meat = self.is_hall_of_meat;
        let Self { hom_bank, hom_bank_files, splicer, rng, .. } = self;
        hom_bank.update(hall_of_meat, rng, |index| {
            hom_bank_files.get(index).is_some_and(|bytes| splicer.load_bank(Arc::clone(bytes), SPLICE_HOM).is_ok())
        });
    }

    /// `MicrophoneMan::UpdateCameras` 0x824f2728 (`AudioPresentation::mLocal` is the local
    /// skater's `SkaterData`).
    fn update_cameras(&mut self, t: f32) {
        let Some(world) = &self.world else { return };
        let local = world.player.skaters.iter().find(|skater| skater.local).map(|skater| (skater.skater_pos, skater.skater_look, skater.skater_vel));
        self.microphones.update_cameras(t, world.frame, world.camera, local);
    }

    /// The world managers' `UpdateParams` in state order (1-12). `PlayCollisionSound` attaches its
    /// collision states while the PLAYER objects update. Without a world only the ambience has
    /// anything to update.
    fn world_update_params(&mut self, t: f32) {
        let loading = self.is_loading();
        let crowd_start = self.crowds.as_ref().and_then(CrowdManager::pending_start);
        let crowd_start = crowd_start.map(|(kind, challenge)| (kind, challenge, self.crowds_load_status() >= module_status::LOADED, self.local_skater_position()));
        let in_own_the_spot = self.in_own_the_spot || self.state.in_own_the_spot;
        let cond_score_delta = self.cond_score - self.cond_prev_score;
        let Self { core, splicer, mixer, master_sub_mix, music_master_sub_mix, global_effects, vault, rng, microphones, players, collisions, collision_requests, cvars, world, aems, state, times, environment, traffic, pedestrians, emitters, emitter_system, crowds, crowd_pod_infos, crowd_banks_loaded, dynamic_objects, speakers, whooshes, object_instances, nis_characters, speech, .. } = self;
        let crowd_env = match world.as_ref() {
            Some(world) => Some(CrowdEnv { skaters: &world.player.skaters, emitter_system: &*emitter_system, pods: &*crowd_pod_infos, banks_loaded: &*crowd_banks_loaded }),
            None => None,
        };
        let Some(map) = mixer.map_mut() else { return };
        let (bail, bail_severity) = world.as_ref().map_or((false, 0.0), |world| (world.bail, world.bail_severity));
        let hall_of_meat = self.is_hall_of_meat;
        let (out_of_bounds, out_of_bounds_amount) = world.as_ref().map_or((false, 0.0), |world| (world.out_of_bounds, world.out_of_bounds_amount));
        let mut ctx = SfxCtx {
            core,
            aems,
            splicer,
            map,
            vault,
            rng,
            master_sub_mix: *master_sub_mix,
            global_effects,
            cvars: *cvars,
            delta_time: t,
            local: false,
            bail,
            bail_severity,
            out_of_bounds,
            out_of_bounds_amount,
            hall_of_meat,
            times: *times,
            in_own_the_spot,
            challenge_type: state.challenge_type,
            camera_position: microphones.microphones[0].cur_position,
            npc_wheel_type: WHEEL_SOFT,
            collisions: collision_requests,
            crowds: crowds.as_mut(),
            crowd_env,
            cond_score_delta,
        };
        if let (Some(players), Some(collisions), Some(world)) = (players.as_mut(), collisions.as_mut(), world.as_ref()) {
            let inputs = PhysicsInputs {
                local: false,
                game_speed: world.player.game_speed,
                is_rewinding: world.player.is_rewinding,
                local_velocity: world.player.skaters.iter().find(|skater| skater.local).map(|skater| skater.skater_vel),
                visual_bail: world.bail,
                npc_wheel_type: WHEEL_SOFT,
                grabbed_object_key: dynamic_objects.as_ref().map_or(0, DynamicObjectManager::grabbed_object_key),
            };
            players.update_params(t, &world.player, microphones, inputs, &mut ctx);
            for info in std::mem::take(ctx.collisions) {
                collisions.play(info, world.sim_tick, &mut ctx);
            }
        }
        if let Some(environment) = environment {
            environment.update_params(t, world.as_ref(), state.is_fe, loading, &mut ctx);
        }
        let (Some(players), Some(collisions), Some(world)) = (players.as_mut(), collisions.as_mut(), world.as_ref()) else { return };
        collisions.update_params(t, microphones, &mut players.shared, ctx.map);
        if let Some(traffic) = traffic {
            traffic.update_params(t, world, microphones, &mut players.shared, whooshes.as_mut(), &mut ctx);
        }
        if let Some(pedestrians) = pedestrians {
            pedestrians.update_params(t, world, microphones, whooshes.as_mut(), Some(&mut *speech), &mut ctx);
        }
        if let Some(emitters) = emitters {
            emitters.update_params(t, microphones, &mut players.shared, ctx.map);
        }
        // `CSTATEMGR_Crowd::UpdateParams` 0x825588d8: a start that waited for the crowd banks,
        // then the states.
        if let Some(crowds) = ctx.crowds.take() {
            if let Some((kind, challenge, loaded, position)) = crowd_start {
                crowds.start_crowds(kind, challenge, loaded, position, crowd_banks_loaded, &mut ctx);
            }
            crowds.update_params(t, microphones, &mut players.shared, &mut ctx);
        }
        if let Some(dynamic_objects) = dynamic_objects {
            dynamic_objects.update_params(t, world, microphones, &mut players.shared, whooshes.as_mut(), &mut ctx);
            for info in std::mem::take(ctx.collisions) {
                collisions.play(info, world.sim_tick, &mut ctx);
            }
        }
        if let Some(speakers) = speakers {
            speakers.update_params(t, emitter_system, microphones, &mut players.shared, speech, *music_master_sub_mix, &mut ctx);
        }
        if let Some(whooshes) = whooshes.as_mut() {
            whooshes.update_params(t, world, microphones, &mut players.shared, &mut ctx);
        }
        if let Some(object_instances) = object_instances {
            object_instances.update_params(t, world, microphones, &mut players.shared, whooshes.as_mut(), &mut ctx);
        }
        if let Some(nis_characters) = nis_characters {
            nis_characters.update_params(t, world, microphones, &mut players.shared, &mut ctx);
        }
    }

    /// `UpdatePresentationData` 0x82504848, which runs after the state managers' `UpdateParams`:
    /// the multiplier flags from `GlobalData`, and with a `Presentation` the player bits and
    /// `mAwayTime`: 0 while the player gives input, else counting the update's whole
    /// milliseconds while unpaused, not loading and in game (`mFEMusicState`, which also rules
    /// out `mbInReplay`); `mTimeSinceUnPaused` always counts. Chase state is
    /// latched here after the world state managers, as in the native update.
    fn update_presentation_data(&mut self, t: f32) {
        if let Some(world) = &self.world {
            if self.chase_audio != world.chase && std::env::var_os("SKATE2_TRACE_SECURITY_CAPTURE").is_some() {
                eprintln!("security audio: frame={} count={} flags={:#04x}", world.frame, world.chase.0, world.chase.1);
            }
            self.chase_audio = world.chase;
            let in_zone = self.chase_audio.1 & 0x80 != 0;
            if self.was_in_no_skate_zone && !in_zone {
                self.with_speech(|speech, ctx| speech.left_no_skate_zone(ctx));
            }
            self.was_in_no_skate_zone = in_zone;
        }
        // 82504848: the bank must be resident; latch once the authored speed
        // threshold is reached, until the recorded GlobalData flag clears.
        let eligible = self.hom_bank.status == AssetStatus::Loaded
            && self.world.as_ref().is_some_and(|world| world.hom_bail)
            && !(self.cvars.no_hom_in_replay && self.state.presentation.is_some_and(|p| p.0));
        let minimum = self.vault.f32(
            crate::vault::Collection::new(crate::vault::class::AUD_GENERAL, 0x3404_9abc_e23c_10ac),
            0x4bd3_e0cc_ea90_be26, // aud_general::HOM_MinGameSpeed 824e3360
        );
        let was_hom = self.is_hall_of_meat;
        self.is_hall_of_meat = eligible && (self.is_hall_of_meat
            || self.world.as_ref().is_some_and(|world| world.player.game_speed <= minimum));
        if was_hom != self.is_hall_of_meat && std::env::var_os("SKATE2_TRACE_REPLAY_AUDIO").is_some() {
            eprintln!("audio HoM={} bank={:?} minimum_speed={minimum}",self.is_hall_of_meat,self.hom_bank.status);
        }
        if let Some(world) = &self.world {
            self.times = world.times;
            // `mCondPrevScore` takes `mCondScore`, which takes the local skater's announcer score.
            if let Some(local) = world.player.skaters.iter().find(|skater| skater.local) {
                self.cond_prev_score = self.cond_score;
                self.cond_score = local.announcer_score as i32;
            }
        }
        let elapsed = ((t * -1000.0) as i32).wrapping_neg();
        if self.state.presentation.is_some() {
            self.player_providing_input = self.state.player_providing_input;
            self.is_male = self.state.player_is_male;
            if self.player_providing_input {
                self.away_time = 0;
            } else if !self.is_paused && !self.is_loading() && self.fe_music_state == music::fe_state::IN_GAME {
                self.away_time = self.away_time.wrapping_add(elapsed);
            }
        }
        self.time_since_unpaused = self.time_since_unpaused.wrapping_add(elapsed);
    }

    /// The state managers' `ProcessUpdate` in state order: MAIN (`SFXObj_Announcer`,
    /// `SFXObj_Music`, `SFXObj_Master`, `SFXObj_CameraMan`, `SFXObj_Reverb`, `SFXObj_NIS`,
    /// `SFXObj_Bloom`), then the world managers 1-12.
    fn process_update(&mut self) {
        if let Some(map) = self.mixer.map() {
            self.announcer_object = Self::speaker_object(map, self.announcer_io);
            self.music.process_object(map);
        }
        self.process_master();
        if let Some(map) = self.mixer.map() {
            self.camera_man_object = Self::speaker_object(map, self.camera_man_io);
        }
        let in_own_the_spot = self.in_own_the_spot || self.state.in_own_the_spot;
        let cond_score_delta = self.cond_score - self.cond_prev_score;
        let Self { core, splicer, mixer, master_sub_mix, global_effects, vault, rng, microphones, players, collisions, collision_requests, cvars, world, aems, state, times, reverb, nis_io, nis_volumes, bloom, environment, traffic, pedestrians, emitters, crowds, dynamic_objects, speakers, whooshes, object_instances, nis_characters, speech, .. } = self;
        let Some(map) = mixer.map_mut() else { return };
        reverb.process_update(core, map, global_effects);
        *nis_volumes = NisVolumes::read(map, *nis_io);
        bloom.process_update(map, aems);
        let (bail, bail_severity) = world.as_ref().map_or((false, 0.0), |world| (world.bail, world.bail_severity));
        let hall_of_meat = self.is_hall_of_meat;
        let (out_of_bounds, out_of_bounds_amount) = world.as_ref().map_or((false, 0.0), |world| (world.out_of_bounds, world.out_of_bounds_amount));
        let mut ctx = SfxCtx {
            core,
            aems,
            splicer,
            map,
            vault,
            rng,
            master_sub_mix: *master_sub_mix,
            global_effects,
            cvars: *cvars,
            delta_time: 0.0,
            local: false,
            bail,
            bail_severity,
            out_of_bounds,
            out_of_bounds_amount,
            hall_of_meat,
            times: *times,
            in_own_the_spot,
            challenge_type: state.challenge_type,
            camera_position: microphones.microphones[0].cur_position,
            npc_wheel_type: players.as_ref().map_or(WHEEL_SOFT, PlayerManager::npc_wheel_type),
            collisions: collision_requests,
            crowds: None,
            crowd_env: None,
            cond_score_delta,
        };
        if let Some(players) = players {
            players.process_update(&mut ctx);
        }
        if let Some(environment) = environment {
            environment.process_update(&mut ctx);
        }
        if let Some(collisions) = collisions {
            collisions.process_update(&mut ctx);
        }
        if let Some(traffic) = traffic {
            traffic.process_update(microphones, &mut ctx);
        }
        if let Some(pedestrians) = pedestrians {
            pedestrians.process_update(Some(speech), &mut ctx);
        }
        if let Some(emitters) = emitters {
            emitters.process_update(&mut ctx);
        }
        if let Some(crowds) = crowds {
            crowds.process_update(microphones, &mut ctx);
        }
        if let Some(dynamic_objects) = dynamic_objects {
            dynamic_objects.process_update(&mut ctx);
        }
        if let Some(speakers) = speakers {
            speakers.process_update(state.challenge_large_arena, &mut ctx);
        }
        if let Some(whooshes) = whooshes {
            whooshes.process_update(&mut ctx);
        }
        if let Some(object_instances) = object_instances {
            object_instances.process_update(&mut ctx);
        }
        if let Some(nis_characters) = nis_characters {
            nis_characters.process_update(&mut ctx);
        }
    }

    /// `AudioSystem::Update` 0x82507b88. Group A (asset loads, the event dispatch, the splice
    /// manager, the state managers' `UpdateParams`, presentation data, the HUD splices) and group B
    /// (`ProcessMixMap`, the state managers' `ProcessUpdate`) alternate per frame with their
    /// accumulated time; both run every frame below 50 fps (`kCompensateForSlowFrameRate`).
    pub fn update(&mut self, dt: f32) {
        if !self.initialized {
            return;
        }
        let group_a_time = self.prev_delta_time_1 + dt;
        let group_b_time = self.prev_delta_time_2 + dt;
        let (group_a, group_b) = if f64::from(dt) > 0.02 {
            self.update_toggle = 0;
            self.prev_delta_time_1 = 0.0;
            self.prev_delta_time_2 = 0.0;
            (true, true)
        } else if self.update_toggle != 0 {
            self.prev_delta_time_1 = dt;
            self.prev_delta_time_2 = 0.0;
            self.update_toggle = (self.update_toggle - 1) & 1;
            (false, true)
        } else {
            self.prev_delta_time_2 = dt;
            self.prev_delta_time_1 = 0.0;
            self.update_toggle = 1;
            (true, false)
        };
        if group_a {
            self.emitter_system.update_assets(&mut self.aems, &mut self.core);
            while let Some(message) = self.messages.pop_front() {
                match message {
                    AudioMessage::PlayMenuSound { key, looping } => self.play_menu_sound_imp(key, looping),
                    AudioMessage::StopMenuSound { key } => self.stop_menu_sound_imp(key),
                    AudioMessage::PlayMissionSpeech { character, id, additional_parameter } => {
                        self.with_speech(|speech, ctx| speech.play_mission_speech_msg(character, id, additional_parameter, ctx));
                    }
                    AudioMessage::LeaderboardOverall { character } => {
                        let unchanged = self.last_contest_leader == character;
                        self.with_speech(|speech, ctx| speech.leaderboard_overall(character, unchanged, ctx));
                        self.last_contest_leader = character;
                    }
                    AudioMessage::PlayCameraManCsisEvent { function } => {
                        self.with_speech(|speech, ctx| speech.camera_man_csis_event_msg(function, ctx));
                    }
                    AudioMessage::TriggerMissionMusic { trigger, music } => self.trigger_mission_music_imp(trigger, music),
                    AudioMessage::PlayMissionSfx { sfx, mission } => self.play_mission_sfx_imp(sfx, mission),
                }
            }
            self.update_music(group_a_time);
            self.update_speech();
            self.update_contexts();
            self.splicer.update(&mut self.core);
            // No additional location loads (`mAdditionalDataKeyToLoad`/`ToUnload` stay 0), so
            // only `Sk8EmitterSystem::UpdateDLCAsyncLoad` 0x824f9098 runs.
            self.emitter_system.update_dlc_async_load();
        }
        if self.world.is_none() {
            if group_a {
                self.main_update_params(group_a_time);
                self.world_update_params(group_a_time);
            }
            if group_b {
                self.mixer.process_mix_map(group_b_time);
                self.process_update();
                self.update_fe_hud_splices(group_b_time);
            }
            return;
        }
        if group_a {
            self.update_cameras(group_a_time);
            self.main_update_params(group_a_time);
            self.world_update_params(group_a_time);
        }
        if group_b {
            self.mixer.process_mix_map(group_b_time);
            self.process_update();
        }
        if group_a {
            self.update_emitter_system(group_a_time);
            self.update_waiting_loads();
            self.update_presentation_data(group_a_time);
            self.update_fe_hud_splices(group_a_time);
            self.update_arena_on_load();
            self.update_hom_bank_status();
            if self.next_hom_time > 0.0 {
                self.next_hom_time -= group_a_time;
            }
            if self.b321_go_count > 0 {
                self.b321_go_count -= 1;
                if self.b321_go_count == 0 {
                    self.b321_go = false;
                }
            }
            let chase = self.music.target_mode() == music::mode::CHASE;
            self.emitter_system.set_allow_new_emitter_volumes(emitter_system::allow_new_emitter_volumes(self.state.challenge_type) && !chase);
        }
    }

    /// `Sk8SpeechSystem::Update` 0x8250d438 (group A, after `MusicSystem::Update`): the simulation
    /// tick, a live (not replaying) presentation and the online flag for `CanPlayLivingWorldSpeech`,
    /// the current challenge, and the MAIN objects `UpdateVoiceInfo` reads (which exist once the
    /// mixer is initialized).
    fn update_speech(&mut self) {
        let game = SpeechGameState {
            chase_flags: self.chase_audio.1,
            tick: self.world.as_ref().map_or(0, |world| world.sim_tick),
            presentation_live: self.state.presentation.is_some_and(|(replay, _)| !replay),
            online_game: self.state.online_game,
            challenge_type: self.state.challenge_type,
            challenge_host: self.state.challenge_host,
            challenge_large_arena: self.state.challenge_large_arena,
        };
        let objects = if self.mixer.map().is_some() {
            let volumes = self.nis_volumes;
            SpeechObjects {
                announcer: Some(self.announcer_object),
                camera_man: Some(self.camera_man_object),
                nis: Some(NisSpeechObject {
                    scripted_speech_vol: volumes.scripted_speech,
                    scripted_speech_pitch: volumes.scripted_speech_pitch,
                    nis_vol: self.nis_vol(),
                    nis_photographer_vol: volumes.nis_photographer,
                    cell_phone_vol: volumes.cell_phone,
                    nis_pro_vol: volumes.nis_pro,
                }),
            }
        } else {
            SpeechObjects::default()
        };
        self.with_speech(|speech, ctx| speech.update(ctx, game, &objects));
    }

    /// `Sk8ContextSysMan::Update` (group A, after `Sk8SpeechSystem::Update`): the context systems
    /// against this system's `Contexts::GameInterface` values.
    fn update_contexts(&mut self) {
        let state = &self.state;
        let values = GameValues {
            challenge_type: state.challenge_type,
            game_mode: state.game_mode,
            online_game: state.online_game,
            in_replay: state.presentation.is_some_and(|(replay, _)| replay),
            player_providing_input: self.player_providing_input,
            away_time: self.away_time,
            is_male: self.is_male,
            fe_music_state: self.fe_music_state,
            scene_type: self.current_scene.scene_type,
            paused: self.is_paused,
            time_since_unpaused: self.time_since_unpaused,
            hud_state: state.hud_state,
            challenge_host: state.challenge_host,
            gvr: state.gvr,
            danny_way_jam: state.danny_way_jam,
            mega_challenge: state.mega_challenge,
        };
        self.with_speech(|speech, ctx| speech.update_contexts(ctx, &values));
    }

    /// `MusicSystem::Update` 0x82508400 (group A, after the event dispatch, which delivers the
    /// front end's `cMsgFEMusicStateChange`).
    fn update_music(&mut self, t: f32) {
        let state = self.state;
        let posted = self.music.fe_music_state();
        self.with_music(|music, io| {
            music.front_end(state.is_fe > 0, state.fe_music_state, state.exiting_to_main_menu, state.trick_tutorial_state, io);
            music.update(t, io);
        });
        let changed = self.music.fe_music_state();
        if changed != posted {
            self.receive_fe_music_state_change(changed);
        }
    }

    /// `ReceiveFEMusicStateChangeMsg` 0x82508378 for the state `ChangeFEMusicState` posted:
    /// `mFEMusicState` and `mCurrentMenu`; its `FrontEndTrigger(FE_BEGIN)` is the music system's,
    /// and `mbInReplay` is the presentation's replay flag here.
    fn receive_fe_music_state_change(&mut self, state: i32) {
        self.fe_music_state = state;
        self.current_menu_pause = state == music::fe_state::PAUSE_MENU;
    }

    /// What the music system reads from the audio system: the current music volume and its key,
    /// the master, music master and environment send submixes, microphone 0, and the challenge,
    /// scene and local skater the interactive music follows (`mCondScore` is not ported).
    fn with_music(&mut self, f: impl FnOnce(&mut MusicSystem, &mut MusicIo<'_>)) {
        let music_volume = self.emitter_system.current_music_volume().copied();
        let music_volume_key = self.emitter_system.music_volume_key();
        let microphone = &self.microphones.microphones[0];
        let listener = Some((microphone.cur_position, microphone.direction));
        let local = self.world.as_ref().and_then(|world| world.player.skaters.iter().find(|skater| skater.local));
        let local_wiping_out = local.is_some_and(|skater| skater.is_wiping_out);
        let local_board = local.map(|skater| (skater.board_speed, skater.off_board));
        let local_race_ranking = self.world.as_ref().map_or(-1, |world| world.local_race_ranking);
        let env_send_sub_mix = self.global_effects.reverb_sub_mix();
        let scene_type = self.current_scene.scene_type;
        let scene_is_replay_nis = self.current_scene.is_replay_nis;
        let contest = self.loaded_contest();
        let cond_score_delta = self.cond_score - self.cond_prev_score;
        let chase = self.chase_audio;
        let restored_frame = self.world.as_ref().map_or(0, |world| world.frame);
        let bodyguard = self.world.as_ref().is_some_and(|world| world.bodyguard);
        let multi_speaker_music = if self.state.online_game {
            Some(self.music_master_sub_mix)
        } else {
            self.speakers.as_ref().and_then(|speakers| speakers.multi_speaker_submix(SPEAKER_MUSIC))
        };
        let Self { core, vault, rng, music, master_sub_mix, music_master_sub_mix, state, .. } = self;
        let mut io = MusicIo {
            core,
            vault,
            rng,
            music_volume,
            music_volume_key,
            master: *master_sub_mix,
            music_master: *music_master_sub_mix,
            env_send_sub_mix,
            multi_speaker_music,
            listener,
            challenge_type: state.challenge_type,
            scene_type,
            scene_is_replay_nis,
            contest,
            cond_score_delta,
            local_wiping_out,
            local_board,
            local_race_ranking,
            music_mode: state.music_mode,
            new_profile: state.new_profile,
            chase,
            in_replay: state.presentation.is_some_and(|(replay, _)| replay),
            is_fe: state.is_fe,
            restored_frame,
            bodyguard,
        };
        f(music, &mut io);
    }

    /// `mpEmitterSystem->Update(dt, MicrophoneMan microphone 0)` after the state managers'
    /// `ProcessUpdate`, with `AudioSystem::GetEmitterKey`.
    fn update_emitter_system(&mut self, t: f32) {
        let presentation_key = self.world.as_ref().filter(|_| self.state.presentation.is_some()).map(|world| world.emitter_key);
        let key = emitter_system::audio_emitter_key(presentation_key, self.state.challenge_type);
        let Self { core, mixer, vault, rng, microphones, aems, emitters, emitter_system, .. } = self;
        let (Some(map), Some(emitters)) = (mixer.map_mut(), emitters.as_mut()) else { return };
        let mut io = EmitterIo { core, aems, map, vault, rng };
        emitter_system.update(t, microphones.microphones[0].cur_position, key, emitters, &mut io);
    }

    /// One 256-frame block of the mastering voice, folded to stereo the way RWAC's
    /// `FoldDownMapper` 6-to-2 map does it (`sFoldDownMaps` 28..33: L = C * 0.707, + FL, + SL).
    /// AEMS, the `GlobalEffects` LFOs, Pathfinder, a movie's audio renderer and the skateboards'
    /// grain players run as the frame's `TIMERSTAGE_BEFORECOMMANDS` timers.
    pub fn render_stereo(&mut self, out: &mut Vec<f32>) {
        let gain = f32::from_bits(0x3f34_fdf4);
        let Self { core, players, rng, aems, global_effects, music, master_sub_mix, movie, .. } = self;
        let frame = core.render_frame_with(|core, id, period| {
            if id == aems::TIMER_ID {
                aems.timer_update(core, period);
            } else if id == movie::TIMER_ID {
                if let Some(movie) = movie.as_mut() {
                    movie.timer(core);
                }
            } else if id == global_effects::LFO_TIMER_ID {
                global_effects.lfo_timer(core, period);
            } else if id == music::path::TIMER_ID {
                music.path_timer(core, *master_sub_mix, period);
            } else if let Some(players) = players.as_mut() {
                players.grain_timer(id, core, rng, period);
            }
        });
        for frame in frame.chunks_exact(6) {
            let (front_left, front_right, center, surround_left, surround_right) = (frame[0], frame[1], frame[2], frame[4], frame[5]);
            out.push(center * gain + front_left + surround_left);
            out.push(center * gain + front_right + surround_right);
        }
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// A seed from the clock, standing in for the time base (`mftb`) `InitApp` seeds with.
fn clock_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.subsec_nanos() ^ elapsed.as_secs() as u32)
}
