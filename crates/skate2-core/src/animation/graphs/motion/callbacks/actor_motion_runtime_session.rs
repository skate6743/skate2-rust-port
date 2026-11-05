//! Same-owner queries for generated Motion evaluation and callbacks.
#[path = "actor_motion_air_attribute.rs"]
mod air_attribute;
#[path = "actor_motion_air_leg.rs"]
pub(crate) mod air_leg;
#[path = "actor_motion_handplant.rs"]
pub(crate) mod handplant;
#[path = "actor_motion_hippy_dismount.rs"]
mod hippy_dismount;
#[path = "actor_motion_locomotion.rs"]
mod locomotion;
#[path = "actor_motion_manual_angle.rs"]
pub(crate) mod manual_angle;
#[path = "actor_motion_moving_objects.rs"]
mod moving_objects;
#[path = "actor_motion_prepare_land_query.rs"]
pub(crate) mod prepare_land;
#[path = "actor_motion_toggle_board_update.rs"]
mod toggle_board;
#[path = "actor_motion_transition_hook.rs"]
mod transition_hook;
#[path = "actor_motion_runtime_trick_conditions.rs"]
pub(crate) mod trick_conditions;
#[path = "actor_motion_wipeout.rs"]
pub(crate) mod wipeout;
#[path = "actor_motion_wipeout_recovery_begin.rs"]
mod wipeout_recovery_begin;
use crate::physics_state_ids::PHYSICS_STATE_HANDPLANT;
use super::{ActorConstruction, ActorIdentity};
use crate::{
    generated_action_load::construct::{HandPlantDirection, HandPlantState},
    generated_motion_load::runtime::MotionUpdateError,
    input_cvar_runtime::InputCvarReferences,
    motion_graph_entity_authority::BoundMotionGraphContext,
    play_animation::FastString36,
};
use std::sync::Arc;
#[path = "actor_motion_attach_intent.rs"]
mod attach_intent;
#[path = "actor_motion_body_spin.rs"]
pub(crate) mod body_spin;
#[path = "actor_motion_break_out_of_push.rs"]
mod break_out_of_push;
#[path = "actor_motion_compute_first_push.rs"]
pub(crate) mod compute_first_push;
#[path = "actor_motion_compute_repush_deadline.rs"]
pub(crate) mod compute_repush_deadline;
#[path = "actor_motion_compute_target_coefs.rs"]
pub(crate) mod compute_target_coefs;
#[path = "actor_motion_create_attribute.rs"]
pub(crate) mod create_attribute;
#[path = "actor_motion_crouching.rs"]
pub(crate) mod crouching;
#[path = "actor_motion_expiry.rs"]
mod expiry;
#[path = "actor_motion_finger_flip_out.rs"]
pub(crate) mod finger_flip_out;
#[path = "actor_motion_grab_callbacks.rs"]
pub(crate) mod grab_callbacks;
#[path = "actor_motion_grinding.rs"]
pub(crate) mod grinding;
#[path = "actor_motion_grounded_timers.rs"]
mod grounded_timers;
#[path = "actor_motion_kickturn_steering.rs"]
pub(crate) mod kickturn_steering;
#[path = "actor_motion_landing_callbacks.rs"]
pub(crate) mod landing_callbacks;
#[path = "actor_motion_landing_offboard_air.rs"]
pub(crate) mod landing_offboard_air;
#[path = "actor_motion_mongo_push_condition.rs"]
mod mongo_push_condition;
#[path = "actor_motion_neutral_lifecycle.rs"]
pub(crate) mod neutral_lifecycle;
#[path = "../generated_motion_runtime_play_animation.rs"]
pub(crate) mod play_animation;
#[path = "actor_motion_power_sliding.rs"]
pub(crate) mod power_sliding;
#[path = "actor_motion_pumping.rs"]
pub(crate) mod pumping;
#[path = "actor_motion_push_cycle.rs"]
pub(crate) mod push_cycle;
#[path = "actor_motion_push_out.rs"]
mod push_out;
#[path = "actor_motion_random_int.rs"]
mod random_int;
#[path = "../generated_motion_runtime_roots.rs"]
mod roots;
#[path = "actor_motion_set_push_coefs.rs"]
pub(crate) mod set_push_coefs;
#[path = "actor_motion_set_trick_height.rs"]
pub(crate) mod set_trick_height;
#[path = "actor_motion_set_turning.rs"]
pub(crate) mod set_turning;
#[path = "actor_motion_setting_body_tilt.rs"]
pub(crate) mod setting_body_tilt;
#[path = "actor_motion_runtime_skitch_conditions.rs"]
pub(crate) mod skitch_conditions;
#[path = "actor_motion_skitch_shimmy.rs"]
mod skitch_shimmy;
#[path = "actor_motion_skitching.rs"]
pub(crate) mod skitching;
#[path = "actor_motion_standing_jump.rs"]
pub(crate) mod standing_jump;
#[path = "actor_motion_store_landing_data.rs"]
pub(crate) mod store_landing_data;

