//! Contest crowds: `Sk8::Challenge::cChallengeCrowdManager` (an activity sub-controller,
//! registered between the callout and scoring managers), its generators
//! (`CrowdGenerator::cCrowdPositionAndGenerationState`, each with a `cGridPositionGenerator`), the
//! `cCrowdPositionBuffer` the presentation draws the crowd from (`cMsgCrowdBufferChanged`), the
//! `Crowd` and `Crowds` natives, and the living world's spectator waypoints.
//!
//! A generator waits for its trigger volume's instance, lays a jittered grid over the volume's
//! bounding box (the Simulation's random), finds each point's ground along a vertical line through
//! the box (`PreUpdateSubmit` submits the batch, the next `Update` reads it), then adds its points
//! to the buffer. The buffer dispatches once every generator has either sent its points or still
//! waits for its volume, and only when its size changed.

use super::{
    attrib_lua::Pointee,
    binding::{get_float_or, get_int, get_int_or, int},
    characters::{self, LivingWorldOp},
    host::Host,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::{Output, World},
};
use crate::native_skater_random::SimulationCommandRandom;

type R = LuaResult<Vec<Value>>;

/// `challenge_local_data` `LargeCrowdData`: the activity's `tLargeCrowd`s.
const LARGE_CROWD_DATA: u64 = 0xcbcf_6323_d300_1851;
/// `challenge_local_data` `LargeCrowdPopulationBreakdown`: its first eight bytes name the
/// `livingworld_categorygroups` collection the crowd's models come from.
const LARGE_CROWD_POPULATION_BREAKDOWN: u64 = 0xdcc1_cc2c_efbd_33be;
/// `cCrowdPositionBuffer`'s breakdown until one is set: `Attrib::StringToKey("default")`.
const DEFAULT_BREAKDOWN: u64 = 0xd7ed_bd36_2d7d_2152;
/// `cCrowdPositionBuffer`'s `fixed_vector`s: the crowd's positions and its volumes' centres.
pub const MAX_CROWD_POSITIONS: usize = 200;
const MAX_VOLUME_CENTERS: usize = 20;
/// `UpdateLocationGeneration` asks the grid generator for at most this many points
/// (`GetGeneratedPoints(100, ...)`); it stops only once it holds more.
const MAX_GENERATED_POINTS: usize = 100;
/// `kMinDistanceBetweenPeopleGrid` and `kPercentOfStepToJitter` (`Challenge_Crowd` cvars) at
/// their registered values.
const MIN_DISTANCE_BETWEEN_PEOPLE: f32 = f32::from_bits(0x3f61_47ae);
const PERCENT_OF_STEP_TO_JITTER: f32 = 0.25;
/// `cGridPositionGenerator`'s constructor takes this off each of the volume's half extents.
const GRID_INSET: f32 = 0.25;
/// `SubmitLineTests` acquires a batch only while `UsedLineBatches` is at most this.
const MAX_USED_LINE_BATCHES: i32 = 5;
/// `SubmitLineTests`' line fatness.
const LINE_FATNESS: f32 = 0.5;
/// `tLargeCrowd` (24 bytes): the trigger volume's ID at +8, the density at +16, the generator
/// type at +20.
const LARGE_CROWD_BYTES: usize = 24;
/// `PointNegGraphData16` (144 bytes): the start time and value, the time limit, the points goal,
/// then sixteen times and sixteen values.
const CROWD_GRAPH_BYTES: usize = 144;

/// `eLocation_Generation_State_Info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GenerationState {
    WaitingForVolumeToStream,
    WaitingToCreateLocations,
    CreatingLocations,
    SubmittingLineTests,
    LocationsReady,
    LocationsSent,
}

/// `tGeneratedPosition`: a crowd member's position and the line its ground is found along.
#[derive(Clone, Copy, Debug, PartialEq)]
struct GeneratedPosition {
    position: [f32; 4],
    line_min: [f32; 4],
    line_max: [f32; 4],
}

/// `cGridPositionGenerator`: the volume's half extents less [`GRID_INSET`] and its world matrix
/// (the bounding volume's transform composed with the trigger instance's), by rows.
#[derive(Clone, Debug, PartialEq)]
struct GridGenerator {
    density: f32,
    dimensions: [f32; 4],
    volume: [[f32; 4]; 4],
}

