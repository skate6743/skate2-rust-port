//! State construction82E02738, reset82E02E58 and teleport82E031D8.
use super::*;

impl Rig {
    pub fn new(avoidance_mode: u32) -> Self {
        let parameters = tracker_parameters();
        let mut rig = Self {
            fields: fields(avoidance_mode),
            heading: AngleTracker(scalar(0.0)),
            heading_parameters: parameters,
            elevation: AngleTracker(scalar(0.0)),
            elevation_parameters: parameters,
            anchor: VectorTracker {
                target: [0.0; 4],
                position: [0.0; 4],
                velocity: [0.0; 4],
                acceleration: [0.0; 4],
                acceleration_clamp: 0.0,
                smoothing: 0.0,
            },
            anchor_parameters: parameters,
            positioner: Positioner {
                distance_tracker: scalar(1.0),
                tracker_parameters: parameters,
                offset: [0.0; 4],
                previous_valid_position: [0.0; 4],
                reference_position: [0.0; 4],
                collision_clear_time: f32::MAX,
                // Host storage for parameters fully supplied by the first rig update.
                config: PositionerConfig {
                    anchor: [0.0; 4],
                    offset: [0.0; 4],
                    reference_heading: 0.0,
                    reference_elevation: 0.0,
                    heading: 0.0,
                    elevation: 0.0,
                    distance: 0.0,
                    minimum_distance: 0.0,
                    collision_enabled: 0,
                    bypass_distance_tracker: 0,
                    clamp_height: 0,
                    floor_height: 0.0,
                    speed_clamp: 0.0,
                    acceleration_clamp: 0.0,
                    smoothing: 0.0,
                    query_starts: [[0.0; 4]; 3],
                },
                position: [0.0; 4],
                velocity: [0.0; 4],
                distance: 0.0,
                heading: 0.0,
                elevation: 0.0,
                radius: f32::from_bits(0x3e051eb8),
                available_distance: 0.0,
                flags: 0,
            },
            paths: [AvoidancePath {
                prediction_flag: 0,
                evaluator: PathEvaluator {
                    request_state: 0,
                    path: PredictionPath {
                        position: [0.0; 4],
                        velocity: [0.0; 4],
                        radius: 0.0,
                        horizon: 0.0,
                    },
                    acceleration_flag: 0,
                    collision_time: f32::MAX,
                    last_valid_time: f32::MAX,
                    found: 0,
                    result_acceleration_flag: 0,
                    submitted_acceleration_flag: 0,
                    pending_polls: 0,
                },
            }; 3],
            breadcrumbs: Breadcrumbs {
                positions: vec![[0.0; 4]; 2],
                index: 0,
                distance_squared_threshold: 1.0,
            },
        };
        rig.reset_angles();
        rig
    }

    /// Reset preserves native subobject history, avoidance mode and the four
    /// low flag bits in517. Orientation is reset separately by its owner.
    pub fn reset(&mut self) {
        let previous = self.fields;
        self.fields = fields(previous.avoidance_mode);
        self.fields.height_transition_weight = previous.height_transition_weight;
        self.fields.flags_517 = (previous.flags_517 & 0x0f) | 0x30;
        self.breadcrumbs.distance_squared_threshold = 1.0;
        self.reset_angles();
    }

