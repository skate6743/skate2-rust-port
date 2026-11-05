//! TU3 Sequence82B95C18 and Blend82B96058/82B961F8/82B963D8.
use super::{
    clip_clock::AdvanceResult,
    output::attributes::{AnimationAttribute, AttributeName},
    playback::TransitionSettings,
    playback_parameters::SettableAttribute,
    playback_tree::{Evaluation, PlaybackTree, PoseCommand},
};

#[derive(Clone, Debug)]
pub struct PlaybackTransition {
    pub from: Box<PlaybackTree>,
    pub to: Box<PlaybackTree>,
    pub settings: TransitionSettings,
    elapsed: f32,
    sequence_complete: bool,
}
impl PlaybackTransition {
    pub fn new(from: PlaybackTree, to: PlaybackTree, settings: TransitionSettings) -> Self {
        Self {
            from: Box::new(from),
            to: Box::new(to),
            settings,
            elapsed: 0.0,
            sequence_complete: false,
        }
    }
    pub fn complete(&self) -> bool {
        if self.settings.kind == 4 {
            self.sequence_complete
        } else {
            self.elapsed > self.settings.seconds
        }
    }
    pub fn advance(&mut self, dt: f32, phase: f32, property: &mut AdvanceResult) {
        let mut discarded = AdvanceResult {
            crossed_end: false,
            overshoot: -1.0,
            remaining_before_wrap: -1.0,
        };
        if self.settings.kind == 4 {
            if self.sequence_complete {
                self.to.advance(dt, phase, property);
            } else {
                self.from.advance(dt, phase, &mut discarded);
                if discarded.crossed_end {
                    self.sequence_complete = true;
                    self.to.advance(discarded.overshoot, phase, property);
                } else {
                    property.crossed_end = false;
                    property.remaining_before_wrap =
                        self.to.length() + discarded.remaining_before_wrap;
                }
            }
            return;
        }
        if self.settings.matching != 1 {
            self.from.advance(dt, phase, &mut discarded);
        }
        if self.settings.matching == 3 {
            let length = self.to.length();
            let time = self.from.time();
            self.to
                .set_time(if time - length >= 0.0 { length } else { time });
        } else if self.settings.matching == 2 && self.from.length() > 0.0 {
            self.to
                .set_time(self.to.length() * (self.from.time() / self.from.length()));
        } else {
            self.to.advance(dt, phase, property);
        }
        self.elapsed += dt;
    }
    pub fn set_attributes(&mut self, attributes: &[SettableAttribute]) -> Result<bool, String> {
        let result = self.to.set_attributes(attributes)?;
        if self.settings.kind == 4 && !self.sequence_complete {
            self.from.set_attributes(attributes)
        } else {
            Ok(result)
        }
    }
    fn attributes_child(&self) -> &PlaybackTree {
        if self.settings.kind == 4 && !self.sequence_complete {
            &self.from
        } else {
            &self.to
        }
    }
    pub fn attributes(&self, mask: u32) -> Result<Vec<AnimationAttribute>, String> {
        self.attributes_child().attributes(mask)
    }
    pub fn query_attribute(
        &self,
        name: AttributeName,
        mask: u32,
        output: &mut AnimationAttribute,
    ) -> Result<bool, String> {
        self.attributes_child().query_attribute(name, mask, output)
    }
    pub fn weight(&self) -> f32 {
        let value = self.elapsed / self.settings.seconds;
        let lower = if -value >= 0.0 { 0.0 } else { value };
        let weight = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
        if self.settings.seconds < f32::from_bits(0x3d4ccccd) || weight > 1.0 || weight < 0.0 {
            return weight;
        }
        let square = weight * weight;
        let cube = square * weight;
        square.mul_add(3.0, -(cube * 2.0))
    }
    pub fn evaluate(
        &mut self,
        parameters: Evaluation,
        enabled: bool,
        output: &mut Vec<PoseCommand>,
    ) -> Result<bool, String> {
        if self.settings.kind == 4 {
            return if self.sequence_complete {
                self.to.evaluate(parameters, enabled, output)
            } else {
                self.from.evaluate(parameters, enabled, output)
            };
        }
        let weight = self.weight();
        let from = self.from.evaluate(parameters, enabled, output)?;
        let to = self.to.evaluate(parameters, enabled, output)?;
        if enabled && from && to {
            output.push(if self.settings.kind == 3 {
                PoseCommand::ChannelBlend {
                    weight,
                    use_channels_from_weights: self.settings.use_channels_from_weights,
                }
            } else {
                PoseCommand::Blend { weight }
            });
        }
        Ok(from || to)
    }
}
