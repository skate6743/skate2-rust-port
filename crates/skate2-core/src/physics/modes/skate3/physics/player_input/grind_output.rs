//! Completed common/Skeleton output needed by the stock grind graph.
use super::PlayerInputRuntime;
use skate3_core::{
    air::trajectory::TrajectorySelector, physics::skeleton_body::SkeletonPhysicalRecord,
};

impl PlayerInputRuntime {
    /// After output reset and final physical-record correction, before the
    /// conditioner/graphs. Board publication separately supplies Ground80 and
    /// the host-stored Motion273 flag. The selector remains the sole Air443 owner.
    pub(crate) fn publish_grind_graph_outputs(
        &mut self,
        record: &SkeletonPhysicalRecord,
        reckoning_up: [f32; 4],
        selector: &TrajectorySelector,
    ) -> Result<(), String> {
        let toolkit = self
            .toolkit
            .as_ref()
            .ok_or("Grind output requires the completed physical input toolkit")?;
        // Common ProcessOutput82DB703C copies selector9653 unconditionally.
        self.physical.air.flag_443 = u8::from(selector.grind_locked_to_middle());
        self.physical
            .skeleton
            .publish_twist(record, toolkit.forward, reckoning_up);
        Ok(())
    }
}
