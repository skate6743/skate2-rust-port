//! A basic NIS from `RunBasicNIS` to `EndNIS`. The simulation side is
//! `Sk8::NISManager` (`RunBasicNIS` `0x828a8d70`, `Update` `0x828a67e8`, `EndNIS`
//! `0x828a3200`); the presentation side is `Sk8::Scene::SceneManagerImpl`
//! (`StartNIS` `0x8286b8b8`, `UpdateNIS` `0x82876608`, `UpdateLoadState`
//! `0x82875bc0`, `StopNIS` `0x8286e438`, `StopDestroyUnloadNIS` `0x8286dfe0`) with
//! its `ScenePlayer` and `Scene` (`Init` `0x82877ac8`, `Update` `0x82870bb8`). The
//! two sides talk through the simulation and presentation message queues; a
//! simulation update dispatches its queue before `NISManager::Update`
//! (`SimController::Update1` `0x828a72c8`), a presentation frame runs
//! `SceneManager::UpdateFrameBegin`, then `Presentation::Update` (its queue, then
//! the transition effect), then `UpdateFrameEnd` (`GameModule::Render` `0x827320f0`).

use super::{
    audio::AudioInterface,
    fe_overlays::{FeNisOverlays, STATE_MAX, STATE_UNKNOWN, ScreenCommand},
    characters::{SceneCharacter, SceneCharacterSource, instance_characters},
    directory::{SceneDirectory, SceneProperties, scene_hash, scene_type},
    subtitles::{SubTitler, SubtitleKey},
    transition::{Callback, Direction, PresTransitions},
};
use crate::{actor_face_playback::presentation::move_root::matrix_inverse::try_inverse, reckoning_numeric::HostReckoningNumeric};
use skate2_formats::ice_camera::{Mover, Playback, compute_transform, vmx_cross, vmx_normalize};
use std::{collections::VecDeque, sync::Arc};

/// `kPAUSE_STATE_NIS`: `PauseSim` (`0x828a1070`) pushes it, `ResumeSim`
/// (`0x828a1170`) pulls it. Its flags (`0xFFFFFFF1`) pause the simulation.
pub const PAUSE_STATE_NIS: u32 = 11;

/// `FrontEndState_NIS`: FE state 2.
pub const FE_STATE_NIS: u32 = 2;

/// `eNISSpecialFlags`: `NIS_SPECIALFLAG_NONE`.
pub const SPECIAL_FLAG_NONE: i32 = 0;
/// `NIS_SPECIALFLAG_FINALHIGHLIGHT`: a highlight reel's highlight not stitched to the next.
pub const SPECIAL_FLAG_FINAL_HIGHLIGHT: i32 = 1;
/// `NIS_SPECIALFLAG_HALLOFMEAT`: a Hall of Meat group's highlight.
pub const SPECIAL_FLAG_HALL_OF_MEAT: i32 = 3;

/// `NISPlaybackOverrides` as a basic NIS sets them; their random playback speed and camera
/// track modes stay off and no player is focused.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaybackOverrides {
    /// `useLocationValues` with `locationPosition` and `locationDirection`: the scene's
    /// location and forward direction in place of its directory record's.
    pub location: Option<[[f32; 3]; 2]>,
    /// `useStreamPositionOnReturn` with `streamPosition`: where the streamer is restored before
    /// the scene fades back out.
    pub stream_position: Option<[f32; 3]>,
    /// `playerIndexVector`: the players whose skaters the scene's "dynamic" records borrow.
    pub player_indexes: Vec<i32>,
}

/// `eNISKillType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KillType {
    None,
    Completed,
    Input,
    Message,
    Error,
}

/// `ICEManager::Update` (`0x82a3b5f0`) for the active scene.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrackUpdate {
    /// `ICEScene::HandleCameraFinished`: the take, or the assembly take's
    /// last interval, is done.
    pub finished: bool,
    /// An assembly take cut to another sub-take: the scene's time becomes
    /// `fSubParameter * subLength` (`ICEScene::SetTime`).
    pub set_time: Option<f32>,
}

/// What `ICECameraMover::UpdateFrameEnd` writes to `CameraPresentation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IceFrame {
    /// `SetCameraMatrix` -> `SetICEMatrix` (Skate space, rows right, up,
    /// forward, position).
    pub matrix: [f32; 16],
    /// `SetFieldOfView`, degrees.
    pub fov: f32,
    /// `ICEUserChannelLens`'s near plane, when it stores one.
    pub near_plane: Option<f32>,
    /// `ICEUserChannelBloom`: the take's bloom threshold and intensity
    /// under its override.
    pub bloom: Option<[f32; 2]>,
    /// `ICEUserChannelFade`'s `SetFadeColor` (red, green, blue, amount),
    /// when it makes the call.
    pub fade: Option<[f32; 4]>,
    pub postfx: PostfxChannels,
}

/// The post-fx channels' take values under their overrides; `None` while
/// an override is off, when the handler writes the default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PostfxChannels {
    pub tonemap: Option<f32>,
    /// `fisheyeWarpMultiplier`.
    pub fisheye: Option<f32>,
    /// `nisFarStart`, `nisFarEnd`.
    pub near_far: Option<[f32; 2]>,
    /// Near, far.
    pub brightness: Option<[f32; 2]>,
    /// Near, far: contrast and midpoint.
    pub contrast: Option<[[f32; 2]; 2]>,
    /// Near, far.
    pub saturate: Option<[f32; 2]>,
    /// Near, far `saturateColour`.
    pub tint: Option<[[f32; 3]; 2]>,
    pub scanline: Option<f32>,
    pub radial_blur: Option<f32>,
}

impl PostfxChannels {
    /// The channels' readers on a take's current values.
    #[must_use]
    pub fn read(playback: &Playback) -> Self {
        Self {
            tonemap: playback.tonemap(),
            fisheye: playback.fisheye(),
            near_far: playback.near_far(),
            brightness: playback.brightness(),
            contrast: playback.contrast(),
            saturate: playback.saturate(),
            tint: playback.tint(),
            scanline: playback.scanline(),
            radial_blur: playback.radial_blur(),
        }
    }
}

/// One side of the NIS grade (`nisNear` / `nisFar`): what
/// `PostFX::SetParameters` (`0x8284bbf0`) hands `ApplyFX_NEW`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    pub brightness: f32,
    pub contrast: f32,
    pub midpoint: f32,
    /// `saturateIntensity`.
    pub saturate: f32,
    /// `saturateColour`.
    pub colour: [f32; 3],
}

/// The ICE post-fx state the channels leave: `mICEPostfxData`'s fields the
/// GPU path reads and the ICE general block's `screenFadeColour`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IcePostfx {
    pub bloom: [f32; 2],
    pub fade: [f32; 4],
    pub bloom_anim: BloomAnim,
    /// `nisNear`, `nisFar`.
    pub grade: [Grade; 2],
    /// `nisFarStart`, `nisFarEnd`.
    pub far: [f32; 2],
    /// `tonemapValue`.
    pub tonemap: f32,
    /// `fisheyeWarpMultiplier`.
    pub fisheye: f32,
    /// `scanlineOffset`.
    pub scanline: f32,
    pub radial_blur: f32,
}

impl Default for IcePostfx {
    /// `PostfxPresentation::SetDefaults` (`0x827eff00`): threshold 1.4,
    /// intensity 0.5, both grades neutral (contrast 1 about 0.5, white
    /// colour), far start 150 and end 40, warp 1, the rest 0;
    /// `ICEUserChannelFade::Init` (`0x826c80b0`) clears the fade once, when
    /// the channel is registered.
    fn default() -> Self {
        let grade = Grade { brightness: 0.0, contrast: 1.0, midpoint: 0.5, saturate: 0.0, colour: [1.0; 3] };
        Self {
            bloom: [1.4, 0.5],
            fade: [0.0; 4],
            bloom_anim: BloomAnim::default(),
            grade: [grade; 2],
            far: [150.0, 40.0],
            tonemap: 0.0,
            fisheye: 1.0,
            scanline: 0.0,
            radial_blur: 0.0,
        }
    }
}

/// `mICEPostfxData`'s bloom pulse: `bloomAnimBaseValue`, `bloomAnimValue`,
/// `bloomAnimUpTime`, `bloomAnimDownTime` and `bloomAnimCurrentTime`,
/// negative while idle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BloomAnim {
    pub base: f32,
    pub value: f32,
    pub up: f32,
    pub down: f32,
    pub time: f32,
}

impl Default for BloomAnim {
    /// `SetDefaults`: base and value 0, the times -1.
    fn default() -> Self {
        Self { base: 0.0, value: 0.0, up: -1.0, down: -1.0, time: -1.0 }
    }
}

impl IcePostfx {
    /// One take frame's ICE user channels. `ICEUserChannelBloom::UpdateFrameEnd`
    /// (`0x826c88e0`): under the override the take's; else the default intensity, and the
    /// default threshold unless a pulse runs. `ICEUserChannelFade` writes only when it calls
    /// `SetFadeColor`. Tonemap, FishEye, NearFar, Brightness, Contrast, Saturate, Tint,
    /// Scanline and RadialBlur write the take under their override, else the defaults.
    pub fn apply_take(&mut self, bloom: Option<[f32; 2]>, fade: Option<[f32; 4]>, channels: PostfxChannels) {
        let defaults = Self::default();
        if let Some(bloom) = bloom {
            self.bloom = bloom;
        } else {
            if self.bloom_anim.time < 0.0 {
                self.bloom[0] = defaults.bloom[0];
            }
            self.bloom[1] = defaults.bloom[1];
        }
        if let Some(fade) = fade {
            self.fade = fade;
        }
        self.tonemap = channels.tonemap.unwrap_or(defaults.tonemap);
        self.fisheye = channels.fisheye.unwrap_or(defaults.fisheye);
        self.far = channels.near_far.unwrap_or(defaults.far);
        for (side, grade) in self.grade.iter_mut().enumerate() {
            let default = defaults.grade[side];
            grade.brightness = channels.brightness.map_or(default.brightness, |b| b[side]);
            [grade.contrast, grade.midpoint] = channels.contrast.map_or([default.contrast, default.midpoint], |c| c[side]);
            grade.saturate = channels.saturate.map_or(default.saturate, |s| s[side]);
            grade.colour = channels.tint.map_or(default.colour, |t| t[side]);
        }
        self.scanline = channels.scanline.unwrap_or(defaults.scanline);
        self.radial_blur = channels.radial_blur.unwrap_or(defaults.radial_blur);
    }

    /// `PostfxPresentation::Update` (`0x827efe58`): the pulse takes the
    /// threshold from its base to its value over the up time, back over the
    /// down time, then rests at the base.
    fn update(&mut self, dt: f32) {
        let a = &mut self.bloom_anim;
        let t = a.time;
        if t < 0.0 {
            return;
        }
        if t < a.up {
            a.time = t + dt;
            self.bloom[0] = (a.value - a.base).mul_add(t / a.up, a.base);
        } else if t < a.down + a.up {
            a.time = t + dt;
            self.bloom[0] = (a.base - a.value).mul_add((t - a.up) / a.down, a.value);
        } else {
            a.time = -1.0;
            self.bloom[0] = (a.base - a.base).mul_add(0.0, a.base);
        }
    }
}

/// `CameraPresentation`'s ICE camera: `mICEMatrix`, `mICEFOV` and
/// `mICENearPlane`. `GetMatrix` (`0x827f6308`), `GetFOVY` (`0x827f64b0`)
/// and `GetNearPlane` (`0x827ef8d8`) return them while `mICECameraActive`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NisCamera {
    /// `None` while the matrix is the game camera's, which `Scene::Init`
    /// copies when the scene has no camera take.
    pub matrix: Option<[f32; 16]>,
    /// Degrees.
    pub fov: f32,
    pub near_plane: f32,
}

impl Default for NisCamera {
    /// The `CameraPresentation` constructor (`0x827f2130`): 30 degrees and a
    /// 0.2 near plane. Scenes leave their last values behind; the
    /// Create-A-Skater cameras write them too.
    fn default() -> Self {
        Self { matrix: None, fov: 30.0, near_plane: 0.2 }
    }
}

/// Where `Scene::Init` froze the stream focus (`kNISFreezesStreamer`, on by
/// default: `MoveStreamFocus` to the camera position, then a
/// `FrozenStreamerController`, whose update does nothing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrozenFocus {
    /// The ICE matrix's position.
    At([f32; 3]),
    /// The game camera's position (the scene had no camera take).
    GameCamera,
}

/// The playback take `ICEManager` drives for the scene.
pub trait CameraTrack: Send {
    /// `ICEManager::Update` at the scene's elapsed time.
    fn update(&mut self, elapsed: f32) -> TrackUpdate;
    /// `ICECameraMover::UpdateSimTime` (`0x82a36d70`): `TIME_SCALE` (element 25)
    /// times 0.01, clamped to [0.01, 1]; `SetSimTimeMultiplier` makes it the
    /// scene player's `mSceneTimeMult`.
    fn sim_time(&self) -> f32;
    /// Host policy for Hall of Meat: use 1.0 for both the scene multiplier
    /// and the camera mover's own time scale. Other tracks keep authored timing.
    fn disable_slow_motion(&mut self) {}
    /// `ICECameraMover::UpdateFrameEnd` for the scene's transform, with what
    /// its anchor and player spaces resolve against and the mover's state.
    fn camera(&mut self, scene: &SceneFrame, anchors: &CameraAnchors, mover: &mut Mover, dt: f32) -> Option<IceFrame>;
    /// `ICECameraMover::UpdateConstraints` (`0x82a36e38`)'s read after the shake: the take's
    /// `AVOID_STATIC_OBJECTS` (element 95).
    fn avoid_static_objects(&self) -> bool {
        false
    }
    /// `SceneCamera::GetCameraComponentsAtTimeZero` (`0x828647f0`): the
    /// take's eye and look positions (Skate space) before it plays.
    fn time_zero(&self, _scene: &SceneFrame, _anchors: &CameraAnchors) -> Option<([f32; 3], [f32; 3])> {
        None
    }
    /// Why the camera could not be built, once it failed.
    fn camera_error(&self) -> Option<&str> {
        None
    }
    /// `ICEUserChannelOverlays::UpdateFrameBegin`'s read: the overlay
    /// (element 107) of channel 7's current interval, none without one.
    fn overlay(&self) -> Option<i64> {
        None
    }
    /// Channel 34's current interval (`ICEUserChannelFreeze`), none without
    /// one.
    fn freeze_interval(&self) -> Option<usize> {
        None
    }
    /// Interval `i`'s freeze command and frame count
    /// (`GetValueInt(141, i)`, `GetValueInt(142, i)`).
    fn freeze_command(&self, _interval: usize) -> (u32, u32) {
        (0, 0)
    }
    /// Channel 30's current interval (`ICEUserChannelAudioEffect`), none
    /// without one.
    fn audio_effect_interval(&self) -> Option<usize> {
        None
    }
    /// Interval `i`'s audio effect command and id
    /// (`GetValueInt(132, i)`, `GetValueInt(133, i)`).
    fn audio_effect_command(&self, _interval: usize) -> (u32, u32) {
        (0, 0)
    }
    /// Channel 31's current interval (`ICEUserChannelAudioMusic`), none
    /// without one.
    fn audio_music_interval(&self) -> Option<usize> {
        None
    }
    /// Interval `i`'s music command and id (`GetValueInt(134, i)`,
    /// `GetValueInt(135, i)`).
    fn audio_music_command(&self, _interval: usize) -> (u32, i32) {
        (0, 0)
    }
    /// Channel 32's current interval (`ICEUserChannelAudioSpeech`), none
    /// without one.
    fn audio_speech_interval(&self) -> Option<usize> {
        None
    }
    /// Interval `i`'s speech command, character and id
    /// (`GetValueInt(136, i)`, `GetValueInt(138, i)`, `GetValueInt(137, i)`).
    fn audio_speech_command(&self, _interval: usize) -> (u32, i32, i32) {
        (0, 0, 0)
    }
    /// Channel 33's current interval (`ICEUserChannelAudioPost`), none
    /// without one.
    fn audio_post_interval(&self) -> Option<usize> {
        None
    }
    /// Interval `i`'s post command and id (`GetValueInt(139, i)`,
    /// `GetValueInt(140, i)`).
    fn audio_post_command(&self, _interval: usize) -> (u32, i32) {
        (0, 0)
    }
    /// The camera take's `fParameter`.
    fn parameter(&self) -> f32 {
        0.0
    }
    /// `SceneCamera::GetTimeAtPhotoShot` (`0x82864d30`): the take's length (a
    /// replay-controlled scene's, as `ConnectScene` wrote it) times the start of the
    /// `index`-th `PhotoShot` freeze interval; 0 without one.
    fn photo_shot_time(&self, _index: usize) -> f32 {
        0.0
    }
}

