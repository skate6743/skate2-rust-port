//! ACS acceptance and retained fetch/SQT operation data for body dispatch.
use super::super::bind_pose::BindPoseCommand as Command;
use super::{
    ActorFaceError, AndaleHierarchy, AndaleSqt, AnimationPoseRef, Arc, BindPoseError,
    BufferedBodyCommands, Mutex,
};
use crate::{
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_face_playback::tree_database::clip_fetch::ClipFetchPlan,
};

#[path = "../pose/actor_animation_external_sqt_commands.rs"]
mod external;
#[path = "../pose/actor_animation_fixed_sqt_commands.rs"]
mod fixed;
#[path = "../pose/actor_animation_uniform_scale.rs"]
mod uniform_scale;
#[path = "actor_animation_nblend.rs"]
pub(crate) mod nblend;
#[path = "actor_animation_tree_eval_numeric.rs"]
pub(crate) mod numeric;
#[path = "../runtime/actor_animation_reuse_buffer.rs"]
pub(crate) mod reuse;

#[derive(Clone)]
pub(crate) enum Fetch {
    Pose(AnimationPoseRef),
    Clip(ClipFetchPlan),
}
#[derive(Clone)]
pub(crate) struct TreeCommands {
    pub(crate) owner: Arc<Mutex<BufferedBodyCommands>>,
    pub(crate) fetches: Vec<(u16, Fetch)>,
    pub(crate) operations: Vec<Command>,
    pub(crate) nblends: Vec<nblend::NBlendRecord>,
    pub(crate) external_sqts: Vec<(
        u16,
        super::super::bind_pose::sqt_output::OwnedAnimationSqtBuffer,
    )>,
    external_set_count: usize,
    stack: Vec<u16>,
    reuse: reuse::ReuseState,
    next_sqt: u16,
    pub(crate) fetch_code: crate::animation_job_owners::AnimationJobCode,
    pub(crate) allocation_start: usize,
    fixed_owner: Option<Arc<Mutex<crate::actor_face_playback::FaceCommands>>>,
}
impl TreeCommands {
    /// Typed LoadCmdBufferInfo8280965c: replay accepted operands and slots on
    /// the PRES ACS, without evaluating the graph or advancing clip clocks.
    /// Pedestrian callback3 replaces SIM's identity SQT with post-adjust data.
    pub(crate) fn replay_pedestrian(&self,
        commands: &Arc<Mutex<crate::actor_face_playback::FaceCommands>>,
        post_adjust: &super::super::bind_pose::sqt_output::OwnedAnimationSqtBuffer,
    ) -> Result<Self, BindPoseError> {
        let mut replay = self.clone();
        replay.owner = commands.lock().map_err(|_| ActorFaceError::Lock)?
            .buffered.clone().ok_or(BindPoseError::InitializationRequired)?;
        replay.fixed_owner = self.fixed_owner.as_ref().map(|_| Arc::clone(commands));
        for (_, fetch) in &mut replay.fetches {
            if let Fetch::Clip(plan) = fetch {
                *plan = plan.replay_on(commands)
                    .map_err(|_| BindPoseError::UnsupportedSetup)?;
            }
        }
        if replay.external_sqts.len() > 1 {
            return Err(BindPoseError::UnsupportedSetup);
        }
        for (_, source) in &mut replay.external_sqts { *source = post_adjust.clone(); }
        Ok(replay)
    }
    /// BeginCommands82a229c0 preserves the command-set dynamic allocation.
    /// EndCommands82a289e0 leaves mNum at the prior stream's high-water mark.
    /// Reuse carries its command-set count and the final current allocation
    /// cursor; the queued scheduler preserves each emitted boundary.
    pub(crate) fn new_stream(owner: Arc<Mutex<BufferedBodyCommands>>)
        -> Result<Self, BindPoseError> {
        let (start, prior_segments) = {
            let buffered = owner.lock().map_err(|_| ActorFaceError::Lock)?;
            buffered.sqt_programs.last().map_or((0, 0), |program|
                (program.commands.allocation_end(), program.commands.reuse_count()))
        };
        let mut result = Self::new(owner);
        result.next_sqt = u16::try_from(start).map_err(|_| BindPoseError::UnsupportedSetup)?;
        result.reuse.high_water = start;
        result.reuse.prior_segments = prior_segments;
        result.allocation_start = start;
        Ok(result)
    }
    pub(crate) fn new(owner: Arc<Mutex<BufferedBodyCommands>>) -> Self {
        Self {
            owner,
            fetches: vec![],
            operations: vec![],
            nblends: vec![],
            external_sqts: vec![],
            external_set_count: 0,
            stack: vec![],
            reuse: reuse::ReuseState::default(),
            next_sqt: 0,
            fetch_code: crate::animation_job_owners::AnimationJobCode::FetchRaw,
            allocation_start: 0,
            fixed_owner: None,
        }
    }
    pub(crate) fn output(&self) -> Result<u16, BindPoseError> {
        if self.stack.len() != 1 {
            return Err(BindPoseError::UnsupportedSetup);
        }
        Ok(self.stack[0])
    }
    /// ExtractData82a24400 selects the stack's LAST producer. EndCommands82a289e0
    /// then resets the command/graph stacks; it does not require depth one.
    /// Keep every emitted operation, retaining only the extracted root for the
    /// host program's deferred output lookup after the native stream ends.
    pub(crate) fn select_stream_extraction_root(&mut self) -> Result<u16, BindPoseError> {
        let output = *self
            .stack
            .last()
            .ok_or(BindPoseError::TreeStackUnderflow("ExtractSQT"))?;
        self.stack.clear();
        self.stack.push(output);
        Ok(output)
    }
    fn push_fetch(&mut self, fetch: Fetch, guid: u16) -> Result<(), BindPoseError> {
        // Buffered GetNextLoc82a24c28 allocates monotonically. The immediate
        // Blend branch's stack release must not be imposed on buffered mode.
        let sqt = self.allocate_dynamic()?;
        let command = match &fetch {
            Fetch::Pose(_) => Command::FetchPose { guid, sqt },
            Fetch::Clip(plan) => Command::FetchClip {
                guid,
                sqt,
                timing: plan.timing,
            },
        };
        self.owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .commands
            .push(command);
        self.fetches.push((sqt, fetch));
        self.produced(&[], sqt);
        self.stack.push(sqt);
        Ok(())
    }
    fn binary(
        &mut self,
        reverse: bool,
        build: impl FnOnce(u16, u16, u16) -> Command,
    ) -> Result<(), BindPoseError> {
        let second = self
            .stack
            .pop()
            .ok_or(BindPoseError::TreeStackUnderflow("binary second"))?;
        let first = self
            .stack
            .pop()
            .ok_or(BindPoseError::TreeStackUnderflow("binary first"))?;
        let (first, second) = if reverse {
            (second, first)
        } else {
            (first, second)
        };
        let output = self.binary_output(first, second)?;
        let command = build(first, second, output);
        self.owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .commands
            .push(command);
        self.operations.push(command);
        self.produced(&[first, second], output);
        self.stack.push(output);
        Ok(())
    }
}
impl AnimationEvalCommands for TreeCommands {
    type Error = BindPoseError;
    fn fixed_sqt(
        &mut self,
        owner: &Arc<Mutex<crate::actor_face_playback::FaceCommands>>,
        id: u32,
    ) -> Option<Result<(), Self::Error>> {
        Some(self.set_fixed_sqt(owner, id))
    }
    fn external_sqt(&mut self, source:&crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer)
        ->Option<Result<(),Self::Error>> {Some(self.set_external_sqt(source))}
    fn fetch(&mut self, plan: ClipFetchPlan) -> Result<(), Self::Error> {
        let guid = plan.guid;
        self.push_fetch(Fetch::Clip(plan), guid)
    }
    fn fetch_pose(&mut self, pose: &AnimationPoseRef) -> Result<(), Self::Error> {
        self.push_fetch(Fetch::Pose(pose.clone()), pose.guid()?)
    }
    fn n_blend(&mut self, coefficients: &[f32]) -> Result<(), Self::Error> {
        self.emit_n_blend(coefficients)
    }
    fn blend(&mut self, coefficient: f32) -> Result<(), Self::Error> {
        self.count_other_sqt()?;
        let coefficient = encode_coefficient(coefficient)?;
        self.binary(false, |first, second, output| Command::Blend {
            first,
            second,
            output,
            coefficient,
        })
    }
    fn channel_blend(
        &mut self,
        coefficient: f32,
        channel_from_first: bool,
    ) -> Result<(), Self::Error> {
        self.count_other_sqt()?;
        let coefficient = encode_coefficient(coefficient)?;
        self.binary(false, |first, second, output| Command::ChannelBlend {
            channel_from_first,
            first,
            second,
            output,
            coefficient,
        })
    }
    fn add_sqt(&mut self, motion_a: bool) -> Result<(), Self::Error> {
        self.count_other_sqt()?;
        // Native AddSQT(true, LeftToRight) reverses both stack operands and
        // flips ChannelFromFirstOperand when LeftToRight is false82a2d6d4..6fc.
        self.binary(!motion_a, |first, second, output| Command::AddSqt {
            first,
            second,
            output,
            channel_from_first: motion_a,
        })
    }
    fn mirror(&mut self, trajectory_use: u32) -> Result<(), Self::Error> {
        self.count_other_sqt()?;
        let input = self
            .stack
            .pop()
            .ok_or(BindPoseError::TreeStackUnderflow("mirror"))?;
        // Mirror82a2e098..e110 pops the command input before GetNextLoc.
        // Fixed SetData/reuse inputs require a NEW dynamic destination;
        // dynamic inputs are mirrored in place. The fixed frame is read-only.
        let output = if input & fixed::FIXED_SQT_BIT != 0 {
            self.allocate_dynamic()?
        } else {
            input
        };
        let command = Command::Mirror {
            input,
            output,
            trajectory_use,
        };
        self.owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .commands
            .push(command);
        self.operations.push(command);
        self.produced(&[input], output);
        self.stack.push(output);
        Ok(())
    }
}
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "source floor then low13 command field"
)]
pub(crate) fn encode_coefficient(coefficient: f32) -> Result<u16, BindPoseError> {
    if !coefficient.is_finite() || coefficient < -0.0001 || coefficient > 1.0001 {
        return Err(BindPoseError::TreeCoefficientBits(coefficient.to_bits()));
    }
    Ok(((coefficient * 8191.0).floor() as i32 as u32 & 0x1fff) as u16)
}

