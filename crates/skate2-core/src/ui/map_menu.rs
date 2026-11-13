//! Native ChallengeMap's free-skate FELocation browser. Career entries require
//! their actual challenge owners and are not synthesized by this controller.

use super::pause_menu::PauseMenuInput;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum MapCategory {
    #[default]
    Root = 0,
    Locations = 10,
    /// The downloaded content, here holding only linked Skate 3 data's areas (PC port, as on the
    /// career's map); its downloaded spawn points are not ported.
    Downloaded = 11,
    Landmarks = 19,
    Plazas = 21,
    Skateparks = 22,
    SpawnPoints = 23,
    /// PC port: a linked Skate 3 area's teleports: its districts', then its downloadable content
    /// maps', in `crate::skate3::Skate3Data::districts` order.
    Skate3Area0 = 26,
    Skate3Area1 = 27,
    Skate3Area2 = 28,
    Skate3Area3 = 29,
    Skate3Area4 = 30,
    Skate3Area5 = 31,
    Skate3Area6 = 32,
    Skate3Area7 = 33,
    Skate3Area8 = 34,
    Skate3Area9 = 35,
    Skate3Area10 = 36,
    Skate3Area11 = 37,
}

impl MapCategory {
    /// PC port: the linked Skate 3 areas' categories, in `crate::skate3::Skate3Data::districts`
    /// order.
    pub const SKATE3_AREAS: [Self; crate::skate3::AREA_SLOTS] = [
        Self::Skate3Area0,
        Self::Skate3Area1,
        Self::Skate3Area2,
        Self::Skate3Area3,
        Self::Skate3Area4,
        Self::Skate3Area5,
        Self::Skate3Area6,
        Self::Skate3Area7,
        Self::Skate3Area8,
        Self::Skate3Area9,
        Self::Skate3Area10,
        Self::Skate3Area11,
    ];

