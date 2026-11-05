//! Authored inputs to normal82BE58A8 and ragdoll82BE59A8/6D60.
use skate3_core::physics::contact::RetailContactMaterial;
use skate_data::{collections::Collections, physics_skeleton::PhysicsSkeleton};
use std::path::Path;

pub(super) struct Settings {
    pub normal_limits: [[u32; 4]; 22],
    pub ragdoll_limits: [[u32; 4]; 22],
    pub inverse_mass: bool,
    pub inverse_inertia: bool,
    pub drag: [f32; 2],
    pub materials: [RetailContactMaterial; 2],
}
impl Settings {
    pub fn load(data: &Collections, assets: &Path, bank_sha: &str) -> Result<Self, String> {
        let skeleton = PhysicsSkeleton::load(
            &assets.join("private/stock/physics-skeletons.json"),
            bank_sha,
            "PHYS_TPOSE",
        )?;
        if skeleton.bones.len() != 24 {
            return Err("Ragdoll requires24 physical bones".into());
        }
        let names = [
            "JOINT_NECK_NECK1",
            "JOINT_SPINE3_NECK",
            "JOINT_LEFT_FOREARM_HAND",
            "JOINT_LEFT_ARM_FOREARM",
            "JOINT_LEFT_SHOULDER_ARM",
            "JOINT_SPINE3_LEFT_SHOULDER",
            "JOINT_RIGHT_FOREARM_HAND",
            "JOINT_RIGHT_ARM_FOREARM",
            "JOINT_RIGHT_SHOULDER_ARM",
            "JOINT_SPINE3_RIGHT_SHOULDER",
            "JOINT_SPINE2_SPINE3",
            "JOINT_SPINE1_SPINE2",
            "JOINT_SPINE_SPINE1",
            "JOINT_HIPS_SPINE",
            "JOINT_LEFT_FOOT_TOE_BASE",
            "JOINT_LEFT_LEG_FOOT",
            "JOINT_LEFT_UPLEG_LEG",
            "JOINT_HIPS_LEFT_LEG",
            "JOINT_RIGHT_FOOT_TOE_BASE",
            "JOINT_RIGHT_LEG_FOOT",
            "JOINT_RIGHT_UPLEG_LEG",
            "JOINT_HIPS_RIGHT_LEG",
        ];
        let swing = data.float("physics_skeleton_joints", "default", "SwingRagdollScalar")?;
        let twist = data.float("physics_skeleton_joints", "default", "TwistRagdollScalar")?;
        let mut normal_limits = [[0; 4]; 22];
        let mut ragdoll_limits = [[0; 4]; 22];
        for (i, name) in names.into_iter().enumerate() {
            let words = data
                .words::<5>("physics_skeleton_joints", "default", name)?
                .map(f32::from_bits);
            let bone = &skeleton.bones[i + 1];
            let s = f32::from_bits(bone.words[21]);
            let t = f32::from_bits(bone.words[20]);
            normal_limits[i] = limits(s * words[1], t * words[2]);
            ragdoll_limits[i] = limits((s * words[3]) * swing, (t * words[4]) * twist);
        }
        let material = |friction, restitution| -> Result<_, String> {
            let friction = data.float("physics_skeleton", "default", friction)?;
            Ok(RetailContactMaterial {
                static_friction: friction,
                dynamic_friction: friction,
                restitution: data.float("physics_skeleton", "default", restitution)?,
            })
        };
        let scale = f32::from_bits(0x426f_ffff);
        Ok(Self {
            normal_limits,
            ragdoll_limits,
            inverse_mass: data.boolean("physics_wipeout", "default", "DoInverseMass")?,
            inverse_inertia: data.boolean("physics_wipeout", "default", "DoInverseInertia")?,
            drag: [
                data.float("physics_wipeout", "default", "LinearDrag")? * scale,
                data.float("physics_wipeout", "default", "AngularDrag")? * scale,
            ],
            materials: [
                material("FrictionRagdoll", "RestitutionRagdoll")?,
                material("FrictionRagdollHead", "RestitutionRagdollHead")?,
            ],
        })
    }
}
fn limits(swing: f32, twist: f32) -> [u32; 4] {
    let swing = if swing - 0.01 >= 0.0 { swing } else { 0.01 };
    let twist = if twist - 0.01 >= 0.0 { twist } else { 0.01 };
    [
        swing,
        twist,
        skate3_core::trigonometry::cos(swing),
        skate3_core::trigonometry::cos(twist),
    ]
    .map(f32::to_bits)
}
