//! Complete82D90148, borrowing the existing shared Wipeout request owner.
use super::dot;
use crate::player::{
    offboard::ground_entry::Vector,
    wipeout::{self, Requests},
};
pub struct CollisionSettings {
    ///physics_wipeout/default layout96..120.
    pub vehicle_scalar: f32,
    pub vehicle_contact: f32,
    pub maximum_displacement: f32,
    pub maximum_arm_contact: f32,
    pub maximum_body_contact: f32,
    pub minimum_speed: f32,
    pub maximum_squash: f32,
    ///Actual attribute472174920C68FBE3. Required when processed2480bit7.
    pub special_scalar: f32,
}
pub struct CollisionInput<'a> {
    pub shared: &'a wipeout::Frame,
    pub skeleton_velocity_16336: Vector,
    pub contact_flag_4072: bool,
    pub contact_force_4056: f32,
}
pub fn check_collision(requests: &mut Requests, s: &CollisionSettings, i: &CollisionInput<'_>) {
    requests.mode = 4;
    if dot(i.skeleton_velocity_16336, i.skeleton_velocity_16336)
        <= s.minimum_speed * s.minimum_speed
    {
        return;
    }
    let requested = if i.shared.flags_2480 & 0x80 != 0 {
        s.special_scalar
    } else {
        1.0
    };
    //The source loads vehicle_scalar whenever the contact flag is present,
    //even when vehicle_contact comparison does not emit reason7.
    let vehicle = if i.contact_flag_4072 {
        if i.contact_force_4056 > s.vehicle_contact {
            requests.request(7, 0.0);
        }
        s.vehicle_scalar
    } else {
        1.0
    };
    let scalar = if requested - vehicle >= 0.0 {
        vehicle
    } else {
        requested
    };
    if i.shared.maximum_pose_error > scalar * s.maximum_squash {
        requests.request(18, 0.0);
    } else {
        let limit = s.maximum_displacement * scalar;
        if dot(i.shared.pose_error, i.shared.pose_error) > limit * limit {
            requests.request(1, 0.0);
        } else if wipeout::regional_force(
            i.shared,
            s.maximum_body_contact * scalar,
            s.maximum_arm_contact * vehicle,
        ) {
            requests.request(0, 0.0);
        }
    }
}
