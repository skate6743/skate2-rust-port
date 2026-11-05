//! `Sk8::VisualDirector`'s challenge indicator records. Each simulation submit
//! (`VDManager::DoSimSubmit` 0x82800120, `VDWriter::WriteData` 0x827ff8c8)
//! `VDWriter::_PrepareChallengeVisualIndicators` 0x827ff138 gathers the
//! indicators `cVisualIndicatorManager` shows around the local skater's
//! `Actor::GetPosition`, and the presentation restores them with
//! `VDReader::ReadData` 0x827fdd78 for `VIPresentation`.
//!
//! Location ribbons cross the replay buffer as `VDWriter_LocationRibbon`: the
//! translation and `RawDiscretize::QuatCompress` of the orientation, and the
//! indices of the ribbon and colour collections (`VDUtils::BuildIndexMap`
//! over every collection of each class). The reader restores the matrix,
//! adds the ribbon's `Offset` and reads its `ribbon_spline` and
//! `RenderSettings` keys.
//!
//! Race gates retain AddRaceGate827fda90's full bounding-volume transform and
//! half lengths through replay. Obstacles, Create-a-Spot volumes and challenge
//! boundaries (AddBoxInfo827fef18) are not yet gathered.

use super::{attrib_db::AttribDb, system::World};
use crate::ai::vmx::{self, V4};
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric};

/// `kMaxDistanceToLocationDataType` (cvar `0x82f6ddf8`: default 500, range 0 to 20000).
pub const MAX_DISTANCE_TO_LOCATION_DATA_TYPE: f32 = 500.0;
/// `fixed_vector<VDWriter_LocationRibbon, 22>`.
pub const MAX_LOCATION_RIBBONS: usize = 22;
/// `fixed_vector<VD_SplineInfo, 16>`.
pub const MAX_SPLINES: usize = 16;
/// `fixed_vector<VD_FollowTargetInfo, 4>`.
pub const MAX_FOLLOW_TARGETS: usize = 4;

/// `StringHash64` of the classes.
pub mod class {
    /// `Attrib::Gen::ribbon_indicator`.
    pub const RIBBON_INDICATOR: u64 = 0x3297_556b_ecb6_05e2;
    /// `Attrib::Gen::ribbon_spline`.
    pub const RIBBON_SPLINE: u64 = 0xfb5f_dab5_dc2b_d228;
    /// `Attrib::Gen::ribbon_spline_colour`.
    pub const RIBBON_SPLINE_COLOUR: u64 = 0xf0db_8767_d611_af94;
    /// `Attrib::Gen::visualindicators`.
    pub const VISUAL_INDICATORS: u64 = 0x2f57_f64d_d2b4_f257;
}

mod field {
    /// `ribbon_indicator.Offset` (`Vector4`).
    pub const OFFSET: u64 = 0x4374_c0e6_f4cb_45ed;
    /// `ribbon_indicator.RenderSettings`: a `visualindicators` RefSpec.
    pub const RENDER_SETTINGS: u64 = 0x66fc_5f54_e2d8_b317;
    /// The `ribbon_spline_colour` text `VIPresentation::GetBackTextureAsset` 0x82800868 hashes.
    pub const BACK_TEXTURE: u64 = 0xff4d_ca21_7e05_b9bc;
    /// `visualindicators` layout +0x00: twelve floats.
    pub const PARAMS: u64 = 0x5157_3d0d_a2f2_11e1;
    pub const JITTER_NEAR: u64 = 0x9e86_cda4_9941_6e79;
    pub const JITTER_FAR: u64 = 0x7ad4_1c91_6657_963a;
    pub const JITTER_PERIOD: u64 = 0xccf3_23a6_570b_a084;
    pub const WIDTH_NEAR: u64 = 0x284e_9294_277d_21dc;
    pub const WIDTH_FAR: u64 = 0xa631_3118_df98_9363;
    pub const INTENSITY: u64 = 0x031d_e0f2_619e_2697;
    pub const BLEND_NEAR: u64 = 0xe23c_15fd_4e31_7c52;
    pub const BLEND_FAR: u64 = 0x16b2_5c44_2494_93ad;
    pub const DARKEN_INTENSITY: u64 = 0x1643_bb6a_dcdc_41ad;
    pub const SAMPLES: u64 = 0x4ffe_9c7f_a608_319c;
    pub const LAYERS: u64 = 0x445e_0bfc_57ec_196b;
}

