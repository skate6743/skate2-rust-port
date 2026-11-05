//! `Sk8::Challenge::cChallengeRaceManager` (ctor `0x825bec78`): a race heat's legs, each a
//! start point and its split-time gates (trigger volume centres), every racer's progress,
//! estimated time to finish and rank, and the leaderboard HUD of the race's group.

use super::{
    attrib_lua::{REF_SPEC_METATABLE, RefSpec, attrib_location},
    binding::{get_float, get_int, int},
    keys::hash_value32,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value, stdlib},
    system::World,
};
use crate::ai::vmx::{self, V4};

type R = LuaResult<Vec<Value>>;

/// `challenge_race_heats` and its `Legs` (`tRaceLegDefinition[]`).
const RACE_HEATS: u64 = 0xe933_655b_7553_d1de;
const LEGS: u64 = 0x48f2_ba87_a4be_11e6;
/// `challenge_race_legs` and its `SplitTimeTriggers` (16-byte entries, the trigger volume ID
/// at +8).
const RACE_LEGS: u64 = 0x7cbd_399d_98bb_0d37;
const SPLIT_TIME_TRIGGERS: u64 = 0xdff5_e686_33a1_8975;

/// `kRaceSplitTimeDivisor` (Challenge_RaceManager, default 9).
const RACE_SPLIT_TIME_DIVISOR: f32 = 9.0;
/// `kRaceUpdateFrames` (default 30): the HUD frames per full update.
const RACE_UPDATE_FRAMES: i32 = 30;

/// `tPlayerRaceInfo` (`0x24` bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerRaceInfo {
    pub racer_id: i32,
    pub estimated_time_to_finish: f32,
    pub estimated_split_time: f32,
    pub finished_time: f32,
    pub rank: i32,
    pub race_timer_id: i32,
    pub current_leg: u32,
    pub current_gate: u32,
    pub finished: bool,
    pub disconnected: bool,
}

/// `tLegInfo` (`0x120` bytes): the leg's start point then its gates' centres.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LegInfo {
    pub gates: Vec<V4>,
    pub total_distance: f32,
}

impl LegInfo {
    /// `GetDistanceFromEndGate` (`0x825b0da8`): the gate-to-gate distance from the gate to the
    /// last.
    fn distance_from_end_gate(&self, start: usize) -> f32 {
        let mut distance = 0.0;
        if start < self.gates.len() {
            for pair in self.gates[start..].windows(2) {
                distance += vmx::length3(vmx::sub(pair[1], pair[0]));
            }
        }
        distance
    }

    /// `GetDistanceFromGate` (`0x825b1040`).
    fn distance_from_gate(&self, position: V4, gate: usize) -> f32 {
        self.gates.get(gate).map_or(0.0, |&at| vmx::length3(vmx::sub(at, position)))
    }

    /// `GetProjectedDistanceFromGate` (`0x825b10d8`): the distance to the gate along the
    /// direction from the gate before it (the plain distance to the first).
    fn projected_distance_from_gate(&self, position: V4, gate: usize) -> f32 {
        if gate >= self.gates.len() {
            return 0.0;
        }
        if gate == 0 {
            return self.distance_from_gate(position, 0);
        }
        let at = self.gates[gate];
        let direction = vmx::normalize_safe(vmx::sub(at, self.gates[gate - 1]), [0.0; 4]);
        vmx::dot3(vmx::sub(at, position), direction).abs()
    }
}

/// `cChallengeRaceManager` (`0xaa0` bytes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RaceManager {
    pub legs: Vec<LegInfo>,
    /// `TrackingRace`.
    pub tracking: bool,
    pub start_point: V4,
    pub end_point: V4,
    pub players: Vec<PlayerRaceInfo>,
    frames_since_last_hud_update: i32,
    hud_leader_id: i32,
    /// `LeaderboardData`: the race's HUD group, by its hash.
    leaderboard_group: Option<u32>,
}

