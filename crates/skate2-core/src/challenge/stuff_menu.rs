//! The career's map, `Sk8::FE::FrontEndState_ChallengeMap` (FE state 21, screen 14
//! `challengemap.apt`) in its Stuff region, and its `Sk8::FE::FEStuffMenu`: the challenge
//! categories with their counts and items. Highlighting a category shows its items' map icons
//! (`FilterIcons` through `MapBackEndManager::StuffMenuFilter`), highlighting an item draws its
//! icon twice as large (`EnlargeIcon`), and selecting one opens the info component, whose option
//! teleports there. A Property item's info component buys the property, through the purchase
//! popups, or teleports to an owned one.
//!
//! The career runs with `GameBackEnd::IsFreeSkateMode` false. The Map and News regions (pan, zoom,
//! district title, news panel), the map cursor's pan onto a highlighted item (`FEMapCursor`), the
//! Create-a-Spots, downloaded content, the Big Black dismissal popup and the
//! chase test of the info option are not ported.

use super::{
    keys::{class, hash64},
    map_icons::{self, MapIcon},
    signpost_progress,
    system::World,
    unlockables,
};

/// `eFEStuffMenuCategory`.
pub mod category {
    pub const LEAF: i32 = -1;
    pub const ROOT: i32 = 0;
    pub const CAREER: i32 = 1;
    pub const RACES: i32 = 2;
    pub const SCONTESTS: i32 = 3;
    pub const TCONTESTS: i32 = 4;
    pub const TUTORIAL: i32 = 5;
    pub const BONUS: i32 = 6;
    pub const SPOTS: i32 = 7;
    pub const CREATE_A_SPOTS: i32 = 8;
    pub const PROPERTY: i32 = 9;
    pub const LOCATIONS: i32 = 10;
    pub const DOWNLOADED: i32 = 11;
    pub const HALL_OF_MEAT: i32 = 12;
    pub const FOLLOW: i32 = 13;
    pub const SMAG: i32 = 14;
    pub const THRASHER: i32 = 15;
    pub const FILM: i32 = 16;
    pub const DEMO: i32 = 17;
    pub const PRO: i32 = 18;
    pub const LANDMARKS: i32 = 19;
    pub const POOLS: i32 = 20;
    pub const PLAZAS: i32 = 21;
    pub const SKATEPARKS: i32 = 22;
    pub const SPAWN_POINTS: i32 = 23;
    pub const DLC_SPAWN_POINTS: i32 = 24;
    pub const DLC_CAREER: i32 = 25;
    /// `eFEStuffMenuCategory_Max`.
    pub const MAX: i32 = 26;
}
use category as cat;

/// `FEStuffMenu::gSubCategory` (`0x82079860`): each category's sub-categories up to its
/// `eFEStuffMenuCategory_None` (-99).
const SUB_CATEGORIES: [&[i32]; 26] = [
    &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 11],
    &[13, 14, 15, 16, 17, 18],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[19, 20, 21, 22, 23],
    &[24, 25],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
];

/// `gStuffCategoryHAL` (`0x82fdb2f0`).
pub const CATEGORY_HAL: [&str; 26] = [
    "ID_COMMON_UNKNOWN",
    "ID_STUFFMENU_CATEGORY_CAREER",
    "ID_STUFFMENU_CATEGORY_RACES",
    "ID_STUFFMENU_CATEGORY_SCONTESTS",
    "ID_STUFFMENU_CATEGORY_TCONTESTS",
    "ID_STUFFMENU_CATEGORY_TUTORIAL",
    "ID_STUFFMENU_CATEGORY_BONUS",
    "ID_STUFFMENU_CATEGORY_SPOTS",
    "ID_STUFFMENU_CATEGORY_CREATEASPOTS",
    "ID_STUFFMENU_CATEGORY_PROPERTY",
    "ID_STUFFMENU_CATEGORY_LOCATIONS",
    "ID_STUFFMENU_CATEGORY_DOWNLOADED",
    "ID_STUFFMENU_CATEGORY_HALLOFMEAT",
    "ID_STUFFMENU_CATEGORY_FOLLOW",
    "ID_STUFFMENU_CATEGORY_SMAG",
    "ID_STUFFMENU_CATEGORY_THRASHER",
    "ID_STUFFMENU_CATEGORY_FILM",
    "ID_STUFFMENU_CATEGORY_DEMO",
    "ID_STUFFMENU_CATEGORY_PRO",
    "ID_STUFFMENU_CATEGORY_LANDMARKS",
    "ID_STUFFMENU_CATEGORY_LANDMARK_POOLS",
    "ID_STUFFMENU_CATEGORY_LANDMARK_PLAZAS",
    "ID_STUFFMENU_CATEGORY_LANDMARK_PARKS",
    "ID_STUFFMENU_CATEGORY_SPAWNPOINTS",
    "ID_STUFFMENU_CATEGORY_DLC_SPAWNPOINTS",
    "ID_STUFFMENU_CATEGORY_DLC_CAREER",
];

/// `GetCategoryHAL` of the root, and the title the screen's constructor gives the Stuff panel.
pub const ROOT_HAL: &str = "ID_CHALLENGE_MAP_STUFF";
const UNKNOWN_HAL: &str = "ID_COMMON_UNKNOWN";

/// Attribute keys the menu reads.
mod field {
    /// `challenges.Title`.
    pub const TITLE: u64 = 0xe2a5_0788_b6c2_0c92;
    /// `challenges.Description`.
    pub const DESCRIPTION: u64 = 0x6b0b_d2ba_e508_b742;
    /// `challenge_global_data.FELargeImage`.
    pub const FE_LARGE_IMAGE: u64 = 0x7974_27ed_230c_9980;
    /// `challenge_global_data.ChallengeIcon`, `ChallengeType`, `Location` and the `tLocationID`
    /// read before `Location`.
    pub const CHALLENGE_ICON: u64 = 0xe616_acdf_5a36_bfac;
    pub const CHALLENGE_TYPE: u64 = 0x02b8_28cc_db47_85e4;
    pub const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
    pub const ICON_LOCATION: u64 = 0x0717_c5a8_5c96_fd90;
    /// `challenge_global_data`'s world location, the `fe_locations` RefSpec
    /// `TeleportLocalPlayerStartChallenge` (`0x828a9678`) reads.
    pub const WORLD_LOCATION: u64 = 0x6c81_4db5_9f16_ebd2;
    /// `fe_locations.WorldFile` and `IsDLC`.
    pub const WORLD_FILE: u64 = 0x116c_a244_49c4_ce44;
    pub const IS_DLC: u64 = 0xd0cc_ffee_3455_65a4;
    /// The `challenge_local_data` challenges that must all be complete before
    /// `PopulateMenuItemsVector` lists a challenge.
    pub const MENU_PREREQUISITES: u64 = 0xecec_13ce_a289_9d81;
    /// `progression_lists.Challenges`.
    pub const LIST_CHALLENGES: u64 = 0xec1c_213b_bdbf_1a68;
    /// `fe_location_group.Location`.
    pub const GROUP_LOCATIONS: u64 = 0xf1c6_d5fa_4a23_951e;
}

/// `Attrib::Gen::fe_location_group`.
const FE_LOCATION_GROUP: u64 = 0xd897_e7db_651e_58dc;
/// The `fe_locations` collection whose leaf descendants `PopulateMenuItemsVector_DlcSpawnPoints`
/// lists.
const DLC_SPAWN_POINTS: u64 = 0x4782_50f6_7442_f962;
/// The `progression_challenges` layout byte that keeps a challenge out of
/// `GetAllChallengesInList` (`0x825d69e0`).
const UNLISTED_BYTE: u32 = 40;
/// `fe_map_icons` layout byte 5, `DisplayOnFEMap`.
const DISPLAY_ON_FE_MAP_BYTE: u32 = 5;
/// `eChallengeTypes` the map tests: Own the Spot, and the type the completed view leaves out.
const TYPE_OWN_THE_SPOT: i32 = 9;
const TYPE_UNLISTED_COMPLETED: i32 = 30;
/// `kFEMapEventIconSize` (`0x83049b08`, default 32).
pub const MAP_EVENT_ICON_SIZE: f32 = 32.0;