impl ActorConstruction {
    /// Motion `IsGrinding`82caa310 shares the native owner reads with the
    /// Action condition, after validating this Motion controller context.
    pub(crate) fn motion_runtime_is_grinding(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        specific_name: Option<&crate::play_animation::FastString30>,
    ) -> Result<bool, MotionUpdateError> {
        self.check_motion_runtime_context(context)?;
        self.read_is_grinding_for_condition(specific_name)
            .map_err(MotionUpdateError::GrindCondition)
    }

    /// `IsBodyFlipping`82c9cb40: source Air+1a9, after physical identity checks.
    pub(crate) fn motion_runtime_is_body_flipping(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        Ok(physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .is_flipping)
    }
    /// `IsRidingGoofy`82c9b308: two reads, regular stance+9a then switch+99.
    pub(crate) fn motion_runtime_is_riding_goofy(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let regular = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_animation_output()
            .map_err(E::PhysOut)?
            .is_regular_stance;
        let switched = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_animation_output()
            .map_err(E::PhysOut)?
            .is_riding_switch;
        Ok(regular == switched)
    }
    /// `TimeToLand`82c9cc40: validity+1a5 before a fresh collision-time+a8 read.
    pub(crate) fn runtime_time_to_land(
        &self,
        comparison: crate::generated_action_load::construct::Comparison,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        if !physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .is_traj_valid
        {
            return Ok(false);
        }
        let time = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .traj_collision_time;
        Ok(crate::generated_action_load::conditions::compare(
            comparison, time,
        ))
    }

    #[cfg(test)]
    pub(crate) fn check_grounded_motion_selection(&self) {
        let controller = self
            .prefix()
            .unwrap()
            .graphs
            .as_ref()
            .unwrap()
            .motion()
            .unwrap()
            .controller
            .as_ref()
            .unwrap()
            .generated_motion()
            .unwrap();
        let retained = controller.lock().unwrap();
        assert_eq!(
            retained.current_state, 1170,
            "actual native grounded selector result"
        );
        assert_eq!(retained.current_behaviours.len(), 35);
    }

