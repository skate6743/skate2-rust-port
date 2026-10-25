//! A remote player's skater: no simulation, only the replay frames its owner sends, restored as
//! a replay restores a recorded skater (`SkaterPresentation::DoSimRestore` 0x827e6438). Its look
//! is the owner's recipe, palette skinning and rig, rebuilt from the game's own assets as a saved
//! replay's characters are (`CharacterRecipeLoader::LoadRecipes`'s role).
//!
//! Frames arrive with network jitter; each is shown a little after it arrives. A line through the
//! recent arrivals (each frame's local arrival tick against its tick) gives the sender's pace, and
//! how late the late ones came behind it the playout delay, so the skater moves at an even pace:
//! its sender's (a game that cannot keep 60 ticks a second sends fewer, a game that runs slower
//! than this one shows more a tick). While the delay changes the moment shown moves on at most a
//! hundredth faster or slower than that pace, a tenth when it is far off, never past the newest
//! frame, and a moment between frames (a lost frame's, or a tick without a pose sample) blends the
//! frames around it, as the owner's game camera does for a console that follows the skater: a
//! lost or late frame neither stops the skater nor makes it jump, and a stream that stops resumes
//! where it stopped.

use super::super::replay_editor_session::archive::{
    characters::Catalog,
    codec::{Reader, Writer},
};
use crate::{
    actor_animation_construction::initialization::body_dispatch::tree_commands::numeric::blend,
    actor_face_playback::presentation::{frame::selected_runtime::render_opacity::SkaterRenderOpacity, move_root::rigid_matrix},
    attribute_runtime::AttributeRuntimeVault,
    character::{material_admission::CharacterMaterialAdmission, palette::CharacterPalette},
    input_cvar_runtime::InputCvarReferences,
    player::mounted_recipe::finalize::FinalizedRecipeScene,
    presentation_restore::{
        math::slerp,
        skater::{HEADER_BYTES, SkaterFrame, SkaterRestore, SkaterRig},
    },
    skater_ropa_runtime::ClothClock,
    skeleton_root_transforms::SkeletonRootMatrix,
    world::try_simulation_quaternion_candidate,
};
use skate2_formats::{andale::AndaleSqt, mount::MountSet};
use crate::online::hud_record::{HudFollower, HudRecord, HudStep};
use skate2_net::frames::{CAMERA_VALUES, HEADER_WORDS, INTERVAL, NetFrame, tick_before};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

/// The recipe message's format.
const CHARACTER_VERSION: u8 = 1;
/// Frames kept behind the one shown.
const KEPT_BEHIND: u32 = 120;
/// How far the moment shown may move back behind the trick HUD records it passed and still be
/// the same run of the sender's ticks (a playout shift); further back, the sender's ticks started
/// again, and the records start over.
const HUD_REWIND: u32 = 600;
/// The most frames kept, the oldest going first: ten seconds of a sender's ticks, past any lag
/// of the moment shown behind the newest (it reaches the newest at once after a hold). Frames
/// stamped far ahead of the sender's ticks are never shown, and would otherwise pile up.
const KEPT_FRAMES: usize = 600;
/// Arrivals the playout follows: three seconds of a sender's frames.
const ARRIVALS: usize = 180;
/// Ticks added to the late arrivals' delay.
const DELAY_MARGIN: f64 = 3.0;
/// The most the late arrivals delay the moment shown; a frame later than this is bridged as a
/// lost one is.
const MAX_DELAY: f64 = 45.0;
/// A target further than this (a stall, a long loss) is reached at once.
const SNAP_TICKS: f64 = 30.0;
/// The sender's pace bounds, in its ticks a local tick.
const MIN_PACE: f64 = 0.75;
const MAX_PACE: f64 = 1.25;
/// Local ticks over which an arrival's weight in the arrival line falls by e: the line follows a
/// sender whose pace changes within a second or two.
const RECENCY_TICKS: f64 = 90.0;
/// Arrivals further than this many ticks from the line weigh less in it, by their distance (a
/// route's spike, or the backlog a hitch of this console's own ticks stamps early), over this
/// many fits again.
const OUTLIER_TICKS: f64 = 2.0;
const REFITS: usize = 3;
/// Within `FAR_TICKS` of its target the moment shown closes `NEAR_GAIN` of the distance a tick, at
/// most `NEAR_RATE` faster or slower than the sender's pace; further off, or while fewer than
/// [`ARRIVALS`] arrivals set the target (a skater just shown, a rebase), `FAR_GAIN`, at most
/// `FAR_RATE`.
const NEAR_GAIN: f64 = 0.03;
const NEAR_RATE: f64 = 0.01;
const FAR_TICKS: f64 = 3.0;
const FAR_GAIN: f64 = 0.1;
const FAR_RATE: f64 = 0.1;
/// Within this many ticks of the newest frame, past the run allowance, the moment shown slows
/// down rather than run into it.
const HEADROOM_TICKS: f64 = 1.0;
/// The newest frames that all arrive later, or all earlier, than the frames before them let them
/// by more than `SHIFT_TICKS`: the sender's ticks stopped against this console's (a travel), or
/// this console's did, and the arrivals before them no longer say when frames come.
const SHIFT_FRAMES: usize = 12;
const SHIFT_TICKS: f64 = 10.0;
/// How far the late arrivals lay behind the median lately is remembered, decaying by this many
/// ticks a tick (8 a minute): a link that spikes every few seconds meets each spike with the delay
/// the one before set, rather than with a hold.
const RESERVE_DECAY: f64 = 8.0 / 3600.0;
/// Header words 32 to 38: the root rotation quaternion and world position.
const ROOT_WORD: usize = 32;
/// Header word 0x30 (byte 192): the locality bit `UpdatePresEntity` reads, and the teleport bit.
const FLAGS_WORD: usize = 48;
const LOCAL_FLAG: u32 = 0x8000;
const TELEPORT_FLAG: u32 = 0x1000;
/// Header words 40 and 41: `peerId` (bytes 160..168).
const PEER_WORD: usize = 40;
/// Ticks without a frame after which a skater no longer shows: its player stopped sending (in a
/// challenge apart from this player's lobby skate, or loading), and comes back with its frames.
/// A shorter hitch (a slow frame, a stall) leaves the skater where it was.
const STALE_TICKS: u32 = 180;

/// The local skater's look as the recipe message carries it: the replay archive's character
/// catalog of its palette (and so its recipe) and rig.
///
/// # Errors
/// A palette the catalog cannot describe, or a message past the server's limit (the server
/// drops a connection that sends one).
pub(super) fn encode_character(frame: &SkaterFrame) -> Result<Vec<u8>, String> {
    let mut catalog = Catalog::default();
    catalog.palette(&frame.palette);
    catalog.rig(&frame.rig);
    let mut writer = Writer(vec![CHARACTER_VERSION]);
    catalog.put(&mut writer)?;
    if writer.0.len() > skate2_net::protocol::MAX_RECIPE {
        return Err(format!("{} bytes, past the server's limit", writer.0.len()));
    }
    Ok(writer.0)
}

