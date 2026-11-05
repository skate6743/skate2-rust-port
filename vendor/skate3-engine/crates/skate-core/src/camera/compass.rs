//! Nine stock gameplay compass headings, TU382DF3770/82DF39B0. These are
//! subject directions consumed by authored shots, not replay camera controls.
use super::manager_state::clamp;
use super::vector_tracker::length;
use super::{ManagerSubject, direction_to_angles};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompassSettings {
    pub heading_response: f32,       //parameter604
    pub time_before_lineup: f32,     //880
    pub lineup_speed: f32,           //884
    pub minimum_deadzone_speed: f32, //888
    pub maximum_deadzone_speed: f32, //892
    pub maximum_deadzone_size: f32,  //896
    pub deadzone_smoothing: f32,     //900
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompassInputs {
    pub ground_normal: [f32; 4],  //0: subject452
    pub landing_normal: [f32; 4], //16: subject412 or448
    /// Publisher cache80, also published as subject376: skeleton root when
    /// offboard/wiping out, otherwise the previous physical transform.
    pub transform: [[f32; 4]; 4], //32
    pub trajectory_direction: [f32; 4], //96: subject420 or normalized392
    pub launch_position: [f32; 4], //112
    pub landing_position: [f32; 4], //128
    pub grind_direction: [f32; 4], //144: subject424
    pub skeleton_direction: [f32; 4], //160: subject440=Skeleton output0
    pub camera_position: [f32; 4], //176
    pub look_target: [f32; 4],    //192: subject468=output6+64
    pub look: [f32; 2],           //208
    pub trajectory_fraction: f32, //224
    pub selected_compass: u32,    //228
    pub no_trajectory: bool,      //232bit80
    pub state_103: bool,          //bit40: subject564
    pub wiping_out: bool,         //bit20
    pub grinding: bool,           //bit10
    pub air_flag_452: bool,       //bit8
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Compass {
    pub headings: [f32; 9],          //240..272 and their working copies280..312
    pub movement_heading: f32,       //316
    pub orbit_delta: f32,            //320
    pub velocity: [f32; 4],          //336
    pub damped_velocity: [f32; 4],   //352
    pub previous_position: [f32; 4], //368
    pub orbit_position: [f32; 4],    //384
    pub previous_reset: bool,        //400
    pub stopped_time: f32,           //404
    pub deadzone_size: f32,          //408
    pub deadzone_position: [f32; 2], //416
    pub deadzone_velocity: [f32; 2], //432
}

impl Compass {
    pub fn new() -> Self {
        Self {
            headings: [0.0; 9],
            movement_heading: 0.0,
            orbit_delta: 0.0,
            velocity: [0.0; 4],
            damped_velocity: [0.0; 4],
            previous_position: [0.0; 4],
            orbit_position: [0.0; 4],
            previous_reset: true,
            stopped_time: 0.0,
            deadzone_size: 0.0,
            deadzone_position: [0.0; 2],
            deadzone_velocity: [0.0; 2],
        }
    }

    ///82DF7B90 supplies0 on subject reset and exactly1/60 otherwise.
    pub fn update(&mut self, dt: f32, input: CompassInputs, settings: CompassSettings) -> [f32; 9] {
        if self.previous_reset {
            self.previous_position = input.transform[3];
            self.stopped_time = 0.0;
            self.deadzone_size = 0.0;
        }
        self.update_movement(dt, input, settings);
        let mut launch = heading(input.ground_normal);
        let mut landing = heading(input.landing_normal);
        if (launch - landing).abs() > PI {
            if launch >= landing {
                landing += TAU;
            } else {
                launch += TAU;
            }
        }
        self.headings[1] = wrap((landing - launch).mul_add(input.trajectory_fraction, launch));
        self.headings[0] = self.movement_heading;
        let difference = horizontal(sub(input.landing_position, input.launch_position));
        if input.trajectory_fraction > 0.0 && length(difference) > 1.0 {
            self.headings[4] = if dt == 0.0 {
                direction_to_angles(input.skeleton_direction)[1]
            } else {
                -direction_to_angles(input.trajectory_direction)[1]
            };
            self.headings[0] = wrap(blend(
                self.headings[0],
                self.headings[4],
                input.trajectory_fraction,
            ));
        }
        self.headings[3] = heading(input.transform[2]);
        self.update_orbit(dt, input);
        self.update_follow(input);
        let direction = horizontal(sub(input.transform[3], input.camera_position));
        let forward = heading(input.transform[2]);
        let camera_heading = heading(direction);
        let delta = wrap_floor(camera_heading - forward);
        let minimum = f32::from_bits(0x3f9c61aa);
        self.headings[8] = if delta.abs() <= minimum {
            if delta >= 0.0 {
                forward + minimum
            } else {
                forward - minimum
            }
        } else {
            camera_heading
        };
        self.previous_reset = dt == 0.0;
        self.headings
    }

    fn update_follow(&mut self, input: CompassInputs) {
        let relative = sub(input.transform[3], input.camera_position);
        if input.selected_compass != 7 {
            self.headings[7] = heading(horizontal(relative));
        } else if input.air_flag_452 {
            self.headings[7] = heading(horizontal(sub(input.look_target, input.transform[3])));
        } else {
            let velocity = horizontal(self.velocity);
            let speed = length(velocity);
            if speed > 0.7 {
                let maximum = clamp((speed - 0.7) * f32::from_bits(0x3ede9bd3), 0.0, 1.0) * 0.6;
                let target = heading(velocity);
                let previous = heading(relative);
                self.headings[7] =
                    clamp(wrap_floor(target - previous) * 0.25, -maximum, maximum) + previous;
            }
        }
    }
}

pub(super) const PI: f32 = f32::from_bits(0x40490fdb);
pub(super) const TAU: f32 = f32::from_bits(0x40c90fdb);
pub(super) fn horizontal(v: [f32; 4]) -> [f32; 4] {
    [v[0], 0.0, v[2], v[3]]
}
pub(super) fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn wrap(v: f32) -> f32 {
    super::rig_tracking::wrap_vmx(v)
}
pub(super) fn blend(from: f32, to: f32, weight: f32) -> f32 {
    wrap(wrap(to - from).mul_add(weight, from))
}
pub(super) fn wrap_floor(v: f32) -> f32 {
    let turns = v.mul_add(f32::from_bits(0x3e22f983), 0.5).floor();
    -turns.mul_add(TAU, -v)
}
///82DF3888 intentionally tests positive Y only and does not normalize XZ.
pub(super) fn heading(v: [f32; 4]) -> f32 {
    if v[1] > f32::from_bits(0x3f7ff2e5) {
        0.0
    } else {
        super::manager_incline::atan_ratio(-v[0], v[2])
    }
}

/// Physical fields not already present in ManagerSubject, used by82DF7B90.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompassPoseInputs {
    pub skeleton_direction: [f32; 4],
    pub look_target: [f32; 4],
    pub board_velocity: [f32; 4],
    pub trajectory_direction: [f32; 4],
    pub state_103: bool,
}
impl CompassPoseInputs {
    pub fn bind(
        self,
        subject: &ManagerSubject,
        camera_position: [f32; 4],
        selected_compass: u32,
    ) -> CompassInputs {
        let trajectory = subject.rig.trajectory_valid != 0;
        let trajectory_direction = if subject.rig.grinding != 0 {
            subject.direction_424
        } else if trajectory {
            self.trajectory_direction
        } else {
            super::orientation_math::normalize(self.board_velocity)
        };
        CompassInputs {
            ground_normal: subject.rig.last_valid_ground_up,
            landing_normal: if trajectory {
                subject.landing_normal
            } else {
                subject.ground_normal
            },
            // 82DF80D8 chooses cache80 before 82DF69C0 publishes setter24.
            // 82DF7B90 copies that same cache into compass32..80. Setter28
            // updates the separate physical history, not this orbit centre.
            transform: subject.rig.transform,
            trajectory_direction,
            launch_position: if trajectory {
                subject.launch_position
            } else {
                [0.0; 4]
            },
            landing_position: if trajectory {
                subject.landing_position
            } else {
                [0.0; 4]
            },
            grind_direction: subject.direction_424,
            skeleton_direction: self.skeleton_direction,
            camera_position,
            look_target: self.look_target,
            look: subject.look,
            trajectory_fraction: if trajectory {
                clamp(
                    subject.trajectory_time / subject.trajectory_duration,
                    0.0,
                    1.0,
                )
            } else {
                0.0
            },
            selected_compass,
            no_trajectory: !trajectory,
            state_103: self.state_103,
            wiping_out: subject.rig.wiping_out != 0,
            grinding: subject.rig.grinding != 0,
            air_flag_452: subject.rig.air_flag_452 != 0,
        }
    }
}
