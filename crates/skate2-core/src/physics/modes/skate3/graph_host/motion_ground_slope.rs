//! GroundSlopeType: TU3 ctor82BA7BD8 and condition82BA7E80.
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GroundSlopeType(u32);

impl GroundSlopeType {
    pub fn parse(attributes: &Attributes<'_>) -> Result<Self, String> {
        let name = attributes
            .text("slopetype")
            .ok_or("GroundSlopeType requires slopetype")?;
        let value = match name {
            "Flat" => 0,
            "StairUpShallow" => 1,
            "StairUpSteep" => 2,
            "StairDnShallow" => 3,
            "StairDnSteep" => 4,
            "RampUpShallow" => 5,
            "RampUpSteep" => 6,
            "RampDnShallow" => 7,
            "RampDnSteep" => 8,
            // The native constructor has no fallback assignment. Reject malformed
            // host data rather than reproducing an uninitialized member read.
            _ => return Err(format!("Unknown GroundSlopeType slopetype {name}")),
        };
        Ok(Self(value))
    }

    /// Exactly PhysOut bundle+72 (OffBoard), integer field+88. The physical
    /// producer owns the classification; this graph leaf does not infer slope.
    pub fn matches(self, physical_slope_type_88: u32) -> bool {
        physical_slope_type_88 == self.0
    }
}
