//! `Sk8::Audio::SFXCTL_PlayerPhysics` (800 bytes): copies the skater's `SkaterData` each update,
//! derives the values the skater SFX objects read, and writes the controller's Nicotine inputs.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::pos3d::MicrophoneMan;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap};
use crate::presentation::{CollisionInfo, SkaterData, MATERIAL_MAX};
use crate::vault::{class, Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};

/// `eWheelType`.
pub const WHEEL_HARD: u32 = 0;
pub const WHEEL_SOFT: u32 = 1;
/// `eSk8AudioTricks::eInvalidAudioTrick` and `eFootPlant`.
pub const INVALID_AUDIO_TRICK: i32 = -1;
pub const FOOT_PLANT: i32 = 0x1F;
/// `eBoardSlideType`.
pub const BOARD_SLIDE_NONE: u32 = 0;
pub const BOARD_SLIDE_TOP: u32 = 1;
pub const BOARD_SLIDE_SIDE: u32 = 2;
/// `eAudioMaterialType_Concrete_Rough`.
const MATERIAL_CONCRETE_ROUGH: u32 = 3;
/// `eSk8Characters`: `eSk8Character_Player`, `eSk8Character_Max`.
const CHARACTER_PLAYER: u32 = 0;
const CHARACTER_MAX: u32 = 0x27;
/// `eFootStepTerrain::eFootStep_Flat`, `eOffBoardLandings::eNoLanding`, `eShortHippy`.
const FOOTSTEP_FLAT: u32 = 1;
const NO_LANDING: u32 = 1;
const SHORT_HIPPY: u32 = 0;

/// `aud_contacts` "default", `aud_collisions` "default", and the `aud_general` collection
/// `UpdateNicotine` looks up for the relative speed.
pub const CONTACTS_DEFAULT: Collection = Collection::new(class::AUD_CONTACTS, 0xD7ED_BD36_2D7D_2152);
pub const COLLISIONS_DEFAULT: Collection = Collection::new(class::AUD_COLLISIONS, 0xD7ED_BD36_2D7D_2152);
const GENERAL_RELATIVE_SPEED: Collection = Collection::new(class::AUD_GENERAL, 0x3213_338D_2D2C_817F);

/// What the controller reads from outside its state each update.
#[derive(Debug, Clone, Copy, Default)]
pub struct PhysicsInputs {
    /// `CSTATE_Player::mStateData.local`.
    pub local: bool,
    /// `AudioPresentation::mGameSpeed` and `mIsRewinding`.
    pub game_speed: f32,
    pub is_rewinding: bool,
    /// The local skater's `skaterVel` (`AudioPresentation::mLocal`), when one exists.
    pub local_velocity: Option<Vec3>,
    /// `VDPresentationState::mOutData.bBail`.
    pub visual_bail: bool,
    /// `AudioSystem::GetNPCWheelType`.
    pub npc_wheel_type: u32,
    /// `AudioSystem::GetGrabbedObjectKey` 0x824e69e8: `CSTATEMGR_DynamicObject::GetGrabbedObjectKey`
    /// 0x82550430, the first attached DYNAMICOBJECT state's non-zero key.
    pub grabbed_object_key: u64,
}

