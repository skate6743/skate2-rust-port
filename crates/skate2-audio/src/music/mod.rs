//! The music system (`Sk8::Audio::MusicSystem` over `MusicCore`, created by
//! `Sk8AppMan::CreateAudioSingletons` 0x8272e4c0) and EA Pathfinder, the interactive-music library
//! it plays the `.mpf` maps and `_Stream.mus` streams through.
//!
//! A target state (front end, environment, chase, challenge, replay, iPod) is set by triggers;
//! `UpdateState` moves the current state towards it, switching the one stream implementation
//! between the world, game and iPod tracks and starting songs with Pathfinder events. The front
//! end plays the iPod project with the front-end playlists; in the world the music volumes of
//! `music.ems` (`Sk8EmitterSystem::GetCurrentMusicVolume`) pick environment playlists.

pub mod dsp;
pub mod path;
pub mod path_track;
pub mod sfx;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::custom_song::CustomSongPlayer;
use crate::formats::pathfinder::MapFile;
use crate::nicotine::MixMap;
use crate::player::physics::point_graph;
use crate::player::pos3d::{MicrophoneMan, Pos3dShared};
use crate::random::RwRandom;
use crate::rwac::core::AudioCore;
use crate::rwac::PluginRef;
use crate::vault::{Collection, Vault};
use crate::vmx::{self, Vec3};
use crate::world::emitter_system::MusicVolumeInfo;
use dsp::{DspInputs, MusicDspVoice, AUD_MUSIC};
use path::Path;
use path_track::SndIo;
use sfx::MusicObject;

/// `MusicSystem::eMusicMode`.
pub mod mode {
    pub const INIT: i32 = 1;
    pub const NONE: i32 = 2;
    pub const FE: i32 = 3;
    pub const ENVIRONMENT: i32 = 4;
    pub const CHASE: i32 = 5;
    pub const CHALLENGE: i32 = 6;
    pub const REPLAY: i32 = 7;
    pub const IPOD: i32 = 8;
    pub const MAX: i32 = 9;
}

/// `MusicCore::eMusicTrigger`.
pub mod trigger {
    pub const READY: i32 = 0;
    pub const INIT: i32 = 1;
    pub const RESET: i32 = 2;
    pub const PAUSE: i32 = 3;
    pub const RESUME: i32 = 4;
    pub const RETURN: i32 = 5;
    pub const STOP: i32 = 6;
}

/// `MusicSystem::eChallengeTrigger`.
pub mod challenge_trigger {
    pub const COUNTDOWN: i32 = 1;
    pub const START: i32 = 2;
    pub const HALFWAY: i32 = 3;
    pub const THREE_QUARTERS: i32 = 4;
    pub const STRAIGHT_AWAY: i32 = 5;
    pub const WIN: i32 = 6;
    pub const LOSE: i32 = 7;
    pub const JUMP_SHORT: i32 = 8;
    pub const ABORT: i32 = 9;
    pub const END_ROUND: i32 = 10;
    pub const OVER: i32 = 11;
    pub const EXIT: i32 = 12;
    pub const REPLAY_START: i32 = 13;
    pub const REPLAY_END: i32 = 14;
    pub const MAX: i32 = 15;
}

/// `FE::eFEMusicState`.
pub mod fe_state {
    pub const MAIN_MENU: i32 = 0;
    pub const CAC: i32 = 1;
    pub const PAUSE_MENU: i32 = 3;
    pub const IN_GAME: i32 = 4;
    pub const REPLAY: i32 = 5;
    pub const LOADING: i32 = 6;
    pub const IGNORE: i32 = 9;
    pub const OFF: i32 = 10;
    pub const OPTIONS: i32 = 11;
    pub const MUSIC_PLAYER: i32 = 12;
    pub const MAX: i32 = 13;
}

/// `FEBackEnd::eFE_STATE`.
pub mod fe_backend {
    pub const IN_FE: i32 = 2;
    pub const EXIT_FE: i32 = 3;
    pub const IN_GAME: i32 = 5;
}

/// `eChannelMapPreset`.
pub mod channel_map {
    pub const STEREO_NORMAL: i32 = 0;
    pub const CONTEST_IPOD: i32 = 1;
    pub const CONTEST_IPOD_REPLAY: i32 = 2;
    pub const DJLOCATION_INTERACTIVE: i32 = 3;
    pub const DJLOCATION_IPOD: i32 = 4;
    pub const MAX: i32 = 5;
}

/// `MusicSystem::eIPodTrigger`.
pub mod ipod_trigger {
    pub const START: i32 = 1;
    pub const START_PLAYLIST: i32 = 2;
    pub const RESTART: i32 = 3;
    pub const STOP: i32 = 4;
}

/// `ProfileBackEnd::eMusicMode`, the profile's `tMusicPlayer::mMusicMode`.
pub mod music_mode {
    pub const OFF: i32 = 0;
    pub const WORLD: i32 = 1;
    pub const MUSIC_PLAYER: i32 = 2;
}

/// `eEnvironmentType`.
pub const ENV_DJ: i32 = 1;
pub const ENV_CONTEST: i32 = 2;

/// The stream tracks: the world, game and iPod projects' track 0 on voice 1, and all three.
pub const WORLD_TRACK: u32 = 0x1100_0001;
pub const GAME_TRACK: u32 = 0x1200_0001;
pub const IPOD_TRACK: u32 = 0x1400_0001;
pub const ALL_STREAM_TRACKS: u32 = 0x1700_0001;

/// `aDataAudioMusic_4`, `gameMapFileName`, `aDataAudioMusic_3`.
const WORLD_MAP_FILE: &str = "data\\audio/music/world.mpf";
const GAME_MAP_FILE: &str = "data\\audio/music/game.mpf";
const IPOD_MAP_FILE: &str = "data\\audio/music/ipod.mpf";

/// `Attrib::Gen::aud_playlists`.
const AUD_PLAYLISTS: u64 = 0xE93A_BF17_955A_5264;
/// `Attrib::Gen::aud_emitter`.
const AUD_EMITTER: u64 = 0xF0CE_F367_088E_FFF8;
/// The `aud_music` collection `mMusicTweaks` holds.
const MUSIC_TWEAKS: u64 = 0xD7ED_BD36_2D7D_2152;
/// The front end's playlists: the main menu (the constructor's `mCurrentPlayList`), Create A
/// Skater, and the music player's custom playlist.
const MAIN_MENU_PLAYLIST: u64 = 0xDE63_C26D_A486_B5C0;
const CAC_PLAYLIST: u64 = 0x82BE_3BCC_575F_A538;
const MUSIC_PLAYER_PLAYLIST: u64 = 0xFB6F_3EEE_B355_D7E6;
/// `aud_playlists` attributes: the iPod songs (16-byte `{id, volume, ?, resume node}`), the DJ
/// songs (12-byte `{id, volume, ?}`) and `aux_dsp_info`.
const SONGS: u64 = 0xE7CC_11BB_1071_55EC;
const DJ_SONGS: u64 = 0xF411_87C8_E359_9333;
const AUX_DSP_INFO: u64 = 0xC71A_D9EF_F39E_1EA2;
/// `aud_music` attributes: `musicTweaks` (`{?, ?, lookAheadInteractive, lookAheadLicensed}`),
/// the pitch ramp time, `rockStarTest` and the interactive music's control levels, score and
/// curve.
const MUSIC_TWEAKS_ATTR: u64 = 0xF30D_A7C3_D67C_2BC6;
/// `aud_music::Challenge_FadeOut` 0x824e3df0: the curve a contest's last round fades out on.
const CHALLENGE_FADE_OUT: u64 = 0x1A32_CF67_42ED_4205;
/// Chase signed-byte control levels (StartChaseMusic82502a40/UpdateChaseState82506cd8).
const CHASE_INTENSITY: u64 = 0xA8A0_F38E_15BE_962C;
const PITCH_RAMP_TIME: u64 = 0x3265_D558_5CC4_B8E5;
const ROCKSTAR_TEST: u64 = 0x6120_06FB_B330_D79D;
const INTERACTIVE_MIN_CONTROL_LEVEL: u64 = 0x24C0_23EB_ABFB_2842;
const INTERACTIVE_MAX_CONTROL_LEVEL: u64 = 0xDD01_9B80_FDF6_BE29;
const INTERACTIVE_MAX_CONTROL_LEVEL_FILM: u64 = 0x6BBD_9E73_03F7_7776;
const INTERACTIVE_MAX_CONTROL_LEVEL_PHOTO: u64 = 0x49EB_6030_E556_C3AB;
const INTERACTIVE_MAX_CONTROL_LEVEL_SKATE: u64 = 0x6072_A2AF_4224_8D42;
const INTERACTIVE_MAX_SCORE: u64 = 0x49B3_F159_7755_3621;
const INTERACTIVE_DECAY: u64 = 0x209B_AD99_9508_DC00;
const INTERACTIVE_SCENE_DECAY: u64 = 0xC15A_4552_627D_5EE2;
const INTERACTIVE_SCORE_CURVE: u64 = 0xC6DC_4B3E_6F1D_646E;
const INTERACTIVE_LEVEL_CURVE: u64 = 0xBF73_7546_D4D8_F11C;
/// `GetInteractiveKey` 0x824f7010's `aud_music` collections (the DJ ipod type's second).
const INTERACTIVE_MUSIC: u64 = 0x6F37_FE62_1C51_0F45;
const INTERACTIVE_MUSIC_IPOD: u64 = 0xAFD8_4945_C6C5_27D3;
/// `aud_emitter` attributes: `dsp_info` and the music volume's channel gains and curves
/// (`{gain0, gain1, gain2, curve1, curve2, curve0}`).
const EMITTER_DSP_INFO: u64 = 0x0805_BEC9_C8BA_DDF4;
const EMITTER_MUSIC_GAINS: u64 = 0xF377_5479_B97D_9485;

/// Pathfinder events: start the iPod song of `ipod`, the contest replay song of `replay`, and the
/// DJ song of `dj`.
const IPOD_EVENT: u32 = 0x0416_D750;
const REPLAY_EVENT: u32 = 0x0437_F4CB;
const DJ_EVENT: u32 = 0x015E_EFA7;

/// `Attrib::Gen::aud_race`, and its `default` collection, `mRaceKey` without a challenge.
const AUD_RACE: u64 = 0x4ADA_C090_BCE2_A930;
const RACE_DEFAULT: u64 = 0xD7ED_BD36_2D7D_2152;
/// `aud_race` attribute: the board speed's intensity curve (eight x, then eight y).
const RACE_SPEED_CURVE: u64 = 0xEAD4_6434_94A5_FF26;
/// `aud_music` attribute: the updates `raceFinishCounter` counts after a race's end before the
/// stopped music resets.
const RACE_FINISH_UPDATES: u64 = 0x59BE_3547_112A_AC36;
/// Pathfinder events of the race music on the game track: its start, the three-quarter mark,
/// the straight, the win, the loss (and abort), the short jump and a wipeout.
const RACE_START_EVENT: u32 = 0x023B_E42D;
const RACE_THREE_QUARTERS_EVENT: u32 = 0x022E_114F;
const RACE_STRAIGHT_AWAY_EVENT: u32 = 0x0263_C103;
const RACE_WIN_EVENT: u32 = 0x02C1_7631;
const RACE_LOSE_EVENT: u32 = 0x0288_B48E;
const RACE_JUMP_SHORT_EVENT: u32 = 0x022F_C215;
const RACE_WIPEOUT_EVENT: u32 = 0x02D6_D1FF;

/// `sPartIDs_ipod` 0x82fd7fd0 (`sPartNames_ipod`).
const IPOD_SONGS: [u32; 53] = [
    0x0000_0000, 0xAFE3_EEA5, 0x9A8D_F70D, 0x4F3A_798E, 0xB239_41D5, 0x0332_583B, 0x4A03_F2B7, 0x3BE6_719F, 0x118F_E8EF,
    0x4AB7_1588, 0xEB4A_90DF, 0xC742_9CCB, 0xA844_33D3, 0x97E2_2BC2, 0xC0AC_755A, 0x5556_5E50, 0x9C08_8078, 0x5F0B_2273,
    0xEF1E_3CE8, 0xFF57_1C44, 0x237D_7603, 0x5FF8_176C, 0x86E2_BDF8, 0x455C_6E3A, 0x09FA_9F89, 0x2B30_4B97, 0x53FE_965C,
    0xDC55_314D, 0x21D0_36B4, 0xA5FD_43DA, 0x6F4E_E5DF, 0x9B5A_B407, 0xE593_56E2, 0x1DEB_2B7C, 0x8F57_B12E, 0xAF11_3790,
    0x5115_A2F8, 0x223E_0841, 0xFB61_10FF, 0xA37D_F7DF, 0xD0CD_350D, 0xD762_6E78, 0x443F_3583, 0xC9A3_4A57, 0xA433_49C9,
    0x4364_91D4, 0x2054_B00B, 0x17F0_CB4A, 0xF191_1881, 0x117E_D49B, 0xF0C1_2B79, 0x5CE0_C8DD, 0x146F_38E1,
];
/// `sPartNames_ipod` 0x82fd7ef8.
const IPOD_SONG_NAMES: [&str; 53] = [
    "IPOD_INVALID",
    "AltheaAndDonna_UptownTopRanking",
    "Anubis_Anubis",
    "ASG_TheDullBlade",
    "AwesomeSnakes_IWantASnake",
    "BlackSabbath_SymptomOfTheUniverse",
    "BlackTide_Shockwave",
    "ChinaCreeps_StayOnOrDie",
    "CutChemist_Addictive",
    "DRI_BeneathTheWheels",
    "DaytonSidewinders_GoAheadOn",
    "DieHuns_MadSociety",
    "Dragonforce_HeroesOfOurTime",
    "ELO_Showdown",
    "Fuijiya&Miyagi_Collarbone",
    "GangStarr_StepIntoTheArena",
    "GaslightAnthem_IdCalledYouWoodyJoe",
    "GoonsOfDoom_SheWoreRatSkinBoots",
    "GuiltySimpson_Piglets",
    "HighTensionWires_TheyFallApart",
    "JudasPriest_FreewheelBurning",
    "KorruptedHoodlums_SKATE",
    "Koushik_ColdBeats",
    "LLCoolJ_RockTheBells",
    "LouisXiv_GuiltByAssociation",
    "McRad_MyWeakness",
    "MoneyYourLove_ForKristoffer",
    "Motorhead_EatTheRich",
    "Nas_MadeYouLook",
    "OhNo_Heavy",
    "OneManArmy_ItsEmpty",
    "PonceDeLeon_GatorJaws",
    "PublicEnemy_HarderThanYouThink",
    "QueenSeaBigShark_HoldYourHand",
    "RadioReelers_RunninOut2",
    "RageAgainstTheMachine_SleepNowInTheFire",
    "RiverboatGamblers_UhOh",
    "SamAndDave_HoldOn",
    "Sly&TheFamilyStone_ThankYou",
    "StiffLittleFingers_AlternativeUlster",
    "SuicidalTendencies_PossessedToSkate",
    "TSOL_SuperficialLove",
    "TeenageBottlerocket_Crashing",
    "TexasThieves_LosPoolRiders",
    "TheClash_DeathOrGlory",
    "TheRiptides_ReturnToBloodBeach",
    "TheSpecials_Ghosttown",
    "TickedPink_ReachOut",
    "UndergroundRailroadToCandyland_SquareBall",
    "War_LowRider",
    "WuTangClan_ProtectYaNeck",
    "YearLongDisaster_LedaAtomica",
    "YouthBrigade_IHateMyLife",
];
/// `sPartIDs_dj_interactive` 0x82fd8100 (`sPartNames_dj_interactive`).
const DJ_SONGS_INTERACTIVE: [u32; 22] = [
    0x0000_0000, 0x7321_2752, 0x9C7C_78EB, 0xFC77_B43D, 0xD80D_C54C, 0x3F21_577C, 0xF9C6_29FE, 0xFCB3_05DA, 0xB35D_A011,
    0xB6F0_F9FC, 0x3CD5_F54F, 0xAB23_51BB, 0x582B_559F, 0x5B16_AC9C, 0xBB53_066B, 0xD5AB_FEE1, 0xCA6C_2DAE, 0x7A4C_E524,
    0x6B91_2974, 0xF61F_A9BF, 0x17A6_0E44, 0xCE8D_38CA,
];

/// The `Audio_Music*` cvars the music system reads, at their defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MusicCvars {
    pub enable_music: bool,
    pub enable_environment: bool,
    pub reset_music: bool,
    pub gain_multiply: f32,
    pub straight_to_dac: bool,
    pub apply_dsp: bool,
    pub global_reverb: bool,
    pub add_dsp_near_outer: bool,
    pub test_channel_config: bool,
    pub test_channel_source: i32,
    pub test_channel_destination: i32,
    pub test_low_pass: i32,
    pub game_speed_low_pass_scalar: i32,
}

