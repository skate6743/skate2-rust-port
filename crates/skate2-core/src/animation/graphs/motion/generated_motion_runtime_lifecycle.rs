//! Initial Motion Enter82952a98 and generated root/default Update callbacks.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, GeneratedMotionController,
    MotionUpdateError, NULL_INDEX,
};
use crate::actor_construction::motion_runtime_session::grinding::{
    GrindingRuntime, Phase as GrindingPhase,
    kernel::{ControlFadeNames, GrindControlFadeInstance, GrindCrouchInstance},
};
use crate::actor_construction::motion_runtime_session::neutral_lifecycle::ForcePhysicsWriter;
use crate::actor_construction::motion_runtime_session::set_turning::{
    TurningRuntime, kernel::Names,
};
use crate::{
    actor_construction::motion_runtime_session::play_animation::PlayAnimationActor,
    dynamic_state_graph_controller::DynamicStateGraphContext,
    generated_motion_load::{
        MotionAllocation,
        construct::{
            MotionBehaviourPayload as Payload, binding, control::MotionControlBehaviour as Control,
            names::NamedMotionBehaviour as Named, plain::PlainMotionBehaviour as Plain,
        },
        controller::{
            BehaviourInstanceSlot, MotionBehaviourInstance as Instance, ToggleBoardInstance,
        },
    },
    motion_graph_entity_authority::MotionGraphInstanceToken,
    play_animation_lifecycle::PlayAnimationInstanceAllocator,
    state_graph_context::StateGraphContextHandle,
};
use std::sync::Arc;

struct Allocation(bool);
impl PlayAnimationInstanceAllocator for Allocation {
    type Error = std::convert::Infallible;
    fn allocate(
        &mut self,
        _size: usize,
        _name: &[u8],
        _alignment: usize,
    ) -> Result<bool, Self::Error> {
        Ok(self.0)
    }
}

impl GeneratedMotionController<ActorIdentity> {
    /// Retained Update82955e20; no forced selection or rollback after callbacks.
    pub(crate) fn try_update(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
        dt: f32,
        allocate: &mut impl FnMut(u16, usize) -> MotionAllocation,
        writer: &mut dyn ForcePhysicsWriter,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let selected = self.select_next(actor, context, dt)?;
        if selected.state == NULL_INDEX {
            return Err(E::InvalidState(selected.state));
        }
        let graph = super::graph::Graph {
            states: &super::super::data::STATES,
            transitions: &super::super::data::TRANSITIONS,
            indices: &super::super::data::INDICES,
            current: self.current_state,
        };
        let (exited, entered) = graph.changes(&selected)?;
        for time in self.time_in_state.values_mut() {
            let updated = dt + *time;
            *time = updated;
        }
        self.last_state = self.current_state;
        self.current_state = selected.state;
        self.exit(actor, context, &exited)?;
        self.update_transition_hooks(actor, context, selected.transition)?;
        self.enter(actor, context, &entered, allocate, writer)?;
        self.update_behaviours(actor, context, writer)
    }

    /// `UpdateTransition` 0x8292b3d0 resolves each authored hook through the
    /// index array after Exit and before Enter, preserving object identity.
    fn update_transition_hooks(
        &self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        transition_index: u16,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        if transition_index == NULL_INDEX {
            return Ok(());
        }
        let transition = super::super::data::TRANSITIONS
            .get(usize::from(transition_index))
            .ok_or(E::InvalidTransition(transition_index))?;
        if transition.hooks.count == 0 {
            return Ok(());
        }
        for offset in 0..usize::from(transition.hooks.count) {
            let index = *super::super::data::INDICES
                .get(usize::from(transition.hooks.index) + offset)
                .ok_or(E::InvalidTransition(transition_index))?;
            let hook = self
                .hook(usize::from(index))
                .map_err(E::Load)?
                .ok_or(E::MissingOwner("Motion transition hook"))?;
            match &hook.payload {
                crate::generated_motion_load::hooks::HookPayload::InitializeLocomotion { .. } => actor.motion_initialize_locomotion(context, &hook.payload)?,
                crate::generated_motion_load::hooks::HookPayload::GrabSlide { right } => actor.motion_grab_slide_transition(context, *right)?,
                // Blend-only Override hooks 2/3/5/9/16, channelblend 10..12 and play hook 0
                // (transition 4, SpawnDismount31 -> Locomotion33; ctor82c8e428 "play" = 1) share
                // the audited PlayAnimation consumer. Channelblend hook 8 (only the 32 grind exits
                // to InAir21) adds blendWithCurrentFrame: TransitionAnim82d7aa68 passes it only to
                // TransitionBlend::Init82d6b810 (bit 29), read only by AddTime82d6df98.
                crate::generated_motion_load::hooks::HookPayload::OverrideNextAnimationTransition { .. } if matches!(transition_index,4|9|11|12|306|308|322|328..=335|348|361|374|387|400|413|426|439|452|465|478|491|498|505|512|519|526|533|540|547|576|605|634|663|1110|1122|1134|1146|1209|1221|1233|1245|1460..=1463|1522|1533|1540|1647..=1649) => actor.motion_apply_transition_hook(context, &hook.payload)?,
                crate::generated_motion_load::hooks::HookPayload::MongoPushToAntic { .. } => actor.motion_mongo_push_to_antic(context, &hook.payload)?,
                _ => return Err(E::PendingTransitionHook(transition_index)),
            }
        }
        Ok(())
    }

