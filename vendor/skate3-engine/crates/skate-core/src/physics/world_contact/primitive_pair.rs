//! Simulation GP-pair contact query, original Skate 3 TU3 82AD43A8.
//!
//! 8277A508 calls this after its pair-culling bitmap and bounds checks, including
//! the same-assembly branch at 8277B2C0. This is not PrimitivePairIntersect
//! 82AD3CD8: that query has a squared-distance gate and different publication.
//! Host geometry is already transformed into world space. Native GP dispatch,
//! feature/prism order, normal refinement and radius application are retained.

use super::ContactPrimitive;
use super::{
    PrimitiveContactManifold, PrimitiveKind, best_separating_direction,
    find_feature_intersection_prism,
    primitive_query::{
        box_box_sat,
        geometry::{pack, xyz},
        maximum, triangle_box_sat,
    },
    prism_math::*,
    project_direction,
};
use crate::{
    math::Vector3,
    physics::collision::{ContactPair, TriangleFixup, fix_up_triangle},
};

#[derive(Clone, Copy, Debug)]
pub struct PrimitivePairSettings {
    /// Volume-record +112 for A/B in 8277A508, separate from shape fatness.
    pub padding_a: f32,
    pub padding_b: f32,
    /// Batch job +36, passed by 8277AB70; added after both volume paddings.
    pub additional_padding: f32,
    /// Native f2/f3 of 82AD43A8, used only for triangle feature correction.
    pub edge_cos_bend_normal_threshold: f32,
    pub convexity_epsilon: f32,
}

impl PrimitivePairSettings {
    /// Original skater/skater batch settings. SerializedAssembly::Create
    /// 82DC3AE4/82DC3B84 stores 82165A00 into every volume's padding (+112).
    /// Island::InitWorld 82765544 initializes additionalPadding to zero;
    /// 82DC1848 -> 82DC19E8 -> 8277B2A4 passes it to this query. The triangle
    /// constants are loaded by 8277ABEC..AC1C and forwarded at 8277A7DC..7EC.
    pub const fn skater_self_collision() -> Self {
        Self {
            padding_a: f32::from_bits(0x3D4C_CCCD),
            padding_b: f32::from_bits(0x3D4C_CCCD),
            additional_padding: 0.0,
            edge_cos_bend_normal_threshold: f32::from_bits(0x3F7F_BE77),
            convexity_epsilon: f32::from_bits(0x3C23_D70A),
        }
    }
}

