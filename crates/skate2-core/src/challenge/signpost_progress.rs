//! `cSignPostManager`'s progression data beyond challenge availability:
//! groups and their exits, point pools, rankings, per-challenge save values,
//! the BLOG and news, and the memcard savable `SignPost`
//! (`SaveData` `0x825cdef8`, `LoadData` `0x825e53d8`).

use super::{
    attrib_db::AttribDb,
    attrib_lua::activity_of,
    binding,
    keys::{class, hash_value32},
    lua::{Lua, LuaResult, NativeCall, Value},
    system::World,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Vault classes the progression data reads.
pub mod classes {
    pub const GROUP_EXITS: u64 = 0x7e94_a8f0_3622_7172;
    pub const POINT_POOLS: u64 = 0x28dd_7b80_0e72_2053;
    pub const RANKING_GROUPS: u64 = 0x75f8_4936_dc1a_647d;
    pub const RANKINGS: u64 = 0x53ae_88ff_9b39_6c43;
    /// Per-challenge-type save values (`InitChallengeProgressionData`).
    pub const CHALLENGE_SAVE_TYPES: u64 = 0x393d_fb7b_7318_e80f;
    pub const PHOTO_ARTICLES: u64 = 0xf453_6c85_3463_dd31;
    pub const MESSAGES: u64 = 0xb9e2_54f1_92f0_721a;
    pub const AWARDS: u64 = 0x302b_f010_a9dc_614d;
    /// `progression_challengeeconomytags`: `Name` (word 0), id (word 2).
    pub const ECONOMY_TAGS: u64 = 0xb883_44f0_72a7_9d1f;
    /// `progression_challengeeconomydata`: value (word 0), `ChallengeID`
    /// (word 1), tag id (word 2).
    pub const ECONOMY_DATA: u64 = 0x22dd_b3d8_601a_e9f5;
    /// `sponsor`: `Name` (word 0).
    pub const SPONSORS: u64 = 0x548b_58b0_c27c_9564;
}

/// Attribute keys.
pub mod field {
    /// `progression_groups`: the group's challenges.
    pub const GROUP_CHALLENGES: u64 = 0xec1c_213b_bdbf_1a68;
    /// `progression_groups`: the group's exits.
    pub const GROUP_EXITS: u64 = 0x57e6_8297_aa1e_8a1d;
    /// `progression_groupexits`: the exit's requirements.
    pub const EXIT_REQUIREMENTS: u64 = 0x1b3e_c9cd_af36_9aae;
    /// A requirement's point pool or group.
    pub const REQUIREMENT_TARGET: u64 = 0xe750_7c88_d532_665e;
    /// State machines a pool, ranking or group exit triggers.
    pub const TRIGGERS: u64 = 0xd102_e741_1533_faa8;
}

/// `cChallengeTypeSaveData`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChallengeSave {
    pub challenge: u64,
    pub save_type: u64,
    pub value: i32,
}

/// `eBLOGTypes`.
pub mod blog_type {
    pub const CHALLENGE_ACTIVATE: u32 = 0;
    pub const CHALLENGE_COMPLETE: u32 = 1;
    pub const PHOTO_ARTICLE: u32 = 2;
    pub const MESSAGE: u32 = 3;
}

/// `cBLOGData`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blog {
    pub order: i32,
    pub new: bool,
}

const IDENTITY: [[f32; 3]; 4] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0; 3]];

#[derive(Debug)]
pub struct Progress {
    /// `mAvailableGroupData`.
    pub groups: BTreeSet<u64>,
    /// `mGroupCountData`: completed challenges per group.
    pub group_counts: BTreeMap<u64, i32>,
    /// `mGroupExitData`.
    pub group_exits: BTreeSet<u64>,
    /// `mPointPoolData`.
    pub point_pools: BTreeMap<u64, i32>,
    /// `mRankingData`: ranking group to its ranking.
    pub rankings: BTreeMap<u64, u64>,
    /// `mChallengeSaveData`, keyed by creation order.
    pub challenge_save: BTreeMap<u64, ChallengeSave>,
    /// `mBLOGData`, keyed by (type, item).
    pub blog: BTreeMap<(u32, u64), Blog>,
    /// `mNewsData`.
    pub news: BTreeSet<u64>,
    /// `ValidProgressionOrientation` and the `ProgressionOrientation` rows.
    pub orientation: (bool, [[f32; 3]; 4]),
    /// `UseOverrideBecauseQuiting`: `QuittingOverride` (`0x825c4688`) set the orientation.
    pub use_override_because_quitting: bool,
    /// `mCashEarnedResults` by `eCashCredit`.
    pub cash_earned: [i32; 7],
    blog_count: i32,
    challenge_save_count: u64,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            groups: BTreeSet::new(),
            group_counts: BTreeMap::new(),
            group_exits: BTreeSet::new(),
            point_pools: BTreeMap::new(),
            rankings: BTreeMap::new(),
            challenge_save: BTreeMap::new(),
            blog: BTreeMap::new(),
            news: BTreeSet::new(),
            orientation: (false, IDENTITY),
            use_override_because_quitting: false,
            cash_earned: [0; 7],
            blog_count: 0,
            challenge_save_count: 0,
        }
    }
}

fn refs(db: &AttribDb, class: u64, collection: u64, key: u64) -> Vec<(u64, u64)> {
    db.attribute(class, collection, key).map(|a| (0..a.len()).filter_map(|i| a.ref_spec(i)).collect()).unwrap_or_default()
}

fn layout_int(db: &AttribDb, class: u64, collection: u64, index: u32) -> i32 {
    db.layout_word(class, collection, index).cast_signed()
}

/// `progression_challenges` layout word 12: `ChallengeID`.
#[must_use]
pub fn challenge_id(db: &AttribDb, challenge: u64) -> u32 {
    db.layout_word(class::PROGRESSION_CHALLENGES, challenge, 12)
}

/// A `cSignPostUtil` id search: the first collection of `class` whose layout
/// word `index` is `id`, or 0.
fn key_from_id(db: &AttribDb, class: u64, index: u32, id: u32) -> u64 {
    db.collections(class).iter().copied().find(|&k| db.layout_word(class, k, index) == id).unwrap_or(0)
}

/// A `cSignPostUtil` name search over the `char*` at layout word `index`.
fn key_from_name(db: &AttribDb, class: u64, index: u32, name: &str) -> u64 {
    db.collections(class).iter().copied().find(|&k| db.layout_text(class, k, index) == Some(name)).unwrap_or(0)
}

/// `GetChallengeKeyFromChallengeID` (`0x825c8668`).
#[must_use]
pub fn challenge_key_from_id(db: &AttribDb, id: u32) -> u64 {
    key_from_id(db, class::PROGRESSION_CHALLENGES, 12, id)
}

/// `GetBLOGOrder(type, key)` (`0x825cc540`).
fn blog_order(world: &World, kind: u32, key: u64) -> i32 {
    world.signpost.progress.blog.get(&(kind, key)).map_or(-1, |b| b.order)
}