impl RaceManager {
    /// `Reset` (`0x825bdcf0`).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// `AddPlayerToRace` (`0x825bdd30`).
    pub fn add_player(&mut self, racer_id: i32) {
        self.players.push(PlayerRaceInfo {
            racer_id,
            estimated_time_to_finish: 0.0,
            estimated_split_time: 0.0,
            finished_time: -1.0,
            rank: -1,
            race_timer_id: -1,
            current_leg: 0,
            current_gate: 0,
            finished: false,
            disconnected: false,
        });
    }

    /// The entry the player-indexed setters write (`PlayerInfos[playerID]`).
    fn player_at(&mut self, index: i32) -> Option<&mut PlayerRaceInfo> {
        usize::try_from(index).ok().and_then(|index| self.players.get_mut(index))
    }

    fn racer(&self, racer_id: i32) -> Option<&PlayerRaceInfo> {
        self.players.iter().find(|player| player.racer_id == racer_id)
    }

    /// `GetEstimatedTimeToFinish` (`0x825b0a78`): 0 for a player not in the race.
    #[must_use]
    pub fn estimated_time_to_finish(&self, racer_id: i32) -> f32 {
        self.racer(racer_id).map_or(0.0, |player| player.estimated_time_to_finish)
    }

    /// `GetPlayerRank` (`0x825b0b08`): -1 for a player not in the race.
    #[must_use]
    pub fn rank(&self, racer_id: i32) -> i32 {
        self.racer(racer_id).map_or(-1, |player| player.rank)
    }

    /// `GetDistanceToFinish` (`0x825b5f58`): along the current leg from its next gate, then the
    /// later legs whole.
    fn distance_to_finish(&self, position: V4, leg: u32, gate: u32) -> f32 {
        let Some(current) = usize::try_from(leg).ok().and_then(|leg| self.legs.get(leg)) else { return 0.0 };
        let next = usize::try_from(gate).map_or(usize::MAX, |gate| gate.saturating_add(1));
        let mut distance = current.projected_distance_from_gate(position, next) + current.distance_from_end_gate(next);
        for later in &self.legs[usize::try_from(leg).unwrap_or(usize::MAX) + 1..] {
            distance += later.total_distance;
        }
        distance
    }
}

/// `SortPlayerInfoOnEstimatedTimeToFinish` (`0x825ad648`): finished racers by rank, then racing
/// ones by estimated time to finish, then disconnected ones by rank.
fn sorts_before(first: &PlayerRaceInfo, second: &PlayerRaceInfo) -> bool {
    if first.finished {
        return !second.finished || first.rank < second.rank;
    }
    if second.finished {
        return false;
    }
    if !first.disconnected {
        return second.disconnected || first.estimated_time_to_finish < second.estimated_time_to_finish;
    }
    second.disconnected && first.rank < second.rank
}

/// `eastl::quick_sort` of at most eight racers: its `insertion_sort`.
fn insertion_sort(players: &mut [PlayerRaceInfo]) {
    for i in 1..players.len() {
        let value = players[i];
        let mut j = i;
        while j > 0 && sorts_before(&value, &players[j - 1]) {
            players[j] = players[j - 1];
            j -= 1;
        }
        players[j] = value;
    }
}

/// `tLegInfo::AddGates` (`0x825bed08`): the leg's start point and its `SplitTimeTriggers`'
/// volume centres, the leg's total length between them; the last gate starts the next leg.
fn add_gates(world: &mut World, leg: &mut LegInfo, start: V4, collection: u64) -> V4 {
    let db = std::sync::Arc::clone(&world.db);
    let triggers = db.attribute(RACE_LEGS, collection, SPLIT_TIME_TRIGGERS);
    let count = triggers.as_ref().map_or(0, |attribute| attribute.len());
    leg.gates = vec![start];
    for index in 0..count {
        let volume = triggers
            .as_ref()
            .and_then(|attribute| attribute.data(index))
            .and_then(|at| db.bytes(at.offset_by(8), 8))
            .map_or(0, |b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]));
        let volume = crate::skate3::id_float(volume);
        let centre = world.host.trigger_volume_center(volume).unwrap_or([0.0; 3]);
        leg.gates.push(vmx::v3(centre));
    }
    for pair in leg.gates.windows(2) {
        leg.total_distance += vmx::length3(vmx::sub(pair[1], pair[0]));
    }
    *leg.gates.last().unwrap_or(&start)
}

