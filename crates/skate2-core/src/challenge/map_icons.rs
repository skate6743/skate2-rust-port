//! The map's and the HUD minimap's icons: `Sk8::FE::MapIconManager` (`mSimIcons`, the icons in
//! the order they are drawn), `Sk8::BE::MapBackEndManager`'s icon lists, and the challenge side
//! that keeps them: `Sk8::Challenge::cChallengeMiniMapManager` (registered twice as a
//! sub-controller, as `MiniMapManager` and `ChallengeMiniMapManager`; the second pass of each hook
//! leaves the same icons), the presentation director's `mMapIcons` and
//! `Sk8::Challenge::cSignPostPinchMan`, whose pinch points pulse.
//!
//! Between challenges `ReevaluateActivityPrereqs` gives every available challenge whose map icon
//! prerequisites are complete an icon at its location; a main challenge removes them all
//! (`RunMainActivity`) and its presentation adds its own.

use super::{
    attrib_lua::{ObjectKind, Pointee, attrib_location},
    binding::{get_int, get_int_or, get_string},
    keys::{class, hash_value32},
    lua::{Lua, LuaResult, NativeCall, Value},
    signpost, signpost_progress,
    system::World,
};
use std::collections::BTreeMap;

/// Attribute keys the icons read.
mod field {
    /// `fe_map_icons.maximumRange`.
    pub const MAXIMUM_RANGE: u64 = 0x2cf2_5b33_b439_ddcd;
    /// `challenge_global_data.ChallengeIcon`, `ChallengeIcon2` and `ChallengeIcon3`
    /// (`tChallengeIconDefinition`, read as a `RefSpec` to `fe_map_icons`).
    pub const CHALLENGE_ICON: u64 = 0xe616_acdf_5a36_bfac;
    pub const CHALLENGE_ICON2: u64 = 0x41f9_7fa8_eca2_568d;
    pub const CHALLENGE_ICON3: u64 = 0x3f37_0601_0578_9b18;
    /// `challenge_global_data.ChallengeType`.
    pub const CHALLENGE_TYPE: u64 = 0x02b8_28cc_db47_85e4;
    /// `challenge_global_data.Global`.
    pub const GLOBAL: u64 = 0x122d_b7b0_f026_dbec;
    /// `challenge_global_data.Location`.
    pub const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
    /// The `challenge_global_data` `tLocationID` `AddChallengeIcon` reads before `Location`.
    pub const ICON_LOCATION: u64 = 0x0717_c5a8_5c96_fd90;
    /// `challenge_local_data.ServiceType`.
    pub const SERVICE_TYPE: u64 = 0x1cb8_71a3_9c4b_0ef0;
    /// The `challenge_local_data` flag `RunMainActivity` gives `ShowAIPlayers`.
    pub const SHOW_AI_PLAYERS: u64 = 0xb0d9_b6b9_16d2_23e6;
    /// The `challenge_local_data` challenges that must all be complete before
    /// `ReevaluateActivityPrereqs` gives an available challenge its icon.
    pub const ICON_PREREQUISITES: u64 = 0xe14d_ecee_593e_36d4;
    /// `challenge_objective.VisualIndicators` (0x50-byte entries: type word, `tLocationID` at
    /// +0x18).
    pub const VISUAL_INDICATORS: u64 = 0x7ae8_fef7_7a54_bb91;
    /// `challenges.PointRequirement`.
    pub const POINT_REQUIREMENT: u64 = 0x0c0c_fefb_9fc6_eb73;
    /// `challenges.Name`.
    pub const NAME: u64 = 0x942a_b8ae_e4b4_14ed;
    /// `fe_location_group.Location` and `fe_location_group.Icon`.
    pub const GROUP_LOCATIONS: u64 = 0xf1c6_d5fa_4a23_951e;
    pub const GROUP_ICON: u64 = 0x7862_55b8_ff42_6a3d;
    /// `progression_pinchpoints` and `progression_lists`: `Challenges`.
    pub const CHALLENGES: u64 = 0xec1c_213b_bdbf_1a68;
}

/// `Attrib::Gen::fe_location_group`.
const FE_LOCATION_GROUP: u64 = 0xd897_e7db_651e_58dc;
/// The `fe_map_icons` `AddLocationIcon` uses when the group is absent.
const LOCATION_ICON_FALLBACK: u64 = 0x310d_fde7_1016_31ac;
/// `fe_map_icons`: `mini_skater` (`UpdatePlayerIcons`) and `mini_bigblack`
/// (`EnableBigBlackIcon`).
const PLAYER_ICON: u64 = 0x34b6_4d03_69e3_e3da;
const BIG_BLACK_ICON: u64 = 0xb9e1_d0db_dcb2_9d7e;
/// The `fe_map_icons` (`mini_teleport`) `AddVisualIndicatorIcon` uses.
const VISUAL_INDICATOR_ICON: u64 = 0x6ae8_ebac_95cc_3108;
/// `key_map_general`, `key_map_plazas`, `key_map_skatepark` and `key_map_spawnpoints`
/// (`0x82045120..0x82045138`).
pub const LOCATION_GROUPS: [u64; 4] = [
    0x0229_d702_2a15_e01e,
    0x10b1_6dc9_c4fe_004f,
    0x1757_7ab7_dec3_54e2,
    0xb632_afae_7d12_0e40,
];

/// `eChallengeTypes` the icons test.
mod challenge_type {
    pub const OWN_THE_SPOT: i32 = 9;
    pub const MAP_ONLY: i32 = 30;
    pub const SERVICE: i32 = 34;
    pub const CREATE_A_SPOT: i32 = 35;
    pub const SPOT_FILM: i32 = 40;
    pub const NO_ICON: i32 = 44;
    pub const PROPERTY: i32 = 45;
}
/// `eServiceType_Draining`.
const SERVICE_DRAINING: i32 = 2;
/// `eGlobalType` 1.
const GLOBAL_ONE: i32 = 1;
/// `PT_Human`.
const PLAYER_TYPE_HUMAN: i32 = 1;

