//! Complete the grind FillOut write to native PhysOut pointer+12's 48-byte
//! block. CURRENT already represents it as PhysicalPlayerInput::physics.
use crate::skate3_native::physics::Skate3ControllerState;

/// Call immediately after grind FillOut sets pending_wipeout_impulse, before
/// the completed physical packet is consumed by the next input phase.
pub(crate) fn publish(skater: &mut Skate3ControllerState) {
    if let Some(impulse) = skater.grind.pending_wipeout_impulse.take() {
        let output = &mut skater.player_input.physical.physics;
        output.vector_16 = impulse.map(f32::to_bits);
        output.flag_32 = 1;
    }
    // No inactive clearing: the ordinary physical output reset owns this block.
    // ProcessInput82DB4CCC/4CDC publishes bit2472.11 and vector816; existing
    // Wipeout Enter82D3B5E8 caches them, Update82D3BFEC applies deck force.
    // This is not a direct body impulse/velocity or animation packet mutation.
}
