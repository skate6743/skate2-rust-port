//! Actual observation values consumed by Wipeout82D8F9E0/82D90358.
use crate::{physics::skeleton_animation_record::AnimationPartTransform, point_graph::PointGraph};
pub type V = [f32; 4];
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub category: u32,
    pub timestep: f32,
    pub time_on_ground: f32,
    pub speed: f32,
    pub animation_up: V,
    pub landing_angle: f32,
    pub deck_velocity: V,
    pub com_velocity: V,
    pub jump_fix_frames: i32,
    ///Actual solver deck pose and the retained Processed0/16/32 basis.
    pub deck: AnimationPartTransform,
    pub input_board: AnimationPartTransform,
    pub world_to_animation: AnimationPartTransform,
    pub closing_velocity: V,
    pub board_material_flags: u32,
    pub board_contact: bool,
    pub wheel_contact: bool,
    pub board_contact_normal: V,
    pub opposing_contact: f32,
    pub regions_force: [f32; 8],
    pub maximum_skater_force: f32,
    pub vehicle_force: f32,
    pub group_8: bool,
    pub conflicting: bool,
    pub compliant: bool,
    pub highest_normal: V,
    pub pose_error: V,
    pub maximum_pose_error: f32,
    pub flip_active: bool,
    pub flip_requested_speed: f32,
    pub system_up_y: f32,
    pub grind_selected: bool,
    pub grind_normal_valid: bool,
    pub grind_normal: V,
}
#[derive(Clone, Copy)]
pub struct Mode {
    pub check_squash: bool,
    pub check_bad_landing: bool,
    pub ground_xz: f32,
    pub bad_landing_scale: f32,
}
pub struct GroundSettings {
    pub vehicle_scalar: f32,
    pub vehicle_contact: f32,
    pub skitch_contact: f32,
    pub skitch_scalar: f32,
    pub skitch_arms_scalar: f32,
    pub skater_scalar: f32,
    pub max_squash: f32,
    pub max_squash_coffin: f32,
    pub max_displacement: f32,
    pub max_contact: f32,
    pub max_arm_contact: f32,
    pub max_deck_error: f32,
    pub opposing_contact: f32,
    pub y_acceleration: f32,
    pub light_dmo_scalar: f32,
    pub player_scalar: f32,
    pub ai_scalar: f32,
    pub skitch_acc_scalar: f32,
    pub balance_total: f32,
    pub balance_min_speed: f32,
    pub balance_base: f32,
}
pub struct AirSettings {
    pub xz_trick: f32,
    pub y_trick: f32,
    pub xz_acceleration: f32,
    pub y_acceleration: f32,
    pub max_squash: f32,
    pub max_displacement: f32,
    pub max_contact: f32,
    pub max_arm_contact: f32,
    pub body_flip_scalar: f32,
    pub body_flip_acc_scalar: f32,
    pub light_dmo_scalar: f32,
    pub ignore_danger_frames: i32,
    pub max_landing_speed: f32,
    pub max_stairs_speed: f32,
    pub max_grind_speed: f32,
    ///Layout0 PointNegGraph; original directly reads X16/Y48.
    pub max_landing_angle: PointGraph<8>,
}
pub struct Settings {
    pub ground: GroundSettings,
    pub air: AirSettings,
    pub lean_contact_y: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct RequestInput {
    pub flags_2468: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub animation_up_y: f32,
    pub category: u32,
}

/// Completed observations used by the shared source FootPlant collision check.
/// A noncontacting board requires no invented closing velocity or matrix.
pub struct PlantInput {
    pub maximum_pose_error: f32,
    pub pose_error: V,
    pub regions_force: [f32; 8],
    pub contact: Option<PlantContact>,
}
pub struct PlantContact {
    pub world_to_animation: AnimationPartTransform,
    pub closing_velocity: V,
}
