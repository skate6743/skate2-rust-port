//! Constructor74DD8 class7DEA05DC70A1ED7D/defaultD7EDBD362D7D2152.
use skate3_core::{
    player::offboard::board_possession::lifecycle::Settings, point_graph::PointGraph,
};
use skate_data::collections::Collections;
///091F8 full hash8D3065CED858A7DF, literal822F860C=426FFFFF.
pub(crate) fn standard_angular_drag(data: &Collections) -> Result<f32, String> {
    Ok(data.float("physicsdeck", "default", "DeckAngularDrag")? * f32::from_bits(0x426fffff))
}
pub(crate) fn load(data: &Collections) -> Result<Settings, String> {
    let float = |name| data.float("physics_skatecontroller", "default", name);
    Ok(Settings {
        hide_distance: float("MaxDistance")?,
        hide_offset: float("DistanceToHideSkateboard")?,
        return_distance: float("DistanceForBoardReturn")?,
        mounted_return_distance: float("DistanceForMountedReturn")?,
        mounting_time: float("MinTimeToRetrieveBoardWhileMounting")?,
        retrieval_time: curve(data, "RetrievalTimeVsDist")?,
        retrieval_weight: curve(data, "RetrievalSpeedCurve")?,
        throw_pitch: float("ThrownSkateboardLaunchPitch")?,
        throw_velocity: curve(data, "ThrownSkateboardVelocity")?,
        throw_target_pitch: float("ThrownSkateboardPitchTarget")?,
        throw_pitch_scalar: float("ThrownSkateboardPitchScalar")?,
        throw_roll_scalar: float("ThrownSkateboardRollScalar")?,
        throw_yaw_scalar: float("ThrownSkateboardYawScalar")?,
    })
}
fn curve(data: &Collections, name: &str) -> Result<PointGraph<8>, String> {
    let f = data.field("physics_skatecontroller", "default", name)?;
    if f.type_name != "Sk8::PointNegGraphData8" {
        return Err(format!("{name}: expected PointNegGraphData8"));
    }
    let words = data
        .words::<20>("physics_skatecontroller", "default", name)?
        .map(f32::from_bits);
    if words.iter().any(|v| !v.is_finite()) {
        return Err(format!("{name}: nonfinite stock graph"));
    }
    let x = std::array::from_fn(|i| words[4 + i]);
    if x.windows(2).any(|p| p[0] > p[1]) {
        return Err(format!("{name}: unordered stock knots"));
    }
    Ok(PointGraph {
        x,
        y: std::array::from_fn(|i| words[12 + i]),
    })
}
