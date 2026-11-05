//! Stock anim_motion/anim_pump; TU3 global308 layout offsets1152/1708..1784.
use skate3_core::{animation::pumping_channel::Settings,point_graph::PointGraph};
use skate_data::collections::Collections;
pub fn load(data:&Collections)->Result<Settings,String> {
    let f=|name|data.float("anim_motion","anim_pump",name);
    let w=data.words::<16>("anim_motion","anim_pump","pump_amplify")?;
    Ok(Settings {amplify:PointGraph {x:std::array::from_fn(|i|f32::from_bits(w[i])),y:std::array::from_fn(|i|f32::from_bits(w[8+i]))},input_blend:f("pump_prop_blend")?,maximum_physics_pump:f("max_phys_pump")?,new_pump_threshold:f("new_pump_thresh")?,blend_in:f("pump_blendin")?,blend_out:f("pump_blendout")?})
}
