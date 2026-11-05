//! `CameraPresentation`'s restore side: the frames `DoSimSubmit` 0x827f23a8 submits each
//! tick, `DoSimRestore` 0x827f60e0's blend through the interpolating `CameraFrame`
//! constructor 0x829d3e98, `GetCurrentData` 0x826cb9e8's choice of frame and
//! `RenderMan::UpdateCamera` 0x82848978's projection of it.

use super::math::slerp;
use crate::{
    actor_face_playback::presentation::move_root::rigid_matrix,
    camera_frame::CameraFrame,
    camera_session::producer_projection::{CameraProducerProjection, project_producer},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
    world::try_simulation_quaternion_candidate,
};

/// `DoSimSubmit` asserts at most sixteen cameras (`mInterpolatedData`).
const MAX_CAMERAS: usize = 16;

/// What the renderer's camera reads besides the restored frame: the rendering
/// attributes' far plane and FOV override and the near clip cvar, sampled with the
/// tick's frames.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CameraLens {
    pub(crate) near_plane: f32,
    pub(crate) far_plane: f32,
    pub(crate) override_fov: Option<f32>,
    pub(crate) update_counter: u32,
}

/// One tick's `DoSimSubmit`: `CameraManager::GetCameraFrame` of each camera, whose
/// `mCameraIndexOverride` the manager's override (+0x6b8) replaces, with the lens the
/// renderer reads.
#[derive(Clone, Debug)]
pub(crate) struct CameraSubmission {
    pub(crate) frames: Vec<CameraFrame>,
    pub(crate) lens: CameraLens,
}

/// The camera a frame is drawn from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraView {
    pub clip_from_world: [f32; 16],
    pub position: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
    /// Near, then far.
    pub planes: [f32; 2],
    pub update_counter: u32,
}

impl CameraView {
    pub(crate) fn of(projection: &CameraProducerProjection) -> Self {
        let columns = projection.camera_matrix.columns();
        Self {
            clip_from_world: projection.clip_from_world,
            position: columns[3].xyz,
            right: columns[0].xyz,
            up: columns[1].xyz,
            forward: columns[2].xyz,
            planes: [projection.near_plane, projection.far_plane],
            update_counter: projection.camera_update_counter,
        }
    }
}

/// `CameraFrame::Lerp` 0x829d3978: each matrix's rotation as a quaternion, slerped,
/// and the translation row lerped.
fn lerp_matrix(a: &SkeletonRootMatrix, b: &SkeletonRootMatrix, amount: f32) -> SkeletonRootMatrix {
    let quaternion = |matrix: &SkeletonRootMatrix| {
        try_simulation_quaternion_candidate(matrix.xyz_scratch(), |magnitudes| {
            Ok::<_, std::convert::Infallible>(magnitudes.map(|value| 1.0 / value.sqrt()))
        })
        .unwrap_or_else(|never| match never {})
    };
    let rotation = slerp(quaternion(a), quaternion(b), amount);
    let rows = rigid_matrix(rotation, [0.0, 0.0, 0.0, 1.0]);
    let [ta, tb] = [a.columns()[3], b.columns()[3]];
    let mut result = SkeletonRootMatrix::known(rows);
    for column in 0..3 {
        // The rotation rows' W lanes are left as the quaternion conversion's scratch.
        let mut axis = result.columns()[column];
        axis.w = None;
        result.set_column(column, axis);
    }
    result.set_column(3, tb.zip(ta, |b, a| (b - a).mul_add(amount, a)));
    result
}

/// A hard cut that moved the camera under 3 m and turned its view under about 45 degrees: a
/// collision nudge or a fast swing, not a cut to another shot or place.
fn nudge(before: &CameraFrame, after: &CameraFrame) -> bool {
    let [a, b] = [before.matrix.columns(), after.matrix.columns()];
    let moved: f32 = (0..3).map(|lane| (b[3].xyz[lane] - a[3].xyz[lane]).powi(2)).sum();
    let facing: f32 = (0..3).map(|lane| a[2].xyz[lane] * b[2].xyz[lane]).sum();
    moved < 9.0 && facing > 0.7
}

/// SKATE2_TRACE_CAMERA_RESTORE: the restored camera's hard cuts and kept blends.
fn trace_restore() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_CAMERA_RESTORE").is_some())
}

fn lerp_vector(a: SkeletonRootVector, b: SkeletonRootVector, amount: f32) -> SkeletonRootVector {
    b.zip(a, |b, a| (b - a).mul_add(amount, a))
}

impl CameraFrame {
    /// `CameraFrame::CameraFrame(frameA, frameB, interpolation)` 0x829d3e98.
    pub(crate) fn interpolated(a: &Self, b: &Self, amount: f32) -> Self {
        let lerp = |a: f32, b: f32| (b - a).mul_add(amount, a);
        Self {
            field_of_view: lerp(a.field_of_view, b.field_of_view),
            framing_x: lerp(a.framing_x, b.framing_x),
            framing_y: lerp(a.framing_y, b.framing_y),
            near_plane: lerp(a.near_plane, b.near_plane),
            matrix: lerp_matrix(&a.matrix, &b.matrix, amount),
            previous_matrix: lerp_matrix(&a.previous_matrix, &b.previous_matrix, amount),
            velocity: lerp_vector(a.velocity, b.velocity, amount),
            angular_velocity: lerp_vector(a.angular_velocity, b.angular_velocity, amount),
            anchor: lerp_vector(a.anchor, b.anchor, amount),
            reference: lerp_vector(a.reference, b.reference, amount),
            motion_blur: lerp(a.motion_blur, b.motion_blur),
            hard_cut: b.hard_cut,
            index_override: b.index_override,
        }
    }
}

