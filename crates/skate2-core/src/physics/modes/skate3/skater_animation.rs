//! The skater's stock graph -> tree -> pose -> physics-packet owner.
//! Physical observations come from the preceding completed simulation output.
mod state;
use crate::skate3_native::{
    animation_pose::PoseEvaluator,
    graph_host::{
        action::ActionHost,
        motion::{MotionHost, MotionPhysical},
        outputs::{ActionGraphInput, ActionInput, MotionGraphInput, PriorMotionState},
    },
    graph_runtime::StockGraphs,
};
use skate3_core::graph::intents::IntentMap;
use skate3_core::{
    animation::{
        body_tilt,
        output::{
            self, Sqt,
            attributes::PacketAttributes,
            packet_reset::{self, AdditionalResetFields, RESET_POSE},
            physics_packet::{self, PhysicsPosePacket},
        },
        physical_feedback::PhysicalFeedback,
        playback::PlaybackContext,
        playback_tree::{Evaluation, PoseCommand},
        riding_fakie,
        skeleton_input::name::encode,
    },
    graph::{conditions::ConditionInputs, controller::Controller},
    riding::push_behaviors::PushFootFrame,
};
use skate_data::collections::Collections;
use std::{path::Path, sync::Arc};

/// Immutable bank bytes and decoded poses survive world changes.
pub(crate) struct AnimationSource {
    banks: skate_data::animation_banks::AnimationBanks,
    evaluator: Arc<PoseEvaluator>,
}
impl AnimationSource {
    pub(crate) fn banks(&self)->&skate_data::animation_banks::AnimationBanks {&self.banks}
    pub(crate) fn evaluator(&self)->Arc<PoseEvaluator>{Arc::clone(&self.evaluator)}
    pub(crate) fn metadata(&self)->Result<skate_data::animation_metadata::AnimationMetadata,String>{self.banks.metadata()}
    pub(crate) fn load(root: &Path) -> Result<Arc<Self>, String> {
        let banks = skate_data::animation_banks::AnimationBanks::load(root)?;
        let mut evaluator = PoseEvaluator::from_banks(&banks)?;
        evaluator.load_authored_clips(root)?;
        Ok(Arc::new(Self { banks, evaluator: Arc::new(evaluator) }))
    }
}

/// Completed native publications, never inferred from a render transform.
pub(crate) struct AnimationPhysical {
    pub conditions: ConditionInputs,
    pub feedback: PhysicalFeedback,
    pub body_tilt: body_tilt::Physical,
    pub fakie: riding_fakie::Physical,
    ///PhysOutAnimation158/157, consumed by IsRidingGoofy.
    pub physical_stance: (bool, bool),
    pub foot_frame: PushFootFrame,
    ///PhysOut72 byte311 (true means a board is present).
    ///both PlayAnimation82BB54D4 and Phase1 consume the byte without inversion.
    pub board_present: bool,
    pub physical_28_byte75: bool,
    pub time_since_teleport: f32,
}

pub(crate) struct SkaterAnimation {
    pub action: ActionHost,
    pub motion: MotionHost,
    pub evaluator: Arc<PoseEvaluator>,
    pub source: Arc<AnimationSource>,
    pub pose: Vec<Sqt>,
    /// The body tree's command list behind `pose`, for its clips' hand streams.
    pub body_commands: Vec<PoseCommand>,
    pub packet: PhysicsPosePacket,
    pub attributes: PacketAttributes,
    pub action_controller: Controller,
    pub motion_controller: Controller,
    state: state::AnimationState,
    pub ticks: u64,
}