/// The item a BLOG id names for its type (`GetBLOGOrder(type, id)`
/// `0x825cfd10`).
fn blog_key(db: &AttribDb, kind: u32, id: u32) -> Option<u64> {
    match kind {
        blog_type::CHALLENGE_ACTIVATE | blog_type::CHALLENGE_COMPLETE => Some(challenge_key_from_id(db, id)),
        blog_type::PHOTO_ARTICLE => Some(key_from_id(db, classes::PHOTO_ARTICLES, 4, id)),
        blog_type::MESSAGE => Some(key_from_id(db, classes::MESSAGES, 2, id)),
        _ => None,
    }
}

/// The BLOG id `SaveData` writes for an entry.
fn blog_id(db: &AttribDb, kind: u32, key: u64) -> u32 {
    match kind {
        blog_type::PHOTO_ARTICLE => db.layout_word(classes::PHOTO_ARTICLES, key, 4),
        blog_type::MESSAGE => db.layout_word(classes::MESSAGES, key, 2),
        _ => challenge_id(db, key),
    }
}

/// The BLOG entry `ActivateChallenge` (text at layout word 6) and
/// `ChallengeComplete` (word 5) add for a challenge with BLOG text. While
/// initialising, only the activation of a `Starting` challenge is recorded.
pub(crate) fn add_challenge_blog(world: &mut World, challenge: u64, kind: u32) {
    let db = Arc::clone(&world.db);
    let text_word = if kind == blog_type::CHALLENGE_COMPLETE { 5 } else { 6 };
    if db.layout_text(class::PROGRESSION_CHALLENGES, challenge, text_word).is_none_or(str::is_empty) {
        return;
    }
    let key = challenge_key_from_id(&db, challenge_id(&db, challenge));
    if blog_order(world, kind, key) > 0 {
        return;
    }
    if world.signpost.during_init()
        && (kind == blog_type::CHALLENGE_COMPLETE || db.layout_byte(class::PROGRESSION_CHALLENGES, key, 28) == 0)
    {
        return;
    }
    let p = &mut world.signpost.progress;
    p.blog_count += 1;
    p.blog.insert((kind, key), Blog { order: p.blog_count, new: true });
}

/// `AddBLOGEntry(type, id, order, new)` (`0x825d9000`). Photo-article
/// captions are not ported.
fn add_blog_entry(world: &mut World, kind: u32, id: u32, order: i32, new: bool) {
    let db = Arc::clone(&world.db);
    let Some(key) = blog_key(&db, kind, id) else { return };
    if blog_order(world, kind, key) > 0 {
        return;
    }
    if world.signpost.during_init()
        && (kind != blog_type::CHALLENGE_ACTIVATE || db.layout_byte(class::PROGRESSION_CHALLENGES, key, 28) == 0)
    {
        return;
    }
    let p = &mut world.signpost.progress;
    p.blog_count += 1;
    let order = if order == -1 { p.blog_count } else { order };
    p.blog.insert((kind, key), Blog { order, new });
}

/// `HandleBLOGNewsEvent`'s new BLOG message (`0x825d9410`): the message key
/// of `id` (`GetMessageKeyFromID` `0x825c8fc8`) becomes a new entry at the
/// next order unless it has one. While initialising the entry is built but
/// never recorded.
pub(crate) fn add_message_blog(world: &mut World, id: u32) {
    let key = key_from_id(&world.db, classes::MESSAGES, 2, id);
    if blog_order(world, blog_type::MESSAGE, key) > 0 || world.signpost.during_init() {
        return;
    }
    let p = &mut world.signpost.progress;
    p.blog_count += 1;
    p.blog.insert((blog_type::MESSAGE, key), Blog { order: p.blog_count, new: true });
}

/// `UpdatePhotoArticlesBLOGS`' entry (`0x825d92c8`): the article becomes a new entry at the
/// next order unless it has one; while initialising it is never recorded. Whether it was added.
pub(crate) fn add_photo_article_blog(world: &mut World, photo_article: u64) -> bool {
    if blog_order(world, blog_type::PHOTO_ARTICLE, photo_article) > 0 || world.signpost.during_init() {
        return false;
    }
    let p = &mut world.signpost.progress;
    p.blog_count += 1;
    p.blog.insert((blog_type::PHOTO_ARTICLE, photo_article), Blog { order: p.blog_count, new: true });
    true
}

/// `RemoveBLOGEntry(type, id)` (`0x825d0128`): the entry at that item's order
/// (`GetBLOGDataInOrder` `0x825c8108`, the last in map order) leaves, the
/// later entries move down a place (`ShuffleBlogEntriesDown` `0x825c8420`)
/// and the count drops.
pub(crate) fn remove_blog_entry(world: &mut World, kind: u32, id: u32) {
    let db = Arc::clone(&world.db);
    let order = blog_key(&db, kind, id).map_or(-1, |key| blog_order(world, kind, key));
    let p = &mut world.signpost.progress;
    let Some(key) = p.blog.iter().filter(|(_, b)| b.order == order).map(|(&(_, key), _)| key).last() else { return };
    p.blog.remove(&(kind, key));
    for blog in p.blog.values_mut() {
        if blog.order > order {
            blog.order -= 1;
        }
    }
    p.blog_count -= 1;
}

/// `AddUserNews(id)` (`0x825d9370`): the message key of `id` joins
/// `mNewsData` unless `CheckUserNews` (`0x825cc6c8`) finds it there.
/// `intUserNewsCount` is not kept: `SaveData` writes the map's size.
pub(crate) fn add_user_news(world: &mut World, id: u32) {
    let key = key_from_id(&world.db, classes::MESSAGES, 2, id);
    world.signpost.progress.news.insert(key);
}

/// `RemoveUserNews(id)` (`0x825cc740`).
pub(crate) fn remove_user_news(world: &mut World, id: u32) {
    let key = key_from_id(&world.db, classes::MESSAGES, 2, id);
    world.signpost.progress.news.remove(&key);
}

/// The challenge-save half of `InitChallengeProgressionData`
/// (`0x825e28a8`): one value per save type and progression challenge of
/// that type's kind (layout word 11), starting at the type's word 1.
pub(crate) fn init_challenge_save_data(world: &mut World) {
    let db = Arc::clone(&world.db);
    let challenges = world.signpost.progression_challenge_keys().to_vec();
    for &save_type in db.collections(classes::CHALLENGE_SAVE_TYPES) {
        let initial = layout_int(&db, classes::CHALLENGE_SAVE_TYPES, save_type, 1);
        let kind = db.layout_word(classes::CHALLENGE_SAVE_TYPES, save_type, 2);
        for &challenge in &challenges {
            if db.layout_byte(class::PROGRESSION_CHALLENGES, challenge, 40) == 0
                && db.layout_word(class::PROGRESSION_CHALLENGES, challenge, 11) == kind
            {
                let p = &mut world.signpost.progress;
                p.challenge_save.insert(p.challenge_save_count, ChallengeSave { challenge, save_type, value: initial });
                p.challenge_save_count += 1;
            }
        }
    }
}

/// `InitGroupProgressionData` (`0x825e2ba8`): a zero count per group, and
/// the groups whose layout byte 4 is set are activated.
pub(crate) fn init_groups(world: &mut World) {
    let db = Arc::clone(&world.db);
    for &group in db.collections(class::PROGRESSION_GROUPS) {
        world.signpost.progress.group_counts.insert(group, 0);
        if db.layout_byte(class::PROGRESSION_GROUPS, group, 4) == 1 {
            activate_group(world, group);
        }
    }
}

