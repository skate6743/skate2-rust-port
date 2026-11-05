//! Owned animation tree evaluation; stock data constructs this topology.
pub mod blend_space;
pub mod selection_space;
use super::{
    clip_clock::AdvanceResult,
    output::attributes::{AnimationAttribute, AttributeName},
    phase_blend::PhaseBlend,
    playback_clip::PlaybackClip,
    playback_parameters::SettableAttribute,
};

#[derive(Clone, Debug)]
pub enum PlaybackTree {
    Clip {
        name: String,
        clip: PlaybackClip,
    },
    PhaseBlend(PhaseBlend),
    BlendSpace(blend_space::BlendSpace),
    SelectionSpace(selection_space::SelectionSpace),
    Transition(super::playback_transition::PlaybackTransition),
    ///SkaterAnim::AddBindPose82B98118, latched when a named tree is created.
    BindPose {
        motion: Box<PlaybackTree>,
        posture: Option<super::posture::PosturePose>,
        board_backwards: bool,
        mirror_modes: Vec<u32>,
        attribute_mirror: std::sync::Arc<super::playback_attributes::AttributeMirror>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum PoseCommand {
    ///82D25B00 supplies both bounded source times and the unconsumed wrap count.
    Clip {
        name: String,
        previous_time: f32,
        time: f32,
        loops: u32,
    },
    ///Blend the two most recent subtree poses, preserving authored nesting.
    Blend {
        weight: f32,
    },
    WeightedBlend {
        weights: Vec<f32>,
    },
    ChannelBlend {
        weight: f32,
        use_channels_from_weights: bool,
    },
    Pose {
        name: String,
    },
    Add {
        motion_is_a: bool,
    },
    Mirror {
        trajectory_mode: u32,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Evaluation {
    pub cull_threshold: f32,
    pub update_history: bool,
}
impl PlaybackTree {
    pub fn length(&self) -> f32 {
        match self {
            Self::Clip { clip, .. } => clip.clock.length,
            Self::PhaseBlend(tree) => tree.length,
            Self::BlendSpace(tree) => tree.length(),
            Self::SelectionSpace(tree) => tree.current().map_or(0.0, Self::length),
            Self::Transition(tree) => tree.to.length(),
            Self::BindPose { motion, .. } => motion.length(),
        }
    }
    pub fn time(&self) -> f32 {
        match self {
            Self::Clip { clip, .. } => clip.clock.sample_time(),
            Self::PhaseBlend(tree) => tree.time,
            Self::BlendSpace(tree) => tree.time,
            Self::SelectionSpace(tree) => tree.current().map_or(0.0, Self::time),
            Self::Transition(tree) => tree.to.time(),
            Self::BindPose { motion, .. } => motion.time(),
        }
    }
    pub fn set_time(&mut self, time: f32) {
        match self {
            Self::Clip { clip, .. } => clip.clock.set_time(time),
            Self::PhaseBlend(tree) => tree.set_time(time),
            Self::BlendSpace(tree) => tree.set_time(time),
            Self::SelectionSpace(tree) => tree.set_time(time),
            Self::Transition(tree) => tree.to.set_time(time),
            Self::BindPose { motion, .. } => motion.set_time(time),
        }
    }
    pub fn set_speed(&mut self, speed: f32) {
        match self {
            Self::Clip { clip, .. } => clip.clock.set_speed(speed),
            Self::PhaseBlend(tree) => tree.set_speed(speed),
            Self::BlendSpace(tree) => tree.set_speed(speed),
            Self::SelectionSpace(tree) => tree.set_speed(speed),
            Self::Transition(tree) => tree.to.set_speed(speed),
            Self::BindPose { motion, .. } => motion.set_speed(speed),
        }
    }
    pub fn advance(&mut self, dt: f32, phase: f32, property: &mut AdvanceResult) {
        match self {
            Self::Clip { clip, .. } => clip.clock.advance(dt, phase, property),
            Self::PhaseBlend(tree) => tree.advance(dt, phase, property),
            Self::BlendSpace(tree) => tree.advance(dt, phase, property),
            Self::SelectionSpace(tree) => {
                if let Some(child) = tree.current_mut() {
                    child.advance(dt, phase, property);
                }
            }
            Self::Transition(tree) => tree.advance(dt, phase, property),
            Self::BindPose { motion, .. } => motion.advance(dt, phase, property),
        }
    }
    pub fn set_attributes(&mut self, attributes: &[SettableAttribute]) -> Result<bool, String> {
        match self {
            Self::Clip { .. } => Ok(false),
            Self::PhaseBlend(tree) => tree.set_attributes(attributes),
            Self::BlendSpace(tree) => tree.set_attributes(attributes),
            Self::SelectionSpace(tree) => tree.set_attributes(attributes),
            Self::Transition(tree) => tree.set_attributes(attributes),
            Self::BindPose { motion, .. } => motion.set_attributes(attributes),
        }
    }
    pub fn attributes(&self, mask: u32) -> Result<Vec<AnimationAttribute>, String> {
        match self {
            Self::Clip { clip, .. } => clip.attributes(mask),
            Self::PhaseBlend(tree) => tree.attributes(mask),
            Self::BlendSpace(tree) => tree.attributes(mask),
            Self::SelectionSpace(tree) => tree
                .current()
                .ok_or("SelectionSpace attributes requested before native selection")?
                .attributes(mask),
            Self::Transition(tree) => tree.attributes(mask),
            Self::BindPose {
                motion,
                mirror_modes,
                attribute_mirror,
                ..
            } => {
                let mut attributes = motion.attributes(mask)?;
                for _ in mirror_modes {
                    for attribute in &mut attributes {
                        attribute_mirror.apply(attribute)?;
                    }
                }
                Ok(attributes)
            }
        }
    }
    pub fn attribute(
        &self,
        name: AttributeName,
        mask: u32,
    ) -> Result<Option<AnimationAttribute>, String> {
        let mut output =
            super::output::attributes::MotionGraphAttribute { name, value: 0.0 }.to_animation();
        Ok(self
            .query_attribute(name, mask, &mut output)?
            .then_some(output))
    }
    pub fn query_attribute(
        &self,
        name: AttributeName,
        mask: u32,
        output: &mut AnimationAttribute,
    ) -> Result<bool, String> {
        match self {
            Self::PhaseBlend(tree) => tree.query_attribute(name, mask, output),
            Self::BlendSpace(tree) => tree.query_attribute(name, mask, output),
            Self::SelectionSpace(tree) => tree
                .current()
                .ok_or("SelectionSpace attribute requested before native selection")?
                .query_attribute(name, mask, output),
            Self::Transition(tree) => tree.query_attribute(name, mask, output),
            Self::BindPose {
                motion,
                mirror_modes,
                attribute_mirror,
                ..
            } => {
                //825476D0 leaves a child's partial output untouched on false.
                let found = motion.query_attribute(name, mask, output)?;
                if found {
                    for _ in mirror_modes {
                        attribute_mirror.apply(output)?;
                    }
                }
                Ok(found)
            }
            Self::Clip { clip, .. } => match clip.attribute(name, mask)? {
                Some(attribute) => {
                    output.copy_from(&attribute);
                    Ok(true)
                }
                None => Ok(false),
            },
        }
    }
    pub fn evaluate(
        &mut self,
        parameters: Evaluation,
        enabled: bool,
        output: &mut Vec<PoseCommand>,
    ) -> Result<bool, String> {
        match self {
            Self::PhaseBlend(tree) => tree.evaluate(parameters, enabled, output),
            Self::BlendSpace(tree) => tree.evaluate(parameters, enabled, output),
            Self::SelectionSpace(tree) => {
                if !enabled {
                    Ok(false)
                } else {
                    tree.current_mut()
                        .ok_or("SelectionSpace evaluated before native selection")?
                        .evaluate(parameters, true, output)
                }
            }
            Self::Transition(tree) => tree.evaluate(parameters, enabled, output),
            Self::BindPose {
                motion,
                posture,
                board_backwards,
                mirror_modes,
                ..
            } => {
                let produced = motion.evaluate(parameters, enabled, output)?;
                if produced {
                    if let Some(pose) = posture {
                        output.push(PoseCommand::Pose {
                            name: pose.name().into(),
                        });
                        output.push(PoseCommand::Add { motion_is_a: true });
                    }
                    output.push(PoseCommand::Pose {
                        name: "RIG_TPOSE".into(),
                    });
                    output.push(PoseCommand::Add { motion_is_a: true });
                    if *board_backwards {
                        output.push(PoseCommand::Pose {
                            name: "BOARD_BACKWARDS".into(),
                        });
                        output.push(PoseCommand::Add { motion_is_a: false });
                        output.push(PoseCommand::Pose {
                            name: "BOARD_BACKWARDS_IK".into(),
                        });
                        output.push(PoseCommand::Add { motion_is_a: true });
                    }
                    for &trajectory_mode in mirror_modes.iter() {
                        output.push(PoseCommand::Mirror { trajectory_mode });
                    }
                }
                Ok(produced)
            }
            Self::Clip { name, clip } => {
                let clock = &mut clip.clock;
                let rate = clock.base_speed * clock.speed;
                let limit = (clock.frames - 1.0) / clock.fps;
                let bound = |time: f32| {
                    let lower = if -time >= 0.0 { 0.0 } else { time };
                    if limit - lower >= 0.0 { lower } else { limit }
                };
                let mut previous_time = bound(clock.previous_time * rate);
                let time = bound(clock.sample_time() * rate);
                if clock.loops_since_evaluation == 0 && previous_time > time {
                    previous_time = time;
                }
                if enabled {
                    output.push(PoseCommand::Clip {
                        name: name.clone(),
                        previous_time,
                        time,
                        loops: clock.loops_since_evaluation,
                    });
                }
                if parameters.update_history {
                    clock.commit_evaluation();
                }
                Ok(enabled)
            }
        }
    }
}

#[cfg(test)]
#[path = "posture/tree_tests.rs"]
mod posture_tests;
