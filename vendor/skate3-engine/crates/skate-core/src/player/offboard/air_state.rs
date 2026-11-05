//! BipedAir retained state82D2EDD8, frame stages82D2EF38/82D2E968,
//! and body-height stage82D2FDD0. Scene and skeleton owners surround these stages.
use super::air_launch::{cross, dot, length, limit_angle, madd, rotate, scale, sub, unit};
use super::{air_launch::V, air_prediction::Packet, ground_entry::Frame};
const UP: V = [0., 1., 0., 0.];
const IDENTITY: Frame = [[1., 0., 0., 0.], UP, [0., 0., 1., 0.], [0.; 4]];
const DT: f32 = f32::from_bits(0x3c888889);
#[derive(Clone, Debug)]
pub struct State {
    pub start: Frame,
    pub target: Frame,
    pub frame: Frame,
    pub packet: Packet,
    pub body_position: V,
    pub body_offset: f32,
    pub lift: f32,
    pub blend: f32,
    pub remaining: f32,
    pub duration: f32,
    pub tick: i32,
    pub landing_direction: V,
    pub launch_up: V,
    pub animation_offset: V,
    pub directed_forward: V,
    pub collision_normal: V,
    pub aligned: bool,
    pub animation_adjusted: bool,
    pub collision_adjusted: bool,
    pub collision_bail: bool,
    pub airborne: bool,
    pub reversed: bool,
    pub directed: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            start: IDENTITY,
            target: IDENTITY,
            frame: IDENTITY,
            packet: Packet::default(),
            body_position: [0.; 4],
            body_offset: 0.,
            lift: 0.,
            blend: 0.,
            remaining: 0.,
            duration: -1.,
            tick: 0,
            landing_direction: [0.; 4],
            launch_up: UP,
            animation_offset: [0.; 4],
            directed_forward: [0., 0., 1., 0.],
            collision_normal: [0., 0., 1., 0.],
            aligned: false,
            animation_adjusted: false,
            collision_adjusted: false,
            collision_bail: false,
            airborne: true,
            reversed: false,
            directed: false,
        }
    }
}
impl State {
    pub fn enter(
        frame: Frame,
        flags_2484: u32,
        body_position: V,
        lifted_position: V,
        position: V,
        up: V,
    ) -> Self {
        Self {
            start: frame,
            frame,
            launch_up: frame[1],
            directed: flags_2484 & 0x4000 != 0,
            body_position,
            body_offset: dot(sub(body_position, position), up),
            lift: dot(sub(lifted_position, body_position), up),
            ..Self::default()
        }
    }
    ///Caller has consumed any pending animation trajectory adjustment before this stage.
    pub fn consume(&mut self, packet: Packet, start_angle: f32, mirrored: bool) {
        self.packet = packet;
        self.duration = packet.duration.max(DT);
        if !self.aligned && packet.contact {
            self.start = self.frame;
            let mut up = UP;
            if packet.normal[1] > 0.85 {
                up = limit_angle(
                    unit(madd(UP, 1., packet.normal), self.start[1]),
                    self.start[1],
                    (packet.remaining * 180.) * f32::from_bits(0x3c8efa35),
                );
            } else if self.start[1][1] < 0.71 {
                self.reversed = true;
            }
            self.landing_frame(
                packet.impact_velocity,
                up,
                packet.remaining,
                start_angle,
                mirrored,
            );
            self.aligned = true;
            self.collision_adjusted = false;
        }
        if packet.contact {
            self.blend = (1. - packet.remaining / self.duration).max(0.).min(1.);
            self.remaining = packet.remaining;
        } else {
            self.blend = 0.;
            self.duration = 10.;
            self.remaining = 10.;
        }
        self.frame =
            crate::animation::foot_ik::interpolate_native(&self.start, &self.target, self.blend).0;
    }
    fn landing_frame(
        &mut self,
        velocity: V,
        up: V,
        remaining: f32,
        start_angle: f32,
        mirrored: bool,
    ) {
        if self.directed {
            self.directed_forward = self.start[2];
            let mut flat = velocity;
            flat[1] = 0.;
            if dot(flat, flat) > 0.01 {
                self.directed_forward = unit(
                    rotate(
                        unit(flat, [0.; 4]),
                        UP,
                        -(if mirrored { -start_angle } else { start_angle }),
                    ),
                    [0.; 4],
                );
            }
            self.target = super::controller::build_frame(up, self.directed_forward);
            return;
        }
        let radians =
            crate::player::wipeout_state::orientation::projected_angle(self.frame[2], velocity, up);
        let turns = radians * f32::from_bits(0x3e22f983);
        let fraction = turns - turns.floor();
        let degrees = ((fraction - if fraction > 0.5 { 1. } else { 0. })
            * f32::from_bits(0x40c90fdb)
            * f32::from_bits(0x42652ee1))
        .abs();
        if self.collision_adjusted {
            self.start = self.frame;
            if degrees > 90. {
                return;
            }
        }
        self.target = self.frame;
        if remaining < f32::from_bits(0x3d072b02) {
            self.start = self.frame;
            return;
        }
        let tangent = sub(velocity, scale(up, dot(velocity, up)));
        let mut right = if length(tangent) > 1. {
            unit_frame(cross(up, velocity), self.target[0])
        } else {
            unit_frame(cross(up, self.frame[2]), self.target[0])
        };
        if length(tangent) > 1. && degrees > 90. && degrees / remaining > 540. {
            self.reversed = true;
            right = scale(right, -1.);
        }
        self.target[0] = right;
        self.target[1] = up;
        self.target[2] = unit_frame(cross(right, up), self.target[2]);
    }
    pub fn update_body(&mut self, up: V, processed_velocity: V, feet: [V; 2], mapped_root: V) {
        let foot_height = dot(sub(feet[0], self.packet.position), up)
            .min(dot(sub(feet[1], self.packet.position), up));
        let delta = (foot_height + f32::from_bits(0x3f4a3d71)) - self.body_offset;
        let limit = (dot(up, processed_velocity) * DT).abs() + 0.05;
        self.body_offset += delta.max(-limit).min(limit);
        self.body_position = madd(up, self.body_offset, self.packet.position);
        let predicted = madd(up, -0.4, madd(self.packet.velocity, DT, mapped_root));
        self.lift = dot(sub(predicted, self.body_position), up);
    }
    pub fn finish(&mut self) {
        if self.remaining <= 0. && self.packet.contact && self.packet.normal[1] > 0.1 {
            self.airborne = false;
        }
    }
}
fn unit_frame(v: V, fallback: V) -> V {
    if length(v) > f32::from_bits(0x37800000) {
        unit(v, fallback)
    } else {
        fallback
    }
}
