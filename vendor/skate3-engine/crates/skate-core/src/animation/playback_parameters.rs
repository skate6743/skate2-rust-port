//! PlayAnimation's parameter refresh82BB4760 and settable queue82D19100.
use super::output::attributes::{AnimationAttribute, AttributeName};

#[derive(Clone, Debug, PartialEq)]
pub enum ParameterSource {
    MotionIntent(String),
    FilteredIntent(String),
    LastAnimation(AttributeName),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaybackParameter {
    pub source: ParameterSource,
    pub rename: Option<AttributeName>,
    pub default_value: Option<f32>,
    pub normalized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettableAttribute {
    pub name: AttributeName,
    pub value: f32,
    pub normalized: bool,
    pub sequence_id: i32,
}

/// Gameplay providers are queried when a node executes, preserving visibility
/// of earlier same-tick graph operations and the last animation tree output.
pub trait ParameterInputs {
    fn motion_intent(&self, name: &str) -> Option<f32>;
    fn filtered_intent(&self, name: &str) -> Option<f32>;
    fn last_attribute(&mut self, name: AttributeName)
    -> Result<Option<AnimationAttribute>, String>;
}

pub trait AttributeSink {
    fn set_attribute(&mut self, attribute: SettableAttribute);
}

#[derive(Clone, Debug, Default)]
pub struct SettableAttributes {
    entries: Vec<SettableAttribute>,
}
impl SettableAttributes {
    pub fn entries(&self) -> &[SettableAttribute] {
        &self.entries
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
impl AttributeSink for SettableAttributes {
    fn set_attribute(&mut self, attribute: SettableAttribute) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.name == attribute.name && entry.sequence_id == attribute.sequence_id
        }) {
            *entry = attribute;
        } else {
            self.entries.push(attribute);
        }
    }
}

impl PlaybackParameter {
    /// Last-animation parameters execute only on Begin. Existing non-scalar
    /// attributes suppress the default too: native does not reinterpret them.
    pub fn update(
        &self,
        beginning: bool,
        inputs: &mut impl ParameterInputs,
        output: &mut impl AttributeSink,
    ) -> Result<(), String> {
        use super::skeleton_input::name::encode;
        let (name, value, normalized) = match &self.source {
            ParameterSource::MotionIntent(source) => (
                self.rename.unwrap_or_else(|| encode(source.as_bytes())),
                inputs.motion_intent(source).or(self.default_value),
                self.normalized,
            ),
            ParameterSource::FilteredIntent(source) => {
                let found = inputs.filtered_intent(source);
                (
                    self.rename.unwrap_or_else(|| encode(source.as_bytes())),
                    found.or(self.default_value),
                    //82BB4908..491C goes through4AF8 and explicitly passes0.
                    found.is_some() && self.normalized,
                )
            }
            ParameterSource::LastAnimation(source) => {
                if !beginning {
                    return Ok(());
                }
                let value = match inputs.last_attribute(*source)? {
                    Some(attribute) if matches!(attribute.kind, 0 | 2) => Some(f32::from_bits(
                        attribute.payload.0[0]
                            .ok_or("Last animation scalar payload is uninitialized")?,
                    )),
                    Some(_) => return Ok(()),
                    None => self.default_value,
                };
                (self.rename.unwrap_or(*source), value, false)
            }
        };
        if let Some(value) = value {
            output.set_attribute(SettableAttribute {
                name,
                value,
                normalized,
                sequence_id: -1,
            });
        }
        Ok(())
    }
}

/// Six-word intent/animation identifiers use the same native six-character
/// chunks as the five-word attribute encoder, but retain up to36 input bytes.
pub fn intent_key(text: &str) -> [u32; 6] {
    let bytes = text.as_bytes();
    let first = super::skeleton_input::name::encode(bytes).0;
    let tail = if bytes.len() > 30 && !bytes[..30].contains(&0) {
        super::skeleton_input::name::encode(&bytes[30..]).0[0]
    } else {
        0
    };
    [first[0], first[1], first[2], first[3], first[4], tail]
}
