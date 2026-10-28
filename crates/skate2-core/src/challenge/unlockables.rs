//! `Sk8::BE::UnlockablesBackEnd` (`BackEndManager` module 5): the profile's
//! unlocked items, each a `kUnlockCategories` category and a `GetStringHash`
//! id, its memcard savable `UNLOCKABLES` (`UnlockablesBackEnd::Data` v1), and
//! the Lua natives that unlock items for the career.

use super::{
    attrib_db::AttribDb,
    binding::get_string,
    keys::hash64,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    signpost_progress::{Reader, Writer},
    system::World,
};

/// `kUnlockCategories` (`0x82fd95d0`).
pub mod category {
    pub const CLOTHING: u32 = 0;
    pub const BOARD_ACCESSORIES: u32 = 1;
    pub const LOCATIONS: u32 = 2;
    pub const CHALLENGES: u32 = 3;
    pub const CHARACTERS: u32 = 4;
    pub const FMV: u32 = 5;
    pub const COVERAGE: u32 = 6;
    pub const SPONSORSHIP: u32 = 7;
    pub const OWN_THE_SPOT: u32 = 8;
    pub const OWN_THE_SPOT_OWNED: u32 = 9;
    pub const SKATE_SHOPS: u32 = 10;
    pub const REPLAY: u32 = 11;
    pub const EGGS: u32 = 12;
    pub const SECURITY_SETTING: u32 = 13;
    pub const GESTURES: u32 = 14;
    pub const CLOTHING_UNLOCK: u32 = 15;
    pub const CLOTHING_SPONSOR: u32 = 16;
    /// The names, in category order (`LoadUnlocksFromFiles` `0x82573100` compares an unlock
    /// list's category with each).
    pub const NAMES: [&str; 17] = [
        "CLOTHING",
        "BOARDACCESSORIES",
        "LOCATIONS",
        "CHALLENGES",
        "CHARACTERS",
        "FMV",
        "COVERAGE",
        "SPONSORSHIP",
        "OWN_THE_SPOT",
        "OWN_THE_SPOT_OWNED",
        "SKATE_SHOPS",
        "REPLAY",
        "EGGS",
        "SECURITY_SETTING",
        "GESTURES",
        "CLOTHING_UNLOCK",
        "CLOTHING_SPONSOR",
    ];
}

/// The savable (`IMemcardSavable` of `UnlockablesBackEnd`), registered first by
/// `BackEndManager::Init`.
pub const SAVABLE: &str = "UNLOCKABLES";
/// `kUnlockablesBackEndHeaderName`.
pub const DATA: &str = "UnlockablesBackEnd::Data";
pub const DATA_VERSION: u32 = 1;
/// `mUnlockedUnlockables` and `mDLCUnlockedUnlockables` hold 500 items each.
pub const CAPACITY: usize = 500;

/// `fe_location_group`, the group `AddDefaultUnlocks` reads, and its
/// `Location` (`RefSpec[]` into `fe_locations`).
const FE_LOCATION_GROUP: u64 = 0xd897_e7db_651e_58dc;
const DEFAULT_LOCATION_GROUP: u64 = 0x2514_27ba_fae9_9434;
const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
/// `characters_marquee`.
const CHARACTERS_MARQUEE: u64 = 0x91bc_a669_3efc_9aa7;

/// `UnlockablesBackEndCore::UnlockableItem` (12 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnlockableItem {
    pub category: u32,
    pub id: u32,
    pub is_new: bool,
}

/// The constructor's (`0x82a739c8`) slot value.
const EMPTY_SLOT: UnlockableItem = UnlockableItem { category: 0, id: 0, is_new: true };

/// `UnlockablesBackEndCore` and `UnlockablesBackEnd::mUnlockAll`. The arrays
/// keep their whole native contents: `Init` only resets the count, and
/// `SaveData` writes every slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unlockables {
    slots: Box<[UnlockableItem; CAPACITY]>,
    count: usize,
    dlc_slots: Box<[UnlockableItem; CAPACITY]>,
    dlc_count: usize,
    /// `mUnlockAll`.
    pub unlock_all: bool,
}

