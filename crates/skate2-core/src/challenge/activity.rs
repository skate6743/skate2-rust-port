//! `Sk8::Challenge::cActivityManager` (`packages/sk8/activitymanager`): one
//! activity per leaf `challenges` collection, the running main challenge, the
//! global activities (a location global runs while the local tester is inside
//! its box, a full global once its prerequisites pass; both stop while a main
//! challenge runs), and the reset list that shuts finished activities down on
//! the next update.

use super::{
    attrib_db::AttribDb,
    keys::{class, hash_value32},
    runtime::{self, StateGraph, field},
    system::{ChallengeSystem, Message, World},
};
use crate::grind_data_octree::OctreeBounds;
use std::collections::{HashMap, VecDeque};

/// `challenge_global_data.Global` (`eGlobalType`).
pub const GLOBAL_TYPE: u64 = 0x122d_b7b0_f026_dbec;
/// `challenge_global_data`: an activity only debug builds create (`kAllowDebugOnlyActivities`,
/// constructed false).
const DEBUG_ONLY: u64 = 0xfd40_e4a3_1027_c984;
/// `challenge_global_data`: the flag an online game (game type 8) needs.
const GAME_TYPE_8: u64 = 0xf9a9_a1dd_ed84_a643;
/// `challenge_global_data.Location` (`tLocationID`).
const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
/// `challenges`: a location box's half size.
const RADIUS: u64 = 0x8022_ba64_cb8e_431d;
/// `ActivityUtils::kDefaultLocationTrackingRadius`.
const DEFAULT_LOCATION_TRACKING_RADIUS: f32 = 40.0;
/// `kRunGlobalChallenges` and `kEnableGlobalTriggers` (`Challenge_ActivityManager` cvars, both
/// constructed true).
const RUN_GLOBAL_CHALLENGES: bool = true;
const ENABLE_GLOBAL_TRIGGERS: bool = true;

/// `eGlobalType` of a global activity.
pub mod global_type {
    /// Runs while the local tester is inside the box at its `Location`.
    pub const LOCATION: i32 = 1;
    /// Runs once its prerequisites pass.
    pub const FULL: i32 = 2;
}

/// `cActivityOctTree` (`cManagedOctTree<cActivity*, 512>`): the boxes up to `EntryFillIndex` (a
/// removed entry is `InvalidEntry`) and `FreeDataEntryIndexices`.
#[derive(Debug, Default)]
pub(crate) struct ActivityTree {
    entries: Vec<Option<(usize, OctreeBounds)>>,
    free: Vec<u32>,
}

impl ActivityTree {
    /// `Insert` (`0x825b4c40`): the most recently freed slot, else the next.
    pub(crate) fn insert(&mut self, activity: usize, bounds: OctreeBounds) {
        if let Some(index) = self.free.pop() {
            self.entries[index as usize] = Some((activity, bounds));
        } else {
            self.entries.push(Some((activity, bounds)));
        }
    }

    /// `Remove(data, previous, 1, nullptr)` (`0x825c0c38`, `0x825beb20`): the first entry holding
    /// the activity becomes `InvalidEntry`, its slot is freed and it leaves the previous
    /// collisions.
    pub(crate) fn remove(&mut self, activity: usize, previous: &mut Vec<u32>) {
        let Some(index) = self.entries.iter().position(|e| e.is_some_and(|(a, _)| a == activity)) else { return };
        self.entries[index] = None;
        let index = u32::try_from(index).expect("512 entries");
        self.free.push(index);
        if let Some(at) = previous.iter().position(|&i| i == index) {
            previous.remove(at);
        }
    }