    /// gStuffCategoryHAL82fdb2f0 and GetCategoryHAL8261cb48. A linked Skate 3 area is named by its
    /// title and count (`crate::skate3::area_label`), as the career's map names it
    /// (`stuff_menu`'s `category_hal`).
    #[must_use]
    pub fn localization_key(self) -> &'static str {
        if let Some(index) = self.skate3_area() {
            return crate::skate3::area_label(index);
        }
        match self {
            Self::Root => "ID_CHALLENGE_MAP_STUFF",
            Self::Locations => "ID_STUFFMENU_CATEGORY_LOCATIONS",
            Self::Downloaded => "ID_STUFFMENU_CATEGORY_DOWNLOADED",
            Self::Landmarks => "ID_STUFFMENU_CATEGORY_LANDMARKS",
            Self::Plazas => "ID_STUFFMENU_CATEGORY_LANDMARK_PLAZAS",
            Self::Skateparks => "ID_STUFFMENU_CATEGORY_LANDMARK_PARKS",
            Self::SpawnPoints => "ID_STUFFMENU_CATEGORY_SPAWNPOINTS",
            _ => "ID_COMMON_UNKNOWN",
        }
    }

    /// FEStuffMenu gSubCategory82079860's free-skate categories; the downloaded content holds
    /// the linked Skate 3 areas.
    const fn sub_categories(self) -> &'static [Self] {
        match self {
            Self::Root => &[Self::Locations, Self::Downloaded],
            Self::Locations => &[Self::Landmarks, Self::Plazas, Self::Skateparks, Self::SpawnPoints],
            Self::Downloaded => &Self::SKATE3_AREAS,
            _ => &[],
        }
    }

    /// PC port: the index of the linked Skate 3 area this category lists.
    #[must_use]
    pub fn skate3_area(self) -> Option<usize> {
        Self::SKATE3_AREAS.iter().position(|&area| area == self)
    }

    fn is_skate3(self) -> bool {
        self.skate3_area().is_some()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapLocation {
    pub key: u64,
    pub localization_key: String,
    pub category: MapCategory,
    pub world_position: [f32; 2],
    pub image: String,
    pub icon: String,
}

/// `FEMapManager::LoadMap`'s map: an image of `data\fe\source\images\map` and the six words of
/// its `fe_locations` record after the name (`MapInfo`'s extents, then the image's size).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapImage {
    pub name: String,
    pub words: [i32; 6],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapMenuSnapshot {
    pub visible: bool,
    pub category: MapCategory,
    pub selected: usize,
    pub info_location: Option<u64>,
    /// In the career, `FrontEndState_ChallengeMap`'s Stuff region and map icons, which the
    /// screen shows in place of the FELocation browser.
    pub career: Option<std::sync::Arc<crate::challenge::stuff_menu::ChallengeMapView>>,
    /// `LoadMap` (`0x8264ccf8`) with an additional world location: that location's map; none
    /// for the default location's, which the front end holds.
    pub map: Option<MapImage>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapMenuRow<'a> {
    pub localization_key: &'a str,
    pub location_key: Option<u64>,
    pub category: MapCategory,
    /// Native category labels format the number of available destinations.
    pub count: usize,
}

impl MapMenuSnapshot {
    /// GetNumSubMenuItems8261c8c0: the locations count Skate 2's destinations, the downloaded
    /// content its Skate 3 districts with teleports (as the career's map counts them), and any
    /// other category its destinations.
    #[must_use]
    pub fn count(category: MapCategory, locations: &[MapLocation]) -> usize {
        match category {
            MapCategory::Root | MapCategory::Locations => {
                locations.iter().filter(|location| !location.category.is_skate3()).count()
            }
            MapCategory::Downloaded => MapCategory::SKATE3_AREAS
                .iter()
                .filter(|&&area| locations.iter().any(|location| location.category == area))
                .count(),
            category => locations.iter().filter(|location| location.category == category).count(),
        }
    }

    /// FEStuffMenu gSubCategory82079860 / FilterCategory82629430. Empty
    /// categories are omitted, and authored destination order is retained.
    #[must_use]
    pub fn rows<'a>(&self, locations: &'a [MapLocation]) -> Vec<MapMenuRow<'a>> {
        if self.info_location.is_some() {
            return vec![MapMenuRow {
                localization_key: "ID_COMMON_TELEPORT",
                location_key: self.info_location,
                category: self.category,
                count: 0,
            }];
        }
        let categories = self.category.sub_categories();
        if !categories.is_empty() {
            return categories
                .iter()
                .filter_map(|&category| {
                    let count = Self::count(category, locations);
                    (count != 0).then_some(MapMenuRow {
                        localization_key: category.localization_key(),
                        location_key: None,
                        category,
                        count,
                    })
                })
                .collect();
        }
        locations
            .iter()
            .filter(|location| location.category == self.category)
            .map(|location| MapMenuRow {
                localization_key: &location.localization_key,
                location_key: Some(location.key),
                category: location.category,
                count: 0,
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapMenuAction {
    #[default]
    None,
    Close,
    Teleport(u64),
}

#[derive(Clone, Debug, Default)]
pub struct MapMenuController {
    state: MapMenuSnapshot,
    history: Vec<(MapCategory, usize)>,
    location_selection: usize,
    /// The `FE_SOUND_*` globals the menu pickers played, in order.
    sounds: Vec<&'static str>,
}

impl MapMenuController {
    #[must_use]
    pub fn snapshot(&self) -> MapMenuSnapshot {
        self.state.clone()
    }

    /// The `FE_SOUND_*` globals played since the last call, in order.
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    /// OnEnter826539e8 / OnLoaded826879b8 activate the Stuff region.
    pub fn open(&mut self) {
        self.state = MapMenuSnapshot {
            visible: true,
            ..MapMenuSnapshot::default()
        };
        self.history.clear();
    }

    pub fn close(&mut self) {
        self.state.visible = false;
    }

    /// ReceiveInput8266b420 and OnMenuSelect826828f0: select a category,
    /// select an FELocation, then confirm its Teleport option. The runtime
    /// owns travel completion, pause release and streaming synchronization.
    /// As on the career's map, the menu pickers sound `core_nav_up` / `core_nav_down` when their
    /// value changes and `core_a_button` before `OnMenuSelect`; going back a level sounds
    /// `core_b_button`.
    pub fn update(&mut self, input: PauseMenuInput, locations: &[MapLocation]) -> MapMenuAction {
        if !self.state.visible {
            return MapMenuAction::None;
        }
        if input.cancel {
            if self.state.info_location.take().is_some() {
                self.state.selected = self.location_selection;
            } else if let Some((category, selected)) = self.history.pop() {
                self.state.category = category;
                self.state.selected = selected;
                self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            } else {
                self.close();
                return MapMenuAction::Close;
            }
            return MapMenuAction::None;
        }
        let rows = self.state.rows(locations);
        if rows.is_empty() {
            return MapMenuAction::None;
        }
        self.state.selected = self.state.selected.min(rows.len() - 1);
        for forward in [input.up.then_some(false), input.down.then_some(true)]
            .into_iter()
            .flatten()
        {
            let before = self.state.selected;
            self.state.selected = if forward {
                (self.state.selected + 1).min(rows.len() - 1)
            } else {
                self.state.selected.saturating_sub(1)
            };
            if self.state.selected != before {
                self.sounds.push(if forward { "FE_SOUND_CORE_NAV_DOWN" } else { "FE_SOUND_CORE_NAV_UP" });
            }
        }
        if input.select {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            let row = rows[self.state.selected];
            if let Some(key) = self.state.info_location {
                return MapMenuAction::Teleport(key);
            }
            if let Some(key) = row.location_key {
                self.location_selection = self.state.selected;
                self.state.info_location = Some(key);
                self.state.selected = 0;
            } else {
                self.history
                    .push((self.state.category, self.state.selected));
                self.state.category = row.category;
                self.state.selected = 0;
            }
        }
        MapMenuAction::None
    }
}

/// PC port: a linked Skate 3 data folder's areas' teleports (`crate::skate3`): its districts',
/// then its downloadable content maps', each under its area's category and titled in Skate 3's
/// English, as the career's map lists them. None draws a map icon: its place is in its area's
/// world, not on this map.
#[must_use]
pub fn skate3_locations() -> Vec<MapLocation> {
    let Some(data) = crate::skate3::linked() else { return Vec::new() };
    let mut locations = Vec::new();
    for (district, &category) in data.districts.iter().zip(&MapCategory::SKATE3_AREAS) {
        locations.extend(district.spots.iter().map(|spot| MapLocation {
            key: spot.key,
            localization_key: format!("#{}", spot.title),
            category,
            world_position: [0.0; 2],
            image: crate::skate3::LOCATION_IMAGE.to_owned(),
            icon: String::new(),
        }));
    }
    locations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(key: u64, category: MapCategory) -> MapLocation {
        MapLocation {
            key,
            localization_key: format!("#{key}"),
            category,
            world_position: [0.0; 2],
            image: String::new(),
            icon: String::new(),
        }
    }

    #[test]
    fn the_district_titles_are_the_skate3_districts() {
        for (category, spec) in MapCategory::SKATE3_AREAS.into_iter().zip(crate::skate3::DISTRICTS) {
            assert_eq!(category.localization_key(), format!("#{} (%d)", spec.title));
        }
    }

    #[test]
    fn the_downloaded_content_lists_the_districts_with_teleports() {
        let locations = [
            location(1, MapCategory::Landmarks),
            location(2, MapCategory::Skate3Area0),
            location(3, MapCategory::Skate3Area0),
            location(4, MapCategory::Skate3Area2),
        ];
        let mut snapshot = MapMenuSnapshot::default();
        let root: Vec<_> = snapshot.rows(&locations).iter().map(|row| (row.category, row.count)).collect();
        assert_eq!(root, [(MapCategory::Locations, 1), (MapCategory::Downloaded, 2)]);
        snapshot.category = MapCategory::Downloaded;
        let districts: Vec<_> = snapshot.rows(&locations).iter().map(|row| (row.category, row.count)).collect();
        assert_eq!(districts, [(MapCategory::Skate3Area0, 2), (MapCategory::Skate3Area2, 1)]);
        snapshot.category = MapCategory::Skate3Area0;
        let teleports: Vec<_> = snapshot.rows(&locations).iter().map(|row| row.location_key).collect();
        assert_eq!(teleports, [Some(2), Some(3)]);
    }

    #[test]
    fn the_pickers_sound_as_on_the_careers_map() {
        let locations = [location(1, MapCategory::Landmarks), location(2, MapCategory::Plazas)];
        let mut menu = MapMenuController::default();
        menu.open();
        let press = |menu: &mut MapMenuController, input: PauseMenuInput| menu.update(input, &locations);
        press(&mut menu, PauseMenuInput { select: true, ..Default::default() });
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        press(&mut menu, PauseMenuInput { down: true, ..Default::default() });
        press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() });
        assert_eq!(menu.take_sounds(), ["FE_SOUND_CORE_A_BUTTON", "FE_SOUND_CORE_NAV_DOWN", "FE_SOUND_CORE_B_BUTTON"]);
        assert_eq!(press(&mut menu, PauseMenuInput { cancel: true, ..Default::default() }), MapMenuAction::Close);
        assert!(menu.take_sounds().is_empty());
    }
}