/// `UnlockablesBackEndCore::GetStringHash` (`0x82a73430`): from -1,
/// `h = tolower((signed char)c) + 33 * h`.
#[must_use]
pub fn string_hash(name: &str) -> u32 {
    name.bytes().fold(u32::MAX, |h, b| {
        let c = i32::from(b.cast_signed());
        let lower = if (i32::from(b'A')..=i32::from(b'Z')).contains(&c) { c + 32 } else { c };
        lower.cast_unsigned().wrapping_add(h.wrapping_mul(33))
    })
}

impl Unlockables {
    /// The constructor (`0x82a739c8`), then `UnlockablesBackEnd::Init`
    /// (`0x8255cca0`) → `UnlockablesBackEndCore::Init` (`0x82a73c80`): no
    /// items, then `AddDefaultUnlocks`. The DLC list holds the downloadable
    /// content's unlock lists (`LoadUnlocksFromFiles`, run when
    /// `DLC::Manager::EnumerateContent` mounts content), which the database
    /// read with its vaults ([`AttribDb::dlc_unlocks`]).
    #[must_use]
    pub fn new(db: &AttribDb) -> Self {
        let mut unlockables = Self {
            slots: Box::new([EMPTY_SLOT; CAPACITY]),
            count: 0,
            dlc_slots: Box::new([EMPTY_SLOT; CAPACITY]),
            dlc_count: 0,
            unlock_all: false,
        };
        unlockables.add_default_unlocks(db);
        for &(category, id) in db.dlc_unlocks() {
            unlockables.unlock(category, id, true);
        }
        unlockables
    }

    /// `UnlockablesBackEnd::AddDefaultUnlocks` (`0x82566030`).
    fn add_default_unlocks(&mut self, db: &AttribDb) {
        for shop in ["dist_financial_skateshop", "dist_urbanres_skateshop"] {
            self.unlock(category::SKATE_SHOPS, string_hash(shop), false);
        }
        for character in ["sammy", "mike", "slappy", "shingo", "cuz", "seb", "freeway"] {
            self.unlock(category::CHARACTERS, string_hash(character), false);
        }
        if let Some(locations) = db.attribute(FE_LOCATION_GROUP, DEFAULT_LOCATION_GROUP, LOCATION) {
            for i in 0..locations.len() {
                // An unresolved RefSpec builds an Instance without a collection,
                // which the loop skips.
                let Some((class, key)) = locations.ref_spec(i).filter(|&(c, k)| db.has_collection(c, k)) else { continue };
                if let Some(name) = db.layout_text(class, key, 0) {
                    self.unlock(category::LOCATIONS, string_hash(name), false);
                }
            }
        }
        self.unlock(category::SECURITY_SETTING, string_hash("security_setting"), false);
        for gesture in [
            "gesture_sky",
            "gesture_what",
            "gesture_snap",
            "gesture_thumbdown",
            "gesture_pumphi",
            "gesture_fonz",
            "gesture_bruce",
            "gesture_shrug",
            "gesture_pumplo",
            "gesture_devil",
            "gesture_fu",
            "gesture_throwarms",
            "gesture_airplane",
            "gesture_clothesline",
            "gesture_fliptable",
        ] {
            self.unlock(category::GESTURES, string_hash(gesture), false);
        }
        self.reset_all_new_unlocks();
    }

    /// The unlocked items in unlock order.
    #[must_use]
    pub fn items(&self) -> &[UnlockableItem] {
        &self.slots[..self.count]
    }

    /// `FindUnlockable(category, id, fromDLC)` (`0x82a73370`).
    fn find(&self, category: u32, id: u32, from_dlc: bool) -> Option<usize> {
        let (slots, count) = if from_dlc { (&self.dlc_slots, self.dlc_count) } else { (&self.slots, self.count) };
        slots[..count].iter().position(|u| u.category == category && u.id == id)
    }