    /// `GetCollisions(bbox)`: the entries whose boxes the box meets, in slot order.
    pub(crate) fn collisions(&self, bounds: OctreeBounds) -> Vec<u32> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_some_and(|(_, b)| b.overlaps(bounds)))
            .map(|(i, _)| u32::try_from(i).expect("512 entries"))
            .collect()
    }

    /// `GetDataEntry(index)` (`0x825b4a88`).
    pub(crate) fn entry(&self, index: u32) -> Option<usize> {
        self.entries.get(index as usize).copied().flatten().map(|(a, _)| a)
    }
}

/// `cActivity` plus its `cStateGraph` (one per activity).
#[derive(Debug)]
pub struct Activity {
    pub key: u64,
    /// `challenges.Name`: the vars registry key and the handler context.
    pub name: String,
    pub global_data: Option<(u64, u64)>,
    pub local_data: Option<(u64, u64)>,
    pub graph: StateGraph,
}

#[derive(Debug, Default)]
pub struct ActivityManager {
    pub activities: Vec<Activity>,
    /// `ActivityMapping`.
    index: HashMap<u64, usize>,
    pub main: Option<usize>,
    pending_reset: Vec<usize>,
    /// `GameLoaded` and `GameLoadFlags`.
    pub game_loaded: bool,
    pub game_load_flags: u32,
    /// `IsQuittingToFE`: set by `Sk8ChallengeSystem::QuittingToFE` (`0x825a2bf0`), cleared by
    /// `GameLoad` and `GameUnload`; `ReevaluatePrereqActivities` does nothing while it is set.
    pub is_quitting_to_fe: bool,
    /// `ClassifcationLists[0]`: the global activities (`Global` 1 and 2) `CreateActivity` kept, in
    /// creation order.
    globals: Vec<usize>,
    /// `GlobalActivitiesPassedPrereq`, ascending.
    global_passed: Vec<usize>,
    /// `GlobalActivityTree`: the location globals' boxes.
    global_tree: ActivityTree,
    /// `PreviousCollisionIndicies`: the boxes the local tester met last update.
    global_previous: Vec<u32>,
    /// `SuspendedGlobalActivities`: the full globals that run again as the main challenge
    /// completes.
    suspended: Vec<usize>,
    /// `cStateGraph::TerminateMission` of these graphs: their local `cMsgTerminateChallenge`,
    /// which the challenge system delivers from its Lua state as soon as the caller returns
    /// ([`super::system::ChallengeSystem::deliver_terminations`]).
    pub(crate) terminations: VecDeque<usize>,
}

fn ref_spec(db: &AttribDb, class: u64, collection: u64, key: u64) -> Option<(u64, u64)> {
    let (c, k) = db.attribute(class, collection, key)?.ref_spec(0)?;
    db.has_collection(c, k).then_some((c, k))
}

impl ActivityManager {
    /// `cActivityManager::Init` (`0x82cdc260`): `CreateActivity` for every
    /// leaf `challenges` collection (`AttribSysUtils::GetLeafCollections`:
    /// sorted keys that are no collection's parent).
    #[must_use]
    pub fn new(db: &AttribDb) -> Self {
        let all = db.collections(class::CHALLENGES);
        let mut parents: Vec<u64> = all.iter().filter_map(|&k| db.parent(class::CHALLENGES, k)).collect();
        parents.sort_unstable();
        let mut manager = Self::default();
        for &key in all {
            if parents.binary_search(&key).is_ok() {
                continue;
            }
            let activity = Activity {
                key,
                name: db
                    .attribute(class::CHALLENGES, key, field::NAME)
                    .and_then(|a| a.text(0))
                    .unwrap_or_default()
                    .to_owned(),
                global_data: ref_spec(db, class::CHALLENGES, key, field::GLOBAL_DATA),
                local_data: ref_spec(db, class::CHALLENGES, key, field::LOCAL_DATA),
                graph: StateGraph::gather(db, key),
            };
            manager.index.insert(key, manager.activities.len());
            manager.activities.push(activity);
        }
        // CreateActivity (0x82cdbca8) classifies every activity that is not debug-only.
        manager.globals = (0..manager.activities.len())
            .filter(|&activity| {
                let global = manager.activities[activity].global_data;
                let read = |key| global.and_then(|(c, k)| db.attribute(c, k, key));
                let debug_only = read(DEBUG_ONLY).and_then(|a| a.bool(0)).unwrap_or(false);
                let kind = read(GLOBAL_TYPE).and_then(|a| a.i32(0)).unwrap_or(0);
                !debug_only && matches!(kind, global_type::LOCATION | global_type::FULL)
            })
            .collect();
        manager
    }

