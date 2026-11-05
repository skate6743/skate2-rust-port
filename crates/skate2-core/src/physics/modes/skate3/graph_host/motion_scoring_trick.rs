//! ScoringTrick lifecycle from original Skate 3 TU3 82BCC030/82BBFA88.
//! Update publishes the same authored encoded name to both score-name slots;
//! Begin and End are empty. No animation, force, or trick-selection side effect.
use skate3_core::animation::{output::attributes::AttributeName, skeleton_input::name::encode};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub trick: AttributeName,
}

impl Operation {
    pub fn parse(attributes: &Attributes<'_>) -> Result<Self, String> {
        let trick = attributes.text("trick").ok_or("ScoringTrick requires authored trick")?;
        Ok(Self { trick: encode(trick.as_bytes()) })
    }

    /// Vtable823214D0: Update at+52, while Begin+48 and End+56 are blr.
    pub fn execute(&self, names: &mut Names, flags: &mut u32, phase: u8) {
        if phase != 1 {
            return;
        }
        //8258FA20: oris flags,0x100; copies the two input names verbatim.
        //82BBFA88 supplies two copies of the same name: no mirror selection.
        *flags |= 0x0100_0000;
        names.first = Some(self.trick);
        names.second = Some(self.trick);
    }
}

/// MotionGraph full-object5924 and5948, retained alongside score flags5972.
/// Native24-byte slots contain five encoded words plus initialized padding;
/// host representation preserves the names without copying native padding.
#[derive(Default, Debug)]
pub struct Names {
    pub first: Option<AttributeName>,
    pub second: Option<AttributeName>,
}