    /// Teleport to this frame's native anchor and shot distance.
    pub fn teleport(&mut self) {
        self.fields.avoidance_heading = 0.0;
        self.fields.avoidance_elevation = 0.0;
        self.fields.heading_weight = 0.0;
        self.fields.elevation_weight = 0.0;
        self.fields.flags_516 &= !1;
        self.reset_angles();
        self.anchor.target = self.fields.anchor;
        self.anchor.position = self.fields.anchor;
        self.anchor.velocity = [0.0; 4];
        self.anchor.acceleration = [0.0; 4];
        self.fields.reference_height = self.fields.anchor[1];
        self.fields.height_offset = 0.0;
        //82E03288 clears the preceding frame's world-space shake offset.
        self.fields.offset = [0.0; 4];
        self.positioner.distance_tracker.target = self.fields.distance;
        self.positioner.distance_tracker.position = self.fields.distance;
        self.positioner.distance_tracker.velocity = 0.0;
        self.positioner.distance_tracker.acceleration = 0.0;
        self.fields.reset_time = f32::from_bits(0x3dcccccd);
        self.fields.disable_avoidance_time = 2.0;
        self.fields.collision_hold_time = 5.0;
        self.fields.height_transition_duration = 0.0;
        self.fields.flags_517 &= !0x40;
        self.fields.previous_acceleration = 0.0;
        self.fields.flags_516 = (self.fields.flags_516 & 0xb3) | 8;
        self.fields.input_time = 0.0;
        self.fields.latch_time = 0.0;
    }

    /// SetElevation82E03150: obstruction is latched until the native reset.
    pub fn set_elevation(&mut self, elevation: f32, use_avoidance: bool) {
        self.fields.elevation_target = elevation;
        self.fields.elevation_reference = elevation;
        if use_avoidance {
            self.fields.flags_516 |= u8::from(self.positioner.flags & 0x10 != 0);
            if self.fields.flags_516 & 1 != 0 {
                let delta =
                    super::rig_tracking::wrap_vmx(self.fields.avoidance_elevation - elevation);
                self.fields.elevation_target =
                    super::rig_tracking::wrap_vmx(super::rig_tracking::wrap_vmx(
                        delta.mul_add(self.fields.elevation_weight, elevation),
                    ));
            }
        }
    }

    fn reset_angles(&mut self) {
        let mut angular = self.angular();
        angular.reset_heading(true);
        angular.reset_elevation(true);
        self.store_angular(angular);
    }
}

fn scalar(position: f32) -> ScalarTracker {
    ScalarTracker {
        target: position,
        position,
        velocity: 0.0,
        acceleration: 0.0,
        acceleration_clamp: 0.0,
        smoothing: 0.0,
    }
}
fn tracker_parameters() -> ScalarTrackerParameters {
    ScalarTrackerParameters {
        delta_umbra: 0.0,
        delta_penumbra: 0.0,
        speed_clamp: 0.0,
        acceleration_clamp_min: 0.0,
        acceleration_clamp_max: 0.0,
        smoothing_min: 0.0,
        smoothing_max: 0.0,
        overshoot_zeroes_velocity: true,
    }
}
fn fields(avoidance_mode: u32) -> RigFields {
    RigFields {
        anchor: [0.0; 4],
        velocity: [0.0; 4],
        acceleration: [0.0; 4],
        offset: [0.0; 4],
        heading_target: f32::from_bits(0x40490fdb),
        heading_reference: f32::from_bits(0x40490fdb),
        elevation_reference: 0.0,
        elevation_target: 0.0,
        elevation_offset: 0.0,
        additional_elevation: 0.0,
        heading_offset: 0.0,
        avoidance_heading: 0.0,
        avoidance_elevation: 0.0,
        smoothed_acceleration: 0.0,
        previous_acceleration: 0.0,
        collision_hold_time: 5.0,
        reset_time: 2.0,
        disable_avoidance_time: 0.0,
        height_transition_start: 0.0,
        height_transition_end: 0.0,
        height_transition_weight: 0.0,
        height_transition_duration: 0.0,
        height_transition_time: 0.0,
        reference_height: 1000.0,
        previous_height: f32::MAX,
        height_offset: 0.0,
        latch_time: 0.0,
        input_time: 0.0,
        distance: 1.0,
        heading_smoothing: 1.0,
        elevation_smoothing: 1.0,
        heading_weight: 0.0,
        elevation_weight: 0.0,
        mode_time: 0.0,
        frame_counter: 0,
        collision_mode: 0,
        avoidance_mode,
        height_mode: 0,
        previous_height_mode: 0,
        flags_516: 0x88,
        flags_517: 0x30,
    }
}
