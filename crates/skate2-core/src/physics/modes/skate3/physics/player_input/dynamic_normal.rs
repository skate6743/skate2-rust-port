//! Stock bindings for original82C02388. Hashes were matched to the actual
//! collection names; XML layoutC85F0-C8190 verifies curve offset1120.
use skate3_core::{physics::board_dynamic_normal::DynamicNormalSettings, point_graph::PointGraph};
use skate_data::collections::Collections;
pub(crate) fn settings(data: &Collections) -> Result<DynamicNormalSettings, String> {
    let f = |field| data.float("physics_reckoning", "default", field);
    let values = data
        .words::<16>("physics_reckoning", "default", "DynamicSpeedMaxDeltaGraphZ")?
        .map(f32::from_bits);
    Ok(DynamicNormalSettings {
        speed_damping: f("DynamicSpeedDamping")?, //AB8F93BA080E0CB3
        up_vector_damping: f("DynamicUpVectorDamping")?, //9F15310DBBE96B8C
        maximum_delta: f("DynamicSpeedMaxDelta")?, //30BF4DA4BD7AE00A
        speed_scale: f("DynamicSpeedMaxDeltaXScale")?, //20F1ECA637708494
        maximum_delta_vs_speed: PointGraph {
            x: values[..8].try_into().unwrap(),
            y: values[8..].try_into().unwrap(),
        },
    })
}