/// Contacts between two simulation volumes. The normal points from B toward A;
/// `points[i].a/b` belong to the corresponding input volume. Positive geometric
/// gaps inside the native padding are retained for the solver to classify.
pub fn primitive_pair_contacts(
    primitive_a: ContactPrimitive,
    primitive_b: ContactPrimitive,
    settings: PrimitivePairSettings,
) -> Option<PrimitiveContactManifold> {
    let (a, a_kind) = pack(primitive_a);
    let (b, b_kind) = pack(primitive_b);
    let (separation, initial_normal) = separating_direction(&a, a_kind, &b, b_kind);
    // 8277A720..744 computes the pair padding before 82AD43F8..4414 adds
    // geometric fatness, preserving all four scalar additions in order.
    let limit = (settings.padding_b + settings.padding_a) + settings.additional_padding;
    let fat_a = f32::from_bits(a[32]);
    let fat_b = f32::from_bits(b[32]);
    if separation > (fat_b + limit) + fat_a {
        return None;
    }
    let mut feature_a = maximum(&a, a_kind, 1, initial_normal);
    let mut feature_b = maximum(&b, b_kind, 0, initial_normal.map(|w| w ^ 0x8000_0000));
    let mut prism = [0; 136];
    if find_feature_intersection_prism(&mut prism, &mut feature_a, &mut feature_b, initial_normal)
        == 0
    {
        return None;
    }
    let count = prism[132] as usize;
    let mut normal = initial_normal.map(f32::from_bits);
    if count == 1 {
        // 82AD44B0..4540 compares *length*, not squared length. The source
        // computes both reciprocal-square-root refinements before selecting
        // zero length for a zero squared distance.
        let delta = sub(load(&prism, 64), load(&prism, 0));
        let squared = dot(delta, delta);
        let inverse = inverse_length(squared);
        let length = if squared == 0.0 {
            0.0
        } else {
            squared * inverse
        };
        if length > f32::from_bits(0x3400_0000) {
            prism[128..132].copy_from_slice(&scale(delta, inverse).map(f32::to_bits));
            prism[133] = 1;
        }
    }
    if prism[133] != 0 {
        let direction: [u32; 4] = prism[128..132].try_into().unwrap();
        let mut interval_a = [0; 12];
        let mut interval_b = [0; 12];
        project_direction(&a, a_kind, direction, &mut interval_a);
        project_direction(&b, b_kind, direction, &mut interval_b);
        let forward = f32::from_bits(interval_a[0]) - f32::from_bits(interval_b[4]);
        let reverse = f32::from_bits(interval_b[0]) - f32::from_bits(interval_a[4]);
        let flip = forward > reverse;
        let refined = if flip { forward } else { reverse };
        if refined >= separation {
            normal = direction.map(|w| f32::from_bits(if flip { w ^ 0x8000_0000 } else { w }));
        }
    }
    let mut points = [ContactPair {
        a: Vector3::ZERO,
        b: Vector3::ZERO,
    }; 16];
    for (i, point) in points[..count].iter_mut().enumerate() {
        point.a = xyz(load(&prism, 4 * i));
        point.b = xyz(load(&prism, 64 + 4 * i));
    }
    // General GP query corrects either triangle, A first. The skater-specific
    // world query corrects only B and supplies a different is_object argument.
    let mut pair_normal = xyz(normal);
    for (primitive, reverse) in [(primitive_a, false), (primitive_b, true)] {
        if let ContactPrimitive::Triangle(triangle) = primitive {
            if !fix_up_triangle(
                triangle.feature,
                &mut pair_normal,
                &mut points[..count],
                TriangleFixup {
                    reverse,
                    edge_cos_bend_normal_threshold: settings.edge_cos_bend_normal_threshold,
                    convexity_epsilon: settings.convexity_epsilon,
                    is_object: false,
                },
            ) {
                return None;
            }
        }
    }
    let direction = [pair_normal.x, pair_normal.y, pair_normal.z, normal[3]];
    let offset_a = scale(direction, fat_a);
    let offset_b = scale(direction, fat_b);
    for point in &mut points[..count] {
        // 82AD46EC..4724 multiplies before adding/subtracting, not fused.
        point.a = Vector3::new(
            point.a.x + offset_a[0],
            point.a.y + offset_a[1],
            point.a.z + offset_a[2],
        );
        point.b = Vector3::new(
            point.b.x - offset_b[0],
            point.b.y - offset_b[1],
            point.b.z - offset_b[2],
        );
    }
    Some(PrimitiveContactManifold {
        normal: Vector3::new(-pair_normal.x, -pair_normal.y, -pair_normal.z),
        points,
        count,
    })
}

