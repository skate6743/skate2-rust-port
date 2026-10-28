//! Freeskate activities: the activities of an online free skate location
//! (`ChallengeOnlineManager::GetOnlineFreeSkateActivities` 0x825bf3d0, its
//! `OnlineFreeSkateActivities`), what the online front end shows of each, and the completed ones a
//! profile keeps (`OnlineBackEnd::OnChallengeCompleted` 0x8257d3b8, `SaveData` 0x8255d338).

use crate::challenge::attrib_db::AttribDb;

/// The `challenges` class and the attributes of a location and its activities.
const CHALLENGES: u64 = 0x8876_E0B5_5674_0E36;
const ONLINE_FREE_SKATE_ACTIVITIES: u64 = 0xABE7_49C8_0E8A_DBCE;
const TITLE: u64 = 0xE2A5_0788_B6C2_0C92;
const DESCRIPTION: u64 = 0x6B0B_D2BA_E508_B742;
const FREESKATE_ACTIVITY_TYPE: u64 = 0x85D6_B2C6_AE61_DD8B;
const FREESKATE_ACCUMULATION_TYPE: u64 = 0xA74F_EC89_C254_DC43;
const FREESKATE_SURVIVAL_TYPE: u64 = 0x6279_1C96_0AAD_AB5F;
const GLOBAL_DATA: u64 = 0x2449_1BF0_0316_AA45;
const FE_LARGE_IMAGE: u64 = 0x7974_27ED_230C_9980;

/// `eChallengeTypes_OnlineFreeSkateActivity`.
pub const CHALLENGE_TYPE: i32 = 42;

/// `FreeskateActivityOverlayHUDData` as the overlay movie reads it
/// (`HUDComponentsCmdTranslator::sMethod_FreeskateActivityOverlay_GetData` 0x826bb768 and
/// `GetTime` 0x826bb7d0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayView {
    /// `mActivityType` (`GetActivityTypeHAL`) and `mGamerTagName`.
    pub type_title: &'static str,
    pub proposer: String,
    /// `mCurrentTime`: the whole seconds left.
    pub seconds: u64,
    /// `mCurrentMode == eFreeskateActivityModes_Accepting`: D-pad up opens the sign-up.
    pub accepting: bool,
}

/// FE state 67's data as screen 108's `challenge_info` reads it (`GameCmdTranslator`'s
/// `Online_Signup_GetData` 0x82622680, `GetCurrentTime` 0x82622700 and `GetCurrentMode`
/// 0x82622738).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignUpView {
    /// `mActivityTypeTitle` (the activity's `Title`), `mDescription` and `mGamerTag`.
    pub title: String,
    pub description: String,
    pub gamer_tag: String,
    /// `GetFreeskateTypeDescription`.
    pub type_description: &'static str,
    /// `mImageName`, the location's `FELargeImage`.
    pub image: String,
    /// `mMode == eFreeskateActivityModes_Accepting`.
    pub accepting: bool,
    /// `GetTime`: the whole seconds left.
    pub seconds: u64,
}

/// `Sk8::Challenge::eFreeskateType`.
pub mod kind {
    pub const ACCUMULATION: i32 = 0;
    pub const GAP: i32 = 1;
    pub const TAG: i32 = 2;
    pub const SIMULTANEOUS_TRICK: i32 = 3;
    pub const SURVIVAL: i32 = 4;
    pub const TRICK_LIST: i32 = 5;
}

/// `SetupFreeskateActivitiesMenu` 0x8268a500's types, in its order.
pub const MENU_KINDS: [i32; 6] = [kind::GAP, kind::ACCUMULATION, kind::TRICK_LIST, kind::SIMULTANEOUS_TRICK, kind::SURVIVAL, kind::TAG];

/// `eFreeskateSurvivalType_Time`.
const SURVIVAL_TIME: i32 = 1;

/// One activity of a location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activity {
    pub key: u64,
    /// `Title` and `Description`, HAL ids.
    pub title: String,
    pub description: String,
    /// `FreeskateActivityType`, `FreeskateAccumulationType` and `FreeskateSurvivalType`.
    pub kind: i32,
    pub accumulation: i32,
    pub survival: i32,
}

/// `GetOnlineFreeSkateActivities` of the location challenge: its activities in their order.
#[must_use]
pub fn activities(db: &AttribDb, location: u64) -> Vec<Activity> {
    let Some(list) = db.attribute(CHALLENGES, location, ONLINE_FREE_SKATE_ACTIVITIES) else { return Vec::new() };
    (0..list.len())
        .filter_map(|index| {
            let (_, key) = list.ref_spec(index)?;
            let int = |field| db.attribute(CHALLENGES, key, field).and_then(|value| value.i32(0)).unwrap_or(0);
            let text = |field| db.attribute(CHALLENGES, key, field).and_then(|value| value.text(0)).unwrap_or("ID_COMMON_UNKNOWN").to_owned();
            Some(Activity {
                key,
                title: text(TITLE),
                description: text(DESCRIPTION),
                kind: int(FREESKATE_ACTIVITY_TYPE),
                accumulation: int(FREESKATE_ACCUMULATION_TYPE),
                survival: int(FREESKATE_SURVIVAL_TYPE),
            })
        })
        .collect()
}

