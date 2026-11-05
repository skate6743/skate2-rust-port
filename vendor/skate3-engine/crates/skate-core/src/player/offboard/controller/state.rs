use super::super::{
    cadence::{BipedCadence, CadenceThresholds},
    contact_correction::ContactCorrection,
    ground_motion::GroundMotionState,
    movement_intent,
    position_output::FrameOutput,
    slide::{Sliding, SpecialMode},
    surface_frame,
};
use super::{Frame, Vector};
pub(super) const ZERO: Vector = [0.0; 4];
pub(super) const UP: Vector = [0.0, 1.0, 0.0, 0.0];
pub(super) const IDENTITY: Frame = [[1.0, 0.0, 0.0, 0.0], UP, [0.0, 0.0, 1.0, 0.0], ZERO];
#[derive(Clone, Copy, Debug)]
pub struct ClipMetric {
    pub translation_z: f32,
    pub end_time: f32,
}
impl ClipMetric {
    fn speed(self) -> f32 {
        (-1.0 / self.end_time) * self.translation_z
    }
}

/// Each persistent original field has one owner. Per-stage packets in step.rs
/// are temporary and copy updated shared fields back before the next producer.
#[derive(Clone, Debug)]
pub struct State {
    pub motion: GroundMotionState,
    pub intent: movement_intent::State,
    pub surface: surface_frame::State,
    pub contact: ContactCorrection,
    pub special: SpecialMode,
    pub sliding: Sliding,
    pub frame_output: FrameOutput,
    pub cadence: BipedCadence,
    pub thresholds: CadenceThresholds,
    pub position_368: Vector,
    pub correction_target_592: Vector,
    pub forward_delta_692: f32,
    pub right_delta_696: f32,
    pub velocity_override_remaining_772: f32,
    pub alternate_709: bool,
}
impl State {
    pub fn new(metrics: [Option<ClipMetric>; 3]) -> Self {
        let speeds = metrics.map(|m| m.map(ClipMetric::speed).unwrap_or(0.0));
        Self {
            motion: reset_motion(ZERO),
            intent: movement_intent::State::default(),
            surface: surface_frame::State::default(),
            contact: ContactCorrection {
                active: false,
                direction: ZERO,
                displacement: ZERO,
            },
            special: SpecialMode {
                enabled_714: false,
                elapsed_788: 0.0,
            },
            sliding: Sliding {
                velocity_528: ZERO,
                active_710: false,
            },
            frame_output: FrameOutput {
                frame: IDENTITY,
                velocity: ZERO,
            },
            cadence: BipedCadence::default(),
            thresholds: CadenceThresholds::from_clip_speeds(speeds[0], speeds[1], speeds[2]),
            position_368: ZERO,
            correction_target_592: ZERO,
            forward_delta_692: 0.0,
            right_delta_696: 0.0,
            velocity_override_remaining_772: -1.0,
            alternate_709: false,
        }
    }
    ///82D7B1C0 deliberately leaves phase716..736 and vectors576/592 alone.
    pub fn reset(&mut self) {
        self.motion = reset_motion(self.motion.correction_576);
        self.intent = movement_intent::State::default();
        self.surface = surface_frame::State::default();
        self.contact = ContactCorrection {
            active: false,
            direction: ZERO,
            displacement: ZERO,
        };
        self.special = SpecialMode {
            enabled_714: false,
            elapsed_788: 0.0,
        };
        self.sliding = Sliding {
            velocity_528: ZERO,
            active_710: false,
        };
        self.frame_output = FrameOutput {
            frame: IDENTITY,
            velocity: ZERO,
        };
        self.position_368 = ZERO;
        self.forward_delta_692 = 0.0;
        self.right_delta_696 = 0.0;
        self.velocity_override_remaining_772 = -1.0;
        self.alternate_709 = false;
    }
}
fn reset_motion(correction_576: Vector) -> GroundMotionState {
    GroundMotionState {
        frame_0: IDENTITY,
        published_frame_64: IDENTITY,
        previous_support_frame_192: IDENTITY,
        support_velocity_256: ZERO,
        predicted_support_velocity_272: ZERO,
        previous_support_velocity_288: ZERO,
        support_acceleration_304: ZERO,
        filtered_local_acceleration_320: ZERO,
        support_yaw_336: 0.0,
        previous_support_yaw_340: 0.0,
        predicted_support_yaw_344: 0.0,
        support_speed_348: 0.0,
        support_id_352: 0,
        velocity_480: ZERO,
        correction_576,
        target_frame_608: IDENTITY,
        angular_velocity_688: 0.0,
        speed_704: 0.0,
        correction_enabled_711: false,
        support_velocity_removed_715: false,
        target_scale_784: -1.0,
    }
}
