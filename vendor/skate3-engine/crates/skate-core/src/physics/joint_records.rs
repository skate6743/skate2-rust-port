//! TU3 skateboard joint definitions reconstructed from stock collection data.
//!
//! `SkateboardBody::CreateJoints` (`0x82C0C268`) builds two deck/truck joints
//! followed by four truck/wheel joints. This module emits the native parameter
//! and frame payloads that `Assembly::Initialize` consumes; it does not replace
//! them with a third-party constraint type.

use super::{
    board::BodyId,
    drive_frames::{
        AuthoredTransformInputs, RetailDriveFrame, RetailDriveFrames, default_truck_drive_frames,
        retail_quaternion_from_basis,
    },
    rigid_body::RetailQuaternion,
};
use crate::{
    math::{Basis3, Vector3},
    trigonometry,
};

pub mod tu3 {
    pub const CREATE_JOINTS: u32 = 0x82C0_C268;
    pub const ASSEMBLY_INITIALIZE: u32 = 0x82AD_FAF8;
    pub const ASSEMBLY_POPULATE: u32 = 0x82AD_FBB8;
    pub const FRAME_PACK_0: u32 = 0x82C0_0940;
    pub const FRAME_PACK_1: u32 = 0x82C0_0AF0;
    pub const FRAME_PACK_2: u32 = 0x82BD_3858;
}

pub const JOINT_COUNT: usize = 6;

const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3C8E_FA35);
const SIMULATION_FREQUENCY: f32 = f32::from_bits(0x426F_FFFF);
const HALF_PI: f32 = f32::from_bits(0x3FC9_0FDB);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailJointSettings {
    pub truck_twist_limit_degrees: f32,
    pub truck_twist_angle_degrees: f32,
    pub wheel_swing_limit_degrees: f32,
}

