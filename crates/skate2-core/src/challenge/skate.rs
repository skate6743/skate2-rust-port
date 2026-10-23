//! `Sk8::Challenge::cSkateChallengeManager` (S.K.A.T.E.'s set trick, the copy attempt and the
//! already-been-done list), the static `cTrickPatternManager` and `cChallengeScoringManager`'s
//! enabled trick types that `World::UpdateConditioners` (`0x827af468`) hands every skater's
//! scoring, and the `Skate` and `Trick` natives over them and the skaters' completed outputs.

use super::{
    attrib_db::AttribDb,
    binding::{get_int, int, int_u32},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    mission::MissionFrame,
    system::World,
};

type R = LuaResult<Vec<Value>>;

fn none() -> R {
    Ok(Vec::new())
}

/// `eTrickPatterMatchTypeExactTrick`.
pub const MATCH_EXACT_TRICK: i32 = 1;

/// The `scoring_trick` class.
pub(crate) const TRICK_CLASS: u64 = 0x6918_4699_84a8_c596;
/// `scoring_trick::TrickInputs`: the first input's gesture.
const TRICK_INPUTS: u64 = 0x03ac_c9f2_0080_7b87;

/// `Sk8::Score::TrickPattern` (`0x24` bytes): `mLength` and four (`mMatchType`, `mValue`)
/// items; the constructor (`0x82de9670`) zeroes all nine words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrickPattern {
    pub length: i32,
    pub items: [(i32, i32); 4],
}

impl TrickPattern {
    /// `TrickPatternItem::MatchesTrick(eTrickTypeNone, trickIndex, false)` (`0x82df7010`) of
    /// the first item.
    fn first_matches_trick(&self, trick_index: i32) -> bool {
        let (match_type, value) = self.items[0];
        match match_type {
            MATCH_EXACT_TRICK => trick_index == value,
            2 => value == 0,
            _ => false,
        }
    }
}

/// `tAlreadyBeenDone` (`0x2c` bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlreadyBeenDone {
    pub pattern: TrickPattern,
    pub completed_air_180_count: i32,
    pub switched_stance: bool,
}

/// `cSkateChallengeManager` (constructor `0x825b4220`).
#[derive(Clone, Debug)]
pub struct SkateChallengeManager {
    pub already_been_done: [AlreadyBeenDone; 5],
    /// `mAlreadyBeenDoneListIndex`: the newest entry, -1 when empty.
    pub index: i32,
    /// `mAlreadyBeenDoneListItemLimit` (at most 5).
    pub limit: i32,
    pub attempt: AlreadyBeenDone,
    pub set: AlreadyBeenDone,
}

impl Default for SkateChallengeManager {
    fn default() -> Self {
        Self {
            already_been_done: [AlreadyBeenDone::default(); 5],
            index: -1,
            limit: 5,
            attempt: AlreadyBeenDone::default(),
            set: AlreadyBeenDone::default(),
        }
    }
}

impl SkateChallengeManager {
    /// `Reset` (`0x825af1c0`).
    pub fn reset(&mut self) {
        self.index = -1;
        for item in self.already_been_done.iter_mut().take(usize::try_from(self.limit).unwrap_or(0)) {
            item.pattern.length = 0;
        }
        self.attempt.pattern.length = 0;
        self.set.pattern.length = 0;
    }

    /// `GetAlreadyBeenDoneListItem` (`0x825af208`): out of range reads the current entry.
    #[must_use]
    pub fn item(&self, index: i32) -> &AlreadyBeenDone {
        let at = if index < 0 || index >= self.limit { self.index } else { index };
        &self.already_been_done[usize::try_from(at).unwrap_or(0).min(4)]
    }

    /// `GetCurrentAlreadyBeenDoneListItem` (`0x825af238`): the first entry while empty.
    #[must_use]
    pub fn current(&self) -> &AlreadyBeenDone {
        &self.already_been_done[usize::try_from(self.index).unwrap_or(0).min(4)]
    }

