//! The replay frames online players send each other: each simulation tick of a skater as the
//! replay recorder keeps it (`SkaterPresentation::WriteData` 0x827dbf98's header, the decoded
//! pose and twist ratios, the camera and `AudioPresentation::DoSimSubmit` 0x827fb2f8's audio
//! record).
//!
//! Only what a remote skater's restore, its audio and a spectator's view read travels: the
//! header's root rotation, world position, board speed, peer and flag words (its hips and board
//! matrices repeat the pose) and the audio record's words every tick, and on sample ticks the
//! pose, twist ratios, the audio record's floats and the game camera, and every tick the trick
//! HUD's view of the skater (`skate2_core::online::hud_record`, opaque words here). The receiver carries the
//! floats and twist of the last tick that had them. A sender's ticks travel in runs, each ending
//! in a sample: [`PLAYING_INTERVAL`] ticks while its skater plays a challenge round,
//! [`INTERVAL`] otherwise. While the other players watch the skater every tick of its runs is a
//! sample, the later ones carrying the run's first audio floats.
//!
//! Each tick is quantized, then coded against frames the receiver has acknowledged and the run's
//! ticks before it: the root, board speed, pose and camera predicted on from the two frames (or
//! samples) before, the acknowledged ones at least [`FRAME_SPAN`] (or [`SAMPLE_SPAN`]) apart, a
//! rotation's largest part from the norm its other three leave, the rest from the frame or sample
//! before. What remains is coded with an adaptive range coder
//! ([`crate::range`]) whose contexts sort each value by how far its prediction moved it, so a
//! value that stays costs a small fraction of a bit. Acks name the newest frame received and which
//! of the 32 before it arrived, so the sender only codes against frames the receiver holds, and a
//! run never waits for a lost one.

use crate::codec::{DecodeError, DecodeResult};
use crate::range::{Bit, Integer, RangeDecoder, RangeEncoder};
use std::collections::VecDeque;
use std::sync::Arc;

/// `SkaterPresentation::Data`: 208 bytes as big-endian words.
pub const HEADER_WORDS: usize = 52;
/// An `AndaleSqt`: scale, rotation and translation lanes.
pub const BONE_LANES: usize = 12;
/// Ticks a run spans while the skater plays a challenge round (its turn, or a challenge everyone
/// plays at once): 30 runs a second.
pub const PLAYING_INTERVAL: u32 = 2;
/// Ticks a run spans otherwise: 20 runs a second.
pub const INTERVAL: u32 = 3;
/// The fewest ticks between the two acknowledged frames, and between the two acknowledged
/// samples, a run is predicted on from: a rate over a longer span is steadier against the
/// quantization's rounding when it is carried across the acknowledgement delay.
pub const FRAME_SPAN: u32 = 2;
pub const SAMPLE_SPAN: u32 = 8;
/// The most ticks one run holds.
pub const MAX_RUN: usize = 8;
/// The camera's values: position, rotation quaternion and vertical field of view.
pub const CAMERA_VALUES: usize = 8;
/// Frames each side keeps as baselines (one second at 60 Hz).
pub const HISTORY: usize = 64;
/// The most bones a pose may have.
pub const MAX_BONES: usize = 512;
/// The most values in the camera or an audio list.
pub const MAX_VALUES: usize = 1024;
/// The most words of a tick's trick HUD record.
pub const MAX_HUD_WORDS: usize = 64;

/// Header words 32 to 38: the root rotation quaternion and world position (`ReadSkater`'s
/// bytes 128 and 144).
const ROOT_WORD: usize = 32;
const ROOT_VALUES: usize = 7;
/// Header word 46, byte 184: the board speed, which no restore reads, so it travels quantized.
const SPEED_WORD: usize = 46;
/// The other header words from 39 on travel as they are (the position's W carrier, peer id,
/// damage, player and opacity, flags).
const KEPT_FROM: usize = ROOT_WORD + ROOT_VALUES;
const KEPT_WORDS: usize = HEADER_WORDS - KEPT_FROM - 1;

const fn kept_word(index: usize) -> usize {
    if KEPT_FROM + index < SPEED_WORD { KEPT_FROM + index } else { KEPT_FROM + index + 1 }
}

/// The bone a pose without a baseline is coded against: unit scale, no rotation, no translation.
const NEUTRAL_BONE: [i32; BONE_LANES] = [1024, 1024, 1024, 1024, 0, 0, 0, 1024, 0, 0, 0, 0];
const SCALE_UNITS: f32 = 1024.0;
const ROTATION_UNITS: f32 = 1024.0;
const TRANSLATION_UNITS: f32 = 1024.0;
const ROOT_ROTATION_UNITS: f32 = 32768.0;
const POSITION_UNITS: f32 = 1024.0;
const SPEED_UNITS: f32 = 256.0;
const TWIST_UNITS: f32 = 4096.0;
const CAMERA_POSITION_UNITS: f32 = 1024.0;
const CAMERA_ROTATION_UNITS: f32 = 16384.0;
const CAMERA_FOV_UNITS: f32 = 256.0;
const AUDIO_UNITS: f32 = 256.0;
/// A unit quaternion's length in the units of each kind of rotation.
const ROTATION_ONE: i64 = 1024;
const ROOT_ROTATION_ONE: i64 = 32768;
const CAMERA_ROTATION_ONE: i64 = 16384;

const fn camera_units(value: usize) -> f32 {
    match value {
        0..=2 => CAMERA_POSITION_UNITS,
        3..=6 => CAMERA_ROTATION_UNITS,
        _ => CAMERA_FOV_UNITS,
    }
}

/// One tick of a remote skater, in the units the game records it in.
#[derive(Clone, Debug, PartialEq)]
pub struct NetFrame {
    pub tick: u32,
    pub header: [u32; HEADER_WORDS],
    /// The pose on sample ticks.
    pub pose: Option<Vec<[f32; BONE_LANES]>>,
    pub twist: [f32; 4],
    /// The game camera on sample ticks: its position, rotation quaternion (x, y, z, w) and
    /// vertical field of view in degrees; empty without one.
    pub camera: Vec<f32>,
    pub audio_floats: Vec<f32>,
    pub audio_words: Vec<u32>,
    /// The tick's trick HUD record; empty without one.
    pub hud: Vec<u32>,
}

/// The values a sample tick adds: the pose, twist ratios, audio floats and camera.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Sample {
    pose: Vec<[i32; BONE_LANES]>,
    twist: [i32; 4],
    audio_floats: Vec<i32>,
    camera: Vec<i32>,
}

