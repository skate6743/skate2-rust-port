//! Typed ActionGraph registration and persistent operation instances.
//! The schedule and concrete gameplay sinks are owned by the caller.

use super::action_nodes::{ActionFactory, ActionInstance, ActionInstances, ActionOperation};
use crate::skate3_native::graph_runtime::{LoadedGraph, OperationRemap};
use skate3_core::graph::conditions::ConditionInputs;
use skate3_core::graph::intent_handlers::{ConstMgIntent, TimeMgIntent};
use skate3_core::graph::intents::IntentMap;
use skate3_core::graph::{
    activation::ConditionHost,
    controller::{BehaviorId, Frame, HookId, Host},
};
use skate3_core::input::body_flip_signal;
use skate3_core::input::graph_intents::{CreateMgIntent, IntentMutation};
#[path = "action_board_adjust.rs"]
mod board_adjust;
use skate_data::collections::Collections;
use super::outputs::{ActionGraphInput, ActionGraphOutput, GraphCapabilityReport, GraphDiagnostics};

/// Original State output consumed by82BA1680/82BA16F0.
#[derive(Clone, Copy, Debug)]
pub struct PhysicalConditions {
    pub requests_dismount: bool,
    pub state: u32,
}

/// Runtime owner for the recovered ActionGraph intent path. Conditions read
/// current published physics and intent values when the graph evaluates them.
pub struct ActionHost {
    pub instances: ActionInstances,
    pub action_intents: IntentMap,
    pub motion_intents: IntentMap,
    pub condition_inputs: ConditionInputs,
    pub physical_conditions: Option<PhysicalConditions>,
    pub gameplay_conditions: Option<super::motion_gameplay_conditions::GameplayConditions>,
    pub animation_attributes: Vec<skate3_core::animation::output::attributes::AnimationAttribute>,
    pub is_tricking: Option<bool>,
    pub dropping_in: Option<bool>,
    pub stance: Option<(bool, bool)>,
    tick: u64,
    pub errors: GraphDiagnostics,
    remap: OperationRemap,
    state_parents: Vec<Option<usize>>,
    next_instance: u32,
    created: Vec<bool>,
    const_handlers: Vec<ConstMgIntent>,
    time_handlers: Vec<TimeMgIntent>,
    board_adjust: Vec<board_adjust::State>,
    body_flip: Vec<body_flip_signal::State>,
    body_flip_settings: Option<body_flip_signal::Settings>,
    /// JuiceHook instance+8 starts with the native null intent name. Its
    /// update removes that pending key and resets it,82BA39A8.
    juice_pending: Vec<String>,
    trick_handlers: Vec<crate::skate3_native::input::gesture_mapping::State>,
}

impl ActionHost {
    pub fn from_graph(graph: &LoadedGraph, data: &Collections) -> Result<Self, String> {
        let operations = graph
            .binding
            .instantiate_operations(&graph.source, &mut ActionFactory)
            .map_err(|error| error.to_string())?;
        let mut instances = ActionInstances::new(operations.operations);
        let mut capabilities = GraphCapabilityReport {
            supported_operations: instances
                .operations
                .iter()
                .filter(|instance| instance.unsupported().is_none())
                .count(),
            unsupported_operations: instances
                .operations
                .iter()
                .filter_map(|instance| instance.unsupported())
                .map(|operation| format!("{:?} `{}`", operation.kind, operation.name))
                .collect(),
            ..GraphCapabilityReport::default()
        };
        for &operation in &graph.runtime.operations.conditions {
            if let Some(instance) = instances.get(operation) {
                if let Some(unsupported) = instance.unsupported() {
                    capabilities.unsupported_conditions.push(format!(
                        "{:?} `{}`",
                        unsupported.kind, unsupported.name
                    ));
                }
            }
        }
        for &operation in &graph.runtime.operations.hooks {
            if let Some(instance) = instances.get(operation) {
                if let Some(unsupported) = instance.unsupported() {
                    capabilities.unsupported_hooks.push(format!(
                        "{:?} `{}`",
                        unsupported.kind, unsupported.name
                    ));
                }
            }
        }
        // Preserve lazy diagnostics for optional unsupported graph branches.
        super::condition_nodes::bind_current_states(graph, &mut instances);
        let mut host = Self::new(instances, graph.runtime.operations.clone());
        host.body_flip_settings = Some(body_flip_signal::Settings {
            //82BA35E0/3790 read Globals296;8289F8F0 binds hash
            //3FC308E69AEA6385 (body_flip), independently checked in stock data.
            gesture_window: data.float("anim_motion", "body_flip", "extend_bodyflip_gesture")?,
            takeoff_window: data.float("anim_motion", "body_flip", "extend_takeoff_point")?,
        });
        host.state_parents = graph
            .binding
            .states
            .iter()
            .map(|state| state.parent)
            .collect();
        Ok(host)
    }

