//! Original82BE7860/82BE7280 and exact locations in stock physics_skeleton XML.
use skate3_core::physics::{
    contact::RetailContactMaterial,
    skeleton_body::{SkeletonCollisionSettings, SkeletonFeedbackSettings},
};
use skate_data::{collections::Collections, physics_skeleton::PhysicsSkeleton};
use std::path::Path;

pub(super) fn load(
    asset_root: &Path,
    data: &Collections,
    bank_sha: &str,
) -> Result<SkeletonCollisionSettings, String> {
    let skeleton = PhysicsSkeleton::load(
        &asset_root.join("private/stock/physics-skeletons.json"),
        bank_sha,
        "PHYS_TPOSE",
    )?;
    if skeleton.bones.len() != 24 {
        return Err("Skeleton collision requires24 bone records".into());
    }
    let mut compliant = [false; 24];
    let mut priority = [0.0; 24];
    compliant[0] = true; //82BE7484; part0 priority is cleared by collisionReset.
    for part in 1..24 {
        let words = data.words::<9>(
            "physics_skeleton_drives",
            "default",
            &format!("PART_{}", skeleton.bones[part].name),
        )?;
        compliant[part] = words[7] >> 24 != 0;
        priority[part] = f32::from_bits(words[8]);
    }
    // XML baseC8840; frictionC8904, restitutionC88EC, enabledC8910,
    // effectC8918 => native offsets196,172,208,216 respectively.
    let friction = data.float("physics_skeleton", "default", "FrictionNormal")?;
    Ok(SkeletonCollisionSettings {
        enabled: data.boolean("physics_skeleton", "default", "EnableCollision")?,
        normal_material: RetailContactMaterial {
            static_friction: friction,
            dynamic_friction: friction,
            restitution: data.float("physics_skeleton", "default", "RestitutionNormal")?,
        },
        compliant,
        priority,
        effect_time: data.float("physics_skeleton", "default", "CollisionEffectTime")?,
    })
}

/// GlobalAttributeCollection+268 binds original low hashes7AA75DF9/2D7D2152
/// at8289F634: physics_collision/default. Stock XML baseC6AE0 independently
/// resolves every layout offset consumed by82BD4A30 and82BD6038.
pub(super) fn feedback(
    data: &Collections,
    body: SkeletonCollisionSettings,
) -> Result<SkeletonFeedbackSettings, String> {
    let value = |name| data.float("physics_collision", "default", name);
    let vector = |name| -> Result<[f32; 4], String> {
        Ok(data
            .words::<4>("physics_collision", "default", name)?
            .map(f32::from_bits))
    };
    Ok(SkeletonFeedbackSettings {
        body,
        response: skate3_core::physics::skeleton_body::CollisionResponseSettings {
            force_scale: value("Hash_FBF5CD7734E8AE5A")?, //120
            velocity_scale: value("Hash_AD7ECC44569EBEF9")?, //124
            divisor: value("Hash_06C10E1F5E1B4730")?, //128
            region_scale: value("Hash_1430BD50F0A33475")?, //164
        },
        small_object_mass: data.float("physics_skeleton", "default", "SmallObjectMassThreshold")?,
        ground_plane_max_distance: value("GroundPlaneMaxDist")?, //136
        ground_plane_max_angle: value("GroundPlaneMaxAngle")?,   //140
        skater_scalar: value("SkaterSkeletonScalar")?,           //116
        ai_scalar: value("AISkeletonScalar")?,                   //188
        groin_offset: vector("GroinLocalOffset")?,               //0
        face_offset: vector("FaceLocalOffset")?,                 //16
        groin_radius: value("GroinRadius")?,                     //144
        face_radius: value("FaceRadius")?,                       //168
    })
}
