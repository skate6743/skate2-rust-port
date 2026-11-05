use super::{FrameFacts, StateSelectionInput, StateSelector};
use crate::player::state::PhysicalStateId;

const HANDPLANT_GROUND_DELAY_SECONDS: f32 = f32::from_bits(0x3E4C_CCCD);

impl StateSelector {
    pub(super) fn select_biped_or_plant(
        &self,
        current: PhysicalStateId,
        input: &StateSelectionInput,
        facts: FrameFacts,
    ) -> PhysicalStateId {
        let p = input.processed;
        match current {
            PhysicalStateId::BipedGround => {
                if facts.wipeout {
                    return PhysicalStateId::WipeoutGround;
                }
                if p.has_2484(0x1_0000) {
                    return PhysicalStateId::BipedAir;
                }
                if p.has_2476(0x20_0000) {
                    return PhysicalStateId::OffBoardPushing;
                }
                if p.has_2476(0x8000) || p.has_2480(0x40000) {
                    return current;
                }
                if !p.has_2484(1) && !p.has_2488(0x1000_0000) && !p.has_2488(0x8000_0000) {
                    return PhysicalStateId::PhysicsAir;
                }
                PhysicalStateId::PhysicsGround
            }
            PhysicalStateId::BipedAir => {
                if facts.wipeout {
                    return PhysicalStateId::WipeoutGround;
                }
                if !p.has_2480(0x10) {
                    //82D8B8D4 loads a BIG-ENDIAN halfword at2484, then
                    //B8D8 tests its low bit: bit16 of the stored u32.
                    //This is OffBoard328, not BipedBoardOnGround bit0.
                    if p.has_2484(0x1_0000) {
                        if p.has_2476(0x8000) || p.has_2476(0x80) {
                            current
                        } else {
                            PhysicalStateId::PhysicsAir
                        }
                    } else {
                        PhysicalStateId::BipedGround
                    }
                } else {
                    PhysicalStateId::LandingOnDeck
                }
            }
            PhysicalStateId::OffBoardPushing => {
                if facts.wipeout {
                    PhysicalStateId::WipeoutGround
                } else if !p.has_2476(0x20_0000) {
                    PhysicalStateId::BipedGround
                } else {
                    current
                }
            }
            PhysicalStateId::LandingOnDeck => {
                if facts.wipeout {
                    PhysicalStateId::WipeoutGround
                } else if p.has_2476(0x1_0000) {
                    PhysicalStateId::PhysicsGround
                } else if p.has_2476(0x80) {
                    PhysicalStateId::BipedAir
                } else {
                    current
                }
            }
            PhysicalStateId::HandPlant => {
                if facts.wipeout {
                    PhysicalStateId::WipeoutGround
                } else if (facts.colliding && p.state_timer_2664 > HANDPLANT_GROUND_DELAY_SECONDS)
                    || (p.flags_2480 as i32) >= 0
                {
                    PhysicalStateId::PhysicsGround
                } else {
                    current
                }
            }
            PhysicalStateId::FootPlant => {
                if facts.wipeout {
                    PhysicalStateId::WipeoutGround
                //82D8B970 loads the first byte of the big-endian word,
                //then tests its low bit: full-word bit24 is Air448.
                } else if !p.has_2480(0x0100_0000) {
                    p.air_variant_from_2468()
                } else {
                    current
                }
            }
            PhysicalStateId::Boneless => {
                if facts.wipeout {
                    return PhysicalStateId::WipeoutGround;
                }
                if p.has_2468(0x400) {
                    return PhysicalStateId::KnownAir;
                }
                if p.has_2476(0x80) {
                    return PhysicalStateId::BipedAir;
                }
                if !p.has_2480(0x4000) && !p.has_2480(0x2000) {
                    PhysicalStateId::PhysicsAir
                } else {
                    current
                }
            }
            _ => unreachable!("off-board/plant dispatch received another state"),
        }
    }
}

#[cfg(test)]
#[path = "tests/offboard.rs"]
mod tests;
