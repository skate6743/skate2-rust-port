use super::{FrameFacts, StateSelectionInput, StateSelector};
use crate::player::state::PhysicalStateId;

impl StateSelector {
    pub(super) fn select_ground_family(
        &mut self,
        current: PhysicalStateId,
        input: &StateSelectionInput,
        facts: FrameFacts,
    ) -> PhysicalStateId {
        let p = input.processed;
        match current {
            PhysicalStateId::PhysicsGround => {
                if p.has_2480(0x1000) && (!p.has_2480(0x400) || !p.has_2480(0x200)) {
                    // TU3 calls `sk83_na_f_01a4` at 0x82D8B23C here. The
                    // target is 0x82B61BB8 and consists solely of `blr`.
                    return PhysicalStateId::LandingOnDeck;
                }
                let handplant_contact =
                    p.has_2480(0x4000_0000) && input.skateboard_contact_count_869 <= 2;
                if p.has_2476(1) && (handplant_contact || input.skateboard_contact_count_869 == 0) {
                    return PhysicalStateId::HandPlant;
                }
                if p.has_2476(0x8000) {
                    return PhysicalStateId::BipedGround;
                }
                if p.has_2476(0x80) {
                    return PhysicalStateId::BipedAir;
                }
                if facts.wipeout {
                    return PhysicalStateId::WipeoutGround;
                }
                if p.field_2572 == 1 {
                    return p.air_variant_from_2468();
                }
                if p.field_2744 != 0.0 {
                    return PhysicalStateId::RevertGround;
                }
                if let Some(grind) = facts.grind {
                    return grind;
                }
                if !facts.colliding && facts.off_ground && self.teleport_countdown == 0 {
                    return PhysicalStateId::PhysicsAir;
                }
                if facts.skateboard_animated && self.teleport_countdown == 0 {
                    return PhysicalStateId::GroundAnimation;
                }
                if p.has_2476(0x20_0000) && p.has_2480(0x40_0000) && self.skitch_exit_countdown == 0
                {
                    return PhysicalStateId::Skitching;
                }
                if p.field_2732 != 0.0 {
                    PhysicalStateId::SlideGround
                } else {
                    current
                }
            }
            PhysicalStateId::SlideGround => {
                if facts.wipeout {
                    return PhysicalStateId::WipeoutGround;
                }
                if p.field_2744 != 0.0 {
                    return PhysicalStateId::RevertGround;
                }
                if let Some(grind) = facts.grind {
                    return grind;
                }
                if !facts.colliding && facts.off_ground {
                    return PhysicalStateId::PhysicsAir;
                }
                if p.field_2732 == 0.0 {
                    PhysicalStateId::PhysicsGround
                } else {
                    current
                }
            }
            PhysicalStateId::RevertGround => {
                if !facts.colliding && facts.off_ground {
                    return PhysicalStateId::PhysicsAir;
                }
                if p.has_2468(0x1_0000) || p.has_2472(0x2_0000) {
                    return PhysicalStateId::PhysicsGround;
                }
                if !p.has_2472(0x20_0000) {
                    self.revert_exited_normally = true;
                    return PhysicalStateId::PhysicsGround;
                }
                if facts.wipeout {
                    PhysicalStateId::WipeoutGround
                } else {
                    current
                }
            }
            PhysicalStateId::GroundAnimation => {
                if facts.wipeout {
                    return PhysicalStateId::WipeoutGround;
                }
                if p.has_2480(0x4000) {
                    return PhysicalStateId::Boneless;
                }
                if p.has_2476(0x8000) {
                    return PhysicalStateId::BipedGround;
                }
                if p.has_2476(0x80) {
                    return PhysicalStateId::BipedAir;
                }
                if p.field_2572 == 1 {
                    return p.air_variant_from_2468();
                }
                if !facts.colliding && facts.off_ground {
                    return PhysicalStateId::PhysicsAir;
                }
                if facts.skateboard_animated {
                    current
                } else {
                    PhysicalStateId::PhysicsGround
                }
            }
            PhysicalStateId::Skitching => {
                if !p.has_2476(0x20_0000) {
                    return PhysicalStateId::PhysicsGround;
                }
                if facts.wipeout {
                    return PhysicalStateId::WipeoutGround;
                }
                if facts.off_ground_skitching && self.teleport_countdown == 0 {
                    PhysicalStateId::PhysicsGround
                } else {
                    current
                }
            }
            PhysicalStateId::FollowPath => {
                if p.field_1776 as u32 & 0x200_0000 != 0 {
                    PhysicalStateId::PhysicsGround
                } else {
                    current
                }
            }
            _ => unreachable!("ground-family dispatch accepts only states 100..105"),
        }
    }
}
