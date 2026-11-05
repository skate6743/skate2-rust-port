//! Owner-qualified live Part get/set, 0x826c3480 / 0x82b25480.
//! Raw Part cache inputs survive; live body getters retain the existing finite
//! XYZ/canonical-W boundary rather than manufacturing native packed pointers.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "live Part relocation awaits player construction")
)]

#[cfg(feature = "skate3-runtime")]
#[path = "skate3_part_mirror.rs"]
mod skate3_part_mirror;

use super::{
    DynamicAssemblyBodyOnlyPartSession, DynamicAssemblyPartAttachmentTransaction,
    DynamicAssemblyPartTransformCache, DynamicAssemblySession, DynamicAssemblySimulationBackend,
};
use crate::{
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    world::{part_attachment_transform, part_get_transform},
};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DynamicAssemblyPartTransformError {
    ForeignBackend,
    AttachedRequiresBackend,
    PartIndex { index: usize, count: usize },
    Body(SimulationRigidBodyPoolError),
    PartialTransformRequiresAttachedBody { index: usize },
    UnknownCacheLanes { index: usize },
    BodyTransformRequiresAttachedBody { index: usize },
    BodyMotionRequiresAttachedBody { index: usize },
}

impl fmt::Display for DynamicAssemblyPartTransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignBackend => f.write_str("Part transform backend mismatch"),
            Self::AttachedRequiresBackend => {
                f.write_str("attached Part transform requires its owning backend")
            }
            Self::PartIndex { index, count } => write!(f, "Part {index} outside {count} Parts"),
            Self::Body(error) => write!(f, "Part transform: {error}"),
            Self::PartialTransformRequiresAttachedBody { index } => {
                write!(f, "partial Part {index} cache requires an attached body")
            }
            Self::UnknownCacheLanes { index } => {
                write!(f, "Part {index} cache contains unknown W lanes")
            }
            Self::BodyTransformRequiresAttachedBody { index } => {
                write!(
                    f,
                    "Part {index} body-transform branch requires an attached body"
                )
            }
            Self::BodyMotionRequiresAttachedBody { index } => {
                write!(f, "Part {index} motion reset requires an attached body")
            }
        }
    }
}
impl std::error::Error for DynamicAssemblyPartTransformError {}

/// Common detached Part store; both full skater and body-only assemblies use
/// the same transaction. No mutation after attachment without its backend.
fn try_set_detached<B: DynamicAssemblySimulationBackend>(
    transaction: &mut DynamicAssemblyPartAttachmentTransaction<B>,
    attached: bool,
    index: usize,
    requested: [f32; 16],
) -> Result<(), DynamicAssemblyPartTransformError> {
    if attached || transaction.parts.iter().any(|part| part.body.is_some()) {
        return Err(DynamicAssemblyPartTransformError::AttachedRequiresBackend);
    }
    let count = transaction.parts.len();
    let part = transaction
        .parts
        .get_mut(index)
        .ok_or(DynamicAssemblyPartTransformError::PartIndex { index, count })?;
    part.part_transform = DynamicAssemblyPartTransformCache::Known(requested);
    Ok(())
}

impl<B: DynamicAssemblySimulationBackend> DynamicAssemblyBodyOnlyPartSession<B> {
    pub(crate) fn try_set_detached_part_transform(
        &mut self,
        index: usize,
        requested: [f32; 16],
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        try_set_detached(
            &mut self.transaction,
            self.attached_authority.is_some(),
            index,
            requested,
        )
    }
}

#[derive(Debug)]
pub(crate) enum DynamicAssemblyPartDisplacementError<N> {
    Part(DynamicAssemblyPartTransformError),
    MissingBody { index: usize },
    Displacement(crate::simulation_rigid_body_pool::SimulationRigidBodyDisplacementError<N>),
}
impl<N: fmt::Debug> fmt::Display for DynamicAssemblyPartDisplacementError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Part(error) => write!(f, "displacement Part: {error}"),
            Self::MissingBody { index } => write!(f, "displacement Part {index} has no body"),
            Self::Displacement(error) => write!(f, "{error}"),
        }
    }
}