/// The volume's matrix applied to a point: `vmaddfp` row by row from the translation.
fn transform(volume: &[[f32; 4]; 4], point: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let x = volume[0][lane].mul_add(point[0], volume[3][lane]);
        let y = volume[1][lane].mul_add(point[1], x);
        volume[2][lane].mul_add(point[2], y)
    })
}

/// A grid size: `fctidz` of the cells' count, of which the low word is kept, at least one.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "fctidz then the low word, as the native")]
fn cells(count: f32) -> u32 {
    ((count as i64) as u32).max(1)
}

impl GridGenerator {
    /// `cGridPositionGenerator::cGridPositionGenerator` (`0x82da1ff8`).
    fn new(density: f32, dimensions: [f32; 4], volume: [[f32; 4]; 4]) -> Self {
        let inset = [GRID_INSET, GRID_INSET, GRID_INSET, 0.0];
        Self { density, dimensions: std::array::from_fn(|lane| dimensions[lane] - inset[lane]), volume }
    }

    /// `cGridPositionGenerator::GetGeneratedPoints` (`0x82dade58`): the volume's X and Z split
    /// into cells at least `kMinDistanceBetweenPeopleGrid / density` across, a point in each cell's
    /// middle jittered by up to `kPercentOfStepToJitter` of a cell (X first, then Z), with the line
    /// through it between the box's bottom and top faces, all in world space; column by column
    /// until the list holds more than `max`.
    #[expect(clippy::cast_precision_loss, reason = "the native's unsigned to float conversions")]
    fn generated_points(&self, max: usize, list: &mut Vec<GeneratedPosition>, random: &mut SimulationCommandRandom) {
        let width = self.dimensions[0] * 2.0;
        let step = MIN_DISTANCE_BETWEEN_PEOPLE / self.density;
        let inverse = 1.0 / step;
        let columns = cells(inverse * width);
        let depth = self.dimensions[2] * 2.0;
        let rows = cells(inverse * depth);
        let x_count = columns as f32;
        let z_count = rows as f32;
        let x_step = -x_count.mul_add(step, -width) / x_count + step;
        let z_step = -z_count.mul_add(step, -depth) / z_count + step;
        let (x_half, z_half) = (x_step * 0.5, z_step * 0.5);
        let (left, back) = (self.dimensions[0] * -1.0, self.dimensions[2] * -1.0);
        let jitter = PERCENT_OF_STEP_TO_JITTER;
        for column in 0..columns {
            let x_base = (column as f32).mul_add(x_step, x_half) + left;
            for row in 0..rows {
                let x = x_base + random.range(-jitter, jitter) * x_step;
                let z = random.range(-jitter, jitter).mul_add(z_step, (row as f32).mul_add(z_step, z_half) + back);
                let point = [x, 0.0, z, 0.0];
                let height = self.dimensions[1];
                list.push(GeneratedPosition {
                    position: transform(&self.volume, point),
                    line_min: transform(&self.volume, [x, -height, z, 0.0]),
                    line_max: transform(&self.volume, [x, height, z, 0.0]),
                });
                if list.len() > max {
                    return;
                }
            }
        }
    }
}

/// A submitted crowd line batch, answered by the session in submission order.
#[derive(Clone, Debug, PartialEq)]
struct CrowdBatch {
    serial: u32,
    /// One result per test the batch took: `None` without a hit.
    results: Option<Vec<Option<[f32; 4]>>>,
}

/// `CrowdGenerator::cCrowdPositionAndGenerationState` (0x40 bytes).
#[derive(Clone, Debug, PartialEq)]
struct CrowdGenerator {
    state: GenerationState,
    /// `LineBatchStartIndex`: the first position whose ground is not found yet.
    line_start: usize,
    /// `TriggerVolumeID`, kept as the float it was converted to.
    volume: f32,
    density: f32,
    batch: Option<CrowdBatch>,
    generator: Option<GridGenerator>,
    positions: Vec<GeneratedPosition>,
    content_loaded: bool,
    /// The `ePosition_Generator_Type` `SetupSpawnPoints` stores; the grid is made either way.
    generator_type: i32,
}

