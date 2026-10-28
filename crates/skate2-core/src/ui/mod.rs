//! Original frontend/HUD behavior, independent of platform rendering.
pub mod apt_playback;
pub mod apt_projection;
pub mod apt_jolt;
pub mod apt_timeline;
pub mod phonelist;
pub mod trick_display_hud;
pub mod trick_backend;
pub mod trick_display;
pub mod trick_context;

pub mod hom_scoring;

pub mod pause_menu;
pub mod main_menu;
pub mod dlc_menu;
pub mod profile_name;
pub mod game_config;
pub mod controller_config;
pub mod online_config;
pub mod online_match;
pub mod online_hub;
pub mod lobby_browser;
pub mod update_prompt;
pub mod trick_guide;
pub mod freeskate_option;
pub mod party_play;
pub mod sponsor_browser;
pub mod slappys_blog;
pub mod skate_profile;
pub mod options;
pub mod music_player;
pub mod movie_player;
pub mod replay_editor;
pub mod skate_reel;
pub mod chyron;
pub mod chase_arrows;

pub mod map_menu;

pub mod map_viewport;
pub mod minimap;
pub mod edit_skater;
pub mod edit_skater_presentation;
pub(crate) mod frontend_fade;

/// A front-end sound request by its `FE_SOUND_*` global: `FE::GlobalFEPlaySound` 0x826150b0,
/// one-shot or looping, or `FE::GlobalFEStopSound` 0x826151a0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeSoundRequest {
    /// A native GlobalFEPlaySound collection key, without looping.
    PlayKey(u64),
    /// Native GlobalFEPlaySound/GlobalFEStopSound collection keys.
    LoopKey(u64),
    StopKey(u64),
    Play(&'static str),
    Loop(&'static str),
    Stop(&'static str),
}

/// An in-game front-end state as `StateManagerFrontEndBase::UpdateFEAudioState` 0x82d67948 reports
/// it: the state id (the audio system's `mSk8HUDState`) and its `gFEStateInfo` 0x82fded80 row's
/// `mFEMusicState`, and `mbPause`, under which `PauseMan` holds `kPAUSE_STATE_FRONTEND`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrontendAudioState {
    pub state: i32,
    pub music_state: i32,
    pub paused: bool,
}

impl FrontendAudioState {
    /// The HUD stack's state in gameplay, which `UpdateFEAudioState(mIdleHUDState)` restores.
    pub const FREE_SKATE: Self = Self { state: 0x01, music_state: 4, paused: false };
    pub const START_HUB: Self = Self { state: 0x07, music_state: 3, paused: true };
    pub const CAC: Self = Self { state: 0x13, music_state: 1, paused: true };
    pub const CHALLENGE_MAP: Self = Self { state: 0x15, music_state: 9, paused: true };
    pub const JACKED: Self = Self { state: 0x2A, music_state: 8, paused: true };
    pub const OPTIONS: Self = Self { state: 0x49, music_state: 11, paused: true };
    pub const MUSIC_PLAYER: Self = Self { state: 0x1E, music_state: 12, paused: true };
    pub const MOVIE_PLAYER: Self = Self { state: 0x1C, music_state: 10, paused: true };
    pub const CREDITS: Self = Self { state: 0x1D, music_state: 10, paused: true };
    pub const EXTRAS: Self = Self { state: 0x18, music_state: 0, paused: false };
    pub const KEYBOARD: Self = Self { state: 0x4B, music_state: 9, paused: false };
    pub const NIS: Self = Self { state: 0x02, music_state: 9, paused: false };
    pub const REPLAY_EDITOR: Self = Self { state: 12, music_state: 9, paused: false };
    /// gFEStateInfo82fded80 row18: pause, transition, hide HUD, FE music1.
    pub const TRICK_GUIDE: Self = Self { state: 18, music_state: 1, paused: true };
    pub const REPLAY_FOOTAGE: Self = Self { state: 56, music_state: 9, paused: true };
    pub const ONLINE_OPTIONS: Self = Self { state: 23, music_state: 0, paused: false };
    pub const ONLINE_HUB: Self = Self { state: 44, music_state: 3, paused: true };
    /// `FrontEndState_FreeskateActivitySignUp`'s row 67: pause, the HUD hidden, FE music 4.
    pub const FREESKATE_ACTIVITY_SIGN_UP: Self = Self { state: 67, music_state: 4, paused: true };
    pub const UNRANKED_FREESKATE: Self = Self { state: 68, music_state: 9, paused: true };
    pub const UNRANKED_CUSTOM_MATCH: Self = Self { state: 69, music_state: 9, paused: true };
    pub const SKATE_REEL_GENERAL: Self = Self { state: 55, music_state: 9, paused: true };
    pub const SKATE_REEL_PHOTO: Self = Self { state: 58, music_state: 9, paused: true };
    pub const END_CHALLENGE_OPTIONS: Self = Self { state: 0x08, music_state: 4, paused: true };
    pub const HOM_RESULTS:Self=Self{state:9,music_state:4,paused:true};
    pub const ANNALS_OF_MEAT:Self=Self{state:77,music_state:9,paused:true};
    pub const UNLOCK_OVERLAY: Self = Self { state: 0x0A, music_state: 4, paused: true };
    pub const SPONSOR_OVERLAY: Self = Self { state: 0x0B, music_state: 4, paused: true };
    pub const SIGN_UP_POSTER: Self = Self { state: 0x20, music_state: 4, paused: false };
    pub const PRESS_A: Self = Self { state: 0x26, music_state: 4, paused: false };
    pub const CHANGE_GAME: Self = Self { state: 0x27, music_state: 9, paused: true };
    pub const SKATE101: Self = Self { state: 0x4F, music_state: 9, paused: true };
    pub const SLAPPYS_BLOG: Self = Self { state: 0x3D, music_state: 9, paused: true };
    pub const SLAPPYS_ARTICLES: Self = Self { state: 0x3E, music_state: 9, paused: true };
    pub const SLAPPYS_COVERS: Self = Self { state: 0x3F, music_state: 9, paused: true };
    pub const SKATE_PROFILE: Self = Self { state: 0x2D, music_state: 9, paused: true };
}

pub mod extras;
