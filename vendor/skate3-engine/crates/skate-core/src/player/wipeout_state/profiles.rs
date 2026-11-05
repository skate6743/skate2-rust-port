//! The five authored physics_wipeout_control profiles and original82D3C3F8.
use super::{
    State,
    math::{self, V},
};
use crate::{physics::skeleton_body::SkeletonBody, point_graph::PointGraph};
#[derive(Clone, Debug)]
pub struct Profile {
    pub roll_axis: V,                     //0
    pub horizontal_axis: V,               //16
    pub align_euler: V,                   //32
    pub spin_vs_time: PointGraph<8>,      //48, source uses X64/Y96
    pub direction_follow: f32,            //128
    pub torque_velocity: f32,             //132
    pub torque_distance: f32,             //136
    pub spin_inertia: f32,                //140
    pub roll_on_ground: bool,             //144
    pub ground_roll_torque: f32,          //148
    pub sideways_spin: f32,               //152
    pub forward_spin: f32,                //156
    pub horizontal_directed: bool,        //160
    pub drift_maximum_speed: f32,         //164
    pub drift_forward: f32,               //168
    pub drift_sideways: f32,              //172
    pub align_with_velocity: bool,        //176
    pub align_ground_with_velocity: bool, //177
    pub ground_align_torque: f32,         //180
    pub tilt_degrees: f32,                //184
    pub drift_drag: f32,                  //188
}
pub fn prepare(state: &mut State, profiles: &[Profile; 5], gesture: [f32; 2], input: [f32; 2]) {
    let [x, z] = gesture;
    let selected = if x.abs() < 0.2 && z.abs() < 0.2 {
        0
    } else if x.abs() <= z.abs() {
        if z <= 0.0 { 3 } else { 1 }
    } else if x <= 0.0 {
        4
    } else {
        2
    };
    state.control_time += f32::from_bits(0x3C88_8889);
    if selected != state.profile {
        state.control_time = 0.0;
        state.profile = selected;
        state.retained_sideways_input = 0.0;
        state.retained_forward_input = 0.0;
    }
    let mut flat_velocity = state.velocity;
    flat_velocity[1] = 0.0;
    let normalized = math::normalize_or(flat_velocity, [0.0; 4]);
    state.forward = if state.direction_initialized {
        let follow = profiles[selected].direction_follow;
        math::normalize_or(
            math::madd(normalized, follow, math::scale(state.forward, 1.0 - follow)),
            [0.0; 4],
        )
    } else {
        state.direction_initialized = true;
        normalized
    };
    state.right = math::cross([0.0, 1.0, 0.0, 0.0], state.forward);
    let input = [input[0], 0.0, input[1], 0.0];
    state.forward_input = math::dot(input, state.forward);
    state.sideways_input = math::dot(input, state.right);
}
///Original82D3C928, after air rotational control; preserves body velocity W.
pub fn drift(state: &State, body: &mut SkeletonBody, p: &Profile) {
    if state.retained_velocity_active {
        return;
    }
    let desired = math::madd(
        state.right,
        p.drift_sideways * state.sideways_input,
        math::scale(state.forward, p.drift_forward * state.forward_input),
    );
    let mut change = state
        .velocity
        .map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000) * p.drift_drag);
    change[0] *= 0.02;
    change[2] *= 0.02;
    if math::dot(state.velocity, math::normalize_or(desired, [0.0; 4])) < p.drift_maximum_speed {
        change = math::add(change, desired);
    }
    let change = math::scale(change, f32::from_bits(0x3C88_8889));
    for part in &mut body.bodies_mut()[1..24] {
        part.rates.linear_velocity.x += change[0];
        part.rates.linear_velocity.y += change[1];
        part.rates.linear_velocity.z += change[2];
    }
}