/// The scene directory and ICE cameras `NISManager::GetPhotoCount` (`0x828a0e70`) reads.
#[derive(Clone)]
pub struct PhotoCounter {
    directory: Arc<SceneDirectory>,
    tracks: Arc<dyn CameraTracks>,
}

impl PhotoCounter {
    /// The scene's camera take's `PhotoShot` freeze intervals; 0 for an unknown scene.
    #[must_use]
    pub fn count(&self, scene: &str) -> i32 {
        let hash = scene_hash(scene);
        let Some(props) = self.directory.properties(hash) else { return 0 };
        self.tracks.photo_count(props, self.directory.camera_track(hash))
    }
}

impl std::fmt::Debug for PhotoCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PhotoCounter")
    }
}

/// What the camera mover's anchor space resolves against (Skate space).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CameraAnchors {
    /// `User_ICECameraAnchor::GetPosition`: the local skater's root.
    pub anchor: Option<[f32; 3]>,
    /// The root's X, Y and Z axes, which `User_ICECameraAnchor::GetOrientation`
    /// (`0x826cd228`) and `GetAvatarOrientation` index 0 read.
    pub axes: Option<[[f32; 3]; 3]>,
    /// `SkaterPresentation::GetHipsPos` (`0x827e3ef0`) of the local skater.
    pub hips: Option<[f32; 3]>,
    /// `SkaterPresentation::GetHipsMatrix` (`0x826c70d8`)'s Z row of the local skater.
    pub hips_forward: Option<[f32; 3]>,
    /// `CameraPresentation::GetReplayMatrix` (`0x826cf400`): the camera presentation's current
    /// matrix (Skate rows): in a replay the recorded game camera it restored, else the game
    /// camera's.
    pub replay: Option<[f32; 16]>,
    /// Each scene entity's `GetObjectTransform` translation, once it has a
    /// render entity.
    pub entities: Vec<Option<[f32; 3]>>,
}

/// The scene frame `Scene::Init` keeps (`mLocation`, `mOrientation`, `mNormal`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SceneFrame {
    pub location: [f32; 3],
    pub orientation: [f32; 3],
    pub normal: [f32; 3],
}

/// `ICEManager::ChooseCameraPlaybackTrack` (`0x82a3bf18`) for a scene.
pub trait CameraTracks: Send + Sync {
    /// The scene's group take for `track` (`SetPlaybackCamera` `0x82a3bd60`:
    /// assembly takes first, then plain takes); `None` when there is none
    /// (`mCameraTrackFound` stays false). With `length` the take's length is
    /// overwritten (`ICESceneBridge::ConnectScene` for a replay-controlled
    /// scene). The second value is the scene time an assembly take's first
    /// sub-take sets.
    fn connect(&self, scene: &SceneProperties, track: u8, length: Option<f32>) -> Option<(Box<dyn CameraTrack>, Option<f32>)>;
    /// `SceneCamera::GetListOfOverlays` (`0x82864f10`) for the scene's
    /// camera track; none when no take lists an overlay.
    fn overlays(&self, _scene: &SceneProperties, _track: u8) -> Option<Vec<u8>> {
        None
    }
    /// `ScenePlayer::AudioQueueForStart`'s read of the scene's camera track:
    /// its assembly or take at parameter 0; none without one.
    fn start_audio(&self, _scene: &SceneProperties, _track: u8) -> Option<StartAudio> {
        None
    }
    /// `NISManager::GetPhotoCount`'s read of the scene's camera track: its `PhotoShot` (4)
    /// freeze intervals.
    fn photo_count(&self, _scene: &SceneProperties, _track: u8) -> i32 {
        0
    }
    /// `SceneCamera::GetFixedFramePhotoCount` (`0x82864b98`) for the scene's camera track: its
    /// `PhotoShot_FixedFrame` (5) freeze intervals.
    fn fixed_frame_photo_count(&self, _scene: &SceneProperties, _track: u8) -> i32 {
        0
    }
}

/// The audio a scene's camera take starts with (`ICETake::GetValueInt` at
/// parameter 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartAudio {
    /// The post's command and id (elements 139 and 140).
    pub post_command: u32,
    pub post_id: i32,
    /// The speech's command, id and character (elements 136, 137 and 138).
    pub speech_command: u32,
    pub speech_id: i32,
    pub speech_character: i32,
}

/// One ICE audio user channel's walk state (`mPreviousInterval`, constructor
/// 0xFFFF, and `mIsTakeChange`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioChannel {
    previous: u16,
    take_change: bool,
}

impl Default for AudioChannel {
    fn default() -> Self {
        Self { previous: 0xffff, take_change: false }
    }
}

impl AudioChannel {
    /// `UpdateFrameBegin`'s walk (`ICEUserChannelAudioEffect` `0x826ce9f8`,
    /// `ICEUserChannelAudioMusic` `0x826cebd8`, `ICEUserChannelAudioSpeech`
    /// `0x826cedb8`, `ICEUserChannelAudioPost` `0x826cefc8`): every interval
    /// crossed fires once, in order; the first after a take change and
    /// backward jumps fire nothing.
    fn walk(&mut self, current: u16) -> Vec<usize> {
        let mut fired = Vec::new();
        if self.previous == 0xffff {
            if self.take_change {
                self.take_change = false;
            } else {
                fired.push(usize::from(current));
            }
        } else {
            let mut i = self.previous.wrapping_add(1);
            while i <= current {
                fired.push(usize::from(i));
                i = i.wrapping_add(1);
                if i == 0 {
                    break;
                }
            }
        }
        self.previous = current;
        fired
    }
}

/// `SceneManagerImpl::UpdateFrameBegin`'s audio clock (`kNISUsesTheAudioClock`
/// 1): `gInitialUpdate`, `gLastAudioTime` and `gFrameAudioBeginDelta`.
#[derive(Clone, Copy, Debug)]
struct AudioClock {
    initial_update: bool,
    last_time: f64,
    frame_delta: f64,
}

impl Default for AudioClock {
    fn default() -> Self {
        Self { initial_update: true, last_time: 0.0, frame_delta: 0.0 }
    }
}

/// `World::LineCollisionTest` from a start to an end point: the hit's
/// position and normal, or none.
pub type GroundLine<'a> = dyn FnMut([f32; 4], [f32; 4]) -> Result<Option<([f32; 3], [f32; 3])>, String> + 'a;

/// `Presentation::AddLineTest` / `GetLineTestResult` from a start to an end point with a
/// radius: the hit's line parameter, or none.
pub type CameraLine<'a> = dyn FnMut([f32; 4], [f32; 4], f32) -> Result<Option<f32>, String> + 'a;

/// `kICECameraRadius` (default 0.14), `kICELOSSmoothing` (0.01) and `kICEMinCameraDistance` (0.6).
const ICE_CAMERA_RADIUS: f32 = 0.14;
const ICE_LOS_SMOOTHING: f32 = 0.01;
const ICE_MIN_CAMERA_DISTANCE: f32 = 0.6;

/// `User_ICEWorldManager` (ctor `0x826d1900`): the scene camera's smoothed line of sight to
/// its anchor and the switch to the replay camera.
#[derive(Clone, Copy, Debug)]
struct IceWorld {
    /// `mSmoothedLOSTime`.
    smoothed_los_time: f32,
    /// `mIsUsingReplayCamera`.
    using_replay_camera: bool,
}

impl Default for IceWorld {
    /// `Reset` (`0x826cfd68`) through `ResetData` (`0x826cd3a8`).
    fn default() -> Self {
        Self { smoothed_los_time: 1.0, using_replay_camera: false }
    }
}

impl IceWorld {
    /// `ResolveAgainstStaticObjects` (`0x826d0268`, `kICEEnableCameraCollision` on): the
    /// replay camera's matrix once it took over; otherwise `HandleCollision` (`0x826cff00`)
    /// from the anchor (`FixUpAnchorPosition`: the local skater's hips, else the camera
    /// anchor) to the camera, the line of sight's parameter smoothed and the camera moved along
    /// it, then `TestMinimumDistance` (`0x826c7698`) hands over to the replay camera when the
    /// camera ends up closer than the minimum.
    fn resolve(
        &mut self,
        matrix: &mut [f32; 16],
        anchor: [f32; 3],
        replay: Option<[f32; 16]>,
        line: &mut CameraLine<'_>,
    ) -> Result<(), String> {
        if self.using_replay_camera {
            if let Some(replay) = replay {
                *matrix = replay;
                for w in [3, 7, 11] {
                    matrix[w] = 0.0;
                }
                matrix[15] = 1.0;
            }
            return Ok(());
        }
        let end = [matrix[12], matrix[13], matrix[14]];
        let delta: [f32; 3] = std::array::from_fn(|lane| end[lane] - anchor[lane]);
        let start = [anchor[0], anchor[1], anchor[2], 1.0];
        let hit = line(start, [end[0], end[1], end[2], 1.0], ICE_CAMERA_RADIUS)?;
        let moved = if let Some(parameter) = hit {
            if self.smoothed_los_time >= parameter {
                self.smoothed_los_time = parameter;
            }
            true
        } else {
            let smoothed = ICE_LOS_SMOOTHING + self.smoothed_los_time;
            self.smoothed_los_time = if 1.0 - smoothed >= 0.0 { smoothed } else { 1.0 };
            self.smoothed_los_time < 1.0
        };
        if moved {
            for lane in 0..3 {
                matrix[12 + lane] = delta[lane].mul_add(self.smoothed_los_time, anchor[lane]);
            }
        }
        let offset: [f32; 3] = std::array::from_fn(|lane| matrix[12 + lane] - anchor[lane]);
        // ICE x, z, y are Skate z, y, x.
        if (offset[2] * offset[2] + (offset[1] * offset[1] + offset[0] * offset[0])).sqrt() < ICE_MIN_CAMERA_DISTANCE {
            self.using_replay_camera = true;
        }
        Ok(())
    }
}

/// Requests the NIS makes of the rest of the game.
#[derive(Clone, Debug, PartialEq)]
pub enum NisEvent {
    /// `PauseSim`: push `kPAUSE_STATE_NIS`.
    PauseSim,
    /// `ResumeSim`: pull `kPAUSE_STATE_NIS`.
    ResumeSim,
    /// `TeleportToLastBreadCrumb` (`0x828a0db8`) for a story or go-to scene:
    /// `cTeleportManager::TeleportLocalToLastBreadcrumb` unless it needs a
    /// load screen.
    TeleportToLastBreadcrumb,
    /// `EndNIS`: `cMsgNISChallengeNotify{NIS_CHALLENGE_NOTIFY_COMPLETED}` then
    /// `cMsgNISComplete` to the challenge system.
    Completed,
    /// `Scene::Update` at the end of a photo outro's camera:
    /// `cMsgNISChallengeNotify{NIS_FE_NOTIFY_ENTER_PHOTOVIEW}`.
    EnterPhotoView,
    /// `EndNIS`: `TransitionManager::BeginTransition(mExitNISTransition,
    /// TRANSITION_END_PARAM_ACTIVATEONLY)` for a type other than NONE (which
    /// does nothing).
    ExitTransition(i32),
    /// `cMsgNISLoadingOverlay`: the FE loading tag.
    LoadingOverlay(bool),
    /// `FrontEndState_NIS::OnEnterState(eSk8HUDNISState_OpenResultsScreen)`
    /// (`0x826675ec..0x82667628`): `PushState(8)` over the NIS state.
    OpenResultsScreen,
    /// `ICEUserChannelFreeze::FireEvent` command 6 (Killable) outside the
    /// ICE editor: `cMsgNISChallengeNotify{NIS_FE_NOTIFY_PLAYING_KILLABLE}`
    /// to the challenge system.
    PlayingKillable,
    /// A freeze command the port does not run: 8 (`LoadScreenAnimChange`).
    FreezeCommandNotPorted(u32),
}

/// `s_fBloom` (`0x82fe2d8c`), `s_fBloomUpTime` (`0x82fe2d88`) and `s_fBloomBackTime`
/// (`0x82fe2d84`): a fixed-frame photo's bloom pulse.
const PHOTO_BLOOM: f32 = -0.6;
const PHOTO_BLOOM_UP_TIME: f32 = 0.15;
const PHOTO_BLOOM_BACK_TIME: f32 = 0.1;
/// `PresentationEntity::GetMinPixelSize` (`0x827d0ac8`).
const MIN_PIXEL_SIZE: f32 = 19.0;

/// `Sk8::NISManager::EResolvingStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resolving {
    None,
    ForPlay,
    Reloading,
}

#[derive(Clone, Debug, PartialEq)]
enum ToPresentation {
    /// `cMsgVisualTransitionBegin{TRANSITION_TYPE_NISLOADSCREEN}`.
    VisualTransitionBegin,
    /// `cMsgNISEnterBasic` (`StartPendingNIS` `0x828a2d30`) with the manager's playback overrides.
    EnterBasic { scene: String, hold_camera: bool, overrides: PlaybackOverrides },
    /// `cMsgNISEnterReplay` (`RunReplayNIS` `0x828a3c70`): the replay recorder's frames the
    /// scene plays over, its playback overrides and special flags.
    EnterReplay { scene: String, start: u32, end: u32, hold_camera: bool, overrides: PlaybackOverrides, special_flags: i32 },
    /// `cMsgEnterRealTime` (`Presentation::EnterRealTime` `0x827f1130`): a real-time clock in
    /// place of the replay scene's.
    EnterRealTime,
    /// `cMsgNISSkip`.
    Skip,
    /// `cMsgNISKill`.
    Kill,
    /// `cMsgNISWorldCollisionResponse`: the ground under the scene.
    WorldCollisionResponse { location: [f32; 3], normal: [f32; 3] },
    /// `cMsgNISLocationListSnapResponse`: the snapped world locations.
    LocationListSnapResponse(Vec<[f32; 3]>),
    /// `cMsgNISFEPreloaded` (`NISManager::FEPreloaded` `0x828a3050`).
    FePreloaded,
}

