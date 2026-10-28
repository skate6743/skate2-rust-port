//! Complete native `PhysOut` constructor82e2f2a8 and Reset82e2f740.
//! Component values live inside the existing assembly-owned output. Every source
//! child constructor completes before its pointer is published; NULL allocations
//! continue in source order, and later access/reset reports the actual NULL.
use super::{COMPONENTS, PhysOutComponent, PhysOutOrigin, PhysicalPlayerPhysOut};
use super::{collision, grinds, path, scoring, simple, trick};
#[path = "../air/physical_player_air_post_solve.rs"]
pub(crate) mod air_post_solve;
#[path = "board/physical_player_phys_out_board_fill.rs"]
pub(crate) mod board_fill;
#[path = "board/physical_player_phys_out_board_fill_contacts.rs"]
pub(crate) mod board_fill_contacts;
#[path = "board/physical_player_phys_out_board_fill_deck.rs"]
pub(crate) mod board_fill_deck;
#[path = "board/physical_player_phys_out_board_fill_final.rs"]
pub(crate) mod board_fill_final;
#[path = "board/physical_player_phys_out_board_fill_forward.rs"]
pub(crate) mod board_fill_forward;
#[path = "board/physical_player_phys_out_board_fill_ground.rs"]
pub(crate) mod board_fill_ground;
#[path = "board/physical_player_phys_out_board_fill_pumping.rs"]
pub(crate) mod board_fill_pumping;
#[path = "board/physical_player_phys_out_board_fill_wheels.rs"]
pub(crate) mod board_fill_wheels;
#[path = "board/physical_player_phys_out_board_fill_wobble.rs"]
pub(crate) mod board_fill_wobble;
#[path = "conditioning/physical_player_phys_out_conditioned_output.rs"]
pub(crate) mod conditioned_output;
#[path = "../conditioning/physical_player_conditioner_animation_output.rs"]
pub(crate) mod conditioner_animation_output;
#[path = "physical_player_phys_out_crouching.rs"]
pub(crate) mod crouching;
#[path = "../lifecycle/physical_player_physics_damped_com_output.rs"]
pub(crate) mod damped_com;
#[path = "../lifecycle/physical_player_physics_foot_side_output.rs"]
pub(crate) mod foot_side;
#[path = "../grind/physical_player_grind_state_fill.rs"]
pub(crate) mod grind_state_fill;
#[path = "physical_player_phys_out_grinds_conditioner_output.rs"]
mod grinds_conditioner_output;
#[path = "../ground/physical_player_ground_animation_fill.rs"]
pub(crate) mod ground_animation_fill;
#[path = "../air/physical_player_known_air_fill.rs"]
pub(crate) mod known_air_fill;
#[path = "../lifecycle/physical_player_physics_landing_quality_output.rs"]
pub(crate) mod landing_quality;
#[path = "physical_player_phys_out_landing_velocity.rs"]
pub(crate) mod landing_velocity;
#[path = "physical_player_phys_out_motion_heading.rs"]
pub(crate) mod motion_heading;
#[path = "physical_player_phys_out_offboard_air_state.rs"]
pub(crate) mod offboard_air_state;
#[path = "physical_player_phys_out_offboard_jump_state.rs"]
pub(crate) mod offboard_jump_state;
#[path = "physical_player_phys_out_outer_locality.rs"]
pub(crate) mod outer_locality;
#[path = "physical_player_phys_out_post_category.rs"]
pub(crate) mod post_category;
#[path = "physical_player_phys_out_audio.rs"]
pub(crate) mod audio;
#[path = "physical_player_phys_out_rumble.rs"]
pub(crate) mod rumble;
#[path = "physical_player_phys_out_presentation.rs"]
mod presentation;
#[path = "physical_player_phys_out_prior_input.rs"]
pub(crate) mod prior_input;
#[path = "physical_player_phys_out_pushing_state.rs"]
pub(crate) mod pushing_state;
#[path = "../board/physical_player_revert_fill.rs"]
pub(crate) mod revert_fill;
#[path = "physical_player_phys_out_skitch.rs"]
mod skitch;
#[path = "../lifecycle/physical_player_physics_stairs_output.rs"]
pub(crate) mod stairs;
#[path = "physical_player_phys_out_update5_animation.rs"]
pub(crate) mod update5_animation;
#[path = "physical_player_phys_out_update5_decisions.rs"]
pub(crate) mod update5_decisions;
#[path = "physical_player_phys_out_update5_finish.rs"]
pub(crate) mod update5_finish;
#[path = "../lifecycle/physical_player_update5_line_output.rs"]
pub(crate) mod update5_line_output;
#[path = "physical_player_phys_out_update5_push_intents.rs"]
pub(crate) mod update5_push_intents;
#[path = "../lifecycle/physical_player_update5_surface_check_output.rs"]
pub(crate) mod update5_surface_check;
#[path = "physical_player_phys_out_update5_trick_payload.rs"]
pub(crate) mod update5_trick_payload;
#[path = "physical_player_phys_out_vehicle_avoidance.rs"]
mod vehicle_avoidance;
#[path = "../wipeout/physical_player_wipeout_fill.rs"]
pub(crate) mod wipeout_state_fill;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysOutAllocation {
    Host,
    Null,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComponentStatus {
    Unattempted,
    AllocationNull,
    Constructed,
}
#[derive(Debug, PartialEq)]
pub(crate) enum ConcretePhysOutError<E> {
    AlreadyStarted,
    NotConstructed,
    Allocation(E),
    MissingComponent(PhysOutComponent),
}
pub(super) struct ConcretePhysOut {
    pub(super) complete: bool,
    pub(super) status: [ComponentStatus; 19],
    pub(super) skateboard_reckoning: Option<Box<simple::SkateboardReckoning>>,
    pub(super) motion: Option<Box<simple::MotionRemainder>>,
    pub(super) air: Option<Box<simple::Air>>,
    pub(super) physics: Option<Box<simple::Physics>>,
    pub(super) grinds: Option<Box<grinds::Grinds>>,
    pub(super) skeleton: Option<Box<simple::Skeleton>>,
    pub(super) collision: Option<Box<collision::CollisionRemainder>>,
    pub(super) state: Option<Box<simple::State>>,
    pub(super) ground: Option<Box<simple::GroundRemainder>>,
    pub(super) system: Option<Box<simple::SystemRemainder>>,
    pub(super) audio: Option<Box<simple::Audio>>,
    pub(super) camera: Option<Box<simple::Camera>>,
    pub(super) trick: Option<Box<trick::Trick>>,
    pub(super) intents: Option<Box<simple::Intents>>,
    pub(super) animation: Option<Box<simple::Animation>>,
    pub(super) scoring: Option<Box<scoring::Scoring2>>,
    pub(super) filtered: Option<Box<grinds::Filtered>>,
    pub(super) path: Option<Box<path::PathRecording>>,
    pub(super) offboard: Option<Box<simple::OffBoard>>,
}
impl ConcretePhysOut {
    pub(super) fn new() -> Self {
        Self {
            complete: false,
            status: [ComponentStatus::Unattempted; 19],
            skateboard_reckoning: None,
            motion: None,
            air: None,
            physics: None,
            grinds: None,
            skeleton: None,
            collision: None,
            state: None,
            ground: None,
            system: None,
            audio: None,
            camera: None,
            trick: None,
            intents: None,
            animation: None,
            scoring: None,
            filtered: None,
            path: None,
            offboard: None,
        }
    }
    fn construct_component(&mut self, component: PhysOutComponent) {
        match component {
            PhysOutComponent::SkateboardReckoning => {
                self.skateboard_reckoning =
                    Some(Box::new(simple::SkateboardReckoning::reset_values()));
            }
            PhysOutComponent::Motion => {
                self.motion = Some(Box::new(simple::MotionRemainder::reset_values()));
            }
            PhysOutComponent::Air => self.air = Some(Box::new(simple::Air::reset_values())),
            PhysOutComponent::Physics => {
                self.physics = Some(Box::new(simple::Physics::reset_values()));
            }
            PhysOutComponent::Grinds => {
                self.grinds = Some(Box::new(grinds::Grinds::reset_values()));
            }
            PhysOutComponent::Skeleton => {
                self.skeleton = Some(Box::new(simple::Skeleton::reset_values()));
            }
            PhysOutComponent::Collision => {
                self.collision = Some(Box::new(collision::CollisionRemainder::reset_values()));
            }
            PhysOutComponent::State => self.state = Some(Box::new(simple::State::reset_values())),
            PhysOutComponent::Ground => {
                self.ground = Some(Box::new(simple::GroundRemainder::reset_values()));
            }
            PhysOutComponent::SystemReckoning => {
                self.system = Some(Box::new(simple::SystemRemainder::reset_values()));
            }
            PhysOutComponent::Audio => self.audio = Some(Box::new(simple::Audio::reset_values())),
            PhysOutComponent::Camera => {
                self.camera = Some(Box::new(simple::Camera::reset_values()));
            }
            PhysOutComponent::Trick => self.trick = Some(Box::new(trick::Trick::reset_values())),
            PhysOutComponent::Intents => {
                self.intents = Some(Box::new(simple::Intents::reset_values()));
            }
            PhysOutComponent::Animation => {
                self.animation = Some(Box::new(simple::Animation::reset_values()));
            }
            PhysOutComponent::Scoring2 => self.scoring = Some(Box::new(scoring::Scoring2::new())),
            PhysOutComponent::Filtered => {
                self.filtered = Some(Box::new(grinds::Filtered::reset_values()));
            }
            PhysOutComponent::PathRecording => {
                self.path = Some(Box::new(path::PathRecording::new()));
            }
            PhysOutComponent::OffBoard => {
                self.offboard = Some(Box::new(simple::OffBoard::reset_values()));
            }
        }
    }
    fn reset_component(&mut self, component: PhysOutComponent) {
        match component {
            PhysOutComponent::SkateboardReckoning => {
                **self
                    .skateboard_reckoning
                    .as_mut()
                    .expect("checked component") = simple::SkateboardReckoning::reset_values();
            }
            PhysOutComponent::Motion => {
                **self.motion.as_mut().expect("checked component") =
                    simple::MotionRemainder::reset_values();
            }
            PhysOutComponent::Air => self.air.as_mut().expect("checked component").reset(),
            PhysOutComponent::Physics => {
                **self.physics.as_mut().expect("checked component") =
                    simple::Physics::reset_values();
            }
            PhysOutComponent::Grinds => {
                **self.grinds.as_mut().expect("checked component") = grinds::Grinds::reset_values();
            }
            PhysOutComponent::Skeleton => {
                **self.skeleton.as_mut().expect("checked component") =
                    simple::Skeleton::reset_values();
            }
            PhysOutComponent::Collision => {
                **self.collision.as_mut().expect("checked component") =
                    collision::CollisionRemainder::reset_values();
            }
            PhysOutComponent::State => {
                **self.state.as_mut().expect("checked component") = simple::State::reset_values();
            }
            PhysOutComponent::Ground => {
                **self.ground.as_mut().expect("checked component") =
                    simple::GroundRemainder::reset_values();
            }
            PhysOutComponent::SystemReckoning => {
                **self.system.as_mut().expect("checked component") =
                    simple::SystemRemainder::reset_values();
            }
            PhysOutComponent::Audio => self.audio.as_mut().expect("checked component").reset(),
            PhysOutComponent::Camera => {
                **self.camera.as_mut().expect("checked component") = simple::Camera::reset_values();
            }
            PhysOutComponent::Trick => {
                **self.trick.as_mut().expect("checked component") = trick::Trick::reset_values();
            }
            PhysOutComponent::Intents => {
                **self.intents.as_mut().expect("checked component") =
                    simple::Intents::reset_values();
            }
            PhysOutComponent::Animation => {
                **self.animation.as_mut().expect("checked component") =
                    simple::Animation::reset_values();
            }
            PhysOutComponent::Scoring2 => self.scoring.as_mut().expect("checked component").reset(),
            PhysOutComponent::Filtered => {
                **self.filtered.as_mut().expect("checked component") =
                    grinds::Filtered::reset_values();
            }
            PhysOutComponent::PathRecording => {
                self.path.as_mut().expect("checked component").reset();
            }
            PhysOutComponent::OffBoard => {
                **self.offboard.as_mut().expect("checked component") =
                    simple::OffBoard::reset_values();
            }
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(super) fn require_complete_component(
        &self,
        component: PhysOutComponent,
    ) -> Result<(), ConcretePhysOutError<core::convert::Infallible>> {
        if !self.concrete.complete {
            return Err(ConcretePhysOutError::NotConstructed);
        }
        if self.concrete.status[component as usize] != ComponentStatus::Constructed {
            return Err(ConcretePhysOutError::MissingComponent(component));
        }
        Ok(())
    }
    /// Direct const `PhysOut` reads in the condition callbacks do not sample the
    /// write latch. `GetPhysOut82e01698` only returns the retained outer pointer.
    pub(crate) fn try_animation_output(
        &self,
    ) -> Result<&simple::Animation, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Animation)?;
        Ok(self
            .concrete
            .animation
            .as_deref()
            .expect("checked component"))
    }
    pub(crate) fn try_filtered_output(
        &self,
    ) -> Result<&grinds::Filtered, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Filtered)?;
        Ok(self
            .concrete
            .filtered
            .as_deref()
            .expect("checked component"))
    }
    /// Direct const component read by native camera anchors; no write guard.
    pub(crate) fn try_camera_intents_output(
        &self,
    ) -> Result<&simple::Intents, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Intents)?;
        Ok(self.concrete.intents.as_deref().expect("checked component"))
    }
    /// Camera `IsBoneless82e34700` reads the actual Trick component boolean.
    pub(crate) fn try_camera_is_push_tricking(
        &self,
    ) -> Result<bool, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Trick)?;
        Ok(self
            .concrete
            .trick
            .as_deref()
            .expect("checked component")
            .is_push_tricking)
    }
    pub(crate) fn try_camera_motion_output(
        &self,
    ) -> Result<&simple::MotionRemainder, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        Ok(self.concrete.motion.as_deref().expect("checked component"))
    }
    pub(crate) fn try_camera_ground_vector(
        &self,
        field: super::PhysOutGroundVector,
    ) -> Result<super::SkeletonRootVector, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Ground)?;
        self.ground_vectors[field as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    pub(crate) fn try_camera_board_output(
        &self,
    ) -> Result<&simple::SkateboardReckoning, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::SkateboardReckoning)?;
        Ok(self
            .concrete
            .skateboard_reckoning
            .as_deref()
            .expect("checked component"))
    }
    /// Direct const component read by native camera anchors; no write guard.
    pub(crate) fn try_camera_system_output(
        &self,
    ) -> Result<&simple::SystemRemainder, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::SystemReckoning)?;
        Ok(self.concrete.system.as_deref().expect("checked component"))
    }
    /// Direct const collision enum read by Breadcrumb Update82ce1934..1944.
    /// The caller owns the output mutex; native has no physics write guard.
    pub(crate) fn try_breadcrumb_collision_surface(
        &self,
    ) -> Result<u32, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Collision)?;
        self.collision_type
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// Direct const `PhysOut_Collision+0` read by
    /// `VerifySkatingSessionMarkerRules`828aa5c0; no write guard.
    pub(crate) fn try_collision_wheels_in_contact(
        &self,
    ) -> Result<i32, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Collision)?;
        Ok(self
            .concrete
            .collision
            .as_deref()
            .expect("checked component")
            .wheels_in_contact)
    }
    /// Direct const component read by native camera anchors; no write guard.
    pub(crate) fn try_camera_grinds_output(
        &self,
    ) -> Result<&grinds::Grinds, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Grinds)?;
        Ok(self.concrete.grinds.as_deref().expect("checked component"))
    }
    /// Direct const vector read by native camera anchors; no write guard.
    pub(crate) fn try_camera_motion_vector(
        &self,
        field: super::PhysOutMotionVector,
    ) -> Result<super::SkeletonRootVector, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        self.motion_vectors[field as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// Direct const vector read by native camera anchors; no write guard.
    pub(crate) fn try_camera_system_vector(
        &self,
        field: super::PhysOutSystemVector,
    ) -> Result<super::SkeletonRootVector, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::SystemReckoning)?;
        self.system_vectors[field as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// `Actor::GetCentreOfMass` 0x825eda80 (`mCentreOfMass` +0x30) and
    /// `Actor::GetVelocity` 0x825ed338 (`mDampedCentreOfMassVelocity` +0x00):
    /// direct const `SystemReckoning` reads through `IActor::GetPhysOut`.
    pub(crate) fn try_system_centre_and_damped_velocity(
        &self,
    ) -> Result<
        (super::SkeletonRootVector, super::SkeletonRootVector),
        ConcretePhysOutError<core::convert::Infallible>,
    > {
        self.require_complete_component(PhysOutComponent::SystemReckoning)?;
        let centre = self.system_vectors[super::PhysOutSystemVector::CentreOfMass as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)?;
        let velocity = self
            .concrete
            .system
            .as_deref()
            .ok_or(ConcretePhysOutError::NotConstructed)?
            .damped_centre_of_mass_velocity;
        Ok((centre, velocity))
    }
    pub(crate) fn try_skeleton_output(
        &self,
    ) -> Result<&simple::Skeleton, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Skeleton)?;
        Ok(self
            .concrete
            .skeleton
            .as_deref()
            .expect("checked component"))
    }
    /// Completed publication consumed by CheckForTeleport82e01db8 next Update1.
    pub(crate) fn try_completed_teleport(
        &self,
    ) -> Result<Option<crate::skeleton_root_transforms::SkeletonRootMatrix>, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::State)?;
        self.require_complete_component(PhysOutComponent::Motion)?;
        let state = self.concrete.state.as_deref().expect("checked component");
        let motion = self.concrete.motion.as_deref().expect("checked component");
        Ok((state.is_teleport_finished && motion.did_teleport).then_some(motion.teleport_transform))
    }
    pub(crate) fn try_state_output(
        &self,
    ) -> Result<&simple::State, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::State)?;
        Ok(self.concrete.state.as_deref().expect("checked component"))
    }
    /// `UpdateRidingFakie`82ca2b44 reads Motion+a8 directly, without a write latch.
    pub(crate) fn try_motion_z_speed(
        &self,
    ) -> Result<f32, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        self.motion_scalars[super::PhysOutMotionScalar::ZSpeed as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// DeckSpeed at Motion+0xa0, distinct from SurfaceSpeed at Motion+0xa4.
    /// This direct component accessor does not sample the write latch.
    pub(crate) fn try_motion_deck_speed(
        &self,
    ) -> Result<f32, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        self.motion_scalars[super::PhysOutMotionScalar::DeckSpeed as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// `SetSpeed::Update` 0x82ca2358 reads Motion+0xa4 directly, without a write latch.
    pub(crate) fn try_motion_surface_speed(
        &self,
    ) -> Result<f32, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        self.motion_scalars[super::PhysOutMotionScalar::SurfaceSpeed as usize]
            .map(|sample| sample.value)
            .ok_or(ConcretePhysOutError::NotConstructed)
    }
    /// `IsHandPlanting`82c9acc8 reads the actual Air component at PhysOut+8.
    pub(crate) fn try_air_output(
        &self,
    ) -> Result<&simple::Air, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Air)?;
        Ok(self.concrete.air.as_deref().expect("checked component"))
    }
    /// Direct const gesture-condition read; source samples no write latch.
    pub(crate) fn try_ground_output(
        &self,
    ) -> Result<&simple::GroundRemainder, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Ground)?;
        Ok(self.concrete.ground.as_deref().expect("checked component"))
    }
    /// The `PhysOut_Scoring2` fields `AI::PathController` reads: `mIsHippyJumping`,
    /// `mIsInState_Air`, `mCurrentAirData.mPlayerSpin180Count`, and each active trick's
    /// scorable (-1 when its settings are invalid).
    pub(crate) fn try_ai_scoring_view(
        &self,
    ) -> Result<(bool, bool, i32, Vec<i32>), ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Scoring2)?;
        let scoring = self.concrete.scoring.as_deref().expect("checked component");
        let count = usize::try_from(scoring.active_tricks.num_entries).unwrap_or(0).min(4);
        let active = scoring.active_tricks.elements[..count]
            .iter()
            .map(|trick| if trick.trick_scoring_settings.is_valid { trick.trick_scoring_settings.scorable_id } else { -1 })
            .collect();
        Ok((
            scoring.is_hippy_jumping,
            scoring.is_in_state_air,
            scoring.current_air_data.player_spin180_count,
            active,
        ))
    }
    /// Direct const gesture-condition read; source samples no write latch.
    pub(crate) fn try_offboard_output(
        &self,
    ) -> Result<&simple::OffBoard, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::OffBoard)?;
        Ok(self
            .concrete
            .offboard
            .as_deref()
            .expect("checked component"))
    }
    /// All nineteen constructors are concrete. This is separate from the
    /// existing selected collaborator projection, which cannot admit callbacks.
    pub(crate) fn try_construct_complete<E>(
        &mut self,
        mut allocate: impl FnMut(PhysOutComponent) -> Result<PhysOutAllocation, E>,
    ) -> Result<(), ConcretePhysOutError<E>> {
        if self.construction_started {
            return Err(ConcretePhysOutError::AlreadyStarted);
        }
        self.construction_started = true;
        for (index, component) in COMPONENTS.into_iter().enumerate() {
            match allocate(component).map_err(ConcretePhysOutError::Allocation)? {
                PhysOutAllocation::Null => {
                    self.concrete.status[index] = ComponentStatus::AllocationNull;
                }
                PhysOutAllocation::Host => {
                    self.concrete.construct_component(component);
                    self.reset_selected(component, PhysOutOrigin::Constructor);
                    self.concrete.status[index] = ComponentStatus::Constructed;
                }
            }
        }
        self.locked = Some(false);
        self.concrete.complete = true;
        self.selected_construction_complete = true;
        Ok(())
    }
    /// The source dereferences each component before advancing to the next.
    /// A NULL leaves earlier reset stores and the old write latch intact.
    pub(crate) fn try_reset_complete(
        &mut self,
    ) -> Result<(), ConcretePhysOutError<core::convert::Infallible>> {
        if !self.concrete.complete {
            return Err(ConcretePhysOutError::NotConstructed);
        }
        self.selected_reset_complete = false;
        self.motion_source_frame = None;
        for (index, component) in COMPONENTS.into_iter().enumerate() {
            if self.concrete.status[index] != ComponentStatus::Constructed {
                return Err(ConcretePhysOutError::MissingComponent(component));
            }
            self.concrete.reset_component(component);
            self.reset_selected(component, PhysOutOrigin::Reset);
        }
        self.locked = Some(false);
        self.selected_reset_complete = true;
        Ok(())
    }
    pub(crate) const fn complete_construction(&self) -> bool {
        self.concrete.complete
    }
    pub(crate) const fn component_statuses(&self) -> &[ComponentStatus; 19] {
        &self.concrete.status
    }
}

