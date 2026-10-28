//! The trick HUD's view of a skater, carried in its replay frames, so a console that follows
//! another player's skater shows that skater's tricks bottom left as it shows its own. The
//! original's `PhysOutModule_Hud::Update` (0x827c79b8) feeds the one `TricksBackEnd` from
//! `SkaterManager::GetLocal()`'s `PhysOut`, and `Online.SwitchCamera` (0x8288a4e0) is
//! `SetLocalIndex`: while a console follows another skater, the HUD shows that skater's tricks.
//! Every original console simulated every skater; here the followed skater's owner sends what its
//! HUD frame ([`PhysicsFrame`]) holds.
//!
//! A record is state, not events: the latest trick begun or converted to as its Vault key (never
//! its name: the receiver builds the name from its own trick database, as the original built it
//! from the skater's; a key, unlike a database index, names the same trick whatever game data
//! each console has), the spins, flips, stance, multiplier, scores and line, the
//! context entries, and a wrapping count of each kind of HUD event (a trick begun, a sequence
//! begun or completed, a landing, an interrupting wipeout, a wipeout, scoring reactivated, an air
//! finished). Frames are not resent, so a lost tick only delays what the next one shows; no event
//! is lost or shown twice. Every value is checked on receipt: a malformed record is dropped as a
//! lost one, and a trick the receiver's database does not hold shows no name.

use crate::ui::trick_backend::{PhysicsFrame, TrickName};
use crate::ui::trick_context::Entry;

/// The record's words before its context entries, two words each.
const FIXED_WORDS: usize = 16;
/// The most context entries a record carries: the 15 big-trick thresholds and as many air
/// "over" entities again, more than the display's three of each kind take.
const MAX_CONTEXTS: usize = 24;
/// The most words of a record; `skate2_net::frames::MAX_HUD_WORDS` holds it.
pub const MAX_WORDS: usize = FIXED_WORDS + 2 * MAX_CONTEXTS;
const _: () = assert!(MAX_WORDS <= skate2_net::frames::MAX_HUD_WORDS);

mod word {
    /// The trick's key, high word then low.
    pub const TRICK: usize = 0;
    pub const COUNTS: usize = 2;
    pub const MORE_COUNTS: usize = 3;
    pub const FLAGS: usize = 4;
    pub const SPINS: usize = 5;
    pub const FLIPS: usize = 6;
    pub const MULTIPLIER: usize = 7;
    pub const HUD_SCORE: usize = 8;
    pub const LINE_SCORE: usize = 9;
    pub const LINE_SECONDS: usize = 10;
    pub const LINE_POINTS: usize = 11;
    pub const FINISHED_SCORE: usize = 12;
    pub const FINISHED_COUNTS: usize = 13;
    pub const LANDED_HEIGHT: usize = 14;
    pub const LANDED_DROP: usize = 15;
}

mod flag {
    pub const CONVERTED: u32 = 1 << 0;
    pub const AERIAL: u32 = 1 << 1;
    pub const SWITCHED: u32 = 1 << 2;
    pub const FAKIE: u32 = 1 << 3;
    pub const JUST_LANDED: u32 = 1 << 4;
    pub const SKETCHY: u32 = 1 << 5;
    pub const ALL: u32 = (1 << 6) - 1;
}

/// A context entry's first word: its kind in the top bits.
const BIG: u32 = 0x8000_0000;
const ENTITY: u32 = 0x4000_0000;
/// `BigTrickUpdate`'s thresholds, and each one's two achievement levels.
const BIG_TYPES: i32 = 15;
const ACHIEVEMENTS: i32 = 2;
/// The entity types the context display names (`entity_label`'s, the DLC ones included).
const ENTITY_TYPES: i32 = 68;

/// Bounds no real skater reaches. The sender holds its values to them, so only a malformed or
/// forged record falls outside.
const MAX_SPINS: i32 = 64;
const MAX_FLIPS: i32 = 16;
const MAX_MULTIPLIER: f32 = 16.0;
const MAX_SECONDS: f32 = 3600.0;
const MAX_POINTS: i32 = 10_000_000;
const MAX_DISTANCE: f32 = 100_000.0;
const MAX_MEASUREMENT: f32 = 1_000_000.0;