#[derive(Clone, Debug, PartialEq)]
enum ToSimulation {
    /// `cMsgVisualTransitionComplete`.
    VisualTransitionComplete,
    /// `cMsgNISFEPushState{fLoadScreenRequired = 0}`.
    FePushState,
    /// `cMsgNISMgrPlayback`.
    Playback,
    /// `cMsgNISTransitionInBegin`.
    TransitionInBegin,
    /// `cMsgNISTransitionOutComplete`.
    TransitionOutComplete,
    /// `cMsgNISMgrComplete`.
    Complete,
    /// `cMsgNISLoadingOverlay`.
    LoadingOverlay(bool),
    /// `cMsgSetSubtitle` (`SubTitler::Update` `0x8269fbd0`, to the front
    /// end's `EventBroadcaster`) with its HAL id.
    SetSubtitle(u32),
    /// `cMsgNISWorldCollisionQuery` (kind `0x27A08C41`) at the scene
    /// location.
    WorldCollisionQuery([f32; 4]),
    /// `cMsgNISLocationListSnapQuery` (kind `0xFE94DD4B`): world locations.
    LocationListSnapQuery(Vec<[f32; 4]>),
    /// `cMsgNISFERequestPreload` (kind `0x77645B8F`): the overlay states to
    /// preload.
    FeRequestPreload(Vec<u8>),
    /// `cMsgNISFEEvent` (kind `0x4EB59F71`): an overlay state.
    FeEvent(i32),
}

/// `NISManager::GetGroundPositionAndOrientationAtLocation` (`0x828a1298`):
/// a line from one unit above the location to 500 below that
/// (`vaddfp`, then `vsubfp` 0x828a131c..20) without fatness; a miss
/// answers the origin and (0, 0, -1).
fn ground_position_and_orientation(line: &mut GroundLine<'_>, location: [f32; 4]) -> Result<([f32; 3], [f32; 3]), String> {
    let start: [f32; 4] = std::array::from_fn(|lane| location[lane] + [0.0, 1.0, 0.0, 0.0][lane]);
    let end = std::array::from_fn(|lane| start[lane] - [0.0, 500.0, 0.0, 0.0][lane]);
    Ok(line(start, end)?.unwrap_or(([0.0; 3], [0.0, 0.0, -1.0])))
}

/// A point `(x, y, z, 1)` through a row-major matrix: the first row's
/// product rounded, the others fused in row order (`vmulfp`, then
/// `vmaddfp`; `SceneLocationSnapper` 0x8286d994..9ac, 0x8286af34..44).
fn transform_point(m: [f32; 16], p: [f32; 3]) -> [f32; 4] {
    std::array::from_fn(|c| m[12 + c].mul_add(1.0, m[8 + c].mul_add(p[2], m[4 + c].mul_add(p[1], m[c] * p[0]))))
}

/// `Scene::ePlayState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayState {
    Stopped,
    Playing,
    Paused,
}

/// `Sk8::Scene::Scene` with the `ScenePlayer` time multipliers.
struct Scene {
    hash: u32,
    kind: u8,
    frame: SceneFrame,
    play_state: PlayState,
    time_elapsed: f32,
    time_delta: f32,
    time_length: f32,
    camera_track_found: bool,
    camera_finished: bool,
    subtitler: Option<SubTitler>,
    track: Option<Box<dyn CameraTrack>>,
    /// `mSceneEntities`: the characters `AddEntities` instanced.
    characters: Vec<SceneCharacter>,
}

impl Scene {
    /// `Scene::SetTimeElapsed` (`0x82865690`): every entity jumps to the
    /// time (`SceneCharacter::SetTime` `0x82862080`).
    fn set_time_elapsed(&mut self, time: f32) {
        self.time_elapsed = time;
        self.characters.retain_mut(|c| c.set_time(time).map_err(|e| eprintln!("NIS: character {}: {e}", c.schema)).is_ok());
    }

    /// `Scene::AddTime` (`0x82865700`): the elapsed time, then every entity
    /// (`SceneCharacter::AddTime` `0x828620d0`).
    fn add_time(&mut self, dt: f32) {
        self.time_elapsed += dt;
        self.characters.retain_mut(|c| c.add_time(dt).map_err(|e| eprintln!("NIS: character {}: {e}", c.schema)).is_ok());
    }
}

/// The `ScenePlayer` multipliers `GetDeltaTimeMult` (`0x82861db8`) multiplies.
#[derive(Clone, Copy, Debug)]
struct TimeMults {
    override_mult: f32,
    stream: f32,
    freeze: f32,
    scene: f32,
    freeze_count: f32,
}

impl TimeMults {
    fn delta_mult(&self) -> f32 {
        self.override_mult * self.stream * self.freeze * self.scene
    }
}

impl Default for TimeMults {
    fn default() -> Self {
        Self { override_mult: 1.0, stream: 1.0, freeze: 1.0, scene: 1.0, freeze_count: 0.0 }
    }
}

/// `FrontEndManager::mGenericSubtitles` (+0x1EC0), the `FESubtitle` the
/// subtitle movie reads. The front end's constructor clears its flags.
#[derive(Clone, Copy, Debug, Default)]
struct FeSubtitle {
    /// `mbAutoTimeout` (+0x0).
    auto_timeout: bool,
    /// `mbActiveSubtitle` (+0x408).
    active: bool,
    /// `mSubtitleText` (+0x8), as the HAL id `SetSubtitleText` formats.
    text: Option<u32>,
}

impl FeSubtitle {
    /// `SetSubtitleText` (`0x82615ec8`): a nonzero id the localizer has
    /// (`DoesStringExist`) becomes the active line, formatted only with the
    /// subtitles option on; anything else clears it. The string table is
    /// the PC front end's, which shows nothing for an id without a string.
    fn set_text(&mut self, hal: u32) {
        if hal == 0 {
            self.clear();
        } else {
            self.active = true;
            self.text = Some(hal);
        }
    }

    /// `ClearSubtitle` (`0x82615e68`).
    fn clear(&mut self) {
        self.active = false;
        self.text = None;
    }

    /// `EnableAutoTimeOut` (`0x82616050`): zeroes `mSpeechTimeout`; turning
    /// the time-out off clears the line.
    fn enable_auto_timeout(&mut self, on: bool) {
        self.auto_timeout = on;
        if !on {
            self.clear();
        }
    }

    /// `FrontEndManager::Update`'s check (`0x826402c4`): with the time-out
    /// on, an active line goes once `mCounter` reaches `mSpeechTimeout`.
    /// Lines come from `SubTitler::Update` alone (no shipped script posts
    /// `cMsgSetSubtitle`): a scene's inside `FrontEndState_NIS` and an FMV's
    /// (`ui::movie_player`) between `FEMoviePlayer::Play` and `Stop`, both
    /// with the time-out off, so the time-out met here is the zero
    /// `EnableAutoTimeOut(true)` left: the first update clears.
    fn update(&mut self) {
        if self.auto_timeout && self.active {
            self.clear();
        }
    }
}

/// The simulation side: `Sk8::NISManager`, the sim `VisualTransitionManager`'s
/// in-transition flag, the HUD's NIS visibility flag and `FrontEndState_NIS`.
#[derive(Debug)]
struct Simulation {
    in_nis: bool,
    in_playback: bool,
    scene_name: String,
    exit_transition: i32,
    hold_camera: bool,
    allow_skip: bool,
    resolving: Resolving,
    ignore_breadcrumb_teleport: bool,
    paused: bool,
    /// `VisualTransitionManager::mIsInTransition`.
    visual_transition: bool,
    /// `HUDManager2` hidden for `eHUDVisibilityFlag_NIS`.
    nis_disable_hud: bool,
    /// `FrontEndState_NIS` on the FE stack (its `OnEnter` hides the HUD).
    fe_state: bool,
    /// `FrontEndManager::mGenericSubtitles`.
    subtitle: FeSubtitle,
    /// `FrontEndState_NIS`'s overlay screens.
    fe: FeNisOverlays,
    /// `mNISPlaybackOverridesInternal`.
    overrides: PlaybackOverrides,
}

/// The presentation side: `SceneManagerImpl`'s NIS fields.
struct Presentation {
    load_state: u8,
    load_timer: f32,
    do_kill: bool,
    kill_type: KillType,
    stopped: bool,
    handle: Option<Scene>,
    hold_camera: bool,
    /// `gNISSceneHash`.
    scene_hash: u32,
    /// The scene's `SceneLoadRequest` was made; `gNISSceneIsLoaded` once done.
    load_requested: bool,
    scene_loaded: bool,
    frame: SceneFrame,
    mults: TimeMults,
    transitions: PresTransitions,
    /// `CameraPresentation::mICECameraActive`.
    ice_camera_active: bool,
    ice: NisCamera,
    frozen_focus: Option<FrozenFocus>,
    /// The camera anchor: the local skater's presented root.
    anchor: Option<[f32; 3]>,
    /// The root's X, Y and Z axes.
    anchor_axes: Option<[[f32; 3]; 3]>,
    /// The camera mover's lasting state.
    mover: Mover,
    ice_world: IceWorld,
    /// The local skater's `GetHipsPos`.
    local_hips: Option<[f32; 3]>,
    local_hips_forward: Option<[f32; 3]>,
    /// The game camera's matrix, which the camera presentation shows outside a scene camera.
    game_camera: Option<[f32; 16]>,
    /// The recorded game camera a replay-controlled scene's restore left.
    replay_camera: Option<[f32; 16]>,
    /// This frame's take built a camera.
    camera_built: bool,
    /// Why the camera's collision could not be resolved, once it failed.
    camera_constraint_error: Option<String>,
    /// The local skater's `m_BR`: half its local bounds' diagonal
    /// (`PresentationEntity::CalculateBoundsInfo` `0x827f2900`).
    local_radius: Option<f32>,
    /// `ICEUserChannelFreeze::mInstanceFrozenSkater`.
    instance_frozen_skater: bool,
    /// `ICEUserChannelFreeze::UpdateFrameEnd` (`0x826cf1a8`) instanced a frozen skater:
    /// `SceneCharacterManager::InstanceFrozenSkater` (`0x82874f90`).
    frozen_skater: bool,
    /// `PostfxPresentation::mICEPostfxActive`.
    ice_postfx_active: bool,
    ice_postfx: IcePostfx,
    /// The visual indicators hidden by `StartNIS` for a non-VI scene.
    vis_indicators_hidden: bool,
    /// `mSnapSceneToGround`.
    snap_scene: bool,
    /// `mCharacterLocationList` with `mLocationNeedsSnapList`, by record.
    locations: Vec<([f32; 3], bool)>,
    /// `mGroundLocationRecieved`.
    ground_received: bool,
    /// `ScenePlayer::mIsFEWaitForPreload`.
    fe_wait_for_preload: bool,
    /// `ICEUserChannelOverlays::mCurrentOverlayState`.
    overlay_state: i64,
    /// `ICEUserChannelFreeze::mPreviousInterval` (constructor 0xFFFF).
    freeze_previous: u16,
    /// `ICEUserChannelFreeze::mIsTakeChange`.
    freeze_take_change: bool,
    /// `ICEUserChannelAudioEffect::mPreviousInterval` (constructor 0xFFFF).
    audio_effect_previous: u16,
    /// `ICEUserChannelAudioEffect::mIsTakeChange`.
    audio_effect_take_change: bool,
    /// `ICEUserChannelAudioMusic`, `ICEUserChannelAudioSpeech` and
    /// `ICEUserChannelAudioPost`.
    audio_music: AudioChannel,
    audio_speech: AudioChannel,
    audio_post: AudioChannel,
    /// `ScenePlayer::mAudioSpeechWait` and `mAudioPostWait`, with the speech
    /// `AudioQueueForStart` queued (`mAudioSpeechCharacter`, `mAudioSpeechID`).
    audio_speech_wait: bool,
    audio_post_wait: bool,
    audio_speech_character: i32,
    audio_speech_id: i32,
    /// `UpdateLoadState` state 1's `MoveStreamFocus` to the scene with a
    /// `FrozenStreamerController`, until `Scene::Init` replaces it or
    /// `Scene::Purge` returns the game's.
    scene_focus: Option<[f32; 3]>,
    /// `SceneLocationSnapper::mInverseSceneTransform`.
    snap_inverse: [f32; 16],
    /// The snapper's scene-space locations once `mSnappedLocationsRecieved`.
    snapped: Option<Vec<[f32; 3]>>,
    /// `IsStreamFocusLoaded(3, kFocusLoadType_GameLoad)` as the game
    /// reports it.
    stream_focus_loaded: bool,
    /// `ScenePlayer::SetReplayStartFrame` and `SetReplayEndFrame`: the frames a
    /// replay-controlled scene's `SceneReplayTimeController` plays.
    replay: Option<(u32, u32)>,
    /// PC deviation: HoM replays ignore cinematic time scaling
    /// and freeze holds. This travels with each replay, never a scene-name guess.
    full_speed_hom_replay: bool,
    /// `SceneManagerImpl::mHideAISkatersForPhotoOutro` (+0x35C).
    hide_ai_skaters_for_photo_outro: bool,
    /// `ScenePlayer::GetPlaybackOverrides`: the running scene's.
    overrides: PlaybackOverrides,
    /// `ScenePlayer::mSpecialFlags` (`SetSpecialFlags` `0x82861d98`).
    special_flags: i32,
    /// `SceneManagerImpl::mWaitStreamerBeforeTransitionOut`.
    wait_streamer: bool,
    /// `SceneManagerImpl::mIsInOnlineGame`: `StartNIS` found the flow's `STATE_OnlineGame`
    /// active.
    in_online_game: bool,
}

/// What the renderer and the front end show of the NIS this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NisView {
    /// The ICE camera while `mICECameraActive`.
    pub camera: Option<NisCamera>,
    /// The stream focus the NIS froze: at the scene from `UpdateLoadState`
    /// state 1, then where `Scene::Init` moved it.
    pub frozen_focus: Option<FrozenFocus>,
    /// The ICE post-fx while `mICEPostfxActive` (from `Scene::Init` to
    /// `Scene::Purge`); the renderer reads the game blocks otherwise.
    pub ice_postfx: Option<IcePostfx>,
    /// `bloomToWhiteValue` of the general post-fx blocks.
    pub bloom_to_white: f32,
    /// The HUD is hidden: `FrontEndState_NIS` is up or `eHUDVisibilityFlag_NIS`.
    pub hud_hidden: bool,
    /// The front end's subtitle line (`FESubtitle::mSubtitleText`) as a HAL
    /// id.
    pub subtitle: Option<u32>,
    /// `VIPresentation::SetRenderTypes(0)` for a non-visual-indicator scene.
    pub vis_indicators_hidden: bool,
    /// The scene playing, with its elapsed time.
    pub scene: Option<(String, f32)>,
    /// `RenderMan::UpdateVisibleTypes` (`0x82839be0`): while a scene exists
    /// (`SceneManagerImpl::IsInNIS`) only these entity types are drawn: 0x286F
    /// for a flythrough or photo intro, 0x282F for a scene with characters
    /// (plus 0x42 when it keeps DMOs), every type otherwise. Gameplay skaters
    /// are type 0x122, pedestrians 0x222, vehicles 0x422, dynamic objects 0x42.
    pub visible_types: u32,
    /// `SceneManagerImpl::IsInPhotoChallengeNIS` (`0x82861158`): a photo outro is playing.
    pub photo_outro: Option<PhotoOutro>,
}

/// The photo outro playing, as `RenderView::DetermineVisibilityFromGameState` reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhotoOutro {
    /// `SceneManagerImpl::mHideAISkatersForPhotoOutro`: its take shoots `PhotoShot_FixedFrame`.
    pub hide_ai_skaters: bool,
}

/// The photographers' `livingworld_models` collections (attiba_jefferson, michael_burnette),
/// whose pedestrians a photo outro leaves out (`0x82829744..0x82829780`).
const PHOTOGRAPHER_MODELS: [u64; 2] = [0x88d7_2df4_43d5_6751, 0x4e6e_5331_d54e_4f61];