/// `InitPointPoolData` (`0x825d8af8`): each pool at its layout word 5.
pub(crate) fn init_point_pools(world: &mut World) {
    let db = Arc::clone(&world.db);
    for &pool in db.collections(classes::POINT_POOLS) {
        world.signpost.progress.point_pools.insert(pool, layout_int(&db, classes::POINT_POOLS, pool, 5));
    }
}

/// `InitRankingGroupData` (`0x825d8c20`): each ranking group at its
/// first-level ranking (word 2 the group id, word 3 the level).
pub(crate) fn init_rankings(world: &mut World) {
    let db = Arc::clone(&world.db);
    for &group in db.collections(classes::RANKING_GROUPS) {
        let id = db.layout_word(classes::RANKING_GROUPS, group, 1);
        for &ranking in db.collections(classes::RANKINGS) {
            if db.layout_word(classes::RANKINGS, ranking, 2) == id && db.layout_word(classes::RANKINGS, ranking, 3) == 1 {
                world.signpost.progress.rankings.insert(group, ranking);
            }
        }
    }
}

/// `ActivateGroup` (`0x825e0680`).
pub(crate) fn activate_group(world: &mut World, group: u64) {
    if group & 0xffff_ffff == 0 {
        return;
    }
    let db = Arc::clone(&world.db);
    for (_, challenge) in refs(&db, class::PROGRESSION_GROUPS, group, field::GROUP_CHALLENGES) {
        super::signpost::activate_challenge(world, challenge, true, false, true);
    }
    world.signpost.progress.groups.insert(group);
}

/// `DeactivateGroup` (`0x825e0828`).
pub(crate) fn deactivate_group(world: &mut World, group: u64) {
    if group & 0xffff_ffff == 0 {
        return;
    }
    let db = Arc::clone(&world.db);
    for (_, challenge) in refs(&db, class::PROGRESSION_GROUPS, group, field::GROUP_CHALLENGES) {
        super::signpost::deactivate_challenge(world, challenge);
    }
    world.signpost.progress.groups.remove(&group);
}

/// The `TRIGGERS` machines of a pool (`PointsEarned` `0x825d7610`), ranking
/// or group exit (`GroupExitTriggerStateMachines` `0x825d74d0`).
fn trigger_state_machines(world: &mut World, class: u64, collection: u64) {
    let db = Arc::clone(&world.db);
    for (_, machine) in refs(&db, class, collection, field::TRIGGERS) {
        super::signpost::queue_triggered_state_machine(world, machine);
    }
}

/// `IncrementPointsPoolPoints(key, value, credit)` (`0x825d78d0`). A gain
/// of pool 43 (cash) with a board sponsor (`GetBoardSponsorKey` != -1) adds
/// the `Sponsor Bonus` setting's percent of it (credit 4,
/// `0x825d7988..0x825d79f4`). The cash pool's `StatsBackEnd::UpdateStats` credits
/// (`0x825d79f8..0x825d7ad0`): a gain adds it and its bonus to statistic 10 and,
/// with credit 1, to 12; anything else adds its magnitude to 11 (forced) and,
/// with credit 1, to 13.
#[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
pub(crate) fn increment_points(world: &mut World, pool: u64, value: i32, credit: usize) {
    let db = Arc::clone(&world.db);
    let bonus = if db.layout_word(classes::POINT_POOLS, pool, 1) == 43 && value > 0 && has_board_sponsor(world) {
        let percent = world.signpost.message_setting_value("Sponsor Bonus").unwrap_or(0).cast_signed();
        if percent > 0 { percent.wrapping_mul(value) / 100 } else { 0 }
    } else {
        0
    };
    let p = &mut world.signpost.progress;
    let Some(points) = p.point_pools.get_mut(&pool) else { return };
    *points += value;
    if let Some(earned) = p.cash_earned.get_mut(credit) {
        *earned += value;
    }
    if bonus != 0 {
        *points += bonus;
        if let Some(earned) = p.cash_earned.get_mut(4) {
            *earned += bonus;
        }
    }
    if db.layout_word(classes::POINT_POOLS, pool, 1) == 43 {
        if value > 0 {
            world.stat_updates.extend([(10, value as f32, false), (10, bonus as f32, false)]);
            if credit == 1 {
                world.stat_updates.push((12, value as f32, false));
            }
        } else {
            let spent = value.wrapping_abs() as f32;
            world.stat_updates.push((11, spent, true));
            if credit == 1 {
                world.stat_updates.push((13, spent, false));
            }
        }
    }
    let minimum = layout_int(&db, classes::POINT_POOLS, pool, 2);
    let maximum = layout_int(&db, classes::POINT_POOLS, pool, 3);
    *points = (*points).max(minimum).min(maximum);
    trigger_state_machines(world, classes::POINT_POOLS, pool);
}

/// `GetPointsPoolPointsFromKey` (`0x825cbf00`): -1 for 0 or an unknown pool.
fn pool_points(world: &World, pool: u64) -> i32 {
    if pool == 0 {
        return -1;
    }
    world.signpost.progress.point_pools.get(&pool).copied().unwrap_or(-1)
}

/// `cSignPostUtil::GetPointPoolKeyFromName` (`0x825c9338`).
fn point_pool_key(db: &AttribDb, name: &str) -> u64 {
    key_from_name(db, classes::POINT_POOLS, 0, name)
}

/// `cSignPostManager::GetPointsPoolPoints(name)` (`0x825cf148`): -1 for an unknown pool.
pub(crate) fn points_by_name(world: &World, name: &str) -> i32 {
    pool_points(world, point_pool_key(&world.db, name))
}

/// `cSignPostManager::IncrementPointsPoolPoints(name, value)` (`0x825ddf68`):
/// a pool found by name takes the value with credit 0.
pub(crate) fn increment_points_by_name(world: &mut World, name: &str, value: i32) {
    let pool = point_pool_key(&world.db, name);
    if pool != 0 {
        increment_points(world, pool, value, 0);
    }
}

/// `GetCash` (`0x825d2348`): the `Cash` pool's points.
#[must_use]
pub fn cash(world: &World) -> i32 {
    pool_points(world, point_pool_key(&world.db, "Cash"))
}

/// The `Cash` pool's points as `InitPointPoolData` (`0x825d8af8`) starts them: layout word 5.
#[must_use]
pub fn starting_cash(db: &AttribDb) -> i32 {
    layout_int(db, classes::POINT_POOLS, point_pool_key(db, "Cash"), 5)
}

/// `SubtractCash` (`0x825dd7f8`): `IncrementPointsPoolPoints` of the `Cash`
/// pool by the negated amount, which still triggers the pool's machines
/// for an amount of 0.
pub(crate) fn subtract_cash(world: &mut World, cash: i32, credit: usize) {
    let pool = point_pool_key(&world.db, "Cash");
    if pool != 0 {
        increment_points(world, pool, cash.wrapping_neg(), credit);
    }
}

/// `AddCash(cash, credit)` (`0x825dd7a0`): `IncrementPointsPoolPoints` of
/// the `Cash` pool.
pub(crate) fn add_cash(world: &mut World, cash: i32, credit: usize) {
    let pool = point_pool_key(&world.db, "Cash");
    if pool != 0 {
        increment_points(world, pool, cash, credit);
    }
}