/// How many of each kind of HUD event the skater's scoring has raised, wrapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    trick: u8,
    sequence_begun: u8,
    sequence_completed: u8,
    landed: u8,
    interrupted: u8,
    wiped_out: u8,
    reactivated: u8,
    air_finished: u8,
}

/// What the trick HUD shows of a skater at one of its ticks.
#[derive(Clone, Debug, PartialEq)]
pub struct HudRecord {
    /// The latest trick begun or converted to: its Vault key, 0 for one the scoring found no valid
    /// trick for (shown as the original shows such a trick: the air's name).
    trick: u64,
    converted: bool,
    counts: Counts,
    aerial: bool,
    switched: bool,
    fakie: bool,
    spins: i32,
    flips: i32,
    multiplier: f32,
    hud_score: u32,
    line_score: u32,
    line_seconds: f32,
    line_points: i32,
    /// The latest completed sequence.
    finished_score: u32,
    finished_tricks: u16,
    finished_taps: u16,
    /// The latest finished air.
    just_landed: bool,
    sketchy: bool,
    landed_height: f32,
    landed_drop: f32,
    contexts: Vec<Entry>,
}

impl Default for HudRecord {
    fn default() -> Self {
        Self {
            trick: 0,
            converted: false,
            counts: Counts::default(),
            aerial: false,
            switched: false,
            fakie: false,
            spins: 0,
            flips: 0,
            multiplier: 0.0,
            hud_score: 0,
            line_score: 0,
            line_seconds: 0.0,
            line_points: 0,
            finished_score: 0,
            finished_tricks: 0,
            finished_taps: 0,
            just_landed: false,
            sketchy: false,
            landed_height: 0.0,
            landed_drop: 0.0,
            contexts: Vec::new(),
        }
    }
}

const fn word_of(value: i32) -> u32 {
    u32::from_ne_bytes(value.to_ne_bytes())
}

const fn int_of(word: u32) -> i32 {
    i32::from_ne_bytes(word.to_ne_bytes())
}

/// `value` within `±limit`; zero for a value that is not finite.
fn held(value: f32, limit: f32) -> f32 {
    if value.is_finite() { value.clamp(-limit, limit) } else { 0.0 }
}

fn within(value: f32, limit: f32) -> bool {
    value.is_finite() && value.abs() <= limit
}

/// A context entry in the shape its producer (`try_scoring_hud_frame`) makes, as two words.
fn context_words(entry: &Entry) -> Option<[u32; 2]> {
    match *entry {
        Entry::Big { index, achievement, measurement } if (0..BIG_TYPES).contains(&index) && (0..ACHIEVEMENTS).contains(&achievement) => {
            Some([BIG | (word_of(achievement) << 8) | word_of(index), held(measurement, MAX_MEASUREMENT).to_bits()])
        }
        Entry::Entity { entity_type, is_entity: true, count: 1, context_type: 0 } if (0..ENTITY_TYPES).contains(&entity_type) => {
            Some([ENTITY | word_of(entity_type), 0])
        }
        _ => None,
    }
}

fn context_entry(words: [u32; 2]) -> Option<Entry> {
    let [kind, value] = words;
    match kind & (BIG | ENTITY) {
        BIG if kind & !(BIG | 0xff00 | 0xff) == 0 => {
            let index = int_of(kind & 0xff);
            let achievement = int_of((kind >> 8) & 0xff);
            let measurement = f32::from_bits(value);
            ((0..BIG_TYPES).contains(&index) && (0..ACHIEVEMENTS).contains(&achievement) && within(measurement, MAX_MEASUREMENT))
                .then_some(Entry::Big { index, achievement, measurement })
        }
        ENTITY if value == 0 => {
            let entity_type = int_of(kind & !ENTITY);
            (0..ENTITY_TYPES).contains(&entity_type).then_some(Entry::Entity { entity_type, is_entity: true, count: 1, context_type: 0 })
        }
        _ => None,
    }
}