/// A frame on the quantized grid both sides code against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantFrame {
    pub tick: u32,
    root: [i32; ROOT_VALUES],
    speed: i32,
    words: [u32; KEPT_WORDS],
    sample: Option<Arc<Sample>>,
    audio_words: Vec<u32>,
    hud: Vec<u32>,
}

fn quantize(value: f32, units: f32) -> i32 {
    if value.is_finite() {
        #[expect(clippy::cast_possible_truncation, reason = "clamped to the i32 range first")]
        let quantized = (value * units).round().clamp(-2_147_483_648.0, 2_147_483_520.0) as i32;
        quantized
    } else {
        0
    }
}

#[expect(clippy::cast_precision_loss, reason = "the quantized grid is coarser than f32's")]
fn dequantize(value: i32, units: f32) -> f32 {
    value as f32 / units
}

const fn lane_units(lane: usize) -> f32 {
    match lane / 4 {
        0 => SCALE_UNITS,
        1 => ROTATION_UNITS,
        _ => TRANSLATION_UNITS,
    }
}

const fn root_units(value: usize) -> f32 {
    if value < 4 { ROOT_ROTATION_UNITS } else { POSITION_UNITS }
}

impl QuantFrame {
    #[must_use]
    pub fn quantize(frame: &NetFrame) -> Self {
        Self {
            tick: frame.tick,
            root: std::array::from_fn(|value| quantize(f32::from_bits(frame.header[ROOT_WORD + value]), root_units(value))),
            speed: quantize(f32::from_bits(frame.header[SPEED_WORD]), SPEED_UNITS),
            words: std::array::from_fn(|word| frame.header[kept_word(word)]),
            sample: frame.pose.as_ref().map(|pose| {
                Arc::new(Sample {
                    pose: pose.iter().map(|bone| std::array::from_fn(|lane| quantize(bone[lane], lane_units(lane)))).collect(),
                    twist: frame.twist.map(|value| quantize(value, TWIST_UNITS)),
                    audio_floats: frame.audio_floats.iter().map(|&value| quantize(value, AUDIO_UNITS)).collect(),
                    camera: frame.camera.iter().enumerate().map(|(index, &value)| quantize(value, camera_units(index))).collect(),
                })
            }),
            audio_words: frame.audio_words.clone(),
            hud: frame.hud.clone(),
        }
    }

    #[must_use]
    pub fn has_pose(&self) -> bool {
        self.sample.is_some()
    }

    fn empty() -> Self {
        Self { tick: 0, root: [0; ROOT_VALUES], speed: 0, words: [0; KEPT_WORDS], sample: None, audio_words: Vec::new(), hud: Vec::new() }
    }
}

/// A run of a sender's ticks, oldest first, coded together against the frames the receiver
/// holds: the newest and the one before it, and the newest sample and the sample before that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedRun {
    /// The run's first tick and how many ticks it holds.
    pub tick: u32,
    pub count: u8,
    pub frame: Option<u32>,
    pub frame_before: Option<u32>,
    pub sample: Option<u32>,
    pub sample_before: Option<u32>,
    pub payload: Vec<u8>,
}

/// One side of the coding: encoding reads each value, decoding writes it.
trait Channel {
    fn decoding(&self) -> bool;
    fn bit(&mut self, context: &mut Bit, value: &mut bool);
    fn direct(&mut self, value: &mut u64, count: u32);
    fn signed(&mut self, context: &mut Integer, value: &mut i64) -> DecodeResult<()>;
    fn unsigned(&mut self, context: &mut Integer, value: &mut u64) -> DecodeResult<()>;
}

impl Channel for RangeEncoder {
    fn decoding(&self) -> bool {
        false
    }

    fn bit(&mut self, context: &mut Bit, value: &mut bool) {
        Self::bit(self, context, *value);
    }

    fn direct(&mut self, value: &mut u64, count: u32) {
        Self::direct(self, *value, count);
    }

    fn signed(&mut self, context: &mut Integer, value: &mut i64) -> DecodeResult<()> {
        Self::signed(self, context, *value);
        Ok(())
    }

    fn unsigned(&mut self, context: &mut Integer, value: &mut u64) -> DecodeResult<()> {
        Self::unsigned(self, context, *value);
        Ok(())
    }
}

impl Channel for RangeDecoder<'_> {
    fn decoding(&self) -> bool {
        true
    }

    fn bit(&mut self, context: &mut Bit, value: &mut bool) {
        *value = Self::bit(self, context);
    }

    fn direct(&mut self, value: &mut u64, count: u32) {
        *value = Self::direct(self, count);
    }

    fn signed(&mut self, context: &mut Integer, value: &mut i64) -> DecodeResult<()> {
        *value = Self::signed(self, context)?;
        Ok(())
    }

    fn unsigned(&mut self, context: &mut Integer, value: &mut u64) -> DecodeResult<()> {
        *value = Self::unsigned(self, context)?;
        Ok(())
    }
}

/// Contexts per bucket: the bit length of how far a value's prediction moved it from its
/// baseline, capped.
const BUCKETS: usize = 8;

/// One kind of value's contexts, a set per bucket.
#[derive(Clone, Debug, Default)]
struct Classes([Integer; BUCKETS]);

impl Classes {
    fn by_step(&mut self, step: i64) -> &mut Integer {
        let length = u64::BITS - step.unsigned_abs().leading_zeros();
        &mut self.0[usize::try_from(length).map_or(BUCKETS - 1, |length| length.min(BUCKETS - 1))]
    }
}

/// A list of words' contexts: how many changed, and each one's distance past the last changed.
#[derive(Clone, Debug, Default)]
struct Words {
    changed: Integer,
    gap: Integer,
}

/// The contexts of one run's coding, fresh for each run.
#[derive(Clone, Debug, Default)]
struct Models {
    tick_gap: Integer,
    root_rotation: Classes,
    root_largest: Integer,
    root_position: Classes,
    speed: Classes,
    words: Words,
    audio_words: Words,
    hud_words: Words,
    sample: Bit,
    same_length: Bit,
    length: Integer,
    scale: Classes,
    rotation: Classes,
    largest: Integer,
    translation: Classes,
    twist: Classes,
    audio_floats: Classes,
    camera: Classes,
    camera_rotation: Classes,
    camera_largest: Integer,
}

