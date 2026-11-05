//! Exact semantic fields of the 184-byte prefix copied by82D31040.
use super::{Frame, IDENTITY, Input, UP, Vector, ZERO, dot, sub};
use crate::air::trajectory::QueryResult;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactPrefix {
    pub position: Vector,        //0
    pub normal: Vector,          //16
    pub support_frame: Frame,    //32..95
    pub target_position: Vector, //96
    pub target_normal: Vector,   //112
    pub edge_position: Vector,   //128
    pub edge_normal: Vector,     //144
    pub scalar_160: f32,
    pub kind_164: u32,
    pub distance_168: f32,
    pub distance_172: f32,
    pub flags_176: u32,
    pub support_180: u32,
}
impl ContactPrefix {
    ///82D2DDD0. Note edge normal144 resets to ZERO, not world up.
    pub const fn reset() -> Self {
        Self {
            position: ZERO,
            normal: UP,
            support_frame: IDENTITY,
            target_position: ZERO,
            target_normal: UP,
            edge_position: ZERO,
            edge_normal: ZERO,
            scalar_160: 0.,
            kind_164: 0,
            distance_168: 1.0e10,
            distance_172: 1.0e10,
            flags_176: 0,
            support_180: 0,
        }
    }
    pub(super) fn consume_support(
        &mut self,
        input: Input,
        hits: &[QueryResult; 3],
        candidate_flags: u32,
    ) {
        let main = hits[0];
        if main.valid() {
            self.position = main.contact_position;
            self.normal = main.landing_normal;
            self.support_frame = main.contact_transform;
            self.flags_176 |= 1;
            if dot(input.up, sub(main.contact_position, input.position)) < -0.1 {
                self.flags_176 |= 8;
                if candidate_flags & 0x40 != 0 {
                    self.flags_176 |= 0x40;
                }
            } else {
                self.flags_176 &= !8;
            }
            self.support_180 = main.geometry;
        }
        for (hit, flag) in [(hits[1], 0x10), (hits[2], 0x20)] {
            //82D85660 strict bounds, no contact-time proximity surrogate.
            let height = dot(sub(hit.contact_position, input.position), input.up);
            if hit.valid() && height < 0.2 && height > -0.2 {
                self.flags_176 |= flag;
                if main.valid() && hit.landing_normal[1] > self.normal[1] {
                    self.normal = hit.landing_normal;
                }
            }
        }
    }
}
impl Default for ContactPrefix {
    fn default() -> Self {
        Self::reset()
    }
}