impl RetailJointSettings {
    /// Values decoded from the stock `physicstrucks` and `physicswheels`
    /// collection XMLs read by `CreateJoints`.
    pub const STOCK: Self = Self {
        truck_twist_limit_degrees: f32::from_bits(0x42AC_3333),
        truck_twist_angle_degrees: f32::from_bits(0x4160_0000),
        wheel_swing_limit_degrees: f32::from_bits(0x4974_2400),
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct RetailJointParametersRaw {
    pub words: [u32; 16],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct RetailJointFramesRaw {
    pub words: [u32; 20],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetailJointRecord {
    pub definition_body_0: BodyId,
    pub definition_body_1: BodyId,
    pub parameters: RetailJointParametersRaw,
    pub frames: RetailJointFramesRaw,
}

impl RetailJointRecord {
    /// Body written to live `Joint+0x10` by assembly activation.
    pub const fn live_body_a(self) -> BodyId {
        self.definition_body_1
    }

    /// Body written to live `Joint+0x14` by assembly activation.
    pub const fn live_body_b(self) -> BodyId {
        self.definition_body_0
    }
}

/// Complete stock record set in the unrolled write order of TU3
/// `SkateboardBody::CreateJoints`.
pub fn default_joint_records() -> [RetailJointRecord; JOINT_COUNT] {
    joint_records(RetailJointSettings::STOCK)
}

pub fn joint_records(settings: RetailJointSettings) -> [RetailJointRecord; JOINT_COUNT] {
    let truck_parameters = truck_parameters(settings);
    let wheel_parameters = wheel_parameters(settings);
    let truck_frames = default_truck_drive_frames().map(truck_joint_frames);
    let wheel_orientation = wheel_joint_orientation();
    let wheel_offset = AuthoredTransformInputs::STOCK.wheel_x_distance;
    let positive_wheel_frames = wheel_joint_frames(wheel_orientation, wheel_offset);
    let negative_wheel_frames = wheel_joint_frames(wheel_orientation, -wheel_offset);

    [
        joint(
            BodyId::Deck,
            BodyId::FrontTruck,
            truck_parameters,
            truck_frames[0],
        ),
        joint(
            BodyId::Deck,
            BodyId::BackTruck,
            truck_parameters,
            truck_frames[1],
        ),
        joint(
            BodyId::FrontTruck,
            BodyId::RightFrontWheel,
            wheel_parameters,
            positive_wheel_frames,
        ),
        joint(
            BodyId::FrontTruck,
            BodyId::LeftFrontWheel,
            wheel_parameters,
            negative_wheel_frames,
        ),
        joint(
            BodyId::BackTruck,
            BodyId::RightBackWheel,
            wheel_parameters,
            positive_wheel_frames,
        ),
        joint(
            BodyId::BackTruck,
            BodyId::LeftBackWheel,
            wheel_parameters,
            negative_wheel_frames,
        ),
    ]
}

fn truck_parameters(settings: RetailJointSettings) -> RetailJointParametersRaw {
    let angle_radians = settings.truck_twist_angle_degrees * DEGREES_TO_RADIANS;
    let (_, angle_cosine) = trigonometry::sin_cos(angle_radians);
    let twist_velocity =
        (settings.truck_twist_limit_degrees * DEGREES_TO_RADIANS) * SIMULATION_FREQUENCY;
    let mut words = [0; 16];
    words[8] = twist_velocity.to_bits();
    // The native parameter block retains the angular boundary in its spare
    // fourth lane while the solver consumes cos(angle) from word 13.
    words[11] = angle_radians.to_bits();
    words[12] = 1.0_f32.to_bits();
    words[13] = angle_cosine.to_bits();
    words[14] = 0;
    words[15] = 1;
    RetailJointParametersRaw { words }
}

fn wheel_parameters(settings: RetailJointSettings) -> RetailJointParametersRaw {
    let swing_velocity =
        (settings.wheel_swing_limit_degrees * DEGREES_TO_RADIANS) * SIMULATION_FREQUENCY;
    let mut words = [0; 16];
    words[9] = swing_velocity.to_bits();
    words[12] = 1.0_f32.to_bits();
    words[13] = 1.0_f32.to_bits();
    words[14] = 3;
    words[15] = 0;
    RetailJointParametersRaw { words }
}

fn truck_joint_frames(frames: RetailDriveFrames) -> RetailJointFramesRaw {
    pack_frames(frames.body_a, frames.body_b, frames.body_b.orientation)
}

fn wheel_joint_orientation() -> RetailQuaternion {
    let (sin, cos) = trigonometry::sin_cos(HALF_PI);
    retail_quaternion_from_basis(Basis3 {
        columns: [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]],
    })
}

fn wheel_joint_frames(orientation: RetailQuaternion, z_offset: f32) -> RetailJointFramesRaw {
    pack_frames(
        RetailDriveFrame {
            orientation,
            translation: Vector3::ZERO,
        },
        RetailDriveFrame {
            orientation,
            translation: Vector3::new(0.0, 0.0, z_offset),
        },
        orientation,
    )
}

fn pack_frames(
    body_a: RetailDriveFrame,
    body_b: RetailDriveFrame,
    linear_orientation_b: RetailQuaternion,
) -> RetailJointFramesRaw {
    let mut words = [0; 20];
    write_quaternion(&mut words, 0, body_a.orientation);
    write_vector(&mut words, 4, body_a.translation);
    write_quaternion(&mut words, 8, body_b.orientation);
    write_vector(&mut words, 12, body_b.translation);
    write_quaternion(&mut words, 16, linear_orientation_b);
    RetailJointFramesRaw { words }
}

fn write_quaternion(words: &mut [u32; 20], offset: usize, value: RetailQuaternion) {
    words[offset..offset + 4].copy_from_slice(&[
        value.x.to_bits(),
        value.y.to_bits(),
        value.z.to_bits(),
        value.w.to_bits(),
    ]);
}

fn write_vector(words: &mut [u32; 20], offset: usize, value: Vector3) {
    words[offset..offset + 3].copy_from_slice(&[
        value.x.to_bits(),
        value.y.to_bits(),
        value.z.to_bits(),
    ]);
}

const fn joint(
    definition_body_0: BodyId,
    definition_body_1: BodyId,
    parameters: RetailJointParametersRaw,
    frames: RetailJointFramesRaw,
) -> RetailJointRecord {
    RetailJointRecord {
        definition_body_0,
        definition_body_1,
        parameters,
        frames,
    }
}

#[cfg(test)]
#[path = "tests/joint_records.rs"]
mod tests;