impl CrowdGenerator {
    fn new(volume: f32, density: f32, generator_type: i32) -> Self {
        Self {
            state: GenerationState::WaitingForVolumeToStream,
            line_start: 0,
            volume,
            density,
            batch: None,
            generator: None,
            positions: Vec::new(),
            content_loaded: false,
            generator_type,
        }
    }
}

/// `cCrowdPositionBuffer`: the crowd's positions and volume centres, and the population
/// breakdown its models come from.
#[derive(Clone, Debug, PartialEq)]
pub struct CrowdPositionBuffer {
    positions: Vec<[f32; 4]>,
    centers: Vec<[f32; 4]>,
    size_at_last_dispatch: usize,
    population_breakdown: u64,
}

impl Default for CrowdPositionBuffer {
    fn default() -> Self {
        Self { positions: Vec::new(), centers: Vec::new(), size_at_last_dispatch: 0, population_breakdown: DEFAULT_BREAKDOWN }
    }
}

impl CrowdPositionBuffer {
    /// `GetNumCrowdPositions` / `GetCrowdPosition`.
    #[must_use]
    pub fn positions(&self) -> &[[f32; 4]] {
        &self.positions
    }

    /// `GetNumCrowdCenters` / `GetCrowdCenter`.
    #[must_use]
    pub fn centers(&self) -> &[[f32; 4]] {
        &self.centers
    }

    /// `GetPopulationBreakdown`.
    #[must_use]
    pub fn population_breakdown(&self) -> u64 {
        self.population_breakdown
    }

    /// `AddCrowdPosition` (`0x825be6f8`): kept while the buffer has room.
    fn add_position(&mut self, position: [f32; 4]) {
        if self.positions.len() < MAX_CROWD_POSITIONS {
            self.positions.push(position);
        }
    }

    /// `AddCrowdVolumeCenter` (`0x825be770`).
    fn add_volume_center(&mut self, center: [f32; 4]) {
        if self.centers.len() < MAX_VOLUME_CENTERS {
            self.centers.push(center);
        }
    }

    /// `DispatchCrowdPositions` (`0x825b9c60`): `cMsgCrowdBufferChanged` when the crowd's size
    /// changed since the last.
    fn dispatch(&mut self) -> bool {
        if self.size_at_last_dispatch == self.positions.len() {
            return false;
        }
        self.size_at_last_dispatch = self.positions.len();
        true
    }
}

/// `cChallengeCrowdManager`: the crowds by activity (`CrowdsList`, a multimap ordered by the
/// activity, equal activities in insertion order) and the buffer.
#[derive(Clone, Debug, Default)]
pub struct CrowdManager {
    crowds: Vec<(usize, CrowdGenerator)>,
    pub buffer: CrowdPositionBuffer,
    /// `cCrowdPositionAndGenerationState::UsedLineBatches`; a generator deleted holding a batch
    /// releases it without counting it back, as the native destructor does.
    used_line_batches: i32,
    next_serial: u32,
}

/// `ResetBuffer(dispatch)` (`0x825b69d0`).
fn reset_buffer(world: &mut World, dispatch: bool) {
    world.crowds.buffer.positions.clear();
    world.crowds.buffer.centers.clear();
    if dispatch {
        dispatch_buffer(world);
    }
}

fn dispatch_buffer(world: &mut World) {
    if world.crowds.buffer.dispatch() {
        world.output(Output::CrowdBufferChanged);
    }
}

/// `ResubmitPositionsToBuffer` (`0x82da3b08`): a generator that sent its positions adds them again.
fn resubmit(generator: &CrowdGenerator, buffer: &mut CrowdPositionBuffer) {
    if generator.state == GenerationState::LocationsSent {
        for position in &generator.positions {
            buffer.add_position(position.position);
        }
    }
}

fn resubmit_all(world: &mut World) {
    let CrowdManager { crowds, buffer, .. } = &mut world.crowds;
    for (_, generator) in crowds.iter() {
        resubmit(generator, buffer);
    }
}

/// `SetupSpawnPoints` (`0x82db0438`): a new generator for the activity, after the activity's
/// others.
fn setup_spawn_points(world: &mut World, activity: usize, volume: f32, density: f32, generator_type: i32) {
    let crowds = &mut world.crowds.crowds;
    let at = crowds.partition_point(|&(other, _)| other <= activity);
    crowds.insert(at, (activity, CrowdGenerator::new(volume, density, generator_type)));
}

