//! The facts a player's skater gives an online challenge each tick: what the challenge reads of
//! another player's completed `PhysOutModule_MissionSystem` output ([`MissionFrame`]). In the
//! original every console simulated every skater and so had every output; here each player sends
//! that part of its own, and every console's challenge system runs the challenge on all of them
//! in the same tick order.
//!
//! What the challenge reads depends on its type ([`FactsScope`], from the script handlers,
//! natives and objective conditions of every online challenge of the type: docs/online/facts.md).
//! The rest stays at its default on every console, the owner's included ([`FactsScope::shared`]),
//! so every console runs the challenge on the same values. What only the owner's console reads of
//! its own skater (the objective natives' player state, its pushes, its stance) it reads from its
//! full output.
//!
//! A frame is coded against the player's previous one: the smooth values the scope reads (the
//! trigger tester's points, and the speeds and the bail's durations of the scopes that read them)
//! as the Elias gamma code of their distance, in the order of their float bits, from where the
//! two values before them extrapolate; everything else as the XOR of a fixed layout with the
//! previous tick's, its zero runs skipped. An event keeps its last value while absent, so it
//! changes the layout once. The coding is lossless: every console reads the same values.

use crate::challenge::mission::{ContextUidLists, EndedTrick, FinishedAir, FinishedGrind, FinishedSequence, HomBail, MissionFrame};
use skate2_net::codec::{BitReader, BitWriter, DecodeError, DecodeResult, Reader, Writer};

/// The longest layout accepted.
const MAX_LAYOUT: usize = 8 * 1024;
/// The most entries of a list.
const MAX_LIST: usize = 256;
/// Zero bytes inside a changed run that stay in it rather than split it.
const ZERO_RUN_SPLIT: usize = 3;
/// `eAnimGestureNames::Count`: no gesture.
const GESTURE_NONE: i32 = 35;

/// The groups of a frame one challenge type reads of another player.
mod group {
    /// `cMsgSkaterMoving`, `Skate.GetPlayerSpeed`.
    pub const DECK_SPEED: u32 = 1 << 0;
    /// `Entity.GetSpeed`.
    pub const SURFACE_SPEED: u32 = 1 << 1;
    /// `cMsgWipeoutAlmostOver`.
    pub const WIPEOUT_ALMOST_OVER: u32 = 1 << 3;
    /// `cMsgGrindCompleted`.
    pub const GRIND: u32 = 1 << 4;
    /// `Trick.IsSkaterInAirOrGrinding`.
    pub const GRIND_STATE: u32 = 1 << 5;
    /// `Trick.GetCompletedGrindDistance`.
    pub const GRIND_DISTANCE: u32 = 1 << 6;
    /// `Grind.IsGrinding`, `Grind.IsGrindingGUID`.
    pub const GRINDING: u32 = 1 << 7;
    /// `cMsgAirCompleted`, `Trick.GetCompletedAir*`, the objectives' air records.
    pub const AIR: u32 = 1 << 8;
    /// The objectives' contextual records and skaters aired over.
    pub const AIR_CONTEXTS: u32 = 1 << 9;
    /// The objectives' big-trick records: air thresholds, big tricks, the grind object and the
    /// finished manual and powerslide.
    pub const ACHIEVEMENTS: u32 = 1 << 10;
    /// `Trick.IsInAirWithValidLaunchTrick`.
    pub const AIR_STATE: u32 = 1 << 11;
    /// `Trick.IsSkaterInAir`, `Trick.IsSkaterInAirOrGrinding`.
    pub const AIRBORNE: u32 = 1 << 12;
    /// `cMsgTrickComplete` (the objectives' completed tricks).
    pub const TRICKS: u32 = 1 << 13;
    /// `Trick.IsDoingTrick`.
    pub const TRICK_ACTIVE: u32 = 1 << 14;
    /// `Trick.IsInManual`, `ObjectiveTricks.IsInCoffin`.
    pub const ACTIVE_TRICKS: u32 = 1 << 15;
    /// `cMsgSequenceBegin`, `cMsgSequenceComplete`.
    pub const SEQUENCE: u32 = 1 << 16;
    /// `Trick.IsInSequence`, `Skate.MatchedPattern`.
    pub const IN_SEQUENCE: u32 = 1 << 17;
    /// `Trick.GetCompletedSequenceScore`, the objectives' full completion.
    pub const SEQUENCE_SCORE: u32 = 1 << 18;
    /// `Skate.MatchedPattern`.
    pub const PATTERNS: u32 = 1 << 19;
    /// `Trick.GetNumTrickType`.
    pub const SEQUENCE_TYPES: u32 = 1 << 20;
    /// `cMsgPointsAwarded`.
    pub const POINTS: u32 = 1 << 21;
    /// `Trick.GetHOMScore`, `ObjectiveHOM.GetScore`.
    pub const HOM_SCORE: u32 = 1 << 22;
    /// The other `ObjectiveHOM` getters.
    pub const HOM_BAIL: u32 = 1 << 23;
    /// `cMsgGesturePerformed`.
    pub const GESTURES: u32 = 1 << 24;
    /// `Gesture.IsGesturing`.
    pub const GESTURE_STATE: u32 = 1 << 25;
    /// `cMsgSkaterOnboard`, `cMsgSkaterOffboard`.
    pub const BOARD: u32 = 1 << 26;
    /// `cMsgSkaterGrabbingObject`.
    pub const GRABBING: u32 = 1 << 27;
    /// The rest of the state the objective natives read of their player (`IsOnGround`,
    /// `SequenceCompleted`, `GetWipeoutDuration`, `IsManualing`, `NoseManualTime`,
    /// `TailManualTime`, `IsWipingOut`): a challenge whose objective groups each read their own
    /// player, every player's on every console (freeskate activities), reads it of every player.
    pub const PLAYER_STATE: u32 = 1 << 28;
    /// What a challenge's objective groups read of their player's messages.
    pub const OBJECTIVES: u32 = GRIND | AIR | TRICKS | SEQUENCE | SEQUENCE_SCORE | POINTS | BOARD | GRABBING;
}

