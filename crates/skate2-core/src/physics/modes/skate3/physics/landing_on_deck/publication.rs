//! State503 FillPhysOut82D4DFC8. No output is inferred from render transforms.
use super::Skate3ControllerState;
pub(crate) fn fill(skater: &mut Skate3ControllerState) -> Result<(), String> {
    let state = &skater.landing_on_deck.state;
    let output = state
        .output
        .as_ref()
        .ok_or("Landing Fill requires its completed Update")?;
    let physical = &mut skater.player_input.physical;
    let off = &mut physical.off_board;
    off.trajectory_valid_331 = u8::from(output.trajectory_valid);
    off.scalar_152 = output.elapsed;
    off.vector_176 = output.launch_position.map(f32::to_bits);
    off.vector_208 = output.landing_position.map(f32::to_bits);
    off.scalar_92 = output.landing_time;
    off.vector_192 = output.normal.map(f32::to_bits);
    off.vector_240 = output.apex_position.map(f32::to_bits);
    off.scalar_156 = output.apex_time;
    off.vector_224 = output.direction.map(f32::to_bits);
    off.vector_160 = output.up.map(f32::to_bits);
    off.scalar_32 = state.time_to_land;
    off.flag_315 = u8::from(state.near_deck);
    off.flag_318 = u8::from(state.turning);
    physical.ground.hippy_takeoff_323 = u8::from(state.takeoff_frames > 0);
    physical.ground.hippy_jumping_322 = u8::from(state.hippy);
    physical.ground.landing_half_turns_312 = state.landing_half_turns();
    skater
        .landing_deck
        .publish(crate::skate3_native::physics::offboard::landing_deck::PublishTargets {
            off_board: off,
            skeleton_position_48: &mut physical.collision.vector_48,
            skeleton_flag_3482: &mut physical.collision.flag_3482,
        });
    Ok(())
}
