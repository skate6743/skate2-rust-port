//! IsPushOffEnabled: original TU3 registration82F88150 -> ctor82BC62F8.
//! The ctor installs vtable8231F6D4; predicate slot+48 is8281DD70.
//! Original bytes386000014E800020 are li r3,1; blr. There is no physical
//! flag read, time gate or runtime setting in this leaf. The graph executor
//! remains responsible for the common active/mask/negation semantics.
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsPushOffEnabled;

impl IsPushOffEnabled {
    pub fn parse(attributes: &Attributes<'_>) -> Option<Self> {
        (attributes.text("name") == Some("IsPushOffEnabled")).then_some(Self)
    }

    ///8281DD70: unconditional true in the verified original Skate3 TU3 image.
    pub fn evaluate(self) -> bool {
        true
    }
}
