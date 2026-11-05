//! Stock physics_wipeout fields read by82D8FEE8/82D90000 only.
use skate3_core::player::offboard::biped_air::recovered::post::{Settings, checks::Thresholds};
use skate_data::collections::Collections;
pub(super) fn load(data: &Collections) -> Result<Settings, String> {
    let f = |name| data.float("physics_wipeout", "default", name);
    Ok(Settings {
        skeleton_air: Thresholds {
            squash: f("Wipeout_AirMaxSquash")?,
            displacement: f("Wipeout_AirSkeletonMaxDisp")?,
            body_contact: f("Wipeout_AirSkeletonMaxContact")?,
            arm_contact: f("Wipeout_AirSkeletonMaxContactArms")?,
        },
        offboard_air: Thresholds {
            squash: f("Wipeout_OB_MaxSquash")?,
            displacement: f("Wipeout_OB_Air_SkelMaxDisp")?,
            body_contact: f("Wipeout_OB_Air_SkelMaxContact")?,
            arm_contact: f("Wipeout_OB_SkeletonMaxContactArms")?,
        },
        offboard_min_speed: f("Wipeout_OB_Air_MinSpeed")?,
    })
}
