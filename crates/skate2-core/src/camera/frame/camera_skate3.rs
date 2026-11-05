//! Source Skate 3 camera result published to the retained host camera transport.
use super::{CameraStartupFields, producer_projection::CameraProducerProjectionError};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

impl CameraStartupFields {
    pub(crate) fn try_publish_skate3_camera(
        &mut self,
        output: &crate::skate3_native::FrameOutput,
        owner: &std::sync::Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        dt: f32,
    ) -> Result<(), CameraProducerProjectionError> {
        use CameraProducerProjectionError as E;
        let source = output.camera.ok_or(E::NoCompletedSelectedUpdate)?;
        let inputs = self.selected_inputs.as_ref().ok_or(E::MissingCapture)?;
        let retained = self.construction_state.as_mut().ok_or(E::MissingConstruction)?;
        // Keep host camera anchors valid for story/scene consumers and a
        // later return to S2. These are observation caches; the source camera
        // remains the only camera graph advanced while its physics is active.
        retained.anchors.try_update(dt, owner,
            SkeletonRootVector::known(output.damped_centre_of_mass.position), &inputs.cvars,
            &mut crate::reckoning_numeric::HostReckoningNumeric)
            .map_err(|error| E::NativeAnchors(format!("{error:?}")))?;
        let man = retained.camera_man.as_ref().ok_or(E::MissingCamera)?;
        let matrix = |basis: [[f32; 3]; 3], position: [f32; 4]| {
            SkeletonRootMatrix::known(crate::skate3_bridge::player::flatten([
                [basis[0][0], basis[0][1], basis[0][2], 0.],
                [basis[1][0], basis[1][1], basis[1][2], 0.],
                [basis[2][0], basis[2][1], basis[2][2], 0.],
                [position[0], position[1], position[2], 1.],
            ]))
        };
        let current = matrix(source.basis.columns, source.position);
        let replaced = {
            let mut man = man.lock().map_err(|_| E::OwnerLock)?;
            let replaced = man.frame.field_of_view;
            man.frame.matrix = current;
            man.frame.previous_matrix =
                matrix(source.previous_basis.columns, source.previous_position);
            man.frame.velocity = SkeletonRootVector::known(source.linear_velocity);
            man.frame.angular_velocity = SkeletonRootVector::known(source.angular_velocity);
            man.frame.field_of_view = source.field_of_view_degrees;
            man.frame.motion_blur = source.blur;
            man.frame.hard_cut = source.discontinuity;
            // The host camera occurrence survives player/controller changes.
            man.update_counter = man.update_counter.wrapping_add(1);
            replaced
        };
        self.skate2_field_of_view.get_or_insert(replaced);
        self.simulation_camera_matrix = Some(current);
        self.producer_snapshot_ready = true;
        self.completed_projection = Some(self.try_provisional_camera_projection()?);
        self.completed_submission = Some(self.try_camera_submission()?);
        Ok(())
    }

    /// Back to the Skate 2 camera: its own frame field of view again, not the Skate 3 one.
    pub(crate) fn restore_skate2_field_of_view(&mut self) {
        let Some(field_of_view) = self.skate2_field_of_view.take() else { return };
        if let Some(man) = self.construction_state.as_ref().and_then(|state| state.camera_man.as_ref())
            && let Ok(mut man) = man.lock()
        {
            man.frame.field_of_view = field_of_view;
        }
    }
}
