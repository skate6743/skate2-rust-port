//! The demo-sponsor reads of `Sk8::Challenge::cSignPostSponsorMan`: the pro the player beat
//! (message setting `Pro Beaten`), that pro's board sponsor and the competitors of the pro's
//! demo. `Init` (`0x825d8328`) maps `GetHashValue32` of each `throwdown_competitors`' name
//! (layout +0x40) and each `sponsor`'s name (layout +0) to its collection.

use super::{attrib_db::Location, keys::hash_value32, signpost_progress::classes::SPONSORS, system::World};

/// `throwdown_competitors`.
const THROWDOWN_COMPETITORS: u64 = 0x9c86_9217_c5ae_e9b4;
/// `throwdown_competitors`' name (`char*`, layout +0x40).
const COMPETITOR_NAME: u32 = 0x40;
/// `throwdown_competitors`' board sponsor (`ClassRefSpec_sponsor`, layout +0x30).
const COMPETITOR_SPONSOR: u32 = 0x30;
/// `throwdown_competitors`' demo competitors (`tCompetitorInfo[]`).
const DEMO_COMPETITORS: u64 = 0xc61e_b299_002c_9fa6;
/// `kMissingSponsorName`.
pub const MISSING_SPONSOR_NAME: &str = "Online";
/// `sponsor`'s world (`char*`), which `ProcessSponsorWorld` and `ProcessDemoWorld` read.
const WORLD: u64 = 0xc8f4_4623_2859_d86b;
/// `sponsor`'s HAL id (`char*`, layout +8).
const HAL_ID: u32 = 2;

/// `GetProBeatenKey` (`0x825d1118`): the `throwdown_competitors` collection whose name hashes to
/// the `Pro Beaten` message setting (missing: the hash of `kMissingSponsorName`); `None` (-1)
/// when no competitor has that name.
#[must_use]
pub fn pro_beaten_key(world: &World) -> Option<u64> {
    let setting = world.signpost.message_setting_value("Pro Beaten").unwrap_or_else(|| hash_value32(MISSING_SPONSOR_NAME));
    let db = &world.db;
    db.collections(THROWDOWN_COMPETITORS).iter().copied().find(|&competitor| {
        db.layout_text(THROWDOWN_COMPETITORS, competitor, COMPETITOR_NAME / 4).is_some_and(|name| hash_value32(name) == setting)
    })
}

/// The beaten pro's board sponsor (`sponsor` collection), none without a beaten pro.
#[must_use]
pub fn board_sponsor_key(world: &World) -> Option<u64> {
    let pro = pro_beaten_key(world)?;
    let db = &world.db;
    let index = COMPETITOR_SPONSOR / 4;
    Some((u64::from(db.layout_word(THROWDOWN_COMPETITORS, pro, index)) << 32) | u64::from(db.layout_word(THROWDOWN_COMPETITORS, pro, index + 1)))
}

/// `GetBoardSponsorKey` (`0x825d1198`): the `sponsor` collection whose name hashes to the
/// `Board Sponsor` message setting (missing: the hash of `kMissingSponsorName`); none (-1) when no
/// sponsor has that name.
#[must_use]
pub fn signed_board_sponsor_key(world: &World) -> Option<u64> {
    let wanted = world.signpost.message_setting_value("Board Sponsor").unwrap_or_else(|| hash_value32(MISSING_SPONSOR_NAME));
    let db = &world.db;
    db.collections(SPONSORS).iter().copied().find(|&key| db.layout_text(SPONSORS, key, 0).is_some_and(|name| hash_value32(name) == wanted))
}

/// `ProcessSponsorWorld` (`0x825634a0`) and `ProcessDemoWorld` (`0x82563610`): the sponsor's
/// world, `kMissingSponsorName` without the sponsor, its world or the world's text.
/// `kForceBoardSponsorWorldToAlien` (off by default) is not in the cvar table.
#[must_use]
pub fn sponsor_world(world: &World, sponsor: Option<u64>) -> String {
    sponsor.and_then(|key| world.db.attribute(SPONSORS, key, WORLD)).and_then(|attribute| attribute.text(0)).unwrap_or(MISSING_SPONSOR_NAME).to_owned()
}

/// The localizer parameter `FEBackEnd::Init` registers for `ProcessSponsorWorld`.
const SPONSOR_WORLD_PARAMETER: &str = "{0:SponsorWorld}";

/// The board sponsor's world the career last published (`cSignPostSponsorMan` lives in the
/// simulation singleton, which `ProcessSponsorWorld` reads whatever asks); none without a career.
static PUBLISHED_SPONSOR_WORLD: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// The career's board sponsor world for [`world_file_name`]; none as the career ends.
pub(crate) fn publish_sponsor_world(world: Option<&str>) {
    let changed = PUBLISHED_SPONSOR_WORLD.read().map_or(true, |held| held.as_deref() != world);
    if changed && let Ok(mut held) = PUBLISHED_SPONSOR_WORLD.write() {
        *held = world.map(str::to_owned);
    }
}