    /// `SetAlreadyBeenDoneListItemLimit` (`0x825af250`).
    pub fn set_limit(&mut self, limit: i32) {
        self.limit = limit.min(5);
    }
}

/// The finished air's absolute half-spin count and its switched start, read from a skater's
/// completed output (`PhysOut_Scoring2::mAirFinishedData`); a skater whose air has not
/// finished asserts natively and reads (-1, false) here.
fn finished_air_spin(frame: Option<&MissionFrame>) -> (i32, bool) {
    frame.and_then(|f| f.finished_air.as_ref()).map_or((-1, false), |air| (air.player_spin180_count.abs(), air.started_switched))
}

/// `AddTrickToAlreadyBeenDoneList` (`0x825b42a8`): a trick not yet in the list with the same
/// half-spin count becomes the newest entry and the set trick, and pattern 0 is armed with it.
pub fn add_trick_to_already_been_done(world: &mut World, trick_index: i32, player: i32) -> bool {
    let (count, switched) = finished_air_spin(skater(world, player));
    let skate = &mut world.skate;
    let limit = usize::try_from(skate.limit).unwrap_or(0).min(5);
    let repeated = skate.already_been_done[..limit]
        .iter()
        .any(|item| item.pattern.length != 0 && item.pattern.first_matches_trick(trick_index) && item.completed_air_180_count == count);
    if repeated {
        return false;
    }
    skate.index += 1;
    if skate.index == skate.limit {
        skate.index = 0;
    }
    let entry = &mut skate.already_been_done[usize::try_from(skate.index).unwrap_or(0).min(4)];
    entry.pattern.length = 1;
    entry.pattern.items[0] = (MATCH_EXACT_TRICK, trick_index);
    entry.switched_stance = switched;
    entry.completed_air_180_count = count;
    let pattern = entry.pattern;
    skate.set.pattern.length = 1;
    skate.set.pattern.items[0] = (MATCH_EXACT_TRICK, trick_index);
    skate.set.switched_stance = switched;
    skate.set.completed_air_180_count = count;
    if let Ok(mut scoring) = world.scoring.lock() {
        scoring.patterns.activate(0);
        scoring.patterns.set(0, pattern);
    }
    true
}

/// `UpdateAttemptItem` (`0x825b44d8`).
pub fn update_attempt_item(world: &mut World, trick_index: i32, player: i32) {
    let (count, switched) = finished_air_spin(skater(world, player));
    let attempt = &mut world.skate.attempt;
    attempt.pattern.length = 1;
    attempt.pattern.items[0] = (MATCH_EXACT_TRICK, trick_index);
    attempt.switched_stance = switched;
    attempt.completed_air_180_count = count;
}

/// `cTrickPatternManager`'s statics: three patterns with their activate and deactivate
/// requests.
#[derive(Clone, Debug, Default)]
pub struct TrickPatternManager {
    activate: [bool; 3],
    deactivate: [bool; 3],
    patterns: [TrickPattern; 3],
}

impl TrickPatternManager {
    /// `ActivateTrickPattern` (`0x82da17f0`).
    pub fn activate(&mut self, index: usize) {
        self.activate[index.min(2)] = true;
    }

    /// `DeactivateTrickPattern` (`0x82da18a8`).
    pub fn deactivate(&mut self, index: usize) {
        self.deactivate[index.min(2)] = true;
    }

    /// `SetTrickPattern` (`0x82da1960`).
    pub fn set(&mut self, index: usize, pattern: TrickPattern) {
        self.patterns[index.min(2)] = pattern;
    }

    /// `GetTrickPattern` (`0x82da19f8`).
    #[must_use]
    pub fn pattern(&self, index: usize) -> TrickPattern {
        self.patterns[index.min(2)]
    }
}

