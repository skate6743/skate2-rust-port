//! Owned pumping fields. Layout offsets are evidence, not a runtime memory map.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PumpingState {
    /// +0 and +16, cached after every successful Update, including the first.
    pub previous_position: [f32; 4],
    pub previous_normal: [f32; 4],
    /// +32/+36/+40.
    pub smoothed_height_change: f32,
    pub pumping_time: f32,
    pub previous_height: f32,
    /// +44/+48/+52.
    pub pumping: f32,
    pub pump_acceleration: f32,
    pub angular_speed: f32,
    /// +56/+60/+64/+68, also copied to physics output.
    pub absorption: f32,
    pub ground_normal_absorption: f32,
    pub minimum_crouch: f32,
    pub deck_angle_absorption: f32,
    /// +72 is reset but not read or written by Update/Calculate.
    pub reset_only_scalar: f32,
    /// +76 and +81 are the only bytes touched by these three functions.
    pub record_valid: bool,
    pub intentional_pumping: u8,
}

impl PumpingState {
    /// TU3 82D8F1A0. Does not initialize bytes +77..80 or +82 onward; those
    /// are outside this owned state and must be preserved by any binary adapter.
    /// Ground Enter 82D378DC and Exit 82D37B58 both call this reset.
    pub fn reset(&mut self) {
        *self = Self::reset_state();
    }

    /// The output of the recovered reset, not a claim about the constructor.
    pub const fn reset_state() -> Self {
        Self {
            previous_position: [0.0; 4],
            previous_normal: [0.0; 4],
            smoothed_height_change: 0.0,
            pumping_time: 0.0,
            previous_height: 0.0,
            pumping: 0.0,
            pump_acceleration: 0.0,
            angular_speed: 0.0,
            absorption: 0.0,
            ground_normal_absorption: 0.0,
            minimum_crouch: 0.0,
            deck_angle_absorption: 0.0,
            reset_only_scalar: 0.0,
            record_valid: false,
            intentional_pumping: 0,
        }
    }

    /// Inline FillPhysOut at 82DB6FA8..7000. Read after Update; force assembly
    /// consumes pump_acceleration earlier, within Ground Update 82D38800.
    pub fn physics_output(&self) -> PumpingOutput {
        PumpingOutput {
            compression: 0.0,
            absorption: self.absorption,
            ground_normal_absorption: self.ground_normal_absorption,
            minimum_crouch: self.minimum_crouch,
            deck_angle_absorption: self.deck_angle_absorption,
            pump_acceleration: self.pump_acceleration,
            intentional_pumping: self.intentional_pumping,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PumpingOutput {
    /// Skeleton output +512/+516/+520/+528/+532.
    pub compression: f32,
    pub absorption: f32,
    pub ground_normal_absorption: f32,
    pub minimum_crouch: f32,
    pub deck_angle_absorption: f32,
    /// Ground output +268/+324.
    pub pump_acceleration: f32,
    pub intentional_pumping: u8,
}
