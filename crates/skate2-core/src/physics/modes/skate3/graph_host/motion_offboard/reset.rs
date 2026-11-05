//! Original registered reset handlers:82F88440/82F89600.
//! Both have Begin effects; Update and End point to82B61BB8 (raw blr).
use skate_data::state_graph::attributes::Attributes;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    SkaterAnimation,
    GivenStance,
}
impl Operation {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        match a.text("name")? {
            "ResetSkaterAnimation" => Some(Self::SkaterAnimation),
            "ResetToGivenStance" => Some(Self::GivenStance),
            _ => None,
        }
    }
    ///82BB8F70 guarantees AG-intents -> MG-output/intents -> SkaterAnim order.
    ///The service must borrow the actual owners; resetting AG/MG controllers or
    ///replacing all animation state with Default would change native behavior.
    pub fn begin<S: ResetOwners>(self, owners: &mut S) -> Result<(), S::Error> {
        match self {
            Self::SkaterAnimation => {
                owners.clear_action_intents()?;
                owners.reset_motion_graph_publications()?;
                owners.reset_skater_animation_playback()
            }
            Self::GivenStance => owners.reset_to_given_stance(),
        }
    }
}
///Required integration boundary for the single ActionGraph, MotionGraph and
///playback owners. No default implementations or fabricated reset observations.
pub trait ResetOwners {
    type Error;
    ///82C0E308 ->82BC1B68: clear actual ActionGraph intent map only.
    fn clear_action_intents(&mut self) -> Result<(), Self::Error>;
    ///825953B0: filtered intents, current MG attrs, selective scalar writes,
    ///motion intents and output2664..2680. Preserve graph execution and trick data.
    fn reset_motion_graph_publications(&mut self) -> Result<(), Self::Error>;
    ///82B98050 +82D1CAE8 +82D180E8: masks/stance bit, channel teardown,
    ///playback tree/transition/attribute/control reset on the canonical owner.
    fn reset_skater_animation_playback(&mut self) -> Result<(), Self::Error>;
    ///82B97308 consumes saved15200 via82B972A8; no playback reset.
    fn reset_to_given_stance(&mut self) -> Result<(), Self::Error>;
}