    #[must_use]
    pub fn find(&self, key: u64) -> Option<usize> {
        self.index.get(&key).copied()
    }

    #[must_use]
    pub fn graph(&self, activity: usize) -> &StateGraph {
        &self.activities[activity].graph
    }

    pub fn graph_mut(&mut self, activity: usize) -> &mut StateGraph {
        &mut self.activities[activity].graph
    }

    /// `challenge_global_data.Global` of an activity (0 when absent).
    #[must_use]
    pub fn global_type(&self, db: &AttribDb, activity: usize) -> i32 {
        self.activities[activity]
            .global_data
            .and_then(|(c, k)| db.attribute(c, k, GLOBAL_TYPE))
            .and_then(|a| a.i32(0))
            .unwrap_or(0)
    }

    /// `AddToPendingResetList` (`0x82cd9ba8`).
    fn add_to_pending_reset(&mut self, activity: usize) {
        if !self.pending_reset.contains(&activity) {
            self.pending_reset.push(activity);
        }
    }

    /// The `MultiDataProviderMetaTable` providers of an activity: the
    /// `challenges` instance, then `GlobalData` and `LocalData`.
    #[must_use]
    pub fn data_providers(&self, activity: usize) -> Vec<(u64, u64)> {
        let a = &self.activities[activity];
        let mut providers = vec![(class::CHALLENGES, a.key)];
        providers.extend(a.global_data);
        providers.extend(a.local_data);
        providers
    }
}

/// `cActivityManager::GameLoad` (`0x82cd92c0`): the sub-controllers'
/// `GameLoad`, then `ReevaluatePrereqActivities` unless the flags have 0x10.
pub fn game_load(world: &mut World, flags: u32) {
    world.activities.is_quitting_to_fe = false;
    world.activities.game_loaded = true;
    world.activities.game_load_flags = flags;
    // cChallengeBEManager::GameLoad 0x825b7800.
    world.goto_allows_signup = true;
    super::ai::game_load(world);
    super::characters::game_load(world, flags);
    super::signup::game_load(world, flags);
    super::sequence_connector::game_load(world);
    super::living_world::game_load(world, flags);
    super::teleport::game_load(world, flags);
    super::dmo::game_load(world, flags);
    super::dynamic_hull::game_load(world, flags);
    super::services::game_load(world, flags);
    super::hom::game_load(world, flags);
    super::ots_discovery::game_load(world, flags);
    if flags & 0x10 != 0x10 {
        reevaluate_prereq_activities(world);
    }
}

/// `cActivityManager::ReevaluatePrereqActivities` (`0x82cdbaa8`): in the career
/// (flags 4) `EvaluateProgressedLaunchedActivities`, which starts nothing while
/// `kEndableProgressionLaunchedChallenges` is off (its default), and the
/// sub-controllers' `ReevaluateActivityPrereqs`; in the career, free skate (2) and
/// online games (8) then `ReevaluateGlobalActivtyBBoxes` (`kRunGlobalChallenges`).
/// The `cMsgReevaluatePrereqActivities` it sends the simulation's
/// `SimUnlockableManager` is not ported.
pub fn reevaluate_prereq_activities(world: &mut World) {
    let manager = &world.activities;
    if !manager.game_loaded || manager.is_quitting_to_fe {
        return;
    }
    match manager.game_load_flags {
        4 => {
            super::ai::reevaluate_activity_prereqs(world);
            super::characters::reevaluate_activity_prereqs(world, 4);
            super::signup::reevaluate_activity_prereqs(world, 4);
            super::film::reevaluate(world);
            super::living_world::reevaluate_activity_prereqs(world);
            super::map_icons::reevaluate_activity_prereqs(world);
        }
        2 | 8 => {}
        _ => return,
    }
    if RUN_GLOBAL_CHALLENGES {
        reevaluate_global_activity_bboxes(world);
    }
}