/// The save entry of a challenge (`ConvertChallengeKeyToSignpostKey`) whose
/// save type is named `name`: `GetChallengeSaveData` (`0x825cf418`) and
/// `SetChallengeSaveData` (`0x825cf560`) search `mChallengeSaveData` for it.
fn challenge_save_entry<'a>(world: &'a mut World, challenge: u64, name: &str) -> Option<&'a mut ChallengeSave> {
    let key = super::signpost::signpost_key_for_challenge(world, challenge).filter(|&k| k != 0)?;
    let db = Arc::clone(&world.db);
    world
        .signpost
        .progress
        .challenge_save
        .values_mut()
        .find(|s| s.challenge == key && db.layout_text(classes::CHALLENGE_SAVE_TYPES, s.save_type, 0) == Some(name))
}

/// `GetChallengeSaveData` (`0x825cf418`): 0 without an entry.
#[must_use]
pub fn challenge_save_data(world: &World, challenge: u64, name: &str) -> i32 {
    let Some(key) = super::signpost::signpost_key_for_challenge(world, challenge).filter(|&k| k != 0) else { return 0 };
    let db = &world.db;
    world
        .signpost
        .progress
        .challenge_save
        .values()
        .find(|s| s.challenge == key && db.layout_text(classes::CHALLENGE_SAVE_TYPES, s.save_type, 0) == Some(name))
        .map_or(0, |s| s.value)
}

/// `SetChallengeSaveData` (`0x825cf560`): only an existing entry changes.
pub(crate) fn set_challenge_save_data(world: &mut World, challenge: u64, name: &str, value: i32) {
    if let Some(entry) = challenge_save_entry(world, challenge, name) {
        entry.value = value;
    }
}

/// `cSignPostUtil::CheckIfChallengeTypeHasSaveDataType` (`0x825c9cf0`): a
/// save type of the progression challenge's kind (layout word 11) is named
/// `name`.
fn challenge_type_has_save_data_type(db: &AttribDb, signpost: u64, name: &str) -> bool {
    let kind = db.layout_word(class::PROGRESSION_CHALLENGES, signpost, 11);
    db.collections(classes::CHALLENGE_SAVE_TYPES).iter().any(|&t| {
        db.layout_word(classes::CHALLENGE_SAVE_TYPES, t, 2) == kind && db.layout_text(classes::CHALLENGE_SAVE_TYPES, t, 0) == Some(name)
    })
}

/// `GetChallengeEconomyData(challengeKey, tagName)` (`0x825d2ff0`): the tag
/// by name (`GetEconomyTagFromName` `0x825c9a98`), then (`0x825cf7f0`) the
/// first economy row of the progression challenge's `ChallengeID` and the
/// tag's id; 0 without one.
#[must_use]
pub fn challenge_economy_data(world: &World, challenge: u64, tag_name: &str) -> i32 {
    let db = &world.db;
    let tag = key_from_name(db, classes::ECONOMY_TAGS, 0, tag_name);
    let Some(signpost) = super::signpost::signpost_key_for_challenge(world, challenge).filter(|&k| k != 0) else { return 0 };
    let id = challenge_id(db, signpost);
    let tag_id = db.layout_word(classes::ECONOMY_TAGS, tag, 2);
    db.collections(classes::ECONOMY_DATA)
        .iter()
        .find(|&&row| db.layout_word(classes::ECONOMY_DATA, row, 1) == id && db.layout_word(classes::ECONOMY_DATA, row, 2) == tag_id)
        .map_or(0, |&row| layout_int(db, classes::ECONOMY_DATA, row, 0))
}

/// The economy tags of the places `AwardPrizeCash` pays (`0x82fdac60`).
const PLACE_TAGS: [&str; 6] = ["1st Place", "2nd Place", "3rd Place", "4th Place", "5th Place", "6th Place"];

/// `AwardPrizeCash(challengeKey, newRank, oldRank)` (`0x825de2d0`), for a
/// challenge whose type keeps `Highest Rank`: a rank of 3 or better is the
/// completion medal (`rank - 1`); a first rank, or one better than the old,
/// pays the difference of the two places' economy data as contest purse
/// and saves the rank. The save-data call gets the converted key and
/// converts it again. All of it is skipped while a challenge is replayed
/// (`sGameInfo.isReplayingChallenge`).
pub(crate) fn award_prize_cash(world: &mut World, challenge: u64, new_rank: i32, old_rank: i32) {
    if world.replaying_challenge {
        return;
    }
    let Some(signpost) = super::signpost::signpost_key_for_challenge(world, challenge).filter(|&k| k != 0) else { return };
    if !challenge_type_has_save_data_type(&world.db, signpost, "Highest Rank") {
        return;
    }
    if new_rank <= 3 {
        world.signpost.completion.medal_type = new_rank.wrapping_sub(1);
    }
    if old_rank > 0 && new_rank >= old_rank {
        return;
    }
    let place = |world: &World, rank: i32| {
        usize::try_from(rank.wrapping_sub(1))
            .ok()
            .and_then(|i| PLACE_TAGS.get(i))
            .map_or(0, |tag| challenge_economy_data(world, challenge, tag))
    };
    let cash = place(world, new_rank).wrapping_sub(place(world, old_rank));
    add_cash(world, cash, 3);
    set_challenge_save_data(world, signpost, "Highest Rank", new_rank);
}

/// `ProcessEntryFee` (`0x825de3f0`): a challenge whose type keeps `Paid
/// Entry Fee` and has not paid it pays its `Entry Fee` (general credit) and
/// is marked paid. The save-data calls get the already converted key and
/// convert it again.
pub(crate) fn process_entry_fee(world: &mut World, challenge: u64) {
    let Some(signpost) = super::signpost::signpost_key_for_challenge(world, challenge).filter(|&k| k != 0) else { return };
    if challenge_type_has_save_data_type(&world.db, signpost, "Paid Entry Fee") && challenge_save_data(world, signpost, "Paid Entry Fee") == 0 {
        let fee = challenge_economy_data(world, challenge, "Entry Fee");
        subtract_cash(world, fee, 0);
        set_challenge_save_data(world, signpost, "Paid Entry Fee", 1);
    }
}

/// `cSignPostSponsorMan::HasBoardSponsor` (`0x825d1290`): `GetBoardSponsorKey`
/// (`0x825d1198`) looks the `Board Sponsor` setting (missing: the hash of
/// `kMissingSponsorName`, which points at "Online") up among the sponsors'
/// name hashes (`cSignPostSponsorMan::Init` `0x825d8328`).
#[must_use]
pub fn has_board_sponsor(world: &World) -> bool {
    let setting = world.signpost.message_setting_value("Board Sponsor").unwrap_or_else(|| hash_value32("Online"));
    let db = &world.db;
    db.collections(classes::SPONSORS)
        .iter()
        .any(|&s| db.layout_text(classes::SPONSORS, s, 0).is_some_and(|name| hash_value32(name) == setting))
}

/// `challenge_local_data` fields `GetExtraEntryFee` reads.
mod extra_fee_field {
    /// Bool: the challenge needs a drained pool.
    pub const POOL_DRAIN: u64 = 0x97cd_1fc9_b27a_1e3c;
    /// RefSpec: the pool-drain challenge.
    pub const POOL_DRAIN_CHALLENGE: u64 = 0xbe79_e831_558f_2113;
    /// Bool: the challenge needs uncapped hulls.
    pub const UNCAP: u64 = 0x1070_acf7_360e_007e;
    /// RefSpec[]: the hulls.
    pub const UNCAP_HULLS: u64 = 0xe576_48e8_0501_973e;
}

