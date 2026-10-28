//! The retained `PhysOut` values `RumbleManager::UpdateRumbleEvents` 0x825f64e0 and its fifteen
//! rumble events read, copied for the rumble manager. Direct const component reads, no write
//! latch, like the audio copy.
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
use crate::physical_player_phys_out::{PhysOutMotionScalar, PhysOutSystemVector};
use crate::physics::FilteredPhysicsState;
use std::convert::Infallible;

/// The `PhysOut` fields the rumble events read, named after their native members.
#[derive(Clone, Debug, Default, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent native output flags keep their meanings"
)]
pub(crate) struct SkaterRumblePhysOut {
    /// `PhysOut_Filtered::mFilteredPhysicsState`.
    pub(crate) filtered_physics_state: FilteredPhysicsState,
    /// `PhysOut_Skeleton::mIsLeftFootOnDeck` and `mIsRightFootOnDeck`.
    pub(crate) is_left_foot_on_deck: bool,
    pub(crate) is_right_foot_on_deck: bool,
    /// `PhysOut_Collision`.
    pub(crate) num_wheels_in_contact: i32,
    pub(crate) front_truck_audio_surface_type: i32,
    pub(crate) back_truck_audio_surface_type: i32,
    pub(crate) deck_audio_surface_type: i32,
    pub(crate) physics_surface_type: u32,
    pub(crate) wheel_audio_surface_type: i32,
    pub(crate) wheel_surface_pattern: i32,
    pub(crate) is_front_truck_in_contact: bool,
    pub(crate) is_back_truck_in_contact: bool,
    pub(crate) is_deck_in_contact: bool,
    /// `PhysOut_SkateboardMotion`.
    pub(crate) skateboard_surface_speed: f32,
    pub(crate) emperical_deck_acceleration: [f32; 3],
    pub(crate) speed_wobble_effect: f32,
    pub(crate) is_physically_pushing: bool,
    pub(crate) is_intentional_pump: bool,
    /// `PhysOut_Audio`.
    pub(crate) carving_slide: f32,
    pub(crate) wheel_slide: f32,
    pub(crate) averaged_deck_contact_force: f32,
    pub(crate) wheel_impact_strength: [f32; 4],
    pub(crate) wheel_landed: [bool; 4],
    pub(crate) prev_wheel_landed: [bool; 4],
    /// `mAveragedSkeletonCollisionInfo.mContactForce`.
    pub(crate) skeleton_contact_force: [f32; 8],
    pub(crate) left_foot_just_landed: bool,
    pub(crate) right_foot_just_landed: bool,
    /// `PhysOut_State`.
    pub(crate) is_foot_braking: bool,
    pub(crate) is_manual_braking: bool,
    pub(crate) physics_requests_wipeout: bool,
    /// `PhysOut_SystemReckoning::mCentreOfMassVelocity.y`.
    pub(crate) centre_of_mass_velocity_y: f32,
    /// `PhysOut_Ground::mPumpingAcceleration`.
    pub(crate) pumping_acceleration: f32,
    /// `PhysOut_Animation::mIsCarving`.
    pub(crate) is_carving: bool,
    /// `PhysOut_OffBoard`.
    pub(crate) is_grabbing_object: bool,
    pub(crate) object_mass: f32,
    pub(crate) anim_obj_angular_speed: f32,
    pub(crate) anim_obj_mov_x: f32,
    pub(crate) anim_obj_mov_z: f32,
}

impl PhysicalPlayerPhysOut {
    /// Every component the rumble events read must be constructed.
    pub(crate) fn try_rumble_phys_out(&self) -> Result<SkaterRumblePhysOut, ConcretePhysOutError<Infallible>> {
        for component in [
            PhysOutComponent::Motion,
            PhysOutComponent::Skeleton,
            PhysOutComponent::Collision,
            PhysOutComponent::State,
            PhysOutComponent::Ground,
            PhysOutComponent::SystemReckoning,
            PhysOutComponent::Audio,
            PhysOutComponent::Animation,
            PhysOutComponent::Filtered,
            PhysOutComponent::OffBoard,
        ] {
            self.require_complete_component(component)?;
        }
        let concrete = &self.concrete;
        let motion = concrete.motion.as_deref().expect("checked component");
        let skeleton = concrete.skeleton.as_deref().expect("checked component");
        let collision = concrete.collision.as_deref().expect("checked component");
        let state = concrete.state.as_deref().expect("checked component");
        let ground = concrete.ground.as_deref().expect("checked component");
        let audio = concrete.audio.as_deref().expect("checked component");
        let animation = concrete.animation.as_deref().expect("checked component");
        let filtered = concrete.filtered.as_deref().expect("checked component");
        let offboard = concrete.offboard.as_deref().expect("checked component");
        let centre_of_mass_velocity = self.system_vectors[PhysOutSystemVector::CentreOfMassVelocity as usize]
            .ok_or(ConcretePhysOutError::NotConstructed)?
            .value;
        let surface_speed = self.motion_scalars[PhysOutMotionScalar::SurfaceSpeed as usize]
            .ok_or(ConcretePhysOutError::NotConstructed)?
            .value;
        Ok(SkaterRumblePhysOut {
            filtered_physics_state: filtered.filtered_physics_state,
            is_left_foot_on_deck: skeleton.is_left_foot_on_deck,
            is_right_foot_on_deck: skeleton.is_right_foot_on_deck,
            num_wheels_in_contact: collision.wheels_in_contact,
            front_truck_audio_surface_type: collision.front_truck_audio_surface,
            back_truck_audio_surface_type: collision.back_truck_audio_surface,
            deck_audio_surface_type: collision.deck_audio_surface,
            physics_surface_type: self.collision_type.map_or(0, |sample| sample.value),
            wheel_audio_surface_type: collision.wheel_audio_surface[0],
            wheel_surface_pattern: collision.wheel_surface_pattern[0],
            is_front_truck_in_contact: collision.front_truck_in_contact,
            is_back_truck_in_contact: collision.back_truck_in_contact,
            is_deck_in_contact: collision.deck_in_contact,
            skateboard_surface_speed: surface_speed,
            emperical_deck_acceleration: motion.emperical_deck_acceleration.xyz,
            speed_wobble_effect: motion.speed_wobble_effect,
            is_physically_pushing: motion.is_physically_pushing,
            is_intentional_pump: motion.is_intentional_pump,
            carving_slide: audio.carving_slide,
            wheel_slide: audio.wheel_slide,
            averaged_deck_contact_force: audio.averaged_deck_contact_force,
            wheel_impact_strength: audio.wheel_landing.impact_strength,
            wheel_landed: audio.wheel_landing.landed,
            prev_wheel_landed: audio.prev_wheel_landing.landed,
            skeleton_contact_force: audio.averaged_skeleton_collision_info.contact_force,
            left_foot_just_landed: audio.left_foot_just_landed,
            right_foot_just_landed: audio.right_foot_just_landed,
            is_foot_braking: state.is_foot_braking,
            is_manual_braking: state.is_manual_braking,
            physics_requests_wipeout: state.physics_requests_wipeout,
            centre_of_mass_velocity_y: centre_of_mass_velocity.xyz[1],
            pumping_acceleration: ground.pumping_acceleration,
            is_carving: animation.is_carving,
            is_grabbing_object: offboard.is_grabbing_object,
            object_mass: offboard.object_mass,
            anim_obj_angular_speed: offboard.anim_obj_angular_speed,
            anim_obj_mov_x: offboard.anim_obj_mov_x,
            anim_obj_mov_z: offboard.anim_obj_mov_z,
        })
    }
}
