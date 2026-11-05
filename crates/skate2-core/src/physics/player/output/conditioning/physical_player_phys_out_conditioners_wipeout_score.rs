//! `WipeoutScorer82df5618` owns three scoring children even in offline startup.
//! This is constructor ownership only; it does not admit or spawn DMO gameplay.
use super::{
    super::{
        SkeletonRootMatrix, SkeletonRootVector, ZERO,
        scoring::{AirDrop, EntityData},
    },
    Retained,
    collector_base::RotationTracker,
};
use crate::attribute_runtime::{
    AttributeRuntimeError, AttributeRuntimeVault,
};
use super::scoring_attributes::ScoringAttributes;
use std::sync::Arc;

fn attributes(vault: &Arc<AttributeRuntimeVault>) -> ScoringAttributes {
    // Raw82deda2c..44, default key at82df5624..40.
    ScoringAttributes::new(vault.instance(0x9e4c_471b_c6ea_b615, 0xd7ed_bd36_2d7d_2152), 0x9e4c_471b_c6ea_b615)
}
pub(super) struct EntityScorer {
    pub(super) attributes: ScoringAttributes,
    pub(super) collided_entities: [EntityData; 8],
    pub(super) count: u32,
    pub(super) tracker_score: u32,
    pub(super) score: u32,
    pub(super) num_new_collisions_this_frame: u32,
}
impl EntityScorer {
    ///82df1950 explicitly initializes every payload field despite noncleared allocation.
    pub(super) fn new(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            attributes: attributes(vault),
            collided_entities: std::array::from_fn(|_| EntityData::new()),
            count: 0,
            tracker_score: 0,
            score: 0,
            num_new_collisions_this_frame: 0,
        }
    }
}
pub(super) struct DmoScorer {
    pub(super) base: EntityScorer,
    pub(super) default_score: Option<u32>,
}
impl DmoScorer {
    pub(super) fn before_attribute(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            base: EntityScorer::new(vault),
            default_score: None,
        }
    }
    /// Constructor82df55a0 loads actual keyf54cf3579c22165b via generic lookup.
    /// NULL alone uses `DefaultDataArea`(4); malformed nonnull data is not zero.
    pub(super) fn finish_constructor(&mut self) -> Result<(), DmoScorerError> {
        let value = self
            .base
            .attributes
            .try_get_data(0xf54c_f357_9c22_165b, 0)
            .map_err(DmoScorerError::Attribute)?;
        self.default_score = Some(match value {
            None => 0,
            Some(data) => u32::from_be_bytes(
                data.bytes()
                    .get(..4)
                    .ok_or(DmoScorerError::TruncatedScore)?
                    .try_into()
                    .expect("checked four bytes"),
            ),
        });
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) enum DmoScorerError {
    Attribute(AttributeRuntimeError),
    TruncatedScore,
}

