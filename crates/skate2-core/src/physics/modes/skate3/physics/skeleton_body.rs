//! Stock physical body construction. Initial mapped pose and spawn are real
//! actor/physics inputs supplied by the coordinator; no pose or COM fallback.
use skate3_core::{
    math::Vector3,
    physics::{
        rigid_body::RetailSimulationStep,
        skeleton_animation_record::AnimationPartTransform,
        skeleton_body::{
            BoneSettings, HatGeometry, SkeletonBody, SkeletonBodyDefinition, SkeletonBodySettings,
        },
    },
};
use skate_data::{collections::Collections, physics_skeleton::PhysicsSkeleton};
use std::path::Path;
#[path = "skeleton_body_collision_settings.rs"]
mod collision_settings;
#[path = "skeleton_body_drive_settings.rs"]
mod drive_settings;

pub(crate) fn load_collision(
    asset_root: &Path,
    data: &Collections,
    bank_sha: &str,
    cull_all_self_pairs: bool,
) -> Result<skate3_core::physics::skeleton_body::SkeletonCollisionMode, String> {
    Ok(
        skate3_core::physics::skeleton_body::SkeletonCollisionMode::new_normal(
            collision_settings::load(asset_root, data, bank_sha)?,
            cull_all_self_pairs,
        ),
    )
}

pub(crate) fn load_feedback(
    data: &Collections,
    settings: skate3_core::physics::skeleton_body::SkeletonCollisionSettings,
) -> Result<skate3_core::physics::skeleton_body::SkeletonCollisionFeedback, String> {
    Ok(
        skate3_core::physics::skeleton_body::SkeletonCollisionFeedback::new(
            collision_settings::feedback(data, settings)?,
        ),
    )
}

/// Build the complete original26-part drive owner using the already-resolved
/// joint topology. The initial bone matrices precede physical-volume mapping.
pub(crate) fn load_drives(
    asset_root: &Path,
    data: &Collections,
    animation_bank_sha256: &str,
    initial_hierarchy: &[AnimationPartTransform],
    bone_indices: &[usize; 24],
    initial_mapped: &[AnimationPartTransform; 24],
    joints: &skate3_core::physics::skeleton_body::SkeletonJoints,
    animation_to_world: AnimationPartTransform,
    spawn: AnimationPartTransform,
    simulation: RetailSimulationStep,
) -> Result<skate3_core::physics::skeleton_body::SkeletonDrives, String> {
    use skate3_core::physics::skeleton_body::{SkeletonDriveSettings, SkeletonDrives};
    if bone_indices.iter().any(|i| *i >= initial_hierarchy.len()) {
        return Err("Skeleton drive initial hierarchy has missing bones".into());
    }
    let skeleton = PhysicsSkeleton::load(
        &asset_root.join("private/stock/physics-skeletons.json"),
        animation_bank_sha256,
        "PHYS_TPOSE",
    )?;
    if skeleton.bones.len() != 24 {
        return Err("Skeleton drives require24 bone records".into());
    }
    let mut parents = [None; 24];
    for joint in &joints.records {
        parents[joint.child] = Some(joint.parent);
    }
    let initial_bones = std::array::from_fn(|i| initial_hierarchy[bone_indices[i]]);
    let mut collision_strength = [[0.0; 2]; 24];
    //82BE7B98..BAC builds057EDDFA341F9492=CollisionDriveScalar.
    let scalar = data.float("physics_skeleton_drives", "default", "CollisionDriveScalar")?;
    for part in 1..24 {
        let name = format!("PART_{}", skeleton.bones[part].name);
        let words = data.words::<9>("physics_skeleton_drives", "default", &name)?;
        collision_strength[part] = [
            f32::from_bits(words[5]) * scalar,
            f32::from_bits(words[6]) * scalar,
        ];
    }
    let settings = SkeletonDriveSettings {
        bone: drive_settings::load(data)?,
        enabled: data.boolean("animation", "default", "Use_Drives")?,
        strength: [
            data.float("animation", "default", "DriveStrengthLocal")?,
            data.float("animation", "default", "DriveStrengthRootLocal")?,
        ],
        collision_strength,
    };
    SkeletonDrives::new(
        &initial_bones,
        initial_mapped,
        &parents,
        animation_to_world,
        spawn,
        simulation,
        settings,
    )
    .map_err(str::to_string)
}