/// A `tTriggerVolumeInstanceID`'s 64-bit ID as `LoadCrowd` passes it on: `__u64tod`, then `frsp`.
fn volume_float(volume: u64) -> f32 {
    crate::skate3::id_float(volume)
}

/// A `tLargeCrowd`: the volume (as the float the generators keep), density and generator type.
fn large_crowd(bytes: &[u8]) -> (f32, f32, i32) {
    let word = |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let volume = (u64::from(word(8)) << 32) | u64::from(word(12));
    (volume_float(volume), f32::from_bits(word(16)), word(20).cast_signed())
}

/// The activity's `LargeCrowdData`.
fn local_large_crowds(world: &World, activity: usize) -> Vec<(f32, f32, i32)> {
    let Some((class, collection)) = world.activities.activities[activity].local_data else { return Vec::new() };
    let Some(attribute) = world.db.attribute(class, collection, LARGE_CROWD_DATA) else { return Vec::new() };
    (0..attribute.len())
        .map(|index| {
            let bytes = attribute
                .data(index)
                .and_then(|location| world.db.bytes(location, LARGE_CROWD_BYTES))
                .map_or_else(|| vec![0; LARGE_CROWD_BYTES], <[u8]>::to_vec);
            large_crowd(&bytes)
        })
        .collect()
}

/// `LoadCrowd(activity)` (`0x82db1aa0`): with any `LargeCrowdData`, the buffer emptied, a
/// generator per crowd, then every sent generator's positions again.
fn load_activity_crowds(world: &mut World, activity: usize) {
    let crowds = local_large_crowds(world, activity);
    if crowds.is_empty() {
        return;
    }
    reset_buffer(world, false);
    for (volume, density, generator_type) in crowds {
        setup_spawn_points(world, activity, volume, density, generator_type);
    }
    resubmit_all(world);
}

/// `RunMainActivity` (`0x82db1c80`): the activity's crowds, then `SetPopulationBreakdown` with its
/// `LargeCrowdPopulationBreakdown` (zero without one, `Attrib::DefaultDataArea`).
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    load_activity_crowds(world, activity);
    let breakdown = world.activities.activities[activity]
        .local_data
        .and_then(|(class, collection)| world.db.attribute(class, collection, LARGE_CROWD_POPULATION_BREAKDOWN))
        .and_then(|attribute| attribute.u64(0))
        .unwrap_or(0);
    world.crowds.buffer.population_breakdown = breakdown;
}

/// Removes the activity's generators that `matches` picks, each other generator's positions added
/// again in order; the buffer is dispatched when any went.
fn unload(world: &mut World, activity: usize, matches: impl Fn(&CrowdGenerator) -> bool) {
    reset_buffer(world, false);
    let CrowdManager { crowds, buffer, .. } = &mut world.crowds;
    let mut removed = false;
    crowds.retain(|(owner, generator)| {
        if *owner == activity && matches(generator) {
            removed = true;
            false
        } else {
            resubmit(generator, buffer);
            true
        }
    });
    if removed {
        dispatch_buffer(world);
    }
}

/// `ShutdownActivity` (`0x82daa088`), `UnLoadCrowd(activity)` (`0x82da9f60`): only an activity
/// with `LargeCrowdData` loses its generators, whichever way they were made.
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    if local_large_crowds(world, activity).is_empty() {
        return;
    }
    unload(world, activity, |_| true);
}

