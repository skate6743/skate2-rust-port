//! Complete field values for the native grind outputs; mounted only by the
//! actual nineteen-component constructor and reset in its original order.
use super::{RIGHT, SkeletonRootVector, UP, ZERO};
use crate::{
    physics::{FilteredPhysicsState, GrindType},
    play_animation::{ANDALE_NULL_STRING_NAME_30, EMPTY_FAST_STRING_30, FastString30},
};
/// Reset82b38fd0: five source zero-valued enums followed by `GRIND_TYPE_NONE`(-1).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GrindChromosome {
    pub(crate) approach: i32,
    pub(crate) location: i32,
    pub(crate) twist: i32,
    pub(crate) tilt: i32,
    pub(crate) orientation: i32,
    pub(crate) grind_type: GrindType,
}
impl GrindChromosome {
    pub(super) fn reset_values() -> Self {
        Self {
            approach: 0,
            location: 0,
            twist: 0,
            tilt: 0,
            orientation: 0,
            grind_type: GrindType::None,
        }
    }
}

/// `PhysOut_Grinds` reset82e2d388; GUID reset stores sign-extended li -1.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(crate) struct Grinds {
    pub(crate) grind_primitive_dir: SkeletonRootVector,
    pub(crate) grind_position: SkeletonRootVector,
    pub(crate) grind_normal: SkeletonRootVector,
    pub(crate) grind_skew: SkeletonRootVector,
    pub(crate) primitive_start_pos: SkeletonRootVector,
    pub(crate) primitive_end_pos: SkeletonRootVector,
    pub(crate) cam_target_root: SkeletonRootVector,
    pub(crate) ledge_high_dir: SkeletonRootVector,
    pub(crate) grind_impact_speed: f32,
    pub(crate) grind_crouch: f32,
    pub(crate) grind_type: GrindType,
    pub(crate) grind_substate: i32,
    pub(crate) skating_grind_type: i32,
    pub(crate) skating_grind_type_scoring: i32,
    pub(crate) grind_scorable_id: i32,
    pub(crate) skating_grind_name: FastString30,
    pub(crate) skating_grind_name_scoring: FastString30,
    pub(crate) anim_grind_name: FastString30,
    pub(crate) grind_audio_surface_type: i32,
    pub(crate) grind_pathed_guid: u64,
    pub(crate) grind_local_guid: u64,
    pub(crate) volatile_chromosome: GrindChromosome,
    pub(crate) grind_chromosome: GrindChromosome,
    pub(crate) grind_chromosome_scoring: GrindChromosome,
    pub(crate) is_grinding: bool,
    pub(crate) is_leaving_grind: bool,
    pub(crate) is_grind_on_front: bool,
    pub(crate) is_grind_facing_forwards: bool,
    pub(crate) is_grinding_ledge: bool,
    pub(crate) is_grinding_curb: bool,
    /// Skate 3 physics: PhysOut_Grinds+0x146, Tipslide FillPhysOut 82D42788's third flag
    /// (state +0x63). TU3 CalculateScoreCollectorType 82DA3FDC runs no Grind collector while set.
    pub(crate) skate3_tipslide_blocks_scoring: bool,
    pub(crate) did_jump: bool,
    pub(crate) hang_primitive_exists: bool,
}
impl Grinds {
    pub(crate) fn reset_values() -> Self {
        Self {
            grind_primitive_dir: RIGHT,
            grind_position: ZERO,
            grind_normal: UP,
            grind_skew: UP,
            primitive_start_pos: ZERO,
            primitive_end_pos: ZERO,
            cam_target_root: ZERO,
            ledge_high_dir: ZERO,
            grind_impact_speed: 0.0,
            grind_crouch: 0.0,
            grind_type: GrindType::None,
            grind_substate: 0,
            skating_grind_type: 0,
            skating_grind_type_scoring: 0,
            grind_scorable_id: -1,
            skating_grind_name: EMPTY_FAST_STRING_30,
            skating_grind_name_scoring: EMPTY_FAST_STRING_30,
            anim_grind_name: ANDALE_NULL_STRING_NAME_30,
            grind_audio_surface_type: 0,
            grind_pathed_guid: u64::MAX,
            grind_local_guid: u64::MAX,
            volatile_chromosome: GrindChromosome::reset_values(),
            grind_chromosome: GrindChromosome::reset_values(),
            grind_chromosome_scoring: GrindChromosome::reset_values(),
            is_grinding: false,
            is_leaving_grind: false,
            is_grind_on_front: false,
            is_grind_facing_forwards: false,
            is_grinding_ledge: false,
            is_grinding_curb: false,
            skate3_tipslide_blocks_scoring: false,
            did_jump: false,
            hang_primitive_exists: false,
        }
    }
}

/// `PhysOut_Filtered` reset82e2d618; GUID reset stores sign-extended li -1.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(crate) struct Filtered {
    pub(crate) filtered_physics_state: FilteredPhysicsState,
    pub(crate) last_filtered_physics_state: FilteredPhysicsState,
    pub(crate) skating_grind_name: FastString30,
    pub(crate) skating_grind_name_scoring: FastString30,
    pub(crate) grind_scorable_id: i32,
    pub(crate) grind_type: GrindType,
    pub(crate) grind_crouch: f32,
    pub(crate) last_grind_dist: f32,
    pub(crate) grind_pathed_guid: u64,
    pub(crate) grind_local_guid: u64,
    pub(crate) is_grinding: bool,
    pub(crate) is_grind_on_front: bool,
}
impl Filtered {
    pub(crate) fn reset_values() -> Self {
        Self {
            filtered_physics_state: FilteredPhysicsState::Invalid,
            last_filtered_physics_state: FilteredPhysicsState::Invalid,
            skating_grind_name: EMPTY_FAST_STRING_30,
            skating_grind_name_scoring: EMPTY_FAST_STRING_30,
            grind_scorable_id: -1,
            grind_type: GrindType::None,
            grind_crouch: 0.0,
            last_grind_dist: -1.0,
            grind_pathed_guid: u64::MAX,
            grind_local_guid: u64::MAX,
            is_grinding: false,
            is_grind_on_front: false,
        }
    }
}
