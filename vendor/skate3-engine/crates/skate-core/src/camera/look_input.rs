//! Stock gameplay FreeCamManager82E084A0, used by a shot's look-stick option.
//! This is the normal camera's offset filter, unrelated to the replay editor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookSettings {
    pub heading_offset_degrees: f32,
    pub heading_speed: f32,
    pub elevation_offset_degrees: f32,
    pub elevation_speed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookInput {
    pub elevation: f32,
    pub heading: f32,
}

impl LookInput {
    pub fn new() -> Self {
        Self {
            elevation: 0.0,
            heading: 0.0,
        }
    }

    pub fn update(&mut self, input: [f32; 2], settings: LookSettings) {
        let radians = f32::from_bits(0x3c8efa35);
        let limit = f32::from_bits(0x3fc90fdb) - 5.0 * radians;
        let clamp = |value: f32| {
            let lower = if -limit - value >= 0.0 { -limit } else { value };
            if limit - lower >= 0.0 { lower } else { limit }
        };
        let heading = clamp((input[0] * settings.heading_offset_degrees) * radians);
        self.heading = (heading - self.heading).mul_add(
            settings.heading_speed * f32::from_bits(0x3c888889),
            self.heading,
        );
        let elevation = clamp((input[1] * settings.elevation_offset_degrees) * radians);
        self.elevation = (elevation - self.elevation).mul_add(
            settings.elevation_speed * f32::from_bits(0x3c888889),
            self.elevation,
        );
    }
}
