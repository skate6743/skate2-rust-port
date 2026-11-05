//! Production animation command evaluator. File IO and buffer ownership are
//! host services; frame selection, trajectory and pose math live in core.
use skate3_core::animation::{
    output::{self, NativeMatrix, Sqt},
    playback_tree::PoseCommand,
    pose_add, pose_blend, pose_mirror,
    pose_sample::{sample_key, select_frames},
    pose_trajectory::{self, LoopTransform},
};
use skate_data::{
    animation_banks::AnimationBanks,
    animation_frames::{AnimationFrames, ClipFrames, SampleWords},
};
use std::path::Path;
mod authored_clips;

pub(crate) struct PoseEvaluator {
    pub frames: AnimationFrames,
    authored: authored_clips::Replacements,
}

impl PoseEvaluator {
    #[cfg(any())]
    pub fn load(asset_root: &Path) -> Result<Self, String> {
        let mut evaluator = Self::from_banks(&AnimationBanks::load(asset_root)?)?;
        evaluator.load_authored_clips(asset_root)?;
        Ok(evaluator)
    }
    pub fn from_banks(banks: &AnimationBanks) -> Result<Self, String> {
        Ok(Self {
            frames: AnimationFrames::from_banks(banks)?,
            authored: Default::default(),
        })
    }

    pub fn load_authored_clips(&mut self, root: &Path) -> Result<(), String> {
        self.authored = authored_clips::Replacements::load(root, &self.frames)?;
        Ok(())
    }

    /// Executes the ordered stock tree. This uses the native immediate ACS
    /// arithmetic; the host does not recreate packed animation job commands.
    pub fn evaluate(&self, commands: &[PoseCommand]) -> Result<Vec<Sqt>, String> {
        let mut stack: Vec<Vec<Sqt>> = Vec::new();
        for command in commands {
            match command {
                PoseCommand::Pose { name } => {
                    // Init82B97E38 retains these references from the actor's
                    // initial database. AddBindPose82B98118 reuses them even
                    // when a later motion clip comes from another bank.
                    stack.push(
                        self.frames
                            .named_pose(name)?
                            .samples
                            .iter()
                            .copied()
                            .map(sqt)
                            .collect(),
                    );
                }
                PoseCommand::Add { motion_is_a } => {
                    let other = stack
                        .pop()
                        .ok_or("Animation add has no reference subtree")?;
                    let motion = stack
                        .last_mut()
                        .ok_or("Animation add has no motion subtree")?;
                    if motion.len() != other.len() {
                        return Err("Animation add bone counts differ".into());
                    }
                    for (motion, other) in motion.iter_mut().zip(other) {
                        *motion = if *motion_is_a {
                            pose_add::add(*motion, other, true)
                        } else {
                            pose_add::add(other, *motion, false)
                        };
                    }
                }
                PoseCommand::Mirror { trajectory_mode } => {
                    let pose = stack.last_mut().ok_or("Animation mirror has no subtree")?;
                    pose_mirror::mirror(
                        pose,
                        &self.frames.parents,
                        &self.frames.mirror_indices,
                        *trajectory_mode,
                    )?;
                }
                PoseCommand::Clip {
                    name,
                    time,
                    previous_time,
                    loops,
                } => {
                    let stock = self.frames.clip(name)?;
                    let clip = self.authored.clip(&stock.name).unwrap_or(stock);
                    let mut pose = sample_clip(clip, *time)?;
                    if self.frames.has_trajectory {
                        let previous = sample_bone(clip, *previous_time, 0)?;
                        pose[0] = pose_trajectory::delta(
                            pose[0],
                            previous,
                            (*loops != 0).then_some(LoopTransform {
                                rotation: clip.loop_rotation_bits.map(f32::from_bits),
                                translation: clip.loop_translation_bits.map(f32::from_bits),
                            }),
                        );
                    }
                    stack.push(pose);
                }
                PoseCommand::WeightedBlend { weights } => {
                    let start=stack.len().checked_sub(weights.len()).ok_or("Weighted blend pose stack underflow")?;
                    let pose=skate3_core::animation::pose_blend::weighted(&stack[start..],weights)
                        .map_err(|e| format!("Weighted blend: {e:?}"))?;
                    stack.truncate(start); stack.push(pose);
                }
                PoseCommand::Blend { weight } | PoseCommand::ChannelBlend { weight, .. } => {
                    let second = stack.pop().ok_or("Animation blend has no second subtree")?;
                    let first = stack
                        .last_mut()
                        .ok_or("Animation blend has no first subtree")?;
                    if first.len() != second.len() {
                        return Err("Animation subtree bone counts differ".into());
                    }
                    for (a, b) in first.iter_mut().zip(second) {
                        *a = match command {
                            PoseCommand::ChannelBlend {
                                use_channels_from_weights,
                                ..
                            } => pose_blend::channel_blend_sample(
                                *a,
                                b,
                                *weight,
                                *use_channels_from_weights,
                            ),
                            _ => pose_blend::blend_sample(*a, b, *weight),
                        };
                    }
                }
            }
        }
        if stack.len() != 1 {
            return Err(format!("Animation evaluation left{} poses", stack.len()));
        }
        Ok(stack.pop().unwrap())
    }