/// `CameraPresentation`'s restored state.
#[derive(Debug)]
pub(crate) struct CameraRestore {
    count: usize,
    interpolated: Vec<CameraFrame>,
    current: i32,
    lens: Option<CameraLens>,
}

impl CameraRestore {
    pub(crate) fn new() -> Self {
        Self {
            count: 0,
            interpolated: vec![CameraFrame::constructed(); MAX_CAMERAS],
            current: -1,
            lens: None,
        }
    }

    /// `DoSimRestore` 0x827f60e0 between the frame before (`previous`) and the restored
    /// frame. A frame count that changed keeps the last blend. `forward` is the
    /// restore direction a hard cut snaps toward.
    pub(crate) fn restore(
        &mut self,
        next: &CameraSubmission,
        previous: Option<&CameraSubmission>,
        percent: f32,
        forward: bool,
    ) {
        let frames = &next.frames;
        self.lens = Some(next.lens);
        self.count = frames.len().min(MAX_CAMERAS);
        if trace_restore() && previous.is_some_and(|previous| previous.frames.len() != frames.len()) {
            eprintln!("camera restore: frame count changed, last blend kept (percent {percent})");
        }
        if let Some(before) = previous.map(|previous| &previous.frames)
            && !before.is_empty()
            && before.len() == frames.len()
        {
            for (index, (before, after)) in before.iter().zip(frames).take(MAX_CAMERAS).enumerate()
            {
                // PC port: the original drew one restore per submitted frame, where snapping a hard
                // cut to the next frame shows nothing. Drawn between two frames, the camera
                // positioner's collision cut (`colliding && !previous_colliding`, a nudge off a
                // wall) would put the camera a fraction of a frame ahead of the skaters it frames.
                // A cut that barely moves or turns the camera blends with them; a real cut snaps.
                let snap = after.hard_cut && !nudge(before, after);
                if trace_restore() && after.hard_cut {
                    eprintln!("camera restore: hard cut at percent {percent}, snapped={snap}");
                }
                let amount = if snap {
                    if forward { 1.0 } else { 0.0 }
                } else {
                    percent
                };
                self.interpolated[index] = CameraFrame::interpolated(before, after, amount);
            }
        }
        self.current = if self.count == 0 {
            -1
        } else if self.current == -1 {
            0
        } else {
            self.current % i32::try_from(self.count).unwrap_or(i32::MAX)
        };
    }

    /// `GetCurrentData` 0x826cb9e8 without the shot and manual cameras: the frame the
    /// current frame's override names, else the current frame.
    fn current_frame(&self) -> Option<&CameraFrame> {
        let current = usize::try_from(self.current).ok()?;
        let frame = self.interpolated.get(current)?;
        let index = frame.index_override;
        match usize::try_from(index) {
            Ok(index) if index < self.count => self.interpolated.get(index),
            _ => Some(frame),
        }
    }

    /// `RenderMan::UpdateCamera` 0x82848978 over the restored frame: `GetMatrix`,
    /// `GetNearPlane` (the near clip cvar), `GetFarPlane` and `GetFOVY` (the rendering
    /// FOV override, else the frame's).
    pub(crate) fn view(&self) -> Option<CameraView> {
        let lens = self.lens?;
        let frame = self.current_frame()?;
        // PC port addition: Gameplay Settings' Field of View widens or narrows the game camera.
        let projection = project_producer(
            frame.matrix,
            frame.previous_matrix,
            frame.field_of_view,
            lens.override_fov.unwrap_or(frame.field_of_view) * crate::ui::options::field_of_view_scale(),
            lens.near_plane,
            lens.far_plane,
            lens.override_fov.is_some(),
            frame.hard_cut,
            frame.index_override,
            lens.update_counter,
        );
        Some(CameraView::of(&projection))
    }

    /// `CameraPresentation::GetReplayMatrix` (`0x826cf400`): the matrix of `GetCurrentData`'s
    /// frame, its axes' W 0 and its translation's 1, as `ResolveAgainstStaticObjects`
    /// (`0x826d0268`) takes it.
    pub(crate) fn replay_matrix(&self) -> Option<[f32; 16]> {
        let c = self.current_frame()?.matrix.columns();
        let [x, y, z, t] = [c[0].xyz, c[1].xyz, c[2].xyz, c[3].xyz];
        Some([x[0], x[1], x[2], 0.0, y[0], y[1], y[2], 0.0, z[0], z[1], z[2], 0.0, t[0], t[1], t[2], 1.0])
    }
}

#[cfg(test)]
#[path = "presentation_restore_camera_tests.rs"]
mod tests;
