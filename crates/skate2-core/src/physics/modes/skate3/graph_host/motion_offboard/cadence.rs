//! BipedCadence82BB9E78, MatchCadence82BBA938/82BBAA00 and LocoState82BA7B50.
//! TU3 default.patched.xex SHA256
//!431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
use skate3_core::animation::{
    playback_parameters::{AttributeSink, SettableAttribute},
    skeleton_input::name::encode,
};
use skate_data::state_graph::attributes::Attributes;
#[cfg(any())]
#[path = "cadence/tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    BipedCadence,
    MatchCadence,
}
impl Operation {
    /// Constructors have no operation-specific XML configuration. The common
    /// graph wrapper retains mask, active and ordering behavior.
    pub(crate) fn parse(a: &Attributes<'_>) -> Option<Self> {
        match a.text("name") {
            Some("BipedCadence") => Some(Self::BipedCadence),
            Some("MatchCadence") => Some(Self::MatchCadence),
            _ => None,
        }
    }
}

///82BB9E78 calls SkaterAnim virtual40=82531200, a direct phase14652 store.
/// Supply the actual OffBoard80 and the existing AnimationState.phase owner.
/// Component absence skips the write. Begin/End are82B61BB8 no-ops.
pub(crate) fn biped_cadence(
    physical_phase: Option<f32>,
    animation_phase: Option<&mut f32>,
    phase: u8,
) {
    if phase == 1 {
        if let (Some(value), Some(output)) = (physical_phase, animation_phase) {
            *output = value;
        }
    }
}

/// Per-node state from82BBAA98: value8=0 and byte12=false. This is a captured
/// Begin value, not a second physical cadence or animation phase owner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MatchCadence {
    pub captured_phase: f32,
    pub pending: bool,
}
impl MatchCadence {
    ///82BBA938 requires both physical and animation components. Missing either
    /// leaves the previous capture and byte untouched.
    pub(crate) fn begin(&mut self, physical_phase: Option<f32>, animation_present: bool) {
        if let (Some(value), true) = (physical_phase, animation_present) {
            self.pending = true;
            self.captured_phase = value;
        }
    }
    ///82BBAA00 publishes on every Update, even if byte12 is already false.
    /// It clears that byte only after writing through virtual80/SetAttribute.
    pub(crate) fn update(&mut self, animation: Option<&mut dyn AttributeSink>) {
        if let Some(animation) = animation {
            animation.set_attribute(SettableAttribute {
                name: encode(b"CadenceStartPercent"),
                value: self.captured_phase,
                normalized: false,
                sequence_id: -1,
            });
            self.pending = false;
        }
    }
    ///Original vtable End82B61BB8 does not reset the capture or pending byte.
    pub(crate) fn end(&mut self) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocoState {
    expected: u32,
}
impl LocoState {
    ///82BA79E8 compares case-sensitive locostate values. An unknown spelling
    /// leaves native field28 unwritten; reject invalid data rather than invent0.
    pub(crate) fn parse(a: &Attributes<'_>) -> Result<Self, String> {
        let expected = match a.text("locostate") {
            Some("Stand") => 0,
            Some("Walk") => 1,
            Some("Run") => 2,
            Some("Sprint") => 3,
            value => return Err(format!("Invalid stock LocoState locostate {value:?}")),
        };
        Ok(Self { expected })
    }
    ///82BA7B50 requires the physical component and compares OffBoard84 exactly.
    /// The graph wrapper applies its separately parsed mask and negation.
    pub(crate) fn evaluate(self, locomotion_state_84: u32) -> bool {
        locomotion_state_84 == self.expected
    }
}
