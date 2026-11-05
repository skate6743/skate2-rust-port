//! Actor field publication825937EC..825939C8 and external payload copy82592810.
//! Runtime services remain explicit source interfaces; their implementation is
//! not supplied here and these fields do not manufacture a gameplay impulse.
use super::{packet_reset::AdditionalResetFields, physics_packet::PhysicsPosePacket};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalPhysicsInput {
    /// Ten complete vectors; word representation preserves non-arithmetic bits.
    pub vectors: [[u32; 4]; 10],
    pub flags: u32,
}
impl ExternalPhysicsInput {
    ///82592810 copies160 bytes and only the high seven flag bits. Remaining
    /// destination flag bits and native padding are not overwritten.
    pub fn copy_from(&mut self, source: &Self) {
        self.vectors = source.vectors;
        self.flags = (self.flags & 0x01ff_ffff) | (source.flags & 0xfe00_0000);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalReset {
    pub transform: [[u32; 4]; 4],
    /// Source payload+64 -> packet10768; retains the complete byte.
    pub byte64: u8,
}

pub struct ActorPacketFields {
    pub external_impulse: [u32; 4],
    pub external_physics: ExternalPhysicsInput,
    pub external_reset: ExternalReset,
    /// Actor1904 bit29 -> packet10784; not reset by82590028.
    pub actor_flag_1904_bit29: bool,
}

pub struct ActorPublicationState {
    pub flags1904: u32,
    pub flags1908: u32,
    pub external_controller_present: bool,
    pub external_impulse: [u32; 4],
    pub external_physics: ExternalPhysicsInput,
    pub external_reset: ExternalReset,
    /// Actor1768 selects the native168-byte per-player settings record.
    pub player_index: u32,
}

/// A source-owned binding to the settings object captured before Actor+56's
/// virtual call. Each read uses the same captured object and168-byte stride.
pub trait PlayerPhysicsSettingsSource {
    type Error;
    /// Record+160. This read is skipped by the fixed-settings branch.
    fn truck_tightness(&mut self, player_index: u32) -> Result<f32, Self::Error>;
    /// Record+164. This read is skipped by the fixed-settings branch.
    fn wheel_hardness(&mut self, player_index: u32) -> Result<f32, Self::Error>;
    /// Record+172. Read after either branch has written both output scalars.
    fn requested_physics_mode(&mut self, player_index: u32) -> Result<u32, Self::Error>;
}

/// Exact unresolved external service boundaries for this field publisher.
/// A game adapter must implement these from their source-owned systems.
pub trait ActorPublicationEnvironment {
    /// Implementations must report unavailable source services as an error.
    /// There is deliberately no default or placeholder implementation.
    type Error;
    type Settings: PlayerPhysicsSettingsSource<Error = Self::Error>;
    ///8259FC28: global830CFD94 ->220 ->328 virtual12. None means no current
    /// object. Otherwise property824639E8(object20), including its native
    /// default, supplies the enum. Property identifier is-616069660.
    fn current_scene_mode(&mut self) -> Result<Option<i32>, Self::Error>;
    /// Read global byte830B7C30 verbatim, without bool normalization.
    fn ignore_respawn_reset_button(&mut self) -> Result<u8, Self::Error>;
    /// Actor subobject+56 virtual16, low return byte. Flags25/28 have already
    /// been consumed when called. Concrete vtable binding remains unresolved.
    fn actor_subobject56_slot16(
        &mut self,
        actor: &mut ActorPublicationState,
    ) -> Result<u8, Self::Error>;
    /// Capture global83067068 at82593954 before slot16 at8259395C. The binding
    /// must stay valid across that callback and the subsequent field reads.
    /// Do not snapshot record values here: native reads them after the callback.
    fn capture_physics_settings(&mut self) -> Result<Self::Settings, Self::Error>;
}

/// Full actor-owned suffix on distinct actor/packet payload storage. Scene
/// lookup is skipped for an external controller or force-braking bit26.
/// This stops before the physical object's borrowed-packet setter82DB3C48.
pub fn publish<E: ActorPublicationEnvironment>(
    actor: &mut ActorPublicationState,
    pose: &mut PhysicsPosePacket,
    reset: &mut AdditionalResetFields,
    fields: &mut ActorPacketFields,
    environment: &mut E,
) -> Result<(), E::Error> {
    reset.actor_flag_1904_bit23 = actor.flags1904 & (1 << 23) != 0;
    reset.actor_flag_1908_bit2 = actor.flags1908 & (1 << 2) != 0;
    reset.external_impulse_active = actor.flags1904 & (1 << 30) != 0;
    fields.external_impulse = actor.external_impulse;
    fields.external_physics.copy_from(&actor.external_physics);
    reset.external_physics_input_active = actor.flags1904 & (1 << 27) != 0;
    let scene_mode_flag = !actor.external_controller_present
        && actor.flags1904 & (1 << 26) == 0
        && matches!(environment.current_scene_mode()?, Some(19 | 20));
    replace_flag(&mut pose.flags, 22, scene_mode_flag);
    reset.externally_controlled = actor.external_controller_present;
    fields.external_reset = actor.external_reset;
    fields.actor_flag_1904_bit29 = actor.flags1904 & (1 << 29) != 0;
    reset.prevent_manual_respawn = actor.flags1904 & (1 << 28) != 0;
    reset.force_braking = actor.flags1904 & (1 << 26) != 0;
    replace_flag(&mut pose.flags, 24, actor.flags1904 & (1 << 25) != 0);
    actor.flags1904 &= !(1 << 25);
    reset.ignore_respawn_reset_button = environment.ignore_respawn_reset_button()?;
    actor.flags1904 &= !(1 << 28);
    let mut settings = environment.capture_physics_settings()?;
    let use_fixed_settings = environment.actor_subobject56_slot16(actor)? != 0;
    if use_fixed_settings {
        reset.truck_tightness = f32::from_bits(0x3f33_3333);
        reset.wheel_hardness = f32::from_bits(0x3f33_3333);
    } else {
        reset.truck_tightness = settings.truck_tightness(actor.player_index)?;
        reset.wheel_hardness = settings.wheel_hardness(actor.player_index)?;
    }
    reset.requested_physics_mode = settings.requested_physics_mode(actor.player_index)?;
    Ok(())
}

fn replace_flag(flags: &mut u32, bit: u32, value: bool) {
    *flags = (*flags & !(1 << bit)) | (u32::from(value) << bit);
}
