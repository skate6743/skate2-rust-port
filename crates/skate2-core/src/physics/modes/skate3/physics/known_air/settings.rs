//! Original KnownAir82D35508/82D35F40/82D360B0/82D36650 stock reads.
use skate3_core::{
    air::known::{KnownAirModeSettings, KnownAirSettings},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;
pub(super) fn load(
    data: &Collections,
) -> Result<(KnownAirSettings, [KnownAirModeSettings; 5]), String> {
    let scalar = |name| data.float("physics_airstates", "default", name);
    let graph = |name| -> Result<PointGraph<8>, String> {
        let values = data
            .words::<20>("physics_airstates", "default", name)?
            .map(f32::from_bits);
        Ok(PointGraph {
            x: values[4..12].try_into().unwrap(),
            y: values[12..20].try_into().unwrap(),
        })
    };
    let settings = KnownAirSettings {
        max_heading_adjust_vs_up_y_160: graph("MaxHeadingAdjustVsUpY")?,
        landing_speed_scalar_vs_ground_normal_y_240: graph("LandingSpeedScalarVsGroundNormalY")?,
        flip_start_collision_time_vs_normal_y: graph("Hash_AA79C93673533C5B")?,
        trajectory_error_blend_away_time_384: scalar("TrajErrorBlendAwayTime")?,
        min_target_heading_velocity_420: scalar("MinTargetHeadingVel")?,
        min_auto_body_speed_424: scalar("MinAutoBodySpeed")?,
        max_spin_speed_428: scalar("MaxSpinSpeed")?,
        frames_for_grind_air_assist_436: scalar("FramesForGrindAirAssist")?,
        body_flip_min_grab_time_fraction_456: scalar("BodyFlipMinGrabTimeFraction")?,
        flip_scalar: data.float("physics_reckoning", "default", "FlipScalar")?,
    };
    let mut modes = Vec::with_capacity(5);
    for key in ["easy", "normal", "hardcore", "motorized", "test"] {
        modes.push(KnownAirModeSettings {
            upside_down_falling_wipeout_enabled_2: data.boolean(
                "physics_mode",
                key,
                "Hash_2B913C786BEFF4CD",
            )?,
            perfect_body_flips_28: data.boolean("physics_mode", key, "PerfectBodyFlips")?,
            body_spin_speed_limit_56: data.float("physics_mode", key, "MaxAutoBodySpinSpeed")?,
        });
    }
    Ok((
        settings,
        modes
            .try_into()
            .map_err(|_| "Expected five physics modes")?,
    ))
}