fn bounded(count: u64, max: usize) -> DecodeResult<usize> {
    usize::try_from(count).ok().filter(|&count| count <= max).ok_or(DecodeError("count out of range"))
}

fn count_of(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

/// `value` as its residual from `predicted`, in `context`.
fn residual(channel: &mut impl Channel, context: &mut Integer, predicted: i32, value: &mut i32) -> DecodeResult<()> {
    let mut difference = i64::from(*value) - i64::from(predicted);
    channel.signed(context, &mut difference)?;
    *value = i64::from(predicted).checked_add(difference).and_then(|value| i32::try_from(value).ok()).ok_or(DecodeError("value out of range"))?;
    Ok(())
}

/// `value` as its residual from `predicted`, in the context of its prediction's step from `base`.
fn stepped(channel: &mut impl Channel, classes: &mut Classes, base: i32, predicted: i32, value: &mut i32) -> DecodeResult<()> {
    residual(channel, classes.by_step(i64::from(predicted) - i64::from(base)), predicted, value)
}

/// A rotation quaternion of `one`-unit lanes against its prediction: the lanes but the
/// prediction's largest as residuals, then the largest as its residual from the length the
/// other three leave it, with the prediction's sign.
fn quaternion(channel: &mut impl Channel, lanes: &mut [i32], predicted: &[i32], base: &[i32], classes: &mut Classes, largest: &mut Integer, one: i64) -> DecodeResult<()> {
    let top = (0..4).max_by_key(|&lane| predicted[lane].unsigned_abs()).unwrap_or(3);
    for lane in (0..4).filter(|&lane| lane != top) {
        stepped(channel, classes, base[lane], predicted[lane], &mut lanes[lane])?;
    }
    let others: i128 = (0..4).filter(|&lane| lane != top).map(|lane| i128::from(lanes[lane]).pow(2)).sum();
    let left = u64::try_from((i128::from(one * one) - others).max(0)).unwrap_or(0);
    let root = left.isqrt();
    let root = i32::try_from(if left - root * root > root { root + 1 } else { root }).unwrap_or(i32::MAX);
    residual(channel, largest, if predicted[top] < 0 { -root } else { root }, &mut lanes[top])
}

/// `value` moved on by `ahead` ticks at the rate it moved from `earlier` over `span` ticks.
fn extrapolate(value: i32, earlier: i32, ahead: i64, span: i64) -> i32 {
    let moved = (i64::from(value) - i64::from(earlier)).saturating_mul(ahead);
    let value = i64::from(value).saturating_add(moved.saturating_add(span / 2).div_euclid(span));
    i32::try_from(value.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
}

/// `last`, of tick `last_tick`, as it would be at `tick` moving on as it moved since `before`.
fn moved(last: i32, last_tick: u32, before: Option<(i32, u32)>, tick: u32) -> i32 {
    match before {
        Some((earlier, before_tick)) => {
            let span = i64::from(last_tick.wrapping_sub(before_tick).max(1));
            extrapolate(last, earlier, i64::from(tick.wrapping_sub(last_tick)), span)
        }
        None => last,
    }
}

/// What a tick is coded from: the two frames before it and the two samples before it, the
/// acknowledged ones and then the run's own.
struct Chain {
    before: Option<QuantFrame>,
    last: Option<QuantFrame>,
    sample_before: Option<(u32, Arc<Sample>)>,
    sample: Option<(u32, Arc<Sample>)>,
}

impl Chain {
    fn new(baselines: &Baselines<'_>) -> Self {
        let sample = |frame: Option<&QuantFrame>| frame.and_then(|frame| Some((frame.tick, Arc::clone(frame.sample.as_ref()?))));
        Self { before: baselines.frame_before.cloned(), last: baselines.frame.cloned(), sample_before: sample(baselines.sample_before), sample: sample(baselines.sample) }
    }

    fn push(&mut self, frame: &QuantFrame) {
        self.before = self.last.replace(frame.clone());
        if let Some(sample) = &frame.sample {
            self.sample_before = self.sample.replace((frame.tick, Arc::clone(sample)));
        }
    }
}

/// Words against the ones before: how many changed, then each one's distance past the last
/// changed and the bits it flipped.
fn code_words(channel: &mut impl Channel, words: &mut [u32], base: &[u32], models: &mut Words) -> DecodeResult<()> {
    if channel.decoding() {
        words.copy_from_slice(base);
    }
    let mut changed = count_of(words.iter().zip(base).filter(|(word, base)| word != base).count());
    channel.unsigned(&mut models.changed, &mut changed)?;
    let mut at = 0;
    for _ in 0..bounded(changed, words.len())? {
        let mut gap = count_of((at..words.len()).position(|index| words[index] != base[index]).unwrap_or(0));
        channel.unsigned(&mut models.gap, &mut gap)?;
        let index = at.checked_add(bounded(gap, words.len())?).filter(|&index| index < words.len()).ok_or(DecodeError("changed word out of range"))?;
        let flipped = words[index] ^ base[index];
        let mut top = u64::from(31 - flipped.leading_zeros().min(31));
        channel.direct(&mut top, 5);
        let top = u32::try_from(top).unwrap_or(0);
        let mut rest = u64::from(flipped) & ((1 << top) - 1);
        channel.direct(&mut rest, top);
        words[index] = base[index] ^ (1 << top) ^ u32::try_from(rest).unwrap_or(0);
        at = index + 1;
    }
    Ok(())
}

/// A list's length: the same as its baseline's, or coded.
fn code_length(channel: &mut impl Channel, length: usize, base: Option<usize>, models: &mut Models, max: usize) -> DecodeResult<(usize, bool)> {
    let mut same = base == Some(length);
    if base.is_some() {
        channel.bit(&mut models.same_length, &mut same);
    }
    match base.filter(|_| same) {
        Some(base) => Ok((base, true)),
        None => {
            let mut length = count_of(length);
            channel.unsigned(&mut models.length, &mut length)?;
            Ok((bounded(length, max)?, false))
        }
    }
}

/// A list of up to `max` words against `base`, in the contexts `contexts` picks.
fn code_word_list(channel: &mut impl Channel, words: &mut Vec<u32>, base: &[u32], models: &mut Models, max: usize, contexts: fn(&mut Models) -> &mut Words) -> DecodeResult<()> {
    let (count, same) = code_length(channel, words.len(), Some(base.len()), models, max)?;
    if channel.decoding() {
        words.clear();
        words.resize(count, 0);
    }
    let zeros = if same { Vec::new() } else { vec![0; count] };
    code_words(channel, words, if same { base } else { &zeros }, contexts(models))
}

/// A tick's header values, audio words and trick HUD record.
fn code_header(channel: &mut impl Channel, frame: &mut QuantFrame, chain: &Chain, models: &mut Models) -> DecodeResult<()> {
    let tick = frame.tick;
    let last = chain.last.as_ref();
    let before = chain.before.as_ref().filter(|_| last.is_some());
    let base = last.map_or([0; ROOT_VALUES], |last| last.root);
    let predicted: [i32; ROOT_VALUES] =
        std::array::from_fn(|value| last.map_or(0, |last| moved(last.root[value], last.tick, before.map(|before| (before.root[value], before.tick)), tick)));
    quaternion(channel, &mut frame.root[..4], &predicted[..4], &base[..4], &mut models.root_rotation, &mut models.root_largest, ROOT_ROTATION_ONE)?;
    for value in 4..ROOT_VALUES {
        stepped(channel, &mut models.root_position, base[value], predicted[value], &mut frame.root[value])?;
    }
    let speed = last.map_or(0, |last| moved(last.speed, last.tick, before.map(|before| (before.speed, before.tick)), tick));
    stepped(channel, &mut models.speed, last.map_or(0, |last| last.speed), speed, &mut frame.speed)?;
    code_words(channel, &mut frame.words, &last.map_or([0; KEPT_WORDS], |last| last.words), &mut models.words)?;
    code_word_list(channel, &mut frame.audio_words, last.map_or(&[], |last| last.audio_words.as_slice()), models, MAX_VALUES, |models| &mut models.audio_words)?;
    code_word_list(channel, &mut frame.hud, last.map_or(&[], |last| last.hud.as_slice()), models, MAX_HUD_WORDS, |models| &mut models.hud_words)
}

/// A sample's pose: each bone's scale and translation lanes and its rotation, predicted on from
/// the two samples before.
fn code_pose(channel: &mut impl Channel, pose: &mut Vec<[i32; BONE_LANES]>, tick: u32, base: Option<&(u32, Arc<Sample>)>, before: Option<&(u32, Arc<Sample>)>, models: &mut Models) -> DecodeResult<()> {
    let (count, same) = code_length(channel, pose.len(), base.map(|(_, base)| base.pose.len()), models, MAX_BONES)?;
    if channel.decoding() {
        pose.clear();
        pose.resize(count, NEUTRAL_BONE);
    }
    let base = base.filter(|_| same);
    let before = before.filter(|(_, before)| before.pose.len() == count);
    for (index, bone) in pose.iter_mut().enumerate() {
        let (base_bone, predicted) = match base {
            Some((base_tick, base)) => {
                let base_bone = base.pose[index];
                let predicted = std::array::from_fn(|lane| moved(base_bone[lane], *base_tick, before.map(|(tick, before)| (before.pose[index][lane], *tick)), tick));
                (base_bone, predicted)
            }
            None => (NEUTRAL_BONE, NEUTRAL_BONE),
        };
        for lane in 0..4 {
            stepped(channel, &mut models.scale, base_bone[lane], predicted[lane], &mut bone[lane])?;
        }
        quaternion(channel, &mut bone[4..8], &predicted[4..8], &base_bone[4..8], &mut models.rotation, &mut models.largest, ROTATION_ONE)?;
        for lane in 8..12 {
            stepped(channel, &mut models.translation, base_bone[lane], predicted[lane], &mut bone[lane])?;
        }
    }
    Ok(())
}

/// The game camera: its position and field of view predicted on from the two samples before,
/// its rotation as a quaternion.
fn code_camera(channel: &mut impl Channel, camera: &mut Vec<i32>, tick: u32, base: Option<&(u32, Arc<Sample>)>, before: Option<&(u32, Arc<Sample>)>, models: &mut Models) -> DecodeResult<()> {
    let (count, same) = code_length(channel, camera.len(), base.map(|(_, base)| base.camera.len()), models, MAX_VALUES)?;
    if channel.decoding() {
        camera.clear();
        camera.resize(count, 0);
    }
    let base = base.filter(|_| same);
    let before = before.filter(|(_, before)| before.camera.len() == count);
    let base_values: Vec<i32> = base.map_or_else(|| vec![0; count], |(_, base)| base.camera.clone());
    let predicted: Vec<i32> =
        (0..count).map(|index| base.map_or(0, |(base_tick, base)| moved(base.camera[index], *base_tick, before.map(|(tick, before)| (before.camera[index], *tick)), tick))).collect();
    if count == CAMERA_VALUES {
        for index in [0, 1, 2, 7] {
            stepped(channel, &mut models.camera, base_values[index], predicted[index], &mut camera[index])?;
        }
        quaternion(channel, &mut camera[3..7], &predicted[3..7], &base_values[3..7], &mut models.camera_rotation, &mut models.camera_largest, CAMERA_ROTATION_ONE)
    } else {
        for (index, value) in camera.iter_mut().enumerate() {
            stepped(channel, &mut models.camera, base_values[index], predicted[index], value)?;
        }
        Ok(())
    }
}

/// The values a sample adds.
fn code_sample(channel: &mut impl Channel, sample: &mut Sample, tick: u32, chain: &Chain, models: &mut Models) -> DecodeResult<()> {
    let base = chain.sample.as_ref();
    let before = chain.sample_before.as_ref().filter(|_| base.is_some());
    code_pose(channel, &mut sample.pose, tick, base, before, models)?;
    // The twist ratios and the audio floats are predicted to stay; how far they moved between
    // the two samples before picks their contexts.
    let twist = base.map_or([0; 4], |(_, base)| base.twist);
    for (index, value) in sample.twist.iter_mut().enumerate() {
        let earlier = before.map_or(twist[index], |(_, before)| before.twist[index]);
        stepped(channel, &mut models.twist, earlier, twist[index], value)?;
    }
    let (count, same) = code_length(channel, sample.audio_floats.len(), base.map(|(_, base)| base.audio_floats.len()), models, MAX_VALUES)?;
    if channel.decoding() {
        sample.audio_floats.clear();
        sample.audio_floats.resize(count, 0);
    }
    let floats = base.filter(|_| same).map(|(_, base)| base.audio_floats.as_slice());
    let earlier = before.map(|(_, before)| before.audio_floats.as_slice()).filter(|earlier| earlier.len() == count);
    for (index, value) in sample.audio_floats.iter_mut().enumerate() {
        let predicted = floats.map_or(0, |floats| floats[index]);
        stepped(channel, &mut models.audio_floats, earlier.map_or(predicted, |earlier| earlier[index]), predicted, value)?;
    }
    code_camera(channel, &mut sample.camera, tick, base, before, models)
}

/// A run's ticks in order, each coded against the chain before it.
fn code_run(channel: &mut impl Channel, frames: &mut [QuantFrame], baselines: &Baselines<'_>) -> DecodeResult<()> {
    let mut models = Box::<Models>::default();
    let mut chain = Chain::new(baselines);
    for index in 0..frames.len() {
        if index > 0 {
            let previous = frames[index - 1].tick;
            let mut gap = u64::from(frames[index].tick.wrapping_sub(previous).wrapping_sub(1));
            channel.unsigned(&mut models.tick_gap, &mut gap)?;
            let gap = u32::try_from(gap).map_err(|_| DecodeError("tick gap"))?;
            frames[index].tick = previous.wrapping_add(gap).wrapping_add(1);
        }
        let frame = &mut frames[index];
        code_header(channel, frame, &chain, &mut models)?;
        let mut sampled = frame.sample.is_some();
        channel.bit(&mut models.sample, &mut sampled);
        frame.sample = if sampled {
            let mut sample = frame.sample.as_deref().cloned().unwrap_or_default();
            code_sample(channel, &mut sample, frame.tick, &chain, &mut models)?;
            Some(Arc::new(sample))
        } else {
            None
        };
        chain.push(frame);
    }
    Ok(())
}

/// The acknowledged frames a run is coded against.
#[derive(Clone, Copy, Default)]
pub struct Baselines<'a> {
    /// The newest, and the one before it.
    pub frame: Option<&'a QuantFrame>,
    pub frame_before: Option<&'a QuantFrame>,
    /// The newest sample, and the sample before it.
    pub sample: Option<&'a QuantFrame>,
    pub sample_before: Option<&'a QuantFrame>,
}

/// `run`, a sender's ticks oldest first, coded against `baselines`; `None` for a run of no
/// frames, more than [`MAX_RUN`], ticks out of order or lists too long to code.
#[must_use]
pub fn encode(run: &[QuantFrame], baselines: &Baselines<'_>) -> Option<EncodedRun> {
    let first = run.first()?.tick;
    let count = u8::try_from(run.len()).ok().filter(|&count| usize::from(count) <= MAX_RUN)?;
    if !run.windows(2).all(|pair| tick_before(pair[0].tick, pair[1].tick)) {
        return None;
    }
    let mut frames = run.to_vec();
    let mut encoder = RangeEncoder::new();
    code_run(&mut encoder, &mut frames, baselines).ok()?;
    Some(EncodedRun {
        tick: first,
        count,
        frame: baselines.frame.map(|frame| frame.tick),
        frame_before: baselines.frame_before.map(|frame| frame.tick),
        sample: baselines.sample.map(|frame| frame.tick),
        sample_before: baselines.sample_before.map(|frame| frame.tick),
        payload: encoder.finish(),
    })
}

/// The run `encoded` from its coding against `baselines`.
///
/// # Errors
/// A payload that is malformed or was coded against frames of another shape.
pub fn decode(encoded: &EncodedRun, baselines: &Baselines<'_>) -> DecodeResult<Vec<QuantFrame>> {
    let count = usize::from(encoded.count);
    if count == 0 || count > MAX_RUN {
        return Err(DecodeError("run length"));
    }
    let mut frames = vec![QuantFrame::empty(); count];
    frames[0].tick = encoded.tick;
    code_run(&mut RangeDecoder::new(&encoded.payload), &mut frames, baselines)?;
    Ok(frames)
}

/// A frame written whole and unquantized, for measuring the coding offline.
#[must_use]
pub fn dump_record(frame: &NetFrame) -> Vec<u8> {
    let mut w = crate::codec::Writer::new();
    w.u32(frame.tick);
    for &word in &frame.header {
        w.u32(word);
    }
    w.bool(frame.pose.is_some());
    if let Some(pose) = &frame.pose {
        w.u32(u32::try_from(pose.len()).unwrap_or(0));
        for bone in pose {
            for &lane in bone {
                w.f32(lane);
            }
        }
    }
    for &value in &frame.twist {
        w.f32(value);
    }
    for list in [&frame.camera, &frame.audio_floats] {
        w.u32(u32::try_from(list.len()).unwrap_or(0));
        for &value in list.iter() {
            w.f32(value);
        }
    }
    for list in [&frame.audio_words, &frame.hud] {
        w.u32(u32::try_from(list.len()).unwrap_or(0));
        for &word in list.iter() {
            w.u32(word);
        }
    }
    w.into_vec()
}

/// A [`dump_record`] read back.
///
/// # Errors
/// The record is malformed.
pub fn read_dump_record(data: &[u8]) -> DecodeResult<NetFrame> {
    let mut r = crate::codec::Reader::new(data);
    let tick = r.u32()?;
    let mut header = [0; HEADER_WORDS];
    for word in &mut header {
        *word = r.u32()?;
    }
    let pose = if r.bool()? {
        let count = usize::try_from(r.u32()?).map_err(|_| DecodeError("bones"))?.min(MAX_BONES);
        let mut pose = Vec::with_capacity(count);
        for _ in 0..count {
            let mut bone = [0.0; BONE_LANES];
            for lane in &mut bone {
                *lane = r.f32()?;
            }
            pose.push(bone);
        }
        Some(pose)
    } else {
        None
    };
    let mut twist = [0.0; 4];
    for value in &mut twist {
        *value = r.f32()?;
    }
    let mut lists: [Vec<f32>; 2] = [Vec::new(), Vec::new()];
    for list in &mut lists {
        let count = usize::try_from(r.u32()?).map_err(|_| DecodeError("values"))?.min(MAX_VALUES);
        for _ in 0..count {
            list.push(r.f32()?);
        }
    }
    let count = usize::try_from(r.u32()?).map_err(|_| DecodeError("words"))?.min(MAX_VALUES);
    let audio_words = (0..count).map(|_| r.u32()).collect::<DecodeResult<Vec<u32>>>()?;
    // Dumps written before the trick HUD record end here.
    let hud = if r.is_empty() {
        Vec::new()
    } else {
        let count = usize::try_from(r.u32()?).map_err(|_| DecodeError("hud words"))?.min(MAX_HUD_WORDS);
        (0..count).map(|_| r.u32()).collect::<DecodeResult<Vec<u32>>>()?
    };
    let [camera, audio_floats] = lists;
    Ok(NetFrame { tick, header, pose, twist, camera, audio_floats, audio_words, hud })
}

/// Whether tick `a` precedes `b`, across the counter's wrap.
#[must_use]
pub fn tick_before(a: u32, b: u32) -> bool {
    b.wrapping_sub(a).wrapping_sub(1) < (1 << 31)
}

/// The last [`HISTORY`] frames of one sender, oldest first.
#[derive(Clone, Debug, Default)]
pub struct FrameRing {
    frames: VecDeque<QuantFrame>,
}

impl FrameRing {
    #[must_use]
    pub fn get(&self, tick: u32) -> Option<&QuantFrame> {
        self.frames.iter().rev().find(|frame| frame.tick == tick)
    }

    #[must_use]
    pub fn latest(&self) -> Option<&QuantFrame> {
        self.frames.back()
    }

    /// Keeps `frame` in tick order; false for a tick already held.
    pub fn insert(&mut self, frame: QuantFrame) -> bool {
        if self.frames.iter().any(|held| held.tick == frame.tick) {
            return false;
        }
        let position = self.frames.iter().rposition(|held| tick_before(held.tick, frame.tick)).map_or(0, |index| index + 1);
        self.frames.insert(position, frame);
        while self.frames.len() > HISTORY {
            self.frames.pop_front();
        }
        true
    }

    /// The newest frame held and a bit per each of the 32 ticks before it (bit `i` for tick
    /// `latest - 1 - i`).
    #[must_use]
    pub fn ack(&self) -> Option<(u32, u32)> {
        let latest = self.latest()?.tick;
        let mut mask = 0_u32;
        for frame in &self.frames {
            let behind = latest.wrapping_sub(frame.tick);
            if (1..=32).contains(&behind) {
                mask |= 1 << (behind - 1);
            }
        }
        Some((latest, mask))
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

/// Which of the sender's ticks the receiver reported holding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AckState {
    latest: Option<u32>,
    mask: u32,
}

impl AckState {
    /// Folds in an ack; an older one than already seen only adds its bits.
    pub fn update(&mut self, latest: u32, mask: u32) {
        match self.latest {
            Some(current) if tick_before(latest, current) => {
                let shift = current.wrapping_sub(latest);
                if shift <= 32 {
                    self.mask |= 1 << (shift - 1);
                    if shift < 32 {
                        self.mask |= mask << shift;
                    }
                }
            }
            Some(current) if current == latest => self.mask |= mask,
            Some(current) => {
                let shift = latest.wrapping_sub(current);
                let carried = if shift >= 32 { 0 } else { (self.mask << shift) | (1 << (shift - 1)) };
                self.latest = Some(latest);
                self.mask = mask | carried;
            }
            None => {
                self.latest = Some(latest);
                self.mask = mask;
            }
        }
    }

    #[must_use]
    pub fn holds(&self, tick: u32) -> bool {
        let Some(latest) = self.latest else { return false };
        let behind = latest.wrapping_sub(tick);
        behind == 0 || ((1..=32).contains(&behind) && self.mask & (1 << (behind - 1)) != 0)
    }

    /// The acknowledged frames of `ring` before tick `first` a run from it is coded against: the
    /// newest, and the newest at least [`FRAME_SPAN`] before it; the newest sample, and the newest
    /// at least [`SAMPLE_SPAN`] before that.
    #[must_use]
    pub fn baselines<'a>(&self, ring: &'a FrameRing, first: u32) -> Baselines<'a> {
        let mut baselines = Baselines::default();
        let spans = |newer: Option<&QuantFrame>, tick: u32, span: u32| newer.is_some_and(|newer| newer.tick.wrapping_sub(tick) >= span);
        for frame in ring.frames.iter().rev() {
            if !tick_before(frame.tick, first) || !self.holds(frame.tick) {
                continue;
            }
            if baselines.frame.is_none() {
                baselines.frame = Some(frame);
            } else if baselines.frame_before.is_none() && spans(baselines.frame, frame.tick, FRAME_SPAN) {
                baselines.frame_before = Some(frame);
            }
            if frame.sample.is_some() {
                if baselines.sample.is_none() {
                    baselines.sample = Some(frame);
                } else if baselines.sample_before.is_none() && spans(baselines.sample, frame.tick, SAMPLE_SPAN) {
                    baselines.sample_before = Some(frame);
                }
            }
            if baselines.frame_before.is_some() && baselines.sample_before.is_some() {
                break;
            }
        }
        baselines
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The sending side of one skater's frames.
#[derive(Clone, Debug, Default)]
pub struct FrameSender {
    ring: FrameRing,
    acks: AckState,
}

impl FrameSender {
    /// Quantizes a run of ticks and codes it against what the receiver holds; `None` for a run
    /// [`encode`] cannot code.
    pub fn encode(&mut self, frames: &[NetFrame]) -> Option<EncodedRun> {
        let run: Vec<QuantFrame> = frames.iter().map(QuantFrame::quantize).collect();
        let encoded = encode_for(&run, &self.ring, &self.acks)?;
        for frame in run {
            self.ring.insert(frame);
        }
        Some(encoded)
    }

    pub fn acknowledge(&mut self, latest: u32, mask: u32) {
        self.acks.update(latest, mask);
    }

    pub fn reset(&mut self) {
        self.ring.clear();
        self.acks.reset();
    }
}

/// `run` coded for a receiver whose acks are `acks`, against the frames of `ring`.
#[must_use]
pub fn encode_for(run: &[QuantFrame], ring: &FrameRing, acks: &AckState) -> Option<EncodedRun> {
    encode(run, &acks.baselines(ring, run.first()?.tick))
}

/// The receiving side of one skater's frames.
#[derive(Clone, Debug, Default)]
pub struct FrameReceiver {
    ring: FrameRing,
    /// The newest sample decoded, whose twist and audio floats ticks without one carry.
    last_sample: Option<(u32, Arc<Sample>)>,
}

impl FrameReceiver {
    /// Decodes a run, keeping its frames as future baselines. `None` for a duplicate or a run
    /// coded against a frame no longer held.
    ///
    /// # Errors
    /// A malformed payload.
    pub fn decode(&mut self, run: &EncodedRun) -> DecodeResult<Option<Vec<QuantFrame>>> {
        if self.ring.get(run.tick).is_some() {
            return Ok(None);
        }
        let lookup = |tick: Option<u32>| -> Result<Option<&QuantFrame>, ()> {
            match tick {
                Some(tick) => self.ring.get(tick).map(Some).ok_or(()),
                None => Ok(None),
            }
        };
        let (Ok(frame), Ok(frame_before), Ok(sample), Ok(sample_before)) = (lookup(run.frame), lookup(run.frame_before), lookup(run.sample), lookup(run.sample_before)) else {
            return Ok(None);
        };
        let decoded = decode(run, &Baselines { frame, frame_before, sample, sample_before })?;
        for frame in &decoded {
            self.ring.insert(frame.clone());
        }
        Ok(Some(decoded))
    }

    /// A decoded frame as the game's values, its missing twist and audio floats carried from the
    /// newest sample decoded.
    pub fn complete(&mut self, frame: &QuantFrame) -> NetFrame {
        if let Some(sample) = &frame.sample
            && self.last_sample.as_ref().is_none_or(|(tick, _)| tick_before(*tick, frame.tick))
        {
            self.last_sample = Some((frame.tick, Arc::clone(sample)));
        }
        let mut header = [0; HEADER_WORDS];
        for (value, &quantized) in frame.root.iter().enumerate() {
            header[ROOT_WORD + value] = dequantize(quantized, root_units(value)).to_bits();
        }
        header[SPEED_WORD] = dequantize(frame.speed, SPEED_UNITS).to_bits();
        for (index, &word) in frame.words.iter().enumerate() {
            header[kept_word(index)] = word;
        }
        let carried = frame.sample.as_ref().or(self.last_sample.as_ref().map(|(_, sample)| sample));
        NetFrame {
            tick: frame.tick,
            header,
            pose: frame
                .sample
                .as_ref()
                .map(|sample| sample.pose.iter().map(|bone| std::array::from_fn(|lane| dequantize(bone[lane], lane_units(lane)))).collect()),
            twist: carried.map_or([0.0; 4], |sample| sample.twist.map(|value| dequantize(value, TWIST_UNITS))),
            camera: frame
                .sample
                .as_ref()
                .map(|sample| sample.camera.iter().enumerate().map(|(index, &value)| dequantize(value, camera_units(index))).collect())
                .unwrap_or_default(),
            audio_floats: carried.map(|sample| sample.audio_floats.iter().map(|&value| dequantize(value, AUDIO_UNITS)).collect()).unwrap_or_default(),
            audio_words: frame.audio_words.clone(),
            hud: frame.hud.clone(),
        }
    }

    #[must_use]
    pub fn ring(&self) -> &FrameRing {
        &self.ring
    }

    #[must_use]
    pub fn ack(&self) -> Option<(u32, u32)> {
        self.ring.ack()
    }

    pub fn reset(&mut self) {
        self.ring.clear();
        self.last_sample = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(clippy::cast_precision_loss, reason = "test data")]
    fn frame(tick: u32, sampled: bool) -> NetFrame {
        let phase = tick as f32 * 0.2;
        let mut header = [0; HEADER_WORDS];
        header[0] = 0x3f80_0000;
        let heading = phase * 0.05;
        header[ROOT_WORD + 1] = (heading.sin()).to_bits();
        header[ROOT_WORD + 3] = (heading.cos()).to_bits();
        header[ROOT_WORD + 4] = (100.0 + phase * 3.0).to_bits();
        header[ROOT_WORD + 6] = (-20.0 + phase).to_bits();
        header[SPEED_WORD] = (4.0 + phase.sin()).to_bits();
        header[48] = if tick % 50 < 3 { 0x0000_1000 } else { 0x0000_c000 };
        NetFrame {
            tick,
            header,
            pose: sampled.then(|| {
                (0..140)
                    .map(|bone| {
                        let angle = phase * 0.01 * (bone % 7) as f32;
                        let mut sqt = [0.0; BONE_LANES];
                        sqt[0..4].copy_from_slice(&[1.0, 1.0, 1.0, 1.0]);
                        sqt[4..8].copy_from_slice(&[angle.sin(), 0.0, 0.0, angle.cos()]);
                        sqt[8..12].copy_from_slice(&[0.1 * bone as f32, 0.25, -0.5, 1.0]);
                        sqt
                    })
                    .collect()
            }),
            twist: [0.5, 0.25, 0.125, 0.0],
            camera: vec![phase, 2.0 * phase, 0.0, 0.0, (phase * 0.05).sin(), 0.0, (phase * 0.05).cos(), 60.0],
            audio_floats: (0..160).map(|i| if i % 3 == 0 { phase * i as f32 } else { 0.0 }).collect(),
            audio_words: (0..40).map(|i| if i == 5 { tick / 10 } else { 7 }).collect(),
            hud: (0..15).map(|i| if i == 7 { tick / 4 } else { u32::from(i == 1) }).chain((tick % 40 < 10).then_some([0x8000_0003, 0x4120_0000]).into_iter().flatten()).collect(),
        }
    }

    /// Runs of `interval` ticks ending in a sample, `ticks` in all.
    fn runs(ticks: u32, interval: u32) -> Vec<Vec<NetFrame>> {
        (0..ticks).collect::<Vec<_>>().chunks(interval as usize).map(|run| run.iter().map(|&tick| frame(tick, tick == *run.last().unwrap())).collect()).collect()
    }

    fn assert_restores(restored: &NetFrame, input: &NetFrame) {
        assert_eq!(restored.header[0], 0, "the hips matrix stays home");
        assert_eq!(restored.header[48], input.header[48]);
        assert!((f32::from_bits(restored.header[ROOT_WORD + 4]) - f32::from_bits(input.header[ROOT_WORD + 4])).abs() < 1e-3);
        assert!((f32::from_bits(restored.header[SPEED_WORD]) - f32::from_bits(input.header[SPEED_WORD])).abs() < 1e-2);
        assert_eq!(restored.audio_words, input.audio_words);
        assert_eq!(restored.hud, input.hud);
        if let (Some(pose), Some(original)) = (&restored.pose, &input.pose) {
            for (bone, original) in pose.iter().zip(original) {
                for lane in 0..BONE_LANES {
                    assert!((bone[lane] - original[lane]).abs() < 1e-3);
                }
            }
            assert_eq!(restored.camera.len(), CAMERA_VALUES);
            for (value, original) in restored.camera.iter().zip(&input.camera) {
                assert!((value - original).abs() < 5e-3);
            }
        } else {
            assert!(restored.camera.is_empty(), "the camera rides on samples");
        }
    }

    #[test]
    fn runs_survive_loss_and_shrink_against_baselines() {
        let mut sender = FrameSender::default();
        let mut receiver = FrameReceiver::default();
        let mut sizes = Vec::new();
        let mut seed = 99_u32;
        let mut decoded_runs = 0;
        for run in runs(480, INTERVAL) {
            let encoded = sender.encode(&run).expect("a run codes");
            sizes.push(encoded.payload.len());
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            if seed % 100 < 20 {
                continue;
            }
            let decoded = receiver.decode(&encoded).unwrap().expect("baselines are acknowledged frames");
            decoded_runs += 1;
            assert_eq!(decoded.len(), run.len());
            for (decoded, input) in decoded.iter().zip(&run) {
                assert_eq!(*decoded, QuantFrame::quantize(input));
                let restored = receiver.complete(decoded);
                assert_restores(&restored, input);
                assert!(!restored.audio_floats.is_empty() || decoded_runs == 1, "carried from the last sample");
            }
            if seed % 100 < 60 {
                let (latest, mask) = receiver.ack().unwrap();
                sender.acknowledge(latest, mask);
            }
        }
        let whole = sizes[0];
        let steady = sizes[80..].iter().sum::<usize>() / (sizes.len() - 80);
        assert!(steady * 4 < whole, "steady {steady} vs whole {whole}");
    }

    #[test]
    fn a_run_may_hold_several_samples_and_skip_ticks() {
        let mut sender = FrameSender::default();
        let mut receiver = FrameReceiver::default();
        for (first, run) in [(0, vec![(0, true)]), (1, vec![(1, false), (2, true), (3, true)]), (6, vec![(6, false), (9, true)])] {
            let frames: Vec<NetFrame> = run.iter().map(|&(tick, sampled)| frame(tick, sampled)).collect();
            let encoded = sender.encode(&frames).unwrap();
            assert_eq!(encoded.tick, first);
            let decoded = receiver.decode(&encoded).unwrap().unwrap();
            assert_eq!(decoded, frames.iter().map(QuantFrame::quantize).collect::<Vec<_>>());
            let (latest, mask) = receiver.ack().unwrap();
            sender.acknowledge(latest, mask);
        }
        assert!(sender.encode(&[frame(12, true), frame(11, true)]).is_none(), "ticks run forward");
        assert!(sender.encode(&[]).is_none());
    }

    #[test]
    fn a_run_against_a_frame_the_receiver_lost_waits_for_none() {
        let mut sender = FrameSender::default();
        let mut receiver = FrameReceiver::default();
        let all = runs(9, INTERVAL);
        let first = sender.encode(&all[0]).unwrap();
        receiver.decode(&first).unwrap().unwrap();
        let (latest, mask) = receiver.ack().unwrap();
        sender.acknowledge(latest, mask);
        // The second run is lost; the third is coded against the first, which the receiver holds.
        let _lost = sender.encode(&all[1]).unwrap();
        let third = sender.encode(&all[2]).unwrap();
        assert_eq!(third.frame, Some(2));
        let decoded = receiver.decode(&third).unwrap().unwrap();
        assert_eq!(decoded.last().unwrap().tick, 8);
        assert!(receiver.decode(&third).unwrap().is_none(), "a duplicate");
    }

    #[test]
    fn rotations_keep_their_sign_and_size() {
        let mut sender = FrameSender::default();
        let mut receiver = FrameReceiver::default();
        let mut input = frame(0, true);
        // A rotation whose largest part is negative, and one near a half turn.
        input.pose.as_mut().unwrap()[3][4..8].copy_from_slice(&[0.1, -0.2, 0.3, -0.927]);
        input.pose.as_mut().unwrap()[4][4..8].copy_from_slice(&[0.999, 0.0, 0.0, 0.0447]);
        let encoded = sender.encode(std::slice::from_ref(&input)).unwrap();
        let decoded = receiver.decode(&encoded).unwrap().unwrap();
        assert_eq!(decoded[0], QuantFrame::quantize(&input));
    }

    #[test]
    fn rates_are_taken_over_the_spans() {
        let mut sender = FrameSender::default();
        let mut receiver = FrameReceiver::default();
        // Every tick a sample, two a run.
        for first in (0..40).step_by(2) {
            let encoded = sender.encode(&[frame(first, true), frame(first + 1, true)]).unwrap();
            receiver.decode(&encoded).unwrap().unwrap();
            let (latest, mask) = receiver.ack().unwrap();
            sender.acknowledge(latest, mask);
        }
        let baselines = sender.acks.baselines(&sender.ring, 40);
        let tick = |frame: Option<&QuantFrame>| frame.map(|frame| frame.tick);
        assert_eq!((tick(baselines.frame), tick(baselines.frame_before)), (Some(39), Some(39 - FRAME_SPAN)));
        assert_eq!((tick(baselines.sample), tick(baselines.sample_before)), (Some(39), Some(39 - SAMPLE_SPAN)));
        let encoded = sender.encode(&[frame(40, true), frame(41, true)]).unwrap();
        assert_eq!(receiver.decode(&encoded).unwrap().unwrap(), [frame(40, true), frame(41, true)].iter().map(QuantFrame::quantize).collect::<Vec<_>>());
    }

    #[test]
    fn acks_record_held_ticks() {
        let mut acks = AckState::default();
        acks.update(10, 0b101);
        assert!(acks.holds(10) && acks.holds(9) && !acks.holds(8) && acks.holds(7));
        acks.update(12, 0);
        assert!(acks.holds(12) && !acks.holds(11) && acks.holds(10) && acks.holds(9) && acks.holds(7));
        acks.update(11, 0);
        assert!(acks.holds(11));
        acks.update(60, 0);
        assert!(acks.holds(60) && !acks.holds(12));
    }

    #[test]
    fn ticks_order_across_the_wrap() {
        assert!(tick_before(1, 2));
        assert!(!tick_before(2, 2));
        assert!(tick_before(u32::MAX, 0));
        let mut ring = FrameRing::default();
        for tick in [5, 3, 4] {
            let mut frame = QuantFrame::empty();
            frame.tick = tick;
            ring.insert(frame);
        }
        assert_eq!(ring.latest().map(|frame| frame.tick), Some(5));
        assert_eq!(ring.ack(), Some((5, 0b11)));
    }
}