/// `kMapIconScaleDir` (`0x8206a5ec`, 0.02) and `kMapIconScale` (`0x82045104`, 1.0).
pub const SCALE_DIR: f32 = f32::from_bits(0x3ca3_d70a);
pub const SCALE: f32 = 1.0;
/// `kFEMiniMapIconPinchPointMaxScale` (default 1.0) and `kFEMiniMapIconPinchPointMinScale`
/// (default 0.5).
pub const PINCH_POINT_MAX_SCALE: f32 = 1.0;
pub const PINCH_POINT_MIN_SCALE: f32 = 0.5;

/// `Sk8::FE::MapIcon` (0x50 bytes); `id` stands for the icon's pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct MapIcon {
    pub id: u32,
    /// `mChallengeKey`: the challenge (or `fe_locations` collection) the icon marks.
    pub challenge_key: u64,
    /// `mIconKey`: the `fe_map_icons` collection.
    pub icon_key: u64,
    /// `mAssetId`'s name: the icon's `minimap_texture` with ".Texture".
    pub texture: String,
    /// `mPos`: world X/Z.
    pub position: [f32; 2],
    pub rotation: f32,
    pub scale: f32,
    pub fade_at_edge: bool,
    pub pinch_point: bool,
    pub pinch_point_non_pulsing: bool,
    /// `mbVisible`.
    pub visible: bool,
    pub scale_dir: f32,
    pub challenge_map_visible: bool,
    pub minimap_visible: bool,
    /// `mScale_map`.
    pub map_scale: f32,
}

impl MapIcon {
    /// The constructor (`0x82601a50`).
    fn new(id: u32) -> Self {
        Self {
            id,
            challenge_key: 0,
            icon_key: 0,
            texture: String::new(),
            position: [0.0; 2],
            rotation: 0.0,
            scale: 1.0,
            fade_at_edge: true,
            pinch_point: false,
            pinch_point_non_pulsing: true,
            visible: true,
            scale_dir: SCALE_DIR,
            challenge_map_visible: false,
            minimap_visible: true,
            map_scale: 1.0,
        }
    }

    /// `UpdateMiniMap`/`UpdateMap`'s step of a pulsing pinch point (`0x826187b8`, `0x82618728`):
    /// the scale moves by its direction and turns back at the cvar bounds.
    fn pulse(&mut self) {
        if !self.pinch_point || self.pinch_point_non_pulsing {
            return;
        }
        let mut scale = self.scale + self.scale_dir;
        if scale >= PINCH_POINT_MAX_SCALE {
            self.scale_dir = -self.scale_dir;
            scale = PINCH_POINT_MAX_SCALE;
        } else if scale <= PINCH_POINT_MIN_SCALE {
            self.scale_dir = -self.scale_dir;
            scale = PINCH_POINT_MIN_SCALE;
        }
        self.scale = scale;
    }
}

/// `MapIconManager::mSimIcons` with `MapBackEndManager`'s lists.
#[derive(Debug, Default)]
pub struct MapIcons {
    /// `mSimIcons`, in the order `RenderMapIcons`/`RenderMiniMapIcons` draw them.
    pub icons: Vec<MapIcon>,
    /// `MapBackEndManager::mMapIcons`.
    back_end: Vec<u32>,
    /// `MapBackEndManager::mVisualIndicatorIcons`.
    visual_indicators: Vec<u32>,
    /// `MapBackEndManager::mPlayerIcons`.
    players: [Option<u32>; 8],
    /// `MapIconManager::mBigBlackIcon`.
    big_black: Option<u32>,
    /// `MapBackEndManager::mbShowAIPlayers`.
    pub show_ai_players: bool,
    next_id: u32,
}

impl MapIcons {
    #[must_use]
    pub fn icon(&self, id: u32) -> Option<&MapIcon> {
        self.icons.iter().find(|icon| icon.id == id)
    }

    fn icon_mut(&mut self, id: u32) -> Option<&mut MapIcon> {
        self.icons.iter_mut().find(|icon| icon.id == id)
    }

    /// The icons `MapBackEndManager::mMapIcons` holds, in its order.
    pub fn back_end_icons(&self) -> impl Iterator<Item = &MapIcon> {
        self.back_end.iter().filter_map(|&id| self.icon(id))
    }

    /// The `mBigBlackIcon` while it is enabled.
    #[must_use]
    pub const fn big_black_icon(&self) -> Option<u32> {
        self.big_black
    }

    fn push(&mut self, mut icon: MapIcon) -> u32 {
        self.next_id += 1;
        icon.id = self.next_id;
        self.icons.push(icon);
        self.next_id
    }

    /// `MapIconManager::RemoveIcon` (`0x82631290`): the icon leaves `mSimIcons`.
    fn remove_icon(&mut self, id: u32) {
        if let Some(at) = self.icons.iter().position(|icon| icon.id == id) {
            self.icons.remove(at);
        }
    }

    /// `MapBackEndManager::RemoveAllIcons` (`0x8256c300`): every `mMapIcons` and
    /// `mVisualIndicatorIcons` icon.
    pub fn remove_all_icons(&mut self) {
        for id in std::mem::take(&mut self.back_end)
            .into_iter()
            .chain(std::mem::take(&mut self.visual_indicators))
        {
            self.remove_icon(id);
        }
    }

