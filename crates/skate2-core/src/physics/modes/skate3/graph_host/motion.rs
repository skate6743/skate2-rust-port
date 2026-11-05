#[path = "motion_toggle_execute.rs"]
mod toggle_execution;
#[path = "motion_fakie_hold.rs"]
mod fakie_hold;
#[path = "motion_air_execute.rs"]
mod air_execution;
#[path = "motion_execute.rs"]
mod execution;
#[path = "motion_execute_leaf.rs"]
mod leaf_execution;
#[path = "motion_score_execute.rs"]
mod score_execution;
#[path = "motion_filter_execute.rs"]
mod filter_execution;
#[path = "motion_sliding_execute.rs"]
mod slide_execution;
#[path = "motion_wipeout_execute.rs"]
mod wipeout_execution;
#[path = "motion_offboard/body_tweak.rs"]
pub(crate) mod body_tweak;
#[path = "motion_offboard/match_air_time.rs"]
pub(crate) mod match_air_time;
// Persistent host called by the production stock MotionGraph controller.
pub use super::motion_animation::MotionAnimation;
use super::{
    motion_nodes::{MotionFactory, MotionOperation},
    pushing::{PushContext, PushInstance},
    pushing_settings::PushingSettings,
};
use crate::skate3_native::graph_runtime::{LoadedGraph, OperationRemap};
use skate3_core::graph::intents::IntentMap;
use skate3_core::{
    animation::{
        playback::{PlayAnimationInstance, PlaybackContext},
        playback_parameters::{AttributeSink, ParameterInputs, SettableAttribute},
        skeleton_input::name::encode,
    },
    graph::{
        activation::ConditionHost,
        conditions::ConditionInputs,
        controller::{BehaviorId, Frame, HookId, Host},
    },
    input::set_turning,
    riding::push_behaviors::{PushFootFrame, PushState},
};
use skate_data::collections::Collections;
use std::collections::BTreeMap;
use super::outputs::{ActionControls, GraphCapabilityReport, GraphDiagnostics, GraphEffects, MotionGraphInput, TurningOutput};