/// `StartRace` (`0x825c3d38`): the heat's legs from the start point, tracked from now, with the
/// leaderboard of the HUD group.
pub(crate) fn start_race(world: &mut World, hud_group: u32, start: V4, heat: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let legs = db.attribute(RACE_HEATS, heat, LEGS);
    let count = legs.as_ref().map_or(0, |attribute| attribute.len());
    world.race.start_point = start;
    world.race.legs.resize(count, LegInfo::default());
    let mut point = start;
    for index in 0..count {
        let collection = legs.as_ref().and_then(|attribute| attribute.ref_spec(index)).map_or(0, |(_, collection)| collection);
        let mut leg = std::mem::take(&mut world.race.legs[index]);
        leg.total_distance = 0.0;
        point = add_gates(world, &mut leg, point, collection);
        world.race.legs[index] = leg;
    }
    let race = &mut world.race;
    race.end_point = point;
    race.tracking = true;
    race.frames_since_last_hud_update = 0;
    race.hud_leader_id = 0;
    race.leaderboard_group = Some(hud_group);
}

/// `cChallengeRaceManager::Update` (`0x825c2500`): each racer's estimated time to finish from
/// its base position (`cTriggerTester::GetBasePosition`), then ranks and split times in sort
/// order, then the HUD.
pub(crate) fn update(world: &mut World) {
    if !world.race.tracking {
        return;
    }
    for index in 0..world.race.players.len() {
        let player = world.race.players[index];
        if player.disconnected {
            continue;
        }
        let Some(base) = super::skate::skater(world, player.racer_id).map(|frame| vmx::v3(frame.tester_base)) else { continue };
        world.race.players[index].estimated_time_to_finish = if player.finished {
            0.0
        } else {
            world.race.distance_to_finish(base, player.current_leg, player.current_gate) / RACE_SPLIT_TIME_DIVISOR
        };
    }
    let race = &mut world.race;
    let mut sorted = race.players.clone();
    insertion_sort(&mut sorted);
    let leader = sorted.first().map_or(0.0, |player| player.estimated_time_to_finish);
    let mut rank = 1;
    for entry in &sorted {
        if let Some(player) = race.players.iter_mut().find(|player| player.racer_id == entry.racer_id) {
            player.rank = rank;
            player.estimated_split_time = player.estimated_time_to_finish - leader;
            rank += 1;
        }
    }
    update_hud(world);
}

/// `UpdateHUD` (`0x825b0b90`): every `kRaceUpdateFrames` the ranks and times (the leader's race
/// time, the others' split times, finish times once finished); between, the leader's time.
fn update_hud(world: &mut World) {
    let Some(group) = world.race.leaderboard_group else { return };
    let Some(data) = world.hud.groups.iter_mut().find(|g| g.hash == group).and_then(|g| g.leaderboard.as_mut()) else { return };
    let race = &mut world.race;
    let row = |index: i32| usize::try_from(index).ok();
    if race.frames_since_last_hud_update < RACE_UPDATE_FRAMES {
        if let Some(leader) = usize::try_from(race.hud_leader_id).ok().and_then(|index| race.players.get(index))
            && !leader.finished
            && !leader.disconnected
        {
            let time = world.timers.time(leader.race_timer_id);
            if let Some(player) = row(leader.racer_id).and_then(|index| data.players.get_mut(index)) {
                player.time = time;
            }
            data.non_order_data_dirty = true;
        }
    } else {
        for player in &race.players {
            let Some(entry) = row(player.racer_id).and_then(|index| data.players.get_mut(index)) else { continue };
            entry.rank = player.rank;
            data.dirty = true;
            let time = if player.disconnected {
                entry.int_var = -1;
                0.0
            } else if player.rank == 1 {
                race.hud_leader_id = player.racer_id;
                if player.finished { player.finished_time } else { world.timers.time(player.race_timer_id) }
            } else if player.finished {
                player.finished_time
            } else {
                player.estimated_split_time
            };
            if let Some(entry) = row(player.racer_id).and_then(|index| data.players.get_mut(index)) {
                entry.time = time;
            }
            data.non_order_data_dirty = true;
        }
        race.frames_since_last_hud_update = 0;
    }
    race.frames_since_last_hud_update += 1;
}

