//! Complete TU3 framing82E03560 and subject-driven mode selection82E053E0.
use super::{AngularRigTracking, Positioner, direction_to_angles};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigFraming {
    pub anchor: [f32; 4],
    pub offset: [f32; 4],
    pub elevation_offset: f32,
    pub additional_elevation: f32,
    pub reference_height: f32,
    pub distance: f32,
}

impl RigFraming {
    /// The threshold is the native global vector830BD350. Its runtime producer
    /// must be bound by the caller; mapped-image zeros are not a tuning default.
    pub fn update(
        &mut self,
        dt: f32,
        angular: &mut AngularRigTracking,
        tracked_anchor: [f32; 4],
        positioner: &Positioner,
        normalization_threshold: [f32; 4],
    ) {
        if angular.flags_517 & 0x40 == 0 {
            return;
        }
        let mut camera = positioner.position;
        if positioner.flags & 0x80 == 0 {
            camera = core::array::from_fn(|i| positioner.velocity[i].mul_add(dt, camera[i]));
        }
        let mut anchor = tracked_anchor;
        anchor[1] = self.reference_height;
        let delta = core::array::from_fn(|i| (camera[i] - anchor[i]) - self.offset[i]);
        let square = super::vector_tracker::dot(delta, delta);
        let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.0), inverse);
        }
        let magnitude = if square == 0.0 { 0.0 } else { square * inverse };
        let direction = core::array::from_fn(|i| {
            if magnitude > normalization_threshold[i] {
                delta[i] * inverse
            } else {
                0.0
            }
        });
        let angles = direction_to_angles(direction);
        angular.heading_target = -angles[1] - angular.heading_offset;
        angular.elevation_target = (angles[0] - self.elevation_offset) - self.additional_elevation;
        self.distance = magnitude;
        self.anchor[1] = self.reference_height;
        angular.reset_heading(false);
        angular.reset_elevation(false);
    }
}

pub trait RigModeSubject {
    fn flag_568(&mut self) -> u8;
    fn flag_596(&mut self) -> u8;
    fn flag_680(&mut self) -> u8;
    fn flag_540(&mut self) -> u8;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigMode {
    pub current: u32,
    pub previous: u32,
    pub elapsed: f32,
}
impl RigMode {
    /// Complete82E053E0. No graph topology or skater-state mapping is inferred
    /// from the native getter slots. Preserve their conditional call order.
    pub fn update(&mut self, subject: &mut impl RigModeSubject) {
        let next = if subject.flag_568() != 0 {
            0
        } else if subject.flag_596() != 0 || subject.flag_680() != 0 {
            2
        } else if subject.flag_540() != 0 {
            1
        } else {
            return;
        };
        if next != self.current {
            self.previous = self.current;
            self.current = next;
            self.elapsed = 0.0;
        }
    }
}