impl HudRecord {
    /// The record as frame words.
    #[must_use]
    pub fn words(&self) -> Vec<u32> {
        let mut words = vec![0; FIXED_WORDS];
        let c = self.counts;
        let [high, low] = [self.trick >> 32, self.trick & 0xffff_ffff].map(|half| u32::try_from(half).unwrap_or(0));
        words[word::TRICK] = high;
        words[word::TRICK + 1] = low;
        words[word::COUNTS] = u32::from_be_bytes([c.trick, c.sequence_begun, c.sequence_completed, c.landed]);
        words[word::MORE_COUNTS] = u32::from_be_bytes([c.interrupted, c.wiped_out, c.reactivated, c.air_finished]);
        let flags = [
            (self.converted, flag::CONVERTED),
            (self.aerial, flag::AERIAL),
            (self.switched, flag::SWITCHED),
            (self.fakie, flag::FAKIE),
            (self.just_landed, flag::JUST_LANDED),
            (self.sketchy, flag::SKETCHY),
        ];
        words[word::FLAGS] = flags.iter().filter(|(set, _)| *set).fold(0, |all, (_, bit)| all | bit);
        words[word::SPINS] = word_of(self.spins);
        words[word::FLIPS] = word_of(self.flips);
        words[word::MULTIPLIER] = self.multiplier.to_bits();
        words[word::HUD_SCORE] = self.hud_score;
        words[word::LINE_SCORE] = self.line_score;
        words[word::LINE_SECONDS] = self.line_seconds.to_bits();
        words[word::LINE_POINTS] = word_of(self.line_points);
        words[word::FINISHED_SCORE] = self.finished_score;
        words[word::FINISHED_COUNTS] = (u32::from(self.finished_tricks) << 16) | u32::from(self.finished_taps);
        words[word::LANDED_HEIGHT] = self.landed_height.to_bits();
        words[word::LANDED_DROP] = self.landed_drop.to_bits();
        for entry in &self.contexts {
            words.extend(context_words(entry).into_iter().flatten());
        }
        words
    }

    /// A received record, `None` for one that is malformed (dropped as a lost tick is). A trick
    /// key is not checked here: one the database does not hold shows no name.
    #[must_use]
    pub fn from_words(words: &[u32]) -> Option<Self> {
        if words.len() < FIXED_WORDS || words.len() > MAX_WORDS || (words.len() - FIXED_WORDS) % 2 != 0 {
            return None;
        }
        let flags = words[word::FLAGS];
        if flags & !flag::ALL != 0 {
            return None;
        }
        let [trick, sequence_begun, sequence_completed, landed] = words[word::COUNTS].to_be_bytes();
        let [interrupted, wiped_out, reactivated, air_finished] = words[word::MORE_COUNTS].to_be_bytes();
        let finished_tricks = u16::try_from(words[word::FINISHED_COUNTS] >> 16).ok()?;
        let finished_taps = u16::try_from(words[word::FINISHED_COUNTS] & 0xffff).ok()?;
        let record = Self {
            trick: (u64::from(words[word::TRICK]) << 32) | u64::from(words[word::TRICK + 1]),
            converted: flags & flag::CONVERTED != 0,
            counts: Counts { trick, sequence_begun, sequence_completed, landed, interrupted, wiped_out, reactivated, air_finished },
            aerial: flags & flag::AERIAL != 0,
            switched: flags & flag::SWITCHED != 0,
            fakie: flags & flag::FAKIE != 0,
            spins: int_of(words[word::SPINS]),
            flips: int_of(words[word::FLIPS]),
            multiplier: f32::from_bits(words[word::MULTIPLIER]),
            hud_score: words[word::HUD_SCORE],
            line_score: words[word::LINE_SCORE],
            line_seconds: f32::from_bits(words[word::LINE_SECONDS]),
            line_points: int_of(words[word::LINE_POINTS]),
            finished_score: words[word::FINISHED_SCORE],
            finished_tricks,
            finished_taps,
            just_landed: flags & flag::JUST_LANDED != 0,
            sketchy: flags & flag::SKETCHY != 0,
            landed_height: f32::from_bits(words[word::LANDED_HEIGHT]),
            landed_drop: f32::from_bits(words[word::LANDED_DROP]),
            contexts: words[FIXED_WORDS..].chunks_exact(2).map(|pair| context_entry([pair[0], pair[1]])).collect::<Option<Vec<_>>>()?,
        };
        let valid = record.spins.unsigned_abs() <= MAX_SPINS.unsigned_abs()
            && record.flips.unsigned_abs() <= MAX_FLIPS.unsigned_abs()
            && record.multiplier.is_finite()
            && (0.0..=MAX_MULTIPLIER).contains(&record.multiplier)
            && within(record.line_seconds, MAX_SECONDS)
            && record.line_points.unsigned_abs() <= MAX_POINTS.unsigned_abs()
            // `TricksBackend::consume` refuses a sequence with more taps than tricks.
            && record.finished_taps <= record.finished_tricks
            && within(record.landed_height, MAX_DISTANCE)
            && within(record.landed_drop, MAX_DISTANCE);
        valid.then_some(record)
    }
}