/// The `progression_lists` ids (layout word 1) `PopulateMenus` lists.
mod list {
    pub const CAREER: i32 = 96;
    pub const FILM: i32 = 90;
    pub const FOLLOW: i32 = 105;
    pub const SMAG: i32 = 89;
    pub const THRASHER: i32 = 97;
    pub const PRO: i32 = 93;
    pub const DEMO: i32 = 91;
    pub const SCONTESTS: i32 = 87;
    pub const TCONTESTS: i32 = 88;
    pub const RACES: i32 = 62;
    pub const BONUS: i32 = 86;
    pub const TUTORIAL: i32 = 85;
    pub const SPOTS: i32 = 61;
    pub const LOCATIONS: i32 = 59;
    pub const POOLS: i32 = 57;
    pub const PROPERTY: i32 = 74;
    pub const HALL_OF_MEAT: i32 = 70;
}

/// `FEStuffMenuItem`; its `visibility` is always `eFEIconVisibility_Unknown`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub hal_id: String,
    pub key: u64,
}

fn slot(category: i32) -> Option<usize> {
    usize::try_from(category)
        .ok()
        .filter(|&i| i < SUB_CATEGORIES.len())
}

/// `cSignPostUtil::GetListKeyFromListID` (`0x825c8d70`): the first `progression_lists`
/// collection, in `GetCollections` order, whose id (layout word 1) matches; 0 without one.
pub(crate) fn list_key(world: &World, id: i32) -> u64 {
    let db = &world.db;
    db.table_order(class::PROGRESSION_LISTS)
        .into_iter()
        .find(|&key| {
            db.layout_word(class::PROGRESSION_LISTS, key, 1)
                .cast_signed()
                == id
        })
        .unwrap_or(0)
}

/// `GetAllChallengesInList(listID, list, asNeutrinoKeys = true, sortedByCompletion = false)`
/// (`0x825d7c10`, `0x825d69e0`): the list's `Challenges` whose layout byte 40 is clear, as their
/// `challenges` keys (`ConvertSignpostKeyToChallengeKey`), those without one left out.
pub(crate) fn challenges_in_list(world: &World, id: i32) -> Vec<u64> {
    let db = &world.db;
    let Some(refs) = db.attribute(
        class::PROGRESSION_LISTS,
        list_key(world, id),
        field::LIST_CHALLENGES,
    ) else {
        return Vec::new();
    };
    (0..refs.len())
        .filter_map(|i| refs.ref_spec(i).map(|(_, signpost)| signpost))
        .filter(|&signpost| {
            db.layout_byte(class::PROGRESSION_CHALLENGES, signpost, UNLISTED_BYTE) == 0
        })
        .filter_map(|signpost| map_icons::challenge_key(world, signpost))
        .collect()
}

/// `challenges.Title`, `ID_COMMON_UNKNOWN` without one.
fn title(world: &World, challenge: u64) -> String {
    world
        .db
        .attribute(class::CHALLENGES, challenge, field::TITLE)
        .and_then(|a| a.text(0))
        .unwrap_or(UNKNOWN_HAL)
        .to_owned()
}

fn global_i32(world: &World, activity: usize, key: u64) -> i32 {
    world.activities.activities[activity]
        .global_data
        .and_then(|(c, k)| world.db.attribute(c, k, key))
        .and_then(|a| a.i32(0))
        .unwrap_or(0)
}

fn global_text(world: &World, activity: usize, key: u64) -> Option<String> {
    let (c, k) = world.activities.activities[activity].global_data?;
    world.db.attribute(c, k, key)?.text(0).map(str::to_owned)
}

/// `FELocationManager::IsLocationUnlocked(key, includeDLCUnlocks)` (`0x82615868`) with
/// `kbUnlockFreeSkateSetting` off: an `fe_locations` collection whose name (layout word 0), or
/// `all_locations_unlock`, is unlocked among the locations.
fn is_location_unlocked(world: &World, location: u64, include_dlc: bool) -> bool {
    if !world.db.has_collection(class::FE_LOCATIONS, location) {
        return false;
    }
    let name = world
        .db
        .layout_text(class::FE_LOCATIONS, location, 0)
        .unwrap_or_default();
    let unlocks = &world.unlockables;
    unlocks.is_unlocked(
        unlockables::category::LOCATIONS,
        unlockables::string_hash("all_locations_unlock"),
        include_dlc,
    ) || unlocks.is_unlocked(
        unlockables::category::LOCATIONS,
        unlockables::string_hash(name),
        include_dlc,
    )
}

/// What selecting an item asks of the map state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    None,
    /// `TriggerInfoComponent(challengeKey)` (`0x8265f070`).
    Challenge(u64),
    /// `TriggerInfoComponent_FELocation(locationKey)` (`0x8265f180`).
    Location(u64),
    /// `TriggerInfoComponent_PurchaseProperty(challengeKey)` (`0x8265f0f8`), which
    /// `SelectMenuItemIn_GenericFEStuffMenuItem` (`0x826297e8`) calls for a Property item.
    Property(u64),
    /// `TriggerHomPanel` (`0x8265f308`).
    HallOfMeat,
    /// `TriggerCreateASpotInfo`, which the port does not have.
    Unported,
}

/// `Sk8::FE::FEStuffMenu`.
#[derive(Clone, Debug)]
pub struct StuffMenu {
    /// `mCategoryList`.
    lists: Vec<Vec<Item>>,
    /// `mCategoryMapping`: the level's shown sub-categories, as `FilterCategory` left them.
    mapping: [i32; 26],
    /// `mPopulateWithActiveChallenge`: the available challenges rather than the completed ones.
    pub populate_active: bool,
    /// `mbHasCompletedChallenge`.
    pub has_completed: bool,
    /// `mMenuLevelHistory`.
    history: Vec<i32>,
    /// `mCurrFilterIndex`.
    filter_index: i32,
    /// `mIsHomActiveDetermined` and `mIsHomActive`.
    hom_active_determined: bool,
    hom_active: bool,
}

impl Default for StuffMenu {
    /// The constructor (`0x82635720`): the root level, the available challenges.
    fn default() -> Self {
        Self {
            lists: vec![Vec::new(); SUB_CATEGORIES.len()],
            mapping: [0; 26],
            populate_active: true,
            has_completed: false,
            history: vec![cat::ROOT],
            filter_index: 0,
            hom_active_determined: false,
            hom_active: false,
        }
    }
}

impl StuffMenu {
    /// `mMenuLevelHistory.back()`.
    #[must_use]
    pub fn level(&self) -> i32 {
        self.history.last().copied().unwrap_or(cat::ROOT)
    }