#[path = "skeleton/physical_player_phys_out_skeleton_fill_prefix.rs"]
pub(crate) mod skeleton_fill_prefix;

#[path = "physical_player_phys_out_heading_condition.rs"]
mod heading_condition;
#[path = "skeleton/physical_player_phys_out_skeleton_part_fill.rs"]
pub(crate) mod skeleton_part_fill;

#[path = "physical_player_phys_out_update5_state.rs"]
pub(crate) mod update5_state;

#[path = "skeleton/physical_player_phys_out_skeleton_fill_suffix.rs"]
pub(crate) mod skeleton_fill_suffix;

#[path = "skeleton/physical_player_phys_out_skeleton_ik_fill.rs"]
pub(crate) mod skeleton_ik_fill;

#[path = "skeleton/physical_player_phys_out_skeleton_audio.rs"]
pub(crate) mod skeleton_audio;

#[path = "physical_player_phys_out_pumping_fill.rs"]
pub(crate) mod pumping_fill;

#[path = "physical_player_phys_out_ground_state_fill.rs"]
pub(crate) mod ground_state_fill;
#[path = "../offboard/physical_player_offboard_state_fill.rs"]
pub(crate) mod offboard_state_fill;

#[path = "physical_player_phys_out_manager_fills.rs"]
pub(crate) mod manager_fills;