fn global_data_attribute(world: &World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'_>> {
    let (class, collection) = world.activities.activities[activity].global_data?;
    world.db.attribute(class, collection, key)
}

/// `ActivityUtils::GenerateLocationActivityBBox` (`0x82cd89c0`): the position ± the activity's
/// radius on each axis, `kDefaultLocationTrackingRadius` when the radius is not positive.
pub(crate) fn generate_location_activity_bbox(radius: f32, position: [f32; 3]) -> OctreeBounds {
    let r = if radius > 0.0 { radius } else { DEFAULT_LOCATION_TRACKING_RADIUS };
    OctreeBounds {
        minimum: [position[0] - r, position[1] - r, position[2] - r, 0.0],
        maximum: [position[0] + r, position[1] + r, position[2] + r, 0.0],
    }
}

/// A location global's box: its `Location` when `cLocationManager::IsValidLocation`, with the
/// challenge's radius.
fn location_activity_bbox(world: &World, activity: usize) -> Option<OctreeBounds> {
    let name = global_data_attribute(world, activity, LOCATION).and_then(|a| a.text(0))?;
    let transform = world.location(hash_value32(name))?;
    let key = world.activities.activities[activity].key;
    let radius = world.db.attribute(class::CHALLENGES, key, RADIUS).and_then(|a| a.f32(0)).unwrap_or(0.0);
    Some(generate_location_activity_bbox(radius, [transform[12], transform[13], transform[14]]))
}

/// `AreGlobalChallengesSuspended` (`0x82cd8918`): a main challenge runs (no sub-controller
/// overrides `AreGlobalActivitiesSuspended`, whose default answers false).
fn are_global_challenges_suspended(world: &World) -> bool {
    world.activities.main.is_some()
}

/// `cActivityManager::CanStartMainChallenge` (`0x82cd8798`): no main activity and every
/// sub-controller's `CanStartMainChallenge`, of which only the AI manager's (`0x825addf8`) can
/// refuse.
pub(crate) fn can_start_main_challenge(world: &World) -> bool {
    world.activities.main.is_none() && super::ai::can_start_main_challenge(world)
}

/// `cStateGraph::TerminateMission` (`0x82ce4f80`): a local `cMsgTerminateChallenge` to the graph,
/// which the challenge system delivers as soon as its Lua state is free.
pub(crate) fn terminate_mission(world: &mut World, activity: usize) {
    world.activities.terminations.push_back(activity);
}

/// `cStateGraph::Run` of a global activity, then the sub-controllers' `RunGlobalActivity` in
/// registration order: the asset manager's (`0x82e12838`) and the DMO manager's (`0x825c3aa8`);
/// every other sub-controller keeps the interface's empty one.
pub(crate) fn run_global_activity(world: &mut World, activity: usize) {
    runtime::run(world, activity);
    super::assets::run_global_activity(world, activity);
    super::dmo::run_global_activity(world, activity);
}

