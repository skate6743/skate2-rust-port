//! Typed boundaries of original82D78xxx/82D79xxx. No synthetic input defaults.
use super::{Frame, QueryRequest, Vector};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    ///physics_landingondeck layout+4; strict board-up.Y comparison.
    pub deck_min_uprightness: f32,
    ///physics_landingondeck layout+24; plane point=deck112+up544*height.
    pub approximate_com_height: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub board_up_80: Vector,
    pub board_position_112: Vector,
    pub board_velocity_400: Vector,
    pub up_544: Vector,
    pub support_1776: i32,
    pub flags_2480: u32,
    pub mode_2540: u32,
    pub wheel_contacts_2556: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssistInput {
    pub processed: Input,
    pub position: Vector,
    pub velocity: Vector,
    pub maximum_velocity_change: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IkInput {
    pub processed: Input,
    pub time_to_land: f32,
    pub animation_root: Frame,
    pub animation_com_10960: Vector,
    pub mapped_position_12608: Vector,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyncInput {
    pub position_592: Vector,
    pub flags_2488: u32,
}
///Complete numeric result82D794A0; offsets describe original output, not Rust ABI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpdateOutput {
    pub can_land: bool,           //0
    pub trajectory_valid: bool,   //1
    pub time_to_land: f32,        //4
    pub elapsed: f32,             //8
    pub landing_time: f32,        //12
    pub apex_time: f32,           //16
    pub position: Vector,         //32
    pub landing_velocity: Vector, //48
    pub launch_position: Vector,  //64
    pub landing_position: Vector, //80
    pub normal: Vector,           //96
    pub apex_position: Vector,    //112
    pub direction: Vector,        //128
    pub up: Vector,               //144
    ///Host must additionally use actual processed filter2952 and submit before
    ///query_submitted(). Native request has radius.3, start/end error.5.
    pub query: Option<QueryRequest>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillOutput {
    pub can_land_316: bool,
    pub hippy_hurdling_317: bool,
    ///Some sets skeleton3482 and writes skeleton48; None leaves both untouched.
    pub moving_contact: Option<Vector>,
}