impl Default for MusicCvars {
    fn default() -> Self {
        Self {
            enable_music: true,
            enable_environment: true,
            reset_music: false,
            gain_multiply: 1.0,
            straight_to_dac: false,
            apply_dsp: true,
            global_reverb: true,
            add_dsp_near_outer: true,
            test_channel_config: false,
            test_channel_source: 0,
            test_channel_destination: 0,
            test_low_pass: 100,
            game_speed_low_pass_scalar: 100,
        }
    }
}

/// `MusicSystem::MusicState`: the mode and its 32-byte `MusicStateData` union by word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicState {
    pub mode: i32,
    pub data: [i32; 8],
}

impl MusicState {
    const fn new(mode: i32) -> Self {
        Self { mode, data: [0; 8] }
    }

    /// `MusicState::SetMode` 0x824e41f0.
    fn set_mode(&mut self, mode: i32) {
        self.data = [-1; 8];
        self.mode = mode;
        match mode {
            mode::FE => {
                self.data[0] = 1;
                self.data[1] = fe_state::IGNORE;
            }
            mode::ENVIRONMENT => {
                self.data[4] = -1;
                self.data[7] = -1;
                self.data[2] = 0;
                self.data[3] = 0;
                self.data[0] = ENV_DJ;
                self.data[5] = 0;
                self.data[6] = 0;
            }
            mode::CHASE => {
                self.data[2] = -1;
                self.data[1] = 0;
                self.data[0] = 8;
                self.data[3] &= 0x00FF_FFFF;
            }
            mode::CHALLENGE => {
                self.data[4] = -1;
                self.data[0] = 15;
                self.data[2] = 0;
                self.data[3] &= 0x0000_FFFF;
                self.data[5] = 0;
                self.data[1] = -1;
            }
            mode::REPLAY => {
                self.data[2] &= 0x00FF_FFFF;
                self.data[0] = 2;
                self.data[1] = -1;
            }
            mode::IPOD => {
                self.data[0] = 0;
                self.data[1] = -1;
            }
            _ => {}
        }
    }

    /// `MusicWorldData::playlist` (words 2 and 3).
    fn playlist(&self) -> u64 {
        (u64::from(self.data[2] as u32) << 32) | u64::from(self.data[3] as u32)
    }

    fn set_playlist(&mut self, playlist: u64) {
        self.data[2] = (playlist >> 32) as i32;
        self.data[3] = playlist as i32;
    }

    /// `MusicWorldData::Equals` 0x824e4180 (word 1 is padding).
    fn world_equals(&self, other: &[i32; 8]) -> bool {
        let data = &self.data;
        data[0] == other[0] && data[2] == other[2] && data[3] == other[3] && data[4] == other[4] && data[5] == other[5] && data[6] == other[6] && data[7] == other[7]
    }

    /// `MusicChallengeData::Equals` 0x824e4120 over its six words (`trigger`, `id`, `intensity`,
    /// the `inWipeout` and `challengeOver` bytes, `resumenode`, `raceFinishCounter`): all but the
    /// resume node and the finish counter.
    fn challenge_equals(&self, other: &[i32; 6]) -> bool {
        let data = &self.data;
        let flags = |word: i32| (word as u32) >> 16;
        data[0] == other[0]
            && f32::from_bits(data[2] as u32) == f32::from_bits(other[2] as u32)
            && flags(data[3]) == flags(other[3])
            && data[1] == other[1]
    }
}

/// `PlayListHistory`.
#[derive(Debug, Clone, Copy, Default)]
struct PlayListHistory {
    key: u64,
    last: i32,
    played: u64,
    mode: u32,
}

/// `MusicTask::eTaskStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TaskStatus {
    Ready,
    Busy,
    PendingSubtasks,
    Complete,
    Failed,
}

/// `MusicAsyncLoadTask` (a map file and its load id), `MusicCreateStreamTask` and
/// `MusicAcquireStreamTask` (a track).
#[derive(Debug, Clone)]
enum TaskKind {
    AsyncLoad { file: &'static str, load_id: u32 },
    CreateStream { track: u32 },
    AcquireStream { track: u32 },
}

#[derive(Debug, Clone)]
struct Task {
    kind: TaskKind,
    status: TaskStatus,
    follow: Vec<Task>,
}

impl Task {
    fn new(kind: TaskKind) -> Self {
        Self { kind, status: TaskStatus::Ready, follow: Vec::new() }
    }
}

/// `Sk8::Audio::SoundSource`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SoundSource {
    pub dir: Vec3,
    pub pos: Vec3,
    pub pan_size: f32,
}

/// What the music system works with from the rest of the audio system.
pub struct MusicIo<'a> {
    pub core: &'a mut AudioCore,
    pub vault: &'a Vault,
    /// `rw::math::Random`.
    pub rng: &'a mut RwRandom,
    /// `AudioSystem::GetCurrentMusicVolume` and `Sk8EmitterSystem::mAttribKeyMusicVolume`.
    pub music_volume: Option<MusicVolumeInfo>,
    pub music_volume_key: u64,
    /// The master submix, `AudioCore::GetMusicMasterSubMix` and `GlobalEffects`'
    /// `mpEnvSendSubPlugInPtrs[0]`.
    pub master: PluginRef,
    pub music_master: PluginRef,
    pub env_send_sub_mix: PluginRef,
    /// The contest presets' target: on line (`AudioSystem::mbIsOnlineGame`) the music master,
    /// else `CSTATEMGR_Speaker::GetMultiSpeakerSubmix(eSpeakerType_Music)` (`None` without
    /// multi-speaker voices, which sends to the master submix).
    pub multi_speaker_music: Option<PluginRef>,
    /// `MicrophoneMan` microphone 0's position and direction, `None` without a microphone.
    pub listener: Option<(Vec3, Vec3)>,
    /// `AudioSystem::mChallengeType` and `mCurrentSceneInfo.sceneType`.
    pub challenge_type: i32,
    pub scene_type: i32,
    /// `mCurrentSceneInfo.isReplayNIS`.
    pub scene_is_replay_nis: bool,
    /// `AudioSystem::mContestId` while a contest is loaded, with its `ChallengeType`.
    pub contest: Option<(u64, i32)>,
    /// `AudioSystem::mCondScore - mCondPrevScore` and the local skater's `isWipingOut`.
    pub cond_score_delta: i32,
    pub local_wiping_out: bool,
    /// The local skater's `boardSpeed` and `offBoard` (`AudioPresentation::mLocal`, `None` without
    /// one) and `GlobalData.localRaceRanking`.
    pub local_board: Option<(f32, bool)>,
    pub local_race_ranking: i32,
    /// `ProfileBackEnd`'s `tMusicPlayer::mMusicMode` (`music_mode`), which `ChangeFEMusicState`
    /// switches to on entering the game, and `tProfileData::mNewProfile` (+0x68), which
    /// `FrontEndTrigger` reads for Create A Skater.
    pub music_mode: i32,
    pub new_profile: bool,
    /// AudioSystem's latched ChaseMan data, and the current presentation / living-world state.
    pub chase: (u32, u8),
    pub in_replay: bool,
    pub is_fe: i32,
    pub restored_frame: u32,
    pub bodyguard: bool,
}