/// The rotation of a camera matrix as `CameraFrame::Lerp` (0x829d3978) takes it.
fn camera_rotation(matrix: &SkeletonRootMatrix) -> [f32; 4] {
    try_simulation_quaternion_candidate(matrix.xyz_scratch(), |magnitudes| Ok::<_, std::convert::Infallible>(magnitudes.map(|value| 1.0 / value.sqrt())))
        .unwrap_or_else(|never| match never {})
}

/// The game camera as frames carry it: its position, its rotation (on the side of `previous`,
/// the rotation last sent, so the values move on smoothly) and its vertical field of view.
pub(super) fn camera_values(matrix: &SkeletonRootMatrix, fov: f32, previous: Option<[f32; 4]>) -> [f32; CAMERA_VALUES] {
    let mut rotation = camera_rotation(matrix);
    if previous.is_some_and(|previous| previous.iter().zip(&rotation).map(|(a, b)| a * b).sum::<f32>() < 0.0) {
        rotation = rotation.map(|value| -value);
    }
    let [x, y, z] = matrix.columns()[3].xyz;
    [x, y, z, rotation[0], rotation[1], rotation[2], rotation[3], fov]
}

/// A remote player's game camera at the tick its skater shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RemoteCamera {
    /// The camera matrix: its axes, then its position.
    pub(super) matrix: [f32; 16],
    /// The vertical field of view in degrees.
    pub(super) fov: f32,
}

/// The look, header and pose source of one remote player.
pub(super) struct Character {
    rig: Arc<SkaterRig>,
    palette: Arc<CharacterPalette>,
    finalized: Option<FinalizedRecipeScene>,
    materials: Option<Arc<Mutex<CharacterMaterialAdmission>>>,
}

impl Character {
    /// Rebuilds a recipe message's skater from the game's assets.
    ///
    /// # Errors
    /// A malformed message or a recipe the assets cannot build.
    fn decode(data: &[u8], mounts: &MountSet, vault: &Arc<AttributeRuntimeVault>) -> Result<Self, String> {
        let (&version, body) = data.split_first().ok_or("empty character")?;
        if version != CHARACTER_VERSION {
            return Err(format!("character format {version}"));
        }
        let mut reader = Reader::new(body);
        // `encode_character` sends one look, palette and rig; a message with more is refused
        // before any of them loads.
        let catalog = Catalog::get_at_most(&mut reader, mounts, vault, [1, 1, 1])?;
        if reader.offset != body.len() {
            return Err("trailing character data".into());
        }
        let palette = catalog.palettes.into_iter().next().ok_or("character without a palette")?;
        let rig = catalog.rigs.into_iter().next().ok_or("character without a rig")?;
        if rig.inverse_bind.len() != palette.skinning().hierarchy_inverse_bind_pose.len() {
            return Err("character rig and palette disagree".into());
        }
        let mut finalized = FinalizedRecipeScene::new(Arc::clone(palette.scene()));
        finalized.try_finalize(3).map_err(|e| e.to_string())?;
        Ok(Self { rig, palette, finalized: Some(finalized), materials: None })
    }

    /// The recipe message's skater with its entity's materials, which `UpdatePresEntity` constructs
    /// from the first header: their only header inputs are the frame's peer, the remote
    /// player's, and its locality, never this console's, so the build need not wait for a frame.
    pub(super) fn build(data: &[u8], user_id: u64, mounts: &MountSet, vault: &Arc<AttributeRuntimeVault>, cvars: &InputCvarReferences) -> Result<Self, String> {
        let mut character = Self::decode(data, mounts, vault)?;
        let mut words = [0; HEADER_WORDS];
        let peer = user_id.to_be_bytes();
        words[PEER_WORD] = u32::from_be_bytes([peer[0], peer[1], peer[2], peer[3]]);
        words[PEER_WORD + 1] = u32::from_be_bytes([peer[4], peer[5], peer[6], peer[7]]);
        character.materials(&header_bytes(&words), vault, cvars)?;
        Ok(character)
    }

    /// The entity's materials, constructed from the first header (`UpdatePresEntity`).
    fn materials(&mut self, header: &[u8], vault: &Arc<AttributeRuntimeVault>, cvars: &InputCvarReferences) -> Result<Arc<Mutex<CharacterMaterialAdmission>>, String> {
        if let Some(materials) = &self.materials {
            return Ok(Arc::clone(materials));
        }
        let finalized = self.finalized.take().ok_or("character materials failed before")?;
        let mut materials = CharacterMaterialAdmission::try_new(&finalized, &self.palette, Arc::clone(vault), cvars.clone()).map_err(|e| e.to_string())?;
        materials.try_construct_from_header(header).map_err(|e| e.to_string())?;
        let materials = Arc::new(Mutex::new(materials));
        self.materials = Some(Arc::clone(&materials));
        Ok(materials)
    }
}

/// When received frames are shown. The line through the recent arrivals (local arrival tick
/// against remote tick) gives the sender's pace; each local tick shows the remote moment that
/// arrives as late behind that line as the late arrivals did, plus a margin and the sender's run
/// spacing, and the moment shown moves on at the pace, closing on that target gently.
struct Playout {
    /// The local tick and the remote tick of each recent arrival, oldest first.
    arrivals: VecDeque<(u32, u32)>,
    /// The newest remote tick arrived, past which the moment shown never runs.
    newest: Option<u32>,
    /// The remote moment shown last and the local tick it was shown at.
    shown: Option<(f64, u32)>,
    /// The sender's ticks a local tick.
    pace: f64,
    /// How far the late arrivals lay behind the median lately, decaying, and the local tick it
    /// was last measured at.
    reserve: f64,
    reserve_at: Option<u32>,
    /// Frames came while this console's ticks were held (a travel, a load, a sync point): the
    /// arrivals before no longer say when frames come, and the next one counted starts afresh.
    forget: bool,
    /// The sender's run spacing, added to the delay: while one run of frames is lost, or late by
    /// the spread of arrivals, the moment shown still lies between the samples around it, and
    /// the skater moves on through the loss rather than stopping at the last sample.
    allowance: u32,
    /// How far behind the newest frame the last moment shown aimed.
    delay: f64,
}

impl Default for Playout {
    fn default() -> Self {
        Self {
            arrivals: VecDeque::new(),
            newest: None,
            shown: None,
            pace: 1.0,
            reserve: 0.0,
            reserve_at: None,
            forget: false,
            allowance: 0,
            delay: 0.0,
        }
    }
}

/// Ticks from `base` to `tick`.
fn ticks_from(tick: u32, base: u32) -> f64 {
    f64::from(i32::try_from(i64::from(tick) - i64::from(base)).unwrap_or(i32::MAX))
}