/// The location challenge's global data `FELargeImage`, the image under
/// `fe\source\images\locations\` the sign-up shows (`GoToFreeskateActivityCIC` 0x82661a98),
/// "default" without one.
#[must_use]
pub fn location_image(db: &AttribDb, location: u64) -> String {
    db.attribute(CHALLENGES, location, GLOBAL_DATA)
        .and_then(|global| global.ref_spec(0))
        .and_then(|(class, key)| db.attribute(class, key, FE_LARGE_IMAGE))
        .and_then(|image| image.text(0))
        .filter(|image| !image.is_empty())
        .unwrap_or("default")
        .to_owned()
}

/// `FrontEndState_OnlineHubMenu::GetActivityTypeHAL` 0x826451c8.
#[must_use]
pub const fn type_title(kind: i32) -> &'static str {
    match kind {
        kind::ACCUMULATION => "ID_CHALLENGE_TYPE_ACCUMULATION",
        kind::GAP => "ID_CHALLENGE_TYPE_GAP",
        kind::TAG => "ID_CHALLENGE_TYPE_TAG",
        kind::SIMULTANEOUS_TRICK => "ID_CHALLENGE_TYPE_SIMULTANEOUSTRICK",
        kind::SURVIVAL => "ID_CHALLENGE_TYPE_SURVIVAL",
        kind::TRICK_LIST => "ID_CHALLENGE_TYPE_GROUPTRICKLIST",
        _ => "#",
    }
}

/// `FrontEndState_FreeskateActivitySignUp::GetFreeskateTypeDescription` 0x8268ecc8.
#[must_use]
pub const fn type_description(kind: i32) -> &'static str {
    match kind {
        kind::ACCUMULATION => "ID_FSA_ACCUMULATION_DESCRIPTION",
        kind::GAP => "ID_FSA_GAPCHALLENGE_DESCRIPTION",
        kind::TAG => "ID_FSA_TAGTHESPOTS_DESCRIPTION",
        kind::SIMULTANEOUS_TRICK => "ID_FSA_SIMULTRICK_DESCRIPTION",
        kind::SURVIVAL => "ID_FSA_GROUPSURVIVAL_DESCRIPTION",
        kind::TRICK_LIST => "ID_FSA_TRICKLIST_DESCRIPTION",
        _ => "#",
    }
}

/// `StateOnlineGame::OnMessage(MT_GameFinalResults)` 0x8276b060's `eResultsLBType` of an
/// activity: Accumulation's string column for grind distance and air distances, else its points;
/// Gap and Tag place, name, score and time; Simultaneous Trick Death Race's time; any other
/// place, name and score.
#[must_use]
pub const fn results_type(kind: i32, accumulation: i32) -> i32 {
    const PLACE_NAME_INT: i32 = 0;
    const DEATH_RACE: i32 = 3;
    const PLACE_NAME_SCORE_TIME: i32 = 6;
    const PLACE_NAME_STRING: i32 = 7;
    match kind {
        kind::ACCUMULATION if matches!(accumulation, 2 | 5 | 6) => PLACE_NAME_STRING,
        kind::GAP | kind::TAG => PLACE_NAME_SCORE_TIME,
        kind::SIMULTANEOUS_TRICK => DEATH_RACE,
        _ => PLACE_NAME_INT,
    }
}

/// `GetChallengeColumnHeader` 0x82605a50 of an activity: Gap, Tag and Group Trick List count
/// objectives, Simultaneous Trick the time, a timed Survival none, the rest trick points.
#[must_use]
pub const fn column_header(kind: i32, survival: i32) -> &'static str {
    match kind {
        kind::GAP | kind::TAG | kind::TRICK_LIST => "ID_LEADERBOARD_HEADER_OBJECTIVES",
        kind::SIMULTANEOUS_TRICK => "ID_LEADERBOARD_HEADER_TIME",
        kind::SURVIVAL if survival == SURVIVAL_TIME => "",
        _ => "ID_LEADERBOARD_HEADER_TRICK_POINTS",
    }
}

