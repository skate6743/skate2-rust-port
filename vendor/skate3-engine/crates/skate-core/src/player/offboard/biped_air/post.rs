//! Air Post82D2FFF8 and its two approved offboard checks; canonical request storage.
use super::{DT, Vector, height::dot};
use crate::player::wipeout::{Frame, Mode, Requests};
#[path = "post/checks.rs"]
pub mod checks;
pub use checks::Settings;
#[cfg(test)]
#[path = "post/tests.rs"]
mod tests;
#[derive(Clone, Copy, Debug)]
pub struct PostInput {
    pub flags_2484: u32,
    pub state_timer_2664: f32,
    pub forward_224: Vector,
    pub side_192: Vector,
    pub input_2708: f32,
    pub input_2704: f32,
}
impl super::State {
    ///Caller synchronizes the canonical landing manager immediately before this.
    pub fn post_physics(
        &self,
        input: PostInput,
        settings: &Settings,
        frame: &Frame,
        mode: Mode,
        root_velocity: Vector,
        requests: &mut Requests,
    ) {
        if input.flags_2484 & 2 != 0 {
            checks::skeleton_air(requests, settings, mode, frame);
        }
        if !self.result.valid_404 && input.state_timer_2664 > f32::from_bits(0x3ff33333) {
            requests.request(26, 0.);
        }
        if self.flags_544_550[3] {
            requests.request(30, 0.);
        }
        checks::offboard_air(requests, settings, mode, frame, root_velocity);
        if input.flags_2484 & 0x400 != 0 && self.time_remaining_444 > DT {
            requests.request(31, 0.);
        }
        if self.flags_544_550[5] && self.result.velocity_288[1] < 0.0 {
            requests.request(31, 0.);
        }
        if self.result.valid_404
            && -dot(self.result.normal_304, self.result.contact_velocity_320)
                > f32::from_bits(0x41473333)
            && self.time_remaining_444 < f32::from_bits(0x3e23d70a)
        {
            requests.request(33, 0.);
        }
        if self.time_remaining_444 <= 0.0 && !self.flags_544_550[6] {
            let forward = dot(input.forward_224, self.result.contact_velocity_320);
            let side = dot(input.side_192, self.result.contact_velocity_320);
            //82D30248 compares forward again. The decompiler aliases both.
            if forward > 20.0 || side > 4.0 || forward < -2.0 {
                requests.request(31, 0.);
            }
        }
        if self.time_remaining_444 < 0.0 && self.flags_544_550[4] {
            requests.request(31, 0.);
        }
        if self.time_remaining_444 > 4.0
            && (input.input_2708.abs() > 0.01 || input.input_2704.abs() > 0.01)
        {
            requests.request(26, 0.);
        }
    }
}
