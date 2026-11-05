//! Persistent original physical-foot output owner. Call after final physical
//! record correction, at Skeleton::FillPhysOut82BE2138/2140 publication time.
use skate3_core::physics::{
    foot_physical_output::{FootPhysicalOutput, FootPhysicalSettings, FootPhysicalState},
    skeleton_body::SkeletonPhysicalRecord,
};
use skate_data::collections::Collections;
pub(crate) struct FootPhysicalOutputs {
    state: FootPhysicalState,
    settings: FootPhysicalSettings,
    pub output: FootPhysicalOutput,
}
impl FootPhysicalOutputs {
    pub fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            state: FootPhysicalState::default(),
            output: FootPhysicalOutput::default(),
            settings: FootPhysicalSettings {
                deck_half_width: data.float("physicsdeck", "default", "DeckWidth")? * 0.5,
                deck_total_half_length: data.float("physicsdeck", "default", "DeckFrontEndSize")?
                    + data.float("physicsdeck", "default", "DeckMidLength")? * 0.5,
                padding: data
                    .words::<4>("physics_skeletonik", "default", "FootOnDeckPadding")?
                    .map(f32::from_bits),
            },
        })
    }
    /// The deck box the feet (and the hands, Skeleton+602/+603) are tested against: DeckWidth / 2
    /// and DeckFrontEndSize + DeckMidLength / 2.
    pub fn deck_box(&self) -> (f32, f32) {
        (self.settings.deck_half_width, self.settings.deck_total_half_length)
    }
    pub fn publish(&mut self, record: &SkeletonPhysicalRecord, dt: f32) -> FootPhysicalOutput {
        self.output = self.state.update(record, dt, self.settings);
        self.output
    }
}