/// `Sk8::Audio::SFXCTL_PlayerPhysics`.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct PlayerPhysics {
    pub io: Option<DMixIoId>,
    pub position: Vec3,
    pub prev_position: Vec3,
    pub forward: Vec3,
    pub velocity: Vec3,
    pub prev_velocity: Vec3,
    pub acceleration: Vec3,
    pub board_position: Vec3,
    pub board_forward: Vec3,
    pub board_velocity: Vec3,
    pub grind_type: i32,
    pub num_wheels_on_ground: u32,
    pub turn: f32,
    pub board_speed: f32,
    pub player_speed: f32,
    pub last_player_speed: f32,
    pub game_speed: f32,
    pub is_rewinding: bool,
    pub grind_impact: f32,
    pub wheel_slide: f32,
    pub time_in_air: f32,
    pub time_to_land: f32,
    pub prev_time_in_air: [f32; 4],
    pub jump_height: f32,
    pub deck_tilt: f32,
    pub right_foot_deck_y_speed: f32,
    pub left_foot_deck_y_speed: f32,
    pub right_foot_slide_deck_speed: f32,
    pub left_foot_slide_deck_speed: f32,
    pub left_foot_slide_world_speed: f32,
    pub right_foot_slide_world_speed: f32,
    pub left_foot_angular_y_world_speed: f32,
    pub right_foot_angular_y_world_speed: f32,
    pub off_board_landing: u32,
    pub hippy_jump_strength: u32,
    pub is_holding_board: bool,
    pub is_dismounting: bool,
    pub is_dismount_contact: bool,
    pub was_in_dismount: bool,
    pub cur_time: f32,
    pub dismount_start_time: f32,
    pub is_air_dismount: bool,
    pub distance: f32,
    pub torso_ang_vel: f32,
    pub in_air: bool,
    pub is_pushing_left: bool,
    pub is_pushing_right: bool,
    pub is_pushing_mongo: bool,
    pub push_started: bool,
    pub is_foot_braking: bool,
    pub is_generally_pushing: bool,
    pub is_generally_pushing_right_foot: bool,
    pub is_manual_braking: bool,
    pub is_manual: bool,
    pub is_rail_grind: bool,
    pub was_rail_grind: bool,
    pub is_performing_trick: bool,
    pub is_trick_with_scrape: bool,
    pub trick_type: i32,
    pub trick_done_type: i32,
    pub is_hippy_jump: bool,
    pub wheel_pos: [Vec3; 4],
    pub impact_level: [i32; 4],
    pub landed: [bool; 4],
    pub jump_fraction: f32,
    pub deck_local_xyz_angular_speed: Vec3,
    pub skeleton_collision_info: CollisionInfo,
    pub front_truck_in_contact: bool,
    pub back_truck_in_contact: bool,
    pub deck_in_contact: bool,
    pub left_foot_on_deck: bool,
    pub right_foot_on_deck: bool,
    pub wheel_material: [u32; 4],
    pub surface_pattern: [u32; 4],
    pub front_truck_material: u32,
    pub back_truck_material: u32,
    pub deck_material: u32,
    pub deck_slide_speed: f32,
    pub deck_contact_force: f32,
    pub limb_speed: f32,
    pub is_wiping_out: bool,
    pub is_wipe_out_over: bool,
    /// TU3 `SFXCTL_PlayerPhysics` +0x301: the skater's darkcatch (Skate 3 physics only).
    pub skate3_dark_catch: bool,
    /// PC port: the trick is a Skate 3-only scorable (`SkaterData::skate3_only_trick`).
    pub skate3_only_trick: bool,
    /// PC port: the skater runs Skate 3 physics.
    pub skate3_physics: bool,
    pub wheel_occlusion: f32,
    pub wheel_hardness: u32,
    pub is_grabbing_left: bool,
    pub is_grabbing_right: bool,
    pub is_reverting: bool,
    pub grind_material: u32,
    pub score: u32,
    pub line_score: u32,
    pub character_type: u32,
    pub is_avoiding_by_braking: bool,
    pub is_avoiding_by_turning: bool,
    pub is_celebrating: bool,
    pub wipeout_seen: bool,
    pub respawn_seen: bool,
    pub big_trick_seen: bool,
    pub big_trick_seen_score: u32,
    pub big_trick_seen_actor_character: u32,
    pub wipeout_seen_actor_character: u32,
    pub respawn_seen_actor_character: u32,
    pub big_trick_completed: bool,
    pub big_trick_completed_score: u32,
    pub can_see_player: bool,
    pub compression: f32,
    pub is_off_board: bool,
    pub is_jump: bool,
    pub right_foot_touching_off_board: bool,
    pub left_foot_touching_off_board: bool,
    pub left_foot_off_board_surface: u32,
    pub right_foot_off_board_surface: u32,
    pub right_foot_surface_id_off_board: u16,
    pub left_foot_surface_id_off_board: u16,
    pub terrain: u32,
    pub is_skitching: bool,
    pub grabbing_dmo: bool,
    pub dmo_key: u64,
    pub is_hand_plant: bool,
    pub hand_plant_audio_surface: u32,
    pub is_foot_planting: bool,
    pub board_slide: u32,
    pub last_relative_speed: f32,
}

