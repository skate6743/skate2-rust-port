//! Actual completed PlayerInput/BoardToolkit fields read by82D78xxx.
use crate::skate3_native::physics::player_input::PlayerInputRuntime;
use skate3_core::player::offboard::landing_deck::{Input, Vector};

pub(super) fn processed(input: &PlayerInputRuntime) -> Result<Input, String> {
    let p = &input.processed;
    let board = input
        .toolkit
        .as_ref()
        .ok_or("Landing manager requires the current processed BoardToolkit")?;
    Ok(Input {
        board_up_80: board.deck[1],
        board_position_112: board.deck[3],
        board_velocity_400: p.vectors_400_416[0].map(f32::from_bits),
        up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        support_1776: p.external_physics_1616.flags as i32,
        flags_2480: p.flags_2480,
        mode_2540: p.surface_mode_2540,
        wheel_contacts_2556: p.wheel_count_2556,
    })
}
pub(super) fn position(input: &PlayerInputRuntime) -> Vector {
    input.processed.vectors_544_560_592_608[2].map(f32::from_bits)
}
pub(super) fn velocity(input: &PlayerInputRuntime) -> Vector {
    input.processed.vectors_544_560_592_608[3].map(f32::from_bits)
}
