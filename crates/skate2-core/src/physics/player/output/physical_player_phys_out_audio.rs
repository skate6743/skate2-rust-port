//! The retained `PhysOut` values `AudioPresentation::FillSkaterData` 0x827f4c48 reads, copied for
//! the audio host. Direct const component reads, no write latch, like the presentation getters.
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
use crate::physical_player_phys_out::{
    PhysOutGroundVector, PhysOutMotionScalar, PhysOutMotionVector, PhysOutSystemVector,
};
use crate::skeleton_root_transforms::SkeletonRootVector;
use std::convert::Infallible;

/// `Sk8::Physics::SkeletonCollisionInfoAudio` (`PhysOut_Audio::mAveragedSkeletonCollisionInfo`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkaterAudioCollision {
    pub contact_force: [f32; 8],
    pub slide_speed: [f32; 8],
    pub audio_surface_type: [i32; 8],
    pub is_groin_contact: bool,
    pub is_face_contact: bool,
    pub vehicle_impact: f32,
    pub skater_impact: f32,
    pub pedestrian_impact: f32,
}

/// The `PhysOut` fields `FillSkaterData` reads, named after their native members. Vectors are
/// four lanes; a lane the port does not know (a velocity's W link) reads 0.
#[derive(Clone, Debug, Default, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent native output flags keep their meanings"
)]
pub struct SkaterAudioPhysOut {
    pub centre_of_mass: [f32; 4],
    pub centre_of_mass_velocity: [f32; 4],
    pub hips_mat_y: [f32; 4],
    pub foot_l_vel_deck_space: [f32; 4],
    pub foot_r_vel_deck_space: [f32; 4],
    pub foot_left_vel: [f32; 4],
    pub foot_right_vel: [f32; 4],
    pub torso_angular_vel: [f32; 4],
    pub right_foot_angular_vel: [f32; 4],
    pub left_foot_angular_vel: [f32; 4],
    pub absorption_compression: f32,
    /// Left leg, right leg, left arm, right arm.
    pub average_limb_speed: [f32; 4],
    pub is_wipeout_over: bool,
    /// Skate 3 physics: `PhysOut_State` +0x50 (a darkcatch), TU3 `FillSkaterData`'s SkaterData
    /// +0x98 bit 0x20; false in Skate 2 physics.
    pub skate3_dark_catch: bool,
    pub skate3_physics: bool,
    pub is_left_foot_on_deck: bool,
    pub is_right_foot_on_deck: bool,
    pub is_grabbing_left: bool,
    pub is_grabbing_right: bool,
    pub effective_transform_y: [f32; 4],
    pub deck_nose_dir: [f32; 4],
    pub deck_position: [f32; 4],
    pub wheel_pos: [[f32; 4]; 4],
    pub deck_velocity: [f32; 4],
    pub skateboard_surface_speed: f32,
    pub deck_tilt: f32,
    pub wheel_hardness: f32,
    pub time_in_known_air: f32,
    pub traj_collision_time: f32,
    pub jump_height: f32,
    pub foot_plant_surface_id: u32,
    pub hand_plant_surface_id: u32,
    pub is_skateboard_airborne: bool,
    pub is_starting_to_jump: bool,
    pub is_foot_planting: bool,
    pub is_left_foot_planting: bool,
    pub is_right_foot_planting: bool,
    pub hand_plant_grabbing: bool,
    pub num_wheels_in_contact: i32,
    pub front_truck_audio_surface_type: i32,
    pub back_truck_audio_surface_type: i32,
    pub deck_audio_surface_type: i32,
    pub deck_slide_speed: f32,
    pub wheel_audio_surface_type: [i32; 4],
    pub wheel_surface_pattern: [i32; 4],
    pub is_front_truck_in_contact: bool,
    pub is_back_truck_in_contact: bool,
    pub is_deck_in_contact: bool,
    pub physics_state: i32,
    pub is_foot_braking: bool,
    pub is_manual_braking: bool,
    pub is_physically_pushing: bool,
    pub is_generally_pushing: bool,
    pub is_generally_pushing_right_foot: bool,
    pub is_wiping_out: bool,
    pub is_manual: bool,
    pub is_reverting: bool,
    pub turn: f32,
    pub ground_up_vector: [f32; 4],
    pub deck_local_xyz_angular_speed: [f32; 4],
    pub wheel_slide: f32,
    pub jump_fraction: f32,
    pub grind_type: i32,
    pub grind_impact_speed: f32,
    pub averaged_deck_contact_force: f32,
    pub grind_audio_surface_type: i32,
    pub off_board_jump_landing: i32,
    pub hippy_jump_strength: i32,
    pub wheel_impact_strength: [f32; 4],
    pub wheel_landed: [bool; 4],
    pub averaged_skeleton_collision_info: SkaterAudioCollision,
    pub is_grinding: bool,
    pub is_on_stairs: bool,
    pub is_walking_up: bool,
    pub is_walking_down: bool,
    pub push_mongo: bool,
    pub line_score: i32,
    pub wipedout_trick_score: i32,
    pub banked_point_change_this_frame: i32,
    pub audio_banked_point_change: i32,
    pub raw_scorable_id: i32,
    /// Skate 3 physics: (AudioTrickType, AudioTrickFinishType, AudioIsTrickWithScrape) of the
    /// scorable's linked Skate 3 record, in place of the Skate 2 Vault lookup.
    pub skate3_scorable_audio: Option<(i32, i32, bool)>,
    pub left_foot_surface_id: u32,
    pub right_foot_surface_id: u32,
    pub is_grabbing_object: bool,
    pub left_foot_down: bool,
    pub right_foot_down: bool,
    pub is_unmounting: bool,
    pub is_air_dismount: bool,
    pub is_holding_skateboard: bool,
    pub is_jumping: bool,
    pub is_starting_standing_jump: bool,
    /// `PhysOut_Scoring2::mMomentumMeterMultiplier`, which `AudioPresentation::DoSimSubmit` turns
    /// into `GlobalData`'s multiplier flags.
    pub momentum_meter_multiplier: f32,
    /// `PhysOut_Scoring2::mHoM_BailData.mOverallWipeoutDurationInSeconds` and
    /// `PhysOut_Collision::mSkeletonCollisionInfo.mContactForceTotalDuringWipeout`, which
    /// `VDWriter::_PrepareBailData` 0x827fcd40 records as the bail's `mNormalizedBailValue`.
    pub overall_wipeout_duration: f32,
    pub contact_force_total_during_wipeout: f32,
    /// AudioPresentation GlobalData bit 22 and VDWriter::_PrepareBailData.
    pub hom_bail_in_progress: bool,
    pub hom_effect_enabled: bool,
    pub hom_score: u32,
}

