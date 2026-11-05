//! Second helper's Reset91BF8, Update91C98, response91DF8 and controls92160.
use super::{ContactResponseInput, apply_velocity_delta, reject};
use crate::{
    physics::{native_arithmetic, skeleton_body::SkeletonBody},
    player::wipeout_state::math::*,
};

pub(super) struct Response {
    normal: V,                 //16
    previous_velocity: V,      //32
    velocity: V,               //48
    frames_since_contact: i32, //64
    active_frames: i32,        //68
    contact_latched: bool,     //72, reset-owned, not cleared each update
    pub active: bool,          //73, explicitly preserved by native Reset
}
impl Response {
    pub fn new() -> Self {
        //The original constructors/reset omit initialization of byte73. Host
        //storage begins false; subsequent native resets preserve its value.
        Self {
            normal: [0.0, 1.0, 0.0, 0.0],
            previous_velocity: [0.0; 4],
            velocity: [0.0; 4],
            frames_since_contact: 100,
            active_frames: 0,
            contact_latched: false,
            active: false,
        }
    }
    pub fn reset(&mut self) {
        let active = self.active;
        *self = Self::new();
        self.active = active;
    }
    pub fn update(&mut self, body: &mut SkeletonBody, input: ContactResponseInput) {
        self.velocity = input.com_velocity_608;
        if dot(self.velocity, self.velocity) > 0.25 {
            if let Some(normal) = input.material11_normal {
                self.frames_since_contact = 0;
                self.contact_latched = true;
                self.normal = normal;
                if !self.active {
                    self.previous_velocity = reject(self.previous_velocity, self.normal);
                }
            }
        }
        self.active = self.contact_latched || self.frames_since_contact < 3;
        if self.active {
            self.apply_response(body);
            self.apply_control(
                body,
                input.effective_axis_224,
                input.wipeout_control_2824_2828,
            );
            self.active_frames = self.active_frames.wrapping_add(1);
        } else {
            self.previous_velocity = self.velocity;
            self.active_frames = 0;
        }
        //Native signed comparison intentionally alternates100->101->100.
        self.frames_since_contact = if self.frames_since_contact <= 100 {
            self.frames_since_contact.wrapping_add(1)
        } else {
            100
        };
    }
    fn apply_response(&self, body: &mut SkeletonBody) {
        let mut retained_delta = [0.0; 4];
        if self.active_frames < 3 {
            let difference = reject(sub(self.previous_velocity, self.velocity), self.normal);
            if dot(difference, self.velocity) > 0.0 {
                retained_delta = scale(difference, f32::from_bits(0x3f66_6666));
            }
        }
        let friction = reject(
            scale(self.velocity, f32::from_bits(0xbb03_126f)),
            self.normal,
        );
        let mut slope_delta = [0.0; 4];
        //91F9C bge skips the slope term at/above original normal.y limit0.97.
        if self.normal[1] < f32::from_bits(0x3f78_51ec) {
            let across = cross([0.0, 1.0, 0.0, 0.0], self.normal);
            let mut direction = normalize_or(cross(across, self.normal), [0.0; 4]);
            if direction[1] > 0.0 {
                direction = scale(direction, -1.0);
            }
            //92058 loads retained NORMAL; only its Y is zeroed before length.
            let mut horizontal_normal = self.normal;
            horizontal_normal[1] = 0.0;
            let candidate = scale(
                direction,
                length(horizontal_normal) * f32::from_bits(0x3dcc_cccd),
            );
            if dot(candidate, self.velocity) > 0.0 {
                slope_delta = candidate;
            }
        }
        apply_velocity_delta(body, add(add(friction, retained_delta), slope_delta));
    }
    fn apply_control(&self, body: &mut SkeletonBody, effective_axis: V, control: [f32; 2]) {
        let planar_velocity = reject(self.velocity, self.normal);
        let planar_direction = normalize_or(planar_velocity, [0.0; 4]);
        if f32::from_bits(0x3f7f_be77) > dot([0.0, 1.0, 0.0, 0.0], effective_axis) {
            let fraction =
                reciprocal(10.0) * native_arithmetic::vector_min(10.0, length(planar_velocity));
            let input = [control[0], 0.0, control[1], 0.0];
            let tangent = reject(input, self.normal);
            let lateral = reject(tangent, planar_direction);
            apply_velocity_delta(body, scale(lateral, fraction * f32::from_bits(0x3d4c_cccd)));
        }
    }
}
