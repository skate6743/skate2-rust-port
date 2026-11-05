//! Normal CameraMan persistent state, constructor/reset82DFDA60/82DFDFA0.
use super::{ScalarTracker, ScalarTrackerParameters};

pub(super) const STEP: f32 = f32::from_bits(0x3c888889);
pub(super) const HALF_PI: f32 = f32::from_bits(0x3fc90fdb);
pub(super) const DEGREES: f32 = f32::from_bits(0x42652ee1);
pub(super) const UP: [f32; 4] = [0.0, 1.0, 0.0, 0.0];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManagerState {
    pub anchor: [f32; 4],                  //624
    pub anchor_velocity: [f32; 4],         //656
    pub landing_normal: [f32; 4],          //720
    pub launch_position: [f32; 4],         //736
    pub ground_normal: [f32; 4],           //752
    pub previous_velocity: [f32; 4],       //768
    pub incline_normal: [f32; 4],          //784
    pub selected_anchor: u32,              //800
    pub frontside_angle: f32,              //808
    pub ground_heading: f32,               //812
    pub velocity_incline: f32,             //816
    pub absolute_velocity_incline: f32,    //820
    pub launch_incline: f32,               //824
    pub signed_landing_incline: f32,       //828
    pub landing_incline: f32,              //832
    pub direction_incline: f32,            //836
    pub field_of_view: f32,                //844
    pub apex_height: f32,                  //848
    pub apex_time: f32,                    //852
    pub time_without_trajectory: f32,      //856
    pub landing_height_delta: f32,         //860
    pub apex_height_ratio: f32,            //864
    pub launch_landing_heading_delta: f32, //868
    pub trajectory_camera_distance: f32,   //872
    pub steering_time: f32,                //876
    pub centred_time: f32,                 //880
    pub opacity: f32,                      //884
    pub aspect_ratio: f32,                 //888
    pub blur: f32,                         //896
    pub effect_weight: f32,                //900
    pub collision_elevation: f32,          //904
    pub frames: u32,                       //908
    pub slowmo_frames: u32,                //912
    pub instant_frames: u32,               //916
    pub flags: u8,                         //920
    pub options: u8,                       //921
    pub heading_mirror: ScalarTracker,     //464
    pub framing_mirror: ScalarTracker,     //524
}

impl ManagerState {
    pub fn new() -> Self {
        Self {
            anchor: [0.0; 4],
            anchor_velocity: [0.0; 4],
            landing_normal: UP,
            launch_position: [0.0; 4],
            ground_normal: UP,
            previous_velocity: [0.0; 4],
            incline_normal: [0.0; 4],
            selected_anchor: 0,
            frontside_angle: 0.0,
            ground_heading: 0.0,
            velocity_incline: 0.0,
            absolute_velocity_incline: 0.0,
            launch_incline: 0.0,
            signed_landing_incline: 0.0,
            landing_incline: 0.0,
            direction_incline: 0.0,
            field_of_view: 0.0,
            apex_height: 0.0,
            apex_time: 0.0,
            time_without_trajectory: f32::MAX,
            landing_height_delta: 0.0,
            apex_height_ratio: 0.0,
            launch_landing_heading_delta: 0.0,
            trajectory_camera_distance: 0.0,
            steering_time: 0.0,
            centred_time: 0.0,
            opacity: 1.0,
            aspect_ratio: f32::from_bits(0x3fe38e39),
            blur: 1.0,
            effect_weight: 1.0,
            collision_elevation: 0.0,
            frames: 0,
            slowmo_frames: 0,
            instant_frames: 3,
            flags: 2,
            options: 0x60,
            heading_mirror: mirror_tracker(),
            framing_mirror: mirror_tracker(),
        }
    }

    pub fn reset_requested(&self) -> bool {
        self.options & 0x80 != 0
    }
    pub fn request_reset(&mut self) {
        self.options |= 0x80;
    }
    pub fn mirrored(&self) -> bool {
        self.flags & 8 != 0
    }
}

pub(super) fn clamp(value: f32, low: f32, high: f32) -> f32 {
    let value = if low - value >= 0.0 { low } else { value };
    if high - value >= 0.0 { value } else { high }
}

fn mirror_tracker() -> ScalarTracker {
    ScalarTracker {
        target: 1.0,
        position: 1.0,
        velocity: 0.0,
        acceleration: 0.0,
        acceleration_clamp: 0.0,
        smoothing: 0.0,
    }
}

pub(super) fn mirror_parameters(speed: f32, acceleration: f32) -> ScalarTrackerParameters {
    ScalarTrackerParameters {
        delta_umbra: 0.0,
        delta_penumbra: 0.0,
        speed_clamp: speed,
        acceleration_clamp_min: acceleration,
        acceleration_clamp_max: acceleration,
        smoothing_min: 0.85,
        smoothing_max: 0.85,
        overshoot_zeroes_velocity: true,
    }
}
