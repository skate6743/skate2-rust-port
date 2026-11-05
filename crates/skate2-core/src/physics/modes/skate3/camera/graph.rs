//! Stock default camera graph, using the same recovered dynamic controller as
//! the action and motion graphs. Shot behavior side effects occur at Begin.
use std::path::Path;
use skate3_core::{camera::{CameraMan, ManagerSubject, SlowMotionController, SlowMotionSettings,
    SimulationRateRequest}, graph::{activation::ConditionHost,
    controller::{Controller, Frame, Host}}};
use skate_data::state_graph::{StateGraph, attributes::Attributes, binding::Binding};
use crate::skate3_native::graph_runtime::CompiledGraph;
use super::{graph_conditions::{Condition, shot_names}, graph_subject::{CameraGraphEnvironment,
    CameraGraphSubject}, shot_data::StockShots};

pub(super) struct CameraGraph {
    program: CompiledGraph,
    controller: Controller,
    conditions: Vec<Condition>,
    behaviors: Vec<Behavior>,
    slow_motion: Vec<Option<SlowMotionController>>,
    slow_motion_settings: SlowMotionSettings,
}
enum Behavior {
    Choose { names: Vec<String>, incoming: f32, outgoing: f32 },
    Print(String),
    SlowMotion,
}
impl CameraGraph {
    pub fn load(path: &Path, slow_motion_settings: SlowMotionSettings) -> Result<Self, String> {
        let source = StateGraph::load(path).map_err(|e| e.to_string())?;
        let binding = Binding::from_graph(&source).map_err(|e| e.to_string())?;
        let program = CompiledGraph::from_binding(&binding).map_err(|e| e.to_string())?;
        let conditions = program.operations.conditions.iter().map(|id| {
            let node = &source.elements[binding.operations[*id].element];
            Condition::parse(&Attributes::new(&node.attributes))
        }).collect::<Result<Vec<_>, _>>()?;
        let behaviors = program.operations.behaviors.iter().map(|id| {
            let operation = &binding.operations[*id];
            let node = &source.elements[operation.element];
            let a = Attributes::new(&node.attributes);
            Ok(match operation.name.as_str() {
                "CameraChooseShot" => {
                    let names = shot_names(&a);
                    if names.is_empty() {
                        return Err("CameraChooseShot has no named shot".into());
                    }
                    Behavior::Choose {
                        names,
                        incoming: f32::from_bits(a.float_bits("transitionIn", (-1.0_f32).to_bits())),
                        outgoing: f32::from_bits(a.float_bits("transitionOut", (-1.0_f32).to_bits())),
                    }
                }
                "PrintText2D" => Behavior::Print(a.text("text").unwrap_or("").into()),
                "SlowMotionController" => Behavior::SlowMotion,
                name => return Err(format!("Unimplemented camera behavior {name}")),
            })
        }).collect::<Result<Vec<_>, String>>()?;
        if !program.operations.hooks.is_empty() {
            return Err("Camera graph has unimplemented transition hooks".into());
        }
        let controller = Controller::new(binding.states.len());
        let slow_motion = vec![None; behaviors.len()];
        Ok(Self { program, controller, conditions, behaviors, slow_motion, slow_motion_settings })
    }

    pub fn update(&mut self, dt: f32, manager: &mut CameraMan, subject: &ManagerSubject,
        physical: CameraGraphSubject, world: &CameraGraphEnvironment, database: &StockShots)
        -> Result<Vec<SimulationRateRequest>, String> {
        let mut host = CameraHost { manager, subject, physical, world, database,
            conditions: &self.conditions, behaviors: &self.behaviors, error: None,
            slow_motion: &mut self.slow_motion, slow_motion_settings: self.slow_motion_settings,
            rates: Vec::new() };
        self.controller.update(&self.program.program, dt, &mut host);
        if let Some(error) = host.error { Err(error) } else { Ok(host.rates) }
    }
}
struct CameraHost<'a> {
    manager: &'a mut CameraMan,
    subject: &'a ManagerSubject,
    physical: CameraGraphSubject,
    world: &'a CameraGraphEnvironment,
    database: &'a StockShots,
    conditions: &'a [Condition],
    behaviors: &'a [Behavior],
    error: Option<String>,
    slow_motion: &'a mut [Option<SlowMotionController>],
    slow_motion_settings: SlowMotionSettings,
    rates: Vec<SimulationRateRequest>,
}
impl ConditionHost for CameraHost<'_> {
    fn condition_activation(&mut self, id: usize, _: &Frame) -> u32 {
        u32::from(self.conditions[id].evaluate(self.manager, self.subject, self.physical, self.world))
    }
}
impl Host for CameraHost<'_> {
    fn context(&self) -> [u32; 6] { [0; 6] }
    fn allocate(&mut self, _: usize, _: &Frame) -> u32 { 0 }
    fn begin(&mut self, id: usize, _: [u32; 6], _: &Frame) {
        match &self.behaviors[id] {
            Behavior::Choose { names, incoming, outgoing } => {
                let Some(name) = names.first() else {
                    self.error = Some("CameraChooseShot has no named shot".into());
                    return;
                };
                if let Err(error) = self.manager.set_shot(name, false, self.subject, self.database) {
                    self.error = Some(error); return;
                }
                self.manager.shots.apply_graph_transition(*incoming, *outgoing);
            }
            Behavior::Print(text) => crate::skate3_native::trace::warn!("Stock camera graph: {text}"),
            Behavior::SlowMotion => {
                let (instance, request) = SlowMotionController::begin(self.slow_motion_settings);
                self.slow_motion[id] = Some(instance);
                self.rates.push(request);
            }
        }
    }
    // CameraChooseShot Update/End both resolve to TU382B61BB8 (return).
    fn update(&mut self, id: usize, _: [u32; 6], frame: &Frame) {
        if let Some(instance) = &mut self.slow_motion[id] {
            self.rates.push(instance.update(frame.dt, self.physical.slow_motion_air_duration,
                self.slow_motion_settings));
        }
    }
    fn end(&mut self, id: usize, _: [u32; 6], _: &Frame) {
        if self.slow_motion[id].take().is_some() { self.rates.push(SlowMotionController::end()); }
    }
    fn hook(&mut self, _: usize, _: &Frame) { unreachable!("camera hooks rejected during load") }
    fn release(&mut self, _: u32) {}
}