/// `GetExtraEntryFee` (`0x825d03e0`): 200 (a service cost getter folded
/// with `GetMaxTriggerIndicators` `0x82db40e0`) while the pool-drain
/// challenge is not complete (`IsChallengeComplete` `0x825cf328`), plus
/// 100 (`GetMaxSplineIndicators` `0x8276cb50`) for each uncap hull in the
/// `Capped` dynamic hull state (`TestForDynamicHullState` `0x82da9670`). The
/// native reads a hull that names no challenge without a check; it is an
/// error here.
pub(crate) fn extra_entry_fee(world: &World, activity: usize) -> Result<i32, String> {
    let Some((c, k)) = world.activities.activities[activity].local_data else { return Ok(0) };
    let db = &world.db;
    let flag = |key| db.attribute(c, k, key).and_then(|a| a.bool(0)).unwrap_or(false);
    let mut fee = 0;
    if flag(extra_fee_field::POOL_DRAIN) {
        let challenge = db.attribute(c, k, extra_fee_field::POOL_DRAIN_CHALLENGE).and_then(|a| a.ref_spec(0)).map_or(0, |(_, key)| key);
        let complete = super::signpost::signpost_key_for_challenge(world, challenge).is_some_and(|s| world.signpost.is_challenge_complete(s));
        if !complete {
            fee = 200;
        }
    }
    if flag(extra_fee_field::UNCAP)
        && let Some(hulls) = db.attribute(c, k, extra_fee_field::UNCAP_HULLS)
    {
        for index in 0..hulls.len() {
            let hull = hulls.ref_spec(index).map_or(0, |(_, key)| key);
            match super::dynamic_hull::test_state(world, hull, "Capped") {
                Some(true) => fee += 100,
                Some(false) => {}
                None => return Err(format!("GetExtraEntryFee: uncap hull {hull:#018x} names no challenge")),
            }
        }
    }
    Ok(fee)
}

/// `RankingEarned` (`0x825d7750`) with `UpdateRanking` (`0x825c7968`).
fn ranking_earned(world: &mut World, ranking: u64) {
    let db = Arc::clone(&world.db);
    let group = key_from_id(&db, classes::RANKING_GROUPS, 1, db.layout_word(classes::RANKINGS, ranking, 2));
    if let Some(current) = world.signpost.progress.rankings.get_mut(&group) {
        *current = ranking;
    }
    trigger_state_machines(world, classes::RANKINGS, ranking);
}

/// `CheckRanking(name)` (`0x825c7bc0`): the ranking group's current ranking
/// is at least the named one's level.
pub(crate) fn check_ranking(world: &World, name: &str) -> bool {
    let db = &world.db;
    let ranking = key_from_name(db, classes::RANKINGS, 0, name);
    let level = layout_int(db, classes::RANKINGS, ranking, 3);
    let group = key_from_id(db, classes::RANKING_GROUPS, 1, db.layout_word(classes::RANKINGS, ranking, 2));
    world.signpost.progress.rankings.get(&group).is_some_and(|&current| layout_int(db, classes::RANKINGS, current, 3) >= level)
}

/// `challenges.Competitors` (`tCompetitorInfo[7]`).
const COMPETITORS: u64 = 0xfa18_5f4f_0928_02aa;
/// `eCashCredit` 1, the betting credit.
const CREDIT_BETTING: usize = 1;

/// `cSignPostManager`'s bet: `bettingChallengeID`, `intPlayersBet`, `intBettingPot` and
/// `playerBetMax`, which only the constructor clears (`Init` keeps them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Betting {
    pub challenge_id: u32,
    pub bet: i32,
    pub pot: i32,
    pub bet_max: bool,
}

/// `SetPlayersBet(challengeKey, bet)` (`0x825cfaf8`): the bet, with no pot and not the maximum;
/// for a challenge the activity manager knows, the pot is the bet times its `Competitors` plus
/// the player, a bet of at least the `MaximumBet` setting is the maximum, and a challenge in the
/// progression (`ConvertNeutrinoChallengeKeyToSignpostKey`) becomes the betting challenge by its
/// `ChallengeID`.
pub(crate) fn set_players_bet(world: &mut World, challenge: u64, bet: i32) {
    let betting = &mut world.signpost.betting;
    betting.pot = 0;
    betting.bet = bet;
    betting.bet_max = false;
    if world.activities.find(challenge).is_none() {
        return;
    }
    let competitors = world.db.attribute(class::CHALLENGES, challenge, COMPETITORS).map_or(0, |a| a.len());
    let pot = (i32::try_from(competitors).unwrap_or(i32::MAX) + 1).wrapping_mul(bet);
    let maximum = world.signpost.message_setting_value("MaximumBet").unwrap_or(0).cast_signed();
    let signpost = super::signpost::signpost_key_for_challenge(world, challenge);
    let id = signpost.map(|key| challenge_id(&world.db, key));
    let betting = &mut world.signpost.betting;
    betting.pot = pot;
    if bet >= maximum {
        betting.bet_max = true;
    }
    if let Some(id) = id {
        betting.challenge_id = id;
    }
}

/// `DeductChallengeLosses` (`0x825de1e0`), as `LoseChallenge` and `AbandonChallenge` run it: a
/// lost betting challenge takes the bet from the cash (betting credit) and clears the bet.
pub(crate) fn deduct_challenge_losses(world: &mut World, challenge: u64) {
    let Some(signpost) = super::signpost::signpost_key_for_challenge(world, challenge) else { return };
    if challenge_id(&world.db, signpost) != world.signpost.betting.challenge_id {
        return;
    }
    let bet = world.signpost.betting.bet;
    let pool = point_pool_key(&world.db, "Cash");
    if pool != 0 {
        increment_points(world, pool, bet.wrapping_neg(), CREDIT_BETTING);
    }
    let betting = &mut world.signpost.betting;
    betting.challenge_id = 0;
    betting.bet = 0;
    betting.pot = 0;
}

/// `IssueChallengeAwards` (`0x825ddfc8`): the challenge's awards, each pool's points (credit 0,
/// a `Cash` award also counted as the challenge's cash); for the betting challenge the pot less
/// the bet is won (betting credit), the pot becomes the betting result and the bet is cleared.
pub(crate) fn issue_challenge_awards(world: &mut World, challenge: u64) {
    let db = Arc::clone(&world.db);
    let id = challenge_id(&db, challenge);
    for &award in db.collections(classes::AWARDS) {
        if db.layout_word(classes::AWARDS, award, 1) != id {
            continue;
        }
        let amount = layout_int(&db, classes::AWARDS, award, 2);
        let pool = key_from_id(&db, classes::POINT_POOLS, 1, db.layout_word(classes::AWARDS, award, 0));
        increment_points(world, pool, amount, 0);
        if db.layout_text(classes::POINT_POOLS, pool, 0) == Some("Cash") {
            world.signpost.progress.cash_earned[2] += amount;
        }
    }
    if id == world.signpost.betting.challenge_id {
        let Betting { bet, pot, .. } = world.signpost.betting;
        let pool = point_pool_key(&db, "Cash");
        if pool != 0 {
            increment_points(world, pool, pot.wrapping_sub(bet), CREDIT_BETTING);
        }
        let betting = &mut world.signpost.betting;
        betting.challenge_id = 0;
        betting.bet = 0;
        betting.pot = 0;
        world.signpost.progress.cash_earned[CREDIT_BETTING] = pot;
    }
}

