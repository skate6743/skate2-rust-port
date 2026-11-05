//! Complete kind3/status&12 dispatch82BDAAA0..82BDACE4.
//! Bone identity is resolved even for an unknown event name. Push speed comes
//! from the evaluated trajectory displacement, not a button-driven force.
use super::{name::encode, scalar_attributes::ScalarAttributeInputs};
use crate::{
    animation::output::{
        NativeMatrix,
        attributes::{AnimationAttribute, AttributeName},
    },
    physics::native_arithmetic::{dot3, reciprocal_square_root_estimate},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactEventState {
    /// ProcessedPhysIn2560, including native GetBone's -1 result.
    pub bone: i32,
    /// ProcessedPhysIn2608, consumed by the native push-force calculation.
    pub push_speed: f32,
}

pub struct ContactEventPose<'a> {
    pub bone_names: &'a [AttributeName],
    pub hierarchy: &'a [NativeMatrix],
    /// Skeleton11716, assigned by SkeletonData initialization.
    pub trajectory_bone: usize,
    /// Skeleton4748: animation index of physical part19 (RIGHTTOEBASE).
    pub right_toe_bone: i32,
    /// Skeleton11704: current animation/physics input timestep.
    pub timestep: f32,
}

pub fn dispatch(
    attribute: &AnimationAttribute,
    pose: &ContactEventPose<'_>,
    fields: &mut ScalarAttributeInputs,
    state: &mut ContactEventState,
) -> Result<(), String> {
    if attribute.kind != 3 || attribute.status & 12 == 0 {
        return Ok(());
    }
    let mut words = [0; 6];
    for (destination, source) in words.iter_mut().zip(attribute.payload.0) {
        *destination = source.ok_or("Active animation bone event has incomplete payload")?;
    }
    let name = AttributeName(words[..5].try_into().unwrap());
    let strength = f32::from_bits(words[5]);
    state.bone = pose
        .bone_names
        .iter()
        .position(|&bone| bone == name)
        .map_or(-1, |i| i as i32);
    if attribute.name == encode(b"push_contact") {
        fields.flags2468 |= 1 << 27;
        replace(&mut fields.flags2468, 26, state.bone == pose.right_toe_bone);
        replace(&mut fields.flags2468, 25, strength == 1.0);
        replace(&mut fields.flags2468, 24, strength == -1.0);
        let translation = pose
            .hierarchy
            .get(pose.trajectory_bone)
            .ok_or("Push contact requires the actual animation trajectory bone")?[3];
        let squared = dot3(translation, translation);
        let mut inverse = reciprocal_square_root_estimate(squared);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.0), inverse);
        }
        // Native clears a zero-length result before multiplying by scalar
        // fdivs(1,dt). Estimate arithmetic remains hardware-unverified.
        let length = if squared == 0.0 {
            0.0
        } else {
            squared * inverse
        };
        let speed = length * (1.0 / pose.timestep);
        state.push_speed = if strength == 1.0 { speed } else { 0.0 };
    } else if attribute.name == encode(b"brake_contact") {
        fields.flags2468 |= 1 << 28;
        replace(&mut fields.flags2468, 30, strength == 1.0);
        replace(&mut fields.flags2468, 23, strength == -1.0);
    } else if attribute.name == encode(b"right_hand_grab")
        || attribute.name == encode(b"left_hand_grab")
    {
        // The payload bone name, not the event's left/right label, selects it.
        fields.flags2472 |= if name == encode(b"righthand") {
            0x100
        } else {
            0x80
        };
    }
    Ok(())
}

fn replace(flags: &mut u32, bit: u32, set: bool) {
    *flags = (*flags & !(1 << bit)) | (u32::from(set) << bit);
}
