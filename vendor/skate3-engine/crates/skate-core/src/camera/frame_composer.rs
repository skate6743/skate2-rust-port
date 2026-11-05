//! Complete single/transition framing82E01260/82E01740. Camera positioning,
//! reference producers and stock shot selection remain the caller's inputs.
use super::{
    Shot,
    orientation_math::{basis_from_angles, look_basis},
    shot_orientation::slerp,
};
use crate::math::Basis3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSettings {
    pub maximum_pitch_degrees: f32,
    pub roll_response: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSubject {
    pub positions: [[f32; 4]; 10],
    /// Subject getter456 (the board-offset direction).
    pub board_offset_direction: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameComposer {
    pub roll: f32,
    pub reference: [f32; 4],
    pub yaw: f32,
    pub pitch: f32,
}

impl FrameComposer {
    pub fn new() -> Self {
        Self {
            roll: 0.0,
            reference: [0.0; 4],
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    /// The native transition evaluates `to` before `from`: the latter's
    /// reference/yaw/pitch are therefore the retained composer fields.
    pub fn compose(
        &mut self,
        dt: f32,
        fraction: f32,
        mirror: f32,
        from: Shot,
        to: Shot,
        fallback_reference: [f32; 4],
        camera_position: [f32; 4],
        subject: FrameSubject,
        settings: FrameSettings,
    ) -> [f32; 4] {
        let to_orientation = self.single(
            to,
            fallback_reference,
            camera_position,
            mirror,
            subject,
            settings,
        );
        let from_orientation = self.single(
            from,
            fallback_reference,
            camera_position,
            mirror,
            subject,
            settings,
        );
        let orientation = slerp(from_orientation, to_orientation, fraction);
        self.roll = if dt > 0.0 {
            ((to.framing[0] - self.roll) * settings.roll_response).mul_add(dt, self.roll)
        } else {
            to.framing[0]
        };
        orientation
    }

    pub fn single(
        &mut self,
        shot: Shot,
        fallback_reference: [f32; 4],
        camera_position: [f32; 4],
        mirror: f32,
        subject: FrameSubject,
        settings: FrameSettings,
    ) -> [f32; 4] {
        self.reference = shot.reference_point(
            &subject.positions,
            subject.board_offset_direction,
            fallback_reference,
        );
        self.yaw = shot.framing[1] * mirror;
        self.pitch = -shot.framing[2];
        let framing = basis_from_angles(self.pitch, self.yaw, 0.0);
        let mut direction: [f32; 3] =
            core::array::from_fn(|i| self.reference[i] - camera_position[i]);
        // Native copies X into W for the comparison, so only XYZ can cause
        // the nonzero path. The retained fourth position lane is not tested.
        if !direction
            .iter()
            .any(|v| v.abs() > f32::from_bits(0x34000000))
        {
            direction = [0.0, 0.0, 1.0];
        } else {
            direction = super::orientation_math::normalize(direction);
            let ceiling = crate::trigonometry::sin(
                settings.maximum_pitch_degrees * f32::from_bits(0x3c8efa35),
            );
            if direction[1] > ceiling {
                direction[1] = ceiling;
                direction = super::orientation_math::normalize(direction);
            }
        }
        let look = look_basis(direction);
        let result = Basis3 {
            columns: framing.columns.map(|column| {
                core::array::from_fn(|i| {
                    look.columns[2][i].mul_add(
                        column[2],
                        look.columns[1][i].mul_add(column[1], look.columns[0][i] * column[0]),
                    )
                })
            }),
        };
        quaternion_from_basis(result)
    }
}

/// The signed diagonal candidates and strict component selection are the
/// inline matrix conversion82E01608..82E01730; signs are XOR negations.
fn quaternion_from_basis(basis: Basis3) -> [f32; 4] {
    let [right, up, at] = basis.columns;
    let [x, y, z] = [right[0], up[1], at[2]];
    let trace = (x + y) + z;
    let (sum, companion, dominant) = if trace > 0.0 {
        (
            (x + y) + (z + 1.0),
            [up[2] - at[1], at[0] - right[2], right[1] - up[0], 0.5],
            3,
        )
    } else if x > y && x > z {
        (
            (x + (-y)) + ((-z) + 1.0),
            [0.5, up[0] + right[1], at[0] + right[2], up[2] - at[1]],
            0,
        )
    } else if y > z {
        (
            ((-x) + y) + ((-z) + 1.0),
            [up[0] + right[1], 0.5, at[1] + up[2], at[0] - right[2]],
            1,
        )
    } else {
        (
            ((-x) + (-y)) + (z + 1.0),
            [at[0] + right[2], at[1] + up[2], 0.5, right[1] - up[0]],
            2,
        )
    };
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(sum);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-sum).mul_add(inverse * inverse, 1.0), inverse);
    }
    let root = sum * inverse;
    let half_inverse = 0.5 * inverse;
    core::array::from_fn(|i| companion[i] * if i == dominant { root } else { half_inverse })
}