/// `VIPresentation::mDefaultSettings`: the `visualindicators` `default` collection.
pub const DEFAULT_SETTINGS: u64 = 0xd7ed_bd36_2d7d_2152;
/// `key_trail` (0x8211ce28), the follow trail's `visualindicators` collection.
pub const TRAIL_SETTINGS: u64 = 0xa391_a729_f737_46b9;
/// The `ribbon_spline_colour` `blue` collection.
const BLUE: u64 = 0x5460_3ffc_31d8_dbf3;

/// `Attrib::Gen::visualindicators` (layout 0x58; `DefaultDataArea` zeros without a collection).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VisualIndicatorSettings {
    /// Layout +0x00 (attribute 0x51573d0da2f211e1). `SplineIndicator::Update` 0x828013f8 reads
    /// [4]..[8] as the near fade start and end, the far fade start and the visible distance.
    pub params: [f32; 12],
    /// +0x30, +0x34: the jitter amplitude at and beyond the blend distances.
    pub jitter: [f32; 2],
    /// +0x38: the seconds between jitter rebuilds.
    pub jitter_period: f32,
    /// +0x3c, +0x40: the ribbon width at and beyond the blend distances.
    pub width: [f32; 2],
    /// +0x44: `mIntensity.x`, the default technique's.
    pub intensity: f32,
    /// +0x48, +0x4c: the camera distances the near and far values blend over.
    pub blend_distance: [f32; 2],
    /// +0x50: `mIntensity.y`, the darken technique's.
    pub darken_intensity: f32,
    /// +0x54: vertices per segment and edge; `Spline::Update` 0x82800f60 caps it at 10.
    pub samples: i16,
    /// +0x56: the jittered copies of the control points.
    pub layers: i16,
}

fn float(db: &AttribDb, collection: u64, key: u64) -> f32 {
    db.attribute(class::VISUAL_INDICATORS, collection, key)
        .and_then(|a| a.f32(0))
        .unwrap_or(0.0)
}

/// `Attrib::Gen::visualindicators::visualindicators` 0x828007f0 of a collection.
#[must_use]
pub fn visual_indicator_settings(db: &AttribDb, collection: u64) -> VisualIndicatorSettings {
    if !db.has_collection(class::VISUAL_INDICATORS, collection) {
        return VisualIndicatorSettings::default();
    }
    let mut params = [0.0; 12];
    if let Some(bytes) = db
        .attribute(class::VISUAL_INDICATORS, collection, field::PARAMS)
        .and_then(|a| a.value_bytes(0))
    {
        for (value, word) in params.iter_mut().zip(bytes.chunks_exact(4)) {
            *value = f32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
    }
    let short = |key| {
        db.attribute(class::VISUAL_INDICATORS, collection, key)
            .and_then(|a| a.i16(0))
            .unwrap_or(0)
    };
    VisualIndicatorSettings {
        params,
        jitter: [
            float(db, collection, field::JITTER_NEAR),
            float(db, collection, field::JITTER_FAR),
        ],
        jitter_period: float(db, collection, field::JITTER_PERIOD),
        width: [
            float(db, collection, field::WIDTH_NEAR),
            float(db, collection, field::WIDTH_FAR),
        ],
        intensity: float(db, collection, field::INTENSITY),
        blend_distance: [
            float(db, collection, field::BLEND_NEAR),
            float(db, collection, field::BLEND_FAR),
        ],
        darken_intensity: float(db, collection, field::DARKEN_INTENSITY),
        samples: short(field::SAMPLES),
        layers: short(field::LAYERS),
    }
}

/// What `VIPresentation` reads through a ribbon's keys.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RibbonStyle {
    /// `ribbon_spline.SplineAssetID` (layout +0) of the ribbon's spline key when that is a
    /// `ribbon_spline` collection: `SplineContainer::mRibbionSplineMap` holds every one.
    pub spline_asset: Option<u64>,
    /// `RwHash64String` of the colour's layout text, the front texture
    /// (`VIPresentation::UpdateLocationSplines` 0x82803528).
    pub front_texture: Option<u64>,
    /// `RwHash64String` of the colour's back texture text, when it names one.
    pub back_texture: Option<u64>,
    pub settings: VisualIndicatorSettings,
}

