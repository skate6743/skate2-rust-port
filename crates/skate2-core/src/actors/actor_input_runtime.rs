//! Runtime input Fill from the same cached physical owner and retained globals.
use super::{ActorConstruction, ActorInputGenerationError, ActorInputPhysics, ReckoningNumeric};
use crate::{
    action_graph_input_listener::fill::ActionInputFillAttributes,
    actor_construction::physical_mount::{ActorPhysicalAllocation, ActorPhysicalMountError},
    input_cvar_runtime::{InputCvarError, InputCvarProvider},
    juice_trick_input::{JuiceTrickInput, JuiceTrickInputError},
    physical_player_phys_out::construct::ConcretePhysOutError,
};
use std::{convert::Infallible, sync::Arc};

/// Shared startup collaborators, outside the Actor. In particular Juice's
/// native globals must be moved/shared from startup, never recreated per tick.
pub(crate) struct ActorInputRuntimeServices {
    pub(crate) attributes: ActionInputFillAttributes,
    pub(crate) cvars: InputCvarProvider,
    pub(crate) juice: JuiceTrickInput,
}
#[derive(Debug)]
pub(crate) enum ActorInputPhysicsError {
    Owner(Arc<ActorPhysicalMountError>),
    Output(ConcretePhysOutError<Infallible>),
    Lock,
}
impl std::fmt::Display for ActorInputPhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "input physical owner: {error}"),
            Self::Output(error) => write!(f, "input physical output: {error:?}"),
            Self::Lock => f.write_str("input physical output lock poisoned"),
        }
    }
}

struct PhysicalInput(Result<Arc<ActorPhysicalAllocation>, Arc<ActorPhysicalMountError>>);

#[test]
fn absent_physical_owner_is_required_only_when_source_queries_it() {
    use super::tests::Unused;
    use crate::actor_graph_construction::ActorGraphIntents;
    for flags in [0x0400_0000, 0x8400_0000, 0] {
        let (actor, _route) = super::super::tests::actor_with_listener();
        actor.try_construct_graphs().unwrap();
        actor.prefix().unwrap().input_generation_flags = Some(flags);
        let mut physical = PhysicalInput(actor.cached_physical_allocation().map_err(Arc::new));
        let result = actor.try_generate_action_intents(
            0.25,
            None,
            &mut Unused,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &mut Unused,
            &mut physical,
        );
        let state = actor.prefix().unwrap();
        let map = state
            .graphs()
            .unwrap()
            .action()
            .unwrap()
            .intents
            .as_ref()
            .unwrap();
        if flags == 0x0400_0000 {
            result.unwrap();
            assert_eq!(
                map.get(&ActorGraphIntents::native_name(b"ForceBrake"))
                    .map(f32::to_bits),
                Some(1.0_f32.to_bits())
            );
        } else {
            assert!(matches!(
                result,
                Err(ActorInputGenerationError::Physics(
                    ActorInputPhysicsError::Owner(_)
                ))
            ));
            assert_eq!(map.len(), 0);
            assert_eq!(state.input_generation_flags, Some(flags));
        }
    }
}
impl ActorInputPhysics for PhysicalInput {
    type Error = ActorInputPhysicsError;
    fn physics_state(&mut self) -> Result<u32, Self::Error> {
        let output = self
            .0
            .as_ref()
            .map_err(|error| ActorInputPhysicsError::Owner(Arc::clone(error)))?
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| ActorInputPhysicsError::Lock)?;
        Ok(u32::from_ne_bytes(
            output
                .try_state_output()
                .map_err(ActorInputPhysicsError::Output)?
                .physics_state
                .to_ne_bytes(),
        ))
    }
    fn effective_anim_z(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        let output = self
            .0
            .as_ref()
            .map_err(|error| ActorInputPhysicsError::Owner(Arc::clone(error)))?
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| ActorInputPhysicsError::Lock)?;
        Ok(output
            .try_skeleton_output()
            .map_err(ActorInputPhysicsError::Output)?
            .effective_anim_z)
    }
    fn is_wiping_out(&mut self) -> Result<bool, Self::Error> {
        let output = self
            .0
            .as_ref()
            .map_err(|error| ActorInputPhysicsError::Owner(Arc::clone(error)))?
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| ActorInputPhysicsError::Lock)?;
        Ok(output
            .try_state_output()
            .map_err(ActorInputPhysicsError::Output)?
            .is_wiping_out)
    }
}
pub(crate) type ActorInputRuntimeError<N> =
    ActorInputGenerationError<N, InputCvarError, JuiceTrickInputError, ActorInputPhysicsError>;
impl ActorConstruction {
    /// `GenerateActionGraphIntents` at 825fda28 uses lazy physical reads during Fill
    /// and the pending-wipeout override. Capture identity before the Actor lock;
    /// defer any owner error and physical payload read until its source branch.
    pub(crate) fn try_generate_runtime_action_intents<N: ReckoningNumeric>(
        &self,
        dt: f32,
        services: &mut ActorInputRuntimeServices,
        numeric: &mut N,
    ) -> Result<(), ActorInputRuntimeError<N::Error>> {
        let physical = self.cached_physical_allocation().map_err(Arc::new);
        self.try_generate_action_intents(
            dt,
            Some(&services.attributes),
            &mut services.cvars,
            numeric,
            &mut services.juice,
            &mut PhysicalInput(physical),
        )
    }
}