    /// TU3828D3800 then828D3B58. NewACS824744B8 initializes the
    /// detached parent to0, so trajectory deltas do not move the local rig.
    pub fn hierarchy(&self, pose: &[Sqt]) -> Result<Vec<NativeMatrix>, String> {
        if pose.len() != self.frames.parents.len() {
            return Err("Animation pose and hierarchy bone counts differ".into());
        }
        let mut globals: Vec<_> = pose.iter().copied().map(output::sqt_to_matrix).collect();
        output::compose_hierarchy_in_place(
            globals.len() as i32,
            &self.frames.parents,
            0,
            &mut globals,
        )
        .map_err(|e| format!("Invalid stock animation hierarchy: {e:?}"))?;
        Ok(globals)
    }
}

fn sample_clip(clip: &ClipFrames, time: f32) -> Result<Vec<Sqt>, String> {
    let selection = select_frames(
        time,
        f32::from_bits(clip.fps_bits),
        clip.frames.len(),
        true,
        0.0,
    )?;
    Ok(clip.frames[selection.first]
        .iter()
        .zip(&clip.frames[selection.second])
        .enumerate()
        .map(|(bone, (&first, &second))| {
            // ClipFrames already contains independently decoded keys. The raw
            // VBR block-boundary compatibility path substitutes the next key,
            // which skips then holds a frame when applied to this flat cache.
            let mut sample = sample_key(sqt(first), sqt(second), selection);
            sample.translation[3] = f32::from_bits(clip.channel_weights[bone]);
            sample
        })
        .collect())
}

fn sample_bone(clip: &ClipFrames, time: f32, bone: usize) -> Result<Sqt, String> {
    let selection = select_frames(
        time,
        f32::from_bits(clip.fps_bits),
        clip.frames.len(),
        true,
        0.0,
    )?;
    let mut sample = sample_key(
        sqt(clip.frames[selection.first][bone]),
        sqt(clip.frames[selection.second][bone]),
        selection,
    );
    sample.translation[3] = f32::from_bits(clip.channel_weights[bone]);
    Ok(sample)
}

fn sqt(words: SampleWords) -> Sqt {
    let [sx, sy, sz, x, y, z, w, tx, ty, tz] = words.map(f32::from_bits);
    Sqt {
        scale: [sx, sy, sz, 1.0],
        rotation: [x, y, z, w],
        translation: [tx, ty, tz, 1.0],
    }
}

#[cfg(any())]
#[path = "tests/animation_pose.rs"]
mod tests;