/// The arrivals as (remote tick, local tick), counted from the oldest's.
fn relative(arrivals: &VecDeque<(u32, u32)>) -> Vec<(f64, f64)> {
    let Some(&(base_local, base_remote)) = arrivals.front() else { return Vec::new() };
    arrivals.iter().map(|&(local, remote)| (ticks_from(remote, base_remote), ticks_from(local, base_local))).collect()
}

/// The arrival line `local = remote * slope + intercept` through `points` (remote, local) by
/// least squares, each point weighted by how recent it is (with `recency`) and, fitted again, by
/// how close it lies to the line before; its slope within the pace bounds.
fn arrival_line(points: &[(f64, f64)], recency: bool) -> Option<(f64, f64)> {
    let newest = points.iter().map(|&(_, local)| local).fold(f64::MIN, f64::max);
    let mut line: Option<(f64, f64)> = None;
    for _ in 0..=REFITS {
        let weights: Vec<f64> = points
            .iter()
            .map(|&(remote, local)| {
                let recent = if recency { ((local - newest) / RECENCY_TICKS).exp() } else { 1.0 };
                let near = line.map_or(1.0, |(slope, intercept)| OUTLIER_TICKS / (local - remote.mul_add(slope, intercept)).abs().max(OUTLIER_TICKS));
                recent * near
            })
            .collect();
        let total: f64 = weights.iter().sum();
        if total <= 0.0 {
            break;
        }
        let weighted = |value: fn(&(f64, f64)) -> f64| points.iter().zip(&weights).map(|(point, weight)| weight * value(point)).sum::<f64>() / total;
        let (mean_remote, mean_local) = (weighted(|point| point.0), weighted(|point| point.1));
        let (spread, covariance) = points.iter().zip(&weights).fold((0.0, 0.0), |(spread, covariance), (&(remote, local), weight)| {
            let along = weight * (remote - mean_remote);
            (along.mul_add(remote - mean_remote, spread), along.mul_add(local - mean_local, covariance))
        });
        if spread <= 0.0 {
            break;
        }
        let slope = (covariance / spread).clamp(MAX_PACE.recip(), MIN_PACE.recip());
        line = Some((slope, slope.mul_add(-mean_remote, mean_local)));
    }
    line
}

/// How far each point lies behind the line, in local ticks, ascending.
fn lateness(points: &[(f64, f64)], (slope, intercept): (f64, f64)) -> Vec<f64> {
    let mut late: Vec<f64> = points.iter().map(|&(remote, local)| local - remote.mul_add(slope, intercept)).collect();
    late.sort_by(f64::total_cmp);
    late
}

/// The value `percent` of the way up `sorted`.
fn percentile(sorted: &[f64], percent: usize) -> f64 {
    sorted[(sorted.len() * percent / 100).min(sorted.len() - 1)]
}

impl Playout {
    fn arrived(&mut self, local: u32, remote: u32) {
        if std::mem::take(&mut self.forget) {
            self.arrivals.clear();
        }
        if self.arrivals.len() == ARRIVALS {
            self.arrivals.pop_front();
        }
        self.arrivals.push_back((local, remote));
        self.newer(remote);
        self.rebase();
    }

    /// A frame that came while this console's ticks were held.
    fn held(&mut self, remote: u32) {
        self.forget = true;
        self.newer(remote);
    }

    fn newer(&mut self, remote: u32) {
        if self.newest.is_none_or(|newest| tick_before(newest, remote)) {
            self.newest = Some(remote);
        }
    }

    /// The arrivals' line: fitted once there are enough of them, else at the pace of real time
    /// through their mean.
    fn line(points: &[(f64, f64)]) -> Option<(f64, f64)> {
        if points.len() >= SHIFT_FRAMES * 2 {
            return arrival_line(points, true);
        }
        let count = f64::from(u32::try_from(points.len()).ok().filter(|&count| count > 0)?);
        Some((1.0, points.iter().map(|&(remote, local)| local - remote).sum::<f64>() / count))
    }

    /// How far the late arrivals lie behind the median, measured over the arrivals before the
    /// newest [`SHIFT_FRAMES`] (a shift of the two consoles' ticks is rebased before it reaches
    /// them), remembered and decaying by [`RESERVE_DECAY`].
    fn measure_reserve(&mut self, local: u32, points: &[(f64, f64)], line: (f64, f64)) {
        if self.reserve_at == Some(local) {
            return;
        }
        let elapsed = self.reserve_at.map_or(0.0, |at| f64::from(local.wrapping_sub(at)));
        self.reserve_at = Some(local);
        let settled = points.len().saturating_sub(SHIFT_FRAMES);
        let measured = if settled >= SHIFT_FRAMES * 2 {
            let older = lateness(&points[..settled], line);
            percentile(&older, 95) - percentile(&older, 50)
        } else {
            0.0
        };
        self.reserve = (-elapsed).mul_add(RESERVE_DECAY, self.reserve).max(measured).max(0.0);
    }

    /// Keeps only the newest frames when they all arrived later, or all earlier, than the older
    /// arrivals' line lets them: the two consoles' ticks moved apart, which jitter does not do (a
    /// run on time among them would show it).
    fn rebase(&mut self) {
        if self.arrivals.len() < SHIFT_FRAMES * 3 {
            return;
        }
        let points = relative(&self.arrivals);
        let split = points.len() - SHIFT_FRAMES;
        let Some(line) = arrival_line(&points[..split], false) else { return };
        let older = lateness(&points[..split], line);
        let newer = lateness(&points[split..], line);
        if newer[0] > percentile(&older, 95) + SHIFT_TICKS || newer[newer.len() - 1] < older[0] - SHIFT_TICKS {
            self.arrivals.drain(..split);
        }
    }