/// What the challenges of one type read of another player's facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactsScope(u32);

impl FactsScope {
    /// Everything the facts can carry.
    pub const ALL: Self = Self(u32::MAX);

    /// The scope of a game challenge type (`challenge_global_data.ChallengeType`): S.K.A.T.E. 4,
    /// Best Trick 8, Spot Battle 9, Jam 15, Death Race 19, Hall of Meat 31 and 1-Up 32. Any other
    /// type reads everything.
    #[must_use]
    pub const fn for_challenge_type(challenge_type: i32) -> Self {
        use group::*;
        Self(match challenge_type {
            4 => DECK_SPEED | AIR | AIR_STATE | SEQUENCE | IN_SEQUENCE | SEQUENCE_SCORE | PATTERNS,
            8 | 15 => OBJECTIVES | SURFACE_SPEED | SEQUENCE_TYPES | HOM_SCORE,
            9 => OBJECTIVES | WIPEOUT_ALMOST_OVER,
            19 => OBJECTIVES | GRIND_DISTANCE | TRICK_ACTIVE | AIRBORNE,
            31 => OBJECTIVES | WIPEOUT_ALMOST_OVER | HOM_SCORE,
            32 => SEQUENCE | AIR_STATE | AIRBORNE | GRIND_STATE,
            _ => u32::MAX,
        })
    }

    const fn has(self, group: u32) -> bool {
        self.0 & group != 0
    }

    /// The frame every console runs the challenge on: the groups outside the scope at their
    /// defaults, the root position at the tester's base (only the map icons read another
    /// skater's), no trick names (`cMsgTrickComplete`'s readers take the attribute key), no bail
    /// save image (only the career saves the local skater's) and no vote (votes travel as
    /// [`skate2_net::protocol::FactsNotes`]). The owner's own frame keeps the pushes
    /// `MissionSystem::Update` reads of the local skater alone. Whether a teleport finished is
    /// always kept: while a teleport is unfinished its player sends facts.
    #[must_use]
    pub fn shared(self, frame: &MissionFrame) -> MissionFrame {
        use group::*;
        let mut shared = MissionFrame {
            player: frame.player,
            local: frame.local,
            position: frame.tester_base,
            tester_base: frame.tester_base,
            tester_top: frame.tester_top,
            tester_centre_of_mass: frame.tester_centre_of_mass,
            tester_valid: frame.tester_valid,
            is_teleport_finished: frame.is_teleport_finished,
            physics_requested_teleport: frame.physics_requested_teleport,
            teleporting: frame.teleporting,
            is_regular_stance: frame.is_regular_stance,
            just_wiped_out: frame.just_wiped_out,
            hom_bail_in_progress: frame.hom_bail_in_progress,
            hom_bail_completed: frame.hom_bail_completed,
            current_air_initial_scorable: -1,
            current_gesture: GESTURE_NONE,
            ..MissionFrame::default()
        };
        if frame.local {
            shared.is_pushing = frame.is_pushing;
            shared.is_physically_pushing = frame.is_physically_pushing;
            shared.is_new_auto_pump = frame.is_new_auto_pump;
        }
        if self.has(DECK_SPEED) {
            shared.deck_speed = frame.deck_speed;
        }
        if self.has(SURFACE_SPEED) {
            shared.surface_speed = frame.surface_speed;
        }
        if self.has(WIPEOUT_ALMOST_OVER) {
            shared.is_wipeout_teleport_pending = frame.is_wipeout_teleport_pending;
            shared.time_until_wipeout_teleport = frame.time_until_wipeout_teleport;
        }
        if self.has(GRIND) {
            shared.finished_grind = frame.finished_grind;
        }
        if self.has(GRIND_STATE) {
            shared.is_in_state_grinding = frame.is_in_state_grinding;
        }
        if self.has(GRIND_DISTANCE) {
            shared.finished_grind_length = frame.finished_grind_length;
        }
        if self.has(GRINDING) {
            shared.filtered_grinding = frame.filtered_grinding;
            shared.filtered_grind_pathed_guid = frame.filtered_grind_pathed_guid;
            shared.filtered_grind_local_guid = frame.filtered_grind_local_guid;
        }
        if self.has(AIR) {
            shared.finished_air = frame.finished_air.map(|air| FinishedAir {
                big_trick_thresholds: if self.has(ACHIEVEMENTS) { air.big_trick_thresholds } else { [[false; 2]; 10] },
                initial_context: if self.has(AIR_CONTEXTS) { air.initial_context } else { ContextUidLists::default() },
                end_context: if self.has(AIR_CONTEXTS) { air.end_context } else { ContextUidLists::default() },
                accumulated_context: if self.has(AIR_CONTEXTS) { air.accumulated_context } else { ContextUidLists::default() },
                ..air
            });
        }
        if self.has(ACHIEVEMENTS) {
            shared.big_trick_achieved = frame.big_trick_achieved;
            shared.finished_grind_object = frame.finished_grind_object;
            shared.finished_manual = frame.finished_manual;
            shared.finished_powerslide = frame.finished_powerslide;
        }
        if self.has(AIR_STATE) {
            shared.is_in_state_air = frame.is_in_state_air;
            shared.current_air_initial_scorable = frame.current_air_initial_scorable;
        }
        if self.has(AIRBORNE) {
            shared.skateboard_airborne = frame.skateboard_airborne;
        }
        if self.has(TRICKS) {
            shared.tricks_ended = frame.tricks_ended.iter().map(|trick| EndedTrick { name: String::new(), ..trick.clone() }).collect();
        }
        if self.has(TRICK_ACTIVE) {
            shared.in_a_current_trick = frame.in_a_current_trick;
        }
        if self.has(ACTIVE_TRICKS) {
            shared.active_trick_types.clone_from(&frame.active_trick_types);
            shared.active_tricks.clone_from(&frame.active_tricks);
        }
        if self.has(SEQUENCE) {
            shared.sequence_begun = frame.sequence_begun;
            shared.sequence_completed = frame.sequence_completed;
        }
        if self.has(IN_SEQUENCE) {
            shared.in_sequence = frame.in_sequence;
        }
        if self.has(SEQUENCE_SCORE) {
            shared.finished_sequence_score = frame.finished_sequence_score;
        }
        if self.has(PATTERNS) {
            shared.finished_pattern_matches = frame.finished_pattern_matches;
            shared.current_pattern_matches = frame.current_pattern_matches;
        }
        if self.has(SEQUENCE_TYPES) {
            shared.trick_types_in_completed_sequence = frame.trick_types_in_completed_sequence;
        }
        if self.has(POINTS) {
            shared.banked_point_change = frame.banked_point_change;
        }
        if self.has(HOM_SCORE) {
            shared.hom_score = frame.hom_score;
        }
        if self.has(HOM_BAIL) {
            shared.hom_bail = HomBail { save_image: None, ..frame.hom_bail };
        }
        if self.has(GESTURES) {
            shared.performed_gesture = frame.performed_gesture;
        }
        if self.has(GESTURE_STATE) {
            shared.current_gesture = frame.current_gesture;
        }
        if self.has(BOARD) {
            shared.is_onboard_or_hippy_jumping = frame.is_onboard_or_hippy_jumping;
            shared.was_onboard_or_hippy_jumping = frame.was_onboard_or_hippy_jumping;
            shared.is_offboard_and_not_hippy_jumping = frame.is_offboard_and_not_hippy_jumping;
            shared.was_offboard_and_not_hippy_jumping = frame.was_offboard_and_not_hippy_jumping;
        }
        if self.has(GRABBING) {
            shared.is_grabbing_object = frame.is_grabbing_object;
        }
        if self.has(PLAYER_STATE) {
            shared.sequence_completed_successfully = frame.sequence_completed_successfully;
            shared.is_in_state_ground = frame.is_in_state_ground;
            shared.moving_wipeout_duration = frame.moving_wipeout_duration;
            shared.manual_length = frame.manual_length;
            shared.manual_time = frame.manual_time;
            shared.anim_trick_bits = frame.anim_trick_bits;
            shared.wiping_out = frame.wiping_out;
        }
        shared
    }
}