/// `Sk8::Audio::MusicSystem` (and `MusicCore`).
pub struct MusicSystem {
    pub path: Path,
    pub cvars: MusicCvars,
    active_stream_track: u32,
    has_stream_imp: bool,
    enable_music: bool,
    play_list_history: [PlayListHistory; 16],
    pending_tasks: Vec<Task>,
    music_trigger: i32,
    game_state: i32,
    user_music_active: bool,
    current_play_list: u64,
    dsp: [Option<MusicDspVoice>; 2],
    world_map_file: bool,
    game_map_file: bool,
    ipod_map_file: bool,
    pub object: Option<MusicObject>,
    target_state: MusicState,
    current_state: MusicState,
    is_new_profile: bool,
    current_play_index: i32,
    current_volume: f32,
    in_notify_sphere: bool,
    world_sound_source: SoundSource,
    default_mode: i32,
    return_state: MusicState,
    in_game_return_state: MusicState,
    song_was: u32,
    /// `mRaceKey`, the `aud_race` collection `mRaceAttrib` holds (0 for `DefaultDataArea`) and
    /// `mRaceTime`, the seconds the race's intensity has waited.
    race_key: u64,
    race_attrib: u64,
    race_time: f32,
    cur_interactive_score: f32,
    cur_interactive_level: i32,
    last_trick_score: i32,
    mid_to_outer_ratio: f32,
    first_init: bool,
    challenge_loading: bool,
    /// `mAllContestRoundsOver` (+0x4B8): set from construction until a contest counts down.
    all_contest_rounds_over: bool,
    /// `mbInChallengeReplay` (+0x60A): a contest's replay music is playing.
    in_challenge_replay: bool,
    environment_trigger_count: i32,
    set_default_mode_count: i32,
    last_chase_end_time: f64,
    last_chase_frame: u32,
    no_chase_frame_count: i32,
    street_guard_was_chasing: bool,
    /// The statics: `SetDefaultMode`'s `sFirstTime`, `MusicCore::Update`'s `sPaused`,
    /// `UpdateIPodState`'s and `UpdateEnvironmentState`'s `wasPlaying`, `MapStreamChannels`'
    /// `presetWas` and submixes, and `CheckConfigVars`' last values.
    first_time: bool,
    paused: bool,
    ipod_was_playing: bool,
    environment_was_playing: bool,
    preset_was: i32,
    destination_sub_mix: Option<PluginRef>,
    dsp_sub_mix: Option<PluginRef>,
    aux_dsp_sub_mix: Option<PluginRef>,
    last_enable_music: bool,
    last_gain_multiply: f32,
    last_straight_to_dac: Option<bool>,
    last_test_channel: (bool, i32, i32),
    streamer_latency: i32,
    /// `ChangeFEMusicState`'s `StateManagerFrontEnd::mFEMusicState` (9 from its base
    /// constructor 0x82d68230), and whether the front end was entered last.
    fe_music_state: i32,
    in_fe: bool,
    /// The song `UpdateChyron` hands `FrontEndManager::mChyron.Display`.
    pub chyron: Option<u32>,
    /// `ProfileBackEnd::tMusicPlayer` as `FE::MusicPlayerManager` reads it for the music system:
    /// the current playlist's first `mNumSongs` songs and `mCurrentSongIndex`.
    music_player_songs: Vec<u32>,
    music_player_index: i32,
    /// `AudioAssetManager` loads of the map files: (load id, the file, its bytes once loaded).
    asset_loads: Vec<(u32, &'static str, Option<Arc<[u8]>>)>,
    /// PC port: the music player's custom songs (`custom_song`): each one's file by its song id,
    /// the one playing as the iPod state's song, and whether a movie holds the music paused.
    custom_songs: HashMap<u32, PathBuf>,
    custom_player: Option<CustomSongPlayer>,
    movie_pause: bool,
}

impl Default for MusicSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// The `aud_playlists` collection key `FindCollection` resolves (0 for a missing one).
fn find_playlist(vault: &Vault, key: u64) -> u64 {
    if key != 0 && vault.has_collection(Collection::new(AUD_PLAYLISTS, key)) { key } else { 0 }
}

fn playlist(key: u64) -> Collection {
    Collection::new(AUD_PLAYLISTS, key)
}

/// `aud_playlists` layout word 7: the playlist type.
fn playlist_type(vault: &Vault, key: u64) -> i32 {
    if key == 0 { 0 } else { vault.layout_word(playlist(key), 7) as i32 }
}

/// The collection key a `RefSpec` (class, collection, pointer) names.
fn refspec_key(bytes: &[u8]) -> u64 {
    bytes.get(8..16).map_or(0, |b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

impl MusicSystem {
    /// `MusicSystem::MusicSystem` 0x824f5ec0 (and `MusicCore::MusicCore` 0x82a805a8).
    #[must_use]
    pub fn new() -> Self {
        Self {
            path: Path::new(),
            cvars: MusicCvars::default(),
            active_stream_track: 0,
            has_stream_imp: false,
            enable_music: true,
            play_list_history: [PlayListHistory::default(); 16],
            pending_tasks: Vec::new(),
            music_trigger: trigger::READY,
            game_state: fe_state::IGNORE,
            user_music_active: false,
            current_play_list: 0,
            dsp: [None, None],
            world_map_file: false,
            game_map_file: false,
            ipod_map_file: false,
            object: None,
            target_state: MusicState::new(mode::INIT),
            current_state: MusicState::new(mode::INIT),
            is_new_profile: false,
            current_play_index: 0,
            current_volume: 1.0,
            in_notify_sphere: false,
            world_sound_source: SoundSource { pan_size: 10.0, ..SoundSource::default() },
            default_mode: mode::ENVIRONMENT,
            return_state: MusicState::new(mode::INIT),
            in_game_return_state: MusicState::new(mode::INIT),
            song_was: 0,
            race_key: 0,
            race_attrib: 0,
            race_time: 0.0,
            cur_interactive_score: 0.0,
            cur_interactive_level: 0,
            last_trick_score: 0,
            mid_to_outer_ratio: 0.0,
            first_init: false,
            challenge_loading: false,
            all_contest_rounds_over: true,
            in_challenge_replay: false,
            environment_trigger_count: 0,
            set_default_mode_count: 0,
            last_chase_end_time: 0.0,
            last_chase_frame: 0,
            no_chase_frame_count: 0,
            street_guard_was_chasing: false,
            first_time: true,
            paused: false,
            ipod_was_playing: false,
            environment_was_playing: false,
            preset_was: channel_map::MAX,
            destination_sub_mix: None,
            dsp_sub_mix: None,
            aux_dsp_sub_mix: None,
            last_enable_music: true,
            last_gain_multiply: 1.0,
            last_straight_to_dac: None,
            last_test_channel: (false, 0, 0),
            streamer_latency: 1000,
            fe_music_state: fe_state::IGNORE,
            in_fe: false,
            chyron: None,
            music_player_songs: Vec::new(),
            music_player_index: 0,
            asset_loads: Vec::new(),
            custom_songs: HashMap::new(),
            custom_player: None,
            movie_pause: false,
        }
    }

    /// PC port: the music player's custom songs, each one's id and file.
    pub fn set_custom_songs(&mut self, songs: &[(u32, PathBuf)]) {
        self.custom_songs = songs.iter().cloned().collect();
    }

    /// PC port: the custom song playing is let go.
    fn stop_custom_song(&mut self, core: &mut AudioCore) {
        if let Some(player) = self.custom_player.take() {
            player.stop(core);
        }
    }

    /// PC port: a custom song plays only as the iPod state's song, and holds its place while the
    /// music is paused as the stream's songs do.
    fn update_custom_song(&mut self, core: &mut AudioCore) {
        if self.custom_player.is_none() {
            return;
        }
        if self.target_state.mode != mode::IPOD {
            self.stop_custom_song(core);
            return;
        }
        let paused = !self.enable_music || !self.music_system_turned_on() || self.paused || self.movie_pause;
        if let Some(player) = self.custom_player.as_mut() {
            player.set_paused(core, paused);
        }
    }

    /// `MusicSystem::Initialize` 0x824e8c98 (`Sk8AppMan::CreateAudioSingletons` 0x8272e4c0 calls
    /// it after the speech system): `MusicCore::Initialize` 0x82a806e0, which needs the music
    /// system enabled, adds the Pathfinder timer (`PATH_vectortorwaudiocore` 0x829c3b40) and seeds
    /// Pathfinder with `rw::math::Random()`; the song lists are the `IPOD_SONGS` and
    /// `DJ_SONGS_INTERACTIVE` tables.
    pub fn initialize(&mut self, core: &mut AudioCore, rng: &mut RwRandom) -> bool {
        if !self.music_system_enabled() {
            return false;
        }
        core.add_before_commands_timer(path::TIMER_ID);
        self.path.seed_random(rng.next_u32());
        true
    }

    /// The constructor's `mCurrentPlayList` (`FindCollection` of the main menu playlist), once the
    /// attribute database is loaded.
    pub fn find_main_menu_play_list(&mut self, vault: &Vault) {
        self.current_play_list = find_playlist(vault, MAIN_MENU_PLAYLIST);
    }

    /// The music state the master object reads (`GetCurrentState`).
    #[must_use]
    pub fn current_mode(&self) -> i32 {
        self.current_state.mode
    }

    #[must_use]
    pub fn target_mode(&self) -> i32 {
        self.target_state.mode
    }

    /// `StateManagerFrontEnd::mFEMusicState`: the state `ChangeFEMusicState` posted last in
    /// `cMsgFEMusicStateChange` (the base constructor's 9 before the first).
    #[must_use]
    pub fn fe_music_state(&self) -> i32 {
        self.fe_music_state
    }

    /// `mActiveStreamTrackID`.
    #[must_use]
    pub fn active_stream_track(&self) -> u32 {
        self.active_stream_track
    }

    #[must_use]
    pub fn enable_music(&self) -> bool {
        self.enable_music
    }

    /// `mWorldSoundSource`, the music object's controller position.
    #[must_use]
    pub fn world_sound_source(&self) -> SoundSource {
        self.world_sound_source
    }

    /// `MusicSystemEnabled` 0x824e8b38.
    fn music_system_enabled(&self) -> bool {
        self.cvars.enable_music
    }

    /// `MusicSystemTurnedOn` 0x824e8b50.
    fn music_system_turned_on(&self) -> bool {
        self.default_mode != mode::NONE
    }

    /// `GameMusicDisabled` 0x82a80350: the user's own (XMP) music never plays here.
    #[must_use]
    pub fn game_music_disabled(&self) -> bool {
        self.user_music_active
    }

    /// `IsMusicPlaying` 0x824e9c88.
    #[must_use]
    pub fn is_music_playing(&self) -> bool {
        // PC port: a custom song counts as the stream's songs do.
        self.enable_music && (self.path.flags.current_node >= 0 || self.custom_player.as_ref().is_some_and(CustomSongPlayer::playing))
    }

    #[must_use]
    pub fn challenge_loading(&self) -> bool {
        self.challenge_loading
    }

    /// `MusicCore::MusicTrigger` 0x82a7fbb0.
    pub fn music_trigger(&mut self, trigger: i32) {
        if self.music_system_enabled() && (self.music_trigger != trigger::INIT || self.has_stream_imp) {
            self.music_trigger = trigger;
        }
    }

    /// `StateChanged` 0x82a7fba0.
    fn state_changed(&mut self) {
        self.path.flags.state_ready = false;
    }

    fn snd<'a>(&self, io: &'a mut MusicIo<'_>) -> SndIo<'a> {
        SndIo { core: io.core, dry_submix: self.path.dry_submix.unwrap_or(io.master) }
    }

    /// The front end's boot: `EnterFE` 0x824fd320 (`MusicTrigger(ALL_INIT)`), then the music
    /// updates of the boot screens until the first initialization's reset (`kResetMusic` from
    /// `MusicSystem::Update` once the stream implementation exists) has run, before any front-end
    /// screen sends its music state.
    pub fn boot(&mut self, io: &mut MusicIo<'_>) {
        self.in_fe = true;
        self.music_trigger(trigger::INIT);
        for _ in 0..600 {
            self.update(1.0 / 60.0, io);
            if self.first_init && !self.cvars.reset_music && self.music_trigger == trigger::READY && self.pending_tasks.is_empty() {
                break;
            }
        }
    }

    /// `AudioAssetManager::Update` for the map files: the loads `AsyncLoad` queued finish.
    fn update_asset_loads(&mut self, io: &MusicIo<'_>) {
        let files = io.core.stream_pools.files();
        for (_, path, data) in &mut self.asset_loads {
            if data.is_none() {
                if let Some(files) = files.as_ref() {
                    let size = files.size(path).unwrap_or(0) as usize;
                    let mut bytes = vec![0u8; size];
                    if size > 0 && files.read_at(path, 0, &mut bytes) {
                        *data = Some(Arc::from(bytes));
                    }
                }
            }
        }
    }

    /// `MusicSystem::Update` 0x82508400 (`MusicCore::Update` 0x82a80920 first).
    pub fn update(&mut self, dt: f32, io: &mut MusicIo<'_>) -> bool {
        self.update_asset_loads(io);
        let result = self.core_update(dt, io);
        self.update_custom_song(io.core);
        if !self.first_init && self.has_stream_imp {
            self.cvars.reset_music = true;
            self.first_init = true;
        }
        self.update_chyron();
        self.check_chase_state(io);
        self.check_for_contest_replay(io);
        if self.set_default_mode_count > 0 {
            self.set_default_mode_count -= 1;
        }
        result
    }

    /// `MusicCore::Update` 0x82a80920.
    fn core_update(&mut self, dt: f32, io: &mut MusicIo<'_>) -> bool {
        self.check_config_vars(io);
        if !self.music_system_enabled() {
            return false;
        }
        if self.music_trigger != trigger::READY {
            self.update_music_state(io);
        }
        if !self.enable_music || !self.music_system_turned_on() {
            return false;
        }
        let paused = self.game_state == fe_state::REPLAY || self.game_state == fe_state::PAUSE_MENU;
        if self.paused != paused {
            self.paused = paused;
            let mut snd = self.snd(io);
            self.path.pause(u32::MAX, paused, &mut snd);
        }
        if paused {
            return false;
        }
        let mut ready = true;
        if !self.check_pending_tasks(io) {
            ready = self.update_state(dt, io);
        }
        if ready {
            self.update_position(io);
            self.update_volume(false, io);
            self.update_dsp(false, io);
        }
        true
    }

    /// `CheckConfigVars` 0x82505db8 (the Pathfinder debug channels, the iPod test song and the
    /// chase, race and replay test triggers are debug cvars at their idle values).
    fn check_config_vars(&mut self, io: &mut MusicIo<'_>) -> bool {
        let enable = self.cvars.enable_music;
        if self.last_enable_music != enable {
            self.last_enable_music = enable;
            let pause = !self.enable_music || !enable;
            if self.active_stream_track != 0 {
                let mut snd = self.snd(io);
                self.path.pause(u32::MAX, pause, &mut snd);
            }
        }
        if !enable || !self.enable_music {
            return false;
        }
        if self.last_gain_multiply != self.cvars.gain_multiply {
            self.last_gain_multiply = self.cvars.gain_multiply;
            self.update_volume(true, io);
        }
        let straight = self.cvars.straight_to_dac;
        if self.last_straight_to_dac.is_some_and(|last| last != straight) {
            self.map_stream_channels(channel_map::MAX, io);
        }
        self.last_straight_to_dac = Some(straight);
        if self.cvars.reset_music {
            self.cvars.reset_music = false;
            self.stop_stream(io);
            self.target_state.mode = mode::NONE;
            self.current_state.mode = mode::NONE;
            self.music_trigger(trigger::RESET);
        }
        let test = (self.cvars.test_channel_config, self.cvars.test_channel_source, self.cvars.test_channel_destination);
        if test != self.last_test_channel {
            self.last_test_channel = test;
            if test.0 {
                self.map_stream_channels(channel_map::MAX, io);
            } else {
                self.stop_stream(io);
                self.target_state.mode = mode::NONE;
                self.current_state.mode = mode::NONE;
                self.music_trigger(trigger::RESET);
            }
        }
        let tweaks = Collection::new(AUD_MUSIC, MUSIC_TWEAKS);
        let index = if self.active_stream_track == GAME_TRACK { 2 } else { 3 };
        let latency = crate::nicotine::shape::fctiwz(f32::from_bits(io.vault.struct_word(tweaks, MUSIC_TWEAKS_ATTR, 0, index)) * 1000.0);
        if self.streamer_latency != latency {
            self.streamer_latency = latency;
            self.path.latency(u32::MAX, latency as i16);
        }
        true
    }

    /// `UpdateMusicState` 0x82a7fc20.
    fn update_music_state(&mut self, io: &mut MusicIo<'_>) -> bool {
        let ramp = crate::nicotine::shape::fctiwz(self.pitch_ramp_time(io.vault) * 1000.0) as u32;
        match self.music_trigger {
            trigger::INIT => {
                self.on_init(io);
                self.music_trigger = trigger::READY;
            }
            trigger::RESET => {
                self.enable_music = true;
                self.stop_stream(io);
                let mut snd = self.snd(io);
                self.path.pause(u32::MAX, false, &mut snd);
                self.path.set_volume(ALL_STREAM_TRACKS, 127, &mut snd);
                self.on_reset(io);
                self.music_trigger = trigger::READY;
            }
            trigger::PAUSE => {
                if self.enable_music {
                    self.path.pitch_mult_fade(ALL_STREAM_TRACKS, 0, ramp, 1);
                }
                self.enable_music = false;
                self.music_trigger = trigger::READY;
            }
            trigger::RESUME => {
                if !self.enable_music && self.music_system_turned_on() {
                    self.path.pitch_mult_fade(ALL_STREAM_TRACKS, 4096, ramp, 1);
                }
                self.music_trigger = trigger::READY;
                self.enable_music = true;
            }
            trigger::RETURN => {
                self.enable_music = true;
                self.on_return();
                self.music_trigger = trigger::READY;
            }
            trigger::STOP => {
                self.stop_stream(io);
                self.on_stop();
                self.music_trigger = trigger::READY;
            }
            _ => self.music_trigger = trigger::READY,
        }
        true
    }

    /// `GetPitchRampTime` 0x824e8e88.
    fn pitch_ramp_time(&self, vault: &Vault) -> f32 {
        let tweaks = Collection::new(AUD_MUSIC, MUSIC_TWEAKS);
        if vault.length(tweaks, PITCH_RAMP_TIME) > 0 { vault.f32(tweaks, PITCH_RAMP_TIME) } else { 1.0 }
    }

    /// `OnInit` 0x824f6760.
    fn on_init(&mut self, io: &mut MusicIo<'_>) {
        let no_imp = !self.has_stream_imp;
        let world = (!self.world_map_file).then_some(WORLD_MAP_FILE);
        self.prepare_stream(false, world, WORLD_TRACK, 0, io);
        let ipod = (!self.ipod_map_file).then_some(IPOD_MAP_FILE);
        self.prepare_stream(false, ipod, IPOD_TRACK, 2, io);
        self.load_game_data(false, io);
        if no_imp {
            self.switch_stream_track(IPOD_TRACK, true, io);
        }
    }

    /// `OnReset` 0x824f6840.
    fn on_reset(&mut self, io: &mut MusicIo<'_>) {
        self.return_state.mode = mode::INIT;
        self.current_play_list = 0;
        self.current_play_index = -1;
        self.environment_trigger(ENV_DJ, 0, 0, 0, -1, io);
    }

    /// `OnReturn` 0x824e9610.
    fn on_return(&mut self) {
        let state = self.return_state;
        self.restore_state(state);
    }

    /// `OnStop` 0x824e9618.
    fn on_stop(&mut self) {
        self.target_state.mode = mode::NONE;
    }

    /// `LoadGameData` 0x824e8b68.
    fn load_game_data(&mut self, mode_switch: bool, io: &mut MusicIo<'_>) -> bool {
        let file = (!self.game_map_file).then_some(GAME_MAP_FILE);
        if mode_switch {
            self.stop_stream(io);
        }
        self.prepare_stream(mode_switch, file, GAME_TRACK, 1, io)
    }

    /// `PrepareStream` 0x82a80ab0: the map file's load with the track's creation to follow, and
    /// with a mode switch the stream's move to the track (after the creation when both).
    fn prepare_stream(&mut self, mode_switch: bool, file: Option<&'static str>, track: u32, file_id: u32, io: &mut MusicIo<'_>) -> bool {
        let mut task = file.map(|file| {
            let mut load = Task::new(TaskKind::AsyncLoad { file, load_id: file_id });
            load.follow.push(Task::new(TaskKind::CreateStream { track }));
            load
        });
        if mode_switch {
            if let Some(switch) = self.switch_stream_task(track, io) {
                match task.as_mut() {
                    Some(load) => load.follow[0].follow.push(switch),
                    None => task = Some(switch),
                }
            }
        }
        if let Some(task) = task {
            self.pending_tasks.push(task);
        }
        !self.check_pending_tasks(io)
    }

    /// `SwitchStreamTask` 0x82a800c0.
    fn switch_stream_task(&mut self, track: u32, io: &mut MusicIo<'_>) -> Option<Task> {
        if track == self.active_stream_track {
            return None;
        }
        self.stop_stream(io);
        Some(Task::new(TaskKind::AcquireStream { track }))
    }

    /// `CheckPendingTasks` 0x82a80820: each ready task starts, finished ones go.
    fn check_pending_tasks(&mut self, io: &mut MusicIo<'_>) -> bool {
        let mut tasks = std::mem::take(&mut self.pending_tasks);
        let mut remaining = 0;
        let mut index = 0;
        while index < tasks.len() {
            if tasks[index].status == TaskStatus::Ready {
                self.task_on_task(&mut tasks[index], io);
            }
            if self.task_is_done(&mut tasks[index], io) {
                tasks.remove(index);
            } else {
                index += 1;
                remaining += 1;
            }
        }
        tasks.append(&mut self.pending_tasks);
        self.pending_tasks = tasks;
        remaining > 0
    }

    /// `OnTask`: `MusicTask::OnTask` 0x82a80c30, `MusicAsyncLoadTask::OnTask` 0x82a80c90
    /// (`AudioAssetManager::AsyncLoad`) and `MusicCreateStreamTask::OnTask` 0x82a80d00.
    fn task_on_task(&mut self, task: &mut Task, io: &mut MusicIo<'_>) {
        task.status = TaskStatus::Busy;
        match task.kind {
            TaskKind::AsyncLoad { file, load_id } => {
                if !self.task_is_done(task, io) && !self.asset_loads.iter().any(|(id, ..)| *id == load_id) {
                    self.asset_loads.push((load_id, file, None));
                }
            }
            TaskKind::CreateStream { track } => {
                let status = if self.create_stream_track(track, io) < 0 { TaskStatus::Failed } else { TaskStatus::Complete };
                self.task_set_status(task, status, io);
            }
            TaskKind::AcquireStream { .. } => {}
        }
    }

    /// `MusicTask::SetStatus` 0x82a80df8: completion starts the follow tasks first.
    fn task_set_status(&mut self, task: &mut Task, status: TaskStatus, io: &mut MusicIo<'_>) {
        if task.status < TaskStatus::PendingSubtasks && status == TaskStatus::Complete {
            for follow in &mut task.follow {
                self.task_on_task(follow, io);
            }
            task.status = TaskStatus::PendingSubtasks;
        } else {
            task.status = status;
        }
    }

    /// `MusicTask::IsDone` 0x82a80d60.
    fn task_base_is_done(&mut self, task: &mut Task, io: &mut MusicIo<'_>) -> bool {
        if task.status <= TaskStatus::PendingSubtasks {
            if task.status < TaskStatus::PendingSubtasks {
                return false;
            }
            for index in 0..task.follow.len() {
                let mut follow = std::mem::replace(&mut task.follow[index], Task::new(TaskKind::AcquireStream { track: 0 }));
                let done = self.task_is_done(&mut follow, io);
                task.follow[index] = follow;
                if !done {
                    return false;
                }
            }
            self.task_set_status(task, TaskStatus::Complete, io);
        }
        true
    }

    /// `IsDone`: `MusicAsyncLoadTask::IsDone` 0x82a80e70 (`GetData`, then `AddMapFile`) and
    /// `MusicAcquireStreamTask::IsDone` 0x82a80f18 (`SwitchStreamTrack`).
    fn task_is_done(&mut self, task: &mut Task, io: &mut MusicIo<'_>) -> bool {
        match task.kind {
            TaskKind::AsyncLoad { load_id, .. } => {
                let mut done = self.task_base_is_done(task, io);
                if !done {
                    let data = self.asset_loads.iter().find(|(id, ..)| *id == load_id).and_then(|(_, _, data)| data.clone());
                    if let Some(data) = data {
                        if self.add_map_file(data, load_id) < 0 {
                            task.status = TaskStatus::Failed;
                        } else {
                            done = true;
                        }
                    }
                }
                if done {
                    self.task_set_status(task, TaskStatus::Complete, io);
                }
                self.task_base_is_done(task, io)
            }
            TaskKind::AcquireStream { track } => {
                let mut done = self.task_base_is_done(task, io);
                if !done {
                    done = self.switch_stream_track(track, false, io) >= 0;
                }
                if done {
                    self.task_set_status(task, TaskStatus::Complete, io);
                }
                self.task_base_is_done(task, io)
            }
            TaskKind::CreateStream { .. } => self.task_base_is_done(task, io),
        }
    }

    /// `AddMapFile` 0x824e9248.
    fn add_map_file(&mut self, data: Arc<[u8]>, load_id: u32) -> i32 {
        let loaded = match load_id {
            0 => &mut self.world_map_file,
            1 => &mut self.game_map_file,
            2 => &mut self.ipod_map_file,
            _ => return 1,
        };
        if *loaded {
            return 1;
        }
        *loaded = true;
        match MapFile::parse(data) {
            Ok(map) => self.path.add_map_file(map),
            Err(_) => -1,
        }
    }

    /// `GetTrackInfo` 0x824e9038: the latency (`musicTweaks` word 3, word 2 for the game track),
    /// the 3.5 s buffer and the stream file.
    fn track_info(&self, track: u32, vault: &Vault) -> Option<(f32, f32, &'static str)> {
        let tweaks = Collection::new(AUD_MUSIC, MUSIC_TWEAKS);
        let word = |index: usize| f32::from_bits(vault.struct_word(tweaks, MUSIC_TWEAKS_ATTR, 0, index));
        match track {
            WORLD_TRACK => Some((word(3), 3.5, "data\\audio/music/World_Stream.mus")),
            GAME_TRACK => Some((word(2), 3.5, "data\\audio/music/Game_Stream.mus")),
            IPOD_TRACK => Some((word(3), 3.5, "data\\audio/music/Ipod_Stream.mus")),
            _ => None,
        }
    }

    /// `CreateStreamTrack` 0x82a7ff60: the track, and for the first one the stream implementation
    /// (six channels), which becomes the active track's.
    fn create_stream_track(&mut self, track: u32, io: &mut MusicIo<'_>) -> i32 {
        let Some((latency, buffer, file)) = self.track_info(track, io.vault) else { return -8 };
        let buffer_ms = crate::nicotine::shape::fctiwz(buffer * 1000.0);
        let create_imp = if self.has_stream_imp { 0 } else { buffer_ms };
        let result = self.path.create_stream_track(track, file, crate::nicotine::shape::fctiwz(latency * 1000.0));
        if result >= 0 && create_imp > 0 {
            let mut snd = self.snd(io);
            if !self.path.create_stream_imp(track, 3, buffer, &mut snd) {
                return -8;
            }
            self.has_stream_imp = true;
            if self.active_stream_track == 0 {
                self.active_stream_track = track;
            }
            if let Some(imp) = self.path.stream_imp() {
                imp.set_num_channels(6);
            }
        }
        result
    }

    /// `SwitchStreamTrack` 0x82a80148: the stream implementation moves to the track; with
    /// `force_snr_load` its stream cache loads at once.
    fn switch_stream_track(&mut self, track: u32, force_snr_load: bool, io: &mut MusicIo<'_>) -> i32 {
        if track != self.active_stream_track {
            let mut snd = self.snd(io);
            let had_imp = self.path.has_stream_imp() && self.active_stream_track != 0;
            self.path.detach_stream_imp(self.active_stream_track, &mut snd);
            if self.path.attach_stream_imp(track, &mut snd) >= 0 {
                self.active_stream_track = track;
            }
            if force_snr_load {
                self.path.cache_stream_data(track, &mut snd);
            }
            if had_imp {
                if let Some(imp) = self.path.stream_imp() {
                    imp.set_pitch_mult(4096, &mut snd);
                }
            }
        }
        0
    }

    /// `StopStream` 0x82a7fa68.
    fn stop_stream(&mut self, io: &mut MusicIo<'_>) {
        self.stop_custom_song(io.core);
        let mut snd = self.snd(io);
        self.path.clear_all_events(ALL_STREAM_TRACKS);
        self.path.pitch_mult(ALL_STREAM_TRACKS, 4096, &mut snd);
        self.path.stop(ALL_STREAM_TRACKS, &mut snd);
        let sets = self.path.stream_imp().map_or(0, |imp| imp.max_channel_sets());
        for set in 0..sets.max(0) as usize {
            self.path.channels_volume(ALL_STREAM_TRACKS, set, 127, &mut snd);
        }
        self.path.flags.song_over = false;
    }

    /// `ResetStream` 0x82a7fb08.
    fn reset_stream(&mut self, io: &mut MusicIo<'_>) {
        self.reset_dsp(io.vault);
        let ramp = crate::nicotine::shape::fctiwz(self.pitch_ramp_time(io.vault) * 1000.0) as u32;
        self.path.pitch_mult_fade(ALL_STREAM_TRACKS, 4096, ramp, 1);
        self.path.clear_all_events(ALL_STREAM_TRACKS);
    }

    /// `ResetDsp` 0x82500fd0.
    fn reset_dsp(&mut self, _vault: &Vault) {
        for dsp in self.dsp.iter_mut().flatten() {
            dsp.clear_dsp_info();
            dsp.set_reverb_channel_set_gain(1.0);
        }
    }

    /// `UpdateState` 0x824f6318.
    fn update_state(&mut self, dt: f32, io: &mut MusicIo<'_>) -> bool {
        let mut was_ready = self.path.flags.state_ready;
        let mut mode_switch = self.current_state.mode != self.target_state.mode;
        self.path.flags.state_ready = true;
        if !self.is_state_really_ready() {
            was_ready = false;
            self.path.flags.state_ready = false;
            mode_switch = true;
        }
        if mode_switch {
            self.path.flags.state_ready = false;
            was_ready = false;
            self.in_game_return_state_save_target();
            self.current_state.set_mode(self.target_state.mode);
        }
        let ready = self.check_state_ready(mode_switch, dt, io);
        if (self.path.flags.state_ready && !was_ready) || ready {
            self.current_state = self.target_state;
            self.update_volume(true, io);
            self.update_dsp(true, io);
        }
        self.path.flags.state_ready
    }

    /// `UpdateState`'s `SaveReturnState(mReturnState)`.
    fn in_game_return_state_save_target(&mut self) {
        self.return_state = self.save_return_state();
    }

    /// `IsStateReallyReady` 0x824e8be8.
    fn is_state_really_ready(&self) -> bool {
        let track = match self.target_state.mode {
            mode::INIT | mode::NONE => return true,
            mode::FE | mode::IPOD | mode::MAX => IPOD_TRACK,
            mode::ENVIRONMENT => {
                if matches!(self.target_state.data[5], 1 | 2 | 4) { IPOD_TRACK } else { WORLD_TRACK }
            }
            mode::CHASE | mode::CHALLENGE | mode::REPLAY => GAME_TRACK,
            _ => 0,
        };
        track == self.active_stream_track
    }

    /// `CheckStateReady` 0x8250d1b0.
    fn check_state_ready(&mut self, mode_switch: bool, dt: f32, io: &mut MusicIo<'_>) -> bool {
        match self.target_state.mode {
            mode::INIT => {
                if self.path.flags.state_ready {
                    return self.update_music_state(io);
                }
                false
            }
            mode::NONE => {
                self.path.flags.state_ready = true;
                false
            }
            mode::FE => {
                if mode_switch {
                    let file = (!self.ipod_map_file).then_some(IPOD_MAP_FILE);
                    self.path.flags.state_ready = self.prepare_stream(mode_switch, file, IPOD_TRACK, 2, io);
                }
                self.path.flags.state_ready && self.target_state.data[1] != self.current_state.data[1]
            }
            mode::ENVIRONMENT => {
                if mode_switch {
                    let file = (!self.world_map_file).then_some(WORLD_MAP_FILE);
                    self.path.flags.state_ready = self.prepare_stream(mode_switch, file, WORLD_TRACK, 0, io);
                }
                self.path.flags.state_ready && self.update_environment_state(dt, io)
            }
            mode::CHASE => {
                if mode_switch {
                    self.path.flags.state_ready = self.load_game_data(mode_switch, io);
                }
                self.path.flags.state_ready && self.update_chase_state(io)
            }
            mode::REPLAY => {
                if mode_switch {
                    self.path.flags.state_ready = self.load_game_data(mode_switch, io);
                }
                false
            }
            mode::CHALLENGE => {
                if mode_switch {
                    self.path.flags.state_ready = self.load_game_data(mode_switch, io);
                }
                self.path.flags.state_ready && self.update_challenge_state(io)
            }
            mode::IPOD => {
                if mode_switch {
                    let file = (!self.ipod_map_file).then_some(IPOD_MAP_FILE);
                    self.path.flags.state_ready = self.prepare_stream(mode_switch, file, IPOD_TRACK, 2, io);
                }
                self.path.flags.state_ready && self.update_ipod_state(io)
            }
            _ => false,
        }
    }

    /// `SaveReturnState` 0x824e9348: the current state with its resume node (per mode) at the
    /// playing node.
    fn save_return_state(&self) -> MusicState {
        let mut state = self.current_state;
        let resume = match state.mode {
            mode::ENVIRONMENT => Some(7),
            mode::CHASE => Some(2),
            mode::CHALLENGE => Some(4),
            mode::REPLAY | mode::IPOD => Some(1),
            _ => None,
        };
        if let Some(index) = resume {
            state.data[index] = self.path.flags.current_node;
        }
        state
    }

    /// `RestoreState` 0x824e9510.
    fn restore_state(&mut self, state: MusicState) -> bool {
        match state.mode {
            mode::NONE => return false,
            mode::ENVIRONMENT | mode::CHASE | mode::REPLAY => {
                if self.default_mode != mode::ENVIRONMENT {
                    return false;
                }
            }
            mode::CHALLENGE => {
                if state.data[1] == 0 && (self.current_state.mode != mode::CHALLENGE || self.current_state.data[1] != 0) {
                    return false;
                }
                if self.default_mode != mode::ENVIRONMENT {
                    return false;
                }
            }
            mode::IPOD => {
                if self.default_mode != mode::IPOD {
                    return false;
                }
            }
            _ => {}
        }
        self.target_state.set_mode(state.mode);
        self.target_state = state;
        self.current_state.set_mode(self.target_state.mode);
        self.path.flags.state_ready = false;
        true
    }

    /// The front end's music messages for one update, from the port's front end: entering it is
    /// `AudioSystem::EnterFE` 0x824fd320 (`MusicTrigger(ALL_INIT)`), a frame before its screens,
    /// whose `gFEStateInfo` 0x82fded80 rows hold `MainMenu`; leaving it (`ExitFE` 0x825087c0) shows
    /// the loading screen (`eSk8HUDState_TipLoader`, `Loading`) while `FEBackEnd::Data.mFEState` is
    /// `eFE_STATE_EXIT_FE`; in the game the state the HUD stack reports goes through.
    pub fn front_end(&mut self, in_fe: bool, music_state: i32, exiting_to_main_menu: bool, trick_tutorial: Option<i32>, io: &mut MusicIo<'_>) {
        if in_fe != self.in_fe {
            self.in_fe = in_fe;
            if in_fe {
                self.music_trigger(trigger::INIT);
            } else {
                self.change_fe_music_state(fe_state::LOADING, fe_backend::EXIT_FE, false, trick_tutorial, io);
            }
            return;
        }
        if in_fe {
            self.change_fe_music_state(fe_state::MAIN_MENU, fe_backend::IN_FE, exiting_to_main_menu, trick_tutorial, io);
        } else {
            self.change_fe_music_state(music_state, fe_backend::IN_GAME, exiting_to_main_menu, trick_tutorial, io);
        }
    }

    /// `StateManagerFrontEnd::ChangeFEMusicState` 0x82656f78's remapping of a front-end screen's
    /// `gFEStateInfo` music state, which posts `cMsgFEMusicStateChange` (handled by `AudioSystem::
    /// ReceiveFEMusicStateChangeMsg` 0x82508378 as `FrontEndTrigger(FE_BEGIN, state)`) on a change.
    /// `fe_backend_state` is `FEBackEnd::Data.mFEState` and `trick_tutorial` the trick tutorial
    /// state's +0x38 (`eSk8HUDState_TrickTutorial`), whose 1 turns Create A Skater's music off.
    /// Leaving the loading screen for the game first calls `MusicPlayerManager::SwitchMusicMode`
    /// 0x82629fc8 with the profile's music mode.
    pub fn change_fe_music_state(&mut self, requested: i32, fe_backend_state: i32, exiting_to_main_menu: bool, trick_tutorial: Option<i32>, io: &mut MusicIo<'_>) {
        let mut state = requested;
        if self.fe_music_state == fe_state::MAIN_MENU && requested == fe_state::IN_GAME {
            state = fe_state::IGNORE;
        } else {
            match requested {
                fe_state::CAC if trick_tutorial == Some(1) => state = fe_state::OFF,
                fe_state::OPTIONS => state = if fe_backend_state == fe_backend::IN_FE { fe_state::MAIN_MENU } else { fe_state::PAUSE_MENU },
                fe_state::IN_GAME if exiting_to_main_menu => state = fe_state::PAUSE_MENU,
                fe_state::LOADING => state = if fe_backend_state == fe_backend::IN_GAME { fe_state::IGNORE } else { fe_state::OFF },
                fe_state::MUSIC_PLAYER => state = fe_state::IN_GAME,
                _ => {}
            }
        }
        if self.fe_music_state == state || state == fe_state::IGNORE {
            return;
        }
        if (self.fe_music_state == fe_state::LOADING || self.fe_music_state == fe_state::OFF) && state == fe_state::IN_GAME {
            self.switch_music_mode(io.music_mode, io);
        }
        self.fe_music_state = state;
        self.front_end_trigger(state, io);
    }

    /// `SetDefaultMode` 0x824f61e0.
    pub fn set_default_mode(&mut self, new_mode: i32, io: &mut MusicIo<'_>) {
        if !self.music_system_enabled() || (self.default_mode == new_mode && !self.first_time) {
            return;
        }
        self.first_time = false;
        match new_mode {
            mode::NONE => self.music_trigger = trigger::PAUSE,
            mode::ENVIRONMENT => {
                self.enable_music = true;
                self.stop_stream(io);
                self.target_state.mode = mode::NONE;
                self.current_state.mode = mode::NONE;
                self.music_trigger(trigger::RESET);
                self.environment_trigger(ENV_DJ, 0, 0, 0, -1, io);
            }
            mode::IPOD => {
                self.enable_music = true;
                self.stop_stream(io);
                self.target_state.mode = mode::NONE;
                self.current_state.mode = mode::NONE;
            }
            _ => {}
        }
        self.default_mode = new_mode;
        self.set_default_mode_count = 60;
        self.environment_trigger_count = 0;
    }

    /// `FE::MusicPlayerManager::SwitchMusicMode` 0x82629fc8 for the profile's `eMusicMode`, which
    /// the caller keeps as `tMusicPlayer::mMusicMode`: the music player mode plays the current
    /// playlist from `mCurrentSongIndex` (`PlayPlaylist` 0x82629cd0).
    pub fn switch_music_mode(&mut self, profile_mode: i32, io: &mut MusicIo<'_>) {
        match profile_mode {
            music_mode::WORLD => self.set_default_mode(mode::ENVIRONMENT, io),
            music_mode::OFF => self.set_default_mode(mode::NONE, io),
            music_mode::MUSIC_PLAYER => {
                self.set_default_mode(mode::IPOD, io);
                let index = if self.music_player_index >= 0 && self.music_player_index < self.music_player_num_songs() { self.music_player_index } else { 0 };
                self.music_player_index = index;
                let song = self.music_player_song(index);
                self.ipod_trigger(ipod_trigger::START, song as i32, 0, -1, io);
            }
            _ => {}
        }
    }

    /// The profile's current music player playlist (`tPlaylist::mSongs` of
    /// `tMusicPlayer::mCurrentPlaylistId`, its first `mNumSongs`) and `mCurrentSongIndex`.
    pub fn set_music_player_playlist(&mut self, songs: &[u32], current_song_index: i32) {
        self.music_player_songs.clear();
        self.music_player_songs.extend_from_slice(songs);
        self.music_player_index = current_song_index;
    }

    /// `tMusicPlayer::mCurrentSongIndex`, which `GetNextSong` 0x826078f0 and `SwitchMusicMode`'s
    /// `PlayPlaylist` move.
    #[must_use]
    pub fn music_player_song_index(&self) -> i32 {
        self.music_player_index
    }

    fn music_player_num_songs(&self) -> i32 {
        i32::try_from(self.music_player_songs.len()).unwrap_or(i32::MAX)
    }

    fn music_player_song(&self, index: i32) -> u32 {
        usize::try_from(index).ok().and_then(|index| self.music_player_songs.get(index)).copied().unwrap_or(0)
    }

    /// `MusicPlayerManager::GetNextSongInCurrentPlaylist` 0x826191b0 (`GetNextSong` 0x826078f0):
    /// the song after `song` (after `mCurrentSongIndex` when the playlist lacks it), wrapping to
    /// the first, whose index becomes `mCurrentSongIndex`.
    fn next_music_player_song(&mut self, song: u32) -> u32 {
        let index = self.music_player_songs.iter().position(|&id| id == song).map_or(self.music_player_index, |index| index as i32);
        let mut next = index.wrapping_add(1);
        if next >= self.music_player_num_songs() {
            next = 0;
        }
        self.music_player_index = next;
        self.music_player_song(next)
    }

    /// `GetCurrentMusicPlayerSongID` 0x824e9628 (`MusicPlayerManager::GetCurrentSongID`
    /// 0x82629e48).
    #[must_use]
    pub fn current_music_player_song_id(&self) -> u32 {
        if self.current_state.mode == mode::IPOD { self.current_state.data[0] as u32 } else { 0 }
    }

    /// `MusicCore::GetNumMusicPlayerSongs` 0x82a803a0: the "IPOD" song list less its leading
    /// `IPOD_INVALID`.
    #[must_use]
    pub fn num_music_player_songs(&self) -> i32 {
        if self.music_system_enabled() { IPOD_SONGS.len() as i32 - 1 } else { -1 }
    }

    /// `MusicCore::GetMusicPlayerSongIDFromIndex` 0x82a80408.
    #[must_use]
    pub fn music_player_song_id_from_index(index: i32) -> u32 {
        usize::try_from(index.wrapping_add(1)).ok().and_then(|index| IPOD_SONGS.get(index)).copied().unwrap_or(0)
    }

    /// `FrontEndTrigger` 0x82506408 (`trigger` is always `FE_BEGIN`).
    pub fn front_end_trigger(&mut self, fe: i32, io: &mut MusicIo<'_>) {
        if !self.music_system_enabled() || self.default_mode == mode::NONE {
            return;
        }
        let game = self.game_state;
        if game == fe || fe == fe_state::IGNORE {
            return;
        }
        if fe != fe_state::IN_GAME {
            let pausing = fe == fe_state::PAUSE_MENU || fe == fe_state::LOADING;
            if game == fe_state::IN_GAME {
                self.in_game_return_state = self.save_return_state();
            } else if self.target_state.mode == mode::NONE || (matches!(game, fe_state::PAUSE_MENU | fe_state::LOADING | fe_state::OFF) && !pausing) {
                self.music_trigger(trigger::RESUME);
            }
        }
        match fe {
            fe_state::MAIN_MENU | fe_state::CAC => {
                if fe == fe_state::MAIN_MENU {
                    self.in_game_return_state.mode = mode::NONE;
                }
                // The main menu, and Create A Skater for a new profile (which also marks
                // `is_new_profile`), keep the music player's song when the default mode is the
                // iPod's (`MusicPlayerManager::GetCurrentSongID`).
                let ipod = if fe == fe_state::MAIN_MENU {
                    self.default_mode == mode::IPOD
                } else if io.new_profile {
                    self.is_new_profile = true;
                    self.default_mode == mode::IPOD
                } else {
                    false
                };
                if ipod {
                    self.stop_stream(io);
                    self.enable_music = true;
                    let song = self.current_music_player_song_id() as i32;
                    self.ipod_trigger(ipod_trigger::START, song, 0, -1, io);
                    self.game_state = fe;
                    return;
                }
                if !self.music_system_enabled() || self.current_state.mode == mode::CHASE || matches!(self.default_mode, mode::NONE | mode::IPOD) {
                    self.game_state = fe;
                    return;
                }
                let key = if fe == fe_state::MAIN_MENU { MAIN_MENU_PLAYLIST } else { CAC_PLAYLIST };
                self.set_play_list(key, 0, io.vault);
                self.choose_start_song(0, io);
                let song = self.current_song_id(io.vault) as i32;
                let resume = if key == MUSIC_PLAYER_PLAYLIST { self.song_word(io.vault, key, 3) } else { -1 };
                self.song_was = 0;
                let same = self.current_state.mode == mode::IPOD && self.target_state.mode == mode::IPOD && self.current_state.data[0] == song && self.current_state.data[1] == resume;
                if !same {
                    self.target_state.set_mode(mode::IPOD);
                    self.target_state.data[0] = song;
                    self.target_state.data[1] = resume;
                    self.state_changed();
                    self.game_state = fe;
                    return;
                }
            }
            fe_state::PAUSE_MENU | fe_state::LOADING => self.music_trigger(trigger::PAUSE),
            fe_state::OFF => self.music_trigger(trigger::STOP),
            fe_state::IN_GAME => {
                self.music_trigger(trigger::RESUME);
                let saved = self.in_game_return_state.mode;
                let reset = if saved == mode::INIT || saved == mode::NONE || saved == self.target_state.mode {
                    self.current_state.mode == mode::NONE || saved == mode::NONE
                } else {
                    let state = self.in_game_return_state;
                    !self.restore_state(state) && self.in_game_return_state.mode == mode::CHALLENGE
                };
                if reset {
                    self.music_trigger(trigger::RESET);
                }
            }
            _ => {}
        }
        self.game_state = fe;
    }

    /// Word `word` of the current playlist index's entry in the songs of playlist `key`.
    fn song_word(&self, vault: &Vault, key: u64, word: usize) -> i32 {
        let index = self.current_play_index.max(0) as u32;
        vault.struct_word(playlist(find_playlist(vault, key)), SONGS, index, word) as i32
    }

    /// `IPodTrigger` 0x824f6d98 (`playlist` 0 for none, `song` -1 to stop).
    pub fn ipod_trigger(&mut self, trigger: i32, song: i32, playlist_key: u64, resume_node: i32, io: &mut MusicIo<'_>) {
        if !self.music_system_enabled() || self.current_state.mode == mode::CHASE || self.default_mode == mode::NONE {
            return;
        }
        if self.default_mode == mode::IPOD && playlist_key != 0 {
            return;
        }
        let mut trigger = trigger;
        let mut song = song;
        if trigger == 5 {
            if self.target_state.mode == mode::IPOD {
                return;
            }
            trigger = 1;
        }
        if song == -1 {
            song = 0;
            trigger = 4;
        }
        let mut value = (song, resume_node);
        match trigger {
            1 => {
                if song == 0 && self.return_state.mode == mode::IPOD {
                    value = (self.return_state.data[0], self.return_state.data[1]);
                }
                if playlist_key != self.current_play_list {
                    self.set_play_list(0, -1, io.vault);
                }
                self.song_was = 0;
            }
            2 => {
                self.set_play_list(playlist_key, 0, io.vault);
                self.choose_start_song(0, io);
                value.0 = self.current_song_id(io.vault) as i32;
                if playlist_key == MUSIC_PLAYER_PLAYLIST {
                    value.1 = self.song_word(io.vault, playlist_key, 3);
                }
                self.song_was = 0;
            }
            3 => {
                self.current_state.data[0] = 0;
                self.current_state.data[1] = value.1;
            }
            4 => value.0 = 0,
            _ => {}
        }
        if self.current_state.mode == mode::IPOD && self.target_state.mode == mode::IPOD && self.current_state.data[0] == value.0 && self.current_state.data[1] == value.1 {
            return;
        }
        self.target_state.set_mode(mode::IPOD);
        self.target_state.data[0] = value.0;
        self.target_state.data[1] = value.1;
        self.state_changed();
    }

    /// `EnvironmentTrigger` 0x824e97c8.
    pub fn environment_trigger(&mut self, env_type: i32, playlist_key: u64, channel_map_type: i32, identifier: u32, resume_node: i32, io: &mut MusicIo<'_>) {
        if !self.music_system_enabled() || self.current_state.mode == mode::CHASE || self.default_mode != mode::ENVIRONMENT {
            return;
        }
        let mut world = MusicState::new(mode::ENVIRONMENT);
        world.data = [0, -1, 0, 0, -1, 0, 0, -1];
        let mut play_index = -1;
        let mut key = 0;
        if self.target_state.mode == mode::ENVIRONMENT {
            world.data = self.target_state.data;
            play_index = world.data[4];
            key = world.playlist();
        }
        world.data[0] = env_type;
        if channel_map_type != channel_map::CONTEST_IPOD_REPLAY {
            play_index = -1;
            world.data[4] = -1;
        }
        if playlist_key != 0 {
            key = playlist_key;
            world.set_playlist(key);
        }
        if channel_map_type != 0 {
            world.data[5] = channel_map_type;
        }
        if identifier != 0 {
            world.data[6] = identifier as i32;
        }
        if resume_node >= 0 {
            world.data[7] = resume_node;
        }
        let both = self.current_state.mode == mode::ENVIRONMENT && self.target_state.mode == mode::ENVIRONMENT;
        if !both {
            let saved = &self.return_state;
            if saved.mode == mode::ENVIRONMENT && key != 0 && env_type == saved.data[0] && key == saved.playlist() && play_index == saved.data[4] && world.data[5] == saved.data[5] {
                world.data[7] = saved.data[7];
            }
            if play_index < 0 && key != 0 {
                world.data[4] = self.choose_start_song(key, io);
            }
        }
        if both && self.current_state.world_equals(&world.data) {
            return;
        }
        self.target_state.set_mode(mode::ENVIRONMENT);
        self.target_state.data = world.data;
        self.state_changed();
    }

    /// `RaceTrigger` 0x824f7548: with the music on, the environment the default mode and no chase
    /// playing, `mRaceKey` becomes the challenge's `AudioRaceTuning` (`default` without a
    /// challenge). The trigger and the challenge ID's low word go into the target's challenge
    /// state (a new one from a countdown or start; a later trigger needs one), which the target
    /// takes unless both states are already that challenge state.
    pub fn race_trigger(&mut self, trigger: i32, id: u64, race_tuning: u64) {
        if !self.music_system_enabled() || self.current_state.mode == mode::CHASE || self.default_mode != mode::ENVIRONMENT {
            return;
        }
        self.race_key = if id == 0 { RACE_DEFAULT } else { race_tuning };
        let target = self.target_state.mode;
        let mut data = [challenge_trigger::MAX, -1, 0, 0, -1, 0];
        if target == mode::CHALLENGE {
            data = self.challenge_target();
        } else if trigger > challenge_trigger::START {
            return;
        }
        data[0] = trigger;
        data[1] = id as i32;
        if self.current_state.mode != mode::CHALLENGE || target != mode::CHALLENGE || !self.current_state.challenge_equals(&data) {
            self.set_challenge_target(data);
        }
    }

    /// `AudioSystem::QuitChallenge` 0x824ff478: with the music on, the environment the default mode
    /// and no chase playing, `mRaceKey` returns to `default`, and a target challenge state takes
    /// `CHALLENGE_OVER` with ID 0 unless the current state already is that.
    pub fn quit_challenge(&mut self) {
        if !self.music_system_enabled() || self.current_state.mode == mode::CHASE || self.default_mode != mode::ENVIRONMENT {
            return;
        }
        self.race_key = RACE_DEFAULT;
        if self.target_state.mode != mode::CHALLENGE {
            return;
        }
        let mut data = self.challenge_target();
        data[0] = challenge_trigger::OVER;
        data[1] = 0;
        if self.current_state.mode != mode::CHALLENGE || !self.current_state.challenge_equals(&data) {
            self.set_challenge_target(data);
        }
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} music quit challenge: current mode {}, trigger {}", self.current_state.mode, self.current_state.data[0]);
        }
    }

    /// `AudioSystem::RestartChallenge` 0x824ff320 in a race: `RaceTrigger(CHALLENGE_OVER, 0)`, then
    /// `mbChallengeLoading`.
    pub fn restart_race(&mut self) {
        self.race_trigger(challenge_trigger::OVER, 0, 0);
        self.challenge_loading = true;
    }

    /// `AudioSystem::RestartChallenge` with a contest loaded: `mbChallengeLoading`.
    pub fn restart_contest(&mut self) {
        self.challenge_loading = true;
    }

    /// `ChallengeTrigger` 0x824f76e8: a contest's music, with the music on, the environment the
    /// default mode and no chase playing. The countdown stops the stream and turns the music
    /// off; the start turns it on and plays the contest's playlist
    /// (`EnvironmentTrigger(ENV_CONTEST)` on the contest channel map, the challenge type, 52
    /// without one, as identifier); an abort or the end pauses it unless every round is over
    /// (a type-26 contest's end is); the last round fades the iPod track's channel sets out; the
    /// exit resets the music while a contest is loaded; a contest replay plays the playlist on
    /// the replay map, and its end pauses it.
    pub fn challenge_trigger(&mut self, request: i32, playlist: u64, challenge_type: i32, contest_loaded: bool, io: &mut MusicIo<'_>) {
        if !self.music_system_enabled() || self.current_state.mode == mode::CHASE || self.default_mode != mode::ENVIRONMENT {
            return;
        }
        let identifier = if challenge_type == -1 { 52 } else { challenge_type };
        match request {
            challenge_trigger::COUNTDOWN => {
                self.stop_stream(io);
                self.all_contest_rounds_over = false;
                self.enable_music = false;
            }
            challenge_trigger::START => {
                let enabled = self.enable_music;
                self.challenge_loading = false;
                if !enabled {
                    self.stop_stream(io);
                    self.enable_music = true;
                }
                let mut snd = self.snd(io);
                self.path.set_volume(ALL_STREAM_TRACKS, 127, &mut snd);
                self.in_notify_sphere = false;
                self.all_contest_rounds_over = false;
                self.environment_trigger(ENV_CONTEST, playlist, channel_map::CONTEST_IPOD, identifier as u32, -1, io);
            }
            challenge_trigger::ABORT | challenge_trigger::OVER => {
                if identifier == 26 {
                    self.all_contest_rounds_over = true;
                }
                if !self.all_contest_rounds_over {
                    self.music_trigger(trigger::PAUSE);
                }
            }
            challenge_trigger::END_ROUND => {
                let curve = io.vault.i32(Collection::new(AUD_MUSIC, MUSIC_TWEAKS), CHALLENGE_FADE_OUT);
                for set in [0, 2, 1, 3] {
                    self.path.channels_volume_fade(IPOD_TRACK, set, 0, 3000, curve);
                }
            }
            challenge_trigger::EXIT => {
                if contest_loaded {
                    self.music_trigger(trigger::RESET);
                }
                self.all_contest_rounds_over = true;
                self.challenge_loading = false;
            }
            challenge_trigger::REPLAY_START => {
                self.stop_stream(io);
                self.music_trigger(trigger::RESUME);
                let mut snd = self.snd(io);
                self.path.set_volume(ALL_STREAM_TRACKS, 127, &mut snd);
                self.in_notify_sphere = false;
                self.environment_trigger(ENV_CONTEST, playlist, channel_map::CONTEST_IPOD_REPLAY, identifier as u32, -1, io);
            }
            challenge_trigger::REPLAY_END => {
                self.music_trigger(trigger::PAUSE);
                if contest_loaded {
                    self.challenge_loading = true;
                }
            }
            _ => {}
        }
    }

    /// `CheckForContestReplay` 0x824f79f8 (`MusicSystem::Update`): with the music on and the
    /// environment the default mode, a replay scene of a loaded contest starts its replay music
    /// (`CHALLENGE_REPLAYSTART`, no playlist); once the target plays the replay map, the scene's
    /// end ends it (`CHALLENGE_REPLAYEND` as a Best Trick).
    fn check_for_contest_replay(&mut self, io: &mut MusicIo<'_>) {
        if !self.music_system_enabled() || self.default_mode != mode::ENVIRONMENT {
            return;
        }
        let contest_loaded = io.contest.is_some();
        if self.target_state.data[5] == channel_map::CONTEST_IPOD_REPLAY {
            if self.in_challenge_replay && !io.scene_is_replay_nis {
                self.challenge_trigger(challenge_trigger::REPLAY_END, 0, 8, contest_loaded, io);
                self.in_challenge_replay = false;
            }
        } else if io.scene_is_replay_nis {
            if let Some((_, challenge_type)) = io.contest {
                self.challenge_trigger(challenge_trigger::REPLAY_START, 0, challenge_type, contest_loaded, io);
                self.in_challenge_replay = true;
            }
        }
    }

    /// The target's `MusicChallengeData`.
    fn challenge_target(&self) -> [i32; 6] {
        let mut data = [0; 6];
        data.copy_from_slice(&self.target_state.data[..6]);
        data
    }

    /// `MusicState::SetMode(MUSIC_CHALLENGE)` on the target, its `MusicChallengeData`, then
    /// `StateChanged`.
    fn set_challenge_target(&mut self, data: [i32; 6]) {
        self.target_state.set_mode(mode::CHALLENGE);
        self.target_state.data[..6].copy_from_slice(&data);
        self.state_changed();
    }

    /// `UpdateChallengeState` 0x82507140. A new target trigger starts the race music (a countdown,
    /// or a start with no node playing), plays its event on the game track (a short jump keeps the
    /// old trigger; a win, loss or abort clears the finish counter) or stops the track (end of
    /// round, over). Otherwise, racing (start to straight), a change of the local skater's wipeout
    /// is kept, its start playing the wipeout event. The intensity (`CalculateRaceIntensity`) then
    /// goes to the game track's controller when its integer changes. From a win on, every update
    /// counts; past the tweaks' count with the stream stopped, the controller goes to 0 and the
    /// music resets.
    fn update_challenge_state(&mut self, io: &mut MusicIo<'_>) -> bool {
        let mut result = false;
        let target = self.target_state.data[0];
        if self.current_state.data[0] != target {
            if let Some(elapsed) = crate::splicer::trace_elapsed() {
                eprintln!("{elapsed:9.3} music race trigger {} -> {target}, node {}", self.current_state.data[0], self.path.flags.current_node);
            }
            let now = path::get_milliseconds(io.core);
            match target {
                challenge_trigger::COUNTDOWN => self.start_race_music(io),
                challenge_trigger::START if self.path.flags.current_node < 0 => self.start_race_music(io),
                challenge_trigger::THREE_QUARTERS => {
                    self.path.event(GAME_TRACK, RACE_THREE_QUARTERS_EVENT, now);
                }
                challenge_trigger::STRAIGHT_AWAY => {
                    self.path.event(GAME_TRACK, RACE_STRAIGHT_AWAY_EVENT, now);
                }
                challenge_trigger::WIN => {
                    self.path.event(GAME_TRACK, RACE_WIN_EVENT, now);
                    self.target_state.data[5] = 0;
                }
                challenge_trigger::LOSE | challenge_trigger::ABORT => {
                    self.path.event(GAME_TRACK, RACE_LOSE_EVENT, now);
                    self.target_state.data[5] = 0;
                }
                challenge_trigger::JUMP_SHORT => {
                    self.path.event(GAME_TRACK, RACE_JUMP_SHORT_EVENT, now);
                    self.target_state.data[0] = self.current_state.data[0];
                }
                challenge_trigger::END_ROUND | challenge_trigger::OVER => {
                    let mut snd = self.snd(io);
                    self.path.stop(GAME_TRACK, &mut snd);
                }
                _ => {}
            }
            result = true;
        } else if (challenge_trigger::START..=challenge_trigger::STRAIGHT_AWAY).contains(&target) {
            let flags = self.target_state.data[3] as u32;
            let wiping_out = u32::from(io.local_wiping_out);
            if wiping_out != flags >> 24 {
                if io.local_wiping_out {
                    self.path.event(GAME_TRACK, RACE_WIPEOUT_EVENT, path::get_milliseconds(io.core));
                }
                self.target_state.data[3] = ((flags & 0x00FF_FFFF) | (wiping_out << 24)) as i32;
                result = true;
            }
        }
        self.calculate_race_intensity(io);
        let intensity = crate::nicotine::shape::fctiwz(f32::from_bits(self.target_state.data[2] as u32));
        if intensity != crate::nicotine::shape::fctiwz(f32::from_bits(self.current_state.data[2] as u32)) {
            if let Some(elapsed) = crate::splicer::trace_elapsed() {
                eprintln!("{elapsed:9.3} music race intensity {intensity}");
            }
            let mut snd = self.snd(io);
            self.path.control(GAME_TRACK, intensity as u32, &mut snd);
            result = true;
        }
        if self.target_state.data[0] < challenge_trigger::WIN {
            return result;
        }
        self.target_state.data[5] = self.target_state.data[5].wrapping_add(1);
        let tweaks = Collection::new(AUD_MUSIC, MUSIC_TWEAKS);
        if self.target_state.data[5] <= io.vault.i32(tweaks, RACE_FINISH_UPDATES) || self.path.flags.current_node != -1 {
            return result;
        }
        let mut snd = self.snd(io);
        self.path.control(GAME_TRACK, 0, &mut snd);
        self.music_trigger(trigger::RESET);
        self.path.flags.state_ready = false;
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} music race reset after {} updates", self.target_state.data[5]);
        }
        false
    }

    /// `StartRaceMusic` 0x82502b50: `mbChallengeLoading` off, `mRaceAttrib` on `mRaceKey`'s
    /// `aud_race` collection, whose start intensity (layout word 1) the target takes, the stereo
    /// channel map, a stream reset, the race's event on the game track, and `mRaceTime` from 0.
    fn start_race_music(&mut self, io: &mut MusicIo<'_>) {
        self.challenge_loading = false;
        let race = Collection::new(AUD_RACE, self.race_key);
        self.race_attrib = if io.vault.has_collection(race) { self.race_key } else { 0 };
        self.target_state.data[2] = io.vault.layout_f32(Collection::new(AUD_RACE, self.race_attrib), 1).to_bits() as i32;
        self.map_stream_channels(channel_map::STEREO_NORMAL, io);
        self.reset_stream(io);
        self.path.event(GAME_TRACK, RACE_START_EVENT, path::get_milliseconds(io.core));
        self.race_time = 0.0;
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} music race start: aud_race {:#x}, active track {:#x}", self.race_attrib, self.active_stream_track);
        }
    }

    /// `CalculateRaceIntensity` 0x824f7b48, with a local skater. Until the race's delay (layout
    /// word 0) has passed in 1/60 s steps only `mRaceTime` moves. Then a ranked race (word 6)
    /// takes the intensity of the skater's `localRaceRanking` (words 5 to 2 for ranks 1 to 4, else
    /// 0); any other follows the board speed in km/h over the race's top speed (word 7), clamped
    /// to 1, through the speed curve times 127, moving towards it by at most the rise (word 8) or
    /// fall (word 9) step, and is 0 while racing (start to straight) off the board.
    fn calculate_race_intensity(&mut self, io: &MusicIo<'_>) {
        let Some((board_speed, off_board)) = io.local_board else { return };
        let race = Collection::new(AUD_RACE, self.race_attrib);
        let step = f32::from_bits(0x3C88_8889);
        if io.vault.layout_word(race, 6) != 0 {
            if self.race_time < io.vault.layout_f32(race, 0) {
                self.race_time += step;
                return;
            }
            let word = match io.local_race_ranking {
                1 => Some(5),
                2 => Some(4),
                3 => Some(3),
                4 => Some(2),
                _ => None,
            };
            let intensity = word.map_or(0.0, |word| io.vault.layout_word(race, word) as i32 as f32);
            self.target_state.data[2] = intensity.to_bits() as i32;
            return;
        }
        let x = crate::utils::clamp_fsel(board_speed * f32::from_bits(0x4066_6666) / io.vault.layout_f32(race, 7), 1.0);
        let level = graph(&io.vault.bytes(race, RACE_SPEED_CURVE, 0, 0x40), x) * 127.0;
        if self.race_time < io.vault.layout_f32(race, 0) {
            self.race_time += step;
        } else {
            let intensity = f32::from_bits(self.target_state.data[2] as u32);
            if level > intensity {
                let rise = level - intensity;
                let limit = io.vault.layout_f32(race, 8);
                self.target_state.data[2] = (crate::utils::fsel(rise - limit, limit, rise) + intensity).to_bits() as i32;
            } else if level < intensity {
                let fall = intensity - level;
                let limit = io.vault.layout_f32(race, 9);
                self.target_state.data[2] = (intensity - crate::utils::fsel(fall - limit, limit, fall)).to_bits() as i32;
            }
        }
        if (challenge_trigger::START..=challenge_trigger::STRAIGHT_AWAY).contains(&self.target_state.data[0]) && off_board {
            self.target_state.data[2] = 0.0_f32.to_bits() as i32;
        }
    }

    /// `UpdateIPodState` 0x82506aa8.
    fn update_ipod_state(&mut self, io: &mut MusicIo<'_>) -> bool {
        // PC port: a custom song plays in place of the stream's.
        let mut playing = self.path.flags.current_node >= 0 || self.custom_player.as_ref().is_some_and(CustomSongPlayer::playing);
        let song = self.target_state.data[0];
        if self.current_state.data[0] != song {
            if playing {
                self.stop_stream(io);
                playing = false;
            }
            self.stop_custom_song(io.core);
            if let Some(file) = self.custom_songs.get(&(song as u32)).cloned() {
                self.map_stream_channels(channel_map::STEREO_NORMAL, io);
                self.reset_stream(io);
                let target = self.destination_sub_mix.unwrap_or(io.master);
                let mut volume = self.calc_volume(io.music_volume.as_ref(), io.vault);
                if !self.enable_music || self.game_music_disabled() {
                    volume = 0.0;
                }
                self.custom_player = Some(CustomSongPlayer::start(file, target, volume));
                playing = true;
            } else if let Some(index) = find_song_index(&IPOD_SONGS, song as u32) {
                self.map_stream_channels(channel_map::STEREO_NORMAL, io);
                self.reset_stream(io);
                self.path.set_named_value(IPOD_TRACK, "ipod", index);
                self.path.set_named_value(IPOD_TRACK, "resumenode", self.target_state.data[1]);
                self.path.event(IPOD_TRACK, IPOD_EVENT, path::get_milliseconds(io.core));
            }
            self.ipod_was_playing = playing;
            return true;
        }
        if song != 0 && !playing && (self.ipod_was_playing || self.path.flags.song_over) {
            let (next, key) = if self.current_song_id(io.vault) != 0 {
                self.choose_next_song(0, io);
                (self.current_song_id(io.vault) as i32, self.current_play_list)
            } else {
                (self.next_music_player_song(song as u32) as i32, 0)
            };
            self.ipod_trigger(1, next, key, -1, io);
            if self.current_state.data[0] == self.target_state.data[0] {
                self.current_state.data[0] = 0;
            }
            self.path.flags.song_over = false;
            self.path.flags.state_ready = false;
        }
        self.ipod_was_playing = playing;
        false
    }

    /// `UpdateEnvironmentState` 0x8250b100.
    fn update_environment_state(&mut self, dt: f32, io: &mut MusicIo<'_>) -> bool {
        if !self.cvars.enable_environment {
            return false;
        }
        let mut result = false;
        let (target, current) = (self.target_state.data, self.current_state.data);
        if current[0] != target[0] || current[5] != target[5] {
            self.path.flags.state_ready = false;
            if target[0] > 0 && target[0] <= 2 {
                let track = if matches!(target[5], 1 | 2 | 4) { IPOD_TRACK } else { WORLD_TRACK };
                if let Some(task) = self.switch_stream_task(track, io) {
                    self.pending_tasks.push(task);
                }
                self.path.flags.state_ready = self.is_state_really_ready();
            }
            self.current_state.data[0] = self.target_state.data[0];
            if !self.path.flags.state_ready {
                return false;
            }
        }
        let mut playing = self.path.flags.current_node >= 0;
        let (target, current) = (self.target_state, self.current_state);
        if current.playlist() == target.playlist() && current.data[4] == target.data[4] && current.data[5] == target.data[5] && current.data[6] == target.data[6] {
            if current.data[4] >= 0 && !playing && (self.environment_was_playing || self.path.flags.song_over) {
                if self.current_song_id(io.vault) != 0 {
                    let next = self.choose_next_song(0, io);
                    self.current_play_index = next;
                    self.target_state.data[4] = next;
                    self.environment_trigger_count = 240;
                }
                if self.current_state.data[4] == self.target_state.data[4] {
                    self.current_state.data[4] = -1;
                }
                self.path.flags.song_over = false;
                self.path.flags.state_ready = false;
                self.target_state.data[7] = 0;
            }
        } else {
            let key = target.playlist();
            let list = find_playlist(io.vault, key);
            result = true;
            if playing {
                self.stop_stream(io);
                playing = false;
            }
            self.set_play_list(key, target.data[4], io.vault);
            if key != 0 && self.target_state.data[4] < 0 {
                self.target_state.data[4] = self.choose_start_song(0, io);
            }
            let list_type = playlist_type(io.vault, list);
            self.map_stream_channels(list_type, io);
            let index = self.target_state.data[4] as u32;
            let resume = self.target_state.data[7];
            let active = self.active_stream_track;
            match list_type {
                1 | 4 => {
                    let song = io.vault.struct_word(playlist(list), SONGS, index, 0);
                    if let Some(found) = find_song_index(&IPOD_SONGS, song) {
                        self.reset_stream(io);
                        if self.target_state.data[5] == channel_map::CONTEST_IPOD_REPLAY {
                            self.path.set_named_value(active, "replay", found);
                            self.path.set_named_value(active, "resumenode", resume);
                            self.path.event(active, REPLAY_EVENT, path::get_milliseconds(io.core));
                        } else {
                            self.path.set_named_value(active, "ipod", found);
                            self.path.set_named_value(active, "resumenode", resume);
                            self.path.event(active, IPOD_EVENT, path::get_milliseconds(io.core));
                        }
                    }
                }
                3 => {
                    let song = io.vault.struct_word(playlist(list), DJ_SONGS, index, 0);
                    if let Some(found) = find_song_index(&DJ_SONGS_INTERACTIVE, song) {
                        self.path.set_named_value(active, "dj", found);
                        self.path.set_named_value(active, "resumenode", resume);
                        self.reset_stream(io);
                        self.path.event(active, DJ_EVENT, path::get_milliseconds(io.core));
                        self.initialize_interactive_music(io.vault);
                    }
                }
                _ => {}
            }
        }
        self.environment_was_playing = playing;
        if self.target_state.data[0] == ENV_DJ {
            self.update_interactive_music(dt, io);
        }
        result
    }

    /// `GetInteractiveKey` 0x824f7010.
    fn interactive_key(&self, vault: &Vault) -> u64 {
        let list = find_playlist(vault, self.target_state.playlist());
        if playlist_type(vault, list) == 4 { INTERACTIVE_MUSIC_IPOD } else { INTERACTIVE_MUSIC }
    }

    /// `InitializeInteractiveMusic` 0x825022f8.
    fn initialize_interactive_music(&mut self, vault: &Vault) {
        let music = Collection::new(AUD_MUSIC, dsp::find_music(vault, self.interactive_key(vault)));
        self.last_trick_score = 0;
        self.cur_interactive_level = vault.i32(music, INTERACTIVE_MIN_CONTROL_LEVEL);
        self.cur_interactive_score = 0.0;
    }

    /// `UpdateInteractiveMusic` 0x82502390: the trick score's control level for the world track.
    fn update_interactive_music(&mut self, dt: f32, io: &mut MusicIo<'_>) {
        let vault = io.vault;
        let music = Collection::new(AUD_MUSIC, dsp::find_music(vault, self.interactive_key(vault)));
        if io.local_wiping_out {
            self.cur_interactive_score = 0.0;
        }
        let delta = io.cond_score_delta;
        if delta != self.last_trick_score {
            self.last_trick_score = delta;
            if delta > 0 {
                self.cur_interactive_score = delta as f32 + self.cur_interactive_score;
                let max = vault.f32(music, INTERACTIVE_MAX_SCORE);
                if self.cur_interactive_score > max {
                    self.cur_interactive_score = max;
                }
            }
        } else if dt > 0.0 {
            let decay = vault.f32(music, INTERACTIVE_DECAY) * dt;
            let mut scene_decay = vault.f32(music, INTERACTIVE_SCENE_DECAY) * dt;
            let photo = matches!(io.challenge_type, 6 | 0x25 | 0x26 | 0x2F | 0x31);
            if photo && io.scene_type == 6 {
                scene_decay = 0.0;
            }
            let score = if io.scene_type == 0 { self.cur_interactive_score - decay } else { self.cur_interactive_score - scene_decay };
            self.cur_interactive_score = score;
            if score < 0.0 {
                self.cur_interactive_score = 0.0;
            }
        }
        let x = self.cur_interactive_score / vault.f32(music, INTERACTIVE_MAX_SCORE);
        let curve = vault.bytes(music, INTERACTIVE_SCORE_CURVE, 0, 0x40);
        let level = graph(&curve, x);
        let mut max = vault.i32(music, INTERACTIVE_MAX_CONTROL_LEVEL);
        if io.challenge_type != 0x34 {
            match io.challenge_type {
                7 | 0x27 | 0x28 | 0x29 | 0x2E | 0x30 => max = vault.i32(music, INTERACTIVE_MAX_CONTROL_LEVEL_FILM),
                6 | 0x25 | 0x26 | 0x2F => max = vault.i32(music, INTERACTIVE_MAX_CONTROL_LEVEL_PHOTO),
                4 | 5 => max = vault.i32(music, INTERACTIVE_MAX_CONTROL_LEVEL_SKATE),
                _ => {}
            }
            if max < self.cur_interactive_level {
                max = self.cur_interactive_level;
            }
        }
        let min = vault.i32(music, INTERACTIVE_MIN_CONTROL_LEVEL);
        let control = crate::nicotine::shape::fctiwz(((max - min) as f32).mul_add(level, min as f32));
        if control != self.cur_interactive_level {
            self.cur_interactive_level = control;
            let mut snd = self.snd(io);
            self.path.control(WORLD_TRACK, control as u32, &mut snd);
        }
    }

    /// `GetInteractiveLevel` 0x825028c0.
    fn interactive_level(&self, vault: &Vault) -> f32 {
        if !self.music_system_enabled() {
            return 0.0;
        }
        let mut level = 0.0;
        if self.target_state.mode == mode::ENVIRONMENT && self.target_state.data[0] == ENV_DJ && self.enable_music {
            let music = Collection::new(AUD_MUSIC, dsp::find_music(vault, self.interactive_key(vault)));
            let x = self.cur_interactive_level as f32 * f32::from_bits(0x3C01_0204);
            level = graph(&vault.bytes(music, INTERACTIVE_LEVEL_CURVE, 0, 0x40), x);
        }
        if self.cvars.add_dsp_near_outer {
            level *= self.mid_to_outer_ratio;
        }
        let tweaks = Collection::new(AUD_MUSIC, MUSIC_TWEAKS);
        if vault.length(tweaks, ROCKSTAR_TEST) > 0 {
            let test = vault.f32(tweaks, ROCKSTAR_TEST);
            if test != 0.0 {
                level = test;
            }
        }
        level
    }

    /// `ChooseStartSong` 0x82a7fe40.
    fn choose_start_song(&mut self, key: u64, io: &mut MusicIo<'_>) -> i32 {
        self.choose_song(key, true, io)
    }

    /// `ChooseNextSong` 0x82a7fe58.
    fn choose_next_song(&mut self, key: u64, io: &mut MusicIo<'_>) -> i32 {
        self.choose_song(key, false, io)
    }

    /// `GetPlayHistory` 0x82a7fe70.
    fn play_history(&mut self, key: u64) -> usize {
        let index = self.play_list_history.iter().position(|history| history.key == key || history.key == 0).unwrap_or_else(|| {
            self.play_list_history[0].key = 0;
            0
        });
        if self.play_list_history[index].key == 0 {
            self.play_list_history[index] = PlayListHistory { key, ..PlayListHistory::default() };
        }
        index
    }

    /// `ChooseSong` 0x824f6530: the first unplayed song from the last one (in order, or at random
    /// for a shuffled playlist); when all have played the history starts again.
    fn choose_song(&mut self, key: u64, starting: bool, io: &mut MusicIo<'_>) -> i32 {
        let key = if key == 0 { self.current_play_list } else { key };
        let list = find_playlist(io.vault, key);
        let h = self.play_history(key);
        self.play_list_history[h].mode = io.vault.layout_word(playlist(list), 6);
        let played = self.play_list_history[h].played;
        let n = (num_songs_in_play_list(io.vault, list) as i32).min(64);
        let mut available = Vec::with_capacity(64);
        let mut skipped = 0;
        if n > 0 {
            let start = self.play_list_history[h].last + n;
            for offset in 0..n {
                let index = (start + offset) % n;
                if (1u64 << index) & played != 0 {
                    skipped += 1;
                } else {
                    available.push(index);
                }
            }
        }
        let mut remaining = n - skipped;
        if remaining == 0 {
            remaining = n;
            self.play_list_history[h].played = 0;
            available = (0..n).collect();
        }
        let first = available.first().copied().unwrap_or(-1);
        let history = self.play_list_history[h];
        let pick = match history.mode {
            0 => {
                if starting { first } else { (history.last + n + 1) % n.max(1) }
            }
            1 => available[(io.rng.next_u32() % remaining as u32) as usize],
            2 if !self.is_new_profile && starting => available[(io.rng.next_u32() % remaining as u32) as usize],
            _ => first,
        };
        let history = &mut self.play_list_history[h];
        history.last = pick;
        history.played |= 1u64.wrapping_shl(pick as u32);
        if self.current_play_list == key {
            self.current_play_index = pick;
        }
        pick
    }

    /// `GetCurrentSongID` 0x824e9178.
    fn current_song_id(&self, vault: &Vault) -> u32 {
        let index = self.current_play_index.max(0) as u32;
        let list = playlist(self.current_play_list);
        match playlist_type(vault, self.current_play_list) {
            0 | 1 | 2 | 4 => vault.struct_word(list, SONGS, index, 0),
            3 => vault.struct_word(list, DJ_SONGS, index, 0),
            _ => 0,
        }
    }

    /// `SetPlayList` 0x82501148: the playlist and its index, and its music (and `aux_dsp_info`)
    /// for the DSP voices.
    fn set_play_list(&mut self, key: u64, index: i32, vault: &Vault) {
        if self.current_play_list == key && self.current_play_index == index {
            return;
        }
        self.current_play_list = find_playlist(vault, key);
        self.current_play_index = index;
        let list = playlist(self.current_play_list);
        let music = if self.current_play_list == 0 {
            0
        } else {
            (u64::from(vault.layout_word(list, 2)) << 32) | u64::from(vault.layout_word(list, 3))
        };
        let aux = if self.current_play_list != 0 && vault.length(list, AUX_DSP_INFO) > 0 { refspec_key(&vault.bytes(list, AUX_DSP_INFO, 0, 24)) } else { 0 };
        if let Some(dsp) = self.dsp[0].as_mut() {
            dsp.set_dsp_info_key(vault, music);
            dsp.set_reverb_channel_set_gain(1.0);
        }
        if let Some(dsp) = self.dsp[1].as_mut() {
            dsp.set_dsp_info_key(vault, aux);
            dsp.set_reverb_channel_set_gain(1.0);
        }
    }

    /// `MapStreamChannels` 0x82501310: the first call creates the two DSP voices and the four
    /// channel sets; each preset then routes the sets, each set's send going to the submix
    /// `SetDrySubmixPlugin` holds at the time (the system's master submix when it holds none).
    /// The contest presets route to the music master on line, else to
    /// `CSTATEMGR_Speaker::GetMultiSpeakerSubmix(eSpeakerType_Music)`.
    fn map_stream_channels(&mut self, preset: i32, io: &mut MusicIo<'_>) {
        if self.preset_was == preset {
            return;
        }
        if self.preset_was == channel_map::MAX {
            let mut dsp0 = MusicDspVoice::new(io.vault);
            dsp0.path_track_id = u32::MAX;
            dsp0.reverb_channel_set = 3;
            self.dsp_sub_mix = dsp0.create_dsp_sub_mix(io.core, 6, self.destination_sub_mix, io.music_master);
            self.dsp[0] = Some(dsp0);
            let mut dsp1 = MusicDspVoice::new(io.vault);
            dsp1.enable_delay = true;
            dsp1.delay_max_seconds = f32::from_bits(0x3E19_999A);
            self.aux_dsp_sub_mix = dsp1.create_dsp_sub_mix(io.core, 6, self.destination_sub_mix, io.music_master);
            self.dsp[1] = Some(dsp1);
            if let Some(imp) = self.path.stream_imp() {
                imp.add_channel_set(0, 6, 45.0, 135.0, 0.0);
                imp.add_channel_set(1, 6, 45.0, 135.0, 0.0);
                imp.add_channel_set(2, 6, 45.0, 135.0, 0.0);
                imp.add_channel_set(3, 1, 45.0, 135.0, 0.0);
            }
        }
        let preset = if preset == channel_map::MAX {
            self.preset_was
        } else {
            self.preset_was = preset;
            preset
        };
        let master = io.master;
        let Self { path, cvars, destination_sub_mix, dsp_sub_mix, aux_dsp_sub_mix, .. } = self;
        let dry = path.dry_submix;
        let Some(imp) = path.stream_imp() else { return };
        let mut snd = SndIo { core: io.core, dry_submix: dry.unwrap_or(master) };
        for set in 0..4 {
            imp.remove_channel_set(set, &mut snd);
        }
        let dry_level = if cvars.straight_to_dac {
            127
        } else {
            if cvars.apply_dsp {
                *destination_sub_mix = *dsp_sub_mix;
            }
            0
        };
        imp.set_dry_level(dry_level, &mut snd);
        if cvars.global_reverb && !cvars.test_channel_config && !cvars.straight_to_dac {
            snd.dry_submix = io.env_send_sub_mix;
            imp.route_channels(3, &[(0, 0)], &mut snd);
            imp.set_channels_volume(3, 0, 0, 0, 0, &mut snd);
        }
        snd.dry_submix = destination_sub_mix.unwrap_or(master);
        match preset {
            channel_map::STEREO_NORMAL => {
                imp.set_dry_level(127, &mut snd);
            }
            channel_map::CONTEST_IPOD | channel_map::CONTEST_IPOD_REPLAY => {
                snd.dry_submix = io.multi_speaker_music.unwrap_or(master);
                imp.route_channels(2, &[(0, 0), (2, 2)], &mut snd);
                imp.set_channels_volume(2, 127, 127, 127, 127, &mut snd);
                imp.set_channels_position(2, 0, 0, 0x4000, 0, &mut snd);
            }
            channel_map::DJLOCATION_INTERACTIVE | channel_map::DJLOCATION_IPOD => {
                imp.route_channels(0, &[(0, 0), (2, 2)], &mut snd);
                snd.dry_submix = aux_dsp_sub_mix.unwrap_or(master);
                imp.route_channels(1, &[(0, 4), (2, 3)], &mut snd);
                snd.dry_submix = io.music_master;
                imp.route_channels(2, &[(0, 0), (2, 2)], &mut snd);
                imp.set_channels_volume(2, 127, 127, 127, 127, &mut snd);
                imp.set_channels_volume(0, 127, 127, 127, 127, &mut snd);
                imp.set_channels_volume(1, 127, 127, 127, 127, &mut snd);
                imp.set_channels_position(2, 0, 0, 0x4000, 0, &mut snd);
                imp.set_channels_position(0, 0, 0x4000, 0, 0, &mut snd);
                imp.set_channels_position(1, 0, 0, 0x4000, 0, &mut snd);
            }
            channel_map::MAX => {
                imp.route_channels(0, &[(cvars.test_channel_source as u8, cvars.test_channel_destination as u8)], &mut snd);
                imp.set_channels_volume(0, 127, 127, 127, 127, &mut snd);
                imp.set_channels_position(0, 0, 0x4000, 0, 0, &mut snd);
            }
            _ => {}
        }
        path.dry_submix = *destination_sub_mix;
    }

    /// `CalcVolume` 0x824f7dd8: the music object's volume for the music type, times the song's
    /// (and for a DJ environment the music volume's) volume.
    #[must_use]
    pub fn calc_volume(&self, music_volume: Option<&MusicVolumeInfo>, vault: &Vault) -> f32 {
        let Some(object) = self.object.as_ref() else { return 1.0 };
        let mut volume = 1.0;
        let index = self.current_play_index.max(0) as u32;
        let list = playlist(self.current_play_list);
        match self.current_state.mode {
            mode::ENVIRONMENT => {
                if self.current_state.data[0] == ENV_DJ {
                    volume = music_volume.map_or(0.0, |info| info.volume);
                }
                match playlist_type(vault, self.current_play_list) {
                    1 | 2 | 4 => volume *= f32::from_bits(vault.struct_word(list, SONGS, index, 1)),
                    3 => volume *= f32::from_bits(vault.struct_word(list, DJ_SONGS, index, 1)),
                    _ => {}
                }
            }
            mode::IPOD => {
                if self.current_state.data[0] != 0 && vault.struct_word(list, SONGS, index, 0) != 0 {
                    volume = f32::from_bits(vault.struct_word(list, SONGS, index, 1));
                }
            }
            _ => {}
        }
        object.vol[self.music_vol_type(vault) as usize] as f32 * f32::from_bits(0x3800_0100) * volume
    }

    /// `GetMusicVolType` 0x824e9cb0.
    fn music_vol_type(&self, vault: &Vault) -> i32 {
        match self.current_state.mode {
            mode::FE | mode::IPOD => 5 + i32::from(self.default_mode == mode::IPOD),
            mode::ENVIRONMENT => match playlist_type(vault, self.current_play_list) {
                1 | 2 => 2,
                3 => 3,
                4 => 4,
                _ => 5,
            },
            mode::CHASE => 0,
            mode::CHALLENGE => 1,
            mode::REPLAY => 8,
            _ => 5,
        }
    }

    /// `CalcWetVolume` 0x824f7ff0.
    fn calc_wet_volume(&self, vault: &Vault) -> f32 {
        self.object.as_ref().map_or(0.0, |object| object.wet_vol[self.music_vol_type(vault) as usize] as f32 * f32::from_bits(0x3800_0100))
    }

    /// `GetNicotineLPF` 0x824f8078.
    fn nicotine_lpf(&self, vault: &Vault) -> f32 {
        self.object.as_ref().map_or(0.0, |object| object.lpf[self.music_vol_type(vault) as usize] as f32)
    }

    /// `UpdateVolume` 0x82503540.
    fn update_volume(&mut self, update_required: bool, io: &mut MusicIo<'_>) {
        let mut volume = self.calc_volume(io.music_volume.as_ref(), io.vault);
        if !self.enable_music || self.game_music_disabled() {
            volume = 0.0;
        }
        let quantise = |value: f32| crate::nicotine::shape::fctiwz(value * 1000.0);
        if update_required || quantise(volume) != quantise(self.current_volume) {
            let mut snd = self.snd(io);
            if self.path.gain(ALL_STREAM_TRACKS, volume, &mut snd) >= 0 {
                self.current_volume = volume;
            }
        }
        // PC port: a custom song takes the same volume.
        if let Some(player) = self.custom_player.as_mut() {
            player.set_volume(io.core, volume.clamp(0.0, 1.0));
        }
    }

    /// `UpdateDsp` 0x82506968.
    fn update_dsp(&mut self, update_required: bool, io: &mut MusicIo<'_>) {
        if !self.cvars.apply_dsp {
            return;
        }
        let (list, index) = (self.current_play_list, self.current_play_index);
        self.set_play_list(list, index, io.vault);
        let inputs = DspInputs {
            wet_volume: self.calc_wet_volume(io.vault),
            nicotine_lpf: self.nicotine_lpf(io.vault),
            test_low_pass: self.cvars.test_low_pass,
            game_speed_low_pass_scalar: self.cvars.game_speed_low_pass_scalar,
        };
        let Self { dsp, path, .. } = self;
        let master = io.master;
        for voice in dsp.iter_mut().flatten() {
            let mut channels_volume = |core: &mut AudioCore, tracks: u32, set: usize, volume: i32| {
                let mut snd = SndIo { core, dry_submix: path.dry_submix.unwrap_or(master) };
                path.channels_volume(tracks, set, volume, &mut snd);
            };
            voice.update(update_required, io.vault, inputs, io.core, &mut channels_volume);
        }
    }

    /// `UpdatePosition` 0x82507400: outside the environment the world sound source is the
    /// listener; in a DJ environment the music volumes place the stream's channel sets.
    fn update_position(&mut self, io: &mut MusicIo<'_>) {
        if !self.cvars.enable_environment {
            return;
        }
        let Some((pos, dir)) = io.listener else { return };
        let camera = SoundSource { dir, pos, pan_size: 10.0 };
        if self.current_state.mode == mode::ENVIRONMENT {
            if (self.current_state.data[0] == ENV_DJ || self.in_notify_sphere) && self.check_environment_zones(&camera, io) != 0 {
                self.update_track_position(io);
            }
        } else {
            self.world_sound_source.pos = pos;
            self.world_sound_source.dir = dir;
        }
    }

    /// `CheckChaseState` 0x824f7088. The PPC offsets after mLastChaseEndTime use
    /// eight-byte doubles; the PDB's 16-byte long-double layout mislabels these counters.
    fn check_chase_state(&mut self, io: &MusicIo<'_>) {
        if self.default_mode != mode::ENVIRONMENT || io.in_replay
            || (self.target_state.mode == mode::CHASE && (self.target_state.data[3] as u32 >> 24) != 0)
            || io.is_fe > 0 || !self.is_stream_data_cached()
            || self.environment_trigger_count > 0 || self.set_default_mode_count > 0
        {
            return;
        }
        if io.restored_frame > self.last_chase_frame {
            if self.no_chase_frame_count > 0 {
                self.no_chase_frame_count -= 1;
            }
            self.last_chase_frame = io.restored_frame;
        }
        if self.no_chase_frame_count > 0 {
            return;
        }
        let flags = io.chase.1;
        let mut next = 8;
        if !io.bodyguard {
            if flags & 0x20 != 0 && flags & 0x80 != 0
                && (self.target_state.mode != mode::CHASE || self.target_state.data[0] < 0)
            {
                next = 0;
            }
            if flags & (0x10 | 8) != 0 {
                if self.target_state.mode == mode::CHASE {
                    if self.target_state.data[0] < 2 {
                        next = 2;
                    }
                } else {
                    next = 5;
                }
                self.street_guard_was_chasing = flags & 8 != 0 && flags & 0x10 == 0;
            }
        }
        if self.target_state.mode == mode::CHASE {
            let target = self.target_state.data[0];
            if target == 2 {
                if io.bodyguard {
                    next = 4;
                    self.street_guard_was_chasing = false;
                } else if self.street_guard_was_chasing {
                    if flags & 4 != 0 {
                        next = 3;
                    } else if flags & 8 == 0 {
                        if flags & 0x10 == 0 {
                            next = 4;
                        }
                        self.street_guard_was_chasing = false;
                    }
                } else if flags & 2 != 0 {
                    next = 3;
                } else if flags & 0x10 == 0 {
                    next = 4;
                }
            } else if (target as u32) <= 1 {
                if io.bodyguard {
                    next = 4;
                    self.street_guard_was_chasing = false;
                } else if flags & (0x80 | 0x10) == 0 {
                    next = 4;
                }
            }
        }
        if next != 8 {
            self.chase_trigger(next, false);
        }
    }

    /// `ChaseTrigger` 0x824e99d8.
    fn chase_trigger(&mut self, next: i32, test: bool) {
        if !self.music_system_enabled() || self.default_mode != mode::ENVIRONMENT {
            return;
        }
        if next == 8 || (self.current_state.mode == mode::CHASE
            && self.target_state.mode == mode::CHASE && self.current_state.data[0] == next)
        {
            return;
        }
        let (intensity, resume, bits) = if self.target_state.mode == mode::CHASE {
            (self.target_state.data[1], self.target_state.data[2], self.target_state.data[3])
        } else {
            (0, -1, 0)
        };
        self.target_state.set_mode(mode::CHASE);
        self.target_state.data[0] = next;
        self.target_state.data[1] = intensity;
        self.target_state.data[2] = resume;
        self.target_state.data[3] = (bits & 0x00ff_ffff) | ((i32::from(test)) << 24);
        self.state_changed();
    }

    /// `StartChaseMusic` 0x82502a40.
    fn start_chase_music(&mut self, io: &mut MusicIo<'_>) {
        self.reset_stream(io);
        self.path.set_named_value(u32::MAX, "chaser", 0);
        self.map_stream_channels(channel_map::STEREO_NORMAL, io);
        self.chase_control(0, io);
    }

    fn chase_control(&mut self, index: u32, io: &mut MusicIo<'_>) {
        let bytes = io.vault.bytes(Collection::new(AUD_MUSIC, MUSIC_TWEAKS), CHASE_INTENSITY, index, 1);
        // lbz/extsb/addi in both native callers, including DefaultDataArea(1).
        let control = i32::from(bytes.first().copied().unwrap_or(0) as i8) + 1;
        let mut snd = self.snd(io);
        self.path.control(GAME_TRACK, control as u32, &mut snd);
    }

    /// `StopChaseMusic` 0x824e9ad8.
    fn stop_chase_music(&mut self, io: &MusicIo<'_>) {
        self.current_state.mode = mode::INIT;
        self.target_state.mode = mode::INIT;
        self.music_trigger(trigger::RESET);
        self.path.flags.state_ready = false;
        self.no_chase_frame_count = 10;
        self.last_chase_end_time = 0.0;
        self.last_chase_frame = io.restored_frame;
    }

    /// `UpdateChaseState` 0x82506cd8. Trigger 8 is the initial state; events are
    /// the original game.mpf IDs, including immediate pursuit and escalation.
    fn update_chase_state(&mut self, io: &mut MusicIo<'_>) -> bool {
        let now = io.core.system_time;
        let since_end = now - self.last_chase_end_time;
        if since_end < now && since_end > 14.0 {
            if self.path.flags.current_node >= 0 {
                return false;
            }
            self.stop_chase_music(io);
            return true;
        }
        let current = self.current_state.data[0];
        let target = self.target_state.data[0];
        let intensity = self.target_state.data[1];
        if current != target {
            if matches!(target, 0 | 1 | 2 | 5 | 6) && current == 8 {
                self.start_chase_music(io);
            }
            let event = match target {
                0 => Some(0x0237_ED89),
                1 => Some(0x0273_7A99),
                2 => Some(0x0236_4A24),
                3 => Some(0x0294_5272),
                4 => Some(0x02B9_B8F8),
                5 => Some(0x0213_35A0),
                6 => Some(0x02CF_36FB),
                _ => None,
            };
            if let Some(event) = event {
                self.path.event(GAME_TRACK, event, path::get_milliseconds(io.core));
            }
            match target {
                3 | 4 => self.last_chase_end_time = now,
                5 | 6 => self.target_state.data[0] = 2,
                7 => self.stop_chase_music(io),
                _ => (),
            }
            if let Some(elapsed) = crate::splicer::trace_elapsed() {
                eprintln!("{elapsed:9.3} music chase trigger {current} -> {target}, flags {:#04x}, count {}, node {}", io.chase.1, io.chase.0, self.path.flags.current_node);
            } else if std::env::var_os("SKATE2_TRACE_SECURITY_CAPTURE").is_some() {
                eprintln!("music chase trigger {current} -> {target}, flags {:#04x}, count {}, node {}", io.chase.1, io.chase.0, self.path.flags.current_node);
            }
        }
        let level = if io.chase.0 > 1 { 2 } else { 1 };
        if level != intensity {
            self.chase_control((level - 1) as u32, io);
        }
        // Native compares against the saved intensity but does not store the new level.
        self.path.flags.state_ready = true;
        true
    }

    /// `IsStreamDataCached` 0x824e9648.
    fn is_stream_data_cached(&mut self) -> bool {
        self.path.stream_imp().is_some_and(|imp| imp.take_stream_cache().0.is_some())
    }

    /// `CheckEnvironmentZones` 0x82503168: the current music volume's playlist (and `aud_emitter`
    /// DSP) for the environment; a new one triggers it, then holds for 240 updates.
    fn check_environment_zones(&mut self, camera: &SoundSource, io: &mut MusicIo<'_>) -> u64 {
        self.in_notify_sphere = false;
        if camera.pos[0] == 0.0 && camera.pos[1] == 0.0 && camera.pos[2] == 0.0 {
            return 0;
        }
        if self.environment_trigger_count > 180 {
            if self.is_stream_data_cached() {
                self.environment_trigger_count -= 1;
            }
            return 1;
        }
        if self.environment_trigger_count > 0 && self.is_stream_data_cached() {
            self.environment_trigger_count -= 1;
        }
        let vault = io.vault;
        let current = self.current_play_list;
        let (mut key, mut dsp_key, mut aux_key, mut env_type, mut preset, mut identifier) = (0, 0, 0, 0, channel_map::MAX, 0);
        if let Some(info) = io.music_volume {
            identifier = info.identifier;
            key = info.playlist;
            self.world_sound_source.pos = [info.pos[0], info.pos[1], info.pos[2], 0.0];
            dsp_key = EMITTER_DSP_INFO;
            let emitter = Collection::new(AUD_EMITTER, io.music_volume_key);
            if vault.length(emitter, EMITTER_DSP_INFO) > 0 {
                dsp_key = refspec_key(&vault.bytes(emitter, EMITTER_DSP_INFO, 0, 24));
            }
            env_type = ENV_DJ;
            if key != 0 {
                let list = playlist(find_playlist(vault, key));
                preset = vault.layout_word(list, 7) as i32;
                dsp_key = (u64::from(vault.layout_word(list, 2)) << 32) | u64::from(vault.layout_word(list, 3));
                if vault.length(list, AUX_DSP_INFO) > 0 {
                    aux_key = refspec_key(&vault.bytes(list, AUX_DSP_INFO, 0, 24));
                }
            }
        }
        if let Some(dsp) = self.dsp[0].as_mut() {
            dsp.set_dsp_info_key(vault, dsp_key);
        }
        if let Some(dsp) = self.dsp[1].as_mut() {
            dsp.set_dsp_info_key(vault, aux_key);
        }
        if (key != current || env_type != self.current_state.data[0]) && preset != channel_map::MAX {
            self.map_stream_channels(preset, io);
            self.environment_trigger(env_type, key, preset, identifier, -1, io);
            self.environment_trigger_count = 240;
        }
        key
    }

    /// `UpdateTrackPosition` 0x82502bf8: the music volume's distance and the music object's
    /// azimuth place channel sets 0 and 1, whose volumes (and set 2's, with the interactive level)
    /// follow the `aud_emitter` gains and curves.
    fn update_track_position(&mut self, io: &mut MusicIo<'_>) {
        let Some(info) = io.music_volume else { return };
        let vault = io.vault;
        let emitter = Collection::new(AUD_EMITTER, io.music_volume_key);
        let azi = self.object.as_ref().map_or(0, |object| object.azi);
        let mid = (1.0 - info.inner_radius).mul_add(info.mid_radius, info.inner_radius);
        let azimuth = azi as f32 * f32::from_bits(0x3780_0000);
        if mid != 1.0 && info.distance_no_inner > mid {
            self.mid_to_outer_ratio = (1.0 - info.distance_no_inner) / (1.0 - mid);
        } else {
            self.mid_to_outer_ratio = 1.0;
        }
        let distance_no_inner = info.distance_no_inner;
        let mut level = self.interactive_level(vault);
        let near = 1.0 - info.distance;
        let mut not_level = 1.0 - level;
        let mut cap = near;
        let mut gain0 = near;
        let mut gain1 = near;
        let word = |index: usize| vault.struct_word(emitter, EMITTER_MUSIC_GAINS, 0, index);
        if word(5) == 1 {
            not_level = vmx::length_from_squared(1.0 - level);
            level = vmx::length_from_squared(level);
        }
        if word(3) == 2 {
            cap = vmx::length_from_squared(near);
            gain0 = cap;
        }
        if word(4) == 2 {
            gain1 = vmx::length_from_squared(near);
        }
        let mut set0 = f32::from_bits(word(0)) * not_level * gain0;
        let mut set1 = f32::from_bits(word(1)) * not_level * gain1;
        let mut set2 = f32::from_bits(word(2)) * level;
        if set0 > cap {
            set0 = cap;
        }
        if set1 > cap {
            set1 = cap;
        }
        if set2 > cap {
            set2 = cap;
        }
        let fctiwz = crate::nicotine::shape::fctiwz;
        let distance = fctiwz(distance_no_inner * 16384.0);
        let angle = fctiwz(azimuth * 16384.0) % 0x4000;
        let master = io.master;
        let dry = self.path.dry_submix.unwrap_or(master);
        if let Some(imp) = self.path.stream_imp() {
            let mut snd = SndIo { core: io.core, dry_submix: dry };
            imp.set_channels_position(0, angle, distance, 0, 0, &mut snd);
            imp.set_channels_position(1, angle, 0, distance, 0, &mut snd);
            imp.set_channels_volume(0, fctiwz(set0 * 127.0), -1, -1, -1, &mut snd);
            imp.set_channels_volume(1, fctiwz(set1 * 127.0), -1, -1, -1, &mut snd);
            imp.set_channels_volume(2, fctiwz(set2 * 127.0), -1, -1, -1, &mut snd);
        }
        for dsp in self.dsp.iter_mut().flatten() {
            dsp.set_reverb_channel_set_gain(set0);
        }
    }

    /// `UpdateChyron` 0x825069f0: a new iPod song for `FrontEndManager::mChyron.Display`.
    fn update_chyron(&mut self) {
        if self.current_state.mode == mode::IPOD {
            let song = self.current_state.data[0] as u32;
            if song != 0 && song != self.song_was {
                self.song_was = song;
                self.chyron = Some(song);
            }
        }
    }

    /// The Pathfinder timer and service task, from the core's `TIMERSTAGE_BEFORECOMMANDS` timer.
    pub fn path_timer(&mut self, core: &mut AudioCore, master: PluginRef, period: f32) {
        let mut snd = SndIo { core: &mut *core, dry_submix: self.path.dry_submix.unwrap_or(master) };
        self.path.timer(period, &mut snd);
        if let Some(player) = self.custom_player.as_mut() {
            player.timer(core);
        }
    }

    /// `PauseForMovie` 0x824f0a18's and `ResumeAfterMovie` 0x824f0a98's `PATH_pause`.
    pub fn pause_for_movie(&mut self, pause: bool, core: &mut AudioCore, master: PluginRef) {
        self.movie_pause = pause;
        if self.active_stream_track != 0 {
            let pause = pause || !self.enable_music;
            let mut snd = SndIo { core, dry_submix: self.path.dry_submix.unwrap_or(master) };
            self.path.pause(u32::MAX, pause, &mut snd);
        }
    }

    /// `SFXObj_Music::InitSFX` 0x82528a00 with `SetSFXObj` 0x824e8ad0: the MAIN state's music
    /// object and position controller, the controller on `mWorldSoundSource`.
    pub fn set_sfx_object(&mut self, vault: &Vault, map: Option<&mut MixMap>) {
        self.object = Some(MusicObject::new(vault, map));
    }

    /// The MAIN state's music part of `CSTATE_Base::UpdateParams`: `SFXCTL_MusicPos` (control 0,
    /// before the objects) on `mWorldSoundSource`, then `SFXObj_Music::UpdateParams` (object 1,
    /// before `SFXObj_Master`).
    #[allow(clippy::too_many_arguments)]
    pub fn update_object(&mut self, t: f32, music_volume: Option<MusicVolumeInfo>, times: [bool; 3], vault: &Vault, mics: &MicrophoneMan, shared: &mut Pos3dShared, map: &mut MixMap) {
        let inputs = self.object_inputs(music_volume, times);
        let source = self.world_sound_source;
        if let Some(object) = self.object.as_mut() {
            object.update_position(t, source.dir, source.pos, mics, shared, map);
            object.update_params(t, inputs, vault, map);
        }
    }

    /// `SFXObj_Music::ProcessUpdate` 0x82515408 (before `SFXObj_Master`'s).
    pub fn process_object(&mut self, map: &MixMap) {
        if let Some(object) = self.object.as_mut() {
            object.process_update(map);
        }
    }

    /// The music object's inputs (`SFXObj_Music::UpdateParams` reads the music system).
    #[must_use]
    pub fn object_inputs(&self, music_volume: Option<MusicVolumeInfo>, times: [bool; 3]) -> sfx::MusicObjectInputs {
        sfx::MusicObjectInputs {
            music_volume,
            times,
            game_music_disabled: self.game_music_disabled(),
            music_playing: self.is_music_playing(),
            challenge_loading: self.challenge_loading,
        }
    }
}