    /// The history's size.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.history.len()
    }

    /// A category's `mCategoryList`.
    #[must_use]
    pub fn items(&self, category: i32) -> &[Item] {
        slot(category).map_or(&[], |i| self.lists[i].as_slice())
    }

    fn clear(&mut self, category: i32) {
        if let Some(i) = slot(category) {
            self.lists[i].clear();
        }
    }

    fn push(&mut self, category: i32, hal_id: String, key: u64) {
        if let Some(i) = slot(category) {
            self.lists[i].push(Item { hal_id, key });
        }
    }

    /// `ClearMenus` (`0x82635b30`): every list, and the history back to the root.
    fn clear_menus(&mut self) {
        for list in &mut self.lists {
            list.clear();
        }
        self.history = vec![cat::ROOT];
    }

    /// `ResetMenu` (`0x82635bc0`).
    pub fn reset(&mut self) {
        self.clear_menus();
        self.populate_active = true;
        self.filter_index = 0;
    }

    /// `Initialize` (`0x82640d88`): the completed challenges are listed first, to learn whether
    /// the root's categories hold any (`mbHasCompletedChallenge`), then the available ones.
    pub fn initialize(&mut self, world: &World) {
        self.populate_active = false;
        self.populate_menus(world);
        let completed: usize = SUB_CATEGORIES[0]
            .iter()
            .map(|&sub| self.num_sub_menu_items(world, sub))
            .sum();
        self.has_completed = completed > 0;
        self.populate_active = true;
        self.populate_menus(world);
    }

    /// `PopulateMenus` (`0x8263e170`). Without `CreateASpotDataManager` light data the
    /// Create-a-Spot list stays empty.
    pub fn populate_menus(&mut self, world: &World) {
        let active = self.populate_active;
        self.populate_challenges(world);
        self.clear(cat::SPOTS);
        if active {
            self.populate_list(world, cat::SPOTS, list::SPOTS, 3);
        }
        self.clear(cat::CREATE_A_SPOTS);
        self.hom_active_determined = false;
        self.hom_active = false;
        self.clear(cat::LOCATIONS);
        if active {
            self.populate_list(world, cat::LOCATIONS, list::LOCATIONS, 3);
        }
        let groups = active
            && super::dynamic_hull::test_state(world, hash64("slappys"), "Unlock") == Some(true);
        if groups {
            for group in map_icons::LOCATION_GROUPS {
                self.populate_location_group(world, cat::LOCATIONS, group);
            }
        }
        let [general, plazas, skateparks, spawn_points] = map_icons::LOCATION_GROUPS;
        self.clear(cat::LANDMARKS);
        if groups {
            self.populate_location_group(world, cat::LANDMARKS, general);
        }
        self.clear(cat::POOLS);
        if active {
            self.populate_list(world, cat::POOLS, list::POOLS, 3);
        }
        for (category, group) in [
            (cat::PLAZAS, plazas),
            (cat::SKATEPARKS, skateparks),
            (cat::SPAWN_POINTS, spawn_points),
        ] {
            self.clear(category);
            if groups {
                self.populate_location_group(world, category, group);
            }
        }
        self.populate_dlc_spawn_points(world);
        self.populate_dlc_career(world);
        self.populate_property(world);
    }

    /// `PopulateChallenges` (`0x82634270`): the tutorials only among the available challenges.
    fn populate_challenges(&mut self, world: &World) {
        for category in [
            cat::CAREER,
            cat::FILM,
            cat::FOLLOW,
            cat::SMAG,
            cat::THRASHER,
            cat::PRO,
            cat::DEMO,
            cat::SCONTESTS,
            cat::TCONTESTS,
            cat::RACES,
            cat::TUTORIAL,
            cat::BONUS,
        ] {
            self.clear(category);
        }
        let active = self.populate_active;
        for (category, id) in [
            (cat::CAREER, list::CAREER),
            (cat::FILM, list::FILM),
            (cat::FOLLOW, list::FOLLOW),
            (cat::SMAG, list::SMAG),
            (cat::THRASHER, list::THRASHER),
            (cat::PRO, list::PRO),
            (cat::DEMO, list::DEMO),
            (cat::SCONTESTS, list::SCONTESTS),
            (cat::TCONTESTS, list::TCONTESTS),
            (cat::RACES, list::RACES),
            (cat::BONUS, list::BONUS),
        ] {
            self.populate_challenge_list(world, category, id, active);
        }
        if active {
            self.populate_challenge_list(world, cat::TUTORIAL, list::TUTORIAL, true);
        }
    }

    /// `PopulateMenuItemsVector_Challenges` (`0x82632418`): the list's available challenges (or
    /// completed ones not of type 30) whose `ChallengeIcon` shows on the map (`DisplayOnFEMap`).
    fn populate_challenge_list(
        &mut self,
        world: &World,
        category: i32,
        id: i32,
        active_view: bool,
    ) {
        for challenge in challenges_in_list(world, id) {
            let listed = if active_view {
                map_icons::is_active(world, challenge)
            } else {
                map_icons::is_complete(world, challenge)
            };
            let Some(activity) = world.activities.find(challenge) else {
                continue;
            };
            if !active_view
                && global_i32(world, activity, field::CHALLENGE_TYPE) == TYPE_UNLISTED_COMPLETED
            {
                continue;
            }
            let icon = world.activities.activities[activity]
                .global_data
                .and_then(|(c, k)| world.db.attribute(c, k, field::CHALLENGE_ICON))
                .and_then(|a| a.ref_spec(0))
                .map_or(0, |(_, key)| key);
            if listed
                && world
                    .db
                    .layout_byte(class::MAP_ICON, icon, DISPLAY_ON_FE_MAP_BYTE)
                    != 0
            {
                self.push(category, title(world, challenge), challenge);
            }
        }
    }

    /// `PopulateMenuItemsVector(category, listID, populateFlags)` (`0x826332a8`): the list's
    /// challenges (spots once found) whose menu prerequisites are all complete, available with
    /// flag 1 or complete with flag 2; a completed pool always counts as available.
    fn populate_list(&mut self, world: &World, category: i32, id: i32, flags: u32) {
        for challenge in challenges_in_list(world, id) {
            let Some(activity) = world.activities.find(challenge) else {
                continue;
            };
            if category == cat::SPOTS
                && signpost_progress::challenge_save_data(world, challenge, "Spot Found") == 0
            {
                continue;
            }
            let prerequisites_complete = world.activities.activities[activity]
                .local_data
                .and_then(|(c, k)| world.db.attribute(c, k, field::MENU_PREREQUISITES))
                .is_none_or(|a| {
                    (0..a.len()).all(|i| {
                        map_icons::is_complete(world, a.ref_spec(i).map_or(0, |(_, key)| key))
                    })
                });
            if !prerequisites_complete {
                continue;
            }
            let complete = map_icons::is_complete(world, challenge);
            let available = (map_icons::is_active(world, challenge) && flags & 1 == 1)
                || (category == cat::POOLS && complete);
            if available || (complete && flags & 2 == 2) {
                self.push(category, title(world, challenge), challenge);
            }
        }
    }

    /// `PopulateMenuItemsFromLocationGroup` (`0x82632cc0`): the group's existing, unlocked
    /// `Location` entries, named by their layout word 2.
    fn populate_location_group(&mut self, world: &World, category: i32, group: u64) {
        let db = &world.db;
        let Some(locations) = db.attribute(FE_LOCATION_GROUP, group, field::GROUP_LOCATIONS) else {
            return;
        };
        for (location_class, location) in (0..locations.len()).filter_map(|i| locations.ref_spec(i))
        {
            if db.has_collection(location_class, location)
                && is_location_unlocked(world, location, false)
            {
                let hal_id = db
                    .layout_text(location_class, location, 2)
                    .unwrap_or_default()
                    .to_owned();
                self.push(category, hal_id, location);
            }
        }
    }

    /// `PopulateMenuItemsVector_DlcSpawnPoints` (`0x82632e90`): the `fe_locations` leaves under
    /// the downloaded spawn points (`GetLeafCollections`), named by their layout word 2.
    fn populate_dlc_spawn_points(&mut self, world: &World) {
        self.clear(cat::DLC_SPAWN_POINTS);
        let db = &world.db;
        for location in
            crate::ui::freeskate_option::leaf_descendants(db, class::FE_LOCATIONS, DLC_SPAWN_POINTS)
        {
            let hal_id = db
                .layout_text(class::FE_LOCATIONS, location, 2)
                .unwrap_or_default()
                .to_owned();
            self.push(cat::DLC_SPAWN_POINTS, hal_id, location);
        }
    }

    /// `PopulateMenuItemsVector_DlcCareer` (`0x82632fe0`) out of free skate, with `PopulateMenus`'
    /// flags (1 for the available challenges, 2 for the completed): with an additional world
    /// location (`sGameInfo.additionalWorldLocation`) that is downloadable content's (`IsDLC`),
    /// its challenges (`cChallengeDLCManager::GetDLCChallenges` 0x825a8d18: the activities whose
    /// world location it is), available or complete, a completed Goto left out, whose
    /// `ChallengeIcon` shows on the map.
    fn populate_dlc_career(&mut self, world: &World) {
        self.clear(cat::DLC_CAREER);
        let Some(location) = world.additional_world_location else { return };
        let is_dlc = world.db.attribute(class::FE_LOCATIONS, location, field::IS_DLC).and_then(|a| a.bool(0)) == Some(true);
        if !is_dlc {
            return;
        }
        let flags = if self.populate_active { 1 } else { 2 };
        let challenges: Vec<u64> = world
            .activities
            .activities
            .iter()
            .filter(|activity| activity_world_location(world, activity) == Some(location))
            .map(|activity| activity.key)
            .collect();
        for challenge in challenges {
            let available = map_icons::is_active(world, challenge) && flags & 1 == 1;
            let completed = map_icons::is_complete(world, challenge) && flags & 2 == 2;
            if !available && !completed {
                continue;
            }
            let Some(activity) = world.activities.find(challenge) else { continue };
            if completed && global_i32(world, activity, field::CHALLENGE_TYPE) == TYPE_UNLISTED_COMPLETED {
                continue;
            }
            let icon = world.activities.activities[activity]
                .global_data
                .and_then(|(c, k)| world.db.attribute(c, k, field::CHALLENGE_ICON))
                .and_then(|a| a.ref_spec(0))
                .map_or(0, |(_, key)| key);
            if world.db.layout_byte(class::MAP_ICON, icon, DISPLAY_ON_FE_MAP_BYTE) == 1 {
                self.push(cat::DLC_CAREER, title(world, challenge), challenge);
            }
        }
    }

    /// `PopulateMenuItemsVector_Property` (`0x826328a8`): among the available challenges, the
    /// property list's available or completed challenges with a `PropertyCost`.
    fn populate_property(&mut self, world: &World) {
        self.clear(cat::PROPERTY);
        if !self.populate_active {
            return;
        }
        for challenge in challenges_in_list(world, list::PROPERTY) {
            let listed =
                map_icons::is_active(world, challenge) || map_icons::is_complete(world, challenge);
            if listed
                && signpost_progress::challenge_economy_data(world, challenge, "PropertyCost") > 0
            {
                self.push(cat::PROPERTY, title(world, challenge), challenge);
            }
        }
    }

    /// `GetNumSubCategories` (`0x82607308`).
    #[must_use]
    pub fn num_sub_categories(category: i32) -> usize {
        slot(category).map_or(0, |i| SUB_CATEGORIES[i].len())
    }

    /// `GetNumSubMenuItems` (`0x8261c8c0`): the root sums its categories; the career counts with
    /// `kEnableProgression`; the downloaded category counts its spawn points and career as one
    /// each, the career only with spawn points; the Hall of Meat counts one in the available view
    /// once a challenge of its list is available, which it determines once per population.
    pub fn num_sub_menu_items(&mut self, world: &World, category: i32) -> usize {
        match category {
            cat::ROOT => SUB_CATEGORIES[0]
                .iter()
                .map(|&sub| self.num_sub_menu_items(world, sub))
                .sum(),
            cat::CAREER => {
                if world.enable_progression {
                    self.items(cat::CAREER).len()
                } else {
                    0
                }
            }
            cat::DOWNLOADED => {
                if self.items(cat::DLC_SPAWN_POINTS).is_empty() {
                    0
                } else if self.items(cat::DLC_CAREER).is_empty() {
                    1
                } else {
                    2
                }
            }
            cat::HALL_OF_MEAT => {
                if !self.hom_active_determined {
                    if challenges_in_list(world, list::HALL_OF_MEAT)
                        .into_iter()
                        .any(|key| map_icons::is_active(world, key))
                    {
                        self.hom_active = true;
                    }
                    self.hom_active_determined = true;
                }
                usize::from(self.hom_active && self.populate_active)
            }
            _ => self.items(category).len(),
        }
    }

    /// `FilterCategory` (`0x82629430`): the category's sub-categories with items, in
    /// `gSubCategory` order, become `mCategoryMapping` (the rest -1); their count.
    pub fn filter_category(&mut self, world: &World, category: i32) -> usize {
        self.mapping = [-1; 26];
        let Some(subs) = slot(category).map(|i| SUB_CATEGORIES[i]) else {
            return 0;
        };
        let mut count = 0;
        for &sub in subs {
            if self.num_sub_menu_items(world, sub) > 0 {
                self.mapping[count] = sub;
                count += 1;
            }
        }
        count
    }

    /// `GetNumMenuItems` (`0x826294f0`).
    pub fn num_menu_items(&mut self, world: &World) -> usize {
        let level = self.level();
        if level == cat::LEAF {
            1
        } else if Self::num_sub_categories(level) == 0 {
            self.num_sub_menu_items(world, level)
        } else {
            self.filter_category(world, level)
        }
    }

    /// `GetCategoryHAL` (`0x8261cb48`).
    pub fn category_hal(&mut self, world: &World, category: i32) -> MenuText {
        match slot(category) {
            Some(0) => MenuText::Copied(ROOT_HAL.to_owned()),
            Some(i) => MenuText::Category {
                hal: CATEGORY_HAL[i],
                count: self.num_sub_menu_items(world, category),
            },
            None => MenuText::Empty,
        }
    }

    /// `GetChallengeHAL` (`0x82639770`): the item's HAL id through `FormatStringCustom`; nothing
    /// at the root, a leaf or outside the list (a Create-a-Spot names its spot, and there are
    /// none).
    #[must_use]
    pub fn challenge_hal(&self, index: usize) -> MenuText {
        match self.level() {
            cat::LEAF | cat::ROOT | cat::MAX | cat::CREATE_A_SPOTS => MenuText::Empty,
            level => self
                .items(level)
                .get(index)
                .map_or(MenuText::Empty, |item| {
                    MenuText::Formatted(item.hal_id.clone())
                }),
        }
    }

    /// `ChallengeMapCmdTranslator::GetStuffMenuItems` (`0x8263c8b8`): `GetNumMenuItems` rows, a
    /// level of categories naming its shown sub-categories, a level of items its items.
    pub fn menu_items(&mut self, world: &World) -> Vec<MenuText> {
        let count = self.num_menu_items(world);
        let level = self.level();
        (0..count)
            .map(|i| {
                if Self::num_sub_categories(level) == 0 {
                    self.challenge_hal(i)
                } else if self.mapping[i] != -1 {
                    self.category_hal(world, self.mapping[i])
                } else {
                    MenuText::Empty
                }
            })
            .collect()
    }

    /// `FilterIcons(index, mapped)` (`0x82607388`): a mapped index names a shown sub-category,
    /// whose items' map icons show (`StuffMenuFilter`); the index is kept for `FilterIcons()`.
    pub fn filter_icons(&mut self, world: &mut World, index: i32, mapped: bool) {
        let category = if mapped {
            self.mapping[usize::try_from(index).ok().filter(|&i| i < 26).unwrap_or(0)]
        } else {
            index
        };
        if (1..=25).contains(&category) {
            let keys: Vec<u64> = self.items(category).iter().map(|item| item.key).collect();
            world.map_icons.stuff_menu_filter(&keys);
            self.filter_index = index;
        }
    }

    /// `FilterIcons()` (`0x82618718`): again for the last filtered index.
    pub fn refilter_icons(&mut self, world: &mut World) {
        self.filter_icons(world, self.filter_index, true);
    }

    /// `HighliteMenuItem` (`0x82629560`): at a level of items other than the Hall of Meat, the
    /// item's icon is drawn enlarged once its location is known
    /// (`HighliteMenuItemIn_GenericFEStuffMenuItem` `0x8261cc28`: the challenge's `tLocationID`,
    /// else its `Location`; `HighlightFELocationItem` `0x8261ceb8`: the location's layout word 1).
    pub fn highlight(&mut self, world: &mut World, index: usize) {
        let level = self.level();
        if Self::num_sub_categories(level) != 0
            || self.num_sub_menu_items(world, level) == 0
            || level == cat::HALL_OF_MEAT
        {
            return;
        }
        let Some(key) = self.items(level).get(index).map(|item| item.key) else {
            return;
        };
        let located = match level {
            cat::LANDMARKS | cat::PLAZAS | cat::SKATEPARKS | cat::SPAWN_POINTS => {
                world.db.layout_text(class::FE_LOCATIONS, key, 1).is_some()
            }
            _ => world.activities.find(key).is_some_and(|activity| {
                let icon_location =
                    global_text(world, activity, field::ICON_LOCATION).unwrap_or_default();
                !icon_location.is_empty() || global_text(world, activity, field::LOCATION).is_some()
            }),
        };
        if located {
            world.map_icons.enlarge_icon(key);
        }
    }

    /// `SelectMenuItem` (`0x826357f0`): a shown sub-category opens as the next level; an item of a
    /// challenge with an activity, or of a location, asks for its info component.
    pub fn select(&mut self, world: &World, index: usize) -> Selection {
        let level = self.level();
        if level == cat::ROOT && self.mapping.get(index) == Some(&cat::HALL_OF_MEAT) {
            return Selection::HallOfMeat;
        }
        if Self::num_sub_categories(level) == 0 {
            let Some(key) = self.items(level).get(index).map(|item| item.key) else {
                return Selection::None;
            };
            return match level {
                cat::LANDMARKS
                | cat::PLAZAS
                | cat::SKATEPARKS
                | cat::SPAWN_POINTS
                | cat::DLC_SPAWN_POINTS => Selection::Location(key),
                cat::CREATE_A_SPOTS => Selection::Unported,
                cat::ROOT | cat::HALL_OF_MEAT => Selection::None,
                _ if world.activities.find(key).is_none() => Selection::None,
                cat::PROPERTY => Selection::Property(key),
                _ => Selection::Challenge(key),
            };
        }
        if let Some(&sub) = self.mapping.get(index).filter(|&&sub| sub != -1) {
            self.history.push(sub);
        }
        Selection::None
    }

    /// `PreviousMenuLevel` (`0x826296b8`): back one level from a leaf or a category; the size of
    /// the history.
    pub fn previous_level(&mut self) -> usize {
        let level = self.level();
        if level == cat::LEAF || (1..cat::MAX).contains(&level) {
            self.history.pop();
        }
        self.history.len()
    }

    /// `ToggleListActiveOrComplete` (`0x8263e468`): at the root, with completed challenges, the
    /// other view is listed and its icons filtered.
    pub fn toggle_active_or_complete(&mut self, world: &mut World) -> bool {
        if self.history.len() != 1 || !self.has_completed {
            return false;
        }
        self.populate_active = !self.populate_active;
        self.populate_menus(world);
        self.refilter_icons(world);
        true
    }
}

