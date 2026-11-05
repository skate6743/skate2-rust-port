//! Concrete SetVelocity82C04168 continuation; retain main's jumper owner.
use skate3_core::{
    math::Vector3,
    physics::{
        board_runtime::BoardRuntime,
        grind_contact::manager::Jumper,
        grind_forces::launch::{self, Input},
    },
};

pub(crate) fn apply(
    board: &mut BoardRuntime,
    jumper: &mut Jumper,
    flags_2476: &mut u32,
    input: Input,
) -> [f32; 4] {
    let output = launch::launch(*jumper, input);
    let v = output.velocity;
    // Original SetVelocity writes every board part, not only the deck. It
    // neither resets angular velocity nor teleports the board's transforms.
    for body in board.bodies_mut() {
        body.rates.linear_velocity = Vector3::new(v[0], v[1], v[2]);
    }
    *jumper = output.jumper;
    *flags_2476 |= output.flags_2476_to_or;
    v
}