/// `Race.ResetRaceStats()` (`0x82894268`).
fn reset_race_stats(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.race.reset();
    Ok(Vec::new())
}

/// `Race.AddPlayerToRace(player)` (`0x82893f40`).
fn add_player_to_race(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.race.add_player(get_int(call, 0));
    Ok(Vec::new())
}

/// `Race.FinishPlayer(player, time)` (`0x82893fd8`).
fn finish_player(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let time = get_float(call, 1);
    if let Some(player) = world.race.player_at(get_int(call, 0)) {
        player.finished = true;
        player.finished_time = time;
    }
    Ok(Vec::new())
}

/// `Race.PlayerDisconnected(player)` (`0x82894090`).
fn player_disconnected(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(player) = world.race.player_at(get_int(call, 0)) {
        player.disconnected = true;
    }
    Ok(Vec::new())
}

/// `Race.RegisterPlayerTimer(player, timer)` (`0x82894128`).
fn register_player_timer(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let timer = get_int(call, 1);
    if let Some(player) = world.race.player_at(get_int(call, 0)) {
        player.race_timer_id = timer;
    }
    Ok(Vec::new())
}

/// `Race.SetPlayerProgress(player, leg, gate)` (`0x828941c0`).
fn set_player_progress(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let leg = u32::from_ne_bytes(get_int(call, 1).to_ne_bytes());
    let gate = u32::from_ne_bytes(get_int(call, 2).to_ne_bytes());
    if let Some(player) = world.race.player_at(get_int(call, 0)) {
        player.current_leg = leg;
        player.current_gate = gate;
    }
    Ok(Vec::new())
}

/// `Race.StartRaceStats(tLocationID, heat)` (`0x828942e8`): a race from a location the session's
/// location manager knows (`cLocationManager::IsValidLocation` and `GetLocationPosition`
/// `0x82a8e330`: its global, streamed and hull arenas, then the simulation location table the
/// challenge system holds), under the calling activity's HUD group (`GetHashValue32` of its name).
fn start_race_stats(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let location = attrib_location(&world.pointers, call.arg(0)).and_then(|at| world.db.text_at(at)).map(hash_value32);
    let heat = lua
        .userdata_payload::<RefSpec>(call.arg(1))
        .copied()
        .ok_or_else(|| stdlib::type_error(lua, call, 2, "StartRaceStats", REF_SPEC_METATABLE))?;
    let transform = location.and_then(|location| world.host.location_orientation(location).or_else(|| world.locations.get(&location).copied()));
    let Some(transform) = transform else { return Ok(Vec::new()) };
    let Some(activity) = world.closure else { return Ok(Vec::new()) };
    let hud_group = hash_value32(&world.activities.activities[activity].name);
    start_race(world, hud_group, [transform[12], transform[13], transform[14], 0.0], heat.collection);
    Ok(Vec::new())
}

/// `Race.StopRaceStats()` (`0x82894440`): `StopRace`.
fn stop_race_stats(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.race.tracking = false;
    Ok(Vec::new())
}