    /// Actor phase1 call825f43a0 immediately precedes Motion Update.
    pub(crate) fn try_clear_motion_attributes(&self) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let mut actor = self.prefix().map_err(E::Actor)?;
        actor
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.state.as_mut())
            .ok_or(E::MissingOwner("Motion state"))?
            .clear_attributes();
        Ok(())
    }

    pub(crate) fn try_update_motion(
        &self,
        dt: f32,
        writer: &mut dyn neutral_lifecycle::ForcePhysicsWriter,
    ) -> Result<(), MotionUpdateError> {
        use crate::{
            dynamic_state_graph_controller::DynamicStateGraphContext,
            generated_motion_load::MotionAllocation, state_graph_context::StateGraphContextHandle,
        };
        use MotionUpdateError as E;
        let (controller, mut context) = {
            let actor = self.prefix().map_err(E::Actor)?;
            let wrapper = actor
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .and_then(|g| g.controller.as_ref())
                .ok_or(E::MissingOwner("Motion controller"))?;
            (
                wrapper
                    .generated_motion()
                    .ok_or(E::MissingOwner("generated Motion controller"))?,
                wrapper.context().copy_for_behaviour(),
            )
        };
        self.check_motion_runtime_context(&context)?;
        let mut retained = controller.lock().map_err(|_| E::Lock)?;
        {
            let actor = self.prefix().map_err(E::Actor)?;
            let current = actor
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .and_then(|g| g.controller.as_ref())
                .and_then(
                    crate::actor_graph_construction::ActorHierarchicalWrapper::generated_motion,
                )
                .ok_or(E::MissingOwner("generated Motion controller"))?;
            if !Arc::ptr_eq(&current, &controller) {
                return Err(E::ForeignOwner("retained Motion controller"));
            }
        }
        let updated = retained.try_update(
            self,
            &mut context,
            dt,
            &mut |_, _| MotionAllocation::Host,
            writer,
        );
        let written = (|| {
            let mut actor = self.prefix().map_err(E::Actor)?;
            let wrapper = actor
                .graphs
                .as_mut()
                .and_then(|g| g.motion_mut())
                .and_then(|g| g.controller.as_mut())
                .ok_or(E::MissingOwner("Motion controller"))?;
            if !wrapper
                .generated_motion()
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &controller))
                || context.controller() != Some(wrapper.controller_identity())
            {
                return Err(E::ForeignOwner("Motion controller writeback"));
            }
            wrapper.set_runtime_instance(
                context
                    .instance()
                    .map(StateGraphContextHandle::shallow_copy),
            );
            Ok(())
        })();
        match (updated, written) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(update), Err(writeback)) => Err(E::UpdateAndWriteback {
                update: Box::new(update),
                writeback: Box::new(writeback),
            }),
        }
    }

    pub(crate) fn check_motion_runtime_context(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(E::ForeignOwner("IActor"));
        }
        let actor = self.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|g| actor.motion_graph_published && g.is_complete())
            .ok_or(E::MissingOwner("Motion graph"))?;
        for index in [1, 2] {
            if !actor.graph_interfaces[index]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, motion.occurrence()))
            {
                return Err(E::ForeignOwner("Motion graph"));
            }
        }
        let wrapper = motion
            .controller
            .as_ref()
            .ok_or(E::MissingOwner("Motion controller"))?;
        if context.controller() != Some(wrapper.controller_identity()) {
            return Err(E::ForeignOwner("Motion controller"));
        }
        Ok(())
    }

    /// `HasIntent` consumes the actual separate Motion map, including presence.
    pub(crate) fn motion_runtime_intent_value(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: &FastString36,
    ) -> Result<Option<f32>, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let actor = self.prefix().map_err(E::Actor)?;
        Ok(actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .and_then(|g| g.intents.as_ref())
            .ok_or(E::MissingOwner("Motion intents"))?
            .get(name))
    }

    /// `PlayAnimation::UpdateAttributes82cc9564`: ISkaterMotionGraph's
    /// filtered-intent getter, followed by `Intents::HasIntent82cc9570`.
    pub(crate) fn motion_runtime_filtered_intent_value(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: &FastString36,
    ) -> Result<Option<f32>, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let actor = self.prefix().map_err(E::Actor)?;
        Ok(actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .and_then(|g| g.filtered_intents.as_ref())
            .ok_or(E::MissingOwner("Filtered Motion intents"))?
            .get(name))
    }

    /// `IsDoneWipingOut82c9ca40`: actual Skeleton output +243.
    pub(crate) fn motion_runtime_done_wiping_out(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        Ok(physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .is_wipeout_over)
    }

    /// `WipeoutTimeToLand::ActivationFunc82c9d2c8`: actual Animation output +90.
    pub(crate) fn motion_runtime_wipeout_time_to_land(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        comparison: crate::generated_action_load::construct::Comparison,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let value = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_animation_output()
            .map_err(E::PhysOut)?
            .wipeout_estimated_time_to_land;
        Ok(crate::generated_action_load::conditions::compare(
            comparison, value,
        ))
    }

    /// `OBTimeToLand::ActivationFunc82c9aff0`: actual OffBoard output +1c,
    /// with no air-trajectory validity gate (unlike `TimeToLand`).
    pub(crate) fn motion_runtime_offboard_time_to_land(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        comparison: crate::generated_action_load::construct::Comparison,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let value = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .time_to_land;
        Ok(crate::generated_action_load::conditions::compare(
            comparison, value,
        ))
    }

    /// `ShouldPrepareOneFootAirForFootplant::ActivationFunc`82ca7700: Air output
    /// +c0. A negative time returns before the threshold read; unordered values
    /// fail the ordered threshold compare.
    pub(crate) fn motion_should_prepare_one_foot_air_for_footplant(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        cvars: &InputCvarReferences,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let time = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .time_to_land_foot_plant;
        if time < 0.0 {
            return Ok(false);
        }
        let threshold = cvars
            .try_foot_plant_time_to_land_threshold()
            .map_err(E::Cvar)?;
        Ok(time <= threshold)
    }

    /// `IsFootPlanting::ActivationFunc`82ca7830: Air output +1b0, then a fresh
    /// +c4 total absorb time through `fcmpu; bge`: only an ordered negative
    /// fails, so an unordered time passes.
    pub(crate) fn motion_runtime_is_foot_planting(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        if !physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .is_foot_planting
        {
            return Ok(false);
        }
        let absorb = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .foot_plant_total_absorb_time;
        Ok(absorb >= 0.0 || absorb.is_nan())
    }

    /// `FootPlantAbsorb::Update`82cb4f18: IPhysical and IAnimatable are both
    /// required; Air output +c4 goes to "absorblength" through `AddSettableAtt`
    /// (vtable +0x48) with normalized false and sequence -1.
    pub(crate) fn motion_foot_plant_absorb_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let absorb = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .foot_plant_total_absorb_time;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(
                crate::play_animation::FastString30::encode("absorblength")
                    .expect("source literal fits FastString30"),
                absorb,
                false,
                -1,
            )
            .map_err(E::AnimationWrite)
    }

    /// `PhysFilteredState::ActivationFunc`, actual cached physical occurrence.
    pub(crate) fn motion_runtime_filtered_state(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<crate::physics::FilteredPhysicsState, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_filtered_output()
            .map_err(E::PhysOut)?
            .filtered_physics_state)
    }

    /// `PhysicsWantsRunout::ActivationFunc`82c99f98: state first, live debug next.
    pub(crate) fn motion_runtime_requests_runout(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        cvars: &InputCvarReferences,
    ) -> Result<bool, MotionUpdateError> {
        self.check_motion_runtime_context(context)?;
        self.runtime_requests_runout(cvars)
    }

    /// Shared physical query after the caller validates its actual graph context.
    pub(crate) fn runtime_requests_runout(
        &self,
        cvars: &InputCvarReferences,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let value = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_state_output()
            .map_err(E::PhysOut)?
            .physics_requests_runout;
        if cvars.try_debug_wipeout_conditions().map_err(E::Cvar)? {
            return Err(E::MissingOwner(
                "DebugOverlay singleton for PhysicsWantsRunout",
            ));
        }
        Ok(value)
    }

    /// `IsPhysicsWiping::ActivationFunc`82c99bd8: State32 before live debug.
    pub(crate) fn motion_runtime_is_physics_wiping(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        cvars: &InputCvarReferences,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let wiping = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_state_output()
            .map_err(E::PhysOut)?
            .is_wiping_out;
        self.motion_runtime_wipeout_debug(cvars)?;
        Ok(wiping)
    }

    /// `PhysicsWantsWipeOut::ActivationFunc`82c99d58: two separate `GetPhysOut` calls.
    pub(crate) fn motion_runtime_wants_wipeout(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        cvars: &InputCvarReferences,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let wants = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_state_output()
            .map_err(E::PhysOut)?
            .phys_state_wants_wipeout;
        let requests = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_state_output()
            .map_err(E::PhysOut)?
            .physics_requests_wipeout;
        self.motion_runtime_wipeout_debug(cvars)?;
        Ok(wants || requests)
    }

    /// Both native debug paths retain one singleton lookup; drawing calls are stripped.
    fn motion_runtime_wipeout_debug(
        &self,
        cvars: &InputCvarReferences,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        if cvars.try_debug_wipeout_conditions().map_err(E::Cvar)? {
            let services = self.action_runtime_services().map_err(E::Actor)?;
            let _ = services
                .as_ref()
                .ok_or(E::MissingOwner("wipeout debug services"))?
                .overlay
                .require()
                .map_err(|error| match error {
                    crate::debug_overlay::DebugOverlayError::Poisoned => E::Lock,
                    _ => E::MissingOwner("DebugOverlay singleton"),
                })?;
        }
        Ok(())
    }

    /// `IsHandPlanting::ActivationFunc`82c9abf0 reads Air before State.
    pub(crate) fn motion_runtime_handplant(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        state: HandPlantState,
        direction: HandPlantDirection,
    ) -> Result<bool, MotionUpdateError> {
        self.check_motion_runtime_context(context)?;
        self.runtime_handplant(state, direction)
    }

    /// Shared physical reads from `IsHandPlanting::ActivationFunc`82c9abf0.
    pub(crate) fn runtime_handplant(
        &self,
        state: HandPlantState,
        direction: HandPlantDirection,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let flags = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_air_output()
            .map_err(E::PhysOut)?
            .anim_hand_plant_info_struct
            .flags;
        let active = match state {
            HandPlantState::Air => {
                physical
                    .owner()
                    .phys_out()
                    .lock()
                    .map_err(|_| E::Lock)?
                    .try_state_output()
                    .map_err(E::PhysOut)?
                    .physics_state
                    == PHYSICS_STATE_HANDPLANT
            }
            HandPlantState::Ground => flags & 0x8000_0000 != 0,
        };
        Ok(active
            && match direction {
                HandPlantDirection::None => true,
                HandPlantDirection::Frontside => flags & 0x2000_0000 != 0,
                HandPlantDirection::Backside => flags & 0x2000_0000 == 0,
            })
    }

    /// `UpdateTimeSinceTeleport`82ca5148, Reset825fe138/Update825fe148.
    pub(crate) fn motion_runtime_update_teleport_time(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        dt: f32,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let filtered = self.motion_runtime_filtered_state(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.state.as_mut())
            .ok_or(E::MissingOwner("Motion state"))?;
        state.time_since_teleport = if filtered == crate::physics::FilteredPhysicsState::Teleport {
            0.0
        } else {
            dt + state.time_since_teleport
        };
        Ok(())
    }
}

