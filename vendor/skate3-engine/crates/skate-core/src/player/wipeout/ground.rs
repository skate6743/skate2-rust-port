//! Complete original Ground checks82D8F9E0 and all its nonexternal leaves.
use super::{Frame, Mode, Requests, Settings, common};
use crate::{
    physics::native_arithmetic::{dot3, vector_max, vector_min},
    trigonometry,
};
pub fn check(state: &mut Requests, s: &Settings, mode: &Mode, f: &Frame) {
    state.enter_ground();
    if state.cooldown > 0.0 {
        state.cooldown -= f.timestep;
        return;
    }
    let g = &s.ground;
    let vehicle = if f.group_8 { g.vehicle_scalar } else { 1.0 };
    let trick = if f.flags_2472 & (1 << 15) != 0 {
        f32::from_bits(0x3e99_999a)
    } else {
        1.0
    };
    let skitch = if f.flags_2476 & 8 != 0 {
        g.skitch_scalar
    } else {
        1.0
    };
    let arms = if f.flags_2476 & 8 != 0 {
        g.skitch_arms_scalar
    } else {
        1.0
    };
    let skater = if f.flags_2472 & (1 << 28) == 0 && f.maximum_skater_force > 0.0 {
        g.skater_scalar
    } else {
        1.0
    };
    let scale = ((vehicle * trick) * skitch) * skater;
    let squash = if f.flags_2476 & (1 << 30) != 0 {
        g.max_squash_coffin
    } else {
        g.max_squash
    };
    let displacement = g.max_displacement * scale;
    if (mode.check_squash || f.time_on_ground > f32::from_bits(0x3d4c_cccd))
        && f.maximum_pose_error > vehicle * squash
    {
        state.request(18, 0.0);
    } else if dot3(f.pose_error, f.pose_error) > displacement * displacement {
        state.request(1, 0.0);
    } else if common::force(f, g.max_contact * scale, g.max_arm_contact * arms) {
        state.request(0, 0.0);
    }
    common::vehicle(state, s, f);
    if f.flags_2484 & (1 << 21) == 0 {
        let cosine = vector_min(
            vector_min(
                dot3(f.deck[0], f.input_board[0]),
                dot3(f.deck[1], f.input_board[1]),
            ),
            dot3(f.deck[2], f.input_board[2]),
        );
        if trigonometry::acos(vector_min(vector_max(cosine, -1.0), 1.0)) > g.max_deck_error {
            state.request(3, 0.0);
        }
    }
    let (xz, y) = if f.flags_2472 & (1 << 15) != 0 {
        (s.air.xz_trick, s.air.y_trick)
    } else {
        let (mut xz, mut y) = (mode.ground_xz, g.y_acceleration);
        if f.board_material_flags & (1 << 30) != 0 {
            let scalar = if f.flags_2472 & (1 << 28) != 0 {
                g.ai_scalar
            } else {
                g.player_scalar
            };
            xz *= scalar;
            y *= scalar;
        }
        if f.board_material_flags & (1 << 29) != 0 {
            xz *= g.light_dmo_scalar;
        }
        if f.flags_2476 & 8 != 0 {
            xz *= g.skitch_acc_scalar;
            y *= g.skitch_acc_scalar;
        }
        if f.flags_2484 & (1 << 21) != 0 {
            xz = 0.0;
            y = 0.0;
        }
        (xz, y)
    };
    common::closing(state, f, xz, y);
    let skitch_gate = if f.flags_2476 & (1 << 23) != 0 {
        0.0
    } else {
        1.0
    };
    let slide_gate = if f.flags_2468 & (1 << 5) != 0 {
        0.0
    } else {
        1.0
    };
    let speed = (g.balance_min_speed - f.speed) / g.balance_min_speed;
    let amount = (speed * (1.0 - f.animation_up[1])) * slide_gate;
    let delta = amount.mul_add(skitch_gate, -g.balance_base);
    state.balance = if delta > 0.0 {
        state.balance + delta
    } else {
        0.0
    };
    if state.balance > g.balance_total {
        state.request(11, 0.0);
    }
    if f.conflicting {
        state.request(16, 0.0);
    }
    common::leaning(state, s, f);
}

///GroundAnimation82D8F918. These additional checks run even when the shared
///ground check returns during its cooldown. Caller82D34140 supplies scale1.
pub fn check_animation(state: &mut Requests, s: &Settings, mode: &Mode, f: &Frame, scale: f32) {
    check(state, s, mode, f);
    if super::common::force(f, s.air.max_contact, s.air.max_arm_contact) {
        state.request(0, 0.0);
    }
    if f.opposing_contact > s.ground.opposing_contact * scale {
        state.request(20, 0.0);
    }
}

///82D8FDC0, FootPlant postphysics checks. HandPlant calls check_air(false).
pub fn check_plant(state: &mut Requests, s: &Settings, f: &Frame) {
    state.mode = 3;
    let input = super::data::PlantInput {
        maximum_pose_error: f.maximum_pose_error,
        pose_error: f.pose_error,
        regions_force: f.regions_force,
        contact: f.board_contact.then_some(super::data::PlantContact {
            world_to_animation: f.world_to_animation,
            closing_velocity: f.closing_velocity,
        }),
    };
    check_plant_input(s, &input, |reason, value| state.request(reason, value));
}

/// The same82D8FDC0 calculation using only its consumed observations.
pub fn check_plant_input(
    s: &Settings,
    f: &super::data::PlantInput,
    mut request: impl FnMut(usize, f32),
) {
    if f.maximum_pose_error > s.ground.max_squash {
        request(18, 0.);
    } else if super::common::length(f.pose_error) > s.air.max_displacement {
        request(1, 0.);
    } else if super::common::force_values(f.regions_force, s.air.max_contact, s.air.max_arm_contact)
    {
        request(0, 0.);
    }
    if let Some(contact) = &f.contact {
        common::closing_values(
            contact.closing_velocity,
            contact.world_to_animation,
            f.regions_force,
            s.air.xz_trick,
            s.air.y_trick,
            &mut request,
        );
    }
}
