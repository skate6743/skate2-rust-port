//! Original BoneDrives constructor82BEC4A0/82BEC7B0 collection bindings.
//!82BEC50C..544 builds local=00132D1E84F0BC32;82BEC5B4..D4 builds
//! root=550B23128E0C3838. The12 transition keys preserve native offsets232..276.
use skate3_core::physics::skeleton_body::{
    AnimationDriveSettings, BoneDriveSettings, DriveInterpolation,
};
use skate_data::collections::Collections;

pub(super) fn load(data: &Collections) -> Result<BoneDriveSettings, String> {
    let scalar = |name| data.float("physics_skeleton_drives", "default", name);
    let animation = |key| -> Result<AnimationDriveSettings, String> {
        let f = |name| data.float("animation_drives", key, name);
        Ok(AnimationDriveSettings {
            linear_strength: f("LinearStrMin")?,
            linear_displacement: f("LinearDispMin")?,
            angular_strength: f("AngularStrMin")?,
            angular_displacement: f("AngularDispMin")?,
        })
    };
    Ok(BoneDriveSettings {
        animation: [animation("local")?, animation("root")?],
        collision_soft_displacement: scalar("CollisionSoftDisp")?,
        collision_soft_strength: scalar("CollisionSoftStrength")?,
        ragdoll_soft_displacement: scalar("RagdollSoftDisp")?,
        ragdoll_soft_strength: scalar("RagdollSoftStrength")?,
        transition_linear: DriveInterpolation {
            spring: [
                scalar("Hash_138976EAF99DF926")?,
                scalar("Hash_C313093118C7A83F")?,
            ],
            strength: [
                scalar("Hash_1E6BDF62043784FC")?,
                scalar("Hash_BD3E4F132848A588")?,
            ],
            damping: [
                scalar("Hash_D7652B4B19BB4747")?,
                scalar("Hash_C4B564DFD233CF04")?,
            ],
        },
        transition_angular: DriveInterpolation {
            spring: [
                scalar("Hash_14A2DDDD0A35C267")?,
                scalar("Hash_24D5AA7F214A901C")?,
            ],
            strength: [
                scalar("Hash_6C6CCD16A0A438AD")?,
                scalar("Hash_7379F8AB250BF611")?,
            ],
            damping: [
                scalar("Hash_28446A9E25DCC04C")?,
                scalar("Hash_B59D493B3DB372C8")?,
            ],
        },
        transition_calls: scalar("Hash_E1B89BB09D725EEC")?,
    })
}
