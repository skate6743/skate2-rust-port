//! The online matchmaking screens over the Online options: `FrontEndState_UnrankedCustomMatch`
//! (FE state 69, screen 110 `unranked/unranked_custom_match.swf`) and
//! `FrontEndState_UnrankedFreeskate` (FE state 68, screen 109
//! `unranked/unranked_custom_freeskate.swf`). Their movies keep the selection and call
//! `Game.SetUnrankedCustomMatchOptions` / `SetUnrankedFreeskateOptions` (`SetMatchSettings`
//! 0x8269b3d0, `SetFreeskateSettings` 0x8268ee20) and `StartUnrankedCustomMatch` /
//! `StartUnrankedFreeskate` (0x826ae328, 0x826ab358); `SendUnrankedMatchMessage` 0x826a85a0 turns
//! the choice into the matchmaking request. The online hub's Host Options open both in their
//! host state (`SetHostState`, `m_bIsNormalMode` and `mIsNormalMode` off): the session's access
//! shows in place of the type, and A asks the game to change its challenge
//! (`Message_GameRequestChange`) instead of matchmaking.

use super::pause_menu::PauseMenuInput;
use crate::online::catalog::{self, OnlineChallenge};
use skate2_net::protocol::{PlayNowMode, challenge};

/// `typesArray`: Join or Create.
pub const MATCH_TYPES: [&str; 2] = ["ID_CUSTOM_UNRANKED_MATCH_JOIN", "ID_CUSTOM_UNRANKED_MATCH_CREATE"];
pub const FREESKATE_TYPES: [&str; 2] = ["ID_UNRANKEDFREESKATE_TYPE_JOIN", "ID_UNRANKEDFREESKATE_TYPE_CREATE"];
/// `mDefaultImage`: the location image of "any location".
pub const ANY_LOCATION_IMAGE: &str = "fe\\source\\images\\locations\\any_location";
/// PC port: with Skate 3 data linked (`crate::skate3`), the first row picks the maps the screen
/// lists (literal text, `#`): Skate 2's, Skate 3's districts', or the maps Skate 3's downloadable
/// content adds. A pool without games of the screen's is left out.
pub const MAP_POOL_TITLE: &str = "#Map Pool";
pub const SKATE2_POOL: &str = "#Skate 2";
pub const SKATE3_POOL: &str = "#Skate 3";
pub const SKATE3_DLC_POOL: &str = "#Skate 3 DLC";

/// What the screen's A asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchChoice {
    pub mode: PlayNowMode,
    pub challenge_type: u8,
    /// The chosen challenge, `None` for any location.
    pub challenge_key: Option<u64>,
    /// Every challenge the choice could be, for a new game to start at.
    pub candidates: Vec<u64>,
    /// Create (a private session) rather than Join.
    pub create: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Back,
    Start(MatchChoice),
}

/// One selector row as `UpdateSelector` sets it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selector {
    pub title: String,
    pub subtitle: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// `ID_CUSTOM_MATCH_TITLE` or `ID_CUSTOM_FREESKATE_TITLE`.
    pub title: &'static str,
    pub selectors: Vec<Selector>,
    /// `mPreviewImage`'s `LoadRX2Texture` path.
    pub image: String,
    /// `txtHelp`: the challenge type's description (custom match only).
    pub description: Option<&'static str>,
    /// `mMatch`'s text while matchmaking (`SetupMatchMaking`).
    pub matchmaking: Option<&'static str>,
    pub freeskate: bool,
}