    /// Exit8292bd58 walks packed active slots backwards, then erases timers.
    fn exit(
        &mut self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        exited: &[u16],
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        for position in (0..self.current_behaviours.len()).rev() {
            let index = self.current_behaviours[position];
            let parent = super::super::data::BEHAVIOURS[usize::from(index)].parent;
            if parent == NULL_INDEX || !exited.contains(&parent) {
                continue;
            }
            if self.call_behaviours {
                let slot = &self.instances[position];
                let object = slot.behaviour.as_ref().ok_or(E::InvalidActiveSet)?;
                let mut copy = context.copy_for_behaviour();
                copy.set_generated_instance(
                    slot.instance
                        .as_ref()
                        .map(Instance::token)
                        .map(StateGraphContextHandle::shallow_copy),
                );
                match &object.payload {
                    Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::SkitchShimmying { .. }) => {
                        actor.motion_end_shimmy_begin(&copy, 0.3)?;
                    }
                    Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::HippyJumpAntic { .. })
                    | Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::MovingObject(_))
                    | Payload::Plain(Plain::AirDismounting) => {},
                    Payload::CharacterGesture
                    | Payload::EnterSkitchingBehaviour(_)
                    | Payload::SkitchingBehaviour(_)
                    | Payload::AttachIntent { .. }
                    | Payload::UpdateTimeSinceTeleport
                    | Payload::ResetSkaterAnimation
                    | Payload::ResetToGivenStance
                    | Payload::UpdateRidingFakie { .. }
                    | Payload::FakieHeadChannel { .. }
                    | Payload::SettingBodyTilt { .. }
                    | Payload::UpdateTimeSinceKickturn
                    | Payload::PowerSliding
                    | Payload::BodySpin
                    | Payload::StoreLandingData
                    | Payload::Plain(Plain::SetLandingData)
                    | Payload::Plain(Plain::InitPush)
                    | Payload::Plain(Plain::InitMovingObjects)
                    | Payload::Plain(Plain::ComputeFirstPushStrength)
                    | Payload::Named(Named::SetHandPlantAnticLength { .. })
                    | Payload::Named(Named::SetManualAngle { .. })
                    | Payload::Named(Named::JumpInto { .. })
                    | Payload::Named(Named::SetPushCoefs { .. })
                    | Payload::Named(Named::PushCycle { .. })
                    | Payload::Named(Named::PushOut { .. })
                    | Payload::ToggleBoard(_)
                    | Payload::Named(Named::ComputeTargetCoefsFromSpeedAndStrength { .. })
                    | Payload::Named(Named::SetSpeed { .. })
                    | Payload::Named(Named::KickTurnSteering { .. })
                | Payload::ControlAirLegExtension { .. }
                    | Payload::Named(Named::Crouching { .. } | Named::EndShimmy { .. } | Named::SetBumpCoefficients { .. } | Named::SetTrickHeight { .. })
                    | Payload::Named(Named::TweakProject { .. })
                    // SetDeckPitchAndYaw End825187c0: BLR.
                    | Payload::Named(Named::SetDeckPitchAndYaw { .. })
                    | Payload::Named(Named::SteppingBoardNoBoard { .. })
                    | Payload::Named(Named::CreateSlide { .. } | Named::PowerSlideDecel { .. })
                    | Payload::Plain(Plain::PowerSlideSpin)
                    | Payload::SetScoreAugmentation { .. }
                    | Payload::Control(Control::ScoringTrick { .. } | Control::ScoringGrabs(_) | Control::ScoringHandPlants(_))
                    | Payload::Control(Control::Wipeout { .. } | Control::SetTurning { .. } | Control::SetDistComToBoard { .. } | Control::ForcePhysics(_) | Control::PlayWipeoutRecoverAnim { .. })
                    // LandingOffBoardAir End825187c0: BLR.
                    | Payload::Control(Control::LandingOffBoardAir { .. })
                    // StandingJumpLandingCOG and StandingJumpDescent End825187c0: BLR.
                    | Payload::Named(Named::StandingJumpLandingCog { .. })
                    | Payload::Plain(Plain::StandingJumpDescent)
                    // FootPlantAbsorb End825187c0: BLR.
                    | Payload::Plain(Plain::FootPlantAbsorb)
                    // LandOnBoard End825187c0: BLR.
                    | Payload::Plain(Plain::LandOnBoard)
                    // FingerFlipOut End825187c0: BLR.
                    | Payload::Named(Named::FingerFlipOut { .. })
                    // CreateGrindAttributes, ControlGrindCrouch, GrindControlFade End825187c0: BLR.
                    | Payload::Plain(Plain::CreateGrindAttributes)
                    | Payload::Control(Control::ControlGrindCrouch { .. } | Control::GrindControlFade { .. }) => {}
                    Payload::Named(Named::ComputeRepushDeadline { regular_attributes }) => {
                        let services = actor.action_runtime_services().map_err(E::Actor)?;
                        actor.motion_runtime_compute_repush_deadline_end(
                            &copy,
                            *regular_attributes,
                            services.as_ref().map(|services| &services.pushing),
                        )?;
                    }
                    Payload::CreateAttribute(payload) => actor.motion_runtime_create_attribute(
                        &copy, payload,
                        crate::actor_construction::motion_runtime_session::create_attribute::kernel::Phase::End,
                    )?,
                    Payload::Plain(Plain::UpdateManualOutTimer) => {
                        actor.motion_runtime_clear_manual_out_timer(&copy)?;
                    }
                    Payload::Plain(Plain::ResetTimeSinceKickturn) => {
                        actor.motion_runtime_reset_time_since_kickturn(&copy)?;
                    }
                    Payload::Plain(Plain::Locomotion) => actor.motion_locomotion_end(&copy)?,
                    Payload::Plain(Plain::RunoutFront) => actor.motion_set_runout_front(&copy, false)?,
                    Payload::Plain(Plain::Dismount) => actor.motion_set_dismounting(&copy, false)?,
                    Payload::Plain(Plain::IsLanding) => actor.motion_set_is_landing(&copy,false)?,
                    Payload::Plain(Plain::OBAir) => actor.motion_set_obair_landing(&copy, false)?,
                    Payload::Control(Control::HandBusy(hand)) => {
                        actor.motion_adjust_hand_busy(&copy, *hand, -1)?;
                    }
                    Payload::Control(Control::EndGesture(_)) => {}
                    Payload::Named(Named::SetGrabType { .. }) => {
                        actor.motion_set_grab_type(&copy, None)?;
                    }
                    Payload::Plain(Plain::IsDoingTrick) => actor.motion_set_is_doing_trick(&copy, false)?,
                    Payload::Plain(Plain::IsManualing) => actor.motion_runtime_set_is_manualing(&copy, false)?,
                    Payload::Plain(Plain::InCandidateSlidingState) => actor.motion_runtime_set_candidate_sliding(&copy, false)?,
                    Payload::Plain(Plain::IsPowerSliding) => actor.motion_runtime_set_power_sliding(&copy, false)?,
                    Payload::Plain(Plain::DisallowPumping) => actor.motion_runtime_disallow_pumping(&copy, false)?,
                    Payload::Plain(Plain::IsAnticipating) => actor.motion_set_is_anticipating(&copy, false)?,
                    Payload::Plain(Plain::UpdateIsWeightOnNose) => actor.motion_set_weight_on_nose(&copy, false)?,
                    Payload::FilterMotionGraphIntent(payload) if matches!(index, 153 | 154 | 156 | 157 | 642..=649 | 1249 | 1250 | 1263 | 1264 | 1294 | 1295 | 1324 | 1325 | 1333 | 1340 | 1341 | 1349 | 1935) => {
                        actor.motion_filter_intent_end(&copy, payload.output_intent)?;
                    }
                    Payload::Control(Control::DisableTricks {..}) => actor.motion_set_allow_trick(&copy,true)?,
                    Payload::Plain(Plain::ClearTrickAttr) => actor.motion_clear_trick_attribute_end()?,
                    Payload::Named(Named::SetManualOutTimer { set_time }) => actor.motion_runtime_set_manual_out_timer(&copy, *set_time)?,
                    Payload::Plain(Plain::PowerSlideManualAtt) => {}, // End825187c0: BLR.
                    Payload::Named(Named::SetTrickAttr { .. }) => actor.motion_set_trick_attribute_end(&copy)?,
                    Payload::Named(Named::ChooseRandomSketchyLanding { random }) => actor.motion_sketchy_landing_attribute(&copy, *random, false)?,
                    Payload::Plain(Plain::ApplyingBodyTilt) => {
                        actor.motion_runtime_set_applying_body_tilt(&copy, false)?;
                    }
                    Payload::Plain(Plain::Pumping) => actor.motion_runtime_pumping(&copy, None, None, None, true)?,
                    Payload::PlayAnimation(behaviour) => behaviour.end_runtime(),
                    Payload::SharedAction(o)
                        if o.class == crate::generated_action_load::BehaviourClass::PrintText2D => {
                    }
                    _ => {
                        return Err(E::PendingCallback {
                            index,
                            address: binding(
                                super::super::data::BEHAVIOURS[usize::from(index)].name,
                            )
                            .ok_or(E::MissingOwner("Motion binding"))?
                            .callbacks[3],
                        });
                    }
                }
            }
            let old_count = self.current_behaviours.len();
            self.current_behaviours.remove(position);
            for slot in position..old_count - 1 {
                self.instances[slot] = std::mem::replace(
                    &mut self.instances[slot + 1],
                    BehaviourInstanceSlot {
                        behaviour: None,
                        instance: None,
                    },
                );
            }
            self.instances[old_count - 1] = BehaviourInstanceSlot {
                behaviour: None,
                instance: None,
            };
        }
        for state in exited {
            if self.time_in_state.remove(state).is_none() {
                return Err(E::InvalidActiveSet);
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "ordered lifecycle allocation and publication for concrete native behaviour classes"
    )]
    fn enter(
        &mut self,
        actor: &ActorConstruction,
        context: &BoundMotionGraphContext<ActorIdentity>,
        entered: &[u16],
        allocate: &mut impl FnMut(u16, usize) -> MotionAllocation,
        writer: &mut dyn ForcePhysicsWriter,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        for &state_index in entered {
            if !self.time_in_state.contains_key(&state_index) && self.time_in_state.len() == 11 {
                return Err(E::Capacity);
            }
            self.time_in_state.insert(state_index, 0.0);
            let state = &super::super::data::STATES[usize::from(state_index)];
            for offset in 0..state.behaviours.count {
                let index = state.behaviours.index + offset;
                let object = self
                    .load
                    .statics
                    .behaviour(usize::from(index))
                    .map_err(E::Load)?
                    .ok_or(E::MissingOwner("Motion behaviour"))?;
                if !object.base.enabled {
                    continue;
                }
                let instance = match &object.payload {
                    Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::SkitchShimmying { .. }) =>
                        (allocate(index, 0xc) == MotionAllocation::Host).then(|| Instance::SkitchShimmying {
                            token: MotionGraphInstanceToken::issue(), speed: 0.0 }),
                    Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::MovingObject(_)) =>
                        (allocate(index, 0x30) == MotionAllocation::Host).then(|| Instance::MovingObject {
                            token: MotionGraphInstanceToken::issue(),
                            data: crate::actor_animation_construction::moving_objects::MovingObjectsCoefficients::after_construction(),
                        }),
                    Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::HippyJumpAntic { .. }) =>
                        (allocate(index, 0xc) == MotionAllocation::Host).then(|| Instance::HippyJumpAntic {token: MotionGraphInstanceToken::issue(), time: 0.0}),
                    Payload::Plain(Plain::AirDismounting) =>
                        (allocate(index, 0xc) == MotionAllocation::Host).then(|| Instance::AirDismounting {token: MotionGraphInstanceToken::issue(), frames: 0}),
                    Payload::CharacterGesture
                    | Payload::UpdateTimeSinceTeleport
                    | Payload::ResetSkaterAnimation
                    | Payload::ResetToGivenStance
                    | Payload::AttachIntent { .. }
                    | Payload::UpdateTimeSinceKickturn
                    | Payload::SetScoreAugmentation { .. }
                    | Payload::Control(Control::ScoringTrick { .. } | Control::ScoringGrabs(_) | Control::ScoringHandPlants(_))
                    | Payload::CreateAttribute(_)
                    | Payload::Named(Named::SetHandPlantAnticLength { .. })
                    | Payload::Named(Named::ComputeTargetCoefsFromSpeedAndStrength { .. })
                    | Payload::Named(Named::ComputeRepushDeadline { .. })
                    | Payload::Named(Named::PushOut { .. })
                    | Payload::Named(Named::EndShimmy { .. } | Named::SetBumpCoefficients { .. } | Named::SetTrickHeight { .. })
                    | Payload::Named(Named::TweakProject { .. })
                    | Payload::Control(Control::SetDistComToBoard { .. } | Control::ForcePhysics(_))
                    | Payload::Control(Control::HandBusy(_))
                    | Payload::Control(Control::EndGesture(_))
                    | Payload::Named(Named::SetGrabType { .. })
                    | Payload::Control(Control::PlayWipeoutRecoverAnim { .. })
                    | Payload::Plain(Plain::RunoutFront | Plain::Dismount | Plain::Locomotion | Plain::ApplyingBodyTilt | Plain::UpdateManualOutTimer | Plain::ClearTrickAttr | Plain::IsLanding | Plain::IsDoingTrick | Plain::IsAnticipating | Plain::SetLandingData | Plain::InitPush | Plain::OBAir | Plain::UpdateIsWeightOnNose) => None,
                    Payload::Control(Control::DisableTricks {..}) => (allocate(index,0x10)==MotionAllocation::Host).then(|| Instance::DisableTricks {token:MotionGraphInstanceToken::issue(),data:crate::actor_construction::motion_runtime_session::landing_callbacks::timer::Instance::default()}),
                    Payload::Named(Named::Crouching { .. }) => {
                        let allocated = allocate(index, 0x38) == MotionAllocation::Host;
                        let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                        actor.motion_runtime_crouching_create(context, cvars.as_ref(), allocated)?
                            .map(|data| Instance::Crouching { token: MotionGraphInstanceToken::issue(), data })
                    }
                    Payload::Control(Control::Wipeout { .. }) =>
                        crate::actor_construction::motion_runtime_session::wipeout::kernel::WipeoutInstance::after_allocation(allocate(index, 0x24) == MotionAllocation::Host)
                            .map(|data| Instance::Wipeout { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::Control(Control::SetTurning { .. }) =>
                        crate::actor_construction::motion_runtime_session::set_turning::kernel::TurningInstance::try_create::<std::convert::Infallible>(|size, _alignment| Ok(allocate(index, size) == MotionAllocation::Host))
                            .map_err(|_| E::Capacity)?
                            .map(|data| Instance::SetTurning { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::Named(Named::KickTurnSteering { .. }) =>
                        crate::motion_kickturn_steering::KickTurnSteeringInstance::try_create::<std::convert::Infallible>(|size, _alignment| Ok(allocate(index, size) == MotionAllocation::Host))
                            .map_err(|_| E::Capacity)?
                            .map(|data| Instance::KickTurnSteering { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::SettingBodyTilt { .. } =>
                        crate::actor_construction::motion_runtime_session::setting_body_tilt::kernel::TiltInstance::after_allocation(allocate(index, 0x14) == MotionAllocation::Host)
                            .map(|data| Instance::SettingBodyTilt { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::PowerSliding =>
                        crate::actor_construction::motion_runtime_session::power_sliding::kernel::SlidingInstance::after_allocation(allocate(index, 0x18) == MotionAllocation::Host)
                            .map(|data| Instance::PowerSliding { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::Named(Named::PowerSlideDecel { .. }) =>
                        (allocate(index, 0x10) == MotionAllocation::Host)
                            .then(|| Instance::PowerSlideDecel { token: MotionGraphInstanceToken::issue(), value: 0.0 }),
                    Payload::BodySpin =>
                        crate::actor_construction::motion_runtime_session::body_spin::kernel::BodySpinInstance::try_create::<std::convert::Infallible>(|size, _alignment| Ok(allocate(index, size) == MotionAllocation::Host))
                            .map_err(|_| E::Capacity)?
                            .map(|data| Instance::BodySpin { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::ControlAirLegExtension { .. } => crate::actor_construction::motion_runtime_session::air_leg::kernel::AirLegInstance::after_allocation(allocate(index,0x14)==MotionAllocation::Host).map(|data| Instance::AirLeg {token:MotionGraphInstanceToken::issue(),data}),
                    Payload::StoreLandingData => (allocate(index, 0x10) == MotionAllocation::Host)
                        .then(|| Instance::StoreLandingData { token: MotionGraphInstanceToken::issue(),
                            data: crate::actor_construction::motion_runtime_session::store_landing_data::kernel::LandingInstance::default() }),
                    Payload::Plain(Plain::Pumping) =>
                        crate::actor_construction::motion_runtime_session::pumping::kernel::PumpingInstance::after_allocation(allocate(index, 0x18) == MotionAllocation::Host)
                            .map(|data| Instance::Pumping { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::Plain(Plain::ComputeFirstPushStrength) =>
                        crate::actor_construction::motion_runtime_session::compute_first_push::FirstPushInstance::after_allocation(allocate(index, 0x10) == MotionAllocation::Host)
                            .map(|data| Instance::FirstPush { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::Named(Named::SetManualAngle { .. }) => (allocate(index, 0x10) == MotionAllocation::Host)
                        .then(|| Instance::SetManualAngle { token: MotionGraphInstanceToken::issue(), angle: 0.0, velocity: 0.0 }),
                    Payload::Named(Named::JumpInto { .. }) => (allocate(index, 0x0c) == MotionAllocation::Host)
                        .then(|| Instance::JumpInto { token: MotionGraphInstanceToken::issue(), first_update: true }),
                    Payload::Plain(Plain::StandingJumpDescent) => (allocate(index, 0x0c) == MotionAllocation::Host)
                        .then(|| Instance::StandingJumpDescent { token: MotionGraphInstanceToken::issue(), begun: false }),
                    // Create82712148 allocates 0x0c, Create82cbce08 0x24; failure keeps null.
                    Payload::Named(Named::FingerFlipOut { .. }) => (allocate(index, 0x0c)
                        == MotionAllocation::Host)
                        .then(|| Instance::FingerFlipOut {
                            token: MotionGraphInstanceToken::issue(),
                            timer: 0.0,
                        }),
                    Payload::Control(Control::ControlGrindCrouch { .. }) => (allocate(index, 0x0c)
                        == MotionAllocation::Host)
                        .then(|| Instance::ControlGrindCrouch {
                            token: MotionGraphInstanceToken::issue(),
                            data: GrindCrouchInstance::default(),
                        }),
                    Payload::Control(Control::GrindControlFade { .. }) => (allocate(index, 0x24)
                        == MotionAllocation::Host)
                        .then(|| Instance::GrindControlFade {
                            token: MotionGraphInstanceToken::issue(),
                            data: GrindControlFadeInstance::after_allocation(),
                        }),
                    Payload::Control(Control::LandingOffBoardAir { .. }) => (allocate(index, 0x18) == MotionAllocation::Host)
                        .then(|| Instance::LandingOffBoardAir {
                            token: MotionGraphInstanceToken::issue(),
                            data: crate::actor_construction::motion_runtime_session::landing_offboard_air::LandingOffBoardAirInstance::default(),
                        }),
                    Payload::Named(Named::SetPushCoefs { .. }) =>
                        crate::actor_construction::motion_runtime_session::set_push_coefs::SetPushCoefsInstance::after_allocation(allocate(index, 0x0c) == MotionAllocation::Host)
                            .map(|data| Instance::SetPushCoefs { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::Named(Named::PushCycle { .. }) =>
                        crate::actor_construction::motion_runtime_session::push_cycle::PushCycleInstance::after_allocation(allocate(index, 0x10) == MotionAllocation::Host)
                            .map(|data| Instance::PushCycle { token: MotionGraphInstanceToken::issue(), data }),
                    Payload::UpdateRidingFakie { .. } => (allocate(index, 0x10)
                        == MotionAllocation::Host)
                        .then(|| Instance::RidingFakie {
                            token: MotionGraphInstanceToken::issue(),
                            slow_time: 0.0,
                            teleport_time: 0.0,
                        }),
                    Payload::EnterSkitchingBehaviour(_) => (allocate(index, 0x14) == MotionAllocation::Host)
                        .then(|| Instance::EnterSkitching { token: MotionGraphInstanceToken::issue(), data: Default::default() }),
                    Payload::SkitchingBehaviour(_) => (allocate(index, 0x10) == MotionAllocation::Host)
                        .then(|| Instance::Skitching { token: MotionGraphInstanceToken::issue(), data: Default::default() }),
                    Payload::ToggleBoard(_) => (allocate(index, 0x64)
                        == MotionAllocation::Host)
                        .then(|| Instance::ToggleBoard {
                            token: MotionGraphInstanceToken::issue(),
                            data: ToggleBoardInstance::after_allocation(),
                        }),
                    Payload::FilterMotionGraphIntent(_) if matches!(index, 153 | 154 | 156 | 157 | 642..=649 | 1249 | 1250 | 1263 | 1264 | 1294 | 1295 | 1324 | 1325 | 1333 | 1340 | 1341 | 1349 | 1935) =>
                        (allocate(index, 0x14) == MotionAllocation::Host)
                            .then(|| Instance::FilterMotionGraphIntent {
                                token: MotionGraphInstanceToken::issue(),
                                data: [0.0; 3],
                            }),
                    Payload::FakieHeadChannel { .. } => (allocate(index, 0x10)
                        == MotionAllocation::Host)
                        .then(|| Instance::FakieHead {
                            token: MotionGraphInstanceToken::issue(),
                            torso: 0.0,
                            active: false,
                        }),
                    Payload::PlayAnimation(behaviour) => behaviour
                        .create_runtime_instance(&mut Allocation(
                            allocate(index, 0xc) == MotionAllocation::Host,
                        ))
                        .map_err(|_| E::Capacity)?
                        .map(|data| Instance::PlayAnimation {
                            token: MotionGraphInstanceToken::issue(),
                            data,
                        }),
                    Payload::SharedAction(o)
                        if o.class == crate::generated_action_load::BehaviourClass::PrintText2D =>
                    {
                        None
                    }
                    _ if binding(super::super::data::BEHAVIOURS[usize::from(index)].name)
                        .is_some_and(|source| source.callbacks[0] == 0x82bb_3da8) =>
                    {
                        // Shared native CreateInstance stores a null intrusive
                        // pointer and returns; it allocates no behaviour state.
                        None
                    }
                    _ => {
                        return Err(E::PendingCallback {
                            index,
                            address: binding(
                                super::super::data::BEHAVIOURS[usize::from(index)].name,
                            )
                            .ok_or(E::MissingOwner("Motion binding"))?
                            .callbacks[0],
                        });
                    }
                };
                let position = self
                    .current_behaviours
                    .binary_search(&index)
                    .unwrap_or_else(|p| p);
                if self.current_behaviours.get(position) == Some(&index) {
                    return Err(E::InvalidActiveSet);
                }
                if self.current_behaviours.len() == 40 {
                    return Err(E::Capacity);
                }
                for slot in (position..self.current_behaviours.len()).rev() {
                    self.instances[slot + 1] = std::mem::replace(
                        &mut self.instances[slot],
                        BehaviourInstanceSlot {
                            behaviour: None,
                            instance: None,
                        },
                    );
                }
                self.instances[position] = BehaviourInstanceSlot {
                    behaviour: Some(object),
                    instance,
                };
                self.current_behaviours.insert(position, index);
                if self.call_behaviours {
                    self.callback(
                        actor,
                        &mut context.copy_for_behaviour(),
                        position,
                        true,
                        writer,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn update_behaviours(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
        writer: &mut dyn ForcePhysicsWriter,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        for (position, &index) in self.current_behaviours.iter().enumerate() {
            let original = self
                .load
                .statics
                .behaviour(usize::from(index))
                .map_err(E::Load)?
                .ok_or(E::InvalidActiveSet)?;
            if !self.instances[position]
                .behaviour
                .as_ref()
                .is_some_and(|object| Arc::ptr_eq(object, &original))
            {
                return Err(E::InvalidActiveSet);
            }
        }
        if !self.call_behaviours {
            return Ok(());
        }
        let state = &super::super::data::STATES[usize::from(self.current_state)];
        // Native1170 thunk828f39b8 expands ancestors inline, once per slot.
        // Calling ancestor thunks again would duplicate their updates.
        let expected: &[u16] = match (self.current_state, state.update_address) {
            // Skitch subtree thunks verified in IDA: OnGround ancestors first,
            // then each packed behaviour slot exactly once (see skitch evidence).
            (87, 0x828e_f070) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 175,
            ],
            (88, 0x828e_f0c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176,
            ],
            (270, 0x828f_01e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 175, 700,
            ],
            (271, 0x828f_01e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 175, 701,
            ],
            (272, 0x828f_0240) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707,
            ],
            (273, 0x828f_0308) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 708, 709, 710, 711, 712,
            ],
            (274, 0x828f_0308) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 713, 714, 715, 716, 717,
            ],
            (696, 0x828f_1948) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 175, 700, 1369, 1370, 1371,
            ],
            (697, 0x828f_19e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 175, 700, 1372, 1373,
            ],
            (698, 0x828f_1948) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 175, 701, 1374, 1375, 1376,
            ],
            (699, 0x828f_19e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 175, 701, 1377, 1378,
            ],
            (700, 0x828f_1a70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1379, 1380,
            ],
            (701, 0x828f_1ad0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381,
            ],
            (702, 0x828f_1b20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 708, 709, 710, 711, 712, 1382, 1383, 1384,
            ],
            (703, 0x828f_1b98) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 708, 709, 710, 711, 712, 1385, 1386, 1387,
            ],
            (704, 0x828f_1b20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 713, 714, 715, 716, 717, 1388, 1389, 1390,
            ],
            (705, 0x828f_1b98) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 713, 714, 715, 716, 717, 1391, 1392, 1393,
            ],
            (1009, 0x828f_2850) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1379, 1380, 1893, 1894, 1895,
            ],
            (1010, 0x828f_28f0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1379, 1380, 1896, 1897, 1898,
            ],
            (1011, 0x828f_2990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1899,
            ],
            (1012, 0x828f_2990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1900,
            ],
            (1013, 0x828f_29f0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1901,
            ],
            (1154, 0x828f_3678) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1899, 2147, 2148, 2149,
            ],
            (1155, 0x828f_3718) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1899, 2150, 2151,
            ],
            (1156, 0x828f_3678) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1900, 2152, 2153, 2154,
            ],
            (1157, 0x828f_3718) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1900, 2155, 2156,
            ],
            (1158, 0x828f_37a0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1901, 2157, 2158, 2159,
            ],
            (1159, 0x828f_3840) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 176, 702, 703, 704, 705, 706, 707, 1381, 1901, 2160, 2161, 2162,
            ],
            (268, 0x828f_0158) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 85, 86,
                87, 88, 89, 174, 694, 695, 696,
            ],
            (269, 0x828f_0158) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 85, 86,
                87, 88, 89, 174, 697, 698, 699,
            ],
            (657, 0x828f_1588) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1286, 1287, 1288,
            ],
            (668, 0x828f_1588) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1317, 1318, 1319,
            ],
            // StateUpdate_LeftFootStride_38 828eeb60 (also RightFootStride37):
            // OffBoard_1 for packed0..13, OBAir's BLR, CreateAttribute,
            // PlayAnimation, LandingOffBoardAir and ControlAirLegExtension.
            (37, 0x828e_eb60) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 118, 119, 120, 121, 122,
            ],
            (38, 0x828e_eb60) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 123, 124, 125, 126, 127,
            ],
            // StateUpdate_Start_115 828ef840: OffBoard_1 for packed0..13, then
            // CreateAttribute, RunoutFront's BLR, PlayAnimation, SetDistComToBoard's
            // BLR and three CreateAttribute Updates, one call per packed slot.
            (115, 0x828e_f840) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 63, 128, 245, 246, 247, 248, 249,
            ],
            // StateUpdate_Fly_116 828ef908 (also Finish117): OffBoard_1 for
            // packed0..13, CreateAttribute, RunoutFront's BLR, CreateAttribute.
            (116, 0x828e_f908) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 63, 128, 250],
            (117, 0x828e_f908) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 63, 128, 251],
            // NoPush1187 aliases Switch1031 (828f2fc0): Riding, turning,
            // score augmentation, AttachIntent and animation, once each.
            (1187, 0x828f_3bf0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1937, 1938, 1939,
            ],
            // StateUpdate_PushQueued_1188 828f4940 (also Right1294/Left1295):
            // Switch_1031 828f2fc0 for packed0..31, then direct
            // ComputeFirstPushStrength Update of packed32.
            (1188 | 1294 | 1295, 0x828f_4940) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1937, 1938, 1939, 2184,
            ],
            // StateUpdate_KickoutDismount_13 828ee2e0: OffBoard_1 828eddc8 for
            // packed0..13, PlayAnimation::Update packed14, ControlAirLegExtension packed15.
            (13, 0x828e_e2e0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 48, 49],
            // 828ee340 (symbol TrickDismount_15; KickoutToAir14 aliases it):
            // OffBoard_1, two CreateAttribute, PlayAnimation, ControlAirLegExtension.
            (14, 0x828e_e340) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 50, 51, 52, 53],
            (15, 0x828e_e340) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 54, 55, 56, 57],
            // Air986: source 0x828f_2668, ordered ancestor and leaf slots.
            (986, 0x828f_2668) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                642, 643, 1863, 1864,
            ],
            // Air987: source 0x828f_2668, ordered ancestor and leaf slots.
            (987, 0x828f_2668) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                644, 645, 1865, 1866,
            ],
            // Air988: source 0x828f_2668, ordered ancestor and leaf slots.
            (988, 0x828f_2668) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1867, 1868,
            ],
            // BSGrabInto1255: source 0x828f_40c0, ordered ancestor and leaf slots.
            (1255, 0x828f_40c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2273, 2274, 2275, 2276, 2277,
            ],
            // BSGrabCyc1256: source 0x828f_4188, ordered ancestor and leaf slots.
            (1256, 0x828f_4188) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2278, 2279, 2280, 2281,
            ],
            // BSGrabOut1257: source 0x828f_4238, ordered ancestor and leaf slots.
            (1257, 0x828f_4238) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2282,
            ],
            // BSGrabToFSGrab1258: source 0x828f_40c0, ordered ancestor and leaf slots.
            (1258, 0x828f_40c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2283, 2284, 2285, 2286, 2287,
            ],
            // BSGrabToDBLGrab1259: source 0x828f_42b0, ordered ancestor and leaf slots.
            (1259, 0x828f_42b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2288, 2289, 2290, 2291, 2292, 2293, 2294,
            ],
            // FSGrabInto1260: source 0x828f_40c0, ordered ancestor and leaf slots.
            (1260, 0x828f_40c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2295, 2296, 2297, 2298, 2299,
            ],
            // FSGrabCyc1261: source 0x828f_4188, ordered ancestor and leaf slots.
            (1261, 0x828f_4188) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2300, 2301, 2302, 2303,
            ],
            // FSGrabOut1262: source 0x828f_4238, ordered ancestor and leaf slots.
            (1262, 0x828f_4238) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2304,
            ],
            // FSGrabToBSGrab1263: source 0x828f_40c0, ordered ancestor and leaf slots.
            (1263, 0x828f_40c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2305, 2306, 2307, 2308, 2309,
            ],
            // FSGrabToDBLGrab1264: source 0x828f_42b0, ordered ancestor and leaf slots.
            (1264, 0x828f_42b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2310, 2311, 2312, 2313, 2314, 2315, 2316,
            ],
            // DBLGrabInto1265: source 0x828f_42b0, ordered ancestor and leaf slots.
            (1265, 0x828f_42b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2317, 2318, 2319, 2320, 2321, 2322, 2323,
            ],
            // DBLGrabCyc1266: source 0x828f_40c0, ordered ancestor and leaf slots.
            (1266, 0x828f_40c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2324, 2325, 2326, 2327, 2328,
            ],
            // DBLGrabOut1267: source 0x828f_4238, ordered ancestor and leaf slots.
            (1267, 0x828f_4238) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2329,
            ],
            // DBLGrabToBSGrab1268: source 0x828f_40c0, ordered ancestor and leaf slots.
            (1268, 0x828f_40c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2330, 2331, 2332, 2333, 2334,
            ],
            // DBLGrabToFSGrab1269: source 0x828f_40c0, ordered ancestor and leaf slots.
            (1269, 0x828f_40c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2335, 2336, 2337, 2338, 2339,
            ],
            // Idle1318: source 0x828f_4b08, ordered ancestor and leaf slots.
            (1318, 0x828f_4b08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2278, 2279, 2280, 2281,
            ],
            // Idle1321: source 0x828f_4b08, ordered ancestor and leaf slots.
            (1321, 0x828f_4b08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2300, 2301, 2302, 2303,
            ],
            // Idle1324: source 0x828f_4b60, ordered ancestor and leaf slots.
            (1324, 0x828f_4b60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2324, 2325, 2326, 2327, 2328,
            ],
            // 828ef110: OnBoard then AttachIntent20. 828f2198: Trick/flip
            // ancestry then PlayAnimation28, with no SetTrickHeight slot.
            (25, 0x828e_f110) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
            ],
            (90, 0x828e_f110) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
            ],
            (525, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 457, 458, 459, 460, 461, 1132,
            ],
            (528, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 462, 463, 464, 465, 466, 1133,
            ],
            (531, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 467, 468, 469, 470, 471, 1134,
            ],
            (534, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 472, 473, 474, 475, 476, 1135,
            ],
            (537, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 477, 478, 479, 480, 481, 1136,
            ],
            (540, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 482, 483, 484, 485, 486, 1137,
            ],
            (543, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 487, 488, 489, 490, 491, 1138,
            ],
            (546, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 492, 493, 494, 495, 496, 1139,
            ],
            (549, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 497, 498, 499, 500, 501, 1140,
            ],
            (552, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 502, 503, 504, 505, 506, 1141,
            ],
            (555, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 507, 508, 509, 510, 511, 1142,
            ],
            (558, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 512, 513, 514, 515, 516, 1143,
            ],
            (570, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 524, 525, 526, 527, 528, 1151,
            ],
            (573, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 529, 530, 531, 532, 533, 1152,
            ],
            (576, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 534, 535, 536, 537, 538, 1153,
            ],
            (579, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 539, 540, 541, 542, 543, 1154,
            ],
            (582, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 544, 545, 546, 547, 548, 1155,
            ],
            (585, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 549, 550, 551, 552, 553, 1156,
            ],
            (588, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 554, 555, 556, 557, 558, 1157,
            ],
            (591, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 559, 560, 561, 562, 563, 1158,
            ],
            (594, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 564, 565, 566, 567, 568, 1159,
            ],
            (597, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 569, 570, 571, 572, 573, 1160,
            ],
            (600, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 574, 575, 576, 577, 578, 1161,
            ],
            (603, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 579, 580, 581, 582, 583, 1162,
            ],
            (804, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 457, 458, 459, 460, 461, 1488,
            ],
            (809, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 462, 463, 464, 465, 466, 1499,
            ],
            (814, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 467, 468, 469, 470, 471, 1510,
            ],
            (819, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 472, 473, 474, 475, 476, 1521,
            ],
            (824, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 477, 478, 479, 480, 481, 1532,
            ],
            (829, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 482, 483, 484, 485, 486, 1543,
            ],
            (834, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 487, 488, 489, 490, 491, 1554,
            ],
            (839, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 492, 493, 494, 495, 496, 1565,
            ],
            (844, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 497, 498, 499, 500, 501, 1576,
            ],
            (849, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 502, 503, 504, 505, 506, 1587,
            ],
            (854, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 507, 508, 509, 510, 511, 1598,
            ],
            (859, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 512, 513, 514, 515, 516, 1609,
            ],
            (883, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 524, 525, 526, 527, 528, 1645,
            ],
            (888, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 529, 530, 531, 532, 533, 1656,
            ],
            (893, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 534, 535, 536, 537, 538, 1667,
            ],
            (898, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 539, 540, 541, 542, 543, 1678,
            ],
            (903, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 544, 545, 546, 547, 548, 1689,
            ],
            (908, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 549, 550, 551, 552, 553, 1700,
            ],
            (913, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 554, 555, 556, 557, 558, 1711,
            ],
            (918, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 559, 560, 561, 562, 563, 1722,
            ],
            (923, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 564, 565, 566, 567, 568, 1733,
            ],
            (928, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 569, 570, 571, 572, 573, 1744,
            ],
            (933, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 574, 575, 576, 577, 578, 1755,
            ],
            (938, 0x828f_2198) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 579, 580, 581, 582, 583, 1766,
            ],
            // ToDBLGrab655/666 (828f13a8): OnBoard, air/grab ancestry,
            // four empty Updates, then PlayAnimation at packed slot27.
            (655, 0x828f_13a8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1278, 1279, 1280, 1281, 1282,
            ],
            (666, 0x828f_13a8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1309, 1310, 1311, 1312, 1313,
            ],
            // LeftGround flip family (828f2138): Trick/ScoringTrick ancestry,
            // PlayAnimation at packed28, then the empty leaf Update at29.
            (805, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 457, 458, 459, 460, 461, 1489, 1490,
            ],
            (808, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 457, 458, 459, 460, 461, 1497, 1498,
            ],
            (810, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 462, 463, 464, 465, 466, 1500, 1501,
            ],
            (813, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 462, 463, 464, 465, 466, 1508, 1509,
            ],
            (815, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 467, 468, 469, 470, 471, 1511, 1512,
            ],
            (818, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 467, 468, 469, 470, 471, 1519, 1520,
            ],
            (820, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 472, 473, 474, 475, 476, 1522, 1523,
            ],
            (823, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 472, 473, 474, 475, 476, 1530, 1531,
            ],
            (825, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 477, 478, 479, 480, 481, 1533, 1534,
            ],
            (828, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 477, 478, 479, 480, 481, 1541, 1542,
            ],
            (830, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 482, 483, 484, 485, 486, 1544, 1545,
            ],
            (833, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 482, 483, 484, 485, 486, 1552, 1553,
            ],
            (835, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 487, 488, 489, 490, 491, 1555, 1556,
            ],
            (838, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 487, 488, 489, 490, 491, 1563, 1564,
            ],
            (840, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 492, 493, 494, 495, 496, 1566, 1567,
            ],
            (843, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 492, 493, 494, 495, 496, 1574, 1575,
            ],
            (845, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 497, 498, 499, 500, 501, 1577, 1578,
            ],
            (848, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 497, 498, 499, 500, 501, 1585, 1586,
            ],
            (850, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 502, 503, 504, 505, 506, 1588, 1589,
            ],
            (853, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 502, 503, 504, 505, 506, 1596, 1597,
            ],
            (855, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 507, 508, 509, 510, 511, 1599, 1600,
            ],
            (858, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 507, 508, 509, 510, 511, 1607, 1608,
            ],
            (860, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 512, 513, 514, 515, 516, 1610, 1611,
            ],
            (863, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 512, 513, 514, 515, 516, 1618, 1619,
            ],
            (884, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 524, 525, 526, 527, 528, 1646, 1647,
            ],
            (887, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 524, 525, 526, 527, 528, 1654, 1655,
            ],
            (889, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 529, 530, 531, 532, 533, 1657, 1658,
            ],
            (892, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 529, 530, 531, 532, 533, 1665, 1666,
            ],
            (894, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 534, 535, 536, 537, 538, 1668, 1669,
            ],
            (897, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 534, 535, 536, 537, 538, 1676, 1677,
            ],
            (899, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 539, 540, 541, 542, 543, 1679, 1680,
            ],
            (902, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 539, 540, 541, 542, 543, 1687, 1688,
            ],
            (904, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 544, 545, 546, 547, 548, 1690, 1691,
            ],
            (907, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 544, 545, 546, 547, 548, 1698, 1699,
            ],
            (909, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 549, 550, 551, 552, 553, 1701, 1702,
            ],
            (912, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 549, 550, 551, 552, 553, 1709, 1710,
            ],
            (914, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 554, 555, 556, 557, 558, 1712, 1713,
            ],
            (917, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 554, 555, 556, 557, 558, 1720, 1721,
            ],
            (919, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 559, 560, 561, 562, 563, 1723, 1724,
            ],
            (922, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 559, 560, 561, 562, 563, 1731, 1732,
            ],
            (924, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 564, 565, 566, 567, 568, 1734, 1735,
            ],
            (927, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 564, 565, 566, 567, 568, 1742, 1743,
            ],
            (929, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 569, 570, 571, 572, 573, 1745, 1746,
            ],
            (932, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 569, 570, 571, 572, 573, 1753, 1754,
            ],
            (934, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 574, 575, 576, 577, 578, 1756, 1757,
            ],
            (937, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 574, 575, 576, 577, 578, 1764, 1765,
            ],
            (939, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 579, 580, 581, 582, 583, 1767, 1768,
            ],
            (942, 0x828f_2138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 579, 580, 581, 582, 583, 1775, 1776,
            ],
            // Still752: OffBoard828eddc8 once, then Standing's attribute
            // at packed14 and Still's animation at packed15 (828f1e28).
            (752, 0x828f_1e28) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 755, 1430],
            // Source-verified ForceBrake and complete tail/nose anticipation family.
            // Each thunk expands ancestors exactly once; callbacks use actual payloads.
            (2, 0x828e_dec8) => &[0, 1, 2, 3, 14, 15, 16],
            (1029, 0x828f_2ed8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1933,
            ],
            (1184, 0x828f_3b70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1933, 2183,
            ],
            (1185, 0x828f_3be8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936,
            ],
            (1186, 0x828f_3be8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936,
            ],
            (1287, 0x828f_4818) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2369, 2370, 2371,
            ],
            (1288, 0x828f_4890) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2372,
            ],
            (1289, 0x828f_48e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2373, 2374,
            ],
            (1290, 0x828f_48e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2375, 2376,
            ],
            (1291, 0x828f_48e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2377, 2378,
            ],
            (1292, 0x828f_48e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2379, 2380,
            ],
            (1293, 0x828f_48e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2381, 2382,
            ],

            // Teleport5 calls Motion0's four root callbacks, the two reset
            // behaviours' shared BLR Updates, then PlayAnimation.Update.
            (319, 0x828f_0980) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 778, 779, 780, 781, 782, 783, 784,
            ],
            (5, 0x828e_e130) => &[0, 1, 2, 3, 36, 37, 38],
            // StateUpdate_DismountFromFakie_100 828ef3c8: OffBoard_1 for slots0..13,
            // then the direct SetTurning and PlayAnimation Updates of slots14/15.
            (100, 0x828e_f3c8) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 193, 194],
            // IdleDismount101 folds Motion0, OffBoard1, then its four leaf slots.
            // OBGround8 and Dismount32 have no behaviours of their own.
            (101, 0x828e_f428) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 195, 196, 197, 198,
            ],
            // StateUpdate_RunDismount_102 828ef4b0: OffBoard_1 for slots0..13,
            // the merged no-op (ForcePhysics), CreateAttribute x2, PlayAnimation,
            // then the merged no-op (Dismount).
            (102, 0x828e_f4b0) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 199, 200, 201, 202, 203,
            ],
            // StateUpdate_RunDismountWaitForLanding_103 828ef550: OffBoard_1,
            // no-op (ForcePhysics), CreateAttribute x3, no-op (Dismount).
            (103, 0x828e_f550) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 204, 205, 206, 207, 208,
            ],
            // StateUpdate_Dismount_36 828eeb00: OffBoard_1, then direct
            // AirDismounting and CreateAttribute Updates of slots14/15.
            (36, 0x828e_eb00) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 116, 117],
            // HippyRunout16 expands Motion0 and OffBoard1 before its five slots.
            (16, 0x828e_e3c8) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 58, 59, 60, 61, 62,
            ],
            // FSGrab Into647 expands OnBoard3/Air18/FSGrab244, then its
            // HandBusy, EndGesture, and PlayAnimation callbacks once each.
            (647, 0x828f_1510) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1258, 1259, 1260,
            ],
            // FSGrab Cyc648 folds the same ancestors, then its seven leaf
            // callbacks once each in the source thunk's order.
            (648, 0x828f_1448) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1261, 1262, 1263, 1264, 1265, 1266, 1267,
            ],
            // FSGrab Out654 reuses the OnBoard/InAir/FSGrab ancestor thunk,
            // then updates its own PlayAnimation once.
            (654, 0x828f_1750) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1277,
            ],
            // Land22 expands OnBoard3, then its eight slots20..27.
            (261, 0x828f_0068) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 77, 78,
                79, 80, 81, 82, 83, 84, 683,
            ],
            (262, 0x828f_0008) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 77, 78,
                79, 80, 81, 82, 83, 84, 684, 685,
            ],
            (263, 0x828f_0068) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 77, 78,
                79, 80, 81, 82, 83, 84, 686,
            ],
            // Both source thunks expand Motion0 -> WipeOut4 -> Air29 -> slot10.
            (98, 0x828e_f378) => &[0, 1, 2, 3, 33, 34, 35, 104, 105, 106, 191],
            (99, 0x828e_f378) => &[0, 1, 2, 3, 33, 34, 35, 104, 105, 106, 192],
            (6, 0x828e_e1a8) => &[0, 1, 2, 3, 39, 40],
            // RestAndRecover30 expands Motion0/WipeOut4 then its one slot7.
            (30, 0x828e_e960) => &[0, 1, 2, 3, 33, 34, 35, 107],
            // IDA reviewed: Riding277 + Sliding716, Into/Out1251..1254,
            // then the authored leaf slots; no repeated ancestor updates.
            (716, 0x828f_2bb8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406,
            ],
            (1019, 0x828f_2bb8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406,
            ],
            (1020, 0x828f_2bb8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406,
            ],
            (1167, 0x828f_2bb8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406,
            ],
            (1168, 0x828f_2bb8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406,
            ],
            (1251, 0x828f_3fa8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2265, 2266, 2267,
            ],
            (1252, 0x828f_4048) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2268,
            ],
            (1253, 0x828f_3fa8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2269, 2270, 2271,
            ],
            (1254, 0x828f_4048) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2272,
            ],
            (1308, 0x828f_4a08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2265, 2266, 2267, 2399,
            ],
            (1309, 0x828f_4a08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2265, 2266, 2267, 2400,
            ],
            (1310, 0x828f_4ab8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2268, 2401,
            ],
            (1311, 0x828f_4ab8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2268, 2402,
            ],
            (1312, 0x828f_4a58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2268, 2403, 2404,
            ],
            (1313, 0x828f_4a08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2269, 2270, 2271, 2405,
            ],
            (1314, 0x828f_4a08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2269, 2270, 2271, 2406,
            ],
            (1315, 0x828f_4ab8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2272, 2407,
            ],
            (1316, 0x828f_4ab8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2272, 2408,
            ],
            (1317, 0x828f_4a58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1405, 1406, 2272, 2409, 2410,
            ],
            (753, 0x828f_1e88) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 755, 1431, 1432, 1433,
            ],
            (754, 0x828f_1e88) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 755, 1434, 1435, 1436,
            ],
            (1170, 0x828f_39b8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1923, 1924, 1925, 1926, 1927, 2172,
            ],
            // Anticipation1030 expands Riding277, Turning719, then its three
            // CreateAttribute/FilterMotionGraphIntent/IsAnticipating slots.
            (1030, 0x828f_2f38) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936,
            ],
            // AnticTail1286 expands Anticipation1030, then its PlayAnimation
            // and SetDistComToBoard slots. The latter Update is a BLR.
            (1286, 0x828f_47b8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1934, 1935, 1936, 2367, 2368,
            ],
            // AnticOut/AnticTail1183 expands Riding277, then Turning719,
            // IsAnticipating's BLR Update and the tail PlayAnimation.
            (1183, 0x828f_3b70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1933, 2182,
            ],
            // Nollie/Takeoff/FromAntic868 invokes the shared Ollie188 update
            // for slots0..27, then PlayAnimation and SetTrickHeight.
            (868, 0x828f_20d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 517, 518, 519, 520, 521, 1629, 1630,
            ],
            // Ollie/Takeoff/FromAntic789 shares the folded Ollie188 thunk,
            // then updates PlayAnimation1472 and SetTrickHeight1473 (BLR).
            (789, 0x828f_20d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 450, 451, 452, 453, 454, 1472, 1473,
            ],
            // The GrindOutAssist leaves Ollie LeftGround786 and Nollie LeftGround865 use the same
            // thunk: Ollie_188 for slots0..27, PlayAnimation (+0xE0), then the merged no-op
            // (SetTrickHeight, +0xE8). Neither GrindOutAssist514/559 adds a slot.
            (786, 0x828f_20d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 450, 451, 452, 453, 454, 1464, 1465,
            ],
            (865, 0x828f_20d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 517, 518, 519, 520, 521, 1621, 1622,
            ],
            // StateUpdate_LeftGround_1065 828f3110 serves the IsProSkater kickflips' leaves (Carroll
            // LeftGround1065/FromAntic1068, Gonzales LeftGround1070/FromAntic1073): the folded
            // TrickInto_509 update for slots0..26, PlayAnimation (+0xD8), then the merged no-op
            // (SetTrickHeight, +0xE0).
            (1065, 0x828f_3110) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1105, 1106, 1107, 1989, 1990,
            ],
            (1068, 0x828f_3110) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1105, 1106, 1107, 1997, 1998,
            ],
            (1070, 0x828f_3110) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1115, 1116, 1117, 2000, 2001,
            ],
            (1073, 0x828f_3110) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1115, 1116, 1117, 2008, 2009,
            ],
            // StateUpdate_FromGrind_788 828f1f68 serves Ollie FromManual787/
            // FromGrind788 and Nollie FromManual866/FromGrind867: Ollie_188
            // 828efa48 for slots0..27, then PlayAnimation, the merged no-op
            // (SetTrickHeight) and JumpInto.
            (787, 0x828f_1f68) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 450, 451, 452, 453, 454, 1466, 1467, 1468,
            ],
            (788, 0x828f_1f68) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 450, 451, 452, 453, 454, 1469, 1470, 1471,
            ],
            (866, 0x828f_1f68) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 517, 518, 519, 520, 521, 1623, 1624, 1625,
            ],
            (867, 0x828f_1f68) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 517, 518, 519, 520, 521, 1626, 1627, 1628,
            ],
            // Ollie/LeftGround516 calls the shared Ollie188 thunk once,
            // then updates its PlayAnimation1125 leaf.
            (516, 0x828f_0d30) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 450, 451, 452, 453, 454, 1125,
            ],
            // The native folded thunk also serves the corresponding Nollie
            // and direct-takeoff states. Same slots, actual authored instances.
            (561, 0x828f_0d30) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 517, 518, 519, 520, 521, 1144,
            ],
            (785, 0x828f_0d30) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 450, 451, 452, 453, 454, 1463,
            ],
            (864, 0x828f_0d30) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 517, 518, 519, 520, 521, 1620,
            ],
            // Push1028 expands Riding277 and Turning719, then the score
            // augmentation and InitPush slots. InitPush::Update is a BLR.
            // RollingForwardsOrStationary1328: Manual, TailManual, Cycle,
            // then PlayAnimation. Source828f4c18 expands all36 slots once.
            // Into1333: Holding1273 (Manual + score + two native no-ops),
            // then AttachIntent and PlayAnimation at packed slots35/36.
            // Nose-manual Cycle descendants reached by Pre251/252.
            (1350, 0x828f_5038) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2345, 2346, 2431, 2432,
                2463, 2464,
            ],
            (1351, 0x828f_50c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2345, 2346, 2431, 2432,
                2465,
            ],
            // RollingBackwards828f4bb8: Cycle35 slots, SetManualAngle, animation.
            (1327, 0x828f_4bb8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2176, 2340, 2341, 2417, 2418,
            ],
            // IDA828f2450/4c68/45c8 and828eeeb8: verified parent expansion
            // plus leaf callbacks, including folded aliases and native no-op slots.
            (84, 0x828e_eeb8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 85, 86,
                87, 88, 89, 162, 163, 164, 165, 166, 167, 168,
            ],
            (958, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1268, 1269, 1270, 1271, 1825,
            ],
            (959, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1268, 1269, 1270, 1271, 1826,
            ],
            (961, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1268, 1269, 1270, 1271, 1827,
            ],
            (970, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1299, 1300, 1301, 1302, 1840,
            ],
            (971, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1299, 1300, 1301, 1302, 1841,
            ],
            (973, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1299, 1300, 1301, 1302, 1842,
            ],
            (1106, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1268, 1269, 1270, 1271, 2078,
            ],
            (1107, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1268, 1269, 1270, 1271, 2079,
            ],
            (1124, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1299, 1300, 1301, 1302, 2104,
            ],
            (1125, 0x828f_2450) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1299, 1300, 1301, 1302, 2105,
            ],
            (1276, 0x828f_45c8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2178, 2179, 2349,
            ],
            (1277, 0x828f_45c8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2178, 2179, 2350,
            ],
            (1329, 0x828f_4c68) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2176, 2342, 2343, 2420, 2421,
            ],
            // The eight Idle steps share 1.02's 827523c0: OffBoard's fourteen
            // slots, CreateAttribute, PlayAnimation (1.02 drops SteppingBoardNoBoard).
            // Idle105 and its intervening parents add no behaviour slots.
            (309, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2469, 2470],
            (310, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2471, 2472],
            (311, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2473, 2474],
            (312, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2475, 2476],
            (313, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2477, 2478],
            (314, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2479, 2480],
            (315, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2481, 2482],
            (316, 0x8275_23c0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 2483, 2484],
            // IDA828f0770 /828f4e38: inherited update then all packed leaf slots.
            (1354, 0x828f_0770) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 216, 217, 218, 219, 764, 765,
            ],
            (1357, 0x828f_0770) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 216, 217, 218, 219, 773, 774,
            ],
            // StateUpdate_StandingJump_106 828ef6b8 (OffBoard_1, CreateAttribute,
            // three AttachIntent), then each leaf's own slots:
            // Takeoff1355 828f07d0: PlayAnimation and two CreateAttribute.
            (1355, 0x828f_07d0) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 216, 217, 218, 219, 766, 767, 768,
            ],
            // Descent1356 828f0848: PlayAnimation, CreateAttribute, LandingCOG's
            // BLR Update 825187c0, StandingJumpDescent 82ca6aa0.
            (1356, 0x828f_0848) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 216, 217, 218, 219, 769, 770, 771,
                772,
            ],
            // JumpingIntoMount1358 828f08d0: AttachIntent, then PlayAnimation.
            (1358, 0x828f_08d0) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 216, 217, 218, 219, 775, 776,
            ],
            // StateUpdate_Default_95 828ef2d8 (also Default96): folded
            // ToLandingOnBoardFromRight_27 828ee7e8 = OnBoard_3, CreateAttribute,
            // ControlAirLegExtension, ApplyingBodyTilt's BLR; then PlayAnimation.
            (95, 0x828e_f2d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 94, 95,
                96, 188,
            ],
            (96, 0x828e_f2d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 97, 98,
                99, 189,
            ],
            // StateUpdate_Default_97 828ef328: LandingOnBoard_28 828ee860 =
            // OnBoard_3, CreateAttribute, LandOnBoard's BLR, ControlAirLegExtension,
            // ApplyingBodyTilt's BLR; then PlayAnimation.
            (97, 0x828e_f328) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 100,
                101, 102, 103, 190,
            ],
            // Out644 828f11d0 = folded StateUpdate_ToOffBoard_646: Lateflip_80
            // (OnBoard_3, ForcePhysics BLR, AttachIntent, CreateAttribute), then
            // ScoringGrabs and PlayAnimation.
            (644, 0x828f_11d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1252,
            ],
            // DBLGrab ToOffBoard646 owns 828f11d0: packed23 ScoringGrabs, packed24 PlayAnimation.
            (646, 0x828f_11d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1257,
            ],
            // FromDBLGrab656/667 828f1510: Lateflip_80, HandBusy and EndGesture
            // BLRs, then PlayAnimation.
            (656, 0x828f_1510) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1283, 1284, 1285,
            ],
            (667, 0x828f_1510) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1314, 1315, 1316,
            ],
            // DBLGrab Into642 828f1070: expanded native slot classes verified per alias.
            (642, 0x828f_1070) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1242, 1243, 1244, 1245, 1246,
            ],
            // DBLGrab Cyc643 828f1120 (TweakX/TweakY filters 1249/1250): slot classes verified.
            (643, 0x828f_1120) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1247, 1248, 1249, 1250, 1251,
            ],
            // DBLGrab SupermanInto953 828f2288: expanded native slot classes verified per alias.
            (953, 0x828f_2288) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1253, 1254, 1255, 1256, 1807, 1808, 1809, 1810, 1811, 1812, 1813,
            ],
            // DBLGrab SupermanCyc954 828f2350: expanded native slot classes verified per alias.
            (954, 0x828f_2350) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1253, 1254, 1255, 1256, 1814, 1815, 1816, 1817, 1818,
            ],
            // DBLGrab SupermanOut955/ToFSChrist956/ToBSChrist957 828f23f0: slot classes verified.
            (955, 0x828f_23f0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1253, 1254, 1255, 1256, 1819, 1820,
            ],
            (956, 0x828f_23f0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1253, 1254, 1255, 1256, 1821, 1822,
            ],
            (957, 0x828f_23f0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 641, 1253, 1254, 1255, 1256, 1823, 1824,
            ],
            // MuteGrab Into669 / StaleGrab Into676 share 828f1510 (Lateflip_80, HandBusy and
            // EndGesture BLRs, PlayAnimation).
            (669, 0x828f_1510) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1320, 1321, 1322,
            ],
            (676, 0x828f_1510) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1336, 1337, 1338,
            ],
            // FingerFlip/FingerFlipVar GrabOut 828f2568 (StateUpdate_GrabOut_969): Lateflip_80,
            // then ScoringTrick, PlayAnimation, SetDistComToBoard's BLR and FingerFlipOut.
            (967, 0x828f_2568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1273, 1833, 1834, 1835,
            ],
            (969, 0x828f_2568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1276, 1837, 1838, 1839,
            ],
            (979, 0x828f_2568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1304, 1848, 1849, 1850,
            ],
            (981, 0x828f_2568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1307, 1852, 1853, 1854,
            ],
            (983, 0x828f_2568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1328, 1856, 1857, 1858,
            ],
            (985, 0x828f_2568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1344, 1860, 1861, 1862,
            ],
            // Up/Down FingerFlip GrabOut 828f3e20 (StateUpdate_GrabOut_1246): Grabbing_995/989,
            // then the same four slots.
            (1235, 0x828f_3e20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2135, 2239, 2240, 2241,
            ],
            (1246, 0x828f_3e20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2144, 2255, 2256, 2257,
            ],
            // OneFootOllie81 Into/Cyc/Out 828f27d8 (folded Out_1005): OneFootOllie_81 (OnBoard_3,
            // ForcePhysics BLR, AttachIntent, ControlAirLegExtension), ScoringTrick, IsDoingTrick
            // BLR, PlayAnimation.
            (997, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1361, 1362, 1881,
            ],
            (998, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1361, 1362, 1882,
            ],
            (999, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1361, 1362, 1883,
            ],
            (1000, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1363, 1364, 1884,
            ],
            (1001, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1363, 1364, 1885,
            ],
            (1002, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1363, 1364, 1886,
            ],
            (1003, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1365, 1366, 1887,
            ],
            (1004, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1365, 1366, 1888,
            ],
            (1005, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1365, 1366, 1889,
            ],
            (1006, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1367, 1368, 1890,
            ],
            (1007, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1367, 1368, 1891,
            ],
            (1008, 0x828f_27d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                161, 1367, 1368, 1892,
            ],
            // Tail/nose grab Into1142/1148 828f3420 (folded Into_1148): Grabbing_995 (Up_250/Down_251
            // filter pair, CreateAttribute, HandBusy/EndGesture BLRs), PlayAnimation.
            (1142, 0x828f_3420) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2130,
            ],
            (1148, 0x828f_3420) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2138,
            ],
            // Tail grab Cycle1143 828f3470: expanded native slot classes verified per alias.
            (1143, 0x828f_3470) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2131, 2132, 2133,
            ],
            // Nose grab Cycle1149 828f3540: expanded native slot classes verified per alias.
            (1149, 0x828f_3540) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2139, 2140, 2141, 2142,
            ],
            // Tail/nose grab Out990/996 828f2778: expanded native slot classes verified per alias.
            (990, 0x828f_2778) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1871,
            ],
            (996, 0x828f_2778) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1880,
            ],
            // Down Into992/Out994 828f26c8: expanded native slot classes verified per alias.
            (992, 0x828f_26c8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1353, 1354, 1355, 1356, 1357, 1874,
            ],
            (994, 0x828f_26c8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1353, 1354, 1355, 1356, 1357, 1877,
            ],
            // Down Cycle993 828f2718: expanded native slot classes verified per alias.
            (993, 0x828f_2718) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1353, 1354, 1355, 1356, 1357, 1875, 1876,
            ],
            // Mute/Stale grab Cyc670/677 828f1600: expanded native slot classes verified per alias.
            (670, 0x828f_1600) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1323, 1324, 1325, 1326, 1327,
            ],
            (677, 0x828f_1600) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1339, 1340, 1341, 1342, 1343,
            ],
            // Mute/Stale grab Out674/681 828f16f0: expanded native slot classes verified per alias.
            (674, 0x828f_16f0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1333, 1334,
            ],
            (681, 0x828f_16f0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1349, 1350,
            ],
            // FS/BS grab variant Into/Cyc 828f3310 (16 aliases): expanded native slot classes verified.
            (1108, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1828, 2080, 2081,
            ],
            (1109, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1828, 2082, 2083,
            ],
            (1112, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1829, 2086, 2087,
            ],
            (1113, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1829, 2088, 2089,
            ],
            (1116, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1830, 2092, 2093,
            ],
            (1117, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1830, 2094, 2095,
            ],
            (1120, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1831, 2098, 2099,
            ],
            (1121, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1831, 2100, 2101,
            ],
            (1126, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1843, 2106, 2107,
            ],
            (1127, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1843, 2108, 2109,
            ],
            (1130, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1844, 2112, 2113,
            ],
            (1131, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1844, 2114, 2115,
            ],
            (1134, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1845, 2118, 2119,
            ],
            (1135, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1845, 2120, 2121,
            ],
            (1138, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1846, 2124, 2125,
            ],
            (1139, 0x828f_3310) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1846, 2126, 2127,
            ],
            // FS/BS grab variant EarlyOut/FullOut 828f3c58 (16 aliases): slot classes verified.
            (1189, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1828, 2185,
            ],
            (1190, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1828, 2186,
            ],
            (1194, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1829, 2191,
            ],
            (1195, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1829, 2192,
            ],
            (1199, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1830, 2197,
            ],
            (1200, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1830, 2198,
            ],
            (1204, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1831, 2203,
            ],
            (1205, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1831, 2204,
            ],
            (1209, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1843, 2209,
            ],
            (1210, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1843, 2210,
            ],
            (1214, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1844, 2215,
            ],
            (1215, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1844, 2216,
            ],
            (1219, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1845, 2221,
            ],
            (1220, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1845, 2222,
            ],
            (1224, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1846, 2227,
            ],
            (1225, 0x828f_3c58) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1846, 2228,
            ],
            // FS/BS grab variant Into/JumpOut 828f3cd0 (16 aliases): slot classes verified.
            (1191, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1828, 2084, 2085, 2187,
            ],
            (1193, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1828, 2084, 2085, 2190,
            ],
            (1196, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1829, 2090, 2091, 2193,
            ],
            (1198, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1829, 2090, 2091, 2196,
            ],
            (1201, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1830, 2096, 2097, 2199,
            ],
            (1203, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1830, 2096, 2097, 2202,
            ],
            (1206, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1831, 2102, 2103, 2205,
            ],
            (1208, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1831, 2102, 2103, 2208,
            ],
            (1211, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1843, 2110, 2111, 2211,
            ],
            (1213, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1843, 2110, 2111, 2214,
            ],
            (1216, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1844, 2116, 2117, 2217,
            ],
            (1218, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1844, 2116, 2117, 2220,
            ],
            (1221, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1845, 2122, 2123, 2223,
            ],
            (1223, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1845, 2122, 2123, 2226,
            ],
            (1226, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1846, 2128, 2129, 2229,
            ],
            (1228, 0x828f_3cd0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1846, 2128, 2129, 2232,
            ],
            // FS/BS grab footplant Absorb 828f3bf8 (8 aliases): expanded native slot classes
            // verified; ForcePhysics/HandBusy updates are BLRs, FootPlantAbsorb -> 82cb4f18.
            (1192, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1828, 2084, 2085, 2188, 2189,
            ],
            (1197, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1829, 2090, 2091, 2194, 2195,
            ],
            (1202, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1830, 2096, 2097, 2200, 2201,
            ],
            (1207, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1272, 1831, 2102, 2103, 2206, 2207,
            ],
            (1212, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1843, 2110, 2111, 2212, 2213,
            ],
            (1217, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1844, 2116, 2117, 2218, 2219,
            ],
            (1222, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1845, 2122, 2123, 2224, 2225,
            ],
            (1227, 0x828f_3bf8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1303, 1846, 2128, 2129, 2230, 2231,
            ],
            // Up/Down grab Into/Cyc/Out and EarlyOut/FullOut 828f4fd8 (14 aliases): slot classes verified.
            (1229, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2134, 2233,
            ],
            (1230, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2134, 2234,
            ],
            (1231, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2134, 2235,
            ],
            (1240, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2143, 2249,
            ],
            (1241, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2143, 2250,
            ],
            (1242, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2143, 2251,
            ],
            (1342, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2236, 2455,
            ],
            (1343, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2236, 2456,
            ],
            (1344, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2237, 2457,
            ],
            (1345, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2237, 2458,
            ],
            (1346, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2252, 2459,
            ],
            (1347, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2252, 2460,
            ],
            (1348, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2253, 2461,
            ],
            (1349, 0x828f_4fd8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2253, 2462,
            ],
            // Up/Down grab Into/Cyc 828f4990 (8 aliases): slot classes verified.
            (1296, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2236, 2383, 2384,
            ],
            (1297, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2236, 2385, 2386,
            ],
            (1299, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2237, 2387, 2388,
            ],
            (1300, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2237, 2389, 2390,
            ],
            (1302, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2252, 2391, 2392,
            ],
            (1303, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2252, 2393, 2394,
            ],
            (1305, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2253, 2395, 2396,
            ],
            (1306, 0x828f_4990) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2253, 2397, 2398,
            ],
            // Up/Down Into1234/1245 828f3d20: expanded native slot classes verified per alias.
            (1234, 0x828f_3d20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2135, 2238,
            ],
            (1245, 0x828f_3d20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2144, 2254,
            ],
            // JumpingIntoMountLeft9/Right10 828ee210 (folded Right_10): OffBoard_1,
            // CreateAttribute, AttachIntent, PlayAnimation.
            (9, 0x828e_e210) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 41, 42, 43],
            (10, 0x828e_e210) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 44, 45, 46],
            // IntoBSGrabOut11 828ee288: OffBoard_1, PlayAnimation.
            (11, 0x828e_e288) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 47],
            // Running/Sprinting mounts 828ef740 (folded Sprinting..Right_109):
            // OffBoard_1, three CreateAttribute, PlayAnimation.
            (108, 0x828e_f740) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 220, 221, 222, 223,
            ],
            (109, 0x828e_f740) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 224, 225, 226, 227,
            ],
            (110, 0x828e_f740) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 228, 229, 230, 231,
            ],
            (111, 0x828e_f740) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 232, 233, 234, 235,
            ],
            // Stall mounts 828ef7c8 (folded StallIntoMount_114): OffBoard_1,
            // AttachIntent, two CreateAttribute.
            (112, 0x828e_f7c8) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 236, 237, 238],
            (113, 0x828e_f7c8) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 239, 240, 241],
            (114, 0x828e_f7c8) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 242, 243, 244],
            // IntoBSGrabOut1359 828f0930: PlayAnimation.
            (1359, 0x828f_0930) => &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 216, 217, 218, 219, 777,
            ],
            (1331, 0x828f_4e38) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2176, 2344, 2423, 2424, 2425,
            ],
            (1332, 0x828f_4e38) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2176, 2344, 2426, 2427, 2428,
            ],
            (1336, 0x828f_4e38) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2348, 2435, 2436, 2437,
            ],
            (1337, 0x828f_4e38) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2348, 2438, 2439, 2440,
            ],
            (1333, 0x828f_4d18) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2345, 2346, 2429, 2430,
            ],
            (1328, 0x828f_4c18) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2176, 2340, 2341, 2419,
            ],
            (1028, 0x828f_2e60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932,
            ],
            // PushContact Right/Left1280/1281 call Push1028 first, then
            // SetPushCoefs, ComputeTargetCoefs, ComputeRepushDeadline's empty
            // Update, and PlayAnimation (0x828f4664..0x828f46b4).
            (1280, 0x828f_4648) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2180, 2351, 2352, 2353,
            ],
            (1281, 0x828f_4648) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2180, 2354, 2355, 2356,
            ],
            // PushCycle Right/Left1282/1283 call Push1028, then the shared
            // SetPushCoefs parent slot and three leaf callbacks in source order.
            (1282, 0x828f_46d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2181, 2357, 2358, 2359,
            ],
            (1283, 0x828f_46d0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2181, 2360, 2361, 2362,
            ],
            // PushEnd Right/Left1284/1285 call Push1028, then PlayAnimation
            // and PushOut's empty Update (folded thunk 0x828f4758).
            // IDA 828f4508/2058/2ca0: exact authored ancestor-expanded slots.
            (806, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 457, 458, 459, 460, 461, 1491, 1492, 1493,
            ],
            (807, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 457, 458, 459, 460, 461, 1494, 1495, 1496,
            ],
            (811, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 462, 463, 464, 465, 466, 1502, 1503, 1504,
            ],
            (812, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 462, 463, 464, 465, 466, 1505, 1506, 1507,
            ],
            (816, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 467, 468, 469, 470, 471, 1513, 1514, 1515,
            ],
            (817, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 467, 468, 469, 470, 471, 1516, 1517, 1518,
            ],
            (821, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 472, 473, 474, 475, 476, 1524, 1525, 1526,
            ],
            (822, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 472, 473, 474, 475, 476, 1527, 1528, 1529,
            ],
            (826, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 477, 478, 479, 480, 481, 1535, 1536, 1537,
            ],
            (827, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 477, 478, 479, 480, 481, 1538, 1539, 1540,
            ],
            (831, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 482, 483, 484, 485, 486, 1546, 1547, 1548,
            ],
            (832, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 482, 483, 484, 485, 486, 1549, 1550, 1551,
            ],
            (836, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 487, 488, 489, 490, 491, 1557, 1558, 1559,
            ],
            (837, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 487, 488, 489, 490, 491, 1560, 1561, 1562,
            ],
            (841, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 492, 493, 494, 495, 496, 1568, 1569, 1570,
            ],
            (842, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 492, 493, 494, 495, 496, 1571, 1572, 1573,
            ],
            (846, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 497, 498, 499, 500, 501, 1579, 1580, 1581,
            ],
            (847, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 497, 498, 499, 500, 501, 1582, 1583, 1584,
            ],
            (851, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 502, 503, 504, 505, 506, 1590, 1591, 1592,
            ],
            (852, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 502, 503, 504, 505, 506, 1593, 1594, 1595,
            ],
            (856, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 507, 508, 509, 510, 511, 1601, 1602, 1603,
            ],
            (857, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 507, 508, 509, 510, 511, 1604, 1605, 1606,
            ],
            (861, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 512, 513, 514, 515, 516, 1612, 1613, 1614,
            ],
            (862, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 512, 513, 514, 515, 516, 1615, 1616, 1617,
            ],
            (885, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 524, 525, 526, 527, 528, 1648, 1649, 1650,
            ],
            (886, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 524, 525, 526, 527, 528, 1651, 1652, 1653,
            ],
            (890, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 529, 530, 531, 532, 533, 1659, 1660, 1661,
            ],
            (891, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 529, 530, 531, 532, 533, 1662, 1663, 1664,
            ],
            (895, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 534, 535, 536, 537, 538, 1670, 1671, 1672,
            ],
            (896, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 534, 535, 536, 537, 538, 1673, 1674, 1675,
            ],
            (900, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 539, 540, 541, 542, 543, 1681, 1682, 1683,
            ],
            (901, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 539, 540, 541, 542, 543, 1684, 1685, 1686,
            ],
            (905, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 544, 545, 546, 547, 548, 1692, 1693, 1694,
            ],
            (906, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 544, 545, 546, 547, 548, 1695, 1696, 1697,
            ],
            (910, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 549, 550, 551, 552, 553, 1703, 1704, 1705,
            ],
            (911, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 549, 550, 551, 552, 553, 1706, 1707, 1708,
            ],
            (915, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 554, 555, 556, 557, 558, 1714, 1715, 1716,
            ],
            (916, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 554, 555, 556, 557, 558, 1717, 1718, 1719,
            ],
            (920, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 559, 560, 561, 562, 563, 1725, 1726, 1727,
            ],
            (921, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 559, 560, 561, 562, 563, 1728, 1729, 1730,
            ],
            (925, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 564, 565, 566, 567, 568, 1736, 1737, 1738,
            ],
            (926, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 564, 565, 566, 567, 568, 1739, 1740, 1741,
            ],
            (930, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 569, 570, 571, 572, 573, 1747, 1748, 1749,
            ],
            (931, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 569, 570, 571, 572, 573, 1750, 1751, 1752,
            ],
            (935, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 574, 575, 576, 577, 578, 1758, 1759, 1760,
            ],
            (936, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 574, 575, 576, 577, 578, 1761, 1762, 1763,
            ],
            (940, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 579, 580, 581, 582, 583, 1769, 1770, 1771,
            ],
            (941, 0x828f_2058) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 579, 580, 581, 582, 583, 1772, 1773, 1774,
            ],
            (1023, 0x828f_2ca0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1412, 1917, 1918, 1919,
            ],
            (1024, 0x828f_2ca0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1412, 1920, 1921, 1922,
            ],
            (1274, 0x828f_4508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2347,
            ],
            // IDA fastplant and grab-out thunks: exact authored ancestors.
            (605, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 584, 585, 586, 587, 588, 589, 590, 1165, 1166, 1167,
            ],
            (606, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 584, 585, 586, 587, 588, 589, 590, 1168, 1169, 1170,
            ],
            (607, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 584, 585, 586, 587, 588, 589, 590, 1171, 1172, 1173,
            ],
            (609, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 591, 592, 593, 594, 595, 596, 597, 1176, 1177, 1178,
            ],
            (610, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 591, 592, 593, 594, 595, 596, 597, 1179, 1180, 1181,
            ],
            (611, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 591, 592, 593, 594, 595, 596, 597, 1182, 1183, 1184,
            ],
            (613, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 598, 599, 600, 601, 602, 603, 604, 1187, 1188, 1189,
            ],
            (614, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 598, 599, 600, 601, 602, 603, 604, 1190, 1191, 1192,
            ],
            (615, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 598, 599, 600, 601, 602, 603, 604, 1193, 1194, 1195,
            ],
            (617, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 605, 606, 607, 608, 609, 610, 611, 1198, 1199, 1200,
            ],
            (618, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 605, 606, 607, 608, 609, 610, 611, 1201, 1202, 1203,
            ],
            (619, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 605, 606, 607, 608, 609, 610, 611, 1204, 1205, 1206,
            ],
            (621, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 612, 613, 614, 615, 616, 617, 618, 1209, 1210, 1211,
            ],
            (622, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 612, 613, 614, 615, 616, 617, 618, 1212, 1213, 1214,
            ],
            (623, 0x828f_0e70) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 612, 613, 614, 615, 616, 617, 618, 1215, 1216, 1217,
            ],
            // StateUpdate_Right_948 828f21e8 serves every PlantingFoot Left/Right leaf:
            // FSFastPlant_220 for packed0..29, PlayAnimation, the SetTrickHeight and
            // InitPush BLRs, then ComputeTargetCoefsFromSpeedAndStrength and SetPushCoefs.
            (943, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 584, 585, 586, 587, 588, 589, 590, 1163, 1164, 1777, 1778, 1779,
            ],
            (944, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 584, 585, 586, 587, 588, 589, 590, 1163, 1164, 1780, 1781, 1782,
            ],
            (945, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 591, 592, 593, 594, 595, 596, 597, 1174, 1175, 1783, 1784, 1785,
            ],
            (946, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 591, 592, 593, 594, 595, 596, 597, 1174, 1175, 1786, 1787, 1788,
            ],
            (947, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 598, 599, 600, 601, 602, 603, 604, 1185, 1186, 1789, 1790, 1791,
            ],
            (948, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 598, 599, 600, 601, 602, 603, 604, 1185, 1186, 1792, 1793, 1794,
            ],
            (949, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 605, 606, 607, 608, 609, 610, 611, 1196, 1197, 1795, 1796, 1797,
            ],
            (950, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 605, 606, 607, 608, 609, 610, 611, 1196, 1197, 1798, 1799, 1800,
            ],
            (951, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 612, 613, 614, 615, 616, 617, 618, 1207, 1208, 1801, 1802, 1803,
            ],
            (952, 0x828f_21e8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 612, 613, 614, 615, 616, 617, 618, 1207, 1208, 1804, 1805, 1806,
            ],
            (1236, 0x828f_3ea8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2136, 2137, 2242,
            ],
            (1238, 0x828f_3ea8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2136, 2137, 2246,
            ],
            (1247, 0x828f_3ea8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2145, 2146, 2258,
            ],
            (1249, 0x828f_3ea8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2145, 2146, 2262,
            ],
            // StateUpdate_SeatbeltFingerFlipShuv_1239 828f3f20 (Crail1250 aliases
            // it): Grabbing_995 for packed0..26, merged no-op Updates of packed27/28
            // (HandBusy/EndGesture), ScoringTrick packed29, PlayAnimation packed30.
            (1239, 0x828f_3f20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2136, 2137, 2247, 2248,
            ],
            (1250, 0x828f_3f20) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2145, 2146, 2263, 2264,
            ],
            (1352, 0x828f_5138) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2345, 2346, 2433, 2434,
                2466, 2467,
            ],
            //828eea28: OffBoard1 followed by Mount34 PlayAnimation/CreateAttribute.
            (34, 0x828e_ea28) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 111, 112],
            (652, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1274, 1275,
            ],
            (663, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1305, 1306,
            ],
            (672, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1329, 1330,
            ],
            (673, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1331, 1332,
            ],
            (679, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1345, 1346,
            ],
            (680, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1347, 1348,
            ],
            (966, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1273, 1832,
            ],
            (968, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1276, 1836,
            ],
            (978, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1304, 1847,
            ],
            (980, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1307, 1851,
            ],
            (982, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1328, 1855,
            ],
            (984, 0x828f_2508) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1344, 1859,
            ],
            (1237, 0x828f_3d80) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                646, 647, 1352, 1869, 1870, 2136, 2137, 2243, 2244, 2245,
            ],
            (1248, 0x828f_3d80) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1358, 1878, 1879, 2145, 2146, 2259, 2260, 2261,
            ],
            (1339, 0x828f_4f60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2445, 2446, 2447,
            ],
            (1341, 0x828f_4f60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2452, 2453, 2454,
            ],
            (85, 0x828e_ef80) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 85, 86,
                87, 88, 89, 169, 170, 171, 172, 173,
            ],
            (1353, 0x828f_51c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2177, 2345, 2346, 2433, 2434,
                2468,
            ],
            (1284, 0x828f_4758) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2363, 2364,
            ],
            (1285, 0x828f_4758) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2365, 2366,
            ],
            // Right/Left PushStart Default share the folded StateUpdate thunk.
            // Push1028 runs first, followed by the four leaf callbacks in order.
            (1338, 0x828f_4ed8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2441, 2442, 2443, 2444,
            ],
            (1340, 0x828f_4ed8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1931, 1932, 2448, 2449, 2450, 2451,
            ],
            // Kickflip FromAntic1078 expands its OnGround and Trick/In parents,
            // then the leaf PlayAnimation and SetTrickHeight slots.
            (1078, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1126, 1127, 1128, 2019, 2020,
            ],
            // Sequence/Default share Air252 parent then PlayAnimation slot25.
            (690, 0x828f_1898) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                650, 651, 652, 1359,
            ],
            (691, 0x828f_1898) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                650, 651, 652, 1360,
            ],
            // Down Air991 thunk828f2668: OnBoard3, InAir21, Down251,
            // then PlayAnimation and ControlAirLegExtension at slots24..25.
            (991, 0x828f_2668) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                648, 649, 1872, 1873,
            ],
            // Bumped276 thunk828f0418: OnGround parent, then six slots23..28.
            (276, 0x828f_0418) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 718, 719, 720, 721,
            ],
            // Shared Takeoff/LeftGround thunk: Motion0 -> OnBoard3 -> Trick19
            // -> In565 -> the leaf PlayAnimation, exactly once per slot.
            (792, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1126, 1127, 1128, 1474,
            ],
            // Held Kickflip Cyc1 reuses the Out1 folded thunk: Trick19,
            // Kickflip189's attribute, then cycle scoring and animation.
            (793, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1475, 1476,
            ],
            (794, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1477, 1478,
            ],
            (795, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1479, 1480,
            ],
            // Out1/2/3 have the same folded thunk but distinct scoring and
            // animation occurrences; do not identify a cycle by its address.
            (1079, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 2021, 2022,
            ],
            (1080, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 2023, 2024,
            ],
            (1081, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 2025, 2026,
            ],
            (799, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1129, 1130, 1131, 1481,
            ],
            (871, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1145, 1146, 1147, 1631,
            ],
            (878, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1148, 1149, 1150, 1638,
            ],
            (1074, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1126, 1127, 1128, 2010,
            ],
            (1082, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1129, 1130, 1131, 2027,
            ],
            (1090, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1145, 1146, 1147, 2044,
            ],
            (1098, 0x828f_3170) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1148, 1149, 1150, 2061,
            ],
            // FromMoving/Riding expands OnGround24 once before AttachIntent
            // and the leaf PlayAnimation update.
            (1160, 0x828f_3958) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1402, 2163,
            ],
            // Brake/Moving/Cyc1161 updates Riding277, AttachIntent, then
            // PlayAnimation and SetSpeed in its two leaf slots.
            (1161, 0x828f_38e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1402, 2164, 2165,
            ],
            (1162, 0x828f_3958) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1402, 2166,
            ],
            (1163, 0x828f_3958) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1402, 2167,
            ],
            (1164, 0x828f_3958) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1402, 2168,
            ],
            (1165, 0x828f_3958) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1402, 2169,
            ],
            (1166, 0x828f_3958) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1402, 2170,
            ],
            // KickTurn leaves fold Riding277, the shared KickTurn715 slots,
            // then the authored FromFakie/Right/Left leaf callbacks.
            (1016, 0x828f_2aa0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1403, 1404, 1902,
            ],
            (1017, 0x828f_2b18) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1403, 1404, 1903, 1904, 1905,
            ],
            (1018, 0x828f_2b18) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1403, 1404, 1906, 1907, 1908,
            ],
            // Source-shared flatground flip family thunks; each retains its authored slots.
            (800, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1482, 1483,
            ],
            (801, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1484, 1485,
            ],
            (802, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1486, 1487,
            ],
            (1087, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 2038, 2039,
            ],
            (1088, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 2040, 2041,
            ],
            (1089, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 2042, 2043,
            ],
            (872, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1632, 1633,
            ],
            (873, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1634, 1635,
            ],
            (874, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1636, 1637,
            ],
            (1095, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 2055, 2056,
            ],
            (1096, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 2057, 2058,
            ],
            (1097, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 2059, 2060,
            ],
            (879, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1639, 1640,
            ],
            (880, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1641, 1642,
            ],
            (881, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1643, 1644,
            ],
            (1103, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 2072, 2073,
            ],
            (1104, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 2074, 2075,
            ],
            (1105, 0x828f_1fe0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 2076, 2077,
            ],
            // FromManual: In parent27 slots, PlayAnimation, native no-op, JumpInto.
            (1076, 0x828f_31c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1126, 1127, 1128, 2013, 2014, 2015,
            ],
            (1084, 0x828f_31c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1129, 1130, 1131, 2030, 2031, 2032,
            ],
            (1092, 0x828f_31c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1145, 1146, 1147, 2047, 2048, 2049,
            ],
            (1100, 0x828f_31c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1148, 1149, 1150, 2064, 2065, 2066,
            ],
            // FromGrind1101: the same In565 parent slots, then SetTrickHeight's no-op, PlayAnimation
            // and JumpInto.
            (1077, 0x828f_3238) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1126, 1127, 1128, 2016, 2017, 2018,
            ],
            (1085, 0x828f_3238) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1129, 1130, 1131, 2033, 2034, 2035,
            ],
            (1093, 0x828f_3238) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1145, 1146, 1147, 2050, 2051, 2052,
            ],
            (1101, 0x828f_3238) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1148, 1149, 1150, 2067, 2068, 2069,
            ],
            (791, 0x828f_2130) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1126, 1127, 1128,
            ],
            (798, 0x828f_2130) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1129, 1130, 1131,
            ],
            (870, 0x828f_2130) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1145, 1146, 1147,
            ],
            (877, 0x828f_2130) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1148, 1149, 1150,
            ],
            (1075, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 455, 1126, 1127, 1128, 2011, 2012,
            ],
            (1083, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1129, 1130, 1131, 2028, 2029,
            ],
            (1086, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 456, 1129, 1130, 1131, 2036, 2037,
            ],
            (1091, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1145, 1146, 1147, 2045, 2046,
            ],
            (1094, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 522, 1145, 1146, 1147, 2053, 2054,
            ],
            (1099, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1148, 1149, 1150, 2062, 2063,
            ],
            (1102, 0x828f_32b0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 523, 1148, 1149, 1150, 2070, 2071,
            ],
            // BSGrab uses the same verified Into/Cyc/Out and landing thunks.
            (658, 0x828f_1510) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1289, 1290, 1291,
            ],
            (659, 0x828f_1448) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1292, 1293, 1294, 1295, 1296, 1297, 1298,
            ],
            (665, 0x828f_1750) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1308,
            ],
            // 828f1750 is StateUpdate_ToOffBoard_682 (Mute675 shares it): Lateflip_80,
            // then the packed23 PlayAnimation.
            (675, 0x828f_1750) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1335,
            ],
            (682, 0x828f_1750) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                159, 1351,
            ],
            (264, 0x828f_0068) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 77, 78,
                79, 80, 81, 82, 83, 84, 687,
            ],
            // FSGrab265 shares BSGrab264's 828f0068 thunk; its leaf PlayAnimation is 688.
            (265, 0x828f_0068) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 77, 78,
                79, 80, 81, 82, 83, 84, 688,
            ],
            // DBLGrab266 is the third 828f0068 alias; its leaf PlayAnimation is 689.
            (266, 0x828f_0068) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 77, 78,
                79, 80, 81, 82, 83, 84, 689,
            ],
            // Ground BS/FS grab powerslides Left/RightSlide 828f4b10 (4 aliases): expanded native
            // slot classes verified; InCandidateSlidingState/HandBusy/DisallowPumping updates
            // are BLRs.
            (1319, 0x828f_4b10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2278, 2279, 2280, 2281, 2411,
            ],
            (1320, 0x828f_4b10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2173, 2278, 2279, 2280, 2281, 2412,
            ],
            (1322, 0x828f_4b10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2300, 2301, 2302, 2303, 2413,
            ],
            (1323, 0x828f_4b10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2174, 2300, 2301, 2302, 2303, 2414,
            ],
            // HippyJump anticipation Carve279/281 828f05e0: expanded native slot classes verified.
            (279, 0x828f_05e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
                179, 180, 728, 729,
            ],
            (281, 0x828f_05e0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
                181, 182, 733, 734,
            ],
            // SpawnDismount31 828ee9b0: OffBoard_1 slots, ForcePhysics BLR, CreateAttribute and
            // PlayAnimation.
            (31, 0x828e_e9b0) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 108, 109, 110],
            // HippyJump KickTurn278/280 828f0568: expanded native slot classes verified;
            // ResetTimeSinceKickturn's update is a BLR.
            (278, 0x828f_0568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
                179, 180, 725, 726, 727,
            ],
            (280, 0x828f_0568) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
                181, 182, 730, 731, 732,
            ],
            // HippyJump Into93 828ef1d8: expanded native slot classes verified.
            (93, 0x828e_f1d8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
                183, 184, 185,
            ],
            // HippyJump Cyc94 828ef260: expanded native slot classes verified.
            (94, 0x828e_f260) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 93,
                186, 187,
            ],
            // Late flips 253..258 828eff60: expanded native slot classes verified; ForcePhysics,
            // SetDistComToBoard and SetDeckPitchAndYaw updates are BLRs.
            (253, 0x828e_ff60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                160, 653, 654, 655, 656, 657,
            ],
            (254, 0x828e_ff60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                160, 658, 659, 660, 661, 662,
            ],
            (255, 0x828e_ff60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                160, 663, 664, 665, 666, 667,
            ],
            (256, 0x828e_ff60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                160, 668, 669, 670, 671, 672,
            ],
            (257, 0x828e_ff60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                160, 673, 674, 675, 676, 677,
            ],
            (258, 0x828e_ff60) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 75, 76,
                160, 678, 679, 680, 681, 682,
            ],
            // InAirMount35 828eea88 (StateUpdate_InAirMount_35): expanded native slot classes
            // verified; folded AIProcess::OnEnd slots are the BLR Updates.
            (35, 0x828e_ea88) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 113, 114, 115],
            // CoffinToDBL267 828f00b8 (StateUpdate_CoffinToDBL_267): expanded native slot classes
            // verified; folded AIProcess::OnEnd slots are the BLR Updates.
            (267, 0x828f_00b8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 85, 86,
                87, 88, 89, 174, 690, 691, 692, 693,
            ],
            // Locomotion Idle ToStepForward317/ToStepBackward318 828f0720
            // (StateUpdate_ToStepBackward_312): expanded native slot classes verified; folded
            // AIProcess::OnEnd slots are the BLR Updates. 1.02 removed both; unreachable here.
            (317, 0x828f_0720) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 762],
            (318, 0x828f_0720) => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 763],
            // Kickflip_MikeCarroll/MarkGonzales TrickCyc1-3 828f0c50 (StateUpdate_TrickCyc3_507):
            // expanded native slot classes verified; folded AIProcess::OnEnd slots are the BLR
            // Updates.
            (505, 0x828f_0c50) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1108, 1109,
            ],
            (506, 0x828f_0c50) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1110, 1111,
            ],
            (507, 0x828f_0c50) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1112, 1113,
            ],
            (510, 0x828f_0c50) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1118, 1119,
            ],
            (511, 0x828f_0c50) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1120, 1121,
            ],
            (512, 0x828f_0c50) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1122, 1123,
            ],
            // Kickflip_MikeCarroll/MarkGonzales TrickOut 828f0cc8 (StateUpdate_TrickOut_513):
            // expanded native slot classes verified; folded AIProcess::OnEnd slots are the BLR
            // Updates.
            (508, 0x828f_0cc8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1114,
            ],
            (513, 0x828f_0cc8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1124,
            ],
            // RidingIdle Random RandGonz..Rand2 828f1c10 (StateUpdate_RandHsu_711): expanded native
            // slot classes verified; folded AIProcess::OnEnd slots are the BLR Updates.
            (706, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1394,
            ],
            (707, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1395,
            ],
            (708, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1396,
            ],
            (709, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1397,
            ],
            (710, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1398,
            ],
            (711, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1399,
            ],
            (712, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1400,
            ],
            (713, 0x828f_1c10) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 1401,
            ],
            // Signature kickflip TrickInto LeftGround/Takeoff 828f30c0 (StateUpdate_Takeoff_1064):
            // expanded native slot classes verified; folded AIProcess::OnEnd slots are the BLR
            // Updates.
            (781, 0x828f_30c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1105, 1106, 1107, 1461,
            ],
            (784, 0x828f_30c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1115, 1116, 1117, 1462,
            ],
            (1064, 0x828f_30c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1105, 1106, 1107, 1988,
            ],
            (1069, 0x828f_30c0) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1115, 1116, 1117, 1999,
            ],
            // Signature kickflip TakeOff FromManual/FromGrind 828f3048
            // (StateUpdate_FromManual_1071): expanded native slot classes verified; folded
            // AIProcess::OnEnd slots are the BLR Updates.
            (1066, 0x828f_3048) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1105, 1106, 1107, 1991, 1992, 1993,
            ],
            (1067, 0x828f_3048) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 448, 1105, 1106, 1107, 1994, 1995, 1996,
            ],
            (1071, 0x828f_3048) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1115, 1116, 1117, 2002, 2003, 2004,
            ],
            (1072, 0x828f_3048) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 67, 68,
                69, 449, 1115, 1116, 1117, 2005, 2006, 2007,
            ],
            // Turning Idle AggressiveOrLoose1169 828f39b8 (StateUpdate_Default_1170): expanded
            // native slot classes verified; folded AIProcess::OnEnd slots are the BLR Updates.
            (1169, 0x828f_39b8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1923, 1924, 1925, 1926, 1927, 2171,
            ],
            // Turning Idle Still1171/Moving1172 828f3a08 (b StateUpdate_Idle_1025): expanded native
            // slot classes verified; folded AIProcess::OnEnd slots are the BLR Updates.
            (1171, 0x828f_3a08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1923, 1924, 1925, 1926, 1927,
            ],
            (1172, 0x828f_3a08) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1923, 1924, 1925, 1926, 1927,
            ],
            // DBLGrabCyc LeftSlide1325/RightSlide1326 828f4b68 (StateUpdate_LeftSlide_1325):
            // expanded native slot classes verified; folded AIProcess::OnEnd slots are the BLR
            // Updates.
            (1325, 0x828f_4b68) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2324, 2325, 2326, 2327, 2328, 2415,
            ],
            (1326, 0x828f_4b68) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 2175, 2324, 2325, 2326, 2327, 2328, 2416,
            ],
            // TailManual Brake Stationary1330 828f4cc8 (StateUpdate_Stationary_1330): expanded
            // native slot classes verified; folded AIProcess::OnEnd slots are the BLR Updates.
            (1330, 0x828f_4cc8) => &[
                0, 1, 2, 3, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 90, 91,
                92, 177, 178, 722, 723, 724, 1413, 1928, 1929, 1930, 2176, 2342, 2343, 2422,
            ],
            (current, address) => super::grind_states::expected(current, address)
                .or_else(|| super::moving_states::expected(current, address))
                .or_else(|| super::handplant_states::expected(current, address))
                .ok_or(E::PendingStateUpdate(address))?,
        };
        if self.current_behaviours != expected {
            if std::env::var_os("SKATE2_TRACE_MOTION_ACTIVE_SET").is_some() {
                eprintln!(
                    "motion active-set mismatch state={} address={:#x} actual={:?} expected={expected:?}",
                    self.current_state, state.update_address, self.current_behaviours
                );
            }
            return Err(E::InvalidActiveSet);
        }
        for position in 0..expected.len() {
            self.callback(actor, context, position, false, writer)?;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "typed native lifecycle dispatch retains per-class instance and live services"
    )]
    fn callback(
        &mut self,
        actor: &ActorConstruction,
        context: &mut BoundMotionGraphContext<ActorIdentity>,
        position: usize,
        begin: bool,
        writer: &mut dyn ForcePhysicsWriter,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let index = self.current_behaviours[position];
        let slot = &mut self.instances[position];
        let object = slot.behaviour.as_ref().ok_or(E::InvalidActiveSet)?;
        context.set_generated_instance(
            slot.instance
                .as_ref()
                .map(Instance::token)
                .map(StateGraphContextHandle::shallow_copy),
        );
        match (&object.payload, slot.instance.as_mut()) {
            (Payload::Named(Named::SetManualAngle { angle: name }), Some(Instance::SetManualAngle { angle, velocity, .. })) => {
                if begin { *angle = 0.0; *velocity = 0.0; Ok(()) }
                else { actor.motion_manual_angle_update(context, *name, angle, velocity) }
            }
            (Payload::Named(Named::SetManualAngle { .. }), None) => Err(E::MissingOwner("SetManualAngle instance")),
            (Payload::Named(Named::JumpInto { .. }), _) if begin => Ok(()), //825187c0
            (Payload::Named(Named::JumpInto { attribute }), Some(Instance::JumpInto { first_update, .. })) => {
                if *first_update {
                    let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone()
                        .ok_or(E::MissingOwner("JumpInto SetAttributes cvars"))?;
                    actor.check_motion_runtime_context(context)?;
                    actor.lifecycle_animation(0)?.lock().map_err(|_| E::Lock)?
                        .motion_jump_into(*attribute, &cvars)?;
                    *first_update = false;
                }
                Ok(())
            }
            (Payload::Named(Named::JumpInto { .. }), None) => Err(E::MissingOwner("JumpInto instance")),
            (Payload::Named(Named::SetDeckPitchAndYaw { yaw, pitch }), None) => {
                // Begin82cb04d0; CreateInstance is the shared null and Update825187c0 a BLR.
                if begin { actor.motion_set_deck_pitch_and_yaw_begin(context, *yaw, *pitch) } else { Ok(()) }
            }
            (Payload::Named(Named::ChooseRandomSketchyLanding { random }), None) => {
                if begin { actor.motion_sketchy_landing_attribute(context, *random, true)?; }
                Ok(()) // Update825187c0 is a native no-op.
            }
            (Payload::Named(Named::SetManualOutTimer { .. }), None) => Ok(()), // Begin/Update825187c0.
            (Payload::Plain(Plain::PowerSlideManualAtt), None) => {
                if begin { Ok(()) } else { actor.motion_runtime_power_slide_manual_attribute(context) }
            }
            (Payload::Plain(Plain::FootPlantAbsorb), None) => {
                // Begin825187c0 is a BLR; Update82cb4f18 publishes "absorblength".
                if begin { Ok(()) } else { actor.motion_foot_plant_absorb_update(context) }
            }
            (Payload::Named(Named::SetTrickAttr { trick }), None) => {
                if begin { actor.motion_set_trick_attribute_begin(context, *trick)?; }
                Ok(()) // Update825187c0 is a BLR; native CreateInstance is null.
            }
            (Payload::ToggleBoard(names), Some(Instance::ToggleBoard { data, .. })) if begin => {
                data.begin(names);
                Ok(())
            }
            (Payload::ToggleBoard(_), None) if begin => {
                Err(E::MissingOwner("ToggleBoard instance"))
            }
            (Payload::ToggleBoard(names), Some(Instance::ToggleBoard { data, .. })) => {
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_runtime_toggle_board_update(context, data, names, cvars.as_ref())
            }
            (Payload::ToggleBoard(_), None) => Err(E::MissingOwner("ToggleBoard instance")),
            (Payload::Named(Named::ComputeRepushDeadline { .. }), None) => Ok(()),
            (Payload::Named(Named::SetSpeed { .. }), None) if begin => {
                Ok(()) // SetSpeed::Begin is the shared BLR at 0x825187c0.
            }
            (Payload::Named(Named::SetSpeed { attribute, set_manually, set_to_value }), None) => {
                actor.motion_runtime_set_speed(context, *attribute, *set_manually, *set_to_value)
            }
            (Payload::Named(Named::KickTurnSteering { max_height, spin_scale }), instance) => {
                let data = match instance {
                    Some(Instance::KickTurnSteering { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let attributes = services.as_ref().map(|services| &services.kickturn);
                if begin {
                    actor.motion_runtime_kickturn_steering_begin(context, data, attributes)
                } else {
                    let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                    actor.motion_runtime_kickturn_steering_update(
                        context,
                        data,
                        *max_height,
                        *spin_scale,
                        attributes,
                        cvars.as_ref(),
                        self.delta_time,
                    )
                }
            }
            (Payload::ResetSkaterAnimation, None) => {
                if begin {
                    actor.try_reset_skater_animation_begin(context)
                } else {
                    Ok(()) // Update is the shared BLR at 0x825187c0.
                }
            }
            (Payload::ResetToGivenStance, None) => {
                if begin {
                    actor.try_reset_to_given_stance_begin(context)
                } else {
                    Ok(()) // Update is the shared BLR at 0x825187c0.
                }
            }
            (Payload::Named(Named::PushOut { is_right_foot }), None) => {
                if begin {
                    actor.motion_runtime_push_out_begin(context, *is_right_foot)
                } else {
                    Ok(()) // Source Update is a BLR at 0x825187c0.
                }
            }
            (Payload::Named(Named::ComputeTargetCoefsFromSpeedAndStrength { regular_attributes }), None) => {
                if begin {
                    Ok(()) // 825187c0: Begin is a BLR.
                } else {
                    actor.motion_runtime_compute_target_coefs(context, *regular_attributes)
                }
            }
            (Payload::Named(Named::SetPushCoefs { on_first_update_only }), Some(Instance::SetPushCoefs { data, .. })) => {
                if begin {
                    data.begin();
                    Ok(())
                } else {
                    let services = actor.action_runtime_services().map_err(E::Actor)?;
                    actor.motion_runtime_set_push_coefs(
                        context,
                        data,
                        *on_first_update_only,
                        services.as_ref().map(|services| &services.pushing),
                        self.delta_time,
                    )
                }
            }
            (Payload::Named(Named::SetPushCoefs { .. }), None) => {
                Err(E::MissingOwner("SetPushCoefs instance"))
            }
            (Payload::Named(Named::PushCycle { new_push }), Some(Instance::PushCycle { data, .. })) => {
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_runtime_push_cycle(
                    context, data, new_push,
                    services.as_ref().map(|services| &services.pushing), cvars.as_ref(),
                    if begin {
                        crate::actor_construction::motion_runtime_session::push_cycle::Phase::Begin
                    } else {
                        crate::actor_construction::motion_runtime_session::push_cycle::Phase::Update
                    },
                )
            }
            (Payload::Named(Named::PushCycle { .. }), None) => Err(E::MissingOwner("PushCycle instance")),
            (Payload::Plain(Plain::ComputeFirstPushStrength), Some(Instance::FirstPush { data, .. })) => {
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_runtime_compute_first_push(
                    context,
                    data,
                    services.as_ref().map(|services| &services.pushing),
                    cvars.as_ref(),
                    self.delta_time,
                    if begin {
                        crate::actor_construction::motion_runtime_session::compute_first_push::Phase::Begin
                    } else {
                        crate::actor_construction::motion_runtime_session::compute_first_push::Phase::Update
                    },
                )
            }
            (Payload::Plain(Plain::ComputeFirstPushStrength), None) => {
                Err(E::MissingOwner("ComputeFirstPushStrength instance"))
            }
            (
                Payload::FilterMotionGraphIntent(payload),
                Some(Instance::FilterMotionGraphIntent { token, data }),
            ) if begin && matches!(index, 153 | 154 | 156 | 157 | 642..=649 | 1249 | 1250 | 1263 | 1264 | 1294 | 1295 | 1324 | 1325 | 1333 | 1340 | 1341 | 1349 | 1935) => {
                // Raw 82cd5e4c/54/58: all Data writes precede graph lookup.
                data[0] = 0.0;
                data[2] = payload.starting_value;
                data[1] = 0.0;
                actor.motion_filter_intent_begin(
                    context,
                    token,
                    payload.output_intent,
                    payload.starting_value,
                )
            }
            (Payload::FilterMotionGraphIntent(_), None) if begin && matches!(index, 153 | 154 | 156 | 157 | 642..=649 | 1249 | 1250 | 1263 | 1264 | 1294 | 1295 | 1324 | 1325 | 1333 | 1340 | 1341 | 1349 | 1935) => {
                Err(E::MissingOwner("FilterMotionGraphIntent instance"))
            }
            (
                Payload::FilterMotionGraphIntent(payload),
                Some(Instance::FilterMotionGraphIntent { token, data }),
            ) if !begin && matches!(index, 153 | 154 | 156 | 157 | 642..=649 | 1249 | 1250 | 1263 | 1264 | 1294 | 1295 | 1324 | 1325 | 1333 | 1340 | 1341 | 1349 | 1935) => {
                if context.controller().is_none() {
                    return Err(E::MissingOwner("Filter Motion controller"));
                }
                crate::actor_construction::motion_filter_intent_update::with_elapsed_time(
                    data,
                    self.delta_time,
                    |data| actor.motion_filter_intent_update(context, token, payload, data),
                )
            }
            (Payload::FilterMotionGraphIntent(_), None) if !begin && matches!(index, 153 | 154 | 156 | 157 | 642..=649 | 1249 | 1250 | 1263 | 1264 | 1294 | 1295 | 1324 | 1325 | 1333 | 1340 | 1341 | 1349 | 1935) => {
                Err(E::MissingOwner("FilterMotionGraphIntent instance"))
            }
            (Payload::Plain(Plain::IsLanding),_)=> if begin {actor.motion_set_is_landing(context,true)}else{Ok(())},
            (Payload::Plain(Plain::RunoutFront), None) => if begin { actor.motion_set_runout_front(context, true) } else { Ok(()) }, // Update825187c0 is BLR
            (Payload::Plain(Plain::Dismount), None) => if begin { actor.motion_set_dismounting(context, true) } else { Ok(()) }, // Update825187c0 is BLR
            (Payload::Plain(Plain::OBAir), None) => if begin { actor.motion_set_obair_landing(context, true) } else { Ok(()) },
            (Payload::Control(Control::HandBusy(hand)), None) => {
                if begin { actor.motion_adjust_hand_busy(context, *hand, 1) } else { Ok(()) }
            }
            (Payload::Control(Control::EndGesture(hand)), None) => {
                if begin { actor.motion_end_hand_gestures(context, *hand) } else { Ok(()) }
            }
            (Payload::Named(Named::SetGrabType { grab }), None) => {
                if begin { actor.motion_set_grab_type(context, Some(*grab)) } else { Ok(()) }
            }
            (Payload::Named(Named::TweakProject { intent_x, intent_y, attribute_x, attribute_y }), None) => {
                if begin {
                    Ok(()) // Native Begin is the shared BLR at 0x825187c0.
                } else {
                    actor.motion_project_grab_tweak(context, intent_x, intent_y, *attribute_x, *attribute_y)
                }
            }
            // Every ScoringGrabs shares one binding (vtable 82302b78): Begin is
            // the BLR 825187c0 and Update 82c8ef88 is one virtual Create call,
            // so the payload alone selects the behaviour.
            (Payload::Named(Named::SetHandPlantAnticLength { antic_length }), None) => {
                if begin { actor.motion_handplant_antic_begin(context, *antic_length)?; }
                Ok(())
            }
            (Payload::Control(Control::ScoringHandPlants(scoring)), None) => {
                if !begin { actor.motion_score_handplant_update(context, scoring)?; }
                Ok(())
            }
            (Payload::Control(Control::ScoringGrabs(scoring)), None) => {
                if begin {
                    Ok(()) // Native Begin is the shared BLR at 0x825187c0.
                } else {
                    actor.motion_score_fs_grab_update(context, scoring)
                }
            }
            (Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::HippyJumpAntic { trick_height, attributes, .. }), instance) => {
                let Some(Instance::HippyJumpAntic {time, ..}) = instance else { return Err(E::MissingOwner("HippyJumpAntic instance")); };
                if begin { Ok(()) } else { actor.motion_hippy_antic(context, trick_height, attributes, time) }
            }
            (Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::MovingObject(behaviour)), instance) => {
                let Some(Instance::MovingObject { data, .. }) = instance else { return Err(E::MissingOwner("MovingObject instance")); };
                if begin {
                    actor.motion_moving_object_begin(context, behaviour.direction, data)
                } else {
                    let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone()
                        .ok_or(E::MissingOwner("MovingObject cvars"))?;
                    actor.motion_moving_object_update(context, behaviour, data, &cvars)
                }
            }
            (Payload::Plain(Plain::AirDismounting), instance) => {
                let Some(Instance::AirDismounting {frames, ..}) = instance else { return Err(E::MissingOwner("AirDismounting instance")); };
                actor.motion_air_dismounting(context, frames, begin)
            }
            (Payload::Plain(Plain::SetLandingData),_)=> if begin {actor.motion_set_landing_data(context)}else{Ok(())},
            (Payload::Control(Control::DisableTricks {timer_length}),instance)=>{
                let data=match instance {Some(Instance::DisableTricks {data,..})=>data,None=>return Err(E::MissingOwner("DisableTricks instance")),_=>return Err(E::InvalidActiveSet)};
                if begin {data.begin(*timer_length,|value|actor.motion_set_allow_trick(context,value))}else{data.update(self.delta_time,|value|actor.motion_set_allow_trick(context,value))}
            }
            (Payload::Control(Control::Wipeout { lean, twist, gesture_x, gesture_y }), instance) => {
                let data = match instance {
                    Some(Instance::Wipeout { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = if begin { None } else { actor.action_runtime_services().map_err(E::Actor)? };
                actor.motion_runtime_wipeout(context, data,
                    crate::actor_construction::motion_runtime_session::wipeout::kernel::Names {
                        lean: *lean, twist: *twist, gesture_x: *gesture_x, gesture_y: *gesture_y,
                    }, services.as_ref().map(|value| &value.wipeout), begin)
            }
            (Payload::Control(Control::ForcePhysics(mode)), _) => {
                if begin { actor.motion_force_physics_begin(context, *mode, writer) } else { Ok(()) }
            }
            (Payload::Control(Control::PlayWipeoutRecoverAnim { transition_length }), None) => {
                if begin {
                    actor.motion_runtime_play_wipeout_recover_anim(context, *transition_length)
                } else {
                    Ok(()) // Update825187c0 is a single blr.
                }
            }
            (Payload::CreateAttribute(payload), _) => actor.motion_runtime_create_attribute(context, payload,
                if begin { crate::actor_construction::motion_runtime_session::create_attribute::kernel::Phase::Begin }
                else { crate::actor_construction::motion_runtime_session::create_attribute::kernel::Phase::Update }),
            (Payload::SetScoreAugmentation { augmentation, mirrored }, _) => {
                if begin { Ok(()) } else { actor.motion_score_augmentation_update(context, *augmentation, *mirrored) }
            }
            (Payload::Control(Control::ScoringTrick { name }), _) => {
                if begin { Ok(()) } else { actor.motion_score_trick_update(context, name) }
            }
            (Payload::Named(Named::EndShimmy { blend_out_time }), _) => {
                if begin { actor.motion_end_shimmy_begin(context, *blend_out_time) } else { Ok(()) }
            }
            (Payload::Named(Named::SetTrickHeight { from, rename, set_manually, set_to_value }), _) => {
                if !begin { return Ok(()); }
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_set_trick_height_begin(context, *from, *rename, *set_manually, *set_to_value, cvars.as_ref())
            }
            (Payload::Control(Control::SetDistComToBoard { attribute, set_manually, adjust_for_velocity }), _) => {
                if !begin { return Ok(()); }
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_dist_com_to_board_begin(context, *attribute, *set_manually, *adjust_for_velocity, cvars.as_ref(), self.delta_time)
            }
            (
                Payload::CharacterGesture
                | Payload::AttachIntent { .. }
                | Payload::UpdateTimeSinceTeleport
                | Payload::UpdateRidingFakie { .. }
                | Payload::FakieHeadChannel { .. }
                | Payload::SettingBodyTilt { .. }
                | Payload::UpdateTimeSinceKickturn
                | Payload::PowerSliding
                | Payload::BodySpin
                | Payload::StoreLandingData
                | Payload::ControlAirLegExtension { .. }
                | Payload::Plain(Plain::UpdateManualOutTimer | Plain::Pumping),
                _,
            ) if begin => Ok(()),
            (Payload::Plain(Plain::Locomotion), _) => {
                if begin {
                    let refs=self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone().ok_or(E::MissingOwner("Motion locomotion cvars"))?;
                    actor.motion_locomotion_begin(context,&refs)
                } else {actor.motion_locomotion_update(context)}
            }
            (Payload::Plain(Plain::ClearTrickAttr), _) => Ok(()), // Create returnsNULL; Begin/Update are825187c0.
            (Payload::Plain(Plain::ApplyingBodyTilt), _) => {
                if begin {
                    actor.motion_runtime_set_applying_body_tilt(context, true)
                } else {
                    Ok(())
                }
            }
            (Payload::Plain(Plain::IsDoingTrick), _) => {
                if begin { actor.motion_set_is_doing_trick(context, true) } else { Ok(()) }
            }
            (Payload::Plain(Plain::IsManualing), None) => {
                if begin { actor.motion_runtime_set_is_manualing(context, true) } else { Ok(()) }
            }
            (Payload::Plain(Plain::InCandidateSlidingState), None) => {
                if begin { actor.motion_runtime_set_candidate_sliding(context, true) } else { Ok(()) }
            }
            (Payload::Named(Named::SteppingBoardNoBoard { board, no_board }), None) => {
                if begin { Ok(()) } else { actor.motion_stepping_board_attribute(context, *board, *no_board) }
            }
            (Payload::Named(Named::CreateSlide { right }), None) => {
                if begin { actor.motion_create_slide_begin(context) } else {
                    let services = actor.action_runtime_services().map_err(E::Actor)?;
                    let services = services.as_ref().ok_or(E::MissingOwner("CreateSlide services"))?;
                    actor.motion_create_slide_update(context, *right, &services.power_sliding)
                }
            }
            (Payload::Named(Named::PowerSlideDecel { .. }), instance) => {
                let Some(Instance::PowerSlideDecel { value, .. }) = instance else { return Err(E::MissingOwner("PowerSlideDecel instance")); };
                if begin { *value = 0.0; Ok(()) } else {
                    let services = actor.action_runtime_services().map_err(E::Actor)?;
                    let services = services.as_ref().ok_or(E::MissingOwner("PowerSlideDecel services"))?;
                    let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                    actor.motion_slide_decel_update(context, value, &services.power_sliding, cvars.as_ref().ok_or(E::MissingOwner("slide cvars"))?, &services.overlay)
                }
            }
            (Payload::Plain(Plain::PowerSlideSpin), None) => {
                if !begin { return Ok(()); }
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let services = services.as_ref().ok_or(E::MissingOwner("PowerSlideSpin services"))?;
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_slide_spin_begin(context, &services.power_sliding, cvars.as_ref().ok_or(E::MissingOwner("slide cvars"))?, &services.overlay)
            }
            (Payload::Plain(Plain::IsPowerSliding), None) => {
                if begin { actor.motion_runtime_set_power_sliding(context, true) } else { Ok(()) }
            }
            (Payload::Plain(Plain::DisallowPumping), None) => {
                if begin { actor.motion_runtime_disallow_pumping(context, true) } else { Ok(()) }
            }
            (Payload::Plain(Plain::UpdateIsWeightOnNose), _) => {
                if begin { actor.motion_set_weight_on_nose(context, true) } else { Ok(()) }
            }
            (Payload::Plain(Plain::IsAnticipating), _) => {
                if begin { actor.motion_set_is_anticipating(context, true) } else { Ok(()) }
            }
            // Native Create returns null; Update/End825187c0 are BLR.
            (Payload::Plain(Plain::InitMovingObjects), None) => {
                if begin { actor.motion_init_moving_objects(context) } else { Ok(()) }
            }
            (Payload::Plain(Plain::InitPush), _) => {
                if begin { actor.motion_runtime_init_push(context) } else { Ok(()) }
            }
            (Payload::Plain(Plain::UpdateManualOutTimer), _) => {
                actor.motion_runtime_update_manual_out_timer(context, self.delta_time)
            }
            (Payload::Plain(Plain::ResetTimeSinceKickturn), None) => {
                if begin {
                    actor.motion_runtime_reset_time_since_kickturn(context)
                } else {
                    Ok(()) // Native Update is the shared BLR at 0x825187c0.
                }
            }
            (Payload::UpdateTimeSinceKickturn, _) => {
                actor.motion_runtime_update_time_since_kickturn(context, self.delta_time)
            }
            (Payload::Named(Named::Crouching { crouch }), instance) => {
                let data = match instance {
                    Some(Instance::Crouching { data, .. }) => data,
                    None => return Err(E::MissingOwner("Crouching instance")),
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self
                    .load
                    .statics
                    .inner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .cvars
                    .clone();
                actor.motion_runtime_crouching(
                    context,
                    services.as_ref().map(|s| &s.crouching),
                    cvars.as_ref(),
                    *crouch,
                    self.delta_time,
                    data,
                    begin,
                )
            }
            (
                Payload::Control(Control::SetTurning {
                    angle,
                    direction,
                    quickness,
                    holding,
                    speed,
                }),
                instance,
            ) => {
                let data = match instance {
                    Some(Instance::SetTurning { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                if begin {
                    data.ok_or(E::MissingOwner("SetTurning instance"))?.begin();
                    return Ok(());
                }
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self
                    .load
                    .statics
                    .inner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .cvars
                    .clone();
                actor.motion_runtime_set_turning(
                    context,
                    data,
                    Names {
                        angle: *angle,
                        direction: *direction,
                        quickness: *quickness,
                        holding: *holding,
                        speed: *speed,
                    },
                    TurningRuntime {
                        attributes: services.as_ref().map(|s| &s.turning),
                        sliding_attributes: services.as_ref().map(|s| &s.power_sliding),
                        cvars: cvars.as_ref(),
                        overlay: services.as_ref().map(|s| &s.overlay),
                        dt: self.delta_time,
                    },
                )
            }
            (Payload::SettingBodyTilt { tilt_x }, instance) => {
                let data = match instance {
                    Some(Instance::SettingBodyTilt { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let physical = actor.cached_physical_allocation().map_err(E::Physical)?;
                let cvars = self
                    .load
                    .statics
                    .inner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .cvars
                    .clone();
                let mut tuning = crate::input_cvar_runtime::body_tilt::BodyTiltRuntime {
                    attributes: services.as_ref().map(|s| &s.body_tilt),
                    cvars: cvars.as_ref(),
                    physical_vault: &physical.owner().inputs().vault,
                };
                actor.motion_runtime_setting_body_tilt(context, data, *tilt_x, &mut tuning)
            }
            (Payload::PowerSliding, instance) => {
                let data = match instance {
                    Some(Instance::PowerSliding { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self
                    .load
                    .statics
                    .inner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .cvars
                    .clone();
                actor.motion_runtime_power_sliding(
                    context,
                    data,
                    services.as_ref().map(|s| &s.power_sliding),
                    cvars.as_ref(),
                    services.as_ref().map(|s| &s.overlay),
                    self.delta_time,
                )
            }
            (Payload::BodySpin, instance) => {
                let data = match instance {
                    Some(Instance::BodySpin { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let physical = actor.cached_physical_allocation().map_err(E::Physical)?;
                let cvars = self
                    .load
                    .statics
                    .inner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .cvars
                    .clone();
                let mut tuning = crate::input_cvar_runtime::body_spin_motion::BodySpinRuntime {
                    attributes: services.as_ref().map(|s| &s.body_spin),
                    cvars: cvars.as_ref(),
                    physical_vault: &physical.owner().inputs().vault,
                };
                actor.motion_runtime_body_spin(context, data, &mut tuning, crate::actor_construction::motion_runtime_session::prepare_land::PrepareLandRuntime {
                    attributes: services.as_ref().map(|s| &s.prepare_land),
                    body_tilt: services.as_ref().map(|s| &s.body_tilt),
                    cvars: cvars.as_ref(),
                })
            }
            // StandingJumpLandingCOG: null instance, Begin82cb44b8, Update BLR.
            (Payload::Named(Named::StandingJumpLandingCog { distance_to_cog }), None) => {
                if begin {
                    actor.motion_standing_jump_landing_cog_begin(context, *distance_to_cog)
                } else {
                    Ok(())
                }
            }
            (
                Payload::Plain(Plain::StandingJumpDescent),
                Some(Instance::StandingJumpDescent { begun, .. }),
            ) => {
                if begin {
                    *begun = true; // Begin82c95bf8: byte +8 only.
                    Ok(())
                } else {
                    actor.motion_standing_jump_descent_update(context)
                }
            }
            (Payload::Plain(Plain::StandingJumpDescent), None) => {
                Err(E::MissingOwner("StandingJumpDescent instance"))
            }
            // LandOnBoard: null instance, Begin82ca5fa8, Update BLR.
            (Payload::Plain(Plain::LandOnBoard), None) => {
                if begin {
                    actor.motion_land_on_board_begin(context)
                } else {
                    Ok(())
                }
            }
            (Payload::Control(Control::LandingOffBoardAir { impact, close_to_land }), instance) => {
                let data = match instance {
                    Some(Instance::LandingOffBoardAir { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                if begin {
                    actor.motion_landing_offboard_air_begin(context, data, cvars.as_ref())
                } else {
                    actor.motion_landing_offboard_air_update(
                        context,
                        data,
                        *impact,
                        *close_to_land,
                        cvars.as_ref(),
                    )
                }
            }
            (Payload::ControlAirLegExtension {drive_distance_to_com,bone},instance) => {
                let data=match instance {Some(Instance::AirLeg {data,..})=>Some(data),None=>None,_=>return Err(E::InvalidActiveSet)};
                let cvars=self.load.statics.inner.lock().map_err(|_|E::Lock)?.cvars.clone();
                actor.motion_air_leg_update(context,data,*drive_distance_to_com,*bone,self.delta_time,cvars.as_ref())
            }
            (Payload::StoreLandingData, instance) => {
                let data = match instance {
                    Some(Instance::StoreLandingData { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                actor.motion_runtime_store_landing_data(context, data)
            }
            // CreateGrindAttributes: null CreateInstance82bb3da8, Begin825187c0 BLR.
            (Payload::Plain(Plain::CreateGrindAttributes), None) => {
                if begin {
                    return Ok(());
                }
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_runtime_create_grind_attributes(
                    context,
                    &GrindingRuntime {
                        attributes: services.as_ref().map(|s| &s.grinding),
                        overlay: services.as_ref().map(|s| &s.overlay),
                        cvars: cvars.as_ref(),
                        dt: self.delta_time,
                    },
                )
            }
            (
                Payload::Control(Control::ControlGrindCrouch {
                    drive_distance_to_com,
                }),
                instance,
            ) => {
                let data = match instance {
                    Some(Instance::ControlGrindCrouch { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_runtime_control_grind_crouch(
                    context,
                    &GrindingRuntime {
                        attributes: services.as_ref().map(|s| &s.grinding),
                        overlay: services.as_ref().map(|s| &s.overlay),
                        cvars: cvars.as_ref(),
                        dt: self.delta_time,
                    },
                    data,
                    *drive_distance_to_com,
                    if begin {
                        GrindingPhase::Begin
                    } else {
                        GrindingPhase::Update
                    },
                )
            }
            (
                Payload::Control(Control::GrindControlFade {
                    distance_board_to_cog,
                    twist,
                    twist_intent,
                }),
                instance,
            ) => {
                let data = match instance {
                    Some(Instance::GrindControlFade { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_runtime_grind_control_fade(
                    context,
                    &GrindingRuntime {
                        attributes: services.as_ref().map(|s| &s.grinding),
                        overlay: services.as_ref().map(|s| &s.overlay),
                        cvars: cvars.as_ref(),
                        dt: self.delta_time,
                    },
                    data,
                    &ControlFadeNames {
                        distance_board_to_cog: *distance_board_to_cog,
                        twist: *twist,
                        twist_intent: *twist_intent,
                    },
                    if begin {
                        GrindingPhase::Begin
                    } else {
                        GrindingPhase::Update
                    },
                )
            }
            (Payload::Plain(Plain::Pumping), instance) => {
                let data = match instance {
                    Some(Instance::Pumping { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let services = actor.action_runtime_services().map_err(E::Actor)?;
                let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone();
                actor.motion_runtime_pumping(context, services.as_ref().map(|s| &s.pumping), cvars.as_ref(), data, false)
            }
            (Payload::SharedAction(o), _)
                if o.class == crate::generated_action_load::BehaviourClass::PrintText2D =>
            {
                Ok(())
            }
            (Payload::PlayAnimation(behaviour), instance) => {
                let data = match instance {
                    Some(Instance::PlayAnimation { data, .. }) => Some(data),
                    None => None,
                    _ => return Err(E::InvalidActiveSet),
                };
                let mut backend = PlayAnimationActor { actor, context };
                if begin {
                    behaviour.begin_runtime(data, &mut backend)
                } else {
                    behaviour.update_runtime(data, &mut backend)
                }
                .map_err(|e| E::PlayAnimation(Box::new(e)))
            }
            (
                Payload::AttachIntent {
                    intent,
                    attribute,
                    settable,
                },
                _,
            ) => actor.motion_runtime_attach_intent(context, intent, *attribute, *settable),
            (Payload::Named(Named::SetBumpCoefficients { x, y }), _) => {
                if begin { actor.motion_runtime_set_bump_coefficients(context, *x, *y) } else { Ok(()) }
            }
            (Payload::CharacterGesture, _) => {
                let cvars = self
                    .load
                    .statics
                    .inner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .cvars
                    .clone()
                    .ok_or(E::MissingOwner("Motion cvars"))?;
                actor
                    .try_update_character_gesture(&cvars, context)
                    .map_err(E::Gesture)
            }
            (Payload::UpdateTimeSinceTeleport, _) => {
                actor.motion_runtime_update_teleport_time(context, self.delta_time)
            }
            (Payload::Vault(crate::generated_motion_load::construct::vault::VaultMotionBehaviour::SkitchShimmying { attributes, desired_direction }), instance) => {
                let Some(Instance::SkitchShimmying { speed, .. }) = instance else {
                    return Err(E::MissingOwner("Skitch shimmy instance"));
                };
                if begin { actor.motion_skitch_shimmy_begin(context, speed) } else {
                    let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone()
                        .ok_or(E::MissingOwner("Skitch shimmy cvars"))?;
                    actor.motion_skitch_shimmy_update(context, attributes, *desired_direction, speed, &cvars)
                }
            }
            (Payload::EnterSkitchingBehaviour(owner), instance) => {
                let Some(Instance::EnterSkitching { data, .. }) = instance else {
                    return Err(E::MissingOwner("EnterSkitching instance"));
                };
                if begin { actor.motion_enter_skitch_begin(context, data) } else {
                    let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone()
                        .ok_or(E::MissingOwner("EnterSkitching cvars"))?;
                    actor.motion_enter_skitch_update(context, &owner.attributes, data, &cvars)
                }
            }
            (Payload::SkitchingBehaviour(owner), instance) => {
                let Some(Instance::Skitching { data, .. }) = instance else {
                    return Err(E::MissingOwner("Skitching instance"));
                };
                if begin { actor.motion_skitch_begin(context, &owner.attributes, data) } else {
                    let cvars = self.load.statics.inner.lock().map_err(|_| E::Lock)?.cvars.clone()
                        .ok_or(E::MissingOwner("Skitching cvars"))?;
                    actor.motion_skitch_update(context, &owner.attributes, data, &cvars)
                }
            }
            (
                payload @ Payload::UpdateRidingFakie { .. },
                Some(Instance::RidingFakie {
                    slow_time,
                    teleport_time,
                    ..
                }),
            ) => actor.motion_runtime_update_riding_fakie(
                context,
                payload,
                slow_time,
                teleport_time,
                self.delta_time,
            ),
            (
                Payload::FakieHeadChannel { channel },
                Some(Instance::FakieHead { torso, active, .. }),
            ) => actor.motion_runtime_update_fakie_head(context, channel, torso, active),
            (
                Payload::Named(Named::FingerFlipOut {
                    grab_intent,
                    grab_blend,
                }),
                Some(Instance::FingerFlipOut { timer, .. }),
            ) => {
                if begin {
                    actor.motion_finger_flip_out_begin(context, timer)
                } else {
                    let services = actor.action_runtime_services().map_err(E::Actor)?;
                    actor.motion_finger_flip_out_update(
                        context,
                        timer,
                        grab_intent,
                        *grab_blend,
                        services.as_ref().map(|services| &services.finger_flip),
                        self.delta_time,
                    )
                }
            }
            (Payload::Named(Named::FingerFlipOut { .. }), None) => {
                Err(E::MissingOwner("FingerFlipOut instance"))
            }
            _ => Err(E::PendingCallback {
                index,
                address: binding(super::super::data::BEHAVIOURS[usize::from(index)].name)
                    .ok_or(E::MissingOwner("Motion binding"))?
                    .callbacks[if begin { 1 } else { 2 }],
            }),
        }
    }
}