    pub(crate) fn new(instances: ActionInstances, remap: OperationRemap) -> Self {
        let count = remap.behaviors.len();
        let const_handlers = remap
            .behaviors
            .iter()
            .map(|&operation| {
                let instance = &instances.operations[operation];
                ConstMgIntent::new(
                    instance
                        .config
                        .float_bits
                        .map(f32::from_bits)
                        .unwrap_or(0.0),
                    instance.config.on_update,
                )
            })
            .collect();
        let time_handlers = vec![TimeMgIntent::default(); count];
        Self {
            instances,
            action_intents: IntentMap::new(),
            motion_intents: IntentMap::new(),
            condition_inputs: ConditionInputs::default(),
            physical_conditions: None,
            gameplay_conditions: None,
            animation_attributes: Vec::new(),
            is_tricking: None,
            dropping_in: None,
            state_parents: Vec::new(),
            remap,
            stance: None,
            tick: 0,
            errors: GraphDiagnostics::default(),
            next_instance: 1,
            created: vec![false; count],
            const_handlers,
            time_handlers,
            board_adjust: vec![board_adjust::State::default(); count],
            body_flip: vec![body_flip_signal::State::default(); count],
            body_flip_settings: None,
            juice_pending: vec![String::new(); count],
            trick_handlers: vec![crate::skate3_native::input::gesture_mapping::State::default(); count],
        }
    }

    pub fn apply(&mut self, name: impl Into<String>, mutation: IntentMutation) {
        let name = name.into();
        match mutation {
            IntentMutation::None => {}
            IntentMutation::Set(value) => {
                self.motion_intents.insert(&name, value);
            }
            IntentMutation::Remove => {
                self.motion_intents.remove(&name);
            }
        }
    }

    pub fn prepare_input(&mut self, input: ActionGraphInput) {
        self.errors.clear();
        self.tick = input.tick;
        self.action_intents = input.controls.into_values();
        self.motion_intents = input.prior_motion.into_values();
        self.animation_attributes = input.animation_attributes;
    }

    pub fn output(&self) -> ActionGraphOutput {
        ActionGraphOutput::from_host(
            self.tick,
            &self.action_intents,
            &self.motion_intents,
            &self.animation_attributes,
        )
    }

    fn operation(&self, id: BehaviorId) -> Option<&ActionInstance> {
        self.remap
            .behaviors
            .get(id)
            .and_then(|&operation| self.instances.get(operation))
    }

    fn unsupported(&mut self, id: BehaviorId, instance: &ActionInstance) {
        if let Some(error) = instance.unsupported() {
            self.errors.push(format!(
                "ActionGraph behavior {id}: {:?} `{}`",
                error.kind, error.name
            ));
        }
    }

    fn publish_board_adjust(&mut self, behavior: BehaviorId, instance: &ActionInstance) {
        let Some(magnitude_name) = instance.config.mg_intent_mag.as_deref() else {
            return;
        };
        let Some(angle_name) = instance.config.mg_intent_angle.as_deref() else {
            return;
        };

        if let Some((magnitude, angle)) = self.board_adjust[behavior].update(
            self.action_intents.get("BoardAdjustMag").copied(),
            self.action_intents.get("BoardAdjustAngle").copied(),
            instance.config.angle_filter, instance.config.negate_on_mirror,
            self.stance.is_some_and(|(_, mirrored)| mirrored),
        ) {
            self.motion_intents.insert(magnitude_name, magnitude);
            self.motion_intents.insert(angle_name, angle);
        } else {
            self.motion_intents.remove(magnitude_name);
            self.motion_intents.remove(angle_name);
        }
    }

}

impl ActionHost {
    /// Diagnostics: a bound condition's operation and stock name.
    pub(crate) fn condition_label(&self, condition: usize) -> String {
        self.remap
            .conditions
            .get(condition)
            .and_then(|&id| self.instances.get(id))
            .map_or_else(
                || format!("unbound{condition}"),
                |instance| format!("{}:{:?}", instance.config.name.as_deref().unwrap_or("?"), instance.operation),
            )
    }
}