/// SkeletonData::Init82BD6C40 / closest physical ancestor82BD3720.
/// Both the rig pose and hierarchy are the actor's evaluated initial RIG_TPOSE.
pub(crate) fn load_joints(
    asset_root: &Path,
    data: &Collections,
    animation_bank_sha256: &str,
    initial_hierarchy: &[AnimationPartTransform],
    hierarchy_parents: &[i32],
    bone_indices: &[usize; 24],
) -> Result<skate3_core::physics::skeleton_body::SkeletonJoints, String> {
    use skate3_core::physics::{
        skeleton_animation_record::physics_bone_frame,
        skeleton_body::{JointBone, JointSettings, SkeletonJointSettings, SkeletonJoints},
    };
    if hierarchy_parents.len() != initial_hierarchy.len()
        || bone_indices.iter().any(|i| *i >= initial_hierarchy.len())
    {
        return Err("Physical joint hierarchy differs from initial rig pose".into());
    }
    let skeleton = PhysicsSkeleton::load(
        &asset_root.join("private/stock/physics-skeletons.json"),
        animation_bank_sha256,
        "PHYS_TPOSE",
    )?;
    if skeleton.bones.len() != 24 {
        return Err("Physical joints require24 bone records".into());
    }
    let initial_bones = std::array::from_fn(|i| initial_hierarchy[bone_indices[i]]);
    let mut parents = [None; 24];
    for child in 0..24 {
        let mut parent = hierarchy_parents[bone_indices[child]];
        let mut visited = 0;
        while parent != -1 {
            let index = usize::try_from(parent).map_err(|_| "Invalid animation parent index")?;
            if index >= hierarchy_parents.len() || visited >= hierarchy_parents.len() {
                return Err("Invalid or cyclic animation hierarchy".into());
            }
            if let Some(physical) = bone_indices.iter().position(|bone| *bone == index) {
                parents[child] = Some(physical);
                break;
            }
            parent = hierarchy_parents[index];
            visited += 1;
        }
    }
    let bones = std::array::from_fn(|i| {
        let bone = &skeleton.bones[i];
        let [x, y, z] = bone.translation();
        JointBone {
            parent_orientation: std::array::from_fn(|lane| f32::from_bits(bone.words[lane])),
            joint_orientation: std::array::from_fn(|lane| f32::from_bits(bone.words[4 + lane])),
            volume_frame: physics_bone_frame(bone.rotation(), [x, y, z, 0.0]),
            swing_limit: f32::from_bits(bone.words[21]),
            twist_limit: f32::from_bits(bone.words[20]),
        }
    });
    // InitAttribData stores these in ascending physical child order1..22.
    let names = JOINT_NAMES;
    let mut settings = Vec::with_capacity(22);
    for name in names {
        let words = data.words::<5>("physics_skeleton_joints", "default", name)?;
        settings.push(JointSettings {
            ball_joint: words[0] >> 24 != 0,
            swing_angle: f32::from_bits(words[1]),
            twist_angle: f32::from_bits(words[2]),
            swing_ragdoll: f32::from_bits(words[3]),
            twist_ragdoll: f32::from_bits(words[4]),
        });
    }
    let global = SkeletonJointSettings {
        displacement_limit: data
            .words::<4>("physics_skeleton", "default", "DisplacementLimit")?
            .map(f32::from_bits),
        twist_displacement_limit: data.float(
            "physics_skeleton",
            "default",
            "TwistDisplacementLimit",
        )?,
        swing_displacement_limit: data.float(
            "physics_skeleton",
            "default",
            "SwingDisplacementLimit",
        )?,
        enforce_swing_free: data.boolean(
            "physics_skeleton_joints",
            "default",
            "EnforceSwingFree",
        )?,
        enforce_twist_free: data.boolean(
            "physics_skeleton_joints",
            "default",
            "EnforceTwistFree",
        )?,
    };
    SkeletonJoints::new(
        &initial_bones,
        &parents,
        &bones,
        &settings
            .try_into()
            .map_err(|_| "Physical joint setting count")?,
        global,
    )
    .map_err(str::to_string)
}

