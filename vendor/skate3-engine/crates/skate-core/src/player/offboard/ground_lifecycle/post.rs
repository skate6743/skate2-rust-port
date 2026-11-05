use super::{CollisionInput, CollisionSettings, check_collision, dot, length};
use crate::player::{
    offboard::ground_entry::{State, Vector},
    wipeout::Requests,
};
pub struct PostInput<'a> {
    pub collision: CollisionInput<'a>,
    pub processed_flags_2484: u32,
    pub processed_velocity_608: Vector,
    pub skeleton_displacement_16288: Vector,
    pub skeleton_displacement_16304: Vector,
    pub ground_kind_356: u32,
}
pub fn post_physics(
    state: &mut State,
    requests: &mut Requests,
    settings: &CollisionSettings,
    input: &PostInput<'_>,
) {
    check_collision(requests, settings, &input.collision);
    if input.processed_flags_2484 & 0x400 != 0 {
        if state.flags_144_to_150[6] && state.flags_144_to_150[2] {
            requests.request(31, 0.0);
        }
        state.elapsed_168 = 0.0;
        if state.flags_144_to_150[2]
            || state.angle_172 > f32::from_bits(0x3fc9_0fdb)
            || matches!(input.ground_kind_356, 2 | 4)
            || 0.5 > state.frame_80[1][1]
        {
            requests.request(31, 0.0);
        } else if !(dot(
            input.processed_velocity_608,
            input.skeleton_displacement_16304,
        ) >= length(input.processed_velocity_608) * f32::from_bits(0xbca3_d70a))
        {
            requests.request(32, 0.0);
        }
    } else {
        let a = dot(input.skeleton_displacement_16304, state.frame_80[1]).abs();
        let b = dot(input.skeleton_displacement_16288, state.frame_80[1]).abs();
        if a > f32::from_bits(0x3eb3_3333) || b > f32::from_bits(0x3e19_999a) {
            state.elapsed_168 += f32::from_bits(0x3c88_8889);
        } else {
            state.elapsed_168 = 0.0;
        }
        if !(state.elapsed_168 < f32::from_bits(0x3d4c_ccce)) {
            requests.request(32, 0.0);
        }
    }
    if state.flags_144_to_150[1] {
        requests.request(28, 0.0);
    }
}