impl ConditionHost for ActionHost {
    fn condition_activation(&mut self, condition: usize, frame: &Frame) -> u32 {
        let result = self
            .remap
            .conditions
            .get(condition)
            .and_then(|&id| self.instances.get(id))
            .ok_or_else(|| format!("ActionGraph condition {condition} is unbound"))
            .and_then(|instance| match &instance.operation {
                ActionOperation::ExtraCondition(condition) => condition.evaluate(self, frame),
                ActionOperation::PhysicsRequestsDismount => self
                    .physical_conditions
                    .map(|p| p.requests_dismount)
                    .ok_or_else(|| "PhysicsRequestsDismount requires published State77".into()),
                ActionOperation::IsLandingOnBoard => self
                    .physical_conditions
                    .map(|p| p.state == 503)
                    .ok_or_else(|| "IsLandingOnBoard requires actual State16".into()),
                ActionOperation::Condition(operation) => operation
                    .evaluate(
                        &self.condition_inputs,
                        &self.action_intents,
                        frame.current,
                        &self.state_parents,
                    )
                    .map_err(|error| format!("ActionGraph condition {condition}: {error}")),
                _ => Err(format!(
                    "Unsupported ActionGraph condition {condition}: {:?}",
                    instance.config.name
                )),
            });
        match result {
            Ok(value) => u32::from(value),
            Err(error) => {
                self.errors.push(error);
                0
            }
        }
    }
}

