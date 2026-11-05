//! SettingBodyTilt Update82BA88A0 and CreateInstance82BA8B40.
use crate::point_graph::PointGraph;

#[derive(Clone, Debug)]
pub struct Settings {
    pub body_spin_factor: PointGraph<4>,
    pub ground_velocity: f32,
    pub ground_acceleration: f32,
    pub air_velocity: f32,
    pub air_acceleration: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Physical {
    ///PhysOutAnimation vector+0 X, consumed with mirrored stance sign.
    pub lateral_tilt: f32,
    ///Expanded PhysOut bundle+8 scalar204; the curve consumes its magnitude.
    pub body_spin_speed: f32,
    pub filtered_category: u32,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    value: f32,
    velocity: f32,
    was_enabled: bool,
}
impl State {
    pub fn disable(&mut self) {
        self.was_enabled = false;
    }
    pub fn update(
        &mut self,
        enabled: bool,
        mirrored: bool,
        p: Physical,
        s: &Settings,
    ) -> Option<f32> {
        if !self.was_enabled && enabled {
            self.value = 0.0;
            self.velocity = 0.0;
        }
        self.was_enabled = enabled;
        if !enabled {
            return None;
        }
        //82B97140 tests fullSkaterAnim15180 bit30. The nonmirrored branch
        //flips the sign bit of the source X component before multiplication.
        let tilt = if mirrored {
            p.lateral_tilt
        } else {
            f32::from_bits(p.lateral_tilt.to_bits() ^ 0x80000000)
        };
        let target = s.body_spin_factor.evaluate(p.body_spin_speed.abs()) * tilt;
        let (velocity, acceleration) = if p.filtered_category == 2 {
            (s.air_velocity, s.air_acceleration)
        } else {
            (s.ground_velocity, s.ground_acceleration)
        };
        let desired = bound(target - self.value, -velocity, velocity);
        let change = bound(desired - self.velocity, -acceleration, acceleration);
        self.velocity += change;
        self.value += self.velocity;
        Some(self.value)
    }
}
fn bound(value: f32, lo: f32, hi: f32) -> f32 {
    let lower = if lo - value >= 0.0 { lo } else { value };
    if hi - lower >= 0.0 { lower } else { hi }
}
