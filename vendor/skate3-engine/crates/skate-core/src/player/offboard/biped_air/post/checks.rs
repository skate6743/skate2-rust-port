//! S3 82D8FEE8/82D90000; S2 82DD8C90/82DD8EF0 cross-check.
//! These are Air's called checks, not replacements for general wipeout.
use super::{Frame, Mode, Requests, Vector, dot};
use crate::player::wipeout::regional_force;
#[derive(Clone, Copy, Debug)]
pub struct Thresholds {
    pub squash: f32,
    pub displacement: f32,
    pub body_contact: f32,
    pub arm_contact: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub skeleton_air: Thresholds,
    pub offboard_air: Thresholds,
    pub offboard_min_speed: f32,
}
fn collision(requests: &mut Requests, thresholds: Thresholds, mode: Mode, frame: &Frame) {
    //FF48/9009C: squash is first and gated by the actual physics mode.
    if mode.check_squash && frame.maximum_pose_error > thresholds.squash {
        requests.request(18, 0.);
    } else if dot(frame.pose_error, frame.pose_error)
        <= thresholds.displacement * thresholds.displacement
    {
        //FFA4/900EC uses <=; retain its unordered-comparison behavior.
        if regional_force(frame, thresholds.body_contact, thresholds.arm_contact) {
            requests.request(0, 0.);
        }
    } else {
        requests.request(1, 0.);
    }
}
pub fn skeleton_air(requests: &mut Requests, settings: &Settings, mode: Mode, frame: &Frame) {
    //82D8FEE8 has neither a mode write nor a speed gate.
    collision(requests, settings.skeleton_air, mode, frame);
}
pub fn offboard_air(
    requests: &mut Requests,
    settings: &Settings,
    mode: Mode,
    frame: &Frame,
    root_velocity: Vector,
) {
    //90018..20 occurs even below the speed threshold.
    requests.mode = 4;
    //90028..5C reads Skeleton16336, not COM velocity.
    if dot(root_velocity, root_velocity) > settings.offboard_min_speed * settings.offboard_min_speed
    {
        collision(requests, settings.offboard_air, mode, frame);
    }
}