impl NisView {
    /// `RenderView::DetermineVisibilityFromGameState` (`0x82829698`) for an entity of
    /// `entity_type`: drawn when the render types cover it. In a photo outro every entity is,
    /// but a pedestrian of a photographer's `livingworld_models` collection (`model`,
    /// `cLivingWorldPresEntity::mModelAttribs`) and, with `hide_ai_skaters`, a skater whose
    /// recipe is not the player's (`SkaterPresentation::GetPlayerRecipeID` `0x827dd270`).
    #[must_use]
    pub fn draws(&self, entity_type: u32, model: u64, players_recipe: bool) -> bool {
        let Some(photo) = self.photo_outro else {
            return entity_type & self.visible_types == entity_type;
        };
        if entity_type & 0x222 == 0x222 {
            !PHOTOGRAPHER_MODELS.contains(&model)
        } else if entity_type & 0x122 == 0x122 {
            players_recipe || !photo.hide_ai_skaters
        } else {
            true
        }
    }
}

pub struct NisSystem {
    directory: Arc<SceneDirectory>,
    tracks: Arc<dyn CameraTracks>,
    subtitle_big: Option<Arc<Vec<u8>>>,
    characters: Option<Arc<dyn SceneCharacterSource>>,
    language: String,
    sim: Simulation,
    pres: Presentation,
    to_presentation: VecDeque<ToPresentation>,
    to_simulation: VecDeque<ToSimulation>,
    events: Vec<NisEvent>,
    /// The `IAudioInterface::PlayNISSfx` ids `ICEUserChannelAudioEffect::FireEvent` (`0x826d6648`)
    /// sent since the audio last took them.
    nis_sfx: Vec<i32>,
    /// `mActorIDs`: the players `RunBasicNISDefinition` names in its playback overrides.
    actor_ids: Vec<i32>,
    /// `mPreloadedSceneHash` and `mPreloadedSceneCompleted`. Scenes load when they run; the
    /// presentation's preload (`StartPendingPreload`) is not ported.
    preloaded_scene: Option<(u32, bool)>,
    /// The audio system (`SingletonHolder<AudioSystem>`); none without audio.
    audio: Option<Arc<dyn AudioInterface>>,
    audio_clock: AudioClock,
    /// `Sk8::Net::IsInOnlineGame` (`0x82559b80`): the flow's `STATE_OnlineGame` is active.
    online_game: bool,
}

impl NisSystem {
    #[must_use]
    pub fn new(directory: Arc<SceneDirectory>, tracks: Arc<dyn CameraTracks>, subtitle_big: Option<Arc<Vec<u8>>>, language: &str) -> Self {
        Self {
            directory,
            tracks,
            subtitle_big,
            characters: None,
            language: language.to_owned(),
            sim: Simulation {
                in_nis: false,
                in_playback: false,
                scene_name: String::new(),
                exit_transition: 0,
                hold_camera: false,
                allow_skip: false,
                resolving: Resolving::None,
                ignore_breadcrumb_teleport: false,
                paused: false,
                visual_transition: false,
                nis_disable_hud: false,
                fe_state: false,
                subtitle: FeSubtitle::default(),
                fe: FeNisOverlays::default(),
                overrides: PlaybackOverrides::default(),
            },
            pres: Presentation {
                load_state: 0,
                load_timer: 0.0,
                do_kill: false,
                kill_type: KillType::None,
                stopped: false,
                handle: None,
                hold_camera: false,
                scene_hash: 0,
                load_requested: false,
                scene_loaded: false,
                frame: SceneFrame::default(),
                mults: TimeMults::default(),
                transitions: PresTransitions::default(),
                ice_camera_active: false,
                ice: NisCamera::default(),
                frozen_focus: None,
                anchor: None,
                anchor_axes: None,
                mover: Mover::default(),
                ice_world: IceWorld::default(),
                local_hips: None,
                local_hips_forward: None,
                game_camera: None,
                replay_camera: None,
                camera_built: false,
                camera_constraint_error: None,
                local_radius: None,
                instance_frozen_skater: false,
                frozen_skater: false,
                ice_postfx_active: false,
                ice_postfx: IcePostfx::default(),
                vis_indicators_hidden: false,
                snap_scene: false,
                locations: Vec::new(),
                ground_received: false,
                fe_wait_for_preload: false,
                overlay_state: i64::from(STATE_UNKNOWN),
                freeze_previous: 0xffff,
                freeze_take_change: false,
                audio_effect_previous: 0xffff,
                audio_effect_take_change: false,
                audio_music: AudioChannel::default(),
                audio_speech: AudioChannel::default(),
                audio_post: AudioChannel::default(),
                audio_speech_wait: false,
                audio_post_wait: false,
                audio_speech_character: 0,
                audio_speech_id: 0,
                scene_focus: None,
                snap_inverse: [0.0; 16],
                snapped: None,
                stream_focus_loaded: false,
                replay: None,
                full_speed_hom_replay: false,
                hide_ai_skaters_for_photo_outro: false,
                overrides: PlaybackOverrides::default(),
                special_flags: SPECIAL_FLAG_NONE,
                wait_streamer: false,
                in_online_game: false,
            },
            to_presentation: VecDeque::new(),
            to_simulation: VecDeque::new(),
            events: Vec::new(),
            nis_sfx: Vec::new(),
            actor_ids: Vec::new(),
            preloaded_scene: None,
            audio: None,
            audio_clock: AudioClock::default(),
            online_game: false,
        }
    }

    /// The audio system the scene's audio and clock go through.
    pub fn set_audio(&mut self, audio: Arc<dyn AudioInterface>) {
        self.audio = Some(audio);
    }

    #[must_use]
    pub fn directory(&self) -> &SceneDirectory {
        &self.directory
    }

    /// SceneManagerImpl::StartSceneCharacterViewMode8286e548. ViewScene is a
    /// separate looping player: no NIS challenge transition or world teleport.
    pub fn start_trickbook(&mut self) -> Result<(), String> {
        let hash=scene_hash("ViewScene");
        self.pres.frame=SceneFrame {location:[0.;3],orientation:[0.,0.,-1.],normal:[0.,1.,0.]};
        self.pres.scene_hash=hash;
        let track=self.directory.camera_track(hash);
        let mut scene=self.create_scene(hash,track).ok_or("ViewScene absent")?;
        scene.play_state=PlayState::Playing;
        self.pres.handle=Some(scene);
        Ok(())
    }

    /// UpdateSceneCharacterViewMode8286e788: swap the original animation,
    /// reset elapsed time, select its camera, and set the take to clip length.
    pub fn set_trickbook_animation(&mut self, animation:&str, camera:u32)->Result<(),String> {
        use super::characters::{SceneAnimData,SceneCharacterAnim,SKATER_FACE};
        let source=self.characters.as_ref().ok_or("Trick Guide character source absent")?;
        let path=format!("data/scene/trickguide/{animation}1.abin");
        let data=Arc::new(SceneAnimData::parse(source.read(&path).ok_or_else(||format!("{path} absent"))?)?);
        let face=skate2_formats::andale::AndaleDatabase::parse(&source.read(SKATER_FACE).ok_or("Trick Guide face data absent")?).map_err(|e|e.to_string())?;
        let local=source.local_player().ok_or("Trick Guide local character absent")?;
        let scene=self.pres.handle.as_mut().ok_or("ViewScene not started")?;
        let character=scene.characters.first_mut().ok_or("ViewScene has no character")?;
        let mut anim=SceneCharacterAnim::new(data,animation,0.,source.twist_ratios())?;
        anim.init_face(&face,&local.schema,local.create_a_character)?;
        scene.time_length=anim.length();
        character.anim=anim;
        scene.set_time_elapsed(0.);
        let props=self.directory.properties(scene.hash).ok_or("ViewScene properties absent")?;
        let (track,_)=self.tracks.connect(props,camera as u8,Some(scene.time_length)).ok_or("Trick Guide camera track absent")?;
        scene.track=Some(track);scene.camera_track_found=true;scene.camera_finished=false;
        self.pres.overlay_state=i64::from(STATE_UNKNOWN);
        self.pres.audio_speech=AudioChannel::default();self.pres.audio_post=AudioChannel::default();
        self.pres.mults=TimeMults::default();
        self.scene_camera_frame_begin(0.);self.scene_camera_frame_end(0.);
        Ok(())
    }

    /// View mode is updated without UpdateNIS's challenge start/stop machine.
    pub fn update_trickbook(&mut self,dt:f32) {
        self.scene_player_update(dt);
        self.scene_camera_frame_begin(dt);
        self.scene_camera_frame_end(dt);
    }

    /// `mIsInNIS`.
    #[must_use]
    pub fn is_in_nis(&self) -> bool {
        self.sim.in_nis
    }

    /// `mIsInPlayback`.
    #[must_use]
    pub fn is_in_playback(&self) -> bool {
        self.sim.in_playback
    }

    /// The NIS pause is pushed.
    #[must_use]
    pub fn is_sim_paused(&self) -> bool {
        self.sim.paused
    }

    /// `FrontEndState_NIS` is on the FE stack.
    #[must_use]
    pub fn fe_state_active(&self) -> bool {
        self.sim.fe_state
    }

    /// Nothing of a NIS is left to update: no NIS, no queued message, no
    /// transition running.
    #[must_use]
    pub fn idle(&self) -> bool {
        !self.sim.in_nis
            && self.to_presentation.is_empty()
            && self.to_simulation.is_empty()
            && !self.pres.transitions.in_transition()
            && self.pres.handle.is_none()
            && self.pres.load_state == 0
    }