/// `UpdateLocationGeneration` (`0x82da52d0`): a streamed volume's grid made (its centre added to
/// the buffer); an instance without a bounding volume gives no positions.
fn update_location_generation(
    generator: &mut CrowdGenerator,
    host: &dyn Host,
    buffer: &mut CrowdPositionBuffer,
    random: Option<&mut SimulationCommandRandom>,
    used_line_batches: &mut i32,
) {
    match generator.state {
        GenerationState::WaitingForVolumeToStream => match host.crowd_volume(generator.volume) {
            None => {}
            Some(Some((matrix, dimensions))) => {
                let volume: [[f32; 4]; 4] = std::array::from_fn(|row| std::array::from_fn(|lane| matrix[row * 4 + lane]));
                buffer.add_volume_center(volume[3]);
                generator.generator = Some(GridGenerator::new(generator.density, dimensions, volume));
                generator.content_loaded = true;
                generator.batch = None;
                generator.state = GenerationState::WaitingToCreateLocations;
            }
            Some(None) => {
                generator.state = GenerationState::LocationsReady;
                generator.positions.clear();
            }
        },
        GenerationState::WaitingToCreateLocations => {
            generator.positions.clear();
            generator.state = GenerationState::CreatingLocations;
        }
        // IsFinishedGenerating (0x826c6d80) is always true.
        GenerationState::CreatingLocations => {
            let (Some(grid), Some(random)) = (generator.generator.as_ref(), random) else { return };
            grid.generated_points(MAX_GENERATED_POINTS, &mut generator.positions, random);
            generator.state = GenerationState::SubmittingLineTests;
        }
        GenerationState::SubmittingLineTests => {
            sync_line_batch(generator, used_line_batches);
            if generator.line_start >= generator.positions.len() {
                generator.state = GenerationState::LocationsReady;
            }
        }
        GenerationState::LocationsReady | GenerationState::LocationsSent => {}
    }
}

/// `SyncLineBatch` (`0x82da3988`): the answered batch's hits become the positions in order,
/// up to its first miss (that position is tested again with the next batch); the batch released.
fn sync_line_batch(generator: &mut CrowdGenerator, used_line_batches: &mut i32) {
    let Some(results) = generator.batch.as_ref().and_then(|batch| batch.results.as_ref()) else { return };
    for result in results {
        let Some(position) = result else { break };
        if let Some(slot) = generator.positions.get_mut(generator.line_start) {
            slot.position = *position;
        }
        generator.line_start += 1;
    }
    generator.batch = None;
    *used_line_batches -= 1;
}

/// `cChallengeCrowdManager::Update` (`0x82da5588`): each generator's `UpdateLocationGeneration`
/// in order, a ready one's positions sent (`SendPositionsToBuffer` `0x82da3a88`); then the buffer
/// dispatched if every generator waits for its volume or has sent its positions.
pub(crate) fn update(world: &mut World) {
    let World { crowds, host, random, .. } = &mut *world;
    let CrowdManager { crowds: generators, buffer, used_line_batches, .. } = crowds;
    let mut settled = 0;
    for (_, generator) in generators.iter_mut() {
        update_location_generation(generator, host.as_ref(), buffer, random.as_mut(), used_line_batches);
        if generator.state == GenerationState::LocationsReady {
            for position in &generator.positions {
                buffer.add_position(position.position);
            }
            generator.state = GenerationState::LocationsSent;
        }
        if matches!(generator.state, GenerationState::WaitingForVolumeToStream | GenerationState::LocationsSent) {
            settled += 1;
        }
    }
    if settled == generators.len() {
        dispatch_buffer(world);
    }
}

/// `cChallengeCrowdManager::PreUpdateSubmit` (`0x82da5530`), as `Simulation::SimUpdate1` runs it
/// before the next update: each generator finding its ground with no batch out
/// (`SubmitLineTests` `0x82da3810`) acquires one while at most [`MAX_USED_LINE_BATCHES`] are
/// out, and tests its untested positions from the box's top to its bottom.
pub(crate) fn pre_update_submit(world: &mut World) {
    let CrowdManager { crowds, used_line_batches, next_serial, .. } = &mut world.crowds;
    let mut submitted = Vec::new();
    for (_, generator) in crowds.iter_mut() {
        if generator.state != GenerationState::SubmittingLineTests
            || !generator.content_loaded
            || generator.line_start >= generator.positions.len()
            || generator.batch.is_some()
            || *used_line_batches > MAX_USED_LINE_BATCHES
        {
            continue;
        }
        *used_line_batches += 1;
        let serial = *next_serial;
        *next_serial = next_serial.wrapping_add(1);
        generator.batch = Some(CrowdBatch { serial, results: None });
        let tests = generator.positions[generator.line_start..]
            .iter()
            .map(|position| (position.line_max, position.line_min, LINE_FATNESS))
            .collect();
        submitted.push(LivingWorldOp::CrowdLineTests { serial, tests });
    }
    for op in submitted {
        characters::op(world, op);
    }
}

