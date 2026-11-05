//! Native82BE7860's per-part records and82BEB0C8/82BEB790 settings.
use skate3_core::player::wipeout_state::drives::Settings;
use skate_data::{collections::Collections, physics_skeleton::PhysicsSkeleton};
use std::path::Path;

pub(crate) fn load(
    data: &Collections,
    asset_root: &Path,
    primary_bank_sha: &str,
) -> Result<Settings, String> {
    let skeleton = PhysicsSkeleton::load(
        &asset_root.join("private/stock/physics-skeletons.json"),
        primary_bank_sha,
        "PHYS_TPOSE",
    )?;
    if skeleton.bones.len() != 24 {
        return Err("Wipeout requires the actual24 physical skeleton parts".into());
    }
    let mut bone = [[0.0; 5]; 24];
    for part in 1..24 {
        let name = format!("PART_{}", skeleton.bones[part].name);
        let words = data.words::<9>("physics_skeleton_drives", "default", &name)?;
        bone[part] = std::array::from_fn(|i| f32::from_bits(words[i]));
    }
    let roots = [
        "root_drive_start_scalar",
        "root_drive_scalar",
        "root_drive_controlled_scalar",
        "root_drive_end_scalar",
    ];
    let mut root = [0.0; 4];
    for (i, name) in roots.into_iter().enumerate() {
        root[i] = data.float("physics_skeleton_drives", "default", name)?;
    }
    Ok(Settings {
        bone,
        root,
        strength: [
            data.float("animation", "default", "DriveStrengthLocal")?,
            data.float("animation", "default", "DriveStrengthRootLocal")?,
        ],
        hook_spring: data.float("physics_animation", "default", "HookSoftDsp")?,
        hook_strength: data.float("physics_animation", "default", "HookSoftStr")?,
        hook_damping: data.float("physics_animation", "default", "HookSoftDmp")?,
    })
}
