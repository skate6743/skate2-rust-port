//! TU3 BoardAdjust instance+8/+12, Begin82BA2BA8, Update82BA2BC8.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct State {
    wrap: u32,
    previous_angle: f32,
}

impl State {
    pub fn begin(&mut self) { *self = Self::default(); }

    pub fn update(&mut self, magnitude: Option<f32>, angle: Option<f32>, filter: u32,
                  negate_on_mirror: bool, mirrored: bool) -> Option<(f32, f32)> {
        let (Some(magnitude), Some(mut angle)) = (magnitude, angle) else {
            //82BA2E88: remove both outputs and reset both instance fields.
            self.begin();
            return None;
        };
        if filter <= 7 { angle = skate3_core::input::graph_intents::apply_filter(angle, filter); }
        if negate_on_mirror && mirrored { angle *= -1.0; }
        //82BA2D70: crossing the +/-pi seam retains the previous side until
        //the raw angle crosses back. Test the previous RAW angle, not output.
        let crossed = self.previous_angle * angle < 0.0
            && self.previous_angle.abs() > f32::from_bits(0x3fc90fdb);
        match self.wrap {
            0 if crossed && self.previous_angle < 0.0 => self.wrap = 1,
            0 if crossed && self.previous_angle > 0.0 => self.wrap = 2,
            1 if crossed && self.previous_angle > 0.0 => self.wrap = 0,
            2 if crossed && self.previous_angle < 0.0 => self.wrap = 0,
            _ => {}
        }
        self.previous_angle = angle;
        let output = match self.wrap {
            1 => f32::from_bits(0xc0490fdb), //822F8908
            2 => f32::from_bits(0x40490fdb), //82060C44
            _ => angle,
        };
        Some((magnitude, output))
    }
}