/// The `ribbon_spline_colour` layout's `char*` and the back texture attribute.
fn colour_textures(db: &AttribDb, colour: u64) -> (Option<u64>, Option<u64>) {
    let hash =
        |name: &str| (!name.is_empty()).then(|| skate2_formats::hash::rw_hash64_string(name));
    let front = db
        .layout_text(class::RIBBON_SPLINE_COLOUR, colour, 0)
        .and_then(hash);
    let back = db
        .attribute(class::RIBBON_SPLINE_COLOUR, colour, field::BACK_TEXTURE)
        .and_then(|a| a.text(0))
        .and_then(hash);
    (front, back)
}

fn layout_u64(db: &AttribDb, class: u64, collection: u64, word: u32) -> u64 {
    (u64::from(db.layout_word(class, collection, word)) << 32)
        | u64::from(db.layout_word(class, collection, word + 1))
}

/// A ribbon's spline, textures and settings by its spline, colour and settings keys.
#[must_use]
pub fn ribbon_style(db: &AttribDb, spline: u64, colour: u64, render_settings: u64) -> RibbonStyle {
    let (front_texture, back_texture) = colour_textures(db, colour);
    RibbonStyle {
        spline_asset: db
            .has_collection(class::RIBBON_SPLINE, spline)
            .then(|| layout_u64(db, class::RIBBON_SPLINE, spline, 0)),
        front_texture,
        back_texture,
        settings: visual_indicator_settings(db, render_settings),
    }
}

/// `VDReader_LocationRibbon`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocationRibbon {
    /// `mTransform`: rows X, Y, Z (W lanes zero) and the translation (W one).
    pub transform: [f32; 16],
    /// `mRibbonSplineKey`: the ribbon indicator's spline RefSpec (layout +8).
    pub spline_key: u64,
    /// `mRibbonColourKey`.
    pub colour_key: u64,
    /// `mRenderSettingsKey`: `RenderSettings`' collection.
    pub render_settings_key: u64,
    pub style: RibbonStyle,
}

/// VD_RaceGate (96 bytes): full matrix, half lengths and the gate/style flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaceGate {
    pub transform: [f32; 16],
    pub half_lengths: [f32; 3],
    pub is_next: bool,
    pub is_finish: bool,
    pub style: RibbonStyle,
    /// UpdateGateSplines leaves a slot's textures alone for colour indices >=32.
    pub colour_valid: bool,
}

/// `VD_SplineInfo`: a streamed spline indicator and its mapped type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplineInfo {
    pub asset: u64,
    pub kind: i32,
}

/// The challenge indicator fields of `VDReaderData` one simulation submit produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VisualIndicatorFrame {
    /// `sVDReader_LocationRibbons`, nearest first.
    pub location_ribbons: Vec<LocationRibbon>,
    /// sRaceGates: up to 32 valid entries, in visible trigger order.
    pub race_gates: Vec<RaceGate>,
    /// `sVDReader_Splines`.
    pub splines: Vec<SplineInfo>,
    /// `sVDReader_FollowMeTargets`: the followed players.
    pub follow_targets: Vec<i32>,
    /// `VIPresentation::mDefaultSettings`, which every ribbon draws its layer count from.
    pub default_settings: VisualIndicatorSettings,
    /// The follow trail's settings (`key_trail`).
    pub trail_settings: VisualIndicatorSettings,
    /// `SkaterPresentation::GetHipsPos` of the first follow target, where
    /// `VIPresentation::UpdateDynamicIndicator` 0x82803fa0 samples the trail.
    pub follow_target_hips: Option<[f32; 3]>,
    /// PC port: the front and back textures of the `blue` ribbon colour, which a linked
    /// Skate 3 data folder's streamed splines (its spots' trick areas, which Skate 3 draws
    /// blue) draw with. Natively a streamed spline binds no texture of its own.
    pub skate3_area_textures: (Option<u64>, Option<u64>),
}

