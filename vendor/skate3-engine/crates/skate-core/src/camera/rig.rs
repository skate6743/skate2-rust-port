//! Native normal-camera rig update82E03300. The rig owns one copy of each
//! shared field; operation adapters below cannot retain inconsistent snapshots.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Breadcrumbs {
    pub positions: Vec<[f32; 4]>,
    pub index: usize,
    pub distance_squared_threshold: f32,
}
impl Breadcrumbs {
    fn last(&self) -> [f32; 4] {
        self.positions[(self.positions.len() + self.index - 1) % self.positions.len()]
    }
    fn update(&mut self, position: [f32; 4]) {
        let delta = core::array::from_fn(|i| position[i] - self.positions[self.index][i]);
        if super::vector_tracker::dot(delta, delta) > self.distance_squared_threshold {
            self.index += 1;
            if self.index >= self.positions.len() {
                self.index = 0;
            }
            self.positions[self.index] = position;
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigSettings {
    pub heading: AngleTrackingSettings,
    pub elevation: AngleTrackingSettings,
    pub anchor: AnchorTrackingSettings,
    pub avoidance: AvoidanceSettings,
    pub positioning: RigPositioningSettings,
    /// Parameter620 and native global830BD350 respectively.
    pub collision_hold_duration: f32,
    pub normalization_threshold: [f32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigFields {
    pub anchor: [f32; 4],
    pub velocity: [f32; 4],
    pub acceleration: [f32; 4],
    pub offset: [f32; 4],
    pub heading_target: f32,
    pub heading_reference: f32,
    pub elevation_reference: f32,
    pub elevation_target: f32,
    pub elevation_offset: f32,
    pub additional_elevation: f32,
    pub heading_offset: f32,
    pub avoidance_heading: f32,
    pub avoidance_elevation: f32,
    pub smoothed_acceleration: f32,
    pub previous_acceleration: f32,
    pub collision_hold_time: f32,
    pub reset_time: f32,
    pub disable_avoidance_time: f32,
    pub height_transition_start: f32,
    pub height_transition_end: f32,
    pub height_transition_weight: f32,
    pub height_transition_duration: f32,
    pub height_transition_time: f32,
    pub reference_height: f32,
    pub previous_height: f32,
    pub height_offset: f32,
    pub latch_time: f32,
    pub input_time: f32,
    pub distance: f32,
    pub heading_smoothing: f32,
    pub elevation_smoothing: f32,
    pub heading_weight: f32,
    pub elevation_weight: f32,
    pub mode_time: f32,
    pub frame_counter: u32,
    pub collision_mode: u32,
    pub avoidance_mode: u32,
    pub height_mode: u32,
    pub previous_height_mode: u32,
    pub flags_516: u8,
    pub flags_517: u8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Rig {
    pub fields: RigFields,
    pub heading: AngleTracker,
    pub heading_parameters: ScalarTrackerParameters,
    pub elevation: AngleTracker,
    pub elevation_parameters: ScalarTrackerParameters,
    pub anchor: VectorTracker,
    pub anchor_parameters: ScalarTrackerParameters,
    pub positioner: Positioner,
    pub paths: [AvoidancePath; 3],
    pub breadcrumbs: Breadcrumbs,
}
impl Rig {
    pub fn update<S>(
        &mut self,
        dt: f32,
        settings: RigSettings,
        subject: &mut S,
        requests: &mut [impl TrajectoryCollisionRequest; 3],
        moving: &mut impl MovingObstacleProvider,
        collision: &mut impl PositionerCollisionProvider,
    ) where
        S: RigModeSubject
            + AvoidanceSubject
            + ReferenceHeightSubject
            + AnchorTrackingSubject
            + RigPositioningSubject,
    {
        self.fields.reset_time = floor_zero(self.fields.reset_time - dt);
        self.fields.disable_avoidance_time = floor_zero(self.fields.disable_avoidance_time - dt);
        self.fields.frame_counter = self.fields.frame_counter.wrapping_add(1);
        let mut mode = RigMode {
            current: self.fields.height_mode,
            previous: self.fields.previous_height_mode,
            elapsed: self.fields.mode_time,
        };
        mode.update(subject);
        self.fields.height_mode = mode.current;
        self.fields.previous_height_mode = mode.previous;
        self.fields.mode_time = mode.elapsed;
        let f = self.fields;
        let mut avoidance = RigAvoidance {
            anchor: f.anchor,
            velocity: f.velocity,
            tracked_anchor: self.anchor.position,
            heading_target: f.heading_target,
            heading_reference: f.heading_reference,
            elevation_target: f.elevation_target,
            avoidance_heading: f.avoidance_heading,
            avoidance_elevation: f.avoidance_elevation,
            reset_time: f.reset_time,
            disable_time: f.disable_avoidance_time,
            height_transition_start: f.height_transition_start,
            height_transition_end: f.height_transition_end,
            height_transition_duration: f.height_transition_duration,
            height_transition_time: f.height_transition_time,
            reference_height: f.reference_height,
            prediction_distance: f.distance,
            heading_weight: f.heading_weight,
            elevation_weight: f.elevation_weight,
            avoidance_mode: f.avoidance_mode,
            height_mode: f.height_mode,
            flags_516: f.flags_516,
            paths: self.paths,
        };
        avoidance.update(
            dt,
            settings.avoidance,
            &self.positioner,
            self.breadcrumbs.last(),
            subject,
            requests,
            moving,
        );
        self.paths = avoidance.paths;
        self.fields.avoidance_heading = avoidance.avoidance_heading;
        self.fields.avoidance_elevation = avoidance.avoidance_elevation;
        self.fields.heading_weight = avoidance.heading_weight;
        self.fields.elevation_weight = avoidance.elevation_weight;
        self.fields.flags_516 = avoidance.flags_516;
        let f = self.fields;
        let mut height = ReferenceHeightTracking {
            anchor_height: f.anchor[1],
            transition_start: f.height_transition_start,
            transition_end: f.height_transition_end,
            transition_weight: f.height_transition_weight,
            transition_duration: f.height_transition_duration,
            transition_time: f.height_transition_time,
            height: f.reference_height,
            previous_height: f.previous_height,
            height_offset: f.height_offset,
            mode: f.height_mode,
            flags_516: f.flags_516,
            flags_517: f.flags_517,
        };
        height.update(dt, subject);
        self.fields.height_transition_duration = height.transition_duration;
        self.fields.height_transition_time = height.transition_time;
        self.fields.reference_height = height.height;
        self.fields.previous_height = height.previous_height;
        self.fields.height_offset = height.height_offset;
        let mut angular = self.angular();
        angular.update_heading(dt, settings.heading);
        angular.update_elevation(dt, settings.elevation);
        self.store_angular(angular);
        let f = self.fields;
        let square = super::vector_tracker::dot(f.velocity, f.velocity);
        let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.0), inverse);
        }
        let magnitude = if square == 0.0 { 0.0 } else { square * inverse };
        // Mapped TU3 read-only constant82139A30.
        let fallback = [0.0, 0.0, 1.0, 0.0];
        let direction = core::array::from_fn(|i| {
            if magnitude > settings.normalization_threshold[i] {
                f.velocity[i] * inverse
            } else {
                fallback[i]
            }
        });
        let acceleration = super::vector_tracker::dot(f.acceleration, direction);
        self.fields.smoothed_acceleration = (f.previous_acceleration + acceleration) * 0.5;
        self.fields.previous_acceleration = acceleration;
        let mut anchor = AnchorRigTracking {
            tracker: self.anchor,
            parameters: self.anchor_parameters,
            anchor: f.anchor,
            velocity: f.velocity,
            reference_height: f.reference_height,
            latch_time: f.latch_time,
            input_time: f.input_time,
            flags_516: f.flags_516,
            flags_517: f.flags_517,
        };
        anchor.update(dt, settings.anchor, subject);
        self.anchor = anchor.tracker;
        self.anchor_parameters = anchor.parameters;
        self.fields.latch_time = anchor.latch_time;
        self.fields.input_time = anchor.input_time;
        self.fields.flags_516 = anchor.flags_516;
        self.fields.collision_hold_time = if self.positioner.flags & 0x80 != 0 {
            settings.collision_hold_duration
        } else {
            floor_zero(self.fields.collision_hold_time - dt)
        };
        let mut breadcrumb = self.anchor.position;
        breadcrumb[1] = self.fields.reference_height;
        self.breadcrumbs.update(breadcrumb);
        let mut framing = self.framing();
        let mut angular = self.angular();
        framing.update(
            dt,
            &mut angular,
            self.anchor.position,
            &self.positioner,
            settings.normalization_threshold,
        );
        self.fields.anchor = framing.anchor;
        self.fields.distance = framing.distance;
        let f = self.fields;
        let mut positioning = RigPositioning {
            heading_reference: f.heading_reference,
            elevation_reference: f.elevation_reference,
            avoidance_heading: f.avoidance_heading,
            avoidance_elevation: f.avoidance_elevation,
            elevation_weight: f.elevation_weight,
            collision_mode: f.collision_mode,
            flags_516: f.flags_516,
        };
        positioning.update(
            dt,
            settings.positioning,
            &mut angular,
            &framing,
            self.anchor.position,
            &mut self.positioner,
            &mut self.breadcrumbs.positions,
            subject,
            collision,
        );
        self.store_angular(angular);
        self.fields.heading_reference = positioning.heading_reference;
        self.fields.elevation_reference = positioning.elevation_reference;
        self.fields.avoidance_heading = positioning.avoidance_heading;
        self.fields.avoidance_elevation = positioning.avoidance_elevation;
        self.fields.elevation_weight = positioning.elevation_weight;
        self.fields.flags_516 = positioning.flags_516;
    }
    pub(super) fn angular(&self) -> AngularRigTracking {
        let f = self.fields;
        AngularRigTracking {
            heading: self.heading,
            heading_parameters: self.heading_parameters,
            elevation: self.elevation,
            elevation_parameters: self.elevation_parameters,
            heading_target: f.heading_target,
            elevation_target: f.elevation_target,
            heading_offset: f.heading_offset,
            reset_time: f.reset_time,
            heading_smoothing: f.heading_smoothing,
            elevation_smoothing: f.elevation_smoothing,
            heading_acceleration_state: f.heading_weight,
            flags_517: f.flags_517,
        }
    }
    pub(super) fn store_angular(&mut self, a: AngularRigTracking) {
        self.heading = a.heading;
        self.heading_parameters = a.heading_parameters;
        self.elevation = a.elevation;
        self.elevation_parameters = a.elevation_parameters;
        self.fields.heading_target = a.heading_target;
        self.fields.elevation_target = a.elevation_target;
        self.fields.heading_weight = a.heading_acceleration_state;
        self.fields.flags_517 = a.flags_517;
    }
    fn framing(&self) -> RigFraming {
        let f = self.fields;
        RigFraming {
            anchor: f.anchor,
            offset: f.offset,
            elevation_offset: f.elevation_offset,
            additional_elevation: f.additional_elevation,
            reference_height: f.reference_height,
            distance: f.distance,
        }
    }
}
fn floor_zero(value: f32) -> f32 {
    if -value >= 0.0 { 0.0 } else { value }
}
