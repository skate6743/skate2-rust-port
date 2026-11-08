//! Production animation command evaluator. File IO and buffer ownership are
//! host services; frame selection, trajectory and pose math live in core.
use skate3_core::animation::{
    output::{self, NativeMatrix, Sqt},
    playback_tree::PoseCommand,
    pose_add, pose_blend, pose_mirror,
    pose_sample::{FrameSelection, sample_key, sample_key_vbr, select_frames},
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
                    let (previous_time, time) = fetch_times(*previous_time, *time, *loops);
                    trace_clip(clip, time, previous_time);
                    let mut pose = sample_clip(clip, time)?;
                    if self.frames.has_trajectory {
                        let previous = sample_bone(clip, previous_time, 0)?;
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
            // Every stock clip is VBR. For a pair across an 8-frame block (8k+7, 8k+8)
            // TU3 decodes the second key for both (Vector_ExtractPackedNVBR 82D1E0C0..E0,
            // DecompressTwoConsecFrames 82E89BD0..BF4): the pose holds frame 8k+8.
            let mut sample = pair_sample(sqt(first), sqt(second), selection);
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
    let mut sample = pair_sample(
        sqt(clip.frames[selection.first][bone]),
        sqt(clip.frames[selection.second][bone]),
        selection,
    );
    sample.translation[3] = f32::from_bits(clip.channel_weights[bone]);
    Ok(sample)
}

/// The times a clip is actually sampled at. `AnimCommandSystem::Fetch` 0x828CAA00 packs the
/// evaluated times into the fetch command and `FetchEngine::EnQueueCmd` 0x828D7B34 unpacks them
/// for `BatchFetch`: the previous time floored to 1/1456.35 s (18 bits); the current time as the
/// floored step from that in 1/1023.5 s (11 bits), or on its own after the clip looped. A step no
/// longer than one previous unit packs as zero. Both land a hair below the evaluated times, so
/// whole-frame times sample just under the frame (no VBR block pair at an exact boundary).
fn fetch_times(previous_time: f32, time: f32, loops: u32) -> (f32, f32) {
    const PREVIOUS_SCALE: f32 = f32::from_bits(0x44b6_0b33);
    const CURRENT_SCALE: f32 = f32::from_bits(0x447f_e000);
    // Runtime reciprocals 0x83035308 / 0x8303530C.
    const PREVIOUS_UNIT: f32 = f32::from_bits(0x3a34_002d);
    const CURRENT_UNIT: f32 = f32::from_bits(0x3a80_1002);
    // fmuls, floor, frsp, fctidz; the command keeps the low bits.
    let pack = |value: f32, bits: u32| (f64::from(value).floor() as i64 as u64 & ((1 << bits) - 1)) as f32;
    let previous_units = pack(previous_time * PREVIOUS_SCALE, 18);
    let previous = previous_units * PREVIOUS_UNIT;
    let current = if loops != 0 {
        pack(time * CURRENT_SCALE, 11) * CURRENT_UNIT
    } else {
        let step = time - previous;
        let step = if step.abs() > PREVIOUS_UNIT { step } else { 0.0 };
        pack(step * CURRENT_SCALE, 11).mul_add(CURRENT_UNIT, previous)
    };
    (previous, current)
}

/// `SKATE2_TRACE_SKATE3_CLIP=<name part>`: each sampled clip whose name contains it, with its time,
/// previous time and the frame pair both select (bit-exact positions).
fn trace_clip(clip: &ClipFrames, time: f32, previous_time: f32) {
    static FILTER: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    let Some(filter) = FILTER.get_or_init(|| std::env::var("SKATE2_TRACE_SKATE3_CLIP").ok()) else {
        return;
    };
    if !clip.name.contains(filter.as_str()) {
        return;
    }
    let fps = f32::from_bits(clip.fps_bits);
    let pick = |t: f32| select_frames(t, fps, clip.frames.len(), true, 0.0).ok().map(|s| (s.first, s.second, s.coefficient));
    eprintln!(
        "skate3 clip: {} time={:.9} ({:#010x}) previous={:.9} position={:.7} pair={:?} previous_pair={:?}",
        clip.name, time, time.to_bits(), previous_time, time * fps, pick(time), pick(previous_time),
    );
}

/// `SKATE2_DIAGNOSTIC_VBR_PAIR=linear` interpolates every pair without the block rule (A/B tests).
fn pair_sample(first: Sqt, second: Sqt, selection: FrameSelection) -> Sqt {
    static LINEAR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *LINEAR.get_or_init(|| std::env::var("SKATE2_DIAGNOSTIC_VBR_PAIR").is_ok_and(|v| v == "linear")) {
        sample_key(first, second, selection)
    } else {
        sample_key_vbr(first, second, selection)
    }
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

#[cfg(test)]
mod block_boundary_tests {
    use super::*;
    fn clip(frames: usize) -> ClipFrames {
        let one = 1f32.to_bits();
        ClipFrames {
            name: "BOUNDARY".into(),
            source_offset: 0,
            fps_bits: 30f32.to_bits(),
            loop_translation_bits: [0; 3],
            loop_rotation_bits: [0, 0, 0, one],
            channel_animation: false,
            channel_weights: vec![one],
            frames: (0..frames)
                .map(|f| vec![[one, one, one, 0, 0, 0, one, (f as f32).to_bits(), 0, 0]])
                .collect(),
        }
    }
    #[test]
    fn fetch_times_floor_whole_frames_below_the_frame() {
        let tick = f32::from_bits(0x3c88_8889);
        let sum = |n: u32| (0..n).fold(0.0f32, |t, _| t + tick);
        // Frame 7 of a 60 fps clip (TU3 BR_RUN_FWD_50_INTO_MOUNT on the seventh tick).
        let (previous, time) = fetch_times(sum(6), sum(7), 0);
        assert_eq!(previous.to_bits(), (145.0f32 * f32::from_bits(0x3a34_002d)).to_bits());
        assert!(time * 60.0 < 7.0 && time * 60.0 > 6.95);
        let (_, next) = fetch_times(sum(7), sum(8), 0);
        assert!(next * 60.0 < 8.0 && next * 60.0 > 7.95);
        // A looped clip packs its time on its own.
        let (_, looped) = fetch_times(0.5, 0.1, 1);
        assert_eq!(looped.to_bits(), (102.0f32 * f32::from_bits(0x3a80_1002)).to_bits());
        // A step no longer than one previous unit packs as zero.
        let (previous, still) = fetch_times(0.25, 0.25, 0);
        assert_eq!(previous.to_bits(), still.to_bits());
    }
    #[test]
    fn a_pair_across_a_vbr_block_holds_the_second_key() {
        let clip = clip(16);
        let x = |frame: f32| sample_bone(&clip, frame / 30.0, 0).unwrap().translation[0];
        assert!((x(6.5) - 6.5).abs() < 1e-3);
        assert!((x(7.25) - 8.0).abs() < 1e-5);
        assert!((x(7.75) - 8.0).abs() < 1e-5);
        assert!((x(8.5) - 8.5).abs() < 1e-3);
        let pose = sample_clip(&clip, 7.5 / 30.0).unwrap();
        assert!((pose[0].translation[0] - 8.0).abs() < 1e-5);
    }
}