/// `VDWriter::_PrepareChallengeVisualIndicators`' `VD_SplineInfo::mType` of an
/// `eSk8ChallengeIndicators` value.
const fn spline_kind(indicator: i32) -> i32 {
    match indicator {
        1 => 3,
        2 => 2,
        4 => 1,
        8 => 4,
        10 => 5,
        _ => 0,
    }
}

/// `VDWriter::_PrepareChallengeVisualIndicators` (`0x827ff138`) and `VDReader::ReadData`
/// (`0x827fdd78`) for the local skater at `point` (`Actor::GetPosition`).
#[must_use]
pub fn prepare(world: &World, point: [f32; 3]) -> VisualIndicatorFrame {
    let db = &world.db;
    let main = world
        .activities
        .main
        .map(|index| world.activities.activities[index].key);
    let range = MAX_DISTANCE_TO_LOCATION_DATA_TYPE;
    let mut location_ribbons = Vec::new();
    for location in world
        .indicators
        .visible_locations(main, point, range, MAX_LOCATION_RIBBONS)
    {
        // AddLocationRibbon 0x827ff050: both keys must be in the index maps.
        if location.enabled
            && db.has_collection(class::RIBBON_INDICATOR, location.ribbon_indicator_key)
            && db.has_collection(class::RIBBON_SPLINE_COLOUR, location.ribbon_colour_key)
        {
            let compressed = compress_transform(&location.transform);
            location_ribbons.push(read_location_ribbon(
                db,
                compressed,
                location.ribbon_indicator_key,
                location.ribbon_colour_key,
            ));
        }
    }
    let splines = world
        .indicators
        .visible_splines(main, MAX_SPLINES)
        .into_iter()
        .map(|(asset, indicator)| SplineInfo {
            asset,
            kind: spline_kind(indicator),
        })
        .collect();
    // 827ff4a8 requests 80 visible triggers; types 2/3/4 consume at most 32 gates.
    // WriteData827ff928 clears both index fields to zero before AddRaceGate.
    let index_key = |class, key| {
        let keys = db.collections(class);
        let index = keys.binary_search(&key).unwrap_or(0) & 255;
        (keys.get(index).copied().unwrap_or(0), index)
    };
    let race_gates = world.indicators.visible_triggers(main, 80).into_iter()
        .filter(|(_, trigger)| (2..=4).contains(&trigger.indicator))
        .filter_map(|(volume, trigger)| {
            let (transform, half_lengths) = world.host.race_gate_volume(volume)?;
            let (ribbon, _) = index_key(class::RIBBON_INDICATOR, trigger.ribbon_indicator_key);
            let (colour, colour_index) = index_key(class::RIBBON_SPLINE_COLOUR, trigger.ribbon_colour_key);
            let spline = layout_u64(db, class::RIBBON_INDICATOR, ribbon, 2);
            let settings = db.attribute(class::RIBBON_INDICATOR, ribbon, field::RENDER_SETTINGS)
                .and_then(|a| a.ref_spec(0)).map_or(0, |(_, key)| key);
            Some(RaceGate {
                transform, half_lengths,
                is_next: trigger.indicator == 2,
                is_finish: trigger.indicator == 4,
                style: ribbon_style(db, spline, colour, settings),
                colour_valid: colour_index < 32,
            })
        }).take(32).collect();
    VisualIndicatorFrame {
        location_ribbons,
        race_gates,
        splines,
        follow_targets: world.indicators.visible_follows(main, MAX_FOLLOW_TARGETS),
        default_settings: visual_indicator_settings(db, DEFAULT_SETTINGS),
        trail_settings: visual_indicator_settings(db, TRAIL_SETTINGS),
        follow_target_hips: None,
        skate3_area_textures: colour_textures(db, BLUE),
    }
}

