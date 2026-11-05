//! Provisional camera-producer snapshot for a static-map skating test.
//! Native presentation submit/restore and its interpolation clock are absent.
#![cfg_attr(not(test), expect(dead_code, reason = "awaits live static-map caller"))]
use super::CameraStartupFields;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    input_cvar_runtime::InputCvarError,
    skeleton_root_transforms::SkeletonRootMatrix,
    world::inverse_collision_affine_reset,
};

const RENDERING_BINDING: usize = 18;
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const TWO_OVER_PI: f32 = f32::from_bits(0x3f22_f983);
const NATIVE_ASPECT: f64 = 1.777_777_777_777_777_7;

#[derive(Debug)]
pub(crate) enum CameraProducerProjectionError {
    NoCompletedSelectedUpdate,
    MissingCapture,
    MissingConstruction,
    MissingCamera,
    OwnerLock,
    MissingRenderingBinding,
    MissingRenderingLayout,
    ShortRenderingLayout,
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    #[cfg(feature = "skate3-runtime")]
    NativeAnchors(String),
}

/// An actual `CameraMan` producer value plus a host projection of that value.
/// This is never the native current `CameraPresentation` restore result.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CameraProducerProjection {
    pub(crate) camera_matrix: SkeletonRootMatrix,
    pub(crate) camera_previous_matrix: SkeletonRootMatrix,
    pub(crate) view_from_world: [f32; 16],
    pub(crate) clip_from_view: [f32; 16],
    pub(crate) clip_from_world: [f32; 16],
    pub(crate) frame_fov_y_degrees: f32,
    pub(crate) chosen_fov_y_degrees: f32,
    pub(crate) near_plane: f32,
    pub(crate) far_plane: f32,
    pub(crate) rendering_fov_override: bool,
    pub(crate) hard_cut: bool,
    pub(crate) camera_index_override: i32,
    pub(crate) camera_update_counter: u32,
}

impl CameraStartupFields {
    /// Available only after this owner completed selected `CameraMan.Update`.
    /// `kbUsePhysOutCamera` is not yet retained; this selects the default
    /// ordinary `CameraMan` frame and must not be reported as a restored draw.
    pub(crate) fn try_provisional_camera_projection(
        &self,
    ) -> Result<CameraProducerProjection, CameraProducerProjectionError> {
        use CameraProducerProjectionError as E;
        if !self.producer_snapshot_ready {
            return self.completed_projection.ok_or(E::NoCompletedSelectedUpdate);
        }
        let inputs = self.selected_inputs.as_ref().ok_or(E::MissingCapture)?;
        let retained = self
            .construction_state
            .as_ref()
            .ok_or(E::MissingConstruction)?;
        let man = retained.camera_man.as_ref().ok_or(E::MissingCamera)?;
        let man = man.lock().map_err(|_| E::OwnerLock)?;
        let frame = &man.frame;
        let camera_matrix = frame.matrix;
        let camera_previous_matrix = frame.previous_matrix;
        let frame_fov = frame.field_of_view;
        let hard_cut = frame.hard_cut;
        let camera_index_override = frame.index_override;
        let camera_update_counter = man.update_counter;
        drop(man);

        // Native CameraPresentation owns a distinct rendering instance.
        // The retained CameraAttribManager binding is only an interim proxy
        // over the same mounted class/default collection.
        // Native RenderMan getter order is far, near, then FOV. These are
        // separate reads, including a conditional FOV layout read.
        let far_plane = {
            let owner = retained.parameters.lock().map_err(|_| E::OwnerLock)?;
            let rendering = owner
                .bindings
                .get(RENDERING_BINDING)
                .ok_or(E::MissingRenderingBinding)?;
            rendering_far_plane(rendering)?
        };
        // This cvar is sampled live; the CameraFrame's near_plane is a
        // different owner.
        let near_plane = inputs
            .cvars
            .try_camera_near_clip_distance()
            .map_err(E::Cvar)?;
        let override_fov = {
            let owner = retained.parameters.lock().map_err(|_| E::OwnerLock)?;
            let rendering = owner
                .bindings
                .get(RENDERING_BINDING)
                .ok_or(E::MissingRenderingBinding)?;
            rendering_override_fov(rendering)?
        };
        let override_enabled = override_fov.is_some();
        // PC port addition: Gameplay Settings' Field of View widens or narrows the game camera.
        let chosen_fov = override_fov.unwrap_or(frame_fov) * crate::ui::options::field_of_view_scale();
        Ok(project_producer(
            camera_matrix,
            camera_previous_matrix,
            frame_fov,
            chosen_fov,
            near_plane,
            far_plane,
            override_enabled,
            hard_cut,
            camera_index_override,
            camera_update_counter,
        ))
    }
}

