use super::super::{PrimitiveKind, prism_math::*};
use crate::{
    math::{Basis3, Vector3},
    physics::{
        collision::{Sphere, Triangle, TriangleFeature},
        reciprocal_sqrt::estimate,
    },
};

/// World-space board geometry. Capsule axes and box basis columns are the
/// transformed native volume axes; shape radii are applied after feature fixup.
#[derive(Clone, Copy, Debug)]
pub enum ContactPrimitive {
    Sphere(Sphere),
    Capsule {
        center: Vector3,
        axis: Vector3,
        half_length: f32,
        radius: f32,
    },
    Triangle(Triangle),
    RoundedBox {
        center: Vector3,
        basis: Basis3,
        half_extents: Vector3,
        radius: f32,
    },
}

/// Authored triangle Volume initialization and CreateGPInstance geometry
/// (82AC75B8, 82ADE468). The normal cache's dirty bit is cleared before copying
/// the Volume flags to GP flags; all other flags and edge cosines copy directly.
/// This constructor recomputes authored geometry. A cached native GPTriangle can
/// instead be passed directly as ContactPrimitive::Triangle.
pub fn triangle_from_volume(
    vertices: [Vector3; 3],
    fatness: f32,
    edge_cosines: [f32; 3],
    volume_flags: u32,
) -> Triangle {
    let points = vertices.map(|v| lanes(v, 1.0));
    let raw_normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
    let squared = dot(raw_normal, raw_normal);
    let normal = if squared > f32::from_bits(0x3400_0000) {
        scale(raw_normal, refined_inverse(squared))
    } else {
        raw_normal
    };
    let raw_edges = [
        sub(points[2], points[0]),
        sub(points[1], points[2]),
        sub(points[0], points[1]),
    ];
    let inverse = raw_edges.map(|e| refined_inverse(dot(e, e)));
    Triangle {
        vertices,
        feature: TriangleFeature {
            normal: xyz(normal),
            edges: std::array::from_fn(|i| xyz(scale(raw_edges[i], inverse[i]))),
            flags: volume_flags & !2,
            edge_cosines,
        },
        // 82ADE66C..: lengths are the twice-refined reciprocal of the refined
        // inverse length, not squared length multiplied by inverse length.
        edge_lengths: inverse.map(reciprocal),
        fatness,
    }
}

/// TriangleVolume::CreateGPInstance with a rigid Volume-to-world transform.
/// Source order computes the local normal first, transforms it without a new
/// normalization, and then builds directions and lengths from world vertices.
pub fn transform_triangle_volume(
    vertices: [Vector3; 3],
    fatness: f32,
    edge_cosines: [f32; 3],
    volume_flags: u32,
    basis: Basis3,
    translation: Vector3,
) -> Triangle {
    let local = triangle_from_volume(vertices, fatness, edge_cosines, volume_flags);
    let axes = basis.columns.map(|c| [c[0], c[1], c[2], 0.0]);
    let world_vertices = vertices.map(|v| {
        xyz(madd(
            axes[2],
            v.z,
            madd(axes[1], v.y, madd(axes[0], v.x, lanes(translation, 1.0))),
        ))
    });
    let n = local.feature.normal;
    let normal = xyz(madd(axes[2], n.z, madd(axes[1], n.y, scale(axes[0], n.x))));
    let mut result = triangle_from_volume(world_vertices, fatness, edge_cosines, volume_flags);
    result.feature.normal = normal;
    result
}
fn refined_inverse(squared: f32) -> f32 {
    let seed = estimate(squared);
    (seed * 0.5).mul_add((-squared).mul_add(seed * seed, 1.0), seed)
}

pub(in super::super) fn pack(shape: ContactPrimitive) -> ([u32; 48], PrimitiveKind) {
    let mut gp = [0; 48];
    let (kind, radius, counts, id) = match shape {
        ContactPrimitive::Sphere(sphere) => {
            put(&mut gp, 0, lanes(sphere.center, 1.0));
            (PrimitiveKind::Sphere, sphere.radius, 0, 1)
        }
        ContactPrimitive::Capsule {
            center,
            axis,
            half_length,
            radius,
        } => {
            put(&mut gp, 0, lanes(center, 1.0));
            put(&mut gp, 16, lanes(axis, 0.0));
            gp[28] = half_length.to_bits();
            (PrimitiveKind::Capsule, radius, 0x0001_0000, 2)
        }
        ContactPrimitive::Triangle(triangle) => {
            put(&mut gp, 0, lanes(triangle.vertices[0], 1.0));
            put(&mut gp, 4, lanes(triangle.feature.normal, 0.0));
            put(&mut gp, 8, lanes(triangle.vertices[1], 1.0));
            put(&mut gp, 12, lanes(triangle.vertices[2], 1.0));
            for i in 0..3 {
                put(&mut gp, 16 + i * 4, lanes(triangle.feature.edges[i], 0.0));
                gp[28 + i] = triangle.edge_lengths[i].to_bits();
                gp[38 + i] = triangle.feature.edge_cosines[i].to_bits();
            }
            gp[37] = triangle.feature.flags;
            (PrimitiveKind::Triangle, triangle.fatness, 0x0103_0000, 3)
        }
        ContactPrimitive::RoundedBox {
            center,
            basis,
            half_extents,
            radius,
        } => {
            put(&mut gp, 0, lanes(center, 1.0));
            let half = [half_extents.x, half_extents.y, half_extents.z];
            for i in 0..3 {
                let v = basis.columns[i];
                let axis = [v[0], v[1], v[2], 0.0];
                put(&mut gp, 4 + i * 4, axis);
                put(&mut gp, 16 + i * 4, axis);
                gp[28 + i] = half[i].to_bits();
            }
            (PrimitiveKind::Box, radius, 0x0303_0000, 4)
        }
    };
    gp[32] = radius.to_bits();
    gp[35] = counts;
    gp[36] = id;
    (gp, kind)
}

pub(in super::super) fn lanes(v: Vector3, w: f32) -> V {
    [v.x, v.y, v.z, w]
}
pub(in super::super) fn xyz(v: V) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn put(out: &mut [u32], offset: usize, v: V) {
    out[offset..offset + 4].copy_from_slice(&v.map(f32::to_bits));
}