/// `VDReader::ReadData`'s location ribbon (0x827fdfdc..0x827fe1c4): the restored matrix with
/// `Offset` (`a0`, `a1`, `a2`) along its rows added to the translation, and the keys.
fn read_location_ribbon(db: &AttribDb, compressed: V4, ribbon: u64, colour: u64) -> LocationRibbon {
    let mut transform = decompress_transform(compressed);
    let offset = db
        .attribute(class::RIBBON_INDICATOR, ribbon, field::OFFSET)
        .and_then(|a| a.value_bytes(0))
        .map_or([0.0; 4], |b| {
            std::array::from_fn(|i| {
                f32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]])
            })
        });
    for lane in 0..3 {
        let along = offset[0].mul_add(transform[lane], offset[1] * transform[4 + lane]);
        transform[12 + lane] += offset[2].mul_add(transform[8 + lane], along);
    }
    let spline_key = layout_u64(db, class::RIBBON_INDICATOR, ribbon, 2);
    let render_settings_key = db
        .attribute(class::RIBBON_INDICATOR, ribbon, field::RENDER_SETTINGS)
        .and_then(|a| a.ref_spec(0))
        .map_or(0, |(_, collection)| collection);
    LocationRibbon {
        transform,
        spline_key,
        colour_key: colour,
        render_settings_key,
        style: ribbon_style(db, spline_key, colour, render_settings_key),
    }
}

const PI: f32 = f32::from_bits(0x4049_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);

fn host() -> HostReckoningNumeric {
    HostReckoningNumeric
}

/// `vrefp` and one fused Newton round.
fn reciprocal_once(x: f32) -> f32 {
    let seed = host().reciprocal_estimate(x).unwrap_or(f32::NAN);
    seed.mul_add((-seed).mul_add(x, 1.0), seed)
}

/// `vmaxfp`/`vminfp` clamp then `fctidz`'s low word: NaN converts to zero.
fn quantize(value: f32, maximum: f32) -> u32 {
    if value.is_nan() {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=2047"
    )]
    let code = value.max(0.0).min(maximum).trunc() as u32;
    code
}

/// `ANIM_CODEC::RawDiscretize::QuatCompress` (`0x825e9b00`): azimuth (11 bits), elevation
/// (10 bits) and length (11 bits) of the vector part, negated when W is negative.
#[must_use]
pub fn quat_compress(q: V4) -> u32 {
    let [mut x, mut y, mut z] = [q[0], q[1], q[2]];
    if 0.0 > q[3] {
        [x, y, z] = [-x, -y, -z];
    }
    let length_squared = z.mul_add(z, x.mul_add(x, y * y));
    let length = vmx::sqrt_guarded(length_squared);
    let (azimuth, elevation) = if length > f32::from_bits(0x3727_c5ac) {
        let elevation =
            crate::reckoning_numeric::try_asin(vmx::reciprocal(length) * z, &mut host())
                .unwrap_or(f32::NAN);
        let angle = crate::skeleton_query_rotation::try_atan(y * reciprocal_once(x), &mut host())
            .unwrap_or(f32::NAN);
        let sign = y.to_bits() & 0x8000_0000;
        let azimuth = if x == 0.0 {
            f32::from_bits(HALF_PI.to_bits() | sign)
        } else if 0.0 > x {
            f32::from_bits(PI.to_bits() | sign) + angle
        } else {
            angle
        };
        (azimuth, elevation)
    } else {
        (0.0, 0.0)
    };
    let length_code = quantize(length.mul_add(2047.0, 0.5), 2047.0);
    let azimuth_code = quantize(
        (vmx::reciprocal(TWO_PI) * (azimuth + PI)).mul_add(2047.0, 0.5),
        2047.0,
    );
    let elevation_code = quantize(
        (vmx::reciprocal(PI) * (elevation + HALF_PI)).mul_add(1023.0, 0.5),
        1023.0,
    );
    (((azimuth_code << 10) | elevation_code) << 11) | length_code
}

