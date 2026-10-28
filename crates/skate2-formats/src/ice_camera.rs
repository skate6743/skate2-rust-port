//! ICE 2.96 camera data (`data\scene\ice_000.bin`) and its playback.
//! `ICEManager::LoadICECameras` (`0x82a3e8a8`) reads one chunk per context
//! (`ICE::RegisterContext` `0x82a4e2d8`: context index `i` has id
//! `0x80000003 + i`, the recorded shakes `0x80000002`); `LoadCameraSet`
//! (`0x82a3e600`) adds each group's takes in file order, those with assembly
//! intervals to the group's `AssemblyList`, the others to its `TakeList`.
//! `ICETake::SetDataPointers` (`0x82a366f0`) lays a take's element data out by
//! ascending element id; chunk words and raw floats are little-endian, packed
//! values are big-endian bit streams.
use std::collections::BTreeMap;
#[path = "ice_camera_math.rs"]
mod math;
#[path = "ice_camera_editor.rs"]
pub mod editor;

type Result<T> = std::result::Result<T, String>;

pub const ELEMENT_COUNT: usize = 146;
const CHANNEL_COUNT: usize = 37;
const ASSEMBLY_CHANNEL: usize = 35;
/// The recorded-shake context id.
pub const SHAKE_CONTEXT: u32 = 0x8000_0002;

/// Element ids (`ICE::RegisterElements` `0x82a4e8b8`).
pub mod element {
    pub const EYE_X: usize = 0;
    pub const LOOK_X: usize = 3;
    pub const DUTCH: usize = 6;
    pub const LENS_LENGTH: usize = 9;
    pub const LENS_LENGTH_AUX: usize = 10;
    pub const CLIPPING_NEAR: usize = 14;
    pub const SHAKE_AMPLITUDE: usize = 23;
    pub const SHAKE_FREQUENCY: usize = 24;
    pub const TIME_SCALE: usize = 25;
    pub const LETTERBOX: usize = 26;
    pub const FADE: usize = 27;
    pub const TONEMAP_VALUE: usize = 34;
    pub const BLOOM_INTENSITY: usize = 36;
    pub const BLOOM_THRESHOLD: usize = 37;
    pub const FISHEYE_MULTIPLIER: usize = 47;
    pub const INTERPOLATION_FAR: usize = 49;
    pub const INTERPOLATION_NEAR: usize = 50;
    pub const BRIGHTNESS_FAR: usize = 51;
    pub const BRIGHTNESS_NEAR: usize = 52;
    pub const CONTRAST_FAR: usize = 53;
    pub const CONTRAST_FAR_MIDPOINT: usize = 54;
    pub const CONTRAST_NEAR: usize = 55;
    pub const CONTRAST_NEAR_MIDPOINT: usize = 56;
    pub const SATURATE_FAR: usize = 57;
    pub const SATURATE_NEAR: usize = 58;
    pub const TINT_NEAR_COLOR_X: usize = 59;
    pub const TINT_FAR_COLOR_X: usize = 62;
    pub const SCANLINE_Y_OFFSET: usize = 74;
    pub const RADIALBLUR: usize = 76;
    pub const SHAKE_QUAT_X: usize = 81;
    pub const SHAKE_QUAT_Y: usize = 82;
    pub const SHAKE_QUAT_Z: usize = 83;
    pub const SHAKE_POS_X: usize = 84;
    pub const SHAKE_POS_Y: usize = 85;
    pub const SHAKE_POS_Z: usize = 86;
    pub const SPACE_EYE: usize = 89;
    pub const SPACE_LOOK: usize = 90;
    pub const AVATAR_EYE: usize = 91;
    pub const AVATAR_LOOK: usize = 92;
    pub const AVOID_STATIC_OBJECTS: usize = 95;
    pub const PLAYER_SPACE_LOCK_TO_RENDER_OBJECT: usize = 96;
    pub const PLAYER_SPACE_NOROLL: usize = 97;
    pub const PLAYER_SPACE_LAG: usize = 98;
    pub const PLAYER_SPACE_BUNGEE: usize = 99;
    pub const LENS_OVERRIDE: usize = 101;
    pub const SHAKE_TYPE: usize = 105;
    pub const OVERLAY: usize = 107;
    pub const FADE_TO_COLOR: usize = 108;
    pub const TONEMAP_OVERRIDE: usize = 109;
    pub const BLOOM_OVERRIDE: usize = 112;
    pub const FISHEYE_OVERRIDE: usize = 115;
    pub const NEAR_FAR_OVERRIDE: usize = 116;
    pub const BRIGHTNESS_OVERRIDE: usize = 117;
    pub const CONTRAST_OVERRIDE: usize = 118;
    pub const SATURATE_OVERRIDE: usize = 119;
    pub const TINT_OVERRIDE: usize = 122;
    pub const SCANLINE_OVERRIDE: usize = 127;
    pub const RADIALBLUR_OVERRIDE: usize = 129;
    pub const AUDIO_EFFECT_CMD: usize = 132;
    pub const AUDIO_EFFECT_ID: usize = 133;
    pub const AUDIO_MUSIC_CMD: usize = 134;
    pub const AUDIO_MUSIC_ID: usize = 135;
    pub const AUDIO_SPEECH_CMD: usize = 136;
    pub const AUDIO_SPEECH_ID: usize = 137;
    pub const AUDIO_SPEECH_CHARACTER: usize = 138;
    pub const AUDIO_POST_CMD: usize = 139;
    pub const AUDIO_POST_ID: usize = 140;
    pub const FREEZE_CMD: usize = 141;
    pub const FREEZE_TIME: usize = 142;
    pub const TAKE_START: usize = 143;
    pub const TAKE_NUMBER: usize = 144;
    pub const CONTAINS_SUBTAKE: usize = 145;
}

/// `ICELITEElementDesc`: channel, key (false) or interval (true) element,
/// `nDataType` (1/2 unsigned, 3 quantised float, 4 raw float), `nDataBits`,
/// default, min, max, and the cubic and tangent element ids `GetSlope`
/// (`0x82a390e8`) reads (`0xFFFF` = none).
#[derive(Clone, Copy, Debug)]
struct Element {
    id: usize,
    channel: usize,
    interval: bool,
    kind: u8,
    bits: usize,
    default: f32,
    low: f32,
    high: f32,
    cubic: usize,
    tangent: usize,
}

#[expect(clippy::too_many_arguments, reason = "one row of the native descriptor table")]
const fn el(id: usize, channel: usize, interval: bool, kind: u8, bits: usize, default: f32, low: f32, high: f32, cubic: usize, tangent: usize) -> Element {
    Element { id, channel, interval, kind, bits, default, low, high, cubic, tangent }
}

const NONE: usize = 0xffff;

