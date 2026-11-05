//! Stock physics_wipeout fields consumed by Biped collision check82D90148.
//! Retain this once with the physical player's off-board settings.
use skate3_core::player::offboard::ground_lifecycle::CollisionSettings;
use skate_data::collections::Collections;

pub(crate) fn load(data: &Collections) -> Result<CollisionSettings, String> {
    let value = |name| data.float("physics_wipeout", "default", name);
    Ok(CollisionSettings {
        vehicle_scalar: value("Wipeout_OB_VehicleScalar")?,
        vehicle_contact: value("Wipeout_OB_VehicleContact")?,
        maximum_displacement: value("Wipeout_OB_SkeletonMaxDisp")?,
        maximum_arm_contact: value("Wipeout_OB_SkeletonMaxContactArms")?,
        maximum_body_contact: value("Wipeout_OB_SkeletonMaxContact")?,
        minimum_speed: value("Wipeout_OB_MinSpeed")?,
        maximum_squash: value("Wipeout_OB_MaxSquash")?,
        special_scalar: value("Hash_472174920C68FBE3")?,
    })
}
