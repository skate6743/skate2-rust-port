//! Recovered drive parameter layout and TU3 truck/wheel construction branches.
const fn retail_f32(bits: u32) -> f32 {
    f32::from_bits(bits)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum RetailDriveType {
    NoDrive = 0,
    SoftDrive = 1,
    HardDrive = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct RetailDriveParams {
    pub spring_or_max_velocity: f32,
    pub damping: f32,
    pub max_strength: f32,
    pub drive_type: RetailDriveType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct RetailDriveDynamics {
    pub linear: RetailDriveParams,
    pub angular: RetailDriveParams,
}

/// Separate collections read by TU3 CreateDrives and SetTruckDriveDynamics.
/// `physicstrucks_drives.UseSoftDrives` is not read by this TU3 path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailTruckDriveSettings {
    /// physicstrucks layout +0; gates only the linear configuration.
    pub use_linear_drives: bool,
    /// physicsdeck full attribute 103FDC19514D4246; read only when enabled.
    pub use_hard_linear_drives: bool,
    /// physicstrucks_drives layout +24, +28 and +20 respectively.
    pub angular_displacement: f32,
    pub angular_damping: f32,
    pub angular_strength: f32,
}

impl RetailTruckDriveSettings {
    /// Decoded default collections, not truck geometry/twist-limit fields.
    pub const STOCK: Self = Self {
        use_linear_drives: false,
        use_hard_linear_drives: true,
        angular_displacement: 10.0,
        angular_damping: 0.0,
        angular_strength: 11.0,
    };
}

impl RetailDriveParams {
    pub const DISABLED: Self = Self {
        spring_or_max_velocity: 0.0,
        damping: 0.0,
        max_strength: 0.0,
        drive_type: RetailDriveType::NoDrive,
    };
}

/// Constructor 82C06298 zeros both halves at +7152/+7168, then CreateDrives
/// 82C0B770 creates the truck pair and applies these settings. Postphysics only
/// refreshes frames in TU3; it does not rerun this parameter setup every tick.
pub fn retail_truck_drive_dynamics(settings: RetailTruckDriveSettings) -> RetailDriveDynamics {
    let linear = if !settings.use_linear_drives {
        RetailDriveParams::DISABLED
    } else if settings.use_hard_linear_drives {
        RetailDriveParams {
            spring_or_max_velocity: retail_f32(0x45BB_7FFF),
            damping: 0.0,
            max_strength: retail_f32(0x48AF_C7FF),
            drive_type: RetailDriveType::HardDrive,
        }
    } else {
        RetailDriveParams {
            spring_or_max_velocity: retail_f32(0x47C3_5000),
            // 82C0B858..B8EC: native sqrt(100000) refinement, then *2.
            damping: soft_linear_damping(),
            max_strength: retail_f32(0x48AF_C7FF),
            drive_type: RetailDriveType::SoftDrive,
        }
    };
    RetailDriveDynamics {
        linear,
        angular: RetailDriveParams {
            spring_or_max_velocity: settings.angular_displacement * retail_f32(0x426F_FFFF),
            damping: settings.angular_damping,
            max_strength: settings.angular_strength * retail_f32(0x4560_FFFE),
            drive_type: RetailDriveType::HardDrive,
        },
    }
}

pub const fn retail_wheel_drive_dynamics(use_hard_drives: bool) -> RetailDriveDynamics {
    if use_hard_drives {
        RetailDriveDynamics {
            linear: RetailDriveParams {
                spring_or_max_velocity: retail_f32(0x45BB_7FFF),
                damping: 0.0,
                max_strength: retail_f32(0x48AF_C7FF),
                drive_type: RetailDriveType::HardDrive,
            },
            angular: RetailDriveParams {
                spring_or_max_velocity: 0.0,
                damping: 0.0,
                max_strength: 0.0,
                drive_type: RetailDriveType::NoDrive,
            },
        }
    } else {
        RetailDriveDynamics {
            linear: RetailDriveParams {
                spring_or_max_velocity: retail_f32(0x4561_0000),
                damping: retail_f32(0x4270_0000),
                max_strength: retail_f32(0x470C_9FFF),
                drive_type: RetailDriveType::SoftDrive,
            },
            angular: RetailDriveParams {
                spring_or_max_velocity: 0.0,
                damping: 0.0,
                max_strength: 0.0,
                drive_type: RetailDriveType::NoDrive,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct RetailDriveFrameRaw {
    /// Retail quaternion in observed `(x, y, z, w)` lane order.
    pub quaternion_lanes: [u32; 4],
    /// Affine translation vector copied from matrix offset `0x30`.
    pub translation_lanes: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct RetailDriveFramesRaw {
    /// Frame emitted by helper `0x82BD3A10`.
    pub body_a: RetailDriveFrameRaw,
    /// Frame emitted by helper `0x82BD3BD0`.
    pub body_b: RetailDriveFrameRaw,
}

/// Original TU3 82C0B8A8..B8EC: two reciprocal-square-root refinements,
/// multiply by the spring, select zero for zero spring, then multiply by two.
/// The estimate uses our independent Rust arithmetic; hardware parity is unverified.
fn soft_linear_damping() -> f32 {
    let spring = retail_f32(0x47C3_5000); // 822F91F0
    let mut reciprocal_root = super::native_arithmetic::reciprocal_square_root_estimate(spring);
    for _ in 0..2 {
        let squared = reciprocal_root * reciprocal_root;
        let half = reciprocal_root * 0.5;
        let residual = (-spring).mul_add(squared, 1.0);
        reciprocal_root = half.mul_add(residual, reciprocal_root);
    }
    let root = if spring == 0.0 {
        0.0
    } else {
        spring * reciprocal_root
    };
    root * retail_f32(0x4000_0000) // 82060C50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_soft_linear_trucks_have_critical_damping() {
        let settings = RetailTruckDriveSettings {
            use_linear_drives: true,
            use_hard_linear_drives: false,
            ..RetailTruckDriveSettings::STOCK
        };
        let dynamics = retail_truck_drive_dynamics(settings);
        assert_eq!(dynamics.linear.drive_type, RetailDriveType::SoftDrive);
        assert_eq!(dynamics.linear.spring_or_max_velocity, 100_000.0);
        assert!((dynamics.linear.damping - 632.455_5).abs() < 0.000_1);
        assert_eq!(dynamics.linear.max_strength.to_bits(), 0x48AF_C7FF);
        assert_eq!(
            dynamics.angular,
            retail_truck_drive_dynamics(RetailTruckDriveSettings::STOCK).angular
        );
    }
}