    /// The remote moment to show at `local`, never earlier than the last one nor past the newest
    /// frame: the moment that arrives as late behind the arrivals' line as the late arrivals did
    /// (the 95th percentile, or the median and the reserve), a margin and the run allowance before
    /// `local`, approached at the sender's pace, at most [`NEAR_RATE`] faster or slower near it
    /// and [`FAR_RATE`] further off or while the arrivals are few, or at once when it is more than
    /// [`SNAP_TICKS`] away.
    fn next(&mut self, local: u32) -> Option<f64> {
        let newest = f64::from(self.newest?);
        let &(base_local, base_remote) = self.arrivals.front()?;
        let points = relative(&self.arrivals);
        let line = Self::line(&points)?;
        let (slope, intercept) = line;
        self.pace = slope.recip();
        self.measure_reserve(local, &points, line);
        let spread = lateness(&points, line);
        let allowance = f64::from(self.allowance);
        let late = percentile(&spread, 95).max(percentile(&spread, 50) + self.reserve).min(spread[0] + MAX_DELAY - DELAY_MARGIN - allowance);
        let target = f64::from(base_remote) + (ticks_from(local, base_local) - intercept - late - DELAY_MARGIN) / slope - allowance;
        self.delay = newest - target;
        let shown = match self.shown {
            None => target.min(newest),
            Some((previous, at)) if at == local => previous,
            Some((previous, at)) => {
                let ticks = f64::from(local.wrapping_sub(at));
                let expected = ticks.mul_add(self.pace, previous);
                let distance = target - expected;
                let shown = if distance.abs() > SNAP_TICKS {
                    target
                } else {
                    let settling = self.arrivals.len() < ARRIVALS;
                    let (gain, rate) = if settling || distance.abs() > FAR_TICKS { (FAR_GAIN, FAR_RATE) } else { (NEAR_GAIN, NEAR_RATE) };
                    let most = rate * self.pace;
                    let mut change = (distance * gain).clamp(-most, most);
                    let headroom = newest - expected - allowance;
                    if headroom < HEADROOM_TICKS {
                        change = change.min(-(FAR_GAIN * (HEADROOM_TICKS - headroom)).min(FAR_RATE) * self.pace);
                    }
                    change.mul_add(ticks, expected)
                };
                // Past the newest frame there is nothing to show: a stream that stops resumes
                // where it stopped rather than from a moment it has to catch up with.
                shown.max(previous).min(newest.max(previous))
            }
        };
        self.shown = Some((shown, local));
        Some(shown)
    }
}

/// The remote tick `time` falls in.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a remote tick, never negative")]
fn floor_tick(time: f64) -> u32 {
    time.floor() as u32
}

/// How far `time` lies from tick `before` to tick `after`.
#[expect(clippy::cast_possible_truncation, reason = "a fraction")]
fn fraction(time: f64, before: u32, after: u32) -> f32 {
    ((time - f64::from(before)) / f64::from(after - before)) as f32
}

/// The header at a moment between two frames: the frame before's, its root rotation and position
/// moved toward the frame after's by the amount given.
fn header_at(before: &NetFrame, after: Option<(&NetFrame, f32)>) -> [u32; HEADER_WORDS] {
    let mut words = before.header;
    let Some((after, amount)) = after else { return words };
    let lane = |frame: &NetFrame, index: usize| f32::from_bits(frame.header[ROOT_WORD + index]);
    let rotation = slerp(std::array::from_fn(|index| lane(before, index)), std::array::from_fn(|index| lane(after, index)), amount);
    for (index, value) in rotation.iter().enumerate() {
        words[ROOT_WORD + index] = value.to_bits();
    }
    for index in 4..7 {
        let (from, to) = (lane(before, index), lane(after, index));
        words[ROOT_WORD + index] = (to - from).mul_add(amount, from).to_bits();
    }
    words
}

/// One tick of a remote skater ready to present.
pub(super) struct Shown {
    /// What the presentation submits for the restore.
    pub(super) frame: SkaterFrame,
    pub(super) audio: Option<crate::SkaterAudioPhysOut>,
    /// The skater root's world position (header words 36 to 38).
    pub(super) root: [f32; 3],
}

/// A remote player's skater.
pub(super) struct RemoteSkater {
    pub(super) user_id: u64,
    pub(super) slot: u8,
    character: Option<Character>,
    frames: BTreeMap<u32, NetFrame>,
    playout: Playout,
    restore: Option<SkaterRestore>,
    previous: Option<SkaterFrame>,
    shown: Option<Shown>,
    camera: Option<RemoteCamera>,
    /// The local tick the last frame arrived on, and the newest sample's tick.
    last_arrival: Option<u32>,
    last_sample: Option<u32>,
    /// The last tick of the newest run, whose spacing from the run before is the playout's
    /// allowance.
    last_run_end: Option<u32>,
    /// Local ticks that showed a moment at or past the newest sample (the pose held there), and
    /// the ticks counted, for the trace.
    held: (u32, u32),
    /// Frames that arrived with the teleport bit, which no blend crosses, for the trace.
    teleports: u32,
    /// No frame for [`STALE_TICKS`]: the skater is not shown.
    stale: bool,
    /// The trick HUD records of the ticks the moment shown has passed, and the newest such tick.
    hud: HudFollower,
    hud_through: Option<u32>,
    /// The character a recipe is building on its own thread, and how long it took.
    building: Option<std::thread::JoinHandle<(Result<Character, String>, std::time::Duration)>>,
}

fn header_bytes(words: &[u32; HEADER_WORDS]) -> [u8; HEADER_BYTES] {
    let mut bytes = [0; HEADER_BYTES];
    for (chunk, word) in bytes.chunks_exact_mut(4).zip(words) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    bytes
}

fn sqt(lanes: &[f32; 12]) -> AndaleSqt {
    AndaleSqt {
        scale: [lanes[0], lanes[1], lanes[2], lanes[3]],
        rotation: [lanes[4], lanes[5], lanes[6], lanes[7]],
        translation: [lanes[8], lanes[9], lanes[10], lanes[11]],
    }
}

impl RemoteSkater {
    pub(super) fn new(user_id: u64, slot: u8) -> Self {
        Self {
            user_id,
            slot,
            character: None,
            frames: BTreeMap::new(),
            playout: Playout::default(),
            restore: None,
            previous: None,
            shown: None,
            camera: None,
            last_arrival: None,
            last_sample: None,
            last_run_end: None,
            held: (0, 0),
            teleports: 0,
            stale: true,
            hud: HudFollower::default(),
            hud_through: None,
            building: None,
        }
    }

    /// Builds a recipe message's character on its own thread: reading its parts from the
    /// archives and building its palette, scene and materials would hold the simulation for many
    /// frames. A newer recipe replaces one still building.
    ///
    /// # Errors
    /// The thread could not start.
    pub(super) fn build_character(&mut self, data: Vec<u8>, mounts: Arc<MountSet>, vault: Arc<AttributeRuntimeVault>, cvars: InputCvarReferences) -> Result<(), String> {
        let user_id = self.user_id;
        let thread = std::thread::Builder::new()
            .name("sk8 remote character".into())
            .spawn(move || {
                let start = std::time::Instant::now();
                (Character::build(&data, user_id, &mounts, &vault, &cvars), start.elapsed())
            })
            .map_err(|e| format!("remote character thread: {e}"))?;
        self.building = Some(thread);
        Ok(())
    }

    /// A character the thread finished building, set: how long it took, or why it failed.
    pub(super) fn finish_character(&mut self) -> Option<Result<std::time::Duration, String>> {
        if !self.building.as_ref()?.is_finished() {
            return None;
        }
        let (built, took) = match self.building.take()?.join() {
            Ok(outcome) => outcome,
            Err(_) => return Some(Err("the character thread panicked".into())),
        };
        Some(built.map(|character| {
            self.set_character(character);
            took
        }))
    }