const ELEMENTS: [Element; ELEMENT_COUNT] = [
    el(0, 0, false, 4, 32, -4.7, -100000.0, 100000.0, 87, 7), // EYE_X
    el(1, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 87, 7), // EYE_Y
    el(2, 0, false, 4, 32, 1.6, -100000.0, 100000.0, 87, 7), // EYE_Z
    el(3, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 88, 8), // LOOK_X
    el(4, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 88, 8), // LOOK_Y
    el(5, 0, false, 4, 32, 0.0, -100000.0, 100000.0, 88, 8), // LOOK_Z
    el(6, 0, false, 3, 10, 0.0, -0.25, 0.25, 87, 7), // DUTCH
    el(7, 0, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_EYE
    el(8, 0, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_LOOK
    el(9, 0, false, 3, 9, 24.0, 10.0, 500.0, 87, 7), // LENS_LENGTH
    el(10, 1, false, 3, 9, 24.0, 10.0, 500.0, 100, 12), // LENS_LENGTH_AUX
    el(11, 1, false, 3, 12, 0.5, 0.05, 100.0, 65535, 65535), // NEAR_CLIP
    el(12, 1, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_LENS
    el(13, 1, false, 3, 12, 0.05, 0.05, 100.0, 65535, 65535), // CLIPPING_FAR
    el(14, 1, false, 3, 12, 0.05, 0.05, 100.0, 65535, 65535), // CLIPPING_NEAR
    el(15, 2, false, 3, 16, 0.0, 0.0, 10000.0, 102, 17), // FOCAL_DISTANCE
    el(16, 2, false, 1, 6, 37.0, 0.0, 37.0, 102, 17), // APERTURE
    el(17, 2, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_FOCUS
    el(18, 3, false, 3, 16, 0.0, 0.0, 10000.0, 103, 22), // NEAR_FOCUS
    el(19, 3, false, 3, 16, 0.0, 0.0, 10000.0, 103, 22), // FAR_FOCUS
    el(20, 3, false, 3, 7, 0.0, 0.0, 1.0, 103, 22), // BLUR_FALLOFF
    el(21, 3, false, 3, 7, 0.0, 0.0, 1.0, 103, 22), // BLUR_INTENSITY
    el(22, 3, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_RAWFOCUS
    el(23, 4, false, 3, 7, 0.0, 0.0, 1.0, 65535, 65535), // SHAKE_AMPLITUDE
    el(24, 4, false, 3, 7, 0.0, 0.0, 1.0, 65535, 65535), // SHAKE_FREQUENCY
    el(25, 5, false, 1, 7, 100.0, 0.0, 100.0, 65535, 65535), // TIME_SCALE
    el(26, 8, false, 1, 7, 0.0, 0.0, 100.0, 65535, 65535), // LETTERBOX
    el(27, 9, false, 1, 7, 0.0, 0.0, 100.0, 65535, 65535), // FADE
    el(28, 10, false, 3, 8, 0.0, 0.0, 360.0, 65535, 65535), // LIGHT_RIG_DIR_U
    el(29, 10, false, 3, 8, 0.0, -90.0, 90.0, 65535, 65535), // LIGHT_RIG_DIR_V
    el(30, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_KEY_INTENSITY
    el(31, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_SEC_INTENSITY
    el(32, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_FILL_INTENSITY
    el(33, 10, false, 3, 8, 1.0, 0.0, 2.0, 65535, 65535), // LIGHT_RIG_RIM_INTENSITY
    el(34, 11, false, 3, 16, 0.0, 0.0, 4.0, 65535, 65535), // TONEMAP_VALUE
    el(35, 12, false, 3, 8, 1.0, 0.0, 10.0, 65535, 65535), // MOTION_BLUR_VALUE
    el(36, 13, false, 3, 8, 1.0, 0.0, 1.0, 113, 38), // BLOOM_INTENSITY
    el(37, 13, false, 3, 8, 1.4, -2.0, 2.0, 114, 39), // BLOOM_THRESHOLD
    el(38, 13, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_BLOOM_INTENSITY
    el(39, 13, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_BLOOM_THRESHOLD
    el(40, 14, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // NOISE_INTENSITY
    el(41, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCALEX
    el(42, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCALEY
    el(43, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCROLLRATE
    el(44, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCROLLX
    el(45, 14, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // NOISE_SCROLLY
    el(46, 15, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // FISHEYE_HORIZONTAL
    el(47, 15, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // FISHEYE_MULTIPLIER
    el(48, 15, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // FISHEYE_VERTICAL
    el(49, 16, false, 4, 32, 100.0, 0.0, 10000.0, 65535, 65535), // INTERPOLATION_FAR
    el(50, 16, false, 4, 32, 100.0, 0.0, 10000.0, 65535, 65535), // INTERPOLATION_NEAR
    el(51, 17, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // BRIGHTNESS_FAR
    el(52, 17, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // BRIGHTNESS_NEAR
    el(53, 18, false, 3, 16, 1.0, 0.0, 50.0, 65535, 65535), // CONTRAST_FAR
    el(54, 18, false, 3, 8, 0.5, 0.0, 1.0, 65535, 65535), // CONTRAST_FAR_MIDPOINT
    el(55, 18, false, 3, 16, 1.0, 0.0, 50.0, 65535, 65535), // CONTRAST_NEAR
    el(56, 18, false, 3, 8, 0.5, 0.0, 1.0, 65535, 65535), // CONTRAST_NEAR_MIDPOINT
    el(57, 19, false, 3, 8, 0.0, -1.0, 10.0, 65535, 65535), // SATURATE_FAR
    el(58, 19, false, 3, 8, 0.0, -1.0, 10.0, 65535, 65535), // SATURATE_NEAR
    el(59, 20, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_NEAR_COLOR_X
    el(60, 20, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_NEAR_COLOR_Y
    el(61, 20, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // TINT_NEAR_COLOR_Z
    el(62, 20, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_FAR_COLOR_X
    el(63, 20, false, 3, 8, 0.0, -1.0, 1.0, 65535, 65535), // TINT_FAR_COLOR_Y
    el(64, 20, false, 3, 8, 1.0, 0.0, 1.0, 65535, 65535), // TINT_FAR_COLOR_Z
    el(65, 21, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // CHARACTER_EFFECT_MASK_VALUE
    el(66, 22, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // FXMODE_VALUE
    el(67, 23, false, 3, 8, 0.0, 0.0, 1.0, 65535, 65535), // SCREEN_WIPE_VALUE
    el(68, 24, false, 3, 16, 2.0, 0.1, 5.0, 65535, 65535), // SHADOW_SCALE
    el(69, 24, false, 3, 16, 1.0, -100.0, 100.0, 65535, 65535), // SHADOW_Y_OFFSET
    el(70, 24, false, 3, 16, 1.0, -100.0, 100.0, 65535, 65535), // SHADOW_Z_OFFSET
    el(71, 24, false, 3, 16, 1.0, -100.0, 100.0, 65535, 65535), // SHADOW_X_OFFSET
    el(72, 25, false, 4, 32, 0.01176471, 0.0001, 0.1, 65535, 65535), // SHADOW_BIAS
    el(73, 26, false, 4, 32, 3.0, 0.0, 20.0, 65535, 65535), // SHADOW_PRECISION
    el(74, 27, false, 3, 16, 0.0, 0.0, 100.0, 128, 75), // SCANLINE_Y_OFFSET
    el(75, 27, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_SCANLINE
    el(76, 28, false, 3, 8, 0.0, 0.0, 1.0, 130, 77), // RADIALBLUR
    el(77, 28, false, 3, 6, 1.0, 0.0, 8.0, 65535, 65535), // TANGENT_RADIALBLUR
    el(78, 29, false, 3, 16, 0.0, 0.0, 180.0, 65535, 65535), // SUN_DIRECTION_X
    el(79, 29, false, 3, 16, 0.0, 0.0, 180.0, 65535, 65535), // SUN_DIRECTION_Y
    el(80, 29, false, 3, 16, 90.0, 0.0, 180.0, 65535, 65535), // SUN_DIRECTION_Z
    el(81, 36, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_QUAT_X
    el(82, 36, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_QUAT_Y
    el(83, 36, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_QUAT_Z
    el(84, 36, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_POS_X
    el(85, 36, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_POS_Y
    el(86, 36, false, 3, 16, 0.0, -1.0, 1.0, 65535, 65535), // SHAKE_POS_Z
    el(87, 0, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_EYE
    el(88, 0, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_LOOK
    el(89, 0, true, 1, 3, 0.0, 0.0, 4.0, 65535, 65535), // SPACE_EYE
    el(90, 0, true, 1, 3, 0.0, 0.0, 4.0, 65535, 65535), // SPACE_LOOK
    el(91, 0, true, 1, 5, 0.0, 0.0, 31.0, 65535, 65535), // AVATAR_EYE
    el(92, 0, true, 1, 5, 0.0, 0.0, 31.0, 65535, 65535), // AVATAR_LOOK
    el(93, 0, true, 1, 5, 0.0, 0.0, 31.0, 65535, 65535), // AVATAR_ORBIT
    el(94, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // AVOID_DYNAMIC_OBJECTS
    el(95, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // AVOID_STATIC_OBJECTS
    el(96, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_LOCK_TO_RENDER_OBJECT
    el(97, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_NOROLL
    el(98, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_LAG
    el(99, 0, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // PLAYER_SPACE_BUNGEE
    el(100, 1, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_LENS
    el(101, 1, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // LENS_OVERRIDE
    el(102, 2, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_FOCUS
    el(103, 3, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_RAWFOCUS
    el(104, 3, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // RAWFOCUS_OVERRIDE
    el(105, 4, true, 1, 5, 0.0, 0.0, 14.0, 65535, 65535), // SHAKE_TYPE
    el(106, 6, true, 2, 32, 0.0, 0.0, 268435455.0, 65535, 65535), // EVENT_TAG
    el(107, 7, true, 1, 16, 0.0, 0.0, 65535.0, 65535, 65535), // OVERLAY
    el(108, 9, true, 1, 3, 0.0, 0.0, 5.0, 65535, 65535), // FADE_TO_COLOR
    el(109, 11, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TONEMAP_OVERRIDE
    el(110, 12, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // MOTION_BLUR_OVERRIDE
    el(111, 12, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // MOTION_BLUR_OBJECTAPPLY
    el(112, 13, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // BLOOM_OVERRIDE
    el(113, 13, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_BLOOM_INTENSITY
    el(114, 13, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_BLOOM_THRESHOLD
    el(115, 15, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // FISHEYE_OVERRIDE
    el(116, 16, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // NEAR_FAR_OVERRIDE
    el(117, 17, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // BRIGHTNESS_OVERRIDE
    el(118, 18, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // CONTRAST_OVERRIDE
    el(119, 19, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // SATURATE_OVERRIDE
    el(120, 20, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TINT_NEAR_NEGATIVE
    el(121, 20, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TINT_FAR_NEGATIVE
    el(122, 20, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // TINT_OVERRIDE
    el(123, 21, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // CHARACTER_EFFECT_MASK_OVERRIDE
    el(124, 22, true, 1, 4, 0.0, 0.0, 2.0, 65535, 65535), // FXMODE
    el(125, 24, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // SHADOW_OVERRIDE
    el(126, 26, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // SHADOW_PRECISION_OVERRIDE
    el(127, 27, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // SCANLINE_OVERRIDE
    el(128, 27, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // CUBIC_SCANLINE
    el(129, 28, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // RADIALBLUR_OVERRIDE
    el(130, 28, true, 1, 1, 1.0, 0.0, 1.0, 65535, 65535), // CUBIC_RADIALBLUR
    el(131, 29, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // SUN_DIRECTION_OVERRIDE
    el(132, 30, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_EFFECT_CMD
    el(133, 30, true, 1, 32, 0.0, 0.0, 65535.0, 65535, 65535), // AUDIO_EFFECT_ID
    el(134, 31, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_MUSIC_CMD
    el(135, 31, true, 1, 32, 0.0, 0.0, 65535.0, 65535, 65535), // AUDIO_MUSIC_ID
    el(136, 32, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_SPEECH_CMD
    el(137, 32, true, 1, 32, 5000.0, 5000.0, 65536.0, 65535, 65535), // AUDIO_SPEECH_ID
    el(138, 32, true, 1, 8, 0.0, 0.0, 255.0, 65535, 65535), // AUDIO_SPEECH_CHARACTER
    el(139, 33, true, 1, 2, 0.0, 0.0, 3.0, 65535, 65535), // AUDIO_POST_CMD
    el(140, 33, true, 1, 32, 5000.0, 5000.0, 65536.0, 65535, 65535), // AUDIO_POST_ID
    el(141, 34, true, 1, 4, 0.0, 0.0, 15.0, 65535, 65535), // FREEZE_CMD
    el(142, 34, true, 1, 16, 0.0, 0.0, 300.0, 65535, 65535), // FREEZE_TIME
    el(143, 35, true, 3, 16, 0.0, 0.0, 1.0, 65535, 65535), // TAKE_START
    el(144, 35, true, 1, 6, 0.0, 0.0, 63.0, 65535, 65535), // TAKE_NUMBER
    el(145, 35, true, 1, 1, 0.0, 0.0, 1.0, 65535, 65535), // CONTAINS_SUBTAKE
];

/// `ICE_SMALL_FLOAT`.
fn small() -> f32 {
    f32::from_bits(0x3727_c5ac)
}

/// A packed `ICELITEParameter` as a parameter (`* 0.000015259022`).
fn unpack(packed: u16) -> f32 {
    f32::from(packed) * f32::from_bits(0x3780_0080)
}

/// The parameter clamp `ICETake::SetParameter` and `ICELITEParameter::SetValue`
/// apply: at most 0 is 0, above 1 is 1.
fn clamp_parameter(p: f32) -> f32 {
    if p <= 0.0 {
        0.0
    } else if p > 1.0 {
        1.0
    } else {
        p
    }
}

/// `ICELITEParameter::SetValue` (`0x82a54488`): clamped to [0, 1], times
/// 65535, then truncated and rounded up from a half.
fn pack(p: f32) -> u16 {
    let x = clamp_parameter(p) * 65535.0;
    let mut i = x as i32;
    let mut floor = i as f32;
    if floor > x {
        floor -= 1.0;
    }
    if x - floor >= 0.5 {
        i += 1;
    }
    i as u16
}

/// `ICETake::SetParameter`'s integer store: truncated, then one up when the
/// fraction above the floor reaches a half.
fn round_int(v: f32) -> f64 {
    let mut i = v as i32;
    let mut floor = i as f32;
    if floor > v {
        floor -= 1.0;
    }
    if v - floor >= 0.5 {
        i += 1;
    }
    f64::from(i)
}

fn word(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).ok_or("truncated ICE word")?.try_into().unwrap()))
}
fn half(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(b.get(o..o + 2).ok_or("truncated ICE half")?.try_into().unwrap()))
}
fn chunk(b: &[u8], at: usize) -> Result<(u32, &[u8], usize)> {
    let id = word(b, at)?;
    let end = at.checked_add(8 + word(b, at + 4)? as usize).ok_or("ICE size overflow")?;
    Ok((id, b.get(at + 8..end).ok_or("ICE chunk exceeds parent")?, end))
}

/// `ICELITEChannel`: interval and key counts, the packed starts of intervals
/// 1.. (`pParameters`) and the first keys of the inner intervals
/// (`pKeyIndices`).
#[derive(Clone, Debug, Default)]
struct Channel {
    intervals: usize,
    keys: usize,
    params: Vec<u16>,
    key_indices: Vec<u16>,
}

impl Channel {
    /// `GetIntervalParameter` (`0x82a34250`).
    fn interval_parameter(&self, i: usize) -> f32 {
        if i == 0 {
            0.0
        } else if i < self.intervals {
            unpack(self.params[i - 1])
        } else {
            1.0
        }
    }

    /// `GetIntervalSize`.
    fn interval_size(&self, i: usize) -> f32 {
        self.interval_parameter(i + 1) - self.interval_parameter(i)
    }

    /// An interval's first key: 0 for the first, `pKeyIndices` for the inner
    /// ones, `nKeys - 2` for the last.
    fn key_index(&self, i: usize) -> usize {
        if i == 0 {
            0
        } else if i + 1 < self.intervals {
            usize::from(self.key_indices[i - 1])
        } else {
            self.keys - 2
        }
    }

    /// `ICEChannel::SetParameter` (`0x82a35f18`): the interval whose packed
    /// start is at most the packed parameter and whose end is above it (the
    /// last interval closed), with its packed bounds; none without intervals.
    fn locate(&self, p: f32) -> Option<(usize, f32, f32)> {
        if self.intervals == 0 {
            return None;
        }
        let q = pack(p);
        let i = self.params.partition_point(|&start| start <= q).min(self.intervals - 1);
        let start = if i == 0 { 0 } else { self.params[i - 1] };
        let end = if i + 1 < self.intervals { self.params[i] } else { 0xffff };
        Some((i, unpack(start), unpack(end)))
    }
}

#[derive(Clone, Debug)]
pub struct Take {
    pub length: f32,
    schema: &'static [Element],
    channels: Vec<Channel>,
    /// Each element's decoded values (`ICEElementDesc::GetValue`
    /// `0x82a574e0`), empty when its channel has none.
    data: Vec<Vec<f64>>,
}

#[derive(Debug)]
pub struct CameraSet {
    pub groups: BTreeMap<(u32, u32), Vec<Take>>,
    pub order: Vec<(u32, u32)>,
}

impl CameraSet {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (id, body, end) = chunk(bytes, 0)?;
        if id != 0x8000_0000 || end != bytes.len() {
            return Err("invalid ICE root".into());
        }
        let mut result = Self { groups: BTreeMap::new(), order: Vec::new() };
        let mut c = 0;
        while c < body.len() {
            let (context, data, next) = chunk(body, c)?;
            c = next;
            let mut g = 0;
            while g < data.len() {
                let (_, group, next) = chunk(data, g)?;
                g = next;
                let key = word(group, 0)?;
                let count = word(group, 4)? as usize;
                if count > group.len() / 168 {
                    return Err("invalid ICE take count".into());
                }
                let mut takes = Vec::new();
                let mut at = 8;
                for _ in 0..count {
                    let (take, size) = Take::parse(&group[at..])?;
                    at += size;
                    takes.push(take);
                }
                if at != group.len() {
                    return Err(format!("ICE group {key:08x} trailing bytes"));
                }
                result.order.push((context, key));
                if result.groups.insert((context, key), takes).is_some() {
                    return Err("duplicate ICE group".into());
                }
            }
        }
        Ok(result)
    }

    /// PC port: a later ICE file's groups (downloadable content's `ice_NNN.bin`) that this set
    /// lacks, in their order. The game's own groups stay this set's.
    pub fn add_new_groups(&mut self, other: Self) {
        let Self { mut groups, order } = other;
        for key in order {
            if self.groups.contains_key(&key) {
                continue;
            }
            if let Some(takes) = groups.remove(&key) {
                self.order.push(key);
                self.groups.insert(key, takes);
            }
        }
    }

    fn take(&self, group: (u32, u32), index: usize) -> &Take {
        &self.groups[&group][index]
    }

    /// The group's `TakeList`: its takes without assembly intervals, in order.
    fn take_list(&self, group: (u32, u32)) -> impl Iterator<Item = usize> + '_ {
        self.groups[&group].iter().enumerate().filter(|(_, t)| !t.is_assembly()).map(|(i, _)| i)
    }

    /// `ICEContextManager::GetCameraGroup` (`0x82a59c68`) and
    /// `ICEManager::SetPlaybackCamera` (`0x82a3bd60`): the group's
    /// `AssemblyList[index]`, else its `TakeList[index]`, set to parameter 0
    /// (`SetParameter(0, force)`); none without a take or with a zero length.
    #[must_use]
    pub fn playback(&self, context: u32, hash: u32, index: usize) -> Option<Playback> {
        let group = (context, hash);
        let takes = self.groups.get(&group)?;
        let take = takes
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_assembly())
            .nth(index)
            .or_else(|| takes.iter().enumerate().filter(|(_, t)| !t.is_assembly()).nth(index))?
            .0;
        let mut playback = Playback {
            group,
            take,
            sub: None,
            parameter: 0.0,
            sub_parameter: 0.0,
            intervals: [None; CHANNEL_COUNT],
            values: ELEMENTS.map(|e| f64::from(e.default)),
        };
        playback.set_parameter(self, 0.0, true, false);
        (self.take(group, take).length > 0.0).then_some(playback)
    }

    /// `SceneCamera::GetListOfOverlays` (`0x82864f10`) up to the photo outro's
    /// shot: with an assembly at `AssemblyList[track]` every `TakeList` take
    /// adds its overlays, else `TakeList[track]` alone; none when no take
    /// added one.
    #[must_use]
    pub fn overlays(&self, context: u32, hash: u32, track: usize) -> Option<Vec<u8>> {
        let takes = self.groups.get(&(context, hash))?;
        let mut list = Vec::new();
        let mut added = false;
        let mut plain = takes.iter().filter(|t| !t.is_assembly());
        if takes.iter().filter(|t| t.is_assembly()).nth(track).is_some() {
            for take in plain {
                added |= take.overlays_into(&mut list);
            }
        } else if let Some(take) = plain.nth(track) {
            added = take.overlays_into(&mut list);
        }
        added.then_some(list)
    }

    /// The frontend's world-space camera at `seconds` into its first
    /// (`NIS_FrontEnd`, context 0x80000006) take.
    pub fn world_camera(&self, hash: u32, seconds: f32) -> Result<([f32; 16], f32)> {
        let group = (0x8000_0006, hash);
        let take = self.groups.get(&group).and_then(|g| g.first()).ok_or("ICE frontend take absent")?;
        let mut values = ELEMENTS.map(|e| f64::from(e.default));
        let p = clamp_parameter(seconds / take.length);
        for ch in 0..CHANNEL_COUNT {
            take.evaluate(ch, p, &mut values);
        }
        camera_matrix(self, &values, p * take.length, &Spaces::default(), &mut Mover::default(), p)
    }

    /// The take `world_camera` reads, evaluated the same way at `seconds`, for the ICE user
    /// channels' readers.
    pub fn world_playback(&self, hash: u32, seconds: f32) -> Result<Playback> {
        let group = (0x8000_0006, hash);
        let take = self.groups.get(&group).and_then(|g| g.first()).ok_or("ICE frontend take absent")?;
        let mut playback = Playback {
            group,
            take: 0,
            sub: None,
            parameter: 0.0,
            sub_parameter: 0.0,
            intervals: [None; CHANNEL_COUNT],
            values: ELEMENTS.map(|e| f64::from(e.default)),
        };
        playback.set_parameter(self, seconds / take.length, true, true);
        Ok(playback)
    }
}

impl Take {
    fn parse(bytes: &[u8]) -> Result<(Self, usize)> {
        Self::parse_schema(bytes, &ELEMENTS, CHANNEL_COUNT)
    }

    fn parse_schema(bytes: &[u8], schema: &'static [Element], channel_count: usize) -> Result<(Self, usize)> {
        let length = f32::from_bits(word(bytes, 12)?);
        if !length.is_finite() || length <= 0. {
            return Err("invalid ICE take duration".into());
        }
        let mut counts = vec![(0usize, 0usize); channel_count];
        for (i, c) in counts.iter_mut().enumerate() {
            *c = (usize::from(half(bytes, 20 + 4 * i)?), usize::from(half(bytes, 22 + 4 * i)?));
            if c.0 > 0 && c.1 < 2 {
                return Err("ICE channel has fewer than two keys".into());
            }
        }
        let header_size = 20 + 4 * channel_count;
        let mut key_at = header_size;
        let mut param_at = header_size + 2 * counts.iter().map(|&(n, _)| n.saturating_sub(2)).sum::<usize>();
        let mut channels = Vec::with_capacity(channel_count);
        for &(n, k) in &counts {
            let mut channel = Channel { intervals: n, keys: k, params: Vec::new(), key_indices: Vec::new() };
            for _ in 0..n.saturating_sub(1) {
                channel.params.push(half(bytes, param_at)?);
                param_at += 2;
            }
            for _ in 0..n.saturating_sub(2) {
                channel.key_indices.push(half(bytes, key_at)?);
                key_at += 2;
            }
            if n > 0 {
                let starts = std::iter::once(0).chain(channel.params.iter().copied()).chain(std::iter::once(0xffff));
                if starts.clone().zip(starts.skip(1)).any(|(a, b)| b <= a) || (0..n).any(|i| channel.key_index(i) + 1 >= k) {
                    return Err("invalid ICE channel intervals".into());
                }
            }
            channels.push(channel);
        }
        let mut at = (param_at + 3) & !3;
        let mut data = Vec::with_capacity(schema.len());
        for e in schema {
            let (n, k) = counts[e.channel];
            let count = if e.interval { n } else { k };
            let size = ((e.bits * count + 31) >> 3) & !3;
            let bits = bytes.get(at..at + size).ok_or("truncated ICE element")?;
            at += size;
            data.push((0..count).map(|i| e.decode(bits, i)).collect());
        }
        Ok((Self { length, schema, channels, data }, at))
    }

    /// The take has assembly intervals (`AssemblyList`).
    #[must_use]
    pub fn is_assembly(&self) -> bool {
        self.channels[ASSEMBLY_CHANNEL].intervals != 0
    }

    #[must_use]
    pub fn key_count(&self, channel: usize) -> usize {
        self.channels[channel].keys
    }

    /// `ICETake::GetValue` (`0x82a38c88`): the default when the element's
    /// channel has no values.
    fn value(&self, id: usize, index: usize) -> f64 {
        self.data[id].get(index).copied().unwrap_or(f64::from(self.schema[id].default))
    }

    /// `ICETake::GetValueFloat` (`0x82a38d80`).
    fn value_float(&self, id: usize, index: usize) -> f32 {
        self.value(id, index) as f32
    }

    /// `ICETake::GetValueInt` (`0x82a38e28`): a float element (data types 3
    /// and 4) rounds half up, an unsigned one reads as it is.
    fn value_int(&self, id: usize, index: usize) -> i64 {
        let v = self.value(id, index);
        if matches!(self.schema[id].kind, 3 | 4) { round_int(v as f32) as i64 } else { v as i64 }
    }

    /// `SceneCamera::GetOverlaysFromTake` (`0x82861dd8`): each channel 7
    /// interval's overlay (`(u8)GetValueInt(107, i)`) not yet listed, while
    /// the list holds fewer than 15; whether one was added.
    fn overlays_into(&self, list: &mut Vec<u8>) -> bool {
        let mut added = false;
        for i in 0..self.channels[ELEMENTS[element::OVERLAY].channel].intervals {
            let value = self.value_int(element::OVERLAY, i) as u8;
            if !list.contains(&value) && list.len() < 15 {
                list.push(value);
                added = true;
            }
        }
        added
    }

    pub fn key(&self, id: usize, index: usize) -> Result<f32> {
        let values = self.data.get(id).ok_or_else(|| format!("unknown ICE element {id}"))?;
        if values.is_empty() {
            return Ok(self.schema[id].default);
        }
        values.get(index).map(|&v| v as f32).ok_or_else(|| "ICE key outside track".into())
    }

    /// The element's value `seconds` into the take.
    pub fn sample(&self, id: usize, seconds: f32) -> Result<f32> {
        let e = self.schema.get(id).ok_or_else(|| format!("unknown ICE element {id}"))?;
        let mut values = ELEMENTS.map(|e| f64::from(e.default));
        self.evaluate(e.channel, clamp_parameter(seconds / self.length), &mut values);
        Ok(values[id] as f32)
    }

    /// `ICETake::GetSlope` (`0x82a390e8`): the interval's delta, blended with
    /// the neighbouring interval's when its cubic flag is set and the two share
    /// the key, then scaled by its tangent element.
    fn slope(&self, e: &Element, interval: usize, which: usize, delta: f32) -> f32 {
        if e.cubic == NONE || self.value(e.cubic, interval) == 0.0 {
            return delta;
        }
        let c = &self.channels[e.channel];
        let key = c.key_index(interval) + which;
        let mut slope = delta;
        let neighbour = if which == 1 { interval + 1 } else { interval.wrapping_sub(1) };
        if neighbour < c.intervals {
            let other_key = c.key_index(neighbour);
            if key == (which ^ 1) + other_key {
                let next = self.value_float(e.id, other_key + 1);
                let previous = self.value_float(e.id, other_key);
                let size = c.interval_size(interval);
                let other = c.interval_size(neighbour);
                let mut weight = 1.0;
                let mut other_weight = 0.0;
                if other + size > small() {
                    weight = other / (other + size);
                    other_weight = 1.0 - weight;
                }
                if other > small() {
                    other_weight *= size / other;
                }
                slope = (next - previous).mul_add(other_weight, weight * delta);
            }
        }
        if e.tangent != NONE {
            slope *= self.value_float(e.tangent, key);
        }
        slope
    }

    /// `ICETake::SetParameter(channel, param, force)` (`0x82a39360`): the
    /// channel's elements at `p`. Key elements follow the interval's cubic
    /// (`a = -(2d - (S0 + S1))`, `b = -(2 S0 - (3d - S1))`, fused as the native
    /// does); integer ones are rounded. Returns the interval.
    fn evaluate(&self, channel: usize, p: f32, out: &mut [f64; ELEMENT_COUNT]) -> Option<usize> {
        let c = &self.channels[channel];
        let located = c.locate(p);
        self.evaluate_interval(channel, p, located, out)
    }

    fn evaluate_interval(&self, channel: usize, p: f32, located: Option<(usize, f32, f32)>, out: &mut [f64; ELEMENT_COUNT]) -> Option<usize> {
        let c = &self.channels[channel];
        let (start, end) = located.map_or((0.0, unpack(0xffff)), |(_, s, e)| (s, e));
        let u = (p - start) / (end - start);
        for e in self.schema.iter().filter(|e| e.channel == channel) {
            if e.interval {
                out[e.id] = located.map_or(f64::from(e.default), |(i, _, _)| self.value(e.id, i));
                continue;
            }
            let (p0, p1, s0, s1) = match located {
                Some((i, _, _)) => {
                    let k = c.key_index(i);
                    let p0 = self.value_float(e.id, k);
                    let p1 = self.value_float(e.id, k + 1);
                    (p0, p1, self.slope(e, i, 0, p1 - p0), self.slope(e, i, 1, p1 - p0))
                }
                None => (e.default, e.default, 0.0, 0.0),
            };
            let d = p1 - p0;
            let a = -d.mul_add(2.0, -(s0 + s1));
            let b = -s0.mul_add(2.0, -d.mul_add(3.0, -s1));
            let v = a.mul_add(u, b).mul_add(u, s0).mul_add(u, p0);
            out[e.id] = if e.kind >= 3 { f64::from(v) } else { round_int(v) };
        }
        located.map(|(i, _, _)| i)
    }
}

impl Element {
    /// `ICEElementDesc::GetValue` (`0x82a574e0`): raw floats; unsigned
    /// integers from the bit stream; quantised floats around the default.
    fn decode(self, b: &[u8], i: usize) -> f64 {
        if self.kind == 4 {
            return f64::from(f32::from_bits(u32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap())));
        }
        let mut raw = 0u32;
        for bit in self.bits * i..self.bits * (i + 1) {
            raw = (raw << 1) | u32::from((b[bit / 8] >> (7 - bit % 8)) & 1);
        }
        if self.kind != 3 {
            return f64::from(raw);
        }
        // ICEElementDesc::Init (0x82a357b8): the slots above the default
        // round half up.
        let slots = ((1u64 << self.bits) - 1) as f32;
        let x = (self.high - self.default) * slots / (self.high - self.low);
        let hi = (x - x.floor() >= 0.5).then(|| x.floor() + 1.0).unwrap_or(x.floor());
        let lo = slots - hi;
        let delta = raw as f32 - lo;
        f64::from(if delta < 0. && lo > 0. {
            self.default + (self.default - self.low) * delta / lo
        } else if delta > 0. && hi > 0. {
            self.default + (self.high - self.default) * delta / hi
        } else {
            self.default
        })
    }
}

/// `ICETake` playing a group's take: its own data, the sub-take an assembly
/// interval switched to (`pSubTakeData`, a `TakeList` take), `fParameter`,
/// `fSubParameter`, each channel's current interval and the element values
/// (`Values`).
#[derive(Clone, Debug)]
pub struct Playback {
    group: (u32, u32),
    take: usize,
    sub: Option<usize>,
    parameter: f32,
    sub_parameter: f32,
    intervals: [Option<usize>; CHANNEL_COUNT],
    values: [f64; ELEMENT_COUNT],
}

impl Playback {
    /// `pTakeData->Length`.
    #[must_use]
    pub fn length(&self, set: &CameraSet) -> f32 {
        set.take(self.group, self.take).length
    }

    /// `pSubTakeData->Length`.
    #[must_use]
    pub fn sub_length(&self, set: &CameraSet) -> Option<f32> {
        self.sub.map(|s| set.take(self.group, s).length)
    }

    #[must_use]
    pub fn parameter(&self) -> f32 {
        self.parameter
    }

    #[must_use]
    pub fn sub_parameter(&self) -> f32 {
        self.sub_parameter
    }

    /// `Channels[35].nCurrentInterval` (none = 0xFFFF).
    #[must_use]
    pub fn assembly_interval(&self) -> Option<usize> {
        self.intervals[ASSEMBLY_CHANNEL]
    }

    /// `ICETake::GetIntervalStart(35, nCurrentInterval)` (`0x82a35d18`).
    #[must_use]
    pub fn assembly_interval_start(&self, set: &CameraSet) -> f32 {
        let c = &set.take(self.group, self.take).channels[ASSEMBLY_CHANNEL];
        self.intervals[ASSEMBLY_CHANNEL].map_or(1.0, |i| c.interval_parameter(i))
    }

    /// `GetValueFloat` of the current value.
    #[must_use]
    pub fn float(&self, id: usize) -> f32 {
        self.values[id] as f32
    }

    /// `GetValueInt` of the current value.
    #[must_use]
    pub fn int(&self, id: usize) -> i64 {
        self.values[id] as i64
    }

    /// `Channels[c].nCurrentInterval` of the element's channel `c`; none
    /// for 0xFFFF.
    #[must_use]
    pub fn current_interval(&self, id: usize) -> Option<usize> {
        self.intervals[ELEMENTS[id].channel]
    }

    /// `ICETake::GetValueInt(id, interval)` from the data the element's
    /// channel plays (the sub-take's once one is set, but for the assembly
    /// channel).
    #[must_use]
    pub fn value_int_at(&self, set: &CameraSet, id: usize, interval: usize) -> i64 {
        let channel = ELEMENTS[id].channel;
        let source = if channel != ASSEMBLY_CHANNEL { self.sub.unwrap_or(self.take) } else { self.take };
        set.take(self.group, source).value_int(id, interval)
    }

    /// `SceneCamera::GetTimeAtPhotoShot` (`0x82864d30`) on a playback at
    /// parameter 0: the take length times the start of the `index`-th
    /// freeze interval whose command is `PhotoShot` (4), 0 without one.
    #[must_use]
    pub fn time_at_photo_shot(&self, set: &CameraSet, index: usize) -> f32 {
        self.photo_shot_parameter(set, index).map_or(0.0, |parameter| self.length(set) * parameter)
    }

    /// The start of the `index`-th freeze interval whose command is `PhotoShot` (4); none when
    /// the index is past the channel's intervals or names no such interval.
    #[must_use]
    pub fn photo_shot_parameter(&self, set: &CameraSet, index: usize) -> Option<f32> {
        let channel = ELEMENTS[element::FREEZE_CMD].channel;
        let source = set.take(self.group, self.sub.unwrap_or(self.take));
        let intervals = source.channels[channel].intervals;
        if index >= intervals {
            return None;
        }
        (0..intervals)
            .filter(|&i| source.value_int(element::FREEZE_CMD, i) as u8 == 4)
            .nth(index)
            .map(|i| source.channels[channel].interval_parameter(i))
    }

    /// `NISManager::GetPhotoCount` (`0x828a0e70`): the freeze intervals whose command is
    /// `PhotoShot` (4).
    #[must_use]
    pub fn photo_count(&self, set: &CameraSet) -> i32 {
        self.freeze_command_count(set, 4)
    }

    /// `SceneCamera::GetFixedFramePhotoCount` (`0x82864b98`): the freeze intervals whose
    /// command is `PhotoShot_FixedFrame` (5).
    #[must_use]
    pub fn fixed_frame_photo_count(&self, set: &CameraSet) -> i32 {
        self.freeze_command_count(set, 5)
    }

    fn freeze_command_count(&self, set: &CameraSet, command: u8) -> i32 {
        let channel = ELEMENTS[element::FREEZE_CMD].channel;
        let source = set.take(self.group, self.sub.unwrap_or(self.take));
        let count = (0..source.channels[channel].intervals).filter(|&i| source.value_int(element::FREEZE_CMD, i) as u8 == command).count();
        i32::try_from(count).unwrap_or(i32::MAX)
    }

    /// `ICEUserChannelOverlays::UpdateFrameBegin`'s read (`0x826d1adc..
    /// 0x826d1af0`): `GetValueInt(107, Channels[7].nCurrentInterval)`; none
    /// without a current interval.
    #[must_use]
    pub fn overlay(&self, set: &CameraSet) -> Option<i64> {
        let interval = self.current_interval(element::OVERLAY)?;
        Some(self.value_int_at(set, element::OVERLAY, interval))
    }

    /// One channel's `SetParameter`: the take's data for the assembly
    /// channel, the sub-take's for the others once one is set. Whether the
    /// interval changed.
    fn set_channel(&mut self, set: &CameraSet, channel: usize, p: f32) -> bool {
        let source = if channel != ASSEMBLY_CHANNEL { self.sub.unwrap_or(self.take) } else { self.take };
        let interval = set.take(self.group, source).evaluate(channel, p, &mut self.values);
        let changed = interval != self.intervals[channel];
        self.intervals[channel] = interval;
        changed
    }

    /// `ICETake::SetSubTake(n, false)` (`0x82a39810`): the group's `TakeList[n]`
    /// when it differs from the current sub-take.
    fn set_sub_take(&mut self, set: &CameraSet, n: usize) {
        if let Some(sub) = set.take_list(self.group).nth(n)
            && self.sub != Some(sub)
        {
            self.sub = Some(sub);
        }
    }

    /// `ICETake::SetParameter(param, force, ignore_assembly)` (`0x82a3b080`).
    /// Returns whether the assembly interval changed.
    pub fn set_parameter(&mut self, set: &CameraSet, param: f32, force: bool, ignore_assembly: bool) -> bool {
        let p = clamp_parameter(param);
        if !force && self.parameter == p {
            return false;
        }
        self.parameter = p;
        let take = set.take(self.group, self.take);
        if take.channels[ASSEMBLY_CHANNEL].intervals == 0 || ignore_assembly {
            for channel in 0..CHANNEL_COUNT {
                self.set_channel(set, channel, p);
            }
            return false;
        }
        let changed = self.set_channel(set, ASSEMBLY_CHANNEL, p);
        if self.values[element::CONTAINS_SUBTAKE] == 0.0 {
            self.values[element::SHAKE_AMPLITUDE] = 0.0;
            self.values[element::SHAKE_FREQUENCY] = 0.0;
            return changed;
        }
        self.set_sub_take(set, self.values[element::TAKE_NUMBER] as usize);
        let mut sub = p;
        if let Some(length) = self.sub_length(set) {
            let start = self.assembly_interval_start(set);
            sub = (p - start).mul_add(take.length / length, self.values[element::TAKE_START] as f32);
        }
        self.sub_parameter = clamp_parameter(sub);
        for channel in (0..CHANNEL_COUNT).filter(|&c| c != ASSEMBLY_CHANNEL) {
            self.set_channel(set, channel, self.sub_parameter);
        }
        changed
    }

    /// `ICECameraMover::UpdateFrameEnd`'s camera for the current values:
    /// `UpdateForwardVector` over `seconds` (`mSimTime * dT`), then the
    /// matrix. `length` is the take length `ICESceneBridge::ConnectScene`
    /// writes over a replay-controlled scene's take, which the shake clock
    /// reads.
    pub fn camera(&self, set: &CameraSet, spaces: &Spaces, mover: &mut Mover, seconds: f32, length: Option<f32>) -> Result<([f32; 16], f32)> {
        // Without a local skater GetOrientation gives the identity.
        mover.update_forward(spaces.anchor_orientation.map_or([1.0, 0.0, 0.0], |o| o[0]), seconds);
        let time = if self.values[element::CONTAINS_SUBTAKE] != 0.0 {
            self.sub_parameter * self.sub_length(set).unwrap_or(0.0)
        } else {
            self.parameter * length.unwrap_or_else(|| self.length(set))
        };
        camera_matrix(set, &self.values, time, spaces, mover, self.parameter)
    }

    /// `ICEUserChannelBloom::UpdateFrameEnd` (`0x826c88e0`): under
    /// `BLOOM_OVERRIDE` the take's bloom threshold and intensity.
    #[must_use]
    pub fn bloom(&self) -> Option<[f32; 2]> {
        (self.values[element::BLOOM_OVERRIDE] as i64 != 0)
            .then(|| [self.values[element::BLOOM_THRESHOLD] as f32, self.values[element::BLOOM_INTENSITY] as f32])
    }

    /// `ICEUserChannelFade::UpdateFrameEnd` (`0x826c80f8`): `SetFadeColor`
    /// with the `FADE_TO_COLOR` colour (black, white, red, green, blue) and
    /// `FADE` percent clamped to [0, 1]; no call for another colour.
    #[must_use]
    pub fn fade(&self) -> Option<[f32; 4]> {
        let amount = clamp(self.values[element::FADE] as f32 * 0.01, 0.0, 1.0);
        let [r, g, b] = match self.values[element::FADE_TO_COLOR] as i64 {
            0 => [0.0, 0.0, 0.0],
            1 => [1.0, 1.0, 1.0],
            2 => [1.0, 0.0, 0.0],
            3 => [0.0, 1.0, 0.0],
            4 => [0.0, 0.0, 1.0],
            _ => return None,
        };
        Some([r, g, b, amount])
    }

    /// `ICEUserChannelTonemap::UpdateFrameEnd` (`0x826c85a0`): `TONEMAP_VALUE`
    /// under `TONEMAP_OVERRIDE`.
    #[must_use]
    pub fn tonemap(&self) -> Option<f32> {
        (self.int(element::TONEMAP_OVERRIDE) != 0).then(|| self.float(element::TONEMAP_VALUE))
    }

    /// `ICEUserChannelFishEye::UpdateFrameEnd` (`0x826c8e70`): under
    /// `FISHEYE_OVERRIDE`, `fisheyeWarpMultiplier` is `FISHEYE_MULTIPLIER`
    /// times 4.7619047. The handler's other fisheye fields reach no reader.
    #[must_use]
    pub fn fisheye(&self) -> Option<f32> {
        (self.int(element::FISHEYE_OVERRIDE) != 0)
            .then(|| self.float(element::FISHEYE_MULTIPLIER) * f32::from_bits(0x4098_6186))
    }

    /// `ICEUserChannelNearFar::UpdateFrameEnd` (`0x826c9108`): under
    /// `NEAR_FAR_OVERRIDE`, `nisFarStart` from `INTERPOLATION_NEAR` and
    /// `nisFarEnd` from `INTERPOLATION_FAR`.
    #[must_use]
    pub fn near_far(&self) -> Option<[f32; 2]> {
        (self.int(element::NEAR_FAR_OVERRIDE) != 0)
            .then(|| [self.float(element::INTERPOLATION_NEAR), self.float(element::INTERPOLATION_FAR)])
    }

    /// `ICEUserChannelBrightness::UpdateFrameEnd` (`0x826c8cc0`): the near
    /// and far brightness under `BRIGHTNESS_OVERRIDE`.
    #[must_use]
    pub fn brightness(&self) -> Option<[f32; 2]> {
        (self.int(element::BRIGHTNESS_OVERRIDE) != 0)
            .then(|| [self.float(element::BRIGHTNESS_NEAR), self.float(element::BRIGHTNESS_FAR)])
    }

    /// `ICEUserChannelContrast::UpdateFrameEnd` (`0x826c92c8`): the near
    /// and far contrast and midpoint under `CONTRAST_OVERRIDE`.
    #[must_use]
    pub fn contrast(&self) -> Option<[[f32; 2]; 2]> {
        (self.int(element::CONTRAST_OVERRIDE) != 0).then(|| {
            [
                [self.float(element::CONTRAST_NEAR), self.float(element::CONTRAST_NEAR_MIDPOINT)],
                [self.float(element::CONTRAST_FAR), self.float(element::CONTRAST_FAR_MIDPOINT)],
            ]
        })
    }

    /// `ICEUserChannelSaturate::UpdateFrameEnd` (`0x826c94f0`): the near
    /// and far saturation under `SATURATE_OVERRIDE`.
    #[must_use]
    pub fn saturate(&self) -> Option<[f32; 2]> {
        (self.int(element::SATURATE_OVERRIDE) != 0)
            .then(|| [self.float(element::SATURATE_NEAR), self.float(element::SATURATE_FAR)])
    }

    /// `ICEUserChannelTint::UpdateFrameEnd` (`0x826c96a0`): under
    /// `TINT_OVERRIDE`, each side's `saturateColour` from its X, Y and Z
    /// through `ICE::XYZ2HSV` and `ICE::HSV2RGB`. Its negative flags reach no
    /// reader.
    #[must_use]
    pub fn tint(&self) -> Option<[[f32; 3]; 2]> {
        (self.int(element::TINT_OVERRIDE) != 0).then(|| {
            [element::TINT_NEAR_COLOR_X, element::TINT_FAR_COLOR_X].map(|x| {
                let [h, s, v] = math::xyz_to_hsv(self.float(x), self.float(x + 1), self.float(x + 2));
                math::hsv_to_rgb(h, s, v)
            })
        })
    }

    /// `ICEUserChannelScanline::UpdateFrameEnd` (`0x826ca388`): under
    /// `SCANLINE_OVERRIDE`, `SCANLINE_Y_OFFSET` less its truncation.
    #[must_use]
    pub fn scanline(&self) -> Option<f32> {
        (self.int(element::SCANLINE_OVERRIDE) != 0).then(|| {
            let offset = self.float(element::SCANLINE_Y_OFFSET);
            offset - (offset as i32) as f32
        })
    }

    /// `ICEUserChannelRadialBlur::UpdateFrameEnd` (`0x826ca4a0`): `RADIALBLUR`
    /// under `RADIALBLUR_OVERRIDE`.
    #[must_use]
    pub fn radial_blur(&self) -> Option<f32> {
        (self.int(element::RADIALBLUR_OVERRIDE) != 0).then(|| self.float(element::RADIALBLUR))
    }

    /// `ICEUserChannelLens::UpdateFrameEnd`'s near plane (`mICENearPlane`):
    /// `CLIPPING_NEAR` clamped to its range, kept only when not negative.
    #[must_use]
    pub fn near_plane(&self) -> Option<f32> {
        let range = &ELEMENTS[element::CLIPPING_NEAR];
        let near = clamp(self.values[element::CLIPPING_NEAR] as f32, range.low, range.high);
        (near >= 0.0).then_some(near)
    }

    /// `SceneCamera::GetCameraComponentsAtTimeZero` (`0x828647f0`) for a
    /// playback that has not advanced (`ICETake::SetData` at parameter 0):
    /// the eye and look positions through `TransformToWorld` with their
    /// spaces, avatar 0 and no player-space options, in Skate space.
    pub fn eye_and_look(&self, spaces: &Spaces) -> Result<([f32; 3], [f32; 3])> {
        let v = |id: usize| self.values[id] as f32;
        let space = |id: usize| self.values[id] as i64;
        // Without LAG or BUNGEE the transform reads no mover state.
        let mover = &mut Mover::default();
        let none = PlayerSpace::default();
        let eye = to_world([v(0), v(1), v(2)], space(element::SPACE_EYE), 0, spaces, mover, none)?;
        let look = to_world([v(3), v(4), v(5)], space(element::SPACE_LOOK), 0, spaces, mover, none)?;
        let skate = |a: [f32; 3]| [a[1], a[2], a[0]];
        Ok((skate(eye), skate(look)))
    }
}

/// The avatars `GetAvatarPosition` answers: the AVATAR elements' range
/// (0..=31) less the anchor's 0.
pub const AVATARS: usize = 31;

/// What the camera's coordinate spaces resolve against, in ICE space (Z up).
#[derive(Clone, Copy, Debug, Default)]
pub struct Spaces {
    /// `ICECameraAnchor::GetPosition` (the local skater).
    pub anchor: Option<[f32; 3]>,
    /// `User_ICECameraAnchor::GetOrientation` (`0x826cd228`): the local
    /// skater's root axes through `MakeICEFromSkate`, rows 0 to 2.
    pub anchor_orientation: Option<[[f32; 3]; 3]>,
    /// `User_ICESceneManager::GetAvatarPosition` (`0x826d1608`) by avatar
    /// index, the AVATAR element less one.
    pub avatars: [Option<[f32; 3]>; AVATARS],
    /// `User_ICESceneManager::GetAvatarOrientation` (`0x826cf5f0`) by avatar
    /// index, rows 0 to 2.
    pub avatar_orientations: [Option<[[f32; 3]; 3]>; AVATARS],
    /// The active ICE scene (`User_ICESceneManager::GetActiveScene`
    /// `0x826c7490`) as `ICELITE::Copy` joins its orientation and position.
    pub scene: Option<[[f32; 4]; 4]>,
}

/// The VMX normalisation `Scene::ComputeTransform` and
/// `SceneCamera::GetCameraComponentsAtTimeZero` share: the three-term dot
/// product, `vrsqrtefp` and two fused Newton steps. The estimate is taken as
/// an exact reciprocal square root and the dot product as f32 operations.
#[must_use]
pub fn vmx_normalize(a: [f32; 3]) -> [f32; 3] {
    let d = a[0] * a[0] + a[1] * a[1] + a[2] * a[2];
    let mut y = 1.0 / d.sqrt();
    for _ in 0..2 {
        let e = (-d).mul_add(y * y, 1.0);
        y = (y * 0.5).mul_add(e, y);
    }
    [a[0] * y, a[1] * y, a[2] * y]
}

/// The scene code's VMX cross product: each lane's first product rounded
/// (`vmulfp128`), the second fused and the result negated (`vnmsubfp`), so
/// an exact cancellation is -0.
#[must_use]
pub fn vmx_cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [-a[2].mul_add(b[1], -(a[1] * b[2])), -a[0].mul_add(b[2], -(a[2] * b[0])), -a[1].mul_add(b[0], -(a[0] * b[1]))]
}

/// `Scene::ComputeTransform` (`0x828652f0`), whose vector code
/// `ICESceneBridge::ICEMatrixFromSkatePointAndNormal` (`0x826cc6a8`) repeats:
/// rows X = normalize(normal x Z), Y = Z x X, Z = normalize(0 - orientation)
/// (`vsubfp128` from zero) and the location with w 1 (Skate space).
#[must_use]
pub fn compute_transform(location: [f32; 3], orientation: [f32; 3], normal: [f32; 3]) -> [[f32; 4]; 4] {
    let z = vmx_normalize([0.0 - orientation[0], 0.0 - orientation[1], 0.0 - orientation[2]]);
    let x = vmx_normalize(vmx_cross(normal, z));
    let y = vmx_cross(z, x);
    [[x[0], x[1], x[2], 0.0], [y[0], y[1], y[2], 0.0], [z[0], z[1], z[2], 0.0], [location[0], location[1], location[2], 1.0]]
}

/// `ICESceneBridge::GetOrientation` (`0x826cf460`) and `GetPosition`
/// (`0x826cf538`) for a scene's location, orientation and normal (Skate
/// space): `ICEMatrixFromSkatePointAndNormal` gives the `ComputeTransform`
/// rows Z, X and Y in ICE axes (z, x, y); `ICELITE::Copy` (`0x82a33210`)
/// adds the location in the same axes.
#[must_use]
pub fn scene_space(location: [f32; 3], orientation: [f32; 3], normal: [f32; 3]) -> [[f32; 4]; 4] {
    let m = compute_transform(location, orientation, normal);
    let ice = |r: [f32; 4], w: f32| [r[2], r[0], r[1], w];
    [ice(m[2], 0.0), ice(m[0], 0.0), ice(m[1], 0.0), ice(m[3], 1.0)]
}

/// `CreateWorldCameraMatrix`'s normalisation: a fused length, then the
/// reciprocal.
fn normalize(a: [f32; 3]) -> Option<[f32; 3]> {
    let l = a[0].mul_add(a[0], a[2].mul_add(a[2], a[1] * a[1])).sqrt();
    (l != 0.0).then(|| {
        let r = 1.0 / l;
        [a[0] * r, a[1] * r, a[2] * r]
    })
}

/// `ICE::Cubic1D` (0x2c bytes): a value easing to its desired value and
/// slope along a cubic over `mDuration` seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cubic1D {
    pub val: f32,
    pub dval: f32,
    pub val_desired: f32,
    pub dval_desired: f32,
    coeff: [f32; 4],
    time: f32,
    duration: f32,
    /// 0 at rest, 1 easing, 2 to restart from the current value and slope.
    pub state: i16,
    flags: u16,
}

impl Default for Cubic1D {
    /// The camera mover constructor's `Cubic3D` (`0x82a3d6b8..0x82a3d748`):
    /// zeros and a one-second duration.
    fn default() -> Self {
        Self { val: 0.0, dval: 0.0, val_desired: 0.0, dval_desired: 0.0, coeff: [0.0; 4], time: 0.0, duration: 1.0, state: 0, flags: 0 }
    }
}

impl Cubic1D {
    /// `ICE::Cubic1D::Update` (`0x82a340c0`).
    pub fn update(&mut self, seconds: f32) {
        match self.state {
            1 => {}
            2 => {
                self.time = 0.0;
                if self.flags == 0 {
                    self.state = 1;
                }
                let delta = self.val_desired - self.val;
                let slopes = self.dval + self.dval_desired;
                self.coeff[2] = self.dval;
                self.coeff[3] = self.val;
                let c = delta.mul_add(3.0, -self.dval_desired);
                self.coeff[0] = -delta.mul_add(2.0, -slopes);
                self.coeff[1] = -self.dval.mul_add(2.0, -c);
            }
            _ => return,
        }
        self.time = if self.duration <= f32::from_bits(0x3a83_126f) { 1.0 } else { seconds / self.duration + self.time };
        let t = self.time;
        if t < 1.0 {
            let [c0, c1, c2, c3] = self.coeff;
            self.val = c0.mul_add(t, c1).mul_add(t, c2).mul_add(t, c3);
            self.dval = (c0 * t).mul_add(3.0, c1 * 2.0).mul_add(t, c2);
        } else {
            self.time = 1.0;
            self.state = 0;
            self.val = self.val_desired;
            self.dval = self.dval_desired;
        }
    }
}

/// The `ICECameraMover` state that outlasts a take: `Sk8::ICE::Init`
/// (`0x8286a6d8`) builds the one mover when `SceneCamera::Init` runs in the
/// `Presentation` constructor, and `UpdateFrameEnd` (`0x82a3b2e0`) carries it
/// from scene to scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mover {
    /// `mForward`, the anchor's eased forward.
    pub forward: [Cubic1D; 3],
    /// `mBungeePlayerPos`.
    pub bungee_position: [f32; 3],
    /// `mBungeePlayerFwd`.
    pub bungee_forward: [f32; 3],
}

impl Default for Mover {
    /// The constructor (`0x82a3d508`) takes the bungee from the anchor
    /// (`0x82a3d768..0x82a3d7b4`); no skater is local yet in the
    /// `Presentation` constructor (`GetLocalIndex` -1), so `GetPosition`
    /// gives the origin and `GetOrientation` the identity.
    fn default() -> Self {
        Self { forward: [Cubic1D::default(); 3], bungee_position: [0.0; 3], bungee_forward: [1.0, 0.0, 0.0] }
    }
}

impl Mover {
    /// `UpdateForwardVector` (`0x82a36fb0`) toward the anchor orientation's
    /// first row over `seconds`. The anchor's velocity
    /// (`User_ICECameraAnchor::GetVelocity` `0x826c7648`) is always zero:
    /// nothing reflects, the speed blend `(speed - 8) / 13` clamps to 0 and
    /// the velocity direction falls back to (1, 0, 0).
    pub fn update_forward(&mut self, anchor_forward: [f32; 3], seconds: f32) {
        let blend = 0.0f32;
        let direction = [1.0f32, 0.0, 0.0];
        for ((cubic, row), dir) in self.forward.iter_mut().zip(anchor_forward).zip(direction) {
            cubic.val_desired = row.mul_add(1.0 - blend, blend * dir);
            if cubic.val_desired != cubic.val {
                cubic.state = 2;
            }
        }
        for cubic in &mut self.forward {
            cubic.update(seconds);
        }
    }
}

/// `TransformToWorld`'s player-space arguments: `PLAYER_SPACE_NOROLL`,
/// `_LAG` and `_BUNGEE` (elements 97 to 99) and the take's `fParameter`.
#[derive(Clone, Copy, Debug, Default)]
struct PlayerSpace {
    noroll: bool,
    lag: bool,
    bungee: bool,
    parameter: f32,
}

/// The camera mover's cross products are fmsubs: the first product exact,
/// the second rounded.
fn fmsubs_cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1].mul_add(b[2], -(a[2] * b[1])), a[2].mul_add(b[0], -(a[0] * b[2])), a[0].mul_add(b[1], -(a[1] * b[0]))]
}

/// `Sk8::Scene::MakeICEFromSkate` (`0x826d71c8`) for a matrix's X, Y and Z
/// axes: ICE rows 0 to 2 from the Skate Z, X and Y axes, each as (z, x, y).
#[must_use]
pub fn make_ice_from_skate(axes: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let ice = |a: [f32; 3]| [a[2], a[0], a[1]];
    [ice(axes[2]), ice(axes[0]), ice(axes[1])]
}

/// `User_ICESceneManager::GetAvatarOrientation`'s frame
/// (`0x826cf97c..0x826cfab8`) for an avatar matrix (Skate rows X, Y, Z, W):
/// (0, 0, 1, 0) through the rows (`vmulfp128`, then `vmaddfp`), flattened as
/// `up x (forward x up)` with up (0, 1, 0), made a frame at the origin by
/// `Math::Matrix44FromLookAtUpEyePos` (`0x829b0938`: Z = normalize(look),
/// X = normalize(up x Z), Y = Z x X), then `MakeICEFromSkate`.
#[must_use]
pub fn avatar_orientation(m: [[f32; 3]; 4]) -> [[f32; 3]; 3] {
    let forward: [f32; 3] = std::array::from_fn(|c| m[3][c].mul_add(0.0, m[2][c].mul_add(1.0, m[1][c].mul_add(0.0, m[0][c] * 0.0))));
    let up = [0.0, 1.0, 0.0];
    let z = vmx_normalize(vmx_cross(up, vmx_cross(forward, up)));
    let x = vmx_normalize(vmx_cross(up, z));
    make_ice_from_skate([x, vmx_cross(z, x), z])
}

/// `TransformToWorld` in player space (0, `0x82a376b0..0x82a37a74`): the
/// avatar's orientation and position (`ICELITE::Copy` `0x82a33210`) above the
/// AVATAR minimum (0), else the anchor's, or under LAG the normalised
/// `mForward` against the anchor orientation's third row at the anchor.
/// NOROLL flattens the first row and rebuilds the others about Z; BUNGEE
/// eases `mBungeePlayerPos` and `mBungeePlayerFwd` toward the frame by
/// `fParameter * 0.9 + 0.1` and rebuilds about the third row there.
fn player_to_world(v: [f32; 3], avatar: i64, spaces: &Spaces, mover: &mut Mover, player: PlayerSpace) -> Result<[f32; 3]> {
    let (mut rows, mut position) = if avatar > 0 {
        let index = usize::try_from(avatar - 1).ok();
        let orientation = index
            .and_then(|i| spaces.avatar_orientations.get(i).copied().flatten())
            .ok_or_else(|| format!("ICE avatar {avatar} orientation not resolved"))?;
        let position = index
            .and_then(|i| spaces.avatars.get(i).copied().flatten())
            .ok_or_else(|| format!("ICE avatar {avatar} not resolved"))?;
        (orientation, position)
    } else {
        let position = spaces.anchor.ok_or("ICE player space needs the anchor")?;
        let orientation = spaces.anchor_orientation.ok_or("ICE player space needs the anchor orientation")?;
        if player.lag {
            let forward = normalize(mover.forward.map(|c| c.val)).unwrap_or([1.0, 0.0, 0.0]);
            let side = fmsubs_cross(orientation[2], forward);
            ([forward, side, fmsubs_cross(forward, side)], position)
        } else {
            (orientation, position)
        }
    };
    if player.noroll {
        let forward = normalize([rows[0][0], rows[0][1], 0.0]).unwrap_or([1.0, 0.0, 0.0]);
        let side = fmsubs_cross([0.0, 0.0, 1.0], forward);
        rows = [forward, side, fmsubs_cross(forward, side)];
    }
    if player.bungee {
        let k = player.parameter.mul_add(f32::from_bits(0x3f66_6666), f32::from_bits(0x3dcc_cccd));
        for c in 0..3 {
            mover.bungee_position[c] = (position[c] - mover.bungee_position[c]) * k + mover.bungee_position[c];
            mover.bungee_forward[c] = (rows[0][c] - mover.bungee_forward[c]) * k + mover.bungee_forward[c];
        }
        let forward = normalize(mover.bungee_forward).unwrap_or([1.0, 0.0, 0.0]);
        mover.bungee_forward = forward;
        let side = fmsubs_cross(rows[2], forward);
        rows = [forward, side, fmsubs_cross(forward, side)];
        position = mover.bungee_position;
    }
    let [x, y, z] = v;
    let [r0, r1, r2] = rows;
    Ok([
        y.mul_add(r1[0], x.mul_add(r0[0], z * r2[0])) + position[0],
        z.mul_add(r2[1], x.mul_add(r0[1], y * r1[1])) + position[1],
        z.mul_add(r2[2], x.mul_add(r0[2], y * r1[2])) + position[2],
    ])
}

/// `ICECameraMover::TransformToWorld` (`0x82a37448`) for the player (0),
/// world (1), anchor-offset (2) and scene (3) spaces. A scene-space point
/// needs no `GetAnimElevationFixup` (`0x82a35348`): the bridge's
/// `IsCameraFixingElevation` is false.
fn to_world(v: [f32; 3], space: i64, avatar: i64, spaces: &Spaces, mover: &mut Mover, player: PlayerSpace) -> Result<[f32; 3]> {
    match space {
        0 => player_to_world(v, avatar, spaces, mover, player),
        1 => Ok(v),
        // An avatar up to the AVATAR element's minimum (0) is the camera
        // anchor, any other the scene manager's avatar one below it
        // (0x82a375d8..0x82a37664).
        2 => {
            let a = if avatar <= 0 {
                spaces.anchor.ok_or("ICE anchor space needs the anchor")?
            } else {
                usize::try_from(avatar - 1)
                    .ok()
                    .and_then(|i| spaces.avatars.get(i).copied().flatten())
                    .ok_or_else(|| format!("ICE avatar {avatar} not resolved"))?
            };
            Ok([a[0] + v[0], a[1] + v[1], a[2] + v[2]])
        }
        3 => {
            // Without an active scene the matrix stays the identity.
            let m = spaces.scene.unwrap_or([[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]]);
            let at = |c: usize| v[0].mul_add(m[0][c], v[2].mul_add(m[2][c], v[1] * m[1][c])) + m[3][c];
            Ok([at(0), at(1), at(2)])
        }
        _ => Err(format!("ICE space {space} (avatar {avatar}) not resolved")),
    }
}

/// `UpdateTransformationMatrix` (`0x82a3a410`), `CreateWorldCameraMatrix`
/// (`0x82a371b8`), `UpdateLens` (`0x82a36be0`) and the recorded
/// `UpdateShake` (`0x82a39db0`), then `ConvertMatrix4ToSkate` (`0x826d2f68`):
/// the camera matrix (Skate rows right, up, forward, position) and the
/// vertical field of view in degrees that `ICEUserChannelLens::UpdateFrameEnd`
/// (`0x826cd7e8`) leaves.
fn camera_matrix(
    set: &CameraSet,
    values: &[f64; ELEMENT_COUNT],
    seconds: f32,
    spaces: &Spaces,
    mover: &mut Mover,
    parameter: f32,
) -> Result<([f32; 16], f32)> {
    let v = |id: usize| values[id] as f32;
    let i = |id: usize| values[id] as i64;
    // mLockToRenderObject (a player-space eye with element 96) has the
    // avatar lookups follow the render object and CreateWorldCameraMatrix
    // take the anchor's up; no shipped take sets it.
    if i(element::SPACE_EYE) == 0 && i(element::PLAYER_SPACE_LOCK_TO_RENDER_OBJECT) != 0 {
        return Err("ICE PLAYER_SPACE_LOCK_TO_RENDER_OBJECT not ported".into());
    }
    let player = PlayerSpace {
        noroll: i(element::PLAYER_SPACE_NOROLL) != 0,
        lag: i(element::PLAYER_SPACE_LAG) != 0,
        bungee: i(element::PLAYER_SPACE_BUNGEE) != 0,
        parameter,
    };
    // PLAYER_SPACE_LAG has UpdateAccelOffset (0x82a34e28) move both points by
    // the smoothed anchor acceleration, scaled and clamped. That acceleration
    // (User_ICECameraAnchor::GetAcceleration 0x826c75f8) is always zero and
    // the smoothing starts at rest at zero (the ICECameraMover constructor
    // 0x82a3d508), so the offset stays zero.
    let eye = to_world([v(0), v(1), v(2)], i(element::SPACE_EYE), i(element::AVATAR_EYE), spaces, mover, player)?;
    let look = to_world([v(3), v(4), v(5)], i(element::SPACE_LOOK), i(element::AVATAR_LOOK), spaces, mover, player)?;
    let cross = fmsubs_cross;
    let forward = normalize([look[0] - eye[0], look[1] - eye[1], look[2] - eye[2]]).unwrap_or([1.0, 0.0, 0.0]);
    // UpdateTransformationMatrix asks for the scene's up with a scene-space
    // eye; the bridge always gives it (IsCameraUsingSceneUp 0x826c6d80): the
    // third row of its orientation. Otherwise Z.
    let world_up = match spaces.scene {
        Some(scene) if i(element::SPACE_EYE) == 3 => [scene[2][0], scene[2][1], scene[2][2]],
        _ => [0., 0., 1.],
    };
    let side = normalize(cross(world_up, forward)).unwrap_or([1.0, 0.0, 0.0]);
    let up = cross(forward, side);
    let row = |a: [f32; 3], w| [a[0], a[1], a[2], w];
    let mut m = [row(forward, 0.), row(side, 0.), row(up, 0.), row([0.0; 3], 0.)];
    // ICE::RotateX (0x82a33be0) by the dutch angle, from ICE::SinCos.
    let dutch = (v(element::DUTCH) * 65536.) as i32 as u16;
    let (s, c) = math::sin_cos(dutch);
    m = math::mul(m, [[1., 0., 0., 0.], [0., c, s, 0.], [0., -s, c, 0.], [0., 0., 0., 1.]]);
    m[3] = row(eye, 1.);
    let lens = &ELEMENTS[element::LENS_LENGTH];
    let fov = lens_fov(v(element::LENS_LENGTH).max(lens.low).min(lens.high));
    // UpdateShake reads the field of view back from the camera
    // (User_ICECamera::GetFieldOfView 0x826cd028) after UpdateLens stored it
    // in degrees.
    let camera_fov = (fov_degrees(fov) * 182.044_45) as i32 as u16;
    let amplitude = v(element::SHAKE_AMPLITUDE);
    let kind = i(element::SHAKE_TYPE);
    if kind != 0 {
        // The camera mover's recorded shake takes, one per type, in file
        // order of the shake context.
        let key = set
            .order
            .iter()
            .filter(|k| k.0 == SHAKE_CONTEXT)
            .nth(kind as usize)
            .ok_or("ICE shake type absent")?;
        let shake = &set.groups[key][0];
        let count = shake.key_count(36);
        if count == 0 {
            return Err("empty recorded ICE shake".into());
        }
        let time = seconds * v(element::SHAKE_FREQUENCY) * 30.;
        let index = time as i32;
        let a = index.rem_euclid(count as i32) as usize;
        let b = (index + 1).rem_euclid(count as i32) as usize;
        let u = time - index as f32;
        let scale = math::sin(camera_fov >> 1) * amplitude / math::sin(0x1555);
        let quat = |k| -> Result<[f32; 4]> {
            let x = shake.key(element::SHAKE_QUAT_Z, k)? * scale;
            let y = shake.key(element::SHAKE_QUAT_X, k)? * scale;
            let z = -shake.key(element::SHAKE_QUAT_Y, k)? * scale;
            let w = (1. - z.mul_add(z, x.mul_add(x, y * y)).sqrt()).sqrt();
            Ok([x, y, z, w])
        };
        let qa = quat(a)?;
        let qb = quat(b)?;
        let [x, y, z, w] = std::array::from_fn(|i| (qb[i] - qa[i]).mul_add(u, qa[i]));
        // The position reads use the first key for both ends (0x82a3a138..1a8).
        let p = [
            -shake.key(element::SHAKE_POS_Z, a)? * scale,
            -shake.key(element::SHAKE_POS_X, a)? * scale,
            shake.key(element::SHAKE_POS_Y, a)? * scale,
            1.,
        ];
        let q = [
            [1. - y * (2. * y) - z * (2. * z), w * (2. * z) + x * (2. * y), x * (2. * z) - w * (2. * y), 0.],
            [x * (2. * y) - w * (2. * z), 1. - x * (2. * x) - z * (2. * z), w * (2. * x) + y * (2. * z), 0.],
            [w * (2. * y) + x * (2. * z), y * (2. * z) - w * (2. * x), 1. - x * (2. * x) - y * (2. * y), 0.],
            p,
        ];
        m = math::mul(m, q);
    } else {
        // The procedural shake (0x82a3a1f8..0x82a3a3f8): four noise samples
        // roll, yaw and offset the camera, scaled to the field of view.
        let t = seconds * v(element::SHAKE_FREQUENCY);
        let n1 = math::noise(t * f32::from_bits(0x3f1e_353f));
        let n2 = math::noise(t * f32::from_bits(0x3ec3_9581));
        let n3 = math::noise(t * f32::from_bits(0x3fcf_1aa0));
        let n4 = math::noise(t * f32::from_bits(0x3fb0_e560));
        let offset_y = n1 * 0.3 * amplitude;
        let offset_z = n2 * 0.2 * amplitude;
        let roll = n3 * 6. * amplitude;
        let yaw = n4 * 4. * amplitude;
        let reference = math::sin(0x1555);
        let half_fov = math::sin(camera_fov >> 1);
        let turn = |degrees: f32| math::asin(half_fov * math::sin((degrees * 182.044_45) as i32 as u16) / reference);
        let identity = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];
        let mut q = math::rotate_z(identity, turn(roll));
        q = math::rotate_y(q, turn(yaw));
        q[3][1] = half_fov * offset_y / reference;
        q[3][2] = half_fov * offset_z / reference;
        m = math::mul(m, q);
    }
    let mut result = [0.; 16];
    for (out, source) in [1, 2, 0, 3].into_iter().enumerate() {
        let a = m[source];
        result[out * 4..out * 4 + 4].copy_from_slice(&[a[1], a[2], a[0], a[3]]);
    }
    if result.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite ICE camera".into());
    }
    // ICEUserChannelLens::UpdateFrameEnd, one of the channels
    // Sk8::ICE::RegisterChannels (0x8286a1d0) registers, runs after the mover
    // (ICEChannelManager::UpdateFrameEnd) and sets the field of view again:
    // the auxiliary lens under LENS_OVERRIDE, clamped to the lens range.
    let lens_length = if i(element::LENS_OVERRIDE) != 0 { v(element::LENS_LENGTH_AUX) } else { v(element::LENS_LENGTH) };
    Ok((result, fov_degrees(lens_fov(clamp(lens_length, lens.low, lens.high)))))
}

/// `ICELITE::Clamp` (`0x82a53950`).
fn clamp(a: f32, min: f32, max: f32) -> f32 {
    let v = if a <= min { min } else { a };
    if max >= v { v } else { max }
}

/// `ICE::ConvertLensLengthToFovAngle` (`0x82a33df0`): the lens length's
/// vertical field of view as a binary angle.
fn lens_fov(lens: f32) -> u16 {
    math::atan(lens, 15.960001).wrapping_mul(2) & 0xfffe
}

/// `User_ICECamera::SetFieldOfView` (`0x826d82c8`): `mICEFOV` in degrees.
fn fov_degrees(fov: u16) -> f32 {
    f32::from(fov) * 0.0054931641
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_table_is_in_id_order() {
        assert!(ELEMENTS.iter().enumerate().all(|(i, e)| e.id == i));
        assert!(ELEMENTS.iter().all(|e| e.cubic == NONE || ELEMENTS[e.cubic].channel == e.channel));
    }

    #[test]
    fn the_anchor_space_offsets_the_anchor_or_the_avatar_below_it() {
        let mut spaces = Spaces { anchor: Some([1.0, 2.0, 3.0]), ..Spaces::default() };
        spaces.avatars[5] = Some([10.0, 20.0, 30.0]);
        let world = |v, space, avatar| to_world(v, space, avatar, &spaces, &mut Mover::default(), PlayerSpace::default());
        assert_eq!(world([0.5, 0.25, 0.0], 2, 0), Ok([1.5, 2.25, 3.0]));
        assert_eq!(world([0.5, 0.25, 0.0], 2, 6), Ok([10.5, 20.25, 30.0]));
        assert!(world([0.0; 3], 2, 7).is_err());
        // World space takes no avatar.
        assert_eq!(world([0.5, 0.25, 0.0], 1, 6), Ok([0.5, 0.25, 0.0]));
    }

    #[test]
    fn parameters_pack_half_up() {
        assert_eq!(pack(0.0), 0);
        assert_eq!(pack(1.0), 0xffff);
        assert_eq!(pack(2.0), 0xffff);
        assert_eq!(pack(0.5 / 65535.0), 1);
        assert_eq!(pack(unpack(12345)), 12345);
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn tutorial_assembly_take_cuts_between_sub_takes() {
        use crate::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\big\\scene.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        let set = CameraSet::parse(&mounts.read_path("data\\scene\\ice_000.bin").unwrap()).unwrap();
        assert_eq!(set.groups.values().map(Vec::len).sum::<usize>(), 598);
        // tutorial_m_1a_goto_slappys (NIS_Tutorial, context index 14): track 0
        // is its assembly take, 12.434 s over sub-takes 1, 0 and 2.
        let mut playback = set.playback(0x8000_0003 + 14, 0x7725_b313, 0).unwrap();
        assert!((playback.length(&set) - 12.434_376).abs() < 1e-4);
        assert_eq!(playback.int(element::TAKE_NUMBER), 1);
        assert_eq!(playback.sub_length(&set), Some(15.0));
        assert!((playback.sub_parameter() - 0.0604).abs() < 1e-3);
        // Each sub-take has its own time scale: 85, 90 and 93 percent.
        assert_eq!(playback.int(element::TIME_SCALE), 85);
        // The take's second interval starts at 3.749 s: sub-take 0.
        let p = 3.8 / playback.length(&set);
        assert!(playback.set_parameter(&set, p, false, false));
        assert_eq!(playback.int(element::TAKE_NUMBER), 0);
        assert_eq!(playback.int(element::TIME_SCALE), 90);
        assert!(playback.camera(&set, &Spaces::default(), &mut Mover::default(), 1.0 / 60.0, None).is_ok());
        // Its last interval has no sub-take.
        assert!(playback.set_parameter(&set, 1.0, false, false));
        assert_eq!(playback.int(element::CONTAINS_SUBTAKE), 0);
    }

    /// `ICEMatrixFromSkatePointAndNormal` run instruction by instruction (an
    /// emulator of its VMX128 code, `vrsqrtefp` as an exact reciprocal square
    /// root) for two scenes; `GetPosition` supplies the fourth row.
    /// `Scene::ComputeTransform` run instruction by instruction: zero lanes
    /// keep the native signs (Z from 0 - orientation, the crosses negated).
    #[test]
    fn the_scene_transform_keeps_the_native_signed_zeros() {
        let cases = [
            ([10.0, 20.0, 30.0], [0.0, 0.0, -1.0], [[1.0, -0.0, -0.0], [-0.0, 1.0, -0.0], [0.0, 0.0, 1.0]]),
            ([1.0, 2.0, 3.0], [0.6, 0.0, 0.8], [[-0.8, -0.0, 0.6], [-0.0, 1.000_000_1, 0.0], [-0.6, 0.0, -0.8]]),
        ];
        for (location, orientation, rows) in cases {
            let m = compute_transform(location, orientation, [0.0, 1.0, 0.0]);
            for (row, want) in m.iter().zip(rows) {
                for (got, want) in row.iter().zip(want) {
                    assert!((got - want).abs() <= 2e-7 && got.is_sign_negative() == want.is_sign_negative(), "{m:?}");
                }
                assert_eq!(row[3].to_bits(), 0);
            }
            assert_eq!(m[3], [location[0], location[1], location[2], 1.0]);
        }
    }

    #[test]
    fn the_scene_space_matches_the_native_matrix() {
        let cases = [
            ([1.5, -2.0, 7.0], [0.3, -0.2, 0.9], [0.1, 0.95, 0.2], [
                [-0.928_279_16, -0.309_426_4, 0.206_284_27],
                [0.322_404_06, -0.946_070_9, 0.031_711_873],
                [0.185_347_05, 0.095_944_35, 0.977_978_3],
            ]),
            ([-3200.5, 41.25, 880.0], [0.6, 0.0, -0.8], [0.0, 1.0, 0.0], [
                [0.8, -0.600_000_02, 0.0],
                [0.600_000_02, 0.8, 0.0],
                [0.0, 0.0, 1.000_000_1],
            ]),
        ];
        for (location, orientation, normal, rows) in cases {
            let m = scene_space(location, orientation, normal);
            for (row, want) in m.iter().zip(rows) {
                for (got, want) in row.iter().zip(want) {
                    assert!((got - want).abs() <= 2e-7, "{m:?}");
                }
                assert_eq!(row[3], 0.0);
            }
            assert_eq!(m[3], [location[2], location[0], location[1], 1.0]);
        }
    }

    #[test]
    fn sin_cos_shares_the_sine_and_follows_the_cosine() {
        for angle in 0..=u16::MAX {
            let (s, c) = math::sin_cos(angle);
            assert_eq!(s.to_bits(), math::sin(angle).to_bits(), "{angle}");
            let radians = f64::from(angle) * std::f64::consts::TAU / 65536.0;
            // The sine series stops at x^9: up to 3.6e-6 off near a quarter turn.
            assert!((f64::from(c) - radians.cos()).abs() < 2e-6, "{angle}");
            assert!((f64::from(s) - radians.sin()).abs() < 4e-6, "{angle}");
        }
    }

    /// Values from a separate model of `ICE::ASin` and `ICE::Noise` built from
    /// the same disassembly.
    #[test]
    fn asin_and_noise_match_their_model() {
        let asin = [(0.0, 0x0000), (0.1, 0x0414), (0.49, 0x14dd), (0.5, 0x1555), (0.73, 0x2157), (0.9, 0x2d9f), (0.999, 0x3e2c), (-0.25, 0xf5b5)];
        for (x, angle) in asin {
            assert_eq!(math::asin(x), angle, "{x}");
        }
        assert_eq!((math::asin(1.0), math::asin(-1.5)), (0x4000, 0xc000));
        let noise = [(0.0, 0xbf0e_05cd), (0.3, 0xbed8_26ab), (1.7, 0x3e06_91ce), (12.25, 0xbf89_29d7), (-3.6, 0x3ec3_d30f), (100.75, 0xbf01_9e5c)];
        for (x, bits) in noise {
            assert_eq!(math::noise(x).to_bits(), bits, "{x}");
        }
    }

    #[test]
    fn mul_multiplies_row_vectors() {
        let identity = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];
        let a = [[0.5, 1.5, -2.0, 0.0], [3.0, 0.25, 1.0, 0.0], [-1.0, 2.0, 0.75, 0.0], [4.0, -5.0, 6.0, 1.0]];
        let b = [[1.0, 0.0, 0.0, 0.0], [0.0, 0.6, 0.8, 0.0], [0.0, -0.8, 0.6, 0.0], [0.0, 0.0, 0.0, 1.0]];
        assert_eq!(math::mul(identity, a), a);
        assert_eq!(math::mul(a, identity), a);
        let m = math::mul(a, b);
        for r in 0..4 {
            for c in 0..4 {
                let want: f32 = (0..4).map(|k| b[r][k] * a[k][c]).sum();
                assert!((m[r][c] - want).abs() < 1e-5);
            }
        }
    }

    fn playback_with(set: impl FnOnce(&mut [f64; ELEMENT_COUNT])) -> Playback {
        let mut values = ELEMENTS.map(|e| f64::from(e.default));
        set(&mut values);
        Playback { group: (0, 0), take: 0, sub: None, parameter: 0.0, sub_parameter: 0.0, intervals: [None; CHANNEL_COUNT], values }
    }

    #[test]
    fn the_lens_bloom_and_fade_channels_read_their_elements() {
        // ICEUserChannelLens: CLIPPING_NEAR clamped to [0.05, 100].
        assert_eq!(playback_with(|v| v[element::CLIPPING_NEAR] = 0.01).near_plane(), Some(0.05));
        assert_eq!(playback_with(|v| v[element::CLIPPING_NEAR] = 0.5).near_plane(), Some(0.5));
        // ICEUserChannelBloom: the take's values only under BLOOM_OVERRIDE.
        assert_eq!(playback_with(|_| ()).bloom(), None);
        let overridden = playback_with(|v| {
            v[element::BLOOM_OVERRIDE] = 1.0;
            v[element::BLOOM_THRESHOLD] = 0.5;
            v[element::BLOOM_INTENSITY] = 0.25;
        });
        assert_eq!(overridden.bloom(), Some([0.5, 0.25]));
        // ICEUserChannelFade: colours 0..=4, the FADE percent clamped to 1.
        let red = playback_with(|v| {
            v[element::FADE_TO_COLOR] = 2.0;
            v[element::FADE] = 50.0;
        });
        assert_eq!(red.fade(), Some([1.0, 0.0, 0.0, 0.5]));
        let white = playback_with(|v| {
            v[element::FADE_TO_COLOR] = 1.0;
            v[element::FADE] = 100.0;
        });
        assert_eq!(white.fade(), Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(playback_with(|v| v[element::FADE_TO_COLOR] = 5.0).fade(), None);
    }

    /// The main menu leaves `g_FinalMult` at 1: no frontend take fades.
    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn frontend_takes_never_fade() {
        use crate::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\big\\scene.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        let set = CameraSet::parse(&mounts.read_path("data\\scene\\ice_000.bin").unwrap()).unwrap();
        let mut samples = 0;
        for (&(context, hash), takes) in &set.groups {
            if context != 0x8000_0006 {
                continue;
            }
            for index in 0..takes.len() {
                let Some(mut playback) = set.playback(context, hash, index) else { continue };
                for step in 0..=64 {
                    playback.set_parameter(&set, step as f32 / 64.0, true, false);
                    assert!(playback.fade().is_none_or(|fade| fade[3] == 0.0));
                    samples += 1;
                }
            }
        }
        assert!(samples > 0);
    }

    #[test]
    fn a_cubic_eases_along_its_hermite_and_restarts_from_its_slope() {
        let mut cubic = Cubic1D { val_desired: 1.0, state: 2, ..Cubic1D::default() };
        cubic.update(0.5);
        // From rest to rest: 3t^2 - 2t^3 and its slope 6t - 6t^2.
        assert_eq!((cubic.val, cubic.dval, cubic.state), (0.5, 1.5, 1));
        cubic.update(0.5);
        assert_eq!((cubic.val, cubic.dval, cubic.state), (1.0, 0.0, 0));
        cubic.update(0.5);
        assert_eq!(cubic.val, 1.0);
        // A restart picks the curve up at the current value and slope.
        let mut cubic = Cubic1D { val_desired: 1.0, state: 2, ..Cubic1D::default() };
        cubic.update(0.25);
        let (val, dval) = (cubic.val, cubic.dval);
        cubic.val_desired = 0.0;
        cubic.state = 2;
        cubic.update(0.0);
        assert_eq!((cubic.val, cubic.dval), (val, dval));
    }

    #[test]
    fn the_mover_restarts_its_forward_ease_every_frame_it_is_away() {
        let mut mover = Mover::default();
        assert_eq!((mover.bungee_position, mover.bungee_forward), ([0.0; 3], [1.0, 0.0, 0.0]));
        mover.update_forward([0.0, 1.0, 0.0], 0.5);
        assert_eq!(mover.forward.map(|c| c.val), [0.0, 0.5, 0.0]);
        assert_eq!(mover.forward.map(|c| c.state), [0, 1, 0]);
        // A -0 target arrives as +0, equal to the resting value. The moving
        // lane starts over from 0.5 with slope 1.5.
        mover.update_forward([-0.0, 1.0, 0.0], 0.5);
        assert_eq!(mover.forward.map(|c| c.val), [0.0, 0.9375, 0.0]);
        assert_eq!(mover.forward.map(|c| c.state), [0, 1, 0]);
    }

    #[test]
    fn player_space_follows_the_anchor_lag_noroll_and_bungee_frames() {
        // The anchor turned a quarter left: ICE forward +y.
        let spaces = Spaces {
            anchor: Some([1.0, 2.0, 3.0]),
            anchor_orientation: Some([[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]),
            ..Spaces::default()
        };
        let none = PlayerSpace::default();
        let mut mover = Mover::default();
        assert_eq!(to_world([2.0, 1.0, 0.5], 0, 0, &spaces, &mut mover, none), Ok([0.0, 4.0, 3.5]));
        // LAG: the mover's forward, still zero, normalises to (1, 0, 0) and
        // stands up against the anchor's third row.
        let lag = PlayerSpace { lag: true, ..none };
        assert_eq!(to_world([2.0, 1.0, 0.5], 0, 0, &spaces, &mut mover, lag), Ok([3.0, 3.0, 3.5]));
        // NOROLL keeps a pitched frame's heading only.
        let pitched = Spaces { anchor_orientation: Some([[0.5, 0.0, 0.75], [0.0, 1.0, 0.0], [-0.75, 0.0, 0.5]]), ..spaces };
        let noroll = PlayerSpace { noroll: true, ..none };
        assert_eq!(to_world([2.0, 1.0, 0.5], 0, 0, &pitched, &mut mover, noroll), Ok([3.0, 3.0, 3.5]));
        // BUNGEE at fParameter 0 eases a tenth of the way from the mover's
        // origin toward the anchor, every call, and places the point there.
        let bungee = PlayerSpace { bungee: true, ..none };
        let k = f32::from_bits(0x3dcc_cccd);
        let eased = |from: [f32; 3], to: [f32; 3]| -> [f32; 3] { std::array::from_fn(|c| (to[c] - from[c]) * k + from[c]) };
        let first = eased([0.0; 3], [1.0, 2.0, 3.0]);
        assert_eq!(to_world([0.0; 3], 0, 0, &spaces, &mut mover, bungee), Ok(first));
        assert_eq!(mover.bungee_position, first);
        let heading = eased([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert_eq!(mover.bungee_forward, normalize(heading).unwrap());
        let second = eased(first, [1.0, 2.0, 3.0]);
        assert_eq!(to_world([0.0; 3], 0, 0, &spaces, &mut mover, bungee), Ok(second));
        // Without the anchor, or above the avatar minimum without an
        // orientation, player space stays unresolved.
        assert!(to_world([0.0; 3], 0, 0, &Spaces::default(), &mut mover, none).is_err());
        let mut avatars = spaces;
        avatars.avatars[0] = spaces.anchor;
        assert!(to_world([0.0; 3], 0, 1, &avatars, &mut mover, none).is_err());
        avatars.avatar_orientations[0] = spaces.anchor_orientation;
        assert_eq!(to_world([2.0, 1.0, 0.5], 0, 1, &avatars, &mut mover, none), Ok([0.0, 4.0, 3.5]));
    }

    #[test]
    fn an_avatar_orientation_is_the_heading_stood_upright() {
        // Skate axes of a skater facing +x, pitched nose up: ICE forward +y,
        // up +z.
        let skate = [[0.0, 0.0, -1.0], [-0.6, 0.8, 0.0], [0.8, 0.6, 0.0], [5.0, 6.0, 7.0]];
        let rows = avatar_orientation(skate);
        let expected = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        for (row, want) in rows.iter().zip(expected) {
            for (a, b) in row.iter().zip(want) {
                assert!((a - b).abs() < 1e-6, "{rows:?}");
            }
        }
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(make_ice_from_skate(identity), identity);
    }

    /// Every shipped camera sample in player space resolves once the anchor,
    /// its orientation and avatar 0 are known.
    #[test]
    fn a_tint_turns_its_xy_into_a_hue_and_its_length_into_a_saturation() {
        // ICE::XYZ2HSV then ICE::HSV2RGB on the shipped near tint (0.078125,
        // 0.5, 1): ATan(0.5, 0.078125) is table entry 40, 1617 of 65536
        // turns, sextant 0.
        let [h, s, v] = math::xyz_to_hsv(0.078_125, 0.5, 1.0);
        assert_eq!((h * 65536.0, v), (1617.0, 1.0));
        let [r, g, b] = math::hsv_to_rgb(h, s, v);
        assert_eq!(r, 1.0);
        assert!((g - 0.5687).abs() < 1e-3 && (b - 0.4939).abs() < 1e-3, "{g} {b}");
        // The saturation is held to 1; a hue past a half turn lands later.
        let [h, s, _] = math::xyz_to_hsv(-0.9, -0.9, 1.0);
        assert_eq!(s, 1.0);
        assert!(h > 0.5 && h < 0.75, "{h}");
        assert_eq!(math::hsv_to_rgb(0.0, 0.0, 0.5), [0.5; 3]);
    }

    #[test]
    #[ignore = "requires SKATE2_GAME_ROOT"]
    fn the_post_fx_overrides_turn_on_in_the_shipped_takes() {
        use crate::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\big\\scene.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        let set = CameraSet::parse(&mounts.read_path("data\\scene\\ice_000.bin").unwrap()).unwrap();
        // Takes whose own channels turn each override on somewhere: tonemap,
        // fisheye, near/far, brightness, contrast, saturate, tint, scanline,
        // radial blur.
        let mut on = [0; 9];
        for (&group, takes) in &set.groups {
            for take in 0..takes.len() {
                let mut playback = Playback {
                    group,
                    take,
                    sub: None,
                    parameter: 0.0,
                    sub_parameter: 0.0,
                    intervals: [None; CHANNEL_COUNT],
                    values: ELEMENTS.map(|e| f64::from(e.default)),
                };
                let mut seen = [false; 9];
                for step in 0..=2048 {
                    playback.set_parameter(&set, step as f32 / 2048.0, true, true);
                    let channels = [
                        playback.tonemap().is_some(),
                        playback.fisheye().is_some_and(|w| (0.0..=4.762).contains(&w)),
                        playback.near_far().is_some(),
                        playback.brightness().is_some(),
                        playback.contrast().is_some(),
                        playback.saturate().is_some(),
                        playback.tint().is_some_and(|t| t.iter().flatten().all(|c| (0.0..=1.0).contains(c))),
                        playback.scanline().is_some_and(|s| (0.0..1.0).contains(&s)),
                        playback.radial_blur().is_some(),
                    ];
                    for (seen, channel) in seen.iter_mut().zip(channels) {
                        *seen |= channel;
                    }
                }
                for (on, seen) in on.iter_mut().zip(seen) {
                    *on += usize::from(seen);
                }
            }
        }
        assert_eq!(on, [44, 422, 0, 4, 115, 113, 116, 77, 77]);
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn every_player_space_sample_resolves_against_the_anchor() {
        use crate::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\big\\scene.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        let set = CameraSet::parse(&mounts.read_path("data\\scene\\ice_000.bin").unwrap()).unwrap();
        let axes = [[0.0, 0.0, -1.0], [-0.6, 0.8, 0.0], [0.8, 0.6, 0.0]];
        let mut spaces = Spaces {
            anchor: Some([20.0, 10.0, 1.0]),
            anchor_orientation: Some(make_ice_from_skate(axes)),
            scene: Some([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]),
            ..Spaces::default()
        };
        spaces.avatars[0] = spaces.anchor;
        spaces.avatar_orientations[0] = Some(avatar_orientation([axes[0], axes[1], axes[2], [0.0; 3]]));
        let mut samples = 0;
        for (&(context, hash), takes) in &set.groups {
            for index in 0..takes.len() {
                let Some(mut playback) = set.playback(context, hash, index) else { continue };
                let mut mover = Mover::default();
                for step in 0..=64 {
                    playback.set_parameter(&set, step as f32 / 64.0, true, false);
                    if playback.int(element::SPACE_EYE) != 0 && playback.int(element::SPACE_LOOK) != 0 {
                        continue;
                    }
                    let camera = playback.camera(&set, &spaces, &mut mover, 1.0 / 30.0, None);
                    assert!(camera.is_ok(), "{context:#x} {hash:#x} take {index} step {step}: {camera:?}");
                    samples += 1;
                }
            }
        }
        assert!(samples > 0);
    }
}
