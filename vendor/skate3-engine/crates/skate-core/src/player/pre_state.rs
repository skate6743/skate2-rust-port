//! Complete TU3 `PhysicalPlayerHiLOD::PreState` (`0x82DB6050`).

/// The 72-byte stack packet filled through current-state vtable slot `+24`.
/// Only the vector at byte `+48` is consumed by the player wrapper, but the
/// entire packet remains part of the state callback contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreStatePacket {
    pub words: [u32; 18],
}

impl PreStatePacket {
    pub const ZERO: Self = Self { words: [0; 18] };

    pub const fn vector_48(self) -> [u32; 4] {
        [
            self.words[12],
            self.words[13],
            self.words[14],
            self.words[15],
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreStatePlayerFields {
    /// PhysicalPlayer+1312.
    pub frame_counter_1312: u32,
    /// Whether PhysicalPlayer+1840 owns the optional pre-state component.
    pub component_1840_present: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreStateSkeletonFields {
    /// Skeleton+16112.
    pub predicted_position_16112: [u32; 4],
    /// Skeleton+16416.
    pub predicted_position_set_16416: bool,
    /// Skeleton+16432 -> nested object +3184.
    pub nested_flag_3184: bool,
}

/// Every call made by the wrapper, in native order. There is deliberately no
/// default implementation: state and component behavior must be supplied by
/// their recovered modules.
pub trait PreStateServices {
    /// Current-state vtable slot `+24`.
    fn fill_packet_vtable_24(&mut self, packet: &mut PreStatePacket);
    /// `0x82D74270`, reached only when Player+1840 is present.
    fn update_component_1840_82d74270(&mut self);
    /// Current-state vtable slot `+4`.
    fn update_before_state_vtable_4(&mut self);
}

pub fn run_pre_state(
    player: &mut PreStatePlayerFields,
    skeleton: &mut PreStateSkeletonFields,
    services: &mut impl PreStateServices,
) {
    player.frame_counter_1312 = player.frame_counter_1312.wrapping_add(1);

    let mut packet = PreStatePacket::ZERO;
    services.fill_packet_vtable_24(&mut packet);
    skeleton.predicted_position_16112 = packet.vector_48();
    skeleton.predicted_position_set_16416 = true;
    skeleton.nested_flag_3184 = false;

    if player.component_1840_present {
        services.update_component_1840_82d74270();
    }
    services.update_before_state_vtable_4();
}

#[cfg(test)]
#[path = "tests/pre_state.rs"]
mod tests;
