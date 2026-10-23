//! Retained `Actor` listener825f9f28, ordered input events and continuous Fill.
//! Uses the same graph issuance, player and Derived input owner.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected listener awaits native Actor construction and graph scheduling"
    )
)]

use crate::physics_state_ids::{PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_SKITCHING};
use crate::{
    game_input_player::{GameInputPlayer, recognizer::GameInputPatternMessage},
    input_pattern::InputPattern,
    motion_graph_entity_authority::{ActorGraphAuthority, MotionGraphEntityAuthority},
};
use std::sync::{Arc, Mutex};

#[path = "action_graph_input_fill.rs"]
pub(crate) mod fill;

#[path = "devices/derived_controller_input.rs"]
mod derived;
pub(crate) use derived::INPUT_LISTENER_CLASS;
pub(crate) use derived::{DerivedControllerInput, DerivedInputAttributes, DerivedInputError};

#[derive(Debug)]
pub(crate) enum ActionInputDerivedError<N> {
    State(ActionInputListenerError),
    Update(DerivedInputError<crate::game_input_player::GameInputPlayerError, N>),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActionInputListenerError {
    GameplayConstructionRequired,
    PlayerLock,
    ConstructionRequired,
    ConstructionAlreadyAttempted,
    MissingPattern,
    Allocation,
}

/// Native event retains name/speed, not the match's error/time/input pointer.
#[derive(Clone, Debug)]
pub(crate) struct ActionInputPatternEvent {
    pub(crate) pattern: Arc<InputPattern>,
    pub(crate) speed: f32,
}
#[derive(Clone, Debug)]
pub(crate) enum ActionInputListenerEvent {
    Input(crate::game_input_player::recognizer::GameInputEvent),
    Pattern(ActionInputPatternEvent),
}
impl ActionInputListenerEvent {
    pub(crate) fn pattern(&self) -> Option<&ActionInputPatternEvent> {
        match self {
            Self::Pattern(pattern) => Some(pattern),
            Self::Input(_) => None,
        }
    }
}

pub(crate) trait ActionInputIntentSink {
    type Error;
    /// Native unique-key insertion, not unconditional replacement.
    fn insert(&mut self, name: &str, value: f32) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActionInputSuffixError<E> {
    State(ActionInputListenerError),
    Insert(E),
}

pub(crate) struct ActionGraphInputListener<K> {
    actor: MotionGraphEntityAuthority<K>,
    player: Arc<Mutex<GameInputPlayer>>,
    player_index: u32,
    attempted: bool,
    constructed: bool,
    events: Vec<ActionInputListenerEvent>,
    candidate: Option<Arc<InputPattern>>,
    held: bool,
    off_board_stick: fill::OffBoardStickLatch,
    derived: Option<DerivedControllerInput>,
    pub(crate) registration:
        Option<crate::game_input_player::recognizer::event_route::GameInputListenerRegistration>,
}

impl<K> ActionGraphInputListener<K> {
    /// Storage around existing owners; native listener construction is staged
    /// separately by the route's three registrations.
    pub(crate) fn new(
        actor: &ActorGraphAuthority<K>,
        player: &Arc<Mutex<GameInputPlayer>>,
    ) -> Result<Self, ActionInputListenerError> {
        let player_index = {
            let player = player
                .lock()
                .map_err(|_| ActionInputListenerError::PlayerLock)?;
            if !player.selected_construction_complete() {
                return Err(ActionInputListenerError::GameplayConstructionRequired);
            }
            player.player_index()
        };
        Ok(Self {
            actor: actor.entity_handle(),
            player: Arc::clone(player),
            player_index,
            attempted: false,
            constructed: false,
            events: Vec::new(),
            candidate: None,
            held: false,
            off_board_stick: fill::OffBoardStickLatch::default(),
            derived: None,
            registration: None,
        })
    }
    pub(crate) fn derived(&self) -> Option<&DerivedControllerInput> {
        self.derived.as_ref()
    }
    /// Selected Fill prefix `0x825fb54c..b5b0` using the same command-owned
    /// player. The caller supplies the actual `PhysOut` state sampled at b594;
    /// this does not execute subsequent continuous-control intent generation.
    pub(crate) fn try_update_derived<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        dt: f32,
        physics_state: u32,
        attributes: Option<&DerivedInputAttributes>,
        numeric: &mut N,
    ) -> Result<(), ActionInputDerivedError<N::Error>> {
        if !self.constructed {
            return Err(ActionInputDerivedError::State(
                ActionInputListenerError::ConstructionRequired,
            ));
        }
        let mut player = self
            .player
            .lock()
            .map_err(|_| ActionInputDerivedError::State(ActionInputListenerError::PlayerLock))?;
        self.derived
            .as_mut()
            .expect("inline derived constructor precedes registrations")
            .try_update(
                &mut *player,
                dt,
                crate::controller_input::ControllerInputActivity {
                    pushing: physics_state == PHYSICS_STATE_OFFBOARD_PUSHING.cast_unsigned(),
                    skitching: physics_state == PHYSICS_STATE_SKITCHING.cast_unsigned(),
                },
                attributes,
                numeric,
            )
            .map_err(ActionInputDerivedError::Update)
    }
    pub(crate) fn actor(&self) -> &MotionGraphEntityAuthority<K> {
        &self.actor
    }
    pub(crate) fn player(&self) -> &Arc<Mutex<GameInputPlayer>> {
        &self.player
    }
    pub(crate) fn events(&self) -> &[ActionInputListenerEvent] {
        &self.events
    }
    pub(crate) const fn held_pending(&self) -> bool {
        self.held
    }
    pub(crate) const fn is_constructed(&self) -> bool {
        self.constructed
    }
    pub(crate) fn registration(
        &self,
    ) -> Option<&crate::game_input_player::recognizer::event_route::GameInputListenerRegistration>
    {
        self.registration.as_ref()
    }
    pub(crate) fn begin_construction(&mut self) -> Result<(), ActionInputListenerError> {
        if self.attempted {
            return Err(ActionInputListenerError::ConstructionAlreadyAttempted);
        }
        self.attempted = true;
        self.derived = Some(DerivedControllerInput::new());
        // Actual inline event capacity20; selected heap representation reserves
        // that capacity without claiming native allocation/layout.
        self.events
            .try_reserve(20)
            .map_err(|_| ActionInputListenerError::Allocation)?;
        self.off_board_stick = fill::OffBoardStickLatch::default();
        self.candidate = None;
        self.held = false;
        Ok(())
    }
    pub(crate) fn finish_construction(&mut self) {
        self.constructed = true;
    }
    pub(crate) fn receive(
        &mut self,
        message: &GameInputPatternMessage,
    ) -> Result<(), ActionInputListenerError> {
        // Earlier handlers can exist after a late constructor refusal. Native
        // callback state was already initialized; do not gate on all-three ready.
        if !self.attempted {
            return Err(ActionInputListenerError::ConstructionRequired);
        }
        match message {
            GameInputPatternMessage::InputEvent {
                event,
                player_index,
                ..
            } => {
                if *player_index != self.player_index {
                    return Ok(());
                }
                // ReceiveInputEventMsg825f9408 appends only event identity;
                // native unused speed lane is not fabricated as a pattern.
                self.events
                    .try_reserve(1)
                    .map_err(|_| ActionInputListenerError::Allocation)?;
                self.events.push(ActionInputListenerEvent::Input(*event));
            }
            GameInputPatternMessage::Match {
                matched,
                player_index,
                ..
            } => {
                if *player_index != self.player_index {
                    return Ok(());
                }
                let pattern = matched
                    .pattern
                    .as_ref()
                    .ok_or(ActionInputListenerError::MissingPattern)?;
                self.events
                    .try_reserve(1)
                    .map_err(|_| ActionInputListenerError::Allocation)?;
                self.events
                    .push(ActionInputListenerEvent::Pattern(ActionInputPatternEvent {
                        pattern: Arc::clone(pattern),
                        speed: matched.normalized_speed,
                    }));
                // Native compares names after append; original occurrence is
                // retained for lifetime, but equality is deliberately by name.
                if ["Kickflip", "Heelflip", "N_Kickflip", "N_Heelflip"]
                    .iter()
                    .any(|name| pattern.name().eq_ignore_ascii_case(name))
                {
                    self.candidate = Some(Arc::clone(pattern));
                }
            }
            GameInputPatternMessage::Held {
                pattern,
                player_index,
            } => {
                if *player_index == self.player_index
                    && self.candidate.as_ref().is_some_and(|candidate| {
                        pattern.name().eq_ignore_ascii_case(candidate.name())
                    })
                {
                    self.held = true;
                }
            }
        }
        Ok(())
    }
    /// Explicit selected `Fill` suffix825fd3c0..825fd5e4 only. The caller must
    /// schedule the preceding stages separately; this issues no frame token.
    /// A failed insertion retains earlier sink writes and the source latch.
    pub(crate) fn try_pattern_intent_suffix<S: ActionInputIntentSink>(
        &mut self,
        sink: &mut S,
    ) -> Result<(), ActionInputSuffixError<S::Error>> {
        if !self.constructed {
            return Err(ActionInputSuffixError::State(
                ActionInputListenerError::ConstructionRequired,
            ));
        }
        if self.held {
            sink.insert("HoldPattern", 1.0)
                .map_err(ActionInputSuffixError::Insert)?;
            self.held = false;
        }
        for event in &self.events {
            let event = match event {
                ActionInputListenerEvent::Input(event) => {
                    sink.insert(event.name(), 1.0)
                        .map_err(ActionInputSuffixError::Insert)?;
                    continue;
                }
                ActionInputListenerEvent::Pattern(event) => event,
            };
            sink.insert("Trick", 1.0)
                .map_err(ActionInputSuffixError::Insert)?;
            sink.insert(event.pattern.name(), 1.0)
                .map_err(ActionInputSuffixError::Insert)?;
            sink.insert("GestureSpeed", event.speed)
                .map_err(ActionInputSuffixError::Insert)?;
        }
        Ok(())
    }
    /// Separate `Actor::GenerateActionGraphIntents`825fdabc publication boundary.
    /// It does not clear the held latch (including when source skips `Fill`).
    pub(crate) fn clear_events_after_generate(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
#[path = "action_graph_input_listener_tests.rs"]
pub(crate) mod tests;

impl<N: std::fmt::Debug> std::fmt::Display for ActionInputDerivedError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(error) => write!(f, "listener state: {error:?}"),
            Self::Update(error) => write!(f, "{error}"),
        }
    }
}