/// What `World::UpdateConditioners` (`0x827af468`) gives every skater's scoring from the
/// challenge system in one simulation update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoringInputs {
    /// `PopActivateTrickPattern`/`PopDeactivateTrickPattern` (`0x82da1848`/`0x82da1900`).
    pub activate: [bool; 3],
    pub deactivate: [bool; 3],
    /// `GetTrickPattern` of each activated pattern; the others stay constructed.
    pub patterns: [TrickPattern; 3],
    /// `cChallengeScoringManager::GetTrickTypeEnabled` (`0x82da22a0`) for the 14 kinds.
    pub enabled: [bool; 14],
    /// What `Physics::IsHallOfMeatEffectEnabled` (`0x827acbd0`) reads of the challenge system.
    pub hom_visuals: bool,
    pub hom_available: bool,
    /// `Physics::IsScoringMultiplierActive` (`0x827accc8`).
    pub multiplier_active: bool,
}

impl Default for ScoringInputs {
    fn default() -> Self {
        Self {
            activate: [false; 3],
            deactivate: [false; 3],
            patterns: [TrickPattern::default(); 3],
            enabled: [true; 14],
            hom_visuals: true,
            hom_available: true,
            multiplier_active: true,
        }
    }
}

/// `cTrickPatternManager`'s statics and `Sk8ChallengeSystem::ScoringManager`
/// (`cChallengeScoringManager`, constructor `0x82da2178`: every kind enabled), which
/// the challenge natives write and every skater's `World::UpdateConditioners`
/// (`0x827af468`) reads, with the HOM manager and signpost state its
/// `IsHallOfMeatEffectEnabled` (`0x827acbd0`) reads.
#[derive(Clone, Debug)]
pub struct ChallengeScoring {
    pub patterns: TrickPatternManager,
    /// `mCurActivity`.
    pub current: Option<usize>,
    /// `mScoringEnabled`.
    pub enabled: [bool; 14],
    /// `ChallengeHOMManager::mHOMVisualsEnabled` (constructor `0x825bc7c0`: enabled).
    pub hom_visuals: bool,
    /// The signpost "Hom Available" message setting (default 1).
    pub hom_available: bool,
    /// `Physics::IsScoringMultiplierActive` (`0x827accc8`): no main challenge, or one whose
    /// `ChallengeType` is none of Photo, BestTrick, OwnTheSpot, MegaRamp, CreateASpot and
    /// CreateASpot_Setup.
    pub multiplier_active: bool,
}

impl Default for ChallengeScoring {
    fn default() -> Self {
        Self {
            patterns: TrickPatternManager::default(),
            current: None,
            enabled: [true; 14],
            hom_visuals: true,
            hom_available: true,
            multiplier_active: true,
        }
    }
}

impl ChallengeScoring {
    /// One `UpdateConditioners` read: a simulation update (`updateMask & 1`) pops the
    /// activate and deactivate requests and copies each activated pattern; every update
    /// reads the enabled kinds.
    pub fn pop(&mut self, simulation: bool) -> ScoringInputs {
        let mut inputs = ScoringInputs {
            enabled: self.enabled,
            hom_visuals: self.hom_visuals,
            hom_available: self.hom_available,
            multiplier_active: self.multiplier_active,
            ..ScoringInputs::default()
        };
        if simulation {
            let patterns = &mut self.patterns;
            for i in 0..3 {
                inputs.activate[i] = std::mem::take(&mut patterns.activate[i]);
                inputs.deactivate[i] = std::mem::take(&mut patterns.deactivate[i]);
                if inputs.activate[i] {
                    inputs.patterns[i] = patterns.patterns[i];
                }
            }
        }
        inputs
    }