/// The frame of a tick a player sends no facts for (pinned, watching another player's turn): its
/// last facts' state with nothing new happening in it.
#[must_use]
pub fn standing(frame: &MissionFrame) -> MissionFrame {
    MissionFrame {
        is_pushing: false,
        is_physically_pushing: false,
        is_new_auto_pump: false,
        is_teleport_finished: false,
        physics_requested_teleport: false,
        teleporting: false,
        just_wiped_out: false,
        hom_bail_completed: false,
        finished_grind: None,
        finished_air: None,
        finished_manual: None,
        finished_powerslide: None,
        tricks_ended: Vec::new(),
        performed_gesture: None,
        sequence_begun: false,
        sequence_completed: None,
        banked_point_change: 0,
        was_onboard_or_hippy_jumping: frame.is_onboard_or_hippy_jumping,
        was_offboard_and_not_hippy_jumping: frame.is_offboard_and_not_hippy_jumping,
        vote: None,
        ..frame.clone()
    }
}

/// The smooth values, in coding order.
const SMOOTH: usize = 23;

impl FactsScope {
    /// Whether the scope reads smooth value `index`: the tester's points always, the speeds, the
    /// wipeout teleport's countdown, the bail's values and the manual's and wipeout's durations
    /// with their groups. The others stay at their defaults in the shared frame, and travel not at
    /// all.
    const fn reads_smooth(self, index: usize) -> bool {
        use group::*;
        match index {
            0..=8 => true,
            9 => self.has(DECK_SPEED),
            10 => self.has(SURFACE_SPEED),
            11 => self.has(WIPEOUT_ALMOST_OVER),
            12..=19 => self.has(HOM_BAIL),
            _ => self.has(PLAYER_STATE),
        }
    }
}

fn smooth(frame: &MissionFrame) -> [f32; SMOOTH] {
    let [bx, by, bz] = frame.tester_base;
    let [tx, ty, tz] = frame.tester_top;
    let [cx, cy, cz] = frame.tester_centre_of_mass;
    let bail = &frame.hom_bail;
    [
        bx,
        by,
        bz,
        tx,
        ty,
        tz,
        cx,
        cy,
        cz,
        frame.deck_speed,
        frame.surface_speed,
        frame.time_until_wipeout_teleport,
        bail.overall_duration,
        bail.moving_duration,
        bail.skeleton_over_duration,
        bail.severe_duration,
        bail.rotation,
        bail.accumulated_air_drop_height,
        bail.max_air_drop_height,
        bail.bail_speed,
        frame.manual_length,
        frame.manual_time,
        frame.moving_wipeout_duration,
    ]
}

