//! Readable port of TU3 `PhysicalPlayerStateChanger::CalcSuggestedState`
//! (`0x82D8ADE8`) and its directly-called predicates.

mod air;
pub mod conditions;
mod ground;
pub mod input;
mod offboard;

use conditions::{
    condition_is_off_ground, condition_is_off_ground_skitching, is_skateboard_animated,
};
pub use input::{ProcessedStateInput, StateSelectionInput};

use crate::player::state::PhysicalStateId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StateSelector {
    pub current_state: Option<PhysicalStateId>,
    pub nonspecific_collision_free_frames: i32,
    pub nonspecific_collision_frames: i32,
    pub something_colliding_frames: i32,
    pub two_wheel_counter: i32,
    pub three_wheel_counter: i32,
    pub post_grind_jump_counter: i32,
    pub air_frames: i32,
    pub teleport_countdown: i32,
    pub skitch_exit_countdown: i32,
    pub revert_exited_normally: bool,
    pub request_teleport: bool,
}

#[derive(Clone, Copy)]
pub(super) struct FrameFacts {
    colliding: bool,
    wipeout: bool,
    force_known_air: bool,
    skateboard_animated: bool,
    off_ground: bool,
    off_ground_skitching: bool,
    grind: Option<PhysicalStateId>,
}

impl StateSelector {
    /// Executes the complete state decision tree and all state-changer-local
    /// counter/flag writes performed by TU3 `0x82D8ADE8`.
    pub fn calculate(
        &mut self,
        current: PhysicalStateId,
        input: &StateSelectionInput,
    ) -> PhysicalStateId {
        let p = input.processed;
        if self.teleport_countdown > 0 {
            self.teleport_countdown = self.teleport_countdown.wrapping_sub(1);
        }
        self.current_state = Some(current);
        self.revert_exited_normally = false;
        self.request_teleport = false;

        let colliding =
            p.wheel_contact_count_2556 > 0 || p.has_2468(0x1_0000) || p.has_2468(0x2_0000);
        self.nonspecific_collision_free_frames =
            if current == PhysicalStateId::Nonspecific && !colliding {
                self.nonspecific_collision_free_frames.wrapping_add(1)
            } else {
                0
            };
        self.nonspecific_collision_frames = if current == PhysicalStateId::Nonspecific && colliding
        {
            self.nonspecific_collision_frames.wrapping_add(1)
        } else {
            0
        };
        self.two_wheel_counter = if input.skateboard_contact_count_869 > 1 {
            self.two_wheel_counter.wrapping_add(1)
        } else {
            0
        };
        self.three_wheel_counter = if input.skateboard_contact_count_869 > 2 {
            self.three_wheel_counter.wrapping_add(1)
        } else {
            0
        };
        self.something_colliding_frames = if colliding {
            self.something_colliding_frames.wrapping_add(1)
        } else {
            0
        };
        let special_grind = p.category_2512 == 400 && p.has_2468(0x40_0000);
        self.post_grind_jump_counter = if special_grind {
            0
        } else {
            self.post_grind_jump_counter.wrapping_add(1)
        };
        self.air_frames = if p.category_2512 == 200 {
            if p.has_2468(8) {
                self.air_frames
            } else {
                self.air_frames.wrapping_add(1)
            }
        } else {
            0
        };
        self.skitch_exit_countdown = if current == PhysicalStateId::Skitching {
            10
        } else {
            if self.skitch_exit_countdown > 0 {
                self.skitch_exit_countdown.wrapping_sub(1)
            } else {
                0
            }
        };

        if p.has_2468(2) || p.has_2472(0x4_0000) {
            return PhysicalStateId::Teleporting;
        }
        if self.air_frames > 300 {
            self.request_teleport = true;
            return current;
        }
        if p.field_1776 < 0
            && (p.field_1776 as u32 & 0x200_0000) == 0
            && self.teleport_countdown == 0
        {
            return PhysicalStateId::FollowPath;
        }

        let facts = FrameFacts {
            colliding,
            wipeout: p.has_2468(0x4_0000),
            force_known_air: p.has_2472(0x40),
            skateboard_animated: is_skateboard_animated(input.skeleton),
            off_ground: condition_is_off_ground(input.board_body, input.normal_off_ground),
            off_ground_skitching: condition_is_off_ground_skitching(
                input.board_body,
                input.skitching_off_ground,
            ),
            grind: check_for_grind(p),
        };

        match current {
            PhysicalStateId::PhysicsGround
            | PhysicalStateId::SlideGround
            | PhysicalStateId::RevertGround
            | PhysicalStateId::GroundAnimation
            | PhysicalStateId::Skitching
            | PhysicalStateId::FollowPath => self.select_ground_family(current, input, facts),
            PhysicalStateId::PhysicsAir => self.select_physics_air(current, input, facts),
            PhysicalStateId::KnownAir => self.select_known_air(current, input, facts),
            PhysicalStateId::PhysicsAirSecondary => {
                if facts.wipeout {
                    PhysicalStateId::WipeoutGround
                } else if !p.has_2484(0x10_0000) {
                    PhysicalStateId::PhysicsAir
                } else {
                    current
                }
            }
            PhysicalStateId::WipeoutGround
            | PhysicalStateId::Sleeping
            | PhysicalStateId::Teleporting => current,
            PhysicalStateId::GrindBoardslide
            | PhysicalStateId::GrindFiftyFifty
            | PhysicalStateId::GrindTipslide
            | PhysicalStateId::GrindFiveO
            | PhysicalStateId::GrindBackslash
            | PhysicalStateId::GrindDarkslide => self.select_grind(current, input, facts),
            PhysicalStateId::BipedGround
            | PhysicalStateId::BipedAir
            | PhysicalStateId::OffBoardPushing
            | PhysicalStateId::LandingOnDeck
            | PhysicalStateId::HandPlant
            | PhysicalStateId::FootPlant
            | PhysicalStateId::Boneless => self.select_biped_or_plant(current, input, facts),
            PhysicalStateId::Nonspecific => self.select_nonspecific(current, input, facts),
        }
    }