/// `ReevaluateGlobalActivtyBBoxes` (`0x82cdb260`): the global activities' prerequisites
/// (`EvaluateActivitiesForPrereqs` `0x82cda860` with `IgnorePrereqForGlobalStart`) against the
/// last pass. A location global that no longer passes loses its box; a full global is
/// terminated and reset. One that newly passes gets its box at its `Location`, or (a full global)
/// runs, or while a main challenge runs joins the suspended list. In a Party Play pass (flag
/// 0x10), or an online one (8) without the activity's flag, it is dropped from the pass.
fn reevaluate_global_activity_bboxes(world: &mut World) {
    let flags = world.activities.game_load_flags;
    let source = world.activities.globals.clone();
    let mut passed = super::signup::evaluate_for_prereqs(world, &source, flags, "IgnorePrereqForGlobalStart");
    let cached = std::mem::take(&mut world.activities.global_passed);
    let newly_failed: Vec<usize> = cached.iter().copied().filter(|a| passed.binary_search(a).is_err()).collect();
    let newly_passed: Vec<usize> = passed.iter().copied().filter(|a| cached.binary_search(a).is_err()).collect();
    for activity in newly_failed {
        match world.activities.global_type(&world.db, activity) {
            global_type::LOCATION => {
                let manager = &mut world.activities;
                manager.global_tree.remove(activity, &mut manager.global_previous);
            }
            global_type::FULL => {
                terminate_mission(world, activity);
                world.activities.add_to_pending_reset(activity);
            }
            _ => {}
        }
    }
    for activity in newly_passed {
        let online_excluded = flags & 8 == 8
            && !global_data_attribute(world, activity, GAME_TYPE_8).and_then(|a| a.bool(0)).unwrap_or(false);
        if flags & 0x10 == 0x10 || online_excluded {
            if let Ok(at) = passed.binary_search(&activity) {
                passed.remove(at);
            }
            continue;
        }
        match world.activities.global_type(&world.db, activity) {
            global_type::LOCATION => {
                if let Some(bounds) = location_activity_bbox(world, activity) {
                    world.activities.global_tree.insert(activity, bounds);
                }
            }
            global_type::FULL => {
                if are_global_challenges_suspended(world) {
                    if !world.activities.suspended.contains(&activity) {
                        world.activities.suspended.push(activity);
                    }
                } else {
                    run_global_activity(world, activity);
                }
            }
            _ => {}
        }
    }
    world.activities.global_passed = passed;
}

/// `PerformBBoxTests` (`0x82cdaf00`) with the local tester's box (`GenerateTesterBBox`
/// `0x82e20ee0`, while the tester is valid): a running location global whose box the tester left
/// is terminated and reset; each one whose box it is in runs (`kEnableGlobalTriggers`) unless it
/// runs already or a main challenge suspends the globals.
pub(crate) fn perform_bbox_tests(world: &mut World) {
    let Some(base) = world.player.tester_base else { return };
    let current = world.activities.global_tree.collisions(super::characters::tester_bounds(base));
    let (_, exited) = super::trigger_tester::enter_exit(&world.activities.global_previous, &current);
    for index in exited {
        let Some(activity) = world.activities.global_tree.entry(index) else { continue };
        if world.activities.graph(activity).running {
            terminate_mission(world, activity);
            world.activities.add_to_pending_reset(activity);
        }
    }
    for &index in &current {
        let Some(activity) = world.activities.global_tree.entry(index) else { continue };
        if !are_global_challenges_suspended(world) && !world.activities.graph(activity).running && ENABLE_GLOBAL_TRIGGERS {
            run_global_activity(world, activity);
        }
    }
    world.activities.global_previous = current;
}

/// `SuspendAllGlobalChallenges` (`0x82cd9c40`), as a main challenge starts: each running global
/// in creation order; a location global is terminated and reset, a full global suspended
/// (`cStateGraph::Suspend` `0x82ce64e0`) into the suspended list with the sub-controllers'
/// `SuspendGlobalActivity` in registration order: the callout manager's (`0x82da5220`), the BE
/// manager's (`0x825adab0`) and the service manager's (`0x8289da38`).
fn suspend_all_global_challenges(world: &mut World) {
    let globals = world.activities.globals.clone();
    for activity in globals {
        if !world.activities.graph(activity).running {
            continue;
        }
        if world.activities.global_type(&world.db, activity) == global_type::LOCATION {
            terminate_mission(world, activity);
            world.activities.add_to_pending_reset(activity);
        } else {
            runtime::suspend(world, activity);
            world.activities.suspended.push(activity);
            super::callouts::suspend_global_activity(world, activity);
            world.goto_allows_signup = true;
            super::services::suspend_global_activity(world);
        }
    }
}

