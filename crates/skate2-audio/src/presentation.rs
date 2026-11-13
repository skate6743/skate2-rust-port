//! `Sk8::AudioPresentation` for the skater: the `PhysOut` values `FillSkaterData` 0x827f4c48 reads
//! (supplied by the host as a [`PhysOutView`]) and the `SkaterData` it builds. The bit words of the
//! native structure are separate fields here; their widths are kept where the native truncates.

use crate::scorables::scorable_id_string;
use crate::vault::{class, Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};

/// `eAudioMaterialType_Max`.
pub const MATERIAL_MAX: u32 = 132;
/// `eAudioMaterialType_Max_Mappable_Surface`.
pub const MATERIAL_MAX_MAPPABLE: u32 = 92;

/// `Sk8::Physics::SkeletonCollisionInfoAudio`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CollisionInfo {
    pub contact_force: [f32; 8],
    pub slide_speed: [f32; 8],
    pub audio_surface_type: [u32; 8],
    pub is_groin_contact: bool,
    pub is_face_contact: bool,
    pub vehicle_impact: f32,
    pub skater_impact: f32,
    pub pedestrian_impact: f32,
}

/// The `PhysOut` fields `FillSkaterData` reads, named after their native members.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhysOutView {
    /// `PhysOut_SystemReckoning`: `mCentreOfMass`, `mCentreOfMassVelocity`.
    pub centre_of_mass: Vec3,
    pub centre_of_mass_velocity: Vec3,
    /// `PhysOut_Skeleton`.
    pub hips_mat_y: Vec3,
    pub foot_l_vel_deck_space: Vec3,
    pub foot_r_vel_deck_space: Vec3,
    pub foot_left_vel: Vec3,
    pub foot_right_vel: Vec3,
    pub torso_angular_vel: Vec3,
    pub right_foot_angular_vel: Vec3,
    pub left_foot_angular_vel: Vec3,
    pub absorption_compression: f32,
    /// `mAverageLeftLegSpeedLocal`, right leg, left arm, right arm.
    pub average_limb_speed: [f32; 4],
    pub is_wipeout_over: bool,
    /// PC port: a Skate 3 physics darkcatch (TU3 `PhysOut_State` +0x50).
    pub skate3_dark_catch: bool,
    /// PC port: a Skate 3 physics drop-in's tail locking on the edge and letting go (TU3
    /// `PhysOut_Intents` +0x3D and +0x3E).
    pub skate3_drop_in_lock: bool,
    pub skate3_drop_in_release: bool,
    /// PC port: the skater runs Skate 3 physics.
    pub skate3_physics: bool,
    pub is_left_foot_on_deck: bool,
    pub is_right_foot_on_deck: bool,
    pub is_grabbing_left: bool,
    pub is_grabbing_right: bool,
    /// `PhysOut_SkateboardReckoning`: `mEffectiveTransform` row Y, `mDeckNoseDir`,
    /// `mDeckPosition`, `mWheelPos`.
    pub effective_transform_y: Vec3,
    pub deck_nose_dir: Vec3,
    pub deck_position: Vec3,
    pub wheel_pos: [Vec3; 4],
    /// `PhysOut_SkateboardMotion`.
    pub deck_velocity: Vec3,
    pub skateboard_surface_speed: f32,
    pub deck_tilt: f32,
    pub wheel_hardness: f32,
    /// `PhysOut_Air`.
    pub time_in_known_air: f32,
    pub traj_collision_time: f32,
    pub jump_height: f32,
    pub foot_plant_surface_id: u32,
    pub hand_plant_surface_id: u32,
    pub is_skateboard_airborne: bool,
    pub is_starting_to_jump: bool,
    pub is_foot_planting: bool,
    pub is_left_foot_planting: bool,
    pub is_right_foot_planting: bool,
    pub hand_plant_grabbing: bool,
    /// `PhysOut_Collision`.
    pub num_wheels_in_contact: i32,
    pub front_truck_audio_surface_type: u32,
    pub back_truck_audio_surface_type: u32,
    pub deck_audio_surface_type: u32,
    pub deck_slide_speed: f32,
    pub wheel_audio_surface_type: [u32; 4],
    pub wheel_surface_pattern: [u32; 4],
    pub is_front_truck_in_contact: bool,
    pub is_back_truck_in_contact: bool,
    pub is_deck_in_contact: bool,
    /// `PhysOut_State`.
    pub physics_state: i32,
    pub is_foot_braking: bool,
    pub is_manual_braking: bool,
    pub is_physically_pushing: bool,
    pub is_generally_pushing: bool,
    pub is_generally_pushing_right_foot: bool,
    pub is_wiping_out: bool,
    pub is_manual: bool,
    pub is_reverting: bool,
    /// `PhysOut_Ground`: `mTurn`, `mGroundUpVector`.
    pub turn: f32,
    pub ground_up_vector: Vec3,
    /// `PhysOut_Audio`.
    pub deck_local_xyz_angular_speed: Vec3,
    pub wheel_slide: f32,
    pub jump_fraction: f32,
    pub grind_type: i32,
    pub grind_impact_speed: f32,
    pub averaged_deck_contact_force: f32,
    pub grind_audio_surface_type: u32,
    pub off_board_jump_landing: i32,
    pub hippy_jump_strength: i32,
    pub wheel_impact_strength: [f32; 4],
    pub wheel_landed: [bool; 4],
    pub averaged_skeleton_collision_info: CollisionInfo,
    pub is_grinding: bool,
    pub is_on_stairs: bool,
    pub is_walking_up: bool,
    pub is_walking_down: bool,
    /// `PhysOut_Animation::mPushMongo`.
    pub push_mongo: bool,
    /// `PhysOut_Scoring2`.
    pub line_score: i32,
    pub wipedout_trick_score: i32,
    pub banked_point_change_this_frame: i32,
    pub audio_banked_point_change: i32,
    pub raw_scorable_id: i32,
    /// The scorable's (AudioTrickType, AudioTrickFinishType, AudioIsTrickWithScrape) when the
    /// host resolves it from another Vault (Skate 3 physics); otherwise read from this Vault.
    pub scorable_audio: Option<(i32, i32, bool)>,
    /// PC port: the raw scorable exists only in the linked Skate 3 data.
    pub skate3_only_scorable: bool,
    /// `PhysOut_OffBoard`.
    pub left_foot_surface_id: u32,
    pub right_foot_surface_id: u32,
    pub is_grabbing_object: bool,
    pub left_foot_down: bool,
    pub right_foot_down: bool,
    pub is_unmounting: bool,
    pub is_air_dismount: bool,
    pub is_holding_skateboard: bool,
    pub is_jumping: bool,
    pub is_starting_standing_jump: bool,
}

