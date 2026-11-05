//! Stock manual animation data. Physical controller state remains in Ground.
use skate3_core::animation::manual::Settings;
use skate_data::collections::Collections;

pub fn settings(data: &Collections) -> Result<Settings, String> {
    let words = data.words::<16>("anim_motion", "manual", "manual_balance")?;
    Ok(Settings {
        balance: skate3_core::point_graph::PointGraph {
            x: std::array::from_fn(|i| f32::from_bits(words[i])),
            y: std::array::from_fn(|i| f32::from_bits(words[8 + i])),
        },
        velocity_limit: data.float("anim_motion", "manual", "manual_clamp_vel")?,
        acceleration_limit: data.float("anim_motion", "manual", "manual_clamp_acc")?,
    })
}