/// `unranked_custom_match.swf`'s state: Type, Event and Location.
#[derive(Clone, Debug)]
pub struct CustomMatch {
    /// `challengeArray`: the network challenge types offered.
    types: Vec<u8>,
    /// Each type's challenges (`GetNumberOfDistricts` and its lists).
    locations: Vec<Vec<OnlineChallenge>>,
    selector: usize,
    type_index: usize,
    challenge_index: usize,
    location_index: usize,
    matchmaking: bool,
    /// `mHostOptionState`: the session's privacy.
    host: Option<bool>,
    pub ready: bool,
    sounds: Vec<&'static str>,
    /// PC port: each Map Pool's name, types and their challenges (Skate 2's first), and the one
    /// shown; none without the Map Pool row.
    pools: Vec<(&'static str, Vec<u8>, Vec<Vec<OnlineChallenge>>)>,
    pool: usize,
}

/// The selector after or before `current` among `rows` (wrapping).
fn step_row(rows: &[usize], current: usize, forward: bool) -> usize {
    let at = rows.iter().position(|&row| row == current).unwrap_or(0);
    let count = rows.len().max(1);
    rows.get(if forward { (at + 1) % count } else { (at + count - 1) % count }).copied().unwrap_or(current)
}

/// `HostActions_IsPrivate`'s subtitle of the host state's first selector.
const fn session_access(private: bool) -> &'static str {
    if private { "ID_ONLINE_PRIVATESESSION" } else { "ID_ONLINE_PUBLICSESSION" }
}

impl CustomMatch {
    /// `InitializeScreen` and `SetNormalState`: the first type, the first challenge type and its
    /// first location, on the first selector.
    #[must_use]
    pub fn new(types: Vec<(u8, Vec<OnlineChallenge>)>) -> Self {
        let (types, locations) = types.into_iter().unzip();
        Self { types, locations, selector: 0, type_index: 0, challenge_index: 0, location_index: 0, matchmaking: false, host: None, ready: false, sounds: Vec::new(), pools: Vec::new(), pool: 0 }
    }

