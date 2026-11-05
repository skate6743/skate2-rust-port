use crate::point_graph::PointGraph;
#[derive(Clone, Copy, Debug, Default)]
pub struct GroundJump {
    pub velocity: [f32; 4], //JumpInfo0, metres/second
    pub scalar_16: f32,     //82D93618 initializes and retains zero
    pub active: bool,       //JumpInfo20
}
#[derive(Clone, Copy, Debug)]
pub struct GroundJumpMode {
    pub minimum_height_64: f32, //physics_mode Hash_BE3F74F978D777E5, metres
    pub minimum_height_68: f32, //JumpMinHeight, metres
    pub maximum_height: f32,    //JumpMaxHeight72, metres
}
pub struct GroundJumpSettings {
    pub vertical_response: PointGraph<16>,      //physics_jump0
    pub y_scalar_vs_normal_y: PointGraph<8>,    //128
    pub speed_scalar_vs_angle: PointGraph<8>,   //208
    pub minimum_height_vs_speed: PointGraph<8>, //288
    pub maximum_height_vs_speed: PointGraph<8>, //352
    pub speed_response_max_speed: f32,          //420, metres/second
    pub minimum_scalar: f32,                    //424
    pub maximum_y_bonus: f32,                   //428, metres/second
    pub adjust_z_factor: f32,                   //432
    pub adjust_x_factor: f32,                   //436
    pub absolute_minimum_height: f32,           //456, metres
}
#[derive(Clone, Copy, Debug)]
pub struct GroundJumpInput {
    pub flags_2468: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub flags_2488: u32,
    pub effective_forward: [f32; 4],         //Processed160
    pub forward: [f32; 4],                   //352
    pub current_velocity: [f32; 4],          //400 metres/second
    pub ground_reference_position: [f32; 4], //480 metres
    pub reference_up: [f32; 4],              //544
    pub filtered_ground_normal: [f32; 4],    //576, BoardToolkit normal192
    pub animation_com_position: [f32; 4],    //592 metres
    pub prepared_velocity: [f32; 4],         //704 metres/second
    pub jump_strength: f32,                  //2624, animation output
    pub jump_controls: [f32; 2],             //2628/2632, animation output
    pub gravity_y: f32,                      //2648 metres/second squared
    pub surface_speed: f32,                  //2656 metres/second
}
