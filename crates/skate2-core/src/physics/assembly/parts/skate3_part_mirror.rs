//! Use the retained host Part mass frame when mirroring an alternate solver.
use super::*;

impl<M, JF: Copy, JL: Copy, C> DynamicAssemblySession<SimulationAssemblyBackend<M, JF, JL>, C> {
    pub(crate) fn try_skate3_mirror_body_transform(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        part: [f32; 16],
    ) -> Result<[f32; 16], DynamicAssemblyPartTransformError> {
        self.validate_part_transform_access(backend, index)?;
        let metadata = &self.part_attachment.template.metadata[index];
        let inverse =
            (metadata.has_inverse_body_transform != 0).then_some(&metadata.inverse_body_transform);
        Ok(part_attachment_transform(part, inverse))
    }
}