fn set_smooth(frame: &mut MissionFrame, values: [f32; SMOOTH]) {
    let [bx, by, bz, tx, ty, tz, cx, cy, cz, deck, surface, until, overall, moving, over, severe, rotation, accumulated, max, speed, manual_length, manual_time, wipeout] =
        values;
    frame.manual_length = manual_length;
    frame.manual_time = manual_time;
    frame.moving_wipeout_duration = wipeout;
    frame.tester_base = [bx, by, bz];
    frame.tester_top = [tx, ty, tz];
    frame.tester_centre_of_mass = [cx, cy, cz];
    frame.deck_speed = deck;
    frame.surface_speed = surface;
    frame.time_until_wipeout_teleport = until;
    let bail = &mut frame.hom_bail;
    bail.overall_duration = overall;
    bail.moving_duration = moving;
    bail.skeleton_over_duration = over;
    bail.severe_duration = severe;
    bail.rotation = rotation;
    bail.accumulated_air_drop_height = accumulated;
    bail.max_air_drop_height = max;
    bail.bail_speed = speed;
}

/// A float's bits in value order: negative values below positive ones, each side in order.
const fn ordered(value: f32) -> u32 {
    let bits = value.to_bits();
    if bits >> 31 == 1 { !bits } else { bits | 0x8000_0000 }
}

const fn unordered(value: u32) -> f32 {
    f32::from_bits(if value >> 31 == 1 { value & 0x7fff_ffff } else { !value })
}

/// Another console's float as the challenge reads it: one that is no number (only a tampered game
/// sends one) reads as zero, so no NaN or infinity reaches the challenge.
fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn f32r(r: &mut Reader<'_>) -> DecodeResult<f32> {
    r.f32().map(finite)
}

/// The last two ticks' smooth values, which the next tick's are coded from.
#[derive(Clone, Debug, Default)]
struct History {
    samples: u8,
    last: [u32; SMOOTH],
    before: [u32; SMOOTH],
}

impl History {
    /// The value the tick before extrapolates to: none at first, then the last value, then the
    /// last two values' line.
    fn predict(&self, index: usize) -> i64 {
        let last = i64::from(self.last[index]);
        match self.samples {
            0 => 0,
            1 => last,
            _ => (2 * last - i64::from(self.before[index])).clamp(0, i64::from(u32::MAX)),
        }
    }

    fn push(&mut self, values: [u32; SMOOTH]) {
        self.before = self.last;
        self.last = values;
        self.samples = (self.samples + 1).min(2);
    }
}

/// The events' last values, which they keep in the layout while absent.
#[derive(Clone, Debug, Default)]
struct Sticky {
    grind: FinishedGrind,
    air: FinishedAir,
    manual: f32,
    powerslide: f32,
    gesture: i32,
    sequence: FinishedSequence,
}

/// A player's frames as they are coded, each against the one before.
#[derive(Clone, Debug)]
pub struct FactsCoder {
    scope: FactsScope,
    previous: Vec<u8>,
    history: History,
    sticky: Sticky,
}

impl Default for FactsCoder {
    fn default() -> Self {
        Self::new(FactsScope::ALL)
    }
}

impl FactsCoder {
    /// The coder of a challenge whose scope is `scope`, alike on the sending and receiving side.
    #[must_use]
    pub fn new(scope: FactsScope) -> Self {
        Self { scope, previous: Vec::new(), history: History::default(), sticky: Sticky::default() }
    }

    /// The next tick's frame, coded. The frame is expected in its shared form
    /// ([`FactsScope::shared`]); what that drops is never coded.
    #[must_use]
    pub fn encode(&mut self, frame: &MissionFrame) -> Vec<u8> {
        let values = smooth(frame).map(ordered);
        let mut bits = BitWriter::new();
        for (index, &value) in values.iter().enumerate().filter(|&(index, _)| self.scope.reads_smooth(index)) {
            bits.signed(i64::from(value) - self.history.predict(index));
        }
        self.history.push(values);
        let layout = layout(frame, &mut self.sticky);
        let mut w = Writer::with_capacity(64);
        w.raw(&bits.finish());
        delta(&mut w, &self.previous, &layout);
        self.previous = layout;
        w.into_vec()
    }

    /// The next tick's frame, decoded as player `player`.
    ///
    /// # Errors
    /// The data is not a coded frame.
    pub fn decode(&mut self, data: &[u8], player: i32) -> DecodeResult<MissionFrame> {
        let mut bits = BitReader::new(data);
        let mut values = [ordered(0.0); SMOOTH];
        for (index, value) in values.iter_mut().enumerate().filter(|&(index, _)| self.scope.reads_smooth(index)) {
            let decoded = self.history.predict(index) + bits.signed()?;
            *value = u32::try_from(decoded).map_err(|_| DecodeError("facts value out of range"))?;
        }
        let mut r = Reader::new(&data[bits.byte_len()..]);
        let layout = undelta(&self.previous, &mut r)?;
        let mut frame = read_layout(&layout)?;
        set_smooth(&mut frame, values.map(|value| finite(unordered(value))));
        frame.position = frame.tester_base;
        frame.player = player;
        frame.local = false;
        self.history.push(values);
        self.previous = layout;
        Ok(frame)
    }
}

/// The layout's length change, then each changed run after the zero run before it.
fn delta(w: &mut Writer, previous: &[u8], current: &[u8]) {
    w.signed(i64::try_from(current.len()).unwrap_or(0) - i64::try_from(previous.len()).unwrap_or(0));
    let xor: Vec<u8> = current.iter().enumerate().map(|(i, byte)| byte ^ previous.get(i).copied().unwrap_or(0)).collect();
    let mut at = 0;
    while at < xor.len() {
        let zeros = xor[at..].iter().take_while(|&&byte| byte == 0).count();
        let start = at + zeros;
        if start >= xor.len() {
            break;
        }
        let mut end = start;
        while end < xor.len() {
            if xor[end] != 0 {
                end += 1;
                continue;
            }
            let gap = xor[end..].iter().take_while(|&&byte| byte == 0).count();
            if gap >= ZERO_RUN_SPLIT || end + gap >= xor.len() {
                break;
            }
            end += gap;
        }
        w.varint(zeros as u64);
        w.bytes(&xor[start..end]);
        at = end;
    }
}

