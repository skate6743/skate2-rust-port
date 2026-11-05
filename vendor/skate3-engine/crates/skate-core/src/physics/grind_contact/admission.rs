//! S3 82D886B8, paired with S2 IsEntryAngleAcuteEnough 82DDA748.
//! Test each family/contact before arbitration, retaining its entry kind.
use super::{V, arithmetic, dot3, scale, sub, within_approach_angle};
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum EntryKind {
    RideFromAbove = 0,
    RideFromBelow = 1,
    RideIntoCoping = 2,
    StayInGrind = 3,
    ChangeGrind = 4,
    AirToGrind = 5,
    DropIn = 6,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub kind: EntryKind,
    pub allowed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Admission<'a> {
    pub category: u32,
    /// The current processed state2508, not the requested/previous state.
    pub state: u32,
    pub speed: f32,
    pub velocity: V,
    /// physics_grinds/DVEntryThreshScalarVsSinSlope, X480/Y496.
    pub threshold_vs_slope: &'a PointGraph<4>,
}

impl Admission<'_> {
    pub fn test(&self, requested_state: u32, tangent: V, allows_drop_in: bool) -> Decision {
        let kind = if self.category == 400 || self.state == 701 {
            if self.state == requested_state || requested_state == 402 && self.state == 404 {
                EntryKind::StayInGrind
            } else {
                EntryKind::ChangeGrind
            }
        } else if self.category == 100 {
            if allows_drop_in && self.speed < 1.45 {
                EntryKind::DropIn
            } else {
                let slope = approach_slope_sine(tangent, self.velocity);
                if slope > 0.96 {
                    EntryKind::RideIntoCoping
                } else if slope > 0.25 {
                    EntryKind::RideFromBelow
                } else {
                    EntryKind::RideFromAbove
                }
            }
        } else {
            EntryKind::AirToGrind
        };
        let allowed = match kind {
            EntryKind::RideFromAbove => within_approach_angle(tangent, self.velocity, 17.0),
            EntryKind::StayInGrind => within_approach_angle(tangent, self.velocity, 90.0),
            EntryKind::ChangeGrind => within_approach_angle(tangent, self.velocity, 24.0),
            EntryKind::AirToGrind | EntryKind::DropIn => true,
            EntryKind::RideFromBelow | EntryKind::RideIntoCoping => {
                let square_speed = dot3(self.velocity, self.velocity);
                let transverse = sub(scale(tangent, dot3(self.velocity, tangent)), self.velocity);
                let square_transverse = dot3(transverse, transverse);
                let ratio = if square_speed <= 0.002 {
                    1.0
                } else {
                    arithmetic::square_root(square_transverse / square_speed)
                };
                let slope = approach_slope_sine(tangent, self.velocity);
                let multiplier = self.threshold_vs_slope.evaluate(slope);
                let limit = (1.0 - ratio).mul_add(2.4, 2.8) * multiplier;
                square_transverse < limit * limit
            }
        };
        Decision { kind, allowed }
    }
}

/// S3 82D861D8 / S2 CalcApproachSlopeSine 82DDA1A8.
/// This is the vertical component of normalized cross-rail velocity, not
/// rail incline or the board's forward/rail angle.
pub fn approach_slope_sine(tangent: V, velocity: V) -> f32 {
    slope_sine(tangent, velocity, 0.01)
}

/// S3 82D860F8 uses a smaller dead zone for coping and engagement.
pub fn engagement_slope_sine(tangent: V, velocity: V) -> f32 {
    slope_sine(tangent, velocity, 0.001)
}

fn slope_sine(tangent: V, velocity: V, dead_zone: f32) -> f32 {
    let transverse = sub(velocity, scale(tangent, dot3(velocity, tangent)));
    let length = arithmetic::square_root(dot3(transverse, transverse));
    if length <= dead_zone {
        0.0
    } else {
        arithmetic::reciprocal(length) * transverse[1]
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;