    /// PC port: the Map Pool row over Skate 2's types and each pool of `more` (named; one without
    /// types is left out), the pool named `shown` shown first, else Skate 2's. Without another
    /// pool there is no row.
    #[must_use]
    pub fn with_pools(mut self, more: Vec<(&'static str, Vec<(u8, Vec<OnlineChallenge>)>)>, shown: &str) -> Self {
        let more: Vec<_> = more.into_iter().filter(|(_, types)| !types.is_empty()).collect();
        if more.is_empty() {
            return self;
        }
        self.pools.push((SKATE2_POOL, std::mem::take(&mut self.types), std::mem::take(&mut self.locations)));
        for (name, types) in more {
            let (types, locations) = types.into_iter().unzip();
            self.pools.push((name, types, locations));
        }
        self.selector += 1;
        self.show_pool(self.pools.iter().position(|(name, ..)| *name == shown).unwrap_or(0));
        self
    }

    fn show_pool(&mut self, pool: usize) {
        let Some((_, types, locations)) = self.pools.get(pool) else { return };
        (self.types, self.locations) = (types.clone(), locations.clone());
        self.pool = pool;
        self.challenge_index = 0;
        self.location_index = 0;
    }

    /// The Map Pool row's offset: the type, event and location rows follow it.
    fn pool_rows(&self) -> usize {
        usize::from(!self.pools.is_empty())
    }

    /// The rows up and down move between: the Map Pool, the type (not in the host state), the
    /// event and the location.
    fn selectable(&self) -> Vec<usize> {
        let pool = self.pool_rows();
        let mut rows: Vec<usize> = (0..pool).collect();
        if self.host.is_none() {
            rows.push(pool);
        }
        rows.extend([pool + 1, pool + 2]);
        rows
    }

    /// `SetHostState`: Match Setup over the hub, on the event selector with Create's locations;
    /// up and down keep to the event and location (`iFirstSelector` 1).
    #[must_use]
    pub fn host(types: Vec<(u8, Vec<OnlineChallenge>)>, private: bool) -> Self {
        Self { selector: 1, type_index: 1, host: Some(private), ..Self::new(types) }
    }

    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    fn create(&self) -> bool {
        self.type_index == 1
    }

    fn challenges(&self) -> &[OnlineChallenge] {
        self.locations.get(self.challenge_index).map_or(&[], Vec::as_slice)
    }

    /// The `locationsArray` entries: Join has "any" first.
    fn location_count(&self) -> usize {
        self.challenges().len() + usize::from(!self.create())
    }

    /// The chosen challenge; `None` for Join's "any".
    fn location(&self) -> Option<&OnlineChallenge> {
        if self.create() { self.challenges().get(self.location_index) } else { self.location_index.checked_sub(1).and_then(|index| self.challenges().get(index)) }
    }

    /// `UpdateInput`: left and right step the current selector (wrapping, a new type or event
    /// back to the first location), up and down move between selectors, A starts.
    pub fn update(&mut self, input: PauseMenuInput) -> Action {
        if !self.ready || self.matchmaking {
            return Action::None;
        }
        if input.cancel {
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            return Action::Back;
        }
        for (pressed, step) in [(input.left, -1_isize), (input.right, 1)] {
            if !pressed {
                continue;
            }
            self.sounds.push(if step < 0 { "FE_SOUND_CORE_NAV_LEFT" } else { "FE_SOUND_CORE_NAV_RIGHT" });
            let wrap = |value: usize, count: usize| (value.cast_signed() + step).rem_euclid(count.max(1).cast_signed()).cast_unsigned();
            if !self.pools.is_empty() && self.selector == 0 {
                self.show_pool(wrap(self.pool, self.pools.len()));
                continue;
            }
            match self.selector - self.pool_rows() {
                0 => {
                    self.type_index = wrap(self.type_index, MATCH_TYPES.len());
                    self.location_index = 0;
                }
                1 => {
                    self.challenge_index = wrap(self.challenge_index, self.types.len());
                    self.location_index = 0;
                }
                _ => self.location_index = wrap(self.location_index, self.location_count()),
            }
        }
        if input.up {
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
            self.selector = step_row(&self.selectable(), self.selector, false);
        }
        if input.down {
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
            self.selector = step_row(&self.selectable(), self.selector, true);
        }
        if input.select && !self.types.is_empty() {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.matchmaking = self.host.is_none();
            let challenge_type = self.types[self.challenge_index];
            return Action::Start(MatchChoice {
                mode: PlayNowMode::Custom,
                challenge_type,
                challenge_key: self.location().map(|location| location.key),
                candidates: self.challenges().iter().map(|location| location.key).collect(),
                create: self.create(),
            });
        }
        Action::None
    }

    /// Matchmaking failed: back to the selectors (`SetState(mNormalState)`).
    pub fn matchmaking_failed(&mut self) {
        self.matchmaking = false;
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let challenge_type = self.types.get(self.challenge_index).copied().unwrap_or(challenge::SKATE);
        let location = if self.create() || self.location_index > 0 {
            self.location().map_or_else(|| "ID_COMMON_UNKNOWN".to_owned(), |location| location.title.clone())
        } else {
            "ID_UNRANKEDCUSTOMMATCH_FIRST_EVENT".to_owned()
        };
        let image = match self.location() {
            Some(location) => format!("fe\\source\\images\\locations\\{}", crate::skate3::small_location_image(&location.image)),
            None => ANY_LOCATION_IMAGE.to_owned(),
        };
        let pool = self.pool_rows();
        let selector = |index: usize, title: &str, subtitle: String| Selector { title: title.to_owned(), subtitle, selected: self.selector == index + pool };
        let first = match self.host {
            Some(private) => selector(0, "ID_ONLINE_CURRENT_TYPE", session_access(private).to_owned()),
            None => selector(0, "ID_UNRANKEDCUSTOMMATCH_TYPE", MATCH_TYPES[self.type_index].to_owned()),
        };
        let mut selectors = Vec::with_capacity(4);
        if let Some((name, ..)) = self.pools.get(self.pool) {
            selectors.push(Selector { title: MAP_POOL_TITLE.to_owned(), subtitle: (*name).to_owned(), selected: self.selector == 0 });
        }
        selectors.extend([
            first,
            selector(1, "ID_UNRANKEDCUSTOMMATCH_EVENT", catalog::challenge_title(challenge_type).to_owned()),
            selector(2, "ID_UNRANKEDCUSTOMMATCH_LOCATION", location),
        ]);
        Snapshot {
            title: if self.host.is_some() { "ID_ONLINE_HUB_NEW_CHALLENGE" } else { "ID_CUSTOM_MATCH_TITLE" },
            selectors,
            image,
            description: Some(catalog::challenge_description(challenge_type)),
            matchmaking: self.matchmaking.then_some(if self.create() { "ID_ONLINE_CREATING_SESSION" } else { "ID_RANKED_MATCHMAKING" }),
            freeskate: false,
        }
    }
}

/// `unranked_custom_freeskate.swf`'s state: Type and Location of an online free skate.
#[derive(Clone, Debug)]
pub struct FreeskateMatch {
    locations: Vec<OnlineChallenge>,
    selector: usize,
    type_index: usize,
    location_index: usize,
    matchmaking: bool,
    /// `mHostOptionState`: the session's privacy.
    host: Option<bool>,
    pub ready: bool,
    sounds: Vec<&'static str>,
    /// PC port: each Map Pool's name and locations (Skate 2's first), and the one shown; none
    /// without the Map Pool row.
    pools: Vec<(&'static str, Vec<OnlineChallenge>)>,
    pool: usize,
}

impl FreeskateMatch {
    #[must_use]
    pub fn new(locations: Vec<OnlineChallenge>) -> Self {
        Self { locations, selector: 0, type_index: 0, location_index: 0, matchmaking: false, host: None, ready: false, sounds: Vec::new(), pools: Vec::new(), pool: 0 }
    }