/// `ResumeSuspendedGlobalChallenges` (`0x82cd9080`), as the main challenge completes: each
/// suspended global runs again (its current state, or `initial` after a reset) with the callout
/// manager's `ResumeGlobalActivity` (`0x82da5278`); the list empties.
fn resume_suspended_global_challenges(world: &mut World) {
    for activity in std::mem::take(&mut world.activities.suspended) {
        runtime::run(world, activity);
        super::callouts::resume_global_activity(world, activity);
    }
}

/// `cActivityManager::StartMainChallenge` (`0x82cda588`): the old main challenge reset, the new
/// one run with the sub-controllers' `RunMainActivity`, then the globals suspended.
pub fn start_main_challenge(world: &mut World, key: u64) {
    if let Some(previous) = world.activities.main.take() {
        world.activities.add_to_pending_reset(previous);
    }
    let Some(activity) = world.activities.find(key) else { return };
    world.activities.main = Some(activity);
    runtime::run(world, activity);
    super::system::run_main_activity(world, activity);
    suspend_all_global_challenges(world);
}

/// `cActivityManager::CompleteChallenge` (`0x82cd9d80`): a completed main challenge resumes the
/// suspended globals.
pub fn complete_challenge(world: &mut World, activity: usize) {
    let global = world.activities.global_type(&world.db, activity);
    if global == 0 || (3..=4).contains(&global) {
        world.activities.add_to_pending_reset(activity);
    }
    let is_main = world.activities.main == Some(activity);
    super::system::complete_challenge_sub_controllers(world, activity, is_main);
    if is_main {
        world.activities.main = None;
        resume_suspended_global_challenges(world);
    }
}

/// `cActivityManager::ProcessPendingResetList` (`0x82cd8fe8`): the activities
/// to shut down (sub-controller `ShutdownActivity`, then `cStateGraph::Reset`).
pub fn take_pending_resets(world: &mut World) -> Vec<usize> {
    std::mem::take(&mut world.activities.pending_reset)
}

/// `cActivityManager::GameUnload` (`0x82cda420`) after the sub-controllers' `GameUnload`: in
/// `ActivityMapping` (key) order each running activity is terminated and reset, the main
/// activity forgotten, and the global boxes, collisions, suspended list and passed list cleared.
pub(crate) fn game_unload(world: &mut World) {
    world.activities.game_loaded = false;
    world.activities.game_load_flags = 0;
    let mut mapping: Vec<(u64, usize)> = world.activities.index.iter().map(|(&key, &activity)| (key, activity)).collect();
    mapping.sort_unstable();
    for (_, activity) in mapping {
        if world.activities.main == Some(activity) {
            world.activities.main = None;
        }
        if world.activities.graph(activity).running {
            terminate_mission(world, activity);
            world.activities.add_to_pending_reset(activity);
        }
    }
    let manager = &mut world.activities;
    manager.is_quitting_to_fe = false;
    manager.global_tree = ActivityTree::default();
    manager.global_previous.clear();
    manager.suspended.clear();
    manager.global_passed.clear();
}

impl ChallengeSystem {
    /// The `cStateGraph::TerminateMission`s queued while the challenge Lua state was busy: each
    /// graph handles its local `cMsgTerminateChallenge` (empty table).
    pub(crate) fn deliver_terminations(&mut self) {
        while let Some(activity) = self.world.activities.terminations.pop_front() {
            let message = Message::new("cMsgTerminateChallenge", Vec::new());
            runtime::handle_local_message(&mut self.challenge, &mut self.world, activity, &message);
        }
    }
}
