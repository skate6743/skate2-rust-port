//! Vertex/edge roots in TU3's triangle feature walk:82ADDB48/82ADA3E0.
use super::triangle_query::{cross, dot, sub};
use crate::math::Vector3;

#[derive(Clone, Copy)]
pub(super) struct Fraction {
    pub numerator: f32,
    pub denominator: f32,
}
impl Fraction {
    pub fn value(self) -> f32 {
        self.numerator / self.denominator
    }
    pub fn before(self, other: Self) -> bool {
        self.numerator * other.denominator < other.numerator * self.denominator
    }
}
pub(super) enum Root {
    Away,
    Miss,
    Hit(Fraction),
}

pub(super) fn sphere(start: Vector3, delta: Vector3, center: Vector3, radius: f32) -> Root {
    let toward_center = sub(center, start);
    let radius_squared = radius * radius;
    if dot(toward_center, toward_center) < radius_squared {
        return Root::Hit(Fraction {
            numerator: 0.0,
            denominator: 1.0,
        });
    }
    let approach = dot(toward_center, delta);
    if !(approach > 0.0) {
        return Root::Away;
    }
    let delta_squared = dot(delta, delta);
    let perpendicular = cross(toward_center, delta);
    let discriminant = delta_squared.mul_add(radius_squared, -dot(perpendicular, perpendicular));
    if discriminant < 0.0 {
        return Root::Miss;
    }
    let overrun = approach - delta_squared;
    if overrun > 0.0 && overrun * overrun > discriminant {
        return Root::Miss;
    }
    Root::Hit(Fraction {
        numerator: approach - discriminant.sqrt(),
        denominator: delta_squared,
    })
}

pub(super) fn cylinder(
    start: Vector3,
    delta: Vector3,
    origin: Vector3,
    edge: Vector3,
    radius: f32,
) -> Root {
    let perpendicular = cross(sub(origin, start), edge);
    let distance_squared = dot(perpendicular, perpendicular);
    let radius_squared = (dot(edge, edge) * radius) * radius;
    if distance_squared < radius_squared {
        return Root::Hit(Fraction {
            numerator: 0.0,
            denominator: 1.0,
        });
    }
    let delta_perpendicular = cross(delta, edge);
    let approach = dot(perpendicular, delta_perpendicular);
    if !(approach > 0.0) {
        return Root::Away;
    }
    let delta_squared = dot(delta_perpendicular, delta_perpendicular);
    let residual = approach.mul_add(approach, -(delta_squared * distance_squared));
    let discriminant = delta_squared.mul_add(radius_squared, residual);
    if discriminant < 0.0 {
        return Root::Miss;
    }
    let overrun = approach - delta_squared;
    // Cylinder rejects equality here; the sphere helper uses strict greater.
    if !(overrun < 0.0) && !(overrun * overrun < discriminant) {
        return Root::Miss;
    }
    Root::Hit(Fraction {
        numerator: approach - discriminant.sqrt(),
        denominator: delta_squared,
    })
}