impl Default for PlayerPhysics {
    fn default() -> Self {
        let mut physics = Self {
            io: None,
            position: ZERO,
            prev_position: ZERO,
            forward: ZERO,
            velocity: ZERO,
            prev_velocity: ZERO,
            acceleration: ZERO,
            board_position: ZERO,
            board_forward: ZERO,
            board_velocity: ZERO,
            grind_type: 0,
            num_wheels_on_ground: 0,
            turn: 0.0,
            board_speed: 0.0,
            player_speed: 0.0,
            last_player_speed: 0.0,
            game_speed: 0.0,
            is_rewinding: false,
            grind_impact: 0.0,
            wheel_slide: 0.0,
            time_in_air: 0.0,
            time_to_land: 0.0,
            prev_time_in_air: [0.0; 4],
            jump_height: 0.0,
            deck_tilt: 0.0,
            right_foot_deck_y_speed: 0.0,
            left_foot_deck_y_speed: 0.0,
            right_foot_slide_deck_speed: 0.0,
            left_foot_slide_deck_speed: 0.0,
            left_foot_slide_world_speed: 0.0,
            right_foot_slide_world_speed: 0.0,
            left_foot_angular_y_world_speed: 0.0,
            right_foot_angular_y_world_speed: 0.0,
            off_board_landing: 0,
            hippy_jump_strength: 0,
            is_holding_board: false,
            is_dismounting: false,
            is_dismount_contact: false,
            was_in_dismount: false,
            cur_time: 0.0,
            dismount_start_time: 0.0,
            is_air_dismount: false,
            distance: 0.0,
            torso_ang_vel: 0.0,
            in_air: false,
            is_pushing_left: false,
            is_pushing_right: false,
            is_pushing_mongo: false,
            push_started: false,
            is_foot_braking: false,
            is_generally_pushing: false,
            is_generally_pushing_right_foot: false,
            is_manual_braking: false,
            is_manual: false,
            is_rail_grind: false,
            was_rail_grind: false,
            is_performing_trick: false,
            is_trick_with_scrape: false,
            trick_type: 0,
            trick_done_type: 0,
            is_hippy_jump: false,
            wheel_pos: [ZERO; 4],
            impact_level: [0; 4],
            landed: [false; 4],
            jump_fraction: 0.0,
            deck_local_xyz_angular_speed: ZERO,
            skeleton_collision_info: CollisionInfo::default(),
            front_truck_in_contact: false,
            back_truck_in_contact: false,
            deck_in_contact: false,
            left_foot_on_deck: false,
            right_foot_on_deck: false,
            wheel_material: [0; 4],
            surface_pattern: [0; 4],
            front_truck_material: 0,
            back_truck_material: 0,
            deck_material: 0,
            deck_slide_speed: 0.0,
            deck_contact_force: 0.0,
            limb_speed: 0.0,
            is_wiping_out: false,
            is_wipe_out_over: false,
            skate3_dark_catch: false,
            skate3_only_trick: false,
            skate3_physics: false,
            wheel_occlusion: 0.0,
            wheel_hardness: 0,
            is_grabbing_left: false,
            is_grabbing_right: false,
            is_reverting: false,
            grind_material: 0,
            score: 0,
            line_score: 0,
            character_type: 0,
            is_avoiding_by_braking: false,
            is_avoiding_by_turning: false,
            is_celebrating: false,
            wipeout_seen: false,
            respawn_seen: false,
            big_trick_seen: false,
            big_trick_seen_score: 0,
            big_trick_seen_actor_character: 0,
            wipeout_seen_actor_character: 0,
            respawn_seen_actor_character: 0,
            big_trick_completed: false,
            big_trick_completed_score: 0,
            can_see_player: false,
            compression: 0.0,
            is_off_board: false,
            is_jump: false,
            right_foot_touching_off_board: false,
            left_foot_touching_off_board: false,
            left_foot_off_board_surface: 0,
            right_foot_off_board_surface: 0,
            right_foot_surface_id_off_board: 0,
            left_foot_surface_id_off_board: 0,
            terrain: 0,
            is_skitching: false,
            grabbing_dmo: false,
            dmo_key: 0,
            is_hand_plant: false,
            hand_plant_audio_surface: 0,
            is_foot_planting: false,
            board_slide: 0,
            last_relative_speed: 0.0,
        };
        physics.reset();
        physics
    }
}