pub(crate) fn execute_fetch(plan: &ClipFetchPlan) -> Result<Vec<AndaleSqt>, BindPoseError> {
    let (parsed, clip_index) = {
        let database = plan.clip
            .database
            .lock()
            .map_err(|_| ActorFaceError::Lock)?;
        let parsed = database
            .parsed
            .as_ref()
            .ok_or(ActorFaceError::MissingRecord)?;
        let index = database.clip_payload_index(plan.clip.record_index)
            .ok_or(ActorFaceError::MissingRecord)?;
        (std::sync::Arc::clone(parsed), index)
    };
    let clip = parsed
        .clips
        .get(clip_index)
        .ok_or(ActorFaceError::MissingRecord)?;
    let identity = AndaleSqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [
            0.0,
            0.0,
            0.0,
            if plan.channel_animation { 0.0 } else { 1.0 },
        ],
    };
    let mut output = vec![identity; plan.total_bones];
    for extract in &plan.parts {
        let part = clip
            .parts
            .get(extract.part_index)
            .ok_or(ActorFaceError::MissingRecord)?;
        let mut current = sample(part, extract.current)?;
        if current.len() != extract.bone_count {
            return Err(BindPoseError::InvalidHierarchy);
        }
        for (index, bone) in current.iter_mut().enumerate() {
            bone.translation[3] = extract
                .channel_weights
                .as_ref()
                .map_or(1.0, |weights| weights[index]);
        }
        if let Some(previous) = extract.previous {
            let previous = sample(part, previous)?;
            let (rotation, translation) = extract
                .trajectory_delta
                .ok_or(BindPoseError::UnsupportedSetup)?;
            numeric::trajectory(
                &mut current[0],
                previous[0],
                rotation,
                translation,
                extract.channel_weights.as_ref().map(|weights| weights[0]),
            );
        }
        output
            .get_mut(extract.bone_offset..extract.bone_offset + extract.bone_count)
            .ok_or(BindPoseError::InvalidHierarchy)?
            .copy_from_slice(&current);
    }
    Ok(output)
}
pub(crate) fn sample(
    part: &skate2_formats::andale::AndaleClipPart,
    pair: crate::actor_face_playback::tree_database::clip_fetch::timing::FetchFramePair,
) -> Result<Vec<AndaleSqt>, BindPoseError> {
    // AnimFetch RAW uses the existing Vector_ExtractPackedN82d6f838 decoder;
    // frame selection and quantized pair blend remain the shared Fetch policy.
    let decode=|frame|->Result<Vec<AndaleSqt>,BindPoseError>{
        if matches!(&part.compression,skate2_formats::andale::AndaleCompressionHeader::RawDiscretize(_)){
            part.sample_raw_frame_sqt(frame).map_err(|e|BindPoseError::Animation(ActorFaceError::Raw(e)))
        }else{part.sample_vbr_sqt(frame).map_err(|e|BindPoseError::Animation(ActorFaceError::Format(e)))}
    };
    if pair.frames[0] == pair.frames[1] {
        return decode(usize::from(pair.frames[0]));
    }
    let [first, second] = if matches!(&part.compression,
        skate2_formats::andale::AndaleCompressionHeader::RawDiscretize(_)) {
        [decode(usize::from(pair.frames[0]))?, decode(usize::from(pair.frames[1]))?]
    } else {
        // Reuse only the immutable decoded VBR block. Keep this live AnimFetch
        // path's quantized coefficient and numeric::blend graph below intact.
        part.sample_vbr_sqt_pair(usize::from(pair.frames[0]), usize::from(pair.frames[1]))
            .map_err(|e| BindPoseError::Animation(ActorFaceError::Format(e)))?
    };
    let coefficient = f32::from(pair.blend) * f32::from_bits(0x3780_0080);
    Ok(first
        .into_iter()
        .zip(second)
        .map(|(a, b)| numeric::blend(a, b, coefficient))
        .collect())
}