/// `GroupExitLogic` (`0x825ddcc8`): count the completed challenge in every
/// group listing it, then `CheckGroupExits` (`0x825ddb58`) for those groups.
pub(crate) fn group_exit_logic(world: &mut World, challenge: u64) {
    let db = Arc::clone(&world.db);
    let id = challenge_id(&db, challenge);
    let mut groups = Vec::new();
    for &group in db.collections(class::PROGRESSION_GROUPS) {
        for (_, member) in refs(&db, class::PROGRESSION_GROUPS, group, field::GROUP_CHALLENGES) {
            if challenge_id(&db, member) == id {
                groups.push(group);
                if let Some(count) = world.signpost.progress.group_counts.get_mut(&group) {
                    *count += 1;
                }
            }
        }
    }
    for group in groups {
        for (_, exit) in refs(&db, class::PROGRESSION_GROUPS, group, field::GROUP_EXITS) {
            if !world.signpost.progress.group_exits.contains(&exit) {
                check_group_exit(world, exit);
            }
        }
    }
}

/// `CheckGroupExit` (`0x825dd850`): every requirement met completes the exit
/// and triggers its machines. A requirement with layout word 5 > 0 needs its
/// group's completed count at word 4, otherwise its pool's points at word 1.
fn check_group_exit(world: &mut World, exit: u64) {
    let db = Arc::clone(&world.db);
    let mut met = true;
    for (rc, requirement) in refs(&db, classes::GROUP_EXITS, exit, field::EXIT_REQUIREMENTS) {
        let target = db.attribute(rc, requirement, field::REQUIREMENT_TARGET).and_then(|a| a.ref_spec(0)).map_or(0, |(_, k)| k);
        let p = &world.signpost.progress;
        met &= if layout_int(&db, rc, requirement, 5) > 0 {
            p.group_counts.get(&target).is_some_and(|&c| layout_int(&db, rc, requirement, 4) <= c)
        } else {
            p.point_pools.get(&target).is_some_and(|&v| layout_int(&db, rc, requirement, 1) <= v)
        };
    }
    if met {
        world.signpost.progress.group_exits.insert(exit);
        trigger_state_machines(world, classes::GROUP_EXITS, exit);
    }
}

// ---- Natives -----------------------------------------------------------------

type R = LuaResult<Vec<Value>>;

fn name_arg(call: &NativeCall) -> String {
    binding::get_string(call, 0).unwrap_or_default()
}

/// `Progression.ActivateGroup(name)` (`0x825e3690`).
fn activate_group_native(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = key_from_name(&world.db, class::PROGRESSION_GROUPS, 0, &name_arg(call));
    activate_group(world, group);
    Ok(Vec::new())
}

/// `Progression.DeactivateGroup(name)` (`0x825e3970`).
fn deactivate_group_native(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = key_from_name(&world.db, class::PROGRESSION_GROUPS, 0, &name_arg(call));
    deactivate_group(world, group);
    Ok(Vec::new())
}

/// `Progression.IncrementPointPoolPoints(name, value)` (`0x825c70a0`) and the challenge state's
/// `ChallengeProgression.IncrementPointPoolPoints` (`0x82890d40`, through `ISignPostManager`) →
/// `IncrementPointsPoolPoints(name, value)` (`0x825ddf68`).
fn increment_point_pool_points(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let pool = key_from_name(&world.db, classes::POINT_POOLS, 0, &name_arg(call));
    if pool != 0 {
        increment_points(world, pool, binding::get_int(call, 1), 0);
    }
    Ok(Vec::new())
}

/// `Progression.PointPoolPoints(name)` (`0x825d1a88`).
fn point_pool_points(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let pool = key_from_name(&world.db, classes::POINT_POOLS, 0, &name_arg(call));
    Ok(vec![binding::int(pool_points(world, pool))])
}

/// `Progression.CheckRanking(name)` (`0x825cb990`).
fn check_ranking_native(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(check_ranking(world, &name_arg(call)))])
}

/// `Progression.RankingEarned(name)` (`0x825d88e0`).
fn ranking_earned_native(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let ranking = key_from_name(&world.db, classes::RANKINGS, 0, &name_arg(call));
    if ranking != 0 {
        ranking_earned(world, ranking);
    }
    Ok(Vec::new())
}

/// `Progression.CheckLastCompletedChallengeTitle(title)` (`0x825cbac0`):
/// against `GetLastCompletedChallengeTitle` (`0x825c85f0`), the title (layout
/// word 1) of the challenge completed last.
fn check_last_completed_challenge_title(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let title = name_arg(call);
    let last = world.signpost.last_completed_challenge();
    let matches = last.and_then(|k| world.db.layout_text(class::PROGRESSION_CHALLENGES, k, 1)).is_some_and(|t| t == title);
    Ok(vec![Value::Boolean(matches)])
}

/// `Progression.CheckMessageSettingString(name, value)` (`0x825cde28`) →
/// `cSignPostMessageMan::CheckMessageSettingString` (`0x825cd890`).
fn check_message_setting_string(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let value = binding::get_string(call, 1).unwrap_or_default();
    let setting = world.signpost.message_setting_value(&name_arg(call));
    Ok(vec![Value::Boolean(setting == Some(hash_value32(&value)))])
}

/// `Progression.IsChallengeActive(name)` (`0x825d3c68`): by
/// `GetChallengeKeyFromName` (`0x825c8780`, layout word 3).
fn is_challenge_active(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let key = key_from_name(&world.db, class::PROGRESSION_CHALLENGES, 3, &name_arg(call));
    Ok(vec![Value::Boolean(key & 0xffff_ffff != 0 && world.signpost.is_challenge_active(key))])
}

/// `ChallengeProgression.UpdateCashAward(data, rank)` (`0x828914c8`):
/// `AwardPrizeCash` of the activity's challenge against its saved
/// `Highest Rank`.
fn update_cash_award(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = activity_of(lua, call.arg(0)) else { return Ok(Vec::new()) };
    let challenge = world.activities.activities[activity].key;
    let rank = binding::get_int(call, 1);
    let old_rank = challenge_save_data(world, challenge, "Highest Rank");
    award_prize_cash(world, challenge, rank, old_rank);
    Ok(Vec::new())
}

/// `Award.GrantAward(award)` (`0x82886078`): `cSignPostManager::GrantAward`
/// (`0x825c4618`) records the award in the completion results. The native
/// first has the stats back end (module 14, `StatsBackEnd::GrantAward`
/// `0x8255eda0`) mark it achieved; that award table is not ported.
fn grant_award(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let completion = &mut world.signpost.completion;
    completion.award_unlocked = true;
    completion.award = binding::get_int(call, 0);
    Ok(Vec::new())
}