impl ActorConstruction {
    /// CanEnterSlide.ActivationFunc82c9c740; actual Motion data is captured
    /// before animation fakie is read, and only the selected slide flag is read.
    pub(crate) fn motion_runtime_can_enter_slide(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        right: bool,
    ) -> Result<bool, MotionUpdateError> {
        use crate::play_animation_lifecycle::{
            PlayAnimationInterfaceKind as I, PlayAnimationInterfaceResolution as R,
            PlayAnimationLifecycleBackend,
        };
        use MotionUpdateError as E;
        let mut source = play_animation::PlayAnimationActor {
            actor: self,
            context,
        };
        if source.resolve_interface(I::SkaterAnim)? != R::Resolved {
            return Err(E::ForeignOwner("CanEnterSlide ISkaterAnim"));
        }
        let actor = self.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(E::MissingOwner("CanEnterSlide ISkaterMotionGraph"))?;
        if !actor.motion_graph_published {
            return Err(E::MissingOwner("CanEnterSlide published Motion"));
        }
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|v| Arc::ptr_eq(v, motion.occurrence()))
        {
            return Err(E::ForeignOwner("CanEnterSlide ISkaterMotionGraph"));
        }
        let data = motion
            .state
            .as_ref()
            .ok_or(E::MissingOwner("CanEnterSlide Motion data"))?;
        let animation = actor
            .animation
            .as_ref()
            .ok_or(E::MissingOwner("CanEnterSlide animation"))?
            .lock()
            .map_err(|_| E::Lock)?;
        let player = animation
            .player()
            .ok_or(E::MissingOwner("CanEnterSlide animation player"))?;
        if player.flags_known_mask & 0x2000_0000 == 0 {
            return Err(E::MissingOwner("CanEnterSlide riding fakie flag"));
        }
        slide_entry_flag(
            &data.slide_bits,
            right,
            player.flags_bits & 0x2000_0000 != 0,
        )
    }
}
fn slide_entry_flag(
    flags: &[crate::actor_graph_construction::ActorGraphKnownBits; 2],
    right: bool,
    fakie: bool,
) -> Result<bool, MotionUpdateError> {
    let selected = &flags[usize::from(right ^ fakie)];
    if selected.known_mask & 0x8000_0000 == 0 {
        return Err(MotionUpdateError::MissingOwner(
            "CanEnterSlide selected slide flag",
        ));
    }
    Ok(selected.bits & 0x8000_0000 != 0)
}
#[cfg(test)]
mod slide_entry_tests {
    use super::*;
    use crate::actor_graph_construction::ActorGraphKnownBits as Bits;
    #[test]
    fn fakie_swaps_selected_flag_without_reading_unknown_opposite_side() {
        for right in [false, true] {
            for fakie in [false, true] {
                let mut flags = [Bits {
                    known_mask: 0,
                    bits: 0,
                }; 2];
                let side = usize::from(right ^ fakie);
                for expected in [false, true] {
                    flags[side] = Bits {
                        known_mask: 0x8000_0000,
                        bits: if expected { 0x8000_0000 } else { 0x4000_0000 },
                    };
                    assert_eq!(slide_entry_flag(&flags, right, fakie).unwrap(), expected);
                }
                assert!(slide_entry_flag(&flags, !right, fakie).is_err());
            }
        }
    }
}

#[path = "actor_motion_runtime_bump_conditions.rs"]
pub(crate) mod bump_conditions;