/// A Stuff panel text as `GetCategoryHAL` and `GetChallengeHAL` write it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuText {
    /// Nothing written.
    Empty,
    /// A string copied as it is: the root's `ID_CHALLENGE_MAP_STUFF`, a location's name.
    Copied(String),
    /// `FormatString` of a `gStuffCategoryHAL` id, the count then printed into it.
    Category { hal: &'static str, count: usize },
    /// `FormatStringCustom` of a HAL id.
    Formatted(String),
}

/// `FrontEndState_ChallengeMap::eMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Select,
    Active,
    InfoComponent,
    HallOfMeat,
}

/// `UpdateLetterBox`'s configurations (screen states 50..54, 71, 72).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Letterbox {
    /// `kLBConfig_Basic`: B Back, A Select.
    #[default]
    Basic,
    /// `kLBConfig_Stuff`: B Back, A Select and X naming the other view
    /// (`GetStuffMenuChallengeFilterType` `0x826237c8`, `mPopulateWithActiveChallenge`).
    Stuff { populate_active: bool },
    /// `kLBConfig_Back`: B Back.
    Back,
}

/// The info component's subject (`mInfoCompSubState` with `mSelectedChallenge`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Info {
    Challenge(u64),
    Location(u64),
    /// `eInfoComponentSubstate_PurchaseProperty`.
    Property(u64),
}

