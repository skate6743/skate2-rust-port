//! SAME Actor/action-map `GenerateActionGraphIntents`, reference 0x825fda28.
use super::{
    ActorConstruction, ActorConstructionError, ActorGraphComponents, ActorGraphIntentError, Arc,
    ReckoningNumeric,
};
use crate::action_graph_input_listener::fill::{
    ActionInputFillAttributes, ActionInputFillCvars, ActionInputFillError, ActionInputJuice,
    ActionInputPhysOut,
};
use std::sync::atomic::{AtomicBool, Ordering};

// TelemetryHelper::sInitialized is zero in the debug image. The currently
// mounted Juice boot paths stop before Initialize; JLogEvent82a76fc0 returns
// without emitting when this flag is clear. A future initialized boot must
// mount the real event service before setting this host mirror true.
static JUICE_TELEMETRY_INITIALIZED: AtomicBool = AtomicBool::new(false);
/// Shared JLogEvent82a76fc0 boot gate. The selected boot has not initialized
/// telemetry; an initialized service must be mounted before this can emit.
pub(crate) fn try_juice_telemetry_gate() -> Result<(), ActorConstructionError> {
    if JUICE_TELEMETRY_INITIALIZED.load(Ordering::Relaxed) {
        Err(ActorConstructionError::TelemetryUnavailable)
    } else { Ok(()) }
}
#[path = "actor_input_runtime.rs"]
pub(crate) mod runtime;
#[cfg(test)]
#[path = "actor_input_generation_tests.rs"]
mod tests;

impl ActorConstruction {
    /// `Manager::CheckSpawnedDynamicObjectAgainstSkaters` 0x8272590c..5914:
    /// `Actor+0x258 |= 0x02000000` (`mRequestCollideWithWorldOnly`), copied
    /// into AnimOutPhysIn by the next physics publication.
    pub(crate) fn try_request_collide_with_world_only(&self) -> Result<(), ActorConstructionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActorConstructionError::Poisoned)?;
        let flags = state
            .input_generation_flags
            .ok_or(ActorConstructionError::GraphPrefixIncomplete)?;
        state.input_generation_flags = Some(flags | 0x0200_0000);
        Ok(())
    }
}

impl ActorConstruction {
    /// Direct `Actor+0x258` reads (`Manager::Update1` 0x82728298).
    pub(crate) fn try_input_generation_flags(&self) -> Result<u32, ActorConstructionError> {
        self.state
            .lock()
            .map_err(|_| ActorConstructionError::Poisoned)?
            .input_generation_flags
            .ok_or(ActorConstructionError::GraphPrefixIncomplete)
    }
}

impl crate::physical_player_physics_state::PhysicalStateActorQuery for &ActorConstruction {
    type Error = ActorConstructionError;

    fn player_id(&mut self) -> Result<i32, Self::Error> {
        // IActorQuery thunk825f9d98 subtracts34, then folded82485d18 reads Actor+1d8.
        let state = self
            .state
            .lock()
            .map_err(|_| ActorConstructionError::Poisoned)?;
        let fields = state
            .fields
            .as_ref()
            .ok_or(ActorConstructionError::ActorFieldsUninitialized)?;
        Ok(i32::from_ne_bytes(fields.player_index.to_ne_bytes()))
    }

    fn log_board_transition(&mut self, _entering_offboard: bool) -> Result<(), Self::Error> {
        try_juice_telemetry_gate()
    }
}

/// Source `PhysOut` queries occur only in the branches that use them. An actual
/// runtime implementation must read this Actor's retained physical owner.
pub(crate) trait ActorInputPhysics {
    type Error;
    fn physics_state(&mut self) -> Result<u32, Self::Error>;
    /// `PhysOut_Skeleton::mEffectiveAnimZ`, read by the 1.02 Fill.
    fn effective_anim_z(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error>;
    fn is_wiping_out(&mut self) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum ActorInputGenerationError<N, C, J, P> {
    Actor(ActorConstructionError),
    Physics(P),
    Fill(ActionInputFillError<N, C, ActorGraphIntentError, J>),
    Insert(ActorGraphIntentError),
}
impl<N: std::fmt::Debug, C: std::fmt::Debug, J: std::fmt::Debug, P: std::fmt::Debug>
    std::fmt::Display for ActorInputGenerationError<N, C, J, P>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "input Actor owner: {error}"),
            Self::Physics(error) => write!(f, "Actor input physical output: {error:?}"),
            Self::Fill(error) => write!(f, "Actor input Fill: {error:?}"),
            Self::Insert(error) => write!(f, "Actor input override: {error:?}"),
        }
    }
}