    /// What every skater reads once the challenge system's game unloaded
    /// (`Sk8ChallengeSystem::GameUnload` `0x825a4330`): its activities are shut down
    /// (`ShutdownActivity` `0x82da2310`: no current activity, every kind enabled) and none is the
    /// main challenge, for which `Physics::IsScoringMultiplierActive` (`0x827accc8`) answers 1.
    /// The patterns are `cTrickPatternManager` statics and stay.
    pub fn game_unload(&mut self) {
        self.current = None;
        self.enabled = [true; 14];
        self.multiplier_active = true;
    }
}

/// `cChallengeScoringManager::SetTrickType`/`SetAllTrickTypes` (`0x82da21b0`/`0x82da2238`):
/// the activity becomes the current one (another activity asserts natively).
fn set_trick_types(world: &mut World, activity: Option<usize>, kinds: std::ops::Range<usize>, enable: bool) {
    if let Ok(mut scoring) = world.scoring.lock() {
        scoring.current = activity;
        for kind in kinds {
            if let Some(slot) = scoring.enabled.get_mut(kind) {
                *slot = enable;
            }
        }
    }
}

/// `cChallengeScoringManager::ShutdownActivity` (`0x82da2310`).
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    if let Ok(mut scoring) = world.scoring.lock()
        && scoring.current == Some(activity)
    {
        scoring.enabled = [true; 14];
        scoring.current = None;
    }
}

/// `SkaterManager::FindByIndex`: the skater with this player index.
#[must_use]
pub fn skater(world: &World, player: i32) -> Option<&MissionFrame> {
    world.skaters.iter().find(|frame| frame.player == player)
}

/// `Sk8::Score::HasPatternMatched` (`0x82de95c0`).
#[must_use]
pub fn has_pattern_matched(frame: &MissionFrame, index: usize) -> bool {
    let finished = frame.sequence_completed.is_some() && frame.finished_pattern_matches[index];
    if frame.in_sequence { frame.current_pattern_matches[index] || finished } else { finished }
}

/// `Sk8::Score::IsValidScorableID` (`0x82dea7e0`) and `GetTrickType` (`0x82dedfc8`).
fn scorable_trick_type(id: i32) -> Option<i32> {
    crate::physical_player_phys_out::conditioners::scorable_id_trick_type(id)
}

/// `TrickScoringSettings(name)`: the `scoring_trick` collection of the lower-cased name and
/// its index among the class's sorted leaf collections (`TrickDatabase`).
fn trick_index_by_name(db: &AttribDb, name: &str) -> Option<i32> {
    let key = super::keys::hash64(&name.to_ascii_lowercase());
    db.leaf_collections(TRICK_CLASS).iter().position(|&k| k == key).and_then(|i| i32::try_from(i).ok())
}

/// `TrickScoringSettings(trickIndex)`: the index's `scoring_trick` collection.
fn trick_by_index(db: &AttribDb, index: i32) -> Option<u64> {
    usize::try_from(index).ok().and_then(|i| db.leaf_collections(TRICK_CLASS).get(i).copied())
}

/// `TrickScoringSettings::GetTrickNameHal` (`0x82601c80`): the trick's `mTrickNameHal`.
#[must_use]
pub fn trick_name_hal(db: &AttribDb, index: i32) -> Option<String> {
    let key = trick_by_index(db, index)?;
    db.layout_text(TRICK_CLASS, key, 0).map(str::to_owned)
}