/// A popup the map opens (`FrontEndManager::OpenPopup`, its `gPopupTemplate` `0x82fdbc58` row):
/// style 0, not cancellable, the first option selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapPopup {
    /// `eSk8FEPopup_Property_PurchaseFailed` (215), without a callback.
    PurchaseFailed,
    /// `eSk8FEPopup_Property_PurchaseConfirm` (216), answered by `ConfirmPurchaseCallback`.
    PurchaseConfirm,
    /// `eSk8FEPopup_BB_DismissalConfirm` (217), answered by `DismissAndTeleportCallback`.
    DismissBigBlack,
}

impl MapPopup {
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::PurchaseFailed | Self::DismissBigBlack => "ID_COMMON_WARNING_TITLE",
            Self::PurchaseConfirm => "ID_CIC_PURCHASE_PROPERTY",
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::PurchaseFailed => "ID_PROPERTY_PURCHASE_FAILED",
            Self::PurchaseConfirm => "ID_PAUSEMENU_CONFIRM",
            Self::DismissBigBlack => "ID_BB_DISMISSAL_CONFIRM",
        }
    }

    #[must_use]
    pub const fn options(self) -> &'static [&'static str] {
        match self {
            Self::PurchaseFailed => &["ID_COMMON_OK"],
            Self::PurchaseConfirm | Self::DismissBigBlack => &["ID_COMMON_NO", "ID_COMMON_YES"],
        }
    }
}

/// What the map asks of the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapAction {
    None,
    /// `PopState`.
    Close,
    /// The info option's fade, then `DoTeleport` (`0x8265f280`): `TeleportToChallenge`
    /// (`0x8256dc10`) to the challenge's `Location` spawn point, or `TeleportToFELocation`
    /// (`0x82565350`), then `PopState(7)`.
    TeleportChallenge(u64),
    /// `DoTeleport` passes `!mPopulateWithActiveChallenge` to the backend: completed
    /// challenges start during loading, with `isReplayingChallenge` set.
    RetryChallenge(u64),
    TeleportLocation(u64),
}

/// The map's input: the menu picker's up and down, A (keycode 2, `OnMenuSelect`), B (3) and
/// X (5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapInput {
    pub up: bool,
    pub down: bool,
    pub select: bool,
    pub back: bool,
    pub switch_list: bool,
}

/// What `RenderIconOnScreenMap` (`0x8262e350`) draws of a map icon.
#[derive(Clone, Debug, PartialEq)]
pub struct MapIconView {
    pub texture: String,
    /// World X/Z.
    pub position: [f32; 2],
    /// `kFEMapEventIconSize * mScale_map * mScale`.
    pub size: f32,
}