/// The profile's savable the completed activities belong to (`OnlineBackEnd`), the savables
/// `BackEndManager::Init` registers before it, and its leaves.
pub const SAVABLE: &str = "online";
pub const SAVABLES_BEFORE: [&str; 3] = ["UNLOCKABLES", "CAC", "SKATER_PROFILE"];
pub const COMPLETED: &str = "completed_fsa";
pub const COMPLETED_DLC: &str = "completed_dlc_fsa";
pub const COMPLETED_VERSION: u32 = 1;
/// `mCompletedFreeSkateActivities` and `mCompletedDLCFreeSkateActivities`.
const CAPACITY: usize = 256;
const DLC_CAPACITY: usize = 128;
/// `DefaultData` 0x8256c440's empty slot.
const EMPTY: u64 = u64::MAX;

/// The activities the profile completed, in the order it did (`mCompletedFreeSkateActivities`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Completed {
    keys: Vec<u64>,
}

impl Completed {
    /// `LoadData` 0x8255d440's `completed_fsa` leaf: its count, then the 256 slots; a leaf of
    /// another version leaves none (`DefaultData`).
    #[must_use]
    pub fn from_leaf(version: u32, tuples: &[Vec<u8>]) -> Self {
        if version != COMPLETED_VERSION {
            return Self::default();
        }
        let word = |tuple: Option<&Vec<u8>>| tuple.and_then(|bytes| <[u8; 4]>::try_from(bytes.as_slice()).ok()).map_or(0, u32::from_be_bytes);
        let count = usize::try_from(word(tuples.first())).unwrap_or(0).min(CAPACITY);
        let keys = tuples
            .iter()
            .skip(1)
            .take(count)
            .filter_map(|bytes| <[u8; 8]>::try_from(bytes.as_slice()).ok().map(u64::from_be_bytes))
            .filter(|&key| key != EMPTY)
            .collect();
        Self { keys }
    }

    /// `IsFSACompleted` 0x82566978.
    #[must_use]
    pub fn contains(&self, key: u64) -> bool {
        self.keys.contains(&key)
    }

    /// `OnChallengeCompleted`: an activity not yet completed joins the list, while there is room.
    /// Whether it did.
    pub fn add(&mut self, key: u64) -> bool {
        if self.contains(key) || self.keys.len() >= CAPACITY {
            return false;
        }
        self.keys.push(key);
        true
    }

    /// `SaveData`'s leaves: `completed_fsa`'s count and its 256 slots, and `completed_dlc_fsa`'s
    /// (the PC port keeps no downloadable activity apart), each slot past the count empty.
    #[must_use]
    pub fn leaves(&self) -> [(&'static str, u32, Vec<Vec<u8>>); 2] {
        let leaf = |keys: &[u64], capacity: usize| {
            let mut tuples = Vec::with_capacity(capacity + 1);
            tuples.push(u32::try_from(keys.len()).unwrap_or(0).to_be_bytes().to_vec());
            tuples.extend((0..capacity).map(|slot| keys.get(slot).copied().unwrap_or(EMPTY).to_be_bytes().to_vec()));
            tuples
        };
        [(COMPLETED, COMPLETED_VERSION, leaf(&self.keys, CAPACITY)), (COMPLETED_DLC, COMPLETED_VERSION, leaf(&[], DLC_CAPACITY))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_activities_round_trip_their_leaf() {
        let mut completed = Completed::default();
        assert!(completed.add(0x433d_b94f_fbfa_c4a9));
        assert!(!completed.add(0x433d_b94f_fbfa_c4a9), "once");
        assert!(completed.add(7));
        let [(name, version, tuples), (dlc, _, dlc_tuples)] = completed.leaves();
        assert_eq!((name, version, tuples.len(), dlc, dlc_tuples.len()), (COMPLETED, 1, 257, COMPLETED_DLC, 129));
        assert_eq!(tuples[3], u64::MAX.to_be_bytes(), "an empty slot");
        let loaded = Completed::from_leaf(version, &tuples);
        assert_eq!(loaded, completed);
        assert!(loaded.contains(7) && !loaded.contains(8));
        assert_eq!(Completed::from_leaf(2, &tuples), Completed::default(), "another version");
    }

    #[test]
    fn an_activity_s_board_follows_its_type() {
        assert_eq!(results_type(kind::GAP, 0), 6);
        assert_eq!(results_type(kind::ACCUMULATION, 2), 7);
        assert_eq!(results_type(kind::ACCUMULATION, 0), 0);
        assert_eq!(results_type(kind::SIMULTANEOUS_TRICK, 0), 3);
        assert_eq!(column_header(kind::SURVIVAL, 1), "");
        assert_eq!(column_header(kind::SURVIVAL, 0), "ID_LEADERBOARD_HEADER_TRICK_POINTS");
        assert_eq!(column_header(kind::TAG, 0), "ID_LEADERBOARD_HEADER_OBJECTIVES");
    }
}