/// Exact non-cylinder entries of original dispatch table 82FD56F0.
fn separating_direction(
    a: &[u32; 48],
    a_kind: PrimitiveKind,
    b: &[u32; 48],
    b_kind: PrimitiveKind,
) -> (f32, [u32; 4]) {
    let (separation, direction) = match (a_kind, b_kind) {
        (PrimitiveKind::Sphere, PrimitiveKind::Sphere) => {
            // Specialized 82ACE9C0. The generic no-edge fallback is Z, so it
            // cannot substitute for this center-to-center sphere-pair axis.
            let delta = sub(load(b, 0), load(a, 0));
            let squared = dot(delta, delta);
            let inverse = inverse_length(squared);
            let length = if squared == 0.0 {
                0.0
            } else {
                squared * inverse
            };
            (
                [length.to_bits(); 4],
                scale(delta, inverse).map(f32::to_bits),
            )
        }
        (PrimitiveKind::Box, PrimitiveKind::Box) => box_box_sat::box_box(a, b),
        (PrimitiveKind::Triangle, PrimitiveKind::Box) => triangle_box_sat::triangle_box(a, b),
        (PrimitiveKind::Box, PrimitiveKind::Triangle) => {
            let (separation, normal) = triangle_box_sat::triangle_box(b, a);
            (separation, normal.map(|w| w ^ 0x8000_0000))
        }
        _ => best_separating_direction(a, a_kind, b, b_kind),
    };
    (f32::from_bits(separation[0]), direction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{math::Basis3, physics::collision::Sphere};

    fn settings() -> PrimitivePairSettings {
        PrimitivePairSettings {
            padding_a: 0.0,
            padding_b: 0.0,
            additional_padding: 0.0,
            edge_cos_bend_normal_threshold: 0.0,
            convexity_epsilon: 0.0,
        }
    }
    fn sphere(center: Vector3, radius: f32) -> ContactPrimitive {
        ContactPrimitive::Sphere(Sphere { center, radius })
    }
    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < 0.00001, "{a} != {b}");
    }
    #[test]
    fn sphere_pair_publishes_both_surfaces_and_solver_normal() {
        let a = sphere(Vector3::ZERO, 1.0);
        let b = sphere(Vector3::new(1.2, 1.6, 0.0), 1.1);
        let result = primitive_pair_contacts(a, b, settings()).unwrap();
        assert_eq!(result.count, 1);
        close(result.normal.x, -0.6);
        close(result.normal.y, -0.8);
        close(result.points[0].a.x, 0.6);
        close(result.points[0].a.y, 0.8);
        close(result.points[0].b.x, 0.54);
        close(result.points[0].b.y, 0.72);
        let reverse = primitive_pair_contacts(b, a, settings()).unwrap();
        close(reverse.normal.x, -result.normal.x);
        close(reverse.points[0].a.x, result.points[0].b.x);
    }
    #[test]
    fn separated_spheres_keep_padding_contacts_without_inflating_surface_points() {
        let a = sphere(Vector3::ZERO, 1.0);
        let b = sphere(Vector3::new(2.25, 0.0, 0.0), 1.0);
        assert!(primitive_pair_contacts(a, b, settings()).is_none());
        let s = PrimitivePairSettings {
            padding_a: 0.125,
            padding_b: 0.125,
            ..settings()
        };
        let result = primitive_pair_contacts(a, b, s).unwrap();
        close(result.points[0].a.x, 1.0);
        close(result.points[0].b.x, 1.25);
    }
    #[test]
    fn box_capsule_pair_supports_reversed_native_feature_dispatch() {
        let a = ContactPrimitive::RoundedBox {
            center: Vector3::ZERO,
            basis: Basis3 {
                columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            },
            half_extents: Vector3::new(1.0, 1.0, 1.0),
            radius: 0.1,
        };
        let b = ContactPrimitive::Capsule {
            center: Vector3::new(1.4, 0.0, 0.0),
            axis: Vector3::new(0.0, 1.0, 0.0),
            half_length: 0.5,
            radius: 0.5,
        };
        let result = primitive_pair_contacts(a, b, settings()).unwrap();
        assert!(result.count > 0);
        close(result.normal.x, -1.0);
        for point in &result.points[..result.count] {
            close(point.a.x, 1.1);
            close(point.b.x, 0.9);
        }
        let reversed = primitive_pair_contacts(b, a, settings()).unwrap();
        close(reversed.normal.x, 1.0);
        assert_eq!(reversed.count, result.count);
    }
}