impl SkaterAnimation {
    pub(crate) fn gesture_flags(&self)->u32 {self.state.flags}
    pub(crate) fn natural_stance(&self) -> i32 { self.state.publication.natural_stance }
    ///82592B68: physical leading foot, including the fakie inversion.
    pub fn checkpoint_stance(&self) -> u32 {
        let p = &self.state.publication;
        u32::from(((p.natural_stance == 0 && p.relative_stance == 0)
            || (p.natural_stance == 1 && p.relative_stance == 1)) ^ self.state.fakie())
    }
    ///82592C08/82B97350: save15200; ResetToGivenStance consumes it later.
    pub fn request_checkpoint_stance(&mut self, foot: u32) {
        let natural = self.state.publication.natural_stance;
        self.motion.animation.requested_stance = u32::from(if foot == 0 { natural != 1 } else { natural == 1 });
    }
    /// Manual markers use the same native leading-foot query as bail checkpoints.
    pub fn foot_forward(&self) -> bool {
        self.checkpoint_stance() != 0
    }
    /// Queue the native stance request; the teleport graph applies it after reset.
    pub fn restore_foot_forward(&mut self, forward: bool) {
        self.request_checkpoint_stance(u32::from(forward));
        // A second Actor target can arrive while Motion.Teleport is already
        // active: its OnEnter will not run GivenStance again. Apply the native
        // reset pose now with a COPY of the request, retaining the graph's real
        // one-shot request for its ordinary dispatch. ResetSkaterFlags82B98050
        // clears outgoing fakie/weight bits before GivenStance82B97308.
        self.state.flags &= 0x0ff7_ffff;
        let mut requested = self.motion.animation.requested_stance;
        let mut relative = self.state.publication.relative_stance != 0;
        skate3_core::player::offboard::reset_animation::given_stance(
            self.state.publication.natural_stance as u32, &mut requested,
            &mut self.state.flags, &mut relative,
        );
        self.state.publication.relative_stance = i32::from(relative);
        self.motion.animation.relative_stance = u32::from(relative);
        self.motion.animation.skater_animation_flags = Some(self.state.flags);
    }
    /// Native Initialize82B97E38 selects both orientation/mirror bits for
    /// regular stance. A profile edit changes the natural basis while retaining
    /// the current relative stance and trick state.
    pub(crate) fn set_natural_stance(&mut self, natural: u32) {
        if natural <= 1 && self.state.publication.natural_stance != natural as i32 {
            self.state.publication.natural_stance = natural as i32;
            self.state.flags ^= 0xC000_0000;
        }
    }
    pub(crate) fn set_customisation(&mut self, natural: u32, style: u32) {
        self.set_natural_stance(natural);
        // GetCACSettings82590D20..D84: 0=null,1=Loose,2=Gonzo,3=Aggressive.
        let name: &[u8] = match style { 1 => b"Loose", 2 => b"Gonzo", 3 => b"Aggressive", _ => b"" };
        self.motion.playback_context.pro_skater = encode(name);
    }
    /// GetCACSettings82590D20/82590B50: profile selection is independent of
    /// the authored Play Begin latch that wraps eligible motion trees.
    pub(crate) fn set_style_preferences(&mut self, settings: skate2_formats::skater_profile::PcSkate3Style) -> Result<(),String> {
        settings.validate().map_err(|e|e.to_string())?;
        self.set_customisation(self.natural_stance() as u32, settings.style);
        self.motion.animation.posture.set_profile(settings.posture);
        Ok(())
    }
    pub fn stance(&self) -> (bool, bool) {
        (self.state.fakie(), self.state.mirrored())
    }
    /// Actor8259147C evaluates the stored initialization pose before creating
    /// the physical skeleton. Initialize82B97E38 selects rig_Tpose;
    /// 82D181D0 stores it at13484, and82531050 emits that pose command.
    /// This is separate from both the graph's first update and AnimOut reset.
    pub fn evaluate_initial_pose(&mut self) -> Result<Vec<output::NativeMatrix>, String> {
        self.pose = self.evaluator.evaluate(&[PoseCommand::Pose {
            name: "RIG_TPOSE".into(),
        }])?;
        self.evaluator.hierarchy(&self.pose)
    }

    pub fn load(
        root: &Path,
        data: &Collections,
        graphs: &StockGraphs,
        pro_skater: &[u8],
    ) -> Result<Self, String> {
        Self::from_source(data, graphs, pro_skater, AnimationSource::load(root)?)
    }

