//! ControlAirLegExtension82BAF538 and its ascending/descending/preland helpers.
use super::output::attributes::AnimationAttribute;
use crate::{physics::native_arithmetic::dot3, point_graph::PointGraph};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bone {
    Board,
    LeftToe,
    RightToe,
}

pub struct Settings {
    pub going_up_speed: f32,
    pub going_down_speed: f32,
    pub minimum_height: f32,
    pub preland_velocity: f32,
    pub preland_final_height: f32,
    pub offboard_multiplier: f32,
    pub arm_extension: PointGraph<4>,
}

///All vectors are preceding completed physical output in stock coordinates.
#[derive(Clone, Copy)]
pub struct Physical {
    pub com_velocity: [f32; 4],
    pub com_position: [f32; 4],
    pub system_up: [f32; 4],
    pub right_toe: [f32; 4],
    pub left_toe: [f32; 4],
    pub animation_height: f32,
    pub offboard_316: bool,
    pub remaining_air_time: f32,
}

///CreateInstance82BAF980 zeros mode8, distance12 and descending_time16.
#[derive(Default)]
pub struct State {
    mode: u32,
    height: f32,
    descending_time: f32,
}
impl State {
    pub fn needs_initial_attribute(&self) -> bool {
        self.mode == 0
    }
    pub fn needs_prelanding_query(&self, p: Physical) -> bool {
        self.mode == 2 || (self.mode == 1 && p.com_velocity[1] < 0.0)
    }
    ///The first update seeds the height but does not also advance its new mode.
    pub fn update(
        &mut self,
        bone: Bone,
        p: Physical,
        settings: &Settings,
        dt: f32,
        initial_attribute: Option<AnimationAttribute>,
        prepare_to_land: bool,
    ) -> [f32; 2] {
        let ascending = p.com_velocity[1] > 0.0;
        match self.mode {
            0 => {
                self.mode = if ascending { 1 } else { 2 };
                if let Some(attribute) = initial_attribute {
                    if attribute.kind == 0 {
                        //The queried union is initialized to zero before GetLastAttr.
                        self.height = f32::from_bits(attribute.payload.0[0].unwrap_or(0));
                    }
                } else {
                    let toe = if p.offboard_316 || bone == Bone::RightToe {
                        Some(p.right_toe)
                    } else if bone == Bone::LeftToe {
                        Some(p.left_toe)
                    } else {
                        None
                    };
                    self.height = match toe {
                        Some(toe) => dot3(
                            std::array::from_fn(|i| p.com_position[i] - toe[i]),
                            p.system_up,
                        ),
                        None => p.animation_height,
                    };
                }
            }
            1 if p.com_velocity[1] >= 0.0 => {
                //82BAFB60 is FNMSUBS, followed by FSEL against minimum1756.
                self.height = maximum(
                    (-dt).mul_add(settings.going_up_speed, self.height),
                    settings.minimum_height,
                );
            }
            1 | 2 => {
                self.mode = 2;
                if prepare_to_land {
                    self.mode = 3;
                    self.preland(bone, settings, dt);
                } else {
                    //82BAFC88..FD0C: both dot products precede the fused blend.
                    let closing = -dot3(p.com_velocity, p.system_up);
                    let vertical = dot3(p.system_up, [0.0, 1.0, 0.0, 0.0]).abs();
                    let speed = maximum(closing, 0.0)
                        .mul_add(vertical, (1.0 - vertical) * settings.going_down_speed);
                    let speed = minimum(speed, settings.going_down_speed);
                    self.height =
                        maximum((-speed).mul_add(dt, self.height), settings.minimum_height);
                }
            }
            3 => self.preland(bone, settings, dt),
            _ => {}
        }
        self.descending_time = if ascending {
            0.0
        } else {
            self.descending_time + dt
        };
        [
            self.height,
            settings.arm_extension.evaluate(self.descending_time),
        ]
    }
    fn preland(&mut self, bone: Bone, settings: &Settings, dt: f32) {
        self.height = dt.mul_add(settings.preland_velocity, self.height);
        if bone != Bone::Board {
            self.height *= settings.offboard_multiplier;
        }
    }
}

///82BAFDB8 checks the named FullExtension attribute after the physical query.
pub fn prepare_to_land(
    physical_query: bool,
    full_extension: Option<AnimationAttribute>,
    p: Physical,
    s: &Settings,
) -> bool {
    if !physical_query {
        return false;
    }
    let Some(attribute) = full_extension else {
        return true;
    };
    //GetLastAttr's scalar output remains initialized zero for nonfloat records.
    let extension = if attribute.kind == 0 {
        f32::from_bits(attribute.payload.0[0].unwrap_or(0))
    } else {
        0.0
    };
    if extension < s.preland_final_height - f32::from_bits(0x3dcccccd) {
        return true;
    }
    let required_time = (extension - p.animation_height) / s.preland_velocity;
    p.remaining_air_time - f32::from_bits(0x3d75c28f) < required_time
}
fn maximum(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { a } else { b }
}
fn minimum(a: f32, b: f32) -> f32 {
    if b - a >= 0.0 { a } else { b }
}
