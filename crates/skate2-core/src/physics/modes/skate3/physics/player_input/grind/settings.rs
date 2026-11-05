use skate3_core::point_graph::PointGraph;
use skate_data::collections::Collections;

pub(super) struct Settings {
    pub friction: PointGraph<4>,
    pub slope_threshold: PointGraph<4>,
    pub vertical_help: PointGraph<4>,
    pub gravity_vertical: PointGraph<4>,
    pub gravity_linear: PointGraph<4>,
    pub exit_lean: PointGraph<8>,
    pub truck_to_wheel: f32,
    pub deck_to_truck: f32,
    pub test_above: f32,
    pub test_below: f32,
    pub max_impact: f32,
}

impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let f = |name| data.float("physics_grinds", "default", name);
        let exit = data
            .words::<20>("physics_grinds", "default", "ExitLeanAngleVsTime")?
            .map(f32::from_bits);
        Ok(Self {
            friction: graph4(data, "FrictionVsTime")?,
            slope_threshold: graph4(data, "DVEntryThreshScalarVsSinSlope")?,
            vertical_help: graph4(data, "VertEngagementHelpVsVelY")?,
            gravity_vertical: graph4(data, "TimeOfGravityReliefVsVerticalSpeed")?,
            gravity_linear: graph4(data, "GravityReliefTimeVsLinearSpeed")?,
            exit_lean: PointGraph {
                x: exit[4..12].try_into().unwrap(),
                y: exit[12..20].try_into().unwrap(),
            },
            truck_to_wheel: f("TruckToWheel")?,
            deck_to_truck: f("DeckCenterToTruck")?,
            test_above: f("TestDepthEpsilon")?,
            test_below: f("TestDepth")?,
            //physics_wipeout layout256,82D87358; not trajectory grind speed.
            max_impact: data.float(
                "physics_wipeout",
                "default",
                "Wipeout_AirMaxSpeedIntoCollisionNearGrind",
            )?,
        })
    }
}

fn graph4(data: &Collections, name: &str) -> Result<PointGraph<4>, String> {
    let words = data
        .words::<12>("physics_grinds", "default", name)?
        .map(f32::from_bits);
    //PointNegGraphData4 has a16-byte display-domain header before X/Y.
    Ok(PointGraph {
        x: words[4..8].try_into().unwrap(),
        y: words[8..12].try_into().unwrap(),
    })
}