/// The first `TrickInputs` entry's `eSk8ControllerGesture` (`ABDHUDData::SetSetTrickData`
/// `0x826b6af8`), `None` (0) when the trick has none.
#[must_use]
pub fn trick_gesture(db: &AttribDb, index: i32) -> u32 {
    trick_by_index(db, index)
        .and_then(|key| db.attribute(TRICK_CLASS, key, TRICK_INPUTS))
        .and_then(|a| a.data(0))
        .and_then(|at| db.bytes(at, 4))
        .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

// ---- Skate ----------------------------------------------------------------

/// `Skate.ResetSkateTrackers()` (`0x8288af60`).
fn reset_skate_trackers(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.skate.reset();
    none()
}

/// `Skate.MatchedPattern(player)` (`0x8288b010`): pattern 0 matched by a finished air whose
/// half-spin count is the newest already-been-done entry's.
fn skate_matched_pattern(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let mut matched = false;
    if let Some(frame) = skater(world, player) {
        let count = frame.finished_air.as_ref().map_or(0, |air| air.player_spin180_count.abs());
        matched = has_pattern_matched(frame, 0) && world.skate.current().completed_air_180_count == count;
    }
    Ok(vec![Value::Boolean(matched)])
}

/// `Skate.GetPlayerSpeed(player)` (`0x8288b218`): `mSkateboardMotion->mDeckSpeed`.
fn skate_get_player_speed(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let speed = skater(world, get_int(call, 0)).map_or(0.0, |f| f.deck_speed);
    Ok(vec![Value::Number(speed)])
}

/// `Skate.AddTrickToAlreadyBeenDoneList(trickIndex, player)` (`0x8288b518`).
fn skate_add_trick(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let done = add_trick_to_already_been_done(world, get_int(call, 0), get_int(call, 1));
    Ok(vec![Value::Boolean(done)])
}

/// `Skate.ChallengeCompleted()` (`0x8288b5e8`): `DeactivateTrickPattern(0)`.
fn skate_challenge_completed(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if let Ok(mut scoring) = world.scoring.lock() {
        scoring.patterns.deactivate(0);
    }
    none()
}

/// `Skate.SetAlreadyBeenDoneListItemLimit(limit)` (`0x8288b698`).
fn skate_set_limit(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.skate.set_limit(get_int(call, 0));
    none()
}

/// `Skate.UpdateAttemptItem(trickIndex, player)` (`0x8288b740`).
fn skate_update_attempt_item(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    update_attempt_item(world, get_int(call, 0), get_int(call, 1));
    none()
}

/// `Skate.ResetAttemptItem()` (`0x8288b800`).
fn skate_reset_attempt_item(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.skate.attempt.pattern.length = 0;
    none()
}

/// `Skate.ResetSetItem()` (`0x8288b8b8`).
fn skate_reset_set_item(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.skate.set.pattern.length = 0;
    none()
}

/// `Skate.SetPlayerRank(player, rank)` (`0x82883200`): `PlayersBackEnd::SetPlayerRank`
/// (`0x8255a538`); a player of 8 or more is the native's assertion.
fn skate_set_player_rank(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (player, rank) = (super::binding::get_int(call, 0), super::binding::get_int(call, 1));
    if let Some(players) = world.players.as_deref() {
        players.set_player_rank(player, rank);
    }
    none()
}

// ---- Trick ----------------------------------------------------------------

/// `Lua::ModuleTrick::GetPlayerPhysOut` (`0x82887240`): the first argument's skater.
fn player_frame<'a>(world: &'a World, call: &NativeCall) -> Option<&'a MissionFrame> {
    skater(world, get_int(call, 0))
}

/// `Trick.DeactivatePattern()` (`0x8287f5c8`).
fn trick_deactivate_pattern(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if let Ok(mut scoring) = world.scoring.lock() {
        scoring.patterns.deactivate(0);
    }
    none()
}

/// `Trick.GetScorableName(id)` (`0x8287f5f0`): `GetScorableIDString`.
fn trick_get_scorable_name(_: &mut Lua<World>, _: &mut World, call: &NativeCall) -> R {
    let name = crate::physical_player_phys_out::conditioners::scorable_id_string(get_int(call, 0)).unwrap_or("INVALID");
    Ok(vec![Value::from(name)])
}

/// `Trick.GetTrickIndexFromScorableID(id)` (`0x8287f638`): the trick database index of the
/// scorable's name, -1 for an invalid scorable or trick.
fn trick_get_index_from_scorable(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = crate::physical_player_phys_out::conditioners::scorable_id_string(get_int(call, 0))
        .and_then(|name| trick_index_by_name(&world.db, name))
        .unwrap_or(-1);
    Ok(vec![int(index)])
}

