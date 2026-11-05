//! Deck collision children constructed by TU3 82C09290 and 82C0A2A0/3E0/510.
//! Allocation is host-owned; dimensions, child order, flags and arithmetic are
//! source-derived. Mass uses the actual primitive/triangle-AABB dispatch in
//! 82AE7058, including children whose collision-enable bit is clear.

use super::{MassMoments, MassShape, primitive_mass};
use crate::{
    math::Vector3,
    physics::{drive_frames::RetailAffineTransform, native_arithmetic},
    trigonometry,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeckGeometrySettings {
    pub width: f32,
    pub mid_length: f32,
    pub thickness: f32,
    /// Both fans read DeckBackEndSize. DeckFrontEndSize is not read here.
    pub back_end_size: f32,
    pub front_end_angle_degrees: f32,
    pub back_end_angle_degrees: f32,
    /// DeckEndCapsules (Int32), despite the triangles produced by this loop.
    pub end_capsule_count: i32,
    pub enable_deck_volume_collisions: bool,
    pub enable_end_volume_collisions: bool,
}

impl DeckGeometrySettings {
    /// physicsdeck/default.xml, retaining the original Float words.
    pub const STOCK: Self = Self {
        width: f32::from_bits(0x3E75_C28F),
        mid_length: f32::from_bits(0x3F17_0A3D),
        thickness: f32::from_bits(0x3C75_C28F),
        back_end_size: f32::from_bits(0x3E28_F5C3),
        front_end_angle_degrees: f32::from_bits(0x4150_0000),
        back_end_angle_degrees: f32::from_bits(0x4148_0000),
        end_capsule_count: 5,
        enable_deck_volume_collisions: true,
        enable_end_volume_collisions: true,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeckShape {
    RoundedBox {
        half_extents: Vector3,
        radius: f32,
    },
    Capsule {
        radius: f32,
        half_length: f32,
    },
    Sphere {
        radius: f32,
    },
    Triangle {
        vertices: [Vector3; 3],
        fatness: f32,
        edge_cosines: [f32; 3],
        /// Volume flags excluding the separately exposed collision-enable bit.
        /// These are Volume flags, not GPTriangle feature flags.
        volume_flags: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeckChild {
    pub shape: DeckShape,
    pub transform: RetailAffineTransform,
    pub collision_enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckGeometry {
    pub children: Vec<DeckChild>,
}

impl DeckGeometry {
    pub fn new(settings: DeckGeometrySettings) -> Self {
        let half_width = settings.width * 0.5;
        let half_length = settings.mid_length * 0.5;
        let half_thickness = settings.thickness * 0.5;
        let radius = half_thickness * f32::from_bits(0x3F66_6666);
        let mut children = vec![DeckChild {
            shape: DeckShape::RoundedBox {
                half_extents: Vector3::new(
                    half_width - radius,
                    half_thickness - radius,
                    half_length - radius,
                ),
                radius,
            },
            transform: RetailAffineTransform::IDENTITY,
            collision_enabled: settings.enable_deck_volume_collisions,
        }];

        // 82E0B1E0: midpoint = first + (second-first)/2. These endpoints are
        // parallel to local Z, so the orientation branch leaves identity.
        let longitudinal_half_delta = (-half_length - half_length) * 0.5;
        let capsule_half_length = vector_length_z(longitudinal_half_delta);
        let capsule_center_z = half_length + longitudinal_half_delta;
        let arc_width = half_width - half_thickness;
        for x in [arc_width, half_thickness - half_width] {
            children.push(DeckChild {
                shape: DeckShape::Capsule {
                    radius: half_thickness,
                    half_length: capsule_half_length,
                },
                transform: translated(Vector3::new(x, 0.0, capsule_center_z)),
                collision_enabled: settings.enable_end_volume_collisions,
            });
        }
        // 82C0A3E0 retains the sphere constructor's enabled flag, independent
        // of the two authored deck collision toggles.
        for z in [f32::from_bits(0x3E70_A3D7), f32::from_bits(0xBE70_A3D7)] {
            children.push(DeckChild {
                shape: DeckShape::Sphere { radius: 0.035 },
                transform: translated(Vector3::new(0.0, f32::from_bits(0xBCA3_D70A), z)),
                collision_enabled: true,
            });
        }

        let radians = f32::from_bits(0x3C8E_FA35);
        let back_angle = settings.back_end_angle_degrees * radians;
        let front_angle = settings.front_end_angle_degrees * radians;
        let back_slope = (trigonometry::sin(back_angle), cosine(back_angle));
        let front_slope = (trigonometry::sin(front_angle), cosine(front_angle));
        let end_length = settings.back_end_size - half_thickness;
        let mut previous_back = (trigonometry::sin(0.0), cosine(0.0));
        let mut previous_front = previous_back;
        for segment in 1..=settings.end_capsule_count {
            let back_arc =
                (segment as f32 * f32::from_bits(0xC049_0FDB)) / settings.end_capsule_count as f32;
            let next_back = (trigonometry::sin(back_arc), cosine(back_arc));
            children.push(fan_triangle(
                previous_back,
                next_back,
                back_slope,
                arc_width,
                end_length,
                -half_length,
                half_thickness,
                false,
                settings.enable_deck_volume_collisions,
            ));
            previous_back = next_back;
            let front_arc =
                (segment as f32 * f32::from_bits(0x4049_0FDB)) / settings.end_capsule_count as f32;
            let next_front = (trigonometry::sin(front_arc), cosine(front_arc));
            children.push(fan_triangle(
                previous_front,
                next_front,
                front_slope,
                arc_width,
                end_length,
                half_length,
                half_thickness,
                true,
                settings.enable_deck_volume_collisions,
            ));
            previous_front = next_front;
        }
        Self { children }
    }

    pub fn mass_moments(&self) -> MassMoments {
        let mut total = MassMoments::ZERO;
        for child in &self.children {
            total.add(child.mass_moments());
        }
        // The aggregate volume's transform is identity (82AD7740).
        total.transform(RetailAffineTransform::IDENTITY.basis, Vector3::ZERO);
        total
    }
}

impl DeckChild {
    pub fn mass_moments(&self) -> MassMoments {
        let (shape, transform) = match self.shape {
            DeckShape::Sphere { radius } => (MassShape::Sphere { radius }, self.transform),
            DeckShape::Capsule {
                radius,
                half_length,
            } => (
                MassShape::Capsule {
                    radius,
                    half_length,
                },
                self.transform,
            ),
            DeckShape::RoundedBox {
                half_extents,
                radius,
            } => (
                MassShape::RoundedBox {
                    half_extents,
                    radius,
                },
                self.transform,
            ),
            DeckShape::Triangle {
                vertices, fatness, ..
            } => {
                // TU3 TriangleVolume::GetBBox 82ADDC40, null transform branch:
                // min(v0,min(v1,v2))-fatness, max(v0,max(v1,v2))+fatness.
                // 82AE7058 deliberately uses this box for triangle mass; no
                // triangle pose transform is applied again after this branch.
                let axis = |get: fn(Vector3) -> f32| {
                    let a = get(vertices[0]);
                    let b = get(vertices[1]);
                    let c = get(vertices[2]);
                    let low = native_arithmetic::vector_min(a, native_arithmetic::vector_min(b, c))
                        - fatness;
                    let high =
                        native_arithmetic::vector_max(a, native_arithmetic::vector_max(b, c))
                            + fatness;
                    let half = (high - low) * 0.5;
                    (half, low + half)
                };
                let x = axis(|v| v.x);
                let y = axis(|v| v.y);
                let z = axis(|v| v.z);
                (
                    MassShape::RoundedBox {
                        half_extents: Vector3::new(x.0, y.0, z.0),
                        radius: 0.0,
                    },
                    translated(Vector3::new(x.1, y.1, z.1)),
                )
            }
        };
        let mut moments = MassMoments::from_primitive(
            primitive_mass(shape).expect("every deck child follows a supported source mass branch"),
        );
        moments.transform(transform.basis, transform.translation);
        moments
    }
}

fn translated(translation: Vector3) -> RetailAffineTransform {
    RetailAffineTransform {
        translation,
        ..RetailAffineTransform::IDENTITY
    }
}

#[allow(clippy::too_many_arguments)]
fn fan_triangle(
    previous: (f32, f32),
    next: (f32, f32),
    slope: (f32, f32),
    width: f32,
    length: f32,
    center_z: f32,
    fatness: f32,
    front: bool,
    enabled: bool,
) -> DeckChild {
    let point = |arc: (f32, f32)| {
        let along_end = arc.0 * length;
        let elevation = if front { along_end } else { -along_end };
        Vector3::new(
            arc.1 * width,
            slope.0 * elevation,
            along_end.mul_add(slope.1, center_z),
        )
    };
    DeckChild {
        shape: DeckShape::Triangle {
            vertices: [
                Vector3::new(0.0, 0.0, center_z),
                point(previous),
                point(next),
            ],
            fatness,
            edge_cosines: [1.0, -1.0, 1.0],
            volume_flags: 0x3E2,
        },
        transform: RetailAffineTransform::IDENTITY,
        collision_enabled: enabled,
    }
}

/// Length path in 82E0B1E0; the shared estimate retains its explicitly marked
/// Xenon arithmetic approximation boundary until hardware validation.
fn vector_length_z(z: f32) -> f32 {
    let squared = native_arithmetic::dot3([0.0, 0.0, z, 0.0], [0.0, 0.0, z, 0.0]);
    let mut inverse = native_arithmetic::reciprocal_square_root_estimate(squared);
    for _ in 0..2 {
        let residual = (-squared).mul_add(inverse * inverse, 1.0);
        inverse = (inverse * 0.5).mul_add(residual, inverse);
    }
    if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    }
}

/// Standalone cosine 82473930. Its even-power tree differs from SinCos.
fn cosine(angle: f32) -> f32 {
    let turns = (angle * f32::from_bits(0x3E22_F983)).round_ties_even();
    let x = (-f32::from_bits(0x40C9_0FDB)).mul_add(turns, angle);
    let x2 = x * x;
    let x4 = x2 * x2;
    let x6 = x4 * x2;
    let x8 = x4 * x4;
    let x10 = x6 * x4;
    let x12 = x6 * x6;
    let x14 = x8 * x6;
    let x16 = x8 * x8;
    let x18 = x10 * x8;
    let x20 = x10 * x10;
    let x22 = x12 * x10;
    let mut result = (-0.5_f32).mul_add(x2, 1.0);
    for (power, coefficient) in [
        (x4, 0x3D2A_AAAB),
        (x6, 0xBAB6_0B61),
        (x8, 0x37D0_0D01),
        (x10, 0xB493_F27E),
        (x12, 0x310F_76C8),
        (x14, 0xAD49_CBA5),
        (x16, 0x2957_3F9F),
        (x18, 0xA534_13C3),
        (x20, 0x20F2_A15D),
        (x22, 0x9C86_71CB),
    ] {
        result = f32::from_bits(coefficient).mul_add(power, result);
    }
    result
}
