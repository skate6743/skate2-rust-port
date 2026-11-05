//! The online challenges the matchmaking screens offer: `ChallengeOnlineManager::
//! GetOnlineChallengesByType` 0x825bf228 and `OnlineProfileManager`'s district lists over it
//! (`GetNumberOfDistricts` 0x82698940, `GetDistrictHALID` 0x826989b0,
//! `GetDistrictFEImageName` 0x82698ad8).

use crate::challenge::attrib_db::AttribDb;
use skate2_net::protocol::challenge;

/// The `challenges` class.
const CHALLENGES: u64 = 0x8876_E0B5_5674_0E36;
/// `challenges`' `GlobalData` and `Title`.
const GLOBAL_DATA: u64 = 0x2449_1BF0_0316_AA45;
const TITLE: u64 = 0xE2A5_0788_B6C2_0C92;
/// `challenge_global_data`'s online flag, `ChallengeType`, `Location` and `FELargeImage`.
const ONLINE: u64 = 0xF9A9_A1DD_ED84_A643;
const CHALLENGE_TYPE: u64 = 0x02B8_28CC_DB47_85E4;
const LOCATION: u64 = 0xF1C6_D5FA_4A23_951E;
const FE_LARGE_IMAGE: u64 = 0x7974_27ED_230C_9980;

/// `Sk8::Net::GetSk8ChallengeType` 0x827840b0: the game's `eChallengeTypes` of a network
/// challenge type.
#[must_use]
pub const fn game_challenge_type(net: u8) -> Option<i32> {
    Some(match net {
        challenge::OWN_THE_SPOT => 9,
        challenge::SLAP => 12,
        challenge::BEST_TRICK => 8,
        challenge::JAM => 15,
        challenge::ONE_UP => 32,
        challenge::SKATE => 4,
        challenge::DEATH_RACE => 19,
        challenge::HALL_OF_MEAT => 31,
        challenge::ONLINE_FREE_SKATE => 22,
        _ => return None,
    })
}

/// `Sk8::Net::GetNetChallengeType` 0x827841d8.
#[must_use]
pub fn net_challenge_type(game: i32) -> Option<u8> {
    (0..challenge::COUNT).find(|&net| game_challenge_type(net) == Some(game))
}

/// `OnlineProfileManager::ChallengeInformation::m_iaChallengeNameHALID` (0x82fe06b0) by
/// `eChallengeTypes`.
#[must_use]
pub fn challenge_title(net: u8) -> &'static str {
    match game_challenge_type(net) {
        Some(9) => "ID_MISSION_TEMPLATE_OWN_THE_SPOT_TITLE",
        Some(12) => "ID_MISSION_TEMPLATE_LAP_TITLE",
        Some(8) => "ID_MISSION_TEMPLATE_BEST_TRICK_TITLE",
        Some(15) => "ID_MISSION_TEMPLATE_JAM_TITLE",
        Some(32) => "ID_MISSION_TEMPLATE_1UP_TITLE",
        Some(4) => "ID_MISSION_TEMPLATE_SKATE_TITLE",
        Some(19) => "ID_MISSION_TEMPLATE_DEATH_RACE_TITLE",
        Some(31) => "ID_MISSION_TEMPLATE_HALLOFMEAT_TITLE",
        Some(22) => "ID_MISSION_TEMPLATE_ONLINE_FREESKATE_TITLE",
        _ => "ID_COMMON_UNKNOWN",
    }
}

/// `m_iaChallengeDescriptionHALID` (0x82fe0780) by `eChallengeTypes`.
#[must_use]
pub fn challenge_description(net: u8) -> &'static str {
    match game_challenge_type(net) {
        Some(9) => "ID_MISSION_TEMPLATE_OWN_THE_SPOT_DESCRIPTION",
        Some(12) => "ID_MISSION_TEMPLATE_LAP_DESCRIPTION",
        Some(8) => "ID_MISSION_TEMPLATE_BEST_TRICK_DESCRIPTION",
        Some(15) => "ID_MISSION_TEMPLATE_JAM_DESCRIPTION",
        Some(32) => "ID_MISSION_TEMPLATE_1UP_DESCRIPTION",
        Some(4) => "ID_MISSION_TEMPLATE_SKATE_DESCRIPTION",
        Some(19) => "ID_MISSION_TEMPLATE_DEATH_RACE_DESCRIPTION",
        Some(31) => "ID_MISSION_TEMPLATE_HALLOFMEAT_DESCRIPTION",
        Some(22) => "ID_MISSION_TEMPLATE_ONLINE_FREESKATE_DESCRIPTION",
        _ => "ID_COMMON_UNKNOWN",
    }
}