pub(crate) fn execute_operations(
    program: &TreeCommands,
    sqts: &mut [Option<Vec<AndaleSqt>>],
    hierarchy: &AndaleHierarchy,
) -> Result<(), BindPoseError> {
    execute_operation_range(program, sqts, hierarchy, 0..program.operations.len())
}
pub(crate) fn execute_operation_range(
    program: &TreeCommands,
    sqts: &mut [Option<Vec<AndaleSqt>>],
    hierarchy: &AndaleHierarchy,
    range: std::ops::Range<usize>,
) -> Result<(), BindPoseError> {
    for command in &program.operations[range] {
        match *command {
            Command::ReplaceSqt{first,second,output,offset,count}=>{
                let mut destination=program.operand(first,sqts)?.into_owned();
                let source=program.operand(second,sqts)?;
                if destination.len()!=hierarchy.parents.len(){return Err(BindPoseError::InvalidHierarchy);}
                destination.get_mut(usize::from(offset)..usize::from(offset)+usize::from(count))
                    .ok_or(BindPoseError::InvalidHierarchy)?.copy_from_slice(
                        source.get(..usize::from(count)).ok_or(BindPoseError::InvalidHierarchy)?);
                sqts[usize::from(output)]=Some(destination);
            }
            Command::UniformScale { input, output, coefficient, scale_trajectory, root_bone } => {
                let source = program.operand(input, sqts)?.into_owned();
                sqts[usize::from(output)] = Some(uniform_scale::execute(&source, hierarchy,
                    coefficient, scale_trajectory, root_bone)?);
            }
            Command::NBlend {
                list,
                count,
                output,
            } => program.execute_n_blend(list, count, output, sqts, hierarchy.parents.len())?,
            Command::Mirror {
                input,
                output,
                trajectory_use,
            } => {
                if input != output {
                    let source = program.operand(input, sqts)?.into_owned();
                    *sqts
                        .get_mut(usize::from(output))
                        .ok_or(BindPoseError::InvalidHierarchy)? = Some(source);
                }
                let pose = sqts
                    .get_mut(usize::from(output))
                    .ok_or(BindPoseError::InvalidHierarchy)?
                    .as_mut()
                    .ok_or(BindPoseError::MissingBindPose)?;
                numeric::mirror(
                    pose,
                    &hierarchy.parents,
                    &hierarchy.bone_metadata,
                    trajectory_use,
                )
                .map_err(|()| BindPoseError::InvalidHierarchy)?;
            }
            Command::Blend {
                first,
                second,
                output,
                coefficient,
            }
            | Command::ChannelBlend {
                first,
                second,
                output,
                coefficient,
                ..
            } => {
                let a = program.operand(first, sqts)?;
                let b = program.operand(second, sqts)?;
                let coefficient = f32::from(coefficient) * f32::from_bits(0x3900_0400);
                let channel = matches!(command, Command::ChannelBlend { .. });
                let result = a
                    .iter()
                    .zip(b.iter())
                    .map(|(a, b)| {
                        let channel_from_first = matches!(
                            command,
                            Command::ChannelBlend {
                                channel_from_first: true,
                                ..
                            }
                        );
                        let channel_weight = if channel_from_first {
                            a.translation[3]
                        } else {
                            b.translation[3]
                        };
                        let weighted = coefficient * channel_weight;
                        let upper = if weighted > 1.0 { 1.0 } else { weighted };
                        let weight = if channel {
                            if 0.0 > upper { 0.0 } else { upper }
                        } else {
                            coefficient
                        };
                        numeric::blend(*a, *b, weight)
                    })
                    .collect();
                sqts[usize::from(output)] = Some(result);
            }
            Command::AddSqt {
                first,
                second,
                output,
                channel_from_first,
            } => {
                let a = program.operand(first, sqts)?;
                let b = program.operand(second, sqts)?;
                let result = a
                    .iter()
                    .zip(b.iter())
                    .map(|(a, b)| numeric::add_sqt(*a, *b, channel_from_first))
                    .collect();
                sqts[usize::from(output)] = Some(result);
            }
            _ => return Err(BindPoseError::UnsupportedSetup),
        }
    }
    Ok(())
}