/// The sending side: the local skater's HUD frames, tick by tick, kept as a record.
#[derive(Debug, Default)]
pub struct HudRecorder {
    record: HudRecord,
}

impl HudRecorder {
    /// Takes one tick's HUD frame of the console's own skater.
    pub fn record(&mut self, frame: &PhysicsFrame) {
        let r = &mut self.record;
        let c = &mut r.counts;
        // The frame's converted trick only counts when none began (`try_scoring_hud_frame`).
        let trick = frame.begun.as_ref().map(|trick| (trick, false)).or_else(|| frame.converted.as_ref().map(|trick| (trick, true)));
        if let Some((trick, converted)) = trick {
            r.trick = if trick.valid { trick.attrib_key } else { 0 };
            r.converted = converted;
            c.trick = c.trick.wrapping_add(1);
        }
        if frame.sequence_begun {
            c.sequence_begun = c.sequence_begun.wrapping_add(1);
        }
        if frame.sequence_completed {
            c.sequence_completed = c.sequence_completed.wrapping_add(1);
            r.finished_score = frame.finished_sequence_score;
            r.finished_tricks = u16::try_from(frame.finished_tricks).unwrap_or(u16::MAX);
            r.finished_taps = u16::try_from(frame.finished_taps).unwrap_or(u16::MAX).min(r.finished_tricks);
        }
        if frame.ended_count > 0 {
            c.landed = c.landed.wrapping_add(1);
        }
        if frame.interrupted {
            c.interrupted = c.interrupted.wrapping_add(1);
        }
        if frame.just_wiped_out {
            c.wiped_out = c.wiped_out.wrapping_add(1);
        }
        if frame.reactivated {
            c.reactivated = c.reactivated.wrapping_add(1);
        }
        if frame.air_finished {
            c.air_finished = c.air_finished.wrapping_add(1);
            r.just_landed = frame.just_landed;
            r.sketchy = frame.sketchy;
            r.landed_height = held(frame.landed_height, MAX_DISTANCE);
            r.landed_drop = held(frame.landed_drop, MAX_DISTANCE);
        }
        r.aerial = frame.aerial;
        r.switched = frame.switched;
        r.fakie = frame.fakie;
        r.spins = frame.spins.clamp(-MAX_SPINS, MAX_SPINS);
        r.flips = frame.flips.clamp(-MAX_FLIPS, MAX_FLIPS);
        r.multiplier = held(frame.multiplier, MAX_MULTIPLIER).max(0.0);
        r.hud_score = frame.hud_score;
        r.line_score = frame.line_score;
        r.line_seconds = held(frame.line_seconds, MAX_SECONDS);
        r.line_points = frame.line_points.clamp(-MAX_POINTS, MAX_POINTS);
        r.contexts = frame.context_entries.iter().filter(|entry| context_words(entry).is_some()).take(MAX_CONTEXTS).cloned().collect();
    }

    /// The record of the latest tick, as frame words.
    #[must_use]
    pub fn words(&self) -> Vec<u32> {
        self.record.words()
    }
}

/// The HUD events between two records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Events {
    trick: bool,
    sequence_begun: bool,
    sequence_completed: bool,
    landed: bool,
    interrupted: bool,
    wiped_out: bool,
    reactivated: bool,
    air_finished: bool,
}