/// `Trick.GetTrickName(index)` (`0x8287f6b0`): `TrickScoringSettings::GetTrickName`
/// (`0x82df2550`), the trick's scorable's `gScorableInfo` name, or the empty string for an
/// invalid trick.
fn trick_get_trick_name(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = trick_by_index(&world.db, get_int(call, 0)).map(|key| {
        let scorable = world.db.layout_word(TRICK_CLASS, key, SCORABLE_ID_WORD);
        crate::physical_player_phys_out::conditioners::scorable_id_string(scorable.cast_signed()).unwrap_or("INVALID").to_owned()
    });
    Ok(vec![Value::from(name.unwrap_or_default().as_str())])
}

/// The `scoring_trick` layout word holding `ScorableID` (layout byte 136).
const SCORABLE_ID_WORD: u32 = 34;

/// `Trick.GetTrickTypeFromScorableID(id)` (`0x82888290`).
fn trick_get_type_from_scorable(_: &mut Lua<World>, _: &mut World, call: &NativeCall) -> R {
    Ok(vec![int(scorable_trick_type(get_int(call, 0)).unwrap_or(0))])
}

/// `Trick.IsInAirWithValidLaunchTrick(player)` (`0x828874d8`): in the air state with a valid
/// initial air or flip trick.
fn trick_is_in_air_with_valid_launch(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let valid = player_frame(world, call).is_some_and(|frame| {
        frame.is_in_state_air && scorable_trick_type(frame.current_air_initial_scorable).is_some_and(|kind| matches!(kind, 1 | 2))
    });
    Ok(vec![Value::Boolean(valid)])
}

/// `Trick.DidCompletedAirStartSwitched(player)` (`0x828882e8`).
fn trick_did_completed_air_start_switched(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let switched = player_frame(world, call).and_then(|f| f.finished_air.as_ref()).is_some_and(|air| air.started_switched);
    Ok(vec![Value::Boolean(switched)])
}

/// `Trick.GetCompletedAir180Count(player)` (`0x82888358`): the finished air's absolute
/// half-spin count, 0 when no air finished.
fn trick_get_completed_air_180_count(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let count = player_frame(world, call).and_then(|f| f.finished_air.as_ref()).map_or(0, |air| air.player_spin180_count.abs());
    Ok(vec![int(count)])
}

/// `Trick.GetLineScore(player)` (`0x82888420`).
fn trick_get_line_score(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![int(player_frame(world, call).map_or(0, |f| f.line_score))])
}

/// `Trick.GetCompletedSequenceScore(player)` (`0x82888468`): `mFinishedSequenceScore`.
fn trick_get_completed_sequence_score(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![int_u32(player_frame(world, call).map_or(0, |f| f.finished_sequence_score))])
}

/// `Trick.MatchedPattern(player)` (`0x82887d20`).
fn trick_matched_pattern(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(player_frame(world, call).is_some_and(|f| has_pattern_matched(f, 0)))])
}

/// `Trick.IsInSequence(player)` (`0x82887420`): `IsInACurrentSequence`.
fn trick_is_in_sequence(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(player_frame(world, call).is_some_and(|f| f.in_sequence))])
}

/// `Trick.IsSkaterInAir(player)` (`0x828873c8`): `PhysOut_Air::mIsSkateboardAirborne`.
fn trick_is_skater_in_air(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(player_frame(world, call).is_some_and(|f| f.skateboard_airborne))])
}

/// `Trick.IsSkaterInAirOrGrinding(player)` (`0x82887360`).
fn trick_is_skater_in_air_or_grinding(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(player_frame(world, call).is_some_and(|f| f.skateboard_airborne || f.is_in_state_grinding))])
}

