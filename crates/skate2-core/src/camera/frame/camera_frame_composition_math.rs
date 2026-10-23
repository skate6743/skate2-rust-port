//! `DoSingleFraming82e3edc0`: full-angle basis and actual Matrix33 look-at.
use super::{
    CameraFrameComposer,
    composition::{FrameCompositionError, math_error},
};
use crate::{
    camera_shot::CameraShot,
    camera_shot_manager::runtime_math::ShotMathError,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningNumeric, cross_typed, try_normalize_unsafe,
    },
    reckoning_sin_cos::{
        HostReckoningSinCosNumeric, ReckoningSinCosNumeric, sine::try_sine, try_sin_cos,
    },
    skeleton_root_transforms::SkeletonRootVector as Vector,
    world::try_simulation_quaternion_candidate,
};

impl CameraFrameComposer {
    pub(super) fn single(
        &mut self,
        shot: &CameraShot,
        subject_position: Vector,
        camera_position: Vector,
        mirror: f32,
    ) -> Result<[f32; 4], FrameCompositionError> {
        self.reference = self.reference_point(shot, subject_position)?;
        self.framing_x = shot.yaw * mirror;
        self.framing_y = -shot.pitch;
        let basis = match framing_basis(
            self.framing_y,
            self.framing_x,
            &mut HostReckoningSinCosNumeric,
        ) {
            Ok(value) => value,
            Err(never) => match never {},
        };
        let delta = self
            .reference
            .zip(camera_position, |reference, camera| reference - camera);
        let look = if exceeds_epsilon(delta) {
            let mut look = try_normalize_unsafe(delta, &mut HostReckoningNumeric)
                .map_err(FrameCompositionError::Numeric)?;
            let angle = self.parameter("max_pitch")? * f32::from_bits(0x3c8e_fa35);
            let maximum = match try_sine(angle, &mut HostReckoningSinCosNumeric) {
                Ok(value) => value,
                Err(never) => match never {},
            };
            if look.xyz[1] > maximum {
                look.xyz[1] = maximum;
                look = try_normalize_unsafe(look, &mut HostReckoningNumeric)
                    .map_err(FrameCompositionError::Numeric)?;
            }
            look
        } else {
            Vector::known([0.0, 0.0, 1.0, 0.0])
        };
        orientation(basis, look, &mut HostReckoningNumeric)
            .map_err(|error| math_error(ShotMathError::Numeric(error)))
    }
}

pub(super) fn exceeds_epsilon(delta: Vector) -> bool {
    delta
        .xyz
        .iter()
        .any(|lane| lane.abs() > f32::from_bits(0x3400_0000))
}

/// All three native `SinCos` calls precede the reference/camera delta normalization.
pub(crate) fn framing_basis<T: ReckoningSinCosNumeric>(
    pitch: f32,
    yaw: f32,
    trig: &mut T,
) -> Result<[[f32; 4]; 3], T::Error> {
    let (sx, cx) = try_sin_cos(pitch, trig)?;
    let (sy, cy) = try_sin_cos(yaw, trig)?;
    let (sz, cz) = try_sin_cos(0.0, trig)?;
    let p = cx * cz;
    let q = sx * sz;
    let r = cx * sz;
    let s = sx * cz;
    let xx = cy * cz;
    let yx = sy * s - r;
    let zx = sy.mul_add(p, q);
    Ok([
        [xx, cy * sz, -sy, xx],
        [yx, sy.mul_add(q, p), cy * sx, yx],
        [zx, sy * r - s, cy * cx, zx],
    ])
}

/// `Matrix33FromLookAt829b0790`: normalize both crosses; retain input Z.
pub(crate) fn look_basis<N: ReckoningNumeric>(
    look: Vector,
    numeric: &mut N,
) -> Result<[Vector; 3], N::Error> {
    let x = try_normalize_unsafe(cross_typed([0.0, 1.0, 0.0, 0.0], look), numeric)?;
    let y = try_normalize_unsafe(cross_typed(look, x), numeric)?;
    Ok([x, y, look])
}

pub(super) fn orientation<N: ReckoningNumeric>(
    basis: [[f32; 4]; 3],
    look: Vector,
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let look = look_basis(look, numeric)?;
    let composed = basis.map(|column| {
        let first = look[0].map(|lane| lane * column[0]);
        let second = look[1].zip(first, |lane, old| lane.mul_add(column[1], old));
        look[2].zip(second, |lane, old| lane.mul_add(column[2], old))
    });
    // The existing candidate consumes only the nine basis XYZ lanes. Neither
    // allocation-unknown W nor unconsumed translation becomes a stored zero.
    let mut matrix = [0.0; 16];
    for (index, column) in composed.iter().enumerate() {
        matrix[index * 4..index * 4 + 3].copy_from_slice(&column.xyz);
    }
    try_simulation_quaternion_candidate(matrix, |magnitudes| {
        let mut estimates = [0.0; 4];
        for (result, value) in estimates.iter_mut().zip(magnitudes) {
            *result = numeric.reciprocal_sqrt_estimate(value)?;
        }
        Ok(estimates)
    })
}
