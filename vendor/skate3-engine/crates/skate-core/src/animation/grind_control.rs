//! S3 TU3 GrindControlFade82BB0D40 and ControlGrindCrouch82BB10F8.
//! Adapted from the authorized ZIP after independent original-image comparison.
//! Animation selection, physical observations and tuning remain external inputs.
pub mod facing;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug)]
pub struct FadeSettings {
    /// anim_motion/grind_twist: offsets1640/1644/1648/1652.
    pub response: f32,
    pub input_scale: f32,
    pub acceleration: f32,
    pub maximum_step: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Fade {
    pub just_began: bool,
    pub elapsed: f32,
    pub value: f32,
    pub target: f32,
    pub step: f32,
    pub minimum: f32,
    pub maximum: f32,
}
impl Fade {
    /// Begin82BB0A30 supplies the actual queried endpoints and mirrored twist.
    pub fn begin(&mut self, minimum: f32, maximum: f32, twist: f32) -> f32 {
        let value = bound(twist, minimum, maximum);
        *self = Self {
            just_began: true,
            minimum,
            maximum,
            value,
            target: value,
            ..Self::default()
        };
        value
    }
    /// First Update only clears the latch. dt affects elapsed, NOT twist speed.
    pub fn update(&mut self, dt: f32, intent: f32, settings: FadeSettings) -> Option<f32> {
        if self.just_began {
            self.just_began = false;
            return None;
        }
        self.elapsed += dt;
        self.target = bound(
            settings.input_scale.mul_add(intent, self.target),
            self.minimum,
            self.maximum,
        );
        let desired =
            (1.0 - settings.response).mul_add(self.value, self.target * settings.response);
        let step = bound(
            desired - self.value,
            self.step - settings.acceleration,
            self.step + settings.acceleration,
        );
        self.step = bound(step, -settings.maximum_step, settings.maximum_step);
        self.value = bound(self.value + self.step, self.minimum, self.maximum);
        Some(self.value)
    }
}

/// S3 always includes physical crouch; S2's optional enable switch is absent.
/// anim_motion/grind_height: min1760/max1792/rate1816; previous starts at Anim+72.
pub fn crouch(
    previous: f32,
    intent: f32,
    grind_crouch: f32,
    minimum: f32,
    maximum: f32,
    rate: f32,
    dt: f32,
) -> f32 {
    let input = bound(1.0 - intent, minimum, maximum);
    let physical = bound(1.0 - grind_crouch, minimum, maximum);
    let desired = if input - physical >= 0.0 {
        physical
    } else {
        input
    };
    let step = dt * rate;
    bound(desired, previous - step, previous + step)
}

/// Begin82BB0CBC..CE4: mirror the Skeleton+504 angle exactly once.
pub fn mirrored_twist(twist: f32, mirrored: bool) -> f32 {
    if !mirrored {
        return twist;
    }
    if twist >= 0.0 {
        std::f32::consts::PI - twist
    } else {
        -std::f32::consts::PI - twist
    }
}

fn bound(value: f32, minimum: f32, maximum: f32) -> f32 {
    let value = if minimum - value >= 0.0 {
        minimum
    } else {
        value
    };
    if maximum - value >= 0.0 {
        value
    } else {
        maximum
    }
}