    pub(super) fn set_character(&mut self, character: Character) {
        self.restore = Some(SkaterRestore::new(CharacterPalette::clone(&character.palette), SkaterRenderOpacity::constructed()));
        self.character = Some(character);
        self.previous = None;
        self.shown = None;
    }

    #[must_use]
    pub(super) fn has_character(&self) -> bool {
        self.character.is_some()
    }

    /// Whether a frame of this skater shows: one has since its character arrived, and its
    /// frames still arrive.
    #[must_use]
    pub(super) fn visible(&self) -> bool {
        self.shown().is_some()
    }

    /// Shows the skater no more until its frames come again, as when they stop: its owner left
    /// for where this game sees nothing of it.
    pub(super) fn hide(&mut self) {
        self.last_arrival = None;
        self.stale = true;
    }

    /// A frame arrived on `local_tick`, the last of its run when `run_end`. While this console's
    /// ticks are held (`paced` false: a travel, a load, a sync point) its arrival says nothing of
    /// when frames come, and the playout does not count it.
    pub(super) fn receive(&mut self, local_tick: u32, frame: NetFrame, paced: bool, run_end: bool) {
        if paced {
            self.playout.arrived(local_tick, frame.tick);
        } else {
            self.playout.held(frame.tick);
        }
        if run_end && self.last_run_end.is_none_or(|last| tick_before(last, frame.tick)) {
            // The spacing of the sender's runs; a wider one is a loss, a tick's a teleport's.
            if let Some(spacing) = self.last_run_end.map(|last| frame.tick.wrapping_sub(last)).filter(|spacing| (2..=INTERVAL).contains(spacing)) {
                self.playout.allowance = spacing;
            }
            self.last_run_end = Some(frame.tick);
        }
        if frame.pose.is_some() && self.last_sample.is_none_or(|last| tick_before(last, frame.tick)) {
            self.last_sample = Some(frame.tick);
        }
        if frame.header[FLAGS_WORD] & TELEPORT_FLAG != 0 {
            self.teleports += 1;
        }
        self.frames.insert(frame.tick, frame);
        while self.frames.len() > KEPT_FRAMES {
            self.frames.pop_first();
        }
        self.last_arrival = Some(local_tick);
    }

    /// Whether a teleport falls after `before` up to `after` (or `tick` without one).
    fn teleported(&self, before: u32, after: Option<u32>, tick: u32) -> bool {
        self.frames.range(before + 1..=after.unwrap_or(tick)).any(|(_, frame)| frame.header[FLAGS_WORD] & TELEPORT_FLAG != 0)
    }

    /// The frames around `time`: the newest at or before it, and the oldest after it with how far
    /// toward it `time` lies, unless none has come yet or a teleport parts them.
    fn around(&self, time: f64) -> Option<(&NetFrame, Option<(&NetFrame, f32)>)> {
        let (&before_tick, before) = self.frames.range(..=floor_tick(time)).next_back()?;
        let after = self
            .frames
            .range(before_tick + 1..)
            .next()
            .filter(|&(&after_tick, _)| f64::from(before_tick) < time && !self.teleported(before_tick, Some(after_tick), after_tick))
            .map(|(&after_tick, frame)| (frame, fraction(time, before_tick, after_tick)));
        Some((before, after))
    }

    /// The pose at `time`: the samples around it blended where no teleport parts them, else the
    /// newest sample at or before it.
    fn pose(&self, time: f64) -> Option<Vec<AndaleSqt>> {
        let (&before_tick, before) = self.frames.range(..=floor_tick(time)).rev().find(|(_, frame)| frame.pose.is_some())?;
        let before_pose = before.pose.as_ref()?;
        let after = self.frames.range(before_tick + 1..).find(|(_, frame)| frame.pose.is_some());
        match after {
            Some((&after_tick, after)) if f64::from(before_tick) < time && !self.teleported(before_tick, Some(after_tick), after_tick) => {
                let after_pose = after.pose.as_ref()?;
                if after_pose.len() != before_pose.len() {
                    return Some(before_pose.iter().map(sqt).collect());
                }
                let coefficient = fraction(time, before_tick, after_tick);
                Some(before_pose.iter().zip(after_pose).map(|(a, b)| blend(sqt(a), sqt(b), coefficient)).collect())
            }
            _ => Some(before_pose.iter().map(sqt).collect()),
        }
    }

    /// The owner's camera at `time`: the samples around it blended as a replay's recorded camera
    /// is (`CameraFrame::Lerp` 0x829d3978: the rotation slerped, the position and field of view
    /// lerped), else the newest sample at or before it where no later one has come or a teleport
    /// separates them.
    fn camera_at(&self, time: f64) -> Option<RemoteCamera> {
        let sampled = |frame: &&NetFrame| frame.camera.len() == CAMERA_VALUES;
        let (&before_tick, before) = self.frames.range(..=floor_tick(time)).rev().find(|(_, frame)| sampled(frame))?;
        let after = self.frames.range(before_tick + 1..).find(|(_, frame)| sampled(frame)).filter(|_| f64::from(before_tick) < time);
        let values: [f32; CAMERA_VALUES] = match after {
            Some((&after_tick, after)) if !self.teleported(before_tick, Some(after_tick), after_tick) => {
                let amount = fraction(time, before_tick, after_tick);
                let lerp = |index: usize| (after.camera[index] - before.camera[index]).mul_add(amount, before.camera[index]);
                let rotation = slerp(
                    [before.camera[3], before.camera[4], before.camera[5], before.camera[6]],
                    [after.camera[3], after.camera[4], after.camera[5], after.camera[6]],
                    amount,
                );
                [lerp(0), lerp(1), lerp(2), rotation[0], rotation[1], rotation[2], rotation[3], lerp(7)]
            }
            _ => std::array::from_fn(|index| before.camera[index]),
        };
        let rotation = [values[3], values[4], values[5], values[6]];
        Some(RemoteCamera { matrix: rigid_matrix(rotation, [values[0], values[1], values[2], 1.0]), fov: values[7] })
    }