/// `fsel(-x, 0, x)` then `fsel(32767 - x, x, 32767)`, `fctiwz`, and the integer 0..0x7FFF clamp.
fn q15_from(value: f32) -> i32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    let high = if 32767.0 - low >= 0.0 { low } else { 32767.0 };
    fctiwz(high).clamp(0, 0x7FFF)
}

/// `fsel` clamp of a time into 0..1 (`time * 0.5` in `UpdateParams`).
fn clamp_unit(value: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}

/// `Sk8::PointGraphEval::Evaluate` 0x824e2c40.
#[must_use]
pub fn point_graph(x: &[f32], y: &[f32], value: f32) -> f32 {
    let n = x.len();
    if value < x[0] {
        return y[0];
    }
    if !(value < x[n - 1]) {
        return y[n - 1];
    }
    let Some(i) = (1..n).find(|&i| value < x[i]) else { return y[0] };
    let dx = x[i] - x[i - 1];
    if dx <= 0.0 {
        return y[i];
    }
    ((y[i] - y[i - 1]) / dx).mul_add(value - x[i - 1], y[i - 1])
}

impl PlayerPhysics {
    fn set(&self, map: &mut MixMap, index: usize, value: i32) {
        if let Some(io) = self.io {
            map.set_input(io, index, value);
        }
    }

    /// `SFXCTL_PlayerPhysics::Reset` 0x82510538 (also `Detach`).
    pub fn reset(&mut self) {
        self.in_air = false;
        self.is_pushing_right = false;
        self.is_pushing_left = false;
        self.turn = 0.0;
        self.is_pushing_mongo = false;
        self.board_speed = 0.0;
        self.is_foot_braking = false;
        self.player_speed = 0.0;
        self.is_manual_braking = false;
        self.deck_tilt = 0.0;
        self.is_manual = false;
        self.right_foot_deck_y_speed = 0.0;
        self.is_rail_grind = false;
        self.left_foot_deck_y_speed = 0.0;
        self.was_rail_grind = false;
        self.right_foot_slide_deck_speed = 0.0;
        self.is_generally_pushing = false;
        self.left_foot_slide_deck_speed = 0.0;
        self.is_generally_pushing_right_foot = false;
        self.left_foot_slide_world_speed = 0.0;
        self.left_foot_on_deck = false;
        self.right_foot_slide_world_speed = 0.0;
        self.right_foot_on_deck = false;
        self.left_foot_angular_y_world_speed = 0.0;
        self.right_foot_angular_y_world_speed = 0.0;
        self.distance = 0.0;
        self.landed = [true; 4];
        self.prev_time_in_air = [0.0; 4];
        self.impact_level = [0; 4];
        self.jump_fraction = 0.0;
        self.trick_type = INVALID_AUDIO_TRICK;
        self.trick_done_type = INVALID_AUDIO_TRICK;
        self.wheel_material = [MATERIAL_MAX; 4];
        self.surface_pattern = [0; 4];
        self.deck_slide_speed = 0.0;
        self.deck_contact_force = 0.0;
        self.time_to_land = 0.0;
        self.front_truck_material = MATERIAL_MAX;
        self.torso_ang_vel = 0.0;
        self.back_truck_material = MATERIAL_MAX;
        self.wheel_occlusion = 0.0;
        self.deck_material = MATERIAL_MAX;
        self.is_wiping_out = false;
        self.game_speed = 1.0;
        self.is_wipe_out_over = false;
        self.skate3_dark_catch = false;
        self.skate3_only_trick = false;
        self.skate3_physics = false;
        self.last_player_speed = 0.0;
        self.wheel_hardness = WHEEL_HARD;
        self.limb_speed = 0.0;
        self.is_rewinding = false;
        self.cur_time = 0.0;
        self.is_grabbing_left = false;
        self.dismount_start_time = 0.0;
        self.is_grabbing_right = false;
        self.last_relative_speed = 0.0;
        self.is_reverting = false;
        self.is_avoiding_by_braking = false;
        self.is_avoiding_by_turning = false;
        self.is_celebrating = false;
        self.wipeout_seen = false;
        self.wipeout_seen_actor_character = CHARACTER_PLAYER;
        self.respawn_seen = false;
        self.respawn_seen_actor_character = CHARACTER_PLAYER;
        self.big_trick_seen = false;
        self.big_trick_seen_score = 0;
        self.big_trick_seen_actor_character = CHARACTER_PLAYER;
        self.big_trick_completed = false;
        self.big_trick_completed_score = 0;
        self.can_see_player = false;
        self.character_type = CHARACTER_MAX;
        self.score = 0;
        self.line_score = 0;
        self.is_off_board = false;
        self.is_jump = false;
        self.right_foot_touching_off_board = false;
        self.left_foot_touching_off_board = false;
        self.right_foot_surface_id_off_board = 0;
        self.left_foot_surface_id_off_board = 0;
        self.left_foot_off_board_surface = MATERIAL_MAX;
        self.right_foot_off_board_surface = MATERIAL_MAX;
        self.terrain = FOOTSTEP_FLAT;
        self.is_hippy_jump = false;
        self.is_skitching = false;
        self.grabbing_dmo = false;
        self.dmo_key = 0;
        self.is_hand_plant = false;
        self.is_foot_planting = false;
        self.off_board_landing = NO_LANDING;
        self.hippy_jump_strength = SHORT_HIPPY;
        self.is_holding_board = false;
        self.is_dismounting = false;
        self.is_dismount_contact = false;
        self.was_in_dismount = false;
        self.is_air_dismount = false;
    }

