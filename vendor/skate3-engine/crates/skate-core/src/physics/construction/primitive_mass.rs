//! Forward principal moments and volume from TU3 82AE7228/82AE6B78.
//! These quantities precede mass scaling and inverse-inertia construction.

use crate::math::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MassShape {
    Sphere {
        radius: f32,
    },
    /// The capsule's longitudinal principal axis is local Z.
    Capsule {
        radius: f32,
        half_length: f32,
    },
    RoundedBox {
        half_extents: Vector3,
        radius: f32,
    },
    Cylinder {
        radius: f32,
        half_length: f32,
        padding: f32,
    },
    /// Triangle, aggregate, or another type rejected by 82AE7228. The caller
    /// must use the aggregate calculation; no moments are produced here.
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimitiveMass {
    pub moments_per_unit_mass: Vector3,
    pub volume: f32,
}

const MIN_RADIUS: f32 = f32::from_bits(0x33d6_bf95);
const SPHERE_VOLUME: f32 = f32::from_bits(0x4086_0a92);
const FOUR_THIRDS: f32 = f32::from_bits(0x3faa_aaab);
const TWO_THIRDS: f32 = f32::from_bits(0x3f2a_aaab);
const ONE_THIRD: f32 = f32::from_bits(0x3eaa_aaab);
const ONE_TWELFTH: f32 = f32::from_bits(0x3daa_aaab);
const PI: f32 = f32::from_bits(0x4049_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

// The native fsubs/fsel chooses the right operand for an unordered difference.
// Keep this ordered comparison, including its signed-zero operand selection.
fn greater_extent(left: f32, right: f32) -> f32 {
    if left - right >= 0.0 { left } else { right }
}

/// Complete shape dispatch in 82AE7228. Unsupported shapes return no result,
/// matching the native false return and absence of output writes. All supported
/// branches retain the native scalar operation order and fused multiply-adds.
pub fn primitive_mass(shape: MassShape) -> Option<PrimitiveMass> {
    Some(match shape {
        MassShape::Unsupported => return None,
        MassShape::Sphere { radius } => {
            let radius = greater_extent(radius, MIN_RADIUS);
            let radius_squared = radius * radius;
            let moment = radius_squared * 0.4;
            PrimitiveMass {
                moments_per_unit_mass: Vector3::new(moment, moment, moment),
                volume: (radius_squared * radius) * SPHERE_VOLUME,
            }
        }
        MassShape::Capsule {
            radius,
            half_length,
        } => {
            let length_squared = half_length * half_length;
            let radius = greater_extent(greater_extent(radius, MIN_RADIUS), half_length * 0.05);
            let numerator = radius
                .mul_add(1.6, half_length * 0.75)
                .mul_add(radius, length_squared * 4.0)
                .mul_add(radius, length_squared * half_length);
            let denominator = radius.mul_add(4.0, half_length * 3.0);
            let transverse = numerator / denominator;
            let axial = (radius * radius) * 0.4;
            PrimitiveMass {
                moments_per_unit_mass: Vector3::new(transverse, transverse, axial),
                volume: ((radius.mul_add(FOUR_THIRDS, half_length * 2.0) * radius) * radius) * PI,
            }
        }
        MassShape::RoundedBox {
            half_extents,
            radius,
        } => rounded_box_mass(half_extents, radius),
        MassShape::Cylinder {
            radius,
            half_length,
            padding,
        } => {
            let radius = radius + padding;
            let half_length = half_length + padding;
            let radius_squared = radius * radius;
            let transverse =
                (half_length * half_length).mul_add(4.0, radius_squared * 3.0) * ONE_TWELFTH;
            PrimitiveMass {
                moments_per_unit_mass: Vector3::new(transverse, transverse, radius_squared * 0.5),
                volume: ((half_length * radius) * radius) * TWO_PI,
            }
        }
    })
}

/// Complete 82AE6B78: native rounded-box dimension floor, moments and volume.
fn rounded_box_mass(half_extents: Vector3, radius: f32) -> PrimitiveMass {
    let largest = greater_extent(
        half_extents.x,
        greater_extent(half_extents.y, half_extents.z),
    );
    let floor = greater_extent((largest + radius) * 0.05, MIN_RADIUS) - radius;
    let x = greater_extent(half_extents.x, floor);
    let y = greater_extent(half_extents.y, floor);
    let z = greater_extent(half_extents.z, floor);
    let radius_moment = (radius * radius) * 0.5;
    let x_squared = x * x;
    let y_squared = y * y;
    let z_squared = z * z;
    let xy_radius = (x + y) * radius;
    let yz_radius = (y + z) * radius;
    let xz_radius = (x + z) * radius;
    let moment_x =
        (yz_radius.mul_add(2.0, z_squared) + y_squared).mul_add(ONE_THIRD, radius_moment);
    let moment_y =
        (xz_radius.mul_add(2.0, x_squared) + z_squared).mul_add(ONE_THIRD, radius_moment);
    let moment_z =
        (xy_radius.mul_add(2.0, x_squared) + y_squared).mul_add(ONE_THIRD, radius_moment);
    let rounded_edges = (((radius.mul_add(TWO_THIRDS, x) + y) + z) * radius) * TWO_PI;
    let face_products = (y + z).mul_add(x, y * z);
    let box_volume = ((x * y) * z) * 8.0;
    PrimitiveMass {
        moments_per_unit_mass: Vector3::new(moment_x, moment_y, moment_z),
        volume: face_products
            .mul_add(8.0, rounded_edges)
            .mul_add(radius, box_volume),
    }
}