fn undelta(previous: &[u8], r: &mut Reader<'_>) -> DecodeResult<Vec<u8>> {
    let length = i64::try_from(previous.len()).ok().and_then(|before| before.checked_add(r.signed().ok()?));
    let length = length.and_then(|length| usize::try_from(length).ok()).filter(|&length| length <= MAX_LAYOUT).ok_or(DecodeError("facts layout out of range"))?;
    let mut out: Vec<u8> = (0..length).map(|i| previous.get(i).copied().unwrap_or(0)).collect();
    let mut at = 0;
    while !r.is_empty() {
        at += layout_offset(r)?;
        let run = r.bytes(MAX_LAYOUT)?;
        let end = at.checked_add(run.len()).filter(|&end| end <= length).ok_or(DecodeError("facts run past the frame"))?;
        for (byte, change) in out[at..end].iter_mut().zip(run) {
            *byte ^= change;
        }
        at = end;
    }
    Ok(out)
}

/// A length or zero run within a layout.
fn layout_offset(r: &mut Reader<'_>) -> DecodeResult<usize> {
    usize::try_from(r.varint()?).ok().filter(|&value| value <= MAX_LAYOUT).ok_or(DecodeError("facts layout out of range"))
}

fn i32w(w: &mut Writer, value: i32) {
    w.u32(value.cast_unsigned());
}

fn i32r(r: &mut Reader<'_>) -> DecodeResult<i32> {
    Ok(r.u32()?.cast_signed())
}

/// The frame's flags, one bit each in this order.
fn flags(frame: &MissionFrame) -> [bool; 38] {
    let [f0, f1, f2] = frame.finished_pattern_matches;
    let [c0, c1, c2] = frame.current_pattern_matches;
    [
        frame.tester_valid,
        frame.physics_requested_teleport,
        frame.teleporting,
        frame.is_regular_stance,
        frame.just_wiped_out,
        frame.hom_bail_in_progress,
        frame.hom_bail_completed,
        frame.is_teleport_finished,
        frame.is_wipeout_teleport_pending,
        frame.is_in_state_grinding,
        frame.filtered_grinding,
        frame.is_in_state_air,
        frame.skateboard_airborne,
        frame.in_a_current_trick,
        frame.sequence_begun,
        frame.in_sequence,
        frame.is_onboard_or_hippy_jumping,
        frame.was_onboard_or_hippy_jumping,
        frame.is_offboard_and_not_hippy_jumping,
        frame.was_offboard_and_not_hippy_jumping,
        frame.is_grabbing_object,
        f0,
        f1,
        f2,
        c0,
        c1,
        c2,
        frame.finished_grind_object.0,
        frame.finished_grind.is_some(),
        frame.finished_air.is_some(),
        frame.finished_manual.is_some(),
        frame.finished_powerslide.is_some(),
        frame.performed_gesture.is_some(),
        frame.sequence_completed.is_some(),
        frame.sequence_completed.is_some_and(|sequence| sequence.successfully),
        frame.sequence_completed_successfully,
        frame.is_in_state_ground,
        frame.wiping_out,
    ]
}

fn pack(bits: &[bool]) -> u64 {
    bits.iter().enumerate().fold(0, |word, (i, &bit)| word | (u64::from(bit) << i))
}

fn unpack<const N: usize>(word: u64) -> [bool; N] {
    std::array::from_fn(|i| (word >> i) & 1 == 1)
}

fn contexts(w: &mut Writer, lists: &ContextUidLists) {
    for group in &lists.uids {
        for &(id, kind) in group {
            w.u32(id);
            i32w(w, kind);
        }
    }
    for &count in &lists.counts {
        w.u32(count);
    }
}

fn read_contexts(r: &mut Reader<'_>) -> DecodeResult<ContextUidLists> {
    let mut lists = ContextUidLists::default();
    for group in &mut lists.uids {
        for entry in group.iter_mut() {
            *entry = (r.u32()?, i32r(r)?);
        }
    }
    for count in &mut lists.counts {
        *count = r.u32()?;
    }
    Ok(lists)
}

fn air(w: &mut Writer, air: &FinishedAir) {
    w.u8(u8::from(air.landed_successfully)
        | (u8::from(air.landing_was_sketchy) << 1)
        | (u8::from(air.started_switched) << 2)
        | (u8::from(air.grab_occured) << 3)
        | (u8::from(air.tweak_occured) << 4));
    i32w(w, air.first_scorable_id);
    i32w(w, air.player_spin180_count);
    i32w(w, air.in_air_frames);
    w.f32(air.height_to_peak);
    w.f32(air.horizontal_distance);
    w.f32(air.grab_time);
    i32w(w, air.player_num_flips);
    w.u32(u32::try_from(pack(air.big_trick_thresholds.as_flattened())).unwrap_or(0));
    contexts(w, &air.initial_context);
    contexts(w, &air.end_context);
    contexts(w, &air.accumulated_context);
}

fn read_air(r: &mut Reader<'_>) -> DecodeResult<FinishedAir> {
    let [landed_successfully, landing_was_sketchy, started_switched, grab_occured, tweak_occured] = unpack(u64::from(r.u8()?));
    let first_scorable_id = i32r(r)?;
    let player_spin180_count = i32r(r)?;
    let in_air_frames = i32r(r)?;
    let height_to_peak = f32r(r)?;
    let horizontal_distance = f32r(r)?;
    let grab_time = f32r(r)?;
    let player_num_flips = i32r(r)?;
    let thresholds: [bool; 20] = unpack(u64::from(r.u32()?));
    Ok(FinishedAir {
        landed_successfully,
        first_scorable_id,
        landing_was_sketchy,
        player_spin180_count,
        started_switched,
        grab_occured,
        tweak_occured,
        in_air_frames,
        height_to_peak,
        horizontal_distance,
        grab_time,
        player_num_flips,
        big_trick_thresholds: std::array::from_fn(|i| [thresholds[2 * i], thresholds[2 * i + 1]]),
        initial_context: read_contexts(r)?,
        end_context: read_contexts(r)?,
        accumulated_context: read_contexts(r)?,
    })
}

