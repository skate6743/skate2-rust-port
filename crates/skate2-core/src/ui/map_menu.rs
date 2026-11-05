//! Native ChallengeMap's free-skate FELocation browser. Career entries require
//! their actual challenge owners and are not synthesized by this controller.

use super::pause_menu::PauseMenuInput;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum MapCategory {
    #[default]
    Root = 0,
    Locations = 10,
    Landmarks = 19,
    Plazas = 21,
    Skateparks = 22,
    SpawnPoints = 23,
}

impl MapCategory {
    /// gStuffCategoryHAL82fdb2f0 and GetCategoryHAL8261cb48.
    #[must_use]
    pub const fn localization_key(self) -> &'static str {
        match self {
            Self::Root => "ID_CHALLENGE_MAP_STUFF",
            Self::Locations => "ID_STUFFMENU_CATEGORY_LOCATIONS",
            Self::Landmarks => "ID_STUFFMENU_CATEGORY_LANDMARKS",
            Self::Plazas => "ID_STUFFMENU_CATEGORY_LANDMARK_PLAZAS",
            Self::Skateparks => "ID_STUFFMENU_CATEGORY_LANDMARK_PARKS",
            Self::SpawnPoints => "ID_STUFFMENU_CATEGORY_SPAWNPOINTS",
        }
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
        let categories: &[MapCategory] = match self.category {
            MapCategory::Root => &[MapCategory::Locations],
            MapCategory::Locations => &[
                MapCategory::Landmarks,
                MapCategory::Plazas,
                MapCategory::Skateparks,
                MapCategory::SpawnPoints,
            ],
            _ => &[],
        };
        if !categories.is_empty() {
            return categories
                .iter()
                .filter_map(|&category| {
                    let count = locations
                        .iter()
                        .filter(|location| {
                            category == MapCategory::Locations || location.category == category
                        })
                        .count();
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
}

impl MapMenuController {
    #[must_use]
    pub fn snapshot(&self) -> MapMenuSnapshot {
        self.state.clone()
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
            self.state.selected = if forward {
                (self.state.selected + 1).min(rows.len() - 1)
            } else {
                self.state.selected.saturating_sub(1)
            };
        }
        if input.select {
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