impl Host for ActionHost {
    fn context(&self) -> [u32; 6] {
        [0; 6]
    }
    fn allocate(&mut self, behavior: BehaviorId, _frame: &Frame) -> u32 {
        // Every activation owns fresh behavior state. Re-entry must not reuse
        // the old pushing clock, even if the source operation ID is unchanged.
        self.created[behavior] = false;
        self.time_handlers[behavior] = TimeMgIntent::default();
        self.body_flip[behavior] = body_flip_signal::State::default();
        self.juice_pending[behavior].clear();
        self.trick_handlers[behavior] = crate::skate3_native::input::gesture_mapping::State::default();
        let result = self.next_instance;
        self.next_instance = self.next_instance.wrapping_add(1).max(1);
        result
    }
    fn begin(&mut self, behavior: BehaviorId, _context: [u32; 6], _frame: &Frame) {
        let Some(instance) = self.operation(behavior).cloned() else {
            return;
        };
        if let ActionOperation::CreateTrickIntentFromGesture { group, override_name } = &instance.operation {
            if let Some((_, mirrored)) = self.stance {
                self.trick_handlers[behavior].begin(*group, override_name.as_deref(), &self.action_intents, &mut self.motion_intents, mirrored);
            } else {
                self.errors.push("CreateTrickIntentFromGesture requires published skater stance");
            }
            return;
        }
        if instance.unsupported().is_some() {
            self.unsupported(behavior, &instance);
            return;
        }
        if let ActionOperation::PrintText(text) = &instance.operation {
            crate::skate3_native::trace::info!("ActionGraph: {text}");
            return;
        }
        if let ActionOperation::BodyFlippingSignal = instance.operation {
            match self.body_flip_settings {
                Some(settings) => self.body_flip[behavior].begin(settings),
                None => self
                    .errors
                    .push("BodyFlippingSignal requires stock anim_motion settings"),
            }
            return;
        }
        if let ActionOperation::BoardAdjust = instance.operation {
            self.board_adjust[behavior].begin();
            return;
        }
        if let ActionOperation::CreateConstMgIntent = instance.operation {
            if let Some(name) = instance.config.mg_intent.clone() {
                let mutation = self.const_handlers[behavior].begin();
                self.apply(name, mutation);
            }
        }
        if let ActionOperation::CreateMgIntent = instance.operation {
            if let (Some(mg), Some(ag)) = (
                instance.config.mg_intent.clone(),
                instance.config.ag_intent.as_deref(),
            ) {
                let handler = CreateMgIntent {
                    on_update: instance.config.on_update,
                    default_value: instance.config.default_value,
                    scale: instance.config.scale.unwrap_or(1.0),
                    filters: instance.config.filters,
                };
                let value = self.action_intents.get(ag).copied();
                let mutation = handler.enter(&mut self.created[behavior], value, self.stance);
                self.apply(mg, mutation);
            } else {
                self.errors.push(format!(
                    "ActionGraph behavior {behavior}: missing AGIntent/MGIntent"
                ));
            }
        }
    }
    fn update(&mut self, behavior: BehaviorId, _context: [u32; 6], frame: &Frame) {
        let Some(instance) = self.operation(behavior).cloned() else {
            return;
        };
        if matches!(instance.operation, ActionOperation::CreateTrickIntentFromGesture { .. }) {
            self.trick_handlers[behavior].update(&mut self.motion_intents);
            return;
        }
        if instance.unsupported().is_some() {
            self.unsupported(behavior, &instance);
            return;
        }
        if let ActionOperation::JuiceHook = instance.operation {
            self.motion_intents.remove(&self.juice_pending[behavior]);
            self.juice_pending[behavior].clear();
            return;
        }
        if let ActionOperation::BodyFlippingSignal = instance.operation {
            let Some(settings) = self.body_flip_settings else {
                self.errors
                    .push("BodyFlippingSignal requires stock anim_motion settings");
                return;
            };
            let Some(physical) = &self.condition_inputs.physical_state else {
                self.errors
                    .push("BodyFlippingSignal requires published filtered state");
                return;
            };
            // TU3 initializers82F84DA8/82F84DC0 bind these exact keys.
            let names = ["FrontFlip", "BackFlip"];
            let present = names.map(|name| self.action_intents.contains_key(name));
            match self.body_flip[behavior].update(present, physical.category, frame.dt, settings) {
                Some(index) => {
                    self.motion_intents.insert(names[index], 1.0);
                }
                None => {
                    for name in names {
                        self.motion_intents.remove(name);
                    }
                }
            }
            return;
        }
        if let ActionOperation::BoardAdjust = instance.operation {
            self.publish_board_adjust(behavior, &instance);
            return;
        }
        if let ActionOperation::CreateConstMgIntent = instance.operation {
            if let Some(name) = instance.config.mg_intent.as_deref() {
                let mutation = self.const_handlers[behavior].update();
                self.apply(name.to_owned(), mutation);
            }
            return;
        }
        let Some(ag_name) = instance.config.ag_intent.as_deref() else {
            return;
        };
        let Some(mg_name) = instance.config.mg_intent.as_deref() else {
            return;
        };
        let ag = self.action_intents.get(ag_name).copied();
        let mutation = match instance.operation {
            ActionOperation::CreateMgIntent => {
                let handler = CreateMgIntent {
                    on_update: instance.config.on_update,
                    default_value: instance.config.default_value,
                    scale: instance.config.scale.unwrap_or(1.0),
                    filters: instance.config.filters,
                };
                handler.update(&mut self.created[behavior], ag, self.stance)
            }
            ActionOperation::CreateMgTimeIntent => {
                self.time_handlers[behavior].update(ag, frame.dt)
            }
            _ => return,
        };
        self.apply(mg_name.to_owned(), mutation);
    }
    fn end(&mut self, behavior: BehaviorId, _context: [u32; 6], _frame: &Frame) {
        let Some(instance) = self.operation(behavior).cloned() else {
            return;
        };
        if matches!(instance.operation, ActionOperation::BoardAdjust) {
            //82BA2EB8: remove both authored names; End does not reset state.
            for name in [instance.config.mg_intent_mag.as_deref(),
                         instance.config.mg_intent_angle.as_deref()].into_iter().flatten() {
                self.motion_intents.remove(name);
            }
            return;
        }
        if matches!(instance.operation, ActionOperation::CreateTrickIntentFromGesture { .. }) {
            self.trick_handlers[behavior].end(&mut self.motion_intents);
            return;
        }
        if let Some(name) = instance.config.mg_intent.as_deref() {
            let mutation = match instance.operation {
                ActionOperation::CreateConstMgIntent => self.const_handlers[behavior].end(),
                ActionOperation::CreateMgTimeIntent => self.time_handlers[behavior].end(),
                _ => IntentMutation::Remove,
            };
            self.apply(name.to_owned(), mutation);
        }
    }
    fn hook(&mut self, hook: HookId, _frame: &Frame) {
        let Some(instance) = self
            .remap
            .hooks
            .get(hook)
            .and_then(|&operation| self.instances.get(operation))
            .cloned()
        else {
            self.errors
                .push(format!("ActionGraph hook index {hook} is unbound"));
            return;
        };
        if instance.unsupported().is_some() {
            self.unsupported(hook, &instance);
        }
    }
    fn release(&mut self, _instance: u32) {}
}