    pub fn from_source(
        data: &Collections, graphs: &StockGraphs, pro_skater: &[u8], source: Arc<AnimationSource>,
    ) -> Result<Self, String> {
        let evaluator = source.evaluator.clone();
        let state = state::AnimationState::new(true);
        let count = evaluator.frames.bone_names.len();
        let mut motion = MotionHost::from_graph(
            &graphs.motion,
            data,
            source.banks.metadata()?,
            PlaybackContext {
                is_switch: Some(state.switch()),
                is_mirrored: Some(state.mirrored()),
                board_available: None,
                pro_skater: encode(pro_skater),
                transition_override: None,
            },
        )
        .map_err(|error| format!("Stock MotionGraph initialization: {error}"))?;
        motion.animation.set_hierarchy(
            &evaluator.frames.bone_names,
            &evaluator.frames.mirror_indices,
        )?;
        //82858810 registers these three static poses in this exact order.
        use skate3_core::animation::posture::PosturePose;
        for posture in [PosturePose::Stiff, PosturePose::Slouch, PosturePose::Buff] {
            let pose = evaluator.frames.named_pose(posture.name())?;
            if pose.samples.len() != count {
                return Err(format!(
                    "Stock posture {} has {} bones; skater hierarchy has {count}",
                    posture.name(),
                    pose.samples.len()
                ));
            }
        }
        motion.animation.posture_bank_valid = true;
        motion.animation.skater_animation_flags = Some(state.flags);
        Ok(Self {
            action: ActionHost::from_graph(&graphs.action, data)
                .map_err(|error| format!("Stock ActionGraph initialization: {error}"))?,
            motion,
            evaluator,
            source,
            pose: Vec::new(),
            body_commands: Vec::new(),
            attributes: PacketAttributes::default(),
            state,
            action_controller: Controller::new(graphs.action.runtime.program.topology.states.len()),
            motion_controller: Controller::new(graphs.motion.runtime.program.topology.states.len()),
            //Represented pose fields after AnimOutPhysIn Reset82590028.
            packet: PhysicsPosePacket {
                bone_count: count as u32,
                hierarchy: vec![RESET_POSE; count],
                local: vec![RESET_POSE; count],
                timestep: f32::from_bits(0x3c88_8889),
                foot_surface_ids: [0; 2],
                flags: 0,
                board_flipped: false,
                mirrored: false,
                riding_switch: false,
                riding_fakie: false,
                weight_forwards: false,
                regular_stance: false,
                air_dismount_revert_frames: 0,
            },
            ticks: 0,
        })
    }

    ///Actor Phase1/2/3 plus SetUpPhysics:82592FD8,82593128,82593230,82593640.
    ///GenerateActionGraphIntents has completed before this call. Keep one MG
    ///intent map across both controllers; native preupdate clears only attrs.
    pub fn advance(
        &mut self,
        graphs: &StockGraphs,
        dt: f32,
        action_intents: &IntentMap,
        physical: AnimationPhysical,
        reset_fields: &mut AdditionalResetFields,
    ) -> Result<(), String> {
        let tick = self.ticks;
        self.publish_physical(physical)?;
        self.action.prepare_input(ActionGraphInput {
            tick,
            controls: ActionInput::from_values(action_intents),
            prior_motion: PriorMotionState::from_values(&self.motion.animation.motion_intents),
            //HasAnimAttribute82BA3A78 reads the preceding cached animation
            //attributes. Specific flag27 is observed before this frame's MG update.
            animation_attributes: self.motion.animation.tree_attributes().to_vec(),
        });
        self.action.is_tricking = Some(self.motion.flags.doing_trick);
        let previous_states = (self.action_controller.frame.current, self.motion_controller.frame.current);
        self.action_controller
            .update(&graphs.action.runtime.program, dt, &mut self.action);
        if !self.action.errors.is_empty() {
            return Err(self.action.errors.join("\n"));
        }
        let action_output = self.action.output();
        self.motion.animation.begin_graph_update();
        self.motion.accept_action_graph(MotionGraphInput {
            tick: action_output.tick,
            action: action_output,
        });
        self.motion.animation_phase = self.state.phase;
        self.motion_controller
            .update(&graphs.motion.runtime.program, dt, &mut self.motion);
        if crate::diagnostics::test_log::enabled() {
            let states = (self.action_controller.frame.current, self.motion_controller.frame.current);
            if states.0 != previous_states.0 {
                crate::diagnostics::test_log::event("skate3_action_state", format_args!(
                    "tick={tick} {} <- {}", state_path(&graphs.action.binding, states.0),
                    state_path(&graphs.action.binding, previous_states.0)));
            }
            if states.1 != previous_states.1 {
                crate::diagnostics::test_log::event("skate3_motion_state", format_args!(
                    "tick={tick} {} <- {}", state_path(&graphs.motion.binding, states.1),
                    state_path(&graphs.motion.binding, previous_states.1)));
            }
        }
        self.state.phase = self.motion.animation_phase;
        if !self.motion.errors.is_empty() {
            return Err(self.motion.errors.join("\n"));
        }
        self.state.flags = self
            .motion
            .animation
            .skater_animation_flags
            .ok_or("MotionGraph lost the SkaterAnim flag owner")?;
        if let Some(frames) = self.motion.animation.air_dismount_revert_frames.take() {
            self.state.publication.air_dismount_revert_frames = frames;
        }
        self.state.publication.relative_stance = self.motion.animation.relative_stance as i32;
        if std::mem::take(&mut self.motion.animation.reset_action_intents) {
            self.action.action_intents.clear();
        }

        self.motion.animation.apply_parameters()?;
        self.motion.animation.advance(dt, self.state.phase);
        //Collect before pose evaluation consumes clip history;82B98980 then
        //uses those exact records for board/mirror/switch event publication.
        self.motion.animation.refresh_tree_attributes()?;
        self.state
            .apply_stance_events(self.motion.animation.tree_attributes());
        let commands = self.motion.animation.evaluate_pose(Evaluation {
            cull_threshold: self.state.cull_threshold(),
            update_history: true,
        })?;
        self.pose = self.evaluator.evaluate(&commands)?;
        self.body_commands = commands;
        let hierarchy = self.evaluator.hierarchy(&self.pose)?;
        let local: Vec<_> = self
            .pose
            .iter()
            .copied()
            .map(output::sqt_to_matrix)
            .collect();
        self.attributes.replace_from(
            &self.motion.animation.motion_attributes,
            self.motion.animation.tree_attributes(),
        );
        self.state.prepare_publication();
        // Actor SetUpPhysics82593640 resets the output after animation has
        // evaluated, before publishing this pose and the actor-owned fields.
        packet_reset::reset(&mut self.packet, reset_fields)
            .map_err(|error| format!("Animation packet reset: {error:?}"))?;
        physics_packet::publish_evaluated(
            &mut self.state.publication,
            &hierarchy,
            &local,
            &mut self.packet,
            //Actor825936AC..D4 passes this fixed physical step even when
            //the graph/clip clock differs or camera slow motion is active.
            f64::from(f32::from_bits(0x3c888889)),
            b"signup",
        )
        .map_err(|e| format!("Skater pose publication: {e:?}"))?;
        self.state.finish_publication();
        self.motion.animation.skater_animation_flags = Some(self.state.flags);
        self.ticks += 1;
        Ok(())
    }