/// `ANIM_CODEC::RawDiscretize::QuatDecompress` (`0x825e9e68`) with `AnimSinCos2` 0x825e9a30.
#[must_use]
pub fn quat_decompress(code: u32) -> V4 {
    #[expect(clippy::cast_precision_loss, reason = "codes of at most 11 bits")]
    let [length, elevation, azimuth] = [
        (code & 0x7ff) as f32,
        ((code >> 11) & 0x3ff) as f32,
        (code >> 21) as f32,
    ];
    let a = azimuth.mul_add(f32::from_bits(0x3b49_2900), -PI);
    let b = elevation.mul_add(f32::from_bits(0x3b49_422c), -HALF_PI);
    let s = length * f32::from_bits(0x3a00_1002);
    let sin_cos = [vmx::sin(a), vmx::cos(a), vmx::sin(b), vmx::cos(b)];
    let scaled = sin_cos.map(|lane| lane * s);
    let w = vmx::sqrt_guarded((-s).mul_add(s, 1.0));
    [scaled[1] * sin_cos[3], scaled[0] * sin_cos[3], scaled[2], w]
}

/// `Sk8::CompressedFromMatrix44Affine` (`0x825e9ff0`): the translation, with `QuatCompress`
/// of `QuatFromMat`'s quaternion in the W lane.
#[must_use]
pub fn compress_transform(matrix: &[f32; 16]) -> V4 {
    let quaternion = crate::world::try_simulation_quaternion_candidate(*matrix, |magnitudes| {
        let mut seeds = [0.0; 4];
        for (seed, magnitude) in seeds.iter_mut().zip(magnitudes) {
            *seed = host().reciprocal_sqrt_estimate(magnitude)?;
        }
        Ok::<_, crate::reckoning_numeric::ReckoningHostNumericError>(seeds)
    })
    .unwrap_or([f32::NAN; 4]);
    [
        matrix[12],
        matrix[13],
        matrix[14],
        f32::from_bits(quat_compress(quaternion)),
    ]
}

