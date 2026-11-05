//! Complete normal-rig tracking stages. Runtime parameters are explicit native
//! bindings; this does not initialize a shot or infer stock camera settings.

use super::vector_tracker::{length, refined_reciprocal};
use super::{AngleTracker, ScalarTrackerParameters, VectorTracker, normalize_angle};
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngleTrackingSettings {
    pub speed_clamp_degrees: f32,
    pub acceleration_min_degrees: f32,
    pub acceleration_max_degrees: f32,
    pub delta_umbra_degrees: f32,
    pub delta_penumbra_degrees: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngularRigTracking {
    pub heading: AngleTracker,
    pub heading_parameters: ScalarTrackerParameters,
    pub elevation: AngleTracker,
    pub elevation_parameters: ScalarTrackerParameters,
    pub heading_target: f32,
    pub elevation_target: f32,
    pub heading_offset: f32,
    pub reset_time: f32,
    pub heading_smoothing: f32,
    pub elevation_smoothing: f32,
    /// Native rig484 controls both the heading gate and acceleration doubling.
    pub heading_acceleration_state: f32,
    /// Native rig517; bit0x20 enables heading update independently of rig484.
    pub flags_517: u8,
}

impl AngularRigTracking {
    /// Complete82E030C0. This VMX wrap differs from scalar8258DB98.
    pub fn current_heading(&self) -> f32 {
        wrap_vmx(self.heading.0.position + self.heading_offset)
    }

    /// Complete82E054D8, including the preserve-velocity reset branch.
    pub fn reset_heading(&mut self, clear_motion: bool) {
        self.heading.0.target = normalize_angle(self.heading_target);
        self.heading.0.position = normalize_angle(self.heading_target);
        if clear_motion {
            self.heading.0.velocity = 0.0;
            self.heading.0.acceleration = 0.0;
        }
    }

    /// Complete82E05558.
    pub fn reset_elevation(&mut self, clear_motion: bool) {
        self.elevation.0.target = normalize_angle(self.elevation_target);
        self.elevation.0.position = normalize_angle(self.elevation_target);
        if clear_motion {
            self.elevation.0.velocity = 0.0;
            self.elevation.0.acceleration = 0.0;
        }
    }

    /// Complete82E03C18 with parameter-block fields516..532.
    pub fn update_heading(&mut self, dt: f32, settings: AngleTrackingSettings) {
        if self.flags_517 & 0x20 == 0 && self.heading_acceleration_state == 0.0 {
            return;
        }
        let error = wrap_vmx(self.heading_target - self.current_heading()).abs();
        let smoothing = 1.0 - self.heading_smoothing;
        let capped = if f32::from_bits(0x3f4ccccd) - smoothing >= 0.0 {
            smoothing
        } else {
            f32::from_bits(0x3f4ccccd)
        };
        let smoothing = if error < f32::from_bits(0x3f490fdb) {
            (capped - smoothing).mul_add(error * f32::from_bits(0x3fa2f984), smoothing)
        } else {
            capped
        };
        if self.reset_time > 0.0 {
            self.reset_heading(true);
        }
        bind_angle_settings(&mut self.heading_parameters, settings, smoothing);
        if self.heading_acceleration_state > 0.0 {
            self.heading_parameters.acceleration_clamp_min *= 2.0;
            self.heading_parameters.acceleration_clamp_max *= 2.0;
        }
        self.heading_parameters.overshoot_zeroes_velocity = false;
        self.heading.update(
            dt,
            normalize_angle(self.heading_target),
            self.heading_parameters,
        );
    }

    /// Complete82E03E28 with parameter-block fields560..576.
    pub fn update_elevation(&mut self, dt: f32, settings: AngleTrackingSettings) {
        if self.reset_time > 0.0 {
            self.reset_elevation(true);
        }
        bind_angle_settings(
            &mut self.elevation_parameters,
            settings,
            1.0 - self.elevation_smoothing,
        );
        self.elevation.update(
            dt,
            normalize_angle(self.elevation_target),
            self.elevation_parameters,
        );
    }
}

fn bind_angle_settings(p: &mut ScalarTrackerParameters, s: AngleTrackingSettings, smoothing: f32) {
    let radians = f32::from_bits(0x3c8efa35);
    p.speed_clamp = s.speed_clamp_degrees * radians;
    p.acceleration_clamp_min = s.acceleration_min_degrees * radians;
    p.acceleration_clamp_max = s.acceleration_max_degrees * radians;
    p.delta_umbra = s.delta_umbra_degrees * radians;
    p.delta_penumbra = s.delta_penumbra_degrees * radians;
    p.smoothing_min = smoothing;
    p.smoothing_max = smoothing;
}

pub(super) fn wrap_vmx(angle: f32) -> f32 {
    let tau = f32::from_bits(0x40c90fdb);
    let pi = f32::from_bits(0x40490fdb);
    let turns = (refined_reciprocal(tau) * angle).trunc();
    let reduced = (-turns).mul_add(tau, angle);
    let lower = if reduced + pi >= 0.0 {
        reduced
    } else {
        reduced + tau
    };
    if reduced - pi >= 0.0 {
        reduced - tau
    } else {
        lower
    }
}

/// Native subject getter slots used by82E04360/82E04478/82E041B8.
/// Slot numbers remain explicit until the coordinator binds the subject producers.
pub trait AnchorTrackingSubject {
    fn flag_584(&mut self) -> u8;
    fn flag_588(&mut self) -> u8;
    fn value_476(&mut self) -> f32;
    fn flag_548(&mut self) -> u8;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorTrackingSettings {
    /// Native X320/Y352, eight points.
    pub latch_curve: PointGraph<8>,
    /// Native X448/Y480, eight points.
    pub input_curve: PointGraph<8>,
    pub speed_clamp: f32,
    pub acceleration_clamp: f32,
    /// Native fields600,624,808.
    pub latch_scale: f32,
    pub input_threshold: f32,
    pub input_scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorRigTracking {
    pub tracker: VectorTracker,
    pub parameters: ScalarTrackerParameters,
    pub anchor: [f32; 4],
    pub velocity: [f32; 4],
    pub reference_height: f32,
    pub latch_time: f32,
    pub input_time: f32,
    pub flags_516: u8,
    pub flags_517: u8,
}

impl AnchorRigTracking {
    /// Complete82E04360, including the conditional second subject getter.
    pub fn latch_smoothing(
        &mut self,
        dt: f32,
        s: AnchorTrackingSettings,
        subject: &mut impl AnchorTrackingSubject,
    ) -> f32 {
        self.latch_time = floor_zero(self.latch_time - dt);
        if subject.flag_584() == 0 {
            self.flags_516 &= !4;
        } else if subject.flag_588() != 0 {
            self.flags_516 |= 4;
        }
        if self.flags_516 & 4 != 0 {
            self.latch_time = 0.5;
        }
        clamp_fraction(s.latch_curve.evaluate((-self.latch_time).mul_add(2.0, 1.0))) * s.latch_scale
    }

    /// Complete82E04478.
    pub fn input_smoothing(
        &mut self,
        dt: f32,
        s: AnchorTrackingSettings,
        subject: &mut impl AnchorTrackingSubject,
    ) -> f32 {
        self.input_time = floor_zero(self.input_time - dt);
        if subject.value_476() > s.input_threshold {
            self.input_time = 0.5;
        }
        clamp_fraction(s.input_curve.evaluate((-self.input_time).mul_add(2.0, 1.0))) * s.input_scale
    }

    /// Complete82E041B8. Both smoothing histories update before the disable gate.
    pub fn update(
        &mut self,
        dt: f32,
        s: AnchorTrackingSettings,
        subject: &mut impl AnchorTrackingSubject,
    ) {
        let latch = self.latch_smoothing(dt, s, subject);
        let input = self.input_smoothing(dt, s, subject);
        let smoothing = if subject.flag_548() != 0 || self.flags_517 & 0x40 != 0 {
            0.0
        } else {
            let maximum = if latch - input >= 0.0 { latch } else { input };
            maximum * clamp_fraction((length(self.velocity) - 8.0) * -0.25)
        };
        self.parameters.speed_clamp = s.speed_clamp;
        self.parameters.acceleration_clamp_min = s.acceleration_clamp;
        self.parameters.acceleration_clamp_max = s.acceleration_clamp;
        self.parameters.smoothing_min = smoothing;
        self.parameters.smoothing_max = smoothing;
        let mut target = self.anchor;
        target[1] = self.reference_height;
        self.tracker.update(dt, target, self.parameters);
    }
}

fn floor_zero(value: f32) -> f32 {
    if -value >= 0.0 { 0.0 } else { value }
}
fn clamp_fraction(value: f32) -> f32 {
    let v = floor_zero(value);
    if 1.0 - v >= 0.0 { v } else { 1.0 }
}

/// Subject getter slots consumed by the complete reference-height stage82E03F20.
pub trait ReferenceHeightSubject {
    fn flag_656(&mut self) -> u8;
    fn flag_608(&mut self) -> u8;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceHeightTracking {
    pub anchor_height: f32,
    pub transition_start: f32,
    pub transition_end: f32,
    pub transition_weight: f32,
    pub transition_duration: f32,
    pub transition_time: f32,
    pub height: f32,
    pub previous_height: f32,
    pub height_offset: f32,
    pub mode: u32,
    pub flags_516: u8,
    pub flags_517: u8,
}

impl ReferenceHeightTracking {
    /// Complete82E03F20, including zero/initial height, transitions and the
    /// conditional subject-query order. No mode is invented or selected here.
    pub fn update(&mut self, dt: f32, subject: &mut impl ReferenceHeightSubject) {
        let immediate = self.flags_516 & 0x40 != 0;
        if immediate {
            self.transition_duration = 0.0;
        }
        let old_height = self.height;
        let mut target = old_height;
        let blending = !immediate && self.transition_time < self.transition_duration;
        if blending {
            let fraction = clamp_fraction(self.transition_time / self.transition_duration);
            target = fraction.mul_add(
                self.transition_end - self.transition_start,
                self.transition_start,
            );
            if self.flags_517 & 8 != 0 {
                let weight = self.transition_weight * self.transition_weight;
                target = (1.0 - weight).mul_add(target, self.anchor_height * weight);
            }
        } else if self.mode == 0 || self.mode == 2 {
            self.previous_height = old_height;
            target = self.anchor_height;
        } else if immediate || old_height > self.anchor_height {
            target = self.anchor_height;
        }
        let mut smoothing = 1.0;
        if old_height != f32::MAX && !blending && self.mode != 2 && subject.flag_656() != 0 {
            smoothing = f32::from_bits(0x3c23d70a);
        }
        self.height = (target - self.height).mul_add(smoothing, self.height);
        if subject.flag_608() != 0 {
            let delta = self.height - self.anchor_height;
            if delta.abs() > f32::from_bits(0x38d1b717) {
                self.height_offset = delta;
                if delta > 0.0 {
                    self.height = self.anchor_height;
                    self.height_offset = 0.0;
                }
            } else if self.height_offset.abs() > f32::from_bits(0x3ba3d70a) {
                self.height_offset *= f32::from_bits(0x3f666666);
                self.height += self.height_offset;
            } else {
                self.height_offset = 0.0;
            }
        } else {
            self.height_offset = self.height - self.anchor_height;
        }
        self.transition_time += dt;
    }
}