    /// `SFXCTL_PlayerPhysics::InitSFX` 0x82510708 (the position controllers' `Assign*Vector`
    /// calls are the owner passing these vectors to them).
    pub fn init_sfx(&mut self, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.velocity = ZERO;
        self.forward = ZERO;
        self.position = ZERO;
        self.wheel_pos = [ZERO; 4];
    }

    /// `ConvertToImpactLevelFromTime` 0x82519840.
    fn impact_level_from_time(vault: &Vault, time: f32) -> i32 {
        if time >= vault.f32(CONTACTS_DEFAULT, 0x6D3D_91A9_BA7A_DCDC) {
            return 2;
        }
        i32::from(time >= vault.f32(CONTACTS_DEFAULT, 0x2A70_BB8A_3825_74E4))
    }

    /// `UpdateWheelLevelFromTime` 0x82519958.
    fn update_wheel_level_from_time(&mut self, vault: &Vault, wheel: usize, time: f32) {
        if time > 0.0 || self.landed[wheel] || self.is_rail_grind {
            let level = Self::impact_level_from_time(vault, self.prev_time_in_air[wheel]);
            self.prev_time_in_air[wheel] = time;
            self.impact_level[wheel] = level;
        }
    }

    /// `UpdateCollisionForces` 0x825195c0.
    fn update_collision_forces(&mut self, vault: &Vault) {
        let x: [f32; 8] = std::array::from_fn(|i| vault.layout_f32(COLLISIONS_DEFAULT, 4 + i as u32));
        let y: [f32; 8] = std::array::from_fn(|i| vault.layout_f32(COLLISIONS_DEFAULT, 12 + i as u32));
        for force in &mut self.skeleton_collision_info.contact_force {
            *force *= point_graph(&x, &y, self.last_player_speed);
        }
        self.last_player_speed = self.player_speed;
    }

    /// `CalculateWheelOcclusion` 0x825196a8 (input 3).
    fn calculate_wheel_occlusion(&mut self, up: Vec3, look: Vec3, mics: &MicrophoneMan, map: &mut MixMap) {
        let camera = mics.microphones[0].direction;
        let facing = vmx::dot3(up, camera);
        let clamped_low = if -1.0 - facing >= 0.0 { -1.0 } else { facing };
        let clamped = if 1.0 - clamped_low >= 0.0 { clamped_low } else { 1.0 };
        let base = (-(clamped + 1.0)).mul_add(0.5, 1.0);
        self.wheel_occlusion = base;
        let scaled = base * f32::from_bits(0x3f33_3333);
        self.wheel_occlusion = scaled;
        let vertical = vmx::dot3(up, [0.0, 1.0, 0.0, 0.0]);
        let vertical = (f64::from(vertical) * f64::from_bits(0x3fd3_3333_3333_3333)) as f32;
        let occlusion = vertical.mul_add(base, scaled);
        self.wheel_occlusion = occlusion;
        let side = vmx::dot3(look, camera);
        let side = (f64::from(f32::from_bits(side.to_bits() & 0x7FFF_FFFF)) * f64::from_bits(0x3fd3_3333_3333_3333)) as f32;
        let occlusion = side.mul_add(base, occlusion);
        let low = if -occlusion >= 0.0 { 0.0 } else { occlusion };
        let occlusion = if 1.0 - low >= 0.0 { low } else { 1.0 };
        self.wheel_occlusion = occlusion;
        self.set(map, 3, fctiwz(occlusion * 32767.0).clamp(0, 0x7FFF));
    }