/// `Trick.IsInManual(player)` (`0x82887478`): `Score::IsDoingATrickOfType(eTrickTypeManual)`.
fn trick_is_in_manual(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    const MANUAL: i32 = 8;
    Ok(vec![Value::Boolean(player_frame(world, call).is_some_and(|f| f.active_trick_types.contains(&MANUAL)))])
}

/// `Trick.GetNumTrickType(player, type)` (`0x828872f0`):
/// `Score::GetCompletedSequence_TrickTypeCount`. Without the skater the stack's top returns.
fn trick_get_num_trick_type(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(frame) = player_frame(world, call) else {
        return Ok(vec![call.arg(call.len().saturating_sub(1)).clone()]);
    };
    let count = usize::try_from(get_int(call, 1)).ok().and_then(|t| frame.trick_types_in_completed_sequence.get(t)).copied().unwrap_or(0);
    Ok(vec![int(i32::from(count))])
}

/// The finished air of the skater's update (`Score::JustFinishedAir` `0x82de8ed8`).
fn finished_air<'a>(world: &'a World, call: &NativeCall) -> Option<&'a super::mission::FinishedAir> {
    player_frame(world, call).and_then(|f| f.finished_air.as_ref())
}

/// `Trick.GetCompletedAirDegreesOfSpin(player)` (`0x82887700`): 180 per half spin.
fn trick_get_completed_air_degrees_of_spin(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    #[expect(clippy::cast_precision_loss, reason = "the native (float)")]
    let degrees = finished_air(world, call).map_or(0.0, |air| (180 * air.player_spin180_count) as f32);
    Ok(vec![Value::Number(degrees)])
}

/// `Trick.GetCompletedAirNumFlips(player)` (`0x82887790`): `GetCompletedAirNumFrontFlips`.
fn trick_get_completed_air_num_flips(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    #[expect(clippy::cast_precision_loss, reason = "the native (float)")]
    let flips = finished_air(world, call).map_or(0.0, |air| air.player_num_flips as f32);
    Ok(vec![Value::Number(flips)])
}

/// `Trick.GetCompletedAirHeight(player)` (`0x82887820`): `mHeightToPeak`.
fn trick_get_completed_air_height(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Number(finished_air(world, call).map_or(0.0, |air| air.height_to_peak))])
}

/// `Trick.GetCompletedAirTime(player)` (`0x828879a0`): `mInAirFrames` at 60 Hz.
fn trick_get_completed_air_time(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    #[expect(clippy::cast_precision_loss, reason = "the native (float)")]
    let time = finished_air(world, call).map_or(0.0, |air| air.in_air_frames as f32 * 0.016_666_668);
    Ok(vec![Value::Number(time)])
}

/// `Trick.GetCompletedAirGrabTime(player)` (`0x82887a20`).
fn trick_get_completed_air_grab_time(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Number(finished_air(world, call).map_or(0.0, |air| air.grab_time))])
}

/// `Trick.GetCompletedAirHorizontalDistance(player)` (`0x82887b60`).
fn trick_get_completed_air_horizontal_distance(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Number(finished_air(world, call).map_or(0.0, |air| air.horizontal_distance))])
}

/// `Trick.GetCompletedGrindDistance(player)` (`0x82887cc0`): the last finished grind's length.
fn trick_get_completed_grind_distance(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Number(player_frame(world, call).map_or(0.0, |f| f.finished_grind_length))])
}

/// `Trick.GetHOMScore(player)` (`0x82888540`): `mHoM_BailData.mScore`.
fn trick_get_hom_score(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![int_u32(player_frame(world, call).map_or(0, |f| f.hom_score))])
}

/// `Trick.IsDoingTrick(player)` (`0x828875c0`): `Score::IsInACurrentTrick`.
fn trick_is_doing_trick(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(player_frame(world, call).is_some_and(|f| f.in_a_current_trick))])
}

// ---- ChallengeManager trick types -----------------------------------------