#[path = "physical_player_phys_out_plant_fill.rs"]
pub(crate) mod plant_fill;

#[path = "physical_player_phys_out_controller_fill.rs"]
pub(crate) mod controller_fill;

#[path = "physical_player_phys_out_wipeout_fill.rs"]
pub(crate) mod wipeout_fill;

impl PhysicalPlayerPhysOut {
    /// IsBumped82c9ce7c/ce8c and SetBumpCoefficients82cb1814/1824:
    /// PhysOut+38 is Animation, whose +70 is mDeckGroundAcceleration.
    /// Motion+70 is a different, world-space vector. No write-latch read.
    pub(crate) fn try_motion_bump_acceleration(
        &self,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootVector,
        ConcretePhysOutError<core::convert::Infallible>,
    > {
        self.require_complete_component(PhysOutComponent::Animation)?;
        Ok(self
            .concrete
            .animation
            .as_ref()
            .expect("checked component")
            .deck_ground_acceleration)
    }
}

impl PhysicalPlayerPhysOut {
    /// TrucksOrDeckInContact82c99b80..bb4: PhysOut+18 is Collision,
    /// +d70 is mIsATruckInContact and +d73 is mIsDeckInContact.
    /// These Motion reads do not sample the write latch.
    pub(crate) fn try_motion_trucks_or_deck_contact(
        &self,
    ) -> Result<bool, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Collision)?;
        let collision = self.concrete.collision.as_ref().expect("checked component");
        Ok(collision.a_truck_in_contact || collision.deck_in_contact)
    }
}

#[path = "physical_player_phys_out_landing_state.rs"]
pub(crate) mod landing_state;

#[path = "physical_player_phys_out_locomotion_state.rs"]
pub(crate) mod locomotion_state;