    /// `SFXCTL_PlayerPhysics::UpdateParams` 0x8254a400. `skater` is `GetSkater(arrayIndex)`;
    /// the return is the wheel type the native also writes to `CSTATE_Player::mWheelType`.
    #[allow(clippy::too_many_lines)]
    pub fn update_params(&mut self, t: f32, skater: &SkaterData, inputs: PhysicsInputs, vault: &Vault, mics: &MicrophoneMan, map: &mut MixMap) -> u32 {
        self.prev_position = self.position;
        self.position = skater.skater_pos;
        self.distance = vmx::length(vmx::sub(self.position, mics.microphones[0].cur_position));
        self.forward = skater.skater_look;
        self.prev_velocity = self.velocity;
        self.velocity = skater.skater_vel;
        self.acceleration = vmx::sub(self.velocity, self.prev_velocity);
        self.board_position = skater.board_pos;
        self.board_forward = skater.board_look;
        self.board_velocity = skater.board_vel;
        let was_pushing_right = self.is_pushing_right;
        self.num_wheels_on_ground = skater.num_wheels_on_ground;
        self.board_speed = skater.board_speed;
        self.is_pushing_mongo = skater.is_push_mongo;
        self.player_speed = skater.player_speed;
        self.game_speed = inputs.game_speed;
        self.turn = skater.turn;
        self.is_rewinding = inputs.is_rewinding;
        self.in_air = skater.in_air;
        self.is_foot_braking = skater.foot_breaking;
        self.is_manual_braking = skater.manual_breaking;
        self.is_generally_pushing = skater.is_generally_pushing;
        self.is_generally_pushing_right_foot = skater.is_generally_pushing_right_foot;
        self.push_started = !was_pushing_right && !self.is_pushing_left && skater.is_pushing;
        self.is_pushing_right = skater.is_pushing && skater.is_generally_pushing_right_foot;
        self.is_pushing_left = skater.is_pushing && !skater.is_generally_pushing_right_foot && skater.is_generally_pushing;
        self.is_manual = skater.is_manual;
        self.was_rail_grind = self.is_rail_grind;
        self.is_rail_grind = skater.is_rail_grind;
        self.grind_type = skater.grind_type;
        self.grind_impact = skater.grind_impact;
        self.wheel_slide = skater.wheel_slide;
        self.time_to_land = skater.time_to_land;
        self.jump_height = skater.jump_height;
        self.deck_tilt = skater.deck_tilt;
        self.time_in_air = skater.time_in_air;
        self.torso_ang_vel = skater.player_angular_vel;
        if skater.jump_fraction != -1.0 {
            self.jump_fraction = skater.jump_fraction;
        }
        self.trick_done_type = skater.trick_finish_type;
        self.trick_type = skater.trick_type;
        self.is_performing_trick = skater.is_performing_trick;
        self.is_trick_with_scrape = skater.is_trick_with_scrape;
        self.is_hippy_jump = skater.is_hippy_jump;
        self.wheel_pos = skater.wheel_pos;
        self.landed = skater.wheel_landed;
        let air_time = if self.in_air { clamp_unit(self.time_in_air * 0.5) } else { 0.0 };
        if vault.bool(CONTACTS_DEFAULT, 0x642C_F9BF_EC6B_E988) {
            for wheel in 0..4 {
                self.update_wheel_level_from_time(vault, wheel, air_time);
            }
        } else {
            for wheel in 0..4 {
                self.impact_level[wheel] = skater.wheel_level[wheel] as i32;
            }
        }
        self.deck_local_xyz_angular_speed = skater.deck_local_xyz_angular_speed;
        self.skeleton_collision_info = skater.skeleton_collision_info;
        self.front_truck_in_contact = skater.front_truck_in_contact;
        self.back_truck_in_contact = skater.back_truck_in_contact;
        self.deck_in_contact = skater.deck_in_contact;
        self.surface_pattern = skater.surface_pattern;
        self.wheel_material = skater.wheel_material;
        self.deck_slide_speed = skater.deck_slide_speed;
        self.deck_contact_force = skater.deck_contact_force;
        self.left_foot_deck_y_speed = skater.left_foot_deck_y_speed;
        self.front_truck_material = skater.front_truck_material;
        self.right_foot_deck_y_speed = skater.right_foot_deck_y_speed;
        self.back_truck_material = skater.back_truck_material;
        self.left_foot_slide_deck_speed = skater.left_foot_slide_deck_speed;
        self.deck_material = skater.deck_material;
        self.right_foot_slide_deck_speed = skater.right_foot_slide_deck_speed;
        self.left_foot_on_deck = skater.left_foot_on_deck;
        self.left_foot_slide_world_speed = skater.left_foot_slide_world_speed;
        self.right_foot_on_deck = skater.right_foot_on_deck;
        self.right_foot_slide_world_speed = skater.right_foot_slide_world_speed;
        self.off_board_landing = skater.jump_landed;
        self.left_foot_angular_y_world_speed = skater.left_foot_angular_y_world_speed;
        self.right_foot_angular_y_world_speed = skater.right_foot_angular_y_world_speed;
        if self.is_performing_trick && self.trick_type != FOOT_PLANT {
            self.off_board_landing = NO_LANDING;
        }
        self.is_dismount_contact = false;
        self.hippy_jump_strength = skater.hippy_strength;
        self.is_dismounting = skater.is_dismounting;
        self.is_holding_board = skater.is_holding_board;
        if self.is_dismounting {
            if !self.was_in_dismount {
                self.was_in_dismount = true;
                self.dismount_start_time = self.cur_time;
            }
            let speed = self.board_speed * f32::from_bits(0x3df5_c28f);
            let speed = if -speed >= 0.0 { 0.0 } else { speed };
            if self.cur_time > self.dismount_start_time {
                let speed = if 1.0 - speed >= 0.0 { speed } else { 1.0 };
                if self.cur_time - self.dismount_start_time > (1.0 - speed) * f32::from_bits(0x3ecc_cccd) {
                    self.is_dismount_contact = true;
                }
            }
        } else {
            self.was_in_dismount = false;
        }
        let up = vmx::normalize_unguarded(skater.board_up);
        let look = vmx::normalize_unguarded(skater.board_look);
        self.cur_time += t;
        self.is_air_dismount = skater.is_air_dismount;
        self.limb_speed = skater.limb_speed;
        self.is_wiping_out = skater.is_wiping_out;
        self.is_wipe_out_over = skater.is_wipe_out_over;
        self.skate3_dark_catch = skater.skate3_dark_catch;
        self.skate3_only_trick = skater.skate3_only_trick;
        self.skate3_physics = skater.skate3_physics;
        self.calculate_wheel_occlusion(up, look, mics, map);
        self.wheel_hardness = u32::from(skater.wheel_hardness);
        let wheel_type = if inputs.local { self.wheel_hardness } else { inputs.npc_wheel_type };
        self.set(map, 12, if wheel_type == WHEEL_SOFT { 0x7FFF } else { 0 });
        self.is_grabbing_left = skater.is_grabbing_left;
        self.is_grabbing_right = skater.is_grabbing_right;
        self.is_reverting = skater.is_reverting;
        self.grind_material = skater.grind_material;
        self.score = skater.score;
        self.line_score = skater.line_score;
        self.character_type = skater.character;
        self.is_avoiding_by_braking = skater.is_avoiding_by_braking;
        self.is_avoiding_by_turning = skater.is_avoiding_by_turning;
        self.can_see_player = skater.can_see_player;
        self.is_celebrating = skater.is_celebrating;
        self.wipeout_seen = skater.wipeout_seen;
        self.wipeout_seen_actor_character = skater.wipeout_seen_actor_character;
        self.respawn_seen = skater.respawn_seen;
        self.respawn_seen_actor_character = skater.respawn_seen_actor_character;
        self.big_trick_seen = skater.big_trick_seen;
        self.big_trick_seen_score = skater.big_trick_seen_score;
        self.big_trick_seen_actor_character = skater.big_trick_seen_actor_character;
        self.big_trick_completed = skater.big_trick_completed;
        self.big_trick_completed_score = skater.big_trick_score;
        self.compression = skater.absorption_compression;
        self.is_off_board = skater.off_board;
        self.is_jump = skater.is_jump;
        self.right_foot_touching_off_board = skater.is_right_foot_touching_ground || self.is_pushing_right || self.is_foot_braking;
        self.left_foot_touching_off_board = skater.is_left_foot_touching_ground || self.is_pushing_left;
        if skater.right_foot_surface_id_off_board != 0 {
            self.right_foot_surface_id_off_board = skater.right_foot_surface_id_off_board;
        }
        if skater.left_foot_surface_id_off_board != 0 {
            self.left_foot_surface_id_off_board = skater.left_foot_surface_id_off_board;
        }
        self.terrain = skater.terrain;
        self.is_skitching = skater.is_skitching;
        self.grabbing_dmo = false;
        if inputs.local && skater.is_grabbing {
            self.dmo_key = inputs.grabbed_object_key;
            if inputs.grabbed_object_key != 0 {
                self.grabbing_dmo = true;
            }
        }
        self.is_hand_plant = skater.is_hand_plant;
        if self.is_hand_plant {
            self.hand_plant_audio_surface = if skater.hand_plant_surface == 0 {
                MATERIAL_CONCRETE_ROUGH
            } else {
                crate::presentation::audio_material(u32::from(skater.hand_plant_surface & 0x7F))
            };
        }
        self.is_foot_planting = skater.is_foot_planting;
        self.left_foot_off_board_surface = crate::presentation::audio_material(u32::from(self.left_foot_surface_id_off_board & 0x7F));
        self.right_foot_off_board_surface = crate::presentation::audio_material(u32::from(self.right_foot_surface_id_off_board & 0x7F));
        self.update_collision_forces(vault);
        self.board_slide = match skater.board_slide {
            1 => BOARD_SLIDE_TOP,
            2 => BOARD_SLIDE_SIDE,
            _ => BOARD_SLIDE_NONE,
        };
        self.update_nicotine(t, inputs, vault, map);
        self.wheel_hardness
    }