/// `Sk8::AudioPresentation::SkaterData` (512 bytes natively).
#[derive(Debug, Clone, Default, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct SkaterData {
    pub skater_pos: Vec3,
    pub skater_look: Vec3,
    pub skater_vel: Vec3,
    pub board_pos: Vec3,
    pub board_look: Vec3,
    pub board_vel: Vec3,
    pub grind_type: i32,
    pub turn: f32,
    pub board_speed: f32,
    pub player_speed: f32,
    pub grind_impact: f32,
    pub wheel_slide: f32,
    pub time_in_air: f32,
    pub time_to_land: f32,
    pub jump_height: f32,
    pub jump_fraction: f32,
    pub deck_tilt: f32,
    pub player_angular_vel: f32,
    pub limb_speed: f32,
    // +0x94
    pub is_reverting: bool,
    pub is_grabbing_left: bool,
    pub is_grabbing_right: bool,
    pub wheel_hardness: bool,
    pub is_wipe_out_over: bool,
    pub is_wiping_out: bool,
    /// TU3 SkaterData +0x98 bit 0x20 (`SFXCTL_PlayerPhysics` +0x301).
    pub skate3_dark_catch: bool,
    /// TU3 SkaterData +0xA8 bit 0 and +0xAC bit 31 (`FillSkaterData` 0x827a2c0c, 0x827a2c1c):
    /// the drop-in's lock and release (`SFXCTL_PlayerPhysics` +0x329, +0x32A).
    pub skate3_drop_in_lock: bool,
    pub skate3_drop_in_release: bool,
    /// PC port: the trick is a Skate 3-only scorable, whose audio types name TU3's sounds.
    pub skate3_only_trick: bool,
    /// PC port: the skater runs Skate 3 physics.
    pub skate3_physics: bool,
    pub right_foot_on_deck: bool,
    pub left_foot_on_deck: bool,
    pub deck_in_contact: bool,
    pub back_truck_in_contact: bool,
    pub front_truck_in_contact: bool,
    /// `flLevel`, `frLevel`, `rlLevel`, `rrLevel` (2 bits).
    pub wheel_level: [u32; 4],
    /// `flLanded` .. `rrLanded`.
    pub wheel_landed: [bool; 4],
    pub is_trick_with_scrape: bool,
    pub is_performing_trick: bool,
    pub is_rail_grind: bool,
    pub is_manual: bool,
    pub manual_breaking: bool,
    pub foot_breaking: bool,
    pub is_generally_pushing: bool,
    pub is_pushing: bool,
    pub in_air: bool,
    // +0x98
    pub respawn_seen: bool,
    pub wipeout_seen: bool,
    pub is_celebrating: bool,
    pub can_see_player: bool,
    pub is_push_mongo: bool,
    pub is_hand_plant: bool,
    pub is_grabbing: bool,
    pub is_skitching: bool,
    pub character: u32,
    pub player_index: u32,
    pub num_wheels_on_ground: u32,
    pub is_hippy_jump: bool,
    pub is_jump: bool,
    pub terrain: u32,
    pub is_left_foot_touching_ground: bool,
    pub is_right_foot_touching_ground: bool,
    pub off_board: bool,
    pub ghosted: bool,
    pub local: bool,
    // +0x9C
    pub big_trick_score: u32,
    pub big_trick_completed: bool,
    pub big_trick_seen_score: u32,
    // +0xA0
    pub hippy_strength: u32,
    pub jump_landed: u32,
    pub board_slide: u32,
    pub is_foot_planting: bool,
    pub is_avoiding_by_turning: bool,
    pub is_avoiding_by_braking: bool,
    pub big_trick_seen: bool,
    pub respawn_seen_actor_character: u32,
    pub wipeout_seen_actor_character: u32,
    pub big_trick_seen_actor_character: u32,
    // +0xA4
    pub is_air_dismount: bool,
    pub score: u32,
    pub announcer_score: u32,
    pub is_generally_pushing_right_foot: bool,
    // +0xA8
    pub is_dismounting: bool,
    pub is_holding_board: bool,
    pub line_score: u32,
    pub wipedout_trick_score: u32,
    pub trick_type: i32,
    pub trick_finish_type: i32,
    pub right_foot_surface_id_off_board: u16,
    pub left_foot_surface_id_off_board: u16,
    pub wheel_pos: [Vec3; 4],
    pub deck_local_xyz_angular_speed: Vec3,
    pub left_foot_deck_y_speed: f32,
    pub right_foot_deck_y_speed: f32,
    pub left_foot_slide_deck_speed: f32,
    pub left_foot_slide_world_speed: f32,
    pub right_foot_slide_world_speed: f32,
    pub right_foot_slide_deck_speed: f32,
    pub right_foot_angular_y_world_speed: f32,
    pub left_foot_angular_y_world_speed: f32,
    pub skeleton_collision_info: CollisionInfo,
    pub wheel_material: [u32; 4],
    pub surface_pattern: [u32; 4],
    pub front_truck_material: u32,
    pub back_truck_material: u32,
    pub deck_material: u32,
    pub deck_slide_speed: f32,
    pub deck_contact_force: f32,
    pub board_up: Vec3,
    pub grind_material: u32,
    pub absorption_compression: f32,
    pub hand_plant_surface: u16,
}

