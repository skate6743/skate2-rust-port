//! Complete Air checks82D90358 including bad landing82D90D08.
use super::{Frame, Mode, Requests, Settings, common};
use crate::physics::native_arithmetic::dot3;
///82D8FEE8, shared off-board air collision check; no on-board cooldown/landing tests.
pub fn check_collision(state: &mut Requests, s: &Settings, mode: &Mode, f: &Frame) {
    if mode.check_squash && f.maximum_pose_error > s.air.max_squash {
        state.request(18, 0.0);
    } else if dot3(f.pose_error, f.pose_error) > s.air.max_displacement * s.air.max_displacement {
        state.request(1, 0.0);
    } else if common::force(f, s.air.max_contact, s.air.max_arm_contact) {
        state.request(0, 0.0);
    }
}
pub fn check(state: &mut Requests, s: &Settings, mode: &Mode, f: &Frame, use_com: bool) {
    if state.mode != 2 {
        state.mode = 2;
        state.contact_frames = 0;
    }
    if f.board_contact {
        state.contact_frames = 4;
    }
    if state.contact_frames > 0 {
        state.contact_frames -= 1;
    }
    if state.cooldown > 0.0 {
        state.cooldown -= f.timestep;
        return;
    }
    common::vehicle(state, s, f);
    let a = &s.air;
    let body_flip = f.flags_2476 & (1 << 26) != 0;
    let scalar = if body_flip { a.body_flip_scalar } else { 1.0 };
    let deck_scalar = if body_flip {
        a.body_flip_acc_scalar
    } else {
        1.0
    };
    let (mut xz, y) = if f.flags_2472 & (1 << 15) != 0 && f.jump_fix_frames > a.ignore_danger_frames
    {
        (a.xz_trick, a.y_trick)
    } else if f.wheel_contact {
        (mode.ground_xz * deck_scalar, s.ground.y_acceleration)
    } else {
        (a.xz_acceleration * deck_scalar, a.y_acceleration)
    };
    if f.board_material_flags & (1 << 29) != 0 {
        xz *= a.light_dmo_scalar;
    }
    common::closing(state, f, xz, y);
    let displacement = a.max_displacement * scalar;
    if mode.check_squash && f.maximum_pose_error > a.max_squash * scalar {
        state.request(18, 0.0);
    } else if dot3(f.pose_error, f.pose_error) > displacement * displacement {
        state.request(1, 0.0);
    } else if common::force(f, a.max_contact * scalar, a.max_arm_contact) {
        state.request(0, 0.0);
    }
    if f.flip_active && f.flip_requested_speed == 0.0 && f.system_up_y < 0.0 {
        state.request(4, 0.0);
    }
    bad_landing(state, s, mode, f, use_com);
    common::leaning(state, s, f);
    //Two independent sensitive conditions can request21 twice.
    if f.flags_2476 & (1 << 26) != 0 && f.compliant {
        state.request(21, 0.0);
    }
    if f.flags_2472 & (1 << 6) != 0 && (f.compliant || f.board_contact) {
        state.request(21, 0.0);
    }
}
fn bad_landing(state: &mut Requests, s: &Settings, mode: &Mode, f: &Frame, use_com: bool) {
    if !mode.check_bad_landing || !f.board_contact {
        return;
    }
    let velocity = if use_com {
        f.com_velocity
    } else {
        f.deck_velocity
    };
    let mut normal = f.board_contact_normal;
    if f.grind_selected {
        if f.grind_normal_valid {
            normal = f.grind_normal;
        }
        if -dot3(normal, velocity) > s.air.max_grind_speed * mode.bad_landing_scale {
            state.request(8, 0.0);
        }
    } else {
        let into = -dot3(normal, velocity);
        let tangent = std::array::from_fn(|i| normal[i].mul_add(into, velocity[i]));
        let limit =
            s.air.max_landing_angle.evaluate(common::length(tangent)) * mode.bad_landing_scale;
        if f.landing_angle.abs() > limit {
            state.request(5, 0.0);
        }
        let limit = if f.board_material_flags & (1 << 31) != 0 {
            s.air.max_stairs_speed
        } else {
            s.air.max_landing_speed
        };
        if into > limit * mode.bad_landing_scale {
            state.request(6, into);
        }
    }
}