/// `Race.GetEstimatedTimeToFinish(player)` (`0x828944c0`).
fn get_estimated_time_to_finish(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Number(world.race.estimated_time_to_finish(get_int(call, 0)))])
}

/// `Race.GetRank(player)` (`0x82894548`).
fn get_rank(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![int(world.race.rank(get_int(call, 0)))])
}

/// `Race.GetRacerDistance(player, volume)` (`0x8288ea60`): from the player's base position to
/// the volume's centre, 0 without the volume.
fn get_racer_distance(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let volume = get_float(call, 1);
    let distance = match (world.host.trigger_volume_center(volume), super::skate::skater(world, player)) {
        (Some(centre), Some(frame)) => vmx::length3(vmx::sub(vmx::v3(centre), vmx::v3(frame.tester_base))),
        _ => 0.0,
    };
    Ok(vec![Value::Number(distance)])
}

pub fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Race", "GetRacerDistance", get_racer_distance),
        ("Race", "AddPlayerToRace", add_player_to_race),
        ("Race", "FinishPlayer", finish_player),
        ("Race", "PlayerDisconnected", player_disconnected),
        ("Race", "RegisterPlayerTimer", register_player_timer),
        ("Race", "SetPlayerProgress", set_player_progress),
        ("Race", "ResetRaceStats", reset_race_stats),
        ("Race", "StartRaceStats", start_race_stats),
        ("Race", "StopRaceStats", stop_race_stats),
        ("Race", "GetEstimatedTimeToFinish", get_estimated_time_to_finish),
        ("Race", "GetRank", get_rank),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn racer(racer_id: i32, estimated_time_to_finish: f32) -> PlayerRaceInfo {
        let mut manager = RaceManager::default();
        manager.add_player(racer_id);
        PlayerRaceInfo { estimated_time_to_finish, ..manager.players[0] }
    }

    #[test]
    fn finished_racers_lead_by_rank_and_disconnected_ones_trail() {
        let mut players = vec![
            PlayerRaceInfo { disconnected: true, rank: 1, ..racer(0, 1.0) },
            racer(1, 5.0),
            PlayerRaceInfo { finished: true, rank: 2, ..racer(2, 0.0) },
            racer(3, 2.0),
            PlayerRaceInfo { finished: true, rank: 1, ..racer(4, 0.0) },
        ];
        insertion_sort(&mut players);
        assert_eq!(players.iter().map(|p| p.racer_id).collect::<Vec<_>>(), [4, 2, 3, 1, 0]);
    }

    #[test]
    fn the_distance_to_finish_runs_along_the_rest_of_the_race() {
        let leg = |gates: &[[f32; 3]]| {
            let gates: Vec<V4> = gates.iter().map(|&g| vmx::v3(g)).collect();
            let total_distance = gates.windows(2).map(|p| vmx::length3(vmx::sub(p[1], p[0]))).sum();
            LegInfo { gates, total_distance }
        };
        let race = RaceManager {
            legs: vec![leg(&[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 0.0, 10.0]]), leg(&[[10.0, 0.0, 10.0], [10.0, 0.0, 30.0]])],
            ..RaceManager::default()
        };
        // Projected 6 to gate 1 along +x, 10 on to the leg's end, then the next leg's 20.
        assert_eq!(race.distance_to_finish(vmx::v3([4.0, 0.0, 3.0]), 0, 0), 36.0);
        assert_eq!(race.distance_to_finish(vmx::v3([4.0, 0.0, 3.0]), 2, 0), 0.0);
    }

    #[test]
    fn a_racer_not_in_the_race_has_no_rank_or_time() {
        let mut race = RaceManager::default();
        race.add_player(3);
        assert_eq!((race.rank(3), race.rank(1)), (-1, -1));
        assert_eq!(race.estimated_time_to_finish(1), 0.0);
        assert!(race.player_at(1).is_none());
    }
}