/// `Sk8::Matrix44FromCompressed` (`0x827fd8c0`): the `gSqrt2s` basis with its W lanes
/// masked, and the translation with W one.
#[must_use]
pub fn decompress_transform(compressed: V4) -> [f32; 16] {
    let rows = vmx::basis_from_quaternion(quat_decompress(compressed[3].to_bits()));
    let mut matrix = [0.0; 16];
    for (row, axis) in rows.iter().enumerate() {
        matrix[row * 4..row * 4 + 3].copy_from_slice(&axis[..3]);
    }
    matrix[12..15].copy_from_slice(&compressed[..3]);
    matrix[15] = 1.0;
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn an_identity_orientation_round_trips_to_the_identity() {
        let identity = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 3., 4., 5., 1.,
        ];
        let compressed = compress_transform(&identity);
        assert_eq!(compressed[..3], [3., 4., 5.]);
        // A zero vector part: azimuth and elevation codes sit at the middle of their ranges.
        assert_eq!(compressed[3].to_bits(), (1024 << 21) | (512 << 11));
        let restored = decompress_transform(compressed);
        for (index, (&a, &b)) in restored.iter().zip(&identity).enumerate() {
            assert!(close(a, b, 1e-3), "lane {index}: {a} against {b}");
        }
        assert_eq!(restored[3], 0.0);
        assert_eq!(restored[15], 1.0);
    }

    #[test]
    fn a_yaw_round_trips_within_the_quantization() {
        let (sin, cos) = 0.7_f32.sin_cos();
        let yaw = [
            cos, 0., -sin, 0., 0., 1., 0., 0., sin, 0., cos, 0., -1., 2., 30., 1.,
        ];
        let restored = decompress_transform(compress_transform(&yaw));
        for (index, (&a, &b)) in restored.iter().zip(&yaw).enumerate() {
            assert!(close(a, b, 4e-3), "lane {index}: {a} against {b}");
        }
    }

    #[test]
    fn quaternion_codes_place_azimuth_elevation_and_length() {
        // Half a turn about Z: the vector part is +Z (elevation pi/2), length one.
        let code = quat_compress([0., 0., 1., 0.]);
        assert_eq!(code & 0x7ff, 2047);
        assert_eq!((code >> 11) & 0x3ff, 1023);
        let q = quat_decompress(code);
        assert!(close(q[2], 1.0, 1e-3) && close(q[3], 0.0, 1e-2), "{q:?}");
        // A negative W flips the vector part first.
        assert_eq!(
            quat_compress([-0.36, -0.48, -0.8, -0.1]),
            quat_compress([0.36, 0.48, 0.8, 0.1])
        );
    }

    #[test]
    fn spline_kinds_follow_the_writer_switch() {
        assert_eq!(
            [1, 2, 3, 4, 8, 10, 9].map(spline_kind),
            [3, 2, 0, 1, 4, 5, 0]
        );
    }

    fn mounts() -> skate2_formats::mount::MountSet {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let mut mounts = MountSet::new();
        for path in [
            "data\\db\\db.big",
            "data\\big\\miscload.big",
            "data\\big\\miscboot.big",
        ] {
            let archive = SKATE2_ARCHIVES
                .iter()
                .find(|a| a.relative_path == path)
                .unwrap();
            mounts.push(
                MountedBig::open(
                    archive.path_under(std::path::Path::new(&root)),
                    archive.use_external_header,
                )
                .unwrap(),
            );
        }
        mounts
    }

    /// The career's challenge system without a started game, and m_1a_03's `SignUpIndicator`
    /// ribbon (`challenge_global_data` 0x284b07325d587cc4).
    fn career_and_sign_up_ribbon() -> (super::super::ChallengeSystem, u64) {
        let mounts = mounts();
        let db = std::sync::Arc::new(
            AttribDb::load(&mounts, &super::super::attrib_db::CAREER_VAULTS).unwrap(),
        );
        let global = mounts
            .read_path("data\\script\\challenge\\global.lua")
            .unwrap();
        let system =
            super::super::ChallengeSystem::new_career(std::sync::Arc::clone(&db), &global).unwrap();
        let activity = system
            .world()
            .activities
            .activities
            .iter()
            .find(|a| a.name == "m_1a_03_tutorial_fliptricks")
            .unwrap();
        let (class, key) = activity.global_data.unwrap();
        let (_, ribbon) = db
            .attribute(class, key, 0x284b_0732_5d58_7cc4)
            .and_then(|a| a.ref_spec(0))
            .unwrap();
        (system, ribbon)
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn a_sign_up_location_within_range_becomes_a_ribbon_with_its_vault_style() {
        let (mut system, ribbon) = career_and_sign_up_ribbon();
        let world = system.world_mut();
        let locator = [-120.0, 4.0, 250.0];
        let transform = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., locator[0], locator[1], locator[2], 1.,
        ];
        let group = world.indicators.create_group(0);
        world
            .indicators
            .add_location(group, 1, transform, ribbon, u64::MAX);
        let near = prepare(world, [locator[0] + 3.0, locator[1], locator[2]]);
        assert_eq!(near.location_ribbons.len(), 1);
        let ribbon_record = &near.location_ribbons[0];
        assert_eq!(
            ribbon_record.colour_key,
            super::super::keys::hash64("default")
        );
        let db = &world.db;
        let offset = db
            .attribute(class::RIBBON_INDICATOR, ribbon, field::OFFSET)
            .and_then(|a| a.value_bytes(0))
            .unwrap();
        let up = f32::from_be_bytes([offset[4], offset[5], offset[6], offset[7]]);
        assert!(
            close(ribbon_record.transform[13], locator[1] + up, 1e-3),
            "{:?}",
            ribbon_record.transform
        );
        assert!(
            close(ribbon_record.transform[12], locator[0], 1e-3)
                && close(ribbon_record.transform[14], locator[2], 1e-3)
        );
        let style = ribbon_record.style;
        assert!(
            style.spline_asset.is_some(),
            "the sign-up ribbon names a ribbon_spline"
        );
        assert_eq!(
            style.front_texture,
            Some(skate2_formats::hash::rw_hash64_string(
                "visual_indicator_orange.Texture"
            ))
        );
        assert!(
            style.settings.params[7] > style.settings.params[6] && style.settings.layers > 0,
            "{:?}",
            style.settings
        );
        assert_eq!(near.default_settings.layers, 3);
        assert_eq!(near.trail_settings.samples, 10);
        // kMaxDistanceToLocationDataType: 500 units.
        let far = prepare(world, [locator[0] + 600.0, locator[1], locator[2]]);
        assert!(far.location_ribbons.is_empty());
        // A colour that is no ribbon_spline_colour collection is not written.
        world.indicators.clear_all(group);
        world
            .indicators
            .add_location(group, 1, transform, ribbon, 5);
        assert!(prepare(world, locator).location_ribbons.is_empty());
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn the_nearest_locations_in_range_fill_the_ribbons_nearest_first() {
        let (mut system, ribbon) = career_and_sign_up_ribbon();
        let world = system.world_mut();
        let others = world.indicators.create_group(0);
        for index in 0..30_u32 {
            #[expect(clippy::cast_precision_loss, reason = "test offsets")]
            let x = index as f32 * 4.0;
            let transform = [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., x, 0., 0., 1.,
            ];
            world
                .indicators
                .add_location(others, index, transform, ribbon, u64::MAX);
        }
        let frame = prepare(world, [0.0, 0.0, 0.0]);
        assert_eq!(frame.location_ribbons.len(), MAX_LOCATION_RIBBONS);
        let xs: Vec<f32> = frame
            .location_ribbons
            .iter()
            .map(|r| r.transform[12])
            .collect();
        assert!(
            xs.windows(2).all(|pair| pair[0] <= pair[1]),
            "nearest first: {xs:?}"
        );
        assert!(close(xs[21], 84.0, 1e-3));
    }

    /// The session's `cLocationManager` with one registered location.
    struct Registered(u32, [f32; 16]);

    impl super::super::host::Host for Registered {
        fn dmo_locator_from_lua_id(&mut self, _: f32) -> Option<u64> {
            None
        }
        fn is_dmo_in_volume(&mut self, _: u64, _: f32) -> bool {
            false
        }
        fn trigger_volume_center(&mut self, _: f32) -> Option<[f32; 3]> {
            None
        }
        fn location_orientation(&self, location: u32) -> Option<[f32; 16]> {
            (location == self.0).then_some(self.1)
        }
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn a_challenge_stream_locator_places_its_location_ribbon() {
        use super::super::indicators::add_location_indicator;
        let (mut system, ribbon) = career_and_sign_up_ribbon();
        let world = system.world_mut();
        // m_1a_01_goto_slappys' cSim_Global locator, which the location table does not hold.
        let name = "m_1a_01_goto_slappys_new_vi01";
        let at = [1510.41, 110.78, -717.74];
        let transform = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., at[0], at[1], at[2], 1.,
        ];
        world.host = Box::new(Registered(
            super::super::keys::hash_value32(name),
            transform,
        ));
        let group = world.indicators.create_group(0);
        add_location_indicator(world, group, name, ribbon, u64::MAX);
        let frame = prepare(world, at);
        assert_eq!(frame.location_ribbons.len(), 1);
        assert!(close(frame.location_ribbons[0].transform[12], at[0], 1e-3));
        // Unregistered, the location is DefaultLocationOrientation's origin, out of range.
        world.host = Box::new(super::super::host::NullHost);
        world.indicators.clear_all(group);
        add_location_indicator(world, group, name, ribbon, u64::MAX);
        assert!(prepare(world, at).location_ribbons.is_empty());
    }
}