    /// `MapBackEndManager::RemoveChallengeObjectIcon` (`0x82572fa0`): an icon that is still in
    /// `mMapIcons`.
    pub fn remove_challenge_object_icon(&mut self, id: u32) {
        if let Some(at) = self.back_end.iter().position(|&icon| icon == id) {
            self.remove_icon(id);
            self.back_end.remove(at);
        }
    }

    /// `MapBackEndManager::RemoveVisualIndicatorIcon` (`0x82573050`).
    pub fn remove_visual_indicator_icon(&mut self, id: u32) {
        if let Some(at) = self.visual_indicators.iter().position(|&icon| icon == id) {
            self.remove_icon(id);
            self.visual_indicators.remove(at);
        }
    }

    /// `MapBackEndManager::EnlargeIcon` (`0x82565478`): the map-visible `mMapIcons` icon of
    /// the challenge is drawn twice as large on the map, every other one at its own size.
    pub fn enlarge_icon(&mut self, challenge: u64) {
        for &id in &self.back_end {
            if let Some(icon) = self.icons.iter_mut().find(|icon| icon.id == id) {
                icon.map_scale = if icon.challenge_map_visible && icon.challenge_key == challenge {
                    2.0
                } else {
                    1.0
                };
            }
        }
    }

    /// `MapBackEndManager::StuffMenuFilter` (`0x825654d8`): the `mMapIcons` icons of the menu's
    /// items show on the map at their own size, the others hide; the count shown.
    pub fn stuff_menu_filter(&mut self, keys: &[u64]) -> usize {
        let mut shown = 0;
        for &id in &self.back_end {
            if let Some(icon) = self.icons.iter_mut().find(|icon| icon.id == id) {
                icon.challenge_map_visible = keys.contains(&icon.challenge_key);
                if icon.challenge_map_visible {
                    icon.map_scale = 1.0;
                    shown += 1;
                }
            }
        }
        shown
    }

    /// The pinch points' pulse, once per `MapIconManager::UpdateMiniMap` (`0x826187b8`) or
    /// `UpdateMap` (`0x82618728`).
    pub fn pulse(&mut self) {
        for icon in &mut self.icons {
            icon.pulse();
        }
    }

    /// `UpdateMiniMap`: the big black icon follows `ILWManager::GetBigBlackEntity`'s transform
    /// (X/Z) while he exists.
    pub fn set_big_black_position(&mut self, position: [f32; 2]) {
        if let Some(icon) = self.big_black.and_then(|id| self.icon_mut(id)) {
            icon.position = position;
        }
    }
}

/// `cSignPostPinchMan::PinchPointMap`: signpost key to its non-pulsing flag.
#[derive(Debug, Default)]
pub struct PinchPoints {
    map: BTreeMap<u64, bool>,
}

/// `MapIconManager::AddIcon(iconType, pos, activity, completed)` (`0x82634548`): an icon of the
/// `fe_map_icons` collection at `position`. An activity's icon carries its challenge key, and its
/// minimap visibility is its `ChallengeIcon`'s `DisplayOnMiniMap` unless completed; without one the
/// constructor's minimap visibility stays.
fn add_icon(
    world: &mut World,
    icon_key: u64,
    position: [f32; 2],
    activity: Option<usize>,
    completed: bool,
) -> u32 {
    let db = &world.db;
    let mut icon = MapIcon::new(0);
    icon.texture = format!(
        "{}.Texture",
        db.layout_text(class::MAP_ICON, icon_key, 0)
            .unwrap_or_default()
    );
    icon.position = position;
    icon.icon_key = icon_key;
    if let Some(activity) = activity {
        icon.challenge_key = world.activities.activities[activity].key;
        let challenge_icon =
            global_ref(world, activity, field::CHALLENGE_ICON).map_or(0, |(_, key)| key);
        icon.minimap_visible =
            !completed && db.layout_byte(class::MAP_ICON, challenge_icon, 4) != 0;
    }
    world.map_icons.push(icon)
}

/// `MapIconManager::AddIcon(iconType, locationKey, pos)` (`0x82634400`): a location's icon,
/// shown on the minimap as its `fe_map_icons` `DisplayOnMiniMap` says.
fn add_location_map_icon(
    world: &mut World,
    icon_key: u64,
    location: u64,
    position: [f32; 2],
) -> u32 {
    let db = &world.db;
    let mut icon = MapIcon::new(0);
    icon.texture = format!(
        "{}.Texture",
        db.layout_text(class::MAP_ICON, icon_key, 0)
            .unwrap_or_default()
    );
    icon.position = position;
    icon.icon_key = icon_key;
    icon.challenge_key = location;
    icon.minimap_visible = db.layout_byte(class::MAP_ICON, icon_key, 4) != 0;
    world.map_icons.push(icon)
}

fn global_ref(world: &World, activity: usize, key: u64) -> Option<(u64, u64)> {
    let (c, k) = world.activities.activities[activity].global_data?;
    world.db.attribute(c, k, key)?.ref_spec(0)
}

/// `Attrib::Instance(ChallengeIcon).mCollection` is set.
fn has_challenge_icon(world: &World, activity: usize) -> bool {
    global_ref(world, activity, field::CHALLENGE_ICON)
        .is_some_and(|(c, k)| world.db.has_collection(c, k))
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

fn local_attribute(
    world: &World,
    activity: usize,
    key: u64,
) -> Option<super::attrib_db::Attribute<'_>> {
    let (c, k) = world.activities.activities[activity].local_data?;
    world.db.attribute(c, k, key)
}

/// `cLocationManager::GetLocationOrientation(name)` (`0x82a8e3b0`)'s translation as X/Z; an
/// unknown name reads `DefaultLocationOrientation` (the origin).
fn location_xz(world: &World, name: &str) -> [f32; 2] {
    world.location(hash_value32(name)).map_or(
        [world.default_location[0], world.default_location[2]],
        |t| [t[12], t[14]],
    )
}

