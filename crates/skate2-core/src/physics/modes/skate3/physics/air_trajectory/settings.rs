//! Stock class hashes429A8F4385DD539D(trajectory),7CE1B6B581709C6F
//! (reckoning),5338958C408B6009(feet), defaultD7EDBD362D7D2152.
//! Layout names/offsets were checked against original TU3 loads and stock XML.
use skate3_core::{air::trajectory::SelectorSettings, point_graph::PointGraph};
use skate_data::collections::Collections;

pub(super) fn load(data: &Collections) -> Result<SelectorSettings, String> {
    let t = |name| data.float("physics_trajectory", "default", name);
    let r = |name| data.float("physics_reckoning", "default", name);
    Ok(SelectorSettings {
        cone_x: t("ConeAngleX")?,
        cone_z: t("ConeAngleZ")?,
        trajectory_max_time: t("TrajectoryMaxTime")?,
        trajectory_max_drop: t("TrajectoryMaxDrop")?, //63673FB6DA092B7E
        trajectory_error_start: t("TrajectoryMaxErrorStart")?,
        trajectory_error_end: t("TrajectoryMaxErrorEnd")?,
        speed_factor_min: t("SpeedFactorMin")?,
        speed_factor_max: t("SpeedFactorMax")?,
        cone_angle_z_vs_speed: graph8(data, "physics_trajectory", "ConeAngleZVsSpeed", false)?,
        cone_x_second_pass: t("ConeAngleXSecondPass")?,
        cone_z_second_pass: t("ConeAngleZSecondPass")?,
        landing_time_bonus: graph8(data, "physics_trajectory", "LandingTimeBonus", true)?,
        landing_com_scalar_vs_slope: graph8(
            data,
            "physics_trajectory",
            "LandingComScalarVsSlope",
            true,
        )?,
        landing_force_scalar: graph8(
            data,
            "physics_trajectory",
            "LandingForceScalarVsDPVelNorm",
            false,
        )?,
        grind_penalty_vs_distance: graph8(
            data,
            "physics_trajectory",
            "GrindPenaltyVsDistToGrind",
            true,
        )?,
        score_middle_bonus: t("ScoreMiddleBonus")?,
        score_landing_force: t("ScoreLandingForce")?,
        score_landing_direction: t("ScoreLandingDirection")?,
        score_transition: t("ScoreTransition")?,
        surface_unrideable_score: t("ScoreSurface_Unrideable")?,
        surface_dont_align_score: t("ScoreSurface_DontAlign")?,
        natural_air_off_verts_scalar: t("NaturalAirOffVertsScalar")?,
        minimum_valid_time: t("MinTimeForTrajectoryToBeValid")?,
        minimum_time_after_apex: t("MinTimeAfterApexForTransition")?,
        minimum_normal_delta_second_pass: t("MinNormalDeltaForSecondPass")?,
        maximum_trajectory_adjust: t("MaxTrajectoryAdjust")?,
        wall_ride_test_distance: t("WallRideTestDist")?,
        wall_ride_minimum_height: t("WallRideMinHeightFromGround")?,
        wall_ride_angle_allow_landing: t("WallRideAngleToAllowLanding")?,
        wall_ride_height_score: t("ScoreWallrideHeight")?,
        wall_ride_boost: negative_graph4(data, "physics_trajectory", "WallRideBoost")?,
        //Globals8289D5C8:ED14 class lookup;ED48 stores188, layout248.
        wall_ride_normal_dot_limit: data.float(
            "physics_feet",
            "default",
            "WallRideMaxDotFloorWall",
        )?,
        displacement_vs_speed: graph8(data, "physics_reckoning", "TrajectoryDispVsSpeed", false)?,
        displacement_vs_ground_normal: graph8(
            data,
            "physics_reckoning",
            "TrajectoryDispVsGroundNorm",
            false,
        )?,
        trajectory_radius: r("TrajectoryRadius")?, //C6710942E297D587
        trajectory_displacement: r("TrajectoryDisplacement")?, //F85A91F6A233DDAE
        minimum_trajectory_frames: data.integer(
            "physics_reckoning",
            "default",
            "MinTrajectorySize",
        )? as i32,
        vert_jump_align_factor: r("VertJumpAlignFactor")?, //0B42DFBB7F7756A0
        vert_jump_align_max_ground_normal_y: r("VertJumpAlignMaxGroundNormalY")?, //94923D136EA2DA37
        vert_jump_align_min_direction_y: r("VertJumpAlignMinJumpDirY")?, //58BFC8D41216C7EE
        vert_jump_align_max_angle: r("VertJumpAlignMaxAngle")?, //723FF102068622CF
    })
}
fn graph8(
    data: &Collections,
    class: &str,
    name: &str,
    negative: bool,
) -> Result<PointGraph<8>, String> {
    let expected = if negative {
        "Sk8::PointNegGraphData8"
    } else {
        "Sk8::PointGraphData8"
    };
    if data.field(class, "default", name)?.type_name != expected {
        return Err(format!("Wrong stock graph type {class}/{name}"));
    }
    let (x, y) = if negative {
        let words = data.words::<20>(class, "default", name)?;
        (
            std::array::from_fn(|i| f32::from_bits(words[4 + i])),
            std::array::from_fn(|i| f32::from_bits(words[12 + i])),
        )
    } else {
        let words = data.words::<16>(class, "default", name)?;
        (
            std::array::from_fn(|i| f32::from_bits(words[i])),
            std::array::from_fn(|i| f32::from_bits(words[8 + i])),
        )
    };
    //82481E10 receives the x/y spans after any PointNeg header.
    Ok(PointGraph { x, y })
}
fn negative_graph4(data: &Collections, class: &str, name: &str) -> Result<PointGraph<4>, String> {
    if data.field(class, "default", name)?.type_name != "Sk8::PointNegGraphData4" {
        return Err(format!("Wrong stock graph type {class}/{name}"));
    }
    let words = data.words::<12>(class, "default", name)?;
    Ok(PointGraph {
        x: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
        y: std::array::from_fn(|i| f32::from_bits(words[8 + i])),
    })
}
