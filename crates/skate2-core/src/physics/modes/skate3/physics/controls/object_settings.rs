//! The original inputlistener_default fields fetched by8259C650..C75C.
use skate_data::collections::Collections;
use skate3_core::{input::object_intentions::Curves, point_graph::PointGraph};
pub(super) fn load(data: &Collections) -> Result<Curves, String> {
    let rotation = data
        .words::<36>("inputlistener", "default", "Hash_9ADFC2E222938C1E")?
        .map(f32::from_bits);
    let x = data
        .words::<20>("inputlistener", "default", "Hash_1A1A7AC37A72DF87")?
        .map(f32::from_bits);
    let z = data
        .words::<20>("inputlistener", "default", "Hash_05BA8B52C23B3481")?
        .map(f32::from_bits);
    Ok(Curves {
        rotate: PointGraph {
            x: rotation[4..20].try_into().unwrap(),
            y: rotation[20..36].try_into().unwrap(),
        },
        x: PointGraph {
            x: x[4..12].try_into().unwrap(),
            y: x[12..20].try_into().unwrap(),
        },
        z: PointGraph {
            x: z[4..12].try_into().unwrap(),
            y: z[12..20].try_into().unwrap(),
        },
    })
}
