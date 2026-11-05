//! SetTurning82BB3830/TurnRemap82BB4248 stock layout binding.
//! anim_carving/default's schema is752 bytes; native global slot372 selects it.
use skate3_core::{input::{set_turning::Settings, turn_remap::TurnRemap}, point_graph::PointGraph};
use skate_data::collections::Collections;

pub fn load(data: &Collections) -> Result<Settings, String> {
    let float = |name| data.float("anim_carving", "default", name);
    Ok(Settings {
        // Mode0 selects layout288/432/744; mode1 selects0/144/740.
        remaps: [
            TurnRemap { magnitude: curve16(data, "quickMagMap")?, angle: curve16(data, "quickAngleMap")?, angle_offset: float("quickAngleOffset")? },
            TurnRemap { magnitude: curve16(data, "slowMagMap")?, angle: curve16(data, "slowAngleMap")?, angle_offset: float("slowAngleOffset")? },
        ],
        speed_tuck: curve8(data, "speed_tuck")?,
        blend: curve8(data, "blend_lean_in")?,
        speed_threshold: float("speed_tuck_start")?,
        maximum_delta: float("clamp_curr_lean")?,
        // Global332 is anim_motion/power_slide, layout1680.
        override_turn: data.float("anim_motion", "power_slide", "slide_turn_value")?,
    })
}
fn curve16(data: &Collections, name: &str) -> Result<PointGraph<16>, String> {
    let words = data.words::<36>("anim_carving", "default", name)?;
    Ok(PointGraph {
        x: std::array::from_fn(|i| f32::from_bits(words[4+i])),
        y: std::array::from_fn(|i| f32::from_bits(words[20+i])),
    })
}
fn curve8(data: &Collections, name: &str) -> Result<PointGraph<8>, String> {
    let words = data.words::<20>("anim_carving", "default", name)?;
    Ok(PointGraph {
        x: std::array::from_fn(|i| f32::from_bits(words[4+i])),
        y: std::array::from_fn(|i| f32::from_bits(words[12+i])),
    })
}