/// The `vcmpeqfp` test the add functions make: both X and Z equal to zero.
fn at_origin(position: [f32; 2]) -> bool {
    position[0] == 0.0 && position[1] == 0.0
}

/// `cSignPostManager::IsChallengeComplete(key, true)` (`0x825cf328`).
pub(super) fn is_complete(world: &World, challenge: u64) -> bool {
    signpost::signpost_key_for_challenge(world, challenge)
        .is_some_and(|k| world.signpost.is_challenge_complete(k))
}

/// `cSignPostManager::IsChallengeActive(key, true)` (`0x825cf288`).
pub(super) fn is_active(world: &World, challenge: u64) -> bool {
    signpost::signpost_key_for_challenge(world, challenge)
        .is_some_and(|k| world.signpost.is_challenge_active(k))
}

/// `cSignPostChallengeHelper::ConvertSignpostKeyToChallengeKey` (`0x825c4910`): the
/// `challenges` collection named by the progression challenge's `Name`, if it exists.
pub(super) fn challenge_key(world: &World, signpost: u64) -> Option<u64> {
    let name = world
        .db
        .attribute(class::PROGRESSION_CHALLENGES, signpost, field::NAME)?
        .text(0)?;
    let key = super::keys::hash64(name);
    world
        .db
        .has_collection(class::CHALLENGES, key)
        .then_some(key)
}

/// `GetAllActiveChallenges` (`0x825d5768`) and `GetAllCompletedChallenges` (`0x825d5bc0`): the
/// available or completed map's signpost keys in key order, as challenge keys.
fn challenges_of(world: &World, signposts: impl Iterator<Item = u64>) -> Vec<u64> {
    signposts
        .filter_map(|signpost| challenge_key(world, signpost))
        .collect()
}

/// `MapBackEndManager::AddChallengeIcon(activity, completed)` (`0x82577aa8`): the activity's
/// `ChallengeIcon` at its location (the `tLocationID` `0x0717c5a85c96fd90`, else `Location`), none
/// at the origin. A drained pool's service, an owned property, an own-the-spot challenge that
/// reached its point requirement or was killed, and a found film spot use their second or third
/// icon.
pub(crate) fn add_challenge_icon(
    world: &mut World,
    activity: usize,
    completed: bool,
) -> Option<u32> {
    let mut name = global_text(world, activity, field::ICON_LOCATION).unwrap_or_default();
    if name.is_empty() {
        name = global_text(world, activity, field::LOCATION)?;
    }
    let position = location_xz(world, &name);
    if at_origin(position) {
        return None;
    }
    let challenge = world.activities.activities[activity].key;
    let mut icon = global_ref(world, activity, field::CHALLENGE_ICON).map_or(0, |(_, key)| key);
    let mut completed = completed;
    let second = |world: &World| {
        global_ref(world, activity, field::CHALLENGE_ICON2).map_or(0, |(_, key)| key)
    };
    let save =
        |world: &World, name: &str| signpost_progress::challenge_save_data(world, challenge, name);
    match global_i32(world, activity, field::CHALLENGE_TYPE) {
        challenge_type::SERVICE => {
            let service = local_attribute(world, activity, field::SERVICE_TYPE)
                .and_then(|a| a.i32(0))
                .unwrap_or(0);
            if service == SERVICE_DRAINING && is_complete(world, challenge) {
                icon = second(world);
                completed = false;
            }
        }
        challenge_type::PROPERTY => {
            if save(world, "PlayerOwns") != 0 {
                icon = second(world);
            }
        }
        challenge_type::OWN_THE_SPOT => {
            let requirement = world
                .db
                .attribute(class::CHALLENGES, challenge, field::POINT_REQUIREMENT)
                .and_then(|a| a.i32(0))
                .unwrap_or(0);
            if save(world, "ABD Points Earned") >= requirement {
                icon = second(world);
            }
            if save(world, "Killed It") != 0 {
                icon =
                    global_ref(world, activity, field::CHALLENGE_ICON3).map_or(0, |(_, key)| key);
            }
        }
        challenge_type::SPOT_FILM => {
            if save(world, "Spot Found") != 0 {
                icon = second(world);
            }
        }
        _ => {}
    }
    let id = add_icon(world, icon, position, Some(activity), completed);
    world.map_icons.back_end.push(id);
    Some(id)
}

/// `MapBackEndManager::RemoveChallengeIcon(activity)` (`0x82572ec0`): the first `mMapIcons` icon
/// of the activity's challenge.
pub(crate) fn remove_challenge_icon(world: &mut World, activity: usize) {
    let challenge = world.activities.activities[activity].key;
    let icons = &mut world.map_icons;
    let found = icons.back_end.iter().position(|&id| {
        icons
            .icon(id)
            .is_some_and(|icon| icon.challenge_key == challenge)
    });
    if let Some(at) = found {
        let id = icons.back_end.remove(at);
        icons.remove_icon(id);
    }
}

/// The last `fe_map_icons` collection, in `AttribSysUtils::GetCollections` order, whose
/// `minimap_texture` is `name` (the `AddChallengeObjectIcon` search).
fn icon_named(world: &World, name: &str) -> Option<u64> {
    world
        .db
        .table_order(class::MAP_ICON)
        .into_iter()
        .filter(|&key| world.db.layout_text(class::MAP_ICON, key, 0) == Some(name))
        .last()
}

fn add_object_icon(world: &mut World, icon: u64, position: [f32; 2]) -> Option<u32> {
    if at_origin(position) {
        return None;
    }
    let id = add_icon(world, icon, position, None, false);
    world.map_icons.back_end.push(id);
    Some(id)
}