    /// Picks this local tick's remote moment and restores it: the presentation's frame, the
    /// posed palette, the audio record and the owner's camera.
    ///
    /// # Errors
    /// Materials or a restore that fail.
    pub(super) fn advance(&mut self, local_tick: u32, camera: [f32; 3], vault: &Arc<AttributeRuntimeVault>, cvars: &InputCvarReferences) -> Result<(), String> {
        self.stale = self.last_arrival.is_none_or(|arrival| local_tick.saturating_sub(arrival) > STALE_TICKS);
        let Some(time) = self.playout.next(local_tick) else { return Ok(()) };
        self.pass_hud(floor_tick(time));
        self.held.1 += 1;
        if self.last_sample.is_some_and(|sample| time >= f64::from(sample)) {
            self.held.0 += 1;
        }
        self.camera = self.camera_at(time).or(self.camera);
        let Some((before, after)) = self.around(time) else { return Ok(()) };
        let mut words = header_at(before, after);
        let twist = before.twist;
        let root = [words[ROOT_WORD + 4], words[ROOT_WORD + 5], words[ROOT_WORD + 6]].map(f32::from_bits);
        let audio = crate::online::audio_record::unflatten(&before.audio_floats, &before.audio_words).map(|record| crate::online::audio_record::at_root(record, root));
        let Some(pose) = self.pose(time) else { return Ok(()) };
        let Some(character) = self.character.as_mut() else { return Ok(()) };
        if pose.len() != character.rig.bind.len() {
            return Err("remote pose does not fit the rig".into());
        }
        words[FLAGS_WORD] &= !LOCAL_FLAG;
        let peer = self.user_id.to_be_bytes();
        words[PEER_WORD] = u32::from_be_bytes([peer[0], peer[1], peer[2], peer[3]]);
        words[PEER_WORD + 1] = u32::from_be_bytes([peer[4], peer[5], peer[6], peer[7]]);
        let header = header_bytes(&words);
        let materials = character.materials(&header, vault, cvars)?;
        materials.lock().map_err(|_| "remote materials lock")?.try_construct_from_header(&header).map_err(|e| e.to_string())?;
        let next = SkaterFrame { rig: Arc::clone(&character.rig), palette: Arc::clone(&character.palette), header, pose, twist_ratios: twist, materials: None };
        let restore = self.restore.as_mut().ok_or("remote restore absent")?;
        // Once a local tick: one presented frame of the original's fade.
        let elapsed = crate::simulation_sim_rate::UPDATE_TIME;
        restore.restore(self.previous.as_ref(), &next, ClothClock { frame: local_tick, fraction: 1.0 }, camera, false, elapsed).map_err(|e| e.to_string())?;
        let submitted = SkaterFrame {
            rig: Arc::clone(&next.rig),
            palette: Arc::clone(&next.palette),
            header: next.header,
            pose: next.pose.clone(),
            twist_ratios: next.twist_ratios,
            materials: Some(materials),
        };
        self.previous = Some(next);
        self.shown = Some(Shown { frame: submitted, audio, root });
        let keep = floor_tick(time).saturating_sub(KEPT_BEHIND);
        self.frames = self.frames.split_off(&keep);
        Ok(())
    }

    /// Hands the trick HUD the records of the ticks the moment shown moved past, up to `through`,
    /// so a followed skater's HUD changes as its skater does. A moment that holds or moves back
    /// passes nothing; one far back (the sender's ticks started again) starts over.
    fn pass_hud(&mut self, through: u32) {
        if let Some(passed) = self.hud_through
            && !tick_before(passed, through)
        {
            if passed.wrapping_sub(through) <= HUD_REWIND {
                return;
            }
            self.hud.forget();
            self.hud_through = None;
        }
        let first = self.hud_through.map_or(through, |passed| passed.wrapping_add(1));
        let passed: Vec<(u32, &NetFrame)> = if first <= through {
            self.frames.range(first..=through).map(|(&tick, frame)| (tick, frame)).collect()
        } else {
            self.frames.range(first..).chain(self.frames.range(..=through)).map(|(&tick, frame)| (tick, frame)).collect()
        };
        for (tick, frame) in passed {
            self.hud.pass(tick, &frame.hud);
        }
        self.hud_through = Some(through);
    }

    /// The trick HUD records passed since the last call, oldest first, and the newest passed.
    pub(super) fn take_hud(&mut self) -> (Vec<HudStep>, Option<HudRecord>) {
        (self.hud.take(), self.hud.latest().cloned())
    }

    /// The ticks that held the pose at or past the newest sample and the ticks shown since the
    /// last call, with the playout delay and the sender's pace now, and the frames since then that
    /// came with the teleport bit.
    pub(super) fn take_held(&mut self) -> (u32, u32, f64, f64, u32) {
        let (held, shown) = std::mem::take(&mut self.held);
        (held, shown, self.playout.delay, self.playout.pace, std::mem::take(&mut self.teleports))
    }

    /// The frame and audio shown at the last local tick, while frames arrive.
    pub(super) fn shown(&self) -> Option<&Shown> {
        self.shown.as_ref().filter(|_| !self.stale)
    }

