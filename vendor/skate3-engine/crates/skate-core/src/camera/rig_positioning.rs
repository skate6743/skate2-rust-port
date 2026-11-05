//! TU3 rig-to-positioner operation82E037F0. Authored settings and the subject
//! adapter are inputs; the world provider retains the positioner's six-line ABI.
use super::{
    AngularRigTracking, Positioner, PositionerCollisionProvider, PositionerConfig, RigFraming,
};

pub trait RigPositioningSubject {
    fn flag_668(&mut self) -> u8;
    fn flag_548(&mut self) -> u8;
    fn flag_660(&mut self) -> u8;
    fn value_524(&mut self) -> f32;
    fn transform_384(&mut self) -> [[f32; 4]; 4];
    fn position_360(&mut self, index: u32) -> [f32; 4];
    fn context_532(&mut self) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistanceTrackingSettings {
    pub speed_clamp: f32,
    pub smoothing: f32,
    pub acceleration_clamp: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigPositioningSettings {
    /// Parameter fields536/540/544 and548/552/556, selected by subject668.
    pub normal: DistanceTrackingSettings,
    pub alternate: DistanceTrackingSettings,
    /// Parameter fields908 and912, selected by subject548.
    pub minimum_distance: f32,
    pub alternate_minimum_distance: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigPositioning {
    pub heading_reference: f32,
    pub elevation_reference: f32,
    pub avoidance_heading: f32,
    pub avoidance_elevation: f32,
    pub elevation_weight: f32,
    pub collision_mode: u32,
    pub flags_516: u8,
}
impl RigPositioning {
    pub fn update(
        &mut self,
        dt: f32,
        settings: RigPositioningSettings,
        angular: &mut AngularRigTracking,
        framing: &RigFraming,
        tracked_anchor: [f32; 4],
        positioner: &mut Positioner,
        breadcrumbs: &mut [[f32; 4]],
        subject: &mut impl RigPositioningSubject,
        collision: &mut impl PositionerCollisionProvider,
    ) {
        let alternate = subject.flag_668() != 0;
        let mut anchor = tracked_anchor;
        anchor[1] = framing.reference_height;
        let reference_heading = angular.current_heading();
        let reference_elevation =
            clamp_elevation(framing.elevation_offset + angular.elevation.0.position);
        let heading = angular.current_heading();
        let elevation = clamp_elevation(
            (framing.additional_elevation + framing.elevation_offset)
                + angular.elevation.0.position,
        );
        let distance = if framing.distance.is_nan() {
            1.0
        } else {
            framing.distance
        };
        let minimum_distance = if subject.flag_548() != 0 {
            settings.alternate_minimum_distance
        } else {
            settings.minimum_distance
        };
        let clamp_height = subject.flag_660();
        let floor_height = subject.value_524();
        let tracking = if alternate {
            settings.alternate
        } else {
            settings.normal
        };
        let up = subject.transform_384()[1];
        let first = subject.position_360(3);
        let second = subject.position_360(0);
        let third = subject.position_360(7);
        let third = core::array::from_fn(|i| up[i].mul_add(f32::from_bits(0x3e19999a), third[i]));
        let config = PositionerConfig {
            anchor,
            offset: framing.offset,
            reference_heading,
            reference_elevation,
            heading,
            elevation,
            distance,
            minimum_distance,
            collision_enabled: u8::from(self.collision_mode != 2),
            bypass_distance_tracker: u8::from(angular.flags_517 & 0x40 != 0),
            clamp_height,
            floor_height,
            speed_clamp: tracking.speed_clamp,
            acceleration_clamp: tracking.acceleration_clamp,
            smoothing: tracking.smoothing,
            query_starts: [first, second, third],
        };
        // Native BLE skips reset; unordered reset_time proceeds into the reset.
        if !(angular.reset_time <= 0.0) {
            positioner.distance_tracker.target = distance;
            positioner.distance_tracker.position = distance;
            positioner.distance_tracker.velocity = 0.0;
            positioner.distance_tracker.acceleration = 0.0;
        }
        angular.flags_517 &= !0x40;
        let context = subject.context_532();
        positioner.update(dt, context, config, collision);
        if positioner.flags & 4 == 0 {
            angular.heading_acceleration_state = 1.0;
            self.elevation_weight = 1.0;
            self.avoidance_heading = positioner.heading;
            self.avoidance_elevation = positioner.elevation;
            breadcrumbs.fill(positioner.position);
            self.heading_reference = positioner.heading;
            angular.heading_target = blend_angle(
                self.avoidance_heading,
                self.heading_reference,
                1.0 - angular.heading_acceleration_state,
            );
            self.set_elevation(positioner.elevation, true, angular, positioner.flags);
            angular.reset_heading(true);
            angular.reset_elevation(true);
            self.flags_516 |= 8;
        }
    }

    /// Complete82E03150, including the optional avoidance-angle blend.
    pub fn set_elevation(
        &mut self,
        elevation: f32,
        avoidance: bool,
        angular: &mut AngularRigTracking,
        positioner_flags: u8,
    ) {
        angular.elevation_target = elevation;
        self.elevation_reference = elevation;
        if avoidance {
            self.flags_516 |= (positioner_flags >> 4) & 1;
            if self.flags_516 & 1 != 0 {
                angular.elevation_target = blend_angle(
                    angular.elevation_target,
                    self.avoidance_elevation,
                    self.elevation_weight,
                );
            }
        }
        angular.elevation_target = super::rig_tracking::wrap_vmx(angular.elevation_target);
    }
}
/// Complete82DF3410: wrap delta, fused interpolation, then wrap result.
fn blend_angle(current: f32, target: f32, weight: f32) -> f32 {
    super::rig_tracking::wrap_vmx(
        weight.mul_add(super::rig_tracking::wrap_vmx(target - current), current),
    )
}
fn clamp_elevation(value: f32) -> f32 {
    let maximum = f32::from_bits(0x3fb2b8c2);
    let value = if -maximum - value >= 0.0 {
        -maximum
    } else {
        value
    };
    if maximum - value >= 0.0 {
        value
    } else {
        maximum
    }
}
