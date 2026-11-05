//! SkeletonBody::Init82BE4900, head82BE5428 and extra capsules82BE5180.
use super::{ANIMATION_PART_COUNT, PART_COUNT};
use crate::{
    math::{Basis3, Vector3},
    physics::{
        mass::{
            MassMoments, MassShape, PartMassInput, RETAIL_UNBOUNDED_VELOCITY,
            aggregate_mass_properties, primitive_mass, primitive_mass_properties,
        },
        native_arithmetic,
        rigid_body::{RetailBodyMassProperties, RetailLocalMassFrame},
        skeleton_animation_record::SkeletonAnimationMasses,
    },
};

#[derive(Clone, Copy, Debug)]
pub struct BoneSettings {
    pub mass_factor: f32,
    pub ragdoll_mass_factor: f32,
    pub has_collision: bool,
    pub use_root_drive: bool,
    pub volume_type: u32,
    pub volume_scalar: f32,
    pub num_parents: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct SkeletonBodySettings {
    /// Native MassOfSkeleton multiplies each original bone-box volume. It is
    /// not a requested total body mass and must not normalize the part masses.
    pub density: f32,
    pub root_radius: f32,
    pub root_half_length: f32,
    pub capsule_radius_scalar: f32,
    pub capsule_length_scalar: f32,
    pub ragdoll_inverse_mass_factor: f32,
    pub inertia_multiply_type: u32,
    pub inertia_factor: f32,
}

/// Authored cylinder and transform from physics_hat, before head aggregation.
#[derive(Clone, Copy, Debug)]
pub struct HatGeometry {
    pub radius: f32,
    pub half_length: f32,
    pub basis: Basis3,
    pub translation: Vector3,
}
impl HatGeometry {
    ///82BE55A0..569C: authored Euler angles feed XMVectorSinCos directly.
    pub fn from_offsets(
        radius: f32,
        thickness: f32,
        angles: Vector3,
        translation: Vector3,
    ) -> Self {
        let (sx, cx) = crate::trigonometry::sin_cos(angles.x);
        let (sy, cy) = crate::trigonometry::sin_cos(angles.y);
        let (sz, cz) = crate::trigonometry::sin_cos(angles.z);
        let cxsz = cx * sz;
        let sxsz = sx * sz;
        let sxcz = sx * cz;
        let cxcz = cx * cz;
        Self {
            radius,
            half_length: thickness * 0.5,
            translation,
            basis: Basis3 {
                columns: [
                    [cy * cz, cy * sz, -sy],
                    [sy * sxcz - cxsz, sy.mul_add(sxsz, cxcz), cy * sx],
                    [sy.mul_add(cxcz, sxsz), sy * cxsz - sxcz, cy * cx],
                ],
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SkeletonPart {
    /// Part1 can additionally contain the separately described hat cylinder.
    pub shape: MassShape,
    pub hat: Option<HatGeometry>,
    pub animated: RetailBodyMassProperties,
    pub ragdoll: RetailBodyMassProperties,
    pub inverse_mass_animated: f32,
    pub inverse_mass_ragdoll: f32,
}

pub struct SkeletonBodyDefinition {
    pub parts: [SkeletonPart; PART_COUNT],
    pub bones: [BoneSettings; ANIMATION_PART_COUNT],
    pub animation_masses: SkeletonAnimationMasses,
}
impl SkeletonBodyDefinition {
    pub fn new(
        sizes: [Vector3; ANIMATION_PART_COUNT],
        bones: [BoneSettings; ANIMATION_PART_COUNT],
        settings: SkeletonBodySettings,
        hat: Option<HatGeometry>,
    ) -> Result<Self, &'static str> {
        let mut parts = Vec::with_capacity(PART_COUNT);
        for i in 0..ANIMATION_PART_COUNT {
            let b = bones[i];
            let size = sizes[i];
            let shape = if i == 0 {
                MassShape::Capsule {
                    radius: settings.root_radius,
                    half_length: settings.root_half_length,
                }
            } else if i == 1 && hat.is_some() || b.volume_type == 1 {
                let x = size.x * 0.5;
                let y = size.y * 0.5;
                MassShape::Sphere {
                    radius: (if x - y >= 0.0 { x } else { y }) * b.volume_scalar,
                }
            } else {
                match b.volume_type {
                    0 => {
                        let scalar = b.volume_scalar * 0.5;
                        MassShape::RoundedBox {
                            half_extents: Vector3::new(
                                size.x * scalar,
                                size.y * scalar,
                                size.z * scalar,
                            ),
                            radius: 0.0,
                        }
                    }
                    2 => {
                        let radius = (size.x * settings.capsule_radius_scalar) * 0.5;
                        let length = size.z * 0.5 - radius;
                        MassShape::Capsule {
                            radius: b.volume_scalar * radius,
                            half_length: ((if -length >= 0.0 { 0.0 } else { length })
                                * settings.capsule_length_scalar)
                                * b.volume_scalar,
                        }
                    }
                    _ => return Err("Unresolved non-root skeleton volume type"),
                }
            };
            let base_mass = if i == 0 {
                f32::from_bits(0x3A83_126F)
            } else {
                settings.density * ((size.x * size.y) * size.z)
            };
            let head_hat = if i == 1 { hat } else { None };
            let animated_mass = b.mass_factor * base_mass;
            let ragdoll_mass = b.ragdoll_mass_factor * base_mass;
            let mut animated = mass_properties(shape, head_hat, animated_mass)?;
            let ragdoll = mass_properties(shape, head_hat, ragdoll_mass)?;
            adjust_inertia(
                &mut animated,
                settings.inertia_multiply_type,
                settings.inertia_factor,
            );
            parts.push(SkeletonPart {
                shape,
                hat: head_hat,
                animated,
                ragdoll,
                inverse_mass_animated: 1.0 / animated_mass,
                inverse_mass_ragdoll: settings.ragdoll_inverse_mass_factor / ragdoll_mass,
            });
        }
        //82BE4F80..4FC4 passes these independent radius/half-length/mass words.
        for (radius, half_length, mass) in [
            (0x3E6B_851F, 0x3DCC_CCCD, 0x3DCC_CCCD),
            (0x3EB8_51EC, 0x3E85_1EB8, 0x3C23_D70A),
        ] {
            let mass = f32::from_bits(mass);
            let shape = MassShape::Capsule {
                radius: f32::from_bits(radius),
                half_length: f32::from_bits(half_length),
            };
            let mut properties = mass_properties(shape, None, mass)?;
            properties.dynamics.maximum_linear_velocity = f32::from_bits(0x41EF_FFFF);
            properties.dynamics.maximum_angular_velocity = f32::from_bits(0x41EF_FFFF);
            adjust_inertia(&mut properties, 1, 5.0);
            parts.push(SkeletonPart {
                shape,
                hat: None,
                animated: properties,
                ragdoll: properties,
                inverse_mass_animated: 1.0 / mass,
                inverse_mass_ragdoll: 1.0 / mass,
            });
        }
        Ok(Self {
            parts: parts.try_into().map_err(|_| "Skeleton part count")?,
            animation_masses: SkeletonAnimationMasses::from_bone_data(
                sizes,
                bones.map(|b| b.volume_type),
                hat.is_some(),
            ),
            bones,
        })
    }
}

fn mass_properties(
    shape: MassShape,
    hat: Option<HatGeometry>,
    mass: f32,
) -> Result<RetailBodyMassProperties, &'static str> {
    if let Some(hat) = hat {
        // Init82BE5428 uses cylinder first, then the head sphere. Preserve
        // accumulation order and the later identity mass-frame override.
        let mut moments = MassMoments::from_primitive(
            primitive_mass(MassShape::Cylinder {
                radius: hat.radius,
                half_length: hat.half_length,
                padding: 0.0,
            })
            .ok_or("Invalid skeleton hat primitive")?,
        );
        moments.transform(hat.basis, hat.translation);
        moments.add(MassMoments::from_primitive(
            primitive_mass(shape).ok_or("Invalid head primitive")?,
        ));
        let mut properties =
            aggregate_mass_properties(moments, mass, RETAIL_UNBOUNDED_VELOCITY, 0.0);
        properties.local_mass_frame = RetailLocalMassFrame::IDENTITY;
        Ok(properties)
    } else {
        primitive_mass_properties(
            PartMassInput {
                shape,
                requested_mass: mass,
            },
            RETAIL_UNBOUNDED_VELOCITY,
            0.0,
        )
        .ok_or("Invalid skeleton primitive")
    }
}

/// DebugMultiplyInertiaTensor82E0B070 / SetSphericalInertiaTensor82E0B110.
fn adjust_inertia(properties: &mut RetailBodyMassProperties, kind: u32, factor: f32) {
    let d = &mut properties.dynamics;
    if kind == 1 {
        let mut reciprocal = native_arithmetic::reciprocal_estimate(factor);
        for _ in 0..2 {
            reciprocal = reciprocal.mul_add((-reciprocal).mul_add(factor, 1.0), reciprocal);
        }
        d.inverse_tensor = Vector3::new(
            d.inverse_tensor.x * reciprocal,
            d.inverse_tensor.y * reciprocal,
            d.inverse_tensor.z * reciprocal,
        );
        let xy = if d.inverse_tensor.x < d.inverse_tensor.y {
            d.inverse_tensor.x
        } else {
            d.inverse_tensor.y
        };
        let xyz = if xy < d.inverse_tensor.z {
            xy
        } else {
            d.inverse_tensor.z
        };
        d.spherical = 1.0 / xyz;
    } else if kind == 2 {
        let reciprocal = 1.0 / (factor * 3.0);
        let value = ((d.inverse_tensor.x + d.inverse_tensor.y) + d.inverse_tensor.z) * reciprocal;
        d.inverse_tensor = Vector3::new(value, value, value);
        d.spherical = 1.0 / value;
    }
}