impl<M, JF: Copy, JL: Copy, C> DynamicAssemblySession<SimulationAssemblyBackend<M, JF, JL>, C> {
    /// Attached arm of `Part::GetBodyTransform` 0x827ab5c0. Unlike `GetTransform`,
    /// it does not compose the inverse Part-local frame. Native packed W is
    /// unknown; the body's host affine W is never promoted to source evidence.
    pub(crate) fn try_attached_part_body_transform(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootMatrix,
        DynamicAssemblyPartTransformError,
    > {
        self.validate_part_transform_access(backend, index)?;
        let body = self.part_attachment.parts[index].body.ok_or(
            DynamicAssemblyPartTransformError::BodyTransformRequiresAttachedBody { index },
        )?;
        let pose = backend
            .try_part_body_pose(body)
            .map_err(DynamicAssemblyPartTransformError::Body)?;
        Ok(
            crate::skeleton_root_transforms::SkeletonRootMatrix::from_columns(std::array::from_fn(
                |column| crate::skeleton_root_transforms::SkeletonRootVector {
                    xyz: [pose[column * 4], pose[column * 4 + 1], pose[column * 4 + 2]],
                    w: None,
                },
            )),
        )
    }
    /// Detached branch of `Part::SetTransform` (0x82b25480): copy the raw
    /// requested cache without body math or a fabricated backend. Checked host
    /// preflight refuses attached/partial attachment before any cache mutation.
    pub(crate) fn try_set_detached_part_transform(
        &mut self,
        index: usize,
        requested: [f32; 16],
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        try_set_detached(
            &mut self.part_attachment,
            self.attached_authority.is_some(),
            index,
            requested,
        )
    }

    fn validate_part_transform_access(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        if self
            .attached_authority
            .is_some_and(|authority| authority != backend.authority())
        {
            return Err(DynamicAssemblyPartTransformError::ForeignBackend);
        }
        let count = self.part_attachment.parts.len();
        if index >= count {
            return Err(DynamicAssemblyPartTransformError::PartIndex { index, count });
        }
        Ok(())
    }

    /// Source-facing `Part::GetTransform` boundary (0x826c3480). Detached cache
    /// lanes retain their actual knowledge. Attached body XYZ uses the existing
    /// Part composition, but host canonical W never becomes native packed W.
    pub(crate) fn try_live_part_transform_typed(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootMatrix,
        DynamicAssemblyPartTransformError,
    > {
        use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
        self.validate_part_transform_access(backend, index)?;
        let part = &self.part_attachment.parts[index];
        if part.body.is_none() {
            return Ok(part.part_transform.typed());
        }
        let projected = self.try_live_part_transform_host(backend, index)?;
        Ok(SkeletonRootMatrix::from_columns(std::array::from_fn(
            |column| SkeletonRootVector {
                xyz: [
                    projected[column * 4],
                    projected[column * 4 + 1],
                    projected[column * 4 + 2],
                ],
                w: None,
            },
        )))
    }

    /// Host XYZ/canonical-W projection. Source-facing copies should use the
    /// typed getter. Live body first; detached known cache copied verbatim.
    pub(crate) fn try_live_part_transform_host(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<[f32; 16], DynamicAssemblyPartTransformError> {
        self.validate_part_transform_access(backend, index)?;
        let part = &self.part_attachment.parts[index];
        // The source selects the body branch before reading the raw cache.
        // An incomplete attached cache is not a live-body getter failure.
        let Some(body) = part.body else {
            return part
                .part_transform
                .known()
                .copied()
                .ok_or(DynamicAssemblyPartTransformError::UnknownCacheLanes { index });
        };
        let body = backend
            .try_part_body_pose(body)
            .map_err(DynamicAssemblyPartTransformError::Body)?;
        let metadata = &self.part_attachment.template.metadata[index];
        let inverse =
            (metadata.has_inverse_body_transform != 0).then_some(&metadata.inverse_body_transform);
        Ok(part_get_transform(
            // Unused by the selected body branch of the original helper.
            body,
            Some(&body),
            inverse,
        ))
    }

    /// Attached body writes precede the unconditional raw cache copy
    /// (0x82b25b70). Host validation errors occur before mutation; no body is
    /// removed/re-added and existing constraints are untouched.
    pub(crate) fn try_set_part_transform_host(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        requested: [f32; 16],
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        self.validate_part_transform_access(backend, index)?;
        let part = &mut self.part_attachment.parts[index];
        if let Some(body) = part.body {
            let metadata = &self.part_attachment.template.metadata[index];
            let inverse = (metadata.has_inverse_body_transform != 0)
                .then_some(&metadata.inverse_body_transform);
            backend
                .try_set_part_body_transform_host(
                    body,
                    part_attachment_transform(requested, inverse),
                )
                .map_err(DynamicAssemblyPartTransformError::Body)?;
        }
        part.part_transform = DynamicAssemblyPartTransformCache::Known(requested);
        Ok(())
    }

    /// Same attached Part body/store graph, retaining unknown requested W in
    /// the actual cache. Private scratch is consumed only by XYZ body math;
    /// it is never published as the requested cache. Partial detached writes
    /// remain outside this attached-hook/reset seam.
    pub(crate) fn try_set_part_transform_typed(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        requested: &crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        if let Some(known) = requested.known_lanes() {
            return self.try_set_part_transform_host(backend, index, known);
        }
        self.validate_part_transform_access(backend, index)?;
        let part = &mut self.part_attachment.parts[index];
        let body = part.body.ok_or(
            DynamicAssemblyPartTransformError::PartialTransformRequiresAttachedBody { index },
        )?;
        let metadata = &self.part_attachment.template.metadata[index];
        let inverse =
            (metadata.has_inverse_body_transform != 0).then_some(&metadata.inverse_body_transform);
        backend
            .try_set_part_body_transform_host(
                body,
                part_attachment_transform(requested.xyz_scratch(), inverse),
            )
            .map_err(DynamicAssemblyPartTransformError::Body)?;
        part.part_transform = DynamicAssemblyPartTransformCache::from_typed(*requested);
        Ok(())
    }

    /// `Body::SetPosition` 0x82de8090: an attached Part's frozen body is activated and its
    /// cooldown cleared before the same `Part::SetTransform`.
    pub(crate) fn try_set_body_position_typed(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        requested: &crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        use crate::dynamic_assembly_session::part_activation::DynamicAssemblyActivationBackend;
        self.validate_part_transform_access(backend, index)?;
        if self.attached_authority.is_some()
            && let Some(body) = self.part_attachment.parts[index].body.as_ref()
        {
            let state = backend
                .try_activation_state(body)
                .map_err(DynamicAssemblyPartTransformError::Body)?;
            if state & 7 == 2 {
                backend
                    .try_activate_frozen(body)
                    .map_err(DynamicAssemblyPartTransformError::Body)?;
            }
            backend
                .try_clear_cooldown(body)
                .map_err(DynamicAssemblyPartTransformError::Body)?;
        }
        self.try_set_part_transform_typed(backend, index, requested)
    }

    /// Read-only typed cache, not the separate attached-body pose getter.
    pub(crate) fn try_cached_part_transform_typed(
        &self,
        index: usize,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootMatrix,
        DynamicAssemblyPartTransformError,
    > {
        self.part_attachment
            .parts
            .get(index)
            .map(|part| part.part_transform.typed())
            .ok_or(DynamicAssemblyPartTransformError::PartIndex {
                index,
                count: self.part_attachment.parts.len(),
            })
    }

    /// `ResetSkeleton`'s body-only motion stores follow each completed Part set.
    /// A detached Part has no body motion to reset, matching the source branch.
    pub(crate) fn try_reset_part_motion_host(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        self.try_reset_part_motion_with_presence(backend, index, false)
    }

    /// Body::ZeroVelocitiesAndForces dereferences every board Part body; a
    /// missing attached body is a typed stop rather than a silent reset skip.
    pub(crate) fn try_reset_part_motion_required_host(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        self.try_reset_part_motion_with_presence(backend, index, true)
    }

    fn try_reset_part_motion_with_presence(
        &mut self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        required: bool,
    ) -> Result<(), DynamicAssemblyPartTransformError> {
        self.validate_part_transform_access(backend, index)?;
        if let Some(body) = self.part_attachment.parts[index].body {
            backend
                .try_reset_part_body_motion_host(body)
                .map_err(DynamicAssemblyPartTransformError::Body)?;
        } else if required {
            return Err(
                DynamicAssemblyPartTransformError::BodyMotionRequiresAttachedBody { index },
            );
        }
        Ok(())
    }

    /// Actual Part/body lookup for 0x82b54634. Unlike reset, source has no
    /// nullable-body branch; a missing body is a typed stop, never a skip.
    pub(crate) fn try_apply_part_displacement<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
        displacement: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), DynamicAssemblyPartDisplacementError<N::Error>> {
        use DynamicAssemblyPartDisplacementError as E;
        self.validate_part_transform_access(backend, index)
            .map_err(E::Part)?;
        let body = self.part_attachment.parts[index]
            .body
            .ok_or(E::MissingBody { index })?;
        backend
            .try_apply_part_body_displacement(body, displacement, numeric)
            .map_err(E::Displacement)
    }
}

#[cfg(test)]
#[path = "dynamic_assembly_part_transform_tests.rs"]
mod tests;