    fn select_nonspecific(
        &self,
        current: PhysicalStateId,
        input: &StateSelectionInput,
        facts: FrameFacts,
    ) -> PhysicalStateId {
        let p = input.processed;
        if p.has_2484(0x10_0000) {
            return PhysicalStateId::PhysicsAirSecondary;
        }
        if facts.wipeout {
            return PhysicalStateId::WipeoutGround;
        }
        if p.has_2468(0x100) {
            return p.air_variant_from_2468();
        }
        if p.has_2468(0x40_0000) {
            return current;
        }
        if p.has_2472(8) {
            return if p.has_2484(0x20_0000) {
                PhysicalStateId::WipeoutGround
            } else {
                PhysicalStateId::PhysicsAir
            };
        }
        if p.has_2472(4) {
            return current;
        }
        if let Some(grind) = facts.grind {
            return grind;
        }
        if p.grind_investigation_flags_1516 & 0x800_0000 != 0 {
            if self.three_wheel_counter > 10 {
                return PhysicalStateId::PhysicsGround;
            }
        } else if self.two_wheel_counter > 2 {
            return if facts.skateboard_animated {
                PhysicalStateId::GroundAnimation
            } else {
                PhysicalStateId::PhysicsGround
            };
        }
        if self.nonspecific_collision_free_frames > 2 {
            return PhysicalStateId::PhysicsAir;
        }
        let collision_exit_frames = if p.has_2484(0x20_0000) { 30 } else { 60 };
        if self.nonspecific_collision_frames > collision_exit_frames {
            PhysicalStateId::PhysicsGround
        } else {
            current
        }
    }
}

fn check_for_grind(input: ProcessedStateInput) -> Option<PhysicalStateId> {
    if !input.grind_candidate_1488 || input.has_2480(0x2_0000) || input.has_2480(0x400_0000) {
        return None;
    }
    match input.grind_type_1248 {
        0 => Some(PhysicalStateId::GrindFiftyFifty),
        1 => Some(PhysicalStateId::GrindBoardslide),
        2 => Some(PhysicalStateId::GrindTipslide),
        3 => Some(PhysicalStateId::GrindFiveO),
        4 => Some(PhysicalStateId::GrindBackslash),
        5 => Some(PhysicalStateId::GrindDarkslide),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