/// `AudioCore::CreateMusicMasterVoice` 0x82a7d658 (`InitializeAudioCore` 0x82a7f008 makes it after
/// the mastering voice): stage 200, `SubMix` "Music Master", `VuMeter` and a `Send` to the master
/// submix, six channels each. Returns `GetMusicMasterSubMix`. The `VuMeter`, which only measures
/// the level `GetMusicVU` reads, is left out.
pub fn create_music_master_voice(core: &mut AudioCore, master: PluginRef) -> Option<PluginRef> {
    use crate::rwac::core::{PlugInConfig, PlugInKind};
    let voice = core.create_voice(
        200,
        "Music Master",
        &[PlugInConfig { kind: PlugInKind::SubMix { name: "Music Master".into() }, output_channels: 6 }, PlugInConfig { kind: PlugInKind::Send, output_channels: 6 }],
    )?;
    core.connect_send(PluginRef { voice, index: 1 }, Some(master));
    Some(PluginRef { voice, index: 0 })
}

/// `FindSongIndex` 0x82a7f850.
fn find_song_index(list: &[u32], song: u32) -> Option<i32> {
    list.iter().position(|&id| id == song).map(|index| index as i32)
}

/// `MusicCore::FindSongStringID` 0x82a7f8c0 for the "IPOD" list: the `ICELITE::StringHash` of
/// `IPOD_<part name>_<suffix>` uppercased, and that label behind the `#` literal prefix.
fn find_song_string_id(song: u32, suffix: &str) -> Option<(u32, String)> {
    let index = usize::try_from(find_song_index(&IPOD_SONGS, song)?).ok()?;
    let label = format!("IPOD_{}_{suffix}", IPOD_SONG_NAMES[index]).to_ascii_uppercase();
    Some((skate2_formats::localization::string_hash(label.as_bytes()), format!("#{label}")))
}