/// `MapBackEndManager::AddChallengeObjectIcon(name, locationName)` (`0x825781f8`): the named
/// icon at `cLocationManager::GetLocationPosition` (`0x82a8e330`), none for an unknown name or a
/// location at the origin.
pub(crate) fn add_challenge_object_icon(
    world: &mut World,
    name: &str,
    location: &str,
) -> Option<u32> {
    let icon = icon_named(world, name)?;
    let position = world.location(hash_value32(location)).map_or(
        [world.default_location[0], world.default_location[2]],
        |t| [t[12], t[14]],
    );
    add_object_icon(world, icon, position)
}

/// `MapBackEndManager::AddChallengeObjectIcon(name, tTriggerInstance)` (`0x82577f80`): the named
/// icon at the trigger volume's centre (`TriggerVolumeUtils::ComputeTriggerVolumeCenter`).
pub(crate) fn add_challenge_object_icon_at(
    world: &mut World,
    name: &str,
    centre: Option<[f32; 3]>,
) -> Option<u32> {
    let icon = icon_named(world, name)?;
    let centre = centre?;
    add_object_icon(world, icon, [centre[0], centre[2]])
}

/// `MapBackEndManager::AddLocationIcon(locationRef, groupKey)` (`0x82577850`): the
/// `fe_locations` collection's icon at its spawn location (layout word 1), the group's `Icon`.
fn add_location_icon(world: &mut World, location: u64, group: u64) -> Option<u32> {
    if !world.db.has_collection(class::FE_LOCATIONS, location) {
        return None;
    }
    let name = world
        .db
        .layout_text(class::FE_LOCATIONS, location, 1)
        .unwrap_or_default()
        .to_owned();
    let position = location_xz(world, &name);
    if at_origin(position) {
        return None;
    }
    let icon = if world.db.has_collection(FE_LOCATION_GROUP, group) {
        world
            .db
            .attribute(FE_LOCATION_GROUP, group, field::GROUP_ICON)
            .and_then(|a| a.ref_spec(0))
            .map_or(0, |(_, key)| key)
    } else {
        LOCATION_ICON_FALLBACK
    };
    let id = add_location_map_icon(world, icon, location, position);
    world.map_icons.back_end.push(id);
    Some(id)
}

/// `MapBackEndManager::AddLocationGroup` (`0x8257cf50`): an icon for each of the group's
/// `Location` entries.
pub(crate) fn add_location_group(world: &mut World, group: u64) {
    let locations: Vec<u64> = world
        .db
        .attribute(FE_LOCATION_GROUP, group, field::GROUP_LOCATIONS)
        .map(|a| {
            (0..a.len())
                .map(|i| a.ref_spec(i).map_or(0, |(_, key)| key))
                .collect()
        })
        .unwrap_or_default();
    for location in locations {
        add_location_icon(world, location, group);
    }
}

/// `MapIconManager::ActivateBigBlackIcon` (`0x82635d48`) / `EnableBigBlackIcon` (`0x82634720`):
/// enabling adds the `mini_bigblack` icon at the origin once, disabling removes it.
pub(crate) fn activate_big_black_icon(world: &mut World, enable: bool) {
    if enable {
        if world.map_icons.big_black.is_none() {
            let id = add_icon(world, BIG_BLACK_ICON, [0.0; 2], None, false);
            world.map_icons.big_black = Some(id);
        }
    } else if let Some(id) = world.map_icons.big_black.take() {
        world.map_icons.remove_icon(id);
    }
}

/// `MapBackEndManager::ShowAIPlayers` (`0x82565578`): the flag, and the existing icon of each
/// skater that is not the local one and whose `PlayersData::mPlayerType` is not `PT_Human`
/// shown or hidden with it.
pub(crate) fn show_ai_players(world: &mut World, show: bool) {
    world.map_icons.show_ai_players = show;
    for (index, slot) in (0..).zip(0..8) {
        let Some(id) = world.map_icons.players[slot] else {
            continue;
        };
        let human = world
            .players
            .as_deref()
            .is_some_and(|players| players.player_type(index) == PLAYER_TYPE_HUMAN);
        if human || world.skaters.get(slot).is_none_or(|frame| frame.local) {
            continue;
        }
        if let Some(icon) = world.map_icons.icon_mut(id) {
            icon.minimap_visible = show;
            icon.visible = show;
        }
    }
}

/// `MapBackEndManager::UpdatePlayerIcons` (`0x825656e0`), each front end update: a skater other
/// than the local one without an icon gets a `mini_skater` icon, shown as the AI-players flag
/// says; a skater's icon follows `Actor::GetPosition` (X/Z); a gone skater's icon is removed.
pub(crate) fn update_player_icons(world: &mut World) {
    for slot in 0..8 {
        let Some(frame) = world.skaters.get(slot) else {
            if let Some(id) = world.map_icons.players[slot].take() {
                world.map_icons.remove_icon(id);
            }
            continue;
        };
        let (local, position) = (frame.local, [frame.position[0], frame.position[2]]);
        if !local && world.map_icons.players[slot].is_none() {
            let id = add_icon(world, PLAYER_ICON, [0.0; 2], None, false);
            let show = world.map_icons.show_ai_players;
            if let Some(icon) = world.map_icons.icon_mut(id) {
                icon.minimap_visible = show;
                icon.visible = show;
            }
            world.map_icons.players[slot] = Some(id);
        }
        if let Some(icon) =
            world.map_icons.players[slot].and_then(|id| world.map_icons.icon_mut(id))
        {
            icon.position = position;
        }
    }
}

