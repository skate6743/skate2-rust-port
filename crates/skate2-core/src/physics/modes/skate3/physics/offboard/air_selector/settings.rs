//! Selector-owned stock settings. Missing authored fields are errors.
#[path = "../settings/curves.rs"]
mod curves;
use skate3_core::{player::offboard::air_selector, point_graph::PointGraph};
use skate_data::collections::Collections;
pub(crate) struct Settings {
    pub query: air_selector::Settings,
    pub blend: PointGraph<8>,
    ///82C209E4: physics_grinds+636, not measured board geometry.
    pub deck_center_to_truck: f32,
}
impl Settings {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        let class = "physics_state_offboard_air";
        let query = air_selector::Settings {
            height: data.float(class, "default", "Hash_AB0D9EAEBFC584E9")?,
            sphere_radius: data.float(class, "default", "TrajectorySphereRadius")?,
            start_index: data.integer(class, "default", "TrajectoryStartIndex")? as i32,
        };
        if !query.height.is_finite()
            || !query.sphere_radius.is_finite()
            || query.sphere_radius <= 0.
        {
            return Err("Invalid stock BipedAir height/radius".into());
        }
        let blend = curves::load::<8>(data, class, "TrajBlendAmountVsTime")?.0;
        let deck_center_to_truck = data.float("physics_grinds", "default", "DeckCenterToTruck")?;
        if !deck_center_to_truck.is_finite() || deck_center_to_truck <= 0. {
            return Err("Invalid stock DeckCenterToTruck".into());
        }
        Ok(Self {
            query,
            blend,
            deck_center_to_truck,
        })
    }
}
