//! Remaining physical subject getters consumed by the stock camera graph.
//! These are publications from physics/animation, not deductions from the shot.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CameraGraphSubject {
    pub time_since_player_input: f32, // getter520
    pub wipeout_tweak: u32,           //536
    pub onboard_air: bool,           //572
    pub preparing_to_jump: bool,     //576
    pub manual: bool,                //580
    pub offboard_air: bool,          //612
    pub running_out: bool,           //616
    pub footplant: bool,             //620
    pub handplant: bool,             //624
    pub hippy_jump: bool,            //628
    pub hippy_hurdle: bool,          //632
    pub boneless: bool,              //636
    pub moving_object: bool,         //644
    pub skitching: bool,             //648
    pub dropping_in: bool,           //664
    /// Physical output bundle2+184, read directly by82BB1EE0.
    pub slow_motion_air_duration: f32,
}

/// Engine-owned camera preferences and authored world camera-region membership.
/// An empty custom world has no road/ledge/volume camera annotations.
#[derive(Clone, Debug)]
pub(crate) struct CameraGraphEnvironment {
    /// Normal stock camera type0=Low, type1=High (graph-authored selection).
    pub camera_type: u32,
    pub on_road: bool,
    pub ledge_left: bool,
    pub ledge_right: bool,
    pub volumes: Vec<String>,
}
