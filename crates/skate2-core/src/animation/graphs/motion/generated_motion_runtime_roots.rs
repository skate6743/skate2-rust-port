//! Root Motion callbacks on current physical, animation and Motion owners.
use super::{
    ActorConstruction, ActorIdentity, MotionUpdateError, play_animation::PlayAnimationActor,
};
use crate::{
    generated_motion_load::construct::MotionBehaviourPayload,
    motion_graph_entity_authority::BoundMotionGraphContext,
    physics::{FilteredPhysicsState, GrindType},
    play_animation::FastString36,
    play_animation_lifecycle::{
        PlayAnimationInterfaceKind as Interface, PlayAnimationLifecycleBackend,
    },
};
use std::{cmp::Ordering, sync::Arc};

fn require_interface(
    backend: &mut PlayAnimationActor<'_>,
    interface: Interface,
) -> Result<(), MotionUpdateError> {
    if backend.resolve_interface(interface)?
        != crate::play_animation_lifecycle::PlayAnimationInterfaceResolution::Resolved
    {
        return Err(MotionUpdateError::ForeignOwner("root callback interface"));
    }
    Ok(())
}

impl ActorConstruction {
    /// Update82ca2808. Compare OLD teleport timer, then increment before the
    /// conditional slide/grind queries; velocity query precedes eligibility.
    pub(crate) fn motion_runtime_update_riding_fakie(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &MotionBehaviourPayload,
        slow_time: &mut f32,
        teleport_time: &mut f32,
        dt: f32,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let MotionBehaviourPayload::UpdateRidingFakie {
            high_speed_threshold,
            low_speed_threshold,
            time_slowly_rolling_backwards_threshold,
            time_from_teleport_threshold,
        } = payload
        else {
            return Err(E::InvalidActiveSet);
        };
        let mut backend = PlayAnimationActor {
            actor: self,
            context,
        };
        for interface in [
            Interface::Physical,
            Interface::SkaterAnim,
            Interface::SkaterMotionGraph,
        ] {
            require_interface(&mut backend, interface)?;
        }
        let filtered = self.motion_runtime_filtered_state(context)?;
        let aged = if filtered == FilteredPhysicsState::Teleport {
            *teleport_time = 0.0;
            false
        } else {
            let old = *teleport_time;
            *teleport_time = old + dt;
            old > *time_from_teleport_threshold
        };
        let mut eligible = false;
        if filtered == FilteredPhysicsState::Ground {
            let actor = self.prefix().map_err(E::Actor)?;
            let flags = &actor
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .and_then(|g| g.state.as_ref())
                .ok_or(E::MissingOwner("Motion state"))?
                .flags;
            // ISkaterMotionGraph+0x64 (82ca2a88) is IsDoingTrick825fe0b8, bit27 of
            // Motion+0xca4: a trick in progress keeps the old riding-fakie answer.
            if flags.known_mask & 0x0800_0000 == 0 {
                return Err(E::MissingOwner("Motion IsDoingTrick flag"));
            }
            eligible = flags.bits & 0x0800_0000 == 0;
        }
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        if !eligible && filtered == FilteredPhysicsState::Grind {
            let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
            eligible = matches!(
                output.try_filtered_output().map_err(E::PhysOut)?.grind_type,
                GrindType::None | GrindType::Tipslide
            );
        }
        let owner = backend.animation()?;
        if !aged {
            owner
                .lock()
                .map_err(|_| E::Lock)?
                .motion_set_riding_fakie(false)?;
            *slow_time = 0.0;
            return Ok(());
        }
        let speed = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_motion_z_speed()
            .map_err(E::PhysOut)?;
        if !eligible {
            *slow_time = 0.0;
            return Ok(());
        }
        if speed < *high_speed_threshold {
            *slow_time = 0.0;
            return owner
                .lock()
                .map_err(|_| E::Lock)?
                .motion_set_riding_fakie(true);
        }
        // Raw bge82ca2b8c tests NOT LT, including unordered operands.
        if speed.partial_cmp(low_speed_threshold) == Some(Ordering::Less) {
            *slow_time += dt;
            if *slow_time > *time_slowly_rolling_backwards_threshold {
                return owner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .motion_set_riding_fakie(true);
            }
        } else {
            *slow_time = 0.0;
        }
        owner
            .lock()
            .map_err(|_| E::Lock)?
            .motion_set_riding_fakie(false)
    }

    /// Update82caf338. The first inactive callback still resolves all five
    /// source interfaces and samples manual/slide and physical/riding flags.
    pub(crate) fn motion_runtime_update_fakie_head(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        channel: &FastString36,
        torso: &mut f32,
        active: &mut bool,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        let mut backend = PlayAnimationActor {
            actor: self,
            context,
        };
        require_interface(&mut backend, Interface::Physical)?;
        require_interface(&mut backend, Interface::Animatable)?;
        let owner = backend.animation()?;
        {
            let actor = self.prefix().map_err(E::Actor)?;
            let animation = owner.lock().map_err(|_| E::Lock)?;
            if !actor.animation_interfaces[1]
                .as_ref()
                .is_some_and(|r| Arc::ptr_eq(r, animation.occurrence()))
            {
                return Err(E::ForeignOwner("IChannelAnimatable"));
            }
        }
        require_interface(&mut backend, Interface::SkaterAnim)?;
        require_interface(&mut backend, Interface::SkaterMotionGraph)?;
        let target = {
            let actor = self.prefix().map_err(E::Actor)?;
            let flags = &actor
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .and_then(|g| g.state.as_ref())
                .ok_or(E::MissingOwner("Motion state"))?
                .flags;
            if flags.known_mask & 0x1000_0000 == 0 {
                return Err(E::MissingOwner("Motion manual flag"));
            }
            if flags.bits & 0x1000_0000 != 0 {
                1.0
            } else {
                if flags.known_mask & 0x0400_0000 == 0 {
                    return Err(E::MissingOwner("Motion powerslide flag"));
                }
                if flags.bits & 0x0400_0000 != 0 {
                    0.0
                } else {
                    0.5
                }
            }
        };
        let grounded = self.motion_runtime_filtered_state(context)? == FilteredPhysicsState::Ground;
        let mut animation = owner.lock().map_err(|_| E::Lock)?;
        let player = animation
            .player()
            .ok_or(E::MissingOwner("SkaterAnim player"))?;
        if player.flags_known_mask & 0x2000_0000 == 0 {
            return Err(E::MissingOwner("riding fakie flag"));
        }
        let enabled = player.flags_bits & 0x2000_0000 != 0 && grounded;
        if !*active && enabled {
            animation
                .try_start_fakie_channel(*channel)
                .map_err(E::AnimationChannel)?;
            // Native778 commits the target immediately after TransitionTo,
            // even when its status is not Created; only then active and torso.
            *torso = target;
        }
        if *active && !enabled {
            animation
                .try_end_gesture_channel(channel)
                .map_err(E::AnimationWrite)?;
        }
        *active = enabled;
        if enabled {
            let delta = target - *torso;
            let lower = if -0.01_f32 - delta >= 0.0 {
                -0.01
            } else {
                delta
            };
            let bounded = if 0.01_f32 - lower >= 0.0 { lower } else { 0.01 };
            let updated = bounded + *torso;
            *torso = updated;
            animation
                .try_add_gesture_attribute(
                    crate::play_animation::FastString30::encode("torso").expect("source literal"),
                    *torso,
                    false,
                    -1,
                )
                .map_err(E::AnimationWrite)?;
        }
        Ok(())
    }
}