impl CameraStartupFields {
    /// `CameraPresentation::DoSimSubmit` 0x827f23a8 after this owner's update:
    /// `CameraManager::GetCameraFrame` of its one camera (`GetNumCameras` answers 1),
    /// with the lens `RenderMan::UpdateCamera` reads, sampled with the frame.
    pub(crate) fn try_camera_submission(
        &self,
    ) -> Result<crate::presentation_restore::camera::CameraSubmission, CameraProducerProjectionError> {
        use CameraProducerProjectionError as E;
        let inputs = self.selected_inputs.as_ref().ok_or(E::MissingCapture)?;
        let retained = self
            .construction_state
            .as_ref()
            .ok_or(E::MissingConstruction)?;
        let man = retained.camera_man.as_ref().ok_or(E::MissingCamera)?;
        let (frame, update_counter) = {
            let man = man.lock().map_err(|_| E::OwnerLock)?;
            (man.frame, man.update_counter)
        };
        let (far_plane, override_fov) = {
            let owner = retained.parameters.lock().map_err(|_| E::OwnerLock)?;
            let rendering = owner
                .bindings
                .get(RENDERING_BINDING)
                .ok_or(E::MissingRenderingBinding)?;
            (rendering_far_plane(rendering)?, rendering_override_fov(rendering)?)
        };
        let near_plane = inputs
            .cvars
            .try_camera_near_clip_distance()
            .map_err(E::Cvar)?;
        Ok(crate::presentation_restore::camera::CameraSubmission {
            frames: vec![frame],
            lens: crate::presentation_restore::camera::CameraLens {
                near_plane,
                far_plane,
                override_fov,
                update_counter,
            },
        })
    }

    /// The submission of the last completed update.
    pub(crate) fn completed_camera_submission(
        &self,
    ) -> Option<crate::presentation_restore::camera::CameraSubmission> {
        self.completed_submission.clone()
    }

    /// The renderer's projection while `CameraPresentation::mICECameraActive`:
    /// `RenderMan::UpdateCamera` (`0x82848978`) reads `GetMatrix`
    /// (`0x827f6308`: `mICEMatrix`), `GetFarPlane` (`0x827f2630`: the rendering
    /// attribute's far plane, as for the game camera), `GetNearPlane`
    /// (`0x827ef8d8`: `mICENearPlane`) and `GetFOVY` (`0x827f64b0`: `mICEFOV`,
    /// without the rendering FOV override). Without an ICE matrix, `mICEMatrix`
    /// holds the game camera's.
    pub(crate) fn try_ice_camera_projection(
        &self,
        matrix: Option<[f32; 16]>,
        fov: f32,
        near_plane: f32,
    ) -> Result<CameraProducerProjection, CameraProducerProjectionError> {
        use CameraProducerProjectionError as E;
        let retained = self
            .construction_state
            .as_ref()
            .ok_or(E::MissingConstruction)?;
        let man = retained.camera_man.as_ref().ok_or(E::MissingCamera)?;
        let (game_matrix, camera_update_counter) = {
            let man = man.lock().map_err(|_| E::OwnerLock)?;
            (man.frame.matrix, man.update_counter)
        };
        let far_plane = {
            let owner = retained.parameters.lock().map_err(|_| E::OwnerLock)?;
            let rendering = owner
                .bindings
                .get(RENDERING_BINDING)
                .ok_or(E::MissingRenderingBinding)?;
            rendering_far_plane(rendering)?
        };
        let matrix = matrix.map_or(game_matrix, SkeletonRootMatrix::known);
        Ok(project_producer(
            matrix,
            matrix,
            fov,
            fov,
            near_plane,
            far_plane,
            false,
            false,
            -1,
            camera_update_counter,
        ))
    }
}

fn rendering_far_plane(
    rendering: &AttributeRuntimeInstance,
) -> Result<f32, CameraProducerProjectionError> {
    use CameraProducerProjectionError as E;
    let far = rendering
        .try_cached_layout_field(0x64, 4)
        .map_err(E::Attribute)?
        .ok_or(E::MissingRenderingLayout)?;
    be_f32(far.bytes())
}

fn rendering_override_fov(
    rendering: &AttributeRuntimeInstance,
) -> Result<Option<f32>, CameraProducerProjectionError> {
    use CameraProducerProjectionError as E;
    let flag = rendering
        .try_cached_layout_field(0x5a, 1)
        .map_err(E::Attribute)?
        .ok_or(E::MissingRenderingLayout)?;
    let flag = *flag.bytes().first().ok_or(E::ShortRenderingLayout)? != 0;
    if !flag {
        return Ok(None);
    }
    let fov = rendering
        .try_cached_layout_field(0x60, 4)
        .map_err(E::Attribute)?
        .ok_or(E::MissingRenderingLayout)?;
    Ok(Some(be_f32(fov.bytes())?))
}

fn be_f32(bytes: &[u8]) -> Result<f32, CameraProducerProjectionError> {
    let word: [u8; 4] = bytes
        .try_into()
        .map_err(|_| CameraProducerProjectionError::ShortRenderingLayout)?;
    Ok(f32::from_be_bytes(word))
}

