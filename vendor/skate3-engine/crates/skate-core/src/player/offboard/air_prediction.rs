//! Persistent result owner82D6CC50 and frame publication82D6E3F8.
use super::{air_completion::Completed, air_launch::V, air_queries::Prepared};
use crate::{air::trajectory::Trajectory, point_graph::PointGraph};
const STEP: f32 = f32::from_bits(0x3c888889);
#[derive(Clone, Copy, Debug, Default)]
pub struct Packet {
    pub position: V,
    pub velocity: V,
    pub normal: V,
    pub impact_velocity: V,
    pub target: V,
    pub offset: V,
    pub apex: V,
    pub remaining: f32,
    pub duration: f32,
    pub impact_y: f32,
    pub apex_time: f32,
    pub tick: i32,
    pub contact: bool,
    pub surface_kind: u32,
}
pub struct Prediction {
    pub launch_up: V,
    pub can_requery: bool,
    pub requery_position: Option<V>,
    pub selected: Trajectory,
    pub original: Trajectory,
    pub result: Completed,
    pub offset: V,
    pub elapsed: f32,
    pub blend: f32,
    pub landing_frame: i32,
    pub duration: f32,
    pub surface_kind: u32,
}
impl Prediction {
    pub fn complete(prepared: &Prepared, mut result: Completed) -> Self {
        let time = -(result.candidate.skipped_frames as f32) * STEP;
        let mut selected = result.candidate.trajectory;
        selected.position = selected.position_at(time);
        selected.velocity = selected.velocity_at(time);
        selected.position = std::array::from_fn(|i| selected.position[i] - prepared.offset[i]);
        if result.result.valid() {
            result.result.contact_frame += result.candidate.skipped_frames as i32;
            result.result.contact_time -= time;
        }
        let landing_frame = result.landing_frame;
        let duration = if result.result.valid() {
            landing_frame as f32 * STEP
        } else {
            0.
        };
        let surface_kind = if result.result.valid() {
            (result.result.surface >> 7) & 31
        } else {
            0
        };
        Self {
            launch_up: prepared.launch_up,
            can_requery: true,
            requery_position: None,
            selected,
            original: prepared.original,
            result,
            offset: [0.; 4],
            elapsed: 0.,
            blend: 0.,
            landing_frame,
            duration,
            surface_kind,
        }
    }
    pub fn packet(&mut self, tick: i32, dt: f32, curve: &PointGraph<8>) -> Packet {
        self.elapsed += dt;
        self.blend = curve.evaluate(self.elapsed);
        let t = tick as f32 * STEP;
        let selected = self.selected.position_at(t);
        let original = self.original.position_at(t);
        Packet {
            position: std::array::from_fn(|i| {
                original[i].mul_add(1. - self.blend, selected[i] * self.blend)
            }),
            velocity: self.selected.velocity_at(t),
            normal: self.result.normal,
            impact_velocity: self.result.impact_velocity,
            target: self.result.impact_position,
            offset: self.offset,
            apex: self.selected.highest_position().0,
            remaining: (self.landing_frame - tick) as f32 * STEP,
            duration: self.duration,
            impact_y: self.result.impact_position[1],
            apex_time: self.selected.highest_position().1,
            tick,
            contact: self.result.contact,
            surface_kind: self.surface_kind,
        }
    }
    ///82D6DE08 adjusts velocity at the current frame, then restores the common epoch.
    pub fn adjust_animation(&mut self, tick: i32, animation: V, frame: [[f32; 4]; 4], radius: f32) {
        use super::air_launch::{dot, length, madd, scale, sub};
        self.offset = scale(frame[2], animation[2]);
        let mut correction = scale(madd(frame[1], animation[1] - radius, self.offset), -1.);
        correction[1] -= 0.08;
        let target = sub(self.result.impact_position, correction);
        let delta = sub(
            target,
            self.selected.position_at(self.landing_frame as f32 * STEP),
        );
        let time = tick as f32 * STEP;
        let mut arc = self.selected;
        arc.position = arc.position_at(time);
        arc.velocity = arc.velocity_at(time);
        let remaining = self.landing_frame - tick;
        if remaining > 0 {
            let correction = scale(delta, 1. / (remaining as f32 * STEP));
            let scalar = if dot(correction, correction) > 0.25 {
                0.5 / length(correction)
            } else {
                1.
            };
            arc.velocity = madd(correction, scalar, arc.velocity);
        }
        arc.position = arc.position_at(-time);
        arc.velocity = arc.velocity_at(-time);
        self.selected = arc;
        let mut flat = arc.velocity;
        flat[1] = 0.;
        let end = arc.position_at(self.landing_frame as f32 * STEP);
        //82D6E018 is the original descending-wall trajectory qualification.
        if self.launch_up[1] < 0.8
            && dot(self.launch_up, flat) <= 0.
            && self.result.normal[1] > 0.9
            && end[1] - arc.position[1] > -0.2
        {
            //82D6E178 recomputes the native9.8 m/s² arc to the same end point.
            let duration = (2. * length(sub(arc.position, end)) / 9.8)
                .sqrt()
                .mul_add(0.66, self.landing_frame as f32 * f32::from_bits(0x3bb9af72));
            if duration > 0.2 {
                let acceleration = [0., -9.8, 0., 0.];
                self.selected.velocity = sub(
                    scale(sub(end, arc.position), 1. / duration),
                    scale(acceleration, 0.5 * duration),
                );
                self.selected.acceleration = acceleration;
                self.selected.duration = -1.;
                self.landing_frame = (duration * f32::from_bits(0x426fffff)) as i32;
                self.duration = self.landing_frame as f32 * STEP;
            }
        }
    }
}
