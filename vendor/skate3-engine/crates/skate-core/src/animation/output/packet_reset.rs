//! AnimOutPhysIn::Reset, TU3 82590028.
//! Reset is selective: it preserves storage, count, intents, attributes and all
//! fields not listed below. Actor::SetUpPhysics owns the later list replacement.
use super::{NativeMatrix, physics_packet::PhysicsPosePacket};
use crate::animation::commands::buffers::BufferError;

/// Additional fields written by Reset, kept separate from the existing pose
/// publication view. Unknown flags retain their verified producer/offset names.
pub struct AdditionalResetFields {
    /// 10388; 10912/10916.
    pub compression: f32,
    pub foot_ik_influence: [f32; 2],
    /// 10369. Position itself is preserved by Reset.
    pub next_step_position_valid: bool,
    /// 10375/10376; overwritten from these Actor bits during SetUpPhysics.
    pub actor_flag_1904_bit23: bool,
    pub actor_flag_1908_bit2: bool,
    /// 10496,10688,10689: active markers only; payloads are not cleared here.
    pub external_impulse_active: bool,
    pub external_physics_input_active: bool,
    pub externally_controlled: bool,
    /// 10786/10787/10796.
    pub prevent_manual_respawn: bool,
    pub ignore_respawn_reset_button: u8,
    pub force_braking: bool,
    /// 10788/10792. Named source fields, same target scalar reset/caller roles.
    pub truck_tightness: f32,
    pub wheel_hardness: f32,
    /// Six complete native vectors at10816,10832,10848,10864,10880,10896.
    /// Individual target semantics remain unassigned; all fourth lanes reset.
    pub auxiliary_vectors: [[f32; 4]; 6],
    /// 10928, later overwritten by the Actor's selected physics settings.
    pub requested_physics_mode: u32,
}

/// Native packed basis, NOT a conventional homogeneous 4x4 identity.
/// Three static vectors at82139A10/20/30 plus an all-zero vector produced
/// inside Reset. All four fourth lanes, including translation W, are zero.
pub const RESET_POSE: NativeMatrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];

/// Complete field coverage for82590028 on valid packet allocations. Host-side
/// extent checks precede mutation; native assumes allocations match the count.
/// Existing vector lengths/capacities and bones past packet.bone_count remain.
pub fn reset(
    packet: &mut PhysicsPosePacket,
    fields: &mut AdditionalResetFields,
) -> Result<(), BufferError> {
    // Both native loop comparisons are signed. A high-bit count skips the
    // matrix loop while scalar/flag resets still run.
    let count = (packet.bone_count as i32).max(0) as usize;
    if packet.hierarchy.len() < count || packet.local.len() < count {
        return Err(BufferError::RangeOutsideAllocation);
    }
    packet.timestep = f32::from_bits(0x3c88_8889);
    fields.external_impulse_active = false;
    fields.compression = 0.5;
    fields.foot_ik_influence[0] = 0.0;
    fields.external_physics_input_active = false;
    fields.foot_ik_influence[1] = 0.0;
    fields.externally_controlled = false;
    fields.truck_tightness = 0.0;
    packet.mirrored = false;
    packet.riding_switch = false;
    packet.riding_fakie = false;
    packet.weight_forwards = false;
    packet.regular_stance = false;
    fields.prevent_manual_respawn = false;
    fields.ignore_respawn_reset_button = 0;
    fields.force_braking = false;
    packet.air_dismount_revert_frames = 0;
    packet.board_flipped = false;
    fields.next_step_position_valid = false;
    fields.actor_flag_1904_bit23 = false;
    packet.flags &= 0x013f_ffff;
    fields.actor_flag_1908_bit2 = false;
    packet.foot_surface_ids[0] = 0;
    fields.wheel_hardness = 0.0;
    packet.foot_surface_ids[1] = 0;
    fields.auxiliary_vectors[0] = [0.0; 4];
    fields.requested_physics_mode = 1;
    for vector in &mut fields.auxiliary_vectors[1..] {
        *vector = [0.0; 4];
    }
    for bone in 0..count {
        // Native completes local first, then hierarchy, for each bone.
        packet.local[bone] = RESET_POSE;
        packet.hierarchy[bone] = RESET_POSE;
    }
    Ok(())
}