/// The bail's values other than its save image and smooth values.
fn hom(w: &mut Writer, bail: &HomBail) {
    for &level in &bail.bone_damage_levels {
        i32w(w, level);
    }
    w.u32(bail.score);
    for &score in bail.bone_scores.iter().chain(&bail.bonus_scores) {
        w.u32(score);
    }
    w.u32(bail.max_air_frames);
    for &count in &bail.entity_counts {
        w.u32(count);
    }
    w.u32(bail.security_guards_hit);
    w.u32(bail.accumulated_air_drop_frames);
    w.u32(bail.bone_score);
    w.u32(bail.bonus_score);
    w.u32(bail.rotation_score);
    w.u32(bail.num_body_tweaks_performed);
    i32w(w, bail.current_body_tweak);
}

fn read_hom(r: &mut Reader<'_>) -> DecodeResult<HomBail> {
    let mut bail = HomBail::default();
    for level in &mut bail.bone_damage_levels {
        *level = i32r(r)?;
    }
    bail.score = r.u32()?;
    for score in bail.bone_scores.iter_mut().chain(bail.bonus_scores.iter_mut()) {
        *score = r.u32()?;
    }
    bail.max_air_frames = r.u32()?;
    for count in &mut bail.entity_counts {
        *count = r.u32()?;
    }
    bail.security_guards_hit = r.u32()?;
    bail.accumulated_air_drop_frames = r.u32()?;
    bail.bone_score = r.u32()?;
    bail.bonus_score = r.u32()?;
    bail.rotation_score = r.u32()?;
    bail.num_body_tweaks_performed = r.u32()?;
    bail.current_body_tweak = i32r(r)?;
    Ok(bail)
}

fn list_length(len: usize) -> u16 {
    u16::try_from(len.min(MAX_LIST)).unwrap_or_default()
}

/// The frame's fixed layout, then its lists. Absent events write their last value.
fn layout(frame: &MissionFrame, sticky: &mut Sticky) -> Vec<u8> {
    let mut w = Writer::with_capacity(512);
    w.u64(pack(&flags(frame)));
    w.u32(u32::try_from(pack(frame.big_trick_achieved.as_flattened())).unwrap_or(0));
    i32w(&mut w, frame.current_air_initial_scorable);
    i32w(&mut w, frame.current_gesture);
    w.u32(frame.finished_sequence_score);
    w.u32(frame.hom_score);
    w.u32(frame.anim_trick_bits);
    i32w(&mut w, frame.banked_point_change);
    w.f32(frame.finished_grind_length);
    w.raw(&frame.trick_types_in_completed_sequence);
    i32w(&mut w, frame.finished_grind_object.1);
    w.u32(frame.finished_grind_object.2);
    w.u64(frame.filtered_grind_pathed_guid);
    w.u64(frame.filtered_grind_local_guid);
    if let Some(grind) = frame.finished_grind {
        sticky.grind = grind;
    }
    w.f32(sticky.grind.length);
    w.u64(sticky.grind.pathed_guid);
    w.u64(sticky.grind.local_guid);
    if let Some(manual) = frame.finished_manual {
        sticky.manual = manual;
    }
    w.f32(sticky.manual);
    if let Some(powerslide) = frame.finished_powerslide {
        sticky.powerslide = powerslide;
    }
    w.f32(sticky.powerslide);
    if let Some(gesture) = frame.performed_gesture {
        sticky.gesture = gesture;
    }
    i32w(&mut w, sticky.gesture);
    if let Some(sequence) = frame.sequence_completed {
        sticky.sequence = sequence;
    }
    for value in [
        sticky.sequence.score,
        sticky.sequence.start_visual_tick,
        sticky.sequence.start_sim_tick,
        sticky.sequence.replay_start_frame,
        sticky.sequence.trick_count,
        sticky.sequence.stored_trick_count,
    ] {
        w.u32(value);
    }
    hom(&mut w, &frame.hom_bail);
    if let Some(finished) = frame.finished_air {
        sticky.air = finished;
    }
    air(&mut w, &sticky.air);
    w.u16(list_length(frame.active_trick_types.len()));
    for &kind in frame.active_trick_types.iter().take(MAX_LIST) {
        i32w(&mut w, kind);
    }
    w.u16(list_length(frame.active_tricks.len()));
    for &(valid, id) in frame.active_tricks.iter().take(MAX_LIST) {
        w.bool(valid);
        i32w(&mut w, id);
    }
    w.u16(list_length(frame.tricks_ended.len()));
    for trick in frame.tricks_ended.iter().take(MAX_LIST) {
        w.u8(u8::from(trick.is_valid) | (u8::from(trick.was_a_tap_trick) << 1) | (u8::from(trick.started_switched) << 2));
        w.u32(trick.trick_index);
        i32w(&mut w, trick.trick_type);
        w.f32(trick.score);
        w.u64(trick.trick_attrib_key);
    }
    w.into_vec()
}

