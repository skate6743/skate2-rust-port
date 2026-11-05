//! Stock physics_skatecontroller/default data used by the board controller.
use skate3_core::player::offboard::board_possession::lifecycle::Settings;
use skate_data::collections::Collections;
pub(super) fn load(data: &Collections) -> Result<Settings, String> {
    let float = |name| data.float("physics_skatecontroller", "default", name);
    let curve = |name| super::curves::load::<8>(data, "physics_skatecontroller", name).map(|v| v.0);
    Ok(Settings {
        //75F00 reads893DA1B966231D76 for the hide trigger;75BB8 reads
        //6CFF6F2C43F0AD41 for the hidden position. Lookup8 verifies both names.
        hide_distance: float("MaxDistance")?,
        hide_offset: float("DistanceToHideSkateboard")?,
        return_distance: float("DistanceForBoardReturn")?,
        mounted_return_distance: float("DistanceForMountedReturn")?,
        mounting_time: float("MinTimeToRetrieveBoardWhileMounting")?,
        retrieval_time: curve("RetrievalTimeVsDist")?,
        retrieval_weight: curve("RetrievalSpeedCurve")?,
        throw_pitch: float("ThrownSkateboardLaunchPitch")?,
        throw_velocity: curve("ThrownSkateboardVelocity")?,
        throw_target_pitch: float("ThrownSkateboardPitchTarget")?,
        throw_pitch_scalar: float("ThrownSkateboardPitchScalar")?,
        throw_roll_scalar: float("ThrownSkateboardRollScalar")?,
        throw_yaw_scalar: float("ThrownSkateboardYawScalar")?,
    })
}
