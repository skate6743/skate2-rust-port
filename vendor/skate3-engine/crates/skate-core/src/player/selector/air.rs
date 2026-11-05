use super::{FrameFacts, StateSelectionInput, StateSelector};
use crate::player::state::PhysicalStateId;

const STATE_SETTLE_SECONDS: f32 = f32::from_bits(0x3DA3_D70A);
const KNOWN_AIR_COLLISION_GRACE_SECONDS: f32 = f32::from_bits(0x3D23_D70A);
const TRAJECTORY_WIPEOUT_SECONDS: f32 = f32::from_bits(0x3D4C_CCCD);
const TRAJECTORY_EXPIRED_SECONDS: f32 = f32::from_bits(0xBE4C_CCCD);

impl StateSelector {
    pub(super) fn select_physics_air(
        &self,
        current: PhysicalStateId,
        input: &StateSelectionInput,
        facts: FrameFacts,
    ) -> PhysicalStateId {
        let p = input.processed;
        if facts.wipeout {
            return PhysicalStateId::WipeoutGround;
        }
        if p.has_2476(0x8000) {
            return PhysicalStateId::BipedGround;
        }
        //82D8B490 lbz2484 reads the high byte of the big-endian flag word.
        if p.has_2476(0x80) && !p.has_2484(0x0100_0000) {
            return PhysicalStateId::BipedAir;
        }
        if facts.force_known_air {
            if facts.colliding || p.has_2468(0x8000) {
                return PhysicalStateId::WipeoutGround;
            }
            return if p.has_2468(0x400) {
                PhysicalStateId::KnownAir
            } else {
                current
            };
        }
        if p.state_timer_2664 > STATE_SETTLE_SECONDS {
            if facts.colliding {
                if p.has_2472(4) {
                    return PhysicalStateId::PhysicsAir;
                }
                return if facts.skateboard_animated {
                    PhysicalStateId::GroundAnimation
                } else {
                    PhysicalStateId::PhysicsGround
                };
            }
            if self.post_grind_jump_counter > 10 {
                if let Some(grind) = facts.grind {
                    return grind;
                }
            }
        }
        if p.has_2468(0x400) {
            PhysicalStateId::KnownAir
        } else {
            current
        }
    }

    pub(super) fn select_known_air(
        &self,
        current: PhysicalStateId,
        input: &StateSelectionInput,
        facts: FrameFacts,
    ) -> PhysicalStateId {
        let p = input.processed;
        if p.has_2480(0x200_0000) {
            return PhysicalStateId::FootPlant;
        }
        if p.trajectory_collision_time_2772 < TRAJECTORY_WIPEOUT_SECONDS
            && (p.has_2476(0x80) || p.has_2476(0x8000))
        {
            return PhysicalStateId::WipeoutGround;
        }
        //82D8B5F0 uses the same byte load for the offboard dismount gate.
        if p.has_2476(0x80) && !p.has_2484(0x0100_0000) {
            return PhysicalStateId::BipedAir;
        }
        if p.has_2476(0x8000) {
            return PhysicalStateId::BipedGround;
        }
        if facts.wipeout {
            return PhysicalStateId::WipeoutGround;
        }
        if facts.force_known_air {
            return if facts.colliding || p.has_2468(0x8000) {
                PhysicalStateId::WipeoutGround
            } else {
                current
            };
        }
        if p.state_timer_2664 > STATE_SETTLE_SECONDS && self.post_grind_jump_counter > 10 {
            if let Some(grind) = facts.grind {
                return grind;
            }
        }
        if !facts.colliding || p.state_timer_2664 < KNOWN_AIR_COLLISION_GRACE_SECONDS {
            if p.trajectory_collision_time_2772 < TRAJECTORY_EXPIRED_SECONDS {
                return if p.has_2476(0x400_0000) {
                    PhysicalStateId::KnownAir
                } else {
                    PhysicalStateId::PhysicsAir
                };
            }
            if p.has_2468(0x8000) {
                return if p.has_2476(0x400_0000) {
                    PhysicalStateId::WipeoutGround
                } else {
                    PhysicalStateId::PhysicsAir
                };
            }
            return current;
        }
        if p.has_2472(4) {
            return PhysicalStateId::PhysicsAir;
        }
        if p.has_2480(0x40000) || p.has_2480(0x20000) || p.has_2480(0x10000) {
            return PhysicalStateId::WipeoutGround;
        }
        if !p.has_2472(0x8000) && !p.has_2472(0x4000) {
            PhysicalStateId::PhysicsGround
        } else {
            PhysicalStateId::GroundAnimation
        }
    }

    pub(super) fn select_grind(
        &self,
        _current: PhysicalStateId,
        input: &StateSelectionInput,
        facts: FrameFacts,
    ) -> PhysicalStateId {
        let p = input.processed;
        if facts.wipeout {
            return PhysicalStateId::WipeoutGround;
        }
        if p.has_2484(0x10_0000) {
            return PhysicalStateId::PhysicsAirSecondary;
        }
        if p.has_2476(0x80) {
            return PhysicalStateId::BipedAir;
        }
        if p.has_2476(0x8000) {
            return PhysicalStateId::BipedGround;
        }
        if p.has_2468(0x100) {
            return p.air_variant_from_2468();
        }
        facts.grind.unwrap_or(PhysicalStateId::Nonspecific)
    }
}