/// `ChallengeManager.DisableAllTrickTypes()` (`0x82892e38`).
fn disable_all_trick_types(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let activity = world.closure;
    set_trick_types(world, activity, 0..14, false);
    none()
}

/// `ChallengeManager.EnableTrickType(kind)` (`0x82892f00`).
fn enable_trick_type(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = world.closure;
    let kind = usize::try_from(get_int(call, 0)).unwrap_or(usize::MAX);
    set_trick_types(world, activity, kind..kind.saturating_add(1), true);
    none()
}

/// `ChallengeManager.ResetTrickTypeScoring()` (`0x82892fd8`).
fn reset_trick_type_scoring(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let activity = world.closure;
    set_trick_types(world, activity, 0..14, true);
    none()
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Skate", "ResetSkateTrackers", reset_skate_trackers),
        ("Skate", "MatchedPattern", skate_matched_pattern),
        ("Skate", "GetPlayerSpeed", skate_get_player_speed),
        ("Skate", "AddTrickToAlreadyBeenDoneList", skate_add_trick),
        ("Skate", "ChallengeCompleted", skate_challenge_completed),
        ("Skate", "SetAlreadyBeenDoneListItemLimit", skate_set_limit),
        ("Skate", "UpdateAttemptItem", skate_update_attempt_item),
        ("Skate", "ResetAttemptItem", skate_reset_attempt_item),
        ("Skate", "ResetSetItem", skate_reset_set_item),
        ("Skate", "SetPlayerRank", skate_set_player_rank),
        ("Trick", "DeactivatePattern", trick_deactivate_pattern),
        ("Trick", "GetScorableName", trick_get_scorable_name),
        ("Trick", "GetTrickIndexFromScorableID", trick_get_index_from_scorable),
        ("Trick", "GetTrickName", trick_get_trick_name),
        ("Trick", "GetTrickTypeFromScorableID", trick_get_type_from_scorable),
        ("Trick", "IsInAirWithValidLaunchTrick", trick_is_in_air_with_valid_launch),
        ("Trick", "DidCompletedAirStartSwitched", trick_did_completed_air_start_switched),
        ("Trick", "GetCompletedAir180Count", trick_get_completed_air_180_count),
        ("Trick", "GetLineScore", trick_get_line_score),
        ("Trick", "GetCompletedSequenceScore", trick_get_completed_sequence_score),
        ("Trick", "MatchedPattern", trick_matched_pattern),
        ("Trick", "IsInSequence", trick_is_in_sequence),
        ("Trick", "IsDoingTrick", trick_is_doing_trick),
        ("Trick", "IsSkaterInAir", trick_is_skater_in_air),
        ("Trick", "IsSkaterInAirOrGrinding", trick_is_skater_in_air_or_grinding),
        ("Trick", "IsInManual", trick_is_in_manual),
        ("Trick", "GetNumTrickType", trick_get_num_trick_type),
        ("Trick", "GetCompletedAirDegreesOfSpin", trick_get_completed_air_degrees_of_spin),
        ("Trick", "GetCompletedAirNumFlips", trick_get_completed_air_num_flips),
        ("Trick", "GetCompletedAirHeight", trick_get_completed_air_height),
        ("Trick", "GetCompletedAirTime", trick_get_completed_air_time),
        ("Trick", "GetCompletedAirGrabTime", trick_get_completed_air_grab_time),
        ("Trick", "GetCompletedAirHorizontalDistance", trick_get_completed_air_horizontal_distance),
        ("Trick", "GetCompletedGrindDistance", trick_get_completed_grind_distance),
        ("Trick", "GetHOMScore", trick_get_hom_score),
        ("ChallengeManager", "DisableAllTrickTypes", disable_all_trick_types),
        ("ChallengeManager", "EnableTrickType", enable_trick_type),
        ("ChallengeManager", "ResetTrickTypeScoring", reset_trick_type_scoring),
    ]
}
