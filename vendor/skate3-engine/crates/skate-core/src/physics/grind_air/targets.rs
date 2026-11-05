use super::*;
pub(super) const CONTACTS: [usize; 7] = [0, 1, 4, 0, 2, 2, 3];
pub(super) const DESIRED: [f32; 7] = [90., 90., 90., 90., 0., 0., 0.];
#[derive(Clone, Debug)]
pub struct Settings {
    pub frames: usize,
    pub stomp: f32,
    pub points: [V; 5],
    pub ranges: [f32; 7],
    pub distances: [f32; 7],
    pub yaw_assist: [f32; 7],
    pub max_offset: f32,
    pub max_delta: f32,
    pub max_angle: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct DeckDimensions {
    pub middle_length: f32,
    pub front_end_size: f32,
    pub front_end_angle_degrees: f32,
    pub truck_z_front: f32,
    pub truck_y: f32,
}
impl DeckDimensions {
    ///82D72B40 uses sine for tip height but adds the unprojected tip length to Z.
    pub fn contact_points(self, tip_fraction: f32) -> [V; 5] {
        let half = self.middle_length * 0.5;
        let tip = self.front_end_size * tip_fraction;
        let y = crate::trigonometry::sin_cos(self.front_end_angle_degrees * DEG).0 * tip;
        let z = half + self.truck_z_front;
        [
            ZERO,
            [0., y, tip + half, 0.],
            [0., self.truck_y, z, 0.],
            [0., self.truck_y, -z, 0.],
            [0., y, -(tip + half), 0.],
        ]
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        // Host data validation, not a gameplay clamp or a fabricated setting.
        if !(2..=12).contains(&self.frames) {
            return Err("Stock GrindAir NumFramesToTest exceeds native 2..12 storage".into());
        }
        Ok(())
    }
}