fn read_layout(data: &[u8]) -> DecodeResult<MissionFrame> {
    let mut r = Reader::new(data);
    let [
        tester_valid,
        physics_requested_teleport,
        teleporting,
        is_regular_stance,
        just_wiped_out,
        hom_bail_in_progress,
        hom_bail_completed,
        is_teleport_finished,
        is_wipeout_teleport_pending,
        is_in_state_grinding,
        filtered_grinding,
        is_in_state_air,
        skateboard_airborne,
        in_a_current_trick,
        sequence_begun,
        in_sequence,
        is_onboard_or_hippy_jumping,
        was_onboard_or_hippy_jumping,
        is_offboard_and_not_hippy_jumping,
        was_offboard_and_not_hippy_jumping,
        is_grabbing_object,
        f0,
        f1,
        f2,
        c0,
        c1,
        c2,
        grind_object_valid,
        has_grind,
        has_air,
        has_manual,
        has_powerslide,
        has_gesture,
        has_sequence,
        sequence_successfully,
        sequence_completed_successfully,
        is_in_state_ground,
        wiping_out,
    ] = unpack(r.u64()?);
    let big: [bool; 30] = unpack(u64::from(r.u32()?));
    let mut frame = MissionFrame {
        tester_valid,
        physics_requested_teleport,
        teleporting,
        is_regular_stance,
        just_wiped_out,
        hom_bail_in_progress,
        hom_bail_completed,
        is_teleport_finished,
        is_wipeout_teleport_pending,
        is_in_state_grinding,
        filtered_grinding,
        is_in_state_air,
        skateboard_airborne,
        in_a_current_trick,
        sequence_begun,
        in_sequence,
        is_onboard_or_hippy_jumping,
        was_onboard_or_hippy_jumping,
        is_offboard_and_not_hippy_jumping,
        was_offboard_and_not_hippy_jumping,
        is_grabbing_object,
        sequence_completed_successfully,
        is_in_state_ground,
        wiping_out,
        finished_pattern_matches: [f0, f1, f2],
        current_pattern_matches: [c0, c1, c2],
        big_trick_achieved: std::array::from_fn(|i| [big[2 * i], big[2 * i + 1]]),
        ..MissionFrame::default()
    };
    frame.current_air_initial_scorable = i32r(&mut r)?;
    frame.current_gesture = i32r(&mut r)?;
    frame.finished_sequence_score = r.u32()?;
    frame.hom_score = r.u32()?;
    frame.anim_trick_bits = r.u32()?;
    frame.banked_point_change = i32r(&mut r)?;
    frame.finished_grind_length = f32r(&mut r)?;
    frame.trick_types_in_completed_sequence.copy_from_slice(r.raw(14)?);
    frame.finished_grind_object = (grind_object_valid, i32r(&mut r)?, r.u32()?);
    frame.filtered_grind_pathed_guid = r.u64()?;
    frame.filtered_grind_local_guid = r.u64()?;
    let grind = FinishedGrind { length: f32r(&mut r)?, pathed_guid: r.u64()?, local_guid: r.u64()? };
    frame.finished_grind = has_grind.then_some(grind);
    frame.finished_manual = Some(f32r(&mut r)?).filter(|_| has_manual);
    frame.finished_powerslide = Some(f32r(&mut r)?).filter(|_| has_powerslide);
    frame.performed_gesture = Some(i32r(&mut r)?).filter(|_| has_gesture);
    let sequence = FinishedSequence {
        score: r.u32()?,
        start_visual_tick: r.u32()?,
        start_sim_tick: r.u32()?,
        replay_start_frame: r.u32()?,
        trick_count: r.u32()?,
        stored_trick_count: r.u32()?,
        successfully: sequence_successfully,
    };
    frame.sequence_completed = has_sequence.then_some(sequence);
    frame.hom_bail = read_hom(&mut r)?;
    let finished = read_air(&mut r)?;
    frame.finished_air = has_air.then_some(finished);
    let count = usize::from(r.u16()?).min(MAX_LIST);
    frame.active_trick_types = (0..count).map(|_| i32r(&mut r)).collect::<DecodeResult<_>>()?;
    let count = usize::from(r.u16()?).min(MAX_LIST);
    frame.active_tricks = (0..count).map(|_| Ok((r.bool()?, i32r(&mut r)?))).collect::<DecodeResult<_>>()?;
    let count = usize::from(r.u16()?).min(MAX_LIST);
    frame.tricks_ended = (0..count)
        .map(|_| {
            let [is_valid, was_a_tap_trick, started_switched] = unpack(u64::from(r.u8()?));
            Ok(EndedTrick {
                name: String::new(),
                is_valid,
                was_a_tap_trick,
                trick_index: r.u32()?,
                trick_type: i32r(&mut r)?,
                score: f32r(&mut r)?,
                started_switched,
                trick_attrib_key: r.u64()?,
            })
        })
        .collect::<DecodeResult<_>>()?;
    r.finish()?;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::challenge::hom_save::BailImage;

    fn sample(tick: u32) -> MissionFrame {
        #[expect(clippy::cast_precision_loss, reason = "test values")]
        let t = tick as f32;
        let mut frame = MissionFrame {
            player: 3,
            local: true,
            position: [10.0 + t * 0.1, 2.0, -40.0 + t * 0.05],
            tester_base: [10.0 + t * 0.1, 1.2, -40.0 + t * 0.05],
            tester_top: [10.0 + t * 0.1, 2.9, -40.0 + t * 0.05],
            tester_centre_of_mass: [10.02 + t * 0.1, 2.1, -40.0 + t * 0.05],
            tester_valid: true,
            is_pushing: tick % 2 == 0,
            deck_speed: 4.5 + t * 0.01,
            surface_speed: 4.4 + t * 0.01,
            is_in_state_ground: true,
            manual_time: t,
            current_gesture: 35,
            current_air_initial_scorable: -1,
            ..MissionFrame::default()
        };
        if tick == 3 {
            frame.finished_air = Some(FinishedAir { landed_successfully: true, first_scorable_id: 17, player_spin180_count: -2, ..FinishedAir::default() });
            frame.tricks_ended.push(EndedTrick { name: "Kickflip".to_owned(), is_valid: true, trick_index: 5, score: 250.0, trick_attrib_key: 0x1234, ..EndedTrick::default() });
            frame.hom_bail.save_image = Some(BailImage::empty());
            frame.hom_bail.bone_damage_levels[4] = 2;
            frame.hom_bail.overall_duration = 0.5;
            frame.hom_score = 9000;
            frame.vote = Some(3);
            frame.is_teleport_finished = true;
        }
        frame
    }

    fn round_trip(scope: FactsScope) {
        let mut sender = FactsCoder::new(scope);
        let mut receiver = FactsCoder::new(scope);
        for tick in 0..8 {
            let shared = scope.shared(&sample(tick));
            let data = sender.encode(&shared);
            let decoded = receiver.decode(&data, 3).expect("decodes");
            let remote = scope.shared(&MissionFrame { local: false, ..sample(tick) });
            assert_eq!(decoded, remote, "tick {tick}");
        }
    }

    #[test]
    fn every_scope_round_trips() {
        for challenge_type in [4, 8, 9, 15, 19, 31, 32, 42] {
            round_trip(FactsScope::for_challenge_type(challenge_type));
        }
    }

    #[test]
    fn the_shared_frame_drops_what_no_other_console_reads() {
        let frame = MissionFrame { is_pushing: true, ..sample(3) };
        let shared = FactsScope::for_challenge_type(9).shared(&frame);
        assert_eq!(shared.position, frame.tester_base);
        assert_eq!(shared.hom_bail, HomBail::default(), "Spot Battle reads no bail");
        assert_eq!(shared.hom_score, 0);
        assert!(shared.tricks_ended[0].name.is_empty());
        assert_eq!(shared.manual_time.to_bits(), 0);
        assert!(shared.is_pushing, "the owner keeps its own pushes");
        assert!(shared.is_teleport_finished && shared.vote.is_none());
        let all = FactsScope::ALL.shared(&frame);
        assert_eq!(all.hom_bail.bone_damage_levels[4], 2);
        assert_eq!(all.hom_bail.save_image, None);
        // A freeskate activity's objectives read every player's own state.
        let activity = FactsScope::for_challenge_type(42).shared(&frame);
        assert_eq!((activity.manual_time, activity.is_in_state_ground), (frame.manual_time, true));
    }

    #[test]
    fn a_scope_sends_only_the_smooth_values_it_reads() {
        // S.K.A.T.E. reads the deck speed of the tester's values, and no surface speed or bail.
        let skate = FactsScope::for_challenge_type(4);
        let mut frame = skate.shared(&sample(0));
        frame.surface_speed = 7.0;
        frame.hom_bail.bail_speed = 3.0;
        let mut scoped = FactsCoder::new(skate);
        let mut everything = FactsCoder::default();
        let (scoped, everything) = (scoped.encode(&frame), everything.encode(&frame));
        assert!(scoped.len() + 4 <= everything.len(), "{} bytes against {}", scoped.len(), everything.len());
        let decoded = FactsCoder::new(skate).decode(&scoped, 3).unwrap();
        assert_eq!(decoded.surface_speed.to_bits(), 0, "values out of the scope stay at their defaults");
        assert_eq!(decoded.hom_bail.bail_speed.to_bits(), 0);
        assert_eq!(decoded.deck_speed, frame.deck_speed);
    }

    #[test]
    fn an_unchanged_tick_is_a_few_bytes() {
        let scope = FactsScope::for_challenge_type(8);
        let mut sender = FactsCoder::default();
        let frame = scope.shared(&sample(0));
        let _ = sender.encode(&frame);
        let _ = sender.encode(&frame);
        assert!(sender.encode(&frame).len() <= 4);
        let mut sender = FactsCoder::default();
        for tick in [0, 1, 2, 4, 5] {
            let _ = sender.encode(&scope.shared(&sample(tick)));
        }
        let moving = sender.encode(&scope.shared(&sample(6)));
        assert!(moving.len() <= 12, "a skater moving steadily costs {} bytes", moving.len());
    }

    #[test]
    fn an_event_costs_one_tick() {
        let scope = FactsScope::for_challenge_type(8);
        let mut sender = FactsCoder::default();
        let mut still = scope.shared(&sample(0));
        still.tester_base = [0.0; 3];
        still.tester_top = [0.0; 3];
        still.tester_centre_of_mass = [0.0; 3];
        still.deck_speed = 0.0;
        still.surface_speed = 0.0;
        let _ = sender.encode(&still);
        let _ = sender.encode(&still);
        let mut landed = still.clone();
        landed.finished_air = Some(FinishedAir { landed_successfully: true, first_scorable_id: 9, height_to_peak: 1.5, ..FinishedAir::default() });
        let landing = sender.encode(&landed).len();
        let after = sender.encode(&still).len();
        assert!(landing > 12 && after <= 8, "the landing costs {landing} bytes, the tick after it {after}");
    }

    #[test]
    fn a_standing_frame_keeps_the_state_and_drops_the_events() {
        let frame = FactsScope::ALL.shared(&sample(3));
        let standing = standing(&frame);
        assert_eq!(standing.tester_base, frame.tester_base);
        assert!(standing.finished_air.is_none() && standing.tricks_ended.is_empty() && !standing.is_teleport_finished);
        assert_eq!(standing.hom_bail, frame.hom_bail);
    }

    #[test]
    fn ordered_bits_keep_the_value_order() {
        let values = [-3.5_f32, -0.25, -0.0, 0.0, 1.0e-6, 2.0, 1000.0];
        for pair in values.windows(2) {
            assert!(ordered(pair[0]) <= ordered(pair[1]));
        }
        for value in values {
            assert_eq!(unordered(ordered(value)).to_bits(), value.to_bits());
        }
    }
}