    /// The local skater's root, `SkaterPresentation::GetSkaterRoot` (Skate
    /// space): `User_ICECameraAnchor::GetPosition` (`0x826cfc78`) gives the
    /// camera mover its translation, `GetOrientation` (`0x826cd228`) its axes.
    pub fn set_camera_anchor(&mut self, root: Option<[f32; 16]>) {
        self.pres.anchor = root.map(|m| [m[12], m[13], m[14]]);
        self.pres.anchor_axes = root.map(|m| [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]]);
    }

    /// Where the NIS froze the stream focus (`MoveStreamFocus` with every
    /// focus, then a `FrozenStreamerController`): at the scene from
    /// `UpdateLoadState` state 1 (`0x82876038..0x82876074`), then where
    /// `Scene::Init` moved it; none once the game's controller is back.
    #[must_use]
    pub fn stream_focus(&self) -> Option<FrozenFocus> {
        self.pres.frozen_focus.or(self.pres.scene_focus.map(FrozenFocus::At))
    }

    /// `cGameAssetManager::IsStreamFocusLoaded(3, kFocusLoadType_GameLoad)`
    /// wherever the focus is.
    pub fn set_stream_focus_loaded(&mut self, loaded: bool) {
        self.pres.stream_focus_loaded = loaded;
    }

    /// `Sk8::Net::IsInOnlineGame` (`0x82559b80`): whether the flow's `STATE_OnlineGame` is
    /// active. A scene's load then waits neither for the simulation's pause nor for the stream
    /// focus (`UpdateLoadState` states 1, 2 and 6).
    pub fn set_online_game(&mut self, online: bool) {
        self.online_game = online;
    }

    /// The local skater's hips as its presentation restored them
    /// (`SkaterPresentation::GetHipsPos`, Skate space), which
    /// `User_ICESceneManager::GetAvatarPosition` gives avatars 3 and 4.
    pub fn set_camera_local_hips_forward(&mut self, forward: Option<[f32; 3]>) {
        self.pres.local_hips_forward = forward;
    }

    /// The game camera's matrix (Skate rows), `CameraPresentation::GetCurrentData` until the
    /// scene's camera shows.
    pub fn set_game_camera(&mut self, matrix: Option<[f32; 16]>) {
        self.pres.game_camera = matrix;
    }

    /// The recorded game camera a replay's restore left (`CameraPresentation` restores the
    /// replay's camera frames beside the scene's camera); none outside a replay.
    pub fn set_replay_camera(&mut self, matrix: Option<[f32; 16]>) {
        self.pres.replay_camera = matrix;
    }

    /// `CameraPresentation::GetReplayMatrix` (`0x826cf400`): its current frame
    /// (`GetCurrentData` `0x826cb9e8`), the recorded game camera in a replay, else the game
    /// camera.
    fn replay_matrix(&self) -> Option<[f32; 16]> {
        self.pres.replay_camera.or(self.pres.game_camera)
    }

    pub fn set_camera_local_hips(&mut self, hips: Option<[f32; 3]>) {
        self.pres.local_hips = hips;
    }

    /// Whether a fixed-frame photo instanced a frozen skater since the last call: a copy of the
    /// local skater in its last saved pose (`CharacterPoseSaver::GetMostRecentPoseBuffer`) at
    /// its transform, added to the scene until the scene ends.
    pub fn take_frozen_skater(&mut self) -> bool {
        std::mem::take(&mut self.pres.frozen_skater)
    }

    /// The local skater's bounds, min then max, for its `m_BR`.
    pub fn set_local_bounds(&mut self, bounds: Option<[[f32; 3]; 2]>) {
        self.pres.local_radius = bounds.map(|[min, max]| {
            let extent: [f32; 3] = std::array::from_fn(|lane| max[lane] - min[lane]);
            0.5 * extent[2].mul_add(extent[2], extent[1].mul_add(extent[1], extent[0] * extent[0])).sqrt()
        });
    }

    /// The scene characters' files and the local player (`AddLoaders`,
    /// `InstanceCharacter`); without it a scene instances none.
    pub fn set_character_source(&mut self, source: Arc<dyn SceneCharacterSource>) {
        self.characters = Some(source);
    }

    /// The running scene's characters.
    #[must_use]
    pub fn scene_characters(&self) -> &[SceneCharacter] {
        self.pres.handle.as_ref().map_or(&[], |s| &s.characters)
    }

    /// The running scene's characters, to give them render entities.
    pub fn scene_characters_mut(&mut self) -> &mut [SceneCharacter] {
        match self.pres.handle.as_mut() {
            Some(scene) => &mut scene.characters,
            None => &mut [],
        }
    }

    /// `SceneManager::IsInCharacterNIS` (`0x82862db0`): a scene exists and
    /// has characters (`IsSceneWithCharacters` `0x8286ece0`). Then its
    /// `mTimeElapsed` (`ScenePlayer::GetTimeElapsed` `0x8286c9e0`), which
    /// `SkaterPresEntity::ClothUpdate` (`0x827d24f8`) steps the cloth by.
    #[must_use]
    pub fn character_nis_time(&self) -> Option<f32> {
        let scene = self.pres.handle.as_ref()?;
        self.directory
            .is_with_characters(scene.hash)
            .then_some(scene.time_elapsed)
    }

    pub fn take_events(&mut self) -> Vec<NisEvent> {
        std::mem::take(&mut self.events)
    }

    /// The `PlayNISSfx` ids fired since the last call.
    pub fn take_nis_sfx(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.nis_sfx)
    }

    /// `AudioSystem::mNISFadeAmount`: `BloomToWhiteTransitionEffect::Update` (`0x827d55c0`)
    /// writes it with the post-fx blocks' `bloomToWhiteValue`.
    #[must_use]
    pub fn bloom_to_white(&self) -> f32 {
        self.pres.transitions.bloom_to_white
    }

    #[must_use]
    pub fn view(&self) -> NisView {
        NisView {
            camera: self.pres.ice_camera_active.then_some(self.pres.ice),
            frozen_focus: self.stream_focus(),
            ice_postfx: self.pres.ice_postfx_active.then_some(self.pres.ice_postfx),
            bloom_to_white: self.pres.transitions.bloom_to_white,
            hud_hidden: self.sim.fe_state || self.sim.nis_disable_hud,
            subtitle: self.sim.subtitle.text,
            vis_indicators_hidden: self.pres.vis_indicators_hidden,
            scene: self.pres.handle.as_ref().map(|s| {
                let name = self.directory.properties(s.hash).map(|p| p.name.clone()).unwrap_or_default();
                (name, s.time_elapsed)
            }),
            visible_types: self.pres.handle.as_ref().map_or(u32::MAX, |s| {
                let kind = self.directory.scene_type(s.hash);
                if kind == scene_type::FLYTHROUGH || kind == scene_type::PHOTO_INTRO {
                    0x286f
                } else if self.directory.is_with_characters(s.hash) {
                    0x282f | if self.directory.is_with_dmos_enabled(s.hash) { 0x42 } else { 0 }
                } else {
                    u32::MAX
                }
            }),
            photo_outro: self
                .is_in_photo_challenge_nis()
                .then_some(PhotoOutro { hide_ai_skaters: self.pres.hide_ai_skaters_for_photo_outro }),
        }
    }

    /// `NISManager::ResetActorIDs` (`0x828a24a8`).
    pub fn reset_actor_ids(&mut self) {
        self.actor_ids.clear();
    }

    /// `NISManager::AddActorID` (`0x828a8648`).
    pub fn add_actor_id(&mut self, player: i32) {
        self.actor_ids.push(player);
    }

    /// `NISManager::PreloadNISList` (`0x828a6998`): the first scene.
    pub fn preload_nis_list(&mut self, scenes: &[String]) {
        if let Some(scene) = scenes.first() {
            self.preloaded_scene = Some((scene_hash(scene), false));
        }
    }

    /// `mPreloadedSceneHash` is set: `cChallengeCharacterManager::TestBBoxAgainstTree`
    /// (`0x825ab590`) spawns no marquee character whose model is not tracked yet.
    #[must_use]
    pub const fn has_preloaded_scene(&self) -> bool {
        self.preloaded_scene.is_some()
    }

    /// `NISManager::UnloadAnyPreloadedNISs` (`0x828a2e40`): `cMsgNISUnloadAll` unloads the
    /// presentation's preloaded scenes, and none are; the preloaded scene's hash stays.
    pub fn unload_any_preloaded_niss(&mut self) {}

    /// `NISManager::IgnoreTeleportToBreadcrumbOnNextNIS`.
    pub fn ignore_teleport_to_breadcrumb_on_next_nis(&mut self) {
        self.sim.ignore_breadcrumb_teleport = true;
    }

    /// `NISManager::RunBasicNISDefinition` (`0x828a8a70`): the definition's
    /// `IgnoreTeleportToLastBreadcrumb` becomes the manager's, which the breadcrumb teleport it
    /// makes first consumes; then the scene runs as [`Self::run_basic_nis`] (whose own teleport
    /// follows) with the definition's location override and the stream position to restore
    /// (`GetRestorePosition` `0x828a1418`, the local skater's position, when the streamer is
    /// restored on completion), the submitted actor IDs as their player indexes.
    pub fn run_basic_nis_definition(
        &mut self,
        scene: &str,
        ignore_breadcrumb: bool,
        exit_transition: i32,
        hold_camera: bool,
        skip_enabled: bool,
        mut overrides: PlaybackOverrides,
    ) {
        self.sim.ignore_breadcrumb_teleport = ignore_breadcrumb;
        self.teleport_to_last_breadcrumb(scene);
        overrides.player_indexes.clone_from(&self.actor_ids);
        self.run_basic_nis_with(scene, exit_transition, hold_camera, skip_enabled, overrides);
    }

    /// `NISManager::RunBasicNIS` with the default playback overrides
    /// (`msDefaultPlaybackOverrides`) and no special flags.
    pub fn run_basic_nis(&mut self, scene: &str, exit_transition: i32, hold_camera: bool, skip_enabled: bool) {
        self.run_basic_nis_with(scene, exit_transition, hold_camera, skip_enabled, PlaybackOverrides::default());
    }

    /// `NISManager::RunBasicNIS` (`0x828a8d70`) with no special flags. A request while a NIS
    /// runs is dropped.
    fn run_basic_nis_with(&mut self, scene: &str, exit_transition: i32, hold_camera: bool, skip_enabled: bool, overrides: PlaybackOverrides) {
        if self.sim.in_nis {
            return;
        }
        self.teleport_to_last_breadcrumb(scene);
        let kind = self.directory.scene_type(scene_hash(scene));
        if kind != scene_type::FRONTEND && kind != scene_type::VIEW_MODE {
            // FrontEndManager::HudManagerSetNISDisableHUD(true).
            self.sim.nis_disable_hud = true;
            // VisualTransitionManager::BeginTransition(NISLOADSCREEN) (0x828a38c0).
            if !self.sim.visual_transition {
                self.sim.visual_transition = true;
                self.to_presentation.push_back(ToPresentation::VisualTransitionBegin);
            }
        }
        self.sim.in_playback = false;
        self.sim.in_nis = true;
        scene.clone_into(&mut self.sim.scene_name);
        self.sim.exit_transition = exit_transition;
        self.sim.hold_camera = hold_camera;
        self.sim.allow_skip = skip_enabled;
        self.sim.overrides = overrides;
        // mPreloadedSceneHash and mPreloadedSceneCompleted clear before PauseSim.
        self.preloaded_scene = None;
        self.pause_sim();
        // NISCharacterResolver::Start: no characters to resolve.
        self.start_pending_nis();
    }

    /// `NISManager::RunReplayNIS` (`0x828a3c70`): outside a NIS, the breadcrumb teleport, the
    /// HUD hidden for the NIS and the load-screen transition, then the simulation paused while
    /// `cMsgNISEnterReplay` plays the scene over the replay recorder's frames `start` to `end`.
    /// The camera hold, playback overrides and special flags travel with the message, not the
    /// manager. `NIS_SPECIALFLAG_HALLOFMEAT` also selects the PC's intentional full-speed HoM
    /// replay policy (2026-10-12).
    #[expect(clippy::too_many_arguments, reason = "the native signature")]
    pub fn run_replay_nis(
        &mut self,
        scene: &str,
        start: u32,
        end: u32,
        exit_transition: i32,
        hold_camera: bool,
        skip_enabled: bool,
        overrides: PlaybackOverrides,
        special_flags: i32,
    ) {
        if self.sim.in_nis {
            return;
        }
        self.teleport_to_last_breadcrumb(scene);
        self.sim.nis_disable_hud = true;
        if !self.sim.visual_transition {
            self.sim.visual_transition = true;
            self.to_presentation.push_back(ToPresentation::VisualTransitionBegin);
        }
        self.sim.exit_transition = exit_transition;
        self.sim.allow_skip = skip_enabled;
        self.sim.in_nis = true;
        self.sim.in_playback = false;
        self.preloaded_scene = None;
        scene.clone_into(&mut self.sim.scene_name);
        self.pause_sim();
        self.to_presentation.push_back(ToPresentation::EnterReplay { scene: scene.to_owned(), start, end, hold_camera, overrides, special_flags });
    }

    /// The ICE camera's position while it is active.
    #[must_use]
    pub fn camera_position(&self) -> Option<[f32; 3]> {
        self.pres.ice_camera_active.then_some(self.pres.ice.matrix).flatten().map(|m| [m[12], m[13], m[14]])
    }

    /// The running replay-controlled scene's frames and elapsed time, which its
    /// `SceneReplayTimeController` (`Update` `0x828727e8`) scrubs the recorder by.
    #[must_use]
    pub fn replay_clock(&self) -> Option<(u32, u32, f32)> {
        let (start, end) = self.pres.replay?;
        let scene = self.pres.handle.as_ref().filter(|scene| self.directory.is_controlling_replay(scene.hash))?;
        Some((start, end, scene.time_elapsed))
    }

    /// SceneReplayTimeController::GetTickCount 0x82872720 uses GetDeltaTimeMult,
    /// including the stream, scene and photo-freeze multipliers.
    #[must_use]
    pub fn replay_speed(&self) -> Option<f32> {
        self.replay_clock().map(|_| self.pres.mults.delta_mult())
    }

    /// Whether the running scene is a photo outro (`SceneManagerImpl::IsInPhotoChallengeNIS`
    /// `0x82861158`).
    #[must_use]
    pub fn is_in_photo_challenge_nis(&self) -> bool {
        self.pres.handle.as_ref().is_some_and(|scene| scene.kind == scene_type::PHOTO_OUTRO)
    }

    /// `SceneManagerImpl::ReceiveNISPhotoSelectMsg` (`0x82861680`): the running scene jumps to
    /// the photo's shot (`SceneCamera::GetTimeAtPhotoShot` `0x82864d30`).
    pub fn photo_select(&mut self, index: usize) {
        let Some(scene) = self.pres.handle.as_mut() else { return };
        let time = scene.track.as_ref().map_or(0.0, |track| track.photo_shot_time(index));
        scene.set_time_elapsed(time);
    }

    /// `NISManager::GetPhotoCount(sceneName)` (`0x828a0e70`): the scene's camera take's
    /// `PhotoShot` freeze intervals.
    #[must_use]
    pub fn photo_count(&self, scene: &str) -> i32 {
        self.photo_counter().count(scene)
    }

    /// What [`Self::photo_count`] reads, for the challenge system's `NIS.GetPhotoNISCount`.
    #[must_use]
    pub fn photo_counter(&self) -> PhotoCounter {
        PhotoCounter { directory: Arc::clone(&self.directory), tracks: Arc::clone(&self.tracks) }
    }

    /// `TeleportToLastBreadCrumb` (`0x828a0db8`).
    fn teleport_to_last_breadcrumb(&mut self, scene: &str) {
        if self.sim.ignore_breadcrumb_teleport {
            self.sim.ignore_breadcrumb_teleport = false;
            return;
        }
        let kind = self.directory.scene_type(scene_hash(scene));
        if kind == scene_type::STORY || kind == scene_type::GOTO {
            self.events.push(NisEvent::TeleportToLastBreadcrumb);
        }
    }

    fn pause_sim(&mut self) {
        if !self.sim.paused {
            self.sim.paused = true;
            self.events.push(NisEvent::PauseSim);
        }
    }

    fn resume_sim(&mut self) {
        if self.sim.paused {
            self.sim.paused = false;
            self.events.push(NisEvent::ResumeSim);
        }
    }

    /// `StartPendingNIS` (`0x828a2d30`).
    fn start_pending_nis(&mut self) {
        self.to_presentation.push_back(ToPresentation::EnterBasic {
            scene: self.sim.scene_name.clone(),
            hold_camera: self.sim.hold_camera,
            overrides: self.sim.overrides.clone(),
        });
    }

    /// `SkipNIS(force)` (`0x828a2f10`).
    pub fn skip_nis(&mut self, force: bool) {
        if (self.sim.allow_skip || force) && self.sim.in_nis {
            self.to_presentation.push_back(ToPresentation::Skip);
        }
    }

    /// FE state 8's OnLeave (0x82658bfc..0x82658c04): forced skip followed by
    /// PopState of the underlying NIS. The simulation pause ends later in EndNIS.
    pub fn leave_results_screen(&mut self) {
        self.skip_nis(true);
        self.leave_front_end();
    }

    fn leave_front_end(&mut self) {
        if self.sim.fe_state {
            self.sim.fe.pop_nis();
            self.sim.fe_state = false;
            self.sim.subtitle.enable_auto_timeout(true);
        }
    }

    /// `KillNIS` (`0x828a2f88`).
    pub fn kill_nis(&mut self) {
        self.to_presentation.push_back(ToPresentation::Kill);
    }

    /// `cMsgEnterRealTime` to the presentation thread (`PresThreadMsgMan::ReceiveEnterRealTimeMsg`
    /// `0x8277bd58`).
    pub fn enter_real_time(&mut self) {
        self.to_presentation.push_back(ToPresentation::EnterRealTime);
    }

    /// `FrontEndState_NIS::ReceiveInput` (`0x826484a0`): keycode 2 (A) skips.
    /// Its `OnEnter` readies the state at once (`OnScreenReady(-1)`).
    pub fn fe_select(&mut self) {
        if self.sim.fe_state {
            self.skip_nis(false);
        }
    }

    /// One simulation update: the front end's subtitle time-out, the queued
    /// presentation messages (`FrontEndManager::Update` checks the one
    /// before it dispatches its events), then `NISManager::Update`.
    ///
    /// # Errors
    /// The ground line test's.
    pub fn update_simulation(&mut self, ground: &mut GroundLine<'_>) -> Result<(), String> {
        // AptIntegration::Update: one Apt frame of the front end's screens.
        self.sim.fe.apt_frame();
        self.fe_preloaded();
        self.sim.subtitle.update();
        while let Some(message) = self.to_simulation.pop_front() {
            match message {
                // ReceiveVisualTransitionCompleteMsg (0x8289eda8).
                ToSimulation::VisualTransitionComplete => self.sim.visual_transition = false,
                // FrontEndManager::ReceiveNISFEPushStateMsg (0x82633f08): push
                // FE state 2 unless it is on the stack; with it there and no
                // load screen, TransitionManager::EndTransition (nothing to
                // end for a basic NIS). FrontEndState_NIS::OnEnter
                // (0x82658610) turns the subtitle time-out off, which clears
                // the line; its SetSubtitleBottom(false) only moves the
                // system notifications (XNotifyPositionUI).
                ToSimulation::FePushState => {
                    if !self.sim.fe_state {
                        self.sim.fe_state = true;
                        self.sim.subtitle.enable_auto_timeout(false);
                    }
                }
                // ReceiveNISMgrPlaybackMsg (0x8289ecf8).
                ToSimulation::Playback => self.sim.in_playback = true,
                // ReceiveNISTransitionInBeginMsg (0x828a3528).
                ToSimulation::TransitionInBegin => self.sim.nis_disable_hud = true,
                // ReceiveNISTransitionOutCompleteMsg (0x828a3500).
                ToSimulation::TransitionOutComplete => self.sim.nis_disable_hud = false,
                // ReceiveNISMgrCompleteMsg (0x828a3440): ReloadMarquees, then
                // EndNIS once reloading is done.
                ToSimulation::Complete => self.sim.resolving = Resolving::Reloading,
                // ReceiveNISLoadingOverlayMsg (0x828a3550).
                ToSimulation::LoadingOverlay(show) => self.events.push(NisEvent::LoadingOverlay(show)),
                // FrontEndManager::ReceiveSetSubtitleMsg (0x8261ba00).
                ToSimulation::SetSubtitle(hal) => self.sim.subtitle.set_text(hal),
                // ReceiveNISMgrWorldCollisionQueryMsg (0x828a3588).
                ToSimulation::WorldCollisionQuery(location) => {
                    let (location, normal) = ground_position_and_orientation(ground, location)?;
                    self.to_presentation.push_back(ToPresentation::WorldCollisionResponse { location, normal });
                }
                // FrontEndManager::ReceiveNISFERequestPreloadStateMsg
                // (0x82634dc8).
                ToSimulation::FeRequestPreload(states) => self.sim.fe.preload_nis_states(&states),
                // FrontEndManager::ReceiveNISFEEventMsg (0x82628570): with FE
                // state 2 on the stack.
                ToSimulation::FeEvent(state) => {
                    if self.sim.fe_state {
                        self.sim.fe.set_state(state);
                    }
                }
                // ReceiveNISMgrLocationListSnapQueryMsg (0x828a3620): each
                // location replaced by its ground position.
                ToSimulation::LocationListSnapQuery(locations) => {
                    let snapped = locations
                        .into_iter()
                        .map(|location| ground_position_and_orientation(ground, location).map(|(position, _)| position))
                        .collect::<Result<_, _>>()?;
                    self.to_presentation.push_back(ToPresentation::LocationListSnapResponse(snapped));
                }
            }
            if self.sim.resolving == Resolving::Reloading {
                self.sim.resolving = Resolving::None;
                self.end_nis();
            }
        }
        self.fe_preloaded();
        if self.sim.resolving == Resolving::ForPlay {
            self.sim.resolving = Resolving::None;
            self.start_pending_nis();
        }
        Ok(())
    }

    /// `CheckLoadComplete`'s `NISManager::FEPreloaded` (`0x828a3050`):
    /// `cMsgNISFEPreloaded` to the presentation while in a NIS; and the
    /// overlay states' `PushState(8)` calls.
    fn fe_preloaded(&mut self) {
        for _ in 0..self.sim.fe.take_preloaded() {
            if self.sim.in_nis {
                self.to_presentation.push_back(ToPresentation::FePreloaded);
            }
        }
        for _ in 0..self.sim.fe.take_results_pushes() {
            self.events.push(NisEvent::OpenResultsScreen);
        }
    }

    /// The front end screen manager's work on the NIS screens since the
    /// last call.
    pub fn take_screen_commands(&mut self) -> Vec<ScreenCommand> {
        self.sim.fe.take_commands()
    }

    /// `EndNIS` (`0x828a3200`).
    fn end_nis(&mut self) {
        self.sim.in_nis = false;
        self.sim.in_playback = false;
        // FrontEndManager::PopNIS (0x82640cb8): the state below (in game)
        // shows the HUD again on its UpdateOnEnter.
        // FrontEndState_NIS::OnLeave (0x8266fd70) turns the subtitle
        // time-out back on.
        self.leave_front_end();
        self.events.push(NisEvent::Completed);
        if self.sim.exit_transition != 0 {
            self.events.push(NisEvent::ExitTransition(self.sim.exit_transition));
        }
        self.resume_sim();
    }

    /// One presentation frame of `dt` seconds. `SceneManagerImpl::
    /// UpdateFrameBegin` (`0x82878428`) with `kNISUsesTheAudioClock` (1) reads
    /// the audio clock every frame (the first only records it) and steps the
    /// scene by the audio time since the last frame, a replay-controlled scene
    /// by `dt` (`kNISReplayUsesTheAudioClock` 0); `UpdateFrameEnd` steps the
    /// scene camera by the same. Without an audio system the scene steps by
    /// `dt`.
    pub fn update_presentation(&mut self, dt: f32) {
        let scene_dt = self.update_presentation_begin(dt);
        self.update_presentation_end(scene_dt, &mut |_, _, _| Ok(None));
    }

    /// [`Self::update_presentation`] up to `SceneManager::UpdateFrameEnd`: the scene's time,
    /// its messages and transitions. Returns the scene's step, which the frame's end takes;
    /// between the two the presentations restore (`DataSysMan::RestoreFrame`), so a replay
    /// scene's camera follows the frame its time controller restored.
    pub fn update_presentation_begin(&mut self, dt: f32) -> f32 {
        if let Some(audio) = self.audio.as_ref() {
            let time = audio.system_time();
            if self.audio_clock.initial_update {
                self.audio_clock.initial_update = false;
            } else {
                self.audio_clock.frame_delta = time - self.audio_clock.last_time;
            }
            self.audio_clock.last_time = time;
        }
        let scene_dt = if self.audio.is_some() && !self.is_in_replay_nis() { self.audio_clock.frame_delta as f32 } else { dt };
        // SceneManager::UpdateFrameBegin -> UpdateDeltaFrameBegin.
        self.update_nis(scene_dt);
        self.scene_player_update(scene_dt);
        self.scene_camera_frame_begin(scene_dt);
        // SceneLoadRequestManager::Update: the scene's loaders run.
        if self.pres.load_requested {
            self.pres.load_requested = false;
            self.pres.scene_loaded = true;
        }
        // Presentation::Update: its message queue, then
        // TransitionPresentation::Update.
        while let Some(message) = self.to_presentation.pop_front() {
            match message {
                // PresVisualTransitionManager::ReceiveVisualTransitionBeginMsg
                // (0x827dffb0).
                ToPresentation::VisualTransitionBegin => {
                    if self.pres.transitions.in_transition() {
                        self.to_simulation.push_back(ToSimulation::VisualTransitionComplete);
                    } else {
                        self.pres.transitions.begin(Callback::SimulationBasedEnd);
                    }
                }
                // ReceiveNISEnterBasicMsg (0x8286bfc0).
                ToPresentation::EnterBasic { scene, hold_camera, overrides } => {
                    self.pres.full_speed_hom_replay = false;
                    self.pres.hold_camera = hold_camera;
                    self.pres.overrides = overrides;
                    self.pres.special_flags = SPECIAL_FLAG_NONE;
                    let mut hash = scene_hash(&scene);
                    // SceneChooser::VerifyScene (0x82863750).
                    if !self.directory.is_valid(hash) {
                        hash = scene_hash("TestFlythrough1");
                    }
                    self.start_nis(hash);
                }
                // ReceiveNISEnterReplayMsg (0x8286c0f8): the player's replay frames, camera hold,
                // playback overrides and special flags, then VerifyScene and StartNIS.
                ToPresentation::EnterReplay { scene, start, end, hold_camera, overrides, special_flags } => {
                    self.pres.replay = Some((start, end));
                    let use_hom_effect = special_flags == SPECIAL_FLAG_HALL_OF_MEAT;
                    self.pres.full_speed_hom_replay = use_hom_effect;
                    if use_hom_effect {
                        // Do not inherit the previous scene's time scale while
                        // the HoM camera is loading or if it has no camera track.
                        self.pres.mults.scene = 1.0;
                    }
                    self.pres.hold_camera = hold_camera;
                    self.pres.overrides = overrides;
                    self.pres.special_flags = special_flags;
                    let mut hash = scene_hash(&scene);
                    if !self.directory.is_valid(hash) {
                        hash = scene_hash("TestFlythrough1");
                    }
                    self.start_nis(hash);
                }
                // Presentation::EnterRealTime (0x827f1130): DataSys takes a RealTimeController,
                // so the restore no longer follows the scene's replay time.
                ToPresentation::EnterRealTime => self.pres.replay = None,
                // ReceiveNISSkipMsg (0x828615f0).
                ToPresentation::Skip => {
                    if !self.pres.do_kill && self.is_nis_skippable() {
                        self.pres.kill_type = KillType::Input;
                        self.pres.do_kill = true;
                    }
                }
                // ReceiveNISKillMsg (0x82861660).
                ToPresentation::Kill => {
                    self.pres.kill_type = KillType::Message;
                    self.pres.do_kill = true;
                }
                // ReceiveNISFEPreloadedMsg (0x82861590) -> SetFEPreloaded.
                ToPresentation::FePreloaded => self.pres.fe_wait_for_preload = false,
                // SceneManagerImpl::ReceiveNISMgrWorldCollisionResponseMsg
                // (0x82861808), in load state 3: the ground's location and
                // normal, the orientation laid on it as N x (O x N).
                ToPresentation::WorldCollisionResponse { location, normal } => {
                    if self.pres.load_state == 3 {
                        let orientation = vmx_cross(normal, vmx_cross(self.pres.frame.orientation, normal));
                        self.pres.frame = SceneFrame { location, orientation, normal };
                        self.pres.ground_received = true;
                    }
                }
                // SceneLocationSnapper::ReceiveNISMgrLocationListSnapResponseMsg
                // (0x8286ae98): back to scene space.
                ToPresentation::LocationListSnapResponse(locations) => {
                    let inverse = self.pres.snap_inverse;
                    self.pres.snapped = Some(
                        locations
                            .into_iter()
                            .map(|p| {
                                let local = transform_point(inverse, p);
                                [local[0], local[1], local[2]]
                            })
                            .collect(),
                    );
                }
            }
        }
        if let Some(callback) = self.pres.transitions.update(dt) {
            self.run_callback(callback);
        }
        self.pres.ice_postfx.update(dt);
        scene_dt
    }

    /// `SceneManager::UpdateFrameEnd` -> `SceneCamera::UpdateFrameEnd`, with the anchors as
    /// they stand and the presentation's line tests for the camera's collision.
    pub fn update_presentation_end(&mut self, scene_dt: f32, line: &mut CameraLine<'_>) {
        self.scene_camera_frame_end(scene_dt);
        if let Err(error) = self.resolve_camera_constraints(line) {
            self.pres.camera_constraint_error.get_or_insert(error);
        }
    }

    /// `ICECameraMover::UpdateConstraints` (`0x82a36e38`) for the frame's camera, once
    /// `UpdateShake` has run: a take that avoids static objects resolves against them.
    fn resolve_camera_constraints(&mut self, line: &mut CameraLine<'_>) -> Result<(), String> {
        if !self.pres.ice_camera_active || !self.pres.camera_built {
            return Ok(());
        }
        let avoid = self
            .pres
            .handle
            .as_ref()
            .and_then(|scene| scene.track.as_ref())
            .is_some_and(|track| track.avoid_static_objects());
        let (Some(mut matrix), Some(anchor)) = (self.pres.ice.matrix, self.pres.local_hips.or(self.pres.anchor)) else {
            return Ok(());
        };
        if !avoid {
            return Ok(());
        }
        let replay = self.replay_matrix();
        self.pres.ice_world.resolve(&mut matrix, anchor, replay, line)?;
        self.pres.ice.matrix = Some(matrix);
        Ok(())
    }

    /// `SceneManagerImpl::IsInReplayNIS` (`0x82861028`): the scene controls a
    /// replay.
    fn is_in_replay_nis(&self) -> bool {
        self.pres.handle.as_ref().is_some_and(|scene| self.directory.is_controlling_replay(scene.hash))
    }

    /// `ScenePlayer::AudioQueueForStart` (`0x82863cb8`) with
    /// `kNISQueuesTheAudio` (1): the scene's camera take at parameter 0
    /// queues its post and its speech when their command is queue (1) or play
    /// (2), and the player waits for each. Without an audio system nothing is
    /// queued.
    fn audio_queue_for_start(&mut self) {
        self.pres.audio_speech_wait = false;
        self.pres.audio_post_wait = false;
        let Some(audio) = self.audio.as_ref() else { return };
        let track = self.directory.camera_track(self.pres.scene_hash);
        let Some(start) = self.directory.properties(self.pres.scene_hash).and_then(|props| self.tracks.start_audio(props, track)) else {
            return;
        };
        if matches!(start.post_command, 1 | 2) {
            audio.queue_nis_post(start.post_id);
            self.pres.audio_post_wait = true;
        }
        if matches!(start.speech_command, 1 | 2) {
            audio.queue_nis_speech(start.speech_character, start.speech_id);
            self.pres.audio_speech_character = start.speech_character;
            self.pres.audio_speech_id = start.speech_id;
            self.pres.audio_speech_wait = true;
        }
    }

    /// `ScenePlayer::IsAudioReadyForStart` (`0x82861b38`): each wait ends once
    /// the audio system has the speech (`IsSpeechQueued`) or the post
    /// (`IsPostQueued`) queued.
    fn is_audio_ready_for_start(&mut self) -> bool {
        if let Some(audio) = self.audio.as_ref() {
            if self.pres.audio_speech_wait && audio.is_speech_queued(self.pres.audio_speech_character, self.pres.audio_speech_id) {
                self.pres.audio_speech_wait = false;
            }
            if self.pres.audio_post_wait && audio.is_post_queued() {
                self.pres.audio_post_wait = false;
            }
        }
        !self.pres.audio_speech_wait && !self.pres.audio_post_wait
    }

    fn run_callback(&mut self, callback: Callback) {
        match callback {
            // SimulationBasedTransitionEndCallback -> SendVisualTransitionCompleteMsg.
            Callback::SimulationBasedEnd => self.to_simulation.push_back(ToSimulation::VisualTransitionComplete),
            // StopNISTransitionOutCallback -> SendNISTransitionOutCompleteMessage.
            Callback::StopNisTransitionOut => self.to_simulation.push_back(ToSimulation::TransitionOutComplete),
            // StopNISTransitionInCallback (0x8286e300): with useStreamPositionOnReturn the
            // streamer freezes at the stream position, which UpdateNIS waits for; without, fade
            // back out, then unload.
            Callback::StopNisTransitionIn => {
                if let Some(position) = self.pres.overrides.stream_position {
                    self.pres.wait_streamer = true;
                    self.pres.frozen_focus = Some(FrozenFocus::At(position));
                    self.pres.stream_focus_loaded = false;
                } else {
                    if let Some(now) = self.pres.transitions.end(Callback::StopNisTransitionOut) {
                        self.run_callback(now);
                    }
                    self.stop_destroy_unload_nis();
                }
            }
        }
    }

    /// `StartNIS` (`0x8286b8b8`): `mIsInOnlineGame` takes `Net::IsInOnlineGame` whatever the
    /// load state.
    fn start_nis(&mut self, hash: u32) {
        self.pres.in_online_game = self.online_game;
        if self.pres.load_state != 0 {
            return;
        }
        self.pres.do_kill = false;
        self.to_simulation.push_back(ToSimulation::FePushState);
        self.pres.load_state = 1;
        self.pres.load_timer = 0.0;
        if self.directory.is_non_vis_indicator(hash) {
            self.pres.vis_indicators_hidden = true;
        }
        self.pres.scene_hash = hash;
    }

    /// `IsNISSkippable` (`0x828608f0`).
    fn is_nis_skippable(&self) -> bool {
        if self.pres.load_state != 0 {
            return false;
        }
        let Some(scene) = self.pres.handle.as_ref() else { return false };
        if !self.directory.is_skippable(scene.hash) {
            return false;
        }
        let kind = self.directory.scene_type(scene.hash);
        !(scene.time_elapsed < 2.0
            && (kind == scene_type::SPOT_INTRO || kind == scene_type::PRO_CHALLENGE_INTRO || kind == scene_type::STORY))
    }

    /// `UpdateNIS` (`0x82876608`).
    fn update_nis(&mut self, dt: f32) {
        self.update_load_state(dt);
        // mWaitStreamerBeforeTransitionOut: once the restored stream position is loaded, fade
        // back out and unload.
        if self.pres.wait_streamer && self.pres.stream_focus_loaded {
            if let Some(now) = self.pres.transitions.end(Callback::StopNisTransitionOut) {
                self.run_callback(now);
            }
            self.stop_destroy_unload_nis();
            self.pres.wait_streamer = false;
        }
        let in_transition = self.pres.transitions.in_transition();
        if self.pres.do_kill && !self.pres.stopped && !in_transition && self.pres.load_state == 0 {
            self.pres.do_kill = false;
            self.stop_nis();
        }
        let playing = self.pres.handle.as_ref().map(|s| s.play_state == PlayState::Playing);
        if !self.pres.stopped && self.pres.load_state == 0 && !in_transition && playing == Some(false) {
            self.pres.kill_type = KillType::Completed;
            self.stop_nis();
        }
    }

    /// `UpdateLoadState` (`0x82875bc0`).
    fn update_load_state(&mut self, dt: f32) {
        if self.pres.load_state == 0 {
            return;
        }
        match self.pres.load_state {
            1 => {
                // Waits for the simulation to be paused, unless the scene started in an online
                // game (`mIsInOnlineGame`).
                if self.pres.in_online_game || self.sim.paused {
                    self.audio_queue_for_start();
                    // ScenePlayer::FERequestPreload (0x8286c618) with
                    // kNISQueuesTheFE (1): the take's overlay states.
                    let overlays = self
                        .directory
                        .properties(self.pres.scene_hash)
                        .and_then(|props| self.tracks.overlays(props, props.camera_track));
                    self.pres.fe_wait_for_preload = overlays.is_some();
                    if let Some(states) = overlays {
                        self.to_simulation.push_back(ToSimulation::FeRequestPreload(states));
                    }
                    let mut snap_scene = false;
                    if let Some(props) = self.directory.properties(self.pres.scene_hash) {
                        // The overrides' location values (0x82875d3c) in place of the record's.
                        let [location, orientation] = self.pres.overrides.location.unwrap_or([props.location, props.orientation]);
                        self.pres.frame = SceneFrame { location, orientation, normal: [0.0, 1.0, 0.0] };
                        snap_scene = props.bit_field & 1 != 0;
                    }
                    self.pres.snap_scene = snap_scene;
                    let snap_characters = !snap_scene && self.pre_snap_fill_location_list();
                    if snap_scene || snap_characters {
                        self.pres.load_state = 2;
                        self.pres.scene_focus = Some(self.pres.frame.location);
                        self.pres.stream_focus_loaded = false;
                    } else {
                        // SceneLoader::LoadAndCallback(NISSceneCallbackFromLoad).
                        self.pres.load_requested = true;
                        self.pres.load_state = 5;
                    }
                }
            }
            2 => {
                // An online game does not wait for the stream focus (`Net::IsInOnlineGame`).
                if self.pres.stream_focus_loaded || self.online_game {
                    if self.pres.snap_scene {
                        let [x, y, z] = self.pres.frame.location;
                        self.to_simulation.push_back(ToSimulation::WorldCollisionQuery([x, y, z, 0.0]));
                        self.pres.snap_scene = false;
                        self.pres.load_state = 3;
                    } else {
                        self.snap_locations_to_ground();
                        self.pres.load_state = 4;
                    }
                }
            }
            3 => {
                if self.pres.ground_received {
                    self.pres.ground_received = false;
                    self.pre_snap_fill_location_list();
                    self.snap_locations_to_ground();
                    self.pres.load_state = 4;
                }
            }
            4 => {
                // UpdateLocationListWithSnappedLocations (0x82868a00): the
                // snapped locations into the flagged slots, in order.
                if let Some(snapped) = self.pres.snapped.take() {
                    let mut snapped = snapped.into_iter();
                    for (location, snap) in &mut self.pres.locations {
                        if *snap && let Some(position) = snapped.next() {
                            *location = position;
                        }
                    }
                    self.pres.load_requested = true;
                    self.pres.load_state = 5;
                }
            }
            5 => {
                let transition = !self.pres.transitions.in_transition() || self.pres.transitions.direction() == Direction::Out;
                // ScenePlayer::IsFEPreloaded (0x82861c20).
                if self.pres.scene_loaded && self.is_audio_ready_for_start() && !self.pres.fe_wait_for_preload && transition {
                    self.pres.scene_loaded = false;
                    let track = self.directory.camera_track(self.pres.scene_hash);
                    // SelectSpeed with the default overrides.
                    self.pres.mults.override_mult = 1.0;
                    self.pres.handle = self.create_scene(self.pres.scene_hash, track);
                    self.pres.load_state = 6;
                }
            }
            6 => {
                // mBitField bit 1 waits for the stream focus outside an online game
                // (`Net::IsInOnlineGame`).
                let streaming = self.directory.properties(self.pres.scene_hash).is_some_and(|props| props.bit_field & 2 != 0)
                    && !self.online_game
                    && !self.pres.stream_focus_loaded;
                if !streaming {
                    if !self.pres.transitions.in_transition()
                        && let Some(now) = self.pres.transitions.end(Callback::StopNisTransitionOut)
                    {
                        self.run_callback(now);
                    }
                    self.pres.load_state = 7;
                }
            }
            7 => {
                // mHideAISkatersForPhotoOutro (0x8287645c..0x828764b0): a photo outro whose
                // camera take shoots fixed frames.
                let hash = self.pres.scene_hash;
                self.pres.hide_ai_skaters_for_photo_outro = self.directory.scene_type(hash) == scene_type::PHOTO_OUTRO
                    && self
                        .directory
                        .properties(hash)
                        .is_some_and(|props| self.tracks.fixed_frame_photo_count(props, self.directory.camera_track(hash)) > 0);
                self.to_simulation.push_back(ToSimulation::Playback);
                // ScenePlayer::Play -> Scene::UnPause -> BindToGame
                // (0x82865c18): ClearFreezeTimeControl, then the channels'
                // OnTakeStart when the camera take is at its start (without
                // one, the scene's time).
                if let Some(scene) = self.pres.handle.as_mut() {
                    scene.play_state = PlayState::Playing;
                    self.pres.mults.freeze = 1.0;
                    self.pres.mults.freeze_count = 0.0;
                    let at_start = scene.track.as_ref().map_or(scene.time_elapsed == 0.0, |t| t.parameter() == 0.0);
                    if at_start {
                        self.pres.overlay_state = i64::from(STATE_UNKNOWN);
                        self.pres.freeze_previous = 0xffff;
                        self.pres.audio_effect_previous = 0xffff;
                        // The music channel's OnTakeStart (0x826cb400) resets
                        // its interval; the speech and post channels' (0x826d6c18)
                        // their take change too.
                        self.pres.audio_music.previous = 0xffff;
                        self.pres.audio_speech = AudioChannel::default();
                        self.pres.audio_post = AudioChannel::default();
                    }
                } else {
                    self.pres.kill_type = KillType::Error;
                    self.stop_destroy_unload_nis();
                }
                if self.pres.load_timer >= 1.5 {
                    self.to_simulation.push_back(ToSimulation::LoadingOverlay(false));
                }
                self.pres.load_state = 0;
            }
            _ => {}
        }
        if self.pres.load_state != 0 {
            if self.pres.load_timer < 1.5 && self.pres.load_timer + dt >= 1.5 {
                self.to_simulation.push_back(ToSimulation::LoadingOverlay(true));
            }
            self.pres.load_timer += dt;
        } else {
            self.pres.load_timer = 0.0;
        }
    }

    /// `ScenePlayer::Create` (`0x82878590`) and `Scene::Init` (`0x82877ac8`).
    /// `PreSnapFillLocationList` (`0x82874ad8`): every record's location,
    /// whether it snaps (`mSnapToGround`), and whether any does.
    fn pre_snap_fill_location_list(&mut self) -> bool {
        self.pres.locations = self
            .directory
            .properties(self.pres.scene_hash)
            .map(|props| self.directory.characters(props).iter().map(|r| (r.location, r.snap_to_ground != 0)).collect())
            .unwrap_or_default();
        self.pres.locations.iter().any(|&(_, snap)| snap)
    }

    /// `SceneManagerImpl::SnapLocationsToGround` (`0x8286e1d0`): the flagged
    /// locations through the scene's `ComputeTransform` to the simulation.
    /// `SceneLocationSnapper::SnapLocationsToGround` (`0x8286d6d8`) keeps
    /// the inverse (`rw::math::vpu::Inverse` `0x82600f38`) for the answer;
    /// each world location leaves as (x, y, z, x) (`vperm <0,5,0,0>`, then
    /// z, 0x8286d9b0..b4).
    fn snap_locations_to_ground(&mut self) {
        let frame = self.pres.frame;
        let m = compute_transform(frame.location, frame.orientation, frame.normal);
        let transform: [f32; 16] = std::array::from_fn(|i| m[i / 4][i % 4]);
        if let Ok(inverse) = try_inverse(transform, &mut HostReckoningNumeric) {
            self.pres.snap_inverse = inverse;
        }
        self.pres.snapped = None;
        let world = self
            .pres
            .locations
            .iter()
            .filter(|&&(_, snap)| snap)
            .map(|&(location, _)| {
                let p = transform_point(transform, location);
                [p[0], p[1], p[2], p[0]]
            })
            .collect();
        self.to_simulation.push_back(ToSimulation::LocationListSnapQuery(world));
    }

    fn create_scene(&mut self, hash: u32, track: u8) -> Option<Scene> {
        let props = self.directory.properties(hash)?.clone();
        // SetPlaybackCamera's OnTakeSetup (0x82a3be44): the overlay
        // channel's (0x826c7730) resets its state, the speech and post
        // channels' (0x826d6c18) their interval and take change; the freeze,
        // effect and music channels' are empty.
        self.pres.overlay_state = i64::from(STATE_UNKNOWN);
        self.pres.audio_speech = AudioChannel::default();
        self.pres.audio_post = AudioChannel::default();
        // ClearFreezeTimeControl.
        self.pres.mults.freeze = 1.0;
        self.pres.mults.freeze_count = 0.0;
        let replay = props.is_replay_on != 0;
        // Scene::Init (0x82877ac8): a replay-controlled scene lasts its replay frames at
        // kUpdateTime each, at least 1.5 s (a basic NIS has none).
        let time_length = if replay {
            let (start, end) = self.pres.replay.unwrap_or((0, 0));
            #[expect(clippy::cast_precision_loss, clippy::cast_possible_wrap, reason = "(float)(int)(end - start)")]
            let length = end.wrapping_sub(start) as i32 as f32 * (1.0 / 60.0);
            if length >= 1.5 { length } else { 1.5 }
        } else {
            props.length
        };
        let mut scene = Scene {
            hash,
            kind: props.kind,
            frame: self.pres.frame,
            play_state: PlayState::Stopped,
            time_elapsed: 0.0,
            time_delta: 0.0,
            time_length,
            camera_track_found: false,
            camera_finished: false,
            subtitler: None,
            track: None,
            characters: Vec::new(),
        };
        // AddEntities, every entity's SetSceneTransform with mTransform, then
        // SetTimeElapsed(0).
        if let Some(source) = self.characters.as_ref() {
            let locations: Vec<_> = self.pres.locations.iter().map(|&(location, _)| location).collect();
            let players = &self.pres.overrides.player_indexes;
            scene.characters = instance_characters(self.directory.characters(&props), &locations, players, source.as_ref());
            let m = compute_transform(scene.frame.location, scene.frame.orientation, scene.frame.normal);
            let transform = std::array::from_fn(|i| m[i / 4][i % 4]);
            for character in &mut scene.characters {
                character.set_scene_transform(transform);
            }
        }
        scene.set_time_elapsed(0.0);
        // CameraControlConnect -> ICESceneBridge::ConnectScene.
        if let Some((mut track, set_time)) = self.tracks.connect(&props, track, replay.then_some(time_length)) {
            if self.pres.full_speed_hom_replay {
                track.disable_slow_motion();
            }
            scene.camera_track_found = true;
            scene.track = Some(track);
            if let Some(time) = set_time {
                scene.set_time_elapsed(time);
            }
        }
        // GetCameraComponentsAtTimeZero -> ComputeTransform(eye, look
        // direction, scene normal) -> SetICEMatrix, else the game camera's
        // matrix; the field of view and near plane keep their values. The
        // ICE camera is active from here.
        let replay_camera = self.replay_matrix();
        self.pres.ice.matrix = scene
            .track
            .as_ref()
            .and_then(|t| {
                let anchors = CameraAnchors {
                    anchor: self.pres.anchor,
                    axes: self.pres.anchor_axes,
                    hips: self.pres.local_hips,
                    hips_forward: self.pres.local_hips_forward,
                    replay: replay_camera,
                    ..CameraAnchors::default()
                };
                t.time_zero(&scene.frame, &anchors)
            })
            .map(|(eye, look)| first_camera_matrix(eye, look, scene.frame.normal));
        self.pres.ice_camera_active = true;
        self.pres.ice_postfx_active = true;
        // SceneManagerImpl::StartNIS and Scene::BindToGame reset TheICEWorldManager.
        self.pres.ice_world = IceWorld::default();
        // A replay-controlled, frontend or "ViewScene" scene leaves the
        // streamer alone; the others move the focus to the camera and freeze
        // it.
        let controls_streamer = !replay && props.kind != scene_type::FRONTEND && hash != scene_hash("ViewScene");
        if controls_streamer {
            self.pres.frozen_focus =
                Some(self.pres.ice.matrix.map_or(FrozenFocus::GameCamera, |m| FrozenFocus::At([m[12], m[13], m[14]])));
            self.pres.scene_focus = None;
            self.pres.stream_focus_loaded = false;
        }
        // NotifyNISPlaybackStart (0x82878328) with the scene's type, the
        // player's special flags (none for a basic NIS), whether it controls
        // the replay and the delta multiplier times its length.
        if let Some(audio) = self.audio.as_ref() {
            audio.notify_nis_playback_start(i32::from(props.kind), self.pres.special_flags, replay, self.pres.mults.delta_mult() * scene.time_length);
        }
        // SubTitler(scene name), Start.
        let data = self.subtitle_big.as_ref().and_then(|big| super::subtitles::load(big, &self.language, &props.name));
        scene.subtitler = Some(SubTitler::start(data));
        Some(scene)
    }

    /// `ScenePlayer::Update` (`0x82872558`) and `Scene::Update` (`0x82870bb8`).
    fn scene_player_update(&mut self, dt: f32) {
        if self.pres.mults.freeze_count > 0.0 {
            self.pres.mults.freeze_count -= dt;
            if self.pres.mults.freeze_count <= 0.0 {
                self.pres.mults.freeze = 1.0;
                self.pres.mults.freeze_count = 0.0;
            }
        }
        let mult = self.pres.mults.delta_mult();
        let Some(scene) = self.pres.handle.as_mut() else { return };
        if scene.play_state != PlayState::Playing {
            return;
        }
        scene.time_delta = mult * dt;
        scene.add_time(scene.time_delta);
        if let Some(subtitler) = scene.subtitler.as_mut()
            && let Some(hal) = subtitler.update((scene.time_delta * 100_000.0) as u32)
        {
            self.to_simulation.push_back(ToSimulation::SetSubtitle(hal));
        }
        if mult > 0.0 && (!scene.camera_track_found || scene.camera_finished) {
            if scene.kind == scene_type::PHOTO_OUTRO {
                self.pres.mults.freeze = 0.0;
                self.events.push(NisEvent::EnterPhotoView);
            } else if scene.hash == scene_hash("ViewScene") {
                // Scene::Update82870bb8 resets the entire scene at the end
                // of its camera take, including entity animation elapsed time.
                scene.set_time_elapsed(0.);
                scene.camera_finished=false;
            } else {
                // Scene::Stop (0x82870b38). "ViewScene" loops instead; it is
                // only played by the trick book's view mode.
                scene.play_state = PlayState::Stopped;
            }
        }
    }

    /// `SceneCamera::UpdateFrameBegin` (`0x828645d0`).
    fn scene_camera_frame_begin(&mut self, dt: f32) {
        let ice_active = self.pres.ice_camera_active;
        let Some(scene) = self.pres.handle.as_mut() else { return };
        let Some(track) = scene.track.as_mut() else { return };
        // ICEManager::Update.
        let update = track.update(scene.time_elapsed);
        let sim_time = track.sim_time();
        let overlay = track.overlay();
        let freeze_interval = track.freeze_interval();
        let audio_effect_interval = track.audio_effect_interval();
        let audio_music_interval = track.audio_music_interval();
        let audio_speech_interval = track.audio_speech_interval();
        let audio_post_interval = track.audio_post_interval();
        // ICEManager::Update's OnTakeChange on an assembly cut: the freeze
        // and audio channels' (0x826d6e78) skip the next interval.
        if update.set_time.is_some() {
            self.pres.freeze_previous = 0xffff;
            self.pres.freeze_take_change = true;
            self.pres.audio_effect_previous = 0xffff;
            self.pres.audio_effect_take_change = true;
            let take_change = AudioChannel { previous: 0xffff, take_change: true };
            self.pres.audio_music = take_change;
            self.pres.audio_speech = take_change;
            self.pres.audio_post = take_change;
        }
        let mut audio_effects = Vec::new();
        if ice_active
            && scene.play_state == PlayState::Playing
            && let Some(current) = audio_effect_interval.and_then(|i| u16::try_from(i).ok())
        {
            // ICEUserChannelAudioEffect::UpdateFrameBegin (0x826ce9f8): the
            // freeze channel's walk over channel 30.
            if self.pres.audio_effect_previous == 0xffff {
                if self.pres.audio_effect_take_change {
                    self.pres.audio_effect_take_change = false;
                } else {
                    audio_effects.push(track.audio_effect_command(usize::from(current)));
                }
            } else {
                let mut i = self.pres.audio_effect_previous.wrapping_add(1);
                while i <= current {
                    audio_effects.push(track.audio_effect_command(usize::from(i)));
                    i = i.wrapping_add(1);
                    if i == 0 {
                        break;
                    }
                }
            }
            self.pres.audio_effect_previous = current;
        }
        let walking = ice_active && scene.play_state == PlayState::Playing;
        let interval = |interval: Option<usize>| interval.filter(|_| walking).and_then(|i| u16::try_from(i).ok());
        let music_commands: Vec<_> = interval(audio_music_interval)
            .map(|current| self.pres.audio_music.walk(current).into_iter().map(|i| track.audio_music_command(i)).collect())
            .unwrap_or_default();
        let speech_commands: Vec<_> = interval(audio_speech_interval)
            .map(|current| self.pres.audio_speech.walk(current).into_iter().map(|i| track.audio_speech_command(i)).collect())
            .unwrap_or_default();
        let post_commands: Vec<_> = interval(audio_post_interval)
            .map(|current| self.pres.audio_post.walk(current).into_iter().map(|i| track.audio_post_command(i)).collect())
            .unwrap_or_default();
        let mut freeze_commands = Vec::new();
        if ice_active
            && scene.play_state == PlayState::Playing
            && let Some(current) = freeze_interval.and_then(|i| u16::try_from(i).ok())
        {
            // ICEUserChannelFreeze::UpdateFrameBegin (0x826d2498): every
            // interval crossed fires once, in order; the first after a take
            // change and backward jumps fire nothing.
            if self.pres.freeze_previous == 0xffff {
                if self.pres.freeze_take_change {
                    self.pres.freeze_take_change = false;
                } else {
                    freeze_commands.push(track.freeze_command(usize::from(current)));
                }
            } else {
                let mut i = self.pres.freeze_previous.wrapping_add(1);
                while i <= current {
                    freeze_commands.push(track.freeze_command(usize::from(i)));
                    i = i.wrapping_add(1);
                    if i == 0 {
                        break;
                    }
                }
            }
            self.pres.freeze_previous = current;
        }
        if let Some(time) = update.set_time {
            scene.set_time_elapsed(time);
        }
        if update.finished {
            scene.camera_finished = true;
        }
        // ICECameraMover::UpdateFrameBegin -> UpdateSimTime, then the user
        // channels.
        if ice_active {
            let _ = dt;
            self.pres.mults.scene = sim_time;
            // ICEUserChannelOverlays::UpdateFrameBegin (0x826d1a28): a
            // playing scene with a positive delta multiplier sends a new
            // state below eSk8HUDNISState_Max.
            if scene.play_state == PlayState::Playing
                && self.pres.mults.delta_mult() > 0.0
                && let Some(state) = overlay
                && state != self.pres.overlay_state
                && state < i64::from(STATE_MAX)
            {
                self.to_simulation.push_back(ToSimulation::FeEvent(state as i32));
                self.pres.overlay_state = state;
            }
            // ICEUserChannelAudioEffect::FireEvent (0x826d6648): command 2 is
            // IAudioInterface::PlayNISSfx(id).
            for (command, id) in audio_effects {
                if command == 2 {
                    self.nis_sfx.push(id as i32);
                }
            }
            // ICEUserChannelAudioMusic::FireEvent (0x826cae18),
            // ICEUserChannelAudioSpeech::FireEvent (0x826d6ba8) and
            // ICEUserChannelAudioPost::FireEvent (0x826cb388): command 1
            // queues, 2 plays.
            if let Some(audio) = self.audio.as_ref() {
                for (command, id) in music_commands {
                    match command {
                        1 => audio.queue_nis_music(id),
                        2 => audio.play_nis_music(id),
                        _ => {}
                    }
                }
                for (command, character, id) in speech_commands {
                    match command {
                        1 => audio.queue_nis_speech(character, id),
                        2 => audio.play_nis_speech(character, id),
                        _ => {}
                    }
                }
                for (command, id) in post_commands {
                    match command {
                        1 => audio.queue_nis_post(id),
                        2 => audio.play_nis_post(),
                        _ => {}
                    }
                }
            }
            for (command, frames) in freeze_commands {
                self.fire_freeze_event(command, frames);
            }
        }
    }

    /// `ICEUserChannelFreeze::FireEvent` (`0x826d1b50`); a frame is 1/30 s
    /// (`(float)frames * 0.033333335`).
    fn fire_freeze_event(&mut self, command: u32, frames: u32) {
        // PC HoM policy: skip cinematic Timeout/Permanent/Owned holds. Keep
        // photo commands, skip/completion notifications and time jumps intact.
        if self.pres.full_speed_hom_replay && matches!(command, 1..=3) {
            return;
        }
        #[expect(clippy::cast_precision_loss, reason = "the native converts the count to float")]
        let seconds = frames as f32 * f32::from_bits(0x3d08_8889);
        match command {
            // Timeout and Owned: SetFreezeTimeMult(0), SetFreezeResetCounter.
            1 | 3 => {
                if frames != 0 {
                    self.pres.mults.freeze = 0.0;
                    self.pres.mults.freeze_count = seconds;
                }
            }
            // Permanent.
            2 => self.pres.mults.freeze = 0.0,
            // PhotoShot: with time running, the freeze, then
            // eSk8HUDNISState_Take_NonFixedPhoto to the front end.
            4 => {
                if self.pres.mults.delta_mult() > 0.0 {
                    if frames != 0 {
                        self.pres.mults.freeze = 0.0;
                        self.pres.mults.freeze_count = seconds;
                    }
                    self.to_simulation.push_back(ToSimulation::FeEvent(86));
                }
            }
            // Killable.
            6 => self.events.push(NisEvent::PlayingKillable),
            // Jump: ICEScene::SetTime.
            7 => {
                if let Some(scene) = self.pres.handle.as_mut() {
                    scene.set_time_elapsed(seconds);
                }
            }
            // PhotoShot_FixedFrame: with time running and the local skater in the last frame's
            // view, the skater instanced frozen at the frame's end, the freeze, the bloom pulse
            // from the threshold to s_fBloom and back, and eSk8HUDNISState_Take_FixedPhoto to the
            // front end.
            5 => {
                if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
                    eprintln!(
                        "career: NIS fixed-frame photo: delta {} hips {:?} radius {:?} camera {:?} fov {} in view {}",
                        self.pres.mults.delta_mult(),
                        self.pres.local_hips,
                        self.pres.local_radius,
                        self.pres.ice.matrix,
                        self.pres.ice.fov,
                        self.local_skater_in_view()
                    );
                }
                if self.pres.mults.delta_mult() > 0.0 && self.local_skater_in_view() {
                    self.pres.instance_frozen_skater = true;
                    if frames != 0 {
                        self.pres.mults.freeze = 0.0;
                        self.pres.mults.freeze_count = seconds;
                    }
                    self.pres.ice_postfx.bloom_anim = BloomAnim {
                        base: self.pres.ice_postfx.bloom[0],
                        value: PHOTO_BLOOM,
                        up: PHOTO_BLOOM_UP_TIME,
                        down: PHOTO_BLOOM_BACK_TIME,
                        time: 0.0,
                    };
                    self.to_simulation.push_back(ToSimulation::FeEvent(85));
                }
            }
            8 => self.events.push(NisEvent::FreezeCommandNotPorted(command)),
            _ => {}
        }
    }

    /// `Frustum::TestSphere_VMX` (`0x8282c208`) on the last frame's camera for the local
    /// skater: `GetBoundsCenter` (its hips), `m_BR` and `GetMinPixelSize` (19, `0x827d0ac8`).
    /// The planes are `Frustum::ComputeFrustum`'s (`0x8282bd28`): w + x, w - x, w - y, w + y
    /// and w + z of the view-projection, normalized, here over the renderer's projection of the
    /// ICE camera. Its far plane (the rendering block's) culls only far beyond a photo's reach
    /// and is not tested. A sphere wholly behind a plane is outside; else it must span the
    /// pixels: `GetPixelSize` (`0x828268a8`) is `m_H` r / (d - r), or `m_H` closer than 2r,
    /// with `m_H` half the 1280-pixel width over tan(fov / 2) (`Frustum::CalculateH`
    /// `0x82826830`).
    fn local_skater_in_view(&self) -> bool {
        let (Some(matrix), Some(hips), Some(radius)) = (self.pres.ice.matrix, self.pres.local_hips, self.pres.local_radius) else {
            return false;
        };
        let camera = crate::skeleton_root_transforms::SkeletonRootMatrix::known(matrix);
        let fov = self.pres.ice.fov;
        let clip = crate::camera_session::producer_projection::project_producer(
            camera, camera, fov, fov, self.pres.ice.near_plane, f32::MAX, false, false, -1, 0,
        )
        .clip_from_world;
        let row = |r: usize| [clip[r], clip[4 + r], clip[8 + r], clip[12 + r]];
        let (x, y, z, w) = (row(0), row(1), row(2), row(3));
        let plane = |side: [f32; 4], sign: f32| -> [f32; 4] { std::array::from_fn(|i| sign.mul_add(side[i], w[i])) };
        for p in [plane(x, 1.0), plane(x, -1.0), plane(y, -1.0), plane(y, 1.0), plane(z, 1.0)] {
            let length = p[2].mul_add(p[2], p[1].mul_add(p[1], p[0] * p[0])).sqrt();
            let distance = (p[2].mul_add(hips[2], p[1].mul_add(hips[1], p[0] * hips[0])) + p[3]) / length;
            if distance < -radius {
                return false;
            }
        }
        let offset: [f32; 3] = std::array::from_fn(|lane| hips[lane] - matrix[12 + lane]);
        let radians = fov.to_radians();
        let distance = offset[2].mul_add(offset[2], offset[1].mul_add(offset[1], offset[0] * offset[0])).sqrt();
        let h = 0.5 * 1280.0 / (radians * 0.5).tan();
        #[expect(clippy::cast_possible_truncation, reason = "GetPixelSize returns the int of its float")]
        let pixels = if distance - radius > radius { (h * radius / (distance - radius)) as i32 } else { h as i32 };
        #[expect(clippy::cast_precision_loss, reason = "the native compares the pixels as a float")]
        let pixels = pixels as f32;
        pixels >= MIN_PIXEL_SIZE
    }

    /// `SceneCamera::UpdateFrameEnd` (`0x828646f8`).
    fn scene_camera_frame_end(&mut self, dt: f32) {
        // ICEUserChannelFreeze::UpdateFrameEnd (0x826cf1a8).
        if std::mem::take(&mut self.pres.instance_frozen_skater) {
            self.pres.frozen_skater = true;
        }
        if !self.pres.ice_camera_active {
            return;
        }
        let mult = self.pres.mults.delta_mult();
        let (anchor, axes, hips) = (self.pres.anchor, self.pres.anchor_axes, self.pres.local_hips);
        let hips_forward = self.pres.local_hips_forward;
        let replay = self.replay_matrix();
        self.pres.camera_built = false;
        let Some(scene) = self.pres.handle.as_mut() else { return };
        let frame = scene.frame;
        // Scene::GetEntityLocation (0x82865490) indexes the entities
        // AddEntities added, the characters in order.
        let entities = scene.characters.iter().map(|c| c.object_transform().map(|m| [m[12], m[13], m[14]])).collect();
        let anchors = CameraAnchors { anchor, axes, hips, hips_forward, replay, entities };
        if let Some(track) = scene.track.as_mut()
            && let Some(camera) = track.camera(&frame, &anchors, &mut self.pres.mover, mult * dt)
        {
            self.pres.ice.matrix = Some(camera.matrix);
            self.pres.camera_built = true;
            self.pres.ice.fov = camera.fov;
            if let Some(near) = camera.near_plane {
                self.pres.ice.near_plane = near;
            }
            self.pres.ice_postfx.apply_take(camera.bloom, camera.fade, camera.postfx);
        }
    }

    /// `StopNIS` (`0x8286e438`).
    fn stop_nis(&mut self) {
        let Some(scene) = self.pres.handle.as_mut() else { return };
        self.pres.stopped = true;
        let kind = self.directory.scene_type(scene.hash);
        if kind == scene_type::FRONTEND || kind == scene_type::VIEW_MODE {
            self.stop_destroy_unload_nis();
            return;
        }
        // ScenePlayer::Pause.
        if scene.play_state == PlayState::Playing {
            scene.play_state = PlayState::Paused;
        }
        self.pres.transitions.begin(Callback::StopNisTransitionIn);
        // SendNISTransitionInBeginMessage.
        self.to_simulation.push_back(ToSimulation::TransitionInBegin);
    }

    /// `StopDestroyUnloadNIS` (`0x8286dfe0`) with `Scene::Purge`.
    fn stop_destroy_unload_nis(&mut self) {
        self.pres.replay = None;
        self.pres.full_speed_hom_replay = false;
        if self.pres.handle.take().is_some() {
            self.pres.scene_focus = None;
            // Scene::Purge's NotifyNISPlaybackFinish (0x8286aa54).
            if let Some(audio) = self.audio.as_ref() {
                audio.notify_nis_playback_finish();
            }
        }
        // Scene::Purge (0x8286a908): the game post-fx return; the game
        // streamer controller and the game camera too unless the camera holds
        // at the end.
        self.pres.ice_postfx_active = false;
        if !self.pres.hold_camera {
            self.pres.ice_camera_active = false;
            self.pres.frozen_focus = None;
        }
        self.pres.vis_indicators_hidden = false;
        self.pres.stopped = false;
        // SendCompletionMessages.
        self.to_simulation.push_back(ToSimulation::Complete);
    }

    /// The scene's subtitle keys as `Scene::Init` would load them.
    #[must_use]
    pub fn subtitle_keys(&self, scene: &str) -> Option<Vec<SubtitleKey>> {
        self.subtitle_big.as_ref().and_then(|big| super::subtitles::load(big, &self.language, scene))
    }
}

/// `Scene::Init`'s first ICE matrix: `Scene::ComputeTransform` of the
/// time-zero eye, its normalised look direction
/// (`GetCameraComponentsAtTimeZero`) and the scene normal.
fn first_camera_matrix(eye: [f32; 3], look: [f32; 3], normal: [f32; 3]) -> [f32; 16] {
    let direction = vmx_normalize([look[0] - eye[0], look[1] - eye[1], look[2] - eye[2]]);
    let m = compute_transform(eye, direction, normal);
    std::array::from_fn(|i| m[i / 4][i % 4])
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