#[derive(Clone, Copy, Debug)]
pub struct MotionPhysical {
    pub turning: set_turning::Physical,
    pub stance: (bool, bool),
    pub forward_speed: f32,
    pub time_since_teleport: f32,
    pub is_switch: bool,
    pub foot_frame: Option<PushFootFrame>,
}
#[path = "motion_instances.rs"]
mod instances;
use instances::Instance;
pub struct MotionHost {
    pub animation: MotionAnimation,
    pub playback_context: PlaybackContext,
    pub condition_inputs: ConditionInputs,
    pub gameplay_conditions: Option<super::motion_gameplay_conditions::GameplayConditions>,
    pub manual_exit: Option<bool>,
    /// Completed Skeleton PhysOut536/540, in native yaw-then-pitch order.
    pub deck_yaw_pitch: Option<[f32; 2]>,
    pub riding_conditions: Option<super::motion_riding_conditions::RidingConditionInputs>,
    pub condition_random: super::motion_riding_conditions::MotionRandom,
    pub native_physical: Option<super::motion_native::Physical>,
    pub prelanding_physical: Option<super::motion_spin::PrelandingPhysical>,
    pub air_leg_physical: Option<skate3_core::animation::air_leg_extension::Physical>,
    pub gesture_physical: Option<super::motion_native::GesturePhysical>,
    pub gesture_publication: Option<super::motion_character_gesture::GesturePublication>,
    pub toggle_board_physical: Option<super::motion_toggle_board::Physical>,
    pub runout_physical: Option<super::motion_runout::Observation>,
    pub grind_physical: Option<super::motion_grind::Physical>,
    pub grind_conditions: Option<super::motion_grind::conditions::Physical>,
    pub(super) grind_settings: super::motion_grind::Settings,
    pub shove_physical: Option<super::motion_shove::ShovePhysical>,
    pub hand_services: super::motion_hand_services::HandServices,
    pub slide_latch: set_turning::SlideLatch,
    ///SpecificCA4 bit26, getter8258F900/setter8258F910; reset clears it.
    pub is_power_sliding: bool,
    pub score_packet: super::motion_native::ScorePacket,
    pub(super) trick_requests: super::motion_tricks::Requests,
    pub(super) trick_height_settings: (bool, bool),
    pub action_controls: ActionControls,
    pub(crate) action_intents: IntentMap,
    /// Typed turning values transported from the ActionGraph boundary.
    pub turning_output: TurningOutput,
    pub graph_effects: GraphEffects,
    pub time_tags: Option<BTreeMap<String, f32>>,
    pub physical: Option<MotionPhysical>,
    /// Completed Biped/OffBoard publications. These remain absent until the
    /// corresponding physical owner is integrated; graph leaves must not
    /// invent replacement values.
    pub offboard_cadence_phase: Option<f32>,
    pub offboard_locomotion_state: Option<u32>,
    pub ground_slope_type: Option<u32>,
    pub biped_ground_thin: Option<bool>,
    pub animation_phase: f32,
    pub moving_objects: super::motion_moving_object::MovingObjects,
    pub push_state: Option<PushState>,
    pub applying_body_tilt: bool,
    pub crouching_physical: Option<skate3_core::animation::crouching::Physical>,
    pub body_tilt_physical: Option<skate3_core::animation::body_tilt::Physical>,
    pub hold_fakie: bool,
    automatic_fakie_conditions: Vec<bool>,
    pub fakie_physical: Option<skate3_core::animation::riding_fakie::Physical>,
    /// PhysOutAnimation158/157, used directly by IsRidingGoofy82BA5AA8.
    pub physical_stance: Option<(bool, bool)>,
    pub flags: super::motion_landing::Flags,
    hippy_jump: super::motion_hippy_jump::Settings,
    finger_flip: super::motion_finger_flip::Settings,
    pub landing_physical: Option<super::motion_landing::Physical>,
    pub wipeout_physical: Option<super::motion_wipeout::Physical>,
    pub wipeout_controls: super::motion_wipeout::Controls,
    wipeout_settings: super::motion_wipeout::Settings,
    ///Native PhysOutGround pumping acceleration268.
    pub pumping_acceleration: Option<f32>,
    /// Completed PhysOutAnimation112 acceleration before bump conditioning.
    pub bump_acceleration: Option<[f32; 4]>,
    bump_settings: super::motion_bump::Settings,
    pub allow_pumping: bool,
    pub riding: super::motion_riding::RidingState,
    pub errors: GraphDiagnostics,
    pub(super) state_parents: Vec<Option<usize>>,
    operations: Vec<MotionOperation>,
    instances: Vec<Instance>,
    remap: OperationRemap,
    pushing: PushingSettings,
    turning: set_turning::Settings,
    crouching: skate3_core::animation::crouching::Settings,
    body_tilt: skate3_core::animation::body_tilt::Settings,
    pumping: skate3_core::animation::pumping_channel::Settings,
    sliding: super::motion_sliding::Settings,
    pub(super) spin: super::motion_spin::Settings,
    air_leg: skate3_core::animation::air_leg_extension::Settings,
    kickturn: skate3_core::animation::kickturn::Settings,
    manual: skate3_core::animation::manual::Settings,
    next_instance: u32,
}
impl MotionHost {
    pub(super) fn end_gesture_channels(&mut self) {
        // EndGesture nodes share the graph-wide gesture channel owner. The
        // active CharacterGesture instance is found by its operation binding.
        for instance in &mut self.instances {
            if let Instance::CharacterGesture(state) = instance {
                state.end(&mut self.animation);
            }
        }
        self.gesture_publication = None;
    }