pub(crate) fn load(
    asset_root: &Path,
    data: &Collections,
    animation_bank_sha256: &str,
    initial_mapped_pose: &[AnimationPartTransform; 24],
    spawn: AnimationPartTransform,
    simulation: RetailSimulationStep,
    hat_collection: Option<&str>,
) -> Result<SkeletonBody, String> {
    let skeleton = PhysicsSkeleton::load(
        &asset_root.join("private/stock/physics-skeletons.json"),
        animation_bank_sha256,
        "PHYS_TPOSE",
    )?;
    if skeleton.bones.len() != 24 {
        return Err("Physical skeleton requires24 bone records".into());
    }
    let mut sizes = [Vector3::ZERO; 24];
    let mut bones = Vec::with_capacity(24);
    for (i, bone) in skeleton.bones.iter().enumerate() {
        let [x, y, z] = bone.size();
        sizes[i] = Vector3::new(x, y, z);
        let w = data.words::<6>(
            "physics_skeleton_bones",
            "default",
            &format!("PART_{}", bone.name),
        )?;
        bones.push(BoneSettings {
            mass_factor: f32::from_bits(w[0]),
            ragdoll_mass_factor: f32::from_bits(w[1]),
            has_collision: w[2] >> 24 != 0,
            use_root_drive: (w[2] >> 16) & 255 != 0,
            volume_type: w[3],
            volume_scalar: f32::from_bits(w[4]),
            num_parents: w[5],
        });
    }
    let value = |name| data.float("physics_skeleton", "default", name);
    let settings = SkeletonBodySettings {
        density: value("MassOfSkeleton")?,
        root_radius: value("SkateRootCapsuleRadius")?,
        root_half_length: value("SkateRootCapsuleLength")?,
        capsule_radius_scalar: data.float(
            "physics_skeleton_bones",
            "default",
            "CapsuleRadiusScalar",
        )?,
        capsule_length_scalar: value("CapsuleLengthScalar")?,
        ragdoll_inverse_mass_factor: data.float(
            "physics_wipeout",
            "default",
            "RagdollInvMassFactor",
        )?,
        inertia_multiply_type: data.integer("animation", "default", "InertiaMultiplyType")?,
        inertia_factor: data.float("animation", "default", "InertiaMultFactor")?,
    };
    let hat = if let Some(key) = hat_collection {
        let vector = |name| -> Result<Vector3, String> {
            let w = data
                .words::<4>("physics_hat", key, name)?
                .map(f32::from_bits);
            Ok(Vector3::new(w[0], w[1], w[2]))
        };
        Some(HatGeometry::from_offsets(
            data.float("physics_hat", key, "Hash_372DF5DB27AF1C79")?,
            data.float("physics_hat", key, "Thickness")?,
            vector("OrientationOffset")?,
            vector("PosOffset")?,
        ))
    } else {
        None
    };
    let definition = SkeletonBodyDefinition::new(
        sizes,
        bones.try_into().map_err(|_| "Skeleton bone count")?,
        settings,
        hat,
    )?;
    Ok(SkeletonBody::new(
        definition,
        initial_mapped_pose,
        spawn,
        simulation,
    ))
}

pub(crate) const JOINT_NAMES: [&str;22] = [
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