/// `CommonBackEnd::GetWorldFilename` (`0x82562480`): a location's `WorldFile` through the
/// localizer, whose `{0:SponsorWorld}` is `ProcessSponsorWorld`'s board sponsor world
/// (`kMissingSponsorName` without one), then the Girl sponsor's training facility, which the
/// disc lacks, as the Online one.
#[must_use]
pub fn world_file_name(raw: &str) -> std::borrow::Cow<'_, str> {
    if !raw.contains(SPONSOR_WORLD_PARAMETER) {
        return std::borrow::Cow::Borrowed(raw);
    }
    let sponsor = PUBLISHED_SPONSOR_WORLD
        .read()
        .ok()
        .and_then(|held| held.clone())
        .filter(|world| !world.is_empty())
        .unwrap_or_else(|| MISSING_SPONSOR_NAME.to_owned());
    let named = raw.replace(SPONSOR_WORLD_PARAMETER, &sponsor);
    std::borrow::Cow::Owned(if named == "DIST_TrainingFacility_Girl" { "DIST_TrainingFacility_Online".to_owned() } else { named })
}

/// `GetBoardSponsorHALID` (`0x825d1358`): the beaten pro's board sponsor's HAL id,
/// `kMissingSponsorName` without a beaten pro.
#[must_use]
pub fn demo_sponsor_hal_id(world: &World) -> String {
    match board_sponsor_key(world) {
        Some(key) => world.db.layout_text(SPONSORS, key, HAL_ID).unwrap_or_default(),
        None => MISSING_SPONSOR_NAME,
    }
    .to_owned()
}

/// `GetBoardSponsorName` (`0x825d12c8`): the beaten pro's board sponsor's name (`sponsor` layout
/// +0), `kMissingSponsorName` without a beaten pro.
#[must_use]
pub fn board_sponsor_name(world: &World) -> String {
    let Some(sponsor) = board_sponsor_key(world) else { return MISSING_SPONSOR_NAME.to_owned() };
    world.db.layout_text(SPONSORS, sponsor, 0).unwrap_or_default().to_owned()
}

/// `GetSponsorName` 0x825d13e8: the selected sponsor's HAL ID (layout +8),
/// through its message setting and SponsorNameMap; missing is "Online".
pub(crate) fn selected_sponsor_hal(world: &World, setting: &str) -> String {
    let wanted = world.signpost.message_setting_value(setting).unwrap_or_else(|| hash_value32(MISSING_SPONSOR_NAME));
    world.db.collections(SPONSORS).iter().copied().find(|&key| {
        world.db.layout_text(SPONSORS, key, 0).is_some_and(|name| hash_value32(name) == wanted)
    }).and_then(|key| world.db.layout_text(SPONSORS, key, 2)).unwrap_or(MISSING_SPONSOR_NAME).to_owned()
}

/// ProcessBoard/Truck/Wheel/ShoeSponsor 0x82562e80..0x82563320 use the
/// selected sponsor's HAL, replacing a missing sponsor with ID_COMMON_UNKNOWN.
pub(crate) fn frontend_sponsor_labels(world: &World) -> [String; 4] {
    ["Board Sponsor", "Truck Sponsor", "Wheel Sponsor", "Shoe Sponsor"].map(|setting| {
        let label = selected_sponsor_hal(world, setting);
        if label == MISSING_SPONSOR_NAME { "ID_COMMON_UNKNOWN".to_owned() } else { label }
    })
}

/// `GetDemoCompetitorCount` (`0x825d15c0`): the beaten pro's demo competitors, 0 without one.
#[must_use]
pub fn demo_competitor_count(world: &World) -> i32 {
    let Some(pro) = pro_beaten_key(world) else { return 0 };
    let count = world.db.attribute(THROWDOWN_COMPETITORS, pro, DEMO_COMPETITORS).map_or(0, |a| a.len());
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// `GetDemoCompetitorInfo(index)` (`0x825d1658`): the beaten pro's demo competitor's
/// `tCompetitorInfo` (`ProfileName`, `intRootCategory`); `None` for `kMissingCompetitorInfo` or
/// the zeroed default area.
#[must_use]
pub fn demo_competitor_info(world: &World, index: usize) -> Option<Location> {
    let pro = pro_beaten_key(world)?;
    world.db.attribute(THROWDOWN_COMPETITORS, pro, DEMO_COMPETITORS)?.data(index)
}

/// A `tCompetitorInfo`'s `ProfileName`.
#[must_use]
pub fn competitor_profile_name(world: &World, info: Option<Location>) -> Option<String> {
    info.and_then(|at| world.db.text_at(at)).map(str::to_owned)
}
#[cfg(test)]
mod world_file_tests {
    use super::*;

    #[test]
    fn sponsor_world_names_the_training_facility() {
        publish_sponsor_world(None);
        assert_eq!(world_file_name("BAM"), "BAM");
        assert_eq!(world_file_name("DIST_TrainingFacility_{0:SponsorWorld}"), "DIST_TrainingFacility_Online");
        publish_sponsor_world(Some("Element"));
        assert_eq!(world_file_name("DIST_TrainingFacility_{0:SponsorWorld}"), "DIST_TrainingFacility_Element");
        publish_sponsor_world(Some("Girl"));
        assert_eq!(world_file_name("DIST_TrainingFacility_{0:SponsorWorld}"), "DIST_TrainingFacility_Online");
        publish_sponsor_world(Some(""));
        assert_eq!(world_file_name("DIST_TrainingFacility_{0:SponsorWorld}"), "DIST_TrainingFacility_Online");
        publish_sponsor_world(None);
    }
}