/// `MusicCore::GetSongInfo` 0x82a80480 for the "IPOD" list (`MusicPlayerCmdTranslator::
/// sMethod_GetSongInfo` 0x82638048): the artist, title and album buffers, `#`-prefixed literals.
/// Each is the string `localize` gives for its `FindSongStringID` id (`FormatString`, behind the
/// prefix) when the localizer has it; otherwise the artist and title keep their label
/// (`#Unknown Artist` and `#Unknown Title` for a song outside the list) and the album is `# `.
pub fn music_player_song_info(song: u32, mut localize: impl FnMut(u32) -> Option<String>) -> (String, String, String) {
    let mut field = |suffix: &str| {
        let (id, label) = find_song_string_id(song, suffix).unzip();
        localize(id.unwrap_or(0)).map(|text| format!("#{text}")).ok_or(label)
    };
    let artist = field("ARTIST").unwrap_or_else(|label| label.unwrap_or_else(|| "#Unknown Artist".to_owned()));
    let title = field("TITLE").unwrap_or_else(|label| label.unwrap_or_else(|| "#Unknown Title".to_owned()));
    let album = field("ALBUM").unwrap_or_else(|_| "# ".to_owned());
    (artist, title, album)
}

/// `GetNumSongsInPlayList` 0x824e8d30.
fn num_songs_in_play_list(vault: &Vault, key: u64) -> u32 {
    let list = playlist(key);
    let list_type = playlist_type(vault, key);
    let songs = || if key == 0 { 0 } else { vault.length(list, SONGS) };
    let mut length = songs();
    if list_type == 3 || length == 0 {
        length = if key == 0 { 0 } else { vault.length(list, DJ_SONGS) };
    }
    for kind in [1, 2, 4] {
        if list_type == kind || length == 0 {
            length = songs();
        }
    }
    length
}

/// `Sk8::PointGraphEval::Evaluate(8, x, y, value)` over a 0x40-byte graph (x at byte 0, y at
/// byte 0x20).
fn graph(bytes: &[u8], value: f32) -> f32 {
    let float = |offset: usize| f32::from_bits(u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]));
    let x: Vec<f32> = (0..8).map(|i| float(4 * i)).collect();
    let y: Vec<f32> = (0..8).map(|i| float(0x20 + 4 * i)).collect();
    point_graph(&x, &y, value)
}