/// `Progression.IsChallengeComplete(name)` (`0x825d3e08`).
fn is_challenge_complete(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let key = key_from_name(&world.db, class::PROGRESSION_CHALLENGES, 3, &name_arg(call));
    Ok(vec![Value::Boolean(key & 0xffff_ffff != 0 && world.signpost.is_challenge_complete(key))])
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, super::lua::NativeFn<World>)> {
    vec![
        ("Progression", "ActivateGroup", activate_group_native),
        ("Progression", "DeactivateGroup", deactivate_group_native),
        ("Progression", "IncrementPointPoolPoints", increment_point_pool_points),
        ("ChallengeProgression", "IncrementPointPoolPoints", increment_point_pool_points),
        ("Progression", "PointPoolPoints", point_pool_points),
        ("Progression", "CheckRanking", check_ranking_native),
        ("Progression", "RankingEarned", ranking_earned_native),
        ("Progression", "CheckLastCompletedChallengeTitle", check_last_completed_challenge_title),
        ("Progression", "CheckMessageSettingString", check_message_setting_string),
        ("Progression", "IsChallengeActive", is_challenge_active),
        ("Progression", "IsChallengeComplete", is_challenge_complete),
        ("ChallengeProgression", "UpdateCashAward", update_cash_award),
        ("Award", "GrantAward", grant_award),
    ]
}

// ---- Memcard -------------------------------------------------------------------

/// The savable (`IMemcardSavable("SignPost")` in the `cSignPostManager`
/// constructor) and its leaves.
pub const SAVABLE: &str = "SignPost";
/// The savables registered before it (`BackEndManager::Init`, then the
/// `Sk8ChallengeSystem` constructor), in order.
pub const SAVABLES_BEFORE: [&str; 7] =
    ["UNLOCKABLES", "CAC", "SKATER_PROFILE", "online", "STATISTICS", "DYNAMICOBJECTS", "Sk8ChallengeSystem"];
pub const DATA: &str = "cSignPost::Data";
pub const DATA_VERSION: u32 = 14;
pub const POSITION: &str = "cSignPost::Position";
pub const MESSAGES: &str = "cSignPostMessageMan::Data";

/// A leaf `SaveData` writes: its name, version and tuples.
pub type Leaf = (&'static str, u32, Vec<Vec<u8>>);

/// Memcard tuple writes (`WriteUInt`/`WriteInt`/`WriteBool`/...): big-endian
/// values, one tuple each.
#[derive(Default)]
pub(crate) struct Writer(Vec<Vec<u8>>);

impl Writer {
    pub(crate) fn word(&mut self, value: u32) {
        self.0.push(value.to_be_bytes().to_vec());
    }
    pub(crate) fn int(&mut self, value: i32) {
        self.0.push(value.to_be_bytes().to_vec());
    }
    pub(crate) fn boolean(&mut self, value: bool) {
        self.0.push(vec![u8::from(value)]);
    }
    pub(crate) fn uint64(&mut self, value: u64) {
        self.0.push(value.to_be_bytes().to_vec());
    }
    pub(crate) fn float(&mut self, value: f32) {
        self.0.push(value.to_be_bytes().to_vec());
    }
    pub(crate) fn into_tuples(self) -> Vec<Vec<u8>> {
        self.0
    }
}

/// Sequential reads over one leaf; a missing or malformed tuple reads 0.
pub(crate) struct Reader<'a> {
    tuples: &'a [Vec<u8>],
    next: usize,
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(tuples: &'a [Vec<u8>]) -> Self {
        Self { tuples, next: 0 }
    }
    fn take(&mut self) -> Option<&'a [u8]> {
        let t = self.tuples.get(self.next)?;
        self.next += 1;
        Some(t)
    }
    pub(crate) fn word(&mut self) -> u32 {
        self.take().and_then(|t| t.try_into().ok()).map_or(0, u32::from_be_bytes)
    }
    pub(crate) fn int(&mut self) -> i32 {
        self.word().cast_signed()
    }
    pub(crate) fn boolean(&mut self) -> bool {
        self.take().is_some_and(|t| t.first().is_some_and(|&b| b != 0))
    }
    pub(crate) fn uint64(&mut self) -> u64 {
        self.take().and_then(|t| t.try_into().ok()).map_or(0, u64::from_be_bytes)
    }
    pub(crate) fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}

/// Whether `SaveProgressionOrientation` (`0x825c7448`) asks the local
/// skater's breadcrumbs: a career (`GameTypeFlags & 4`) in the main world
/// (`sGameInfo.additionalWorldLocation` -1, 0x825c74b4) with completed
/// challenges, unless `QuittingOverride` (the start hub's Quit Game) set the
/// orientation. In an additional world the saved orientation stays as it was.
#[must_use]
pub fn asks_progression_orientation(world: &World, in_additional_world: bool) -> bool {
    world.signpost.game_type_flags & 4 == 4
        && !in_additional_world
        && !world.signpost.progress.use_override_because_quitting
        && !world.signpost.completed.is_empty()
}

/// `SaveProgressionOrientation`: the suggested respawn crumb's transform
/// rows become the valid orientation, which stays valid for later saves.
pub fn set_progression_orientation(world: &mut World, rows: [[f32; 3]; 4]) {
    world.signpost.progress.orientation = (true, rows);
}

/// `SaveData` (`0x825cdef8`): `cSignPost::Data`, then
/// `SaveProgressionOrientation` (`0x825c7448`) and
/// `cSignPostMessageMan::SaveData` (`0x825c6dd8`). Each table is written in
/// key order and padded to its fixed size with zeroed entries.
#[must_use]
pub fn save(world: &World) -> [Leaf; 3] {
    let db = &world.db;
    let sp = &world.signpost;
    let p = &sp.progress;
    let mut w = Writer::default();
    let mut available = sp.available.iter();
    for _ in 0..2048 {
        let (id, new) = available.next().map_or((0, false), |(&k, &new)| (challenge_id(db, k), new));
        w.word(id);
        w.boolean(new);
    }
    let mut groups = p.groups.iter();
    for _ in 0..80 {
        w.word(groups.next().map_or(0, |&g| db.layout_word(class::PROGRESSION_GROUPS, g, 2)));
    }
    let mut completed = sp.completed.iter();
    for _ in 0..2048 {
        let (id, order, photo) =
            completed.next().map_or((0, 0, 0), |(&k, c)| (challenge_id(db, k), c.order, c.photo_caption));
        w.word(id);
        w.int(order);
        w.int(photo);
    }
    // The second word is the point pool lookup of the exit's number (word
    // 2), as the native writes it.
    let mut exits = p.group_exits.iter();
    for _ in 0..80 {
        let (group, pool) = exits.next().map_or((0, 0), |&exit| {
            let number = i64::from(layout_int(db, classes::GROUP_EXITS, exit, 2)).cast_unsigned();
            (db.layout_word(classes::GROUP_EXITS, exit, 1), pool_points(world, number))
        });
        w.word(group);
        w.int(pool);
    }
    let mut counts = p.group_counts.iter();
    for _ in 0..80 {
        let (id, count) = counts.next().map_or((0, 0), |(&g, &c)| (db.layout_word(class::PROGRESSION_GROUPS, g, 2), c));
        w.word(id);
        w.int(count);
    }
    let mut pools = p.point_pools.keys();
    for _ in 0..80 {
        let (id, points) =
            pools.next().map_or((0, 0), |&pool| (db.layout_word(classes::POINT_POOLS, pool, 1), pool_points(world, pool)));
        w.word(id);
        w.int(points);
    }
    let mut rankings = p.rankings.iter();
    for _ in 0..32 {
        let (group, ranking) = rankings.next().map_or((0, 0), |(&g, &r)| {
            (db.layout_word(classes::RANKING_GROUPS, g, 1), db.layout_word(classes::RANKINGS, r, 1))
        });
        w.word(group);
        w.word(ranking);
    }
    let mut saves = p.challenge_save.values();
    for _ in 0..512 {
        let (id, kind, value) = saves.next().map_or((0, 0, 0), |s| {
            (challenge_id(db, s.challenge), db.layout_word(classes::CHALLENGE_SAVE_TYPES, s.save_type, 3), s.value)
        });
        w.word(id);
        w.word(kind);
        w.int(value);
    }
    for _ in 0..256 {
        w.uint64(0);
        w.int(0);
    }
    let mut blog = p.blog.iter();
    for _ in 0..512 {
        let (id, kind, order, new) =
            blog.next().map_or((0, 0, 0, false), |(&(kind, key), b)| (blog_id(db, kind, key), kind, b.order, b.new));
        w.word(id);
        w.word(kind);
        w.int(order);
        w.boolean(new);
    }
    let mut news = p.news.iter();
    for _ in 0..32 {
        w.word(news.next().map_or(0, |&n| db.layout_word(classes::MESSAGES, n, 2)));
    }
    let mut position = Writer::default();
    position.boolean(p.orientation.0);
    for row in p.orientation.1 {
        for v in row {
            position.float(v);
        }
    }
    let mut messages = Writer::default();
    let data = sp.message_data();
    messages.word(160);
    messages.word(u32::try_from(data.len()).unwrap_or(u32::MAX));
    for (&key, &value) in data {
        messages.uint64(key);
        messages.word(value);
    }
    for _ in data.len()..160 {
        messages.uint64(0);
        messages.word(0xdead_beef);
    }
    [(DATA, DATA_VERSION, w.0), (POSITION, 1, position.0), (MESSAGES, 1, messages.0)]
}

