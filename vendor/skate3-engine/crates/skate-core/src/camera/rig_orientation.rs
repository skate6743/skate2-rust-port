//! Normal gameplay rig pitch/yaw tracking and roll, TU3 82E045E0.
use super::{
    AngleTracker, ScalarTracker, ScalarTrackerParameters, direction_to_angles, normalize_angle,
};
use crate::{
    math::Basis3,
    physics::rigid_body::{RetailQuaternion, basis_from_quaternion},
    point_graph::PointGraph,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientationTrackerSettings {
    pub acceleration_min_degrees: f32,
    pub acceleration_max_degrees: f32,
    pub smoothing_min: f32,
    pub delta_umbra_degrees: f32,
    pub delta_penumbra_degrees: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientationSettings {
    pub pan_umbra_degrees: f32,
    pub pan_penumbra_degrees: f32,
    pub pan_smoothing_curve: PointGraph<8>,
    pub pan_minimum_smoothing: f32,
    pub pan: OrientationTrackerSettings,
    pub tilt: OrientationTrackerSettings,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigOrientation {
    pub target: [f32; 4],
    pub basis: Basis3,
    pub pitch: AngleTracker,
    pub yaw: AngleTracker,
    pub pitch_parameters: ScalarTrackerParameters,
    pub yaw_parameters: ScalarTrackerParameters,
    pub pan_smoothing: f32,
    pub tilt_smoothing: f32,
    pub roll: f32,
    pub framing_mirror: f32,
}

impl RigOrientation {
    /// Construction82E02738 and reset82E02E58. The angle parameters are host
    /// storage: every consumed numerical field is rebound before tracking.
    pub fn new() -> Self {
        let tracker = AngleTracker(ScalarTracker {
            target: 0.0,
            position: 0.0,
            velocity: 0.0,
            acceleration: 0.0,
            acceleration_clamp: 0.0,
            smoothing: 0.0,
        });
        let parameters = ScalarTrackerParameters {
            delta_umbra: 0.0,
            delta_penumbra: 0.0,
            speed_clamp: 0.0,
            acceleration_clamp_min: 0.0,
            acceleration_clamp_max: 0.0,
            smoothing_min: 0.0,
            smoothing_max: 0.0,
            overshoot_zeroes_velocity: true,
        };
        Self {
            target: [0.0; 4],
            basis: Basis3 {
                columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            },
            pitch: tracker,
            yaw: tracker,
            pitch_parameters: parameters,
            yaw_parameters: parameters,
            pan_smoothing: 0.0,
            tilt_smoothing: 0.0,
            roll: 0.0,
            framing_mirror: 1.0,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        settings: OrientationSettings,
        reset_time: f32,
        flags_516: &mut u8,
        flags_517: u8,
    ) {
        // The same sqrt(2)-scaled quaternion expansion occurs in the rigid
        // body's basis helper and82E0466C..82E046AC.
        let [x, y, z, w] = self.target;
        let target = basis_from_quaternion(RetailQuaternion { x, y, z, w });
        let at = target.columns[2];
        let angles = direction_to_angles([at[0], at[1], at[2], 0.0]);
        let pitch = -angles[0];
        let yaw = angles[1];
        let error = normalize_angle(normalize_angle(yaw) - self.yaw.0.position);
        let error = if error < normalize_angle(0.0) {
            normalize_angle(normalize_angle(-error))
        } else {
            error
        };
        let radians = f32::from_bits(0x3c8efa35);
        let umbra = settings.pan_umbra_degrees * radians;
        let penumbra = settings.pan_penumbra_degrees * radians;
        let fraction = if error < umbra {
            1.0
        } else if error < penumbra {
            1.0 - super::rig_tracking::wrap_vmx(error - umbra)
                / super::rig_tracking::wrap_vmx(penumbra - umbra)
        } else {
            0.0
        };
        let response = settings.pan_smoothing_curve.evaluate(fraction);
        let minimum = 1.0 - settings.pan_minimum_smoothing;
        let minimum = -(minimum * minimum - 1.0);
        let pan = if self.pan_smoothing >= minimum {
            (self.pan_smoothing - minimum) * response + minimum
        } else {
            self.pan_smoothing
        };
        let pan = cap_smoothing(pan);
        self.tilt_smoothing = cap_smoothing(self.tilt_smoothing);
        if *flags_516 & 8 != 0 || reset_time > 0.0 {
            reset(&mut self.pitch, pitch);
            reset(&mut self.yaw, yaw);
            *flags_516 &= !8;
        }
        if *flags_516 & 0x40 != 0 {
            bind(
                &mut self.pitch_parameters,
                settings.tilt,
                self.tilt_smoothing,
                flags_517 & 0x10 != 0,
            );
            self.pitch
                .update(dt, normalize_angle(pitch), self.pitch_parameters);
        }
        bind(&mut self.yaw_parameters, settings.pan, pan, true);
        self.yaw
            .update(dt, normalize_angle(yaw), self.yaw_parameters);
        let basis = super::orientation_math::basis_from_angles(
            self.pitch.0.position,
            self.yaw.0.position,
            0.0,
        );
        self.basis = super::orientation_math::rotate_about_axis(
            basis,
            basis.columns[2],
            self.roll * self.framing_mirror,
        );
    }
}

fn reset(tracker: &mut AngleTracker, target: f32) {
    tracker.0.target = normalize_angle(target);
    tracker.0.position = normalize_angle(target);
    tracker.0.velocity = 0.0;
    tracker.0.acceleration = 0.0;
}

fn cap_smoothing(value: f32) -> f32 {
    if value >= 1.0 {
        1.0
    } else if f32::from_bits(0x3f666666) - value >= 0.0 {
        value
    } else {
        f32::from_bits(0x3f666666)
    }
}

fn bind(
    p: &mut ScalarTrackerParameters,
    s: OrientationTrackerSettings,
    smoothing: f32,
    clamp_acceleration: bool,
) {
    let radians = f32::from_bits(0x3c8efa35);
    p.speed_clamp = f32::MAX;
    p.acceleration_clamp_min = if clamp_acceleration {
        s.acceleration_min_degrees * radians
    } else {
        f32::MAX
    };
    p.acceleration_clamp_max = if clamp_acceleration {
        s.acceleration_max_degrees * radians
    } else {
        f32::MAX
    };
    p.smoothing_min = if smoothing < 1.0 {
        if s.smoothing_min - smoothing >= 0.0 {
            smoothing
        } else {
            s.smoothing_min
        }
    } else {
        1.0
    };
    p.smoothing_max = if smoothing < 1.0 { smoothing } else { 1.0 };
    p.delta_umbra = s.delta_umbra_degrees * radians;
    p.delta_penumbra = s.delta_penumbra_degrees * radians;
}