    /// PC port: the Map Pool row over Skate 2's locations and each pool of `more` (named; one
    /// without locations is left out), the pool named `shown` shown first, else Skate 2's.
    /// Without another pool there is no row.
    #[must_use]
    pub fn with_pools(mut self, more: Vec<(&'static str, Vec<OnlineChallenge>)>, shown: &str) -> Self {
        let more: Vec<_> = more.into_iter().filter(|(_, locations)| !locations.is_empty()).collect();
        if more.is_empty() {
            return self;
        }
        self.pools.push((SKATE2_POOL, std::mem::take(&mut self.locations)));
        self.pools.extend(more);
        self.selector += 1;
        self.show_pool(self.pools.iter().position(|(name, _)| *name == shown).unwrap_or(0));
        self
    }

    fn show_pool(&mut self, pool: usize) {
        let Some((_, locations)) = self.pools.get(pool) else { return };
        self.locations = locations.clone();
        self.pool = pool;
        self.location_index = 0;
    }

    /// The Map Pool row's offset: the type and location rows follow it.
    fn pool_rows(&self) -> usize {
        usize::from(!self.pools.is_empty())
    }

    /// `SetHostState`: Change Location over the hub, on the location selector.
    #[must_use]
    pub fn host(locations: Vec<OnlineChallenge>, private: bool) -> Self {
        Self { selector: 1, host: Some(private), ..Self::new(locations) }
    }

    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    /// `UpdateInput`: left and right step the selector, up and down switch selectors, A starts
    /// (`StartUnrankedFreeskate`).
    pub fn update(&mut self, input: PauseMenuInput) -> Action {
        if !self.ready || self.matchmaking {
            return Action::None;
        }
        if input.cancel {
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            return Action::Back;
        }
        for (pressed, step) in [(input.left, -1_isize), (input.right, 1)] {
            if !pressed {
                continue;
            }
            self.sounds.push(if step < 0 { "FE_SOUND_CORE_NAV_LEFT" } else { "FE_SOUND_CORE_NAV_RIGHT" });
            let wrap = |value: usize, count: usize| (value.cast_signed() + step).rem_euclid(count.max(1).cast_signed()).cast_unsigned();
            let pool = self.pool_rows();
            if !self.pools.is_empty() && self.selector == 0 {
                self.show_pool(wrap(self.pool, self.pools.len()));
            } else if self.selector == pool {
                self.type_index = wrap(self.type_index, FREESKATE_TYPES.len());
            } else {
                self.location_index = wrap(self.location_index, self.locations.len());
            }
        }
        if input.up || input.down {
            self.sounds.push(if input.up { "FE_SOUND_CORE_NAV_UP" } else { "FE_SOUND_CORE_NAV_DOWN" });
            let rows: Vec<usize> = (0..self.pool_rows() + 2).collect();
            self.selector = step_row(&rows, self.selector, input.down);
        }
        if input.select && !self.locations.is_empty() {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.matchmaking = self.host.is_none();
            return Action::Start(MatchChoice {
                mode: PlayNowMode::Freeskate,
                challenge_type: challenge::ONLINE_FREE_SKATE,
                challenge_key: self.locations.get(self.location_index).map(|location| location.key),
                candidates: self.locations.iter().map(|location| location.key).collect(),
                create: self.type_index == 1,
            });
        }
        Action::None
    }

    pub fn matchmaking_failed(&mut self) {
        self.matchmaking = false;
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let location = self.locations.get(self.location_index);
        let pool = self.pool_rows();
        let selector = |index: usize, title: &str, subtitle: String| Selector { title: title.to_owned(), subtitle, selected: self.selector == index + pool };
        let first = match self.host {
            Some(private) => selector(0, "ID_ONLINE_CURRENT_TYPE", session_access(private).to_owned()),
            None => selector(0, "ID_UNRANKEDFREESKATE_TYPE", FREESKATE_TYPES[self.type_index].to_owned()),
        };
        let mut selectors = Vec::with_capacity(3);
        if let Some((name, _)) = self.pools.get(self.pool) {
            selectors.push(Selector { title: MAP_POOL_TITLE.to_owned(), subtitle: (*name).to_owned(), selected: self.selector == 0 });
        }
        selectors.extend([
            first,
            selector(1, "ID_UNRANKEDFREESKATE_LOCATION", location.map_or_else(|| "ID_COMMON_UNKNOWN".to_owned(), |location| location.title.clone())),
        ]);
        Snapshot {
            title: if self.host.is_some() { "ID_ONLINE_HUB_CHANGE_LOCATION" } else { "ID_CUSTOM_FREESKATE_TITLE" },
            selectors,
            image: location.map_or_else(
                || ANY_LOCATION_IMAGE.to_owned(),
                |location| format!("fe\\source\\images\\locations\\{}", crate::skate3::small_location_image(&location.image)),
            ),
            description: None,
            matchmaking: self.matchmaking.then_some(if self.type_index == 1 { "ID_ONLINE_CREATING_SESSION" } else { "ID_RANKED_MATCHMAKING" }),
            freeskate: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn challenge(key: u64) -> OnlineChallenge {
        OnlineChallenge { key, title: format!("T{key}"), image: format!("i{key}"), location: String::new() }
    }

    fn press(f: impl FnOnce(&mut PauseMenuInput)) -> PauseMenuInput {
        let mut input = PauseMenuInput::default();
        f(&mut input);
        input
    }

    #[test]
    fn join_offers_any_location_first_and_create_does_not() {
        let mut screen = CustomMatch::new(vec![(challenge::SKATE, vec![challenge(1), challenge(2)]), (challenge::JAM, vec![challenge(3)])]);
        screen.ready = true;
        assert_eq!(screen.snapshot().selectors[2].subtitle, "ID_UNRANKEDCUSTOMMATCH_FIRST_EVENT");
        screen.update(press(|i| i.down = true));
        screen.update(press(|i| i.down = true));
        screen.update(press(|i| i.right = true));
        assert_eq!(screen.snapshot().selectors[2].subtitle, "T1");
        screen.update(press(|i| i.right = true));
        screen.update(press(|i| i.right = true));
        assert_eq!(screen.snapshot().selectors[2].subtitle, "ID_UNRANKEDCUSTOMMATCH_FIRST_EVENT");
        screen.update(press(|i| i.down = true));
        screen.update(press(|i| i.right = true));
        let Action::Start(choice) = screen.update(press(|i| i.select = true)) else { panic!("start") };
        assert!(choice.create);
        assert_eq!(choice.challenge_key, Some(1));
        assert_eq!(screen.update(press(|i| i.cancel = true)), Action::None);
    }

    #[test]
    fn the_map_pool_steps_through_skate2_skate3_and_skate3_dlc() {
        let skate3 = vec![(challenge::SKATE, vec![challenge(7)])];
        let dlc = vec![(challenge::SKATE, vec![challenge(9)])];
        let mut screen = CustomMatch::new(vec![(challenge::SKATE, vec![challenge(1)])]).with_pools(vec![(SKATE3_POOL, skate3), (SKATE3_DLC_POOL, dlc)], SKATE3_DLC_POOL);
        screen.ready = true;
        let pool = |screen: &CustomMatch| screen.snapshot().selectors[0].subtitle.clone();
        assert_eq!(pool(&screen), SKATE3_DLC_POOL, "the game's own pool shows first");
        screen.update(press(|i| i.up = true));
        screen.update(press(|i| i.right = true));
        assert_eq!(pool(&screen), SKATE2_POOL, "the pools wrap");
        screen.update(press(|i| i.left = true));
        screen.update(press(|i| i.left = true));
        assert_eq!(pool(&screen), SKATE3_POOL);
        screen.update(press(|i| i.down = true));
        screen.update(press(|i| i.right = true));
        screen.update(press(|i| i.down = true));
        screen.update(press(|i| i.down = true));
        screen.update(press(|i| i.right = true));
        let Action::Start(choice) = screen.update(press(|i| i.select = true)) else { panic!("start") };
        assert_eq!((choice.challenge_key, choice.candidates), (Some(7), vec![7]));
        // Without downloadable content maps, two pools.
        let free = FreeskateMatch::new(vec![challenge(1)]).with_pools(vec![(SKATE3_POOL, vec![challenge(7)]), (SKATE3_DLC_POOL, Vec::new())], SKATE2_POOL);
        assert_eq!(free.pools.iter().map(|(name, _)| *name).collect::<Vec<_>>(), [SKATE2_POOL, SKATE3_POOL]);
        // Without any other, no row.
        let alone = FreeskateMatch::new(vec![challenge(1)]).with_pools(vec![(SKATE3_POOL, Vec::new())], SKATE2_POOL);
        assert_ne!(alone.snapshot().selectors[0].title, MAP_POOL_TITLE);
    }

    #[test]
    fn host_state_shows_the_session_access_and_keeps_to_event_and_location() {
        let mut screen = CustomMatch::host(vec![(challenge::SKATE, vec![challenge(1), challenge(2)])], true);
        screen.ready = true;
        let snapshot = screen.snapshot();
        assert_eq!(snapshot.title, "ID_ONLINE_HUB_NEW_CHALLENGE");
        assert_eq!(snapshot.selectors[0].subtitle, "ID_ONLINE_PRIVATESESSION");
        assert!(snapshot.selectors[1].selected);
        assert_eq!(snapshot.selectors[2].subtitle, "T1");
        screen.update(press(|i| i.up = true));
        assert!(screen.snapshot().selectors[2].selected);
        screen.update(press(|i| i.right = true));
        let Action::Start(choice) = screen.update(press(|i| i.select = true)) else { panic!("start") };
        assert_eq!(choice.challenge_key, Some(2));
        assert_eq!(screen.snapshot().matchmaking, None);
    }
}
