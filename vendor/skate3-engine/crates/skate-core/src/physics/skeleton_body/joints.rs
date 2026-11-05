//! SkeletonBody::InitJoints82BE5B30 and SetUpNormalJointLimits82BE58A8.
//! Frames attach the child to its nearest physical ancestor; the initial
//! animation bone poses and authored volume frames are distinct inputs.
use super::{ANIMATION_PART_COUNT, JOINT_COUNT, PART_COUNT};
use crate::{
    math::Basis3,
    physics::{
        assembly::BodySnapshot,
        constraint_batch::compile_active,
        drive_frames::retail_quaternion_from_basis,
        joint_builder::{RetailJointBodyInput, RetailJointBuildInput, build_retail_joint_jacobian},
        joint_records::{RetailJointFramesRaw, RetailJointParametersRaw},
        native_arithmetic,
        rigid_body::pack_world_inverse_inertia,
        skeleton_animation_record::{AnimationPartTransform, compose_affine, physics_bone_frame},
        skeleton_root::inverse_rigid,
        solver::JointConstraint,
    },
    trigonometry,
};

#[derive(Clone, Copy, Debug)]
pub struct JointBone {
    pub parent_orientation: [f32; 4],
    pub joint_orientation: [f32; 4],
    pub volume_frame: AnimationPartTransform,
    pub swing_limit: f32,
    pub twist_limit: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct JointSettings {
    pub ball_joint: bool,
    pub swing_angle: f32,
    pub twist_angle: f32,
    pub swing_ragdoll: f32,
    pub twist_ragdoll: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SkeletonJointSettings {
    pub displacement_limit: [f32; 4],
    pub twist_displacement_limit: f32,
    pub swing_displacement_limit: f32,
    pub enforce_swing_free: bool,
    pub enforce_twist_free: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SkeletonJoint {
    pub parent: usize,
    pub child: usize,
    pub parameters: RetailJointParametersRaw,
    pub frames: RetailJointFramesRaw,
}

pub struct SkeletonJoints {
    pub records: [SkeletonJoint; JOINT_COUNT],
}
impl SkeletonJoints {
    pub fn new(
        initial_bones: &[AnimationPartTransform; ANIMATION_PART_COUNT],
        parents: &[Option<usize>; ANIMATION_PART_COUNT],
        bones: &[JointBone; ANIMATION_PART_COUNT],
        settings: &[JointSettings; JOINT_COUNT],
        global: SkeletonJointSettings,
    ) -> Result<Self, &'static str> {
        let mut records = Vec::with_capacity(JOINT_COUNT);
        for child in 0..ANIMATION_PART_COUNT {
            let Some(parent) = parents[child] else {
                continue;
            };
            if parent >= ANIMATION_PART_COUNT || parent == child {
                return Err("Invalid physical skeleton joint parent");
            }
            let setting = *settings
                .get(records.len())
                .ok_or("Too many physical skeleton joints")?;
            let bone = bones[child];
            let inverse_child_volume = inverse_rigid(&bone.volume_frame);
            let inverse_parent_volume = inverse_rigid(&bones[parent].volume_frame);
            let parent_orientation = physics_bone_frame(bone.parent_orientation, [0.0; 4]);
            let joint_orientation = physics_bone_frame(bone.joint_orientation, [0.0; 4]);
            let parent_joint =
                compose_affine(&inverse_parent_volume, &inverse_rigid(&parent_orientation));
            let relative_bones = compose_affine(
                &inverse_rigid(&initial_bones[parent]),
                &initial_bones[child],
            );
            let parent_anchor = compose_affine(&inverse_parent_volume, &relative_bones)[3];
            let child_frame = compose_affine(&inverse_child_volume, &joint_orientation);
            let parent_frame = compose_affine(&parent_joint, &joint_orientation);
            let mut frames = RetailJointFramesRaw { words: [0; 20] };
            pack_frame(&mut frames.words[..8], child_frame, inverse_child_volume[3]);
            pack_frame(&mut frames.words[8..16], parent_frame, parent_anchor);
            // InitJoints passes the identity matrix to SetLinearFrame82BD3858.
            frames.words[19] = 1.0f32.to_bits();

            let mut parameters = RetailJointParametersRaw { words: [0; 16] };
            let frequency = inverse_native_step();
            parameters.words[4..8]
                .copy_from_slice(&global.displacement_limit.map(|v| (v * frequency).to_bits()));
            let scalar_frequency = f32::from_bits(0x426F_FFFF);
            parameters.words[8] = (global.twist_displacement_limit * scalar_frequency).to_bits();
            parameters.words[9] = (global.swing_displacement_limit * scalar_frequency).to_bits();
            parameters.words[14] = if setting.ball_joint || global.enforce_swing_free {
                4
            } else {
                1
            };
            parameters.words[15] = if setting.ball_joint || global.enforce_twist_free {
                2
            } else {
                1
            };
            // SetUpNormal runs after InitJoints and writes limits even for a
            // free/ball joint. It does not replace the mode selectors.
            let swing = minimum_angle(bone.swing_limit * setting.swing_angle);
            let twist = minimum_angle(bone.twist_limit * setting.twist_angle);
            parameters.words[10..14].copy_from_slice(
                &[
                    swing,
                    twist,
                    trigonometry::cos(swing),
                    trigonometry::cos(twist),
                ]
                .map(f32::to_bits),
            );
            records.push(SkeletonJoint {
                parent,
                child,
                parameters,
                frames,
            });
        }
        Ok(Self {
            records: records
                .try_into()
                .map_err(|_| "Physical skeleton requires22 joints")?,
        })
    }

    /// Append these to the same constraint pass as board contacts/joints/drives.
    /// Reaction indices are supplied by the unified simulation owner.
    pub fn build(
        &self,
        bodies: &[BodySnapshot; PART_COUNT],
        reaction_base: usize,
        time_step: f32,
    ) -> Vec<JointConstraint> {
        compile_active(
            &self.records,
            |record| {
                [
                    bodies[record.child].state_flags,
                    bodies[record.parent].state_flags,
                ]
            },
            |record| JointConstraint {
                jacobian: build_retail_joint_jacobian(RetailJointBuildInput {
                    parameters: record.parameters,
                    frames: record.frames,
                    body_a: joint_body(bodies[record.child]),
                    body_b: joint_body(bodies[record.parent]),
                    time_step,
                    joint_guest_address: 0,
                }),
                reaction_a: reaction_base + record.child,
                reaction_b: reaction_base + record.parent,
            },
        )
    }
}

fn inverse_native_step() -> f32 {
    let step = f32::from_bits(0x3C88_8889); //820849C8
    let mut reciprocal = native_arithmetic::reciprocal_estimate(step);
    for _ in 0..2 {
        reciprocal = reciprocal.mul_add((-reciprocal).mul_add(step, 1.0), reciprocal);
    }
    reciprocal
}
fn minimum_angle(value: f32) -> f32 {
    let minimum = f32::from_bits(0x3C23_D70A);
    if value - minimum >= 0.0 {
        value
    } else {
        minimum
    }
}
fn pack_frame(words: &mut [u32], frame: AnimationPartTransform, anchor: [f32; 4]) {
    let q = retail_quaternion_from_basis(Basis3 {
        columns: std::array::from_fn(|i| [frame[i][0], frame[i][1], frame[i][2]]),
    });
    words[..4].copy_from_slice(&[q.x, q.y, q.z, q.w].map(f32::to_bits));
    words[4..8].copy_from_slice(&anchor.map(f32::to_bits));
}
fn joint_body(body: BodySnapshot) -> RetailJointBodyInput {
    let rates = body.rates;
    RetailJointBodyInput {
        reaction_guest_address: 0,
        state: body.state_flags,
        orientation: rates.orientation,
        center_of_mass: rates.position,
        basis: rates.basis,
        linear_velocity: rates.linear_velocity,
        angular_velocity: rates.angular_velocity,
        force_acceleration: rates.force_acceleration,
        torque_acceleration: rates.torque_acceleration,
        inverse_mass: body.inertia.inverse_mass,
        world_inverse_inertia: pack_world_inverse_inertia(rates.world_inverse_inertia),
    }
}
