//! anim_motion/crouching (global300) and auto_pump (global348), TU3 schema.
use skate3_core::{animation::crouching::{Settings,AutoPumpSettings},point_graph::PointGraph};
use skate_data::collections::Collections;
pub fn load(data:&Collections)->Result<Settings,String> {
    let c=|name|data.float("anim_motion","crouching",name);
    let a=|name|data.float("anim_motion","auto_pump",name);
    Ok(Settings {
        maximum_height:negative8(data,"crouching","crouching_max_height")?,
        absorption_upforce:plain8(data,"crouching","absorption_upforce")?,
        pump_maxspeed:plain8(data,"crouching","pump_maxspeed")?,
        minimum_height:c("crouching_min_height")?,maximum_ratio:c("crouch_max_ratio")?,maximum_delta_delta:c("crouch_max_delta_delta")?,maximum_delta:c("crouch_max_delta")?,input_blend:c("crouch_blend_input")?,pump_vertical_speed:c("pump_maxspeed_yaxis")?,maximum_crouch_from_deck:c("MaxCrouchFromDeckAngle")?,skateboard_damping:c("absorption_skateboard_damping")?,maximum_force:c("absorption_maxforce")?,maximum_ground_force:c("absorption_ground_maxforce")?,absorption_factor:c("absorption_factor")?,
        auto_pump:AutoPumpSettings {maximum_crouch:negative4(data,"auto_pump","auto_pump_max_crouch")?,sufficient_crouch:a("auto_pump_sufficient_crouch")?,standing_threshold:a("auto_pump_standing_thresh")?,rise_speed:a("auto_pump_rise_speed")?,pump_speed:a("auto_pump_pump_speed")?,potential_threshold:a("auto_pump_potential_thresh")?,potential_blend:a("auto_pump_potential_blend")?,intent_magnitude_start:a("auto_pump_intent_mag_start")?,intent_angle_region:a("auto_pump_intent_angle_region")?,crouch_time:a("auto_pump_crouch_time")?,crouch_speed:a("auto_pump_crouch_speed")?},
    })
}
fn negative8(data:&Collections,key:&str,name:&str)->Result<PointGraph<8>,String> {let w=data.words::<20>("anim_motion",key,name)?;Ok(PointGraph{x:std::array::from_fn(|i|f32::from_bits(w[4+i])),y:std::array::from_fn(|i|f32::from_bits(w[12+i]))})}
fn negative4(data:&Collections,key:&str,name:&str)->Result<PointGraph<4>,String> {let w=data.words::<12>("anim_motion",key,name)?;Ok(PointGraph{x:std::array::from_fn(|i|f32::from_bits(w[4+i])),y:std::array::from_fn(|i|f32::from_bits(w[8+i]))})}
fn plain8(data:&Collections,key:&str,name:&str)->Result<PointGraph<8>,String> {let w=data.words::<16>("anim_motion",key,name)?;Ok(PointGraph{x:std::array::from_fn(|i|f32::from_bits(w[i])),y:std::array::from_fn(|i|f32::from_bits(w[8+i]))})}