    pub fn from_graph(
        graph: &LoadedGraph,
        data: &Collections,
        mut metadata: skate_data::animation_metadata::AnimationMetadata,
        playback_context: PlaybackContext,
    ) -> Result<Self, String> {
        let mut operations = graph
            .binding
            .instantiate_operations(&graph.source, &mut MotionFactory)
            .map_err(|e| e.to_string())?
            .operations;
        super::motion_conditions::bind_states(graph, &mut operations);
        let instances = graph
            .runtime
            .operations
            .behaviors
            .iter()
            .map(|&id| Instance::new(&operations[id]))
            .collect();
        let mut capabilities = GraphCapabilityReport {
            supported_operations: operations
                .iter()
                .filter(|operation| !matches!(operation, MotionOperation::Unsupported { .. }))
                .count(),
            unsupported_operations: operations
                .iter()
                .filter_map(|operation| match operation {
                    MotionOperation::Unsupported { kind, name } => {
                        Some(format!("{:?} `{name}`", kind))
                    }
                    _ => None,
                })
                .collect(),
            ..GraphCapabilityReport::default()
        };
        for &operation in &graph.runtime.operations.conditions {
            if let Some(MotionOperation::Unsupported { kind, name }) = operations.get(operation) {
                capabilities
                    .unsupported_conditions
                    .push(format!("{kind:?} `{name}`"));
            }
        }
        for &operation in &graph.runtime.operations.hooks {
            if let Some(MotionOperation::Unsupported { kind, name }) = operations.get(operation) {
                capabilities.unsupported_hooks.push(format!("{kind:?} `{name}`"));
            }
        }
        // Preserve lazy diagnostics for optional unsupported graph branches.
        //8258F488 and final reset825953B0 explicitly zero the complete push
        //state; reset clears manualing and body-tilt flag bits too.
        let zero = skate3_core::riding::push_animation::PushBlendParameters {
            hstr_vel_b: 0.0,
            lstr_vel_b: 0.0,
            vel_e: 0.0,
        };
        let push_state = Some(PushState {
            out_factor: 0.0,
            current_push_dv: 0.0,
            current: zero,
            target: zero,
            continue_push: false,
        });
        let pushing = PushingSettings::load(data, &mut metadata)?;
        Ok(Self {
            animation: MotionAnimation::from_metadata(metadata),
            playback_context,
            condition_inputs: ConditionInputs::default(),
            gameplay_conditions: None,
            manual_exit: None,
            deck_yaw_pitch: None,
            riding_conditions: None,
            condition_random: super::motion_riding_conditions::MotionRandom::new(),
            native_physical: None,
            prelanding_physical: None,
            air_leg_physical: None,
            gesture_physical: None,
            gesture_publication: None,
            toggle_board_physical: None,
            runout_physical: None,
            grind_physical: None,
            grind_conditions: None,
            grind_settings: super::motion_grind::Settings::load(data)?,
            shove_physical: None,
            hand_services: Default::default(),
            slide_latch: set_turning::SlideLatch::default(),
            is_power_sliding: false,
            score_packet: super::motion_native::ScorePacket::default(),
            trick_requests: Default::default(),
            trick_height_settings: (true, true),
            action_controls: ActionControls::default(),
            action_intents: IntentMap::new(),
            turning_output: TurningOutput::default(),
            graph_effects: GraphEffects::default(),
            time_tags: None,
            physical: None,
            offboard_cadence_phase: None,
            offboard_locomotion_state: None,
            ground_slope_type: None,
            biped_ground_thin: None,
            animation_phase: 0.0,
            moving_objects: super::motion_moving_object::MovingObjects::new(
                super::motion_moving_object::Settings::load(data)?,
            ),
            push_state,
            applying_body_tilt: false,
            crouching_physical: None,
            body_tilt_physical: None,
            hold_fakie: false,
            automatic_fakie_conditions: fakie_hold::bind(&graph.binding),
            fakie_physical: None,
            physical_stance: None,
            flags: Default::default(),
            landing_physical: None,
            wipeout_physical: None,
            wipeout_controls: Default::default(),
            wipeout_settings: super::motion_wipeout::Settings::load(data)?,
            pumping_acceleration: None,
            bump_acceleration: None,
            bump_settings: super::motion_bump::Settings::load(data)?,
            //8258F488 seeds bit24, and reset825953B0 preserves that bit.
            allow_pumping: true,
            riding: super::motion_riding::RidingState::new(),
            errors: GraphDiagnostics::default(),
            state_parents: graph.binding.states.iter().map(|s| s.parent).collect(),
            operations,
            instances,
            remap: graph.runtime.operations.clone(),
            pushing,
            turning: super::turning_settings::load(data)?,
            crouching: super::crouching_settings::load(data)?,
            body_tilt: super::motion_riding::body_tilt_settings(data)?,
            pumping: super::pumping_settings::load(data)?,
            sliding: super::motion_sliding::Settings::load(data)?,
            spin: super::motion_spin::Settings::load(data)?,
            air_leg: super::motion_air_leg::load(data)?,
            kickturn: super::motion_kickturn::load_settings(data)?,
            manual: super::motion_manual::settings(data)?,
            hippy_jump: super::motion_hippy_jump::Settings::load(data)?,
            finger_flip: super::motion_finger_flip::Settings::load(data)?,
            next_instance: 1,
        })
    }

