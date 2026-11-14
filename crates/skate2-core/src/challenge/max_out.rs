//! PC port addition: Extras' Max Out Save (`session/selected_session_extras.rs`). A profile's
//! career is put where the whole career leaves it: every progression challenge completed, the
//! world's gates, properties, pools and skate stoppers as the career ends them, the cash at
//! [`CASH`], and every unlock the career's scripts and the clothing catalog hold. See
//! `docs/evidence/max-out-save-2026-11-14.md`.
use super::{
    dynamic_hull, signpost, signpost_progress,
    system::World,
    unlockables::{category, string_hash},
};
use std::sync::Arc;

/// The cash a maxed-out save holds.
pub const CASH: i32 = 9_000_000;

/// The point pools that count what the career has done, each at its maximum, which is the count of
/// what it counts: the properties bought, the pools drained and found, the spots owned, the pros
/// beaten, the sponsorships earned, and the Anywhere and Circulate films.
const COUNTERS: [&str; 8] = [
    "Properties Purchased",
    "Pools Drained",
    "Pools Found",
    "Spots Owned",
    "Pros Completed",
    "Sponsorships Earned",
    "AnywhereFilms",
    "Circulate Pool",
];

/// The clothing a catalog's parts wait for: their unlocks (category 15) and their sponsor unlocks
/// (category 16), as the shop's `LockedFilter` reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clothing {
    pub unlocks: Vec<String>,
    pub sponsors: Vec<String>,
}

/// Maxes out the career `world` holds (the profile's challenge system, its savables loaded).
pub fn max_out(world: &mut World, clothing: &Clothing) {
    signpost::complete_every_challenge(world);
    signpost_progress::max_out_challenge_save_data(world);
    signpost_progress::max_out_point_pools(world, CASH, &COUNTERS);
    signpost_progress::max_out_rankings(world);
    unlock_everything(world, clothing);
    dynamic_hull::max_out(world);
}

/// Every unlock the career's scripts make (the free-skate characters and locations, the gestures,
/// the clothing and the sponsors' clothing), Big Black (the Extras code's character), every
/// character and location at once (`all_characters_unlock`, `all_locations_unlock`), and the
/// clothing `clothing` names; none of them left new.
fn unlock_everything(world: &mut World, clothing: &Clothing) {
    let db = Arc::clone(&world.db);
    let scripted = |call: &str| signpost::scripted_names(&db, call);
    let characters = scripted("Unlock.UnlockCharacterForFreeskate");
    let locations = scripted("Unlock.UnlockLocationForFreeskate");
    let gestures = scripted("Progression.UnlockGestureByName");
    let unlocks = scripted("Progression.UnlockUnlockableByName");
    let sponsors = scripted("Progression.UnlockSponsorByName");
    let unlockables = &mut world.unlockables;
    for name in &characters {
        unlockables.unlock_freeskate_character(&db, name);
    }
    for name in ["big_black", "all_characters_unlock"] {
        unlockables.unlock(category::CHARACTERS, string_hash(name), false);
    }
    for name in locations.iter().map(String::as_str).chain(["all_locations_unlock"]) {
        unlockables.unlock(category::LOCATIONS, string_hash(name), false);
    }
    for name in &gestures {
        unlockables.unlock(category::GESTURES, string_hash(name), false);
    }
    for name in clothing.unlocks.iter().chain(&unlocks) {
        unlockables.unlock(category::CLOTHING_UNLOCK, string_hash(name), false);
    }
    for name in clothing.sponsors.iter().chain(&sponsors) {
        unlockables.unlock(category::CLOTHING_SPONSOR, string_hash(name), false);
    }
    unlockables.reset_all_new_unlocks();
}