    /// `UnlockablesBackEndCore::Unlock(category, id, fromDLC)` (`0x82a73a78`):
    /// true when the item is added to that list (new unless from DLC). The
    /// virtual `UpdateUnlockStats` is the empty function in `UnlockablesBackEnd`.
    /// A full list asserts in the native and writes past its array; the port
    /// stops adding there.
    pub fn unlock(&mut self, category: u32, id: u32, from_dlc: bool) -> bool {
        if self.find(category, id, from_dlc).is_some() {
            return false;
        }
        let (slots, count) = if from_dlc { (&mut self.dlc_slots, &mut self.dlc_count) } else { (&mut self.slots, &mut self.count) };
        if *count >= CAPACITY {
            return false;
        }
        slots[*count] = UnlockableItem { category, id, is_new: !from_dlc };
        *count += 1;
        true
    }

    /// `UnlockablesBackEnd::IsDebugCheatEnabled` (`0x8255ccd0`): `kBEUnlockAll`
    /// (default off) or `mUnlockAll`.
    #[must_use]
    pub const fn is_debug_cheat_enabled(&self) -> bool {
        self.unlock_all
    }

    /// `IsUnlocked(category, id, includeDLC)` (`0x82a73c98`).
    #[must_use]
    pub fn is_unlocked(&self, category: u32, id: u32, include_dlc: bool) -> bool {
        // Intentional PC quality-of-life override, requested 2026-10-13:
        // always expose Film Pack editing, saved footage and its Extras tutorial.
        // Query-time policy survives profile loads without writing DLC save slots.
        if category == category::REPLAY
            && (id == string_hash("filmpack") || id == string_hash("filmpackmovie"))
        {
            return true;
        }
        if self.is_debug_cheat_enabled() {
            return true;
        }
        self.find(category, id, false).is_some() || (include_dlc && self.find(category, id, true).is_some())
    }

    /// `IsNewUnlock(category, id)` (`0x82a73d28`): the main list's item, else
    /// the DLC one.
    #[must_use]
    pub fn is_new_unlock(&self, category: u32, id: u32) -> bool {
        if self.is_debug_cheat_enabled() {
            return false;
        }
        match (self.find(category, id, false), self.find(category, id, true)) {
            (Some(i), _) => self.slots[i].is_new,
            (None, Some(i)) => self.dlc_slots[i].is_new,
            (None, None) => false,
        }
    }

    /// `ResetAllNewUnlocks` (`0x82a732f8`): the main list only.
    pub fn reset_all_new_unlocks(&mut self) {
        for item in &mut self.slots[..self.count] {
            item.is_new = false;
        }
    }

    /// `ResetAllNewUnlocksFromCategory` (`0x82a73330`): the main list's items of `category`.
    pub fn reset_all_new_unlocks_from_category(&mut self, category: u32) {
        for item in &mut self.slots[..self.count] {
            if item.category == category {
                item.is_new = false;
            }
        }
    }