/// `MapBackEndManager::AddVisualIndicatorIcon` (`0x82578470`): the `mini_teleport` icon
/// `0x6ae8ebac95cc3108` at the location's position, none at the origin, kept in
/// `mVisualIndicatorIcons`.
fn add_visual_indicator_icon(world: &mut World, location: &str) -> Option<u32> {
    let position = world.location(hash_value32(location)).map_or(
        [world.default_location[0], world.default_location[2]],
        |t| [t[12], t[14]],
    );
    if at_origin(position) {
        return None;
    }
    let id = add_icon(world, VISUAL_INDICATOR_ICON, position, None, false);
    world.map_icons.visual_indicators.push(id);
    Some(id)
}

/// `MapIconManager::UpdateMiniMap` (`0x826187b8`) as `HUD::MiniMap::Update` runs it while the
/// minimap component is bound (`HUDManager2::Update` `0x82cee4f0`), then
/// `FrontEndManager::Update`'s (`0x82640130`) `UpdatePlayerIcons`. The local skater's own icon
/// is not shown on the minimap (`Initialize` `0x82635c00`) and the map draws it itself.
pub(crate) fn update_frontend(world: &mut World) {
    if world.hud.binding("minimap") != super::hud::Binding::Unbound {
        // UpdateMiniMap826187b8: the protector's actual Entity.GetPosition.
        if world.map_icons.big_black_icon().is_some()
            && let Some(position)=world.characters.facts().and_then(|facts|facts.big_black_position()) {
            world.map_icons.set_big_black_position([position[0],position[2]]);
        }
        world.map_icons.pulse();
    }
    update_player_icons(world);
}

// ---- cSignPostPinchMan ----------------------------------------------------------------------

/// `cSignPostPinchMan::CheckPinchPoints` (`0x825dff58`), from `cSignPostManager::Init`,
/// `EvaluateActivities` and `LoadData`: every `progression_pinchpoints` collection
/// (`pinchPointVector`) is checked afresh. A pinch point whose `Challenges` still hold at least
/// its threshold (layout word 3) of incomplete challenges, available or not (`CheckPinchPoint`
/// `0x825dd4b0`), marks each of them, non-pulsing as its layout byte 8 says; a challenge in
/// several keeps non-pulsing once one sets it (`ActivatePinchPoint` `0x825d8750`).
pub(crate) fn check_pinch_points(world: &mut World) {
    let db = std::sync::Arc::clone(&world.db);
    let mut map = BTreeMap::new();
    for &pinch in db.collections(class::PROGRESSION_PINCH_POINTS) {
        let challenges: Vec<u64> = db
            .attribute(class::PROGRESSION_PINCH_POINTS, pinch, field::CHALLENGES)
            .map(|a| {
                (0..a.len())
                    .map(|i| a.ref_spec(i).map_or(0, |(_, key)| key))
                    .collect()
            })
            .unwrap_or_default();
        let incomplete = challenges
            .iter()
            .filter(|&&key| !world.signpost.is_challenge_complete(key))
            .count();
        let threshold = db
            .layout_word(class::PROGRESSION_PINCH_POINTS, pinch, 3)
            .cast_signed();
        if i64::from(threshold) > i64::try_from(incomplete).unwrap_or(i64::MAX) {
            continue;
        }
        let non_pulsing = db.layout_byte(class::PROGRESSION_PINCH_POINTS, pinch, 8) != 0;
        for key in challenges {
            let entry = map.entry(key).or_insert(non_pulsing);
            if !*entry {
                *entry = non_pulsing;
            }
        }
    }
    world.pinch_points.map = map;
}

/// `IsChallengePinchPoint(challengeKey, true)` (`0x825d19a8`).
fn is_pinch_point(world: &World, challenge: u64) -> bool {
    signpost::signpost_key_for_challenge(world, challenge)
        .is_some_and(|key| world.pinch_points.map.contains_key(&key))
}

/// `IsChallengeNonPulsing(challengeKey, true)` (`0x825d1a18`).
fn is_non_pulsing(world: &World, challenge: u64) -> bool {
    signpost::signpost_key_for_challenge(world, challenge)
        .and_then(|key| world.pinch_points.map.get(&key).copied())
        .unwrap_or(false)
}

// ---- cChallengeMiniMapManager ----------------------------------------------------------------

/// `RunMainActivity` (`0x825a2aa8`): every challenge icon goes, and the AI players show as the
/// main challenge's local data asks.
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    world.map_icons.remove_all_icons();
    let show = local_attribute(world, activity, field::SHOW_AI_PLAYERS)
        .and_then(|a| a.bool(0))
        .unwrap_or(false);
    show_ai_players(world, show);
}

/// `CompleteChallenge` (`0x825a2b40`).
pub(crate) fn complete_challenge(world: &mut World) {
    show_ai_players(world, false);
}

