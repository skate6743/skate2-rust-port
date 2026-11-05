//! Wheel/truck mass-input producers in TU3 82C0AA78 and 82C0A6F8.
//! Allocation, material publication and final inverse inertia are separate stages.

use super::{MassShape, PrimitiveMass, primitive_mass};
use crate::math::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelMassSettings {
    /// physicswheels.WheelRadius; 82C0AB44..82C0AB8C.
    pub radius: f32,
    /// physicswheels.WheelMass; 82C0ACC8..82C0AD04.
    pub mass: f32,
    /// physics_world.SkateboardMassFactor; 82C0AD0C..82C0AD58.
    pub mass_factor: f32,
}

impl WheelMassSettings {
    /// Decoded stock collection inputs, not cached mass-property outputs.
    pub const STOCK: Self = Self {
        radius: f32::from_bits(0x3cfd_f3b6),
        mass: f32::from_bits(0x3da9_fbe7),
        mass_factor: f32::from_bits(0x40a0_0000),
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TruckMassSettings {
    pub wheel_radius: f32,
    pub wheel_x_distance: f32,
    /// physicstrucks layout +52 and +56.
    pub radius_scalar: f32,
    pub half_height_scalar: f32,
    /// physicstrucks layout +40 and physics_world.SkateboardMassFactor.
    pub mass: f32,
    pub mass_factor: f32,
}

impl TruckMassSettings {
    pub const STOCK: Self = Self {
        wheel_radius: f32::from_bits(0x3cfd_f3b6),
        wheel_x_distance: f32::from_bits(0x3dc2_8f5c),
        radius_scalar: f32::from_bits(0x3f09_999a),
        half_height_scalar: f32::from_bits(0x3fac_cccd),
        mass: f32::from_bits(0x3ec7_ae14),
        mass_factor: f32::from_bits(0x40a0_0000),
    };
}

/// Actual shape dimensions and requested mass passed to 82AE7770. Its inverse
/// principal moments still require the unresolved native reciprocal estimate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartMassInput {
    pub shape: MassShape,
    pub requested_mass: f32,
}

/// 82C0AA78 creates one shared sphere and applies the same mass inputs to all
/// four wheel definitions. WheelCapsuleLength is not read by this function.
pub fn wheel_mass_input(settings: WheelMassSettings) -> PartMassInput {
    PartMassInput {
        shape: MassShape::Sphere {
            radius: settings.radius,
        },
        requested_mass: settings.mass * settings.mass_factor,
    }
}

/// 82C0A6F8 -> 82AD9708: capsule radius and half-length each include the final
/// multiplication by one half. Both truck definitions share this shape.
pub fn truck_mass_input(settings: TruckMassSettings) -> PartMassInput {
    PartMassInput {
        shape: MassShape::Capsule {
            radius: (settings.wheel_radius * settings.radius_scalar) * 0.5,
            half_length: (settings.wheel_x_distance * settings.half_height_scalar) * 0.5,
        },
        requested_mass: settings.mass * settings.mass_factor,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForwardMassProperties {
    pub primitive: PrimitiveMass,
    pub mass: f32,
    pub principal_moments: Vector3,
}

/// The supported primitive stage of 82AE7770: shape calculation, density-one
/// mass fallback at 82AE79EC, then principal-moment scaling at 82AE7A24.
/// An unsupported shape needs the aggregate producer and returns no result.
/// This does not supply inverse inertia or claim the part is ready to simulate.
pub fn forward_mass_properties(input: PartMassInput) -> Option<ForwardMassProperties> {
    let primitive = primitive_mass(input.shape)?;
    let mass = if input.requested_mass < f32::MIN_POSITIVE {
        primitive.volume
    } else {
        input.requested_mass
    };
    let moments = primitive.moments_per_unit_mass;
    Some(ForwardMassProperties {
        primitive,
        mass,
        principal_moments: Vector3::new(moments.x * mass, moments.y * mass, moments.z * mass),
    })
}