/// The info component (`challenge_info` `SetMode(3)`, a property's `SetMode(4)`): its header and
/// its one option (`GetChallengeComponentNumOptions` is 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoView {
    /// `GetChallengeTitle` (`0x8264d1b0`).
    pub title: MenuText,
    /// `GetChallengeComponentOption(0)` (`0x8264d938`); a property's `ID_CIC_PURCHASE_PROPERTY`,
    /// or `ID_COMMON_TELEPORT` once `IsPropertyOwned`.
    pub option: &'static str,
    pub property: Option<PropertyView>,
}

/// What `challenge_info`'s purchase mode shows besides the header and the option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyView {
    /// `GetChallengeDescription` (`0x8264d2f0`): the challenge's `Description` through
    /// `FormatStringCustom`.
    pub description: MenuText,
    /// `GetChallengeLocationImage` (`0x82623b50`): the image under `fe\source\images\locations\`,
    /// the global data's `FELargeImage` with `_sm`.
    pub image: String,
    /// `GetPropertyPriceOrPaid` (`0x8264db58`): the `PropertyCost` the screen shows as currency,
    /// or `None` once owned, which shows `ID_CIC_PURCHASED`.
    pub price: Option<i32>,
    /// `mCash`, shown while the property is not owned: `FEUnlock.Cash_GetPlayerCashTotal`.
    pub cash: Option<i32>,
}

/// `FEChallengeMap.GetHomData` (`0x8261b128`), in the original list order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomPathView {
    /// The native counter includes both completed AND active challenges.
    pub reached: usize,
    pub total: usize,
    /// First active objective's HAL key and FormatStringCustom arguments.
    pub objective: Option<(String, String)>,
}

fn hom_path_view(world: &World, list_id: i32) -> HomPathView {
    let challenges = challenges_in_list(world, list_id);
    let mut result = HomPathView { reached: 0, total: challenges.len(), objective: None };
    let mut found_active = false;
    for challenge in challenges {
        let active = map_icons::is_active(world, challenge);
        if active || map_icons::is_complete(world, challenge) {
            result.reached += 1;
        }
        if active && !found_active {
            found_active = true;
            result.objective = world.db
                .attribute(class::CHALLENGES, challenge, super::signup_poster::challenge_field::OBJECTIVES)
                .and_then(|a| a.ref_spec(0))
                .map(|(_, key)| super::objectives::title(&world.db, class::CHALLENGE_OBJECTIVE, key));
        }
    }
    result
}

/// What the screen shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeMapView {
    /// The Stuff panel's title: the constructor's `ID_CHALLENGE_MAP_STUFF`, then
    /// `GetStuffTitleText` (screen state 80) after each level change.
    pub title: MenuText,
    /// `GetStuffMenuItems` (screen state 10).
    pub rows: Vec<MenuText>,
    /// The menu picker's value.
    pub selected: usize,
    pub info: Option<InfoView>,
    pub hom: Option<[HomPathView; 4]>,
    pub letterbox: Letterbox,
    /// The icons `RenderMapIcons` (`0x82631138`) draws after the local skater's: the
    /// `mChallengeMapVisible` ones, in `mSimIcons` order.
    pub icons: Vec<MapIconView>,
    /// The popup over the map and its selected option.
    pub popup: Option<(MapPopup, usize)>,
}

/// `Sk8::FE::FrontEndState_ChallengeMap` in its Stuff region.
#[derive(Clone, Debug, Default)]
pub struct ChallengeMap {
    pub menu: StuffMenu,
    mode: Mode,
    /// `mSelectedChallenge` with `mInfoCompSubState`.
    info: Option<Info>,
    hom: Option<[HomPathView; 4]>,
    selected: usize,
    rows: Vec<MenuText>,
    title: Option<MenuText>,
    letterbox: Letterbox,
    /// `FrontEndState_Popup` over the map, with its menu picker's value.
    popup: Option<(MapPopup, usize)>,
    /// The `FE_SOUND_*` globals the map's pickers and states played, in order.
    sounds: Vec<&'static str>,
}

impl ChallengeMap {
    /// `OnEnter` (`0x826539e8`) then `OnLoaded` (`0x826879b8`): the menu is initialized, its rows
    /// listed (screen state 10, in select mode, so the picker's highlight does nothing), then the
    /// Stuff region activated (`SetHighlightRegion` `0x8265ef80`: `FilterIcons()`, active mode,
    /// screen states 60 and 71) and the letterbox set (53 at the root with completed challenges,
    /// else 50, or 54 without rows).
    #[must_use]
    pub fn open(world: &mut World) -> Self {
        let mut map = Self::default();
        map.menu.initialize(world);
        map.mode = Mode::Select;
        map.list_rows(world);
        map.menu.refilter_icons(world);
        map.mode = Mode::Active;
        map.letterbox = if map.menu.depth() == 1 && map.menu.has_completed {
            Letterbox::Stuff {
                populate_active: map.menu.populate_active,
            }
        } else if map.menu.num_sub_menu_items(world, cat::ROOT) > 0 {
            Letterbox::Basic
        } else {
            Letterbox::Back
        };
        map
    }

    /// `OnLeave` (`0x8265ec98`): `ResetMenu`. The map icons keep their filter.
    pub fn close(&mut self) {
        self.menu.reset();
        self.mode = Mode::Active;
    }

    /// `FEMapManager::Update` (`0x82629208`), each front-end update while the state is on top:
    /// `MapIconManager::UpdateMap` (`0x82618728`) pulses the pinch points.
    pub fn tick(world: &mut World) {
        world.map_icons.pulse();
    }

    /// Screen state 10: `GetStuffMenuItems`, the picker's `SetValue(0)` and, with rows, its
    /// `Update`, which reports the highlight (`OnMenuHilight`).
    fn list_rows(&mut self, world: &mut World) {
        self.rows = self.menu.menu_items(world);
        self.selected = 0;
        if !self.rows.is_empty() {
            self.on_menu_hilight(world, 0);
        }
    }

    /// `OnMenuHilight` (`0x82644870`): in active mode, a level of categories shows the highlighted
    /// one's icons, and the item is highlighted.
    fn on_menu_hilight(&mut self, world: &mut World, index: usize) {
        if self.mode != Mode::Active {
            return;
        }
        if StuffMenu::num_sub_categories(self.menu.level()) > 0 {
            self.menu
                .filter_icons(world, i32::try_from(index).unwrap_or(i32::MAX), true);
        }
        self.menu.highlight(world, index);
    }

    /// A level change: screen states 80 (the title), 10 (the rows) and the letterbox, 53 at the
    /// root with completed challenges, else 50.
    fn level_changed(&mut self, world: &mut World) {
        self.title = Some(self.menu.category_hal(world, self.menu.level()));
        self.list_rows(world);
        self.letterbox = if self.menu.depth() == 1 && self.menu.has_completed {
            Letterbox::Stuff {
                populate_active: self.menu.populate_active,
            }
        } else {
            Letterbox::Basic
        };
    }