/// `ReevaluateActivityPrereqs` (`0x825a5bd0`): every icon goes, then
/// - each available challenge (in signpost key order) that has a `ChallengeIcon`, is not of
///   type 44, and whose `0xe14decee593e36d4` challenges are all complete gets its icon, marked
///   as a pinch point (and non-pulsing) as the pinch points say, at `kMapIconScale` with
///   `kMapIconScaleDir`;
/// - each completed challenge with a `ChallengeIcon` gets a completed icon unless it is of type
///   0, 30, 35 or 44 or `Global` 1, though a type 34 challenge with `ServiceType` 2 always does;
/// - each Create-a-Spot (`CreateASpotDataManager`, not ported) gets its challenge's icon;
/// - with the `slappys` dynamic hull unlocked, the four map location groups' icons.
pub(crate) fn reevaluate_activity_prereqs(world: &mut World) {
    world.map_icons.remove_all_icons();
    let active = challenges_of(
        world,
        world
            .signpost
            .available
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter(),
    );
    for challenge in active {
        let Some(activity) = world.activities.find(challenge) else {
            continue;
        };
        if global_i32(world, activity, field::CHALLENGE_TYPE) == challenge_type::NO_ICON {
            continue;
        }
        if !has_challenge_icon(world, activity) {
            continue;
        }
        let prerequisites: Vec<u64> = local_attribute(world, activity, field::ICON_PREREQUISITES)
            .map(|a| {
                (0..a.len())
                    .map(|i| a.ref_spec(i).map_or(0, |(_, key)| key))
                    .collect()
            })
            .unwrap_or_default();
        if prerequisites.iter().any(|&key| !is_complete(world, key)) {
            continue;
        }
        let pinch_point = is_pinch_point(world, challenge);
        let non_pulsing = is_non_pulsing(world, challenge);
        if let Some(id) = add_challenge_icon(world, activity, false)
            && let Some(icon) = world.map_icons.icon_mut(id)
        {
            icon.pinch_point = pinch_point;
            icon.pinch_point_non_pulsing = non_pulsing;
            icon.scale_dir = SCALE_DIR;
            icon.scale = SCALE;
        }
    }
    let completed = challenges_of(
        world,
        world
            .signpost
            .completed
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter(),
    );
    for challenge in completed {
        let Some(activity) = world.activities.find(challenge) else {
            continue;
        };
        let kind = global_i32(world, activity, field::CHALLENGE_TYPE);
        let excluded = matches!(
            kind,
            challenge_type::NO_ICON | challenge_type::MAP_ONLY | 0 | challenge_type::CREATE_A_SPOT
        ) || global_i32(world, activity, field::GLOBAL) == GLOBAL_ONE;
        if excluded {
            let service = local_attribute(world, activity, field::SERVICE_TYPE)
                .and_then(|a| a.i32(0))
                .unwrap_or(0);
            if kind != challenge_type::SERVICE || service != SERVICE_DRAINING {
                continue;
            }
        }
        if has_challenge_icon(world, activity) {
            add_challenge_icon(world, activity, true);
        }
    }
    if super::dynamic_hull::test_state(world, super::keys::hash64("slappys"), "Unlock")
        == Some(true)
    {
        for group in LOCATION_GROUPS {
            add_location_group(world, group);
        }
    }
}

// ---- cChallengePresentationDirector::mMapIcons ------------------------------------------------

/// `RemoveChallengeMapIcon` (`0x82e06c38`): the icon held under `GetHashValue32(name)` leaves the
/// back end and the director.
pub(crate) fn remove_presentation_icon(world: &mut World, name: &str) {
    if let Some(id) = world.presentation.map_icons.remove(&hash_value32(name)) {
        world.map_icons.remove_challenge_object_icon(id);
    }
}

/// Presentation event 30 (`PlayNextPresentationEvent` `0x82e085d0`): the old icon of the name
/// goes, and the back end's `AddChallengeObjectIcon(name, location)` icon, if any, is held under
/// the name.
pub(crate) fn add_presentation_icon(world: &mut World, name: &str, location: &str) {
    remove_presentation_icon(world, name);
    if let Some(id) = add_challenge_object_icon(world, name, location) {
        world.presentation.map_icons.insert(hash_value32(name), id);
    }
}

/// `RemoveAllChallengeMapIcons` (`0x82e06cb8`), from `ShutdownActivity` and
/// `RemovePresentationGroup`: every icon the director holds.
pub(crate) fn remove_all_presentation_icons(world: &mut World) {
    for id in std::mem::take(&mut world.presentation.map_icons).into_values() {
        world.map_icons.remove_challenge_object_icon(id);
    }
}

// ---- Lua ModuleMap --------------------------------------------------------------------------

type R = LuaResult<Vec<Value>>;

/// A `MapIcon*` as `lua_pushlightuserdata` pushes it, null included.
pub(crate) fn icon_handle(world: &mut World, id: Option<u32>) -> Value {
    Value::LightUserData(id.map_or(0, |id| {
        world.pointers.handle(Pointee::Object {
            kind: ObjectKind::MapIcon,
            id,
        })
    }))
}

/// `Map.AddChallengeIcon(activity)` (`0x828839e0`, also bound as `HideChallengeIcon`):
/// `AddChallengeIcon(activity, false)`.
fn add_challenge_icon_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(activity) = super::attrib_lua::activity_of(lua, call.arg(0)) {
        add_challenge_icon(world, activity, false);
    }
    Ok(Vec::new())
}

/// `Map.ShowChallengeIcon(activity)` (`0x82883a40`): `RemoveChallengeIcon` of the activity's key.
fn show_challenge_icon(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(activity) = super::attrib_lua::activity_of(lua, call.arg(0)) {
        remove_challenge_icon(world, activity);
    }
    Ok(Vec::new())
}

/// `Map.RemoveAllChallengeIcons(activity)` (`0x82883aa0`): reads its argument and does nothing.
fn remove_all_challenge_icons(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(Vec::new())
}

/// `Map.AddMapIcon(name, location)` (`0x8288c288`): the back end's
/// `AddChallengeObjectIcon(name, location)`, as a `lightuserdata`.
fn add_map_icon(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = get_string(call, 0).unwrap_or_default();
    let location = attrib_location(&world.pointers, call.arg(1))
        .and_then(|at| world.db.text_at(at))
        .map(str::to_owned);
    let id = location.and_then(|location| add_challenge_object_icon(world, &name, &location));
    Ok(vec![icon_handle(world, id)])
}

/// `Map.RemoveMapIcon(icon)` (`0x8288c348`, shared with `RemoveRaceObject`): the back end's
/// `RemoveChallengeObjectIcon`.
fn remove_map_icon(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(id) = world.pointers.object(call.arg(0), ObjectKind::MapIcon) {
        world.map_icons.remove_challenge_object_icon(id);
    }
    Ok(Vec::new())
}