    /// `UnlockablesBackEnd::SaveData` (`0x8255ce38`): the count, then all 500
    /// slots as (category, id, new).
    #[must_use]
    pub fn save(&self) -> [(&'static str, u32, Vec<Vec<u8>>); 1] {
        let mut w = Writer::default();
        w.word(u32::try_from(self.count).unwrap_or(u32::MAX));
        for item in self.slots.iter() {
            w.word(item.category);
            w.word(item.id);
            w.boolean(item.is_new);
        }
        [(DATA, DATA_VERSION, w.into_tuples())]
    }

    /// `UnlockablesBackEnd::LoadData` (`0x8255cd08`): each of the saved count's
    /// items unlocked again with its saved `new` flag.
    pub fn load(&mut self, saved: Option<(u32, &[Vec<u8>])>) {
        let Some((DATA_VERSION, tuples)) = saved else { return };
        let mut r = Reader::new(tuples);
        let count = r.int();
        for _ in 0..count {
            let category = r.word();
            let id = r.word();
            let is_new = r.boolean();
            self.unlock(category, id, false);
            if let Some(i) = self.find(category, id, false) {
                self.slots[i].is_new = is_new;
            }
        }
    }
}

// ---- Natives ---------------------------------------------------------------

type R = LuaResult<Vec<Value>>;

fn name_arg(call: &NativeCall) -> String {
    get_string(call, 0).unwrap_or_default()
}

/// `Unlock.UnlockCharacterForFreeskate(name)` (`0x8289b220`): the
/// `characters_marquee` collection `StringToKey(name)`'s id string (layout
/// word 0) unlocked as a character, and the key added to the completion
/// results. A name without a collection reads a null string in the native.
fn unlock_character_for_freeskate(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let key = hash64(&name_arg(call));
    if let Some(id) = world.db.layout_text(CHARACTERS_MARQUEE, key, 0).map(string_hash) {
        world.unlockables.unlock(category::CHARACTERS, id, false);
    }
    world.signpost.completion.unlocked_characters.push(key);
    Ok(Vec::new())
}

/// `Unlock.UnlockLocationForFreeskate(name)` (`0x8289b358`).
fn unlock_location_for_freeskate(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = name_arg(call);
    world.unlockables.unlock(category::LOCATIONS, string_hash(&name), false);
    world.signpost.completion.unlocked_locations.push(hash64(&name));
    Ok(Vec::new())
}

/// `Progression.UnlockUnlockableByName(name)` (`0x825d5250`): a newly unlocked
/// `CLOTHING_UNLOCK` item joins the completion results.
fn progression_unlock_unlockable(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = name_arg(call);
    if world.unlockables.unlock(category::CLOTHING_UNLOCK, string_hash(&name), false) {
        world.signpost.completion.unlocked_cac_items.push(hash64(&name));
    }
    Ok(Vec::new())
}

/// `Progression.UnlockSponsorByName(name)` (`0x825d5108`).
fn progression_unlock_sponsor(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = name_arg(call);
    if world.unlockables.unlock(category::CLOTHING_SPONSOR, string_hash(&name), false) {
        world.signpost.completion.unlocked_sponsor_cac_items.push(hash64(&name));
    }
    Ok(Vec::new())
}

/// `Progression.UnlockGestureByName(name)` (`0x825c4270`).
fn progression_unlock_gesture(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.unlockables.unlock(category::GESTURES, string_hash(&name_arg(call)), false);
    Ok(Vec::new())
}

/// `Progression.IsUnlockUnlockedByName(name)` (`0x825c41e0`): a
/// `CLOTHING_UNLOCK` item, DLC included.
fn progression_is_unlock_unlocked(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(world.unlockables.is_unlocked(category::CLOTHING_UNLOCK, string_hash(&name_arg(call)), true))])
}

/// `ChallengeProgression.UnlockUnlockableByName(name)` (`0x8289bba0`). The
/// native adds the key to a copy of the completion results, which it then
/// destroys, so only the unlock remains.
fn challenge_unlock_unlockable(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.unlockables.unlock(category::CLOTHING_UNLOCK, string_hash(&name_arg(call)), false);
    Ok(Vec::new())
}

/// `ChallengeProgression.UnlockSponsorByName(name)` (`0x8289bcd8`), with the
/// same copied completion results.
fn challenge_unlock_sponsor(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.unlockables.unlock(category::CLOTHING_SPONSOR, string_hash(&name_arg(call)), false);
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Unlock", "UnlockCharacterForFreeskate", unlock_character_for_freeskate),
        ("Unlock", "UnlockLocationForFreeskate", unlock_location_for_freeskate),
        ("Progression", "UnlockUnlockableByName", progression_unlock_unlockable),
        ("Progression", "UnlockSponsorByName", progression_unlock_sponsor),
        ("Progression", "UnlockGestureByName", progression_unlock_gesture),
        ("Progression", "IsUnlockUnlockedByName", progression_is_unlock_unlocked),
        ("ChallengeProgression", "UnlockUnlockableByName", challenge_unlock_unlockable),
        ("ChallengeProgression", "UnlockSponsorByName", challenge_unlock_sponsor),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_hash_lowers_and_starts_from_minus_one() {
        assert_eq!(string_hash(""), u32::MAX);
        let expected = u32::from(b'a').wrapping_add(u32::MAX.wrapping_mul(33));
        assert_eq!(string_hash("a"), expected);
        assert_eq!(string_hash("A"), expected);
        assert_eq!(string_hash("Reda"), string_hash("reda"));
    }
}
