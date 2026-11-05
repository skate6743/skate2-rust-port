//! Conditioner capability publication from original Skate 3 TU3 0x8275F430.
//!
//! World writes ExternalInput+2780, then 0x82DA7888 copies that word to
//! PhysOutScoring2+204. The word gates controller and camera behavior; it is
//! not a count or mask of currently broken bones.

/// Host settings needed by the original whole-word capability calculation.
/// There is deliberately no implicit default for these gameplay choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionerCapabilityContext {
    /// Original FEBackEnd state equals 2 (in front end). This is a menu gate.
    pub in_front_end: bool,
    /// Original game settings +184 equals 1. UI option 29 is Hall of Meat mode.
    pub hall_of_meat_enabled: bool,
    /// Original challenge subsystem +328 virtual method at vtable+12 is nonzero.
    /// Its precise challenge interface name remains unidentified. A host with
    /// no challenge system supplies false rather than inventing that system.
    pub challenge_query_active: bool,
    /// Original challenge subsystem +512 points to a record whose byte 44 is
    /// nonzero. Only consulted when the challenge query above is active.
    pub challenge_configuration_enabled: bool,
}

impl ConditionerCapabilityContext {
    /// Preserve all bits and branch priority from 0x8275F4D8..0x8275F558.
    pub fn capabilities(self) -> u32 {
        if self.in_front_end {
            0
        } else if self.challenge_query_active && self.challenge_configuration_enabled {
            0xffff_fff7
        } else if self.hall_of_meat_enabled && !self.challenge_query_active {
            0xffff_f7f7
        } else {
            0x0000_01c0
        }
    }
}

#[cfg(test)]
mod tests;