    /// `ReceiveInput` (`0x8266b420`) in the Stuff region, with what it leaves to the screen: the
    /// menu picker's movement (no wrap) and `OnMenuSelect` (`0x826828f0`); in the info component,
    /// its picker's one option. A popup over the map takes the input instead. The menu pickers
    /// (`menu_picker`) sound `core_nav_up` / `core_nav_down` when their value changes and `core_a_button`
    /// before `OnMenuSelect`; going back a level sounds `core_b_button`.
    pub fn update(&mut self, world: &mut World, input: MapInput) -> MapAction {
        if let Some((popup, selected)) = self.popup {
            return self.update_popup(world, input, popup, selected);
        }
        match self.mode {
            Mode::Select => MapAction::None,
            Mode::HallOfMeat => {
                // ReceiveInput8266b420: only B is admitted in eMode_HOM.
                // DisableHomPanel8265f368 restores the existing root/cursor.
                if input.back {
                    self.mode = Mode::Active;
                    self.hom = None;
                    self.letterbox = if self.menu.depth() == 1 && self.menu.has_completed {
                        Letterbox::Stuff { populate_active: self.menu.populate_active }
                    } else {
                        Letterbox::Basic
                    };
                }
                MapAction::None
            }
            Mode::InfoComponent => {
                if input.back {
                    // DisableInfoComponent (0x8265f208): screen states 21 and 71.
                    self.mode = Mode::Active;
                    self.info = None;
                    self.letterbox = Letterbox::Basic;
                    MapAction::None
                } else if input.select {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    match self.info {
                        Some(Info::Property(key)) => self.select_property(world, key),
                        _ => self.request_teleport(world),
                    }
                } else {
                    MapAction::None
                }
            }
            Mode::Active => {
                if input.switch_list {
                    if self.menu.toggle_active_or_complete(world) {
                        self.list_rows(world);
                        self.letterbox = Letterbox::Stuff {
                            populate_active: self.menu.populate_active,
                        };
                    }
                    return MapAction::None;
                }
                if input.back {
                    let depth = self.menu.depth();
                    if depth <= 1 {
                        return if depth == 1 {
                            MapAction::Close
                        } else {
                            MapAction::None
                        };
                    }
                    if self.menu.previous_level() < depth {
                        self.level_changed(world);
                        self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                    }
                    return MapAction::None;
                }
                let before = self.selected;
                if input.up {
                    self.selected = self.selected.saturating_sub(1);
                    if self.selected != before {
                        self.sounds.push("FE_SOUND_CORE_NAV_UP");
                    }
                }
                if input.down && self.selected + 1 < self.rows.len() {
                    self.selected += 1;
                    self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
                }
                if self.selected != before {
                    self.on_menu_hilight(world, self.selected);
                }
                if input.select && !self.rows.is_empty() {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    let depth = self.menu.depth();
                    match self.menu.select(world, self.selected) {
                        // TriggerInfoComponent (0x8265f070) / _FELocation (0x8265f180): screen
                        // states 20 and 72; _PurchaseProperty (0x8265f0f8): 72 and 22.
                        Selection::Challenge(key) => self.trigger_info(Info::Challenge(key)),
                        Selection::Location(key) => self.trigger_info(Info::Location(key)),
                        Selection::Property(key) => self.trigger_info(Info::Property(key)),
                        Selection::HallOfMeat => {
                            // TriggerHomPanel8265f308: states70,40; APT
                            // SetHomPanelData snapshots lists70,69,68,67.
                            self.hom = Some([70, 69, 68, 67].map(|id| hom_path_view(world, id)));
                            self.mode = Mode::HallOfMeat;
                            self.letterbox = Letterbox::Back;
                        }
                        Selection::None | Selection::Unported => {
                            if self.menu.depth() != depth {
                                self.level_changed(world);
                            }
                        }
                    }
                }
                MapAction::None
            }
        }
    }

    fn trigger_info(&mut self, info: Info) {
        self.info = Some(info);
        self.mode = Mode::InfoComponent;
        self.letterbox = Letterbox::Basic;
    }

    /// `OnMenuSelectChallengeComponentProperty` (`0x82679600`), item 0: an owned property
    /// teleports there through the same Big Black dismissal confirmation as the info option;
    /// otherwise `Property_PurchaseFailed` while the cash is short
    /// of the `PropertyCost`, else `Property_PurchaseConfirm`.
    fn select_property(&mut self, world: &World, key: u64) -> MapAction {
        if property_owned(world, key) {
            return self.request_teleport(world);
        }
        let cost = signpost_progress::challenge_economy_data(world, key, "PropertyCost");
        let popup = if signpost_progress::cash(world) < cost {
            MapPopup::PurchaseFailed
        } else {
            MapPopup::PurchaseConfirm
        };
        // FrontEndState_Popup::OnEnter (0x82658474) sounds core_popup.
        self.popup = Some((popup, 0));
        self.sounds.push("FE_SOUND_CORE_POPUP");
        MapAction::None
    }

    /// The popup's input: not cancellable; its menu picker wraps, sounding a changed value, and A
    /// sounds, closes it and answers the template's callback with the option.
    fn update_popup(&mut self, world: &mut World, input: MapInput, popup: MapPopup, mut selected: usize) -> MapAction {
        let count = popup.options().len();
        if input.up && count > 1 {
            selected = (selected + count - 1) % count;
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
        }
        if input.down && count > 1 {
            selected = (selected + 1) % count;
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
        }
        if !input.select {
            self.popup = Some((popup, selected));
            return MapAction::None;
        }
        self.sounds.push("FE_SOUND_CORE_A_BUTTON");
        self.popup = None;
        match (popup, selected, self.info) {
            (MapPopup::PurchaseConfirm, 1, Some(Info::Property(key))) => self.confirm_purchase(world, key),
            (MapPopup::DismissBigBlack, 1, _) => {
                // DismissAndTeleportCallback826750f0: deactivate BEFORE the
                // map blocks input and starts its existing fade/teleport.
                super::services::deactivate_security(world);
                self.teleport()
            }
            _ => MapAction::None,
        }
    }

    /// `ConfirmPurchaseCallback` (`0x826751f8`) with Yes: complete the purchase, then ask
    /// to dismiss Big Black before teleporting. Declining dismissal does not undo the purchase.
    /// The property list only fills among
    /// the available challenges, so the teleport never retries the challenge.
    fn confirm_purchase(&mut self, world: &mut World, key: u64) -> MapAction {
        let Some(activity) = world.activities.find(key) else {
            return MapAction::None;
        };
        super::dynamic_hull::purchase_property(world, activity);
        self.request_teleport(world)
    }

    /// The `FE_SOUND_*` globals played since the last call, in order.
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    /// OnMenuSelect826828f0 and the property callbacks use popup217 whenever
    /// the living world's actual Big Black entity is present. No is selected
    /// initially; until Yes, retain the current page and destination.
    fn request_teleport(&mut self, world: &World) -> MapAction {
        if world.services.big_black_active {
            self.popup = Some((MapPopup::DismissBigBlack, 0));
            self.sounds.push("FE_SOUND_CORE_POPUP");
            MapAction::None
        } else {
            self.teleport()
        }
    }

    /// The accepted info option's fade and `DoTeleport`. The backend also starts an active
    /// Own the Spot during loading; only the completed list asks for replay mode.
    fn teleport(&self) -> MapAction {
        match self.info {
            Some(Info::Location(key)) => MapAction::TeleportLocation(key),
            Some(Info::Property(key)) => MapAction::TeleportChallenge(key),
            Some(Info::Challenge(key)) => {
                if self.menu.populate_active {
                    MapAction::TeleportChallenge(key)
                } else {
                    MapAction::RetryChallenge(key)
                }
            }
            None => MapAction::None,
        }
    }

