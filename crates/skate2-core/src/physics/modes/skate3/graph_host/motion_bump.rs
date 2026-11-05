//! SetBumpCoefficients Begin82BB0660, original Skate 3 TU3.
use skate_data::collections::Collections;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub scale_x_acc: f32,
    pub min_bump_mag: f32,
    pub max_bump_mag: f32,
    pub min_bump_blend_value: f32,
}
impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let value = |name| data.float("anim_motion", "bumps", name);
        Ok(Self {
            scale_x_acc: value("scale_x_acc")?,
            min_bump_mag: value("min_bump_mag")?,
            max_bump_mag: value("max_bump_mag")?,
            min_bump_blend_value: value("min_bump_blend_value")?,
        })
    }

    /// Begin82BB0660 uses the retained Xenon arithmetic implementation.
    pub fn coefficients(&self, acceleration: [f32; 4], mirrored: bool) -> [f32; 2] {
        skate3_core::animation::bump::coefficients(acceleration, mirrored, &skate3_core::animation::bump::Settings {
            scale_x_acc: self.scale_x_acc,
            min_bump_mag: self.min_bump_mag,
            max_bump_mag: self.max_bump_mag,
            min_bump_blend_value: self.min_bump_blend_value,
        })
    }
}