    /// `SFXCTL_PlayerPhysics::UpdateNicotine` 0x8253dc30.
    fn update_nicotine(&mut self, dt: f32, inputs: PhysicsInputs, vault: &Vault, map: &mut MixMap) {
        let flag = |value: bool| if value { 0x7FFF } else { 0 };
        self.set(map, 2, flag(self.num_wheels_on_ground == 0));
        let wheels = match self.num_wheels_on_ground {
            1 => 0x1FFF,
            2 => 0x3FFF,
            3 => 24575,
            4 => 0x7FFF,
            _ => 0,
        };
        self.set(map, 10, wheels);
        self.set(map, 4, flag(self.is_foot_braking));
        self.set(map, 5, flag(self.is_manual_braking));
        self.set(map, 0, q15_from(self.board_speed * f32::from_bits(0x46b8_507a)));
        self.set(map, 1, q15_from(self.board_speed * f32::from_bits(0x4575_c0a3)));
        self.set(map, 7, q15_from(self.board_speed * f32::from_bits(0x4513_7395)));
        self.set(map, 8, q15_from(self.board_speed * f32::from_bits(0x44d2_a51e)));
        self.set(map, 14, q15_from(self.player_speed * f32::from_bits(0x44d2_a51e)));
        self.set(map, 6, flag(self.is_performing_trick));
        self.set(map, 9, flag(!inputs.local));
        self.set(map, 11, flag(inputs.visual_bail));
        let mut velocity = self.velocity;
        if let Some(local) = inputs.local_velocity {
            velocity = vmx::sub(velocity, local);
        }
        let mut speed = vmx::length(velocity);
        let max = vault.f32(GENERAL_RELATIVE_SPEED, 0x204D_CC92_96DC_9400);
        if speed > max {
            speed = max;
        }
        let slew = vault.f32(GENERAL_RELATIVE_SPEED, 0x0395_642C_A6FC_543A) * dt;
        let last = self.last_relative_speed;
        if speed > last {
            if speed - last > slew {
                speed = slew + last;
            }
        } else if speed < last && last - speed > slew {
            speed = last - slew;
        }
        self.last_relative_speed = speed;
        let value = fctiwz(speed / vault.f32(GENERAL_RELATIVE_SPEED, 0x204D_CC92_96DC_9400) * 32767.0).clamp(0, 0x7FFF);
        self.set(map, 13, value);
    }
}