    /// The posed skater for the tick's visual snapshot: palette, materials, hips, bounds and
    /// render opacity.
    pub(super) fn visual(&self) -> Option<crate::SelectedSkaterVisual<'_>> {
        let restore = self.restore.as_ref()?;
        let shown = self.shown()?;
        Some(crate::SelectedSkaterVisual {
            name_tag: None,
            palette: &restore.palette,
            materials: Arc::clone(shown.frame.materials.as_ref()?),
            hips_position: restore.hips_position,
            world_bounds: restore.bounds,
            opacity: restore.opacity.opacity,
            ghosted: restore.opacity.ghosted,
            entity_type: 0x122,
            players_recipe: false,
            lobby_ghost: false,
        })
    }

    /// The owner's game camera as its skater shows now.
    pub(super) const fn camera(&self) -> Option<RemoteCamera> {
        self.camera
    }

    /// The hips of the skater shown, for distances and the camera's subject.
    pub(super) fn hips(&self) -> Option<[f32; 3]> {
        self.shown().and(self.restore.as_ref()).map(|restore| restore.hips_position)
    }

    /// The root of the skater shown, as `SkaterPresentation::GetSkaterRoot` reads it.
    pub(super) fn root(&self) -> Option<[f32; 3]> {
        self.shown().map(|shown| shown.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// Every tick of a sender's stream in runs of `interval`, each arriving whole on the local
    /// tick `arrival` gives its last tick, shown each local tick to `until`: the moment shown and
    /// the newest sample arrived. A run's last tick is a sample, and with `every` all its ticks.
    fn play(interval: u32, every: bool, until: u32, arrival: impl Fn(u32) -> u32) -> Vec<(f64, u32)> {
        let mut skater = RemoteSkater::new(1, 1);
        let mut frames: Vec<(u32, u32)> = (0..until).map(|tick| (arrival(tick.div_ceil(interval) * interval), tick)).collect();
        frames.sort_unstable();
        let mut frames = frames.into_iter().peekable();
        let mut newest: Option<u32> = None;
        let mut shown = Vec::new();
        for local in 0..until {
            while let Some((_, tick)) = frames.next_if(|&(at, _)| at <= local) {
                let mut frame = posed(tick, 0.0);
                if every || tick % interval == 0 {
                    newest = newest.max(Some(tick));
                } else {
                    frame.pose = None;
                }
                skater.receive(local, frame, true, tick % interval == 0);
            }
            if let (Some(time), Some(newest)) = (skater.playout.next(local), newest) {
                shown.push((time, newest));
            }
        }
        shown
    }

    #[test]
    fn playout_follows_the_late_arrivals_without_stopping_or_jumping() {
        let mut playout = Playout::default();
        // Each frame arrives ten ticks after its tick, every fifth three later still, until from
        // frame 140 on every fifth arrives nine later.
        let arrival = |remote: u32| remote + 10 + if remote % 5 == 0 { if remote < 140 { 3 } else { 9 } } else { 0 };
        let mut previous: Option<f64> = None;
        for local in 0..900_u32 {
            for remote in local.saturating_sub(20)..local {
                if arrival(remote) == local {
                    playout.arrived(local, remote);
                }
            }
            let Some(shown) = playout.next(local) else { continue };
            assert!(playout.next(local).is_some_and(|again| near(again, shown)), "one moment a local tick");
            if local == 120 {
                assert!((shown - (120.0 - 13.0 - DELAY_MARGIN)).abs() < 0.05, "settles at the late arrivals' delay: {shown}");
            }
            if let Some(previous) = previous.filter(|_| local > 40) {
                let step = shown - previous;
                assert!((0.89..1.11).contains(&step), "tick {local} moved {step}");
            }
            previous = Some(shown);
        }
        // Later frames widened the delay: the skater slowed down, then kept pace.
        let shown = previous.unwrap();
        assert!((shown - (899.0 - 19.0 - DELAY_MARGIN)).abs() < 0.25, "keeps pace at the new delay: {shown}");
    }

    #[test]
    fn a_slower_sender_is_shown_at_its_own_even_pace() {
        // A game at 50 ticks a second: each of its ticks takes 1.2 of this console's.
        let shown = play(2, false, 3000, |sample| sample * 6 / 5 + 8 + sample % 7 % 4);
        let late: Vec<&(f64, u32)> = shown[1000..].iter().filter(|&&(time, newest)| time >= f64::from(newest)).collect();
        assert!(late.is_empty(), "{} ticks held at the newest sample", late.len());
        for pair in shown[1000..].windows(2) {
            let step = pair[1].0 - pair[0].0;
            assert!((step / (5.0 / 6.0) - 1.0).abs() < 0.02, "moved {step}");
        }
    }

    #[test]
    fn a_faster_sender_is_shown_at_its_own_even_pace() {
        // This console at 55 ticks a second: each of its ticks takes 12/11 of the sender's, whose
        // frames run out after local tick 2750.
        let shown = play(2, true, 3000, |sample| sample * 11 / 12 + 6 + sample % 5 % 3);
        let late = shown[1000..2600].iter().filter(|&&(time, newest)| time >= f64::from(newest)).count();
        assert_eq!(late, 0, "ticks held at the newest sample");
        for pair in shown[1000..2600].windows(2) {
            let step = pair[1].0 - pair[0].0;
            assert!((step / (12.0 / 11.0) - 1.0).abs() < 0.02, "moved {step}");
        }
    }

    #[test]
    fn a_watched_stream_keeps_its_run_allowance() {
        // Every tick a sample, the runs two ticks long.
        let mut skater = RemoteSkater::new(1, 1);
        for tick in 0..600_u32 {
            let run_end = tick % 2 == 0;
            let arrival = tick.div_ceil(2) * 2 + 7;
            skater.receive(arrival, posed(tick, 0.0), true, run_end);
        }
        assert_eq!(skater.playout.allowance, 2);
        let shown = play(2, true, 2400, |sample| sample + 7 + sample % 9 % 4);
        let late = shown[600..].iter().filter(|&&(time, newest)| time >= f64::from(newest)).count();
        assert_eq!(late, 0, "ticks held at the newest sample");
        for pair in shown[600..].windows(2) {
            assert!((pair[1].0 - pair[0].0 - 1.0).abs() < 0.02, "moved {}", pair[1].0 - pair[0].0);
        }
    }

    #[test]
    fn a_hitch_of_this_consoles_ticks_leaves_the_pace_alone() {
        // Every three seconds this console's ticks stop for 24 of the sender's, and the frames
        // that came meanwhile are stamped with the tick it goes on with, which runs first.
        let shown = play(2, true, 3600, |sample| {
            let due = sample + 8;
            if due % 180 < 24 { due - due % 180 } else { due }
        });
        let late = shown[600..].iter().filter(|&&(time, newest)| time >= f64::from(newest)).count();
        assert_eq!(late, 0, "ticks held at the newest sample");
        let uneven = shown[600..].windows(2).filter(|pair| (pair[1].0 - pair[0].0 - 1.0).abs() > 0.05).count();
        assert!(uneven * 100 < shown.len() - 600, "{uneven} uneven steps");
    }

    #[test]
    fn a_link_that_spikes_meets_each_spike_with_the_delay_the_last_set() {
        // Every ten seconds the runs of a quarter second come twelve ticks later.
        let spike = |sample: u32| if sample % 600 >= 300 && sample % 600 < 315 { 12 } else { 0 };
        let shown = play(2, false, 3600, |sample| sample + 8 + sample % 5 % 3 + spike(sample));
        let first = shown.iter().position(|&(_, newest)| newest > 330).unwrap();
        let held = shown[first + 60..].iter().filter(|&&(time, newest)| time >= f64::from(newest)).count();
        assert_eq!(held, 0, "held at the newest sample at a spike after the first");
    }

    #[test]
    fn frames_while_this_console_is_held_start_the_arrivals_afresh() {
        // This console's ticks stop for 90 of the sender's after local tick 600 (a travel's
        // fade), the frames meanwhile kept but not counted; then its clock goes on 90 behind.
        let mut skater = RemoteSkater::new(1, 1);
        let mut newest = 0;
        let mut after = Vec::new();
        for remote in 0..1800_u32 {
            let arrival = remote + 8;
            let held = (600..690).contains(&arrival);
            let local = if arrival < 600 { arrival } else if held { 600 } else { arrival - 90 };
            let mut frame = posed(remote, 0.0);
            if remote % 2 != 0 {
                frame.pose = None;
            } else {
                newest = remote;
            }
            skater.receive(local, frame, !held, remote % 2 == 0);
            if remote % 2 == 0 && !held {
                let time = skater.playout.next(local);
                if let Some(time) = time.filter(|_| local > 720) {
                    after.push((time, newest, skater.playout.delay));
                }
            }
        }
        assert!(after.iter().all(|&(time, newest, _)| time < f64::from(newest)), "held after this console's ticks went on");
        assert!(after.iter().all(|&(_, _, delay)| delay < 20.0), "a delay the held frames widened");
    }

    #[test]
    fn a_stream_that_stops_resumes_where_it_stopped() {
        // The sender's ticks stop for 90 of this console's after tick 600, as a travel's do.
        let shown = play(2, false, 1400, |sample| (if sample <= 600 { sample } else { sample + 90 }) + 8 + sample % 3);
        // From a second after it resumes the skater moves on between its samples again.
        let resumed = shown.iter().position(|&(_, newest)| newest > 600).unwrap();
        let held = shown[resumed + 60..].iter().filter(|&&(time, newest)| time >= f64::from(newest)).count();
        assert_eq!(held, 0, "held at the newest sample after the stream resumed");
        for pair in shown[resumed..].windows(2) {
            assert!(pair[1].0 - pair[0].0 < 2.5, "a jump after the stream resumed");
        }
    }

    #[test]
    fn a_lost_run_is_bridged_by_the_samples_around_it() {
        let mut skater = RemoteSkater::new(1, 1);
        // Runs of three ticks, the last of each a sample, arrive together two ticks after it.
        let mut newest_sample = 0;
        for run in 0..40_u32 {
            let last = run * 3 + 2;
            let lost = run == 30;
            for tick in run * 3..=last {
                if !lost {
                    let mut frame = posed(tick, 0.0);
                    if tick != last {
                        frame.pose = None;
                    }
                    skater.receive(last + 2, frame, true, tick == last);
                }
            }
            if !lost {
                newest_sample = last;
            }
            for local in last + 2..last + 5 {
                if let Some(shown) = skater.playout.next(local) {
                    assert!(shown <= f64::from(newest_sample), "tick {local} shows {shown}, past the newest sample {newest_sample}");
                }
            }
        }
        assert_eq!(skater.playout.allowance, 3);
    }

    #[test]
    fn a_hold_keeps_the_newest_frames_and_stamped_ones_stay_bounded() {
        let mut skater = RemoteSkater::new(1, 1);
        // Half a minute of a sender's frames while this console's ticks are held.
        for tick in 0..1800 {
            skater.receive(0, posed(tick, 0.0), false, tick % 3 == 2);
        }
        assert_eq!(skater.frames.len(), KEPT_FRAMES);
        assert_eq!(skater.frames.keys().next(), Some(&(1800 - u32::try_from(KEPT_FRAMES).unwrap())));
        assert_eq!(skater.frames.keys().next_back(), Some(&1799));
        for tick in 0..10_000 {
            skater.receive(1, posed(u32::MAX - tick, 0.0), true, false);
        }
        assert_eq!(skater.frames.len(), KEPT_FRAMES);
    }

    fn frame(tick: u32, camera: Vec<f32>) -> NetFrame {
        NetFrame { tick, header: [0; HEADER_WORDS], pose: None, twist: [0.0; 4], camera, audio_floats: Vec::new(), audio_words: Vec::new(), hud: Vec::new() }
    }

    /// A frame with its root and one bone at `x`.
    fn posed(tick: u32, x: f32) -> NetFrame {
        let mut posed = frame(tick, Vec::new());
        posed.header[ROOT_WORD + 3] = 1.0_f32.to_bits();
        posed.header[ROOT_WORD + 4] = x.to_bits();
        posed.pose = Some(vec![[1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, x, 0.0, 0.0, 0.0]]);
        posed
    }

    fn root_x(skater: &RemoteSkater, time: f64) -> f32 {
        let (before, after) = skater.around(time).unwrap();
        f32::from_bits(header_at(before, after)[ROOT_WORD + 4])
    }

    #[test]
    fn the_trick_hud_passes_each_record_once_as_the_moment_shown_reaches_it() {
        let mut skater = RemoteSkater::new(1, 0);
        let mut recorder = crate::online::hud_record::HudRecorder::default();
        for tick in 0..12 {
            if tick == 5 {
                recorder.record(&crate::ui::trick_backend::PhysicsFrame { sequence_begun: true, ..Default::default() });
            }
            let mut frame = frame(tick, Vec::new());
            frame.hud = recorder.words();
            if tick != 7 {
                skater.receive(tick, frame, true, true);
            }
        }
        let mut passed = |through: u32| {
            skater.pass_hud(through);
            skater.take_hud().0.len()
        };
        assert_eq!(passed(3), 1, "the first record shown");
        assert_eq!(passed(8), 4, "4, 5, 6 and 8: the lost 7 passes nothing");
        assert_eq!(passed(8), 0, "a moment that holds passes nothing");
        assert_eq!(passed(6), 0, "nor one that moves back");
        assert_eq!(passed(11), 3);
        // Far back: the sender's ticks started again, and the records start over from there.
        assert_eq!(passed(11_u32.wrapping_sub(HUD_REWIND + 1)), 0);
        assert!(skater.take_hud().1.is_none());
    }

    #[test]
    fn a_lost_frame_is_filled_from_the_frames_around_it() {
        let mut skater = RemoteSkater::new(1, 1);
        skater.frames.insert(10, posed(10, 0.0));
        skater.frames.insert(12, posed(12, 2.0));
        assert!((root_x(&skater, 11.0) - 1.0).abs() < 1e-5, "the root halfway");
        assert!((skater.pose(11.0).unwrap()[0].translation[0] - 1.0).abs() < 1e-5, "the pose halfway");
        assert!((root_x(&skater, 10.5) - 0.5).abs() < 1e-5, "between ticks");
        assert!((root_x(&skater, 12.5) - 2.0).abs() < 1e-5, "held past the newest frame");
        let mut teleported = posed(12, 9.0);
        teleported.header[FLAGS_WORD] = TELEPORT_FLAG;
        skater.frames.insert(12, teleported);
        assert!(root_x(&skater, 11.0).abs() < 1e-5, "no blend across a teleport");
    }

    #[test]
    fn the_owner_camera_blends_between_its_samples() {
        let mut skater = RemoteSkater::new(1, 1);
        let turned = (0.25_f32 * std::f32::consts::PI).sin();
        skater.frames.insert(10, frame(10, vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 50.0]));
        skater.frames.insert(11, frame(11, Vec::new()));
        skater.frames.insert(12, frame(12, vec![2.0, 1.0, 4.0, 0.0, turned, 0.0, turned, 60.0]));
        let halfway = skater.camera_at(11.0).unwrap();
        assert!((halfway.fov - 55.0).abs() < 1e-4);
        assert!((halfway.matrix[12] - 1.0).abs() < 1e-4 && (halfway.matrix[14] - 2.0).abs() < 1e-4);
        let held = skater.camera_at(13.0).unwrap();
        assert!((held.fov - 60.0).abs() < 1e-4, "no later sample yet");
        let mut teleported = frame(12, vec![9.0, 1.0, 9.0, 0.0, 0.0, 0.0, 1.0, 50.0]);
        teleported.header[FLAGS_WORD] = TELEPORT_FLAG;
        skater.frames.insert(12, teleported);
        assert!((skater.camera_at(11.0).unwrap().matrix[12]).abs() < 1e-4, "no blend across a teleport");
    }
}