/// The session's answer to [`LivingWorldOp::CrowdLineTests`]: one result per test the batch
/// took, in order (`AddTest` stops at the first the batch refuses).
pub fn line_tests_answered(world: &mut World, serial: u32, results: Vec<Option<[f32; 4]>>) {
    let batch = world
        .crowds
        .crowds
        .iter_mut()
        .filter_map(|(_, generator)| generator.batch.as_mut())
        .find(|batch| batch.serial == serial && batch.results.is_none());
    if let Some(batch) = batch {
        batch.results = Some(results);
    }
}

/// The Vault struct a native's light userdata argument points at (`lua_touserdata(1)` of a
/// native that asserts it is light userdata): `Attrib::DefaultDataArea`'s zeros for a missing
/// value.
fn vault_struct(lua: &Lua<World>, world: &World, call: &NativeCall, native: &str, size: usize) -> LuaResult<Vec<u8>> {
    let Value::LightUserData(handle) = call.arg(0) else {
        return Err(lua.error(format!("{native}: expected light userdata")));
    };
    match world.pointers.get(*handle) {
        Some(Pointee::Attrib(location)) => world
            .db
            .bytes(location, size)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| lua.error(format!("{native}: Vault value out of range"))),
        Some(Pointee::DefaultData) => Ok(vec![0; size]),
        _ => Err(lua.error(format!("{native}: expected a Vault value"))),
    }
}

/// `Crowd.LoadLargeCrowd(tLargeCrowd)` (`0x828932c0`): `LoadCrowd(activity, crowd)`
/// (`0x82db1be8`): the buffer emptied, a generator for the crowd, then every sent generator's
/// positions again.
fn load_large_crowd(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let bytes = vault_struct(lua, world, call, "Crowd.LoadLargeCrowd", LARGE_CROWD_BYTES)?;
    let Some(activity) = world.closure else { return Ok(Vec::new()) };
    let (volume, density, generator_type) = large_crowd(&bytes);
    reset_buffer(world, false);
    setup_spawn_points(world, activity, volume, density, generator_type);
    resubmit_all(world);
    Ok(Vec::new())
}

/// `Crowd.UnloadLargeCrowd(tLargeCrowd)` (`0x828933c0`): `UnLoadCrowd(activity, crowd)`
/// (`0x82da8028`): the activity's generators on the crowd's volume deleted.
fn unload_large_crowd(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let bytes = vault_struct(lua, world, call, "Crowd.UnloadLargeCrowd", LARGE_CROWD_BYTES)?;
    let Some(activity) = world.closure else { return Ok(Vec::new()) };
    let (volume, ..) = large_crowd(&bytes);
    unload(world, activity, |generator| generator.volume == volume);
    Ok(Vec::new())
}

/// `fsel`: `b` when `a` is at least zero, else `c` (NaN takes `c`).
fn fsel(a: f32, b: f32, c: f32) -> f32 {
    if a >= 0.0 { b } else { c }
}

/// `Crowds.SpawnLargeCrowd(volumeID = -1, generatorType = 1, density = 1)` (`0x82db0518`): a
/// generator for the volume with the density clamped to 0..1 (`SetupSpawnPoints`, the buffer
/// untouched); a volume of -1 does nothing.
fn spawn_large_crowd(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let volume = get_float_or(call, 0, -1.0);
    let generator_type = get_int_or(call, 1, 1);
    let density = get_float_or(call, 2, 1.0);
    let floor = fsel(-density, 0.0, density);
    if volume != -1.0 && let Some(activity) = world.closure {
        setup_spawn_points(world, activity, volume, fsel(1.0 - floor, floor, 1.0), generator_type);
    }
    Ok(Vec::new())
}

/// A `PointNegGraphData16`'s floats.
fn crowd_graph(bytes: &[u8]) -> [f32; 36] {
    std::array::from_fn(|index| f32::from_be_bytes([bytes[index * 4], bytes[index * 4 + 1], bytes[index * 4 + 2], bytes[index * 4 + 3]]))
}

/// `Crowd.GetTimeLimit(graph)` (`0x8288e058`).
fn get_time_limit(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let graph = crowd_graph(&vault_struct(lua, world, call, "Crowd.GetTimeLimit", CROWD_GRAPH_BYTES)?);
    Ok(vec![Value::Number(graph[2])])
}

/// `Crowd.GetPointsGoal(graph)` (`0x8288e0c8`).
fn get_points_goal(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let graph = crowd_graph(&vault_struct(lua, world, call, "Crowd.GetPointsGoal", CROWD_GRAPH_BYTES)?);
    Ok(vec![Value::Number(graph[3])])
}

