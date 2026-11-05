//! Complete TU3 Skeleton::UpdateLandingAdjust82BD9028. This owns the four
//! landing modes and history; it does not synthesize physical COM observations.
use super::skeleton_board_offset::SkateboardOffset;
use crate::point_graph::PointGraph;
const STEP: f32 = f32::from_bits(0x3C88_8889);

#[derive(Clone, Debug)]
pub struct LandingSettings {
    pub manual_blend: PointGraph<4>,
    pub grind_blend: PointGraph<4>,
    pub coffin_height: PointGraph<4>,
    pub minimum_height: f32,
    pub maximum_velocity: f32,
    pub manual_damping: f32,
    pub manual_spring: f32,
    pub ground_minimum_compression_time: f32,
    pub ground_damping: f32,
    pub ground_spring: f32,
    pub grind_animation_target_time: f32,
    pub grind_damping: f32,
    pub grind_spring: f32,
    pub grind_target_delta: f32,
    pub desired_com_height: f32,
    pub coffin_time: f32,
    pub coffin_maximum_velocity: f32,
    pub coffin_blend_frames: f32,
    pub coffin_base_height: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct LandingInput {
    pub filtered_state: u32,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub balance: f32,
    /// dot(Processed608 physical COM velocity, Processed544 ground up).
    pub physical_com_velocity_along_up: f32,
    /// dot(Skeleton10928 physical COM - Skeleton9680 physical board, up).
    pub physical_com_height: f32,
    /// Previous animation COM Skeleton10960.y minus current board pose12672.y.
    pub animation_com_height: f32,
}

#[derive(Clone, Debug)]
pub struct LandingAdjustment {
    pub active: bool,
    pub kind: u32,
    pub time: f32,
    pub position: f32,
    pub velocity: f32,
    pub previous_com_velocity: f32,
    pub previous_animation_height: f32,
    pub previous_filtered_state: u32,
    pub desired_grind_com: f32,
}
impl Default for LandingAdjustment {
    fn default() -> Self {
        // Skeleton ctor82BD7A9C..7ABC.
        Self {
            active: false,
            kind: 0,
            time: 0.0,
            position: 0.0,
            velocity: 0.0,
            previous_com_velocity: 0.0,
            previous_animation_height: f32::MAX,
            previous_filtered_state: 0,
            desired_grind_com: 0.0,
        }
    }
}
impl LandingAdjustment {
    pub fn update(
        &mut self,
        input: LandingInput,
        settings: &LandingSettings,
        offset: &mut SkateboardOffset,
    ) {
        let from_offboard = self.previous_filtered_state == 6 && input.filtered_state == 1;
        let from_air = self.previous_filtered_state == 2 && input.filtered_state == 1;
        let ground = (from_offboard || from_air) && input.flags_2476 & 0x1000_0000 != 0;
        let manual = from_air && input.balance != 0.0;
        let grind = self.previous_filtered_state == 2
            && input.filtered_state == 3
            && input.flags_2468 & 0x20 == 0;
        let coffin_flag = input.flags_2472 & 1 != 0;
        let coffin = coffin_flag && (grind || from_air);
        let cancel = input.flags_2476 & 0x4000_0000 != 0 && !coffin_flag;
        if cancel {
            self.active = false;
        }
        let mut first_ground_update = false;
        if !cancel && !self.active && (ground || coffin || manual || grind) {
            self.time = 0.0;
            self.active = true;
            self.kind = if coffin {
                3
            } else if manual {
                2
            } else if ground {
                0
            } else {
                1
            };
            first_ground_update = self.kind == 0;
            let velocity = if grind || coffin {
                self.previous_com_velocity
            } else {
                input.physical_com_velocity_along_up
            };
            let lower = select(
                -settings.maximum_velocity - velocity,
                -settings.maximum_velocity,
                velocity,
            );
            self.velocity = select(-lower, lower, 0.0);
            self.previous_animation_height = f32::MAX;
            self.position = input.physical_com_height;
        }
        if self.active {
            let mut spring = 0.0;
            let mut damping = 0.0;
            let mut blend = 0.0;
            let mut release = false;
            let mut allow_upwards = true;
            match self.kind {
                0 => {
                    spring = settings.ground_spring;
                    damping = settings.ground_damping;
                    allow_upwards = false;
                    blend = 1.0;
                    release = input.filtered_state != 1 || input.flags_2476 & 0x1000_0000 == 0;
                }
                1 => {
                    spring = settings.grind_spring;
                    damping = settings.grind_damping;
                    blend = settings.grind_blend.evaluate(self.time);
                    release = input.filtered_state != 3;
                }
                2 => {
                    spring = settings.manual_spring;
                    damping = settings.manual_damping;
                    blend = settings.manual_blend.evaluate(self.time);
                    release = input.filtered_state != 1;
                }
                3 => {
                    blend = 1.0;
                    release = input.filtered_state != 1
                        || self.time > settings.coffin_time
                        || !coffin_flag;
                }
                _ => {}
            }
            let animation_height = input.animation_com_height;
            if self.kind == 3 {
                let ratio = self.velocity.abs() / settings.coffin_maximum_velocity;
                let positive_ratio = select(-ratio, 0.0, ratio);
                let scale = select(1.0 - positive_ratio, positive_ratio, 1.0);
                let height = settings
                    .coffin_height
                    .evaluate(self.time / settings.coffin_time);
                self.position = height.mul_add(scale, settings.coffin_base_height);
            } else {
                let mut target = settings.desired_com_height;
                if self.kind == 1 {
                    if self.time > settings.grind_animation_target_time {
                        let minimum = self.desired_grind_com - settings.grind_target_delta;
                        let maximum = self.desired_grind_com + settings.grind_target_delta;
                        let lower = select(minimum - animation_height, minimum, animation_height);
                        target = select(maximum - lower, lower, maximum);
                    }
                    self.desired_grind_com = target;
                }
                let displacement = self.position - target;
                let previous_velocity = self.velocity;
                // fnmsubs of damping*velocity and negative spring displacement.
                let spring_force = -(displacement * spring);
                let acceleration = -(previous_velocity.mul_add(damping, -spring_force));
                let candidate = acceleration.mul_add(STEP, previous_velocity);
                let lower = select(
                    -settings.maximum_velocity - candidate,
                    -settings.maximum_velocity,
                    candidate,
                );
                let maximum = if allow_upwards {
                    settings.maximum_velocity
                } else {
                    0.0
                };
                self.velocity = select(maximum - lower, lower, maximum);
                let integrated =
                    ((self.velocity + previous_velocity) * 0.5).mul_add(STEP, displacement);
                let height = integrated + target;
                self.position = select(
                    height - settings.minimum_height,
                    height,
                    settings.minimum_height,
                );
            }
            let mut displacement = animation_height - self.position;
            if first_ground_update {
                let correction = self.velocity * f32::from_bits(0x3C75_C28F);
                let lower = select(-0.1 - correction, -0.1, correction);
                let down = select(-lower, lower, 0.0);
                let combined = down + displacement;
                displacement = select(combined, combined, 0.0);
            }
            if self.kind == 0
                && self.time > settings.ground_minimum_compression_time
                && self.previous_animation_height < animation_height
                && displacement > 0.0
            {
                release = true;
            }
            if self.kind == 1 && input.flags_2468 & 0x20 != 0 {
                release = true;
            }
            if self.time > 1.5 {
                release = true;
            }
            let force_blend = input.flags_2468 & (1 << 21) != 0;
            if !release && blend >= 0.05 || force_blend {
                let mut frames = if self.kind == 3 {
                    settings.coffin_blend_frames
                } else {
                    15.0
                };
                if force_blend {
                    self.active = false;
                    frames = 6.0;
                }
                offset.refresh_height(displacement * blend, frames);
            } else {
                self.active = false;
            }
            self.time += STEP;
            self.previous_animation_height = animation_height;
        }
        self.previous_com_velocity = input.physical_com_velocity_along_up;
        self.previous_filtered_state = input.filtered_state;
    }
}
fn select(selector: f32, positive: f32, negative: f32) -> f32 {
    if selector >= 0.0 { positive } else { negative }
}

#[cfg(test)]
#[path = "tests/skeleton_landing.rs"]
mod tests;