/// One saved leaf: its version and tuples.
pub type Saved<'a> = Option<(u32, &'a [Vec<u8>])>;

/// `LoadData` (`0x825e53d8`): `LoadProgressionData(14)` (`0x825e41d0`),
/// `LoadProgressionOrientation` (`0x825c4340`) and
/// `cSignPostMessageMan::LoadData` (`0x825d8628`).
pub fn load(world: &mut World, data: Saved<'_>, position: Saved<'_>, messages: Saved<'_>) {
    if let Some((DATA_VERSION, tuples)) = data {
        load_progression_data(world, tuples);
    }
    if let Some((1, tuples)) = position {
        let mut r = Reader::new(tuples);
        let valid = r.boolean();
        let mut rows = IDENTITY;
        for row in &mut rows {
            for v in row.iter_mut() {
                *v = r.float();
            }
        }
        world.signpost.progress.orientation = (valid, rows);
    }
    if let Some((1, tuples)) = messages {
        let mut r = Reader::new(tuples);
        let _ = r.word();
        for _ in 0..r.word() {
            let key = r.uint64();
            let value = r.word();
            world.signpost.set_message_data(key, value);
        }
    }
    super::map_icons::check_pinch_points(world);
}

/// `LoadProgressionData` (`0x825e41d0`): a fresh `DeInit`/`Init`, then each
/// saved table re-applied with `DuringInit` set until the BLOG.
fn load_progression_data(world: &mut World, tuples: &[Vec<u8>]) {
    super::signpost::reinit(world);
    let db = Arc::clone(&world.db);
    world.signpost.set_during_init(true);
    let mut r = Reader::new(tuples);
    for _ in 0..2048 {
        let id = r.word();
        let new = r.boolean();
        if id != 0 {
            let key = challenge_key_from_id(&db, id);
            if key & 0xffff_ffff != 0 {
                super::signpost::activate_challenge(world, key, true, false, new);
            }
        }
    }
    for _ in 0..80 {
        let id = r.word();
        if id != 0 {
            activate_group(world, key_from_id(&db, class::PROGRESSION_GROUPS, 2, id));
        }
    }
    for _ in 0..2048 {
        let id = r.word();
        let order = r.int();
        let photo = r.int();
        if id != 0 {
            super::signpost::update_completed_challenge_map(world, challenge_key_from_id(&db, id), order, photo);
        }
    }
    for _ in 0..80 {
        let group = r.word();
        let number = r.word();
        if group != 0 && number != 0 {
            // GetGroupExitKeyFromID 0x825c8b18.
            let class = classes::GROUP_EXITS;
            let exit = db
                .collections(class)
                .iter()
                .copied()
                .find(|&k| db.layout_word(class, k, 1) == group && db.layout_word(class, k, 2) == number)
                .unwrap_or(0);
            world.signpost.progress.group_exits.insert(exit);
        }
    }
    for _ in 0..80 {
        let id = r.word();
        let count = r.int();
        if id != 0 && count != 0 {
            let group = key_from_id(&db, class::PROGRESSION_GROUPS, 2, id);
            if let Some(current) = world.signpost.progress.group_counts.get_mut(&group) {
                *current = if count == -1 { *current + 1 } else { count };
            }
        }
    }
    for _ in 0..80 {
        let id = r.word();
        let points = r.int();
        if id != 0 {
            let pool = key_from_id(&db, classes::POINT_POOLS, 1, id);
            if world.signpost.progress.point_pools.contains_key(&pool) {
                let minimum = layout_int(&db, classes::POINT_POOLS, pool, 2);
                let maximum = layout_int(&db, classes::POINT_POOLS, pool, 3);
                world.signpost.progress.point_pools.insert(pool, points.max(minimum).min(maximum));
                trigger_state_machines(world, classes::POINT_POOLS, pool);
            }
        }
    }
    for _ in 0..32 {
        let group_id = r.word();
        let ranking_id = r.word();
        if group_id != 0 {
            let group = key_from_id(&db, classes::RANKING_GROUPS, 1, group_id);
            let ranking = key_from_id(&db, classes::RANKINGS, 1, ranking_id);
            if let Some(current) = world.signpost.progress.rankings.get_mut(&group) {
                *current = ranking;
            }
        }
    }
    for _ in 0..512 {
        let challenge = r.word();
        let kind = r.word();
        let value = r.int();
        if challenge != 0 && kind != 0 {
            // UpdateChallengeSaveTypeData 0x825c7ea8: the first entry of that
            // challenge id and save type id.
            if let Some(entry) = world.signpost.progress.challenge_save.values_mut().find(|s| {
                challenge_id(&db, s.challenge) == challenge
                    && db.layout_word(classes::CHALLENGE_SAVE_TYPES, s.save_type, 3) == kind
            }) {
                entry.value = value;
            }
        }
    }
    for _ in 0..256 {
        r.uint64();
        r.int();
    }
    world.signpost.set_during_init(false);
    for _ in 0..512 {
        let id = r.word();
        let kind = r.word();
        let order = r.int();
        let new = r.boolean();
        if id != 0 && order != 0 {
            add_blog_entry(world, kind, id, order, new);
        }
    }
    for _ in 0..32 {
        let id = r.word();
        if id != 0 {
            world.signpost.progress.news.insert(key_from_id(&db, classes::MESSAGES, 2, id));
        }
    }
}
