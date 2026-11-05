use skate3_core::animation::grind_control::FadeSettings;
use skate_data::collections::Collections;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub fade: FadeSettings,
    pub height: [f32; 3],
}
impl Settings {
    pub fn load(d: &Collections) -> Result<Self, String> {
        let f = |n| d.float("anim_motion", "grind_twist", n);
        Ok(Self {
            fade: FadeSettings {
                response: f("twist_smoothing")?,
                input_scale: f("twist_sensitivity")?,
                acceleration: f("twist_max_delta_delta")?,
                maximum_step: f("twist_max_delta")?,
            },
            height: [
                d.float("anim_motion", "grind_height", "min_grind_disttocog")?,
                d.float("anim_motion", "grind_height", "max_grind_disttocog")?,
                d.float("anim_motion", "grind_height", "grind_disttocog_speed")?,
            ],
        })
    }
}
