//! Original recovery82D3EAE0, request82D3E980, scalar82D3E5E8 and post82D3ED30.
use super::State;
use crate::physics::native_arithmetic::dot3;
const DT: f32 = f32::from_bits(0x3C88_8889);
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub minimum_time: f32,      //physics_wipeout280
    pub minimum_settled: f32,   //284
    pub maximum_time: f32,      //292
    pub fade_time: f32,         //296
    pub over_speed: f32,        //80
    pub over_minimum_time: f32, //84
}
impl State {
    pub fn response_strength(&self) -> f32 {
        let speed = self.maximum_speed * 0.1;
        let count = (4u32.wrapping_sub(self.response_count) as i32) as f32;
        let remaining = (count + 1.0) * 0.5;
        let result = if speed - remaining >= 0.0 {
            speed
        } else {
            remaining
        };
        if self.slow_time > 1.0 || result <= 0.2 || self.special_surface || self.below_surface {
            0.0
        } else {
            result
        }
    }

    pub fn manage_recovery(
        &mut self,
        flags2468: u32,
        flags2472: u32,
        flags2484: u32,
        timestep: f32,
        settings: &Settings,
    ) {
        self.prevent_manual |= flags2468 & 1 != 0;
        self.ignore_reset |= flags2472 & 0x8000_0000 != 0;
        self.reset_ever |= flags2472 & 0x0010_0000 != 0;
        self.ever_settled |= self.over;
        self.recovery_eligible |= self.ever_settled || self.time > 1.6;
        self.settled_time = if self.over {
            self.settled_time + DT
        } else {
            0.0
        };
        if self.teleport_countdown < 0
            && self.should_teleport(flags2472, flags2484, timestep, settings)
        {
            self.teleport_countdown = 2;
        }
        if self.teleport_countdown == 0 {
            self.request_teleport = true;
        } else if self.teleport_countdown > 0 {
            self.teleport_countdown -= 1;
        }
    }

    fn should_teleport(
        &mut self,
        flags2472: u32,
        flags2484: u32,
        timestep: f32,
        settings: &Settings,
    ) -> bool {
        if self.ever_impaled {
            self.impaled_time += DT;
        }
        if flags2484 & 0x0040_0000 != 0 {
            self.ever_impaled = true;
            if self.impaled_time > 0.5 {
                return true;
            }
        }
        if self.prevent_manual {
            return false;
        }
        if self.recovery_eligible && self.reset_ever && !self.ignore_reset {
            return true;
        }
        if self.teleport_pending {
            self.time_until_teleport -= timestep;
            return self.time_until_teleport <= 0.0;
        }
        let automatic = if flags2472 & 0x1000_0000 != 0 {
            self.time > 8.0
        } else {
            (self.time > settings.maximum_time && self.response_time > settings.maximum_time)
                || (self.time > settings.minimum_time
                    && self.settled_time > settings.minimum_settled
                    && self.response_time > settings.minimum_settled)
        };
        if automatic {
            self.teleport_pending = true;
            self.time_until_teleport = settings.fade_time;
        }
        false
    }

    pub fn post_physics(
        &mut self,
        hips_velocity: [f32; 4],
        neck_velocity: [f32; 4],
        support_contact: bool,
        settings: &Settings,
    ) {
        let bypass_neck = self.extra_weight_zero_time > 0.5;
        let speed = if bypass_neck {
            settings.over_speed * 2.0
        } else {
            settings.over_speed
        };
        let minimum_time = self.time > settings.over_minimum_time;
        let hips_squared = dot3(hips_velocity, hips_velocity);
        let neck_squared = dot3(neck_velocity, neck_velocity);
        self.over = hips_squared < speed * speed
            && (neck_squared < speed * speed || bypass_neck)
            && minimum_time
            && self.response_strength() == 0.0;
        self.slow = hips_squared < 1.0 && minimum_time;
        self.no_support_time = if support_contact {
            0.0
        } else {
            self.no_support_time + DT
        };
    }
}