    pub fn accept_action_graph(&mut self, input: MotionGraphInput) {
        let MotionGraphInput { tick, action } = input;
        debug_assert_eq!(tick, action.tick);
        self.errors.clear();
        self.turning_output = action.turning;
        self.graph_effects = action.effects.clone();
        self.action_controls = action.controls.clone();
        // Clone before handing the full output to MotionAnimation so the two
        // consumers observe one immutable packet.
        self.action_intents = action.controls.authored_values();
        self.animation.accept_action_graph(action);
    }
    fn run(&mut self, id: BehaviorId, frame: &Frame, phase: u8) {
        if let Err(error) = self.execute(id, frame, phase) {
            self.errors
                .push(format!("MotionGraph behavior {id}: {error}"));
        }
    }
}
impl ConditionHost for MotionHost {
    fn condition_activation(&mut self, condition: usize, frame: &Frame) -> u32 {
        // Gate only the authored automatic switch transitions. Other fakie
        // conditions (tricks, pushes, dismounts) still see the real stance.
        if self.remap.conditions.get(condition).is_some_and(|&id|
            fakie_hold::blocked(self.hold_fakie, &self.automatic_fakie_conditions, id)) {
            return 0;
        }
        let result = self
            .remap
            .conditions
            .get(condition)
            .and_then(|&id| self.operations.get(id))
            .ok_or_else(|| "Unbound MotionGraph condition".to_owned())
            .and_then(|op| match op {
                MotionOperation::Condition(condition) => condition.evaluate(self, frame),
                other => Err(format!("Unsupported MotionGraph condition {other:?}")),
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
impl Host for MotionHost {
    fn context(&self) -> [u32; 6] {
        [0; 6]
    }
    fn allocate(&mut self, behavior: BehaviorId, _frame: &Frame) -> u32 {
        if let Some(&id) = self.remap.behaviors.get(behavior) {
            self.instances[behavior] = Instance::new(&self.operations[id]);
        }
        let handle = self.next_instance;
        self.next_instance = self.next_instance.wrapping_add(1).max(1);
        handle
    }
    fn begin(&mut self, id: BehaviorId, _context: [u32; 6], frame: &Frame) {
        self.run(id, frame, 0);
    }
    fn update(&mut self, id: BehaviorId, _context: [u32; 6], frame: &Frame) {
        self.run(id, frame, 1);
    }
    fn end(&mut self, id: BehaviorId, _context: [u32; 6], frame: &Frame) {
        self.run(id, frame, 2);
    }
    fn hook(&mut self, hook: HookId, _frame: &Frame) {
        if let Some(MotionOperation::Hook(operation)) = self
            .remap
            .hooks
            .get(hook)
            .and_then(|&id| self.operations.get(id))
            .cloned()
        {
            match operation {

                super::motion_hooks::MotionHook::GrabSlide { right } => {
                    self.slide_latch.grab(right)
                }
                super::motion_hooks::MotionHook::Override(settings) => {
                    self.playback_context.transition_override = Some(settings)
                }
                super::motion_hooks::MotionHook::MongoPushToAntic { animation } => {
                    //82BBBBB8 passes the authored name for both channel/tree.
                    let settings = skate3_core::animation::channel_playback::ChannelSettings {
                        priority: 0,
                        keep_alive: false,
                        mirrored: false,
                        speed: 1.0,
                        blend_in: f32::from_bits(0x3dcccccd),
                        hold_during_blend_in: false,
                        blend_out: f32::from_bits(0x3d8f5c29),
                        hold_during_blend_out: false,
                        use_attributes: false,
                    };
                    if let Err(error) = self.animation.new_channel(&animation, &animation, settings)
                    {
                        self.errors.push(error);
                    }
                }
            }
            return;
        }
        self.errors.push(format!(
            "Unsupported MotionGraph hook {:?}",
            self.remap
                .hooks
                .get(hook)
                .and_then(|&id| self.operations.get(id))
        ));
    }
    fn release(&mut self, _instance: u32) {}
}

#[cfg(any())]
#[path = "tests/motion_stock.rs"]
mod tests;

#[cfg(any())]
#[path = "tests/motion_slide.rs"]
mod slide_tests;

#[cfg(any())]
#[path = "tests/motion_jump_into.rs"]
mod jump_into_tests;

#[cfg(any())]
#[path = "tests/motion_deck_angles.rs"]
mod deck_angles_tests;

#[cfg(any())]
#[path = "tests/motion_finger_flip.rs"]
mod finger_flip_tests;

#[cfg(any())]
#[path = "tests/motion_board_adjust.rs"]
mod board_adjust_tests;