/// `AudioSystem::GetAudioMaterial` 0x824e67c0: `tAudioSurfaceType` 0 is undefined.
#[must_use]
pub fn audio_material(surface: u32) -> u32 {
    if surface == 0 {
        return MATERIAL_MAX;
    }
    let material = surface.wrapping_sub(1);
    if material > 132 { MATERIAL_MAX } else { material }
}

/// `aud_contacts` "default", `AudioPresentation::mContactsAttrib`.
const CONTACTS_DEFAULT: Collection = Collection::new(class::AUD_CONTACTS, 0xD7ED_BD36_2D7D_2152);
/// The scorable tuning class `FillSkaterData` looks scorable names up in.
const SCORABLE_CLASS: u64 = 0x6918_4699_84A8_C596;

/// `AudioPresentation::ConvertToImpactLevelFromForce` 0x827ef130.
fn impact_level_from_force(vault: &Vault, value: f32) -> u32 {
    if value >= vault.f32(CONTACTS_DEFAULT, 0x1578_F5AD_78C9_A844) {
        return 2;
    }
    u32::from(value >= vault.f32(CONTACTS_DEFAULT, 0x63B9_250B_E57D_A556))
}

/// `fabs`.
fn abs(value: f32) -> f32 {
    f32::from_bits(value.to_bits() & 0x7FFF_FFFF)
}

