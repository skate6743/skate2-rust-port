//! Launch data82D33448/82BE33D0 and the source-owned caller observations.
use super::math::{IDENTITY, Transform, Vector, ZERO};
use crate::point_graph::PointGraph;
#[derive(Clone, Copy, Debug)]
pub struct LaunchInfo {
    pub reckoning_transform: Transform,  //0=Reckoning816
    pub reckoning_inverse: Transform,    //64=Reckoning944
    pub start_velocity: Vector,          //128=Processed400 m/s
    pub com_velocity: Vector,            //144=Processed704 m/s
    pub skeleton_vector_160: Vector,     //160=Skeleton16208
    pub skeleton_vector_176: Vector,     //176=Skeleton16240
    pub board_position: Vector,          //192=Processed112 metres
    pub animation_com_position: Vector,  //208=Processed592 metres
    pub start_position_override: Vector, //224 metres
    pub board_position_override: Vector, //240 metres
    pub cone_angle_x: f32,               //256 degrees
    pub cone_angle_z: f32,               //260 degrees
    pub timestep: f32,                   //264 seconds
    pub player_jumped: bool,             //268
    pub use_position_override: bool,     //269
    pub trajectory_count: u16,           //270 GetLaunchInfo selects1 or7
}
impl Default for LaunchInfo {
    ///Original ctor82D33448; Fill must supply current physical observations.
    fn default() -> Self {
        Self {
            reckoning_transform: IDENTITY,
            reckoning_inverse: IDENTITY,
            start_velocity: ZERO,
            com_velocity: ZERO,
            skeleton_vector_160: ZERO,
            skeleton_vector_176: ZERO,
            board_position: ZERO,
            animation_com_position: ZERO,
            start_position_override: ZERO,
            board_position_override: ZERO,
            cone_angle_x: 0.0,
            cone_angle_z: 0.0,
            timestep: 0.0,
            player_jumped: false,
            use_position_override: false,
            trajectory_count: 0,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SelectorInput {
    pub gravity: Vector,              //PhysicsWorld32 m/s^2
    pub ground_normal: Vector,        //Processed464
    pub contact_position: Vector,     //496 metres
    pub heading_direction: Vector,    //512
    pub reference_up: Vector,         //544
    pub board_vertical_velocity: f32, //404 m/s
    pub directional_input: f32,       //2636
    pub previous_physics_state: u32,  //2504
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub offboard_flags_1776: u32,
    pub grind_lock_distance: f32, //active physics_mode80 metres
}
///Stock skater collection values; offsets refer to physics_trajectory unless stated.
#[derive(Clone, Debug)]
pub struct SelectorSettings {
    pub cone_x: f32,                                //576 degrees, Skeleton GetLaunchInfo
    pub cone_z: f32,                                //568 degrees, Skeleton GetLaunchInfo
    pub trajectory_max_time: f32,                   //428 seconds
    pub trajectory_max_drop: f32,                   //63673FB6DA092B7E metres
    pub trajectory_error_start: f32,                //432
    pub trajectory_error_end: f32,                  //436
    pub speed_factor_min: f32,                      //440 m/s
    pub speed_factor_max: f32,                      //444 m/s
    pub cone_angle_z_vs_speed: PointGraph<8>,       //304
    pub cone_x_second_pass: f32,                    //572 degrees
    pub cone_z_second_pass: f32,                    //564 degrees
    pub landing_time_bonus: PointGraph<8>,          //0 x16/y48
    pub landing_com_scalar_vs_slope: PointGraph<8>, //80 x96/y128
    pub landing_force_scalar: PointGraph<8>,        //240
    pub grind_penalty_vs_distance: PointGraph<8>,   //160 x176/y208
    pub score_middle_bonus: f32,                    //468
    pub score_landing_force: f32,                   //472
    pub score_landing_direction: f32,               //476
    pub score_transition: f32,                      //456
    pub surface_unrideable_score: f32,              //460
    pub surface_dont_align_score: f32,              //464
    pub natural_air_off_verts_scalar: f32,          //484
    pub minimum_valid_time: f32,                    //488 seconds
    pub minimum_time_after_apex: f32,               //492 seconds
    pub minimum_normal_delta_second_pass: f32,      //496 radians
    pub maximum_trajectory_adjust: f32,             //500
    pub wall_ride_test_distance: f32,               //416 metres
    pub wall_ride_minimum_height: f32,              //420 metres
    pub wall_ride_angle_allow_landing: f32,         //424
    pub wall_ride_height_score: f32,                //452
    pub wall_ride_boost: PointGraph<4>,             //368 x384/y400
    pub wall_ride_normal_dot_limit: f32,            //physics global188/layout248
    pub displacement_vs_speed: PointGraph<8>,       //physics_reckoning544
    pub displacement_vs_ground_normal: PointGraph<8>, //physics_reckoning608
    pub trajectory_radius: f32,                     //physics_reckoning C6710942E297D587 metres
    pub trajectory_displacement: f32,               //physics_reckoning runtime attribute
    pub minimum_trajectory_frames: i32,             //physics_reckoning runtime attribute
    pub vert_jump_align_factor: f32,                //0B42DFBB7F7756A0
    pub vert_jump_align_max_ground_normal_y: f32,   //94923D136EA2DA37
    pub vert_jump_align_min_direction_y: f32,       //58BFC8D41216C7EE
    pub vert_jump_align_max_angle: f32,             //723FF102068622CF
}
