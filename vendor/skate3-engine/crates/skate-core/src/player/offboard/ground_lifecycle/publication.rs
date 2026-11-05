use super::length;
use crate::player::offboard::ground_entry::{State, Vector};
pub struct PublicationInput {
    pub ground_flags_752_to_754: [bool; 3],
    pub contact_flags_368: u32,
    pub contact_position_192: Vector,
    pub query_position_816: Vector,
    pub processed_flags_2476: u32,
    pub ground_kind_356: u32,
    pub ground_scalar_360: f32,
    pub motion_vector_1040: Vector,
    pub motion_up_864: Vector,
    ///Actual hand manager flags105/106/107 and217/218/219.
    pub hand_flags: [[bool; 3]; 2],
}
///Only fields written by82D32D38/82D785F8; caller preserves all other output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Publication {
    pub physics_counter_36: u32,
    pub physics_counter_40: u32,
    pub physics_flag_86: bool,
    pub offboard_flag_304: bool,
    pub offboard_kind_88: u32,
    pub offboard_scalar_112: f32,
    pub offboard_flag_329: bool,
    pub offboard_flag_330: bool,
    pub offboard_distance_116: f32,
    pub offboard_flag_334: bool,
    pub offboard_scalar_32: f32,
    pub offboard_flag_328: bool,
    pub animation_vector_144: Vector,
    pub animation_flag_164: bool,
    pub offboard_hand_flags_306_307: [bool; 2],
}
pub fn publish(state: &State, i: &PublicationInput) -> Publication {
    let near = i.ground_flags_752_to_754[1] || !(state.distance_164 >= f32::from_bits(0x3daa_aaab));
    let blocked = state.flags_144_to_150[3] || i.contact_flags_368 & 8 != 0;
    let distance = if i.ground_flags_752_to_754[0] && i.contact_flags_368 & 1 != 0 {
        length(std::array::from_fn(|n| {
            i.contact_position_192[n] - i.query_position_816[n]
        }))
    } else {
        f32::from_bits(0x5015_02f9)
    };
    let projection = super::dot(i.motion_up_864, i.motion_vector_1040);
    Publication {
        physics_counter_36: state.counter_152,
        physics_counter_40: state.counter_156,
        physics_flag_86: i.processed_flags_2476 & 0x0040_0000 != 0 && !state.flags_144_to_150[0],
        offboard_flag_304: state.flags_144_to_150[0],
        offboard_kind_88: i.ground_kind_356,
        offboard_scalar_112: i.ground_scalar_360,
        offboard_flag_329: blocked,
        offboard_flag_330: near && !blocked,
        offboard_distance_116: distance,
        offboard_flag_334: i.ground_flags_752_to_754[2],
        offboard_scalar_32: if state.flags_144_to_150[2] { 1.0 } else { 0.0 },
        offboard_flag_328: state.flags_144_to_150[2],
        animation_vector_144: std::array::from_fn(|n| {
            i.motion_vector_1040[n] - i.motion_up_864[n] * projection
        }),
        animation_flag_164: true,
        offboard_hand_flags_306_307: i.hand_flags.map(|f| (f[0] && !f[1]) || f[2]),
    }
}