#[expect(
    clippy::too_many_arguments,
    reason = "immutable native snapshot and renderer scalar sources"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the provisional host projection intentionally narrows numeric intermediates to f32 matrix lanes"
)]
pub(crate) fn project_producer(
    camera_matrix: SkeletonRootMatrix,
    camera_previous_matrix: SkeletonRootMatrix,
    frame_fov: f32,
    chosen_fov: f32,
    near_plane: f32,
    far_plane: f32,
    rendering_fov_override: bool,
    hard_cut: bool,
    camera_index_override: i32,
    camera_update_counter: u32,
) -> CameraProducerProjection {
    // Reuse the retained general affine inverse's XYZ cofactor graph. Its W
    // residual is specific to the collision caller; RenderMan stores the
    // canonical view W lanes separately. Xenon vrefp bit parity is unresolved.
    // Both native inverse and this helper use input XYZ for all visible
    // inverse lanes. The helper's W scratch never publishes: RenderMan writes
    // canonical W immediately after its inverse, as done below. The two
    // retained source matrices keep their actual Option<f32> W separately.
    let mut view = inverse_collision_affine_reset(camera_matrix.xyz_scratch());
    view[3] = 0.0;
    view[7] = 0.0;
    view[11] = 0.0;
    view[15] = 1.0;

    let radians = chosen_fov * DEGREES_TO_RADIANS;
    let normalized_angle = radians * TWO_OVER_PI;
    let horizontal = (f64::from(normalized_angle) * NATIVE_ASPECT) as f32;
    let x_scale = (-1.0_f64 / f64::from(horizontal)) as f32;
    let y_scale = (1.0_f64 / f64::from(normalized_angle)) as f32;
    let far = f64::from(far_plane);
    let near = f64::from(near_plane);
    let z_scale = (far / (far - near)) as f32;
    let z_offset = (far * near / (near - far)) as f32;
    let projection = [
        x_scale, 0.0, 0.0, 0.0, 0.0, y_scale, 0.0, 0.0, 0.0, 0.0, z_scale, 1.0, 0.0, 0.0, z_offset,
        0.0,
    ];
    let clip_from_world = multiply_column_major(projection, view);
    CameraProducerProjection {
        camera_matrix,
        camera_previous_matrix,
        view_from_world: view,
        clip_from_view: projection,
        clip_from_world,
        frame_fov_y_degrees: frame_fov,
        chosen_fov_y_degrees: chosen_fov,
        near_plane,
        far_plane,
        rendering_fov_override,
        hard_cut,
        camera_index_override,
        camera_update_counter,
    }
}

fn multiply_column_major(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    std::array::from_fn(|index| {
        let row = index % 4;
        let column = index / 4;
        let x = a[row] * b[column * 4];
        let xy = a[4 + row].mul_add(b[column * 4 + 1], x);
        let xyz = a[8 + row].mul_add(b[column * 4 + 2], xy);
        a[12 + row].mul_add(b[column * 4 + 3], xyz)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonrigid_camera_uses_full_inverse_and_positive_z_projection() {
        let camera = [
            2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 0.0, 0.0, 1.0,
        ];
        let camera = SkeletonRootMatrix::known(camera);
        let output = project_producer(camera, camera, 60.0, 60.0, 0.1, 7700.0, false, false, -1, 1);
        assert_eq!(output.view_from_world[0], 0.5);
        assert_eq!(output.view_from_world[12], -2.5);
        assert!(output.clip_from_view[0] < 0.0);
        assert!(output.clip_from_view[5] > 0.0);
        assert_eq!(output.clip_from_view[11], 1.0);
        assert_eq!(output.clip_from_view[15], 0.0);
    }

    #[test]
    fn rendering_override_changes_projection_source() {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let identity = SkeletonRootMatrix::known(identity);
        let normal = project_producer(
            identity, identity, 70.0, 70.0, 0.1, 7700.0, false, false, -1, 1,
        );
        let overridden = project_producer(
            identity, identity, 70.0, 60.0, 0.1, 7700.0, true, false, -1, 1,
        );
        assert!(overridden.clip_from_view[5] > normal.clip_from_view[5]);
        assert_eq!(overridden.chosen_fov_y_degrees, 60.0);
    }

    #[test]
    fn unknown_camera_w_remains_typed_and_does_not_change_view_xyz() {
        let known = SkeletonRootMatrix::known([
            2.0, 0.0, 0.0, 3.0, 0.0, 1.0, 0.0, 4.0, 0.0, 0.0, 1.0, 5.0, 5.0, 0.0, 0.0, 6.0,
        ]);
        let mut columns = *known.columns();
        for column in &mut columns {
            column.w = None;
        }
        let partial = SkeletonRootMatrix::from_columns(columns);
        let expected = project_producer(known, known, 60.0, 60.0, 0.1, 7700.0, false, false, -1, 1);
        let actual = project_producer(
            partial, partial, 60.0, 60.0, 0.1, 7700.0, false, false, -1, 1,
        );
        assert!(
            actual
                .camera_matrix
                .columns()
                .iter()
                .all(|column| column.w.is_none())
        );
        assert!(
            actual
                .camera_previous_matrix
                .columns()
                .iter()
                .all(|column| column.w.is_none())
        );
        assert_eq!(actual.view_from_world, expected.view_from_world);
        assert_eq!(actual.clip_from_world, expected.clip_from_world);
    }
}