    /// `GetChallengeComponentOption(0)` (`0x8264d938`): Teleport, Play for an Own the Spot,
    /// Retry among the completed challenges. During a chase (`ChaseIsOn`) it shows disabled; the
    /// map does not read the port's chase. `challenge_info`'s purchase mode sets its own option:
    /// `ID_COMMON_TELEPORT` for an owned property, else `ID_CIC_PURCHASE_PROPERTY`.
    fn info_option(&self, world: &World) -> &'static str {
        let key = match self.info {
            Some(Info::Challenge(key)) => key,
            Some(Info::Property(key)) if !property_owned(world, key) => return "ID_CIC_PURCHASE_PROPERTY",
            _ => return "ID_COMMON_TELEPORT",
        };
        match world.activities.find(key) {
            None => "ID_COMMON_TELEPORT",
            Some(activity)
                if global_i32(world, activity, field::CHALLENGE_TYPE) == TYPE_OWN_THE_SPOT =>
            {
                "ID_CIC_PLAY_OTS"
            }
            Some(_) if self.menu.populate_active => "ID_COMMON_TELEPORT",
            Some(_) => "ID_COMMON_RETRY",
        }
    }

    /// `GetChallengeTitle` (`0x8264d1b0`): a location's name (layout word 2) as it is, a
    /// challenge's `Title` through `FormatStringCustom`.
    fn info_title(world: &World, info: Info) -> MenuText {
        match info {
            Info::Location(key) => world
                .db
                .layout_text(class::FE_LOCATIONS, key, 2)
                .map_or(MenuText::Empty, |name| MenuText::Copied(name.to_owned())),
            Info::Challenge(key) | Info::Property(key) => match world
                .db
                .attribute(class::CHALLENGES, key, field::TITLE)
                .and_then(|a| a.text(0))
            {
                Some(title) if world.activities.find(key).is_some() => {
                    MenuText::Formatted(title.to_owned())
                }
                _ => MenuText::Empty,
            },
        }
    }

    #[must_use]
    pub fn view(&self, world: &World) -> ChallengeMapView {
        ChallengeMapView {
            title: self
                .title
                .clone()
                .unwrap_or_else(|| MenuText::Copied(ROOT_HAL.to_owned())),
            rows: self.rows.clone(),
            selected: self.selected,
            info: self.info.map(|info| InfoView {
                title: Self::info_title(world, info),
                option: self.info_option(world),
                property: match info {
                    Info::Property(key) => Some(property_view(world, key)),
                    Info::Challenge(_) | Info::Location(_) => None,
                },
            }),
            letterbox: self.letterbox,
            hom: self.hom.clone(),
            icons: map_icons_view(&world.map_icons.icons),
            popup: self.popup,
        }
    }
}

/// `IsPropertyOwned` (`0x8264daa0`): the challenge's `PlayerOwns` save data above 0.
fn property_owned(world: &World, key: u64) -> bool {
    signpost_progress::challenge_save_data(world, key, "PlayerOwns") > 0
}

/// `challenge_info`'s purchase mode texts: the description, the location image, the price or
/// `ID_CIC_PURCHASED`, and the wallet while the property is for sale.
fn property_view(world: &World, key: u64) -> PropertyView {
    let activity = world.activities.find(key);
    let description = world
        .db
        .attribute(class::CHALLENGES, key, field::DESCRIPTION)
        .and_then(|a| a.text(0))
        .filter(|_| activity.is_some())
        .map_or(MenuText::Empty, |text| MenuText::Formatted(text.to_owned()));
    let image = activity.and_then(|a| global_text(world, a, field::FE_LARGE_IMAGE)).unwrap_or_default();
    let owned = property_owned(world, key);
    PropertyView {
        description,
        image: format!("{image}_sm"),
        price: (!owned).then(|| signpost_progress::challenge_economy_data(world, key, "PropertyCost")),
        cash: (!owned).then(|| signpost_progress::cash(world)),
    }
}

/// `RenderIconOnScreenMap`'s icons: the `mChallengeMapVisible` ones, `kFEMapEventIconSize` wide
/// at `mScale_map * mScale`.
fn map_icons_view(icons: &[MapIcon]) -> Vec<MapIconView> {
    icons
        .iter()
        .filter(|icon| icon.challenge_map_visible)
        .map(|icon| MapIconView {
            texture: icon.texture.clone(),
            position: icon.position,
            size: MAP_EVENT_ICON_SIZE * icon.map_scale * icon.scale,
        })
        .collect()
}

/// `TeleportToChallenge`'s location: the challenge's `Location`; none without one, which
/// teleports nowhere.
#[must_use]
pub fn challenge_location(world: &World, challenge: u64) -> Option<String> {
    let activity = world.activities.find(challenge)?;
    global_text(world, activity, field::LOCATION).filter(|name| !name.is_empty())
}

/// The challenge's world location (an `fe_locations` key), which
/// `TeleportLocalPlayerStartChallenge` (`0x828a9678`) teleports into.
#[must_use]
pub fn challenge_world_location(world: &World, challenge: u64) -> Option<u64> {
    let activity = world.activities.find(challenge)?;
    activity_world_location(world, &world.activities.activities[activity])
}

fn activity_world_location(world: &World, activity: &super::activity::Activity) -> Option<u64> {
    let (class, key) = activity.global_data?;
    world.db.attribute(class, key, field::WORLD_LOCATION)?.ref_spec(0).map(|(_, location)| location)
}

/// An `fe_locations` world location's `WorldFile`; none where `IsMainWorldLocation`
/// (`0x828a95b8`) holds for the invalid key or a missing collection.
#[must_use]
pub fn location_world_file(world: &World, location: u64) -> Option<String> {
    if location == u64::MAX || !world.db.has_collection(class::FE_LOCATIONS, location) {
        return None;
    }
    world.db.attribute(class::FE_LOCATIONS, location, field::WORLD_FILE)?.text(0).map(str::to_owned)
}

/// An `fe_locations` collection's start location (layout text 1), which
/// `MapBackEndManager::TeleportToFELocation` (`0x82565350`) teleports to.
#[must_use]
pub fn location_spawn(world: &World, location: u64) -> Option<String> {
    world.db.layout_text(class::FE_LOCATIONS, location, 1).map(str::to_owned).filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sub_category_table_is_the_native_one() {
        assert_eq!(StuffMenu::num_sub_categories(cat::ROOT), 12);
        assert_eq!(SUB_CATEGORIES[0][10], cat::HALL_OF_MEAT);
        assert_eq!(SUB_CATEGORIES[0][11], cat::DOWNLOADED);
        assert_eq!(StuffMenu::num_sub_categories(cat::CAREER), 6);
        assert_eq!(StuffMenu::num_sub_categories(cat::LOCATIONS), 5);
        assert_eq!(StuffMenu::num_sub_categories(cat::DOWNLOADED), 2);
        assert_eq!(StuffMenu::num_sub_categories(cat::TUTORIAL), 0);
        assert_eq!(StuffMenu::num_sub_categories(cat::LEAF), 0);
        assert_eq!(CATEGORY_HAL[5], "ID_STUFFMENU_CATEGORY_TUTORIAL");
    }

    #[test]
    fn previous_level_stays_at_the_root() {
        let mut menu = StuffMenu::default();
        assert_eq!(menu.previous_level(), 1);
        menu.history.push(cat::TUTORIAL);
        assert_eq!(menu.previous_level(), 1);
        assert_eq!(menu.level(), cat::ROOT);
    }

    #[test]
    fn only_map_visible_icons_draw_at_their_map_scale() {
        let enlarged = MapIcon {
            challenge_map_visible: true,
            map_scale: 2.0,
            ..test_icon("mini_tutorial.Texture")
        };
        let view = map_icons_view(&[enlarged, test_icon("mini_smag.Texture")]);
        assert_eq!(view.len(), 1);
        assert_eq!(
            (view[0].texture.as_str(), view[0].size),
            ("mini_tutorial.Texture", 64.0)
        );
    }

    fn test_icon(texture: &str) -> MapIcon {
        MapIcon {
            id: 0,
            challenge_key: 0,
            icon_key: 0,
            texture: texture.to_owned(),
            position: [0.0; 2],
            rotation: 0.0,
            scale: 1.0,
            fade_at_edge: true,
            pinch_point: false,
            pinch_point_non_pulsing: true,
            visible: true,
            scale_dir: map_icons::SCALE_DIR,
            challenge_map_visible: false,
            minimap_visible: true,
            map_scale: 1.0,
        }
    }
}
