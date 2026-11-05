//! Original TU3 shared Wipeout request calculation and persistent history.
//! Physical bail/recovery state response is scheduled by the game coordinator.
mod air;
mod common;
mod data;
mod ground;
mod requests;
pub use air::check as check_air;
pub use air::check_collision as check_air_collision;
pub(crate) use common::force as regional_force;
pub use data::{
    AirSettings, Frame, GroundSettings, Mode, PlantContact, PlantInput, RequestInput, Settings, V,
};
pub use ground::{
    check as check_ground, check_animation as check_ground_animation, check_plant,
    check_plant_input,
};
pub use requests::Requests;

#[cfg(test)]
mod tests;