fn lanes(vector: SkeletonRootVector) -> [f32; 4] {
    [vector.xyz[0], vector.xyz[1], vector.xyz[2], vector.w.unwrap_or(0.0)]
}

impl PhysicalPlayerPhysOut {
    /// What `AISkaterManager::UpdateAudioEvents` 0x824c3668 reads: `PhysOut_Audio::mJustWipedOut`
    /// and `mJustRespawned`, and `PhysOut_Scoring2::mBankedPointChangeThisFrame`.
    pub(crate) fn try_observed_events(&self) -> Result<(bool, bool, i32), ConcretePhysOutError<Infallible>> {
        self.require_complete_component(PhysOutComponent::Audio)?;
        self.require_complete_component(PhysOutComponent::Scoring2)?;
        let audio = self.concrete.audio.as_deref().expect("checked component");
        let scoring = self.concrete.scoring.as_deref().expect("checked component");
        Ok((audio.just_wiped_out, audio.just_respawned, scoring.banked_point_change_this_frame))
    }

    /// Every component `FillSkaterData` reads must be constructed.
    pub(crate) fn try_audio_phys_out(
        &self,
    ) -> Result<SkaterAudioPhysOut, ConcretePhysOutError<Infallible>> {
        for component in [
            PhysOutComponent::SkateboardReckoning,
            PhysOutComponent::Motion,
            PhysOutComponent::Air,
            PhysOutComponent::Skeleton,
            PhysOutComponent::Collision,
            PhysOutComponent::State,
            PhysOutComponent::Ground,
            PhysOutComponent::SystemReckoning,
            PhysOutComponent::Audio,
            PhysOutComponent::Animation,
            PhysOutComponent::Scoring2,
            PhysOutComponent::OffBoard,
        ] {
            self.require_complete_component(component)?;
        }
        let concrete = &self.concrete;
        let board = concrete.skateboard_reckoning.as_deref().expect("checked component");
        let motion = concrete.motion.as_deref().expect("checked component");
        let air = concrete.air.as_deref().expect("checked component");
        let skeleton = concrete.skeleton.as_deref().expect("checked component");
        let collision = concrete.collision.as_deref().expect("checked component");
        let state = concrete.state.as_deref().expect("checked component");
        let ground = concrete.ground.as_deref().expect("checked component");
        let audio = concrete.audio.as_deref().expect("checked component");
        let animation = concrete.animation.as_deref().expect("checked component");
        let scoring = concrete.scoring.as_deref().expect("checked component");
        let offboard = concrete.offboard.as_deref().expect("checked component");
        let system = |field: PhysOutSystemVector| {
            self.system_vectors[field as usize]
                .map(|sample| lanes(sample.value))
                .ok_or(ConcretePhysOutError::NotConstructed)
        };
        let deck_velocity = self.motion_vectors[PhysOutMotionVector::DeckVelocity as usize]
            .map(|sample| lanes(sample.value))
            .ok_or(ConcretePhysOutError::NotConstructed)?;
        let surface_speed = self.motion_scalars[PhysOutMotionScalar::SurfaceSpeed as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)?;
        let ground_up = self.ground_vectors[PhysOutGroundVector::GroundUp as usize]
            .map(|sample| lanes(sample.value))
            .ok_or(ConcretePhysOutError::NotConstructed)?;
        let effective = lanes(board.effective_transform.columns()[1]);
        let info = &audio.averaged_skeleton_collision_info;
        Ok(SkaterAudioPhysOut {
            centre_of_mass: system(PhysOutSystemVector::CentreOfMass)?,
            centre_of_mass_velocity: system(PhysOutSystemVector::CentreOfMassVelocity)?,
            hips_mat_y: lanes(skeleton.skeleton_hips_mat_y),
            foot_l_vel_deck_space: lanes(skeleton.skeleton_foot_l_vel_deck_space),
            foot_r_vel_deck_space: lanes(skeleton.skeleton_foot_r_vel_deck_space),
            foot_left_vel: lanes(skeleton.skeleton_foot_left_vel),
            foot_right_vel: lanes(skeleton.skeleton_foot_right_vel),
            torso_angular_vel: lanes(skeleton.skeleton_torso_angular_vel),
            right_foot_angular_vel: lanes(skeleton.skeleton_right_foot_angular_vel),
            left_foot_angular_vel: lanes(skeleton.skeleton_left_foot_angular_vel),
            absorption_compression: skeleton.absorption_compression,
            average_limb_speed: [
                skeleton.average_left_leg_speed_local,
                skeleton.average_right_leg_speed_local,
                skeleton.average_left_arm_speed_local,
                skeleton.average_right_arm_speed_local,
            ],
            is_wipeout_over: skeleton.is_wipeout_over,
            is_left_foot_on_deck: skeleton.is_left_foot_on_deck,
            is_right_foot_on_deck: skeleton.is_right_foot_on_deck,
            is_grabbing_left: skeleton.is_grabbing_left,
            is_grabbing_right: skeleton.is_grabbing_right,
            effective_transform_y: effective,
            deck_nose_dir: lanes(board.deck_nose_dir),
            deck_position: lanes(board.deck_position),
            wheel_pos: board.wheel_pos.map(lanes),
            deck_velocity,
            skateboard_surface_speed: surface_speed,
            deck_tilt: motion.deck_tilt,
            wheel_hardness: motion.wheel_hardness,
            time_in_known_air: air.time_in_known_air,
            traj_collision_time: air.traj_collision_time,
            jump_height: air.jump_height,
            foot_plant_surface_id: air.foot_plant_surface_id,
            hand_plant_surface_id: air.hand_plant_surface_id,
            is_skateboard_airborne: air.is_skateboard_airborne,
            is_starting_to_jump: air.is_starting_to_jump,
            is_foot_planting: air.is_foot_planting,
            is_left_foot_planting: air.is_left_foot_planting,
            is_right_foot_planting: air.is_right_foot_planting,
            hand_plant_grabbing: air.hand_plant_grabbing,
            num_wheels_in_contact: collision.wheels_in_contact,
            front_truck_audio_surface_type: collision.front_truck_audio_surface,
            back_truck_audio_surface_type: collision.back_truck_audio_surface,
            deck_audio_surface_type: collision.deck_audio_surface,
            deck_slide_speed: collision.deck_slide_speed,
            wheel_audio_surface_type: collision.wheel_audio_surface,
            wheel_surface_pattern: collision.wheel_surface_pattern,
            is_front_truck_in_contact: collision.front_truck_in_contact,
            is_back_truck_in_contact: collision.back_truck_in_contact,
            is_deck_in_contact: collision.deck_in_contact,
            physics_state: state.physics_state,
            is_foot_braking: state.is_foot_braking,
            is_manual_braking: state.is_manual_braking,
            is_physically_pushing: state.is_physically_pushing,
            is_generally_pushing: state.is_generally_pushing,
            is_generally_pushing_right_foot: state.is_generally_pushing_right_foot,
            is_wiping_out: state.is_wiping_out,
            skate3_dark_catch: state.skate3_dark_catch,
            skate3_physics: state.skate3_physics,
            is_manual: state.is_manual,
            is_reverting: state.is_reverting,
            turn: ground.turn,
            ground_up_vector: ground_up,
            deck_local_xyz_angular_speed: lanes(audio.deck_local_xyz_angular_speed),
            wheel_slide: audio.wheel_slide,
            jump_fraction: audio.jump_fraction,
            grind_type: audio.grind_type,
            grind_impact_speed: audio.grind_impact_speed,
            averaged_deck_contact_force: audio.averaged_deck_contact_force,
            grind_audio_surface_type: audio.grind_audio_surface_type,
            off_board_jump_landing: audio.off_board_jump_landing,
            hippy_jump_strength: audio.hippy_jump_strength,
            wheel_impact_strength: audio.wheel_landing.impact_strength,
            wheel_landed: audio.wheel_landing.landed,
            averaged_skeleton_collision_info: SkaterAudioCollision {
                contact_force: info.contact_force,
                slide_speed: info.slide_speed,
                audio_surface_type: info.audio_surface_type,
                is_groin_contact: info.is_groin_contact,
                is_face_contact: info.is_face_contact,
                vehicle_impact: info.vehicle_impact,
                skater_impact: info.skater_impact,
                pedestrian_impact: info.pedestrian_impact,
            },
            is_grinding: audio.is_grinding,
            is_on_stairs: audio.is_on_stairs,
            is_walking_up: audio.is_walking_up,
            is_walking_down: audio.is_walking_down,
            push_mongo: animation.push_mongo,
            line_score: scoring.line_score,
            wipedout_trick_score: scoring.wipedout_trick_score,
            banked_point_change_this_frame: scoring.banked_point_change_this_frame,
            audio_banked_point_change: scoring.audio_banked_point_change,
            raw_scorable_id: scoring.raw_scorable_id,
            skate3_scorable_audio: scoring.skate3_scorable_audio,
            left_foot_surface_id: offboard.left_foot_surface_id,
            right_foot_surface_id: offboard.right_foot_surface_id,
            is_grabbing_object: offboard.is_grabbing_object,
            left_foot_down: offboard.left_foot_down,
            right_foot_down: offboard.right_foot_down,
            is_unmounting: offboard.is_unmounting,
            is_air_dismount: offboard.is_air_dismount,
            is_holding_skateboard: offboard.is_holding_skateboard,
            is_jumping: offboard.is_jumping,
            is_starting_standing_jump: offboard.is_starting_standing_jump,
            momentum_meter_multiplier: scoring.momentum_meter_multiplier,
            overall_wipeout_duration: scoring.ho_m_bail_data.overall_wipeout_duration_in_seconds,
            hom_bail_in_progress: scoring.ho_m_bail_in_progress,
            hom_effect_enabled: scoring.ho_m_is_effect_enabled,
            hom_score: scoring.ho_m_bail_data.score,
            contact_force_total_during_wipeout: collision.skeleton.contact_force_during_wipeout,
        })
    }
}