/// `fsel(|a| - |b|, |a|, |b|)`.
fn dominant(a: f32, b: f32) -> f32 {
    let (a, b) = (abs(a), abs(b));
    if a - b >= 0.0 { a } else { b }
}

/// `AudioPresentation::DoSimSubmit` 0x827fba0c: `GlobalData`'s `timesOneFive`, `timesTwo` and
/// `timesThree` from the local skater's `PhysOut_Scoring2::mMomentumMeterMultiplier`, all clear
/// without a local skater.
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn multiplier_flags(momentum_meter_multiplier: Option<f32>) -> [bool; 3] {
    let Some(multiplier) = momentum_meter_multiplier else { return [false; 3] };
    if !(multiplier < 3.0) {
        [false, false, true]
    } else if !(multiplier < 2.0) {
        [false, true, false]
    } else if !(multiplier < 1.5) {
        [true, false, false]
    } else {
        [false; 3]
    }
}

/// `AudioPresentation::FillSkaterData` 0x827f4c48 for one skater (`DoSimSubmit` clears the record
/// first). `player_id` is `Actor::GetPlayerID`; the native's player-index-8 substitution and the
/// non-local seen/avoidance flags (from `AIOut`) are not produced.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn fill_skater_data(
    view: &PhysOutView,
    vault: &Vault,
    is_local: bool,
    player_id: i32,
    can_see_player: bool,
    ai_audio: Option<&[u32; 10]>,
) -> SkaterData {
    let mut data = SkaterData { local: is_local, player_index: (player_id as u32) & 0xF, ..SkaterData::default() };
    data.is_jump = view.is_jumping || view.is_starting_standing_jump;
    data.off_board = (500..600).contains(&view.physics_state);
    data.skater_pos = view.centre_of_mass;
    data.skater_look = view.hips_mat_y;
    data.skater_vel = view.centre_of_mass_velocity;
    data.board_pos = view.deck_position;
    data.board_look = view.deck_nose_dir;
    data.board_vel = view.deck_velocity;
    data.num_wheels_on_ground = (view.num_wheels_in_contact as u32) & 7;
    data.is_pushing = view.is_physically_pushing;
    data.is_generally_pushing = view.is_generally_pushing;
    data.is_generally_pushing_right_foot = view.is_generally_pushing_right_foot;
    data.board_speed = view.skateboard_surface_speed;
    data.player_speed = abs(vmx::length(view.centre_of_mass_velocity));
    data.deck_tilt = view.deck_tilt;
    data.turn = view.turn;
    data.in_air = view.is_skateboard_airborne;
    data.foot_breaking = view.is_foot_braking;
    data.manual_breaking = view.is_manual_braking;
    data.is_manual = view.is_manual;
    data.is_rail_grind = view.is_grinding;
    data.grind_type = view.grind_type;
    data.grind_impact = view.grind_impact_speed;
    data.wheel_slide = view.wheel_slide;
    data.time_in_air = view.time_in_known_air;
    data.time_to_land = view.traj_collision_time;
    data.jump_height = view.jump_height;
    data.player_angular_vel = vmx::length(view.torso_angular_vel);
    let [left_leg, right_leg, left_arm, right_arm] = view.average_limb_speed;
    data.limb_speed = (((right_arm + left_arm) + right_leg) + left_leg) * 0.25;
    data.trick_type = -1;
    data.trick_finish_type = -1;
    let id = view.raw_scorable_id;
    if id != -1 {
        data.is_performing_trick = true;
        data.is_hippy_jump = id == 234;
        data.skate3_only_trick = view.skate3_only_scorable;
        if let Some((trick, finish, scrape)) = view.scorable_audio {
            data.trick_type = trick;
            data.trick_finish_type = finish;
            data.is_trick_with_scrape = scrape;
        } else {
            let key = skate2_formats_key(scorable_id_string(id));
            let collection = Collection::new(SCORABLE_CLASS, key);
            if vault.has_collection(collection) {
                data.trick_type = vault.layout_word(collection, 41) as i32;
                data.trick_finish_type = vault.layout_word(collection, 43) as i32;
                data.is_trick_with_scrape = vault.layout_word(collection, 44) >> 24 != 0;
            }
        }
    }
    data.wheel_pos = view.wheel_pos;
    for wheel in 0..4 {
        data.wheel_landed[wheel] = view.wheel_landed[wheel];
        if view.wheel_landed[wheel] {
            data.wheel_level[wheel] = impact_level_from_force(vault, view.wheel_impact_strength[wheel]) & 3;
        }
    }
    data.jump_fraction = if view.is_starting_to_jump { view.jump_fraction } else { -1.0 };
    data.deck_local_xyz_angular_speed = view.deck_local_xyz_angular_speed;
    data.skeleton_collision_info = view.averaged_skeleton_collision_info;
    data.is_wiping_out = view.is_wiping_out;
    data.is_wipe_out_over = view.is_wipeout_over;
    data.skate3_dark_catch = view.skate3_dark_catch;
    data.skate3_drop_in_lock = view.skate3_drop_in_lock;
    data.skate3_drop_in_release = view.skate3_drop_in_release;
    data.skate3_physics = view.skate3_physics;
    data.front_truck_in_contact = view.is_front_truck_in_contact;
    data.back_truck_in_contact = view.is_back_truck_in_contact;
    data.deck_in_contact = view.is_deck_in_contact;
    for wheel in 0..4 {
        data.wheel_material[wheel] = audio_material(view.wheel_audio_surface_type[wheel]);
        data.surface_pattern[wheel] = view.wheel_surface_pattern[wheel];
    }
    data.front_truck_material = audio_material(view.front_truck_audio_surface_type);
    data.back_truck_material = audio_material(view.back_truck_audio_surface_type);
    data.deck_material = audio_material(view.deck_audio_surface_type);
    data.grind_material = audio_material(view.grind_audio_surface_type);
    data.deck_slide_speed = view.deck_slide_speed;
    data.deck_contact_force = view.averaged_deck_contact_force;
    data.left_foot_on_deck = view.is_left_foot_on_deck;
    data.right_foot_on_deck = view.is_right_foot_on_deck;
    data.left_foot_deck_y_speed = abs(view.foot_l_vel_deck_space[1]);
    data.right_foot_deck_y_speed = abs(view.foot_r_vel_deck_space[1]);
    data.left_foot_slide_deck_speed = dominant(view.foot_l_vel_deck_space[0], view.foot_l_vel_deck_space[2]);
    data.right_foot_slide_deck_speed = dominant(view.foot_r_vel_deck_space[0], view.foot_r_vel_deck_space[2]);
    data.right_foot_slide_world_speed = dominant(view.foot_right_vel[0], view.foot_right_vel[2]);
    data.left_foot_slide_world_speed = dominant(view.foot_left_vel[0], view.foot_left_vel[2]);
    data.right_foot_angular_y_world_speed = abs(view.right_foot_angular_vel[1]);
    data.left_foot_angular_y_world_speed = abs(view.left_foot_angular_vel[1]);
    data.is_right_foot_touching_ground = view.right_foot_down || view.is_right_foot_planting;
    data.is_left_foot_touching_ground = view.left_foot_down || view.is_left_foot_planting;
    data.right_foot_surface_id_off_board = view.right_foot_surface_id as u16;
    data.left_foot_surface_id_off_board = view.left_foot_surface_id as u16;
    data.is_foot_planting = view.is_foot_planting;
    if data.is_foot_planting {
        data.right_foot_surface_id_off_board = view.foot_plant_surface_id as u16;
        data.left_foot_surface_id_off_board = view.foot_plant_surface_id as u16;
    }
    data.terrain = if view.is_walking_up {
        if view.is_on_stairs { 2 } else { 4 }
    } else if view.is_walking_down {
        if view.is_on_stairs { 3 } else { 5 }
    } else {
        1
    };
    data.jump_landed = (view.off_board_jump_landing as u32) & 7;
    data.hippy_strength = (view.hippy_jump_strength as u32) & 3;
    data.is_holding_board = view.is_holding_skateboard;
    data.is_dismounting = view.is_unmounting;
    data.is_air_dismount = view.is_air_dismount;
    data.board_up = view.effective_transform_y;
    data.wheel_hardness = view.wheel_hardness < 0.5;
    data.is_grabbing_left = view.is_grabbing_left;
    data.is_grabbing_right = view.is_grabbing_right;
    data.is_reverting = view.is_reverting;
    if view.banked_point_change_this_frame > 0 {
        data.score = (view.banked_point_change_this_frame as u32) & 0x7FFF;
    }
    data.announcer_score = (view.audio_banked_point_change as u32) & 0x7FFF;
    data.line_score = (view.line_score as u32) & 0x7FFF;
    data.wipedout_trick_score = (view.wipedout_trick_score as u32) & 0x7FFF;
    data.is_push_mongo = view.push_mongo;
    data.is_skitching = view.physics_state == 104;
    data.is_grabbing = is_local && view.is_grabbing_object;
    if view.hand_plant_grabbing {
        data.is_hand_plant = true;
        data.hand_plant_surface = view.hand_plant_surface_id as u16;
    }
    if (data.is_wiping_out || data.off_board) && data.deck_in_contact && data.deck_material < MATERIAL_MAX_MAPPABLE {
        let up = vmx::dot3(data.board_up, view.ground_up_vector);
        if up < f32::from_bits(0xbf66_6666) {
            data.board_slide = 1;
        } else if up < f32::from_bits(0x3dcc_cccd) && up > f32::from_bits(0xbdcc_cccd) {
            data.board_slide = 2;
        }
    }
    data.can_see_player = can_see_player;
    data.absorption_compression = view.absorption_compression;
    if let Some(words) = ai_audio {
        let flags = words[9];
        data.is_avoiding_by_braking = flags >> 31 != 0;
        data.is_avoiding_by_turning = flags >> 30 & 1 != 0;
        data.is_celebrating = flags >> 29 & 1 != 0;
        data.wipeout_seen = flags >> 28 & 1 != 0;
        if data.wipeout_seen {
            data.wipeout_seen_actor_character = words[2] & 0x7F;
        }
        data.respawn_seen = flags >> 27 & 1 != 0;
        if data.respawn_seen {
            data.respawn_seen_actor_character = words[4] & 0x7F;
        }
        data.big_trick_seen = flags >> 26 & 1 != 0;
        if data.big_trick_seen {
            data.big_trick_seen_score = words[6] & 0x7FFF;
            data.big_trick_seen_actor_character = words[8] & 0x7F;
        }
        data.big_trick_completed = flags >> 25 & 1 != 0;
        if data.big_trick_completed {
            data.big_trick_score = words[7] & 0xFFFF;
        }
    }
    data
}

/// `Attrib::StringToLowerCaseKey`.
fn skate2_formats_key(name: &str) -> u64 {
    skate2_formats::attrib::string_hash64(&name.to_ascii_lowercase())
}

impl SkaterData {
    /// The record `DoSimSubmit` publishes when no skater exists (all zero).
    #[must_use]
    pub fn empty() -> Self {
        Self { skater_pos: ZERO, ..Self::default() }
    }
}