/// One online challenge: the challenge collection, its `Title`, `FELargeImage` and location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnlineChallenge {
    pub key: u64,
    /// `GetDistrictHALID`: the title, "ID_COMMON_UNKNOWN" without one.
    pub title: String,
    /// `GetDistrictFEImageName`: the image under `fe\source\images\locations\`, "default"
    /// without one.
    pub image: String,
    /// The global data's `Location`, a `cLocationManager` location name.
    pub location: String,
}

/// `GetOnlineChallengesByType(type, keys, sortByName = true)`: the leaf `challenges` whose global
/// data is flagged online, of the type and at a valid location, sorted by their localized titles
/// (`SortChallengeByName` 0x825b1690).
#[must_use]
pub fn online_challenges(db: &AttribDb, net_type: u8, valid_location: &dyn Fn(&str) -> bool, localize: &dyn Fn(&str) -> String) -> Vec<OnlineChallenge> {
    let Some(game_type) = game_challenge_type(net_type) else { return Vec::new() };
    let mut challenges: Vec<(String, OnlineChallenge)> = db
        .leaf_collections(CHALLENGES)
        .iter()
        .filter_map(|&key| {
            let (class, global) = db.attribute(CHALLENGES, key, GLOBAL_DATA)?.ref_spec(0)?;
            if db.attribute(class, global, ONLINE).and_then(|flag| flag.bool(0)) != Some(true) {
                return None;
            }
            if db.attribute(class, global, CHALLENGE_TYPE).and_then(|value| value.i32(0)) != Some(game_type) {
                return None;
            }
            let location = db.attribute(class, global, LOCATION).and_then(|value| value.text(0))?.to_owned();
            if !valid_location(&location) {
                return None;
            }
            let title = db.attribute(CHALLENGES, key, TITLE).and_then(|value| value.text(0)).filter(|title| !title.is_empty());
            let image = db.attribute(class, global, FE_LARGE_IMAGE).and_then(|value| value.text(0)).filter(|image| !image.is_empty());
            let sort = title.map(localize).unwrap_or_default().to_ascii_lowercase();
            Some((
                sort,
                OnlineChallenge {
                    key,
                    title: title.unwrap_or("ID_COMMON_UNKNOWN").to_owned(),
                    image: image.unwrap_or("default").to_owned(),
                    location,
                },
            ))
        })
        .collect();
    challenges.sort_by(|a, b| a.0.cmp(&b.0));
    challenges.into_iter().map(|(_, challenge)| challenge).collect()
}

/// `OnlineProfileManager::IsChallengeTypeValid` 0x826988b0: a type with an online challenge.
#[must_use]
pub fn challenge_type_valid(db: &AttribDb, net_type: u8, valid_location: &dyn Fn(&str) -> bool) -> bool {
    !online_challenges(db, net_type, valid_location, &|title| title.to_owned()).is_empty()
}

/// The challenge types the unranked custom match lists (`unranked_custom_match.swf`'s
/// `InitializeScreen`): every type of `GetNumberOfOnlineChallenges` (9) but the last, online
/// free skate, that is valid. The original's list also leaves out 1-Up (`m_1upIndex` 4), whose
/// online challenges the game ships; the PC port lists it (user request).
#[must_use]
pub fn unranked_match_types(db: &AttribDb, valid_location: &dyn Fn(&str) -> bool) -> Vec<u8> {
    (0..challenge::ONLINE_FREE_SKATE).filter(|&net| challenge_type_valid(db, net, valid_location)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With `SKATE2_GAME_ROOT`, the game's online 1-Ups are online challenges of their type, which
    /// the custom match lists.
    #[test]
    fn the_game_ships_online_1ups() {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let Some(root) = std::env::var_os("SKATE2_GAME_ROOT") else { return };
        let archive = SKATE2_ARCHIVES.iter().find(|archive| archive.relative_path == "data\\db\\db.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        let db = AttribDb::load_career(&mounts).unwrap();
        let listed = online_challenges(&db, challenge::ONE_UP, &|_| true, &|title: &str| title.to_owned());
        eprintln!("1-Ups: {:?}", listed.iter().map(|one_up| (&one_up.title, &one_up.location)).collect::<Vec<_>>());
        let keys: Vec<u64> = listed.iter().map(|one_up| one_up.key).collect();
        for name in ["ol_1up_01_svm_mountainup", "ol_1up_02_dt_matrix", "ol_1up_03_ot_cathedral", "ol_1up_04_wf_gvr", "ol_1up_05_ur_lombardier"] {
            assert!(keys.contains(&crate::challenge::keys::hash64(name)), "{name} is listed");
        }
        assert!(unranked_match_types(&db, &|_| true).contains(&challenge::ONE_UP));
    }
}
