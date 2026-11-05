//! Scalar port of Skate 3 TU3's skateboard drive-frame construction.
//!
//! `SkateboardBody::SetDriveFrames2` (`0x82C0C088`) writes an identity matrix
//! to the child/second-body frame and writes the computed relative matrix to
//! the parent/first-body frame. The retail matrices are affine column-basis
//! matrices (`Ri`, `Up`, `At`, translation). This module keeps that convention
//! explicit and does not substitute a Bevy joint.

use super::{
    board::*,
    drive_parameters::{RetailDriveFrameRaw, RetailDriveFramesRaw},
    mass::retail_deck_mass_properties,
    native_arithmetic,
    rigid_body::RetailQuaternion,
};
use crate::{
    math::{Basis3, Vector3},
    trigonometry,
};

#[path = "construction/authored_transforms.rs"]
mod authored;
pub use authored::{AuthoredTransformInputs, authored_body_pose_records, authored_body_transforms};

pub mod tu3 {
    pub const CALCULATE_TRUCK_TRANSFORMS: u32 = 0x82C0_BC90;
    pub const SET_DRIVE_FRAMES_2: u32 = 0x82C0_C088;
    pub const MATRIX_TO_FRAME_A: u32 = 0x82BD_3A10;
    pub const MATRIX_TO_FRAME_B: u32 = 0x82BD_3BD0;
    pub const PART_SET_TRANSFORM: u32 = 0x82BD_4318;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailAffineTransform {
    /// Column vectors in retail `Ri`, `Up`, `At` order.
    pub basis: Basis3,
    pub translation: Vector3,
}

impl RetailAffineTransform {
    pub const IDENTITY: Self = Self {
        basis: Basis3 {
            columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        translation: Vector3::ZERO,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailDriveFrame {
    pub orientation: RetailQuaternion,
    pub translation: Vector3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailDriveFrames {
    /// Native child/second-body record written by `SetChildFrame` at +0/+16.
    pub body_a: RetailDriveFrame,
    /// Native parent/first-body record written by `SetParentFrame` at +32/+48.
    pub body_b: RetailDriveFrame,
}

/// Inputs read by TU3 `SkateboardBody::CalculateTruckTransforms`.
///
/// The two longitudinal fields retain the decoded retail attribute names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailTruckTransformInputs {
    pub deck_mid_length: f32,
    pub truck_z_position_front: f32,
    pub truck_z_position_back: f32,
    pub truck_y_position: f32,
    pub truck_rotation_axis_angle_degrees: f32,
}

pub const RETAIL_DEFAULT_TRUCK_TRANSFORM_INPUTS: RetailTruckTransformInputs =
    RetailTruckTransformInputs {
        deck_mid_length: RETAIL_DECK_MID_LENGTH,
        truck_z_position_front: RETAIL_TRUCK_Z_POSITION_FRONT,
        truck_z_position_back: RETAIL_TRUCK_Z_POSITION_BACK,
        truck_y_position: RETAIL_TRUCK_Y_POSITION,
        truck_rotation_axis_angle_degrees: RETAIL_TRUCK_ROTATION_AXIS_ANGLE_DEGREES,
    };

/// TU3 `SkateboardBody::CalculateTruckTransforms` (`0x82C0BC90`) evaluated
/// from the stock skater-collection inputs.
///
/// The returned order is the native `+7712`, `+7776` storage order. The names
/// of the front/back Z inputs are retained even though the native slots do not
/// follow an intuitive front/back naming order.
pub fn default_truck_transforms() -> [RetailAffineTransform; 2] {
    calculate_truck_transforms(RETAIL_DEFAULT_TRUCK_TRANSFORM_INPUTS)
}

/// Authored part matrices from `82C0ADF0`, evaluated from stock collection inputs.
/// These are the inputs to Part::SetTransform, not settled or live COM poses.
pub fn default_body_transforms() -> [RetailAffineTransform; BODY_COUNT] {
    authored_body_transforms(AuthoredTransformInputs::STOCK)
}

/// Live center-of-mass transforms emitted by TU3 `Part::SetTransform`
/// (`0x82BD4318`) from the seven authored part transforms.
pub fn default_live_body_transforms() -> [RetailAffineTransform; BODY_COUNT] {
    let mut transforms = default_body_transforms();
    let deck_mass_frame = retail_deck_mass_properties().local_mass_frame;
    transforms[BodyId::Deck.index()] = compose_affine(
        transforms[BodyId::Deck.index()],
        inverse_affine(RetailAffineTransform {
            basis: deck_mass_frame.basis,
            translation: deck_mass_frame.translation,
        }),
    );
    transforms
}

/// Quaternions written beside the live center-of-mass transforms by
/// `Part::SetTransform`.
pub fn default_live_body_orientations() -> [RetailQuaternion; BODY_COUNT] {
    default_live_body_transforms().map(|transform| retail_quaternion_from_basis(transform.basis))
}

/// Deck-center height that places the initialized wheel spheres exactly on a
/// level Y=0 riding surface.
///
/// This is a geometry-derived fixture placement, not a tuned suspension or
/// contact constant.
pub const RETAIL_LEVEL_GROUND_DECK_CENTER_HEIGHT: f32 =
    RETAIL_WHEEL_RADIUS - RETAIL_TRUCK_Y_POSITION;

/// Zero-steering truck/deck frames in native drive order. TU3 consumes the
/// `+7776` transform before the `+7712` transform.
pub fn default_truck_drive_frames() -> [RetailDriveFrames; 2] {
    let base = default_truck_transforms();
    [
        set_drive_frames_2(RetailAffineTransform::IDENTITY, base[1]),
        set_drive_frames_2(RetailAffineTransform::IDENTITY, base[0]),
    ]
}

/// Wheel/truck drive frames emitted by the four `CreateWheelDrives` calls.
pub fn default_wheel_drive_frames() -> [RetailDriveFrames; 4] {
    let positive = wheel_drive_frame(AuthoredTransformInputs::STOCK.wheel_x_distance);
    let negative = wheel_drive_frame(-AuthoredTransformInputs::STOCK.wheel_x_distance);
    [positive, negative, positive, negative]
}

/// Matrix stage of `SkateboardBody::SetDriveFrames2`.
///
/// The first input is the parent/first body's world transform and the second
/// input is the child/second body's world transform. TU3 writes:
///
/// - identity to the child frame through `SetChildFrame` (`0x82BD3A10`);
/// - `transpose(parent.basis) * child.basis` and the corresponding relative
///   translation to the parent frame through `SetParentFrame` (`0x82BD3BD0`).
///
pub fn set_drive_frames_2(
    parent_world: RetailAffineTransform,
    child_world: RetailAffineTransform,
) -> RetailDriveFrames {
    let parent_frame = relative_parent_transform(parent_world, child_world);
    let child_frame = RetailDriveFrame {
        orientation: RetailQuaternion::IDENTITY,
        translation: Vector3::ZERO,
    };

    RetailDriveFrames {
        body_a: child_frame,
        body_b: RetailDriveFrame {
            orientation: retail_quaternion_from_basis(parent_frame.basis),
            translation: parent_frame.translation,
        },
    }
}

/// Dominant-component matrix-to-quaternion calculation in TU3 child/parent
/// frame setters `0x82BD3A10` and `0x82BD3BD0`.
///
/// The function evaluates the same four component candidates as the vector
/// implementation, selects trace/X/Y/Z with its strict comparisons, and uses
/// the accepted Xenon reciprocal-square-root estimate with two recovered fused
/// refinements. Matrix storage remains column-major `Ri`, `Up`, `At`.
pub fn retail_quaternion_from_basis(basis: Basis3) -> RetailQuaternion {
    let m00 = basis.columns[0][0];
    let m01 = basis.columns[1][0];
    let m02 = basis.columns[2][0];
    let m10 = basis.columns[0][1];
    let m11 = basis.columns[1][1];
    let m12 = basis.columns[2][1];
    let m20 = basis.columns[0][2];
    let m21 = basis.columns[1][2];
    let m22 = basis.columns[2][2];

    let trace = (m00 + m11) + m22;
    if trace > 0.0 {
        quaternion_candidate(
            (m00 + m11) + (m22 + 1.0),
            [m21 - m12, m02 - m20, m10 - m01, 0.0],
            3,
        )
    } else if m00 > m11 && m00 > m22 {
        quaternion_candidate(
            (m00 - m11) + ((0.0 - m22) + 1.0),
            [0.0, m01 + m10, m02 + m20, m21 - m12],
            0,
        )
    } else if m11 > m22 {
        quaternion_candidate(
            ((0.0 - m00) + m11) + ((0.0 - m22) + 1.0),
            [m01 + m10, 0.0, m12 + m21, m02 - m20],
            1,
        )
    } else {
        quaternion_candidate(
            ((0.0 - m00) - m11) + (m22 + 1.0),
            [m02 + m20, m12 + m21, 0.0, m10 - m01],
            2,
        )
    }
}

fn refined_reciprocal_square_root(value: f32) -> f32 {
    let mut estimate = native_arithmetic::reciprocal_square_root_estimate(value);
    for _ in 0..2 {
        let square = estimate * estimate;
        let half_estimate = estimate * 0.5;
        let residual = (-value).mul_add(square, 1.0);
        estimate = half_estimate.mul_add(residual, estimate);
    }
    estimate
}

fn wheel_drive_frame(z_offset: f32) -> RetailDriveFrames {
    RetailDriveFrames {
        body_a: RetailDriveFrame {
            orientation: RetailQuaternion::IDENTITY,
            translation: Vector3::ZERO,
        },
        body_b: RetailDriveFrame {
            orientation: RetailQuaternion::IDENTITY,
            translation: Vector3::new(0.0, 0.0, z_offset),
        },
    }
}

fn inverse_affine(transform: RetailAffineTransform) -> RetailAffineTransform {
    let columns = transform.basis.columns;
    let inverse_basis = Basis3 {
        columns: [
            [columns[0][0], columns[1][0], columns[2][0]],
            [columns[0][1], columns[1][1], columns[2][1]],
            [columns[0][2], columns[1][2], columns[2][2]],
        ],
    };
    let translated = multiply_basis_vector(inverse_basis, transform.translation);
    RetailAffineTransform {
        basis: inverse_basis,
        translation: Vector3::new(0.0 - translated.x, 0.0 - translated.y, 0.0 - translated.z),
    }
}

fn compose_affine(
    parent: RetailAffineTransform,
    local: RetailAffineTransform,
) -> RetailAffineTransform {
    let translated = multiply_basis_vector(parent.basis, local.translation);
    RetailAffineTransform {
        basis: multiply_basis(parent.basis, local.basis),
        translation: Vector3::new(
            translated.x + parent.translation.x,
            translated.y + parent.translation.y,
            translated.z + parent.translation.z,
        ),
    }
}

fn multiply_basis_vector(basis: Basis3, vector: Vector3) -> Vector3 {
    let component = |lane| {
        let first = vector.x * basis.columns[0][lane];
        let second = vector.y.mul_add(basis.columns[1][lane], first);
        vector.z.mul_add(basis.columns[2][lane], second)
    };
    Vector3::new(component(0), component(1), component(2))
}

fn quaternion_candidate(sum: f32, companion: [f32; 4], dominant: usize) -> RetailQuaternion {
    let inverse_root = refined_reciprocal_square_root(sum);
    let half_inverse_root = inverse_root * 0.5;
    let half_root = (sum * inverse_root) * 0.5;
    let mut lanes = companion.map(|value| value * half_inverse_root);
    lanes[dominant] = half_root;
    RetailQuaternion {
        x: lanes[0],
        y: lanes[1],
        z: lanes[2],
        w: lanes[3],
    }
}

impl From<RetailDriveFrames> for RetailDriveFramesRaw {
    fn from(frames: RetailDriveFrames) -> Self {
        Self {
            body_a: raw_frame(frames.body_a),
            body_b: raw_frame(frames.body_b),
        }
    }
}

fn raw_frame(frame: RetailDriveFrame) -> RetailDriveFrameRaw {
    RetailDriveFrameRaw {
        quaternion_lanes: [
            frame.orientation.x.to_bits(),
            frame.orientation.y.to_bits(),
            frame.orientation.z.to_bits(),
            frame.orientation.w.to_bits(),
        ],
        translation_lanes: [
            frame.translation.x.to_bits(),
            frame.translation.y.to_bits(),
            frame.translation.z.to_bits(),
            0,
        ],
    }
}

pub fn calculate_truck_transforms(
    inputs: RetailTruckTransformInputs,
) -> [RetailAffineTransform; 2] {
    const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3C8E_FA35);
    const HALF: f32 = f32::from_bits(0x3F00_0000);
    const NEGATIVE_HALF_PI: f32 = f32::from_bits(0xBFC9_0FDB);
    const POSITIVE_HALF_PI: f32 = f32::from_bits(0x3FC9_0FDB);

    let alpha = inputs.truck_rotation_axis_angle_degrees * DEGREES_TO_RADIANS;
    let front_z = inputs
        .deck_mid_length
        .mul_add(HALF, inputs.truck_z_position_front);
    let back_z = -inputs
        .deck_mid_length
        .mul_add(HALF, inputs.truck_z_position_back);

    [
        RetailAffineTransform {
            basis: multiply_basis(rotation_x(alpha), rotation_y(POSITIVE_HALF_PI)),
            translation: Vector3::new(0.0, inputs.truck_y_position, back_z),
        },
        RetailAffineTransform {
            basis: multiply_basis(rotation_x(-alpha), rotation_y(NEGATIVE_HALF_PI)),
            translation: Vector3::new(0.0, inputs.truck_y_position, front_z),
        },
    ]
}

fn rotation_x(angle: f32) -> Basis3 {
    let (sin, cos) = trigonometry::sin_cos(angle);
    Basis3 {
        columns: [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]],
    }
}

fn rotation_y(angle: f32) -> Basis3 {
    let (sin, cos) = trigonometry::sin_cos(angle);
    Basis3 {
        columns: [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]],
    }
}

fn multiply_basis(left: Basis3, right: Basis3) -> Basis3 {
    Basis3 {
        columns: right.columns.map(|column| {
            core::array::from_fn(|lane| {
                let first = column[0] * left.columns[0][lane];
                let second = column[1].mul_add(left.columns[1][lane], first);
                column[2].mul_add(left.columns[2][lane], second)
            })
        }),
    }
}

fn relative_parent_transform(
    parent_world: RetailAffineTransform,
    child_world: RetailAffineTransform,
) -> RetailAffineTransform {
    RetailAffineTransform {
        basis: transpose_multiply_basis(parent_world.basis, child_world.basis),
        translation: transpose_relative_translation(
            parent_world.basis,
            parent_world.translation,
            child_world.translation,
        ),
    }
}

fn transpose_multiply_basis(parent: Basis3, child: Basis3) -> Basis3 {
    Basis3 {
        columns: core::array::from_fn(|column| {
            let child_column = child.columns[column];
            core::array::from_fn(|lane| {
                let parent_column = parent.columns[lane];
                let first = child_column[0] * parent_column[0];
                let second = child_column[1].mul_add(parent_column[1], first);
                child_column[2].mul_add(parent_column[2], second)
            })
        }),
    }
}

fn transpose_relative_translation(
    parent_basis: Basis3,
    parent_translation: Vector3,
    child_translation: Vector3,
) -> Vector3 {
    let neg_parent = Vector3::new(
        0.0 - parent_translation.x,
        0.0 - parent_translation.y,
        0.0 - parent_translation.z,
    );
    let component = |parent_column: [f32; 3]| {
        let result = neg_parent.z * parent_column[2];
        let result = neg_parent.y.mul_add(parent_column[1], result);
        let result = neg_parent.x.mul_add(parent_column[0], result);
        let result = child_translation.x.mul_add(parent_column[0], result);
        let result = child_translation.y.mul_add(parent_column[1], result);
        child_translation.z.mul_add(parent_column[2], result)
    };
    Vector3::new(
        component(parent_basis.columns[0]),
        component(parent_basis.columns[1]),
        component(parent_basis.columns[2]),
    )
}

#[cfg(test)]
#[path = "tests/drive_frames.rs"]
mod tests;
