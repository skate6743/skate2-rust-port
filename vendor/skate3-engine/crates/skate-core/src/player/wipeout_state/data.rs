//! Original WipeoutGround Reset82D3B3A8. Native offsets document ownership.
#[derive(Clone, Debug)]
pub struct State {
    pub time: f32,                      //304
    pub settled_time: f32,              //308
    pub time_until_teleport: f32,       //312
    pub impaled_time: f32,              //316
    pub no_support_time: f32,           //320
    pub response_time: f32,             //324
    pub extra_weight_zero_time: f32,    //328
    pub slow_time: f32,                 //332
    pub surface_height: f32,            //336
    pub board_offset: [f32; 4],         //352
    pub forward: [f32; 4],              //368
    pub right: [f32; 4],                //384
    pub angular_velocity: [f32; 4],     //400
    pub retained_tilt: [f32; 4],        //416
    pub velocity: [f32; 4],             //432
    pub retained_velocity: [f32; 4],    //448
    pub predicted_position: [f32; 4],   //464
    pub over: bool,                     //480
    pub slow: bool,                     //481
    pub material_eleven_response: bool, //482
    pub air_collision_mode: bool,       //483
    pub request_teleport: bool,         //484
    pub move_board: bool,               //485
    pub special_surface: bool,          //486
    pub below_surface: bool,            //487
    pub retained_velocity_active: bool, //488
    pub sideways_input: f32,            //492
    pub forward_input: f32,             //496
    pub orientation: f32,               //500
    pub collision_weight: f32,          //504
    pub controlled_weight: f32,         //508
    pub control_time: f32,              //512
    pub retained_sideways_input: f32,   //516
    pub retained_forward_input: f32,    //520
    pub response_scalar: f32,           //524
    pub extra_weight: f32,              //528
    pub maximum_speed: f32,             //532
    pub response_start_speed: f32,      //536
    pub response_change: f32,           //540
    pub board_move_frames: u32,         //544, initialized by Enter
    pub response_count: u32,            //548
    pub response_frames: i32,           //552
    pub airborne_frames: i32,           //556
    pub teleport_countdown: i32,        //560
    pub prevent_manual: bool,           //564
    pub ignore_reset: bool,             //565
    pub reset_ever: bool,               //566
    pub ever_settled: bool,             //567
    pub recovery_eligible: bool,        //568
    pub teleport_pending: bool,         //569
    pub ever_impaled: bool,             //570
    pub direction_initialized: bool,    //571
    pub material_ten_response: bool,    //572
    pub imminent_surface_twelve: bool,  //573
    pub allow_retained_velocity: bool,  //574
    pub response_finished: bool,        //575
    pub surface_query: bool,            //576
    pub profile: usize,                 //580
    pub predicted_time: f32,            //832
}
impl Default for State {
    fn default() -> Self {
        Self {
            time: 0.0,
            settled_time: 0.0,
            time_until_teleport: 0.0,
            impaled_time: 0.0,
            no_support_time: 0.0,
            response_time: 0.0,
            extra_weight_zero_time: 0.0,
            slow_time: 0.0,
            surface_height: 0.0,
            board_offset: [0.0; 4],
            forward: [0.0; 4],
            right: [0.0; 4],
            angular_velocity: [0.0; 4],
            retained_tilt: [0.0; 4],
            velocity: [0.0; 4],
            retained_velocity: [0.0; 4],
            predicted_position: [0.0; 4],
            over: false,
            slow: false,
            material_eleven_response: false,
            air_collision_mode: false,
            request_teleport: false,
            move_board: false,
            special_surface: false,
            below_surface: false,
            retained_velocity_active: false,
            sideways_input: 0.0,
            forward_input: 0.0,
            orientation: 0.0,
            collision_weight: 0.0,
            controlled_weight: 0.0,
            control_time: 0.0,
            retained_sideways_input: 0.0,
            retained_forward_input: 0.0,
            response_scalar: 0.0,
            extra_weight: 1.0,
            maximum_speed: 10.0,
            response_start_speed: 0.0,
            response_change: 0.0,
            board_move_frames: 0,
            response_count: 0,
            response_frames: -1,
            airborne_frames: -1,
            teleport_countdown: -1,
            prevent_manual: false,
            ignore_reset: false,
            reset_ever: false,
            ever_settled: false,
            recovery_eligible: false,
            teleport_pending: false,
            ever_impaled: false,
            direction_initialized: false,
            material_ten_response: false,
            imminent_surface_twelve: false,
            allow_retained_velocity: true,
            response_finished: false,
            surface_query: false,
            profile: 0,
            predicted_time: f32::MAX,
        }
    }
}
