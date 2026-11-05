//! Concrete SkaterAnim::GetPhysUpdateData, TU3 82B985E8.
//! This owns only fields written by that function, not the full AnimOutPhysIn
//! reset, motion-graph merge, attribute dispatch, or physical force consumers.
use super::NativeMatrix;
use crate::animation::commands::{
    batch::CompletedSetData,
    buffers::{BoneSlice, BufferError},
};

/// Optional object at SkaterAnim+15104. The original class name is unresolved.
pub struct AnimationSignal {
    pub name_hash: u32,
    pub active: u8,
}

/// Fields read or consumed from the concrete native SkaterAnim. Other object
/// fields remain with their owners. The two unknown requests are NOT push flags.
pub struct SkaterPublicationState {
    /// Native packed flags+15180 bits31,30,29,28 respectively.
    pub orientation_bit31: bool,
    pub mirrored: bool,
    pub riding_fakie: bool,
    pub weight_on_nose: bool,
    /// +15196 and +15188 are distinct enums. Only equality is tested here.
    pub relative_stance: i32,
    pub natural_stance: i32,
    /// Native flags bits16 and15, consumed at publication.
    pub request_bit16: bool,
    pub request_bit15: bool,
    /// Native flags bit20 and persistent count+15184.
    pub air_dismount_revert_requested: bool,
    pub air_dismount_revert_frames: i32,
    pub signal: Option<AnimationSignal>,
}

pub struct ActorPoseBuffers {
    /// Actor-owned hierarchy-global matrices, full SkaterAnim+13460.
    pub hierarchy: BoneSlice<NativeMatrix>,
    /// Actor-owned local matrices, full SkaterAnim+13468.
    pub local: BoneSlice<NativeMatrix>,
}

/// Physics-packet-owned arrays are separate from the animation allocations.
/// Count is packet+10380, which native publication reads without rewriting.
/// Unlisted packet fields must be preserved by the surrounding packet owner.
pub struct PhysicsPosePacket {
    pub bone_count: u32,
    pub hierarchy: Vec<NativeMatrix>,
    pub local: Vec<NativeMatrix>,
    pub timestep: f32,
    pub foot_surface_ids: [u32; 2],
    pub flags: u32,
    pub board_flipped: bool,
    pub mirrored: bool,
    pub riding_switch: bool,
    pub riding_fakie: bool,
    pub weight_forwards: bool,
    pub regular_stance: bool,
    pub air_dismount_revert_frames: i32,
}

/// Publish after the source worker completes. Before modifying state, this safe
/// adapter rejects invalid/uninitialized ranges; native assumes valid storage.
/// `signal_name` is the current string selected by native global830301E4 (the
/// inspected TU3 image selects "signup"). Supply it from verified startup data.
/// Concrete getters 82B970D8/82B97100/82B97128 are pure field reads here;
/// polymorphic replacement getters are outside this concrete SkaterAnim port.
pub fn publish(
    state: &mut SkaterPublicationState,
    completed: &CompletedSetData,
    poses: &ActorPoseBuffers,
    packet: &mut PhysicsPosePacket,
    timestep: f64,
    signal_name: &[u8],
) -> Result<i32, BufferError> {
    let count = packet.bone_count as usize;
    if packet.hierarchy.len() < count || packet.local.len() < count {
        return Err(BufferError::RangeOutsideAllocation);
    }
    let hierarchy = completed.buffers().matrices.read(poses.hierarchy, count)?;
    let local = completed.buffers().matrices.read(poses.local, count)?;

    publish_evaluated(state, &hierarchy, &local, packet, timestep, signal_name)
}

/// Same publication with host-owned evaluated matrices. Completing animation
/// evaluation before this call preserves the source dependency without
/// requiring the original job/cache allocation system in the game.
pub fn publish_evaluated(
    state: &mut SkaterPublicationState,
    hierarchy: &[NativeMatrix],
    local: &[NativeMatrix],
    packet: &mut PhysicsPosePacket,
    timestep: f64,
    signal_name: &[u8],
) -> Result<i32, BufferError> {
    let count = packet.bone_count as usize;
    if hierarchy.len() < count
        || local.len() < count
        || packet.hierarchy.len() < count
        || packet.local.len() < count
    {
        return Err(BufferError::RangeOutsideAllocation);
    }

    packet.timestep = timestep as f32;
    packet.foot_surface_ids = [0; 2];
    replace_flag(&mut packet.flags, 27, state.request_bit16);
    state.request_bit16 = false;
    replace_flag(&mut packet.flags, 23, state.request_bit15);
    state.request_bit15 = false;
    packet.flags &= !((1 << 25) | (1 << 26));
    if let Some(signal) = &mut state.signal {
        signal.name_hash = signal_hash(signal_name);
        signal.active = 0;
    }
    packet.hierarchy[..count].copy_from_slice(&hierarchy[..count]);
    packet.local[..count].copy_from_slice(&local[..count]);
    packet.board_flipped = state.orientation_bit31 ^ state.mirrored;
    packet.mirrored = state.mirrored;
    packet.riding_switch = state.relative_stance == 1;
    packet.riding_fakie = state.riding_fakie;
    packet.weight_forwards = if state.riding_fakie {
        !state.weight_on_nose
    } else {
        state.weight_on_nose
    };
    packet.regular_stance = state.natural_stance == 0;
    replace_flag(&mut packet.flags, 28, state.air_dismount_revert_requested);
    state.air_dismount_revert_requested = false;
    packet.air_dismount_revert_frames = state.air_dismount_revert_frames;
    Ok(state.air_dismount_revert_frames)
}

fn replace_flag(flags: &mut u32, bit: u32, value: bool) {
    *flags = (*flags & !(1 << bit)) | (u32::from(value) << bit);
}

/// 8296EA10: NUL-terminated, signed-byte name hashing. This deliberately does
/// not substitute a library hash or unsigned-byte variant.
pub fn signal_hash(name: &[u8]) -> u32 {
    let mut hash = 0_u32;
    for &byte in name.iter().take_while(|&&byte| byte != 0) {
        hash = hash
            .wrapping_shl(4)
            .wrapping_add((byte as i8 as i32) as u32);
        let high = hash & 0xf000_0000;
        if high != 0 {
            hash ^= (high >> 23) ^ high;
        }
    }
    hash
}
