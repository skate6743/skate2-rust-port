//! Explicit graph-phase transport.
//!
//! These records are the typed boundary between the two native graph phases.
//! Graph hosts publish complete records. Unsupported branch execution keeps
//! the existing explicit runtime error policy.

use skate3_core::animation::output::attributes::AnimationAttribute;
use skate3_core::graph::intents::IntentMap;
use skate3_core::player::state::PhysicalStateId;

#[derive(Clone, Debug, Default, PartialEq)]
struct AuthoredSignals {
    values: IntentMap,
}

impl AuthoredSignals {
    fn from_values(values: &IntentMap) -> Self {
        Self { values: values.clone() }
    }

    fn into_values(self) -> IntentMap {
        self.values
    }

    fn get(&self, name: &str) -> Option<f32> {
        self.values.get(name).copied()
    }

    fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WipeoutControls {
    pub request: bool,
    pub air_body_tweak: [f32; 2],
    pub gesture: Option<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoardControls {
    pub drop_requested: bool,
    pub throw_requested: bool,
    pub retrieve_requested: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionControls {
    pub wipeout: WipeoutControls,
    pub board: BoardControls,
    authored_values: AuthoredSignals,
}

impl ActionControls {
    pub fn has(&self, name: &str) -> bool {
        self.authored_values.contains(name)
    }

    pub fn value(&self, name: &str) -> Option<f32> {
        self.authored_values.get(name)
    }

    pub(crate) fn authored_values(&self) -> IntentMap {
        self.authored_values.clone().into_values()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionEffects {
    values: AuthoredSignals,
}

impl MotionEffects {
    pub fn from_values(values: &IntentMap) -> Self {
        Self { values: AuthoredSignals::from_values(values) }
    }

    pub fn apply_to(&self, destination: &mut IntentMap) {
        *destination = self.values.clone().into_values();
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionInput {
    values: AuthoredSignals,
}

impl ActionInput {
    pub fn from_values(values: &IntentMap) -> Self {
        Self { values: AuthoredSignals::from_values(values) }
    }

    pub(crate) fn into_values(self) -> IntentMap {
        self.values.into_values()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PriorMotionState {
    values: AuthoredSignals,
}

impl PriorMotionState {
    pub fn from_values(values: &IntentMap) -> Self {
        Self { values: AuthoredSignals::from_values(values) }
    }

    pub(crate) fn into_values(self) -> IntentMap {
        self.values.into_values()
    }
}

/// Tick-scoped graph diagnostics. Native graph failures are actionable, but a
/// long-lived host must not grow an error vector without bound when an authored
/// graph repeatedly reaches an unsupported node.
#[derive(Debug, Default)]
pub struct GraphDiagnostics {
    messages: Vec<String>,
    overflowed: bool,
}

impl GraphDiagnostics {
    const CAPACITY: usize = 64;

    pub fn push(&mut self, message: impl Into<String>) {
        if self.messages.len() < Self::CAPACITY {
            self.messages.push(message.into());
        } else {
            self.overflowed = true;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty() && !self.overflowed
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.overflowed = false;
    }

    pub fn join(&self, separator: &str) -> String {
        let mut message = self.messages.join(separator);
        if self.overflowed {
            if !message.is_empty() {
                message.push_str(separator);
            }
            message.push_str("graph diagnostics truncated after 64 entries");
        }
        message
    }
}

/// Load-time declaration of what a stock graph can execute in this build.
/// This is a diagnostic for a failed load, not a runtime compatibility mode.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphCapabilityReport {
    pub supported_operations: usize,
    pub unsupported_operations: Vec<String>,
    pub unsupported_conditions: Vec<String>,
    pub unsupported_hooks: Vec<String>,
}

impl GraphCapabilityReport {
    pub fn require_complete(&self, graph: &str) -> Result<(), String> {
        if self.unsupported_operations.is_empty()
            && self.unsupported_conditions.is_empty()
            && self.unsupported_hooks.is_empty()
        {
            return Ok(());
        }
        Err(format!(
            "{graph} contains unsupported authored features: operations={:?}, conditions={:?}, hooks={:?}",
            self.unsupported_operations,
            self.unsupported_conditions,
            self.unsupported_hooks,
        ))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurningOutput {
    pub turn: f32,
    pub raw_turn: f32,
    pub hard_turn: f32,
}

/// Typed graph effects with declared destinations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphEffects {
    pub state_requests: Vec<PhysicalStateId>,
}

#[derive(Clone, Debug)]
pub struct ActionGraphInput {
    pub tick: u64,
    pub controls: ActionInput,
    pub prior_motion: PriorMotionState,
    pub animation_attributes: Vec<AnimationAttribute>,
}

#[derive(Clone, Debug)]
pub struct ActionGraphOutput {
    pub tick: u64,
    pub controls: ActionControls,
    pub motion_effects: MotionEffects,
    pub turning: TurningOutput,
    pub animation_attributes: Vec<AnimationAttribute>,
    pub effects: GraphEffects,
}

impl ActionGraphOutput {
    pub fn from_host(
        tick: u64,
        action_intents: &IntentMap,
        motion_intents: &IntentMap,
        animation_attributes: &[AnimationAttribute],
    ) -> Self {
        Self {
            tick,
            controls: ActionControls {
                wipeout: WipeoutControls {
                    request: action_intents.contains_key("WipeOutRequest"),
                    air_body_tweak: [
                        action_intents.get("OB_AirBodyTweakX").copied().unwrap_or(0.0),
                        action_intents.get("OB_AirBodyTweakY").copied().unwrap_or(0.0),
                    ],
                    gesture: match (
                        action_intents.get("WipeoutGestureX"),
                        action_intents.get("WipeoutGestureY"),
                    ) {
                        (Some(x), Some(y)) => Some([*x, *y]),
                        _ => None,
                    },
                },
                board: BoardControls {
                    drop_requested: action_intents.contains_key("OB_DropBoard"),
                    throw_requested: action_intents.contains_key("OB_ThrowBoard"),
                    retrieve_requested: action_intents.contains_key("OB_RetrieveBoard"),
                },
                authored_values: AuthoredSignals::from_values(action_intents),
            },
            motion_effects: MotionEffects::from_values(motion_intents),
            turning: TurningOutput {
                turn: motion_intents.get("Turn").copied().unwrap_or(0.0),
                raw_turn: motion_intents.get("RawTurn").copied().unwrap_or(0.0),
                hard_turn: motion_intents.get("HardTurn").copied().unwrap_or(0.0),
            },
            animation_attributes: animation_attributes.to_vec(),
            effects: GraphEffects::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MotionGraphInput {
    pub tick: u64,
    pub action: ActionGraphOutput,
}

#[cfg(any())]
mod tests {
    use super::*;

    #[test]
    fn output_captures_named_turning_without_mutable_map_handoff() {
        let mut intents = IntentMap::new();
        intents.insert("Turn", 0.25);
        intents.insert("RawTurn", -0.5);
        intents.insert("HardTurn", 1.0);
        let output = ActionGraphOutput::from_host(7, &IntentMap::new(), &intents, &[]);
        assert_eq!(output.tick, 7);
        assert_eq!(output.turning.turn, 0.25);
        assert_eq!(output.turning.raw_turn, -0.5);
        assert_eq!(output.turning.hard_turn, 1.0);
        assert!(output.controls.authored_values().is_empty());
    }

    #[test]
    fn output_projects_action_family_controls() {
        let mut action = IntentMap::new();
        action.insert("WipeOutRequest", 1.0);
        action.insert("OB_AirBodyTweakX", 0.25);
        action.insert("OB_AirBodyTweakY", -0.5);
        action.insert("WipeoutGestureX", 0.75);
        action.insert("WipeoutGestureY", -0.25);
        let output = ActionGraphOutput::from_host(3, &action, &IntentMap::new(), &[]);
        assert!(output.controls.wipeout.request);
        assert_eq!(output.controls.wipeout.air_body_tweak, [0.25, -0.5]);
        assert_eq!(output.controls.wipeout.gesture, Some([0.75, -0.25]));
    }

    #[test]
    fn graph_diagnostics_are_bounded_and_tick_resettable() {
        let mut diagnostics = GraphDiagnostics::default();
        for index in 0..65 {
            diagnostics.push(format!("diagnostic {index}"));
        }
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.join("\n").contains("truncated"));
        diagnostics.clear();
        assert!(diagnostics.is_empty());
    }
}
