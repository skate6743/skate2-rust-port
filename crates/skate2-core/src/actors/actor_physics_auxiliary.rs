//! Selected Actor auxiliary allocations825ff458..4ec, before submittable/jobs.
use super::{ActorConstruction, ActorConstructionError, anim_out_phys_in::AnimationAiInput};

pub(crate) struct ActorPhysicsAuxiliary {
    pub(crate) living_world_impulse: [f32; 4],
    pub(crate) ai: AnimationAiInput,
    /// Actor+214, null from825fe964 until `CreateAiController`825f40b8 attaches the
    /// session's `AI::AIController`; the Actor reads only its client type
    /// (`IsNonPlayerAISkater`825ed100).
    pub(crate) ai_controller: Option<std::sync::Arc<crate::ai::actor::AiClientType>>,
    /// The map the AI controller filled at this frame's `GenerateActionGraphIntents`.
    pub(crate) ai_intents: Option<crate::actor_graph_construction::ActorGraphIntents>,
}
impl ActorPhysicsAuxiliary {
    fn construct_in_sim_allocation() -> Self {
        // v127=zero at825feb94. Stores480..4d4 initialize all nine vectors;
        // trajectory time at4d0 is four -1 lanes. 4e0 preserves low25 flags,
        // already zero from category55 OR100, and clears the high7.
        let ai = AnimationAiInput {
            trajectory_time: [-1.0; 4],
            ..AnimationAiInput::default()
        };
        Self {
            living_world_impulse: [0.0; 4],
            ai,
            ai_controller: None,
            ai_intents: None,
        }
    }
}
#[derive(Debug)]
pub(crate) enum ActorPhysicsAuxiliaryError {
    Actor(ActorConstructionError),
    MissingPhysicalReturn,
    MissingExternalReset,
    AlreadyConstructed,
}
impl std::fmt::Display for ActorPhysicsAuxiliaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "Actor auxiliary owner: {error}"),
            Self::MissingPhysicalReturn => {
                f.write_str("Actor physical constructor has not returned")
            }
            Self::MissingExternalReset => f.write_str("Actor external reset allocation missing"),
            Self::AlreadyConstructed => {
                f.write_str("Actor physics auxiliaries already constructed")
            }
        }
    }
}
impl ActorConstruction {
    /// Selected successful Sim allocations, not a claim that intervening
    /// breadcrumbs or subsequent submittable/jobs have been reconstructed.
    pub(crate) fn try_construct_physics_auxiliary(&self) -> Result<(), ActorPhysicsAuxiliaryError> {
        let mut state = self.prefix().map_err(ActorPhysicsAuxiliaryError::Actor)?;
        if state.physical_cached.is_none() {
            return Err(ActorPhysicsAuxiliaryError::MissingPhysicalReturn);
        }
        if state.external_player_reset.is_none() {
            return Err(ActorPhysicsAuxiliaryError::MissingExternalReset);
        }
        if state.physics_auxiliary.is_some() {
            return Err(ActorPhysicsAuxiliaryError::AlreadyConstructed);
        }
        state.physics_auxiliary = Some(ActorPhysicsAuxiliary::construct_in_sim_allocation());
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sim_zero_residuals_and_explicit_negative_trajectory_time_are_distinct() {
        let owner = ActorPhysicsAuxiliary::construct_in_sim_allocation();
        assert_eq!(owner.living_world_impulse.map(f32::to_bits), [0; 4]);
        assert_eq!(owner.ai.trajectory_time.map(f32::to_bits), [0xbf80_0000; 4]);
        assert_eq!(owner.ai.desired_deck_transform.map(f32::to_bits), [0; 16]);
        assert_eq!(owner.ai.flags, 0);
        assert!(owner.ai_controller.is_none());
    }
}

impl ActorConstruction {
    /// Actor.LivingWorldWipeOut825ed130 -> ForceWipeOut825edad0. Publish to
    /// the existing animation/physics suffix; no separate bail command path.
    pub(crate) fn try_living_world_wipe_out(&self,source:[f32;4])->Result<(),String>{
        let allocation=self.cached_physical_allocation().map_err(|e|format!("LivingWorld wipeout allocation: {e:?}"))?;
        let position={
            let output=allocation.owner().phys_out().lock().map_err(|_|"LivingWorld wipeout PhysOut lock")?;
            let p=output.try_skeleton_output().map_err(|e|format!("LivingWorld wipeout position: {e:?}"))?
                .skeleton_root_matrix.columns()[3];
            [p.xyz[0],p.xyz[1],p.xyz[2],p.w.unwrap_or(0.)]
        };
        let delta=std::array::from_fn::<_,4,_>(|i|position[i]-source[i]);
        let impulse=crate::reckoning_numeric::try_normalize_safe(delta,[0.;4],
            &mut crate::reckoning_numeric::HostReckoningNumeric).map_err(|e|format!("LivingWorld wipeout direction: {e:?}"))?;
        let mut prefix=self.prefix().map_err(|e|format!("LivingWorld wipeout Actor: {e:?}"))?;
        prefix.physics_auxiliary.as_mut().ok_or("LivingWorld wipeout auxiliary absent")?.living_world_impulse=impulse;
        let flags=prefix.input_generation_flags.as_mut().ok_or("LivingWorld wipeout flags absent")?;
        *flags|=0x4000_0000;
        Ok(())
    }

    /// PC port: a Skate 3 physics shove on this Skate 2 physics skater (TU3 delivers it to the
    /// target's WipeoutGround entry, 0x82d3b838): the same LivingWorld wipeout, along the shove's
    /// velocity instead of away from a source point.
    pub(crate) fn try_shove_wipe_out(&self,velocity:[f32;4])->Result<(),String>{
        let impulse=crate::reckoning_numeric::try_normalize_safe([velocity[0],velocity[1],velocity[2],0.],[0.;4],
            &mut crate::reckoning_numeric::HostReckoningNumeric).map_err(|e|format!("shove wipeout direction: {e:?}"))?;
        let mut prefix=self.prefix().map_err(|e|format!("shove wipeout Actor: {e:?}"))?;
        prefix.physics_auxiliary.as_mut().ok_or("shove wipeout auxiliary absent")?.living_world_impulse=impulse;
        let flags=prefix.input_generation_flags.as_mut().ok_or("shove wipeout flags absent")?;
        *flags|=0x4000_0000;
        Ok(())
    }
}