pub(super) struct CharacterArea {
    pub(super) damage_level: i32,
    pub(super) intensity_percentage: f32,
    pub(super) is_fading_in: bool,
}
pub(super) struct BoneInfo {
    pub(super) position: SkeletonRootVector,
    pub(super) position_last_frame: SkeletonRootVector,
    pub(super) damage_level: i32,
    pub(super) damage_level_last_frame: i32,
    pub(super) damage_amount: f32,
    pub(super) contact_surface_id: u32,
    pub(super) is_airborne: bool,
    /// Skate 3 physics: TU3 BoneInfo+44, the decaying running damage total.
    pub(super) running_damage: f32,
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent native wipeout scoring flags"
)]
pub(super) struct WipeoutScorer {
    pub(super) attributes: ScoringAttributes,
    pub(super) pedestrian: Option<Box<EntityScorer>>,
    pub(super) vehicle: Option<Box<EntityScorer>>,
    pub(super) dmo: Retained<Box<DmoScorer>>,
    pub(super) character_damage: [CharacterArea; 7],
    pub(super) pitch_tracker: RotationTracker,
    pub(super) yaw_tracker: RotationTracker,
    pub(super) roll_tracker: RotationTracker,
    pub(super) air_start_position: SkeletonRootVector,
    pub(super) air_apex_position: SkeletonRootVector,
    pub(super) bail_start_velocity: SkeletonRootVector,
    pub(super) output: Arc<()>,
    pub(super) bones: [BoneInfo; 25],
    pub(super) overall_wipeout_duration_frames: u32,
    pub(super) moving_wipeout_duration_frames: u32,
    pub(super) skeleton_wipeout_over_duration_frames: u32,
    pub(super) severe_wipeout_duration_frames: u32,
    pub(super) body_tweak_duration_frames: u32,
    pub(super) frames_since_heavy_landing_signal: i32,
    pub(super) bail_speed_score: u32,
    pub(super) current_rotation_score: u32,
    pub(super) max_rotation_score: u32,
    pub(super) body_tweak_score: u32,
    pub(super) current_body_tweak_score: u32,
    pub(super) num_body_tweaks_performed: u32,
    pub(super) banked_combined_rotation: f32,
    pub(super) current_combined_rotation: f32,
    pub(super) max_combined_rotation: f32,
    pub(super) left_foot_heavy_landing_speed: f32,
    pub(super) right_foot_heavy_landing_speed: f32,
    pub(super) current_body_tweak: i32,
    pub(super) current_air_drop: AirDrop,
    pub(super) max_air_drop: AirDrop,
    pub(super) accumulated_air_drop: AirDrop,
    pub(super) is_wiping_out: bool,
    pub(super) is_skeleton_wipeout_over: bool,
    pub(super) is_in_severe_wipeout: bool,
    pub(super) is_airborne: bool,
    pub(super) body_tweak_valid: bool,
    pub(super) just_performed_body_tweak: bool,
    #[cfg(feature = "skate3-runtime")]
    pub(super) skate3: super::scoring_wipeout_skate3::Skate3HoM,
}
impl WipeoutScorer {
    /// Retain before the three source child allocations. NULL outcomes publish
    /// NULL and continue; a failed DMO attribute lookup retains its partial child.
    pub(super) fn before_children(output: Arc<()>, vault: &Arc<AttributeRuntimeVault>) -> Self {
        let rotation = |rotation_type| RotationTracker {
            last_frame_matrix: Some(SkeletonRootMatrix::from_columns([ZERO; 4])),
            accumulated_angle_radians: 0.0,
            rotation_type,
        };
        Self {
            attributes: attributes(vault),
            pedestrian: None,
            vehicle: None,
            dmo: Retained::new(),
            character_damage: std::array::from_fn(|_| CharacterArea {
                damage_level: -1,
                intensity_percentage: 0.0,
                is_fading_in: false,
            }),
            // Raw82df56d0/d8/e0 stores pitch2, yaw0, roll1.
            pitch_tracker: rotation(2),
            yaw_tracker: rotation(0),
            roll_tracker: rotation(1),
            air_start_position: ZERO,
            air_apex_position: ZERO,
            bail_start_velocity: ZERO,
            output,
            bones: std::array::from_fn(|_| BoneInfo {
                position: ZERO,
                position_last_frame: ZERO,
                damage_level: -1,
                damage_level_last_frame: -1,
                damage_amount: 0.0,
                contact_surface_id: 0,
                is_airborne: false,
                running_damage: 0.0,
            }),
            overall_wipeout_duration_frames: 0,
            moving_wipeout_duration_frames: 0,
            skeleton_wipeout_over_duration_frames: 0,
            severe_wipeout_duration_frames: 0,
            body_tweak_duration_frames: 0,
            frames_since_heavy_landing_signal: -1,
            bail_speed_score: 0,
            current_rotation_score: 0,
            max_rotation_score: 0,
            body_tweak_score: 0,
            current_body_tweak_score: 0,
            num_body_tweaks_performed: 0,
            banked_combined_rotation: 0.0,
            current_combined_rotation: 0.0,
            max_combined_rotation: 0.0,
            left_foot_heavy_landing_speed: 0.0,
            right_foot_heavy_landing_speed: 0.0,
            current_body_tweak: -1,
            current_air_drop: AirDrop::new(),
            max_air_drop: AirDrop::new(),
            accumulated_air_drop: AirDrop::new(),
            is_wiping_out: false,
            is_skeleton_wipeout_over: false,
            is_in_severe_wipeout: false,
            is_airborne: false,
            body_tweak_valid: false,
            just_performed_body_tweak: false,
            #[cfg(feature = "skate3-runtime")]
            skate3: Default::default(),
        }
    }
}