/// `Map.AddObjectiveVisualIndicatorToMap(objectiveGroup, objective, indicator = 0)`
/// (`0x8288c3e0`): the objective's `VisualIndicators` entry (`GetObjectiveInfo` `0x82d435d8`);
/// one whose type word is 0 gets `AddVisualIndicatorIcon` of its `tLocationID`, as a
/// `lightuserdata`; nil without entries or for another type.
fn add_objective_visual_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let objective = usize::try_from(get_int(call, 1)).ok();
    let indicator = usize::try_from(get_int_or(call, 2, 0)).unwrap_or(usize::MAX);
    let key = world
        .pointers
        .object(call.arg(0), ObjectKind::ObjectiveGroup)
        .and_then(|group| {
            let group = &world.objectives.groups[world.objectives.group_index(group)?];
            Some(group.objectives.get(objective?)?.key)
        });
    let db = std::sync::Arc::clone(&world.db);
    let Some(entries) =
        key.and_then(|key| db.attribute(class::CHALLENGE_OBJECTIVE, key, field::VISUAL_INDICATORS))
    else {
        return Ok(vec![Value::Nil]);
    };
    if entries.is_empty() {
        return Ok(vec![Value::Nil]);
    }
    let entry = entries.data(indicator);
    if entry
        .and_then(|at| db.bytes(at, 4))
        .is_some_and(|word| word.iter().any(|&b| b != 0))
    {
        return Ok(vec![Value::Nil]);
    }
    let location = entry.and_then(|at| db.text_at(at.offset_by(0x18)));
    let id = location.and_then(|location| add_visual_indicator_icon(world, location));
    Ok(vec![icon_handle(world, id)])
}

/// `Map.RemoveObjectiveVisualIndicatorFromMap(icon)` (`0x8288c570`).
fn remove_objective_visual_indicator(
    _: &mut Lua<World>,
    world: &mut World,
    call: &NativeCall,
) -> R {
    if let Some(id) = world.pointers.object(call.arg(0), ObjectKind::MapIcon) {
        world.map_icons.remove_visual_indicator_icon(id);
    }
    Ok(Vec::new())
}

/// The `Map` natives this module binds.
pub(crate) fn natives() -> Vec<(&'static str, &'static str, super::lua::NativeFn<World>)> {
    vec![
        ("Map", "AddChallengeIcon", add_challenge_icon_native),
        ("Map", "HideChallengeIcon", add_challenge_icon_native),
        ("Map", "ShowChallengeIcon", show_challenge_icon),
        ("Map", "RemoveAllChallengeIcons", remove_all_challenge_icons),
        ("Map", "AddMapIcon", add_map_icon),
        ("Map", "RemoveMapIcon", remove_map_icon),
        (
            "Map",
            "AddObjectiveVisualIndicatorToMap",
            add_objective_visual_indicator,
        ),
        (
            "Map",
            "RemoveObjectiveVisualIndicatorFromMap",
            remove_objective_visual_indicator,
        ),
    ]
}

/// What the HUD minimap draws of the icons (`RenderIconOnScreenMiniMap` `0x8262ea20` takes
/// `mMiniMapVisible` and `mbVisible` icons), with each `fe_map_icons` `maximumRange`.
#[must_use]
pub fn minimap_icons(world: &World) -> Vec<crate::ui::minimap::Icon> {
    world
        .map_icons
        .icons
        .iter()
        .map(|icon| crate::ui::minimap::Icon {
            texture: icon.texture.clone(),
            position: icon.position,
            rotation: icon.rotation,
            scale: icon.scale,
            visible: icon.minimap_visible && icon.visible,
            fade_at_edge: icon.fade_at_edge,
            maximum_range: world
                .db
                .attribute(class::MAP_ICON, icon.icon_key, field::MAXIMUM_RANGE)
                .and_then(|a| a.f32(0))
                .unwrap_or(0.0),
            pinch_point: icon.pinch_point,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pinch_point_pulses_between_the_cvar_scales() {
        let mut icon = MapIcon::new(1);
        icon.pinch_point = true;
        icon.pinch_point_non_pulsing = false;
        icon.pulse();
        // 1.0 + 0.02 reaches kFEMiniMapIconPinchPointMaxScale: back to 1.0, shrinking.
        assert_eq!((icon.scale, icon.scale_dir), (1.0, -SCALE_DIR));
        let mut steps = 1;
        while icon.scale_dir < 0.0 {
            icon.pulse();
            steps += 1;
        }
        assert_eq!(icon.scale, PINCH_POINT_MIN_SCALE);
        assert!((25..=27).contains(&steps), "{steps}");
        icon.pinch_point_non_pulsing = true;
        let before = icon.scale;
        icon.pulse();
        assert_eq!(icon.scale, before);
    }

    #[test]
    fn the_back_end_filter_and_enlarge_touch_only_its_own_icons() {
        let mut icons = MapIcons::default();
        let ids: Vec<u32> = [10, 20, 30]
            .into_iter()
            .map(|key| {
                let mut icon = MapIcon::new(0);
                icon.challenge_key = key;
                icons.push(icon)
            })
            .collect();
        icons.back_end.extend(&ids[..2]);
        assert_eq!(icons.stuff_menu_filter(&[20, 30]), 1);
        let visible: Vec<bool> = icons
            .icons
            .iter()
            .map(|icon| icon.challenge_map_visible)
            .collect();
        assert_eq!(visible, [false, true, false]);
        icons.enlarge_icon(20);
        let scales: Vec<f32> = icons.icons.iter().map(|icon| icon.map_scale).collect();
        assert_eq!(scales, [1.0, 2.0, 1.0]);
        icons.remove_all_icons();
        assert_eq!(
            icons
                .icons
                .iter()
                .map(|icon| icon.challenge_key)
                .collect::<Vec<_>>(),
            [30]
        );
    }
}
