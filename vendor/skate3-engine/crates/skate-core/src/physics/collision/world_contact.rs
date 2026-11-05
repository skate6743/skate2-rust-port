//! Physics/world pair query 8277B720, called by triangle dispatch 8277BC58.
//! This is distinct from the general Volume query (82AD3CD8).
use super::{
    ContactPair, Sphere, Triangle, TriangleFixup, arithmetic::*, fix_up_triangle,
    sphere_triangle::build_prism,
};
use crate::math::Vector3;

#[derive(Clone, Copy, Debug)]
pub struct WorldContactSettings {
    /// Per-volume padding at cached primitive +112, excluding its radius.
    pub volume_padding: f32,
    /// Query settings +0. Bounds the approaching velocity's prediction.
    pub maximum_separating_distance: f32,
    /// Query settings +4 and +8, passed directly to triangle fixup.
    pub edge_cos_bend_normal_threshold: f32,
    pub convexity_epsilon: f32,
    /// World-query context byte +61, forwarded to TU3 vertex handling.
    pub is_object: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct WorldContact {
    /// B (world) toward A (moving primitive), ready for contact construction.
    pub normal: Vector3,
    pub points: ContactPair,
}

/// 8277BF0C..BF8C. The original predicts using the literal 1/60, independent
/// of the simulation timestep. Do not replace it with a caller's frame dt.
pub fn world_separation_limit(
    velocity: Vector3,
    triangle_normal: Vector3,
    padding: f32,
    maximum: f32,
) -> f32 {
    let approach = dot(
        neg(triangle_normal),
        scale(velocity, f32::from_bits(0x3C88_8889)),
    );
    let positive = if -approach >= 0.0 { 0.0 } else { approach };
    let limited = if maximum - positive >= 0.0 {
        positive
    } else {
        maximum
    };
    limited + padding
}

/// Complete sphere branch of the physics/world pair query. Input GP geometry
/// is in world space; world broad phase, exclusion tables and duplicate-contact
/// buffering remain the producer's responsibilities.
pub fn sphere_triangle_world_contact(
    sphere: Sphere,
    triangle: Triangle,
    velocity: Vector3,
    settings: WorldContactSettings,
) -> Option<WorldContact> {
    let separation = world_separation_limit(
        velocity,
        triangle.feature.normal,
        settings.volume_padding,
        settings.maximum_separating_distance,
    );
    let prism = build_prism(sphere, triangle, separation, false, true)?;
    let mut normal = prism.normal;
    let mut points = [prism.raw_points];
    if !fix_up_triangle(
        triangle.feature,
        &mut normal,
        &mut points,
        TriangleFixup {
            reverse: true,
            edge_cos_bend_normal_threshold: settings.edge_cos_bend_normal_threshold,
            convexity_epsilon: settings.convexity_epsilon,
            is_object: settings.is_object,
        },
    ) {
        return None;
    }
    // 8277B9E0..BA7C: fixup precedes publication. Both fatness operations
    // use separate multiply and add/subtract (the Volume query uses an FMA).
    let a = scale(normal, sphere.radius);
    Some(WorldContact {
        normal: neg(normal),
        points: ContactPair {
            a: Vector3::new(
                points[0].a.x + a.x,
                points[0].a.y + a.y,
                points[0].a.z + a.z,
            ),
            b: sub(points[0].b, scale(normal, triangle.fatness)),
        },
    })
}
