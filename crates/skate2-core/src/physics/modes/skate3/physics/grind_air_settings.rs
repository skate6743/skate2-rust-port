//! Stock GrindAirAdjust settings82D72B40; lengths from the actual deck/truck collections.
use skate3_core::physics::grind_air::Settings;
use skate_data::collections::Collections;
pub(super) fn load(data: &Collections) -> Result<Settings, String> {
    let air = |key| data.float("physics_grinds_air", "default", key);
    let deck = |key| data.float("physicsdeck", "default", key);
    let half = deck("DeckMidLength")? * 0.5;
    let tip = deck("DeckFrontEndSize")? * air("TipOffsetFraction")?;
    let angle = deck("DeckFrontEndAngle")? * core::f32::consts::PI / 180.0;
    let z = half + data.float("physicstrucks", "default", "TruckZPosFront")?;
    let y = data.float("physicstrucks", "default", "TruckYPos")?;
    let frames = data.integer("physics_grinds_air", "default", "NumFramesToTest")? as usize;
    if !(2..=12).contains(&frames) {
        return Err("GrindAir NumFramesToTest exceeds native12-frame storage".into());
    }
    Ok(Settings {
        frames,
        stomp: air("StompAccelerationAdjust")?,
        points: [
            [0.; 4],
            [
                0.,
                tip * skate3_core::trigonometry::sin_cos(angle).0,
                half + tip,
                0.,
            ],
            [0., y, z, 0.],
            [0., y, -z, 0.],
            [
                0.,
                tip * skate3_core::trigonometry::sin_cos(angle).0,
                -half - tip,
                0.,
            ],
        ],
        ranges: [
            90.,
            air("AngleRangeTipSlide")?,
            air("AngleRangeTipSlide")?,
            air("AngleRangeBoardSlide")?,
            air("AngleRange50_50")?,
            air("AngleRange5_0")?,
            air("AngleRange5_0")?,
        ],
        distances: [
            z,
            z * air("MaxDistTipSlide")?,
            z * air("MaxDistTipSlide")?,
            z * air("MaxDistBoardSlide")?,
            z * air("MaxDist50_50")?,
            z * air("MaxDist5_0")?,
            z * air("MaxDist5_0")?,
        ],
        yaw_assist: [
            air("AngleYAssistBoardSlide")?,
            air("AngleYAssistTipSlide")?,
            air("AngleYAssistTipSlide")?,
            air("AngleYAssistBoardSlide")?,
            air("AngleYAssist50_50")?,
            air("AngleYAssist5_0")?,
            air("AngleYAssist5_0")?,
        ],
        max_offset: air("MaxOffsetDist")?,
        max_delta: air("MaxOffsetDeltaPerFrame")?,
        max_angle: air("MaxAngleDeltaPerFrame")?,
    })
}
