//! Complete TU3 normal-camera avoidance82E04C20. Three native PathEvaluators
//! retain separate asynchronous requests. World/subject adapters are external.

use super::vector_tracker::{dot, length, refined_reciprocal};
use super::{
    MovingObstacleProvider, PathEvaluator, Positioner, PredictionPath, TrajectoryCollisionRequest,
    direction_to_angles, position_from_angles,
};

pub trait AvoidanceSubject {
    fn transform_376(&mut self) -> [[f32; 4]; 4];
    fn vector_452(&mut self) -> [f32; 4];
    fn vector_444(&mut self) -> [f32; 4];
    fn context_532(&mut self) -> u32;
    fn flag_608(&mut self) -> u8;
    fn flag_548(&mut self) -> u8;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvoidancePath {
    pub evaluator: PathEvaluator,
    /// Native path byte56, cleared by each avoidance submission.
    pub prediction_flag: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvoidanceSettings {
    /// Parameter offsets628,632,636,640,920.
    pub horizon: f32,
    pub heading_smoothing: f32,
    pub elevation_smoothing: f32,
    pub ease_out_time: f32,
    pub radius_padding: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigAvoidance {
    pub anchor: [f32; 4],
    pub velocity: [f32; 4],
    pub tracked_anchor: [f32; 4],
    pub heading_target: f32,
    pub heading_reference: f32,
    pub elevation_target: f32,
    pub avoidance_heading: f32,
    pub avoidance_elevation: f32,
    pub reset_time: f32,
    pub disable_time: f32,
    pub height_transition_start: f32,
    pub height_transition_end: f32,
    pub height_transition_duration: f32,
    pub height_transition_time: f32,
    pub reference_height: f32,
    pub prediction_distance: f32,
    pub heading_weight: f32,
    pub elevation_weight: f32,
    pub avoidance_mode: u32,
    pub height_mode: u32,
    pub flags_516: u8,
    pub paths: [AvoidancePath; 3],
}

impl RigAvoidance {
    /// Complete82E04C20. The last breadcrumb is supplied from the native ring
    /// selector (capacity+index-1)%capacity; no sampled/fitted trail is generated.
    pub fn update(
        &mut self,
        dt: f32,
        settings: AvoidanceSettings,
        positioner: &Positioner,
        last_breadcrumb: [f32; 4],
        subject: &mut impl AvoidanceSubject,
        requests: &mut [impl TrajectoryCollisionRequest; 3],
        moving: &mut impl MovingObstacleProvider,
    ) {
        if self.avoidance_mode == 1 && self.subject_speed(subject) < 0.5 {
            return;
        }
        if self.flags_516 & 2 != 0 || self.disable_time > 0.0 || self.avoidance_mode == 0 {
            self.heading_weight = 0.0;
            self.elevation_weight = 0.0;
            return;
        }
        match self.avoidance_mode {
            1 => {
                let mut anchor = self.tracked_anchor;
                anchor[1] = self.reference_height;
                let delta = core::array::from_fn(|i| last_breadcrumb[i] - anchor[i]);
                let magnitude = length(delta);
                if f32::from_bits(0x38d1b717) > magnitude {
                    return;
                }
                let inverse = refined_reciprocal(magnitude);
                let angles = direction_to_angles(delta.map(|v| inverse * v));
                self.avoidance_heading = -angles[1];
                self.avoidance_elevation = angles[0];
            }
            2 => {
                let angles = direction_to_angles(subject.vector_452());
                self.avoidance_heading = -angles[1];
                self.avoidance_elevation = angles[0];
            }
            3 => {
                self.avoidance_heading = self.heading_reference;
                self.avoidance_elevation = f32::from_bits(0x3fbde44e);
            }
            4 => {
                self.avoidance_heading =
                    super::rig_tracking::wrap_vmx(self.heading_target + f32::from_bits(0x40490fdb));
                let low = f32::from_bits(0x3e32b8c2);
                self.avoidance_elevation = if low - self.elevation_target >= 0.0 {
                    low
                } else {
                    self.elevation_target
                };
            }
            _ => {}
        }
        if self.avoidance_elevation.is_nan() {
            return;
        }
        let prediction = position_from_angles(
            self.anchor,
            [0.0; 4],
            self.avoidance_elevation,
            self.avoidance_heading,
            self.prediction_distance,
            positioner.config.clamp_height,
            positioner.radius + positioner.config.floor_height,
        );
        let transitioning = self.height_transition_time < self.height_transition_duration;
        let mut velocity = self.velocity;
        if transitioning {
            velocity[1] = (self.height_transition_end - self.height_transition_start)
                / self.height_transition_duration;
        }
        let acceleration_flag = u8::from(self.height_mode == 1 && !transitioning);
        self.paths[0].set(
            PredictionPath {
                // v127 preserves the current positioner position across the
                // prediction helper; its returned point is used by path1.
                position: positioner.position,
                velocity,
                radius: settings.radius_padding + positioner.radius,
                horizon: settings.horizon,
            },
            acceleration_flag,
        );
        let context = subject.context_532();
        self.paths[0]
            .evaluator
            .update(context, &mut requests[0], moving);
        let camera_time = self.paths[0].evaluator.last_valid_time;
        let second_start = if self.height_mode == 1 {
            prediction
        } else {
            [
                self.anchor[0],
                positioner.position[1],
                self.anchor[2],
                self.anchor[0],
            ]
        };
        self.paths[1].set(
            PredictionPath {
                position: second_start,
                velocity,
                radius: f32::from_bits(0x3ca3d70a),
                horizon: settings.horizon,
            },
            acceleration_flag,
        );
        let context = subject.context_532();
        self.paths[1]
            .evaluator
            .update(context, &mut requests[1], moving);
        let third_start = subject.vector_444();
        self.paths[2].set(
            PredictionPath {
                position: third_start,
                velocity,
                radius: f32::from_bits(0x3ca3d70a),
                horizon: settings.horizon,
            },
            acceleration_flag,
        );
        let context = subject.context_532();
        self.paths[2]
            .evaluator
            .update(context, &mut requests[2], moving);
        let second_time = self.paths[1].evaluator.last_valid_time;
        let third_time = self.paths[2].evaluator.last_valid_time;
        let any_acceleration = self.paths[1].evaluator.result_acceleration_flag != 0
            || self.paths[2].evaluator.result_acceleration_flag != 0;
        let other_time = if second_time - third_time >= 0.0 {
            second_time
        } else {
            third_time
        };
        if third_time >= second_time {
            self.flags_516 |= 1;
        }
        let subject_flag = subject.flag_608();
        if subject_flag == 0 && positioner.flags & 0x20 != 0 {
            self.heading_weight = dt * 0.5 + self.heading_weight;
            self.elevation_weight = dt * 0.5 + self.elevation_weight;
        } else if subject_flag == 0
            && self.height_mode != 1
            && (if any_acceleration {
                other_time * 0.5 > camera_time
            } else {
                other_time > camera_time
            })
        {
            let target = clamp(1.0 - camera_time / settings.horizon);
            let heading_target = if self.heading_weight - target >= 0.0 {
                self.heading_weight
            } else {
                target
            };
            let elevation_target = if self.elevation_weight - target >= 0.0 {
                self.elevation_weight
            } else {
                target
            };
            self.heading_weight = (1.0 - settings.heading_smoothing)
                .mul_add(heading_target - self.heading_weight, self.heading_weight);
            self.elevation_weight = (1.0 - settings.elevation_smoothing).mul_add(
                elevation_target - self.elevation_weight,
                self.elevation_weight,
            );
        } else if (self.subject_speed(subject) > f32::from_bits(0x3e19999a)
            && self.reset_time <= 0.0)
            || subject.flag_548() != 0
        {
            self.heading_weight = floor_zero(self.heading_weight - dt / settings.ease_out_time);
            self.elevation_weight = floor_zero(self.elevation_weight - dt / settings.ease_out_time);
            self.flags_516 &= !1;
        }
        self.heading_weight = clamp(self.heading_weight);
        self.elevation_weight = clamp(self.elevation_weight);
    }

    /// Complete82E03060; subject transform+32 is its forward vector.
    fn subject_speed(&self, subject: &mut impl AvoidanceSubject) -> f32 {
        dot(self.velocity, subject.transform_376()[2])
    }
}

impl AvoidancePath {
    fn set(&mut self, path: PredictionPath, acceleration_flag: u8) {
        self.evaluator.path = path;
        self.prediction_flag = 0;
        self.evaluator.acceleration_flag = acceleration_flag;
    }
}
fn floor_zero(v: f32) -> f32 {
    if v >= 0.0 { v } else { 0.0 }
}
fn clamp(v: f32) -> f32 {
    let v = if -v >= 0.0 { 0.0 } else { v };
    if 1.0 - v >= 0.0 { v } else { 1.0 }
}