/// `Crowd.CheckCrowdGraph(graph)` (`0x8288e138`): the index of the first of the sixteen times
/// earlier than the one before it (the start time before the first), -1 when none is.
fn check_crowd_graph(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let graph = crowd_graph(&vault_struct(lua, world, call, "Crowd.CheckCrowdGraph", CROWD_GRAPH_BYTES)?);
    let mut previous = graph[0];
    for index in 0..16 {
        if graph[4 + index] < previous {
            #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap, reason = "an index below 16")]
            return Ok(vec![int(index as i32)]);
        }
        previous = graph[4 + index];
    }
    Ok(vec![int(-1)])
}

/// `Crowd.GetCrowdValueAtTime(graph, time)` (`0x8288e1e8`): the value at the integer time,
/// interpolated between the points around it (from the start point before the first). Past the
/// last point the native interpolates between time 2 and itself: NaN.
fn get_crowd_value_at_time(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let graph = crowd_graph(&vault_struct(lua, world, call, "Crowd.GetCrowdValueAtTime", CROWD_GRAPH_BYTES)?);
    let time = get_int(call, 1);
    #[expect(clippy::cast_precision_loss, reason = "the native's (float) conversion")]
    let at = time as f32;
    let after = (0..16).find(|&index| graph[4 + index] >= at);
    let value = match after {
        Some(0) => ((at - graph[0]) / (graph[4] - graph[0])).mul_add(graph[20] - graph[1], graph[1]),
        Some(after) => {
            let before = after - 1;
            let (start, end) = (graph[4 + before], graph[4 + after]);
            ((at - start) / (end - start)).mul_add(graph[20 + after] - graph[20 + before], graph[20 + before])
        }
        None => ((at - graph[2]) / (graph[2] - graph[2])).mul_add(graph[18] - graph[18], graph[18]),
    };
    Ok(vec![Value::Number(value)])
}

/// `tSpectatorSpawnInfo` (48 bytes): the waypoint's GUID (+8), `ForceSpawnPercentage` (+16) and
/// the `LivingWorldCategoryGroup` `RefSpec` (+24, its collection at +32).
const SPECTATOR_SPAWN_INFO_BYTES: usize = 48;

fn spectator_spawn_info(bytes: &[u8]) -> (u64, f32, u64) {
    let word = |at: usize| u64::from_be_bytes(bytes[at..at + 8].try_into().expect("eight bytes"));
    (word(8), f32::from_be_bytes(bytes[16..20].try_into().expect("four bytes")), word(32))
}

/// `LivingWorld.ActivateSpectatorWaypoint(tSpectatorSpawnInfo)` (`0x82893da8`): a waypoint group
/// found by the info's waypoint GUID (`ILWManager::GetWaypointByGUID`) that has waypoints gets
/// `WaypointMan::ActivateSpectatorWaypoint` (`0x82ce9180`): a census-managed cull, then the
/// group's share (`ForceSpawnPercentage`) of forced spectator spawns of the info's living world
/// category group, queued. A null info does nothing.
fn activate_spectator_waypoint(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if matches!(call.arg(0), Value::LightUserData(0)) {
        return Ok(Vec::new());
    }
    let bytes = vault_struct(lua, world, call, "LivingWorld.ActivateSpectatorWaypoint", SPECTATOR_SPAWN_INFO_BYTES)?;
    let (guid, force_spawn_percentage, category_group) = spectator_spawn_info(&bytes);
    let keep = world.characters.challenge_characters();
    characters::op(world, LivingWorldOp::ActivateSpectatorWaypoint { guid, force_spawn_percentage, category_group, keep });
    Ok(Vec::new())
}

