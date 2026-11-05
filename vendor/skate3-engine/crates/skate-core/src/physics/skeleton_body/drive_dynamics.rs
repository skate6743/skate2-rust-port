//! BoneDrives::EnableDrive82BECCE0 and mode-five interpolation82BECFB8.
//! A bone shares its interpolation counter between the LOCAL and LOCAL_ROOT
//! channels. Dispatch order is therefore observable even within one tick.
use crate::physics::drive_parameters::{RetailDriveDynamics, RetailDriveParams, RetailDriveType};

const FREQUENCY: f32 = f32::from_bits(0x426F_FFFF);
const FREQUENCY_SQUARED: f32 = f32::from_bits(0x4560_FFFE);
const HARD_VELOCITY: f32 = f32::from_bits(0x4415_FFFF);
const HARD_STRENGTH: f32 = f32::from_bits(0x470C_9FFF);

#[derive(Clone, Copy, Debug)]
pub struct AnimationDriveSettings {
    pub linear_strength: f32,
    pub linear_displacement: f32,
    pub angular_strength: f32,
    pub angular_displacement: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct DriveInterpolation {
    pub spring: [f32; 2],
    pub strength: [f32; 2],
    pub damping: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct BoneDriveSettings {
    pub animation: [AnimationDriveSettings; 2],
    pub collision_soft_displacement: f32,
    pub collision_soft_strength: f32,
    pub ragdoll_soft_displacement: f32,
    pub ragdoll_soft_strength: f32,
    pub transition_linear: DriveInterpolation,
    pub transition_angular: DriveInterpolation,
    pub transition_calls: f32,
}

#[derive(Clone, Debug)]
pub struct BoneDriveDynamics {
    pub channels: [RetailDriveDynamics; 2],
    pub mode: u32,
    pub strengths: [f32; 2],
    pub transition_counter: f32,
    pub transition_active: bool,
}
impl Default for BoneDriveDynamics {
    fn default() -> Self {
        Self {
            channels: [RetailDriveDynamics {
                linear: RetailDriveParams::DISABLED,
                angular: RetailDriveParams::DISABLED,
            }; 2],
            mode: 0,
            strengths: [0.0; 2],
            transition_counter: 0.0,
            transition_active: false,
        }
    }
}
impl BoneDriveDynamics {
    pub fn enable(&mut self, channel: usize, strength: f32, settings: BoneDriveSettings) {
        assert!(channel < 2);
        if self.mode > 5 {
            return;
        }
        if self.mode != 5 {
            self.transition_active = false;
        }
        let animation = settings.animation[channel];
        let dynamics = match self.mode {
            0 => RetailDriveDynamics {
                linear: hard(
                    animation.linear_displacement * FREQUENCY,
                    animation.linear_strength * FREQUENCY_SQUARED,
                ),
                angular: hard(
                    animation.angular_displacement * FREQUENCY,
                    animation.angular_strength * FREQUENCY_SQUARED,
                ),
            },
            1 => RetailDriveDynamics {
                linear: hard(HARD_VELOCITY, HARD_STRENGTH),
                angular: hard(HARD_VELOCITY, HARD_STRENGTH),
            },
            2 => RetailDriveDynamics {
                linear: hard(0.0, 0.0),
                angular: hard(
                    animation.angular_displacement * FREQUENCY,
                    animation.angular_strength * FREQUENCY_SQUARED,
                ),
            },
            3 => RetailDriveDynamics {
                linear: soft(0.0, 0.0, 0.0),
                angular: soft(
                    settings.collision_soft_displacement * strength,
                    200.0,
                    (settings.collision_soft_strength * strength) * FREQUENCY_SQUARED,
                ),
            },
            4 => RetailDriveDynamics {
                linear: if strength <= 0.5 {
                    soft(0.0, 0.0, 0.0)
                } else {
                    soft(
                        (settings.ragdoll_soft_displacement * strength) * 0.5,
                        200.0,
                        (strength * settings.ragdoll_soft_strength) * FREQUENCY_SQUARED,
                    )
                },
                angular: soft(
                    settings.ragdoll_soft_displacement * strength,
                    200.0,
                    (strength * settings.ragdoll_soft_strength) * FREQUENCY_SQUARED,
                ),
            },
            5 => {
                if !self.transition_active {
                    self.transition_active = true;
                    self.transition_counter = 1.0;
                } else if self.transition_counter < settings.transition_calls {
                    self.transition_counter += 1.0;
                }
                let progress = self.transition_counter / settings.transition_calls;
                RetailDriveDynamics {
                    linear: interpolate(settings.transition_linear, strength, progress),
                    angular: interpolate(settings.transition_angular, strength, progress),
                }
            }
            _ => unreachable!(),
        };
        self.channels[channel] = dynamics;
    }
}

fn hard(velocity: f32, strength: f32) -> RetailDriveParams {
    RetailDriveParams {
        spring_or_max_velocity: velocity,
        damping: 0.0,
        max_strength: strength,
        drive_type: RetailDriveType::HardDrive,
    }
}
fn soft(spring: f32, damping: f32, strength: f32) -> RetailDriveParams {
    RetailDriveParams {
        spring_or_max_velocity: spring,
        damping,
        max_strength: strength,
        drive_type: RetailDriveType::SoftDrive,
    }
}
fn interpolate(settings: DriveInterpolation, strength: f32, progress: f32) -> RetailDriveParams {
    // Original82BED100..114 are scalar fmadds (primary59,extended29).
    // The retained older pseudocode expands them to multiply+add expressions.
    let spring0 = settings.spring[0] * strength;
    let strength0 = settings.strength[0] * strength;
    let spring = (settings.spring[1] * strength - spring0).mul_add(progress, spring0);
    let force = (settings.strength[1] * strength - strength0).mul_add(progress, strength0);
    let damping =
        (settings.damping[1] - settings.damping[0]).mul_add(progress, settings.damping[0]);
    let nonnegative = |value| if value >= 0.0 { value } else { 0.0 };
    RetailDriveParams {
        spring_or_max_velocity: nonnegative(spring) * FREQUENCY,
        max_strength: nonnegative(force) * FREQUENCY_SQUARED,
        damping: nonnegative(damping),
        drive_type: RetailDriveType::HardDrive,
    }
}
