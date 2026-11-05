//! Stock PhysicsAnimation cache +560/+576 and +592/+608, TU3 82DE6480.
use skate3_core::{animation::landing_quality::Settings, point_graph::PointGraph};
use skate_data::collections::Collections;

pub(crate) fn load(data: &Collections) -> Result<Settings, String> {
    let graph = |name| -> Result<PointGraph<4>, String> {
        let words = data.words::<8>("physics_animation", "default", name)?;
        Ok(PointGraph {
            x: std::array::from_fn(|i| f32::from_bits(words[i])),
            y: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
        })
    };
    Ok(Settings {
        twist_spin: graph("LandingSketchyTwistSpin")?,
        side_speed: graph("LandingSketchySideSpeed")?,
    })
}