    fn publish_physical(&mut self, p: AnimationPhysical) -> Result<(), String> {
        self.motion.animation.natural_stance = self.state.publication.natural_stance as u32;
        self.motion.animation.relative_stance = self.state.publication.relative_stance as u32;
        let speed = p
            .conditions
            .speeds
            .ok_or("Animation requires completed board speed output")?;
        let stance = (self.state.fakie(), self.state.mirrored());
        self.action.stance = Some(stance);
        self.action.condition_inputs = p.conditions.clone();
        self.motion.condition_inputs = p.conditions;
        self.motion.playback_context.is_mirrored = Some(self.state.mirrored());
        self.motion.playback_context.is_switch = Some(self.state.switch());
        self.motion.playback_context.board_available = Some(p.board_present);
        self.motion.physical = Some(MotionPhysical {
            turning: p.feedback.turning,
            stance,
            forward_speed: speed.forward_speed,
            time_since_teleport: p.time_since_teleport,
            is_switch: self.state.switch(),
            foot_frame: Some(p.foot_frame),
        });
        self.motion.physical_stance = Some(p.physical_stance);
        self.motion.crouching_physical = Some(p.feedback.crouching);
        self.motion.body_tilt_physical = Some(p.body_tilt);
        self.motion.fakie_physical = Some(p.fakie);
        self.motion.pumping_acceleration = Some(p.feedback.pumping_acceleration);

        //8259303C..78: on-board category keeps this true even when the
        //off-board owner's board-present byte is false. 82B971A8 copies it
        //straight into flag17. It is not a no-board bit.
        let board_attached_or_onboard = p.board_present || !p.physical_28_byte75;
        self.state.flags =
            (self.state.flags & !(1 << 17)) | (u32::from(board_attached_or_onboard) << 17);
        self.motion.animation.skater_animation_flags = Some(self.state.flags);
        Ok(())
    }
}

/// A graph state's dotted path below the root, for the state-change trace.
fn state_path(binding: &skate_data::state_graph::binding::Binding, state: Option<usize>) -> String {
    let mut names = Vec::new();
    let mut cursor = state;
    while let Some(id) = cursor {
        names.push(binding.states[id].name.as_str());
        cursor = binding.states[id].parent;
    }
    names.reverse();
    names.join(".")
}