/// `LivingWorld.DeactivateSpectatorWaypoint(tSpectatorSpawnInfo, forceCull = false)`
/// (`0x8288e488`): the group found by the info's waypoint GUID gets
/// `WaypointMan::DeactivateSpectatorWaypoint(group, true)` (`0x82ce8ed8`; the native reads
/// `forceCull` and passes `true`), which drops the group's queued activation and releases its
/// spectators. A null info does nothing.
fn deactivate_spectator_waypoint(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if matches!(call.arg(0), Value::LightUserData(0)) {
        return Ok(Vec::new());
    }
    let bytes = vault_struct(lua, world, call, "LivingWorld.DeactivateSpectatorWaypoint", SPECTATOR_SPAWN_INFO_BYTES)?;
    let (guid, ..) = spectator_spawn_info(&bytes);
    characters::op(world, LivingWorldOp::DeactivateSpectatorWaypoint { guid });
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Crowd", "GetTimeLimit", get_time_limit),
        ("Crowd", "GetPointsGoal", get_points_goal),
        ("Crowd", "CheckCrowdGraph", check_crowd_graph),
        ("Crowd", "GetCrowdValueAtTime", get_crowd_value_at_time),
        ("Crowd", "LoadLargeCrowd", load_large_crowd),
        ("Crowd", "UnloadLargeCrowd", unload_large_crowd),
        ("Crowds", "SpawnLargeCrowd", spawn_large_crowd),
        ("LivingWorld", "ActivateSpectatorWaypoint", activate_spectator_waypoint),
        ("LivingWorld", "DeactivateSpectatorWaypoint", deactivate_spectator_waypoint),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> [[f32; 4]; 4] {
        [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
    }

    #[test]
    fn the_grid_fills_the_inset_box_column_by_column_and_stops_past_its_bound() {
        let gate = crate::native_skater_random::NativeRandomGate::from_static_initializer();
        let mut random = SimulationCommandRandom::construct_field(gate);
        // Half extents 2.25 x 1.25 x 1.25: 4 x 2 m inside the inset, 4 x 2 cells of 1 m at
        // density 0.88.
        let grid = GridGenerator::new(MIN_DISTANCE_BETWEEN_PEOPLE, [2.25, 1.25, 1.25, 0.0], identity());
        let mut points = Vec::new();
        grid.generated_points(MAX_GENERATED_POINTS, &mut points, &mut random);
        assert_eq!(points.len(), 8);
        for (index, point) in points.iter().enumerate() {
            let (column, row) = (index / 2, index % 2);
            let (x, z) = (column as f32 - 1.5, row as f32 - 0.5);
            assert!((point.position[0] - x).abs() <= 0.25 && (point.position[2] - z).abs() <= 0.25, "{point:?}");
            assert_eq!(point.line_min[1], -1.0);
            assert_eq!(point.line_max[1], 1.0);
            assert_eq!(point.position[3], 1.0);
        }
        let mut bounded = Vec::new();
        let wide = GridGenerator::new(MIN_DISTANCE_BETWEEN_PEOPLE, [20.25, 1.25, 20.25, 0.0], identity());
        wide.generated_points(MAX_GENERATED_POINTS, &mut bounded, &mut random);
        assert_eq!(bounded.len(), MAX_GENERATED_POINTS + 1, "the native checks after each push");
    }

    #[test]
    fn a_miss_stops_the_batch_and_is_tested_again() {
        let mut generator = CrowdGenerator::new(1.0, 1.0, 1);
        generator.state = GenerationState::SubmittingLineTests;
        generator.positions = vec![GeneratedPosition { position: [0.0; 4], line_min: [0.0; 4], line_max: [0.0; 4] }; 3];
        generator.batch = Some(CrowdBatch { serial: 0, results: Some(vec![Some([1.0, 2.0, 3.0, 1.0]), None, Some([9.0; 4])]) });
        let mut used = 1;
        sync_line_batch(&mut generator, &mut used);
        assert_eq!(generator.line_start, 1);
        assert_eq!(generator.positions[0].position, [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(generator.positions[2].position, [0.0; 4]);
        assert!(generator.batch.is_none());
        assert_eq!(used, 0);
    }

    #[test]
    fn the_buffer_dispatches_only_a_new_size() {
        let mut buffer = CrowdPositionBuffer::default();
        assert!(!buffer.dispatch());
        buffer.add_position([1.0; 4]);
        assert!(buffer.dispatch());
        assert!(!buffer.dispatch());
        for _ in 0..MAX_CROWD_POSITIONS + 5 {
            buffer.add_position([0.0; 4]);
        }
        assert_eq!(buffer.positions().len(), MAX_CROWD_POSITIONS);
        assert_eq!(buffer.population_breakdown(), DEFAULT_BREAKDOWN);
    }
}