impl Events {
    fn between(before: Counts, after: Counts) -> Self {
        Self {
            trick: before.trick != after.trick,
            sequence_begun: before.sequence_begun != after.sequence_begun,
            sequence_completed: before.sequence_completed != after.sequence_completed,
            landed: before.landed != after.landed,
            interrupted: before.interrupted != after.interrupted,
            wiped_out: before.wiped_out != after.wiped_out,
            reactivated: before.reactivated != after.reactivated,
            air_finished: before.air_finished != after.air_finished,
        }
    }

    /// What ends a trick, a sequence or a wipeout.
    const fn ends(self) -> bool {
        self.sequence_completed || self.landed || self.interrupted || self.wiped_out || self.reactivated || self.air_finished
    }

    /// What starts a trick or a sequence.
    const fn starts(self) -> bool {
        self.trick || self.sequence_begun
    }

    const fn any(self) -> bool {
        self.ends() || self.starts()
    }

    const fn only_ends(self) -> Self {
        Self { trick: false, sequence_begun: false, ..self }
    }

    const fn only_starts(self) -> Self {
        Self { trick: self.trick, sequence_begun: self.sequence_begun, ..Self::none() }
    }

    const fn none() -> Self {
        Self { trick: false, sequence_begun: false, sequence_completed: false, landed: false, interrupted: false, wiped_out: false, reactivated: false, air_finished: false }
    }
}

/// One record a remote skater's playout passed, with the events since the record before it.
#[derive(Clone, Debug)]
pub struct HudStep {
    record: HudRecord,
    events: Events,
    /// Ticks were lost between the two records, so their events' order is unknown.
    after_gap: bool,
}

/// The most steps kept between two local ticks: a playout catching up after a stall passes many
/// ticks at once.
const MAX_STEPS: usize = 600;

/// The receiving side for one remote skater: its records as its playout passes their ticks. The
/// records are followed whether or not the console follows the skater, so following it starts
/// from its state then, never from a burst of the events before.
#[derive(Debug, Default)]
pub struct HudFollower {
    last: Option<(u32, HudRecord)>,
    steps: Vec<HudStep>,
}

impl HudFollower {
    /// The playout passed `tick`, whose frame carried `words`. A malformed record is skipped as a
    /// lost tick is.
    pub fn pass(&mut self, tick: u32, words: &[u32]) {
        let Some(record) = HudRecord::from_words(words) else { return };
        let step = match &self.last {
            Some((last_tick, last)) => HudStep { events: Events::between(last.counts, record.counts), after_gap: tick.wrapping_sub(*last_tick) > 1, record: record.clone() },
            None => HudStep { events: Events::default(), after_gap: false, record: record.clone() },
        };
        self.last = Some((tick, record));
        if self.steps.len() == MAX_STEPS {
            self.steps.remove(0);
        }
        self.steps.push(step);
    }

    /// The steps since the last call, oldest first.
    pub fn take(&mut self) -> Vec<HudStep> {
        std::mem::take(&mut self.steps)
    }

    /// The newest record passed.
    #[must_use]
    pub fn latest(&self) -> Option<&HudRecord> {
        self.last.as_ref().map(|(_, record)| record)
    }

    /// Starts over: the skater's ticks restarted, so its next record is a new baseline.
    pub fn forget(&mut self) {
        *self = Self::default();
    }
}

/// A trick's name from its key: 0 is the scoring's own "no valid trick", built as the original
/// builds it (`try_scoring_hud_frame`'s invalid name); a key the receiver's database does not hold
/// (`name` returns `None`) has no name.
fn trick_name(key: u64, name: &mut impl FnMut(u64) -> Option<TrickName>) -> Option<TrickName> {
    if key == 0 { Some(TrickName { index: -1, ..Default::default() }) } else { name(key) }
}

