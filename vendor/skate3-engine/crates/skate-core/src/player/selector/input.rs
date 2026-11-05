use super::conditions::{BoardBodyState, SkeletonAnimationState, TwoStageThresholds};
use crate::player::state::PhysicalStateId;

/// Values read by TU3 `PhysicalPlayerStateChanger::CalcSuggestedState`.
/// Offset-bearing names are retained where the producer's semantic name has
/// not yet been proved in Skate 3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessedStateInput {
    pub grind_type_1248: u32,
    pub grind_candidate_1488: bool,
    pub field_1776: i32,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub flags_2488: u32,
    pub category_2512: u32,
    pub wheel_contact_count_2556: i32,
    pub field_2572: u32,
    pub state_timer_2664: f32,
    pub field_2732: f32,
    pub field_2744: f32,
    pub trajectory_collision_time_2772: f32,
    pub grind_investigation_flags_1516: u32,
}

impl ProcessedStateInput {
    pub(crate) const fn has_2468(self, mask: u32) -> bool {
        self.flags_2468 & mask != 0
    }

    pub(crate) const fn has_2472(self, mask: u32) -> bool {
        self.flags_2472 & mask != 0
    }

    pub(crate) const fn has_2476(self, mask: u32) -> bool {
        self.flags_2476 & mask != 0
    }

    pub(crate) const fn has_2480(self, mask: u32) -> bool {
        self.flags_2480 & mask != 0
    }

    pub(crate) const fn has_2484(self, mask: u32) -> bool {
        self.flags_2484 & mask != 0
    }

    pub(crate) const fn has_2488(self, mask: u32) -> bool {
        self.flags_2488 & mask != 0
    }

    pub(crate) const fn air_variant_from_2468(self) -> PhysicalStateId {
        if self.has_2468(0x400) {
            PhysicalStateId::KnownAir
        } else {
            PhysicalStateId::PhysicsAir
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateSelectionInput {
    pub processed: ProcessedStateInput,
    /// Byte at `SkateboardBody + 869`.
    pub skateboard_contact_count_869: u8,
    pub board_body: BoardBodyState,
    pub skeleton: SkeletonAnimationState,
    /// Three collection values consumed by TU3 `0x82D8BBB8`.
    pub skitching_off_ground: TwoStageThresholds,
    /// Three physics-mode values consumed by TU3 `0x82D8BD50`.
    pub normal_off_ground: TwoStageThresholds,
}