impl ActorConstruction {
    /// Native source clears the SAME map first, then takes the AI controller's
    /// map (`AIController::GenerateActionGraphIntents`, run for this Actor just
    /// before) or, unpinned, the input fill; then it clears listener events and
    /// applies pending overrides. It does not tick the Action/Motion controllers.
    #[expect(
        clippy::type_complexity,
        reason = "separate source failures retain different prefixes"
    )]
    pub(crate) fn try_generate_action_intents<N, C, J, P>(
        &self,
        dt: f32,
        attributes: Option<&ActionInputFillAttributes>,
        cvars: &mut C,
        numeric: &mut N,
        juice: &mut J,
        physics: &mut P,
    ) -> Result<(), ActorInputGenerationError<N::Error, C::Error, J::Error, P::Error>>
    where
        N: ReckoningNumeric,
        C: ActionInputFillCvars,
        J: ActionInputJuice,
        P: ActorInputPhysics,
    {
        use ActorInputGenerationError as E;
        let mut state = self.prefix().map_err(E::Actor)?;
        let listener = Arc::clone(
            state
                .listener
                .as_ref()
                .filter(|_| state.listener_published)
                .ok_or(E::Actor(ActorConstructionError::ListenerPrefixIncomplete))?,
        );
        let constructor_ai_null = state
            .fields
            .as_ref()
            .ok_or(E::Actor(ActorConstructionError::GraphPrefixIncomplete))?
            .ai_controller_null;
        let ai_null = state
            .physics_auxiliary
            .as_ref()
            .map_or(constructor_ai_null, |auxiliary| {
                auxiliary.ai_controller.is_none()
            });
        let ai_intents = state
            .physics_auxiliary
            .as_mut()
            .and_then(|auxiliary| auxiliary.ai_intents.take());
        let flags = state
            .input_generation_flags
            .ok_or(E::Actor(ActorConstructionError::GraphPrefixIncomplete))?;
        let pinned_gestures = state.pinned_gestures;
        let intents = state
            .graphs
            .as_mut()
            .and_then(ActorGraphComponents::action_mut)
            .filter(|action| action.is_complete())
            .and_then(|action| action.intents.as_mut())
            .ok_or(E::Actor(ActorConstructionError::GraphPrefixIncomplete))?;
        intents.clear();
        let pinned = flags & 0x0400_0000 != 0;
        if !ai_null {
            if let Some(ai_intents) = ai_intents {
                *intents = ai_intents;
            }
        } else if !pinned || pinned_gestures {
            let phys_out = ActionInputPhysOut {
                physics_state: physics.physics_state().map_err(E::Physics)?,
                effective_anim_z: physics.effective_anim_z().map_err(E::Physics)?,
            };
            listener
                .lock()
                .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?
                .try_fill_selected(dt, phys_out, attributes, cvars, numeric, intents, juice)
                .map_err(E::Fill)?;
            if pinned {
                // Retail online play: a skater the challenge pins (a countdown, waiting for
                // another player's turn) still gestures, and does nothing else.
                let gestures: Vec<_> = GESTURE_INTENTS
                    .iter()
                    .map(|name| crate::actor_graph_construction::ActorGraphIntents::native_name(name))
                    .filter_map(|name| intents.get(&name).map(|value| (name, value)))
                    .collect();
                intents.clear();
                for (name, value) in gestures {
                    intents.insert_or_assign(name, value).map_err(E::Insert)?;
                }
            }
        }
        listener
            .lock()
            .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?
            .clear_events_after_generate();
        let flags = state
            .input_generation_flags
            .expect("constructor flags retained");
        if flags & 0x8000_0000 != 0 && !physics.is_wiping_out().map_err(E::Physics)? {
            let intents = state
                .graphs
                .as_mut()
                .and_then(ActorGraphComponents::action_mut)
                .and_then(|action| action.intents.as_mut())
                .expect("same retained Action map");
            intents
                .insert_or_assign(
                    crate::actor_graph_construction::ActorGraphIntents::native_name(
                        b"WipeOutRequest",
                    ),
                    1.0,
                )
                .map_err(E::Insert)?;
        }
        // Native pending-bit clear precedes the possible ForceBrake insertion.
        let flags = state
            .input_generation_flags
            .expect("constructor flags retained");
        state.input_generation_flags = Some(flags & 0x7fff_ffff);
        if flags & 0x0400_0000 != 0 {
            state
                .graphs
                .as_mut()
                .and_then(ActorGraphComponents::action_mut)
                .and_then(|action| action.intents.as_mut())
                .expect("same retained Action map")
                .insert_or_assign(
                    crate::actor_graph_construction::ActorGraphIntents::native_name(b"ForceBrake"),
                    1.0,
                )
                .map_err(E::Insert)?;
        }
        Ok(())
    }
}

impl crate::physical_player_physics_state::PhysicalStateActorQuery for ActorConstruction {
    type Error = ActorConstructionError;
    fn player_id(&mut self) -> Result<i32, Self::Error> {
        crate::physical_player_physics_state::PhysicalStateActorQuery::player_id(&mut &*self)
    }
    fn log_board_transition(&mut self, value: bool) -> Result<(), Self::Error> {
        crate::physical_player_physics_state::PhysicalStateActorQuery::log_board_transition(
            &mut &*self,
            value,
        )
    }
}

/// The input fill's D-pad gestures, in its order.
const GESTURE_INTENTS: [&[u8]; 4] = [b"GestureDown", b"GestureLeft", b"GestureRight", b"GestureUp"];

impl ActorConstruction {
    /// Whether a pinned skater keeps its gestures (online play).
    pub(crate) fn try_set_pinned_gestures(&self, on: bool) -> Result<(), ActorConstructionError> {
        self.state.lock().map_err(|_| ActorConstructionError::Poisoned)?.pinned_gestures = on;
        Ok(())
    }

    /// `SkaterManager::OnCommand` 82600920 case 7: `Actor+0x258` bit `0x04000000`
    /// (the pin the input fill and `ForceBrake` read) set or cleared.
    pub(crate) fn try_set_pinned(&self, pin: bool) -> Result<(), ActorConstructionError> {
        let mut state = self.prefix()?;
        let flags = state.input_generation_flags.as_mut().ok_or(ActorConstructionError::GraphPrefixIncomplete)?;
        if pin {
            *flags |= 0x0400_0000;
        } else {
            *flags &= !0x0400_0000;
        }
        Ok(())
    }
}

impl ActorConstruction {
    /// PreventManualRespawn825ed228 writes the existing one-shot input bits.
    pub(crate) fn try_prevent_manual_respawn(&self)->Result<(),ActorConstructionError>{
        let mut state=self.prefix()?;
        let flags=state.input_generation_flags.as_mut().ok_or(ActorConstructionError::GraphPrefixIncomplete)?;
        *flags|=0x1000_0000;Ok(())
    }
}