/// The HUD frame of one step's events, with the record's values.
fn frame(record: &HudRecord, events: Events, name: &mut impl FnMut(u64) -> Option<TrickName>) -> PhysicsFrame {
    let trick = if events.trick { trick_name(record.trick, name) } else { None };
    let (begun, converted) = if record.converted { (None, trick) } else { (trick, None) };
    let air = events.air_finished;
    PhysicsFrame {
        begun,
        converted,
        ended_count: u32::from(events.landed),
        sequence_begun: events.sequence_begun,
        sequence_completed: events.sequence_completed,
        finished_sequence_score: record.finished_score,
        finished_tricks: u32::from(record.finished_tricks),
        finished_taps: u32::from(record.finished_taps),
        interrupted: events.interrupted,
        just_wiped_out: events.wiped_out,
        reactivated: events.reactivated,
        air_finished: air,
        just_landed: air && record.just_landed,
        sketchy: air && record.sketchy,
        landed_height: if air { record.landed_height } else { 0.0 },
        landed_drop: if air { record.landed_drop } else { 0.0 },
        ..held_frame(record)
    }
}

/// A frame of the record's values and no events.
fn held_frame(record: &HudRecord) -> PhysicsFrame {
    PhysicsFrame {
        context_entries: record.contexts.clone(),
        line_seconds: record.line_seconds,
        line_points: record.line_points,
        line_score: record.line_score,
        multiplier: record.multiplier,
        hud_score: record.hud_score,
        aerial: record.aerial,
        spins: record.spins,
        flips: record.flips,
        switched: record.switched,
        fakie: record.fakie,
        ..PhysicsFrame::default()
    }
}

/// The HUD frames of a followed remote skater for one local tick, replacing the local skater's
/// `local` frame: one frame per step with events (two where a gap hides whether a start came
/// before an end: the ends go first, as a skater ends a trick before its next), the newest
/// values on the last, and a frame of the newest values when no step brought an event, so the
/// HUD keeps its once-a-tick pace. The local frame's backend commands (simulation requests the
/// one `TricksBackEnd` takes whichever skater it follows), its simulation rate and its line
/// maximum (a Vault constant) carry over. `name` finds a trick's name in the receiver's database.
///
/// The Hall of Meat HUD and the minimap's bail state stay the local skater's (the records carry
/// no bail), and a reactivation of scoring takes the HoM HUD down and gives the minimap back
/// (`PhysOutModule_Hud::ReactivateScoring` 0x827c7230): the followed skater's, as the original's
/// `GetLocal()` makes it, and the local skater's own at its respawn, so a bail shown before the
/// view moved to the next player never stays up.
pub fn followed_frames(steps: &[HudStep], latest: Option<&HudRecord>, local: PhysicsFrame, mut name: impl FnMut(u64) -> Option<TrickName>) -> Vec<PhysicsFrame> {
    let mut frames = Vec::new();
    for step in steps.iter().filter(|step| step.events.any()) {
        if step.after_gap && step.events.ends() && step.events.starts() {
            frames.push(frame(&step.record, step.events.only_ends(), &mut name));
            frames.push(frame(&step.record, step.events.only_starts(), &mut name));
        } else {
            frames.push(frame(&step.record, step.events, &mut name));
        }
    }
    let newest = steps.last().map(|step| &step.record).or(latest);
    let fallback = HudRecord::default();
    let newest = newest.unwrap_or(&fallback);
    match frames.last_mut() {
        Some(last) => {
            let values = held_frame(newest);
            last.context_entries = values.context_entries;
            last.line_seconds = values.line_seconds;
            last.line_points = values.line_points;
            last.line_score = values.line_score;
            last.multiplier = values.multiplier;
            last.hud_score = values.hud_score;
            last.aerial = values.aerial;
            last.spins = values.spins;
            last.flips = values.flips;
            last.switched = values.switched;
            last.fakie = values.fakie;
        }
        None => frames.push(held_frame(newest)),
    }
    for frame in &mut frames {
        frame.simulation_rate = local.simulation_rate;
        frame.line_max_points = local.line_max_points;
    }
    let reactivated = local.hom.just_reactivated || steps.iter().any(|step| step.events.reactivated);
    if let Some(last) = frames.last_mut() {
        last.hom.just_reactivated = reactivated;
    }
    frames[0].backend_commands = local.backend_commands;
    frames
}

#[cfg(test)]
#[path = "hud_record_tests.rs"]
mod tests;
